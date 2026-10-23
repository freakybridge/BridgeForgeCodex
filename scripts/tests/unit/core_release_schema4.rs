use super::*;
use crate::SystemProcessRunner;
use std::process::Command;

struct Repo(PathBuf);
impl Drop for Repo {
    fn drop(&mut self) {
        if self
            .0
            .file_name()
            .is_some_and(|n| n.to_string_lossy().starts_with("bf-schema4-"))
            && self.0.parent() == Some(std::env::temp_dir().as_path())
        {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
}
fn git(root: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap().trim().into()
}
fn write(root: &Path, path: &str, bytes: &[u8]) {
    fs::create_dir_all(root.join(path).parent().unwrap()).unwrap();
    fs::write(root.join(path), bytes).unwrap();
}
fn fixture(strategy: &str, rust: bool) -> Repo {
    let root = std::env::temp_dir().join(format!(
        "bf-schema4-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&root).unwrap();
    git(&root, &["init"]);
    git(&root, &["config", "user.name", "Schema4 Test"]);
    git(&root, &["config", "user.email", "schema4@example.invalid"]);
    write(&root, "VERSION", b"1.0.0\n");
    write(&root, "CHANGELOG.md", b"# Changelog\n");
    write(&root, "README.md", b"baseline\n");
    write(&root, ".gitignore", b".runtime/\n");
    write(
        &root,
        ".codex/managed-skeleton.json",
        br#"{"schema_version":1,"assets":[]}"#,
    );
    let cfg = if rust {
        write(
            &root,
            "pkg/Cargo.toml",
            b"[package]\nname='a'\nversion='1.0.0'\nedition='2021'\n[workspace]\n",
        );
        write(&root, "pkg/src/lib.rs", b"pub fn value()->u8{1}\n");
        write(
            &root,
            "pkg/Cargo.lock",
            b"version=4\n[[package]]\nname='a'\nversion='1.0.0'\n",
        );
        json!({"schema_version":3,"release_policy":"explicit_release","strategy":"independent","adapters":{"rust":{"workspaces":["pkg/Cargo.toml"]}},"packages":[{"id":"a","language":"rust","manifest":"pkg/Cargo.toml","name":"a"}],"product_inputs":["README.md",".gitignore","pkg/Cargo.lock"]})
    } else {
        write(
            &root,
            "pkg/pyproject.toml",
            b"[project]\nname='a'\nversion='1.0.0'\n",
        );
        write(&root, "pkg/a.py", b"VALUE=1\n");
        json!({"schema_version":3,"release_policy":"explicit_release","strategy":"independent","adapters":{"python":{"projects":["pkg/pyproject.toml"]}},"packages":[{"id":"a","language":"python","manifest":"pkg/pyproject.toml","name":"a"}],"product_inputs":["README.md",".gitignore"]})
    };
    write(&root, CONFIG, &serde_json::to_vec(&cfg).unwrap());
    git(&root, &["add", "."]);
    git(&root, &["commit", "-m", "chore: baseline"]);
    if strategy != "legacy" {
        let proposal = if strategy == "synchronized" {
            let mut v = cfg.clone();
            v["schema_version"] = json!(4);
            v["strategy"] = json!(strategy);
            write(
                &root,
                ".runtime/proposal.json",
                &serde_json::to_vec(&v).unwrap(),
            );
            Some(".runtime/proposal.json")
        } else {
            None
        };
        let planned = migration_plan(&root, &SystemProcessRunner, proposal);
        assert_eq!(planned.code, 0, "{}", planned.stderr);
        let fingerprint = planned.receipt.unwrap()["fingerprint"]
            .as_str()
            .unwrap()
            .to_string();
        let applied = migration_apply(&root, &SystemProcessRunner, proposal, &fingerprint, true);
        assert_eq!(applied.code, 0, "{}", applied.stderr);
    }
    Repo(root)
}
fn change(root: &Path, rust: bool) {
    write(
        root,
        if rust { "pkg/src/lib.rs" } else { "pkg/a.py" },
        if rust {
            b"pub fn value()->u8{2}\n"
        } else {
            b"VALUE=2\n"
        },
    );
}

fn external_cargo_fixture() -> Repo {
    let r = fixture("legacy", true);
    write(&r.0, "pkg/Cargo.toml", b"[package]\nname='a'\nversion='1.0.0'\nedition='2021'\nbuild='../external/build.rs'\n[workspace]\n[lib]\npath='../external/lib.rs'\n[[bin]]\nname='inventory'\npath='../external/all_inventory.rs'\n[[test]]\nname='check'\npath='../external/check.rs'\n[[bench]]\nname='bench'\npath='../external/bench.rs'\n[[example]]\nname='example'\npath='../external/example.rs'\n");
    for name in ["lib", "build", "all_inventory", "check", "bench", "example"] {
        write(&r.0, &format!("external/{name}.rs"), b"fn main() {}\n");
    }
    git(&r.0, &["add", "."]);
    git(&r.0, &["commit", "--amend", "--no-edit"]);
    let p = migration_plan(&r.0, &SystemProcessRunner, None);
    assert_eq!(p.code, 0, "{}", p.stderr);
    let v = p.receipt.unwrap();
    let a = migration_apply(&r.0, &SystemProcessRunner, None, v["fingerprint"].as_str().unwrap(), true);
    assert_eq!(a.code, 0, "{}", a.stderr);
    r
}

fn legacy_source_roots(root: &Path) -> Value {
    let mut v: Value = serde_json::from_slice(&fs::read(root.join(CONFIG)).unwrap()).unwrap();
    v["baseline"]["packages"]["a"]["source_roots"] = json!(["pkg/"]);
    v["baseline"]["input_hashes"].as_object_mut().unwrap().retain(|path, _| !path.starts_with("external/"));
    write(root, CONFIG, &serde_json::to_vec(&v).unwrap());
    v
}

fn historical_sharing_fixture() -> Repo {
    let r = fixture("legacy", true);
    write(&r.0, "pkg/src/gui.rs", b"pub fn gui() {}\n");
    write(&r.0, "pkg/src/helper.rs", b"pub fn helper() {}\n");
    write(&r.0, "tools/Cargo.toml", b"[package]\nname='b'\nversion='1.0.0'\nedition='2021'\n[workspace]\n[[test]]\nname='shared'\npath='../checks/main.rs'\n");
    write(&r.0, "tools/Cargo.lock", b"version=4\n[[package]]\nname='b'\nversion='1.0.0'\n");
    write(&r.0, "checks/main.rs", b"#[path=\"../pkg/src/gui.rs\"] mod gui;\n#[path=\"../pkg/src/helper.rs\"] mod helper;\n");
    let mut cfg: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    cfg["adapters"]["rust"]["workspaces"].as_array_mut().unwrap().push(json!("tools/Cargo.toml"));
    cfg["packages"].as_array_mut().unwrap().push(json!({"id":"b","name":"b","language":"rust","manifest":"tools/Cargo.toml"}));
    cfg["product_inputs"].as_array_mut().unwrap().push(json!("tools/Cargo.lock"));
    write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
    git(&r.0, &["add", "."]);
    git(&r.0, &["commit", "--amend", "--no-edit"]);
    let p = migration_plan(&r.0, &SystemProcessRunner, None);
    assert_eq!(p.code, 0, "{}", p.stderr);
    let v = p.receipt.unwrap();
    assert_eq!(v["ownership_analysis"]["shared_input_additions"]["historical"].as_array().unwrap().len(), 2);
    assert_eq!(migration_apply(&r.0, &SystemProcessRunner, None, v["fingerprint"].as_str().unwrap(), true).code, 0);
    let migrated = load(&r.0).unwrap().unwrap();
    let captured = capture(&r.0, &migrated, &SystemProcessRunner).unwrap();
    assert!(!captured.changes.iter().any(|c| c.path == CONFIG), "inferred sharing alone is migration bookkeeping");
    let mut changed: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    changed["build_inputs"] = json!(["scripts/recipe.json"]);
    assert!(!migration::is_initial_configuration(&changed, &migrated, &captured.current).unwrap(), "sharing must not conceal other configuration changes");
    // Simulate an existing schema-4 migration that already repaired file roots,
    // but predates the shared-source correction (the reported downstream case).
    let mut v: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    v["baseline"]["shared_inputs"] = json!([]);
    v["shared_inputs"] = json!([]);
    write(&r.0, CONFIG, &serde_json::to_vec(&v).unwrap());
    git(&r.0, &["add", "."]);
    git(&r.0, &["commit", "-m", "chore: old schema4 sharing gap"]);
    r
}

#[test]
fn schema4_baseline_repair_sharing_is_snapshot_bound_atomic_and_release_ready() {
    let r = historical_sharing_fixture();
    let root = &r.0;
    let before = fs::read(root.join(CONFIG)).unwrap();
    let original: Value = serde_json::from_slice(&before).unwrap();
    let index = fs::read(root.join(".git/index")).unwrap();
    let head = git(root, &["rev-parse", "HEAD"]);
    // The current tool no longer consumes helper.rs. History must still show it.
    write(root, "checks/main.rs", b"#[path=\"../pkg/src/gui.rs\"] mod gui;\n");
    let p = repair_baseline_plan(root, &SystemProcessRunner);
    assert_eq!(p.code, 0, "{}", p.stderr);
    let p = p.receipt.unwrap();
    let additions = &p["ownership_analysis"]["shared_input_additions"];
    assert_eq!(additions["historical"].as_array().unwrap().len(), 2);
    assert_eq!(additions["current"].as_array().unwrap().len(), 1);
    assert_eq!(additions["current"][0]["path"], "pkg/src/gui.rs");
    assert_eq!(additions["historical"][0]["packages"], json!(["a", "b"]));
    assert!(additions["historical"][0]["content_sha256"].as_str().unwrap().starts_with("sha256:"));
    assert_eq!(additions["historical"][0]["evidence"].as_array().unwrap().len(), 2);
    assert_eq!(fs::read(root.join(CONFIG)).unwrap(), before);
    let fp = p["fingerprint"].as_str().unwrap();
    assert_ne!(repair_baseline_apply(root, &SystemProcessRunner, fp, false).code, 0);
    write(root, "pkg/src/gui.rs", b"pub fn changed_after_plan() {}\n");
    assert_ne!(repair_baseline_apply(root, &SystemProcessRunner, fp, true).code, 0);
    assert_eq!(fs::read(root.join(CONFIG)).unwrap(), before);
    let p = repair_baseline_plan(root, &SystemProcessRunner).receipt.unwrap();
    let applied = repair_baseline_apply(root, &SystemProcessRunner, p["fingerprint"].as_str().unwrap(), true);
    assert_eq!(applied.code, 0, "{}", applied.stderr);
    let repaired: Value = serde_json::from_slice(&fs::read(root.join(CONFIG)).unwrap()).unwrap();
    let mut normalized = repaired.clone();
    normalized["baseline"]["shared_inputs"] = original["baseline"]["shared_inputs"].clone();
    normalized["shared_inputs"] = original["shared_inputs"].clone();
    assert_eq!(normalized, original, "only missing shared declarations may change");
    assert_eq!(fs::read(root.join(".git/index")).unwrap(), index);
    assert_eq!(git(root, &["rev-parse", "HEAD"]), head);
    assert_eq!(fs::read(root.join("VERSION")).unwrap(), b"1.0.0\n");
    let cfg = load(root).unwrap().unwrap();
    let c = capture(root, &cfg, &SystemProcessRunner).unwrap();
    assert_eq!(polyglot::owner(&c.baseline, "pkg/src/helper.rs", &cfg.native).unwrap(), vec!["a", "b"]);
    assert_eq!(polyglot::owner(&c.current, "pkg/src/helper.rs", &cfg.native).unwrap(), vec!["a"]);
    assert_eq!(c.changes.iter().find(|c| c.path == "pkg/src/gui.rs").unwrap().packages, vec!["a", "b"]);
    let request = analyze(root, "patch", Some("patch"));
    let mut analysis: Value = serde_json::from_slice(&fs::read(root.join(".runtime/analysis.json")).unwrap()).unwrap();
    analysis["packages"]["b"] = json!({"level":"patch","reason":"changed shared test source","compatibility":"compatible"});
    write(root, ".runtime/analysis.json", &serde_json::to_vec(&analysis).unwrap());
    let release = build(root, "fix: shared sources", &request, &SystemProcessRunner).unwrap().unwrap();
    assert_eq!(release.old_version.to_string(), "1.0.0");
    let second = repair_baseline_plan(root, &SystemProcessRunner).receipt.unwrap();
    assert_eq!(second["configuration"], repaired, "repair must be idempotent");
    apply_file_release_plan(&release).unwrap();
    git(root, &["add", "."]);
    git(root, &["commit", "-m", "fix: fixture release"]);
    let published = fs::read(root.join(CONFIG)).unwrap();
    assert!(repair_baseline_plan(root, &SystemProcessRunner).stderr.contains("published migration baseline cannot be repaired"));
    assert_eq!(fs::read(root.join(CONFIG)).unwrap(), published);
}

#[test]
fn schema4_baseline_repair_sharing_rejects_forged_and_broad_declarations() {
    let r = historical_sharing_fixture();
    let original: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    for mapping in [
        json!({"path":"pkg/src/gui.rs","packages":["a"]}),
        json!({"path":"pkg/src/","packages":["a","b"]}),
        json!({"path":"pkg/src/missing.rs","packages":["a","b"]}),
    ] {
        let mut forged = original.clone();
        forged["baseline"]["shared_inputs"] = json!([mapping]);
        let bytes = serde_json::to_vec(&forged).unwrap();
        write(&r.0, CONFIG, &bytes);
        assert_ne!(repair_baseline_plan(&r.0, &SystemProcessRunner).code, 0);
        assert_eq!(fs::read(r.0.join(CONFIG)).unwrap(), bytes);
    }
}

#[test]
fn schema4_baseline_repair_sharing_preserves_explicit_roles_and_opaque_boundaries() {
    let r = historical_sharing_fixture();
    let cfg = load(&r.0).unwrap().unwrap();
    let mut snapshot = polyglot::historical(&r.0, &cfg.baseline.revision, &cfg.native, &SystemProcessRunner).unwrap();
    let additions = polyglot::proven_shared_inputs(&snapshot, &cfg.native);
    assert_eq!(additions.len(), 2);
    snapshot.config.as_mut().unwrap().shared_inputs.push(polyglot::SharedInput { path:"pkg/src/".into(), packages:vec!["a".into()] });
    assert!(polyglot::proven_shared_inputs(&snapshot, &cfg.native).is_empty());
    snapshot.config.as_mut().unwrap().shared_inputs.clear();
    snapshot.config.as_mut().unwrap().product_inputs.push("pkg/src/".into());
    assert!(polyglot::proven_shared_inputs(&snapshot, &cfg.native).is_empty());
    snapshot.config.as_mut().unwrap().product_inputs.retain(|p| p != "pkg/src/");
    snapshot.inputs.remove("pkg/src/gui.rs");
    assert!(!polyglot::proven_shared_inputs(&snapshot, &cfg.native).iter().any(|m| m.path == "pkg/src/gui.rs"));
    snapshot.packages.get_mut("a").unwrap().language = Language::Python;
    assert!(polyglot::proven_shared_inputs(&snapshot, &cfg.native).is_empty());
    write(&r.0, "checks/main.rs", b"include!(env!(\"SHARED_SOURCE\"));\n");
    let current = polyglot::snapshot(&cfg.native, independent::inventory(&r.0, &SystemProcessRunner).unwrap(), false, &mut |p| config::read_current(&r.0, p)).unwrap();
    assert!(polyglot::proven_shared_inputs(&current, &cfg.native).is_empty(), "opaque includes must not invent consumers");
}

#[test]
fn schema4_external_cargo_targets_survive_moves_without_claiming_siblings() {
    let r = external_cargo_fixture();
    let cfg = load(&r.0).unwrap().unwrap();
    assert_eq!(cfg.baseline.packages["a"].source_roots.len(), 7);
    fs::rename(r.0.join("external/all_inventory.rs"), r.0.join("pkg/src/inventory.rs")).unwrap();
    let manifest = fs::read_to_string(r.0.join("pkg/Cargo.toml")).unwrap().replace("../external/all_inventory.rs", "src/inventory.rs");
    write(&r.0, "pkg/Cargo.toml", manifest.as_bytes());
    let c = capture(&r.0, &cfg, &SystemProcessRunner).unwrap();
    let old = c.changes.iter().find(|c| c.path == "external/all_inventory.rs").unwrap();
    assert_eq!(old.packages, vec!["a"]);
    write(&r.0, "external/unrelated.rs", b"fn unrelated() {}\n");
    let e = capture(&r.0, &cfg, &SystemProcessRunner).err().unwrap();
    assert!(e.contains("unattributed release input: external/unrelated.rs"), "{e}");
}

#[test]
fn schema4_opaque_include_does_not_claim_unknown_changed_file() {
    let r = external_cargo_fixture();
    let cfg = load(&r.0).unwrap().unwrap();
    write(&r.0, "external/lib.rs", b"include!(env!(\"SOURCE_FILE\"));\n");
    write(&r.0, "unknown/generated.rs", b"pub fn generated(){}\n");
    let error = capture(&r.0, &cfg, &SystemProcessRunner).err().unwrap();
    assert!(error.contains("unattributed release input: unknown/generated.rs"), "{error}");
}

#[test]
fn schema4_missing_external_target_keeps_only_declared_ownership_without_hash() {
    let r = external_cargo_fixture();
    let cfg = load(&r.0).unwrap().unwrap();
    let manifest = format!("{}\n[[test]]\nname='removed'\npath='../external/missing.rs'\n", fs::read_to_string(r.0.join("pkg/Cargo.toml")).unwrap());
    write(&r.0, "pkg/Cargo.toml", manifest.as_bytes());
    let c = capture(&r.0, &cfg, &SystemProcessRunner).unwrap();
    assert!(c.current.packages["a"].source_roots.contains(&"external/missing.rs".into()));
    assert!(!c.current.inputs.contains_key("external/missing.rs"));
    assert_eq!(polyglot::owner(&c.current, "external/missing.rs", &cfg.native).unwrap(), vec!["a"]);
    assert!(polyglot::owner(&c.current, "external/missing.rs/unrelated.txt", &cfg.native).unwrap_err().contains("unattributed"));
    write(&r.0, "external/missing.rs", b"mod restored;\n");
    write(&r.0, "external/restored.rs", b"pub fn restored() {}\n");
    let c = capture(&r.0, &cfg, &SystemProcessRunner).unwrap();
    assert!(c.current.packages["a"].source_roots.contains(&"external/restored.rs".into()));
    assert_eq!(c.current.inputs["external/missing.rs"], Some(b"mod restored;\n".to_vec()));
    assert!(c.current.inputs["external/restored.rs"].is_some());
}

#[test]
fn schema4_baseline_repair_is_proven_atomic_and_accepts_committed_migration() {
    let r = external_cargo_fixture();
    let old = legacy_source_roots(&r.0);
    git(&r.0, &["add", "."]);
    git(&r.0, &["commit", "-m", "chore: old migration"]);
    let head = git(&r.0, &["rev-parse", "HEAD"]);
    let index = fs::read(r.0.join(".git/index")).unwrap();
    let config = fs::read(r.0.join(CONFIG)).unwrap();
    let lock = fs::read(r.0.join("pkg/Cargo.lock")).unwrap();
    let blocked = preview(&r.0, "fix: release", &Request::default(), &SystemProcessRunner);
    assert!(blocked.stderr.contains("--repair-baseline-plan"), "{}", blocked.stderr);
    let p = repair_baseline_plan(&r.0, &SystemProcessRunner);
    assert_eq!(p.code, 0, "{}", p.stderr);
    let v = p.receipt.unwrap();
    assert_eq!(v["status"], "baseline-repair-planned");
    assert_eq!(fs::read(r.0.join(CONFIG)).unwrap(), config);
    assert_eq!(fs::read(r.0.join(".git/index")).unwrap(), index);
    let fp = v["fingerprint"].as_str().unwrap();
    assert_ne!(repair_baseline_apply(&r.0, &SystemProcessRunner, fp, false).code, 0);
    assert_eq!(fs::read(r.0.join(CONFIG)).unwrap(), config);
    // A previously missing static reference appearing after planning must
    // invalidate the inventory even though it has no old input hash.
    write(&r.0, "external/lib.rs", b"mod restored;\n");
    let missing_plan = repair_baseline_plan(&r.0, &SystemProcessRunner).receipt.unwrap();
    write(&r.0, "external/restored.rs", b"pub fn restored(){}\n");
    assert_ne!(repair_baseline_apply(&r.0, &SystemProcessRunner, missing_plan["fingerprint"].as_str().unwrap(), true).code, 0);
    assert_eq!(fs::read(r.0.join(CONFIG)).unwrap(), config);
    fs::remove_file(r.0.join("external/restored.rs")).unwrap();
    write(&r.0, "external/lib.rs", b"fn main() {}\n");
    write(&r.0, "README.md", b"changed after plan\n");
    assert_ne!(repair_baseline_apply(&r.0, &SystemProcessRunner, fp, true).code, 0);
    assert_eq!(fs::read(r.0.join(CONFIG)).unwrap(), config);
    let p = repair_baseline_plan(&r.0, &SystemProcessRunner).receipt.unwrap();
    let a = repair_baseline_apply(&r.0, &SystemProcessRunner, p["fingerprint"].as_str().unwrap(), true);
    assert_eq!(a.code, 0, "{}", a.stderr);
    let mut repaired: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    assert_eq!(repaired["baseline"]["packages"]["a"]["source_roots"].as_array().unwrap().len(), 7);
    repaired["baseline"]["packages"]["a"]["source_roots"] = json!(["pkg/"]);
    repaired["baseline"]["input_hashes"].as_object_mut().unwrap().retain(|path, _| !path.starts_with("external/"));
    assert_eq!(repaired, old);
    assert_eq!(git(&r.0, &["rev-parse", "HEAD"]), head);
    assert_eq!(fs::read(r.0.join(".git/index")).unwrap(), index);
    assert_eq!(fs::read(r.0.join("pkg/Cargo.lock")).unwrap(), lock);
    assert_eq!(fs::read(r.0.join("VERSION")).unwrap(), b"1.0.0\n");
    assert_eq!(preview(&r.0, "fix: release", &Request::default(), &SystemProcessRunner).code, 0);
    let request = analyze(&r.0, "patch", None);
    let release = build(&r.0, "fix: product", &request, &SystemProcessRunner).unwrap().unwrap();
    apply_file_release_plan(&release).unwrap();
    git(&r.0, &["add", "."]);
    git(&r.0, &["commit", "-m", "fix: product"]);
    let published = fs::read(r.0.join(CONFIG)).unwrap();
    let blocked = repair_baseline_plan(&r.0, &SystemProcessRunner);
    assert!(blocked.stderr.contains("published migration baseline cannot be repaired"), "{}", blocked.stderr);
    assert_eq!(fs::read(r.0.join(CONFIG)).unwrap(), published);
}

#[test]
fn schema4_baseline_repair_rejects_forged_facts_and_ownership() {
    for field in ["version", "source_roots", "changed_hash", "removed_hash", "invented_hash"] {
        let r = external_cargo_fixture();
        let mut v = legacy_source_roots(&r.0);
        match field {
            "version" => v["baseline"]["packages"]["a"][field] = json!("9.0.0"),
            "source_roots" => v["baseline"]["packages"]["a"][field] = json!(["pkg/", "unowned/"]),
            "changed_hash" => {
                let hashes = v["baseline"]["input_hashes"].as_object_mut().unwrap();
                let key = hashes.keys().next().unwrap().clone();
                hashes.insert(key, json!("sha256:forged"));
            }
            "removed_hash" => {
                let hashes = v["baseline"]["input_hashes"].as_object_mut().unwrap();
                let key = hashes.keys().next().unwrap().clone();
                hashes.remove(&key);
            }
            "invented_hash" => v["baseline"]["input_hashes"]["external/unrelated.rs"] = json!("sha256:forged"),
            _ => unreachable!(),
        }
        write(&r.0, CONFIG, &serde_json::to_vec(&v).unwrap());
        let before = fs::read(r.0.join(CONFIG)).unwrap();
        let p = repair_baseline_plan(&r.0, &SystemProcessRunner);
        assert_ne!(p.code, 0, "forged {field} accepted");
        assert_eq!(fs::read(r.0.join(CONFIG)).unwrap(), before);
    }
}

#[test]
fn schema4_external_targets_reject_ambiguity_and_path_escape() {
    let r = external_cargo_fixture();
    let cfg = load(&r.0).unwrap().unwrap();
    let mut snapshot = baseline_snapshot(&r.0, &cfg, &cfg.baseline.revision, &SystemProcessRunner).unwrap();
    let mut second = snapshot.packages["a"].clone();
    second.manifest = "other/Cargo.toml".into();
    snapshot.packages.insert("b".into(), second);
    assert!(polyglot::owner(&snapshot, "external/lib.rs", &cfg.native).unwrap_err().contains("ambiguous"));
    assert!(polyglot::owner(&snapshot, "external/lib.rs/unrelated.txt", &cfg.native).unwrap_err().contains("unattributed"));
    snapshot.packages.get_mut("b").unwrap().source_roots = vec!["external/".into()];
    assert!(polyglot::owner(&snapshot, "external/lib.rs", &cfg.native).unwrap_err().contains("ambiguous"));
    // Python explicit version sources retain their existing precedence in the
    // absence of Cargo external declarations; a cross-language collision does not.
    for package in snapshot.packages.values_mut() { package.language = Language::Python; }
    assert_eq!(polyglot::owner(&snapshot, "external/lib.rs", &cfg.native).unwrap(), vec!["a"]);
    snapshot.packages.get_mut("b").unwrap().language = Language::Rust;
    snapshot.packages.get_mut("b").unwrap().source_roots = vec!["external/lib.rs".into()];
    assert!(polyglot::owner(&snapshot, "external/lib.rs", &cfg.native).unwrap_err().contains("ambiguous"));
    let text = fs::read_to_string(r.0.join("pkg/Cargo.toml")).unwrap().replace("../external/lib.rs", "../../outside.rs");
    write(&r.0, "pkg/Cargo.toml", text.as_bytes());
    assert_ne!(preview(&r.0, "fix: release", &Request::default(), &SystemProcessRunner).code, 0);
}

#[test]
fn schema4_inline_cargo_targets_keep_the_same_ownership() {
    let r = external_cargo_fixture();
    let cfg = load(&r.0).unwrap().unwrap();
    let manifest = fs::read_to_string(r.0.join("pkg/Cargo.toml")).unwrap();
    let manifest = format!("bin = [{{name='inventory',path='../external/all_inventory.rs'}}]\n{}",
        manifest.replace("[[bin]]\nname='inventory'\npath='../external/all_inventory.rs'\n", ""));
    write(&r.0, "pkg/Cargo.toml", manifest.as_bytes());
    let c = capture(&r.0, &cfg, &SystemProcessRunner).unwrap();
    assert_eq!(c.current.packages["a"].source_roots, cfg.baseline.packages["a"].source_roots);
}

#[test]
fn schema4_external_lib_module_history_is_repaired_before_module_move_release() {
    for exact_entry_already_recorded in [false, true] {
        let r = fixture("legacy", true);
        write(&r.0, "pkg/Cargo.toml", b"[package]\nname='a'\nversion='1.0.0'\nedition='2021'\n[workspace]\n[lib]\npath='../legacy/lib.rs'\n");
        write(&r.0, "legacy/lib.rs", b"pub mod all_inventory;\n");
        write(&r.0, "legacy/all_inventory.rs", b"#[cfg(test)]\n#[path=\"all_inventory_tests.rs\"] mod tests; pub fn value()->u8{1}\n");
        write(&r.0, "legacy/all_inventory_tests.rs", b"#[test] fn example(){}\n");
        write(&r.0, "legacy/unrelated.rs", b"pub fn unrelated(){}\n");
        git(&r.0, &["add", "."]);
        git(&r.0, &["commit", "--amend", "--no-edit"]);
        let p = migration_plan(&r.0, &SystemProcessRunner, None);
        assert_eq!(p.code, 0, "{}", p.stderr);
        let p = p.receipt.unwrap();
        assert_eq!(migration_apply(&r.0, &SystemProcessRunner, None, p["fingerprint"].as_str().unwrap(), true).code, 0);
        let mut old: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
        old["baseline"]["packages"]["a"]["source_roots"] = if exact_entry_already_recorded { json!(["pkg/", "legacy/lib.rs"]) } else { json!(["pkg/"]) };
        old["baseline"]["input_hashes"].as_object_mut().unwrap().retain(|path, _| !path.starts_with("legacy/"));
        write(&r.0, CONFIG, &serde_json::to_vec(&old).unwrap());
        git(&r.0, &["add", "."]);
        git(&r.0, &["commit", "-m", "chore: old metadata migration"]);

        for name in ["lib.rs", "all_inventory.rs", "all_inventory_tests.rs"] {
            fs::rename(r.0.join("legacy").join(name), r.0.join("pkg/src").join(name)).unwrap();
        }
        let manifest = fs::read_to_string(r.0.join("pkg/Cargo.toml")).unwrap().replace("../legacy/lib.rs", "src/lib.rs");
        write(&r.0, "pkg/Cargo.toml", manifest.as_bytes());
        let config_before = fs::read(r.0.join(CONFIG)).unwrap();
        let index = fs::read(r.0.join(".git/index")).unwrap();
        let planned = repair_baseline_plan(&r.0, &SystemProcessRunner);
        assert_eq!(planned.code, 0, "{}", planned.stderr);
        let planned = planned.receipt.unwrap();
        let baseline = &planned["configuration"]["baseline"];
        assert_eq!(baseline["revision"], old["baseline"]["revision"]);
        assert!(baseline["packages"]["a"]["source_roots"].as_array().unwrap().contains(&json!("legacy/all_inventory.rs")));
        assert_eq!(baseline["input_hashes"]["legacy/all_inventory.rs"], json!(digest(b"#[cfg(test)]\n#[path=\"all_inventory_tests.rs\"] mod tests; pub fn value()->u8{1}\n")));
        assert!(!baseline["packages"]["a"]["source_roots"].as_array().unwrap().contains(&json!("legacy/unrelated.rs")));
        assert_eq!(fs::read(r.0.join(CONFIG)).unwrap(), config_before);
        assert_eq!(repair_baseline_apply(&r.0, &SystemProcessRunner, planned["fingerprint"].as_str().unwrap(), true).code, 0);
        assert_eq!(fs::read(r.0.join(".git/index")).unwrap(), index);
        assert_eq!(fs::read(r.0.join("VERSION")).unwrap(), b"1.0.0\n");
        let cfg = load(&r.0).unwrap().unwrap();
        let captured = capture(&r.0, &cfg, &SystemProcessRunner).unwrap();
        for path in ["legacy/all_inventory.rs", "pkg/src/all_inventory.rs"] {
            assert_eq!(captured.changes.iter().find(|c| c.path == path).unwrap().packages, vec!["a"]);
        }
    }
}
fn analyze(root: &Path, product: &str, package: Option<&str>) -> Request {
    let p = preview(
        root,
        "fix: release",
        &Request::default(),
        &SystemProcessRunner,
    );
    assert_eq!(p.code, 0, "{}", p.stderr);
    let v = p.receipt.unwrap();
    assert_eq!(v["status"], "analysis-required");
    let mut entries = serde_json::Map::new();
    if let Some(level) = package {
        entries.insert(
            "a".into(),
            json!({"level":level,"reason":"changed own source","compatibility":"compatible"}),
        );
    }
    write(root,".runtime/analysis.json",&serde_json::to_vec(&json!({"schema":1,"fingerprint":v["fingerprint"],"product":{"level":product,"reason":"actual final release changes","compatibility":"compatible"},"packages":entries})).unwrap());
    Request {
        analysis_file: Some(PathBuf::from(".runtime/analysis.json")),
    }
}
#[test]
fn schema4_two_strategies_share_both_language_adapters() {
    for strategy in ["synchronized", "independent"] {
        for rust in [false, true] {
            let r = fixture(strategy, rust);
            let before = fs::read(r.0.join(".git/index")).unwrap();
            change(&r.0, rust);
            let request = analyze(&r.0, "minor", Some("patch"));
            let p = build(
                &r.0,
                "feat: program improvement",
                &request,
                &SystemProcessRunner,
            )
            .unwrap()
            .unwrap();
            assert_eq!(p.new_version.to_string(), "1.1.0");
            assert_eq!(
                p.package_changes[0].version_after,
                if strategy == "synchronized" {
                    "1.1.0"
                } else {
                    "1.0.1"
                }
            );
            assert_eq!(fs::read(r.0.join(".git/index")).unwrap(), before);
            assert_eq!(fs::read(r.0.join("VERSION")).unwrap(), b"1.0.0\n");
        }
    }
}
#[test]
fn schema4_migration_preview_is_zero_write_and_apply_rejects_drift() {
    let r = fixture("legacy", false);
    let before = fs::read(r.0.join(CONFIG)).unwrap();
    let index = fs::read(r.0.join(".git/index")).unwrap();
    let p = migration_plan(&r.0, &SystemProcessRunner, None);
    assert_eq!(p.code, 0, "{}", p.stderr);
    assert_eq!(fs::read(r.0.join(CONFIG)).unwrap(), before);
    assert_eq!(fs::read(r.0.join(".git/index")).unwrap(), index);
    write(
        &r.0,
        "pkg/pyproject.toml",
        b"[project]\nname='a'\nversion='1.0.1'\n",
    );
    let a = migration_apply(
        &r.0,
        &SystemProcessRunner,
        None,
        p.receipt.unwrap()["fingerprint"].as_str().unwrap(),
        true,
    );
    assert_ne!(a.code, 0);
    assert_eq!(fs::read(r.0.join(CONFIG)).unwrap(), before);
}
#[test]
fn schema4_final_difference_cancels_unpublished_commits() {
    let r = fixture("independent", false);
    change(&r.0, false);
    git(&r.0, &["add", "."]);
    git(&r.0, &["commit", "-m", "feat: temporary package change"]);
    write(&r.0, "pkg/a.py", b"VALUE=1\n");
    let c = capture(&r.0, &load(&r.0).unwrap().unwrap(), &SystemProcessRunner).unwrap();
    assert!(
        c.changes.iter().all(|c| c.role == "record"),
        "{}",
        serde_json::to_string(&c.changes).unwrap()
    );
    assert!(build(
        &r.0,
        "fix: restored",
        &Request::default(),
        &SystemProcessRunner
    )
    .unwrap()
    .is_none());
}
#[test]
fn schema4_nested_cargo_lock_only_changes_product() {
    let r = fixture("independent", true);
    write(
        &r.0,
        "pkg/Cargo.lock",
        b"# whitespace and comment only\nversion=4\n[[package]]\nname='a'\nversion='1.0.0'\n",
    );
    let request = analyze(&r.0, "patch", None);
    let p = build(&r.0, "chore: lock metadata", &request, &SystemProcessRunner)
        .unwrap()
        .unwrap();
    assert!(p.package_changes.is_empty());
    assert_eq!(p.new_version.to_string(), "1.0.1");
}
#[test]
fn schema4_analysis_binds_final_bytes_and_missing_package_is_rejected() {
    let r = fixture("independent", false);
    change(&r.0, false);
    let request = analyze(&r.0, "patch", None);
    assert!(build(&r.0, "fix: change", &request, &SystemProcessRunner)
        .unwrap_err()
        .contains("analysis missing"));
    write(&r.0, "pkg/a.py", b"VALUE=3\n");
    assert!(build(&r.0, "fix: change", &request, &SystemProcessRunner)
        .unwrap_err()
        .contains("stale"));
}
#[test]
fn schema4_explicit_targets_reject_lower_and_allow_documented_pure_bump() {
    let r = fixture("independent", false);
    change(&r.0, false);
    let request = analyze(&r.0, "minor", Some("minor"));
    let mut v: Value =
        serde_json::from_slice(&fs::read(r.0.join(".runtime/analysis.json")).unwrap()).unwrap();
    v["targets"] = json!({"a":"1.0.1"});
    write(
        &r.0,
        ".runtime/analysis.json",
        &serde_json::to_vec(&v).unwrap(),
    );
    assert!(build(&r.0, "feat: change", &request, &SystemProcessRunner)
        .unwrap_err()
        .contains("below suggested"));
    write(&r.0, "pkg/a.py", b"VALUE=1\n");
    let cfg = load(&r.0).unwrap().unwrap();
    let c = capture(&r.0, &cfg, &SystemProcessRunner).unwrap();
    v["fingerprint"] = json!(c.fingerprint);
    v["product"]["level"] = json!("patch");
    v["packages"]["a"]["level"] = json!("patch");
    v["pure_version_reason"] = json!("explicit maintenance release");
    v["targets"] = json!({"a":"1.0.1"});
    write(
        &r.0,
        ".runtime/analysis.json",
        &serde_json::to_vec(&v).unwrap(),
    );
    let p = build(
        &r.0,
        "chore: maintenance release",
        &request,
        &SystemProcessRunner,
    )
    .unwrap()
    .unwrap();
    assert_eq!(p.package_changes[0].version_after, "1.0.1");
}

fn migrate(root: &Path) {
    let p = migration_plan(root, &SystemProcessRunner, None);
    assert_eq!(p.code, 0, "{}", p.stderr);
    let p = migration_apply(
        root,
        &SystemProcessRunner,
        None,
        p.receipt.unwrap()["fingerprint"].as_str().unwrap(),
        true,
    );
    assert_eq!(p.code, 0, "{}", p.stderr);
}
fn analysis_value(root: &Path) -> Value {
    serde_json::from_slice(&fs::read(root.join(".runtime/analysis.json")).unwrap()).unwrap()
}
fn save_analysis(root: &Path, v: &Value) {
    write(
        root,
        ".runtime/analysis.json",
        &serde_json::to_vec(v).unwrap(),
    );
}

fn python_environment_fixture(root: &Path) -> Value {
    write(
        root,
        ".runtime/python-environment/python.exe",
        b"fixture interpreter one",
    );
    write(
        root,
        ".runtime/python-environment/runtime/stdlib.py",
        b"fixture standard library",
    );
    write(
        root,
        ".runtime/python-environment/site-packages/example.py",
        b"fixture installed dependency one",
    );
    json!({"interpreter":".runtime/python-environment/python.exe","runtime_roots":[".runtime/python-environment/runtime"],"dependency_roots":[".runtime/python-environment/site-packages"]})
}

#[test]
fn schema4_dependency_upgrade_only_bumps_owner_when_requirement_changes() {
    for pinned in [false, true] {
        let r = fixture("legacy", false);
        let mut cfg: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
        cfg["adapters"]["python"]["projects"] =
            json!(["pkg/pyproject.toml", "consumer/pyproject.toml"]);
        cfg["adapters"]["python"]["local_sources"] =
            json!({"consumer/pyproject.toml":{"a":"pkg/pyproject.toml"}});
        cfg["packages"].as_array_mut().unwrap().push(
            json!({"id":"b","language":"python","manifest":"consumer/pyproject.toml","name":"b"}),
        );
        write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
        write(&r.0, "consumer/b.py", b"VALUE=2\n");
        write(
            &r.0,
            "consumer/pyproject.toml",
            format!(
                "[project]\nname='b'\nversion='1.0.0'\ndependencies=['a{}1.0.0']\n",
                if pinned { "==" } else { ">=" }
            )
            .as_bytes(),
        );
        git(&r.0, &["add", "."]);
        git(&r.0, &["commit", "--amend", "--no-edit"]);
        migrate(&r.0);
        change(&r.0, false);
        let request = analyze(&r.0, "patch", Some("patch"));
        let plan = build(&r.0, "fix: a", &request, &SystemProcessRunner);
        if pinned {
            assert!(plan.unwrap_err().contains("b: package analysis missing"));
            let mut v = analysis_value(&r.0);
            v["packages"]["b"] = json!({"level":"patch","reason":"effective local dependency requirement changes","compatibility":"compatible"});
            save_analysis(&r.0, &v);
            let p = build(&r.0, "fix: a", &request, &SystemProcessRunner)
                .unwrap()
                .unwrap();
            assert_eq!(p.package_changes.len(), 2);
            assert!(
                String::from_utf8_lossy(&p.writes[&r.0.join("consumer/pyproject.toml")])
                    .contains("1.0.1")
            );
        } else {
            let p = plan.unwrap().unwrap();
            assert_eq!(p.package_changes.len(), 1);
            assert_eq!(
                p.details.unwrap()["packages"][1]["reason"],
                "no_final_package_change"
            );
        }
    }
}

#[test]
fn schema4_mixed_languages_keep_per_package_impact() {
    let r = fixture("legacy", true);
    let mut cfg: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    cfg["adapters"]["python"] = json!({"projects":["py/pyproject.toml"]});
    cfg["packages"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"p","language":"python","manifest":"py/pyproject.toml","name":"p"}));
    write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
    write(
        &r.0,
        "py/pyproject.toml",
        b"[project]\nname='p'\nversion='1.0.0'\n",
    );
    write(&r.0, "py/p.py", b"VALUE=1\n");
    git(&r.0, &["add", "."]);
    git(&r.0, &["commit", "--amend", "--no-edit"]);
    migrate(&r.0);
    change(&r.0, true);
    write(&r.0, "py/p.py", b"VALUE=2\n");
    let request = analyze(&r.0, "minor", Some("patch"));
    let mut v = analysis_value(&r.0);
    v["packages"]["p"] =
        json!({"level":"minor","reason":"compatible Python feature","compatibility":"compatible"});
    save_analysis(&r.0, &v);
    let p = build(&r.0, "feat: mixed release", &request, &SystemProcessRunner)
        .unwrap()
        .unwrap();
    assert_eq!(p.new_version.to_string(), "1.1.0");
    assert!(p
        .package_changes
        .iter()
        .any(|p| p.name == "a" && p.version_after == "1.0.1"));
    assert!(p
        .package_changes
        .iter()
        .any(|p| p.name == "p" && p.version_after == "1.1.0"));
}

#[test]
fn schema4_stratus_style_build_recipe_and_ledger_do_not_bump_package() {
    let r = fixture("independent", false);
    let mut cfg: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    cfg["record_inputs"] = json!(["pkg/versions.json"]);
    cfg["build_inputs"] = json!(["scripts/build-recipes/common.json"]);
    write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
    write(&r.0, "pkg/versions.json", b"{\"template-one\":7}\n");
    let config_request = analyze(&r.0, "patch", None);
    assert!(build(
        &r.0,
        "chore: roles and record",
        &config_request,
        &SystemProcessRunner
    )
    .unwrap()
    .unwrap()
    .package_changes
    .is_empty());
    write(
        &r.0,
        "scripts/build-recipes/common.json",
        b"{\"opt-level\":3}\n",
    );
    let request = analyze(&r.0, "patch", None);
    let p = build(
        &r.0,
        "chore: compiler recipe",
        &request,
        &SystemProcessRunner,
    )
    .unwrap()
    .unwrap();
    assert!(p.package_changes.is_empty());
    assert!(!p.writes.contains_key(&r.0.join("pkg/versions.json")));
    assert_eq!(
        fs::read(r.0.join("pkg/versions.json")).unwrap(),
        b"{\"template-one\":7}\n"
    );
    cfg["build_inputs"] = json!(["pkg/a.py"]);
    write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
    assert!(load(&r.0).unwrap_err().contains("cannot suppress"));
}

#[test]
fn schema4_breaking_and_unknown_changes_block_until_bound_user_acceptance() {
    let r = fixture("independent", false);
    change(&r.0, false);
    let request = analyze(&r.0, "major", Some("major"));
    let mut v = analysis_value(&r.0);
    v["packages"]["a"]["compatibility"] = json!("breaking");
    save_analysis(&r.0, &v);
    assert!(build(&r.0, "feat!: change", &request, &SystemProcessRunner)
        .unwrap_err()
        .contains("explicit user acceptance"));
    v["packages"]["a"]["acceptance"] = json!({"fingerprint":v["fingerprint"],"scope":["a"],"reason":"user accepts public API migration","source":"explicit fixture user decision"});
    save_analysis(&r.0, &v);
    assert!(build(&r.0, "feat!: change", &request, &SystemProcessRunner)
        .unwrap()
        .is_some());
    v["packages"]["a"]["compatibility"] = json!("unknown");
    save_analysis(&r.0, &v);
    assert!(build(&r.0, "feat!: change", &request, &SystemProcessRunner)
        .unwrap_err()
        .contains("unknown"));
}

#[test]
fn schema4_validation_binds_report_scope_and_bytes() {
    let r = fixture("independent", false);
    let mut cfg: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    cfg["required_checks"] = json!(["unit"]);
    cfg["python_environment"] = python_environment_fixture(&r.0);
    write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
    change(&r.0, false);
    let request = analyze(&r.0, "patch", Some("patch"));
    assert!(build(&r.0, "fix: change", &request, &SystemProcessRunner)
        .unwrap_err()
        .contains("required release evidence missing"));
    let mut v = analysis_value(&r.0);
    let validation = capture(&r.0, &load(&r.0).unwrap().unwrap(), &SystemProcessRunner)
        .unwrap()
        .validation_fingerprint;
    let proof = json!({"id":"unit","status":"passed","fingerprint":validation,"scope":["product","a"],"reason":"fixture input-bound validation"});
    let bytes = serde_json::to_vec(&proof).unwrap();
    write(&r.0, ".runtime/evidence.json", &bytes);
    v["checks"] = json!([{"id":"unit","status":"passed","fingerprint":validation,"scope":["product","a"],"reason":"fixture input-bound validation","artifact":".runtime/evidence.json","sha256":digest(&bytes)}]);
    save_analysis(&r.0, &v);
    assert!(build(&r.0, "fix: change", &request, &SystemProcessRunner)
        .unwrap()
        .is_some());
    write(&r.0, ".runtime/evidence.json", b"{}");
    assert!(build(&r.0, "fix: change", &request, &SystemProcessRunner)
        .unwrap_err()
        .contains("artifact changed"));
}

#[test]
fn schema4_schema1_migration_preserves_unpublished_program_change() {
    let r = fixture("legacy", false);
    write(
        &r.0,
        CONFIG,
        br#"{"schema_version":1,"release_policy":"explicit_release","manifest_policy":"auto"}"#,
    );
    git(&r.0, &["add", "."]);
    git(&r.0, &["commit", "--amend", "--no-edit"]);
    write(&r.0, "README.md", b"new program documentation\n");
    git(&r.0, &["add", "README.md"]);
    git(&r.0, &["commit", "-m", "docs: unpublished"]);
    migrate(&r.0);
    let request = analyze(&r.0, "patch", None);
    let p = build(&r.0, "docs: release", &request, &SystemProcessRunner)
        .unwrap()
        .unwrap();
    assert_eq!(p.new_version.to_string(), "1.0.1");
    assert!(p.package_changes.is_empty());
}

#[test]
fn schema4_replays_multiple_moves_but_rates_only_final_change() {
    let r = fixture("independent", false);
    let base = load(&r.0).unwrap().unwrap().baseline.revision;
    for (from, to, id) in [("pkg", "moved", "move-one"), ("moved", "final", "move-two")] {
        fs::rename(r.0.join(from), r.0.join(to)).unwrap();
        let mut cfg: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
        cfg["adapters"]["python"]["projects"] = json!([format!("{to}/pyproject.toml")]);
        cfg["packages"][0]["manifest"] = json!(format!("{to}/pyproject.toml"));
        if cfg.get("lifecycle").is_none() {
            cfg["lifecycle"] = json!([]);
        }
        cfg["lifecycle"].as_array_mut().unwrap().push(json!({"id":id,"base":base,"operation":"move","compatibility":"mechanical","from":[{"id":"a","language":"python","manifest":format!("{from}/pyproject.toml"),"name":"a"}],"to":[{"id":"a","language":"python","manifest":format!("{to}/pyproject.toml"),"name":"a"}]}));
        write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
        git(&r.0, &["add", "."]);
        git(&r.0, &["commit", "-m", "refactor: move package"]);
    }
    let request = analyze(&r.0, "patch", Some("patch"));
    let p = build(
        &r.0,
        "refactor: release move",
        &request,
        &SystemProcessRunner,
    )
    .unwrap()
    .unwrap();
    assert_eq!(p.package_changes[0].version_after, "1.0.1");
    assert_eq!(p.package_events[0].operation, "move");
    assert_eq!(p.lifecycle_events.len(), 2);
}

#[test]
fn schema4_retirement_survives_release_and_cannot_reuse_identity() {
    let r = fixture("independent", false);
    let base = load(&r.0).unwrap().unwrap().baseline.revision;
    let mut cfg: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    fs::remove_dir_all(r.0.join("pkg")).unwrap();
    cfg["adapters"]["python"]["projects"] = json!([]);
    cfg["packages"] = json!([]);
    cfg["lifecycle"] = json!([{"id":"remove-a","base":base,"operation":"remove","compatibility":"compatible","from":[{"id":"a","language":"python","manifest":"pkg/pyproject.toml","name":"a"}],"to":[]}]);
    write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
    let request = analyze(&r.0, "patch", Some("patch"));
    let p = build(
        &r.0,
        "chore: retire internal package",
        &request,
        &SystemProcessRunner,
    )
    .unwrap()
    .unwrap();
    assert!(p.package_events[0].version_after.is_none());
    apply_file_release_plan(&p).unwrap();
    git(&r.0, &["add", "."]);
    git(&r.0, &["commit", "-m", "chore: release retirement"]);
    write(
        &r.0,
        "pkg/pyproject.toml",
        b"[project]\nname='a'\nversion='1.0.0'\n",
    );
    write(&r.0, "pkg/a.py", b"VALUE=1\n");
    cfg["adapters"]["python"]["projects"] = json!(["pkg/pyproject.toml"]);
    cfg["packages"] =
        json!([{"id":"a","language":"python","manifest":"pkg/pyproject.toml","name":"a"}]);
    write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
    assert!(build(
        &r.0,
        "fix: reuse",
        &Request::default(),
        &SystemProcessRunner
    )
    .unwrap_err()
    .contains("retired identity"));
}

#[test]
fn schema4_synchronized_rust_keeps_workspace_inheritance() {
    let r = fixture("legacy", true);
    write(
        &r.0,
        "Cargo.toml",
        b"[workspace]\nmembers=['pkg']\nresolver='2'\n[workspace.package]\nversion='1.0.0'\n",
    );
    write(
        &r.0,
        "pkg/Cargo.toml",
        b"[package]\nname='a'\nversion.workspace=true\nedition='2021'\n",
    );
    fs::rename(r.0.join("pkg/Cargo.lock"), r.0.join("Cargo.lock")).unwrap();
    let mut cfg: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    cfg["adapters"]["rust"]["workspaces"] = json!(["Cargo.toml"]);
    cfg["product_inputs"] = json!(["Cargo.toml", "Cargo.lock", "README.md", ".gitignore"]);
    write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
    git(&r.0, &["add", "."]);
    git(&r.0, &["commit", "--amend", "--no-edit"]);
    cfg["schema_version"] = json!(4);
    cfg["strategy"] = json!("synchronized");
    write(
        &r.0,
        ".runtime/proposal.json",
        &serde_json::to_vec(&cfg).unwrap(),
    );
    let p = configuration_plan(
        &r.0,
        &SystemProcessRunner,
        Some(".runtime/proposal.json"),
        None,
    );
    assert_eq!(p.code, 0, "{}", p.stderr);
    let p = configuration_apply(
        &r.0,
        &SystemProcessRunner,
        Some(".runtime/proposal.json"),
        None,
        p.receipt.unwrap()["fingerprint"].as_str().unwrap(),
        true,
    );
    assert_eq!(p.code, 0, "{}", p.stderr);
    change(&r.0, true);
    let request = analyze(&r.0, "minor", Some("patch"));
    let p = build(&r.0, "feat: uniform", &request, &SystemProcessRunner)
        .unwrap()
        .unwrap();
    let manifest = String::from_utf8_lossy(&p.writes[&r.0.join("pkg/Cargo.toml")])
        .parse::<toml_edit::DocumentMut>()
        .unwrap();
    assert_eq!(
        manifest["package"]["version"]["workspace"].as_bool(),
        Some(true)
    );
    assert!(String::from_utf8_lossy(&p.writes[&r.0.join("Cargo.toml")]).contains("1.1.0"));
}

#[test]
fn schema4_breaking_lifecycle_cannot_disguise_major_as_compatible() {
    let r = fixture("independent", false);
    let mut cfg: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    let base = cfg["baseline"]["revision"].clone();
    fs::remove_dir_all(r.0.join("pkg")).unwrap();
    cfg["adapters"]["python"]["projects"] = json!([]);
    cfg["packages"] = json!([]);
    cfg["lifecycle"] = json!([{"id":"remove-a","base":base,"operation":"remove","compatibility":"breaking","from":[{"id":"a","language":"python","manifest":"pkg/pyproject.toml","name":"a"}],"to":[]}]);
    write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
    let request = analyze(&r.0, "major", Some("major"));
    assert!(build(&r.0, "feat!: retire", &request, &SystemProcessRunner)
        .unwrap_err()
        .contains("breaking lifecycle"));
    let mut v = analysis_value(&r.0);
    let acceptance = json!({"fingerprint":v["fingerprint"],"scope":["product","a"],"reason":"user accepts removal and migration scope","source":"explicit fixture user decision"});
    for id in ["product", "a"] {
        let object = if id == "product" {
            &mut v["product"]
        } else {
            &mut v["packages"][id]
        };
        object["compatibility"] = json!("breaking");
        object["acceptance"] = acceptance.clone();
    }
    save_analysis(&r.0, &v);
    assert!(build(&r.0, "feat!: retire", &request, &SystemProcessRunner)
        .unwrap()
        .is_some());
}

#[test]
fn schema4_stat_only_index_refresh_keeps_analysis_valid_without_weakening_drift_checks() {
    let r = fixture("independent", false);
    git(&r.0, &["config", "core.autocrlf", "false"]);
    change(&r.0, false);
    let file = fs::OpenOptions::new().write(true).open(r.0.join("README.md")).unwrap();
    file.set_times(fs::FileTimes::new().set_modified(
        std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000),
    )).unwrap();
    let candidates = git(&r.0, &["--no-optional-locks", "-c", "diff.autoRefreshIndex=false", "diff", "--name-only", "HEAD", "--"]);
    assert!(candidates.lines().any(|p| p == "README.md"), "stat-only fixture did not create a false candidate: {candidates}");
    let cfg = load(&r.0).unwrap().unwrap();
    let index = fs::read(r.0.join(".git/index")).unwrap();
    let before = capture(&r.0, &cfg, &SystemProcessRunner).unwrap();
    let request = analyze(&r.0, "patch", Some("patch"));
    assert_eq!(fs::read(r.0.join(".git/index")).unwrap(), index, "inspection must remain read-only");
    git(&r.0, &["diff", "--name-only"]);
    let after = capture(&r.0, &cfg, &SystemProcessRunner).unwrap();
    assert_eq!(serde_json::to_value(&before.changes).unwrap(), serde_json::to_value(&after.changes).unwrap());
    assert_eq!(before.validation_fingerprint, after.validation_fingerprint);
    assert_eq!(before.fingerprint, after.fingerprint, "stat refresh must not invalidate an unchanged release analysis");
    assert!(build(&r.0, "fix: release", &request, &SystemProcessRunner).unwrap().is_some());

    write(&r.0, "README.md", b"real product change\n");
    assert!(build(&r.0, "fix: release", &request, &SystemProcessRunner).err().unwrap().contains("stale"));
    let request = analyze(&r.0, "patch", Some("patch"));
    git(&r.0, &["add", "pkg/a.py"]);
    assert!(build(&r.0, "fix: release", &request, &SystemProcessRunner).err().unwrap().contains("stale"));
    let request = analyze(&r.0, "patch", Some("patch"));
    git(&r.0, &["update-index", "--chmod=+x", "README.md"]);
    assert!(build(&r.0, "fix: release", &request, &SystemProcessRunner).err().unwrap().contains("stale"));
}

#[test]
fn schema4_content_diff_retains_binary_empty_mode_and_delimited_paths() {
    let r = fixture("legacy", false);
    git(&r.0, &["config", "core.autocrlf", "false"]);
    let mut paths = vec!["binary.asset", "empty.asset", "spaced name.asset"];
    if cfg!(unix) {
        paths.push("tab\tname.asset");
        paths.push("line\nname.asset");
    }
    let mut value: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    value["product_inputs"].as_array_mut().unwrap().push(json!("new-empty.asset"));
    for path in &paths {
        write(&r.0, path, if *path == "empty.asset" { b"" } else { b"old\0bytes\n" });
        value["product_inputs"].as_array_mut().unwrap().push(json!(path));
    }
    write(&r.0, CONFIG, &serde_json::to_vec(&value).unwrap());
    git(&r.0, &["add", "."]);
    git(&r.0, &["commit", "--amend", "--no-edit"]);
    migrate(&r.0);
    write(&r.0, "new-empty.asset", b"");
    git(&r.0, &["add", "new-empty.asset"]);
    for path in &paths {
        if *path == "empty.asset" {
            fs::remove_file(r.0.join(path)).unwrap();
        } else {
            write(&r.0, path, b"new\0bytes\n");
        }
    }
    git(&r.0, &["update-index", "--chmod=+x", "README.md"]);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let path = r.0.join("README.md");
        let mut permissions = fs::metadata(&path).unwrap().permissions();
        permissions.set_mode(permissions.mode() | 0o111);
        fs::set_permissions(path, permissions).unwrap();
    }
    let captured = capture(&r.0, &load(&r.0).unwrap().unwrap(), &SystemProcessRunner).unwrap();
    for path in paths {
        let changed = captured.changes.iter().find(|c| c.path == path).unwrap_or_else(|| panic!("missing content diff: {path:?}"));
        assert_ne!(changed.before, changed.after);
    }
    assert!(captured.changes.iter().find(|c| c.path == "README.md").unwrap().mode_changed);
    let added = captured.changes.iter().find(|c| c.path == "new-empty.asset").unwrap();
    assert!(added.before.is_none());
    assert!(added.after.is_some());
}

