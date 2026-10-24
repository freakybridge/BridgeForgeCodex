mod memory_commands;

use bridgeforge_core::{CommandOutcome, EXIT_BLOCKED, ProjectContext, SystemProcessRunner};
use serde_json::{Value, json};
use std::ffi::OsString;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

fn emit(outcome: CommandOutcome) -> i32 {
    if let Some(receipt) = outcome.receipt {
        println!("{receipt}");
    } else if !outcome.stdout.is_empty() {
        print!("{}", outcome.stdout);
    }
    if !outcome.stderr.is_empty() {
        let _ = std::io::stderr().write_all(outcome.stderr.as_bytes());
    }
    outcome.code
}

fn blocked(label: &str, error: impl std::fmt::Display) -> CommandOutcome {
    CommandOutcome::blocked(format!("[{label}] BLOCKED: {error}\n"))
}

fn value(args: &[String], flag: &str) -> Option<String> {
    args.windows(2)
        .find(|pair| pair[0] == flag)
        .map(|pair| pair[1].clone())
}

fn values(args: &[String], flag: &str) -> Vec<String> {
    args.windows(2)
        .filter(|pair| pair[0] == flag)
        .map(|pair| pair[1].clone())
        .collect()
}

fn has(args: &[String], flag: &str) -> bool {
    args.iter().any(|item| item == flag)
}

fn path_value(args: &[String], flag: &str) -> Result<PathBuf, String> {
    value(args, flag)
        .map(PathBuf::from)
        .ok_or_else(|| format!("{flag} is required"))
}

fn self_test() -> CommandOutcome {
    let mut outcome=CommandOutcome::with_receipt(json!({
        "schema": 1,
        "name": "bridgeforge",
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "capabilities": ["git-sync-explicit-release-v1", "git-sync-prepared-release-v1", "git-sync-release-only-v1", "git-sync-release-readiness-v1", "git-sync-direct-release-v1", "git-sync-timing-v1", "git-sync-setup-v1", "git-sync-analysis-reuse-v1", "git-sync-package-lifecycle-v1", "user-cli-reuse-v1", "project-sync-build-reuse-v1", "git-sync-preflight-v1", "git-sync-phase-timing-v1", "public-artifacts-v1"]
    }));
    if let Some(capabilities)=outcome.receipt.as_mut().and_then(|r|r["capabilities"].as_array_mut()){capabilities.push(json!("git-sync-schema4-v1"));capabilities.push(json!("git-sync-product-packages-v1"));capabilities.push(json!("git-sync-baseline-repair-v1"));}
    if let Some(capabilities) = outcome.receipt.as_mut().and_then(|r| r["capabilities"].as_array_mut()) {
        capabilities.push(json!("factory-runtime-prepare-v1"));
        capabilities.push(json!("package-development-v1"));
        capabilities.push(json!("package-stability-v1"));
        capabilities.push(json!("git-sync-development-closure-v1"));
    }
    outcome
}

fn instruction_source() -> CommandOutcome {
    let hook = std::env::current_exe()
        .ok()
        .as_deref()
        .and_then(Path::parent)
        .map(|directory| {
            #[cfg(windows)]
            {
                directory.join("bridgeforge-hook.exe")
            }
            #[cfg(not(windows))]
            {
                directory.join("bridgeforge-hook")
            }
        });
    match (ProjectContext::discover(None), hook) {
        (Ok(context), Some(hook)) => {
            let mut request =
                bridgeforge_core::ProcessRequest::new(hook.into_os_string(), context.root());
            request.args = vec![
                OsString::from("instruction-source"),
                OsString::from("--pre-commit"),
            ];
            request.timeout = Duration::from_secs(30);
            match bridgeforge_core::ProcessRunner::run(&SystemProcessRunner, &request) {
                Ok(output) => CommandOutcome {
                    code: output.code,
                    stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                    stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
                    receipt: None,
                },
                Err(error) => blocked("instruction-source", error),
            }
        }
        (Err(error), _) => blocked("instruction-source", error),
        (_, None) => blocked("instruction-source", "cannot locate hook binary"),
    }
}

