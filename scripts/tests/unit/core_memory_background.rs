use super::*;
use std::fs::{self, OpenOptions};
use std::os::windows::fs::OpenOptionsExt;
use std::os::windows::io::AsRawHandle;
use std::os::windows::process::CommandExt;
use std::process::Command;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use windows_sys::Win32::Foundation::{HANDLE_FLAG_INHERIT, SetHandleInformation};

#[test]
fn background_rejects_invalid_input_and_reports_launch_errors() {
    let executable = std::env::current_exe().unwrap();
    for argument in [
        OsString::from("embedded\0nul"),
        OsString::from("x".repeat(32767)),
    ] {
        assert_eq!(
            spawn(&executable, &[argument]).unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
    }
    assert_eq!(
        spawn(Path::new("relative.exe"), &[]).unwrap_err().kind(),
        io::ErrorKind::InvalidInput
    );
    assert!(
        spawn(
            &executable.with_extension("missing-native-memory-test"),
            &[]
        )
        .is_err()
    );
}

#[test]
fn background_does_not_inherit_extra_handles_and_preserves_native_contract() {
    let folder = std::env::temp_dir().join(format!(
        "bf-background 空格 ' $ {}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&folder).unwrap();
    let binary = folder.join("worker probe.exe");
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .unwrap();
    let built = Command::new("rustc")
        .arg(root.join("scripts/tests/fixtures/native_memory_background_probe.rs"))
        .arg("-o")
        .arg(&binary)
        .creation_flags(0x08000000)
        .output()
        .unwrap();
    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );
    // An extra inheritable, exclusively opened handle models a hook pipe copy.
    // Redirecting only the three standard handles must not leak this handle.
    let sentinel = folder.join("exclusive");
    let held = OpenOptions::new()
        .write(true)
        .create_new(true)
        .share_mode(0)
        .open(&sentinel)
        .unwrap();
    assert_ne!(
        unsafe {
            SetHandleInformation(
                held.as_raw_handle(),
                HANDLE_FLAG_INHERIT,
                HANDLE_FLAG_INHERIT,
            )
        },
        0
    );
    let values = [
        "",
        "normal",
        "中文 空格",
        "a\"b",
        "C:\\path with space\\",
        "\\\"",
        "' $ `",
    ];
    let mut args = vec![folder.as_os_str().to_owned()];
    args.extend(values.iter().map(OsString::from));
    let started = Instant::now();
    let pid = spawn(&binary, &args).unwrap();
    drop(held);
    while !folder.join("ready").exists() && started.elapsed() < Duration::from_secs(2) {
        std::thread::sleep(Duration::from_millis(10));
    }
    let ready = fs::read_to_string(folder.join("ready"));
    let still_running = super::super::worker::process_alive(pid);
    let reopened = OpenOptions::new().write(true).open(&sentinel);
    // Allow the bounded probe to exit even on a failing assertion.
    while super::super::worker::process_alive(pid) && started.elapsed() < Duration::from_secs(8) {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(
        ready.unwrap_or_else(|error| panic!(
            "{error}; phase={:?}; pid={pid}",
            fs::read_to_string(folder.join("phase"))
        )),
        values.join("\n")
    );
    assert!(still_running, "assertions must run before the worker exits");
    assert!(
        reopened.is_ok(),
        "worker inherited a non-stdio handle: {reopened:?}"
    );
    drop(reopened);
    assert!(folder.join("done").exists(), "worker did not finish");
    fs::remove_dir_all(folder).unwrap();
}
