//! Cross-checkout evidence, scoped release gates and comparable release work.
use super::*;

struct PortableChecks;

struct ResumeChecks {
    first_calls: std::sync::atomic::AtomicUsize,
    fail_second: std::sync::atomic::AtomicBool,
    fail_first: std::sync::atomic::AtomicBool,
}
impl ResumeChecks {
    fn new() -> Self {
        Self {
            first_calls: std::sync::atomic::AtomicUsize::new(0),
            fail_second: std::sync::atomic::AtomicBool::new(true),
            fail_first: std::sync::atomic::AtomicBool::new(false),
        }
    }
    fn calls(&self) -> usize {
        self.first_calls.load(std::sync::atomic::Ordering::SeqCst)
    }
    fn allow_completion(&self) {
        self.fail_second
            .store(false, std::sync::atomic::Ordering::SeqCst);
    }
}
impl ProcessRunner for ResumeChecks {
    fn run(&self, request: &ProcessRequest) -> std::io::Result<crate::ProcessOutput> {
        if request
            .args
            .first()
            .is_some_and(|arg| arg == "fixture-second-check")
        {
            return Ok(crate::ProcessOutput {
                code: if self.fail_second.load(std::sync::atomic::Ordering::SeqCst) {
                    1
                } else {
                    0
                },
                stdout: b"second".to_vec(),
                stderr: vec![],
                timed_out: false,
            });
        }
        if request
            .args
            .first()
            .is_some_and(|arg| arg == "fixture-portable-check")
        {
            self.first_calls
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
        let mut result = PortableChecks.run(request)?;
        if request
            .args
            .first()
            .is_some_and(|arg| arg == "fixture-portable-check")
            && self.fail_first.load(std::sync::atomic::Ordering::SeqCst)
        {
            result.code = 1;
        }
        Ok(result)
    }
}

fn resumable_fixture() -> Repo {
    let r = zero_fixture();
    configure_portable(&r.0, false);
    let mut cfg: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    cfg["development"]["a"]["checks"][0]["reusable"] = json!(true);
    cfg["development"]["a"]["checks"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"second","program":"cargo","args":["fixture-second-check"]}));
    write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
    write(&r.0, "pkg/src/lib.rs", b"pub fn value()->u8{2}\n");
    r
}

#[test]
fn task_development_cache_reuses_success_after_failed_transaction() {
    let r = resumable_fixture();
    let runner = ResumeChecks::new();
    let q = request(&r.0, "patch", false, &runner);
    assert!(
        !r.0.join(".runtime/bridgeforge-codex/develop-check-cache")
            .exists(),
        "preview wrote a cache"
    );
    let failed = close(&r.0, &q, &runner);
    assert_ne!(failed.code, 0, "{}", failed.stderr);
    assert_eq!(runner.calls(), 1);
    assert!(!r.0.join(development::RECORD).exists());
    runner.allow_completion();
    let q = request(&r.0, "patch", false, &runner);
    let done = close(&r.0, &q, &runner);
    assert_eq!(done.code, 0, "{}", done.stderr);
    assert_eq!(
        runner.calls(),
        1,
        "successful first check was executed again"
    );
    assert_eq!(done.receipt.unwrap()["checks"]["reused"], 1);
    let ledger = development::Ledger::load(&r.0).unwrap();
    assert_eq!(ledger.latest("a").unwrap().checks[0]["cache_reused"], true);
}

#[test]
fn task_development_cache_rejects_changed_source_or_missing_artifact() {
    for source_changed in [false, true] {
        let r = resumable_fixture();
        let runner = ResumeChecks::new();
        let q = request(&r.0, "patch", false, &runner);
        assert_ne!(close(&r.0, &q, &runner).code, 0);
        if source_changed {
            write(&r.0, "pkg/src/lib.rs", b"pub fn value()->u8{3}\n");
        } else {
            fs::remove_file(r.0.join(".runtime/result.bin")).unwrap();
        }
        runner.allow_completion();
        let q = request(&r.0, "patch", false, &runner);
        let done = close(&r.0, &q, &runner);
        assert_eq!(done.code, 0, "{}", done.stderr);
        assert_eq!(runner.calls(), 2);
    }
}

