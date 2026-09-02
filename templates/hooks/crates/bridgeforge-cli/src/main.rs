use bridgeforge_core::{CommandOutcome, EXIT_BLOCKED, ProjectContext, SystemProcessRunner};
use serde_json::{Value, json};
use std::ffi::OsString;
use std::fs;
use std::io::{Read, Write};
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
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
    CommandOutcome::with_receipt(json!({
        "schema": 1,
        "name": "bridgeforge",
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION")
    }))
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
        "project-structure" => {
            let report = bridgeforge_core::project_structure::inspect(context.root());
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

fn project_sync(args: &[String]) -> CommandOutcome {
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
    let applying = has(args, "--apply");
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
    let contract: Value = match fs::read(template_root.join("templates/managed-skeleton.json"))
        .map_err(|error| error.to_string())
        .and_then(|payload| serde_json::from_slice(&payload).map_err(|error| error.to_string()))
    {
        Ok(value) => value,
        Err(error) => {
            return blocked(
                "project-sync",
                format!("cannot read generated asset contract: {error}"),
            );
        }
    };
    if let Err(error) = bridgeforge_core::project_sync::attach_generated_assets(
        &mut plan,
        &template_root,
        &project_root,
        &contract,
        &SystemProcessRunner,
    ) {
        return bridgeforge_core::project_sync::outcome_receipt_with_format(
            Err(error),
            &output_format,
        );
    }
    if !applying {
        return bridgeforge_core::project_sync::outcome_plan_with_format(Ok(plan), &output_format);
    }
    bridgeforge_core::project_sync::outcome_receipt_with_format(
        bridgeforge_core::project_sync::apply_plan(
            plan,
            fingerprint
                .as_deref()
                .expect("apply fingerprint was validated"),
            has(args, "--confirmed-risk"),
        ),
        &output_format,
    )
}

fn git_sync(args: &[String]) -> CommandOutcome {
    let root = value(args, "--root").map(PathBuf::from);
    let context = match ProjectContext::discover(root.as_deref()) {
        Ok(value) => value,
        Err(error) => return blocked("git-sync", error),
    };
    bridgeforge_core::git_sync::sync(
        context.root(),
        &SystemProcessRunner,
        bridgeforge_core::git_sync::GitSyncOptions {
            message: value(args, "--message").or_else(|| value(args, "-m")),
            message_file: value(args, "--message-file").map(PathBuf::from),
            remote: value(args, "--remote").unwrap_or_else(|| "origin".into()),
            skip_fetch: has(args, "--skip-fetch"),
            skip_push: has(args, "--skip-push"),
        },
    )
}

fn memory_paths(args: &[String]) -> Result<(PathBuf, PathBuf, PathBuf, PathBuf), String> {
    if let Some(project) = value(args, "--project-root") {
        ProjectContext::discover(Some(Path::new(&project))).map_err(|error| error.to_string())?;
    }
    let codex = value(args, "--codex-home")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("CODEX_HOME").map(PathBuf::from))
        .or_else(|| std::env::var_os("USERPROFILE").map(|home| PathBuf::from(home).join(".codex")))
        .ok_or_else(|| {
            "--codex-home is required when CODEX_HOME and USERPROFILE are unavailable".to_string()
        })?;
    let memories = value(args, "--memories")
        .map(PathBuf::from)
        .unwrap_or_else(|| codex.join("memories"));
    let state = value(args, "--state-dir")
        .map(PathBuf::from)
        .unwrap_or_else(|| codex.join(".bridgeforge-codex/native-memory-sync"));
    for (actual, expected, flag) in [
        (&memories, codex.join("memories"), "--memories"),
        (
            &state,
            codex.join(".bridgeforge-codex/native-memory-sync"),
            "--state-dir",
        ),
    ] {
        if memory_path_identity(actual)? != memory_path_identity(&expected)? {
            return Err(format!(
                "{flag} is outside the fixed authorized Codex home scope"
            ));
        }
    }
    let ledger = codex.join("bridgeforge-codex-managed.json");
    Ok((codex, memories, state, ledger))
}

