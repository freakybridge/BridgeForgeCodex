use super::*;
use std::collections::BTreeMap;

const CONFIG: &str = ".codex/development-checks.json";
const CACHE: &str = ".runtime/bridgeforge-codex/release-preparation";
type Inputs = BTreeMap<String, Option<String>>;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Check {
    id: String,
    program: String,
    args: Vec<String>,
    timeout_seconds: u64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    schema: u32,
    checks: Vec<Check>,
    #[serde(default)]
    audit_required: bool,
    // Explicit project opt-in for narrative records, never executable inputs.
    #[serde(default)]
    record_documents: Vec<String>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Audit {
    schema: u32,
    fingerprint: String,
    reviewer: String,
    verdict: String,
    summary: String,
}
#[derive(Serialize, Deserialize)]
struct CheckResult {
    check: Check,
    exit_code: i32,
    elapsed_ms: u128,
    log_sha256: String,
}
#[derive(Serialize, Deserialize)]
pub(super) struct Prepared {
    schema: u32,
    status: String,
    root: String,
    branch: String,
    git_config_sha256: String,
    inputs: Inputs,
    target_version: String,
    checks: Vec<CheckResult>,
    audit: Option<Audit>,
    artifacts: BTreeMap<String, String>,
}

fn path(root: &Path, relative: &str) -> Result<PathBuf, String> {
    write_plan::join(root, relative)
}

fn read_json<T: for<'a> Deserialize<'a>>(file: &Path) -> Result<T, String> {
    let bytes = fs::read(file).map_err(|e| format!("{}: {e}", file.display()))?;
    let value = crate::baseline::parse_unique_json(&bytes, "release preparation")?;
    serde_json::from_value(value).map_err(|e| e.to_string())
}

fn config(root: &Path) -> Result<Config, String> {
    let cfg: Config = read_json(&path(root, CONFIG)?)?;
    if cfg.schema != 1 || cfg.checks.is_empty() {
        return Err("development checks must use schema=1 and contain checks".into());
    }
    let mut ids = std::collections::BTreeSet::new();
    for check in &cfg.checks {
        if check.id.is_empty()
            || !ids.insert(&check.id)
            || check.program.trim().is_empty()
            || check.timeout_seconds == 0
            || check.timeout_seconds > 7200
        {
            return Err("invalid or duplicated development check".into());
        }
    }
    for relative in &cfg.record_documents {
        let permitted = relative == "doc/README.md"
            || relative == "doc/0_architecture/TODO-INDEX.md"
            || ["doc/1_delivery/", "doc/2_bugs/", "doc/3_reference/", "doc/5_project_knowledgebase/"]
                .iter().any(|prefix| relative.starts_with(prefix));
        if !permitted || relative.contains('\\')
            || relative.split('/').any(|part| matches!(part, "." | "..") || part.contains(['*', '?', ':']))
            || !(relative.ends_with('/') || relative.ends_with(".md"))
            || relative.split('/').any(|part| part.eq_ignore_ascii_case("AGENTS.md") || part.eq_ignore_ascii_case("SKILL.md"))
        {
            return Err(format!("invalid record document scope: {relative}"));
        }
    }
    Ok(cfg)
}

pub(super) fn setup_status(root: &Path) -> serde_json::Value {
    if !root.join("VERSION").exists() {
        return json!({"status":"not-applicable"});
    }
    let file = match path(root, CONFIG) {
        Ok(file) => file,
        Err(error) => return json!({"status":"invalid-config","config":CONFIG,"reason":error}),
    };
    if !file.exists() {
        return json!({"status":"not-configured","config":CONFIG});
    }
    match config(root) {
        Ok(_) => json!({"status":"configured","config":CONFIG}),
        Err(error) => json!({"status":"invalid-config","config":CONFIG,"reason":error}),
    }
}

fn is_record(relative: &str, cfg: &Config) -> bool {
    relative.ends_with(".md")
        && !relative.split('/').any(|part| part.eq_ignore_ascii_case("AGENTS.md") || part.eq_ignore_ascii_case("SKILL.md"))
        && cfg.record_documents.iter().any(|scope|
            if scope.ends_with('/') { relative.starts_with(scope) } else { relative == scope })
}

fn development_inputs(inputs: &Inputs, cfg: &Config) -> Inputs {
    inputs.iter().filter(|(relative, _)| !is_record(relative, cfg))
        .map(|(relative, hash)| (relative.clone(), hash.clone())).collect()
}

fn check_records(root: &Path, inputs: &Inputs, cfg: &Config) -> Result<(), String> {
    if cfg.record_documents.is_empty() { return Ok(()); }
    for relative in inputs.keys().filter(|relative| is_record(relative, cfg)) {
        let bytes = fs::read(path(root, relative)?).map_err(|e| e.to_string())?;
        if bytes.contains(&0) || std::str::from_utf8(&bytes).is_err() {
            return Err(format!("record document must be UTF-8 text: {relative}"));
        }
    }
    let report = crate::project_structure::inspect(root);
    if !report.errors.is_empty() {
        return Err(format!("record document structure check failed: {}", serde_json::to_string(&report.errors).map_err(|e| e.to_string())?));
    }
    Ok(())
}

// Only documented lifecycle fields are metadata. Body/config/dependency changes
// remain part of the fingerprint, including every other Markdown file.
fn normalized(relative: &str, bytes: &[u8]) -> Vec<u8> {
    if !(relative.starts_with("doc/1_delivery/") || relative.starts_with("doc/2_bugs/"))
        || !relative.ends_with(".md")
    {
        return bytes.to_vec();
    }
    let Ok(text) = std::str::from_utf8(bytes) else {
        return bytes.to_vec();
    };
    let text = text.replace("\r\n", "\n");
    let mut lines = text.split_inclusive('\n');
    if lines.next() != Some("---\n") {
        return bytes.to_vec();
    }
    let mut output = "---\n".to_string();
    let mut frontmatter = true;
    for line in lines {
        let trimmed = line.trim_end_matches('\n');
        if frontmatter && trimmed == "---" {
            frontmatter = false;
        }
        if frontmatter {
            if let Some(value) = trimmed.strip_prefix("lifecycle:") {
                if matches!(
                    value.trim(),
                    "active" | "completed" | "superseded" | "archived"
                ) {
                    output.push_str("lifecycle: <record>\n");
                    continue;
                }
            }
            if let Some(value) = trimmed.strip_prefix("validation_status:") {
                if matches!(
                    value.trim(),
                    "not_started"
                        | "in_progress"
                        | "awaiting_validation"
                        | "awaiting_user_acceptance"
                        | "verified"
                ) {
                    output.push_str("validation_status: <record>\n");
                    continue;
                }
            }
        }
        output.push_str(line);
    }
    if frontmatter {
        bytes.to_vec()
    } else {
        output.into_bytes()
    }
}

fn inputs(root: &Path, git: &Git<'_>) -> Result<Inputs, String> {
    collect_inputs(root, git, true)
}

fn raw_inputs(root: &Path, git: &Git<'_>) -> Result<Inputs, String> {
    collect_inputs(root, git, false)
}

fn collect_inputs(root: &Path, git: &Git<'_>, normalize: bool) -> Result<Inputs, String> {
    let mut request = ProcessRequest::new("git", root);
    request.args = [
        "ls-files",
        "--cached",
        "--others",
        "--exclude-standard",
        "-z",
    ]
    .iter()
    .map(OsString::from)
    .collect();
    request.timeout = Duration::from_secs(45);
    let output = git.runner.run(&request).map_err(|e| e.to_string())?;
    if output.timed_out || output.code != 0 {
        return Err("cannot enumerate development inputs".into());
    }
    let files = String::from_utf8(output.stdout).map_err(|e| e.to_string())?;
    let mut inputs = BTreeMap::new();
    for relative in files.split('\0').filter(|s| !s.is_empty()) {
        if relative.starts_with(".runtime/") {
            return Err(
                "release preparation requires .runtime to remain untracked and ignored".into(),
            );
        }
        let file = path(root, relative)?;
        let payload = match fs::read(&file) {
            Ok(bytes) => Some(payload_sha(&if normalize { normalized(relative, &bytes) } else { bytes })),
            // A deletion has the same content state before and after git add.
            // Reappearance is still detected as a new key in the inventory.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(format!("cannot fingerprint {relative}: {e}")),
        };
        inputs.insert(relative.into(), payload);
    }
    Ok(inputs)
}

fn fingerprint(inputs: &Inputs) -> Result<String, String> {
    serde_json::to_vec(inputs)
        .map(|b| payload_sha(&b))
        .map_err(|e| e.to_string())
}

fn blob(root: &Path, hash: &str) -> Result<PathBuf, String> {
    let suffix = hash
        .strip_prefix("sha256:")
        .ok_or("invalid preparation blob hash")?;
    if suffix.len() != 64 || !suffix.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err("invalid preparation blob hash".into());
    }
    path(root, &format!("{CACHE}/blobs/{suffix}"))
}

