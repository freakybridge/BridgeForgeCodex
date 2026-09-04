use bridgeforge_core::project_sync::{SyncMode, apply_plan, build_plan, build_plan_with_inputs};
use bridgeforge_core::{ProcessOutput, ProcessRequest, ProcessRunner};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static ID: AtomicU64 = AtomicU64::new(0);

#[test]
fn upstream_write_authorization_is_a_shared_public_rule() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap();
    let public = |text: &str| {
        text.split_once("<!-- BRIDGEFORGE:PUBLIC:BEGIN -->")
            .unwrap()
            .1
            .split_once("<!-- BRIDGEFORGE:PUBLIC:END -->")
            .unwrap()
            .0
            .replace("\r\n", "\n")
    };
    let template = fs::read_to_string(root.join("templates/AGENTS.md")).unwrap();
    let dogfood = fs::read_to_string(root.join("AGENTS.md")).unwrap();
    let shared = public(&template);
    assert!(shared.contains("用户仅授权当前项目改动时，禁止修改 BridgeForge 上游模板或其他项目"));
    assert!(
        shared
            .contains("反哺上游必须先说明对其他项目与未来初始化的影响、收益和风险，并取得明确授权")
    );
    assert!(shared.contains("未获授权时必须仅作为后续候选记录，禁止执行上游写入"));
    assert_eq!(shared, public(&dogfood));
}

#[test]
fn project_map_skills_refresh_generated_indexes_without_user_maintenance_prompts() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap();
    let find_doc = fs::read_to_string(root.join("skills/find-doc/SKILL.md")).unwrap();
    let sync_docs = fs::read_to_string(root.join("skills/sync-docs/SKILL.md")).unwrap();
    for (name, text) in [("find-doc", find_doc), ("sync-docs", sync_docs)] {
        assert!(
            text.contains("project-map ensure-current"),
            "{name} must refresh the generated map before reading it"
        );
        assert!(
            text.contains("禁止要求用户") && text.contains("自动生成的 Map"),
            "{name} must keep generated map maintenance away from the user"
        );
        assert!(
            !text.contains("候选映射并询问") && !text.contains("要不要顺手"),
            "{name} still contains the retired manual-maintenance prompt"
        );
    }
    assert!(
        !root
            .join("skills/find-doc/references/map-reminder-sop.md")
            .exists(),
        "manual map reminder SOP must be retired"
    );
}

#[cfg(windows)]
#[test]
fn shared_bundle_commit_keeps_components_consistent_when_old_image_is_running() {
    let fixture = Fixture::new();
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap();
    let mut request = ProcessRequest::new("powershell.exe", root);
    request.args = [
        "-NoProfile",
        "-NonInteractive",
        "-ExecutionPolicy",
        "Bypass",
        "-File",
    ]
    .into_iter()
    .map(Into::into)
    .collect();
    request.args.extend([
        root.join("scripts/tests/shared_transaction.ps1")
            .into_os_string(),
        "-RepositoryRoot".into(),
        root.as_os_str().to_owned(),
        "-Base".into(),
        fixture.0.as_os_str().to_owned(),
    ]);
    request.timeout = std::time::Duration::from_secs(60);
    let output = bridgeforge_core::SystemProcessRunner.run(&request).unwrap();
    assert!(!output.timed_out);
    assert_eq!(
        output.code,
        0,
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout)
            .contains("shared bundle rollback and deferred committed cleanup passed")
    );
}
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "bfc-distribution-{}-{}",
            std::process::id(),
            ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn write(path: &Path, content: impl AsRef<[u8]>) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}
fn hash(data: impl AsRef<[u8]>) -> String {
    format!("sha256:{:x}", Sha256::digest(data.as_ref()))
}
fn contract() -> Value {
    json!({
        "schema_version":4,"baseline_model": "current-only", "compatibility_baseline": "1.8.2","host":"codex","release_version":"1.8.2",
        "stamp":".codex/.bridgeforge_codex_version","contract_target":".codex/managed-skeleton.json",
        "assets":[{"id":"example","source":"templates/example.txt","target":".codex/example.txt","strategy":"whole","current_sha256":hash(b"managed\n")}],"generated_assets":[]
    })
}
fn minimal(f: &Fixture) -> (PathBuf, PathBuf) {
    let factory = f.0.join("factory");
    let project = f.0.join("project");
    fs::create_dir_all(&project).unwrap();
    write(&factory.join("templates/example.txt"), b"managed\n");
    write(
        &factory.join("templates/managed-skeleton.json"),
        serde_json::to_vec_pretty(&contract()).unwrap(),
    );
    (factory, project)
}

