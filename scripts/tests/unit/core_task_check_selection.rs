//! Permission and argument boundaries for task-selected final-candidate checks.
use super::*;

fn command(args: &[&str]) -> CheckCommand {
    serde_json::from_value(json!({"id":"unit","program":"cargo","args":args})).unwrap()
}

#[test]
fn task_selection_allows_only_appended_cargo_test_filters() {
    for (registered, selected) in [
        (
            vec!["test", "--locked", "--manifest-path", "Cargo.toml", "--lib"],
            vec![
                "test",
                "--locked",
                "--manifest-path",
                "Cargo.toml",
                "--lib",
                "task_filter",
            ],
        ),
        (
            vec![
                "test",
                "--package",
                "library",
                "--target-dir",
                ".build/target",
                "--",
                "--test-threads=1",
            ],
            vec![
                "test",
                "--package",
                "library",
                "--target-dir",
                ".build/target",
                "task_filter",
                "--",
                "--test-threads=1",
            ],
        ),
    ] {
        assert!(task_arguments_allowed(
            &command(&registered),
            &command(&registered)
        ));
        assert!(task_arguments_allowed(
            &command(&registered),
            &command(&selected)
        ));
    }
}

#[test]
fn task_selection_cannot_split_option_values_or_replace_flags() {
    for (registered, selected) in [
        (
            vec!["test", "--manifest-path", "Cargo.toml", "--lib"],
            vec![
                "test",
                "--manifest-path",
                "other/Cargo.toml",
                "Cargo.toml",
                "--lib",
            ],
        ),
        (
            vec!["test", "--package", "library"],
            vec!["test", "--package", "other", "library"],
        ),
        (
            vec!["test", "--target-dir", ".build/target"],
            vec!["test", "--target-dir", "other", ".build/target"],
        ),
        (
            vec!["test", "--locked", "--lib"],
            vec!["test", "--lib", "task_filter"],
        ),
        (
            vec!["test", "--lib", "--", "--test-threads=1"],
            vec!["test", "--lib", "task_filter", "--", "--ignored"],
        ),
        (vec!["test", "--lib"], vec!["test", "--lib", "--help"]),
    ] {
        assert!(
            !task_arguments_allowed(&command(&registered), &command(&selected)),
            "{selected:?}"
        );
    }
}

#[test]
fn task_selection_keeps_registered_input_output_timeout_and_reuse_boundaries() {
    let policy: Policy = serde_json::from_value(json!({"checks":[{
        "id":"unit","program":"cargo","args":["test","--lib"],
        "inputs":["test.rs"],"artifacts":[".runtime/result.bin"],"timeout_seconds":300
    }]}))
    .unwrap();
    assert!(validate_task_selection("a", &policy, &[]).is_err());
    assert!(validate_task_selection("a", &policy, &policy.checks).is_ok());
    for mutation in [
        "program",
        "inputs",
        "artifacts",
        "timeout",
        "reuse",
        "kind",
        "id",
    ] {
        let mut selected = policy.checks.clone();
        match mutation {
            "program" => selected[0].program = "other".into(),
            "inputs" => selected[0].inputs.clear(),
            "artifacts" => selected[0].artifacts.clear(),
            "timeout" => selected[0].timeout_seconds = 301,
            "reuse" => selected[0].reusable = true,
            "kind" => selected[0].kind = "core".into(),
            _ => selected[0].id = "unregistered".into(),
        }
        assert!(
            validate_task_selection("a", &policy, &selected).is_err(),
            "{mutation}"
        );
    }
}

#[test]
fn task_selection_rejects_successful_cargo_runs_with_zero_executed_tests() {
    struct CargoResult(usize);
    impl ProcessRunner for CargoResult {
        fn run(&self, _: &ProcessRequest) -> std::io::Result<crate::ProcessOutput> {
            Ok(crate::ProcessOutput {
                code: 0,
                timed_out: false,
                stderr: Vec::new(),
                stdout: format!(
                    "test result: ok. {} passed; 0 failed; 0 ignored; 0 measured; 8 filtered out\n",
                    self.0
                )
                .into_bytes(),
            })
        }
    }
    let check = command(&["test", "--lib", "specific_test"]);
    let root = test_artifacts::root(env!("CARGO_MANIFEST_DIR"), "task-check-selection");
    let error = run_check(&root, &check, None, true, &CargoResult(0), None).unwrap_err();
    assert!(error.contains("executed zero tests"));
    assert!(run_check(&root, &check, None, true, &CargoResult(1), None).is_ok());
    for stdout in [
        b"".as_slice(),
        b"specific_test: test\n1 test, 0 benchmarks\n",
        b"{\"passed\":1}\n",
        b"test result: ok. unknown passed; 0 failed;\n",
    ] {
        assert!(verify_cargo_execution(&check, stdout, true).is_err());
    }
    // Unfiltered registered custom harnesses retain their existing exit-code contract.
    assert!(
        verify_cargo_execution(
            &command(&["test", "--lib"]),
            b"custom harness passed\n",
            false
        )
        .is_ok()
    );
    for flag in ["--list", "--help", "--no-run"] {
        assert!(verify_cargo_execution(&command(&["test", "--", flag]), b"", false).is_err());
    }
}

#[allow(dead_code)]
#[path = "../support/artifact_paths.rs"]
mod test_artifacts;
