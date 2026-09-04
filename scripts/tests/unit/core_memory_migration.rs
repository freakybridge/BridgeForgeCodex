use super::*;
use crate::memory::{
    build_snapshot, record_conflict, record_native_memories_consent, snapshot_files, worker,
};
use serde_json::json;

fn fixture(name: &str) -> PathBuf {
    let token = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let home = std::env::temp_dir().join(format!("bridgeforge-{name}-{token}"));
    fs::create_dir_all(&home).unwrap();
    fs::write(
        home.join("bridgeforge-codex-managed.json"),
        json!({"schema_version":1,"platform":"codex","records":{}}).to_string(),
    )
    .unwrap();
    home
}

fn authorize(home: &Path, remote: &str) {
    record_native_memories_consent(
        &home.join("bridgeforge-codex-managed.json"),
        "approved",
        true,
        Some(remote),
    )
    .unwrap();
}

#[test]
fn durable_legacy_state_migrates_once_without_copying_transient_history() {
    let home = fixture("memory-state-migration");
    let legacy = legacy_state_dir(&home);
    let state = home.join(".bridgeforge-codex/native-memory-sync");
    let remote = "https://github.com/offline-fixture/bridgeforge-codex-memories";
    authorize(&home, remote);
    fs::create_dir_all(&legacy).unwrap();
    fs::write(legacy.join("remote.txt"), format!("{remote}\n")).unwrap();
    fs::write(
        legacy.join("pending.json"),
        serde_json::to_vec_pretty(&PendingState {
            schema_version: 2,
            first_pending_utc: "2026-08-01T00:00:00Z".into(),
            updated_utc: "2026-08-02T00:00:00Z".into(),
            trigger: "sessionend".into(),
            triggers: vec!["sessionstart".into(), "sessionend".into()],
        })
        .unwrap(),
    )
    .unwrap();
    fs::write(legacy.join("worker.json"), b"transient").unwrap();
    fs::write(legacy.join("health.json"), b"legacy-health").unwrap();
    fs::create_dir_all(legacy.join("conflicts/unreferenced")).unwrap();
    fs::write(
        legacy.join("conflicts/unreferenced/should-not-copy.txt"),
        b"history",
    )
    .unwrap();

    let baseline_memories = home.join("baseline-memories");
    fs::create_dir_all(&baseline_memories).unwrap();
    fs::write(baseline_memories.join("MEMORY.md"), b"baseline").unwrap();
    let baseline = legacy.join("last-synced-snapshot");
    let baseline_manifest = build_snapshot(&baseline_memories, &baseline, 7).unwrap();
    fs::write(
        legacy.join("last-synced.json"),
        serde_json::to_vec_pretty(&json!({
            "schemaVersion": 2,
            "content_sha256": baseline_manifest.content_sha256,
            "revision": baseline_manifest.revision,
            "commit": "fixture",
            "utc": "2026-08-02T00:00:00Z"
        }))
        .unwrap(),
    )
    .unwrap();

    let local = home.join("conflict-local");
    let remote_snapshot = home.join("conflict-remote");
    fs::write(baseline_memories.join("MEMORY.md"), b"local").unwrap();
    build_snapshot(&baseline_memories, &local, 8).unwrap();
    fs::write(baseline_memories.join("MEMORY.md"), b"remote").unwrap();
    build_snapshot(&baseline_memories, &remote_snapshot, 8).unwrap();
    let merged = snapshot_files(&local).unwrap();
    let conflict = record_conflict(
        &legacy,
        &local,
        &remote_snapshot,
        Some(&baseline),
        &merged,
        &["MEMORY.md".into()],
        Some("fixture"),
        8,
        "fixture conflict",
    )
    .unwrap();

    let receipt = migrate_legacy_state(&home, &state, &home.join("bridgeforge-codex-managed.json"))
        .unwrap()
        .unwrap();
    assert_eq!(receipt.status, "completed");
    assert!(receipt.migrated.contains(&"remote".into()));
    assert!(receipt.migrated.contains(&"baseline".into()));
    assert!(receipt.migrated.contains(&"active-conflict".into()));
    assert!(receipt.migrated.contains(&"pending".into()));
    assert_eq!(
        super::normalize_remote(&fs::read_to_string(state.join("remote.txt")).unwrap()),
        remote
    );
    assert_eq!(
        worker::read_pending(&state)
            .unwrap()
            .unwrap()
            .first_pending_utc,
        "2026-08-01T00:00:00Z"
    );
    assert!(state.join("last-synced-snapshot").is_dir());
    assert!(state.join("conflicts").join(&conflict.conflict_id).is_dir());
    let active: Value =
        serde_json::from_slice(&fs::read(state.join("active-conflict.json")).unwrap()).unwrap();
    let expected_conflict_path = state
        .join("conflicts")
        .join(&conflict.conflict_id)
        .display()
        .to_string();
    assert_eq!(
        active["path"].as_str(),
        Some(expected_conflict_path.as_str())
    );
    assert!(!state.join("worker.json").exists());
    assert!(!state.join("health.json").exists());
    assert!(!state.join("conflicts/unreferenced").exists());

    fs::remove_file(state.join("active-conflict.json")).unwrap();
    fs::remove_dir_all(state.join("conflicts").join(&conflict.conflict_id)).unwrap();
    let repeated =
        migrate_legacy_state(&home, &state, &home.join("bridgeforge-codex-managed.json"))
            .unwrap()
            .unwrap();
    assert_eq!(repeated.source_fingerprint, receipt.source_fingerprint);
    assert!(!state.join("active-conflict.json").exists());
    assert!(!state.join("conflicts").join(&conflict.conflict_id).exists());
    fs::remove_dir_all(home).unwrap();
}

#[test]
fn legacy_remote_mismatch_fails_without_publishing_a_completion_marker() {
    let home = fixture("memory-state-migration-scope");
    let legacy = legacy_state_dir(&home);
    let state = home.join(".bridgeforge-codex/native-memory-sync");
    authorize(
        &home,
        "https://github.com/offline-fixture/bridgeforge-codex-memories",
    );
    fs::create_dir_all(&legacy).unwrap();
    fs::write(
        legacy.join("remote.txt"),
        "https://github.com/another/bridgeforge-codex-memories",
    )
    .unwrap();
    let error = migrate_legacy_state(&home, &state, &home.join("bridgeforge-codex-managed.json"))
        .unwrap_err()
        .to_string();
    assert!(error.contains("differs from the approved"));
    assert!(!state.join(MARKER_NAME).exists());
    fs::remove_dir_all(home).unwrap();
}
