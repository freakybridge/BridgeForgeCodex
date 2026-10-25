//! Factory materialization, cache, preparation and rollback tests.
use crate::git_sync_plan as write_plan;
use bridgeforge_core::{
    CommandOutcome, ProcessOutput, ProcessRequest, ProcessRunner, SystemProcessRunner,
};
use bridgeforge_workflow::git_sync::{self, GitSyncOptions, development_status, release_status};
use serde_json::json;
use std::{
    fs,
    path::{Path, PathBuf},
};

#[path = "../../support/schema4_agent.rs"]
mod fixture;
#[path = "../../support/git_repository.rs"]
mod repository;
use repository::*;

fn sync(root: &Path, runner: &dyn ProcessRunner, mut options: GitSyncOptions) -> CommandOutcome {
    let factory_runner = crate::runner::FactoryRunner::new(runner);
    if options.release
        && options.release_request.analysis_file.is_none()
        && fs::read(root.join(".codex/bridgeforge-version.json"))
            .ok()
            .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
            .is_some_and(|v| v["schema_version"] == 4)
    {
        match fixture::analysis(root, &factory_runner, "patch", &[]) {
            Ok(request) => options.release_request = request,
            Err(error) => return CommandOutcome::blocked(error),
        }
    }
    git_sync::sync(root, &factory_runner, options)
}

fn prepare_release(
    root: &Path,
    runner: &dyn ProcessRunner,
    message: &str,
    audit: Option<&Path>,
) -> CommandOutcome {
    let factory_runner = crate::runner::FactoryRunner::new(runner);
    let request = if fs::read(root.join(".codex/bridgeforge-version.json"))
        .ok()
        .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
        .is_some_and(|v| v["schema_version"] == 4)
    {
        match fixture::analysis(root, &factory_runner, "patch", &[]) {
            Ok(request) => request,
            Err(error) => return CommandOutcome::blocked(error),
        }
    } else {
        Default::default()
    };
    git_sync::prepare_release_with_request(root, &factory_runner, message, audit, &request)
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
    fixture::migrate(&repo.0, &SystemProcessRunner).unwrap();
    git_ok(&repo.0, &["add", "."]);
    git_ok(&repo.0, &["commit", "-m", "chore: schema4 factory fixture"]);
    fs::remove_dir_all(repo.0.join(".runtime")).unwrap();
    fs::write(repo.0.join("tracked.txt"), b"user change\n").unwrap();
    (repo, remote)
}

struct GeneratedRunner {
    root: PathBuf,
    mode: &'static str,
}

fn is_short_cache_probe(program: &Path) -> bool {
    program.parent().is_some_and(|parent| {
        parent.parent() == Some(std::env::temp_dir().as_path())
            && parent
                .file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with("bf-probe-"))
    })
}

