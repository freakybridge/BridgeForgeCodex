use super::*;
use std::{
    fs,
    io::{Read, Write},
    process::{Command, Stdio},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[test]
#[ignore = "child process fixture for replacing its own loaded executable"]
fn loaded_image_child() {
    let root = PathBuf::from(std::env::var_os("BFC_IMAGE_ROOT").unwrap());
    let path = std::env::current_exe().unwrap();
    let before = fs::read(&path).unwrap();
    let mut after = before.clone();
    after.extend_from_slice(b"new-image-overlay");
    write_binary(&root, &path, &after).unwrap();
    assert_eq!(fs::read(&path).unwrap(), after);
    fs::write(root.join("ready"), b"ready").unwrap();
    std::io::stdin().read_exact(&mut [0u8; 1]).unwrap();
    write_binary(&root, &path, &before).unwrap();
    assert_eq!(fs::read(&path).unwrap(), before);
}

#[test]
fn generated_binary_can_replace_its_loaded_image_and_restore() {
    let root = std::env::temp_dir().join(format!(
        "bfc-image-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&root).unwrap();
    let path = root.join(if cfg!(windows) {
        "loaded.exe"
    } else {
        "loaded"
    });
    let before = fs::read(std::env::current_exe().unwrap()).unwrap();
    write_binary(&root, &path, &before).unwrap();
    let mut command = Command::new(&path);
    command
        .args([
            "--exact",
            "runtime::tests::loaded_image_child",
            "--ignored",
            "--nocapture",
        ])
        .env("BFC_IMAGE_ROOT", &root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let mut child = command.spawn().unwrap();
    let deadline = Instant::now() + Duration::from_secs(20);
    while !root.join("ready").exists()
        && Instant::now() < deadline
        && child.try_wait().unwrap().is_none()
    {
        std::thread::sleep(Duration::from_millis(20));
    }
    if !root.join("ready").exists() {
        let _ = child.kill();
        let output = child.wait_with_output().unwrap();
        let _ = fs::remove_dir_all(&root);
        panic!(
            "child failed: {} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let mut expected = before.clone();
    expected.extend_from_slice(b"new-image-overlay");
    assert_eq!(fs::read(&path).unwrap(), expected);
    assert!(
        child.try_wait().unwrap().is_none(),
        "original process must stay alive after replacement"
    );
    child.stdin.take().unwrap().write_all(b"x").unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read(&path).unwrap(), before);
    cleanup_images(&root).unwrap();
    assert_eq!(
        fs::read_dir(image_directory(&root).unwrap())
            .unwrap()
            .count(),
        0
    );
    fs::remove_dir_all(root).unwrap();
}
