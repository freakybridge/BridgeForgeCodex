use super::*;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(windows)]
#[test]
fn session_configuration_write_failure_preserves_original_and_reports_failure() {
    use std::os::windows::fs::OpenOptionsExt;
    let fixture = Fixture::new();
    let path = fixture.root.join(".codex/settings.json");
    let original = b"{\"effortLevel\":\"high\"}";
    fs::write(&path, original).unwrap();
    let held = fs::OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(&path)
        .unwrap();
    let result = session::enforce_no_effort();
    assert_eq!(result.code, 1);
    assert!(result.stdout.is_empty());
    assert!(result.stderr.contains("enforce-no-effortlevel"));
    assert_eq!(fs::read(&path).unwrap(), original);
    assert_eq!(fs::read(path.with_extension("json.bak")).unwrap(), original);
    drop(held);
}

struct Fixture {
    root: PathBuf,
    env: Vec<(&'static str, Option<std::ffi::OsString>)>,
    _guard: std::sync::MutexGuard<'static, ()>,
}
impl Fixture {
    fn new() -> Self {
        let guard = ENV_LOCK.lock().unwrap();
        let root = std::env::temp_dir().join(format!(
            "bf-hook-lifecycle-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(root.join(".codex")).unwrap();
        fs::create_dir_all(root.join("fake-home/.codex")).unwrap();
        fs::write(root.join(".codex/hooks.json"), b"{\"hooks\":{}}").unwrap();
        let mut env = Vec::new();
        for (name, value) in [
            ("BRIDGEFORGE_HOOK_ROOT", root.clone()),
            ("USERPROFILE", root.join("fake-home")),
            ("HOME", root.join("fake-home")),
        ] {
            env.push((name, std::env::var_os(name)));
            // All environment-dependent tests in this binary hold ENV_LOCK.
            unsafe {
                std::env::set_var(name, value);
            }
        }
        Self {
            root,
            env,
            _guard: guard,
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        for (name, value) in &self.env {
            unsafe {
                match value {
                    Some(value) => std::env::set_var(name, value),
                    None => std::env::remove_var(name),
                }
            }
        }
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn instruction_source_uses_trusted_public_hash_without_fixed_editorial_headings() {
    use sha2::{Digest, Sha256};

    let fixture = Fixture::new();
    let public = "<!-- BRIDGEFORGE:PUBLIC:BEGIN -->\n## Reorganized guidance\n\nKeep user scope.\n<!-- BRIDGEFORGE:PUBLIC:END -->\n";
    let project = "<!-- BRIDGEFORGE:PROJECT:BEGIN -->\n## 项目级专区\n### 项目架构红线\n### 项目业务与安全红线\n### 项目目录地图\n### 项目快速命令\n### 目录级 AGENTS 索引\n<!-- BRIDGEFORGE:PROJECT:END -->\n";
    let contract_path = fixture.root.join(".codex/managed-skeleton.json");
    let write_contract = |block: &str| {
        let hash = format!("sha256:{:x}", Sha256::digest(block.as_bytes()));
        fs::write(
            &contract_path,
            serde_json::to_vec(&serde_json::json!({"assets":[{
                "id":"root.agents", "agents_zones":{"public":{"current_sha256":hash}}
            }]}))
            .unwrap(),
        )
        .unwrap();
    };
    let agents_path = fixture.root.join("AGENTS.md");
    write_contract(public);
    fs::write(&agents_path, format!("{public}\n{project}")).unwrap();
    assert!(post::instruction_source(&Value::Null).stderr.is_empty());

    fs::write(
        &agents_path,
        format!(
            "{}\n{project}",
            public.replace("Keep user scope.", "Unexpected policy.")
        ),
    )
    .unwrap();
    assert!(
        post::instruction_source(&Value::Null)
            .stderr
            .contains("public zone was modified")
    );

    fs::write(
        &agents_path,
        format!("{public}\n{}", project.replace("### 项目快速命令\n", "")),
    )
    .unwrap();
    assert!(
        post::instruction_source(&Value::Null)
            .stderr
            .contains("project zone must contain exactly one")
    );

    fs::write(
        &agents_path,
        format!("{public}\n{project}<!-- BRIDGEFORGE:PUBLIC:END -->\n"),
    )
    .unwrap();
    assert!(
        post::instruction_source(&Value::Null)
            .stderr
            .contains("markers must each appear exactly once")
    );

    let unclosed = public.replace("Keep user scope.", "```text\nKeep user scope.");
    write_contract(&unclosed);
    fs::write(&agents_path, format!("{unclosed}\n{project}")).unwrap();
    assert!(
        post::instruction_source(&Value::Null)
            .stderr
            .contains("unclosed fenced code block")
    );

    fs::write(&agents_path, format!("{public}\n{project}")).unwrap();
    fs::remove_file(&contract_path).unwrap();
    assert!(
        post::instruction_source(&Value::Null)
            .stderr
            .contains("cannot be verified")
    );
}

fn seed_project_map_inputs(fixture: &Fixture) {
    fs::create_dir_all(fixture.root.join("src/domain")).unwrap();
    fs::create_dir_all(fixture.root.join("doc/0_architecture/design")).unwrap();
    fs::create_dir_all(fixture.root.join("doc/2_bugs/proposal")).unwrap();
    fs::write(
        fixture.root.join("AGENTS.md"),
        "# Trading Runtime\n\n- `OrderBook` changes require review.\n",
    )
    .unwrap();
    fs::write(
        fixture.root.join("Cargo.toml"),
        "[package]\nname = \"fixture\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    fs::write(fixture.root.join("src/domain/mod.rs"), "pub fn run() {}\n").unwrap();
    fs::write(fixture.root.join("src/unmapped.rs"), "pub fn idle() {}\n").unwrap();
    fs::write(
        fixture.root.join("doc/0_architecture/design/runtime.md"),
        "# Runtime\n\nImplemented by `src/domain/mod.rs`. The similar domain name alone is not evidence. `src/missing.rs` does not exist.\n",
    )
    .unwrap();
    fs::write(
        fixture.root.join("doc/2_bugs/proposal/AGENTS.md"),
        "# Retired Draft Instruction\n\n- `DoNotIndexThis` is proposal text.\n",
    )
    .unwrap();
    fs::write(
        fixture.root.join("doc/2_bugs/proposal/README.md"),
        "# Bug proposal\n\nDraft reference: `src/unmapped.rs`.\n",
    )
    .unwrap();
}

#[test]
fn project_maps_replace_legacy_content_use_only_proven_links_and_are_idempotent() {
    let fixture = Fixture::new();
    seed_project_map_inputs(&fixture);
    let find_path = fixture
        .root
        .join(".runtime/bridgeforge-codex/find-doc.map.md");
    let sync_path = fixture
        .root
        .join(".runtime/bridgeforge-codex/sync-docs.map.md");
    fs::create_dir_all(find_path.parent().unwrap()).unwrap();
    fs::write(&find_path, "legacy topic_to_rules\n").unwrap();
    fs::write(&sync_path, "legacy guessed mapping\n").unwrap();

    let result = project_map::ensure_current();
    assert_eq!(result.code, 0, "{}", result.stderr);
    assert!(result.stdout.is_empty());
    assert!(result.stderr.is_empty());
    let find = fs::read_to_string(&find_path).unwrap();
    let sync = fs::read_to_string(&sync_path).unwrap();
    assert!(!fixture.root.join(".codex/find-doc.map.md").exists());
    assert!(!fixture.root.join(".codex/sync-docs.map.md").exists());
    assert!(find.starts_with("<!-- bridgeforge-project-map schema=1 kind=find-doc input=sha256:"));
    assert!(find.contains("此文件由 BridgeForge 自动生成，禁止手工维护"));
    assert!(find.contains("`trading runtime`"));
    assert!(find.contains("`orderbook`"));
    assert!(!find.contains("retired draft instruction"));
    assert!(!find.contains("donotindexthis"));
    assert!(!find.contains("legacy topic_to_rules"));
    assert!(sync.starts_with("<!-- bridgeforge-project-map schema=1 kind=sync-docs input=sha256:"));
    assert!(sync.contains("`src/domain/mod.rs`"));
    assert!(sync.contains("`doc/0_architecture/design/runtime.md`"));
    assert!(!sync.contains("src/missing.rs"));
    assert!(!sync.contains("doc/2_bugs/proposal/README.md"));
    assert!(!sync.contains("legacy guessed mapping"));

    let before_modified = fs::metadata(&find_path).unwrap().modified().unwrap();
    std::thread::sleep(Duration::from_millis(25));
    assert_eq!(project_map::ensure_current().code, 0);
    assert_eq!(fs::read_to_string(&find_path).unwrap(), find);
    assert_eq!(fs::read_to_string(&sync_path).unwrap(), sync);
    assert_eq!(
        fs::metadata(&find_path).unwrap().modified().unwrap(),
        before_modified,
        "unchanged inputs must not rewrite the map"
    );
}

#[test]
fn project_map_dirty_tracking_is_scoped_and_strict_route_repairs_maps() {
    let fixture = Fixture::new();
    seed_project_map_inputs(&fixture);
    assert_eq!(run(vec!["project-map".into(), "ensure-current".into()]), 0);
    let marker = fixture
        .root
        .join(".runtime/bridgeforge-codex/project-map-dirty");

    let unrelated = json!({"tool_input":{"file_path":"README.md"}});
    assert_eq!(project_map::mark_dirty(&unrelated).code, 0);
    assert!(!marker.exists());

    let source = json!({"tool_input":{"file_path":"src/domain/mod.rs"}});
    assert_eq!(project_map::mark_dirty(&source).code, 0);
    assert!(marker.is_file());
    fs::write(
        fixture.root.join("AGENTS.md"),
        "# Updated Runtime\n\n- `OrderBook` changes require review.\n",
    )
    .unwrap();
    assert_eq!(project_map::ensure_if_dirty().code, 0);
    assert!(!marker.exists());
    let find = fs::read_to_string(
        fixture
            .root
            .join(".runtime/bridgeforge-codex/find-doc.map.md"),
    )
    .unwrap();
    assert!(find.contains("`updated runtime`"));
    assert!(!find.contains("`trading runtime`"));
}

#[test]
fn post_edit_and_stop_connect_dirty_tracking_to_silent_rebuild() {
    let fixture = Fixture::new();
    seed_project_map_inputs(&fixture);
    assert_eq!(project_map::ensure_current().code, 0);
    fs::write(
        fixture.root.join("AGENTS.md"),
        "# Lifecycle Updated\n\n- `OrderBook` changes require review.\n",
    )
    .unwrap();
    let payload = json!({
        "tool_name": "Edit",
        "tool_input": {"file_path": "AGENTS.md"}
    });
    assert_eq!(post_edit(&payload), 0);
    let marker = fixture
        .root
        .join(".runtime/bridgeforge-codex/project-map-dirty");
    assert!(marker.is_file());
    assert_eq!(lifecycle("stop"), 0);
    assert!(!marker.exists());
    let find = fs::read_to_string(
        fixture
            .root
            .join(".runtime/bridgeforge-codex/find-doc.map.md"),
    )
    .unwrap();
    assert!(find.contains("`lifecycle updated`"));
}

#[test]
fn project_map_rejects_non_file_targets_before_writing_any_map() {
    let fixture = Fixture::new();
    seed_project_map_inputs(&fixture);
    let map_root = fixture.root.join(".runtime/bridgeforge-codex");
    fs::create_dir_all(&map_root).unwrap();
    fs::create_dir(map_root.join("find-doc.map.md")).unwrap();
    let result = project_map::ensure_current();
    assert_eq!(result.code, 1);
    assert!(result.stderr.contains("is not a plain file"));
    assert!(
        !fixture
            .root
            .join(".runtime/bridgeforge-codex/sync-docs.map.md")
            .exists()
    );
}

#[test]
fn lifecycle_snapshots_stop_dedup_and_manual_route_succeed() {
    let fixture = Fixture::new();
    assert_eq!(lifecycle("session-start"), 0);
    assert_eq!(lifecycle("post-compact"), 0);
    let directory = fixture.root.join(".runtime/session_state");
    let paths = fs::read_dir(&directory)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    assert_eq!(paths.len(), 1);
    let before = fs::read(&paths[0]).unwrap();
    assert!(String::from_utf8_lossy(&before).contains("post-compact"));
    assert_eq!(lifecycle("stop"), 0);
    assert_eq!(fs::read(&paths[0]).unwrap(), before);
    assert_eq!(run(vec!["snapshot".into(), "manual".into()]), 0);
    assert_eq!(fs::read_dir(&directory).unwrap().count(), 2);
    assert_eq!(fs::read(&paths[0]).unwrap(), before);
}

#[test]
fn hook_topic4_batch_scans_once_and_checks_later_files() {
    use std::sync::atomic::Ordering::Relaxed;
    let fixture = Fixture::new();
    let mut command = String::from("*** Begin Patch\n");
    fs::create_dir_all(fixture.root.join("doc")).unwrap();
    for i in 0..10 {
        let path = format!("doc/note-{i}.md");
        fs::write(
            fixture.root.join(&path),
            if i == 9 {
                "bad \x3f\x3f\x3f marker"
            } else {
                "valid"
            },
        )
        .unwrap();
        command.push_str(&format!("*** Update File: {path}\n@@\n-old\n+new\n"));
    }
    command.push_str("*** End Patch");
    let payload = json!({"tool_name":"apply_patch", "tool_input":{"command":command}});
    let virtuals = edits(&payload);
    let diagnostic = post::encoding(&virtuals);
    assert_eq!(diagnostic.code, 0);
    assert!(diagnostic.stderr.contains("note-9.md"));
    for count in [1, 10] {
        let input = if count == 1 {
            json!({"tool_name":"Edit","tool_input":{"file_path":"doc/note-0.md"}})
        } else {
            payload.clone()
        };
        post::ENCODING_SCANS.store(0, Relaxed);
        post::INSTRUCTION_SCANS.store(0, Relaxed);
        assert_eq!(post_edit(&input), 0);
        assert_eq!(post::ENCODING_SCANS.load(Relaxed), 1);
        assert_eq!(post::INSTRUCTION_SCANS.load(Relaxed), 1);
    }
    fs::write(fixture.root.join("doc/note-9.md"), b"\xef\xbb\xbfblocked").unwrap();
    post::INSTRUCTION_SCANS.store(0, Relaxed);
    assert_eq!(post_edit(&payload), 2);
    assert_eq!(post::INSTRUCTION_SCANS.load(Relaxed), 0);
}

#[test]
fn hook_topic4_map_failures_propagate_without_skipping_other_work() {
    let fixture = Fixture::new();
    seed_project_map_inputs(&fixture);
    let map_root = fixture.root.join(".runtime/bridgeforge-codex");
    fs::create_dir_all(&map_root).unwrap();
    fs::create_dir(map_root.join("project-map-dirty")).unwrap();
    let payload = json!({"tool_name":"Edit","tool_input":{"file_path":"src/domain/mod.rs"}});
    post::INSTRUCTION_SCANS.store(0, std::sync::atomic::Ordering::Relaxed);
    assert_eq!(post_edit(&payload), 1);
    assert_eq!(
        post::INSTRUCTION_SCANS.load(std::sync::atomic::Ordering::Relaxed),
        1
    );
    assert_eq!(
        fs::read_to_string(fixture.root.join("src/domain/mod.rs")).unwrap(),
        "pub fn run() {}\n"
    );
    fs::create_dir(map_root.join("find-doc.map.md")).unwrap();
    assert_eq!(lifecycle("session-start"), 1);
    assert_eq!(lifecycle("stop"), 1);
    assert!(fixture.root.join(".runtime/session_state").is_dir());
    let failed = project_map::ensure_current();
    let mut output = HookOutput::default();
    output.absorb_map(&failed);
    assert_eq!(output.contexts.len(), 1);
    assert!(output.contexts[0].contains("直接检索原文件"));
    assert!(failed.stderr.contains("is not a plain file"));
}

#[test]
fn hook_topic4_patch_smell_matches_edit_and_ignores_unchanged_removed_or_split_code() {
    for (path, text) in [
        ("x.py", "except Exception:\n    pass"),
        ("x.ts", "catch (e) {}"),
    ] {
        let edit = json!({"tool_input":{"file_path":path,"new_string":text}});
        let patch = format!(
            "*** Begin Patch\n*** Add File: {path}\n{}\n*** End Patch",
            text.lines()
                .map(|line| format!("+{line}"))
                .collect::<Vec<_>>()
                .join("\n")
        );
        let virtuals = virtual_edits(&Value::Null, &patch);
        assert_eq!(
            post::fallback_smell(&edit).stdout,
            post::fallback_smell(&virtuals[0]).stdout
        );
        assert!(!post::fallback_smell(&virtuals[0]).stdout.is_empty());
    }
    let cases = [
        ("@@\n except Exception:\n-    raise\n+    pass", true),
        ("@@\n-except Exception:\n-    pass\n+raise", false),
        (
            "@@\n except Exception:\n     pass\n-old = 1\n+new = 2",
            false,
        ),
        ("@@\n+except Exception:\n@@\n+    pass", false),
        ("@@\n+except Exception:\n+    raise", false),
    ];
    for (hunks, expected) in cases {
        let patch = format!("*** Begin Patch\n*** Update File: x.py\n{hunks}\n*** End Patch");
        let virtuals = virtual_edits(&Value::Null, &patch);
        assert_eq!(
            !post::fallback_smell(&virtuals[0]).stdout.is_empty(),
            expected,
            "{patch}"
        );
    }
    let moved = virtual_edits(
        &Value::Null,
        "*** Begin Patch\n*** Update File: old.py\n*** Move to: new.py\n@@\n+except Exception:\n+    pass\n*** Delete File: deleted.py\n*** End Patch",
    );
    assert_eq!(moved.len(), 3);
    assert!(post::fallback_smell(&moved[0]).stdout.is_empty());
    assert!(post::fallback_smell(&moved[1]).stdout.contains("new.py"));
    assert!(post::fallback_smell(&moved[2]).stdout.is_empty());
}

fn handoff_text(event: &str) -> String {
    format!(
        "**Event**: {event}\n## 交接摘要（agent 填）\n### 已完成\n- checked\n### 关键决定 / 当前假设\n- read only\n### 改动文件\n- none\n### 下一步\n- inspect\n"
    )
}

#[test]
fn snapshot_git_observation_distinguishes_failure_clean_unborn_and_detached() {
    for text in [None, Some(""), Some("# branch.head main\n")] {
        let state = session::parse_git_state(text);
        assert!(!state.known);
        assert_eq!(state.ahead, "unknown");
    }
    let clean = session::parse_git_state(Some("# branch.oid aaaa\n# branch.head main\n"));
    assert!(clean.known);
    assert_eq!(clean.head, "aaaa");
    assert!(clean.changes.is_empty());
    assert_eq!(clean.ahead, "no-upstream");
    let changed = session::parse_git_state(Some(
        "# branch.oid bbbb\n# branch.head main\n# branch.upstream origin/main\n# branch.ab +1 -2\n? new file.txt\n",
    ));
    assert!(changed.known);
    assert_ne!(clean.head, changed.head);
    assert_eq!(changed.changes, "? new file.txt");
    assert_eq!(changed.ahead, "+1 -2");
    let unborn = session::parse_git_state(Some("# branch.oid (initial)\n# branch.head main\n"));
    assert!(unborn.known);
    assert_eq!(unborn.head, "(initial)");
    let detached = session::parse_git_state(Some("# branch.oid cccc\n# branch.head (detached)\n"));
    assert!(detached.known);
    assert_eq!(detached.branch, "(detached)");
}

#[test]
fn snapshot_failed_git_query_is_saved_as_unknown_not_clean() {
    let fixture = Fixture::new(); // Not a Git repository: the real command exits nonzero.
    let result = session::snapshot("manual");
    assert_eq!(result.code, 0); // File saved, even though Git observation is unknown.
    let candidates = session::snapshot_candidates(&fixture.root.join(".runtime/session_state"));
    assert_eq!(candidates.len(), 1);
    assert!(!candidates[0].1);
    let content = fs::read_to_string(&candidates[0].0).unwrap();
    assert!(
        result
            .stdout
            .contains(&candidates[0].0.display().to_string())
    );
    assert!(content.contains("**Git observation**: unknown"));
    assert!(content.contains("**HEAD**: unknown"));
    assert!(!content.contains("(clean)"));
    assert!(!content.contains("no-upstream"));
    assert!(session::show_state().stdout.contains("dirty=unknown"));
}

#[test]
fn snapshot_same_timestamp_concurrent_creation_preserves_both_files() {
    let fixture = Fixture::new();
    let directory = fixture.root.join("snapshots");
    fs::create_dir(&directory).unwrap();
    let barrier = std::sync::Barrier::new(2);
    let paths = std::thread::scope(|scope| {
        let first = scope.spawn(|| {
            barrier.wait();
            session::write_new_snapshot(&directory, "2026-09-06_120000", b"first").unwrap()
        });
        let second = scope.spawn(|| {
            barrier.wait();
            session::write_new_snapshot(&directory, "2026-09-06_120000", b"second").unwrap()
        });
        (first.join().unwrap(), second.join().unwrap())
    });
    assert_ne!(paths.0, paths.1);
    assert_eq!(fs::read(paths.0).unwrap(), b"first");
    assert_eq!(fs::read(paths.1).unwrap(), b"second");
    assert_eq!(fs::read_dir(directory).unwrap().count(), 2);
}

#[test]
fn snapshot_selection_and_retention_preserve_handoffs_and_unknown_files() {
    let fixture = Fixture::new();
    let directory = fixture.root.join(".runtime/session_state");
    fs::create_dir_all(&directory).unwrap();
    let handoff = directory.join("old-manual.md");
    let complete = handoff_text("manual").replace('\n', "\r\n");
    fs::write(&handoff, &complete).unwrap();
    let incomplete = directory.join("incomplete-manual.md");
    fs::write(&incomplete, "**Event**: manual\n").unwrap();
    let unknown = directory.join("legacy.md");
    fs::write(&unknown, "unrecognized legacy content").unwrap();
    let appended = directory.join("annotated-stop.md");
    fs::write(&appended, "**Event**: stop\n## 交接摘要\npartial").unwrap();
    for index in 0..25 {
        let path = directory.join(format!("auto-{index:02}.md"));
        fs::write(&path, "**Event**: stop\n").unwrap();
        fs::File::options()
            .write(true)
            .open(path)
            .unwrap()
            .set_times(
                fs::FileTimes::new()
                    .set_modified(SystemTime::now() + Duration::from_secs(index + 1)),
            )
            .unwrap();
    }
    assert_eq!(
        session::snapshot_candidates(&directory),
        vec![(handoff.clone(), true)]
    );
    let before_count = fs::read_dir(&directory).unwrap().count();
    let selected = session::select_snapshot(false);
    assert!(selected.stdout.contains(&handoff.display().to_string()));
    assert!(!selected.stdout.contains("state-only"));
    assert!(
        session::show_state()
            .stdout
            .contains(&handoff.display().to_string())
    );
    assert_eq!(fs::read_dir(&directory).unwrap().count(), before_count);
    assert_eq!(session::retain_automatic_snapshots(&directory).unwrap(), 5);
    assert_eq!(fs::read_to_string(&handoff).unwrap(), complete);
    assert!(incomplete.exists() && unknown.exists() && appended.exists());
    assert!(!directory.join("auto-00.md").exists());
    assert!(directory.join("auto-24.md").exists());
    assert_eq!(fs::read_dir(&directory).unwrap().count(), 24);
}

#[test]
fn snapshot_selector_is_read_only_and_labels_missing_handoff() {
    let fixture = Fixture::new();
    let directory = fixture.root.join(".runtime/session_state");
    assert_eq!(run(vec!["snapshot".into(), "latest".into()]), 1);
    assert!(!directory.exists());
    fs::create_dir_all(&directory).unwrap();
    let path = directory.join("auto.md");
    let body = "**Event**: post-compact\n";
    fs::write(&path, body).unwrap();
    let result = session::select_snapshot(false);
    assert_eq!(result.code, 0);
    assert!(result.stdout.contains("state-only / incomplete"));
    assert_eq!(run(vec!["snapshot".into(), "list".into()]), 0);
    assert_eq!(fs::read_to_string(path).unwrap(), body);
    assert_eq!(fs::read_dir(directory).unwrap().count(), 1);
}

#[test]
fn lifecycle_write_failures_are_observable_without_success_output() {
    let fixture = Fixture::new();
    fs::write(fixture.root.join(".runtime"), b"not a directory").unwrap();
    for event in ["manual", "post-compact", "stop"] {
        let result = session::snapshot(event);
        assert_eq!(result.code, 1);
        assert!(result.stdout.is_empty());
        assert!(result.stderr.contains("state operation failed"));
    }
    assert_eq!(lifecycle("post-compact"), 1);
    assert_eq!(lifecycle("stop"), 1);
    fs::write(
        fixture.root.join(".codex/settings.json"),
        b"{\"effortLevel\":\"high\"}",
    )
    .unwrap();
    fs::create_dir(fixture.root.join(".codex/settings.json.bak")).unwrap();
    assert_eq!(lifecycle("session-start"), 1);
    assert_eq!(
        fs::read(fixture.root.join(".codex/settings.json")).unwrap(),
        b"{\"effortLevel\":\"high\"}"
    );
}

fn test_payload() -> Value {
    json!({"tool_name":"shell_command", "tool_input":{"command":"cargo test"}, "tool_response":{"exit_code":7}})
}

#[test]
fn asynchronous_test_receipt_never_infers_success() {
    let fixture = Fixture::new();
    for response in [
        json!({"session_id":123,"exit_code":null,"output":"Process running with session ID 123"}),
        json!({"output":"still no terminal exit evidence"}),
        json!({"session_id":123,"exit_code":null,"output":"child helper exit code 0\nProcess running with session ID 123"}),
    ] {
        let payload = json!({"tool_name":"shell_command","tool_input":{"command":"cargo test"},"tool_response":response});
        assert_eq!(post::test_receipt(&payload).code, 0);
    }
    let text =
        fs::read_to_string(fixture.root.join(".runtime/test_receipts/receipts.jsonl")).unwrap();
    for line in text.lines() {
        let receipt: Value = serde_json::from_str(line).unwrap();
        assert!(receipt["exit_code"].is_null());
        assert_eq!(receipt["source"], "unknown");
    }
}

#[test]
fn precommit_encoding_checks_index_not_worktree() {
    let fixture = Fixture::new();
    let git = |args: &[&str]| {
        let output = util::run_command(
            "git",
            args.iter().copied(),
            &fixture.root,
            Duration::from_secs(10),
        )
        .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    };
    git(&["init"]);
    fs::create_dir_all(fixture.root.join("scripts")).unwrap();
    let path = fixture.root.join("scripts/probe.rs");
    fs::write(&path, b"\xef\xbb\xbf// staged BOM\n").unwrap();
    git(&["add", "scripts/probe.rs"]);
    fs::write(&path, b"// clean worktree\n").unwrap();
    assert_eq!(post::precommit_encoding(), 2);
    git(&["add", "scripts/probe.rs"]);
    fs::write(&path, b"\xef\xbb\xbf// unstaged BOM\n").unwrap();
    assert_eq!(post::precommit_encoding(), 0);
    fs::write(&path, b"// \x3f\x3f\x3f staged garble\n").unwrap();
    git(&["add", "scripts/probe.rs"]);
    fs::write(&path, b"// clean worktree\n").unwrap();
    assert_eq!(post::precommit_encoding(), 2);
}

#[test]
fn post_tool_receipt_preserves_exit_and_reports_directory_open_and_sample_failures() {
    for failure in ["none", "directory", "open", "sample"] {
        let fixture = Fixture::new();
        let directory = fixture.root.join(".runtime/test_receipts");
        if failure == "directory" {
            fs::write(fixture.root.join(".runtime"), b"blocked").unwrap();
        } else {
            fs::create_dir_all(&directory).unwrap();
            if failure == "open" {
                fs::create_dir(directory.join("receipts.jsonl")).unwrap();
            }
            if failure == "sample" {
                fs::write(directory.join("payload_samples"), b"blocked").unwrap();
            }
        }
        let result = post::test_receipt(&test_payload());
        if failure == "none" {
            assert_eq!(result.code, 0);
            let value: Value =
                serde_json::from_slice(&fs::read(directory.join("receipts.jsonl")).unwrap())
                    .unwrap();
            assert_eq!(value["exit_code"], 7);
            assert!(result.stdout.contains("exit=7"));
        } else {
            assert_eq!(result.code, 1, "{failure}");
            assert!(result.stdout.is_empty());
            assert!(result.stderr.contains("completed tool action is unchanged"));
            assert_eq!(post_shell(&test_payload()), 1);
        }
    }
}

#[test]
fn atomic_write_replaces_existing_file_and_cleans_failed_temporary() {
    let fixture = Fixture::new();
    let path = fixture.root.join("state.json");
    let old_style = fixture.root.join("old-style.tmp");
    fs::write(&path, b"first").unwrap();
    fs::write(&old_style, b"second").unwrap();
    fs::rename(&old_style, &path).unwrap(); // The former Windows-rename hypothesis is tested, not assumed.
    assert_eq!(fs::read(&path).unwrap(), b"second");
    util::atomic_write(&path, b"third").unwrap();
    assert_eq!(fs::read(&path).unwrap(), b"third");
    let invalid = fixture.root.join("directory");
    fs::create_dir(&invalid).unwrap();
    assert!(util::atomic_write(&invalid, b"no").is_err());
    assert!(!fs::read_dir(&fixture.root).unwrap().any(|item| {
        item.unwrap()
            .file_name()
            .to_string_lossy()
            .contains(".write-")
    }));
}

#[test]
fn registered_hook_routes_have_lifecycle_and_tool_coverage() {
    let document: Value = serde_json::from_slice(
        &fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../hooks.json")).unwrap(),
    )
    .unwrap();
    let routes = document["hooks"]
        .as_object()
        .unwrap()
        .values()
        .flat_map(|groups| groups.as_array().unwrap())
        .flat_map(|group| group["hooks"].as_array().unwrap())
        .map(|hook| {
            hook["command"]
                .as_str()
                .unwrap()
                .split_whitespace()
                .last()
                .unwrap()
                .to_string()
        })
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        routes,
        [
            "pre-tool",
            "post-edit",
            "post-shell",
            "post-compact",
            "stop",
            "session-start"
        ]
        .into_iter()
        .map(str::to_string)
        .collect()
    );
}

#[test]
#[ignore = "subprocess fixture invoked by hook_command_adapter_preserves_output_exit_and_timeout"]
fn adapter_stream_child() {
    std::io::stdout()
        .write_all(&vec![b'o'; 128 * 1024])
        .unwrap();
    std::io::stderr()
        .write_all(&vec![b'e'; 128 * 1024])
        .unwrap();
    std::process::exit(7);
}

#[test]
#[ignore = "subprocess fixture invoked by hook_command_adapter_preserves_output_exit_and_timeout"]
fn adapter_timeout_child() {
    std::thread::sleep(Duration::from_secs(5));
}

#[test]
fn hook_command_adapter_preserves_output_exit_and_timeout() {
    let fixture = Fixture::new();
    let binary = std::env::current_exe().unwrap();
    let result = util::run_command(
        binary.to_str().unwrap(),
        [
            "--ignored",
            "--exact",
            "tests::adapter_stream_child",
            "--nocapture",
        ],
        &fixture.root,
        Duration::from_secs(10),
    )
    .unwrap();
    assert_eq!(result.status.code(), Some(7));
    assert!(result.stdout.len() >= 128 * 1024);
    assert!(result.stderr.len() >= 128 * 1024);
    let started = Instant::now();
    let error = util::run_command(
        binary.to_str().unwrap(),
        [
            "--ignored",
            "--exact",
            "tests::adapter_timeout_child",
            "--nocapture",
        ],
        &fixture.root,
        Duration::from_millis(300),
    )
    .unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::TimedOut);
    assert!(started.elapsed() < Duration::from_secs(3));
}

#[test]
fn rejects_bad_payloads() {
    assert!(parse_payload(b"").is_err());
    assert!(parse_payload(b"[]").is_err());
    assert!(parse_payload(b"{broken").is_err());
}

#[test]
fn expands_apply_patch_move_targets() {
    let payload = json!({"tool_name":"apply_patch","tool_input":{"command":"*** Update File: inside.md\n*** Move to: D:/outside.md"}});
    let values = edits(&payload);
    assert_eq!(values.len(), 2);
    assert_eq!(values[1]["tool_input"]["file_path"], "D:/outside.md");
}
