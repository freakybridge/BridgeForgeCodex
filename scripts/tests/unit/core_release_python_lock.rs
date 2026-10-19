use super::super::model::NativeDependency;
use super::*;

fn fixture(format: &str, lock: &str) -> (LockSpec, LanguageSnapshot) {
    let path = match format {
        "uv" => "uv.lock",
        "poetry" => "poetry.lock",
        "pdm" => "pdm.lock",
        "pylock" => "pylock.toml",
        _ => "requirements.txt",
    };
    let spec = LockSpec {
        path: path.into(),
        format: format.into(),
        members: vec!["pyproject.toml".into(), "dep/pyproject.toml".into()],
        metadata: None,
        tool_version: None,
    };
    let mut snapshot = LanguageSnapshot::default();
    for (manifest, name) in [("pyproject.toml", "app"), ("dep/pyproject.toml", "dep")] {
        snapshot.packages.insert(
            manifest.into(),
            NativePackage {
                manifest: manifest.into(),
                name: name.into(),
                version: "1.0.0".into(),
                language: Language::Python,
                workspace: "pyproject.toml".into(),
                source_roots: vec![],
            },
        );
    }
    snapshot.dependencies.push(NativeDependency {
        owner: "pyproject.toml".into(),
        target: "dep/pyproject.toml".into(),
        kind: "runtime".into(),
    });
    snapshot
        .inputs
        .insert(path.into(), Some(lock.as_bytes().to_vec()));
    snapshot.inputs.insert("pyproject.toml".into(), Some(b"[project]\nname='app'\nversion='1.0.0'\ndependencies=['dep==1.0.0']\nrequires-python='>=3.10'\n".to_vec()));
    snapshot.inputs.insert(
        "dep/pyproject.toml".into(),
        Some(b"[project]\nname='dep'\nversion='1.0.0'\n".to_vec()),
    );
    (spec, snapshot)
}

fn versions() -> BTreeMap<String, String> {
    BTreeMap::from([
        ("pyproject.toml".into(), "1.0.1".into()),
        ("dep/pyproject.toml".into(), "1.1.0".into()),
    ])
}

const UV: &str = "version = 1\nrevision = 3\n[[package]]\nname = 'app'\nversion = '1.0.0'\nsource = { editable = '.' }\ndependencies = [{name = 'dep', version = '1.0.0', source = {editable = 'dep'}}]\n[package.metadata]\nrequires-dist = [{name = 'dep', specifier = '==1.0.0', editable = 'dep'}]\n[[package]]\nname = 'dep'\nversion = '1.0.0'\nsource = { editable = 'dep' }\n[[package]]\nname = 'third-party'\nversion = '9.0.0'\nsource = { registry = 'https://example.invalid/simple' }\nwheels = [{url = 'https://example.invalid/third.whl', hash = 'sha256:original'}]\n";
const POETRY: &str = "[[package]]\nname='dep'\nversion='1.0.0'\nfiles=[]\n[package.source]\ntype='directory'\nurl='dep'\n[metadata]\nlock-version='2.1'\ncontent-hash='5170cc942c42e940c5c95387895254a71392121345e6cf375c7c4ad8d60999cb'\n";
const PDM: &str = "[[package]]\nname='dep'\nversion='1.0.0'\npath='dep'\neditable=true\n[metadata]\nlock_version='4.5.0'\ncontent_hash='sha256:59edfb9de99116f3a84847045c1f0069e8a5b960e54f707d91a4e53d6f2c245a'\n";
const PYLOCK: &str = "lock-version='1.0'\ncreated-by='uv'\n[[packages]]\nname='app'\ndependencies=[{name='dep'}]\n[packages.directory]\npath='.'\neditable=true\n[[packages]]\nname='dep'\n[packages.directory]\npath='dep'\neditable=true\n";

