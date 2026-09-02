use bridgeforge_core::{ProcessOutput, ProcessRequest, ProcessRunner, SystemProcessRunner};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const SHELLS: &[&str] = &["Bash", "PowerShell", "shell_command"];
const EDITS: &[&str] = &["Edit", "Write", "MultiEdit", "NotebookEdit"];

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

struct Fixture {
    directory: PathBuf,
    root: PathBuf,
    home: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let token = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "bridgeforge-guard-matrix-{}-{token}",
            std::process::id()
        ));
        let root = directory.join("project");
        let home = root.join("fake-home");
        fs::create_dir_all(home.join(".codex")).unwrap();
        fs::write(home.join(".codex/config.toml"), b"protected = true\n").unwrap();
        Self {
            directory,
            root,
            home,
        }
    }

    fn run(&self, event: &str, tool: &str, input: Value) -> ProcessOutput {
        let binary = std::env::var_os("BRIDGEFORGE_TEST_HOOK")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                repository().join(if cfg!(windows) {
                    ".codex/bin/bridgeforge-hook.exe"
                } else {
                    ".codex/bin/bridgeforge-hook"
                })
            });
        assert!(
            binary.is_file(),
            "build the managed Hook before testing: {}",
            binary.display()
        );
        let mut request = ProcessRequest::new(binary, &self.root);
        request.args = vec![event.into()];
        request.timeout = Duration::from_secs(10);
        request.env.insert(
            "BRIDGEFORGE_HOOK_ROOT".into(),
            self.root.clone().into_os_string(),
        );
        request
            .env
            .insert("USERPROFILE".into(), self.home.clone().into_os_string());
        request
            .env
            .insert("HOME".into(), self.home.clone().into_os_string());
        request.stdin = serde_json::to_vec(&json!({"tool_name":tool,"tool_input":input})).unwrap();
        let result = SystemProcessRunner.run(&request).unwrap();
        assert!(!result.timed_out, "{event}/{tool} timed out");
        result
    }

    fn expect(&self, event: &str, tool: &str, input: Value, code: i32, diagnostic: &str) {
        let result = self.run(event, tool, input);
        let stderr = String::from_utf8_lossy(&result.stderr);
        assert_eq!(result.code, code, "{event}/{tool}: {stderr}");
        assert!(stderr.contains(diagnostic), "{event}/{tool}: {stderr}");
        assert_eq!(
            fs::read(self.home.join(".codex/config.toml")).unwrap(),
            b"protected = true\n"
        );
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}

#[test]
fn registration_covers_every_supported_tool_without_post_route_overlap() {
    for path in ["templates/hooks.json", ".codex/hooks.json"] {
        let document: Value =
            serde_json::from_slice(&fs::read(repository().join(path)).unwrap()).unwrap();
        let matcher = |event: &str, suffix: &str| -> BTreeSet<String> {
            let groups = document["hooks"][event].as_array().unwrap();
            let group = groups
                .iter()
                .find(|group| {
                    group["hooks"].as_array().unwrap().iter().any(|hook| {
                        hook["bridgeforgeCodexId"]
                            .as_str()
                            .unwrap_or("")
                            .ends_with(suffix)
                    })
                })
                .unwrap();
            group["matcher"]
                .as_str()
                .unwrap()
                .split('|')
                .map(str::to_owned)
                .collect()
        };
        let shells: BTreeSet<String> = SHELLS.iter().map(|name| (*name).into()).collect();
        let edits: BTreeSet<String> = EDITS
            .iter()
            .copied()
            .chain(["apply_patch"])
            .map(str::to_owned)
            .collect();
        assert_eq!(
            matcher("PreToolUse", ":pre-tool"),
            shells.union(&edits).cloned().collect(),
            "{path}"
        );
        assert_eq!(matcher("PostToolUse", ":post-shell"), shells, "{path}");
        assert_eq!(matcher("PostToolUse", ":post-edit"), edits, "{path}");
    }
}

#[test]
fn bulk_add_in_a_non_repository_is_blocked_for_every_shell_alias() {
    let fixture = Fixture::new();
    for tool in SHELLS {
        fixture.expect(
            "pre-tool",
            tool,
            json!({"command":"git add ."}),
            2,
            "[git-add-guard]",
        );
    }
}

