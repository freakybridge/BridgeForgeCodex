use super::*;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs};
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = test_artifacts::root(env!("CARGO_MANIFEST_DIR"), "cleanup-eligibility")
            .join(format!("{}-{nonce}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let f = Self(root);
        let mut git = ProcessRequest::new("git", &f.0);
        git.args = vec!["init".into(), "-q".into()];
        assert_eq!(crate::SystemProcessRunner.run(&git).unwrap().code, 0);
        f.put(".gitignore", artifact_paths::IGNORE_RULES.as_bytes());
        f.put(".codex/.bridgeforge_codex_version", b"9.0.0\n");
        f.put("seed.txt", b"seed\n");
        let contract = json!({"schema_version":4,"release_version":"9.0.0","host":"codex","stamp":".codex/.bridgeforge_codex_version",
            "contract_target":".codex/managed-skeleton.json","baseline_model":"current-only","compatibility_baseline":"8.0.0",
            "artifact_layout":artifact_paths::CONTRACT,"generated_assets":[],"assets":[{"id":"fixture.seed","source":"templates/seed.txt","target":"seed.txt","strategy":"whole","current_sha256":format!("sha256:{:x}",Sha256::digest(b"seed\n"))}]});
        f.put(
            ".codex/managed-skeleton.json",
            &serde_json::to_vec(&contract).unwrap(),
        );
        f
    }
    fn put(&self, path: &str, bytes: &[u8]) {
        let path = self.0.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }
    fn run(&self, preview: bool) -> CommandOutcome {
        let mut args = vec![
            "cleanup".into(),
            "--root".into(),
            self.0.to_string_lossy().into_owned(),
        ];
        if preview {
            args.push("--dry-run".into());
        }
        command(&args, &crate::SystemProcessRunner)
    }
    fn tree(&self) -> BTreeMap<String, Vec<u8>> {
        walkdir::WalkDir::new(&self.0)
            .into_iter()
            .map(|e| {
                let e = e.unwrap();
                (
                    e.path()
                        .strip_prefix(&self.0)
                        .unwrap()
                        .to_string_lossy()
                        .into_owned(),
                    if e.file_type().is_file() {
                        fs::read(e.path()).unwrap()
                    } else {
                        vec![]
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
fn cleanup_eligibility_is_zero_write_and_rejects_legacy_unknown_and_subdirectory_roots() {
    let f = Fixture::new();
    f.put(".runtime/data.txt", b"data");
    let before = f.tree();
    let output = f.run(true);
    assert_eq!(output.code, 0, "{:?}", output.receipt);
    assert_eq!(f.tree(), before);
    let path = f.0.join(".codex/managed-skeleton.json");
    let mut contract: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    contract.as_object_mut().unwrap().remove("artifact_layout");
    fs::write(path, serde_json::to_vec(&contract).unwrap()).unwrap();
    let before = f.tree();
    assert_ne!(f.run(false).code, 0);
    assert_eq!(f.tree(), before);
    let args = vec![
        "cleanup".into(),
        "--root".into(),
        f.0.join(".runtime").to_string_lossy().into_owned(),
    ];
    assert_ne!(command(&args, &crate::SystemProcessRunner).code, 0);
    assert_eq!(f.tree(), before);
}

#[test]
fn cleanup_rejects_unfinished_recovery_and_wrong_artifact_layout() {
    let f = Fixture::new();
    f.put(".runtime/keep.txt", b"keep");
    for path in [
        ".runtime/bridgeforge-codex/develop-transaction.json",
        ".runtime/bridgeforge-codex/release-preparation/commit-guard.json",
    ] {
        f.put(path, b"{}\n");
        let before = f.tree();
        assert_ne!(f.run(false).code, 0);
        assert_eq!(f.tree(), before);
        fs::remove_file(f.0.join(path)).unwrap();
    }
    f.put("target/old.bin", b"old");
    let before = f.tree();
    assert_ne!(f.run(false).code, 0);
    assert_eq!(f.tree(), before);
}

#[cfg(windows)]
#[test]
fn cleanup_command_preserves_dependencies_and_returns_receipt_without_repopulating_roots() {
    let f = Fixture::new();
    f.put(".runtime/config/settings.txt", b"settings");
    f.put(".runtime/download/file.zip", b"download");
    f.put(".deps/tool.txt", b"tool");
    f.put(".runtime/tests/task/report.txt", b"report");
    f.put(".build/bridgeforge/cargo/local/result.bin", b"result");
    f.put(".build/bridgeforge/bin/current.exe", b"current");
    f.put(".build/bridgeforge/bin/receipt.json", b"receipt");
    f.put(
        ".codex/artifact-layout.json",
        &serde_json::to_vec(&json!({"schema":1,"outputs":[
            {"path":".build/bridgeforge/bin/current.exe","kind":"build","cleanup":"keep"},
            {"path":".build/bridgeforge/bin/receipt.json","kind":"build","cleanup":"keep"}
        ]}))
        .unwrap(),
    );
    let report = f.run(false);
    assert_eq!(report.code, 0, "{:?}", report.receipt);
    assert_eq!(report.receipt.unwrap()["status"], "completed");
    assert!(!f.0.join(".build/bridgeforge/cargo/local").exists());
    assert_eq!(
        fs::read(f.0.join(".build/bridgeforge/bin/current.exe")).unwrap(),
        b"current"
    );
    assert_eq!(
        fs::read(f.0.join(".build/bridgeforge/bin/receipt.json")).unwrap(),
        b"receipt"
    );
    assert_eq!(fs::read_dir(f.0.join(".runtime")).unwrap().count(), 2);
    assert_eq!(fs::read(f.0.join(".deps/tool.txt")).unwrap(), b"tool");
    assert_eq!(f.run(false).code, 0);
}

#[cfg(windows)]
#[test]
fn cleanup_declared_current_outputs_override_cache_and_history_with_policy_drift_guard() {
    let f = Fixture::new();
    let layout = json!({"schema":1,"outputs":[
        {"path":".build/app","kind":"build","directory":true,"cleanup":"cache"},
        {"path":".build/app/current.exe","kind":"build","cleanup":"keep"},
        {"path":".build/app/current.receipt.json","kind":"build","cleanup":"keep"},
        {"path":".build/app/unclassified.bin","kind":"build"},
        {"path":".build/app/resources","kind":"build","directory":true,"cleanup":"keep"},
        {"path":".build/releases/previous","kind":"build","directory":true,"cleanup":"history"}
    ]});
    f.put(
        ".codex/artifact-layout.json",
        &serde_json::to_vec(&layout).unwrap(),
    );
    f.put(".build/app/current.exe", b"current");
    f.put(".build/app/current.receipt.json", b"receipt");
    f.put(".build/app/cache.obj", b"cache");
    f.put(".build/app/unclassified.bin", b"not a current product");
    f.put(".build/app/resources/runtime.dll", b"current DLL");
    f.put(".build/app/resources/data.json", b"current resource");
    f.put(".build/releases/previous/old.exe", b"old");
    f.put(".build/unknown/data.bin", b"unknown");
    let plan = Plan::capture_with_build(&f.0, build_policy(&f.0).unwrap()).unwrap();
    f.put(".codex/artifact-layout.json", b"{\"schema\":1}");
    let blocked = plan.execute();
    assert_eq!(blocked.status, "blocked");
    assert_eq!(blocked.deleted_files, 0);
    f.put(
        ".codex/artifact-layout.json",
        &serde_json::to_vec(&layout).unwrap(),
    );
    let result = f.run(false);
    assert_eq!(result.code, 0, "{:?}", result.receipt);
    assert_eq!(
        fs::read(f.0.join(".build/app/current.exe")).unwrap(),
        b"current"
    );
    assert_eq!(
        fs::read(f.0.join(".build/app/current.receipt.json")).unwrap(),
        b"receipt"
    );
    assert!(!f.0.join(".build/app/cache.obj").exists());
    assert!(!f.0.join(".build/releases/previous").exists());
    assert!(!f.0.join(".build/unknown").exists());
    assert!(!f.0.join(".build/app/unclassified.bin").exists());
    assert_eq!(
        fs::read(f.0.join(".build/app/resources/runtime.dll")).unwrap(),
        b"current DLL"
    );
    assert_eq!(
        fs::read(f.0.join(".build/app/resources/data.json")).unwrap(),
        b"current resource"
    );
    assert_eq!(result.receipt.unwrap()["skipped_build_paths"], json!([]));
}

#[allow(dead_code)]
#[path = "../support/artifact_paths.rs"]
mod test_artifacts;

#[cfg(windows)]
#[test]
fn cleanup_generated_products_are_exact_and_contract_drift_blocks_deletion() {
    let f = Fixture::new();
    let path = f.0.join(".codex/managed-skeleton.json");
    let mut contract: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    contract["generated_assets"] = json!([{
        "binary_targets":{"windows-x86_64":".build/tools/current.exe"},
        "receipt_target":".build/tools/current.json"
    }]);
    let bytes = serde_json::to_vec(&contract).unwrap();
    fs::write(&path, &bytes).unwrap();
    f.put(".build/tools/current.exe", b"current");
    f.put(".build/tools/current.json", b"receipt");
    f.put(".build/tools/old.exe", b"old");
    let plan = Plan::capture_with_build(&f.0, build_policy(&f.0).unwrap()).unwrap();
    contract["generated_assets"] = json!([]);
    fs::write(&path, serde_json::to_vec(&contract).unwrap()).unwrap();
    let blocked = plan.execute();
    assert_eq!(blocked.status, "blocked");
    assert_eq!(blocked.deleted_files, 0);
    fs::write(&path, bytes).unwrap();
    let result = Plan::capture_with_build(&f.0, build_policy(&f.0).unwrap())
        .unwrap()
        .execute();
    assert_eq!(result.status, "completed", "{:?}", result.failures);
    assert_eq!(result.deleted_files, 1);
    assert_eq!(
        fs::read(f.0.join(".build/tools/current.exe")).unwrap(),
        b"current"
    );
    assert_eq!(
        fs::read(f.0.join(".build/tools/current.json")).unwrap(),
        b"receipt"
    );
}

#[test]
fn cleanup_preserves_registered_project_hook_program_and_receipt() {
    let f = Fixture::new();
    f.put(".codex/hooks.json", br#"{"hooks":{}}"#);
    f.put(
        ".codex/project-hooks.json",
        br#"{"schema_version":1,"hooks":[{"id":"demo","events":[{"event":"Stop"}]}]}"#,
    );
    let binary = format!(
        ".build/bridgeforge/bin/project_demo{}",
        std::env::consts::EXE_SUFFIX
    );
    let receipt = ".build/bridgeforge/bin/build-receipt-project-demo.json";
    f.put(&binary, b"private program");
    f.put(receipt, b"private receipt");
    f.put(".build/old-object", b"old");
    let result = Plan::capture_with_build(&f.0, build_policy(&f.0).unwrap())
        .unwrap()
        .execute();
    assert_eq!(result.status, "completed", "{:?}", result.failures);
    assert_eq!(fs::read(f.0.join(binary)).unwrap(), b"private program");
    assert_eq!(fs::read(f.0.join(receipt)).unwrap(), b"private receipt");
    assert!(!f.0.join(".build/old-object").exists());
}

#[test]
fn cleanup_output_type_mismatch_and_invalid_roles_delete_nothing() {
    let f = Fixture::new();
    f.put(".runtime/remove.txt", b"runtime");
    f.put(
        ".build/app/nested/file.txt",
        b"must survive file declaration",
    );
    for output in [
        json!({"path":".build/app","kind":"build","cleanup":"history"}),
        json!({"path":".build","kind":"build","directory":true,"cleanup":"cache"}),
        json!({"path":".runtime/data","kind":"runtime","directory":true,"cleanup":"history"}),
    ] {
        f.put(
            ".codex/artifact-layout.json",
            &serde_json::to_vec(&json!({"schema":1,"outputs":[output]})).unwrap(),
        );
        let before = f.tree();
        let result = f.run(false);
        assert_ne!(result.code, 0, "{:?}", result.receipt);
        assert_eq!(f.tree(), before);
    }
}

#[cfg(windows)]
#[test]
fn cleanup_old_image_selection_rejects_replacement_then_completes() {
    let f = Fixture::new();
    let bytes = b"owned old image";
    let name = format!(
        ".build/bridgeforge/git-sync-images/{:x}-1-2.exe",
        Sha256::digest(bytes)
    );
    f.put(&name, bytes);
    let plan = Plan::capture_with_build(&f.0, build_policy(&f.0).unwrap()).unwrap();
    f.put(&name, b"unknown replacement");
    assert_eq!(plan.execute().status, "blocked");
    f.put(&name, bytes);
    let result = f.run(false);
    assert_eq!(result.code, 0, "{:?}", result.receipt);
    assert_eq!(result.receipt.unwrap()["deleted_files"], 1);
    assert!(!f.0.join(name).exists());
}

#[cfg(windows)]
#[test]
#[ignore = "requires a freshly prepared factory binary in BFC_CLEANUP_TOOL"]
fn cleanup_actual_project_factory_keeps_program_and_receipt_usable() {
    let f = Fixture::new();
    let source = PathBuf::from(std::env::var_os("BFC_CLEANUP_TOOL").unwrap());
    let binary = f.0.join(".build/bridgeforge/bin/bridgeforge-factory.exe");
    fs::create_dir_all(binary.parent().unwrap()).unwrap();
    fs::copy(&source, &binary).unwrap();
    let receipt = fs::read(source.with_file_name("build-receipt-factory.json")).unwrap();
    f.put(".codex/artifact-layout.json", &serde_json::to_vec(&json!({"schema":1,"outputs":[
        {"path":".build/bridgeforge/bin/build-receipt-factory.json","kind":"build","cleanup":"keep"}
    ]})).unwrap());
    f.put(
        ".build/bridgeforge/bin/build-receipt-factory.json",
        &receipt,
    );
    f.put(".runtime/config/settings.json", b"{}");
    f.put(".runtime/download/archive.zip", b"download");
    f.put(".runtime/tests/previous/result.txt", b"old runtime result");
    f.put(".build/bridgeforge/cargo/local/old.obj", b"compile cache");
    let before_binary = fs::read(&binary).unwrap();
    let run = |args: Vec<std::ffi::OsString>| {
        let mut request = ProcessRequest::new(binary.as_os_str(), &f.0);
        request.args = args;
        crate::SystemProcessRunner.run(&request).unwrap()
    };
    let result = run(vec![
        "cleanup".into(),
        "--root".into(),
        f.0.as_os_str().into(),
    ]);
    assert_eq!(
        result.code,
        0,
        "{} {}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    let report: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(report["status"], "completed");
    assert_eq!(report["deleted_files"], 2);
    assert_eq!(fs::read(&binary).unwrap(), before_binary);
    assert_eq!(
        fs::read(binary.with_file_name("build-receipt-factory.json")).unwrap(),
        receipt
    );
    assert_eq!(
        fs::read(f.0.join(".runtime/config/settings.json")).unwrap(),
        b"{}"
    );
    assert_eq!(
        fs::read(f.0.join(".runtime/download/archive.zip")).unwrap(),
        b"download"
    );
    let self_test = run(vec!["self-test".into(), "--json".into()]);
    assert_eq!(
        self_test.code,
        0,
        "{}",
        String::from_utf8_lossy(&self_test.stderr)
    );
    let self_test: Value = serde_json::from_slice(&self_test.stdout).unwrap();
    assert_eq!(self_test["status"], "ok");
    assert_eq!(self_test["name"], "bridgeforge-factory");
}
