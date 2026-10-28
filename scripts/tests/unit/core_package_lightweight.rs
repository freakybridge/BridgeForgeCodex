//! Current-task validation and durable version coverage are separate contracts.
mod staged_tests {
    //! Task validation and closure share final-candidate evidence, not chat claims.
    use super::*;

    fn staged_fixture(stable: bool) -> Repo {
        let r = zero_fixture();
        configure(&r.0, stable);
        // The runner below simulates execution, but the actual entrypoint bytes are
        // bound by the same production tool-identity check.
        write(&r.0, "pkg/check-tool.exe", b"fixture tool identity v1");
        let mut cfg: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
        cfg["development"]["a"]["environment"] = json!({
            "tools":["./pkg/check-tool.exe","cargo","rustc"],"variables":[],"inputs":{}
        });
        for check in cfg["development"]["a"]["checks"].as_array_mut().unwrap() {
            check["program"] = json!("./pkg/check-tool.exe");
        }
        if stable {
            for check in cfg["development"]["a"]["stability"]["checks"]
                .as_array_mut()
                .unwrap()
            {
                check["program"] = json!("./pkg/check-tool.exe");
            }
        }
        write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
        write(&r.0, "pkg/src/lib.rs", b"pub fn value()->u8{2}\n");
        r
    }

    fn validate(root: &Path, q: &Request, runner: &Checks) -> crate::CommandOutcome {
        development::validate_for_packages(root, q, None, runner, None)
    }

    #[test]
    fn staged_validation_restores_versions_and_closure_reuses_final_candidate() {
        for stable in [false, true] {
            let r = staged_fixture(stable);
            let runner = Checks::new();
            let q = request(&r.0, "patch", false, &runner);
            let before = fs::read(r.0.join("pkg/Cargo.toml")).unwrap();
            let started = std::time::Instant::now();
            let checked = validate(&r.0, &q, &runner);
            assert_eq!(checked.code, 0, "{}", checked.stderr);
            assert_eq!(checked.receipt.unwrap()["status"], "development-validated");
            assert_eq!(fs::read(r.0.join("pkg/Cargo.toml")).unwrap(), before);
            assert!(!r.0.join(development::RECORD).exists());
            assert!(
                !r.0.join(".runtime/bridgeforge-codex/develop-transaction.json")
                    .exists()
            );
            let count = runner.calls.lock().unwrap().len();
            assert_eq!(count, if stable { 3 } else { 1 });
            let closed = development::apply(&r.0, &q, "", &runner);
            assert_eq!(closed.code, 0, "{}", closed.stderr);
            let receipt = closed.receipt.unwrap();
            assert_eq!(receipt["checks"]["executed"], 0);
            assert_eq!(receipt["checks"]["reused"], count);
            assert_eq!(runner.calls.lock().unwrap().len(), count);
            assert_eq!(
                receipt["package_changes"][0]["version_after"],
                if stable { "1.0.0" } else { "0.3.1" }
            );
            assert_eq!(
                development::apply(&r.0, &Request::default(), "", &runner)
                    .receipt
                    .unwrap()["status"],
                "nothing-to-close"
            );
            println!(
                "staged stable={stable} validate+close_ms={} executed={count} duplicate_checks=0",
                started.elapsed().as_millis()
            );
        }
    }

    #[test]
    fn staged_source_or_tool_drift_requires_new_validation() {
        for tool in [false, true] {
            let r = staged_fixture(false);
            let runner = Checks::new();
            let q = request(&r.0, "patch", false, &runner);
            assert_eq!(validate(&r.0, &q, &runner).code, 0);
            write(
                &r.0,
                if tool {
                    "pkg/check-tool.exe"
                } else {
                    "pkg/src/lib.rs"
                },
                b"changed input",
            );
            let stale = development::apply(&r.0, &q, "", &runner);
            assert_ne!(stale.code, 0);
            assert!(!r.0.join(development::RECORD).exists());
            let fresh = request(&r.0, "patch", false, &runner);
            let closed = development::apply(&r.0, &fresh, "", &runner);
            assert_eq!(closed.code, 0, "{}", closed.stderr);
            assert_eq!(closed.receipt.unwrap()["checks"]["executed"], 1);
            assert_eq!(runner.calls.lock().unwrap().len(), 2);
        }
    }

