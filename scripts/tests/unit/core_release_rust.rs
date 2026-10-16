use super::*;

fn config(roots: &[&str]) -> RustConfig {
    RustConfig {
        workspaces: roots.iter().map(|s| s.to_string()).collect(),
        dependency_bump: "patch".into(),
        shared_inputs: Vec::new(),
        product_inputs: vec!["README.md".into()],
    }
}

fn fixture(config: &RustConfig, files: &[(&str, &str)]) -> Result<Workspace, String> {
    let files = files
        .iter()
        .map(|(p, b)| (p.to_string(), b.as_bytes().to_vec()))
        .collect::<BTreeMap<_, _>>();
    load(
        config,
        &files.keys().cloned().collect::<Vec<_>>(),
        &mut |p| Ok(files.get(p).cloned()),
    )
}

fn commit(kind: &str, breaking: bool, paths: &[&str]) -> (CommitInfo, Vec<String>) {
    (
        CommitInfo {
            kind: kind.into(),
            description: "change".into(),
            section: "Changed".into(),
            breaking,
        },
        paths.iter().map(|p| p.to_string()).collect(),
    )
}

fn graph() -> Vec<(&'static str, &'static str)> {
    vec![
        ("Cargo.toml", "[workspace]\nmembers = ['a','b','c']\n"),
        ("a/Cargo.toml", "[package]\nname = 'a'\nversion = '1.2.3'\n"),
        (
            "b/Cargo.toml",
            "[package]\nname = 'b'\nversion = '4.5.6'\n[dependencies]\na = { path = '../a', version = '=1.2.3' }\n",
        ),
        ("c/Cargo.toml", "[package]\nname = 'c'\nversion = '7.8.9'\n"),
        (
            "Cargo.lock",
            "version = 4\n[[package]]\nname = 'a'\nversion = '1.2.3'\n[[package]]\nname = 'b'\nversion = '4.5.6'\ndependencies = ['a']\n[[package]]\nname = 'c'\nversion = '7.8.9'\n",
        ),
    ]
}

#[test]
fn rust_direct_reverse_and_unrelated_package_versions() {
    let workspace = fixture(&config(&["Cargo.toml"]), &graph()).unwrap();
    let plan = workspace
        .plan(&[commit("feat", false, &["a/src/lib.rs"])])
        .unwrap();
    assert_eq!(plan.packages.len(), 2);
    assert_eq!(plan.packages[0].version_after, "1.3.0");
    assert_eq!(plan.packages[1].version_after, "4.5.7");
    assert_eq!(plan.packages[1].reason, "dependency_change");
    assert_eq!(plan.packages[1].dependencies, ["a/Cargo.toml"]);
    assert!(!plan.writes.contains_key("c/Cargo.toml"));
    assert!(
        String::from_utf8(plan.writes["b/Cargo.toml"].clone())
            .unwrap()
            .contains("=1.3.0")
    );
    assert!(
        String::from_utf8(plan.writes["Cargo.lock"].clone())
            .unwrap()
            .contains("1.3.0")
    );
}

#[test]
fn rust_highest_direct_level_wins_and_cycles_stop() {
    let mut files = graph();
    files[1].1 = "[package]\nname='a'\nversion='1.2.3'\n[dev-dependencies]\nb={path='../b'}\n";
    let workspace = fixture(&config(&["Cargo.toml"]), &files).unwrap();
    let plan = workspace
        .plan(&[
            commit("fix", false, &["a/src/lib.rs"]),
            commit("feat", true, &["b/src/lib.rs"]),
        ])
        .unwrap();
    assert_eq!(plan.packages.len(), 2);
    assert_eq!(plan.packages[0].version_after, "1.2.4");
    assert_eq!(plan.packages[1].version_after, "5.0.0");
    assert_eq!(plan.packages[0].reason, "direct_change");
}

