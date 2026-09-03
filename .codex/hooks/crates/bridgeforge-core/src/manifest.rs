use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};
use walkdir::WalkDir;

fn sha_raw(payload: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(payload))
}

fn sha_git(payload: &[u8]) -> String {
    if payload.contains(&0) {
        sha_raw(payload)
    } else {
        sha_raw(
            String::from_utf8_lossy(payload)
                .replace("\r\n", "\n")
                .replace('\r', "\n")
                .as_bytes(),
        )
    }
}

fn safe_join(root: &Path, relative: &str) -> Result<PathBuf, String> {
    let path = Path::new(relative);
    if relative.is_empty()
        || relative.contains('\\')
        || path.is_absolute()
        || path.components().any(|part| {
            matches!(
                part,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(format!("unsafe manifest path: {relative}"));
    }
    Ok(root.join(path))
}

fn canonical(value: &Value) -> Value {
    match value {
        Value::Object(object) => {
            let sorted = object
                .iter()
                .map(|(key, value)| (key.clone(), canonical(value)))
                .collect::<BTreeMap<_, _>>();
            Value::Object(sorted.into_iter().collect::<Map<_, _>>())
        }
        Value::Array(values) => Value::Array(values.iter().map(canonical).collect()),
        value => value.clone(),
    }
}

pub(crate) fn canonical_sha(value: &Value) -> Result<String, String> {
    serde_json::to_vec(&canonical(value))
        .map(|payload| sha_raw(&payload))
        .map_err(|error| error.to_string())
}

fn marker_block(payload: &[u8], begin: &str, end: &str) -> Result<Vec<u8>, String> {
    let text =
        String::from_utf8(payload.to_vec()).map_err(|_| "managed marker source is not UTF-8")?;
    let lines = text.split_inclusive('\n').collect::<Vec<_>>();
    let starts = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.trim_end_matches(['\r', '\n']) == begin)
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    let stops = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.trim_end_matches(['\r', '\n']) == end)
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    if starts.len() != 1 || stops.len() != 1 || starts[0] >= stops[0] {
        return Err(format!(
            "managed markers must appear exactly once: {begin} / {end}"
        ));
    }
    Ok(lines[starts[0]..=stops[0]].concat().into_bytes())
}

fn hook_asset_id(relative: &str) -> String {
    let mut slug = String::new();
    let mut dash = false;
    for character in relative.to_lowercase().chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character);
            dash = false;
        } else if !dash && !slug.is_empty() {
            slug.push('-');
            dash = true;
        }
    }
    while slug.ends_with('-') {
        slug.pop();
    }
    format!("codex.hooks.{slug}")
}

fn hook_sources(root: &Path) -> Result<Vec<String>, String> {
    generated_sources(&root.join("templates/hooks"))
}

pub(crate) fn generated_sources(source_root: &Path) -> Result<Vec<String>, String> {
    let mut result = Vec::new();
    for entry in WalkDir::new(source_root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            entry.depth() != 1
                || (entry.file_name() != "target"
                    && !entry.file_name().to_string_lossy().starts_with("project_"))
        })
    {
        let entry = entry.map_err(|error| error.to_string())?;
        if entry.file_type().is_symlink() {
            return Err(format!(
                "generated source tree contains a link: {}",
                entry.path().display()
            ));
        }
        if !entry.file_type().is_file() {
            continue;
        }
        let relative = entry
            .path()
            .strip_prefix(source_root)
            .map_err(|error| error.to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        if relative == "target" || relative.starts_with("target/") {
            continue;
        }
        result.push(relative);
    }
    result.sort();
    if !result.iter().any(|path| path == "Cargo.toml")
        || !result.iter().any(|path| path == "Cargo.lock")
    {
        return Err("templates/hooks must contain Cargo.toml and Cargo.lock".into());
    }
    Ok(result)
}

fn source_tree_sha(root: &Path, sources: &[String]) -> Result<String, String> {
    generated_source_sha(&root.join("templates/hooks"), sources)
}

pub(crate) fn generated_source_sha(root: &Path, sources: &[String]) -> Result<String, String> {
    let mut records = Vec::new();
    for relative in sources.iter().filter(|path| path.as_str() != "Cargo.lock") {
        let payload = fs::read(root.join(relative)).map_err(|error| error.to_string())?;
        records.push(json!({"path": relative, "sha256": sha_git(&payload)}));
    }
    canonical_sha(&Value::Array(records))
}

