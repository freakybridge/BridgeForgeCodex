use super::*;

fn source_graph(entry: &str, files: &[(&str, &str)]) -> Result<(BTreeSet<String>, BTreeMap<String, Option<Vec<u8>>>), String> {
    let files = files.iter().map(|(p, s)| (p.to_string(), s.as_bytes().to_vec())).collect::<BTreeMap<_, _>>();
    let inventory = files.keys().cloned().collect();
    let mut inputs = BTreeMap::new();
    let graph = sources::reachable(&[entry.into()], &inventory, &mut inputs, &mut |p| Ok(files.get(p).cloned()))?;
    Ok((graph, inputs))
}

#[test]
fn rust_external_source_graph_tracks_modules_paths_includes_and_package_bridges() {
    let files = [
        ("outside/entry.rs", "pub mod all_inventory; mod inline { #[path=\"other.rs\"] mod nested; } #[path=\"../pkg/bridge.rs\"] mod bridge; include!(\"included/items.rs\"); const TEXT:&str=include_str!(\"data.txt\"); const BYTES:&[u8]=include_bytes!(\"data.bin\"); const VALUE:u8=include!(\"value.rs\"); // mod decoy;\nconst DECOY:&str=\"mod decoy;\";"),
        ("outside/all_inventory.rs", "pub mod child; #[path=\"all_inventory_tests.rs\"] mod tests;"),
        ("outside/all_inventory/child/mod.rs", "pub mod leaf;"),
        ("outside/all_inventory/child/leaf.rs", ""),
        ("outside/all_inventory_tests.rs", ""),
        ("outside/inline/other.rs", ""),
        ("pkg/bridge.rs", "#[path=\"../outside/leaf.rs\"] mod leaf;"),
        ("outside/leaf.rs", ""),
        ("outside/included/items.rs", "mod included_child;"),
        ("outside/included/included_child.rs", ""),
        ("outside/data.txt", "not Rust"),
        ("outside/data.bin", "bytes"),
        ("outside/value.rs", "42"),
        ("outside/decoy.rs", ""),
        ("outside/unrelated.rs", ""),
    ];
    let (graph, inputs) = source_graph("outside/entry.rs", &files).unwrap();
    assert_eq!(graph.len(), files.len() - 2);
    assert!(graph.contains("outside/all_inventory.rs"));
    assert!(graph.contains("pkg/bridge.rs"));
    assert!(!graph.contains("outside/unrelated.rs"));
    assert!(!graph.contains("outside/decoy.rs"));
    assert_eq!(graph, inputs.keys().cloned().collect());
    for (path, bytes) in inputs { assert_eq!(bytes.unwrap(), files.iter().find(|(p, _)| *p == path).unwrap().1.as_bytes()); }
}

#[test]
fn rust_external_source_graph_handles_conditional_paths_and_explicit_directory_semantics() {
    let (graph, _) = source_graph("outside/arbitrary_root.rs", &[
        ("outside/arbitrary_root.rs", "#[cfg_attr(windows,path=\"windows.rs\")] mod platform; #[cfg(unix)] mod missing_platform; #[path=\"special.rs\"] mod renamed;"),
        ("outside/platform.rs", ""),
        ("outside/windows.rs", ""),
        ("outside/special.rs", "mod child; #[path=\"thread_files\"] mod thread { #[path=\"tls.rs\"] mod local_data; }"),
        ("outside/child.rs", ""),
        ("outside/thread_files/tls.rs", ""),
        ("outside/special/child.rs", ""),
    ]).unwrap();
    assert!(graph.contains("outside/platform.rs") && graph.contains("outside/windows.rs"));
    assert!(graph.contains("outside/child.rs") && graph.contains("outside/thread_files/tls.rs"));
    assert!(!graph.contains("outside/special/child.rs"));
}