    #[test]
    fn staged_failed_refresh_does_not_reuse_previous_success() {
        let r = staged_fixture(false);
        let runner = Checks::new();
        let q = request(&r.0, "patch", false, &runner);
        assert_eq!(validate(&r.0, &q, &runner).code, 0);
        let failing = Checks {
            fail: Some("normal".into()),
            action: None,
            calls: Mutex::new(vec![]),
        };
        let failed = validate(&r.0, &q, &failing);
        assert_ne!(failed.code, 0);
        assert!(!r.0.join(development::RECORD).exists());
        let closed = development::apply(&r.0, &q, "", &runner);
        assert_eq!(closed.code, 0, "{}", closed.stderr);
        assert_eq!(closed.receipt.unwrap()["checks"]["executed"], 1);
    }

    #[test]
    fn staged_always_run_checks_keep_live_state_boundary() {
        let r = staged_fixture(false);
        let mut cfg: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
        cfg["development"]["a"]["checks"][0]["always_run"] = json!(true);
        write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
        let runner = Checks::new();
        let q = request(&r.0, "patch", false, &runner);
        assert_eq!(validate(&r.0, &q, &runner).code, 0);
        let closed = development::apply(&r.0, &q, "", &runner);
        assert_eq!(closed.code, 0, "{}", closed.stderr);
        assert_eq!(closed.receipt.unwrap()["checks"]["executed"], 1);
        assert_eq!(runner.calls.lock().unwrap().len(), 2);
    }

    #[test]
    fn staged_missing_artifact_or_tampered_log_is_rechecked() {
        for artifact in [false, true] {
            let r = staged_fixture(false);
            let mut cfg: Value =
                serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
            if artifact {
                cfg["development"]["a"]["checks"][0]["artifacts"] = json!([".runtime/result.bin"]);
                write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
            }
            let runner = Checks {
                fail: None,
                action: if artifact {
                    Some("artifact-drift".into())
                } else {
                    None
                },
                calls: Mutex::new(vec![]),
            };
            let q = request(&r.0, "patch", false, &runner);
            let validated = validate(&r.0, &q, &runner);
            assert_eq!(validated.code, 0, "{}", validated.stderr);
            if artifact {
                fs::remove_file(r.0.join(".runtime/result.bin")).unwrap();
            } else {
                let path = fs::read_dir(r.0.join(".runtime/bridgeforge-codex/develop-validation"))
                    .unwrap()
                    .next()
                    .unwrap()
                    .unwrap()
                    .path();
                let mut record: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
                record["stdout"] = json!([1, 2, 3]);
                fs::write(path, serde_json::to_vec(&record).unwrap()).unwrap();
            }
            let closed = development::apply(&r.0, &q, "", &runner);
            assert_eq!(closed.code, 0, "{}", closed.stderr);
            assert_eq!(closed.receipt.unwrap()["checks"]["executed"], 1);
        }
    }

    #[test]
    fn staged_preview_is_readonly_and_missing_analysis_does_not_validate() {
        let r = staged_fixture(false);
        let runner = Checks::new();
        let before = fs::read(r.0.join("pkg/Cargo.toml")).unwrap();
        assert_eq!(
            development::preview(&r.0, &Request::default(), &runner).code,
            0
        );
        assert!(
            !r.0.join(".runtime/bridgeforge-codex/develop-validation")
                .exists()
        );
        assert_ne!(validate(&r.0, &Request::default(), &runner).code, 0);
        assert!(runner.calls.lock().unwrap().is_empty());
        assert_eq!(fs::read(r.0.join("pkg/Cargo.toml")).unwrap(), before);
    }

    #[test]
    fn staged_unknown_environment_does_not_certify_external_dependencies() {
        let r = staged_fixture(false);
        let mut cfg: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
        cfg["development"]["a"]
            .as_object_mut()
            .unwrap()
            .remove("environment");
        write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
        let runner = Checks::new();
        let q = request(&r.0, "patch", false, &runner);
        let result = validate(&r.0, &q, &runner);
        assert_eq!(result.code, 0, "{}", result.stderr);
        assert!(
            !r.0.join(".runtime/bridgeforge-codex/develop-validation")
                .exists()
        );
        let closed = development::apply(&r.0, &q, "", &runner);
        assert_eq!(closed.code, 0, "{}", closed.stderr);
        assert_eq!(closed.receipt.unwrap()["checks"]["executed"], 1);
    }

