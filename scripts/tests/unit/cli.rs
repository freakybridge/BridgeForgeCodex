use super::*;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn development_capabilities_and_mutation_flags_are_explicit() {
    let identity=self_test().receipt.unwrap();
    for capability in ["package-development-v1","package-stability-v1","git-sync-development-closure-v1"] {
        assert!(identity["capabilities"].as_array().unwrap().iter().any(|v|v==capability));
    }
    for flags in [vec!["--apply","--dry-run"],vec!["--recover","--check"],vec!["--preview","--apply"],vec![]] {
        let mut args=vec!["develop".into()];
        args.extend(flags.into_iter().map(String::from));
        let result=run(&args);
        assert_eq!(result.code,EXIT_BLOCKED);
        assert!(result.stderr.contains("choose exactly one"),"{}",result.stderr);
    }
}

#[test]
fn todo_contract_query_is_read_only_and_invalid_table_blocks_archive() {
    let root = std::env::temp_dir().join(format!(
        "bf-cli-todo-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(root.join("doc/0_architecture")).unwrap();
    let args = vec![
        "check".into(),
        "todo".into(),
        "--root".into(),
        root.display().to_string(),
        "--contract".into(),
    ];
    let result = run(&args);
    assert_eq!(result.code, 0);
    assert_eq!(
        result.receipt.unwrap()["columns"].as_array().unwrap().len(),
        6
    );
    let file = root.join("doc/0_architecture/TODO-INDEX.md");
    assert!(!file.exists(), "contract query must not initialize a table");
    fs::write(&file, "damaged table\n").unwrap();
    let before = fs::read(&file).unwrap();
    let result = run(&[
        "check".into(),
        "todo".into(),
        "--root".into(),
        root.display().to_string(),
    ]);
    assert_eq!(result.code, EXIT_BLOCKED);
    assert!(
        !result.receipt.unwrap()["errors"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let result = run(&[
        "archive-scan".into(),
        "--root".into(),
        root.display().to_string(),
    ]);
    assert_eq!(result.code, EXIT_BLOCKED);
    assert_eq!(fs::read(&file).unwrap(), before);
    let _ = fs::remove_dir_all(root);
}

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
fn git_sync_preflight_is_capability_gated_and_mutually_exclusive() {
    let capabilities=self_test().receipt.unwrap()["capabilities"].as_array().unwrap().clone();
    assert!(capabilities.iter().any(|v|v=="git-sync-preflight-v1"));
    assert!(capabilities.iter().any(|v|v=="git-sync-phase-timing-v1"));
    let root=std::env::temp_dir().join(format!("bf-cli-preflight-{}-{}",std::process::id(),SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
    fs::create_dir_all(root.join(".codex")).unwrap();
    for mode in ["--release","--setup-apply","--prepare-release"] {
        let result=run(&["git-sync".into(),"--root".into(),root.display().to_string(),"--preflight".into(),mode.into()]);
        assert_eq!(result.code,EXIT_BLOCKED);assert!(result.stderr.contains("mutually exclusive"));
        assert!(!root.join(".git").exists());assert!(!root.join(".runtime").exists());
    }
    fs::remove_dir_all(root).unwrap();
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
fn ordinary_sync_does_not_require_release_choice_but_release_does() {
    let root=std::env::temp_dir().join(format!("bf-cli-first-use-{}-{}",std::process::id(),SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
    fs::create_dir_all(&root).unwrap();fs::write(root.join("app.py"),"pass\n").unwrap();
    for args in [vec!["init"],vec!["config","user.name","Fixture"],vec!["config","user.email","fixture@example.invalid"],vec!["config","core.hooksPath",".git/hooks"],vec!["add","."],vec!["commit","-m","chore: initial"]] {
        let output=std::process::Command::new("git").args(&args).current_dir(&root).output().unwrap();assert!(output.status.success(),"{}",String::from_utf8_lossy(&output.stderr));
    }
    let index=fs::read(root.join(".git/index")).unwrap();
    for release in [false,true] {
        let mut args=vec!["git-sync".into(),"--root".into(),root.display().to_string(),"--message".into(),"chore: test".into()];
        if release {args.push("--release".into());}
        let output=run(&args);assert_eq!(output.code,EXIT_BLOCKED);
        if release{assert_eq!(output.receipt.unwrap()["status"],"configuration-required");}
        else{assert_eq!(output.receipt.unwrap()["status"],"blocked");assert!(output.stderr.contains("no push target"),"{}",output.stderr);}
        assert_eq!(fs::read(root.join(".git/index")).unwrap(),index);assert!(!root.join(".git/FETCH_HEAD").exists());assert!(!root.join(".runtime").exists());
    }
    fs::remove_dir_all(root).unwrap();
}