#[test]
fn rust_external_source_graph_propagates_scoped_conditions_and_standard_includes() {
    let source = r##"
        #[cfg(any())] fn disabled() { include!("missing-function.rs"); }
        #[cfg(any())] const DISABLED: () = include!("missing-constant.rs");
        fn enabled() {
            #[cfg(any())] let disabled = include!("missing-local.rs");
            #[cfg(any())] include!("missing-statement.rs");
        }
        struct Example;
        impl Example { #[cfg(any())] fn disabled() { include!("missing-method.rs"); } }
        #[doc = include_str!("module-doc.md")]
        #[cfg_attr(feature="docs", doc=include_str!("conditional.md"))] mod child;
        std::include!("standard.rs",);
        const BYTES: &[u8] = core::include_bytes!("data.bin");
        const VALUE: u8 = ::std::include!("value.rs");
        other::include!("unowned.rs");
    "##;
    let files = [("out/lib.rs", source), ("out/child.rs", ""), ("out/module-doc.md", "docs"),
        ("out/standard.rs", ""), ("out/data.bin", "bytes"), ("out/value.rs", "42"), ("out/unowned.rs", ""), ("out/conditional.md", "docs")];
    let (graph, inputs) = source_graph("out/lib.rs", &files).unwrap();
    assert_eq!(graph.len(), 7);
    assert_eq!(graph, inputs.keys().cloned().collect());
    assert!(graph.contains("out/module-doc.md"));
    assert!(graph.contains("out/conditional.md"));
    assert!(!graph.contains("out/unowned.rs"));
    let unconditional = format!("{source}\ninclude!(\"unconditionally-missing.rs\"); const OLD:&str=include_str!(\"missing.txt\"); const OLD_BYTES:&[u8]=include_bytes!(\"missing.bin\"); #[path=\"stale.rs\"] mod stale; mod absent;");
    let mut files = files.to_vec();
    files[0].1 = &unconditional;
    assert_eq!(source_graph("out/lib.rs", &files).unwrap().0, graph);
    // A real inventory entry with unavailable bytes remains a read failure,
    // rather than being mistaken for an absent historical declaration.
    let inventory = BTreeSet::from(["out/lib.rs".into(), "out/broken.rs".into()]);
    let error = sources::reachable(&["out/lib.rs".into()], &inventory, &mut BTreeMap::new(), &mut |path| {
        Ok((path == "out/lib.rs").then(|| b"mod broken;".to_vec()))
    }).unwrap_err();
    assert!(error.contains("file is missing: out/broken.rs"));
}

#[test]
fn rust_external_source_graph_rejects_ambiguity_cycles_and_escape() {
    for (files, expected) in [
        (vec![("out/lib.rs", "mod both;"), ("out/both.rs", ""), ("out/both/mod.rs", "")], "ambiguous"),
        (vec![("out/lib.rs", "#[path=\"lib.rs\"] mod cycle;")], "cyclic"),
        (vec![("out/lib.rs", "#[path=\"../../escape.rs\"] mod escape;")], "escapes"),
        (vec![("out/lib.rs", "include_str!(\"../../escape.txt\");")], "escapes"),
    ] {
        let error = source_graph("out/lib.rs", &files).unwrap_err();
        assert!(error.contains(expected), "{expected}: {error}");
    }
}

#[test]
fn rust_external_source_graph_does_not_guess_dynamic_includes_or_consumers() {
    let (graph, inputs) = source_graph("out/lib.rs", &[
        ("out/lib.rs", "include!(env!(\"PATH\")); include!(concat!(\"../other/\",\"generated.rs\")); const DATA:&str=include_str!(env!(\"SOURCE_FILE\"));"),
        ("other/generated.rs", "pub fn unrelated(){}"),
        ("out/generated.rs", "pub fn decoy(){}"),
    ]).unwrap();
    assert_eq!(graph, BTreeSet::from(["out/lib.rs".into()]));
    assert_eq!(inputs.keys().cloned().collect::<BTreeSet<_>>(), graph);
}

#[test]
fn rust_source_ownership_preserves_metadata_only_local_targets() {
    let mut document: DocumentMut = "[package]\nname='tool'\nversion='1.0.0'\nbuild='build.rs'\n[lib]\npath='lib.rs'\n[[bin]]\nname='tool'\npath='src/main.rs'\n[[test]]\nname='check'\npath='tests/check.rs'\n[[bench]]\nname='bench'\npath='benches/bench.rs'\n[[example]]\nname='example'\npath='examples/example.rs'\n".parse().unwrap();
    let mut inventory = BTreeSet::from(["pkg/Cargo.toml".into()]);
    assert_eq!(cargo_source_roots("pkg/Cargo.toml", &document, &inventory).unwrap(), vec!["pkg/"]);
    assert_eq!(cargo_source_roots("Cargo.toml", &document, &BTreeSet::from(["Cargo.toml".into()])).unwrap(), vec![""]);

    document["lib"]["path"] = value("../shared/lib.rs");
    assert_eq!(cargo_source_roots("pkg/Cargo.toml", &document, &inventory).unwrap(), vec!["pkg/", "shared/lib.rs"]);
    let (missing, hashes) = source_graph("shared/lib.rs", &[("shared/lib.rs/unrelated.txt", "not a source file")]).unwrap();
    assert!(missing.is_empty() && hashes.is_empty());
    inventory.insert("shared/lib.rs".into());
    assert_eq!(cargo_source_roots("pkg/Cargo.toml", &document, &inventory).unwrap(), vec!["pkg/", "shared/lib.rs"]);
}

fn config(roots: &[&str]) -> RustConfig {
    RustConfig {
        workspaces: roots.iter().map(|s| s.to_string()).collect(),
        dependency_bump: "patch".into(),
        shared_inputs: Vec::new(),
        product_inputs: vec!["README.md".into()],
    }
}

fn fixture(config: &RustConfig, files: &[(&str, &str)]) -> Result<Workspace, String> {
    let files = files
        .iter()
        .map(|(p, b)| (p.to_string(), b.as_bytes().to_vec()))
        .collect::<BTreeMap<_, _>>();
    load(
        config,
        &files.keys().cloned().collect::<Vec<_>>(),
        &mut |p| Ok(files.get(p).cloned()),
    )
}

struct RenderedFixture {
    writes: Writes,
    packages: Vec<PackageChange>,
}
fn render_fixture(workspace: &Workspace, targets: &[(&str, &str)]) -> RenderedFixture {
    let versions = targets
        .iter()
        .map(|(p, v)| (p.to_string(), v.parse::<SemVer>().unwrap()))
        .collect::<BTreeMap<_, _>>();
    let writes = workspace.render_versions(&versions).unwrap();
    let packages = versions
        .iter()
        .map(|(path, version)| {
            let old = &workspace.packages[path];
            PackageChange {
                manifest: path.clone(),
                name: old.name.clone(),
                version_before: old.version.to_string(),
                version_after: version.to_string(),
                reason: "explicit_native_target".into(),
                dependencies: workspace
                    .dependencies
                    .iter()
                    .filter(|d| d.owner == *path && versions.contains_key(&d.target))
                    .map(|d| d.target.clone())
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect(),
            }
        })
        .collect();
    RenderedFixture { writes, packages }
}

fn graph() -> Vec<(&'static str, &'static str)> {
    vec![
        ("Cargo.toml", "[workspace]\nmembers = ['a','b','c']\n"),
        ("a/Cargo.toml", "[package]\nname = 'a'\nversion = '1.2.3'\n"),
        (
            "b/Cargo.toml",
            "[package]\nname = 'b'\nversion = '4.5.6'\n[dependencies]\na = { path = '../a', version = '=1.2.3' }\n",
        ),
        ("c/Cargo.toml", "[package]\nname = 'c'\nversion = '7.8.9'\n"),
        (
            "Cargo.lock",
            "version = 4\n[[package]]\nname = 'a'\nversion = '1.2.3'\n[[package]]\nname = 'b'\nversion = '4.5.6'\ndependencies = ['a']\n[[package]]\nname = 'c'\nversion = '7.8.9'\n",
        ),
    ]
}

#[test]
fn rust_explicit_versions_preserve_unrelated_packages() {
    let workspace = fixture(&config(&["Cargo.toml"]), &graph()).unwrap();
    let plan = render_fixture(
        &workspace,
        &[("a/Cargo.toml", "1.3.0"), ("b/Cargo.toml", "4.5.7")],
    );
    assert_eq!(plan.packages.len(), 2);
    assert_eq!(plan.packages[0].version_after, "1.3.0");
    assert_eq!(plan.packages[1].version_after, "4.5.7");
    assert_eq!(plan.packages[1].reason, "explicit_native_target");
    assert_eq!(plan.packages[1].dependencies, ["a/Cargo.toml"]);
    assert!(!plan.writes.contains_key("c/Cargo.toml"));
    assert!(
        String::from_utf8(plan.writes["b/Cargo.toml"].clone())
            .unwrap()
            .contains("=1.3.0")
    );
    assert!(
        String::from_utf8(plan.writes["Cargo.lock"].clone())
            .unwrap()
            .contains("1.3.0")
    );
}

#[test]
fn rust_explicit_versions_render_dependency_cycles() {
    let mut files = graph();
    files[1].1 = "[package]\nname='a'\nversion='1.2.3'\n[dev-dependencies]\nb={path='../b'}\n";
    let workspace = fixture(&config(&["Cargo.toml"]), &files).unwrap();
    let plan = render_fixture(
        &workspace,
        &[("a/Cargo.toml", "1.2.4"), ("b/Cargo.toml", "5.0.0")],
    );
    assert_eq!(plan.packages.len(), 2);
    assert_eq!(plan.packages[0].version_after, "1.2.4");
    assert_eq!(plan.packages[1].version_after, "5.0.0");
    assert_eq!(plan.packages[0].reason, "explicit_native_target");
}

#[test]
fn rust_inherited_versions_materialize_without_bumping_unrelated_members() {
    let files = [
        (
            "Cargo.toml",
            "[workspace]\nmembers=['a','b']\n[workspace.package]\nversion='1.2.3'\n[workspace.dependencies]\nalias={package='a',path='a',version='^1.2.3'}\n",
        ),
        (
            "a/Cargo.toml",
            "[package]\nname='a'\nversion.workspace=true\n",
        ),
        (
            "b/Cargo.toml",
            "[package]\nname='b'\nversion.workspace=true\n[target.'cfg(windows)'.build-dependencies]\nalias={workspace=true,optional=true}\n",
        ),
    ];
    let workspace = fixture(&config(&["Cargo.toml"]), &files).unwrap();
    let plan = render_fixture(
        &workspace,
        &[("a/Cargo.toml", "2.0.0"), ("b/Cargo.toml", "1.2.4")],
    );
    assert_eq!(plan.packages[0].version_after, "2.0.0");
    assert_eq!(plan.packages[1].version_after, "1.2.4");
    assert!(
        String::from_utf8(plan.writes["Cargo.toml"].clone())
            .unwrap()
            .contains("^2.0.0")
    );
    assert!(
        !String::from_utf8(plan.writes["b/Cargo.toml"].clone())
            .unwrap()
            .contains("version.workspace")
    );
    let unchanged = render_fixture(&workspace, &[]);
    assert!(unchanged.packages.is_empty());
    assert!(
        String::from_utf8(unchanged.writes["a/Cargo.toml"].clone())
            .unwrap()
            .contains("1.2.3")
    );
}

#[test]
fn rust_multiple_workspaces_and_renamed_cross_workspace_dependencies() {
    let files = [
        ("x/Cargo.toml", "[workspace]\nmembers=['a']\n"),
        ("x/a/Cargo.toml", "[package]\nname='a'\nversion='1.0.0'\n"),
        ("y/Cargo.toml", "[workspace]\nmembers=['b']\n"),
        (
            "y/b/Cargo.toml",
            "[package]\nname='b'\nversion='8.0.0'\n[dependencies]\nrenamed={package='a',path='../../x/a',version='1'}\n",
        ),
    ];
    let workspace = fixture(&config(&["x/Cargo.toml", "y/Cargo.toml"]), &files).unwrap();
    let plan = render_fixture(
        &workspace,
        &[("x/a/Cargo.toml", "1.0.1"), ("y/b/Cargo.toml", "8.0.1")],
    );
    assert_eq!(plan.packages.len(), 2);
    assert_eq!(plan.packages[1].version_after, "8.0.1");
    assert!(
        String::from_utf8(plan.writes["y/b/Cargo.toml"].clone())
            .unwrap()
            .contains("version='1'")
    );
}

#[test]
fn rust_lock_identity_does_not_rewrite_same_named_registry_package() {
    let mut files = graph();
    files[4].1 = "version=4\n[[package]]\nname='a'\nversion='1.2.3'\n[[package]]\nname='a'\nversion='1.2.3'\nsource='registry+https://example.invalid/index'\nchecksum='untouched'\n[[package]]\nname='b'\nversion='4.5.6'\ndependencies=['a 1.2.3', 'a 1.2.3 (registry+https://example.invalid/index)']\n[[package]]\nname='c'\nversion='7.8.9'\n";
    let workspace = fixture(&config(&["Cargo.toml"]), &files).unwrap();
    let plan = render_fixture(
        &workspace,
        &[("a/Cargo.toml", "1.2.4"), ("b/Cargo.toml", "4.5.7")],
    );
    let lock = parse("Cargo.lock", &plan.writes["Cargo.lock"]).unwrap();
    let entries = lock["package"].as_array_of_tables().unwrap();
    assert_eq!(entries.get(0).unwrap()["version"].as_str(), Some("1.2.4"));
    assert_eq!(entries.get(1).unwrap()["version"].as_str(), Some("1.2.3"));
    let deps = entries.get(2).unwrap()["dependencies"].as_array().unwrap();
    assert_eq!(deps.get(0).unwrap().as_str(), Some("a 1.2.4"));
    assert_eq!(
        deps.get(1).unwrap().as_str(),
        Some("a 1.2.3 (registry+https://example.invalid/index)")
    );
}

#[test]
fn rust_baseline_rejects_identity_and_unmanaged_version_changes() {
    let old = fixture(&config(&["Cargo.toml"]), &graph()).unwrap();
    let mut files = graph();
    files[1].1 = "[package]\nname='a'\nversion='9.0.0'\n";
    files[4].1 = "version=4\n[[package]]\nname='a'\nversion='9.0.0'\n[[package]]\nname='b'\nversion='4.5.6'\n[[package]]\nname='c'\nversion='7.8.9'\n";
    files[2].1 = "[package]\nname='b'\nversion='4.5.6'\n";
    let current = fixture(&config(&["Cargo.toml"]), &files).unwrap();
    assert!(
        current
            .validate_baseline(&old)
            .unwrap_err()
            .contains("outside")
    );
}

#[test]
fn rust_addition_guards_reject_removal_rename_boundary_and_conflicts() {
    let cfg = config(&["Cargo.toml"]);
    let old_files = [
        ("Cargo.toml", "[workspace]\nmembers=['a']\n"),
        ("a/Cargo.toml", "[package]\nname='a'\nversion='1.0.0'\n"),
        (
            "unmanaged/Cargo.toml",
            "[package]\nname='unmanaged'\nversion='0.1.0'\n",
        ),
    ];
    let old = fixture(&cfg, &old_files).unwrap();
    let cases = [
        (
            vec![
                ("Cargo.toml", "[workspace]\nmembers=['new']\n"),
                ("new/Cargo.toml", "[package]\nname='a'\nversion='1.0.0'\n"),
            ],
            "removal/rename",
        ),
        (
            vec![
                ("Cargo.toml", "[workspace]\nmembers=['a']\n"),
                (
                    "a/Cargo.toml",
                    "[package]\nname='renamed'\nversion='1.0.0'\n",
                ),
            ],
            "identity",
        ),
        (
            vec![
                ("Cargo.toml", "[workspace]\nmembers=['a','unmanaged']\n"),
                old_files[1],
                old_files[2],
            ],
            "existing manifest",
        ),
        (
            vec![
                ("Cargo.toml", "[workspace]\nmembers=['a','new']\n"),
                old_files[1],
                ("new/Cargo.toml", "[package]\nname='a'\nversion='0.1.0'\n"),
            ],
            "conflicts",
        ),
    ];
    for (files, expected) in cases {
        let error = fixture(&cfg, &files)
            .unwrap()
            .validate_baseline(&old)
            .unwrap_err();
        assert!(error.contains(expected), "{expected}: {error}");
    }
    let files = [
        ("Cargo.toml", "[workspace]\nmembers=['b']\n"),
        (
            "b/Cargo.toml",
            "[package]\nname='b'\nversion='1.0.0'\n[workspace]\nmembers=['../a']\n",
        ),
        old_files[1],
    ];
    assert!(
        fixture(&config(&["b/Cargo.toml"]), &files)
            .unwrap()
            .validate_baseline(&old)
            .unwrap_err()
            .contains("boundary")
    );
}

#[test]
fn rust_rejects_stale_lock_unregistered_dependency_and_escape() {
    let mut files = graph();
    files[4].1 = "version=4\n[[package]]\nname='a'\nversion='9.0.0'\n";
    assert!(
        fixture(&config(&["Cargo.toml"]), &files)
            .unwrap_err()
            .contains("stale")
    );
    files = graph();
    files[2].1 =
        "[package]\nname='b'\nversion='4.5.6'\n[dependencies]\nmissing={path='../missing'}\n";
    assert!(
        fixture(&config(&["Cargo.toml"]), &files)
            .unwrap_err()
            .contains("missing")
    );
    files[2].1 = "[package]\nname='b'\nversion='4.5.6'\n[dependencies]\nmissing={path='../../../../outside'}\n";
    assert!(
        fixture(&config(&["Cargo.toml"]), &files)
            .unwrap_err()
            .contains("escapes")
    );
}

#[test]
fn rust_member_globs_and_excludes_are_explicit_and_deterministic() {
    let files = [
        (
            "Cargo.toml",
            "[workspace]\nmembers=['crates/*']\nexclude=['crates/ignored']\n",
        ),
        (
            "crates/a/Cargo.toml",
            "[package]\nname='a'\nversion='1.0.0'\n",
        ),
        (
            "crates/ignored/Cargo.toml",
            "[package]\nname='ignored'\nversion='1.0.0'\n",
        ),
    ];
    let workspace = fixture(&config(&["Cargo.toml"]), &files).unwrap();
    assert_eq!(workspace.packages.len(), 1);
    assert!(!workspace.packages.contains_key("crates/ignored/Cargo.toml"));
}

#[test]
fn rust_version_requirement_ranges_preserve_compatible_constraints() {
    for (raw, yes, no) in [
        ("1", "1.9.0", "2.0.0"),
        ("0.2", "0.2.9", "0.3.0"),
        ("~1.2", "1.2.9", "1.3.0"),
        ("=1.2.3", "1.2.3", "1.2.4"),
        (">=1.2, <2", "1.9.0", "2.0.0"),
        ("1.*", "1.8.2", "2.0.0"),
        (">1.2", "1.3.0", "1.2.9"),
        ("<=1.2", "1.2.9", "1.3.0"),
    ] {
        assert!(accepts(raw, yes.parse().unwrap()).unwrap(), "{raw}: {yes}");
        assert!(!accepts(raw, no.parse().unwrap()).unwrap(), "{raw}: {no}");
    }
    assert!(updated_requirement(">=1, <2", "2.0.0".parse().unwrap()).is_err());
    assert!(accepts("01.2.3", "1.2.3".parse().unwrap()).is_err());
    assert!(accepts(">=1.*", "1.2.3".parse().unwrap()).is_err());
}

#[test]
fn rust_cross_workspace_lock_requires_complete_local_dependency_closure() {
    let mut files = vec![
        (
            "x/Cargo.toml",
            "[package]\nname='a'\nversion='1.0.0'\n[workspace]\n",
        ),
        (
            "y/Cargo.toml",
            "[package]\nname='b'\nversion='8.0.0'\n[workspace]\n[dependencies]\na={path='../x'}\n",
        ),
        (
            "y/Cargo.lock",
            "version=4\n[[package]]\nname='b'\nversion='8.0.0'\n",
        ),
    ];
    let configuration = config(&["x/Cargo.toml", "y/Cargo.toml"]);
    assert!(
        fixture(&configuration, &files)
            .unwrap_err()
            .contains("missing")
    );
    files[2].1 = "version=4\n[[package]]\nname='b'\nversion='8.0.0'\ndependencies=['a']\n[[package]]\nname='a'\nversion='1.0.0'\n";
    let workspace = fixture(&configuration, &files).unwrap();
    let plan = render_fixture(
        &workspace,
        &[("x/Cargo.toml", "1.0.1"), ("y/Cargo.toml", "8.0.1")],
    );
    assert_eq!(plan.packages.len(), 2);
    let lock = parse("y/Cargo.lock", &plan.writes["y/Cargo.lock"]).unwrap();
    assert_eq!(
        lock["package"]
            .as_array_of_tables()
            .unwrap()
            .get(1)
            .unwrap()["version"]
            .as_str(),
        Some("1.0.1")
    );
    assert_eq!(workspace.inputs()["x/Cargo.lock"], None);
    assert!(!plan.writes.contains_key("x/Cargo.lock"));
}

fn cross_workspace_dev_fixture() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "x/Cargo.toml",
            "[workspace]\nmembers=['core','helper']\nresolver='2'\n",
        ),
        (
            "x/core/Cargo.toml",
            "[package]\nname='local-core'\nversion='1.0.0'\nedition='2024'\n[dev-dependencies]\nlocal-helper={path='../helper'}\n",
        ),
        (
            "x/helper/Cargo.toml",
            "[package]\nname='local-helper'\nversion='2.0.0'\nedition='2024'\n",
        ),
        (
            "y/Cargo.toml",
            "[workspace]\nmembers=['app']\nresolver='2'\n",
        ),
        (
            "y/app/Cargo.toml",
            "[package]\nname='local-app'\nversion='3.0.0'\nedition='2024'\n[dependencies]\nlocal-core={path='../../x/core'}\n",
        ),
        ("x/core/src/lib.rs", "pub fn core() {}\n"),
        ("x/helper/src/lib.rs", "pub fn helper() {}\n"),
        ("y/app/src/lib.rs", "pub fn app() {}\n"),
    ]
}

