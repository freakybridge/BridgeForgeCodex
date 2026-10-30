use super::*;

fn shell_payload(command: &str) -> Value {
    json!({"tool_name":"PowerShell","tool_input":{"command":command}})
}

#[test]
fn path_guard_distinguishes_reads_writes_and_resolves_working_directory() {
    let fixture = Fixture::new();
    let external = fixture.root.parent().unwrap().join("outside input.txt");
    let inside = fixture.root.join("inside file.txt");
    for command in [
        format!("Copy-Item '{}' '{}'", external.display(), inside.display()),
        format!(
            "Copy-Item -LiteralPath '{}' -Destination '{}'",
            external.display(),
            inside.display()
        ),
        "Set-Content inside.txt x".into(),
        "Add-Content -Path inside.txt -Value x".into(),
        "Write-Output 'Set-Content ../outside.txt x'".into(),
        "rg 'cargo test|Set-Content ../outside.txt' source.rs".into(),
        "# Set-Content ../outside.txt x\nGet-Content ../outside.txt".into(),
        "<# Set-Content ../outside.txt x #> Get-Content ../outside.txt".into(),
        "Write-Output x > inside.txt".into(),
        "'Remove-Item'".into(),
        "'Remove-Item' > inside.txt".into(),
    ] {
        let result = guards::cross_project_write(&shell_payload(&command));
        assert_eq!(result.code, 0, "{command}: {}", result.stderr);
    }
    for command in [
        "Set-Content ../outside.txt x".into(),
        "Set-'Content' ../outside.txt x".into(),
        "Set-\"Content\" ../outside.txt x".into(),
        "Write-Output x >> '../outside file.txt'".into(),
        "Copy-Item inside.txt ../outside.txt".into(),
        "Move-Item ../outside.txt inside.txt".into(),
        "Rename-Item ../outside.txt renamed.txt".into(),
        "Remove-Item inside.txt ../outside.txt".into(),
        "git -C .. status; Set-Content ../outside.txt x".into(),
        "git -C .. add file.txt".into(),
        "git -C .. -C . commit -m x".into(),
        "git -c core.autocrlf=false -C .. add file.txt".into(),
        "Set-Content -Path $target -Value x".into(),
        "Set-Content -Unknown inside.txt x".into(),
        "cd ..; Set-Content outside.txt x".into(),
        "cd ..; pwsh -c 'Set-Content outside.txt x'".into(),
        "Set-Content 'unterminated x".into(),
        "Write-Output x >".into(),
        "'Remove-Item' > ../outside.txt".into(),
        "& 'Remove-Item' ../outside.txt".into(),
        "bash -c '\"rm\" ../outside.txt'".into(),
        "Set-Content C:outside.txt x".into(),
        format!("Move-Item '{}' '{}'", external.display(), inside.display()),
    ] {
        let result = guards::cross_project_write(&shell_payload(&command));
        assert_eq!(result.code, 2, "{command}: {}", result.stderr);
    }
    fs::create_dir(fixture.root.join("subdir")).unwrap();
    let mut payload = shell_payload("Set-Content ../inside.txt x");
    payload["tool_input"]["workdir"] = json!(fixture.root.join("subdir"));
    assert_eq!(guards::cross_project_write(&payload).code, 0);
    payload["tool_input"]["workdir"] = json!(fixture.root.parent().unwrap());
    assert_eq!(guards::cross_project_write(&payload).code, 2);
    assert!(!external.exists());
    assert!(!inside.exists());
}

#[cfg(windows)]
#[test]
fn path_guard_rejects_unc_and_existing_junction_escapes() {
    let fixture = Fixture::new();
    assert_eq!(
        guards::cross_project_write(&shell_payload(r"Set-Content \\server\share\outside.txt x"))
            .code,
        2
    );
    let outside = fixture
        .root
        .parent()
        .unwrap()
        .join(format!("bf-link-outside-{}", std::process::id()));
    fs::create_dir_all(&outside).unwrap();
    let link = fixture.root.join("linked");
    let output = crate::util::run_command(
        "cmd",
        [
            "/c",
            "mklink",
            "/J",
            link.to_str().unwrap(),
            outside.to_str().unwrap(),
        ],
        &fixture.root,
        Duration::from_secs(10),
    )
    .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let result = guards::cross_project_write(&shell_payload("Set-Content linked/outside.txt x"));
    assert_eq!(result.code, 2, "{}", result.stderr);
    fs::remove_dir(&link).unwrap();
    fs::remove_dir(&outside).unwrap();
}

