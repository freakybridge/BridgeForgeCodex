use super::*;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn hook_removal_matches_path_boundaries_and_platform_commands() {
    let payload = json!({"hooks":{"Stop":[{"hooks":[
        {"command":"run .codex/hooks/project_a/entrypoint.rs"},
        {"command":"run .codex/hooks/project_ab/entrypoint.rs"},
        {"commandWindows":"run C:\\repo\\.codex\\hooks\\project_a\\entrypoint.rs"},
        {"commandWindows":"run C:\\repo\\.codex\\hooks\\project_ab\\entrypoint.rs"}
    ]}]}});
    let result: Value = serde_json::from_slice(
        &remove_hook_registrations(
            &serde_json::to_vec(&payload).unwrap(),
            &[".codex/hooks/project_a".into()],
        )
        .unwrap(),
    )
    .unwrap();
    let handlers = result["hooks"]["Stop"][0]["hooks"].as_array().unwrap();
    assert_eq!(handlers.len(), 2);
    assert!(
        handlers
            .iter()
            .all(|handler| handler.to_string().contains("project_ab"))
    );
}

#[test]
fn retired_hook_directories_rollback_with_their_files() {
    let (root, mut plan) = transaction_fixture(true);
    let bundle = root.join(".codex/hooks/project_old");
    fs::create_dir_all(bundle.join("empty/subdirectory")).unwrap();
    let entry = bundle.join("entrypoint.rs");
    fs::write(&entry, b"fn main() {}\n").unwrap();
    plan.preservation_manifest = json!({"entries":[{"kind":"project-hook-bundle","target":".codex/hooks/project_old","disposition":"delete","before_sha256":tree_sha(&bundle).unwrap()}]});
    plan.deletes.push(entry.clone());
    plan.risk.push(SyncAction {
        id: "rebuild.remove:old-hook".into(),
        target: ".codex/hooks/project_old/entrypoint.rs".into(),
        operation: "delete".into(),
        risk: true,
        before_sha256: Some(sha_git(b"fn main() {}\n")),
        after_sha256: "sha256:deleted".into(),
    });
    plan.aggregate_fingerprint = plan_fingerprint(
        &plan.mode,
        &plan.current_version,
        &plan.safe,
        &plan.risk,
        &plan.preservation_manifest,
        &plan.generated_source_fingerprints,
    )
    .unwrap();
    let observer = |_: &Path, _: &Path| {
        assert!(!bundle.exists());
        Err("injected finalize failure".into())
    };
    assert!(
        apply_plan_internal(
            plan.clone(),
            &plan.aggregate_fingerprint,
            true,
            Some(&observer)
        )
        .unwrap_err()
        .contains("rolled back")
    );
    assert_eq!(fs::read(&entry).unwrap(), b"fn main() {}\n");
    assert!(bundle.join("empty/subdirectory").is_dir());
    apply_plan(plan.clone(), &plan.aggregate_fingerprint, true).unwrap();
    assert!(!bundle.exists());
    fs::remove_dir_all(root).unwrap();
}

struct NeverRun;
impl ProcessRunner for NeverRun {
    fn run(&self, _: &ProcessRequest) -> std::io::Result<crate::ProcessOutput> {
        panic!("no command may run before acquiring the project lock");
    }
}

#[test]
fn project_lock_blocks_apply_and_build_before_targets_change() {
    let (root, plan) = transaction_fixture(true);
    let lock = ProjectLock::acquire(&root).unwrap();
    let fingerprint = plan.aggregate_fingerprint.clone();
    assert!(
        apply_plan(plan.clone(), &fingerprint, false)
            .unwrap_err()
            .contains("lock unavailable")
    );
    assert!(
        build_generated_assets(&root, &json!({}), &NeverRun)
            .unwrap_err()
            .contains("lock unavailable")
    );
    assert_eq!(fs::read(root.join("managed.txt")).unwrap(), b"old\n");
    let (other, _) = transaction_fixture(true);
    drop(ProjectLock::acquire(&other).unwrap());
    drop(lock);
    apply_plan(plan, &fingerprint, false).unwrap();
    assert!(
        root.join(".runtime/bridgeforge-codex/project-sync.lock")
            .exists()
    );
    fs::remove_dir_all(root).unwrap();
    fs::remove_dir_all(other).unwrap();
}