#[test]
fn schema4_net_reverted_dirty_source_is_still_bound_against_concurrent_change() {
    let r = fixture("independent", false);
    change(&r.0, false);
    git(&r.0, &["add", "."]);
    git(&r.0, &["commit", "-m", "feat: unpublished"]);
    write(&r.0, "pkg/a.py", b"VALUE=1\n");
    write(&r.0, "README.md", b"product repair\n");
    let request = analyze(&r.0, "patch", None);
    let p = build(&r.0, "fix: product", &request, &SystemProcessRunner)
        .unwrap()
        .unwrap();
    assert!(p.package_changes.is_empty());
    let status = git(&r.0, &["status", "--porcelain"]);
    write(&r.0, "pkg/a.py", b"VALUE=99\n");
    assert_eq!(git(&r.0, &["status", "--porcelain"]), status);
    assert!(apply_file_release_plan(&p).unwrap_err().contains("changed"));
    assert_eq!(fs::read(r.0.join("VERSION")).unwrap(), b"1.0.0\n");
}

#[test]
fn schema4_real_configuration_changes_after_migration_are_product_changes() {
    let r = fixture("independent", false);
    git(&r.0, &["add", "."]);
    git(&r.0, &["commit", "-m", "chore: migrate"]);
    let c = capture(&r.0, &load(&r.0).unwrap().unwrap(), &SystemProcessRunner).unwrap();
    assert!(
        c.changes.iter().all(|c| c.role == "record"),
        "{}",
        serde_json::to_string(&c.changes).unwrap()
    );
    assert!(build(
        &r.0,
        "chore: no changes",
        &Request::default(),
        &SystemProcessRunner
    )
    .unwrap()
    .is_none());
    let mut cfg: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    cfg["build_inputs"] = json!(["scripts/recipe.json"]);
    write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
    let request = analyze(&r.0, "patch", None);
    assert!(
        build(&r.0, "chore: role contract", &request, &SystemProcessRunner)
            .unwrap()
            .is_some()
    );
}

