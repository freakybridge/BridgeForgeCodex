use super::*;
use std::collections::BTreeMap;

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = test_artifacts::root(env!("CARGO_MANIFEST_DIR"), "cleanup")
            .join(format!("{:x}-{nonce:x}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        Self(root.canonicalize().unwrap())
    }
    fn put(&self, path: &str, bytes: &[u8]) {
        let path = self.0.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }
    fn tree(&self) -> BTreeMap<String, Vec<u8>> {
        walkdir::WalkDir::new(&self.0)
            .into_iter()
            .map(|e| {
                let e = e.unwrap();
                let path = e
                    .path()
                    .strip_prefix(&self.0)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned();
                (
                    path,
                    if e.file_type().is_file() {
                        fs::read(e.path()).unwrap()
                    } else {
                        Vec::new()
                    },
                )
            })
            .collect()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn cleanup_preview_is_zero_write_and_protects_exact_subtrees() {
    let f = Fixture::new();
    for path in [
        ".runtime/config/配置.txt",
        ".runtime/download/.partial",
        ".deps/tool.exe",
        "src/keep.rs",
        ".build/bridgeforge/bin/bridgeforge.exe",
        ".build/bridgeforge/bin/build-receipt-cli.json",
        ".build/unknown.bin",
        ".runtime/tests/unit/report.txt",
        ".runtime/config-old/x",
        ".build/bridgeforge/cargo/local/config/x",
    ] {
        f.put(path, b"preserve or remove by exact scope");
    }
    let before = f.tree();
    let plan = Plan::capture(&f.0).unwrap();
    assert_eq!(plan.preview().planned_files, 3);
    assert_eq!(plan.preview().status, "preview");
    assert_eq!(f.tree(), before);
    #[cfg(windows)]
    {
        let report = plan.execute();
        assert_eq!(report.status, "completed", "{:?}", report.failures);
        assert_eq!(report.deleted_files, 3);
        for path in [
            ".runtime/config/配置.txt",
            ".runtime/download/.partial",
            ".deps/tool.exe",
            "src/keep.rs",
            ".build/bridgeforge/bin/bridgeforge.exe",
            ".build/bridgeforge/bin/build-receipt-cli.json",
            ".build/unknown.bin",
        ] {
            assert_eq!(
                fs::read(f.0.join(path)).unwrap(),
                b"preserve or remove by exact scope"
            );
        }
        assert_eq!(Plan::capture(&f.0).unwrap().execute().deleted_files, 0);
    }
}

#[cfg(windows)]
#[test]
fn cleanup_active_lock_blocks_all_deletion_and_partial_failure_is_reported() {
    let f = Fixture::new();
    f.put(".runtime/a.txt", b"a");
    f.put(".build/bridgeforge/cargo/local/b.txt", b"b");
    let held = crate::file_lock::FileLock::acquire(&f.0.join(".runtime/work.lock")).unwrap();
    let before = Plan::capture(&f.0).unwrap().entries;
    let report = Plan::capture(&f.0).unwrap().execute();
    assert_eq!(report.status, "blocked");
    assert_eq!(report.deleted_files, 0);
    assert_eq!(Plan::capture(&f.0).unwrap().entries, before);
    drop(held);
    let report = Plan::capture(&f.0)
        .unwrap()
        .execute_observed(&mut || Err("injected interruption".into()));
    assert_eq!(report.status, "partial");
    assert_eq!(report.deleted_files, 1);
    assert!(report.remaining_entries.unwrap() > 0);
}

#[cfg(windows)]
#[test]
fn cleanup_recreated_path_is_preserved_and_counted_as_partial() {
    let f = Fixture::new();
    f.put(".runtime/only.txt", b"original");
    let report = Plan::capture(&f.0).unwrap().execute_observed(&mut || {
        fs::write(f.0.join(".runtime/only.txt"), b"new").map_err(|e| e.to_string())
    });
    assert_eq!(report.status, "partial");
    assert_eq!(report.deleted_files, 1);
    assert_eq!(report.remaining_entries, Some(1));
    assert_eq!(fs::read(f.0.join(".runtime/only.txt")).unwrap(), b"new");
}

#[cfg(windows)]
#[test]
#[ignore = "isolated child executor preflight"]
fn cleanup_self_child() {
    use sha2::{Digest, Sha256};
    let root = PathBuf::from(std::env::var_os("BFC_CLEANUP_FIXTURE").unwrap());
    let unit = std::env::current_exe()
        .unwrap()
        .canonicalize()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let mut members = BTreeMap::from([("".into(), None)]);
    for name in ["executor.exe", "receipt.json", "entry.json"] {
        members.insert(
            name.into(),
            Some(format!(
                "sha256:{:x}",
                Sha256::digest(fs::read(unit.join(name)).unwrap())
            )),
        );
    }
    let mut policy = BuildPolicy::default();
    policy.selections.push(Selection {
        path: unit,
        members,
    });
    let report = Plan::capture_with_build(&root, policy).unwrap().execute();
    println!("{}", serde_json::to_string(&report).unwrap());
    assert_eq!(report.status, "completed", "{:?}", report.failures);
    assert_eq!(report.deleted_files, 2);
    assert!(std::env::current_exe().unwrap().exists());
}

#[cfg(windows)]
#[test]
fn cleanup_project_executor_survives_while_runtime_and_caches_are_removed() {
    use crate::ProcessRunner;
    let f = Fixture::new();
    f.put(".runtime/config/keep.txt", b"keep");
    f.put(".runtime/tests/task/report.txt", b"report");
    f.put(".build/bridgeforge/cargo/local/old.obj", b"cache");
    let binary =
        f.0.join(".build/bridgeforge/cargo/local/current-unit/executor.exe");
    fs::create_dir_all(binary.parent().unwrap()).unwrap();
    fs::copy(std::env::current_exe().unwrap(), &binary).unwrap();
    let alias = f.0.join(".build/bridgeforge/cargo/local/current-alias.exe");
    fs::hard_link(&binary, &alias).unwrap();
    fs::write(binary.with_file_name("receipt.json"), b"current receipt").unwrap();
    fs::write(binary.with_file_name("entry.json"), b"current identity").unwrap();
    let mut request = crate::ProcessRequest::new(binary.as_os_str(), &f.0);
    request.args = [
        "--exact",
        "artifact_cleanup::tests::cleanup_self_child",
        "--ignored",
        "--nocapture",
    ]
    .map(Into::into)
    .to_vec();
    request
        .env
        .insert("BFC_CLEANUP_FIXTURE".into(), f.0.as_os_str().into());
    let result = crate::SystemProcessRunner.run(&request).unwrap();
    assert_eq!(
        result.code,
        0,
        "{} {}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(String::from_utf8_lossy(&result.stdout).contains("\"status\":\"completed\""));
    assert!(binary.exists());
    assert!(alias.exists());
    assert_eq!(
        fs::read(binary.with_file_name("receipt.json")).unwrap(),
        b"current receipt"
    );
    assert_eq!(
        fs::read(binary.with_file_name("entry.json")).unwrap(),
        b"current identity"
    );
    assert_eq!(
        fs::read(f.0.join(".runtime/config/keep.txt")).unwrap(),
        b"keep"
    );
}

#[cfg(windows)]
#[test]
fn cleanup_rejects_junction_before_any_deletion() {
    use std::os::windows::process::CommandExt;
    let f = Fixture::new();
    f.put(".runtime/remove.txt", b"remove");
    f.put("external/keep.txt", b"keep");
    fs::create_dir(f.0.join(".build")).unwrap();
    let link = f.0.join(".build/link");
    let output = std::process::Command::new("cmd")
        .creation_flags(0x08000000)
        .args(["/c", "mklink", "/J"])
        .arg(&link)
        .arg(f.0.join("external"))
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(Plan::capture(&f.0).is_err());
    assert!(f.0.join(".runtime/remove.txt").exists());
    fs::remove_dir(link).unwrap();
}

#[cfg(windows)]
#[test]
fn cleanup_cargo_hardlinks_are_removed_without_self_conflict_and_replacements_survive() {
    let f = Fixture::new();
    let a = ".build/bridgeforge/cargo/local/release/app.exe";
    let b = ".build/bridgeforge/cargo/local/release/deps/app.exe";
    f.put(a, b"old program");
    fs::create_dir_all(f.0.join(b).parent().unwrap()).unwrap();
    fs::hard_link(f.0.join(a), f.0.join(b)).unwrap();
    let report = Plan::capture(&f.0).unwrap().execute();
    assert_eq!(report.status, "completed", "{:?}", report.failures);
    assert_eq!(report.deleted_files, 2);
    assert!(!f.0.join(a).exists());
    assert!(!f.0.join(b).exists());

    f.put(a, b"old program");
    fs::create_dir_all(f.0.join(b).parent().unwrap()).unwrap();
    fs::hard_link(f.0.join(a), f.0.join(b)).unwrap();
    let mut replaced = false;
    let report = Plan::capture(&f.0).unwrap().execute_observed(&mut || {
        if !replaced {
            fs::rename(f.0.join(b), f.0.join("preserved-old.bin")).map_err(|e| e.to_string())?;
            fs::write(f.0.join(b), b"external replacement").map_err(|e| e.to_string())?;
            replaced = true;
        }
        Ok(())
    });
    assert_eq!(report.status, "partial", "{:?}", report.failures);
    assert_eq!(report.deleted_files, 1);
    assert_eq!(fs::read(f.0.join(b)).unwrap(), b"external replacement");
    assert_eq!(
        fs::read(f.0.join("preserved-old.bin")).unwrap(),
        b"old program"
    );
}

#[allow(dead_code)]
#[path = "../support/artifact_paths.rs"]
mod test_artifacts;
