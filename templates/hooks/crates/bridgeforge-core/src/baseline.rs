use crate::{BaselineReport, BaselineState, ProcessRequest, ProcessRunner, SystemProcessRunner};
use regex::Regex;
use serde::Deserialize;
use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

struct UniqueJson(Value);

impl<'de> Deserialize<'de> for UniqueJson {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct UniqueVisitor;
        impl<'de> Visitor<'de> for UniqueVisitor {
            type Value = UniqueJson;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a JSON value without duplicate object keys")
            }

            fn visit_bool<E: de::Error>(self, value: bool) -> Result<Self::Value, E> {
                Ok(UniqueJson(Value::Bool(value)))
            }

            fn visit_i64<E: de::Error>(self, value: i64) -> Result<Self::Value, E> {
                Ok(UniqueJson(Value::Number(value.into())))
            }

            fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
                Ok(UniqueJson(Value::Number(value.into())))
            }

            fn visit_f64<E: de::Error>(self, value: f64) -> Result<Self::Value, E> {
                serde_json::Number::from_f64(value)
                    .map(Value::Number)
                    .map(UniqueJson)
                    .ok_or_else(|| E::custom("non-finite JSON number"))
            }

            fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
                Ok(UniqueJson(Value::String(value.to_string())))
            }

            fn visit_string<E: de::Error>(self, value: String) -> Result<Self::Value, E> {
                Ok(UniqueJson(Value::String(value)))
            }

            fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(UniqueJson(Value::Null))
            }

            fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(UniqueJson(Value::Null))
            }

            fn visit_seq<A: SeqAccess<'de>>(
                self,
                mut sequence: A,
            ) -> Result<Self::Value, A::Error> {
                let mut values = Vec::new();
                while let Some(value) = sequence.next_element::<UniqueJson>()? {
                    values.push(value.0);
                }
                Ok(UniqueJson(Value::Array(values)))
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut values = serde_json::Map::new();
                while let Some((key, value)) = map.next_entry::<String, UniqueJson>()? {
                    if values.insert(key.clone(), value.0).is_some() {
                        return Err(de::Error::custom(format!("duplicate JSON key: {key}")));
                    }
                }
                Ok(UniqueJson(Value::Object(values)))
            }
        }
        deserializer.deserialize_any(UniqueVisitor)
    }
}

pub(crate) fn parse_unique_json(payload: &[u8], label: &str) -> Result<Value, String> {
    serde_json::from_slice::<UniqueJson>(payload)
        .map(|value| value.0)
        .map_err(|error| format!("{label} is invalid JSON: {error}"))
}

pub(crate) fn compatibility_baseline(contract: &Value) -> Result<crate::release::SemVer, String> {
    let floor = contract["compatibility_baseline"]
        .as_str()
        .ok_or("managed contract compatibility_baseline is missing")?
        .parse::<crate::release::SemVer>()?;
    let release = contract["release_version"]
        .as_str()
        .ok_or("managed contract release_version is missing")?
        .parse::<crate::release::SemVer>()?;
    if floor > release {
        return Err("compatibility_baseline is newer than the product release".into());
    }
    Ok(floor)
}

fn sha(payload: &[u8]) -> String {
    let normalized = if payload.contains(&0) {
        payload.to_vec()
    } else {
        String::from_utf8_lossy(payload)
            .replace("\r\n", "\n")
            .replace('\r', "\n")
            .into_bytes()
    };
    format!("sha256:{:x}", Sha256::digest(normalized))
}

fn raw_sha(payload: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(payload))
}

fn safe_target(root: &Path, value: &Value, label: &str) -> Result<PathBuf, String> {
    let raw = value
        .as_str()
        .ok_or_else(|| format!("{label} must be a string"))?;
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
    Ok(root.join(relative))
}

fn marker_block(payload: &[u8], begin: &str, end: &str) -> Result<Vec<u8>, String> {
    let text = String::from_utf8(payload.to_vec())
        .map_err(|_| "managed text is not UTF-8")?
        .replace("\r\n", "\n")
        .replace('\r', "\n");
    let lines = text.split_inclusive('\n').collect::<Vec<_>>();
    let starts = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.trim_end_matches('\n') == begin)
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    let stops = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.trim_end_matches('\n') == end)
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    if starts.len() != 1 || stops.len() != 1 || starts[0] >= stops[0] {
        return Err(format!(
            "managed markers must appear exactly once: {begin} / {end}"
        ));
    }
    Ok(lines[starts[0]..=stops[0]].concat().into_bytes())
}

