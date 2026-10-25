//! Explicit Agent claims used only by isolated fixtures. No production defaults.
use crate::release::schema4::{Request, development, migration_apply, migration_plan, preview};
use bridgeforge_core::ProcessRunner;
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
};
const CONFIG: &str = ".codex/bridgeforge-version.json";

pub(crate) fn migrate(root: &Path, runner: &dyn ProcessRunner) -> Result<(), String> {
    let p = migration_plan(root, runner, None);
    if p.code != 0 {
        return Err(p.stderr);
    }
    let fp = p
        .receipt
        .as_ref()
        .and_then(|r| r["fingerprint"].as_str())
        .ok_or("migration fingerprint missing")?;
    let result = migration_apply(root, runner, None, fp, true);
    if result.code != 0 {
        return Err(result.stderr);
    }
    Ok(())
}

pub(crate) fn analysis(
    root: &Path,
    runner: &dyn ProcessRunner,
    product: &str,
    overrides: &[(&str, &str)],
) -> Result<Request, String> {
    close_packages(root, runner, overrides)?;
    let p = preview(root, "chore: fixture release", &Request::default(), runner);
    if p.code != 0 {
        return Err(p.stderr);
    }
    let value = p.receipt.ok_or("analysis preview missing")?;
    if value["status"] == "nothing-to-release" {
        return Ok(Request::default());
    }
    let packages = serde_json::Map::<String, Value>::new();
    let analysis = json!({"schema":1,"fingerprint":value["fingerprint"],"product":{"level":product,"reason":"fixture explicitly declares the program impact","compatibility":"compatible"},"packages":packages});
    fs::create_dir_all(root.join(".runtime")).map_err(|e| e.to_string())?;
    fs::write(
        root.join(".runtime/schema4-agent.json"),
        serde_json::to_vec(&analysis).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    Ok(Request {
        analysis_file: Some(PathBuf::from(".runtime/schema4-agent.json")),
    })
}

/// Explicit isolated-fixture development step, before the release under test.
/// Native manifests and locks are finalized here, never by release.
pub(crate) fn development_analysis(
    root: &Path,
    runner: &dyn ProcessRunner,
    overrides: &[(&str, &str)],
    accept_breaking: bool,
) -> Result<Request, String> {
    let first = development::preview(root, &Request::default(), runner);
    if first.code != 0 {
        return Err(first.stderr);
    }
    if first
        .receipt
        .as_ref()
        .is_some_and(|v| v["status"] == "nothing-to-close")
    {
        return Ok(Request::default());
    }
    let mut cfg: Value =
        serde_json::from_slice(&fs::read(root.join(CONFIG)).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    let mut policies = serde_json::Map::new();
    for p in cfg["packages"]
        .as_array()
        .ok_or("fixture packages missing")?
    {
        policies.insert(
            p["id"].as_str().ok_or("fixture id missing")?.into(),
            json!({"checks":[{"id":"fixture-toolchain","program":"rustc","args":["--version"]}]}),
        );
    }
    for id in first.receipt.as_ref().unwrap()["affected_packages"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(Value::as_str)
    {
        policies.entry(id.to_string()).or_insert(
            json!({"checks":[{"id":"fixture-toolchain","program":"rustc","args":["--version"]}]}),
        );
    }
    if cfg["adapters"].get("python").is_some() && cfg.get("python_environment").is_none() {
        let env = root.join(".runtime/fixture-python");
        fs::create_dir_all(env.join("runtime")).map_err(|e| e.to_string())?;
        fs::create_dir_all(env.join("dependencies")).map_err(|e| e.to_string())?;
        fs::write(
            env.join("python.exe"),
            b"isolated fixture interpreter identity",
        )
        .map_err(|e| e.to_string())?;
        fs::write(env.join("runtime/stdlib.py"), b"fixture runtime").map_err(|e| e.to_string())?;
        cfg["python_environment"] = json!({"interpreter":".runtime/fixture-python/python.exe","runtime_roots":[".runtime/fixture-python/runtime"],"dependency_roots":[".runtime/fixture-python/dependencies"]});
    }
    cfg["development"] = json!(policies);
    fs::write(root.join(CONFIG), serde_json::to_vec(&cfg).unwrap()).map_err(|e| e.to_string())?;
    let p = development::preview(root, &Request::default(), runner);
    if p.code != 0 {
        return Err(p.stderr);
    }
    let value = p.receipt.ok_or("development preview missing")?;
    let mut packages = serde_json::Map::new();
    // Include explicit dependency owners supplied by the individual scenario.
    for id in value["affected_packages"]
        .as_array()
        .ok_or("affected packages missing")?
        .iter()
        .filter_map(Value::as_str)
        .chain(overrides.iter().map(|(id, _)| *id))
    {
        let level = overrides
            .iter()
            .find(|(key, _)| *key == id)
            .map(|(_, level)| *level)
            .unwrap_or("patch");
        let breaking = accept_breaking
            && cfg["lifecycle"].as_array().is_some_and(|ts| {
                ts.iter().any(|t| {
                    t["compatibility"] == "breaking"
                        && ["from", "to"].iter().any(|side| {
                            t[side]
                                .as_array()
                                .is_some_and(|ends| ends.iter().any(|p| p["id"] == id))
                        })
                })
            });
        let version = value["packages"][id]["version"]
            .as_str()
            .or_else(|| cfg["baseline"]["packages"][id]["version"].as_str())
            .unwrap_or("1.0.0");
        let level = if breaking {
            if version.starts_with("0.") {
                "minor"
            } else {
                "major"
            }
        } else {
            level
        };
        let mut object = json!({"level":level,"reason":"explicit isolated fixture development","compatibility":if breaking{"breaking"}else{"compatible"}});
        if breaking {
            object["acceptance"] = json!({"fingerprint":value["fingerprint"],"scope":[id],"reason":"fixture user accepts lifecycle migration","source":"explicit fixture user decision"});
        }
        packages.insert(id.into(), object);
    }
    fs::create_dir_all(root.join(".runtime")).map_err(|e| e.to_string())?;
    fs::write(
        root.join(".runtime/develop-fixture.json"),
        serde_json::to_vec(
            &json!({"schema":1,"fingerprint":value["fingerprint"],"packages":packages}),
        )
        .unwrap(),
    )
    .map_err(|e| e.to_string())?;
    Ok(Request {
        analysis_file: Some(".runtime/develop-fixture.json".into()),
    })
}
pub(crate) fn close_packages(
    root: &Path,
    runner: &dyn ProcessRunner,
    overrides: &[(&str, &str)],
) -> Result<(), String> {
    close_packages_accepted(root, runner, overrides, false)
}
pub(crate) fn close_packages_accepted(
    root: &Path,
    runner: &dyn ProcessRunner,
    overrides: &[(&str, &str)],
    accept_breaking: bool,
) -> Result<(), String> {
    let request = development_analysis(root, runner, overrides, accept_breaking)?;
    if request.analysis_file.is_none() {
        return Ok(());
    }
    let path = root.join(request.analysis_file.as_ref().unwrap());
    let mut analysis: Value = serde_json::from_slice(&fs::read(&path).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let cfg: Value =
        serde_json::from_slice(&fs::read(root.join(CONFIG)).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    let configured = cfg["packages"]
        .as_array()
        .ok_or("fixture packages missing")?;
    for _ in 0..=configured.len() {
        let p = development::preview(root, &request, runner);
        if p.code == 0 {
            let result = development::apply(
                root,
                &request,
                p.receipt
                    .as_ref()
                    .and_then(|v| v["fingerprint"].as_str())
                    .ok_or("fixture fingerprint missing")?,
                runner,
            );
            return if result.code == 0 {
                Ok(())
            } else {
                Err(result.stderr)
            };
        }
        let error = p.stderr.trim_start_matches("[package-development] ");
        let Some((id, reason)) = error.split_once(": ") else {
            return Err(p.stderr);
        };
        if !reason.starts_with("development analysis missing")
            || !configured.iter().any(|p| p["id"] == id)
            || analysis["packages"].get(id).is_some()
        {
            return Err(p.stderr);
        }
        analysis["packages"][id] = json!({"level":"patch","reason":"fixture confirms effective dependency requirement update","compatibility":"compatible"});
        fs::write(&path, serde_json::to_vec(&analysis).unwrap()).map_err(|e| e.to_string())?;
    }
    Err("fixture dependency closure did not converge".into())
}
