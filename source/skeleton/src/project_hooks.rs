//! Project-owned Rust hooks, with either legacy managed dependencies or a
//! self-contained, project-owned Cargo package and lockfile.
use crate::{ProcessRequest, ProcessRunner};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

const REGISTRY: &str = "bridgeforgeProjectHooks";
pub const REGISTRY_PATH: &str = ".codex/project-hooks.json";
const HANDLER_ID: &str = "bridgeforgeProjectHookId";

// Reserve the same portable names in working-tree, prospective and index inputs.
pub fn reserved_package_component(name: &str) -> bool {
    name.eq_ignore_ascii_case(".cargo") || name.eq_ignore_ascii_case(".bridgeforge-main.rs")
}

mod package;
pub use package::{Input, capture_input, verify_reads};

fn cache_asset(hook: &Hook, input: &Input, contract: &Value) -> Result<Value, String> {
    let fingerprint = input.identity(hook, contract)?;
    let build = crate::manifest::generated_build_recipe(&format!("project_{}", hook.id));
    let test = json!({"args":["--bridgeforge-self-test"],"expected_json":{"id":hook.id,"input_sha256":fingerprint,"status":"ok"}});
    // User Hook sources can use env!; retain the complete environment identity.
    // A Hook that can reference environment macros must also bind the changing
    // tracking value. Runtime-only `std::env` conservatively takes this path too.
    let references_env = input.package.as_ref().map_or_else(
        || String::from_utf8_lossy(&input.source).contains("env"),
        |files| {
            files
                .values()
                .any(|b| String::from_utf8_lossy(b).contains("env"))
        },
    );
    let vars = std::env::vars_os()
        .filter(|(k, _)| references_env || k != "CODEX_TOOL_CALL_ID")
        .map(|(k, v)| {
            (
                k.to_string_lossy().into_owned(),
                v.to_string_lossy().into_owned(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    Ok(
        json!({"id":format!("project-hook:{}",hook.id),"hook_id":hook.id,"cache_kind":"project-hook-v1",
        "hook_environment":crate::manifest::canonical_sha(&json!(vars))?,
        "source_tree_sha256":fingerprint,"lockfile_sha256":fingerprint,"build":build,
        "build_recipe_sha256":crate::manifest::canonical_sha(&build)?,"self_test":test,
        "self_test_sha256":crate::manifest::canonical_sha(&test)?}),
    )
}

/// Merge registry inputs in memory only. The legacy root key is never serialized
/// back into Codex's native configuration.
pub fn with_registry(document: &Value, sidecar: Option<&[u8]>) -> Result<Value, String> {
    let mut combined = document.clone();
    if let Some(payload) = sidecar {
        let registry = crate::json::parse_unique_json(payload, REGISTRY_PATH)?;
        if let Some(legacy) = document.get(REGISTRY) {
            if legacy != &registry {
                return Err(
                    "conflicting legacy and standalone project Rust hook registries".into(),
                );
            }
        }
        combined
            .as_object_mut()
            .ok_or("hooks config must be an object")?
            .insert(REGISTRY.into(), registry);
    }
    hooks(&combined)?;
    Ok(combined)
}

pub fn registry(document: &Value) -> Option<&Value> {
    document.get(REGISTRY)
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Hook {
    pub id: String,
    pub events: Vec<Event>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Event {
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

pub fn hooks(document: &Value) -> Result<Vec<Hook>, String> {
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
            ".build/bridgeforge/bin/project_{}{}",
            self.id,
            std::env::consts::EXE_SUFFIX
        )
    }
    pub fn receipt(&self) -> String {
        format!(
            ".build/bridgeforge/bin/build-receipt-project-{}.json",
            self.id
        )
    }
}

pub fn read(root: &Path, relative: &str) -> Result<Option<Vec<u8>>, String> {
    let target = root.join(relative);
    for path in target.ancestors() {
        match fs::symlink_metadata(path) {
            Ok(metadata) => {
                if crate::persistence::is_link_or_reparse(path).map_err(|e| e.to_string())? {
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

pub fn render(document: &Value) -> Result<Value, String> {
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
    result
        .as_object_mut()
        .ok_or("hooks config must be an object")?
        .remove(REGISTRY);
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
                "command":format!(".build/bridgeforge/bin/project_{}{args}",hook.id),
                "commandWindows":format!(".build/bridgeforge/bin/project_{}.exe{args}",hook.id),
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

pub fn identity(hook: &Hook, source: &[u8], contract: &Value) -> Result<String, String> {
    let core = contract["generated_assets"]
        .as_array()
        .and_then(|items| items.first())
        .ok_or("project Rust hooks require managed generated workspace")?;
    crate::manifest::canonical_sha(&json!({"schema_version":1,"hook":hook,
        "source_sha256":sha(source),"workspace_sha256":core["source_tree_sha256"],
        "lockfile_sha256":core["lockfile_sha256"],"wrapper_version":1,
        "build":crate::manifest::generated_build_recipe(&format!("project_{}",hook.id))}))
}

pub fn current(root: &Path, hook: &Hook, identity: &str) -> Result<bool, String> {
    let Some(binary) = read(root, &hook.binary())? else {
        return Ok(false);
    };
    let Some(receipt) = read(root, &hook.receipt())? else {
        return Ok(false);
    };
    let Ok(document) = crate::json::parse_unique_json(&receipt, "project hook receipt") else {
        return Ok(false);
    };
    Ok(document
        == json!({"schema_version":1,"id":hook.id,"input_sha256":identity,
        "platform":std::env::consts::OS,"binary_sha256":sha(&binary)}))
}

pub fn owned(root: &Path, hook: &Hook) -> Result<bool, String> {
    let Some(payload) = read(root, &hook.receipt())? else {
        return Ok(false);
    };
    let Ok(receipt) = crate::json::parse_unique_json(&payload, "project hook receipt") else {
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

/// Exact legacy outputs from registered generated handlers. Independent,
/// unregistered programs remain project-owned and are never guessed from names.
pub fn legacy_artifacts(root: &Path) -> Result<BTreeMap<PathBuf, Vec<u8>>, String> {
    let Some(config) = read(root, ".codex/hooks.json")? else {
        return Ok(BTreeMap::new());
    };
    let document = crate::json::parse_unique_json(&config, "legacy project hooks")?;
    let combined = with_registry(&document, read(root, REGISTRY_PATH)?.as_deref())?;
    let mut owned = BTreeMap::new();
    for hook in hooks(&combined)? {
        let binary = format!(
            ".codex/bin/project_{}{}",
            hook.id,
            std::env::consts::EXE_SUFFIX
        );
        let receipt = format!(".codex/bin/build-receipt-project-{}.json", hook.id);
        let payload = read(root, &binary)?;
        let record = read(root, &receipt)?;
        if payload.is_none() && record.is_none() {
            continue;
        }
        let (payload, record) = payload
            .zip(record)
            .ok_or("legacy project hook pair is incomplete; preserve it")?;
        let value = crate::json::parse_unique_json(&record, "legacy project hook receipt")?;
        let identity = value["input_sha256"]
            .as_str()
            .ok_or("legacy project hook identity missing")?;
        if !identity
            .strip_prefix("sha256:")
            .is_some_and(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
            || value
                != json!({"schema_version":1,"id":hook.id,"input_sha256":identity,"platform":std::env::consts::OS,"binary_sha256":sha(&payload)})
        {
            return Err(format!(
                "legacy project hook ownership unknown: {}",
                hook.id
            ));
        }
        owned.insert(root.join(binary), payload);
        owned.insert(root.join(receipt), record);
    }
    if !owned.is_empty() {
        let migrated = render(&combined)?;
        let mut expected = migrated.clone();
        for groups in expected["hooks"]
            .as_object_mut()
            .ok_or("hook groups missing")?
            .values_mut()
        {
            for group in groups.as_array_mut().ok_or("invalid hook group")? {
                for handler in group["hooks"]
                    .as_array_mut()
                    .ok_or("invalid hook handlers")?
                {
                    if handler.get(HANDLER_ID).is_some() {
                        for key in ["command", "commandWindows"] {
                            if let Some(command) = handler[key].as_str() {
                                handler[key] = json!(command.replacen(
                                    ".build/bridgeforge/bin/",
                                    ".codex/bin/",
                                    1
                                ));
                            }
                        }
                    }
                }
            }
        }
        let mut original = document.clone();
        original
            .as_object_mut()
            .ok_or("invalid hook document")?
            .remove(REGISTRY);
        if expected != original && migrated != original {
            return Err("legacy project hook handlers have external changes; preserve them".into());
        }
    }
    Ok(owned)
}

pub fn migrate_legacy_registration(root: &Path, payload: &[u8]) -> Result<Vec<u8>, String> {
    if legacy_artifacts(root)?.is_empty() {
        return Ok(payload.to_vec());
    }
    let document = crate::json::parse_unique_json(payload, "prospective hooks")?;
    let combined = with_registry(&document, read(root, REGISTRY_PATH)?.as_deref())?;
    let mut bytes = serde_json::to_vec_pretty(&render(&combined)?).map_err(|e| e.to_string())?;
    bytes.push(b'\n');
    Ok(bytes)
}

pub fn verify(root: &Path, contract: &Value, runtime: bool) -> Result<Vec<String>, String> {
    let sidecar = read(root, REGISTRY_PATH)?;
    let Some(payload) = read(root, ".codex/hooks.json")? else {
        if sidecar.is_some() {
            return Err("project Rust hook registry requires hooks.json".into());
        }
        return Ok(Vec::new());
    };
    let document = crate::json::parse_unique_json(&payload, "hooks.json")?;
    let combined = with_registry(&document, sidecar.as_deref())?;
    if render(&combined)? != document {
        return Err("project Rust hook registrations drifted".into());
    }
    let mut checked = Vec::new();
    for hook in hooks(&combined)? {
        let source = read(root, &hook.source())?.ok_or("project Rust hook source is missing")?;
        let input = capture_input(
            root,
            &hook,
            source,
            &BTreeMap::new(),
            &[],
            &mut BTreeMap::new(),
        )?
        .identity(&hook, contract)?;
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
pub fn build(
    workspace: &Path,
    root: &Path,
    contract: &Value,
    inputs: &[(Hook, Input)],
    runner: &dyn ProcessRunner,
) -> Result<BTreeMap<PathBuf, Vec<u8>>, String> {
    let mut writes = BTreeMap::new();
    let mut legacy = Vec::new();
    for (hook, input) in inputs {
        if let Some(files) = &input.package {
            writes.extend(package::build(
                workspace, root, hook, input, files, contract, runner,
            )?);
        } else {
            legacy.push((hook.clone(), input.source.clone()));
        }
    }
    writes.extend(build_legacy(workspace, root, contract, &legacy, runner)?);
    Ok(writes)
}

fn build_legacy(
    workspace: &Path,
    root: &Path,
    contract: &Value,
    inputs: &[(Hook, Vec<u8>)],
    runner: &dyn ProcessRunner,
) -> Result<BTreeMap<PathBuf, Vec<u8>>, String> {
    if inputs.is_empty() {
        return Ok(BTreeMap::new());
    }
    let temporary = Temporary(bridgeforge_core::artifact_paths::temporary_build(
        root,
        "project-hooks",
    )?);
    let item = contract["generated_assets"]
        .as_array()
        .and_then(|items| items.first())
        .ok_or("project hooks require managed workspace")?;
    let snapshot = temporary.0.join("workspace");
    let captured = crate::build_inputs::BuildInputs::capture(workspace, snapshot.clone(), item)?;
    // Public distributions use a virtual workspace. Keep legacy single-file
    // hooks in the existing Hook package so the locked package graph is stable.
    let manifest_relative = if snapshot.join("source/hook/Cargo.toml").is_file() {
        "source/hook/Cargo.toml"
    } else {
        "Cargo.toml"
    };
    let mut manifest = fs::read_to_string(snapshot.join(manifest_relative))
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
        bin["path"] = toml_edit::value(if manifest_relative == "Cargo.toml" {
            wrapper
        } else {
            format!("../../{wrapper}")
        });
        bins.push(bin);
    }
    expected.insert(manifest_relative.into(), manifest.to_string().into_bytes());
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
    let mut cache = if runner.artifact_cache_enabled() {
        crate::artifact_cache::Cache::open_temporary(root, &snapshot, runner)
    } else {
        None
    };
    let mut writes = BTreeMap::new();
    let mut pending = Vec::new();
    for (hook, source) in inputs {
        let asset = cache_asset(
            hook,
            &Input {
                source: source.clone(),
                package: None,
            },
            contract,
        )?;
        if let Some((payload, receipt)) = cache.as_mut().and_then(|c| c.load(&asset, runner)) {
            writes.insert(root.join(hook.binary()), payload);
            writes.insert(root.join(hook.receipt()), receipt);
        } else {
            pending.push((hook.clone(), source.clone()));
        }
    }
    check_snapshot()?;
    if pending.is_empty() {
        return Ok(writes);
    }
    let inputs = pending.as_slice();
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
        let actual = crate::json::parse_unique_json(&tested.stdout, "project hook self-test")?;
        if tested.timed_out
            || tested.code != 0
            || actual != json!({"id":hook.id,"input_sha256":fingerprint,"status":"ok"})
            || fs::read(&binary).map_err(|e| e.to_string())? != payload
        {
            return Err(format!("project Rust hook self-test failed: {}", hook.id));
        }
        let receipt = json!({"schema_version":1,"id":hook.id,"input_sha256":fingerprint,
            "platform":std::env::consts::OS,"binary_sha256":sha(&payload)});
        let encoded = serde_json::to_vec_pretty(&receipt).map_err(|e| e.to_string())?;
        check_snapshot()?;
        let asset = cache_asset(
            hook,
            &Input {
                source: source.clone(),
                package: None,
            },
            contract,
        )?;
        if let Some(c) = cache.as_mut() {
            let _ = c.store(&asset, &payload, &encoded, runner);
        }
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
    verify_dependencies_with_generated(depfile, snapshot, expected, &BTreeMap::new())
}

fn verify_dependencies_with_generated(
    depfile: &Path,
    snapshot: &Path,
    expected: &BTreeMap<String, Vec<u8>>,
    generated: &BTreeMap<PathBuf, Vec<u8>>,
) -> Result<(), String> {
    let allowed = expected
        .keys()
        .map(|relative| snapshot.join(relative))
        .chain(generated.keys().cloned())
        .map(|path| fs::canonicalize(path).map_err(|e| e.to_string()))
        .collect::<Result<BTreeSet<_>, _>>()?;
    for absolute in dependency_paths(depfile, snapshot)? {
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

pub fn dependency_paths(depfile: &Path, snapshot: &Path) -> Result<Vec<PathBuf>, String> {
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
    Ok(paths
        .into_iter()
        .map(|path| {
            let path = PathBuf::from(path);
            if path.is_absolute() {
                path
            } else {
                snapshot.join(path)
            }
        })
        .collect())
}

pub use bridgeforge_core::build_receipt::verify_windows_gui;

#[cfg(all(test, bridgeforge_factory_tests))]
#[path = "../../../scripts/tests/unit/core_project_hooks.rs"]
mod tests;