#[test]
fn current_version_repair_applies_and_receipt_proves_finalization() {
    let f = Fixture::new();
    let (factory, project) = minimal(&f);
    let first = build_plan(&project, &factory, SyncMode::Init).unwrap();
    apply_plan(first.clone(), &first.aggregate_fingerprint, false).unwrap();
    write(&project.join(".codex/example.txt"), b"drift\n");
    let plan = build_plan(&project, &factory, SyncMode::Update).unwrap();
    assert!(!plan.safe.iter().any(|a| a.id == "managed.stamp"));
    let receipt = apply_plan(plan.clone(), &plan.aggregate_fingerprint, true).unwrap();
    assert!(receipt.stamp_written_last);
    assert_eq!(receipt.project_readiness, "ready");
    assert_eq!(
        fs::read(project.join(".codex/example.txt")).unwrap(),
        b"managed\n"
    );
    assert_eq!(
        build_plan(&project, &factory, SyncMode::Update)
            .unwrap()
            .status,
        "current"
    );
}

#[test]
fn doc_structure_accepts_directory_instructions() {
    for layout in ["flat", "milestone"] {
        let f = Fixture::new();
        write(
            &f.0.join("doc/README.md"),
            format!("---\ndelivery_layout: {layout}\n---\n"),
        );
        for layer in [
            "0_architecture",
            "1_delivery",
            "2_bugs",
            "3_reference",
            "4_archive",
            "5_project_knowledgebase",
        ] {
            fs::create_dir_all(f.0.join("doc").join(layer)).unwrap();
        }
        write(
            &f.0.join("doc/AGENTS.md"),
            "# Documentation rules\n\n- Preserve project instructions.\n",
        );
        let report = bridgeforge_core::project_structure::inspect(&f.0);
        assert!(report.errors.is_empty(), "{layout}: {report:?}");
    }
}

#[test]
fn doc_structure_rejects_unexpected_entries() {
    for (name, directory) in [
        ("extra.md", false),
        ("6_unknown", true),
        ("AGENTS.md", true),
    ] {
        let f = Fixture::new();
        write(
            &f.0.join("doc/README.md"),
            "---\ndelivery_layout: flat\n---\n",
        );
        let path = f.0.join("doc").join(name);
        if directory {
            fs::create_dir_all(path).unwrap();
        } else {
            write(&path, "# Unexpected document\n");
        }
        let report = bridgeforge_core::project_structure::inspect(&f.0);
        assert!(
            report.errors.iter().any(|finding| {
                finding.code == "unexpected-doc-entry" && finding.path == format!("doc/{name}")
            }),
            "{name}: {report:?}"
        );
    }
}

#[test]
fn knowledge_structure_accepts_old_and_new_layout_and_never_auto_archives_topics() {
    let f = Fixture::new();
    write(
        &f.0.join("doc/README.md"),
        "---\ndelivery_layout: flat\n---\n",
    );
    for layer in [
        "0_architecture",
        "1_delivery",
        "2_bugs",
        "3_reference",
        "4_archive",
    ] {
        fs::create_dir_all(f.0.join("doc").join(layer)).unwrap();
    }
    assert!(
        bridgeforge_core::project_structure::inspect(&f.0)
            .errors
            .is_empty()
    );
    write(
        &f.0.join("doc/5_project_knowledgebase/topic/note.md"),
        "---\nlifecycle: completed\n---\n# 2000-01-01 topic\n",
    );
    assert!(
        bridgeforge_core::project_structure::inspect(&f.0)
            .errors
            .is_empty()
    );
    assert!(
        bridgeforge_core::archive_scan::scan(&f.0)
            .unwrap()
            .is_empty()
    );
    fs::create_dir_all(f.0.join("doc/6_unknown")).unwrap();
    assert!(
        bridgeforge_core::project_structure::inspect(&f.0)
            .errors
            .iter()
            .any(|finding| finding.code == "unexpected-doc-entry")
    );
}