#[test]
fn schema4_synchronized_rejects_ambiguous_shared_package_source() {
    let r = fixture("legacy", false);
    let mut cfg: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    cfg["adapters"]["python"]["projects"] = json!(["pkg/pyproject.toml", "b/pyproject.toml"]);
    cfg["adapters"]["python"]["import_roots"] =
        json!({"pkg/pyproject.toml":["shared"],"b/pyproject.toml":["shared"]});
    cfg["packages"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"b","language":"python","manifest":"b/pyproject.toml","name":"b"}));
    write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
    write(
        &r.0,
        "b/pyproject.toml",
        b"[project]\nname='b'\nversion='1.0.0'\n",
    );
    write(&r.0, "shared/data.py", b"VALUE=1\n");
    git(&r.0, &["add", "."]);
    git(&r.0, &["commit", "--amend", "--no-edit"]);
    let p = configuration_plan(&r.0, &SystemProcessRunner, None, Some("synchronized"));
    assert_eq!(p.code, 0, "{}", p.stderr);
    let p = configuration_apply(
        &r.0,
        &SystemProcessRunner,
        None,
        Some("synchronized"),
        p.receipt.unwrap()["fingerprint"].as_str().unwrap(),
        true,
    );
    assert_eq!(p.code, 0, "{}", p.stderr);
    write(&r.0, "shared/data.py", b"VALUE=2\n");
    assert!(build(
        &r.0,
        "fix: shared",
        &Request::default(),
        &SystemProcessRunner
    )
    .unwrap_err()
    .contains("ambiguous"));
}