fn cross_workspace_optional_fixture(
    default_on: bool,
    explicit_on: bool,
    disable_default: bool,
) -> Vec<(&'static str, &'static str)> {
    let mut files = cross_workspace_dev_fixture();
    files[1].1 = if default_on {
        "[package]\nname='local-core'\nversion='1.0.0'\nedition='2024'\n[dependencies]\nlocal-helper={path='../helper',optional=true}\n[features]\ndefault=['with-helper']\nwith-helper=['dep:local-helper']\n"
    } else {
        "[package]\nname='local-core'\nversion='1.0.0'\nedition='2024'\n[dependencies]\nlocal-helper={path='../helper',optional=true}\n[features]\nwith-helper=['dep:local-helper']\n"
    };
    files[4].1 = if explicit_on {
        "[package]\nname='local-app'\nversion='3.0.0'\nedition='2024'\n[dependencies]\nlocal-core={path='../../x/core',features=['with-helper']}\n"
    } else if disable_default {
        "[package]\nname='local-app'\nversion='3.0.0'\nedition='2024'\n[dependencies]\nlocal-core={path='../../x/core',default-features=false}\n"
    } else {
        "[package]\nname='local-app'\nversion='3.0.0'\nedition='2024'\n[dependencies]\nlocal-core={path='../../x/core'}\n"
    };
    files
}

