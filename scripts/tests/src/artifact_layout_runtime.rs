//! Offline CLI measurements. Every mutation belongs to a disposable fixture.
use bridgeforge_core::{ProcessOutput, ProcessRequest, ProcessRunner, SystemProcessRunner};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn call(root: &Path, program: &Path, args: &[&str]) -> (ProcessOutput, u128) {
    let mut request = ProcessRequest::new(program.as_os_str(), root);
    request.args = args.iter().map(Into::into).collect();
    request.timeout = Duration::from_secs(180);
    let start = Instant::now();
    let output = SystemProcessRunner.run(&request).unwrap();
    let elapsed = start.elapsed().as_millis();
    assert_eq!(
        output.code,
        0,
        "{program:?} {args:?}: {} {}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(!output.timed_out);
    (output, elapsed)
}
fn git(root: &Path, args: &[&str]) -> String {
    String::from_utf8(call(root, Path::new("git"), args).0.stdout)
        .unwrap()
        .trim()
        .into()
}
fn write(root: &Path, path: &str, bytes: &[u8]) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}
fn setup(cli: &Path, label: &str, configured: bool, modern: bool) -> Fixture {
    let token = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let fixture = Fixture(std::env::temp_dir().join(format!("bf-layout-{label}-{token}")));
    let root = fixture.0.join("project");
    fs::create_dir_all(&root).unwrap();
    write(&root, "VERSION", b"1.0.0\n");
    write(&root, "CHANGELOG.md", b"# Changelog\n");
    write(
        &root,
        "Cargo.toml",
        b"[package]\nname='layout-fixture'\nversion='1.0.0'\nedition='2024'\n",
    );
    write(
        &root,
        "Cargo.lock",
        b"version = 4\n[[package]]\nname = 'layout-fixture'\nversion = '1.0.0'\n",
    );
    write(&root, "src/lib.rs", b"pub fn sample() {}\n");
    write(&root, ".gitignore", b".runtime/\n");
    use sha2::{Digest, Sha256};
    write(&root, "templates/seed.txt", b"seed\n");
    write(&root, "seed.txt", b"seed\n");
    write(&root, ".codex/.bridgeforge_codex_version", b"8.0.0\n");
    write(&root, ".codex/managed-skeleton.json", &serde_json::to_vec(&json!({
        "schema_version":4,"release_version":"8.0.0","host":"codex",
        "stamp":".codex/.bridgeforge_codex_version","contract_target":".codex/managed-skeleton.json",
        "baseline_model":"current-only","compatibility_baseline":"8.0.0","generated_assets":[],
        "assets":[{"id":"fixture.seed","source":"templates/seed.txt","target":"seed.txt","strategy":"whole","current_sha256":format!("sha256:{:x}",Sha256::digest(b"seed\n"))}]
    })).unwrap());
    write(
        &root,
        ".codex/bridgeforge-version.json",
        br#"{"schema_version":1,"release_policy":"explicit_release","manifests":["Cargo.toml"]}"#,
    );
    if configured {
        write(&root, ".codex/artifact-layout.json", br#"{"schema":1,"source_roots":["src/"],"tests_root":"scripts/tests/","tools_root":"scripts/","python_artifacts":{"enabled":true,"allowed_roots":[".runtime/"]},"forbidden_source_components":["tests","benches","examples"],"cargo_registration":{"manifest_roots":["src/"],"forbidden_target_roots":["scripts/"]}}"#);
    }
    for layer in [
        "0_architecture",
        "1_delivery",
        "2_bugs",
        "3_reference",
        "4_archive",
        "5_project_knowledgebase",
    ] {
        fs::create_dir_all(root.join("doc").join(layer)).unwrap();
    }
    write(
        &root,
        "doc/README.md",
        b"---\ndelivery_layout: flat\n---\n[demo](1_delivery/demo/requirements_demo.md)\n",
    );
    write(
        &root,
        "doc/1_delivery/demo/requirements_demo.md",
        b"---\nlifecycle: active\nvalidation_status: not_started\n---\n# Demo\n",
    );
    for n in 0..16 {
        write(
            &root,
            &format!("doc/1_delivery/demo/note_{n}.md"),
            b"---\nlifecycle: active\nvalidation_status: not_started\n---\n# Note\n",
        );
    }
    git(&root, &["init"]);
    git(&root, &["config", "user.name", "Layout Fixture"]);
    git(&root, &["config", "user.email", "layout@example.invalid"]);
    git(&root, &["config", "core.hooksPath", ".githooks"]);
    git(&root, &["add", "."]);
    git(&root, &["commit", "-m", "chore: fixture seed"]);
    super::schema4_fixture::migrate(cli, &root);
    let cli_path = cli
        .to_string_lossy()
        .replace('\\', "/")
        .replace('\'', "'\\''");
    write(
        &root,
        ".githooks/pre-commit",
        format!(
            "#!/bin/sh\nexec '{cli_path}' check project-structure --root .{}\n",
            if modern { " --pre-commit" } else { "" }
        )
        .as_bytes(),
    );
    git(&root, &["add", "."]);
    git(&root, &["commit", "-m", "chore: fixture ready"]);
    let remote = fixture.0.join("remote.git");
    git(&root, &["init", "--bare", remote.to_str().unwrap()]);
    git(
        &root,
        &["remote", "add", "origin", remote.to_str().unwrap()],
    );
    git(&root, &["push", "-u", "origin", "HEAD"]);
    fixture
}

#[test]
fn artifact_layout_real_git_sync_blocks_only_new_violations_and_keeps_remote() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap();
    let cli = source.join(if cfg!(windows) {
        ".codex/bin/bridgeforge.exe"
    } else {
        ".codex/bin/bridgeforge"
    });
    let fixture = setup(&cli, "workflow", true, true);
    let root = fixture.0.join("project");
    let original = git(&root, &["rev-parse", "HEAD"]);
    write(&root, "tests/new.rs", b"fn probe() {}\n");
    let mut request = ProcessRequest::new(cli.as_os_str(), &root);
    request.args = ["git-sync", "--message", "fix: rejected layout"]
        .iter()
        .map(Into::into)
        .collect();
    request.timeout = Duration::from_secs(180);
    let failure = SystemProcessRunner.run(&request).unwrap();
    assert_ne!(failure.code, 0);
    assert!(
        String::from_utf8_lossy(&failure.stderr).contains("legacy-test-root"),
        "{}",
        String::from_utf8_lossy(&failure.stderr)
    );
    assert_eq!(git(&root, &["rev-parse", "HEAD"]), original);
    assert_eq!(git(&root, &["rev-parse", "@{push}"]), original);
    fs::remove_file(root.join("tests/new.rs")).unwrap();
    write(&root, "src/lib.rs", b"pub fn changed() {}\n");
    let output = super::schema4_fixture::invoke(
        &cli,
        &root,
        &["git-sync", "--message", "fix: valid layout"],
        &[],
    );
    assert_eq!(output["status"], "synced");
    assert_eq!(output["version_bumped"], false);
    assert_eq!(
        fs::read_to_string(root.join("VERSION")).unwrap().trim(),
        "1.0.0"
    );
    assert_eq!(
        git(
            &root,
            &["rev-list", "--left-right", "--count", "HEAD...@{push}"]
        ),
        "0\t0"
    );
    // An ignored historical cache is visible to audit, but cannot block A mode.
    write(&root, ".gitignore", b".runtime/\nlegacy-cache/\n");
    git(&root, &["add", ".gitignore"]);
    git(&root, &["commit", "-m", "chore: ignore legacy cache"]);
    write(&root, "legacy-cache/__pycache__/old.pyc", b"old\n");
    write(&root, "src/lib.rs", b"pub fn another() {}\n");
    let output = super::schema4_fixture::invoke(
        &cli,
        &root,
        &["git-sync", "--message", "fix: unrelated cache"],
        &[],
    );
    assert_eq!(output["status"], "synced");
    let mut request = ProcessRequest::new(cli.as_os_str(), &root);
    request.args = ["check", "project-structure", "--root", "."]
        .iter()
        .map(Into::into)
        .collect();
    let audit = SystemProcessRunner.run(&request).unwrap();
    assert_ne!(audit.code, 0);
    assert!(String::from_utf8_lossy(&audit.stdout).contains("misplaced-python-artifact"));
}

