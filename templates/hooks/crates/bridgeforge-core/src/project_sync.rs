use crate::{CommandOutcome, ProcessRequest, ProcessRunner};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use walkdir::WalkDir;

#[path = "build_inputs.rs"]
pub(crate) mod build_inputs;

pub(crate) struct ProjectLock {
    _guard: crate::file_lock::FileLock,
}

impl ProjectLock {
    pub(crate) fn acquire(root: &Path) -> Result<Self, String> {
        let path = safe_join(
            root,
            ".runtime/bridgeforge-codex/project-sync.lock",
            "project lock",
        )?;
        let guard = crate::file_lock::FileLock::acquire(&path).map_err(|error| {
            format!("project-sync lock unavailable; another transaction may be active: {error}")
        })?;
        Ok(Self { _guard: guard })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SyncMode {
    Auto,
    Init,
    Adopt,
    Update,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SyncAction {
    pub id: String,
    pub target: String,
    pub operation: String,
    pub risk: bool,
    pub before_sha256: Option<String>,
    pub after_sha256: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SyncPlan {
    pub schema: u32,
    pub status: String,
    pub readiness: String,
    pub mode: SyncMode,
    pub previous_version: Option<String>,
    pub current_version: String,
    pub safe: Vec<SyncAction>,
    pub risk: Vec<SyncAction>,
    pub gaps: Vec<String>,
    pub blockers: Vec<String>,
    pub asset_migration: Value,
    pub preservation_manifest: Value,
    pub confirmation_required: bool,
    pub aggregate_fingerprint: String,
    #[serde(skip)]
    project_root: PathBuf,
    #[serde(skip)]
    writes: BTreeMap<PathBuf, Vec<u8>>,
    #[serde(skip)]
    deletes: Vec<PathBuf>,
    #[serde(skip)]
    generated_source_fingerprints: BTreeMap<String, String>,
    #[serde(skip)]
    project_hook_inputs: Vec<(crate::project_hooks::Hook, Vec<u8>)>,
    #[serde(skip)]
    project_hook_reads: BTreeMap<String, Option<Vec<u8>>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SyncReceipt {
    pub schema: u32,
    pub status: String,
    pub execution_status: String,
    pub mode: SyncMode,
    pub previous_version: Option<String>,
    pub current_version: String,
    pub aggregate_fingerprint: String,
    pub applied: Vec<SyncAction>,
    pub rollback_performed: bool,
    pub stamp_written_last: bool,
    pub project_readiness: String,
    pub asset_migration_manifest_sha256: Option<String>,
    pub preserved_asset_ids: Vec<String>,
}

fn sha_raw(payload: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(payload))
}

const LEGACY_RECEIPT: &str = ".codex/bin/build-receipt.json";
const RETIRED_SYNC_ROLE: &str = "mechanical-sync-worker";
const RETIRED_SYNC_TARGET: &str = ".codex/agents/mechanical-sync-worker.toml";
const RETIRED_SYNC_INPUT: &str = "retired:mechanical-sync-worker:inputs";
// Published payloads from 3645472, d3af932, and 0dea502; never trust a downstream manifest.
const RETIRED_SYNC_HASHES: &[&str] = &[
    "sha256:57315c89fc965f1fb13dfa611242658a17391a7ee379f7aee753a22deb989cc0",
    "sha256:3561c215ce3610421734e5173f911e38dc43657eee93272ec28ab520e6f8b871",
    "sha256:4c553960c7733ad76bed8bdb078709836df6a75560390e99f348d19918e80ecf",
];
const RETIRED_PROJECT_MAPS: &[(&str, &str)] = &[
    ("retired:project-map:find-doc", ".codex/find-doc.map.md"),
    ("retired:project-map:sync-docs", ".codex/sync-docs.map.md"),
];

fn legacy_receipt(root: &Path) -> Result<Option<(PathBuf, Vec<u8>)>, String> {
    let path = safe_join(root, LEGACY_RECEIPT, "legacy build receipt")?;
    for ancestor in path.ancestors().filter(|path| path.exists()) {
        if crate::memory::is_link_or_reparse(ancestor).map_err(|error| error.to_string())? {
            return Err("legacy build receipt traverses a link; refusing to delete".into());
        }
    }
    let metadata = match fs::symlink_metadata(&path) {
        Ok(value) => value,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.to_string()),
    };
    if !metadata.is_file()
        || crate::memory::is_link_or_reparse(&path).map_err(|error| error.to_string())?
    {
        return Err(
            "legacy build receipt is not a plain file; preserve it for manual inspection".into(),
        );
    }
    let bytes = fs::read(&path).map_err(|error| error.to_string())?;
    let value = crate::baseline::parse_unique_json(&bytes, "legacy build receipt")
        .map_err(|_| "unknown legacy build receipt; refusing to delete")?;
    let valid_hash = |key: &str| {
        value[key].as_str().is_some_and(|hash| {
            hash.strip_prefix("sha256:").is_some_and(|raw| {
                raw.len() == 64 && raw.bytes().all(|byte| byte.is_ascii_hexdigit())
            })
        })
    };
    if value.as_object().is_none_or(|object| object.len() != 9)
        || value["schema_version"] != 1
        || value["generated_asset_id"] != "codex.hooks"
        || !matches!(
            value["platform"].as_str(),
            Some("windows-x86_64" | "linux-x86_64" | "macos-aarch64" | "macos-x86_64")
        )
        || !value["cargo_version"]
            .as_str()
            .is_some_and(|text| text.starts_with("cargo ") && text.len() > 6)
        || ![
            "source_tree_sha256",
            "cargo_lock_sha256",
            "build_recipe_sha256",
            "self_test_sha256",
            "binary_sha256",
        ]
        .iter()
        .all(|key| valid_hash(key))
    {
        return Err("unknown legacy build receipt; refusing to delete".into());
    }
    Ok(Some((path, bytes)))
}

fn plan_legacy_receipt_retirement(
    root: &Path,
    safe: &mut Vec<SyncAction>,
    deletes: &mut Vec<PathBuf>,
) -> Result<(), String> {
    if let Some((path, bytes)) = legacy_receipt(root)? {
        safe.push(SyncAction {
            id: "retired:codex.hooks.build-receipt.v1".into(),
            target: LEGACY_RECEIPT.into(),
            operation: "delete".into(),
            risk: false,
            before_sha256: Some(sha_git(&bytes)),
            after_sha256: "sha256:deleted".into(),
        });
        deletes.push(path);
    }
    Ok(())
}

fn plan_project_map_retirement(
    root: &Path,
    safe: &mut Vec<SyncAction>,
    deletes: &mut Vec<PathBuf>,
) -> Result<(), String> {
    for (id, target) in RETIRED_PROJECT_MAPS {
        let path = safe_join(root, target, "retired project map")?;
        let Some(bytes) = transaction_file_state(&path)? else {
            continue;
        };
        safe.push(SyncAction {
            id: (*id).into(),
            target: (*target).into(),
            operation: "delete".into(),
            risk: false,
            before_sha256: Some(sha_git(&bytes)),
            after_sha256: "sha256:deleted".into(),
        });
        deletes.push(path);
    }
    Ok(())
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

fn sync_role_input(path: &Path) -> Result<Option<(String, bool)>, String> {
    validate_transaction_path(path)?;
    let mut file = match fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.to_string()),
    };
    if !file.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err(format!("role input is not a plain file: {}", path.display()));
    }
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 65536];
    let mut tail = Vec::new();
    let mut referenced = false;
    loop {
        let count = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if count == 0 { break; }
        digest.update(&buffer[..count]);
        tail.extend_from_slice(&buffer[..count]);
        referenced |= tail.windows(RETIRED_SYNC_ROLE.len())
            .any(|window| window == RETIRED_SYNC_ROLE.as_bytes());
        let keep = tail.len().saturating_sub(RETIRED_SYNC_ROLE.len() - 1);
        tail.drain(..keep);
    }
    Ok(Some((format!("sha256:{:x}", digest.finalize()), referenced)))
}

fn sync_role_inputs(root: &Path) -> Result<BTreeMap<String, (String, bool)>, String> {
    let mut inputs = BTreeMap::new();
    for target in ["AGENTS.md", ".codex/config.toml", RETIRED_SYNC_TARGET] {
        if let Some(input) = sync_role_input(&safe_join(root, target, "role input")?)? {
            inputs.insert(target.into(), input);
        }
    }
    for directory in [".codex/skills", ".codex/agents"] {
        let path = safe_join(root, directory, "role references")?;
        match fs::symlink_metadata(&path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.to_string()),
            Ok(metadata) if !metadata.is_dir() => {
                return Err(format!("role reference directory is not a plain directory: {directory}"));
            }
            Ok(_) => {}
        }
        for entry in WalkDir::new(&path).sort_by_file_name() {
            let entry = entry.map_err(|error| error.to_string())?;
            if crate::memory::is_link_or_reparse(entry.path()).map_err(|e| e.to_string())? {
                return Err(format!("linked role input is unsafe: {}", entry.path().display()));
            }
            if entry.file_type().is_file() {
                let extension = entry.path().extension().and_then(|s| s.to_str()).unwrap_or("").to_ascii_lowercase();
                if !matches!(extension.as_str(), "md" | "toml" | "json" | "yaml" | "yml" | "txt" | "rs" | "py" | "ps1" | "sh" | "cmd" | "bat" | "js" | "ts") {
                    continue;
                }
                let target = relative_posix(root, entry.path())?;
                let input = sync_role_input(entry.path())?
                    .ok_or_else(|| format!("role input disappeared: {target}"))?;
                inputs.insert(target, input);
            }
        }
    }
    Ok(inputs)
}

fn sync_role_references(inputs: &BTreeMap<String, (String, bool)>) -> Vec<String> {
    inputs.iter().filter_map(|(path, (_, referenced))| {
        (path != RETIRED_SYNC_TARGET && *referenced)
            .then(|| path.clone())
    }).collect()
}

fn sync_role_fingerprint(inputs: &BTreeMap<String, (String, bool)>) -> Result<String, String> {
    Ok(sha_raw(&serde_json::to_vec(inputs).map_err(|e| e.to_string())?))
}

fn plan_sync_role_retirement(
    root: &Path,
    safe: &mut Vec<SyncAction>,
    deletes: &mut Vec<PathBuf>,
    gaps: &mut Vec<String>,
    fingerprints: &mut BTreeMap<String, String>,
) -> Result<(), String> {
    let inputs = sync_role_inputs(root)?;
    fingerprints.insert(RETIRED_SYNC_INPUT.into(), sync_role_fingerprint(&inputs)?);
    let references = sync_role_references(&inputs);
    for path in &references {
        gaps.push(format!("retired role {RETIRED_SYNC_ROLE} is referenced by {path}; preserve and resolve before update"));
    }
    if !inputs.contains_key(RETIRED_SYNC_TARGET) { return Ok(()); }
    let path = safe_join(root, RETIRED_SYNC_TARGET, "retired role")?;
    // Known templates are below 4 KiB. Larger custom files need no in-memory payload.
    let hash = if fs::metadata(&path).map_err(|e| e.to_string())?.len() <= 4096 {
        let bytes = transaction_file_state(&path)?.ok_or("retired role disappeared")?;
        if sha_raw(&bytes) != inputs[RETIRED_SYNC_TARGET].0 {
            return Err("retired role changed while planning".into());
        }
        sha_git(&bytes)
    } else {
        String::new()
    };
    if !RETIRED_SYNC_HASHES.contains(&hash.as_str()) {
        gaps.push(format!("retired role has custom or unknown content: {RETIRED_SYNC_TARGET}; preserve for review"));
    } else if references.is_empty() {
        safe.push(SyncAction {
            id: "retired:codex.agent.mechanical-sync-worker".into(),
            target: RETIRED_SYNC_TARGET.into(),
            operation: "delete".into(),
            risk: false,
            before_sha256: Some(hash),
            after_sha256: "sha256:deleted".into(),
        });
        deletes.push(safe_join(root, RETIRED_SYNC_TARGET, "retired role")?);
    }
    Ok(())
}