#[test]
fn rust_foreign_optional_lock_edges_follow_defaults_and_explicit_features() {
    for (default_on, explicit_on, disable_default, expected) in [
        (false, false, false, false),
        (true, false, true, false),
        (false, true, false, true),
        (true, false, false, true),
    ] {
        let mut files = cross_workspace_optional_fixture(default_on, explicit_on, disable_default);
        files.extend([
            ("x/Cargo.lock", "version=4\n[[package]]\nname='local-core'\nversion='1.0.0'\ndependencies=['local-helper']\n[[package]]\nname='local-helper'\nversion='2.0.0'\n"),
            ("y/Cargo.lock", "version=4\n[[package]]\nname='local-app'\nversion='3.0.0'\ndependencies=['local-core']\n[[package]]\nname='local-core'\nversion='1.0.0'\n"),
        ]);
        let result = fixture(&config(&["x/Cargo.toml", "y/Cargo.toml"]), &files);
        if expected {
            assert!(
                result.unwrap_err().contains("local-helper"),
                "default={default_on},explicit={explicit_on},disabled={disable_default}"
            );
        } else {
            let workspace = result.unwrap();
            let plan = render_fixture(
                &workspace,
                &[
                    ("x/core/Cargo.toml", "1.0.1"),
                    ("x/helper/Cargo.toml", "2.0.1"),
                    ("y/app/Cargo.toml", "3.0.1"),
                ],
            );
            assert_eq!(
                plan.packages.len(),
                3,
                "all three native targets were explicitly requested"
            );
            assert!(
                !String::from_utf8(plan.writes["y/Cargo.lock"].clone())
                    .unwrap()
                    .contains("local-helper")
            );
        }
    }
}