fn exact_keys(value: &Value, expected: &[&str], label: &str) -> Result<(), String> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("{label} must be an object"))?;
    let actual = object.keys().map(String::as_str).collect::<BTreeSet<_>>();
    let expected = expected.iter().copied().collect::<BTreeSet<_>>();
    if actual != expected {
        return Err(format!("{label} fields are not schema exact"));
    }
    Ok(())
}

fn hash_string(value: &Value) -> bool {
    value.as_str().is_some_and(|value| {
        value.len() == 71
            && value.starts_with("sha256:")
            && value[7..]
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    })
}

fn safe_relative_string(value: &Value) -> bool {
    let Some(raw) = value.as_str() else {
        return false;
    };
    !raw.is_empty()
        && !raw.contains('\x5c')
        && !raw.contains(['*', '?', '['])
        && !Path::new(raw).is_absolute()
        && !Path::new(raw).components().any(|part| {
            matches!(
                part,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
}

fn validate_contract(value: &Value) -> Result<(), String> {
    exact_keys(
        value,
        &[
            "schema_version",
            "release_version",
            "host",
            "stamp",
            "contract_target",
            "assets",
            "baseline_model",
            "compatibility_baseline",
            "generated_assets",
        ],
        "current baseline top-level",
    )?;
    if value["schema_version"].as_u64() != Some(4)
        || value["host"].as_str() != Some("codex")
        || value["baseline_model"].as_str() != Some("current-only")
        || value["contract_target"].as_str() != Some(".codex/managed-skeleton.json")
        || !safe_relative_string(&value["stamp"])
    {
        return Err("current baseline identity is invalid".into());
    }
    value["release_version"]
        .as_str()
        .ok_or("current baseline release_version is missing")?
        .parse::<crate::release::SemVer>()?;
    compatibility_baseline(value)?;
    let assets = value["assets"]
        .as_array()
        .filter(|assets| !assets.is_empty())
        .ok_or("current baseline has no assets")?;
    let allowed_asset = [
        "id",
        "source",
        "target",
        "strategy",
        "current_sha256",
        "render",
        "agents_zones",
        "managed_blocks",
        "merge_policy",
        "merge_validation",
        "region",
    ];
    let id_pattern = Regex::new(r"^[a-z0-9][a-z0-9._-]*$").map_err(|error| error.to_string())?;
    let mut ids = BTreeSet::new();
    let mut targets = BTreeSet::new();
    for asset in assets {
        let object = asset
            .as_object()
            .ok_or("current baseline contains a non-object asset")?;
        if object
            .keys()
            .any(|key| !allowed_asset.contains(&key.as_str()))
        {
            return Err("current baseline asset has non-schema fields".into());
        }
        let id = asset["id"].as_str().ok_or("asset id is missing")?;
        let target = asset["target"].as_str().ok_or("asset target is missing")?;
        if !id_pattern.is_match(id)
            || !ids.insert(id.to_string())
            || !safe_relative_string(&asset["source"])
            || !safe_relative_string(&asset["target"])
            || !targets.insert(target.to_lowercase())
            || !matches!(
                asset["strategy"].as_str(),
                Some("whole" | "merge" | "region" | "seed")
            )
            || !hash_string(&asset["current_sha256"])
        {
            return Err(format!("invalid or duplicate asset identity: {id}"));
        }
        if let Some(managed) = asset.get("managed_blocks") {
            if !managed.is_object()
                || !managed["headings"].is_array()
                || !managed["keyed_tables"].is_array()
                || !hash_string(&managed["current_projection_sha256"])
            {
                return Err(format!("asset {id} Markdown ownership is invalid"));
            }
        }
        if let Some(zones) = asset.get("agents_zones") {
            exact_keys(
                zones,
                &["format", "public", "project"],
                &format!("asset {id} AGENTS zones"),
            )?;
            exact_keys(
                &zones["public"],
                &["begin", "end", "current_sha256"],
                &format!("asset {id} AGENTS public zone"),
            )?;
            exact_keys(
                &zones["project"],
                &[
                    "begin",
                    "end",
                    "required_headings",
                    "required_content_headings",
                ],
                &format!("asset {id} AGENTS project zone"),
            )?;
            if zones["format"].as_str() != Some("bridgeforge-agents-zones")
                || !zones["public"]["begin"].is_string()
                || !zones["public"]["end"].is_string()
                || !hash_string(&zones["public"]["current_sha256"])
                || !zones["project"]["begin"].is_string()
                || !zones["project"]["end"].is_string()
                || !zones["project"]["required_headings"].is_array()
                || !zones["project"]["required_content_headings"].is_array()
            {
                return Err(format!("asset {id} AGENTS ownership is invalid"));
            }
        }
        if asset["strategy"].as_str() == Some("region") {
            let region = &asset["region"];
            if !region["begin"].is_string()
                || !region["end"].is_string()
                || !hash_string(&region["current_sha256"])
            {
                return Err(format!("asset {id} region ownership is invalid"));
            }
        }
        if asset["strategy"].as_str() == Some("merge") {
            let validation = &asset["merge_validation"];
            match asset["merge_policy"].as_str() {
                Some("git-attributes-default-lf")
                    if validation["format"].as_str() == Some("git-attributes-default-lf-v1")
                        && validation["required"]
                            == serde_json::json!({"pattern": "*", "text": "auto", "eol": "lf"}) => {
                }
                Some("codex-hooks")
                    if validation["format"].as_str() == Some("codex-hooks-current-v1")
                        && validation["required_handlers"].is_array() => {}
                Some(_) | None
                    if validation["format"].as_str() == Some("json-subset-current-v1")
                        && validation["required"].is_object() => {}
                _ => return Err(format!("asset {id} merge ownership is invalid")),
            }
        }
    }
    let generated = value["generated_assets"]
        .as_array()
        .ok_or("current baseline generated assets are missing")?;
    let generated_fields = [
        "id",
        "source_root",
        "target_source_root",
        "manifest",
        "lockfile",
        "binary_targets",
        "receipt_target",
        "build",
        "self_test",
        "source_tree_sha256",
        "lockfile_sha256",
        "build_recipe_sha256",
        "self_test_sha256",
    ];
    for item in generated {
        exact_keys(item, &generated_fields, "generated asset")?;
        let id = item["id"].as_str().ok_or("generated asset id is missing")?;
        if !id_pattern.is_match(id)
            || !ids.insert(id.to_string())
            || !safe_relative_string(&item["source_root"])
            || !safe_relative_string(&item["target_source_root"])
            || !safe_relative_string(&item["receipt_target"])
            || !item["binary_targets"].is_object()
            || !hash_string(&item["source_tree_sha256"])
            || !hash_string(&item["lockfile_sha256"])
            || !hash_string(&item["build_recipe_sha256"])
            || !hash_string(&item["self_test_sha256"])
        {
            return Err(format!("generated asset is invalid: {id}"));
        }
    }
    Ok(())
}

fn load(path: &Path) -> Result<Value, String> {
    let bytes = fs::read(path).map_err(|error| format!("cannot read current baseline: {error}"))?;
    let value = parse_unique_json(&bytes, "current baseline")?;
    validate_contract(&value)?;
    Ok(value)
}

struct TempDirectory(PathBuf);

impl Drop for TempDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn isolated_git_request(cwd: &Path, args: &[&str]) -> ProcessRequest {
    let mut request = ProcessRequest::new("git", cwd);
    request.args = args.iter().map(OsString::from).collect();
    request.timeout = Duration::from_secs(30);
    request.env.insert("GIT_ATTR_NOSYSTEM".into(), "1".into());
    request.env_remove = std::env::vars_os()
        .filter_map(|(key, _)| {
            key.to_string_lossy()
                .to_ascii_uppercase()
                .starts_with("GIT_")
                .then_some(key)
        })
        .collect();
    request
}

fn verify_gitattributes(payload: &[u8]) -> Result<(), String> {
    std::str::from_utf8(payload).map_err(|_| ".gitattributes is not valid UTF-8".to_string())?;
    let token = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "bridgeforge-gitattributes-{}-{token}",
        std::process::id()
    ));
    fs::create_dir(&root).map_err(|error| format!("cannot create attribute sandbox: {error}"))?;
    let temporary = TempDirectory(root.clone());
    fs::write(root.join(".gitattributes"), payload)
        .map_err(|error| format!("cannot prepare attribute sandbox: {error}"))?;
    let global = root.join("global-attributes");
    fs::write(&global, b"").map_err(|error| error.to_string())?;
    let initialized = SystemProcessRunner
        .run(&isolated_git_request(&root, &["init", "-q"]))
        .map_err(|error| format!("cannot initialize attribute sandbox: {error}"))?;
    if initialized.code != 0 || initialized.timed_out {
        return Err(format!(
            "cannot initialize attribute sandbox: {}",
            String::from_utf8_lossy(&initialized.stderr).trim()
        ));
    }
    let probes = [
        "BRIDGEFORGE_DEFAULT_EOL_PROBE",
        "nested/BRIDGEFORGE_DEFAULT_EOL_PROBE",
        ".codex/BRIDGEFORGE_DEFAULT_EOL_PROBE.py",
        "doc/BRIDGEFORGE_DEFAULT_EOL_PROBE.md",
    ];
    let global_arg = format!("core.attributesFile={}", global.display());
    let mut args = vec!["-c", global_arg.as_str(), "check-attr", "text", "eol", "--"];
    args.extend(probes);
    let checked = SystemProcessRunner
        .run(&isolated_git_request(&root, &args))
        .map_err(|error| format!("cannot evaluate .gitattributes with Git: {error}"))?;
    drop(temporary);
    if checked.code != 0 || checked.timed_out {
        return Err(format!(
            "cannot evaluate .gitattributes with Git: {}",
            String::from_utf8_lossy(&checked.stderr).trim()
        ));
    }
    let mut states = probes
        .iter()
        .map(|probe| ((*probe).to_string(), BTreeMap::<String, String>::new()))
        .collect::<BTreeMap<_, _>>();
    for line in String::from_utf8_lossy(&checked.stdout).lines() {
        let fields = line.rsplitn(3, ": ").collect::<Vec<_>>();
        if fields.len() != 3 {
            return Err("Git returned an invalid .gitattributes evaluation".into());
        }
        let (value, attribute, path) = (fields[0], fields[1], fields[2]);
        let state = states
            .get_mut(path)
            .ok_or("Git returned an unknown .gitattributes probe")?;
        state.insert(attribute.to_string(), value.to_string());
    }
    if states.values().all(|state| {
        state.get("text").map(String::as_str) == Some("auto")
            && state.get("eol").map(String::as_str) == Some("lf")
    }) {
        Ok(())
    } else {
        Err(".gitattributes default LF policy is missing or overridden".into())
    }
}

fn canonical_json(value: &Value) -> Value {
    match value {
        Value::Object(object) => Value::Object(
            object
                .iter()
                .map(|(key, value)| (key.clone(), canonical_json(value)))
                .collect::<BTreeMap<_, _>>()
                .into_iter()
                .collect(),
        ),
        Value::Array(values) => Value::Array(values.iter().map(canonical_json).collect()),
        value => value.clone(),
    }
}

fn canonical_sha(value: &Value) -> Result<String, String> {
    serde_json::to_vec(&canonical_json(value))
        .map(|payload| raw_sha(&payload))
        .map_err(|error| error.to_string())
}

fn deep_subset(expected: &Value, actual: &Value, path: &str) -> Result<(), String> {
    if let Some(expected) = expected.as_object() {
        let actual = actual
            .as_object()
            .ok_or_else(|| format!("managed JSON value drifted: {path}"))?;
        for (key, value) in expected {
            let child = actual
                .get(key)
                .ok_or_else(|| format!("managed JSON key is missing: {path}.{key}"))?;
            deep_subset(value, child, &format!("{path}.{key}"))?;
        }
    } else if expected != actual {
        return Err(format!("managed JSON value drifted: {path}"));
    }
    Ok(())
}

fn heading_section(payload: &[u8], heading: &str) -> Result<Vec<u8>, String> {
    let text = String::from_utf8(payload.to_vec())
        .map_err(|_| "managed Markdown is not UTF-8".to_string())?
        .replace("\r\n", "\n")
        .replace('\r', "\n");
    let lines = text.split_inclusive('\n').collect::<Vec<_>>();
    let starts = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.trim_end_matches('\n') == heading)
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    let level = heading
        .chars()
        .take_while(|character| *character == '#')
        .count();
    if starts.len() != 1 || level == 0 {
        return Err(format!(
            "Markdown heading is missing or duplicated: {heading}"
        ));
    }
    let stop = lines
        .iter()
        .enumerate()
        .skip(starts[0] + 1)
        .find(|(_, line)| {
            let trimmed = line.trim_start();
            let candidate = trimmed
                .chars()
                .take_while(|character| *character == '#')
                .count();
            candidate > 0 && candidate <= level && trimmed.chars().nth(candidate) == Some(' ')
        })
        .map(|(index, _)| index)
        .unwrap_or(lines.len());
    Ok(lines[starts[0]..stop].concat().into_bytes())
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
        let mut key = line
            .trim()
            .trim_matches('|')
            .split('|')
            .next()
            .unwrap_or("")
            .trim()
            .to_string();
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

fn markdown_projection(payload: &[u8], managed: &Value) -> Result<Value, String> {
    let mut headings = serde_json::Map::new();
    for heading in managed["headings"].as_array().into_iter().flatten() {
        let heading = heading.as_str().ok_or("managed heading is invalid")?;
        headings.insert(
            heading.to_string(),
            Value::String(sha(&heading_section(payload, heading)?)),
        );
    }
    let mut tables = serde_json::Map::new();
    for table in managed["keyed_tables"].as_array().into_iter().flatten() {
        let heading = table["heading"]
            .as_str()
            .ok_or("managed table heading is invalid")?;
        let rows = table_rows(&heading_section(payload, heading)?)?;
        let mut projected = serde_json::Map::new();
        for raw in table["managed_keys"].as_array().into_iter().flatten() {
            let key = raw
                .as_str()
                .ok_or("managed Markdown key is invalid")?
                .trim()
                .trim_matches('`')
                .to_lowercase();
            let row = rows
                .get(&key)
                .ok_or_else(|| format!("managed Markdown row is missing: {heading} :: {key}"))?;
            projected.insert(key, Value::String(sha(row)));
        }
        tables.insert(heading.to_string(), Value::Object(projected));
    }
    Ok(serde_json::json!({"headings": headings, "keyed_tables": tables}))
}

pub(crate) fn hook_handlers(document: &Value) -> Result<BTreeMap<String, Value>, String> {
    let hooks = document["hooks"]
        .as_object()
        .ok_or("hooks.json has no hooks object")?;
    let mut handlers = BTreeMap::new();
    for (event, groups) in hooks {
        let groups = groups
            .as_array()
            .ok_or_else(|| format!("hooks.json event is invalid: {event}"))?;
        for group in groups {
            let matcher = group["matcher"].as_str().unwrap_or("");
            let entries = group["hooks"]
                .as_array()
                .ok_or_else(|| format!("hooks.json group is invalid: {event}"))?;
            for handler in entries {
                let Some(id) = handler["bridgeforgeCodexId"].as_str() else {
                    continue;
                };
                let record =
                    serde_json::json!({"event": event, "matcher": matcher, "handler": handler});
                if handlers.insert(id.to_string(), record).is_some() {
                    return Err(format!("managed hook handler is duplicated: {id}"));
                }
            }
        }
    }
    Ok(handlers)
}

fn verify_hooks(payload: &[u8], asset: &Value) -> Result<(), String> {
    let actual = parse_unique_json(payload, "hooks.json")?;
    let required = asset["merge_validation"]["required_handlers"]
        .as_array()
        .ok_or("hooks required_handlers is missing")?;
    let handlers = hook_handlers(&actual)?;
    let expected = required
        .iter()
        .map(|record| {
            record["id"]
                .as_str()
                .map(str::to_string)
                .ok_or("managed hook id is missing")
        })
        .collect::<Result<BTreeSet<_>, _>>()?;
    let actual_managed = handlers
        .keys()
        .filter(|id| id.starts_with("bridgeforge-codex.project-hook.v1:"))
        .cloned()
        .collect::<BTreeSet<_>>();
    if expected.len() != required.len() || actual_managed != expected {
        return Err("managed hook handler identity set drifted".into());
    }
    for required in required {
        let id = required["id"]
            .as_str()
            .ok_or("managed hook id is missing")?;
        let record = handlers
            .get(id)
            .ok_or_else(|| format!("managed hook is missing: {id}"))?;
        if record["event"] != required["event"] {
            return Err(format!("managed hook event drifted: {id}"));
        }
        if record["matcher"] != required["matcher"] {
            return Err(format!("managed hook matcher drifted: {id}"));
        }
        if canonical_sha(&record["handler"])? != required["sha256"].as_str().unwrap_or("") {
            return Err(format!("managed hook payload drifted: {id}"));
        }
    }
    if let Some(top) = asset["merge_validation"].get("managed_top_level") {
        deep_subset(top, &actual, "hooks.json")?;
    }
    Ok(())
}

pub(crate) fn verify_asset_payload(asset: &Value, payload: &[u8]) -> Result<String, String> {
    let id = asset["id"]
        .as_str()
        .ok_or("asset id is missing")?
        .to_string();
    let strategy = asset["strategy"]
        .as_str()
        .ok_or("asset strategy is missing")?;
    if let Some(blocks) = asset
        .get("managed_blocks")
        .filter(|value| value.is_object())
    {
        let actual = canonical_sha(&markdown_projection(payload, blocks)?)?;
        if actual != blocks["current_projection_sha256"].as_str().unwrap_or("") {
            return Err(format!("managed Markdown projection drifted: {id}"));
        }
        return Ok(id);
    }
    if strategy == "seed" {
        return Ok(format!("skip:{id}"));
    }
    if let Some(zones) = asset.get("agents_zones") {
        let public = &zones["public"];
        let block = marker_block(
            payload,
            public["begin"]
                .as_str()
                .ok_or("AGENTS public begin is missing")?,
            public["end"]
                .as_str()
                .ok_or("AGENTS public end is missing")?,
        )?;
        if sha(&block) != public["current_sha256"].as_str().unwrap_or("") {
            return Err(format!("managed AGENTS public zone drifted: {id}"));
        }
        let project = &zones["project"];
        marker_block(
            payload,
            project["begin"]
                .as_str()
                .ok_or("AGENTS project begin is missing")?,
            project["end"]
                .as_str()
                .ok_or("AGENTS project end is missing")?,
        )?;
    } else if strategy == "region" {
        let region = &asset["region"];
        let block = marker_block(
            payload,
            region["begin"].as_str().ok_or("region begin is missing")?,
            region["end"].as_str().ok_or("region end is missing")?,
        )?;
        if sha(&block) != region["current_sha256"].as_str().unwrap_or("") {
            return Err(format!("managed region drifted: {id}"));
        }
    } else if strategy == "merge" {
        match asset["merge_policy"].as_str() {
            Some("git-attributes-default-lf") => verify_gitattributes(payload)?,
            Some("codex-hooks") => verify_hooks(payload, asset)?,
            _ => {
                let actual = parse_unique_json(payload, "managed JSON")?;
                deep_subset(
                    &asset["merge_validation"]["required"],
                    &actual,
                    asset["target"].as_str().unwrap_or("managed JSON"),
                )?;
            }
        }
    } else if sha(payload) != asset["current_sha256"].as_str().unwrap_or("") {
        return Err(format!("managed asset drifted: {id}: {}", asset["target"]));
    }
    Ok(id)
}

fn verify_asset(root: &Path, asset: &Value) -> Result<String, String> {
    let id = asset["id"].as_str().ok_or("asset id is missing")?;
    if asset["strategy"].as_str() == Some("seed") {
        return Ok(format!("skip:{id}"));
    }
    let target = safe_target(root, &asset["target"], &format!("asset {id} target"))?;
    let payload = fs::read(&target)
        .map_err(|_| format!("managed asset is missing: {id}: {}", asset["target"]))?;
    verify_asset_payload(asset, &payload)
}

fn platform_key() -> Result<&'static str, String> {
    if !cfg!(target_arch = "x86_64") {
        return Err("unsupported generated-asset architecture".into());
    }
    if cfg!(windows) {
        Ok("windows-x86_64")
    } else if cfg!(target_os = "linux") {
        Ok("linux-x86_64")
    } else if cfg!(target_os = "macos") {
        Ok("macos-x86_64")
    } else {
        Err("unsupported generated-asset operating system".into())
    }
}