fn save_blob(root: &Path, bytes: &[u8]) -> Result<String, String> {
    let hash = payload_sha(bytes);
    crate::persistence::atomic_write(&blob(root, &hash)?, bytes).map_err(|e| e.to_string())?;
    Ok(hash)
}

fn load_blob(root: &Path, hash: &str) -> Result<Vec<u8>, String> {
    let bytes =
        fs::read(blob(root, hash)?).map_err(|e| format!("prepared artifact missing: {e}"))?;
    if payload_sha(&bytes) != hash {
        return Err("prepared artifact hash mismatch".into());
    }
    Ok(bytes)
}

fn validate_audit(cfg: &Config, audit: Option<&Audit>, key: &str) -> Result<(), String> {
    match audit {
        None if cfg.audit_required => {
            Err("matching independent audit record is required before preparation".into())
        }
        Some(a)
            if a.schema != 1
                || a.fingerprint != key
                || a.reviewer.trim().is_empty()
                || a.verdict != "passed"
                || a.summary.trim().is_empty() =>
        {
            Err("audit record does not match current inputs or has not passed".into())
        }
        _ => Ok(()),
    }
}

pub(super) fn status(root: &Path, runner: &dyn ProcessRunner) -> Result<serde_json::Value, String> {
    let setup = setup_status(root);
    if setup["status"] == "not-applicable" {
        return Ok(json!({"schema":1,"status":"not-applicable","setup":setup}));
    }
    let mut blockers = Vec::new();
    match setup["status"].as_str() {
        Some("not-configured") => blockers.push(json!({
            "code":"development-checks-missing", "path":CONFIG,
            "message":"独立开发验证尚未配置检查清单；Git 同步和升版不依赖该清单"
        })),
        Some("invalid-config") => blockers.push(json!({
            "code":"development-checks-invalid", "path":CONFIG,
            "message":"独立开发检查配置无效，不会自动覆盖或降级为空检查；不阻断 Git 同步和升版",
            "reason":setup["reason"]
        })),
        _ => (),
    }
    if let Err(error) = crate::release::validate_release_baseline(root, runner) {
        blockers.push(json!({
            "code":"release-baseline-blocked",
            "message":"发布版本基线无法通过验证；先核对 VERSION、原生版本文件和已提交发布历史",
            "reason":error
        }));
    }
    if !blockers.is_empty() {
        return Ok(json!({
            "schema":1,
            "status":if setup["status"] == "configured" { "blocked" } else { "setup-required" },
            "setup":setup,"blockers":blockers,
            "next_step":"需要独立开发验证时按项目约定配置并执行检查；Git 同步及升版不需要该验证记录"
        }));
    }
    let git = Git { root, runner };
    let current = inputs(root, &git)?;
    let cfg = config(root)?;
    let key = fingerprint(&development_inputs(&current, &cfg))?;
    let saved = read_json::<Prepared>(&path(root, &format!("{CACHE}/current.json"))?);
    let mut result = json!({"schema":1,"validation_fingerprint":key,"status":"not-prepared"});
    if let Ok(record) = saved {
        match record.validate(root, &git) {
            Ok(_) => {
                result["status"] = json!("prepared");
                result["target_version"] = json!(record.target_version);
                result["record_documents_changed"] = json!(current != record.inputs);
                result["record_documents_check"] = json!(if cfg.record_documents.is_empty() { "not-configured" } else { "passed" });
            }
            Err(error) => {
                result["status"] = json!("stale");
                result["reason"] = json!(error);
            }
        }
    }
    Ok(result)
}

