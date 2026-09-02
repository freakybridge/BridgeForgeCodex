use super::*;

struct RestoreFixture(PathBuf);
impl RestoreFixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "bf-restore-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}
impl Drop for RestoreFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn restore_uses_only_verified_manifest_files() {
    let temp = RestoreFixture::new();
    let snapshot = temp.0.join("snapshot");
    snapshot_from_files(
        &snapshot,
        &BTreeMap::from([("nested/retained.md".into(), b"approved bytes".to_vec())]),
        1,
    )
    .unwrap();
    fs::write(snapshot.join("memories/extra.tmp"), b"not approved").unwrap();
    fs::create_dir(snapshot.join("memories/.git")).unwrap();
    fs::write(snapshot.join("memories/.git/config"), b"not approved").unwrap();
    let destination = temp.0.join("memories");
    fs::create_dir(&destination).unwrap();
    fs::write(destination.join("old.md"), b"old").unwrap();
    let expected = super::super::capture_manifest(&destination, 0, None).unwrap();
    restore_snapshot(&snapshot, &destination, Some(&expected)).unwrap();
    assert_eq!(
        fs::read(destination.join("nested/retained.md")).unwrap(),
        b"approved bytes"
    );
    assert!(!destination.join("extra.tmp").exists());
    assert!(!destination.join(".git").exists());
    assert!(!destination.join("old.md").exists());
}

#[test]
fn corrupted_declared_file_blocks_restore_and_preserves_destination() {
    let temp = RestoreFixture::new();
    let snapshot = temp.0.join("snapshot");
    snapshot_from_files(
        &snapshot,
        &BTreeMap::from([("retained.md".into(), b"approved bytes".to_vec())]),
        1,
    )
    .unwrap();
    fs::write(snapshot.join("memories/retained.md"), b"corrupted").unwrap();
    let destination = temp.0.join("memories");
    fs::create_dir(&destination).unwrap();
    fs::write(destination.join("old.md"), b"original").unwrap();
    assert!(
        restore_snapshot(&snapshot, &destination, None)
            .unwrap_err()
            .to_string()
            .contains("manifest")
    );
    assert_eq!(fs::read(destination.join("old.md")).unwrap(), b"original");
    assert!(!destination.join("retained.md").exists());
}

const TEST_REMOTE: &str = "https://github.com/owner/bridgeforge-codex-memories.git";

struct LocalGit {
    root: PathBuf,
    bare: PathBuf,
    injection: std::sync::Mutex<Option<(bool, PathBuf)>>,
}
impl LocalGit {
    fn new(root: &Path) -> Self {
        fs::create_dir_all(root).unwrap();
        let runner = Self {
            root: root.into(),
            bare: root.join("remote.git"),
            injection: std::sync::Mutex::new(None),
        };
        Git { runner: &runner }
            .required(root, &["init", "--bare", runner.bare.to_str().unwrap()])
            .unwrap();
        runner
    }
    fn head(&self) -> String {
        Git { runner: self }
            .required(&self.bare, &["rev-parse", "main"])
            .unwrap()
    }
    fn inject_on_push(&self, after: bool, target: &Path) {
        *self.injection.lock().unwrap() = Some((after, target.into()));
    }
}
impl ProcessRunner for LocalGit {
    fn run(&self, request: &ProcessRequest) -> std::io::Result<crate::ProcessOutput> {
        assert!(request.cwd.starts_with(&self.root));
        let output = |value: &str| {
            Ok(crate::ProcessOutput {
                code: 0,
                stdout: value.as_bytes().to_vec(),
                stderr: Vec::new(),
                timed_out: false,
            })
        };
        if request.program == "gh" {
            return output("PRIVATE");
        }
        assert_eq!(request.program, "git");
        let args = request
            .args
            .iter()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        if args == ["ls-remote", "--get-url", TEST_REMOTE]
            || args == ["remote", "get-url", "--all", "origin"]
            || args == ["remote", "get-url", "--push", "--all", "origin"]
        {
            if args.contains(&"--push".into()) {
                let mut injection = self.injection.lock().unwrap();
                if injection.as_ref().is_some_and(|(after, _)| !after) {
                    fs::write(
                        injection.take().unwrap().1,
                        b"written during synchronization",
                    )?;
                }
            }
            return output(TEST_REMOTE);
        }
        let mut local = request.clone();
        for arg in &mut local.args {
            if arg == TEST_REMOTE {
                *arg = self.bare.clone().into_os_string();
            }
            assert!(!arg.to_string_lossy().contains("https://"));
        }
        local.env.insert(
            "GIT_CONFIG_GLOBAL".into(),
            self.root.join("global.gitconfig").into_os_string(),
        );
        local.env.insert("GIT_CONFIG_NOSYSTEM".into(), "1".into());
        let result = crate::SystemProcessRunner.run(&local)?;
        if args.first().is_some_and(|arg| arg == "push") {
            let mut injection = self.injection.lock().unwrap();
            if injection.as_ref().is_some_and(|(after, _)| *after) {
                fs::write(
                    injection.take().unwrap().1,
                    b"written during synchronization",
                )?;
            }
        }
        Ok(result)
    }
}