fn verify_generated(root: &Path, generated: &Value) -> Result<String, String> {
    let id = generated["id"]
        .as_str()
        .ok_or("generated asset id is missing")?;
    let key = platform_key()?;
    let binary = safe_target(root, &generated["binary_targets"][key], "generated binary")?;
    let receipt = safe_target(root, &generated["receipt_target"], "generated receipt")?;
    let binary_payload =
        fs::read(&binary).map_err(|_| format!("generated binary is missing: {id}"))?;
    let document: Value = serde_json::from_slice(
        &fs::read(&receipt).map_err(|_| format!("generated receipt is missing: {id}"))?,
    )
    .map_err(|error| format!("invalid generated receipt: {error}"))?;
    if document["schema_version"].as_u64() != Some(2)
        || document["generated_asset_id"].as_str() != Some(id)
        || document["platform"].as_str() != Some(key)
        || document["binary_sha256"].as_str() != Some(raw_sha(&binary_payload).as_str())
        || document["source_tree_sha256"] != generated["source_tree_sha256"]
        || document["lockfile_sha256"] != generated["lockfile_sha256"]
        || document["build_recipe_sha256"] != generated["build_recipe_sha256"]
        || document["self_test_sha256"] != generated["self_test_sha256"]
    {
        return Err(format!("generated asset receipt drifted: {id}"));
    }
    Ok(id.to_string())
}