impl Prepared {
    pub(super) fn validate(&self, root: &Path, git: &Git<'_>) -> Result<Inputs, String> {
        if self.schema != 1
            || self.status != "prepared"
            || self.root
                != root
                    .canonicalize()
                    .map_err(|e| e.to_string())?
                    .to_string_lossy()
            || self.branch
                != git.required(
                    &["symbolic-ref", "--quiet", "HEAD"],
                    Duration::from_secs(30),
                )?
        {
            return Err(
                "release preparation belongs to another repository/branch or is incomplete".into(),
            );
        }
        let cfg = config(root)?;
        let exact = raw_inputs(root, git)?;
        let current = inputs(root, git)?;
        if development_inputs(&current, &cfg) != development_inputs(&self.inputs, &cfg) {
            return Err("development inputs changed; return to develop to prepare again".into());
        }
        check_records(root, &current, &cfg)?;
        if RepositoryIdentity::capture(git)?.common_config_sha256 != self.git_config_sha256 {
            return Err("Git configuration changed since development preparation".into());
        }
        if cfg.checks.len() != self.checks.len() {
            return Err("development check records are incomplete".into());
        }
        for (expected, actual) in cfg.checks.iter().zip(&self.checks) {
            if serde_json::to_value(expected).unwrap()
                != serde_json::to_value(&actual.check).unwrap()
                || actual.exit_code != 0
            {
                return Err("development check record mismatch".into());
            }
            load_blob(root, &actual.log_sha256)?;
        }
        validate_audit(&cfg, self.audit.as_ref(), &fingerprint(&development_inputs(&self.inputs, &cfg))?)?;
        self.target_version.parse::<crate::release::SemVer>()?;
        if root.join("templates/managed-skeleton.json").is_file() {
            let contract: serde_json::Value =
                read_json(&path(root, "templates/managed-skeleton.json")?)?;
            let platform = if cfg!(windows) {
                "windows-x86_64"
            } else if cfg!(target_os = "linux") {
                "linux-x86_64"
            } else {
                "macos-x86_64"
            };
            let mut expected = std::collections::BTreeSet::new();
            for item in contract["generated_assets"]
                .as_array()
                .ok_or("missing factory generated contract")?
            {
                expected.insert(
                    item["binary_targets"][platform]
                        .as_str()
                        .ok_or("missing prepared binary target")?
                        .to_string(),
                );
                expected.insert(
                    item["receipt_target"]
                        .as_str()
                        .ok_or("missing prepared receipt target")?
                        .to_string(),
                );
            }
            if expected != self.artifacts.keys().cloned().collect() {
                return Err("prepared artifact inventory is incomplete".into());
            }
        } else if !self.artifacts.is_empty() {
            return Err("unexpected artifacts for non-factory preparation".into());
        }
        for hash in self.artifacts.values() {
            load_blob(root, hash)?;
        }
        // Capture the full inventory, including current record documents, for
        // subsequent write/index/commit guards. This does not certify semantics.
        if raw_inputs(root, git)? != exact {
            return Err("inputs changed while checking release preparation".into());
        }
        Ok(exact)
    }

}

