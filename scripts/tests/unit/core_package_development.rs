//! Lifecycle tests use isolated Git repositories; no real downstream writes.
use super::*;
use std::sync::Mutex;

#[test]
fn package_development_release_reports_more_than_three_changed_packages() {
    let r = fixture("legacy", true);
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
    let r = fixture("legacy", true);
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
    let r = fixture("legacy", true);
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
        if request.program == "fixture-check" {
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
    let r = fixture("legacy", true);
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
    for id in v["affected_packages"].as_array().unwrap() {
        let mut a = json!({"level":level,"reason":"fixture change","compatibility":if breaking{"breaking"}else{"compatible"}});
        if breaking {
            a["acceptance"] = json!({"fingerprint":v["fingerprint"],"scope":[id],"reason":"authorized breaking fixture","source":"isolated test authorization"});
        }
        packages.insert(id.as_str().unwrap().into(), a);
    }
    write(
        root,
        ".runtime/develop.json",
        &serde_json::to_vec(
            &json!({"schema":1,"fingerprint":v["fingerprint"],"packages":packages}),
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
fn package_development_configuration_change_revalidates_without_bump() {
    let r = fixture("independent", true);
    configure(&r.0, false);
    change(&r.0, true);
    let runner = Checks::new();
    let q = request(&r.0, "patch", false, &runner);
    let a = close(&r.0, &q, &runner);
    assert_eq!(a.code, 0, "{}", a.stderr);
    let mut cfg: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    cfg["development"]["a"]["checks"][0]["timeout_seconds"] = json!(30);
    write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
    let q = request(&r.0, "patch", false, &runner);
    let a = close(&r.0, &q, &runner);
    assert_eq!(a.code, 0, "{}", a.stderr);
    let change = &a.receipt.unwrap()["package_changes"][0];
    assert_eq!(change["version_before"], "1.0.1");
    assert_eq!(change["version_after"], "1.0.1");
}
#[test]
fn package_development_real_cargo_verifies_final_manifest_and_lock() {
    let r = fixture("independent", true);
    configure(&r.0, false);
    change(&r.0, true);
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
    let r = fixture("legacy", false);
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
    let r = fixture("legacy", true);
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
        let synced = crate::git_sync::sync(
            &r.0,
            &SystemProcessRunner,
            crate::git_sync::GitSyncOptions::default(),
        );
        assert!(
            synced.stderr.contains("develop --recover"),
            "{}",
            synced.stderr
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
