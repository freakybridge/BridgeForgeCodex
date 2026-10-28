//! Lifecycle tests use isolated Git repositories; no real downstream writes.
use super::*;
use std::sync::Mutex;

#[path = "core_package_eol.rs"]
mod eol_tests;

#[path = "core_package_lightweight.rs"]
mod lightweight_tests;

fn task_development_fixture(dependency: bool) -> Repo {
    let r = fixture("unconfigured", true);
    write(
        &r.0,
        "second/Cargo.toml",
        b"[package]\nname='b'\nversion='0.2.0'\nedition='2021'\n[workspace]\n",
    );
    write(
        &r.0,
        "second/Cargo.lock",
        b"version=4\n[[package]]\nname='b'\nversion='0.2.0'\n",
    );
    write(&r.0, "second/src/lib.rs", b"pub fn b(){}\n");
    if dependency {
        write(&r.0, "pkg/Cargo.toml", b"[package]\nname='a'\nversion='1.0.0'\nedition='2021'\n[dependencies]\nb={path='../second'}\n[workspace]\n");
        write(&r.0, "pkg/Cargo.lock", b"version=4\n[[package]]\nname='a'\nversion='1.0.0'\ndependencies=['b']\n[[package]]\nname='b'\nversion='0.2.0'\n");
    }
    let mut cfg: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    cfg["adapters"]["rust"]["workspaces"]
        .as_array_mut()
        .unwrap()
        .push(json!("second/Cargo.toml"));
    cfg["packages"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"b","name":"b","language":"rust","manifest":"second/Cargo.toml"}));
    cfg["product_inputs"]
        .as_array_mut()
        .unwrap()
        .push(json!("second/Cargo.lock"));
    write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
    git(&r.0, &["add", "."]);
    git(&r.0, &["commit", "--amend", "--no-edit"]);
    migrate(&r.0);
    let mut cfg: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    cfg["development"] = json!({"a":policy(false),"b":policy(false)});
    write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
    write(&r.0, "pkg/src/lib.rs", b"pub fn value()->u8{2}\n");
    write(&r.0, "second/src/lib.rs", b"pub fn b()->u8{2}\n");
    let runner = Checks::new();
    let q = request(&r.0, "patch", false, &runner);
    let closed = close(&r.0, &q, &runner);
    assert_eq!(closed.code, 0, "{}", closed.stderr);
    r
}

#[test]
fn task_development_ignores_unrelated_pending_and_preserves_repository_diagnostic() {
    let r = task_development_fixture(false);
    let runner = Checks::new();
    write(&r.0, "second/src/lib.rs", b"pub fn b()->u8{3}\n");
    let before = fs::read(r.0.join(development::RECORD)).unwrap();
    let task = development::preview_for_packages(&r.0, &Request::default(), "a", &runner, None);
    assert_eq!(task.code, 0, "{}", task.stderr);
    let task = task.receipt.unwrap();
    assert_eq!(task["status"], "task-development-current");
    assert_eq!(task["readiness_scope"], "task");
    assert_eq!(task["outside_task_evidence_checked"], false);
    let global = development::preview(&r.0, &Request::default(), &runner);
    assert_eq!(global.receipt.unwrap()["affected_packages"], json!(["b"]));
    assert_eq!(fs::read(r.0.join(development::RECORD)).unwrap(), before);
    assert!(runner.calls.lock().unwrap().is_empty());
}

#[test]
fn task_development_closes_only_selected_independent_package() {
    let r = task_development_fixture(false);
    let runner = Checks::new();
    write(&r.0, "pkg/src/lib.rs", b"pub fn value()->u8{3}\n");
    write(&r.0, "second/src/lib.rs", b"pub fn b()->u8{3}\n");
    let b_before = development::Ledger::load(&r.0)
        .unwrap()
        .latest("b")
        .unwrap()
        .hash
        .clone();
    let p = development::preview_for_packages(&r.0, &Request::default(), "a", &runner, None);
    assert_eq!(p.code, 0, "{}", p.stderr);
    let p = p.receipt.unwrap();
    assert_eq!(p["affected_packages"], json!(["a"]));
    write(&r.0, ".runtime/task.json", &serde_json::to_vec(&json!({"schema":1,"fingerprint":p["fingerprint"],"packages":{"a":{"level":"patch","reason":"selected task","compatibility":"compatible"}},"validation":{"a":policy(false)["checks"]}})).unwrap());
    let q = Request {
        analysis_file: Some(".runtime/task.json".into()),
    };
    let p = development::preview_for_packages(&r.0, &q, "a", &runner, None);
    assert_eq!(p.code, 0, "{}", p.stderr);
    let done = development::apply_for_packages(
        &r.0,
        &q,
        p.receipt.unwrap()["fingerprint"].as_str().unwrap(),
        "a",
        &runner,
        None,
    );
    assert_eq!(done.code, 0, "{}", done.stderr);
    assert_eq!(done.receipt.unwrap()["readiness_scope"], "task");
    assert_eq!(runner.calls.lock().unwrap().as_slice(), ["normal"]);
    assert_eq!(
        development::Ledger::load(&r.0)
            .unwrap()
            .latest("b")
            .unwrap()
            .hash,
        b_before
    );
    assert_eq!(
        development::preview_for_packages(&r.0, &Request::default(), "a", &runner, None)
            .receipt
            .unwrap()["status"],
        "task-development-current"
    );
    assert_eq!(
        development::preview(&r.0, &Request::default(), &runner)
            .receipt
            .unwrap()["affected_packages"],
        json!(["b"])
    );
}

#[test]
fn task_development_captures_dependencies_consumers_and_scope_identity() {
    let r = task_development_fixture(true);
    let runner = Checks::new();
    write(&r.0, "second/src/lib.rs", b"pub fn b()->u8{3}\n");
    let a = development::preview_for_packages(&r.0, &Request::default(), "a", &runner, None);
    let b = development::preview_for_packages(&r.0, &Request::default(), "b", &runner, None);
    assert_eq!(a.code, 0, "{}", a.stderr);
    assert_eq!(b.code, 0, "{}", b.stderr);
    let a = a.receipt.unwrap();
    let b = b.receipt.unwrap();
    assert_eq!(a["effective_task_packages"], json!(["a", "b"]));
    assert_eq!(b["effective_task_packages"], json!(["a", "b"]));
    assert_ne!(a["fingerprint"], b["fingerprint"]);
    assert!(
        a["affected_packages"]
            .as_array()
            .unwrap()
            .contains(&json!("b"))
    );
}

#[test]
fn task_development_rejects_unknown_duplicate_and_empty_selection() {
    let r = task_development_fixture(false);
    let runner = Checks::new();
    let before = fs::read(r.0.join(development::RECORD)).unwrap();
    for packages in ["", "a,a", "a,", "unknown"] {
        let p =
            development::preview_for_packages(&r.0, &Request::default(), packages, &runner, None);
        assert_ne!(p.code, 0, "accepted {packages}");
    }
    assert_eq!(fs::read(r.0.join(development::RECORD)).unwrap(), before);
    assert!(runner.calls.lock().unwrap().is_empty());
}

#[test]
fn task_development_dependency_identity_transition_includes_endpoints_and_consumers() {
    let r = fixture("independent", true);
    let mut cfg = load(&r.0).unwrap().unwrap();
    let mut c = capture(&r.0, &cfg, &SystemProcessRunner).unwrap();
    let template = c.current.packages["a"].clone();
    for id in ["b", "d", "e", "sibling"] {
        let mut p = template.clone();
        p.name = id.into();
        p.manifest = format!("{id}/Cargo.toml");
        c.current.packages.insert(id.into(), p);
    }
    for (owner, target) in [("a", "b"), ("e", "d"), ("sibling", "b")] {
        c.current.dependencies.push(NativeDependency {
            owner: owner.into(),
            target: target.into(),
            kind: "normal".into(),
        });
    }
    let requested = BTreeSet::from(["a".into()]);
    assert_eq!(
        development::task_scope(&cfg, &c, &requested).unwrap(),
        BTreeSet::from(["a".into(), "b".into()])
    );
    let b = json!({"id":"b","language":"rust","manifest":"b/Cargo.toml","name":"b"});
    let d = json!({"id":"d","language":"rust","manifest":"d/Cargo.toml","name":"d"});
    cfg.native.lifecycle.push(serde_json::from_value(json!({"id":"split-b","base":c.boundary,"operation":"split","compatibility":"compatible","from":[b.clone()],"to":[b,d]})).unwrap());
    assert_eq!(
        development::task_scope(&cfg, &c, &requested).unwrap(),
        BTreeSet::from([
            "a".into(),
            "b".into(),
            "d".into(),
            "e".into(),
            "sibling".into()
        ])
    );
}

#[path = "core_package_portability.rs"]
mod portability_tests;

// Reconstruct the published 6.0.0 serializer independently of the production
// compatibility branch. This fixture has one standalone Rust package, no
// dependencies, no extra check inputs and no stability contracts.
fn use_legacy_closure(root: &Path, runner: &dyn ProcessRunner) {
    let cfg = load(root).unwrap().unwrap();
    let current = capture(root, &cfg, runner).unwrap();
    let mut ledger = development::Ledger::load(root).unwrap();
    let entry = ledger.entries.last_mut().unwrap();
    let evidence = json!({
        "content":entry.content,
        "dependencies":{"a":entry.content},
        "build":{"pkg/Cargo.lock":digest(&fs::read(root.join("pkg/Cargo.lock")).unwrap())},
        "environment":current.environment,
        "policy":cfg.development.get("a"),
        "contracts":{},"check_inputs":{},
        "adapters":format!("{:?}",cfg.native.adapters)
    });
    entry.evidence = digest(&serde_json::to_vec(&evidence).unwrap());
    entry.evidence_schema = None;
    entry.evidence_components = None;
    entry.hash.clear();
    entry.hash = digest(&serde_json::to_vec(entry).unwrap());
    write(
        root,
        development::RECORD,
        &serde_json::to_vec(&ledger).unwrap(),
    );
}

// Independently reconstruct the 7.0.x host-bound serializer for these
// single-registered-package fixtures; do not derive it from new components.
fn use_host_closure(root: &Path, runner: &dyn ProcessRunner, unmarked: bool) {
    let cfg = load(root).unwrap().unwrap();
    let current = capture(root, &cfg, runner).unwrap();
    assert!(current.current.dependencies.is_empty());
    let mut ledger = development::Ledger::load(root).unwrap();
    let entry = ledger.entries.last_mut().unwrap();
    let package = &current.current.packages[&entry.id];
    let mut workspace = fs::read_to_string(root.join(&package.workspace))
        .unwrap()
        .parse::<toml_edit::DocumentMut>()
        .unwrap();
    if !cfg.native.product_packages.is_empty() {
        workspace["workspace"]["package"]
            .as_table_like_mut()
            .unwrap()
            .remove("version");
    }
    let lock_path = Path::new(&package.workspace)
        .with_file_name("Cargo.lock")
        .to_string_lossy()
        .replace('\\', "/");
    let lock = crate::source_projection::lock_dependency_closure(
        &fs::read(root.join(&lock_path)).unwrap(),
        &BTreeSet::from([package.name.clone()]),
    )
    .unwrap();
    let config: Value = serde_json::from_slice(&fs::read(root.join(CONFIG)).unwrap()).unwrap();
    let build_inputs = cfg
        .build_inputs
        .iter()
        .filter(|path| {
            current.inventory.contains(path) || current.current.inputs.contains_key(*path)
        })
        .map(|path| {
            assert!(!path.ends_with('/'), "fixture uses explicit build files");
            (path.clone(), digest(&fs::read(root.join(path)).unwrap()))
        })
        .collect::<BTreeMap<_, _>>();
    let mut check_inputs = BTreeMap::new();
    for check in config["development"][&entry.id]["checks"]
        .as_array()
        .unwrap()
    {
        for path in check["inputs"].as_array().into_iter().flatten() {
            let path = path.as_str().unwrap();
            check_inputs.insert(path, digest(&fs::read(root.join(path)).unwrap()));
        }
    }
    entry.evidence = digest(&serde_json::to_vec(&json!({
        "content":entry.content,"dependencies":{entry.id.clone():entry.content},
        "build":build_inputs,"rust_build":BTreeMap::from([
            (package.workspace.clone(),digest(workspace.to_string().as_bytes())),
            (lock_path,digest(&lock))
        ]),"environment":current.environment,"policy":cfg.development.get(&entry.id),
        "contracts":{},"check_inputs":check_inputs,"adapters":format!("{:?}",cfg.native.adapters)
    })).unwrap());
    entry.evidence_schema = if unmarked { None } else { Some(2) };
    entry.evidence_components = None;
    entry.hash.clear();
    entry.hash = digest(&serde_json::to_vec(entry).unwrap());
    write(
        root,
        development::RECORD,
        &serde_json::to_vec(&ledger).unwrap(),
    );
}

#[test]
fn package_legacy_aggregate_gains_version_coverage_without_rewrite_or_bump() {
    let r = zero_fixture();
    configure(&r.0, false);
    write(&r.0, "pkg/src/lib.rs", b"pub fn value()->u8{2}\n");
    let runner = Checks::new();
    let q = request(&r.0, "patch", false, &runner);
    let result = close(&r.0, &q, &runner);
    assert_eq!(result.code, 0, "{}", result.stderr);
    use_legacy_closure(&r.0, &runner);
    let before = fs::read(r.0.join(development::RECORD)).unwrap();
    git(&r.0, &["add", "."]);
    git(&r.0, &["commit", "-m", "legacy development closure"]);
    let calls = runner.calls.lock().unwrap().len();
    let p = development::preview(&r.0, &Request::default(), &runner);
    assert_eq!(p.code, 0, "{}", p.stderr);
    assert_eq!(p.receipt.unwrap()["affected_packages"], json!(["a"]));
    let refresh = request(&r.0, "patch", false, &runner);
    let refreshed = close(&r.0, &refresh, &runner);
    assert_eq!(refreshed.code, 0, "{}", refreshed.stderr);
    let change = &refreshed.receipt.unwrap()["package_changes"][0];
    assert_eq!(change["version_before"], "0.3.1");
    assert_eq!(change["version_after"], "0.3.1");
    let q = release_request(&r.0, "patch");
    let plan = build(&r.0, "fix: release after upgrade", &q, &runner)
        .unwrap()
        .unwrap();
    assert_eq!(plan.package_changes[0].version_after, "0.3.1");
    assert_eq!(runner.calls.lock().unwrap().len(), calls + 1);
    let old = development::Ledger::parse(Some(&before)).unwrap();
    development::Ledger::load(&r.0)
        .unwrap()
        .extends(&old)
        .unwrap();
    // A real package edit still blocks and explains why, even with a valid old record.
    write(&r.0, "pkg/src/lib.rs", b"pub fn value()->u8{3}\n");
    let error = preview(&r.0, "fix: unpublished edit", &Request::default(), &runner).stderr;
    assert!(error.contains("a: package content changed"), "{error}");
}