fn plan_fingerprint(
    mode: &SyncMode,
    version: &str,
    safe: &[SyncAction],
    risk: &[SyncAction],
    preservation_manifest: &Value,
    generated_source_fingerprints: &BTreeMap<String, String>,
) -> Result<String, String> {
    let mut material = format!("schema=1\nmode={mode:?}\nversion={version}\n");
    for action in safe.iter().chain(risk.iter()) {
        material.push_str(&format!(
            "{}\t{}\t{}\t{}\n",
            action.id,
            action.target,
            action.before_sha256.as_deref().unwrap_or("missing"),
            generated_source_fingerprints
                .get(&action.id)
                .unwrap_or(&action.after_sha256)
        ));
    }
    material.push_str(
        &serde_json::to_string(preservation_manifest).map_err(|error| error.to_string())?,
    );
    if let Some(registry) = generated_source_fingerprints.get("project-hook:registry-input") {
        material.push_str(&format!("\nproject-hook:registry-input={registry}\n"));
    }
    if let Some(inputs) = generated_source_fingerprints.get(RETIRED_SYNC_INPUT) {
        material.push_str(&format!("\n{RETIRED_SYNC_INPUT}={inputs}\n"));
    }
    Ok(sha_raw(material.as_bytes()))
}

fn safe_join(root: &Path, raw: &str, label: &str) -> Result<PathBuf, String> {
    if raw.is_empty() || raw.contains('\\') || raw.contains(['*', '?', '[']) {
        return Err(format!("{label} is unsafe: {raw}"));
    }
    let relative = Path::new(raw);
    if relative.is_absolute()
        || relative.components().any(|part| {
            matches!(
                part,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(format!("{label} is unsafe: {raw}"));
    }
    let target = root.join(relative);
    let mut cursor = target.parent();
    while let Some(path) = cursor {
        if path == root {
            break;
        }
        if path.exists()
            && fs::symlink_metadata(path)
                .map_err(|error| error.to_string())?
                .file_type()
                .is_symlink()
        {
            return Err(format!("{label} traverses a linked path: {raw}"));
        }
        cursor = path.parent();
    }
    Ok(target)
}

fn atomic_write(path: &Path, payload: &[u8]) -> Result<(), String> {
    crate::memory::atomic_write(path, payload).map_err(|error| error.to_string())
}

fn marker_replace(
    current: &[u8],
    source: &[u8],
    begin: &str,
    end: &str,
) -> Result<Vec<u8>, String> {
    let find = |payload: &[u8]| -> Result<(usize, usize), String> {
        let text =
            String::from_utf8(payload.to_vec()).map_err(|_| "managed region is not UTF-8")?;
        let start = text
            .find(begin)
            .ok_or_else(|| format!("managed begin marker is missing: {begin}"))?;
        let tail = &text[start..];
        let stop = tail
            .find(end)
            .map(|index| start + index + end.len())
            .ok_or_else(|| format!("managed end marker is missing: {end}"))?;
        if text[start + begin.len()..].contains(begin) || text[stop..].contains(end) {
            return Err("managed markers must appear exactly once".into());
        }
        Ok((start, stop))
    };
    let (current_start, current_stop) = find(current)?;
    let (source_start, source_stop) = find(source)?;
    let mut result = Vec::new();
    result.extend_from_slice(&current[..current_start]);
    result.extend_from_slice(&source[source_start..source_stop]);
    result.extend_from_slice(&current[current_stop..]);
    Ok(result)
}

fn merge_json(required: &Value, actual: &mut Value) {
    match (required, actual) {
        (Value::Object(required), Value::Object(actual)) => {
            for (key, value) in required {
                match actual.get_mut(key) {
                    Some(current) => merge_json(value, current),
                    None => {
                        actual.insert(key.clone(), value.clone());
                    }
                }
            }
        }
        (Value::Array(required), Value::Array(actual)) => {
            for value in required {
                if !actual.contains(value) {
                    actual.push(value.clone());
                }
            }
        }
        (required, actual) => *actual = required.clone(),
    }
}

fn render_source(source: &[u8], asset: &Value, project_root: &Path) -> Result<Vec<u8>, String> {
    if asset["render"].as_str() != Some("project-name") {
        return Ok(source.to_vec());
    }
    let text = String::from_utf8(source.to_vec())
        .map_err(|_| format!("asset {} render source is not UTF-8", asset["id"]))?;
    let name = project_root
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or("project root has no UTF-8 name")?;
    Ok(text.replace("{{PROJECT_NAME}}", name).into_bytes())
}

fn marker_span(payload: &[u8], begin: &str, end: &str) -> Result<(usize, usize), String> {
    let text = String::from_utf8(payload.to_vec()).map_err(|_| "managed content is not UTF-8")?;
    let starts = text
        .match_indices(begin)
        .map(|item| item.0)
        .collect::<Vec<_>>();
    let stops = text
        .match_indices(end)
        .map(|item| item.0)
        .collect::<Vec<_>>();
    if starts.len() != 1 || stops.len() != 1 || starts[0] >= stops[0] {
        return Err(format!(
            "managed markers are missing, duplicated, or reversed: {begin} / {end}"
        ));
    }
    Ok((starts[0], stops[0] + end.len()))
}

fn merge_agents(
    source: &[u8],
    current: Option<&[u8]>,
    asset: &Value,
    project_root: &Path,
) -> Result<Vec<u8>, String> {
    let canonical = render_source(source, asset, project_root)?;
    let Some(current) = current else {
        return Ok(canonical);
    };
    let project = &asset["agents_zones"]["project"];
    let begin = project["begin"]
        .as_str()
        .ok_or("project zone begin is missing")?;
    let end = project["end"]
        .as_str()
        .ok_or("project zone end is missing")?;
    let (canonical_start, canonical_stop) = marker_span(&canonical, begin, end)?;
    let (current_start, current_stop) = marker_span(current, begin, end)?;
    let mut merged = Vec::new();
    merged.extend_from_slice(&canonical[..canonical_start]);
    merged.extend_from_slice(&current[current_start..current_stop]);
    merged.extend_from_slice(&canonical[canonical_stop..]);
    Ok(merged)
}

fn markdown_section(text: &str, heading: &str) -> Result<(usize, usize), String> {
    optional_markdown_section(text, heading)?
        .ok_or_else(|| format!("managed Markdown heading is missing or duplicated: {heading}"))
}

fn optional_markdown_section(text: &str, heading: &str) -> Result<Option<(usize, usize)>, String> {
    let mut found = Vec::new();
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        if line.trim_end_matches(['\r', '\n']) == heading {
            found.push(offset);
        }
        offset += line.len();
    }
    if found.is_empty() {
        return Ok(None);
    }
    if found.len() > 1 {
        return Err(format!(
            "managed Markdown heading is missing or duplicated: {heading}"
        ));
    }
    let start = found[0];
    let mut end = text.len();
    let mut cursor = start;
    for line in text[start..].split_inclusive('\n') {
        if cursor > start && line.starts_with("## ") {
            end = cursor;
            break;
        }
        cursor += line.len();
    }
    Ok(Some((start, end)))
}

fn ensure_markdown_section(
    merged: &mut String,
    source: &str,
    heading: &str,
    allow_upgrade: bool,
) -> Result<(usize, usize), String> {
    if let Some(span) = optional_markdown_section(merged, heading)? {
        return Ok(span);
    }
    if !allow_upgrade {
        return markdown_section(merged, heading);
    }
    let (start, end) = markdown_section(source, heading)?;
    let mut insertion = merged.len();
    for line in source[end..].lines().filter(|line| line.starts_with("## ")) {
        if let Some((offset, _)) = optional_markdown_section(merged, line)? {
            insertion = offset;
            break;
        }
    }
    let prefix = if insertion > 0 && !merged[..insertion].ends_with('\n') {
        "\n\n"
    } else {
        ""
    };
    merged.insert_str(insertion, &format!("{prefix}{}", &source[start..end]));
    markdown_section(merged, heading)
}

fn table_header(section: &str) -> Result<(&str, &str, usize), String> {
    let lines = section.split_inclusive('\n').collect::<Vec<_>>();
    let separators = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| {
            let cells = line.trim().trim_matches('|').split('|').collect::<Vec<_>>();
            line.trim_start().starts_with('|')
                && cells.iter().all(|cell| {
                    let cell = cell.trim().trim_matches(':');
                    cell.len() >= 3 && cell.bytes().all(|byte| byte == b'-')
                })
        })
        .collect::<Vec<_>>();
    if separators.len() != 1 || separators[0].0 == 0 {
        return Err("managed Markdown table is missing or ambiguous".into());
    }
    let (index, separator) = separators[0];
    let header = lines[index - 1];
    let columns = separator.trim().trim_matches('|').split('|').count();
    if !header.trim_start().starts_with('|')
        || header.trim().trim_matches('|').split('|').count() != columns
    {
        return Err("managed Markdown table header column count changed".into());
    }
    Ok((header, separator, columns))
}

fn table_ranges(section: &str) -> Result<Vec<(usize, usize)>, String> {
    let lines = section.split_inclusive('\n').collect::<Vec<_>>();
    let mut offsets = Vec::with_capacity(lines.len() + 1);
    offsets.push(0);
    for line in &lines {
        offsets.push(offsets.last().unwrap() + line.len());
    }
    let mut ranges = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        let cells = line.trim().trim_matches('|').split('|').collect::<Vec<_>>();
        let separator = line.trim_start().starts_with('|')
            && cells.iter().all(|cell| {
                let cell = cell.trim().trim_matches(':');
                cell.len() >= 3 && cell.bytes().all(|byte| byte == b'-')
            });
        if !separator {
            continue;
        }
        if index == 0 {
            return Err("managed Markdown table is missing or ambiguous".into());
        }
        let mut end = index + 1;
        while end < lines.len() && lines[end].trim_start().starts_with('|') {
            end += 1;
        }
        ranges.push((offsets[index - 1], offsets[end]));
    }
    Ok(ranges)
}

fn table_key(line: &str) -> Option<String> {
    if !line.trim_start().starts_with('|') {
        return None;
    }
    let raw = line.split('|').nth(1)?.trim();
    let key = if let Some((_, href)) = raw.split_once("](") {
        href.split(')').next().unwrap_or(href).to_string()
    } else if let Some(rest) = raw.strip_prefix("[`") {
        rest.split("`]").next().unwrap_or(rest).to_string()
    } else {
        raw.trim_matches('`').to_string()
    };
    (!key.is_empty() && !key.chars().all(|value| value == '-' || value == ':')).then_some(key)
}

