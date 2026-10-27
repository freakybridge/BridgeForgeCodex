//! Real Git proves EOL equivalence; no downstream repository is mutated.
use super::*;

// Complete legacy schema-3 fixture. Keep its eight-part integrity contract;
// merely changing a new three-part record's schema would create invalid data.
fn schema3_parts(root: &Path, runner: &dyn ProcessRunner, entry: &mut development::Entry) {
    let cfg = load(root).unwrap().unwrap();
    let captured = capture(root, &cfg, runner).unwrap();
    let parts = entry.evidence_components.as_mut().unwrap();
    parts.remove("normalization");
    parts.insert("environment".into(), captured.environment);
    parts.insert(
        "policy".into(),
        digest(&serde_json::to_vec(&cfg.development.get(&entry.id)).unwrap()),
    );
    parts.insert("contracts".into(), digest(b"{}"));
    let check_inputs = BTreeMap::from([(
        "scripts/check.ps1",
        Some(digest(&fs::read(root.join("scripts/check.ps1")).unwrap())),
    )]);
    parts.insert(
        "check_inputs".into(),
        digest(&serde_json::to_vec(&check_inputs).unwrap()),
    );
    parts.insert(
        "adapters".into(),
        digest(&serde_json::to_vec(&format!("{:?}", cfg.native.adapters)).unwrap()),
    );
    entry.evidence_schema = Some(3);
}

fn eol_fixture(crlf: bool) -> (Repo, Checks) {
    let r = zero_fixture();
    configure(&r.0, false);
    let mut cfg: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    cfg["product_inputs"]
        .as_array_mut()
        .unwrap()
        .extend([json!(".gitattributes"), json!("scripts/check.ps1")]);
    cfg["build_inputs"] = json!([".runtime/declared.txt"]);
    cfg["development"]["a"]["checks"][0]["inputs"] = json!(["scripts/check.ps1"]);
    write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
    write(
        &r.0,
        ".gitattributes",
        b"* text=auto eol=lf\n*.ps1 text eol=crlf\n*.bin -text\n",
    );
    write(
        &r.0,
        "pkg/src/lib.rs",
        if crlf {
            b"pub fn value()->u8{2}\r\n"
        } else {
            b"pub fn value()->u8{2}\n"
        },
    );
    write(
        &r.0,
        "scripts/check.ps1",
        if crlf {
            b"Write-Output ok\r\n"
        } else {
            b"Write-Output ok\n"
        },
    );
    write(
        &r.0,
        ".runtime/declared.txt",
        if crlf { b"declared\r\n" } else { b"declared\n" },
    );
    let runner = Checks::new();
    let q = request(&r.0, "patch", false, &runner);
    let result = close(&r.0, &q, &runner);
    assert_eq!(result.code, 0, "{}", result.stderr);
    (r, runner)
}

#[test]
fn eol_closure_reuses_both_directions_and_rejects_real_changes_without_writes() {
    for crlf in [false, true] {
        let (r, runner) = eol_fixture(crlf);
        let ledger = fs::read(r.0.join(development::RECORD)).unwrap();
        let manifest = fs::read(r.0.join("pkg/Cargo.toml")).unwrap();
        let index = fs::read(r.0.join(".git/index")).unwrap();
        write(
            &r.0,
            "pkg/src/lib.rs",
            if crlf {
                b"pub fn value()->u8{2}\n"
            } else {
                b"pub fn value()->u8{2}\r\n"
            },
        );
        write(
            &r.0,
            "scripts/check.ps1",
            if crlf {
                b"Write-Output ok\n"
            } else {
                b"Write-Output ok\r\n"
            },
        );
        write(
            &r.0,
            ".runtime/declared.txt",
            if crlf { b"declared\n" } else { b"declared\r\n" },
        );
        let p = development::preview(&r.0, &Request::default(), &runner);
        assert_eq!(p.code, 0, "{}", p.stderr);
        assert_eq!(p.receipt.unwrap()["status"], "nothing-to-close");
        let q = release_request(&r.0, "patch");
        assert!(build(&r.0, "fix: EOL", &q, &runner).unwrap().is_some());
        assert_eq!(fs::read(r.0.join(development::RECORD)).unwrap(), ledger);
        assert_eq!(fs::read(r.0.join("pkg/Cargo.toml")).unwrap(), manifest);
        assert_eq!(fs::read(r.0.join(".git/index")).unwrap(), index);
        write(&r.0, "pkg/src/lib.rs", b"pub fn value()->u8{3}\r\n");
        assert!(
            preview(&r.0, "fix: real edit", &Request::default(), &runner)
                .stderr
                .contains("package content changed")
        );
    }
}