#[test]
fn rust_inherited_versions_materialize_without_bumping_unrelated_members() {
    let files = [
        (
            "Cargo.toml",
            "[workspace]\nmembers=['a','b']\n[workspace.package]\nversion='1.2.3'\n[workspace.dependencies]\nalias={package='a',path='a',version='^1.2.3'}\n",
        ),
        (
            "a/Cargo.toml",
            "[package]\nname='a'\nversion.workspace=true\n",
        ),
        (
            "b/Cargo.toml",
            "[package]\nname='b'\nversion.workspace=true\n[target.'cfg(windows)'.build-dependencies]\nalias={workspace=true,optional=true}\n",
        ),
    ];
    let workspace = fixture(&config(&["Cargo.toml"]), &files).unwrap();
    let plan = workspace
        .plan(&[commit("feat", true, &["a/src/lib.rs"])])
        .unwrap();
    assert_eq!(plan.packages[0].version_after, "2.0.0");
    assert_eq!(plan.packages[1].version_after, "1.2.4");
    assert!(
        String::from_utf8(plan.writes["Cargo.toml"].clone())
            .unwrap()
            .contains("^2.0.0")
    );
    assert!(
        !String::from_utf8(plan.writes["b/Cargo.toml"].clone())
            .unwrap()
            .contains("version.workspace")
    );
    let unchanged = workspace
        .plan(&[commit("docs", false, &["README.md"])])
        .unwrap();
    assert!(unchanged.packages.is_empty());
    assert!(
        String::from_utf8(unchanged.writes["a/Cargo.toml"].clone())
            .unwrap()
            .contains("1.2.3")
    );
}

#[test]
fn rust_multiple_workspaces_and_renamed_cross_workspace_dependencies() {
    let files = [
        ("x/Cargo.toml", "[workspace]\nmembers=['a']\n"),
        ("x/a/Cargo.toml", "[package]\nname='a'\nversion='1.0.0'\n"),
        ("y/Cargo.toml", "[workspace]\nmembers=['b']\n"),
        (
            "y/b/Cargo.toml",
            "[package]\nname='b'\nversion='8.0.0'\n[dependencies]\nrenamed={package='a',path='../../x/a',version='1'}\n",
        ),
    ];
    let workspace = fixture(&config(&["x/Cargo.toml", "y/Cargo.toml"]), &files).unwrap();
    let plan = workspace
        .plan(&[commit("fix", false, &["x/a/src/lib.rs"])])
        .unwrap();
    assert_eq!(plan.packages.len(), 2);
    assert_eq!(plan.packages[1].version_after, "8.0.1");
    assert!(
        String::from_utf8(plan.writes["y/b/Cargo.toml"].clone())
            .unwrap()
            .contains("version='1'")
    );
}

#[test]
fn rust_lock_identity_does_not_rewrite_same_named_registry_package() {
    let mut files = graph();
    files[4].1 = "version=4\n[[package]]\nname='a'\nversion='1.2.3'\n[[package]]\nname='a'\nversion='1.2.3'\nsource='registry+https://example.invalid/index'\nchecksum='untouched'\n[[package]]\nname='b'\nversion='4.5.6'\ndependencies=['a 1.2.3', 'a 1.2.3 (registry+https://example.invalid/index)']\n[[package]]\nname='c'\nversion='7.8.9'\n";
    let workspace = fixture(&config(&["Cargo.toml"]), &files).unwrap();
    let plan = workspace
        .plan(&[commit("fix", false, &["a/src/lib.rs"])])
        .unwrap();
    let lock = parse("Cargo.lock", &plan.writes["Cargo.lock"]).unwrap();
    let entries = lock["package"].as_array_of_tables().unwrap();
    assert_eq!(entries.get(0).unwrap()["version"].as_str(), Some("1.2.4"));
    assert_eq!(entries.get(1).unwrap()["version"].as_str(), Some("1.2.3"));
    let deps = entries.get(2).unwrap()["dependencies"].as_array().unwrap();
    assert_eq!(deps.get(0).unwrap().as_str(), Some("a 1.2.4"));
    assert_eq!(
        deps.get(1).unwrap().as_str(),
        Some("a 1.2.3 (registry+https://example.invalid/index)")
    );
}

#[test]
fn rust_lock_changes_and_unowned_paths_need_explicit_attribution() {
    let workspace = fixture(&config(&["Cargo.toml"]), &graph()).unwrap();
    assert!(
        workspace
            .affected("Cargo.lock")
            .unwrap_err()
            .contains("explicit")
    );
    assert!(
        workspace
            .affected("build/common.rs")
            .unwrap_err()
            .contains("unattributed")
    );
    assert_eq!(workspace.affected("Cargo.toml").unwrap().len(), 3);
    assert!(workspace.affected("README.md").unwrap().is_empty());
    let mut configuration = config(&["Cargo.toml"]);
    configuration.shared_inputs.push(SharedInput {
        path: "Cargo.lock".into(),
        packages: vec!["a/Cargo.toml".into()],
    });
    let workspace = fixture(&configuration, &graph()).unwrap();
    assert_eq!(workspace.affected("Cargo.lock").unwrap(), ["a/Cargo.toml"]);
}