#[test]
fn package_legacy_closure_rejects_changed_validation_and_schema_downgrade() {
    let r = zero_fixture();
    configure(&r.0, false);
    write(&r.0, "pkg/src/lib.rs", b"pub fn value()->u8{2}\n");
    let runner = Checks::new();
    let q = request(&r.0, "patch", false, &runner);
    assert_eq!(close(&r.0, &q, &runner).code, 0);
    use_legacy_closure(&r.0, &runner);
    let before = fs::read(r.0.join(development::RECORD)).unwrap();
    let original_lock = fs::read(r.0.join("pkg/Cargo.lock")).unwrap();
    write(
        &r.0,
        "pkg/Cargo.lock",
        &[
            original_lock.as_slice(),
            b"\n# changed legacy build input\n",
        ]
        .concat(),
    );
    let error = preview(&r.0, "fix: changed evidence", &Request::default(), &runner).stderr;
    assert!(
        error.contains("a: package dependency/build coverage"),
        "{error}"
    );
    write(&r.0, "pkg/Cargo.lock", &original_lock);
    let mut ledger = development::Ledger::load(&r.0).unwrap();
    let entry = ledger.entries.last_mut().unwrap();
    entry.evidence_schema = Some(2);
    entry.hash.clear();
    entry.hash = digest(&serde_json::to_vec(entry).unwrap());
    write(
        &r.0,
        development::RECORD,
        &serde_json::to_vec(&ledger).unwrap(),
    );
    let p = development::preview(&r.0, &Request::default(), &runner);
    assert_eq!(p.receipt.unwrap()["affected_packages"], json!(["a"]));
    // Unknown schemas and a tampered hash chain fail closed.
    ledger.entries.last_mut().unwrap().evidence_schema = Some(99);
    assert!(
        development::Ledger::parse(Some(&serde_json::to_vec(&ledger).unwrap()))
            .unwrap_err()
            .contains("unsupported closure evidence schema")
    );
    let mut tampered: Value = serde_json::from_slice(&before).unwrap();
    tampered["entries"][0]["reason"] = json!("tampered");
    assert!(development::Ledger::parse(Some(&serde_json::to_vec(&tampered).unwrap())).is_err());
}

#[test]
fn package_legacy_closure_mixed_history_preserves_prefix_and_modern_unmarked_records() {
    let r = zero_fixture();
    configure(&r.0, false);
    write(&r.0, "pkg/src/lib.rs", b"pub fn value()->u8{2}\n");
    let runner = Checks::new();
    let q = request(&r.0, "patch", false, &runner);
    assert_eq!(close(&r.0, &q, &runner).code, 0);
    // 7.0.x used the current evidence algorithm but omitted its schema too.
    assert_eq!(
        development::Ledger::load(&r.0)
            .unwrap()
            .entries
            .last()
            .unwrap()
            .evidence_schema,
        Some(6)
    );
    use_host_closure(&r.0, &runner, true);
    let p = development::preview(&r.0, &Request::default(), &runner);
    assert_eq!(p.receipt.unwrap()["affected_packages"], json!(["a"]));
    use_legacy_closure(&r.0, &runner);
    let old = development::Ledger::load(&r.0).unwrap();
    git(&r.0, &["add", "."]);
    git(&r.0, &["commit", "-m", "published legacy closure"]);
    write(&r.0, "pkg/src/lib.rs", b"pub fn value()->u8{3}\n");
    let q = request(&r.0, "patch", false, &runner);
    let result = close(&r.0, &q, &runner);
    assert_eq!(result.code, 0, "{}", result.stderr);
    let new = development::Ledger::load(&r.0).unwrap();
    new.extends(&old).unwrap();
    assert_eq!(
        serde_json::to_value(&new.entries[..old.entries.len()]).unwrap(),
        serde_json::to_value(&old.entries).unwrap()
    );
    assert_eq!(new.entries.last().unwrap().evidence_schema, Some(6));
    assert_eq!(new.entries.last().unwrap().after.as_deref(), Some("0.3.2"));
    assert_eq!(
        development::preview(&r.0, &Request::default(), &runner)
            .receipt
            .unwrap()["status"],
        "nothing-to-close"
    );
}

struct AlternateClosureHost<'a>(&'a Checks);

#[test]
fn package_transaction_environment_retains_strict_host_and_correlation_guards() {
    let baseline = BTreeMap::from([
        ("CODEX_THREAD_ID".into(), "thread-a".into()),
        ("CODEX_SESSION_ID".into(), "session-a".into()),
        ("CODEX_APP_TOOLS_PIPE_PATH".into(), "pipe-a".into()),
        ("PATH".into(), "same-tool-directory".into()),
        ("RUSTFLAGS".into(), "-C opt-level=1".into()),
        ("tool:rustc".into(), "compiler-a".into()),
        ("python:environment".into(), "runtime-a".into()),
        ("PROJECT_CUSTOM_INPUT".into(), "private-test-value".into()),
    ]);
    let before = inputs::environment_fingerprints(&baseline).unwrap();
    for key in [
        "CODEX_THREAD_ID",
        "CODEX_SESSION_ID",
        "CODEX_APP_TOOLS_PIPE_PATH",
    ] {
        let mut changed = baseline.clone();
        changed.insert(key.into(), "other-identity".into());
        let after = inputs::environment_fingerprints(&changed).unwrap();
        assert_ne!(before.planning, after.planning, "{key}");
    }
    for key in [
        "PATH",
        "RUSTFLAGS",
        "tool:rustc",
        "python:environment",
        "PROJECT_CUSTOM_INPUT",
    ] {
        let mut changed = baseline.clone();
        changed.insert(key.into(), "changed-input".into());
        let after = inputs::environment_fingerprints(&changed).unwrap();
        assert_ne!(before.planning, after.planning, "{key}");
    }
}

// Invoked in isolated processes by the parent test. No process-global env
// mutation is used, so the ordinary parallel test suite remains deterministic.
#[test]
fn portable_closure_process_probe() {
    let Some(root) = std::env::var_os("BRIDGEFORGE_PORTABLE_CLOSURE_PROBE_ROOT") else {
        return;
    };
    let root = PathBuf::from(root);
    let runner = Checks::new();
    if !root.join(development::RECORD).exists() {
        let q = request(&root, "patch", false, &runner);
        let result = close(&root, &q, &runner);
        assert_eq!(result.code, 0, "{}", result.stderr);
        assert_eq!(result.receipt.unwrap()["status"], "development-closed");
    }
    let development = development::preview(&root, &Request::default(), &runner);
    let release = preview(&root, "fix: portable proof", &Request::default(), &runner);
    let snapshot = capture(&root, &load(&root).unwrap().unwrap(), &runner).unwrap();
    println!(
        "\nPORTABLE_PROBE_RESULT={}",
        json!({
            "development":development.receipt,"release":release.receipt,
            "release_code":release.code,"release_error":release.stderr,
            "planning_environment":snapshot.environment,
        })
    );
}

fn portable_probe(root: &Path, identity: &str, flags: Option<&str>) -> Value {
    let mut command = ProcessRequest::new(std::env::current_exe().unwrap(), root);
    command.args = vec![
        "--exact".into(),
        format!(
            "{}::portable_closure_process_probe",
            module_path!().split_once("::").unwrap().1
        )
        .into(),
        "--nocapture".into(),
        "--test-threads=1".into(),
    ];
    command.timeout = std::time::Duration::from_secs(120);
    command.env.insert(
        "BRIDGEFORGE_PORTABLE_CLOSURE_PROBE_ROOT".into(),
        root.as_os_str().to_owned(),
    );
    for key in [
        "CODEX_THREAD_ID",
        "CODEX_SESSION_ID",
        "CODEX_APP_TOOLS_PIPE_PATH",
    ] {
        command.env.insert(key.into(), identity.into());
    }
    if let Some(flags) = flags {
        command.env.insert("RUSTFLAGS".into(), flags.into());
    }
    let output = SystemProcessRunner.run(&command).unwrap();
    assert_eq!(
        output.code,
        0,
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!output.timed_out);
    let stdout = String::from_utf8(output.stdout).unwrap();
    serde_json::from_str(
        stdout
            .lines()
            .find_map(|line| line.strip_prefix("PORTABLE_PROBE_RESULT="))
            .expect("child executed the probe"),
    )
    .unwrap()
}

#[test]
fn package_version_coverage_survives_host_changes_without_rewriting_history() {
    let r = zero_fixture();
    configure(&r.0, false);
    change(&r.0, true);
    let first = portable_probe(&r.0, "development-conversation", None);
    assert_eq!(first["development"]["status"], "nothing-to-close");
    let ledger = fs::read(r.0.join(development::RECORD)).unwrap();
    let index = fs::read(r.0.join(".git/index")).unwrap();
    let manifest = fs::read(r.0.join("pkg/Cargo.toml")).unwrap();
    let second = portable_probe(&r.0, "release-conversation", None);
    assert_ne!(
        first["planning_environment"],
        second["planning_environment"]
    );
    assert_eq!(second["development"]["status"], "nothing-to-close");
    assert_eq!(second["release_code"], 0);
    assert_eq!(second["release"]["status"], "analysis-required");
    let third = portable_probe(
        &r.0,
        "release-conversation",
        Some("--cfg materially_different_build"),
    );
    assert_eq!(third["development"]["status"], "nothing-to-close");
    assert_eq!(third["release_code"], 0);
    assert!(
        !third["release_error"]
            .as_str()
            .unwrap()
            .contains("materially_different_build")
    );
    assert_eq!(fs::read(r.0.join(development::RECORD)).unwrap(), ledger);
    assert_eq!(fs::read(r.0.join(".git/index")).unwrap(), index);
    assert_eq!(fs::read(r.0.join("pkg/Cargo.toml")).unwrap(), manifest);
}

#[test]
fn package_portable_evidence_diagnoses_inputs_and_preserves_chain() {
    let r = zero_fixture();
    configure(&r.0, false);
    change(&r.0, true);
    let runner = Checks::new();
    let q = request(&r.0, "patch", false, &runner);
    assert_eq!(close(&r.0, &q, &runner).code, 0);
    let ledger = fs::read(r.0.join(development::RECORD)).unwrap();
    let parsed = development::Ledger::parse(Some(&ledger)).unwrap();
    let entry = parsed.entries.last().unwrap();
    assert_eq!(entry.evidence_schema, Some(6));
    assert_eq!(entry.evidence_components.as_ref().unwrap().len(), 3);
    let alternate = AlternateClosureHost(&runner);
    let result = preview(&r.0, "fix: env", &Request::default(), &alternate);
    assert_eq!(result.code, 0, "{}", result.stderr);
    let config = fs::read(r.0.join(CONFIG)).unwrap();
    let mut changed: Value = serde_json::from_slice(&config).unwrap();
    changed["development"]["a"]["checks"][0]["timeout_seconds"] = json!(99);
    write(&r.0, CONFIG, &serde_json::to_vec(&changed).unwrap());
    let result = preview(&r.0, "fix: policy", &Request::default(), &runner);
    assert_eq!(result.code, 0, "{}", result.stderr);
    write(&r.0, CONFIG, &config);
    // Even an internally rehashed record must carry matching component evidence.
    let mut invalid = parsed.clone();
    let entry = invalid.entries.last_mut().unwrap();
    entry
        .evidence_components
        .as_mut()
        .unwrap()
        .insert("environment".into(), "invented".into());
    entry.hash.clear();
    entry.hash = digest(&serde_json::to_vec(entry).unwrap());
    assert!(
        development::Ledger::parse(Some(&serde_json::to_vec(&invalid).unwrap()))
            .unwrap_err()
            .contains("components")
    );
    assert_eq!(fs::read(r.0.join(development::RECORD)).unwrap(), ledger);
}

#[test]
fn package_published_host_closure_scopes_unrelated_lock_but_keeps_real_inputs() {
    let r = scoped_rust_build_fixture();
    let runner = Checks::new();
    let q = release_request(&r.0, "minor");
    let plan = build(&r.0, "feat: publish", &q, &runner).unwrap().unwrap();
    use_host_closure(&r.0, &runner, false);
    for (path, bytes) in plan.writes {
        fs::write(path, bytes).unwrap();
    }
    git(&r.0, &["add", "."]);
    git(&r.0, &["commit", "-m", "published host-bound proof"]);
    let ledger = fs::read(r.0.join(development::RECORD)).unwrap();
    change_locked_version(&r.0, "unrelated", "1.0.1");
    let alternate = AlternateClosureHost(&runner);
    let p = development::preview(&r.0, &Request::default(), &alternate);
    assert_eq!(p.code, 0, "{}", p.stderr);
    assert_eq!(p.receipt.unwrap()["status"], "nothing-to-close");
    for native in ["Cargo.toml", "Cargo.lock"] {
        git(&r.0, &["update-index", "--chmod=+x", "--", native]);
        let changed_mode = development::preview(&r.0, &Request::default(), &alternate);
        assert_eq!(changed_mode.code, 0, "{}", changed_mode.stderr);
        assert_eq!(
            changed_mode.receipt.unwrap()["affected_packages"],
            json!(["library"])
        );
        git(&r.0, &["update-index", "--chmod=-x", "--", native]);
    }
    change_locked_version(&r.0, "remote", "1.0.1");
    let p = development::preview(&r.0, &Request::default(), &alternate);
    assert_eq!(p.receipt.unwrap()["affected_packages"], json!(["library"]));
    assert_eq!(fs::read(r.0.join(development::RECORD)).unwrap(), ledger);
}

