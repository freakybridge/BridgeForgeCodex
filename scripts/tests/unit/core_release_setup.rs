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
fn setup_python_ignores_managed_rust_and_preserves_auto_release() {
    let root = repo(&[
        ("app.py", "print('app')\n"),
        (
            ".codex/hooks/Cargo.toml",
            "[package]\nname='hook'\nversion='1.0.0'\n",
        ),
    ]);
    let index = fs::read(root.0.join(".git/index")).unwrap();
    let result = status(&root.0, &SystemProcessRunner);
    assert_eq!(result.code, 0, "{}", result.stderr);
    let receipt = result.receipt.unwrap();
    assert_eq!(receipt["status"], "configuration-required");
    assert_eq!(receipt["options"].as_array().unwrap().len(), 1);
    assert!(!root.0.join(CONFIG).exists());
    assert!(!root.0.join(".runtime").exists());
    assert_eq!(fs::read(root.0.join(".git/index")).unwrap(), index);
    let denied = plan(&root.0, &SystemProcessRunner, "rust_independent");
    assert_ne!(denied.code, 0);
    assert!(!root.0.join(CONFIG).exists());
    apply_current(&root.0);
    assert!(is_confirmed(&root.0).unwrap());
    assert!(
        status(&root.0, &SystemProcessRunner)
            .receipt
            .unwrap()
            .get("options")
            .is_none()
    );
    assert!(
        status_with(&root.0, &SystemProcessRunner, true)
            .receipt
            .unwrap()
            .get("options")
            .is_some()
    );
    assert!(
        super::super::configured_manifests(&root.0)
            .unwrap()
            .is_empty()
    );
    git(&root.0, &["add", "."]);
    git(&root.0, &["commit", "-m", "chore: remember choice"]);
    assert!(super::super::validate_release_baseline(&root.0, &SystemProcessRunner).is_ok());
    let configured = fs::read(root.0.join(CONFIG)).unwrap();
    fs::write(root.0.join("VERSION"), "2.0.0\n").unwrap();
    assert!(is_confirmed(&root.0).unwrap());
    let clone = Repo(root.0.with_extension("clone"));
    fs::create_dir_all(clone.0.join(".codex")).unwrap();
    fs::write(clone.0.join(CONFIG), configured).unwrap();
    assert!(is_confirmed(&clone.0).unwrap());
}

#[test]
fn setup_existing_configuration_is_preserved_and_reconfiguration_is_explicit() {
    let root = repo(&[
        (
            CONFIG,
            r#"{"schema_version":1,"manifests":["Cargo.toml"],"project_note":"keep"}"#,
        ),
        ("Cargo.toml", "[package]\nname=\"app\"\nversion=\"1.0.0\"\n"),
        ("src/lib.rs", "pub fn x() {}\n"),
    ]);
    assert!(!is_confirmed(&root.0).unwrap());
    apply_current(&root.0);
    let value: Value = serde_json::from_slice(&fs::read(root.0.join(CONFIG)).unwrap()).unwrap();
    assert_eq!(value["project_note"], "keep");
    assert_eq!(value["manifests"], json!(["Cargo.toml"]));
    assert!(is_confirmed(&root.0).unwrap());
}

#[test]
fn setup_missing_confirmation_or_discovery_drift_never_overwrites_config() {
    let root = repo(&[("app.py", "pass\n")]);
    let proposal = plan(&root.0, &SystemProcessRunner, "current")
        .receipt
        .unwrap();
    let key = proposal["fingerprint"].as_str().unwrap();
    assert_ne!(
        apply(&root.0, &SystemProcessRunner, "current", key, false).code,
        0
    );
    assert!(!root.0.join(".runtime").exists());
    fs::write(
        root.0.join("Cargo.toml"),
        "[package]\nname='new'\nversion='1.0.0'\n",
    )
    .unwrap();
    assert_ne!(
        apply(&root.0, &SystemProcessRunner, "current", key, true).code,
        0
    );
    assert!(!root.0.join(CONFIG).exists());
}

