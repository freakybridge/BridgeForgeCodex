//! Factory-only preparation, separate from downstream project adoption and Git publication.
use crate::{CommandOutcome, ProcessRunner, generated_assets, manifest, project_sync};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

fn identity(root: &Path) -> Result<(Value, Vec<u8>), String> {
    if !root.join("templates/managed-skeleton.json").is_file() {
        return Err("factory runtime preparation requires the factory repository".into());
    }
    if root.join(".codex/.bridgeforge_codex_version").exists() {
        return Err("factory must not carry a downstream skeleton version stamp".into());
    }
    if manifest::rebuild(root, true)? {
        return Err("factory manifests are stale; synchronize source/dogfood and regenerate manifests before preparation".into());
    }
    crate::baseline::verify(root, None, false)?;
    let versions = crate::factory_version::check(root);
    if !versions.healthy {
        return Err(versions.issues.join("; "));
    }
    let bytes = fs::read(root.join(".codex/managed-skeleton.json")).map_err(|e| e.to_string())?;
    let contract: Value = crate::json::parse_unique_json(&bytes, "factory runtime contract")?;
    // The manifests bind every product input. Rechecking also validates the
    // current source inventory, normalized source bytes and dogfood mirror.
    let mut bound = BTreeMap::new();
    for relative in [
        "VERSION",
        "templates/managed-skeleton.json",
        ".codex/managed-skeleton.json",
        "bridgeforge-codex-manifest.json",
        crate::user_agents::MANIFEST,
    ] {
        bound.insert(
            relative,
            fs::read(root.join(relative)).map_err(|e| e.to_string())?,
        );
    }
    Ok((
        contract,
        serde_json::to_vec(&bound).map_err(|e| e.to_string())?,
    ))
}

fn paths(root: &Path, asset: &Value) -> Result<(PathBuf, PathBuf), String> {
    let platform = if cfg!(windows) {
        "windows-x86_64"
    } else if cfg!(target_os = "linux") {
        "linux-x86_64"
    } else {
        "macos-x86_64"
    };
    let join = |value: &Value| {
        crate::managed_paths::safe_join(
            root,
            value.as_str().ok_or("missing generated target")?,
            "factory generated target",
        )
    };
    Ok((
        join(&asset["binary_targets"][platform])?,
        join(&asset["receipt_target"])?,
    ))
}

pub fn prepare(root: &Path, check: bool, runner: &dyn ProcessRunner) -> CommandOutcome {
    match prepare_inner(root, check, runner) {
        Ok(result) => CommandOutcome::with_receipt(result),
        Err(error) => CommandOutcome::blocked(format!("[factory-runtime] {error}\n")),
    }
}

fn prepare_inner(root: &Path, check: bool, runner: &dyn ProcessRunner) -> Result<Value, String> {
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let (contract, original) = identity(&root)?;
    let assets = contract["generated_assets"]
        .as_array()
        .ok_or("missing generated assets")?;
    if check {
        // Deliberately no locks, Cargo, self-test processes, cache or receipts.
        for asset in assets {
            crate::baseline::verify_generated(&root, asset)?;
        }
        if identity(&root)?.1 != original {
            return Err("factory inputs changed during inspection".into());
        }
        return Ok(
            json!({"schema":1,"status":"current","read_only":true,"self_test_executed":false,"reused":assets.len(),"built":0}),
        );
    }
    // Same Git-common-dir lock as git-sync, including linked worktrees.
    let _sync = crate::git_sync::lock_factory_runtime(&root, runner)?;
    let _project = project_sync::ProjectLock::acquire(&root)?;
    if identity(&root)?.1 != original {
        return Err("factory inputs changed before preparation".into());
    }
    let mut before = BTreeMap::new();
    let mut binaries = BTreeSet::new();
    let mut pending = Vec::new();
    for asset in assets {
        let (binary, receipt) = paths(&root, asset)?;
        before.insert(
            binary.clone(),
            project_sync::transaction_file_state(&binary)?,
        );
        before.insert(
            receipt.clone(),
            project_sync::transaction_file_state(&receipt)?,
        );
        if !generated_assets::installed_current(&root, asset, &binary, runner) {
            binaries.insert(binary);
            pending.push(asset.clone());
        }
    }
    let rebuilt = pending.len();
    let environment = if rebuilt > 0 && runner.artifact_cache_enabled() {
        Some(
            crate::artifact_cache::environment(&root.join(".codex/hooks"), runner)
                .ok_or("cannot establish the factory compiler environment")?,
        )
    } else {
        None
    };
    let mut selected = contract.clone();
    selected["generated_assets"] = Value::Array(pending);
    let (writes, _) = if rebuilt == 0 {
        (BTreeMap::new(), Vec::new())
    } else {
        generated_assets::generated_writes_reusing(
            &root,
            "target_source_root",
            &root,
            &selected,
            runner,
        )?
    };
    if identity(&root)?.1 != original {
        return Err("factory inputs changed during build; installed files preserved".into());
    }
    if let Some(expected) = &environment {
        if crate::artifact_cache::environment(&root.join(".codex/hooks"), runner).as_ref()
            != Some(expected)
        {
            return Err(
                "factory compiler environment changed during build; installed files preserved"
                    .into(),
            );
        }
    }
    for (path, value) in &before {
        if &project_sync::transaction_file_state(path)? != value {
            return Err(format!(
                "factory target changed during build: {}",
                path.display()
            ));
        }
    }
    project_sync::install_generated_transaction(&root, &writes, &before, &binaries, || {
        if identity(&root)?.1 != original {
            return Err("factory inputs changed during installation".into());
        }
        for asset in assets {
            let (binary, _) = paths(&root, asset)?;
            if !generated_assets::installed_current(&root, asset, &binary, runner) {
                return Err(format!(
                    "installed factory asset verification failed: {}",
                    asset["id"]
                ));
            }
        }
        if identity(&root)?.1 != original {
            return Err("factory inputs changed during final verification".into());
        }
        Ok(())
    })?;
    Ok(
        json!({"schema":1,"status":if rebuilt == 0 {"current"} else {"prepared"},
        "read_only":false,"self_test_executed":true,"reused":assets.len()-rebuilt,
        "prepared":rebuilt,"written_files":writes.len(),"build_policy":"verified-cache-or-locked-cargo"}),
    )
}

#[cfg(all(test, bridgeforge_factory_tests))]
#[path = "../../../../../scripts/tests/unit/core_factory_runtime.rs"]
mod tests;
