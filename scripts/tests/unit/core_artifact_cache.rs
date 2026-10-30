use super::*;
use crate::{ProcessOutput, SystemProcessRunner};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let root =
            test_artifacts::root(env!("CARGO_MANIFEST_DIR"), "artifact-cache").join(format!(
                "bf-artifact-cache-{}-{}-{}",
                std::process::id(),
                now(),
                NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            ));
        fs::create_dir_all(&root).unwrap();
        let mut git = ProcessRequest::new("git", &root);
        git.args = vec!["init".into()];
        assert_eq!(SystemProcessRunner.run(&git).unwrap().code, 0);
        fs::write(root.join(".gitignore"), crate::artifact_paths::IGNORE_RULES).unwrap();
        Self(root)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

struct Runner {
    fail: bool,
    mutate: bool,
}

#[test]
fn explicit_cleanup_selects_verified_entries_and_can_retry_partial_deletion() {
    let f = Fixture::new();
    let (asset, payload, receipt) = artifact("cleanup");
    let mut cache = Cache::open(&f.0, &f.0, &runner()).unwrap();
    cache.store(&asset, &payload, &receipt, &runner()).unwrap();
    let key = Cache::key(&cache.identity(&asset)).unwrap();
    let entry = cache.directory.join(&key);
    drop(cache);
    fs::write(f.0.join(DIRECTORY).join("unknown.txt"), b"keep").unwrap();
    assert_eq!(cleanup_candidates(&f.0).unwrap(), vec![entry.clone()]);
    fs::remove_file(entry.join("bridgeforge.exe")).unwrap();
    assert_eq!(cleanup_candidates(&f.0).unwrap(), vec![entry.clone()]);
    fs::write(entry.join("foreign.bin"), b"keep").unwrap();
    assert!(cleanup_candidates(&f.0).unwrap().is_empty());
    fs::remove_file(entry.join("foreign.bin")).unwrap();
    fs::remove_file(entry.join("receipt.json")).unwrap();
    fs::remove_file(entry.join("entry.json")).unwrap();
    assert_eq!(cleanup_candidates(&f.0).unwrap(), vec![entry.clone()]);
    fs::remove_dir(entry).unwrap();
    let mut cache = Cache::open(&f.0, &f.0, &runner()).unwrap();
    cache.store(&asset, &payload, &receipt, &runner()).unwrap();
    assert_eq!(cache.load(&asset, &runner()).unwrap(), (payload, receipt));
    assert_eq!(
        fs::read(f.0.join(DIRECTORY).join("unknown.txt")).unwrap(),
        b"keep"
    );
}

