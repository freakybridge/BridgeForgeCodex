//! Project-scoped, durable first-use choice. No stdin prompts and no Git writes in inspection.
use super::*;
use crate::CommandOutcome;

const CONFIG: &str = ".codex/bridgeforge-version.json";

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Confirmation {
    schema: u32,
    confirmed: bool,
    selection: String,
}

pub(super) fn confirmation(value: &Value) -> Result<bool, String> {
    let Some(record) = value.get("git_sync_setup") else {
        return Ok(false);
    };
    let record: Confirmation = serde_json::from_value(record.clone())
        .map_err(|e| format!("invalid git_sync_setup: {e}"))?;
    if record.schema != 1
        || !record.confirmed
        || !matches!(record.selection.as_str(), "current" | "rust_independent" | "independent")
    {
        return Err(
            "git_sync_setup must contain schema=1, confirmed=true and a supported selection".into(),
        );
    }
    Ok(true)
}

fn read_config(root: &Path) -> Result<(Option<Vec<u8>>, Value), String> {
    let bytes = config::read_current(root, CONFIG)?;
    let value = if let Some(bytes) = &bytes {
        let value = crate::json::parse_unique_json(bytes, "version sync config")?;
        config::parse(bytes)?;
        explicit_release_policy(root)?;
        if value["schema_version"].as_u64() == Some(1) && !config::auto_manifests(&value)? {
            let manifests = value["manifests"]
                .as_array()
                .filter(|a| !a.is_empty())
                .ok_or("schema 1 manifests must be a nonempty array")?;
            let mut seen = std::collections::BTreeSet::new();
            for entry in manifests {
                let path = entry
                    .as_str()
                    .ok_or("schema 1 manifest path must be a string")?;
                safe_relative(root, path)?;
                if !matches!(
                    Path::new(path).file_name().and_then(|s| s.to_str()),
                    Some("Cargo.toml" | "package.json")
                ) || !seen.insert(path)
                {
                    return Err("schema 1 manifests contain unsupported or duplicate paths".into());
                }
            }
        }
        value
    } else {
        json!({"schema_version":1,"release_policy":"explicit_release","manifest_policy":"auto"})
    };
    Ok((bytes, value))
}

/// Fast path for a normal sync: durable consent is independent of product version.
pub fn is_confirmed(root: &Path) -> Result<bool, String> {
    let (_, value) = read_config(root)?;
    confirmation(&value)
}

struct Inspection {
    value: Value,
    inputs: BTreeMap<PathBuf, Option<Vec<u8>>>,
    inventory: Vec<String>,
    head: String,
    roots: Vec<String>,
    adaptation: Vec<String>,
    rust_errors: Vec<String>,
    current_strategy: &'static str,
}

fn excluded(path: &str) -> bool {
    path.starts_with(".codex/")
        || path.starts_with(".runtime/")
        || path.starts_with(".git/")
        || path
            .split('/')
            .any(|p| matches!(p, "target" | "node_modules" | "vendor" | ".venv" | "venv"))
}

