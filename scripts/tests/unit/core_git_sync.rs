use super::*;
use crate::{ProcessOutput, ProcessRequest, SystemProcessRunner};
use std::cell::RefCell;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn linked_worktrees_share_sync_lock_and_release_it_on_drop() {
    let repo = real_repository("shared-lock");
    let linked = repo.0.join("linked");
    git_ok(
        &repo.0,
        &[
            "worktree",
            "add",
            "-b",
            "lock-linked",
            linked.to_str().unwrap(),
        ],
    );
    let main_git = Git {
        root: &repo.0,
        runner: &SystemProcessRunner,
    };
    let linked_git = Git {
        root: &linked,
        runner: &SystemProcessRunner,
    };
    let held = SyncLock::acquire(&main_git).unwrap();
    assert!(SyncLock::acquire(&linked_git).is_err());
    drop(held);
    drop(SyncLock::acquire(&linked_git).unwrap());
    assert!(repo.0.join(".git/bridgeforge-git-sync.lock").exists());
}

#[test]
fn held_lock_blocks_even_status_fetch_and_stash() {
    let repo = real_repository("early-lock");
    let held = SyncLock::acquire(&Git {
        root: &repo.0,
        runner: &SystemProcessRunner,
    })
    .unwrap();
    struct OnlyIdentity;
    impl ProcessRunner for OnlyIdentity {
        fn run(&self, request: &ProcessRequest) -> std::io::Result<ProcessOutput> {
            assert!(
                request.args.iter().any(|value| value == "rev-parse"),
                "unexpected command before lock: {:?}",
                request.args
            );
            SystemProcessRunner.run(request)
        }
    }
    let result = sync(&repo.0, &OnlyIdentity, GitSyncOptions::default());
    assert_eq!(result.code, 2);
    assert!(result.stderr.contains("already running"));
    drop(held);
}

struct RealRepository(PathBuf);