#[test]
fn package_published_version_does_not_expire_with_another_host() {
    let r = zero_fixture();
    configure(&r.0, false);
    change(&r.0, true);
    let runner = Checks::new();
    let q = request(&r.0, "patch", false, &runner);
    assert_eq!(close(&r.0, &q, &runner).code, 0);
    let q = release_request(&r.0, "patch");
    for (path, bytes) in build(&r.0, "fix: publish", &q, &runner)
        .unwrap()
        .unwrap()
        .writes
    {
        fs::write(path, bytes).unwrap();
    }
    git(&r.0, &["add", "."]);
    git(&r.0, &["commit", "-m", "published portable proof"]);
    let alternate = AlternateClosureHost(&runner);
    let p = development::preview(&r.0, &Request::default(), &alternate);
    assert_eq!(p.receipt.unwrap()["status"], "nothing-to-close");
}

#[test]
fn package_release_external_analysis_is_preview_only_and_tracks_drift() {
    let r = zero_fixture();
    configure(&r.0, false);
    change(&r.0, true);
    let runner = Checks::new();
    let q = request(&r.0, "patch", false, &runner);
    assert_eq!(close(&r.0, &q, &runner).code, 0);
    let q = release_request(&r.0, "patch");
    struct External(PathBuf);
    impl Drop for External {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
        }
    }
    let external = External(r.0.with_extension("preview-analysis.json"));
    let payload = fs::read(r.0.join(q.analysis_file.unwrap())).unwrap();
    fs::write(&external.0, &payload).unwrap();
    let q = Request {
        analysis_file: Some(external.0.clone()),
    };
    let protected = [
        "VERSION",
        "CHANGELOG.md",
        "pkg/Cargo.toml",
        "pkg/Cargo.lock",
        development::RECORD,
        ".git/index",
    ]
    .into_iter()
    .map(|p| (p, fs::read(r.0.join(p)).unwrap()))
    .collect::<BTreeMap<_, _>>();
    let status = git(&r.0, &["status", "--porcelain=v1", "--untracked-files=all"]);
    let p = preview(&r.0, "fix: readonly external plan", &q, &runner);
    assert_eq!(p.code, 0, "{}", p.stderr);
    assert_eq!(p.receipt.unwrap()["status"], "release-planned");
    for (path, bytes) in &protected {
        assert_eq!(&fs::read(r.0.join(path)).unwrap(), bytes, "{path}");
    }
    assert_eq!(
        git(&r.0, &["status", "--porcelain=v1", "--untracked-files=all"]),
        status
    );
    assert!(
        build(&r.0, "fix: forbidden external execution", &q, &runner)
            .unwrap_err()
            .contains("must belong to this project")
    );

    let cfg = load(&r.0).unwrap().unwrap();
    let mut captured = capture(&r.0, &cfg, &runner).unwrap();
    development::release_gate(&r.0, &cfg, &mut captured, &runner).unwrap();
    analysis::read_preview(&r.0, &q, &mut captured).unwrap();
    fs::write(&external.0, b"changed after capture").unwrap();
    assert!(verify_release_inputs(&captured.inputs).is_err());
    let mut stale: Value = serde_json::from_slice(&payload).unwrap();
    stale["fingerprint"] = json!("stale");
    fs::write(&external.0, serde_json::to_vec(&stale).unwrap()).unwrap();
    assert!(
        analysis::read_preview(&r.0, &q, &mut captured)
            .unwrap_err()
            .contains("stale")
    );
    fs::write(&external.0, br#"{"schema":1,"schema":1}"#).unwrap();
    assert!(analysis::read_preview(&r.0, &q, &mut captured).is_err());
    fs::write(&external.0, &payload).unwrap();
    // Mutation entry points retain their original project-local file boundary.
    let original = fs::read(r.0.join("pkg/src/lib.rs")).unwrap();
    write(&r.0, "pkg/src/lib.rs", b"pub fn value()->u8{3}\n");
    let p = development::preview(&r.0, &q, &runner);
    assert_ne!(p.code, 0);
    assert!(
        p.stderr.contains("must belong to this project"),
        "{}",
        p.stderr
    );
    write(&r.0, "pkg/src/lib.rs", &original);
}

#[test]
fn package_declared_ignored_build_file_binds_presence_bytes_and_legacy_coverage() {
    let r = zero_fixture();
    configure(&r.0, false);
    let mut config: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    config["build_inputs"] = json!([".runtime/global.txt"]);
    write(&r.0, CONFIG, &serde_json::to_vec(&config).unwrap());
    change(&r.0, true);
    let runner = Checks::new();
    let q = request(&r.0, "patch", false, &runner);
    assert_eq!(close(&r.0, &q, &runner).code, 0);
    let version = fs::read(r.0.join("pkg/Cargo.toml")).unwrap();
    assert_eq!(
        development::preview(&r.0, &Request::default(), &runner)
            .receipt
            .unwrap()["status"],
        "nothing-to-close"
    );
    write(&r.0, ".runtime/global.txt", b"input-v1");
    let p = development::preview(&r.0, &Request::default(), &runner);
    assert!(
        p.receipt.unwrap()["pending_reasons"]["a"]
            .as_str()
            .unwrap()
            .contains("dependency/build coverage")
    );
    let q = request(&r.0, "patch", false, &runner);
    assert_eq!(close(&r.0, &q, &runner).code, 0);
    assert_eq!(fs::read(r.0.join("pkg/Cargo.toml")).unwrap(), version);
    for payload in [Some(b"input-v2".as_slice()), None] {
        match payload {
            Some(bytes) => write(&r.0, ".runtime/global.txt", bytes),
            None => fs::remove_file(r.0.join(".runtime/global.txt")).unwrap(),
        }
        let p = preview(
            &r.0,
            "fix: changed build input",
            &Request::default(),
            &runner,
        );
        assert!(
            p.stderr.contains("dependency/build coverage"),
            "{}",
            p.stderr
        );
    }
    write(&r.0, ".runtime/global.txt", b"input-v1");
    use_host_closure(&r.0, &runner, false);
    let ledger = fs::read(r.0.join(development::RECORD)).unwrap();
    let p = preview(
        &r.0,
        "fix: incomplete old proof",
        &Request::default(),
        &runner,
    );
    assert!(
        p.stderr.contains("dependency/build coverage"),
        "{}",
        p.stderr
    );
    assert_eq!(fs::read(r.0.join(development::RECORD)).unwrap(), ledger);
}

#[test]
fn package_declared_build_directory_tracks_ignored_and_empty_members() {
    let r = zero_fixture();
    configure(&r.0, false);
    let mut config: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    config["build_inputs"] = json!([".runtime/global/"]);
    write(&r.0, CONFIG, &serde_json::to_vec(&config).unwrap());
    change(&r.0, true);
    let runner = Checks::new();
    let q = request(&r.0, "patch", false, &runner);
    assert_eq!(close(&r.0, &q, &runner).code, 0);
    fs::create_dir_all(r.0.join(".runtime/global/empty")).unwrap();
    let p = development::preview(&r.0, &Request::default(), &runner);
    assert!(
        p.receipt.unwrap()["pending_reasons"]["a"]
            .as_str()
            .unwrap()
            .contains("dependency/build coverage")
    );
    write(&r.0, ".runtime/global/input.txt", b"global-v1");
    let q = request(&r.0, "patch", false, &runner);
    assert_eq!(close(&r.0, &q, &runner).code, 0);
    let ledger = fs::read(r.0.join(development::RECORD)).unwrap();
    for action in ["modify", "delete", "add", "remove-empty"] {
        match action {
            "modify" => write(&r.0, ".runtime/global/input.txt", b"global-v2"),
            "delete" => fs::remove_file(r.0.join(".runtime/global/input.txt")).unwrap(),
            "add" => write(&r.0, ".runtime/global/new.txt", b"new"),
            _ => fs::remove_dir(r.0.join(".runtime/global/empty")).unwrap(),
        }
        let p = preview(&r.0, "fix: changed directory", &Request::default(), &runner);
        assert_ne!(p.code, 0, "{action}");
        assert!(
            p.stderr.contains("dependency/build coverage"),
            "{}",
            p.stderr
        );
        write(&r.0, ".runtime/global/input.txt", b"global-v1");
        if action == "add" {
            fs::remove_file(r.0.join(".runtime/global/new.txt")).unwrap();
        }
        fs::create_dir_all(r.0.join(".runtime/global/empty")).unwrap();
    }
    assert_eq!(fs::read(r.0.join(development::RECORD)).unwrap(), ledger);
}

#[test]
fn package_declared_build_directory_rejects_drift_but_allows_declared_artifact() {
    for artifact in [false, true] {
        let r = zero_fixture();
        configure(&r.0, false);
        let mut config: Value =
            serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
        config["build_inputs"] = json!([".runtime/global/"]);
        if artifact {
            config["development"]["a"]["checks"][0]["artifacts"] =
                json!([".runtime/global/new.txt"]);
        }
        write(&r.0, CONFIG, &serde_json::to_vec(&config).unwrap());
        change(&r.0, true);
        let mut runner = Checks::new();
        runner.action = Some("new-build-file".into());
        let q = request(&r.0, "patch", false, &runner);
        let manifest = fs::read(r.0.join("pkg/Cargo.toml")).unwrap();
        let result = close(&r.0, &q, &runner);
        if artifact {
            assert_eq!(result.code, 0, "{}", result.stderr);
            assert_eq!(result.receipt.unwrap()["status"], "development-closed");
            assert_eq!(
                development::preview(&r.0, &Request::default(), &runner)
                    .receipt
                    .unwrap()["status"],
                "nothing-to-close"
            );
        } else {
            assert_ne!(result.code, 0);
            assert!(
                result.stderr.contains("directory membership changed"),
                "{}",
                result.stderr
            );
            assert!(!r.0.join(development::RECORD).exists());
            assert_eq!(fs::read(r.0.join("pkg/Cargo.toml")).unwrap(), manifest);
        }
    }
}

#[test]
fn package_declared_build_directory_blocks_release_apply_after_plan() {
    let r = zero_fixture();
    configure(&r.0, false);
    let mut config: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    config["build_inputs"] = json!([".runtime/global/"]);
    write(&r.0, CONFIG, &serde_json::to_vec(&config).unwrap());
    write(&r.0, ".runtime/global/input.txt", b"original");
    change(&r.0, true);
    let runner = Checks::new();
    let q = request(&r.0, "patch", false, &runner);
    let closed = close(&r.0, &q, &runner);
    assert_eq!(closed.code, 0, "{}", closed.stderr);
    let q = release_request(&r.0, "patch");
    let plan = build(&r.0, "fix: release", &q, &runner).unwrap().unwrap();
    let version = fs::read(r.0.join("VERSION")).unwrap();
    let changelog = fs::read(r.0.join("CHANGELOG.md")).unwrap();
    let index = fs::read(r.0.join(".git/index")).unwrap();
    for directory in [false, true] {
        let path = r.0.join(".runtime/global/extra");
        if directory {
            fs::create_dir(&path).unwrap();
        } else {
            fs::write(&path, b"ignored addition").unwrap();
        }
        let error = apply_file_release_plan(&plan).unwrap_err();
        assert!(error.contains("directory membership changed"), "{error}");
        assert_eq!(fs::read(r.0.join("VERSION")).unwrap(), version);
        assert_eq!(fs::read(r.0.join("CHANGELOG.md")).unwrap(), changelog);
        assert_eq!(fs::read(r.0.join(".git/index")).unwrap(), index);
        if directory {
            fs::remove_dir(path).unwrap();
        } else {
            fs::remove_file(path).unwrap();
        }
    }
    apply_file_release_plan(&plan).unwrap();
    assert_eq!(fs::read(r.0.join("VERSION")).unwrap(), b"1.0.1\n");
}
impl ProcessRunner for AlternateClosureHost<'_> {
    fn run(&self, command: &ProcessRequest) -> std::io::Result<crate::ProcessOutput> {
        let mut output = self.0.run(command)?;
        if command.program == "rustc" && command.args == vec![std::ffi::OsString::from("-vV")] {
            output
                .stdout
                .extend_from_slice(b"\nfixture alternate validation host\n");
        }
        Ok(output)
    }
}