#[test]
fn uv_updates_source_versions_edges_metadata_without_touching_artifacts() {
    let (spec, snapshot) = fixture("uv", UV);
    validate(&[spec.clone()], &snapshot).unwrap();
    let writes = render(&[spec.clone()], &snapshot, &versions()).unwrap();
    let after = std::str::from_utf8(&writes["uv.lock"])
        .unwrap()
        .parse::<DocumentMut>()
        .unwrap();
    let packages = after["package"].as_array_of_tables().unwrap();
    assert_eq!(packages.get(0).unwrap()["version"].as_str(), Some("1.0.1"));
    assert_eq!(packages.get(1).unwrap()["version"].as_str(), Some("1.1.0"));
    assert!(after.to_string().contains("==1.1.0"));
    assert!(after.to_string().contains("hash = 'sha256:original'"));
    assert_eq!(packages.get(2).unwrap()["version"].as_str(), Some("9.0.0"));
    assert_eq!(snapshot.inputs["uv.lock"].as_deref(), Some(UV.as_bytes()));
}

#[test]
fn uv_registry_same_name_is_distinguished_by_source() {
    let lock = format!(
        "{UV}[[package]]\nname='dep'\nversion='1.0.0'\nsource={{registry='https://example.invalid/simple'}}\nwheels=[{{hash='sha256:registry-original'}}]\n"
    );
    let (spec, snapshot) = fixture("uv", &lock);
    let writes = render(&[spec], &snapshot, &versions()).unwrap();
    let after = std::str::from_utf8(&writes["uv.lock"])
        .unwrap()
        .parse::<DocumentMut>()
        .unwrap();
    assert_eq!(
        after["package"]
            .as_array_of_tables()
            .unwrap()
            .get(3)
            .unwrap()["version"]
            .as_str(),
        Some("1.0.0")
    );
    assert!(after.to_string().contains("sha256:registry-original"));
}

#[test]
fn uv_rejects_unresolved_removed_or_renamed_local_topology() {
    for lock in [
        UV.replace("editable = 'dep'", "editable = 'old-dep'"),
        UV.replace("name = 'dep'\nversion", "name = 'old-dep'\nversion"),
        UV.replace(
            "version = '1.0.0'\nsource = { editable = 'dep' }",
            "version = '0.9.0'\nsource = { editable = 'dep' }",
        ),
    ] {
        let (spec, snapshot) = fixture("uv", &lock);
        assert!(validate(&[spec], &snapshot).is_err());
    }
}

#[test]
fn uv_unknown_schema_and_revision_fail_closed() {
    for lock in [
        UV.replacen("version = 1", "version = 2", 1),
        UV.replace("revision = 3", "revision = 99"),
    ] {
        let (spec, snapshot) = fixture("uv", &lock);
        assert!(
            validate(&[spec], &snapshot)
                .unwrap_err()
                .contains("format/revision")
        );
    }
}

#[test]
fn uv_stale_dependency_source_is_not_hidden_by_current_package_entries() {
    let lock = UV.replace(
        "source = {editable = 'dep'}",
        "source = {editable = 'removed'}",
    );
    let (spec, snapshot) = fixture("uv", &lock);
    assert!(
        validate(&[spec], &snapshot)
            .unwrap_err()
            .contains("stale/undeclared")
    );
}

#[test]
fn uv_missing_current_edge_or_dangling_old_name_is_rejected() {
    for lock in [
        UV.replace(
            "dependencies = [{name = 'dep', version = '1.0.0', source = {editable = 'dep'}}]",
            "dependencies = []",
        ),
        UV.replace(
            "dependencies = [{name = 'dep', version = '1.0.0', source = {editable = 'dep'}}]",
            "dependencies = [{name = 'removed'}]",
        ),
    ] {
        let (spec, snapshot) = fixture("uv", &lock);
        assert!(validate(&[spec], &snapshot).is_err());
    }
}

