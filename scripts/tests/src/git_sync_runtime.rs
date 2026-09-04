use bridgeforge_core::{ProcessRequest, ProcessRunner, SystemProcessRunner};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn copy_tree(source: &Path, target: &Path) {
    fs::create_dir_all(target).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        if matches!(
            entry.file_name().to_str(),
            Some("target" | ".git" | ".runtime" | ".venv" | "__pycache__")
        ) {
            continue;
        }
        assert!(!entry.file_type().unwrap().is_symlink());
        let destination = target.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &destination);
        } else {
            fs::copy(entry.path(), destination).unwrap();
        }
    }
}
fn git(root: &Path, args: &[&str]) -> String {
    let mut request = ProcessRequest::new("git", root);
    request.args = args.iter().map(Into::into).collect();
    request.timeout = Duration::from_secs(120);
    let output = SystemProcessRunner.run(&request).unwrap();
    assert!(
        !output.timed_out && output.code == 0,
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().into()
}

#[test]
fn real_factory_cli_sync_builds_new_runtime_and_commits_through_precommit() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap();
    let fixture = Fixture(std::env::temp_dir().join(format!(
            "bf-real-factory-sync-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )));
    let root = fixture.0.join("factory");
    copy_tree(source, &root);
    git(&root, &["init"]);
    git(&root, &["config", "user.name", "BridgeForge Fixture"]);
    git(&root, &["config", "user.email", "fixture@example.invalid"]);
    git(&root, &["config", "core.autocrlf", "false"]);
    git(&root, &["config", "core.hooksPath", ".git/hooks"]);
    git(&root, &["add", "."]);
    git(&root, &["commit", "-m", "fixture seed"]);
    git(&root, &["config", "core.hooksPath", ".githooks"]);
    let remote = fixture.0.join("origin.git");
    git(&root, &["init", "--bare", remote.to_str().unwrap()]);
    git(
        &root,
        &["remote", "add", "origin", remote.to_str().unwrap()],
    );
    git(&root, &["push", "-u", "origin", "HEAD"]);
    let old: bridgeforge_core::release::SemVer = fs::read_to_string(root.join("VERSION"))
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    let cli = root.join(if cfg!(windows) {
        ".codex/bin/bridgeforge.exe"
    } else {
        ".codex/bin/bridgeforge"
    });
    let original = fs::read(&cli).unwrap();
    let mut readme = fs::read(root.join("README.md")).unwrap();
    readme.extend_from_slice(b"\nFixture: exercise complete automatic release.\n");
    fs::write(root.join("README.md"), &readme).unwrap();
    let mut request = ProcessRequest::new(cli.as_os_str(), &root);
    request.args = [
        "git-sync",
        "--message",
        "fix: verify factory automatic runtime release",
    ]
    .iter()
    .map(Into::into)
    .collect();
    request.timeout = Duration::from_secs(2400);
    let output = SystemProcessRunner.run(&request).unwrap();
    assert!(
        !output.timed_out && output.code == 0,
        "actual CLI sync failed: {} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(git(&root, &["status", "--porcelain=v1"]).is_empty());
    assert_eq!(
        git(
            &root,
            &["rev-list", "--left-right", "--count", "HEAD...@{u}"]
        ),
        "0\t0"
    );
    let next = format!("{}.{}.{}", old.major, old.minor, old.patch + 1);
    assert_eq!(
        fs::read_to_string(root.join("VERSION")).unwrap().trim(),
        next.to_string()
    );
    assert_ne!(fs::read(&cli).unwrap(), original);
    assert_eq!(fs::read(root.join("README.md")).unwrap(), readme);
    bridgeforge_core::baseline::verify(&root, None, true).unwrap();
    bridgeforge_core::baseline::verify_index(&root, &SystemProcessRunner).unwrap();
    assert!(!bridgeforge_core::manifest::rebuild(&root, true).unwrap());
    request.args = ["self-test", "--json"].iter().map(Into::into).collect();
    request.timeout = Duration::from_secs(30);
    let tested = SystemProcessRunner.run(&request).unwrap();
    let receipt: serde_json::Value = serde_json::from_slice(&tested.stdout).unwrap();
    assert_eq!(tested.code, 0);
    assert_eq!(receipt["version"], next.to_string());

    let released_head = git(&root, &["rev-parse", "HEAD"]);
    let hook_receipt = root.join(".codex/bin/build-receipt-hook.json");
    fs::write(&hook_receipt, b"{}\n").unwrap();
    request.args = ["git-sync"].iter().map(Into::into).collect();
    request.timeout = Duration::from_secs(2400);
    let repaired = SystemProcessRunner.run(&request).unwrap();
    assert!(
        !repaired.timed_out && repaired.code == 0,
        "clean factory runtime repair failed: {} {}",
        String::from_utf8_lossy(&repaired.stdout),
        String::from_utf8_lossy(&repaired.stderr)
    );
    assert_eq!(git(&root, &["rev-parse", "HEAD"]), released_head);
    assert_eq!(
        fs::read_to_string(root.join("VERSION")).unwrap().trim(),
        next
    );
    assert!(git(&root, &["status", "--porcelain=v1"]).is_empty());
    assert_eq!(
        git(
            &root,
            &["rev-list", "--left-right", "--count", "HEAD...@{u}"]
        ),
        "0\t0"
    );
    bridgeforge_core::baseline::verify(&root, None, true).unwrap();
    assert_ne!(fs::read(&hook_receipt).unwrap(), b"{}\n");

    let cli_modified = fs::metadata(&cli).unwrap().modified().unwrap();
    let receipt_modified = fs::metadata(&hook_receipt).unwrap().modified().unwrap();
    let fast_path = SystemProcessRunner.run(&request).unwrap();
    assert!(
        !fast_path.timed_out && fast_path.code == 0,
        "healthy clean factory fast path failed: {} {}",
        String::from_utf8_lossy(&fast_path.stdout),
        String::from_utf8_lossy(&fast_path.stderr)
    );
    assert_eq!(
        fs::metadata(&cli).unwrap().modified().unwrap(),
        cli_modified
    );
    assert_eq!(
        fs::metadata(&hook_receipt).unwrap().modified().unwrap(),
        receipt_modified
    );
    assert!(git(&root, &["status", "--porcelain=v1"]).is_empty());
    assert_eq!(
        git(
            &root,
            &["rev-list", "--left-right", "--count", "HEAD...@{u}"]
        ),
        "0\t0"
    );
    println!(
        "real factory CLI: version {old} -> {next}; clean runtime self-healed without a release; healthy fast path did not rewrite runtime; real pre-commit accepted; runtime and index verified; local remote parity 0/0"
    );
}
