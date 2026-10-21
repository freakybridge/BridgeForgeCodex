use super::*;
use crate::{ProcessRequest, ProcessRunner, SystemProcessRunner};
use std::time::{SystemTime, UNIX_EPOCH};

struct Repo(PathBuf);
impl Drop for Repo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn git(root: &Path, args: &[&str]) {
    let mut request = ProcessRequest::new("git", root);
    request.args = args.iter().map(Into::into).collect();
    let output = SystemProcessRunner.run(&request).unwrap();
    assert_eq!(
        output.code,
        0,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
fn repo(files: &[(&str, &str)]) -> Repo {
    let root = Repo(std::env::temp_dir().join(format!(
            "bf-release-setup-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )));
    fs::create_dir_all(root.0.join(".codex")).unwrap();
    for (p, b) in files {
        let p = root.0.join(p);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, b).unwrap();
    }
    fs::write(root.0.join(".gitignore"), ".runtime/\n").unwrap();
    fs::write(root.0.join("VERSION"), "1.0.0\n").unwrap();
    fs::write(root.0.join("CHANGELOG.md"), "# Changelog\n").unwrap();
    fs::write(
        root.0.join(".codex/managed-skeleton.json"),
        r#"{"contract_target":".codex/managed-skeleton.json","assets":[]}"#,
    )
    .unwrap();
    git(&root.0, &["init"]);
    git(&root.0, &["config", "user.name", "Fixture"]);
    git(
        &root.0,
        &["config", "user.email", "fixture@example.invalid"],
    );
    git(&root.0, &["add", "."]);
    git(&root.0, &["commit", "-m", "chore: initial"]);
    root
}
fn apply_current(root: &Path) {
    let proposal = plan(root, &SystemProcessRunner, "current");
    assert_eq!(proposal.code, 0, "{}", proposal.stderr);
    let key = proposal.receipt.unwrap()["fingerprint"]
        .as_str()
        .unwrap()
        .to_string();
    let result = apply(root, &SystemProcessRunner, "current", &key, true);
    assert_eq!(result.code, 0, "{}", result.stderr);
}

#[test]
fn setup_two_choices_ignore_managed_rust_and_generate_only_schema4() {
    let root = repo(&[
        ("app.py", "pass\n"),
        (
            ".codex/hooks/Cargo.toml",
            "[package]\nname='hook'\nversion='1.0.0'\n",
        ),
    ]);
    let index = fs::read(root.0.join(".git/index")).unwrap();
    let status = status(&root.0, &SystemProcessRunner);
    assert_eq!(status.code, 0, "{}", status.stderr);
    let v = status.receipt.unwrap();
    assert_eq!(v["options"].as_array().unwrap().len(), 2);
    assert_eq!(v["options"][0]["id"], "synchronized");
    assert_eq!(v["options"][1]["id"], "independent");
    assert_eq!(fs::read(root.0.join(".git/index")).unwrap(), index);
    assert!(!root.0.join(CONFIG).exists());
    assert!(!root.0.join(".runtime").exists());
    assert_ne!(
        plan(&root.0, &SystemProcessRunner, "rust_independent").code,
        0
    );
    apply_current(&root.0);
    let cfg: Value = serde_json::from_slice(&fs::read(root.0.join(CONFIG)).unwrap()).unwrap();
    assert_eq!(cfg["schema_version"], 4);
    assert_eq!(cfg["strategy"], "synchronized");
    assert!(cfg["packages"].as_array().unwrap().is_empty());
    assert!(is_confirmed(&root.0).unwrap());
    assert!(
        !status_with(&root.0, &SystemProcessRunner, false)
            .receipt
            .unwrap()["migration_required"]
            .as_bool()
            .unwrap()
    );
}

#[test]
fn setup_legacy_rust_manifest_mapping_is_preserved_without_version_writes() {
    let root = repo(&[
        (
            CONFIG,
            r#"{"schema_version":1,"release_policy":"explicit_release","manifests":["Cargo.toml"]}"#,
        ),
        ("Cargo.toml", "[package]\nname='app'\nversion='1.0.0'\n"),
    ]);
    let native = fs::read(root.0.join("Cargo.toml")).unwrap();
    apply_current(&root.0);
    let value: Value = serde_json::from_slice(&fs::read(root.0.join(CONFIG)).unwrap()).unwrap();
    assert_eq!(value["synchronized_manifests"], json!(["Cargo.toml"]));
    assert_eq!(value["baseline"]["source_schema"], 1);
    assert_eq!(fs::read(root.0.join("Cargo.toml")).unwrap(), native);
    assert_eq!(fs::read(root.0.join("VERSION")).unwrap(), b"1.0.0\n");
}

#[test]
fn setup_missing_confirmation_and_source_drift_never_write_config() {
    let root = repo(&[("app.py", "pass\n")]);
    let p = plan(&root.0, &SystemProcessRunner, "current");
    assert_eq!(p.code, 0, "{}", p.stderr);
    let fp = p.receipt.unwrap()["fingerprint"]
        .as_str()
        .unwrap()
        .to_string();
    assert_ne!(
        apply(&root.0, &SystemProcessRunner, "current", &fp, false).code,
        0
    );
    fs::write(root.0.join("new.py"), b"pass\n").unwrap();
    assert_ne!(
        apply(&root.0, &SystemProcessRunner, "current", &fp, true).code,
        0
    );
    assert!(!root.0.join(CONFIG).exists());
}

#[test]
fn setup_explicit_python_proposal_binds_file_and_only_accepts_schema4() {
    let root = repo(&[
        (
            "pkg/pyproject.toml",
            "[project]\nname='pkg'\nversion='0.2.0'\n",
        ),
        ("pkg/pkg.py", "VALUE=1\n"),
    ]);
    fs::create_dir_all(root.0.join(".runtime")).unwrap();
    let path = ".runtime/proposal.json";
    let mut cfg = json!({"schema_version":3,"release_policy":"explicit_release","strategy":"independent","adapters":{"python":{"projects":["pkg/pyproject.toml"]}},"packages":[{"id":"pkg","language":"python","manifest":"pkg/pyproject.toml","name":"pkg"}],"product_inputs":[".gitignore"]});
    fs::write(root.0.join(path), serde_json::to_vec(&cfg).unwrap()).unwrap();
    assert_ne!(
        plan_configuration(&root.0, &SystemProcessRunner, path).code,
        0
    );
    cfg["schema_version"] = json!(4);
    fs::write(root.0.join(path), serde_json::to_vec(&cfg).unwrap()).unwrap();
    let p = plan_configuration(&root.0, &SystemProcessRunner, path);
    assert_eq!(p.code, 0, "{}", p.stderr);
    assert!(!root.0.join(CONFIG).exists());
    let fp = p.receipt.unwrap()["fingerprint"]
        .as_str()
        .unwrap()
        .to_string();
    let result = apply_configuration(&root.0, &SystemProcessRunner, path, &fp, true);
    assert_eq!(result.code, 0, "{}", result.stderr);
    assert_eq!(version_strategy(&root.0).unwrap(), "independent");
    assert_eq!(
        fs::read(root.0.join("pkg/pyproject.toml")).unwrap(),
        b"[project]\nname='pkg'\nversion='0.2.0'\n"
    );
}

#[test]
fn setup_invalid_config_or_confirmation_cannot_be_silently_replaced() {
    for content in [
        r#"{"schema_version":1,"schema_version":2}"#,
        r#"{"schema_version":1}"#,
        r#"{"schema_version":1,"manifests":[]}"#,
        r#"{"schema_version":1,"manifests":"Cargo.toml"}"#,
        r#"{"schema_version":1,"manifests":["Cargo.toml"],"git_sync_setup":{"schema":1,"confirmed":false,"selection":"current"}}"#,
    ] {
        let root = repo(&[(CONFIG, content)]);
        assert!(is_confirmed(&root.0).is_err());
        assert_ne!(status(&root.0, &SystemProcessRunner).code, 0);
        assert_eq!(fs::read_to_string(root.0.join(CONFIG)).unwrap(), content);
    }
}

#[test]
fn setup_plan_binds_ignored_native_lock_and_choice() {
    let config = r#"{"schema_version":2,"release_policy":"explicit_release","strategy":"rust_independent","rust":{"workspaces":["Cargo.toml"]}}"#;
    let root = repo(&[
        (CONFIG, config),
        ("Cargo.toml", "[package]\nname='app'\nversion='1.0.0'\n"),
        (
            "Cargo.lock",
            "version=4\n[[package]]\nname='app'\nversion='1.0.0'\n",
        ),
    ]);
    fs::write(root.0.join(".gitignore"), b".runtime/\nCargo.lock\n").unwrap();
    git(&root.0, &["rm", "--cached", "Cargo.lock"]);
    git(&root.0, &["add", "."]);
    git(&root.0, &["commit", "-m", "chore: ignore lock"]);
    let p = plan(&root.0, &SystemProcessRunner, "current");
    assert_eq!(p.code, 0, "{}", p.stderr);
    let fp = p.receipt.unwrap()["fingerprint"]
        .as_str()
        .unwrap()
        .to_string();
    assert_ne!(
        apply(&root.0, &SystemProcessRunner, "synchronized", &fp, true).code,
        0
    );
    fs::write(
        root.0.join("Cargo.lock"),
        b"version=3\n[[package]]\nname='app'\nversion='1.0.0'\n",
    )
    .unwrap();
    assert_ne!(
        apply(&root.0, &SystemProcessRunner, "current", &fp, true).code,
        0
    );
    assert_eq!(fs::read_to_string(root.0.join(CONFIG)).unwrap(), config);
}
