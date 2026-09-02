use bridgeforge_core::memory::remote::{reconcile, resolve_conflict_with_choices};
use bridgeforge_core::memory::{
    MemoryRemoteClient, authorization_payload, remote_targets_managed_repository,
};
use bridgeforge_core::{ProcessOutput, ProcessRequest, ProcessRunner};
use serde_json::json;
use std::cell::Cell;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const REMOTE: &str = "https://github.com/owner/bridgeforge-codex-memories.git";

struct Runner<F>(F);
impl<F: Fn(&ProcessRequest) -> std::io::Result<ProcessOutput>> ProcessRunner for Runner<F> {
    fn run(&self, request: &ProcessRequest) -> std::io::Result<ProcessOutput> {
        (self.0)(request)
    }
}

fn output(code: i32, text: &str, timed_out: bool) -> ProcessOutput {
    ProcessOutput {
        code,
        stdout: text.as_bytes().to_vec(),
        stderr: Vec::new(),
        timed_out,
    }
}

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let token = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "bridgeforge-security-{}-{token}",
            std::process::id()
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn git_status_errors_timeouts_and_nonzero_exits_block_bulk_add() {
    for command in ["git add .", "git add -A", "git add --all"] {
        for failure in 0..4 {
            let calls = Cell::new(0);
            let runner = Runner(|request: &ProcessRequest| {
                calls.set(calls.get() + 1);
                assert_eq!(request.program, "git");
                assert_eq!(
                    request.args,
                    ["status", "--porcelain=v1", "-z", "--untracked-files=all"]
                );
                assert_eq!(request.timeout, Duration::from_secs(10));
                match failure {
                    0 => Err(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        "missing git",
                    )),
                    1 => Err(std::io::Error::new(
                        std::io::ErrorKind::PermissionDenied,
                        "denied",
                    )),
                    2 => Ok(output(128, "", false)),
                    _ => Ok(output(0, "?? safe.txt\0", true)),
                }
            });
            let result = bridgeforge_hook::git_add_all_with_runner(
                &json!({"tool_input":{"command":command}}),
                Path::new("."),
                &runner,
            );
            assert_eq!(calls.get(), 1);
            assert_eq!(result.code, 2, "case {failure}: {}", result.stderr);
            assert!(result.stderr.contains("[git-add-guard]"));
        }
    }
}

#[test]
fn git_status_success_preserves_sensitive_checks_and_precise_add_skips_query() {
    for (status, expected) in [
        ("", 0),
        ("?? safe.txt\0", 0),
        ("?? .env.example\0", 0),
        ("?? .env\0", 2),
        ("?? credentials/private.pem\0", 2),
        ("?? .runtime/state.json\0", 2),
    ] {
        let runner = Runner(|_: &ProcessRequest| Ok(output(0, status, false)));
        let result = bridgeforge_hook::git_add_all_with_runner(
            &json!({"tool_input":{"command":"git add ."}}),
            Path::new("."),
            &runner,
        );
        assert_eq!(result.code, expected, "{status}: {}", result.stderr);
    }
    let never = Runner(|_: &ProcessRequest| panic!("non-bulk commands must not query git"));
    for command in ["git add exact.txt", "git status", "echo safe"] {
        assert_eq!(
            bridgeforge_hook::git_add_all_with_runner(
                &json!({"tool_input":{"command":command}}),
                Path::new("."),
                &never,
            )
            .code,
            0
        );
    }
}

#[test]
fn bulk_add_detects_sensitive_files_inside_untracked_directories() {
    let fixture = Temp::new();
    let mut init = ProcessRequest::new("git", &fixture.0);
    init.args = vec!["init".into()];
    assert_eq!(
        bridgeforge_core::SystemProcessRunner
            .run(&init)
            .unwrap()
            .code,
        0
    );
    fs::create_dir_all(fixture.0.join("new directory")).unwrap();
    fs::write(fixture.0.join("new directory/.env"), b"dummy test secret").unwrap();
    let result = bridgeforge_hook::git_add_all_with_runner(
        &json!({"tool_input":{"command":"git add -A"}}),
        &fixture.0,
        &bridgeforge_core::SystemProcessRunner,
    );
    assert_eq!(result.code, 2);
    assert!(result.stderr.contains("new directory/.env"));
}

