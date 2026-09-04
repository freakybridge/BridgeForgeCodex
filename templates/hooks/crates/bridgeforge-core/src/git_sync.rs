use crate::{CommandOutcome, ProcessRequest, ProcessRunner};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[path = "git_sync_plan.rs"]
mod write_plan;

#[derive(Clone, Debug, Default)]
pub struct GitSyncOptions {
    pub message: Option<String>,
    pub message_file: Option<PathBuf>,
    pub remote: String,
    pub skip_fetch: bool,
    pub skip_push: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitSyncReceipt {
    pub schema: u32,
    pub status: String,
    pub commit: Option<String>,
    pub push_target: Option<String>,
    pub push_performed: bool,
    pub working_tree: String,
    pub ahead: u64,
    pub behind: u64,
    pub autostash_retained: bool,
}

struct Git<'a> {
    root: &'a Path,
    runner: &'a dyn ProcessRunner,
}

impl<'a> Git<'a> {
    fn run_raw(&self, args: &[&str], timeout: Duration) -> Result<(i32, String, String), String> {
        let mut request = ProcessRequest::new("git", self.root);
        request.args = args.iter().map(OsString::from).collect();
        request.timeout = timeout;
        let output = self
            .runner
            .run(&request)
            .map_err(|error| format!("cannot launch git: {error}"))?;
        if output.timed_out {
            return Err(format!("git {} timed out", args.join(" ")));
        }
        Ok((
            output.code,
            String::from_utf8_lossy(&output.stdout).trim().to_string(),
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ))
    }

    fn required(&self, args: &[&str], timeout: Duration) -> Result<String, String> {
        let (code, stdout, stderr) = self.run_raw(args, timeout)?;
        if code == 0 {
            Ok(stdout)
        } else {
            Err(format!(
                "git {} failed: {}",
                args.join(" "),
                if stderr.is_empty() { stdout } else { stderr }
            ))
        }
    }

    fn status(&self) -> Result<String, String> {
        self.required(
            &["status", "--porcelain=v1", "-uall"],
            Duration::from_secs(45),
        )
    }

    fn ahead_behind(&self) -> Result<(u64, u64), String> {
        self.ahead_behind_target("@{u}")
    }

    fn ahead_behind_target(&self, target: &str) -> Result<(u64, u64), String> {
        let range = format!("HEAD...{target}");
        let value = self.required(
            &["rev-list", "--left-right", "--count", &range],
            Duration::from_secs(45),
        )?;
        let parts = value.split_whitespace().collect::<Vec<_>>();
        if parts.len() != 2 {
            return Err(format!("unexpected ahead/behind output: {value:?}"));
        }
        Ok((
            parts[0]
                .parse()
                .map_err(|_| format!("invalid ahead count: {}", parts[0]))?,
            parts[1]
                .parse()
                .map_err(|_| format!("invalid behind count: {}", parts[1]))?,
        ))
    }