#[test]
fn rust_explicit_cross_workspace_versions_preserve_foreign_lock_boundaries() {
    let mut files = cross_workspace_dev_fixture();
    files.extend([
        ("x/Cargo.lock", "version=4\n[[package]]\nname='local-core'\nversion='1.0.0'\ndependencies=['local-helper']\n[[package]]\nname='local-helper'\nversion='2.0.0'\n"),
        ("y/Cargo.lock", "version=4\n[[package]]\nname='local-app'\nversion='3.0.0'\ndependencies=['local-core']\n[[package]]\nname='local-core'\nversion='1.0.0'\n"),
    ]);
    let workspace = fixture(&config(&["x/Cargo.toml", "y/Cargo.toml"]), &files).unwrap();
    // Targets are explicit; lock validation follows Cargo's narrower graph.
    let plan = render_fixture(
        &workspace,
        &[
            ("x/core/Cargo.toml", "1.0.1"),
            ("x/helper/Cargo.toml", "2.0.1"),
            ("y/app/Cargo.toml", "3.0.1"),
        ],
    );
    assert_eq!(plan.packages.len(), 3);
    assert_eq!(
        plan.packages
            .iter()
            .find(|p| p.name == "local-core")
            .unwrap()
            .reason,
        "explicit_native_target"
    );
    let lock = String::from_utf8(plan.writes["y/Cargo.lock"].clone()).unwrap();
    assert!(lock.contains("1.0.1"));
    assert!(!lock.contains("local-helper"));
}

#[test]
fn rust_cross_workspace_locks_are_accepted_by_offline_locked_cargo() {
    use std::fs;
    use std::path::PathBuf;
    use std::process::{Command, Stdio};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static COUNTER: AtomicU64 = AtomicU64::new(0);
    struct FixtureDirectory(PathBuf);
    impl Drop for FixtureDirectory {
        fn drop(&mut self) {
            // The only recursively removed path is this generated, absolute,
            // test-owned child of the operating system's temporary directory.
            let temp = std::env::temp_dir();
            if self.0.is_absolute()
                && self.0.starts_with(&temp)
                && self
                    .0
                    .file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with("bridgeforge-rust-lock-"))
            {
                let _ = fs::remove_dir_all(&self.0);
            }
        }
    }
    let scenarios = vec![
        (cross_workspace_dev_fixture(), false),
        (cross_workspace_optional_fixture(false, false, false), false),
        (cross_workspace_optional_fixture(true, false, true), false),
        (cross_workspace_optional_fixture(false, true, false), true),
        (cross_workspace_optional_fixture(true, false, false), true),
    ];
    for (files, helper_in_foreign_lock) in scenarios {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = FixtureDirectory(std::env::temp_dir().join(format!(
            "bridgeforge-rust-lock-{unique}-{}",
            COUNTER.fetch_add(1, Ordering::Relaxed)
        )));
        for (relative, contents) in &files {
            let path = root.0.join(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, contents).unwrap();
        }
        let cargo = |command: &str, manifest: &str, locked: bool| {
            let mut process = Command::new(env!("CARGO"));
            process
                .arg(command)
                .arg("--offline")
                .arg("--manifest-path")
                .arg(root.0.join(manifest));
            if locked {
                process.arg("--locked").arg("--format-version").arg("1");
            }
            let output = process
                .current_dir(&root.0)
                .stdin(Stdio::null())
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "cargo {command} {manifest}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        };
        for manifest in ["x/Cargo.toml", "y/Cargo.toml"] {
            cargo("generate-lockfile", manifest, false);
        }
        let mut inventory = files.iter().map(|(p, _)| p.to_string()).collect::<Vec<_>>();
        inventory.extend(["x/Cargo.lock".into(), "y/Cargo.lock".into()]);
        let workspace = load(
            &config(&["x/Cargo.toml", "y/Cargo.toml"]),
            &inventory,
            &mut |p| match fs::read(root.0.join(p)) {
                Ok(bytes) => Ok(Some(bytes)),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
                Err(error) => Err(error.to_string()),
            },
        )
        .unwrap();
        let plan = render_fixture(
            &workspace,
            &[
                ("x/core/Cargo.toml", "1.0.1"),
                ("x/helper/Cargo.toml", "2.0.1"),
                ("y/app/Cargo.toml", "3.0.1"),
            ],
        );
        assert_eq!(plan.packages.len(), 3);
        assert_eq!(
            String::from_utf8(plan.writes["y/Cargo.lock"].clone())
                .unwrap()
                .contains("local-helper"),
            helper_in_foreign_lock
        );
        for (path, bytes) in plan.writes {
            fs::write(root.0.join(path), bytes).unwrap();
        }
        for manifest in ["x/Cargo.toml", "y/Cargo.toml"] {
            cargo("metadata", manifest, true);
        }
    }
}

#[test]
fn rust_unknown_lock_formats_are_rejected_before_render() {
    for header in ["", "version=2\n", "version=5\n", "version='4'\n"] {
        let files = [
            ("Cargo.toml", "[package]\nname='a'\nversion='1.0.0'\n"),
            ("Cargo.lock", header),
        ];
        assert!(
            fixture(&config(&["Cargo.toml"]), &files)
                .unwrap_err()
                .contains("format version 3 or 4")
        );
    }
    let files = [
        ("Cargo.toml", "[package]\nname='a'\nversion='1.0.0'\n"),
        (
            "Cargo.lock",
            "version=3\n[[package]]\nname='a'\nversion='1.0.0'\n",
        ),
    ];
    assert!(fixture(&config(&["Cargo.toml"]), &files).is_ok());
}

#[test]
fn rust_workspace_member_cannot_be_registered_as_an_independent_root() {
    let files = [
        ("Cargo.toml", "[workspace]\nmembers=['core']\n"),
        (
            "core/Cargo.toml",
            "[package]\nname='core'\nversion='1.0.0'\n",
        ),
        (
            "Cargo.lock",
            "version=4\n[[package]]\nname='core'\nversion='1.0.0'\n",
        ),
    ];
    let error = fixture(&config(&["core/Cargo.toml"]), &files).unwrap_err();
    assert!(error.contains("actual workspace root"), "{error}");
    let workspace = fixture(&config(&["Cargo.toml"]), &files).unwrap();
    let plan = render_fixture(&workspace, &[("core/Cargo.toml", "1.0.1")]);
    assert!(plan.writes.contains_key("Cargo.lock"));
    assert!(!plan.writes.contains_key("core/Cargo.lock"));
}

#[test]
fn rust_workspace_pointer_requires_the_actual_root() {
    let files = [
        ("Cargo.toml", "[workspace]\nmembers=['core']\n"),
        (
            "core/Cargo.toml",
            "[package]\nname='core'\nversion='1.0.0'\nworkspace='..'\n",
        ),
    ];
    assert!(
        fixture(&config(&["core/Cargo.toml"]), &files)
            .unwrap_err()
            .contains("package.workspace identifies Cargo.toml")
    );
    assert!(fixture(&config(&["Cargo.toml"]), &files).is_ok());
}