#[test]
fn rust_mechanical_migration_ignores_trivia_but_not_substantive_edits() {
    let files = [
        (
            "Cargo.toml",
            "[workspace]\nmembers=['a']\n[workspace.package]\nversion='1.2.3'\n",
        ),
        ("a/Cargo.toml", "[package]\nname='a'\nversion='1.2.3'\n"),
    ];
    let workspace = fixture(&config(&["Cargo.toml"]), &files).unwrap();
    assert!(
        workspace
            .is_mechanical_change(
                "a/Cargo.toml",
                Some(b"[package]\nname='a'\nversion.workspace=true\n"),
                Some(files[1].1.as_bytes())
            )
            .unwrap()
    );
    assert!(
        !workspace
            .is_mechanical_change(
                "a/Cargo.toml",
                Some(b"[package]\nname='a'\nversion='1.2.2'\n"),
                Some(files[1].1.as_bytes())
            )
            .unwrap()
    );
    assert!(
        !workspace
            .is_mechanical_change(
                "a/Cargo.toml",
                Some(b"[package]\nname='a'\nversion.workspace=true\nlicense='MIT'\n"),
                Some(files[1].1.as_bytes())
            )
            .unwrap()
    );
}

#[test]
fn rust_baseline_rejects_identity_and_unmanaged_version_changes() {
    let old = fixture(&config(&["Cargo.toml"]), &graph()).unwrap();
    let mut files = graph();
    files[1].1 = "[package]\nname='a'\nversion='9.0.0'\n";
    files[4].1 = "version=4\n[[package]]\nname='a'\nversion='9.0.0'\n[[package]]\nname='b'\nversion='4.5.6'\n[[package]]\nname='c'\nversion='7.8.9'\n";
    files[2].1 = "[package]\nname='b'\nversion='4.5.6'\n";
    let current = fixture(&config(&["Cargo.toml"]), &files).unwrap();
    assert!(
        current
            .validate_baseline(&old)
            .unwrap_err()
            .contains("outside")
    );
}

#[test]
fn rust_rejects_stale_lock_unregistered_dependency_and_escape() {
    let mut files = graph();
    files[4].1 = "version=4\n[[package]]\nname='a'\nversion='9.0.0'\n";
    assert!(
        fixture(&config(&["Cargo.toml"]), &files)
            .unwrap_err()
            .contains("stale")
    );
    files = graph();
    files[2].1 =
        "[package]\nname='b'\nversion='4.5.6'\n[dependencies]\nmissing={path='../missing'}\n";
    assert!(
        fixture(&config(&["Cargo.toml"]), &files)
            .unwrap_err()
            .contains("registered")
    );
    files[2].1 = "[package]\nname='b'\nversion='4.5.6'\n[dependencies]\nmissing={path='../../../../outside'}\n";
    assert!(
        fixture(&config(&["Cargo.toml"]), &files)
            .unwrap_err()
            .contains("escapes")
    );
}

#[test]
fn rust_member_globs_and_excludes_are_explicit_and_deterministic() {
    let files = [
        (
            "Cargo.toml",
            "[workspace]\nmembers=['crates/*']\nexclude=['crates/ignored']\n",
        ),
        (
            "crates/a/Cargo.toml",
            "[package]\nname='a'\nversion='1.0.0'\n",
        ),
        (
            "crates/ignored/Cargo.toml",
            "[package]\nname='ignored'\nversion='1.0.0'\n",
        ),
    ];
    let workspace = fixture(&config(&["Cargo.toml"]), &files).unwrap();
    assert_eq!(workspace.packages.len(), 1);
    assert!(workspace.affected("crates/ignored/src/lib.rs").is_err());
}

