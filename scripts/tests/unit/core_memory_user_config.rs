use super::*;
#[cfg(windows)]
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(windows)]
fn python_hook_fixture(home: &Path) -> Value {
    let parent = home.parent().unwrap().to_string_lossy();
    let mut document = json!({"description":"retained", "hooks":{}});
    for event in ["SessionStart", "Stop", "SessionEnd"] {
        let mut handler: Value = serde_json::from_str(r#"{
          "type":"command",
          "command":"root=\"$(git rev-parse --show-toplevel)\" && \"$root/.venv/Scripts/python.exe\" 'C:\\Users\\test\\.bridgeforge-codex\\scripts\\codex_memory_sync.py' hook-run --event SessionStart --project-root \"$root\"",
          "commandWindows":"powershell.exe -NoProfile -NonInteractive -WindowStyle Hidden -ExecutionPolicy Bypass -File \"C:\\Users\\test\\.bridgeforge-codex\\scripts\\codex_memory_sync_hook.ps1\" SessionStart",
          "bridgeforgeCodexId":"bridgeforge-codex.native-memory-sync.v1:SessionStart",
          "timeout":120
        }"#).unwrap();
        for key in ["command", "commandWindows", "bridgeforgeCodexId"] {
            handler[key] = Value::String(
                handler[key]
                    .as_str()
                    .unwrap()
                    .replace(r"C:\Users\test", &parent)
                    .replace("SessionStart", event),
            );
        }
        if event == "Stop" {
            handler["async"] = json!(true);
        }
        if event == "SessionEnd" {
            handler["timeout"] = json!(3);
        }
        document["hooks"][event] = json!([{"hooks":[handler]}]);
    }
    document["hooks"]["UserPromptSubmit"] = json!([{"hooks":[{
        "type":"command", "command":"external-keep", "timeout":7
    }]}]);
    document
}

