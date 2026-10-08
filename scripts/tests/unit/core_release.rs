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

fn explicit_repository(name: &str) -> ReleaseRepository {
    release_repository(
        name,
        &json!({"contract_target": ".codex/managed-skeleton.json", "assets": []}),
        &[
            ("VERSION", b"1.47.11\n"),
            ("CHANGELOG.md", b"# Changelog\n"),
            (
                "Cargo.toml",
                b"[package]\nname = \"example\"\nversion = \"1.47.11\"\n",
            ),
            (
                "Cargo.lock",
                b"version = 4\n[[package]]\nname = \"example\"\nversion = \"1.47.11\"\n",
            ),
            ("business.txt", b"base\n"),
        ],
    )
}

fn history_commit(repo: &ReleaseRepository, content: &[u8], message: &str) {
    fs::write(repo.0.join("business.txt"), content).unwrap();
    git_ok(&repo.0, &["add", "business.txt"]);
    git_ok(&repo.0, &["commit", "-m", message]);
}

#[test]
fn explicit_release_aggregates_highest_level_and_keeps_preview_read_only() {
    for (message, expected) in [
        ("perf: faster", "1.47.12"),
        ("feat: new feature", "1.48.0"),
        ("fix!: incompatible", "2.0.0"),
        ("fix: migrate\n\nBREAKING CHANGE: new format", "2.0.0"),
    ] {
        let repo = explicit_repository(expected);
        history_commit(&repo, b"one\n", "fix: first repair");
        history_commit(&repo, b"two\n", message);
        history_commit(&repo, b"three\n", "fix: last repair");
        let index = fs::read(repo.0.join(".git/index")).unwrap();
        let plan = build_explicit_release_plan(
            &repo.0,
            "feat!: ignored release label",
            vec![],
            &SystemProcessRunner,
        )
        .unwrap()
        .unwrap();
        assert_eq!(plan.new_version.to_string(), expected);
        assert_eq!(fs::read(repo.0.join("VERSION")).unwrap(), b"1.47.11\n");
        assert_eq!(fs::read(repo.0.join(".git/index")).unwrap(), index);
        let log = String::from_utf8(plan.writes[&repo.0.join("CHANGELOG.md")].clone()).unwrap();
        assert!(log.contains("first repair") && log.contains("last repair"));
        assert!(!log.contains("ignored release label"));
        for name in ["Cargo.toml", "Cargo.lock"] {
            assert!(String::from_utf8_lossy(&plan.writes[&repo.0.join(name)]).contains(expected));
        }
        apply_file_release_plan(&plan).unwrap();
        git_ok(&repo.0, &["add", "."]);
        git_ok(&repo.0, &["commit", "-m", "chore: release"]);
        assert!(
            build_explicit_release_plan(&repo.0, "feat: no changes", vec![], &SystemProcessRunner)
                .unwrap()
                .is_none()
        );
    }
}

#[test]
fn explicit_release_current_changes_participate_and_invalid_history_blocks() {
    let repo = explicit_repository("current");
    history_commit(&repo, b"one\n", "fix: repair");
    fs::write(repo.0.join("business.txt"), b"two\n").unwrap();
    let plan = build_explicit_release_plan(
        &repo.0,
        "feat: current feature",
        vec!["business.txt".into()],
        &SystemProcessRunner,
    )
    .unwrap()
    .unwrap();
    assert_eq!(plan.new_version.to_string(), "1.48.0");
    history_commit(&repo, b"three\n", "unclassified change");
    assert!(
        build_explicit_release_plan(&repo.0, "fix: release", vec![], &SystemProcessRunner)
            .unwrap_err()
            .contains("cannot classify unreleased commit")
    );
}

#[test]
fn explicit_release_blocks_manual_version_and_shallow_history() {
    let repo = explicit_repository("manual-version");
    fs::write(repo.0.join("VERSION"), b"9.0.0\n").unwrap();
    assert!(
        build_explicit_release_plan(
            &repo.0,
            "fix: release",
            vec!["VERSION".into()],
            &SystemProcessRunner
        )
        .unwrap_err()
        .contains("VERSION differs")
    );
    fs::write(repo.0.join("VERSION"), b"1.47.11\n").unwrap();
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(&repo.0)
        .output()
        .unwrap();
    fs::write(repo.0.join(".git/shallow"), output.stdout).unwrap();
    assert!(
        build_explicit_release_plan(&repo.0, "fix: release", vec![], &SystemProcessRunner)
            .unwrap_err()
            .contains("complete Git history")
    );
}