#[test]
fn knowledge_seed_initializes_and_upgrades_without_owning_topic_bytes() {
    for existing in [false, true] {
        let f = Fixture::new();
        let (factory, project) = minimal(&f);
        let mut current = contract();
        current["release_version"] = json!("1.8.10");
        current["compatibility_baseline"] = json!("1.8.6");
        write(
            &factory.join("templates/managed-skeleton.json"),
            serde_json::to_vec(&current).unwrap(),
        );
        if existing {
            let plan = build_plan(&project, &factory, SyncMode::Init).unwrap();
            apply_plan(plan.clone(), &plan.aggregate_fingerprint, false).unwrap();
        }
        let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .unwrap();
        let real: Value = serde_json::from_slice(
            &fs::read(repository.join("templates/managed-skeleton.json")).unwrap(),
        )
        .unwrap();
        let seed = real["assets"]
            .as_array()
            .unwrap()
            .iter()
            .find(|asset| asset["id"] == "codex.doc.knowledgebase-seed")
            .unwrap()
            .clone();
        assert_eq!(seed["strategy"], "seed");
        current["assets"].as_array_mut().unwrap().push(seed.clone());
        current["release_version"] = json!("1.9.0");
        write(&factory.join(seed["source"].as_str().unwrap()), b"");
        write(
            &factory.join("templates/managed-skeleton.json"),
            serde_json::to_vec(&current).unwrap(),
        );
        let plan = build_plan(&project, &factory, SyncMode::Auto).unwrap();
        let target = project.join(seed["target"].as_str().unwrap());
        assert!(!target.exists(), "planning must not create the seed");
        apply_plan(plan.clone(), &plan.aggregate_fingerprint, false).unwrap();
        assert!(target.is_file());
        let topic = project.join("doc/5_project_knowledgebase/topic/note.md");
        write(&topic, b"project knowledge\r\n");
        write(&target, b"project-owned seed\r\n");
        current["release_version"] = json!("1.9.1");
        write(
            &factory.join("templates/managed-skeleton.json"),
            serde_json::to_vec(&current).unwrap(),
        );
        let plan = build_plan(&project, &factory, SyncMode::Update).unwrap();
        apply_plan(plan.clone(), &plan.aggregate_fingerprint, false).unwrap();
        assert_eq!(fs::read(&topic).unwrap(), b"project knowledge\r\n");
        assert_eq!(fs::read(&target).unwrap(), b"project-owned seed\r\n");
        assert_eq!(
            build_plan(&project, &factory, SyncMode::Update)
                .unwrap()
                .status,
            "current"
        );
    }
}

fn knowledge_migration_fixture(f: &Fixture, invalid: bool) -> (PathBuf, PathBuf, Value) {
    let (factory, project, mut manifest) = composite_fixture(f, invalid);
    for decision in manifest["sources"][0]["decisions"].as_array_mut().unwrap() {
        if decision["target"] == "doc/3_reference/migrated.md" {
            decision["target"] = json!("doc/5_project_knowledgebase/topic/migrated.md");
        }
        if decision["target"] == "doc/README.md" {
            decision["content_utf8"] = json!(decision["content_utf8"].as_str().unwrap().replace(
                "3_reference/migrated.md",
                "5_project_knowledgebase/topic/migrated.md"
            ));
        }
    }
    write(
        &project.join("doc/5_project_knowledgebase/other.md"),
        b"keep exactly\r\n",
    );
    (factory, project, manifest)
}

#[test]
fn knowledge_migration_is_indexed_atomic_and_preserves_existing_topics() {
    for invalid in [false, true] {
        let f = Fixture::new();
        let (factory, project, manifest) = knowledge_migration_fixture(&f, invalid);
        let plan =
            build_plan_with_inputs(&project, &factory, SyncMode::Adopt, Some(&manifest), None)
                .unwrap();
        let target = project.join("doc/5_project_knowledgebase/topic/migrated.md");
        assert!(!target.exists());
        let result = apply_plan(plan.clone(), &plan.aggregate_fingerprint, true);
        if invalid {
            assert!(result.unwrap_err().contains("rolled back"));
            assert!(!target.exists());
            assert!(!project.join(".codex/.bridgeforge_codex_version").exists());
            assert!(!project.join("doc/README.md").exists());
            assert_eq!(
                fs::read(project.join(".codex/rules/legacy.md")).unwrap(),
                b"legacy rule\r\n"
            );
        } else {
            assert!(result.unwrap().stamp_written_last);
            assert_eq!(fs::read(target).unwrap(), b"# migrated\n");
            assert!(!project.join(".codex/rules/legacy.md").exists());
            assert!(
                fs::read_to_string(project.join("doc/README.md"))
                    .unwrap()
                    .contains("5_project_knowledgebase/topic/migrated.md")
            );
        }
        assert_eq!(
            fs::read(project.join("doc/5_project_knowledgebase/other.md")).unwrap(),
            b"keep exactly\r\n"
        );
    }
}

