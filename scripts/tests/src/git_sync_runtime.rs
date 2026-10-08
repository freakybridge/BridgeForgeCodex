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

fn fixture_development_checks(root: &Path) {
    fs::write(root.join(".codex/development-checks.json"), br#"{"schema":1,"checks":[{"id":"fixture-diff","program":"git","args":["diff","--check"],"timeout_seconds":30}]}"#).unwrap();
}

fn prepare_fixture(cli: &Path, root: &Path, message: &str) {
    let mut request = ProcessRequest::new(cli.as_os_str(), root);
    request.args = ["git-sync", "--prepare-release", "--message", message]
        .iter()
        .map(Into::into)
        .collect();
    request.timeout = Duration::from_secs(2400);
    let output = SystemProcessRunner.run(&request).unwrap();
    assert!(
        !output.timed_out && output.code == 0,
        "prepare: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let receipt: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(receipt["status"], "prepared");
}

#[test]
fn real_cli_explicit_release_preview_and_execution() {
    for policy in ["missing-file", "missing-field", "per_commit", "explicit_release"] {
        real_cli_release_flow(policy);
    }
}

fn real_cli_release_flow(policy: &str) {
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap();
    let cli = source.join(if cfg!(windows) {
        ".codex/bin/bridgeforge.exe"
    } else {
        ".codex/bin/bridgeforge"
    });
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let fixture_name = format!("bf-explicit-cli-{}-{nonce}", std::process::id());
    let fixture = Fixture(std::env::temp_dir().join(fixture_name));
    let root = fixture.0.join("project");
    fs::create_dir_all(root.join(".codex")).unwrap();
    let call = |args: &[&str]| {
        let mut request = ProcessRequest::new(cli.as_os_str(), &root);
        request.args = args.iter().map(Into::into).collect();
        request.timeout = Duration::from_secs(120);
        let output = SystemProcessRunner.run(&request).unwrap();
        assert!(
            !output.timed_out && output.code == 0,
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap()
    };
    assert!(
        call(&["self-test", "--json"])["capabilities"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == "git-sync-release-readiness-v1")
    );
    for flag in ["--dry-run", "--check"] {
        let mut probe = ProcessRequest::new(cli.as_os_str(), &root);
        probe.args = ["git-sync", "--prepare-release", flag]
            .iter()
            .map(Into::into)
            .collect();
        let result = SystemProcessRunner.run(&probe).unwrap();
        assert_eq!(result.code, 2);
        assert!(!root.join(".runtime").exists());
    }
    fs::write(root.join("VERSION"), b"1.0.0\n").unwrap();
    fs::write(
        root.join("Cargo.toml"),
        b"[package]\nname = \"demo\"\nversion = \"1.0.0\"\n",
    )
    .unwrap();
    fs::write(root.join("CHANGELOG.md"), b"# Changelog\n").unwrap();
    fs::write(root.join(".gitignore"), b".runtime/\n").unwrap();
    fs::write(root.join(".codex/.bridgeforge_codex_version"), b"1.0.0\n").unwrap();
    let policy_path = root.join(".codex/bridgeforge-version.json");
    if policy != "missing-file" {
        let mut config = serde_json::json!({"schema_version": 1, "manifests": ["Cargo.toml"]});
        if policy != "missing-field" {
            config["release_policy"] = serde_json::json!(policy);
        }
        fs::write(&policy_path, serde_json::to_vec(&config).unwrap()).unwrap();
    }
    fs::write(root.join("Cargo.lock"), b"version = 4\n[[package]]\nname = \"demo\"\nversion = \"1.0.0\"\n").unwrap();
    use sha2::{Digest, Sha256};
    fs::write(root.join("managed.txt"), b"managed\n").unwrap();
    let contract = serde_json::json!({
        "schema_version": 4, "release_version": "1.0.0", "host": "codex",
        "stamp": ".codex/.bridgeforge_codex_version", "contract_target": ".codex/managed-skeleton.json",
        "assets": [{"id": "managed.asset", "source": "templates/managed.txt", "target": "managed.txt",
            "strategy": "whole", "current_sha256": format!("sha256:{:x}", Sha256::digest(b"managed\n"))}],
        "generated_assets": [], "baseline_model": "current-only", "compatibility_baseline": "1.0.0"
    });
    fs::write(
        root.join(".codex/managed-skeleton.json"),
        serde_json::to_vec_pretty(&contract).unwrap(),
    )
    .unwrap();
    git(&root, &["init"]);
    git(&root, &["config", "user.name", "BridgeForge Test"]);
    git(&root, &["config", "user.email", "test@example.invalid"]);
    git(&root, &["config", "core.hooksPath", ".git/hooks"]);
    git(&root, &["add", "."]);
    git(&root, &["commit", "-m", "baseline"]);
    let remote = fixture.0.join("origin.git");
    git(&root, &["init", "--bare", remote.to_str().unwrap()]);
    git(
        &root,
        &["remote", "add", "origin", remote.to_str().unwrap()],
    );
    git(&root, &["push", "-u", "origin", "HEAD"]);
    let initial_head = git(&root, &["rev-parse", "HEAD"]);
    let initial_index = fs::read(root.join(".git/index")).unwrap();
    fs::write(root.join("VERSION"), b"0.9.9\n").unwrap();
    let mut probe = ProcessRequest::new(cli.as_os_str(), &root);
    probe.args = ["git-sync", "--release-status"].iter().map(Into::into).collect();
    let output = SystemProcessRunner.run(&probe).unwrap();
    assert_eq!(output.code, 2);
    let setup: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(setup["status"], "setup-required");
    assert_eq!(setup["blockers"].as_array().unwrap().len(), 2);
    assert_eq!(git(&root, &["rev-parse", "HEAD"]), initial_head);
    assert_eq!(fs::read(root.join(".git/index")).unwrap(), initial_index);
    assert!(!root.join(".runtime").exists());
    assert!(!root.join(".codex/development-checks.json").exists());
    fs::write(root.join("VERSION"), b"1.0.0\n").unwrap();
    let version_files = ["VERSION", "Cargo.toml", "Cargo.lock", "CHANGELOG.md"];
    let version_bytes = version_files.map(|name| fs::read(root.join(name)).unwrap());
    let policy_bytes = fs::read(&policy_path).ok();
    fs::write(root.join("business.txt"), b"feature\n").unwrap();
    let ordinary = call(&["git-sync", "--message", "feat: new capability"]);
    assert_eq!(ordinary["version_bumped"], false);
    assert_eq!(ordinary["release_requested"], false);
    assert_eq!(ordinary["release_policy"], "explicit_release");
    assert_eq!(ordinary["version_before"], "1.0.0");
    assert_eq!(ordinary["version_after"], "1.0.0");
    assert_eq!(ordinary["working_tree"], "clean");
    assert_eq!(ordinary["ahead"], 0);
    assert_eq!(ordinary["behind"], 0);
    for (name, bytes) in version_files.iter().zip(&version_bytes) {
        assert_eq!(&fs::read(root.join(name)).unwrap(), bytes, "{policy}: {name}");
    }
    assert_eq!(fs::read(&policy_path).ok(), policy_bytes);
    // Ordinary sync succeeded without release checks. Only development setup
    // creates the project-owned checks before preparing an explicit release.
    fixture_development_checks(&root);
    let before = git(&root, &["rev-parse", "HEAD"]);
    let index = fs::read(root.join(".git/index")).unwrap();
    let fetch = fs::read(root.join(".git/FETCH_HEAD")).unwrap();
    let preview = call(&["git-sync", "--release-preview", "--message", "fix: release"]);
    assert_eq!(preview["version_after"], "1.1.0");
    assert_eq!(git(&root, &["rev-parse", "HEAD"]), before);
    assert_eq!(fs::read(root.join(".git/index")).unwrap(), index);
    assert_eq!(fs::read(root.join(".git/FETCH_HEAD")).unwrap(), fetch);
    assert_eq!(fs::read(root.join("VERSION")).unwrap(), b"1.0.0\n");
    prepare_fixture(&cli, &root, "fix: release");
    assert_eq!(
        call(&["git-sync", "--release-status"])["status"],
        "prepared"
    );
    let released = call(&["git-sync", "--release", "--message", "fix: release"]);
    assert_eq!(released["version_after"], "1.1.0");
    assert_eq!(released["release_requested"], true);
    assert_eq!(released["version_bumped"], true);
    assert_eq!(fs::read(&policy_path).ok(), policy_bytes);
    assert_eq!(released["working_tree"], "clean");
    assert_eq!(released["ahead"], 0);
    assert_eq!(released["behind"], 0);
    assert_eq!(released["generated_assets_built"], 0);
    let repeated = call(&["git-sync", "--release", "--message", "feat!: ignored"]);
    assert_eq!(repeated["version_bumped"], false);
    assert_eq!(repeated["commit"], released["commit"]);
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
    fixture_development_checks(&root);
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
    prepare_fixture(&cli, &root, "fix: verify factory automatic runtime release");
    let mut request = ProcessRequest::new(cli.as_os_str(), &root);
    request.args = [
        "git-sync",
        "--release",
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

#[test]
#[ignore = "explicit factory performance experiment using isolated local Git only"]
fn factory_ordinary_sync_performance() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let fixture =
        Fixture(std::env::temp_dir().join(format!("bf-sync-perf-{}-{nonce}", std::process::id())));
    let root = fixture.0.join("factory");
    copy_tree(source, &root);
    fixture_development_checks(&root);
    let config_path = root.join(".codex/bridgeforge-version.json");
    let mut config: serde_json::Value =
        serde_json::from_slice(&fs::read(&config_path).unwrap()).unwrap();
    config["release_policy"] = serde_json::json!("explicit_release");
    fs::write(config_path, serde_json::to_vec_pretty(&config).unwrap()).unwrap();
    git(&root, &["init"]);
    git(&root, &["config", "user.name", "BridgeForge Benchmark"]);
    git(
        &root,
        &["config", "user.email", "benchmark@example.invalid"],
    );
    git(&root, &["config", "core.autocrlf", "false"]);
    git(&root, &["config", "core.hooksPath", ".git/hooks"]);
    git(&root, &["add", "."]);
    git(&root, &["commit", "-m", "chore: benchmark baseline"]);
    git(&root, &["config", "core.hooksPath", ".githooks"]);
    let remote = fixture.0.join("origin.git");
    git(&root, &["init", "--bare", remote.to_str().unwrap()]);
    git(
        &root,
        &["remote", "add", "origin", remote.to_str().unwrap()],
    );
    git(&root, &["push", "-u", "origin", "HEAD"]);
    let version = fs::read(root.join("VERSION")).unwrap();
    let cli = root.join(if cfg!(windows) {
        ".codex/bin/bridgeforge.exe"
    } else {
        ".codex/bin/bridgeforge"
    });
    let cli_before = fs::metadata(&cli).unwrap().modified().unwrap();
    let mut readme = fs::read(root.join("README.md")).unwrap();
    readme.extend_from_slice(b"\nPerformance fixture: documentation-only change.\n");
    fs::write(root.join("README.md"), readme).unwrap();
    let mut request = ProcessRequest::new(cli.as_os_str(), &root);
    request.args = ["git-sync", "--message", "docs: benchmark ordinary sync"]
        .iter()
        .map(Into::into)
        .collect();
    request.timeout = Duration::from_secs(1200);
    println!("performance fixture prepared; timed CLI starts now");
    let started = std::time::Instant::now();
    let output = SystemProcessRunner.run(&request).unwrap();
    let elapsed_ms = started.elapsed().as_millis();
    assert!(
        !output.timed_out && output.code == 0,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let receipt: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(receipt["version_bumped"], false);
    assert_eq!(fs::read(root.join("VERSION")).unwrap(), version);
    assert_eq!(receipt["working_tree"], "clean");
    assert_eq!(receipt["ahead"], 0);
    assert_eq!(receipt["behind"], 0);
    if std::env::var_os("BRIDGEFORGE_PERF_EXPECT_REUSE").is_some() {
        assert_eq!(receipt["generated_assets_reused"], 2);
        assert_eq!(receipt["generated_assets_built"], 0);
        assert_eq!(fs::metadata(&cli).unwrap().modified().unwrap(), cli_before);
    }
    println!(
        "PERF {}",
        serde_json::json!({"elapsed_ms": elapsed_ms, "receipt": receipt})
    );
    if std::env::var_os("BRIDGEFORGE_PERF_RELEASE").is_some() {
        let mut readme = fs::read(root.join("README.md")).unwrap();
        readme.extend_from_slice(b"\nPerformance fixture: explicit release.\n");
        fs::write(root.join("README.md"), readme).unwrap();
        prepare_fixture(&cli, &root, "docs: benchmark ordinary sync");
        request.args.insert(1, "--release".into());
        println!("performance explicit release starts now");
        let started = std::time::Instant::now();
        let output = SystemProcessRunner.run(&request).unwrap();
        let elapsed_ms = started.elapsed().as_millis();
        assert!(
            !output.timed_out && output.code == 0,
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let receipt: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(receipt["version_bumped"], true);
        assert_eq!(receipt["generated_assets_reused"], 2);
        assert_eq!(receipt["generated_assets_built"], 0);
        assert_eq!(receipt["working_tree"], "clean");
        assert_eq!(receipt["ahead"], 0);
        assert_eq!(receipt["behind"], 0);
        println!(
            "PERF_RELEASE {}",
            serde_json::json!({"elapsed_ms": elapsed_ms, "receipt": receipt})
        );
    }
}
