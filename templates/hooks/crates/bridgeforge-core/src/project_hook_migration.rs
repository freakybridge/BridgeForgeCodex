//! One-time migration notices. The update hot path reads registration and small
//! state records, never inventories Hook sources or invokes a compiler.
use crate::managed_paths::safe_join;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

pub const POLICY: &str = ".codex/project-hook-policy.json";
pub const STATE: &str = ".codex/project-hook-migrations.json";
pub const NOTICES: &str = ".runtime/bridgeforge-codex/project-hook-notices.json";
const OWNER: &str = "bridgeforge-project-hook-migration-v1";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Policy {
    schema: u32,
    owner: String,
    migration_id: String,
    legacy_versions_through: String,
    runtime_compatible: bool,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct State {
    schema: u32,
    owner: String,
    migration_id: String,
    registration: String,
    state: String,
    evidence: Value,
}

pub(crate) struct Inspection {
    pub notice: Value,
    pub reads: BTreeMap<String, Option<Vec<u8>>>,
    pub writes: BTreeMap<PathBuf, Vec<u8>>,
    pub blocked: bool,
}

pub fn enabled(contract: &Value) -> bool {
    contract["assets"].as_array().is_some_and(|a| {
        a.iter()
            .any(|a| a["id"] == "codex.project-hook-policy" && a["target"] == POLICY)
    })
}

fn read(root: &Path, relative: &str) -> Result<Option<Vec<u8>>, String> {
    let path = safe_join(root, relative, "project Hook migration")?;
    crate::managed_paths::read_metadata(&path, 128 * 1024)
}

fn state(bytes: Option<&[u8]>, id: &str) -> Result<Option<State>, String> {
    let Some(bytes) = bytes else { return Ok(None) };
    let value = crate::json::parse_unique_json(bytes, "project Hook migration state")?;
    let value: State = serde_json::from_value(value).map_err(|e| e.to_string())?;
    if value.schema != 1
        || value.owner != OWNER
        || value.migration_id != id
        || !matches!(
            value.state.as_str(),
            "pending" | "completed" | "not_applicable" | "emitted"
        )
    {
        return Err("unknown project Hook migration state; preserve it for inspection".into());
    }
    if value.state == "completed"
        && (value.evidence["schema"] != 1
            || value.evidence["builds"]
                .as_array()
                .is_none_or(|b| b.is_empty()))
    {
        return Err("completed project Hook migration has no verified build evidence".into());
    }
    Ok(Some(value))
}

fn encode(value: &State) -> Result<Vec<u8>, String> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn registrations(
    root: &Path,
    reads: &mut BTreeMap<String, Option<Vec<u8>>>,
) -> Result<(String, Vec<String>), String> {
    let registry = read(root, crate::project_hooks::REGISTRY_PATH)?;
    let hooks = read(root, ".codex/hooks.json")?;
    reads.insert(crate::project_hooks::REGISTRY_PATH.into(), registry.clone());
    // The entire native configuration is protected by project-sync's asset
    // reads. The migration identity only includes project-owned registrations.
    let document = hooks
        .as_deref()
        .map(|b| crate::json::parse_unique_json(b, "project hooks"))
        .transpose()?
        .unwrap_or(json!({"hooks":{}}));
    let registry = registry
        .as_deref()
        .map(|b| crate::json::parse_unique_json(b, "project Hook registry"))
        .transpose()?
        .or_else(|| document.get("bridgeforgeProjectHooks").cloned())
        .unwrap_or(Value::Null);
    let mut handlers = Vec::new();
    let mut ids = Vec::new();
    if let Some(entries) = registry["hooks"].as_array() {
        for hook in entries {
            if let Some(id) = hook["id"].as_str() {
                ids.push(id.to_string());
            }
        }
    }
    if let Some(events) = document["hooks"].as_object() {
        for (event, groups) in events {
            for group in groups.as_array().into_iter().flatten() {
                for handler in group["hooks"].as_array().into_iter().flatten() {
                    if let Some(id) = handler["bridgeforgeProjectHookId"].as_str() {
                        ids.push(id.split(':').next().unwrap_or(id).to_string());
                        handlers.push(
                            json!({"event":event,"matcher":group["matcher"],"handler":handler}),
                        );
                    }
                }
            }
        }
    }
    ids.sort();
    ids.dedup();
    if ids.iter().any(|id| {
        id.is_empty()
            || id.len() > 64
            || !id
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
    }) {
        return Err("unsafe project Hook migration identity".into());
    }
    Ok((
        crate::manifest::canonical_sha(&json!({"registry":registry,"handlers":handlers}))?,
        ids,
    ))
}

pub(crate) fn inspect(
    root: &Path,
    policy_bytes: &[u8],
    previous: Option<&str>,
) -> Result<Inspection, String> {
    let policy: Policy = serde_json::from_value(crate::json::parse_unique_json(
        policy_bytes,
        "project Hook policy",
    )?)
    .map_err(|e| e.to_string())?;
    if policy.schema != 1
        || policy.owner != "bridgeforge-project-hook-policy-v1"
        || policy.migration_id != "project-hooks-independent-v1"
    {
        return Err("unsupported project Hook migration declaration".into());
    }
    let through = policy
        .legacy_versions_through
        .parse::<crate::release::SemVer>()?;
    let state_bytes = read(root, STATE)?;
    let notice_bytes = read(root, NOTICES)?;
    let saved = state(state_bytes.as_deref(), &policy.migration_id)?;
    let emitted = state(notice_bytes.as_deref(), &policy.migration_id)?;
    let mut result = Inspection {
        notice: Value::Null,
        reads: BTreeMap::from([
            (STATE.into(), state_bytes.clone()),
            (NOTICES.into(), notice_bytes.clone()),
        ]),
        writes: BTreeMap::new(),
        blocked: false,
    };
    let applicable = saved.is_some()
        || previous.is_none()
        || previous
            .map(str::parse::<crate::release::SemVer>)
            .transpose()?
            .is_some_and(|v| v <= through);
    if !applicable {
        return Ok(result);
    }
    let (registration, ids) = registrations(root, &mut result.reads)?;
    let unchanged = saved
        .as_ref()
        .is_some_and(|s| s.registration == registration);
    let status = if unchanged {
        saved.as_ref().unwrap().state.as_str()
    } else if ids.is_empty() {
        "not_applicable"
    } else {
        "pending"
    };
    if unchanged && matches!(status, "completed" | "not_applicable") {
        result.notice =
            json!({"id":policy.migration_id,"state":status,"show":false,"blocking":false});
        return Ok(result);
    }
    // Limited first encounter only: existence of a manifest is informative,
    // never proof that a project is independent or that its business tests ran.
    let mut legacy = Vec::new();
    if !unchanged {
        for id in &ids {
            let relative = format!(".codex/hooks/project_{id}/Cargo.toml");
            if read(root, &relative)?.is_none() {
                legacy.push(id.clone());
            }
        }
    }
    let next = State {
        schema: 1,
        owner: OWNER.into(),
        migration_id: policy.migration_id.clone(),
        registration: registration.clone(),
        state: status.into(),
        evidence: if unchanged {
            saved.as_ref().unwrap().evidence.clone()
        } else {
            Value::Null
        },
    };
    let payload = encode(&next)?;
    if state_bytes.as_deref() != Some(payload.as_slice()) {
        result
            .writes
            .insert(safe_join(root, STATE, "migration state")?, payload);
    }
    let show = status == "pending"
        && !emitted
            .as_ref()
            .is_some_and(|s| s.registration == registration && s.state == "emitted");
    result.blocked = status == "pending" && !policy.runtime_compatible;
    if status == "pending" {
        result.notice = json!({"id":policy.migration_id,"state":"pending","show":show,"blocking":result.blocked,
            "hooks":ids,"legacy_without_manifest":legacy,
            "message":"项目 Hook 的独立维护状态尚未完成一次性验证；本次保留程序与注册，不自动迁移或编译。"});
        if show {
            let emitted = State {
                state: "emitted".into(),
                ..next
            };
            result.writes.insert(
                safe_join(root, NOTICES, "migration notice")?,
                encode(&emitted)?,
            );
        }
    }
    Ok(result)
}

pub fn status(root: &Path) -> Result<Value, String> {
    let policy = read(root, POLICY)?.ok_or("project Hook migration policy is not installed")?;
    let version = read(root, ".codex/.bridgeforge_codex_version")?;
    let version = version
        .as_deref()
        .map(|b| std::str::from_utf8(b).map(str::trim))
        .transpose()
        .map_err(|e| e.to_string())?;
    let result = inspect(root, &policy, version)?;
    Ok(
        json!({"schema":1,"notice":result.notice,"blocking":result.blocked,"writes_performed":false}),
    )
}

fn inventory(root: &Path) -> Result<BTreeMap<String, Vec<u8>>, String> {
    let mut files = BTreeMap::new();
    for item in walkdir::WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            e.depth() == 0
                || !matches!(e.file_name().to_str(), Some("target" | ".git" | ".runtime"))
        })
    {
        let item = item.map_err(|e| e.to_string())?;
        if crate::persistence::is_link_or_reparse(item.path()).map_err(|e| e.to_string())? {
            return Err("independent Hook source contains a link".into());
        }
        if item.file_type().is_file() {
            let relative = item
                .path()
                .strip_prefix(root)
                .map_err(|e| e.to_string())?
                .to_string_lossy()
                .replace('\\', "/");
            files.insert(relative, fs::read(item.path()).map_err(|e| e.to_string())?);
        }
    }
    Ok(files)
}