#[test]
fn eol_schema3_proof_is_reused_without_rewriting_or_bumping() {
    let (r, runner) = eol_fixture(false);
    let mut ledger = development::Ledger::load(&r.0).unwrap();
    let entry = ledger.entries.last_mut().unwrap();
    schema3_parts(&r.0, &runner, entry);
    entry.evidence =
        digest(&serde_json::to_vec(entry.evidence_components.as_ref().unwrap()).unwrap());
    entry.hash.clear();
    entry.hash = digest(&serde_json::to_vec(entry).unwrap());
    let bytes = serde_json::to_vec(&ledger).unwrap();
    write(&r.0, development::RECORD, &bytes);
    let analysis = release_request(&r.0, "patch");
    write(
        &r.0,
        ".git/info/attributes",
        b"pkg/src/lib.rs text eol=crlf\n",
    );
    let error = build(&r.0, "fix: stale rules", &analysis, &runner).unwrap_err();
    assert!(
        error.contains("stale") || error.contains("fingerprint"),
        "{error}"
    );
    fs::remove_file(r.0.join(".git/info/attributes")).unwrap();
    write(&r.0, "pkg/src/lib.rs", b"pub fn value()->u8{2}\r\n");
    let p = development::preview(&r.0, &Request::default(), &runner);
    assert_eq!(p.code, 0, "{}", p.stderr);
    assert_eq!(p.receipt.unwrap()["status"], "nothing-to-close");
    assert!(
        build(
            &r.0,
            "fix: legacy EOL",
            &release_request(&r.0, "patch"),
            &runner
        )
        .unwrap()
        .is_some()
    );
    assert_eq!(fs::read(r.0.join(development::RECORD)).unwrap(), bytes);
    assert_eq!(
        development::Ledger::load(&r.0)
            .unwrap()
            .latest("a")
            .unwrap()
            .after
            .as_deref(),
        Some("0.3.1")
    );
}

#[test]
fn eol_old_crlf_certificate_without_source_diff_cannot_invent_a_bump() {
    let (r, runner) = eol_fixture(false);
    write(&r.0, "pkg/src/lib.rs", b"pub fn value()->u8{2}\r\n");
    let cfg = load(&r.0).unwrap().unwrap();
    let captured = capture(&r.0, &cfg, &runner).unwrap();
    let mut ledger = development::Ledger::load(&r.0).unwrap();
    let entry = ledger.entries.last_mut().unwrap();
    // Independently reconstruct the old raw-byte content serializer for this
    // standalone package: manifest and source are the only content members.
    let owned = ["pkg/Cargo.toml", "pkg/src/lib.rs"]
        .into_iter()
        .map(|path| {
            (
                path,
                json!({"hash":digest(&fs::read(r.0.join(path)).unwrap()),"mode":"100644"}),
            )
        })
        .collect::<BTreeMap<_, _>>();
    entry.content = digest(
        &serde_json::to_vec(&json!({"package":captured.current.packages["a"],"files":owned}))
            .unwrap(),
    );
    schema3_parts(&r.0, &runner, entry);
    let parts = entry.evidence_components.as_mut().unwrap();
    parts.remove("normalization");
    parts.insert("content".into(), entry.content.clone());
    parts.insert(
        "dependencies".into(),
        digest(&serde_json::to_vec(&BTreeMap::from([("a", &entry.content)])).unwrap()),
    );
    entry.evidence = digest(&serde_json::to_vec(parts).unwrap());
    entry.hash.clear();
    entry.hash = digest(&serde_json::to_vec(entry).unwrap());
    write(
        &r.0,
        development::RECORD,
        &serde_json::to_vec(&ledger).unwrap(),
    );
    let plan = build(
        &r.0,
        "fix: old CRLF release",
        &release_request(&r.0, "patch"),
        &runner,
    )
    .unwrap()
    .unwrap();
    for (path, bytes) in plan.writes {
        fs::write(path, bytes).unwrap();
    }
    git(&r.0, &["add", "."]);
    git(&r.0, &["commit", "-m", "published old CRLF certificate"]);
    write(&r.0, "pkg/src/lib.rs", b"pub fn value()->u8{2}\n");
    let manifest = fs::read(r.0.join("pkg/Cargo.toml")).unwrap();
    let proof = fs::read(r.0.join(development::RECORD)).unwrap();
    let q = request(&r.0, "patch", false, &runner);
    let p = close(&r.0, &q, &runner);
    assert_ne!(p.code, 0);
    assert!(
        p.stderr
            .contains("legacy content representation cannot be proven"),
        "{}",
        p.stderr
    );
    assert_eq!(fs::read(r.0.join("pkg/Cargo.toml")).unwrap(), manifest);
    assert_eq!(fs::read(r.0.join(development::RECORD)).unwrap(), proof);
}