fn write_memory(root: &Path, name: &str, bytes: &[u8]) {
    let target = root.join(name);
    fs::create_dir_all(target.parent().unwrap()).unwrap();
    fs::write(target, bytes).unwrap();
}

fn synchronized_pair(root: &Path) -> (LocalGit, PathBuf, PathBuf, PathBuf, PathBuf) {
    let runner = LocalGit::new(root);
    let a = root.join("a/memories");
    let sa = root.join("a/state");
    let b = root.join("b/memories");
    let sb = root.join("b/state");
    write_memory(&a, "note.md", b"baseline");
    fs::create_dir_all(&sa).unwrap();
    assert_eq!(reconcile(&a, &sa, TEST_REMOTE, &runner).unwrap(), "push");
    fs::create_dir_all(&sb).unwrap();
    assert_eq!(reconcile(&b, &sb, TEST_REMOTE, &runner).unwrap(), "restore");
    (runner, a, sa, b, sb)
}

fn conflicting_pair(root: &Path) -> (LocalGit, PathBuf, PathBuf, PathBuf, PathBuf, String) {
    let (runner, a, sa, b, sb) = synchronized_pair(root);
    write_memory(&a, "note.md", b"remote edit");
    reconcile(&a, &sa, TEST_REMOTE, &runner).unwrap();
    write_memory(&b, "note.md", b"local edit");
    assert_eq!(
        reconcile(&b, &sb, TEST_REMOTE, &runner).unwrap(),
        "conflicted"
    );
    let active: serde_json::Value =
        serde_json::from_slice(&fs::read(sb.join("active-conflict.json")).unwrap()).unwrap();
    (
        runner,
        a,
        sa,
        b,
        sb,
        active["conflictId"].as_str().unwrap().into(),
    )
}

#[test]
fn conflict_resolution_blocks_new_local_files_without_publishing() {
    let fixture = RestoreFixture::new();
    let (runner, _, _, b, sb, id) = conflicting_pair(&fixture.0);
    let before = runner.head();
    write_memory(&b, "fresh.md", b"new after conflict capture");
    let error = resolve_conflict_with_choices(
        &b,
        &sb,
        TEST_REMOTE,
        &id,
        &[("note.md".into(), "local".into())],
        &runner,
    )
    .unwrap_err();
    assert!(error.to_string().contains("local native memories changed"));
    assert_eq!(
        fs::read(b.join("fresh.md")).unwrap(),
        b"new after conflict capture"
    );
    assert_eq!(runner.head(), before);
    assert!(sb.join("active-conflict.json").exists());
    assert_eq!(
        reconcile(&b, &sb, TEST_REMOTE, &runner).unwrap(),
        "conflicted"
    );
    let active: serde_json::Value =
        serde_json::from_slice(&fs::read(sb.join("active-conflict.json")).unwrap()).unwrap();
    let fresh_id = active["conflictId"].as_str().unwrap();
    assert_eq!(
        resolve_conflict_with_choices(
            &b,
            &sb,
            TEST_REMOTE,
            fresh_id,
            &[("note.md".into(), "local".into())],
            &runner
        )
        .unwrap(),
        "resolved"
    );
    assert_eq!(
        fs::read(b.join("fresh.md")).unwrap(),
        b"new after conflict capture"
    );
    assert!(!sb.join("active-conflict.json").exists());
}

#[test]
fn automatic_merge_preserves_concurrent_local_writes_before_and_after_push() {
    for after_push in [false, true] {
        let fixture = RestoreFixture::new();
        let (runner, a, sa, b, sb) = synchronized_pair(&fixture.0);
        write_memory(&a, "remote.md", b"remote addition");
        reconcile(&a, &sa, TEST_REMOTE, &runner).unwrap();
        write_memory(&b, "local.md", b"local addition");
        let before = runner.head();
        runner.inject_on_push(after_push, &b.join("fresh.md"));
        let error = reconcile(&b, &sb, TEST_REMOTE, &runner).unwrap_err();
        assert!(error.to_string().contains("local native memories changed"));
        assert_eq!(
            fs::read(b.join("fresh.md")).unwrap(),
            b"written during synchronization"
        );
        assert_eq!(fs::read(b.join("local.md")).unwrap(), b"local addition");
        if !after_push {
            assert_eq!(runner.head(), before);
        }
        // A subsequent run merges all three additions without data loss.
        reconcile(&b, &sb, TEST_REMOTE, &runner).unwrap();
        assert!(b.join("fresh.md").exists());
        assert!(b.join("remote.md").exists());
        assert!(b.join("local.md").exists());
    }
}

