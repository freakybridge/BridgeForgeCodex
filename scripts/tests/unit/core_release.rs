use super::*;
use crate::{ProcessOutput, SystemProcessRunner};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

struct ReleaseRepository(PathBuf);

impl Drop for ReleaseRepository {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn git_ok(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn release_repository(name: &str, contract: &Value, files: &[(&str, &[u8])]) -> ReleaseRepository {
    let token = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("bridgeforge-release-{name}-{token}"));
    fs::create_dir_all(root.join(".codex")).unwrap();
    git_ok(&root, &["init"]);
    git_ok(&root, &["config", "user.name", "BridgeForge Test"]);
    git_ok(
        &root,
        &["config", "user.email", "bridgeforge@example.invalid"],
    );
    fs::write(
        root.join(".codex/managed-skeleton.json"),
        serde_json::to_vec_pretty(contract).unwrap(),
    )
    .unwrap();
    for (relative, payload) in files {
        let target = root.join(relative);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(target, payload).unwrap();
    }
    git_ok(&root, &["add", "."]);
    git_ok(&root, &["commit", "-m", "baseline"]);
    ReleaseRepository(root)
}

struct TimeoutRunner;

impl ProcessRunner for TimeoutRunner {
    fn run(&self, _request: &ProcessRequest) -> std::io::Result<ProcessOutput> {
        Ok(ProcessOutput {
            code: -1,
            stdout: Vec::new(),
            stderr: Vec::new(),
            timed_out: true,
        })
    }
}

#[test]
fn head_payload_timeout_is_a_hard_error() {
    let error = head_payload(Path::new("."), "managed.md", &TimeoutRunner).unwrap_err();
    assert!(error.contains("timed out"), "{error}");
}

#[test]
fn gitattributes_adoption_preserves_existing_project_rules() {
    let head_contract = json!({
        "contract_target": ".codex/managed-skeleton.json",
        "assets": []
    });
    let current_contract = json!({
        "contract_target": ".codex/managed-skeleton.json",
        "assets": [{
            "id": "root.gitattributes",
            "target": ".gitattributes",
            "strategy": "merge",
            "merge_policy": "git-attributes-default-lf",
            "merge_validation": {
                "required": {"pattern": "*", "text": "auto", "eol": "lf"}
            }
        }]
    });
    let repo = release_repository(
        "gitattributes-adoption",
        &head_contract,
        &[(".gitattributes", b"*.bat text eol=crlf\n")],
    );
    fs::write(
        repo.0.join(".codex/managed-skeleton.json"),
        serde_json::to_vec_pretty(&current_contract).unwrap(),
    )
    .unwrap();
    fs::write(
        repo.0.join(".gitattributes"),
        b"* text=auto eol=lf\n*.bat text eol=crlf\n",
    )
    .unwrap();
    let changed = [
        ".codex/managed-skeleton.json".into(),
        ".gitattributes".into(),
    ];
    assert_eq!(
        classify(&repo.0, &changed, &SystemProcessRunner).unwrap(),
        ReleaseKind::SkeletonOnly
    );
    fs::write(
        repo.0.join(".gitattributes"),
        b"* text=auto eol=lf\n*.bat text eol=crlf working-tree-encoding=UTF-8\n",
    )
    .unwrap();
    assert_eq!(
        classify(&repo.0, &changed, &SystemProcessRunner).unwrap(),
        ReleaseKind::Business
    );
}

#[test]
fn invalid_head_payload_never_becomes_a_trusted_same_contract_baseline() {
    let contract = json!({
        "contract_target": ".codex/managed-skeleton.json",
        "assets": [{
            "id": "managed.whole",
            "target": "managed.txt",
            "strategy": "whole",
            "current_sha256": digest(b"good\n")
        }]
    });
    let repo = release_repository("invalid-head", &contract, &[("managed.txt", b"bad\n")]);
    fs::write(repo.0.join("managed.txt"), b"good\n").unwrap();
    let error = classify(&repo.0, &["managed.txt".into()], &SystemProcessRunner).unwrap_err();
    assert!(
        error.contains("HEAD ownership baseline is invalid"),
        "{error}"
    );
}

#[test]
fn invalid_old_payload_during_contract_transition_is_conservative_business() {
    let head_contract = json!({
        "contract_target": ".codex/managed-skeleton.json",
        "assets": [{
            "id": "managed.whole",
            "target": "managed.txt",
            "strategy": "whole",
            "current_sha256": digest(b"claimed-old\n")
        }]
    });
    let current_contract = json!({
        "contract_target": ".codex/managed-skeleton.json",
        "assets": [{
            "id": "managed.whole",
            "target": "managed.txt",
            "strategy": "whole",
            "current_sha256": digest(b"new\n")
        }]
    });
    let repo = release_repository(
        "invalid-old-transition",
        &head_contract,
        &[("managed.txt", b"not-claimed-old\n")],
    );
    fs::write(
        repo.0.join(".codex/managed-skeleton.json"),
        serde_json::to_vec_pretty(&current_contract).unwrap(),
    )
    .unwrap();
    fs::write(repo.0.join("managed.txt"), b"new\n").unwrap();
    assert_eq!(
        classify(
            &repo.0,
            &[".codex/managed-skeleton.json".into(), "managed.txt".into()],
            &SystemProcessRunner,
        )
        .unwrap(),
        ReleaseKind::Business
    );
}

#[test]
fn semver_is_strict_and_ordered() {
    assert!("1.02.3".parse::<SemVer>().is_err());
    assert!("1.2".parse::<SemVer>().is_err());
    assert!("1.9.9".parse::<SemVer>().unwrap() < "2.0.0".parse().unwrap());
}

#[test]
fn configured_version_manifests_use_codex_config_directory() {
    let contract = json!({
        "contract_target": ".codex/managed-skeleton.json",
        "assets": []
    });
    let config = br#"{
        "schema_version": 1,
        "manifests": ["native/Cargo.toml"]
    }"#;
    let repo = release_repository(
        "version-config-location",
        &contract,
        &[
            (".codex/bridgeforge-version.json", config),
            (
                "native/Cargo.toml",
                b"[package]\nname = \"native\"\nversion = \"1.0.0\"\n",
            ),
        ],
    );
    assert_eq!(
        configured_manifests(&repo.0).unwrap(),
        vec![repo.0.join("native/Cargo.toml")]
    );
}

