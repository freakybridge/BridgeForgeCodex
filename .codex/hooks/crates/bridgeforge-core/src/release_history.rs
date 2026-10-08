use super::*;

pub fn explicit_release_policy(root: &Path) -> Result<bool, String> {
    let Some(payload) = release_input(&root.join(".codex/bridgeforge-version.json"))? else {
        return Ok(false);
    };
    let value = crate::baseline::parse_unique_json(&payload, "version sync config")?;
    if value["schema_version"].as_u64() != Some(1) {
        return Err("version sync config must use schema_version=1".into());
    }
    match value.get("release_policy") {
        None => Ok(false),
        Some(Value::String(policy)) if policy == "per_commit" => Ok(false),
        Some(Value::String(policy)) if policy == "explicit_release" => Ok(true),
        _ => Err("release_policy must be per_commit or explicit_release".into()),
    }
}

fn git(root: &Path, runner: &dyn ProcessRunner, args: &[&str]) -> Result<String, String> {
    let mut request = ProcessRequest::new("git", root);
    request.args = args.iter().map(std::ffi::OsString::from).collect();
    request.timeout = Duration::from_secs(45);
    let output = runner.run(&request).map_err(|e| e.to_string())?;
    if output.timed_out || output.code != 0 {
        return Err(format!(
            "release history query failed (timed_out={}): {}",
            output.timed_out,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    String::from_utf8(output.stdout).map_err(|e| e.to_string())
}

fn paths(raw: &str) -> Vec<String> {
    raw.split('\0')
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

fn version_at(
    root: &Path,
    revision: &str,
    runner: &dyn ProcessRunner,
) -> Result<Option<SemVer>, String> {
    revision_payload(root, revision, "VERSION", runner)?
        .map(|bytes| {
            std::str::from_utf8(&bytes)
                .map_err(|e| e.to_string())?
                .trim()
                .parse()
        })
        .transpose()
}

fn changelog_sections(payload: &[u8], version: SemVer) -> Result<usize, String> {
    let heading = format!("## [{version}]");
    Ok(std::str::from_utf8(payload)
        .map_err(|e| e.to_string())?
        .lines()
        .filter(|line| {
            line.strip_prefix(&heading)
                .is_some_and(|rest| rest.is_empty() || rest.starts_with(" - "))
        })
        .count())
}

fn verify_historical_manifests(
    root: &Path,
    revision: &str,
    version: SemVer,
    runner: &dyn ProcessRunner,
) -> Result<(), String> {
    let config = revision_payload(root, revision, ".codex/bridgeforge-version.json", runner)?;
    let mut manifests = Vec::new();
    if let Some(payload) = config {
        let config =
            crate::baseline::parse_unique_json(&payload, "release baseline version config")?;
        if config["schema_version"].as_u64() != Some(1) {
            return Err("invalid release baseline version config".into());
        }
        let entries = config["manifests"]
            .as_array()
            .ok_or("release baseline manifests are missing")?;
        if entries.is_empty() {
            return Err("release baseline manifests are empty".into());
        }
        for entry in entries {
            manifests.push(
                entry
                    .as_str()
                    .ok_or("invalid release baseline manifest path")?
                    .to_string(),
            );
        }
    } else {
        for path in ["Cargo.toml", "package.json"] {
            if revision_payload(root, revision, path, runner)?.is_some() {
                manifests.push(path.into());
            }
        }
    }
    for path in manifests {
        safe_relative(root, &path)?;
        let payload = revision_payload(root, revision, &path, runner)?
            .ok_or("release baseline native manifest missing")?;
        let found = match Path::new(&path).file_name().and_then(|s| s.to_str()) {
            Some("Cargo.toml") => cargo_version_text(
                std::str::from_utf8(&payload).map_err(|e| e.to_string())?,
                &path,
            )?,
            Some("package.json") => {
                let value: Value = serde_json::from_slice(&payload).map_err(|e| e.to_string())?;
                value["version"]
                    .as_str()
                    .map(str::parse::<SemVer>)
                    .transpose()?
            }
            _ => {
                return Err(format!(
                    "unsupported release baseline native manifest: {path}"
                ));
            }
        };
        if found != Some(version) {
            return Err(format!(
                "release baseline native manifest disagrees with VERSION: {path}"
            ));
        }
    }
    Ok(())
}

fn release_boundary(root: &Path, head: &str, runner: &dyn ProcessRunner) -> Result<String, String> {
    let candidates = git(
        root,
        runner,
        &[
            "log",
            "--first-parent",
            "--format=%H",
            head,
            "--",
            "VERSION",
        ],
    )?;
    for candidate in candidates.lines().filter(|s| !s.is_empty()) {
        let current =
            version_at(root, candidate, runner)?.ok_or("release history deletes VERSION")?;
        let parents = git(
            root,
            runner,
            &["rev-list", "--parents", "-n", "1", candidate],
        )?;
        let parent = parents.split_whitespace().nth(1);
        let previous = parent
            .map(|p| version_at(root, p, runner))
            .transpose()?
            .flatten();
        let Some(previous) = previous else {
            if let Some(parent) = parent {
                if !git(
                    root,
                    runner,
                    &["log", "-1", "--format=%H", parent, "--", "VERSION"],
                )?
                .trim()
                .is_empty()
                {
                    return Err("release history reintroduces VERSION after deletion".into());
                }
            }
            verify_historical_manifests(root, candidate, current, runner)?;
            return Ok(candidate.to_string());
        };
        if current == previous {
            continue;
        }
        if current < previous {
            return Err("release history decreases VERSION".into());
        }
        let current_log =
            revision_payload(root, candidate, "CHANGELOG.md", runner)?.unwrap_or_default();
        let previous_log =
            revision_payload(root, parent.unwrap(), "CHANGELOG.md", runner)?.unwrap_or_default();
        if changelog_sections(&current_log, current)? != 1
            || changelog_sections(&previous_log, current)? != 0
        {
            return Err(format!(
                "VERSION change at {candidate} has no unique newly added CHANGELOG release section"
            ));
        }
        verify_historical_manifests(root, candidate, current, runner)?;
        return Ok(candidate.to_string());
    }
    Err("explicit release requires a committed VERSION baseline".into())
}

pub fn build_explicit_release_plan(
    root: &Path,
    message: &str,
    changed: Vec<String>,
    runner: &dyn ProcessRunner,
) -> Result<Option<FileReleasePlan>, String> {
    // A read-only preview and the write transaction use the same planner.
    explicit_release_policy(root)?;
    if git(root, runner, &["rev-parse", "--is-shallow-repository"])?.trim() != "false" {
        return Err("explicit release requires complete Git history".into());
    }
    let head = git(root, runner, &["rev-parse", "HEAD"])?;
    let head = head.trim();
    let boundary = release_boundary(root, head, runner)?;
    let boundary = boundary.as_str();
    let version = fs::read_to_string(root.join("VERSION")).map_err(|e| e.to_string())?;
    let baseline = revision_payload(root, boundary, "VERSION", runner)?
        .ok_or("release baseline VERSION is missing")?;
    if version.trim()
        != std::str::from_utf8(&baseline)
            .map_err(|e| e.to_string())?
            .trim()
    {
        return Err(
            "VERSION differs from the committed release baseline; review manual version edits"
                .into(),
        );
    }
    version.trim().parse::<SemVer>()?;
    let factory = root.join("templates/managed-skeleton.json").is_file();
    let range = format!("{boundary}..{head}");
    let commits = git(
        root,
        runner,
        &["rev-list", "--reverse", "--topo-order", &range],
    )?;
    let mut infos = Vec::new();
    let mut all_paths = BTreeSet::new();
    for commit in commits.lines().filter(|s| !s.is_empty()) {
        let parents = git(root, runner, &["rev-list", "--parents", "-n", "1", commit])?;
        let parts = parents.split_whitespace().collect::<Vec<_>>();
        if parts.len() != 2 {
            return Err(format!(
                "release history requires review of merge/root commit {commit}"
            ));
        }
        let before = parts[1];
        let mut touched = paths(&git(
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
        if version_at(root, before, runner)? == version_at(root, commit, runner)? {
            touched.retain(|path| path != "VERSION");
        }
        if touched.is_empty() {
            continue;
        }
        let kind = if factory {
            ReleaseKind::Factory
        } else {
            let current = revision_payload(root, commit, ".codex/managed-skeleton.json", runner)?
                .ok_or_else(|| {
                format!("release history has no ownership contract at {commit}")
            })?;
            classify_payloads(
                &touched,
                &current,
                |path| revision_payload(root, before, path, runner),
                |path| revision_payload(root, commit, path, runner),
            )?
        };
        if kind == ReleaseKind::SkeletonOnly {
            continue;
        }
        let text = git(root, runner, &["show", "-s", "--format=%B", commit])?;
        infos.push(
            parse_commit_message(&text)
                .map_err(|e| format!("cannot classify unreleased commit {commit}: {e}"))?,
        );
        all_paths.extend(touched);
    }
    let changed = changed
        .into_iter()
        .filter(|path| path != "VERSION")
        .collect::<Vec<_>>();
    if !changed.is_empty() && classify(root, &changed, runner)? != ReleaseKind::SkeletonOnly {
        infos.push(parse_commit_message(message)?);
        all_paths.extend(changed);
    }
    let plan = build_selected_release_plan(
        root,
        &infos,
        if factory {
            ReleaseKind::Factory
        } else {
            ReleaseKind::Business
        },
        &all_paths.into_iter().collect::<Vec<_>>(),
    )?;
    if git(root, runner, &["rev-parse", "HEAD"])?.trim() != head {
        return Err("HEAD changed during release planning; no apply".into());
    }
    Ok(plan)
}

pub fn preview(root: &Path, message: &str, runner: &dyn ProcessRunner) -> crate::CommandOutcome {
    let result = (|| {
        let mut changed = BTreeSet::new();
        for args in [
            vec!["diff", "--name-only", "--no-renames", "-z"],
            vec!["diff", "--cached", "--name-only", "--no-renames", "-z"],
            vec!["ls-files", "--others", "--exclude-standard", "-z"],
        ] {
            changed.extend(paths(&git(root, runner, &args)?));
        }
        build_explicit_release_plan(root, message, changed.into_iter().collect(), runner)
    })();
    match result {
        Ok(Some(plan)) => crate::CommandOutcome::with_receipt(json!({
            "schema": 1, "status": "release-planned", "version_before": plan.old_version.to_string(),
            "version_after": plan.new_version.to_string(), "kind": plan.kind,
            "changelog": String::from_utf8_lossy(&plan.writes[&root.join("CHANGELOG.md")]),
            "writes": plan.writes.keys().map(|p| p.strip_prefix(root).unwrap_or(p).to_string_lossy()).collect::<Vec<_>>()
        })),
        Ok(None) => crate::CommandOutcome::with_receipt(
            json!({"schema": 1, "status": "nothing-to-release"}),
        ),
        Err(error) => {
            crate::CommandOutcome::blocked(format!("[git-sync] release preview blocked: {error}\n"))
        }
    }
}
