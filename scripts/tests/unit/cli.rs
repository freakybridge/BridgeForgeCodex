use super::*;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn explicit_memory_parameters_cannot_bypass_consent_or_scope() {
    let home = std::env::temp_dir().join(format!(
        "bf-auth-scope-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&home).unwrap();
    let remote = "https://github.com/offline-fixture/bridgeforge-codex-memories";
    let mut args = vec![
        "memory-sync".into(),
        "reconcile".into(),
        "--codex-home".into(),
        home.display().to_string(),
        "--remote".into(),
        remote.into(),
    ];
    let result = run(&args);
    assert_eq!(result.code, EXIT_BLOCKED);
    assert!(
        !home.join(".bridgeforge-codex").exists(),
        "unauthorized calls must not create runtime state"
    );
    fs::write(
        home.join("bridgeforge-codex-managed.json"),
        json!({"schema_version":1,"platform":"codex","records":{}}).to_string(),
    )
    .unwrap();
    bridgeforge_core::memory::user_config::configure(
        &home,
        &std::env::current_exe().unwrap(),
        remote,
        true,
    )
    .unwrap();
    args[5] = "https://github.com/another/bridgeforge-codex-memories".into();
    assert!(run(&args).stderr.contains("differs from the approved"));
    args[5] = remote.into();
    for flag in ["--memories", "--state-dir"] {
        let mut outside = args.clone();
        outside.extend([flag.into(), home.join("outside").display().to_string()]);
        assert!(run(&outside).stderr.contains("fixed authorized"));
    }
    assert_eq!(
        authorized_memory_remote(
            &args,
            &home.join("bridgeforge-codex-managed.json"),
            &home.join(".bridgeforge-codex/native-memory-sync")
        )
        .unwrap(),
        remote
    );
    fs::remove_dir_all(home).unwrap();
}

#[test]
fn default_metadata_gate_checks_native_project_skills() {
    let home = std::env::temp_dir().join(format!(
        "bf-skill-gate-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(home.join(".codex/skills/broken")).unwrap();
    fs::write(
        home.join(".codex/skills/broken/SKILL.md"),
        b"# Missing metadata\n",
    )
    .unwrap();
    let result = run(&[
        "check".into(),
        "skill-metadata".into(),
        "--root".into(),
        home.display().to_string(),
    ]);
    assert_eq!(result.code, EXIT_BLOCKED);
    assert!(
        result.receipt.unwrap()["issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|issue| issue.as_str().unwrap().contains("frontmatter"))
    );
    fs::remove_dir_all(home).unwrap();
}

#[test]
fn every_lifecycle_event_queues_and_reuses_worker_without_network() {
    use bridgeforge_core::memory::{user_config, worker};
    let token = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let home = std::env::temp_dir().join(format!("bfc-lifecycle-queue-{token}"));
    let state = home.join(".bridgeforge-codex/native-memory-sync");
    fs::create_dir_all(&state).unwrap();
    let remote = "https://github.com/offline-fixture/bridgeforge-codex-memories";
    fs::write(
        home.join("bridgeforge-codex-managed.json"),
        json!({"schema_version":1,"platform":"codex","records":{}}).to_string(),
    )
    .unwrap();
    user_config::configure(&home, &std::env::current_exe().unwrap(), remote, true).unwrap();
    assert_eq!(
        authorized_remote(&home.join("bridgeforge-codex-managed.json"), &state).unwrap(),
        remote
    );
    let worker::WorkerReservation::Acquired(lease) = worker::reserve_worker(&state).unwrap() else {
        panic!()
    };
    assert!(worker::mark_worker_started(&state, &lease.token, std::process::id()).unwrap());
    for event in user_config::HOOK_EVENTS {
        let outcome = run(&[
            "memory-sync".into(),
            "hook-run".into(),
            "--event".into(),
            (*event).into(),
            "--codex-home".into(),
            home.display().to_string(),
        ]);
        assert_eq!(outcome.code, 0, "{event}: {}", outcome.stderr);
        assert_eq!(
            worker::read_worker_state(&state).unwrap().unwrap().token,
            lease.token
        );
        assert!(
            worker::read_pending(&state)
                .unwrap()
                .unwrap()
                .triggers
                .contains(&event.to_lowercase())
        );
        let attempt: Value =
            serde_json::from_slice(&fs::read(state.join("hook-attempt.json")).unwrap()).unwrap();
        assert_eq!(attempt["status"], "completed");
        assert_eq!(attempt["event"], *event);
        let runtime: Value =
            serde_json::from_slice(&fs::read(state.join("hook-runtime.json")).unwrap()).unwrap();
        assert_eq!(runtime["lastEvent"], *event);
        assert!(!state.join("last-synced.json").exists());
    }
    worker::release_worker(&state, &lease.token).unwrap();
    fs::write(
        state.join("remote.txt"),
        "https://github.com/another/bridgeforge-codex-memories",
    )
    .unwrap();
    assert!(authorized_remote(&home.join("bridgeforge-codex-managed.json"), &state).is_err());
    fs::remove_dir_all(home).unwrap();
}

#[test]
fn self_test_has_stable_identity() {
    let receipt = self_test().receipt.expect("receipt");
    assert_eq!(receipt["name"], "bridgeforge");
    assert_eq!(receipt["status"], "ok");
}

#[test]
fn memory_session_start_notifies_failure_once_and_rearms_after_recovery() {
    let home = std::env::temp_dir().join(format!(
        "bf-memory-notice-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let state = home.join(".bridgeforge-codex/native-memory-sync");
    fs::create_dir_all(&state).unwrap();
    let args = vec![
        "hook-run".into(),
        "--codex-home".into(),
        home.display().to_string(),
    ];
    bridgeforge_core::memory::record_health(&state, "failed", Some("network failure"), None)
        .unwrap();
    assert!(
        memory_hook_notice(&args, &state, "Stop")
            .unwrap()
            .receipt
            .is_none()
    );
    let first = memory_hook_notice(&args, &state, "SessionStart").unwrap();
    assert!(
        first.receipt.unwrap()["systemMessage"]
            .as_str()
            .unwrap()
            .contains("Memory")
    );
    assert!(
        memory_hook_notice(&args, &state, "SessionStart")
            .unwrap()
            .receipt
            .is_none()
    );
    let query = vec![
        "status".into(),
        "--codex-home".into(),
        home.display().to_string(),
    ];
    assert_eq!(memory_sync(&query).receipt.unwrap()["syncHealth"], "failed");
    bridgeforge_core::memory::record_health(&state, "healthy", None, Some("noop")).unwrap();
    bridgeforge_core::memory::record_health(&state, "failed", Some("network failure"), None)
        .unwrap();
    assert!(
        memory_hook_notice(&args, &state, "SessionStart")
            .unwrap()
            .receipt
            .is_some()
    );
    bridgeforge_core::memory::record_health(&state, "healthy", None, Some("noop")).unwrap();
    assert!(
        memory_hook_notice(&args, &state, "SessionStart")
            .unwrap()
            .receipt
            .is_none()
    );
    fs::remove_dir_all(home).unwrap();
}

#[test]
fn memory_hook_failure_notifies_and_busy_does_not_erase_the_failure() {
    let home = std::env::temp_dir().join(format!(
        "bf-memory-failed-hook-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let args = vec![
        "memory-sync".into(),
        "hook-run".into(),
        "--codex-home".into(),
        home.display().to_string(),
        "--event".into(),
        "SessionStart".into(),
    ];
    fs::create_dir_all(&home).unwrap();
    fs::write(
        home.join("config.toml"),
        "[features]\nmemories = true\n[memories]\ngenerate_memories = true\nuse_memories = true\n",
    )
    .unwrap();
    let failed = run(&args);
    assert_ne!(failed.code, 0);
    assert!(failed.receipt.unwrap()["systemMessage"].is_string());
    let state = home.join(".bridgeforge-codex/native-memory-sync");
    let before = fs::read(state.join("health.json")).unwrap();
    memory_operation_outcome(&state, Ok("busy".into()));
    assert_eq!(fs::read(state.join("health.json")).unwrap(), before);
    assert!(run(&args).receipt.is_none());
    fs::remove_dir_all(home).unwrap();
}

#[test]
fn memory_status_rejects_corrupt_state_and_missing_baseline_without_writes() {
    let home = std::env::temp_dir().join(format!(
        "bf-memory-corrupt-state-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let state = home.join(".bridgeforge-codex/native-memory-sync");
    fs::create_dir_all(&state).unwrap();
    let args = vec![
        "memory-sync".into(),
        "status".into(),
        "--codex-home".into(),
        home.display().to_string(),
    ];
    for name in [
        "pending.json",
        "worker.json",
        "health.json",
        "active-conflict.json",
        "last-synced.json",
    ] {
        let path = state.join(name);
        fs::write(&path, b"{truncated").unwrap();
        let result = run(&args);
        assert_eq!(result.code, EXIT_BLOCKED, "{name}");
        assert_eq!(result.receipt.unwrap()["syncHealth"], "failed");
        assert_eq!(fs::read(&path).unwrap(), b"{truncated");
        assert!(!state.join("alert-state.json").exists());
        fs::remove_file(path).unwrap();
    }
    fs::write(state.join("last-synced.json"), json!({"schemaVersion":2,"content_sha256":"missing","revision":1,"commit":null,"utc":"2026-01-01T00:00:00Z"}).to_string()).unwrap();
    assert!(
        run(&args).receipt.unwrap()["runtimeStateError"]
            .as_str()
            .unwrap()
            .contains("baseline")
    );
    fs::remove_dir_all(home).unwrap();
}

#[test]
fn memory_conflict_and_stale_pending_notices_respect_acknowledgement() {
    for kind in ["conflicted", "degraded"] {
        let home = std::env::temp_dir().join(format!(
            "bf-memory-{kind}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let state = home.join(".bridgeforge-codex/native-memory-sync");
        fs::create_dir_all(&state).unwrap();
        if kind == "conflicted" {
            fs::write(
                state.join("active-conflict.json"),
                json!({"schemaVersion":1,"conflictId":"fixture"}).to_string(),
            )
            .unwrap();
        } else {
            let mut pending =
                bridgeforge_core::memory::worker::mark_pending(&state, "stop").unwrap();
            pending.first_pending_utc = "2000-01-01T00:00:00Z".into();
            bridgeforge_core::memory::atomic_write_json(&state.join("pending.json"), &pending)
                .unwrap();
        }
        let args = vec![
            "hook-run".into(),
            "--codex-home".into(),
            home.display().to_string(),
        ];
        assert!(
            memory_hook_notice(&args, &state, "SessionStart")
                .unwrap()
                .receipt
                .unwrap()["systemMessage"]
                .is_string()
        );
        assert!(
            memory_hook_notice(&args, &state, "SessionStart")
                .unwrap()
                .receipt
                .is_none()
        );
        let status = memory_sync(&[
            "status".into(),
            "--codex-home".into(),
            home.display().to_string(),
        ])
        .receipt
        .unwrap();
        assert_eq!(status["syncHealth"], kind);
        fs::remove_file(state.join("alert-state.json")).unwrap();
        bridgeforge_core::memory::acknowledge_alert(
            &state,
            status["activeAlertId"].as_str().unwrap(),
        )
        .unwrap();
        assert!(
            memory_hook_notice(&args, &state, "SessionStart")
                .unwrap()
                .receipt
                .is_none()
        );
        fs::remove_dir_all(home).unwrap();
    }
}

#[test]
fn parser_collects_repeated_batch_roots_in_order() {
    let args = vec![
        "--project-root".into(),
        "a".into(),
        "--project-root".into(),
        "b".into(),
    ];
    assert_eq!(values(&args, "--project-root"), vec!["a", "b"]);
}

#[test]
fn memory_hook_repair_requires_prior_authorization() {
    let token = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let home = std::env::temp_dir().join(format!("bridgeforge-memory-repair-{token}"));
    fs::create_dir_all(&home).unwrap();
    let outcome = run(&[
        "memory-sync".into(),
        "repair-hook".into(),
        "--codex-home".into(),
        home.display().to_string(),
    ]);
    assert_eq!(outcome.code, EXIT_BLOCKED);
    assert!(!home.join("hooks.json").exists());
    fs::remove_dir_all(home).unwrap();
}

#[test]
fn memory_status_reports_consent_runtime_remote_health_and_alert_fields() {
    let token = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let home = std::env::temp_dir().join(format!("bridgeforge-memory-status-{token}"));
    fs::create_dir_all(&home).unwrap();
    let outcome = run(&[
        "memory-sync".into(),
        "status".into(),
        "--codex-home".into(),
        home.display().to_string(),
    ]);
    assert_eq!(outcome.code, 0);
    let receipt = outcome.receipt.unwrap();
    for field in [
        "consent",
        "enabled",
        "hookInstalled",
        "hookConfigured",
        "hookDispatchObserved",
        "hookRuntimeVerified",
        "remoteConfigured",
        "stateMigrationNeeded",
        "stateMigrationCompleted",
        "syncHealth",
        "alertId",
    ] {
        assert!(receipt.get(field).is_some(), "missing status field {field}");
    }
    assert_eq!(receipt["syncHealth"], "gap");
    assert_eq!(receipt["hookConfigured"], false);
    assert_eq!(receipt["hookDispatchObserved"], false);
    assert_eq!(receipt["hookRuntimeVerified"], false);
    assert_eq!(receipt["remoteConfigured"], false);
    fs::remove_dir_all(home).unwrap();
}

#[cfg(windows)]
#[test]
fn memory_status_agrees_for_windows_path_separator_aliases_without_writing() {
    let token = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let home = std::env::temp_dir().join(format!("memory-cli-path-alias-{token}"));
    fs::create_dir_all(&home).unwrap();
    let binary = std::env::current_exe().unwrap();
    let document = bridgeforge_core::memory::user_config::expected_document(&binary, &home);
    let payload = serde_json::to_vec_pretty(&document).unwrap();
    fs::write(home.join("hooks.json"), &payload).unwrap();
    let mut receipts = Vec::new();
    for spelling in [
        home.to_string_lossy().replace('\\', "/"),
        home.to_string_lossy().replace('/', "\\"),
    ] {
        let outcome = run(&[
            "memory-sync".into(),
            "status".into(),
            "--codex-home".into(),
            spelling,
        ]);
        assert_eq!(outcome.code, 0);
        let receipt = outcome.receipt.unwrap();
        assert_eq!(receipt["hookInstalled"], true);
        assert_eq!(receipt["hookRuntimeVerified"], false);
        receipts.push(receipt);
        assert_eq!(fs::read(home.join("hooks.json")).unwrap(), payload);
        assert!(!home.join(".bridgeforge-codex").exists());
    }
    assert_eq!(receipts[0], receipts[1]);
    fs::remove_dir_all(home).unwrap();
}

#[test]
fn memory_status_is_read_only_and_alert_remains_until_acknowledged() {
    let token = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let home = std::env::temp_dir().join(format!("bridgeforge-memory-health-{token}"));
    let state = home.join(".bridgeforge-codex/native-memory-sync");
    fs::create_dir_all(&state).unwrap();
    bridgeforge_core::memory::record_health(&state, "failed", Some("simulated failure"), None)
        .unwrap();
    let args = vec![
        "memory-sync".into(),
        "status".into(),
        "--codex-home".into(),
        home.display().to_string(),
    ];
    let health_before = fs::read(state.join("health.json")).unwrap();
    let first = run(&args);
    assert_eq!(first.code, EXIT_BLOCKED);
    assert!(first.receipt.as_ref().unwrap()["alertId"].is_string());
    assert_eq!(first.receipt.as_ref().unwrap()["syncHealth"], "failed");
    let second = run(&args);
    assert_eq!(
        second.receipt.as_ref().unwrap()["alertId"],
        first.receipt.as_ref().unwrap()["alertId"]
    );
    assert!(second.receipt.as_ref().unwrap()["activeAlertId"].is_string());
    assert_eq!(fs::read(state.join("health.json")).unwrap(), health_before);
    assert!(!state.join("alert-state.json").exists());
    let alert_id = second.receipt.as_ref().unwrap()["alertId"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(
        run(&[
            "memory-sync".into(),
            "ack-alert".into(),
            "--alert-id".into(),
            alert_id,
            "--codex-home".into(),
            home.display().to_string(),
        ])
        .code,
        0
    );
    assert_eq!(run(&args).receipt.as_ref().unwrap()["alertId"], Value::Null);
    bridgeforge_core::memory::record_health(&state, "healthy", None, Some("recovered")).unwrap();
    bridgeforge_core::memory::record_health(&state, "failed", Some("simulated failure"), None)
        .unwrap();
    let recurring = run(&args);
    assert!(recurring.receipt.as_ref().unwrap()["alertId"].is_string());
    assert!(!bridgeforge_core::memory::runtime_receipt_healthy(
        &json!({"schema": 1})
    ));
    assert!(bridgeforge_core::memory::runtime_receipt_healthy(&json!({
        "schema": 1,
        "lastEvent": "SessionStart",
        "handlerRevision": bridgeforge_core::memory::user_config::HOOK_ID,
        "verifiedUtc": bridgeforge_core::memory::utc_now(),
    })));
    assert!(!bridgeforge_core::memory::runtime_receipt_healthy(&json!({
        "schema": 1,
        "lastEvent": "SessionStart",
        "handlerRevision": "obsolete-handler",
        "verifiedUtc": bridgeforge_core::memory::utc_now(),
    })));
    assert!(!bridgeforge_core::memory::runtime_receipt_healthy(&json!({
        "schema": 1,
        "lastEvent": "SessionStart",
        "handlerRevision": bridgeforge_core::memory::user_config::HOOK_ID,
        "verifiedUtc": "2000-01-01T00:00:00Z",
    })));
    fs::remove_dir_all(home).unwrap();
}
