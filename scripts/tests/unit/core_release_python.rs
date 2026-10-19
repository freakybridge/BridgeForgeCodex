use super::*;
use serde_json::json;

fn config(projects: &[&str]) -> Config {
    serde_json::from_value(json!({"projects":projects})).unwrap()
}

fn files(entries: &[(&str, &str)]) -> BTreeMap<String, Vec<u8>> {
    entries
        .iter()
        .map(|(path, bytes)| (path.to_string(), bytes.as_bytes().to_vec()))
        .collect()
}

fn load(cfg: &Config, files: &BTreeMap<String, Vec<u8>>) -> Result<LanguageSnapshot, String> {
    snapshot(
        cfg,
        &files.keys().cloned().collect::<Vec<_>>(),
        &mut |path| Ok(files.get(path).cloned()),
    )
}

fn render(
    cfg: &Config,
    files: &BTreeMap<String, Vec<u8>>,
    versions: &[(&str, &str)],
) -> Result<Writes, String> {
    render_versions(
        cfg,
        &files.keys().cloned().collect::<Vec<_>>(),
        &mut |path| Ok(files.get(path).cloned()),
        &versions
            .iter()
            .map(|(path, version)| (path.to_string(), version.to_string()))
            .collect(),
    )
}

#[test]
fn python_static_distributions_use_pep_name_identity_not_import_names() {
    let mut cfg = config(&["a/pyproject.toml", "b/pyproject.toml"]);
    cfg.import_roots
        .insert("a/pyproject.toml".into(), vec!["shared/import_a".into()]);
    let files = files(&[
        (
            "a/pyproject.toml",
            "[project]\nname='A_Package'\nversion='1.2.3'\ndependencies=['B.Package>=1.0']\n",
        ),
        (
            "b/pyproject.toml",
            "[project]\nname='b-package'\nversion='1.0.0'\n",
        ),
    ]);
    let snapshot = load(&cfg, &files).unwrap();
    assert_eq!(snapshot.packages["a/pyproject.toml"].name, "a-package");
    assert_eq!(
        snapshot.packages["a/pyproject.toml"].source_roots,
        ["a", "shared/import_a"]
    );
    assert!(
        snapshot.dependencies.is_empty(),
        "Equal registry names must not infer local dependency identity"
    );
}

#[test]
fn python_explicit_local_sources_update_versions_preserving_markers_and_extras() {
    let mut cfg = config(&["a/pyproject.toml", "b/pyproject.toml"]);
    cfg.local_sources.insert(
        "a/pyproject.toml".into(),
        BTreeMap::from([("B_Package".into(), "b/pyproject.toml".into())]),
    );
    let files = files(&[
        (
            "a/pyproject.toml",
            "[project]\nname='a'\nversion='1.2.3'\ndependencies=['b-package[fast]==2.0.0 ; python_version < \"3.13\"']\n",
        ),
        (
            "b/pyproject.toml",
            "[project]\nname='b-package'\nversion='2.0.0'\n",
        ),
    ]);
    let snapshot = load(&cfg, &files).unwrap();
    assert_eq!(snapshot.dependencies.len(), 1);
    let writes = render(
        &cfg,
        &files,
        &[("a/pyproject.toml", "1.2.4"), ("b/pyproject.toml", "2.1.0")],
    )
    .unwrap();
    let rewritten = metadata::parse(&writes["a/pyproject.toml"], "a/pyproject.toml").unwrap();
    let expected = pep508_rs::Requirement::<pep508_rs::VerbatimUrl>::from_str(
        "b-package[fast]==2.1.0 ; python_version < '3.13'",
    )
    .unwrap();
    let actual = pep508_rs::Requirement::<pep508_rs::VerbatimUrl>::from_str(
        &rewritten.dependencies[0].requirement,
    )
    .unwrap();
    assert_eq!(actual, expected);
    assert_eq!(rewritten.version.as_deref(), Some("1.2.4"));
}