#[test]
fn rust_version_requirement_ranges_preserve_compatible_constraints() {
    for (raw, yes, no) in [
        ("1", "1.9.0", "2.0.0"),
        ("0.2", "0.2.9", "0.3.0"),
        ("~1.2", "1.2.9", "1.3.0"),
        ("=1.2.3", "1.2.3", "1.2.4"),
        (">=1.2, <2", "1.9.0", "2.0.0"),
        ("1.*", "1.8.2", "2.0.0"),
        (">1.2", "1.3.0", "1.2.9"),
        ("<=1.2", "1.2.9", "1.3.0"),
    ] {
        assert!(accepts(raw, yes.parse().unwrap()).unwrap(), "{raw}: {yes}");
        assert!(!accepts(raw, no.parse().unwrap()).unwrap(), "{raw}: {no}");
    }
    assert!(updated_requirement(">=1, <2", "2.0.0".parse().unwrap()).is_err());
    assert!(accepts("01.2.3", "1.2.3".parse().unwrap()).is_err());
    assert!(accepts(">=1.*", "1.2.3".parse().unwrap()).is_err());
}

#[test]
fn rust_cross_workspace_lock_requires_complete_local_dependency_closure() {
    let mut files = vec![
        (
            "x/Cargo.toml",
            "[package]\nname='a'\nversion='1.0.0'\n[workspace]\n",
        ),
        (
            "y/Cargo.toml",
            "[package]\nname='b'\nversion='8.0.0'\n[workspace]\n[dependencies]\na={path='../x'}\n",
        ),
        (
            "y/Cargo.lock",
            "version=4\n[[package]]\nname='b'\nversion='8.0.0'\n",
        ),
    ];
    let configuration = config(&["x/Cargo.toml", "y/Cargo.toml"]);
    assert!(
        fixture(&configuration, &files)
            .unwrap_err()
            .contains("missing")
    );
    files[2].1 = "version=4\n[[package]]\nname='b'\nversion='8.0.0'\ndependencies=['a']\n[[package]]\nname='a'\nversion='1.0.0'\n";
    let workspace = fixture(&configuration, &files).unwrap();
    let plan = workspace
        .plan(&[commit("fix", false, &["x/src/lib.rs"])])
        .unwrap();
    assert_eq!(plan.packages.len(), 2);
    let lock = parse("y/Cargo.lock", &plan.writes["y/Cargo.lock"]).unwrap();
    assert_eq!(
        lock["package"]
            .as_array_of_tables()
            .unwrap()
            .get(1)
            .unwrap()["version"]
            .as_str(),
        Some("1.0.1")
    );
    assert_eq!(workspace.inputs()["x/Cargo.lock"], None);
    assert!(!plan.writes.contains_key("x/Cargo.lock"));
}

fn cross_workspace_dev_fixture() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "x/Cargo.toml",
            "[workspace]\nmembers=['core','helper']\nresolver='2'\n",
        ),
        (
            "x/core/Cargo.toml",
            "[package]\nname='local-core'\nversion='1.0.0'\nedition='2024'\n[dev-dependencies]\nlocal-helper={path='../helper'}\n",
        ),
        (
            "x/helper/Cargo.toml",
            "[package]\nname='local-helper'\nversion='2.0.0'\nedition='2024'\n",
        ),
        (
            "y/Cargo.toml",
            "[workspace]\nmembers=['app']\nresolver='2'\n",
        ),
        (
            "y/app/Cargo.toml",
            "[package]\nname='local-app'\nversion='3.0.0'\nedition='2024'\n[dependencies]\nlocal-core={path='../../x/core'}\n",
        ),
        ("x/core/src/lib.rs", "pub fn core() {}\n"),
        ("x/helper/src/lib.rs", "pub fn helper() {}\n"),
        ("y/app/src/lib.rs", "pub fn app() {}\n"),
    ]
}