    fn changed_paths(&self) -> Result<Vec<String>, String> {
        let mut paths = Vec::new();
        for args in [
            &["diff", "--name-only"][..],
            &["diff", "--cached", "--name-only"][..],
            &["ls-files", "--others", "--exclude-standard"][..],
        ] {
            let output = self.required(args, Duration::from_secs(45))?;
            paths.extend(
                output
                    .lines()
                    .map(|line| line.trim().replace('\\', "/"))
                    .filter(|line| !line.is_empty()),
            );
        }
        paths.sort();
        paths.dedup();
        Ok(paths)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RepositoryIdentity {
    git_dir: String,
    common_dir: String,
    index_path: PathBuf,
    symbolic_head: String,
    head_oid: String,
    common_config_sha256: String,
}

fn payload_sha(payload: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(payload))
}

fn verify_split_index_disabled(shared_index_path: &str, configured: &str) -> Result<(), String> {
    if !shared_index_path.is_empty() {
        return Err("split index is not supported by transactional git-sync".into());
    }
    if configured == "true" {
        return Err("core.splitIndex=true is not supported by transactional git-sync".into());
    }
    Ok(())
}

impl RepositoryIdentity {
    fn capture(git: &Git<'_>) -> Result<Self, String> {
        let git_dir = git.required(
            &["rev-parse", "--path-format=absolute", "--git-dir"],
            Duration::from_secs(30),
        )?;
        let common_dir = git.required(
            &["rev-parse", "--path-format=absolute", "--git-common-dir"],
            Duration::from_secs(30),
        )?;
        let index_path = PathBuf::from(git.required(
            &["rev-parse", "--path-format=absolute", "--git-path", "index"],
            Duration::from_secs(30),
        )?);
        let split = git.required(
            &["rev-parse", "--shared-index-path"],
            Duration::from_secs(30),
        )?;
        let split_enabled = git
            .required(
                &["config", "--bool", "--get", "core.splitIndex"],
                Duration::from_secs(30),
            )
            .unwrap_or_else(|_| "false".into());
        verify_split_index_disabled(&split, &split_enabled)?;
        let bare = git
            .required(
                &["config", "--bool", "--get", "core.bare"],
                Duration::from_secs(30),
            )
            .unwrap_or_else(|_| "false".into());
        if bare == "true" {
            return Err("core.bare=true repositories are not supported by git-sync".into());
        }
        let symbolic_head =
            git.required(&["symbolic-ref", "-q", "HEAD"], Duration::from_secs(30))?;
        let head_oid = git.required(&["rev-parse", "HEAD"], Duration::from_secs(30))?;
        let common_config_sha256 = payload_sha(
            &fs::read(Path::new(&common_dir).join("config"))
                .map_err(|error| format!("cannot read common Git config: {error}"))?,
        );
        Ok(Self {
            git_dir,
            common_dir,
            index_path,
            symbolic_head,
            head_oid,
            common_config_sha256,
        })
    }