#[test]
fn eol_rules_and_raw_transaction_drift_remain_blocking() {
    let (r, runner) = eol_fixture(false);
    let q = release_request(&r.0, "patch");
    let plan = build(&r.0, "fix: EOL", &q, &runner).unwrap().unwrap();
    write(&r.0, "pkg/src/lib.rs", b"pub fn value()->u8{2}\r\n");
    assert!(
        apply_file_release_plan(&plan)
            .unwrap_err()
            .contains("changed concurrently")
    );
    write(&r.0, "pkg/src/lib.rs", b"pub fn value()->u8{2}\n");
    write(&r.0, ".git/info/attributes", b"pkg/src/lib.rs -text\n");
    assert!(
        apply_file_release_plan(&plan)
            .unwrap_err()
            .contains("normalization rules changed")
    );
    let p = development::preview(&r.0, &Request::default(), &runner);
    assert_eq!(p.code, 0, "{}", p.stderr);
    assert_eq!(p.receipt.unwrap()["status"], "nothing-to-close");
    // A rules-only edit does not change already-LF version content, while the
    // same CRLF bytes under -text are now a real content change.
    write(&r.0, "pkg/src/lib.rs", b"pub fn value()->u8{2}\r\n");
    let p = development::preview(&r.0, &Request::default(), &runner);
    assert!(
        p.receipt.unwrap()["pending_reasons"]["a"]
            .as_str()
            .unwrap()
            .contains("content")
    );
}

#[test]
fn eol_binary_and_special_attributes_stay_exact_and_ignored_filters_are_rejected() {
    let r = zero_fixture();
    write(
        &r.0,
        ".gitattributes",
        b"* text=auto eol=lf\n*.bin -text\n*.id ident\n*.enc working-tree-encoding=UTF-16LE\n",
    );
    let paths = ["a.txt", "a.bin", "a.id", "a.enc", "binary.txt"].map(String::from);
    let rules = content_identity::Rules::capture(&r.0, paths, &SystemProcessRunner).unwrap();
    let identity = content_identity::Identity::new(&r.0, rules, &SystemProcessRunner);
    assert_eq!(
        identity.bytes("a.txt", b"one\r\ntwo\r\n").unwrap(),
        b"one\ntwo\n"
    );
    for path in ["a.bin", "a.id", "a.enc"] {
        assert_eq!(identity.bytes(path, b"one\r\n").unwrap(), b"one\r\n");
    }
    assert_eq!(
        identity.bytes("binary.txt", b"one\0\r\n").unwrap(),
        b"one\0\r\n"
    );
    write(&r.0, "a.txt", b"one\r\ntwo\r\n");
    write(&r.0, "binary.txt", b"one\r\ntwo\rthree");
    let inputs = BTreeMap::from([
        (r.0.join("a.txt"), Some(b"one\r\ntwo\r\n".to_vec())),
        (r.0.join("binary.txt"), Some(b"one\r\ntwo\rthree".to_vec())),
    ]);
    identity.prepare(&inputs).unwrap();
    assert_eq!(
        identity.bytes("a.txt", b"one\r\ntwo\r\n").unwrap(),
        b"one\ntwo\n"
    );
    assert_eq!(
        identity.bytes("binary.txt", b"one\r\ntwo\rthree").unwrap(),
        b"one\r\ntwo\rthree"
    );
    write(
        &r.0,
        ".git/info/attributes",
        b".runtime/ignored filter=unsafe\n",
    );
    assert!(
        content_identity::Rules::capture(&r.0, [".runtime/ignored".into()], &SystemProcessRunner)
            .unwrap_err()
            .contains("external Git filter")
    );
    write(
        &r.0,
        ".git/info/attributes",
        b".runtime/ignored filter=unspecified\n",
    );
    git(
        &r.0,
        &[
            "config",
            "filter.unspecified.clean",
            "echo unsafe > .runtime/filter-ran",
        ],
    );
    assert!(
        content_identity::Rules::capture(&r.0, [".runtime/ignored".into()], &SystemProcessRunner)
            .unwrap_err()
            .contains("external Git filter")
    );
    assert!(!r.0.join(".runtime/filter-ran").exists());
}
