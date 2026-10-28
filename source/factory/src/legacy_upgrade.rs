//! Disposable factory entry point for four explicitly identified repositories.
//! No repository names or legacy exception switches enter the public CLI.
use crate::{CommandOutcome, ProcessRequest, ProcessRunner, project_sync};
use serde_json::json;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

fn project(id: &str) -> Result<(&'static str, &'static str), String> {
    match id {
        "StratusAgent" => Ok(("github.com/freakybridge/StratusAgent", "76d756a6979e6c05fe89bf1a8708d14fd2bc6ea5")),
        "BridgePersonalAssist" => Ok(("github.com/freakybridge/BridgePersonalAssist", "4f5bc646b59af4c57a1a7916cfb08a3d3100f4cc")),
        "CausisRiskSuite" => Ok(("git.causis.com.cn:8095/jinqiao.xue/causis_risk_suite", "0325d573adf11aba04327add79853a56dc832371")),
        "HoldemTrainer" => Ok(("github.com/freakybridge/HoldemTrainer", "1d412eb035512ad0fe472c9d7098a8cc4bbcd575")),
        _ => Err("temporary upgrade only supports StratusAgent, BridgePersonalAssist, CausisRiskSuite, HoldemTrainer".into()),
    }
}

fn origin_identity(url: &str) -> &str {
    url.strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .or_else(|| url.strip_prefix("ssh://git@"))
        .unwrap_or(url)
        .trim_end_matches('/')
        .trim_end_matches(".git")
}

fn git(root: &Path, args: &[&str], runner: &dyn ProcessRunner) -> Result<String, String> {
    let mut request = ProcessRequest::new("git", root);
    request.args = ["--no-replace-objects"]
        .into_iter()
        .chain(args.iter().copied())
        .map(Into::into)
        .collect();
    request.env_remove = [
        "GIT_DIR",
        "GIT_WORK_TREE",
        "GIT_COMMON_DIR",
        "GIT_INDEX_FILE",
    ]
    .into_iter()
    .map(Into::into)
    .collect();
    let result = runner.run(&request).map_err(|e| e.to_string())?;
    if result.code != 0 || result.timed_out {
        return Err(format!(
            "repository identity check failed: {}",
            String::from_utf8_lossy(&result.stderr)
        ));
    }
    Ok(String::from_utf8(result.stdout)
        .map_err(|e| e.to_string())?
        .trim()
        .to_owned())
}

fn identity(root: &Path, id: &str, runner: &dyn ProcessRunner) -> Result<Vec<PathBuf>, String> {
    let (expected, anchor) = project(id)?;
    let top = PathBuf::from(git(root, &["rev-parse", "--show-toplevel"], runner)?)
        .canonicalize()
        .map_err(|e| e.to_string())?;
    if top != root {
        return Err("project-root must be the repository worktree root".into());
    }
    let remote = git(
        root,
        &[
            "config",
            "--local",
            "--no-includes",
            "--get-all",
            "remote.origin.url",
        ],
        runner,
    )?;
    // Reject multiple origins, credentials, URL rewriting and name-only matches.
    let normalized = if let Some(tail) = remote.strip_prefix("git@github.com:") {
        format!("github.com/{}", tail.trim_end_matches(".git"))
    } else {
        origin_identity(&remote).to_owned()
    };
    if normalized != expected {
        return Err("origin does not match the selected authorized project".into());
    }
    git(
        root,
        &["merge-base", "--is-ancestor", anchor, "HEAD"],
        runner,
    )?;
    let mut paths = Vec::new();
    for name in ["config", "config.worktree", "HEAD", "commondir"] {
        let path = git(
            root,
            &["rev-parse", "--path-format=absolute", "--git-path", name],
            runner,
        )?;
        paths.push(PathBuf::from(path));
    }
    // Symbolic HEAD can stay byte-identical when its branch moves. Bind loose
    // and packed refs too; detached worktrees are covered by HEAD itself.
    let head_path = paths[2].clone();
    if let Some(reference) = fs::read_to_string(&head_path)
        .map_err(|e| e.to_string())?
        .trim()
        .strip_prefix("ref: ")
    {
        for name in [reference, "packed-refs"] {
            paths.push(PathBuf::from(git(
                root,
                &["rev-parse", "--path-format=absolute", "--git-path", name],
                runner,
            )?));
        }
    }
    if root.join(".git").is_file() {
        paths.push(root.join(".git"));
    }
    Ok(paths)
}