#[test]
fn knowledge_migration_rejects_unindexed_unsafe_or_unconfirmed_targets_before_writing() {
    for case in ["index", "sibling", "traversal", "confirmation", "hash"] {
        let f = Fixture::new();
        let (factory, project, mut manifest) = knowledge_migration_fixture(&f, false);
        let source = &mut manifest["sources"][0];
        if case == "confirmation" {
            source["confirmed"] = json!(false);
        }
        for decision in source["decisions"].as_array_mut().unwrap() {
            if case == "index" && decision["target"] == "doc/README.md" {
                decision["content_utf8"] = json!("# no index\n");
            }
            if decision["target"] == "doc/5_project_knowledgebase/topic/migrated.md" {
                if case == "sibling" {
                    decision["target"] = json!("doc/6_other/migrated.md");
                }
                if case == "traversal" {
                    decision["target"] = json!("doc/5_project_knowledgebase/../../outside.md");
                }
                if case == "hash" {
                    decision["target_before_sha256"] = json!(hash(b"missing"));
                }
            }
        }
        assert!(
            build_plan_with_inputs(&project, &factory, SyncMode::Adopt, Some(&manifest), None)
                .is_err(),
            "{case}"
        );
        assert_eq!(
            fs::read(project.join(".codex/rules/legacy.md")).unwrap(),
            b"legacy rule\r\n"
        );
        assert!(!project.join("doc/README.md").exists());
        assert!(!project.join(".codex/.bridgeforge_codex_version").exists());
    }
}

#[test]
fn legacy_identity_is_validated_and_retired_atomically() {
    let f = Fixture::new();
    let (factory, project) = minimal(&f);
    let old = project.join(".codex/.bridgeforge_version");
    write(&old, b"1.7.0\n");
    let plan = build_plan(&project, &factory, SyncMode::Auto).unwrap();
    assert_eq!(plan.previous_version.as_deref(), Some("1.7.0"));
    assert!(
        plan.preservation_manifest["destructive_rebuild"]
            .as_bool()
            .unwrap()
    );
    apply_plan(plan.clone(), &plan.aggregate_fingerprint, false).unwrap();
    assert!(!old.exists());
    write(&old, b"1.7.0\n");
    assert!(
        build_plan(&project, &factory, SyncMode::Update)
            .unwrap_err()
            .contains("multiple")
    );
    fs::remove_file(project.join(".codex/.bridgeforge_codex_version")).unwrap();
    write(&old, b"9.0.0\n");
    assert!(
        build_plan(&project, &factory, SyncMode::Update)
            .unwrap_err()
            .contains("newer")
    );
    write(&old, b"not-a-version\n");
    assert!(build_plan(&project, &factory, SyncMode::Adopt).is_err());
}

#[test]
fn old_manifest_never_proves_deletion_ownership() {
    let f = Fixture::new();
    let (factory, project) = minimal(&f);
    write(
        &project.join(".codex/.bridgeforge_codex_version"),
        b"1.7.0\n",
    );
    write(
        &project.join(".codex/managed-skeleton.json"),
        b"unparseable retired contract",
    );
    write(&project.join(".codex/old.txt"), b"project data\n");
    let plan = build_plan(&project, &factory, SyncMode::Update).unwrap();
    assert!(!plan.gaps.is_empty());
    assert!(!plan.safe.iter().any(|a| a.operation == "delete"));
    let choices = json!({"preserve":["P:project-file:.codex/old.txt"],"delete":[]});
    let plan =
        build_plan_with_inputs(&project, &factory, SyncMode::Update, None, Some(&choices)).unwrap();
    let receipt = apply_plan(plan.clone(), &plan.aggregate_fingerprint, false).unwrap();
    assert_eq!(
        receipt.preserved_asset_ids,
        vec!["P:project-file:.codex/old.txt"]
    );
    assert_eq!(
        fs::read(project.join(".codex/old.txt")).unwrap(),
        b"project data\n"
    );
}

