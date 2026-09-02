use super::*;

#[test]
fn active_lifecycle_wins() {
    let mut root = std::env::temp_dir();
    root.push(format!("bridgeforge-archive-{}", std::process::id()));
    let topic = root.join("doc/1_delivery/demo");
    fs::create_dir_all(&topic).unwrap();
    fs::write(topic.join("done.md"), "---\nlifecycle: completed\n---\n").unwrap();
    fs::write(topic.join("active.md"), "---\nlifecycle: active\n---\n").unwrap();
    assert!(scan(&root).unwrap().is_empty());
    let _ = fs::remove_dir_all(root);
}