fn check(command: &str, args: &[String]) -> CommandOutcome {
    let root = value(args, "--root").map(PathBuf::from);
    let context = match ProjectContext::discover(root.as_deref()) {
        Ok(value) => value,
        Err(error) => return blocked(command, error),
    };
    match command {
        "baseline" => {
            if has(args, "--index") {
                return match bridgeforge_core::baseline::verify_index(
                    context.root(),
                    &SystemProcessRunner,
                ) {
                    Ok(report) => {
                        CommandOutcome::with_receipt(serde_json::to_value(report).unwrap())
                    }
                    Err(error) => blocked("current-baseline", error),
                };
            }
            let factory_contract = context.root().join("templates/managed-skeleton.json");
            let contract = factory_contract.is_file().then_some(factory_contract);
            match bridgeforge_core::baseline::verify(
                context.root(),
                contract.as_deref(),
                !has(args, "--skip-generated-runtime"),
            ) {
                Ok(report) => CommandOutcome::with_receipt(serde_json::to_value(report).unwrap()),
                Err(error) => blocked("current-baseline", error),
            }
        }
        "todo" => {
            if has(args, "--contract") {
                return CommandOutcome::with_receipt(bridgeforge_core::todo::contract());
            }
            let report = bridgeforge_core::todo::inspect(context.root());
            CommandOutcome {
                code: if report.errors.is_empty() {
                    0
                } else {
                    EXIT_BLOCKED
                },
                receipt: Some(serde_json::to_value(report).unwrap()),
                ..CommandOutcome::default()
            }
        }
        "project-structure" => {
            let report = if has(args, "--pre-commit") {
                bridgeforge_core::project_structure::pre_commit(context.root())
            } else {
                bridgeforge_core::project_structure::inspect(context.root())
            };
            CommandOutcome {
                code: if report.errors.is_empty() {
                    0
                } else {
                    EXIT_BLOCKED
                },
                receipt: Some(serde_json::to_value(report).unwrap()),
                ..CommandOutcome::default()
            }
        }
        "skill-metadata" => {
            let roots = if let Some(path) = value(args, "--skill-root") {
                vec![PathBuf::from(path)]
            } else {
                let mut roots = vec![context.root().join(".codex/skills")];
                if context
                    .root()
                    .join("bridgeforge-codex-manifest.json")
                    .is_file()
                {
                    roots.push(context.root().join("skills"));
                }
                roots
            };
            let mut report = bridgeforge_core::skill_metadata::SkillReport::default();
            for skill_root in roots {
                let checked = bridgeforge_core::skill_metadata::validate_tree(&skill_root);
                report.issues.extend(checked.issues);
                report.warnings.extend(checked.warnings);
            }
            CommandOutcome {
                code: if report.issues.is_empty() {
                    0
                } else {
                    EXIT_BLOCKED
                },
                receipt: Some(serde_json::to_value(report).unwrap()),
                ..CommandOutcome::default()
            }
        }
        "instruction-source" => instruction_source(),
        "factory-version" => {
            let report = bridgeforge_core::factory_version::check(context.root());
            CommandOutcome {
                code: if report.healthy { 0 } else { EXIT_BLOCKED },
                receipt: Some(serde_json::to_value(report).unwrap()),
                ..CommandOutcome::default()
            }
        }
        "proposal" => {
            let proposal = value(args, "--proposal-root")
                .map(PathBuf::from)
                .unwrap_or_else(|| context.root().join("doc/2_bugs/BUG-agents-ia/proposal"));
            let report = bridgeforge_core::proposal_contract::validate(&proposal);
            CommandOutcome {
                code: if report.healthy { 0 } else { EXIT_BLOCKED },
                receipt: Some(serde_json::to_value(report).unwrap()),
                ..CommandOutcome::default()
            }
        }
        _ => blocked("check", format!("unknown check: {command}")),
    }
}

fn with_build_metrics(
    mut out: CommandOutcome,
    metrics: bridgeforge_core::build_metrics::BuildMetrics,
    format: &str,
) -> CommandOutcome {
    if format != "human" {
        let receipt = out
            .receipt
            .get_or_insert_with(|| json!({"status":"blocked"}));
        let machine = if format == "combined" {
            &mut receipt["machine"]
        } else {
            receipt
        };
        machine["build_metrics"] = serde_json::to_value(metrics).unwrap();
    }
    out
}