#[test]
fn all_shell_aliases_share_both_write_guards_and_allow_read_only_commands() {
    let fixture = Fixture::new();
    for tool in SHELLS {
        fixture.expect(
            "pre-tool",
            tool,
            json!({"command":format!(
                "git -C \"{}\" add exact.txt", fixture.directory.join("outside").display()
            )}),
            2,
            "[cross-project-write-guard]",
        );
        fixture.expect(
            "pre-tool",
            tool,
            json!({"command":"Set-Content \"$HOME/.codex/config.toml\" value"}),
            2,
            "[user-config-write-guard]",
        );
        fixture.expect(
            "pre-tool",
            tool,
            json!({"command":"Get-Content \"$HOME/.codex/config.toml\""}),
            0,
            "",
        );
        fixture.expect("pre-tool", tool, json!({"command":"echo safe"}), 0, "");
        fixture.expect("post-shell", tool, json!({"command":"echo safe"}), 0, "");
        fixture.expect(
            "pre-tool",
            tool,
            json!({"command":"Set-Content local.txt 中文"}),
            2,
            "[non-ascii-shell-guard]",
        );
        fixture.expect(
            "pre-tool",
            tool,
            json!({"command":""}),
            2,
            "[hook-dispatch]",
        );
    }
}

#[test]
fn every_edit_alias_and_supported_path_field_reaches_both_guards() {
    let fixture = Fixture::new();
    for tool in EDITS {
        let mut keys = vec!["file_path", "path"];
        if *tool == "NotebookEdit" {
            keys.push("notebook_path");
        }
        for key in keys {
            fixture.expect(
                "pre-tool",
                tool,
                json!({key:fixture.directory.join("outside.ipynb")}),
                2,
                "[cross-project-write-guard]",
            );
            fixture.expect(
                "pre-tool",
                tool,
                json!({key:fixture.home.join(".codex/config.toml")}),
                2,
                "[user-config-write-guard]",
            );
            fixture.expect("pre-tool", tool, json!({key:"inside.ipynb"}), 0, "");
            fixture.expect("post-edit", tool, json!({key:"inside.ipynb"}), 0, "");
        }
    }
}

#[test]
fn missing_conflicting_and_unknown_tool_payloads_fail_closed() {
    let fixture = Fixture::new();
    for tool in EDITS {
        for input in [
            json!({}),
            json!({"file_path":""}),
            json!({"file_path":42}),
            json!({"file_path":"inside.txt","path":"../outside.txt"}),
        ] {
            fixture.expect("pre-tool", tool, input, 2, "[hook-dispatch]");
        }
    }
    fixture.expect(
        "pre-tool",
        "NotebookEdit",
        json!({
            "file_path":"inside.ipynb", "notebook_path":"../outside.ipynb"
        }),
        2,
        "conflicting target paths",
    );
    fixture.expect(
        "pre-tool",
        "unregistered_writer",
        json!({"file_path":"inside.txt"}),
        2,
        "unsupported tool",
    );
    fixture.expect(
        "post-shell",
        "NotebookEdit",
        json!({"notebook_path":"inside.ipynb"}),
        2,
        "unsupported tool",
    );
    fixture.expect(
        "post-edit",
        "shell_command",
        json!({"command":"echo safe"}),
        2,
        "unsupported tool",
    );
}

#[test]
fn apply_patch_checks_add_update_delete_and_both_move_endpoints() {
    let fixture = Fixture::new();
    for target in [
        fixture.directory.join("outside.txt"),
        fixture.home.join(".codex/config.toml"),
    ] {
        let guard = if target.starts_with(&fixture.root) {
            "[user-config-write-guard]"
        } else {
            "[cross-project-write-guard]"
        };
        for command in [
            format!(
                "*** Begin Patch\n*** Add File: {}\n+value\n*** End Patch",
                target.display()
            ),
            format!(
                "*** Begin Patch\n*** Update File: {}\n@@\n-old\n+new\n*** End Patch",
                target.display()
            ),
            format!(
                "*** Begin Patch\n*** Delete File: {}\n*** End Patch",
                target.display()
            ),
            format!(
                "*** Begin Patch\n*** Update File: inside.txt\n*** Move to: {}\n*** End Patch",
                target.display()
            ),
            format!(
                "*** Begin Patch\n*** Update File: {}\n*** Move to: inside.txt\n*** End Patch",
                target.display()
            ),
        ] {
            fixture.expect(
                "pre-tool",
                "apply_patch",
                json!({"command":command}),
                2,
                guard,
            );
        }
    }
    fixture.expect(
        "pre-tool",
        "apply_patch",
        json!({
            "command":"*** Begin Patch\n*** Add File: inside.txt\n+value\n*** End Patch"
        }),
        0,
        "",
    );
    fixture.expect(
        "pre-tool",
        "apply_patch",
        json!({"command":"not a patch"}),
        2,
        "[hook-dispatch]",
    );
}