impl Drop for RealRepository {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn git_ok(root: &Path, args: &[&str]) {
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

fn real_repository(name: &str) -> RealRepository {
    let token = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "bridgeforge-git-sync-{name}-{}-{token}",
        std::process::id()
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

fn managed_contract(asset: &[u8]) -> Vec<u8> {
    serde_json::to_vec_pretty(&json!({
        "schema_version": 4,
        "release_version": "1.0.0",
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

fn managed_repository(name: &str) -> (RealRepository, PathBuf) {
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
    fs::write(repo.0.join(".codex/.bridgeforge_codex_version"), b"1.0.0\n").unwrap();
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

#[test]
fn distinct_push_target_receives_commits_even_when_upstream_has_parity() {
    let (repo, upstream) = managed_repository("distinct-push");
    let fork = repo.0.with_extension("fork.git");
    git_ok(
        &repo.0,
        &[
            "clone",
            "--bare",
            upstream.to_str().unwrap(),
            fork.to_str().unwrap(),
        ],
    );
    git_ok(&repo.0, &["remote", "add", "fork", fork.to_str().unwrap()]);
    fs::write(
        repo.0.join("tracked.txt"),
        b"new commit already on upstream\n",
    )
    .unwrap();
    git_ok(&repo.0, &["add", "tracked.txt"]);
    git_ok(&repo.0, &["commit", "-m", "advance upstream"]);
    git_ok(&repo.0, &["push", "origin", "HEAD"]);
    git_ok(&repo.0, &["config", "remote.pushDefault", "fork"]);
    git_ok(&repo.0, &["config", "push.default", "current"]);
    git_ok(&repo.0, &["fetch", "fork"]);
    let git = Git {
        root: &repo.0,
        runner: &SystemProcessRunner,
    };
    assert_eq!(git.ahead_behind().unwrap(), (0, 0));
    assert_eq!(git.ahead_behind_target("@{push}").unwrap(), (1, 0));
    let result = sync(&repo.0, &SystemProcessRunner, GitSyncOptions::default());
    assert_eq!(result.code, 0, "{}", result.stderr);
    let receipt = result.receipt.unwrap();
    assert_eq!(receipt["status"], "synced");
    assert_eq!(receipt["push_performed"], true);
    assert_eq!(git.ahead_behind_target("@{push}").unwrap(), (0, 0));
    let remote_head = git
        .required(&["ls-remote", "fork", "HEAD"], Duration::from_secs(10))
        .unwrap();
    assert!(remote_head.starts_with(receipt["commit"].as_str().unwrap()));
    fs::remove_dir_all(upstream).unwrap();
    fs::remove_dir_all(fork).unwrap();
}

struct FakeRunner {
    outputs: RefCell<Vec<ProcessOutput>>,
}

fn versioned_repository(name: &str, explicit: bool) -> (RealRepository, PathBuf) {
    let (repo, remote) = managed_repository(name);
    fs::write(repo.0.join("VERSION"), b"1.0.0\n").unwrap();
    fs::write(repo.0.join("CHANGELOG.md"), b"# Changelog\n").unwrap();
    fs::write(
        repo.0.join("Cargo.toml"),
        b"[package]\nname = \"sample\"\nversion = \"1.0.0\"\n",
    )
    .unwrap();
    fs::write(
        repo.0.join("Cargo.lock"),
        b"version = 4\n[[package]]\nname = \"sample\"\nversion = \"1.0.0\"\n",
    )
    .unwrap();
    let mut config = json!({"schema_version": 1, "manifests": ["Cargo.toml"]});
    if explicit {
        config["release_policy"] = json!("explicit_release");
    }
    fs::write(
        repo.0.join(".codex/bridgeforge-version.json"),
        serde_json::to_vec(&config).unwrap(),
    )
    .unwrap();
    install_development_check(&repo.0);
    git_ok(&repo.0, &["add", "."]);
    git_ok(&repo.0, &["commit", "-m", "chore: version baseline"]);
    git_ok(&repo.0, &["push"]);
    (repo, remote)
}

#[test]
fn development_readiness_reports_missing_config_and_version_drift_without_writes() {
    let (repo, remote) = versioned_repository("release-readiness", true);
    fs::remove_file(repo.0.join(".codex/development-checks.json")).unwrap();
    fs::write(repo.0.join("VERSION"), b"0.9.9\n").unwrap();
    let index = fs::read(repo.0.join(".git/index")).unwrap();
    let head = fs::read(repo.0.join(".git/HEAD")).unwrap();
    let outcome = development_status(&repo.0, &SystemProcessRunner);
    assert_eq!(outcome.code, crate::EXIT_BLOCKED);
    let receipt = outcome.receipt.unwrap();
    assert_eq!(receipt["status"], "setup-required");
    let blockers = receipt["blockers"].as_array().unwrap();
    assert_eq!(blockers.len(), 2);
    assert_eq!(blockers[0]["code"], "development-checks-missing");
    assert_eq!(blockers[1]["code"], "release-baseline-blocked");
    assert!(blockers[1]["reason"].as_str().unwrap().contains("VERSION differs"));
    assert_eq!(fs::read(repo.0.join(".git/index")).unwrap(), index);
    assert_eq!(fs::read(repo.0.join(".git/HEAD")).unwrap(), head);
    assert_eq!(fs::read(repo.0.join("VERSION")).unwrap(), b"0.9.9\n");
    assert!(!repo.0.join(".runtime").exists());
    assert!(!repo.0.join(".codex/development-checks.json").exists());
    fs::write(repo.0.join("VERSION"), b"1.0.0\n").unwrap();
    for bytes in [b"{bad".as_slice(), br#"{"schema":1,"checks":[]}"#.as_slice()] {
        fs::write(repo.0.join(".codex/development-checks.json"), bytes).unwrap();
        let outcome = development_status(&repo.0, &SystemProcessRunner);
        assert_eq!(outcome.code, crate::EXIT_BLOCKED);
        let value = outcome.receipt.unwrap();
        assert_eq!(value["setup"]["status"], "invalid-config");
        assert_eq!(fs::read(repo.0.join(".codex/development-checks.json")).unwrap(), bytes);
    }
    install_development_check(&repo.0);
    let outcome = development_status(&repo.0, &SystemProcessRunner);
    assert_eq!(outcome.code, 0);
    assert_eq!(outcome.receipt.unwrap()["status"], "not-prepared");
    fs::remove_dir_all(remote).unwrap();
}

#[test]
fn development_readiness_checks_current_manifests_and_locks_before_preparation() {
    let (repo, remote) = versioned_repository("native-readiness", true);
    let config = repo.0.join(".codex/development-checks.json");
    fs::remove_file(&config).unwrap();
    let manifest = repo.0.join("Cargo.toml");
    let lock = repo.0.join("Cargo.lock");
    let policy = repo.0.join(".codex/bridgeforge-version.json");
    let originals = [&manifest, &lock, &policy].map(|path| fs::read(path).unwrap());
    for mode in ["manifest", "lock", "manifest-list"] {
        for (path, bytes) in [&manifest, &lock, &policy].iter().zip(&originals) { fs::write(path, bytes).unwrap(); }
        match mode {
            "manifest" => fs::write(&manifest, b"[package]\nname=\"sample\"\nversion=\"0.9.9\"\n").unwrap(),
            "lock" => fs::write(&lock, b"version=4\n[[package]]\nname=\"sample\"\nversion=\"0.9.9\"\n").unwrap(),
            _ => fs::write(&policy, br#"{"schema_version":1,"manifests":["missing/Cargo.toml"]}"#).unwrap(),
        }
        let before = [&manifest, &lock, &policy].map(|path| fs::read(path).unwrap());
        let index = fs::read(repo.0.join(".git/index")).unwrap();
        let head = fs::read(repo.0.join(".git/HEAD")).unwrap();
        let outcome = development_status(&repo.0, &SystemProcessRunner);
        assert_eq!(outcome.code, crate::EXIT_BLOCKED, "{mode}");
        let value = outcome.receipt.unwrap();
        assert_eq!(value["blockers"].as_array().unwrap().len(), 2, "{mode}");
        assert_eq!(value["blockers"][1]["code"], "release-baseline-blocked");
        for (path, bytes) in [&manifest, &lock, &policy].iter().zip(&before) { assert_eq!(&fs::read(path).unwrap(), bytes); }
        assert_eq!(fs::read(repo.0.join(".git/index")).unwrap(), index);
        assert_eq!(fs::read(repo.0.join(".git/HEAD")).unwrap(), head);
        assert!(!config.exists());
        assert!(!repo.0.join(".runtime").exists());
    }
    fs::remove_dir_all(remote).unwrap();
}

#[test]
fn development_readiness_reports_unsupported_js_locks_before_preparation() {
    let (repo, remote) = versioned_repository("js-readiness", true);
    fs::remove_file(repo.0.join(".codex/development-checks.json")).unwrap();
    fs::write(repo.0.join(".codex/bridgeforge-version.json"), br#"{"schema_version":1,"manifests":["package.json"]}"#).unwrap();
    fs::write(repo.0.join("package.json"), br#"{"name":"sample","version":"1.0.0"}"#).unwrap();
    for name in ["pnpm-lock.yaml", "yarn.lock"] {
        let lock = repo.0.join(name);
        fs::write(&lock, b"project-owned unsupported lock\n").unwrap();
        let index = fs::read(repo.0.join(".git/index")).unwrap();
        let head = fs::read(repo.0.join(".git/HEAD")).unwrap();
        let value = development_status(&repo.0, &SystemProcessRunner).receipt.unwrap();
        assert_eq!(value["blockers"].as_array().unwrap().len(), 2);
        assert_eq!(value["blockers"][1]["reason"], format!("unsupported JavaScript lock file: {name}"));
        assert_eq!(fs::read(&lock).unwrap(), b"project-owned unsupported lock\n");
        assert_eq!(fs::read(repo.0.join(".git/index")).unwrap(), index);
        assert_eq!(fs::read(repo.0.join(".git/HEAD")).unwrap(), head);
        assert!(!repo.0.join(".runtime").exists());
        fs::remove_file(lock).unwrap();
    }
    fs::remove_dir_all(remote).unwrap();
}

#[test]
fn explicit_policy_syncs_twice_then_releases_clean_history_once() {
    let (repo, remote) = versioned_repository("explicit-flow", true);
    let names = ["VERSION", "Cargo.toml", "Cargo.lock", "CHANGELOG.md"];
    let before = names.map(|name| fs::read(repo.0.join(name)).unwrap());
    for message in ["feat: add feature", "fix: repair feature"] {
        fs::write(repo.0.join("tracked.txt"), message).unwrap();
        let outcome = sync(
            &repo.0,
            &SystemProcessRunner,
            GitSyncOptions {
                message: Some(message.into()),
                ..Default::default()
            },
        );
        assert_eq!(outcome.code, 0, "{}", outcome.stderr);
        assert_eq!(outcome.receipt.unwrap()["version_bumped"], false);
        for (name, bytes) in names.iter().zip(&before) {
            assert_eq!(&fs::read(repo.0.join(name)).unwrap(), bytes);
        }
    }
    let preview = crate::release::preview(&repo.0, "fix: release", &SystemProcessRunner);
    assert_eq!(preview.code, 0, "{}", preview.stderr);
    assert_eq!(preview.receipt.unwrap()["version_after"], "1.1.0");
    let outcome = sync(
        &repo.0,
        &SystemProcessRunner,
        GitSyncOptions {
            release: true,
            message: Some("fix: release".into()),
            ..Default::default()
        },
    );
    assert_eq!(outcome.code, 0, "{}", outcome.stderr);
    let receipt = outcome.receipt.unwrap();
    assert_eq!(receipt["version_before"], "1.0.0");
    assert_eq!(receipt["version_after"], "1.1.0");
    assert_eq!(receipt["ahead"], 0);
    assert_eq!(receipt["behind"], 0);
    assert_eq!(receipt["working_tree"], "clean");
    let log = fs::read_to_string(repo.0.join("CHANGELOG.md")).unwrap();
    assert!(log.contains("add feature") && log.contains("repair feature"));
    let outcome = sync(
        &repo.0,
        &SystemProcessRunner,
        GitSyncOptions {
            release: true,
            message: Some("feat!: no repeated release".into()),
            ..Default::default()
        },
    );
    assert_eq!(outcome.code, 0, "{}", outcome.stderr);
    let repeated = outcome.receipt.unwrap();
    assert_eq!(repeated["version_bumped"], false);
    assert_eq!(repeated["commit"], receipt["commit"]);
    assert_eq!(repeated["push_performed"], false);
    fs::remove_dir_all(remote).unwrap();
}

#[test]
fn ordinary_sync_never_bumps_even_with_missing_or_legacy_config() {
    for policy in ["missing-file", "missing-field", "per_commit", "explicit_release"] {
        let (repo, remote) = versioned_repository(policy, false);
        let path = repo.0.join(".codex/bridgeforge-version.json");
        if policy == "missing-file" {
            fs::remove_file(&path).unwrap();
        } else if policy != "missing-field" {
            fs::write(&path, serde_json::to_vec(&json!({
                "schema_version": 1, "release_policy": policy, "manifests": ["Cargo.toml"]
            })).unwrap()).unwrap();
        }
        git_ok(&repo.0, &["add", "."]);
        git_ok(&repo.0, &["commit", "--allow-empty", "-m", "chore: policy baseline"]);
        git_ok(&repo.0, &["push"]);
        let config_before = fs::read(&path).ok();
        let names = ["VERSION", "Cargo.toml", "Cargo.lock", "CHANGELOG.md"];
        let before = names.map(|name| fs::read(repo.0.join(name)).unwrap());
        fs::write(repo.0.join("tracked.txt"), b"change\n").unwrap();
        let outcome = sync(
            &repo.0,
            &SystemProcessRunner,
            GitSyncOptions {
                message: Some("fix: repair".into()),
                ..Default::default()
            },
        );
        assert_eq!(outcome.code, 0, "{policy}: {}", outcome.stderr);
        let receipt = outcome.receipt.unwrap();
        assert_eq!(receipt["release_policy"], "explicit_release");
        assert_eq!(receipt["release_requested"], false);
        assert_eq!(receipt["version_bumped"], false);
        assert_eq!(receipt["version_before"], "1.0.0");
        assert_eq!(receipt["version_after"], "1.0.0");
        assert_eq!(receipt["working_tree"], "clean");
        assert_eq!(receipt["ahead"], 0);
        assert_eq!(receipt["behind"], 0);
        for (name, bytes) in names.iter().zip(&before) {
            assert_eq!(&fs::read(repo.0.join(name)).unwrap(), bytes, "{policy}: {name}");
        }
        assert_eq!(fs::read(&path).ok(), config_before);
        let released = sync(&repo.0, &SystemProcessRunner, GitSyncOptions {
            release: true, message: Some("fix: release".into()), ..Default::default()
        });
        assert_eq!(released.code, 0, "{policy}: {}", released.stderr);
        assert_eq!(released.receipt.unwrap()["version_after"], "1.0.1");
        assert_eq!(fs::read(&path).ok(), config_before);
        fs::remove_dir_all(remote).unwrap();
    }
}

#[test]
fn explicit_release_rejected_hook_restores_native_versions_log_and_index() {
    let (repo, remote) = versioned_repository("explicit-rollback", true);
    fs::write(repo.0.join("tracked.txt"), b"uncommitted feature\n").unwrap();
    fs::write(repo.0.join(".git/hooks/pre-commit"), b"#!/bin/sh\nexit 1\n").unwrap();
    git_ok(&repo.0, &["status", "--porcelain=v1"]);
    let index = fs::read(repo.0.join(".git/index")).unwrap();
    let names = ["VERSION", "Cargo.toml", "Cargo.lock", "CHANGELOG.md"];
    let before = names.map(|name| fs::read(repo.0.join(name)).unwrap());
    let outcome = sync(
        &repo.0,
        &SystemProcessRunner,
        GitSyncOptions {
            release: true,
            message: Some("feat: new feature".into()),
            ..Default::default()
        },
    );
    assert_eq!(outcome.code, 2, "{}", outcome.stderr);
    assert!(
        outcome.stderr.contains("were rolled back"),
        "{}",
        outcome.stderr
    );
    for (name, bytes) in names.iter().zip(&before) {
        assert_eq!(&fs::read(repo.0.join(name)).unwrap(), bytes);
    }
    assert_eq!(fs::read(repo.0.join(".git/index")).unwrap(), index);
    assert_eq!(
        fs::read(repo.0.join("tracked.txt")).unwrap(),
        b"uncommitted feature\n"
    );
    fs::remove_dir_all(remote).unwrap();
}

fn install_development_check(root: &Path) {
    fs::write(root.join(".codex/development-checks.json"), br#"{"schema":1,"checks":[{"id":"diff","program":"git","args":["diff","--check"],"timeout_seconds":30}]}"#).unwrap();
}

fn install_record_documents(root: &Path) {
    install_development_check(root);
    let file = root.join(".codex/development-checks.json");
    let mut cfg: serde_json::Value = serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
    cfg["record_documents"] = json!(["doc/README.md", "doc/0_architecture/TODO-INDEX.md", "doc/1_delivery/", "doc/3_reference/"]);
    fs::write(file, serde_json::to_vec(&cfg).unwrap()).unwrap();
    fs::create_dir_all(root.join("doc/3_reference")).unwrap();
    fs::create_dir_all(root.join("doc/0_architecture")).unwrap();
    fs::write(root.join("doc/README.md"), b"---\ndelivery_layout: flat\n---\n# Docs\n").unwrap();
}

#[test]
fn preparation_record_documents_reuse_artifacts_but_bind_release_content() {
    let (repo, remote) = factory_repository("record-release");
    install_record_documents(&repo.0);
    let prepared = prepare_release(&repo.0, &GeneratedRunner { root: repo.0.clone(), mode: "ok" }, "fix: prepare", None);
    assert_eq!(prepared.code, 0, "{}", prepared.stderr);
    let key = prepared.receipt.unwrap()["validation_fingerprint"].clone();
    fs::write(repo.0.join("doc/3_reference/lessons.md"), b"# Lessons\nA tested solution.\n").unwrap();
    fs::write(repo.0.join("doc/0_architecture/TODO-INDEX.md"), b"# TODO\n- [x] Completed\n- [ ] Known remaining issue\n").unwrap();
    let status = development_status(&repo.0, &SystemProcessRunner).receipt.unwrap();
    assert_eq!(status["status"], "prepared");
    assert_eq!(status["validation_fingerprint"], key);
    assert_eq!(status["record_documents_changed"], true);
    assert_eq!(status["record_documents_check"], "passed");
    fs::write(repo.0.join("doc/3_reference/lessons.md"), b"# Changed after binding\n").unwrap();
    assert_eq!(development_status(&repo.0, &SystemProcessRunner).receipt.unwrap()["status"], "prepared");
    let record = repo.0.join("doc/1_delivery/sample/requirements_2026-10-09_sample.md");
    fs::create_dir_all(record.parent().unwrap()).unwrap();
    fs::write(repo.0.join("doc/README.md"), b"---\ndelivery_layout: flat\n---\n[sample](1_delivery/sample/requirements_2026-10-09_sample.md)\n").unwrap();
    fs::write(&record, b"---\nlifecycle: active\nvalidation_status: in_progress\n---\n# Task\n").unwrap();
    fs::write(&record, b"---\nlifecycle: invalid-lifecycle\nvalidation_status: in_progress\n---\n# Task\n").unwrap();
    assert_eq!(development_status(&repo.0, &SystemProcessRunner).receipt.unwrap()["status"], "stale");
    fs::write(&record, b"---\nlifecycle: completed\nvalidation_status: verified\n---\n# Task\n").unwrap();
    let result = sync(&repo.0, &GeneratedRunner { root: repo.0.clone(), mode: "ok" }, GitSyncOptions { release: true, message: Some("fix: release records".into()), ..Default::default() });
    assert_eq!(result.code, 0, "{}", result.stderr);
    let receipt = result.receipt.unwrap();
    assert_eq!(receipt["generated_assets_built"], 0);
    assert_eq!(receipt["generated_assets_reused"], 2);
    assert_eq!(receipt["working_tree"], "clean");
    let committed = Git { root: &repo.0, runner: &SystemProcessRunner }
        .required(&["show", "HEAD:doc/3_reference/lessons.md"], Duration::from_secs(30)).unwrap();
    assert!(committed.contains("Changed after binding"));
    fs::remove_dir_all(remote).unwrap();
}

#[test]
fn preparation_record_scope_does_not_exempt_instructions_code_or_invalid_documents() {
    let (repo, remote) = versioned_repository("record-boundaries", true);
    install_record_documents(&repo.0);
    let prepared = prepare_release(&repo.0, &SystemProcessRunner, "fix: prepare", None);
    assert_eq!(prepared.code, 0, "{}", prepared.stderr);
    for relative in ["doc/3_reference/AGENTS.md", "doc/3_reference/SKILL.md", "doc/3_reference/tool.rs", "doc/0_architecture/design.md", "skills/example/SKILL.md"] {
        let file = repo.0.join(relative);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(&file, b"new input\n").unwrap();
        assert_eq!(development_status(&repo.0, &SystemProcessRunner).receipt.unwrap()["status"], "stale", "{relative}");
        fs::remove_file(file).unwrap();
    }
    let doc = repo.0.join("doc/3_reference/invalid.md");
    fs::write(&doc, b"\xff\x00").unwrap();
    assert_eq!(development_status(&repo.0, &SystemProcessRunner).receipt.unwrap()["status"], "stale");
    fs::remove_file(&doc).unwrap();
    fs::write(repo.0.join("doc/README.md"), b"missing layout\n").unwrap();
    assert_eq!(development_status(&repo.0, &SystemProcessRunner).receipt.unwrap()["status"], "stale");
    fs::write(repo.0.join("doc/README.md"), b"---\ndelivery_layout: flat\n---\n# Docs\n").unwrap();
    let cfg_path = repo.0.join(".codex/development-checks.json");
    let cfg: serde_json::Value = serde_json::from_slice(&fs::read(&cfg_path).unwrap()).unwrap();
    for scope in ["skills/", "doc/", "doc/3_reference/../", "doc/3_reference/AGENTS.md"] {
        let mut bad = cfg.clone();
        bad["record_documents"] = json!([scope]);
        fs::write(&cfg_path, serde_json::to_vec(&bad).unwrap()).unwrap();
        assert_ne!(development_status(&repo.0, &SystemProcessRunner).code, 0, "{scope}");
    }
    let mut changed = cfg;
    changed["record_documents"] = json!(["doc/README.md"]);
    fs::write(&cfg_path, serde_json::to_vec(&changed).unwrap()).unwrap();
    assert_eq!(development_status(&repo.0, &SystemProcessRunner).receipt.unwrap()["status"], "stale");
    fs::remove_dir_all(remote).unwrap();
}

#[test]
fn independent_preparation_preserves_worktree_and_direct_release_reuses_cached_tools() {
    let (repo, remote) = factory_repository("prepared-factory");
    install_development_check(&repo.0);
    let before_version = fs::read(repo.0.join("VERSION")).unwrap();
    let before_index = fs::read(repo.0.join(".git/index")).unwrap();
    let prepared = prepare_release(
        &repo.0,
        &GeneratedRunner {
            root: repo.0.clone(),
            mode: "ok",
        },
        "fix: prepare",
        None,
    );
    assert_eq!(prepared.code, 0, "{}", prepared.stderr);
    assert_eq!(fs::read(repo.0.join("VERSION")).unwrap(), before_version);
    assert_eq!(fs::read(repo.0.join(".git/index")).unwrap(), before_index);
    assert_eq!(prepared.receipt.unwrap()["checks_passed"], 1);
    let result = sync(
        &repo.0,
        &GeneratedRunner {
            root: repo.0.clone(),
            mode: "ok",
        },
        GitSyncOptions {
            release: true,
            message: Some("fix: new description".into()),
            ..Default::default()
        },
    );
    assert_eq!(result.code, 0, "{}", result.stderr);
    let receipt = result.receipt.unwrap();
    assert_eq!(receipt["version_after"], "1.0.1");
    assert_eq!(receipt["generated_assets_built"], 0);
    assert_eq!(receipt["generated_assets_reused"], 2);
    assert!(
        fs::read_to_string(repo.0.join("CHANGELOG.md"))
            .unwrap()
            .contains("new description")
    );
    fs::remove_dir_all(remote).unwrap();
}

#[test]
fn direct_release_ignores_missing_invalid_failed_or_stale_development_state() {
    for mode in ["missing", "invalid", "failed", "stale"] {
        let (repo, remote) = versioned_repository(mode, true);
        let config = repo.0.join(".codex/development-checks.json");
        if mode == "missing" { fs::remove_file(&config).unwrap(); }
        else if mode == "invalid" { fs::write(&config, b"not-json\n").unwrap(); }
        else { fs::write(&config, br#"{"schema":1,"audit_required":true,"checks":[{"id":"must-not-run","program":"not-a-real-test-program","args":[],"timeout_seconds":1}]}"#).unwrap(); }
        let before_config = fs::read(&config).ok();
        let record = repo.0.join(".runtime/bridgeforge-codex/release-preparation/current.json");
        if mode != "missing" {
            fs::create_dir_all(record.parent().unwrap()).unwrap();
            fs::write(&record, br#"{"schema":1,"status":"failed","root":"another repository","target_version":"9.9.9"}"#).unwrap();
        }
        let before_record = fs::read(&record).ok();
        fs::write(repo.0.join("tracked.txt"), b"new business change\n").unwrap();
        let state = release_status(&repo.0, &SystemProcessRunner);
        assert_eq!(state.code, 0, "{mode}: {}", state.stderr);
        assert_eq!(state.receipt.unwrap()["status"], "release-ready");
        let result = sync(&repo.0, &SystemProcessRunner, GitSyncOptions {
            release: true, message: Some("fix: direct release".into()), ..Default::default()
        });
        assert_eq!(result.code, 0, "{mode}: {}", result.stderr);
        let receipt = result.receipt.unwrap();
        assert_eq!(receipt["version_before"], "1.0.0");
        assert_eq!(receipt["version_after"], "1.0.1");
        assert_eq!(receipt["working_tree"], "clean");
        assert_eq!(receipt["ahead"], 0);
        assert_eq!(receipt["behind"], 0);
        assert_eq!(fs::read(&config).ok(), before_config);
        assert_eq!(fs::read(&record).ok(), before_record);
        fs::remove_dir_all(remote).unwrap();
    }
}

#[test]
fn preparation_accepts_only_status_metadata_changes_and_checks_new_file_inventory() {
    let (repo, remote) = versioned_repository("record-metadata", true);
    let record = repo.0.join("doc/1_delivery/sample/requirements.md");
    fs::create_dir_all(record.parent().unwrap()).unwrap();
    fs::write(
        &record,
        b"---\nlifecycle: active\nvalidation_status: in_progress\n---\nBody\n",
    )
    .unwrap();
    fs::write(repo.0.join("tracked.txt"), b"new\n").unwrap();
    let result = prepare_release(&repo.0, &SystemProcessRunner, "fix: prepare", None);
    assert_eq!(result.code, 0, "{}", result.stderr);
    fs::write(
        &record,
        b"---\nlifecycle: completed\nvalidation_status: verified\n---\nBody\n",
    )
    .unwrap();
    assert_eq!(
        development_status(&repo.0, &SystemProcessRunner)
            .receipt
            .unwrap()["status"],
        "prepared"
    );
    fs::write(repo.0.join("new-unverified.txt"), b"new file\n").unwrap();
    assert_eq!(
        development_status(&repo.0, &SystemProcessRunner)
            .receipt
            .unwrap()["status"],
        "stale"
    );
    fs::remove_file(repo.0.join("new-unverified.txt")).unwrap();
    fs::write(
        &record,
        b"---\nlifecycle: completed\nvalidation_status: verified\n---\nChanged body\n",
    )
    .unwrap();
    assert_eq!(
        development_status(&repo.0, &SystemProcessRunner)
            .receipt
            .unwrap()["status"],
        "stale"
    );
    fs::remove_dir_all(remote).unwrap();
}

#[test]
fn preparation_failed_check_and_mismatched_audit_never_certify() {
    let (repo, remote) = versioned_repository("failed-quality", true);
    fs::write(repo.0.join("tracked.txt"), b"new\n").unwrap();
    fs::write(repo.0.join(".codex/development-checks.json"), br#"{"schema":1,"audit_required":true,"checks":[{"id":"diff","program":"git","args":["diff","--exit-code"],"timeout_seconds":30}]}"#).unwrap();
    let no_audit = prepare_release(&repo.0, &SystemProcessRunner, "fix: prepare", None);
    assert_eq!(no_audit.code, 2);
    let key = development_status(&repo.0, &SystemProcessRunner)
        .receipt
        .unwrap()["validation_fingerprint"]
        .clone();
    fs::create_dir_all(repo.0.join(".runtime")).unwrap();
    let audit = repo.0.join(".runtime/audit.json");
    fs::write(&audit, serde_json::to_vec(&json!({"schema":1,"fingerprint":"wrong","reviewer":"review-auditor","verdict":"passed","summary":"fixture"})).unwrap()).unwrap();
    assert_eq!(
        prepare_release(&repo.0, &SystemProcessRunner, "fix: prepare", Some(&audit)).code,
        2
    );
    fs::write(&audit, serde_json::to_vec(&json!({"schema":1,"fingerprint":key,"reviewer":"review-auditor","verdict":"passed","summary":"fixture"})).unwrap()).unwrap();
    let failed = prepare_release(&repo.0, &SystemProcessRunner, "fix: prepare", Some(&audit));
    assert_eq!(failed.code, 2);
    assert!(
        failed.stderr.contains("development check diff failed"),
        "{}",
        failed.stderr
    );
    assert_ne!(
        development_status(&repo.0, &SystemProcessRunner)
            .receipt
            .unwrap()["status"],
        "prepared"
    );
    fs::remove_dir_all(remote).unwrap();
}

#[test]
fn preparation_corrupt_artifact_and_foreign_record_cannot_be_consumed() {
    let (repo, remote) = factory_repository("bad-artifact");
    install_development_check(&repo.0);
    let result = prepare_release(
        &repo.0,
        &GeneratedRunner {
            root: repo.0.clone(),
            mode: "ok",
        },
        "fix: prepare",
        None,
    );
    assert_eq!(result.code, 0, "{}", result.stderr);
    let file = repo
        .0
        .join(".runtime/bridgeforge-codex/release-preparation/current.json");
    let mut record: serde_json::Value = serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
    let original = record.clone();
    let mut incomplete = original.clone();
    incomplete["artifacts"].as_object_mut().unwrap().clear();
    fs::write(&file, serde_json::to_vec(&incomplete).unwrap()).unwrap();
    assert_eq!(
        development_status(&repo.0, &SystemProcessRunner)
            .receipt
            .unwrap()["status"],
        "stale"
    );
    record["root"] = json!("another repository");
    fs::write(&file, serde_json::to_vec(&record).unwrap()).unwrap();
    assert_eq!(
        development_status(&repo.0, &SystemProcessRunner)
            .receipt
            .unwrap()["status"],
        "stale"
    );
    fs::write(&file, serde_json::to_vec(&original).unwrap()).unwrap();
    let hash = original["artifacts"]
        .as_object()
        .unwrap()
        .values()
        .next()
        .unwrap()
        .as_str()
        .unwrap()
        .strip_prefix("sha256:")
        .unwrap();
    fs::write(
        repo.0.join(format!(
            ".runtime/bridgeforge-codex/release-preparation/blobs/{hash}"
        )),
        b"corrupt",
    )
    .unwrap();
    assert_eq!(
        development_status(&repo.0, &SystemProcessRunner)
            .receipt
            .unwrap()["status"],
        "stale"
    );
    fs::remove_dir_all(remote).unwrap();
}

#[test]
fn direct_release_uses_current_deletion_or_reappearance_without_development_gate() {
    for reappear in [false, true] {
        let (repo, remote) = versioned_repository("prepared-deletion", true);
        fs::remove_file(repo.0.join("tracked.txt")).unwrap();
        let result = prepare_release(
            &repo.0,
            &SystemProcessRunner,
            "fix: remove obsolete file",
            None,
        );
        assert_eq!(result.code, 0, "{}", result.stderr);
        if reappear {
            fs::write(repo.0.join("tracked.txt"), b"not validated\n").unwrap();
        }
        let result = sync(
            &repo.0,
            &SystemProcessRunner,
            GitSyncOptions {
                release: true,
                message: Some("fix: remove obsolete file".into()),
                ..Default::default()
            },
        );
        assert_eq!(
            result.code,
            0,
            "{}",
            result.stderr
        );
        if !reappear {
            assert!(!repo.0.join("tracked.txt").exists());
        } else {
            assert_eq!(
                fs::read(repo.0.join("tracked.txt")).unwrap(),
                b"not validated\n"
            );
        }
        fs::remove_dir_all(remote).unwrap();
    }
}

#[test]
fn independent_preparation_rejects_contract_drift_without_gating_git() {
    let (repo, remote) = factory_repository("development-contract-drift");
    install_development_check(&repo.0);
    let ready = prepare_release(&repo.0, &GeneratedRunner { root:repo.0.clone(), mode:"ok" }, "fix: prepare", None);
    assert_eq!(ready.code, 0, "{}", ready.stderr);
    let file = repo.0.join("templates/managed-skeleton.json");
    let mut contract: serde_json::Value = serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
    contract["external_marker"] = json!(true);
    fs::write(&file, serde_json::to_vec(&contract).unwrap()).unwrap();
    let state = development_status(&repo.0, &SystemProcessRunner).receipt.unwrap();
    assert_eq!(state["status"], "stale");
    assert_eq!(fs::read(repo.0.join("VERSION")).unwrap(), b"1.0.0\n");
    assert_eq!(serde_json::from_slice::<serde_json::Value>(&fs::read(file).unwrap()).unwrap()["external_marker"], true);
    fs::remove_dir_all(remote).unwrap();
}

#[test]
fn all_sync_modes_block_hook_rewrites_and_later_ordinary_retry() {
    for release in [false, true] {
    let (repo, remote) = versioned_repository("prepared-hook-rewrite", true);
    git_ok(&repo.0, &["config", "core.hooksPath", ".git/hooks"]);
    fs::write(repo.0.join("tracked.txt"), b"validated change\n").unwrap();
    let hook = repo.0.join(".git/hooks/pre-commit");
    fs::write(&hook, b"#!/bin/sh\nprintf 'unverified hook change\\n' > tracked.txt\ngit add tracked.txt\nexit 0\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&hook, fs::Permissions::from_mode(0o755)).unwrap();
    }
    let result = sync(
        &repo.0,
        &SystemProcessRunner,
        GitSyncOptions {
            release,
            message: Some("fix: release".into()),
            ..Default::default()
        },
    );
    assert_eq!(result.code, 2, "{}", result.stderr);
    assert!(
        result
            .stderr
            .contains("commit hook changed verified content"),
        "{}",
        result.stderr
    );
    let git = Git {
        root: &repo.0,
        runner: &SystemProcessRunner,
    };
    assert_eq!(git.ahead_behind().unwrap(), (1, 0));
    assert_eq!(
        fs::read(repo.0.join("tracked.txt")).unwrap(),
        b"unverified hook change\n"
    );
    let retry = sync(&repo.0, &SystemProcessRunner, GitSyncOptions::default());
    assert_eq!(retry.code, 2);
    assert!(retry.stderr.contains("manual review required"));
    assert_eq!(git.ahead_behind().unwrap(), (1, 0));
    fs::remove_dir_all(remote).unwrap();
    }
}

#[test]
fn preparation_check_cannot_silently_stage_current_repository() {
    let (repo, remote) = versioned_repository("check-index-change", true);
    fs::write(repo.0.join("tracked.txt"), b"new\n").unwrap();
    fs::write(repo.0.join(".codex/development-checks.json"), br#"{"schema":1,"checks":[{"id":"bad-check","program":"git","args":["add","tracked.txt"],"timeout_seconds":30}]}"#).unwrap();
    let result = prepare_release(&repo.0, &SystemProcessRunner, "fix: prepare", None);
    assert_eq!(result.code, 2);
    assert!(
        result.stderr.contains("changed Git state"),
        "{}",
        result.stderr
    );
    assert_ne!(
        development_status(&repo.0, &SystemProcessRunner)
            .receipt
            .unwrap()["status"],
        "prepared"
    );
    fs::remove_dir_all(remote).unwrap();
}

impl ProcessRunner for FakeRunner {
    fn run(&self, _: &ProcessRequest) -> std::io::Result<ProcessOutput> {
        Ok(self.outputs.borrow_mut().remove(0))
    }
}

fn out(code: i32, stdout: &str) -> ProcessOutput {
    ProcessOutput {
        code,
        stdout: stdout.as_bytes().to_vec(),
        stderr: Vec::new(),
        timed_out: false,
    }
}

#[test]
fn clean_parity_has_stable_receipt() {
    let repo = real_repository("fake-clean-parity");
    let runner = FakeRunner {
        outputs: RefCell::new(vec![
            out(0, ".git"),
            out(0, &repo.0.join(".git").to_string_lossy()),
            out(0, "origin/main"),
            out(0, "origin/main"),
            out(0, ""),
            out(0, "0 0"),
            out(0, "0 0"),
            out(0, ""),
            out(0, "0 0"),
            out(0, "abc"),
        ]),
    };
    let result = sync(
        Path::new("."),
        &runner,
        GitSyncOptions {
            remote: "origin".into(),
            skip_fetch: true,
            ..GitSyncOptions::default()
        },
    );
    assert_eq!(result.code, 0);
    assert_eq!(result.receipt.unwrap()["status"], "synced");
}

#[test]
fn push_failure_is_blocking_and_never_claims_synced() {
    let repo = real_repository("push-failure");
    let remote = repo.0.with_file_name(format!(
        "{}-remote.git",
        repo.0.file_name().unwrap().to_string_lossy()
    ));
    git_ok(
        &repo.0,
        &["init", "--bare", remote.to_string_lossy().as_ref()],
    );
    git_ok(
        &repo.0,
        &["remote", "add", "origin", remote.to_string_lossy().as_ref()],
    );
    git_ok(&repo.0, &["push", "-u", "origin", "HEAD"]);
    fs::write(repo.0.join("ahead.txt"), b"ahead\n").unwrap();
    git_ok(&repo.0, &["add", "ahead.txt"]);
    git_ok(&repo.0, &["commit", "-m", "ahead"]);
    fs::write(repo.0.join(".git/hooks/pre-push"), b"#!/bin/sh\nexit 1\n").unwrap();
    let result = sync(
        &repo.0,
        &SystemProcessRunner,
        GitSyncOptions {
            remote: "origin".into(),
            skip_fetch: true,
            ..GitSyncOptions::default()
        },
    );
    assert_eq!(result.code, 2);
    assert!(result.stderr.contains("git push failed"));
    assert!(result.receipt.is_none());
    fs::remove_dir_all(remote).unwrap();
}

#[test]
fn clean_ahead_push_identity_drift_never_claims_synced() {
    let repo = real_repository("clean-push-drift");
    let remote = repo.0.with_file_name(format!(
        "{}-remote.git",
        repo.0.file_name().unwrap().to_string_lossy()
    ));
    git_ok(
        &repo.0,
        &["init", "--bare", remote.to_string_lossy().as_ref()],
    );
    git_ok(
        &repo.0,
        &["remote", "add", "origin", remote.to_string_lossy().as_ref()],
    );
    git_ok(&repo.0, &["push", "-u", "origin", "HEAD"]);
    fs::write(repo.0.join("ahead.txt"), b"ahead\n").unwrap();
    git_ok(&repo.0, &["add", "ahead.txt"]);
    git_ok(&repo.0, &["commit", "-m", "ahead"]);
    fs::write(
        repo.0.join(".git/hooks/pre-push"),
        b"#!/bin/sh\ngit config bridgeforge.audit changed\nexit 0\n",
    )
    .unwrap();
    let result = sync(
        &repo.0,
        &SystemProcessRunner,
        GitSyncOptions {
            remote: "origin".into(),
            skip_fetch: true,
            ..GitSyncOptions::default()
        },
    );
    assert_eq!(result.code, 2);
    assert!(result.stderr.contains("after successful push"));
    assert!(result.receipt.is_none());
    fs::remove_dir_all(remote).unwrap();
}

#[test]
fn failed_autostash_pop_returns_a_retained_stash_receipt() {
    let repo = real_repository("fake-autostash");
    let runner = FakeRunner {
        outputs: RefCell::new(vec![
            out(0, ".git"),
            out(0, &repo.0.join(".git").to_string_lossy()),
            out(0, "origin/main"),
            out(0, "origin/main"),
            out(0, " M local.txt"),
            out(0, "0 1"),
            out(0, "Saved working directory"),
            out(0, "Fast-forward"),
            out(1, "conflict"),
        ]),
    };
    let result = sync(
        Path::new("."),
        &runner,
        GitSyncOptions {
            remote: "origin".into(),
            skip_fetch: true,
            ..GitSyncOptions::default()
        },
    );
    assert_eq!(result.code, 2);
    assert_eq!(
        result.receipt.as_ref().unwrap()["status"],
        "autostash-retained"
    );
    assert_eq!(result.receipt.as_ref().unwrap()["autostash_retained"], true);
}

#[test]
fn split_index_is_blocked_before_transactional_sync() {
    let repo = real_repository("split-index");
    git_ok(&repo.0, &["config", "core.splitIndex", "true"]);
    git_ok(&repo.0, &["update-index", "--split-index"]);
    let git = Git {
        root: &repo.0,
        runner: &SystemProcessRunner,
    };
    let error = RepositoryIdentity::capture(&git).unwrap_err();
    assert!(error.contains("split index is not supported"));
}

#[test]
fn configured_split_index_is_blocked_before_shared_index_exists() {
    let error = verify_split_index_disabled("", "true").unwrap_err();
    assert!(error.contains("core.splitIndex=true"));
}

#[test]
fn linked_worktree_uses_its_own_index_and_common_identity() {
    let repo = real_repository("linked-worktree");
    let linked = repo.0.join("linked");
    git_ok(
        &repo.0,
        &[
            "worktree",
            "add",
            "-b",
            "linked-test",
            linked.to_string_lossy().as_ref(),
        ],
    );
    let git = Git {
        root: &linked,
        runner: &SystemProcessRunner,
    };
    let identity = RepositoryIdentity::capture(&git).unwrap();
    assert_ne!(identity.git_dir, identity.common_dir);
    assert!(identity.index_path.is_file());
    assert!(identity.index_path.to_string_lossy().contains("worktrees"));
}

#[test]
fn repository_config_drift_blocks_automatic_restore() {
    let repo = real_repository("config-drift");
    let git = Git {
        root: &repo.0,
        runner: &SystemProcessRunner,
    };
    let identity = RepositoryIdentity::capture(&git).unwrap();
    let original_index = fs::read(&identity.index_path).unwrap();
    let marker = repo.0.join("automatic.txt");
    fs::write(&marker, b"new\n").unwrap();
    git_ok(&repo.0, &["config", "bridgeforge.audit", "changed"]);
    let error = restore_snapshots_guarded(
        &git,
        &identity,
        &original_index,
        original_index.clone(),
        vec![FileSnapshot {
            binary: false,
            path: marker.clone(),
            before: Some(b"old\n".to_vec()),
            planned: vec![Some(b"new\n".to_vec())],
        }],
    )
    .unwrap_err();
    assert!(error.contains("HIGH: repository identity drift"));
    assert_eq!(fs::read(marker).unwrap(), b"new\n");
}

#[test]
fn concurrent_index_change_blocks_automatic_restore() {
    let repo = real_repository("index-drift");
    let git = Git {
        root: &repo.0,
        runner: &SystemProcessRunner,
    };
    let identity = RepositoryIdentity::capture(&git).unwrap();
    let original_index = fs::read(&identity.index_path).unwrap();
    let marker = repo.0.join("automatic.txt");
    fs::write(&marker, b"new\n").unwrap();
    fs::write(repo.0.join("foreign.txt"), b"concurrent change\n").unwrap();
    git_ok(&repo.0, &["add", "foreign.txt"]);
    let error = restore_snapshots_guarded(
        &git,
        &identity,
        &original_index,
        original_index.clone(),
        vec![FileSnapshot {
            binary: false,
            path: marker.clone(),
            before: Some(b"old\n".to_vec()),
            planned: vec![Some(b"new\n".to_vec())],
        }],
    )
    .unwrap_err();
    assert!(error.contains("HIGH: Git index changed concurrently"));
    assert_eq!(fs::read(marker).unwrap(), b"new\n");
}

#[test]
fn concurrent_automatic_target_change_blocks_restore() {
    let repo = real_repository("automatic-target-drift");
    let git = Git {
        root: &repo.0,
        runner: &SystemProcessRunner,
    };
    let identity = RepositoryIdentity::capture(&git).unwrap();
    let original_index = fs::read(&identity.index_path).unwrap();
    let marker = repo.0.join("automatic.txt");
    fs::write(&marker, b"foreign\n").unwrap();
    let error = restore_snapshots_guarded(
        &git,
        &identity,
        &original_index,
        original_index.clone(),
        vec![FileSnapshot {
            binary: false,
            path: marker.clone(),
            before: Some(b"old\n".to_vec()),
            planned: vec![Some(b"planned\n".to_vec())],
        }],
    )
    .unwrap_err();
    assert!(error.contains("HIGH: automatic target changed concurrently"));
    assert_eq!(fs::read(marker).unwrap(), b"foreign\n");
}

#[test]
fn concurrent_head_change_blocks_restore() {
    let repo = real_repository("head-drift");
    let git = Git {
        root: &repo.0,
        runner: &SystemProcessRunner,
    };
    let identity = RepositoryIdentity::capture(&git).unwrap();
    let original_index = fs::read(&identity.index_path).unwrap();
    fs::write(repo.0.join("concurrent.txt"), b"commit\n").unwrap();
    git_ok(&repo.0, &["add", "concurrent.txt"]);
    git_ok(&repo.0, &["commit", "-m", "concurrent"]);
    let current_index = fs::read(&identity.index_path).unwrap();
    let error =
        restore_snapshots_guarded(&git, &identity, &current_index, original_index, Vec::new())
            .unwrap_err();
    assert!(error.contains("HIGH: repository identity drift"));
}

#[test]
fn rejected_commit_hook_restores_index_but_preserves_user_worktree() {
    let (repo, remote) = managed_repository("rejected-hook");
    fs::write(repo.0.join(".git/hooks/pre-commit"), b"#!/bin/sh\nexit 1\n").unwrap();
    fs::write(repo.0.join("managed.txt"), b"new\n").unwrap();
    fs::write(
        repo.0.join(".codex/managed-skeleton.json"),
        managed_contract(b"new\n"),
    )
    .unwrap();
    let adaptation = repo
        .0
        .join(".runtime/bridgeforge-codex/explicit-adaptation.json");
    fs::create_dir_all(adaptation.parent().unwrap()).unwrap();
    fs::write(&adaptation, b"legacy receipt\n").unwrap();
    git_ok(&repo.0, &["status", "--porcelain=v1"]);
    let original_index = fs::read(repo.0.join(".git/index")).unwrap();
    let outcome = sync(
        &repo.0,
        &SystemProcessRunner,
        GitSyncOptions {
            remote: "origin".into(),
            skip_fetch: true,
            skip_push: true,
            message: Some("chore: verify rejected hook recovery".into()),
            message_file: None,
            release: false,
        },
    );
    assert_eq!(outcome.code, 2, "{}", outcome.stderr);
    assert!(
        outcome.stderr.contains("Git index were rolled back"),
        "{}",
        outcome.stderr
    );
    assert_eq!(fs::read(repo.0.join(".git/index")).unwrap(), original_index);
    assert_eq!(fs::read(repo.0.join("managed.txt")).unwrap(), b"new\n");
    let staged = Command::new("git")
        .args(["diff", "--cached", "--quiet"])
        .current_dir(&repo.0)
        .status()
        .unwrap();
    assert!(staged.success());
    assert!(adaptation.is_file());
    fs::remove_dir_all(remote).unwrap();
}

#[test]
fn dirty_commit_push_identity_drift_never_claims_synced() {
    let (repo, remote) = managed_repository("dirty-push-drift");
    let adaptation = repo
        .0
        .join(".runtime/bridgeforge-codex/explicit-adaptation.json");
    fs::create_dir_all(adaptation.parent().unwrap()).unwrap();
    fs::write(&adaptation, b"legacy receipt\n").unwrap();
    fs::write(repo.0.join("managed.txt"), b"new\n").unwrap();
    fs::write(
        repo.0.join(".codex/managed-skeleton.json"),
        managed_contract(b"new\n"),
    )
    .unwrap();
    fs::write(
        repo.0.join(".git/hooks/pre-push"),
        b"#!/bin/sh\ngit config bridgeforge.audit changed\nexit 0\n",
    )
    .unwrap();
    let outcome = sync(
        &repo.0,
        &SystemProcessRunner,
        GitSyncOptions {
            remote: "origin".into(),
            skip_fetch: true,
            skip_push: false,
            message: Some("chore: update managed skeleton".into()),
            message_file: None,
            release: false,
        },
    );
    assert_eq!(outcome.code, 2, "{}", outcome.stderr);
    assert!(outcome.stderr.contains("after successful push"));
    assert!(outcome.receipt.is_none());
    assert!(!adaptation.exists());
    fs::remove_dir_all(remote).unwrap();
}

fn factory_repository(name: &str) -> (RealRepository, PathBuf) {
    let (repo, remote) = managed_repository(name);
    fs::remove_file(repo.0.join(".codex/.bridgeforge_codex_version")).unwrap();
    fs::create_dir_all(repo.0.join("templates/hooks")).unwrap();
    fs::create_dir_all(repo.0.join("templates/user")).unwrap();
    fs::write(
        repo.0.join(crate::user_agents::SOURCE),
        b"user instructions\n",
    )
    .unwrap();
    fs::create_dir_all(repo.0.join(".codex/hooks")).unwrap();
    fs::write(repo.0.join("templates/managed.txt"), b"old\n").unwrap();
    fs::write(
        repo.0.join("templates/managed-skeleton.json"),
        managed_contract(b"old\n"),
    )
    .unwrap();
    fs::write(
        repo.0.join("bridgeforge-codex-manifest.json"),
        br#"{"platforms":{"windows":{"skills":[]}}}"#,
    )
    .unwrap();
    fs::write(repo.0.join("VERSION"), b"1.0.0\n").unwrap();
    fs::write(repo.0.join("CHANGELOG.md"), b"# Changelog\n").unwrap();
    fs::write(repo.0.join(".codex/bridgeforge-version.json"), br#"{"schema_version":1,"manifests":["templates/hooks/Cargo.toml",".codex/hooks/Cargo.toml"]}"#).unwrap();
    for base in ["templates/hooks", ".codex/hooks"] {
        fs::write(
            repo.0.join(base).join("Cargo.toml"),
            b"[package]\nname = \"bridgeforge-hook\"\nversion = \"1.0.0\"\n",
        )
        .unwrap();
        fs::write(
            repo.0.join(base).join("Cargo.lock"),
            b"version = 4\n[[package]]\nname = \"bridgeforge-hook\"\nversion = \"1.0.0\"\n",
        )
        .unwrap();
    }
    crate::manifest::rebuild(&repo.0, false).unwrap();
    git_ok(&repo.0, &["add", "."]);
    git_ok(&repo.0, &["commit", "-m", "factory baseline"]);
    fs::write(repo.0.join("tracked.txt"), b"user change\n").unwrap();
    (repo, remote)
}

struct GeneratedRunner {
    root: PathBuf,
    mode: &'static str,
}
impl ProcessRunner for GeneratedRunner {
    fn run(&self, request: &ProcessRequest) -> std::io::Result<ProcessOutput> {
        if request.args.first().is_some_and(|arg|arg=="--version") &&
            (request.program=="cargo" || request.program=="rustc") {
            return Ok(out(0,&format!("{} 1.88.0 (fixture)",request.program.to_string_lossy())));
        }
        if self.mode == "policy-drift" && request.args.iter().any(|arg| arg == "check-ignore") {
            fs::write(self.root.join(".codex/bridgeforge-version.json"), b"external policy edit\n")?;
        }
        if request.program == "cargo" {
            assert_eq!(
                fs::read(self.root.join("VERSION")).unwrap(),
                b"1.0.0\n",
                "build must precede repository writes"
            );
            if self.mode == "fail" {
                return Ok(out(1, ""));
            }
            let args = request
                .args
                .iter()
                .map(|p| p.to_string_lossy().into_owned())
                .collect::<Vec<_>>();
            let output =
                PathBuf::from(&args[args.iter().position(|p| p == "--target-dir").unwrap() + 1]);
            let binary = &args[args.iter().position(|p| p == "--bin").unwrap() + 1];
            fs::create_dir_all(output.join("release"))?;
            fs::write(
                output.join("release").join(if cfg!(windows) {
                    format!("{binary}.exe")
                } else {
                    binary.clone()
                }),
                format!("new {binary}"),
            )?;
            if self.mode == "drift" {
                fs::write(
                    self.root.join("templates/hooks/Cargo.lock"),
                    b"external edit\n",
                )?;
            }
            if self.mode == "user-agents-drift" {
                fs::write(
                    self.root.join(crate::user_agents::SOURCE),
                    b"external user template edit\n",
                )?;
            }
            return Ok(out(0, ""));
        }
        if request.args.first().is_some_and(|arg| arg == "self-test") {
            let name = if request
                .program
                .to_string_lossy()
                .contains("bridgeforge-hook")
            {
                "bridgeforge-hook"
            } else {
                "bridgeforge"
            };
            return Ok(out(
                0,
                &json!({"schema":1,"name":name,"status":"ok"}).to_string(),
            ));
        }
        SystemProcessRunner.run(request)
    }
}

#[test]
fn factory_build_failure_and_input_drift_do_not_apply_release() {
    for mode in ["fail", "drift", "policy-drift", "user-agents-drift"] {
        let (repo, remote) = factory_repository(mode);
        let manifest = fs::read(repo.0.join("templates/managed-skeleton.json")).unwrap();
        let user_manifest = fs::read(repo.0.join(crate::user_agents::MANIFEST)).unwrap();
        let outcome = sync(
            &repo.0,
            &GeneratedRunner {
                root: repo.0.clone(),
                mode,
            },
            GitSyncOptions {
                message: Some("fix: generated planning".into()),
                skip_fetch: true,
                skip_push: true,
                ..Default::default()
            },
        );
        assert_eq!(outcome.code, 2, "{}", outcome.stderr);
        assert!(
            outcome.stderr.contains("before apply"),
            "{}",
            outcome.stderr
        );
        assert_eq!(fs::read(repo.0.join("VERSION")).unwrap(), b"1.0.0\n");
        assert_eq!(
            fs::read(repo.0.join("templates/managed-skeleton.json")).unwrap(),
            manifest
        );
        assert!(!repo.0.join(".codex/bin/build-receipt-cli.json").exists());
        assert_eq!(
            fs::read(repo.0.join(crate::user_agents::MANIFEST)).unwrap(),
            user_manifest
        );
        if mode == "user-agents-drift" {
            assert_eq!(
                fs::read(repo.0.join(crate::user_agents::SOURCE)).unwrap(),
                b"external user template edit\n"
            );
        }
        if mode == "drift" {
            assert_eq!(
                fs::read(repo.0.join("templates/hooks/Cargo.lock")).unwrap(),
                b"external edit\n"
            );
        }
        if mode == "policy-drift" {
            assert_eq!(
                fs::read(repo.0.join(".codex/bridgeforge-version.json")).unwrap(),
                b"external policy edit\n"
            );
        }
        fs::remove_dir_all(remote).unwrap();
    }
}

#[test]
fn factory_rejected_commit_restores_versions_binaries_receipts_and_index() {
    let (repo, remote) = factory_repository("generated-rollback");
    let user_manifest_before = fs::read(repo.0.join(crate::user_agents::MANIFEST)).unwrap();
    fs::write(
        repo.0.join(crate::user_agents::SOURCE),
        b"authorized template edit\n",
    )
    .unwrap();
    fs::create_dir_all(repo.0.join(".codex/bin")).unwrap();
    let cli = repo.0.join(if cfg!(windows) {
        ".codex/bin/bridgeforge.exe"
    } else {
        ".codex/bin/bridgeforge"
    });
    fs::write(&cli, b"original binary").unwrap();
    let receipt = repo.0.join(".codex/bin/build-receipt-cli.json");
    fs::write(&receipt, b"original receipt").unwrap();
    let before_contract = fs::read(repo.0.join("templates/managed-skeleton.json")).unwrap();
    fs::write(
        repo.0.join(".git/hooks/pre-commit"),
        b"#!/bin/sh\necho fixture-reject >&2\nexit 1\n",
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(
            repo.0.join(".git/hooks/pre-commit"),
            fs::Permissions::from_mode(0o755),
        )
        .unwrap();
    }
    git_ok(&repo.0, &["status", "--porcelain=v1"]);
    let index = fs::read(repo.0.join(".git/index")).unwrap();
    let outcome = sync(
        &repo.0,
        &GeneratedRunner {
            root: repo.0.clone(),
            mode: "ok",
        },
        GitSyncOptions {
            message: Some("fix: generated rollback".into()),
            skip_fetch: true,
            skip_push: true,
            ..Default::default()
        },
    );
    assert_eq!(
        fs::read(repo.0.join(crate::user_agents::MANIFEST)).unwrap(),
        user_manifest_before
    );
    assert_eq!(
        fs::read(repo.0.join(crate::user_agents::SOURCE)).unwrap(),
        b"authorized template edit\n"
    );
    assert_eq!(outcome.code, 2, "{}", outcome.stderr);
    assert!(
        outcome.stderr.contains("fixture-reject") && outcome.stderr.contains("rolled back"),
        "{}",
        outcome.stderr
    );
    assert_eq!(fs::read(repo.0.join("VERSION")).unwrap(), b"1.0.0\n");
    assert_eq!(fs::read(&cli).unwrap(), b"original binary");
    assert_eq!(fs::read(&receipt).unwrap(), b"original receipt");
    assert_eq!(
        fs::read(repo.0.join("templates/managed-skeleton.json")).unwrap(),
        before_contract
    );
    assert_eq!(fs::read(repo.0.join(".git/index")).unwrap(), index);
    assert_eq!(
        fs::read(repo.0.join("tracked.txt")).unwrap(),
        b"user change\n"
    );
    fs::remove_dir_all(remote).unwrap();
}

fn factory_with_valid_generated_assets(name: &str) -> (RealRepository, PathBuf) {
    let (repo, remote) = factory_repository(name);
    let runner = GeneratedRunner {
        root: repo.0.clone(),
        mode: "ok",
    };
    let plan = write_plan::WritePlan::prepare(
        &repo.0,
        Default::default(),
        Default::default(),
        true,
        &runner,
    )
    .unwrap();
    assert_eq!(plan.generated_built, 2);
    for (path, bytes) in plan.writes {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }
    (repo, remote)
}

#[test]
fn factory_unchanged_generated_inputs_reuse_without_cargo_or_binary_writes() {
    let (repo, remote) = factory_with_valid_generated_assets("reuse");
    let runner = GeneratedRunner {
        root: repo.0.clone(),
        mode: "fail",
    };
    let plan = write_plan::WritePlan::prepare(
        &repo.0,
        Default::default(),
        Default::default(),
        true,
        &runner,
    )
    .unwrap();
    assert_eq!(plan.generated_reused, 2);
    assert_eq!(plan.generated_built, 0);
    assert!(
        !plan
            .writes
            .keys()
            .any(|p| p.starts_with(repo.0.join(".codex/bin")))
    );
    plan.verify_unchanged(&repo.0).unwrap();
    let cli = repo.0.join(if cfg!(windows) {
        ".codex/bin/bridgeforge.exe"
    } else {
        ".codex/bin/bridgeforge"
    });
    fs::write(&cli, b"external concurrent replacement").unwrap();
    assert!(
        plan.verify_unchanged(&repo.0)
            .unwrap_err()
            .contains("changed concurrently")
    );
    fs::remove_dir_all(remote).unwrap();
}

#[test]
fn factory_reuse_invalidates_source_lock_binary_and_receipt_mismatches() {
    for mode in ["source", "lock", "binary", "receipt"] {
        let (repo, remote) = factory_with_valid_generated_assets(mode);
        let path = match mode {
            "source" => repo.0.join("templates/hooks/new.rs"),
            "lock" => repo.0.join("templates/hooks/Cargo.lock"),
            "binary" => repo.0.join(if cfg!(windows) {
                ".codex/bin/bridgeforge.exe"
            } else {
                ".codex/bin/bridgeforge"
            }),
            _ => repo.0.join(".codex/bin/build-receipt-cli.json"),
        };
        fs::write(path, b"changed\n").unwrap();
        let runner = GeneratedRunner {
            root: repo.0.clone(),
            mode: "ok",
        };
        let plan = write_plan::WritePlan::prepare(
            &repo.0,
            Default::default(),
            Default::default(),
            true,
            &runner,
        )
        .unwrap();
        // Corrupt installed outputs can be repaired from the independent valid cache.
        let count = if matches!(mode, "source" | "lock") { 2 } else { 0 };
        assert_eq!(plan.generated_built, count, "{mode}");
        assert_eq!(plan.generated_reused, 2 - count, "{mode}");
        fs::remove_dir_all(remote).unwrap();
    }
}

#[test]
fn factory_release_never_reuses_previous_version_binaries() {
    let (repo, remote) = factory_with_valid_generated_assets("release-miss");
    let runner = GeneratedRunner {
        root: repo.0.clone(),
        mode: "ok",
    };
    let release = crate::release::build_file_release_plan(
        &repo.0,
        "fix: release",
        vec!["tracked.txt".into()],
        &runner,
    )
    .unwrap()
    .unwrap();
    let plan =
        write_plan::WritePlan::prepare(&repo.0, release.writes, release.inputs, true, &runner)
            .unwrap();
    assert_eq!(plan.generated_built, 2);
    assert_eq!(plan.generated_reused, 0);
    assert_eq!(fs::read(repo.0.join("VERSION")).unwrap(), b"1.0.0\n");
    fs::remove_dir_all(remote).unwrap();
}

#[test]
fn factory_reuse_self_test_failure_rebuilds_and_drift_blocks_apply() {
    struct ReuseRunner {
        inner: GeneratedRunner,
        drift: bool,
    }
    impl ProcessRunner for ReuseRunner {
        fn run(&self, request: &ProcessRequest) -> std::io::Result<ProcessOutput> {
            if Path::new(&request.program).starts_with(self.inner.root.join(".codex/bin")) ||
                Path::new(&request.program).starts_with(self.inner.root.join(crate::artifact_cache::DIRECTORY)) {
                if self.drift {
                    fs::write(&request.program, b"external change")?;
                    return self.inner.run(request);
                }
                return Ok(out(0, "{}"));
            }
            self.inner.run(request)
        }
    }
    for drift in [false, true] {
        let (repo, remote) = factory_with_valid_generated_assets("self-test-reuse");
        let runner = ReuseRunner {
            inner: GeneratedRunner {
                root: repo.0.clone(),
                mode: "ok",
            },
            drift,
        };
        let result = write_plan::WritePlan::prepare(
            &repo.0,
            Default::default(),
            Default::default(),
            true,
            &runner,
        );
        if drift {
            assert!(result.err().unwrap().contains("changed concurrently"));
        } else {
            let plan = result.unwrap();
            assert_eq!(plan.generated_built, 2);
            assert_eq!(plan.generated_reused, 0);
        }
        fs::remove_dir_all(remote).unwrap();
    }
}

#[test]
fn factory_partial_reuse_rejected_commit_preserves_reused_asset_and_restores_miss() {
    let (repo, remote) = factory_with_valid_generated_assets("partial-reuse-rollback");
    // Keep this scenario as one installed hit plus one real build, not two cache hits.
    fs::remove_dir_all(repo.0.join(crate::artifact_cache::DIRECTORY)).unwrap();
    let config_path = repo.0.join(".codex/bridgeforge-version.json");
    let mut config: serde_json::Value =
        serde_json::from_slice(&fs::read(&config_path).unwrap()).unwrap();
    config["release_policy"] = json!("explicit_release");
    fs::write(config_path, serde_json::to_vec(&config).unwrap()).unwrap();
    let hook = repo.0.join(if cfg!(windows) {
        ".codex/bin/bridgeforge-hook.exe"
    } else {
        ".codex/bin/bridgeforge-hook"
    });
    let cli = repo.0.join(if cfg!(windows) {
        ".codex/bin/bridgeforge.exe"
    } else {
        ".codex/bin/bridgeforge"
    });
    let receipt = repo.0.join(".codex/bin/build-receipt-cli.json");
    let hook_before = fs::read(&hook).unwrap();
    let hook_time = fs::metadata(&hook).unwrap().modified().unwrap();
    fs::write(&cli, b"broken original CLI").unwrap();
    fs::write(&receipt, b"{}").unwrap();
    fs::write(repo.0.join(".git/hooks/pre-commit"), b"#!/bin/sh\nexit 1\n").unwrap();
    git_ok(&repo.0, &["status", "--porcelain=v1"]);
    let index = fs::read(repo.0.join(".git/index")).unwrap();
    struct CountingRunner {
        inner: GeneratedRunner,
        cargo: std::cell::Cell<usize>,
    }
    impl ProcessRunner for CountingRunner {
        fn run(&self, request: &ProcessRequest) -> std::io::Result<ProcessOutput> {
            if request.program == "cargo" && request.args.first().is_some_and(|arg|arg=="build") {
                self.cargo.set(self.cargo.get() + 1);
            }
            self.inner.run(request)
        }
    }
    let runner = CountingRunner {
        inner: GeneratedRunner {
            root: repo.0.clone(),
            mode: "ok",
        },
        cargo: Default::default(),
    };
    let outcome = sync(
        &repo.0,
        &runner,
        GitSyncOptions {
            message: Some("docs: partial reuse rollback".into()),
            skip_fetch: true,
            skip_push: true,
            ..Default::default()
        },
    );
    assert_eq!(outcome.code, 2, "{}", outcome.stderr);
    assert!(outcome.stderr.contains("rolled back"), "{}", outcome.stderr);
    assert_eq!(runner.cargo.get(), 1);
    assert_eq!(fs::read(&hook).unwrap(), hook_before);
    assert_eq!(fs::metadata(&hook).unwrap().modified().unwrap(), hook_time);
    assert_eq!(fs::read(&cli).unwrap(), b"broken original CLI");
    assert_eq!(fs::read(&receipt).unwrap(), b"{}");
    assert_eq!(fs::read(repo.0.join(".git/index")).unwrap(), index);
    assert_eq!(
        fs::read(repo.0.join("tracked.txt")).unwrap(),
        b"user change\n"
    );
    fs::remove_dir_all(remote).unwrap();
}

#[test]
fn independent_cache_release_ignores_missing_or_corrupted_prepared() {
    for corrupt in [false,true] {
        let (repo,remote)=factory_repository("independent-cache-record");
        install_development_check(&repo.0);
        let runner=GeneratedRunner{root:repo.0.clone(),mode:"ok"};
        assert_eq!(prepare_release(&repo.0,&runner,"fix: cache",None).code,0);
        let record=repo.0.join(".runtime/bridgeforge-codex/release-preparation/current.json");
        if corrupt {fs::write(&record,b"{invalid prepared/audit").unwrap();} else {fs::remove_file(record).unwrap();}
        struct NoBuild<'a>(&'a GeneratedRunner);
        impl ProcessRunner for NoBuild<'_> {
            fn run(&self,r:&ProcessRequest)->std::io::Result<ProcessOutput> {
                assert!(!(r.program=="cargo" && r.args.first().is_some_and(|a|a=="build")),"cache hit must not compile");
                self.0.run(r)
            }
        }
        let result=sync(&repo.0,&NoBuild(&runner),GitSyncOptions{release:true,message:Some("fix: new description".into()),..Default::default()});
        assert_eq!(result.code,0,"{}",result.stderr);
        let receipt=result.receipt.unwrap();assert_eq!(receipt["generated_assets_built"],0);assert_eq!(receipt["generated_assets_reused"],2);
        fs::remove_dir_all(remote).unwrap();
    }
}

#[test]
fn mixed_cache_hit_and_build_rejected_commit_restore_all_targets() {
    let (repo,remote)=factory_repository("mixed-cache-rollback");install_development_check(&repo.0);
    let runner=GeneratedRunner{root:repo.0.clone(),mode:"ok"};assert_eq!(prepare_release(&repo.0,&runner,"fix: cache",None).code,0);
    let directory=repo.0.join(crate::artifact_cache::DIRECTORY);
    for item in fs::read_dir(&directory).unwrap() {
        let item=item.unwrap();let meta=item.path().join("entry.json");
        if meta.is_file() {
            let value:serde_json::Value=serde_json::from_slice(&fs::read(meta).unwrap()).unwrap();
            if value["identity"]["id"]=="codex.bridgeforge-cli" {fs::remove_dir_all(item.path()).unwrap();}
        }
    }
    fs::write(repo.0.join(".git/hooks/pre-commit"),b"#!/bin/sh\nexit 1\n").unwrap();
    git_ok(&repo.0,&["status","--porcelain=v1"]);let index=fs::read(repo.0.join(".git/index")).unwrap();
    struct MixedRunner {inner:GeneratedRunner,builds:std::cell::Cell<usize>,hits:std::cell::Cell<usize>}
    impl ProcessRunner for MixedRunner {
        fn run(&self,r:&ProcessRequest)->std::io::Result<ProcessOutput> {
            if r.program=="cargo" && r.args.first().is_some_and(|a|a=="build") {self.builds.set(self.builds.get()+1);}
            if Path::new(&r.program).starts_with(self.inner.root.join(crate::artifact_cache::DIRECTORY)) &&
                r.args.first().is_some_and(|a|a=="self-test") {self.hits.set(self.hits.get()+1);}
            self.inner.run(r)
        }
    }
    let paths=["VERSION","CHANGELOG.md","templates/hooks/Cargo.toml","templates/hooks/Cargo.lock",
        ".codex/hooks/Cargo.toml",".codex/hooks/Cargo.lock","templates/managed-skeleton.json",".codex/managed-skeleton.json",
        "bridgeforge-codex-manifest.json",crate::user_agents::MANIFEST,
        ".codex/bin/build-receipt-cli.json",".codex/bin/build-receipt-hook.json"];
    let mut before=paths.into_iter().map(|p|{let p=repo.0.join(p);let bytes=fs::read(&p).ok();(p,bytes)}).collect::<std::collections::BTreeMap<_,_>>();
    for name in ["bridgeforge","bridgeforge-hook"] {
        let path=repo.0.join(".codex/bin").join(if cfg!(windows){format!("{name}.exe")}else{name.into()});
        before.insert(path.clone(),fs::read(path).ok());
    }
    let counted=MixedRunner{inner:runner,builds:Default::default(),hits:Default::default()};
    let result=sync(&repo.0,&counted,GitSyncOptions{release:true,message:Some("fix: mixed cache".into()),skip_fetch:true,skip_push:true,..Default::default()});
    assert_eq!(result.code,2,"{}",result.stderr);assert!(result.stderr.contains("rolled back"));
    assert_eq!(fs::read(repo.0.join("VERSION")).unwrap(),b"1.0.0\n");assert_eq!(fs::read(repo.0.join(".git/index")).unwrap(),index);
    assert!(!repo.0.join(".codex/bin/bridgeforge.exe").exists());assert!(!repo.0.join(".codex/bin/bridgeforge-hook.exe").exists());
    assert_eq!(counted.builds.get(),1,"mixed scenario must really compile one miss");
    assert_eq!(counted.hits.get(),1,"mixed scenario must really self-test one cache hit");
    for (path,bytes) in before {assert_eq!(fs::read(&path).ok(),bytes,"automatic target drift: {}",path.display());}
    fs::remove_dir_all(remote).unwrap();
}

#[test]
fn factory_read_only_status_queries_do_not_initialize_artifact_cache() {
    let (repo,remote)=factory_repository("cache-readonly");
    assert!(!repo.0.join(crate::artifact_cache::DIRECTORY).exists());
    let runner=GeneratedRunner{root:repo.0.clone(),mode:"fail"};
    let _=release_status(&repo.0,&runner);let _=development_status(&repo.0,&runner);
    assert!(!repo.0.join(crate::artifact_cache::DIRECTORY).exists());
    assert_eq!(fs::read(repo.0.join("VERSION")).unwrap(),b"1.0.0\n");
    fs::remove_dir_all(remote).unwrap();
}