#[test]
fn schema4_validation_content_identity_survives_staging_and_record_commits() {
    let r = fixture("independent", false);
    let mut cfg: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    cfg["record_inputs"] = json!(["pkg/versions.json"]);
    write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
    change(&r.0, false);
    let c = capture(&r.0, &load(&r.0).unwrap().unwrap(), &SystemProcessRunner).unwrap();
    write(&r.0, "pkg/versions.json", b"{\"build\":1}\n");
    git(&r.0, &["add", "."]);
    git(&r.0, &["commit", "-m", "fix: package and build record"]);
    let after = capture(&r.0, &load(&r.0).unwrap().unwrap(), &SystemProcessRunner).unwrap();
    assert_eq!(c.validation_fingerprint, after.validation_fingerprint);
    assert_ne!(c.fingerprint, after.fingerprint);
    write(&r.0, "pkg/a.py", b"VALUE=3\n");
    assert_ne!(
        after.validation_fingerprint,
        capture(&r.0, &load(&r.0).unwrap().unwrap(), &SystemProcessRunner)
            .unwrap()
            .validation_fingerprint
    );
}

#[test]
fn schema4_migration_snapshot_tampering_cannot_conceal_manual_package_version() {
    let r = fixture("independent", false);
    let index = fs::read(r.0.join(".git/index")).unwrap();
    let mut cfg: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    cfg["baseline"]["packages"]["a"]["version"] = json!("2.0.0");
    write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
    write(
        &r.0,
        "pkg/pyproject.toml",
        b"[project]\nname='a'\nversion='2.0.0'\n",
    );
    let error = build(
        &r.0,
        "fix: forged baseline",
        &Request::default(),
        &SystemProcessRunner,
    )
    .unwrap_err();
    assert!(error.contains("immutable native history"), "{error}");
    assert_eq!(fs::read(r.0.join(".git/index")).unwrap(), index);
    assert_eq!(fs::read(r.0.join("VERSION")).unwrap(), b"1.0.0\n");
}