fn composite_fixture(f: &Fixture, invalid: bool) -> (PathBuf, PathBuf, Value) {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap();
    let factory = f.0.join("factory");
    let project = f.0.join("project");
    let mut current: Value = serde_json::from_slice(
        &fs::read(repository.join("templates/managed-skeleton.json")).unwrap(),
    )
    .unwrap();
    current["assets"].as_array_mut().unwrap().retain(|a| {
        matches!(
            a["target"].as_str(),
            Some("AGENTS.md" | ".codex/hooks.json" | "doc/README.md")
        )
    });
    current["generated_assets"] = json!([]);
    current["release_version"] = json!("1.8.2");
    current["compatibility_baseline"] = json!("1.8.2");
    let mut decisions = Vec::new();
    for asset in current["assets"].as_array().unwrap() {
        let source = asset["source"].as_str().unwrap();
        let target = asset["target"].as_str().unwrap();
        let bytes = fs::read(repository.join(source)).unwrap();
        write(&factory.join(source), &bytes);
        let text = String::from_utf8(bytes).unwrap();
        let (kind, content) = match target {
            "AGENTS.md" => (
                "agents",
                text.replace(
                    "<!-- BRIDGEFORGE:PROJECT:END -->",
                    "must preserve project rule\n<!-- BRIDGEFORGE:PROJECT:END -->",
                )
                .replace("公共架构红线", "obsolete public rule"),
            ),
            "doc/README.md" => (
                "documentation",
                format!("{text}\n[migrated](3_reference/migrated.md)\n"),
            ),
            _ => {
                let mut hooks: Value = serde_json::from_str(&text).unwrap();
                hooks["hooks"]["Stop"].as_array_mut().unwrap().push(json!({"hooks":[{"type":"command","command":"run .codex/hooks/project_sample/entrypoint.rs"}]}));
                ("hook-registration", serde_json::to_string(&hooks).unwrap())
            }
        };
        decisions.push(json!({"target":target,"asset_type":kind,"content_utf8":content,"target_before_sha256":null}));
    }
    decisions.push(json!({"target":"doc/3_reference/migrated.md","asset_type":"documentation","content_utf8":"# migrated\n","target_before_sha256":null}));
    decisions.push(json!({"target":".codex/hooks/project_sample/entrypoint.rs","asset_type":"hook","content_utf8":"fn main() {}\n","target_before_sha256":null}));
    if invalid {
        current["assets"][0]["agents_zones"]["public"]["current_sha256"] = json!("sha256:invalid");
    }
    write(
        &factory.join("templates/managed-skeleton.json"),
        serde_json::to_vec_pretty(&current).unwrap(),
    );
    let source = b"legacy rule\r\n";
    write(&project.join(".codex/rules/legacy.md"), source);
    let manifest = json!({"schema_version":1,"sources":[{"asset_id":"legacy-rule:.codex/rules/legacy.md","source_path":".codex/rules/legacy.md","source_sha256":hash(source),"kind":"legacy-rule","confirmed":true,"retire_source":true,"decisions":decisions,"discarded":[]}]});
    (factory, project, manifest)
}

#[test]
fn composite_migration_preserves_project_content_and_latest_public_baseline() {
    let f = Fixture::new();
    let (factory, project, manifest) = composite_fixture(&f, false);
    let plan =
        build_plan_with_inputs(&project, &factory, SyncMode::Adopt, Some(&manifest), None).unwrap();
    let receipt = apply_plan(plan.clone(), &plan.aggregate_fingerprint, true).unwrap();
    assert!(receipt.asset_migration_manifest_sha256.is_some());
    assert!(!project.join(".codex/rules/legacy.md").exists());
    let agents = fs::read_to_string(project.join("AGENTS.md")).unwrap();
    assert!(agents.contains("must preserve project rule"));
    assert!(!agents.contains("obsolete public rule"));
    let public = agents
        .split("<!-- BRIDGEFORGE:PUBLIC:END -->")
        .next()
        .unwrap();
    assert!(
        public
            .contains("反哺上游必须先说明对其他项目与未来初始化的影响、收益和风险，并取得明确授权")
    );
    assert!(
        fs::read_to_string(project.join("doc/README.md"))
            .unwrap()
            .contains("3_reference/migrated.md")
    );
    assert!(
        fs::read_to_string(project.join(".codex/hooks.json"))
            .unwrap()
            .contains("project_sample/entrypoint.rs")
    );
}

#[test]
fn composite_migration_failure_restores_every_source_before_stamping() {
    let f = Fixture::new();
    let (factory, project, manifest) = composite_fixture(&f, true);
    let plan =
        build_plan_with_inputs(&project, &factory, SyncMode::Adopt, Some(&manifest), None).unwrap();
    assert!(
        apply_plan(plan.clone(), &plan.aggregate_fingerprint, true)
            .unwrap_err()
            .contains("rolled back")
    );
    assert_eq!(
        fs::read(project.join(".codex/rules/legacy.md")).unwrap(),
        b"legacy rule\r\n"
    );
    for path in [
        "AGENTS.md",
        ".codex/hooks.json",
        "doc/README.md",
        "doc/3_reference/migrated.md",
        ".codex/.bridgeforge_codex_version",
    ] {
        assert!(!project.join(path).exists(), "{path}");
    }
}

