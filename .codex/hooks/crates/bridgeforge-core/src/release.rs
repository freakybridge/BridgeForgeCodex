use chrono::Local;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::str::FromStr;
use std::time::Duration;

use crate::{ProcessRequest, ProcessRunner};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SemVer {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

impl FromStr for SemVer {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let parts = value.trim().split('.').collect::<Vec<_>>();
        if parts.len() != 3 {
            return Err(format!("invalid semantic version: {value}"));
        }
        let parse = |raw: &str| {
            if raw.is_empty() || (raw.len() > 1 && raw.starts_with('0')) {
                return Err(format!("invalid semantic version: {value}"));
            }
            raw.parse::<u64>()
                .map_err(|_| format!("invalid semantic version: {value}"))
        };
        Ok(Self {
            major: parse(parts[0])?,
            minor: parse(parts[1])?,
            patch: parse(parts[2])?,
        })
    }
}

impl Display for SemVer {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl Ord for SemVer {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.major, self.minor, self.patch).cmp(&(other.major, other.minor, other.patch))
    }
}

impl PartialOrd for SemVer {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Bump {
    Major,
    Minor,
    Patch,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommitInfo {
    pub kind: String,
    pub description: String,
    pub section: String,
    pub breaking: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseKind {
    None,
    SkeletonOnly,
    Business,
    Factory,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReleasePlan {
    pub schema: u32,
    pub kind: ReleaseKind,
    pub changed_paths: Vec<String>,
    pub requires_business_version: bool,
}

#[derive(Clone, Debug)]
pub struct FileReleasePlan {
    pub inputs: BTreeMap<PathBuf, Option<Vec<u8>>>,
    pub old_version: SemVer,
    pub new_version: SemVer,
    pub kind: ReleaseKind,
    pub writes: BTreeMap<PathBuf, Vec<u8>>,
}

pub fn parse_commit_message(message: &str) -> Result<CommitInfo, String> {
    let header = message.replace("\r\n", "\n");
    let first = header.lines().next().unwrap_or("").trim();
    let expression =
        Regex::new(r"^(feat|fix|docs|refactor|chore|perf)(?:\([^\)\r\n]+\))?(!)?:\s+(.+?)\s*$")
            .map_err(|error| error.to_string())?;
    let captures = expression.captures(first).ok_or(
        "commit message must use feat/fix/docs/refactor/chore/perf with Conventional Commits",
    )?;
    let kind = captures[1].to_string();
    let breaking = captures.get(2).is_some()
        || Regex::new(r"(?m)^BREAKING CHANGE:\s*\S")
            .map_err(|error| error.to_string())?
            .is_match(&header);
    let section = match kind.as_str() {
        "feat" => "Added",
        "fix" => "Fixed",
        _ => "Changed",
    };
    Ok(CommitInfo {
        kind,
        description: captures[3].to_string(),
        section: section.into(),
        breaking,
    })
}

fn bump(version: SemVer, info: &CommitInfo) -> SemVer {
    let level = if info.breaking {
        Bump::Major
    } else if info.kind == "feat" {
        Bump::Minor
    } else {
        Bump::Patch
    };
    match level {
        Bump::Major => SemVer {
            major: version.major + 1,
            minor: 0,
            patch: 0,
        },
        Bump::Minor => SemVer {
            major: version.major,
            minor: version.minor + 1,
            patch: 0,
        },
        Bump::Patch => SemVer {
            major: version.major,
            minor: version.minor,
            patch: version.patch + 1,
        },
    }
}

pub fn build_release_plan(paths: impl IntoIterator<Item = String>) -> ReleasePlan {
    let mut paths = paths
        .into_iter()
        .map(|path| path.replace('\\', "/"))
        .filter(|path| !path.trim().is_empty())
        .collect::<Vec<_>>();
    paths.sort();
    paths.dedup();
    if paths.is_empty() {
        return ReleasePlan {
            schema: 1,
            kind: ReleaseKind::None,
            changed_paths: paths,
            requires_business_version: false,
        };
    }
    let skeleton_only = paths.iter().all(|path| {
        path == "AGENTS.md"
            || path == "INSTALL.md"
            || path == "README.md"
            || path == "CHANGELOG.md"
            || path == "bridgeforge-codex-manifest.json"
            || path.starts_with(".codex/")
            || path.starts_with(".githooks/")
            || path.starts_with("doc/")
            || path.starts_with("scripts/")
            || path.starts_with("skills/")
            || path.starts_with("templates/")
    });
    ReleasePlan {
        schema: 1,
        kind: if skeleton_only {
            ReleaseKind::SkeletonOnly
        } else {
            ReleaseKind::Business
        },
        changed_paths: paths,
        requires_business_version: !skeleton_only,
    }
}

fn safe_relative(root: &Path, raw: &str) -> Result<PathBuf, String> {
    let relative = Path::new(raw);
    if relative.is_absolute()
        || relative.components().any(|part| {
            matches!(
                part,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(format!("version manifest escapes repository: {raw}"));
    }
    Ok(root.join(relative))
}

fn configured_manifests(root: &Path) -> Result<Vec<PathBuf>, String> {
    let config = root.join(".codex/bridgeforge-version.json");
    if config.is_file() {
        let document: Value =
            serde_json::from_slice(&fs::read(&config).map_err(|error| error.to_string())?)
                .map_err(|error| format!("invalid version sync config: {error}"))?;
        if document["schema_version"].as_u64() != Some(1) {
            return Err("version sync config must use schema_version=1".into());
        }
        let items = document["manifests"]
            .as_array()
            .ok_or("version sync config manifests are missing")?;
        if items.is_empty() {
            return Err("version sync config manifests cannot be empty".into());
        }
        let mut result = Vec::new();
        for item in items {
            let raw = item
                .as_str()
                .ok_or("configured manifest path must be a string")?;
            let path = safe_relative(root, raw)?;
            if !path.is_file()
                || !matches!(
                    path.file_name().and_then(|value| value.to_str()),
                    Some("Cargo.toml" | "package.json")
                )
            {
                return Err(format!("unsupported or missing configured manifest: {raw}"));
            }
            result.push(path);
        }
        let unique = result.iter().collect::<BTreeSet<_>>();
        if unique.len() != result.len() {
            return Err("configured manifests contain duplicates".into());
        }
        return Ok(result);
    }
    Ok(["Cargo.toml", "package.json"]
        .iter()
        .map(|name| root.join(name))
        .filter(|path| path.is_file())
        .collect())
}

fn cargo_version(path: &Path) -> Result<Option<SemVer>, String> {
    let text = fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let value =
        Regex::new(r#"(?m)^version\s*=\s*"([^"]+)"\s*$"#).map_err(|error| error.to_string())?;
    let mut versions = Vec::new();
    let mut package_section = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            package_section = matches!(trimmed, "[package]" | "[workspace.package]");
            continue;
        }
        if package_section && let Some(capture) = value.captures(line) {
            versions.push(capture[1].parse::<SemVer>()?);
        }
    }
    match versions.as_slice() {
        [] => Ok(None),
        [only] => Ok(Some(*only)),
        _ => Err(format!(
            "ambiguous Cargo version fields in {}",
            path.display()
        )),
    }
}

fn json_version(path: &Path) -> Result<Option<SemVer>, String> {
    let payload = fs::read(path).map_err(|error| error.to_string())?;
    let document: Value = serde_json::from_slice(&payload)
        .map_err(|error| format!("invalid JSON {}: {error}", path.display()))?;
    document
        .get("version")
        .and_then(Value::as_str)
        .map(str::parse)
        .transpose()
}

fn render_cargo(path: &Path, old: SemVer, new: SemVer) -> Result<Vec<u8>, String> {
    let text = fs::read_to_string(path).map_err(|error| error.to_string())?;
    let version = Regex::new(&format!(
        r#"(?m)^(version\s*=\s*)"{}"(\s*)$"#,
        regex::escape(&old.to_string())
    ))
    .map_err(|error| error.to_string())?;
    let mut output = String::with_capacity(text.len());
    let mut count = 0;
    let mut package_section = false;
    for line in text.split_inclusive('\n') {
        let body = line.trim_end_matches(['\r', '\n']);
        let ending = &line[body.len()..];
        let trimmed = body.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            package_section = matches!(trimmed, "[package]" | "[workspace.package]");
        }
        if package_section && version.is_match(body) {
            output.push_str(&version.replace(body, format!("${{1}}\"{new}\"${{2}}")));
            output.push_str(ending);
            count += 1;
        } else {
            output.push_str(line);
        }
    }
    if count != 1 {
        return Err(format!(
            "Cargo manifest has no unique version {old}: {}",
            path.display()
        ));
    }
    Ok(output.into_bytes())
}

fn render_cargo_lock(path: &Path, old: SemVer, new: SemVer) -> Result<Vec<u8>, String> {
    let text = fs::read_to_string(path).map_err(|error| error.to_string())?;
    let marker = "[[package]]";
    let version = Regex::new(&format!(
        r#"(?m)^(version\s*=\s*)"{}"(\s*)$"#,
        regex::escape(&old.to_string())
    ))
    .map_err(|error| error.to_string())?;
    let mut changed = 0;
    let mut rendered = Vec::new();
    for (index, part) in text.split(marker).enumerate() {
        if index == 0 {
            rendered.push(part.to_string());
            continue;
        }
        let prefix = format!("{marker}{part}");
        if !part
            .lines()
            .any(|line| line.trim_start().starts_with("source ="))
            && version.is_match(part)
        {
            changed += 1;
            rendered.push(
                version
                    .replace(&prefix, format!("${{1}}\"{new}\"${{2}}"))
                    .to_string(),
            );
        } else {
            rendered.push(prefix);
        }
    }
    if changed == 0 {
        return Err(format!(
            "Cargo.lock has no local package at version {old}: {}",
            path.display()
        ));
    }
    Ok(rendered.concat().into_bytes())
}

fn render_json_version(path: &Path, new: SemVer) -> Result<Vec<u8>, String> {
    let payload = fs::read(path).map_err(|error| error.to_string())?;
    let mut document: Value =
        serde_json::from_slice(&payload).map_err(|error| error.to_string())?;
    if !document.get("version").is_some_and(Value::is_string) {
        return Err(format!("missing top-level version in {}", path.display()));
    }
    document["version"] = Value::String(new.to_string());
    let mut encoded = serde_json::to_vec_pretty(&document).map_err(|error| error.to_string())?;
    encoded.push(b'\n');
    Ok(encoded)
}

fn render_package_lock(path: &Path, old: SemVer, new: SemVer) -> Result<Vec<u8>, String> {
    let payload = fs::read(path).map_err(|error| error.to_string())?;
    let mut document: Value =
        serde_json::from_slice(&payload).map_err(|error| error.to_string())?;
    let old_text = old.to_string();
    if document["version"].as_str() != Some(old_text.as_str())
        || document["packages"][""]["version"].as_str() != Some(old_text.as_str())
    {
        return Err(format!(
            "package-lock root version is missing or inconsistent: {}",
            path.display()
        ));
    }
    document["version"] = Value::String(new.to_string());
    document["packages"][""]["version"] = Value::String(new.to_string());
    let mut encoded = serde_json::to_vec_pretty(&document).map_err(|error| error.to_string())?;
    encoded.push(b'\n');
    Ok(encoded)
}

fn git_bytes(payload: &[u8]) -> Vec<u8> {
    String::from_utf8_lossy(payload)
        .replace("\r\n", "\n")
        .into_bytes()
}

fn digest(payload: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(payload))
}

fn marker_projection(payload: &[u8], begin: &str, end: &str) -> Result<(Vec<u8>, Vec<u8>), String> {
    let text = String::from_utf8(git_bytes(payload)).map_err(|_| "managed text is not UTF-8")?;
    let lines = text.split_inclusive('\n').collect::<Vec<_>>();
    let starts = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.trim_end_matches('\n') == begin)
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    let ends = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.trim_end_matches('\n') == end)
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    if starts.len() != 1 || ends.len() != 1 || starts[0] >= ends[0] {
        return Err(format!(
            "managed markers are missing or duplicated: {begin} / {end}"
        ));
    }
    let block = lines[starts[0]..=ends[0]].concat().into_bytes();
    let mut outside = Vec::new();
    outside.extend_from_slice(lines[..starts[0]].concat().as_bytes());
    outside.extend_from_slice(lines[ends[0] + 1..].concat().as_bytes());
    Ok((block, outside))
}

fn heading_section(payload: &[u8], heading: &str) -> Result<Vec<u8>, String> {
    let text =
        String::from_utf8(git_bytes(payload)).map_err(|_| "managed Markdown is not UTF-8")?;
    let lines = text.split_inclusive('\n').collect::<Vec<_>>();
    let heading_level = heading.chars().take_while(|value| *value == '#').count();
    let matches = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.trim_end_matches('\n') == heading)
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    if matches.len() != 1 || heading_level == 0 {
        return Err(format!(
            "managed Markdown heading is missing or duplicated: {heading}"
        ));
    }
    let start = matches[0];
    let stop = lines
        .iter()
        .enumerate()
        .skip(start + 1)
        .find(|(_, line)| {
            let trimmed = line.trim_start();
            let level = trimmed.chars().take_while(|value| *value == '#').count();
            level > 0 && level <= heading_level && trimmed.chars().nth(level) == Some(' ')
        })
        .map(|(index, _)| index)
        .unwrap_or(lines.len());
    Ok(lines[start..stop].concat().into_bytes())
}