pub(crate) fn generated_build_recipe(binary: &str) -> Value {
    json!({
        "tool": "cargo",
        "args": ["build", "--locked", "--profile", "release", "--manifest-path", "{manifest}", "--target-dir", "{target_dir}", "--bin", "{binary_name}"],
        "profile": "release",
        "binary_name": binary,
    })
}

fn generated_asset(
    binary: &str,
    receipt: &str,
    sources_sha: &str,
    lock_sha: &str,
) -> Result<Value, String> {
    let build = generated_build_recipe(binary);
    let expected_name = if binary == "bridgeforge-hook" {
        "bridgeforge-hook"
    } else {
        "bridgeforge"
    };
    let self_test = json!({
        "args": ["self-test", "--json"],
        "expected_json": {"schema": 1, "name": expected_name, "status": "ok"},
    });
    let file = if binary == "bridgeforge-hook" {
        "bridgeforge-hook"
    } else {
        "bridgeforge"
    };
    Ok(json!({
        "id": if binary == "bridgeforge-hook" { "codex.hooks" } else { "codex.bridgeforge-cli" },
        "source_root": "templates/hooks",
        "target_source_root": ".codex/hooks",
        "manifest": "Cargo.toml",
        "lockfile": "Cargo.lock",
        "binary_targets": {
            "windows-x86_64": format!(".codex/bin/{file}.exe"),
            "linux-x86_64": format!(".codex/bin/{file}"),
            "macos-x86_64": format!(".codex/bin/{file}"),
        },
        "receipt_target": receipt,
        "build": build,
        "self_test": self_test,
        "source_tree_sha256": sources_sha,
        "lockfile_sha256": lock_sha,
        "build_recipe_sha256": canonical_sha(&build)?,
        "self_test_sha256": canonical_sha(&self_test)?,
    }))
}

pub fn render_managed_contract(root: &Path) -> Result<Vec<u8>, String> {
    let path = root.join("templates/managed-skeleton.json");
    let mut contract: Value = serde_json::from_slice(
        &fs::read(&path).map_err(|error| format!("cannot read managed contract: {error}"))?,
    )
    .map_err(|error| format!("invalid managed contract: {error}"))?;
    if contract["schema_version"].as_u64() != Some(4)
        || contract["baseline_model"].as_str() != Some("current-only")
    {
        return Err("managed contract must use schema 4 current-only".into());
    }
    contract["release_version"] = Value::String(
        fs::read_to_string(root.join("VERSION"))
            .map_err(|error| error.to_string())?
            .trim()
            .to_string(),
    );
    crate::baseline::compatibility_baseline(&contract)?;
    let sources = hook_sources(root)?;
    let existing = contract["assets"]
        .as_array()
        .ok_or("contract assets are missing")?;
    let mut assets = existing
        .iter()
        .filter(|asset| {
            let source = asset["source"].as_str().unwrap_or("");
            let target = asset["target"].as_str().unwrap_or("");
            !source.ends_with(".py")
                && !target.ends_with(".py")
                && !source.starts_with("templates/hooks/")
                && !target.starts_with(".codex/hooks/")
        })
        .cloned()
        .collect::<Vec<_>>();
    for relative in &sources {
        assets.push(json!({
            "id": hook_asset_id(relative),
            "source": format!("templates/hooks/{relative}"),
            "target": format!(".codex/hooks/{relative}"),
            "strategy": "whole",
            "current_sha256": "",
        }));
    }
    for asset in &mut assets {
        let source = asset["source"].as_str().ok_or("asset source is missing")?;
        let payload =
            fs::read(safe_join(root, source)?).map_err(|error| format!("{source}: {error}"))?;
        asset["current_sha256"] = Value::String(sha_git(&payload));
        if let Some(blocks) = asset
            .get_mut("managed_blocks")
            .filter(|value| value.is_object())
        {
            let projection = crate::baseline::markdown_projection(&payload, blocks)?;
            blocks["current_projection_sha256"] = Value::String(canonical_sha(&projection)?);
        }
        if asset["merge_policy"].as_str() == Some("codex-hooks") {
            let document = crate::baseline::parse_unique_json(&payload, "hooks.json")?;
            let handlers = crate::baseline::hook_handlers(&document)?;
            let mut required = Vec::new();
            for (id, record) in handlers {
                if !id.starts_with("bridgeforge-codex.project-hook.v1:") {
                    continue;
                }
                let mut projected = asset["merge_validation"]["required_handlers"]
                    .as_array()
                    .and_then(|entries| entries.iter().find(|entry| entry["id"] == id))
                    .cloned()
                    .unwrap_or_else(|| json!({}));
                projected["id"] = Value::String(id);
                projected["event"] = record["event"].clone();
                projected["matcher"] = record["matcher"].clone();
                projected["sha256"] = Value::String(canonical_sha(&record["handler"])?);
                required.push(projected);
            }
            if required.is_empty() {
                return Err("hooks.json has no managed project handlers".into());
            }
            asset["merge_validation"]["required_handlers"] = Value::Array(required);
        }
        if let Some(public) = asset
            .get_mut("agents_zones")
            .and_then(|zones| zones.get_mut("public"))
        {
            let block = marker_block(
                &payload,
                public["begin"]
                    .as_str()
                    .ok_or("AGENTS public begin is missing")?,
                public["end"]
                    .as_str()
                    .ok_or("AGENTS public end is missing")?,
            )?;
            public["current_sha256"] = Value::String(sha_git(&block));
        }
        if let Some(region) = asset.get_mut("region") {
            let block = marker_block(
                &payload,
                region["begin"].as_str().ok_or("region begin is missing")?,
                region["end"].as_str().ok_or("region end is missing")?,
            )?;
            region["current_sha256"] = Value::String(sha_git(&block));
        }
    }
    contract["assets"] = Value::Array(assets);
    let tree_sha = source_tree_sha(root, &sources)?;
    let lock_sha = sha_git(
        &fs::read(root.join("templates/hooks/Cargo.lock")).map_err(|error| error.to_string())?,
    );
    contract["generated_assets"] = Value::Array(vec![
        generated_asset(
            "bridgeforge-hook",
            ".codex/bin/build-receipt-hook.json",
            &tree_sha,
            &lock_sha,
        )?,
        generated_asset(
            "bridgeforge",
            ".codex/bin/build-receipt-cli.json",
            &tree_sha,
            &lock_sha,
        )?,
    ]);
    let mut encoded = serde_json::to_vec_pretty(&contract).map_err(|error| error.to_string())?;
    encoded.push(b'\n');
    Ok(encoded)
}