fn inspect(root: &Path, runner: &dyn ProcessRunner) -> Result<Inspection, String> {
    let (bytes, value) = read_config(root)?;
    let current_strategy = version_strategy(root)?;
    let inventory = independent::inventory(root, runner)?;
    let head = history::git(root, runner, &["rev-parse", "HEAD"])?;
    let mut inputs = BTreeMap::from([(root.join(CONFIG), bytes)]);
    for path in ["VERSION", "CHANGELOG.md"] {
        inputs.insert(root.join(path), config::read_current(root, path)?);
    }
    let mut manifests = BTreeMap::new();
    let mut adaptation = Vec::new();
    let mut rust_errors = Vec::new();
    for path in &inventory {
        if path.starts_with(".codex/hooks/project_") || path.starts_with(".codex/bin/project_") {
            adaptation.push("project_owned_hooks_require_adaptation_review".to_string());
        }
        if excluded(path) {
            continue;
        }
        if path.rsplit('/').next() == Some("Cargo.lock") {
            inputs.insert(root.join(path), config::read_current(root, path)?);
        } else if path.rsplit('/').next() == Some("Cargo.toml") {
            let bytes = config::read_current(root, path)?;
            inputs.insert(root.join(path), bytes.clone());
            let doc = bytes
                .as_deref()
                .ok_or_else(|| format!("{path}: business Cargo manifest was deleted"))
                .and_then(|b| std::str::from_utf8(b).map_err(|e| e.to_string()))
                .and_then(|s| {
                    s.parse::<toml_edit::DocumentMut>()
                        .map_err(|e| format!("{path}: {e}"))
                });
            match doc {
                Ok(doc) => {
                    manifests.insert(path.clone(), doc);
                }
                Err(error) => rust_errors.push(error),
            }
        } else if path.ends_with(".rs") {
            let bytes = config::read_current(root, path)?;
            if bytes
                .as_ref()
                .is_some_and(|b| String::from_utf8_lossy(b).contains("CARGO_PKG_VERSION"))
            {
                adaptation.push("package_version_usage_requires_product_version_review".into());
            }
            inputs.insert(root.join(path), bytes);
        }
    }
    let mut roots = Vec::new();
    for (path, doc) in &manifests {
        if doc.get("workspace").is_some() {
            roots.push(path.clone());
            continue;
        }
        if doc.get("package").is_none() {
            continue;
        }
        if doc
            .get("package")
            .and_then(|p| p.get("workspace"))
            .is_some()
        {
            continue;
        }
        let mut dir = Path::new(path).parent().unwrap_or(Path::new(""));
        let mut owned = false;
        while let Some(parent) = dir.parent() {
            let candidate = parent
                .join("Cargo.toml")
                .to_string_lossy()
                .replace('\\', "/");
            if manifests
                .get(&candidate)
                .is_some_and(|d| d.get("workspace").is_some())
            {
                owned = true;
                break;
            }
            dir = parent;
        }
        if !owned {
            roots.push(path.clone());
        }
    }
    roots.sort();
    roots.dedup();
    adaptation.sort();
    adaptation.dedup();
    Ok(Inspection {
        value,
        inputs,
        inventory,
        head,
        roots,
        adaptation,
        rust_errors,
        current_strategy,
    })
}

fn fingerprint(
    root: &Path,
    plan: &Inspection,
    strategy: &str,
    output: &Value,
) -> Result<String, String> {
    let normalized_inputs = plan
        .inputs
        .iter()
        .map(|(p, b)| {
            (
                p.strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/"),
                b.as_ref().map(|b| digest(b)),
            )
        })
        .collect::<BTreeMap<_, _>>();
    Ok(digest(
        &serde_json::to_vec(&json!({"inputs":normalized_inputs,"inventory":plan.inventory,"head":plan.head,"strategy":strategy,"configuration":output}))
            .map_err(|e| e.to_string())?,
    ))
}

fn report(plan: &Inspection) -> Result<Value, String> {
    let mut options = vec![json!({"id":"current","label":"保留当前发布规则","available":true})];
    if !plan.roots.is_empty() {
        options.push(
            json!({"id":"rust_independent","label":"Rust 独立包升版","available":plan.rust_errors.is_empty(),
            "workspaces":plan.roots,"adaptation_review":plan.adaptation,"discovery_errors":plan.rust_errors}),
        );
    }
    let python=plan.inventory.iter().filter(|p|!excluded(p)&&(p.ends_with("/pyproject.toml")||p.as_str()=="pyproject.toml"||p.ends_with("/setup.cfg")||p.as_str()=="setup.cfg")).cloned().collect::<Vec<_>>();
    if !python.is_empty()||!plan.roots.is_empty()||plan.current_strategy=="independent"{
        options.push(json!({"id":"independent","label":"Rust / Python 通用独立包发布","available":true,"configuration_proposal_required":true,"python_candidates":python,"note":"requires explicit package identities, input ownership and lock scope"}));
    }
    Ok(
        json!({"schema":1,"status":if confirmation(&plan.value)? {"configured"} else {"configuration-required"},
        "configuration":CONFIG,"current_strategy":plan.current_strategy,"current_configuration":plan.value,
        "options":options,"rust_discovery_errors":plan.rust_errors}),
    )
}

