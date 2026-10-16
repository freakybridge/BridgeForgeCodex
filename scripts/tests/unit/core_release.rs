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

fn independent_repository(name: &str, legacy: bool) -> ReleaseRepository {
    let config = if legacy { json!({"schema_version":1,"release_policy":"explicit_release","manifests":["Cargo.toml"]}) }
        else { independent_config() };
    release_repository(name, &json!({"contract_target":".codex/managed-skeleton.json","assets":[{
        "id":"managed.asset","target":"managed.txt","strategy":"whole","current_sha256":digest(b"old\n")
    }]}), &[
        ("managed.txt", b"old\n"),
        ("VERSION", b"1.0.0\n"), ("CHANGELOG.md", b"# Changelog\n"),
        (".codex/bridgeforge-version.json", &serde_json::to_vec(&config).unwrap()),
        // core is a legitimate implicit path member of app; history and disk
        // must discover it identically rather than reporting a new package.
        ("Cargo.toml", b"[workspace]\nmembers = [\"app\", \"idle\"]\nresolver = \"3\"\n[workspace.package]\nversion = \"1.0.0\"\n"),
        ("core/Cargo.toml", b"[package]\nname = \"core\"\nversion.workspace = true\nedition = \"2024\"\n"),
        ("app/Cargo.toml", b"[package]\nname = \"app\"\nversion.workspace = true\nedition = \"2024\"\n[dependencies]\ncore = { path = \"../core\", version = \"1.0.0\" }\n"),
        ("idle/Cargo.toml", b"[package]\nname = \"idle\"\nversion.workspace = true\nedition = \"2024\"\n"),
        ("core/src/lib.rs", b"pub fn value() -> u32 { 1 }\n"),
        ("app/src/lib.rs", b"pub fn value() -> u32 { core::value() }\n"),
        ("idle/src/lib.rs", b"pub fn value() -> u32 { 0 }\n"),
        ("Cargo.lock", b"version = 4\n[[package]]\nname = \"app\"\nversion = \"1.0.0\"\ndependencies = [\"core\"]\n[[package]]\nname = \"core\"\nversion = \"1.0.0\"\n[[package]]\nname = \"idle\"\nversion = \"1.0.0\"\n"),
    ])
}

fn independent_config() -> Value {
    json!({"schema_version":2,"release_policy":"explicit_release","strategy":"rust_independent",
        "rust":{"workspaces":["Cargo.toml"],"dependency_bump":"patch","shared_inputs":[],"product_inputs":["README.md","doc/"]}})
}

#[test]
fn independent_release_preserves_history_attribution_and_is_idempotent() {
    let repo = independent_repository("independent-history", false);
    fs::write(repo.0.join("core/src/lib.rs"), b"pub fn value() -> u32 { 2 }\n").unwrap();
    git_ok(&repo.0, &["add", "."]);
    git_ok(&repo.0, &["commit", "-m", "feat: core feature"]);
    fs::write(repo.0.join("app/src/lib.rs"), b"pub fn value() -> u32 { core::value() + 1 }\n").unwrap();
    git_ok(&repo.0, &["add", "."]);
    git_ok(&repo.0, &["commit", "-m", "fix: app repair"]);
    let index = fs::read(repo.0.join(".git/index")).unwrap();
    let plan = build_explicit_release_plan(&repo.0,"feat!: ignored",vec![],&SystemProcessRunner).unwrap().unwrap();
    assert_eq!(plan.new_version.to_string(),"1.1.0");
    let changes = serde_json::to_value(&plan.package_changes).unwrap();
    let find = |name: &str| changes.as_array().unwrap().iter().find(|v| v["name"] == name).unwrap()["version_after"].as_str().unwrap().to_string();
    assert_eq!(find("core"),"1.1.0");
    assert_eq!(find("app"),"1.0.1");
    assert!(!changes.as_array().unwrap().iter().any(|v|v["name"]=="idle"));
    assert_eq!(fs::read(repo.0.join(".git/index")).unwrap(),index);
    assert_eq!(fs::read(repo.0.join("VERSION")).unwrap(),b"1.0.0\n");
    apply_file_release_plan(&plan).unwrap();
    // Cargo itself must accept the generated manifests and lock, without fixing them.
    let mut request = ProcessRequest::new("cargo",&repo.0);
    request.args = ["metadata","--locked","--offline","--format-version","1"].iter().map(Into::into).collect();
    let output = SystemProcessRunner.run(&request).unwrap();
    assert_eq!(output.code,0,"{}",String::from_utf8_lossy(&output.stderr));
    git_ok(&repo.0,&["add","."]);
    git_ok(&repo.0,&["commit","-m","chore: release"]);
    assert!(build_explicit_release_plan(&repo.0,"feat!: repeat",vec![],&SystemProcessRunner).unwrap().is_none());
}

