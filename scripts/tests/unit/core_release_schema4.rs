use super::*;
use crate::SystemProcessRunner;
use std::process::Command;

#[path = "core_package_development.rs"]
mod development_tests;

struct Repo(PathBuf);
impl Drop for Repo {
    fn drop(&mut self) {
        if self
            .0
            .file_name()
            .is_some_and(|n| n.to_string_lossy().starts_with("bf-schema4-"))
            && self.0.parent()
                == Some(
                    test_artifacts::root(env!("CARGO_MANIFEST_DIR"), "release-schema4").as_path(),
                )
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
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let root = test_artifacts::root(env!("CARGO_MANIFEST_DIR"), "release-schema4").join(format!(
        "bf-schema4-{}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
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
        br#"{"schema_version":1,"release_version":"8.0.0","assets":[]}"#,
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
    if strategy != "unconfigured" {
        assert_eq!(
            strategy, "independent",
            "only the current model belongs in current fixtures"
        );
        super::fixture::migrate(&root, &SystemProcessRunner).unwrap();
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

const MANAGED_CARGO: &str = ".codex/hooks/crates/bridgeforge-core/Cargo.toml";

#[test]
fn release_host_bookkeeping_is_not_a_toolchain_or_input_change() {
    let base = BTreeMap::from([
        ("PATH".into(), "stable-tools".into()),
        ("RUSTFLAGS".into(), "-C opt-level=2".into()),
        ("WT_SESSION".into(), "terminal-one".into()),
        (
            "PSModuleAnalysisCachePath".into(),
            "first-shell-cache".into(),
        ),
    ]);
    let expected = inputs::release_environment_fingerprints(&base)
        .unwrap()
        .planning;
    let mut other = base.clone();
    other.insert("WT_SESSION".into(), "terminal-two".into());
    other.insert(
        "PSModuleAnalysisCachePath".into(),
        "another-shell-cache".into(),
    );
    other.insert("CODEX_TOOL_CALL_ID".into(), "different-call".into());
    assert_eq!(
        inputs::release_environment_fingerprints(&other)
            .unwrap()
            .planning,
        expected
    );
    assert_ne!(
        inputs::environment_fingerprints(&other).unwrap().planning,
        inputs::environment_fingerprints(&base).unwrap().planning,
        "develop keeps its strict host guard"
    );
    for key in ["PATH", "RUSTFLAGS", "CUSTOM_BUILD_INPUT"] {
        let mut changed = other.clone();
        changed.insert(key.into(), "changed".into());
        assert_ne!(
            inputs::release_environment_fingerprints(&changed)
                .unwrap()
                .planning,
            expected
        );
    }
}

#[test]
#[ignore = "child process for the cross-host release regression"]
fn release_host_context_probe() {
    let root = PathBuf::from(std::env::var_os("BRIDGEFORGE_RELEASE_HOST_ROOT").unwrap());
    let path = root.join(".runtime/host-analysis.json");
    if !path.exists() {
        let result = preview(
            &root,
            "fix: host fixture",
            &Request::default(),
            &SystemProcessRunner,
        );
        assert_eq!(result.code, 0, "{}", result.stderr);
        let receipt = result.receipt.unwrap();
        write(&root, ".runtime/host-analysis.json", &serde_json::to_vec(&json!({
            "schema":1,"fingerprint":receipt["fingerprint"],
            "product":{"level":"patch","reason":"isolated host regression","compatibility":"compatible"}
        })).unwrap());
    } else {
        let result = build(
            &root,
            "fix: host fixture",
            &Request {
                analysis_file: Some(".runtime/host-analysis.json".into()),
            },
            &SystemProcessRunner,
        );
        if root.join(".runtime/expect-stale").exists() {
            assert!(result.unwrap_err().contains("stale"));
        } else {
            assert_eq!(result.unwrap().unwrap().new_version.to_string(), "1.0.1");
        }
    }
}

#[test]
fn release_host_context_cross_process_preserves_only_valid_analysis() {
    let r = fixture("independent", false);
    write(&r.0, "README.md", b"new product content\n");
    for step in 0..3 {
        if step == 2 {
            write(&r.0, ".runtime/expect-stale", b"yes");
        }
        let mut command = ProcessRequest::new(std::env::current_exe().unwrap(), &r.0);
        command.args = vec![
            "--exact".into(),
            format!(
                "{}::release_host_context_probe",
                module_path!().split_once("::").unwrap().1
            )
            .into(),
            "--ignored".into(),
            "--nocapture".into(),
        ];
        command.env.insert(
            "BRIDGEFORGE_RELEASE_HOST_ROOT".into(),
            r.0.as_os_str().into(),
        );
        command
            .env
            .insert("WT_SESSION".into(), format!("terminal-{step}").into());
        command.env.insert(
            "PSModuleAnalysisCachePath".into(),
            format!("shell-cache-{step}").into(),
        );
        command.env.insert(
            "CUSTOM_BUILD_INPUT".into(),
            if step == 2 { "changed" } else { "stable" }.into(),
        );
        command.timeout = Duration::from_secs(180);
        let result = SystemProcessRunner.run(&command).unwrap();
        assert!(!result.timed_out);
        assert_eq!(
            result.code,
            0,
            "{}\n{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
    }
}

#[test]
fn release_host_explicit_project_variable_still_invalidates_analysis() {
    let r = fixture("independent", true);
    change(&r.0, true);
    super::fixture::close_packages(&r.0, &SystemProcessRunner, &[]).unwrap();
    let path = r.0.join(CONFIG);
    let mut cfg: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    cfg["development"]["a"]["environment"] = json!({
        "tools":["cargo","rustc"],"variables":["WT_SESSION"],"inputs":{}
    });
    write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
    for step in 0..2 {
        if step == 1 {
            write(&r.0, ".runtime/expect-stale", b"yes");
        }
        let mut command = ProcessRequest::new(std::env::current_exe().unwrap(), &r.0);
        command.args = vec![
            "--exact".into(),
            format!(
                "{}::release_host_context_probe",
                module_path!().split_once("::").unwrap().1
            )
            .into(),
            "--ignored".into(),
            "--nocapture".into(),
        ];
        command.env.insert(
            "BRIDGEFORGE_RELEASE_HOST_ROOT".into(),
            r.0.as_os_str().into(),
        );
        command
            .env
            .insert("WT_SESSION".into(), format!("declared-input-{step}").into());
        command.timeout = Duration::from_secs(180);
        let result = SystemProcessRunner.run(&command).unwrap();
        assert!(!result.timed_out);
        assert_eq!(
            result.code,
            0,
            "{}\n{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
    }
}

fn cumulative_skeleton(root: &Path, generation: u8, project: &str) {
    let cargo = format!("[package]\nname='managed-tool'\nversion='1.0.{generation}'\n");
    let public = format!("BEGIN\npublic {generation}\nEND\n");
    write(root, MANAGED_CARGO, cargo.as_bytes());
    write(root, "README.md", format!("{public}{project}\n").as_bytes());
    let contract = json!({"release_version":"8.0.0","contract_target":".codex/managed-skeleton.json","assets":[
        {"id":"managed.cargo","target":MANAGED_CARGO,"strategy":"whole","current_sha256":digest(cargo.as_bytes())},
        {"id":"managed.readme","target":"README.md","strategy":"region",
         "region":{"begin":"BEGIN","end":"END","current_sha256":digest(public.as_bytes())}}
    ]});
    write(
        root,
        ".codex/managed-skeleton.json",
        &serde_json::to_vec(&contract).unwrap(),
    );
}

fn cumulative_skeleton_fixture() -> Repo {
    let r = fixture("unconfigured", true);
    cumulative_skeleton(&r.0, 0, "project original");
    git(&r.0, &["add", "."]);
    git(&r.0, &["commit", "--amend", "--no-edit"]);
    migrate(&r.0);
    r
}

#[test]
fn schema4_cumulative_skeleton_committed_update_is_not_business_release() {
    let r = cumulative_skeleton_fixture();
    cumulative_skeleton(&r.0, 1, "project original");
    for committed in [false, true] {
        if committed {
            git(&r.0, &["add", "."]);
            git(
                &r.0,
                &["commit", "-m", "chore: skeleton update without release"],
            );
        }
        let index = fs::read(r.0.join(".git/index")).unwrap();
        let cfg = load(&r.0).unwrap().unwrap();
        let captured = capture(&r.0, &cfg, &SystemProcessRunner).unwrap();
        assert!(captured.changes.is_empty(), "{:?}", captured.changes);
        assert!(
            build(
                &r.0,
                "chore: skeleton",
                &Request::default(),
                &SystemProcessRunner
            )
            .unwrap()
            .is_none()
        );
        assert_eq!(fs::read(r.0.join(".git/index")).unwrap(), index);
        assert_eq!(fs::read(r.0.join("VERSION")).unwrap(), b"1.0.0\n");
    }
    // Cumulative skeleton changes must not block or absorb a real package change.
    change(&r.0, true);
    let request = analyze(&r.0, "patch", Some("patch"));
    let plan = build(&r.0, "fix: business", &request, &SystemProcessRunner)
        .unwrap()
        .unwrap();
    assert_eq!(plan.new_version.to_string(), "1.0.1");
    assert_eq!(plan.package_changes.len(), 1);
    assert_eq!(plan.package_changes[0].version_after, "1.0.1");
}

#[test]
fn schema4_cumulative_skeleton_preserves_committed_project_region_changes() {
    let r = cumulative_skeleton_fixture();
    cumulative_skeleton(&r.0, 1, "project changed");
    git(&r.0, &["add", "."]);
    git(
        &r.0,
        &["commit", "-m", "fix: project customization with skeleton"],
    );
    // HEAD-to-worktree sees only the new public region; the release also owes
    // the already committed project customization since the formal boundary.
    cumulative_skeleton(&r.0, 2, "project changed");
    let captured = capture(&r.0, &load(&r.0).unwrap().unwrap(), &SystemProcessRunner).unwrap();
    assert_eq!(captured.changes.len(), 1);
    assert_eq!(captured.changes[0].path, "README.md");
    assert_eq!(captured.changes[0].role, "product");
    let request = analyze(&r.0, "patch", None);
    let plan = build(&r.0, "fix: customization", &request, &SystemProcessRunner)
        .unwrap()
        .unwrap();
    assert_eq!(plan.new_version.to_string(), "1.0.1");
    assert!(plan.package_changes.is_empty());
}

#[test]
fn schema4_cumulative_skeleton_rejects_tampering_and_unknown_inputs() {
    let r = cumulative_skeleton_fixture();
    cumulative_skeleton(&r.0, 1, "project original");
    git(&r.0, &["add", "."]);
    git(&r.0, &["commit", "-m", "chore: skeleton"]);
    write(&r.0, MANAGED_CARGO, b"unverified payload\n");
    let cfg = load(&r.0).unwrap().unwrap();
    let error = capture(&r.0, &cfg, &SystemProcessRunner).err().unwrap();
    assert!(
        error.contains("current ownership baseline is invalid"),
        "{error}"
    );
    cumulative_skeleton(&r.0, 1, "project original");
    write(&r.0, ".codex/hooks/unknown.rs", b"fn unknown() {}\n");
    let error = capture(&r.0, &cfg, &SystemProcessRunner).err().unwrap();
    assert!(
        error.contains("unattributed release input: .codex/hooks/unknown.rs"),
        "{error}"
    );
}

#[test]
fn schema4_relocated_skeleton_uses_formal_boundary_and_requires_business_closure() {
    const OLD: &str = ".codex/hooks/crates/bridgeforge-core/src/memory/ownership.rs";
    const NEW: &str = ".codex/hooks/source/memory/src/ownership.rs";
    let contract = |target: &str, bytes: &[u8]| {
        json!({"release_version":"8.0.0","assets":[{
            "id":"managed.memory", "target":target, "strategy":"whole", "current_sha256":digest(bytes)
        }]})
    };
    for updated in [false, true] {
        let r = fixture("unconfigured", true);
        write(&r.0, OLD, b"original\n");
        write(
            &r.0,
            ".codex/managed-skeleton.json",
            &serde_json::to_vec(&contract(OLD, b"original\n")).unwrap(),
        );
        git(&r.0, &["add", "."]);
        git(&r.0, &["commit", "--amend", "--no-edit"]);
        migrate(&r.0);
        let bytes = if updated {
            b"updated!\n"
        } else {
            b"original\n"
        };
        fs::remove_file(r.0.join(OLD)).unwrap();
        write(&r.0, NEW, bytes);
        write(
            &r.0,
            ".codex/managed-skeleton.json",
            &serde_json::to_vec(&contract(NEW, bytes)).unwrap(),
        );
        for committed in [false, true] {
            if committed {
                git(&r.0, &["add", "."]);
                git(&r.0, &["commit", "-m", "chore: skeleton path migration"]);
            }
            let index = fs::read(r.0.join(".git/index")).unwrap();
            let config = fs::read(r.0.join(CONFIG)).unwrap();
            let p = preview(
                &r.0,
                "chore: skeleton migration",
                &Request::default(),
                &SystemProcessRunner,
            );
            assert_eq!(p.code, 0, "{}", p.stderr);
            assert_eq!(p.receipt.unwrap()["status"], "nothing-to-release");
            assert_eq!(fs::read(r.0.join(".git/index")).unwrap(), index);
            assert_eq!(fs::read(r.0.join(CONFIG)).unwrap(), config);
            assert_eq!(fs::read(r.0.join("VERSION")).unwrap(), b"1.0.0\n");
        }
        // A committed migration stays excluded, but real unpublished package
        // changes still require develop; release cannot certify them implicitly.
        change(&r.0, true);
        let request = analyze(&r.0, "patch", None);
        let error = build(&r.0, "fix: business", &request, &SystemProcessRunner).unwrap_err();
        assert!(
            error.contains("closure") || error.contains("develop"),
            "{error}"
        );
        let request = analyze(&r.0, "patch", Some("patch"));
        let plan = build(&r.0, "fix: business", &request, &SystemProcessRunner)
            .unwrap()
            .unwrap();
        assert_eq!(plan.new_version.to_string(), "1.0.1");
        assert_eq!(plan.package_changes.len(), 1);
        assert_eq!(plan.package_changes[0].version_after, "1.0.1");
        write(&r.0, ".codex/hooks/unknown.rs", b"project owned\n");
        let cfg = load(&r.0).unwrap().unwrap();
        let error = capture(&r.0, &cfg, &SystemProcessRunner).err().unwrap();
        assert!(
            error.contains("unattributed release input: .codex/hooks/unknown.rs"),
            "{error}"
        );
    }
}

#[test]
fn schema4_cutover_preserves_unreleased_changes_across_legacy_intermediate_commits() {
    let r = fixture("unconfigured", true);
    let original = fs::read(r.0.join(CONFIG)).unwrap();
    let release = git(&r.0, &["rev-parse", "HEAD"]);
    let proposal = fixture::current_fixture::proposal(&r.0).unwrap();
    let planned = migration_plan(
        &r.0,
        &SystemProcessRunner,
        proposal.as_ref().map(|p| p.to_str().unwrap()),
    );
    assert_eq!(planned.code, 0, "{}", planned.stderr);
    let mut old = planned.receipt.unwrap()["configuration"].clone();
    old.as_object_mut().unwrap().remove("release_model");
    old["baseline"]["initial_configuration"]
        .as_object_mut()
        .unwrap()
        .remove("release_model");
    old["baseline"]["initial_configuration_hash"] = json!(
        configuration::configuration_hash(&old["baseline"]["initial_configuration"]).unwrap()
    );
    write(&r.0, CONFIG, &serde_json::to_vec(&old).unwrap());
    git(&r.0, &["add", "."]);
    git(&r.0, &["commit", "-m", "chore: old schema4 migration"]);
    write(&r.0, CONFIG, &original);
    change(&r.0, true);
    git(&r.0, &["add", "."]);
    git(&r.0, &["commit", "-m", "fix: unpublished legacy change"]);
    write(&r.0, CONFIG, &serde_json::to_vec(&old).unwrap());
    git(&r.0, &["add", "."]);
    git(&r.0, &["commit", "-m", "chore: restore old schema4"]);
    let head = git(&r.0, &["rev-parse", "HEAD"]);
    let before = fs::read(r.0.join(CONFIG)).unwrap();
    let upgrade = prepare_project_upgrade(&r.0).unwrap().unwrap();
    let payload = upgrade.payload().unwrap();
    let value: Value = serde_json::from_slice(&payload).unwrap();
    assert_eq!(value["baseline"]["revision"], release);
    assert_eq!(value["baseline"]["migration_head"], head);
    assert_eq!(value["baseline"]["source_schema"], 4);
    assert_eq!(fs::read(r.0.join(CONFIG)).unwrap(), before);
    upgrade.verify(&r.0).unwrap();
    write(&r.0, "pkg/src/lib.rs", b"pub fn value()->u8{3}\n");
    assert!(upgrade.verify(&r.0).unwrap_err().contains("drifted"));
    change(&r.0, true);
    write(&r.0, CONFIG, &payload);
    let request = analyze(&r.0, "patch", Some("patch"));
    let plan = build(
        &r.0,
        "fix: preserved change",
        &request,
        &SystemProcessRunner,
    )
    .unwrap()
    .unwrap();
    assert_eq!(plan.new_version.to_string(), "1.0.1");
    assert_eq!(plan.package_changes[0].version_after, "1.0.1");
    assert_eq!(fs::read(r.0.join("VERSION")).unwrap(), b"1.0.0\n");
    git(&r.0, &["add", "."]);
    git(&r.0, &["commit", "-m", "chore: one-time cutover"]);
    assert!(prepare_project_upgrade(&r.0).unwrap().is_none());
    // Reintroducing an old contract cannot obtain another automatic cutover.
    write(&r.0, CONFIG, &original);
    git(&r.0, &["add", "."]);
    git(
        &r.0,
        &["commit", "-m", "chore: invalid legacy reintroduction"],
    );
    assert!(
        prepare_project_upgrade(&r.0)
            .unwrap_err()
            .contains("legacy configuration migration is retired")
    );
    write(&r.0, CONFIG, &payload);
    assert!(
        build(
            &r.0,
            "fix: no bypass",
            &Request::default(),
            &SystemProcessRunner
        )
        .unwrap_err()
        .contains("post-migration")
    );
}

#[test]
fn schema4_current_config_rejects_retired_strategy_and_manual_package_versions() {
    let r = fixture("independent", true);
    assert!(
        configuration_plan(&r.0, &SystemProcessRunner, None, Some("synchronized"))
            .stderr
            .contains("one model")
    );
    let before = fs::read(r.0.join(CONFIG)).unwrap();
    let index = fs::read(r.0.join(".git/index")).unwrap();
    write(
        &r.0,
        "pkg/Cargo.toml",
        b"[package]\nname='a'\nversion='2.0.0'\nedition='2021'\n[workspace]\n",
    );
    let result = preview(
        &r.0,
        "fix: manual version",
        &Request::default(),
        &SystemProcessRunner,
    );
    assert_ne!(result.code, 0, "manual package version must be rejected");
    assert_eq!(fs::read(r.0.join(CONFIG)).unwrap(), before);
    assert_eq!(fs::read(r.0.join(".git/index")).unwrap(), index);
}

#[test]
fn schema4_product_mapping_cannot_implicitly_upgrade_registered_workspace_members() {
    let r = fixture("unconfigured", true);
    write(
        &r.0,
        "Cargo.toml",
        b"[workspace]\nmembers=['pkg']\nresolver='2'\n",
    );
    write(
        &r.0,
        "pkg/Cargo.toml",
        b"[package]\nname='a'\nversion='1.0.0'\nedition='2021'\n",
    );
    fs::rename(r.0.join("pkg/Cargo.lock"), r.0.join("Cargo.lock")).unwrap();
    let mut cfg: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    cfg["adapters"]["rust"]["workspaces"] = json!(["Cargo.toml"]);
    cfg["product_inputs"] = json!(["Cargo.toml", "Cargo.lock", "README.md", ".gitignore"]);
    write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
    git(&r.0, &["add", "."]);
    git(&r.0, &["commit", "--amend", "--no-edit"]);
    migrate(&r.0);
    let mut cfg: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    cfg["product_manifests"] = json!(["Cargo.toml"]);
    write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
    write(&r.0, "README.md", b"product-only change\n");
    let error = build(
        &r.0,
        "fix: product only",
        &Request::default(),
        &SystemProcessRunner,
    )
    .unwrap_err();
    assert!(
        error.contains("must not include independently registered packages"),
        "{error}"
    );
    cfg.as_object_mut().unwrap().remove("baseline");
    write(
        &r.0,
        ".runtime/overlap.json",
        &serde_json::to_vec(&cfg).unwrap(),
    );
    let plan = configuration_plan(
        &r.0,
        &SystemProcessRunner,
        Some(".runtime/overlap.json"),
        None,
    );
    assert!(
        plan.stderr
            .contains("must not include independently registered packages"),
        "{}",
        plan.stderr
    );
}

fn mixed_product_native() -> (polyglot::Config, BTreeMap<String, Vec<u8>>) {
    let config: polyglot::Config = serde_json::from_value(json!({
        "schema_version":3,"release_policy":"explicit_release","strategy":"independent",
        "git_sync_setup":{"schema":1,"confirmed":true,"selection":"independent"},
        "adapters":{"rust":{"workspaces":["Cargo.toml"]}},
        "packages":[{"id":"library","name":"library","language":"rust","manifest":"source/library/Cargo.toml"}],
        "product_packages":[
            {"id":"cli","name":"cli","language":"rust","manifest":"source/cli/Cargo.toml"},
            {"id":"hook","name":"hook","language":"rust","manifest":"source/hook/Cargo.toml"}
        ]
    })).unwrap();
    let files = BTreeMap::from([
        ("Cargo.toml".into(), b"[workspace]\nmembers=['source/library','source/cli','source/hook']\nresolver='2'\n[workspace.package]\nversion='1.0.0'\n".to_vec()),
        ("source/library/Cargo.toml".into(), b"[package]\nname='library'\nversion='0.4.0'\nedition='2021'\n".to_vec()),
        ("source/cli/Cargo.toml".into(), b"[package]\nname='cli'\nversion.workspace=true\nedition='2021'\n[dependencies]\nlibrary={path='../library',version='^0.4'}\n".to_vec()),
        ("source/hook/Cargo.toml".into(), b"[package]\nname='hook'\nversion.workspace=true\nedition='2021'\n".to_vec()),
        ("Cargo.lock".into(), b"version=4\n[[package]]\nname='library'\nversion='0.4.0'\n[[package]]\nname='cli'\nversion='1.0.0'\ndependencies=['library']\n[[package]]\nname='hook'\nversion='1.0.0'\n".to_vec()),
    ]);
    (config, files)
}

#[test]
fn schema4_explicit_product_packages_share_a_workspace_with_independent_versions() {
    let (cfg, files) = mixed_product_native();
    let paths = files.keys().cloned().collect::<Vec<_>>();
    let snapshot = polyglot::snapshot(&cfg, paths.clone(), false, &mut |p| {
        Ok(files.get(p).cloned())
    })
    .unwrap();
    assert_eq!(snapshot.packages.len(), 1);
    assert_eq!(snapshot.packages["library"].version, "0.4.0");
    assert!(snapshot.dependencies.is_empty());
    let rust = cfg.adapters.rust.as_ref().unwrap();
    let inputs = rust::product_package_inputs(
        rust,
        &cfg.product_packages,
        &paths,
        &mut |p| Ok(files.get(p).cloned()),
        "1.0.0".parse().unwrap(),
    )
    .unwrap();
    assert!(inputs.contains_key("Cargo.lock"));
    assert!(inputs.contains_key("source/library/Cargo.toml"));
    assert!(
        rust::product_package_inputs(
            rust,
            &cfg.product_packages,
            &paths,
            &mut |p| Ok(files.get(p).cloned()),
            "1.0.1".parse().unwrap()
        )
        .is_err()
    );
}

#[test]
fn schema4_explicit_product_release_only_changes_carriers_and_their_lock_entries() {
    let (cfg, files) = mixed_product_native();
    let paths = files.keys().cloned().collect::<Vec<_>>();
    let writes = rust::render_product_packages(
        cfg.adapters.rust.as_ref().unwrap(),
        &cfg.product_packages,
        &paths,
        &mut |p| Ok(files.get(p).cloned()),
        "1.0.0".parse().unwrap(),
        "1.0.1".parse().unwrap(),
    )
    .unwrap();
    assert!(!writes.contains_key("source/library/Cargo.toml"));
    assert!(!writes.contains_key("source/cli/Cargo.toml"));
    assert!(!writes.contains_key("source/hook/Cargo.toml"));
    let workspace = std::str::from_utf8(&writes["Cargo.toml"])
        .unwrap()
        .parse::<toml_edit::DocumentMut>()
        .unwrap();
    assert_eq!(
        workspace["workspace"]["package"]["version"].as_str(),
        Some("1.0.1")
    );
    let snapshot = rust::snapshot(cfg.adapters.rust.as_ref().unwrap(), &paths, &mut |p| {
        Ok(writes.get(p).or_else(|| files.get(p)).cloned())
    })
    .unwrap();
    assert_eq!(
        snapshot.packages["source/library/Cargo.toml"].version,
        "0.4.0"
    );
    assert_eq!(snapshot.packages["source/cli/Cargo.toml"].version, "1.0.1");
    assert_eq!(snapshot.packages["source/hook/Cargo.toml"].version, "1.0.1");
}

#[test]
fn schema4_explicit_product_development_preserves_inheritance_and_updates_caller_requirement() {
    let (cfg, files) = mixed_product_native();
    let paths = files.keys().cloned().collect::<Vec<_>>();
    let writes = rust::render_versions_with_product_packages(
        cfg.adapters.rust.as_ref().unwrap(),
        &cfg.product_packages,
        &paths,
        &mut |p| Ok(files.get(p).cloned()),
        &BTreeMap::from([("source/library/Cargo.toml".into(), "0.5.0".into())]),
    )
    .unwrap();
    assert!(!writes.contains_key("Cargo.toml"));
    assert!(!writes.contains_key("source/hook/Cargo.toml"));
    let cli = std::str::from_utf8(&writes["source/cli/Cargo.toml"])
        .unwrap()
        .parse::<toml_edit::DocumentMut>()
        .unwrap();
    assert_eq!(cli["package"]["version"]["workspace"].as_bool(), Some(true));
    assert_eq!(
        cli["dependencies"]["library"]["version"].as_str(),
        Some("^0.5.0")
    );
    let snapshot = rust::snapshot(cfg.adapters.rust.as_ref().unwrap(), &paths, &mut |p| {
        Ok(writes.get(p).or_else(|| files.get(p)).cloned())
    })
    .unwrap();
    assert_eq!(
        snapshot.packages["source/library/Cargo.toml"].version,
        "0.5.0"
    );
    assert_eq!(snapshot.packages["source/cli/Cargo.toml"].version, "1.0.0");
}

#[test]
fn schema4_explicit_product_roles_reject_conflicts_missing_members_and_implicit_followers() {
    let (cfg, files) = mixed_product_native();
    let paths = files.keys().cloned().collect::<Vec<_>>();
    let mut value = json!({
        "schema_version":3,"release_policy":"explicit_release","strategy":"independent",
        "git_sync_setup":{"schema":1,"confirmed":true,"selection":"independent"},
        "adapters":{"rust":{"workspaces":["Cargo.toml"]}},
        "packages":[{"id":"library","name":"library","language":"rust","manifest":"source/library/Cargo.toml"}],
        "product_packages":[{"id":"library","name":"cli","language":"rust","manifest":"source/cli/Cargo.toml"}]
    });
    assert!(
        polyglot::parse(&serde_json::to_vec(&value).unwrap())
            .unwrap_err()
            .contains("disjoint")
    );
    value["product_packages"][0]["id"] = json!("cli");
    value["product_packages"][0]["manifest"] = json!("source/library/Cargo.toml");
    assert!(
        polyglot::parse(&serde_json::to_vec(&value).unwrap())
            .unwrap_err()
            .contains("disjoint")
    );
    let mut missing = cfg.clone();
    missing.product_packages[0].manifest = "source/missing/Cargo.toml".into();
    assert!(
        polyglot::snapshot(&missing, paths.clone(), false, &mut |p| Ok(files
            .get(p)
            .cloned()))
        .is_err()
    );
    let mut inherited = files.clone();
    inherited.insert(
        "source/library/Cargo.toml".into(),
        b"[package]\nname='library'\nversion.workspace=true\nedition='2021'\n".to_vec(),
    );
    inherited.insert(
        "Cargo.lock".into(),
        String::from_utf8(files["Cargo.lock"].clone())
            .unwrap()
            .replace("0.4.0", "1.0.0")
            .into_bytes(),
    );
    inherited.insert("source/cli/Cargo.toml".into(), b"[package]\nname='cli'\nversion.workspace=true\nedition='2021'\n[dependencies]\nlibrary={path='../library',version='^1'}\n".to_vec());
    assert!(
        rust::product_package_inputs(
            cfg.adapters.rust.as_ref().unwrap(),
            &cfg.product_packages,
            &paths,
            &mut |p| Ok(inherited.get(p).cloned()),
            "1.0.0".parse().unwrap()
        )
        .unwrap_err()
        .contains("must own its version")
    );
    let mut undeclared = cfg.clone();
    undeclared.product_packages.clear();
    assert!(
        polyglot::snapshot(&undeclared, paths, false, &mut |p| Ok(files
            .get(p)
            .cloned()))
        .unwrap_err()
        .contains("registered identity")
    );
}

fn mixed_product_repo() -> Repo {
    let r = fixture("unconfigured", true);
    fs::remove_dir_all(r.0.join("pkg")).unwrap();
    let (_, files) = mixed_product_native();
    for (path, bytes) in files {
        write(&r.0, &path, &bytes);
    }
    write(
        &r.0,
        "source/library/src/lib.rs",
        b"pub fn value()->u8{1}\n",
    );
    write(&r.0, "source/cli/src/main.rs", b"fn main(){}\n");
    write(&r.0, "source/hook/src/main.rs", b"fn main(){}\n");
    let cfg = json!({
        "schema_version":3,"release_policy":"explicit_release","strategy":"independent",
        "git_sync_setup":{"schema":1,"confirmed":true,"selection":"independent"},
        "adapters":{"rust":{"workspaces":["Cargo.toml"]}},
        "packages":[{"id":"library","name":"library","language":"rust","manifest":"source/library/Cargo.toml"}],
        "product_packages":[
            {"id":"cli","name":"cli","language":"rust","manifest":"source/cli/Cargo.toml"},
            {"id":"hook","name":"hook","language":"rust","manifest":"source/hook/Cargo.toml"}
        ],
        "product_inputs":["Cargo.toml","README.md",".gitignore","source/cli/","source/hook/"]
    });
    write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
    git(&r.0, &["add", "."]);
    git(&r.0, &["commit", "--amend", "--no-edit"]);
    migrate(&r.0);
    r
}

#[test]
fn schema4_explicit_product_migration_and_release_keep_independent_library_version() {
    let r = mixed_product_repo();
    let cfg = load(&r.0).unwrap().unwrap();
    assert_eq!(cfg.native.product_packages.len(), 2);
    assert_eq!(cfg.baseline.packages.len(), 1);
    assert_eq!(cfg.baseline.packages["library"].version, "0.4.0");
    let before = fs::read(r.0.join("source/library/Cargo.toml")).unwrap();
    let index = fs::read(r.0.join(".git/index")).unwrap();
    write(&r.0, "README.md", b"product documentation change\n");
    let request = analyze(&r.0, "patch", None);
    let plan = build(&r.0, "fix: product only", &request, &SystemProcessRunner)
        .unwrap()
        .unwrap();
    assert_eq!(plan.new_version.to_string(), "1.0.1");
    assert!(
        !plan
            .writes
            .contains_key(&r.0.join("source/library/Cargo.toml"))
    );
    assert!(plan.package_changes.is_empty());
    assert!(plan.writes.contains_key(&r.0.join("Cargo.toml")));
    assert_eq!(
        fs::read(r.0.join("source/library/Cargo.toml")).unwrap(),
        before
    );
    assert_eq!(fs::read(r.0.join(".git/index")).unwrap(), index);
}

#[test]
fn schema4_explicit_product_development_updates_product_caller_without_product_bump() {
    let r = mixed_product_repo();
    write(
        &r.0,
        "source/library/src/lib.rs",
        b"pub fn value()->u8{2}\npub fn added()->u8{3}\n",
    );
    fixture::close_packages(&r.0, &SystemProcessRunner, &[("library", "minor")]).unwrap();
    let library = fs::read_to_string(r.0.join("source/library/Cargo.toml"))
        .unwrap()
        .parse::<toml_edit::DocumentMut>()
        .unwrap();
    assert_eq!(library["package"]["version"].as_str(), Some("0.5.0"));
    let cli = fs::read_to_string(r.0.join("source/cli/Cargo.toml"))
        .unwrap()
        .parse::<toml_edit::DocumentMut>()
        .unwrap();
    assert_eq!(cli["package"]["version"]["workspace"].as_bool(), Some(true));
    assert_eq!(
        cli["dependencies"]["library"]["version"].as_str(),
        Some("^0.5.0")
    );
    assert_eq!(
        fs::read_to_string(r.0.join("VERSION")).unwrap().trim(),
        "1.0.0"
    );
    let ledger = development::Ledger::load(&r.0).unwrap();
    assert!(ledger.latest("library").is_some());
    assert!(ledger.latest("cli").is_none());
    assert!(ledger.latest("hook").is_none());
}

#[test]
fn schema4_formal_boundary_cannot_reintroduce_old_release_model() {
    let r = fixture("independent", true);
    let payload = fs::read(r.0.join(CONFIG)).unwrap();
    git(&r.0, &["add", "."]);
    git(&r.0, &["commit", "-m", "chore: cutover"]);
    let mut legacy: Value = serde_json::from_slice(&payload).unwrap();
    legacy.as_object_mut().unwrap().remove("release_model");
    write(&r.0, CONFIG, &serde_json::to_vec(&legacy).unwrap());
    write(&r.0, "VERSION", b"1.0.1\n");
    write(
        &r.0,
        "CHANGELOG.md",
        b"# Changelog\n\n## [1.0.1]\n\n- invalid old-model release\n",
    );
    git(&r.0, &["add", "."]);
    git(&r.0, &["commit", "-m", "chore: invalid release"]);
    write(&r.0, CONFIG, &payload);
    let error = build(&r.0, "fix: next", &Request::default(), &SystemProcessRunner).unwrap_err();
    assert!(
        error.contains("release history must preserve product-packages model"),
        "{error}"
    );
}

fn external_cargo_fixture() -> Repo {
    let r = fixture("unconfigured", true);
    write(&r.0, "pkg/Cargo.toml", b"[package]\nname='a'\nversion='1.0.0'\nedition='2021'\nbuild='../external/build.rs'\n[workspace]\n[lib]\npath='../external/lib.rs'\n[[bin]]\nname='inventory'\npath='../external/all_inventory.rs'\n[[test]]\nname='check'\npath='../external/check.rs'\n[[bench]]\nname='bench'\npath='../external/bench.rs'\n[[example]]\nname='example'\npath='../external/example.rs'\n");
    for name in ["lib", "build", "all_inventory", "check", "bench", "example"] {
        write(&r.0, &format!("external/{name}.rs"), b"fn main() {}\n");
    }
    git(&r.0, &["add", "."]);
    git(&r.0, &["commit", "--amend", "--no-edit"]);
    let proposal = fixture::current_fixture::proposal(&r.0).unwrap();
    let proposal = proposal.as_ref().map(|p| p.to_str().unwrap());
    let p = migration_plan(&r.0, &SystemProcessRunner, proposal);
    assert_eq!(p.code, 0, "{}", p.stderr);
    let v = p.receipt.unwrap();
    let a = migration_apply(
        &r.0,
        &SystemProcessRunner,
        proposal,
        v["fingerprint"].as_str().unwrap(),
        true,
    );
    assert_eq!(a.code, 0, "{}", a.stderr);
    r
}

fn legacy_source_roots(root: &Path) -> Value {
    let mut v: Value = serde_json::from_slice(&fs::read(root.join(CONFIG)).unwrap()).unwrap();
    v["baseline"]["packages"]["a"]["source_roots"] = json!(["pkg/"]);
    v["baseline"]["input_hashes"]
        .as_object_mut()
        .unwrap()
        .retain(|path, _| !path.starts_with("external/"));
    write(root, CONFIG, &serde_json::to_vec(&v).unwrap());
    v
}

fn historical_sharing_fixture() -> Repo {
    let r = fixture("unconfigured", true);
    write(&r.0, "pkg/src/gui.rs", b"pub fn gui() {}\n");
    write(&r.0, "pkg/src/helper.rs", b"pub fn helper() {}\n");
    write(&r.0, "tools/Cargo.toml", b"[package]\nname='b'\nversion='1.0.0'\nedition='2021'\n[workspace]\n[[test]]\nname='shared'\npath='../checks/main.rs'\n");
    write(
        &r.0,
        "tools/Cargo.lock",
        b"version=4\n[[package]]\nname='b'\nversion='1.0.0'\n",
    );
    write(
        &r.0,
        "checks/main.rs",
        b"#[path=\"../pkg/src/gui.rs\"] mod gui;\n#[path=\"../pkg/src/helper.rs\"] mod helper;\n",
    );
    let mut cfg: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    cfg["adapters"]["rust"]["workspaces"]
        .as_array_mut()
        .unwrap()
        .push(json!("tools/Cargo.toml"));
    cfg["packages"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"b","name":"b","language":"rust","manifest":"tools/Cargo.toml"}));
    cfg["product_inputs"]
        .as_array_mut()
        .unwrap()
        .push(json!("tools/Cargo.lock"));
    write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
    git(&r.0, &["add", "."]);
    git(&r.0, &["commit", "--amend", "--no-edit"]);
    let proposal = fixture::current_fixture::proposal(&r.0).unwrap();
    let proposal = proposal.as_ref().map(|p| p.to_str().unwrap());
    let p = migration_plan(&r.0, &SystemProcessRunner, proposal);
    assert_eq!(p.code, 0, "{}", p.stderr);
    let v = p.receipt.unwrap();
    assert_eq!(
        v["ownership_analysis"]["shared_input_additions"]["historical"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        migration_apply(
            &r.0,
            &SystemProcessRunner,
            proposal,
            v["fingerprint"].as_str().unwrap(),
            true
        )
        .code,
        0
    );
    let migrated = load(&r.0).unwrap().unwrap();
    let captured = capture(&r.0, &migrated, &SystemProcessRunner).unwrap();
    assert!(
        !captured.changes.iter().any(|c| c.path == CONFIG),
        "inferred sharing alone is migration bookkeeping"
    );
    let mut changed: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    changed["build_inputs"] = json!(["scripts/recipe.json"]);
    assert!(
        !migration::is_initial_configuration(&changed, &migrated, &captured.current).unwrap(),
        "sharing must not conceal other configuration changes"
    );
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
    write(
        root,
        "checks/main.rs",
        b"#[path=\"../pkg/src/gui.rs\"] mod gui;\n",
    );
    let p = repair_baseline_plan(root, &SystemProcessRunner);
    assert_eq!(p.code, 0, "{}", p.stderr);
    let p = p.receipt.unwrap();
    let additions = &p["ownership_analysis"]["shared_input_additions"];
    assert_eq!(additions["historical"].as_array().unwrap().len(), 2);
    assert_eq!(additions["current"].as_array().unwrap().len(), 1);
    assert_eq!(additions["current"][0]["path"], "pkg/src/gui.rs");
    assert_eq!(additions["historical"][0]["packages"], json!(["a", "b"]));
    assert!(
        additions["historical"][0]["content_sha256"]
            .as_str()
            .unwrap()
            .starts_with("sha256:")
    );
    assert_eq!(
        additions["historical"][0]["evidence"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(fs::read(root.join(CONFIG)).unwrap(), before);
    let fp = p["fingerprint"].as_str().unwrap();
    assert_ne!(
        repair_baseline_apply(root, &SystemProcessRunner, fp, false).code,
        0
    );
    write(root, "pkg/src/gui.rs", b"pub fn changed_after_plan() {}\n");
    assert_ne!(
        repair_baseline_apply(root, &SystemProcessRunner, fp, true).code,
        0
    );
    assert_eq!(fs::read(root.join(CONFIG)).unwrap(), before);
    let p = repair_baseline_plan(root, &SystemProcessRunner)
        .receipt
        .unwrap();
    let applied = repair_baseline_apply(
        root,
        &SystemProcessRunner,
        p["fingerprint"].as_str().unwrap(),
        true,
    );
    assert_eq!(applied.code, 0, "{}", applied.stderr);
    let repaired: Value = serde_json::from_slice(&fs::read(root.join(CONFIG)).unwrap()).unwrap();
    let mut normalized = repaired.clone();
    normalized["baseline"]["shared_inputs"] = original["baseline"]["shared_inputs"].clone();
    normalized["shared_inputs"] = original["shared_inputs"].clone();
    assert_eq!(
        normalized, original,
        "only missing shared declarations may change"
    );
    assert_eq!(fs::read(root.join(".git/index")).unwrap(), index);
    assert_eq!(git(root, &["rev-parse", "HEAD"]), head);
    assert_eq!(fs::read(root.join("VERSION")).unwrap(), b"1.0.0\n");
    let cfg = load(root).unwrap().unwrap();
    let c = capture(root, &cfg, &SystemProcessRunner).unwrap();
    assert_eq!(
        polyglot::owner(&c.baseline, "pkg/src/helper.rs", &cfg.native).unwrap(),
        vec!["a", "b"]
    );
    assert_eq!(
        polyglot::owner(&c.current, "pkg/src/helper.rs", &cfg.native).unwrap(),
        vec!["a"]
    );
    assert_eq!(
        c.changes
            .iter()
            .find(|c| c.path == "pkg/src/gui.rs")
            .unwrap()
            .packages,
        vec!["a", "b"]
    );
    let request = analyze(root, "patch", Some("patch"));
    let release = build(root, "fix: shared sources", &request, &SystemProcessRunner)
        .unwrap()
        .unwrap();
    assert_eq!(release.old_version.to_string(), "1.0.0");
    let second = repair_baseline_plan(root, &SystemProcessRunner)
        .receipt
        .unwrap();
    assert_eq!(
        second["configuration"]["baseline"], repaired["baseline"],
        "repair must preserve the baseline after development policy configuration"
    );
    apply_file_release_plan(&release).unwrap();
    git(root, &["add", "."]);
    git(root, &["commit", "-m", "fix: fixture release"]);
    let published = fs::read(root.join(CONFIG)).unwrap();
    assert!(
        repair_baseline_plan(root, &SystemProcessRunner)
            .stderr
            .contains("published migration baseline cannot be repaired")
    );
    assert_eq!(fs::read(root.join(CONFIG)).unwrap(), published);
}

#[test]
fn schema4_history_prefetch_does_not_load_unregistered_business_blobs() {
    use std::cell::RefCell;
    struct Counting(RefCell<Vec<Vec<u8>>>);
    impl ProcessRunner for Counting {
        fn run(&self, request: &ProcessRequest) -> std::io::Result<crate::ProcessOutput> {
            if request.args.iter().any(|arg| arg == "cat-file") {
                self.0.borrow_mut().push(request.stdin.clone());
            }
            SystemProcessRunner.run(request)
        }
    }
    let r = fixture("independent", true);
    write(&r.0, "business.bin", &[42; 65536]);
    git(&r.0, &["add", "."]);
    git(
        &r.0,
        &["commit", "-m", "chore: unrelated business resource"],
    );
    let head = git(&r.0, &["rev-parse", "HEAD"]);
    let blob = git(&r.0, &["rev-parse", "HEAD:business.bin"]);
    let cfg = load(&r.0).unwrap().unwrap();
    let runner = Counting(RefCell::new(Vec::new()));
    let reader = history_cache::Reader::new(&r.0, &runner);
    prefetch_history(&cfg, &head, &reader).unwrap();
    assert!(!runner.0.borrow().iter().any(|bytes| {
        String::from_utf8_lossy(bytes)
            .lines()
            .any(|oid| oid == blob)
    }));
    let count = runner.0.borrow().len();
    prefetch_history(&cfg, &head, &reader).unwrap();
    assert_eq!(runner.0.borrow().len(), count);
    reader.verify_unchanged().unwrap();
}

#[test]
fn schema4_reused_classifier_binds_current_contract_bytes() {
    let r = fixture("independent", false);
    let cfg = load(&r.0).unwrap().unwrap();
    let bookkeeping = capture(&r.0, &cfg, &SystemProcessRunner).unwrap();
    assert!(
        !bookkeeping
            .inputs
            .contains_key(&r.0.join(".codex/managed-skeleton.json")),
        "pure migration bookkeeping does not construct a classifier"
    );
    change(&r.0, false);
    let captured = capture(&r.0, &cfg, &SystemProcessRunner).unwrap();
    let contract = r.0.join(".codex/managed-skeleton.json");
    assert!(captured.inputs.contains_key(&contract));
    write(
        &r.0,
        ".codex/managed-skeleton.json",
        br#"{"schema_version":1,"release_version":"8.0.0","assets":[],"changed":true}"#,
    );
    assert!(verify_release_inputs(&captured.inputs).is_err());
}

#[test]
fn schema4_reused_classifier_rejects_contract_drift_before_its_path_is_captured() {
    struct ChangeContract {
        spec: String,
        changed: std::cell::Cell<bool>,
    }
    impl ProcessRunner for ChangeContract {
        fn run(&self, request: &ProcessRequest) -> std::io::Result<crate::ProcessOutput> {
            let output = SystemProcessRunner.run(request)?;
            if !self.changed.get()
                && request.args.iter().any(|a| a == "show")
                && request
                    .args
                    .iter()
                    .any(|a| a.to_string_lossy() == self.spec)
            {
                fs::write(
                    request.cwd.join(".codex/managed-skeleton.json"),
                    br#"{"schema_version":1,"release_version":"8.0.0","assets":[],"note":"B"}"#,
                )?;
                self.changed.set(true);
            }
            Ok(output)
        }
    }
    let r = fixture("independent", false);
    let boundary = git(&r.0, &["rev-parse", "HEAD"]);
    git(&r.0, &["add", "."]);
    git(
        &r.0,
        &["commit", "-m", "chore: commit migration before drift probe"],
    );
    // Disable only the optional object cache. History interpretation remains
    // real and verified; grafts are not an acceptable test cache switch.
    let mut value: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    value["build_inputs"] = json!(["recipe.json"]);
    write(&r.0, CONFIG, &serde_json::to_vec(&value).unwrap());
    write(
        &r.0,
        ".codex/managed-skeleton.json",
        br#"{"schema_version":1,"release_version":"8.0.0","assets":[],"note":"A"}"#,
    );
    let runner = ChangeContract {
        spec: format!("{boundary}:.codex/managed-skeleton.json"),
        changed: std::cell::Cell::new(false),
    };
    let reader = history_cache::Reader::uncached(&r.0, &runner);
    let error = inputs::capture_release(&r.0, &load(&r.0).unwrap().unwrap(), &reader)
        .err()
        .unwrap();
    reader.verify_unchanged().unwrap();
    assert!(runner.changed.get());
    assert!(error.contains("ownership contract changed"), "{error}");
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
    let mut snapshot = polyglot::historical(
        &r.0,
        &cfg.baseline.revision,
        &cfg.native,
        &SystemProcessRunner,
    )
    .unwrap();
    let additions = polyglot::proven_shared_inputs(&snapshot, &cfg.native);
    assert_eq!(additions.len(), 2);
    snapshot
        .config
        .as_mut()
        .unwrap()
        .shared_inputs
        .push(polyglot::SharedInput {
            path: "pkg/src/".into(),
            packages: vec!["a".into()],
        });
    assert!(polyglot::proven_shared_inputs(&snapshot, &cfg.native).is_empty());
    snapshot.config.as_mut().unwrap().shared_inputs.clear();
    snapshot
        .config
        .as_mut()
        .unwrap()
        .product_inputs
        .push("pkg/src/".into());
    assert!(polyglot::proven_shared_inputs(&snapshot, &cfg.native).is_empty());
    snapshot
        .config
        .as_mut()
        .unwrap()
        .product_inputs
        .retain(|p| p != "pkg/src/");
    snapshot.inputs.remove("pkg/src/gui.rs");
    assert!(
        !polyglot::proven_shared_inputs(&snapshot, &cfg.native)
            .iter()
            .any(|m| m.path == "pkg/src/gui.rs")
    );
    snapshot.packages.get_mut("a").unwrap().language = Language::Python;
    assert!(polyglot::proven_shared_inputs(&snapshot, &cfg.native).is_empty());
    write(
        &r.0,
        "checks/main.rs",
        b"include!(env!(\"SHARED_SOURCE\"));\n",
    );
    let current = polyglot::snapshot(
        &cfg.native,
        independent::inventory(&r.0, &SystemProcessRunner).unwrap(),
        false,
        &mut |p| config::read_current(&r.0, p),
    )
    .unwrap();
    assert!(
        polyglot::proven_shared_inputs(&current, &cfg.native).is_empty(),
        "opaque includes must not invent consumers"
    );
}

#[test]
fn schema4_external_cargo_targets_survive_moves_without_claiming_siblings() {
    let r = external_cargo_fixture();
    let cfg = load(&r.0).unwrap().unwrap();
    assert_eq!(cfg.baseline.packages["a"].source_roots.len(), 7);
    fs::rename(
        r.0.join("external/all_inventory.rs"),
        r.0.join("pkg/src/inventory.rs"),
    )
    .unwrap();
    let manifest = fs::read_to_string(r.0.join("pkg/Cargo.toml"))
        .unwrap()
        .replace("../external/all_inventory.rs", "src/inventory.rs");
    write(&r.0, "pkg/Cargo.toml", manifest.as_bytes());
    let c = capture(&r.0, &cfg, &SystemProcessRunner).unwrap();
    let old = c
        .changes
        .iter()
        .find(|c| c.path == "external/all_inventory.rs")
        .unwrap();
    assert_eq!(old.packages, vec!["a"]);
    write(&r.0, "external/unrelated.rs", b"fn unrelated() {}\n");
    let e = capture(&r.0, &cfg, &SystemProcessRunner).err().unwrap();
    assert!(
        e.contains("unattributed release input: external/unrelated.rs"),
        "{e}"
    );
}

#[test]
fn schema4_opaque_include_does_not_claim_unknown_changed_file() {
    let r = external_cargo_fixture();
    let cfg = load(&r.0).unwrap().unwrap();
    write(
        &r.0,
        "external/lib.rs",
        b"include!(env!(\"SOURCE_FILE\"));\n",
    );
    write(&r.0, "unknown/generated.rs", b"pub fn generated(){}\n");
    let error = capture(&r.0, &cfg, &SystemProcessRunner).err().unwrap();
    assert!(
        error.contains("unattributed release input: unknown/generated.rs"),
        "{error}"
    );
}

#[test]
fn schema4_missing_external_target_keeps_only_declared_ownership_without_hash() {
    let r = external_cargo_fixture();
    let cfg = load(&r.0).unwrap().unwrap();
    let manifest = format!(
        "{}\n[[test]]\nname='removed'\npath='../external/missing.rs'\n",
        fs::read_to_string(r.0.join("pkg/Cargo.toml")).unwrap()
    );
    write(&r.0, "pkg/Cargo.toml", manifest.as_bytes());
    let c = capture(&r.0, &cfg, &SystemProcessRunner).unwrap();
    assert!(
        c.current.packages["a"]
            .source_roots
            .contains(&"external/missing.rs".into())
    );
    assert!(!c.current.inputs.contains_key("external/missing.rs"));
    assert_eq!(
        polyglot::owner(&c.current, "external/missing.rs", &cfg.native).unwrap(),
        vec!["a"]
    );
    assert!(
        polyglot::owner(&c.current, "external/missing.rs/unrelated.txt", &cfg.native)
            .unwrap_err()
            .contains("unattributed")
    );
    write(&r.0, "external/missing.rs", b"mod restored;\n");
    write(&r.0, "external/restored.rs", b"pub fn restored() {}\n");
    let c = capture(&r.0, &cfg, &SystemProcessRunner).unwrap();
    assert!(
        c.current.packages["a"]
            .source_roots
            .contains(&"external/restored.rs".into())
    );
    assert_eq!(
        c.current.inputs["external/missing.rs"],
        Some(b"mod restored;\n".to_vec())
    );
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
    let blocked = preview(
        &r.0,
        "fix: release",
        &Request::default(),
        &SystemProcessRunner,
    );
    assert!(
        blocked.stderr.contains("--repair-baseline-plan"),
        "{}",
        blocked.stderr
    );
    let p = repair_baseline_plan(&r.0, &SystemProcessRunner);
    assert_eq!(p.code, 0, "{}", p.stderr);
    let v = p.receipt.unwrap();
    assert_eq!(v["status"], "baseline-repair-planned");
    assert_eq!(fs::read(r.0.join(CONFIG)).unwrap(), config);
    assert_eq!(fs::read(r.0.join(".git/index")).unwrap(), index);
    let fp = v["fingerprint"].as_str().unwrap();
    assert_ne!(
        repair_baseline_apply(&r.0, &SystemProcessRunner, fp, false).code,
        0
    );
    assert_eq!(fs::read(r.0.join(CONFIG)).unwrap(), config);
    // A previously missing static reference appearing after planning must
    // invalidate the inventory even though it has no old input hash.
    write(&r.0, "external/lib.rs", b"mod restored;\n");
    let missing_plan = repair_baseline_plan(&r.0, &SystemProcessRunner)
        .receipt
        .unwrap();
    write(&r.0, "external/restored.rs", b"pub fn restored(){}\n");
    assert_ne!(
        repair_baseline_apply(
            &r.0,
            &SystemProcessRunner,
            missing_plan["fingerprint"].as_str().unwrap(),
            true
        )
        .code,
        0
    );
    assert_eq!(fs::read(r.0.join(CONFIG)).unwrap(), config);
    fs::remove_file(r.0.join("external/restored.rs")).unwrap();
    write(&r.0, "external/lib.rs", b"fn main() {}\n");
    write(&r.0, "README.md", b"changed after plan\n");
    assert_ne!(
        repair_baseline_apply(&r.0, &SystemProcessRunner, fp, true).code,
        0
    );
    assert_eq!(fs::read(r.0.join(CONFIG)).unwrap(), config);
    let p = repair_baseline_plan(&r.0, &SystemProcessRunner)
        .receipt
        .unwrap();
    let a = repair_baseline_apply(
        &r.0,
        &SystemProcessRunner,
        p["fingerprint"].as_str().unwrap(),
        true,
    );
    assert_eq!(a.code, 0, "{}", a.stderr);
    let mut repaired: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    assert_eq!(
        repaired["baseline"]["packages"]["a"]["source_roots"]
            .as_array()
            .unwrap()
            .len(),
        7
    );
    repaired["baseline"]["packages"]["a"]["source_roots"] = json!(["pkg/"]);
    repaired["baseline"]["input_hashes"]
        .as_object_mut()
        .unwrap()
        .retain(|path, _| !path.starts_with("external/"));
    assert_eq!(repaired, old);
    assert_eq!(git(&r.0, &["rev-parse", "HEAD"]), head);
    assert_eq!(fs::read(r.0.join(".git/index")).unwrap(), index);
    assert_eq!(fs::read(r.0.join("pkg/Cargo.lock")).unwrap(), lock);
    assert_eq!(fs::read(r.0.join("VERSION")).unwrap(), b"1.0.0\n");
    assert_eq!(
        preview(
            &r.0,
            "fix: release",
            &Request::default(),
            &SystemProcessRunner
        )
        .code,
        0
    );
    let request = analyze(&r.0, "patch", None);
    let release = build(&r.0, "fix: product", &request, &SystemProcessRunner)
        .unwrap()
        .unwrap();
    apply_file_release_plan(&release).unwrap();
    git(&r.0, &["add", "."]);
    git(&r.0, &["commit", "-m", "fix: product"]);
    let published = fs::read(r.0.join(CONFIG)).unwrap();
    let blocked = repair_baseline_plan(&r.0, &SystemProcessRunner);
    assert!(
        blocked
            .stderr
            .contains("published migration baseline cannot be repaired"),
        "{}",
        blocked.stderr
    );
    assert_eq!(fs::read(r.0.join(CONFIG)).unwrap(), published);
}

#[test]
fn schema4_baseline_repair_rejects_forged_facts_and_ownership() {
    for field in [
        "version",
        "source_roots",
        "changed_hash",
        "removed_hash",
        "invented_hash",
    ] {
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
            "invented_hash" => {
                v["baseline"]["input_hashes"]["external/unrelated.rs"] = json!("sha256:forged")
            }
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
    let mut snapshot =
        baseline_snapshot(&r.0, &cfg, &cfg.baseline.revision, &SystemProcessRunner).unwrap();
    let mut second = snapshot.packages["a"].clone();
    second.manifest = "other/Cargo.toml".into();
    snapshot.packages.insert("b".into(), second);
    assert!(
        polyglot::owner(&snapshot, "external/lib.rs", &cfg.native)
            .unwrap_err()
            .contains("ambiguous")
    );
    assert!(
        polyglot::owner(&snapshot, "external/lib.rs/unrelated.txt", &cfg.native)
            .unwrap_err()
            .contains("unattributed")
    );
    snapshot.packages.get_mut("b").unwrap().source_roots = vec!["external/".into()];
    assert!(
        polyglot::owner(&snapshot, "external/lib.rs", &cfg.native)
            .unwrap_err()
            .contains("ambiguous")
    );
    // Python explicit version sources retain their existing precedence in the
    // absence of Cargo external declarations; a cross-language collision does not.
    for package in snapshot.packages.values_mut() {
        package.language = Language::Python;
    }
    assert_eq!(
        polyglot::owner(&snapshot, "external/lib.rs", &cfg.native).unwrap(),
        vec!["a"]
    );
    snapshot.packages.get_mut("b").unwrap().language = Language::Rust;
    snapshot.packages.get_mut("b").unwrap().source_roots = vec!["external/lib.rs".into()];
    assert!(
        polyglot::owner(&snapshot, "external/lib.rs", &cfg.native)
            .unwrap_err()
            .contains("ambiguous")
    );
    let text = fs::read_to_string(r.0.join("pkg/Cargo.toml"))
        .unwrap()
        .replace("../external/lib.rs", "../../outside.rs");
    write(&r.0, "pkg/Cargo.toml", text.as_bytes());
    assert_ne!(
        preview(
            &r.0,
            "fix: release",
            &Request::default(),
            &SystemProcessRunner
        )
        .code,
        0
    );
}

#[test]
fn schema4_inline_cargo_targets_keep_the_same_ownership() {
    let r = external_cargo_fixture();
    let cfg = load(&r.0).unwrap().unwrap();
    let manifest = fs::read_to_string(r.0.join("pkg/Cargo.toml")).unwrap();
    let manifest = format!(
        "bin = [{{name='inventory',path='../external/all_inventory.rs'}}]\n{}",
        manifest.replace(
            "[[bin]]\nname='inventory'\npath='../external/all_inventory.rs'\n",
            ""
        )
    );
    write(&r.0, "pkg/Cargo.toml", manifest.as_bytes());
    let c = capture(&r.0, &cfg, &SystemProcessRunner).unwrap();
    assert_eq!(
        c.current.packages["a"].source_roots,
        cfg.baseline.packages["a"].source_roots
    );
}

#[test]
fn schema4_external_lib_module_history_is_repaired_before_module_move_release() {
    for exact_entry_already_recorded in [false, true] {
        let r = fixture("unconfigured", true);
        write(&r.0, "pkg/Cargo.toml", b"[package]\nname='a'\nversion='1.0.0'\nedition='2021'\n[workspace]\n[lib]\npath='../legacy/lib.rs'\n");
        write(&r.0, "legacy/lib.rs", b"pub mod all_inventory;\n");
        write(
            &r.0,
            "legacy/all_inventory.rs",
            b"#[cfg(test)]\n#[path=\"all_inventory_tests.rs\"] mod tests; pub fn value()->u8{1}\n",
        );
        write(
            &r.0,
            "legacy/all_inventory_tests.rs",
            b"#[test] fn example(){}\n",
        );
        write(&r.0, "legacy/unrelated.rs", b"pub fn unrelated(){}\n");
        git(&r.0, &["add", "."]);
        git(&r.0, &["commit", "--amend", "--no-edit"]);
        let proposal = fixture::current_fixture::proposal(&r.0).unwrap();
        let proposal = proposal.as_ref().map(|p| p.to_str().unwrap());
        let p = migration_plan(&r.0, &SystemProcessRunner, proposal);
        assert_eq!(p.code, 0, "{}", p.stderr);
        let p = p.receipt.unwrap();
        assert_eq!(
            migration_apply(
                &r.0,
                &SystemProcessRunner,
                proposal,
                p["fingerprint"].as_str().unwrap(),
                true
            )
            .code,
            0
        );
        let mut old: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
        old["baseline"]["packages"]["a"]["source_roots"] = if exact_entry_already_recorded {
            json!(["pkg/", "legacy/lib.rs"])
        } else {
            json!(["pkg/"])
        };
        old["baseline"]["input_hashes"]
            .as_object_mut()
            .unwrap()
            .retain(|path, _| !path.starts_with("legacy/"));
        write(&r.0, CONFIG, &serde_json::to_vec(&old).unwrap());
        git(&r.0, &["add", "."]);
        git(&r.0, &["commit", "-m", "chore: old metadata migration"]);

        for name in ["lib.rs", "all_inventory.rs", "all_inventory_tests.rs"] {
            fs::rename(
                r.0.join("legacy").join(name),
                r.0.join("pkg/src").join(name),
            )
            .unwrap();
        }
        let manifest = fs::read_to_string(r.0.join("pkg/Cargo.toml"))
            .unwrap()
            .replace("../legacy/lib.rs", "src/lib.rs");
        write(&r.0, "pkg/Cargo.toml", manifest.as_bytes());
        let config_before = fs::read(r.0.join(CONFIG)).unwrap();
        let index = fs::read(r.0.join(".git/index")).unwrap();
        let planned = repair_baseline_plan(&r.0, &SystemProcessRunner);
        assert_eq!(planned.code, 0, "{}", planned.stderr);
        let planned = planned.receipt.unwrap();
        let baseline = &planned["configuration"]["baseline"];
        assert_eq!(baseline["revision"], old["baseline"]["revision"]);
        assert!(
            baseline["packages"]["a"]["source_roots"]
                .as_array()
                .unwrap()
                .contains(&json!("legacy/all_inventory.rs"))
        );
        assert_eq!(baseline["input_hashes"]["legacy/all_inventory.rs"], json!(digest(b"#[cfg(test)]\n#[path=\"all_inventory_tests.rs\"] mod tests; pub fn value()->u8{1}\n")));
        assert!(
            !baseline["packages"]["a"]["source_roots"]
                .as_array()
                .unwrap()
                .contains(&json!("legacy/unrelated.rs"))
        );
        assert_eq!(fs::read(r.0.join(CONFIG)).unwrap(), config_before);
        assert_eq!(
            repair_baseline_apply(
                &r.0,
                &SystemProcessRunner,
                planned["fingerprint"].as_str().unwrap(),
                true
            )
            .code,
            0
        );
        assert_eq!(fs::read(r.0.join(".git/index")).unwrap(), index);
        assert_eq!(fs::read(r.0.join("VERSION")).unwrap(), b"1.0.0\n");
        let cfg = load(&r.0).unwrap().unwrap();
        let captured = capture(&r.0, &cfg, &SystemProcessRunner).unwrap();
        for path in ["legacy/all_inventory.rs", "pkg/src/all_inventory.rs"] {
            assert_eq!(
                captured
                    .changes
                    .iter()
                    .find(|c| c.path == path)
                    .unwrap()
                    .packages,
                vec!["a"]
            );
        }
    }
}
fn analyze(root: &Path, product: &str, package: Option<&str>) -> Request {
    if let Some(level) = package {
        fixture::close_packages(root, &SystemProcessRunner, &[("a", level)]).unwrap();
    }
    let cfg = load(root).unwrap().unwrap();
    let inspection = InspectionRunner(&SystemProcessRunner);
    let reader = history_cache::Reader::new(root, &inspection);
    let mut captured = inputs::capture_release(root, &cfg, &reader).unwrap();
    // Match the public release preview, including normalization rules. Some
    // callers deliberately leave packages open to assert build's gate error.
    let _ = development::release_gate(root, &cfg, &mut captured, &SystemProcessRunner);
    write(root,".runtime/analysis.json",&serde_json::to_vec(&json!({"schema":1,"fingerprint":captured.fingerprint,"product":{"level":product,"reason":"actual final release changes","compatibility":"compatible"}})).unwrap());
    Request {
        analysis_file: Some(PathBuf::from(".runtime/analysis.json")),
    }
}
#[test]
fn schema4_single_model_uses_both_language_adapters() {
    for strategy in ["independent"] {
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
            assert_eq!(p.package_changes[0].version_after, "1.0.1");
            assert_eq!(fs::read(r.0.join(".git/index")).unwrap(), before);
            assert_eq!(fs::read(r.0.join("VERSION")).unwrap(), b"1.0.0\n");
        }
    }
}
#[test]
fn schema4_migration_preview_is_zero_write_and_apply_rejects_drift() {
    let r = fixture("unconfigured", false);
    let proposal = fixture::current_fixture::proposal(&r.0).unwrap();
    let proposal = proposal.as_ref().map(|p| p.to_str().unwrap());
    let before = fs::read(r.0.join(CONFIG)).ok();
    let index = fs::read(r.0.join(".git/index")).unwrap();
    let p = migration_plan(&r.0, &SystemProcessRunner, proposal);
    assert_eq!(p.code, 0, "{}", p.stderr);
    assert_eq!(fs::read(r.0.join(CONFIG)).ok(), before);
    assert_eq!(fs::read(r.0.join(".git/index")).unwrap(), index);
    write(
        &r.0,
        "pkg/pyproject.toml",
        b"[project]\nname='a'\nversion='1.0.1'\n",
    );
    let a = migration_apply(
        &r.0,
        &SystemProcessRunner,
        proposal,
        p.receipt.unwrap()["fingerprint"].as_str().unwrap(),
        true,
    );
    assert_ne!(a.code, 0);
    assert_eq!(fs::read(r.0.join(CONFIG)).ok(), before);
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
    assert!(
        build(
            &r.0,
            "fix: restored",
            &Request::default(),
            &SystemProcessRunner
        )
        .unwrap()
        .is_none()
    );
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
    assert!(
        build(&r.0, "fix: change", &request, &SystemProcessRunner)
            .unwrap_err()
            .contains("development closure required")
    );
    let request = analyze(&r.0, "patch", Some("patch"));
    write(&r.0, "README.md", b"new product bytes\n");
    assert!(
        build(&r.0, "fix: change", &request, &SystemProcessRunner)
            .unwrap_err()
            .contains("stale")
    );
}

#[test]
fn schema4_explicit_targets_reject_lower_and_unchanged_package_bumps() {
    let r = fixture("independent", false);
    change(&r.0, false);
    let request = analyze(&r.0, "minor", Some("minor"));
    let mut v = analysis_value(&r.0);
    v["targets"] = json!({"product":"1.0.1"});
    save_analysis(&r.0, &v);
    assert!(
        build(&r.0, "feat: change", &request, &SystemProcessRunner)
            .unwrap_err()
            .contains("below suggested")
    );
    v["targets"] = json!({"a":"1.2.0"});
    save_analysis(&r.0, &v);
    assert!(
        build(&r.0, "feat: change", &request, &SystemProcessRunner)
            .unwrap_err()
            .contains("release only analyzes product")
    );
    write(&r.0, "pkg/a.py", b"VALUE=1\n");
    assert!(
        build(
            &r.0,
            "fix: reverted after closure",
            &request,
            &SystemProcessRunner
        )
        .unwrap_err()
        .contains("development closure required")
    );
}

fn migrate(root: &Path) {
    super::fixture::migrate(root, &SystemProcessRunner).unwrap();
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
        let r = fixture("unconfigured", false);
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
        let overrides = if pinned {
            vec![("a", "patch"), ("b", "patch")]
        } else {
            vec![("a", "patch")]
        };
        fixture::close_packages(&r.0, &SystemProcessRunner, &overrides).unwrap();
        let request = analyze(&r.0, "patch", None);
        let p = build(&r.0, "fix: a", &request, &SystemProcessRunner)
            .unwrap()
            .unwrap();
        assert_eq!(p.package_changes.len(), if pinned { 2 } else { 1 });
        assert!(!p.writes.contains_key(&r.0.join("consumer/pyproject.toml")));
        let native = fs::read_to_string(r.0.join("consumer/pyproject.toml")).unwrap();
        if pinned {
            assert!(native.contains("1.0.1"));
        } else {
            // Dependency evidence is now actually renewed even for an old
            // package with no certificate; that must not invent a version bump.
            let ledger = development::Ledger::load(&r.0).unwrap();
            let entry = ledger.latest("b").unwrap();
            assert_eq!(entry.level, 0);
            assert_eq!(entry.before.as_deref(), Some("1.0.0"));
            assert_eq!(entry.after, entry.before);
            assert!(entry.checks.iter().all(|check| check["status"] == "passed"));
        }
    }
}

#[test]
fn schema4_mixed_languages_keep_per_package_impact() {
    let r = fixture("unconfigured", true);
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
    fixture::close_packages(
        &r.0,
        &SystemProcessRunner,
        &[("a", "patch"), ("p", "minor")],
    )
    .unwrap();
    let request = analyze(&r.0, "minor", None);
    let p = build(&r.0, "feat: mixed release", &request, &SystemProcessRunner)
        .unwrap()
        .unwrap();
    assert_eq!(p.new_version.to_string(), "1.1.0");
    assert!(
        p.package_changes
            .iter()
            .any(|p| p.name == "a" && p.version_after == "1.0.1")
    );
    assert!(
        p.package_changes
            .iter()
            .any(|p| p.name == "p" && p.version_after == "1.1.0")
    );
}

#[test]
fn schema4_stratus_style_build_recipe_and_ledger_do_not_bump_package() {
    let r = fixture("independent", false);
    let mut cfg: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    cfg["record_inputs"] = json!(["pkg/versions.json"]);
    cfg["build_inputs"] = json!(["scripts/build-recipes/common.json"]);
    write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
    write(&r.0, "pkg/versions.json", b"{\"template-one\":7}\n");
    fixture::close_packages(&r.0, &SystemProcessRunner, &[]).unwrap();
    let config_request = analyze(&r.0, "patch", None);
    assert!(
        build(
            &r.0,
            "chore: roles and record",
            &config_request,
            &SystemProcessRunner
        )
        .unwrap()
        .unwrap()
        .package_changes
        .is_empty()
    );
    write(
        &r.0,
        "scripts/build-recipes/common.json",
        b"{\"opt-level\":3}\n",
    );
    fixture::close_packages(&r.0, &SystemProcessRunner, &[]).unwrap();
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
    let r = fixture("independent", true);
    change(&r.0, true);
    development_tests::configure(&r.0, false);
    let runner = development_tests::Checks::new();
    let request = development_tests::request(&r.0, "major", false, &runner);
    let path = r.0.join(".runtime/develop.json");
    let mut v: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    v["packages"]["a"]["compatibility"] = json!("breaking");
    fs::write(&path, serde_json::to_vec(&v).unwrap()).unwrap();
    assert!(
        development::preview(&r.0, &request, &runner)
            .stderr
            .contains("explicit user acceptance")
    );
    v["packages"]["a"]["acceptance"] = json!({"fingerprint":v["fingerprint"],"scope":["a"],"reason":"accept migration","source":"fixture user decision"});
    fs::write(&path, serde_json::to_vec(&v).unwrap()).unwrap();
    assert_eq!(development::preview(&r.0, &request, &runner).code, 0);
    v["packages"]["a"]["compatibility"] = json!("unknown");
    fs::write(&path, serde_json::to_vec(&v).unwrap()).unwrap();
    assert!(
        development::preview(&r.0, &request, &runner)
            .stderr
            .contains("unknown")
    );
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
    assert!(
        build(&r.0, "fix: change", &request, &SystemProcessRunner)
            .unwrap_err()
            .contains("required release evidence missing")
    );
    let mut v = analysis_value(&r.0);
    let validation = capture(&r.0, &load(&r.0).unwrap().unwrap(), &SystemProcessRunner)
        .unwrap()
        .validation_fingerprint;
    let proof = json!({"id":"unit","status":"passed","fingerprint":validation,"scope":["product","a"],"reason":"fixture input-bound validation"});
    let bytes = serde_json::to_vec(&proof).unwrap();
    write(&r.0, ".runtime/evidence.json", &bytes);
    v["checks"] = json!([{"id":"unit","status":"passed","fingerprint":validation,"scope":["product","a"],"reason":"fixture input-bound validation","artifact":".runtime/evidence.json","sha256":digest(&bytes)}]);
    save_analysis(&r.0, &v);
    assert!(
        build(&r.0, "fix: change", &request, &SystemProcessRunner)
            .unwrap()
            .is_some()
    );
    write(&r.0, ".runtime/evidence.json", b"{}");
    assert!(
        build(&r.0, "fix: change", &request, &SystemProcessRunner)
            .unwrap_err()
            .contains("artifact changed")
    );
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
    assert!(
        build(
            &r.0,
            "fix: reuse",
            &Request::default(),
            &SystemProcessRunner
        )
        .unwrap_err()
        .contains("retired identity")
    );
}

#[test]
fn schema4_single_model_splits_rust_workspace_version_for_changed_package() {
    let r = fixture("unconfigured", true);
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
    cfg["strategy"] = json!("independent");
    write(
        &r.0,
        ".runtime/proposal.json",
        &serde_json::to_vec(&cfg).unwrap(),
    );
    fs::remove_file(r.0.join(CONFIG)).unwrap();
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
    assert!(!p.writes.contains_key(&r.0.join("pkg/Cargo.toml")));
    let manifest = fs::read_to_string(r.0.join("pkg/Cargo.toml"))
        .unwrap()
        .parse::<toml_edit::DocumentMut>()
        .unwrap();
    assert_eq!(manifest["package"]["version"].as_str(), Some("1.0.1"));
    assert!(!p.writes.contains_key(&r.0.join("Cargo.toml")));
}

#[test]
fn schema4_breaking_lifecycle_cannot_disguise_major_as_compatible() {
    let r = fixture("independent", false);
    let mut cfg: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    let base = cfg["baseline"]["revision"].clone();
    fs::remove_dir_all(r.0.join("pkg")).unwrap();
    cfg["adapters"]["python"]["projects"] = json!([]);
    cfg["packages"] = json!([]);
    cfg["development"] = json!({"a":development_tests::policy(false)});
    cfg["python_environment"] = python_environment_fixture(&r.0);
    cfg["lifecycle"] = json!([{"id":"remove-a","base":base,"operation":"remove","compatibility":"breaking","from":[{"id":"a","language":"python","manifest":"pkg/pyproject.toml","name":"a"}],"to":[]}]);
    write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
    let runner = development_tests::Checks::new();
    let q = development_tests::request(&r.0, "major", false, &runner);
    assert!(
        development::preview(&r.0, &q, &runner)
            .stderr
            .contains("breaking lifecycle")
    );
    let q = development_tests::request(&r.0, "major", true, &runner);
    let result = development_tests::close(&r.0, &q, &runner);
    assert_eq!(result.code, 0, "{}", result.stderr);
    let q = analyze(&r.0, "major", None);
    assert!(
        build(&r.0, "feat!: retire", &q, &runner)
            .unwrap_err()
            .contains("breaking")
    );
    let mut v = analysis_value(&r.0);
    v["product"]["compatibility"] = json!("breaking");
    v["product"]["acceptance"] = json!({"fingerprint":v["fingerprint"],"scope":["product"],"reason":"accepted product migration","source":"existing fixture decision"});
    save_analysis(&r.0, &v);
    assert!(build(&r.0, "feat!: retire", &q, &runner).unwrap().is_some());
}

#[test]
fn schema4_stat_only_index_refresh_keeps_analysis_valid_without_weakening_drift_checks() {
    let r = fixture("independent", false);
    git(&r.0, &["config", "core.autocrlf", "false"]);
    change(&r.0, false);
    let file = fs::OpenOptions::new()
        .write(true)
        .open(r.0.join("README.md"))
        .unwrap();
    file.set_times(
        fs::FileTimes::new()
            .set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000)),
    )
    .unwrap();
    let candidates = git(
        &r.0,
        &[
            "--no-optional-locks",
            "-c",
            "diff.autoRefreshIndex=false",
            "diff",
            "--name-only",
            "HEAD",
            "--",
        ],
    );
    assert!(
        candidates.lines().any(|p| p == "README.md"),
        "stat-only fixture did not create a false candidate: {candidates}"
    );
    let request = analyze(&r.0, "patch", Some("patch"));
    let cfg = load(&r.0).unwrap().unwrap();
    let index = fs::read(r.0.join(".git/index")).unwrap();
    let before = capture(&r.0, &cfg, &SystemProcessRunner).unwrap();
    assert_eq!(
        fs::read(r.0.join(".git/index")).unwrap(),
        index,
        "inspection must remain read-only"
    );
    git(&r.0, &["diff", "--name-only"]);
    let after = capture(&r.0, &cfg, &SystemProcessRunner).unwrap();
    assert_eq!(
        serde_json::to_value(&before.changes).unwrap(),
        serde_json::to_value(&after.changes).unwrap()
    );
    assert_eq!(before.validation_fingerprint, after.validation_fingerprint);
    assert_eq!(
        before.fingerprint, after.fingerprint,
        "stat refresh must not invalidate an unchanged release analysis"
    );
    assert!(
        build(&r.0, "fix: release", &request, &SystemProcessRunner)
            .unwrap()
            .is_some()
    );

    write(&r.0, "README.md", b"real product change\n");
    assert!(
        build(&r.0, "fix: release", &request, &SystemProcessRunner)
            .err()
            .unwrap()
            .contains("stale")
    );
    let request = analyze(&r.0, "patch", Some("patch"));
    git(&r.0, &["add", "pkg/a.py"]);
    assert!(
        build(&r.0, "fix: release", &request, &SystemProcessRunner)
            .err()
            .unwrap()
            .contains("stale")
    );
    let request = analyze(&r.0, "patch", Some("patch"));
    git(&r.0, &["update-index", "--chmod=+x", "README.md"]);
    assert!(
        build(&r.0, "fix: release", &request, &SystemProcessRunner)
            .err()
            .unwrap()
            .contains("stale")
    );
}

#[test]
fn schema4_content_diff_retains_binary_empty_mode_and_delimited_paths() {
    let r = fixture("unconfigured", false);
    git(&r.0, &["config", "core.autocrlf", "false"]);
    let mut paths = vec!["binary.asset", "empty.asset", "spaced name.asset"];
    if cfg!(unix) {
        paths.push("tab\tname.asset");
        paths.push("line\nname.asset");
    }
    let mut value: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    value["product_inputs"]
        .as_array_mut()
        .unwrap()
        .push(json!("new-empty.asset"));
    for path in &paths {
        write(
            &r.0,
            path,
            if *path == "empty.asset" {
                b""
            } else {
                b"old\0bytes\n"
            },
        );
        value["product_inputs"]
            .as_array_mut()
            .unwrap()
            .push(json!(path));
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
        let changed = captured
            .changes
            .iter()
            .find(|c| c.path == path)
            .unwrap_or_else(|| panic!("missing content diff: {path:?}"));
        assert_ne!(changed.before, changed.after);
    }
    assert!(
        captured
            .changes
            .iter()
            .find(|c| c.path == "README.md")
            .unwrap()
            .mode_changed
    );
    let added = captured
        .changes
        .iter()
        .find(|c| c.path == "new-empty.asset")
        .unwrap();
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
    assert!(
        build(
            &r.0,
            "chore: no changes",
            &Request::default(),
            &SystemProcessRunner
        )
        .unwrap()
        .is_none()
    );
    let mut cfg: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    cfg["build_inputs"] = json!(["scripts/recipe.json"]);
    write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
    fixture::close_packages(&r.0, &SystemProcessRunner, &[]).unwrap();
    let request = analyze(&r.0, "patch", None);
    assert!(
        build(&r.0, "chore: role contract", &request, &SystemProcessRunner)
            .unwrap()
            .is_some()
    );
}

#[test]
fn schema4_single_model_rejects_ambiguous_shared_package_source() {
    let r = fixture("unconfigured", false);
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
    let proposal = fixture::current_fixture::proposal(&r.0).unwrap();
    let proposal = proposal.as_ref().map(|p| p.to_str().unwrap());
    let p = configuration_plan(&r.0, &SystemProcessRunner, proposal, Some("independent"));
    assert_eq!(p.code, 0, "{}", p.stderr);
    let p = configuration_apply(
        &r.0,
        &SystemProcessRunner,
        proposal,
        Some("independent"),
        p.receipt.unwrap()["fingerprint"].as_str().unwrap(),
        true,
    );
    assert_eq!(p.code, 0, "{}", p.stderr);
    write(&r.0, "shared/data.py", b"VALUE=2\n");
    assert!(
        build(
            &r.0,
            "fix: shared",
            &Request::default(),
            &SystemProcessRunner
        )
        .unwrap_err()
        .contains("ambiguous")
    );
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
    assert!(
        build(
            &r.0,
            "fix: forged retirement",
            &Request::default(),
            &SystemProcessRunner
        )
        .unwrap_err()
        .contains("baseline")
    );
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
    assert!(
        apply_file_release_plan(&plan)
            .unwrap_err()
            .contains("environment changed")
    );
    let stale = build(&r.0, "fix: stale evidence", &request, &SystemProcessRunner).unwrap_err();
    assert!(stale.contains("release analysis is stale"), "{stale}");
    let request = analyze(&r.0, "patch", Some("patch"));
    let plan = build(&r.0, "fix: native change", &request, &SystemProcessRunner)
        .unwrap()
        .unwrap();
    write(
        &r.0,
        ".runtime/python-environment/python.exe",
        b"fixture interpreter two",
    );
    assert!(
        apply_file_release_plan(&plan)
            .unwrap_err()
            .contains("environment changed")
    );
    assert_eq!(fs::read(r.0.join("VERSION")).unwrap(), b"1.0.0\n");
}

#[test]
fn schema4_rust_patch_overflow_fails_without_touching_versions() {
    let r = fixture("unconfigured", true);
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
    development_tests::configure(&r.0, false);
    let runner = development_tests::Checks::new();
    let request = development_tests::request(&r.0, "patch", false, &runner);
    assert!(
        development::preview(&r.0, &request, &runner)
            .stderr
            .contains("overflow")
    );
    assert_eq!(fs::read(r.0.join("VERSION")).unwrap(), b"1.0.0\n");
}

#[test]
fn schema4_runtime_guard_and_record_roles_fail_closed_without_writes() {
    let r = fixture("unconfigured", false);
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
        assert!(
            parse(&serde_json::to_vec(&cfg).unwrap())
                .unwrap_err()
                .contains("record_inputs")
        );
    }
}

#[test]
fn schema4_forwarded_public_interface_changes_are_explicit_and_do_not_propagate_automatically() {
    let r = development_tests::python_pair(false);
    let runner = development_tests::Checks::new();
    change(&r.0, false);
    let q = development_tests::request(&r.0, "minor", false, &runner);
    let path = r.0.join(".runtime/develop.json");
    let mut v: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    let single = development::preview(&r.0, &q, &runner);
    assert_eq!(single.code, 0, "{}", single.stderr);
    let versions = single.receipt.unwrap()["versions"].clone();
    assert_eq!(versions["pkg/pyproject.toml"], "1.1.0");
    assert_eq!(
        versions["consumer/pyproject.toml"], "1.0.0",
        "dependency validation must not imply a version bump"
    );
    v["packages"]["b"] =
        json!({"level":"minor","reason":"re-exported API","compatibility":"compatible"});
    fs::write(&path, serde_json::to_vec(&v).unwrap()).unwrap();
    let unchanged = development::preview(&r.0, &q, &runner);
    assert_eq!(unchanged.code, 0, "{}", unchanged.stderr);
    assert_eq!(
        unchanged.receipt.unwrap()["versions"]["consumer/pyproject.toml"],
        "1.0.0"
    );
    v["interface_impacts"] = json!({"b":{"dependencies":["a"],"reason":"public re-export"}});
    let config: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    v["validation"]["b"] = config["development"]["b"]["checks"].clone();
    fs::write(&path, serde_json::to_vec(&v).unwrap()).unwrap();
    let both = development::preview(&r.0, &q, &runner);
    assert_eq!(both.code, 0, "{}", both.stderr);
    assert!(
        both.receipt.unwrap()["versions"]
            .as_object()
            .unwrap()
            .values()
            .all(|v| v == "1.1.0")
    );
    v["interface_impacts"]["b"]["reason"] = json!("");
    fs::write(&path, serde_json::to_vec(&v).unwrap()).unwrap();
    assert!(
        development::preview(&r.0, &q, &runner)
            .stderr
            .contains("reason")
    );
    v["interface_impacts"]["b"] = json!({"dependencies":["product"],"reason":"invalid dependency"});
    fs::write(&path, serde_json::to_vec(&v).unwrap()).unwrap();
    assert!(
        development::preview(&r.0, &q, &runner)
            .stderr
            .contains("no declared dependency")
    );
    v["interface_impacts"] = json!({"a":{"dependencies":["b"],"reason":"invalid cycle"}});
    fs::write(&path, serde_json::to_vec(&v).unwrap()).unwrap();
    assert!(
        development::preview(&r.0, &q, &runner)
            .stderr
            .contains("no declared dependency")
    );
    write(&r.0, "pkg/a.py", b"VALUE=1\n");
    let no_change = development::preview(&r.0, &Request::default(), &runner);
    assert_eq!(no_change.code, 0, "{}", no_change.stderr);
    assert_eq!(no_change.receipt.unwrap()["status"], "nothing-to-close");
}

#[test]
fn schema4_required_checks_combine_their_declared_object_scopes() {
    let r = fixture("unconfigured", false);
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
    analysis["checks"] = json!(claims);
    save_analysis(&r.0, &analysis);
    assert!(
        build(&r.0, "fix: scoped evidence", &request, &SystemProcessRunner)
            .unwrap()
            .is_some()
    );
    let complete = analysis.clone();
    analysis["checks"].as_array_mut().unwrap().remove(0);
    save_analysis(&r.0, &analysis);
    assert!(
        build(
            &r.0,
            "fix: missing package evidence",
            &request,
            &SystemProcessRunner
        )
        .unwrap_err()
        .contains("required release evidence missing")
    );
    analysis = complete;
    analysis["checks"].as_array_mut().unwrap().pop();
    save_analysis(&r.0, &analysis);
    assert!(
        build(
            &r.0,
            "fix: missing integration",
            &request,
            &SystemProcessRunner
        )
        .unwrap_err()
        .contains("required release evidence missing")
    );
}

#[test]
fn schema4_scoped_required_check_does_not_require_product_evidence() {
    let r = fixture("independent", true);
    let mut cfg: Value = serde_json::from_slice(&fs::read(r.0.join(CONFIG)).unwrap()).unwrap();
    cfg["required_checks"] = json!(["unit-a"]);
    cfg["check_scopes"] = json!({"unit-a":["a"]});
    write(&r.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
    change(&r.0, true);
    let request = analyze(&r.0, "patch", Some("patch"));
    let ready = preview(
        &r.0,
        "fix: scoped",
        &Request::default(),
        &SystemProcessRunner,
    );
    assert_eq!(ready.code, 0, "{}", ready.stderr);
    let ready = ready.receipt.unwrap();
    assert_eq!(ready["applicable_required_checks"], json!({"unit-a":["a"]}));
    let proof = json!({"id":"unit-a","status":"passed","fingerprint":ready["validation_fingerprint"],"scope":["a"],"reason":"package-only release requirement"});
    let bytes = serde_json::to_vec(&proof).unwrap();
    write(&r.0, ".runtime/unit-a.json", &bytes);
    let mut claim = proof;
    claim["artifact"] = json!(".runtime/unit-a.json");
    claim["sha256"] = json!(digest(&bytes));
    let mut analysis = analysis_value(&r.0);
    analysis["checks"] = json!([claim]);
    save_analysis(&r.0, &analysis);
    assert!(
        build(&r.0, "fix: scoped", &request, &SystemProcessRunner)
            .unwrap()
            .is_some()
    );

    // The scoped requirement itself remains mandatory.
    analysis["checks"] = json!([]);
    save_analysis(&r.0, &analysis);
    assert!(
        build(&r.0, "fix: missing", &request, &SystemProcessRunner)
            .unwrap_err()
            .contains("required release evidence missing: unit-a")
    );

    // Unscoped requirements still include the product; no global weakening.
    let cfg = load(&r.0).unwrap().unwrap();
    let affected = BTreeSet::from(["a".to_string(), "product".to_string()]);
    let mut unscoped = cfg.clone();
    unscoped.check_scopes.clear();
    assert_eq!(
        analysis::required_check_scopes(&unscoped, &affected)["unit-a"],
        affected
    );
    assert!(analysis::required_check_scopes(&cfg, &BTreeSet::from(["product".into()])).is_empty());
}

#[allow(dead_code)]
#[path = "../support/artifact_paths.rs"]
mod test_artifacts;