pub fn verify(
    root: &Path,
    contract_path: Option<&Path>,
    verify_generated_runtime: bool,
) -> Result<BaselineReport, String> {
    let contract_path = contract_path
        .map(Path::to_path_buf)
        .unwrap_or_else(|| root.join(".codex/managed-skeleton.json"));
    let contract = load(&contract_path)?;
    let version = contract["release_version"].as_str().unwrap().to_string();
    let stamp = safe_target(root, &contract["stamp"], "current baseline stamp")?;
    let factory = root.join("templates/managed-skeleton.json").is_file();
    let stamp_version = if stamp.is_file() {
        fs::read_to_string(&stamp)
            .map_err(|error| error.to_string())?
            .trim()
            .to_string()
    } else if factory {
        fs::read_to_string(root.join("VERSION"))
            .map_err(|_| "current baseline version stamp is missing")?
            .trim()
            .to_string()
    } else {
        return Err("current baseline version stamp is missing".into());
    };
    if stamp_version != version {
        return Err(format!(
            "current baseline identity mismatch: contract={version}, stamp={stamp_version}"
        ));
    }
    verify_contract_assets(
        root,
        &contract_path,
        &contract,
        &version,
        verify_generated_runtime,
    )
}

pub fn verify_prospective(
    root: &Path,
    contract_path: &Path,
    expected_version: &str,
    verify_generated_runtime: bool,
) -> Result<BaselineReport, String> {
    let contract = load(contract_path)?;
    let version = contract["release_version"]
        .as_str()
        .ok_or("current baseline release_version is missing")?;
    if version != expected_version {
        return Err(format!(
            "prospective baseline identity mismatch: contract={version}, expected={expected_version}"
        ));
    }
    verify_contract_assets(
        root,
        contract_path,
        &contract,
        version,
        verify_generated_runtime,
    )
}