#[test]
fn independent_migration_keeps_versions_and_unreleased_business_history() {
    let repo = independent_repository("independent-migration",true);
    fs::write(repo.0.join("app/src/lib.rs"),b"pub fn value() -> u32 { 7 }\n").unwrap();
    git_ok(&repo.0,&["add","."]);
    git_ok(&repo.0,&["commit","-m","fix: earlier app repair"]);
    fs::write(repo.0.join(".codex/bridgeforge-version.json"),serde_json::to_vec(&independent_config()).unwrap()).unwrap();
    let plan = build_explicit_release_plan(&repo.0,"chore: enable independent release",vec![".codex/bridgeforge-version.json".into()],&SystemProcessRunner).unwrap().unwrap();
    assert_eq!(plan.package_changes.len(),1);
    assert_eq!(plan.package_changes[0].name,"app");
    assert_eq!(plan.package_changes[0].version_after,"1.0.1");
    let idle = String::from_utf8(plan.writes[&repo.0.join("idle/Cargo.toml")].clone()).unwrap();
    assert!(idle.contains("1.0.0"));
    assert!(!idle.contains("version.workspace"));
}

fn historical_lock_config(legacy: bool) -> Value {
    if legacy {
        json!({"schema_version":1,"release_policy":"explicit_release","manifests":["native/Cargo.toml"]})
    } else {
        json!({"schema_version":2,"release_policy":"explicit_release","strategy":"rust_independent",
            "rust":{"workspaces":["native/Cargo.toml","strategy/Cargo.toml"],
            "product_inputs":["strategy/Cargo.lock"]}})
    }
}

fn strategy_lock(stale: bool) -> &'static [u8] {
    if stale {
        b"version = 4\n[[package]]\nname = 'strategy'\nversion = '1.0.0'\ndependencies = ['native']\n[[package]]\nname = 'native'\nversion = '0.9.0'\n"
    } else {
        b"version = 4\n[[package]]\nname = 'strategy'\nversion = '1.0.0'\ndependencies = ['native']\n[[package]]\nname = 'native'\nversion = '1.0.0'\n"
    }
}

fn historical_lock_repository(name: &str, config: Value, stale: bool) -> ReleaseRepository {
    release_repository(name, &json!({"contract_target":".codex/managed-skeleton.json","assets":[]}), &[
        ("VERSION", b"1.0.0\n"), ("CHANGELOG.md", b"# Changelog\n"),
        (".codex/bridgeforge-version.json", &serde_json::to_vec(&config).unwrap()),
        ("native/Cargo.toml", b"[package]\nname = 'native'\nversion = '1.0.0'\nedition = '2024'\n[workspace]\n"),
        ("native/src/lib.rs", b"pub fn value() -> u32 { 1 }\n"),
        ("native/Cargo.lock", b"version = 4\n[[package]]\nname = 'native'\nversion = '1.0.0'\n"),
        ("strategy/Cargo.toml", b"[package]\nname = 'strategy'\nversion = '1.0.0'\nedition = '2024'\n[workspace]\n[dependencies]\nnative = { path = '../native', version = '1.0.0' }\n"),
        ("strategy/src/lib.rs", b"pub fn value() -> u32 { native::value() }\n"),
        ("strategy/Cargo.lock", strategy_lock(stale)),
    ])
}

fn write_independent_lock_config(repo: &ReleaseRepository) {
    fs::write(repo.0.join(".codex/bridgeforge-version.json"),
        serde_json::to_vec(&historical_lock_config(false)).unwrap()).unwrap();
}

struct ReadOnlyReleaseRunner;

impl ProcessRunner for ReadOnlyReleaseRunner {
    fn run(&self, request: &ProcessRequest) -> std::io::Result<ProcessOutput> {
        assert_eq!(request.program, "git", "release preview must not invoke Cargo or a resolver");
        for forbidden in ["fetch", "pull", "push", "add", "commit", "reset", "checkout"] {
            assert!(!request.args.iter().any(|arg| arg == forbidden), "unexpected mutation/network command: {request:?}");
        }
        SystemProcessRunner.run(request)
    }
}