#[test]
fn receipt_recognizes_executables_and_never_promotes_shell_composite_results() {
    let fixture = Fixture::new();
    for command in [
        "rg 'pytest|cargo test' source.rs",
        "echo 'pytest'",
        "# pytest\necho ok",
        "Write-Output 'make'",
        "Get-Content tsc.txt",
        "'pytest'",
        "\"make\"",
    ] {
        let mut payload = shell_payload(command);
        payload["tool_response"] = json!({"exit_code":0});
        assert_eq!(post::test_receipt(&payload).code, 0);
        assert!(
            !fixture.root.join(".runtime/tests/test-receipts").exists(),
            "{command}"
        );
    }
    for (command, response, expected) in [
        ("cargo test --locked", json!({"exit_code":7}), Some(7)),
        ("py'test' tests", json!({"exit_code":0}), Some(0)),
        ("cargo test\n", json!({"exit_code":0}), Some(0)),
        ("cargo test\n# comment", json!({"exit_code":0}), Some(0)),
        ("cargo test;", json!({"exit_code":0}), Some(0)),
        ("cargo test\necho ok", json!({"exit_code":0}), None),
        (
            "& 'C:/tools/pytest.exe' tests",
            json!({"exit_code":0}),
            Some(0),
        ),
        ("python -m pytest", json!({"exit_code":1}), Some(1)),
        ("cargo test; echo ok", json!({"exit_code":0}), None),
        ("cargo test | Out-Null", json!({"exit_code":0}), None),
        ("cargo test &", json!({"exit_code":0}), None),
        (
            "cargo test",
            json!({"exit_code":0,"interrupted":true}),
            None,
        ),
        ("cargo test", json!({"exit_code":0,"session_id":123}), None),
        (
            "cargo test",
            json!({"stdout":"child says exit code 0"}),
            None,
        ),
    ] {
        let mut payload = shell_payload(command);
        payload["tool_response"] = response;
        assert_eq!(post::test_receipt(&payload).code, 0);
        let text = fs::read_to_string(
            fixture
                .root
                .join(".runtime/tests/test-receipts/receipts.jsonl"),
        )
        .unwrap();
        let last: Value = serde_json::from_str(text.lines().last().unwrap()).unwrap();
        assert_eq!(last["exit_code"].as_i64(), expected, "{command}: {last}");
    }
    let payload = json!({"tool_name":"Bash","tool_input":{"command":"'pytest' tests\n"},"tool_response":{"exit_code":3}});
    assert_eq!(post::test_receipt(&payload).code, 0);
    let text = fs::read_to_string(
        fixture
            .root
            .join(".runtime/tests/test-receipts/receipts.jsonl"),
    )
    .unwrap();
    let last: Value = serde_json::from_str(text.lines().last().unwrap()).unwrap();
    assert_eq!(last["exit_code"], 3);
}

struct GitReplies {
    replies: std::sync::Mutex<std::collections::VecDeque<bridgeforge_core::ProcessOutput>>,
    calls: std::sync::Mutex<Vec<Vec<String>>>,
}