impl ProcessRunner for GeneratedRunner {
    fn run(&self, request: &ProcessRequest) -> std::io::Result<ProcessOutput> {
        if request
            .args
            .first()
            .is_some_and(|arg| matches!(arg.to_str(), Some("--version" | "-vV" | "-Vv")))
            && (request.program == "cargo" || request.program == "rustc")
        {
            return Ok(out(
                0,
                &format!("{} 1.88.0 (fixture)", request.program.to_string_lossy()),
            ));
        }
        if self.mode == "policy-drift" && request.args.iter().any(|arg| arg == "check-ignore") {
            fs::write(
                self.root.join(".codex/bridgeforge-version.json"),
                b"external policy edit\n",
            )?;
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
        let count = if matches!(mode, "source" | "lock") {
            2
        } else {
            0
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
    let request = fixture::analysis(&repo.0, &runner, "patch", &[]).unwrap();
    let release = crate::release::build_explicit_release_plan_with_request(
        &repo.0,
        "fix: release",
        vec!["tracked.txt".into()],
        &runner,
        &request,
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
            if Path::new(&request.program).starts_with(self.inner.root.join(".codex/bin"))
                || Path::new(&request.program)
                    .starts_with(self.inner.root.join(crate::artifact_cache::DIRECTORY))
                || is_short_cache_probe(Path::new(&request.program))
            {
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
            if request.program == "cargo" && request.args.first().is_some_and(|arg| arg == "build")
            {
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
    for corrupt in [false, true] {
        let (repo, remote) = factory_repository("independent-cache-record");
        install_development_check(&repo.0);
        let runner = GeneratedRunner {
            root: repo.0.clone(),
            mode: "ok",
        };
        assert_eq!(
            prepare_release(&repo.0, &runner, "fix: cache", None).code,
            0
        );
        let record = repo
            .0
            .join(".runtime/bridgeforge-codex/release-preparation/current.json");
        if corrupt {
            fs::write(&record, b"{invalid prepared/audit").unwrap();
        } else {
            fs::remove_file(record).unwrap();
        }
        struct NoBuild<'a>(&'a GeneratedRunner);
        impl ProcessRunner for NoBuild<'_> {
            fn run(&self, r: &ProcessRequest) -> std::io::Result<ProcessOutput> {
                assert!(
                    !(r.program == "cargo" && r.args.first().is_some_and(|a| a == "build")),
                    "cache hit must not compile"
                );
                self.0.run(r)
            }
        }
        let result = sync(
            &repo.0,
            &NoBuild(&runner),
            GitSyncOptions {
                release: true,
                message: Some("fix: new description".into()),
                ..Default::default()
            },
        );
        assert_eq!(result.code, 0, "{}", result.stderr);
        let receipt = result.receipt.unwrap();
        assert_eq!(receipt["generated_assets_built"], 0);
        assert_eq!(receipt["generated_assets_reused"], 2);
        fs::remove_dir_all(remote).unwrap();
    }
}

#[test]
fn mixed_cache_hit_and_build_rejected_commit_restore_all_targets() {
    let (repo, remote) = factory_repository("mixed-cache-rollback");
    install_development_check(&repo.0);
    let runner = GeneratedRunner {
        root: repo.0.clone(),
        mode: "ok",
    };
    assert_eq!(
        prepare_release(&repo.0, &runner, "fix: cache", None).code,
        0
    );
    let directory = repo.0.join(crate::artifact_cache::DIRECTORY);
    for item in fs::read_dir(&directory).unwrap() {
        let item = item.unwrap();
        let meta = item.path().join("entry.json");
        if meta.is_file() {
            let value: serde_json::Value =
                serde_json::from_slice(&fs::read(meta).unwrap()).unwrap();
            if value["identity"]["id"] == "codex.bridgeforge-cli" {
                fs::remove_dir_all(item.path()).unwrap();
            }
        }
    }
    fs::write(repo.0.join(".git/hooks/pre-commit"), b"#!/bin/sh\nexit 1\n").unwrap();
    git_ok(&repo.0, &["status", "--porcelain=v1"]);
    let index = fs::read(repo.0.join(".git/index")).unwrap();
    struct MixedRunner {
        inner: GeneratedRunner,
        builds: std::cell::Cell<usize>,
        hits: std::cell::Cell<usize>,
    }
    impl ProcessRunner for MixedRunner {
        fn run(&self, r: &ProcessRequest) -> std::io::Result<ProcessOutput> {
            if r.program == "cargo" && r.args.first().is_some_and(|a| a == "build") {
                self.builds.set(self.builds.get() + 1);
            }
            if (Path::new(&r.program)
                .starts_with(self.inner.root.join(crate::artifact_cache::DIRECTORY))
                || is_short_cache_probe(Path::new(&r.program)))
                && r.args.first().is_some_and(|a| a == "self-test")
            {
                self.hits.set(self.hits.get() + 1);
            }
            self.inner.run(r)
        }
    }
    let paths = [
        "VERSION",
        "CHANGELOG.md",
        "templates/hooks/Cargo.toml",
        "templates/hooks/Cargo.lock",
        ".codex/hooks/Cargo.toml",
        ".codex/hooks/Cargo.lock",
        "templates/managed-skeleton.json",
        ".codex/managed-skeleton.json",
        "bridgeforge-codex-manifest.json",
        crate::user_agents::MANIFEST,
        ".codex/bin/build-receipt-cli.json",
        ".codex/bin/build-receipt-hook.json",
    ];
    let mut before = paths
        .into_iter()
        .map(|p| {
            let p = repo.0.join(p);
            let bytes = fs::read(&p).ok();
            (p, bytes)
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    for name in ["bridgeforge", "bridgeforge-hook"] {
        let path = repo.0.join(".codex/bin").join(if cfg!(windows) {
            format!("{name}.exe")
        } else {
            name.into()
        });
        before.insert(path.clone(), fs::read(path).ok());
    }
    let counted = MixedRunner {
        inner: runner,
        builds: Default::default(),
        hits: Default::default(),
    };
    let result = sync(
        &repo.0,
        &counted,
        GitSyncOptions {
            release: true,
            message: Some("fix: mixed cache".into()),
            skip_fetch: true,
            skip_push: true,
            ..Default::default()
        },
    );
    assert_eq!(result.code, 2, "{}", result.stderr);
    assert!(result.stderr.contains("rolled back"));
    assert_eq!(fs::read(repo.0.join("VERSION")).unwrap(), b"1.0.0\n");
    assert_eq!(fs::read(repo.0.join(".git/index")).unwrap(), index);
    assert!(!repo.0.join(".codex/bin/bridgeforge.exe").exists());
    assert!(!repo.0.join(".codex/bin/bridgeforge-hook.exe").exists());
    assert_eq!(
        counted.builds.get(),
        1,
        "mixed scenario must really compile one miss"
    );
    assert_eq!(
        counted.hits.get(),
        1,
        "mixed scenario must really self-test one cache hit"
    );
    for (path, bytes) in before {
        assert_eq!(
            fs::read(&path).ok(),
            bytes,
            "automatic target drift: {}",
            path.display()
        );
    }
    fs::remove_dir_all(remote).unwrap();
}

#[test]
fn factory_read_only_status_queries_do_not_initialize_artifact_cache() {
    let (repo, remote) = factory_repository("cache-readonly");
    assert!(!repo.0.join(crate::artifact_cache::DIRECTORY).exists());
    let runner = GeneratedRunner {
        root: repo.0.clone(),
        mode: "fail",
    };
    let _ = release_status(&repo.0, &runner);
    let _ = development_status(&repo.0, &runner);
    assert!(!repo.0.join(crate::artifact_cache::DIRECTORY).exists());
    assert_eq!(fs::read(repo.0.join("VERSION")).unwrap(), b"1.0.0\n");
    fs::remove_dir_all(remote).unwrap();
}

#[test]
fn preparation_record_documents_reuse_artifacts_but_bind_release_content() {
    let (repo, remote) = factory_repository("record-release");
    install_record_documents(&repo.0);
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
    let key = prepared.receipt.unwrap()["validation_fingerprint"].clone();
    fs::write(
        repo.0.join("doc/3_reference/lessons.md"),
        b"# Lessons\nA tested solution.\n",
    )
    .unwrap();
    fs::write(repo.0.join("doc/3_reference/acceptance.md"), "---\nlifecycle: completed\nvalidation_status: verified\n---\nTODO-001 用户已验收本事项。\n").unwrap();
    let todo_file = repo.0.join("doc/0_architecture/TODO-INDEX.md");
    let todo_text = "# 项目待办总表\n| 编号 | 日期 | 内容 | 状态 | 归属 | 关联文档 |\n|---|---|---|---|---|---|\n| TODO-001 | 2026-10-11 | Completed | 已完成 | 原需求 | [验收](../3_reference/acceptance.md) |\n| TODO-002 | 2026-10-11 | Known remaining idea | 待确认 | 原需求 | 本轮用户输入，暂无正式文档 |\n";
    fs::write(&todo_file, todo_text).unwrap();
    let status = development_status(&repo.0, &SystemProcessRunner)
        .receipt
        .unwrap();
    assert_eq!(status["status"], "prepared");
    assert_eq!(status["validation_fingerprint"], key);
    assert_eq!(status["record_documents_changed"], true);
    assert_eq!(status["record_documents_check"], "passed");
    fs::write(&todo_file, "# TODO\n- [x] Unsupported legacy format\n").unwrap();
    assert_eq!(
        development_status(&repo.0, &SystemProcessRunner)
            .receipt
            .unwrap()["status"],
        "stale"
    );
    fs::write(&todo_file, todo_text).unwrap();
    fs::write(
        repo.0.join("doc/3_reference/lessons.md"),
        b"# Changed after binding\n",
    )
    .unwrap();
    assert_eq!(
        development_status(&repo.0, &SystemProcessRunner)
            .receipt
            .unwrap()["status"],
        "prepared"
    );
    let record = repo
        .0
        .join("doc/1_delivery/sample/requirements_2026-10-09_sample.md");
    fs::create_dir_all(record.parent().unwrap()).unwrap();
    fs::write(repo.0.join("doc/README.md"), b"---\ndelivery_layout: flat\n---\n[sample](1_delivery/sample/requirements_2026-10-09_sample.md)\n").unwrap();
    fs::write(
        &record,
        b"---\nlifecycle: active\nvalidation_status: in_progress\n---\n# Task\n",
    )
    .unwrap();
    fs::write(
        &record,
        b"---\nlifecycle: invalid-lifecycle\nvalidation_status: in_progress\n---\n# Task\n",
    )
    .unwrap();
    assert_eq!(
        development_status(&repo.0, &SystemProcessRunner)
            .receipt
            .unwrap()["status"],
        "stale"
    );
    fs::write(
        &record,
        b"---\nlifecycle: completed\nvalidation_status: verified\n---\n# Task\n",
    )
    .unwrap();
    let result = sync(
        &repo.0,
        &GeneratedRunner {
            root: repo.0.clone(),
            mode: "ok",
        },
        GitSyncOptions {
            release: true,
            message: Some("fix: release records".into()),
            ..Default::default()
        },
    );
    assert_eq!(result.code, 0, "{}", result.stderr);
    let receipt = result.receipt.unwrap();
    assert_eq!(receipt["generated_assets_built"], 0);
    assert_eq!(receipt["generated_assets_reused"], 2);
    assert_eq!(receipt["working_tree"], "clean");
    let committed = git_output(&repo.0, &["show", "HEAD:doc/3_reference/lessons.md"]);
    assert!(committed.contains("Changed after binding"));
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
fn independent_preparation_rejects_contract_drift_without_gating_git() {
    let (repo, remote) = factory_repository("development-contract-drift");
    install_development_check(&repo.0);
    let ready = prepare_release(
        &repo.0,
        &GeneratedRunner {
            root: repo.0.clone(),
            mode: "ok",
        },
        "fix: prepare",
        None,
    );
    assert_eq!(ready.code, 0, "{}", ready.stderr);
    let file = repo.0.join("templates/managed-skeleton.json");
    let mut contract: serde_json::Value =
        serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
    contract["external_marker"] = json!(true);
    fs::write(&file, serde_json::to_vec(&contract).unwrap()).unwrap();
    let state = development_status(&repo.0, &SystemProcessRunner)
        .receipt
        .unwrap();
    assert_eq!(state["status"], "stale");
    assert_eq!(fs::read(repo.0.join("VERSION")).unwrap(), b"1.0.0\n");
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&fs::read(file).unwrap()).unwrap()["external_marker"],
        true
    );
    fs::remove_dir_all(remote).unwrap();
}
