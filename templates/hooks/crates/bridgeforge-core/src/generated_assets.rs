//! Shared isolated builds. Returns payloads; callers own installation and transactions.
use crate::{ProcessRequest, ProcessRunner, build_inputs, managed_paths::safe_join};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn sha_raw(payload: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(payload))
}

fn json_contains(actual: &Value, expected: &Value) -> bool {
    match (actual, expected) {
        (Value::Object(actual), Value::Object(expected)) => expected.iter().all(|(key, value)| {
            actual
                .get(key)
                .is_some_and(|item| json_contains(item, value))
        }),
        (Value::Array(actual), Value::Array(expected)) => {
            actual.len() == expected.len()
                && actual
                    .iter()
                    .zip(expected)
                    .all(|(left, right)| json_contains(left, right))
        }
        _ => actual == expected,
    }
}

pub(crate) fn generated_writes(
    source_base: &Path,
    source_key: &str,
    project_root: &Path,
    contract: &Value,
    runner: &dyn ProcessRunner,
) -> Result<(BTreeMap<PathBuf, Vec<u8>>, Vec<Value>), String> {
    let generated = contract["generated_assets"]
        .as_array()
        .ok_or("generated_assets is missing")?;
    let token = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_nanos();
    let target_dir = std::env::temp_dir().join(format!(
        "bridgeforge-generated-{}-{token}",
        std::process::id()
    ));
    fs::create_dir_all(&target_dir).map_err(|error| error.to_string())?;
    let result = (|| {
        let mut writes = BTreeMap::new();
        let mut receipts = Vec::new();
        let mut snapshots = BTreeMap::<PathBuf, build_inputs::BuildInputs>::new();
        for item in generated {
            let id = item["id"].as_str().ok_or("generated asset id is missing")?;
            let source_root = safe_join(
                source_base,
                item[source_key]
                    .as_str()
                    .ok_or_else(|| format!("generated {source_key} is missing"))?,
                "generated source root",
            )?;
            let snapshot = target_dir.join(format!("source-{}", snapshots.len()));
            // Reuse a verified snapshot for the Hook/CLI sharing one workspace.
            if !snapshots.contains_key(&source_root) {
                let inputs = build_inputs::BuildInputs::capture(&source_root, snapshot, item)?;
                snapshots.insert(source_root.clone(), inputs);
            }
            let inputs = snapshots
                .get(&source_root)
                .ok_or("generated snapshot is missing")?;
            inputs.verify_unchanged()?;
            // Validate each asset's recipe/self-test too, including shared workspaces.
            let binary = item["build"]["binary_name"]
                .as_str()
                .ok_or("missing binary_name")?;
            let recipe = crate::manifest::generated_build_recipe(binary);
            let recipe_sha = crate::manifest::canonical_sha(&recipe)?;
            let self_test_sha = crate::manifest::canonical_sha(&item["self_test"])?;
            if item["build"] != recipe
                || item["build_recipe_sha256"] != recipe_sha
                || item["self_test_sha256"] != self_test_sha
                || item["source_tree_sha256"] != inputs.hashes["source_tree_sha256"]
                || item["lockfile_sha256"] != inputs.hashes["lockfile_sha256"]
                || item["manifest"] != "Cargo.toml"
                || item["lockfile"] != "Cargo.lock"
                || !matches!(binary, "bridgeforge" | "bridgeforge-hook")
                || !item["self_test"]["expected_json"].is_object()
            {
                return Err("generated build input contract mismatch".into());
            }
            let manifest = inputs.snapshot.join("Cargo.toml");
            let binary_name = item["build"]["binary_name"]
                .as_str()
                .ok_or("generated binary_name is missing")?;
            // One target per isolated workspace shares dependency compilation.
            // Remove this asset's final executable first: a successful no-op
            // runner must never be able to validate the previous asset's output.
            let output_dir = inputs.snapshot.with_extension("target");
            let built = output_dir.join("release").join(if cfg!(windows) {
                format!("{binary_name}.exe")
            } else {
                binary_name.into()
            });
            match fs::remove_file(&built) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => {
                    return Err(format!(
                        "cannot clear generated binary before build: {error}"
                    ));
                }
            }
            let mut request = ProcessRequest::new("cargo", &inputs.snapshot);
            request.args = vec![
                OsString::from("build"),
                OsString::from("--locked"),
                OsString::from("--profile"),
                OsString::from("release"),
                OsString::from("--manifest-path"),
                manifest.into_os_string(),
                OsString::from("--target-dir"),
                output_dir.clone().into_os_string(),
                OsString::from("--bin"),
                OsString::from(binary_name),
            ];
            request.timeout = Duration::from_secs(900);
            let output = runner.run(&request).map_err(|error| error.to_string())?;
            if output.timed_out || output.code != 0 {
                return Err(format!(
                    "generated asset build failed: {id}: {}",
                    String::from_utf8_lossy(&output.stderr).trim()
                ));
            }
            let platform = if cfg!(windows) {
                "windows-x86_64"
            } else if cfg!(target_os = "linux") {
                "linux-x86_64"
            } else {
                "macos-x86_64"
            };
            let target = safe_join(
                project_root,
                item["binary_targets"][platform]
                    .as_str()
                    .ok_or("generated binary target is missing")?,
                "generated binary target",
            )?;
            let payload =
                fs::read(&built).map_err(|error| format!("cannot read built binary: {error}"))?;
            let self_test_args = item["self_test"]["args"]
                .as_array()
                .ok_or("generated self_test args are missing")?
                .iter()
                .map(|argument| {
                    argument
                        .as_str()
                        .map(OsString::from)
                        .ok_or("generated self_test arg must be text")
                })
                .collect::<Result<Vec<_>, _>>()?;
            let mut self_test =
                ProcessRequest::new(built.clone().into_os_string(), &inputs.snapshot);
            self_test.args = self_test_args;
            self_test.timeout = Duration::from_secs(60);
            let tested = runner.run(&self_test).map_err(|error| error.to_string())?;
            if tested.timed_out || tested.code != 0 {
                return Err(format!("generated asset self-test failed: {id}"));
            }
            let actual: Value = serde_json::from_slice(&tested.stdout)
                .map_err(|error| format!("generated asset self-test is not JSON: {id}: {error}"))?;
            if !json_contains(&actual, &item["self_test"]["expected_json"]) {
                return Err(format!("generated asset self-test contract mismatch: {id}"));
            }
            inputs.verify_unchanged()?;
            if fs::read(&built).map_err(|error| error.to_string())? != payload {
                return Err(format!("generated binary changed during self-test: {id}"));
            }
            let receipt = json!({
                "schema_version": 2,
                "generated_asset_id": id,
                "platform": platform,
                "binary_sha256": sha_raw(&payload),
                "source_tree_sha256": inputs.hashes["source_tree_sha256"],
                "lockfile_sha256": inputs.hashes["lockfile_sha256"],
                "build_recipe_sha256": recipe_sha,
                "self_test_sha256": self_test_sha,
            });
            let receipt_target = safe_join(
                project_root,
                item["receipt_target"]
                    .as_str()
                    .ok_or("receipt target is missing")?,
                "generated receipt target",
            )?;
            let mut encoded =
                serde_json::to_vec_pretty(&receipt).map_err(|error| error.to_string())?;
            encoded.push(b'\n');
            writes.insert(target, payload);
            writes.insert(receipt_target, encoded);
            receipts.push(receipt);
        }
        for inputs in snapshots.values() {
            inputs.verify_unchanged()?;
        }
        Ok((writes, receipts))
    })();
    let cleanup = fs::remove_dir_all(&target_dir);
    match (result, cleanup) {
        (Ok(value), Ok(())) => Ok(value),
        (Ok(_), Err(error)) => Err(format!("cannot remove generated build directory: {error}")),
        (Err(error), _) => Err(error),
    }
}