#[test]
fn poetry_hash_matches_official_python_json_algorithm() {
    let (spec, snapshot) = fixture("poetry", POETRY);
    assert_eq!(
        content_hash(&spec, &snapshot).unwrap(),
        "5170cc942c42e940c5c95387895254a71392121345e6cf375c7c4ad8d60999cb"
    );
    validate(&[spec.clone()], &snapshot).unwrap();
    assert_eq!(additional_inputs(&spec).unwrap(), ["pyproject.toml"]);
}

#[test]
fn unrelated_toml_settings_do_not_enter_resolution_hash() {
    let (spec, mut snapshot) = fixture("poetry", POETRY);
    let mut bytes = snapshot.inputs["pyproject.toml"].clone().unwrap();
    bytes.extend_from_slice(b"[tool.unrelated]\nratio=0.5\ntime=1979-05-27T07:32:00Z\n");
    snapshot.inputs.insert("pyproject.toml".into(), Some(bytes));
    validate(&[spec], &snapshot).unwrap();
}

#[test]
fn poetry_updates_local_versions_and_hash_from_final_manifests() {
    let (spec, mut snapshot) = fixture("poetry", POETRY);
    validate(&[spec.clone()], &snapshot).unwrap();
    let original_hash = content_hash(&spec, &snapshot).unwrap();
    snapshot.inputs.insert("pyproject.toml".into(), Some(b"[project]\nname='app'\nversion='1.0.1'\ndependencies=['dep==1.1.0']\nrequires-python='>=3.10'\n".to_vec()));
    let expected_hash = content_hash(&spec, &snapshot).unwrap();
    assert_ne!(original_hash, expected_hash);
    let writes = render(&[spec], &snapshot, &versions()).unwrap();
    let after = std::str::from_utf8(&writes["poetry.lock"])
        .unwrap()
        .parse::<DocumentMut>()
        .unwrap();
    assert_eq!(
        after["metadata"]["content-hash"].as_str(),
        Some(expected_hash.as_str())
    );
    assert_eq!(
        after["package"]
            .as_array_of_tables()
            .unwrap()
            .get(0)
            .unwrap()["version"]
            .as_str(),
        Some("1.1.0")
    );
}

#[test]
fn poetry_and_pdm_stale_hashes_are_rejected_before_render() {
    for (format, lock) in [
        ("poetry", POETRY.replace("5170cc94", "00000000")),
        ("pdm", PDM.replace("59edfb9d", "00000000")),
    ] {
        let (spec, snapshot) = fixture(format, &lock);
        assert!(validate(&[spec], &snapshot).unwrap_err().contains("stale"));
    }
}

#[test]
fn pdm_legacy_hash_and_local_directory_updates() {
    let (spec, snapshot) = fixture("pdm", PDM);
    assert_eq!(
        content_hash(&spec, &snapshot).unwrap(),
        "59edfb9de99116f3a84847045c1f0069e8a5b960e54f707d91a4e53d6f2c245a"
    );
    validate(&[spec.clone()], &snapshot).unwrap();
    let writes = render(&[spec], &snapshot, &versions()).unwrap();
    let after = std::str::from_utf8(&writes["pdm.lock"])
        .unwrap()
        .parse::<DocumentMut>()
        .unwrap();
    assert_eq!(
        after["package"]
            .as_array_of_tables()
            .unwrap()
            .get(0)
            .unwrap()["version"]
            .as_str(),
        Some("1.1.0")
    );
    assert_eq!(
        after["metadata"]["content_hash"].as_str(),
        Some("sha256:59edfb9de99116f3a84847045c1f0069e8a5b960e54f707d91a4e53d6f2c245a")
    );
}

