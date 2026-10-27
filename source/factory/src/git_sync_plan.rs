use crate::ProcessRunner;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

pub(super) struct WritePlan {
    pub generated_reused: usize,
    pub generated_built: usize,
    pub writes: BTreeMap<PathBuf, Vec<u8>>,
    pub binaries: BTreeSet<PathBuf>,
    pub before: BTreeMap<PathBuf, Option<Vec<u8>>>,
    inputs: BTreeMap<PathBuf, Vec<u8>>,
    release_inputs: BTreeMap<PathBuf, Option<Vec<u8>>>,
    source_inventory: Vec<String>,
}

pub(super) fn join(root: &Path, relative: &str) -> Result<PathBuf, String> {
    if relative.is_empty()
        || relative.contains('\\')
        || Path::new(relative)
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err(format!("unsafe factory input path: {relative}"));
    }
    let path = root.join(relative);
    for ancestor in path.ancestors().filter(|p| p.exists()) {
        if crate::persistence::is_link_or_reparse(ancestor).map_err(|e| e.to_string())? {
            return Err(format!(
                "factory input traverses a link: {}",
                path.display()
            ));
        }
    }
    Ok(path)
}

struct Temporary(PathBuf);
impl Drop for Temporary {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

impl WritePlan {
    pub fn prepare(
        root: &Path,
        mut writes: BTreeMap<PathBuf, Vec<u8>>,
        release_inputs: BTreeMap<PathBuf, Option<Vec<u8>>>,
        factory: bool,
        runner: &dyn ProcessRunner,
    ) -> Result<Self, String> {
        crate::release::verify_release_inputs(&release_inputs)?;
        let mut plan = Self {
            generated_reused: 0,
            generated_built: 0,
            writes: BTreeMap::new(),
            binaries: BTreeSet::new(),
            before: BTreeMap::new(),
            inputs: BTreeMap::new(),
            release_inputs,
            source_inventory: Vec::new(),
        };
        for path in writes.keys() {
            plan.before.insert(path.clone(), read_optional(path)?);
        }
        if !factory {
            plan.writes = writes;
            return Ok(plan);
        }

        let managed = join(root, "templates/managed-skeleton.json")?;
        let distribution = join(root, "bridgeforge-codex-manifest.json")?;
        let managed_bytes = fs::read(&managed).map_err(|e| e.to_string())?;
        let distribution_bytes = fs::read(&distribution).map_err(|e| e.to_string())?;
        let contract: serde_json::Value =
            serde_json::from_slice(&managed_bytes).map_err(|e| e.to_string())?;
        let skills: serde_json::Value =
            serde_json::from_slice(&distribution_bytes).map_err(|e| e.to_string())?;
        let mut paths = BTreeSet::from(["VERSION".to_string()]);
        paths.insert(crate::user_agents::SOURCE.into());
        paths.insert(crate::user_agents::MANIFEST.into());
        let modern = crate::source_projection::available(root);
        if modern {
            paths.insert("templates/source-layout.json".into());
            paths.insert("templates/hooks/.gitignore".into());
            if root.join("scripts/tests/Cargo.toml").is_file() {
                paths.insert("scripts/tests/Cargo.toml".into());
                paths.insert("scripts/tests/Cargo.lock".into());
            }
            plan.source_inventory = crate::source_projection::render(root, false)?
                .into_keys()
                .collect();
            // The projection's ignore file is rendered from the already-bound
            // templates/hooks input, not the repository's own Git settings.
            paths.extend(
                plan.source_inventory
                    .iter()
                    .filter(|p| p.as_str() != ".gitignore")
                    .cloned(),
            );
        } else {
            plan.source_inventory =
                crate::manifest::generated_sources(&root.join("templates/hooks"))?;
            paths.extend(
                plan.source_inventory
                    .iter()
                    .map(|p| format!("templates/hooks/{p}")),
            );
        }
        for asset in contract["assets"]
            .as_array()
            .ok_or("missing factory assets")?
        {
            let source = asset["source"].as_str().ok_or("missing asset source")?;
            let target = asset["target"].as_str().ok_or("missing asset target")?;
            if !source.ends_with(".py")
                && !target.ends_with(".py")
                && !source.starts_with("templates/hooks/")
                && !target.starts_with(".codex/hooks/")
            {
                paths.insert(source.into());
            }
        }
        for platform in skills["platforms"]
            .as_object()
            .ok_or("missing distribution platforms")?
            .values()
        {
            for skill in platform["skills"]
                .as_array()
                .ok_or("missing platform skills")?
            {
                for file in skill["files"].as_array().ok_or("missing skill files")? {
                    paths.insert(
                        file["source"]
                            .as_str()
                            .ok_or("missing skill source")?
                            .into(),
                    );
                }
            }
        }
        plan.inputs.insert(managed.clone(), managed_bytes);
        plan.inputs.insert(distribution.clone(), distribution_bytes);
        for relative in paths {
            let path = join(root, &relative)?;
            plan.inputs
                .insert(path.clone(), fs::read(path).map_err(|e| e.to_string())?);
        }
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos();
        let temporary = Temporary(std::env::temp_dir().join(format!(
            "bridgeforge-sync-plan-{}-{nonce}",
            std::process::id()
        )));
        fs::create_dir(&temporary.0).map_err(|e| e.to_string())?;
        for (path, payload) in plan.inputs.iter().chain(writes.iter()) {
            let relative = path.strip_prefix(root).map_err(|e| e.to_string())?;
            let target = join(&temporary.0, &relative.to_string_lossy().replace('\\', "/"))?;
            fs::create_dir_all(target.parent().unwrap()).map_err(|e| e.to_string())?;
            fs::write(target, payload).map_err(|e| e.to_string())?;
        }
        let managed_after = crate::manifest::render_managed_contract(&temporary.0)?;
        let distribution_after = crate::manifest::render_distribution_manifest(&temporary.0)?;
        let public_after: serde_json::Value =
            serde_json::from_slice(&managed_after).map_err(|e| e.to_string())?;
        let contract_after = crate::manifest::runtime_contract(&temporary.0, &public_after)?;
        writes.insert(managed, managed_after.clone());
        writes.insert(join(root, ".codex/managed-skeleton.json")?, managed_after);
        writes.insert(distribution, distribution_after);
        writes.insert(
            join(root, crate::user_agents::MANIFEST)?,
            crate::user_agents::render_manifest(&temporary.0)?,
        );
        if modern {
            for (relative, payload) in
                crate::manifest::development_projection(&temporary.0, &BTreeMap::new())?
            {
                let target = join(root, &relative)?;
                if writes.get(&target).is_some_and(|bytes| bytes != &payload) {
                    return Err(format!(
                        "factory derived output conflicts with native release: {relative}"
                    ));
                }
                if read_optional(&target)?.as_deref() != Some(payload.as_slice()) {
                    writes.insert(target, payload);
                }
            }
        }
        let platform = if cfg!(windows) {
            "windows-x86_64"
        } else if cfg!(target_os = "linux") {
            "linux-x86_64"
        } else {
            "macos-x86_64"
        };
        let mut pending_assets = Vec::new();
        let all_assets = contract_after["generated_assets"]
            .as_array()
            .ok_or("missing generated assets")?
            .clone();
        let mut built_assets = BTreeSet::new();
        let mut cache_contexts = BTreeMap::<
            String,
            (
                crate::build_inputs::BuildInputs,
                Temporary,
                Option<crate::artifact_cache::Cache>,
            ),
        >::new();
        for asset in contract_after["generated_assets"]
            .as_array()
            .ok_or("missing generated assets")?
        {
            let binary = join(
                root,
                asset["binary_targets"][platform]
                    .as_str()
                    .ok_or("missing binary target")?,
            )?;
            let receipt = join(
                root,
                asset["receipt_target"]
                    .as_str()
                    .ok_or("missing receipt target")?,
            )?;
            plan.before.insert(binary.clone(), read_optional(&binary)?);
            plan.before
                .insert(receipt.clone(), read_optional(&receipt)?);
            if crate::generated_assets::installed_current(root, asset, &binary, runner) {
                plan.generated_reused += 1;
            } else {
                let cache_key = serde_json::to_string(&(
                    asset["source_root"].clone(),
                    asset["source_projection"].clone(),
                ))
                .map_err(|e| e.to_string())?;
                if !cache_contexts.contains_key(&cache_key) {
                    let context = Temporary(std::env::temp_dir().join(format!(
                        "bridgeforge-cache-context-{}-{nonce}-{}",
                        std::process::id(),
                        cache_contexts.len()
                    )));
                    let source = source_root(&temporary.0, asset)?;
                    let inputs = crate::build_inputs::BuildInputs::capture(
                        &source,
                        context.0.join("source-0"),
                        asset,
                    )?;
                    let cache = crate::artifact_cache::Cache::open(root, &inputs.snapshot, runner);
                    cache_contexts.insert(cache_key.clone(), (inputs, context, cache));
                }
                if let Some((payload, receipt_payload)) = cache_contexts
                    .get_mut(&cache_key)
                    .and_then(|(_, _, cache)| cache.as_mut())
                    .and_then(|cache| cache.load(asset, runner))
                {
                    writes.insert(binary.clone(), payload);
                    writes.insert(receipt, receipt_payload);
                    plan.generated_reused += 1;
                } else {
                    pending_assets.push(asset.clone());
                    built_assets
                        .insert(asset["id"].as_str().ok_or("missing asset id")?.to_string());
                }
                plan.binaries.insert(binary);
            }
        }
        for path in writes.keys() {
            if !plan.before.contains_key(path) {
                plan.before.insert(path.clone(), read_optional(path)?);
            }
        }
        plan.generated_built = pending_assets.len();
        if !pending_assets.is_empty() {
            let mut captured = BTreeMap::new();
            for asset in &pending_assets {
                let cache_key = serde_json::to_string(&(
                    asset["source_root"].clone(),
                    asset["source_projection"].clone(),
                ))
                .map_err(|error| error.to_string())?;
                if let Some((inputs, _, _)) = cache_contexts.get(&cache_key) {
                    captured.insert(
                        (
                            source_root(&temporary.0, asset)?,
                            asset["source_projection"].as_str().map(str::to_owned),
                        ),
                        inputs,
                    );
                }
            }
            let mut pending_contract = contract_after;
            pending_contract["generated_assets"] = serde_json::Value::Array(pending_assets);
            let (generated, _) = crate::generated_assets::generated_writes_captured(
                &temporary.0,
                "source_root",
                root,
                &pending_contract,
                runner,
                &captured,
            )?;
            writes.extend(generated);
        }
        for (inputs, _, _) in cache_contexts.values() {
            inputs.verify_unchanged()?;
        }
        plan.writes = writes;
        plan.verify_unchanged(root)?;
        for asset in &all_assets {
            let cache_key = serde_json::to_string(&(
                asset["source_root"].clone(),
                asset["source_projection"].clone(),
            ))
            .map_err(|e| e.to_string())?;
            if let Some(cache) = cache_contexts
                .get_mut(&cache_key)
                .and_then(|(_, _, cache)| cache.as_mut())
            {
                if !built_assets.contains(asset["id"].as_str().ok_or("missing asset id")?) {
                    continue;
                }
                let binary = join(
                    root,
                    asset["binary_targets"][platform]
                        .as_str()
                        .ok_or("missing binary target")?,
                )?;
                let receipt = join(
                    root,
                    asset["receipt_target"]
                        .as_str()
                        .ok_or("missing receipt target")?,
                )?;
                if let (Some(payload), Some(receipt_payload)) =
                    (plan.writes.get(&binary), plan.writes.get(&receipt))
                {
                    // Loaded cache entries already have a lease and fresh recency.
                    if let Err(error) = cache.store(asset, payload, receipt_payload, runner) {
                        eprintln!("[artifact-cache] optional cache publication skipped: {error}");
                    }
                }
            }
        }
        for (inputs, _, _) in cache_contexts.values() {
            inputs.verify_unchanged()?;
        }
        plan.verify_unchanged(root)?;
        Ok(plan)
    }