pub fn render_distribution_manifest(root: &Path) -> Result<Vec<u8>, String> {
    let path = root.join("bridgeforge-codex-manifest.json");
    let mut manifest: Value =
        serde_json::from_slice(&fs::read(&path).map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string())?;
    let platforms = manifest["platforms"]
        .as_object_mut()
        .ok_or("distribution manifest platforms are missing")?;
    for platform in platforms.values_mut() {
        let skills = platform["skills"]
            .as_array_mut()
            .ok_or("platform skills are missing")?;
        for skill in skills {
            let files = skill["files"]
                .as_array_mut()
                .ok_or("skill files are missing")?;
            for item in files {
                let source = item["source"].as_str().ok_or("skill source is missing")?;
                let payload = fs::read(safe_join(root, source)?)
                    .map_err(|error| format!("{source}: {error}"))?;
                item["sha256"] = Value::String(sha_git(&payload));
            }
        }
    }
    let mut encoded = serde_json::to_vec_pretty(&manifest).map_err(|error| error.to_string())?;
    encoded.push(b'\n');
    Ok(encoded)
}

pub fn rebuild(root: &Path, check: bool) -> Result<bool, String> {
    let contract = render_managed_contract(root)?;
    let distribution = render_distribution_manifest(root)?;
    let targets = [
        (
            root.join("templates/managed-skeleton.json"),
            contract.clone(),
        ),
        (root.join(".codex/managed-skeleton.json"), contract),
        (root.join("bridgeforge-codex-manifest.json"), distribution),
    ];
    let changed = targets
        .iter()
        .any(|(path, payload)| fs::read(path).ok().as_deref() != Some(payload.as_slice()));
    if changed && !check {
        for (path, payload) in targets {
            fs::write(path, payload).map_err(|error| error.to_string())?;
        }
    }
    Ok(changed)
}

#[cfg(all(test, bridgeforge_factory_tests))]
#[path = "../../../../../scripts/tests/unit/core_manifest.rs"]
mod tests;