#[cfg(windows)]
#[test]
fn explicit_cleanup_binds_ownership_and_removes_cache_lock_after_occupancy_ends() {
    use crate::artifact_cleanup::{BuildPolicy, Plan};
    let f = Fixture::new();
    let root = f.0.canonicalize().unwrap();
    let (asset, payload, receipt) = artifact("cleanup-drift");
    let mut cache = Cache::open(&root, &root, &runner()).unwrap();
    cache.store(&asset, &payload, &receipt, &runner()).unwrap();
    let selections = cleanup_selections(&root).unwrap();
    let entry = selections[0].path.clone();
    let policy = BuildPolicy {
        remove: selections
            .iter()
            .map(|s| {
                s.path
                    .strip_prefix(&root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect(),
        selections,
        ..BuildPolicy::default()
    };
    let blocked = Plan::capture_with_build(&root, policy.clone())
        .unwrap()
        .execute();
    assert_eq!(blocked.status, "blocked");
    assert_eq!(blocked.deleted_files, 0);
    drop(cache);
    fs::write(entry.join("foreign.bin"), b"keep").unwrap();
    assert!(Plan::capture_with_build(&root, policy.clone()).is_err());
    fs::remove_file(entry.join("foreign.bin")).unwrap();
    let plan = Plan::capture_with_build(&root, policy.clone()).unwrap();
    fs::write(entry.join("bridgeforge.exe"), b"changed").unwrap();
    let blocked = plan.execute();
    assert_eq!(blocked.status, "blocked");
    assert_eq!(blocked.deleted_files, 0);
    fs::write(entry.join("bridgeforge.exe"), &payload).unwrap();
    let complete = Plan::capture_with_build(&root, policy).unwrap().execute();
    assert_eq!(complete.status, "completed", "{:?}", complete.failures);
    assert_eq!(complete.deleted_files, 5);
    assert!(!entry.exists());
    assert!(!root.join(DIRECTORY).exists());
    assert!(Cache::open(&root, &root, &runner()).is_some());
}
impl ProcessRunner for Runner {
    fn run(&self, request: &ProcessRequest) -> std::io::Result<ProcessOutput> {
        if request.program == "cargo" || request.program == "rustc" {
            return Ok(ProcessOutput {
                code: 0,
                stdout: format!("{} 1.88.0 (fixture)\n", request.program.to_string_lossy())
                    .into_bytes(),
                stderr: Vec::new(),
                timed_out: false,
            });
        }
        if request.args.first().is_some_and(|arg| arg == "self-test") {
            if self.mutate {
                fs::write(&request.program, b"concurrent cache corruption")?;
            }
            return Ok(ProcessOutput {
                code: 0,
                stdout: if self.fail {
                    b"{}".to_vec()
                } else {
                    br#"{"schema":1,"name":"bridgeforge","status":"ok"}"#.to_vec()
                },
                stderr: Vec::new(),
                timed_out: false,
            });
        }
        SystemProcessRunner.run(request)
    }
}

fn artifact(revision: &str) -> (Value, Vec<u8>, Vec<u8>) {
    let payload = format!("compiled {revision}").into_bytes();
    let recipe = crate::manifest::generated_build_recipe("bridgeforge");
    let test = json!({"args":["self-test","--json"],"expected_json":{"schema":1,"name":"bridgeforge","status":"ok"}});
    let asset = json!({"id":"codex.bridgeforge-cli","build":recipe,"self_test":test,
        "source_tree_sha256":hash(revision.as_bytes()),"lockfile_sha256":hash(b"lock"),
        "build_recipe_sha256":crate::manifest::canonical_sha(&recipe).unwrap(),
        "self_test_sha256":crate::manifest::canonical_sha(&test).unwrap()});
    let receipt=serde_json::to_vec(&json!({"schema_version":2,"generated_asset_id":asset["id"],"platform":platform(),
        "binary_sha256":hash(&payload),"source_tree_sha256":asset["source_tree_sha256"],"lockfile_sha256":asset["lockfile_sha256"],
        "build_recipe_sha256":asset["build_recipe_sha256"],"self_test_sha256":asset["self_test_sha256"]})).unwrap();
    (asset, payload, receipt)
}

fn runner() -> Runner {
    Runner {
        fail: false,
        mutate: false,
    }
}

#[test]
fn actual_cargo_context_must_match_cache_environment() {
    let f = Fixture::new();
    let context = f.0.join("transient/source");
    let actual = f.0.join("cargo/public/source");
    fs::create_dir_all(&context).unwrap();
    fs::create_dir_all(&actual).unwrap();
    let cache = Cache::open_project(&f.0, &context, &runner())
        .unwrap()
        .unwrap();
    cache
        .verify_compiler_environment(&actual, &runner())
        .unwrap();
    fs::create_dir_all(f.0.join("cargo/.cargo")).unwrap();
    fs::write(
        f.0.join("cargo/.cargo/config.toml"),
        b"[build]\nrustflags=['--cfg','custom_build']\n",
    )
    .unwrap();
    assert!(
        cache
            .verify_compiler_environment(&actual, &runner())
            .is_err()
    );
}

#[test]
fn legacy_cache_migration_requires_exact_owned_entries_and_retains_lock_inode() {
    let f = Fixture::new();
    let mut cache = Cache::open_project(&f.0, &f.0, &runner()).unwrap().unwrap();
    let (asset, binary, receipt) = artifact("legacy");
    cache.store(&asset, &binary, &receipt, &runner()).unwrap();
    let current = cache.directory.clone();
    drop(cache);
    let old = f.0.join(LEGACY_DIRECTORY);
    fs::create_dir_all(old.parent().unwrap()).unwrap();
    fs::rename(current, &old).unwrap();
    let owned = legacy_files(&f.0).unwrap();
    assert_eq!(owned.len(), 4);
    let held = lock_legacy(&f.0).unwrap().unwrap();
    assert!(lock_legacy(&f.0).is_err());
    fs::write(old.join("unowned.txt"), b"preserve").unwrap();
    assert!(legacy_files(&f.0).is_err());
    fs::remove_file(old.join("unowned.txt")).unwrap();
    for path in owned.keys() {
        fs::remove_file(path).unwrap();
    }
    assert!(old.join("cache.lock").is_file());
    assert!(legacy_files(&f.0).unwrap().is_empty());
    drop(held);
}

#[test]
fn public_prepared_references_are_read_only_and_bound_to_user_and_bytes() {
    let fixture = Fixture::new();
    let user = fixture.0.join("codex");
    fs::create_dir(&user).unwrap();
    let (asset, payload, receipt) = artifact("public");
    let mut cache = Cache::open_shared(&user, &fixture.0, &runner()).unwrap();
    cache.store(&asset, &payload, &receipt, &runner()).unwrap();
    let reference = cache.prepared_reference(&asset).unwrap();
    let entry_path = cache
        .directory
        .join(reference["key"].as_str().unwrap())
        .join("entry.json");
    let before = fs::read(&entry_path).unwrap();
    drop(cache);
    for _ in 0..2 {
        assert_eq!(
            Cache::read_prepared(&user, &asset, &reference).unwrap(),
            (payload.clone(), receipt.clone())
        );
    }
    assert_eq!(
        fs::read(&entry_path).unwrap(),
        before,
        "installation cannot update cache recency"
    );
    let (different, _, _) = artifact("different");
    assert!(Cache::read_prepared(&user, &different, &reference).is_err());
    let binary = entry_path
        .parent()
        .unwrap()
        .join(binary_name(&reference["identity"]).unwrap());
    fs::write(&binary, b"unowned changed bytes").unwrap();
    assert!(Cache::read_prepared(&user, &asset, &reference).is_err());
    assert_eq!(fs::read(binary).unwrap(), b"unowned changed bytes");
}

#[test]
fn probe_cleanup_preserves_replaced_bytes_and_directory() {
    for replace_directory in [false, true] {
        let f = Fixture::new();
        let probe = Probe::create(Some(&f.0), "bridgeforge.exe", b"owned probe").unwrap();
        let binary = probe.binary.clone();
        let directory = probe.directory.clone();
        if replace_directory {
            fs::rename(&directory, f.0.join("original-probe")).unwrap();
            fs::create_dir(&directory).unwrap();
        }
        fs::write(&binary, b"unknown replacement").unwrap();
        drop(probe);
        assert_eq!(fs::read(&binary).unwrap(), b"unknown replacement");
        fs::remove_file(binary).unwrap();
        fs::remove_dir(directory).unwrap();
    }
}

#[test]
fn cached_original_change_during_probe_is_rejected_and_preserved() {
    struct MutateOriginal(PathBuf);
    impl ProcessRunner for MutateOriginal {
        fn run(&self, r: &ProcessRequest) -> std::io::Result<ProcessOutput> {
            if r.args.first().is_some_and(|a| a == "self-test") {
                fs::write(&self.0, b"external cache edit")?;
            }
            runner().run(r)
        }
    }
    let f = Fixture::new();
    let (asset, payload, receipt) = artifact("original-change");
    let mut cache = Cache::open(&f.0, &f.0, &runner()).unwrap();
    cache.store(&asset, &payload, &receipt, &runner()).unwrap();
    let key = Cache::key(&cache.identity(&asset)).unwrap();
    let original = cache
        .directory
        .join(key)
        .join(binary_name(&cache.identity(&asset)).unwrap());
    assert!(
        cache
            .load(&asset, &MutateOriginal(original.clone()))
            .is_none()
    );
    assert_eq!(fs::read(original).unwrap(), b"external cache edit");
}

#[cfg(windows)]
#[test]
fn long_cache_image_path_executes_only_verified_short_probe() {
    let f = Fixture::new();
    // A live installation may correctly be stale while develop validates new
    // native versions. Exercise real PE execution with an isolated fixture,
    // rather than requiring live binaries to match a candidate source contract.
    let source = f.0.join("cache_probe.rs");
    let build_folder = test_artifacts::build_root(env!("CARGO_MANIFEST_DIR"), "artifact-cache")
        .join(f.0.file_name().unwrap());
    fs::create_dir_all(&build_folder).unwrap();
    let binary = build_folder.join("cache_probe.exe");
    fs::write(
        &source,
        br##"fn main() { println!("{}", r#"{"schema":1,"name":"bridgeforge","status":"ok"}"#); }"##,
    )
    .unwrap();
    let mut compile = ProcessRequest::new("rustc", &f.0);
    compile.args = vec![
        source.into_os_string(),
        "-o".into(),
        binary.clone().into_os_string(),
    ];
    let compiled = SystemProcessRunner.run(&compile).unwrap();
    assert_eq!(
        compiled.code,
        0,
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let (asset, _, receipt) = artifact("long-probe");
    let payload = fs::read(binary).unwrap();
    fs::remove_dir_all(build_folder).unwrap();
    let mut receipt: Value = serde_json::from_slice(&receipt).unwrap();
    receipt["binary_sha256"] = json!(hash(&payload));
    let receipt = serde_json::to_vec(&receipt).unwrap();
    let directory = f.0.join(format!("long-{}", "a".repeat(180)));
    let mut cache = Cache::open_directory(&f.0, &f.0, directory, &SystemProcessRunner)
        .unwrap()
        .unwrap();
    cache
        .store(&asset, &payload, &receipt, &SystemProcessRunner)
        .unwrap();
    let key = Cache::key(&cache.identity(&asset)).unwrap();
    let original = cache.directory.join(key).join("bridgeforge.exe");
    assert!(original.to_string_lossy().len() > 260);
    assert_eq!(
        cache.load(&asset, &SystemProcessRunner).unwrap(),
        (payload.clone(), receipt)
    );
    assert_eq!(fs::read(original).unwrap(), payload);
}

#[test]
fn cache_hit_validates_bytes_self_test_and_exclusive_lease() {
    let f = Fixture::new();
    let (asset, payload, receipt) = artifact("one");
    let mut cache = Cache::open(&f.0, &f.0, &runner()).expect("cache enabled");
    cache.store(&asset, &payload, &receipt, &runner()).unwrap();
    assert!(
        Cache::open(&f.0, &f.0, &runner()).is_none(),
        "an active cache lease excludes another actor"
    );
    assert_eq!(
        cache.load(&asset, &runner()).unwrap(),
        (payload.clone(), receipt.clone())
    );
    drop(cache);
    let mut cache = Cache::open(&f.0, &f.0, &runner()).unwrap();
    assert_eq!(cache.load(&asset, &runner()).unwrap(), (payload, receipt));
}

#[test]
fn cache_rejects_corruption_self_test_failure_and_execution_drift() {
    for mode in ["binary", "receipt", "self-test", "drift", "meta"] {
        let f = Fixture::new();
        let (asset, payload, receipt) = artifact("one");
        let mut cache = Cache::open(&f.0, &f.0, &runner()).unwrap();
        cache.store(&asset, &payload, &receipt, &runner()).unwrap();
        let key = Cache::key(&cache.identity(&asset)).unwrap();
        let directory = cache.directory.join(key);
        match mode {
            "binary" => fs::write(directory.join("bridgeforge.exe"), b"corrupt").unwrap(),
            "receipt" => fs::write(directory.join("receipt.json"), b"{}").unwrap(),
            "meta" => fs::write(directory.join("entry.json"), b"{}").unwrap(),
            _ => {}
        }
        let run = Runner {
            fail: mode == "self-test",
            mutate: mode == "drift",
        };
        assert!(cache.load(&asset, &run).is_none(), "{mode}");
    }
}

#[test]
fn cache_identity_invalidates_each_declared_compilation_input() {
    let f = Fixture::new();
    let (asset, payload, receipt) = artifact("one");
    let mut cache = Cache::open(&f.0, &f.0, &runner()).unwrap();
    cache.store(&asset, &payload, &receipt, &runner()).unwrap();
    for key in [
        "source_tree_sha256",
        "lockfile_sha256",
        "build_recipe_sha256",
        "self_test_sha256",
    ] {
        let mut changed = asset.clone();
        changed[key] = json!(hash(b"different"));
        assert!(cache.load(&changed, &runner()).is_none(), "{key}");
    }
    let mut changed = asset.clone();
    changed["self_test"]["expected_json"]["version"] = json!("9.0.0");
    assert!(cache.load(&changed, &runner()).is_none());
}

#[test]
fn cache_prunes_to_two_versions_and_preserves_foreign_content() {
    let f = Fixture::new();
    let mut keys = Vec::new();
    for revision in ["one", "two", "three"] {
        let (asset, payload, receipt) = artifact(revision);
        let mut cache = Cache::open(&f.0, &f.0, &runner()).unwrap();
        keys.push(Cache::key(&cache.identity(&asset)).unwrap());
        cache.store(&asset, &payload, &receipt, &runner()).unwrap();
    }
    let directory = f.0.join(DIRECTORY);
    assert!(!directory.join(&keys[0]).exists());
    assert!(directory.join(&keys[1]).exists() && directory.join(&keys[2]).exists());
    let foreign = directory.join("user-note.txt");
    fs::write(&foreign, b"keep me").unwrap();
    let (asset, payload, receipt) = artifact("four");
    let mut cache = Cache::open(&f.0, &f.0, &runner()).unwrap();
    cache.store(&asset, &payload, &receipt, &runner()).unwrap();
    assert_eq!(fs::read(foreign).unwrap(), b"keep me");
}

#[test]
fn cache_capacity_does_not_evict_a_currently_leased_entry() {
    let f = Fixture::new();
    let (asset, payload, receipt) = artifact("one");
    let mut cache = Cache::open(&f.0, &f.0, &runner()).unwrap();
    cache.store(&asset, &payload, &receipt, &runner()).unwrap();
    assert!(cache.reserve(&cache.identity(&asset), MAX_BYTES).is_err());
    assert!(
        cache
            .reserve(&cache.identity(&asset), MAX_BYTES + 1)
            .is_err()
    );
    assert_eq!(cache.load(&asset, &runner()).unwrap(), (payload, receipt));
}

#[test]
fn cache_does_not_claim_a_preexisting_unowned_root() {
    let f = Fixture::new();
    let directory = f.0.join(DIRECTORY);
    fs::create_dir_all(&directory).unwrap();
    fs::write(directory.join("user-data"), b"original").unwrap();
    assert!(Cache::open(&f.0, &f.0, &runner()).is_none());
    assert_eq!(fs::read_dir(directory).unwrap().count(), 1);
}

#[test]
fn cache_uses_build_context_toolchain_not_project_pin() {
    struct ContextRunner {
        project: PathBuf,
        build: PathBuf,
        version: std::cell::Cell<u32>,
    }
    impl ProcessRunner for ContextRunner {
        fn run(&self, r: &ProcessRequest) -> std::io::Result<ProcessOutput> {
            if r.program == "cargo" || r.program == "rustc" {
                let version = if r.cwd == self.project {
                    88
                } else {
                    assert_eq!(r.cwd, self.build);
                    self.version.get()
                };
                return Ok(ProcessOutput {
                    code: 0,
                    stdout: format!("{} 1.{version}.0 (fixture)", r.program.to_string_lossy())
                        .into_bytes(),
                    stderr: Vec::new(),
                    timed_out: false,
                });
            }
            runner().run(r)
        }
    }
    let f = Fixture::new();
    let build = f.0.join("snapshot");
    fs::create_dir(&build).unwrap();
    let r = ContextRunner {
        project: f.0.clone(),
        build: build.clone(),
        version: std::cell::Cell::new(89),
    };
    let (asset, payload, receipt) = artifact("one");
    let mut cache = Cache::open(&f.0, &build, &r).unwrap();
    cache.store(&asset, &payload, &receipt, &r).unwrap();
    drop(cache);
    r.version.set(90);
    let mut cache = Cache::open(&f.0, &build, &r).unwrap();
    assert!(cache.load(&asset, &r).is_none());
}

#[test]
fn cache_rejects_cargo_compiler_and_environment_overrides() {
    let f = Fixture::new();
    fs::create_dir(f.0.join(".cargo")).unwrap();
    for content in [
        "[build]\nrustc = 'different-compiler'\n",
        "[ env ]\nRUSTC = 'different'\n",
        "[\"env\"]\nCC = 'different'\n",
    ] {
        fs::write(f.0.join(".cargo/config.toml"), content).unwrap();
        assert!(Cache::open(&f.0, &f.0, &runner()).is_none());
    }
}

#[test]
fn cache_recovers_interrupted_stage_retirement_and_recency_write() {
    for kind in ["stage", "retired", "touch"] {
        let f = Fixture::new();
        let (asset, payload, receipt) = artifact("one");
        let mut cache = Cache::open(&f.0, &f.0, &runner()).unwrap();
        cache.store(&asset, &payload, &receipt, &runner()).unwrap();
        let key = Cache::key(&cache.identity(&asset)).unwrap();
        let op = cache
            .operation(&key, binary_name(&cache.identity(&asset)).unwrap(), kind)
            .unwrap();
        let (_, directory) = cache.operation_paths(&op).unwrap();
        if kind == "stage" {
            fs::create_dir(&directory).unwrap();
            fs::write(directory.join("entry.json"), b"partial metadata").unwrap();
        } else if kind == "retired" {
            fs::rename(cache.directory.join(&key), &directory).unwrap();
            fs::remove_file(directory.join("bridgeforge.exe")).unwrap();
        } else {
            fs::write(
                cache
                    .directory
                    .join(&key)
                    .join(format!(".entry-meta-{}.tmp", op.nonce)),
                b"partial",
            )
            .unwrap();
        }
        drop(cache);
        let mut cache = Cache::open(&f.0, &f.0, &runner()).unwrap();
        assert!(!cache.operation_paths(&op).unwrap().0.exists());
        if kind == "touch" {
            assert!(cache.load(&asset, &runner()).is_some());
        }
        let (next, bytes, receipt) = artifact("next");
        cache.store(&next, &bytes, &receipt, &runner()).unwrap();
        assert!(cache.load(&next, &runner()).is_some());
    }
}

#[test]
fn cache_partial_journal_does_not_poison_other_publications() {
    let f = Fixture::new();
    let cache = Cache::open(&f.0, &f.0, &runner()).unwrap();
    let journal = cache.directory.join("operation-123.json");
    fs::write(&journal, b"{partial").unwrap();
    drop(cache);
    let mut cache = Cache::open(&f.0, &f.0, &runner()).unwrap();
    let (asset, payload, receipt) = artifact("one");
    cache.store(&asset, &payload, &receipt, &runner()).unwrap();
    assert_eq!(fs::read(journal).unwrap(), b"{partial");
    assert!(cache.load(&asset, &runner()).is_some());
}

#[test]
fn cache_recency_is_monotonic_even_with_future_old_timestamp() {
    let f = Fixture::new();
    let mut keys = Vec::new();
    for revision in ["one", "two"] {
        let (asset, payload, receipt) = artifact(revision);
        let mut cache = Cache::open(&f.0, &f.0, &runner()).unwrap();
        let key = Cache::key(&cache.identity(&asset)).unwrap();
        cache.store(&asset, &payload, &receipt, &runner()).unwrap();
        keys.push(key);
    }
    let cache = Cache::open(&f.0, &f.0, &runner()).unwrap();
    let (mut entry, _) = cache.owned(&keys[0]).unwrap();
    entry.used = now() + 10_000_000_000_000;
    crate::persistence::atomic_write_json(
        &cache.directory.join(&keys[0]).join("entry.json"),
        &entry,
    )
    .unwrap();
    drop(cache);
    let (asset, payload, receipt) = artifact("three");
    let mut cache = Cache::open(&f.0, &f.0, &runner()).unwrap();
    cache.store(&asset, &payload, &receipt, &runner()).unwrap();
    let newkey = Cache::key(&cache.identity(&asset)).unwrap();
    assert!(cache.owned(&newkey).unwrap().0.used > entry.used);
    assert!(!cache.directory.join(&keys[1]).exists());
}

#[test]
#[cfg(windows)]
fn cache_rejects_linked_root_without_writing_outside() {
    let f = Fixture::new();
    let real = f.0.join("external");
    fs::create_dir(&real).unwrap();
    let link = f.0.join(DIRECTORY);
    fs::create_dir_all(link.parent().unwrap()).unwrap();
    let mut request = ProcessRequest::new("cmd.exe", &f.0);
    request.args = vec![
        "/D".into(),
        "/C".into(),
        "mklink".into(),
        "/J".into(),
        link.to_string_lossy().replace('/', "\\").into(),
        real.to_string_lossy().replace('/', "\\").into(),
    ];
    assert_eq!(SystemProcessRunner.run(&request).unwrap().code, 0);
    assert!(Cache::open(&f.0, &f.0, &runner()).is_none());
    assert_eq!(fs::read_dir(real).unwrap().count(), 0);
}

#[test]
#[cfg(windows)]
fn cache_does_not_evict_an_open_executable() {
    use std::os::windows::fs::OpenOptionsExt;
    let f = Fixture::new();
    let (asset, payload, receipt) = artifact("one");
    let mut cache = Cache::open(&f.0, &f.0, &runner()).unwrap();
    cache.store(&asset, &payload, &receipt, &runner()).unwrap();
    drop(cache);
    let cache = Cache::open(&f.0, &f.0, &runner()).unwrap();
    let key = Cache::key(&cache.identity(&asset)).unwrap();
    let (entry, _) = cache.owned(&key).unwrap();
    let held = OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(cache.directory.join(&key).join("bridgeforge.exe"))
        .unwrap();
    assert!(cache.remove(&entry).is_err());
    assert!(cache.directory.join(&key).join("entry.json").exists());
    drop(held);
    cache.remove(&entry).unwrap();
    assert!(!cache.directory.join(&key).exists());
}

#[test]
fn cache_byte_capacity_evicts_an_owned_large_entry_without_touching_foreign_data() {
    let f = Fixture::new();
    let (first, payload, receipt) = artifact("one");
    let mut cache = Cache::open(&f.0, &f.0, &runner()).unwrap();
    cache.store(&first, &payload, &receipt, &runner()).unwrap();
    let firstkey = Cache::key(&cache.identity(&first)).unwrap();
    drop(cache);
    // Simulate a real on-disk footprint increase using a bounded sparse test file.
    let mut cache = Cache::open(&f.0, &f.0, &runner()).unwrap();
    OpenOptions::new()
        .write(true)
        .open(cache.directory.join(&firstkey).join("bridgeforge.exe"))
        .unwrap()
        .set_len(129 * 1024 * 1024)
        .unwrap();
    let second = artifact("two").0;
    cache
        .reserve(&cache.identity(&second), 128 * 1024 * 1024)
        .unwrap();
    assert!(!cache.directory.join(&firstkey).exists());
    let (asset, payload, receipt) = artifact("two");
    cache.store(&asset, &payload, &receipt, &runner()).unwrap();
    assert!(footprint(&cache.directory).unwrap() <= MAX_BYTES);
}

#[allow(dead_code)]
#[path = "../support/artifact_paths.rs"]
mod test_artifacts;