#[test]
fn pdm_canonical_and_workspace_hashes_are_not_forged() {
    let (spec, mut snapshot) = fixture("pdm", PDM);
    snapshot.inputs.insert(
        "pyproject.toml".into(),
        Some(
            b"[project]\nname='app'\nversion='1.0.0'\n[tool.pdm.workspace]\nmembers=['dep']\n"
                .to_vec(),
        ),
    );
    assert!(
        validate(&[spec.clone()], &snapshot)
            .unwrap_err()
            .contains("workspace/canonical")
    );
    let (spec, snapshot) = fixture(
        "pdm",
        &PDM.replace("[metadata]", "[metadata]\nlock_inputs={version=1}"),
    );
    assert!(
        validate(&[spec], &snapshot)
            .unwrap_err()
            .contains("format/revision")
    );
}

#[test]
fn pylock_source_trees_stay_versionless_and_third_party_is_unchanged() {
    let lock = format!(
        "{PYLOCK}[[packages]]\nname='requests'\nversion='2.0.0'\n[[packages.wheels]]\nurl='https://example.invalid/requests.whl'\nhashes={{sha256='original'}}\n"
    );
    let (spec, snapshot) = fixture("pylock", &lock);
    validate(&[spec.clone()], &snapshot).unwrap();
    assert!(render(&[spec], &snapshot, &versions()).unwrap().is_empty());
}

#[test]
fn pylock_source_tree_version_or_artifact_mixture_is_rejected() {
    for lock in [
        PYLOCK.replace("name='dep'", "name='dep'\nversion='1.0.0'"),
        PYLOCK.replace(
            "[packages.directory]\npath='dep'",
            "archive={path='dep.whl',hashes={sha256='old'}}\n[packages.directory]\npath='dep'",
        ),
    ] {
        let (spec, snapshot) = fixture("pylock", &lock);
        assert!(validate(&[spec], &snapshot).is_err());
    }
}

#[test]
fn requirements_export_editable_paths_are_stable_during_version_release() {
    let (spec, snapshot) = fixture(
        "requirements-export",
        "# exported\n-e .\n-e dep\nrequests==2.0.0 --hash=sha256:original\n",
    );
    validate(&[spec.clone()], &snapshot).unwrap();
    assert!(render(&[spec], &snapshot, &versions()).unwrap().is_empty());
}

#[test]
fn requirements_pin_is_not_relabelled_without_new_artifact_hashes() {
    let (spec, snapshot) = fixture(
        "requirements-export",
        "-e .\ndep==1.0.0 --hash=sha256:old\n",
    );
    assert!(
        validate(&[spec], &snapshot)
            .unwrap_err()
            .contains("immutable exported pin")
    );
}

#[test]
fn requirements_include_and_missing_lock_inputs_fail_closed() {
    let (spec, mut snapshot) = fixture("requirements-export", "-r unguarded.txt\n");
    assert!(
        validate(&[spec.clone()], &snapshot)
            .unwrap_err()
            .contains("directive")
    );
    snapshot.inputs.insert(spec.path.clone(), None);
    assert!(
        validate(&[spec], &snapshot)
            .unwrap_err()
            .contains("missing")
    );
}

#[test]
fn python_json_sorted_ascii_and_surrogate_pair_encoding() {
    let value = serde_json::json!({"z":"\u{e9}\u{1f600}", "a":true});
    assert_eq!(
        python_json(&value).unwrap(),
        "{\"a\": true, \"z\": \"\\u00e9\\ud83d\\ude00\"}"
    );
}

#[test]
fn lock_paths_are_guarded_and_provider_names_are_explicit() {
    let (mut spec, snapshot) = fixture("uv", UV);
    spec.path = "../uv.lock".into();
    assert!(validate(&[spec], &snapshot).is_err());
    let (mut spec, snapshot) = fixture("uv", UV);
    spec.format = "requirements".into();
    assert!(
        validate(&[spec], &snapshot)
            .unwrap_err()
            .contains("unsupported")
    );
}

