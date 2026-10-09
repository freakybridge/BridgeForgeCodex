use super::*;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn user_instruction_staging_rejects_preview_flags_before_writing() {
    for flag in ["--check", "--dry-run"] {
        let args = vec!["user-agents-stage".into(), flag.into()];
        let outcome = run(&args);
        assert_eq!(outcome.code, EXIT_BLOCKED);
        assert!(outcome.stderr.contains("preview flags are not supported"));
    }
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
