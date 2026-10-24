use super::*;
use crate::{ProcessOutput, ProcessRequest};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

struct Factory(PathBuf);
impl Factory {
    fn new() -> Self {
        let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(4)
            .unwrap();
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
            if target != "AGENTS.md" {
                copy(source, target);
            }
        }
        // Include newly introduced Rust sources before the real manifest is rebuilt.
        for relative in manifest::generated_sources(&repository.join("templates/hooks")).unwrap() {
            copy(
                &format!("templates/hooks/{relative}"),
                &format!("templates/hooks/{relative}"),
            );
            copy(
                &format!("templates/hooks/{relative}"),
                &format!(".codex/hooks/{relative}"),
            );
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
                    self.root.join(".codex/hooks/src/main.rs"),
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
            let asset = contract["generated_assets"]
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
    assert_eq!(runner.builds.load(Ordering::Relaxed), 2);
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
    assert_eq!(runner.builds.load(Ordering::Relaxed), 2);
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
    assert_eq!(runner.builds.load(Ordering::Relaxed), 3);
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
            fs::write(
                fixture.0.join("templates/hooks/src/main.rs"),
                b"stale source",
            )
            .unwrap();
        }
        if failure == "mirror" {
            fs::write(fixture.0.join(".codex/hooks/src/main.rs"), b"mirror drift").unwrap();
        }
        let result = prepare(&fixture.0, false, &runner);
        assert_eq!(result.code, 2, "{failure}: {}", result.stderr);
        assert!(!fixture.0.join(".codex/bin/bridgeforge.exe").exists());
        assert!(!fixture.0.join(".codex/bin/bridgeforge").exists());
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
    assert_eq!(runner.builds.load(Ordering::Relaxed), 3);
    for relative in ["templates/hooks/Cargo.lock", ".codex/hooks/Cargo.lock"] {
        let path = fixture.0.join(relative);
        let mut bytes = fs::read(&path).unwrap();
        bytes.extend_from_slice(b"\n# changed lock input\n");
        fs::write(path, bytes).unwrap();
    }
    assert_eq!(
        prepare(&fixture.0, false, &runner).code,
        2,
        "stale manifest must not be repaired by preparation"
    );
    manifest::rebuild(&fixture.0, false).unwrap();
    assert_eq!(prepare(&fixture.0, false, &runner).code, 0);
    assert_eq!(runner.builds.load(Ordering::Relaxed), 5);
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