fn merge_keyed_table(
    source_section: &str,
    current_section: &str,
    keys: &BTreeSet<String>,
    allow_upgrade: bool,
) -> Result<String, String> {
    let target_ranges = table_ranges(current_section)?;
    if target_ranges.len() > 1 {
        let candidates = target_ranges
            .iter()
            .filter(|(start, end)| {
                current_section[*start..*end]
                    .split_inclusive('\n')
                    .filter_map(table_key)
                    .any(|key| keys.contains(&key))
            })
            .copied()
            .collect::<Vec<_>>();
        if candidates.len() != 1 {
            return Err("managed Markdown table is missing or ambiguous".into());
        }
        let (start, end) = candidates[0];
        let table = merge_keyed_table(
            source_section,
            &current_section[start..end],
            keys,
            allow_upgrade,
        )?;
        return Ok(format!(
            "{}{}{}",
            &current_section[..start],
            table,
            &current_section[end..]
        ));
    }
    let (source_header, source_separator, columns) = table_header(source_section)?;
    let (current_header, current_separator, current_columns) = table_header(current_section)?;
    if columns != current_columns {
        return Err("managed Markdown table column count changed".into());
    }
    let upgraded;
    let current_section = if allow_upgrade {
        upgraded = current_section
            .replacen(current_header, source_header, 1)
            .replacen(current_separator, source_separator, 1);
        upgraded.as_str()
    } else {
        if current_header.trim() != source_header.trim() {
            return Err("managed Markdown table header drifted".into());
        }
        current_section
    };
    let mut required = BTreeMap::<String, String>::new();
    for line in source_section.split_inclusive('\n') {
        if let Some(key) = table_key(line)
            && keys.contains(&key)
        {
            if required.insert(key.clone(), line.to_string()).is_some() {
                return Err(format!("managed Markdown source row is duplicated: {key}"));
            }
        }
    }
    if required.len() != keys.len() {
        let missing = keys
            .difference(&required.keys().cloned().collect())
            .cloned()
            .collect::<Vec<_>>();
        return Err(format!(
            "managed Markdown source rows are missing: {missing:?}"
        ));
    }
    let mut rendered = Vec::<String>::new();
    let mut seen = BTreeSet::new();
    let mut insertion = None;
    for line in current_section.split_inclusive('\n') {
        if insertion.is_none() && line.trim() == source_separator.trim() {
            rendered.push(line.to_string());
            insertion = Some(rendered.len());
            continue;
        }
        if let Some(key) = table_key(line)
            && keys.contains(&key)
        {
            if !seen.insert(key.clone()) {
                return Err(format!("managed Markdown target row is duplicated: {key}"));
            }
            rendered.push(required[&key].clone());
        } else {
            rendered.push(line.to_string());
        }
    }
    let insert_at = insertion.ok_or("managed Markdown table separator is missing")?;
    for key in keys {
        if !seen.contains(key) {
            rendered.insert(insert_at, required[key].clone());
        }
    }
    Ok(rendered.concat())
}

