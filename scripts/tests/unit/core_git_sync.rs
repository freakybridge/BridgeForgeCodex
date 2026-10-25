use super::*;
#[path = "../support/git_repository.rs"]
mod repository;
use repository::*;
#[path = "../support/schema4_agent.rs"]
mod fixture;
use crate::{ProcessOutput, ProcessRequest, SystemProcessRunner};
use std::cell::RefCell;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn unfinished_development_transaction_blocks_sync_before_network() {
    struct LocalInspectionOnly(std::cell::Cell<usize>, PathBuf);
    impl ProcessRunner for LocalInspectionOnly {
        fn run(&self, request: &ProcessRequest) -> std::io::Result<ProcessOutput> {
            self.0.set(self.0.get() + 1);
            if request.program == "git"
                && (request.args == ["rev-parse", "--git-dir"]
                    || request.args == ["rev-parse", "--path-format=absolute", "--git-common-dir"])
            {
                return Ok(ProcessOutput {
                    code: 0,
                    stdout: self.1.join(".git").to_string_lossy().as_bytes().to_vec(),
                    stderr: Vec::new(),
                    timed_out: false,
                });
            }
            Err(std::io::Error::other(
                "only local identity inspection is permitted before recovery",
            ))
        }
    }
    let root = std::env::temp_dir().join(format!(
        "bf-sync-unfinished-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(root.join(".runtime/bridgeforge-codex")).unwrap();
    fs::create_dir_all(root.join(".git")).unwrap();
    fs::write(
        root.join(".runtime/bridgeforge-codex/develop-transaction.json"),
        b"{}",
    )
    .unwrap();
    let runner = LocalInspectionOnly(std::cell::Cell::new(0), root.clone());
    let result = super::sync(&root, &runner, GitSyncOptions::default());
    assert!(
        result.stderr.contains("develop --recover"),
        "{}",
        result.stderr
    );
    assert_eq!(runner.0.get(), 2);
    assert_eq!(
        fs::read(root.join(".runtime/bridgeforge-codex/develop-transaction.json")).unwrap(),
        b"{}"
    );
    fs::remove_dir_all(root).unwrap();
}

fn sync(root: &Path, runner: &dyn ProcessRunner, mut options: GitSyncOptions) -> CommandOutcome {
    if options.release
        && options.release_request.analysis_file.is_none()
        && fs::read(root.join(".codex/bridgeforge-version.json"))
            .ok()
            .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
            .is_some_and(|v| v["schema_version"] == 4)
    {
        match fixture::analysis(root, runner, "patch", &[]) {
            Ok(request) => options.release_request = request,
            Err(e) => return CommandOutcome::blocked(e),
        }
    }
    super::sync(root, runner, options)
}
fn prepare_release(
    root: &Path,
    runner: &dyn ProcessRunner,
    message: &str,
    audit: Option<&Path>,
) -> CommandOutcome {
    let request = if fs::read(root.join(".codex/bridgeforge-version.json"))
        .ok()
        .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
        .is_some_and(|v| v["schema_version"] == 4)
    {
        match fixture::analysis(root, runner, "patch", &[]) {
            Ok(request) => request,
            Err(e) => return CommandOutcome::blocked(e),
        }
    } else {
        Default::default()
    };
    super::prepare_release_with_request(root, runner, message, audit, &request)
}

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
    versioned_repository_with_strategy(name, explicit, false)
}
fn versioned_repository_with_strategy(
    name: &str,
    explicit: bool,
    independent: bool,
) -> (RealRepository, PathBuf) {
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
    let mut config = if independent {
        json!({"schema_version":2,"release_policy":"explicit_release","strategy":"rust_independent","rust":{"workspaces":["Cargo.toml"]}})
    } else {
        json!({"schema_version": 1, "manifests": ["Cargo.toml"]})
    };
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
    fixture::migrate(&repo.0, &SystemProcessRunner).unwrap();
    git_ok(&repo.0, &["add", "."]);
    git_ok(&repo.0, &["commit", "-m", "chore: schema4 fixture"]);
    fs::remove_dir_all(repo.0.join(".runtime")).unwrap();
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
    assert!(
        blockers[1]["reason"]
            .as_str()
            .unwrap()
            .contains("VERSION differs")
    );
    assert_eq!(fs::read(repo.0.join(".git/index")).unwrap(), index);
    assert_eq!(fs::read(repo.0.join(".git/HEAD")).unwrap(), head);
    assert_eq!(fs::read(repo.0.join("VERSION")).unwrap(), b"0.9.9\n");
    assert!(!repo.0.join(".runtime").exists());
    assert!(!repo.0.join(".codex/development-checks.json").exists());
    fs::write(repo.0.join("VERSION"), b"1.0.0\n").unwrap();
    for bytes in [
        b"{bad".as_slice(),
        br#"{"schema":1,"checks":[]}"#.as_slice(),
    ] {
        fs::write(repo.0.join(".codex/development-checks.json"), bytes).unwrap();
        let outcome = development_status(&repo.0, &SystemProcessRunner);
        assert_eq!(outcome.code, crate::EXIT_BLOCKED);
        let value = outcome.receipt.unwrap();
        assert_eq!(value["setup"]["status"], "invalid-config");
        assert_eq!(
            fs::read(repo.0.join(".codex/development-checks.json")).unwrap(),
            bytes
        );
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
        for (path, bytes) in [&manifest, &lock, &policy].iter().zip(&originals) {
            fs::write(path, bytes).unwrap();
        }
        match mode {
            "manifest" => fs::write(
                &manifest,
                b"[package]\nname=\"sample\"\nversion=\"0.9.9\"\n",
            )
            .unwrap(),
            "lock" => fs::write(
                &lock,
                b"version=4\n[[package]]\nname=\"sample\"\nversion=\"0.9.9\"\n",
            )
            .unwrap(),
            _ => fs::write(
                &policy,
                br#"{"schema_version":1,"manifests":["missing/Cargo.toml"]}"#,
            )
            .unwrap(),
        }
        let before = [&manifest, &lock, &policy].map(|path| fs::read(path).unwrap());
        let index = fs::read(repo.0.join(".git/index")).unwrap();
        let head = fs::read(repo.0.join(".git/HEAD")).unwrap();
        let outcome = development_status(&repo.0, &SystemProcessRunner);
        assert_eq!(outcome.code, crate::EXIT_BLOCKED, "{mode}");
        let value = outcome.receipt.unwrap();
        assert_eq!(value["blockers"].as_array().unwrap().len(), 2, "{mode}");
        assert_eq!(value["blockers"][1]["code"], "release-baseline-blocked");
        for (path, bytes) in [&manifest, &lock, &policy].iter().zip(&before) {
            assert_eq!(&fs::read(path).unwrap(), bytes);
        }
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
    fs::write(
        repo.0.join(".codex/bridgeforge-version.json"),
        br#"{"schema_version":1,"manifests":["package.json"]}"#,
    )
    .unwrap();
    fs::write(
        repo.0.join("package.json"),
        br#"{"name":"sample","version":"1.0.0"}"#,
    )
    .unwrap();
    for name in ["pnpm-lock.yaml", "yarn.lock"] {
        let lock = repo.0.join(name);
        fs::write(&lock, b"project-owned unsupported lock\n").unwrap();
        let index = fs::read(repo.0.join(".git/index")).unwrap();
        let head = fs::read(repo.0.join(".git/HEAD")).unwrap();
        let value = development_status(&repo.0, &SystemProcessRunner)
            .receipt
            .unwrap();
        assert_eq!(value["blockers"].as_array().unwrap().len(), 2);
        assert_eq!(
            value["blockers"][1]["reason"],
            format!("unsupported JavaScript lock file: {name}")
        );
        assert_eq!(
            fs::read(&lock).unwrap(),
            b"project-owned unsupported lock\n"
        );
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
    let request = fixture::analysis(&repo.0, &SystemProcessRunner, "minor", &[]).unwrap();
    let preview =
        crate::release::schema4::preview(&repo.0, "fix: release", &request, &SystemProcessRunner);
    assert_eq!(preview.code, 0, "{}", preview.stderr);
    assert_eq!(preview.receipt.unwrap()["version_after"], "1.1.0");
    let outcome = sync(
        &repo.0,
        &SystemProcessRunner,
        GitSyncOptions {
            release: true,
            release_request: request,
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
    assert!(log.contains("fixture explicitly declares the program impact"));
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
    for policy in [
        "missing-file",
        "missing-field",
        "per_commit",
        "explicit_release",
    ] {
        let (repo, remote) = versioned_repository(policy, false);
        let path = repo.0.join(".codex/bridgeforge-version.json");
        if policy == "missing-file" {
            fs::remove_file(&path).unwrap();
        } else {
            fs::write(&path, serde_json::to_vec(&json!({
                "schema_version": 1, "release_policy":if policy=="missing-field"{"explicit_release"}else{policy}, "manifests": ["Cargo.toml"]
            })).unwrap()).unwrap();
        }
        git_ok(&repo.0, &["add", "."]);
        git_ok(
            &repo.0,
            &["commit", "--allow-empty", "-m", "chore: policy baseline"],
        );
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
            assert_eq!(
                &fs::read(repo.0.join(name)).unwrap(),
                bytes,
                "{policy}: {name}"
            );
        }
        assert_eq!(fs::read(&path).ok(), config_before);
        let released = sync(
            &repo.0,
            &SystemProcessRunner,
            GitSyncOptions {
                release: true,
                message: Some("fix: release".into()),
                ..Default::default()
            },
        );
        assert_ne!(released.code, 0);
        assert!(
            released.stderr.contains("schema 4 migration"),
            "{}",
            released.stderr
        );
        assert_eq!(
            fs::read_to_string(repo.0.join("VERSION")).unwrap().trim(),
            "1.0.0"
        );
        assert_eq!(fs::read(&path).ok(), config_before);
        fs::remove_dir_all(remote).unwrap();
    }
}

#[test]
fn explicit_release_rejected_hook_restores_native_versions_log_and_index() {
    for mode in ["synchronized", "independent", "addition"] {
        let independent = mode != "synchronized";
        let (repo, remote) =
            versioned_repository_with_strategy("explicit-rollback", true, independent);
        if mode == "addition" {
            fs::create_dir_all(repo.0.join("new")).unwrap();
            fs::write(
                repo.0.join("new/Cargo.toml"),
                b"[package]\nname='new'\nversion='0.2.7'\n[workspace]\n",
            )
            .unwrap();
            fs::write(
                repo.0.join("new/Cargo.lock"),
                b"version=4\n[[package]]\nname='new'\nversion='0.2.7'\n",
            )
            .unwrap();
            let cfg_path = repo.0.join(".codex/bridgeforge-version.json");
            let mut cfg: serde_json::Value =
                serde_json::from_slice(&fs::read(&cfg_path).unwrap()).unwrap();
            cfg["adapters"]["rust"]["workspaces"] = json!(["Cargo.toml", "new/Cargo.toml"]);
            cfg["packages"].as_array_mut().unwrap().push(
                json!({"id":"new","language":"rust","manifest":"new/Cargo.toml","name":"new"}),
            );
            fs::write(cfg_path, serde_json::to_vec(&cfg).unwrap()).unwrap();
        }
        fs::write(repo.0.join("tracked.txt"), b"uncommitted feature\n").unwrap();
        // Package closure is a completed development transaction, outside release rollback.
        let release_request =
            fixture::analysis(&repo.0, &SystemProcessRunner, "patch", &[]).unwrap();
        fs::write(repo.0.join(".git/hooks/pre-commit"), b"#!/bin/sh\nexit 1\n").unwrap();
        git_ok(&repo.0, &["status", "--porcelain=v1"]);
        let index = fs::read(repo.0.join(".git/index")).unwrap();
        let mut names = vec![
            "VERSION",
            "Cargo.toml",
            "Cargo.lock",
            "CHANGELOG.md",
            ".codex/bridgeforge-version.json",
        ];
        if mode == "addition" {
            names.extend(["new/Cargo.toml", "new/Cargo.lock"]);
        }
        let before = names
            .iter()
            .map(|name| fs::read(repo.0.join(name)).unwrap())
            .collect::<Vec<_>>();
        let outcome = sync(
            &repo.0,
            &SystemProcessRunner,
            GitSyncOptions {
                release: true,
                release_request,
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
}

struct ReusableCheckRunner {
    calls: RefCell<Vec<String>>,
    fail_second: bool,
}

impl ProcessRunner for ReusableCheckRunner {
    fn run(&self, request: &ProcessRequest) -> std::io::Result<ProcessOutput> {
        let args = request
            .args
            .iter()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        if request.program == "git" && (args == ["diff", "--check"] || args == ["diff", "--stat"]) {
            self.calls.borrow_mut().push(args[1].clone());
            return Ok(ProcessOutput {
                code: if self.fail_second && args[1] == "--stat" {
                    1
                } else {
                    0
                },
                stdout: args.join(" ").into_bytes(),
                stderr: Vec::new(),
                timed_out: false,
            });
        }
        SystemProcessRunner.run(request)
    }
}

#[test]
fn reusable_preparation_resumes_failed_checks_and_rechecks_corrupt_logs_and_inputs() {
    let (repo, remote) = versioned_repository("check-resume", true);
    install_record_documents(&repo.0);
    let cfg = repo.0.join(".codex/development-checks.json");
    let mut value: serde_json::Value = serde_json::from_slice(&fs::read(&cfg).unwrap()).unwrap();
    value["checks"] = json!([
        {"id":"first","program":"git","args":["diff","--check"],"timeout_seconds":30,"reusable":true},
        {"id":"second","program":"git","args":["diff","--stat"],"timeout_seconds":30,"reusable":true}
    ]);
    fs::write(&cfg, serde_json::to_vec(&value).unwrap()).unwrap();
    fs::write(repo.0.join("tracked.txt"), b"business change\n").unwrap();
    let fail = ReusableCheckRunner {
        calls: RefCell::new(Vec::new()),
        fail_second: true,
    };
    let failed = prepare_release(&repo.0, &fail, "fix: checkpoint", None);
    assert_eq!(failed.code, 2, "{}", failed.stderr);
    assert!(failed.stderr.contains("second failed"), "{}", failed.stderr);
    let runner = ReusableCheckRunner {
        calls: RefCell::new(Vec::new()),
        fail_second: false,
    };
    let resumed = prepare_release(&repo.0, &runner, "fix: checkpoint", None);
    assert_eq!(resumed.code, 0, "{}", resumed.stderr);
    assert_eq!(resumed.receipt.unwrap()["checks_reused"], 1);
    assert_eq!(*runner.calls.borrow(), vec!["--stat"]);
    runner.calls.borrow_mut().clear();
    fs::write(
        repo.0.join("doc/3_reference/result.md"),
        b"# Result\nRecorded after checks.\n",
    )
    .unwrap();
    let repeat = prepare_release(&repo.0, &runner, "feat: revised release message", None);
    assert_eq!(repeat.code, 0, "{}", repeat.stderr);
    assert_eq!(repeat.receipt.as_ref().unwrap()["checks_reused"], 2);
    assert_eq!(repeat.receipt.unwrap()["target_version"], "1.0.1");
    assert!(runner.calls.borrow().is_empty());
    let progress: serde_json::Value = serde_json::from_slice(
        &fs::read(
            repo.0
                .join(".runtime/bridgeforge-codex/release-preparation/checks.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let hash = progress["checks"][0]["log_sha256"]
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
    let repaired = prepare_release(&repo.0, &runner, "fix: checkpoint", None);
    assert_eq!(repaired.code, 0, "{}", repaired.stderr);
    assert_eq!(repaired.receipt.unwrap()["checks_executed"], 1);
    runner.calls.borrow_mut().clear();
    git_ok(&repo.0, &["add", "tracked.txt"]);
    let restaged = prepare_release(&repo.0, &runner, "fix: checkpoint", None);
    assert_eq!(restaged.code, 0, "{}", restaged.stderr);
    assert_eq!(restaged.receipt.unwrap()["checks_reused"], 0);
    runner.calls.borrow_mut().clear();
    fs::write(repo.0.join("tracked.txt"), b"different input\n").unwrap();
    let changed = prepare_release(&repo.0, &runner, "fix: checkpoint", None);
    assert_eq!(changed.code, 0, "{}", changed.stderr);
    assert_eq!(changed.receipt.unwrap()["checks_reused"], 0);
    assert_eq!(
        fs::read_to_string(repo.0.join("VERSION")).unwrap().trim(),
        "1.0.0"
    );
    fs::remove_dir_all(remote).unwrap();
}

#[test]
fn development_checks_without_reuse_opt_in_always_execute() {
    let (repo, remote) = versioned_repository("check-no-reuse", true);
    fs::write(repo.0.join("tracked.txt"), b"change\n").unwrap();
    let runner = ReusableCheckRunner {
        calls: RefCell::new(Vec::new()),
        fail_second: false,
    };
    for _ in 0..2 {
        let outcome = prepare_release(&repo.0, &runner, "fix: prepare", None);
        assert_eq!(outcome.code, 0, "{}", outcome.stderr);
        assert_eq!(outcome.receipt.unwrap()["checks_reused"], 0);
    }
    assert_eq!(runner.calls.borrow().len(), 2);
    fs::remove_dir_all(remote).unwrap();
}

#[test]
fn reusable_preparation_invalidates_changed_tools_and_malformed_checkpoint() {
    struct ToolRunner(RefCell<usize>);
    impl ProcessRunner for ToolRunner {
        fn run(&self, request: &ProcessRequest) -> std::io::Result<ProcessOutput> {
            if request.args == [OsString::from("cache-test")] {
                *self.0.borrow_mut() += 1;
                return Ok(ProcessOutput {
                    code: 0,
                    stdout: b"checked\n".to_vec(),
                    stderr: Vec::new(),
                    timed_out: false,
                });
            }
            SystemProcessRunner.run(request)
        }
    }
    let (repo, remote) = versioned_repository("check-tool-identity", true);
    fs::create_dir_all(repo.0.join(".runtime")).unwrap();
    let tool = repo.0.join(".runtime/check-tool.exe");
    fs::write(&tool, b"tool-v1").unwrap();
    fs::write(repo.0.join(".codex/development-checks.json"), serde_json::to_vec(&json!({"schema":1,"checks":[{"id":"tool","program":tool,"args":["cache-test"],"timeout_seconds":30,"reusable":true}]})).unwrap()).unwrap();
    fs::write(repo.0.join("tracked.txt"), b"change\n").unwrap();
    let runner = ToolRunner(RefCell::new(0));
    for (change, expected) in [
        ("initial", 0),
        ("repeat", 1),
        ("tool", 0),
        ("corrupt", 0),
        ("missing-tool", 0),
    ] {
        match change {
            "tool" => fs::write(&tool, b"tool-v2").unwrap(),
            "corrupt" => fs::write(
                repo.0
                    .join(".runtime/bridgeforge-codex/release-preparation/checks.json"),
                b"{broken",
            )
            .unwrap(),
            "missing-tool" => fs::remove_file(&tool).unwrap(),
            _ => (),
        }
        let result = prepare_release(&repo.0, &runner, "fix: tool check", None);
        assert_eq!(result.code, 0, "{change}: {}", result.stderr);
        let receipt = result.receipt.unwrap();
        assert_eq!(receipt["checks_reused"], expected, "{change}");
        assert_eq!(
            receipt["checks_reuse_available"],
            change != "missing-tool",
            "{change}"
        );
    }
    assert_eq!(*runner.0.borrow(), 4);
    fs::remove_dir_all(remote).unwrap();
}

#[test]
fn reusable_preparation_invalidates_git_attributes_outside_worktree_inputs() {
    struct Counting(RefCell<usize>);
    impl ProcessRunner for Counting {
        fn run(&self, request: &ProcessRequest) -> std::io::Result<ProcessOutput> {
            if request.args == [OsString::from("diff"), OsString::from("--check")] {
                *self.0.borrow_mut() += 1;
            }
            SystemProcessRunner.run(request)
        }
    }
    let (repo, remote) = versioned_repository("check-external-attributes", true);
    let cfg = repo.0.join(".codex/development-checks.json");
    let mut value: serde_json::Value = serde_json::from_slice(&fs::read(&cfg).unwrap()).unwrap();
    value["checks"][0]["reusable"] = json!(true);
    fs::write(&cfg, serde_json::to_vec(&value).unwrap()).unwrap();
    fs::write(repo.0.join("tracked.txt"), b"change \n").unwrap();
    fs::write(
        repo.0.join(".git/info/attributes"),
        b"tracked.txt -whitespace\n",
    )
    .unwrap();
    let runner = Counting(RefCell::new(0));
    let first = prepare_release(&repo.0, &runner, "fix: attributes", None);
    assert_eq!(first.code, 0, "{}", first.stderr);
    fs::write(
        repo.0.join(".git/info/attributes"),
        b"tracked.txt whitespace=blank-at-eol\n",
    )
    .unwrap();
    let second = prepare_release(&repo.0, &runner, "fix: attributes", None);
    assert_eq!(second.code, 2, "{}", second.stderr);
    assert!(
        second.stderr.contains("development check diff failed"),
        "{}",
        second.stderr
    );
    assert_eq!(*runner.0.borrow(), 2);
    fs::remove_dir_all(remote).unwrap();
}

#[test]
fn release_replans_when_fetch_changes_replacement_history_without_changing_head() {
    struct ReplaceOnFetch {
        original: String,
        replacement: String,
        plans: RefCell<usize>,
    }
    impl ProcessRunner for ReplaceOnFetch {
        fn run(&self, request: &ProcessRequest) -> std::io::Result<ProcessOutput> {
            let args = request
                .args
                .iter()
                .map(|arg| arg.to_string_lossy())
                .collect::<Vec<_>>();
            if args.first().is_some_and(|arg| arg == "rev-list")
                && args.iter().any(|arg| arg == "--topo-order")
            {
                *self.plans.borrow_mut() += 1;
            }
            let output = SystemProcessRunner.run(request)?;
            if args.first().is_some_and(|arg| arg == "fetch") {
                git_ok(
                    &request.cwd,
                    &["replace", "-f", &self.original, &self.replacement],
                );
            }
            Ok(output)
        }
    }
    let (repo, remote) = versioned_repository("replacement-fetch", true);
    fs::write(repo.0.join("tracked.txt"), b"feature\n").unwrap();
    git_ok(&repo.0, &["add", "tracked.txt"]);
    git_ok(&repo.0, &["commit", "-m", "feat: original feature"]);
    let git = Git {
        root: &repo.0,
        runner: &SystemProcessRunner,
    };
    let original = git
        .required(&["rev-parse", "HEAD"], Duration::from_secs(30))
        .unwrap();
    let tree = git
        .required(&["rev-parse", "HEAD^{tree}"], Duration::from_secs(30))
        .unwrap();
    let parent = git
        .required(&["rev-parse", "HEAD^"], Duration::from_secs(30))
        .unwrap();
    let replacement = git
        .required(
            &[
                "commit-tree",
                &tree,
                "-p",
                &parent,
                "-m",
                "fix: replacement correction",
            ],
            Duration::from_secs(30),
        )
        .unwrap();
    let runner = ReplaceOnFetch {
        original,
        replacement,
        plans: RefCell::new(0),
    };
    let result = sync(
        &repo.0,
        &runner,
        GitSyncOptions {
            release: true,
            skip_push: true,
            message: Some("fix: release history".into()),
            ..Default::default()
        },
    );
    assert_eq!(result.code, 2, "{}", result.stderr);
    assert!(result.stderr.contains("stale"), "{}", result.stderr);
    assert_eq!(
        fs::read_to_string(repo.0.join("VERSION")).unwrap().trim(),
        "1.0.0"
    );
    fs::remove_dir_all(remote).unwrap();
}

#[test]
fn release_reuses_plan_only_when_fetch_preserves_all_planning_inputs() {
    struct Runner {
        plans: RefCell<usize>,
        mutate: bool,
    }
    impl ProcessRunner for Runner {
        fn run(&self, request: &ProcessRequest) -> std::io::Result<ProcessOutput> {
            let args = request
                .args
                .iter()
                .map(|arg| arg.to_string_lossy())
                .collect::<Vec<_>>();
            if args.first().is_some_and(|arg| arg == "rev-list")
                && args.iter().any(|arg| arg == "--topo-order")
            {
                *self.plans.borrow_mut() += 1;
            }
            let output = SystemProcessRunner.run(request)?;
            if self.mutate && args.first().is_some_and(|arg| arg == "fetch") {
                fs::write(request.cwd.join("tracked.txt"), b"changed during fetch\n")?;
            }
            Ok(output)
        }
    }
    for mutate in [false, true] {
        let (repo, remote) = versioned_repository("plan-reuse", true);
        fs::write(repo.0.join("tracked.txt"), b"feature\n").unwrap();
        let runner = Runner {
            plans: RefCell::new(0),
            mutate,
        };
        let result = sync(
            &repo.0,
            &runner,
            GitSyncOptions {
                release: true,
                message: Some("feat: release".into()),
                ..Default::default()
            },
        );
        if mutate {
            assert_eq!(result.code, 2, "{}", result.stderr);
            assert!(result.stderr.contains("stale"), "{}", result.stderr);
            assert_eq!(
                fs::read_to_string(repo.0.join("VERSION")).unwrap().trim(),
                "1.0.0"
            );
        } else {
            assert_eq!(result.code, 0, "{}", result.stderr);
            let receipt = result.receipt.unwrap();
            assert_eq!(receipt["release_plan_reused"], true);
            assert_eq!(receipt["version_after"], "1.0.1");
            assert_eq!(receipt["working_tree"], "clean");
            let second = sync(
                &repo.0,
                &SystemProcessRunner,
                GitSyncOptions {
                    release: true,
                    message: Some("fix: repeated".into()),
                    ..Default::default()
                },
            );
            assert_eq!(second.code, 0, "{}", second.stderr);
            assert_eq!(second.receipt.unwrap()["version_bumped"], false);
        }
        fs::remove_dir_all(remote).unwrap();
    }
}

#[test]
fn release_snapshot_unavailable_disables_reuse_without_new_preparation_requirements() {
    let (repo, remote) = versioned_repository("release-snapshot-fallback", true);
    fs::create_dir_all(repo.0.join(".runtime")).unwrap();
    fs::write(
        repo.0.join(".runtime/legacy.txt"),
        b"tracked legacy runtime\n",
    )
    .unwrap();
    git_ok(&repo.0, &["add", "-f", ".runtime/legacy.txt"]);
    git_ok(&repo.0, &["commit", "-m", "fix: legacy runtime input"]);
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
    assert_eq!(receipt["release_plan_reused"], false);
    assert_eq!(receipt["version_after"], "1.0.1");
    assert_eq!(receipt["working_tree"], "clean");
    fs::remove_dir_all(remote).unwrap();
}

#[test]
fn release_declared_directory_drift_blocks_reuse_and_final_write() {
    struct Drift {
        late: bool,
        fetched: std::cell::Cell<bool>,
        identities: std::cell::Cell<usize>,
        injected: std::cell::Cell<bool>,
    }
    impl ProcessRunner for Drift {
        fn run(&self, request: &ProcessRequest) -> std::io::Result<ProcessOutput> {
            let args = request
                .args
                .iter()
                .map(|a| a.to_string_lossy())
                .collect::<Vec<_>>();
            let output = SystemProcessRunner.run(request)?;
            let mut inject = false;
            if request.program == "git" && args.first().is_some_and(|a| a == "fetch") {
                self.fetched.set(true);
                inject = !self.late;
            }
            if self.late
                && self.fetched.get()
                && args.iter().any(|a| a == "--git-path")
                && args.iter().any(|a| a == "--git-common-dir")
            {
                self.identities.set(self.identities.get() + 1);
                // Transaction capture, plan-reuse capture, then the post-prepare guard.
                inject = self.identities.get() == 3;
            }
            if inject {
                fs::create_dir_all(request.cwd.join(".runtime/global/new-empty"))?;
                self.injected.set(true);
            }
            Ok(output)
        }
    }
    for late in [false, true] {
        let (repo, remote) =
            versioned_repository_with_strategy("declared-directory-drift", true, true);
        let config_path = repo.0.join(".codex/bridgeforge-version.json");
        let mut cfg: serde_json::Value =
            serde_json::from_slice(&fs::read(&config_path).unwrap()).unwrap();
        cfg["build_inputs"] = json!([".runtime/global/"]);
        fs::write(config_path, serde_json::to_vec(&cfg).unwrap()).unwrap();
        fs::create_dir_all(repo.0.join(".runtime/global")).unwrap();
        fs::write(repo.0.join("tracked.txt"), b"changed package\n").unwrap();
        let request = fixture::analysis(&repo.0, &SystemProcessRunner, "patch", &[]).unwrap();
        let index = fs::read(repo.0.join(".git/index")).unwrap();
        let version = fs::read(repo.0.join("VERSION")).unwrap();
        let changelog = fs::read(repo.0.join("CHANGELOG.md")).unwrap();
        let runner = Drift {
            late,
            fetched: false.into(),
            identities: 0.into(),
            injected: false.into(),
        };
        let result = super::sync(
            &repo.0,
            &runner,
            GitSyncOptions {
                release: true,
                skip_push: true,
                message: Some("fix: directory guarded release".into()),
                release_request: request,
                ..Default::default()
            },
        );
        assert!(
            runner.injected.get(),
            "did not reach intended mutation boundary: {}",
            result.stderr
        );
        assert_eq!(result.code, 2, "{}", result.stderr);
        if late {
            assert!(
                result.stderr.contains("directory membership changed"),
                "{}",
                result.stderr
            );
        } else {
            assert!(
                result.stderr.contains("components: build"),
                "{}",
                result.stderr
            );
        }
        assert_eq!(fs::read(repo.0.join("VERSION")).unwrap(), version);
        assert_eq!(fs::read(repo.0.join("CHANGELOG.md")).unwrap(), changelog);
        assert_eq!(fs::read(repo.0.join(".git/index")).unwrap(), index);
        fs::remove_dir_all(remote).unwrap();
    }
}

#[test]
fn preparation_declared_directory_drift_is_not_certified() {
    struct CheckMutation;
    impl ProcessRunner for CheckMutation {
        fn run(&self, request: &ProcessRequest) -> std::io::Result<ProcessOutput> {
            if request.program == "fixture-prepare-check" {
                fs::create_dir_all(request.cwd.join(".runtime/global/added-empty"))?;
                return Ok(out(0, "fixture check passed but added an input"));
            }
            SystemProcessRunner.run(request)
        }
    }
    let (repo, remote) = versioned_repository_with_strategy("prepare-directory-drift", true, true);
    let config_path = repo.0.join(".codex/bridgeforge-version.json");
    let mut cfg: serde_json::Value =
        serde_json::from_slice(&fs::read(&config_path).unwrap()).unwrap();
    cfg["build_inputs"] = json!([".runtime/global/"]);
    fs::write(config_path, serde_json::to_vec(&cfg).unwrap()).unwrap();
    fs::create_dir_all(repo.0.join(".runtime/global")).unwrap();
    fs::write(repo.0.join(".codex/development-checks.json"),
        serde_json::to_vec(&json!({"schema":1,"checks":[{"id":"mutate","program":"fixture-prepare-check","args":[],"timeout_seconds":30}]})).unwrap()).unwrap();
    fs::write(repo.0.join("tracked.txt"), b"package change\n").unwrap();
    let result = prepare_release(&repo.0, &CheckMutation, "fix: preparation guard", None);
    assert_eq!(result.code, 2, "{}", result.stderr);
    assert!(
        result.stderr.contains("directory membership changed"),
        "{}",
        result.stderr
    );
    let receipt: serde_json::Value = serde_json::from_slice(
        &fs::read(
            repo.0
                .join(".runtime/bridgeforge-codex/release-preparation/current.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_ne!(receipt["status"], "prepared");
    assert_eq!(fs::read(repo.0.join("VERSION")).unwrap(), b"1.0.0\n");
    fs::remove_dir_all(remote).unwrap();
}

#[test]
fn preparation_record_scope_does_not_exempt_instructions_code_or_invalid_documents() {
    let (repo, remote) = versioned_repository("record-boundaries", true);
    install_record_documents(&repo.0);
    let prepared = prepare_release(&repo.0, &SystemProcessRunner, "fix: prepare", None);
    assert_eq!(prepared.code, 0, "{}", prepared.stderr);
    for relative in [
        "doc/3_reference/AGENTS.md",
        "doc/3_reference/SKILL.md",
        "doc/3_reference/tool.rs",
        "doc/0_architecture/design.md",
        "skills/example/SKILL.md",
    ] {
        let file = repo.0.join(relative);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(&file, b"new input\n").unwrap();
        assert_eq!(
            development_status(&repo.0, &SystemProcessRunner)
                .receipt
                .unwrap()["status"],
            "stale",
            "{relative}"
        );
        fs::remove_file(file).unwrap();
    }
    let doc = repo.0.join("doc/3_reference/invalid.md");
    fs::write(&doc, b"\xff\x00").unwrap();
    assert_eq!(
        development_status(&repo.0, &SystemProcessRunner)
            .receipt
            .unwrap()["status"],
        "stale"
    );
    fs::remove_file(&doc).unwrap();
    fs::write(repo.0.join("doc/README.md"), b"missing layout\n").unwrap();
    assert_eq!(
        development_status(&repo.0, &SystemProcessRunner)
            .receipt
            .unwrap()["status"],
        "stale"
    );
    fs::write(
        repo.0.join("doc/README.md"),
        b"---\ndelivery_layout: flat\n---\n# Docs\n",
    )
    .unwrap();
    let cfg_path = repo.0.join(".codex/development-checks.json");
    let cfg: serde_json::Value = serde_json::from_slice(&fs::read(&cfg_path).unwrap()).unwrap();
    for scope in [
        "skills/",
        "doc/",
        "doc/3_reference/../",
        "doc/3_reference/AGENTS.md",
    ] {
        let mut bad = cfg.clone();
        bad["record_documents"] = json!([scope]);
        fs::write(&cfg_path, serde_json::to_vec(&bad).unwrap()).unwrap();
        assert_ne!(
            development_status(&repo.0, &SystemProcessRunner).code,
            0,
            "{scope}"
        );
    }
    let mut changed = cfg;
    changed["record_documents"] = json!(["doc/README.md"]);
    fs::write(&cfg_path, serde_json::to_vec(&changed).unwrap()).unwrap();
    assert_eq!(
        development_status(&repo.0, &SystemProcessRunner)
            .receipt
            .unwrap()["status"],
        "stale"
    );
    fs::remove_dir_all(remote).unwrap();
}

#[test]
fn direct_release_ignores_missing_invalid_failed_or_stale_development_state() {
    for mode in ["missing", "invalid", "failed", "stale"] {
        let (repo, remote) = versioned_repository(mode, true);
        let config = repo.0.join(".codex/development-checks.json");
        if mode == "missing" {
            fs::remove_file(&config).unwrap();
        } else if mode == "invalid" {
            fs::write(&config, b"not-json\n").unwrap();
        } else {
            fs::write(&config, br#"{"schema":1,"audit_required":true,"checks":[{"id":"must-not-run","program":"not-a-real-test-program","args":[],"timeout_seconds":1}]}"#).unwrap();
        }
        let before_config = fs::read(&config).ok();
        let record = repo
            .0
            .join(".runtime/bridgeforge-codex/release-preparation/current.json");
        if mode != "missing" {
            fs::create_dir_all(record.parent().unwrap()).unwrap();
            fs::write(&record, br#"{"schema":1,"status":"failed","root":"another repository","target_version":"9.9.9"}"#).unwrap();
        }
        let before_record = fs::read(&record).ok();
        fs::write(repo.0.join("tracked.txt"), b"new business change\n").unwrap();
        let state = release_status(&repo.0, &SystemProcessRunner);
        assert_eq!(state.code, 0, "{mode}: {}", state.stderr);
        assert_eq!(state.receipt.unwrap()["status"], "release-ready");
        let result = sync(
            &repo.0,
            &SystemProcessRunner,
            GitSyncOptions {
                release: true,
                message: Some("fix: direct release".into()),
                ..Default::default()
            },
        );
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
        assert_eq!(result.code, 0, "{}", result.stderr);
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
    let receipt = result.receipt.unwrap();
    assert_eq!(receipt["status"], "synced");
    assert!(receipt["sync_elapsed_ms"].as_u64().is_some());
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
    let receipt = result.receipt.unwrap();
    assert_eq!(receipt["status"], "blocked");
    assert!(receipt["timings_ms"]["push"].is_number());
    assert!(receipt.get("push_performed").is_none());
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
    let receipt = result.receipt.unwrap();
    assert_eq!(receipt["status"], "blocked");
    assert!(receipt["sync_elapsed_ms"].is_number());
    assert!(receipt.get("push_performed").is_none());
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
fn repository_identity_batches_facts_without_caching_later_guards() {
    struct Counting(std::cell::Cell<usize>);
    impl ProcessRunner for Counting {
        fn run(&self, request: &ProcessRequest) -> std::io::Result<ProcessOutput> {
            if request.args.first().is_some_and(|arg| arg == "rev-parse") {
                self.0.set(self.0.get() + 1);
            }
            SystemProcessRunner.run(request)
        }
    }
    let repo = real_repository("identity-batch");
    let counter = Counting(std::cell::Cell::new(0));
    let git = Git {
        root: &repo.0,
        runner: &counter,
    };
    let first = RepositoryIdentity::capture(&git).unwrap();
    assert_eq!(counter.0.get(), 2); // Batched facts plus independent split-index query.
    let independent = Git {
        root: &repo.0,
        runner: &SystemProcessRunner,
    };
    for (args, expected) in [
        (
            vec!["rev-parse", "--path-format=absolute", "--git-dir"],
            first.git_dir.as_str(),
        ),
        (
            vec!["rev-parse", "--path-format=absolute", "--git-common-dir"],
            first.common_dir.as_str(),
        ),
        (
            vec!["rev-parse", "--path-format=absolute", "--git-path", "index"],
            first.index_path.to_str().unwrap(),
        ),
        (vec!["rev-parse", "HEAD"], first.head_oid.as_str()),
    ] {
        assert_eq!(
            independent
                .required(&args, Duration::from_secs(30))
                .unwrap(),
            expected
        );
    }
    fs::write(repo.0.join("tracked.txt"), b"next\n").unwrap();
    git_ok(&repo.0, &["add", "."]);
    git_ok(&repo.0, &["commit", "-m", "next identity"]);
    let second = RepositoryIdentity::capture(&git).unwrap();
    assert_eq!(counter.0.get(), 4);
    assert_ne!(first.head_oid, second.head_oid);
    assert!(first.same_layout_and_config(&second));
}

#[test]
fn repository_identity_rejects_malformed_batch_instead_of_using_partial_facts() {
    let root = std::env::temp_dir();
    let directory = root.to_string_lossy();
    for body in [
        "".to_owned(),
        format!("{directory}\n{directory}\n{directory}\nnot-an-oid"),
        format!("relative\n{directory}\n{directory}\n{}", "a".repeat(40)),
        format!(
            "{directory}\n{directory}\n{directory}\n{}\nextra",
            "a".repeat(40)
        ),
    ] {
        let runner = FakeRunner {
            outputs: RefCell::new(vec![out(0, &body)]),
        };
        let git = Git {
            root: &root,
            runner: &runner,
        };
        assert!(
            RepositoryIdentity::capture(&git)
                .unwrap_err()
                .contains("unexpected repository identity")
        );
    }
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
            release_request: Default::default(),
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
            release_request: Default::default(),
        },
    );
    assert_eq!(outcome.code, 2, "{}", outcome.stderr);
    assert!(outcome.stderr.contains("after successful push"));
    let receipt = outcome.receipt.unwrap();
    assert_eq!(receipt["status"], "blocked");
    assert!(receipt["sync_elapsed_ms"].is_number());
    assert!(receipt.get("push_performed").is_none());
    assert!(!adaptation.exists());
    fs::remove_dir_all(remote).unwrap();
}

#[test]
fn preflight_clean_and_dirty_context_never_writes_or_fetches() {
    struct ReadQueries;
    impl ProcessRunner for ReadQueries {
        fn run(&self, request: &ProcessRequest) -> std::io::Result<ProcessOutput> {
            assert_eq!(request.program, "git");
            assert_eq!(
                request.env.get(std::ffi::OsStr::new("GIT_OPTIONAL_LOCKS")),
                Some(&OsString::from("0"))
            );
            assert_eq!(
                &request.args[..2],
                &[OsString::from("-c"), OsString::from("core.fsmonitor=false")]
            );
            assert_eq!(
                &request.args[2..4],
                &[
                    OsString::from("-c"),
                    OsString::from("diff.autoRefreshIndex=false")
                ]
            );
            assert!(matches!(
                request.args.get(4).and_then(|v| v.to_str()),
                Some(
                    "config"
                        | "ls-files"
                        | "check-attr"
                        | "symbolic-ref"
                        | "rev-parse"
                        | "status"
                        | "diff"
                )
            ));
            SystemProcessRunner.run(request)
        }
    }
    for dirty in [false, true] {
        let (repo, remote) = managed_repository("preflight-read-only");
        let config = repo.0.join(".codex/bridgeforge-version.json");
        fs::write(&config, br#"{"schema_version":1,"release_policy":"explicit_release","manifest_policy":"auto","git_sync_setup":{"schema":1,"confirmed":true,"selection":"current"}}"#).unwrap();
        git_ok(&repo.0, &["add", "."]);
        git_ok(&repo.0, &["commit", "-m", "chore: confirm policy"]);
        git_ok(&repo.0, &["push"]);
        if dirty {
            fs::write(repo.0.join("tracked.txt"), b"staged\n").unwrap();
            git_ok(&repo.0, &["add", "tracked.txt"]);
            fs::write(repo.0.join("tracked.txt"), b"unstaged\n").unwrap();
            fs::write(repo.0.join("untracked.txt"), b"new\n").unwrap();
        }
        fs::write(
            repo.0.join(".git/hooks/preflight-monitor"),
            b"#!/bin/sh\nprintf invoked > monitor-marker\nprintf 'token\\0'\n",
        )
        .unwrap();
        git_ok(
            &repo.0,
            &["config", "core.fsmonitor", ".git/hooks/preflight-monitor"],
        );
        git_ok(&repo.0, &["status", "--porcelain=v1"]);
        assert!(
            repo.0.join("monitor-marker").exists(),
            "fixture must execute the configured monitor without the read-only override"
        );
        fs::remove_file(repo.0.join("monitor-marker")).unwrap();
        // A byte-identical rewrite plus another dirty file triggers Git's
        // optional diff index refresh unless the explicit read-only flag is set.
        fs::write(repo.0.join("managed.txt"), b"old\n").unwrap();
        let paths = [repo.0.join(".git/index"), repo.0.join(".git/HEAD"), config];
        let before = paths
            .iter()
            .map(|p| {
                (
                    fs::read(p).unwrap(),
                    fs::metadata(p).unwrap().modified().unwrap(),
                )
            })
            .collect::<Vec<_>>();
        let result = preflight(&repo.0, &ReadQueries);
        assert_eq!(result.code, 0, "{}", result.stderr);
        let receipt = result.receipt.unwrap();
        assert_eq!(receipt["status"], "ready");
        assert_eq!(
            receipt["working_tree"],
            if dirty { "dirty" } else { "clean" }
        );
        assert_eq!(receipt["diff_review_required"], dirty);
        assert_eq!(receipt["remote_state"], "not_fetched");
        assert_eq!(receipt["upstream"], receipt["push_target"]);
        if dirty {
            assert!(
                receipt["status_porcelain"]
                    .as_str()
                    .unwrap()
                    .contains("untracked.txt")
            );
        }
        for (path, (bytes, time)) in paths.iter().zip(before) {
            assert_eq!(fs::read(path).unwrap(), bytes);
            assert_eq!(fs::metadata(path).unwrap().modified().unwrap(), time);
        }
        assert!(!repo.0.join(".runtime").exists());
        assert!(!repo.0.join(".git/bridgeforge-git-sync.lock").exists());
        assert!(
            !repo.0.join("monitor-marker").exists(),
            "preflight must not execute the configured monitor"
        );
        fs::remove_dir_all(remote).unwrap();
    }
}

#[test]
fn preflight_unconfirmed_and_invalid_configuration_remain_zero_write() {
    let repo = real_repository("preflight-choice");
    let index = fs::read(repo.0.join(".git/index")).unwrap();
    let first = preflight(&repo.0, &SystemProcessRunner);
    assert_eq!(first.code, crate::EXIT_BLOCKED);
    let receipt = first.receipt.unwrap();
    assert_eq!(receipt["status"], "blocked");
    assert!(
        !first.stderr.contains("configuration-required"),
        "Git-only preflight must reach repository checks without requiring release setup"
    );
    assert!(!repo.0.join(".codex/bridgeforge-version.json").exists());
    fs::create_dir_all(repo.0.join(".codex")).unwrap();
    let config = repo.0.join(".codex/bridgeforge-version.json");
    fs::write(&config, b"{invalid").unwrap();
    let second = preflight(&repo.0, &SystemProcessRunner);
    assert_eq!(second.code, crate::EXIT_BLOCKED);
    assert_eq!(second.receipt.unwrap()["status"], "blocked");
    assert_eq!(fs::read(&config).unwrap(), b"{invalid");
    assert_eq!(fs::read(repo.0.join(".git/index")).unwrap(), index);
    assert!(!repo.0.join(".runtime").exists());
}

#[test]
fn preflight_blocks_clean_filters_before_running_project_helpers() {
    let (repo, remote) = managed_repository("preflight-clean-filter");
    fs::write(repo.0.join(".codex/bridgeforge-version.json"),br#"{"schema_version":1,"manifest_policy":"auto","git_sync_setup":{"schema":1,"confirmed":true,"selection":"current"}}"#).unwrap();
    fs::write(
        repo.0.join(".git/hooks/preflight-clean"),
        b"#!/bin/sh\nprintf invoked > clean-marker\ncat\n",
    )
    .unwrap();
    fs::write(
        repo.0.join(".gitattributes"),
        b"tracked.txt filter=fixture\n",
    )
    .unwrap();
    git_ok(
        &repo.0,
        &[
            "config",
            "filter.fixture.clean",
            ".git/hooks/preflight-clean",
        ],
    );
    fs::write(repo.0.join("tracked.txt"), b"modified by fixture\n").unwrap();
    git_ok(&repo.0, &["status", "--porcelain=v1"]);
    git_ok(
        &repo.0,
        &["diff", "--no-ext-diff", "--no-textconv", "--stat"],
    );
    assert!(
        repo.0.join("clean-marker").exists(),
        "fixture must demonstrate ordinary Git inspection can execute the clean helper"
    );
    fs::remove_file(repo.0.join("clean-marker")).unwrap();
    let before = fs::read(repo.0.join(".git/index")).unwrap();
    let result = preflight(&repo.0, &SystemProcessRunner);
    assert_eq!(result.code, crate::EXIT_BLOCKED);
    assert!(
        result.receipt.unwrap()["reason"]
            .as_str()
            .unwrap()
            .contains("clean/process")
    );
    assert!(!repo.0.join("clean-marker").exists());
    assert_eq!(fs::read(repo.0.join(".git/index")).unwrap(), before);
    fs::remove_dir_all(remote).unwrap();
}

#[test]
fn preflight_rejects_truncated_attributes_and_submodule_helper_scope() {
    struct MissingAttributes;
    impl ProcessRunner for MissingAttributes {
        fn run(&self, r: &ProcessRequest) -> std::io::Result<ProcessOutput> {
            if r.args.iter().any(|v| v == "check-attr") {
                return Ok(out(0, ""));
            }
            SystemProcessRunner.run(r)
        }
    }
    let (repo, remote) = managed_repository("preflight-attribute-guard");
    fs::write(repo.0.join(".codex/bridgeforge-version.json"),br#"{"schema_version":1,"manifest_policy":"auto","git_sync_setup":{"schema":1,"confirmed":true,"selection":"current"}}"#).unwrap();
    git_ok(&repo.0, &["config", "filter.fixture.clean", "cat"]);
    let result = preflight(&repo.0, &MissingAttributes);
    assert_eq!(result.code, crate::EXIT_BLOCKED);
    assert!(
        result.receipt.unwrap()["reason"]
            .as_str()
            .unwrap()
            .contains("attribute framing")
    );
    let git = Git {
        root: &repo.0,
        runner: &SystemProcessRunner,
    };
    let head = git
        .required(&["rev-parse", "HEAD"], Duration::from_secs(30))
        .unwrap();
    git_ok(
        &repo.0,
        &[
            "update-index",
            "--add",
            "--cacheinfo",
            "160000",
            &head,
            "nested",
        ],
    );
    let before = fs::read(repo.0.join(".git/index")).unwrap();
    let result = preflight(&repo.0, &SystemProcessRunner);
    assert_eq!(result.code, crate::EXIT_BLOCKED);
    assert!(
        result.receipt.unwrap()["reason"]
            .as_str()
            .unwrap()
            .contains("submodule")
    );
    assert_eq!(fs::read(repo.0.join(".git/index")).unwrap(), before);
    fs::remove_dir_all(remote).unwrap();
}

#[test]
fn preflight_missing_push_target_blocks_without_fabricating_parity() {
    let repo = real_repository("preflight-no-remote");
    fs::create_dir_all(repo.0.join(".codex")).unwrap();
    fs::write(repo.0.join(".codex/bridgeforge-version.json"),br#"{"schema_version":1,"manifest_policy":"auto","git_sync_setup":{"schema":1,"confirmed":true,"selection":"current"}}"#).unwrap();
    let result = preflight(&repo.0, &SystemProcessRunner);
    assert_eq!(result.code, crate::EXIT_BLOCKED);
    let receipt = result.receipt.unwrap();
    assert_eq!(receipt["status"], "blocked");
    assert!(receipt["reason"].as_str().unwrap().contains("rev-parse"));
    assert!(receipt.get("ahead").is_none());
    assert!(receipt.get("working_tree").is_none());
}

#[test]
fn preflight_distinguishes_push_target_and_blocks_detached_head() {
    let (repo, remote) = managed_repository("preflight-push-target");
    fs::write(repo.0.join(".codex/bridgeforge-version.json"),br#"{"schema_version":1,"manifest_policy":"auto","git_sync_setup":{"schema":1,"confirmed":true,"selection":"current"}}"#).unwrap();
    git_ok(&repo.0, &["add", "."]);
    git_ok(&repo.0, &["commit", "-m", "chore: confirm policy"]);
    git_ok(&repo.0, &["push"]);
    let fork = repo.0.with_extension("fork.git");
    git_ok(
        &repo.0,
        &[
            "clone",
            "--bare",
            remote.to_str().unwrap(),
            fork.to_str().unwrap(),
        ],
    );
    git_ok(&repo.0, &["remote", "add", "fork", fork.to_str().unwrap()]);
    git_ok(&repo.0, &["config", "remote.pushDefault", "fork"]);
    git_ok(&repo.0, &["config", "push.default", "current"]);
    git_ok(&repo.0, &["fetch", "fork"]);
    let result = preflight(&repo.0, &SystemProcessRunner);
    assert_eq!(result.code, 0, "{}", result.stderr);
    let receipt = result.receipt.unwrap();
    assert!(receipt["upstream"].as_str().unwrap().starts_with("origin/"));
    assert!(
        receipt["push_target"]
            .as_str()
            .unwrap()
            .starts_with("fork/")
    );
    git_ok(&repo.0, &["checkout", "--detach"]);
    let index = fs::read(repo.0.join(".git/index")).unwrap();
    let result = preflight(&repo.0, &SystemProcessRunner);
    assert_eq!(result.code, crate::EXIT_BLOCKED);
    assert!(
        result.receipt.unwrap()["reason"]
            .as_str()
            .unwrap()
            .contains("symbolic-ref")
    );
    assert_eq!(fs::read(repo.0.join(".git/index")).unwrap(), index);
    fs::remove_dir_all(remote).unwrap();
    fs::remove_dir_all(fork).unwrap();
}

#[test]
fn phase_timing_observer_preserves_artifact_cache_runner_contract() {
    struct CacheAware {
        hits: std::cell::Cell<usize>,
        misses: RefCell<Vec<&'static str>>,
    }
    impl ProcessRunner for CacheAware {
        fn run(&self, _: &ProcessRequest) -> std::io::Result<ProcessOutput> {
            Ok(out(0, "ok"))
        }
        fn artifact_cache_enabled(&self) -> bool {
            true
        }
        fn cache_hit(&self) {
            self.hits.set(self.hits.get() + 1)
        }
        fn cache_miss(&self, reason: &'static str) {
            self.misses.borrow_mut().push(reason)
        }
    }
    let runner = CacheAware {
        hits: Default::default(),
        misses: Default::default(),
    };
    let timings = timing::Timings::new();
    let observed = timing::Observed {
        runner: &runner,
        timings: &timings,
    };
    assert!(observed.artifact_cache_enabled());
    observed.cache_hit();
    observed.cache_miss("entry_unavailable_or_invalid");
    assert_eq!(runner.hits.get(), 1);
    assert_eq!(
        *runner.misses.borrow(),
        vec!["entry_unavailable_or_invalid"]
    );
}

#[test]
fn phase_timing_retains_tls_timeout_and_launch_failures_without_retry() {
    struct FailingFetch {
        kind: &'static str,
        fetches: std::cell::Cell<usize>,
    }
    impl ProcessRunner for FailingFetch {
        fn run(&self, r: &ProcessRequest) -> std::io::Result<ProcessOutput> {
            if r.program == "git" && r.args.first().is_some_and(|v| v == "fetch") {
                self.fetches.set(self.fetches.get() + 1);
                std::thread::sleep(Duration::from_millis(12));
                if self.kind == "launch" {
                    return Err(std::io::Error::other("fixture launch failure"));
                }
                return Ok(ProcessOutput {
                    code: 128,
                    stdout: Vec::new(),
                    stderr: b"SSL_ERROR_SYSCALL".to_vec(),
                    timed_out: self.kind == "timeout",
                });
            }
            SystemProcessRunner.run(r)
        }
    }
    for kind in ["tls", "timeout", "launch"] {
        let (repo, remote) = managed_repository("timed-fetch-error");
        let before = fs::read(repo.0.join(".git/index")).unwrap();
        let runner = FailingFetch {
            kind,
            fetches: Default::default(),
        };
        let result = sync(&repo.0, &runner, GitSyncOptions::default());
        assert_eq!(result.code, crate::EXIT_BLOCKED);
        assert_eq!(runner.fetches.get(), 1);
        assert!(result.stderr.contains(match kind {
            "tls" => "SSL_ERROR_SYSCALL",
            "timeout" => "timed out",
            _ => "fixture launch failure",
        }));
        let receipt = result.receipt.unwrap();
        assert_eq!(receipt["status"], "blocked");
        let fetch = receipt["timings_ms"]["fetch"].as_u64().unwrap();
        assert!(fetch >= 10);
        assert!(receipt["sync_elapsed_ms"].as_u64().unwrap() >= fetch);
        assert_eq!(receipt["timings_ms"]["commit_and_hooks"], 0);
        assert_eq!(receipt["timings_ms"]["push"], 0);
        assert!(receipt.get("push_performed").is_none());
        assert!(receipt.get("working_tree").is_none());
        assert_eq!(fs::read(repo.0.join(".git/index")).unwrap(), before);
        fs::remove_dir_all(remote).unwrap();
    }
}

#[test]
fn phase_timing_success_preserves_commit_hook_and_remote_receipt() {
    let (repo, remote) = managed_repository("timed-commit");
    fs::write(repo.0.join("tracked.txt"), b"business change\n").unwrap();
    fs::write(repo.0.join(".git/hooks/pre-commit"), b"#!/bin/sh\nexit 0\n").unwrap();
    let result = sync(
        &repo.0,
        &SystemProcessRunner,
        GitSyncOptions {
            message: Some("chore: timing fixture".into()),
            ..Default::default()
        },
    );
    assert_eq!(result.code, 0, "{}", result.stderr);
    let receipt = result.receipt.unwrap();
    assert_eq!(receipt["status"], "synced");
    assert_eq!(receipt["working_tree"], "clean");
    assert_eq!(receipt["ahead"], 0);
    assert_eq!(receipt["behind"], 0);
    assert_eq!(receipt["push_performed"], true);
    let total = receipt["sync_elapsed_ms"].as_u64().unwrap();
    for value in receipt["timings_ms"].as_object().unwrap().values() {
        assert!(value.as_u64().unwrap() <= total);
    }
    assert!(receipt["timings_ms"]["commit_and_hooks"].as_u64().unwrap() > 0);
    assert!(
        receipt["timing_semantics"]
            .as_str()
            .unwrap()
            .contains("must not be summed")
    );
    fs::remove_dir_all(remote).unwrap();
}
