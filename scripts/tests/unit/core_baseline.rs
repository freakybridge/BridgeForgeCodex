use super::*;

fn factory_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .unwrap()
        .to_path_buf()
}

fn contract_and_asset(id: &str) -> (Value, Value) {
    let contract = load(&factory_root().join("templates/managed-skeleton.json")).unwrap();
    let asset = contract["assets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|asset| asset["id"].as_str() == Some(id))
        .unwrap()
        .clone();
    (contract, asset)
}

#[test]
fn rejects_unsafe_target() {
    assert!(safe_target(Path::new("."), &Value::String("../x".into()), "x").is_err());
    assert!(safe_target(Path::new("."), &Value::String("a\\b".into()), "x").is_err());
}

#[test]
fn contract_schema_is_exact_and_asset_identities_are_unique() {
    let (mut contract, _) = contract_and_asset("codex.hooks-config");
    contract["unexpected"] = Value::Bool(true);
    assert!(validate_contract(&contract).is_err());

    let (mut contract, _) = contract_and_asset("codex.hooks-config");
    let duplicate = contract["assets"][0].clone();
    contract["assets"].as_array_mut().unwrap().push(duplicate);
    assert!(validate_contract(&contract).is_err());

    assert!(
        parse_unique_json(br#"{"schema_version":4,"schema_version":4}"#, "contract")
            .unwrap_err()
            .contains("duplicate JSON key")
    );
}

#[test]
fn agents_contract_and_payload_require_both_unique_zones() {
    let (_, mut asset) = contract_and_asset("root.agents");
    asset["agents_zones"]["project"]
        .as_object_mut()
        .unwrap()
        .remove("end");
    let (mut contract, _) = contract_and_asset("root.agents");
    let position = contract["assets"]
        .as_array()
        .unwrap()
        .iter()
        .position(|candidate| candidate["id"].as_str() == Some("root.agents"))
        .unwrap();
    contract["assets"][position] = asset;
    assert!(validate_contract(&contract).is_err());

    let (_, asset) = contract_and_asset("root.agents");
    let source = factory_root().join(asset["source"].as_str().unwrap());
    let payload = fs::read(source).unwrap();
    verify_asset_payload(&asset, &payload).unwrap();
    let drifted = String::from_utf8(payload)
        .unwrap()
        .replace("<!-- BRIDGEFORGE:PROJECT:END -->", "")
        .into_bytes();
    assert!(verify_asset_payload(&asset, &drifted).is_err());
}

#[test]
fn gitattributes_uses_effective_git_semantics() {
    verify_gitattributes(b"* text=auto eol=lf\n").unwrap();
    let error = verify_gitattributes(b"* text=auto eol=lf\n* eol=crlf\n").unwrap_err();
    assert!(error.contains("overridden"), "{error}");
}

#[test]
fn real_managed_markdown_projection_and_link_keys_are_verified() {
    let (_, asset) = contract_and_asset("codex.doc.readme");
    let source = factory_root().join(asset["source"].as_str().unwrap());
    let payload = fs::read(source).unwrap();
    verify_asset_payload(&asset, &payload).unwrap();
    let drifted = String::from_utf8(payload)
        .unwrap()
        .replace(
            "3_reference/codex-hook-signals.md",
            "3_reference/codex-hook-signals-drifted.md",
        )
        .into_bytes();
    assert!(verify_asset_payload(&asset, &drifted).is_err());
}

#[test]
fn real_hooks_require_exact_identity_event_matcher_and_handler_hash() {
    let (_, asset) = contract_and_asset("codex.hooks-config");
    let source = factory_root().join(asset["source"].as_str().unwrap());
    let payload = fs::read(source).unwrap();
    verify_asset_payload(&asset, &payload).unwrap();

    let mut document: Value = serde_json::from_slice(&payload).unwrap();
    document["hooks"]["Stop"][0]["hooks"][0]["command"] = Value::String("wrong".into());
    assert!(verify_asset_payload(&asset, &serde_json::to_vec(&document).unwrap()).is_err());

    let duplicate = document["hooks"]["Stop"][0]["hooks"][0].clone();
    document["hooks"]["SessionStart"][0]["hooks"]
        .as_array_mut()
        .unwrap()
        .push(duplicate);
    assert!(verify_asset_payload(&asset, &serde_json::to_vec(&document).unwrap()).is_err());
}

#[test]
fn generic_merge_requires_the_full_declared_json_subset() {
    let (_, asset) = contract_and_asset("codex.settings");
    let source = factory_root().join(asset["source"].as_str().unwrap());
    let payload = fs::read(source).unwrap();
    verify_asset_payload(&asset, &payload).unwrap();
    let mut document: Value = serde_json::from_slice(&payload).unwrap();
    document["permissions"]
        .as_object_mut()
        .unwrap()
        .remove("defaultMode");
    assert!(verify_asset_payload(&asset, &serde_json::to_vec(&document).unwrap()).is_err());
}