#[test]
fn project_hook_registry_migration_receipt_matches_final_native_payload() {
    let f = Fixture::new();
    let (factory, project, mut manifest) = composite_fixture(&f, false);
    let decision = manifest["sources"][0]["decisions"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|d| d["target"] == ".codex/hooks.json")
        .unwrap();
    let mut document: Value =
        serde_json::from_str(decision["content_utf8"].as_str().unwrap()).unwrap();
    document["bridgeforgeProjectHooks"] = json!({"schema_version":1,"hooks":[]});
    decision["content_utf8"] = json!(document.to_string());
    let plan =
        build_plan_with_inputs(&project, &factory, SyncMode::Adopt, Some(&manifest), None).unwrap();
    let target = plan.asset_migration["targets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["target"] == ".codex/hooks.json")
        .unwrap();
    let expected = target["after_sha256"].as_str().unwrap().to_string();
    apply_plan(plan.clone(), &plan.aggregate_fingerprint, true).unwrap();
    assert_eq!(
        hash(fs::read(project.join(".codex/hooks.json")).unwrap()),
        expected
    );
    assert!(project.join(".codex/project-hooks.json").is_file());
}

#[test]
fn standalone_project_hook_registry_is_required_preserve_during_rebuild() {
    let f = Fixture::new();
    let (factory, project) = minimal(&f);
    write(&project.join(".codex/hooks.json"), b"{\"hooks\":{}}");
    write(
        &project.join(".codex/project-hooks.json"),
        b"{\"schema_version\":1,\"hooks\":[]}",
    );
    let plan = build_plan(&project, &factory, SyncMode::Adopt).unwrap();
    assert!(
        plan.preservation_manifest["entries"]
            .as_array()
            .unwrap()
            .iter()
            .any(
                |e| e["id"] == "R:project-hook-registry" && e["disposition"] == "required-preserve"
            )
    );
    assert!(
        !plan
            .gaps
            .iter()
            .any(|gap| gap.contains("project-hooks.json"))
    );
}

#[test]
fn legacy_hook_migration_accepts_explicit_standalone_registry() {
    let f = Fixture::new();
    let (_, project, mut manifest) = composite_fixture(&f, false);
    let decisions = manifest["sources"][0]["decisions"].as_array_mut().unwrap();
    let native = decisions
        .iter_mut()
        .find(|d| d["target"] == ".codex/hooks.json")
        .unwrap();
    let mut document: Value =
        serde_json::from_str(native["content_utf8"].as_str().unwrap()).unwrap();
    document["hooks"]["Stop"].as_array_mut().unwrap().pop();
    native["content_utf8"] = json!(document.to_string());
    decisions.push(json!({"target":".codex/project-hooks.json","asset_type":"hook-registration","target_before_sha256":null,"content_utf8":json!({"schema_version":1,"hooks":[{"id":"sample","events":[{"event":"Stop"}]}]}).to_string()}));
    let validated =
        bridgeforge_core::asset_migration::validate_manifest(&project, &manifest, &[]).unwrap();
    assert!(
        validated
            .targets
            .iter()
            .any(|t| t.target == ".codex/project-hooks.json")
    );
}

#[test]
fn migration_cannot_register_a_hook_only_in_a_note() {
    let f = Fixture::new();
    let (factory, project, mut manifest) = composite_fixture(&f, false);
    let decision = manifest["sources"][0]["decisions"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|d| d["target"] == ".codex/hooks.json")
        .unwrap();
    let mut document: Value =
        serde_json::from_str(decision["content_utf8"].as_str().unwrap()).unwrap();
    document["hooks"]["Stop"].as_array_mut().unwrap().pop();
    document["note"] = json!(".codex/hooks/project_sample/entrypoint.rs");
    decision["content_utf8"] = json!(document.to_string());
    let error = build_plan_with_inputs(&project, &factory, SyncMode::Adopt, Some(&manifest), None)
        .unwrap_err();
    assert!(error.contains("not registered"));
    assert!(project.join(".codex/rules/legacy.md").exists());
}

#[test]
fn migrating_new_hook_does_not_resurrect_deleted_hook_registration() {
    let f = Fixture::new();
    let (factory, project, mut manifest) = composite_fixture(&f, false);
    let target = ".codex/hooks/project_old/entrypoint.rs";
    write(&project.join(target), b"fn main() {}\n");
    let kept = ".codex/hooks/project_old_extra/entrypoint.rs";
    write(&project.join(kept), b"fn main() {}\n");
    let kept_handler = json!({"hooks":[{"type":"command","command":format!("run {kept}")}]});
    let mut original: Value =
        serde_json::from_slice(&fs::read(factory.join("templates/hooks.json")).unwrap()).unwrap();
    let handler = json!({"hooks":[{"type":"command","command":format!("run {target}")}]});
    original["hooks"]["Stop"]
        .as_array_mut()
        .unwrap()
        .push(handler.clone());
    original["hooks"]["Stop"]
        .as_array_mut()
        .unwrap()
        .push(kept_handler.clone());
    let bytes = serde_json::to_vec(&original).unwrap();
    write(&project.join(".codex/hooks.json"), &bytes);
    let decision = manifest["sources"][0]["decisions"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|d| d["target"] == ".codex/hooks.json")
        .unwrap();
    decision["target_before_sha256"] = json!(hash(&bytes));
    let mut proposed: Value =
        serde_json::from_str(decision["content_utf8"].as_str().unwrap()).unwrap();
    proposed["hooks"]["Stop"]
        .as_array_mut()
        .unwrap()
        .push(handler);
    proposed["hooks"]["Stop"]
        .as_array_mut()
        .unwrap()
        .push(kept_handler);
    decision["content_utf8"] = json!(proposed.to_string());
    let choices = json!({"preserve":["P:project-hook-bundle:.codex/hooks/project_old_extra"],"delete":["P:project-hook-bundle:.codex/hooks/project_old"]});
    let plan = build_plan_with_inputs(
        &project,
        &factory,
        SyncMode::Adopt,
        Some(&manifest),
        Some(&choices),
    )
    .unwrap();
    apply_plan(plan.clone(), &plan.aggregate_fingerprint, true).unwrap();
    assert!(!project.join(target).exists());
    let hooks = fs::read_to_string(project.join(".codex/hooks.json")).unwrap();
    assert!(!hooks.contains("project_old/entrypoint.rs"));
    assert!(hooks.contains("project_old_extra/entrypoint.rs"));
    assert!(project.join(kept).is_file());
    assert!(!project.join(".codex/hooks/project_old").exists());
    assert!(hooks.contains("project_sample"));
    let manifest_path = factory.join("templates/managed-skeleton.json");
    let mut current: Value = serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    current["release_version"] = json!("1.8.3");
    write(&manifest_path, current.to_string());
    let next = build_plan(&project, &factory, SyncMode::Update).unwrap();
    assert_eq!(next.preservation_manifest["destructive_rebuild"], false);
    assert!(next.blockers.is_empty(), "{:?}", next.blockers);
    apply_plan(next.clone(), &next.aggregate_fingerprint, true).unwrap();
    let hooks = fs::read_to_string(project.join(".codex/hooks.json")).unwrap();
    assert!(!hooks.contains("project_old/entrypoint.rs"));
    assert!(hooks.contains("project_old_extra/entrypoint.rs"));
    assert!(hooks.contains("project_sample/entrypoint.rs"));
    assert!(project.join(kept).is_file());
}

#[test]
fn concurrent_launchers_reclaim_one_dead_worker_exactly_once() {
    use bridgeforge_core::memory::worker::*;
    let f = Fixture::new();
    write(&f.0.join("worker.json"),json!({"schemaVersion":1,"token":"dead","pid":0,"launcherPid":0,"startedUtc":"2000-01-01T00:00:00Z"}).to_string());
    let stale_queue = f.0.join("queue/reconcile.lock");
    write(
        &stale_queue,
        json!({"pid":0,"token":"dead","utc":"2000-01-01T00:00:00Z"}).to_string(),
    );
    fs::File::options()
        .write(true)
        .open(&stale_queue)
        .unwrap()
        .set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(1))
        .unwrap();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(8));
    let handles = (0..8)
        .map(|_| {
            let barrier = barrier.clone();
            let path = f.0.clone();
            std::thread::spawn(move || {
                barrier.wait();
                reserve_worker(&path).unwrap()
            })
        })
        .collect::<Vec<_>>();
    let results = handles
        .into_iter()
        .map(|h| h.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        results
            .iter()
            .filter(|r| matches!(r, WorkerReservation::Acquired(_)))
            .count(),
        1
    );
    let current = read_worker_state(&f.0).unwrap().unwrap();
    for result in results {
        let state = match result {
            WorkerReservation::Acquired(s) | WorkerReservation::Reused(s) => s,
        };
        assert_eq!(state.token, current.token);
    }
    assert!(!release_worker(&f.0, "dead").unwrap());
    assert_eq!(
        read_worker_state(&f.0).unwrap().unwrap().token,
        current.token
    );
}