fn project_sync(args: &[String]) -> CommandOutcome {
    let started = std::time::Instant::now();
    let mut out = project_sync_inner(args);
    let format = value(args, "--output-format").unwrap_or_else(|| "machine".into());
    if format != "human" {
        if let Some(receipt) = out.receipt.as_mut() {
            let machine = if format == "combined" {
                &mut receipt["machine"]
            } else {
                receipt
            };
            machine["elapsed_ms"] = json!(started.elapsed().as_millis());
        }
    }
    out
}

fn project_sync_inner(args: &[String]) -> CommandOutcome {
    let output_format = value(args, "--output-format").unwrap_or_else(|| "machine".into());
    if !matches!(output_format.as_str(), "machine" | "human" | "combined") {
        return blocked(
            "project-sync",
            "invalid --output-format; expected machine|human|combined",
        );
    }
    let project_root = match path_value(args, "--project-root") {
        Ok(value) => value,
        Err(error) => return blocked("project-sync", error),
    };
    let template_root = match path_value(args, "--template-root") {
        Ok(value) => value,
        Err(error) => return blocked("project-sync", error),
    };
    let mode = match value(args, "--mode").as_deref().unwrap_or("auto") {
        "auto" => bridgeforge_core::project_sync::SyncMode::Auto,
        "init" => bridgeforge_core::project_sync::SyncMode::Init,
        "adopt" => bridgeforge_core::project_sync::SyncMode::Adopt,
        "update" => bridgeforge_core::project_sync::SyncMode::Update,
        other => return blocked("project-sync", format!("invalid mode: {other}")),
    };
    let migration_manifest: Option<Value> = match value(args, "--asset-migration-manifest") {
        Some(path) => {
            let payload = if path == "-" {
                let mut payload = Vec::new();
                match std::io::stdin().read_to_end(&mut payload) {
                    Ok(_) => Ok(payload),
                    Err(error) => Err(error.to_string()),
                }
            } else {
                fs::read(&path).map_err(|error| error.to_string())
            };
            match payload.and_then(|payload| {
                serde_json::from_slice(&payload).map_err(|error| error.to_string())
            }) {
                Ok(value) => Some(value),
                Err(error) => {
                    return blocked(
                        "project-sync",
                        format!("cannot read asset migration manifest: {error}"),
                    );
                }
            }
        }
        None => None,
    };
    let preserved = values(args, "--preserve-project-asset");
    let deleted = values(args, "--delete-project-asset");
    let preservation = (!preserved.is_empty() || !deleted.is_empty())
        .then(|| json!({"preserve": preserved, "delete": deleted}));
    let planning_started = std::time::Instant::now();
    let plan_result = bridgeforge_core::project_sync::build_plan_with_inputs(
        &project_root,
        &template_root,
        mode,
        migration_manifest.as_ref(),
        preservation.as_ref(),
    );
    let mut plan = match plan_result {
        Ok(value) => value,
        Err(error) => {
            return bridgeforge_core::project_sync::outcome_receipt_with_format(
                Err(error),
                &output_format,
            );
        }
    };
    let planning_ms = planning_started.elapsed().as_millis();
    let applying = has(args, "--apply");
    if !applying {
        plan.build_metrics
            .timings_ms
            .insert("planning".into(), planning_ms);
        return bridgeforge_core::project_sync::outcome_plan_with_format(Ok(plan), &output_format);
    }
    let fingerprint = if applying {
        let Some(value) = value(args, "--plan-fingerprint") else {
            return bridgeforge_core::project_sync::outcome_receipt_with_format(
                Err(
                    "--apply requires --plan-fingerprint from the immediately preceding plan"
                        .into(),
                ),
                &output_format,
            );
        };
        if plan.aggregate_fingerprint != value {
            return bridgeforge_core::project_sync::outcome_receipt_with_format(
                Err("aggregate fingerprint drifted; regenerate the plan".into()),
                &output_format,
            );
        }
        Some(value)
    } else {
        None
    };
    if applying
        && plan.asset_migration["status"].as_str() == Some("confirmed")
        && !has(args, "--confirmed-asset-migration")
    {
        return blocked(
            "project-sync",
            "confirmed migration packages require --confirmed-asset-migration",
        );
    }
    let preservation_decided = plan.preservation_manifest["entries"]
        .as_array()
        .is_some_and(|entries| {
            entries
                .iter()
                .any(|entry| matches!(entry["disposition"].as_str(), Some("preserve" | "delete")))
        });
    if applying && preservation_decided && !has(args, "--confirmed-preservation-manifest") {
        return blocked(
            "project-sync",
            "project asset decisions require --confirmed-preservation-manifest",
        );
    }
    let contract_bytes = match fs::read(template_root.join("templates/managed-skeleton.json")) {
        Ok(bytes) => bytes,
        Err(error) => return blocked("project-sync", error),
    };
    let contract: Value = match serde_json::from_slice(&contract_bytes) {
        Ok(value) => value,
        Err(error) => {
            return blocked(
                "project-sync",
                format!("cannot read generated asset contract: {error}"),
            );
        }
    };
    let prepared = if plan.public_installation_required {
        let path = match value(args, "--public-artifacts")
            .map(PathBuf::from)
            .map(Ok)
            .unwrap_or_else(|| {
                bridgeforge_core::public_artifacts::codex_home().and_then(|root| {
                    bridgeforge_core::public_artifacts::prepared_path(&root, &contract)
                })
            }) {
            Ok(path) => path,
            Err(error) => return blocked("project-sync", error),
        };
        bridgeforge_core::project_sync::attach_prepared_public(&mut plan, &contract_bytes, &path)
    } else if contract["generated_assets"]
        .as_array()
        .is_some_and(|items| !items.is_empty())
    {
        Err("target product lacks the prepared-public-artifact contract; refresh the official product first".into())
    } else {
        Ok(())
    };
    if let Err(error) = prepared {
        plan.build_metrics
            .timings_ms
            .insert("planning".into(), planning_ms);
        return with_build_metrics(
            bridgeforge_core::project_sync::outcome_receipt_with_format(Err(error), &output_format),
            plan.build_metrics.clone(),
            &output_format,
        );
    }
    plan.build_metrics
        .timings_ms
        .insert("planning".into(), planning_ms);
    let mut failed_metrics = plan.build_metrics.clone();
    let applying_started = std::time::Instant::now();
    let result = bridgeforge_core::project_sync::apply_plan(
        plan,
        fingerprint
            .as_deref()
            .expect("apply fingerprint was validated"),
        has(args, "--confirmed-risk"),
    )
    .map(|mut receipt| {
        receipt
            .build_metrics
            .timings_ms
            .insert("apply".into(), applying_started.elapsed().as_millis());
        receipt
    });
    failed_metrics
        .timings_ms
        .insert("apply".into(), applying_started.elapsed().as_millis());
    if result.is_err() {
        return with_build_metrics(
            bridgeforge_core::project_sync::outcome_receipt_with_format(result, &output_format),
            failed_metrics,
            &output_format,
        );
    }
    bridgeforge_core::project_sync::outcome_receipt_with_format(result, &output_format)
}

