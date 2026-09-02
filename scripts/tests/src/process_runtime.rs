use bridgeforge_core::{ProcessRequest, ProcessRunner, SystemProcessRunner};
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "bf-process-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn request(mode: &str, temp: &Temp) -> ProcessRequest {
    let mut request = ProcessRequest::new(std::env::current_exe().unwrap(), &temp.0);
    request.args = [
        "--ignored",
        "--exact",
        "process_runtime::child_helper",
        "--nocapture",
    ]
    .into_iter()
    .map(Into::into)
    .collect();
    request.env.insert("BF_PROCESS_FIXTURE".into(), mode.into());
    request
        .env
        .insert("BF_PROCESS_PID".into(), temp.0.join("pid").into());
    request.timeout = Duration::from_millis(800);
    request
}

#[test]
#[ignore = "child process fixture invoked explicitly by process runtime tests"]
fn child_helper() {
    let Ok(mode) = std::env::var("BF_PROCESS_FIXTURE") else {
        return;
    };
    if mode == "streams" {
        std::io::stdout()
            .write_all(&vec![b'o'; 128 * 1024])
            .unwrap();
        std::io::stderr()
            .write_all(&vec![b'e'; 128 * 1024])
            .unwrap();
        let mut input = Vec::new();
        std::io::stdin().read_to_end(&mut input).unwrap();
        println!("INPUT:{}", input.len());
        std::process::exit(7);
    }
    if mode == "leaf" {
        fs::write(
            std::env::var_os("BF_PROCESS_PID").unwrap(),
            std::process::id().to_string(),
        )
        .unwrap();
        println!("leaf-ready");
        std::io::stdout().flush().unwrap();
    }
    if mode == "tree" || mode == "orphan-pipe" {
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "--ignored",
                "--exact",
                "process_runtime::child_helper",
                "--nocapture",
            ])
            .env("BF_PROCESS_FIXTURE", "leaf")
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x0800_0000);
        }
        let mut child = command.spawn().unwrap();
        if mode == "orphan-pipe" {
            std::process::exit(0);
        }
        child.wait().unwrap();
        std::process::exit(0);
    }
    std::thread::sleep(Duration::from_secs(8));
    std::process::exit(0);
}

#[test]
fn streams_and_nonzero_exit_remain_exact() {
    let temp = Temp::new();
    let mut request = request("streams", &temp);
    request.timeout = Duration::from_secs(10);
    request.stdin = vec![b'i'; 1024 * 1024];
    let output = SystemProcessRunner.run(&request).unwrap();
    assert_eq!(output.code, 7);
    assert!(!output.timed_out);
    assert_eq!(
        output.stdout.iter().filter(|&&b| b == b'o').count(),
        128 * 1024
    );
    assert_eq!(output.stderr, vec![b'e'; 128 * 1024]);
    assert!(String::from_utf8_lossy(&output.stdout).contains("INPUT:1048576"));
}

#[test]
fn timeout_includes_blocked_stdin() {
    let temp = Temp::new();
    let mut request = request("blocked-stdin", &temp);
    request.stdin = vec![b'i'; 4 * 1024 * 1024];
    let start = Instant::now();
    let output = SystemProcessRunner.run(&request).unwrap();
    assert!(output.timed_out);
    assert_ne!(output.code, 0);
    assert!(
        start.elapsed() < Duration::from_secs(4),
        "{:?}",
        start.elapsed()
    );
}

#[cfg(windows)]
fn assert_process_gone(pid: u32) {
    use windows_sys::Win32::Foundation::{CloseHandle, WAIT_OBJECT_0};
    use windows_sys::Win32::System::Threading::{
        OpenProcess, PROCESS_SYNCHRONIZE, WaitForSingleObject,
    };
    // A live handle is queried only for the recorded fixture PID.
    let handle = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, 0, pid) };
    if handle.is_null() {
        assert_eq!(std::io::Error::last_os_error().raw_os_error(), Some(87));
        return;
    }
    let state = unsafe { WaitForSingleObject(handle, 2000) };
    unsafe {
        CloseHandle(handle);
    }
    assert_eq!(state, WAIT_OBJECT_0, "descendant still running: {pid}");
}

#[test]
fn timeout_kills_descendants_even_after_parent_exits() {
    for mode in ["tree", "orphan-pipe"] {
        let temp = Temp::new();
        let mut request = request(mode, &temp);
        // Allow both fixture processes to start while other tests compile real workspaces.
        // The leaf sleeps for eight seconds, so only tree termination can satisfy this bound.
        request.timeout = Duration::from_secs(3);
        let start = Instant::now();
        let output = SystemProcessRunner.run(&request).unwrap();
        assert!(output.timed_out, "{mode}");
        assert_ne!(output.code, 0);
        assert!(
            start.elapsed() < Duration::from_secs(6),
            "{mode}: {:?}",
            start.elapsed()
        );
        let pid: u32 = fs::read_to_string(temp.0.join("pid"))
            .unwrap()
            .parse()
            .unwrap();
        #[cfg(windows)]
        assert_process_gone(pid);
        #[cfg(not(windows))]
        let _ = pid;
        assert!(String::from_utf8_lossy(&output.stdout).contains("leaf-ready"));
        assert!(
            start.elapsed() < Duration::from_secs(6),
            "{mode}: terminal verification exceeded timeout bound: {:?}",
            start.elapsed()
        );
    }
}

#[test]
fn drains_large_stdout_before_child_exit() {
    let temp = Temp::new();
    let mut request = ProcessRequest::new("git", &temp.0);
    request.args = [
        "-c",
        "alias.large=!for i in 1 2 3 4 5 6 7 8; do git help -a; done",
        "large",
    ]
    .into_iter()
    .map(Into::into)
    .collect();
    request.timeout = Duration::from_secs(20);
    let output = SystemProcessRunner.run(&request).unwrap();
    assert!(!output.timed_out);
    assert_eq!(output.code, 0);
    assert!(output.stdout.len() > 64 * 1024);
}

#[test]
fn missing_program_remains_an_error() {
    let temp = Temp::new();
    let request = ProcessRequest::new(temp.0.join("nonexistent.exe"), &temp.0);
    assert!(SystemProcessRunner.run(&request).is_err());
}