#[test]
fn independent_historical_lock_migration_accepts_legacy_boundary_and_intermediate() {
    for stale_boundary in [true, false] {
        let repo = historical_lock_repository("legacy-lock", historical_lock_config(true), stale_boundary);
        fs::write(repo.0.join("strategy/Cargo.lock"), strategy_lock(true)).unwrap();
        fs::write(repo.0.join("strategy/src/lib.rs"), b"pub fn value() -> u32 { native::value() + 1 }\n").unwrap();
        git_ok(&repo.0, &["add", "."]);
        git_ok(&repo.0, &["commit", "-m", "feat: strategy improvement"]);
        fs::write(repo.0.join("strategy/Cargo.lock"), strategy_lock(false)).unwrap();
        let legacy_config = fs::read(repo.0.join(".codex/bridgeforge-version.json")).unwrap();
        let candidate = config::parse(&serde_json::to_vec(&historical_lock_config(false)).unwrap()).unwrap().unwrap();
        history::validate_candidate_baseline(&repo.0, &ReadOnlyReleaseRunner, &candidate).unwrap();
        assert_eq!(fs::read(repo.0.join(".codex/bridgeforge-version.json")).unwrap(), legacy_config);
        write_independent_lock_config(&repo);
        let index = fs::read(repo.0.join(".git/index")).unwrap();
        let lock = fs::read(repo.0.join("strategy/Cargo.lock")).unwrap();
        let config = fs::read(repo.0.join(".codex/bridgeforge-version.json")).unwrap();
        let plan = build_explicit_release_plan(&repo.0, "chore: migrate", vec![], &ReadOnlyReleaseRunner).unwrap().unwrap();
        assert_eq!(plan.new_version.to_string(), "1.1.0");
        assert_eq!(plan.package_changes.len(), 1);
        assert_eq!(plan.package_changes[0].name, "strategy");
        assert_eq!(plan.package_changes[0].version_after, "1.1.0");
        let preview = preview(&repo.0, "chore: migrate", &ReadOnlyReleaseRunner);
        assert_eq!(preview.code, 0, "{}", preview.stderr);
        assert_eq!(preview.receipt.unwrap()["status"], "release-planned");
        assert_eq!(fs::read(repo.0.join(".git/index")).unwrap(), index);
        assert_eq!(fs::read(repo.0.join("strategy/Cargo.lock")).unwrap(), lock);
        assert_eq!(fs::read(repo.0.join(".codex/bridgeforge-version.json")).unwrap(), config);
        assert_eq!(fs::read(repo.0.join("VERSION")).unwrap(), b"1.0.0\n");
        assert!(!repo.0.join(".runtime").exists());
    }
}

#[test]
fn independent_historical_lock_migration_keeps_current_and_schema2_locks_strict() {
    for (legacy, stale_boundary) in [(true, false), (false, true), (false, false)] {
        let repo = historical_lock_repository("strict-lock", historical_lock_config(legacy), stale_boundary);
        if !stale_boundary {
            fs::write(repo.0.join("strategy/Cargo.lock"), strategy_lock(true)).unwrap();
            git_ok(&repo.0, &["add", "."]);
            git_ok(&repo.0, &["commit", "-m", "fix: intermediate lock"]);
        }
        if !legacy {
            fs::write(repo.0.join("strategy/Cargo.lock"), strategy_lock(false)).unwrap();
        }
        write_independent_lock_config(&repo);
        let error = build_explicit_release_plan(&repo.0, "fix: migrate", vec![], &SystemProcessRunner).unwrap_err();
        assert!(error.contains("strategy/Cargo.lock: local package native has missing, ambiguous, or stale version"), "{error}");
    }
}