fn verify_contract_assets(
    root: &Path,
    contract_path: &Path,
    contract: &Value,
    version: &str,
    verify_generated_runtime: bool,
) -> Result<BaselineReport, String> {
    let mut checked = Vec::new();
    let mut skipped = Vec::new();
    let factory = root.join("templates/managed-skeleton.json").is_file();
    for asset in contract["assets"].as_array().unwrap() {
        if factory
            && asset["id"].as_str() == Some("codex.doc.readme")
            && asset["managed_blocks"].is_object()
        {
            skipped.push("codex.doc.readme".into());
            continue;
        }
        let result = verify_asset(root, asset)?;
        if let Some(id) = result.strip_prefix("skip:") {
            skipped.push(id.to_string())
        } else {
            checked.push(result)
        }
    }
    for generated in contract["generated_assets"].as_array().unwrap() {
        if verify_generated_runtime {
            checked.push(verify_generated(root, generated)?)
        } else {
            skipped.push(generated["id"].as_str().unwrap_or("").to_string())
        }
    }
    let contract_bytes = fs::read(&contract_path).map_err(|error| error.to_string())?;
    let fingerprint = sha(format!(
        "{}\n{}\n{}",
        version,
        sha(&contract_bytes),
        checked.join("\n")
    )
    .as_bytes());
    Ok(BaselineReport {
        schema: 1,
        state: BaselineState::Clean,
        project_version: Some(version.to_string()),
        incoming_version: None,
        fingerprint: Some(fingerprint),
        reasons: Vec::new(),
        details: serde_json::json!({"checked": checked, "skipped": skipped}),
    })
}