struct RuntimeRunner {
    old: bool,
    lock_failed: bool,
}
impl ProcessRunner for RuntimeRunner {
    fn run(&self, r: &ProcessRequest) -> std::io::Result<ProcessOutput> {
        let (code, text) = if r.args.first().is_some_and(|a| a == "metadata") {
            (if self.lock_failed { 1 } else { 0 }, "{}".to_string())
        } else if r.program == "cargo" {
            (
                0,
                format!(
                    "cargo {} (fixture)",
                    if self.old { "1.85.0" } else { "1.88.0" }
                ),
            )
        } else if r.program == "rustc" {
            (0, "rustc 1.88.0 (fixture)".into())
        } else {
            (0,json!({"schema":1,"name":"bridgeforge","status":"ok","version":env!("CARGO_PKG_VERSION")}).to_string())
        };
        Ok(ProcessOutput {
            code,
            stdout: text.into_bytes(),
            stderr: vec![],
            timed_out: false,
        })
    }
}

#[test]
fn product_preflight_uses_product_workspace_and_enforces_toolchain_and_lock() {
    let f = Fixture::new();
    let product = f.0.join("product");
    let downstream = f.0.join("empty");
    fs::create_dir(&downstream).unwrap();
    write(&product.join("templates/hooks/Cargo.toml"), b"fixture");
    write(&product.join("templates/hooks/Cargo.lock"), b"fixture");
    write(&product.join("VERSION"), env!("CARGO_PKG_VERSION"));
    let binary = f.0.join("bridgeforge.exe");
    write(&binary, b"fixture");
    let validate = |old, lock_failed| {
        bridgeforge_core::runtime::validate_product(
            &product,
            &binary,
            &RuntimeRunner { old, lock_failed },
        )
    };
    assert!(validate(false, false).is_ok());
    assert!(validate(true, false).unwrap_err().contains("at least"));
    assert!(validate(false, true).unwrap_err().contains("lockfile"));
    assert_eq!(fs::read_dir(downstream).unwrap().count(), 0);
}