#[test]
fn schema4_committed_migration_baseline_is_immutable() {
    let r = fixture("independent", false);
    git(&r.0, &["add", "."]);
    git(&r.0, &["commit", "-m", "chore: migrate"]);
    let mut cfg: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    cfg["baseline"]["retired_ids"] = json!(["invented"]);
    write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
    assert!(build(
        &r.0,
        "fix: forged retirement",
        &Request::default(),
        &SystemProcessRunner
    )
    .unwrap_err()
    .contains("baseline"));
}

#[test]
fn schema4_python_environment_bytes_invalidate_evidence_and_apply_at_same_paths() {
    let r = fixture("independent", false);
    let mut cfg: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    cfg["python_environment"] = python_environment_fixture(&r.0);
    write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
    change(&r.0, false);
    let request = analyze(&r.0, "patch", Some("patch"));
    let plan = build(&r.0, "fix: native change", &request, &SystemProcessRunner)
        .unwrap()
        .unwrap();
    let original = capture(&r.0, &load(&r.0).unwrap().unwrap(), &SystemProcessRunner)
        .unwrap()
        .validation_fingerprint;
    write(
        &r.0,
        ".runtime/python-environment/site-packages/example.py",
        b"fixture installed dependency two",
    );
    assert_ne!(
        original,
        capture(&r.0, &load(&r.0).unwrap().unwrap(), &SystemProcessRunner)
            .unwrap()
            .validation_fingerprint
    );
    assert!(apply_file_release_plan(&plan)
        .unwrap_err()
        .contains("environment changed"));
    assert!(
        build(&r.0, "fix: stale evidence", &request, &SystemProcessRunner)
            .unwrap_err()
            .contains("stale")
    );
    let request = analyze(&r.0, "patch", Some("patch"));
    let plan = build(&r.0, "fix: native change", &request, &SystemProcessRunner)
        .unwrap()
        .unwrap();
    write(
        &r.0,
        ".runtime/python-environment/python.exe",
        b"fixture interpreter two",
    );
    assert!(apply_file_release_plan(&plan)
        .unwrap_err()
        .contains("environment changed"));
    assert_eq!(fs::read(r.0.join("VERSION")).unwrap(), b"1.0.0\n");
}

