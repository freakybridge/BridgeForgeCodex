#[path = "../support/factory_root.rs"]
mod factory_repository;

use super::*;

#[test]
fn minimum_supported_skeleton_version_rejects_old_projects_without_writes() {
    let token = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = test_artifacts::root(env!("CARGO_MANIFEST_DIR"), "baseline")
        .join(format!("bf-support-floor-{token}"));
    fs::create_dir_all(root.join(".codex")).unwrap();
    for version in ["0.1.0", "7.0.5", "8.0.0", "8.0.1", "9.0.0"] {
        let stamp = root.join(".codex/.bridgeforge_codex_version");
        fs::write(&stamp, version).unwrap();
        let result = require_supported_project(&root);
        assert_eq!(
            result.is_ok(),
            version.parse::<bridgeforge_core::SemVer>().unwrap()
                >= MIN_SUPPORTED_VERSION.parse().unwrap(),
            "{version}: {result:?}"
        );
        assert_eq!(fs::read_to_string(stamp).unwrap(), version);
        assert!(!root.join(".runtime").exists());
    }
    fs::remove_file(root.join(".codex/.bridgeforge_codex_version")).unwrap();
    for unknown in [
        b"{}".as_slice(),
        br#"{"release_version":null}"#,
        br#"{"release_version":7}"#,
        b"not-json",
    ] {
        let path = root.join(".codex/managed-skeleton.json");
        fs::write(&path, unknown).unwrap();
        assert!(require_supported_project(&root).is_err());
        assert_eq!(fs::read(path).unwrap(), unknown);
        assert!(!root.join(".runtime").exists());
    }
    fs::write(
        root.join(".codex/managed-skeleton.json"),
        br#"{"release_version":"7.0.5"}"#,
    )
    .unwrap();
    assert!(
        require_supported_project(&root)
            .unwrap_err()
            .contains("minimum supported")
    );
    assert!(!root.join(".runtime").exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn minimum_supported_version_does_not_use_business_or_package_versions() {
    let token = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = test_artifacts::root(env!("CARGO_MANIFEST_DIR"), "baseline")
        .join(format!("bf-support-business-{token}"));
    fs::create_dir_all(root.join(".codex")).unwrap();
    fs::write(root.join("VERSION"), b"0.1.0\n").unwrap();
    fs::write(
        root.join("Cargo.toml"),
        b"[package]\nname='business'\nversion='0.1.0'\n",
    )
    .unwrap();
    assert!(
        require_supported_project(&root).is_ok(),
        "new projects remain supported"
    );
    fs::write(root.join(".codex/.bridgeforge_codex_version"), b"8.0.0\n").unwrap();
    assert!(require_supported_project(&root).is_ok());
    assert_eq!(fs::read(root.join("VERSION")).unwrap(), b"0.1.0\n");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn strict_attribute_inspection_accepts_proven_defaults_and_never_falls_back() {
    for payload in [
        b"# defaults\n* text=auto eol=lf\n".as_slice(),
        b"* eol=lf text=auto\r\n",
    ] {
        verify_gitattributes_read_only(payload).unwrap();
    }
    for payload in [
        b"# no rule\n".as_slice(),
        b"* text=auto eol=lf\n*.bin -text\n",
        b"* text=auto eol=lf\nnested/* -text\n",
        b"[attr]default text=auto eol=lf\n* default\n",
    ] {
        assert!(verify_gitattributes_read_only(payload).is_err());
    }
}

#[test]
fn markdown_characterization_projection_boundaries_and_keys() {
    assert_eq!(
        heading_section("前言\n## 索引\n正文\n# 尾部".as_bytes(), "## 索引").unwrap(),
        "## 索引\n正文\n".as_bytes()
    );
    assert_eq!(
        heading_section(b"```md\n## Index\ninside\n```\n", "## Index").unwrap(),
        b"## Index\ninside\n```\n"
    );
    let unicode_rows =
        table_rows("| K | V |\n|---|---|\n| [`标题`](文档/Ä.md) | 保留 |\n".as_bytes()).unwrap();
    assert_eq!(
        unicode_rows["文档/ä.md"],
        "| [`标题`](文档/Ä.md) | 保留 |\n".as_bytes()
    );
    for ending in ["\n", "\r\n", "\r"] {
        let text = "# Root\n## Index\nbody\n### Child\nnested\n# End\ntail".replace('\n', ending);
        assert_eq!(
            heading_section(text.as_bytes(), "## Index").unwrap(),
            b"## Index\nbody\n### Child\nnested\n"
        );
    }
    assert_eq!(
        heading_section(b"## Index\nlast", "## Index").unwrap(),
        b"## Index\nlast"
    );
    assert_eq!(
        heading_section(b"## Index\nx\n  ## Next\ny", "## Index").unwrap(),
        b"## Index\nx\n"
    );
    assert_eq!(
        heading_section(b"## Index\n## Index", "## Index").unwrap_err(),
        "Markdown heading is missing or duplicated: ## Index"
    );
    assert_eq!(
        heading_section(&[0xff], "## Index").unwrap_err(),
        "managed Markdown is not UTF-8"
    );
    let rows = table_rows(b"| Name | Value |\n|---|---|\n| [`Label`](Target) | x |\n| [Label](Other) | y |\n| `MiXeD` | z |\n| | empty |\n").unwrap();
    assert_eq!(
        rows.keys().map(String::as_str).collect::<Vec<_>>(),
        vec!["", "[label](other)", "mixed", "target"]
    );
    assert_eq!(rows["target"], b"| [`Label`](Target) | x |\n");
    assert_eq!(
        table_rows(b"| K | V |\n|---|---|\n| A | x |\n| a | y |").unwrap_err(),
        "managed Markdown table key is duplicated: a"
    );
}

#[test]
fn markdown_characterization_projection_multitable_and_hash() {
    let payload = b"## Index\n| K | V |\n|---|---|\n| project | keep |\n<!-- example\n| K | V |\n|---|---|\n| managed | new |\n-->\n";
    let blocks = serde_json::json!({"headings":[],"keyed_tables":[{"heading":"## Index","managed_keys":["MANAGED"]}]});
    let expected = serde_json::json!({"headings":{},"keyed_tables":{"## Index":{"managed":sha(b"| managed | new |\n")}}});
    assert_eq!(markdown_projection(payload, &blocks).unwrap(), expected);
    let asset = serde_json::json!({"id":"fixture", "strategy":"merge", "managed_blocks":{
        "headings":[], "keyed_tables":blocks["keyed_tables"],
        "current_projection_sha256":canonical_sha(&expected).unwrap()
    }});
    verify_asset_payload(&asset, payload).unwrap();
    let drifted = String::from_utf8(payload.to_vec())
        .unwrap()
        .replace("managed | new", "managed | changed");
    assert_eq!(
        verify_asset_payload(&asset, drifted.as_bytes()).unwrap_err(),
        "managed Markdown projection drifted: fixture"
    );
}

#[test]
fn project_hook_index_reads_package_only_from_index() {
    struct IndexFiles(std::collections::BTreeMap<String, Vec<u8>>);
    impl ProcessRunner for IndexFiles {
        fn run(&self, request: &ProcessRequest) -> std::io::Result<crate::ProcessOutput> {
            let args = request
                .args
                .iter()
                .map(|s| s.to_string_lossy().to_string())
                .collect::<Vec<_>>();
            let stdout = if args[0] == "ls-files" {
                self.0
                    .keys()
                    .map(|p| format!("100644 abc 0\t.codex/hooks/project_demo/{p}\0"))
                    .collect::<String>()
                    .into_bytes()
            } else {
                let key = args
                    .last()
                    .unwrap()
                    .strip_prefix(":.codex/hooks/project_demo/")
                    .unwrap();
                return Ok(crate::ProcessOutput {
                    code: if self.0.contains_key(key) { 0 } else { 1 },
                    stdout: self.0.get(key).cloned().unwrap_or_default(),
                    stderr: Vec::new(),
                    timed_out: false,
                });
            };
            Ok(crate::ProcessOutput {
                code: 0,
                stdout,
                stderr: Vec::new(),
                timed_out: false,
            })
        }
    }
    let hook = crate::project_hooks::Hook {
        id: "demo".into(),
        events: Vec::new(),
    };
    let mut files = std::collections::BTreeMap::from([
        (
            "entrypoint.rs".into(),
            b"pub fn run(_:Vec<String>)->i32{0}".to_vec(),
        ),
        (
            "Cargo.toml".into(),
            b"[package]\nname=\"demo\"\nversion=\"0.1.0\"\n".to_vec(),
        ),
    ]);
    assert!(
        project_hook_index_input(
            Path::new("absent-worktree"),
            &hook,
            &IndexFiles(files.clone())
        )
        .unwrap_err()
        .contains("Cargo.lock")
    );
    files.insert("Cargo.lock".into(), b"version=4\n".to_vec());
    for name in [
        ".cargo/config.toml",
        ".CARGO/config.toml",
        "nested/.CaRgO/config",
        ".BRIDGEFORGE-MAIN.RS",
        "nested/.BridgeForge-Main.rs",
    ] {
        let mut reserved = files.clone();
        reserved.insert(name.into(), b"reserved".to_vec());
        assert!(
            project_hook_index_input(Path::new("absent-worktree"), &hook, &IndexFiles(reserved))
                .unwrap_err()
                .contains("reserved"),
            "{name}"
        );
    }
    let input =
        project_hook_index_input(Path::new("absent-worktree"), &hook, &IndexFiles(files)).unwrap();
    assert_eq!(input.package.unwrap().len(), 3);
}

fn factory_root() -> PathBuf {
    factory_repository::from_manifest_dir(env!("CARGO_MANIFEST_DIR")).to_path_buf()
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
    contract["release_model"] = Value::String("product-packages".into());
    assert!(validate_contract(&contract).is_ok());
    contract["release_model"] = Value::String("synchronized".into());
    assert!(
        validate_contract(&contract)
            .unwrap_err()
            .contains("release_model")
    );
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
    let root = factory_repository::from_manifest_dir(env!("CARGO_MANIFEST_DIR"))
        .join(".runtime/tests/attributes");
    fs::create_dir_all(&root).unwrap();
    verify_gitattributes(b"* text=auto eol=lf\n", Some(&root)).unwrap();
    let error = verify_gitattributes(b"* text=auto eol=lf\n* eol=crlf\n", Some(&root)).unwrap_err();
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

#[allow(dead_code)]
#[path = "../support/artifact_paths.rs"]
mod test_artifacts;
