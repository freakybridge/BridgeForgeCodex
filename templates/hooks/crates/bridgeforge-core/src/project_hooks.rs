//! Project-owned Rust hooks. Only an explicit entrypoint is compiled; dependencies
//! come from the verified, locked managed workspace, never from a project manifest.
use crate::{ProcessRequest, ProcessRunner};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const REGISTRY: &str = "bridgeforgeProjectHooks";
const HANDLER_ID: &str = "bridgeforgeProjectHookId";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Hook {
    pub id: String,
    pub events: Vec<Event>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Event {
    pub event: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub matcher: String,
    #[serde(default = "default_timeout")]
    pub timeout: u64,
}

fn default_timeout() -> u64 {
    150
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Registry {
    schema_version: u32,
    hooks: Vec<Hook>,
}

fn token(text: &str) -> bool {
    !text.is_empty()
        && text.len() <= 64
        && text
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
        && text.as_bytes()[0].is_ascii_lowercase()
}

pub(crate) fn hooks(document: &Value) -> Result<Vec<Hook>, String> {
    let Some(value) = document.get(REGISTRY) else {
        return Ok(Vec::new());
    };
    let registry: Registry = serde_json::from_value(value.clone())
        .map_err(|error| format!("invalid project Rust hook registry: {error}"))?;
    if registry.schema_version != 1 || registry.hooks.len() > 32 {
        return Err("unsupported project Rust hook registry".into());
    }
    let mut ids = BTreeSet::new();
    for hook in &registry.hooks {
        if !token(&hook.id)
            || !ids.insert(&hook.id)
            || hook.events.is_empty()
            || hook.events.len() > 16
        {
            return Err("invalid or duplicate project Rust hook id/events".into());
        }
        let mut events = BTreeSet::new();
        for event in &hook.events {
            if !matches!(
                event.event.as_str(),
                "SessionStart" | "Stop" | "PreToolUse" | "PostToolUse" | "PostCompact"
            ) || !events.insert((&event.event, &event.matcher))
                || event.timeout == 0
                || event.timeout > 900
                || event.matcher.contains(['\r', '\n'])
                || event.args.iter().any(|arg| !token(arg))
                || event.args.len() > 16
            {
                return Err(format!("invalid project Rust hook event: {}", hook.id));
            }
        }
    }
    Ok(registry.hooks)
}

impl Hook {
    pub fn source(&self) -> String {
        format!(".codex/hooks/project_{}/entrypoint.rs", self.id)
    }
    pub fn binary(&self) -> String {
        format!(
            ".codex/bin/project_{}{}",
            self.id,
            std::env::consts::EXE_SUFFIX
        )
    }
    pub fn receipt(&self) -> String {
        format!(".codex/bin/build-receipt-project-{}.json", self.id)
    }
}

pub(crate) fn read(root: &Path, relative: &str) -> Result<Option<Vec<u8>>, String> {
    let target = root.join(relative);
    for path in target.ancestors() {
        match fs::symlink_metadata(path) {
            Ok(metadata) => {
                if crate::memory::is_link_or_reparse(path).map_err(|e| e.to_string())? {
                    return Err(format!(
                        "project Rust hook path traverses a link: {relative}"
                    ));
                }
                if path == target && !metadata.is_file() {
                    return Err(format!(
                        "project Rust hook target is not a file: {relative}"
                    ));
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.to_string()),
        }
        if path == root {
            break;
        }
    }
    match fs::read(target) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.to_string()),
    }
}

pub(crate) fn render(document: &Value) -> Result<Value, String> {
    if document.get(REGISTRY).is_none() {
        if document["hooks"]
            .as_object()
            .into_iter()
            .flat_map(|o| o.values())
            .flat_map(|v| v.as_array().into_iter().flatten())
            .flat_map(|g| g["hooks"].as_array().into_iter().flatten())
            .any(|h| h.get(HANDLER_ID).is_some())
        {
            return Err("project Rust hook handler has no registry".into());
        }
        return Ok(document.clone());
    }
    let entries = hooks(document)?;
    let mut result = document.clone();
    let Some(groups) = result.get_mut("hooks").and_then(Value::as_object_mut) else {
        if entries.is_empty() {
            return Ok(result);
        }
        return Err("project Rust hooks require hooks object".into());
    };
    // Only generated handlers bearing our separate namespace are replaced.
    for groups in groups.values_mut() {
        let groups = groups.as_array_mut().ok_or("hook event must be an array")?;
        for group in groups.iter_mut() {
            if let Some(handlers) = group["hooks"].as_array_mut() {
                handlers.retain(|handler| handler.get(HANDLER_ID).is_none());
            }
        }
        groups.retain(|group| {
            group["hooks"]
                .as_array()
                .is_none_or(|handlers| !handlers.is_empty())
        });
    }
    for hook in entries {
        for (index, event) in hook.events.iter().enumerate() {
            let args = if event.args.is_empty() {
                String::new()
            } else {
                format!(" {}", event.args.join(" "))
            };
            let group = json!({"matcher":event.matcher,"hooks":[{
                "type":"command", HANDLER_ID:format!("{}:{index}",hook.id),
                "command":format!(".codex/bin/project_{}{args}",hook.id),
                "commandWindows":format!(".codex/bin/project_{}.exe{args}",hook.id),
                "timeout":event.timeout
            }]});
            groups
                .entry(event.event.clone())
                .or_insert_with(|| json!([]))
                .as_array_mut()
                .ok_or("hook event must be an array")?
                .push(group);
        }
    }
    Ok(result)
}

fn sha(payload: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(payload))
}

