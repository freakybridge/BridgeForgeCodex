use bridgeforge_core::memory::ownership::{
    ExpectedHooksState, MANAGED_ID_KEY, canonicalize, expected_groups, hooks_file_healthy,
    load_document, managed_document_healthy, merge_hooks_file, merge_managed_document,
};
use bridgeforge_core::memory::worker::{
    ReconcileLock, WorkerReservation, mark_pending, read_pending, reserve_worker,
};
use bridgeforge_core::memory::{
    atomic_write, atomic_write_json, build_snapshot, capture_manifest, managed_ledger,
    memory_files, record_conflict, snapshot_files, three_way_merge, validate_authorization,
};
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static TEMP_ID: AtomicU64 = AtomicU64::new(0);

struct TempDirectory {
    path: PathBuf,
}

impl TempDirectory {
    fn new(label: &str) -> Self {
        let id = TEMP_ID.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "bridgeforge-memory-test-{label}-{}-{id}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir(&path).expect("create test directory");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn expected_hook_document() -> Value {
    json!({
        "hooks": {
            "SessionStart": [{
                "hooks": [{
                    "type": "command",
                    "command": ".codex/bin/bridgeforge memory-sync hook-run --event SessionStart",
                    MANAGED_ID_KEY: "bridgeforge-codex.native-memory-sync.v1:SessionStart"
                }]
            }]
        }
    })
}

#[test]
fn ownership_merge_preserves_external_hooks_and_rejects_marked_drift() {
    let expected_document = expected_hook_document();
    let expected = expected_groups(
        &expected_document,
        "bridgeforge-codex.native-memory-sync.v1:",
    )
    .expect("expected hooks");
    let external = br#"{
      "hooks": {
        "Stop": [{"matcher":"external","hooks":[{"type":"command","command":"external"}]}]
      }
    }"#;
    let merged = merge_managed_document(
        Some(external),
        &expected,
        &["bridgeforge-codex.native-memory-sync.v1:"],
        "hooks.json",
        None,
    )
    .expect("merge managed hooks");
    let document = load_document(&merged, "hooks.json").expect("merged document");
    assert_eq!(
        document["hooks"]["Stop"][0]["hooks"][0]["command"],
        "external"
    );
    assert!(managed_document_healthy(
        &merged,
        &expected,
        &["bridgeforge-codex.native-memory-sync.v1:"],
        "hooks.json",
        None,
    ));

    let mut drifted = expected_document;
    drifted["hooks"]["SessionStart"][0]["hooks"][0]["command"] =
        Value::String("tampered".to_string());
    let error = canonicalize(
        &drifted,
        &expected,
        &["bridgeforge-codex.native-memory-sync.v1:"],
        "hooks.json",
        None,
        false,
        None,
    )
    .expect_err("marked drift must block");
    assert!(error.to_string().contains("content drifted"));

    let temp = TempDirectory::new("hooks-cas");
    let hooks_path = temp.path().join("hooks.json");
    assert!(
        merge_hooks_file(
            &hooks_path,
            &expected,
            &["bridgeforge-codex.native-memory-sync.v1:"],
            None,
            ExpectedHooksState::Missing,
        )
        .expect("atomic hooks merge")
    );
    assert!(hooks_file_healthy(
        &hooks_path,
        &expected,
        &["bridgeforge-codex.native-memory-sync.v1:"],
        None,
    ));
    assert!(
        !merge_hooks_file(
            &hooks_path,
            &expected,
            &["bridgeforge-codex.native-memory-sync.v1:"],
            None,
            ExpectedHooksState::Any,
        )
        .expect("idempotent hooks merge")
    );
}

#[test]
fn authorization_ledger_is_exact_and_invalid_scope_blocks() {
    let temp = TempDirectory::new("ledger");
    let ledger = temp.path().join("ledger.json");
    let invalid = json!({
        "schema_version": 1,
        "platform": "codex",
        "records": {},
        "consents": {
            "native_memories": {
                "decision": "approved",
                "policy_version": 1,
                "scope": "~/too-broad/**",
                "sync_mode": "bidirectional",
                "auto_hook_maintenance": true,
                "repository": "bridgeforge-codex-memories",
                "require_private": true,
                "remote": "git@github.com:owner/bridgeforge-codex-memories"
            }
        }
    });
    atomic_write_json(&ledger, &invalid).expect("write invalid ledger");
    let error = managed_ledger(&ledger).expect_err("invalid authorization must block");
    assert!(error.to_string().contains("authorization scope"));

    let value = invalid["consents"]["native_memories"].clone();
    assert!(validate_authorization(&value).is_err());

    let duplicate = br#"{"schema_version":1,"schema_version":1,"platform":"codex","records":{}}"#;
    atomic_write(&ledger, duplicate).expect("write duplicate-key ledger");
    assert!(
        managed_ledger(&ledger)
            .expect_err("duplicate key must block")
            .to_string()
            .contains("duplicate JSON key")
    );
}

