use super::*;
fn identity(id: &str, path: &str) -> Identity {
    Identity {
        id: id.into(),
        manifest: path.into(),
        name: id.into(),
        language: Language::Python,
        workspace: None,
    }
}
fn package(p: &Identity) -> NativePackage {
    NativePackage {
        manifest: p.manifest.clone(),
        name: p.name.clone(),
        version: "1.2.3".into(),
        language: p.language.clone(),
        workspace: "pyproject.toml".into(),
        source_roots: vec![],
    }
}
fn transition(op: &str, from: Vec<Identity>, to: Vec<Identity>) -> Transition {
    Transition {
        id: "event-1".into(),
        base: "a".repeat(40),
        operation: op.into(),
        compatibility: "compatible".into(),
        from,
        to,
    }
}
#[test]
fn lifecycle_move_preserves_identity_but_never_hides_manual_versions() {
    let a = identity("a", "old/pyproject.toml");
    let b = identity("a", "new/pyproject.toml");
    let t = transition("move", vec![a.clone()], vec![b.clone()]);
    t.validate().unwrap();
    let before = BTreeMap::from([("a".into(), package(&a))]);
    let mut after = BTreeMap::from([("a".into(), package(&b))]);
    let mut used = BTreeSet::new();
    validate_step(&before, &after, &[t.clone()], &mut used).unwrap();
    assert!(used.contains("event-1"));
    after.get_mut("a").unwrap().version = "9.0.0".into();
    assert!(
        validate_step(&before, &after, &[t], &mut BTreeSet::new())
            .unwrap_err()
            .contains("version")
    );
}
#[test]
fn lifecycle_missing_declaration_blocks_removal() {
    let a = identity("a", "a/pyproject.toml");
    assert!(
        validate_step(
            &BTreeMap::from([("a".into(), package(&a))]),
            &BTreeMap::new(),
            &[],
            &mut BTreeSet::new()
        )
        .unwrap_err()
        .contains("removed")
    );
}
#[test]
fn lifecycle_split_and_merge_require_explicit_cardinality() {
    let a = identity("a", "a/pyproject.toml");
    let b = identity("b", "b/pyproject.toml");
    let c = identity("c", "c/pyproject.toml");
    transition("split", vec![a.clone()], vec![b.clone(), c.clone()])
        .validate()
        .unwrap();
    transition("merge", vec![b, c], vec![a.clone()])
        .validate()
        .unwrap();
    assert!(
        transition("split", vec![a.clone()], vec![a])
            .validate()
            .is_err()
    );
}
#[test]
fn lifecycle_lineage_routes_old_events_into_split_outputs() {
    let a = identity("a", "a/pyproject.toml");
    let b = identity("b", "b/pyproject.toml");
    let c = identity("c", "c/pyproject.toml");
    let t = transition("split", vec![a], vec![b, c]);
    assert_eq!(
        next_ids("a", &[t]),
        BTreeSet::from(["b".into(), "c".into()])
    );
}
#[test]
fn lifecycle_removal_has_no_synthetic_successor() {
    let a = identity("a", "a/pyproject.toml");
    assert!(next_ids("a", &[transition("remove", vec![a], vec![])]).is_empty());
}
#[test]
fn lifecycle_observed_order_supports_consecutive_retaining_splits() {
    let a = identity("a", "a/pyproject.toml");
    let c = identity("c", "c/pyproject.toml");
    let d = identity("d", "d/pyproject.toml");
    let t1 = transition("split", vec![a.clone()], vec![a.clone(), c.clone()]);
    let mut t2 = transition("split", vec![a.clone()], vec![a.clone(), d.clone()]);
    t2.id = "event-2".into();
    let before = BTreeMap::from([("a".into(), package(&a))]);
    let middle = BTreeMap::from([("a".into(), package(&a)), ("c".into(), package(&c))]);
    let after = BTreeMap::from([
        ("a".into(), package(&a)),
        ("c".into(), package(&c)),
        ("d".into(), package(&d)),
    ]);
    let mut used = BTreeSet::new();
    let declarations = [t2, t1];
    assert_eq!(
        validate_step(&before, &middle, &declarations, &mut used).unwrap(),
        vec!["event-1"]
    );
    assert_eq!(
        validate_step(&middle, &after, &declarations, &mut used).unwrap(),
        vec!["event-2"]
    );
}