pub fn status(root: &Path, runner: &dyn ProcessRunner) -> CommandOutcome {
    status_with(root, runner, false)
}

pub fn status_with(root: &Path, runner: &dyn ProcessRunner, reconfigure: bool) -> CommandOutcome {
    if !reconfigure {
        match read_config(root).and_then(|(_, value)| Ok((confirmation(&value)?, value))) {
            Ok((true, value)) => {
                return CommandOutcome::with_receipt(
                    json!({"schema":1,"status":"configured","configuration":CONFIG,
            "current_strategy":match value["schema_version"].as_u64(){Some(3)=>"independent",Some(2)=>"rust_independent",_=>"synchronized"}}),
                );
            }
            Ok((false, _)) => {}
            Err(e) => return CommandOutcome::blocked(format!("[git-sync-setup] {e}\n")),
        }
    }
    match inspect(root, runner).and_then(|p| report(&p)) {
        Ok(value) => CommandOutcome::with_receipt(value),
        Err(error) => CommandOutcome::blocked(format!("[git-sync-setup] {error}\n")),
    }
}

fn selected(
    root: &Path,
    plan: &mut Inspection,
    strategy: &str,
    runner: &dyn ProcessRunner,
) -> Result<Value, String> {
    let mut value = plan.value.clone();
    match strategy {
        "current" => {}
        "independent" if plan.current_strategy == "independent" => {}
        "rust_independent" if plan.current_strategy == "rust_independent" => {}
        "rust_independent" => {
            if !plan.rust_errors.is_empty() {
                return Err(format!(
                    "project adaptation required: {}",
                    plan.rust_errors.join(", ")
                ));
            }
            if plan.roots.is_empty() {
                return Err("Rust independent release is unavailable: no business Rust workspace; managed hooks do not count".into());
            }
            if !plan.adaptation.is_empty() {
                return Err(format!(
                    "project adaptation required before enabling Rust independent release: {}",
                    plan.adaptation.join(", ")
                ));
            }
            let product_inputs = vec![
                "README.md".into(),
                "doc/".into(),
                "LICENSE".into(),
                ".gitignore".into(),
                ".gitattributes".into(),
            ];
            let cfg = rust::RustConfig {
                workspaces: plan.roots.clone(),
                dependency_bump: "patch".into(),
                shared_inputs: Vec::new(),
                product_inputs: product_inputs.clone(),
            };
            let workspace = rust::load(&cfg, &plan.inventory, &mut |p| {
                config::read_current(root, p)
            })?;
            for (path, bytes) in workspace.inputs() {
                plan.inputs.insert(root.join(path), bytes.clone());
            }
            for path in &plan.inventory {
                if excluded(path)
                    || path.starts_with(".githooks/")
                    || matches!(path.as_str(), "VERSION" | "CHANGELOG.md" | "AGENTS.md")
                {
                    continue;
                }
                if workspace.affected(path).is_err() {
                    return Err(format!(
                        "project adaptation required: declare shared/product ownership for {path}"
                    ));
                }
            }
            // Existing packages and history must already be consistent. Do not
            // fix business Hooks, version display or manually edited versions.
            let (head, boundary) = history::validate_candidate_baseline(root, runner, &cfg)
                .map_err(|e| format!("project adaptation required: {e}"))?;
            let reader = super::history_cache::Reader::new(root, runner);
            let release = independent::build(
                root,
                "chore: validate release configuration",
                Vec::new(),
                &head,
                &boundary,
                &cfg,
                &reader,
            )
            .map_err(|e| format!("project adaptation required: {e}"))?;
            reader.verify_unchanged().map_err(|e| format!("project adaptation required: {e}"))?;
            if let Some(release) = release {
                for (path, bytes) in release.inputs {
                    plan.inputs.insert(path, bytes);
                }
            }
            value = json!({"schema_version":2,"release_policy":"explicit_release","strategy":"rust_independent",
                "rust":{"workspaces":plan.roots,"dependency_bump":"patch","product_inputs":product_inputs}});
        }
        "independent" => return Err("schema 3 setup requires --setup-config-file with an explicit package and ownership proposal".into()),
        _ => return Err("setup strategy must be current, rust_independent or independent".into()),
    }
    if let Some(cfg) = config::parse(&serde_json::to_vec(&value).map_err(|e| e.to_string())?)? {
        if cfg.workspaces.iter().any(|p| excluded(p)) {
            return Err("configured Rust workspaces must belong to business code, not managed hooks or caches".into());
        }
        let workspace = rust::load(&cfg, &plan.inventory, &mut |p| {
            config::read_current(root, p)
        })?;
        for (path, bytes) in workspace.inputs() {
            plan.inputs.insert(root.join(path), bytes.clone());
        }
    }
    if let Some(cfg)=polyglot::parse(&serde_json::to_vec(&value).map_err(|e|e.to_string())?)? {
        let reader=history_cache::Reader::new(root,runner);
        let release=history::validate_polyglot_candidate(root,&cfg,&reader)?;
        for(path,bytes)in release{plan.inputs.insert(path,bytes);}
    }
    value["git_sync_setup"] = json!({"schema":1,"confirmed":true,"selection":strategy});
    config::parse(&serde_json::to_vec(&value).map_err(|e| e.to_string())?)?;
    Ok(value)
}