#[test]
fn rust_nested_independent_workspaces_and_excluded_packages_keep_their_lock_boundary() {
    let files = [
        (
            "Cargo.toml",
            "[workspace]\nmembers=['core']\nexclude=['standalone']\n",
        ),
        (
            "core/Cargo.toml",
            "[package]\nname='core'\nversion='1.0.0'\n",
        ),
        (
            "standalone/Cargo.toml",
            "[package]\nname='standalone'\nversion='2.0.0'\n",
        ),
        (
            "nested/Cargo.toml",
            "[package]\nname='nested'\nversion='3.0.0'\n[workspace]\n",
        ),
    ];
    let workspace = fixture(
        &config(&["standalone/Cargo.toml", "nested/Cargo.toml"]),
        &files,
    )
    .unwrap();
    assert!(workspace.inputs().contains_key("Cargo.toml"));
    let plan = render_fixture(&workspace, &[("standalone/Cargo.toml", "2.0.1")]);
    assert_eq!(plan.packages.len(), 1);
    assert!(!plan.writes.contains_key("Cargo.lock"));
    assert_eq!(workspace.inputs()["standalone/Cargo.lock"], None);
}

#[test]
fn rust_absent_ancestor_manifests_are_release_inputs() {
    let files = [(
        "standalone/nested/Cargo.toml",
        "[package]\nname='standalone'\nversion='1.0.0'\n",
    )];
    let workspace = fixture(&config(&["standalone/nested/Cargo.toml"]), &files).unwrap();
    assert_eq!(workspace.inputs()["standalone/Cargo.toml"], None);
    assert_eq!(workspace.inputs()["Cargo.toml"], None);
}

#[test]
fn rust_same_named_local_lock_identity_is_rejected_across_workspaces() {
    let files = [
        (
            "x/Cargo.toml",
            "[package]\nname='same'\nversion='1.0.0'\n[workspace]\n",
        ),
        (
            "y/Cargo.toml",
            "[package]\nname='same'\nversion='1.0.0'\n[workspace]\n",
        ),
        (
            "x/Cargo.lock",
            "version=4\n[[package]]\nname='same'\nversion='1.0.0'\n",
        ),
    ];
    assert!(
        fixture(&config(&["x/Cargo.toml", "y/Cargo.toml"]), &files)
            .unwrap_err()
            .contains("unique registered identity")
    );
}

#[test]
fn rust_configuration_rejects_ambiguous_maps_and_unsupported_patch() {
    let mut configuration = config(&["Cargo.toml"]);
    configuration.shared_inputs.push(SharedInput {
        path: "README.md".into(),
        packages: vec!["a/Cargo.toml".into()],
    });
    assert!(
        fixture(&configuration, &graph())
            .unwrap_err()
            .contains("overlapping")
    );
    let mut files = graph();
    files[0].1 = "[workspace]\nmembers=['a','b','c']\n[patch.crates-io]\na={path='a'}\n";
    assert!(
        fixture(&config(&["Cargo.toml"]), &files)
            .unwrap_err()
            .contains("patch/replace")
    );
}

fn implicit_graph() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "Cargo.toml",
            "[workspace]\nmembers=['app']\n[workspace.package]\nversion='1.0.0'\n[workspace.dependencies]\nrenamed={package='research',path='research'}\n",
        ),
        (
            "app/Cargo.toml",
            "[package]\nname='app'\nversion.workspace=true\n[target.'cfg(windows)'.build-dependencies]\nrenamed={workspace=true}\n",
        ),
        (
            "research/Cargo.toml",
            "[package]\nname='research'\nversion.workspace=true\n[dev-dependencies]\nleaf={path='../leaf'}\n",
        ),
        (
            "leaf/Cargo.toml",
            "[package]\nname='leaf'\nversion.workspace=true\n",
        ),
    ]
}

#[test]
fn rust_implicit_members_recurse_through_inherited_and_target_dependencies() {
    let mut cfg = config(&["Cargo.toml"]);
    cfg.shared_inputs.push(SharedInput {
        path: "shared.txt".into(),
        packages: vec!["research/Cargo.toml".into()],
    });
    let workspace = fixture(&cfg, &implicit_graph()).unwrap();
    assert_eq!(workspace.packages.len(), 3);
    let plan = render_fixture(
        &workspace,
        &[
            ("research/Cargo.toml", "1.1.0"),
            ("app/Cargo.toml", "1.0.1"),
        ],
    );
    assert_eq!(plan.packages.len(), 2);
    assert!(
        plan.packages
            .iter()
            .any(|p| p.name == "research" && p.version_after == "1.1.0")
    );
    assert!(
        plan.packages
            .iter()
            .any(|p| p.name == "app" && p.version_after == "1.0.1")
    );
    workspace
        .validate_baseline(&fixture(&cfg, &implicit_graph()).unwrap())
        .unwrap();
}

#[test]
fn rust_implicit_discovery_respects_excludes_and_independent_roots() {
    let mut files = implicit_graph();
    files[0].1 = "[workspace]\nmembers=['app']\nexclude=['research']\n[workspace.package]\nversion='1.0.0'\n[workspace.dependencies]\nrenamed={package='research',path='research'}\n";
    assert!(
        fixture(&config(&["Cargo.toml"]), &files)
            .unwrap_err()
            .contains("registered")
    );
    files[2].1 = "[package]\nname='research'\nversion='1.0.0'\n[workspace]\n";
    let workspace = fixture(&config(&["Cargo.toml", "research/Cargo.toml"]), &files).unwrap();
    assert_eq!(workspace.roots["Cargo.toml"].len(), 1);
    assert_eq!(workspace.roots["research/Cargo.toml"].len(), 1);
}

#[test]
fn rust_implicit_members_match_offline_cargo_metadata() {
    use std::fs;
    let root = std::env::temp_dir().join(format!(
        "bf-implicit-cargo-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    let root = Cleanup(root);
    let mut files = implicit_graph();
    files.extend([
        ("app/src/lib.rs", "pub fn x() {}"),
        ("research/src/lib.rs", "pub fn y() {}"),
        ("leaf/src/lib.rs", "pub fn z() {}"),
    ]);
    for (path, body) in &files {
        let path = root.0.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, body).unwrap();
    }
    let output = std::process::Command::new(env!("CARGO"))
        .args(["metadata", "--offline", "--format-version", "1"])
        .current_dir(&root.0)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let metadata: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let workspace = fixture(&config(&["Cargo.toml"]), &files).unwrap();
    let expected = metadata["workspace_members"].as_array().unwrap().len();
    assert_eq!(workspace.packages.len(), expected);
    assert_eq!(expected, 3);
}

fn adapter_files(files: &[(&str, &str)]) -> BTreeMap<String, Vec<u8>> {
    files
        .iter()
        .map(|(path, bytes)| (path.to_string(), bytes.as_bytes().to_vec()))
        .collect()
}

#[test]
fn rust_common_snapshot_preserves_kinds_aliases_and_current_identity() {
    let files = adapter_files(&[
        (
            "Cargo.toml",
            "[workspace]\nmembers=['foundation','consumer']\n",
        ),
        (
            "foundation/Cargo.toml",
            "[package]\nname='foundation'\nversion='1.2.3'\n",
        ),
        (
            "consumer/Cargo.toml",
            "[package]\nname='consumer'\nversion='2.0.0'\n[dependencies]\napi={package='foundation',path='../foundation'}\n[target.'cfg(windows)'.build-dependencies]\nbuild_api={package='foundation',path='../foundation'}\n[dev-dependencies]\ntest_api={package='foundation',path='../foundation'}\n",
        ),
    ]);
    let snapshot = snapshot(
        &config(&["Cargo.toml"]),
        &files.keys().cloned().collect::<Vec<_>>(),
        &mut |path| Ok(files.get(path).cloned()),
    )
    .unwrap();
    let package = &snapshot.packages["foundation/Cargo.toml"];
    assert_eq!(package.language, Language::Rust);
    assert_eq!(package.name, "foundation");
    assert_eq!(package.version, "1.2.3");
    assert_eq!(package.workspace, "Cargo.toml");
    assert_eq!(package.source_roots, ["foundation/"]);
    assert_eq!(
        snapshot
            .dependencies
            .iter()
            .map(|d| d.kind.as_str())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["normal", "build", "dev"])
    );
    assert!(
        snapshot
            .dependencies
            .iter()
            .all(|d| d.owner == "consumer/Cargo.toml" && d.target == "foundation/Cargo.toml")
    );
    assert_eq!(snapshot.inputs["Cargo.lock"], None);
}