#[test]
fn artifact_layout_defaults_block_nested_tests_without_project_configuration() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap();
    let cli = source.join(if cfg!(windows) {
        ".codex/bin/bridgeforge.exe"
    } else {
        ".codex/bin/bridgeforge"
    });
    let fixture = setup(&cli, "defaults", false, true);
    let root = fixture.0.join("project");
    let original = git(&root, &["rev-parse", "HEAD"]);
    write(&root, "src/tests/new.rs", b"fn probe() {}\n");
    let mut request = ProcessRequest::new(cli.as_os_str(), &root);
    request.args = ["git-sync", "--message", "fix: rejected nested test"]
        .iter()
        .map(Into::into)
        .collect();
    request.timeout = Duration::from_secs(180);
    let output = SystemProcessRunner.run(&request).unwrap();
    assert_ne!(output.code, 0);
    assert!(String::from_utf8_lossy(&output.stderr).contains("misplaced-test-directory"));
    assert_eq!(git(&root, &["rev-parse", "HEAD"]), original);
    assert_eq!(git(&root, &["rev-parse", "@{push}"]), original);
    fs::remove_file(root.join("src/tests/new.rs")).unwrap();
    write(&root, "scripts/tests/new.rs", b"fn probe() {}\n");
    let receipt = super::schema4_fixture::invoke(
        &cli,
        &root,
        &["git-sync", "--message", "fix: valid nested test"],
        &[],
    );
    assert_eq!(receipt["status"], "synced");
    assert!(!root.join(".codex/artifact-layout.json").exists());
}