#[test]
fn explicit_release_excludes_historical_skeleton_commits_using_their_own_contracts() {
    let old = b"old\n";
    let contract = |payload: &[u8]| {
        json!({"contract_target": ".codex/managed-skeleton.json", "assets": [{
            "id": "managed.asset", "target": "managed.txt", "strategy": "whole", "current_sha256": digest(payload)
        }]})
    };
    let repo = release_repository(
        "history-ownership",
        &contract(old),
        &[("VERSION", b"1.0.0\n"), ("managed.txt", old)],
    );
    for payload in [b"second\n".as_slice(), b"third\n".as_slice()] {
        fs::write(repo.0.join("managed.txt"), payload).unwrap();
        fs::write(
            repo.0.join(".codex/managed-skeleton.json"),
            serde_json::to_vec(&contract(payload)).unwrap(),
        )
        .unwrap();
        git_ok(&repo.0, &["add", "."]);
        git_ok(&repo.0, &["commit", "-m", "feat!: skeleton only"]);
    }
    assert!(
        build_explicit_release_plan(&repo.0, "feat: release", vec![], &SystemProcessRunner)
            .unwrap()
            .is_none()
    );
    history_commit(&repo, b"business\n", "fix: actual business");
    let plan = build_explicit_release_plan(&repo.0, "feat: release", vec![], &SystemProcessRunner)
        .unwrap()
        .unwrap();
    assert_eq!(plan.new_version.to_string(), "1.0.1");
    assert!(
        !String::from_utf8_lossy(&plan.writes[&repo.0.join("CHANGELOG.md")])
            .contains("skeleton only")
    );
}

#[test]
fn release_policy_is_always_explicit_and_rejects_unknown_or_duplicate_values() {
    let repo = explicit_repository("policy");
    assert!(explicit_release_policy(&repo.0).unwrap());
    for policy in ["per_commit", "explicit_release"] {
        fs::write(
            repo.0.join(".codex/bridgeforge-version.json"),
            serde_json::to_vec(&json!({"schema_version": 1, "release_policy": policy})).unwrap(),
        )
        .unwrap();
        assert!(explicit_release_policy(&repo.0).unwrap());
    }
    for value in [br#"{"schema_version":1,"release_policy":"typo"}"#.as_slice(), br#"{"schema_version":1,"release_policy":"per_commit","release_policy":"explicit_release"}"#.as_slice()] {
        fs::write(repo.0.join(".codex/bridgeforge-version.json"), value).unwrap();
        assert!(explicit_release_policy(&repo.0).is_err());
    }
}

#[test]
fn explicit_release_keeps_project_region_changes_in_history() {
    let public = b"BEGIN\npublic\nEND\n";
    let contract = json!({"contract_target": ".codex/managed-skeleton.json", "assets": [{
        "id": "mixed", "target": "mixed.md", "strategy": "region", "region": {
            "begin": "BEGIN", "end": "END", "current_sha256": digest(public)
        }
    }]});
    let repo = release_repository(
        "mixed-history",
        &contract,
        &[
            ("VERSION", b"1.0.0\n"),
            ("mixed.md", b"BEGIN\npublic\nEND\nproject old\n"),
        ],
    );
    fs::write(
        repo.0.join("mixed.md"),
        b"BEGIN\npublic\nEND\nproject new\n",
    )
    .unwrap();
    git_ok(&repo.0, &["add", "."]);
    git_ok(&repo.0, &["commit", "-m", "feat: project region feature"]);
    let plan = build_explicit_release_plan(&repo.0, "fix: release", vec![], &SystemProcessRunner)
        .unwrap()
        .unwrap();
    assert_eq!(plan.new_version.to_string(), "1.1.0");
}