#[test]
fn edit_checks_cover_patch_targets_and_only_scan_instructions_when_relevant() {
    use std::sync::atomic::Ordering::Relaxed;
    let fixture = Fixture::new();
    fs::create_dir_all(fixture.root.join("doc")).unwrap();
    fs::write(
        fixture.root.join("doc/unrelated.md"),
        b"\xef\xbb\xbfunrelated",
    )
    .unwrap();
    fs::write(fixture.root.join("doc/edited.md"), b"normal").unwrap();
    let ordinary = json!({"tool_name":"Edit","tool_input":{"file_path":"doc/edited.md"}});
    assert!(!post::edit_requires_instruction_check(&[ordinary.clone()]));
    assert_eq!(post::encoding(&[ordinary.clone()]).code, 0);
    post::INSTRUCTION_SCANS.store(0, Relaxed);
    assert_eq!(post_edit(&ordinary), 0);
    assert_eq!(post::INSTRUCTION_SCANS.load(Relaxed), 0);
    for path in [
        "AGENTS.md",
        "sub/AGENTS.md",
        ".codex/managed-skeleton.json",
        ".codex/rules/check.rules",
        "doc/README.md",
    ] {
        let edit = json!({"tool_name":"Edit","tool_input":{"file_path":path}});
        assert!(
            post::edit_requires_instruction_check(&[ordinary.clone(), edit]),
            "{path}"
        );
    }
    let instruction = json!({"tool_name":"Edit","tool_input":{"file_path":"AGENTS.md"}});
    assert_eq!(post_edit(&instruction), 0);
    assert_eq!(post::INSTRUCTION_SCANS.load(Relaxed), 1);
}

#[test]
fn instruction_scan_prunes_only_the_existing_excluded_subtrees() {
    use std::sync::atomic::Ordering::Relaxed;
    let fixture = Fixture::new();
    fs::write(fixture.root.join("AGENTS.md"), b"# Fixture\n").unwrap();
    for directory in [".runtime", ".git", "doc/4_archive"] {
        fs::create_dir_all(fixture.root.join(directory)).unwrap();
    }
    post::INSTRUCTION_ENTRIES.store(0, Relaxed);
    let original = post::instruction_source(&Value::Null).stderr;
    let visits = post::INSTRUCTION_ENTRIES.load(Relaxed);
    for directory in [".runtime", ".git", "doc/4_archive"] {
        for i in 0..40 {
            let path = fixture
                .root
                .join(directory)
                .join(format!("nested/{i}/AGENTS.md"));
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, b"---\npaths: '**'\n---\n").unwrap();
        }
    }
    post::INSTRUCTION_ENTRIES.store(0, Relaxed);
    assert_eq!(post::instruction_source(&Value::Null).stderr, original);
    assert_eq!(post::INSTRUCTION_ENTRIES.load(Relaxed), visits);
    fs::create_dir_all(fixture.root.join("src/nested")).unwrap();
    fs::write(
        fixture.root.join("src/nested/AGENTS.md"),
        b"\xef\xbb\xbfactive instruction",
    )
    .unwrap();
    assert!(
        post::instruction_source(&Value::Null)
            .stderr
            .contains("src/nested/AGENTS.md")
    );
}
impl bridgeforge_core::ProcessRunner for GitReplies {
    fn run(
        &self,
        request: &bridgeforge_core::ProcessRequest,
    ) -> std::io::Result<bridgeforge_core::ProcessOutput> {
        self.calls.lock().unwrap().push(
            request
                .args
                .iter()
                .map(|s| s.to_string_lossy().into_owned())
                .collect(),
        );
        self.replies
            .lock()
            .unwrap()
            .pop_front()
            .ok_or_else(|| std::io::Error::other("unexpected Git write/query"))
    }
}

#[test]
fn session_preserves_effective_custom_hooks_and_fails_closed_on_queries() {
    let fixture = Fixture::new();
    fs::create_dir(fixture.root.join(".git")).unwrap();
    fs::create_dir(fixture.root.join(".githooks")).unwrap();
    fs::write(fixture.root.join(".githooks/pre-commit"), b"managed").unwrap();
    for (code, stdout, expected) in [
        (0, b"file:.git/config\0.githooks\0".as_slice(), 0),
        (0, b"file:global\0custom-hooks\0".as_slice(), 0),
        (0, b"file:worktree\0/custom/hooks\0".as_slice(), 0),
        (128, b"".as_slice(), 1),
        (0, b"malformed".as_slice(), 1),
    ] {
        let runner = GitReplies {
            replies: std::sync::Mutex::new(
                [bridgeforge_core::ProcessOutput {
                    code,
                    stdout: stdout.to_vec(),
                    stderr: vec![],
                    timed_out: false,
                }]
                .into(),
            ),
            calls: std::sync::Mutex::new(Vec::new()),
        };
        let result = session::githooks_path_with_runner(&fixture.root, &runner);
        assert_eq!(result.code, expected, "{}", result.stderr);
        assert_eq!(runner.calls.lock().unwrap().len(), 1);
    }
}

