use super::*;
use std::time::{SystemTime, UNIX_EPOCH};

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
        b"invalid legacy contract",
    )
    .unwrap();
    fs::write(factory.join("templates/managed.txt"), b"new\n").unwrap();
    let contract = json!({
        "schema_version":4,"release_version":release,"compatibility_baseline":"1.8.6",
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
fn fixed_baseline_does_not_move_with_the_release() {
    for (previous, release, rebuild) in [
        ("1.0.0", "1.8.6", true),
        ("1.8.5", "1.8.6", true),
        ("1.8.6", "1.8.6", false),
        ("1.8.6", "1.8.7", false),
        ("1.8.7", "1.9.0", false),
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
            "1.8.6"
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
fn fixed_baseline_rejects_missing_invalid_future_floor_and_downgrade() {
    let (root, project, factory) = upgrade_fixture("1.8.8", "1.8.7");
    assert!(
        build_plan(&project, &factory, SyncMode::Update)
            .unwrap_err()
            .contains("newer than")
    );
    let path = factory.join("templates/managed-skeleton.json");
    let original: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    for floor in [Value::Null, json!("invalid"), json!("2.0.0")] {
        let mut contract = original.clone();
        contract["compatibility_baseline"] = floor;
        fs::write(&path, serde_json::to_vec(&contract).unwrap()).unwrap();
        assert!(build_plan(&project, &factory, SyncMode::Update).is_err());
        assert!(!project.join(".runtime").exists());
    }
    fs::remove_dir_all(root).unwrap();
}

fn confirmed_sources(project: &Path) -> Value {
    let sources = crate::asset_migration::scan_sources(project).unwrap();
    json!({"schema_version":1,"sources":sources.iter().enumerate().map(|(index, source)| {
        json!({"asset_id":source.asset_id,"source_path":source.source_path,
            "source_sha256":source.source_sha256,"kind":source.kind,"confirmed":true,
            "retire_source":true,"retirement_reason":"migrated",
            "decisions":[{"target":format!(".codex/rules/retained-{index}.rules"),
                "asset_type":"command-rule","reason":"user confirmed exact content",
                "target_before_sha256":null,"content_utf8":fs::read_to_string(project.join(&source.source_path)).unwrap()}],
            "discarded":[]})
    }).collect::<Vec<_>>()})
}

#[test]
fn every_legacy_source_requires_confirmation_on_both_upgrade_routes() {
    for previous in ["1.8.5", "1.8.6"] {
        let (root, project, factory) = upgrade_fixture(previous, "1.8.7");
        fs::create_dir_all(project.join(".codex/memory")).unwrap();
        fs::create_dir_all(project.join(".codex/rules")).unwrap();
        fs::write(
            project.join(".codex/memory/one.md"),
            b"memory knowledge\r\n",
        )
        .unwrap();
        fs::write(project.join(".codex/rules/two.md"), b"rule knowledge\r\n").unwrap();
        let before = tree_sha(&project).unwrap();
        let plan = build_plan(&project, &factory, SyncMode::Update).unwrap();
        assert_eq!(plan.asset_migration["source_count"], 2);
        assert!(
            apply_plan(plan.clone(), &plan.aggregate_fingerprint, true)
                .unwrap_err()
                .contains("gap")
        );
        assert_eq!(tree_sha(&project).unwrap(), before);
        assert!(!project.join(".runtime").exists());
        let manifest = confirmed_sources(&project);
        let mut partial = manifest.clone();
        partial["sources"].as_array_mut().unwrap().pop();
        assert!(
            build_plan_with_inputs(&project, &factory, SyncMode::Update, Some(&partial), None)
                .is_err()
        );
        let mut unconfirmed = manifest.clone();
        unconfirmed["sources"][0]["confirmed"] = json!(false);
        assert!(
            build_plan_with_inputs(
                &project,
                &factory,
                SyncMode::Update,
                Some(&unconfirmed),
                None
            )
            .is_err()
        );
        assert_eq!(tree_sha(&project).unwrap(), before);
        let plan =
            build_plan_with_inputs(&project, &factory, SyncMode::Update, Some(&manifest), None)
                .unwrap();
        let fail = |_: &Path, _: &Path| Err("injected migration failure".into());
        assert!(
            apply_plan_internal(plan.clone(), &plan.aggregate_fingerprint, true, Some(&fail))
                .unwrap_err()
                .contains("rolled back")
        );
        assert_eq!(
            fs::read(project.join(".codex/memory/one.md")).unwrap(),
            b"memory knowledge\r\n"
        );
        assert_eq!(
            fs::read(project.join(".codex/rules/two.md")).unwrap(),
            b"rule knowledge\r\n"
        );
        assert!(!project.join(".codex/rules/retained-0.rules").exists());
        apply_plan(plan.clone(), &plan.aggregate_fingerprint, true).unwrap();
        assert!(!project.join(".codex/memory/one.md").exists());
        assert!(!project.join(".codex/rules/two.md").exists());
        assert_eq!(
            fs::read(project.join(".codex/rules/retained-0.rules")).unwrap(),
            b"memory knowledge\r\n"
        );
        assert_eq!(
            fs::read(project.join(".codex/rules/retained-1.rules")).unwrap(),
            b"rule knowledge\r\n"
        );
        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn new_or_changed_legacy_sources_invalidate_apply_without_writing() {
    for previous in ["1.8.5", "1.8.6"] {
        let (root, project, factory) = upgrade_fixture(previous, "1.8.7");
        let plan = build_plan(&project, &factory, SyncMode::Update).unwrap();
        fs::create_dir_all(project.join(".codex/memory")).unwrap();
        let source = project.join(".codex/memory/one.md");
        fs::write(&source, b"knowledge").unwrap();
        let before = tree_sha(&project).unwrap();
        assert!(
            apply_plan(plan.clone(), &plan.aggregate_fingerprint, true)
                .unwrap_err()
                .contains("sources or confirmations drifted")
        );
        assert_eq!(tree_sha(&project).unwrap(), before);
        let manifest = confirmed_sources(&project);
        let plan =
            build_plan_with_inputs(&project, &factory, SyncMode::Update, Some(&manifest), None)
                .unwrap();
        fs::write(&source, b"changed knowledge").unwrap();
        let before = tree_sha(&project).unwrap();
        assert!(
            build_plan_with_inputs(&project, &factory, SyncMode::Update, Some(&manifest), None)
                .is_err()
        );
        assert!(apply_plan(plan.clone(), &plan.aggregate_fingerprint, true).is_err());
        assert_eq!(tree_sha(&project).unwrap(), before);
        assert!(!project.join(".runtime").exists());
        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn markdown_upgrade_inserts_sections_and_preserves_project_content() {
    let source = b"# Docs\n\n## Start\n| Task | Read |\n|---|---|\n| arch | new |\n\n## Lifecycle\ncanonical\n\n## Index\n| File | Purpose |\n|---|---|\n| managed | new |\n\n";
    let current = b"# Custom\r\ndelivery_layout: milestone\r\n\r\n## Project\r\nkeep bytes\r\n\r\n## Index\r\n| Old | Header |\r\n|---|---|\r\n| custom | personal |\r\n| managed | old |\r\n\r\n";
    let asset = json!({"managed_blocks":{"headings":["## Lifecycle"],"keyed_tables":[
        {"heading":"## Start","managed_keys":["arch"]},{"heading":"## Index","managed_keys":["managed"]}]}});
    let merged =
        merge_managed_markdown(source, current, &asset, Path::new("Example"), true).unwrap();
    let text = String::from_utf8(merged.clone()).unwrap();
    assert!(text.contains("## Project\r\nkeep bytes\r\n\r\n"));
    assert!(text.contains("| custom | personal |\r\n"));
    assert!(text.contains("delivery_layout: milestone\r\n"));
    assert!(text.find("## Start").unwrap() < text.find("## Lifecycle").unwrap());
    assert!(text.find("## Lifecycle").unwrap() < text.find("## Index").unwrap());
    assert_eq!(
        merge_managed_markdown(source, &merged, &asset, Path::new("Example"), false).unwrap(),
        merged
    );
    assert!(merge_managed_markdown(source, current, &asset, Path::new("Example"), false).is_err());
    let duplicate = format!("{text}\n## Lifecycle\nambiguous");
    assert!(
        merge_managed_markdown(
            source,
            duplicate.as_bytes(),
            &asset,
            Path::new("Example"),
            true
        )
        .is_err()
    );
    let changed_columns = text.replace(
        "| File | Purpose |\n|---|---|",
        "| File | Purpose | Extra |\n|---|---|---|",
    );
    assert!(
        merge_managed_markdown(
            source,
            changed_columns.as_bytes(),
            &asset,
            Path::new("Example"),
            true
        )
        .is_err()
    );
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
    let (root, project, factory) = upgrade_fixture("1.5.8", "1.8.7");
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
        build_plan_with_inputs(&project, &factory, SyncMode::Update, None, Some(&choices)).unwrap();
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
        b"1.5.8\n",
    )
    .unwrap();
    let choices = json!({"preserve":[bundle_id],"delete":file_ids});
    let plan =
        build_plan_with_inputs(&project, &factory, SyncMode::Update, None, Some(&choices)).unwrap();
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
        "1.8.7\n"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn project_prefixed_file_retirement_removes_only_its_hook_registration_atomically() {
    let (root, project, factory) = upgrade_fixture("1.5.8", "1.8.7");
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .unwrap();
    let current: Value = serde_json::from_slice(
        &fs::read(repository.join("templates/managed-skeleton.json")).unwrap(),
    )
    .unwrap();
    let asset = current["assets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|asset| asset["target"] == ".codex/hooks.json")
        .unwrap()
        .clone();
    let source_path = asset["source"].as_str().unwrap();
    let source = fs::read(repository.join(source_path)).unwrap();
    fs::write(factory.join(source_path), &source).unwrap();
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
        build_plan_with_inputs(&project, &factory, SyncMode::Update, None, Some(&choices)).unwrap();
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
    let (root, project, factory) = upgrade_fixture("1.5.8", "1.8.7");
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
    assert!(crate::memory::is_link_or_reparse(&link).unwrap());
    let plan = build_plan(&project, &factory, SyncMode::Update).unwrap();
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
    let (root, project, factory) = upgrade_fixture("1.5.8", "1.8.6");
    let bundle = project.join(".codex/hooks/project_business");
    fs::create_dir_all(&bundle).unwrap();
    fs::write(bundle.join("entrypoint.py"), b"project owned Python\r\n").unwrap();
    let plan = build_plan(&project, &factory, SyncMode::Update).unwrap();
    assert!(plan.blockers.is_empty(), "{:?}", plan.blockers);
    assert!(!plan.gaps.is_empty());
    let decisions = json!({"preserve":["P:project-hook-bundle:.codex/hooks/project_business"]});
    let plan = build_plan_with_inputs(&project, &factory, SyncMode::Update, None, Some(&decisions))
        .unwrap();
    assert!(plan.gaps.is_empty());
    fs::write(bundle.join("late.txt"), b"concurrent content").unwrap();
    assert!(
        apply_plan(plan.clone(), &plan.aggregate_fingerprint, true)
            .unwrap_err()
            .contains("preserved project asset drifted")
    );
    let plan = build_plan_with_inputs(&project, &factory, SyncMode::Update, None, Some(&decisions))
        .unwrap();
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
fn real_template_upgrades_legacy_markdown_without_old_contract_or_losing_index() {
    let factory = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .unwrap();
    let contract: Value =
        serde_json::from_slice(&fs::read(factory.join("templates/managed-skeleton.json")).unwrap())
            .unwrap();
    let asset = contract["assets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["id"] == "codex.doc.readme")
        .unwrap();
    let source = fs::read(factory.join("templates/doc/README.md")).unwrap();
    let mut old = String::from_utf8(source.clone()).unwrap();
    for heading in ["## 从这里开始", "## 文档生命周期"] {
        let (start, end) = markdown_section(&old, heading).unwrap();
        old.replace_range(start..end, "");
    }
    old = old.replace("| 文件 | 说明 |", "| 旧标题 | 项目说明 |");
    old.push_str("\n## 项目自有\n不得丢失\n");
    let merged =
        merge_managed_markdown(&source, old.as_bytes(), asset, Path::new("Example"), true).unwrap();
    assert!(String::from_utf8_lossy(&merged).contains("## 项目自有\n不得丢失\n"));
    crate::baseline::verify_asset_payload(asset, &merged).unwrap();
    assert_eq!(
        merge_managed_markdown(&source, &merged, asset, Path::new("Example"), false).unwrap(),
        merged
    );
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
        b"1.0.0\n"
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
        b"1.0.0\n"
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

fn transaction_fixture(valid_asset_hash: bool) -> (PathBuf, SyncPlan) {
    let token = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "bridgeforge-project-sync-{}-{token}",
        std::process::id()
    ));
    let contract_path = root.join(".codex/managed-skeleton.json");
    let stamp_path = root.join(".codex/.bridgeforge_codex_version");
    let asset_path = root.join("managed.txt");
    fs::create_dir_all(root.join(".codex")).unwrap();
    fs::write(&contract_path, b"old-contract\n").unwrap();
    fs::write(&stamp_path, b"1.0.0\n").unwrap();
    fs::write(&asset_path, b"old\n").unwrap();
    let current_sha = if valid_asset_hash {
        sha_git(b"new\n")
    } else {
        sha_git(b"different\n")
    };
    let contract = serde_json::to_vec_pretty(&json!({
        "schema_version": 4,
        "release_version": "2.0.0",
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
        "baseline_model": "current-only", "compatibility_baseline": "1.0.0",
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
        "2.0.0",
        &safe,
        &[],
        &preservation_manifest,
        &generated_source_fingerprints,
    )
    .unwrap();
    let plan = SyncPlan {
        schema: 1,
        status: "planned".into(),
        readiness: "ready".into(),
        mode: SyncMode::Update,
        previous_version: Some("1.0.0".into()),
        current_version: "2.0.0".into(),
        safe,
        risk: Vec::new(),
        gaps: Vec::new(),
        blockers: Vec::new(),
        asset_migration: json!({}),
        preservation_manifest,
        confirmation_required: false,
        aggregate_fingerprint: fingerprint,
        project_root: root.clone(),
        writes: BTreeMap::from([
            (asset_path, b"new\n".to_vec()),
            (contract_path, contract),
            (stamp_path, b"2.0.0\n".to_vec()),
        ]),
        deletes: Vec::new(),
        generated_source_fingerprints,
    };
    (root, plan)
}

#[test]
fn json_merge_preserves_external_keys_and_adds_required_values() {
    let required = json!({"permissions": {"allow": ["Read"], "mode": "safe"}});
    let mut actual = json!({"permissions": {"allow": ["Custom"], "extra": true}});
    merge_json(&required, &mut actual);
    assert_eq!(actual["permissions"]["extra"], true);
    assert_eq!(actual["permissions"]["mode"], "safe");
    assert_eq!(actual["permissions"]["allow"], json!(["Custom", "Read"]));
}

#[test]
fn marker_replace_preserves_project_tail() {
    let current = b"x\nBEGIN\nold\nEND\ntail\n";
    let source = b"BEGIN\nnew\nEND\n";
    assert_eq!(
        marker_replace(current, source, "BEGIN", "END").unwrap(),
        b"x\nBEGIN\nnew\nEND\ntail\n"
    );
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
        assert_eq!(fs::read(stamp_path).unwrap(), b"1.0.0\n");
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
        b"1.0.0\n"
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
        b"1.0.0\n"
    );
    fs::remove_dir_all(root).unwrap();
}
