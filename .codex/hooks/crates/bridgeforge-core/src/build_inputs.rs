use crate::manifest::{
    canonical_sha, generated_build_recipe, generated_source_sha, generated_sources,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) struct BuildInputs {
    original: PathBuf,
    pub snapshot: PathBuf,
    files: BTreeMap<String, Vec<u8>>,
    pub hashes: Value,
}

fn read_files(root: &Path) -> Result<BTreeMap<String, Vec<u8>>, String> {
    generated_sources(root)?
        .into_iter()
        .map(|relative| {
            let payload = fs::read(root.join(&relative)).map_err(|error| error.to_string())?;
            Ok((relative, payload))
        })
        .collect()
}

fn lock_sha(payload: &[u8]) -> String {
    let normalized = String::from_utf8_lossy(payload)
        .replace("\r\n", "\n")
        .replace('\r', "\n");
    format!("sha256:{:x}", Sha256::digest(normalized.as_bytes()))
}

impl BuildInputs {
    pub fn capture(original: &Path, snapshot: PathBuf, item: &Value) -> Result<Self, String> {
        if item["manifest"] != "Cargo.toml" || item["lockfile"] != "Cargo.lock" {
            return Err("generated build requires workspace Cargo.toml and Cargo.lock".into());
        }
        let binary = item["build"]["binary_name"]
            .as_str()
            .ok_or("missing binary_name")?;
        if !matches!(binary, "bridgeforge" | "bridgeforge-hook") {
            return Err("unsupported generated binary_name".into());
        }
        let recipe = generated_build_recipe(binary);
        if item["build"] != recipe {
            return Err("generated build recipe does not match executed Cargo command".into());
        }
        let args = item["self_test"]["args"]
            .as_array()
            .ok_or("missing self-test args")?;
        if args.iter().any(|arg| !arg.is_string())
            || !item["self_test"]["expected_json"].is_object()
        {
            return Err("invalid generated self-test contract".into());
        }
        let files = read_files(original)?;
        fs::create_dir_all(&snapshot).map_err(|error| error.to_string())?;
        for (relative, payload) in &files {
            let path = snapshot.join(relative);
            fs::create_dir_all(path.parent().ok_or("snapshot parent is missing")?)
                .map_err(|error| error.to_string())?;
            fs::write(path, payload).map_err(|error| error.to_string())?;
        }
        let sources = generated_sources(&snapshot)?;
        let hashes = json!({
            "source_tree_sha256": generated_source_sha(&snapshot, &sources)?,
            "lockfile_sha256": lock_sha(files.get("Cargo.lock").ok_or("missing Cargo.lock")?),
            "build_recipe_sha256": canonical_sha(&recipe)?,
            "self_test_sha256": canonical_sha(&item["self_test"])?,
        });
        for (key, actual) in hashes.as_object().ok_or("invalid input hashes")? {
            if &item[key] != actual {
                return Err(format!("generated build input hash mismatch: {key}"));
            }
        }
        let inputs = Self {
            original: original.to_path_buf(),
            snapshot,
            files,
            hashes,
        };
        inputs.verify_unchanged()?;
        Ok(inputs)
    }

    pub fn verify_unchanged(&self) -> Result<(), String> {
        self.verify_original_unchanged()?;
        if read_files(&self.snapshot)? != self.files {
            return Err("generated isolated build inputs changed during build".into());
        }
        Ok(())
    }

    pub fn verify_original_unchanged(&self) -> Result<(), String> {
        if read_files(&self.original)? != self.files {
            return Err("generated source inputs changed during build".into());
        }
        Ok(())
    }
}
