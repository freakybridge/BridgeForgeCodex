use super::*;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn retirement_only_plans_verified_files_and_preserves_unknown_and_project_hooks() {
    let root = std::env::temp_dir().join(format!(
        "bf-retire-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(root.join(".codex/hooks/crates/old/src")).unwrap();
    fs::create_dir_all(root.join(".codex/hooks/project_custom")).unwrap();
    let known = ".codex/hooks/crates/old/src/known.rs";
    let unknown = ".codex/hooks/crates/old/src/custom.rs";
    let absent = ".codex/hooks/crates/old/src/absent.rs";
    fs::write(root.join(known), b"published\r\n").unwrap();
    fs::write(root.join(unknown), b"custom\n").unwrap();
    fs::write(
        root.join(".codex/hooks/project_custom/lib.rs"),
        b"project\n",
    )
    .unwrap();
    let hash = sha_git(b"published\n");
    let contract = json!({"retired_assets":[
        {"id":"known","target":known,"known_hashes":[hash]},
        {"id":"unknown","target":unknown,"known_hashes":[sha_git(b"old\n")]},
        {"id":"absent","target":absent,"known_hashes":[sha_git(b"old\n")]}
    ]});
    let (mut actions, mut deletes, mut gaps, mut reads, mut entries) = (
        Vec::new(),
        Vec::new(),
        Vec::new(),
        BTreeMap::new(),
        Vec::new(),
    );
    plan(
        &root,
        &contract,
        &mut actions,
        &mut deletes,
        &mut gaps,
        &mut reads,
        &mut entries,
    )
    .unwrap();
    assert_eq!(actions.len(), 1);
    assert_eq!(deletes, vec![root.join(known)]);
    assert_eq!(gaps.len(), 1);
    assert_eq!(entries[1]["disposition"], "required-preserve");
    assert_eq!(reads.len(), 3);
    assert_eq!(reads[absent], None);
    assert_eq!(fs::read(root.join(known)).unwrap(), b"published\r\n");
    assert_eq!(fs::read(root.join(unknown)).unwrap(), b"custom\n");
    assert_eq!(
        fs::read(root.join(".codex/hooks/project_custom/lib.rs")).unwrap(),
        b"project\n"
    );
    fs::remove_dir_all(root).unwrap();
}