    #[test]
    fn staged_reuse_keeps_final_git_identity_guard() {
        struct ChangeRefs<'a>(&'a Checks);
        impl ProcessRunner for ChangeRefs<'_> {
            fn run(&self, command: &ProcessRequest) -> std::io::Result<crate::ProcessOutput> {
                if Path::new(&command.program)
                    .file_name()
                    .is_some_and(|n| n == "check-tool.exe")
                    && command.args.first().is_some_and(|s| s == "live")
                {
                    git(
                        &command.cwd,
                        &["update-ref", "refs/heads/check-side-effect", "HEAD"],
                    );
                }
                self.0.run(command)
            }
        }
        let r = staged_fixture(false);
        let mut cfg: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
        cfg["development"]["a"]["checks"]
            .as_array_mut()
            .unwrap()
            .push(json!({
                "id":"live","program":"./pkg/check-tool.exe","args":["live"],"always_run":true
            }));
        write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
        let runner = Checks::new();
        let q = request(&r.0, "patch", false, &runner);
        assert_eq!(validate(&r.0, &q, &runner).code, 0);
        let closed = development::apply(&r.0, &q, "", &ChangeRefs(&runner));
        assert_ne!(closed.code, 0);
        assert!(
            closed.stderr.contains("Git identity changed"),
            "{}",
            closed.stderr
        );
        assert!(!r.0.join(development::RECORD).exists());
        assert_eq!(
            runner
                .calls
                .lock()
                .unwrap()
                .iter()
                .filter(|s| *s == "normal")
                .count(),
            1
        );
    }

    #[test]
    fn staged_independent_package_fix_preserves_other_package_proof() {
        let r = task_development_fixture(false);
        write(&r.0, "pkg/check-tool.exe", b"fixture tool identity v1");
        let mut cfg: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
        for id in ["a", "b"] {
            cfg["development"][id]["checks"][0]["program"] = json!("./pkg/check-tool.exe");
            cfg["development"][id]["checks"][0]["args"] = json!([id]);
            cfg["development"][id]["environment"] = json!({"tools":["./pkg/check-tool.exe","cargo","rustc"],"variables":[],"inputs":{}});
        }
        write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
        write(&r.0, "pkg/src/lib.rs", b"pub fn value()->u8{3}\n");
        write(&r.0, "second/src/lib.rs", b"pub fn b()->u8{3}\n");
        let runner = Checks::new();
        let q = request(&r.0, "patch", false, &runner);
        let result = validate(&r.0, &q, &runner);
        assert_eq!(result.code, 0, "{}", result.stderr);
        write(&r.0, "pkg/src/lib.rs", b"pub fn value()->u8{4}\n");
        let q = request(&r.0, "patch", false, &runner);
        let closed = development::apply(&r.0, &q, "", &runner);
        assert_eq!(closed.code, 0, "{}", closed.stderr);
        let receipt = closed.receipt.unwrap();
        assert_eq!(receipt["checks"]["executed"], 1);
        assert_eq!(receipt["checks"]["reused"], 1);
        assert_eq!(runner.calls.lock().unwrap().as_slice(), &["a", "b", "a"]);
    }

    #[test]
    fn staged_stability_fallback_and_interrupted_validation_recover() {
        let r = staged_fixture(true);
        let runner = Checks {
            fail: Some("core".into()),
            action: None,
            calls: Mutex::new(vec![]),
        };
        let q = request(&r.0, "patch", false, &runner);
        let checked = validate(&r.0, &q, &runner);
        assert_eq!(checked.code, 0, "{}", checked.stderr);
        assert_eq!(
            checked.receipt.unwrap()["versions"]["pkg/Cargo.toml"],
            "0.3.1"
        );
        assert!(!r.0.join(development::RECORD).exists());
        let closed = development::apply(&r.0, &q, "", &runner);
        assert_eq!(closed.code, 0, "{}", closed.stderr);
        assert_eq!(
            closed.receipt.unwrap()["package_changes"][0]["version_after"],
            "0.3.1"
        );

        let r = staged_fixture(false);
        let runner = Checks {
            fail: None,
            action: Some("crash".into()),
            calls: Mutex::new(vec![]),
        };
        let q = request(&r.0, "patch", false, &runner);
        let before = fs::read(r.0.join("pkg/Cargo.toml")).unwrap();
        assert!(std::panic::catch_unwind(|| validate(&r.0, &q, &runner)).is_err());
        assert!(
            r.0.join(".runtime/bridgeforge-codex/develop-transaction.json")
                .exists()
        );
        let recovered = development::recover(&r.0);
        assert_eq!(recovered.code, 0, "{}", recovered.stderr);
        assert_eq!(fs::read(r.0.join("pkg/Cargo.toml")).unwrap(), before);
        assert!(!r.0.join(development::RECORD).exists());
    }
}
use super::*;