#[test]
fn task_development_cache_rejects_tampered_output() {
    let r = resumable_fixture();
    let runner = ResumeChecks::new();
    let q = request(&r.0, "patch", false, &runner);
    assert_ne!(close(&r.0, &q, &runner).code, 0);
    let cache = fs::read_dir(r.0.join(".runtime/bridgeforge-codex/develop-check-cache"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let mut data: Value = serde_json::from_slice(&fs::read(&cache).unwrap()).unwrap();
    data["stdout"] = json!([1, 2, 3]);
    fs::write(cache, serde_json::to_vec(&data).unwrap()).unwrap();
    runner.allow_completion();
    let q = request(&r.0, "patch", false, &runner);
    let done = close(&r.0, &q, &runner);
    assert_eq!(done.code, 0, "{}", done.stderr);
    assert_eq!(runner.calls(), 2);
}

#[test]
fn task_development_cache_requires_explicit_environment_contract() {
    let r = zero_fixture();
    configure(&r.0, false);
    let mut cfg: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    cfg["development"]["a"]["checks"][0]["reusable"] = json!(true);
    write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
    let result = development::preview(&r.0, &Request::default(), &Checks::new());
    assert_ne!(result.code, 0);
    assert!(
        result.stderr.contains("explicit environment"),
        "{}",
        result.stderr
    );
}

#[test]
fn task_development_cache_never_saves_failed_check() {
    let r = resumable_fixture();
    let runner = ResumeChecks::new();
    runner
        .fail_first
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let q = request(&r.0, "patch", false, &runner);
    assert_ne!(close(&r.0, &q, &runner).code, 0);
    assert_eq!(runner.calls(), 1);
    assert!(
        !r.0.join(".runtime/bridgeforge-codex/develop-check-cache")
            .exists()
    );
    assert!(!r.0.join(development::RECORD).exists());
}

#[test]
fn task_development_cache_rejects_tracked_transaction_output_before_writes() {
    let r = resumable_fixture();
    let runner = ResumeChecks::new();
    let mut cfg: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    cfg["development"]["a"]["checks"][0]["artifacts"] = json!(["README.md"]);
    cfg["build_inputs"] = json!(["README.md"]);
    write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
    let manifest = fs::read(r.0.join("pkg/Cargo.toml")).unwrap();
    let q = request(&r.0, "patch", false, &runner);
    let result = close(&r.0, &q, &runner);
    assert_ne!(result.code, 0);
    assert!(
        result.stderr.contains("require ignored artifacts"),
        "{}",
        result.stderr
    );
    assert_eq!(fs::read(r.0.join("pkg/Cargo.toml")).unwrap(), manifest);
    assert_eq!(runner.calls(), 0);
    assert!(!r.0.join(development::RECORD).exists());
}

#[test]
fn task_development_cache_rejects_changed_git_identity() {
    let r = resumable_fixture();
    let runner = ResumeChecks::new();
    let q = request(&r.0, "patch", false, &runner);
    assert_ne!(close(&r.0, &q, &runner).code, 0);
    git(
        &r.0,
        &["config", "user.name", "Different cache validation identity"],
    );
    runner.allow_completion();
    let q = request(&r.0, "patch", false, &runner);
    let done = close(&r.0, &q, &runner);
    assert_eq!(done.code, 0, "{}", done.stderr);
    assert_eq!(runner.calls(), 2);
}

#[test]
#[ignore = "child process used by the cache host-environment regression"]
fn cache_host_environment_probe() {
    let root =
        PathBuf::from(std::env::var_os("BRIDGEFORGE_CACHE_PROBE_ROOT").expect("cache probe root"));
    let runner = ResumeChecks::new();
    let resume = root.join(".runtime/allow-second").exists();
    if resume {
        runner.allow_completion();
    }
    let q = request(&root, "patch", false, &runner);
    let result = close(&root, &q, &runner);
    assert_eq!(result.code == 0, resume, "{}", result.stderr);
    assert_eq!(
        runner.calls(),
        1,
        "changed host environment reused a previous check"
    );
}

#[test]
fn task_development_cache_rejects_changed_host_environment() {
    let r = resumable_fixture();
    for resume in [false, true] {
        if resume {
            write(&r.0, ".runtime/allow-second", b"yes");
        }
        let mut command = ProcessRequest::new(std::env::current_exe().unwrap(), &r.0);
        command.args = vec![
            "--exact".into(),
            format!(
                "{}::cache_host_environment_probe",
                module_path!().split_once("::").unwrap().1
            )
            .into(),
            "--ignored".into(),
            "--nocapture".into(),
        ];
        command.env.insert(
            "BRIDGEFORGE_CACHE_PROBE_ROOT".into(),
            r.0.as_os_str().into(),
        );
        command.env.insert(
            "CARGO_TARGET_DIR".into(),
            r.0.join(if resume {
                ".runtime/host-b"
            } else {
                ".runtime/host-a"
            })
            .as_os_str()
            .into(),
        );
        command.timeout = std::time::Duration::from_secs(240);
        let output = SystemProcessRunner.run(&command).unwrap();
        assert_eq!(
            output.code,
            0,
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(!output.timed_out);
    }
}
impl ProcessRunner for PortableChecks {
    fn run(&self, request: &ProcessRequest) -> std::io::Result<crate::ProcessOutput> {
        if request
            .args
            .first()
            .is_some_and(|a| a == "fixture-portable-check")
        {
            assert!(Path::new(&request.program).is_absolute());
            assert_eq!(
                request
                    .env
                    .get(std::ffi::OsStr::new("LC_ALL"))
                    .map(|v| v.as_os_str()),
                Some(std::ffi::OsStr::new("C"))
            );
            if std::env::var_os("BRIDGEFORGE_SDK_ROOT").is_some() {
                assert!(
                    !request
                        .env_remove
                        .iter()
                        .any(|key| key == "BRIDGEFORGE_SDK_ROOT")
                );
            }
            fs::create_dir_all(request.cwd.join(".runtime"))?;
            fs::write(request.cwd.join(".runtime/result.bin"), b"checked artifact")?;
            return Ok(crate::ProcessOutput {
                code: 0,
                stdout: b"verified".to_vec(),
                stderr: vec![],
                timed_out: false,
            });
        }
        SystemProcessRunner.run(request)
    }
}

fn configure_portable(root: &Path, delivery: bool) {
    configure(root, false);
    let mut cfg: Value = serde_json::from_slice(&fs::read(root.join(CONFIG)).unwrap()).unwrap();
    cfg["development"]["a"]["checks"] = json!([{"id":"portable","program":"cargo","args":["fixture-portable-check"],"artifacts":[".runtime/result.bin"]}]);
    cfg["development"]["a"]["environment"] =
        json!({"tools":["cargo","rustc"],"variables":[],"inputs":{}});
    if delivery {
        cfg["development"]["a"]["release_artifacts"] = json!([".runtime/result.bin"]);
    }
    write(root, CONFIG, &serde_json::to_vec(&cfg).unwrap());
}

fn publish_fixture(root: &Path, runner: &dyn ProcessRunner) {
    // Build analysis against the same runner as the actual gate.
    let p = preview(root, "fix: fixture release", &Request::default(), runner);
    assert_eq!(p.code, 0, "{}", p.stderr);
    let v = p.receipt.unwrap();
    write(
        root,
        ".runtime/portable-release.json",
        &serde_json::to_vec(&json!({"schema":1,"fingerprint":v["fingerprint"],
        "product":{"level":"patch","reason":"fixture release","compatibility":"compatible"}}))
        .unwrap(),
    );
    let q = Request {
        analysis_file: Some(".runtime/portable-release.json".into()),
    };
    for (path, bytes) in build(root, "fix: fixture release", &q, runner)
        .unwrap()
        .unwrap()
        .writes
    {
        fs::write(path, bytes).unwrap();
    }
    git(root, &["add", "."]);
    git(root, &["commit", "-m", "fix: formal release"]);
}

#[test]
fn portable_machine_probe() {
    let Some(root) = std::env::var_os("BRIDGEFORGE_MACHINE_PROBE") else {
        return;
    };
    let root = PathBuf::from(root);
    let p = development::preview(&root, &Request::default(), &PortableChecks);
    let release = preview(
        &root,
        "fix: portable release",
        &Request::default(),
        &PortableChecks,
    );
    println!(
        "MACHINE_RESULT={}",
        json!({"develop":p.receipt,"develop_error":p.stderr,"release":release.receipt,"release_code":release.code,"release_error":release.stderr})
    );
}

#[test]
fn portable_sdk_mapping_probe() {
    let Some(root) = std::env::var_os("BRIDGEFORGE_SDK_PROBE") else {
        return;
    };
    let root = PathBuf::from(root);
    let mut cfg: Value = serde_json::from_slice(&fs::read(root.join(CONFIG)).unwrap()).unwrap();
    cfg["development"]["a"]["environment"]["inputs"] = json!({"sdk":"${BRIDGEFORGE_SDK_ROOT}"});
    write(&root, CONFIG, &serde_json::to_vec(&cfg).unwrap());
    let q = request(&root, "patch", false, &PortableChecks);
    assert_eq!(close(&root, &q, &PortableChecks).code, 0);
    let cfg = load(&root).unwrap().unwrap();
    let snapshot = capture(&root, &cfg, &PortableChecks).unwrap();
    let contract: portable_environment::Contract = serde_json::from_value(
        json!({"tools":["cargo","rustc"],"inputs":{"sdk":"${BRIDGEFORGE_SDK_ROOT}"}}),
    )
    .unwrap();
    let facts = contract.snapshot(&root, &snapshot).unwrap();
    assert!(!facts.keys().any(|k| k == "variable:BRIDGEFORGE_SDK_ROOT"));
}

#[test]
fn portable_sdk_path_variables_are_passed_and_certified_by_content() {
    let r = zero_fixture();
    configure_portable(&r.0, false);
    change(&r.0, true);
    write(&r.0, ".runtime/sdk/library.bin", b"sdk identity");
    let mut command = ProcessRequest::new(std::env::current_exe().unwrap(), &r.0);
    command.args = vec![
        "--exact".into(),
        format!(
            "{}::portable_sdk_mapping_probe",
            module_path!().split_once("::").unwrap().1
        )
        .into(),
        "--nocapture".into(),
    ];
    command.timeout = std::time::Duration::from_secs(240);
    command
        .env
        .insert("BRIDGEFORGE_SDK_PROBE".into(), r.0.as_os_str().into());
    command.env.insert(
        "BRIDGEFORGE_SDK_ROOT".into(),
        r.0.join(".runtime/sdk").into_os_string(),
    );
    let output = SystemProcessRunner.run(&command).unwrap();
    assert_eq!(
        output.code,
        0,
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!output.timed_out);
}

#[test]
fn portable_sdk_change_invalidates_required_release_checks_without_reopening_version() {
    let r = zero_fixture();
    configure_portable(&r.0, false);
    change(&r.0, true);
    let mut value: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    value["development"]["a"]["environment"]["inputs"] = json!({"sdk":".runtime/sdk"});
    value["required_checks"] = json!(["sdk-check"]);
    write(&r.0, CONFIG, &serde_json::to_vec(&value).unwrap());
    write(&r.0, ".runtime/sdk/library.bin", b"first sdk");
    let q = request(&r.0, "patch", false, &PortableChecks);
    assert_eq!(close(&r.0, &q, &PortableChecks).code, 0);
    let cfg = load(&r.0).unwrap().unwrap();
    let snapshot = || {
        let mut captured = capture(&r.0, &cfg, &PortableChecks).unwrap();
        development::release_gate(&r.0, &cfg, &mut captured, &PortableChecks).unwrap();
        captured
    };
    let make_analysis = |captured: &Capture| {
        let proof = json!({"id":"sdk-check","status":"passed","fingerprint":captured.validation_fingerprint,
            "scope":["a","product"],"reason":"fixture required SDK check"});
        let bytes = serde_json::to_vec(&proof).unwrap();
        write(&r.0, ".runtime/sdk-check.json", &bytes);
        let mut check = proof;
        check["artifact"] = json!(".runtime/sdk-check.json");
        check["sha256"] = json!(digest(&bytes));
        serde_json::from_value::<analysis::Analysis>(
            json!({"schema":1,"fingerprint":captured.fingerprint,"checks":[check]}),
        )
        .unwrap()
    };
    let scope = BTreeSet::from(["a".into(), "product".into()]);
    let mut before = snapshot();
    let old = make_analysis(&before);
    analysis::validate_checks(&r.0, &cfg, &mut before, &old, &scope).unwrap();
    write(&r.0, ".runtime/sdk/library.bin", b"different sdk");
    let mut after = snapshot();
    assert_ne!(before.validation_fingerprint, after.validation_fingerprint);
    assert!(
        analysis::validate_checks(&r.0, &cfg, &mut after, &old, &scope)
            .unwrap_err()
            .contains("validation evidence is stale")
    );
    let refreshed = make_analysis(&after);
    analysis::validate_checks(&r.0, &cfg, &mut after, &refreshed, &scope).unwrap();
    assert_eq!(
        development::preview(&r.0, &Request::default(), &PortableChecks)
            .receipt
            .unwrap()["status"],
        "nothing-to-close"
    );
}

#[test]
fn portable_external_input_guard_reaches_release_apply() {
    let r = zero_fixture();
    configure_portable(&r.0, false);
    change(&r.0, true);
    let mut cfg: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    cfg["development"]["a"]["environment"]["inputs"] = json!({"sdk":".runtime/sdk"});
    write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
    write(&r.0, ".runtime/sdk/library.bin", b"first sdk");
    let q = request(&r.0, "patch", false, &PortableChecks);
    let result = close(&r.0, &q, &PortableChecks);
    assert_eq!(result.code, 0, "{}", result.stderr);
    let p = preview(&r.0, "fix: sdk", &Request::default(), &PortableChecks)
        .receipt
        .unwrap();
    write(&r.0, ".runtime/release.json", &serde_json::to_vec(&json!({"schema":1,"fingerprint":p["fingerprint"],"product":{"level":"patch","reason":"sdk fixture","compatibility":"compatible"}})).unwrap());
    let q = Request {
        analysis_file: Some(".runtime/release.json".into()),
    };
    let plan = build(&r.0, "fix: sdk", &q, &PortableChecks)
        .unwrap()
        .unwrap();
    let before = fs::read(r.0.join("VERSION")).unwrap();
    write(&r.0, ".runtime/sdk/library.bin", b"different sdk");
    assert!(
        crate::release::apply_file_release_plan(&plan)
            .unwrap_err()
            .contains("portable validation environment changed")
    );
    assert_eq!(fs::read(r.0.join("VERSION")).unwrap(), before);
    write(&r.0, ".runtime/sdk/library.bin", b"first sdk");
    crate::release::apply_file_release_plan(&plan).unwrap();
    assert_ne!(fs::read(r.0.join("VERSION")).unwrap(), before);
}

#[test]
fn scoped_shared_lock_only_invalidates_actual_dependency_closure() {
    let r = scoped_rust_build_fixture();
    let runner = Checks::new();
    let q = release_request(&r.0, "minor");
    for (path, bytes) in build(&r.0, "feat: fixture", &q, &runner)
        .unwrap()
        .unwrap()
        .writes
    {
        fs::write(path, bytes).unwrap();
    }
    git(&r.0, &["add", "."]);
    git(&r.0, &["commit", "-m", "feat: formal release"]);
    change_locked_version(&r.0, "unrelated", "1.0.1");
    let alternate = AlternateClosureHost(&runner);
    let p = preview(&r.0, "fix: unrelated lock", &Request::default(), &alternate);
    assert_eq!(p.code, 0, "{}", p.stderr);
    change_locked_version(&r.0, "remote", "1.0.1");
    let p = preview(
        &r.0,
        "fix: actual dependency",
        &Request::default(),
        &alternate,
    );
    assert!(
        p.stderr
            .contains("library: package dependency/build coverage"),
        "{}",
        p.stderr
    );
}

#[test]
fn portable_declared_project_tools_and_configs_participate_in_release_scope() {
    let r = zero_fixture();
    configure_portable(&r.0, false);
    change(&r.0, true);
    write(&r.0, "scripts/wrapper.exe", b"verified tool");
    write(&r.0, "scripts/check.toml", b"[build]\njobs=2\n");
    let mut cfg: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    cfg["product_inputs"]
        .as_array_mut()
        .unwrap()
        .push(json!("scripts/"));
    cfg["development"]["a"]["environment"]["tools"]
        .as_array_mut()
        .unwrap()
        .push(json!("scripts/wrapper.exe"));
    cfg["development"]["a"]["environment"]["cargo_configs"] = json!(["scripts/check.toml"]);
    // These fixture files model actual build inputs as well as tools used by
    // validation; only the former creates durable version coverage impact.
    cfg["build_inputs"] = json!(["scripts/wrapper.exe", "scripts/check.toml"]);
    write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
    let q = request(&r.0, "patch", false, &PortableChecks);
    let result = close(&r.0, &q, &PortableChecks);
    assert_eq!(result.code, 0, "{}", result.stderr);
    publish_fixture(&r.0, &PortableChecks);
    for (path, bytes) in [
        ("scripts/wrapper.exe", b"changed tool".as_slice()),
        ("scripts/check.toml", b"[build]\njobs=3\n".as_slice()),
    ] {
        let original = fs::read(r.0.join(path)).unwrap();
        write(&r.0, path, bytes);
        let p = preview(
            &r.0,
            "fix: tool input",
            &Request::default(),
            &PortableChecks,
        );
        assert!(
            p.stderr.contains("package development closure required"),
            "{path}: {}",
            p.stderr
        );
        write(&r.0, path, &original);
    }
}

#[test]
fn portable_unmodeled_cargo_tool_override_is_rejected() {
    let r = zero_fixture();
    configure_portable(&r.0, false);
    let cfg = load(&r.0).unwrap().unwrap();
    let snapshot = capture(&r.0, &cfg, &PortableChecks).unwrap();
    let contract: portable_environment::Contract =
        serde_json::from_value(json!({"tools":["cargo","rustc"]})).unwrap();
    write(&r.0, ".runtime/custom-linker.exe", b"unverified compiler");
    write(
        &r.0,
        ".cargo/config.toml",
        b"[target.x86_64-pc-windows-msvc]\nlinker='.runtime/custom-linker.exe'\n",
    );
    assert!(
        contract
            .snapshot(&r.0, &snapshot)
            .unwrap_err()
            .contains("undeclared compiler/linker/wrapper")
    );
    for text in [
        "[build]\nrustflags=['-C','linker=.runtime/custom-linker.exe']\n",
        "[env]\nRUSTFLAGS='-C linker=.runtime/custom-linker.exe'\n",
    ] {
        write(&r.0, ".cargo/config.toml", text.as_bytes());
        assert!(
            contract
                .snapshot(&r.0, &snapshot)
                .unwrap_err()
                .contains("linker/sysroot flags")
        );
    }
    fs::remove_file(r.0.join(".cargo/config.toml")).unwrap();
    write(&r.0, "tools/custom-linker.exe", b"wrong root linker");
    write(
        &r.0,
        ".runtime/tools/custom-linker.exe",
        b"actual config linker",
    );
    write(
        &r.0,
        ".runtime/config/check.toml",
        b"[target.x86_64-pc-windows-msvc]\nlinker='tools/custom-linker.exe'\n",
    );
    let mut contract: portable_environment::Contract = serde_json::from_value(json!({"tools":["cargo","rustc","tools/custom-linker.exe"],"cargo_configs":[".runtime/config/check.toml"]})).unwrap();
    let error = contract.snapshot(&r.0, &snapshot).unwrap_err();
    assert!(
        error.contains("undeclared compiler/linker/wrapper"),
        "{error}"
    );
    contract
        .tools
        .push(".runtime/tools/custom-linker.exe".into());
    contract.snapshot(&r.0, &snapshot).unwrap();
    for text in [
        "paths=['outside']\n",
        "[source.local]\ndirectory='vendor'\n",
        "[patch.crates-io]\na={path='../outside'}\n",
        "[registry]\ncredential-provider='outside'\n",
        "[registries.test]\ncredential-provider='outside'\n",
        "[credential-alias]\ncustom=['outside']\n",
        "[env]\nSDK={value='../sdk',relative=true}\n",
    ] {
        write(&r.0, ".runtime/config/check.toml", text.as_bytes());
        assert!(
            contract
                .snapshot(&r.0, &snapshot)
                .unwrap_err()
                .contains("configuration section is not modeled")
        );
    }
    for text in [
        "build={rustc='tools/custom-linker.exe'}\n",
        "target={host={linker='tools/custom-linker.exe'}}\n",
    ] {
        write(&r.0, ".runtime/config/check.toml", text.as_bytes());
        assert!(
            contract
                .snapshot(&r.0, &snapshot)
                .unwrap_err()
                .contains("inline build/target")
        );
    }
    write(&r.0, ".runtime/config/check.toml", b"[source.crates-io]\nreplace-with='mirror'\n[source.mirror]\nregistry='sparse+https://example.invalid/index/'\n[registries.mirror]\nindex='sparse+https://example.invalid/index/'\n[net]\nretry=2\ngit-fetch-with-cli=true\n");
    contract.snapshot(&r.0, &snapshot).unwrap();
    contract
        .inputs
        .insert("sdk".into(), ".runtime/../sdk".into());
    write(&r.0, "sdk/lib.bin", b"deleted SDK input");
    let present = contract.snapshot(&r.0, &snapshot).unwrap();
    fs::remove_file(r.0.join("sdk/lib.bin")).unwrap();
    fs::remove_dir(r.0.join("sdk")).unwrap();
    let missing = contract.snapshot(&r.0, &snapshot).unwrap();
    assert_ne!(present["input:sdk"], missing["input:sdk"]);
    contract.inputs.clear();
    contract
        .inputs
        .insert("sdk".into(), r.0.join("sdk").to_string_lossy().into_owned());
    let absolute_missing = contract.snapshot(&r.0, &snapshot).unwrap();
    assert_eq!(absolute_missing["input:sdk"], missing["input:sdk"]);
}

#[test]
#[ignore = "explicit old/new CLI comparison; needs preserved baseline and candidate"]
fn package_successful_release_cli_benchmark() {
    let old = std::env::var_os("BRIDGEFORGE_BENCH_OLD").expect("baseline CLI");
    let new = std::env::var_os("BRIDGEFORGE_BENCH_NEW").expect("candidate CLI");
    assert_eq!(std::env::var("BRIDGEFORGE_PROFILE").unwrap(), "1");
    for scenario in ["single-package", "dependency"] {
        let r = if scenario == "dependency" {
            scoped_rust_build_fixture()
        } else {
            zero_fixture()
        };
        if scenario == "single-package" {
            configure(&r.0, false);
            change(&r.0, true);
            let q = request(&r.0, "patch", false, &Checks::new());
            assert_eq!(close(&r.0, &q, &Checks::new()).code, 0);
        }
        write(
            &r.0,
            ".runtime/benchmark-message.txt",
            b"fix: benchmark release",
        );
        for sample in 0..3 {
            for (label, binary) in [("before", &old), ("after", &new)] {
                let started = std::time::Instant::now();
                let mut request = ProcessRequest::new(binary, &r.0);
                request.args = [
                    "git-sync",
                    "--release-preview",
                    "--message-file",
                    ".runtime/benchmark-message.txt",
                ]
                .iter()
                .map(Into::into)
                .collect();
                request.timeout = std::time::Duration::from_secs(180);
                let output = SystemProcessRunner.run(&request).unwrap();
                assert_eq!(
                    output.code,
                    0,
                    "{label}: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
                let first: Value = serde_json::from_slice(&output.stdout).unwrap();
                assert_eq!(first["status"], "analysis-required", "{first}");
                write(&r.0, ".runtime/benchmark-analysis.json", &serde_json::to_vec(&json!({"schema":1,"fingerprint":first["fingerprint"],
                    "product":{"level":"minor","reason":"benchmark cumulative changes","compatibility":"compatible"}})).unwrap());
                request.args.extend(
                    [
                        "--release-analysis-file",
                        ".runtime/benchmark-analysis.json",
                    ]
                    .iter()
                    .map(Into::into),
                );
                let output = SystemProcessRunner.run(&request).unwrap();
                assert_eq!(
                    output.code,
                    0,
                    "{label}: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
                let final_plan: Value = serde_json::from_slice(&output.stdout).unwrap();
                assert_eq!(final_plan["status"], "release-planned", "{final_plan}");
                println!(
                    "RELEASE_SUCCESS_BENCH={}",
                    json!({"scenario":scenario,"sample":sample,"variant":label,"elapsed_ms":started.elapsed().as_millis(),"first_profile":first["profile"],"final_profile":final_plan["profile"]})
                );
            }
        }
    }
}

#[test]
#[ignore = "paired blocked preview comparison; requires old/new CLI paths"]
fn package_blocked_release_cli_benchmark() {
    let old = std::env::var_os("BRIDGEFORGE_BENCH_OLD").expect("baseline CLI");
    let new = std::env::var_os("BRIDGEFORGE_BENCH_NEW").expect("candidate CLI");
    assert_eq!(std::env::var("BRIDGEFORGE_PROFILE").unwrap(), "1");
    let r = zero_fixture();
    configure(&r.0, false);
    change(&r.0, true);
    write(
        &r.0,
        ".runtime/benchmark-message.txt",
        b"fix: blocked preview",
    );
    for sample in 0..3 {
        for (label, binary) in [("before", &old), ("after", &new)] {
            let mut request = ProcessRequest::new(binary, &r.0);
            request.args = [
                "git-sync",
                "--release-preview",
                "--message-file",
                ".runtime/benchmark-message.txt",
            ]
            .iter()
            .map(Into::into)
            .collect();
            request.timeout = std::time::Duration::from_secs(120);
            let start = std::time::Instant::now();
            let output = SystemProcessRunner.run(&request).unwrap();
            assert_eq!(
                output.code,
                2,
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(String::from_utf8_lossy(&output.stderr).contains("missing closure record"));
            let receipt: Value = serde_json::from_slice(&output.stdout).unwrap();
            println!(
                "RELEASE_BLOCKED_BENCH={}",
                json!({"sample":sample,"variant":label,"elapsed_ms":start.elapsed().as_millis(),"profile":receipt["profile"]})
            );
        }
    }
}

fn machine_probe(root: &Path, flags: Option<&str>) -> Value {
    let mut command = ProcessRequest::new(std::env::current_exe().unwrap(), root);
    command.args = vec![
        "--exact".into(),
        format!(
            "{}::portable_machine_probe",
            module_path!().split_once("::").unwrap().1
        )
        .into(),
        "--nocapture".into(),
    ];
    command.timeout = std::time::Duration::from_secs(180);
    command
        .env
        .insert("BRIDGEFORGE_MACHINE_PROBE".into(), root.as_os_str().into());
    for name in [
        "COMPUTERNAME",
        "USERNAME",
        "CODEX_THREAD_ID",
        "TERM",
        "NO_COLOR",
    ] {
        command.env.insert(name.into(), "different-host".into());
    }
    if let Some(flags) = flags {
        command.env.insert("RUSTFLAGS".into(), flags.into());
    }
    let output = SystemProcessRunner.run(&command).unwrap();
    assert_eq!(
        output.code,
        0,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!output.timed_out);
    let stdout = String::from_utf8(output.stdout).unwrap();
    serde_json::from_str(
        stdout
            .lines()
            .find_map(|s| s.strip_prefix("MACHINE_RESULT="))
            .unwrap(),
    )
    .unwrap()
}

#[test]
fn portable_machine_evidence_roundtrip_without_local_artifacts() {
    let a = zero_fixture();
    configure_portable(&a.0, false);
    change(&a.0, true);
    write(&a.0, "pkg/src/new.rs", b"pub fn new_source() {}\n");
    let q = request(&a.0, "patch", false, &PortableChecks);
    let result = close(&a.0, &q, &PortableChecks);
    assert_eq!(result.code, 0, "{}", result.stderr);
    let ledger = fs::read(a.0.join(development::RECORD)).unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&ledger).unwrap()["entries"][0]["evidence_schema"],
        6
    );
    git(&a.0, &["add", "."]);
    git(&a.0, &["commit", "-m", "fix: certified source"]);
    let b = Repo(a.0.with_file_name(format!(
        "{}-clone",
        a.0.file_name().unwrap().to_string_lossy()
    )));
    git(
        &a.0,
        &[
            "clone",
            "--no-hardlinks",
            a.0.to_str().unwrap(),
            b.0.to_str().unwrap(),
        ],
    );
    git(&b.0, &["config", "user.name", "Second Host"]);
    git(&b.0, &["config", "user.email", "host@example.invalid"]);
    assert!(!b.0.join(".runtime/result.bin").exists());
    let p = machine_probe(&b.0, None);
    assert_eq!(p["develop"]["status"], "nothing-to-close", "{p}");
    assert_eq!(p["release_code"], 0, "{p}");
    let changed = machine_probe(&b.0, Some("--cfg changed_build"));
    assert_eq!(
        changed["develop"]["status"], "nothing-to-close",
        "{changed}"
    );
    assert_eq!(changed["release_code"], 0, "{changed}");
    assert!(!changed.to_string().contains("--cfg changed_build"));
    write(&b.0, "pkg/src/lib.rs", b"pub fn value()->u8{3}\n");
    let q = request(&b.0, "patch", false, &PortableChecks);
    let result = close(&b.0, &q, &PortableChecks);
    assert_eq!(result.code, 0, "{}", result.stderr);
    git(&b.0, &["add", "."]);
    git(&b.0, &["commit", "-m", "fix: continued development"]);
    git(&a.0, &["fetch", b.0.to_str().unwrap()]);
    git(&a.0, &["merge", "--ff-only", "FETCH_HEAD"]);
    let p = development::preview(&a.0, &Request::default(), &PortableChecks);
    assert_eq!(p.receipt.unwrap()["status"], "nothing-to-close");
    let after = development::Ledger::load(&a.0).unwrap();
    assert_eq!(after.entries.len(), 2);
    assert_eq!(
        after.entries[0].hash,
        serde_json::from_slice::<Value>(&ledger).unwrap()["entries"][0]["hash"]
    );
    assert_eq!(after.entries[1].after.as_deref(), Some("0.3.2"));
}

#[test]
fn portable_declared_delivery_artifacts_still_gate_release() {
    let r = zero_fixture();
    configure_portable(&r.0, true);
    change(&r.0, true);
    let q = request(&r.0, "patch", false, &PortableChecks);
    assert_eq!(close(&r.0, &q, &PortableChecks).code, 0);
    fs::remove_file(r.0.join(".runtime/result.bin")).unwrap();
    let p = preview(&r.0, "fix: delivery", &Request::default(), &PortableChecks);
    assert!(p.stderr.contains("artifact missing"), "{}", p.stderr);
    let p = development::preview(&r.0, &Request::default(), &PortableChecks);
    assert_eq!(p.receipt.unwrap()["affected_packages"], json!(["a"]));
    let version = fs::read(r.0.join("pkg/Cargo.toml")).unwrap();
    let q = request(&r.0, "patch", false, &PortableChecks);
    let refreshed = close(&r.0, &q, &PortableChecks);
    assert_eq!(refreshed.code, 0, "{}", refreshed.stderr);
    assert_eq!(fs::read(r.0.join("pkg/Cargo.toml")).unwrap(), version);
    assert_eq!(
        preview(&r.0, "fix: delivery", &Request::default(), &PortableChecks).code,
        0
    );
}

#[test]
fn lightweight_new_and_replaced_delivery_declarations_require_coverage() {
    let repo = zero_fixture();
    configure_portable(&repo.0, false);
    change(&repo.0, true);
    let analysis = request(&repo.0, "patch", false, &PortableChecks);
    assert_eq!(close(&repo.0, &analysis, &PortableChecks).code, 0);
    let before = fs::read(repo.0.join(development::RECORD)).unwrap();
    for path in [
        ".runtime/new-delivery.bin",
        ".runtime/replaced-delivery.bin",
    ] {
        let mut cfg: Value =
            serde_json::from_slice(&fs::read(repo.0.join(CONFIG)).unwrap()).unwrap();
        cfg["development"]["a"]["checks"][0]["artifacts"] = json!([path]);
        cfg["development"]["a"]["release_artifacts"] = json!([path]);
        write(&repo.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
        let release = preview(
            &repo.0,
            "fix: delivery declaration",
            &Request::default(),
            &PortableChecks,
        );
        assert_ne!(release.code, 0);
        assert!(
            release.stderr.contains("no verified coverage"),
            "{}",
            release.stderr
        );
        let pending = development::preview(&repo.0, &Request::default(), &PortableChecks);
        assert_eq!(pending.receipt.unwrap()["affected_packages"], json!(["a"]));
        assert_eq!(fs::read(repo.0.join(development::RECORD)).unwrap(), before);
    }
}

#[test]
fn scoped_release_skeleton_skips_old_evidence_but_preserves_transaction_guard() {
    let r = cumulative_skeleton_fixture();
    configure(&r.0, false);
    change(&r.0, true);
    let runner = Checks::new();
    let q = request(&r.0, "patch", false, &runner);
    assert_eq!(close(&r.0, &q, &runner).code, 0);
    publish_fixture(&r.0, &runner);
    cumulative_skeleton(&r.0, 1, "project original");
    let alternate = AlternateClosureHost(&runner);
    crate::release_metrics::begin();
    let p = preview(&r.0, "chore: skeleton", &Request::default(), &alternate);
    let metrics = crate::release_metrics::finish();
    assert_eq!(p.code, 0, "{}", p.stderr);
    assert_eq!(p.receipt.unwrap()["status"], "nothing-to-release");
    assert!(metrics["counters"].get("evidence.fingerprints").is_none());
    assert!(metrics["counters"].get("process.rustc.other").is_none());
    write(
        &r.0,
        ".runtime/bridgeforge-codex/develop-transaction.json",
        b"{}",
    );
    let p = preview(&r.0, "chore: skeleton", &Request::default(), &alternate);
    assert!(
        p.stderr.contains("unfinished development transaction"),
        "{}",
        p.stderr
    );
}

#[test]
fn scoped_product_only_release_does_not_revalidate_unaffected_packages() {
    let r = zero_fixture();
    configure(&r.0, false);
    change(&r.0, true);
    let runner = Checks::new();
    let q = request(&r.0, "patch", false, &runner);
    assert_eq!(close(&r.0, &q, &runner).code, 0);
    publish_fixture(&r.0, &runner);
    write(&r.0, "README.md", b"new product documentation\n");
    let p = preview(
        &r.0,
        "docs: product",
        &Request::default(),
        &AlternateClosureHost(&runner),
    );
    assert_eq!(p.code, 0, "{}", p.stderr);
    change(&r.0, true);
    write(&r.0, "pkg/src/lib.rs", b"pub fn value()->u8{4}\n");
    git(&r.0, &["add", "."]);
    git(&r.0, &["commit", "-m", "fix: committed but not certified"]);
    let p = preview(&r.0, "fix: source", &Request::default(), &runner);
    assert!(
        p.stderr.contains("package development closure required"),
        "{}",
        p.stderr
    );
}

#[test]
fn scoped_product_only_plan_keeps_git_rule_guard_until_apply() {
    let r = zero_fixture();
    configure(&r.0, false);
    let mut config: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    config["build_inputs"] = json!([
        ".runtime/declared.txt",
        ".runtime/missing.txt",
        ".runtime/global/"
    ]);
    write(&r.0, CONFIG, &serde_json::to_vec(&config).unwrap());
    write(&r.0, ".runtime/declared.txt", b"fixed input\n");
    write(&r.0, ".runtime/global/seed.txt", b"seed input\n");
    change(&r.0, true);
    let runner = Checks::new();
    let q = request(&r.0, "patch", false, &runner);
    assert_eq!(close(&r.0, &q, &runner).code, 0);
    publish_fixture(&r.0, &runner);
    write(&r.0, "README.md", b"product documentation only\n");
    crate::release_metrics::begin();
    let p = preview(&r.0, "docs: product", &Request::default(), &runner);
    assert_eq!(p.code, 0, "{}", p.stderr);
    let metrics = crate::release_metrics::finish();
    assert!(metrics["counters"].get("evidence.fingerprints").is_none());
    write(
        &r.0,
        ".runtime/product-only.json",
        &serde_json::to_vec(
            &json!({"schema":1,"fingerprint":p.receipt.unwrap()["fingerprint"],
        "product":{"level":"patch","reason":"documentation only","compatibility":"compatible"}}),
        )
        .unwrap(),
    );
    let q = Request {
        analysis_file: Some(".runtime/product-only.json".into()),
    };
    let plan = build(&r.0, "docs: product", &q, &runner).unwrap().unwrap();
    let before = [
        fs::read(r.0.join("VERSION")).unwrap(),
        fs::read(r.0.join("CHANGELOG.md")).unwrap(),
    ];
    for variant in ["attributes", "config"] {
        if variant == "attributes" {
            write(&r.0, ".git/info/attributes", b"README.md -text\n");
        } else {
            git(&r.0, &["config", "core.eol", "crlf"]);
        }
        assert_eq!(
            independent::status(&r.0, &runner).unwrap(),
            *plan.release_status.as_ref().unwrap()
        );
        let error = crate::release::apply_file_release_plan(&plan).unwrap_err();
        assert!(
            error.contains("normalization rules changed"),
            "{variant}: {error}"
        );
        assert_eq!(fs::read(r.0.join("VERSION")).unwrap(), before[0]);
        assert_eq!(fs::read(r.0.join("CHANGELOG.md")).unwrap(), before[1]);
        if variant == "attributes" {
            fs::remove_file(r.0.join(".git/info/attributes")).unwrap();
        } else {
            git(&r.0, &["config", "--unset", "core.eol"]);
        }
    }
    for variant in [
        "value",
        "missing",
        "new-file",
        "removed-file",
        "empty-directory",
    ] {
        match variant {
            "value" => write(&r.0, ".runtime/declared.txt", b"changed input\n"),
            "missing" => write(&r.0, ".runtime/missing.txt", b"newly present\n"),
            "new-file" => write(&r.0, ".runtime/global/new.txt", b"new member\n"),
            "removed-file" => fs::remove_file(r.0.join(".runtime/global/seed.txt")).unwrap(),
            _ => fs::create_dir(r.0.join(".runtime/global/empty")).unwrap(),
        }
        assert_eq!(
            independent::status(&r.0, &runner).unwrap(),
            *plan.release_status.as_ref().unwrap()
        );
        assert!(
            crate::release::apply_file_release_plan(&plan).is_err(),
            "{variant} must block"
        );
        assert_eq!(fs::read(r.0.join("VERSION")).unwrap(), before[0]);
        assert_eq!(fs::read(r.0.join("CHANGELOG.md")).unwrap(), before[1]);
        match variant {
            "value" => write(&r.0, ".runtime/declared.txt", b"fixed input\n"),
            "missing" => fs::remove_file(r.0.join(".runtime/missing.txt")).unwrap(),
            "new-file" => fs::remove_file(r.0.join(".runtime/global/new.txt")).unwrap(),
            "removed-file" => write(&r.0, ".runtime/global/seed.txt", b"seed input\n"),
            _ => fs::remove_dir(r.0.join(".runtime/global/empty")).unwrap(),
        }
    }
    crate::release::apply_file_release_plan(&plan).unwrap();
}

#[test]
#[ignore = "explicit before/after benchmark, excludes fixture preparation"]
fn package_release_scope_benchmark() {
    for scenario in ["skeleton", "single-package", "dependency"] {
        let r = if scenario == "skeleton" {
            cumulative_skeleton_fixture()
        } else {
            zero_fixture()
        };
        if scenario == "skeleton" {
            cumulative_skeleton(&r.0, 1, "project original");
        } else {
            configure(&r.0, false);
            change(&r.0, true);
            if scenario == "dependency" {
                write(&r.0, "pkg/Cargo.toml", b"[package]\nname='a'\nversion='0.3.0'\nedition='2021'\n[workspace]\n[dependencies]\nhelper={path='../helper'}\n");
                write(
                    &r.0,
                    "helper/Cargo.toml",
                    b"[package]\nname='helper'\nversion='0.1.0'\nedition='2021'\n[workspace]\n",
                );
                write(&r.0, "helper/src/lib.rs", b"pub fn helper(){}\n");
                write(
                    &r.0,
                    "helper/Cargo.lock",
                    b"version=4\n[[package]]\nname='helper'\nversion='0.1.0'\n",
                );
                write(&r.0, "pkg/Cargo.lock", b"version=4\n[[package]]\nname='a'\nversion='0.3.0'\ndependencies=['helper']\n[[package]]\nname='helper'\nversion='0.1.0'\n");
                let mut cfg: Value =
                    serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
                cfg["packages"].as_array_mut().unwrap().push(json!({"id":"helper","language":"rust","manifest":"helper/Cargo.toml","name":"helper"}));
                cfg["adapters"]["rust"]["workspaces"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!("helper/Cargo.toml"));
                cfg["development"]["helper"] = policy(false);
                write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
            }
        }
        for sample in 0..3 {
            crate::release_metrics::begin();
            let start = std::time::Instant::now();
            let result = preview(&r.0, "fix: benchmark", &Request::default(), &Checks::new());
            let elapsed = start.elapsed().as_millis();
            let metrics = crate::release_metrics::finish();
            assert!(
                result.code == 0
                    || result
                        .stderr
                        .contains("package development closure required"),
                "{}",
                result.stderr
            );
            println!(
                "RELEASE_SCOPE_BENCH={}",
                json!({"scenario":scenario,"sample":sample,"elapsed_ms":elapsed,"exit_code":result.code,"status":result.receipt.as_ref().map(|r|&r["status"]),"error":result.stderr,"metrics":metrics})
            );
        }
    }
}