/// Explicit migration completion, never invoked by update. Projects must first
/// provide their own Cargo package, main and dependencies. Build and installed
/// binary equality are machine-verified; business behavior is an explicit user
/// decision, not inferred by executing arbitrary Hook events.
pub fn verify_completed(
    root: &Path,
    confirmed_behavior: bool,
    runner: &dyn crate::ProcessRunner,
) -> Result<Value, String> {
    if !confirmed_behavior {
        return Err(
            "independent Hook behavior must be explicitly validated before completion".into(),
        );
    }
    let _lock = crate::project_sync::ProjectLock::acquire(root)?;
    let policy_bytes = read(root, POLICY)?.ok_or("project Hook migration policy is missing")?;
    let policy: Policy = serde_json::from_value(crate::json::parse_unique_json(
        &policy_bytes,
        "project Hook policy",
    )?)
    .map_err(|e| e.to_string())?;
    if policy.schema != 1
        || policy.owner != "bridgeforge-project-hook-policy-v1"
        || policy.migration_id != "project-hooks-independent-v1"
    {
        return Err("unknown migration policy".into());
    }
    let mut reads = BTreeMap::new();
    let (registration, ids) = registrations(root, &mut reads)?;
    if ids.is_empty() {
        return Err("no registered project Hooks to verify".into());
    }
    let previous = read(root, STATE)?;
    let _ = state(previous.as_deref(), &policy.migration_id)?;
    let mut builds = Vec::new();
    for id in ids {
        let directory = safe_join(
            root,
            &format!(".codex/hooks/project_{id}"),
            "independent Hook package",
        )?;
        let files = inventory(&directory)?;
        if !files.contains_key("Cargo.toml") || !files.contains_key("Cargo.lock") {
            return Err(format!(
                "project Hook {id} needs its own Cargo.toml and Cargo.lock"
            ));
        }
        let manifest = directory.join("Cargo.toml");
        let mut metadata = crate::ProcessRequest::new("cargo", &directory);
        metadata.args = vec![
            "metadata".into(),
            "--locked".into(),
            "--offline".into(),
            "--format-version".into(),
            "1".into(),
            "--manifest-path".into(),
            manifest.clone().into_os_string(),
        ];
        let output = runner.run(&metadata).map_err(|e| e.to_string())?;
        if output.code != 0 || output.timed_out {
            return Err(format!("independent Hook metadata failed: {id}"));
        }
        let data = crate::json::parse_unique_json(&output.stdout, "independent Hook metadata")?;
        let workspace = PathBuf::from(
            data["workspace_root"]
                .as_str()
                .ok_or("Hook workspace root missing")?,
        )
        .canonicalize()
        .map_err(|e| e.to_string())?;
        if workspace != directory.canonicalize().map_err(|e| e.to_string())? {
            return Err(format!(
                "project Hook {id} still belongs to an external workspace"
            ));
        }
        let binary_name = format!("project_{id}");
        let packages = data["packages"].as_array().ok_or("Hook packages missing")?;
        let mut found = false;
        for package in packages {
            if package["source"].is_null() {
                let manifest = Path::new(
                    package["manifest_path"]
                        .as_str()
                        .ok_or("Hook package manifest is missing")?,
                )
                .canonicalize()
                .map_err(|e| e.to_string())?;
                if !manifest.starts_with(&workspace) {
                    return Err("Hook local package escapes its independent workspace".into());
                }
            }
            for dependency in package["dependencies"].as_array().into_iter().flatten() {
                if matches!(
                    dependency["name"].as_str(),
                    Some("bridgeforge-core" | "bridgeforge-hook" | "bridgeforge-cli")
                ) {
                    return Err(format!("project Hook {id} still depends on the skeleton"));
                }
                if let Some(path) = dependency["path"]
                    .as_str()
                    .filter(|_| package["source"].is_null())
                {
                    if !Path::new(path)
                        .canonicalize()
                        .map_err(|e| e.to_string())?
                        .starts_with(&workspace)
                    {
                        return Err("Hook path dependency escapes its package".into());
                    }
                }
            }
            found |= package["targets"].as_array().is_some_and(|targets| {
                targets.iter().any(|t| {
                    t["name"] == binary_name
                        && t["kind"]
                            .as_array()
                            .is_some_and(|k| k.iter().any(|x| x == "bin"))
                })
            });
        }
        if !found {
            return Err(format!(
                "project Hook {id} needs its own executable target {binary_name}"
            ));
        }
        let installed = safe_join(
            root,
            &format!(".codex/bin/{binary_name}{}", std::env::consts::EXE_SUFFIX),
            "project Hook executable",
        )?;
        if crate::persistence::is_link_or_reparse(&installed).map_err(|e| e.to_string())? {
            return Err("installed project Hook is a linked path".into());
        }
        let installed_bytes = fs::read(&installed).map_err(|_| {
            format!("project Hook {id} must be installed by its project build workflow first")
        })?;
        let mut build = crate::ProcessRequest::new("cargo", &directory);
        build.args = vec![
            "build".into(),
            "--locked".into(),
            "--release".into(),
            "--manifest-path".into(),
            manifest.into_os_string(),
            "--bin".into(),
            binary_name.clone().into(),
        ];
        build.timeout = std::time::Duration::from_secs(900);
        let output = runner.run(&build).map_err(|e| e.to_string())?;
        if output.code != 0 || output.timed_out {
            return Err(format!("independent Hook build failed: {id}"));
        }
        let target = PathBuf::from(
            data["target_directory"]
                .as_str()
                .ok_or("Hook target directory missing")?,
        )
        .canonicalize()
        .map_err(|e| e.to_string())?;
        let built = target
            .join("release")
            .join(format!("{binary_name}{}", std::env::consts::EXE_SUFFIX));
        let bytes = fs::read(&built).map_err(|e| e.to_string())?;
        let generated_root = target.join("release/build");
        let mut generated = BTreeMap::new();
        for dependency in crate::project_hooks::dependency_paths(
            &target.join("release").join(format!("{binary_name}.d")),
            &directory,
        )? {
            if crate::persistence::is_link_or_reparse(&dependency).map_err(|e| e.to_string())? {
                return Err("Hook compiled input is linked".into());
            }
            let canonical = dependency.canonicalize().map_err(|e| e.to_string())?;
            if canonical.starts_with(&workspace) && !canonical.starts_with(&target) {
                continue;
            }
            if !generated_root.exists()
                || !canonical.starts_with(generated_root.canonicalize().map_err(|e| e.to_string())?)
            {
                return Err(
                    "Hook compiled input is outside its independent package/build outputs".into(),
                );
            }
            generated.insert(
                canonical.to_string_lossy().into_owned(),
                crate::manifest::canonical_sha(&json!(
                    fs::read(&canonical).map_err(|e| e.to_string())?
                ))?,
            );
        }
        if bytes != installed_bytes || fs::read(&installed).map_err(|e| e.to_string())? != bytes {
            return Err(format!(
                "project Hook {id} installed binary does not match its independent build"
            ));
        }
        if cfg!(windows) {
            crate::project_hooks::verify_windows_gui(&bytes)?;
        }
        if inventory(&directory)? != files {
            return Err(format!(
                "project Hook {id} inputs changed during verification"
            ));
        }
        let source_hashes: BTreeMap<_, _> = files
            .iter()
            .map(|(name, bytes)| {
                (
                    name.clone(),
                    crate::manifest::canonical_sha(&json!(bytes)).unwrap(),
                )
            })
            .collect();
        builds.push(
            json!({"hook":id,"inputs":crate::manifest::canonical_sha(&json!(source_hashes))?,"generated":generated,
            "binary":crate::manifest::canonical_sha(&json!(bytes))?,"behavior_confirmed":true}),
        );
    }
    if registrations(root, &mut BTreeMap::new())?.0 != registration
        || read(root, STATE)? != previous
        || read(root, POLICY)?.as_deref() != Some(policy_bytes.as_slice())
    {
        return Err("Hook registration or migration state changed during verification".into());
    }
    let completed = State {
        schema: 1,
        owner: OWNER.into(),
        migration_id: policy.migration_id,
        registration,
        state: "completed".into(),
        evidence: json!({"schema":1,"builds":builds}),
    };
    crate::persistence::atomic_write(
        &safe_join(root, STATE, "migration completion")?,
        &encode(&completed)?,
    )
    .map_err(|e| e.to_string())?;
    Ok(json!({"schema":1,"status":"completed","migration":completed.migration_id,"builds":builds}))
}

#[cfg(all(test, bridgeforge_factory_tests))]
#[path = "../../../../../scripts/tests/unit/core_project_hook_migration.rs"]
mod tests;