#[test]
fn independent_historical_lock_migration_keeps_identity_and_effective_version_guards() {
    for replacement in ["name = 'renamed'", "version = '0.9.0'"] {
        let repo = historical_lock_repository("legacy-identity", historical_lock_config(true), true);
        let original = fs::read_to_string(repo.0.join("strategy/Cargo.toml")).unwrap();
        let changed = if replacement.starts_with("name") {
            original.replace("name = 'strategy'", replacement)
        } else {
            original.replacen("version = '1.0.0'", replacement, 1)
        };
        fs::write(repo.0.join("strategy/Cargo.toml"), changed).unwrap();
        git_ok(&repo.0, &["add", "."]);
        git_ok(&repo.0, &["commit", "-m", "fix: untracked identity change"]);
        fs::write(repo.0.join("strategy/Cargo.toml"), original).unwrap();
        fs::write(repo.0.join("strategy/Cargo.lock"), strategy_lock(false)).unwrap();
        write_independent_lock_config(&repo);
        let error = build_explicit_release_plan(&repo.0, "chore: migrate", vec![], &SystemProcessRunner).unwrap_err();
        assert!(error.contains("package identity or effective version changed"), "{error}");
    }
}

#[test]
fn independent_historical_lock_migration_does_not_exempt_legacy_managed_roots() {
    let mut legacy = historical_lock_config(true);
    legacy["manifests"] = json!(["native/Cargo.toml", "strategy/Cargo.toml"]);
    let repo = historical_lock_repository("legacy-managed-lock", legacy, true);
    fs::write(repo.0.join("strategy/Cargo.lock"), strategy_lock(false)).unwrap();
    write_independent_lock_config(&repo);
    let error = build_explicit_release_plan(&repo.0, "chore: migrate", vec![], &ReadOnlyReleaseRunner).unwrap_err();
    assert!(error.contains("strategy/Cargo.lock: local package native has missing, ambiguous, or stale version"), "{error}");
}

#[test]
fn independent_historical_lock_migration_requires_proven_legacy_contract() {
    for missing in [true, false] {
        let repo = historical_lock_repository("unproven-legacy-lock", historical_lock_config(true), false);
        if missing {
            fs::remove_file(repo.0.join(".codex/bridgeforge-version.json")).unwrap();
        } else {
            fs::write(repo.0.join(".codex/bridgeforge-version.json"), b"{\"schema_version\":99}").unwrap();
        }
        fs::write(repo.0.join("strategy/Cargo.lock"), strategy_lock(true)).unwrap();
        git_ok(&repo.0, &["add", "."]);
        git_ok(&repo.0, &["commit", "-m", "fix: unproven historical configuration"]);
        fs::write(repo.0.join("strategy/Cargo.lock"), strategy_lock(false)).unwrap();
        write_independent_lock_config(&repo);
        let error = build_explicit_release_plan(&repo.0, "chore: migrate", vec![], &ReadOnlyReleaseRunner).unwrap_err();
        if missing {
            assert!(error.contains("strategy/Cargo.lock: local package native has missing, ambiguous, or stale version"), "{error}");
        } else {
            assert!(error.contains("schema_version=1 or 2"), "{error}");
        }
    }
}

#[test]
fn independent_historical_lock_migration_respects_auto_and_member_manifest_scope() {
    // The automatic legacy contract manages only root manifests, not every
    // nested workspace that independent release later registers.
    let repo = historical_lock_repository("auto-legacy-lock", json!({
        "schema_version":1,"manifest_policy":"auto","release_policy":"explicit_release"
    }), true);
    fs::write(repo.0.join("strategy/Cargo.lock"), strategy_lock(false)).unwrap();
    write_independent_lock_config(&repo);
    assert!(build_explicit_release_plan(&repo.0, "chore: migrate", vec![], &ReadOnlyReleaseRunner).is_ok());

    // Explicitly managing a member also keeps its containing root's lock
    // strict, even when the root manifest itself was not in the legacy list.
    let cfg = rust::RustConfig {
        workspaces: vec!["Cargo.toml".into()], dependency_bump: "patch".into(),
        shared_inputs: vec![], product_inputs: vec![],
    };
    let files = BTreeMap::from([
        ("Cargo.toml", b"[workspace]\nmembers=['member']\n".as_slice()),
        ("member/Cargo.toml", b"[package]\nname='member'\nversion='1.0.0'\n".as_slice()),
        ("Cargo.lock", b"version=4\n[[package]]\nname='member'\nversion='0.9.0'\n".as_slice()),
    ]);
    let inventory = files.keys().map(|p| p.to_string()).collect::<Vec<_>>();
    let mut read = |path: &str| Ok(files.get(path).map(|b| b.to_vec()));
    let error = rust::load_legacy_history(&cfg, &inventory,
        &BTreeSet::from(["member/Cargo.toml".into()]), &mut read).unwrap_err();
    assert!(error.contains("Cargo.lock: local package member has missing, ambiguous, or stale version"), "{error}");
    assert!(rust::load_legacy_history(&cfg, &inventory,
        &BTreeSet::from(["unrelated/Cargo.toml".into()]), &mut read).is_ok());
}