#[test]
fn schema4_rust_patch_overflow_fails_without_touching_versions() {
    let r = fixture("legacy", true);
    let maximum = u64::MAX.to_string();
    let version = format!("1.0.{maximum}");
    write(
        &r.0,
        "pkg/Cargo.toml",
        format!("[package]\nname='a'\nversion='{version}'\nedition='2021'\n[workspace]\n")
            .as_bytes(),
    );
    write(
        &r.0,
        "pkg/Cargo.lock",
        format!("version=4\n[[package]]\nname='a'\nversion='{version}'\n").as_bytes(),
    );
    git(&r.0, &["add", "."]);
    git(&r.0, &["commit", "--amend", "--no-edit"]);
    migrate(&r.0);
    change(&r.0, true);
    let request = analyze(&r.0, "patch", Some("patch"));
    assert!(build(&r.0, "fix: overflow", &request, &SystemProcessRunner)
        .unwrap_err()
        .contains("overflow"));
    assert_eq!(fs::read(r.0.join("VERSION")).unwrap(), b"1.0.0\n");
}

#[test]
fn schema4_runtime_guard_and_record_roles_fail_closed_without_writes() {
    let r = fixture("legacy", false);
    let config = fs::read(r.0.join(CONFIG)).unwrap();
    let index = fs::read(r.0.join(".git/index")).unwrap();
    write(&r.0, ".gitignore", b"# runtime not ignored\n");
    let p = migration_plan(&r.0, &SystemProcessRunner, None);
    assert_ne!(p.code, 0);
    assert!(p.stderr.contains(".runtime/"));
    assert_eq!(fs::read(r.0.join(CONFIG)).unwrap(), config);
    assert_eq!(fs::read(r.0.join(".git/index")).unwrap(), index);
    write(&r.0, ".gitignore", b".runtime/\n");
    migrate(&r.0);
    let mut cfg: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    for path in ["pkg/a.pyi", "pkg/code.dll", "pkg/resource.bin"] {
        cfg["record_inputs"] = json!([path]);
        assert!(parse(&serde_json::to_vec(&cfg).unwrap())
            .unwrap_err()
            .contains("record_inputs"));
    }
}