#[test]
fn lock_members_cannot_silently_drop_missing_or_foreign_packages() {
    let (mut spec, snapshot) = fixture("uv", UV);
    spec.members.push("retired/pyproject.toml".into());
    assert!(
        validate(&[spec], &snapshot)
            .unwrap_err()
            .contains("not a current package")
    );
}

#[test]
fn uv_retired_all_packages_accepts_empty_current_lock() {
    let mut snapshot = LanguageSnapshot::default();
    snapshot.inputs.insert(
        "uv.lock".into(),
        Some(b"version=1\nrevision=3\npackage=[]\n".to_vec()),
    );
    let spec = LockSpec {
        path: "uv.lock".into(),
        format: "uv".into(),
        members: vec![],
        metadata: None,
        tool_version: None,
    };
    validate(&[spec], &snapshot).unwrap();
}

fn with_leaf(format: &str, lock: &str) -> (LockSpec, LanguageSnapshot) {
    let (mut spec, mut snapshot) = fixture(format, lock);
    spec.members.push("leaf/pyproject.toml".into());
    snapshot.packages.insert(
        "leaf/pyproject.toml".into(),
        NativePackage {
            manifest: "leaf/pyproject.toml".into(),
            name: "leaf".into(),
            version: "1.0.0".into(),
            language: Language::Python,
            workspace: "pyproject.toml".into(),
            source_roots: vec![],
        },
    );
    snapshot.dependencies.push(NativeDependency {
        owner: "dep/pyproject.toml".into(),
        target: "leaf/pyproject.toml".into(),
        kind: "runtime".into(),
    });
    snapshot.inputs.insert(
        "leaf/pyproject.toml".into(),
        Some(b"[project]\nname='leaf'\nversion='1.0.0'\n".to_vec()),
    );
    snapshot.inputs.insert(
        "dep/pyproject.toml".into(),
        Some(b"[project]\nname='dep'\nversion='1.0.0'\ndependencies=['leaf==1.0.0']\n".to_vec()),
    );
    (spec, snapshot)
}

#[test]
fn every_provider_requires_declared_members_to_be_a_complete_local_closure() {
    for (format, lock) in [
        ("uv", UV),
        ("poetry", POETRY),
        ("pdm", PDM),
        ("pylock", PYLOCK),
        ("requirements-export", "-e .\n-e dep\n"),
    ] {
        let (mut spec, snapshot) = fixture(format, lock);
        spec.members
            .retain(|manifest| manifest != "dep/pyproject.toml");
        assert!(
            validate(&[spec], &snapshot)
                .unwrap_err()
                .contains("omit required local dependency")
        );
    }
}

#[test]
fn local_subproject_new_edge_is_not_hidden_by_unchanged_root_hash() {
    for (format, lock) in [
        (
            "poetry",
            format!(
                "{POETRY}[[package]]\nname='leaf'\nversion='1.0.0'\nfiles=[]\n[package.source]\ntype='directory'\nurl='leaf'\n"
            ),
        ),
        (
            "pdm",
            format!("{PDM}[[package]]\nname='leaf'\nversion='1.0.0'\npath='leaf'\n"),
        ),
        (
            "pylock",
            format!("{PYLOCK}[[packages]]\nname='leaf'\n[packages.directory]\npath='leaf'\n"),
        ),
    ] {
        let (spec, snapshot) = with_leaf(format, &lock);
        assert!(
            validate(&[spec.clone()], &snapshot)
                .unwrap_err()
                .contains("missing current local dependency")
        );
        assert!(
            render(&[spec], &snapshot, &BTreeMap::new())
                .unwrap_err()
                .contains("missing current local dependency")
        );
    }
}