#[test]
fn independent_config_and_package_drift_fail_closed() {
    let repo = independent_repository("independent-drift",false);
    let mut cfg = independent_config();
    cfg["unknown"] = json!(true);
    fs::write(repo.0.join(".codex/bridgeforge-version.json"),serde_json::to_vec(&cfg).unwrap()).unwrap();
    assert!(explicit_release_policy(&repo.0).is_err());
    fs::write(repo.0.join(".codex/bridgeforge-version.json"),serde_json::to_vec(&independent_config()).unwrap()).unwrap();
    fs::write(repo.0.join("core/src/lib.rs"),b"pub fn value() -> u32 { 9 }\n").unwrap();
    let plan = build_explicit_release_plan(&repo.0,"fix: core",vec!["core/src/lib.rs".into()],&SystemProcessRunner).unwrap().unwrap();
    fs::write(repo.0.join("core/src/lib.rs"),b"pub fn value() -> u32 { 10 }\n").unwrap();
    assert!(apply_file_release_plan(&plan).unwrap_err().contains("changed concurrently"));
    assert_eq!(fs::read(repo.0.join("VERSION")).unwrap(),b"1.0.0\n");
    fs::write(repo.0.join("core/src/lib.rs"),b"pub fn value() -> u32 { 9 }\n").unwrap();
    fs::write(repo.0.join("idle/src/lib.rs"),b"pub fn value() -> u32 { 1 }\n").unwrap();
    assert!(apply_file_release_plan(&plan).unwrap_err().contains("status changed concurrently"));
    fs::write(repo.0.join("idle/src/lib.rs"),b"pub fn value() -> u32 { 0 }\n").unwrap();
    fs::write(repo.0.join("unknown.txt"),b"unmapped").unwrap();
    assert!(apply_file_release_plan(&plan).unwrap_err().contains("inventory"));
}

#[test]
fn independent_release_collects_actual_changes_when_caller_omits_paths() {
    let repo = independent_repository("independent-actual-diff",false);
    fs::write(repo.0.join("app/src/lib.rs"),b"pub fn value() -> u32 { 8 }\n").unwrap();
    fs::write(repo.0.join("idle/src/lib.rs"),b"pub fn value() -> u32 { 9 }\n").unwrap();
    let plan = build_explicit_release_plan(&repo.0,"fix: repairs",vec!["app/src/lib.rs".into()],&SystemProcessRunner).unwrap().unwrap();
    assert_eq!(plan.package_changes.len(),2);
    assert!(plan.package_changes.iter().any(|p|p.name=="idle" && p.reason=="direct_change"));
}

#[test]
fn independent_mixed_skeleton_changes_do_not_become_package_changes() {
    for committed in [false,true] {
        let repo = independent_repository("independent-mixed",false);
        fs::write(repo.0.join("managed.txt"),b"updated\n").unwrap();
        let contract_path = repo.0.join(".codex/managed-skeleton.json");
        let mut contract: Value = serde_json::from_slice(&fs::read(&contract_path).unwrap()).unwrap();
        contract["assets"][0]["current_sha256"] = json!(digest(b"updated\n"));
        fs::write(&contract_path,serde_json::to_vec(&contract).unwrap()).unwrap();
        fs::write(repo.0.join("app/src/lib.rs"),b"pub fn value() -> u32 { 12 }\n").unwrap();
        let changed = if committed {
            git_ok(&repo.0,&["add","."]);
            git_ok(&repo.0,&["commit","-m","fix: app and skeleton"]);
            vec![]
        } else { vec!["managed.txt".into(),".codex/managed-skeleton.json".into(),"app/src/lib.rs".into()] };
        let plan = build_explicit_release_plan(&repo.0,"fix: app and skeleton",changed,&SystemProcessRunner).unwrap().unwrap();
        assert_eq!(plan.package_changes.len(),1);
        assert_eq!(plan.package_changes[0].name,"app");
        assert_eq!(plan.package_changes[0].version_after,"1.0.1");
    }
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
