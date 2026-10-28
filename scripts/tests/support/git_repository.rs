//! Shared isolated Git fixtures; no production dependency on this module.
use bridgeforge_core::{ProcessOutput, ProcessRequest, ProcessRunner, SystemProcessRunner};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

fn payload_sha(payload: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(payload))
}

// Some package contexts only need git_ok; factory assertions also need output.
#[allow(dead_code)]
pub(crate) fn git_output(root: &Path, args: &[&str]) -> String {
    let mut request = ProcessRequest::new("git", root);
    request.args = args.iter().map(std::ffi::OsString::from).collect();
    request.timeout = Duration::from_secs(30);
    let output = SystemProcessRunner.run(&request).unwrap();
    assert!(
        !output.timed_out && output.code == 0,
        "git {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().into()
}

pub(crate) struct RealRepository(pub(crate) PathBuf);

impl Drop for RealRepository {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

pub(crate) fn git_ok(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr)
    );
}

pub(crate) fn real_repository(name: &str) -> RealRepository {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let token = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "bridgeforge-git-sync-{name}-{}-{}-{token}",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    fs::create_dir_all(&root).unwrap();
    git_ok(&root, &["init"]);
    git_ok(&root, &["config", "user.name", "BridgeForge Test"]);
    git_ok(
        &root,
        &["config", "user.email", "bridgeforge@example.invalid"],
    );
    fs::write(root.join("tracked.txt"), b"base\n").unwrap();
    git_ok(&root, &["add", "tracked.txt"]);
    git_ok(&root, &["commit", "-m", "seed"]);
    RealRepository(root)
}

pub(crate) fn managed_contract(asset: &[u8]) -> Vec<u8> {
    serde_json::to_vec_pretty(&json!({
        "schema_version": 4,
        "release_version": "8.0.0",
        "host": "codex",
        "stamp": ".codex/.bridgeforge_codex_version",
        "contract_target": ".codex/managed-skeleton.json",
        "assets": [{
            "id": "managed.asset",
            "source": "templates/managed.txt",
            "target": "managed.txt",
            "strategy": "whole",
            "current_sha256": payload_sha(asset),
        }],
        "baseline_model": "current-only", "compatibility_baseline": "1.0.0",
        "generated_assets": [],
    }))
    .unwrap()
}

pub(crate) fn managed_repository(name: &str) -> (RealRepository, PathBuf) {
    let repo = real_repository(name);
    let remote = repo.0.with_file_name(format!(
        "{}-remote.git",
        repo.0.file_name().unwrap().to_string_lossy()
    ));
    fs::create_dir_all(repo.0.join(".codex")).unwrap();
    fs::write(repo.0.join("managed.txt"), b"old\n").unwrap();
    fs::write(repo.0.join(".gitignore"), b".runtime/\n").unwrap();
    fs::write(
        repo.0.join(".codex/managed-skeleton.json"),
        managed_contract(b"old\n"),
    )
    .unwrap();
    fs::write(repo.0.join(".codex/.bridgeforge_codex_version"), b"8.0.0\n").unwrap();
    git_ok(&repo.0, &["add", "."]);
    git_ok(&repo.0, &["commit", "-m", "baseline"]);
    git_ok(
        &repo.0,
        &["init", "--bare", remote.to_string_lossy().as_ref()],
    );
    git_ok(
        &repo.0,
        &["remote", "add", "origin", remote.to_string_lossy().as_ref()],
    );
    git_ok(&repo.0, &["push", "-u", "origin", "HEAD"]);
    (repo, remote)
}

pub(crate) fn out(code: i32, stdout: &str) -> ProcessOutput {
    ProcessOutput {
        code,
        stdout: stdout.as_bytes().to_vec(),
        stderr: Vec::new(),
        timed_out: false,
    }
}