fn merge_managed_markdown(
    source: &[u8],
    current: &[u8],
    asset: &Value,
    project_root: &Path,
    allow_upgrade: bool,
) -> Result<Vec<u8>, String> {
    let source = String::from_utf8(render_source(source, asset, project_root)?)
        .map_err(|_| "managed Markdown source is not UTF-8")?;
    let mut merged =
        String::from_utf8(current.to_vec()).map_err(|_| "managed Markdown target is not UTF-8")?;
    for heading in asset["managed_blocks"]["headings"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
    {
        let (source_start, source_end) = markdown_section(&source, heading)?;
        let (target_start, target_end) =
            ensure_markdown_section(&mut merged, &source, heading, allow_upgrade)?;
        merged.replace_range(target_start..target_end, &source[source_start..source_end]);
    }
    for contract in asset["managed_blocks"]["keyed_tables"]
        .as_array()
        .into_iter()
        .flatten()
    {
        let heading = contract["heading"]
            .as_str()
            .ok_or("keyed table heading is missing")?;
        let keys = contract["managed_keys"]
            .as_array()
            .ok_or("managed table keys are missing")?
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect::<BTreeSet<_>>();
        let (source_start, source_end) = markdown_section(&source, heading)?;
        let (target_start, target_end) =
            ensure_markdown_section(&mut merged, &source, heading, allow_upgrade)?;
        let section = merge_keyed_table(
            &source[source_start..source_end],
            &merged[target_start..target_end],
            &keys,
            allow_upgrade,
        )?;
        merged.replace_range(target_start..target_end, &section);
    }
    Ok(merged.into_bytes())
}

fn retired_managed_hook(command: &str) -> bool {
    const RETIRED: &[&str] = &[
        "hook_dispatcher.py",
        "cargo_default_run_check.py",
        "config_health_check.py",
        "cross_project_write_guard.py",
        "encoding_check.py",
        "enforce_no_effortlevel.py",
        "fallback_smell_check.py",
        "git_add_all_guard.py",
        "githooks_path_check.py",
        "instruction_source_check.py",
        "non_ascii_shell_guard.py",
        "project_structure_check.py",
        "requirements_check.py",
        "session_snapshot.py",
        "show_state.py",
        "skill_metadata_check.py",
        "skill_sync_check.py",
        "test_receipt.py",
        "user_config_write_guard.py",
    ];
    RETIRED.iter().any(|name| command.contains(name))
}

fn merge_project_hooks(source: &[u8], current: Option<&[u8]>) -> Result<Vec<u8>, String> {
    let canonical: Value = serde_json::from_slice(source)
        .map_err(|error| format!("canonical project hooks are invalid: {error}"))?;
    let Some(current) = current else {
        let mut bytes = serde_json::to_vec_pretty(&canonical).map_err(|error| error.to_string())?;
        bytes.push(b'\n');
        return Ok(bytes);
    };
    let local: Value = serde_json::from_slice(current)
        .map_err(|error| format!("project hooks target is invalid: {error}"))?;
    let mut external = BTreeMap::<String, Vec<Value>>::new();
    let hooks = local["hooks"]
        .as_object()
        .ok_or("project hooks target has no hooks object")?;
    for (event, groups) in hooks {
        for group in groups
            .as_array()
            .ok_or("project hook event is not an array")?
        {
            let mut kept = Vec::new();
            for handler in group["hooks"]
                .as_array()
                .ok_or("project hook group has no hooks array")?
            {
                let managed_id = handler["bridgeforgeCodexId"].as_str();
                if managed_id.is_some_and(|id| id.starts_with("bridgeforge-codex.project-hook.v1:"))
                {
                    continue;
                }
                if managed_id.is_some() {
                    return Err(format!(
                        "project hook has a non-canonical managed identity: {event}"
                    ));
                }
                if handler["command"]
                    .as_str()
                    .is_some_and(retired_managed_hook)
                {
                    continue;
                }
                kept.push(handler.clone());
            }
            if !kept.is_empty() {
                let mut group = group.clone();
                group["hooks"] = Value::Array(kept);
                external.entry(event.clone()).or_default().push(group);
            }
        }
    }
    let mut result = local;
    result["hooks"] = canonical["hooks"].clone();
    let result_hooks = result["hooks"]
        .as_object_mut()
        .ok_or("canonical hooks object is invalid")?;
    for (event, groups) in external {
        result_hooks
            .entry(event)
            .or_insert_with(|| Value::Array(Vec::new()))
            .as_array_mut()
            .ok_or("canonical hook event is not an array")?
            .extend(groups);
    }
    if let Some(metadata) = canonical.get("bridgeforgeCodex") {
        result["bridgeforgeCodex"] = metadata.clone();
    }
    let mut bytes = serde_json::to_vec_pretty(&result).map_err(|error| error.to_string())?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn render_asset(
    project_root: &Path,
    template_root: &Path,
    asset: &Value,
    migration_payload: Option<&[u8]>,
    allow_structure_upgrade: bool,
) -> Result<(Vec<u8>, Option<Vec<u8>>), String> {
    let id = asset["id"].as_str().ok_or("asset id is missing")?;
    let source_raw = asset["source"]
        .as_str()
        .ok_or_else(|| format!("asset source is missing: {id}"))?;
    let target_raw = asset["target"]
        .as_str()
        .ok_or_else(|| format!("asset target is missing: {id}"))?;
    let source = safe_join(template_root, source_raw, "asset source")?;
    let target = safe_join(project_root, target_raw, "asset target")?;
    let source_payload = fs::read(&source)
        .map_err(|error| format!("cannot read asset source {source_raw}: {error}"))?;
    let current = if target.is_file() {
        Some(fs::read(&target).map_err(|error| error.to_string())?)
    } else {
        None
    };
    let strategy = asset["strategy"].as_str().unwrap_or("whole");
    let effective_current = migration_payload.or(current.as_deref());
    let rendered = if asset.get("agents_zones").is_some() {
        merge_agents(&source_payload, effective_current, asset, project_root)?
    } else if asset.get("managed_blocks").is_some() && effective_current.is_some() {
        merge_managed_markdown(
            &source_payload,
            effective_current.unwrap(),
            asset,
            project_root,
            allow_structure_upgrade,
        )?
    } else if asset["merge_policy"].as_str() == Some("codex-hooks") {
        merge_project_hooks(&source_payload, effective_current)?
    } else {
        match (strategy, effective_current) {
            ("seed", Some(payload)) => payload.to_vec(),
            ("seed", None) => render_source(&source_payload, asset, project_root)?,
            ("whole", _) => render_source(&source_payload, asset, project_root)?,
            ("region", Some(payload)) => {
                let region = &asset["region"];
                marker_replace(
                    payload,
                    &source_payload,
                    region["begin"].as_str().ok_or("region begin is missing")?,
                    region["end"].as_str().ok_or("region end is missing")?,
                )?
            }
            ("region", None) => source_payload,
            ("merge", Some(payload)) if asset["merge_policy"] == "git-attributes-default-lf" => {
                let text = String::from_utf8(payload.to_vec())
                    .map_err(|_| ".gitattributes is not UTF-8")?;
                if text.lines().any(|line| line.trim() == "* text=auto eol=lf") {
                    payload.to_vec()
                } else {
                    let mut result = text;
                    if !result.is_empty() && !result.ends_with('\n') {
                        result.push('\n');
                    }
                    result.push_str("* text=auto eol=lf\n");
                    result.into_bytes()
                }
            }
            ("merge", Some(payload)) => {
                let required: Value = serde_json::from_slice(&source_payload)
                    .map_err(|error| format!("managed merge source is invalid: {id}: {error}"))?;
                let mut actual: Value = serde_json::from_slice(payload)
                    .map_err(|error| format!("managed merge target is invalid: {id}: {error}"))?;
                merge_json(&required, &mut actual);
                let mut bytes =
                    serde_json::to_vec_pretty(&actual).map_err(|error| error.to_string())?;
                bytes.push(b'\n');
                bytes
            }
            ("merge", None) => source_payload,
            (_, _) => return Err(format!("unsupported asset strategy: {id}: {strategy}")),
        }
    };
    Ok((rendered, current))
}

fn read_contract(template_root: &Path) -> Result<(Value, Vec<u8>), String> {
    let path = template_root.join("templates/managed-skeleton.json");
    let bytes =
        fs::read(&path).map_err(|error| format!("cannot read managed contract: {error}"))?;
    let value = crate::baseline::parse_unique_json(&bytes, "managed contract")?;
    if value["schema_version"].as_u64() != Some(4)
        || value["baseline_model"].as_str() != Some("current-only")
    {
        return Err("managed contract must use schema 4 current-only".into());
    }
    crate::baseline::compatibility_baseline(&value)?;
    Ok((value, bytes))
}

fn resolved_mode(project_root: &Path, requested: SyncMode) -> Result<SyncMode, String> {
    let has_stamp = project_identity(project_root)?.is_some();
    let has_assets =
        project_root.join(".codex").exists() || project_root.join("AGENTS.md").exists();
    match requested {
        SyncMode::Auto => Ok(if has_stamp {
            SyncMode::Update
        } else if has_assets {
            SyncMode::Adopt
        } else {
            SyncMode::Init
        }),
        SyncMode::Init if has_assets || has_stamp => {
            Err("init requires a project with no existing skeleton identity".into())
        }
        SyncMode::Update if !has_stamp => {
            Err("update requires one valid version stamp; use adopt for unstamped assets".into())
        }
        SyncMode::Adopt if has_stamp || !has_assets => {
            Err("adopt requires unstamped existing skeleton assets".into())
        }
        mode => Ok(mode),
    }
}

fn project_identity(root: &Path) -> Result<Option<(String, bool)>, String> {
    let mut identity = None;
    for (name, legacy) in [
        (".bridgeforge_codex_version", false),
        (".bridgeforge_version", true),
    ] {
        let path = root.join(".codex").join(name);
        let metadata = match fs::symlink_metadata(&path) {
            Ok(value) => value,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.to_string()),
        };
        if !metadata.is_file()
            || crate::memory::is_link_or_reparse(&path).map_err(|e| e.to_string())?
        {
            return Err("version stamp must be a plain file".into());
        }
        if identity.is_some() {
            return Err("multiple version stamps are not allowed".into());
        }
        let version = fs::read_to_string(&path)
            .map_err(|e| e.to_string())?
            .trim()
            .to_string();
        version.parse::<crate::release::SemVer>()?;
        identity = Some((version, legacy));
    }
    Ok(identity)
}

fn generated_asset_current(project_root: &Path, item: &Value) -> Result<bool, String> {
    let platform = if cfg!(windows) {
        "windows-x86_64"
    } else if cfg!(target_os = "linux") {
        "linux-x86_64"
    } else {
        "macos-x86_64"
    };
    let binary = safe_join(
        project_root,
        item["binary_targets"][platform]
            .as_str()
            .ok_or("generated binary target is missing")?,
        "generated binary target",
    )?;
    let receipt = safe_join(
        project_root,
        item["receipt_target"]
            .as_str()
            .ok_or("generated receipt target is missing")?,
        "generated receipt target",
    )?;
    let Ok(binary_payload) = fs::read(binary) else {
        return Ok(false);
    };
    let Ok(receipt_payload) = fs::read(receipt) else {
        return Ok(false);
    };
    let Ok(document) = serde_json::from_slice::<Value>(&receipt_payload) else {
        return Ok(false);
    };
    Ok(document["schema_version"].as_u64() == Some(2)
        && document["generated_asset_id"] == item["id"]
        && document["platform"].as_str() == Some(platform)
        && document["binary_sha256"].as_str() == Some(sha_raw(&binary_payload).as_str())
        && document["source_tree_sha256"] == item["source_tree_sha256"]
        && document["lockfile_sha256"] == item["lockfile_sha256"]
        && document["build_recipe_sha256"] == item["build_recipe_sha256"]
        && document["self_test_sha256"] == item["self_test_sha256"])
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

fn current_platform() -> &'static str {
    if cfg!(windows) {
        "windows-x86_64"
    } else if cfg!(target_os = "linux") {
        "linux-x86_64"
    } else {
        "macos-x86_64"
    }
}

fn relative_posix(root: &Path, path: &Path) -> Result<String, String> {
    path.strip_prefix(root)
        .map_err(|_| format!("path escaped project root: {}", path.display()))
        .map(|relative| relative.to_string_lossy().replace('\\', "/"))
}

fn tree_sha(root: &Path) -> Result<String, String> {
    let mut material = Vec::new();
    for entry in WalkDir::new(root).sort_by_file_name() {
        let entry = entry.map_err(|error| error.to_string())?;
        if entry.file_type().is_symlink() {
            return Err(format!(
                "linked project asset is unsafe: {}",
                entry.path().display()
            ));
        }
        if entry.file_type().is_file() {
            let relative = relative_posix(root, entry.path())?;
            material.extend_from_slice(relative.as_bytes());
            material.push(0);
            material.extend_from_slice(&fs::read(entry.path()).map_err(|error| error.to_string())?);
            material.push(0xff);
        }
    }
    Ok(sha_raw(&material))
}

struct DestructiveInventory {
    entries: Vec<Value>,
    actions: Vec<SyncAction>,
    deletes: Vec<PathBuf>,
    blockers: Vec<String>,
    candidate_ids: BTreeSet<String>,
    deleted_hook_prefixes: Vec<String>,
}

fn remove_hook_registrations(payload: &[u8], prefixes: &[String]) -> Result<Vec<u8>, String> {
    let mut document: Value = serde_json::from_slice(payload)
        .map_err(|error| format!("project hooks target is invalid: {error}"))?;
    let hooks = document["hooks"]
        .as_object_mut()
        .ok_or("project hooks target has no hooks object")?;
    for groups in hooks.values_mut() {
        let groups = groups
            .as_array_mut()
            .ok_or("project hook event is not an array")?;
        for group in groups.iter_mut() {
            let handlers = group["hooks"]
                .as_array_mut()
                .ok_or("project hook group has no hooks array")?;
            handlers.retain(|handler| {
                !["command", "commandWindows"].iter().any(|key| {
                    handler[*key].as_str().is_some_and(|command| {
                        let normalized = command.replace('\\', "/");
                        prefixes.iter().any(|prefix| {
                            let (command, prefix) = if *key == "commandWindows" || cfg!(windows) {
                                (normalized.to_lowercase(), prefix.to_lowercase())
                            } else {
                                (normalized.clone(), prefix.clone())
                            };
                            command.match_indices(&prefix).any(|(index, _)| {
                                let boundary = |ch: char| {
                                    ch.is_whitespace()
                                        || matches!(ch, '/' | '\'' | '"' | '=')
                                        || ch as u32 == 96
                                };
                                command[..index].chars().next_back().is_none_or(boundary)
                                    && command[index + prefix.len()..]
                                        .chars()
                                        .next()
                                        .is_none_or(boundary)
                            })
                        })
                    })
                })
            });
        }
        groups.retain(|group| {
            group["hooks"]
                .as_array()
                .is_some_and(|items| !items.is_empty())
        });
    }
    let mut encoded = serde_json::to_vec_pretty(&document).map_err(|error| error.to_string())?;
    encoded.push(b'\n');
    Ok(encoded)
}

fn destructive_inventory(
    project_root: &Path,
    contract: &Value,
    migration_sources: &[crate::asset_migration::SourceAsset],
    known_existing: &[String],
    preserve_ids: &BTreeSet<String>,
    delete_ids: &BTreeSet<String>,
) -> Result<DestructiveInventory, String> {
    let mut entries = Vec::new();
    let mut actions = Vec::new();
    let mut deletes = Vec::new();
    let mut blockers = Vec::new();
    let mut candidate_ids = BTreeSet::new();
    let mut deleted_hook_prefixes = Vec::new();
    let mut exact = BTreeSet::from([
        RETIRED_SYNC_TARGET.to_string(),
        ".codex/.bridgeforge_version".to_string(),
        contract["stamp"]
            .as_str()
            .unwrap_or_default()
            .to_lowercase(),
        contract["contract_target"]
            .as_str()
            .unwrap_or_default()
            .to_lowercase(),
    ]);
    exact.extend(
        RETIRED_PROJECT_MAPS
            .iter()
            .map(|(_, target)| target.to_lowercase()),
    );
    for asset in contract["assets"].as_array().into_iter().flatten() {
        if let Some(target) = asset["target"].as_str() {
            exact.insert(target.to_lowercase());
        }
    }
    for generated in contract["generated_assets"]
        .as_array()
        .into_iter()
        .flatten()
    {
        if let Some(target) = generated["binary_targets"][current_platform()].as_str() {
            exact.insert(target.to_lowercase());
        }
        if let Some(target) = generated["receipt_target"].as_str() {
            exact.insert(target.to_lowercase());
        }
    }
    for source in migration_sources {
        exact.insert(source.source_path.to_lowercase());
    }
    exact.extend(known_existing.iter().map(|target| target.to_lowercase()));

    let mut required_prefixes = Vec::<String>::new();
    let registry_target = crate::project_hooks::REGISTRY_PATH;
    if let Some(payload) = crate::project_hooks::read(project_root, registry_target)? {
        if !exact.contains(&registry_target.to_lowercase()) {
            entries.push(json!({
                "id": "R:project-hook-registry", "kind": "project-hook-registry",
                "target": registry_target, "before_sha256": sha_git(&payload),
                "disposition": "required-preserve"
            }));
            exact.insert(registry_target.to_lowercase());
        }
    }
    let skills = project_root.join(".codex/skills");
    if skills.exists() {
        if !skills.is_dir()
            || fs::symlink_metadata(&skills)
                .map_err(|error| error.to_string())?
                .file_type()
                .is_symlink()
        {
            blockers.push("required-preserve .codex/skills is not a plain directory".into());
        } else {
            entries.push(json!({
                "id": "R:skills",
                "kind": "skills",
                "target": ".codex/skills",
                "before_sha256": tree_sha(&skills)?,
                "disposition": "required-preserve"
            }));
            required_prefixes.push(".codex/skills".into());
        }
    }

    let mut selectable_prefixes = Vec::<(String, String)>::new();
    let hooks_root = project_root.join(".codex/hooks");
    if hooks_root.is_dir() {
        for child in fs::read_dir(&hooks_root).map_err(|error| error.to_string())? {
            let child = child.map_err(|error| error.to_string())?.path();
            let Some(name) = child.file_name().and_then(|value| value.to_str()) else {
                continue;
            };
            if !name.starts_with("project_") {
                continue;
            }
            let relative = relative_posix(project_root, &child)?;
            let linked =
                crate::memory::is_link_or_reparse(&child).map_err(|error| error.to_string())?;
            if child.is_file() && !linked {
                // Plain project_* files are handled by the exact-file inventory below.
                continue;
            }
            if !child.is_dir() || linked {
                blockers.push(format!(
                    "project hook bundle must be a plain project_* directory: {relative}"
                ));
                continue;
            }
            let id = format!("P:project-hook-bundle:{relative}");
            candidate_ids.insert(id.clone());
            let before_sha = tree_sha(&child)?;
            if delete_ids.contains(&id) {
                deleted_hook_prefixes.push(relative.clone());
                for item in WalkDir::new(&child).sort_by_file_name() {
                    let item = item.map_err(|error| error.to_string())?;
                    if !item.file_type().is_file() {
                        continue;
                    }
                    let target = relative_posix(project_root, item.path())?;
                    let payload = fs::read(item.path()).map_err(|error| error.to_string())?;
                    deletes.push(item.path().to_path_buf());
                    actions.push(SyncAction {
                        id: format!("rebuild.remove:{id}:{target}"),
                        target,
                        operation: "delete".into(),
                        risk: true,
                        before_sha256: Some(sha_git(&payload)),
                        after_sha256: "sha256:deleted".into(),
                    });
                }
            }
            entries.push(json!({
                "id": id,
                "kind": "project-hook-bundle",
                "target": relative.clone(),
                "before_sha256": before_sha,
                "disposition": if delete_ids.contains(&id) {"delete"} else if preserve_ids.contains(&id) {"preserve"} else {"user-decision"}
            }));
            selectable_prefixes.push((id, relative));
        }
    }
    let rules_root = project_root.join(".codex/rules");
    if rules_root.is_dir() {
        for entry in WalkDir::new(&rules_root).sort_by_file_name() {
            let entry = entry.map_err(|error| error.to_string())?;
            if !entry.file_type().is_file() {
                continue;
            }
            let relative = relative_posix(project_root, entry.path())?;
            if relative.ends_with(".md") || exact.contains(&relative.to_lowercase()) {
                continue;
            }
            if !relative.ends_with(".rules") {
                blockers.push(format!(
                    "unknown .codex rule structure must be classified before rebuild: {relative}"
                ));
                continue;
            }
            let id = format!("P:rule:{relative}");
            candidate_ids.insert(id.clone());
            exact.insert(relative.to_lowercase());
            let before_sha = sha_git(&fs::read(entry.path()).map_err(|error| error.to_string())?);
            let disposition = if delete_ids.contains(&id) {
                deletes.push(entry.path().to_path_buf());
                actions.push(SyncAction {
                    id: format!("rebuild.remove:{id}"),
                    target: relative.clone(),
                    operation: "delete".into(),
                    risk: true,
                    before_sha256: Some(before_sha.clone()),
                    after_sha256: "sha256:deleted".into(),
                });
                "delete"
            } else if preserve_ids.contains(&id) {
                "preserve"
            } else {
                "user-decision"
            };
            entries.push(json!({
                "id": id,
                "kind": "rule",
                "target": relative,
                "before_sha256": before_sha,
                "disposition": disposition
            }));
        }
    }

    if project_root.join(".codex").is_dir() {
        for entry in WalkDir::new(project_root.join(".codex")).sort_by_file_name() {
            let entry = entry.map_err(|error| error.to_string())?;
            let relative = relative_posix(project_root, entry.path())?;
            if relative == ".codex" {
                continue;
            }
            if entry.file_type().is_symlink() {
                blockers.push(format!(
                    "unknown or unsafe .codex structure blocks rebuild: {relative}"
                ));
                continue;
            }
            let folded = relative.to_lowercase();
            let covered = exact.contains(&folded)
                || required_prefixes.iter().any(|prefix| {
                    relative == *prefix || relative.starts_with(&format!("{prefix}/"))
                })
                || selectable_prefixes.iter().any(|(_, prefix)| {
                    relative == *prefix || relative.starts_with(&format!("{prefix}/"))
                });
            let ancestor = exact
                .iter()
                .any(|target| target.starts_with(&(folded.clone() + "/")))
                || required_prefixes
                    .iter()
                    .any(|target| target.starts_with(&(relative.clone() + "/")))
                || selectable_prefixes
                    .iter()
                    .any(|(_, target)| target.starts_with(&(relative.clone() + "/")));
            if !covered && !ancestor && entry.file_type().is_file() {
                // Existing files are project decisions, never ownership proved by an old manifest.
                let id = format!("P:project-file:{relative}");
                candidate_ids.insert(id.clone());
                let before_sha = sha_git(&fs::read(entry.path()).map_err(|e| e.to_string())?);
                let disposition = if delete_ids.contains(&id) {
                    if relative.starts_with(".codex/hooks/project_")
                        && project_root.join(".codex/hooks.json").is_file()
                    {
                        deleted_hook_prefixes.push(relative.clone());
                    }
                    deletes.push(entry.path().to_path_buf());
                    actions.push(SyncAction {
                        id: format!("rebuild.remove:{id}"),
                        target: relative.clone(),
                        operation: "delete".into(),
                        risk: true,
                        before_sha256: Some(before_sha.clone()),
                        after_sha256: "sha256:deleted".into(),
                    });
                    "delete"
                } else if preserve_ids.contains(&id) {
                    "preserve"
                } else {
                    "user-decision"
                };
                entries.push(json!({"id": id, "kind": "project-file", "target": relative,
                    "before_sha256": before_sha, "disposition": disposition}));
            }
        }
    }
    Ok(DestructiveInventory {
        entries,
        actions,
        deletes,
        blockers,
        candidate_ids,
        deleted_hook_prefixes,
    })
}

pub fn build_plan(
    project_root: &Path,
    template_root: &Path,
    requested_mode: SyncMode,
) -> Result<SyncPlan, String> {
    build_plan_with_inputs(project_root, template_root, requested_mode, None, None)
}

pub fn build_plan_with_migration(
    project_root: &Path,
    template_root: &Path,
    requested_mode: SyncMode,
    migration_manifest: Option<&Value>,
) -> Result<SyncPlan, String> {
    build_plan_with_inputs(
        project_root,
        template_root,
        requested_mode,
        migration_manifest,
        None,
    )
}

pub fn build_plan_with_inputs(
    project_root: &Path,
    template_root: &Path,
    requested_mode: SyncMode,
    migration_manifest: Option<&Value>,
    preservation_decisions: Option<&Value>,
) -> Result<SyncPlan, String> {
    let project_root = project_root
        .canonicalize()
        .map_err(|error| format!("project root is unavailable: {error}"))?;
    let template_root = template_root
        .canonicalize()
        .map_err(|error| format!("template root is unavailable: {error}"))?;
    let mode = resolved_mode(&project_root, requested_mode)?;
    let (contract, contract_bytes) = read_contract(&template_root)?;
    let version = contract["release_version"]
        .as_str()
        .ok_or("managed contract release_version is missing")?
        .to_string();
    let current_semver = version.parse::<crate::release::SemVer>()?;
    let compatibility_baseline = crate::baseline::compatibility_baseline(&contract)?;
    let stamp_target = safe_join(
        &project_root,
        contract["stamp"]
            .as_str()
            .ok_or("contract stamp is missing")?,
        "contract stamp",
    )?;
    let contract_target = safe_join(
        &project_root,
        contract["contract_target"]
            .as_str()
            .ok_or("contract target is missing")?,
        "contract target",
    )?;
    let identity = project_identity(&project_root)?;
    let legacy_stamp = identity.as_ref().is_some_and(|(_, legacy)| *legacy);
    let previous_version = identity.map(|(version, _)| version);
    let previous_semver = previous_version
        .as_deref()
        .map(str::parse::<crate::release::SemVer>)
        .transpose()?;
    if previous_semver
        .as_ref()
        .is_some_and(|value| value > &current_semver)
    {
        return Err(format!(
            "project version {} is newer than {version}",
            previous_version.as_deref().unwrap_or_default()
        ));
    }
    let destructive_rebuild = mode == SyncMode::Adopt
        || legacy_stamp
        || previous_semver
            .as_ref()
            .is_some_and(|value| value < &compatibility_baseline);
    let allow_structure_upgrade = previous_semver
        .as_ref()
        .is_some_and(|value| value < &current_semver);
    let migration_sources = crate::asset_migration::scan_sources(&project_root)?;
    let mut safe = Vec::new();
    let mut risk = Vec::new();
    let mut writes = BTreeMap::new();
    let mut deletes = Vec::new();
    let mut gaps = Vec::new();
    let mut blockers = Vec::new();
    let mut generated_source_fingerprints = BTreeMap::<String, String>::new();
    let mut ids = BTreeSet::new();
    let mut targets = BTreeSet::new();
    let assets = contract["assets"]
        .as_array()
        .ok_or("contract assets are missing")?;
    let reserved = assets
        .iter()
        .filter_map(|asset| asset["target"].as_str().map(str::to_string))
        .collect::<Vec<_>>();
    let mut migration = migration_manifest
        .map(|manifest| {
            crate::asset_migration::validate_manifest(&project_root, manifest, &reserved)
        })
        .transpose()?;
    for asset in assets {
        let id = asset["id"]
            .as_str()
            .ok_or("asset id is missing")?
            .to_string();
        let target_raw = asset["target"]
            .as_str()
            .ok_or("asset target is missing")?
            .to_string();
        if !ids.insert(id.clone()) || !targets.insert(target_raw.to_lowercase()) {
            return Err(format!("managed contract has duplicate asset: {id}"));
        }
        let target = safe_join(&project_root, &target_raw, "asset target")?;
        let proposed = migration
            .as_ref()
            .and_then(|m| m.targets.iter().find(|t| t.target == target_raw))
            .map(|t| t.payload.as_slice());
        let (payload, before) = render_asset(
            &project_root,
            &template_root,
            asset,
            proposed,
            allow_structure_upgrade,
        )?;
        if before.as_deref() == Some(payload.as_slice()) {
            continue;
        }
        let strategy = asset["strategy"].as_str().unwrap_or("whole");
        let fully_whole_owned = strategy == "whole"
            && asset.get("agents_zones").is_none()
            && asset.get("managed_blocks").is_none();
        let is_risk = before.is_some() && fully_whole_owned && mode != SyncMode::Init;
        let action = SyncAction {
            id,
            target: target_raw,
            operation: if before.is_some() {
                "replace"
            } else {
                "create"
            }
            .into(),
            risk: is_risk,
            before_sha256: before.as_deref().map(sha_git),
            after_sha256: sha_git(&payload),
        };
        writes.insert(target, payload);
        if is_risk {
            risk.push(action);
        } else {
            safe.push(action);
        }
    }
    let preserve_ids = preservation_decisions
        .and_then(|value| value["preserve"].as_array())
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect::<BTreeSet<_>>();
    let delete_ids = preservation_decisions
        .and_then(|value| value["delete"].as_array())
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect::<BTreeSet<_>>();
    if !preserve_ids.is_disjoint(&delete_ids) {
        return Err("a project asset cannot be both preserved and deleted".into());
    }
    let mut preservation_entries = Vec::<Value>::new();
    let mut preservation_candidates = BTreeSet::new();
    let mut removed_hook_prefixes = Vec::new();
    if destructive_rebuild {
        let known_existing = preservation_entries
            .iter()
            .filter_map(|entry| entry["target"].as_str().map(str::to_string))
            .collect::<Vec<_>>();
        let inventory = destructive_inventory(
            &project_root,
            &contract,
            &migration_sources,
            &known_existing,
            &preserve_ids,
            &delete_ids,
        )?;
        for entry in &inventory.entries {
            if entry["disposition"].as_str() == Some("user-decision") {
                gaps.push(format!(
                    "project asset requires preserve/delete decision: {} ({})",
                    entry["id"].as_str().unwrap_or("<unknown>"),
                    entry["target"].as_str().unwrap_or("<unknown>")
                ));
            }
        }
        if !inventory.deleted_hook_prefixes.is_empty() {
            removed_hook_prefixes = inventory.deleted_hook_prefixes.clone();
            let hooks_target =
                safe_join(&project_root, ".codex/hooks.json", "project hooks target")?;
            let before = fs::read(&hooks_target).ok();
            let base = writes
                .get(&hooks_target)
                .cloned()
                .or_else(|| before.clone())
                .ok_or("project hook bundle has no hooks.json registration")?;
            let filtered = remove_hook_registrations(&base, &inventory.deleted_hook_prefixes)?;
            let after = sha_git(&filtered);
            writes.insert(hooks_target, filtered);
            if let Some(action) = safe
                .iter_mut()
                .chain(risk.iter_mut())
                .find(|action| action.target == ".codex/hooks.json")
            {
                action.after_sha256 = after;
            } else {
                risk.push(SyncAction {
                    id: "rebuild.remove:project-hook-registration".into(),
                    target: ".codex/hooks.json".into(),
                    operation: "replace".into(),
                    risk: true,
                    before_sha256: before.as_deref().map(sha_git),
                    after_sha256: after,
                });
            }
        }
        preservation_entries.extend(inventory.entries);
        preservation_candidates.extend(inventory.candidate_ids);
        risk.extend(inventory.actions);
        deletes.extend(inventory.deletes);
        blockers.extend(inventory.blockers);
    }
    let supplied = preserve_ids
        .union(&delete_ids)
        .cloned()
        .collect::<BTreeSet<_>>();
    let unknown = supplied
        .difference(&preservation_candidates)
        .cloned()
        .collect::<Vec<_>>();
    if !unknown.is_empty() {
        return Err(format!(
            "unknown project asset decision(s): {}",
            unknown.join(", ")
        ));
    }
    let preservation_manifest = json!({
        "status": if preservation_entries.is_empty() { "not-required" } else if preservation_entries.iter().any(|entry| entry["disposition"].as_str() == Some("user-decision")) { "awaiting-confirmation" } else { "confirmed" },
        "destructive_rebuild": destructive_rebuild,
        "compatibility_baseline": contract["compatibility_baseline"],
        "entries": preservation_entries,
    });
    for item in contract["generated_assets"]
        .as_array()
        .ok_or("generated_assets is missing")?
    {
        let id = item["id"].as_str().ok_or("generated asset id is missing")?;
        if generated_asset_current(&project_root, item)? {
            continue;
        }
        let platform = if cfg!(windows) {
            "windows-x86_64"
        } else if cfg!(target_os = "linux") {
            "linux-x86_64"
        } else {
            "macos-x86_64"
        };
        let target = item["binary_targets"][platform]
            .as_str()
            .ok_or("generated binary target is missing")?;
        generated_source_fingerprints.insert(
            format!("generated:{id}"),
            item["source_tree_sha256"]
                .as_str()
                .ok_or("generated source_tree_sha256 is missing")?
                .to_string(),
        );
        safe.push(SyncAction {
            id: format!("generated:{id}"),
            target: target.into(),
            operation: if project_root.join(target).is_file() {
                "rebuild"
            } else {
                "build"
            }
            .into(),
            risk: false,
            before_sha256: fs::read(project_root.join(target))
                .ok()
                .as_deref()
                .map(sha_raw),
            after_sha256: item["source_tree_sha256"]
                .as_str()
                .ok_or("generated source_tree_sha256 is missing")?
                .into(),
        });
    }
    if fs::read(&contract_target).ok().as_deref() != Some(contract_bytes.as_slice()) {
        writes.insert(contract_target.clone(), contract_bytes.clone());
        safe.push(SyncAction {
            id: "managed.contract".into(),
            target: contract["contract_target"].as_str().unwrap().into(),
            operation: if contract_target.exists() {
                "replace"
            } else {
                "create"
            }
            .into(),
            risk: false,
            before_sha256: fs::read(&contract_target).ok().as_deref().map(sha_git),
            after_sha256: sha_git(&contract_bytes),
        });
    }
    let stamp = format!("{version}\n").into_bytes();
    if fs::read(&stamp_target).ok().as_deref() != Some(stamp.as_slice()) {
        writes.insert(stamp_target.clone(), stamp.clone());
        safe.push(SyncAction {
            id: "managed.stamp".into(),
            target: contract["stamp"].as_str().unwrap().into(),
            operation: if stamp_target.exists() {
                "replace"
            } else {
                "create"
            }
            .into(),
            risk: false,
            before_sha256: fs::read(&stamp_target).ok().as_deref().map(sha_git),
            after_sha256: sha_git(&stamp),
        });
    }
    if legacy_stamp {
        let path = safe_join(
            &project_root,
            ".codex/.bridgeforge_version",
            "legacy version stamp",
        )?;
        let before = fs::read(&path).map_err(|e| e.to_string())?;
        deletes.push(path);
        safe.push(SyncAction {
            id: "managed.legacy-stamp".into(),
            target: ".codex/.bridgeforge_version".into(),
            operation: "delete".into(),
            risk: false,
            before_sha256: Some(sha_git(&before)),
            after_sha256: "sha256:deleted".into(),
        });
    }
    let mut asset_migration = if migration_sources.is_empty() {
        json!({"status": "not-required", "source_count": 0, "target_count": 0})
    } else if let Some(validated) = migration.as_mut() {
        for target in &mut validated.targets {
            let path = safe_join(&project_root, &target.target, "migration target")?;
            if let Some(asset) = assets
                .iter()
                .find(|a| a["target"].as_str() == Some(&target.target))
            {
                let (payload, _) = render_asset(
                    &project_root,
                    &template_root,
                    asset,
                    Some(&target.payload),
                    allow_structure_upgrade,
                )?;
                target.payload = payload;
            }
            if target.target == ".codex/hooks.json" && !removed_hook_prefixes.is_empty() {
                target.payload =
                    remove_hook_registrations(&target.payload, &removed_hook_prefixes)?;
            }
            target.after_sha256 = sha_raw(&target.payload);
            writes.insert(path, target.payload.clone());
            safe.retain(|action| action.target != target.target);
            risk.retain(|action| action.target != target.target);
            risk.push(SyncAction {
                id: format!("migration.target:{}", target.target),
                target: target.target.clone(),
                operation: if target.before_sha256.is_some() {
                    "replace"
                } else {
                    "create"
                }
                .into(),
                risk: true,
                before_sha256: target.before_sha256.clone(),
                after_sha256: target.after_sha256.clone(),
            });
        }
        for source in &validated.sources {
            let path = safe_join(&project_root, &source.source_path, "migration source")?;
            deletes.push(path);
            risk.push(SyncAction {
                id: format!("migration.retire:{}", source.asset_id),
                target: source.source_path.clone(),
                operation: "delete".into(),
                risk: true,
                before_sha256: Some(source.source_sha256.clone()),
                after_sha256: "sha256:deleted".into(),
            });
        }
        serde_json::to_value(validated).map_err(|error| error.to_string())?
    } else {
        gaps.push(format!(
            "legacy asset migration requires one confirmed package for each of {} source file(s)",
            migration_sources.len()
        ));
        json!({
            "status": "awaiting-confirmation",
            "source_count": migration_sources.len(),
            "target_count": 0,
            "sources": migration_sources,
        })
    };
    plan_legacy_receipt_retirement(&project_root, &mut safe, &mut deletes)?;
    plan_project_map_retirement(&project_root, &mut safe, &mut deletes)?;
    plan_sync_role_retirement(
        &project_root, &mut safe, &mut deletes, &mut gaps, &mut generated_source_fingerprints,
    )?;
    let (project_hook_inputs, project_hook_reads) = plan_project_hooks(
        &project_root,
        &contract,
        &mut writes,
        &deletes,
        &mut safe,
        &mut risk,
        &mut generated_source_fingerprints,
    )?;
    // Hook projection can rewrite a migration target. Receipts must describe the
    // final transactional payload, not the pre-projection configuration.
    if let Some(targets) = asset_migration["targets"].as_array_mut() {
        for target in targets {
            if let Some(path) = target["target"].as_str() {
                if let Some(payload) = writes.get(&project_root.join(path)) {
                    target["after_sha256"] = json!(sha_raw(payload));
                }
            }
        }
    }
    safe.sort_by(|left, right| left.target.cmp(&right.target));
    risk.sort_by(|left, right| left.target.cmp(&right.target));
    let fingerprint = plan_fingerprint(
        &mode,
        &version,
        &safe,
        &risk,
        &preservation_manifest,
        &generated_source_fingerprints,
    )?;
    let confirmation_required = !risk.is_empty();
    Ok(SyncPlan {
        schema: 1,
        status: if safe.is_empty() && risk.is_empty() {
            "current"
        } else {
            "planned"
        }
        .into(),
        readiness: if confirmation_required || !gaps.is_empty() {
            "action_required"
        } else {
            "ready"
        }
        .into(),
        mode,
        previous_version,
        current_version: version,
        safe,
        risk,
        gaps,
        blockers,
        asset_migration,
        preservation_manifest,
        confirmation_required,
        aggregate_fingerprint: fingerprint,
        project_root,
        writes,
        deletes,
        generated_source_fingerprints,
        project_hook_inputs,
        project_hook_reads,
    })
}

fn plan_project_hooks(
    root: &Path,
    contract: &Value,
    writes: &mut BTreeMap<PathBuf, Vec<u8>>,
    deletes: &[PathBuf],
    safe: &mut Vec<SyncAction>,
    risk: &mut Vec<SyncAction>,
    fingerprints: &mut BTreeMap<String, String>,
) -> Result<
    (
        Vec<(crate::project_hooks::Hook, Vec<u8>)>,
        BTreeMap<String, Option<Vec<u8>>>,
    ),
    String,
> {
    let mut reads = BTreeMap::new();
    let mut inputs = Vec::new();
    let registry_target = crate::project_hooks::REGISTRY_PATH;
    let registry_path = root.join(registry_target);
    if deletes.contains(&registry_path) {
        return Err("project Rust hook registry is scheduled for deletion".into());
    }
    let registry_original = crate::project_hooks::read(root, registry_target)?;
    let registry_payload = writes
        .get(&registry_path)
        .cloned()
        .or_else(|| registry_original.clone());
    reads.insert(registry_target.into(), registry_original.clone());
    let original = crate::project_hooks::read(root, ".codex/hooks.json")?;
    let path = root.join(".codex/hooks.json");
    let Some(payload) = writes.get(&path).cloned().or_else(|| original.clone()) else {
        if registry_payload.is_some() {
            return Err("project Rust hook registry requires hooks.json".into());
        }
        return Ok((inputs, reads));
    };
    let document = crate::baseline::parse_unique_json(&payload, "hooks.json")?;
    let combined = crate::project_hooks::with_registry(&document, registry_payload.as_deref())?;
    let registered = crate::project_hooks::render(&combined)?;
    if let Some(registry) = crate::project_hooks::registry(&combined) {
        fingerprints.insert(
            "project-hook:registry-input".into(),
            crate::manifest::canonical_sha(registry)?,
        );
        if registry_payload.is_none() {
            let mut payload = serde_json::to_vec_pretty(registry).map_err(|e| e.to_string())?;
            payload.push(b'\n');
            risk.push(SyncAction {
                id: "project-hook:registry".into(),
                target: registry_target.into(),
                operation: "create".into(),
                risk: true,
                before_sha256: None,
                after_sha256: sha_git(&payload),
            });
            writes.insert(registry_path, payload);
        }
    }
    reads.insert(".codex/hooks.json".into(), original.clone());
    if registered != document {
        let mut payload = serde_json::to_vec_pretty(&registered).map_err(|e| e.to_string())?;
        payload.push(b'\n');
        if let Some(action) = safe
            .iter_mut()
            .chain(risk.iter_mut())
            .find(|action| action.target == ".codex/hooks.json")
        {
            action.after_sha256 = if action.id.starts_with("migration.") {
                sha_raw(&payload)
            } else {
                sha_git(&payload)
            };
        } else {
            risk.push(SyncAction {
                id: "project-hook:registration".into(),
                target: ".codex/hooks.json".into(),
                operation: "replace".into(),
                risk: true,
                before_sha256: original.as_deref().map(sha_git),
                after_sha256: sha_git(&payload),
            });
        }
        writes.insert(path, payload);
    }
    for hook in crate::project_hooks::hooks(&combined)? {
        let source_path = root.join(hook.source());
        if deletes.contains(&source_path) {
            return Err("registered project Rust hook source is scheduled for deletion".into());
        }
        let original = crate::project_hooks::read(root, &hook.source())?;
        let source = writes
            .get(&source_path)
            .cloned()
            .or_else(|| original.clone())
            .ok_or_else(|| format!("project Rust hook source is missing: {}", hook.source()))?;
        reads.insert(hook.source(), original);
        let fingerprint = crate::project_hooks::identity(&hook, &source, contract)?;
        if crate::project_hooks::current(root, &hook, &fingerprint)? {
            continue;
        }
        let owned = crate::project_hooks::owned(root, &hook)?;
        for target in [hook.binary(), hook.receipt()] {
            if deletes.contains(&root.join(&target)) {
                return Err("project hook build target is scheduled for deletion".into());
            }
            let before = crate::project_hooks::read(root, &target)?;
            let id = format!("project-hook:generated:{target}");
            fingerprints.insert(id.clone(), fingerprint.clone());
            let conflicting = before.is_some() && !owned;
            let action = SyncAction {
                id,
                target,
                operation: "build".into(),
                risk: conflicting,
                before_sha256: before.as_deref().map(sha_git),
                after_sha256: fingerprint.clone(),
            };
            if conflicting {
                risk.push(action);
            } else {
                safe.push(action);
            }
        }
        inputs.push((hook, source));
    }
    Ok((inputs, reads))
}

fn verify_project_hook_reads(plan: &SyncPlan) -> Result<(), String> {
    for (relative, expected) in &plan.project_hook_reads {
        if crate::project_hooks::read(&plan.project_root, relative)? != *expected {
            return Err(format!(
                "project Rust hook input changed after plan: {relative}"
            ));
        }
    }
    Ok(())
}

pub fn apply_plan(
    plan: SyncPlan,
    confirmed_plan_fingerprint: &str,
    confirmed_risk: bool,
) -> Result<SyncReceipt, String> {
    apply_plan_internal(plan, confirmed_plan_fingerprint, confirmed_risk, None)
}

fn validate_transaction_path(path: &Path) -> Result<(), String> {
    for ancestor in path.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(_) => {
                if crate::memory::is_link_or_reparse(ancestor).map_err(|error| error.to_string())? {
                    return Err(format!(
                        "transaction path traverses a link: {}",
                        ancestor.display()
                    ));
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.to_string()),
        }
    }
    Ok(())
}

