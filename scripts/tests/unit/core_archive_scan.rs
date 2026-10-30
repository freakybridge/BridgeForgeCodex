use super::*;

#[test]
fn active_lifecycle_wins() {
    let mut root = test_artifacts::root(env!("CARGO_MANIFEST_DIR"), "archive-scan");
    root.push(format!("bridgeforge-archive-{}", std::process::id()));
    let topic = root.join("doc/1_delivery/demo");
    fs::create_dir_all(&topic).unwrap();
    fs::write(topic.join("done.md"), "---\nlifecycle: completed\n---\n").unwrap();
    fs::write(topic.join("active.md"), "---\nlifecycle: active\n---\n").unwrap();
    assert!(scan(&root).unwrap().is_empty());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn formal_cards_drive_archive_not_historical_reports_and_todo_ownership_matters() {
    let root = test_artifacts::root(env!("CARGO_MANIFEST_DIR"), "archive-scan")
        .join(format!("bf-archive-formal-{}", std::process::id()));
    let topic = root.join("doc/1_delivery/demo");
    fs::create_dir_all(&topic).unwrap();
    fs::create_dir_all(root.join("doc/0_architecture")).unwrap();
    fs::write(
        topic.join("requirements_test.md"),
        "---\nlifecycle: completed\nvalidation_status: verified\n---\nTODO-001 用户验收完成\n",
    )
    .unwrap();
    fs::write(
        topic.join("scan.md"),
        "---\nlifecycle: active\n---\n历史扫描状态\n",
    )
    .unwrap();
    assert_eq!(scan(&root).unwrap().len(), 1);
    let todo = root.join("doc/0_architecture/TODO-INDEX.md");
    let table = "| 编号 | 日期 | 内容 | 状态 | 归属 | 关联文档 |\n|---|---|---|---|---|---|\n| TODO-001 | 2026-10-11 | 验证 | 待验证 | 原需求 | [需求](../1_delivery/demo/requirements_test.md) |\n";
    fs::write(&todo, table).unwrap();
    assert!(scan(&root).unwrap().is_empty());
    fs::write(&todo, table.replace("原需求", "后续事项")).unwrap();
    let before = fs::read(&todo).unwrap();
    assert_eq!(scan(&root).unwrap().len(), 1);
    assert_eq!(fs::read(&todo).unwrap(), before);
    fs::write(
        topic.join("requirements_test.md"),
        "---\nlifecycle: active\nvalidation_status: awaiting_validation\n---\n",
    )
    .unwrap();
    assert!(
        scan(&root).unwrap().is_empty(),
        "original acceptance cannot be bypassed by a follow-up TODO"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn milestone_bug_forms_and_legacy_evidence_have_one_policy() {
    let root = test_artifacts::root(env!("CARGO_MANIFEST_DIR"), "archive-scan")
        .join(format!("bf-archive-layout-{}", std::process::id()));
    let topic = root.join("doc/1_delivery/M1/demo");
    fs::create_dir_all(&topic).unwrap();
    fs::create_dir_all(root.join("doc/2_bugs/BUG-directory")).unwrap();
    fs::write(
        root.join("doc/README.md"),
        "---\ndelivery_layout: milestone\n---\n",
    )
    .unwrap();
    let retired = "---\nlifecycle: 'completed'\nvalidation_status: verified\n---\n";
    fs::write(topic.join("requirements_demo.md"), retired).unwrap();
    fs::write(root.join("doc/2_bugs/BUG-file.md"), retired).unwrap();
    fs::write(
        root.join("doc/2_bugs/BUG-directory/README.md"),
        "状态：已解决\n",
    )
    .unwrap();
    let found = scan(&root).unwrap();
    assert_eq!(found.len(), 3);
    assert!(
        found
            .iter()
            .any(|c| c.target == root.join("doc/4_archive/delivery/M1/demo"))
    );
    assert!(
        found
            .iter()
            .any(|c| c.evidence == "legacy" && c.lifecycle == "unclassified")
    );
    fs::write(
        topic.join("requirements_demo.md"),
        "---\nlifecycle: completed\nvalidation_status: awaiting_validation\n---\n",
    )
    .unwrap();
    assert_eq!(scan(&root).unwrap().len(), 2);
    fs::write(
        root.join("doc/2_bugs/BUG-file.md"),
        "---\nlifecycle: completed\nlifecycle: active\n---\n",
    )
    .unwrap();
    assert!(
        scan(&root).is_err(),
        "duplicate metadata must not become a candidate"
    );
    let _ = fs::remove_dir_all(root);
}

#[allow(dead_code)]
#[path = "../support/artifact_paths.rs"]
mod test_artifacts;
