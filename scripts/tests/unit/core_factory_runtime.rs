#[path = "../support/factory_root.rs"]
mod factory_repository;

use super::*;
use crate::{ProcessOutput, ProcessRequest};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

struct Factory(PathBuf);

#[test]
#[ignore = "child process proves strict inspection needs neither a temp directory nor Git"]
fn strict_inspection_child() {
    struct NoProcesses;
    impl ProcessRunner for NoProcesses {
        fn run(&self, _: &ProcessRequest) -> std::io::Result<ProcessOutput> {
            panic!("strict inspection launched a process");
        }
    }
    let root = PathBuf::from(std::env::var_os("BFC_STRICT_FACTORY_ROOT").unwrap());
    let result = prepare(&root, true, &NoProcesses);
    assert_eq!(result.code, 0, "{}", result.stderr);
}

#[test]
fn strict_inspection_succeeds_with_no_temp_parent_and_no_git_on_path() {
    let fixture = Factory::new();
    let runner = Runner::new(&fixture.0, "ok");
    let built = prepare(&fixture.0, false, &runner);
    assert_eq!(built.code, 0, "{}", built.stderr);
    let before = tree(&fixture.0);
    let missing = fixture.0.join("strict-temp-must-not-exist");
    let mut request = ProcessRequest::new(
        std::env::current_exe().unwrap().into_os_string(),
        &fixture.0,
    );
    request.args = [
        "--exact",
        "factory_runtime::tests::strict_inspection_child",
        "--ignored",
        "--nocapture",
    ]
    .iter()
    .map(Into::into)
    .collect();
    request.env.insert(
        "BFC_STRICT_FACTORY_ROOT".into(),
        fixture.0.clone().into_os_string(),
    );
    request
        .env
        .insert("TEMP".into(), missing.clone().into_os_string());
    request
        .env
        .insert("TMP".into(), missing.clone().into_os_string());
    request.env.insert("PATH".into(), "".into());
    let result = crate::SystemProcessRunner.run(&request).unwrap();
    assert_eq!(
        result.code,
        0,
        "{} {}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(!missing.exists());
    assert_eq!(tree(&fixture.0), before);
}

#[test]
fn mixed_product_release_materializer_updates_consumer_lock_without_bumping_harness() {
    let fixture = Factory::new();
    let root = &fixture.0;
    let repository = factory_repository::from_manifest_dir(env!("CARGO_MANIFEST_DIR"));
    fs::create_dir_all(root.join("scripts/tests")).unwrap();
    for name in ["Cargo.toml", "Cargo.lock"] {
        fs::copy(
            repository.join("scripts/tests").join(name),
            root.join("scripts/tests").join(name),
        )
        .unwrap();
    }
    let before = tree(root);
    let mut cargo = fs::read_to_string(root.join("Cargo.toml"))
        .unwrap()
        .parse::<toml_edit::DocumentMut>()
        .unwrap();
    let next = "7.0.0";
    cargo["workspace"]["package"]["version"] = toml_edit::value(next);
    let mut lock = fs::read_to_string(root.join("Cargo.lock"))
        .unwrap()
        .parse::<toml_edit::DocumentMut>()
        .unwrap();
    for package in lock["package"].as_array_of_tables_mut().unwrap().iter_mut() {
        if matches!(
            package["name"].as_str(),
            Some("bridgeforge-cli" | "bridgeforge-hook")
        ) {
            package["version"] = toml_edit::value(next);
        }
    }
    let writes = BTreeMap::from([
        (root.join("VERSION"), format!("{next}\n").into_bytes()),
        (root.join("Cargo.toml"), cargo.to_string().into_bytes()),
        (root.join("Cargo.lock"), lock.to_string().into_bytes()),
    ]);
    struct CandidateRunner(Runner);
    impl ProcessRunner for CandidateRunner {
        fn run(&self, request: &ProcessRequest) -> std::io::Result<ProcessOutput> {
            if request.program == "git" && request.args.first().is_some_and(|a| a == "check-ignore")
            {
                // This fixture exercises candidate materialization, with no
                // persistent cache policy in its placeholder Git directory.
                return Ok(ProcessOutput {
                    code: 1,
                    stdout: vec![],
                    stderr: vec![],
                    timed_out: false,
                });
            }
            if Path::new(&request.program)
                .file_stem()
                .and_then(|s| s.to_str())
                == Some("bridgeforge-factory")
            {
                let cargo = fs::read_to_string(request.cwd.join("Cargo.toml"))?
                    .parse::<toml_edit::DocumentMut>()
                    .unwrap();
                let factory = fs::read_to_string(request.cwd.join("source/factory/Cargo.toml"))?
                    .parse::<toml_edit::DocumentMut>()
                    .unwrap();
                return Ok(ProcessOutput{code:0,stdout:serde_json::to_vec(&json!({"schema":1,"name":"bridgeforge-factory","status":"ok",
                    "version":factory["package"]["version"].as_str().unwrap(),"product_version":cargo["workspace"]["package"]["version"].as_str().unwrap()})).unwrap(),stderr:vec![],timed_out:false});
            }
            self.0.run(request)
        }
    }
    let runner = CandidateRunner(Runner::new(root, "ok"));
    let plan =
        crate::git_sync_plan::WritePlan::prepare(root, writes, BTreeMap::new(), true, &runner)
            .unwrap();
    let consumer = &plan.writes[&root.join("scripts/tests/Cargo.lock")];
    let parsed = std::str::from_utf8(consumer)
        .unwrap()
        .parse::<toml_edit::DocumentMut>()
        .unwrap();
    let packages = parsed["package"].as_array_of_tables().unwrap();
    let version = |name: &str| {
        packages
            .iter()
            .find(|p| p["name"].as_str() == Some(name))
            .unwrap()["version"]
            .as_str()
            .unwrap()
    };
    assert_eq!(version("bridgeforge-hook"), next);
    let original = fs::read_to_string(root.join("scripts/tests/Cargo.lock"))
        .unwrap()
        .parse::<toml_edit::DocumentMut>()
        .unwrap();
    for package in original["package"]
        .as_array_of_tables()
        .unwrap()
        .iter()
        .filter(|p| p.get("source").is_none() && p["name"].as_str() != Some("bridgeforge-hook"))
    {
        assert_eq!(
            version(package["name"].as_str().unwrap()),
            package["version"].as_str().unwrap()
        );
    }
    assert!(
        !plan
            .writes
            .contains_key(&root.join("scripts/tests/Cargo.toml"))
    );
    assert_eq!(
        tree(root),
        before,
        "candidate preparation must not install release state"
    );
    fs::write(
        root.join("scripts/tests/Cargo.lock"),
        b"external consumer lock",
    )
    .unwrap();
    assert!(
        plan.verify_unchanged(root)
            .unwrap_err()
            .contains("input changed concurrently")
    );
    assert_eq!(
        fs::read(root.join("scripts/tests/Cargo.lock")).unwrap(),
        b"external consumer lock"
    );
}
impl Factory {
    fn new() -> Self {
        let repository = factory_repository::from_manifest_dir(env!("CARGO_MANIFEST_DIR"));
        let root = std::env::temp_dir().join(format!(
            "bf-factory-runtime-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(root.join(".git")).unwrap();
        let copy = |source: &str, target: &str| {
            let destination = root.join(target);
            fs::create_dir_all(destination.parent().unwrap()).unwrap();
            fs::copy(repository.join(source), destination).unwrap();
        };
        for path in [
            "VERSION",
            "CHANGELOG.md",
            "AGENTS.md",
            "templates/managed-skeleton.json",
            "bridgeforge-codex-manifest.json",
            "Cargo.toml",
            "Cargo.lock",
            "templates/source-layout.json",
            "templates/hooks/.gitignore",
            crate::user_agents::SOURCE,
            crate::user_agents::MANIFEST,
        ] {
            copy(path, path);
        }
        let contract: Value = serde_json::from_slice(
            &fs::read(repository.join("templates/managed-skeleton.json")).unwrap(),
        )
        .unwrap();
        for asset in contract["assets"].as_array().unwrap() {
            let source = asset["source"].as_str().unwrap();
            let target = asset["target"].as_str().unwrap();
            copy(source, source);
            if target != "AGENTS.md" && asset.get("render").is_none() {
                copy(source, target);
            }
        }
        // Copy all nine canonical packages, including private factory sources.
        // The public mirror must come from the pure eight-package projection.
        for relative in crate::source_projection::render(repository, false)
            .unwrap()
            .keys()
        {
            if relative.starts_with("source/") {
                copy(relative, relative);
            }
        }
        for (relative, bytes) in crate::source_projection::render(&root, true).unwrap() {
            let target = root.join(".codex/hooks").join(relative);
            fs::create_dir_all(target.parent().unwrap()).unwrap();
            fs::write(target, bytes).unwrap();
        }
        let distribution: Value = serde_json::from_slice(
            &fs::read(root.join("bridgeforge-codex-manifest.json")).unwrap(),
        )
        .unwrap();
        for platform in distribution["platforms"].as_object().unwrap().values() {
            for skill in platform["skills"].as_array().unwrap() {
                for file in skill["files"].as_array().unwrap() {
                    let source = file["source"].as_str().unwrap();
                    copy(source, source);
                }
            }
        }
        manifest::rebuild(&root, false).unwrap();
        identity(&root).unwrap();
        Self(root)
    }
}
impl Drop for Factory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

struct Runner {
    root: PathBuf,
    calls: AtomicUsize,
    builds: AtomicUsize,
    failure: &'static str,
}
impl Runner {
    fn new(root: &Path, failure: &'static str) -> Self {
        Self {
            root: root.canonicalize().unwrap(),
            calls: AtomicUsize::new(0),
            builds: AtomicUsize::new(0),
            failure,
        }
    }
}
impl ProcessRunner for Runner {
    fn run(&self, request: &ProcessRequest) -> std::io::Result<ProcessOutput> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        let executable = Path::new(&request.program)
            .file_stem()
            .unwrap()
            .to_string_lossy();
        let args: Vec<_> = request
            .args
            .iter()
            .map(|s| s.to_string_lossy().to_string())
            .collect();
        let mut out = ProcessOutput {
            code: 0,
            stdout: Vec::new(),
            stderr: Vec::new(),
            timed_out: false,
        };
        if executable == "git" {
            assert!(args.iter().any(|s| s == "--git-common-dir"));
            out.stdout = self.root.join(".git").to_string_lossy().as_bytes().to_vec();
        } else if executable == "cargo" {
            self.builds.fetch_add(1, Ordering::Relaxed);
            if self.failure == "build" {
                out.code = 1;
                return Ok(out);
            }
            let value = |flag: &str| args[args.iter().position(|s| s == flag).unwrap() + 1].clone();
            assert!(args.contains(&"--locked".into()));
            let name = value("--bin");
            let target =
                PathBuf::from(value("--target-dir"))
                    .join("release")
                    .join(if cfg!(windows) {
                        format!("{name}.exe")
                    } else {
                        name.clone()
                    });
            fs::create_dir_all(target.parent().unwrap())?;
            fs::write(target, format!("verified-fixture-{name}"))?;
            if self.failure == "source" {
                fs::write(
                    self.root.join("source/hook/src/main.rs"),
                    b"concurrent change",
                )?;
            }
            if self.failure == "target" {
                fs::create_dir_all(self.root.join(".codex/bin"))?;
                fs::write(
                    self.root.join(".codex/bin/build-receipt-cli.json"),
                    b"external change",
                )?;
            }
        } else {
            let contract: Value =
                serde_json::from_slice(&fs::read(self.root.join(".codex/managed-skeleton.json"))?)
                    .unwrap();
            let runtime = manifest::runtime_contract(&self.root, &contract).unwrap();
            let asset = runtime["generated_assets"]
                .as_array()
                .unwrap()
                .iter()
                .find(|a| a["build"]["binary_name"] == executable.as_ref())
                .unwrap();
            out.stdout = serde_json::to_vec(&asset["self_test"]["expected_json"]).unwrap();
            if self.failure == "installed-test"
                && Path::new(&request.program).starts_with(self.root.join(".codex/bin"))
            {
                out.code = 1;
            }
        }
        Ok(out)
    }
}

fn tree(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    walkdir::WalkDir::new(root)
        .into_iter()
        .map(Result::unwrap)
        .filter(|e| e.file_type().is_file())
        .map(|e| {
            (
                e.path().strip_prefix(root).unwrap().to_path_buf(),
                fs::read(e.path()).unwrap(),
            )
        })
        .collect()
}

#[test]
fn factory_prepare_checks_without_writes_and_reuses_healthy_cli_and_hook() {
    let fixture = Factory::new();
    let runner = Runner::new(&fixture.0, "");
    let original = tree(&fixture.0);
    assert_eq!(prepare(&fixture.0, true, &runner).code, 2);
    assert_eq!(runner.calls.load(Ordering::Relaxed), 0);
    assert_eq!(tree(&fixture.0), original);
    let result = prepare(&fixture.0, false, &runner);
    assert_eq!(result.code, 0, "{}", result.stderr);
    assert_eq!(runner.builds.load(Ordering::Relaxed), 3);
    let installed = tree(&fixture.0);
    let contract = identity(&fixture.0).unwrap().0;
    let times: Vec<_> = contract["generated_assets"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|asset| {
            let (binary, receipt) = paths(&fixture.0, asset).unwrap();
            [binary, receipt]
        })
        .map(|path| {
            (
                path.clone(),
                fs::metadata(path).unwrap().modified().unwrap(),
            )
        })
        .collect();
    let calls = runner.calls.load(Ordering::Relaxed);
    assert_eq!(prepare(&fixture.0, true, &runner).code, 0);
    assert_eq!(runner.calls.load(Ordering::Relaxed), calls);
    assert_eq!(prepare(&fixture.0, false, &runner).code, 0);
    assert_eq!(runner.builds.load(Ordering::Relaxed), 3);
    assert_eq!(tree(&fixture.0), installed);
    for (path, time) in times {
        assert_eq!(fs::metadata(path).unwrap().modified().unwrap(), time);
    }
    fs::write(
        fixture.0.join(".codex/bin/build-receipt-cli.json"),
        b"invalid",
    )
    .unwrap();
    assert_eq!(prepare(&fixture.0, false, &runner).code, 0);
    assert_eq!(runner.builds.load(Ordering::Relaxed), 4);
}

#[test]
fn factory_prepare_stops_on_manifest_mirror_source_build_or_target_drift() {
    for failure in [
        "manifest",
        "mirror",
        "build",
        "source",
        "target",
        "installed-test",
    ] {
        let fixture = Factory::new();
        let runner = Runner::new(&fixture.0, failure);
        if failure == "manifest" {
            fs::write(fixture.0.join("source/hook/src/main.rs"), b"stale source").unwrap();
        }
        if failure == "mirror" {
            fs::write(
                fixture.0.join(".codex/hooks/source/hook/src/main.rs"),
                b"mirror drift",
            )
            .unwrap();
        }
        let result = prepare(&fixture.0, false, &runner);
        assert_eq!(result.code, 2, "{failure}: {}", result.stderr);
        assert!(!fixture.0.join(".codex/bin/bridgeforge.exe").exists());
        assert!(!fixture.0.join(".codex/bin/bridgeforge").exists());
        assert!(
            !fixture
                .0
                .join(".codex/bin/bridgeforge-factory.exe")
                .exists()
        );
        assert!(!fixture.0.join(".codex/bin/bridgeforge-factory").exists());
        if failure == "target" {
            assert_eq!(
                fs::read(fixture.0.join(".codex/bin/build-receipt-cli.json")).unwrap(),
                b"external change"
            );
        }
        if matches!(failure, "manifest" | "mirror") {
            assert_eq!(runner.calls.load(Ordering::Relaxed), 0);
        }
    }
}

#[test]
fn generated_install_rolls_back_own_changes_and_preserves_external_changes() {
    let fixture = Factory::new();
    let a = fixture.0.join("first.bin");
    let b = fixture.0.join("second.json");
    fs::write(&a, b"old").unwrap();
    let before = BTreeMap::from([(a.clone(), Some(b"old".to_vec())), (b.clone(), None)]);
    let writes = BTreeMap::from([
        (a.clone(), b"new".to_vec()),
        (b.clone(), b"receipt".to_vec()),
    ]);
    let result = project_sync::install_generated_transaction(
        &fixture.0,
        &writes,
        &before,
        &BTreeSet::new(),
        || Err("verification failure".into()),
    );
    assert!(result.unwrap_err().contains("rolled back"));
    assert_eq!(fs::read(&a).unwrap(), b"old");
    assert!(!b.exists());
    let result = project_sync::install_generated_transaction(
        &fixture.0,
        &writes,
        &before,
        &BTreeSet::new(),
        || {
            fs::write(&a, b"external").unwrap();
            Err("concurrent".into())
        },
    );
    assert!(result.unwrap_err().contains("external change preserved"));
    assert_eq!(fs::read(&a).unwrap(), b"external");
    assert!(!b.exists());
}

#[test]
fn factory_prepare_serializes_with_sync_and_detects_binary_lock_and_recipe_drift() {
    let fixture = Factory::new();
    let runner = Runner::new(&fixture.0, "");
    let lock = crate::git_sync::lock_factory_runtime(&fixture.0, &runner).unwrap();
    let result = prepare(&fixture.0, false, &runner);
    assert_eq!(result.code, 2);
    assert!(result.stderr.contains("lock"));
    assert_eq!(runner.builds.load(Ordering::Relaxed), 0);
    drop(lock);
    assert_eq!(prepare(&fixture.0, false, &runner).code, 0);
    let contract = identity(&fixture.0).unwrap().0;
    let (binary, _) = paths(&fixture.0, &contract["generated_assets"][0]).unwrap();
    fs::write(&binary, b"corrupt executable").unwrap();
    assert_eq!(prepare(&fixture.0, true, &runner).code, 2);
    assert_eq!(prepare(&fixture.0, false, &runner).code, 0);
    assert_eq!(runner.builds.load(Ordering::Relaxed), 4);
    let path = fixture.0.join("Cargo.lock");
    let mut bytes = b"# changed lock input\n".to_vec();
    bytes.extend_from_slice(&fs::read(&path).unwrap());
    fs::write(path, bytes).unwrap();
    for (relative, bytes) in crate::source_projection::render(&fixture.0, true).unwrap() {
        fs::write(fixture.0.join(".codex/hooks").join(relative), bytes).unwrap();
    }
    assert_eq!(
        prepare(&fixture.0, false, &runner).code,
        2,
        "stale manifest must not be repaired by preparation"
    );
    manifest::rebuild(&fixture.0, false).unwrap();
    assert_eq!(prepare(&fixture.0, false, &runner).code, 0);
    assert_eq!(runner.builds.load(Ordering::Relaxed), 7);
    let path = fixture.0.join(".codex/managed-skeleton.json");
    let mut altered: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    altered["generated_assets"][0]["build"]["args"] = json!(["unsafe"]);
    fs::write(path, serde_json::to_vec(&altered).unwrap()).unwrap();
    let calls = runner.calls.load(Ordering::Relaxed);
    assert_eq!(prepare(&fixture.0, false, &runner).code, 2);
    assert_eq!(runner.calls.load(Ordering::Relaxed), calls);
}

#[cfg(windows)]
#[test]
fn generated_install_locked_target_restores_earlier_receipt() {
    use std::os::windows::fs::OpenOptionsExt;
    let fixture = Factory::new();
    let receipt = fixture.0.join("a-receipt.json");
    let binary = fixture.0.join("z-image.exe");
    fs::write(&receipt, b"old receipt").unwrap();
    fs::write(&binary, b"old binary").unwrap();
    let before = BTreeMap::from([
        (receipt.clone(), Some(b"old receipt".to_vec())),
        (binary.clone(), Some(b"old binary".to_vec())),
    ]);
    let writes = BTreeMap::from([
        (receipt.clone(), b"new receipt".to_vec()),
        (binary.clone(), b"new binary".to_vec()),
    ]);
    let held = fs::OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(&binary)
        .unwrap();
    let result = project_sync::install_generated_transaction(
        &fixture.0,
        &writes,
        &before,
        &BTreeSet::from([binary.clone()]),
        || Ok(()),
    );
    assert!(result.is_err());
    assert_eq!(fs::read(&receipt).unwrap(), b"old receipt");
    assert_eq!(fs::read(&binary).unwrap(), b"old binary");
    drop(held);
}
