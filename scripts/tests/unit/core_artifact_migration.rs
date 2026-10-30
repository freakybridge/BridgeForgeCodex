use super::*;

#[test]
fn legacy_test_receipts_relocate_exact_known_files_and_preserve_unknowns() {
    let root = test_artifacts::root(env!("CARGO_MANIFEST_DIR"), "artifact-migration")
        .join(format!("{}", std::process::id()));
    fs::create_dir_all(root.join(".runtime/test_receipts/payload_samples")).unwrap();
    let record = root.join(".runtime/test_receipts/receipts.jsonl");
    let sample = root.join(".runtime/test_receipts/payload_samples/2026-10-30T123456.json");
    let bytes=br#"{"cmd":"cargo test","kind":"test","ts":"2026-10-30T12:34:56","sha1":"012345abcdef","exit_code":0}"#;
    fs::write(&record, bytes).unwrap();
    fs::write(&sample, b"{\"tool_name\":\"exec_command\"}").unwrap();
    let found = legacy_test_outputs(&root).unwrap();
    assert_eq!(found.len(), 2);
    assert_eq!(found[&record], bytes);
    assert_eq!(
        relocated_test_output(&root, &record).unwrap(),
        root.join(".runtime/tests/legacy-test-receipts/receipts.jsonl")
    );
    let unknown = root.join(".runtime/test_receipts/user-notes.txt");
    fs::write(&unknown, b"keep").unwrap();
    assert!(
        legacy_test_outputs(&root)
            .unwrap_err()
            .contains("unknown legacy test output")
    );
    assert_eq!(fs::read(&unknown).unwrap(), b"keep");
    assert_eq!(fs::read(&record).unwrap(), bytes);
    fs::remove_file(unknown).unwrap();
    fs::write(&record, b"{\"unknown\":true}").unwrap();
    assert!(
        legacy_test_outputs(&root)
            .unwrap_err()
            .contains("unknown legacy test receipt")
    );
    fs::remove_dir_all(root).unwrap();
}

#[allow(dead_code)]
#[path = "../support/artifact_paths.rs"]
mod test_artifacts;

fn runtime_fixture(label: &str) -> PathBuf {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = test_artifacts::root(env!("CARGO_MANIFEST_DIR"), "legacy-runtime-preservation")
        .join(format!("{}-{nonce}-{label}", std::process::id()));
    let directory = root.join(".codex/bin");
    fs::create_dir_all(&directory).unwrap();
    let profiles: Vec<Value> = serde_json::from_str(include_str!(
        "../../../source/core/src/legacy_artifact_receipts.json"
    ))
    .unwrap();
    let mut receipt = profiles
        .into_iter()
        .find(|p| p["id"] == "codex.hooks")
        .unwrap();
    receipt["schema_version"] = serde_json::json!(2);
    receipt["generated_asset_id"] = serde_json::json!("codex.hooks");
    receipt["platform"] = serde_json::json!(crate::build_receipt::platform_key().unwrap());
    receipt["binary_sha256"] = serde_json::json!(format!("sha256:{:x}", Sha256::digest(b"public")));
    fs::write(
        directory.join(format!("bridgeforge-hook{}", std::env::consts::EXE_SUFFIX)),
        b"public",
    )
    .unwrap();
    fs::write(
        directory.join("build-receipt-hook.json"),
        serde_json::to_vec(&receipt).unwrap(),
    )
    .unwrap();
    fs::write(directory.join(".gitignore"), b"*\r\n!.gitignore\r\n").unwrap();
    root
}

#[test]
fn legacy_runtime_retires_only_public_files_and_keeps_private_files_and_ignore_policy() {
    let root = runtime_fixture("mixed");
    let directory = root.join(".codex/bin");
    assert_eq!(legacy_runtime(&root).unwrap().len(), 3);
    let private = [
        ("project_demo.exe", b"private executable".as_slice()),
        (
            "build-receipt-project-demo.json",
            b"not a public receipt".as_slice(),
        ),
        ("user-data.txt", b"independent data".as_slice()),
    ];
    for (name, bytes) in private {
        fs::write(directory.join(name), bytes).unwrap();
    }
    let ignore = b"*\r\n!.gitignore\r\n# private policy\r\n";
    fs::write(directory.join(".gitignore"), ignore).unwrap();
    let retired = legacy_runtime(&root).unwrap();
    assert_eq!(retired.len(), 2);
    for path in retired.keys() {
        fs::remove_file(path).unwrap();
    }
    assert!(legacy_runtime(&root).unwrap().is_empty());
    for (name, bytes) in private {
        assert_eq!(fs::read(directory.join(name)).unwrap(), bytes);
    }
    assert_eq!(fs::read(directory.join(".gitignore")).unwrap(), ignore);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn legacy_runtime_preservation_does_not_bypass_public_ownership_or_directory_guards() {
    let root = runtime_fixture("guards");
    let directory = root.join(".codex/bin");
    fs::write(directory.join("private.txt"), b"keep").unwrap();
    let binary = directory.join(format!("bridgeforge-hook{}", std::env::consts::EXE_SUFFIX));
    fs::write(&binary, b"corrupt public bytes").unwrap();
    assert!(
        legacy_runtime(&root)
            .unwrap_err()
            .contains("ownership is unknown")
    );
    fs::write(&binary, b"public").unwrap();
    fs::create_dir(directory.join("unknown-directory")).unwrap();
    assert!(
        legacy_runtime(&root)
            .unwrap_err()
            .contains("unsafe legacy runtime entry")
    );
    fs::remove_dir(directory.join("unknown-directory")).unwrap();
    fs::remove_file(directory.join("build-receipt-hook.json")).unwrap();
    assert!(
        legacy_runtime(&root)
            .unwrap_err()
            .contains("pair is incomplete")
    );
    assert_eq!(fs::read(directory.join("private.txt")).unwrap(), b"keep");
    fs::remove_dir_all(root).unwrap();
}