#[test]
fn explicit_release_stops_at_unreviewed_merge_history() {
    let repo = explicit_repository("merge-history");
    git_ok(&repo.0, &["checkout", "-b", "feature"]);
    history_commit(&repo, b"feature\n", "feat: side feature");
    git_ok(&repo.0, &["checkout", "-"]);
    fs::write(repo.0.join("main.txt"), b"main\n").unwrap();
    git_ok(&repo.0, &["add", "."]);
    git_ok(&repo.0, &["commit", "-m", "fix: main repair"]);
    git_ok(
        &repo.0,
        &["merge", "--no-ff", "feature", "-m", "chore: merge feature"],
    );
    assert!(
        build_explicit_release_plan(&repo.0, "fix: release", vec![], &SystemProcessRunner)
            .unwrap_err()
            .contains("merge/root commit")
    );
    assert_eq!(fs::read(repo.0.join("VERSION")).unwrap(), b"1.47.11\n");
}

#[test]
fn version_formatting_commits_do_not_reset_release_history_or_create_releases() {
    for has_feature in [false, true] {
        let repo = explicit_repository("version-formatting");
        if has_feature {
            history_commit(&repo, b"feature\n", "feat: must remain in release");
        }
        fs::write(repo.0.join("VERSION"), b" 1.47.11\n\n").unwrap();
        git_ok(&repo.0, &["add", "VERSION"]);
        git_ok(&repo.0, &["commit", "-m", "docs: version whitespace"]);
        let plan =
            build_explicit_release_plan(&repo.0, "fix: release", vec![], &SystemProcessRunner)
                .unwrap();
        if has_feature {
            let plan = plan.unwrap();
            assert_eq!(plan.new_version.to_string(), "1.48.0");
            assert!(
                String::from_utf8_lossy(&plan.writes[&repo.0.join("CHANGELOG.md")])
                    .contains("must remain in release")
            );
        } else {
            assert!(plan.is_none());
        }
        fs::write(repo.0.join("VERSION"), b"1.47.11\n").unwrap();
        let plan = build_explicit_release_plan(
            &repo.0,
            "feat!: formatting only",
            vec!["VERSION".into()],
            &SystemProcessRunner,
        )
        .unwrap();
        assert_eq!(
            plan.map(|p| p.new_version.to_string()),
            has_feature.then(|| "1.48.0".to_string())
        );
    }
}

#[test]
fn committed_version_changes_need_new_release_notes_and_matching_native_versions() {
    for mode in [
        "missing-notes",
        "existing-notes",
        "wrong-native",
        "decrease",
        "invalid",
    ] {
        let repo = explicit_repository(mode);
        if mode == "existing-notes" {
            fs::write(repo.0.join("CHANGELOG.md"), b"# Changelog\n\n## [1.48.0]\n").unwrap();
            git_ok(&repo.0, &["add", "CHANGELOG.md"]);
            git_ok(&repo.0, &["commit", "-m", "docs: prepare notes"]);
        }
        let version = match mode {
            "decrease" => "1.47.10\n",
            "invalid" => "invalid\n",
            _ => "1.48.0\n",
        };
        fs::write(repo.0.join("VERSION"), version).unwrap();
        if mode == "wrong-native" {
            fs::write(repo.0.join("CHANGELOG.md"), b"# Changelog\n\n## [1.48.0]\n").unwrap();
        }
        git_ok(&repo.0, &["add", "."]);
        git_ok(&repo.0, &["commit", "-m", "chore: manual version"]);
        let error =
            build_explicit_release_plan(&repo.0, "fix: release", vec![], &SystemProcessRunner)
                .unwrap_err();
        let expected = match mode {
            "decrease" => "decreases VERSION",
            "invalid" => "invalid semantic version",
            "wrong-native" => "native manifest disagrees",
            _ => "newly added CHANGELOG",
        };
        assert!(error.contains(expected), "{mode}: {error}");
    }
}

#[test]
fn deleted_or_reintroduced_version_is_not_an_initial_baseline() {
    let repo = explicit_repository("version-reintroduced");
    git_ok(&repo.0, &["rm", "VERSION"]);
    git_ok(&repo.0, &["commit", "-m", "chore: delete version"]);
    assert!(
        build_explicit_release_plan(&repo.0, "fix: release", vec![], &SystemProcessRunner)
            .unwrap_err()
            .contains("deletes VERSION")
    );
    fs::write(repo.0.join("VERSION"), b"1.47.11\n").unwrap();
    git_ok(&repo.0, &["add", "VERSION"]);
    git_ok(&repo.0, &["commit", "-m", "chore: restore version"]);
    assert!(
        build_explicit_release_plan(&repo.0, "fix: release", vec![], &SystemProcessRunner)
            .unwrap_err()
            .contains("reintroduces VERSION")
    );
}