fn table_rows(section: &[u8]) -> Result<BTreeMap<String, Vec<u8>>, String> {
    let text = String::from_utf8_lossy(section);
    let lines = text
        .lines()
        .filter(|line| line.trim_start().starts_with('|'))
        .collect::<Vec<_>>();
    if lines.len() < 2 {
        return Err("managed Markdown table is missing".into());
    }
    let link = Regex::new(r"^\[`[^`]+`\]\(([^)]+)\)$").map_err(|error| error.to_string())?;
    let mut result = BTreeMap::new();
    for line in lines.into_iter().skip(2) {
        let cells = line
            .trim()
            .trim_matches('|')
            .split('|')
            .map(str::trim)
            .collect::<Vec<_>>();
        let Some(raw_key) = cells.first() else {
            continue;
        };
        let mut key = raw_key.to_string();
        if let Some(captures) = link.captures(&key) {
            key = captures[1].to_string();
        }
        key = key.trim_matches('`').to_lowercase();
        if result
            .insert(key.clone(), format!("{line}\n").into_bytes())
            .is_some()
        {
            return Err(format!("managed Markdown table key is duplicated: {key}"));
        }
    }
    Ok(result)
}

fn split_json_ownership(
    required: &Value,
    actual: &Value,
    path: &str,
) -> Result<(Value, Value), String> {
    let Some(required) = required.as_object() else {
        if required != actual {
            return Err(format!("managed JSON value drifted: {path}"));
        }
        return Ok((actual.clone(), Value::Null));
    };
    let actual = actual
        .as_object()
        .ok_or_else(|| format!("managed JSON value drifted: {path}"))?;
    let mut public = serde_json::Map::new();
    let mut project = actual
        .iter()
        .filter(|(key, _)| !required.contains_key(*key))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect::<serde_json::Map<_, _>>();
    for (key, expected) in required {
        let value = actual
            .get(key)
            .ok_or_else(|| format!("managed JSON key is missing: {path}.{key}"))?;
        let (managed, extra) = split_json_ownership(expected, value, &format!("{path}.{key}"))?;
        public.insert(key.clone(), managed);
        if !matches!(&extra, Value::Null)
            && extra.as_object().is_none_or(|value| !value.is_empty())
            && extra.as_array().is_none_or(|value| !value.is_empty())
        {
            project.insert(key.clone(), extra);
        }
    }
    Ok((Value::Object(public), Value::Object(project)))
}