#[test]
fn valid_local_subproject_edges_validate_without_evaluating_platform_markers() {
    let cases = [
        ("poetry",format!("{}[[package]]\nname='leaf'\nversion='1.0.0'\nfiles=[]\n[package.source]\ntype='directory'\nurl='leaf'\n",POETRY.replace("[package.source]","[package.dependencies]\nleaf={version='==1.0.0',markers=\"sys_platform == 'win32'\"}\n[package.source]"))),
        ("pdm",format!("{}[[package]]\nname='leaf'\nversion='1.0.0'\npath='leaf'\n",PDM.replace("editable=true","editable=true\ndependencies=['leaf==1.0.0; sys_platform == \"win32\"']"))),
        ("pylock",format!("{}[[packages]]\nname='leaf'\n[packages.directory]\npath='leaf'\n",PYLOCK.replace("name='dep'\n[packages.directory]","name='dep'\ndependencies=[{name='leaf'}]\n[packages.directory]"))),
    ];
    for (format, lock) in cases {
        let (spec, mut snapshot) = with_leaf(format, &lock);
        if format != "pylock" {
            snapshot.inputs.insert("dep/pyproject.toml".into(), Some(b"[project]\nname='dep'\nversion='1.0.0'\ndependencies=['leaf==1.0.0; sys_platform == \"win32\"']\n".to_vec()));
        }
        validate(&[spec], &snapshot).unwrap();
    }
}

#[test]
fn child_marker_or_extras_drift_is_not_hidden_by_name_only_graph() {
    for (format,lock) in [
        ("poetry",format!("{}[[package]]\nname='leaf'\nversion='1.0.0'\nfiles=[]\n[package.source]\ntype='directory'\nurl='leaf'\n",POETRY.replace("[package.source]","[package.dependencies]\nleaf={version='==1.0.0',markers=\"sys_platform == 'win32'\"}\n[package.source]"))),
        ("pdm",format!("{}[[package]]\nname='leaf'\nversion='1.0.0'\npath='leaf'\n",PDM.replace("editable=true","editable=true\ndependencies=['leaf[missing]==1.0.0']"))),
    ] {
        let (spec,snapshot)=with_leaf(format,&lock);
        assert!(validate(&[spec],&snapshot).unwrap_err().contains("marker/extras differ"));
    }
}

