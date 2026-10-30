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
