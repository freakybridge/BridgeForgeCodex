use super::*;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[test]
#[ignore = "child process helper"]
fn lock_holder_process() {
    let root = std::env::var_os("BF_FILE_LOCK_FIXTURE").expect("fixture");
    let root = std::path::PathBuf::from(root);
    let _held = FileLock::acquire(&root.join("operation.lock")).unwrap();
    fs::write(root.join("ready"), b"ready").unwrap();
    loop {
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[test]
fn os_lock_excludes_contenders_and_recovers_after_process_death() {
    let root = std::env::temp_dir().join(format!(
        "bfc-file-lock-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&root).unwrap();
    let path = root.join("operation.lock");
    fs::write(&path, b"stale owner metadata is not a held lock").unwrap();
    let held = FileLock::acquire(&path).unwrap();
    assert!(FileLock::acquire(&path).is_err());
    drop(held);
    assert!(path.exists(), "the stable inode must never be unlinked");
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--ignored",
            "--exact",
            "file_lock::tests::lock_holder_process",
        ])
        .env("BF_FILE_LOCK_FIXTURE", &root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let mut child = command.spawn().unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !root.join("ready").exists() && Instant::now() < deadline {
        if child.try_wait().unwrap().is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let ready = root.join("ready").exists();
    let excluded = ready && FileLock::acquire(&path).is_err();
    let _ = child.kill();
    child.wait().unwrap();
    assert!(ready, "lock holder child failed to start");
    assert!(excluded, "a live process must exclude a second lock holder");
    drop(FileLock::acquire(&path).expect("process death must release the OS lock"));
    fs::remove_dir_all(root).unwrap();
}
