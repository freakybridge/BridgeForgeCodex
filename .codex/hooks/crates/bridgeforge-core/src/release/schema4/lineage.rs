use super::*;

/// History is replayed only to audit identities and native version continuity.
/// Severity always comes from the final difference, never accumulated commits.
pub(super) fn validate(
    root: &Path,
    cfg: &Config,
    baseline: &polyglot::Snapshot,
    current: &polyglot::Snapshot,
    head: &str,
    boundary: &str,
    runner: &dyn ProcessRunner,
) -> Result<(), String> {
    let prior = baseline
        .config
        .as_ref()
        .map(|c| c.lifecycle.as_slice())
        .unwrap_or(&[]);
    if cfg.native.lifecycle.len() < prior.len() || cfg.native.lifecycle[..prior.len()] != *prior {
        return Err("published lifecycle history must be preserved verbatim".into());
    }
    let transitions = cfg
        .native
        .lifecycle
        .iter()
        .filter(|t| t.base == boundary)
        .cloned()
        .collect::<Vec<_>>();
    if cfg
        .native
        .lifecycle
        .iter()
        .any(|t| t.base != boundary && !prior.iter().any(|p| p.id == t.id))
    {
        return Err("new lifecycle event must bind the current release baseline".into());
    }
    let mut consumed = BTreeSet::new();
    let mut previous = baseline.packages.clone();
    let mut retired = cfg.baseline.retired_ids.clone();
    for transition in prior {
        for p in &transition.from {
            if !transition.to.iter().any(|n| n.id == p.id) {
                retired.insert(p.id.clone());
            }
        }
    }
    let mut versions = previous
        .iter()
        .map(|(id, p)| (id.clone(), p.version.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut steps = Vec::new();
    let start = if boundary == cfg.baseline.revision {
        steps.extend(cfg.baseline.lineage.iter().map(|s| s.packages.clone()));
        cfg.baseline.migration_head.as_str()
    } else {
        boundary
    };
    let commits = history::git(
        root,
        runner,
        &[
            "rev-list",
            "--reverse",
            "--first-parent",
            &format!("{start}..{head}"),
        ],
    )?;
    for commit in commits.lines().filter(|s| !s.is_empty()) {
        // Every post-migration revision must carry the new contract. The old
        // parser is never re-entered by daily publishing.
        let bytes = revision_payload(root, commit, CONFIG, runner)?
            .ok_or("post-migration revision has no release configuration")?;
        let committed = parse(&bytes)?.ok_or("post-migration revision must use schema 4")?;
        if serde_json::to_value(&committed.baseline).map_err(|e| e.to_string())?
            != serde_json::to_value(&cfg.baseline).map_err(|e| e.to_string())?
        {
            return Err("committed schema 4 migration baseline cannot be changed".into());
        }
        steps.push(polyglot::historical(root, commit, &cfg.native, runner)?.packages);
    }
    steps.push(current.packages.clone());
    for next in steps {
        lifecycle::validate_step(&previous, &next, &transitions, &mut consumed)?;
        for id in previous.keys().filter(|id| !next.contains_key(*id)) {
            retired.insert(id.clone());
        }
        for (id, p) in &next {
            if retired.contains(id) {
                return Err(format!("{id}: retired identity cannot be reused"));
            }
            if let Some(version) = versions.get(id) {
                if version != &p.version {
                    return Err(format!(
                        "{id}: native version changed outside a formal release"
                    ));
                }
            } else {
                versions.insert(id.clone(), p.version.clone());
            }
        }
        previous = next;
    }
    if transitions.iter().any(|t| !consumed.contains(&t.id)) {
        return Err("lifecycle declaration has no matching observed transition".into());
    }
    Ok(())
}
