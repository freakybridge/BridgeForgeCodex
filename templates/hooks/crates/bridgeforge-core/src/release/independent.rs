//! Language-neutral release orchestration for the opt-in Rust policy.
use super::rust::{RustConfig, Workspace};
use super::*;

pub(super) fn status(root: &Path, runner: &dyn ProcessRunner) -> Result<String, String> {
    history::git(
        root,
        runner,
        &[
            "--no-optional-locks",
            "-c",
            "core.quotepath=false",
            "status",
            "--porcelain=v1",
            "-z",
            "--untracked-files=all",
        ],
    )
}

pub(super) fn inventory(root: &Path, runner: &dyn ProcessRunner) -> Result<Vec<String>, String> {
    let mut paths = BTreeSet::new();
    for args in [
        vec!["ls-files", "-z"],
        vec!["ls-files", "--others", "--exclude-standard", "-z"],
    ] {
        paths.extend(history::paths(&history::git(root, runner, &args)?));
    }
    Ok(paths.into_iter().collect())
}

pub(super) fn historical_workspace(
    root: &Path,
    revision: &str,
    cfg: &RustConfig,
    runner: &dyn ProcessRunner,
) -> Result<Workspace, String> {
    let paths = history::paths(&history::git(
        root,
        runner,
        &["ls-tree", "-r", "--name-only", "-z", revision],
    )?);
    super::rust::load(cfg, &paths, &mut |path| {
        revision_payload(root, revision, path, runner)
    })
}

fn current_workspace(
    root: &Path,
    cfg: &RustConfig,
    runner: &dyn ProcessRunner,
) -> Result<Workspace, String> {
    super::rust::load(cfg, &inventory(root, runner)?, &mut |path| {
        config::read_current(root, path)
    })
}

pub(super) fn validate_current(
    root: &Path,
    boundary: &str,
    cfg: &RustConfig,
    runner: &dyn ProcessRunner,
) -> Result<(), String> {
    let current = current_workspace(root, cfg, runner)?;
    let baseline = historical_workspace(root, boundary, cfg, runner)?;
    current.validate_baseline(&baseline)
}

