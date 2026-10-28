use bridgeforge_core::{ProcessRequest, ProcessRunner, SystemProcessRunner};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[test]
#[ignore = "explicit R10 offline CLI pipeline comparison; not chat end-to-end timing"]
fn git_sync_preflight_pipeline_performance() {
    use sha2::Digest;
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap();
    let baseline = PathBuf::from(
        std::env::var_os("BRIDGEFORGE_SYNC_BASELINE_CLI").expect("explicit baseline CLI required"),
    );
    let current = source.join(if cfg!(windows) {
        ".codex/bin/bridgeforge.exe"
    } else {
        ".codex/bin/bridgeforge"
    });
    let token = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let fixture = Fixture(std::env::temp_dir().join(format!("bf-sync-pipeline-{token}")));
    let root = fixture.0.join("project");
    let remote = fixture.0.join("remote.git");
    fs::create_dir_all(root.join(".codex")).unwrap();
    fs::create_dir_all(root.join("templates")).unwrap();
    fs::write(root.join("tracked.txt"), b"seed\n").unwrap();
    fs::write(root.join("templates/tracked.txt"), b"seed\n").unwrap();
    fs::write(root.join("VERSION"), b"1.0.0\n").unwrap();
    fs::write(root.join("CHANGELOG.md"), b"# Changelog\n").unwrap();
    fs::write(root.join(".gitignore"), b".runtime/\n").unwrap();
    fs::write(root.join(".codex/.bridgeforge_codex_version"), b"8.0.0\n").unwrap();
    let asset_hash = format!("sha256:{:x}", sha2::Sha256::digest(b"seed\n"));
    fs::write(root.join(".codex/managed-skeleton.json"),serde_json::to_vec(&serde_json::json!({
        "schema_version":4,"release_version":"8.0.0","host":"codex","stamp":".codex/.bridgeforge_codex_version",
        "contract_target":".codex/managed-skeleton.json","baseline_model":"current-only","compatibility_baseline":"8.0.0",
        "assets":[{"id":"fixture.asset","source":"templates/tracked.txt","target":"tracked.txt","strategy":"whole","current_sha256":asset_hash}],"generated_assets":[]
    })).unwrap()).unwrap();
    fs::write(root.join(".codex/bridgeforge-version.json"),br#"{"schema_version":4,"release_policy":"explicit_release","strategy":"independent","adapters":{},"packages":[],"git_sync_setup":{"schema":1,"confirmed":true,"selection":"current"}}"#).unwrap();
    for args in [
        vec!["init"],
        vec!["config", "user.name", "Fixture"],
        vec!["config", "user.email", "fixture@example.invalid"],
        vec!["config", "core.hooksPath", ".git/hooks"],
        vec!["add", "."],
        vec!["commit", "-m", "chore: fixture baseline"],
    ] {
        git(&root, &args);
    }
    git(&root, &["init", "--bare", remote.to_str().unwrap()]);
    git(
        &root,
        &["remote", "add", "origin", remote.to_str().unwrap()],
    );
    git(&root, &["push", "-u", "origin", "HEAD"]);
    let head = git(&root, &["rev-parse", "HEAD"]);
    fn call(root: &Path, program: &Path, args: &[&str]) -> (bridgeforge_core::ProcessOutput, u128) {
        let mut request = ProcessRequest::new(program.as_os_str(), root);
        request.args = args.iter().map(Into::into).collect();
        request.timeout = Duration::from_secs(120);
        let started = std::time::Instant::now();
        let result = SystemProcessRunner.run(&request).unwrap();
        let elapsed = started.elapsed().as_millis();
        assert!(
            !result.timed_out && result.code == 0,
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        (result, elapsed)
    }
    let mut measurements = Vec::new();
    for sample in 0..5 {
        let order = if sample % 2 == 0 {
            [false, true]
        } else {
            [true, false]
        };
        for modern in order {
            let cli = if modern { &current } else { &baseline };
            let total = std::time::Instant::now();
            let precheck = std::time::Instant::now();
            let (identity, _) = call(&root, cli, &["self-test", "--json"]);
            let identity: serde_json::Value = serde_json::from_slice(&identity.stdout).unwrap();
            assert_eq!(
                identity["capabilities"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|v| v == "git-sync-preflight-v1"),
                modern
            );
            if modern {
                let (result, _) = call(&root, cli, &["git-sync", "--preflight"]);
                let value: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
                assert_eq!(value["status"], "ready");
                assert_eq!(value["head"], head);
                assert_eq!(value["working_tree"], "clean");
                assert_eq!(value["diff_review_required"], false);
            } else {
                let (result, _) = call(&root, cli, &["git-sync", "--setup-status"]);
                let value: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
                assert_eq!(value["status"], "configured");
                for args in [
                    vec!["symbolic-ref", "--quiet", "--short", "HEAD"],
                    vec!["rev-parse", "HEAD"],
                    vec!["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"],
                    vec![
                        "rev-parse",
                        "--abbrev-ref",
                        "--symbolic-full-name",
                        "@{push}",
                    ],
                    vec!["status", "--porcelain=v1", "-uall"],
                    vec!["diff", "--stat"],
                    vec!["diff", "--cached", "--stat"],
                ] {
                    call(&root, Path::new("git"), &args);
                }
            }
            let precheck_ms = precheck.elapsed().as_millis();
            let (output, sync_ms) = call(
                &root,
                cli,
                &["git-sync", "--message", "chore: no-op fixture"],
            );
            let receipt: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(receipt["status"], "synced");
            assert_eq!(receipt["working_tree"], "clean");
            assert_eq!(receipt["ahead"], 0);
            assert_eq!(receipt["behind"], 0);
            assert_eq!(receipt["push_performed"], false);
            assert_eq!(receipt["generated_assets_built"], 0);
            assert_eq!(receipt["version_bumped"], false);
            let verify = std::time::Instant::now();
            assert_eq!(git(&root, &["rev-parse", "HEAD"]), head);
            assert!(git(&root, &["status", "--porcelain=v1", "-uall"]).is_empty());
            assert_eq!(
                git(
                    &root,
                    &["rev-list", "--left-right", "--count", "HEAD...@{push}"]
                ),
                "0\t0"
            );
            measurements.push(serde_json::json!({"sample":sample,"variant":if modern{"new"}else{"baseline"},"precheck_ms":precheck_ms,"sync_process_ms":sync_ms,"sync_elapsed_ms":receipt["sync_elapsed_ms"],"verify_ms":verify.elapsed().as_millis(),"pipeline_ms":total.elapsed().as_millis()}));
        }
    }
    let result = serde_json::json!({"scope":"isolated_cli_pipeline_not_chat_end_to_end","samples_per_variant":5,"generated_assets":0,"remote":"local_bare_only","baseline_sha256":format!("{:x}",sha2::Sha256::digest(fs::read(&baseline).unwrap())),"new_sha256":format!("{:x}",sha2::Sha256::digest(fs::read(&current).unwrap())),"measurements":measurements});
    let output = source.join(".runtime/git-sync-performance/cli-comparison.json");
    fs::create_dir_all(output.parent().unwrap()).unwrap();
    fs::write(output, serde_json::to_vec_pretty(&result).unwrap()).unwrap();
    println!("{result}");
}

struct Fixture(PathBuf);

#[test]
fn new_project_setup_with_product_manifest() {
    new_project_setup("product");
}

#[test]
fn new_project_setup_with_independent_root_package() {
    new_project_setup("independent");
}

#[test]
fn new_project_setup_with_virtual_workspace() {
    new_project_setup("virtual");
}

fn new_project_setup(kind: &str) {
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap();
    let cli = source.join(if cfg!(windows) {
        ".codex/bin/bridgeforge.exe"
    } else {
        ".codex/bin/bridgeforge"
    });
    let token = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let fixture = Fixture(std::env::temp_dir().join(format!("bf-new-setup-{kind}-{token}")));
    let root = &fixture.0;
    fs::create_dir_all(root.join(".codex")).unwrap();
    fs::create_dir_all(root.join(".runtime")).unwrap();
    fs::write(root.join("VERSION"), b"1.0.0\n").unwrap();
    fs::write(root.join("CHANGELOG.md"), b"# Changelog\n").unwrap();
    fs::write(root.join(".gitignore"), b".runtime/\n").unwrap();
    fs::write(root.join(".codex/.bridgeforge_codex_version"), b"8.0.0\n").unwrap();
    let manifest = if kind == "virtual" {
        "pkg/Cargo.toml"
    } else {
        "Cargo.toml"
    };
    let package_version = if kind == "product" { "1.0.0" } else { "0.4.0" };
    let package = root.join(manifest).parent().unwrap().to_path_buf();
    fs::create_dir_all(package.join("src")).unwrap();
    fs::write(package.join("src/lib.rs"), b"pub fn value() -> u8 { 1 }\n").unwrap();
    fs::write(
        root.join(manifest),
        format!("[package]\nname='demo'\nversion='{package_version}'\nedition='2024'\n"),
    )
    .unwrap();
    if kind == "virtual" {
        fs::write(
            root.join("Cargo.toml"),
            b"[workspace]\nmembers=['pkg']\nresolver='3'\n",
        )
        .unwrap();
    }
    fs::write(
        root.join("Cargo.lock"),
        format!("version=4\n[[package]]\nname='demo'\nversion='{package_version}'\n"),
    )
    .unwrap();
    let proposal = if kind == "product" {
        serde_json::json!({"schema_version":4,"release_policy":"explicit_release","strategy":"independent","adapters":{},"packages":[],"product_manifests":["Cargo.toml"]})
    } else {
        serde_json::json!({"schema_version":4,"release_policy":"explicit_release","strategy":"independent","adapters":{"rust":{"workspaces":["Cargo.toml"]}},"packages":[{"id":"demo","language":"rust","manifest":manifest,"name":"demo"}]})
    };
    let proposal_path = ".runtime/current-configuration.json";
    fs::write(
        root.join(proposal_path),
        serde_json::to_vec(&proposal).unwrap(),
    )
    .unwrap();
    for args in [
        vec!["init"],
        vec!["config", "user.name", "Fixture"],
        vec!["config", "user.email", "fixture@example.invalid"],
        vec!["config", "core.hooksPath", ".git/hooks"],
        vec!["add", "."],
        vec!["commit", "-m", "initial business project"],
    ] {
        git(root, &args);
    }
    let head = git(root, &["rev-parse", "HEAD"]);
    let index = fs::read(root.join(".git/index")).unwrap();
    let plan = super::schema4_fixture::invoke(
        &cli,
        root,
        &[
            "git-sync",
            "--setup-plan",
            "--setup-config-file",
            proposal_path,
        ],
        &[],
    );
    assert!(!root.join(".codex/bridgeforge-version.json").exists());
    assert_eq!(fs::read(root.join(".git/index")).unwrap(), index);
    let result = super::schema4_fixture::invoke(
        &cli,
        root,
        &[
            "git-sync",
            "--setup-apply",
            "--setup-config-file",
            proposal_path,
            "--setup-fingerprint",
            plan["fingerprint"].as_str().unwrap(),
            "--confirmed-choice",
        ],
        &[],
    );
    assert_eq!(result["sync_performed"], false);
    assert_eq!(git(root, &["rev-parse", "HEAD"]), head);
    assert_eq!(fs::read(root.join(".git/index")).unwrap(), index);
    assert_eq!(fs::read(root.join("VERSION")).unwrap(), b"1.0.0\n");
    let config: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join(".codex/bridgeforge-version.json")).unwrap())
            .unwrap();
    assert_eq!(config["release_model"], "product-packages");
    assert_eq!(config["baseline"]["revision"], head);
    assert!(
        fs::read_to_string(root.join(manifest))
            .unwrap()
            .contains(&format!("version='{package_version}'"))
    );
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn copy_tree(source: &Path, target: &Path) {
    fs::create_dir_all(target).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        if matches!(
            entry.file_name().to_str(),
            Some("target" | ".git" | ".runtime" | ".venv" | "__pycache__")
        ) {
            continue;
        }
        assert!(!entry.file_type().unwrap().is_symlink());
        let destination = target.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &destination);
        } else {
            fs::copy(entry.path(), destination).unwrap();
        }
    }
}
fn git(root: &Path, args: &[&str]) -> String {
    let mut request = ProcessRequest::new("git", root);
    request.args = args.iter().map(Into::into).collect();
    request.timeout = Duration::from_secs(120);
    let output = SystemProcessRunner.run(&request).unwrap();
    assert!(
        !output.timed_out && output.code == 0,
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().into()
}

fn fixture_configuration(root: &Path) {
    assert!(
        root.starts_with(std::env::temp_dir()) && !root.join(".git").exists(),
        "fixture setup requires a fresh temporary repository"
    );
    // Factory build/rollback fixtures start from an already confirmed policy;
    // first-use interaction is tested through the real CLI below.
    let path = root.join(".codex/bridgeforge-version.json");
    let copied: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    // Each copied fixture starts with its own native history. Preserve the
    // mixed graph while migrating a fresh schema-3 seed; never copy the real
    // repository's immutable baseline, adoption provenance or closure ledger.
    let config = if copied.get("product_packages").is_some() {
        serde_json::json!({
            "schema_version":3,"release_policy":"explicit_release","strategy":"independent",
            "git_sync_setup":{"schema":1,"confirmed":true,"selection":"independent"},
            "adapters":copied["adapters"],"packages":copied["packages"],
            "product_packages":copied["product_packages"],"product_inputs":copied["product_inputs"]
        })
    } else {
        let mappings = copied
            .get("product_manifests")
            .or_else(|| copied.get("synchronized_manifests"))
            .or_else(|| copied.get("manifests"))
            .expect("fixture version mappings");
        serde_json::json!({"schema_version":1,"release_policy":"explicit_release","manifests":mappings,"git_sync_setup":{"schema":1,"confirmed":true,"selection":"current"}})
    };
    fs::write(path, serde_json::to_vec_pretty(&config).unwrap()).unwrap();
    let closures = root.join(".codex/package-closures.json");
    if closures.exists() {
        fs::remove_file(closures).unwrap();
    }
}

#[test]
fn real_cli_explicit_release_preview_and_execution() {
    for policy in ["missing-file", "explicit_release"] {
        real_cli_release_flow(policy);
    }
}

#[test]
fn real_cli_explicit_release_rust_independent() {
    real_cli_release_flow("rust-independent");
}

#[test]
fn real_cli_explicit_release_rust_additions() {
    real_cli_release_flow("rust-additions");
}

fn real_cli_release_flow(policy: &str) {
    let additions = policy == "rust-additions";
    let policy = if additions {
        "rust-independent"
    } else {
        policy
    };
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap();
    let cli = source.join(if cfg!(windows) {
        ".codex/bin/bridgeforge.exe"
    } else {
        ".codex/bin/bridgeforge"
    });
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let fixture_name = format!("bf-explicit-cli-{}-{nonce}", std::process::id());
    let fixture = Fixture(std::env::temp_dir().join(fixture_name));
    let root = fixture.0.join("project");
    fs::create_dir_all(root.join(".codex")).unwrap();
    let call = |args: &[&str]| {
        let mut request = ProcessRequest::new(cli.as_os_str(), &root);
        request.args = args.iter().map(Into::into).collect();
        request.timeout = Duration::from_secs(120);
        let started = std::time::Instant::now();
        let output = SystemProcessRunner.run(&request).unwrap();
        let elapsed_ms = started.elapsed().as_millis();
        assert!(
            !output.timed_out && output.code == 0,
            "policy={policy}, command={args:?}, root={}: {}",
            root.display(),
            String::from_utf8_lossy(&output.stderr)
        );
        let receipt = serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap();
        if receipt["status"] == "synced" {
            let inner_ms = receipt["sync_elapsed_ms"]
                .as_u64()
                .expect("real sync reports its internal duration");
            assert!(
                u128::from(inner_ms) <= elapsed_ms,
                "internal timing must not include the Agent or exceed the caller's process duration"
            );
        }
        receipt
    };
    assert!(
        call(&["self-test", "--json"])["capabilities"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == "git-sync-direct-release-v1")
    );
    assert!(
        call(&["self-test", "--json"])["capabilities"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == "git-sync-schema4-v1")
    );
    assert!(
        call(&["self-test", "--json"])["capabilities"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == "git-sync-remote-context-v1")
    );
    for flag in ["--dry-run", "--check"] {
        let mut probe = ProcessRequest::new(cli.as_os_str(), &root);
        probe.args = ["git-sync", "--prepare-release", flag]
            .iter()
            .map(Into::into)
            .collect();
        let result = SystemProcessRunner.run(&probe).unwrap();
        assert_eq!(result.code, 2);
        assert!(!root.join(".runtime").exists());
    }
    fs::write(root.join("VERSION"), b"1.0.0\n").unwrap();
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("src/lib.rs"),
        b"pub fn value() -> u32 { 1 }\n#[test] fn smoke(){assert_eq!(value(),1);}\n",
    )
    .unwrap();
    fs::write(
        root.join("Cargo.toml"),
        if policy == "rust-independent" {
            b"[package]\nname = \"demo\"\nversion = \"0.4.0\"\n".as_slice()
        } else {
            b"[package]\nname = \"demo\"\nversion = \"1.0.0\"\n".as_slice()
        },
    )
    .unwrap();
    fs::write(root.join("CHANGELOG.md"), b"# Changelog\n").unwrap();
    fs::write(root.join(".gitignore"), b".runtime/\n").unwrap();
    fs::write(root.join(".codex/.bridgeforge_codex_version"), b"8.0.0\n").unwrap();
    let policy_path = root.join(".codex/bridgeforge-version.json");
    if policy != "missing-file" {
        let config = if policy == "rust-independent" {
            serde_json::json!({"schema_version":2,"release_policy":"explicit_release","strategy":"rust_independent",
                "rust":{"workspaces":["Cargo.toml"],"dependency_bump":"patch"}})
        } else {
            serde_json::json!({"schema_version":1,"release_policy":"explicit_release","manifests":["Cargo.toml"]})
        };
        fs::write(&policy_path, serde_json::to_vec(&config).unwrap()).unwrap();
    }
    fs::write(
        root.join("Cargo.lock"),
        if policy == "rust-independent" {
            b"version = 4\n[[package]]\nname = \"demo\"\nversion = \"0.4.0\"\n".as_slice()
        } else {
            b"version = 4\n[[package]]\nname = \"demo\"\nversion = \"1.0.0\"\n".as_slice()
        },
    )
    .unwrap();
    use sha2::{Digest, Sha256};
    fs::write(root.join("managed.txt"), b"managed\n").unwrap();
    let contract = serde_json::json!({
        "schema_version": 4, "release_version": "8.0.0", "host": "codex",
        "stamp": ".codex/.bridgeforge_codex_version", "contract_target": ".codex/managed-skeleton.json",
        "assets": [{"id": "managed.asset", "source": "templates/managed.txt", "target": "managed.txt",
            "strategy": "whole", "current_sha256": format!("sha256:{:x}", Sha256::digest(b"managed\n"))}],
        "generated_assets": [], "baseline_model": "current-only", "compatibility_baseline": "8.0.0"
    });
    fs::write(
        root.join(".codex/managed-skeleton.json"),
        serde_json::to_vec_pretty(&contract).unwrap(),
    )
    .unwrap();
    git(&root, &["init"]);
    git(&root, &["config", "user.name", "BridgeForge Test"]);
    git(&root, &["config", "user.email", "test@example.invalid"]);
    git(&root, &["config", "core.hooksPath", ".git/hooks"]);
    git(&root, &["add", "."]);
    git(&root, &["commit", "-m", "baseline"]);
    let remote = fixture.0.join("origin.git");
    git(&root, &["init", "--bare", remote.to_str().unwrap()]);
    git(
        &root,
        &["remote", "add", "origin", remote.to_str().unwrap()],
    );
    git(&root, &["push", "-u", "origin", "HEAD"]);
    let initial_head = git(&root, &["rev-parse", "HEAD"]);
    let initial_entries = git(&root, &["ls-files", "--stage"]);
    let initial_index = fs::read(root.join(".git/index")).unwrap();
    fs::write(root.join("VERSION"), b"0.9.9\n").unwrap();
    let mut probe = ProcessRequest::new(cli.as_os_str(), &root);
    probe.args = ["git-sync", "--release-status"]
        .iter()
        .map(Into::into)
        .collect();
    let output = SystemProcessRunner.run(&probe).unwrap();
    assert_eq!(output.code, 2);
    let setup: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(setup["status"], "blocked");
    assert_eq!(setup["development_checks_required"], false);
    assert_eq!(git(&root, &["rev-parse", "HEAD"]), initial_head);
    assert_eq!(fs::read(root.join(".git/index")).unwrap(), initial_index);
    assert!(!root.join(".runtime").exists());
    assert!(!root.join(".codex/development-checks.json").exists());
    fs::write(root.join("VERSION"), b"1.0.0\n").unwrap();
    let version_files = ["VERSION", "Cargo.toml", "Cargo.lock", "CHANGELOG.md"];
    if policy == "missing-file" {
        let mut first = ProcessRequest::new(cli.as_os_str(), &root);
        first.args = ["git-sync", "--message", "feat: wait for choice"]
            .iter()
            .map(Into::into)
            .collect();
        let initial = SystemProcessRunner.run(&first).unwrap();
        assert_eq!(
            initial.code,
            0,
            "{}",
            String::from_utf8_lossy(&initial.stderr)
        );
        let initial: serde_json::Value = serde_json::from_slice(&initial.stdout).unwrap();
        assert_eq!(initial["status"], "synced");
        assert_eq!(git(&root, &["rev-parse", "HEAD"]), initial_head);
        assert_eq!(git(&root, &["ls-files", "--stage"]), initial_entries);
        assert!(!root.join(".runtime").exists());
    }
    // Old configuration remains only in immutable fixture history. The current
    // declaration is an explicit proposal, never an unverified live schema 4.
    let setup_path = ".runtime/current-configuration.json";
    fs::create_dir_all(root.join(".runtime")).unwrap();
    let setup_config = if policy == "rust-independent" {
        serde_json::json!({"schema_version":4,"release_policy":"explicit_release","strategy":"independent",
            "adapters":{"rust":{"workspaces":["Cargo.toml"],"dependency_bump":"patch"}},
            "packages":[{"id":"demo","language":"rust","manifest":"Cargo.toml","name":"demo"}]})
    } else {
        serde_json::json!({"schema_version":4,"release_policy":"explicit_release","strategy":"independent",
            "adapters":{},"packages":[],"product_manifests":["Cargo.toml"]})
    };
    fs::write(
        root.join(setup_path),
        serde_json::to_vec(&setup_config).unwrap(),
    )
    .unwrap();
    if policy_path.exists() {
        fs::remove_file(&policy_path).unwrap();
    }
    let proposal = call(&[
        "git-sync",
        "--setup-plan",
        "--setup-config-file",
        setup_path,
    ]);
    let selected = call(&[
        "git-sync",
        "--setup-apply",
        "--setup-config-file",
        setup_path,
        "--setup-fingerprint",
        proposal["fingerprint"].as_str().unwrap(),
        "--confirmed-choice",
    ]);
    assert_eq!(selected["sync_performed"], false);
    assert_eq!(
        call(&["git-sync", "--setup-status"])["status"],
        "configured"
    );
    if additions {
        fs::create_dir_all(root.join("new/src")).unwrap();
        fs::write(
            root.join("new/Cargo.toml"),
            b"[package]\nname='new'\nversion='0.1.7'\nedition='2024'\n[workspace]\n",
        )
        .unwrap();
        fs::write(
            root.join("new/src/lib.rs"),
            b"pub fn value() -> u32 { 7 }\n#[test] fn value_is_seven() { assert_eq!(value(), 7); }\n",
        )
        .unwrap();
        fs::write(
            root.join("new/Cargo.lock"),
            b"version=4\n[[package]]\nname='new'\nversion='0.1.7'\n",
        )
        .unwrap();
        fs::write(root.join("Cargo.toml"), b"[package]\nname='demo'\nversion='0.4.0'\nedition='2024'\n[dependencies]\nnew={path='new',version='=0.1.7'}\n").unwrap();
        fs::write(root.join("Cargo.lock"), b"version=4\n[[package]]\nname='demo'\nversion='0.4.0'\ndependencies=['new']\n[[package]]\nname='new'\nversion='0.1.7'\n").unwrap();
        let mut cfg: serde_json::Value =
            serde_json::from_slice(&fs::read(&policy_path).unwrap()).unwrap();
        cfg["adapters"]["rust"]["workspaces"] = serde_json::json!(["Cargo.toml", "new/Cargo.toml"]);
        cfg["packages"].as_array_mut().unwrap().push(serde_json::json!({"id":"new","language":"rust","manifest":"new/Cargo.toml","name":"new"}));
        fs::write(&policy_path, serde_json::to_vec(&cfg).unwrap()).unwrap();
    }
    let version_bytes = version_files.map(|name| fs::read(root.join(name)).unwrap());
    let policy_bytes = fs::read(&policy_path).ok();
    fs::write(root.join("business.txt"), b"feature\n").unwrap();
    let ordinary = call(&["git-sync", "--message", "feat: new capability"]);
    assert_eq!(ordinary["version_bumped"], false);
    assert_eq!(ordinary["version_strategy"], "independent");
    assert_eq!(ordinary["release_requested"], false);
    assert_eq!(ordinary["release_policy"], "explicit_release");
    assert_eq!(ordinary["version_before"], "1.0.0");
    assert_eq!(ordinary["version_after"], "1.0.0");
    assert_eq!(ordinary["working_tree"], "clean");
    assert_eq!(ordinary["ahead"], 0);
    assert_eq!(ordinary["behind"], 0);
    for (name, bytes) in version_files.iter().zip(&version_bytes) {
        assert_eq!(
            &fs::read(root.join(name)).unwrap(),
            bytes,
            "{policy}: {name}"
        );
    }
    assert_eq!(fs::read(&policy_path).ok(), policy_bytes);
    let before = git(&root, &["rev-parse", "HEAD"]);
    assert_eq!(call(&["git-sync", "--preflight"])["status"], "ready");
    let integrated = call(&["git-sync", "--sync-remote"]);
    assert_eq!(integrated["status"], "remote-ready");
    assert_eq!(integrated["commit_performed"], false);
    assert_eq!(integrated["push_performed"], false);
    assert_eq!(integrated["version_bumped"], false);
    assert_eq!(git(&root, &["rev-parse", "HEAD"]), before);
    for (name, bytes) in version_files.iter().zip(&version_bytes) {
        assert_eq!(&fs::read(root.join(name)).unwrap(), bytes);
    }
    let index = fs::read(root.join(".git/index")).unwrap();
    let fetch = fs::read(root.join(".git/FETCH_HEAD")).unwrap();
    super::schema4_fixture::analyze(&cli, &root, "fix: release", "minor", &[], &[]);
    let policy_bytes = fs::read(&policy_path).ok();
    let preview = call(&[
        "git-sync",
        "--release-preview",
        "--message",
        "fix: release",
        "--release-analysis-file",
        ".runtime/schema4-analysis.json",
    ]);
    assert_eq!(preview["version_after"], "1.1.0");
    if policy == "rust-independent" {
        assert_eq!(preview["version_strategy"], "independent");
        assert_eq!(preview["package_changes"][0]["version_after"], "0.5.0");
        if additions {
            assert_eq!(preview["package_events"][1]["operation"], "add");
            assert_eq!(preview["package_changes"][1]["version_after"], "0.1.7");
        }
    }
    assert_eq!(git(&root, &["rev-parse", "HEAD"]), before);
    assert_eq!(fs::read(root.join(".git/index")).unwrap(), index);
    assert_eq!(fs::read(root.join(".git/FETCH_HEAD")).unwrap(), fetch);
    assert_eq!(fs::read(root.join("VERSION")).unwrap(), b"1.0.0\n");
    assert_eq!(
        call(&["git-sync", "--release-status"])["status"],
        "release-ready"
    );
    // Use a product-only input: a root package owns business.txt and changing
    // it would correctly trip package coverage before analysis validation.
    // Rejection must precede version/index writes and preserve user edits.
    let release_version_bytes = version_files.map(|name| fs::read(root.join(name)).unwrap());
    let changelog = fs::read(root.join("CHANGELOG.md")).unwrap();
    let mut changed_changelog = changelog.clone();
    changed_changelog.extend_from_slice(b"\n<!-- changed after preview -->\n");
    fs::write(root.join("CHANGELOG.md"), &changed_changelog).unwrap();
    let mut stale = ProcessRequest::new(cli.as_os_str(), &root);
    stale.args = [
        "git-sync", "--release", "--skip-fetch", "--message", "fix: release",
        "--release-analysis-file", ".runtime/schema4-analysis.json",
    ].iter().map(Into::into).collect();
    stale.timeout = Duration::from_secs(120);
    let rejected = SystemProcessRunner.run(&stale).unwrap();
    assert!(!rejected.timed_out);
    assert_eq!(rejected.code, 2, "{}", String::from_utf8_lossy(&rejected.stderr));
    assert!(
        String::from_utf8_lossy(&rejected.stderr).contains("stale"),
        "policy={policy}: {}",
        String::from_utf8_lossy(&rejected.stderr)
    );
    assert_eq!(git(&root, &["rev-parse", "HEAD"]), before);
    assert_eq!(fs::read(root.join(".git/index")).unwrap(), index);
    assert_eq!(fs::read(root.join(".git/FETCH_HEAD")).unwrap(), fetch);
    for (name, bytes) in version_files.iter().zip(&release_version_bytes) {
        if *name != "CHANGELOG.md" {
            assert_eq!(&fs::read(root.join(name)).unwrap(), bytes);
        }
    }
    assert_eq!(fs::read(root.join("CHANGELOG.md")).unwrap(), changed_changelog);
    fs::write(root.join("CHANGELOG.md"), changelog).unwrap();
    let released = call(&[
        "git-sync",
        "--release",
        "--skip-fetch",
        "--message",
        "fix: release",
        "--release-analysis-file",
        ".runtime/schema4-analysis.json",
    ]);
    assert_eq!(released["version_after"], "1.1.0");
    assert_eq!(released["release_requested"], true);
    assert_eq!(released["version_bumped"], true);
    assert_eq!(fs::read(&policy_path).ok(), policy_bytes);
    assert_eq!(released["working_tree"], "clean");
    assert_eq!(released["ahead"], 0);
    assert_eq!(released["behind"], 0);
    assert_eq!(released["generated_assets_built"], 0);
    assert_eq!(released["release_planning"], "once-after-remote");
    assert_eq!(fs::read(root.join(".git/FETCH_HEAD")).unwrap(), fetch);
    if policy == "rust-independent" {
        assert_eq!(released["package_changes"][0]["version_after"], "0.5.0");
        assert_eq!(
            released["compatibility_validation"],
            "not_performed_by_git_sync"
        );
        if additions {
            assert_eq!(released["package_changes"], preview["package_changes"]);
            assert_eq!(
                fs::read(root.join("new/Cargo.toml")).unwrap(),
                b"[package]\nname='new'\nversion='0.1.7'\nedition='2024'\n[workspace]\n"
            );
        }
    }
    let repeated = call(&["git-sync", "--release", "--skip-fetch", "--message", "feat!: ignored"]);
    assert_eq!(repeated["version_bumped"], false);
    assert_eq!(repeated["version_strategy"], "independent");
    assert_eq!(repeated["commit"], released["commit"]);
    if additions {
        fs::write(
            root.join("new/src/lib.rs"),
            b"pub fn value() -> u32 { 8 }\n#[test] fn value_is_eight() { assert_eq!(value(), 8); }\n",
        )
        .unwrap();
        super::schema4_fixture::analyze(
            &cli,
            &root,
            "fix: improve existing new package",
            "patch",
            &[("demo", "patch")],
            &[],
        );
        let next = call(&[
            "git-sync",
            "--release-preview",
            "--message",
            "fix: improve existing new package",
            "--release-analysis-file",
            ".runtime/schema4-analysis.json",
        ]);
        assert_eq!(next["version_after"], "1.1.1");
        assert_eq!(next["package_changes"][0]["version_after"], "0.5.1");
        assert!(
            next["package_changes"][0]["reason"]
                .as_str()
                .unwrap()
                .contains("requirement")
        );
        assert_eq!(next["package_changes"][1]["version_after"], "0.1.8");
        assert!(
            next["package_changes"][1]["reason"]
                .as_str()
                .unwrap()
                .contains("actual package")
        );
    }
}