#[test]
fn rust_common_historical_conversion_preserves_legacy_lock_scope() {
    let files = adapter_files(&[
        (
            "x/Cargo.toml",
            "[package]\nname='x'\nversion='1.0.0'\n[workspace]\n",
        ),
        (
            "x/Cargo.lock",
            "version=4\n[[package]]\nname='x'\nversion='1.0.0'\n",
        ),
        (
            "y/Cargo.toml",
            "[package]\nname='y'\nversion='2.0.0'\n[workspace]\n",
        ),
        (
            "y/Cargo.lock",
            "version=4\n[[package]]\nname='y'\nversion='1.0.0'\n",
        ),
    ]);
    let inventory = files.keys().cloned().collect::<Vec<_>>();
    let cfg = config(&["x/Cargo.toml", "y/Cargo.toml"]);
    let historical = load_legacy_history(
        &cfg,
        &inventory,
        &BTreeSet::from(["x/Cargo.toml".into()]),
        &mut |path| Ok(files.get(path).cloned()),
    )
    .unwrap();
    let historical = from_workspace(historical, &mut |path| Ok(files.get(path).cloned())).unwrap();
    assert_eq!(historical.packages["y/Cargo.toml"].version, "2.0.0");
    assert_eq!(
        historical.inputs["y/Cargo.lock"],
        files.get("y/Cargo.lock").cloned()
    );
    assert!(
        snapshot(&cfg, &inventory, &mut |path| Ok(files.get(path).cloned()))
            .unwrap_err()
            .contains("stale version")
    );
}

#[test]
fn rust_common_renderer_accepts_post_lifecycle_graphs_and_exact_versions() {
    // Old paths/names deliberately do not exist. The core validates the old/new
    // relation; the adapter only renders the complete post-refactor graph.
    for (members, manifests, lock) in [
        (
            "['renamed']",
            vec![(
                "renamed/Cargo.toml",
                "[package]\nname='renamed'\nversion='1.2.3'\n",
            )],
            "version=4\n[[package]]\nname='renamed'\nversion='1.2.3'\n",
        ),
        (
            "['new/location']",
            vec![(
                "new/location/Cargo.toml",
                "[package]\nname='moved'\nversion='1.2.3'\n",
            )],
            "version=4\n[[package]]\nname='moved'\nversion='1.2.3'\n",
        ),
        (
            "['survivor']",
            vec![(
                "survivor/Cargo.toml",
                "[package]\nname='survivor'\nversion='1.2.3'\n",
            )],
            "version=4\n[[package]]\nname='survivor'\nversion='1.2.3'\n",
        ),
        (
            "['part','new_part']",
            vec![
                (
                    "part/Cargo.toml",
                    "[package]\nname='part'\nversion='1.2.3'\n",
                ),
                (
                    "new_part/Cargo.toml",
                    "[package]\nname='new_part'\nversion='0.1.0'\n",
                ),
            ],
            "version=4\n[[package]]\nname='part'\nversion='1.2.3'\n[[package]]\nname='new_part'\nversion='0.1.0'\n",
        ),
        (
            "['merged']",
            vec![(
                "merged/Cargo.toml",
                "[package]\nname='merged'\nversion='1.2.3'\n",
            )],
            "version=4\n[[package]]\nname='merged'\nversion='1.2.3'\n",
        ),
    ] {
        let root = format!("[workspace]\nmembers={members}\n");
        let mut files = adapter_files(&manifests);
        files.insert("Cargo.toml".into(), root.into_bytes());
        files.insert("Cargo.lock".into(), lock.as_bytes().to_vec());
        let inventory = files.keys().cloned().collect::<Vec<_>>();
        let target = manifests[0].0;
        let versions = BTreeMap::from([(target.to_string(), "2.0.0".into())]);
        let writes = render_versions(
            &config(&["Cargo.toml"]),
            &inventory,
            &mut |path| Ok(files.get(path).cloned()),
            &versions,
        )
        .unwrap();
        assert_eq!(
            parse(target, &writes[target]).unwrap()["package"]["version"].as_str(),
            Some("2.0.0")
        );
        assert!(
            String::from_utf8(writes["Cargo.lock"].clone())
                .unwrap()
                .contains("2.0.0")
        );
        assert!(!writes.contains_key("new_part/Cargo.toml"));
    }
}

#[test]
fn rust_common_renderer_rejects_stale_local_locks_and_unknown_final_targets() {
    let mut files = adapter_files(&graph());
    let inventory = files.keys().cloned().collect::<Vec<_>>();
    let cfg = config(&["Cargo.toml"]);
    files.insert("Cargo.lock".into(), b"version=4\n[[package]]\nname='a'\nversion='1.2.3'\n[[package]]\nname='b'\nversion='4.5.6'\ndependencies=['old_name']\n[[package]]\nname='c'\nversion='7.8.9'\n".to_vec());
    assert!(
        snapshot(&cfg, &inventory, &mut |path| Ok(files.get(path).cloned()))
            .unwrap_err()
            .contains("missing or ambiguous target")
    );
    files.insert("Cargo.lock".into(), b"version=4\n[[package]]\nname='a'\nversion='1.2.3'\n[[package]]\nname='b'\nversion='4.5.6'\n[[package]]\nname='c'\nversion='7.8.9'\n".to_vec());
    assert!(
        snapshot(&cfg, &inventory, &mut |path| Ok(files.get(path).cloned()))
            .unwrap_err()
            .contains("missing local dependency edge b -> a")
    );
    let files = adapter_files(&graph());
    assert!(
        render_versions(
            &cfg,
            &inventory,
            &mut |path| Ok(files.get(path).cloned()),
            &BTreeMap::from([("retired/Cargo.toml".into(), "1.0.1".into())])
        )
        .unwrap_err()
        .contains("unknown package")
    );
    assert!(
        render_versions(
            &cfg,
            &inventory,
            &mut |path| Ok(files.get(path).cloned()),
            &BTreeMap::from([("a/Cargo.toml".into(), "1.0.1-beta.1".into())])
        )
        .unwrap_err()
        .contains("invalid final Rust version")
    );
    assert!(
        render_versions(
            &cfg,
            &inventory,
            &mut |path| Ok(files.get(path).cloned()),
            &BTreeMap::from([("a/Cargo.toml".into(), "1.2.3".into())])
        )
        .unwrap()
        .is_empty()
    );
}