fn proposal(root:&Path,inspection:&mut Inspection,path:&str,runner:&dyn ProcessRunner)->Result<Value,String>{
    let candidate=safe_relative(root,path)?;
    let bytes=config::read_current(root,path)?.ok_or("setup configuration proposal is missing")?;
    let mut value=crate::json::parse_unique_json(&bytes,"setup configuration proposal")?;
    value["git_sync_setup"]=json!({"schema":1,"confirmed":true,"selection":"independent"});
    let cfg=polyglot::parse(&serde_json::to_vec(&value).map_err(|e|e.to_string())?)?.ok_or("configuration proposal must use schema 3")?;
    if cfg.packages.iter().any(|p|excluded(&p.manifest)) {return Err("release package manifests must belong to business code".into());}
    inspection.inputs.insert(candidate,Some(bytes));
    let reader=history_cache::Reader::new(root,runner);
    for(path,bytes)in history::validate_polyglot_candidate(root,&cfg,&reader)?{inspection.inputs.insert(path,bytes);}
    reader.verify_unchanged()?;
    Ok(value)
}

/// Explicit schema 3 configuration is planned against actual Git and adapter
/// inputs. No business metadata, version, or lock is repaired by setup.
pub fn plan_configuration(root:&Path,runner:&dyn ProcessRunner,path:&str)->CommandOutcome{
    let result=(||{
        let mut inspection=inspect(root,runner)?;
        let value=proposal(root,&mut inspection,path,runner)?;
        let key=fingerprint(root,&inspection,"independent",&value)?;
        verify_release_inputs(&inspection.inputs)?;
        verify_release_inventory(root,runner,&inspection.inventory)?;
        Ok::<_,String>(json!({"schema":1,"status":"configuration-planned","fingerprint":key,"strategy":"independent","configuration":value,"writes":[CONFIG]}))
    })();
    match result{Ok(v)=>CommandOutcome::with_receipt(v),Err(e)=>CommandOutcome::blocked(format!("[git-sync-setup] {e}\n"))}
}