fn git_index_blob(root: &Path, path: &str, runner: &dyn ProcessRunner) -> Result<Vec<u8>, String> {
    let mut request = ProcessRequest::new("git", root);
    request.args = vec![OsString::from("show"), OsString::from(format!(":{path}"))];
    request.timeout = std::time::Duration::from_secs(45);
    let output = runner
        .run(&request)
        .map_err(|error| format!("cannot launch git index reader: {error}"))?;
    if output.timed_out || output.code != 0 {
        return Err(format!(
            "managed asset is missing from staged index: {path}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(output.stdout)
}

pub fn verify_index(root: &Path, runner: &dyn ProcessRunner) -> Result<BaselineReport, String> {
    let contract_bytes = git_index_blob(root, ".codex/managed-skeleton.json", runner)?;
    let contract = parse_unique_json(&contract_bytes, "staged current baseline")?;
    validate_contract(&contract)?;
    let version = contract["release_version"]
        .as_str()
        .ok_or("staged current baseline release_version is missing")?
        .to_string();
    let stamp_path = contract["stamp"]
        .as_str()
        .ok_or("staged stamp path is missing")?;
    let staged_identity = if root.join("templates/managed-skeleton.json").is_file() {
        "VERSION"
    } else {
        stamp_path
    };
    let stamp = String::from_utf8(git_index_blob(root, staged_identity, runner)?)
        .map_err(|_| "staged version stamp is not UTF-8")?;
    if stamp.trim() != version {
        return Err(format!(
            "staged current baseline identity mismatch: contract={version}, stamp={}",
            stamp.trim()
        ));
    }
    let mut checked = Vec::new();
    let mut skipped = Vec::new();
    let factory = root.join("templates/managed-skeleton.json").is_file();
    for asset in contract["assets"]
        .as_array()
        .ok_or("staged assets are missing")?
    {
        let id = asset["id"].as_str().ok_or("staged asset id is missing")?;
        if factory && id == "codex.doc.readme" && asset["managed_blocks"].is_object() {
            skipped.push(id.to_string());
            continue;
        }
        if asset["strategy"].as_str() == Some("seed") {
            skipped.push(id.to_string());
            continue;
        }
        let target = asset["target"]
            .as_str()
            .ok_or("staged asset target is missing")?;
        let payload = git_index_blob(root, target, runner)?;
        let result = verify_asset_payload(asset, &payload)?;
        if let Some(id) = result.strip_prefix("skip:") {
            skipped.push(id.to_string());
        } else {
            checked.push(result);
        }
    }
    if let Some(generated_assets) = contract["generated_assets"].as_array() {
        for generated in generated_assets {
            skipped.push(generated["id"].as_str().unwrap_or("").to_string());
        }
    }
    Ok(BaselineReport {
        schema: 1,
        state: BaselineState::Clean,
        project_version: Some(version.clone()),
        incoming_version: None,
        fingerprint: Some(sha(format!(
            "index\n{version}\n{}\n{}",
            sha(&contract_bytes),
            checked.join("\n")
        )
        .as_bytes())),
        reasons: Vec::new(),
        details: serde_json::json!({"source": "index", "checked": checked, "skipped": skipped}),
    })
}

#[cfg(all(test, bridgeforge_factory_tests))]
#[path = "../../../../../scripts/tests/unit/core_baseline.rs"]
mod tests;