fn cross_workspace_optional_fixture(
    default_on: bool,
    explicit_on: bool,
    disable_default: bool,
) -> Vec<(&'static str, &'static str)> {
    let mut files = cross_workspace_dev_fixture();
    files[1].1 = if default_on {
        "[package]\nname='local-core'\nversion='1.0.0'\nedition='2024'\n[dependencies]\nlocal-helper={path='../helper',optional=true}\n[features]\ndefault=['with-helper']\nwith-helper=['dep:local-helper']\n"
    } else {
        "[package]\nname='local-core'\nversion='1.0.0'\nedition='2024'\n[dependencies]\nlocal-helper={path='../helper',optional=true}\n[features]\nwith-helper=['dep:local-helper']\n"
    };
    files[4].1 = if explicit_on {
        "[package]\nname='local-app'\nversion='3.0.0'\nedition='2024'\n[dependencies]\nlocal-core={path='../../x/core',features=['with-helper']}\n"
    } else if disable_default {
        "[package]\nname='local-app'\nversion='3.0.0'\nedition='2024'\n[dependencies]\nlocal-core={path='../../x/core',default-features=false}\n"
    } else {
        "[package]\nname='local-app'\nversion='3.0.0'\nedition='2024'\n[dependencies]\nlocal-core={path='../../x/core'}\n"
    };
    files
}

#[test]
fn rust_foreign_optional_lock_edges_follow_defaults_and_explicit_features() {
    for (default_on, explicit_on, disable_default, expected) in [
        (false, false, false, false),
        (true, false, true, false),
        (false, true, false, true),
        (true, false, false, true),
    ] {
        let mut files = cross_workspace_optional_fixture(default_on, explicit_on, disable_default);
        files.extend([
            ("x/Cargo.lock", "version=4\n[[package]]\nname='local-core'\nversion='1.0.0'\ndependencies=['local-helper']\n[[package]]\nname='local-helper'\nversion='2.0.0'\n"),
            ("y/Cargo.lock", "version=4\n[[package]]\nname='local-app'\nversion='3.0.0'\ndependencies=['local-core']\n[[package]]\nname='local-core'\nversion='1.0.0'\n"),
        ]);
        let result = fixture(&config(&["x/Cargo.toml", "y/Cargo.toml"]), &files);
        if expected {
            assert!(
                result.unwrap_err().contains("local-helper"),
                "default={default_on},explicit={explicit_on},disabled={disable_default}"
            );
        } else {
            let workspace = result.unwrap();
            let plan = workspace
                .plan(&[commit("fix", false, &["x/helper/src/lib.rs"])])
                .unwrap();
            assert_eq!(
                plan.packages.len(),
                3,
                "version propagation still uses all declaration edges"
            );
            assert!(
                !String::from_utf8(plan.writes["y/Cargo.lock"].clone())
                    .unwrap()
                    .contains("local-helper")
            );
        }
    }
}

#[test]
fn rust_cross_workspace_dev_edges_propagate_versions_but_not_foreign_locks() {
    let mut files = cross_workspace_dev_fixture();
    files.extend([
        ("x/Cargo.lock", "version=4\n[[package]]\nname='local-core'\nversion='1.0.0'\ndependencies=['local-helper']\n[[package]]\nname='local-helper'\nversion='2.0.0'\n"),
        ("y/Cargo.lock", "version=4\n[[package]]\nname='local-app'\nversion='3.0.0'\ndependencies=['local-core']\n[[package]]\nname='local-core'\nversion='1.0.0'\n"),
    ]);
    let workspace = fixture(&config(&["x/Cargo.toml", "y/Cargo.toml"]), &files).unwrap();
    // Version propagation deliberately includes the declaration graph's dev
    // edge: helper -> core -> app; lock validation follows Cargo's narrower graph.
    let plan = workspace
        .plan(&[commit("fix", false, &["x/helper/src/lib.rs"])])
        .unwrap();
    assert_eq!(plan.packages.len(), 3);
    assert_eq!(
        plan.packages
            .iter()
            .find(|p| p.name == "local-core")
            .unwrap()
            .reason,
        "dependency_change"
    );
    let lock = String::from_utf8(plan.writes["y/Cargo.lock"].clone()).unwrap();
    assert!(lock.contains("1.0.1"));
    assert!(!lock.contains("local-helper"));
}