    pub fn verify_unchanged(&self, root: &Path) -> Result<(), String> {
        crate::release::verify_release_inputs(&self.release_inputs)?;
        for (path, expected) in &self.inputs {
            if fs::read(path).ok().as_ref() != Some(expected) {
                return Err(format!(
                    "factory build input changed concurrently: {}",
                    path.display()
                ));
            }
        }
        for (path, expected) in &self.before {
            if &read_optional(path)? != expected {
                return Err(format!(
                    "automatic target changed concurrently before apply: {}",
                    path.display()
                ));
            }
        }
        let current_inventory = if crate::source_projection::available(root) {
            crate::source_projection::render(root, false)?
                .into_keys()
                .collect::<Vec<_>>()
        } else {
            crate::manifest::generated_sources(&root.join("templates/hooks"))?
        };
        if !self.source_inventory.is_empty() && current_inventory != self.source_inventory {
            return Err("factory source inventory changed concurrently".into());
        }
        Ok(())
    }
}

fn source_root(root: &Path, asset: &serde_json::Value) -> Result<PathBuf, String> {
    let relative = asset["source_root"].as_str().ok_or("missing source_root")?;
    if relative == "."
        && matches!(
            asset["source_projection"].as_str(),
            Some(crate::source_projection::PUBLIC | crate::source_projection::FACTORY)
        )
    {
        Ok(root.to_path_buf())
    } else {
        join(root, relative)
    }
}

fn read_optional(path: &Path) -> Result<Option<Vec<u8>>, String> {
    match fs::read(path) {
        Ok(value) => Ok(Some(value)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("cannot snapshot {}: {error}", path.display())),
    }
}
