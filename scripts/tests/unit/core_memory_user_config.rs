use super::*;
use std::time::{SystemTime, UNIX_EPOCH};

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