#[test]
fn native_memory_accepts_only_exact_github_https_and_ssh_repository_addresses() {
    for remote in [
        REMOTE,
        "git@github.com:owner/bridgeforge-codex-memories.git",
        "ssh://git@github.com/owner/bridgeforge-codex-memories",
        "ssh://git@github.com:22/owner/bridgeforge-codex-memories.git",
        "https://github.com:443/owner/bridgeforge-codex-memories",
        "HTTPS://GITHUB.COM/Owner/BRIDGEFORGE-CODEX-MEMORIES.GIT",
    ] {
        assert!(remote_targets_managed_repository(remote), "{remote}");
        assert!(
            authorization_payload("approved", Some(remote)).is_ok(),
            "{remote}"
        );
    }
    for remote in [
        "https://gitlab.com/owner/bridgeforge-codex-memories",
        "https://github.com.evil.test/owner/bridgeforge-codex-memories",
        "https://evil.test/github.com/owner/bridgeforge-codex-memories",
        "https://evilgithub.com/owner/bridgeforge-codex-memories",
        "https://github.com@evil.test/owner/bridgeforge-codex-memories",
        "https://user:token@github.com/owner/bridgeforge-codex-memories",
        "git@evil.test:github.com/owner/bridgeforge-codex-memories",
        "git@github.com.evil.test:owner/bridgeforge-codex-memories",
        "ssh://user@github.com/owner/bridgeforge-codex-memories",
        "http://github.com/owner/bridgeforge-codex-memories",
        "https://github.com:444/owner/bridgeforge-codex-memories",
        "https://github.com//owner/bridgeforge-codex-memories",
        "https://github.com/../bridgeforge-codex-memories",
        "https://github.com/owner/other/bridgeforge-codex-memories",
        "https://github.com/owner/bridgeforge-codex-memories?query",
        "https://github.com/owner/bridgeforge-codex-memories#fragment",
        "https://github.com/owner/other-repository",
        "https://github.com/ow%6eer/bridgeforge-codex-memories",
        "file:///tmp/bridgeforge-codex-memories",
        "C:/tmp/bridgeforge-codex-memories",
        "/tmp/bridgeforge-codex-memories",
        "github.com:owner/bridgeforge-codex-memories",
    ] {
        assert!(!remote_targets_managed_repository(remote), "{remote}");
        assert!(
            authorization_payload("approved", Some(remote)).is_err(),
            "{remote}"
        );
        let never =
            Runner(|_: &ProcessRequest| panic!("invalid address must not launch processes"));
        assert!(
            MemoryRemoteClient::new(&never)
                .verify_private_github_repository(Path::new("."), remote)
                .is_err()
        );
    }
}

#[test]
fn native_memory_verifies_private_on_explicit_github_host_and_fails_closed() {
    for response in ["PRIVATE", "PUBLIC", "INTERNAL", "", "garbage"] {
        let calls = Cell::new(0);
        let runner = Runner(|request: &ProcessRequest| {
            calls.set(calls.get() + 1);
            if request.program == "git" {
                assert_eq!(request.args, ["ls-remote", "--get-url", REMOTE]);
                return Ok(output(0, REMOTE, false));
            }
            assert_eq!(request.program, "gh");
            assert_eq!(
                request.args,
                [
                    "repo",
                    "view",
                    "https://github.com/owner/bridgeforge-codex-memories",
                    "--json",
                    "visibility",
                    "--jq",
                    ".visibility"
                ]
            );
            Ok(output(0, response, false))
        });
        let result = MemoryRemoteClient::new(&runner)
            .verify_private_github_repository(Path::new("."), REMOTE);
        assert_eq!(result.is_ok(), response == "PRIVATE");
        assert_eq!(calls.get(), 2);
    }
    for failed_program in ["git", "gh"] {
        for failure in 0..3 {
            let runner = Runner(|request: &ProcessRequest| {
                if request.program != failed_program {
                    assert_eq!(request.program, "git");
                    return Ok(output(0, REMOTE, false));
                }
                match failure {
                    0 => Err(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        "missing tool",
                    )),
                    1 => Ok(output(1, "PRIVATE", false)),
                    _ => Ok(output(0, "PRIVATE", true)),
                }
            });
            assert!(
                MemoryRemoteClient::new(&runner)
                    .verify_private_github_repository(Path::new("."), REMOTE)
                    .is_err()
            );
        }
    }
}

