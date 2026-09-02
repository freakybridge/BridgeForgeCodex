use super::*;

#[test]
fn hook_asset_ids_are_stable() {
    assert_eq!(
        hook_asset_id("crates/core/src/lib.rs"),
        "codex.hooks.crates-core-src-lib-rs"
    );
}

#[test]
fn canonical_hash_ignores_object_insertion_order() {
    assert_eq!(
        canonical_sha(&json!({"b": 1, "a": 2})).unwrap(),
        canonical_sha(&json!({"a": 2, "b": 1})).unwrap()
    );
}