fn hooks_projection(document: &Value) -> Result<(Value, Value), String> {
    let object = document.as_object().ok_or("hooks.json must be an object")?;
    let hooks = object
        .get("hooks")
        .and_then(Value::as_object)
        .ok_or("hooks.json has no hooks object")?;
    let mut managed = Vec::new();
    let mut project = Vec::new();
    let mut seen = BTreeSet::new();
    for (event, entries) in hooks {
        for entry in entries
            .as_array()
            .ok_or_else(|| format!("hooks.json event is invalid: {event}"))?
        {
            let group = entry
                .as_object()
                .ok_or_else(|| format!("hooks.json group is invalid: {event}"))?;
            let matcher = group.get("matcher").and_then(Value::as_str).unwrap_or("");
            for handler in group
                .get("hooks")
                .and_then(Value::as_array)
                .ok_or_else(|| format!("hooks.json group is invalid: {event}"))?
            {
                let handler_id = handler["bridgeforgeCodexId"].as_str();
                let record = json!({"event": event, "matcher": matcher, "handler": handler});
                if handler_id.is_some_and(|value| value.starts_with("bridgeforge-codex.")) {
                    let handler_id = handler_id.unwrap();
                    if !seen.insert(handler_id.to_string()) {
                        return Err(format!("managed hook handler is duplicated: {handler_id}"));
                    }
                    managed.push(record);
                } else {
                    project.push(record);
                }
            }
        }
    }
    let top_level = object
        .iter()
        .filter(|(key, _)| !matches!(key.as_str(), "description" | "hooks"))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect::<serde_json::Map<_, _>>();
    Ok((
        json!({"description": object.get("description"), "handlers": managed}),
        json!({"top_level": top_level, "handlers": project}),
    ))
}

