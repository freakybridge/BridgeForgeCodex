#[path = "../support/factory_root.rs"]
mod factory_repository;

use super::*;

#[test]
fn unsupported_skeleton_plan_and_apply_do_not_write() {
    let (root, project, factory) = upgrade_fixture("7.0.5", "8.1.0");
    assert!(
        build_plan(&project, &factory, SyncMode::Update)
            .unwrap_err()
            .contains("minimum supported")
    );
    assert!(!project.join(".runtime").exists());
    assert!(!project.join("managed.txt").exists());
    fs::remove_dir_all(root).unwrap();
    let (root, plan) = transaction_fixture(true);
    fs::write(root.join(".codex/.bridgeforge_codex_version"), b"7.0.5\n").unwrap();
    let fp = plan.aggregate_fingerprint.clone();
    assert!(
        apply_plan(plan, &fp, true)
            .unwrap_err()
            .contains("minimum supported")
    );
    assert!(!root.join(".runtime").exists());
    assert_eq!(fs::read(root.join("managed.txt")).unwrap(), b"old\n");
    fs::remove_dir_all(root).unwrap();
}
use crate::ProcessRequest;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn r05_render_asset_preserves_strategy_bytes_and_disk_snapshot() {
    let (root, _) = transaction_fixture(true);
    let source = root.join("source.txt");
    let target = root.join("target.txt");
    let cases: &[(&str, &[u8], Option<&[u8]>, &[u8])] = &[
        ("seed", b"new\n", None, b"new\n"),
        ("seed", b"new\n", Some(b"custom\r\n"), b"custom\r\n"),
        ("whole", b"new\r\n", Some(b"old\n"), b"new\r\n"),
        (
            "region",
            b"BEGIN\nnew\nEND\n",
            Some(b"prefix\nBEGIN\nold\nEND\ntail"),
            b"prefix\nBEGIN\nnew\nEND\ntail",
        ),
        ("region", b"BEGIN\nnew\nEND\n", None, b"BEGIN\nnew\nEND\n"),
        ("merge", b"{\"a\":1}", None, b"{\"a\":1}"),
        (
            "merge",
            b"{\"a\":1}",
            Some(b"{\"custom\":true}"),
            b"{\n  \"custom\": true,\n  \"a\": 1\n}\n",
        ),
    ];
    for &(strategy, incoming, current, expected) in cases {
        fs::write(&source, incoming).unwrap();
        if let Some(current) = current {
            fs::write(&target, current).unwrap();
        } else if target.exists() {
            fs::remove_file(&target).unwrap();
        }
        let asset = json!({"id":"fixture", "source":"source.txt", "target":"target.txt",
            "strategy":strategy, "region":{"begin":"BEGIN", "end":"END"}});
        let (rendered, before) = render_asset(&root, &root, &asset, false).unwrap();
        assert_eq!(rendered, expected, "{strategy}");
        assert_eq!(before.as_deref(), current, "{strategy}");
        assert_eq!(fs::read(&target).ok().as_deref(), current);
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn r05_render_asset_special_policy_precedence() {
    let (root, _) = transaction_fixture(true);
    let source = root.join("source.txt");
    let target = root.join("target.txt");
    fs::write(&source, b"unused").unwrap();
    fs::write(&target, b"disk\r\n").unwrap();
    let mut asset =
        json!({"id":"fixture", "source":"source.txt", "target":"target.txt", "strategy":"seed"});
    // AGENTS zones take precedence over both seed and the Hook merge policy.
    asset["agents_zones"] = json!({"project":{"begin":"BEGIN", "end":"END"}});
    asset["merge_policy"] = json!("codex-hooks");
    fs::write(&source, b"public-new\nBEGIN\ndefault\nEND\n").unwrap();
    fs::write(&target, b"public-old\nBEGIN\ncustom\r\nEND\n").unwrap();
    assert_eq!(
        render_asset(&root, &root, &asset, false).unwrap().0,
        b"public-new\nBEGIN\ncustom\r\nEND\n"
    );

    asset.as_object_mut().unwrap().remove("agents_zones");
    asset["managed_blocks"] = json!({"headings":["## Managed"], "keyed_tables":[]});
    fs::write(&source, b"## Managed\nnew\n").unwrap();
    fs::write(&target, b"## Managed\nold\n## Custom\nkeep\n").unwrap();
    assert_eq!(
        render_asset(&root, &root, &asset, false).unwrap().0,
        b"## Managed\nnew\n## Custom\nkeep\n"
    );

    asset.as_object_mut().unwrap().remove("managed_blocks");
    fs::write(&source, b"{\"hooks\":{}}").unwrap();
    fs::write(&target, b"{\"hooks\":{},\"custom\":true}").unwrap();
    assert_eq!(
        render_asset(&root, &root, &asset, false).unwrap().0,
        b"{\n  \"hooks\": {},\n  \"custom\": true\n}\n"
    );

    asset["strategy"] = json!("merge");
    asset["merge_policy"] = json!("git-attributes-default-lf");
    fs::write(&target, b"custom -text").unwrap();
    let merged = render_asset(&root, &root, &asset, false).unwrap().0;
    assert_eq!(merged, b"custom -text\n* text=auto eol=lf\n");
    fs::write(&target, &merged).unwrap();
    assert_eq!(render_asset(&root, &root, &asset, false).unwrap().0, merged);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn r05_render_asset_retains_rendering_and_failure_order() {
    let (root, _) = transaction_fixture(true);
    let source = root.join("source.txt");
    let target = root.join("target.txt");
    let mut asset = json!({"id":"fixture", "source":"source.txt", "target":"target.txt",
        "strategy":"whole", "render":"project-name"});
    fs::write(&source, b"{{PROJECT_NAME}}\r\n").unwrap();
    assert_eq!(
        render_asset(&root, &root, &asset, false).unwrap().0,
        format!("{}\r\n", root.file_name().unwrap().to_str().unwrap()).into_bytes()
    );
    fs::write(&source, [0xff]).unwrap();
    assert_eq!(
        render_asset(&root, &root, &asset, false).unwrap_err(),
        "asset \"fixture\" render source is not UTF-8"
    );
    asset["strategy"] = json!("seed");
    fs::write(&target, b"keep").unwrap();
    assert_eq!(
        render_asset(&root, &root, &asset, false).unwrap().0,
        b"keep"
    );
    asset["strategy"] = json!("unknown");
    assert_eq!(
        render_asset(&root, &root, &asset, false).unwrap_err(),
        "unsupported asset strategy: fixture: unknown"
    );
    fs::remove_file(&source).unwrap();
    assert!(
        render_asset(&root, &root, &asset, false)
            .unwrap_err()
            .starts_with("cannot read asset source source.txt:")
    );
    assert_eq!(fs::read(target).unwrap(), b"keep");
    fs::remove_dir_all(root).unwrap();
}

fn upgrade_fixture(previous: &str, release: &str) -> (PathBuf, PathBuf, PathBuf) {
    let (root, _) = transaction_fixture(true);
    let factory = root.join("factory");
    let project = root.join("project");
    fs::create_dir_all(factory.join("templates")).unwrap();
    fs::create_dir_all(project.join(".codex")).unwrap();
    fs::write(
        project.join(".codex/.bridgeforge_codex_version"),
        format!("{previous}\n"),
    )
    .unwrap();
    // Pre-baseline contracts may be arbitrary bytes; no compatibility parser may read them.
    fs::write(
        project.join(".codex/managed-skeleton.json"),
        serde_json::to_vec(&json!({"release_version":previous})).unwrap(),
    )
    .unwrap();
    fs::write(factory.join("templates/managed.txt"), b"new\n").unwrap();
    let contract = json!({
        "schema_version":4,"release_version":release,"compatibility_baseline":"8.0.1",
        "baseline_model":"current-only","host":"codex",
        "stamp":".codex/.bridgeforge_codex_version","contract_target":".codex/managed-skeleton.json",
        "assets":[{"id":"managed.asset","source":"templates/managed.txt","target":"managed.txt",
            "strategy":"whole","current_sha256":sha_git(b"new\n")}],"generated_assets":[]
    });
    fs::write(
        factory.join("templates/managed-skeleton.json"),
        serde_json::to_vec(&contract).unwrap(),
    )
    .unwrap();
    (root, project, factory)
}

#[test]
fn retired_legacy_assets_use_project_preservation_without_semantic_migration() {
    for previous in ["8.0.0", "8.0.1"] {
        let (root, project, factory) = upgrade_fixture(previous, "8.1.0");
        let paths = [
            ".codex/rules/old.md",
            ".codex/memory/old.md",
            ".codex/memory/MEMORY.md",
        ];
        for path in paths {
            fs::create_dir_all(project.join(path).parent().unwrap()).unwrap();
            fs::write(project.join(path), [0xff, 0, 1]).unwrap();
        }
        let plan = build_plan(&project, &factory, SyncMode::Update).unwrap();
        let serialized = serde_json::to_value(&plan).unwrap();
        assert!(serialized.get("asset_migration").is_none());
        let plan = if previous == "8.0.0" {
            assert!(!plan.gaps.is_empty() || !plan.blockers.is_empty());
            let fingerprint = plan.aggregate_fingerprint.clone();
            assert!(apply_plan(plan, &fingerprint, true).is_err());
            assert!(!project.join(".runtime").exists());
            let choices =
                json!({"preserve":paths.map(|p| format!("P:project-file:{p}")),"delete":[]});
            build_plan_with_inputs(&project, &factory, SyncMode::Update, Some(&choices)).unwrap()
        } else {
            plan
        };
        let fingerprint = plan.aggregate_fingerprint.clone();
        let receipt = apply_plan(plan, &fingerprint, true).unwrap();
        assert!(
            serde_json::to_value(receipt)
                .unwrap()
                .get("asset_migration_manifest_sha256")
                .is_none()
        );
        for path in paths {
            assert_eq!(fs::read(project.join(path)).unwrap(), [0xff, 0, 1]);
        }
        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn retired_legacy_asset_explicit_delete_keeps_drift_and_rollback_guards() {
    for drift in [false, true] {
        let (root, project, factory) = upgrade_fixture("8.0.0", "8.1.0");
        let relative = ".codex/rules/old.md";
        let path = project.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"original\r\n").unwrap();
        let choices = json!({"preserve":[], "delete":[format!("P:project-file:{relative}")]});
        let plan =
            build_plan_with_inputs(&project, &factory, SyncMode::Update, Some(&choices)).unwrap();
        let fingerprint = plan.aggregate_fingerprint.clone();
        if drift {
            fs::write(&path, b"external").unwrap();
        }
        let fail = |_: &Path, _: &Path| Err("retirement rollback probe".to_string());
        assert!(apply_plan_internal(plan, &fingerprint, true, Some(&fail)).is_err());
        assert_eq!(
            fs::read(&path).unwrap(),
            if drift {
                b"external".as_slice()
            } else {
                b"original\r\n".as_slice()
            }
        );
        assert_eq!(
            fs::read_to_string(project.join(".codex/.bridgeforge_codex_version")).unwrap(),
            "8.0.0\n"
        );
        assert!(!project.join("managed.txt").exists());
        fs::remove_dir_all(root).unwrap();
    }
    let (root, project, factory) = upgrade_fixture("8.0.0", "8.1.0");
    fs::create_dir_all(project.join(".codex/memory")).unwrap();
    fs::write(project.join(".codex/memory/old.md"), b"old").unwrap();
    let choices = json!({"preserve":[], "delete":["P:project-file:.codex/memory/old.md"]});
    let plan =
        build_plan_with_inputs(&project, &factory, SyncMode::Update, Some(&choices)).unwrap();
    let fingerprint = plan.aggregate_fingerprint.clone();
    apply_plan(plan, &fingerprint, true).unwrap();
    assert!(!project.join(".codex/memory/old.md").exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn current_configuration_init_is_atomic_with_project_update_and_rejects_input_drift() {
    for scenario in ["success", "file-drift", "head-drift", "rollback"] {
        let (root, project, factory) = upgrade_fixture("8.0.1", "8.1.0");
        let contract_path = factory.join("templates/managed-skeleton.json");
        let mut contract: Value =
            serde_json::from_slice(&fs::read(&contract_path).unwrap()).unwrap();
        contract["release_model"] = json!("product-packages");
        fs::write(&contract_path, serde_json::to_vec(&contract).unwrap()).unwrap();
        let config_path = project.join(".codex/bridgeforge-version.json");
        let original = fs::read(&config_path).ok();
        fs::write(project.join("VERSION"), b"1.0.0\n").unwrap();
        fs::write(project.join("CHANGELOG.md"), b"# Changelog\n").unwrap();
        fs::write(project.join(".gitignore"), b".runtime/\n").unwrap();
        fs::write(project.join("app.txt"), b"before\n").unwrap();
        let git = |args: &[&str]| {
            let output = std::process::Command::new("git")
                .args(args)
                .current_dir(&project)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
        };
        git(&["init"]);
        git(&["config", "user.name", "Fixture"]);
        git(&["config", "user.email", "fixture@example.invalid"]);
        git(&["add", "."]);
        git(&["commit", "-m", "chore: initial"]);
        let plan = build_plan(&project, &factory, SyncMode::Update).unwrap();
        assert!(plan.risk.iter().any(|a| a.id == "migration.release-model"));
        assert_eq!(fs::read(&config_path).ok(), original);
        assert!(apply_plan(plan.clone(), &plan.aggregate_fingerprint, false).is_err());
        if scenario == "file-drift" {
            fs::write(project.join("app.txt"), b"after\n").unwrap();
        } else if scenario == "head-drift" {
            git(&["commit", "--allow-empty", "-m", "chore: concurrent commit"]);
        }
        let result = if scenario == "rollback" {
            apply_plan_internal(
                plan.clone(),
                &plan.aggregate_fingerprint,
                true,
                Some(&|_, _| Err("injected failure".into())),
            )
        } else {
            apply_plan(plan.clone(), &plan.aggregate_fingerprint, true)
        };
        if scenario == "success" {
            let receipt = result.unwrap();
            assert!(receipt.stamp_written_last);
            let updated = fs::read(&config_path).unwrap();
            let cfg: Value = serde_json::from_slice(&updated).unwrap();
            assert_eq!(cfg["release_model"], "product-packages");
            assert_eq!(fs::read(project.join("VERSION")).unwrap(), b"1.0.0\n");
            let next = build_plan(&project, &factory, SyncMode::Update).unwrap();
            assert!(!next.risk.iter().any(|a| a.id == "migration.release-model"));
            assert_eq!(fs::read(&config_path).unwrap(), updated);
        } else {
            assert!(result.is_err(), "{scenario}");
            assert_eq!(fs::read(&config_path).ok(), original);
            assert_eq!(
                fs::read(project.join(".codex/.bridgeforge_codex_version")).unwrap(),
                b"8.0.1\n"
            );
        }
        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn release_cutover_fresh_init_defers_until_committed_version_baseline_exists() {
    let (root, _, factory) = upgrade_fixture("8.0.1", "8.1.0");
    let fresh = root.join("fresh");
    fs::create_dir_all(&fresh).unwrap();
    fs::write(fresh.join("VERSION"), b"1.0.0\n").unwrap();
    let contract_path = factory.join("templates/managed-skeleton.json");
    let mut contract: Value = serde_json::from_slice(&fs::read(&contract_path).unwrap()).unwrap();
    contract["release_model"] = json!("product-packages");
    fs::write(&contract_path, serde_json::to_vec(&contract).unwrap()).unwrap();
    let plan = build_plan(&fresh, &factory, SyncMode::Init).unwrap();
    assert!(plan.release_upgrade.is_none());
    assert_eq!(plan.release_setup["status"], "migration-required");
    let receipt = apply_plan(plan.clone(), &plan.aggregate_fingerprint, false).unwrap();
    assert_eq!(receipt.release_setup["status"], "migration-required");
    assert!(!fresh.join(".codex/bridgeforge-version.json").exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn fixed_baseline_does_not_move_with_the_release() {
    for (previous, release, rebuild) in [
        ("8.0.0", "8.0.1", true),
        ("8.0.1", "8.0.1", false),
        ("8.0.1", "8.1.0", false),
        ("8.1.0", "8.2.0", false),
    ] {
        let (root, project, factory) = upgrade_fixture(previous, release);
        fs::write(
            project.join(".codex/local-config.json"),
            b"project-owned\r\n",
        )
        .unwrap();
        let plan = build_plan(&project, &factory, SyncMode::Update).unwrap();
        assert_eq!(plan.preservation_manifest["destructive_rebuild"], rebuild);
        assert_eq!(
            plan.preservation_manifest["compatibility_baseline"],
            "8.0.1"
        );
        assert_eq!(!plan.gaps.is_empty(), rebuild);
        if !rebuild {
            apply_plan(plan.clone(), &plan.aggregate_fingerprint, true).unwrap();
            assert_eq!(
                fs::read(project.join(".codex/local-config.json")).unwrap(),
                b"project-owned\r\n"
            );
            assert_eq!(
                build_plan(&project, &factory, SyncMode::Update)
                    .unwrap()
                    .status,
                "current"
            );
        }
        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn upgrade_keeps_skeleton_ready_and_preserves_project_release_checks() {
    for payload in [None, Some(br#"{"schema":1,"checks":[{"id":"project","program":"git","args":["diff","--check"],"timeout_seconds":30}]}"#.as_slice()), Some(b"invalid project config\r\n".as_slice())] {
        let (root, project, factory) = upgrade_fixture("8.0.1", "8.1.0");
        fs::write(project.join("VERSION"), b"1.0.0\n").unwrap();
        let config = project.join(".codex/development-checks.json");
        if let Some(bytes) = payload { fs::write(&config, bytes).unwrap(); }
        let plan = build_plan(&project, &factory, SyncMode::Update).unwrap();
        assert_eq!(plan.readiness, "ready");
        assert!(!plan.confirmation_required);
        assert!(plan.gaps.is_empty());
        // This fixture has VERSION but no product-packages configuration.
        // Skeleton updates must preserve that release-setup gap, not certify it.
        let expected = "migration-required";
        assert_eq!(plan.release_setup["status"], expected);
        assert!(!plan.safe.iter().any(|action| action.target == ".codex/development-checks.json"));
        assert_eq!(fs::read(&config).ok().as_deref(), payload);
        let fingerprint = plan.aggregate_fingerprint.clone();
        let receipt = apply_plan(plan, &fingerprint, false).unwrap();
        assert_eq!(receipt.project_readiness, "ready");
        assert_eq!(receipt.release_setup["status"], expected);
        assert_eq!(fs::read(&config).ok().as_deref(), payload);
        let rendered = outcome_receipt_with_format(Ok(receipt), "combined").receipt.unwrap();
        assert_eq!(rendered["human"]["conclusion"], "已完成");
        let next = build_plan(&project, &factory, SyncMode::Update).unwrap();
        assert_eq!(next.status, "current");
        assert!(human_plan(&next)["pending"].as_array().unwrap().is_empty());
        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn artifact_layout_policy_remains_project_owned_during_update_and_dry_plan() {
    let (root, project, factory) = upgrade_fixture("8.0.1", "8.1.0");
    let path = project.join(".codex/artifact-layout.json");
    let bytes = b"{\r\n  \"schema\": 1, \"source_roots\": [\"custom-src/\"]\r\n}\r\n";
    fs::write(&path, bytes).unwrap();
    let plan = build_plan(&project, &factory, SyncMode::Update).unwrap();
    assert_eq!(
        fs::read(&path).unwrap(),
        bytes,
        "planning must be zero-write"
    );
    assert!(
        !plan
            .safe
            .iter()
            .chain(plan.risk.iter())
            .any(|a| a.target == ".codex/artifact-layout.json")
    );
    let fingerprint = plan.aggregate_fingerprint.clone();
    apply_plan(plan, &fingerprint, false).unwrap();
    assert_eq!(
        fs::read(&path).unwrap(),
        bytes,
        "upgrade must preserve policy bytes"
    );
    fs::remove_file(&path).unwrap();
    let plan = build_plan(&project, &factory, SyncMode::Update).unwrap();
    let fingerprint = plan.aggregate_fingerprint.clone();
    assert!(
        !path.exists(),
        "planning must not seed default policy files"
    );
    apply_plan(plan, &fingerprint, false).unwrap();
    assert!(
        !path.exists(),
        "defaults must work without creating project policy"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn compatible_update_retires_old_project_map_paths() {
    let (root, project, factory) = upgrade_fixture("8.0.1", "8.1.0");
    for target in [".codex/find-doc.map.md", ".codex/sync-docs.map.md"] {
        fs::write(project.join(target), b"old project map\n").unwrap();
    }

    let plan = build_plan(&project, &factory, SyncMode::Update).unwrap();
    for target in [".codex/find-doc.map.md", ".codex/sync-docs.map.md"] {
        assert!(plan.safe.iter().any(|action| {
            action.target == target
                && action.operation == "delete"
                && action.id.starts_with("retired:project-map:")
        }));
    }
    let fingerprint = plan.aggregate_fingerprint.clone();
    apply_plan(plan, &fingerprint, false).unwrap();
    assert!(!project.join(".codex/find-doc.map.md").exists());
    assert!(!project.join(".codex/sync-docs.map.md").exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn fixed_baseline_rejects_missing_invalid_future_floor_and_downgrade() {
    let (root, project, factory) = upgrade_fixture("8.1.1", "8.1.0");
    assert!(
        build_plan(&project, &factory, SyncMode::Update)
            .unwrap_err()
            .contains("newer than")
    );
    let path = factory.join("templates/managed-skeleton.json");
    let original: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    for floor in [Value::Null, json!("invalid"), json!("8.2.0")] {
        let mut contract = original.clone();
        contract["compatibility_baseline"] = floor;
        fs::write(&path, serde_json::to_vec(&contract).unwrap()).unwrap();
        assert!(build_plan(&project, &factory, SyncMode::Update).is_err());
        assert!(!project.join(".runtime").exists());
    }
    fs::remove_dir_all(root).unwrap();
}

fn retired_role_payloads() -> Vec<String> {
    let rust = include_str!("../fixtures/retired-mechanical-sync-worker.txt").replace("\r\n", "\n");
    vec![
        rust.clone(),
        rust.replace(
            ".codex/bin/bridgeforge.exe git-sync",
            ".venv/Scripts/python.exe .codex/scripts/codex_git_sync.py",
        ),
        rust.replace(
            ".codex/bin/bridgeforge.exe git-sync",
            "python .codex/scripts/codex_git_sync.py",
        ),
    ]
}

fn write_role_input(project: &Path, target: &str, bytes: &[u8]) {
    let path = project.join(target);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

#[test]
fn retired_role_scan_streams_text_and_ignores_binary_assets() {
    let (root, project, factory) = upgrade_fixture("8.0.1", "8.1.0");
    let mut text = vec![b'x'; 2 * 1024 * 1024];
    // Place the role name across a 64 KiB read boundary.
    text.splice(65530..65530, RETIRED_SYNC_ROLE.bytes());
    write_role_input(&project, ".codex/skills/local/references/large.md", &text);
    write_role_input(&project, ".codex/skills/local/assets/image.bin", &text);
    let inputs = sync_role_inputs(&project).unwrap();
    assert_eq!(
        inputs[".codex/skills/local/references/large.md"],
        (sha_raw(&text), true)
    );
    assert!(!inputs.contains_key(".codex/skills/local/assets/image.bin"));
    assert!(serde_json::to_vec(&inputs).unwrap().len() < 1024);
    write_role_input(
        &project,
        ".codex/skills/local/references/large.md",
        b"no role dependency",
    );
    let plan = build_plan(&project, &factory, SyncMode::Update).unwrap();
    assert!(plan.gaps.is_empty());
    apply_plan(plan.clone(), &plan.aggregate_fingerprint, true).unwrap();
    assert_eq!(
        fs::read(project.join(".codex/skills/local/assets/image.bin")).unwrap(),
        text
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn retired_role_is_absent_on_init_and_removed_on_adopt() {
    let (root, project, factory) = upgrade_fixture("8.0.1", "8.1.0");
    let fresh = root.join("fresh");
    fs::create_dir_all(&fresh).unwrap();
    let init = build_plan(&fresh, &factory, SyncMode::Init).unwrap();
    apply_plan(init.clone(), &init.aggregate_fingerprint, false).unwrap();
    assert!(!fresh.join(RETIRED_SYNC_TARGET).exists());
    fs::remove_file(project.join(".codex/.bridgeforge_codex_version")).unwrap();
    write_role_input(
        &project,
        RETIRED_SYNC_TARGET,
        retired_role_payloads()[0].as_bytes(),
    );
    let adopt = build_plan(&project, &factory, SyncMode::Adopt).unwrap();
    assert!(adopt.gaps.is_empty(), "{:?}", adopt.gaps);
    apply_plan(adopt.clone(), &adopt.aggregate_fingerprint, true).unwrap();
    assert!(!project.join(RETIRED_SYNC_TARGET).exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn retired_role_known_releases_are_transactional_and_idempotent() {
    for (index, payload) in retired_role_payloads().iter().enumerate() {
        assert_eq!(sha_git(payload.as_bytes()), RETIRED_SYNC_HASHES[index]);
        for content in [payload.clone(), payload.replace('\n', "\r\n")] {
            let (root, project, factory) = upgrade_fixture("8.0.1", "8.1.0");
            write_role_input(&project, RETIRED_SYNC_TARGET, content.as_bytes());
            let before = tree_sha(&project).unwrap();
            let plan = build_plan(&project, &factory, SyncMode::Update).unwrap();
            assert_eq!(
                tree_sha(&project).unwrap(),
                before,
                "planning must be read-only"
            );
            assert!(plan.gaps.is_empty(), "{:?}", plan.gaps);
            assert!(
                plan.safe
                    .iter()
                    .any(|a| a.target == RETIRED_SYNC_TARGET && a.operation == "delete")
            );
            let receipt = apply_plan(plan.clone(), &plan.aggregate_fingerprint, true).unwrap();
            assert!(receipt.stamp_written_last);
            assert!(!project.join(RETIRED_SYNC_TARGET).exists());
            let next = build_plan(&project, &factory, SyncMode::Update).unwrap();
            assert_eq!(next.status, "current");
            assert!(next.gaps.is_empty());
            fs::remove_dir_all(root).unwrap();
        }
    }
}

#[test]
fn retired_role_custom_or_unknown_content_is_preserved_without_any_apply() {
    for previous in ["8.0.0", "8.0.1"] {
        for content in [b"custom role\n".as_slice(), b"", b"\xff\x00"] {
            let (root, project, factory) = upgrade_fixture(previous, "8.1.0");
            write_role_input(&project, RETIRED_SYNC_TARGET, content);
            // Even an old manifest claiming ownership is not retirement evidence.
            fs::write(
                project.join(".codex/managed-skeleton.json"),
                serde_json::to_vec(&json!({
                    "assets":[{"target":RETIRED_SYNC_TARGET,"current_sha256":sha_git(content)}]
                }))
                .unwrap(),
            )
            .unwrap();
            let before = tree_sha(&project).unwrap();
            let plan = build_plan(&project, &factory, SyncMode::Update).unwrap();
            assert!(plan.gaps.iter().any(|g| g.contains("custom or unknown")));
            assert!(!plan.deletes.contains(&project.join(RETIRED_SYNC_TARGET)));
            assert!(apply_plan(plan.clone(), &plan.aggregate_fingerprint, true).is_err());
            assert_eq!(tree_sha(&project).unwrap(), before);
            fs::remove_dir_all(root).unwrap();
        }
    }
}

#[test]
fn retired_role_references_block_update_even_when_role_is_already_missing() {
    for target in [
        ".codex/skills/local/SKILL.md",
        ".codex/skills/local/references/routing.md",
        "AGENTS.md",
        ".codex/config.toml",
        ".codex/agents/local.toml",
    ] {
        for role_exists in [false, true] {
            let (root, project, factory) = upgrade_fixture("8.0.1", "8.1.0");
            if role_exists {
                write_role_input(
                    &project,
                    RETIRED_SYNC_TARGET,
                    retired_role_payloads()[0].as_bytes(),
                );
            }
            write_role_input(&project, target, b"delegate to mechanical-sync-worker\n");
            let before = tree_sha(&project).unwrap();
            let plan = build_plan(&project, &factory, SyncMode::Update).unwrap();
            assert!(plan.gaps.iter().any(|g| g.contains(target)));
            assert!(!plan.deletes.contains(&project.join(RETIRED_SYNC_TARGET)));
            assert!(apply_plan(plan.clone(), &plan.aggregate_fingerprint, true).is_err());
            assert_eq!(tree_sha(&project).unwrap(), before);
            fs::remove_dir_all(root).unwrap();
        }
    }
}

#[test]
fn retired_role_and_reference_changes_invalidate_the_approved_plan() {
    for target in [
        RETIRED_SYNC_TARGET,
        ".codex/skills/new/SKILL.md",
        ".codex/config.toml",
    ] {
        for role_exists in [false, true] {
            let (root, project, factory) = upgrade_fixture("8.0.1", "8.1.0");
            if role_exists {
                write_role_input(
                    &project,
                    RETIRED_SYNC_TARGET,
                    retired_role_payloads()[0].as_bytes(),
                );
            }
            let original_stamp =
                fs::read_to_string(project.join(".codex/.bridgeforge_codex_version")).unwrap();
            let plan = build_plan(&project, &factory, SyncMode::Update).unwrap();
            write_role_input(
                &project,
                target,
                b"mechanical-sync-worker changed after planning\n",
            );
            let error = apply_plan(plan.clone(), &plan.aggregate_fingerprint, true).unwrap_err();
            assert!(error.contains("inputs drifted"), "{error}");
            assert!(!project.join("managed.txt").exists());
            assert_eq!(
                fs::read_to_string(project.join(".codex/.bridgeforge_codex_version")).unwrap(),
                original_stamp
            );
            let replanned = build_plan(&project, &factory, SyncMode::Update).unwrap();
            assert_ne!(plan.aggregate_fingerprint, replanned.aggregate_fingerprint);
            assert!(!replanned.gaps.is_empty());
            fs::remove_dir_all(root).unwrap();
        }
    }
}

#[test]
fn retired_role_is_restored_on_failure_and_late_references_are_preserved() {
    for late_reference in [false, true] {
        let (root, project, factory) = upgrade_fixture("8.0.1", "8.1.0");
        let payload = retired_role_payloads()[0].replace('\n', "\r\n");
        write_role_input(&project, RETIRED_SYNC_TARGET, payload.as_bytes());
        let original_contract = fs::read(project.join(".codex/managed-skeleton.json")).unwrap();
        let original_stamp = fs::read(project.join(".codex/.bridgeforge_codex_version")).unwrap();
        let plan = build_plan(&project, &factory, SyncMode::Update).unwrap();
        let observer = |root: &Path, _: &Path| {
            assert!(!root.join(RETIRED_SYNC_TARGET).exists());
            if late_reference {
                write_role_input(
                    root,
                    ".codex/skills/late/SKILL.md",
                    b"mechanical-sync-worker",
                );
                Ok(())
            } else {
                Err("injected after role retirement".into())
            }
        };
        let error = apply_plan_internal(
            plan.clone(),
            &plan.aggregate_fingerprint,
            true,
            Some(&observer),
        )
        .unwrap_err();
        assert!(error.contains("rolled back"), "{error}");
        assert_eq!(
            fs::read(project.join(RETIRED_SYNC_TARGET)).unwrap(),
            payload.as_bytes()
        );
        assert_eq!(
            fs::read(project.join(".codex/managed-skeleton.json")).unwrap(),
            original_contract
        );
        assert_eq!(
            fs::read(project.join(".codex/.bridgeforge_codex_version")).unwrap(),
            original_stamp
        );
        assert!(!project.join("managed.txt").exists());
        if late_reference {
            assert_eq!(
                fs::read(project.join(".codex/skills/late/SKILL.md")).unwrap(),
                b"mechanical-sync-worker"
            );
        }
        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn hook_removal_matches_path_boundaries_and_platform_commands() {
    let payload = json!({"hooks":{"Stop":[{"hooks":[
        {"command":"run .codex/hooks/project_a/entrypoint.rs"},
        {"command":"run .codex/hooks/project_ab/entrypoint.rs"},
        {"commandWindows":"run C:\\repo\\.codex\\hooks\\project_a\\entrypoint.rs"},
        {"commandWindows":"run C:\\repo\\.codex\\hooks\\project_ab\\entrypoint.rs"}
    ]}]}});
    let result: Value = serde_json::from_slice(
        &remove_hook_registrations(
            &serde_json::to_vec(&payload).unwrap(),
            &[".codex/hooks/project_a".into()],
        )
        .unwrap(),
    )
    .unwrap();
    let handlers = result["hooks"]["Stop"][0]["hooks"].as_array().unwrap();
    assert_eq!(handlers.len(), 2);
    assert!(
        handlers
            .iter()
            .all(|handler| handler.to_string().contains("project_ab"))
    );
}

#[test]
fn project_prefixed_files_use_exact_file_decisions_not_hook_bundles() {
    let (root, project, factory) = upgrade_fixture("8.0.0", "8.1.0");
    let bundle = project.join(".codex/hooks/project_business");
    fs::create_dir_all(&bundle).unwrap();
    fs::write(bundle.join("entrypoint.py"), b"business hook\r\n").unwrap();
    let targets = [
        ".codex/hooks/project_structure_check.py",
        ".codex/hooks/project_notes",
        ".codex/hooks/project_custom.rs",
    ];
    for target in targets {
        fs::write(project.join(target), b"project file\r\n").unwrap();
    }
    let before = tree_sha(&project).unwrap();
    let plan = build_plan(&project, &factory, SyncMode::Update).unwrap();
    assert_eq!(plan.preservation_manifest["destructive_rebuild"], true);
    assert!(plan.blockers.is_empty(), "{:?}", plan.blockers);
    for target in targets {
        let entry = plan.preservation_manifest["entries"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["target"] == target)
            .unwrap();
        assert_eq!(entry["kind"], "project-file");
        assert_eq!(entry["id"], format!("P:project-file:{target}"));
        assert_eq!(entry["disposition"], "user-decision");
    }
    assert!(apply_plan(plan.clone(), &plan.aggregate_fingerprint, true).is_err());
    assert_eq!(tree_sha(&project).unwrap(), before);
    assert!(!project.join(".runtime").exists());

    let file_ids = targets.map(|target| format!("P:project-file:{target}"));
    let bundle_id = "P:project-hook-bundle:.codex/hooks/project_business";
    let mut preserve_ids = file_ids.to_vec();
    preserve_ids.push(bundle_id.into());
    let choices = json!({"preserve":preserve_ids});
    let plan =
        build_plan_with_inputs(&project, &factory, SyncMode::Update, Some(&choices)).unwrap();
    assert!(plan.blockers.is_empty());
    assert!(plan.gaps.is_empty());
    fs::write(project.join(targets[0]), b"concurrent edit").unwrap();
    assert!(apply_plan(plan.clone(), &plan.aggregate_fingerprint, true).is_err());
    assert_eq!(
        fs::read(project.join(targets[0])).unwrap(),
        b"concurrent edit"
    );
    fs::write(project.join(targets[0]), b"project file\r\n").unwrap();
    apply_plan(plan.clone(), &plan.aggregate_fingerprint, true).unwrap();
    for target in targets {
        assert_eq!(fs::read(project.join(target)).unwrap(), b"project file\r\n");
    }

    // Re-enter the below-baseline fixture to exercise explicitly confirmed file retirement.
    fs::write(
        project.join(".codex/.bridgeforge_codex_version"),
        b"8.0.0\n",
    )
    .unwrap();
    let choices = json!({"preserve":[bundle_id],"delete":file_ids});
    let plan =
        build_plan_with_inputs(&project, &factory, SyncMode::Update, Some(&choices)).unwrap();
    assert!(plan.blockers.is_empty());
    assert!(plan.gaps.is_empty());
    let fail = |_: &Path, _: &Path| Err("injected file retirement failure".into());
    assert!(
        apply_plan_internal(plan.clone(), &plan.aggregate_fingerprint, true, Some(&fail))
            .unwrap_err()
            .contains("rolled back")
    );
    for target in targets {
        assert_eq!(fs::read(project.join(target)).unwrap(), b"project file\r\n");
    }
    apply_plan(plan.clone(), &plan.aggregate_fingerprint, true).unwrap();
    for target in targets {
        assert!(!project.join(target).exists());
    }
    assert_eq!(
        fs::read(bundle.join("entrypoint.py")).unwrap(),
        b"business hook\r\n"
    );
    assert_eq!(
        fs::read_to_string(project.join(".codex/.bridgeforge_codex_version")).unwrap(),
        "8.1.0\n"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn project_prefixed_file_retirement_removes_only_its_hook_registration_atomically() {
    let (root, project, factory) = upgrade_fixture("8.0.0", "8.1.0");
    let repository = factory_repository::from_manifest_dir(env!("CARGO_MANIFEST_DIR"));
    let current: Value = serde_json::from_slice(
        &fs::read(repository.join("templates/managed-skeleton.json")).unwrap(),
    )
    .unwrap();
    let mut asset = current["assets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|asset| asset["target"] == ".codex/hooks.json")
        .unwrap()
        .clone();
    let source_path = asset["source"].as_str().unwrap();
    let source = fs::read(repository.join(source_path)).unwrap();
    fs::write(factory.join(source_path), &source).unwrap();
    let document: Value = serde_json::from_slice(&source).unwrap();
    let handlers = crate::baseline::hook_handlers(&document).unwrap();
    for required in asset["merge_validation"]["required_handlers"]
        .as_array_mut()
        .unwrap()
    {
        required["sha256"] = json!(
            crate::manifest::canonical_sha(&handlers[required["id"].as_str().unwrap()]["handler"])
                .unwrap()
        );
    }
    let contract_path = factory.join("templates/managed-skeleton.json");
    let mut contract: Value = serde_json::from_slice(&fs::read(&contract_path).unwrap()).unwrap();
    contract["assets"].as_array_mut().unwrap().push(asset);
    fs::write(&contract_path, serde_json::to_vec(&contract).unwrap()).unwrap();
    let deleted = ".codex/hooks/project_custom.py";
    let kept = ".codex/hooks/project_custom.py.bak";
    fs::create_dir_all(project.join(".codex/hooks")).unwrap();
    for target in [deleted, kept] {
        fs::write(project.join(target), b"custom script\r\n").unwrap();
    }
    let mut hooks: Value = serde_json::from_slice(&source).unwrap();
    let groups = hooks["hooks"]["Stop"].as_array_mut().unwrap();
    groups.push(
        json!({"hooks":[{"type":"command","command":format!("run {deleted}"),
        "commandWindows":format!("run \"{}\"", deleted.replace('/', "\\"))}]}),
    );
    groups.push(
        json!({"hooks":[{"type":"command","command":format!("run {kept}"),
        "commandWindows":format!("run \"{}\"", kept.replace('/', "\\"))}]}),
    );
    let original = serde_json::to_vec_pretty(&hooks).unwrap();
    let hooks_path = project.join(".codex/hooks.json");
    fs::write(&hooks_path, &original).unwrap();
    let choices = json!({"preserve":[format!("P:project-file:{kept}")],
        "delete":[format!("P:project-file:{deleted}")]});
    let plan =
        build_plan_with_inputs(&project, &factory, SyncMode::Update, Some(&choices)).unwrap();
    assert_eq!(plan.preservation_manifest["destructive_rebuild"], true);
    assert!(plan.blockers.is_empty());
    assert!(plan.gaps.is_empty());
    let fail = |_: &Path, _: &Path| Err("injected registration failure".into());
    assert!(
        apply_plan_internal(plan.clone(), &plan.aggregate_fingerprint, true, Some(&fail))
            .unwrap_err()
            .contains("rolled back")
    );
    assert_eq!(fs::read(&hooks_path).unwrap(), original);
    assert_eq!(
        fs::read(project.join(deleted)).unwrap(),
        b"custom script\r\n"
    );
    apply_plan(plan.clone(), &plan.aggregate_fingerprint, true).unwrap();
    assert!(!project.join(deleted).exists());
    assert_eq!(fs::read(project.join(kept)).unwrap(), b"custom script\r\n");
    let result: Value = serde_json::from_slice(&fs::read(&hooks_path).unwrap()).unwrap();
    let commands = result["hooks"]["Stop"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|group| group["hooks"].as_array().unwrap())
        .flat_map(|handler| {
            ["command", "commandWindows"]
                .into_iter()
                .filter_map(move |key| handler[key].as_str())
        })
        .map(|command| command.replace('\\', "/"))
        .collect::<Vec<_>>();
    assert!(
        !commands
            .iter()
            .any(|command| command == &format!("run {deleted}")
                || command == &format!("run \"{deleted}\""))
    );
    assert!(commands.contains(&format!("run {kept}")));
    assert!(commands.contains(&format!("run \"{kept}\"")));
    fs::remove_dir_all(root).unwrap();
}

#[test]
#[cfg(windows)]
fn project_prefixed_junction_still_blocks_rebuild() {
    let (root, project, factory) = upgrade_fixture("8.0.0", "8.1.0");
    let external = root.join("external");
    fs::create_dir_all(&external).unwrap();
    fs::write(external.join("keep.txt"), b"external content").unwrap();
    let link = project.join(".codex/hooks/project_structure_check.py");
    fs::create_dir_all(link.parent().unwrap()).unwrap();
    let mut request = ProcessRequest::new("cmd.exe", &root);
    request.args = vec![
        "/c".into(),
        "mklink".into(),
        "/J".into(),
        link.to_string_lossy().replace('/', "\\").into(),
        external.to_string_lossy().replace('/', "\\").into(),
    ];
    request.timeout = std::time::Duration::from_secs(10);
    let output = crate::SystemProcessRunner.run(&request).unwrap();
    assert!(!output.timed_out);
    assert_eq!(
        output.code,
        0,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(crate::persistence::is_link_or_reparse(&link).unwrap());
    let plan = build_plan(&project, &factory, SyncMode::Update).unwrap();
    assert_eq!(plan.preservation_manifest["destructive_rebuild"], true);
    assert!(
        plan.blockers
            .iter()
            .any(|item| item.contains("project hook bundle must be a plain"))
    );
    assert!(apply_plan(plan.clone(), &plan.aggregate_fingerprint, true).is_err());
    assert!(!project.join(".runtime").exists());
    assert_eq!(
        fs::read(external.join("keep.txt")).unwrap(),
        b"external content"
    );
    fs::remove_dir(&link).unwrap();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn legacy_business_hook_bundle_is_language_neutral_and_fingerprinted() {
    let (root, project, factory) = upgrade_fixture("8.0.0", "8.0.1");
    let bundle = project.join(".codex/hooks/project_business");
    fs::create_dir_all(&bundle).unwrap();
    fs::write(bundle.join("entrypoint.py"), b"project owned Python\r\n").unwrap();
    let plan = build_plan(&project, &factory, SyncMode::Update).unwrap();
    assert_eq!(plan.preservation_manifest["destructive_rebuild"], true);
    assert!(plan.blockers.is_empty(), "{:?}", plan.blockers);
    assert!(!plan.gaps.is_empty());
    let decisions = json!({"preserve":["P:project-hook-bundle:.codex/hooks/project_business"]});
    let plan =
        build_plan_with_inputs(&project, &factory, SyncMode::Update, Some(&decisions)).unwrap();
    assert!(plan.gaps.is_empty());
    fs::write(bundle.join("late.txt"), b"concurrent content").unwrap();
    assert!(
        apply_plan(plan.clone(), &plan.aggregate_fingerprint, true)
            .unwrap_err()
            .contains("preserved project asset drifted")
    );
    let plan =
        build_plan_with_inputs(&project, &factory, SyncMode::Update, Some(&decisions)).unwrap();
    apply_plan(plan.clone(), &plan.aggregate_fingerprint, true).unwrap();
    assert_eq!(
        fs::read(bundle.join("entrypoint.py")).unwrap(),
        b"project owned Python\r\n"
    );
    assert_eq!(
        fs::read(bundle.join("late.txt")).unwrap(),
        b"concurrent content"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn retired_hook_directories_rollback_with_their_files() {
    let (root, mut plan) = transaction_fixture(true);
    let bundle = root.join(".codex/hooks/project_old");
    fs::create_dir_all(bundle.join("empty/subdirectory")).unwrap();
    let entry = bundle.join("entrypoint.rs");
    fs::write(&entry, b"fn main() {}\n").unwrap();
    plan.preservation_manifest = json!({"entries":[{"kind":"project-hook-bundle","target":".codex/hooks/project_old","disposition":"delete","before_sha256":tree_sha(&bundle).unwrap()}]});
    plan.deletes.push(entry.clone());
    plan.risk.push(SyncAction {
        id: "rebuild.remove:old-hook".into(),
        target: ".codex/hooks/project_old/entrypoint.rs".into(),
        operation: "delete".into(),
        risk: true,
        before_sha256: Some(sha_git(b"fn main() {}\n")),
        after_sha256: "sha256:deleted".into(),
    });
    plan.aggregate_fingerprint = plan_fingerprint(
        &plan.mode,
        &plan.current_version,
        &plan.safe,
        &plan.risk,
        &plan.preservation_manifest,
        &plan.generated_source_fingerprints,
    )
    .unwrap();
    let observer = |_: &Path, _: &Path| {
        assert!(!bundle.exists());
        Err("injected finalize failure".into())
    };
    assert!(
        apply_plan_internal(
            plan.clone(),
            &plan.aggregate_fingerprint,
            true,
            Some(&observer)
        )
        .unwrap_err()
        .contains("rolled back")
    );
    assert_eq!(fs::read(&entry).unwrap(), b"fn main() {}\n");
    assert!(bundle.join("empty/subdirectory").is_dir());
    apply_plan(plan.clone(), &plan.aggregate_fingerprint, true).unwrap();
    assert!(!bundle.exists());
    fs::remove_dir_all(root).unwrap();
}

struct NeverRun;
impl ProcessRunner for NeverRun {
    fn run(&self, _: &ProcessRequest) -> std::io::Result<crate::ProcessOutput> {
        panic!("no command may run before acquiring the project lock");
    }
}

#[test]
fn project_lock_blocks_apply_and_build_before_targets_change() {
    let (root, plan) = transaction_fixture(true);
    let lock = ProjectLock::acquire(&root).unwrap();
    let fingerprint = plan.aggregate_fingerprint.clone();
    assert!(
        apply_plan(plan.clone(), &fingerprint, false)
            .unwrap_err()
            .contains("lock unavailable")
    );
    assert!(
        build_generated_assets(&root, &json!({}), &NeverRun)
            .unwrap_err()
            .contains("lock unavailable")
    );
    assert_eq!(fs::read(root.join("managed.txt")).unwrap(), b"old\n");
    let (other, _) = transaction_fixture(true);
    drop(ProjectLock::acquire(&other).unwrap());
    drop(lock);
    apply_plan(plan, &fingerprint, false).unwrap();
    assert!(
        root.join(".runtime/bridgeforge-codex/project-sync.lock")
            .exists()
    );
    fs::remove_dir_all(root).unwrap();
    fs::remove_dir_all(other).unwrap();
}

#[test]
fn project_lock_covers_finalize_rollback_and_releases_on_failure() {
    let (root, plan) = transaction_fixture(true);
    let fingerprint = plan.aggregate_fingerprint.clone();
    let concurrent = plan.clone();
    let observer = |_: &Path, _: &Path| {
        let child = concurrent.clone();
        let result = std::thread::spawn(move || {
            let fingerprint = child.aggregate_fingerprint.clone();
            apply_plan(child, &fingerprint, false)
        })
        .join()
        .unwrap();
        assert!(result.unwrap_err().contains("lock unavailable"));
        Err("injected finalize failure".into())
    };
    assert!(
        apply_plan_internal(plan.clone(), &fingerprint, false, Some(&observer))
            .unwrap_err()
            .contains("rolled back")
    );
    assert_eq!(fs::read(root.join("managed.txt")).unwrap(), b"old\n");
    apply_plan(plan, &fingerprint, false).unwrap();
    fs::remove_dir_all(root).unwrap();
}

fn legacy_bytes() -> Vec<u8> {
    let hash = format!("sha256:{}", "a".repeat(64));
    serde_json::to_vec(
        &json!({"schema_version":1,"generated_asset_id":"codex.hooks",
        "platform":"windows-x86_64","source_tree_sha256":hash,"cargo_lock_sha256":hash,
        "build_recipe_sha256":hash,"self_test_sha256":hash,"binary_sha256":hash,
        "cargo_version":"cargo 1.94.1"}),
    )
    .unwrap()
}

#[test]
fn apply_rollback_preserves_external_receipt_recreated_after_retirement() {
    let (root, mut plan) = transaction_fixture(true);
    let legacy = root.join(LEGACY_RECEIPT);
    fs::create_dir_all(legacy.parent().unwrap()).unwrap();
    fs::write(&legacy, legacy_bytes()).unwrap();
    plan_legacy_receipt_retirement(&root, &mut plan.safe, &mut plan.deletes).unwrap();
    plan.aggregate_fingerprint = plan_fingerprint(
        &plan.mode,
        &plan.current_version,
        &plan.safe,
        &plan.risk,
        &plan.preservation_manifest,
        &plan.generated_source_fingerprints,
    )
    .unwrap();
    let fingerprint = plan.aggregate_fingerprint.clone();
    let observer = |_: &Path, _: &Path| {
        assert!(!legacy.exists());
        fs::write(&legacy, b"external receipt").unwrap();
        Err("injected after external receipt recreation".into())
    };
    let error = apply_plan_internal(plan, &fingerprint, false, Some(&observer)).unwrap_err();
    assert!(error.contains("rollback incomplete"), "{error}");
    assert!(error.contains("external state preserved"), "{error}");
    assert_eq!(fs::read(&legacy).unwrap(), b"external receipt");
    assert_eq!(fs::read(root.join("managed.txt")).unwrap(), b"old\n");
    assert_eq!(
        fs::read(root.join(".codex/.bridgeforge_codex_version")).unwrap(),
        b"8.0.0\n"
    );
    assert!(
        root.join(".runtime/bridgeforge-codex/project-sync.lock")
            .exists()
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn delete_rechecks_exact_original_receipt_bytes_before_removal() {
    let (root, _) = transaction_fixture(true);
    let legacy = root.join(LEGACY_RECEIPT);
    fs::create_dir_all(legacy.parent().unwrap()).unwrap();
    let before = legacy_bytes();
    fs::write(&legacy, b"external receipt").unwrap();
    let error = delete_unchanged_file(&legacy, Some(&before)).unwrap_err();
    assert!(error.contains("changed before removal"));
    assert_eq!(fs::read(&legacy).unwrap(), b"external receipt");
    fs::remove_file(&legacy).unwrap();
    fs::create_dir(&legacy).unwrap();
    let error = delete_unchanged_file(&legacy, Some(&before)).unwrap_err();
    assert!(error.contains("cannot read transaction target"));
    assert!(legacy.is_dir());
    fs::remove_dir_all(root).unwrap();
}

#[cfg(windows)]
#[test]
fn failed_atomic_replace_preserves_old_file_and_allows_transaction_rollback() {
    use std::os::windows::fs::OpenOptionsExt;
    let (root, plan) = transaction_fixture(true);
    let asset = root.join("managed.txt");
    // Allow reads, but deny delete/rename so the real Windows atomic replacement fails.
    let held = fs::OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(&asset)
        .unwrap();
    assert!(atomic_write(&asset, b"replacement\n").is_err());
    assert_eq!(
        transaction_file_state(&asset).unwrap(),
        Some(b"old\n".to_vec())
    );
    let fingerprint = plan.aggregate_fingerprint.clone();
    let error = apply_plan(plan.clone(), &fingerprint, false).unwrap_err();
    assert!(error.contains("transaction rolled back"), "{error}");
    assert!(!error.contains("rollback incomplete"), "{error}");
    assert_eq!(fs::read(&asset).unwrap(), b"old\n");
    assert_eq!(
        fs::read(root.join(".codex/managed-skeleton.json")).unwrap(),
        b"old-contract\n"
    );
    assert_eq!(
        fs::read(root.join(".codex/.bridgeforge_codex_version")).unwrap(),
        b"8.0.0\n"
    );
    drop(held);
    apply_plan(plan, &fingerprint, false).unwrap();
    assert_eq!(fs::read(&asset).unwrap(), b"new\n");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn apply_rollback_preserves_external_edit_to_written_target() {
    let (root, plan) = transaction_fixture(true);
    let fingerprint = plan.aggregate_fingerprint.clone();
    let observer = |root: &Path, _: &Path| {
        fs::write(root.join("managed.txt"), b"external managed edit").unwrap();
        Err("injected external edit".into())
    };
    let error = apply_plan_internal(plan, &fingerprint, false, Some(&observer)).unwrap_err();
    assert!(error.contains("rollback incomplete"));
    assert_eq!(
        fs::read(root.join("managed.txt")).unwrap(),
        b"external managed edit"
    );
    assert_eq!(
        fs::read(root.join(".codex/managed-skeleton.json")).unwrap(),
        b"old-contract\n"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn legacy_receipt_planning_is_read_only_and_apply_retirement_rolls_back() {
    let (root, mut plan) = transaction_fixture(true);
    let legacy = root.join(LEGACY_RECEIPT);
    fs::create_dir_all(legacy.parent().unwrap()).unwrap();
    let original = legacy_bytes();
    fs::write(&legacy, &original).unwrap();
    plan_legacy_receipt_retirement(&root, &mut plan.safe, &mut plan.deletes).unwrap();
    assert_eq!(fs::read(&legacy).unwrap(), original);
    plan.aggregate_fingerprint = plan_fingerprint(
        &plan.mode,
        &plan.current_version,
        &plan.safe,
        &plan.risk,
        &plan.preservation_manifest,
        &plan.generated_source_fingerprints,
    )
    .unwrap();
    let fingerprint = plan.aggregate_fingerprint.clone();
    let observer = |_: &Path, _: &Path| {
        assert!(!legacy.exists());
        Err("injected after receipt retirement".into())
    };
    assert!(
        apply_plan_internal(plan.clone(), &fingerprint, false, Some(&observer))
            .unwrap_err()
            .contains("rolled back")
    );
    assert_eq!(fs::read(&legacy).unwrap(), original);
    apply_plan(plan, &fingerprint, false).unwrap();
    assert!(!legacy.exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn unknown_legacy_receipt_blocks_apply_without_deletion() {
    let (root, plan) = transaction_fixture(true);
    let legacy = root.join(LEGACY_RECEIPT);
    fs::create_dir_all(legacy.parent().unwrap()).unwrap();
    fs::write(&legacy, b"{\"schema_version\":99}").unwrap();
    let fingerprint = plan.aggregate_fingerprint.clone();
    assert!(
        apply_plan(plan, &fingerprint, false)
            .unwrap_err()
            .contains("unknown legacy")
    );
    assert_eq!(fs::read(root.join("managed.txt")).unwrap(), b"old\n");
    assert_eq!(fs::read(&legacy).unwrap(), b"{\"schema_version\":99}");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn source_retirement_rechecks_absence_and_content_and_rolls_back_deleted_files() {
    let (root, mut plan) = transaction_fixture(true);
    let known = ".codex/hooks/crates/old/src/lib.rs";
    let absent = ".codex/hooks/crates/old/src/late.rs";
    fs::create_dir_all(root.join(known).parent().unwrap()).unwrap();
    fs::write(root.join(known), b"published\n").unwrap();
    let retire = json!({"retired_assets":[
        {"id":"old.lib","target":known,"known_hashes":[sha_git(b"published\n")]},
        {"id":"old.late","target":absent,"known_hashes":[sha_git(b"late\n")]}
    ]});
    source_retirement::plan(
        &root,
        &retire,
        &mut plan.safe,
        &mut plan.deletes,
        &mut plan.gaps,
        &mut plan.project_hook_reads,
        plan.preservation_manifest["entries"]
            .as_array_mut()
            .unwrap(),
    )
    .unwrap();
    let contract_path = root.join(".codex/managed-skeleton.json");
    let mut contract: Value = serde_json::from_slice(&plan.writes[&contract_path]).unwrap();
    contract["retired_assets"] = retire["retired_assets"].clone();
    plan.writes
        .insert(contract_path, serde_json::to_vec(&contract).unwrap());
    plan.aggregate_fingerprint = plan_fingerprint(
        &plan.mode,
        &plan.current_version,
        &plan.safe,
        &plan.risk,
        &plan.preservation_manifest,
        &plan.generated_source_fingerprints,
    )
    .unwrap();
    let fp = plan.aggregate_fingerprint.clone();
    let observer = |root: &Path, _: &Path| {
        assert!(!root.join(known).exists());
        Err("injected after source retirement".into())
    };
    assert!(
        apply_plan_internal(plan.clone(), &fp, false, Some(&observer))
            .unwrap_err()
            .contains("rolled back")
    );
    assert_eq!(fs::read(root.join(known)).unwrap(), b"published\n");
    assert_eq!(fs::read(root.join("managed.txt")).unwrap(), b"old\n");
    fs::write(root.join(known), b"external").unwrap();
    assert!(
        apply_plan(plan.clone(), &fp, false)
            .unwrap_err()
            .contains("changed after plan")
    );
    assert_eq!(fs::read(root.join(known)).unwrap(), b"external");
    fs::write(root.join(known), b"published\n").unwrap();
    fs::write(root.join(absent), b"late\n").unwrap();
    assert!(
        apply_plan(plan.clone(), &fp, false)
            .unwrap_err()
            .contains("changed after plan")
    );
    assert_eq!(fs::read(root.join(absent)).unwrap(), b"late\n");
    fs::remove_file(root.join(absent)).unwrap();
    apply_plan(plan, &fp, false).unwrap();
    assert!(!root.join(known).exists());
    assert_eq!(fs::read(root.join("managed.txt")).unwrap(), b"new\n");
    fs::remove_dir_all(root).unwrap();
}

fn transaction_fixture(valid_asset_hash: bool) -> (PathBuf, SyncPlan) {
    let token = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = test_artifacts::root(env!("CARGO_MANIFEST_DIR"), "project-sync").join(format!(
        "bridgeforge-project-sync-{}-{token}",
        std::process::id()
    ));
    let contract_path = root.join(".codex/managed-skeleton.json");
    let stamp_path = root.join(".codex/.bridgeforge_codex_version");
    let asset_path = root.join("managed.txt");
    fs::create_dir_all(root.join(".codex")).unwrap();
    fs::write(&contract_path, b"old-contract\n").unwrap();
    fs::write(&stamp_path, b"8.0.0\n").unwrap();
    fs::write(&asset_path, b"old\n").unwrap();
    let current_sha = if valid_asset_hash {
        sha_git(b"new\n")
    } else {
        sha_git(b"different\n")
    };
    let contract = serde_json::to_vec_pretty(&json!({
        "schema_version": 4,
        "release_version": "8.1.0",
        "host": "codex",
        "stamp": ".codex/.bridgeforge_codex_version",
        "contract_target": ".codex/managed-skeleton.json",
        "assets": [{
            "id": "managed.asset",
            "source": "templates/managed.txt",
            "target": "managed.txt",
            "strategy": "whole",
            "current_sha256": current_sha,
        }],
        "baseline_model": "current-only", "compatibility_baseline": "8.0.0",
        "generated_assets": [],
    }))
    .unwrap();
    let action = SyncAction {
        id: "managed.asset".into(),
        target: "managed.txt".into(),
        operation: "replace".into(),
        risk: false,
        before_sha256: Some(sha_git(b"old\n")),
        after_sha256: sha_git(b"new\n"),
    };
    let safe = vec![action];
    let preservation_manifest = json!({"entries": []});
    let generated_source_fingerprints = BTreeMap::new();
    let fingerprint = plan_fingerprint(
        &SyncMode::Update,
        "8.1.0",
        &safe,
        &[],
        &preservation_manifest,
        &generated_source_fingerprints,
    )
    .unwrap();
    let plan = SyncPlan {
        build_metrics: Default::default(),
        schema: 1,
        status: "planned".into(),
        readiness: "ready".into(),
        release_setup: json!({"status":"not-applicable"}),
        mode: SyncMode::Update,
        previous_version: Some("8.0.0".into()),
        current_version: "8.1.0".into(),
        safe,
        risk: Vec::new(),
        gaps: Vec::new(),
        blockers: Vec::new(),
        preservation_manifest,
        confirmation_required: false,
        aggregate_fingerprint: fingerprint,
        public_installation_required: false,
        hook_migration: Value::Null,
        project_root: root.clone(),
        writes: BTreeMap::from([
            (asset_path, b"new\n".to_vec()),
            (contract_path, contract),
            (stamp_path, b"8.1.0\n".to_vec()),
        ]),
        deletes: Vec::new(),
        generated_source_fingerprints,
        project_hook_inputs: Vec::new(),
        project_hook_reads: BTreeMap::new(),
        public_actions: Vec::new(),
        release_upgrade: None,
    };
    (root, plan)
}

#[test]
fn failed_prospective_check_restores_project_hook_payloads_and_retired_source() {
    let (root, mut plan) = transaction_fixture(false);
    let old = root.join(".codex/hooks/project_old/entrypoint.py");
    fs::create_dir_all(old.parent().unwrap()).unwrap();
    fs::write(&old, b"legacy source").unwrap();
    let hooks = root.join(".codex/hooks.json");
    fs::write(&hooks, b"{\"hooks\":{}}").unwrap();
    let original_hooks = fs::read(&hooks).unwrap();
    for (target, payload) in [
        (
            ".codex/hooks/project_demo/entrypoint.rs",
            b"new rust".as_slice(),
        ),
        (
            ".codex/hooks.json",
            b"{\"hooks\":{},\"changed\":true}".as_slice(),
        ),
        (".codex/bin/project_demo.exe", b"binary".as_slice()),
        (
            ".codex/bin/build-receipt-project-demo.json",
            b"receipt".as_slice(),
        ),
    ] {
        let path = root.join(target);
        plan.safe.push(SyncAction {
            id: format!("project-hook:fixture:{target}"),
            target: target.into(),
            operation: "write".into(),
            risk: false,
            before_sha256: fs::read(&path).ok().as_deref().map(sha_git),
            after_sha256: sha_git(payload),
        });
        plan.writes.insert(path, payload.to_vec());
    }
    plan.deletes.push(old.clone());
    plan.safe.push(SyncAction {
        id: "retired:fixture".into(),
        target: ".codex/hooks/project_old/entrypoint.py".into(),
        operation: "delete".into(),
        risk: false,
        before_sha256: Some(sha_git(b"legacy source")),
        after_sha256: "sha256:deleted".into(),
    });
    plan.aggregate_fingerprint = plan_fingerprint(
        &plan.mode,
        &plan.current_version,
        &plan.safe,
        &plan.risk,
        &plan.preservation_manifest,
        &plan.generated_source_fingerprints,
    )
    .unwrap();
    assert!(
        apply_plan(plan.clone(), &plan.aggregate_fingerprint, true)
            .unwrap_err()
            .contains("rolled back")
    );
    assert_eq!(fs::read(old).unwrap(), b"legacy source");
    assert_eq!(fs::read(hooks).unwrap(), original_hooks);
    for target in [
        ".codex/hooks/project_demo/entrypoint.rs",
        ".codex/bin/project_demo.exe",
        ".codex/bin/build-receipt-project-demo.json",
    ] {
        assert!(!root.join(target).exists());
    }
    assert_eq!(
        fs::read(root.join(".codex/.bridgeforge_codex_version")).unwrap(),
        b"8.0.0\n"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn project_hook_registration_serialization_is_stable_on_next_merge() {
    let (root, _) = transaction_fixture(true);
    fs::create_dir_all(root.join(".codex/hooks/project_demo")).unwrap();
    fs::write(
        root.join(".codex/hooks/project_demo/entrypoint.rs"),
        b"pub fn run(_:Vec<String>)->i32{0}",
    )
    .unwrap();
    let config = json!({"hooks":{},"bridgeforgeProjectHooks":{"schema_version":1,"hooks":[{"id":"demo","events":[{"event":"Stop"}]}]}});
    fs::write(
        root.join(".codex/hooks.json"),
        serde_json::to_vec(&config).unwrap(),
    )
    .unwrap();
    let contract =
        json!({"generated_assets":[{"source_tree_sha256":"source","lockfile_sha256":"lock"}]});
    let mut writes = BTreeMap::new();
    plan_project_hooks(
        &root,
        &contract,
        &mut writes,
        &[],
        &mut Vec::new(),
        &mut Vec::new(),
        &mut BTreeMap::new(),
    )
    .unwrap();
    let payload = writes.get(&root.join(".codex/hooks.json")).unwrap();
    assert_eq!(
        content::render(
            "fixture",
            b"{\"hooks\":{}}",
            Some(payload),
            &json!({"merge_policy":"codex-hooks"}),
            &root,
            false,
        )
        .unwrap(),
        *payload,
        "first install must already use the stable hook JSON encoding"
    );
    let native: Value = serde_json::from_slice(payload).unwrap();
    assert!(
        native
            .as_object()
            .unwrap()
            .keys()
            .all(|key| matches!(key.as_str(), "description" | "hooks"))
    );
    let registry = writes
        .get(&root.join(crate::project_hooks::REGISTRY_PATH))
        .unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(registry).unwrap(),
        config["bridgeforgeProjectHooks"]
    );
    for (path, payload) in &writes {
        fs::write(path, payload).unwrap();
    }
    let mut next_writes = BTreeMap::new();
    let (_, reads) = plan_project_hooks(
        &root,
        &contract,
        &mut next_writes,
        &[],
        &mut Vec::new(),
        &mut Vec::new(),
        &mut BTreeMap::new(),
    )
    .unwrap();
    assert!(
        next_writes.is_empty(),
        "canonical registry must not be rewritten"
    );
    assert!(reads.contains_key(crate::project_hooks::REGISTRY_PATH));
    fs::write(
        root.join(".codex/hooks.json"),
        serde_json::to_vec(&config).unwrap(),
    )
    .unwrap();
    fs::write(
        root.join(crate::project_hooks::REGISTRY_PATH),
        br#"{"schema_version":1,"hooks":[]}"#,
    )
    .unwrap();
    assert!(
        plan_project_hooks(
            &root,
            &contract,
            &mut BTreeMap::new(),
            &[],
            &mut Vec::new(),
            &mut Vec::new(),
            &mut BTreeMap::new()
        )
        .unwrap_err()
        .contains("conflicting")
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn project_hook_registry_migration_is_atomic_and_rejects_input_drift() {
    let (root, mut plan) = transaction_fixture(true);
    let original=br#"{"description":"keep","hooks":{},"bridgeforgeProjectHooks":{"schema_version":1,"hooks":[]}}"#;
    fs::write(root.join(".codex/hooks.json"), original).unwrap();
    let (inputs, reads) = plan_project_hooks(
        &root,
        &json!({}),
        &mut plan.writes,
        &[],
        &mut plan.safe,
        &mut plan.risk,
        &mut plan.generated_source_fingerprints,
    )
    .unwrap();
    plan.project_hook_inputs = inputs;
    plan.project_hook_reads = reads;
    plan.confirmation_required = true;
    plan.aggregate_fingerprint = plan_fingerprint(
        &plan.mode,
        &plan.current_version,
        &plan.safe,
        &plan.risk,
        &plan.preservation_manifest,
        &plan.generated_source_fingerprints,
    )
    .unwrap();
    let fingerprint = plan.aggregate_fingerprint.clone();
    assert!(apply_plan(plan.clone(), &fingerprint, false).is_err());
    let registry_path = root.join(crate::project_hooks::REGISTRY_PATH);
    assert!(!registry_path.exists());
    fs::write(&registry_path, b"unexpected").unwrap();
    assert!(
        apply_plan(plan.clone(), &fingerprint, true)
            .unwrap_err()
            .contains("input changed after plan")
    );
    fs::remove_file(&registry_path).unwrap();
    let observer = |_: &Path, _: &Path| {
        assert!(registry_path.is_file());
        assert!(
            serde_json::from_slice::<Value>(&fs::read(root.join(".codex/hooks.json")).unwrap())
                .unwrap()
                .get("bridgeforgeProjectHooks")
                .is_none()
        );
        Err("injected after registry migration".into())
    };
    assert!(
        apply_plan_internal(plan.clone(), &fingerprint, true, Some(&observer))
            .unwrap_err()
            .contains("rolled back")
    );
    assert!(!registry_path.exists());
    assert_eq!(fs::read(root.join(".codex/hooks.json")).unwrap(), original);
    apply_plan(plan, &fingerprint, true).unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&fs::read(registry_path).unwrap()).unwrap(),
        json!({"schema_version":1,"hooks":[]})
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn project_hook_package_plan_tracks_dependency_files_and_rejects_inventory_drift() {
    let (root, mut plan) = transaction_fixture(true);
    let directory = root.join(".codex/hooks/project_demo");
    fs::create_dir_all(&directory).unwrap();
    for (path, bytes) in [
        ("entrypoint.rs", "pub fn run(_:Vec<String>)->i32{0}"),
        (
            "Cargo.toml",
            "[package]\nname=\"demo\"\nversion=\"0.1.0\"\n",
        ),
        ("Cargo.lock", "version=4\n"),
        ("helper.rs", "const VALUE:i32=1;"),
    ] {
        fs::write(directory.join(path), bytes).unwrap();
    }
    fs::write(root.join(".codex/hooks.json"), br#"{"hooks":{},"bridgeforgeProjectHooks":{"schema_version":1,"hooks":[{"id":"demo","events":[{"event":"Stop"}]}]}}"#).unwrap();
    let (inputs, reads) = plan_project_hooks(
        &root,
        &json!({}),
        &mut plan.writes,
        &[],
        &mut plan.safe,
        &mut plan.risk,
        &mut plan.generated_source_fingerprints,
    )
    .unwrap();
    assert!(reads.contains_key(".codex/hooks/project_demo/Cargo.lock"));
    assert!(
        inputs[0]
            .1
            .package
            .as_ref()
            .unwrap()
            .contains_key("helper.rs")
    );
    plan.project_hook_reads = reads;
    verify_project_hook_reads(&plan).unwrap();
    let before = plan_fingerprint(
        &plan.mode,
        &plan.current_version,
        &[],
        &[],
        &json!({}),
        &plan.generated_source_fingerprints,
    )
    .unwrap();
    let mut changed = plan.generated_source_fingerprints.clone();
    changed.insert("project-hook:input:demo".into(), "changed".into());
    assert_ne!(
        before,
        plan_fingerprint(
            &plan.mode,
            &plan.current_version,
            &[],
            &[],
            &json!({}),
            &changed
        )
        .unwrap()
    );
    fs::write(directory.join("new.rs"), "new").unwrap();
    assert!(
        verify_project_hook_reads(&plan)
            .unwrap_err()
            .contains("inventory")
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn stamp_is_last_and_finalize_failure_rolls_back_every_write() {
    let (root, plan) = transaction_fixture(true);
    let fingerprint = plan.aggregate_fingerprint.clone();
    let observer = |project_root: &Path, stamp_path: &Path| {
        assert_eq!(
            fs::read(project_root.join("managed.txt")).unwrap(),
            b"new\n"
        );
        assert_eq!(fs::read(stamp_path).unwrap(), b"8.0.0\n");
        Err("injected failure before version stamp".into())
    };
    let error = apply_plan_internal(plan, &fingerprint, false, Some(&observer)).unwrap_err();
    assert!(error.contains("transaction rolled back"));
    assert_eq!(fs::read(root.join("managed.txt")).unwrap(), b"old\n");
    assert_eq!(
        fs::read(root.join(".codex/managed-skeleton.json")).unwrap(),
        b"old-contract\n"
    );
    assert_eq!(
        fs::read(root.join(".codex/.bridgeforge_codex_version")).unwrap(),
        b"8.0.0\n"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn prospective_baseline_failure_rolls_back_before_stamp() {
    let (root, plan) = transaction_fixture(false);
    let fingerprint = plan.aggregate_fingerprint.clone();
    let error = apply_plan(plan, &fingerprint, false).unwrap_err();
    assert!(error.contains("managed asset drifted"));
    assert_eq!(fs::read(root.join("managed.txt")).unwrap(), b"old\n");
    assert_eq!(
        fs::read(root.join(".codex/managed-skeleton.json")).unwrap(),
        b"old-contract\n"
    );
    assert_eq!(
        fs::read(root.join(".codex/.bridgeforge_codex_version")).unwrap(),
        b"8.0.0\n"
    );
    fs::remove_dir_all(root).unwrap();
}

fn recorded_transaction_fixture() -> (PathBuf, SyncPlan) {
    let (root, mut plan) = transaction_fixture(true);
    let path = root.join(".codex/managed-skeleton.json");
    let mut contract: Value = serde_json::from_slice(&plan.writes[&path]).unwrap();
    for (id, target, bytes) in [
        (
            "codex.project-hook-policy",
            crate::project_hook_migration::POLICY,
            b"{}".as_slice(),
        ),
        ("unchanged.asset", "unchanged.txt", b"retained".as_slice()),
    ] {
        fs::write(root.join(target), bytes).unwrap();
        contract["assets"].as_array_mut().unwrap().push(json!({
            "id":id,"source":format!("templates/{target}"),"target":target,
            "strategy":"whole","current_sha256":sha_git(bytes)
        }));
    }
    plan.writes
        .insert(path, serde_json::to_vec(&contract).unwrap());
    (root, plan)
}

#[test]
fn recorded_transaction_preserves_external_stamp_and_rolls_back_own_writes() {
    let (root, plan) = recorded_transaction_fixture();
    let fingerprint = plan.aggregate_fingerprint.clone();
    let observer = |_: &Path, stamp: &Path| {
        fs::write(stamp, b"external stamp").unwrap();
        Ok(())
    };
    let error = apply_plan_internal(plan, &fingerprint, false, Some(&observer)).unwrap_err();
    assert!(error.contains("target changed before write"), "{error}");
    assert!(error.contains("rollback incomplete"), "{error}");
    assert_eq!(fs::read(root.join("managed.txt")).unwrap(), b"old\n");
    assert_eq!(
        fs::read(root.join(".codex/.bridgeforge_codex_version")).unwrap(),
        b"external stamp"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn recorded_evidence_detects_post_validation_edits_and_preserves_external_bytes() {
    for target in ["unchanged.txt", "managed.txt"] {
        let (root, plan) = recorded_transaction_fixture();
        let fingerprint = plan.aggregate_fingerprint.clone();
        let observer = |root: &Path, _: &Path| {
            fs::write(root.join(target), b"external edit").unwrap();
            Ok(())
        };
        let error = apply_plan_internal(plan, &fingerprint, false, Some(&observer)).unwrap_err();
        let expected = if target == "managed.txt" {
            "installed payload changed during transaction"
        } else {
            "installed asset changed after verification"
        };
        assert!(error.contains(expected), "{error}");
        assert_eq!(fs::read(root.join(target)).unwrap(), b"external edit");
        assert_eq!(
            fs::read(root.join(".codex/.bridgeforge_codex_version")).unwrap(),
            b"8.0.0\n"
        );
        assert_eq!(
            fs::read(root.join(".codex/managed-skeleton.json")).unwrap(),
            b"old-contract\n"
        );
        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn write_guard_rejects_changed_missing_and_replaced_targets() {
    let (root, _) = transaction_fixture(true);
    let target = root.join("pending.txt");
    fs::write(&target, b"external").unwrap();
    assert!(write_unchanged_file(&target, None, b"ours").is_err());
    assert!(write_unchanged_file(&target, Some(b"original"), b"ours").is_err());
    assert_eq!(fs::read(&target).unwrap(), b"external");
    fs::remove_file(&target).unwrap();
    fs::create_dir(&target).unwrap();
    assert!(write_unchanged_file(&target, Some(b"external"), b"ours").is_err());
    assert!(target.is_dir());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn recorded_transaction_rolls_back_public_payload_with_project_configuration() {
    let (root, mut plan) = recorded_transaction_fixture();
    let relative = ".codex/bin/bridgeforge.exe";
    let target = root.join(relative);
    fs::create_dir_all(target.parent().unwrap()).unwrap();
    fs::write(&target, b"previous public payload").unwrap();
    plan.writes
        .insert(target.clone(), b"prepared public payload".to_vec());
    plan.public_actions.push(SyncAction {
        id: "public:bridgeforge".into(),
        target: relative.into(),
        operation: "replace".into(),
        risk: false,
        before_sha256: Some(sha_raw(b"previous public payload")),
        after_sha256: sha_raw(b"prepared public payload"),
    });
    let fingerprint = plan.aggregate_fingerprint.clone();
    let observer = |root: &Path, _: &Path| {
        assert_eq!(
            fs::read(root.join(relative)).unwrap(),
            b"prepared public payload"
        );
        assert_eq!(fs::read(root.join("managed.txt")).unwrap(), b"new\n");
        Err("injected after joint installation".into())
    };
    let error = apply_plan_internal(plan, &fingerprint, false, Some(&observer)).unwrap_err();
    assert!(error.contains("transaction rolled back"), "{error}");
    assert_eq!(fs::read(target).unwrap(), b"previous public payload");
    assert_eq!(fs::read(root.join("managed.txt")).unwrap(), b"old\n");
    assert_eq!(
        fs::read(root.join(".codex/.bridgeforge_codex_version")).unwrap(),
        b"8.0.0\n"
    );
    fs::remove_dir_all(root).unwrap();
}

#[allow(dead_code)]
#[path = "../support/artifact_paths.rs"]
mod test_artifacts;
