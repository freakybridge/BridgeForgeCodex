use super::*;
#[path = "../support/artifact_paths.rs"]
mod test_artifacts;
use std::time::{SystemTime, UNIX_EPOCH};

fn config() -> Value {
    json!({"hooks":{"SessionStart":[{"matcher":"","hooks":[{"type":"command","command":"keep-custom"}]}]},
        "bridgeforgeProjectHooks":{"schema_version":1,"hooks":[{"id":"demo","events":[{"event":"SessionStart","args":["check"]},{"event":"Stop","args":["snapshot"]}]}]}})
}

#[test]
fn public_only_legacy_directory_does_not_parse_private_registry() {
    let temp = temporary();
    fs::create_dir_all(temp.0.join(".codex/bin")).unwrap();
    fs::write(
        temp.0.join(".codex/bin/bridgeforge.exe"),
        b"public placeholder",
    )
    .unwrap();
    fs::write(
        temp.0.join(".codex/hooks.json"),
        b"invalid private configuration",
    )
    .unwrap();
    fs::write(temp.0.join(REGISTRY_PATH), b"invalid private registry").unwrap();
    assert!(legacy_artifacts(&temp.0).unwrap().is_empty());
}

#[test]
fn legacy_project_outputs_require_registered_handlers_receipts_and_exact_bytes() {
    let temp = temporary();
    fs::create_dir_all(temp.0.join(".codex/bin")).unwrap();
    let config = config();
    let current = render(&config).unwrap();
    let old = serde_json::to_string(&current)
        .unwrap()
        .replace(".build/bridgeforge/bin/", ".codex/bin/");
    fs::write(temp.0.join(".codex/hooks.json"), old.as_bytes()).unwrap();
    fs::write(
        temp.0.join(REGISTRY_PATH),
        serde_json::to_vec(&config[REGISTRY]).unwrap(),
    )
    .unwrap();
    let image = temp.0.join(format!(
        ".codex/bin/project_demo{}",
        std::env::consts::EXE_SUFFIX
    ));
    fs::write(&image, b"old binary").unwrap();
    let receipt = json!({"schema_version":1,"id":"demo","input_sha256":sha(b"old source"),"platform":std::env::consts::OS,"binary_sha256":sha(b"old binary")});
    fs::write(
        temp.0.join(".codex/bin/build-receipt-project-demo.json"),
        serde_json::to_vec(&receipt).unwrap(),
    )
    .unwrap();
    assert_eq!(legacy_artifacts(&temp.0).unwrap().len(), 2);
    fs::write(
        temp.0.join(".codex/hooks.json"),
        old.replace("project_demo check", "project_demo changed"),
    )
    .unwrap();
    assert!(
        legacy_artifacts(&temp.0)
            .unwrap_err()
            .contains("external changes")
    );
    fs::write(temp.0.join(".codex/hooks.json"), old.as_bytes()).unwrap();
    fs::write(&image, b"unowned change").unwrap();
    assert!(
        legacy_artifacts(&temp.0)
            .unwrap_err()
            .contains("ownership unknown")
    );
    assert_eq!(fs::read(image).unwrap(), b"unowned change");
}

struct Temporary(PathBuf);
impl Drop for Temporary {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn temporary() -> Temporary {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = test_artifacts::root(env!("CARGO_MANIFEST_DIR"), "project-hooks")
        .join(format!("{:x}-{nonce:x}", std::process::id()));
    fs::create_dir(&root).unwrap();
    Temporary(root)
}

#[test]
fn registry_is_strict_and_render_is_idempotent_and_preserves_custom_handlers() {
    let original = config();
    let rendered = render(&original).unwrap();
    assert!(rendered.get(REGISTRY).is_none());
    let sidecar = serde_json::to_vec(&original[REGISTRY]).unwrap();
    assert_eq!(
        render(&with_registry(&rendered, Some(&sidecar)).unwrap()).unwrap(),
        rendered
    );
    assert_eq!(
        rendered["hooks"]["SessionStart"][0],
        original["hooks"]["SessionStart"][0]
    );
    assert_eq!(
        rendered["hooks"]["Stop"][0]["hooks"][0]["commandWindows"],
        ".build/bridgeforge/bin/project_demo.exe snapshot"
    );
    for id in ["../escape", "Foo", "a-b", "a/b", "con.exe", "a & calc"] {
        let mut invalid = original.clone();
        invalid[REGISTRY]["hooks"][0]["id"] = json!(id);
        assert!(hooks(&invalid).is_err(), "{id}");
    }
    for (key, value) in [
        ("args", json!(["--bad;evil"])),
        ("timeout", json!(0)),
        ("event", json!("Unknown")),
    ] {
        let mut invalid = original.clone();
        invalid[REGISTRY]["hooks"][0]["events"][0][key] = value;
        assert!(hooks(&invalid).is_err());
    }
    let mut duplicate = original.clone();
    duplicate[REGISTRY]["hooks"]
        .as_array_mut()
        .unwrap()
        .push(original[REGISTRY]["hooks"][0].clone());
    assert!(hooks(&duplicate).is_err());
    let mut orphan = rendered.clone();
    orphan.as_object_mut().unwrap().remove(REGISTRY);
    assert!(render(&orphan).is_err());
    assert_eq!(
        render(&json!({"hooks":{"Stop":[]}})).unwrap(),
        json!({"hooks":{"Stop":[]}})
    );
}

#[test]
fn standalone_registry_conflicts_and_missing_native_config_fail_closed() {
    let original = config();
    let registry = serde_json::to_vec(&original[REGISTRY]).unwrap();
    assert_eq!(with_registry(&original, Some(&registry)).unwrap(), original);
    assert!(
        with_registry(&original, Some(br#"{"schema_version":1,"hooks":[]}"#))
            .unwrap_err()
            .contains("conflicting")
    );
    assert!(
        with_registry(
            &json!({"hooks":{}}),
            Some(br#"{"schema_version":1,"schema_version":1,"hooks":[]}"#)
        )
        .is_err()
    );
    assert!(
        with_registry(
            &json!({"hooks":{}}),
            Some(br#"{"schema_version":1,"hooks":[],"unknown":true}"#)
        )
        .is_err()
    );
}