#[test]
fn schema4_forwarded_public_interface_changes_are_explicit_and_do_not_propagate_automatically() {
    let r = fixture("legacy", false);
    let mut cfg: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    cfg["adapters"]["python"]["projects"] =
        json!(["pkg/pyproject.toml", "consumer/pyproject.toml"]);
    cfg["adapters"]["python"]["local_sources"] =
        json!({"consumer/pyproject.toml":{"a":"pkg/pyproject.toml"}});
    cfg["packages"].as_array_mut().unwrap().push(
        json!({"id":"b","language":"python","manifest":"consumer/pyproject.toml","name":"b"}),
    );
    write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
    write(
        &r.0,
        "consumer/pyproject.toml",
        b"[project]\nname='b'\nversion='1.0.0'\ndependencies=['a>=1.0.0']\n",
    );
    write(&r.0, "consumer/b.py", b"from a import PublicType\n");
    write(&r.0, "pkg/a.py", b"class PublicType: pass\n");
    git(&r.0, &["add", "."]);
    git(&r.0, &["commit", "--amend", "--no-edit"]);
    migrate(&r.0);
    write(
        &r.0,
        "pkg/a.py",
        b"class PublicType:\n    def new_method(self): pass\n",
    );
    let request = analyze(&r.0, "minor", Some("minor"));
    let single = build(&r.0, "feat: public API", &request, &SystemProcessRunner)
        .unwrap()
        .unwrap();
    assert_eq!(single.package_changes.len(), 1);
    let mut analysis = analysis_value(&r.0);
    analysis["packages"]["b"] = json!({"level":"minor","reason":"re-exported PublicType gains new_method","compatibility":"compatible"});
    save_analysis(&r.0, &analysis);
    assert!(
        build(&r.0, "feat: forwarded API", &request, &SystemProcessRunner)
            .unwrap_err()
            .contains("no final change")
    );
    analysis["interface_impacts"] =
        json!({"b":{"dependencies":["a"],"reason":"consumer.b re-exports the changed PublicType"}});
    save_analysis(&r.0, &analysis);
    let both = build(&r.0, "feat: forwarded API", &request, &SystemProcessRunner)
        .unwrap()
        .unwrap();
    assert_eq!(both.package_changes.len(), 2);
    assert!(both
        .package_changes
        .iter()
        .all(|p| p.version_after == "1.1.0"));
    analysis["interface_impacts"]["b"]["reason"] = json!("");
    save_analysis(&r.0, &analysis);
    assert!(
        build(&r.0, "feat: unexplained", &request, &SystemProcessRunner)
            .unwrap_err()
            .contains("reason")
    );
    analysis["interface_impacts"]["b"] =
        json!({"dependencies":["product"],"reason":"not a package dependency"});
    save_analysis(&r.0, &analysis);
    assert!(
        build(&r.0, "feat: unrelated", &request, &SystemProcessRunner)
            .unwrap_err()
            .contains("no declared dependency")
    );
    analysis["interface_impacts"]["a"] = json!({"dependencies":["b"],"reason":"invalid cycle"});
    save_analysis(&r.0, &analysis);
    assert!(build(
        &r.0,
        "feat: invalid dependency",
        &request,
        &SystemProcessRunner
    )
    .unwrap_err()
    .contains("no declared dependency"));
    write(&r.0, "pkg/a.py", b"class PublicType: pass\n");
    write(&r.0, "README.md", b"only program documentation changed\n");
    let request = analyze(&r.0, "minor", None);
    let mut analysis = analysis_value(&r.0);
    analysis["packages"]["b"] =
        json!({"level":"minor","reason":"claimed forwarding change","compatibility":"compatible"});
    analysis["interface_impacts"] =
        json!({"b":{"dependencies":["a"],"reason":"unchanged dependency cannot seed this claim"}});
    save_analysis(&r.0, &analysis);
    assert!(build(
        &r.0,
        "feat: unrooted interface claim",
        &request,
        &SystemProcessRunner
    )
    .unwrap_err()
    .contains("not rooted"));
}

