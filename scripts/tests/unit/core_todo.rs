use super::*;
use std::sync::atomic::{AtomicU64, Ordering};
static ID: AtomicU64 = AtomicU64::new(0);

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let root = test_artifacts::root(env!("CARGO_MANIFEST_DIR"), "todo").join(format!(
            "bf-todo-{}-{}",
            std::process::id(),
            ID.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir_all(root.join("doc/0_architecture")).unwrap();
        fs::create_dir_all(root.join("doc/1_delivery/demo")).unwrap();
        fs::write(
            root.join("doc/1_delivery/demo/requirements_test.md"),
            "---\nlifecycle: completed\nvalidation_status: verified\n---\nTODO-001 用户验收完成\n",
        )
        .unwrap();
        Self(root)
    }
    fn write(&self, row: &str) {
        fs::write(self.0.join("doc/0_architecture/TODO-INDEX.md"), format!("# 项目待办总表\n\n| 编号 | 日期 | 内容 | 状态 | 归属 | 关联文档 |\n|---|---|---|---|---|---|\n{row}\n")).unwrap();
    }
    fn row(state: &str, origin: &str, refs: &str) -> String {
        format!("| TODO-001 | 2026-10-11 | 独立事项 | {state} | {origin} | {refs} |")
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
const SOURCE: &str = "[来源：需求](../1_delivery/demo/requirements_test.md)";

#[test]
fn missing_table_is_optional_and_contract_checks_do_not_write() {
    let f = Fixture::new();
    let file = f.0.join("doc/0_architecture/TODO-INDEX.md");
    assert!(inspect(&f.0).errors.is_empty());
    assert!(!file.exists());
    f.write(&Fixture::row("待验证", "后续事项", SOURCE));
    let before = fs::read(&file).unwrap();
    let report = inspect(&f.0);
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    assert_eq!(report.entries.len(), 1);
    assert_eq!(fs::read(&file).unwrap(), before);
    assert!(
        blockers(&report, &f.0.join("doc/1_delivery/demo"))
            .unwrap()
            .is_empty()
    );
}

#[test]
fn original_open_todo_blocks_but_independent_followup_does_not() {
    let f = Fixture::new();
    f.write(&Fixture::row("待验证", "原需求", SOURCE));
    assert_eq!(
        blockers(&inspect(&f.0), &f.0.join("doc/1_delivery/demo")).unwrap(),
        vec!["TODO-001"]
    );
    f.write(&Fixture::row("待验证", "后续事项", SOURCE));
    assert!(
        blockers(&inspect(&f.0), &f.0.join("doc/1_delivery/demo"))
            .unwrap()
            .is_empty()
    );
}

#[test]
fn completion_needs_this_todo_evidence_and_not_parent_acceptance() {
    let f = Fixture::new();
    f.write(&Fixture::row("已完成", "后续事项", SOURCE));
    assert!(
        inspect(&f.0)
            .errors
            .iter()
            .any(|e| e.code == "missing-todo-closure-evidence")
    );
    f.write(&Fixture::row(
        "已完成",
        "原需求",
        "[验收：本事项](../1_delivery/demo/requirements_test.md)",
    ));
    assert!(inspect(&f.0).errors.is_empty());
    fs::write(
        f.0.join("doc/1_delivery/demo/requirements_test.md"),
        "---\nlifecycle: completed\nvalidation_status: verified\n---\n只验收原交付，无本事项编号\n",
    )
    .unwrap();
    assert!(
        inspect(&f.0)
            .errors
            .iter()
            .any(|e| e.code == "missing-todo-closure-evidence")
    );
}

#[test]
fn cancellation_keeps_record_and_requires_explicit_evidence() {
    let f = Fixture::new();
    f.write(&Fixture::row(
        "已取消",
        "原需求",
        "[取消：决定](../1_delivery/demo/requirements_test.md)",
    ));
    assert!(!inspect(&f.0).errors.is_empty());
    fs::write(
        f.0.join("doc/1_delivery/demo/requirements_test.md"),
        "用户明确取消 TODO-001，原因：替代方案已采用。\n",
    )
    .unwrap();
    assert!(inspect(&f.0).errors.is_empty());
    assert_eq!(inspect(&f.0).entries[0].status, "已取消");
}

#[test]
fn malformed_state_origin_id_and_missing_sources_fail_closed() {
    let f = Fixture::new();
    for row in [
        Fixture::row("待执行／待验证", "后续事项", SOURCE),
        Fixture::row("待执行", "unknown", SOURCE),
        Fixture::row("待执行", "后续事项", "暂无来源"),
        Fixture::row("待执行", "原需求", "[需求](../1_delivery/absent.md)"),
        Fixture::row("待执行", "原需求", "[需求](../../../outside.md)"),
        Fixture::row("待执行", "原需求", SOURCE).replace("TODO-001", "TODO-000"),
    ] {
        f.write(&row);
        assert!(!inspect(&f.0).errors.is_empty(), "{row}");
    }
    let row = Fixture::row("待执行", "原需求", SOURCE);
    f.write(&format!("{row}\n{row}"));
    assert!(
        inspect(&f.0)
            .errors
            .iter()
            .any(|e| e.code == "invalid-todo-id")
    );
    fs::write(
        f.0.join("doc/0_architecture/TODO-INDEX.md"),
        "| 编号 | 日期 | 内容 | 状态 | 关联文档 |\n|---|---|---|---|---|\n",
    )
    .unwrap();
    assert!(
        inspect(&f.0)
            .errors
            .iter()
            .any(|e| e.code == "legacy-todo-format")
    );
}

#[test]
fn escaped_pipe_and_contract_states_are_accepted_without_guessing() {
    let f = Fixture::new();
    for (state, _) in contract()["states"].as_object().unwrap() {
        if matches!(state.as_str(), "已完成" | "已取消") {
            continue;
        }
        f.write(&Fixture::row(state, "原需求", SOURCE).replace("独立事项", "内容含 \\| 分隔符"));
        let report = inspect(&f.0);
        assert!(report.errors.is_empty(), "{state}: {:?}", report.errors);
        assert_eq!(report.entries[0].content, "内容含 | 分隔符");
    }
}

#[test]
fn missing_initial_pipe_preserves_original_blocker_and_truncated_row_fails_closed() {
    let f = Fixture::new();
    f.write(Fixture::row("待验证", "原需求", SOURCE).trim_start_matches("| "));
    let report = inspect(&f.0);
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    assert_eq!(
        blockers(&report, &f.0.join("doc/1_delivery/demo")).unwrap(),
        vec!["TODO-001"]
    );
    assert!(crate::archive_scan::scan(&f.0).unwrap().is_empty());
    f.write("TODO-001 missing columns");
    assert!(
        inspect(&f.0)
            .errors
            .iter()
            .any(|e| e.code == "invalid-todo-row")
    );
    assert!(crate::archive_scan::scan(&f.0).is_err());
}

#[test]
fn acceptance_and_cancellation_reject_other_id_prefixes() {
    let f = Fixture::new();
    for (state, label) in [("已完成", "验收"), ("已取消", "取消")] {
        f.write(&Fixture::row(
            state,
            "原需求",
            &format!("[{label}：记录](../1_delivery/demo/requirements_test.md)"),
        ));
        for other in ["TODO-0010", "TODO-0011", "TODO-001-old"] {
            fs::write(f.0.join("doc/1_delivery/demo/requirements_test.md"), format!("---\nlifecycle: completed\nvalidation_status: verified\n---\n用户验收/取消 {other}，不是当前事项。\n")).unwrap();
            assert!(
                inspect(&f.0)
                    .errors
                    .iter()
                    .any(|e| e.code == "missing-todo-closure-evidence"),
                "{state}: {other}"
            );
        }
        fs::write(f.0.join("doc/1_delivery/demo/requirements_test.md"), "---\nlifecycle: completed\nvalidation_status: verified\n---\n用户明确验收/取消TODO-001。\n").unwrap();
        assert!(
            inspect(&f.0).errors.is_empty(),
            "{state}: exact ID should work next to Chinese punctuation"
        );
    }
}

#[allow(dead_code)]
#[path = "../support/artifact_paths.rs"]
mod test_artifacts;