#[test]
fn real_factory_cli_sync_builds_new_runtime_and_commits_through_precommit() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap();
    let fixture = Fixture(std::env::temp_dir().join(format!(
            "bf-real-factory-sync-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )));
    let root = fixture.0.join("factory");
    copy_tree(source, &root);
    fixture_configuration(&root);
    git(&root, &["init"]);
    git(&root, &["config", "user.name", "BridgeForge Fixture"]);
    git(&root, &["config", "user.email", "fixture@example.invalid"]);
    git(&root, &["config", "core.autocrlf", "false"]);
    git(&root, &["config", "core.hooksPath", ".git/hooks"]);
    git(&root, &["add", "."]);
    git(&root, &["commit", "-m", "fixture seed"]);
    git(&root, &["config", "core.hooksPath", ".githooks"]);
    let remote = fixture.0.join("origin.git");
    git(&root, &["init", "--bare", remote.to_str().unwrap()]);
    git(
        &root,
        &["remote", "add", "origin", remote.to_str().unwrap()],
    );
    git(&root, &["push", "-u", "origin", "HEAD"]);
    let old: bridgeforge_lifecycle::release::SemVer = fs::read_to_string(root.join("VERSION"))
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    let cli = root.join(if cfg!(windows) {
        ".codex/bin/bridgeforge.exe"
    } else {
        ".codex/bin/bridgeforge"
    });
    super::schema4_fixture::migrate(&cli, &root);
    let original = fs::read(&cli).unwrap();
    let mut readme = fs::read(root.join("README.md")).unwrap();
    readme.extend_from_slice(b"\nFixture: exercise complete automatic release.\n");
    fs::write(root.join("README.md"), &readme).unwrap();
    super::schema4_fixture::analyze(
        &cli,
        &root,
        "fix: verify factory automatic runtime release",
        "patch",
        &[],
        &[],
    );
    let mut request = ProcessRequest::new(cli.as_os_str(), &root);
    request.args = [
        "git-sync",
        "--release",
        "--release-analysis-file",
        ".runtime/schema4-analysis.json",
        "--message",
        "fix: verify factory automatic runtime release",
    ]
    .iter()
    .map(Into::into)
    .collect();
    request.timeout = Duration::from_secs(2400);
    let output = SystemProcessRunner.run(&request).unwrap();
    assert!(
        !output.timed_out && output.code == 0,
        "actual CLI sync failed: {} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(git(&root, &["status", "--porcelain=v1"]).is_empty());
    assert_eq!(
        git(
            &root,
            &["rev-list", "--left-right", "--count", "HEAD...@{u}"]
        ),
        "0\t0"
    );
    let next = format!("{}.{}.{}", old.major, old.minor, old.patch + 1);
    assert_eq!(
        fs::read_to_string(root.join("VERSION")).unwrap().trim(),
        next.to_string()
    );
    assert_ne!(fs::read(&cli).unwrap(), original);
    assert_eq!(fs::read(root.join("README.md")).unwrap(), readme);
    bridgeforge_skeleton::baseline::verify(&root, None, true).unwrap();
    bridgeforge_skeleton::baseline::verify_index(&root, &SystemProcessRunner).unwrap();
    assert!(!bridgeforge_factory::manifest::rebuild(&root, true).unwrap());
    request.args = ["self-test", "--json"].iter().map(Into::into).collect();
    request.timeout = Duration::from_secs(30);
    let tested = SystemProcessRunner.run(&request).unwrap();
    let receipt: serde_json::Value = serde_json::from_slice(&tested.stdout).unwrap();
    assert_eq!(tested.code, 0);
    assert_eq!(receipt["version"], next.to_string());

    let released_head = git(&root, &["rev-parse", "HEAD"]);
    let hook_receipt = root.join(".codex/bin/build-receipt-hook.json");
    fs::write(&hook_receipt, b"{}\n").unwrap();
    request.args = ["git-sync"].iter().map(Into::into).collect();
    request.timeout = Duration::from_secs(2400);
    let repaired = SystemProcessRunner.run(&request).unwrap();
    assert!(
        !repaired.timed_out && repaired.code == 0,
        "clean factory runtime repair failed: {} {}",
        String::from_utf8_lossy(&repaired.stdout),
        String::from_utf8_lossy(&repaired.stderr)
    );
    assert_eq!(git(&root, &["rev-parse", "HEAD"]), released_head);
    assert_eq!(
        fs::read_to_string(root.join("VERSION")).unwrap().trim(),
        next
    );
    assert!(git(&root, &["status", "--porcelain=v1"]).is_empty());
    assert_eq!(
        git(
            &root,
            &["rev-list", "--left-right", "--count", "HEAD...@{u}"]
        ),
        "0\t0"
    );
    bridgeforge_skeleton::baseline::verify(&root, None, true).unwrap();
    assert_ne!(fs::read(&hook_receipt).unwrap(), b"{}\n");

    let cli_modified = fs::metadata(&cli).unwrap().modified().unwrap();
    let receipt_modified = fs::metadata(&hook_receipt).unwrap().modified().unwrap();
    let fast_path = SystemProcessRunner.run(&request).unwrap();
    assert!(
        !fast_path.timed_out && fast_path.code == 0,
        "healthy clean factory fast path failed: {} {}",
        String::from_utf8_lossy(&fast_path.stdout),
        String::from_utf8_lossy(&fast_path.stderr)
    );
    assert_eq!(
        fs::metadata(&cli).unwrap().modified().unwrap(),
        cli_modified
    );
    assert_eq!(
        fs::metadata(&hook_receipt).unwrap().modified().unwrap(),
        receipt_modified
    );
    assert!(git(&root, &["status", "--porcelain=v1"]).is_empty());
    assert_eq!(
        git(
            &root,
            &["rev-list", "--left-right", "--count", "HEAD...@{u}"]
        ),
        "0\t0"
    );
    println!(
        "real factory CLI: version {old} -> {next}; clean runtime self-healed without a release; healthy fast path did not rewrite runtime; real pre-commit accepted; runtime and index verified; local remote parity 0/0"
    );
}

#[test]
#[ignore = "explicit factory performance experiment using isolated local Git only"]
fn factory_ordinary_sync_performance() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let fixture =
        Fixture(std::env::temp_dir().join(format!("bf-sync-perf-{}-{nonce}", std::process::id())));
    let root = fixture.0.join("factory");
    copy_tree(source, &root);
    fixture_configuration(&root);
    let config_path = root.join(".codex/bridgeforge-version.json");
    let mut config: serde_json::Value =
        serde_json::from_slice(&fs::read(&config_path).unwrap()).unwrap();
    config["release_policy"] = serde_json::json!("explicit_release");
    fs::write(config_path, serde_json::to_vec_pretty(&config).unwrap()).unwrap();
    git(&root, &["init"]);
    git(&root, &["config", "user.name", "BridgeForge Benchmark"]);
    git(
        &root,
        &["config", "user.email", "benchmark@example.invalid"],
    );
    git(&root, &["config", "core.autocrlf", "false"]);
    git(&root, &["config", "core.hooksPath", ".git/hooks"]);
    git(&root, &["add", "."]);
    git(&root, &["commit", "-m", "chore: benchmark baseline"]);
    git(&root, &["config", "core.hooksPath", ".githooks"]);
    let remote = fixture.0.join("origin.git");
    git(&root, &["init", "--bare", remote.to_str().unwrap()]);
    git(
        &root,
        &["remote", "add", "origin", remote.to_str().unwrap()],
    );
    git(&root, &["push", "-u", "origin", "HEAD"]);
    let version = fs::read(root.join("VERSION")).unwrap();
    let cli = root.join(if cfg!(windows) {
        ".codex/bin/bridgeforge.exe"
    } else {
        ".codex/bin/bridgeforge"
    });
    super::schema4_fixture::migrate(&cli, &root);
    let cli_before = fs::metadata(&cli).unwrap().modified().unwrap();
    let mut readme = fs::read(root.join("README.md")).unwrap();
    readme.extend_from_slice(b"\nPerformance fixture: documentation-only change.\n");
    fs::write(root.join("README.md"), readme).unwrap();
    let mut request = ProcessRequest::new(cli.as_os_str(), &root);
    request.args = ["git-sync", "--message", "docs: benchmark ordinary sync"]
        .iter()
        .map(Into::into)
        .collect();
    request.timeout = Duration::from_secs(1200);
    println!("performance fixture prepared; timed CLI starts now");
    let started = std::time::Instant::now();
    let output = SystemProcessRunner.run(&request).unwrap();
    let elapsed_ms = started.elapsed().as_millis();
    assert!(
        !output.timed_out && output.code == 0,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let receipt: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(receipt["version_bumped"], false);
    assert_eq!(fs::read(root.join("VERSION")).unwrap(), version);
    assert_eq!(receipt["working_tree"], "clean");
    assert_eq!(receipt["ahead"], 0);
    assert_eq!(receipt["behind"], 0);
    let sync_elapsed_ms = receipt["sync_elapsed_ms"].as_u64().expect("sync timing");
    assert!(u128::from(sync_elapsed_ms) <= elapsed_ms);
    if std::env::var_os("BRIDGEFORGE_PERF_EXPECT_REUSE").is_some() {
        assert_eq!(receipt["generated_assets_reused"], 2);
        assert_eq!(receipt["generated_assets_built"], 0);
        assert_eq!(fs::metadata(&cli).unwrap().modified().unwrap(), cli_before);
    }
    println!(
        "PERF {}",
        serde_json::json!({"elapsed_ms": elapsed_ms, "receipt": receipt})
    );
    if std::env::var_os("BRIDGEFORGE_PERF_RELEASE").is_some() {
        let mut readme = fs::read(root.join("README.md")).unwrap();
        readme.extend_from_slice(b"\nPerformance fixture: explicit release.\n");
        fs::write(root.join("README.md"), readme).unwrap();
        super::schema4_fixture::analyze(
            &cli,
            &root,
            "perf: explicit fixture release",
            "patch",
            &[],
            &[],
        );
        request.args.insert(1, "--release".into());
        request.args.extend([
            "--release-analysis-file".into(),
            ".runtime/schema4-analysis.json".into(),
        ]);
        println!("performance explicit release starts now");
        let started = std::time::Instant::now();
        let output = SystemProcessRunner.run(&request).unwrap();
        let elapsed_ms = started.elapsed().as_millis();
        assert!(
            !output.timed_out && output.code == 0,
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let receipt: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(receipt["version_bumped"], true);
        assert_eq!(receipt["generated_assets_reused"], 0);
        assert_eq!(receipt["generated_assets_built"], 2);
        assert_eq!(receipt["working_tree"], "clean");
        assert_eq!(receipt["ahead"], 0);
        assert_eq!(receipt["behind"], 0);
        println!(
            "PERF_RELEASE {}",
            serde_json::json!({"elapsed_ms": elapsed_ms, "receipt": receipt})
        );
    }
}