#[test]
fn rust_common_snapshot_rejects_removed_edge_when_target_remains_locked_for_another_owner() {
    let mut source = graph();
    source[2].1 = "[package]\nname='b'\nversion='4.5.6'\n";
    source[3].1 = "[package]\nname='c'\nversion='7.8.9'\n[target.'cfg(windows)'.build-dependencies]\na={path='../a',version='=1.2.3',optional=true}\n";
    source[4].1 = "version=4\n[[package]]\nname='a'\nversion='1.2.3'\n[[package]]\nname='b'\nversion='4.5.6'\ndependencies=['a']\n[[package]]\nname='c'\nversion='7.8.9'\ndependencies=['a']\n";
    let mut files = adapter_files(&source);
    let inventory = files.keys().cloned().collect::<Vec<_>>();
    let cfg = config(&["Cargo.toml"]);
    // A remains a live workspace member and a legitimate target of C. The
    // schema 2 contract deliberately remains unchanged, but the common path
    // must reject B's deleted declaration instead of accepting its old lock edge.
    assert!(load(&cfg, &inventory, &mut |path| Ok(files.get(path).cloned())).is_ok());
    assert!(
        snapshot(&cfg, &inventory, &mut |path| Ok(files.get(path).cloned()))
            .unwrap_err()
            .contains("stale local dependency edge b -> a")
    );
    assert!(
        render_versions(
            &cfg,
            &inventory,
            &mut |path| Ok(files.get(path).cloned()),
            &BTreeMap::new()
        )
        .unwrap_err()
        .contains("stale local dependency edge b -> a")
    );

    // A declaration in another kind/target still makes the union edge valid;
    // current-platform filtering must not incorrectly discard it.
    files.insert("b/Cargo.toml".into(),b"[package]\nname='b'\nversion='4.5.6'\n[target.'cfg(unix)'.dev-dependencies]\na={path='../a'}\n".to_vec());
    assert!(snapshot(&cfg, &inventory, &mut |path| Ok(files.get(path).cloned())).is_ok());

    // Once the lock drops B's stale edge, retaining C's edge is valid.
    files.insert("b/Cargo.toml".into(), source[2].1.as_bytes().to_vec());
    files.insert("Cargo.lock".into(),b"version=4\n[[package]]\nname='a'\nversion='1.2.3'\n[[package]]\nname='b'\nversion='4.5.6'\n[[package]]\nname='c'\nversion='7.8.9'\ndependencies=['a']\n".to_vec());
    assert!(snapshot(&cfg, &inventory, &mut |path| Ok(files.get(path).cloned())).is_ok());
}

#[test]
fn rust_common_renderer_does_not_silently_widen_bounded_requirements() {
    let mut files = graph();
    files[2].1 = "[package]\nname='b'\nversion='4.5.6'\n[dependencies]\na={path='../a',version='>=1.0.0, <2.0.0'}\n";
    let files = adapter_files(&files);
    let inventory = files.keys().cloned().collect::<Vec<_>>();
    assert!(
        render_versions(
            &config(&["Cargo.toml"]),
            &inventory,
            &mut |path| Ok(files.get(path).cloned()),
            &BTreeMap::from([("a/Cargo.toml".into(), "2.0.0".into())])
        )
        .unwrap_err()
        .contains("cannot safely widen")
    );
}

#[test]
fn rust_common_renderer_preserves_registry_identity_and_unaffected_packages() {
    let mut files = graph();
    files[4].1 = "version=4\n[[package]]\nname='a'\nversion='1.2.3'\n[[package]]\nname='a'\nversion='1.2.3'\nsource='registry+https://example.invalid/index'\nchecksum='untouched'\n[[package]]\nname='b'\nversion='4.5.6'\ndependencies=['a 1.2.3', 'a 1.2.3 (registry+https://example.invalid/index)']\n[[package]]\nname='c'\nversion='7.8.9'\n";
    let files = adapter_files(&files);
    let inventory = files.keys().cloned().collect::<Vec<_>>();
    let writes = render_versions(
        &config(&["Cargo.toml"]),
        &inventory,
        &mut |path| Ok(files.get(path).cloned()),
        &BTreeMap::from([("a/Cargo.toml".into(), "2.0.0".into())]),
    )
    .unwrap();
    let lock = parse("Cargo.lock", &writes["Cargo.lock"]).unwrap();
    let entries = lock["package"].as_array_of_tables().unwrap();
    assert_eq!(entries.get(0).unwrap()["version"].as_str(), Some("2.0.0"));
    assert_eq!(entries.get(1).unwrap()["version"].as_str(), Some("1.2.3"));
    assert_eq!(
        entries.get(1).unwrap()["checksum"].as_str(),
        Some("untouched")
    );
    assert!(!writes.contains_key("c/Cargo.toml"));
    assert!(
        String::from_utf8(writes["b/Cargo.toml"].clone())
            .unwrap()
            .contains("=2.0.0")
    );
}

#[test]
fn rust_common_final_versions_match_locked_offline_cargo_metadata() {
    use std::fs;
    let path = std::env::temp_dir().join(format!(
        "bf-release-cargo-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    let root = Cleanup(path);
    let mut files = adapter_files(&[
        (
            "Cargo.toml",
            "[workspace]\nmembers=['foundation','consumer']\nresolver='3'\n[workspace.package]\nversion='1.2.3'\n[workspace.dependencies]\napi={package='foundation',path='foundation',version='=1.2.3'}\n",
        ),
        (
            "foundation/Cargo.toml",
            "[package]\nname='foundation'\nversion.workspace=true\nedition='2024'\n",
        ),
        (
            "consumer/Cargo.toml",
            "[package]\nname='consumer'\nversion.workspace=true\nedition='2024'\n[dependencies]\napi.workspace=true\n",
        ),
        ("foundation/src/lib.rs", "pub fn value() -> u8 { 1 }\n"),
        (
            "consumer/src/lib.rs",
            "pub fn value() -> u8 { api::value() }\n",
        ),
    ]);
    for (path, bytes) in &files {
        let path = root.0.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }
    let initial = std::process::Command::new(env!("CARGO"))
        .args(["metadata", "--offline", "--format-version", "1"])
        .current_dir(&root.0)
        .output()
        .unwrap();
    assert!(
        initial.status.success(),
        "{}",
        String::from_utf8_lossy(&initial.stderr)
    );
    files.insert(
        "Cargo.lock".into(),
        fs::read(root.0.join("Cargo.lock")).unwrap(),
    );
    let inventory = files.keys().cloned().collect::<Vec<_>>();
    let writes = render_versions(
        &config(&["Cargo.toml"]),
        &inventory,
        &mut |path| Ok(files.get(path).cloned()),
        &BTreeMap::from([
            ("foundation/Cargo.toml".into(), "2.0.0".into()),
            ("consumer/Cargo.toml".into(), "1.2.4".into()),
        ]),
    )
    .unwrap();
    for (path, bytes) in &writes {
        fs::write(root.0.join(path), bytes).unwrap();
    }
    let final_metadata = std::process::Command::new(env!("CARGO"))
        .args(["metadata", "--locked", "--offline", "--format-version", "1"])
        .current_dir(&root.0)
        .output()
        .unwrap();
    assert!(
        final_metadata.status.success(),
        "{}",
        String::from_utf8_lossy(&final_metadata.stderr)
    );
    let metadata: serde_json::Value = serde_json::from_slice(&final_metadata.stdout).unwrap();
    let versions = metadata["packages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| (p["name"].as_str().unwrap(), p["version"].as_str().unwrap()))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(
        versions,
        BTreeMap::from([("consumer", "1.2.4"), ("foundation", "2.0.0")])
    );
    assert_eq!(
        fs::read(root.0.join("Cargo.lock")).unwrap(),
        writes["Cargo.lock"]
    );
}