fn ownership_projection(asset: &Value, payload: &[u8]) -> Result<(Value, Value), String> {
    if let Some(zones) = asset.get("agents_zones").and_then(Value::as_object) {
        let public = &zones["public"];
        let project = &zones["project"];
        let (public_block, without_public) = marker_projection(
            payload,
            public["begin"]
                .as_str()
                .ok_or("public begin marker is missing")?,
            public["end"]
                .as_str()
                .ok_or("public end marker is missing")?,
        )?;
        let (project_block, outside) = marker_projection(
            &without_public,
            project["begin"]
                .as_str()
                .ok_or("project begin marker is missing")?,
            project["end"]
                .as_str()
                .ok_or("project end marker is missing")?,
        )?;
        return Ok((
            json!(digest(&public_block)),
            json!({"project": digest(&project_block), "outside": digest(&outside)}),
        ));
    }
    if let Some(managed) = asset.get("managed_blocks").and_then(Value::as_object) {
        let mut headings = serde_json::Map::new();
        let mut tables = serde_json::Map::new();
        let mut project = git_bytes(payload);
        for heading in managed
            .get("headings")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let heading = heading.as_str().ok_or("managed heading is invalid")?;
            let section = heading_section(payload, heading)?;
            headings.insert(heading.to_string(), json!(digest(&section)));
            if let Some(index) = project
                .windows(section.len())
                .position(|item| item == section)
            {
                project.drain(index..index + section.len());
            }
        }
        for table in managed
            .get("keyed_tables")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let heading = table["heading"]
                .as_str()
                .ok_or("managed table heading is invalid")?;
            let section = heading_section(payload, heading)?;
            let rows = table_rows(&section)?;
            let mut projected = serde_json::Map::new();
            for key in table["managed_keys"].as_array().into_iter().flatten() {
                let raw = key.as_str().ok_or("managed table key is invalid")?;
                let normalized = raw.trim().trim_matches('`').to_lowercase();
                let row = rows.get(&normalized).ok_or_else(|| {
                    format!("managed Markdown row is missing: {heading} :: {raw}")
                })?;
                projected.insert(normalized, json!(digest(row)));
                if let Some(index) = project.windows(row.len()).position(|item| item == row) {
                    project.drain(index..index + row.len());
                }
            }
            tables.insert(heading.to_string(), Value::Object(projected));
        }
        return Ok((
            json!({"headings": headings, "keyed_tables": tables}),
            json!(digest(&project)),
        ));
    }
    match asset["strategy"].as_str().unwrap_or("") {
        "region" => {
            let region = &asset["region"];
            let (managed, outside) = marker_projection(
                payload,
                region["begin"]
                    .as_str()
                    .ok_or("region begin marker is missing")?,
                region["end"]
                    .as_str()
                    .ok_or("region end marker is missing")?,
            )?;
            Ok((json!(digest(&managed)), json!(digest(&outside))))
        }
        "merge" if asset["merge_policy"].as_str() == Some("git-attributes-default-lf") => {
            let mut required = false;
            let mut project = Vec::new();
            for line in String::from_utf8_lossy(&git_bytes(payload)).lines() {
                let fields = line.split_whitespace().collect::<Vec<_>>();
                if fields.first() == Some(&"*")
                    && fields.contains(&"text=auto")
                    && fields.contains(&"eol=lf")
                {
                    required = true;
                } else {
                    project.extend_from_slice(format!("{line}\n").as_bytes());
                }
            }
            Ok((json!(required), json!(digest(&project))))
        }
        "merge" => {
            let document: Value = serde_json::from_slice(payload)
                .map_err(|error| format!("managed JSON is invalid: {error}"))?;
            if asset["merge_policy"].as_str() == Some("codex-hooks") {
                hooks_projection(&document)
            } else {
                let required = &asset["merge_validation"]["required"];
                split_json_ownership(required, &document, "managed JSON")
            }
        }
        "seed" => Ok((json!(null), json!(digest(&git_bytes(payload))))),
        "whole" => Ok((json!(digest(&git_bytes(payload))), json!(null))),
        strategy => Err(format!("unsupported ownership strategy: {strategy}")),
    }
}

