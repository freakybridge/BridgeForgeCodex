//! Explicit Agent claims used only by isolated fixtures. No production defaults.
use super::*;

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
    let p = preview(root, "chore: fixture release", &Request::default(), runner);
    if p.code != 0 {
        return Err(p.stderr);
    }
    let value = p.receipt.ok_or("analysis preview missing")?;
    if value["status"] == "nothing-to-release" {
        return Ok(Request::default());
    }
    let mut packages = serde_json::Map::new();
    for id in value["affected_packages"]
        .as_array()
        .ok_or("affected package set missing")?
    {
        packages.insert(id.as_str().ok_or("invalid package identity")?.into(),json!({"level":"patch","reason":"fixture explicitly declares an internal repair","compatibility":"compatible"}));
    }
    for (id, level) in overrides {
        packages.insert((*id).into(),json!({"level":level,"reason":"fixture explicitly declares this object's impact","compatibility":"compatible"}));
    }
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
