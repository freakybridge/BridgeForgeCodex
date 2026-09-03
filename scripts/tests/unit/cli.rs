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
        "hookRuntimeVerified",
        "remoteConfigured",
        "syncHealth",
        "alertId",
    ] {
        assert!(receipt.get(field).is_some(), "missing status field {field}");
    }
    assert_eq!(receipt["syncHealth"], "gap");
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
fn memory_failed_health_is_persistent_and_alerts_once() {
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
    let first = run(&args);
    assert_eq!(first.code, EXIT_BLOCKED);
    assert!(first.receipt.as_ref().unwrap()["alertId"].is_string());
    assert_eq!(first.receipt.as_ref().unwrap()["syncHealth"], "failed");
    let second = run(&args);
    assert_eq!(second.receipt.as_ref().unwrap()["alertId"], Value::Null);
    assert!(second.receipt.as_ref().unwrap()["activeAlertId"].is_string());
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