fn memory_path_identity(path: &Path) -> Result<PathBuf, String> {
    let absolute = std::path::absolute(path).map_err(|error| error.to_string())?;
    let mut normalized = PathBuf::new();
    for part in absolute.components() {
        match part {
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            std::path::Component::CurDir => {}
            _ => normalized.push(part),
        }
    }
    let mut ancestor = normalized.as_path();
    while !ancestor.exists() {
        ancestor = ancestor
            .parent()
            .ok_or("memory scope has no existing ancestor")?;
    }
    Ok(ancestor
        .canonicalize()
        .map_err(|error| error.to_string())?
        .join(
            normalized
                .strip_prefix(ancestor)
                .map_err(|error| error.to_string())?,
        ))
}

fn authorized_memory_remote(
    args: &[String],
    ledger: &Path,
    state: &Path,
) -> Result<String, String> {
    let approved = authorized_remote(ledger, state)?;
    if let Some(explicit) = value(args, "--remote")
        && bridgeforge_core::memory::normalize_remote(&explicit) != approved
    {
        return Err("--remote differs from the approved native memories remote".into());
    }
    Ok(approved)
}

fn authorized_remote(ledger: &Path, state: &Path) -> Result<String, String> {
    bridgeforge_core::memory::require_runtime_authorization(ledger, &state.join("remote.txt"))
        .map_err(|error| error.to_string())?
        .remote
        .ok_or_else(|| "approved native memories authorization has no remote".into())
}

fn memory_operation_outcome(
    state_dir: &Path,
    result: bridgeforge_core::memory::MemoryResult<String>,
) -> CommandOutcome {
    match result {
        Ok(action) => {
            let status = match action.as_str() {
                "conflicted" => "conflicted",
                "busy" => "busy",
                _ => "healthy",
            };
            if let Err(error) =
                bridgeforge_core::memory::record_health(state_dir, status, None, Some(&action))
            {
                return blocked("memory-sync", error);
            }
            CommandOutcome::with_receipt(json!({"schema": 1, "status": status, "action": action}))
        }
        Err(error) => {
            let detail = error.to_string();
            let _ =
                bridgeforge_core::memory::record_health(state_dir, "failed", Some(&detail), None);
            blocked("memory-sync", detail)
        }
    }
}

fn memory_failure_outcome(state_dir: &Path, detail: impl ToString) -> CommandOutcome {
    let detail = detail.to_string();
    let _ = bridgeforge_core::memory::record_health(state_dir, "failed", Some(&detail), None);
    blocked("memory-sync", detail)
}

