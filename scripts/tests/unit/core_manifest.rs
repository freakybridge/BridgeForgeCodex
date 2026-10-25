use super::*;

#[test]
fn hook_asset_ids_are_stable() {
    assert_eq!(
        hook_asset_id("crates/core/src/lib.rs"),
        "codex.hooks.crates-core-src-lib-rs"
    );
}

#[test]
fn canonical_hash_ignores_object_insertion_order() {
    assert_eq!(
        canonical_sha(&json!({"b": 1, "a": 2})).unwrap(),
        canonical_sha(&json!({"a": 2, "b": 1})).unwrap()
    );
}

#[test]
fn factory_skill_mirrors_are_explicit_private_and_readonly() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let contract: Value =
        serde_json::from_slice(include_bytes!("../../../source/factory/src/dogfood.json")).unwrap();
    let before = contract["assets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|asset| {
            let target = asset["target"].as_str().unwrap().to_owned();
            let bytes = fs::read(root.join(&target)).unwrap();
            (target, bytes)
        })
        .collect::<BTreeMap<_, _>>();
    let outputs = crate::dogfood::render(root).unwrap();
    assert_eq!(outputs.len(), 4);
    let public: Value = serde_json::from_slice(&render_managed_contract(root).unwrap()).unwrap();
    for asset in contract["assets"].as_array().unwrap() {
        let source = asset["source"].as_str().unwrap();
        let target = asset["target"].as_str().unwrap();
        assert_eq!(outputs[target], fs::read(root.join(source)).unwrap());
        assert!(
            public["assets"]
                .as_array()
                .unwrap()
                .iter()
                .all(|a| a["target"] != target)
        );
        assert_eq!(fs::read(root.join(target)).unwrap(), before[target]);
    }
}

#[test]
fn actual_source_layout_preserves_asset_ids_and_public_boundary_without_writes() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let before = fs::read(root.join("templates/managed-skeleton.json")).unwrap();
    let rendered = render_managed_contract(root).unwrap();
    let contract: Value = serde_json::from_slice(&rendered).unwrap();
    crate::baseline::validate_contract(&contract).unwrap();
    for target in [
        ".codex/hooks/project_custom/entrypoint.rs",
        ".codex/hooks/Project_custom/entrypoint.rs",
    ] {
        let mut invalid = contract.clone();
        invalid["retired_assets"][0]["target"] = json!(target);
        assert!(
            crate::baseline::validate_contract(&invalid)
                .unwrap_err()
                .contains("invalid retired source asset")
        );
    }
    let layout: Value =
        serde_json::from_slice(&fs::read(root.join("templates/source-layout.json")).unwrap())
            .unwrap();
    let assets = contract["assets"].as_array().unwrap();
    for row in layout["relocations"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r.get("source").is_some())
    {
        let item = assets
            .iter()
            .find(|a| a["source"] == row["source"])
            .unwrap();
        assert_eq!(item["id"], row["id"]);
        assert_eq!(item["target"], row["target"]);
    }
    assert!(
        assets
            .iter()
            .all(|a| !a["source"].as_str().unwrap().starts_with("source/factory/"))
    );
    assert_eq!(contract["generated_assets"].as_array().unwrap().len(), 2);
    assert_eq!(
        fs::read(root.join("templates/managed-skeleton.json")).unwrap(),
        before
    );
}

#[test]
fn consumer_lock_follows_candidate_native_versions_and_dependency_closure() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let original = fs::read(root.join("Cargo.lock")).unwrap();
    let mut lock = String::from_utf8(original)
        .unwrap()
        .parse::<toml_edit::DocumentMut>()
        .unwrap();
    for entry in lock["package"].as_array_of_tables_mut().unwrap().iter_mut() {
        if entry["name"].as_str() == Some("bridgeforge-core") {
            entry["version"] = toml_edit::value("7.0.0");
        }
    }
    let candidate = BTreeMap::from([("Cargo.lock".into(), lock.to_string().into_bytes())]);
    let consumer = consumer_lock(root, &candidate).unwrap();
    let document = String::from_utf8(consumer)
        .unwrap()
        .parse::<toml_edit::DocumentMut>()
        .unwrap();
    let entries = document["package"].as_array_of_tables().unwrap();
    assert_eq!(
        entries
            .iter()
            .find(|e| e["name"].as_str() == Some("bridgeforge-core"))
            .unwrap()["version"]
            .as_str(),
        Some("7.0.0")
    );
    assert!(
        entries
            .iter()
            .any(|e| e["name"].as_str() == Some("bridgeforge-factory-tests"))
    );
    assert!(
        !entries
            .iter()
            .any(|e| e["name"].as_str() == Some("bridgeforge-cli"))
    );
}

#[test]
fn manifest_install_failure_rolls_back_own_writes_and_preserves_external_edits() {
    use std::time::{SystemTime, UNIX_EPOCH};
    let root = std::env::temp_dir().join(format!(
        "bf-manifest-tx-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&root).unwrap();
    let (a, b, c) = (root.join("a"), root.join("b"), root.join("c"));
    let targets = vec![
        (a.clone(), b"new-a".to_vec()),
        (b.clone(), b"new-b".to_vec()),
        (c.clone(), b"new-c".to_vec()),
    ];
    for change_installed in [false, true] {
        fs::write(&a, b"old-a").unwrap();
        fs::write(&b, b"old-b").unwrap();
        let before = BTreeMap::from([
            (a.clone(), Some(b"old-a".to_vec())),
            (b.clone(), Some(b"old-b".to_vec())),
            (c.clone(), None),
        ]);
        let error = install_targets(&targets, &before, &|index| {
            if index == 1 {
                fs::write(if change_installed { &a } else { &b }, b"external").unwrap();
                return Err("injected source drift".into());
            }
            Ok(())
        })
        .unwrap_err();
        assert!(error.contains("injected source drift"), "{error}");
        assert_eq!(
            fs::read(&a).unwrap(),
            if change_installed {
                b"external".as_slice()
            } else {
                b"old-a".as_slice()
            }
        );
        assert_eq!(
            fs::read(&b).unwrap(),
            if change_installed {
                b"old-b".as_slice()
            } else {
                b"external".as_slice()
            }
        );
        assert!(!c.exists());
    }
    fs::write(&a, b"old-a").unwrap();
    fs::write(&b, b"old-b").unwrap();
    let before = BTreeMap::from([
        (a.clone(), Some(b"old-a".to_vec())),
        (b.clone(), Some(b"old-b".to_vec())),
        (c.clone(), None),
    ]);
    let error = install_targets(&targets, &before, &|index| {
        if index == 2 {
            fs::remove_file(&b).unwrap();
            fs::create_dir(&b).unwrap();
            return Err("injected external directory".into());
        }
        Ok(())
    })
    .unwrap_err();
    assert!(
        error.contains("external rollback state preserved"),
        "{error}"
    );
    assert_eq!(
        fs::read(&a).unwrap(),
        b"old-a",
        "one unreadable target must not abort remaining rollback"
    );
    assert!(b.is_dir());
    assert!(!c.exists());
    fs::remove_dir_all(root).unwrap();
}