#[cfg(windows)]
#[test]
fn exact_official_python_handlers_migrate_to_rust_and_preserve_external_hooks() {
    let home = std::env::temp_dir()
        .join(format!(
            "bf-python-hook-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
        .join(".codex");
    fs::create_dir_all(&home).unwrap();
    let before = python_hook_fixture(&home);
    let binary = home.join("bin/bridgeforge.exe");
    fs::write(home.join("hooks.json"), before.to_string()).unwrap();
    assert!(!user_hooks_healthy(&home, &binary));
    assert!(merge_user_hooks(&home, &binary).unwrap());
    assert!(user_hooks_healthy(&home, &binary));
    let after = load_document(&fs::read(home.join("hooks.json")).unwrap(), "test").unwrap();
    assert_eq!(after["description"], before["description"]);
    assert_eq!(
        after["hooks"]["UserPromptSubmit"],
        before["hooks"]["UserPromptSubmit"]
    );
    assert!(!after.to_string().contains("python.exe"));
    assert!(!merge_user_hooks(&home, &binary).unwrap());
    fs::remove_dir_all(home.parent().unwrap()).unwrap();
}

#[cfg(windows)]
#[test]
fn partial_official_python_hooks_migrate_and_fill_missing_events() {
    let home = std::env::temp_dir()
        .join(format!("bf-python-partial-{}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()))
        .join(".codex");
    fs::create_dir_all(&home).unwrap();
    let mut before = python_hook_fixture(&home);
    before["hooks"].as_object_mut().unwrap().remove("Stop");
    before["hooks"].as_object_mut().unwrap().remove("SessionEnd");
    let binary = home.join("bin/bridgeforge.exe");
    fs::write(home.join("hooks.json"), before.to_string()).unwrap();
    assert!(merge_user_hooks(&home, &binary).unwrap());
    assert!(user_hooks_healthy(&home, &binary));
    let after = load_document(&fs::read(home.join("hooks.json")).unwrap(), "test").unwrap();
    assert_eq!(after["description"], before["description"]);
    assert_eq!(after["hooks"]["UserPromptSubmit"], before["hooks"]["UserPromptSubmit"]);
    for event in HOOK_EVENTS {
        assert_eq!(after["hooks"][event].as_array().unwrap().len(), 1);
    }
    assert!(!merge_user_hooks(&home, &binary).unwrap());
    fs::remove_dir_all(home.parent().unwrap()).unwrap();
}

#[cfg(windows)]
#[test]
fn modified_python_handler_blocks_entire_migration_without_writes() {
    let home = std::env::temp_dir()
        .join(format!(
            "bf-python-drift-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
        .join(".codex");
    fs::create_dir_all(&home).unwrap();
    for change in [
        "command",
        "commandWindows",
        "timeout",
        "matcher",
        "identity",
    ] {
        let mut document = python_hook_fixture(&home);
        let group = &mut document["hooks"]["Stop"][0];
        match change {
            "matcher" => group["matcher"] = json!("custom"),
            "timeout" => group["hooks"][0]["timeout"] = json!(121),
            "identity" => {
                group["hooks"][0]["bridgeforgeCodexId"] =
                    json!("bridgeforge-codex.native-memory-sync.v1:Other")
            }
            key => group["hooks"][0][key] = json!("custom command"),
        }
        let payload = document.to_string();
        fs::write(home.join("hooks.json"), &payload).unwrap();
        assert!(
            merge_user_hooks(&home, &home.join("bin/bridgeforge.exe")).is_err(),
            "{change}"
        );
        assert_eq!(
            fs::read_to_string(home.join("hooks.json")).unwrap(),
            payload
        );
    }
    fs::remove_dir_all(home.parent().unwrap()).unwrap();
}

#[test]
fn expected_handlers_use_rust_binary_only() {
    let value = expected_document(
        Path::new("/tools/bridgeforge"),
        Path::new("/home/me/.codex"),
    );
    let text = value.to_string();
    assert!(text.contains("bridgeforge"));
    assert!(!text.contains("python"));
    assert!(!text.contains(".venv"));
    let hooks = value["hooks"].as_object().unwrap();
    assert_eq!(hooks.len(), 3);
    for (event, timeout, asynchronous) in [
        ("SessionStart", 120, None),
        ("Stop", 120, Some(true)),
        ("SessionEnd", 3, None),
    ] {
        let groups = hooks[event].as_array().unwrap();
        assert_eq!(groups.len(), 1);
        let handlers = groups[0]["hooks"].as_array().unwrap();
        assert_eq!(handlers.len(), 1);
        let handler = &handlers[0];
        assert_eq!(handler["type"], "command");
        assert_eq!(handler["timeout"], timeout);
        assert_eq!(handler.get("async").and_then(Value::as_bool), asynchronous);
        assert!(
            handler["command"]
                .as_str()
                .unwrap()
                .contains(&format!("memory-sync hook-run --event {event}"))
        );
        assert!(
            handler["commandWindows"]
                .as_str()
                .unwrap()
                .contains(&format!("memory-sync hook-run --event {event}"))
        );
    }
}

#[cfg(windows)]
#[test]
fn native_memory_hook_generation_uses_stable_windows_path_spelling() {
    assert_eq!(
        expected_document(
            Path::new(r"C:\Program Files\BridgeForge\bridgeforge.exe"),
            Path::new(r"C:\Users\test\.codex"),
        ),
        expected_document(
            Path::new("C:/Program Files/BridgeForge/bridgeforge.exe"),
            Path::new("C:/Users/test/.codex"),
        )
    );
}

#[cfg(windows)]
#[test]
fn native_memory_hook_path_aliases_are_healthy_and_repair_is_noop() {
    let token = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let home = std::env::temp_dir().join(format!("memory hook aliases {token}"));
    let forward_home = PathBuf::from(home.to_string_lossy().replace('\\', "/"));
    let backward_home = PathBuf::from(home.to_string_lossy().replace('/', "\\"));
    let binary = Path::new(r"C:\Program Files\BridgeForge\bridgeforge.exe");
    fs::create_dir_all(&home).unwrap();
    for style in ["forward", "backward", "mixed"] {
        let mut document = expected_document(binary, &home);
        for event in HOOK_EVENTS {
            for key in ["command", "commandWindows"] {
                let value = document["hooks"][event][0]["hooks"][0][key]
                    .as_str()
                    .unwrap();
                let value = match style {
                    "backward" => value.replace('/', "\\"),
                    "mixed" => value.replacen('/', "\\", 1),
                    _ => value.to_string(),
                };
                document["hooks"][event][0]["hooks"][0][key] = Value::String(value);
            }
        }
        document["hooks"]["UserPromptSubmit"] = json!([{
            "hooks": [{"type": "command", "command": "external /keep\\literal", "timeout": 7}]
        }]);
        let before = super::super::ownership::render_document(&document).unwrap();
        fs::write(home.join("hooks.json"), &before).unwrap();
        for spelling in [&forward_home, &backward_home] {
            assert!(user_hooks_healthy(spelling, binary), "{style}");
            assert!(!merge_user_hooks(spelling, binary).unwrap(), "{style}");
            assert_eq!(fs::read(home.join("hooks.json")).unwrap(), before);
        }
    }
    fs::remove_dir_all(home).unwrap();
}

#[cfg(windows)]
#[test]
fn native_memory_path_aliases_do_not_hide_real_hook_changes() {
    let token = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let home = std::env::temp_dir().join(format!("memory-hook-drift-{token}"));
    let binary = Path::new(r"C:\tools\bridgeforge.exe");
    fs::create_dir_all(&home).unwrap();
    for change in [
        "binary", "home", "suffix", "event", "timeout", "async", "type", "id", "extra",
    ] {
        let mut document = expected_document(binary, &home);
        let handler = &mut document["hooks"]["SessionEnd"][0]["hooks"][0];
        let command = handler["commandWindows"]
            .as_str()
            .unwrap()
            .replace('/', "\\");
        handler["commandWindows"] = Value::String(command.clone());
        match change {
            "binary" => {
                handler["commandWindows"] = command.replace("bridgeforge.exe", "other.exe").into()
            }
            "home" => {
                handler["commandWindows"] = command
                    .replace("memory-hook-drift-", "different-home-")
                    .into()
            }
            "suffix" => handler["commandWindows"] = format!("{command} && other.exe").into(),
            "event" => {
                handler["commandWindows"] =
                    command.replace("--event SessionEnd", "--event Stop").into()
            }
            "timeout" => handler["timeout"] = 120.into(),
            "async" => handler["async"] = true.into(),
            "type" => handler["type"] = "prompt".into(),
            "id" => handler[MANAGED_ID_KEY] = format!("{HOOK_ID}:unknown").into(),
            "extra" => handler["unrecognized"] = true.into(),
            _ => unreachable!(),
        }
        let before = super::super::ownership::render_document(&document).unwrap();
        fs::write(home.join("hooks.json"), &before).unwrap();
        assert!(!user_hooks_healthy(&home, binary), "{change}");
        assert!(merge_user_hooks(&home, binary).is_err(), "{change}");
        assert_eq!(
            fs::read(home.join("hooks.json")).unwrap(),
            before,
            "{change}"
        );
    }
    fs::remove_dir_all(home).unwrap();
}

#[test]
fn native_memory_path_comparison_preserves_quoting_and_non_drive_paths() {
    let binary = Path::new("C:/Program Files/a'b/bridgeforge.exe");
    let home = Path::new("C:/Users/test/.codex");
    let expected = expected_document(binary, home);
    let command = expected["hooks"]["SessionEnd"][0]["hooks"][0]["command"]
        .as_str()
        .unwrap();
    assert!(hook_command_matches(
        command,
        binary,
        home,
        "SessionEnd",
        shell_quote
    ));
    assert!(!hook_command_matches(
        &command.replace("'\\''", "'/''"),
        binary,
        home,
        "SessionEnd",
        shell_quote
    ));
    for path in [
        "relative/path",
        "/home/test/.codex",
        "//server/share/.codex",
        "//?/C:/Users/test/.codex",
    ] {
        let quoted = shell_quote(path);
        assert!(!quoted_path_matches(
            &quoted.replace('/', "\\"),
            Path::new(path),
            shell_quote
        ));
    }
}

#[cfg(windows)]
#[test]
fn native_memory_windows_trailing_backslash_is_not_a_path_alias() {
    for path in ["C:/Users/test/.codex/", "C:/Users/test/.codex//", "C:/"] {
        let quoted = windows_quote(path);
        assert!(quoted_path_matches(&quoted, Path::new(path), windows_quote));
        assert!(!quoted_path_matches(
            &quoted.replace('/', "\\"),
            Path::new(path),
            windows_quote,
        ));
    }
    let token = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let home = std::env::temp_dir().join(format!("memory-hook-trailing-{token}"));
    let spelling = PathBuf::from(format!("{}/", home.to_string_lossy().replace('\\', "/")));
    let binary = Path::new("C:/tools/bridgeforge.exe");
    fs::create_dir_all(&home).unwrap();
    let mut document = expected_document(binary, &spelling);
    let handler = &mut document["hooks"]["SessionEnd"][0]["hooks"][0];
    handler["commandWindows"] = handler["commandWindows"]
        .as_str()
        .unwrap()
        .replace('/', "\\")
        .into();
    let before = super::super::ownership::render_document(&document).unwrap();
    fs::write(home.join("hooks.json"), &before).unwrap();
    assert!(!user_hooks_healthy(&spelling, binary));
    assert!(merge_user_hooks(&spelling, binary).is_err());
    assert_eq!(fs::read(home.join("hooks.json")).unwrap(), before);
    fs::remove_dir_all(home).unwrap();
}

#[test]
fn user_hooks_writer_refuses_an_active_shared_lock() {
    let token = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let home = std::env::temp_dir().join(format!("bridgeforge-user-hooks-lock-{token}"));
    fs::create_dir_all(home.join(".bridgeforge-codex")).unwrap();
    let held = UserHooksLock::acquire(&home).unwrap();
    let error = merge_user_hooks(&home, Path::new("bridgeforge")).unwrap_err();
    assert!(error.to_string().contains("update is running"));
    drop(held);
    assert!(merge_user_hooks(&home, Path::new("bridgeforge")).is_ok());
    fs::remove_dir_all(home).unwrap();
}

#[test]
fn annotated_quoted_and_inline_tables_are_edited_without_duplication() {
    for input in [
        "# keep me\n[features] # user comment\nmemories = false # preference\n",
        "[\"features\"] # quoted table\n\"memories\" = false # preference\n",
        "features = { memories = false } # inline table\n",
        "features.memories = false # dotted key\n",
    ] {
        let token = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let home = std::env::temp_dir().join(format!("bridgeforge-toml-{token}"));
        fs::create_dir_all(&home).unwrap();
        fs::write(home.join("config.toml"), input).unwrap();
        assert!(enable_memories(&home, true).unwrap());
        let updated = fs::read_to_string(home.join("config.toml")).unwrap();
        let parsed = parse_config(&updated).unwrap();
        assert_eq!(parsed["features"]["memories"].as_bool(), Some(true));
        assert!(memories_enabled(&home));
        for comment in [
            "keep me",
            "user comment",
            "preference",
            "quoted table",
            "inline table",
            "dotted key",
        ] {
            if input.contains(comment) {
                assert!(updated.contains(comment), "lost comment: {comment}");
            }
        }
        assert!(!enable_memories(&home, true).unwrap());
        assert_eq!(
            fs::read_to_string(home.join("config.toml")).unwrap(),
            updated
        );
        fs::remove_dir_all(home).unwrap();
    }
}

#[test]
fn invalid_or_wrong_shaped_toml_is_rejected_without_writing() {
    for input in [
        "[features]\n[features]\n",
        "features = 42\n",
        "unclosed = [\n",
    ] {
        let token = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let home = std::env::temp_dir().join(format!("bridgeforge-toml-invalid-{token}"));
        fs::create_dir_all(&home).unwrap();
        let config = home.join("config.toml");
        fs::write(&config, input).unwrap();
        assert!(enable_memories(&home, true).is_err());
        assert_eq!(fs::read_to_string(&config).unwrap(), input);
        assert!(!memories_enabled(&home));
        assert!(!home.join("config.toml.tmp").exists());
        fs::remove_dir_all(home).unwrap();
    }
}
