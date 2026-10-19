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
    // Factory build/rollback fixtures start from an already confirmed policy;
    // first-use interaction is tested through the real CLI below.
    let path=root.join(".codex/bridgeforge-version.json");
    let mut config:serde_json::Value=serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    config["git_sync_setup"]=serde_json::json!({"schema":1,"confirmed":true,"selection":"current"});
    fs::write(path,serde_json::to_vec_pretty(&config).unwrap()).unwrap();
}

#[test]
fn real_cli_check_reuse_ignores_invocation_trace_but_binds_business_environment() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().nth(2).unwrap();
    let cli = source.join(if cfg!(windows) { ".codex/bin/bridgeforge.exe" } else { ".codex/bin/bridgeforge" });
    let token = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let fixture = Fixture(std::env::temp_dir().join(format!("bridgeforge-cross-process-checks-{token}")));
    let root = &fixture.0;
    fs::create_dir_all(root.join(".codex")).unwrap();
    fs::write(root.join("VERSION"), b"1.0.0\n").unwrap();
    fs::write(root.join("CHANGELOG.md"), b"# Changelog\n").unwrap();
    fs::write(root.join("Cargo.toml"), b"[package]\nname = \"demo\"\nversion = \"1.0.0\"\n").unwrap();
    fs::write(root.join("Cargo.lock"), b"version = 4\n[[package]]\nname = \"demo\"\nversion = \"1.0.0\"\n").unwrap();
    fs::write(root.join(".gitignore"), b".runtime/\n").unwrap();
    fs::write(root.join(".codex/managed-skeleton.json"), br#"{"assets":[]}"#).unwrap();
    fs::write(root.join(".codex/bridgeforge-version.json"), br#"{"schema_version":1,"release_policy":"explicit_release","manifests":["Cargo.toml"]}"#).unwrap();
    fs::write(root.join(".codex/development-checks.json"), br#"{"schema":1,"checks":[{"id":"diff","program":"git","args":["diff","--check"],"timeout_seconds":30,"reusable":true}]}"#).unwrap();
    git(root, &["init"]);
    git(root, &["config", "user.name", "BridgeForge Test"]);
    git(root, &["config", "user.email", "bridgeforge@example.invalid"]);
    git(root, &["add", "."]);
    git(root, &["commit", "-m", "chore: baseline"]);
    fs::write(root.join("business.txt"), b"change\n").unwrap();
    for (trace, business, reused) in [("call-a", "a", 0), ("call-b", "a", 1), ("call-c", "b", 0), ("call-d", "b", 1)] {
        let mut request = ProcessRequest::new(cli.as_os_str(), root);
        request.args = ["git-sync", "--prepare-release", "--message", "fix: repeat"].iter().map(Into::into).collect();
        request.env.insert("CODEX_TOOL_CALL_ID".into(), trace.into());
        request.env.insert("BRIDGEFORGE_TEST_BUSINESS_INPUT".into(), business.into());
        request.timeout = Duration::from_secs(120);
        let output = SystemProcessRunner.run(&request).unwrap();
        assert!(!output.timed_out && output.code == 0, "{}", String::from_utf8_lossy(&output.stderr));
        let receipt: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(receipt["checks_reused"], reused, "{trace}");
        assert_eq!(receipt["checks_executed"], 1-reused, "{trace}");
        assert_eq!(receipt["target_version"], "1.0.1");
    }
    assert_eq!(fs::read(root.join("VERSION")).unwrap(), b"1.0.0\n");
}

#[test]
fn real_cli_explicit_release_preview_and_execution() {
    for policy in ["missing-file", "missing-field", "per_commit", "explicit_release", "rust-independent", "rust-additions"] {
        real_cli_release_flow(policy);
    }
}

fn real_cli_release_flow(policy: &str) {
    let additions = policy == "rust-additions";
    let policy = if additions { "rust-independent" } else { policy };
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
        let started = std::time::Instant::now();
        let output = SystemProcessRunner.run(&request).unwrap();
        let elapsed_ms = started.elapsed().as_millis();
        assert!(
            !output.timed_out && output.code == 0,
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let receipt = serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap();
        if receipt["status"] == "synced" {
            let inner_ms = receipt["sync_elapsed_ms"].as_u64().expect("real sync reports its internal duration");
            assert!(u128::from(inner_ms) <= elapsed_ms, "internal timing must not include the Agent or exceed the caller's process duration");
        }
        receipt
    };
    assert!(
        call(&["self-test", "--json"])["capabilities"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == "git-sync-direct-release-v1")
    );
    assert!(call(&["self-test", "--json"])["capabilities"].as_array().unwrap().iter().any(|v| v == "git-sync-rust-package-additions-v1"));
    assert!(call(&["self-test", "--json"])["capabilities"].as_array().unwrap().iter().any(|v| v == "git-sync-analysis-reuse-v1"));
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
        if policy == "rust-independent" {
            b"[package]\nname = \"demo\"\nversion = \"0.4.0\"\n".as_slice()
        } else { b"[package]\nname = \"demo\"\nversion = \"1.0.0\"\n".as_slice() },
    )
    .unwrap();
    fs::write(root.join("CHANGELOG.md"), b"# Changelog\n").unwrap();
    fs::write(root.join(".gitignore"), b".runtime/\n").unwrap();
    fs::write(root.join(".codex/.bridgeforge_codex_version"), b"1.0.0\n").unwrap();
    let policy_path = root.join(".codex/bridgeforge-version.json");
    if policy != "missing-file" {
        let mut config = if policy == "rust-independent" {
            serde_json::json!({"schema_version":2,"release_policy":"explicit_release","strategy":"rust_independent",
                "rust":{"workspaces":["Cargo.toml"],"dependency_bump":"patch"}})
        } else { serde_json::json!({"schema_version": 1, "manifests": ["Cargo.toml"]}) };
        if policy != "missing-field" && policy != "rust-independent" {
            config["release_policy"] = serde_json::json!(policy);
        }
        fs::write(&policy_path, serde_json::to_vec(&config).unwrap()).unwrap();
    }
    fs::write(root.join("Cargo.lock"), if policy == "rust-independent" {
        b"version = 4\n[[package]]\nname = \"demo\"\nversion = \"0.4.0\"\n".as_slice()
    } else { b"version = 4\n[[package]]\nname = \"demo\"\nversion = \"1.0.0\"\n".as_slice() }).unwrap();
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
    assert_eq!(setup["status"], "blocked");
    assert_eq!(setup["development_checks_required"], false);
    assert_eq!(git(&root, &["rev-parse", "HEAD"]), initial_head);
    assert_eq!(fs::read(root.join(".git/index")).unwrap(), initial_index);
    assert!(!root.join(".runtime").exists());
    assert!(!root.join(".codex/development-checks.json").exists());
    fs::write(root.join("VERSION"), b"1.0.0\n").unwrap();
    let version_files = ["VERSION", "Cargo.toml", "Cargo.lock", "CHANGELOG.md"];
    let mut first=ProcessRequest::new(cli.as_os_str(),&root);
    first.args=["git-sync","--message","feat: wait for choice"].iter().map(Into::into).collect();
    let initial=SystemProcessRunner.run(&first).unwrap();assert_eq!(initial.code,2);
    let initial:serde_json::Value=serde_json::from_slice(&initial.stdout).unwrap();assert_eq!(initial["status"],"configuration-required");
    assert_eq!(git(&root,&["rev-parse","HEAD"]),initial_head);
    assert_eq!(fs::read(root.join(".git/index")).unwrap(),initial_index);
    assert!(!root.join(".runtime").exists());
    let proposal=call(&["git-sync","--setup-plan","--strategy","current"]);
    let key=proposal["fingerprint"].as_str().unwrap();
    let selected=call(&["git-sync","--setup-apply","--strategy","current","--setup-fingerprint",key,"--confirmed-choice"]);
    assert_eq!(selected["sync_performed"],false);
    assert_eq!(call(&["git-sync","--setup-status"])["status"],"configured");
    if additions {
        fs::create_dir_all(root.join("new/src")).unwrap();
        fs::write(root.join("new/Cargo.toml"), b"[package]\nname='new'\nversion='0.1.7'\nedition='2024'\n[workspace]\n").unwrap();
        fs::write(root.join("new/src/lib.rs"), b"pub fn value() -> u32 { 7 }\n").unwrap();
        fs::write(root.join("new/Cargo.lock"), b"version=4\n[[package]]\nname='new'\nversion='0.1.7'\n").unwrap();
        fs::write(root.join("Cargo.toml"), b"[package]\nname='demo'\nversion='0.4.0'\nedition='2024'\n[dependencies]\nnew={path='new',version='=0.1.7'}\n").unwrap();
        fs::write(root.join("Cargo.lock"), b"version=4\n[[package]]\nname='demo'\nversion='0.4.0'\ndependencies=['new']\n[[package]]\nname='new'\nversion='0.1.7'\n").unwrap();
        let mut cfg: serde_json::Value = serde_json::from_slice(&fs::read(&policy_path).unwrap()).unwrap();
        cfg["rust"]["workspaces"] = serde_json::json!(["Cargo.toml", "new/Cargo.toml"]);
        cfg["rust"]["product_inputs"] = serde_json::json!(["new/Cargo.lock"]);
        cfg["rust"]["shared_inputs"] = serde_json::json!([{"path":"Cargo.lock","packages":["Cargo.toml"]}]);
        fs::write(&policy_path, serde_json::to_vec(&cfg).unwrap()).unwrap();
    }
    let version_bytes = version_files.map(|name| fs::read(root.join(name)).unwrap());
    let policy_bytes = fs::read(&policy_path).ok();
    fs::write(root.join("business.txt"), b"feature\n").unwrap();
    let ordinary = call(&["git-sync", "--message", "feat: new capability"]);
    assert_eq!(ordinary["version_bumped"], false);
    assert_eq!(ordinary["version_strategy"],if policy == "rust-independent" {"rust_independent"} else {"synchronized"});
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
    // Deliberately missing, invalid, or failing development data must never gate release.
    let development = root.join(".codex/development-checks.json");
    if policy == "missing-field" { fs::write(&development, b"not-json\n").unwrap(); }
    else if policy != "missing-file" { fs::write(&development, br#"{"schema":1,"audit_required":true,"checks":[{"id":"must-not-run","program":"not-a-real-test-program","args":[],"timeout_seconds":1}]}"#).unwrap(); }
    let development_bytes = fs::read(&development).ok();
    let record = root.join(".runtime/bridgeforge-codex/release-preparation/current.json");
    if policy != "missing-file" { fs::create_dir_all(record.parent().unwrap()).unwrap(); fs::write(&record, br#"{"status":"failed","root":"another repository"}"#).unwrap(); }
    let record_bytes = fs::read(&record).ok();
    let before = git(&root, &["rev-parse", "HEAD"]);
    let index = fs::read(root.join(".git/index")).unwrap();
    let fetch = fs::read(root.join(".git/FETCH_HEAD")).unwrap();
    let preview = call(&["git-sync", "--release-preview", "--message", "fix: release"]);
    assert_eq!(preview["version_after"], "1.1.0");
    if policy == "rust-independent" {
        assert_eq!(preview["version_strategy"],"rust_independent");
        assert_eq!(preview["package_changes"][0]["version_after"],"0.5.0");
        if additions {
            assert_eq!(preview["package_changes"][1]["reason"], "first_release");
            assert_eq!(preview["package_changes"][1]["version_after"], "0.1.7");
        }
    }
    assert_eq!(git(&root, &["rev-parse", "HEAD"]), before);
    assert_eq!(fs::read(root.join(".git/index")).unwrap(), index);
    assert_eq!(fs::read(root.join(".git/FETCH_HEAD")).unwrap(), fetch);
    assert_eq!(fs::read(root.join("VERSION")).unwrap(), b"1.0.0\n");
    assert_eq!(
        call(&["git-sync", "--release-status"])["status"],
        "release-ready"
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
    if policy == "rust-independent" {
        assert_eq!(released["package_changes"][0]["version_after"],"0.5.0");
        assert_eq!(released["compatibility_validation"],"not_performed_by_git_sync");
        if additions {
            assert_eq!(released["package_changes"], preview["package_changes"]);
            assert_eq!(fs::read(root.join("new/Cargo.toml")).unwrap(), b"[package]\nname='new'\nversion='0.1.7'\nedition='2024'\n[workspace]\n");
        }
    }
    assert_eq!(fs::read(&development).ok(), development_bytes);
    assert_eq!(fs::read(&record).ok(), record_bytes);
    let repeated = call(&["git-sync", "--release", "--message", "feat!: ignored"]);
    assert_eq!(repeated["version_bumped"], false);
    assert_eq!(repeated["version_strategy"],if policy == "rust-independent" {"rust_independent"} else {"synchronized"});
    assert_eq!(repeated["commit"], released["commit"]);
    if additions {
        fs::write(root.join("new/src/lib.rs"), b"pub fn value() -> u32 { 8 }\n").unwrap();
        let next = call(&["git-sync", "--release-preview", "--message", "fix: improve existing new package"]);
        assert_eq!(next["version_after"], "1.1.1");
        assert_eq!(next["package_changes"][0]["version_after"], "0.5.1");
        assert_eq!(next["package_changes"][0]["reason"], "dependency_change");
        assert_eq!(next["package_changes"][1]["version_after"], "0.1.8");
        assert_eq!(next["package_changes"][1]["reason"], "direct_change");
    }
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
    let sync_elapsed_ms = receipt["sync_elapsed_ms"].as_u64().expect("sync timing");
    assert!(u128::from(sync_elapsed_ms) <= elapsed_ms);
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
        assert_eq!(receipt["generated_assets_reused"], 0);
        assert_eq!(receipt["generated_assets_built"], 2);
        assert_eq!(receipt["working_tree"], "clean");
        assert_eq!(receipt["ahead"], 0);
        assert_eq!(receipt["behind"], 0);
        println!(
            "PERF_RELEASE {}",
            serde_json::json!({"elapsed_ms": elapsed_ms, "receipt": receipt})
        );
    }
}

#[test]
#[ignore = "explicit R04 real artifact cache comparison using isolated local Git only"]
fn factory_artifact_cache_performance() {
    let source=Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().nth(2).unwrap();
    let nonce=SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let fixture=Fixture(std::env::temp_dir().join(format!("bf-artifact-perf-{}-{nonce}",std::process::id())));
    let mut measurements=Vec::new();
    for warm in [false,true] {
        let root=fixture.0.join(if warm {"warm"} else {"cold"});copy_tree(source,&root);fixture_development_checks(&root);
        let config_path=root.join(".codex/bridgeforge-version.json");
        let mut config:serde_json::Value=serde_json::from_slice(&fs::read(&config_path).unwrap()).unwrap();
        config["release_policy"]=serde_json::json!("explicit_release");fs::write(config_path,serde_json::to_vec_pretty(&config).unwrap()).unwrap();
        git(&root,&["init"]);git(&root,&["config","user.name","BridgeForge Cache Benchmark"]);
        git(&root,&["config","user.email","cache-benchmark@example.invalid"]);git(&root,&["config","core.autocrlf","false"]);
        git(&root,&["config","core.hooksPath",".git/hooks"]);git(&root,&["add","."]);git(&root,&["commit","-m","chore: cache baseline"]);
        git(&root,&["config","core.hooksPath",".githooks"]);
        let remote=fixture.0.join(if warm {"warm-origin.git"} else {"cold-origin.git"});
        git(&root,&["init","--bare",remote.to_str().unwrap()]);git(&root,&["remote","add","origin",remote.to_str().unwrap()]);git(&root,&["push","-u","origin","HEAD"]);
        let mut readme=fs::read(root.join("README.md")).unwrap();readme.extend_from_slice(b"\nR04 identical documentation-only release workload.\n");fs::write(root.join("README.md"),readme).unwrap();
        let cli=root.join(if cfg!(windows){".codex/bin/bridgeforge.exe"}else{".codex/bin/bridgeforge"});
        let before_version=fs::read(root.join("VERSION")).unwrap();
        let mut request=ProcessRequest::new(cli.as_os_str(),&root);
        request.args=["git-sync","--prepare-release","--message","perf: cache benchmark"].iter().map(Into::into).collect();request.timeout=Duration::from_secs(1800);
        let prepared=SystemProcessRunner.run(&request).unwrap();assert_eq!(prepared.code,0,"{}",String::from_utf8_lossy(&prepared.stderr));
        assert_eq!(fs::read(root.join("VERSION")).unwrap(),before_version);
        let cache=root.join(".runtime/bridgeforge-codex/build-artifact-cache");assert!(cache.join("owner.json").is_file(),"benchmark requires supported compiler inputs");
        if !warm {fs::remove_dir_all(&cache).unwrap();}
        // A corrupted prepared record must have no influence on actual cache reuse.
        fs::write(root.join(".runtime/bridgeforge-codex/release-preparation/current.json"),b"invalid prepared/audit").unwrap();
        request.args=["git-sync","--release","--message","perf: cache benchmark"].iter().map(Into::into).collect();
        let started=std::time::Instant::now();let output=SystemProcessRunner.run(&request).unwrap();let elapsed_ms=started.elapsed().as_millis();
        assert!(!output.timed_out && output.code==0,"{}",String::from_utf8_lossy(&output.stderr));
        let receipt:serde_json::Value=serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(receipt["generated_assets_reused"],if warm{2}else{0});assert_eq!(receipt["generated_assets_built"],if warm{0}else{2});
        assert_eq!(receipt["working_tree"],"clean");assert_eq!(receipt["ahead"],0);assert_eq!(receipt["behind"],0);
        let measurement=serde_json::json!({"cache":if warm{"warm"}else{"cold"},"elapsed_ms":elapsed_ms,"receipt":receipt});
        println!("R04_PERF {measurement}");measurements.push(measurement);
    }
    println!("R04_COMPARE {}",serde_json::to_string(&measurements).unwrap());
}