#[test]
fn schema4_required_checks_combine_their_declared_object_scopes() {
    let r = fixture("legacy", false);
    let mut cfg: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    cfg["adapters"]["python"]["projects"] = json!(["pkg/pyproject.toml", "other/pyproject.toml"]);
    cfg["packages"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"b","language":"python","manifest":"other/pyproject.toml","name":"b"}));
    write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
    write(
        &r.0,
        "other/pyproject.toml",
        b"[project]\nname='b'\nversion='1.0.0'\n",
    );
    write(&r.0, "other/b.py", b"VALUE=1\n");
    git(&r.0, &["add", "."]);
    git(&r.0, &["commit", "--amend", "--no-edit"]);
    migrate(&r.0);
    let mut cfg: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    cfg["required_checks"] = json!(["unit-a", "unit-b", "integration"]);
    cfg["check_scopes"] = json!({"unit-a":["a"],"unit-b":["b"],"integration":["product"]});
    cfg["python_environment"] = python_environment_fixture(&r.0);
    write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
    change(&r.0, false);
    write(&r.0, "other/b.py", b"VALUE=2\n");
    let request = analyze(&r.0, "patch", Some("patch"));
    let capture = capture(&r.0, &load(&r.0).unwrap().unwrap(), &SystemProcessRunner).unwrap();
    let mut claims = Vec::new();
    for (id, scope) in [
        ("unit-a", vec!["a"]),
        ("unit-b", vec!["b"]),
        ("integration", vec!["product"]),
    ] {
        let artifact = format!(".runtime/{id}.json");
        let proof = json!({"id":id,"status":"passed","fingerprint":capture.validation_fingerprint,"scope":scope,"reason":"fixture checks own declared contract"});
        let bytes = serde_json::to_vec(&proof).unwrap();
        write(&r.0, &artifact, &bytes);
        let mut claim = proof;
        claim["artifact"] = json!(artifact);
        claim["sha256"] = json!(digest(&bytes));
        claims.push(claim);
    }
    let mut analysis = analysis_value(&r.0);
    analysis["packages"]["b"] =
        json!({"level":"patch","reason":"own changed source","compatibility":"compatible"});
    analysis["checks"] = json!(claims);
    save_analysis(&r.0, &analysis);
    assert!(
        build(&r.0, "fix: scoped evidence", &request, &SystemProcessRunner)
            .unwrap()
            .is_some()
    );
    analysis["checks"].as_array_mut().unwrap().pop();
    save_analysis(&r.0, &analysis);
    assert!(build(
        &r.0,
        "fix: missing integration",
        &request,
        &SystemProcessRunner
    )
    .unwrap_err()
    .contains("required release evidence missing"));
}