#[test]
fn python_groups_optional_and_build_dependencies_are_all_retained() {
    let mut cfg = config(&["a/pyproject.toml", "b/pyproject.toml"]);
    cfg.local_sources.insert(
        "a/pyproject.toml".into(),
        BTreeMap::from([("b".into(), "b/pyproject.toml".into())]),
    );
    let files = files(&[
        (
            "a/pyproject.toml",
            "[project]\nname='a'\nversion='1.0.0'\n[project.optional-dependencies]\nfast=['b==1.0.0; sys_platform == \"win32\"']\n[dependency-groups]\ntest=['b>=1.0', {include-group='lint'}]\nlint=['b==1.0.0']\n[build-system]\nrequires=['b==1.0.0']\n",
        ),
        ("b/pyproject.toml", "[project]\nname='b'\nversion='1.0.0'\n"),
    ]);
    let snapshot = load(&cfg, &files).unwrap();
    assert_eq!(snapshot.dependencies.len(), 4);
    let writes = render(&cfg, &files, &[("b/pyproject.toml", "1.1.0")]).unwrap();
    let parsed = metadata::parse(&writes["a/pyproject.toml"], "a/pyproject.toml").unwrap();
    assert_eq!(parsed.dependencies.len(), 4);
    assert!(
        parsed
            .dependencies
            .iter()
            .any(|d| d.kind == "extra:fast" && d.requirement.contains("win32"))
    );
    assert!(
        parsed
            .dependencies
            .iter()
            .any(|d| d.requirement == "b>=1.0")
    );
}

#[test]
fn python_uv_workspace_source_resolves_explicit_identity() {
    let cfg = config(&["a/pyproject.toml", "b/pyproject.toml"]);
    let files = files(&[
        (
            "a/pyproject.toml",
            "[project]\nname='a'\nversion='1.0.0'\ndependencies=['b==1.0.0']\n[tool.uv.sources]\nb={workspace=true}\n",
        ),
        ("b/pyproject.toml", "[project]\nname='b'\nversion='1.0.0'\n"),
    ]);
    let snapshot = load(&cfg, &files).unwrap();
    assert_eq!(snapshot.dependencies[0].target, "b/pyproject.toml");
}

#[test]
fn python_poetry_path_dependencies_keep_source_and_update_constraint() {
    let mut cfg = config(&["a/pyproject.toml", "b/pyproject.toml"]);
    cfg.local_sources.insert(
        "a/pyproject.toml".into(),
        BTreeMap::from([("b".into(), "b/pyproject.toml".into())]),
    );
    let files = files(&[
        (
            "a/pyproject.toml",
            "[project]\nname='a'\nversion='1.0.0'\ndependencies=['b==1.0.0']\n[tool.poetry.dependencies]\npython='^3.11'\nb={path='../b', develop=true}\n",
        ),
        ("b/pyproject.toml", "[project]\nname='b'\nversion='1.0.0'\n"),
    ]);
    let writes = render(
        &cfg,
        &files,
        &[("a/pyproject.toml", "1.0.1"), ("b/pyproject.toml", "2.0.0")],
    )
    .unwrap();
    let doc = metadata::document(&writes["a/pyproject.toml"], "a/pyproject.toml").unwrap();
    let dep = &doc["tool"]["poetry"]["dependencies"]["b"];
    assert_eq!(dep["path"].as_str(), Some("../b"));
    assert!(dep.get("version").is_none());
    assert_eq!(
        doc["project"]["dependencies"]
            .as_array()
            .unwrap()
            .get(0)
            .unwrap()
            .as_str(),
        Some("b==2.0.0")
    );
    assert_eq!(dep["develop"].as_bool(), Some(true));
    let invalid=b"[tool.poetry]\nname='a'\nversion='1.0.0'\n[tool.poetry.dependencies]\nb={path='../b',version='1.0.0'}\n";
    assert!(
        metadata::parse(invalid, "a/pyproject.toml")
            .unwrap_err()
            .contains("cannot contain version")
    );
}