fn git_sync(args: &[String]) -> CommandOutcome {
    if has(args, "--check") || has(args, "--dry-run") {
        return blocked(
            "git-sync",
            "--check/--dry-run cannot execute synchronization or preparation; use --release-status or --release-preview",
        );
    }
    let root = value(args, "--root").map(PathBuf::from);
    let context = match ProjectContext::discover(root.as_deref()) {
        Ok(value) => value,
        Err(error) => return blocked("git-sync", error),
    };
    let modes = [
        "--preflight",
        "--setup-status",
        "--setup-plan",
        "--setup-apply",
        "--release",
        "--release-preview",
        "--prepare-release",
        "--release-status",
        "--development-status",
        "--migration-plan",
        "--migration-apply",
        "--repair-baseline-plan",
        "--repair-baseline-apply",
    ]
    .iter()
    .filter(|flag| has(args, flag))
    .count();
    if modes > 1 {
        return blocked("git-sync", "release modes are mutually exclusive");
    }
    let release_request=bridgeforge_core::release::schema4::Request{analysis_file:value(args,"--release-analysis-file").map(PathBuf::from)};
    if has(args, "--repair-baseline-plan") || has(args, "--repair-baseline-apply") {
        if value(args, "--strategy").is_some() || value(args, "--setup-config-file").is_some() {
            return blocked("git-sync", "baseline repair cannot change configuration strategy or accept a proposal");
        }
        return if has(args, "--repair-baseline-plan") {
            bridgeforge_core::release::schema4::repair_baseline_plan(context.root(), &SystemProcessRunner)
        } else {
            bridgeforge_core::release::schema4::repair_baseline_apply(context.root(), &SystemProcessRunner, &value(args, "--repair-baseline-fingerprint").unwrap_or_default(), has(args, "--confirmed-choice"))
        };
    }
    if has(args,"--migration-plan") {
        return bridgeforge_core::release::schema4::configuration_plan(context.root(),&SystemProcessRunner,value(args,"--setup-config-file").as_deref(),value(args,"--strategy").as_deref());
    }
    if has(args,"--migration-apply") {
        return bridgeforge_core::release::schema4::configuration_apply(context.root(),&SystemProcessRunner,value(args,"--setup-config-file").as_deref(),value(args,"--strategy").as_deref(),&value(args,"--migration-fingerprint").unwrap_or_default(),has(args,"--confirmed-choice"));
    }
    if has(args, "--preflight") {
        let mut outcome =
            bridgeforge_core::git_sync::preflight(context.root(), &SystemProcessRunner);
        if let Some(receipt) = outcome.receipt.as_mut().and_then(|r| r.as_object_mut()) {
            receipt.insert(
                "tool".into(),
                self_test().receipt.expect("self-test receipt"),
            );
        }
        return outcome;
    }
    if has(args, "--setup-status") {
        return bridgeforge_core::release::setup::status_with(
            context.root(),
            &SystemProcessRunner,
            has(args, "--reconfigure"),
        );
    }
    if has(args, "--setup-plan") {
        if let Some(path) = value(args, "--setup-config-file") {
            if value(args, "--strategy").is_some_and(|s| s != "independent") {
                return blocked(
                    "git-sync",
                    "release uses product-packages; alternative strategies are retired",
                );
            }
            return bridgeforge_core::release::setup::plan_configuration(
                context.root(),
                &SystemProcessRunner,
                &path,
            );
        }
        return bridgeforge_core::release::setup::plan(
            context.root(),
            &SystemProcessRunner,
            &value(args, "--strategy").unwrap_or_default(),
        );
    }
    if has(args, "--setup-apply") {
        if let Some(path) = value(args, "--setup-config-file") {
            if value(args, "--strategy").is_some_and(|s| s != "independent") {
                return blocked(
                    "git-sync",
                    "release uses product-packages; alternative strategies are retired",
                );
            }
            return bridgeforge_core::release::setup::apply_configuration(
                context.root(),
                &SystemProcessRunner,
                &path,
                &value(args, "--setup-fingerprint").unwrap_or_default(),
                has(args, "--confirmed-choice"),
            );
        }
        return bridgeforge_core::release::setup::apply(
            context.root(),
            &SystemProcessRunner,
            &value(args, "--strategy").unwrap_or_default(),
            &value(args, "--setup-fingerprint").unwrap_or_default(),
            has(args, "--confirmed-choice"),
        );
    }
    if has(args, "--release-status") {
        return bridgeforge_core::git_sync::release_status(context.root(), &SystemProcessRunner);
    }
    if has(args, "--development-status") {
        return bridgeforge_core::git_sync::development_status(
            context.root(),
            &SystemProcessRunner,
        );
    }
    if has(args, "--prepare-release") {
        let message = if let Some(path) = value(args, "--message-file") {
            match fs::read_to_string(path) {
                Ok(value) => value,
                Err(error) => return blocked("git-sync", error),
            }
        } else {
            value(args, "--message")
                .or_else(|| value(args, "-m"))
                .unwrap_or_default()
        };
        let audit = value(args, "--audit-file").map(PathBuf::from);
        return bridgeforge_core::git_sync::prepare_release_with_request(
            context.root(),
            &SystemProcessRunner,
            &message,
            audit.as_deref(),
            &release_request,
        );
    }
    if has(args, "--release-preview") {
        let message = if let Some(path) = value(args, "--message-file") {
            match fs::read_to_string(path) {
                Ok(value) => value,
                Err(error) => return blocked("git-sync", error),
            }
        } else {
            value(args, "--message")
                .or_else(|| value(args, "-m"))
                .unwrap_or_else(|| "chore: release accumulated changes".into())
        };
        return bridgeforge_core::release::schema4::preview(context.root(), &message, &release_request, &SystemProcessRunner);
    }
    if has(args,"--release") { match bridgeforge_core::release::setup::is_confirmed(context.root()) {
        Ok(true) => {}
        Ok(false) => {
            let mut outcome =
                bridgeforge_core::release::setup::status(context.root(), &SystemProcessRunner);
            if outcome.code == 0 {
                outcome.code = EXIT_BLOCKED;
            }
            return outcome;
        }
        Err(error) => return blocked("git-sync-setup", error),
    } }
    bridgeforge_core::git_sync::sync(
        context.root(),
        &SystemProcessRunner,
        bridgeforge_core::git_sync::GitSyncOptions {
            message: value(args, "--message").or_else(|| value(args, "-m")),
            message_file: value(args, "--message-file").map(PathBuf::from),
            remote: value(args, "--remote").unwrap_or_else(|| "origin".into()),
            skip_fetch: has(args, "--skip-fetch"),
            skip_push: has(args, "--skip-push"),
            release: has(args, "--release"),
            release_request,
        },
    )
}