    fn same_layout_and_config(&self, other: &Self) -> bool {
        self.git_dir == other.git_dir
            && self.common_dir == other.common_dir
            && self.index_path == other.index_path
            && self.symbolic_head == other.symbolic_head
            && self.common_config_sha256 == other.common_config_sha256
    }
}

struct SyncLock {
    _guard: crate::file_lock::FileLock,
}

impl SyncLock {
    fn acquire(git: &Git<'_>) -> Result<Self, String> {
        let raw = git.required(
            &["rev-parse", "--path-format=absolute", "--git-common-dir"],
            Duration::from_secs(30),
        )?;
        if raw.is_empty() || !Path::new(&raw).is_absolute() {
            return Err("Git common directory is not an absolute path".into());
        }
        let path = PathBuf::from(raw).join("bridgeforge-git-sync.lock");
        let guard = crate::file_lock::FileLock::acquire(&path).map_err(|error| {
            format!(
                "another bridgeforge git-sync is already running or lock is unavailable: {error}"
            )
        })?;
        Ok(Self { _guard: guard })
    }
}

fn restore_snapshots(
    root: &Path,
    snapshots: Vec<(PathBuf, Option<Vec<u8>>, bool)>,
) -> Result<(), String> {
    let mut failures = Vec::new();
    for (path, before, binary) in snapshots.into_iter().rev() {
        let result = match before {
            Some(payload) if binary => crate::runtime::write_binary(root, &path, &payload),
            Some(payload) => {
                crate::memory::atomic_write(&path, &payload).map_err(|error| error.to_string())
            }
            None if path.exists() => fs::remove_file(&path).map_err(|error| error.to_string()),
            None => Ok(()),
        };
        if let Err(error) = result {
            failures.push(format!("{}: {error}", path.display()));
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join("; "))
    }
}

#[derive(Clone, Debug)]
struct FileSnapshot {
    binary: bool,
    path: PathBuf,
    before: Option<Vec<u8>>,
    planned: Vec<Option<Vec<u8>>>,
}

impl FileSnapshot {
    fn permits(&self, actual: &Option<Vec<u8>>) -> bool {
        actual == &self.before || self.planned.iter().any(|value| value == actual)
    }
}

fn restore_snapshots_guarded(
    git: &Git<'_>,
    identity: &RepositoryIdentity,
    expected_index: &[u8],
    original_index: Vec<u8>,
    snapshots: Vec<FileSnapshot>,
) -> Result<(), String> {
    let current = RepositoryIdentity::capture(git)?;
    if &current != identity {
        return Err(
            "HIGH: repository identity drift detected; no automatic repository recovery".into(),
        );
    }
    let actual_index = fs::read(&identity.index_path)
        .map_err(|error| format!("cannot verify current Git index: {error}"))?;
    if actual_index != expected_index {
        return Err("HIGH: Git index changed concurrently; no automatic index recovery".into());
    }
    for snapshot in &snapshots {
        let actual = fs::read(&snapshot.path).ok();
        if !snapshot.permits(&actual) {
            return Err(format!(
                "HIGH: automatic target changed concurrently; no automatic recovery: {}",
                snapshot.path.display()
            ));
        }
    }
    let mut restore = snapshots
        .into_iter()
        .map(|snapshot| (snapshot.path, snapshot.before, snapshot.binary))
        .collect::<Vec<_>>();
    restore.push((identity.index_path.clone(), Some(original_index), false));
    restore_snapshots(git.root, restore)
}

fn receipt(
    status: &str,
    commit: Option<String>,
    push_target: Option<String>,
    pushed: bool,
    dirty: bool,
    ahead: u64,
    behind: u64,
    autostash_retained: bool,
) -> GitSyncReceipt {
    GitSyncReceipt {
        schema: 1,
        status: status.into(),
        commit,
        push_target,
        push_performed: pushed,
        working_tree: if dirty { "dirty" } else { "clean" }.into(),
        ahead,
        behind,
        autostash_retained,
    }
}

pub fn sync(
    root: &Path,
    runner: &dyn ProcessRunner,
    mut options: GitSyncOptions,
) -> CommandOutcome {
    if options.remote.is_empty() {
        options.remote = "origin".into();
    }
    let git = Git { root, runner };
    let blocked =
        |message: String| CommandOutcome::blocked(format!("[git-sync] BLOCKED: {message}\n"));
    let git_dir = match git.required(&["rev-parse", "--git-dir"], Duration::from_secs(30)) {
        Ok(value) => value,
        Err(error) => return blocked(error),
    };
    if git_dir.is_empty() {
        return blocked("not a Git repository".into());
    }
    let sync_lock = match SyncLock::acquire(&git) {
        Ok(value) => value,
        Err(error) => return blocked(error),
    };
    let push_target = match git.required(
        &[
            "rev-parse",
            "--abbrev-ref",
            "--symbolic-full-name",
            "@{push}",
        ],
        Duration::from_secs(30),
    ) {
        Ok(value) => value,
        Err(_) => {
            return blocked("no push target; configure the current branch before git-sync".into());
        }
    };
    let upstream = match git.required(
        &["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"],
        Duration::from_secs(30),
    ) {
        Ok(value) => value,
        Err(_) => {
            return blocked("no upstream branch; set upstream before running git-sync".into());
        }
    };
    let push_comparison = if push_target == upstream {
        "@{u}"
    } else {
        "@{push}"
    };
    if push_target != upstream && !options.skip_fetch {
        let branch = match git.required(
            &["symbolic-ref", "--quiet", "HEAD"],
            Duration::from_secs(30),
        ) {
            Ok(value) => value,
            Err(error) => return blocked(error),
        };
        let remote = match git.required(
            &["for-each-ref", "--format=%(push:remotename)", &branch],
            Duration::from_secs(30),
        ) {
            Ok(value) if !value.is_empty() && !value.contains('\n') => value,
            _ => return blocked("cannot resolve the distinct push remote".into()),
        };
        if remote != options.remote
            && let Err(error) = git.required(&["fetch", &remote], Duration::from_secs(180))
        {
            return blocked(error);
        }
    }
    let mut dirty = match git.status() {
        Ok(value) => !value.is_empty(),
        Err(error) => return blocked(error),
    };
    if !options.skip_fetch
        && let Err(error) = git.required(&["fetch", &options.remote], Duration::from_secs(180))
    {
        return blocked(error);
    }
    let (mut ahead, mut behind) = match git.ahead_behind() {
        Ok(value) => value,
        Err(error) => return blocked(error),
    };
    if ahead > 0 && behind > 0 {
        return CommandOutcome {
            code: 2,
            receipt: Some(json!(receipt(
                "diverged",
                None,
                Some(push_target),
                false,
                dirty,
                ahead,
                behind,
                false
            ))),
            stderr: "[git-sync] branch diverged; manual decision required\n".into(),
            ..CommandOutcome::default()
        };
    }
    let mut autostashed = false;
    let mut transaction_identity = None;
    if behind > 0 {
        if dirty {
            match git.required(
                &["stash", "push", "-u", "-m", "codex_git_sync_autostash"],
                Duration::from_secs(180),
            ) {
                Ok(value) => autostashed = !value.contains("No local changes to save"),
                Err(error) => return blocked(error),
            }
        }
        if let Err(error) = git.required(&["pull", "--ff-only"], Duration::from_secs(180)) {
            let mut outcome = blocked(error);
            outcome.receipt = Some(json!(receipt(
                "pull-failed",
                None,
                Some(push_target),
                false,
                dirty,
                ahead,
                behind,
                autostashed
            )));
            return outcome;
        }
        if autostashed {
            let (code, stdout, stderr) =
                match git.run_raw(&["stash", "pop"], Duration::from_secs(180)) {
                    Ok(value) => value,
                    Err(error) => return blocked(error),
                };
            if code != 0 {
                let detail = if stderr.is_empty() { stdout } else { stderr };
                let mut outcome = blocked(format!(
                    "git stash pop failed; stash is kept for manual recovery: {detail}"
                ));
                outcome.receipt = Some(json!(receipt(
                    "autostash-retained",
                    None,
                    Some(push_target),
                    false,
                    true,
                    ahead,
                    behind,
                    true
                )));
                return outcome;
            }
            autostashed = false;
        }
        let (pulled_ahead, pulled_behind) = match git.ahead_behind() {
            Ok(value) => value,
            Err(error) => return blocked(error),
        };
        if pulled_ahead > 0 && pulled_behind > 0 {
            return blocked("branch diverged after pull; manual decision required".into());
        }
        dirty = match git.status() {
            Ok(value) => !value.is_empty(),
            Err(error) => return blocked(error),
        };
    }
    let factory = root.join("templates/managed-skeleton.json").is_file();
    let factory_runtime_repair = if factory && !dirty {
        if let Err(error) = crate::baseline::verify(root, None, false) {
            return blocked(format!("current baseline blocked git-sync: {error}"));
        }
        crate::baseline::verify(root, None, true).is_err()
    } else {
        false
    };
    if dirty || factory_runtime_repair {
        let identity = match RepositoryIdentity::capture(&git) {
            Ok(value) => value,
            Err(error) => return blocked(error),
        };
        let original_index = match fs::read(&identity.index_path) {
            Ok(value) => value,
            Err(error) => return blocked(format!("cannot snapshot Git index: {error}")),
        };
        let adaptation_path = root.join(".runtime/bridgeforge-codex/explicit-adaptation.json");
        let adaptation_before = fs::read(&adaptation_path).ok();
        transaction_identity = Some(identity.clone());
        let (message, release) = if dirty {
            let message = match (&options.message_file, &options.message) {
                (Some(path), _) => match fs::read_to_string(path) {
                    Ok(value) if !value.trim().is_empty() => value.trim().to_string(),
                    Ok(_) => return blocked("commit message is empty".into()),
                    Err(error) => {
                        return blocked(format!("cannot read commit message: {error}"));
                    }
                },
                (None, Some(value)) if !value.trim().is_empty() => value.trim().to_string(),
                _ => return blocked("commit message is required when real changes exist".into()),
            };
            let changed_paths = match git.changed_paths() {
                Ok(value) => value,
                Err(error) => return blocked(error),
            };
            let release = match crate::release::build_file_release_plan(
                root,
                &message,
                changed_paths,
                runner,
            ) {
                Ok(value) => value,
                Err(error) => {
                    return blocked(format!("automatic release planning failed: {error}"));
                }
            };
            (message, release)
        } else {
            (String::new(), None)
        };
        let _project_lock = if factory {
            match crate::project_sync::ProjectLock::acquire(root) {
                Ok(lock) => Some(lock),
                Err(error) => return blocked(error),
            }
        } else {
            None
        };
        if factory {
            if let Err(error) = git.required(
                &[
                    "check-ignore",
                    ".runtime/bridgeforge-codex/git-sync-images/probe",
                ],
                Duration::from_secs(30),
            ) {
                return blocked(format!(
                    "runtime image cache must be ignored before sync: {error}"
                ));
            }
            if let Err(error) = crate::runtime::cleanup_images(root) {
                return blocked(error);
            }
        }
        let (release_inputs, release_writes) =
            release.map(|p| (p.inputs, p.writes)).unwrap_or_default();
        let plan = match write_plan::WritePlan::prepare(
            root,
            release_writes,
            release_inputs,
            factory,
            runner,
        ) {
            Ok(plan) => plan,
            Err(error) => {
                return blocked(format!(
                    "automatic write planning failed before apply: {error}"
                ));
            }
        };
        if RepositoryIdentity::capture(&git).as_ref() != Ok(&identity)
            || fs::read(&identity.index_path).ok().as_ref() != Some(&original_index)
        {
            return blocked(
                "repository or index changed during automatic write planning; no apply".into(),
            );
        }
        if let Err(error) = plan.verify_unchanged(root) {
            return blocked(error);
        }
        let snapshots = plan
            .writes
            .iter()
            .map(|(path, payload)| FileSnapshot {
                path: path.clone(),
                before: plan.before.get(path).cloned().unwrap_or(None),
                planned: vec![Some(payload.clone())],
                binary: plan.binaries.contains(path),
            })
            .collect::<Vec<_>>();
        let fail = |message: String, expected_index: &[u8], snapshots: Vec<FileSnapshot>| {
            let detail = match restore_snapshots_guarded(
                &git,
                &identity,
                expected_index,
                original_index.clone(),
                snapshots,
            ) {
                Ok(()) => format!("{message}; automatic release and Git index were rolled back"),
                Err(error) => format!("{message}; rollback also failed: {error}"),
            };
            blocked(detail)
        };
        for (path, payload) in &plan.writes {
            let result = if plan.binaries.contains(path) {
                crate::runtime::write_binary(root, path, payload)
            } else {
                crate::memory::atomic_write(path, payload).map_err(|e| e.to_string())
            };
            if let Err(error) = result {
                return fail(
                    format!("automatic write apply failed: {error}"),
                    &original_index,
                    snapshots,
                );
            }
        }
        if let Err(error) = crate::baseline::verify(root, None, true) {
            return fail(
                format!("current baseline blocked git-sync: {error}"),
                &original_index,
                snapshots,
            );
        }
        if let Err(error) = git.required(&["add", "."], Duration::from_secs(120)) {
            return fail(error, &original_index, snapshots);
        }
        let post_add_index = match fs::read(&identity.index_path) {
            Ok(value) => value,
            Err(error) => {
                return fail(
                    format!("cannot verify staged Git index: {error}"),
                    &original_index,
                    snapshots,
                );
            }
        };
        let post_add_semantic =
            match git.required(&["ls-files", "--stage", "-v"], Duration::from_secs(30)) {
                Ok(value) => value,
                Err(error) => return fail(error, &post_add_index, snapshots),
            };
        let (code, _, _) =
            match git.run_raw(&["diff", "--cached", "--quiet"], Duration::from_secs(30)) {
                Ok(value) => value,
                Err(error) => return fail(error, &post_add_index, snapshots),
            };
        if code == 1
            && let Err(error) = git.required(&["commit", "-m", &message], Duration::from_secs(180))
        {
            let current_semantic =
                match git.required(&["ls-files", "--stage", "-v"], Duration::from_secs(30)) {
                    Ok(value) => value,
                    Err(verify_error) => {
                        return blocked(format!(
                            "{error}; cannot verify failed-commit index state: {verify_error}"
                        ));
                    }
                };
            if current_semantic != post_add_semantic {
                return blocked(format!(
                    "{error}; HIGH: Git index changed logically during failed commit; no automatic index recovery"
                ));
            }
            let current_index = match fs::read(&identity.index_path) {
                Ok(value) => value,
                Err(verify_error) => {
                    return blocked(format!(
                        "{error}; cannot read failed-commit Git index: {verify_error}"
                    ));
                }
            };
            return fail(error, &current_index, snapshots);
        }
        if code == 1 {
            let committed_parent =
                match git.required(&["rev-parse", "HEAD^"], Duration::from_secs(30)) {
                    Ok(value) => value,
                    Err(error) => {
                        return blocked(format!(
                            "commit succeeded but its parent could not be verified: {error}"
                        ));
                    }
                };
            if committed_parent != identity.head_oid {
                return blocked(
                    "HIGH: HEAD changed unexpectedly during commit; no automatic push".into(),
                );
            }
            let committed_identity = match RepositoryIdentity::capture(&git) {
                Ok(value) => value,
                Err(error) => {
                    return blocked(format!(
                        "commit succeeded but repository identity could not be verified: {error}"
                    ));
                }
            };
            if !identity.same_layout_and_config(&committed_identity) {
                return blocked(
                    "HIGH: repository layout or config changed during commit; no automatic push"
                        .into(),
                );
            }
            transaction_identity = Some(committed_identity);
            if let Some(before) = adaptation_before {
                if fs::read(&adaptation_path).ok().as_deref() != Some(before.as_slice()) {
                    return blocked(
                        "HIGH: adaptation receipt changed concurrently after commit; it was not removed"
                            .into(),
                    );
                }
                if let Err(error) = fs::remove_file(&adaptation_path) {
                    return blocked(format!(
                        "commit succeeded but retired adaptation receipt could not be removed: {error}"
                    ));
                }
            }
        }
        if code != 0 && code != 1 {
            return fail(
                format!("git diff --cached --quiet returned unexpected exit code {code}"),
                &post_add_index,
                snapshots,
            );
        }
    }
    if factory && let Err(error) = crate::baseline::verify(root, None, true) {
        return blocked(format!("current baseline blocked git-sync: {error}"));
    }
    (ahead, behind) = match git.ahead_behind_target(push_comparison) {
        Ok(value) => value,
        Err(error) => return blocked(error),
    };
    let mut pushed = false;
    if behind > 0 {
        return blocked("remote advanced during git-sync; rerun after reviewing state".into());
    }
    if ahead > 0 && !options.skip_push {
        let push_identity = match &transaction_identity {
            Some(identity) => match RepositoryIdentity::capture(&git) {
                Ok(current) if &current == identity => identity.clone(),
                Ok(_) => {
                    return blocked(
                        "HIGH: repository identity drift detected after commit; push was not attempted"
                            .into(),
                    );
                }
                Err(error) => return blocked(error),
            },
            None => match RepositoryIdentity::capture(&git) {
                Ok(identity) => identity,
                Err(error) => return blocked(error),
            },
        };
        if let Err(error) = git.required(&["push"], Duration::from_secs(240)) {
            match RepositoryIdentity::capture(&git) {
                Ok(current) if current == push_identity => {}
                Ok(_) => {
                    return blocked(format!(
                        "{error}; HIGH: repository identity drift detected after failed push"
                    ));
                }
                Err(identity_error) => {
                    return blocked(format!(
                        "{error}; cannot revalidate repository identity: {identity_error}"
                    ));
                }
            }
            return blocked(error);
        }
        match RepositoryIdentity::capture(&git) {
            Ok(current) if current == push_identity => {}
            Ok(_) => {
                return blocked(
                    "HIGH: repository identity drift detected after successful push; remote may be updated but local state requires review"
                        .into(),
                );
            }
            Err(error) => {
                return blocked(format!(
                    "push succeeded but repository identity could not be revalidated: {error}"
                ));
            }
        }
        pushed = true;
    }
    dirty = match git.status() {
        Ok(value) => !value.is_empty(),
        Err(error) => return blocked(error),
    };
    (ahead, behind) = match git.ahead_behind_target(push_comparison) {
        Ok(value) => value,
        Err(error) => return blocked(error),
    };
    let commit = git
        .required(&["rev-parse", "HEAD"], Duration::from_secs(30))
        .ok();
    let status = if !dirty && behind == 0 && (ahead == 0 || options.skip_push) {
        if ahead == 0 { "synced" } else { "local-ahead" }
    } else {
        "remaining-state"
    };
    drop(sync_lock);
    CommandOutcome {
        code: if status == "synced" {
            0
        } else if status == "local-ahead" {
            3
        } else {
            3
        },
        receipt: Some(json!(receipt(
            status,
            commit,
            Some(push_target),
            pushed,
            dirty,
            ahead,
            behind,
            autostashed
        ))),
        stdout: if status == "synced" {
            "[git-sync] synced\n".into()
        } else {
            "[git-sync] finished with remaining state\n".into()
        },
        ..CommandOutcome::default()
    }
}

#[cfg(all(test, bridgeforge_factory_tests))]
#[path = "../../../../../scripts/tests/unit/core_git_sync.rs"]
mod tests;