#[test]
fn lifecycle_queue_drains_events_arriving_during_sync() {
    use bridgeforge_core::memory::worker::*;
    let f = Fixture::new();
    mark_pending(&f.0, "SessionStart").unwrap();
    let WorkerReservation::Acquired(worker) = reserve_worker(&f.0).unwrap() else {
        panic!()
    };
    let mut calls = 0;
    let (_, restart) = drain_pending(&f.0, &worker.token, || {
        calls += 1;
        let before = fs::read(f.0.join("pending.json")).unwrap();
        if calls == 1 {
            mark_pending(&f.0, "Stop").unwrap();
        }
        clear_pending_if_unchanged(&f.0, Some(&before)).unwrap();
        Ok("noop".into())
    })
    .unwrap();
    assert_eq!(calls, 2);
    assert!(!restart);
    assert!(read_pending(&f.0).unwrap().is_none());
    assert!(read_worker_state(&f.0).unwrap().is_none());
}

struct CredentialRunner {
    private: bool,
}
impl ProcessRunner for CredentialRunner {
    fn run(&self, r: &ProcessRequest) -> std::io::Result<ProcessOutput> {
        let args = r
            .args
            .iter()
            .map(|a| a.to_string_lossy())
            .collect::<Vec<_>>();
        let (code, text) = if args.contains(&"fill".into()) {
            assert_eq!(
                r.env
                    .get(std::ffi::OsStr::new("GIT_TERMINAL_PROMPT"))
                    .unwrap(),
                "0"
            );
            (
                0,
                "protocol=https\nhost=github.com\nusername=test\npassword=fixture-secret\n"
                    .to_string(),
            )
        } else if r.program == "git" {
            (
                0,
                "https://github.com/owner/bridgeforge-codex-memories".into(),
            )
        } else if r.env.contains_key(std::ffi::OsStr::new("GH_TOKEN")) {
            assert_eq!(
                r.env.get(std::ffi::OsStr::new("GH_TOKEN")).unwrap(),
                "fixture-secret"
            );
            assert!(!args.iter().any(|a| a.contains("fixture-secret")));
            (0, if self.private { "PRIVATE" } else { "PUBLIC" }.into())
        } else {
            (1, "expired authentication".into())
        };
        Ok(ProcessOutput {
            code,
            stdout: text.into_bytes(),
            stderr: b"fixture-secret diagnostic".to_vec(),
            timed_out: false,
        })
    }
}

#[test]
fn git_credentials_verify_private_api_without_exposing_secrets() {
    use bridgeforge_core::memory::MemoryRemoteClient;
    let f = Fixture::new();
    let remote = "https://github.com/owner/bridgeforge-codex-memories";
    MemoryRemoteClient::new(&CredentialRunner { private: true })
        .verify_private_github_repository(&f.0, remote)
        .unwrap();
    let error = MemoryRemoteClient::new(&CredentialRunner { private: false })
        .verify_private_github_repository(&f.0, remote)
        .unwrap_err()
        .to_string();
    assert!(error.contains("private"));
    assert!(!error.contains("fixture-secret"));
}