fn batch(args: &[String]) -> CommandOutcome {
    let Some(command) = args.first().map(String::as_str) else {
        return blocked("batch", "a subcommand is required");
    };
    let state = || path_value(args, "--state");
    let outcome = match command {
        "plan" => {
            let template = match path_value(args, "--template-root") {
                Ok(value) => value,
                Err(error) => return blocked("batch", error),
            };
            let roots = values(args, "--project-root")
                .into_iter()
                .map(PathBuf::from)
                .collect::<Vec<_>>();
            return match bridgeforge_core::batch::plan(&template, &roots) {
                Ok(plan) => CommandOutcome {
                    code: if plan.status == "planned" { 0 } else { 2 },
                    receipt: Some(serde_json::to_value(plan).unwrap()),
                    ..CommandOutcome::default()
                },
                Err(error) => blocked("batch", error),
            };
        }
        "start" => {
            let state = match state() {
                Ok(value) => value,
                Err(error) => return blocked("batch", error),
            };
            let template = match path_value(args, "--template-root") {
                Ok(value) => value,
                Err(error) => return blocked("batch", error),
            };
            let roots = values(args, "--project-root")
                .into_iter()
                .map(PathBuf::from)
                .collect::<Vec<_>>();
            let fingerprint = match value(args, "--plan-fingerprint") {
                Some(value) => value,
                None => return blocked("batch", "start requires --plan-fingerprint"),
            };
            bridgeforge_core::batch::start(&state, &template, &roots, &fingerprint)
        }
        "begin" => state().and_then(|state| bridgeforge_core::batch::begin(&state)),
        "finish" => {
            let state = match state() {
                Ok(value) => value,
                Err(error) => return blocked("batch", error),
            };
            let succeeded = match value(args, "--status").as_deref() {
                Some("succeeded") => true,
                Some("deferred") => false,
                _ => return blocked("batch", "finish requires --status succeeded|deferred"),
            };
            bridgeforge_core::batch::finish(
                &state,
                succeeded,
                value(args, "--result")
                    .unwrap_or_else(|| if succeeded { "completed" } else { "deferred" }.into()),
                value(args, "--issue-signature"),
            )
        }
        "retry" => {
            let state = match state() {
                Ok(value) => value,
                Err(error) => return blocked("batch", error),
            };
            let order = match value(args, "--order").and_then(|value| value.parse::<usize>().ok()) {
                Some(value) => value,
                None => return blocked("batch", "retry requires numeric --order"),
            };
            let fingerprint = match value(args, "--plan-fingerprint") {
                Some(value) => value,
                None => return blocked("batch", "retry requires --plan-fingerprint"),
            };
            bridgeforge_core::batch::retry(&state, order, &fingerprint)
        }
        "restart" => {
            let state = match state() {
                Ok(value) => value,
                Err(error) => return blocked("batch", error),
            };
            let bug_doc = match value(args, "--bug-doc") {
                Some(value) => value,
                None => return blocked("batch", "restart requires --bug-doc"),
            };
            bridgeforge_core::batch::restart(&state, &bug_doc)
        }
        "summary" => state().and_then(|state| bridgeforge_core::batch::load(&state)),
        "close" => state().and_then(|state| bridgeforge_core::batch::close(&state)),
        _ => return blocked("batch", format!("unknown subcommand: {command}")),
    };
    match outcome {
        Ok(state) => CommandOutcome::with_receipt(serde_json::to_value(state).unwrap()),
        Err(error) => blocked("batch", error),
    }
}