pub(crate) fn identity(hook: &Hook, source: &[u8], contract: &Value) -> Result<String, String> {
    let core = contract["generated_assets"]
        .as_array()
        .and_then(|items| items.first())
        .ok_or("project Rust hooks require managed generated workspace")?;
    crate::manifest::canonical_sha(&json!({"schema_version":1,"hook":hook,
        "source_sha256":sha(source),"workspace_sha256":core["source_tree_sha256"],
        "lockfile_sha256":core["lockfile_sha256"],"wrapper_version":1,
        "build":crate::manifest::generated_build_recipe(&format!("project_{}",hook.id))}))
}

pub(crate) fn current(root: &Path, hook: &Hook, identity: &str) -> Result<bool, String> {
    let Some(binary) = read(root, &hook.binary())? else {
        return Ok(false);
    };
    let Some(receipt) = read(root, &hook.receipt())? else {
        return Ok(false);
    };
    let Ok(document) = crate::baseline::parse_unique_json(&receipt, "project hook receipt") else {
        return Ok(false);
    };
    Ok(document
        == json!({"schema_version":1,"id":hook.id,"input_sha256":identity,
        "platform":std::env::consts::OS,"binary_sha256":sha(&binary)}))
}

pub(crate) fn owned(root: &Path, hook: &Hook) -> Result<bool, String> {
    let Some(payload) = read(root, &hook.receipt())? else {
        return Ok(false);
    };
    let Ok(receipt) = crate::baseline::parse_unique_json(&payload, "project hook receipt") else {
        return Ok(false);
    };
    let Some(input) = receipt["input_sha256"].as_str() else {
        return Ok(false);
    };
    if !input.starts_with("sha256:") || input.len() != 71 {
        return Ok(false);
    }
    current(root, hook, input)
}

pub(crate) fn verify(root: &Path, contract: &Value, runtime: bool) -> Result<Vec<String>, String> {
    let Some(payload) = read(root, ".codex/hooks.json")? else {
        return Ok(Vec::new());
    };
    let document = crate::baseline::parse_unique_json(&payload, "hooks.json")?;
    if render(&document)? != document {
        return Err("project Rust hook registrations drifted".into());
    }
    let mut checked = Vec::new();
    for hook in hooks(&document)? {
        let source = read(root, &hook.source())?.ok_or("project Rust hook source is missing")?;
        let input = identity(&hook, &source, contract)?;
        if runtime && !current(root, &hook, &input)? {
            return Err(format!(
                "project Rust hook build receipt drifted: {}",
                hook.id
            ));
        }
        checked.push(format!("project-hook:{}:{input}", hook.id));
    }
    Ok(checked)
}