#[test]
fn python_setup_cfg_static_metadata_and_multiline_requirements() {
    let mut cfg = config(&["a/setup.cfg", "b/pyproject.toml"]);
    cfg.local_sources.insert(
        "a/setup.cfg".into(),
        BTreeMap::from([("b".into(), "b/pyproject.toml".into())]),
    );
    let files = files(&[
        (
            "a/setup.cfg",
            "[metadata]\nname = A\nversion = 1.0.0\n[options]\ninstall_requires =\n    b==2.0.0\n    requests>=2\n[options.extras_require]\ntest =\n    b==2.0.0 ; python_version > '3.9'\n",
        ),
        ("b/pyproject.toml", "[project]\nname='b'\nversion='2.0.0'\n"),
    ]);
    let writes = render(
        &cfg,
        &files,
        &[("a/setup.cfg", "1.0.1"), ("b/pyproject.toml", "2.1.0")],
    )
    .unwrap();
    let info = metadata::parse(&writes["a/setup.cfg"], "a/setup.cfg").unwrap();
    assert_eq!(info.version.as_deref(), Some("1.0.1"));
    assert!(
        info.dependencies
            .iter()
            .filter(|d| d.name == "b")
            .all(|d| d.requirement.contains("2.1.0"))
    );
    assert!(info.dependencies.iter().any(|d| d.name == "requests"));
    let mut shimmed = files.clone();
    shimmed.insert(
        "a/setup.py".into(),
        b"from setuptools import setup\nsetup()\n".to_vec(),
    );
    load(&cfg, &shimmed).unwrap();
    shimmed.insert(
        "a/setup.py".into(),
        b"from setuptools import setup\nsetup(version=calculate_version())\n".to_vec(),
    );
    assert!(
        load(&cfg, &shimmed)
            .unwrap_err()
            .contains("Dynamic setup.py")
    );
}

#[test]
fn python_dynamic_metadata_is_rejected_without_executing_project_code() {
    let cfg = config(&["pyproject.toml"]);
    for raw in [
        "[project]\nname='a'\ndynamic=['version']\n",
        "[project]\nname='a'\nversion='1.0.0'\ndynamic=['dependencies']\n",
        "[tool.setuptools_scm]\n",
    ] {
        let error = load(
            &cfg,
            &files(&[
                ("pyproject.toml", raw),
                ("setup.py", "raise RuntimeError('must never run')"),
            ]),
        )
        .unwrap_err();
        assert!(error.contains("dynamic") || error.contains("Dynamic") || error.contains("SCM"));
    }
}

#[test]
fn python_safe_external_version_source_has_one_fact_source() {
    let mut cfg = config(&["pyproject.toml"]);
    cfg.version_sources.insert(
        "pyproject.toml".into(),
        VersionSource {
            path: "src/version.py".into(),
            kind: "python".into(),
            key: Some("__version__".into()),
        },
    );
    let files = files(&[
        (
            "pyproject.toml",
            "[project]\nname='a'\ndynamic=['version']\n",
        ),
        (
            "src/version.py",
            "# never import this module\n__version__ = '1.2.3' # stable\n",
        ),
    ]);
    let snapshot = load(&cfg, &files).unwrap();
    assert_eq!(snapshot.packages["pyproject.toml"].version, "1.2.3");
    let writes = render(&cfg, &files, &[("pyproject.toml", "1.3.0")]).unwrap();
    assert!(!writes.contains_key("pyproject.toml"));
    let output = std::str::from_utf8(&writes["src/version.py"]).unwrap();
    assert!(output.contains("__version__ = '1.3.0' # stable"));
    let mut executable = files.clone();
    executable
        .get_mut("src/version.py")
        .unwrap()
        .extend_from_slice(b"raise RuntimeError('must never run')\n");
    assert!(
        load(&cfg, &executable)
            .unwrap_err()
            .contains("dedicated static file")
    );
    let mut scm = files.clone();
    scm.get_mut("pyproject.toml")
        .unwrap()
        .extend_from_slice(b"[tool.setuptools_scm]\nversion_file='src/version.py'\n");
    assert!(
        load(&cfg, &scm).unwrap_err().contains("SCM/tag-derived"),
        "Generated SCM version files cannot become a static primary source"
    );
}