#[test]
#[ignore = "explicit artifact-layout offline performance baseline/comparison"]
fn artifact_layout_performance() {
    use sha2::{Digest, Sha256};
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap();
    let defaults_comparison =
        std::env::var("BRIDGEFORGE_LAYOUT_PERF_DEFAULTS").as_deref() == Ok("1");
    let baseline = source.join(if defaults_comparison {
        ".runtime/artifact-layout-defaults/baseline/bridgeforge.exe"
    } else {
        ".runtime/artifact-layout/baseline/bridgeforge.exe"
    });
    let current = source.join(".codex/bin/bridgeforge.exe");
    let baseline_only = std::env::var("BRIDGEFORGE_LAYOUT_PERF_PHASE").as_deref() == Ok("baseline");
    let focused = std::env::var("BRIDGEFORGE_LAYOUT_PERF_SCENARIOS").ok();
    let mut measurements = Vec::new();
    let scenarios = [
        "clean",
        "small",
        "batch",
        "documents",
        "unconfigured",
        "release-preview",
        "release",
        "configuration",
        "minimal-configuration",
        "deleted-configuration",
    ];
    if let Some(selected) = &focused {
        assert!(selected.split(',').all(|item| scenarios.contains(&item)));
    }
    for scenario in scenarios {
        if !defaults_comparison
            && ["minimal-configuration", "deleted-configuration"].contains(&scenario)
        {
            continue;
        }
        if focused
            .as_ref()
            .is_some_and(|selected| !selected.split(',').any(|item| item == scenario))
        {
            continue;
        }
        for sample in 0..5 {
            for modern in if sample % 2 == 0 {
                [false, true]
            } else {
                [true, false]
            } {
                if modern && baseline_only {
                    continue;
                }
                let cli = if modern { &current } else { &baseline };
                let configured = !["documents", "unconfigured"].contains(&scenario);
                // Both candidates already use the staged entry point in a defaults comparison.
                let fixture = setup(cli, scenario, configured, modern || defaults_comparison);
                let root = fixture.0.join("project");
                match scenario {
                    "clean" => (),
                    "documents" => {
                        write(&root, "doc/1_delivery/demo/note_0.md", b"# Revised note\n")
                    }
                    "batch" => {
                        for n in 0..100 {
                            write(&root, &format!("src/item_{n}.rs"), b"pub fn sample() {}\n");
                        }
                    }
                    "configuration" => {
                        let path = root.join(".codex/artifact-layout.json");
                        let mut cfg: Value =
                            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
                        cfg["forbidden_source_components"] =
                            json!(["tests", "benches", "examples", "scripts"]);
                        fs::write(path, serde_json::to_vec(&cfg).unwrap()).unwrap();
                    }
                    "minimal-configuration" => {
                        write(&root, ".codex/artifact-layout.json", br#"{"schema":1}"#)
                    }
                    "deleted-configuration" => {
                        fs::remove_file(root.join(".codex/artifact-layout.json")).unwrap()
                    }
                    _ => write(&root, "src/lib.rs", b"pub fn sample() { let _ = 1; }\n"),
                }
                let message = "fix: layout fixture";
                let mut args = vec!["git-sync", "--message", message];
                if scenario == "release" {
                    super::schema4_fixture::analyze(cli, &root, message, "patch", &[], &[]);
                    args.extend([
                        "--release",
                        "--release-analysis-file",
                        ".runtime/schema4-analysis.json",
                    ]);
                } else if scenario == "release-preview" {
                    args.push("--release-preview");
                }
                let (output, elapsed) = call(&root, cli, &args);
                let receipt: Value = serde_json::from_slice(&output.stdout).unwrap();
                if scenario == "release-preview" {
                    assert_eq!(receipt["status"], "analysis-required");
                } else {
                    assert_eq!(receipt["status"], "synced");
                    assert_eq!(receipt["version_bumped"], scenario == "release");
                    assert_eq!(receipt["generated_assets_built"], 0);
                    assert_eq!(receipt["ahead"], 0);
                    assert_eq!(receipt["behind"], 0);
                }
                measurements.push(json!({"scenario":scenario,"sample":sample,"variant":if modern {"merged"}else{"baseline"},"configured":configured,"elapsed_ms":elapsed,"sync_elapsed_ms":receipt["sync_elapsed_ms"],"timings_ms":receipt["timings_ms"],"generated_assets_built":receipt["generated_assets_built"],"generated_assets_reused":receipt["generated_assets_reused"]}));
            }
        }
        println!("performance scenario completed: {scenario}");
    }
    let result = json!({"scope":"offline_cli_local_bare_remote_not_chat_end_to_end","samples_per_variant":5,"baseline_sha256":format!("{:x}",Sha256::digest(fs::read(&baseline).unwrap())),"current_sha256":format!("{:x}",Sha256::digest(fs::read(&current).unwrap())),"measurements":measurements});
    let file = source.join(if defaults_comparison && focused.is_some() {
        ".runtime/artifact-layout-defaults/performance-focused.json"
    } else if defaults_comparison {
        ".runtime/artifact-layout-defaults/performance-comparison.json"
    } else if focused.is_some() {
        ".runtime/artifact-layout/performance-focused.json"
    } else if baseline_only {
        ".runtime/artifact-layout/performance-baseline.json"
    } else {
        ".runtime/artifact-layout/performance-comparison.json"
    });
    fs::write(file, serde_json::to_vec_pretty(&result).unwrap()).unwrap();
    println!(
        "artifact-layout performance: {} measurements",
        result["measurements"].as_array().unwrap().len()
    );
}