#[test]
fn package_published_aggregate_reuses_eol_check_inputs_but_rejects_real_source_edits() {
    let r = zero_fixture();
    configure(&r.0, false);
    let mut cfg: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    cfg["product_inputs"]
        .as_array_mut()
        .unwrap()
        .extend([json!(".gitattributes"), json!("scripts/check.ps1")]);
    cfg["development"]["a"]["checks"][0]["inputs"] = json!(["scripts/check.ps1"]);
    write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
    write(&r.0, ".gitattributes", b"*.ps1 text eol=crlf\n");
    write(&r.0, "scripts/check.ps1", b"Write-Output 'validated'\n");
    write(&r.0, "pkg/src/lib.rs", b"pub fn value()->u8{2}\n");
    let runner = Checks::new();
    let q = request(&r.0, "patch", false, &runner);
    assert_eq!(close(&r.0, &q, &runner).code, 0);
    let q = release_request(&r.0, "patch");
    let plan = build(&r.0, "fix: publish", &q, &runner).unwrap().unwrap();
    // Model a publication by the old tool, not a new certification of an
    // unpublished aggregate record by the current tool.
    use_host_closure(&r.0, &runner, false);
    for (path, bytes) in plan.writes {
        fs::write(path, bytes).unwrap();
    }
    git(&r.0, &["add", "."]);
    git(
        &r.0,
        &["commit", "-m", "formal release with Windows check input"],
    );
    write(&r.0, "scripts/check.ps1", b"Write-Output 'validated'\r\n");
    assert_eq!(
        revision_payload(&r.0, "HEAD", "scripts/check.ps1", &runner)
            .unwrap()
            .unwrap(),
        b"Write-Output 'validated'\n"
    );
    assert!(
        git(
            &r.0,
            &["diff", "--name-only", "HEAD", "--", "scripts/check.ps1"]
        )
        .is_empty()
    );
    let ledger = fs::read(r.0.join(development::RECORD)).unwrap();
    let alternate = AlternateClosureHost(&runner);
    let p = development::preview(&r.0, &Request::default(), &alternate);
    assert_eq!(p.code, 0, "{}", p.stderr);
    assert_eq!(p.receipt.unwrap()["status"], "nothing-to-close");
    assert_eq!(fs::read(r.0.join(development::RECORD)).unwrap(), ledger);
    write(
        &r.0,
        "scripts/check.ps1",
        b"Write-Output 'actually changed'\r\n",
    );
    let p = development::preview(&r.0, &Request::default(), &alternate);
    assert_eq!(p.receipt.unwrap()["status"], "nothing-to-close");
    assert_eq!(fs::read(r.0.join(development::RECORD)).unwrap(), ledger);
    write(&r.0, "pkg/src/lib.rs", b"pub fn value()->u8{3}\n");
    let p = development::preview(&r.0, &Request::default(), &alternate);
    assert_eq!(p.receipt.unwrap()["affected_packages"], json!(["a"]));
}

#[test]
fn package_published_closure_reuses_history_but_rejects_unpublished_or_changed_inputs() {
    let r = zero_fixture();
    configure(&r.0, false);
    write(&r.0, "pkg/src/lib.rs", b"pub fn value()->u8{2}\n");
    let runner = Checks::new();
    let alternate = AlternateClosureHost(&runner);
    let q = request(&r.0, "patch", false, &runner);
    assert_eq!(close(&r.0, &q, &runner).code, 0);
    // Obtain only the native/product publication payload while modern proof is
    // available. Later fixture writes model an old tool's historical release;
    // the current tool must never rewrite the already committed legacy chain.
    let q = release_request(&r.0, "patch");
    let plan = build(&r.0, "fix: publish", &q, &runner).unwrap().unwrap();
    use_legacy_closure(&r.0, &runner);
    git(&r.0, &["add", "."]);
    git(&r.0, &["commit", "-m", "unpublished package closure"]);
    let p = development::preview(&r.0, &Request::default(), &alternate);
    assert_eq!(p.receipt.unwrap()["affected_packages"], json!(["a"]));
    for (path, bytes) in plan.writes {
        fs::write(path, bytes).unwrap();
    }
    git(&r.0, &["add", "."]);
    git(&r.0, &["commit", "-m", "formal product release"]);
    let ledger = fs::read(r.0.join(development::RECORD)).unwrap();
    let index = fs::read(r.0.join(".git/index")).unwrap();
    let p = development::preview(&r.0, &Request::default(), &alternate);
    assert_eq!(p.code, 0, "{}", p.stderr);
    let receipt = p.receipt.unwrap();
    assert_eq!(receipt["status"], "nothing-to-close");
    assert_eq!(receipt["reused_published_packages"], json!(["a"]));
    assert_eq!(fs::read(r.0.join(development::RECORD)).unwrap(), ledger);
    assert_eq!(fs::read(r.0.join(".git/index")).unwrap(), index);
    // A separate product-only change can release without recertifying this package.
    let readme = fs::read(r.0.join("README.md")).unwrap();
    write(&r.0, "README.md", b"unrelated product documentation\n");
    let p = preview(&r.0, "docs: product", &Request::default(), &alternate);
    assert_eq!(p.code, 0, "{}", p.stderr);
    assert_eq!(p.receipt.unwrap()["status"], "analysis-required");
    write(&r.0, "README.md", &readme);
    for (path, bytes) in [
        ("pkg/src/lib.rs", b"pub fn value()->u8{3}\n".as_slice()),
        (
            "pkg/Cargo.lock",
            b"version=4\n[[package]]\nname='a'\nversion='0.3.1'\n# changed\n".as_slice(),
        ),
    ] {
        let original = fs::read(r.0.join(path)).unwrap();
        write(&r.0, path, bytes);
        let p = development::preview(&r.0, &Request::default(), &alternate);
        assert_eq!(p.code, 0, "{}", p.stderr);
        assert_eq!(
            p.receipt.unwrap()["affected_packages"],
            json!(["a"]),
            "{path}"
        );
        write(&r.0, path, &original);
    }
}

