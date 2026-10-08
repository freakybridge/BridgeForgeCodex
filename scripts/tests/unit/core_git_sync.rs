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
    git_ok(&repo.0, &["add", "."]);
    git_ok(&repo.0, &["commit", "-m", "chore: version baseline"]);
    git_ok(&repo.0, &["push"]);
    (repo, remote)
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
fn legacy_policy_still_bumps_on_ordinary_sync() {
    let (repo, remote) = versioned_repository("legacy-release", false);
    fs::write(repo.0.join("tracked.txt"), b"change\n").unwrap();
    let outcome = sync(
        &repo.0,
        &SystemProcessRunner,
        GitSyncOptions {
            message: Some("fix: repair".into()),
            ..Default::default()
        },
    );
    assert_eq!(outcome.code, 0, "{}", outcome.stderr);
    assert_eq!(outcome.receipt.unwrap()["version_after"], "1.0.1");
    fs::remove_dir_all(remote).unwrap();
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
        if self.mode == "release-drift" && request.args.iter().any(|arg| arg == "check-ignore") {
            fs::write(self.root.join("CHANGELOG.md"), b"external changelog edit\n")?;
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
    for mode in ["fail", "drift", "release-drift", "user-agents-drift"] {
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
        if mode == "release-drift" {
            assert_eq!(
                fs::read(repo.0.join("CHANGELOG.md")).unwrap(),
                b"external changelog edit\n"
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
        let count = if matches!(mode, "source" | "lock") {
            2
        } else {
            1
        };
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
            if Path::new(&request.program).starts_with(self.inner.root.join(".codex/bin")) {
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
            if request.program == "cargo" {
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