fn run(args: &[String]) -> CommandOutcome {
    let Some(command) = args.first().map(String::as_str) else {
        return blocked("bridgeforge", "a command is required");
    };
    match command {
        "self-test" if has(args, "--json") => self_test(),
        "project-hook-migration" => {
            let result = (|| {
                let root = path_value(args,"--root")?.canonicalize().map_err(|e|e.to_string())?;
                match args.get(1).map(String::as_str) {
                    Some("status")=>bridgeforge_core::project_hook_migration::status(&root),
                    Some("verify")=>{
                        if has(args,"--check")||has(args,"--dry-run") {return Err("migration verification builds project code and records completion; use status for a read-only report".into());}
                        bridgeforge_core::project_hook_migration::verify_completed(&root,has(args,"--confirmed-behavior"),&SystemProcessRunner)
                    },
                    _=>Err("expected project-hook-migration status or verify".into()),
                }
            })();
            match result {Ok(v)=>CommandOutcome::with_receipt(v),Err(e)=>blocked("project-hook-migration",e)}
        },
        "public-artifacts" => {
            if args.get(1).map(String::as_str) == Some("locate") {
                return bridgeforge_core::public_artifacts::outcome((|| {
                    bridgeforge_core::public_artifacts::locate_cli(&path_value(args,"--product-root")?,&path_value(args,"--codex-home")?,&SystemProcessRunner)
                })());
            }
            if args.get(1).map(String::as_str) != Some("prepare") {
                return blocked("public-artifacts", "expected prepare");
            }
            if has(args, "--check") || has(args, "--dry-run") {
                return blocked("public-artifacts", "preparation writes a derived cache; preview flags are not supported");
            }
            let result = (|| {
                let product = path_value(args, "--product-root")?;
                let codex = path_value(args, "--codex-home")?;
                let bootstrap = value(args, "--bootstrap-cli").map(PathBuf::from);
                let identity = value(args, "--bootstrap-identity");
                let target = value(args, "--bootstrap-target").map(PathBuf::from);
                bridgeforge_core::public_artifacts::prepare(&product, &codex, bootstrap.as_deref(), identity.as_deref(), target.as_deref(), &SystemProcessRunner)
            })();
            bridgeforge_core::public_artifacts::outcome(result)
        },
        "build-environment" => match path_value(args, "--source-root") {
            Ok(root) => bridgeforge_core::runtime::build_environment(&root, &SystemProcessRunner),
            Err(e) => blocked("build-environment", e),
        },
        "doctor" => match value(args, "--product-root") {
            Some(root) => match std::env::current_exe().map_err(|e| e.to_string()).and_then(|binary|
                bridgeforge_core::runtime::validate_product(Path::new(&root), &binary, &SystemProcessRunner)) {
                Ok(receipt) => CommandOutcome::with_receipt(serde_json::to_value(receipt).unwrap()),
                Err(error) => blocked("bridgeforge-runtime", error),
            },
            None => bridgeforge_core::runtime::outcome(value(args, "--root").as_deref().map(Path::new), &SystemProcessRunner, false),
        },
        "check" => args
            .get(1)
            .map(|name| check(name, args))
            .unwrap_or_else(|| blocked("check", "a check name is required")),
        "archive-scan" => match ProjectContext::discover(value(args, "--root").as_deref().map(Path::new))
            .and_then(|context| bridgeforge_core::archive_scan::scan(context.root()))
        {
            Ok(candidates) => CommandOutcome::with_receipt(json!({"schema": 1, "count": candidates.len(), "candidates": candidates})),
            Err(error) => blocked("archive-scan", error),
        },
        "audit-user-allow" => {
            let path = match path_value(args, "--settings") {
                Ok(value) => value,
                Err(error) => return blocked("audit-user-allow", error),
            };
            match bridgeforge_core::audit_user_allow::audit(&path) {
                Ok(findings) => CommandOutcome::with_receipt(json!({"schema": 1, "count": findings.len(), "findings": findings})),
                Err(error) => blocked("audit-user-allow", error),
            }
        }
        "project-sync" => project_sync(args),
        "user-agents-stage" => {
            if has(args, "--check") || has(args, "--dry-run") {
                return blocked("user-agents-stage", "staging is a write operation; preview flags are not supported");
            }
            let result = (|| {
                let root = path_value(args, "--product-root")?;
                let profile = path_value(args, "--user-profile")?;
                let operation = value(args, "--operation-id").ok_or("--operation-id is required")?;
                bridgeforge_core::user_agents::stage(&root, &profile, &operation)
            })();
            match result {
                Ok(plan) => CommandOutcome::with_receipt(plan),
                Err(error) => blocked("user-agents-stage", error),
            }
        }
        "git-sync" => git_sync(args),
        "develop" => {
            let root=value(args,"--root").map(PathBuf::from);
            let context=match ProjectContext::discover(root.as_deref()) { Ok(c)=>c,Err(e)=>return blocked("develop",e) };
            if ["--preview","--apply","--recover"].iter().filter(|f|has(args,f)).count()!=1
                || ((has(args,"--apply") || has(args,"--recover")) && (has(args,"--check") || has(args,"--dry-run"))) {
                return blocked("develop","choose exactly one of --preview, --apply, --recover; preview flags cannot authorize writes");
            }
            let request=bridgeforge_core::release::schema4::Request{analysis_file:value(args,"--analysis-file").map(PathBuf::from)};
            if has(args,"--recover") { bridgeforge_core::release::schema4::development::recover(context.root()) }
            else if has(args,"--apply") { bridgeforge_core::release::schema4::development::apply(context.root(),&request,&value(args,"--fingerprint").unwrap_or_default(),&SystemProcessRunner) }
            else { bridgeforge_core::release::schema4::development::preview(context.root(),&request,&SystemProcessRunner) }
        }
        "memory-sync" => memory_commands::memory_sync(&args[1..]),
        "manifest" => match ProjectContext::discover(value(args, "--root").as_deref().map(Path::new)) {
            Ok(context) => match bridgeforge_core::manifest::rebuild(context.root(), has(args, "--check")) {
                Ok(true) if has(args, "--check") => blocked("manifest", "generated manifests are stale"),
                Ok(changed) => CommandOutcome::with_receipt(json!({"schema": 1, "status": "ok", "changed": changed})),
                Err(error) => blocked("manifest", error),
            },
            Err(error) => blocked("manifest", error),
        },
        "build-assets" => {
            let project = match path_value(args, "--project-root") {
                Ok(value) => value,
                Err(error) => return blocked("build-assets", error),
            };
            if has(args, "--factory") {
                return bridgeforge_core::factory_runtime::prepare(&project,
                    has(args, "--check") || has(args, "--dry-run"), &SystemProcessRunner);
            }
            if has(args, "--check") || has(args, "--dry-run") {
                return blocked("build-assets", "read-only inspection requires --factory; no build performed");
            }
            let contract_path = project.join(".codex/managed-skeleton.json");
            let contract: Value = match fs::read(&contract_path)
                .map_err(|error| error.to_string())
                .and_then(|payload| serde_json::from_slice(&payload).map_err(|error| error.to_string()))
            {
                Ok(value) => value,
                Err(error) => return blocked("build-assets", error),
            };
            match bridgeforge_core::project_sync::build_generated_assets(
                &project,
                &contract,
                &SystemProcessRunner,
            ) {
                Ok(receipts) => CommandOutcome::with_receipt(json!({"schema": 1, "status": "built", "receipts": receipts})),
                Err(error) => blocked("build-assets", error),
            }
        }
        "batch" => batch(&args[1..]),
        _ => CommandOutcome {
            code: EXIT_BLOCKED,
            stderr: "usage: bridgeforge <self-test|doctor|check|archive-scan|audit-user-allow|project-sync|public-artifacts|project-hook-migration|develop|git-sync|memory-sync|manifest|build-assets|batch>\n".into(),
            ..CommandOutcome::default()
        },
    }
}

fn main() {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    let profile = arguments.first().is_some_and(|v| v == "git-sync")
        && std::env::var_os("BRIDGEFORGE_PROFILE").is_some_and(|v| v == "1");
    if profile { bridgeforge_core::release_metrics::begin(); }
    let started = std::time::Instant::now();
    let mut outcome = run(&arguments);
    if profile {
        let mut metrics = bridgeforge_core::release_metrics::finish();
        metrics["cli_elapsed_us"] = json!(started.elapsed().as_micros() as u64);
        outcome.receipt.get_or_insert_with(|| json!({"status":if outcome.code == 0 {"ok"} else {"blocked"}}))["profile"] = metrics;
    }
    std::process::exit(emit(outcome));
}

#[cfg(all(test, bridgeforge_factory_tests))]
#[path = "../../../../../scripts/tests/unit/cli.rs"]
mod tests;