pub(super) fn prepare(
    root: &Path,
    runner: &dyn ProcessRunner,
    message: &str,
    audit_file: Option<&Path>,
) -> Result<serde_json::Value, String> {
    let git = Git { root, runner };
    let _lock = SyncLock::acquire(&git)?;
    let identity = RepositoryIdentity::capture(&git)?;
    let index_state = git.required(
        &["ls-files", "--stage", "-v", "-z"],
        Duration::from_secs(30),
    )?;
    let _project_lock = crate::project_sync::ProjectLock::acquire(root)?;
    git.required(
        &["check-ignore", &format!("{CACHE}/current.json")],
        Duration::from_secs(30),
    )?;
    let cfg = config(root)?;
    let source = inputs(root, &git)?;
    check_records(root, &source, &cfg)?;
    let key = fingerprint(&development_inputs(&source, &cfg))?;
    let audit: Option<Audit> = audit_file.map(read_json).transpose()?;
    validate_audit(&cfg, audit.as_ref(), &key)?;
    let Some(release) =
        crate::release::build_explicit_release_plan(root, message, git.changed_paths()?, runner)?
    else {
        return Ok(json!({"schema":1,"status":"nothing-to-release","validation_fingerprint":key}));
    };
    let target_version = release.new_version.to_string();
    let current = path(root, &format!("{CACHE}/current.json"))?;
    crate::persistence::atomic_write_json(&current, &json!({"schema":1,"status":"preparing"}))
        .map_err(|e| e.to_string())?;
    let mut checks = Vec::new();
    for check in cfg.checks {
        let program = if check.program == "{bridgeforge}" {
            root.join(if cfg!(windows) {
                ".codex/bin/bridgeforge.exe"
            } else {
                ".codex/bin/bridgeforge"
            })
            .into_os_string()
        } else {
            OsString::from(&check.program)
        };
        let mut request = ProcessRequest::new(program, root);
        request.args = check.args.iter().map(OsString::from).collect();
        request.timeout = Duration::from_secs(check.timeout_seconds);
        let started = std::time::Instant::now();
        let output = runner.run(&request).map_err(|e| e.to_string())?;
        let mut log = output.stdout;
        log.extend_from_slice(b"\n--- stderr ---\n");
        log.extend_from_slice(&output.stderr);
        let hash = save_blob(root, &log)?;
        if output.timed_out || output.code != 0 {
            return Err(format!(
                "development check {} failed (code={}, timeout={}); log={hash}",
                check.id, output.code, output.timed_out
            ));
        }
        checks.push(CheckResult {
            check,
            exit_code: output.code,
            elapsed_ms: started.elapsed().as_millis(),
            log_sha256: hash,
        });
        if inputs(root, &git)? != source {
            return Err("development checks changed inputs; no preparation was certified".into());
        }
        if RepositoryIdentity::capture(&git)? != identity
            || git.required(
                &["ls-files", "--stage", "-v", "-z"],
                Duration::from_secs(30),
            )? != index_state
        {
            return Err("development check changed Git state; no preparation was certified".into());
        }
    }
    let factory = root.join("templates/managed-skeleton.json").is_file();
    let plan =
        write_plan::WritePlan::prepare(root, release.writes, release.inputs, factory, runner)?;
    let mut artifacts = BTreeMap::new();
    if factory {
        let contract: serde_json::Value =
            serde_json::from_slice(&plan.writes[&root.join("templates/managed-skeleton.json")])
                .map_err(|e| e.to_string())?;
        let platform = if cfg!(windows) {
            "windows-x86_64"
        } else if cfg!(target_os = "linux") {
            "linux-x86_64"
        } else {
            "macos-x86_64"
        };
        for item in contract["generated_assets"]
            .as_array()
            .ok_or("missing prepared assets")?
        {
            for relative in [
                item["binary_targets"][platform].as_str(),
                item["receipt_target"].as_str(),
            ] {
                let relative = relative.ok_or("missing prepared asset target")?;
                let file = path(root, relative)?;
                let bytes = match plan.writes.get(&file) {
                    Some(b) => b.clone(),
                    None => fs::read(file).map_err(|e| e.to_string())?,
                };
                artifacts.insert(relative.to_string(), save_blob(root, &bytes)?);
            }
        }
    }
    plan.verify_unchanged(root)?;
    if inputs(root, &git)? != source
        || RepositoryIdentity::capture(&git)? != identity
        || git.required(
            &["ls-files", "--stage", "-v", "-z"],
            Duration::from_secs(30),
        )? != index_state
    {
        return Err("repository changed during preparation; not certified".into());
    }
    let record = Prepared {
        schema: 1,
        status: "prepared".into(),
        root: root
            .canonicalize()
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .into(),
        branch: identity.symbolic_head,
        git_config_sha256: identity.common_config_sha256,
        inputs: source,
        target_version: target_version.clone(),
        checks,
        audit,
        artifacts,
    };
    let check_count = record.checks.len();
    crate::persistence::atomic_write_json(
        &current,
        &serde_json::to_value(record).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    Ok(
        json!({"schema":1,"status":"prepared","target_version":target_version,"validation_fingerprint":key,"checks_passed":check_count,"record":current}),
    )
}