#[test]
fn python_complex_versions_require_explicit_target_and_pep_ordering() {
    for version in ["1.0.0rc1", "1!2.0.0", "1.2.3.post1", "1.2.3+local", "1.2"] {
        validate_version(version).unwrap();
        assert!(bump_version(version, 1).is_err());
    }
    validate_increase("1.0.0rc1", "1.0.0").unwrap();
    assert!(validate_increase("1.0.0", "1.0.0rc2").is_err());
    assert_eq!(bump_version("1.2.3", 1).unwrap(), "1.2.4");
    assert_eq!(bump_version("1.2.3", 2).unwrap(), "1.3.0");
    assert_eq!(bump_version("1.2.3", 3).unwrap(), "2.0.0");
    assert!(validate_version("banana").is_err());
}

#[test]
fn python_constraint_updates_never_silently_widen_composite_bounds() {
    assert_eq!(
        deps::update_constraint(">=1.0,<2", "1.0.0", "1.1.0").unwrap(),
        ">=1.0,<2"
    );
    assert!(deps::update_constraint(">=1.0,<2", "1.0.0", "2.0.0").is_err());
    assert!(deps::update_constraint("==1.*", "1.0.0", "2.0.0").is_err());
    assert_eq!(
        deps::update_constraint("==1.0.0", "1.0.0", "2.0.0").unwrap(),
        "==2.0.0"
    );
    assert!(deps::parse("a>=1; unknown_marker == 'yes'").is_err());
    assert!(deps::parse("a>=1; and and and").is_err());
}

#[test]
fn python_invalid_sources_duplicate_names_and_group_cycles_fail_closed() {
    let cfg = config(&["pyproject.toml"]);
    let bad = "[project]\nname='a'\nversion='1.0.0'\n[dependency-groups]\na=[{include-group='b'}]\nb=[{include-group='a'}]\n";
    assert!(
        load(&cfg, &files(&[("pyproject.toml", bad)]))
            .unwrap_err()
            .contains("Cyclic")
    );
    assert!(load(&config(&["../outside/pyproject.toml"]), &BTreeMap::new()).is_err());
    assert!(load(&config(&["pyproject.toml", "setup.cfg"]), &BTreeMap::new()).is_err());
}

#[test]
fn python_external_poetry_sources_cannot_masquerade_as_local_packages() {
    let mut cfg = config(&["a/pyproject.toml", "b/pyproject.toml"]);
    cfg.local_sources.insert(
        "a/pyproject.toml".into(),
        BTreeMap::from([("b".into(), "b/pyproject.toml".into())]),
    );
    let files = files(&[
        (
            "a/pyproject.toml",
            "[tool.poetry]\nname='a'\nversion='1.0.0'\n[tool.poetry.dependencies]\nb={git='https://example.invalid/b', rev='abcdef'}\n",
        ),
        ("b/pyproject.toml", "[project]\nname='b'\nversion='1.0.0'\n"),
    ]);
    assert!(load(&cfg, &files).unwrap_err().contains("External Poetry"));
}

#[test]
fn python_explicit_complex_target_is_written_without_dropping_pep440_parts() {
    let cfg = config(&["pyproject.toml"]);
    let files = files(&[(
        "pyproject.toml",
        "[project]\nname='a'\nversion='1.0.0rc1'\n",
    )]);
    let writes = render(&cfg, &files, &[("pyproject.toml", "1!2.0.0rc2")]).unwrap();
    assert_eq!(
        metadata::parse(&writes["pyproject.toml"], "pyproject.toml")
            .unwrap()
            .version
            .as_deref(),
        Some("1!2.0.0rc2")
    );
}

