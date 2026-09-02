use super::*;

#[test]
fn blocked_outcome_uses_stable_exit_code() {
    let outcome = CommandOutcome::blocked("no");
    assert_eq!(outcome.code, EXIT_BLOCKED);
    assert_eq!(outcome.stderr, "no");
}

#[test]
fn project_context_never_exposes_python_runtime() {
    let context = ProjectContext::discover(None).expect("context");
    assert!(context.root().is_absolute());
    assert_eq!(context.codex_root(), context.root().join(".codex"));
}