#[test]
fn project_lock_covers_finalize_rollback_and_releases_on_failure() {
    let (root, plan) = transaction_fixture(true);
    let fingerprint = plan.aggregate_fingerprint.clone();
    let concurrent = plan.clone();
    let observer = |_: &Path, _: &Path| {
        let child = concurrent.clone();
        let result = std::thread::spawn(move || {
            let fingerprint = child.aggregate_fingerprint.clone();
            apply_plan(child, &fingerprint, false)
        })
        .join()
        .unwrap();
        assert!(result.unwrap_err().contains("lock unavailable"));
        Err("injected finalize failure".into())
    };
    assert!(
        apply_plan_internal(plan.clone(), &fingerprint, false, Some(&observer))
            .unwrap_err()
            .contains("rolled back")
    );
    assert_eq!(fs::read(root.join("managed.txt")).unwrap(), b"old\n");
    apply_plan(plan, &fingerprint, false).unwrap();
    fs::remove_dir_all(root).unwrap();
}

fn legacy_bytes() -> Vec<u8> {
    let hash = format!("sha256:{}", "a".repeat(64));
    serde_json::to_vec(
        &json!({"schema_version":1,"generated_asset_id":"codex.hooks",
        "platform":"windows-x86_64","source_tree_sha256":hash,"cargo_lock_sha256":hash,
        "build_recipe_sha256":hash,"self_test_sha256":hash,"binary_sha256":hash,
        "cargo_version":"cargo 1.94.1"}),
    )
    .unwrap()
}