pub(super) fn build(
    root: &Path,
    message: &str,
    changed: Vec<String>,
    head: &str,
    boundary: &str,
    cfg: &RustConfig,
    runner: &dyn ProcessRunner,
) -> Result<Option<FileReleasePlan>, String> {
    let initial_status = status(root, runner)?;
    let initial_inventory = inventory(root, runner)?;
    let workspace = super::rust::load(cfg, &initial_inventory, &mut |p| {
        config::read_current(root, p)
    })?;
    let mut inputs = BTreeMap::new();
    for (path, bytes) in workspace.inputs() {
        inputs.insert(root.join(path), bytes.clone());
    }
    for path in [".codex/bridgeforge-version.json", "VERSION", "CHANGELOG.md"] {
        inputs.insert(root.join(path), config::read_current(root, path)?);
    }
    let mut events = Vec::new();
    let mut all_paths = BTreeSet::new();
    let commits = history::git(
        root,
        runner,
        &[
            "rev-list",
            "--reverse",
            "--topo-order",
            &format!("{boundary}..{head}"),
        ],
    )?;
    let factory = root.join("templates/managed-skeleton.json").is_file();
    for commit in commits.lines().filter(|s| !s.is_empty()) {
        let parents = history::git(root, runner, &["rev-list", "--parents", "-n", "1", commit])?;
        let parts = parents.split_whitespace().collect::<Vec<_>>();
        if parts.len() != 2 {
            return Err(format!(
                "release history requires review of merge/root commit {commit}"
            ));
        }
        let before = parts[1];
        let touched = history::paths(&history::git(
            root,
            runner,
            &[
                "diff",
                "--name-only",
                "--no-renames",
                "-z",
                before,
                commit,
                "--",
            ],
        )?);
        if touched.is_empty() {
            continue;
        }
        let kind = if factory {
            ReleaseKind::Factory
        } else {
            let contract = revision_payload(root, commit, ".codex/managed-skeleton.json", runner)?
                .ok_or_else(|| format!("release history has no ownership contract at {commit}"))?;
            classify_payloads(
                &touched,
                &contract,
                |p| revision_payload(root, before, p, runner),
                |p| revision_payload(root, commit, p, runner),
            )?
        };
        if kind == ReleaseKind::SkeletonOnly {
            continue;
        }
        // Validate identities and versions at every intermediate revision as well.
        workspace.validate_baseline(&historical_workspace(root, commit, cfg, runner)?)?;
        let info = parse_commit_message(&history::git(
            root,
            runner,
            &["show", "-s", "--format=%B", commit],
        )?)
        .map_err(|e| format!("cannot classify unreleased commit {commit}: {e}"))?;
        let mut own = Vec::new();
        for path in &touched {
            if !factory {
                let contract =
                    revision_payload(root, commit, ".codex/managed-skeleton.json", runner)?
                        .ok_or("release history ownership contract missing")?;
                if classify_payloads(
                    std::slice::from_ref(path),
                    &contract,
                    |p| revision_payload(root, before, p, runner),
                    |p| revision_payload(root, commit, p, runner),
                )? == ReleaseKind::SkeletonOnly
                {
                    continue;
                }
            }
            if matches!(
                path.as_str(),
                "VERSION" | "CHANGELOG.md" | ".codex/bridgeforge-version.json"
            ) {
                continue;
            }
            let old = revision_payload(root, before, path, runner)?;
            let new = revision_payload(root, commit, path, runner)?;
            if !workspace.is_mechanical_change(path, old.as_deref(), new.as_deref())? {
                own.push(path.clone());
            }
        }
        all_paths.extend(touched);
        events.push((info, own));
    }
    let mut actual_changed = changed.into_iter().collect::<BTreeSet<_>>();
    actual_changed.extend(history::paths(&history::git(
        root,
        runner,
        &["diff", "--name-only", "--no-renames", "-z", "HEAD", "--"],
    )?));
    actual_changed.extend(history::paths(&history::git(
        root,
        runner,
        &["ls-files", "--others", "--exclude-standard", "-z"],
    )?));
    let changed = actual_changed
        .into_iter()
        .into_iter()
        .filter(|p| p != "VERSION")
        .collect::<Vec<_>>();
    if !changed.is_empty() && classify(root, &changed, runner)? != ReleaseKind::SkeletonOnly {
        let mut own = Vec::new();
        for path in &changed {
            let now = config::read_current(root, path)?;
            inputs.insert(root.join(path), now.clone());
            if !factory
                && classify(root, std::slice::from_ref(path), runner)? == ReleaseKind::SkeletonOnly
            {
                continue;
            }
            if matches!(
                path.as_str(),
                "CHANGELOG.md" | ".codex/bridgeforge-version.json"
            ) {
                continue;
            }
            let old = revision_payload(root, head, path, runner)?;
            if !workspace.is_mechanical_change(path, old.as_deref(), now.as_deref())? {
                own.push(path.clone());
            }
        }
        events.push((parse_commit_message(message)?, own));
        all_paths.extend(changed);
    }
    if events.is_empty() {
        return Ok(None);
    }
    let adapter = workspace.plan(&events)?;
    for (path, bytes) in adapter.inputs {
        inputs.insert(root.join(path), bytes);
    }
    // Also guard the current contents of historical source inputs used for attribution.
    for path in &all_paths {
        inputs
            .entry(root.join(path))
            .or_insert(config::read_current(root, path)?);
    }
    let old_version = std::str::from_utf8(
        inputs[&root.join("VERSION")]
            .as_deref()
            .ok_or("root VERSION missing")?,
    )
    .map_err(|e| e.to_string())?
    .trim()
    .parse::<SemVer>()?;
    let infos = events.iter().map(|(i, _)| i.clone()).collect::<Vec<_>>();
    let mut candidates = Vec::new();
    for info in &infos {
        let component = if info.breaking {
            old_version.major
        } else if info.kind == "feat" {
            old_version.minor
        } else {
            old_version.patch
        };
        component.checked_add(1).ok_or("product version overflow")?;
        candidates.push(bump(old_version, info));
    }
    let new_version = candidates.into_iter().max().unwrap();
    let kind = if factory {
        ReleaseKind::Factory
    } else {
        ReleaseKind::Business
    };
    let paths = all_paths.into_iter().collect::<Vec<_>>();
    let mut log = String::from_utf8(render_changelog(root, new_version, &infos, &kind, &paths)?)
        .map_err(|e| e.to_string())?;
    if !adapter.packages.is_empty() {
        let start = log
            .find(&format!("## [{new_version}]"))
            .ok_or("release heading missing")?;
        let insert = log[start + 4..]
            .find("\n## [")
            .map(|n| start + 4 + n)
            .unwrap_or(log.len());
        let mut section = String::from("\n### Internal packages\n\n");
        for package in &adapter.packages {
            section.push_str(&format!(
                "- `{}`: {} -> {} ({})\n",
                package.manifest, package.version_before, package.version_after, package.reason
            ));
        }
        section.push('\n');
        log.insert_str(insert, &section);
    }
    let mut writes = adapter
        .writes
        .into_iter()
        .map(|(p, b)| (root.join(p), b))
        .collect::<BTreeMap<_, _>>();
    writes.insert(
        root.join("VERSION"),
        format!("{new_version}\n").into_bytes(),
    );
    writes.insert(root.join("CHANGELOG.md"), log.into_bytes());
    verify_release_inputs(&inputs)?;
    if status(root, runner)? != initial_status
        || inventory(root, runner)? != initial_inventory
        || history::git(root, runner, &["rev-parse", "HEAD"])?.trim() != head
    {
        return Err(
            "repository inventory or HEAD changed during release planning; no apply".into(),
        );
    }
    Ok(Some(FileReleasePlan {
        inputs,
        old_version,
        new_version,
        kind,
        writes,
        package_changes: adapter.packages,
        release_inventory: Some(initial_inventory),
        release_status: Some(initial_status),
    }))
}