#[test]
fn snapshot_blobs_ignore_attributes_filters_ignores_and_line_endings() {
    let fixture = RestoreFixture::new();
    let (runner, a, sa, _, _) = synchronized_pair(&fixture.0);
    let attributes = fixture.0.join("global.attributes");
    fs::write(&attributes, b"* text eol=crlf filter=must-not-run\n").unwrap();
    Git { runner: &runner }
        .required(
            &fixture.0,
            &[
                "config",
                "--file",
                "global.gitconfig",
                "core.attributesFile",
                attributes.to_str().unwrap(),
            ],
        )
        .unwrap();
    Git { runner: &runner }
        .required(
            &fixture.0,
            &[
                "config",
                "--file",
                "global.gitconfig",
                "filter.must-not-run.clean",
                "missing-memory-filter-command",
            ],
        )
        .unwrap();
    Git { runner: &runner }
        .required(
            &fixture.0,
            &[
                "config",
                "--file",
                "global.gitconfig",
                "filter.must-not-run.smudge",
                "missing-memory-filter-command",
            ],
        )
        .unwrap();
    Git { runner: &runner }
        .required(
            &fixture.0,
            &[
                "config",
                "--file",
                "global.gitconfig",
                "filter.must-not-run.required",
                "true",
            ],
        )
        .unwrap();
    write_memory(&a, ".gitattributes", b"*.md text eol=lf\n");
    write_memory(&a, ".gitignore", b"ignored.md\n");
    write_memory(&a, "ignored.md", b"must remain in opaque snapshot\r\n");
    write_memory(&a, "note.md", b"first\r\nsecond\r\n");
    write_memory(&a, "binary.dat", &[0, 255, 128, 13, 10]);
    assert_eq!(reconcile(&a, &sa, TEST_REMOTE, &runner).unwrap(), "push");
    assert_eq!(
        Git { runner: &runner }
            .bytes(
                &runner.bare,
                &["cat-file", "blob", "main:memories/note.md"],
                &[]
            )
            .unwrap(),
        b"first\r\nsecond\r\n"
    );
    let third = fixture.0.join("third/memories");
    let state = fixture.0.join("third/state");
    fs::create_dir_all(&state).unwrap();
    assert_eq!(
        reconcile(&third, &state, TEST_REMOTE, &runner).unwrap(),
        "restore"
    );
    assert_eq!(
        super::super::capture_manifest(&third, 0, None)
            .unwrap()
            .files,
        super::super::capture_manifest(&a, 0, None).unwrap().files
    );
}

#[test]
fn successful_convergence_clears_only_active_conflict_marker() {
    let fixture = RestoreFixture::new();
    let (runner, a, sa, b, sb, id) = conflicting_pair(&fixture.0);
    write_memory(&a, "note.md", b"local edit");
    reconcile(&a, &sa, TEST_REMOTE, &runner).unwrap();
    assert_eq!(reconcile(&b, &sb, TEST_REMOTE, &runner).unwrap(), "noop");
    assert!(!sb.join("active-conflict.json").exists());
    assert!(sb.join("conflicts").join(id).exists());
    for (target, state, expected) in [(&a, &sa, "push"), (&b, &sb, "restore")] {
        fs::write(state.join("active-conflict.json"), b"{}").unwrap();
        if expected == "push" {
            write_memory(target, "new.md", b"addition");
        }
        assert_eq!(
            reconcile(target, state, TEST_REMOTE, &runner).unwrap(),
            expected
        );
        assert!(!state.join("active-conflict.json").exists());
    }
}

#[test]
fn replacement_retains_old_tree_for_writers_with_open_handles() {
    use std::io::Write;
    let fixture = RestoreFixture::new();
    let memories = fixture.0.join("memories");
    write_memory(&memories, "note.md", b"old");
    let mut writer = fs::OpenOptions::new()
        .append(true)
        .open(memories.join("note.md"))
        .unwrap();
    let expected = super::super::capture_manifest(&memories, 0, None).unwrap();
    let snapshot = fixture.0.join("snapshot");
    snapshot_from_files(
        &snapshot,
        &BTreeMap::from([("note.md".into(), b"new".to_vec())]),
        1,
    )
    .unwrap();
    match restore_snapshot(&snapshot, &memories, Some(&expected)) {
        Ok(()) => {
            writer.write_all(b" late write").unwrap();
            writer.flush().unwrap();
            drop(writer);
        }
        Err(error) => {
            // Windows can reject a directory rename while a descendant is open.
            // That must leave the original live tree intact; after the writer
            // closes, a fresh capture can be replaced and retained normally.
            assert!(
                cfg!(windows) && error.to_string().contains("os error 5"),
                "{error}"
            );
            assert_eq!(fs::read(memories.join("note.md")).unwrap(), b"old");
            writer.write_all(b" late write").unwrap();
            writer.flush().unwrap();
            drop(writer);
            let fresh = super::super::capture_manifest(&memories, 0, None).unwrap();
            restore_snapshot(&snapshot, &memories, Some(&fresh)).unwrap();
        }
    }
    let backup = fs::read_dir(&fixture.0)
        .unwrap()
        .filter_map(Result::ok)
        .find(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .contains(".before-sync.")
        })
        .unwrap()
        .path();
    assert_eq!(fs::read(backup.join("note.md")).unwrap(), b"old late write");
    assert_eq!(fs::read(memories.join("note.md")).unwrap(), b"new");
}