#[test]
fn apply_rollback_preserves_external_receipt_recreated_after_retirement() {
    let (root, mut plan) = transaction_fixture(true);
    let legacy = root.join(LEGACY_RECEIPT);
    fs::create_dir_all(legacy.parent().unwrap()).unwrap();
    fs::write(&legacy, legacy_bytes()).unwrap();
    plan_legacy_receipt_retirement(&root, &mut plan.safe, &mut plan.deletes).unwrap();
    plan.aggregate_fingerprint = plan_fingerprint(
        &plan.mode,
        &plan.current_version,
        &plan.safe,
        &plan.risk,
        &plan.preservation_manifest,
        &plan.generated_source_fingerprints,
    )
    .unwrap();
    let fingerprint = plan.aggregate_fingerprint.clone();
    let observer = |_: &Path, _: &Path| {
        assert!(!legacy.exists());
        fs::write(&legacy, b"external receipt").unwrap();
        Err("injected after external receipt recreation".into())
    };
    let error = apply_plan_internal(plan, &fingerprint, false, Some(&observer)).unwrap_err();
    assert!(error.contains("rollback incomplete"), "{error}");
    assert!(error.contains("external state preserved"), "{error}");
    assert_eq!(fs::read(&legacy).unwrap(), b"external receipt");
    assert_eq!(fs::read(root.join("managed.txt")).unwrap(), b"old\n");
    assert_eq!(
        fs::read(root.join(".codex/.bridgeforge_codex_version")).unwrap(),
        b"1.0.0\n"
    );
    assert!(
        root.join(".runtime/bridgeforge-codex/project-sync.lock")
            .exists()
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn delete_rechecks_exact_original_receipt_bytes_before_removal() {
    let (root, _) = transaction_fixture(true);
    let legacy = root.join(LEGACY_RECEIPT);
    fs::create_dir_all(legacy.parent().unwrap()).unwrap();
    let before = legacy_bytes();
    fs::write(&legacy, b"external receipt").unwrap();
    let error = delete_unchanged_file(&legacy, Some(&before)).unwrap_err();
    assert!(error.contains("changed before removal"));
    assert_eq!(fs::read(&legacy).unwrap(), b"external receipt");
    fs::remove_file(&legacy).unwrap();
    fs::create_dir(&legacy).unwrap();
    let error = delete_unchanged_file(&legacy, Some(&before)).unwrap_err();
    assert!(error.contains("cannot read transaction target"));
    assert!(legacy.is_dir());
    fs::remove_dir_all(root).unwrap();
}

#[cfg(windows)]
#[test]
fn failed_atomic_replace_preserves_old_file_and_allows_transaction_rollback() {
    use std::os::windows::fs::OpenOptionsExt;
    let (root, plan) = transaction_fixture(true);
    let asset = root.join("managed.txt");
    // Allow reads, but deny delete/rename so the real Windows atomic replacement fails.
    let held = fs::OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(&asset)
        .unwrap();
    assert!(atomic_write(&asset, b"replacement\n").is_err());
    assert_eq!(
        transaction_file_state(&asset).unwrap(),
        Some(b"old\n".to_vec())
    );
    let fingerprint = plan.aggregate_fingerprint.clone();
    let error = apply_plan(plan.clone(), &fingerprint, false).unwrap_err();
    assert!(error.contains("transaction rolled back"), "{error}");
    assert!(!error.contains("rollback incomplete"), "{error}");
    assert_eq!(fs::read(&asset).unwrap(), b"old\n");
    assert_eq!(
        fs::read(root.join(".codex/managed-skeleton.json")).unwrap(),
        b"old-contract\n"
    );
    assert_eq!(
        fs::read(root.join(".codex/.bridgeforge_codex_version")).unwrap(),
        b"1.0.0\n"
    );
    drop(held);
    apply_plan(plan, &fingerprint, false).unwrap();
    assert_eq!(fs::read(&asset).unwrap(), b"new\n");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn apply_rollback_preserves_external_edit_to_written_target() {
    let (root, plan) = transaction_fixture(true);
    let fingerprint = plan.aggregate_fingerprint.clone();
    let observer = |root: &Path, _: &Path| {
        fs::write(root.join("managed.txt"), b"external managed edit").unwrap();
        Err("injected external edit".into())
    };
    let error = apply_plan_internal(plan, &fingerprint, false, Some(&observer)).unwrap_err();
    assert!(error.contains("rollback incomplete"));
    assert_eq!(
        fs::read(root.join("managed.txt")).unwrap(),
        b"external managed edit"
    );
    assert_eq!(
        fs::read(root.join(".codex/managed-skeleton.json")).unwrap(),
        b"old-contract\n"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn legacy_receipt_planning_is_read_only_and_apply_retirement_rolls_back() {
    let (root, mut plan) = transaction_fixture(true);
    let legacy = root.join(LEGACY_RECEIPT);
    fs::create_dir_all(legacy.parent().unwrap()).unwrap();
    let original = legacy_bytes();
    fs::write(&legacy, &original).unwrap();
    plan_legacy_receipt_retirement(&root, &mut plan.safe, &mut plan.deletes).unwrap();
    assert_eq!(fs::read(&legacy).unwrap(), original);
    plan.aggregate_fingerprint = plan_fingerprint(
        &plan.mode,
        &plan.current_version,
        &plan.safe,
        &plan.risk,
        &plan.preservation_manifest,
        &plan.generated_source_fingerprints,
    )
    .unwrap();
    let fingerprint = plan.aggregate_fingerprint.clone();
    let observer = |_: &Path, _: &Path| {
        assert!(!legacy.exists());
        Err("injected after receipt retirement".into())
    };
    assert!(
        apply_plan_internal(plan.clone(), &fingerprint, false, Some(&observer))
            .unwrap_err()
            .contains("rolled back")
    );
    assert_eq!(fs::read(&legacy).unwrap(), original);
    apply_plan(plan, &fingerprint, false).unwrap();
    assert!(!legacy.exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn unknown_legacy_receipt_blocks_apply_without_deletion() {
    let (root, plan) = transaction_fixture(true);
    let legacy = root.join(LEGACY_RECEIPT);
    fs::create_dir_all(legacy.parent().unwrap()).unwrap();
    fs::write(&legacy, b"{\"schema_version\":99}").unwrap();
    let fingerprint = plan.aggregate_fingerprint.clone();
    assert!(
        apply_plan(plan, &fingerprint, false)
            .unwrap_err()
            .contains("unknown legacy")
    );
    assert_eq!(fs::read(root.join("managed.txt")).unwrap(), b"old\n");
    assert_eq!(fs::read(&legacy).unwrap(), b"{\"schema_version\":99}");
    fs::remove_dir_all(root).unwrap();
}

fn transaction_fixture(valid_asset_hash: bool) -> (PathBuf, SyncPlan) {
    let token = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "bridgeforge-project-sync-{}-{token}",
        std::process::id()
    ));
    let contract_path = root.join(".codex/managed-skeleton.json");
    let stamp_path = root.join(".codex/.bridgeforge_codex_version");
    let asset_path = root.join("managed.txt");
    fs::create_dir_all(root.join(".codex")).unwrap();
    fs::write(&contract_path, b"old-contract\n").unwrap();
    fs::write(&stamp_path, b"1.0.0\n").unwrap();
    fs::write(&asset_path, b"old\n").unwrap();
    let current_sha = if valid_asset_hash {
        sha_git(b"new\n")
    } else {
        sha_git(b"different\n")
    };
    let contract = serde_json::to_vec_pretty(&json!({
        "schema_version": 4,
        "release_version": "2.0.0",
        "host": "codex",
        "stamp": ".codex/.bridgeforge_codex_version",
        "contract_target": ".codex/managed-skeleton.json",
        "assets": [{
            "id": "managed.asset",
            "source": "templates/managed.txt",
            "target": "managed.txt",
            "strategy": "whole",
            "current_sha256": current_sha,
        }],
        "baseline_model": "current-only",
        "generated_assets": [],
    }))
    .unwrap();
    let action = SyncAction {
        id: "managed.asset".into(),
        target: "managed.txt".into(),
        operation: "replace".into(),
        risk: false,
        before_sha256: Some(sha_git(b"old\n")),
        after_sha256: sha_git(b"new\n"),
    };
    let safe = vec![action];
    let preservation_manifest = json!({"entries": []});
    let generated_source_fingerprints = BTreeMap::new();
    let fingerprint = plan_fingerprint(
        &SyncMode::Update,
        "2.0.0",
        &safe,
        &[],
        &preservation_manifest,
        &generated_source_fingerprints,
    )
    .unwrap();
    let plan = SyncPlan {
        schema: 1,
        status: "planned".into(),
        readiness: "ready".into(),
        mode: SyncMode::Update,
        previous_version: Some("1.0.0".into()),
        current_version: "2.0.0".into(),
        safe,
        risk: Vec::new(),
        gaps: Vec::new(),
        blockers: Vec::new(),
        asset_migration: json!({}),
        preservation_manifest,
        confirmation_required: false,
        aggregate_fingerprint: fingerprint,
        project_root: root.clone(),
        writes: BTreeMap::from([
            (asset_path, b"new\n".to_vec()),
            (contract_path, contract),
            (stamp_path, b"2.0.0\n".to_vec()),
        ]),
        deletes: Vec::new(),
        generated_source_fingerprints,
    };
    (root, plan)
}

#[test]
fn json_merge_preserves_external_keys_and_adds_required_values() {
    let required = json!({"permissions": {"allow": ["Read"], "mode": "safe"}});
    let mut actual = json!({"permissions": {"allow": ["Custom"], "extra": true}});
    merge_json(&required, &mut actual);
    assert_eq!(actual["permissions"]["extra"], true);
    assert_eq!(actual["permissions"]["mode"], "safe");
    assert_eq!(actual["permissions"]["allow"], json!(["Custom", "Read"]));
}

#[test]
fn marker_replace_preserves_project_tail() {
    let current = b"x\nBEGIN\nold\nEND\ntail\n";
    let source = b"BEGIN\nnew\nEND\n";
    assert_eq!(
        marker_replace(current, source, "BEGIN", "END").unwrap(),
        b"x\nBEGIN\nnew\nEND\ntail\n"
    );
}

#[test]
fn stamp_is_last_and_finalize_failure_rolls_back_every_write() {
    let (root, plan) = transaction_fixture(true);
    let fingerprint = plan.aggregate_fingerprint.clone();
    let observer = |project_root: &Path, stamp_path: &Path| {
        assert_eq!(
            fs::read(project_root.join("managed.txt")).unwrap(),
            b"new\n"
        );
        assert_eq!(fs::read(stamp_path).unwrap(), b"1.0.0\n");
        Err("injected failure before version stamp".into())
    };
    let error = apply_plan_internal(plan, &fingerprint, false, Some(&observer)).unwrap_err();
    assert!(error.contains("transaction rolled back"));
    assert_eq!(fs::read(root.join("managed.txt")).unwrap(), b"old\n");
    assert_eq!(
        fs::read(root.join(".codex/managed-skeleton.json")).unwrap(),
        b"old-contract\n"
    );
    assert_eq!(
        fs::read(root.join(".codex/.bridgeforge_codex_version")).unwrap(),
        b"1.0.0\n"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn prospective_baseline_failure_rolls_back_before_stamp() {
    let (root, plan) = transaction_fixture(false);
    let fingerprint = plan.aggregate_fingerprint.clone();
    let error = apply_plan(plan, &fingerprint, false).unwrap_err();
    assert!(error.contains("managed asset drifted"));
    assert_eq!(fs::read(root.join("managed.txt")).unwrap(), b"old\n");
    assert_eq!(
        fs::read(root.join(".codex/managed-skeleton.json")).unwrap(),
        b"old-contract\n"
    );
    assert_eq!(
        fs::read(root.join(".codex/.bridgeforge_codex_version")).unwrap(),
        b"1.0.0\n"
    );
    fs::remove_dir_all(root).unwrap();
}
