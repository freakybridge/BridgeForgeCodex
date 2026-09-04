use bridgeforge_core::memory::remote::{
    apply_conflict_choices, reconcile, resolve_conflict_with_choices,
};
use bridgeforge_core::memory::worker::mark_pending;
use bridgeforge_core::project_sync::{
    SyncMode, apply_plan, attach_generated_assets, build_plan, build_plan_with_inputs,
};
use bridgeforge_core::{ProcessOutput, ProcessRequest, ProcessRunner, SystemProcessRunner};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const MEMORY_REMOTE: &str = "https://github.com/owner/bridgeforge-codex-memories.git";

// Only the factory tests route an approved GitHub identity to an isolated local
// bare repository. Production code has no local-remote or privacy bypass.
struct LocalMemoryRunner(PathBuf, Option<&'static str>);

impl ProcessRunner for LocalMemoryRunner {
    fn run(&self, request: &ProcessRequest) -> std::io::Result<ProcessOutput> {
        assert!(request.cwd.starts_with(self.0.parent().unwrap()));
        if request.program == "gh" {
            assert_eq!(
                request.args,
                [
                    "repo",
                    "view",
                    "https://github.com/owner/bridgeforge-codex-memories",
                    "--json",
                    "visibility",
                    "--jq",
                    ".visibility"
                ]
            );
            return Ok(ProcessOutput {
                code: 0,
                stdout: b"PRIVATE\n".to_vec(),
                stderr: Vec::new(),
                timed_out: false,
            });
        }
        assert_eq!(request.program, "git");
        if request.args == ["remote", "get-url", "--push", "--all", "origin"] {
            return Ok(ProcessOutput {
                code: 0,
                stdout: self.1.unwrap_or(MEMORY_REMOTE).as_bytes().to_vec(),
                stderr: Vec::new(),
                timed_out: false,
            });
        }
        if request.args == ["ls-remote", "--get-url", MEMORY_REMOTE]
            || request.args == ["remote", "get-url", "--all", "origin"]
        {
            return Ok(ProcessOutput {
                code: 0,
                stdout: MEMORY_REMOTE.as_bytes().to_vec(),
                stderr: Vec::new(),
                timed_out: false,
            });
        }
        let mut local = request.clone();
        for argument in &mut local.args {
            if argument == MEMORY_REMOTE {
                *argument = self.0.clone().into_os_string();
            }
            assert!(
                !argument.to_string_lossy().contains("https://"),
                "no network in fixture"
            );
        }
        SystemProcessRunner.run(&local)
    }
}

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new(label: &str) -> Self {
        let token = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "bridgeforge-rust-{label}-{}-{token}",
            std::process::id()
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn write(path: &Path, payload: &[u8]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, payload).unwrap();
}

fn git_at(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn synced_repo(parent: &Path, name: &str) -> PathBuf {
    let remote = parent.join(format!("{name}.git"));
    let repo = parent.join(name);
    git_at(
        parent,
        &["init", "--bare", remote.to_string_lossy().as_ref()],
    );
    git_at(
        parent,
        &[
            "clone",
            remote.to_string_lossy().as_ref(),
            repo.to_string_lossy().as_ref(),
        ],
    );
    git_at(&repo, &["config", "user.email", "batch@example.invalid"]);
    git_at(&repo, &["config", "user.name", "Batch Test"]);
    git_at(&repo, &["checkout", "-b", "main"]);
    repo
}

fn mark_official_factory(root: &Path) {
    let output = Command::new("git")
        .current_dir(root)
        .args(["remote", "get-url", "origin"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let local_push = String::from_utf8(output.stdout).unwrap().trim().to_string();
    git_at(
        root,
        &["remote", "set-url", "--push", "origin", &local_push],
    );
    git_at(
        root,
        &[
            "remote",
            "set-url",
            "origin",
            "https://github.com/freakybridge/BridgeForgeCodex.git",
        ],
    );
}

fn commit_and_push(root: &Path, message: &str) {
    git_at(root, &["add", "."]);
    git_at(root, &["commit", "-m", message]);
    git_at(root, &["push", "-u", "origin", "HEAD"]);
}

#[test]
fn project_sync_plan_apply_and_fingerprint_gate_are_transactional() {
    let temp = TestDirectory::new("project-sync");
    let factory = temp.0.join("factory");
    let project = temp.0.join("project");
    fs::create_dir_all(factory.join("templates")).unwrap();
    fs::create_dir_all(&project).unwrap();
    write(&factory.join("VERSION"), b"1.2.3\n");
    write(&factory.join("templates/example.txt"), b"managed\n");
    let contract = json!({
        "schema_version": 4,
        "release_version": "1.2.3",
        "host": "codex",
        "stamp": ".codex/.bridgeforge_codex_version",
        "contract_target": ".codex/managed-skeleton.json",
        "baseline_model": "current-only", "compatibility_baseline": "1.2.3",
        "assets": [{
            "id": "example",
            "source": "templates/example.txt",
            "target": ".codex/example.txt",
            "strategy": "whole",
            "current_sha256": format!("sha256:{:x}", Sha256::digest(b"managed\n"))
        }],
        "generated_assets": []
    });
    write(
        &factory.join("templates/managed-skeleton.json"),
        format!("{}\n", serde_json::to_string_pretty(&contract).unwrap()).as_bytes(),
    );

    let plan = build_plan(&project, &factory, SyncMode::Init).unwrap();
    assert!(!plan.confirmation_required);
    let wrong = apply_plan(plan.clone(), "sha256:wrong", false).unwrap_err();
    assert!(wrong.contains("fingerprint"));
    assert!(!project.join(".codex/example.txt").exists());

    write(&project.join(".codex/example.txt"), b"external drift\n");
    let drift = apply_plan(plan.clone(), &plan.aggregate_fingerprint, false).unwrap_err();
    assert!(drift.contains("drifted"));
    fs::remove_file(project.join(".codex/example.txt")).unwrap();

    let fingerprint = plan.aggregate_fingerprint.clone();
    let receipt = apply_plan(plan, &fingerprint, false).unwrap();
    assert_eq!(receipt.execution_status, "succeeded");
    assert_eq!(
        fs::read(project.join(".codex/example.txt")).unwrap(),
        b"managed\n"
    );
    assert_eq!(
        fs::read_to_string(project.join(".codex/.bridgeforge_codex_version"))
            .unwrap()
            .trim(),
        "1.2.3"
    );
}

#[test]
fn project_sync_real_init_builds_and_applies_generated_assets() {
    let temp = TestDirectory::new("project-sync-generated-init");
    let project = temp.0.join("project");
    fs::create_dir_all(&project).unwrap();
    let factory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf();
    let contract: serde_json::Value =
        serde_json::from_slice(&fs::read(factory.join("templates/managed-skeleton.json")).unwrap())
            .unwrap();
    // A registered project-owned Rust program shares the verified locked workspace,
    let mut initial = build_plan(&project, &factory, SyncMode::Init).unwrap();
    attach_generated_assets(
        &mut initial,
        &factory,
        &project,
        &contract,
        &SystemProcessRunner,
    )
    .unwrap();
    let initial_fingerprint = initial.aggregate_fingerprint.clone();
    apply_plan(initial, &initial_fingerprint, false).unwrap();
    bridgeforge_core::baseline::verify(&project, None, true).unwrap();
    let initial_stamp = fs::read(project.join(".codex/.bridgeforge_codex_version")).unwrap();
    // Project hook installation is a subsequent compatible update.
    // but remains outside managed source ownership. Its regular branch reads stdin.
    write(&project.join(".codex/hooks/project_demo/entrypoint.rs"),b"pub fn run(args:Vec<String>)->i32 { use std::io::Read; let mut input=String::new(); std::io::stdin().read_to_string(&mut input).unwrap(); println!(\"{}:{}\",args.join(\",\"),input); 0 }");
    let hooks_path = project.join(".codex/hooks.json");
    let mut hooks_config: serde_json::Value =
        serde_json::from_slice(&fs::read(&hooks_path).unwrap()).unwrap();
    hooks_config["bridgeforgeProjectHooks"] = json!({"schema_version":1,"hooks":[{"id":"demo","events":[{"event":"SessionStart","args":["check"]}]}]});
    write(&hooks_path, &serde_json::to_vec(&hooks_config).unwrap());
    let project_binary = project.join(format!(
        ".codex/bin/project_demo{}",
        std::env::consts::EXE_SUFFIX
    ));
    write(&project_binary, b"existing project program");
    let mut plan = build_plan(&project, &factory, SyncMode::Update).unwrap();
    assert!(
        plan.risk
            .iter()
            .any(|action| action.id.starts_with("project-hook:generated:"))
    );
    assert!(apply_plan(plan.clone(), &plan.aggregate_fingerprint, false).is_err());
    assert_eq!(
        fs::read(&project_binary).unwrap(),
        b"existing project program"
    );
    let confirmed_source_fingerprint = plan.aggregate_fingerprint.clone();
    let receipts = attach_generated_assets(
        &mut plan,
        &factory,
        &project,
        &contract,
        &SystemProcessRunner,
    )
    .unwrap();
    assert!(!receipts.is_empty());
    assert_eq!(plan.aggregate_fingerprint, confirmed_source_fingerprint);
    let input_path = project.join(".codex/hooks/project_demo/entrypoint.rs");
    let original_input = fs::read(&input_path).unwrap();
    fs::write(&input_path, b"source drift after build").unwrap();
    assert!(
        apply_plan(plan.clone(), &plan.aggregate_fingerprint, true)
            .unwrap_err()
            .contains("input changed after plan")
    );
    assert_eq!(
        fs::read(&project_binary).unwrap(),
        b"existing project program"
    );
    assert_eq!(
        fs::read(project.join(".codex/.bridgeforge_codex_version")).unwrap(),
        initial_stamp
    );
    fs::write(input_path, original_input).unwrap();
    let registry_path = project.join(".codex/project-hooks.json");
    assert!(
        !registry_path.exists(),
        "plan/build must not migrate registration early"
    );
    fs::write(&registry_path, b"registry appeared after plan").unwrap();
    assert!(
        apply_plan(plan.clone(), &plan.aggregate_fingerprint, true)
            .unwrap_err()
            .contains("input changed after plan")
    );
    fs::remove_file(&registry_path).unwrap();
    let expected = plan
        .safe
        .iter()
        .filter(|item| item.id.starts_with("generated:"))
        .map(|action| (action.target.clone(), action.after_sha256.clone()))
        .collect::<Vec<_>>();
    let fingerprint = plan.aggregate_fingerprint.clone();
    apply_plan(plan, &fingerprint, true).unwrap();
    let native: serde_json::Value =
        serde_json::from_slice(&fs::read(&hooks_path).unwrap()).unwrap();
    assert!(
        native
            .as_object()
            .unwrap()
            .keys()
            .all(|key| matches!(key.as_str(), "description" | "hooks"))
    );
    let registry = fs::read(&registry_path).unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&registry).unwrap(),
        hooks_config["bridgeforgeProjectHooks"]
    );
    // Index verification cannot silently use working-tree registry contents.
    for args in [vec!["init"], vec!["add", "--force", "."]] {
        let mut request = ProcessRequest::new("git", &project);
        request.args = args.into_iter().map(Into::into).collect();
        let output = SystemProcessRunner.run(&request).unwrap();
        assert_eq!(
            output.code,
            0,
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    fs::write(&registry_path, b"invalid working-tree registry").unwrap();
    bridgeforge_core::baseline::verify_index(&project, &SystemProcessRunner).unwrap();
    fs::write(&registry_path, &registry).unwrap();
    let mut unstage = ProcessRequest::new("git", &project);
    unstage.args = [
        "rm",
        "--cached",
        "--force",
        "--",
        ".codex/project-hooks.json",
    ]
    .into_iter()
    .map(Into::into)
    .collect();
    assert_eq!(SystemProcessRunner.run(&unstage).unwrap().code, 0);
    assert!(
        bridgeforge_core::baseline::verify_index(&project, &SystemProcessRunner)
            .unwrap_err()
            .contains("no registry")
    );
    for (target, hash) in expected {
        assert_eq!(
            hash,
            format!(
                "sha256:{:x}",
                Sha256::digest(fs::read(project.join(target)).unwrap())
            )
        );
    }
    bridgeforge_core::baseline::verify(&project, None, true).unwrap();
    let again = build_plan(&project, &factory, SyncMode::Update).unwrap();
    assert_eq!(again.status, "current");
    let mut project_hook = ProcessRequest::new(
        project
            .join(format!(
                ".codex/bin/project_demo{}",
                std::env::consts::EXE_SUFFIX
            ))
            .into_os_string(),
        &project,
    );
    project_hook.args = vec!["check".into()];
    project_hook.stdin = b"stdin-payload".to_vec();
    let result = SystemProcessRunner.run(&project_hook).unwrap();
    assert_eq!(result.code, 0);
    assert!(String::from_utf8_lossy(&result.stdout).contains("check:stdin-payload"));
    let source_path = project.join(".codex/hooks/project_demo/entrypoint.rs");
    let source = fs::read(&source_path).unwrap();
    fs::write(&source_path, b"changed").unwrap();
    assert!(
        bridgeforge_core::baseline::verify(&project, None, true)
            .unwrap_err()
            .contains("project Rust hook")
    );
    fs::write(source_path, source).unwrap();
    assert!(!project.join("scripts/tests").exists());
    let mut request = ProcessRequest::new("cargo", &project);
    request.args = [
        "test",
        "--locked",
        "--all-features",
        "--workspace",
        "--manifest-path",
        ".codex/hooks/Cargo.toml",
    ]
    .into_iter()
    .map(Into::into)
    .collect();
    request.timeout = std::time::Duration::from_secs(180);
    request.env_remove = vec!["RUSTFLAGS".into(), "CARGO_ENCODED_RUSTFLAGS".into()];
    let result = SystemProcessRunner.run(&request).unwrap();
    assert!(!result.timed_out);
    assert_eq!(
        result.code,
        0,
        "ordinary downstream Cargo test must not require factory tests: {}",
        String::from_utf8_lossy(&result.stderr)
    );
}

/// Explicitly opt in to reading ONLY the two named files of a real downstream.
/// No vault mappings/data are copied and no project business mode is executed.
#[test]
#[ignore = "requires an explicitly authorized BRIDGEFORGE_ASSIST_FIXTURE_SOURCE"]
fn assist_registry_migration_in_isolated_fixture() {
    let source = PathBuf::from(
        std::env::var_os("BRIDGEFORGE_ASSIST_FIXTURE_SOURCE").expect("explicit source required"),
    );
    let native_path = source.join(".codex/hooks.json");
    let entry_path = source.join(".codex/hooks/project_vault/entrypoint.rs");
    let original_native = fs::read(&native_path).unwrap();
    let original_entry = fs::read(&entry_path).unwrap();
    let original: serde_json::Value = serde_json::from_slice(&original_native).unwrap();
    assert_eq!(
        original["bridgeforgeProjectHooks"]["hooks"][0]["id"],
        "vault"
    );
    let temp = TestDirectory::new("assist-registry-compatibility");
    let project = temp.0.join("project");
    fs::create_dir_all(&project).unwrap();
    let factory = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap();
    let contract: serde_json::Value =
        serde_json::from_slice(&fs::read(factory.join("templates/managed-skeleton.json")).unwrap())
            .unwrap();
    let mut initial = build_plan(&project, factory, SyncMode::Init).unwrap();
    attach_generated_assets(
        &mut initial,
        factory,
        &project,
        &contract,
        &SystemProcessRunner,
    )
    .unwrap();
    let fingerprint = initial.aggregate_fingerprint.clone();
    apply_plan(initial, &fingerprint, false).unwrap();
    write(&project.join(".codex/hooks.json"), &original_native);
    write(
        &project.join(".codex/hooks/project_vault/entrypoint.rs"),
        &original_entry,
    );
    let mut plan = build_plan(&project, factory, SyncMode::Update).unwrap();
    assert!(plan.gaps.is_empty(), "{:?}", plan.gaps);
    assert!(
        plan.risk
            .iter()
            .any(|action| action.target == ".codex/project-hooks.json")
    );
    let fingerprint = plan.aggregate_fingerprint.clone();
    attach_generated_assets(
        &mut plan,
        factory,
        &project,
        &contract,
        &SystemProcessRunner,
    )
    .unwrap();
    assert_eq!(plan.aggregate_fingerprint, fingerprint);
    apply_plan(plan, &fingerprint, true).unwrap();
    let migrated: serde_json::Value =
        serde_json::from_slice(&fs::read(project.join(".codex/hooks.json")).unwrap()).unwrap();
    assert!(
        migrated
            .as_object()
            .unwrap()
            .keys()
            .all(|key| matches!(key.as_str(), "description" | "hooks"))
    );
    assert_eq!(
        migrated["hooks"], original["hooks"],
        "Assist commands/events must stay identical"
    );
    let registry: serde_json::Value =
        serde_json::from_slice(&fs::read(project.join(".codex/project-hooks.json")).unwrap())
            .unwrap();
    assert_eq!(registry, original["bridgeforgeProjectHooks"]);
    assert_eq!(
        fs::read(project.join(".codex/hooks/project_vault/entrypoint.rs")).unwrap(),
        original_entry
    );
    bridgeforge_core::baseline::verify(&project, None, true).unwrap();
    assert_eq!(
        build_plan(&project, factory, SyncMode::Update)
            .unwrap()
            .status,
        "current"
    );
    assert!(!project.join("vault").exists());
    assert!(!project.join("vault-mirror").exists());
    assert!(!project.join("vault_node_map").exists());
    assert_eq!(
        fs::read(&native_path).unwrap(),
        original_native,
        "real downstream config is read-only"
    );
    assert_eq!(
        fs::read(&entry_path).unwrap(),
        original_entry,
        "real business source is read-only"
    );
    println!(
        "Assist fixture: native commands identical; registry moved; source sha256:{:x}; locked build/baseline passed; second plan current; no vault business execution",
        Sha256::digest(&original_entry)
    );
}

#[test]
fn project_sync_retired_assets_require_exact_preservation_decisions() {
    let temp = TestDirectory::new("project-sync-preservation");
    let factory = temp.0.join("factory");
    fs::create_dir_all(factory.join("templates")).unwrap();
    write(&factory.join("VERSION"), b"1.2.3\n");
    write(&factory.join("templates/example.txt"), b"managed\n");
    let current_contract = json!({
        "schema_version": 4,
        "release_version": "1.2.3",
        "host": "codex",
        "stamp": ".codex/.bridgeforge_codex_version",
        "contract_target": ".codex/managed-skeleton.json",
        "baseline_model": "current-only", "compatibility_baseline": "1.2.3",
        "assets": [{
            "id": "example",
            "source": "templates/example.txt",
            "target": ".codex/example.txt",
            "strategy": "whole",
            "current_sha256": format!("sha256:{:x}", Sha256::digest(b"managed\n"))
        }],
        "generated_assets": []
    });
    write(
        &factory.join("templates/managed-skeleton.json"),
        format!(
            "{}\n",
            serde_json::to_string_pretty(&current_contract).unwrap()
        )
        .as_bytes(),
    );
    let original = b"original\n";
    let original_sha = format!("sha256:{:x}", Sha256::digest(original));
    let old_contract = json!({
        "schema_version": 4,
        "release_version": "1.2.2",
        "host": "codex",
        "stamp": ".codex/.bridgeforge_codex_version",
        "contract_target": ".codex/managed-skeleton.json",
        "baseline_model": "current-only", "compatibility_baseline": "1.2.3",
        "assets": [{
            "id": "old-python-hook",
            "source": "templates/old.py",
            "target": ".codex/old.py",
            "strategy": "whole",
            "current_sha256": original_sha
        }],
        "generated_assets": []
    });
    let prepare = |project: &Path, content: &[u8]| {
        fs::create_dir_all(project).unwrap();
        write(&project.join(".codex/old.py"), content);
        write(
            &project.join(".codex/.bridgeforge_codex_version"),
            b"1.2.2\n",
        );
        write(
            &project.join(".codex/managed-skeleton.json"),
            format!("{}\n", serde_json::to_string_pretty(&old_contract).unwrap()).as_bytes(),
        );
    };

    let exact = temp.0.join("exact");
    prepare(&exact, original);
    let exact_plan = build_plan(&exact, &factory, SyncMode::Update).unwrap();
    assert!(!exact_plan.gaps.is_empty());
    assert!(
        !exact_plan
            .safe
            .iter()
            .any(|item| item.operation == "delete")
    );
    assert!(exact.join(".codex/old.py").exists());

    let preserve = temp.0.join("preserve");
    prepare(&preserve, b"project custom\n");
    let undecided = build_plan(&preserve, &factory, SyncMode::Update).unwrap();
    assert!(
        undecided
            .gaps
            .iter()
            .any(|gap| gap.contains("P:project-file:.codex/old.py"))
    );
    let preserve_decision = json!({"preserve": ["P:project-file:.codex/old.py"], "delete": []});
    let preserve_plan = build_plan_with_inputs(
        &preserve,
        &factory,
        SyncMode::Update,
        None,
        Some(&preserve_decision),
    )
    .unwrap();
    assert!(preserve_plan.gaps.is_empty());
    let fingerprint = preserve_plan.aggregate_fingerprint.clone();
    apply_plan(preserve_plan, &fingerprint, false).unwrap();
    assert_eq!(
        fs::read(preserve.join(".codex/old.py")).unwrap(),
        b"project custom\n"
    );

    let delete = temp.0.join("delete");
    prepare(&delete, b"project custom\n");
    let delete_decision = json!({"preserve": [], "delete": ["P:project-file:.codex/old.py"]});
    let delete_plan = build_plan_with_inputs(
        &delete,
        &factory,
        SyncMode::Update,
        None,
        Some(&delete_decision),
    )
    .unwrap();
    assert!(delete_plan.confirmation_required);
    assert!(
        delete_plan
            .risk
            .iter()
            .any(|item| item.id == "rebuild.remove:P:project-file:.codex/old.py")
    );
    let fingerprint = delete_plan.aggregate_fingerprint.clone();
    apply_plan(delete_plan, &fingerprint, true).unwrap();
    assert!(!delete.join(".codex/old.py").exists());
}

#[test]
fn destructive_rebuild_requires_unknown_file_decisions_and_retires_old_project_maps() {
    let temp = TestDirectory::new("project-sync-destructive");
    let factory = temp.0.join("factory");
    let project = temp.0.join("project");
    fs::create_dir_all(factory.join("templates")).unwrap();
    fs::create_dir_all(&project).unwrap();
    write(&factory.join("VERSION"), b"1.2.3\n");
    write(&factory.join("templates/example.txt"), b"managed\n");
    let contract = json!({
        "schema_version": 4,
        "release_version": "1.2.3",
        "host": "codex",
        "stamp": ".codex/.bridgeforge_codex_version",
        "contract_target": ".codex/managed-skeleton.json",
        "baseline_model": "current-only", "compatibility_baseline": "1.2.3",
        "assets": [{
            "id": "example",
            "source": "templates/example.txt",
            "target": ".codex/example.txt",
            "strategy": "whole",
            "current_sha256": format!("sha256:{:x}", Sha256::digest(b"managed\n"))
        }],
        "generated_assets": []
    });
    write(
        &factory.join("templates/managed-skeleton.json"),
        format!("{}\n", serde_json::to_string_pretty(&contract).unwrap()).as_bytes(),
    );
    write(&project.join(".codex/find-doc.map.md"), b"project map\n");
    write(&project.join(".codex/sync-docs.map.md"), b"project map\n");
    write(
        &project.join(".codex/skills/custom/SKILL.md"),
        b"---\nname: custom\ndescription: project skill\n---\n",
    );
    write(
        &project.join(".codex/rules/custom.rules"),
        b"prefix_rule(pattern=[\"custom\"], decision=\"allow\")\n",
    );
    write(&project.join(".codex/unknown.bin"), b"unknown\n");

    let blocked = build_plan(&project, &factory, SyncMode::Adopt).unwrap();
    assert!(blocked.gaps.iter().any(|item| item.contains("unknown.bin")));
    for target in [".codex/find-doc.map.md", ".codex/sync-docs.map.md"] {
        assert!(blocked.safe.iter().any(|action| {
            action.target == target
                && action.operation == "delete"
                && action.id.starts_with("retired:project-map:")
        }));
    }
    assert!(
        blocked.preservation_manifest["entries"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["id"] == "R:skills" && item["disposition"] == "required-preserve")
    );
    fs::remove_file(project.join(".codex/unknown.bin")).unwrap();

    let undecided = build_plan(&project, &factory, SyncMode::Adopt).unwrap();
    assert!(undecided.blockers.is_empty(), "{:?}", undecided.blockers);
    assert!(
        undecided
            .gaps
            .iter()
            .any(|item| item.contains("P:rule:.codex/rules/custom.rules"))
    );
    let decisions = json!({
        "preserve": ["P:rule:.codex/rules/custom.rules"],
        "delete": []
    });
    let plan = build_plan_with_inputs(&project, &factory, SyncMode::Adopt, None, Some(&decisions))
        .unwrap();
    assert!(plan.gaps.is_empty(), "{:?}", plan.gaps);
    let combined =
        bridgeforge_core::project_sync::outcome_plan_with_format(Ok(plan.clone()), "combined");
    assert!(combined.receipt.as_ref().unwrap()["machine"].is_object());
    assert_eq!(
        combined.receipt.as_ref().unwrap()["human"]["conclusion"],
        "可直接执行"
    );
    let fingerprint = plan.aggregate_fingerprint.clone();
    apply_plan(plan, &fingerprint, false).unwrap();
    assert!(!project.join(".codex/find-doc.map.md").exists());
    assert!(!project.join(".codex/sync-docs.map.md").exists());
    assert!(project.join(".codex/skills/custom/SKILL.md").is_file());
    assert!(project.join(".codex/rules/custom.rules").is_file());
}

#[test]
fn batch_confirmation_detects_target_identity_drift_before_write() {
    let temp = TestDirectory::new("batch-identity");
    let factory = synced_repo(&temp.0, "factory");
    // Match the official factory: operational state is not source content.
    write(&factory.join(".gitignore"), b".runtime/\n");
    let target = synced_repo(&temp.0, "target");
    let payload = b"managed\n";
    let payload_sha = format!("sha256:{:x}", Sha256::digest(payload));
    let contract = json!({
        "schema_version": 4,
        "release_version": "1.2.3",
        "host": "codex",
        "stamp": ".codex/.bridgeforge_codex_version",
        "contract_target": ".codex/managed-skeleton.json",
        "baseline_model": "current-only", "compatibility_baseline": "1.2.3",
        "assets": [{
            "id": "example",
            "source": "templates/example.txt",
            "target": ".codex/example.txt",
            "strategy": "whole",
            "current_sha256": payload_sha
        }],
        "generated_assets": []
    });
    write(&factory.join("VERSION"), b"1.2.3\n");
    write(&factory.join("templates/example.txt"), payload);
    write(&factory.join(".codex/example.txt"), payload);
    for path in [
        factory.join("templates/managed-skeleton.json"),
        factory.join(".codex/managed-skeleton.json"),
    ] {
        write(
            &path,
            format!("{}\n", serde_json::to_string_pretty(&contract).unwrap()).as_bytes(),
        );
    }
    write(
        &factory.join("templates/hooks/crates/bridgeforge-core/src/batch.rs"),
        b"batch controller witness\n",
    );
    commit_and_push(&factory, "factory");
    mark_official_factory(&factory);
    write(&target.join("README.md"), b"target\n");
    commit_and_push(&target, "target");

    let roots = vec![target.clone()];
    let plan = bridgeforge_core::batch::plan(&factory, &roots).unwrap();
    assert_eq!(plan.status, "planned");
    assert!(!plan.projects[0].git.as_ref().unwrap().dirty);
    let state_path = temp.0.join("batch/state.json");
    bridgeforge_core::batch::start(&state_path, &factory, &roots, &plan.aggregate_fingerprint)
        .unwrap();
    let other_state = temp.0.join("other-state/state.json");
    assert!(
        bridgeforge_core::batch::start(&other_state, &factory, &roots, &plan.aggregate_fingerprint)
            .unwrap_err()
            .contains("another batch is already active")
    );
    assert!(!other_state.exists());
    write(&target.join("after-confirmation.txt"), b"drift\n");
    let state = bridgeforge_core::batch::begin(&state_path).unwrap();
    assert_eq!(state.current_order, None);
    assert_eq!(state.projects[0].status, "deferred");
    assert!(
        state.projects[0]
            .result
            .as_deref()
            .unwrap()
            .contains("drifted")
    );
}

#[test]
fn batch_restart_requires_committed_bug_and_changed_controller_witness() {
    let temp = TestDirectory::new("batch-restart");
    let factory = synced_repo(&temp.0, "factory");
    // Match the official factory: operational state is not source content.
    write(&factory.join(".gitignore"), b".runtime/\n");
    let first = synced_repo(&temp.0, "first");
    let second = synced_repo(&temp.0, "second");
    let payload = b"managed\n";
    let payload_sha = format!("sha256:{:x}", Sha256::digest(payload));
    let contract = json!({
        "schema_version": 4,
        "release_version": "1.2.3",
        "host": "codex",
        "stamp": ".codex/.bridgeforge_codex_version",
        "contract_target": ".codex/managed-skeleton.json",
        "baseline_model": "current-only", "compatibility_baseline": "1.2.3",
        "assets": [{
            "id": "example",
            "source": "templates/example.txt",
            "target": ".codex/example.txt",
            "strategy": "whole",
            "current_sha256": payload_sha
        }],
        "generated_assets": []
    });
    write(&factory.join("VERSION"), b"1.2.3\n");
    write(&factory.join("templates/example.txt"), payload);
    write(&factory.join(".codex/example.txt"), payload);
    for path in [
        factory.join("templates/managed-skeleton.json"),
        factory.join(".codex/managed-skeleton.json"),
    ] {
        write(
            &path,
            format!("{}\n", serde_json::to_string_pretty(&contract).unwrap()).as_bytes(),
        );
    }
    let controller = factory.join("templates/hooks/crates/bridgeforge-core/src/batch.rs");
    write(&controller, b"batch controller witness v1\n");
    commit_and_push(&factory, "factory v1");
    mark_official_factory(&factory);
    for target in [&first, &second] {
        write(&target.join("README.md"), b"target\n");
        commit_and_push(target, "target");
    }

    let roots = vec![first, second];
    let plan = bridgeforge_core::batch::plan(&factory, &roots).unwrap();
    let state_path = temp.0.join("batch/state.json");
    bridgeforge_core::batch::start(&state_path, &factory, &roots, &plan.aggregate_fingerprint)
        .unwrap();
    for _ in 0..2 {
        bridgeforge_core::batch::begin(&state_path).unwrap();
        bridgeforge_core::batch::finish(
            &state_path,
            false,
            "shared controller failure".into(),
            Some("bridgeforge:batch-controller".into()),
        )
        .unwrap();
    }
    assert!(bridgeforge_core::batch::restart(&state_path, "doc/2_bugs/BUG-batch.md").is_err());

    write(&controller, b"batch controller witness v2\n");
    write(
        &factory.join("doc/2_bugs/BUG-batch.md"),
        b"# Batch controller regression\n",
    );
    commit_and_push(&factory, "fix batch controller");
    let restarted =
        bridgeforge_core::batch::restart(&state_path, "doc/2_bugs/BUG-batch.md").unwrap();
    assert_eq!(restarted.generation, 2);
    assert!(
        restarted
            .projects
            .iter()
            .all(|project| project.status == "planned")
    );
    assert!(restarted.common_issue_signature.is_none());
}

#[test]
fn project_sync_real_contract_preserves_project_owned_zones_rows_and_hooks() {
    let temp = TestDirectory::new("project-sync-real-contract");
    let project = temp.0.join("ExampleProject");
    fs::create_dir_all(project.join("doc")).unwrap();
    let factory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();

    let canonical_agents = fs::read_to_string(factory.join("templates/AGENTS.md")).unwrap();
    let project_begin = "<!-- BRIDGEFORGE:PROJECT:BEGIN -->";
    let project_end = "<!-- BRIDGEFORGE:PROJECT:END -->";
    let start = canonical_agents.find(project_begin).unwrap();
    let stop = canonical_agents.find(project_end).unwrap() + project_end.len();
    let custom_project_zone = format!(
        "{project_begin}\n## 项目级专区\n\n### 项目架构红线\n\n- CUSTOM-PROJECT-RULE\n\n### 项目目录地图\n\n- custom/**\n\n### 项目快速命令\n\n```text\ncustom-check\n```\n{project_end}"
    );
    let agents = format!(
        "{}{}{}",
        &canonical_agents[..start],
        custom_project_zone,
        &canonical_agents[stop..]
    );
    write(&project.join("AGENTS.md"), agents.as_bytes());

    let canonical_doc = fs::read_to_string(factory.join("templates/doc/README.md"))
        .unwrap()
        .replace("{{PROJECT_NAME}}", "ExampleProject");
    let doc = canonical_doc.replace(
        "## 2_bugs/",
        "| project-only/ | 用户项目内容 |\n\n## 2_bugs/",
    );
    write(&project.join("doc/README.md"), doc.as_bytes());

    let mut hooks: serde_json::Value =
        serde_json::from_slice(&fs::read(factory.join("templates/hooks.json")).unwrap()).unwrap();
    hooks["hooks"]["SessionStart"]
        .as_array_mut()
        .unwrap()
        .extend([
            json!({"matcher": "", "hooks": [{"type": "command", "command": "custom-tool --check"}]}),
            json!({"matcher": "", "hooks": [{"type": "command", "command": ".venv/Scripts/python.exe .codex/hooks/hook_dispatcher.py session-start"}]}),
        ]);
    write(
        &project.join(".codex/hooks.json"),
        format!("{}\n", serde_json::to_string_pretty(&hooks).unwrap()).as_bytes(),
    );
    fs::create_dir_all(project.join(".codex/bin")).unwrap();
    for name in [
        "bridgeforge.exe",
        "bridgeforge-hook.exe",
        "build-receipt-hook.json",
        "build-receipt-cli.json",
    ] {
        fs::copy(
            factory.join(".codex/bin").join(name),
            project.join(".codex/bin").join(name),
        )
        .unwrap();
    }

    write(
        &project.join(".codex/.bridgeforge_codex_version"),
        &fs::read(factory.join("VERSION")).unwrap(),
    );
    let plan = build_plan(&project, &factory, SyncMode::Update).unwrap();
    assert!(plan.gaps.is_empty(), "{:?}", plan.gaps);
    assert!(plan.blockers.is_empty(), "{:?}", plan.blockers);
    let fingerprint = plan.aggregate_fingerprint.clone();
    apply_plan(plan, &fingerprint, true).unwrap();

    let agents = fs::read_to_string(project.join("AGENTS.md")).unwrap();
    assert!(agents.contains("CUSTOM-PROJECT-RULE"));
    assert!(!agents.contains("{{PROJECT_NAME}}"));
    let doc = fs::read_to_string(project.join("doc/README.md")).unwrap();
    assert!(doc.contains("| project-only/ | 用户项目内容 |"));
    let hooks = fs::read_to_string(project.join(".codex/hooks.json")).unwrap();
    assert!(hooks.contains("custom-tool --check"));
    assert!(!hooks.contains("hook_dispatcher.py"));
    assert!(hooks.contains("bridgeforge-codex.project-hook.v1:session-start"));
}

#[test]
fn memory_push_rejects_rewritten_or_multiple_unapproved_destinations() {
    for destination in [
        "https://gitlab.com/owner/bridgeforge-codex-memories",
        "https://github.com/other/bridgeforge-codex-memories",
        "https://github.com/owner/bridgeforge-codex-memories\nhttps://github.com/other/bridgeforge-codex-memories",
    ] {
        let temp = TestDirectory::new("memory-push-destination");
        let remote = temp.0.join("memory.git");
        git_at(&temp.0, &["init", "--bare", remote.to_str().unwrap()]);
        let memories = temp.0.join("memories");
        let state = temp.0.join("state");
        write(&memories.join("note.md"), b"must not be published");
        let runner = LocalMemoryRunner(remote.clone(), Some(destination));
        assert!(reconcile(&memories, &state, MEMORY_REMOTE, &runner).is_err());
        let request = ProcessRequest {
            args: vec!["for-each-ref".into()],
            ..ProcessRequest::new("git", &remote)
        };
        let refs = SystemProcessRunner.run(&request).unwrap();
        assert_eq!(refs.code, 0);
        assert!(
            refs.stdout.is_empty(),
            "unapproved destination must not receive a commit"
        );
        assert_eq!(
            fs::read(memories.join("note.md")).unwrap(),
            b"must not be published"
        );
    }
}

#[test]
fn memory_remote_push_then_restore_uses_real_local_git_remote() {
    let temp = TestDirectory::new("memory-remote");
    let remote = temp.0.join("memory.git");
    let runner = LocalMemoryRunner(remote.clone(), None);
    let result = Command::new("git")
        .args(["init", "--bare", remote.to_string_lossy().as_ref()])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );

    let first_memories = temp.0.join("first/memories");
    let first_state = temp.0.join("first/state");
    write(&first_memories.join("note.md"), b"native memory\n");
    mark_pending(&first_state, "test-push").unwrap();
    let action = reconcile(&first_memories, &first_state, MEMORY_REMOTE, &runner).unwrap();
    assert_eq!(action, "push");
    assert!(!first_state.join("pending.json").exists());

    let second_memories = temp.0.join("second/memories");
    let second_state = temp.0.join("second/state");
    mark_pending(&second_state, "test-restore").unwrap();
    let action = reconcile(&second_memories, &second_state, MEMORY_REMOTE, &runner).unwrap();
    assert_eq!(action, "restore");
    assert_eq!(
        fs::read(second_memories.join("note.md")).unwrap(),
        b"native memory\n"
    );
    assert!(!second_state.join("pending.json").exists());
}

#[test]
fn memory_same_path_conflict_requires_explicit_resolution() {
    let temp = TestDirectory::new("memory-conflict");
    let remote = temp.0.join("memory.git");
    let runner = LocalMemoryRunner(remote.clone(), None);
    assert!(
        Command::new("git")
            .args(["init", "--bare", remote.to_string_lossy().as_ref()])
            .status()
            .unwrap()
            .success()
    );
    let first_memories = temp.0.join("first/memories");
    let first_state = temp.0.join("first/state");
    let second_memories = temp.0.join("second/memories");
    let second_state = temp.0.join("second/state");
    write(&first_memories.join("note.md"), b"base\n");
    mark_pending(&first_state, "seed").unwrap();
    reconcile(&first_memories, &first_state, MEMORY_REMOTE, &runner).unwrap();
    mark_pending(&second_state, "restore").unwrap();
    reconcile(&second_memories, &second_state, MEMORY_REMOTE, &runner).unwrap();

    write(&first_memories.join("note.md"), b"first\n");
    mark_pending(&first_state, "first-edit").unwrap();
    reconcile(&first_memories, &first_state, MEMORY_REMOTE, &runner).unwrap();
    write(&second_memories.join("note.md"), b"second\n");
    mark_pending(&second_state, "second-edit").unwrap();
    assert_eq!(
        reconcile(&second_memories, &second_state, MEMORY_REMOTE, &runner,).unwrap(),
        "conflicted"
    );
    let active: serde_json::Value =
        serde_json::from_slice(&fs::read(second_state.join("active-conflict.json")).unwrap())
            .unwrap();
    let conflict_id = active["conflictId"].as_str().unwrap();
    // The remote advances after conflict capture, but converges to the exact
    // bytes that were captured as this device's local side. Resolution must
    // safely replay against the new parent instead of forcing a stale commit.
    write(&first_memories.join("note.md"), b"second\n");
    mark_pending(&first_state, "converge-remote").unwrap();
    assert_eq!(
        reconcile(&first_memories, &first_state, MEMORY_REMOTE, &runner,).unwrap(),
        "push"
    );
    assert_eq!(
        resolve_conflict_with_choices(
            &second_memories,
            &second_state,
            MEMORY_REMOTE,
            conflict_id,
            &[("note.md".into(), "local".into())],
            &runner,
        )
        .unwrap(),
        "resolved"
    );
    assert_eq!(
        fs::read(second_memories.join("note.md")).unwrap(),
        b"second\n"
    );
    assert!(!second_state.join("active-conflict.json").exists());
    assert!(
        apply_conflict_choices(
            &second_state,
            conflict_id,
            &[("note.md".into(), "remote".into())],
        )
        .is_err()
    );
}