struct Temporary(PathBuf);
impl Drop for Temporary {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// The wrapper intercepts self-test before calling the user's run function.
/// Project Rust source is trusted executable code, not a security sandbox.
pub(crate) fn build(
    workspace: &Path,
    root: &Path,
    contract: &Value,
    inputs: &[(Hook, Vec<u8>)],
    runner: &dyn ProcessRunner,
) -> Result<BTreeMap<PathBuf, Vec<u8>>, String> {
    if inputs.is_empty() {
        return Ok(BTreeMap::new());
    }
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let temporary = Temporary(std::env::temp_dir().join(format!(
        "bridgeforge-project-hooks-{}-{nonce}",
        std::process::id()
    )));
    fs::create_dir(&temporary.0).map_err(|e| e.to_string())?;
    let item = contract["generated_assets"]
        .as_array()
        .and_then(|items| items.first())
        .ok_or("project hooks require managed workspace")?;
    let snapshot = temporary.0.join("workspace");
    let captured =
        crate::project_sync::build_inputs::BuildInputs::capture(workspace, snapshot.clone(), item)?;
    let mut manifest = fs::read_to_string(snapshot.join("Cargo.toml"))
        .map_err(|e| e.to_string())?
        .parse::<toml_edit::DocumentMut>()
        .map_err(|e| e.to_string())?;
    let bins = manifest
        .entry("bin")
        .or_insert(toml_edit::Item::ArrayOfTables(
            toml_edit::ArrayOfTables::new(),
        ))
        .as_array_of_tables_mut()
        .ok_or("managed workspace bin entries are invalid")?;
    let mut expected = BTreeMap::new();
    for relative in crate::manifest::generated_sources(&snapshot)? {
        expected.insert(
            relative.clone(),
            fs::read(snapshot.join(relative)).map_err(|e| e.to_string())?,
        );
    }
    for (hook, source) in inputs {
        std::str::from_utf8(source).map_err(|_| "project Rust hook source must be UTF-8")?;
        let directory = format!("project_{}", hook.id);
        let wrapper = format!("{directory}/main.rs");
        let fingerprint = identity(hook, source, contract)?;
        let response =
            serde_json::to_string(&json!({"id":hook.id,"input_sha256":fingerprint,"status":"ok"}))
                .map_err(|e| e.to_string())?;
        let generated = format!(
            "#![cfg_attr(windows, windows_subsystem = \"windows\")]\nmod entrypoint;\nfn main() {{\nlet args: Vec<String> = std::env::args().skip(1).collect();\nif args == [\"--bridgeforge-self-test\"] {{ println!(\"{{}}\", {response:?}); return; }}\nstd::process::exit(entrypoint::run(args));\n}}\n"
        );
        expected.insert(format!("{directory}/entrypoint.rs"), source.clone());
        expected.insert(wrapper.clone(), generated.into_bytes());
        let mut bin = toml_edit::Table::new();
        bin["name"] = toml_edit::value(format!("project_{}", hook.id));
        bin["path"] = toml_edit::value(wrapper);
        bins.push(bin);
    }
    expected.insert("Cargo.toml".into(), manifest.to_string().into_bytes());
    for (relative, payload) in &expected {
        let path = snapshot.join(relative);
        fs::create_dir_all(path.parent().ok_or("missing source parent")?)
            .map_err(|e| e.to_string())?;
        fs::write(path, payload).map_err(|e| e.to_string())?;
    }
    let check_snapshot = || -> Result<(), String> {
        captured.verify_original_unchanged()?;
        for (relative, payload) in &expected {
            if fs::read(snapshot.join(relative)).map_err(|e| e.to_string())? != *payload {
                return Err(format!("project hook build input drifted: {relative}"));
            }
        }
        Ok(())
    };
    let output_dir = temporary.0.join("output");
    let mut request = ProcessRequest::new("cargo", &snapshot);
    request.args = vec![
        "build".into(),
        "--locked".into(),
        "--profile".into(),
        "release".into(),
        "--manifest-path".into(),
        snapshot.join("Cargo.toml").into_os_string(),
        "--target-dir".into(),
        output_dir.clone().into_os_string(),
    ];
    for (hook, _) in inputs {
        request
            .args
            .extend(["--bin".into(), format!("project_{}", hook.id).into()]);
    }
    request.timeout = Duration::from_secs(900);
    let built = runner.run(&request).map_err(|e| e.to_string())?;
    if built.timed_out || built.code != 0 {
        return Err(format!(
            "project Rust hook build failed: {}",
            String::from_utf8_lossy(&built.stderr)
        ));
    }
    check_snapshot()?;
    let mut writes = BTreeMap::new();
    for (hook, source) in inputs {
        let prefix = format!("project_{}/", hook.id);
        let allowed = expected
            .iter()
            .filter(|(path, _)| !path.starts_with("project_") || path.starts_with(&prefix))
            .map(|(path, payload)| (path.clone(), payload.clone()))
            .collect();
        verify_dependencies(
            &output_dir
                .join("release")
                .join(format!("project_{}.d", hook.id)),
            &snapshot,
            &allowed,
        )?;
        let binary = output_dir.join("release").join(format!(
            "project_{}{}",
            hook.id,
            std::env::consts::EXE_SUFFIX
        ));
        let payload = fs::read(&binary).map_err(|e| e.to_string())?;
        if cfg!(windows) {
            verify_windows_gui(&payload)?;
        }
        let fingerprint = identity(hook, source, contract)?;
        let mut test = ProcessRequest::new(binary.clone().into_os_string(), &snapshot);
        test.args = vec!["--bridgeforge-self-test".into()];
        test.timeout = Duration::from_secs(30);
        let tested = runner.run(&test).map_err(|e| e.to_string())?;
        let actual = crate::baseline::parse_unique_json(&tested.stdout, "project hook self-test")?;
        if tested.timed_out
            || tested.code != 0
            || actual != json!({"id":hook.id,"input_sha256":fingerprint,"status":"ok"})
            || fs::read(&binary).map_err(|e| e.to_string())? != payload
        {
            return Err(format!("project Rust hook self-test failed: {}", hook.id));
        }
        let receipt = json!({"schema_version":1,"id":hook.id,"input_sha256":fingerprint,
            "platform":std::env::consts::OS,"binary_sha256":sha(&payload)});
        writes.insert(root.join(hook.binary()), payload);
        writes.insert(
            root.join(hook.receipt()),
            serde_json::to_vec_pretty(&receipt).map_err(|e| e.to_string())?,
        );
    }
    check_snapshot()?;
    Ok(writes)
}

fn verify_dependencies(
    depfile: &Path,
    snapshot: &Path,
    expected: &BTreeMap<String, Vec<u8>>,
) -> Result<(), String> {
    let text = fs::read_to_string(depfile)
        .map_err(|e| format!("project hook dependency receipt missing: {e}"))?;
    let (_, dependencies) = text
        .lines()
        .next()
        .and_then(|line| line.split_once(": "))
        .ok_or("invalid project hook dependency receipt")?;
    let mut paths = Vec::new();
    let mut token = String::new();
    let mut chars = dependencies.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\'
            && chars
                .peek()
                .is_some_and(|next| matches!(next, ' ' | '\\' | '#'))
        {
            token.push(chars.next().unwrap());
        } else if ch.is_whitespace() {
            if !token.is_empty() {
                paths.push(std::mem::take(&mut token));
            }
        } else {
            token.push(ch);
        }
    }
    if !token.is_empty() {
        paths.push(token);
    }
    if paths.is_empty() {
        return Err("empty project hook dependency receipt".into());
    }
    let allowed = expected
        .keys()
        .map(|relative| fs::canonicalize(snapshot.join(relative)).map_err(|e| e.to_string()))
        .collect::<Result<BTreeSet<_>, _>>()?;
    for path in paths {
        let path = PathBuf::from(path);
        let absolute = if path.is_absolute() {
            path
        } else {
            snapshot.join(path)
        };
        let canonical = fs::canonicalize(&absolute)
            .map_err(|e| format!("cannot verify project hook dependency: {e}"))?;
        if !allowed.contains(&canonical) {
            return Err(format!(
                "project Rust hook used uncaptured dependency: {}",
                absolute.display()
            ));
        }
    }
    Ok(())
}

fn verify_windows_gui(bytes: &[u8]) -> Result<(), String> {
    let offset = bytes
        .get(0x3c..0x40)
        .and_then(|b| b.try_into().ok())
        .map(u32::from_le_bytes)
        .ok_or("invalid project hook PE")? as usize;
    if bytes.get(..2) != Some(b"MZ")
        || bytes.get(offset..offset + 4) != Some(b"PE\0\0")
        || bytes.get(offset + 24 + 68..offset + 24 + 70) != Some(&[2, 0])
    {
        return Err("project Rust hook must use Windows GUI subsystem".into());
    }
    Ok(())
}

#[cfg(all(test, bridgeforge_factory_tests))]
#[path = "../../../../../scripts/tests/unit/core_project_hooks.rs"]
mod tests;