fn transaction_file_state(path: &Path) -> Result<Option<Vec<u8>>, String> {
    validate_transaction_path(path)?;
    match fs::read(path) {
        Ok(payload) => Ok(Some(payload)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!(
            "cannot read transaction target {}: {error}",
            path.display()
        )),
    }
}

fn delete_unchanged_file(path: &Path, before: Option<&[u8]>) -> Result<(), String> {
    if transaction_file_state(path)?.as_deref() != before {
        return Err(format!(
            "transaction delete target changed before removal: {}",
            path.display()
        ));
    }
    if before.is_some() {
        fs::remove_file(path).map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn apply_plan_internal(
    plan: SyncPlan,
    confirmed_plan_fingerprint: &str,
    confirmed_risk: bool,
    before_stamp: Option<&dyn Fn(&Path, &Path) -> Result<(), String>>,
) -> Result<SyncReceipt, String> {
    // Rejected confirmation must not even create a lock file in the project.
    if !plan.gaps.is_empty() || !plan.blockers.is_empty() {
        return Err("unresolved gap or blocker prevents apply".into());
    }
    if !plan.risk.is_empty() && !confirmed_risk {
        return Err("risk actions require --confirmed-risk".into());
    }
    let sources = crate::asset_migration::scan_sources(&plan.project_root)?;
    let confirmed_sources = plan.asset_migration["sources"].as_array();
    if !sources.is_empty() || confirmed_sources.is_some_and(|items| !items.is_empty()) {
        if plan.asset_migration["status"] != "confirmed"
            || serde_json::to_value(&sources).map_err(|error| error.to_string())?
                != plan.asset_migration["sources"]
        {
            return Err(
                "legacy asset sources or confirmations drifted; regenerate the plan".into(),
            );
        }
    }
    let _lock = ProjectLock::acquire(&plan.project_root)?;
    if let Some(expected) = plan.generated_source_fingerprints.get(RETIRED_SYNC_INPUT)
        && sync_role_fingerprint(&sync_role_inputs(&plan.project_root)?)? != *expected
    {
        return Err("retired role or reference inputs drifted; regenerate the plan".into());
    }
    verify_project_hook_reads(&plan)?;
    let legacy = legacy_receipt(&plan.project_root)?;
    if legacy
        .as_ref()
        .is_some_and(|(path, _)| !plan.deletes.contains(path))
    {
        return Err("legacy build receipt appeared after plan; regenerate the plan".into());
    }
    for (_, target) in RETIRED_PROJECT_MAPS {
        let path = safe_join(&plan.project_root, target, "retired project map")?;
        if transaction_file_state(&path)?.is_some() && !plan.deletes.contains(&path) {
            return Err(format!(
                "retired project map appeared after plan; regenerate the plan: {target}"
            ));
        }
    }
    let current_fingerprint = plan_fingerprint(
        &plan.mode,
        &plan.current_version,
        &plan.safe,
        &plan.risk,
        &plan.preservation_manifest,
        &plan.generated_source_fingerprints,
    )?;
    if plan.aggregate_fingerprint != confirmed_plan_fingerprint
        || plan.aggregate_fingerprint != current_fingerprint
    {
        return Err("aggregate fingerprint drifted; regenerate the plan".into());
    }
    if !plan.risk.is_empty() && !confirmed_risk {
        return Err("risk actions require --confirmed-risk".into());
    }
    if !plan.gaps.is_empty() || !plan.blockers.is_empty() {
        return Err("unresolved gap or blocker prevents apply".into());
    }
    for entry in plan.preservation_manifest["entries"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|entry| {
            matches!(
                entry["disposition"].as_str(),
                Some("preserve" | "required-preserve")
            ) || (entry["disposition"] == "delete" && entry["kind"] == "project-hook-bundle")
        })
    {
        let relative = entry["target"]
            .as_str()
            .ok_or("preservation entry target is missing")?;
        let target = safe_join(&plan.project_root, relative, "preserved project asset")?;
        let actual = if target.is_dir() {
            tree_sha(&target)?
        } else {
            sha_git(
                &fs::read(&target)
                    .map_err(|_| format!("preserved project asset disappeared: {relative}"))?,
            )
        };
        if entry["before_sha256"].as_str() != Some(actual.as_str()) {
            return Err(format!(
                "preserved project asset drifted; regenerate the plan: {relative}"
            ));
        }
    }
    for action in plan.safe.iter().chain(plan.risk.iter()) {
        let relative = Path::new(&action.target);
        let target = plan
            .writes
            .keys()
            .chain(plan.deletes.iter())
            .find(|path| path.ends_with(relative))
            .ok_or_else(|| {
                format!(
                    "planned target is missing from transaction: {}",
                    action.target
                )
            })?;
        let current = fs::read(target).ok();
        let digest = if action.id.starts_with("migration.") {
            sha_raw
        } else {
            sha_git
        };
        let current_hash = current.as_deref().map(digest);
        if current_hash != action.before_sha256 {
            return Err(format!(
                "aggregate fingerprint drifted at {}; regenerate the plan",
                action.target
            ));
        }
        if let Some(payload) = plan.writes.get(target)
            && digest(payload) != action.after_sha256
        {
            return Err(format!(
                "planned payload drifted internally: {}",
                action.target
            ));
        }
    }
    let snapshots = plan
        .writes
        .keys()
        .chain(plan.deletes.iter())
        .map(|path| transaction_file_state(path).map(|before| (path.clone(), before)))
        .collect::<Result<Vec<_>, _>>()?;
    let contract_path = plan.project_root.join(".codex/managed-skeleton.json");
    let contract_payload = plan
        .writes
        .get(&contract_path)
        .cloned()
        .or_else(|| fs::read(&contract_path).ok())
        .ok_or("prospective managed-skeleton contract is missing")?;
    let prospective_contract: Value =
        serde_json::from_slice(&contract_payload).map_err(|error| error.to_string())?;
    let stamp_relative = prospective_contract["stamp"]
        .as_str()
        .ok_or("prospective baseline stamp is missing")?;
    let stamp_path = safe_join(
        &plan.project_root,
        stamp_relative,
        "prospective baseline stamp",
    )?;
    let stamp_payload = plan
        .writes
        .get(&stamp_path)
        .cloned()
        .or_else(|| fs::read(&stamp_path).ok())
        .ok_or("version stamp is missing from the transaction")?;
    let mut retired_directories = Vec::new();
    for entry in plan.preservation_manifest["entries"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|entry| entry["disposition"] == "delete" && entry["kind"] == "project-hook-bundle")
    {
        let directory = safe_join(
            &plan.project_root,
            entry["target"]
                .as_str()
                .ok_or("retired bundle target missing")?,
            "retired hook bundle",
        )?;
        for item in WalkDir::new(directory).contents_first(true) {
            let item = item.map_err(|error| error.to_string())?;
            if item.file_type().is_dir() {
                retired_directories.push(item.path().to_path_buf());
            }
        }
    }
    let mut removed_directories = Vec::new();
    let mut expected_after = BTreeMap::<PathBuf, Option<Vec<u8>>>::new();
    let apply_result = (|| {
        for (path, payload) in &plan.writes {
            if path == &stamp_path {
                continue;
            }
            atomic_write(path, payload)?;
            expected_after.insert(path.clone(), Some(payload.clone()));
        }
        for path in &plan.deletes {
            let original = legacy
                .as_ref()
                .filter(|(legacy_path, _)| legacy_path == path)
                .map(|(_, bytes)| Some(bytes.as_slice()))
                .unwrap_or_else(|| {
                    snapshots
                        .iter()
                        .find(|(target, _)| target == path)
                        .and_then(|(_, bytes)| bytes.as_deref())
                });
            delete_unchanged_file(path, original)?;
            expected_after.insert(path.clone(), None);
        }
        for directory in &retired_directories {
            fs::remove_dir(directory).map_err(|error| {
                format!(
                    "cannot retire hook directory {}: {error}",
                    directory.display()
                )
            })?;
            removed_directories.push(directory.clone());
        }
        crate::baseline::verify_prospective(
            &plan.project_root,
            &contract_path,
            &plan.current_version,
            true,
        )?;
        if let Some(observer) = before_stamp {
            observer(&plan.project_root, &stamp_path)?;
        }
        if plan.generated_source_fingerprints.contains_key(RETIRED_SYNC_INPUT) {
            let inputs = sync_role_inputs(&plan.project_root)?;
            if inputs.contains_key(RETIRED_SYNC_TARGET) || !sync_role_references(&inputs).is_empty() {
                return Err("retired role or reference appeared during apply; regenerate the plan".into());
            }
        }
        atomic_write(&stamp_path, &stamp_payload)?;
        expected_after.insert(stamp_path.clone(), Some(stamp_payload.clone()));
        crate::baseline::verify(&plan.project_root, None, true)?;
        if project_identity(&plan.project_root)? != Some((plan.current_version.clone(), false)) {
            return Err("final version identity does not match the installed baseline".into());
        }
        Ok::<(), String>(())
    })();
    if let Err(error) = apply_result {
        let mut rollback_errors = Vec::new();
        for directory in removed_directories.iter().rev() {
            if let Err(item) = fs::create_dir_all(directory) {
                rollback_errors.push(format!(
                    "cannot restore directory {}: {item}",
                    directory.display()
                ));
            }
        }
        for (path, before) in snapshots.into_iter().rev() {
            let actual = match transaction_file_state(&path) {
                Ok(value) => value,
                Err(item) => {
                    rollback_errors.push(format!(
                        "{}: {item}; external state preserved",
                        path.display()
                    ));
                    continue;
                }
            };
            if actual == before {
                continue;
            }
            if expected_after.get(&path) != Some(&actual) {
                rollback_errors.push(format!(
                    "{}: target changed outside the transaction; external state preserved",
                    path.display()
                ));
                continue;
            }
            let result = match before {
                Some(payload) => atomic_write(&path, &payload),
                None if path.exists() => fs::remove_file(&path).map_err(|item| item.to_string()),
                None => Ok(()),
            };
            if let Err(item) = result {
                rollback_errors.push(format!("{}: {item}", path.display()));
            }
        }
        return if rollback_errors.is_empty() {
            Err(format!("transaction rolled back: {error}"))
        } else {
            Err(format!(
                "transaction rollback incomplete after {error}: {}",
                rollback_errors.join(", ")
            ))
        };
    }
    let mut applied = plan.safe.clone();
    applied.extend(plan.risk.clone());
    Ok(SyncReceipt {
        schema: 1,
        status: "applied".into(),
        execution_status: "succeeded".into(),
        mode: plan.mode,
        previous_version: plan.previous_version,
        current_version: plan.current_version,
        aggregate_fingerprint: plan.aggregate_fingerprint,
        applied,
        rollback_performed: false,
        stamp_written_last: true,
        project_readiness: "ready".into(),
        asset_migration_manifest_sha256: plan.asset_migration["manifest_sha256"]
            .as_str()
            .map(str::to_string),
        preserved_asset_ids: plan.preservation_manifest["entries"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|entry| {
                matches!(
                    entry["disposition"].as_str(),
                    Some("preserve" | "required-preserve")
                )
            })
            .filter_map(|entry| entry["id"].as_str().map(str::to_string))
            .collect(),
    })
}

fn human_plan(plan: &SyncPlan) -> Value {
    let (conclusion, next_step) = if !plan.blockers.is_empty() {
        ("未完成", "先处理同步器报告的阻断项，再重新生成计划")
    } else if !plan.gaps.is_empty() || plan.confirmation_required {
        ("等待确认", "完成当前计划中的用户决定后重新生成计划")
    } else if plan.status == "current" {
        ("无需处理", "本次操作已结束，无需继续处理")
    } else {
        ("可直接执行", "按当前计划执行骨架事务")
    };
    let pending = if !plan.blockers.is_empty() {
        vec![format!("仍有 {} 项阻断需要处理", plan.blockers.len())]
    } else if !plan.gaps.is_empty() {
        vec![format!("仍有 {} 项决定需要确认", plan.gaps.len())]
    } else if plan.confirmation_required {
        vec![format!("有 {} 项风险动作等待统一确认", plan.risk.len())]
    } else {
        Vec::new()
    };
    json!({
        "conclusion": conclusion,
        "pending": pending,
        "next_step": next_step,
        "current_version": plan.current_version
    })
}

fn formatted_outcome(machine: Value, human: Value, format: &str, code: i32) -> CommandOutcome {
    let receipt = match format {
        "machine" => machine,
        "human" => human,
        "combined" => json!({"machine": machine, "human": human}),
        _ => {
            return CommandOutcome::blocked(
                "[project-sync] BLOCKED: invalid --output-format; expected machine|human|combined\n",
            );
        }
    };
    CommandOutcome {
        code,
        receipt: Some(receipt),
        ..CommandOutcome::default()
    }
}

pub fn outcome_plan_with_format(result: Result<SyncPlan, String>, format: &str) -> CommandOutcome {
    match result {
        Ok(plan) => {
            let code = if plan.blockers.is_empty() { 0 } else { 2 };
            let human = human_plan(&plan);
            let machine = serde_json::to_value(plan)
                .unwrap_or_else(|error| json!({"error": error.to_string()}));
            formatted_outcome(machine, human, format, code)
        }
        Err(error) if format == "machine" => {
            CommandOutcome::blocked(format!("[project-sync] BLOCKED: {error}\n"))
        }
        Err(error) => formatted_outcome(
            json!({"status": "blocked", "error": error}),
            json!({
                "conclusion": "未完成",
                "pending": ["同步器在写入前停止"],
                "next_step": "先处理停止原因，再重新生成计划"
            }),
            format,
            2,
        ),
    }
}

pub fn outcome_plan(result: Result<SyncPlan, String>) -> CommandOutcome {
    outcome_plan_with_format(result, "machine")
}

pub fn outcome_receipt_with_format(
    result: Result<SyncReceipt, String>,
    format: &str,
) -> CommandOutcome {
    match result {
        Ok(receipt) => {
            let human = json!({
                "conclusion": "已完成",
                "pending": [],
                "next_step": "需要保存到 GitHub 时运行 $git-sync",
                "current_version": receipt.current_version,
                "changed_count": receipt.applied.len()
            });
            let machine = serde_json::to_value(receipt)
                .unwrap_or_else(|error| json!({"error": error.to_string()}));
            formatted_outcome(machine, human, format, 0)
        }
        Err(error) if format == "machine" => {
            CommandOutcome::blocked(format!("[project-sync] BLOCKED: {error}\n"))
        }
        Err(error) => formatted_outcome(
            json!({"status": "blocked", "error": error}),
            json!({
                "conclusion": "未完成",
                "pending": ["同步事务没有完成"],
                "next_step": "保留现场并先处理停止原因"
            }),
            format,
            2,
        ),
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
            // A successful Cargo invocation must not reuse a previous asset's binary.
            let output_dir = target_dir.join(format!("output-{}", receipts.len()));
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
            let built = output_dir.join("release").join(if cfg!(windows) {
                format!("{binary_name}.exe")
            } else {
                binary_name.into()
            });
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

pub fn attach_generated_assets(
    plan: &mut SyncPlan,
    template_root: &Path,
    project_root: &Path,
    contract: &Value,
    runner: &dyn ProcessRunner,
) -> Result<Vec<Value>, String> {
    let mut receipts = Vec::new();
    if plan
        .safe
        .iter()
        .any(|action| action.id.starts_with("generated:"))
    {
        let (writes, generated_receipts) =
            generated_writes(template_root, "source_root", project_root, contract, runner)?;
        plan.writes.extend(writes);
        receipts.extend(generated_receipts);
    }
    verify_project_hook_reads(plan)?;
    let project_writes = crate::project_hooks::build(
        &template_root.join("templates/hooks"),
        project_root,
        contract,
        &plan.project_hook_inputs,
        runner,
    )?;
    verify_project_hook_reads(plan)?;
    for (hook, _) in &plan.project_hook_inputs {
        if let Some(payload) = project_writes.get(&project_root.join(hook.receipt())) {
            receipts.push(serde_json::from_slice(payload).map_err(|e| e.to_string())?);
        }
    }
    plan.writes.extend(project_writes);
    for action in plan
        .safe
        .iter_mut()
        .chain(plan.risk.iter_mut())
        .filter(|action| {
            action.id.starts_with("generated:") || action.id.starts_with("project-hook:generated:")
        })
    {
        let target = safe_join(project_root, &action.target, "generated action target")?;
        let payload = plan
            .writes
            .get(&target)
            .ok_or_else(|| format!("generated build did not attach payload: {}", action.target))?;
        action.after_sha256 = sha_git(payload);
    }
    plan.aggregate_fingerprint = plan_fingerprint(
        &plan.mode,
        &plan.current_version,
        &plan.safe,
        &plan.risk,
        &plan.preservation_manifest,
        &plan.generated_source_fingerprints,
    )?;
    Ok(receipts)
}

pub fn build_generated_assets(
    project_root: &Path,
    contract: &Value,
    runner: &dyn ProcessRunner,
) -> Result<Vec<Value>, String> {
    let _lock = ProjectLock::acquire(project_root)?;
    let legacy = legacy_receipt(project_root)?;
    let (mut writes, mut receipts) = generated_writes(
        project_root,
        "target_source_root",
        project_root,
        contract,
        runner,
    )?;
    let mut project_writes = BTreeMap::new();
    let mut safe = Vec::new();
    let mut risk = Vec::new();
    let (inputs, reads) = plan_project_hooks(
        project_root,
        contract,
        &mut project_writes,
        &[],
        &mut safe,
        &mut risk,
        &mut BTreeMap::new(),
    )?;
    if !risk.is_empty() || !project_writes.is_empty() {
        return Err(
            "project Rust hook registration changes require project-sync plan/apply".into(),
        );
    }
    let outputs = crate::project_hooks::build(
        &project_root.join(".codex/hooks"),
        project_root,
        contract,
        &inputs,
        runner,
    )?;
    for (relative, expected) in &reads {
        if crate::project_hooks::read(project_root, relative)? != *expected {
            return Err(format!(
                "project Rust hook input changed during build: {relative}"
            ));
        }
    }
    for (hook, _) in &inputs {
        receipts.push(
            serde_json::from_slice(
                outputs
                    .get(&project_root.join(hook.receipt()))
                    .ok_or("project receipt missing")?,
            )
            .map_err(|e| e.to_string())?,
        );
    }
    writes.extend(outputs);
    let snapshots = writes
        .keys()
        .map(|path| transaction_file_state(path).map(|state| (path.clone(), state)))
        .collect::<Result<Vec<_>, _>>()?;
    let result = (|| {
        for (path, payload) in &writes {
            atomic_write(path, payload)?;
        }
        crate::project_hooks::verify(project_root, contract, true)?;
        if let Some((path, before)) = &legacy {
            if fs::read(path).map_err(|error| error.to_string())? != *before {
                return Err("legacy build receipt changed during build".into());
            }
            fs::remove_file(path).map_err(|error| error.to_string())?;
        }
        Ok::<(), String>(())
    })();
    if let Err(error) = result {
        let mut failures = Vec::new();
        for (target, before) in snapshots.into_iter().rev() {
            let current = match transaction_file_state(&target) {
                Ok(state) => state,
                Err(error) => {
                    failures.push(format!(
                        "{}: {error}; external target preserved",
                        target.display()
                    ));
                    continue;
                }
            };
            if current == before {
                continue;
            }
            if current.as_deref() != writes.get(&target).map(Vec::as_slice) {
                failures.push(format!(
                    "{}: external change preserved during rollback",
                    target.display()
                ));
                continue;
            }
            let restored = match before {
                Some(value) => atomic_write(&target, &value),
                None if target.exists() => {
                    fs::remove_file(&target).map_err(|error| error.to_string())
                }
                None => Ok(()),
            };
            if let Err(error) = restored {
                failures.push(format!("{}: {error}", target.display()));
            }
        }
        return Err(if failures.is_empty() {
            format!("generated asset transaction rolled back: {error}")
        } else {
            format!(
                "generated asset rollback incomplete after {error}: {}",
                failures.join("; ")
            )
        });
    }
    Ok(receipts)
}

#[cfg(all(test, bridgeforge_factory_tests))]
#[path = "../../../../../scripts/tests/unit/core_project_sync.rs"]
mod tests;