#[test]
fn lightweight_task_selection_skips_unneeded_checks_but_keeps_full_stability() {
    for subset in [false, true] {
        let repo = zero_fixture();
        configure(&repo.0, true);
        let mut cfg: Value =
            serde_json::from_slice(&fs::read(repo.0.join(CONFIG)).unwrap()).unwrap();
        cfg["development"]["a"]["checks"]
            .as_array_mut()
            .unwrap()
            .push(json!({"id":"unrelated","program":"fixture-check","args":["unrelated"]}));
        write(&repo.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
        write(&repo.0, "pkg/src/lib.rs", b"pub fn value()->u8{2}\n");
        let runner = Checks::new();
        let q = request(&repo.0, "patch", false, &runner);
        let path = q.analysis_file.as_ref().unwrap();
        let mut selected: Value =
            serde_json::from_slice(&fs::read(repo.0.join(path)).unwrap()).unwrap();
        if subset {
            selected["validation"]["a"]
                .as_array_mut()
                .unwrap()
                .truncate(1);
        }
        write(
            &repo.0,
            path.to_str().unwrap(),
            &serde_json::to_vec(&selected).unwrap(),
        );
        let started = std::time::Instant::now();
        let result = close(&repo.0, &q, &runner);
        let elapsed_ms = started.elapsed().as_millis();
        assert_eq!(result.code, 0, "{}", result.stderr);
        let expected = if subset {
            vec!["normal", "core", "integration"]
        } else {
            vec!["normal", "unrelated", "core", "integration"]
        };
        assert_eq!(runner.calls.lock().unwrap().as_slice(), expected.as_slice());
        println!(
            "task-selected checks: subset={subset}, elapsed_ms={elapsed_ms}, executed={}",
            expected.len()
        );
        assert_eq!(
            result.receipt.unwrap()["package_changes"][0]["version_after"],
            "1.0.0"
        );
    }
}

#[test]
fn lightweight_task_selection_requires_nonempty_effective_coverage_without_writes() {
    let repo = zero_fixture();
    configure(&repo.0, false);
    write(&repo.0, "pkg/src/lib.rs", b"pub fn value()->u8{2}\n");
    let runner = Checks::new();
    let q = request(&repo.0, "patch", false, &runner);
    let path = q.analysis_file.as_ref().unwrap();
    let before = fs::read(repo.0.join("pkg/Cargo.toml")).unwrap();
    let mut selected: Value =
        serde_json::from_slice(&fs::read(repo.0.join(path)).unwrap()).unwrap();
    selected["validation"] = json!({"a":[]});
    write(
        &repo.0,
        path.to_str().unwrap(),
        &serde_json::to_vec(&selected).unwrap(),
    );
    let result = close(&repo.0, &q, &runner);
    assert_ne!(result.code, 0);
    assert!(runner.calls.lock().unwrap().is_empty());
    assert_eq!(fs::read(repo.0.join("pkg/Cargo.toml")).unwrap(), before);
    assert!(!repo.0.join(development::RECORD).exists());
}

#[test]
fn lightweight_version_history_ignores_host_and_check_policy_changes() {
    struct Host<'a>(&'a Checks);
    impl ProcessRunner for Host<'_> {
        fn run(&self, request: &ProcessRequest) -> std::io::Result<crate::ProcessOutput> {
            let mut output = self.0.run(request)?;
            if request.program == "rustc" && request.args.iter().any(|arg| arg == "-vV") {
                output
                    .stdout
                    .extend_from_slice(b"\nfixture host identity changed\n");
            }
            Ok(output)
        }
    }
    let repo = zero_fixture();
    configure(&repo.0, false);
    write(&repo.0, "pkg/src/lib.rs", b"pub fn value()->u8{2}\n");
    let runner = Checks::new();
    let analysis = request(&repo.0, "patch", false, &runner);
    let result = close(&repo.0, &analysis, &runner);
    assert_eq!(result.code, 0, "{}", result.stderr);
    let before = fs::read(repo.0.join(development::RECORD)).unwrap();
    let ledger: Value = serde_json::from_slice(&before).unwrap();
    let entry = &ledger["entries"][0];
    assert_eq!(entry["evidence_schema"], 6);
    assert_eq!(entry["evidence_components"].as_object().unwrap().len(), 3);
    assert!(entry["evidence_components"].get("environment").is_none());
    let mut config: Value =
        serde_json::from_slice(&fs::read(repo.0.join(CONFIG)).unwrap()).unwrap();
    config["development"]["a"]["checks"][0]["timeout_seconds"] = json!(600);
    write(&repo.0, CONFIG, &serde_json::to_vec(&config).unwrap());
    let changed_host = Host(&runner);
    let current = development::preview(&repo.0, &Request::default(), &changed_host);
    assert_eq!(current.code, 0, "{}", current.stderr);
    assert_eq!(current.receipt.unwrap()["status"], "nothing-to-close");
    assert_eq!(fs::read(repo.0.join(development::RECORD)).unwrap(), before);
    write(&repo.0, "pkg/src/lib.rs", b"pub fn value()->u8{3}\n");
    let pending = development::preview(&repo.0, &Request::default(), &changed_host);
    assert_eq!(pending.code, 0, "{}", pending.stderr);
    assert_eq!(pending.receipt.unwrap()["affected_packages"], json!(["a"]));
    let next = request(&repo.0, "patch", false, &changed_host);
    let result = close(&repo.0, &next, &changed_host);
    assert_eq!(result.code, 0, "{}", result.stderr);
    let after: Value =
        serde_json::from_slice(&fs::read(repo.0.join(development::RECORD)).unwrap()).unwrap();
    assert_eq!(after["entries"][0], ledger["entries"][0]);
    assert_eq!(
        after["entries"][1]["previous"],
        ledger["entries"][0]["hash"]
    );
    let current = development::preview(&repo.0, &Request::default(), &changed_host);
    assert_eq!(current.receipt.unwrap()["status"], "nothing-to-close");
}