#[test]
fn native_memory_rejects_git_url_rewrites_to_other_repositories() {
    for effective in [
        "https://gitlab.com/owner/bridgeforge-codex-memories",
        "https://github.com/other/bridgeforge-codex-memories",
        "/tmp/bridgeforge-codex-memories",
    ] {
        let runner = Runner(|request: &ProcessRequest| {
            assert_eq!(request.program, "git", "must reject before gh");
            Ok(output(0, effective, false))
        });
        assert!(
            MemoryRemoteClient::new(&runner)
                .verify_private_github_repository(Path::new("."), REMOTE)
                .is_err()
        );
    }
}

#[test]
fn native_memory_reconcile_and_resolve_block_before_sync_writes() {
    for remote in [
        REMOTE,
        "https://gitlab.com/owner/bridgeforge-codex-memories",
    ] {
        let temp = Temp::new();
        let memories = temp.0.join("memories");
        let state = temp.0.join("state-not-created");
        fs::create_dir_all(&memories).unwrap();
        fs::write(memories.join("note.md"), b"unchanged").unwrap();
        let calls = Cell::new(0);
        let runner = Runner(|request: &ProcessRequest| {
            calls.set(calls.get() + 1);
            match request.program.to_str().unwrap() {
                "git" => {
                    assert_eq!(request.args, ["ls-remote", "--get-url", REMOTE]);
                    Ok(output(0, REMOTE, false))
                }
                "gh" => Ok(output(0, "PUBLIC", false)),
                _ => panic!("unexpected process"),
            }
        });
        assert!(reconcile(&memories, &state, remote, &runner).is_err());
        assert!(
            resolve_conflict_with_choices(
                &memories,
                &state,
                remote,
                "unread-conflict",
                &[],
                &runner
            )
            .is_err()
        );
        assert!(
            !state.exists(),
            "failed verification must not create sync state"
        );
        assert_eq!(fs::read(memories.join("note.md")).unwrap(), b"unchanged");
        assert_eq!(calls.get(), if remote == REMOTE { 4 } else { 0 });
    }
}

#[test]
fn native_memory_rejects_includeif_rewrite_in_the_actual_fetch_repository() {
    let temp = Temp::new();
    let config = temp.0.join("gitconfig");
    fs::write(
        &config,
        b"[includeIf \"gitdir:**/remote.git/\"]\n    path = rewrite.conf\n",
    )
    .unwrap();
    fs::write(
        temp.0.join("rewrite.conf"),
        b"[url \"https://gitlab.com/\"]\n    insteadOf = https://github.com/\n",
    )
    .unwrap();
    let memories = temp.0.join("memories");
    fs::create_dir_all(&memories).unwrap();
    fs::write(memories.join("note.md"), b"unchanged").unwrap();
    let initial_checked = Cell::new(false);
    let actual_checked = Cell::new(false);
    let runner = Runner(|request: &ProcessRequest| {
        if request.program == "gh" {
            return Ok(output(0, "PRIVATE", false));
        }
        assert_eq!(request.program, "git");
        assert!(
            !request
                .args
                .iter()
                .any(|arg| arg == "fetch" || arg == "push"),
            "unapproved actual destination must be blocked before network access"
        );
        let mut isolated = request.clone();
        isolated
            .env
            .insert("GIT_CONFIG_NOSYSTEM".into(), "1".into());
        isolated
            .env
            .insert("GIT_CONFIG_GLOBAL".into(), config.clone().into_os_string());
        let result = bridgeforge_core::SystemProcessRunner.run(&isolated)?;
        if request.args == ["ls-remote", "--get-url", REMOTE] {
            initial_checked.set(true);
            assert_eq!(String::from_utf8_lossy(&result.stdout).trim(), REMOTE);
        }
        if request.args == ["remote", "get-url", "--all", "origin"] {
            actual_checked.set(true);
            assert!(String::from_utf8_lossy(&result.stdout).starts_with("https://gitlab.com/"));
        }
        Ok(result)
    });
    assert!(reconcile(&memories, &temp.0.join("state"), REMOTE, &runner).is_err());
    assert!(initial_checked.get());
    assert!(actual_checked.get());
    assert_eq!(fs::read(memories.join("note.md")).unwrap(), b"unchanged");
}