#[test]
fn filtering_manifest_and_directory_snapshot_are_stable() {
    let temp = TempDirectory::new("snapshot");
    let memories = temp.path().join("memories");
    fs::create_dir_all(memories.join("nested")).expect("create memories");
    fs::create_dir_all(memories.join(".git")).expect("create excluded git");
    fs::write(memories.join("alpha.md"), b"alpha").expect("write alpha");
    fs::write(memories.join("nested/beta.jsonl"), b"beta").expect("write beta");
    fs::write(memories.join("ignored.tmp"), b"ignored").expect("write excluded suffix");
    fs::write(memories.join(".git/config"), b"ignored").expect("write excluded directory");
    fs::write(memories.join("snapshot-manifest.json"), b"ignored").expect("write excluded name");

    let files = memory_files(&memories).expect("scan memories");
    assert_eq!(files.len(), 2);
    let manifest = capture_manifest(&memories, 7, Some("2026-09-01T00:00:00+00:00"))
        .expect("capture manifest");
    assert_eq!(manifest.revision, 7);
    assert_eq!(
        manifest
            .files
            .iter()
            .map(|item| item.path.as_str())
            .collect::<Vec<_>>(),
        vec!["alpha.md", "nested/beta.jsonl"]
    );

    let snapshot = temp.path().join("snapshot");
    let built = build_snapshot(&memories, &snapshot, 7).expect("build snapshot");
    assert_eq!(built.content_sha256, manifest.content_sha256);
    let payloads = snapshot_files(&snapshot).expect("read snapshot");
    assert_eq!(payloads["alpha.md"], b"alpha");
    assert_eq!(payloads["nested/beta.jsonl"], b"beta");
}

#[test]
fn reconcile_lock_and_worker_reservation_are_single_owner() {
    let temp = TempDirectory::new("worker");
    let state = temp.path().join("state");
    fs::create_dir(&state).expect("create state");
    let first = ReconcileLock::try_acquire(&state)
        .expect("first lock")
        .expect("first lock acquired");
    assert!(
        ReconcileLock::try_acquire(&state)
            .expect("second lock attempt")
            .is_none()
    );
    drop(first);
    assert!(
        ReconcileLock::try_acquire(&state)
            .expect("lock after release")
            .is_some()
    );

    let first_worker = reserve_worker(&state).expect("reserve worker");
    let token = match first_worker {
        WorkerReservation::Acquired(ref value) => value.token.clone(),
        WorkerReservation::Reused(_) => panic!("first worker must be acquired"),
    };
    match reserve_worker(&state).expect("reuse worker") {
        WorkerReservation::Reused(value) => assert_eq!(value.token, token),
        WorkerReservation::Acquired(_) => panic!("second worker must be reused"),
    }

    mark_pending(&state, "SessionStart").expect("first pending");
    mark_pending(&state, "Stop").expect("second pending");
    let pending = read_pending(&state)
        .expect("read pending")
        .expect("pending state");
    assert_eq!(pending.triggers, vec!["SessionStart", "Stop"]);
}

#[test]
fn three_way_conflict_creates_durable_evidence() {
    let temp = TempDirectory::new("conflict");
    let base_source = temp.path().join("base-source");
    let local_source = temp.path().join("local-source");
    let remote_source = temp.path().join("remote-source");
    for source in [&base_source, &local_source, &remote_source] {
        fs::create_dir(source).expect("create source");
    }
    fs::write(base_source.join("same.md"), b"before").expect("write base");
    fs::write(local_source.join("same.md"), b"local").expect("write local");
    fs::write(remote_source.join("same.md"), b"remote").expect("write remote");
    let base = temp.path().join("base");
    let local = temp.path().join("local");
    let remote = temp.path().join("remote");
    build_snapshot(&base_source, &base, 1).expect("base snapshot");
    build_snapshot(&local_source, &local, 2).expect("local snapshot");
    build_snapshot(&remote_source, &remote, 2).expect("remote snapshot");

    let merged = three_way_merge(
        &snapshot_files(&base).expect("base files"),
        &snapshot_files(&local).expect("local files"),
        &snapshot_files(&remote).expect("remote files"),
    );
    assert_eq!(merged.conflicts, vec!["same.md"]);
    assert!(!merged.files.contains_key("same.md"));

    let state = temp.path().join("state");
    let record = record_conflict(
        &state,
        &local,
        &remote,
        Some(&base),
        &merged.files,
        &merged.conflicts,
        Some("0123456789012345678901234567890123456789"),
        3,
        "both local and remote changed",
    )
    .expect("record conflict");
    assert_eq!(record.conflict_id.len(), 16);
    assert!(
        state
            .join("conflicts")
            .join(&record.conflict_id)
            .join("local")
            .is_dir()
    );
    let active: Value = serde_json::from_slice(
        &fs::read(state.join("active-conflict.json")).expect("active conflict"),
    )
    .expect("active JSON");
    assert_eq!(active["conflictId"], record.conflict_id);
}

#[test]
fn atomic_write_replaces_content_without_leaving_stage_files() {
    let temp = TempDirectory::new("atomic");
    let target = temp.path().join("state.json");
    atomic_write(&target, b"old").expect("write old");
    atomic_write(&target, b"new").expect("replace atomically");
    assert_eq!(fs::read(&target).expect("read target"), b"new");
    let names = fs::read_dir(temp.path())
        .expect("read directory")
        .map(|entry| entry.expect("entry").file_name())
        .collect::<Vec<_>>();
    assert_eq!(names, vec![target.file_name().unwrap().to_os_string()]);
}
