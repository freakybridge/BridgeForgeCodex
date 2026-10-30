use super::*;
use std::{
    fs,
    time::{Duration, Instant},
};

#[test]
#[ignore = "isolated child process"]
fn activity_child() {
    let root = std::path::PathBuf::from(std::env::var_os("BFC_ACTIVITY_FIXTURE").unwrap());
    fs::write(root.join("ready"), b"ready").unwrap();
    let started = Instant::now();
    while !root.join("finish").exists() && started.elapsed() < Duration::from_secs(15) {
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn running_legacy_image_blocks_migration_until_process_exits() {
    let root = test_artifacts::root(env!("CARGO_MANIFEST_DIR"), "process-activity")
        .join(format!("{}", std::process::id()));
    fs::create_dir_all(root.join(".codex/bin")).unwrap();
    let root = root.canonicalize().unwrap();
    let binary = root.join(format!(
        ".codex/bin/bridgeforge-factory{}",
        std::env::consts::EXE_SUFFIX
    ));
    fs::copy(std::env::current_exe().unwrap(), &binary).unwrap();
    let mut command = std::process::Command::new(&binary);
    command
        .args([
            "--exact",
            "process_activity::tests::activity_child",
            "--ignored",
        ])
        .env("BFC_ACTIVITY_FIXTURE", &root)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = command.spawn().unwrap();
    let started = Instant::now();
    while !root.join("ready").exists() && started.elapsed() < Duration::from_secs(5) {
        std::thread::sleep(Duration::from_millis(20));
    }
    let child_id = child.id();
    // The live enumeration proves this fixture's image is visible. Other
    // projects and the managed develop parent are not owned by this fixture.
    let snapshot = images().unwrap();
    assert!(snapshot.iter().any(|(pid, path)| *pid == child_id
        && path.canonicalize().unwrap() == binary.canonicalize().unwrap()));
    let result = legacy_clients_quiet_with_images(
        &root,
        snapshot
            .into_iter()
            .filter(|(pid, _)| *pid == child_id)
            .collect(),
    );
    fs::write(root.join("finish"), b"finish").unwrap();
    assert!(child.wait().unwrap().success());
    let error = result.unwrap_err();
    assert!(
        error.contains(&format!(
            "active legacy-capable BridgeForge process {child_id}:"
        )),
        "{error}"
    );
    let after = images().unwrap();
    assert!(!after.iter().any(|(pid, _)| *pid == child_id));
    legacy_clients_quiet_with_images(
        &root,
        after
            .into_iter()
            .filter(|(pid, _)| *pid == child_id)
            .collect(),
    )
    .unwrap();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn external_managed_images_remain_blocked_while_current_layout_is_allowed() {
    let root = test_artifacts::root(env!("CARGO_MANIFEST_DIR"), "process-activity-policy");
    fs::create_dir_all(&root).unwrap();
    let root = root.canonicalize().unwrap();
    let foreign_pid = std::process::id().wrapping_add(1);
    let image = format!("bridgeforge-factory{}", std::env::consts::EXE_SUFFIX);
    for path in [
        root.join(".codex/bin").join(&image),
        root.join(".runtime/bridgeforge-codex/git-sync-images")
            .join(&image),
        root.join("outside-project").join(&image),
        root.join("outside-project")
            .join(crate::artifact_paths::FACTORY_BIN)
            .join(&image),
    ] {
        assert!(
            legacy_clients_quiet_with_images(&root, vec![(foreign_pid, path)])
                .unwrap_err()
                .contains("active legacy-capable")
        );
    }
    for directory in [
        crate::artifact_paths::FACTORY_BIN,
        crate::artifact_paths::PROJECT_BIN,
    ] {
        legacy_clients_quiet_with_images(
            &root,
            vec![(foreign_pid, root.join(directory).join(&image))],
        )
        .unwrap();
    }
    fs::remove_dir_all(root).unwrap();
}

#[allow(dead_code)]
#[path = "../support/artifact_paths.rs"]
mod test_artifacts;
