use super::*;

#[test]
fn knowledge_documentation_has_a_fixed_directory_boundary() {
    assert!(valid_target_type(
        "documentation",
        "doc/5_project_knowledgebase/topic/note.md"
    ));
    assert!(!valid_target_type(
        "documentation",
        "doc/5_project_knowledgebase_extra/note.md"
    ));
    assert!(!valid_target_type("documentation", "doc/6_other/note.md"));
    assert!(!valid_target_type(
        "delivery",
        "doc/5_project_knowledgebase/note.md"
    ));
}

#[test]
fn rust_hook_and_test_targets_are_supported() {
    assert!(valid_target_type("hook-registration", ".codex/hooks.json"));
    assert!(valid_target_type(
        "hook-registration",
        ".codex/project-hooks.json"
    ));
    assert!(!valid_target_type(
        "hook-registration",
        ".codex/other-hooks.json"
    ));
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