#[test]
fn package_published_closure_checks_shared_workspace_inputs() {
    let r = scoped_rust_build_fixture();
    let runner = Checks::new();
    let q = release_request(&r.0, "minor");
    let plan = build(&r.0, "feat: publish", &q, &runner).unwrap().unwrap();
    // Model an already published historical record. The current version tool
    // must not newly certify an unpublished aggregate legacy proof.
    use_host_closure(&r.0, &runner, false);
    for (path, bytes) in plan.writes {
        fs::write(path, bytes).unwrap();
    }
    git(&r.0, &["add", "."]);
    git(&r.0, &["commit", "-m", "formal scoped product release"]);
    let alternate = AlternateClosureHost(&runner);
    let p = development::preview(&r.0, &Request::default(), &alternate);
    assert_eq!(p.code, 0, "{}", p.stderr);
    assert_eq!(p.receipt.unwrap()["status"], "nothing-to-close");
    // Every package sharing a changed global Cargo profile requires validation.
    let manifest = fs::read(r.0.join("Cargo.toml")).unwrap();
    write(&r.0, ".git/info/attributes", b"Cargo.toml text eol=lf\n");
    write(
        &r.0,
        "Cargo.toml",
        String::from_utf8(manifest.clone())
            .unwrap()
            .replace("\r\n", "\n")
            .replace('\n', "\r\n")
            .as_bytes(),
    );
    let equivalent = development::preview(&r.0, &Request::default(), &alternate);
    assert_eq!(equivalent.code, 0, "{}", equivalent.stderr);
    assert_eq!(equivalent.receipt.unwrap()["status"], "nothing-to-close");
    write(
        &r.0,
        "Cargo.toml",
        &[manifest.as_slice(), b"\n[profile.release]\nlto=true\n"].concat(),
    );
    let p = development::preview(&r.0, &Request::default(), &alternate);
    assert_eq!(p.code, 0, "{}", p.stderr);
    assert!(
        !p.receipt.unwrap()["affected_packages"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn package_published_closure_checks_declared_files_and_unproven_ignored_inputs() {
    for ignored_build in [false, true] {
        let r = zero_fixture();
        configure(&r.0, false);
        let mut cfg: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
        if ignored_build {
            cfg["build_inputs"] = json!([".runtime/global-build-input.txt"]);
            write(&r.0, ".runtime/global-build-input.txt", b"build input\n");
        } else {
            cfg["development"]["a"]["checks"][0]["inputs"] = json!(["README.md"]);
        }
        write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
        write(&r.0, "pkg/src/lib.rs", b"pub fn value()->u8{2}\n");
        let runner = Checks::new();
        let q = request(&r.0, "patch", false, &runner);
        assert_eq!(close(&r.0, &q, &runner).code, 0);
        let q = release_request(&r.0, "patch");
        let plan = build(&r.0, "fix: publish", &q, &runner).unwrap().unwrap();
        // Model a historical publication by the old tool. The new tool must
        // not newly certify the old algorithm's missing ignored-input coverage.
        use_host_closure(&r.0, &runner, false);
        for (path, bytes) in plan.writes {
            fs::write(path, bytes).unwrap();
        }
        git(&r.0, &["add", "."]);
        git(
            &r.0,
            &["commit", "-m", "formal release with explicit inputs"],
        );
        let alternate = AlternateClosureHost(&runner);
        if !ignored_build {
            assert_eq!(
                development::preview(&r.0, &Request::default(), &alternate)
                    .receipt
                    .unwrap()["status"],
                "nothing-to-close"
            );
            write(&r.0, "README.md", b"changed explicit check input\n");
            let check_only = development::preview(&r.0, &Request::default(), &alternate);
            assert_eq!(check_only.receipt.unwrap()["status"], "nothing-to-close");
            // A true global build-input declaration still requires coverage;
            // the old aggregate cannot prove it by a check-input digest.
            cfg["build_inputs"] = json!(["README.md"]);
            write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
        }
        let p = development::preview(&r.0, &Request::default(), &alternate);
        assert_eq!(p.code, 0, "{}", p.stderr);
        let receipt = p.receipt.unwrap();
        assert_eq!(receipt["affected_packages"], json!(["a"]));
        assert_eq!(receipt["reused_published_packages"], json!([]));
    }
}

/// The native crate predates registration and inherits version 6 from its
/// workspace. The immutable schema-4 baseline deliberately has no packages.
fn adoption_fixture(broken_source_lock: bool) -> Repo {
    let r = fixture("unconfigured", true);
    write(&r.0, "pkg/Cargo.toml", b"[package]\nname='a'\nversion.workspace=true\nedition='2021'\n[workspace]\n[workspace.package]\nversion='6.0.0'\n");
    write(
        &r.0,
        "pkg/Cargo.lock",
        if broken_source_lock {
            b"version=4\n[[package]]\nname='a'\nversion='5.0.0'\n"
        } else {
            b"version=4\n[[package]]\nname='a'\nversion='6.0.0'\n"
        },
    );
    let cfg = json!({"schema_version":3,"release_policy":"explicit_release","strategy":"independent",
        "adapters":{},"packages":[],"product_inputs":["README.md",".gitignore"]});
    write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
    git(&r.0, &["add", "."]);
    git(&r.0, &["commit", "--amend", "--no-edit"]);
    migrate(&r.0);
    git(&r.0, &["add", "."]);
    git(&r.0, &["commit", "-m", "chore: empty package registry"]);
    let revision = git(&r.0, &["rev-parse", "HEAD"]);
    let mut cfg: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    assert_eq!(cfg["baseline"]["packages"], json!({}));
    cfg["adapters"] = json!({"rust":{"workspaces":["Cargo.toml"]}});
    cfg["product_inputs"] = json!([
        "README.md",
        ".gitignore",
        "Cargo.toml",
        "source/cli/",
        "source/hook/"
    ]);
    cfg["package_adoptions"] = json!([{"id":"core","revision":revision,"version":"6.0.0",
        "source":{"id":"core","language":"rust","manifest":"pkg/Cargo.toml","name":"a","workspace":"pkg/Cargo.toml"}}]);
    cfg["packages"] = json!([]);
    cfg["development"] = json!({});
    let mut members = Vec::new();
    let mut lock = String::from("version=4\n");
    for id in ["core", "b", "c", "d", "e", "f", "g", "cli", "hook"] {
        let name = if id == "core" { "a" } else { id };
        let product = matches!(id, "cli" | "hook");
        let version = if id == "core" {
            "6.0.0"
        } else if product {
            "1.0.0"
        } else {
            "0.1.0"
        };
        let path = format!("source/{id}/Cargo.toml");
        members.push(format!("'source/{id}'"));
        let version_line = if product {
            "version.workspace=true".into()
        } else {
            format!("version='{version}'")
        };
        write(
            &r.0,
            &path,
            format!("[package]\nname='{name}'\n{version_line}\nedition='2021'\n").as_bytes(),
        );
        write(
            &r.0,
            &format!("source/{id}/src/lib.rs"),
            b"pub fn extracted()->u8{2}\n",
        );
        lock.push_str(&format!(
            "[[package]]\nname='{name}'\nversion='{version}'\n"
        ));
        let identity = json!({"id":id,"language":"rust","manifest":path,"name":name});
        if !product {
            cfg["packages"].as_array_mut().unwrap().push(identity);
            cfg["development"][id] = policy(false);
        }
    }
    cfg["product_packages"] = json!([
        {"id":"cli","language":"rust","manifest":"source/cli/Cargo.toml","name":"cli"},
        {"id":"hook","language":"rust","manifest":"source/hook/Cargo.toml","name":"hook"}]);
    write(
        &r.0,
        "Cargo.toml",
        format!(
            "[workspace]\nmembers=[{}]\nresolver='2'\n[workspace.package]\nversion='1.0.0'\n",
            members.join(",")
        )
        .as_bytes(),
    );
    write(&r.0, "Cargo.lock", lock.as_bytes());
    for path in ["pkg/Cargo.toml", "pkg/Cargo.lock", "pkg/src/lib.rs"] {
        fs::remove_file(r.0.join(path)).unwrap();
    }
    write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
    r
}

fn adoption_request(root: &Path, runner: &dyn ProcessRunner) -> Request {
    let q = request(root, "major", true, runner);
    let mut analysis: Value =
        serde_json::from_slice(&fs::read(root.join(".runtime/develop.json")).unwrap()).unwrap();
    for id in ["b", "c", "d", "e", "f", "g"] {
        analysis["packages"][id] = json!({"level":"patch","reason":"new independently versioned package","compatibility":"compatible"});
    }
    write(
        root,
        ".runtime/develop.json",
        &serde_json::to_vec(&analysis).unwrap(),
    );
    q
}

#[test]
fn package_adoption_native_continuity_and_new_identities_close_once() {
    let r = adoption_fixture(false);
    let runner = Checks::new();
    let before: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    let mut proposal = before.clone();
    proposal.as_object_mut().unwrap().remove("baseline");
    write(
        &r.0,
        ".runtime/adoption-proposal.json",
        &serde_json::to_vec(&proposal).unwrap(),
    );
    let plan = migration_plan(&r.0, &runner, Some(".runtime/adoption-proposal.json"));
    assert_eq!(plan.code, 0, "{}", plan.stderr);
    assert_eq!(
        plan.receipt.unwrap()["configuration"]["baseline"],
        before["baseline"]
    );
    let q = adoption_request(&r.0, &runner);
    let closed = close(&r.0, &q, &runner);
    assert_eq!(closed.code, 0, "{}", closed.stderr);
    let ledger: Value =
        serde_json::from_slice(&fs::read(r.0.join(development::RECORD)).unwrap()).unwrap();
    let entries = ledger["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 7);
    let core = entries.iter().find(|e| e["id"] == "core").unwrap();
    assert_eq!(core["before"], "6.0.0");
    assert_eq!(core["after"], "7.0.0");
    for entry in entries.iter().filter(|e| e["id"] != "core") {
        assert!(entry["before"].is_null());
        assert_eq!(entry["after"], "0.1.0");
    }
    assert_eq!(fs::read(r.0.join("VERSION")).unwrap(), b"1.0.0\n");
    let after: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    assert_eq!(after["baseline"], before["baseline"]);
    for committed in [false, true] {
        if committed {
            git(&r.0, &["add", "."]);
            git(
                &r.0,
                &[
                    "commit",
                    "-m",
                    "feat!: register extracted packages with native provenance",
                ],
            );
        }
        let repeat = development::preview(&r.0, &Request::default(), &runner);
        assert_eq!(repeat.code, 0, "{}", repeat.stderr);
        assert_eq!(repeat.receipt.unwrap()["status"], "nothing-to-close");
        assert_eq!(
            fs::read(r.0.join(development::RECORD)).unwrap(),
            serde_json::to_vec_pretty(&ledger).unwrap()
        );
    }
    let q = release_request(&r.0, "major");
    let mut analysis: Value =
        serde_json::from_slice(&fs::read(r.0.join(".runtime/release.json")).unwrap()).unwrap();
    analysis["product"]["compatibility"] = json!("breaking");
    analysis["product"]["acceptance"] = json!({"fingerprint":analysis["fingerprint"],"scope":["product"],"reason":"accepted native API extraction","source":"explicit isolated fixture decision"});
    write(
        &r.0,
        ".runtime/release.json",
        &serde_json::to_vec(&analysis).unwrap(),
    );
    let release = build(&r.0, "feat!: product announcement", &q, &runner)
        .unwrap()
        .unwrap();
    let core = release
        .package_changes
        .iter()
        .find(|p| p.name == "a")
        .unwrap();
    assert_eq!(core.version_before, "6.0.0");
    assert_eq!(core.version_after, "7.0.0");
    assert_eq!(release.package_changes.len(), 7);
    assert!(
        !release
            .writes
            .contains_key(&r.0.join("source/core/Cargo.toml"))
    );
}

#[test]
fn package_adoption_rejects_forged_versions_duplicate_roles_and_manual_bumps() {
    let r = adoption_fixture(false);
    let original: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    for case in [
        "version",
        "duplicate",
        "name",
        "product",
        "stale",
        "unrelated",
    ] {
        let mut cfg = original.clone();
        match case {
            "version" => cfg["package_adoptions"][0]["version"] = json!("5.0.0"),
            "duplicate" => {
                let duplicate = cfg["package_adoptions"][0].clone();
                cfg["package_adoptions"]
                    .as_array_mut()
                    .unwrap()
                    .push(duplicate);
            }
            "name" => cfg["package_adoptions"][0]["source"]["name"] = json!("unrelated"),
            "product" => {
                cfg["package_adoptions"][0]["source"]["manifest"] = json!("source/cli/Cargo.toml")
            }
            "stale" => {
                cfg["package_adoptions"][0]["revision"] = json!(git(&r.0, &["rev-parse", "HEAD^"]))
            }
            "unrelated" => {
                let tree = git(&r.0, &["rev-parse", "HEAD^{tree}"]);
                cfg["package_adoptions"][0]["revision"] =
                    json!(git(&r.0, &["commit-tree", &tree, "-m", "unrelated root"]));
            }
            _ => unreachable!(),
        }
        write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
        let p = development::preview(&r.0, &Request::default(), &Checks::new());
        assert_ne!(p.code, 0, "{case}: {}", p.stdout);
        assert!(p.stderr.contains("adoption"), "{case}: {}", p.stderr);
        assert!(!r.0.join(development::RECORD).exists());
    }
    write(&r.0, CONFIG, &serde_json::to_vec(&original).unwrap());
    let manifest = fs::read_to_string(r.0.join("source/core/Cargo.toml"))
        .unwrap()
        .replace("6.0.0", "7.0.0");
    write(&r.0, "source/core/Cargo.toml", manifest.as_bytes());
    let lock = fs::read_to_string(r.0.join("Cargo.lock"))
        .unwrap()
        .replace("6.0.0", "7.0.0");
    write(&r.0, "Cargo.lock", lock.as_bytes());
    let p = development::preview(&r.0, &Request::default(), &Checks::new());
    assert_ne!(p.code, 0);
    assert!(
        p.stderr.contains("outside managed development closure"),
        "{}",
        p.stderr
    );
}

#[test]
fn package_adoption_requires_consistent_historical_lock_and_unregistered_source() {
    let r = adoption_fixture(true);
    let p = development::preview(&r.0, &Request::default(), &Checks::new());
    assert_ne!(p.code, 0);
    assert!(p.stderr.contains("lock"), "{}", p.stderr);
    let r = fixture("independent", true);
    let mut cfg: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    cfg["package_adoptions"] = json!([{"id":"a","revision":git(&r.0,&["rev-parse","HEAD"]),"version":"1.0.0",
        "source":{"id":"a","name":"a","manifest":"pkg/Cargo.toml","language":"rust","workspace":"pkg/Cargo.toml"}}]);
    write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
    let p = development::preview(&r.0, &Request::default(), &Checks::new());
    assert_ne!(p.code, 0);
    assert!(p.stderr.contains("already registered"), "{}", p.stderr);
}

#[test]
fn package_adoption_committed_provenance_cannot_be_removed_or_replaced() {
    let r = adoption_fixture(false);
    let runner = Checks::new();
    let q = adoption_request(&r.0, &runner);
    let result = close(&r.0, &q, &runner);
    assert_eq!(result.code, 0, "{}", result.stderr);
    git(&r.0, &["add", "."]);
    git(&r.0, &["commit", "-m", "feat!: adopt native package"]);
    let original: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    for remove in [true, false] {
        let mut cfg = original.clone();
        if remove {
            cfg.as_object_mut().unwrap().remove("package_adoptions");
        } else {
            cfg["package_adoptions"][0]["version"] = json!("5.0.0");
        }
        write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
        let p = development::preview(&r.0, &Request::default(), &runner);
        assert_ne!(p.code, 0);
        assert!(p.stderr.contains("adoption"), "{}", p.stderr);
    }
}

#[test]
fn package_adoption_preserves_later_managed_identity_moves() {
    let r = adoption_fixture(false);
    let runner = Checks::new();
    let q = adoption_request(&r.0, &runner);
    let result = close(&r.0, &q, &runner);
    assert_eq!(result.code, 0, "{}", result.stderr);
    git(&r.0, &["add", "."]);
    git(&r.0, &["commit", "-m", "feat!: adopt native package"]);
    fs::rename(r.0.join("source/core"), r.0.join("source/renamed")).unwrap();
    let workspace = fs::read_to_string(r.0.join("Cargo.toml"))
        .unwrap()
        .replace("source/core", "source/renamed");
    write(&r.0, "Cargo.toml", workspace.as_bytes());
    let mut cfg: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    let from = cfg["packages"][0].clone();
    cfg["packages"][0]["manifest"] = json!("source/renamed/Cargo.toml");
    let to = cfg["packages"][0].clone();
    cfg["lifecycle"] = json!([{"id":"move-core","base":cfg["baseline"]["revision"],"operation":"move","compatibility":"compatible","from":[from],"to":[to]}]);
    write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
    let q = request(&r.0, "patch", false, &runner);
    let result = close(&r.0, &q, &runner);
    assert_eq!(result.code, 0, "{}", result.stderr);
    let ledger: Value =
        serde_json::from_slice(&fs::read(r.0.join(development::RECORD)).unwrap()).unwrap();
    let latest = ledger["entries"]
        .as_array()
        .unwrap()
        .iter()
        .rev()
        .find(|e| e["id"] == "core")
        .unwrap();
    assert_eq!(latest["before"], "7.0.0");
    assert_eq!(latest["after"], "7.0.1");
}

fn scoped_rust_build_fixture() -> Repo {
    let r = mixed_product_repo();
    let mut cfg: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    cfg["development"] = json!({"library":policy(false)});
    write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
    let manifest = fs::read_to_string(r.0.join("source/library/Cargo.toml")).unwrap();
    write(
        &r.0,
        "source/library/Cargo.toml",
        format!("{manifest}\n[dependencies]\nremote='1'\n").as_bytes(),
    );
    let mut lock = fs::read_to_string(r.0.join("Cargo.lock"))
        .unwrap()
        .parse::<toml_edit::DocumentMut>()
        .unwrap();
    let packages = lock["package"].as_array_of_tables_mut().unwrap();
    let library = packages
        .iter_mut()
        .find(|p| p["name"].as_str() == Some("library"))
        .unwrap();
    let mut dependencies = toml_edit::Array::new();
    dependencies.push("remote");
    library["dependencies"] = toml_edit::value(dependencies);
    for name in ["remote", "unrelated"] {
        let mut table = toml_edit::Table::new();
        table["name"] = toml_edit::value(name);
        table["version"] = toml_edit::value("1.0.0");
        table["source"] = toml_edit::value("registry+https://github.com/rust-lang/crates.io-index");
        packages.push(table);
    }
    write(&r.0, "Cargo.lock", lock.to_string().as_bytes());
    let runner = Checks::new();
    let q = request(&r.0, "minor", false, &runner);
    let result = close(&r.0, &q, &runner);
    assert_eq!(result.code, 0, "{}", result.stderr);
    r
}

fn change_locked_version(root: &Path, name: &str, version: &str) {
    let mut lock = fs::read_to_string(root.join("Cargo.lock"))
        .unwrap()
        .parse::<toml_edit::DocumentMut>()
        .unwrap();
    let package = lock["package"]
        .as_array_of_tables_mut()
        .unwrap()
        .iter_mut()
        .find(|p| p["name"].as_str() == Some(name))
        .unwrap();
    package["version"] = toml_edit::value(version);
    write(root, "Cargo.lock", lock.to_string().as_bytes());
}

#[test]
fn package_development_scoped_rust_build_preserves_product_release_evidence() {
    let r = scoped_rust_build_fixture();
    let ledger = fs::read(r.0.join(development::RECORD)).unwrap();
    let q = release_request(&r.0, "minor");
    let release = build(&r.0, "fix: product announcement", &q, &Checks::new())
        .unwrap()
        .unwrap();
    assert_eq!(release.new_version.to_string(), "1.1.0");
    for (path, bytes) in &release.writes {
        fs::write(path, bytes).unwrap();
    }
    // A version write before the release commit remains a manual-version gate;
    // only a committed formal release establishes the new product boundary.
    let uncommitted = development::preview(&r.0, &Request::default(), &Checks::new());
    assert_ne!(uncommitted.code, 0);
    assert!(
        uncommitted.stderr.contains("VERSION differs"),
        "{}",
        uncommitted.stderr
    );
    git(&r.0, &["add", "."]);
    git(&r.0, &["commit", "-m", "fix: product release"]);
    let p = development::preview(&r.0, &Request::default(), &Checks::new());
    assert_eq!(p.code, 0, "{}", p.stderr);
    assert_eq!(p.receipt.unwrap()["status"], "nothing-to-close");
    assert_eq!(fs::read(r.0.join(development::RECORD)).unwrap(), ledger);
}

#[test]
fn package_development_scoped_rust_build_revalidates_profile_and_real_dependencies() {
    let r = scoped_rust_build_fixture();
    let runner = Checks::new();
    let manifest = fs::read_to_string(r.0.join("Cargo.toml")).unwrap();
    write(
        &r.0,
        "Cargo.toml",
        format!("{manifest}\n[profile.release]\nlto=true\n").as_bytes(),
    );
    for dependency_change in [false, true] {
        if dependency_change {
            change_locked_version(&r.0, "remote", "1.0.1");
        }
        let q = request(&r.0, "patch", false, &runner);
        let result = close(&r.0, &q, &runner);
        assert_eq!(result.code, 0, "{}", result.stderr);
        let ledger = development::Ledger::load(&r.0).unwrap();
        let entry = ledger.latest("library").unwrap();
        assert_eq!(entry.before.as_deref(), Some("0.5.0"));
        assert_eq!(entry.after.as_deref(), Some("0.5.0"));
        assert_eq!(entry.level, 0);
    }
    assert_eq!(runner.calls.lock().unwrap().len(), 2);
}

#[test]
fn package_development_check_inputs_do_not_replace_explicit_build_input_scope() {
    let r = scoped_rust_build_fixture();
    let runner = Checks::new();
    change_locked_version(&r.0, "unrelated", "1.0.1");
    let p = development::preview(&r.0, &Request::default(), &runner);
    assert_eq!(p.code, 0, "{}", p.stderr);
    assert_eq!(p.receipt.unwrap()["status"], "nothing-to-close");
    let mut cfg: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    cfg["development"]["library"]["checks"][0]["inputs"] = json!(["Cargo.lock"]);
    let ledger = fs::read(r.0.join(development::RECORD)).unwrap();
    write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
    let q = request(&r.0, "patch", false, &runner);
    assert_eq!(close(&r.0, &q, &runner).code, 0);
    change_locked_version(&r.0, "unrelated", "1.0.2");
    let p = development::preview(&r.0, &Request::default(), &runner);
    assert_eq!(p.code, 0, "{}", p.stderr);
    assert_eq!(p.receipt.unwrap()["status"], "nothing-to-close");
    assert_eq!(fs::read(r.0.join(development::RECORD)).unwrap(), ledger);
    // Full-lock durable coverage is still available when the project actually
    // declares it as a global build input, rather than only a task-check input.
    cfg["build_inputs"] = json!(["Cargo.lock"]);
    write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
    let p = development::preview(&r.0, &Request::default(), &runner);
    assert_eq!(p.code, 0, "{}", p.stderr);
    assert_eq!(p.receipt.unwrap()["affected_packages"], json!(["library"]));
}

#[test]
fn package_development_scoped_rust_build_revalidates_actual_core_consumers() {
    let r = adoption_fixture(false);
    let manifest = fs::read_to_string(r.0.join("source/b/Cargo.toml")).unwrap();
    write(
        &r.0,
        "source/b/Cargo.toml",
        format!("{manifest}\n[dependencies]\na={{path='../core',version='^6'}}\n").as_bytes(),
    );
    let mut lock = fs::read_to_string(r.0.join("Cargo.lock"))
        .unwrap()
        .parse::<toml_edit::DocumentMut>()
        .unwrap();
    let b = lock["package"]
        .as_array_of_tables_mut()
        .unwrap()
        .iter_mut()
        .find(|p| p["name"].as_str() == Some("b"))
        .unwrap();
    let mut dependencies = toml_edit::Array::new();
    dependencies.push("a");
    b["dependencies"] = toml_edit::value(dependencies);
    write(&r.0, "Cargo.lock", lock.to_string().as_bytes());
    let runner = Checks::new();
    let q = adoption_request(&r.0, &runner);
    let result = close(&r.0, &q, &runner);
    assert_eq!(result.code, 0, "{}", result.stderr);
    write(
        &r.0,
        "source/core/src/lib.rs",
        b"pub fn extracted()->u8{3}\n",
    );
    let q = request(&r.0, "patch", false, &runner);
    let result = close(&r.0, &q, &runner);
    assert_eq!(result.code, 0, "{}", result.stderr);
    let ledger = development::Ledger::load(&r.0).unwrap();
    assert_eq!(
        ledger.latest("core").unwrap().after.as_deref(),
        Some("7.0.1")
    );
    assert_eq!(ledger.latest("b").unwrap().after.as_deref(), Some("0.1.0"));
    assert_eq!(ledger.latest("b").unwrap().level, 0);
    for id in ["c", "d", "e", "f", "g"] {
        assert_eq!(ledger.entries.iter().filter(|e| e.id == id).count(), 1);
    }
}

fn candidate_projection(
    _root: &Path,
    writes: &BTreeMap<String, Vec<u8>>,
) -> Result<BTreeMap<String, Vec<u8>>, String> {
    Ok(BTreeMap::from([(
        ".runtime/candidate-projection.txt".into(),
        writes.get("pkg/Cargo.toml").cloned().unwrap_or_default(),
    )]))
}

#[test]
fn package_development_projection_preview_is_readonly_and_final_outputs_are_closed_together() {
    let r = fixture("independent", true);
    configure(&r.0, false);
    change(&r.0, true);
    write(
        &r.0,
        ".runtime/candidate-projection.txt",
        b"original projection",
    );
    let runner = Checks::new();
    let q = request(&r.0, "patch", false, &runner);
    let p = development::preview_with_materializer(&r.0, &q, &runner, Some(&candidate_projection));
    assert_eq!(p.code, 0, "{}", p.stderr);
    assert_eq!(
        fs::read(r.0.join(".runtime/candidate-projection.txt")).unwrap(),
        b"original projection"
    );
    let result = development::apply_with_materializer(
        &r.0,
        &q,
        p.receipt.unwrap()["fingerprint"].as_str().unwrap(),
        &runner,
        Some(&candidate_projection),
    );
    assert_eq!(result.code, 0, "{}", result.stderr);
    assert_eq!(
        fs::read(r.0.join(".runtime/candidate-projection.txt")).unwrap(),
        fs::read(r.0.join("pkg/Cargo.toml")).unwrap()
    );
    let repeat = development::preview_with_materializer(
        &r.0,
        &Request::default(),
        &runner,
        Some(&candidate_projection),
    );
    assert_eq!(repeat.receipt.unwrap()["status"], "nothing-to-close");
}

#[test]
fn package_development_projection_is_rolled_back_when_validation_fails() {
    let r = fixture("independent", true);
    configure(&r.0, false);
    change(&r.0, true);
    write(
        &r.0,
        ".runtime/candidate-projection.txt",
        b"original projection",
    );
    let mut runner = Checks::new();
    runner.fail = Some("normal".into());
    let before = fs::read(r.0.join("pkg/Cargo.toml")).unwrap();
    let q = request(&r.0, "patch", false, &runner);
    let p = development::preview_with_materializer(&r.0, &q, &runner, Some(&candidate_projection));
    assert_eq!(p.code, 0, "{}", p.stderr);
    let result = development::apply_with_materializer(
        &r.0,
        &q,
        p.receipt.unwrap()["fingerprint"].as_str().unwrap(),
        &runner,
        Some(&candidate_projection),
    );
    assert_ne!(result.code, 0);
    assert_eq!(
        fs::read(r.0.join(".runtime/candidate-projection.txt")).unwrap(),
        b"original projection"
    );
    assert_eq!(fs::read(r.0.join("pkg/Cargo.toml")).unwrap(), before);
    assert!(!r.0.join(development::RECORD).exists());
}

#[test]
fn package_development_projection_cannot_change_product_version_or_configuration() {
    let r = fixture("independent", true);
    configure(&r.0, false);
    change(&r.0, true);
    let runner = Checks::new();
    let q = request(&r.0, "patch", false, &runner);
    let renderer = |_root: &Path, _writes: &BTreeMap<String, Vec<u8>>| {
        Ok(BTreeMap::from([("VERSION".into(), b"9.0.0\n".to_vec())]))
    };
    let p = development::preview_with_materializer(&r.0, &q, &runner, Some(&renderer));
    assert_ne!(p.code, 0);
    assert!(p.stderr.contains("protected state"));
    assert_eq!(fs::read(r.0.join("VERSION")).unwrap(), b"1.0.0\n");
}

#[test]
fn package_development_projection_recovers_after_process_interruption() {
    let r = fixture("independent", true);
    configure(&r.0, false);
    change(&r.0, true);
    write(
        &r.0,
        ".runtime/candidate-projection.txt",
        b"original projection",
    );
    let mut runner = Checks::new();
    runner.action = Some("crash".into());
    let q = request(&r.0, "patch", false, &runner);
    let before = fs::read(r.0.join("pkg/Cargo.toml")).unwrap();
    let p = development::preview_with_materializer(&r.0, &q, &runner, Some(&candidate_projection));
    let fingerprint = p.receipt.unwrap()["fingerprint"]
        .as_str()
        .unwrap()
        .to_owned();
    let interrupted = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        development::apply_with_materializer(
            &r.0,
            &q,
            &fingerprint,
            &runner,
            Some(&candidate_projection),
        )
    }));
    assert!(interrupted.is_err());
    assert_eq!(development::recover(&r.0).code, 0);
    assert_eq!(
        fs::read(r.0.join(".runtime/candidate-projection.txt")).unwrap(),
        b"original projection"
    );
    assert_eq!(fs::read(r.0.join("pkg/Cargo.toml")).unwrap(), before);
}

#[test]
fn package_development_release_reports_more_than_three_changed_packages() {
    let r = fixture("unconfigured", true);
    let mut cfg: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    for id in ["b", "c", "d"] {
        let manifest = format!("{id}/Cargo.toml");
        write(
            &r.0,
            &manifest,
            format!("[package]\nname='{id}'\nversion='1.0.0'\nedition='2021'\n[workspace]\n")
                .as_bytes(),
        );
        write(
            &r.0,
            &format!("{id}/src/lib.rs"),
            b"pub fn value()->u8{1}\n",
        );
        write(
            &r.0,
            &format!("{id}/Cargo.lock"),
            format!("version=4\n[[package]]\nname='{id}'\nversion='1.0.0'\n").as_bytes(),
        );
        cfg["adapters"]["rust"]["workspaces"]
            .as_array_mut()
            .unwrap()
            .push(json!(manifest));
        cfg["packages"]
            .as_array_mut()
            .unwrap()
            .push(json!({"id":id,"name":id,"language":"rust","manifest":manifest}));
    }
    write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
    git(&r.0, &["add", "."]);
    git(&r.0, &["commit", "--amend", "--no-edit"]);
    migrate(&r.0);
    let mut cfg: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    cfg["development"] =
        json!({"a":policy(false),"b":policy(false),"c":policy(false),"d":policy(false)});
    write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
    change(&r.0, true);
    for id in ["b", "c", "d"] {
        write(
            &r.0,
            &format!("{id}/src/lib.rs"),
            b"pub fn value()->u8{2}\n",
        );
    }
    let runner = Checks::new();
    let q = request(&r.0, "patch", false, &runner);
    let result = close(&r.0, &q, &runner);
    assert_eq!(result.code, 0, "{}", result.stderr);
    let q = release_request(&r.0, "patch");
    let p = build(&r.0, "fix: all packages", &q, &runner)
        .unwrap()
        .unwrap();
    assert_eq!(p.package_changes.len(), 4);
    for change in &p.package_changes {
        assert_eq!(change.version_before, "1.0.0");
        assert_eq!(change.version_after, "1.0.1");
        assert!(!change.reason.is_empty());
    }
    assert_eq!(p.details.unwrap()["packages"].as_array().unwrap().len(), 4);
}

#[test]
fn package_development_declared_ignored_inputs_bind_preview() {
    for field in ["inputs", "contracts"] {
        let r = zero_fixture();
        configure(&r.0, true);
        write(&r.0, ".runtime/contract.txt", b"original contract");
        let mut cfg: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
        if field == "inputs" {
            cfg["development"]["a"]["checks"][0]["inputs"] = json!([".runtime/contract.txt"]);
        } else {
            cfg["development"]["a"]["stability"]["contracts"] = json!([".runtime/contract.txt"]);
        }
        write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
        change(&r.0, true);
        let runner = Checks::new();
        let q = request(&r.0, "patch", false, &runner);
        let p = development::preview(&r.0, &q, &runner);
        assert_eq!(p.code, 0, "{}", p.stderr);
        let before = fs::read(r.0.join("pkg/Cargo.toml")).unwrap();
        write(&r.0, ".runtime/contract.txt", b"changed contract");
        let result = development::apply(
            &r.0,
            &q,
            p.receipt.unwrap()["fingerprint"].as_str().unwrap(),
            &runner,
        );
        assert_ne!(result.code, 0, "{field}");
        assert!(
            result.stderr.contains("fingerprint") || result.stderr.contains("stale"),
            "{}",
            result.stderr
        );
        assert!(runner.calls.lock().unwrap().is_empty());
        assert_eq!(fs::read(r.0.join("pkg/Cargo.toml")).unwrap(), before);
    }
}

#[test]
fn package_development_validation_only_change_cannot_fake_interface_impact() {
    let r = python_pair(false);
    change(&r.0, false);
    write(&r.0, "consumer/b.py", b"VALUE=2\n");
    let runner = Checks::new();
    let q = request(&r.0, "patch", false, &runner);
    assert_eq!(close(&r.0, &q, &runner).code, 0);
    let before = fs::read(r.0.join(development::RECORD)).unwrap();
    let mut cfg: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    cfg["development"]["a"]["checks"][0]["timeout_seconds"] = json!(120);
    write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
    let current = development::preview(&r.0, &Request::default(), &runner);
    assert_eq!(current.receipt.unwrap()["status"], "nothing-to-close");
    // Enter a real development task for b. A's policy-only edit cannot serve
    // as the changed dependency that justifies b's public re-export claim.
    write(&r.0, "consumer/b.py", b"VALUE=3\n");
    let q = request(&r.0, "patch", false, &runner);
    let mut a: Value =
        serde_json::from_slice(&fs::read(r.0.join(".runtime/develop.json")).unwrap()).unwrap();
    a["interface_impacts"] =
        json!({"b":{"dependencies":["a"],"reason":"unfounded re-export claim"}});
    a["packages"]["b"] = json!({"level":"minor","reason":"unfounded","compatibility":"compatible"});
    write(
        &r.0,
        ".runtime/develop.json",
        &serde_json::to_vec(&a).unwrap(),
    );
    let result = close(&r.0, &q, &runner);
    assert!(result.stderr.contains("not rooted"), "{}", result.stderr);
    assert_eq!(fs::read(r.0.join(development::RECORD)).unwrap(), before);
}

#[test]
fn package_development_deleted_source_remains_closed_after_commit() {
    let r = fixture("unconfigured", true);
    write(&r.0, "pkg/src/obsolete.rs", b"// obsolete\n");
    git(&r.0, &["add", "."]);
    git(&r.0, &["commit", "--amend", "--no-edit"]);
    migrate(&r.0);
    configure(&r.0, false);
    fs::remove_file(r.0.join("pkg/src/obsolete.rs")).unwrap();
    let runner = Checks::new();
    let q = request(&r.0, "patch", false, &runner);
    let result = close(&r.0, &q, &runner);
    assert_eq!(result.code, 0, "{}", result.stderr);
    git(&r.0, &["add", "."]);
    git(&r.0, &["commit", "-m", "fix: remove obsolete source"]);
    let p = development::preview(&r.0, &Request::default(), &runner);
    assert_eq!(p.code, 0, "{}", p.stderr);
    assert_eq!(p.receipt.unwrap()["status"], "nothing-to-close");
}

#[test]
fn package_development_stable_child_breaking_maps_to_unstable_product_minor() {
    let r = fixture("unconfigured", true);
    write(&r.0, "VERSION", b"0.5.0\n");
    git(&r.0, &["add", "."]);
    git(&r.0, &["commit", "--amend", "--no-edit"]);
    migrate(&r.0);
    configure(&r.0, false);
    change(&r.0, true);
    let runner = Checks::new();
    let q = request(&r.0, "major", true, &runner);
    let result = close(&r.0, &q, &runner);
    assert_eq!(result.code, 0, "{}", result.stderr);
    let preview = super::preview(&r.0, "feat: release", &Request::default(), &runner);
    assert_eq!(preview.code, 0, "{}", preview.stderr);
    let constraints = preview.receipt.unwrap()["product_constraints"].clone();
    assert_eq!(constraints["minimum_level"], "minor");
    assert_eq!(constraints["minimum_version"], "0.6.0");
    assert_eq!(constraints["acceptance_required"], true);
    let q = release_request(&r.0, "minor");
    let mut a: Value =
        serde_json::from_slice(&fs::read(r.0.join(".runtime/release.json")).unwrap()).unwrap();
    a["product"]["compatibility"] = json!("breaking");
    a["product"]["acceptance"] = json!({"fingerprint":a["fingerprint"],"scope":["product"],"reason":"accepted breaking product","source":"explicit fixture authorization"});
    write(
        &r.0,
        ".runtime/release.json",
        &serde_json::to_vec(&a).unwrap(),
    );
    let p = build(&r.0, "feat!: release", &q, &runner).unwrap().unwrap();
    assert_eq!(p.new_version.to_string(), "0.6.0");
    assert_eq!(p.package_changes[0].version_after, "2.0.0");
    assert!(String::from_utf8_lossy(&p.writes[&r.0.join("CHANGELOG.md")]).contains("BREAKING"));
}

pub(super) struct Checks {
    fail: Option<String>,
    action: Option<String>,
    calls: Mutex<Vec<String>>,
}
impl Checks {
    pub(super) fn new() -> Self {
        Self {
            fail: None,
            action: None,
            calls: Mutex::new(vec![]),
        }
    }
}
impl ProcessRunner for Checks {
    fn run(&self, request: &ProcessRequest) -> std::io::Result<crate::ProcessOutput> {
        if request.program == "fixture-check"
            || Path::new(&request.program)
                .file_name()
                .is_some_and(|n| n == "check-tool.exe")
        {
            let id = request.args.first().unwrap().to_string_lossy().into_owned();
            self.calls.lock().unwrap().push(id.clone());
            if self.action.as_deref() == Some("crash") {
                panic!("simulated process interruption after candidate writes");
            }
            if self.action.as_deref() == Some("mutate") {
                fs::write(request.cwd.join("pkg/src/lib.rs"), b"external edit\n")?;
            }
            if self.action.as_deref() == Some("new-file") {
                fs::write(request.cwd.join("pkg/src/new.rs"), b"// generated source\n")?;
            }
            if self.action.as_deref() == Some("new-build-file") {
                fs::create_dir_all(request.cwd.join(".runtime/global"))?;
                fs::write(
                    request.cwd.join(".runtime/global/new.txt"),
                    b"generated build input",
                )?;
            }
            if self.action.as_deref() == Some("artifact-drift") {
                fs::create_dir_all(request.cwd.join(".runtime"))?;
                fs::write(request.cwd.join(".runtime/result.bin"), id.as_bytes())?;
            }
            let fail = self.fail.as_ref() == Some(&id);
            return Ok(crate::ProcessOutput {
                code: if fail { 1 } else { 0 },
                stdout: b"checked final version".to_vec(),
                stderr: if fail {
                    b"fixture failure".to_vec()
                } else {
                    vec![]
                },
                timed_out: false,
            });
        }
        SystemProcessRunner.run(request)
    }
}
pub(super) fn policy(stable: bool) -> Value {
    let mut p = json!({"checks":[{"id":"normal","program":"fixture-check","args":["normal"]}]});
    if stable {
        p["stability"] = json!({"contracts":["README.md"],"checks":[
        {"id":"core","kind":"core","program":"fixture-check","args":["core"]},
        {"id":"integration","kind":"integration","program":"fixture-check","args":["integration"]}]});
    }
    p
}
pub(super) fn configure(root: &Path, stable: bool) {
    let mut v: Value = serde_json::from_slice(&fs::read(root.join(CONFIG)).unwrap()).unwrap();
    v["development"] = json!({"a":policy(stable)});
    write(root, CONFIG, &serde_json::to_vec(&v).unwrap());
}
fn zero_fixture() -> Repo {
    let r = fixture("unconfigured", true);
    write(
        &r.0,
        "pkg/Cargo.toml",
        b"[package]\nname='a'\nversion='0.3.0'\nedition='2021'\n[workspace]\n",
    );
    write(
        &r.0,
        "pkg/Cargo.lock",
        b"version=4\n[[package]]\nname='a'\nversion='0.3.0'\n",
    );
    git(&r.0, &["add", "."]);
    git(&r.0, &["commit", "--amend", "--no-edit"]);
    migrate(&r.0);
    r
}
pub(super) fn request(
    root: &Path,
    level: &str,
    breaking: bool,
    runner: &dyn ProcessRunner,
) -> Request {
    let p = development::preview(root, &Request::default(), runner);
    assert_eq!(p.code, 0, "{}", p.stderr);
    let v = p.receipt.unwrap();
    let mut packages = serde_json::Map::new();
    let config: Value = serde_json::from_slice(&fs::read(root.join(CONFIG)).unwrap()).unwrap();
    let mut validation = serde_json::Map::new();
    for id in v["affected_packages"].as_array().unwrap() {
        let mut a = json!({"level":level,"reason":"fixture change","compatibility":if breaking{"breaking"}else{"compatible"}});
        if breaking {
            a["acceptance"] = json!({"fingerprint":v["fingerprint"],"scope":[id],"reason":"authorized breaking fixture","source":"isolated test authorization"});
        }
        packages.insert(id.as_str().unwrap().into(), a);
        validation.insert(
            id.as_str().unwrap().into(),
            config["development"][id.as_str().unwrap()]["checks"].clone(),
        );
    }
    write(
        root,
        ".runtime/develop.json",
        &serde_json::to_vec(
            &json!({"schema":1,"fingerprint":v["fingerprint"],"packages":packages,"validation":validation}),
        )
        .unwrap(),
    );
    Request {
        analysis_file: Some(".runtime/develop.json".into()),
    }
}
pub(super) fn close(
    root: &Path,
    request: &Request,
    runner: &dyn ProcessRunner,
) -> crate::CommandOutcome {
    let p = development::preview(root, request, runner);
    if p.code != 0 {
        return p;
    }
    development::apply(
        root,
        request,
        p.receipt.unwrap()["fingerprint"].as_str().unwrap(),
        runner,
    )
}
fn release_request(root: &Path, level: &str) -> Request {
    let p = preview(
        root,
        "fix: release",
        &Request::default(),
        &SystemProcessRunner,
    );
    assert_eq!(p.code, 0, "{}", p.stderr);
    write(root,".runtime/release.json",&serde_json::to_vec(&json!({"schema":1,"fingerprint":p.receipt.unwrap()["fingerprint"],"product":{"level":level,"reason":"cumulative release","compatibility":"compatible"}})).unwrap());
    Request {
        analysis_file: Some(".runtime/release.json".into()),
    }
}

#[test]
fn package_development_closes_once_and_release_only_writes_product() {
    let r = fixture("independent", true);
    configure(&r.0, false);
    change(&r.0, true);
    let runner = Checks::new();
    let q = request(&r.0, "patch", false, &runner);
    let index = fs::read(r.0.join(".git/index")).unwrap();
    let head = git(&r.0, &["rev-parse", "HEAD"]);
    let p = development::preview(&r.0, &q, &runner);
    assert_eq!(p.code, 0, "{}", p.stderr);
    assert!(!r.0.join(development::RECORD).exists());
    let result = close(&r.0, &q, &runner);
    assert_eq!(result.code, 0, "{}", result.stderr);
    assert_eq!(
        result.receipt.unwrap()["package_changes"][0]["version_after"],
        "1.0.1"
    );
    assert_eq!(fs::read(r.0.join("VERSION")).unwrap(), b"1.0.0\n");
    assert_eq!(fs::read(r.0.join(".git/index")).unwrap(), index);
    assert_eq!(git(&r.0, &["rev-parse", "HEAD"]), head);
    let repeat = development::preview(&r.0, &Request::default(), &runner);
    assert_eq!(repeat.code, 0, "{}", repeat.stderr);
    assert_eq!(repeat.receipt.unwrap()["status"], "nothing-to-close");
    let q = release_request(&r.0, "patch");
    let p = build(&r.0, "fix: publish", &q, &SystemProcessRunner)
        .unwrap()
        .unwrap();
    assert_eq!(p.package_changes[0].version_before, "1.0.0");
    assert_eq!(p.package_changes[0].version_after, "1.0.1");
    assert!(!p.writes.contains_key(&r.0.join("pkg/Cargo.toml")));
    assert!(!p.writes.contains_key(&r.0.join("pkg/Cargo.lock")));
}
#[test]
fn package_development_blocks_release_until_validated_and_after_new_source() {
    let r = fixture("independent", true);
    configure(&r.0, false);
    change(&r.0, true);
    let p = preview(
        &r.0,
        "fix: release",
        &Request::default(),
        &SystemProcessRunner,
    );
    assert!(
        p.stderr.contains("development closure required: a"),
        "{}",
        p.stderr
    );
    let runner = Checks::new();
    let q = request(&r.0, "patch", false, &runner);
    assert_eq!(close(&r.0, &q, &runner).code, 0);
    write(&r.0, "pkg/src/lib.rs", b"pub fn value()->u8{3}\n");
    assert!(
        preview(&r.0, "fix: release", &Request::default(), &runner)
            .stderr
            .contains("development closure required")
    );
}
#[test]
fn package_development_accumulates_multiple_committed_closures() {
    let r = zero_fixture();
    configure(&r.0, false);
    let runner = Checks::new();
    change(&r.0, true);
    let q = request(&r.0, "patch", false, &runner);
    let a = close(&r.0, &q, &runner);
    assert_eq!(a.code, 0, "{}", a.stderr);
    git(&r.0, &["add", "."]);
    git(&r.0, &["commit", "-m", "fix: first development"]);
    write(&r.0, "pkg/src/lib.rs", b"pub fn value()->u8{3}\n");
    let q = request(&r.0, "minor", false, &runner);
    let a = close(&r.0, &q, &runner);
    assert_eq!(a.code, 0, "{}", a.stderr);
    let q = release_request(&r.0, "minor");
    let p = build(&r.0, "feat: publish", &q, &runner).unwrap().unwrap();
    assert_eq!(p.package_changes[0].version_before, "0.3.0");
    assert_eq!(p.package_changes[0].version_after, "0.4.0");
    assert_eq!(p.new_version.to_string(), "1.1.0");
}
#[test]
fn package_development_zero_breaking_is_minor_and_acceptance_persists() {
    let r = zero_fixture();
    configure(&r.0, true);
    change(&r.0, true);
    let runner = Checks::new();
    let q = request(&r.0, "minor", true, &runner);
    let a = close(&r.0, &q, &runner);
    assert_eq!(a.code, 0, "{}", a.stderr);
    assert_eq!(
        a.receipt.unwrap()["package_changes"][0]["version_after"],
        "0.4.0"
    );
    fs::remove_file(r.0.join(".runtime/develop.json")).unwrap();
    let ledger = development::Ledger::load(&r.0).unwrap();
    let preview = super::preview(&r.0, "feat: release", &Request::default(), &runner);
    assert_eq!(preview.code, 0, "{}", preview.stderr);
    let constraints = preview.receipt.unwrap()["product_constraints"].clone();
    assert_eq!(constraints["minimum_level"], "major");
    assert_eq!(constraints["minimum_version"], "2.0.0");
    assert_eq!(constraints["breaking_packages"][0]["id"], "a");
    assert_eq!(constraints["acceptance_scope"], json!(["product"]));
    assert_eq!(
        ledger.entries[0].acceptance.as_ref().unwrap()["source"],
        "isolated test authorization"
    );
    let q = release_request(&r.0, "minor");
    assert!(
        build(&r.0, "feat: release", &q, &runner)
            .unwrap_err()
            .contains("cumulative breaking")
    );
}
#[test]
fn package_development_automatic_stability_and_failed_optional_check() {
    for fail in [false, true] {
        let r = zero_fixture();
        configure(&r.0, true);
        change(&r.0, true);
        let mut runner = Checks::new();
        if fail {
            runner.fail = Some("integration".into());
        }
        let q = request(&r.0, "patch", false, &runner);
        let a = close(&r.0, &q, &runner);
        assert_eq!(a.code, 0, "{}", a.stderr);
        assert_eq!(
            a.receipt.unwrap()["package_changes"][0]["version_after"],
            if fail { "0.3.1" } else { "1.0.0" }
        );
        let calls = runner.calls.lock().unwrap();
        assert!(calls.contains(&"integration".to_string()));
        assert_eq!(
            calls.iter().filter(|c| *c == "normal").count(),
            if fail { 2 } else { 1 }
        );
    }
}
#[test]
fn package_development_failed_normal_check_rolls_back_without_closure() {
    let r = fixture("independent", true);
    configure(&r.0, false);
    change(&r.0, true);
    let mut runner = Checks::new();
    runner.fail = Some("normal".into());
    let manifest = fs::read(r.0.join("pkg/Cargo.toml")).unwrap();
    let lock = fs::read(r.0.join("pkg/Cargo.lock")).unwrap();
    let q = request(&r.0, "patch", false, &runner);
    let a = close(&r.0, &q, &runner);
    assert!(a.stderr.contains("rolled back"), "{}", a.stderr);
    assert_eq!(fs::read(r.0.join("pkg/Cargo.toml")).unwrap(), manifest);
    assert_eq!(fs::read(r.0.join("pkg/Cargo.lock")).unwrap(), lock);
    assert!(!r.0.join(development::RECORD).exists());
    assert!(
        !r.0.join(".runtime/bridgeforge-codex/develop-transaction.json")
            .exists()
    );
}
#[test]
fn package_development_rejects_source_and_inventory_drift_preserving_external_edits() {
    for action in ["mutate", "new-file"] {
        let r = fixture("independent", true);
        configure(&r.0, false);
        change(&r.0, true);
        let mut runner = Checks::new();
        runner.action = Some(action.into());
        let q = request(&r.0, "patch", false, &runner);
        let a = close(&r.0, &q, &runner);
        assert_ne!(a.code, 0);
        assert!(a.stderr.contains("rolled back"), "{}", a.stderr);
        if action == "mutate" {
            assert_eq!(
                fs::read(r.0.join("pkg/src/lib.rs")).unwrap(),
                b"external edit\n"
            );
        } else {
            assert!(r.0.join("pkg/src/new.rs").exists());
        }
    }
}
#[test]
fn package_development_rejects_artifact_drift_and_manual_version_edit() {
    let r = fixture("independent", true);
    configure(&r.0, false);
    change(&r.0, true);
    let mut cfg: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    cfg["development"]["a"]["checks"] = json!([
        {"id":"first","program":"fixture-check","args":["first"],"artifacts":[".runtime/result.bin"]},
        {"id":"second","program":"fixture-check","args":["second"]}]);
    write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
    let mut runner = Checks::new();
    runner.action = Some("artifact-drift".into());
    let q = request(&r.0, "patch", false, &runner);
    assert!(
        close(&r.0, &q, &runner)
            .stderr
            .contains("final artifact changed")
    );
    let bytes = fs::read_to_string(r.0.join("pkg/Cargo.toml"))
        .unwrap()
        .replace("1.0.0", "1.0.8");
    write(&r.0, "pkg/Cargo.toml", bytes.as_bytes());
    assert_ne!(
        development::preview(&r.0, &Request::default(), &runner).code,
        0
    );
}
#[test]
fn package_development_check_policy_change_does_not_reopen_version() {
    let r = fixture("independent", true);
    configure(&r.0, false);
    change(&r.0, true);
    let runner = Checks::new();
    let q = request(&r.0, "patch", false, &runner);
    let a = close(&r.0, &q, &runner);
    assert_eq!(a.code, 0, "{}", a.stderr);
    let before = fs::read(r.0.join(development::RECORD)).unwrap();
    let mut cfg: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    cfg["development"]["a"]["checks"][0]["timeout_seconds"] = json!(30);
    write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
    let q = request(&r.0, "patch", false, &runner);
    let a = close(&r.0, &q, &runner);
    assert_eq!(a.code, 0, "{}", a.stderr);
    assert_eq!(a.receipt.unwrap()["status"], "nothing-to-close");
    assert_eq!(fs::read(r.0.join(development::RECORD)).unwrap(), before);
}
#[test]
fn package_development_real_cargo_verifies_final_manifest_and_lock() {
    let r = fixture("independent", true);
    configure(&r.0, false);
    change(&r.0, true);
    write(&r.0, "pkg/src/lib.rs", b"pub fn value()->u8{2}\n#[test] fn final_candidate(){assert_eq!(value(),2);assert_eq!(env!(\"CARGO_PKG_VERSION\"),\"1.0.1\");}\n");
    let mut cfg: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    cfg["development"]["a"]["checks"] = json!([{"id":"cargo-test","program":"cargo","args":["test","--locked","--manifest-path","pkg/Cargo.toml","--target-dir",".runtime/target"]}]);
    write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
    let q = request(&r.0, "patch", false, &SystemProcessRunner);
    let a = close(&r.0, &q, &SystemProcessRunner);
    assert_eq!(a.code, 0, "{}", a.stderr);
    assert!(
        fs::read_to_string(r.0.join("pkg/Cargo.lock"))
            .unwrap()
            .contains("1.0.1")
    );
}

pub(super) fn python_pair(pinned: bool) -> Repo {
    let r = fixture("unconfigured", false);
    let mut cfg: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    cfg["adapters"]["python"]["projects"] =
        json!(["pkg/pyproject.toml", "consumer/pyproject.toml"]);
    cfg["adapters"]["python"]["local_sources"] =
        json!({"consumer/pyproject.toml":{"a":"pkg/pyproject.toml"}});
    cfg["packages"].as_array_mut().unwrap().push(
        json!({"id":"b","name":"b","language":"python","manifest":"consumer/pyproject.toml"}),
    );
    write(
        &r.0,
        "consumer/pyproject.toml",
        format!(
            "[project]\nname='b'\nversion='1.0.0'\ndependencies=['a{}1.0.0']\n",
            if pinned { "==" } else { ">=" }
        )
        .as_bytes(),
    );
    write(&r.0, "consumer/b.py", b"VALUE=1\n");
    write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
    git(&r.0, &["add", "."]);
    git(&r.0, &["commit", "--amend", "--no-edit"]);
    migrate(&r.0);
    let mut cfg: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    cfg["development"] = json!({"a":policy(false),"b":policy(false)});
    cfg["python_environment"] = python_environment_fixture(&r.0);
    write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
    r
}
#[test]
fn package_development_existing_caller_bumps_only_for_requirements_or_public_interface() {
    for scenario in ["compatible-range", "pinned", "forwarded"] {
        let r = python_pair(scenario == "pinned");
        let runner = Checks::new();
        change(&r.0, false);
        write(&r.0, "consumer/b.py", b"VALUE=2\n");
        let q = request(&r.0, "patch", false, &runner);
        let a = close(&r.0, &q, &runner);
        assert_eq!(a.code, 0, "{}", a.stderr);
        write(&r.0, "pkg/a.py", b"VALUE=3\n");
        let q = request(&r.0, "patch", false, &runner);
        if scenario == "forwarded" {
            let mut analysis: Value =
                serde_json::from_slice(&fs::read(r.0.join(".runtime/develop.json")).unwrap())
                    .unwrap();
            analysis["interface_impacts"] =
                json!({"b":{"dependencies":["a"],"reason":"public re-export changed"}});
            analysis["packages"]["b"]["level"] = json!("minor");
            write(
                &r.0,
                ".runtime/develop.json",
                &serde_json::to_vec(&analysis).unwrap(),
            );
        }
        let a = close(&r.0, &q, &runner);
        assert_eq!(a.code, 0, "{}", a.stderr);
        let ledger = development::Ledger::load(&r.0).unwrap();
        assert_eq!(
            ledger.latest("b").unwrap().after.as_deref(),
            Some(match scenario {
                "pinned" => "1.0.2",
                "forwarded" => "1.1.0",
                _ => "1.0.1",
            })
        );
    }
}
#[test]
fn package_development_committed_closure_cannot_be_removed_or_rewritten() {
    let r = fixture("independent", true);
    configure(&r.0, false);
    change(&r.0, true);
    let runner = Checks::new();
    let q = request(&r.0, "patch", false, &runner);
    assert_eq!(close(&r.0, &q, &runner).code, 0);
    git(&r.0, &["add", "."]);
    git(&r.0, &["commit", "-m", "fix: closed development"]);
    let bytes = fs::read(r.0.join(development::RECORD)).unwrap();
    fs::remove_file(r.0.join(development::RECORD)).unwrap();
    let p = development::preview(&r.0, &Request::default(), &runner);
    assert!(p.stderr.contains("preserved verbatim"), "{}", p.stderr);
    let mut record: Value = serde_json::from_slice(&bytes).unwrap();
    record["entries"][0]["after"] = json!("1.0.8");
    write(
        &r.0,
        development::RECORD,
        &serde_json::to_vec(&record).unwrap(),
    );
    assert!(
        development::preview(&r.0, &Request::default(), &runner)
            .stderr
            .contains("invalid closure chain")
    );
}
#[test]
fn package_development_and_sync_share_the_same_lock() {
    let r = fixture("independent", true);
    configure(&r.0, false);
    change(&r.0, true);
    let runner = Checks::new();
    let q = request(&r.0, "patch", false, &runner);
    let p = development::preview(&r.0, &q, &runner);
    let _lock = crate::git_sync::lock_factory_runtime(&r.0, &runner).unwrap();
    let a = development::apply(
        &r.0,
        &q,
        p.receipt.unwrap()["fingerprint"].as_str().unwrap(),
        &runner,
    );
    assert!(
        a.stderr.contains("already running or lock is unavailable"),
        "{}",
        a.stderr
    );
    assert!(runner.calls.lock().unwrap().is_empty());
    assert!(
        development::recover(&r.0)
            .stderr
            .contains("already running or lock is unavailable")
    );
}
#[test]
fn package_development_stale_analysis_and_empty_checks_fail_closed() {
    let r = zero_fixture();
    configure(&r.0, false);
    change(&r.0, true);
    let runner = Checks::new();
    let q = request(&r.0, "patch", false, &runner);
    write(&r.0, "pkg/src/lib.rs", b"pub fn value()->u8{3}\n");
    assert!(
        development::preview(&r.0, &q, &runner)
            .stderr
            .contains("stale")
    );
    let mut cfg: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    cfg["development"]["a"]["checks"] = json!([]);
    write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
    assert!(
        development::preview(&r.0, &Request::default(), &runner)
            .stderr
            .contains("nonempty mandatory")
    );
}
#[test]
fn package_development_graduates_each_package_independently() {
    let r = fixture("unconfigured", true);
    write(
        &r.0,
        "pkg/Cargo.toml",
        b"[package]\nname='a'\nversion='0.3.0'\nedition='2021'\n[workspace]\n",
    );
    write(
        &r.0,
        "pkg/Cargo.lock",
        b"version=4\n[[package]]\nname='a'\nversion='0.3.0'\n",
    );
    write(
        &r.0,
        "second/Cargo.toml",
        b"[package]\nname='b'\nversion='0.2.0'\nedition='2021'\n[workspace]\n",
    );
    write(
        &r.0,
        "second/Cargo.lock",
        b"version=4\n[[package]]\nname='b'\nversion='0.2.0'\n",
    );
    write(&r.0, "second/src/lib.rs", b"pub fn b(){}\n");
    let mut cfg: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    cfg["adapters"]["rust"]["workspaces"]
        .as_array_mut()
        .unwrap()
        .push(json!("second/Cargo.toml"));
    cfg["packages"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"b","name":"b","language":"rust","manifest":"second/Cargo.toml"}));
    cfg["product_inputs"]
        .as_array_mut()
        .unwrap()
        .push(json!("second/Cargo.lock"));
    write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
    git(&r.0, &["add", "."]);
    git(&r.0, &["commit", "--amend", "--no-edit"]);
    migrate(&r.0);
    let mut cfg: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    cfg["development"] = json!({"a":policy(true),"b":policy(true)});
    cfg["development"]["b"]["stability"]["checks"][1]["args"] = json!(["fail-b"]);
    write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
    change(&r.0, true);
    write(&r.0, "second/src/lib.rs", b"pub fn b()->u8{2}\n");
    let mut runner = Checks::new();
    runner.fail = Some("fail-b".into());
    let q = request(&r.0, "patch", false, &runner);
    let a = close(&r.0, &q, &runner);
    assert_eq!(a.code, 0, "{}", a.stderr);
    let ledger = development::Ledger::load(&r.0).unwrap();
    assert_eq!(ledger.latest("a").unwrap().after.as_deref(), Some("1.0.0"));
    assert_eq!(ledger.latest("b").unwrap().after.as_deref(), Some("0.2.1"));
}