#[test]
fn lightweight_task_inputs_and_stability_contracts_remain_guarded() {
    struct Drift<'a> {
        inner: &'a Checks,
        path: &'a str,
    }
    impl ProcessRunner for Drift<'_> {
        fn run(&self, request: &ProcessRequest) -> std::io::Result<crate::ProcessOutput> {
            if request.program == "fixture-check"
                && request.args.first().is_some_and(|arg| arg == "normal")
            {
                fs::write(request.cwd.join(self.path), b"external check input edit\n")?;
            }
            self.inner.run(request)
        }
    }
    for stable in [false, true] {
        let repo = zero_fixture();
        configure(&repo.0, stable);
        let guarded = if stable {
            "README.md"
        } else {
            ".runtime/check-input.txt"
        };
        write(&repo.0, guarded, b"current task input\n");
        if !stable {
            let mut cfg: Value =
                serde_json::from_slice(&fs::read(repo.0.join(CONFIG)).unwrap()).unwrap();
            cfg["development"]["a"]["checks"][0]["inputs"] = json!([guarded]);
            write(&repo.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
        }
        write(&repo.0, "pkg/src/lib.rs", b"pub fn value()->u8{2}\n");
        let before = fs::read(repo.0.join("pkg/Cargo.toml")).unwrap();
        let runner = Checks::new();
        let drift = Drift {
            inner: &runner,
            path: guarded,
        };
        let analysis = request(&repo.0, "patch", false, &drift);
        let result = close(&repo.0, &analysis, &drift);
        assert_ne!(result.code, 0, "input drift was accepted");
        assert!(result.stderr.contains("changed"), "{}", result.stderr);
        assert!(!repo.0.join(development::RECORD).exists());
        assert_eq!(fs::read(repo.0.join("pkg/Cargo.toml")).unwrap(), before);
        assert_eq!(
            fs::read(repo.0.join(guarded)).unwrap(),
            b"external check input edit\n"
        );
    }
}
