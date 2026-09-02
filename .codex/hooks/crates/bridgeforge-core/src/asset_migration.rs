use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};
use walkdir::WalkDir;

const DERIVED: &[&str] = &["MEMORY.md", "MEMORY_COLD.md", "_stats.json"];

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SourceAsset {
    pub asset_id: String,
    pub source_path: String,
    pub source_sha256: String,
    pub kind: String,
    pub fixed_retirement: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TargetWrite {
    pub source_asset_ids: Vec<String>,
    pub target: String,
    pub asset_type: String,
    pub reason: String,
    pub before_sha256: Option<String>,
    pub after_sha256: String,
    #[serde(skip)]
    pub payload: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidatedMigration {
    pub schema_version: u64,
    pub status: String,
    pub manifest_sha256: String,
    pub sources: Vec<SourceAsset>,
    pub targets: Vec<TargetWrite>,
}

fn sha(payload: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(payload))
}

fn relative(raw: &str, label: &str) -> Result<PathBuf, String> {
    let path = Path::new(raw);
    if raw.is_empty()
        || raw.contains('\\')
        || path.is_absolute()
        || path.components().any(|part| {
            matches!(
                part,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(format!(
            "{label} must be a canonical POSIX relative path: {raw}"
        ));
    }
    Ok(path.to_path_buf())
}

fn scan_root(
    root: &Path,
    folder: &str,
    memory: bool,
    output: &mut Vec<SourceAsset>,
) -> Result<(), String> {
    let base = root.join(folder);
    if !base.exists() {
        return Ok(());
    }
    for entry in WalkDir::new(&base).follow_links(false) {
        let entry = entry.map_err(|error| error.to_string())?;
        if entry.file_type().is_symlink() {
            return Err(format!(
                "legacy source tree contains a linked path: {}",
                entry.path().display()
            ));
        }
        if !entry.file_type().is_file()
            || (!memory && entry.path().extension().and_then(|value| value.to_str()) != Some("md"))
        {
            continue;
        }
        let path = entry
            .path()
            .strip_prefix(root)
            .map_err(|error| error.to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        let name = entry.file_name().to_string_lossy();
        let fixed = memory && DERIVED.contains(&name.as_ref());
        output.push(SourceAsset {
            asset_id: format!("legacy-{}:{path}", if memory { "memory" } else { "rule" }),
            source_path: path,
            source_sha256: sha(&fs::read(entry.path()).map_err(|error| error.to_string())?),
            kind: if fixed {
                "derived-memory"
            } else if memory {
                "legacy-memory"
            } else {
                "legacy-rule"
            }
            .into(),
            fixed_retirement: fixed,
        });
    }
    Ok(())
}

pub fn scan_sources(root: &Path) -> Result<Vec<SourceAsset>, String> {
    let mut result = Vec::new();
    scan_root(root, ".codex/rules", false, &mut result)?;
    scan_root(root, ".codex/memory", true, &mut result)?;
    result.sort_by(|left, right| {
        left.source_path
            .to_lowercase()
            .cmp(&right.source_path.to_lowercase())
    });
    let mut seen = BTreeSet::new();
    for source in &result {
        if !seen.insert(source.source_path.to_lowercase()) {
            return Err(format!(
                "legacy source paths collide case-insensitively: {}",
                source.source_path
            ));
        }
    }
    Ok(result)
}

fn valid_target_type(kind: &str, target: &str) -> bool {
    match kind {
        "agents" => target.ends_with("AGENTS.md"),
        "command-rule" => target.starts_with(".codex/rules/") && target.ends_with(".rules"),
        "skill" => target.starts_with(".codex/skills/") && target.ends_with("/SKILL.md"),
        "hook" => target
            .strip_prefix(".codex/hooks/")
            .and_then(|path| path.split_once('/'))
            .is_some_and(|(bundle, entry)| {
                bundle.starts_with("project_") && bundle.len() > 8 && entry == "entrypoint.rs"
            }),
        "hook-registration" => target == ".codex/hooks.json",
        "test" => target.starts_with("scripts/tests/") && target.ends_with(".rs"),
        "delivery" | "todo" => target.starts_with("doc/1_delivery/") && target.ends_with(".md"),
        "bug" => target.starts_with("doc/2_bugs/") && target.ends_with(".md"),
        "documentation" => {
            target == "doc/README.md"
                || target.starts_with("doc/0_architecture/")
                || target.starts_with("doc/3_reference/")
                || target.starts_with("doc/4_archive/")
        }
        _ => false,
    }
}

pub fn validate_manifest(
    root: &Path,
    manifest: &Value,
    reserved_targets: &[String],
) -> Result<ValidatedMigration, String> {
    if manifest["schema_version"].as_u64() != Some(1) {
        return Err("migration manifest schema_version must be 1".into());
    }
    let records = manifest["sources"]
        .as_array()
        .ok_or("migration manifest sources must be an array")?;
    let scanned = scan_sources(root)?;
    let by_path = scanned
        .iter()
        .map(|item| (item.source_path.as_str(), item))
        .collect::<BTreeMap<_, _>>();
    let retired = scanned
        .iter()
        .map(|item| item.source_path.to_lowercase())
        .collect::<BTreeSet<_>>();
    let reserved = reserved_targets
        .iter()
        .map(|item| item.to_lowercase())
        .collect::<BTreeSet<_>>();
    let mut seen_sources = BTreeSet::new();
    let mut targets: BTreeMap<String, TargetWrite> = BTreeMap::new();
    for record in records {
        let source_path = record["source_path"]
            .as_str()
            .ok_or("migration source_path is missing")?;
        relative(source_path, "migration source")?;
        if !seen_sources.insert(source_path.to_string()) {
            return Err(format!("migration manifest repeats source: {source_path}"));
        }
        let source = by_path
            .get(source_path)
            .ok_or_else(|| format!("migration names unknown source: {source_path}"))?;
        if record["asset_id"].as_str() != Some(&source.asset_id)
            || record["source_sha256"].as_str() != Some(&source.source_sha256)
            || record["kind"].as_str() != Some(&source.kind)
        {
            return Err(format!("migration source identity drifted: {source_path}"));
        }
        if record["confirmed"].as_bool() != Some(true)
            || record["retire_source"].as_bool() != Some(true)
        {
            return Err(format!(
                "source migration is not confirmed for retirement: {source_path}"
            ));
        }
        let decisions = record["decisions"]
            .as_array()
            .ok_or("migration decisions must be an array")?;
        let discarded = record["discarded"]
            .as_array()
            .ok_or("migration discarded must be an array")?;
        if source.fixed_retirement {
            if !decisions.is_empty()
                || !discarded.is_empty()
                || record["retirement_reason"].as_str() != Some("fixed-derived-retirement")
            {
                return Err(format!(
                    "derived memory must use fixed retirement: {source_path}"
                ));
            }
        } else if decisions.is_empty() && discarded.is_empty() {
            return Err(format!(
                "migration source has no target or discard decision: {source_path}"
            ));
        }
        for decision in decisions {
            let target = decision["target"]
                .as_str()
                .ok_or("migration target is missing")?;
            relative(target, "migration target")?;
            let folded = target.to_lowercase();
            if retired.contains(&folded)
                || (reserved.contains(&folded)
                    && !matches!(target, "AGENTS.md" | ".codex/hooks.json" | "doc/README.md"))
                || folded.starts_with(".codex/memory/")
            {
                return Err(format!("migration target is reserved or retired: {target}"));
            }
            let asset_type = decision["asset_type"]
                .as_str()
                .ok_or("migration asset_type is missing")?;
            if !valid_target_type(asset_type, target) {
                return Err(format!("asset_type {asset_type} cannot target {target}"));
            }
            let content = decision["content_utf8"]
                .as_str()
                .ok_or("content_utf8 must be text")?;
            let target_path = root.join(target);
            let before = fs::read(&target_path).ok().map(|payload| sha(&payload));
            if decision.get("target_before_sha256").and_then(Value::as_str) != before.as_deref() {
                return Err(format!("migration target hash drifted: {target}"));
            }
            let write = TargetWrite {
                source_asset_ids: vec![source.asset_id.clone()],
                target: target.into(),
                asset_type: asset_type.into(),
                reason: decision["reason"].as_str().unwrap_or("").into(),
                before_sha256: before,
                after_sha256: sha(content.as_bytes()),
                payload: content.as_bytes().to_vec(),
            };
            match targets.get_mut(&folded) {
                Some(existing)
                    if existing.target == write.target
                        && existing.asset_type == write.asset_type
                        && existing.payload == write.payload =>
                {
                    existing.source_asset_ids.push(source.asset_id.clone());
                }
                Some(_) => return Err(format!("shared migration target disagrees: {target}")),
                None => {
                    targets.insert(folded, write);
                }
            }
        }
    }
    let missing = by_path
        .keys()
        .filter(|path| !seen_sources.contains(**path))
        .copied()
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(format!(
            "manifest does not cover every legacy source: {}",
            missing.join(", ")
        ));
    }
    for target in targets.values() {
        if target.asset_type == "hook" {
            let registration = targets
                .get(".codex/hooks.json")
                .ok_or("hook migration requires its hooks.json registration")?;
            let document: Value = serde_json::from_slice(&registration.payload)
                .map_err(|_| "hook registration is not JSON")?;
            let registered = document["hooks"]
                .as_object()
                .into_iter()
                .flat_map(|hooks| hooks.values())
                .flat_map(|groups| groups.as_array().into_iter().flatten())
                .flat_map(|group| group["hooks"].as_array().into_iter().flatten())
                .filter(|handler| {
                    handler["type"] == "command" && handler.get("bridgeforgeCodexId").is_none()
                })
                .any(|handler| {
                    ["command", "commandWindows"].iter().any(|key| {
                        handler[*key].as_str().is_some_and(|command| {
                            command
                                .replace('\\', "/")
                                .split(|c: char| c.is_whitespace() || matches!(c, '\'' | '"'))
                                .any(|argument| {
                                    argument == target.target
                                        || argument.ends_with(&format!("/{}", target.target))
                                })
                        })
                    })
                });
            if !registered {
                return Err(format!(
                    "hook migration is not registered: {}",
                    target.target
                ));
            }
        }
        if target.target.starts_with("doc/")
            && target.target != "doc/README.md"
            && target.before_sha256.is_none()
        {
            let index = targets
                .get("doc/readme.md")
                .ok_or("new migration documents require doc/README.md")?;
            let text =
                std::str::from_utf8(&index.payload).map_err(|_| "document index is not UTF-8")?;
            if !text.contains(target.target.strip_prefix("doc/").unwrap()) {
                return Err(format!(
                    "migration document is not indexed: {}",
                    target.target
                ));
            }
        }
    }
    let encoded = serde_json::to_vec(manifest).map_err(|error| error.to_string())?;
    Ok(ValidatedMigration {
        schema_version: 1,
        status: "confirmed".into(),
        manifest_sha256: sha(&encoded),
        sources: scanned,
        targets: targets.into_values().collect(),
    })
}

#[cfg(all(test, bridgeforge_factory_tests))]
#[path = "../../../../../scripts/tests/unit/core_asset_migration.rs"]
mod tests;