#[test]
fn removed_child_edge_is_rejected_even_when_another_package_keeps_target_alive() {
    let cases = [
        (
            "uv",
            format!(
                "{}[[package]]\nname='leaf'\nversion='1.0.0'\nsource={{editable='leaf'}}\n[[package]]\nname='other'\nversion='1.0.0'\nsource={{editable='other'}}\ndependencies=[{{name='leaf'}}]\n",
                UV.replace(
                    "source = { editable = 'dep' }\n",
                    "source = { editable = 'dep' }\ndependencies=[{name='leaf'}]\n"
                )
            ),
            "dependencies=[{name='leaf'}]\n",
        ),
        (
            "poetry",
            format!(
                "{}[[package]]\nname='leaf'\nversion='1.0.0'\nfiles=[]\n[package.source]\ntype='directory'\nurl='leaf'\n[[package]]\nname='other'\nversion='1.0.0'\nfiles=[]\n[package.dependencies]\nleaf='==1.0.0'\n[package.source]\ntype='directory'\nurl='other'\n",
                POETRY.replace(
                    "[package.source]",
                    "[package.dependencies]\nleaf='==1.0.0'\n[package.source]"
                )
            ),
            "[package.dependencies]\nleaf='==1.0.0'\n",
        ),
        (
            "pdm",
            format!(
                "{}[[package]]\nname='leaf'\nversion='1.0.0'\npath='leaf'\n[[package]]\nname='other'\nversion='1.0.0'\npath='other'\ndependencies=['leaf==1.0.0']\n",
                PDM.replace(
                    "editable=true",
                    "editable=true\ndependencies=['leaf==1.0.0']"
                )
            ),
            "dependencies=['leaf==1.0.0']\n",
        ),
        (
            "pylock",
            format!(
                "{}[[packages]]\nname='leaf'\n[packages.directory]\npath='leaf'\n[[packages]]\nname='other'\ndependencies=[{{name='leaf'}}]\n[packages.directory]\npath='other'\n",
                PYLOCK.replace(
                    "name='dep'\n[packages.directory]",
                    "name='dep'\ndependencies=[{name='leaf'}]\n[packages.directory]"
                )
            ),
            "dependencies=[{name='leaf'}]\n",
        ),
    ];
    for (format, lock, removed) in cases {
        let (mut spec, mut snapshot) = with_leaf(format, &lock);
        snapshot
            .dependencies
            .retain(|dep| dep.owner != "dep/pyproject.toml");
        snapshot.inputs.insert(
            "dep/pyproject.toml".into(),
            Some(b"[project]\nname='dep'\nversion='1.0.0'\n".to_vec()),
        );
        spec.members.push("other/pyproject.toml".into());
        snapshot.packages.insert(
            "other/pyproject.toml".into(),
            NativePackage {
                manifest: "other/pyproject.toml".into(),
                name: "other".into(),
                version: "1.0.0".into(),
                language: Language::Python,
                workspace: "pyproject.toml".into(),
                source_roots: vec![],
            },
        );
        snapshot.dependencies.push(NativeDependency {
            owner: "other/pyproject.toml".into(),
            target: "leaf/pyproject.toml".into(),
            kind: "runtime".into(),
        });
        snapshot.inputs.insert(
            "other/pyproject.toml".into(),
            Some(
                b"[project]\nname='other'\nversion='1.0.0'\ndependencies=['leaf==1.0.0']\n"
                    .to_vec(),
            ),
        );
        assert!(
            validate(&[spec.clone()], &snapshot)
                .unwrap_err()
                .contains("stale"),
            "{format}"
        );
        assert!(render(&[spec.clone()], &snapshot, &BTreeMap::new()).is_err());
        snapshot.inputs.insert(
            spec.path.clone(),
            Some(lock.replacen(removed, "", 1).into_bytes()),
        );
        validate(&[spec], &snapshot).unwrap();
    }
}

#[test]
fn uv_removed_dependency_kind_or_changed_marker_is_rejected() {
    for lock in [
        UV.replace(
            "dependencies = [{name = 'dep', version = '1.0.0', source = {editable = 'dep'}}]",
            "[package.optional-dependencies]\nold=[{name='dep'}]",
        ),
        UV.replace(
            "dependencies = [{name = 'dep', version = '1.0.0', source = {editable = 'dep'}}]",
            "dependencies=[{name='dep', marker=\"sys_platform == 'win32'\"}]",
        ),
    ] {
        let (spec, snapshot) = fixture("uv", &lock);
        assert!(
            validate(&[spec], &snapshot)
                .unwrap_err()
                .contains("kinds/markers/extras")
        );
    }
}

fn uv_empty_extra_fixture() -> (LockSpec, LanguageSnapshot) {
    let lock=UV
        .replace("dependencies = [{name = 'dep', version = '1.0.0', source = {editable = 'dep'}}]","dependencies=[{name='dep',marker=\"sys_platform == 'win32'\"}]")
        .replace("requires-dist = [{name = 'dep', specifier = '==1.0.0', editable = 'dep'}]","requires-dist=[{name='dep',specifier='==1.0.0',extras=['fast'],marker=\"sys_platform == 'win32'\",virtual='dep'}]")
        .replace("source = { editable = 'dep' }","source={virtual='dep'}\n[package.metadata]\nprovides-extras=['fast']");
    let (spec, mut snapshot) = fixture("uv", &lock);
    snapshot.inputs.insert("pyproject.toml".into(),Some(b"[project]\nname='app'\nversion='1.0.0'\ndependencies=[\"dep[fast]==1.0.0 ; sys_platform == 'win32'\"]\nrequires-python='>=3.10'\n".to_vec()));
    snapshot.inputs.insert(
        "dep/pyproject.toml".into(),
        Some(
            b"[project]\nname='dep'\nversion='1.0.0'\n[project.optional-dependencies]\nfast=[]\n"
                .to_vec(),
        ),
    );
    (spec, snapshot)
}