#[test]
fn python_toml_external_version_and_dependency_updates_compose_in_one_file() {
    let mut cfg = config(&["a/pyproject.toml", "b/pyproject.toml"]);
    cfg.version_sources.insert(
        "a/pyproject.toml".into(),
        VersionSource {
            path: "a/pyproject.toml".into(),
            kind: "toml".into(),
            key: Some("tool.app.version".into()),
        },
    );
    cfg.local_sources.insert(
        "a/pyproject.toml".into(),
        BTreeMap::from([("b".into(), "b/pyproject.toml".into())]),
    );
    let files = files(&[
        (
            "a/pyproject.toml",
            "[project]\nname='a'\ndynamic=['version']\ndependencies=['b==1.0.0']\n[tool.app]\nversion='2.0.0'\n",
        ),
        ("b/pyproject.toml", "[project]\nname='b'\nversion='1.0.0'\n"),
    ]);
    let writes = render(
        &cfg,
        &files,
        &[("a/pyproject.toml", "2.0.1"), ("b/pyproject.toml", "1.1.0")],
    )
    .unwrap();
    let doc = metadata::document(&writes["a/pyproject.toml"], "a/pyproject.toml").unwrap();
    assert_eq!(doc["tool"]["app"]["version"].as_str(), Some("2.0.1"));
    assert_eq!(
        doc["project"]["dependencies"]
            .as_array()
            .unwrap()
            .get(0)
            .unwrap()
            .as_str(),
        Some("b==1.1.0")
    );
}

#[test]
fn python_pdm_project_root_sources_are_static_and_repository_relative() {
    let cfg = config(&["a/pyproject.toml", "b/pyproject.toml"]);
    for source in [
        "file:///${PROJECT_ROOT}/../b",
        "{root:uri}/../b",
        "file:../b",
    ] {
        let requirement = format!("b @ {source}");
        assert_eq!(deps::parse(&requirement).unwrap().specifier, source);
        let pyproject =
            format!("[project]\nname='a'\nversion='1.0.0'\ndependencies=['{requirement}']\n");
        let files = files(&[
            ("a/pyproject.toml", &pyproject),
            ("b/pyproject.toml", "[project]\nname='b'\nversion='1.0.0'\n"),
        ]);
        let snapshot = load(&cfg, &files).unwrap();
        assert_eq!(snapshot.dependencies[0].target, "b/pyproject.toml");
        let writes = render(&cfg, &files, &[("b/pyproject.toml", "1.1.0")]).unwrap();
        assert!(
            !writes.contains_key("a/pyproject.toml"),
            "Directory URLs have no package version to relabel"
        );
    }
}

#[test]
fn python_shared_version_file_cannot_alias_the_same_slot_across_formats() {
    let mut cfg = config(&["a/pyproject.toml", "b/pyproject.toml"]);
    for (project, kind) in [("a/pyproject.toml", "python"), ("b/pyproject.toml", "toml")] {
        cfg.version_sources.insert(
            project.into(),
            VersionSource {
                path: "common_version.py".into(),
                kind: kind.into(),
                key: Some("VERSION".into()),
            },
        );
    }
    let files = files(&[
        (
            "a/pyproject.toml",
            "[project]\nname='a'\ndynamic=['version']\n",
        ),
        (
            "b/pyproject.toml",
            "[project]\nname='b'\ndynamic=['version']\n",
        ),
        ("common_version.py", "VERSION='1.2.3'\n"),
    ]);
    assert!(
        load(&cfg, &files)
            .unwrap_err()
            .contains("multiple parser kinds")
    );
    cfg.version_sources
        .get_mut("b/pyproject.toml")
        .unwrap()
        .kind = "python".into();
    assert!(
        load(&cfg, &files)
            .unwrap_err()
            .contains("same version fact source")
    );
    cfg.version_sources.get_mut("b/pyproject.toml").unwrap().key = Some("B_VERSION".into());
    let mut separate = files.clone();
    separate
        .get_mut("common_version.py")
        .unwrap()
        .extend_from_slice(b"B_VERSION='3.0.0'\n");
    load(&cfg, &separate).unwrap();
    cfg.version_sources
        .get_mut("b/pyproject.toml")
        .unwrap()
        .path = "COMMON_VERSION.py".into();
    assert!(
        load(&cfg, &separate)
            .unwrap_err()
            .contains("differ only by case")
    );
}
