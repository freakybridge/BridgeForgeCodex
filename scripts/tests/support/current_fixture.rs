//! Current schema-4 setup for fixtures whose committed history may use older formats.
//! Historical payloads stay in Git; they never authorize a production legacy migration.
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub(crate) fn proposal(root: &Path) -> Result<Option<PathBuf>, String> {
    let identity = root.join(".codex/managed-skeleton.json");
    if let Ok(bytes) = fs::read(&identity) {
        let mut value: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        if value.get("release_version").is_none() {
            // This is explicit current fixture setup, not production identity repair.
            value["release_version"] = json!("8.0.0");
            fs::write(
                identity,
                serde_json::to_vec(&value).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
        }
    }
    let config = root.join(".codex/bridgeforge-version.json");
    let bytes = match fs::read(&config) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.to_string()),
    };
    let definition: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if definition["release_model"] == "product-packages" {
        return Ok(None);
    }
    let target = root.join(".runtime/current-configuration.json");
    if !target.exists() {
        let mut current = match definition["schema_version"].as_u64() {
            Some(1) => json!({"schema_version":4,"release_policy":"explicit_release",
                "strategy":"independent","adapters":{},"packages":[],
                "product_manifests":definition["manifests"].as_array().cloned().unwrap_or_else(||
                    ["Cargo.toml"].iter().filter(|p| root.join(p).is_file()).map(|p|json!(p)).collect())}),
            Some(3 | 4) => definition,
            _ => {
                return Err(
                    "fixture must declare an explicit current configuration proposal".into(),
                );
            }
        };
        current["schema_version"] = json!(4);
        current.as_object_mut().unwrap().remove("baseline");
        if let Some(mapping) = current
            .as_object_mut()
            .unwrap()
            .remove("synchronized_manifests")
        {
            current["product_manifests"] = mapping;
        }
        fs::create_dir_all(target.parent().unwrap()).map_err(|e| e.to_string())?;
        fs::write(
            &target,
            serde_json::to_vec(&current).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
    }
    // First-use setup has no live configuration. The old committed file is only
    // an immutable baseline read by the historical parser under test.
    fs::remove_file(config).map_err(|e| e.to_string())?;
    Ok(Some(PathBuf::from(".runtime/current-configuration.json")))
}