#[test]
fn package_development_recovers_interrupted_candidate_and_preserves_external_conflict() {
    for conflict in [false, true] {
        let r = fixture("independent", true);
        configure(&r.0, false);
        change(&r.0, true);
        let mut runner = Checks::new();
        runner.action = Some("crash".into());
        let before = fs::read(r.0.join("pkg/Cargo.toml")).unwrap();
        let q = request(&r.0, "patch", false, &runner);
        let interrupted =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| close(&r.0, &q, &runner)));
        assert!(interrupted.is_err());
        let head = git(&r.0, &["rev-parse", "HEAD"]);
        let index = fs::read(r.0.join(".git/index")).unwrap();
        assert!(
            development::require_completed_transaction(&r.0)
                .unwrap_err()
                .contains("develop --recover")
        );
        assert_eq!(git(&r.0, &["rev-parse", "HEAD"]), head);
        assert_eq!(fs::read(r.0.join(".git/index")).unwrap(), index);
        assert!(!r.0.join(".git/FETCH_HEAD").exists());
        assert!(
            development::preview(&r.0, &Request::default(), &runner)
                .stderr
                .contains("unfinished")
        );
        if conflict {
            write(&r.0, "pkg/Cargo.toml", b"external manifest edit\n");
        }
        let recovered = development::recover(&r.0);
        if conflict {
            assert!(
                recovered.stderr.contains("preserved external edits"),
                "{}",
                recovered.stderr
            );
            assert_eq!(
                fs::read(r.0.join("pkg/Cargo.toml")).unwrap(),
                b"external manifest edit\n"
            );
        } else {
            assert_eq!(recovered.code, 0, "{}", recovered.stderr);
            assert_eq!(fs::read(r.0.join("pkg/Cargo.toml")).unwrap(), before);
            assert!(!r.0.join(development::RECORD).exists());
        }
    }
}

#[test]
fn package_development_unrelated_documents_do_not_invalidate_closed_package() {
    let r = fixture("independent", true);
    configure(&r.0, false);
    change(&r.0, true);
    let runner = Checks::new();
    let q = request(&r.0, "patch", false, &runner);
    let a = close(&r.0, &q, &runner);
    assert_eq!(a.code, 0, "{}", a.stderr);
    git(&r.0, &["add", "."]);
    git(&r.0, &["commit", "-m", "fix: closed package"]);
    write(&r.0, "README.md", b"unrelated product documentation\n");
    let result = development::preview(&r.0, &Request::default(), &runner);
    assert_eq!(result.code, 0, "{}", result.stderr);
    assert_eq!(result.receipt.unwrap()["status"], "nothing-to-close");
    let q = release_request(&r.0, "patch");
    assert!(build(&r.0, "fix: publish", &q, &runner).is_ok());
}
