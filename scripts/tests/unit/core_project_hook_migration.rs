use super::*;
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "bf-hook-migration-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(root.join(".codex/hooks/project_demo")).unwrap();
        fs::write(root.join(".codex/project-hooks.json"),br#"{"schema_version":1,"hooks":[{"id":"demo","events":[{"event":"Stop","args":[]}]}]}"#).unwrap();
        fs::write(root.join(".codex/hooks.json"),br#"{"hooks":{"Stop":[{"hooks":[{"type":"command","bridgeforgeProjectHookId":"demo:0","command":".codex/bin/project_demo"}]}]}}"#).unwrap();
        // This deliberately invalid source must never be parsed by update.
        fs::write(
            root.join(".codex/hooks/project_demo/entrypoint.rs"),
            [0xff, 0x00],
        )
        .unwrap();
        Self(root)
    }
    fn publish(&self, inspection: &Inspection) {
        for (path, bytes) in &inspection.writes {
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, bytes).unwrap();
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn policy(compatible: bool) -> Vec<u8> {
    serde_json::to_vec(&json!({"schema":1,"owner":"bridgeforge-project-hook-policy-v1","migration_id":"project-hooks-independent-v1","legacy_versions_through":"2.6.2","runtime_compatible":compatible})).unwrap()
}

#[test]
fn legacy_notice_is_read_only_deduplicated_and_survives_upgrade() {
    let f = Fixture::new();
    let first = inspect(&f.0, &policy(true), Some("2.6.2")).unwrap();
    assert_eq!(first.notice["show"], true);
    assert!(!first.blocked);
    assert!(!f.0.join(STATE).exists());
    assert!(first.reads.keys().all(|p| !p.ends_with(".rs")));
    f.publish(&first);
    let again = inspect(&f.0, &policy(true), Some("9.0.0")).unwrap();
    assert_eq!(again.notice["state"], "pending");
    assert_eq!(again.notice["show"], false);
    assert!(again.writes.is_empty());
    let incompatible = inspect(&f.0, &policy(false), Some("9.0.0")).unwrap();
    assert!(
        incompatible.blocked,
        "notice deduplication cannot suppress actual incompatibility"
    );
}

#[test]
fn registration_change_rechecks_only_its_small_identity() {
    let f = Fixture::new();
    f.publish(&inspect(&f.0, &policy(true), Some("2.6.2")).unwrap());
    let path = f.0.join(".codex/hooks.json");
    let mut document: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    document["hooks"]["SessionStart"] = json!([{"hooks":[{"bridgeforgeCodexId":"bridgeforge-codex.project-hook.v1:session-start","command":"new public hook"}]}]);
    fs::write(&path, serde_json::to_vec(&document).unwrap()).unwrap();
    assert_eq!(
        inspect(&f.0, &policy(true), Some("9.0.0")).unwrap().notice["show"],
        false
    );
    document["hooks"]["Stop"][0]["hooks"][0]["command"] = json!(".codex/bin/project_demo changed");
    fs::write(path, serde_json::to_vec(&document).unwrap()).unwrap();
    assert_eq!(
        inspect(&f.0, &policy(true), Some("9.0.0")).unwrap().notice["show"],
        true
    );
}

#[test]
fn missing_or_unknown_completion_evidence_is_not_success() {
    let f = Fixture::new();
    f.publish(&inspect(&f.0, &policy(true), Some("2.6.2")).unwrap());
    let path = f.0.join(STATE);
    let mut value: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    value["state"] = json!("completed");
    fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(
        inspect(&f.0, &policy(true), Some("9.0.0"))
            .err()
            .unwrap()
            .contains("evidence")
    );
}

#[test]
fn unconfirmed_migration_does_not_create_a_lock_or_build() {
    struct Never;
    impl crate::ProcessRunner for Never {
        fn run(&self, _: &crate::ProcessRequest) -> std::io::Result<crate::ProcessOutput> {
            panic!("unexpected process")
        }
    }
    let f = Fixture::new();
    assert!(verify_completed(&f.0, false, &Never).is_err());
    assert!(!f.0.join(".runtime").exists());
}
