//! Current-task validation and durable version coverage are separate contracts.
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