#[test]
fn uv_proven_empty_extra_may_be_simplified_only_in_resolved_graph() {
    let (spec, snapshot) = uv_empty_extra_fixture();
    validate(&[spec.clone()], &snapshot).unwrap();
    let writes = render(&[spec], &snapshot, &versions()).unwrap();
    let after = std::str::from_utf8(&writes["uv.lock"])
        .unwrap()
        .parse::<DocumentMut>()
        .unwrap();
    let root = after["package"]
        .as_array_of_tables()
        .unwrap()
        .get(0)
        .unwrap();
    let edge = root["dependencies"]
        .as_array()
        .unwrap()
        .get(0)
        .unwrap()
        .as_inline_table()
        .unwrap();
    assert!(edge.get("extra").is_none());
    assert_eq!(
        edge.get("marker").and_then(Value::as_str),
        Some("sys_platform == 'win32'")
    );
    assert_eq!(
        root["metadata"]["requires-dist"]
            .as_array()
            .unwrap()
            .get(0)
            .unwrap()
            .as_inline_table()
            .unwrap()
            .get("extras")
            .and_then(Value::as_array)
            .unwrap()
            .get(0)
            .and_then(Value::as_str),
        Some("fast")
    );
}

#[test]
fn uv_empty_extra_simplification_requires_all_three_independent_proofs() {
    let (spec, snapshot) = uv_empty_extra_fixture();
    let original = std::str::from_utf8(snapshot.inputs["uv.lock"].as_deref().unwrap()).unwrap();
    for lock in [
        original.replace("extras=['fast']", "extras=[]"),
        original.replace("provides-extras=['fast']", "provides-extras=[]"),
        original.replace("virtual='dep'}]\n", "editable='dep'}]\n"),
        original.replacen(
            "marker=\"sys_platform == 'win32'\"",
            "marker=\"sys_platform == 'linux'\"",
            1,
        ),
    ] {
        let mut invalid = snapshot.clone();
        invalid
            .inputs
            .insert("uv.lock".into(), Some(lock.into_bytes()));
        assert!(
            validate(&[spec.clone()], &invalid)
                .unwrap_err()
                .contains("kinds/markers/extras")
        );
    }
    let mut invalid = snapshot.clone();
    invalid.inputs.insert("dep/pyproject.toml".into(),Some(b"[project]\nname='dep'\nversion='1.0.0'\n[project.optional-dependencies]\nfast=['requests']\n".to_vec()));
    assert!(
        validate(&[spec], &invalid)
            .unwrap_err()
            .contains("kinds/markers/extras")
    );
}

#[test]
fn poetry_and_pdm_stale_child_constraints_fail_with_no_version_bumps() {
    for (format, lock) in [
        (
            "poetry",
            format!(
                "{}[[package]]\nname='leaf'\nversion='1.0.0'\nfiles=[]\n[package.source]\ntype='directory'\nurl='leaf'\n",
                POETRY.replace(
                    "[package.source]",
                    "[package.dependencies]\nleaf='==0.9.0'\n[package.source]"
                )
            ),
        ),
        (
            "pdm",
            format!(
                "{}[[package]]\nname='leaf'\nversion='1.0.0'\npath='leaf'\n",
                PDM.replace(
                    "editable=true",
                    "editable=true\ndependencies=['leaf==0.9.0']"
                )
            ),
        ),
    ] {
        let (spec, snapshot) = with_leaf(format, &lock);
        assert!(
            validate(&[spec.clone()], &snapshot)
                .unwrap_err()
                .to_lowercase()
                .contains("stale")
        );
        assert!(render(&[spec], &snapshot, &BTreeMap::new()).is_err());
    }
}