pub fn apply_configuration(root:&Path,runner:&dyn ProcessRunner,path:&str,expected:&str,confirmed:bool)->CommandOutcome{
    let result=(||{
        if !confirmed||expected.is_empty(){return Err("explicit choice confirmation and setup fingerprint are required".into());}
        let _lock=crate::project_sync::ProjectLock::acquire(root)?;
        let mut inspection=inspect(root,runner)?;
        let value=proposal(root,&mut inspection,path,runner)?;
        if fingerprint(root,&inspection,"independent",&value)?!=expected{return Err("setup proposal or project inputs changed; inspect again".into());}
        verify_release_inputs(&inspection.inputs)?;verify_release_inventory(root,runner,&inspection.inventory)?;
        if history::git(root,runner,&["rev-parse","HEAD"])?!=inspection.head{return Err("HEAD changed during configuration planning".into());}
        crate::persistence::atomic_write_json(&root.join(CONFIG),&value).map_err(|e|e.to_string())?;
        Ok::<_,String>(json!({"schema":1,"status":"configured","configuration":CONFIG,"strategy":"independent","sync_performed":false}))
    })();
    match result{Ok(v)=>CommandOutcome::with_receipt(v),Err(e)=>CommandOutcome::blocked(format!("[git-sync-setup] {e}\n"))}
}

pub fn plan(root: &Path, runner: &dyn ProcessRunner, strategy: &str) -> CommandOutcome {
    let result = (|| {
        let mut inspection = inspect(root, runner)?;
        let value = selected(root, &mut inspection, strategy, runner)?;
        let key = fingerprint(root, &inspection, strategy, &value)?;
        verify_release_inputs(&inspection.inputs)?;
        verify_release_inventory(root, runner, &inspection.inventory)?;
        Ok::<_, String>(
            json!({"schema":1,"status":"configuration-planned","fingerprint":key,
            "strategy":strategy,"configuration":value,"writes":[CONFIG]}),
        )
    })();
    match result {
        Ok(value) => CommandOutcome::with_receipt(value),
        Err(e) => CommandOutcome::blocked(format!("[git-sync-setup] {e}\n")),
    }
}

pub fn apply(
    root: &Path,
    runner: &dyn ProcessRunner,
    strategy: &str,
    expected: &str,
    confirmed: bool,
) -> CommandOutcome {
    let result = (|| {
        if !confirmed || expected.is_empty() {
            return Err(
                "explicit choice confirmation and setup fingerprint are required".to_string(),
            );
        }
        let _lock = crate::project_sync::ProjectLock::acquire(root)?;
        let mut inspection = inspect(root, runner)?;
        let value = selected(root, &mut inspection, strategy, runner)?;
        if fingerprint(root, &inspection, strategy, &value)? != expected {
            return Err(
                "project configuration or discovery inputs changed; inspect setup again".into(),
            );
        }
        verify_release_inputs(&inspection.inputs)?;
        verify_release_inventory(root, runner, &inspection.inventory)?;
        if history::git(root, runner, &["rev-parse", "HEAD"])? != inspection.head {
            return Err("HEAD changed during configuration planning".into());
        }
        if value != inspection.value {
            crate::persistence::atomic_write_json(&root.join(CONFIG), &value)
                .map_err(|e| e.to_string())?;
        }
        Ok(
            json!({"schema":1,"status":"configured","configuration":CONFIG,"strategy":strategy,"sync_performed":false}),
        )
    })();
    match result {
        Ok(value) => CommandOutcome::with_receipt(value),
        Err(e) => CommandOutcome::blocked(format!("[git-sync-setup] {e}\n")),
    }
}

#[cfg(all(test, bridgeforge_factory_tests))]
#[path = "../../../../../../scripts/tests/unit/core_release_setup.rs"]
mod tests;