#[test]
fn rust_cross_workspace_locks_are_accepted_by_offline_locked_cargo() {
    use std::fs;
    use std::path::PathBuf;
    use std::process::{Command, Stdio};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static COUNTER: AtomicU64 = AtomicU64::new(0);
    struct FixtureDirectory(PathBuf);
    impl Drop for FixtureDirectory {
        fn drop(&mut self) {
            // The only recursively removed path is this generated, absolute,
            // test-owned child of the operating system's temporary directory.
            let temp = std::env::temp_dir();
            if self.0.is_absolute()
                && self.0.starts_with(&temp)
                && self
                    .0
                    .file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with("bridgeforge-rust-lock-"))
            {
                let _ = fs::remove_dir_all(&self.0);
            }
        }
    }
    let scenarios = vec![
        (cross_workspace_dev_fixture(), false),
        (cross_workspace_optional_fixture(false, false, false), false),
        (cross_workspace_optional_fixture(true, false, true), false),
        (cross_workspace_optional_fixture(false, true, false), true),
        (cross_workspace_optional_fixture(true, false, false), true),
    ];
    for (files, helper_in_foreign_lock) in scenarios {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = FixtureDirectory(std::env::temp_dir().join(format!(
            "bridgeforge-rust-lock-{unique}-{}",
            COUNTER.fetch_add(1, Ordering::Relaxed)
        )));
        for (relative, contents) in &files {
            let path = root.0.join(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, contents).unwrap();
        }
        let cargo = |command: &str, manifest: &str, locked: bool| {
            let mut process = Command::new(env!("CARGO"));
            process
                .arg(command)
                .arg("--offline")
                .arg("--manifest-path")
                .arg(root.0.join(manifest));
            if locked {
                process.arg("--locked").arg("--format-version").arg("1");
            }
            let output = process
                .current_dir(&root.0)
                .stdin(Stdio::null())
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "cargo {command} {manifest}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        };
        for manifest in ["x/Cargo.toml", "y/Cargo.toml"] {
            cargo("generate-lockfile", manifest, false);
        }
        let mut inventory = files.iter().map(|(p, _)| p.to_string()).collect::<Vec<_>>();
        inventory.extend(["x/Cargo.lock".into(), "y/Cargo.lock".into()]);
        let workspace = load(
            &config(&["x/Cargo.toml", "y/Cargo.toml"]),
            &inventory,
            &mut |p| match fs::read(root.0.join(p)) {
                Ok(bytes) => Ok(Some(bytes)),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
                Err(error) => Err(error.to_string()),
            },
        )
        .unwrap();
        let plan = workspace
            .plan(&[commit("fix", false, &["x/helper/src/lib.rs"])])
            .unwrap();
        assert_eq!(plan.packages.len(), 3);
        assert_eq!(
            String::from_utf8(plan.writes["y/Cargo.lock"].clone())
                .unwrap()
                .contains("local-helper"),
            helper_in_foreign_lock
        );
        for (path, bytes) in plan.writes {
            fs::write(root.0.join(path), bytes).unwrap();
        }
        for manifest in ["x/Cargo.toml", "y/Cargo.toml"] {
            cargo("metadata", manifest, true);
        }
    }
}

#[test]
fn rust_unknown_lock_formats_are_rejected_before_render() {
    for header in ["", "version=2\n", "version=5\n", "version='4'\n"] {
        let files = [
            ("Cargo.toml", "[package]\nname='a'\nversion='1.0.0'\n"),
            ("Cargo.lock", header),
        ];
        assert!(
            fixture(&config(&["Cargo.toml"]), &files)
                .unwrap_err()
                .contains("format version 3 or 4")
        );
    }
    let files = [
        ("Cargo.toml", "[package]\nname='a'\nversion='1.0.0'\n"),
        (
            "Cargo.lock",
            "version=3\n[[package]]\nname='a'\nversion='1.0.0'\n",
        ),
    ];
    assert!(fixture(&config(&["Cargo.toml"]), &files).is_ok());
}

#[test]
fn rust_workspace_member_cannot_be_registered_as_an_independent_root() {
    let files = [
        ("Cargo.toml", "[workspace]\nmembers=['core']\n"),
        (
            "core/Cargo.toml",
            "[package]\nname='core'\nversion='1.0.0'\n",
        ),
        (
            "Cargo.lock",
            "version=4\n[[package]]\nname='core'\nversion='1.0.0'\n",
        ),
    ];
    let error = fixture(&config(&["core/Cargo.toml"]), &files).unwrap_err();
    assert!(error.contains("actual workspace root"), "{error}");
    let workspace = fixture(&config(&["Cargo.toml"]), &files).unwrap();
    let plan = workspace
        .plan(&[commit("fix", false, &["core/src/lib.rs"])])
        .unwrap();
    assert!(plan.writes.contains_key("Cargo.lock"));
    assert!(!plan.writes.contains_key("core/Cargo.lock"));
}