struct IsolatedGit {
    global: PathBuf,
}
impl bridgeforge_core::ProcessRunner for IsolatedGit {
    fn run(
        &self,
        request: &bridgeforge_core::ProcessRequest,
    ) -> std::io::Result<bridgeforge_core::ProcessOutput> {
        let mut isolated = request.clone();
        isolated.env_remove.extend(
            std::env::vars_os()
                .map(|(key, _)| key)
                .filter(|key| key.to_string_lossy().starts_with("GIT_")),
        );
        isolated
            .env
            .insert("GIT_CONFIG_NOSYSTEM".into(), "1".into());
        isolated.env.insert(
            "GIT_CONFIG_GLOBAL".into(),
            self.global.clone().into_os_string(),
        );
        bridgeforge_core::ProcessRunner::run(&bridgeforge_core::SystemProcessRunner, &isolated)
    }
}

#[test]
fn session_real_git_keeps_global_local_worktree_and_default_hooks() {
    let fixture = Fixture::new();
    let runner = IsolatedGit {
        global: fixture.root.join("fake-home/gitconfig"),
    };
    let run = |args: &[&str]| {
        let mut request = bridgeforge_core::ProcessRequest::new("git", &fixture.root);
        request.args = args.iter().map(std::ffi::OsString::from).collect();
        let result = bridgeforge_core::ProcessRunner::run(&runner, &request).unwrap();
        assert_eq!(
            result.code,
            0,
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        result
    };
    run(&["init", "--quiet"]);
    fs::create_dir(fixture.root.join(".githooks")).unwrap();
    fs::write(fixture.root.join(".githooks/pre-commit"), b"managed").unwrap();
    // Git's sample files must not prevent initialization.
    assert_eq!(
        session::githooks_path_with_runner(&fixture.root, &runner).code,
        0
    );
    assert_eq!(
        String::from_utf8_lossy(&run(&["config", "--local", "--get", "core.hooksPath"]).stdout)
            .trim(),
        ".githooks"
    );
    run(&["config", "--local", "--unset", "core.hooksPath"]);
    let config = fixture.root.join(".git/config");
    let original = fs::read(&config).unwrap();
    fs::write(
        fixture.root.join(".git/hooks/pre-commit"),
        b"custom default",
    )
    .unwrap();
    let result = session::githooks_path_with_runner(&fixture.root, &runner);
    assert!(
        result.stdout.contains("Default Git hooks preserved"),
        "{} {}",
        result.stdout,
        result.stderr
    );
    assert_eq!(fs::read(&config).unwrap(), original);
    fs::remove_file(fixture.root.join(".git/hooks/pre-commit")).unwrap();
    run(&["config", "--global", "core.hooksPath", "custom global"]);
    let global = fs::read(&runner.global).unwrap();
    assert_eq!(
        session::githooks_path_with_runner(&fixture.root, &runner).code,
        0
    );
    assert_eq!(fs::read(&config).unwrap(), original);
    assert_eq!(fs::read(&runner.global).unwrap(), global);
    run(&["config", "--local", "core.hooksPath", "custom local"]);
    let local = fs::read(&config).unwrap();
    assert_eq!(
        session::githooks_path_with_runner(&fixture.root, &runner).code,
        0
    );
    assert_eq!(fs::read(&config).unwrap(), local);
    run(&["config", "--local", "extensions.worktreeConfig", "true"]);
    run(&["config", "--worktree", "core.hooksPath", "custom worktree"]);
    let worktree = fs::read(fixture.root.join(".git/config.worktree")).unwrap();
    let result = session::githooks_path_with_runner(&fixture.root, &runner);
    assert!(
        result.stdout.contains("custom worktree"),
        "{} {}",
        result.stdout,
        result.stderr
    );
    assert_eq!(
        fs::read(fixture.root.join(".git/config.worktree")).unwrap(),
        worktree
    );
}
