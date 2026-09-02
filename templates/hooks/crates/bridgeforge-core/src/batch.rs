use crate::project_sync::{SyncMode, build_plan};
use crate::{ProcessRequest, ProcessRunner, SystemProcessRunner};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitIdentity {
    pub head: String,
    pub branch: String,
    pub upstream: String,
    pub remote_url: String,
    pub common_dir: String,
    pub dirty: bool,
    pub dirty_fingerprint: String,
    pub ahead: u64,
    pub behind: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FactoryWitness {
    pub git: GitIdentity,
    pub skeleton_fingerprint: String,
    pub batch_controller_blob: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchProject {
    pub order: usize,
    pub project_root: String,
    pub status: String,
    pub fingerprint: Option<String>,
    pub safe_count: usize,
    pub risk_count: usize,
    pub blockers: Vec<String>,
    pub git: Option<GitIdentity>,
    pub skeleton_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub issue_signature: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchPlan {
    pub schema: u32,
    pub status: String,
    pub projects: Vec<BatchProject>,
    pub stopped_at: Option<usize>,
    pub factory: FactoryWitness,
    pub aggregate_fingerprint: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchState {
    pub schema: u32,
    pub batch_id: String,
    pub generation: u32,
    pub status: String,
    pub template_root: String,
    pub created_at: String,
    pub updated_at: String,
    pub projects: Vec<BatchProject>,
    pub current_order: Option<usize>,
    pub common_issue_signature: Option<String>,
    pub factory: FactoryWitness,
    pub confirmed_plan_fingerprint: String,
}

fn sha(value: impl AsRef<[u8]>) -> String {
    format!("sha256:{:x}", Sha256::digest(value.as_ref()))
}

fn git(root: &Path, args: &[&str]) -> Result<String, String> {
    let mut request = ProcessRequest::new("git", root);
    request.args = args.iter().map(OsString::from).collect();
    request.timeout = Duration::from_secs(45);
    let output = SystemProcessRunner
        .run(&request)
        .map_err(|error| format!("failed to launch git: {error}"))?;
    if output.timed_out {
        return Err(format!("git {} timed out", args.join(" ")));
    }
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if output.code == 0 {
        Ok(stdout)
    } else {
        Err(format!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

fn git_identity(root: &Path) -> Result<GitIdentity, String> {
    let head = git(root, &["rev-parse", "HEAD"])?;
    let branch = git(root, &["symbolic-ref", "--short", "HEAD"])?;
    let upstream = git(
        root,
        &["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"],
    )?;
    let remote_url = git(root, &["remote", "get-url", "origin"])?;
    let common_dir = git(
        root,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )?;
    let status = git(root, &["status", "--porcelain=v1", "-uall"])?;
    let counts = git(
        root,
        &["rev-list", "--left-right", "--count", "HEAD...@{u}"],
    )?;
    let mut parts = counts.split_whitespace();
    let ahead = parts
        .next()
        .ok_or_else(|| "git ahead/behind receipt is incomplete".to_string())?
        .parse::<u64>()
        .map_err(|_| "git ahead count is invalid".to_string())?;
    let behind = parts
        .next()
        .ok_or_else(|| "git ahead/behind receipt is incomplete".to_string())?
        .parse::<u64>()
        .map_err(|_| "git behind count is invalid".to_string())?;
    Ok(GitIdentity {
        head,
        branch,
        upstream,
        remote_url,
        common_dir,
        dirty: !status.is_empty(),
        dirty_fingerprint: sha(status.as_bytes()),
        ahead,
        behind,
    })
}

fn validate_official_factory(identity: &GitIdentity) -> Result<(), String> {
    if identity.branch != "main"
        || identity.upstream != "origin/main"
        || identity.remote_url != "https://github.com/freakybridge/BridgeForgeCodex.git"
    {
        return Err(
            "factory must use main, origin/main, and the official BridgeForgeCodex origin URL"
                .into(),
        );
    }
    Ok(())
}

fn skeleton_version(root: &Path) -> Option<String> {
    fs::read_to_string(root.join(".codex/.bridgeforge_codex_version"))
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn factory_witness(root: &Path) -> Result<FactoryWitness, String> {
    let identity = git_identity(root)?;
    let baseline = crate::baseline::verify(root, None, false)?;
    let skeleton_fingerprint = baseline
        .fingerprint
        .ok_or_else(|| "factory baseline did not produce a fingerprint".to_string())?;
    let batch_controller_blob = git(
        root,
        &[
            "rev-parse",
            "HEAD:templates/hooks/crates/bridgeforge-core/src/batch.rs",
        ],
    )?;
    Ok(FactoryWitness {
        git: identity,
        skeleton_fingerprint,
        batch_controller_blob,
    })
}

fn aggregate_fingerprint(factory: &FactoryWitness, projects: &[BatchProject]) -> String {
    sha(serde_json::to_vec(&(factory, projects)).expect("batch receipt must serialize"))
}

fn planned_project(order: usize, root: &Path, template_root: &Path) -> BatchProject {
    let identity = match git_identity(root) {
        Ok(value) => value,
        Err(error) => {
            return BatchProject {
                order,
                project_root: root.display().to_string(),
                status: "blocked".into(),
                fingerprint: None,
                safe_count: 0,
                risk_count: 0,
                blockers: vec![format!("target Git identity is unavailable: {error}")],
                git: None,
                skeleton_version: skeleton_version(root),
                issue_signature: None,
                result: None,
            };
        }
    };
    match build_plan(root, template_root, SyncMode::Auto) {
        Ok(plan) => {
            let blocked = !plan.blockers.is_empty() || !plan.gaps.is_empty();
            let mut blockers = plan.blockers;
            blockers.extend(plan.gaps.into_iter().map(|gap| format!("gap: {gap}")));
            let fingerprint = sha(serde_json::to_vec(&(plan.aggregate_fingerprint, &identity))
                .expect("project receipt must serialize"));
            BatchProject {
                order,
                project_root: root.display().to_string(),
                status: if blocked { "blocked" } else { "planned" }.into(),
                fingerprint: Some(fingerprint),
                safe_count: plan.safe.len(),
                risk_count: plan.risk.len(),
                blockers,
                git: Some(identity),
                skeleton_version: skeleton_version(root),
                issue_signature: None,
                result: None,
            }
        }
        Err(error) => BatchProject {
            order,
            project_root: root.display().to_string(),
            status: "blocked".into(),
            fingerprint: None,
            safe_count: 0,
            risk_count: 0,
            blockers: vec![error],
            git: Some(identity),
            skeleton_version: skeleton_version(root),
            issue_signature: None,
            result: None,
        },
    }
}

pub fn plan(template_root: &Path, roots: &[PathBuf]) -> Result<BatchPlan, String> {
    if roots.is_empty() {
        return Err("batch requires at least one absolute project path".into());
    }
    let template_root = template_root
        .canonicalize()
        .map_err(|error| format!("template root is unavailable: {error}"))?;
    let factory = factory_witness(&template_root)?;
    let mut seen = BTreeSet::new();
    let mut projects = Vec::new();
    let mut stopped_at = None;
    for (index, root) in roots.iter().enumerate() {
        if !root.is_absolute() {
            return Err(format!(
                "batch project path must be absolute: {}",
                root.display()
            ));
        }
        let canonical = root.canonicalize().map_err(|error| {
            format!("batch project is unavailable: {}: {error}", root.display())
        })?;
        let folded = canonical.to_string_lossy().to_lowercase();
        if !seen.insert(folded) {
            return Err(format!("batch project is duplicated: {}", root.display()));
        }
        let project = planned_project(index + 1, &canonical, &template_root);
        if project.status == "blocked" {
            stopped_at = Some(index + 1);
        }
        projects.push(project);
    }
    let aggregate_fingerprint = aggregate_fingerprint(&factory, &projects);
    Ok(BatchPlan {
        schema: 1,
        status: if stopped_at.is_some() {
            "blocked"
        } else {
            "planned"
        }
        .into(),
        projects,
        stopped_at,
        factory,
        aggregate_fingerprint,
    })
}

fn lock_path(template_root: &Path) -> PathBuf {
    template_root.join(".runtime/bridgeforge-codex/batch/active-batch.lock")
}

fn factory_batch_lock(template_root: &Path) -> Result<crate::file_lock::FileLock, String> {
    crate::file_lock::FileLock::acquire(&lock_path(template_root).with_extension("mutex"))
}

fn require_active_owner(state: &BatchState) -> Result<(), String> {
    let owner = fs::read_to_string(lock_path(Path::new(&state.template_root)))
        .map_err(|error| format!("batch active lock is unavailable: {error}"))?;
    if owner != state.batch_id {
        return Err("batch active lock belongs to a different batch".into());
    }
    Ok(())
}

struct StateMutationLock {
    _guard: crate::file_lock::FileLock,
}

impl StateMutationLock {
    fn acquire(state_path: &Path) -> Result<Self, String> {
        let parent = state_path
            .parent()
            .ok_or("batch state path has no parent")?;
        let name = state_path
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or("invalid batch state path")?;
        let path = parent.join(format!(".{name}.mutation.lock"));
        let guard = crate::file_lock::FileLock::acquire(&path).map_err(|error| {
            format!(
                "another batch state mutation is already running or lock is unavailable: {error}"
            )
        })?;
        Ok(Self { _guard: guard })
    }
}

fn write_state(state_path: &Path, state: &BatchState) -> Result<(), String> {
    let parent = state_path
        .parent()
        .ok_or_else(|| "batch state path has no parent".to_string())?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    crate::memory::atomic_write_json(state_path, state).map_err(|error| error.to_string())
}

pub fn load(state_path: &Path) -> Result<BatchState, String> {
    serde_json::from_slice(&fs::read(state_path).map_err(|error| error.to_string())?)
        .map_err(|error| error.to_string())
}

pub fn start(
    state_path: &Path,
    template_root: &Path,
    roots: &[PathBuf],
    confirmed_plan_fingerprint: &str,
) -> Result<BatchState, String> {
    let batch = plan(template_root, roots)?;
    if batch.status != "planned" {
        return Err("batch plan contains blockers".into());
    }
    if batch.aggregate_fingerprint != confirmed_plan_fingerprint {
        return Err("confirmed batch plan fingerprint does not match current preflight".into());
    }
    validate_official_factory(&batch.factory.git)?;
    if batch.factory.git.dirty || batch.factory.git.ahead != 0 || batch.factory.git.behind != 0 {
        return Err("factory must be clean and synchronized before batch start".into());
    }
    let _state_lock = StateMutationLock::acquire(state_path)?;
    let _factory_lock = factory_batch_lock(template_root)?;
    if state_path.exists() {
        return Err("batch state already exists; use a new state path".into());
    }
    let lock = lock_path(template_root);
    if let Some(parent) = lock.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let mut handle = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&lock)
        .map_err(|_| "another batch is already active".to_string())?;
    let now = Utc::now().to_rfc3339();
    let mut digest = Sha256::new();
    digest.update(now.as_bytes());
    for root in roots {
        digest.update(root.as_os_str().to_string_lossy().as_bytes());
    }
    let batch_id = format!("batch-{:x}", digest.finalize());
    handle
        .write_all(batch_id.as_bytes())
        .map_err(|error| error.to_string())?;
    let state = BatchState {
        schema: 1,
        batch_id,
        generation: 1,
        status: "active".into(),
        template_root: template_root
            .canonicalize()
            .map_err(|error| error.to_string())?
            .display()
            .to_string(),
        created_at: now.clone(),
        updated_at: now,
        projects: batch.projects,
        current_order: None,
        common_issue_signature: None,
        factory: batch.factory,
        confirmed_plan_fingerprint: confirmed_plan_fingerprint.to_string(),
    };
    if let Err(error) = write_state(state_path, &state) {
        let _ = fs::remove_file(lock);
        return Err(error);
    }
    Ok(state)
}

fn mutate(
    state_path: &Path,
    change: impl FnOnce(&mut BatchState) -> Result<(), String>,
) -> Result<BatchState, String> {
    let _lock = StateMutationLock::acquire(state_path)?;
    let mut state = load(state_path)?;
    if state.status != "active" {
        return Err(format!("batch is not active: {}", state.status));
    }
    let _factory_lock = factory_batch_lock(Path::new(&state.template_root))?;
    require_active_owner(&state)?;
    change(&mut state)?;
    state.updated_at = Utc::now().to_rfc3339();
    write_state(state_path, &state)?;
    Ok(state)
}

pub fn begin(state_path: &Path) -> Result<BatchState, String> {
    mutate(state_path, |state| {
        if state.current_order.is_some() {
            return Err("a batch project is already running".into());
        }
        if state.common_issue_signature.is_some() {
            return Err("batch is stopped by a common BridgeForge issue".into());
        }
        let current_factory = factory_witness(Path::new(&state.template_root))?;
        if current_factory != state.factory {
            return Err(
                "factory identity or skeleton fingerprint drifted after confirmation".into(),
            );
        }
        let index = state
            .projects
            .iter()
            .position(|project| project.status == "planned")
            .ok_or_else(|| "batch has no pending project".to_string())?;
        let refreshed = planned_project(
            state.projects[index].order,
            Path::new(&state.projects[index].project_root),
            Path::new(&state.template_root),
        );
        if refreshed.status != "planned"
            || refreshed.fingerprint != state.projects[index].fingerprint
        {
            let project = &mut state.projects[index];
            project.status = "deferred".into();
            project.result =
                Some("target identity or project-sync plan drifted after confirmation".into());
            project.blockers = refreshed.blockers;
            project.git = refreshed.git;
            project.skeleton_version = refreshed.skeleton_version;
            return Ok(());
        }
        let project = &mut state.projects[index];
        project.status = "running".into();
        state.current_order = Some(project.order);
        Ok(())
    })
}

pub fn finish(
    state_path: &Path,
    succeeded: bool,
    result: String,
    issue_signature: Option<String>,
) -> Result<BatchState, String> {
    mutate(state_path, |state| {
        let order = state
            .current_order
            .ok_or_else(|| "batch has no running project".to_string())?;
        if let Some(signature) = issue_signature.as_deref()
            && !signature.starts_with("bridgeforge:")
        {
            return Err("issue signature must use the bridgeforge: namespace".into());
        }
        if succeeded {
            let project_root = Path::new(
                &state
                    .projects
                    .iter()
                    .find(|project| project.order == order)
                    .ok_or_else(|| "running project is missing".to_string())?
                    .project_root,
            );
            let identity = git_identity(project_root)?;
            if identity.dirty || identity.ahead != 0 || identity.behind != 0 {
                return Err(
                    "successful batch finish requires a clean synchronized target checkout".into(),
                );
            }
            let factory_version =
                fs::read_to_string(Path::new(&state.template_root).join("VERSION"))
                    .map_err(|error| format!("factory VERSION is unavailable: {error}"))?;
            if skeleton_version(project_root).as_deref() != Some(factory_version.trim()) {
                return Err(
                    "successful batch finish requires the target skeleton version to match the factory"
                        .into(),
                );
            }
            crate::baseline::verify(project_root, None, true).map_err(|error| {
                format!("successful batch finish requires a clean runtime baseline: {error}")
            })?;
        }
        let project = state
            .projects
            .iter_mut()
            .find(|project| project.order == order)
            .ok_or_else(|| "running project is missing".to_string())?;
        project.status = if succeeded { "succeeded" } else { "deferred" }.into();
        project.result = Some(result);
        project.issue_signature = issue_signature.clone();
        state.current_order = None;
        let mut counts = BTreeMap::<String, usize>::new();
        for signature in state
            .projects
            .iter()
            .filter_map(|project| project.issue_signature.as_ref())
        {
            *counts.entry(signature.clone()).or_default() += 1;
        }
        state.common_issue_signature = counts
            .into_iter()
            .find_map(|(signature, count)| (count >= 2).then_some(signature));
        Ok(())
    })
}

pub fn retry(state_path: &Path, order: usize, fingerprint: &str) -> Result<BatchState, String> {
    mutate(state_path, |state| {
        if state.current_order.is_some() {
            return Err("cannot retry while another project is running".into());
        }
        if state.common_issue_signature.is_some() {
            return Err("common BridgeForge issue requires restart with repair evidence".into());
        }
        let template = PathBuf::from(&state.template_root);
        let project = state
            .projects
            .iter_mut()
            .find(|project| project.order == order)
            .ok_or_else(|| format!("unknown batch project order: {order}"))?;
        if project.status != "deferred" {
            return Err("only a deferred project can be retried".into());
        }
        let refreshed = planned_project(order, Path::new(&project.project_root), &template);
        if refreshed.status != "planned" {
            return Err("refreshed project plan is blocked".into());
        }
        if refreshed.fingerprint.as_deref() != Some(fingerprint) {
            return Err("confirmed project plan fingerprint does not match".into());
        }
        *project = refreshed;
        Ok(())
    })
}

pub fn restart(state_path: &Path, bug_doc: &str) -> Result<BatchState, String> {
    mutate(state_path, |state| {
        if state.current_order.is_some() {
            return Err("cannot restart while a project is running".into());
        }
        let signature = state
            .common_issue_signature
            .as_deref()
            .ok_or_else(|| "restart requires a recorded common BridgeForge issue".to_string())?;
        let bug_path = Path::new(bug_doc);
        if bug_path.is_absolute()
            || bug_path.extension().and_then(|value| value.to_str()) != Some("md")
            || !bug_path.starts_with("doc/2_bugs")
            || bug_path
                .components()
                .any(|component| matches!(component, Component::ParentDir))
        {
            return Err("restart --bug-doc must name a relative doc/2_bugs/*.md path".into());
        }
        let template = PathBuf::from(&state.template_root);
        let current_factory = factory_witness(&template)?;
        if current_factory.git.head == state.factory.git.head {
            return Err("restart requires a new committed factory HEAD".into());
        }
        if current_factory.git.dirty
            || current_factory.git.ahead != 0
            || current_factory.git.behind != 0
        {
            return Err("restart requires a clean synchronized factory".into());
        }
        git(&template, &["cat-file", "-e", &format!("HEAD:{bug_doc}")])?;
        if signature.starts_with("bridgeforge:batch-") {
            if current_factory.batch_controller_blob == state.factory.batch_controller_blob {
                return Err(
                    "batch controller witness did not change for the recorded issue".into(),
                );
            }
        } else if current_factory.skeleton_fingerprint == state.factory.skeleton_fingerprint {
            return Err("skeleton fingerprint did not change for the recorded issue".into());
        }
        let refreshed = state
            .projects
            .iter()
            .map(|project| {
                planned_project(project.order, Path::new(&project.project_root), &template)
            })
            .collect::<Vec<_>>();
        if refreshed.iter().any(|project| project.status != "planned") {
            return Err("restart preflight contains blockers".into());
        }
        state.projects = refreshed;
        state.confirmed_plan_fingerprint = aggregate_fingerprint(&current_factory, &state.projects);
        state.factory = current_factory;
        state.generation += 1;
        state.common_issue_signature = None;
        Ok(())
    })
}

pub fn close(state_path: &Path) -> Result<BatchState, String> {
    let _lock = StateMutationLock::acquire(state_path)?;
    let mut state = load(state_path)?;
    let _factory_lock = factory_batch_lock(Path::new(&state.template_root))?;
    let active_lock = lock_path(Path::new(&state.template_root));
    if state.status == "completed" {
        if fs::read_to_string(&active_lock).ok().as_deref() == Some(state.batch_id.as_str()) {
            fs::remove_file(active_lock).map_err(|error| error.to_string())?;
        }
        return Ok(state);
    }
    if state.status != "active" {
        return Err(format!("batch is not active: {}", state.status));
    }
    require_active_owner(&state)?;
    if state.current_order.is_some() {
        return Err("cannot close while a project is running".into());
    }
    if state
        .projects
        .iter()
        .any(|project| project.status != "succeeded")
    {
        return Err("cannot close before every project succeeds".into());
    }
    state.status = "completed".into();
    state.updated_at = Utc::now().to_rfc3339();
    write_state(state_path, &state)?;
    fs::remove_file(active_lock).map_err(|error| error.to_string())?;
    Ok(state)
}

#[cfg(all(test, bridgeforge_factory_tests))]
#[path = "../../../../../scripts/tests/unit/core_batch.rs"]
mod tests;