#[test]
fn rust_workspace_pointer_requires_the_actual_root() {
    let files = [
        ("Cargo.toml", "[workspace]\nmembers=['core']\n"),
        (
            "core/Cargo.toml",
            "[package]\nname='core'\nversion='1.0.0'\nworkspace='..'\n",
        ),
    ];
    assert!(
        fixture(&config(&["core/Cargo.toml"]), &files)
            .unwrap_err()
            .contains("package.workspace identifies Cargo.toml")
    );
    assert!(fixture(&config(&["Cargo.toml"]), &files).is_ok());
}

#[test]
fn rust_nested_independent_workspaces_and_excluded_packages_keep_their_lock_boundary() {
    let files = [
        (
            "Cargo.toml",
            "[workspace]\nmembers=['core']\nexclude=['standalone']\n",
        ),
        (
            "core/Cargo.toml",
            "[package]\nname='core'\nversion='1.0.0'\n",
        ),
        (
            "standalone/Cargo.toml",
            "[package]\nname='standalone'\nversion='2.0.0'\n",
        ),
        (
            "nested/Cargo.toml",
            "[package]\nname='nested'\nversion='3.0.0'\n[workspace]\n",
        ),
    ];
    let workspace = fixture(
        &config(&["standalone/Cargo.toml", "nested/Cargo.toml"]),
        &files,
    )
    .unwrap();
    assert!(workspace.inputs().contains_key("Cargo.toml"));
    let plan = workspace
        .plan(&[commit("fix", false, &["standalone/src/lib.rs"])])
        .unwrap();
    assert_eq!(plan.packages.len(), 1);
    assert!(!plan.writes.contains_key("Cargo.lock"));
    assert_eq!(workspace.inputs()["standalone/Cargo.lock"], None);
}

#[test]
fn rust_absent_ancestor_manifests_are_release_inputs() {
    let files = [(
        "standalone/nested/Cargo.toml",
        "[package]\nname='standalone'\nversion='1.0.0'\n",
    )];
    let workspace = fixture(&config(&["standalone/nested/Cargo.toml"]), &files).unwrap();
    assert_eq!(workspace.inputs()["standalone/Cargo.toml"], None);
    assert_eq!(workspace.inputs()["Cargo.toml"], None);
}

#[test]
fn rust_same_named_local_lock_identity_is_rejected_across_workspaces() {
    let files = [
        (
            "x/Cargo.toml",
            "[package]\nname='same'\nversion='1.0.0'\n[workspace]\n",
        ),
        (
            "y/Cargo.toml",
            "[package]\nname='same'\nversion='1.0.0'\n[workspace]\n",
        ),
        (
            "x/Cargo.lock",
            "version=4\n[[package]]\nname='same'\nversion='1.0.0'\n",
        ),
    ];
    assert!(
        fixture(&config(&["x/Cargo.toml", "y/Cargo.toml"]), &files)
            .unwrap_err()
            .contains("unique registered identity")
    );
}

#[test]
fn rust_version_overflow_is_an_error_without_panicking() {
    let files = [(
        "Cargo.toml",
        "[package]\nname='a'\nversion='1.0.18446744073709551615'\n",
    )];
    let workspace = fixture(&config(&["Cargo.toml"]), &files).unwrap();
    assert!(
        workspace
            .plan(&[commit("fix", false, &["src/lib.rs"])])
            .unwrap_err()
            .contains("overflow")
    );
}

#[test]
fn rust_configuration_rejects_ambiguous_maps_and_unsupported_patch() {
    let mut configuration = config(&["Cargo.toml"]);
    configuration.shared_inputs.push(SharedInput {
        path: "README.md".into(),
        packages: vec!["a/Cargo.toml".into()],
    });
    assert!(
        fixture(&configuration, &graph())
            .unwrap_err()
            .contains("overlapping")
    );
    let mut files = graph();
    files[0].1 = "[workspace]\nmembers=['a','b','c']\n[patch.crates-io]\na={path='a'}\n";
    assert!(
        fixture(&config(&["Cargo.toml"]), &files)
            .unwrap_err()
            .contains("patch/replace")
    );
}