fn launch_memory_worker(args: &[String], state: &Path) -> Result<String, String> {
    use bridgeforge_core::memory::worker::{WorkerReservation, reserve_worker};
    let reservation = reserve_worker(state).map_err(|error| error.to_string())?;
    let WorkerReservation::Acquired(worker) = reservation else {
        return Ok("reused".into());
    };
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    let codex = path_value(args, "--codex-home")?;
    let mut command = Command::new(executable);
    command
        .args([
            "memory-sync",
            "worker",
            "--codex-home",
            codex.to_string_lossy().as_ref(),
            "--token",
            &worker.token,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    for flag in ["--state-dir", "--memories"] {
        if let Some(path) = value(args, flag) {
            command.args([flag, &path]);
        }
    }
    #[cfg(windows)]
    command.creation_flags(0x0800_0000 | 0x0000_0008);
    command.spawn().map_err(|error| {
        let _ = bridgeforge_core::memory::worker::release_worker(state, &worker.token);
        format!("cannot launch hidden memory worker: {error}")
    })?;
    Ok("launched".into())
}

fn memory_sync(args: &[String]) -> CommandOutcome {
    let Some(command) = args.first().map(String::as_str) else {
        return blocked("memory-sync", "a subcommand is required");
    };
    let (codex, memories, state_dir, ledger) = match memory_paths(args) {
        Ok(value) => value,
        Err(error) => return blocked("memory-sync", error),
    };
    match command {
        "setup" => {
            let Some(remote) = value(args, "--remote") else {
                return blocked("memory-sync", "setup requires --remote");
            };
            if let Err(error) =
                bridgeforge_core::memory::MemoryRemoteClient::new(&SystemProcessRunner)
                    .verify_private_github_repository(&codex, &remote)
            {
                return blocked("memory-sync", error);
            }
            let binary = match std::env::current_exe() {
                Ok(value) => value,
                Err(error) => return blocked("memory-sync", error),
            };
            match bridgeforge_core::memory::user_config::configure(
                &codex,
                &binary,
                &remote,
                has(args, "--confirmed-enable"),
            ) {
                Ok(authorization) => CommandOutcome::with_receipt(json!({
                    "schema": 1,
                    "status": "configured",
                    "hookInstalled": true,
                    "runtime": binary,
                    "authorization": authorization,
                })),
                Err(error) => blocked("memory-sync", error),
            }
        }
        "decline" => {
            match bridgeforge_core::memory::record_native_memories_consent(
                &ledger,
                "declined",
                has(args, "--confirmed"),
                None,
            ) {
                Ok(changed) => CommandOutcome::with_receipt(
                    json!({"schema": 1, "status": "declined", "changed": changed}),
                ),
                Err(error) => blocked("memory-sync", error),
            }
        }
        "maintain" | "repair-hook" => {
            if let Err(error) = authorized_remote(&ledger, &state_dir) {
                return blocked("memory-sync", error);
            }
            if !bridgeforge_core::memory::user_config::memories_enabled(&codex) {
                return blocked(
                    "memory-sync",
                    "native memories are disabled by the user; hook repair is not authorized",
                );
            }
            let binary = match std::env::current_exe() {
                Ok(value) => value,
                Err(error) => return blocked("memory-sync", error),
            };
            match bridgeforge_core::memory::user_config::merge_user_hooks(&codex, &binary) {
                Ok(changed) => CommandOutcome::with_receipt(
                    json!({"schema": 1, "status": "healthy", "hookRepair": if changed {"applied"} else {"unchanged"}}),
                ),
                Err(error) => blocked("memory-sync", error),
            }
        }
        "mark" => {
            let trigger = value(args, "--trigger").unwrap_or_else(|| "bridgeforge".into());
            match bridgeforge_core::memory::worker::mark_pending(&state_dir, &trigger) {
                Ok(state) => CommandOutcome::with_receipt(serde_json::to_value(state).unwrap()),
                Err(error) => blocked("memory-sync", error),
            }
        }
        "status" => {
            let authorization = if ledger.is_file() {
                bridgeforge_core::memory::native_memories_authorization(&ledger)
            } else {
                Ok(None)
            };
            let authorization_error = authorization.as_ref().err().map(ToString::to_string);
            let authorization = authorization.ok().flatten();
            let consent = authorization.as_ref().map(|value| value.decision.clone());
            let enabled = bridgeforge_core::memory::user_config::memories_enabled(&codex);
            let binary = std::env::current_exe().ok();
            let hook_installed = binary.as_deref().is_some_and(|binary| {
                bridgeforge_core::memory::user_config::user_hooks_healthy(&codex, binary)
            });
            let hook_runtime: Option<Value> = fs::read(state_dir.join("hook-runtime.json"))
                .ok()
                .and_then(|payload| serde_json::from_slice(&payload).ok());
            let hook_runtime_verified = hook_runtime
                .as_ref()
                .is_some_and(bridgeforge_core::memory::runtime_receipt_healthy);
            let configured_remote = fs::read_to_string(state_dir.join("remote.txt"))
                .ok()
                .map(|value| bridgeforge_core::memory::normalize_remote(&value));
            let remote_configured = authorization
                .as_ref()
                .and_then(|value| value.remote.as_deref())
                .zip(configured_remote.as_deref())
                .is_some_and(|(authorized, configured)| authorized == configured);
            let pending = bridgeforge_core::memory::worker::read_pending(&state_dir)
                .ok()
                .flatten();
            let pending_age_seconds = pending
                .as_ref()
                .and_then(|_| bridgeforge_core::memory::worker::pending_age(&state_dir).ok())
                .map(|age| age.as_secs())
                .unwrap_or(0);
            let worker = bridgeforge_core::memory::worker::read_worker_state(&state_dir)
                .ok()
                .flatten();
            let worker_active = worker
                .as_ref()
                .is_some_and(bridgeforge_core::memory::worker::worker_is_live);
            let conflict: Option<Value> = fs::read(state_dir.join("active-conflict.json"))
                .ok()
                .and_then(|payload| serde_json::from_slice(&payload).ok());
            let last_receipt: Option<Value> = fs::read(state_dir.join("last-synced.json"))
                .ok()
                .and_then(|payload| serde_json::from_slice(&payload).ok());
            let health_receipt = bridgeforge_core::memory::read_health(&state_dir)
                .ok()
                .flatten();
            let (sync_health, active_alert_id) = if authorization_error.is_some() {
                (
                    "failed",
                    Some("native-memory:authorization-invalid".to_string()),
                )
            } else if conflict.is_some() {
                (
                    "conflicted",
                    conflict
                        .as_ref()
                        .and_then(|value| value["conflictId"].as_str())
                        .map(|id| format!("native-memory:conflict:{id}")),
                )
            } else if health_receipt
                .as_ref()
                .and_then(|value| value["status"].as_str())
                == Some("failed")
            {
                (
                    "failed",
                    health_receipt
                        .as_ref()
                        .and_then(|value| value["alertId"].as_str())
                        .map(str::to_string),
                )
            } else if pending_age_seconds > 300 {
                (
                    "degraded",
                    pending.as_ref().map(|value| {
                        format!("native-memory:pending-stale:{}", value.first_pending_utc)
                    }),
                )
            } else if worker_active {
                ("busy", None)
            } else if pending.is_some() {
                ("pending", None)
            } else if consent.as_deref() == Some("approved")
                && enabled
                && hook_installed
                && hook_runtime_verified
                && remote_configured
                && last_receipt.is_some()
            {
                ("healthy", None)
            } else {
                ("gap", None)
            };
            let alert_id =
                bridgeforge_core::memory::emit_alert_once(&state_dir, active_alert_id.as_deref())
                    .ok()
                    .flatten();
            let code = if sync_health == "failed" {
                EXIT_BLOCKED
            } else {
                0
            };
            CommandOutcome {
                code,
                receipt: Some(json!({
                "schema": 1,
                "consent": consent,
                "enabled": enabled,
                "disabledByUser": consent.as_deref() == Some("approved") && !enabled,
                "hookInstalled": hook_installed,
                "hookRuntimeVerified": hook_runtime_verified,
                "hookRuntime": hook_runtime,
                "remoteConfigured": remote_configured,
                "pending": pending,
                "pendingAgeSeconds": pending_age_seconds,
                "worker": worker,
                "workerActive": worker_active,
                "activeConflict": conflict,
                "lastReceipt": last_receipt,
                "healthReceipt": health_receipt,
                "syncHealth": sync_health,
                "alertId": alert_id,
                "activeAlertId": active_alert_id,
                "error": authorization_error,
                })),
                ..CommandOutcome::default()
            }
        }
        "ack-alert" => {
            let Some(alert_id) = value(args, "--alert-id") else {
                return blocked("memory-sync", "ack-alert requires --alert-id");
            };
            match bridgeforge_core::memory::acknowledge_alert(&state_dir, &alert_id) {
                Ok(()) => CommandOutcome::with_receipt(
                    json!({"schema": 1, "status": "acknowledged", "alertId": alert_id}),
                ),
                Err(error) => blocked("memory-sync", error),
            }
        }
        "reconcile" => {
            let remote = match authorized_memory_remote(args, &ledger, &state_dir) {
                Ok(value) => value,
                Err(error) => return blocked("memory-sync", error),
            };
            memory_operation_outcome(
                &state_dir,
                bridgeforge_core::memory::remote::reconcile(
                    &memories,
                    &state_dir,
                    &remote,
                    &SystemProcessRunner,
                ),
            )
        }
        "resolve" => {
            let Some(conflict_id) = value(args, "--conflict-id") else {
                return blocked("memory-sync", "resolve requires --conflict-id");
            };
            let remote = match authorized_memory_remote(args, &ledger, &state_dir) {
                Ok(value) => value,
                Err(error) => return memory_failure_outcome(&state_dir, error),
            };
            let choices = match values(args, "--choose")
                .into_iter()
                .map(|choice| {
                    choice
                        .split_once('=')
                        .map(|(path, side)| (path.to_string(), side.to_string()))
                        .ok_or_else(|| format!("invalid conflict choice: {choice}"))
                })
                .collect::<Result<Vec<_>, _>>()
            {
                Ok(value) if !value.is_empty() => value,
                Ok(_) => {
                    return blocked(
                        "memory-sync",
                        "resolve requires one --choose per conflict path",
                    );
                }
                Err(error) => return blocked("memory-sync", error),
            };
            memory_operation_outcome(
                &state_dir,
                bridgeforge_core::memory::remote::resolve_conflict_with_choices(
                    &memories,
                    &state_dir,
                    &remote,
                    &conflict_id,
                    &choices,
                    &SystemProcessRunner,
                ),
            )
        }
        "kick" => {
            let trigger = value(args, "--trigger").unwrap_or_else(|| "bridgeforge".into());
            if let Err(error) = authorized_remote(&ledger, &state_dir) {
                return memory_failure_outcome(&state_dir, error);
            }
            if let Err(error) = bridgeforge_core::memory::worker::mark_pending(&state_dir, &trigger)
            {
                return memory_failure_outcome(&state_dir, error);
            }
            match launch_memory_worker(args, &state_dir) {
                Ok(action) => CommandOutcome::with_receipt(
                    json!({"schema": 1, "status": "pending", "worker": action}),
                ),
                Err(error) => memory_failure_outcome(&state_dir, error),
            }
        }
        "worker" => {
            let Some(token) = value(args, "--token") else {
                return blocked("memory-sync", "worker requires --token");
            };
            match bridgeforge_core::memory::worker::mark_worker_started(
                &state_dir,
                &token,
                std::process::id(),
            ) {
                Ok(true) => {}
                Ok(false) => {
                    return blocked("memory-sync", "worker reservation is no longer owned");
                }
                Err(error) => return blocked("memory-sync", error),
            }
            let remote = match authorized_remote(&ledger, &state_dir) {
                Ok(value) => value,
                Err(error) => {
                    let _ = bridgeforge_core::memory::worker::release_worker(&state_dir, &token);
                    return memory_failure_outcome(&state_dir, error);
                }
            };
            let result =
                bridgeforge_core::memory::worker::drain_pending(&state_dir, &token, || {
                    bridgeforge_core::memory::remote::reconcile(
                        &memories,
                        &state_dir,
                        &remote,
                        &SystemProcessRunner,
                    )
                })
                .and_then(|(action, restart)| {
                    if restart {
                        launch_memory_worker(args, &state_dir)
                            .map_err(bridgeforge_core::memory::MemorySyncError::new)?;
                    }
                    Ok(action)
                });
            memory_operation_outcome(&state_dir, result)
        }
        "hook-run" => {
            let Some(event) = value(args, "--event") else {
                return blocked("memory-sync", "hook-run requires --event");
            };
            if !bridgeforge_core::memory::user_config::HOOK_EVENTS.contains(&event.as_str()) {
                return blocked("memory-sync", format!("unsupported hook event: {event}"));
            }
            if !bridgeforge_core::memory::user_config::memories_enabled(&codex) {
                return CommandOutcome::ok();
            }
            if let Err(error) = authorized_remote(&ledger, &state_dir) {
                return memory_failure_outcome(&state_dir, error);
            }
            if let Err(error) = bridgeforge_core::memory::atomic_write_json(
                &state_dir.join("hook-runtime.json"),
                &json!({
                    "schema": 1,
                    "lastEvent": event,
                    "handlerRevision": bridgeforge_core::memory::user_config::HOOK_ID,
                    "verifiedUtc": bridgeforge_core::memory::utc_now(),
                }),
            ) {
                return blocked("memory-sync", error);
            }
            let trigger = event.to_lowercase();
            if let Err(error) = bridgeforge_core::memory::worker::mark_pending(&state_dir, &trigger)
            {
                return memory_failure_outcome(&state_dir, error);
            }
            match launch_memory_worker(args, &state_dir) {
                Ok(_) => CommandOutcome::ok(),
                Err(error) => memory_failure_outcome(&state_dir, error),
            }
        }
        _ => blocked("memory-sync", format!("unknown subcommand: {command}")),
    }
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
        "git-sync" => git_sync(args),
        "memory-sync" => memory_sync(&args[1..]),
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
            stderr: "usage: bridgeforge <self-test|doctor|check|archive-scan|audit-user-allow|project-sync|git-sync|memory-sync|manifest|build-assets|batch>\n".into(),
            ..CommandOutcome::default()
        },
    }
}

fn main() {
    std::process::exit(emit(run(&std::env::args().skip(1).collect::<Vec<_>>())));
}

#[cfg(all(test, bridgeforge_factory_tests))]
#[path = "../../../../../scripts/tests/unit/cli.rs"]
mod tests;