fn verify_contract_payload(asset: &Value, payload: &[u8]) -> Result<(), String> {
    crate::baseline::verify_asset_payload(asset, payload).map(|_| ())
}

fn contract_assets(
    contract: &Value,
    label: &str,
) -> Result<(BTreeMap<String, Value>, BTreeMap<String, Value>), String> {
    let mut by_target = BTreeMap::new();
    let mut by_id = BTreeMap::new();
    for asset in contract["assets"]
        .as_array()
        .ok_or_else(|| format!("{label} contract assets are missing"))?
    {
        let id = asset["id"]
            .as_str()
            .ok_or_else(|| format!("{label} asset id is missing"))?;
        let target = asset["target"]
            .as_str()
            .ok_or_else(|| format!("{label} asset target is missing"))?
            .replace('\\', "/");
        if by_target.insert(target, asset.clone()).is_some()
            || by_id.insert(id.to_string(), asset.clone()).is_some()
        {
            return Err(format!(
                "{label} contract asset identity is duplicated: {id}"
            ));
        }
    }
    Ok((by_target, by_id))
}

fn head_payload(
    root: &Path,
    relative: &str,
    runner: &dyn ProcessRunner,
) -> Result<Option<Vec<u8>>, String> {
    let mut exists = ProcessRequest::new("git", root);
    exists.args = [
        "-c".into(),
        format!("safe.directory={}", root.display()).into(),
        "ls-tree".into(),
        "-z".into(),
        "HEAD".into(),
        "--".into(),
        relative.into(),
    ]
    .to_vec();
    exists.timeout = Duration::from_secs(30);
    let listed = runner.run(&exists).map_err(|error| error.to_string())?;
    if listed.timed_out {
        return Err(format!("timed out while checking HEAD payload: {relative}"));
    }
    if listed.code != 0 {
        return Err(format!(
            "cannot check HEAD payload {relative}: {}",
            String::from_utf8_lossy(&listed.stderr).trim()
        ));
    }
    if listed.stdout.is_empty() {
        return Ok(None);
    }
    let mut request = ProcessRequest::new("git", root);
    request.args = [
        "-c".into(),
        format!("safe.directory={}", root.display()).into(),
        "show".into(),
        format!("HEAD:{relative}").into(),
    ]
    .to_vec();
    request.timeout = Duration::from_secs(30);
    let output = runner.run(&request).map_err(|error| error.to_string())?;
    if output.timed_out {
        return Err(format!("timed out while reading HEAD payload: {relative}"));
    }
    if output.code != 0 {
        return Err(format!(
            "cannot read HEAD payload {relative}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(Some(output.stdout))
}

fn classify(
    root: &Path,
    changed: &[String],
    runner: &dyn ProcessRunner,
) -> Result<ReleaseKind, String> {
    if root.join("templates/managed-skeleton.json").is_file() {
        return Ok(ReleaseKind::Factory);
    }
    let contract_path = root.join(".codex/managed-skeleton.json");
    let current_bytes = fs::read(&contract_path).map_err(|_| "current-only baseline is missing")?;
    let current = crate::baseline::parse_unique_json(&current_bytes, "current-only baseline")?;
    let (current_by_target, current_by_id) = contract_assets(&current, "current")?;
    let contract_target = current["contract_target"]
        .as_str()
        .unwrap_or(".codex/managed-skeleton.json")
        .replace('\\', "/");
    let head_contract_bytes = head_payload(root, &contract_target, runner)?;
    let contract_transition = head_contract_bytes
        .as_deref()
        .is_none_or(|payload| git_bytes(payload) != git_bytes(&current_bytes));
    let (head, head_by_target, head_by_id, unverifiable_head) = match head_contract_bytes {
        Some(payload) if contract_transition => {
            match crate::baseline::parse_unique_json(&payload, "HEAD current-only baseline")
                .and_then(|value| contract_assets(&value, "HEAD").map(|maps| (value, maps)))
            {
                Ok((value, (by_target, by_id))) => (value, by_target, by_id, false),
                Err(_) => (json!({}), BTreeMap::new(), BTreeMap::new(), true),
            }
        }
        Some(_) => (
            current.clone(),
            current_by_target.clone(),
            current_by_id.clone(),
            false,
        ),
        None => (json!({}), BTreeMap::new(), BTreeMap::new(), false),
    };
    let mut transition_paths = BTreeSet::from([contract_target]);
    for candidate in [current.get("stamp"), head.get("stamp")]
        .into_iter()
        .flatten()
    {
        if let Some(value) = candidate.as_str() {
            transition_paths.insert(value.replace('\\', "/"));
        }
    }
    let mut public_changed = unverifiable_head;
    let mut project_changed = unverifiable_head;
    let mut handled = BTreeSet::new();
    for relative in changed
        .iter()
        .filter(|path| !matches!(path.as_str(), "VERSION" | "CHANGELOG.md"))
    {
        if transition_paths.contains(relative) {
            public_changed = true;
            continue;
        }
        let mut current_asset = current_by_target.get(relative);
        let mut head_asset = head_by_target.get(relative);
        if head_asset.is_none() {
            if let Some(asset) = current_asset {
                head_asset = asset["id"].as_str().and_then(|id| head_by_id.get(id));
            }
        }
        if current_asset.is_none() {
            if let Some(asset) = head_asset {
                current_asset = asset["id"].as_str().and_then(|id| current_by_id.get(id));
            }
        }
        if let (Some(asset), None) = (current_asset, head_asset)
            && asset["merge_policy"].as_str() == Some("git-attributes-default-lf")
        {
            let target = asset["target"]
                .as_str()
                .ok_or("gitattributes target is missing")?;
            let current_payload = fs::read(root.join(target)).unwrap_or_default();
            verify_contract_payload(asset, &current_payload)
                .map_err(|error| format!("current ownership baseline is invalid: {error}"))?;
            let before_payload = head_payload(root, target, runner)?.unwrap_or_default();
            let (_, old_project) = ownership_projection(asset, &before_payload)?;
            let (_, new_project) = ownership_projection(asset, &current_payload)?;
            public_changed = true;
            project_changed |= old_project != new_project;
            continue;
        }
        let (Some(current_asset), Some(head_asset)) = (current_asset, head_asset) else {
            if current_asset.is_none() && head_asset.is_none() {
                project_changed = true;
            } else {
                public_changed = true;
                let asset = current_asset.or(head_asset).unwrap();
                project_changed |= asset["strategy"].as_str() != Some("whole");
            }
            continue;
        };
        let id = current_asset["id"]
            .as_str()
            .ok_or("current asset id is missing")?;
        if head_asset["id"].as_str() != Some(id) {
            return Err(format!(
                "HEAD and current ownership identities disagree for one target: {relative}"
            ));
        }
        if !handled.insert(id.to_string()) {
            continue;
        }
        let current_target = current_asset["target"].as_str().unwrap().replace('\\', "/");
        let head_target = head_asset["target"].as_str().unwrap().replace('\\', "/");
        let current_payload = fs::read(root.join(&current_target)).unwrap_or_default();
        let Some(before_payload) = head_payload(root, &head_target, runner)? else {
            public_changed = true;
            project_changed = true;
            continue;
        };
        if let Err(error) = verify_contract_payload(head_asset, &before_payload) {
            if contract_transition {
                public_changed = true;
                project_changed = true;
                continue;
            }
            return Err(format!("HEAD ownership baseline is invalid: {error}"));
        }
        verify_contract_payload(current_asset, &current_payload)
            .map_err(|error| format!("current ownership baseline is invalid: {error}"))?;
        match (
            ownership_projection(head_asset, &before_payload),
            ownership_projection(current_asset, &current_payload),
        ) {
            (Ok((old_public, old_project)), Ok((new_public, new_project))) => {
                public_changed |= old_public != new_public;
                project_changed |= old_project != new_project;
            }
            _ if contract_transition => {
                public_changed = true;
                project_changed = true;
            }
            (Err(error), _) | (_, Err(error)) => return Err(error),
        }
    }
    Ok(if project_changed || !public_changed {
        ReleaseKind::Business
    } else {
        ReleaseKind::SkeletonOnly
    })
}

fn render_changelog(
    root: &Path,
    version: SemVer,
    info: &CommitInfo,
    kind: &ReleaseKind,
    changed: &[String],
) -> Result<Vec<u8>, String> {
    let path = root.join("CHANGELOG.md");
    let mut text = fs::read_to_string(&path).unwrap_or_else(|_| "# Changelog\n".into());
    let heading = format!("## [{version}]");
    if text.lines().any(|line| line.starts_with(&heading)) {
        return Err(format!("CHANGELOG already contains version {version}"));
    }
    let prefix = if *kind == ReleaseKind::Factory {
        let mut tags = Vec::new();
        if changed
            .iter()
            .any(|path| path.starts_with("templates/") || path.starts_with("skills/"))
        {
            tags.push("[product]");
        }
        if changed.iter().any(|path| {
            path.starts_with(".codex/")
                || path.starts_with("scripts/")
                || path.starts_with(".githooks/")
        }) {
            tags.push("[repo]");
        }
        if changed.iter().any(|path| {
            path.starts_with("doc/")
                || matches!(path.as_str(), "README.md" | "AGENTS.md" | "INSTALL.md")
        }) {
            tags.push("[meta]");
        }
        if tags.is_empty() {
            tags.push("[repo]");
        }
        format!("{} ", tags.concat())
    } else {
        String::new()
    };
    let breaking = if info.breaking { " **BREAKING:**" } else { "" };
    let entry = format!(
        "## [{version}] - {}\n\n### {}\n\n- {prefix}{}{breaking}\n\n",
        Local::now().date_naive(),
        info.section,
        info.description
    );
    let headings = Regex::new(r"(?m)^## \[").map_err(|error| error.to_string())?;
    let positions = headings
        .find_iter(&text)
        .map(|item| item.start())
        .collect::<Vec<_>>();
    let insert = if let Some(first) = positions.first() {
        if text[*first..].starts_with("## [Unreleased]") {
            positions.get(1).copied().unwrap_or(text.len())
        } else {
            *first
        }
    } else {
        text.len()
    };
    let suffix = text.split_off(insert);
    text = format!("{}\n\n{entry}{suffix}", text.trim_end());
    Ok(text.into_bytes())
}

pub fn build_file_release_plan(
    root: &Path,
    message: &str,
    changed_paths: Vec<String>,
    runner: &dyn ProcessRunner,
) -> Result<Option<FileReleasePlan>, String> {
    let mut changed = changed_paths
        .into_iter()
        .map(|path| path.replace('\\', "/"))
        .filter(|path| !path.is_empty())
        .collect::<Vec<_>>();
    changed.sort();
    changed.dedup();
    if changed.is_empty() {
        return Ok(None);
    }
    let info = parse_commit_message(message)?;
    let kind = classify(root, &changed, runner)?;
    if kind == ReleaseKind::SkeletonOnly {
        return Ok(None);
    }
    let mut inputs = BTreeMap::new();
    let config = root.join(".codex/bridgeforge-version.json");
    inputs.insert(config.clone(), release_input(&config)?);
    let manifests = configured_manifests(root)?;
    for path in &manifests {
        inputs.insert(path.clone(), release_input(path)?);
        for name in [
            "Cargo.lock",
            "package-lock.json",
            "pnpm-lock.yaml",
            "yarn.lock",
        ] {
            let lock = path.with_file_name(name);
            inputs.insert(lock.clone(), release_input(&lock)?);
        }
    }
    let changelog = root.join("CHANGELOG.md");
    inputs.insert(changelog.clone(), release_input(&changelog)?);
    let version_path = root.join("VERSION");
    inputs.insert(version_path.clone(), release_input(&version_path)?);
    let old_version = fs::read_to_string(&version_path)
        .map_err(|_| "root VERSION is missing")?
        .trim()
        .parse::<SemVer>()?;
    let new_version = bump(old_version, &info);
    let mut writes = BTreeMap::new();
    writes.insert(version_path, format!("{new_version}\n").into_bytes());
    for path in manifests {
        let current = if path.file_name().and_then(|value| value.to_str()) == Some("Cargo.toml") {
            cargo_version(&path)?
        } else {
            json_version(&path)?
        }
        .ok_or_else(|| {
            format!(
                "configured manifest has no static version: {}",
                path.display()
            )
        })?;
        if current != old_version {
            return Err(format!(
                "native manifest disagrees with VERSION: {}",
                path.display()
            ));
        }
        if path.file_name().and_then(|value| value.to_str()) == Some("Cargo.toml") {
            writes.insert(path.clone(), render_cargo(&path, old_version, new_version)?);
            let lock = path.with_file_name("Cargo.lock");
            if lock.is_file() {
                writes.insert(
                    lock.clone(),
                    render_cargo_lock(&lock, old_version, new_version)?,
                );
            }
        } else {
            writes.insert(path.clone(), render_json_version(&path, new_version)?);
            let lock = path.with_file_name("package-lock.json");
            if lock.is_file() {
                writes.insert(
                    lock.clone(),
                    render_package_lock(&lock, old_version, new_version)?,
                );
            }
            for unsupported in ["pnpm-lock.yaml", "yarn.lock"] {
                if path.with_file_name(unsupported).is_file() {
                    return Err(format!("unsupported JavaScript lock file: {unsupported}"));
                }
            }
        }
    }
    writes.insert(
        root.join("CHANGELOG.md"),
        render_changelog(root, new_version, &info, &kind, &changed)?,
    );
    verify_release_inputs(&inputs)?;
    Ok(Some(FileReleasePlan {
        inputs,
        old_version,
        new_version,
        kind,
        writes,
    }))
}

pub fn apply_file_release_plan(plan: &FileReleasePlan) -> Result<(), String> {
    verify_release_inputs(&plan.inputs)?;
    for (path, payload) in &plan.writes {
        crate::memory::atomic_write(path, payload).map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn release_input(path: &Path) -> Result<Option<Vec<u8>>, String> {
    match fs::read(path) {
        Ok(payload) => Ok(Some(payload)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!(
            "cannot snapshot release input {}: {error}",
            path.display()
        )),
    }
}

pub(crate) fn verify_release_inputs(
    inputs: &BTreeMap<PathBuf, Option<Vec<u8>>>,
) -> Result<(), String> {
    for (path, before) in inputs {
        if &release_input(path)? != before {
            return Err(format!(
                "release input changed concurrently: {}",
                path.display()
            ));
        }
    }
    Ok(())
}

#[cfg(all(test, bridgeforge_factory_tests))]
#[path = "../../../../../scripts/tests/unit/core_release.rs"]
mod tests;
