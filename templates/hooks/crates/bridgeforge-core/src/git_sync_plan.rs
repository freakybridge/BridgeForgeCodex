use crate::ProcessRunner;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

pub(super) struct WritePlan {
    pub writes: BTreeMap<PathBuf, Vec<u8>>,
    pub binaries: BTreeSet<PathBuf>,
    pub before: BTreeMap<PathBuf, Option<Vec<u8>>>,
    inputs: BTreeMap<PathBuf, Vec<u8>>,
    release_inputs: BTreeMap<PathBuf, Option<Vec<u8>>>,
    source_inventory: Vec<String>,
}

fn join(root: &Path, relative: &str) -> Result<PathBuf, String> {
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
        if crate::memory::is_link_or_reparse(ancestor).map_err(|e| e.to_string())? {
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
        let platform = if cfg!(windows) {
            "windows-x86_64"
        } else if cfg!(target_os = "linux") {
            "linux-x86_64"
        } else {
            "macos-x86_64"
        };
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
            plan.binaries.insert(binary);
        }
        for path in writes.keys() {
            if !plan.before.contains_key(path) {
                plan.before.insert(path.clone(), read_optional(path)?);
            }
        }
        let (generated, _) = crate::project_sync::generated_writes(
            &temporary.0,
            "source_root",
            root,
            &contract_after,
            runner,
        )?;
        writes.extend(generated);
        plan.writes = writes;
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

fn read_optional(path: &Path) -> Result<Option<Vec<u8>>, String> {
    match fs::read(path) {
        Ok(value) => Ok(Some(value)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("cannot snapshot {}: {error}", path.display())),
    }
}