#[test]
fn setup_rust_can_enable_safe_workspace_but_requires_hook_adaptation() {
    let root = repo(&[
        ("Cargo.toml", "[package]\nname='app'\nversion='1.0.0'\n"),
        ("src/lib.rs", "pub fn x() {}\n"),
    ]);
    let proposal = plan(&root.0, &SystemProcessRunner, "rust_independent");
    assert_eq!(proposal.code, 0, "{}", proposal.stderr);
    let key = proposal.receipt.unwrap()["fingerprint"]
        .as_str()
        .unwrap()
        .to_string();
    let result = apply(
        &root.0,
        &SystemProcessRunner,
        "rust_independent",
        &key,
        true,
    );
    assert_eq!(result.code, 0, "{}", result.stderr);
    assert_eq!(version_strategy(&root.0).unwrap(), "rust_independent");
    assert!(is_confirmed(&root.0).unwrap());
    let hooked = repo(&[
        ("Cargo.toml", "[package]\nname='app'\nversion='1.0.0'\n"),
        (".codex/hooks/project_custom/entrypoint.rs", "fn main() {}"),
    ]);
    let denied = plan(&hooked.0, &SystemProcessRunner, "rust_independent");
    assert!(denied.stderr.contains("adaptation"));
    assert!(!hooked.0.join(CONFIG).exists());
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
fn setup_current_survives_deleted_or_invalid_rust_candidate() {
    for deleted in [false, true] {
        let root = repo(&[("Cargo.toml", "[package]\nname='app'\nversion='1.0.0'\n")]);
        if deleted {
            fs::remove_file(root.0.join("Cargo.toml")).unwrap();
        } else {
            fs::write(root.0.join("Cargo.toml"), "[broken").unwrap();
        }
        let result = status(&root.0, &SystemProcessRunner);
        assert_eq!(result.code, 0, "{}", result.stderr);
        assert!(
            !result.receipt.unwrap()["rust_discovery_errors"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        apply_current(&root.0);
        assert!(is_confirmed(&root.0).unwrap());
    }
}

#[test]
fn setup_plan_binds_ignored_lock_and_strategy() {
    let config = r#"{"schema_version":2,"release_policy":"explicit_release","strategy":"rust_independent","rust":{"workspaces":["Cargo.toml"]}}"#;
    let root = repo(&[
        (CONFIG, config),
        ("Cargo.toml", "[package]\nname='app'\nversion='1.0.0'\n"),
        (
            "Cargo.lock",
            "version=4\n[[package]]\nname='app'\nversion='1.0.0'\n",
        ),
    ]);
    fs::write(root.0.join(".gitignore"), ".runtime/\nCargo.lock\n").unwrap();
    git(&root.0, &["rm", "--cached", "Cargo.lock"]);
    git(&root.0, &["add", ".gitignore"]);
    git(&root.0, &["commit", "-m", "chore: ignore lock"]);
    let proposal = plan(&root.0, &SystemProcessRunner, "current");
    assert_eq!(proposal.code, 0, "{}", proposal.stderr);
    let key = proposal.receipt.unwrap()["fingerprint"]
        .as_str()
        .unwrap()
        .to_string();
    assert_ne!(
        apply(
            &root.0,
            &SystemProcessRunner,
            "rust_independent",
            &key,
            true
        )
        .code,
        0
    );
    fs::write(
        root.0.join("Cargo.lock"),
        "version=3\n[[package]]\nname='app'\nversion='1.0.0'\n",
    )
    .unwrap();
    let result = apply(&root.0, &SystemProcessRunner, "current", &key, true);
    assert_ne!(result.code, 0);
    assert!(result.stderr.contains("changed"));
    assert_eq!(fs::read_to_string(root.0.join(CONFIG)).unwrap(), config);
}

#[test]
fn setup_rust_rejects_new_package_missing_from_release_baseline() {
    let root = repo(&[("app.py", "pass\n")]);
    fs::write(
        root.0.join("Cargo.toml"),
        "[package]\nname='app'\nversion='1.0.0'\n",
    )
    .unwrap();
    fs::create_dir_all(root.0.join("src")).unwrap();
    fs::write(root.0.join("src/lib.rs"), "pub fn f() {}\n").unwrap();
    let result = plan(&root.0, &SystemProcessRunner, "rust_independent");
    assert_ne!(result.code, 0);
    assert!(result.stderr.contains("adaptation"));
    assert!(!root.0.join(CONFIG).exists());
}

#[test]
fn setup_confirmed_rust_rejects_invalid_static_policy() {
    for rust in [
        json!({"workspaces":[]}),
        json!({"workspaces":["Cargo.toml"],"dependency_bump":"foo"}),
    ] {
        let root = repo(&[]);
        let value = json!({"schema_version":2,"release_policy":"explicit_release","strategy":"rust_independent","rust":rust,
            "git_sync_setup":{"schema":1,"confirmed":true,"selection":"current"}});
        fs::write(root.0.join(CONFIG), serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(is_confirmed(&root.0).is_err());
        assert_ne!(status(&root.0, &SystemProcessRunner).code, 0);
    }
}

#[test]
fn setup_rust_preflights_unreleased_intermediate_package_versions() {
    let root = repo(&[
        ("Cargo.toml", "[package]\nname='app'\nversion='1.0.0'\n"),
        ("src/lib.rs", "pub fn f() {}\n"),
    ]);
    for version in ["1.0.1", "1.0.0"] {
        fs::write(
            root.0.join("Cargo.toml"),
            format!("[package]\nname='app'\nversion='{version}'\n"),
        )
        .unwrap();
        git(&root.0, &["add", "."]);
        git(&root.0, &["commit", "-m", "chore: manual version"]);
    }
    let result = plan(&root.0, &SystemProcessRunner, "rust_independent");
    assert_ne!(result.code, 0);
    assert!(result.stderr.contains("adaptation"));
    assert!(!root.0.join(CONFIG).exists());
}
