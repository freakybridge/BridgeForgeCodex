use super::*;

#[test]
fn rust_hook_and_test_targets_are_supported() {
    assert!(valid_target_type(
        "hook",
        ".codex/hooks/project_x/entrypoint.rs"
    ));
    assert!(valid_target_type("test", "scripts/tests/example.rs"));
    assert!(!valid_target_type(
        "hook",
        ".codex/hooks/project_x/entrypoint.py"
    ));
}