#[test]
fn conventional_commit_drives_release_level() {
    let info =
        parse_commit_message("feat(core)!: replace runtime\n\nBREAKING CHANGE: changed").unwrap();
    assert!(info.breaking);
    assert_eq!(bump("1.2.3".parse().unwrap(), &info).to_string(), "2.0.0");
    assert!(parse_commit_message("update runtime").is_err());
}

#[test]
fn empty_and_skeleton_changes_do_not_require_business_release() {
    assert_eq!(build_release_plan(Vec::new()).kind, ReleaseKind::None);
    let plan = build_release_plan(vec!["templates/AGENTS.md".into()]);
    assert_eq!(plan.kind, ReleaseKind::SkeletonOnly);
    assert!(!plan.requires_business_version);
}

#[test]
fn region_projection_distinguishes_public_and_project_changes() {
    let head_contract = json!({
        "contract_target": ".codex/managed-skeleton.json",
        "assets": [{
            "id": "managed.region",
            "target": "managed.md",
            "strategy": "region",
            "region": {
                "begin": "BEGIN",
                "end": "END",
                "current_sha256": digest(b"BEGIN\nold public\nEND\n")
            }
        }]
    });
    let current_contract = json!({
        "contract_target": ".codex/managed-skeleton.json",
        "assets": [{
            "id": "managed.region",
            "target": "managed.md",
            "strategy": "region",
            "region": {
                "begin": "BEGIN",
                "end": "END",
                "current_sha256": digest(b"BEGIN\nnew public\nEND\n")
            }
        }]
    });
    let repo = release_repository(
        "region",
        &head_contract,
        &[("managed.md", b"BEGIN\nold public\nEND\nproject\n")],
    );
    fs::write(
        repo.0.join(".codex/managed-skeleton.json"),
        serde_json::to_vec_pretty(&current_contract).unwrap(),
    )
    .unwrap();
    fs::write(
        repo.0.join("managed.md"),
        b"BEGIN\nnew public\nEND\nproject\n",
    )
    .unwrap();
    assert_eq!(
        classify(
            &repo.0,
            &[".codex/managed-skeleton.json".into(), "managed.md".into()],
            &SystemProcessRunner,
        )
        .unwrap(),
        ReleaseKind::SkeletonOnly
    );
    fs::write(
        repo.0.join("managed.md"),
        b"BEGIN\nnew public\nEND\nproject changed\n",
    )
    .unwrap();
    assert_eq!(
        classify(
            &repo.0,
            &[".codex/managed-skeleton.json".into(), "managed.md".into()],
            &SystemProcessRunner,
        )
        .unwrap(),
        ReleaseKind::Business
    );
}