fn parse(args: &[String]) -> Result<BTreeMap<String, String>, String> {
    let mut options = BTreeMap::new();
    let mut i = 1;
    while i < args.len() {
        let key = &args[i];
        let val = match key.as_str() {
            "--preview" | "--apply" | "--confirmed-risk" => String::new(),
            "--root" | "--project-root" | "--project" | "--plan-fingerprint"
            | "--public-artifacts" => {
                i += 1;
                args.get(i)
                    .filter(|v| !v.starts_with("--"))
                    .cloned()
                    .ok_or_else(|| format!("{key} requires a value"))?
            }
            _ => return Err(format!("unknown temporary-upgrade argument: {key}")),
        };
        if options.insert(key.clone(), val).is_some() {
            return Err(format!("duplicate argument: {key}"));
        }
        i += 1;
    }
    if options.contains_key("--preview") == options.contains_key("--apply") {
        return Err("choose exactly one of --preview or --apply".into());
    }
    for key in ["--root", "--project-root", "--project"] {
        if !options.contains_key(key) {
            return Err(format!("{key} is required"));
        }
    }
    project(&options["--project"])?;
    if options.contains_key("--apply") && !options.contains_key("--plan-fingerprint") {
        return Err(
            "--apply requires --plan-fingerprint from the immediately preceding preview".into(),
        );
    }
    Ok(options)
}

fn execute(args: &[String], runner: &dyn ProcessRunner) -> Result<CommandOutcome, String> {
    let options = parse(args)?;
    let root = PathBuf::from(&options["--project-root"])
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let factory = PathBuf::from(&options["--root"])
        .canonicalize()
        .map_err(|e| e.to_string())?;
    if root == factory || root.join("templates/managed-skeleton.json").exists() {
        return Err("temporary upgrade cannot target a factory repository".into());
    }
    let id = &options["--project"];
    let identity_paths = identity(&root, id, runner)?;
    let contract_bytes =
        fs::read(factory.join("templates/managed-skeleton.json")).map_err(|e| e.to_string())?;
    let contract = crate::json::parse_unique_json(&contract_bytes, "upgrade target")?;
    let target = contract["release_version"]
        .as_str()
        .ok_or("target release missing")?;
    let version = target.parse::<crate::release::SemVer>()?;
    if version.major != 9 {
        return Err("temporary upgrade target must be 9.x".into());
    }
    let stamp =
        project_sync::transaction_file_state(&root.join(".codex/.bridgeforge_codex_version"))?
            .ok_or("source stamp missing")?;
    let source = std::str::from_utf8(&stamp)
        .map_err(|e| e.to_string())?
        .trim();
    let mut plan = if source == target {
        project_sync::build_plan(&root, &factory, project_sync::SyncMode::Update)?
    } else {
        project_sync::build_legacy_upgrade_plan(&root, &factory)?
    };
    project_sync::bind_identity_reads(&mut plan, &identity_paths)?;
    if options.contains_key("--preview") {
        let mut output = project_sync::outcome_plan_with_format(Ok(plan), "machine");
        if let Some(receipt) = output.receipt.as_mut() {
            receipt["temporary_project"] = json!(id);
        }
        return Ok(output);
    }
    let fingerprint = &options["--plan-fingerprint"];
    if &plan.aggregate_fingerprint != fingerprint {
        return Err("aggregate fingerprint drifted; regenerate preview".into());
    }
    if !plan.gaps.is_empty() || !plan.blockers.is_empty() {
        return Err("unresolved gap or blocker prevents upgrade".into());
    }
    if !plan.risk.is_empty() && !options.contains_key("--confirmed-risk") {
        return Err("risk actions require --confirmed-risk".into());
    }
    if plan.public_installation_required {
        let prepared = options
            .get("--public-artifacts")
            .ok_or("--apply requires --public-artifacts from managed public-artifacts prepare")?;
        project_sync::attach_prepared_public(&mut plan, &contract_bytes, Path::new(prepared))?;
    }
    if identity(&root, id, runner)? != identity_paths {
        return Err("repository identity paths drifted".into());
    }
    let receipt =
        project_sync::apply_plan(plan, fingerprint, options.contains_key("--confirmed-risk"))?;
    let mut value = serde_json::to_value(receipt).map_err(|e| e.to_string())?;
    value["temporary_project"] = json!(id);
    Ok(CommandOutcome::with_receipt(value))
}

pub fn run(args: &[String], runner: &dyn ProcessRunner) -> CommandOutcome {
    execute(args, runner).unwrap_or_else(|error| {
        CommandOutcome::blocked(format!("[upgrade-legacy-seven] BLOCKED: {error}\n"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn legacy_upgrade_argument_guards() {
        for tail in [
            vec![],
            vec!["--apply"],
            vec!["--preview", "--apply"],
            vec!["--check"],
            vec!["--preview", "--preview"],
        ] {
            let args = std::iter::once("upgrade-legacy-seven")
                .chain(tail)
                .map(str::to_owned)
                .collect::<Vec<_>>();
            assert!(parse(&args).is_err());
        }
        assert!(project("OtherProject").is_err());
        assert_ne!(
            origin_identity("https://github.com/other/StratusAgent.git"),
            project("StratusAgent").unwrap().0
        );
    }
}
