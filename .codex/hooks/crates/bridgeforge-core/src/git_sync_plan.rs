use crate::{ProcessRequest, ProcessRunner};
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
        plan.source_inventory = crate::manifest::generated_sources(&root.join("templates/hooks"))?;
        paths.extend(
            plan.source_inventory
                .iter()
                .map(|p| format!("templates/hooks/{p}")),
        );
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
        let contract_after: serde_json::Value =
            serde_json::from_slice(&managed_after).map_err(|e| e.to_string())?;
        writes.insert(managed, managed_after.clone());
        writes.insert(join(root, ".codex/managed-skeleton.json")?, managed_after);
        writes.insert(distribution, distribution_after);
        writes.insert(
            join(root, crate::user_agents::MANIFEST)?,
            crate::user_agents::render_manifest(&temporary.0)?,
        );
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
        let mut cache = None;
        let mut cache_attempted = false;
        let mut cache_context = None;
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
            if reusable_generated(root, asset, &binary, runner) {
                plan.generated_reused += 1;
            } else {
                if !cache_attempted {
                    let context = Temporary(std::env::temp_dir().join(format!(
                        "bridgeforge-cache-context-{}-{nonce}",
                        std::process::id()
                    )));
                    let source = join(
                        &temporary.0,
                        asset["source_root"].as_str().ok_or("missing source_root")?,
                    )?;
                    let inputs = crate::project_sync::build_inputs::BuildInputs::capture(
                        &source,
                        context.0.join("source-0"),
                        asset,
                    )?;
                    cache = crate::artifact_cache::Cache::open(root, &inputs.snapshot, runner);
                    cache_context = Some((inputs, context));
                    cache_attempted = true;
                }
                if let Some((payload, receipt_payload)) =
                    cache.as_mut().and_then(|cache| cache.load(asset, runner))
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
            let mut pending_contract = contract_after;
            pending_contract["generated_assets"] = serde_json::Value::Array(pending_assets);
            let (generated, _) = crate::project_sync::generated_writes(
                &temporary.0,
                "source_root",
                root,
                &pending_contract,
                runner,
            )?;
            writes.extend(generated);
        }
        if let Some((inputs, _)) = &cache_context {
            inputs.verify_unchanged()?;
        }
        plan.writes = writes;
        plan.verify_unchanged(root)?;
        if let Some(cache) = cache.as_mut() {
            for asset in &all_assets {
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
        if let Some((inputs, _)) = &cache_context {
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
        if !self.source_inventory.is_empty()
            && crate::manifest::generated_sources(&root.join("templates/hooks"))?
                != self.source_inventory
        {
            return Err("factory source inventory changed concurrently".into());
        }
        Ok(())
    }
}

fn reusable_generated(
    root: &Path,
    asset: &serde_json::Value,
    binary: &Path,
    runner: &dyn ProcessRunner,
) -> bool {
    // The prospective contract binds the exact source tree, lock, recipe and
    // self-test. A receipt alone never establishes a hit without the binary hash.
    if crate::baseline::verify_generated(root, asset).is_err() {
        return false;
    }
    let Some(args) = asset["self_test"]["args"].as_array() else {
        return false;
    };
    let Some(args) = args
        .iter()
        .map(|arg| arg.as_str().map(std::ffi::OsString::from))
        .collect::<Option<Vec<_>>>()
    else {
        return false;
    };
    let mut request = ProcessRequest::new(binary.as_os_str(), root);
    request.args = args;
    request.timeout = std::time::Duration::from_secs(60);
    let Ok(output) = runner.run(&request) else {
        return false;
    };
    let Ok(actual) = serde_json::from_slice::<serde_json::Value>(&output.stdout) else {
        return false;
    };
    let Some(expected) = asset["self_test"]["expected_json"].as_object() else {
        return false;
    };
    !output.timed_out
        && output.code == 0
        && expected
            .iter()
            .all(|(key, value)| actual.get(key) == Some(value))
        && crate::baseline::verify_generated(root, asset).is_ok()
}

fn read_optional(path: &Path) -> Result<Option<Vec<u8>>, String> {
    match fs::read(path) {
        Ok(value) => Ok(Some(value)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("cannot snapshot {}: {error}", path.display())),
    }
}