#[test]
fn hooks_json_projection_preserves_external_handler_ownership() {
    let handler = |command: &str| {
        json!({
            "bridgeforgeCodexId": "bridgeforge-codex.project-hook.v1:stop",
            "command": command
        })
    };
    let contract = |command: &str| {
        json!({
            "contract_target": ".codex/managed-skeleton.json",
            "assets": [{
                "id": "hooks",
                "target": ".codex/hooks.json",
                "strategy": "merge",
                "merge_policy": "codex-hooks",
                "merge_validation": {
                    "required_handlers": [{
                        "id": "bridgeforge-codex.project-hook.v1:stop",
                        "event": "Stop",
                        "matcher": "",
                        "sha256": canonical_digest(&handler(command)).unwrap()
                    }]
                }
            }]
        })
    };
    let hooks = |managed: &str, external: &str| {
        serde_json::to_vec(&json!({
        "description": "managed",
        "hooks": {"Stop": [{"hooks": [
            {"bridgeforgeCodexId": "bridgeforge-codex.project-hook.v1:stop", "command": managed},
            {"command": external}
        ]}]}
    })).unwrap()
    };
    let head = hooks("old", "external");
    let repo = release_repository("hooks", &contract("old"), &[(".codex/hooks.json", &head)]);
    fs::write(
        repo.0.join(".codex/managed-skeleton.json"),
        serde_json::to_vec_pretty(&contract("new")).unwrap(),
    )
    .unwrap();
    fs::write(repo.0.join(".codex/hooks.json"), hooks("new", "external")).unwrap();
    assert_eq!(
        classify(
            &repo.0,
            &[
                ".codex/managed-skeleton.json".into(),
                ".codex/hooks.json".into()
            ],
            &SystemProcessRunner,
        )
        .unwrap(),
        ReleaseKind::SkeletonOnly
    );
    fs::write(
        repo.0.join(".codex/hooks.json"),
        hooks("new", "external changed"),
    )
    .unwrap();
    assert_eq!(
        classify(
            &repo.0,
            &[
                ".codex/managed-skeleton.json".into(),
                ".codex/hooks.json".into()
            ],
            &SystemProcessRunner,
        )
        .unwrap(),
        ReleaseKind::Business
    );
}

#[test]
fn cross_contract_marker_migration_uses_each_sides_contract() {
    let head_contract = json!({
        "contract_target": ".codex/managed-skeleton.json",
        "stamp": ".codex/version",
        "assets": [{
            "id": "managed.region",
            "target": "managed.md",
            "strategy": "region",
            "region": {
                "begin": "OLD-BEGIN",
                "end": "OLD-END",
                "current_sha256": digest(b"OLD-BEGIN\nold public\nOLD-END\n")
            }
        }]
    });
    let current_contract = json!({
        "contract_target": ".codex/managed-skeleton.json",
        "stamp": ".codex/version",
        "assets": [{
            "id": "managed.region",
            "target": "managed.md",
            "strategy": "region",
            "region": {
                "begin": "NEW-BEGIN",
                "end": "NEW-END",
                "current_sha256": digest(b"NEW-BEGIN\nnew public\nNEW-END\n")
            }
        }]
    });
    let repo = release_repository(
        "transition",
        &head_contract,
        &[("managed.md", b"OLD-BEGIN\nold public\nOLD-END\nproject\n")],
    );
    fs::write(
        repo.0.join(".codex/managed-skeleton.json"),
        serde_json::to_vec_pretty(&current_contract).unwrap(),
    )
    .unwrap();
    fs::write(
        repo.0.join("managed.md"),
        b"NEW-BEGIN\nnew public\nNEW-END\nproject\n",
    )
    .unwrap();
    assert_eq!(
        classify(
            &repo.0,
            &[".codex/managed-skeleton.json".into(), "managed.md".into()],
            &SystemProcessRunner,
        )
        .unwrap(),
        ReleaseKind::SkeletonOnly
    );
}

fn canonical_json(value: &Value) -> Value {
    match value {
        Value::Object(object) => Value::Object(
            object
                .iter()
                .map(|(key, value)| (key.clone(), canonical_json(value)))
                .collect::<BTreeMap<_, _>>()
                .into_iter()
                .collect(),
        ),
        Value::Array(values) => Value::Array(values.iter().map(canonical_json).collect()),
        value => value.clone(),
    }
}

fn canonical_digest(value: &Value) -> Result<String, String> {
    serde_json::to_vec(&canonical_json(value))
        .map(|payload| digest(&payload))
        .map_err(|error| error.to_string())
}
