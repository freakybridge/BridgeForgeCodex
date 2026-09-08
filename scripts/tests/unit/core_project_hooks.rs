use super::*;

fn config() -> Value {
    json!({"hooks":{"SessionStart":[{"matcher":"","hooks":[{"type":"command","command":"keep-custom"}]}]},
        "bridgeforgeProjectHooks":{"schema_version":1,"hooks":[{"id":"demo","events":[{"event":"SessionStart","args":["check"]},{"event":"Stop","args":["snapshot"]}]}]}})
}

fn temporary() -> Temporary {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "bridgeforge-project-hook-test-{}-{nonce}",
        std::process::id()
    ));
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
        ".codex/bin/project_demo.exe snapshot"
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
    let temp = temporary();
    fs::create_dir_all(temp.0.join(".codex")).unwrap();
    fs::write(temp.0.join(REGISTRY_PATH), &registry).unwrap();
    assert!(
        verify(&temp.0, &json!({}), false)
            .unwrap_err()
            .contains("requires hooks.json")
    );
    fs::write(
        temp.0.join(".codex/hooks.json"),
        serde_json::to_vec(&original).unwrap(),
    )
    .unwrap();
    assert!(
        verify(&temp.0, &json!({}), false)
            .unwrap_err()
            .contains("registrations drifted")
    );
}

#[test]
fn source_workspace_lock_registration_and_binary_are_bound_to_receipt() {
    let temp = temporary();
    let hook = hooks(&config()).unwrap().remove(0);
    let contract =
        json!({"generated_assets":[{"source_tree_sha256":"source","lockfile_sha256":"lock"}]});
    let input = identity(&hook, b"pub fn run(_:Vec<String>)->i32{0}", &contract).unwrap();
    assert_ne!(input, identity(&hook, b"changed", &contract).unwrap());
    for key in ["source_tree_sha256", "lockfile_sha256"] {
        let mut changed = contract.clone();
        changed["generated_assets"][0][key] = json!("changed");
        assert_ne!(
            input,
            identity(&hook, b"pub fn run(_:Vec<String>)->i32{0}", &changed).unwrap()
        );
    }
    fs::create_dir_all(temp.0.join(".codex/bin")).unwrap();
    fs::write(temp.0.join(hook.binary()), b"binary").unwrap();
    fs::write(temp.0.join(hook.receipt()),serde_json::to_vec(&json!({"schema_version":1,"id":hook.id,"input_sha256":input,"platform":std::env::consts::OS,"binary_sha256":sha(b"binary")})).unwrap()).unwrap();
    assert!(current(&temp.0, &hook, &input).unwrap());
    fs::write(temp.0.join(hook.binary()), b"drift").unwrap();
    assert!(!current(&temp.0, &hook, &input).unwrap());
}

#[test]
fn gui_subsystem_is_enforced() {
    let mut pe = vec![0; 256];
    pe[..2].copy_from_slice(b"MZ");
    pe[0x3c] = 128;
    pe[128..132].copy_from_slice(b"PE\0\0");
    pe[128 + 24 + 68] = 2;
    assert!(verify_windows_gui(&pe).is_ok());
    pe[128 + 24 + 68] = 3;
    assert!(verify_windows_gui(&pe).is_err());
    assert!(verify_windows_gui(b"not a PE").is_err());
}

#[test]
fn official_source_identity_excludes_project_owned_entrypoint() {
    let temp = temporary();
    fs::write(temp.0.join("Cargo.toml"), b"workspace").unwrap();
    fs::write(temp.0.join("Cargo.lock"), b"locked").unwrap();
    let before = crate::manifest::generated_sources(&temp.0).unwrap();
    fs::create_dir(temp.0.join("project_demo")).unwrap();
    fs::write(temp.0.join("project_demo/entrypoint.rs"), b"project owned").unwrap();
    assert_eq!(crate::manifest::generated_sources(&temp.0).unwrap(), before);
}

#[test]
fn dependency_receipt_rejects_external_and_cross_hook_sources() {
    let temp = temporary();
    fs::create_dir(temp.0.join("project_a")).unwrap();
    fs::create_dir(temp.0.join("project_b")).unwrap();
    fs::write(temp.0.join("project_a/entrypoint.rs"), b"own").unwrap();
    fs::write(temp.0.join("project_b/entrypoint.rs"), b"other").unwrap();
    let allowed = BTreeMap::from([("project_a/entrypoint.rs".into(), b"own".to_vec())]);
    let depfile = temp.0.join("project_a.d");
    let dep = |relative: &str| {
        format!(
            "output.exe: {}\n",
            temp.0.join(relative).to_string_lossy().replace(' ', "\\ ")
        )
    };
    fs::write(&depfile, dep("project_a/entrypoint.rs")).unwrap();
    verify_dependencies(&depfile, &temp.0, &allowed).unwrap();
    fs::write(&depfile, dep("project_b/entrypoint.rs")).unwrap();
    assert!(
        verify_dependencies(&depfile, &temp.0, &allowed)
            .unwrap_err()
            .contains("uncaptured dependency")
    );
    fs::write(temp.0.join("outside.rs"), b"external").unwrap();
    fs::write(&depfile, dep("outside.rs")).unwrap();
    assert!(verify_dependencies(&depfile, &temp.0, &allowed).is_err());
}

struct BuildRunner {
    root: PathBuf,
    response: Value,
    fail: bool,
    mutate: bool,
}
impl ProcessRunner for BuildRunner {
    fn run(&self, request: &ProcessRequest) -> std::io::Result<crate::ProcessOutput> {
        if request.program == "cargo" {
            assert!(request.args.iter().any(|a| a == "--locked"));
            assert_ne!(request.cwd, self.root);
            if self.fail {
                return Ok(crate::ProcessOutput {
                    code: 1,
                    stderr: b"fixture failure".to_vec(),
                    stdout: Vec::new(),
                    timed_out: false,
                });
            }
            let position = request
                .args
                .iter()
                .position(|a| a == "--target-dir")
                .unwrap();
            let directory = PathBuf::from(&request.args[position + 1]).join("release");
            fs::create_dir_all(&directory)?;
            let mut pe = vec![0; 256];
            pe[..2].copy_from_slice(b"MZ");
            pe[0x3c] = 128;
            pe[128..132].copy_from_slice(b"PE\0\0");
            pe[220] = 2;
            fs::write(
                directory.join(format!("project_demo{}", std::env::consts::EXE_SUFFIX)),
                pe,
            )?;
            fs::write(
                directory.join("project_demo.d"),
                format!(
                    "binary: {}\n",
                    request
                        .cwd
                        .join("project_demo/entrypoint.rs")
                        .to_string_lossy()
                        .replace(' ', "\\ ")
                ),
            )?;
            if self.mutate {
                fs::write(request.cwd.join("Cargo.lock"), b"changed")?;
            }
            Ok(crate::ProcessOutput {
                code: 0,
                stdout: Vec::new(),
                stderr: Vec::new(),
                timed_out: false,
            })
        } else {
            Ok(crate::ProcessOutput {
                stdout: serde_json::to_vec(&self.response).unwrap(),
                code: 0,
                stderr: Vec::new(),
                timed_out: false,
            })
        }
    }
}

#[test]
fn build_is_isolated_and_failures_never_install_or_mutate_the_lock() {
    let temp = temporary();
    let workspace = temp.0.join("managed");
    let project = temp.0.join("project");
    fs::create_dir(&workspace).unwrap();
    fs::create_dir(&project).unwrap();
    fs::write(
        workspace.join("Cargo.toml"),
        b"[package]\nname=\"bridgeforge-hook\"\nversion=\"1.0.0\"\n",
    )
    .unwrap();
    fs::write(workspace.join("Cargo.lock"), b"locked\n").unwrap();
    let recipe = crate::manifest::generated_build_recipe("bridgeforge-hook");
    let self_test = json!({"args":["self-test","--json"],"expected_json":{"status":"ok"}});
    let item = json!({"manifest":"Cargo.toml","lockfile":"Cargo.lock","build":recipe,
        "self_test":self_test,"source_tree_sha256":crate::manifest::generated_source_sha(&workspace,&crate::manifest::generated_sources(&workspace).unwrap()).unwrap(),
        "lockfile_sha256":sha(b"locked\n"),"build_recipe_sha256":crate::manifest::canonical_sha(&recipe).unwrap(),"self_test_sha256":crate::manifest::canonical_sha(&self_test).unwrap()});
    let contract = json!({"generated_assets":[item]});
    let hook = hooks(&config()).unwrap().remove(0);
    let source = b"pub fn run(_:Vec<String>)->i32{0}".to_vec();
    let response = json!({"id":"demo","input_sha256":identity(&hook,&source,&contract).unwrap(),"status":"ok"});
    for (fail, mutate) in [(true, false), (false, true), (false, false)] {
        let runner = BuildRunner {
            root: project.clone(),
            response: response.clone(),
            fail,
            mutate,
        };
        let result = build_legacy(
            &workspace,
            &project,
            &contract,
            &[(hook.clone(), source.clone())],
            &runner,
        );
        if fail || mutate {
            assert!(result.is_err());
        } else {
            assert_eq!(result.unwrap().len(), 2);
        }
        assert!(fs::read_dir(&project).unwrap().next().is_none());
        assert_eq!(fs::read(workspace.join("Cargo.lock")).unwrap(), b"locked\n");
    }
}

fn independent_package(root: &Path, hook: &Hook) {
    let directory = root.join(format!(".codex/hooks/project_{}", hook.id));
    fs::create_dir_all(directory.join("extra/src")).unwrap();
    fs::write(directory.join("Cargo.toml"), b"[package]\nname=\"independent-hook\"\nversion=\"0.1.0\"\nedition=\"2021\"\n[workspace]\n[dependencies]\nextra={path=\"extra\"}\n").unwrap();
    fs::write(directory.join("Cargo.lock"), b"version = 4\n[[package]]\nname = \"independent-hook\"\nversion = \"0.1.0\"\ndependencies = [\"extra\"]\n[[package]]\nname = \"extra\"\nversion = \"0.1.0\"\n").unwrap();
    fs::write(
        directory.join("extra/Cargo.toml"),
        b"[package]\nname=\"extra\"\nversion=\"0.1.0\"\nedition=\"2021\"\n",
    )
    .unwrap();
    fs::write(
        directory.join("extra/src/lib.rs"),
        b"pub fn value()->i32{37}\n",
    )
    .unwrap();
    fs::write(
        directory.join("entrypoint.rs"),
        b"pub fn run(_:Vec<String>)->i32{println!(\"independent:{}\",extra::value());7}\n",
    )
    .unwrap();
}

#[test]
fn project_package_reserved_paths_reject_case_variants_and_preserve_similar_names() {
    let hook = hooks(&config()).unwrap().remove(0);
    for name in [
        ".cargo/config.toml",
        ".CARGO/config.toml",
        "nested/.CaRgO/config",
        ".BRIDGEFORGE-MAIN.RS",
        "nested/.BridgeForge-Main.rs",
    ] {
        let temp = temporary();
        independent_package(&temp.0, &hook);
        let relative = format!(".codex/hooks/project_demo/{name}");
        let path = temp.0.join(&relative);
        let source = read(&temp.0, &hook.source()).unwrap().unwrap();
        let writes = BTreeMap::from([(path.clone(), b"reserved".to_vec())]);
        assert!(
            capture_input(
                &temp.0,
                &hook,
                source.clone(),
                &writes,
                &[],
                &mut BTreeMap::new()
            )
            .unwrap_err()
            .contains("reserved"),
            "prospective: {name}"
        );
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"reserved").unwrap();
        assert!(
            capture_input(
                &temp.0,
                &hook,
                source.clone(),
                &BTreeMap::new(),
                &[],
                &mut BTreeMap::new()
            )
            .unwrap_err()
            .contains("reserved"),
            "{name}"
        );
    }
    let temp = temporary();
    independent_package(&temp.0, &hook);
    for name in [".cargo-notes", ".bridgeforge-main.rs.txt"] {
        fs::write(
            temp.0.join(format!(".codex/hooks/project_demo/{name}")),
            b"allowed",
        )
        .unwrap();
    }
    capture_input(
        &temp.0,
        &hook,
        read(&temp.0, &hook.source()).unwrap().unwrap(),
        &BTreeMap::new(),
        &[],
        &mut BTreeMap::new(),
    )
    .unwrap();
}

#[test]
fn project_package_captures_dependencies_lock_and_inventory_without_writes() {
    let temp = temporary();
    let hook = hooks(&config()).unwrap().remove(0);
    independent_package(&temp.0, &hook);
    let source = read(&temp.0, &hook.source()).unwrap().unwrap();
    let mut reads = BTreeMap::new();
    let input = capture_input(
        &temp.0,
        &hook,
        source.clone(),
        &BTreeMap::new(),
        &[],
        &mut reads,
    )
    .unwrap();
    verify_reads(&temp.0, &reads).unwrap();
    let identity = input.identity(&hook, &json!({})).unwrap();
    let helper = temp.0.join(".codex/hooks/project_demo/extra/src/lib.rs");
    fs::write(&helper, b"pub fn value()->i32{38}").unwrap();
    assert!(verify_reads(&temp.0, &reads).is_err());
    let changed = capture_input(
        &temp.0,
        &hook,
        source.clone(),
        &BTreeMap::new(),
        &[],
        &mut BTreeMap::new(),
    )
    .unwrap();
    assert_ne!(identity, changed.identity(&hook, &json!({})).unwrap());
    fs::write(&helper, b"pub fn value()->i32{37}\n").unwrap();
    let extra = temp.0.join(".codex/hooks/project_demo/added.rs");
    fs::write(&extra, b"new").unwrap();
    assert!(
        verify_reads(&temp.0, &reads)
            .unwrap_err()
            .contains("inventory")
    );
    fs::remove_file(extra).unwrap();
    let lock = temp.0.join(".codex/hooks/project_demo/Cargo.lock");
    fs::remove_file(lock).unwrap();
    assert!(
        capture_input(
            &temp.0,
            &hook,
            source,
            &BTreeMap::new(),
            &[],
            &mut BTreeMap::new()
        )
        .unwrap_err()
        .contains("Cargo.lock")
    );
}

#[test]
fn project_package_rejects_external_paths_and_tracks_legacy_mode_switch() {
    let temp = temporary();
    let hook = hooks(&config()).unwrap().remove(0);
    fs::create_dir_all(temp.0.join(".codex/hooks/project_demo")).unwrap();
    let mut reads = BTreeMap::new();
    let input = capture_input(
        &temp.0,
        &hook,
        b"legacy".to_vec(),
        &BTreeMap::new(),
        &[],
        &mut reads,
    )
    .unwrap();
    assert!(input.package.is_none());
    independent_package(&temp.0, &hook);
    assert!(verify_reads(&temp.0, &reads).is_err());
    fs::write(
        temp.0.join(".codex/hooks/project_demo/Cargo.toml"),
        b"[package]\nname=\"demo\"\nversion=\"0.1.0\"\n[dependencies]\nbad={path=\"../outside\"}\n",
    )
    .unwrap();
    assert!(
        capture_input(
            &temp.0,
            &hook,
            b"source".to_vec(),
            &BTreeMap::new(),
            &[],
            &mut BTreeMap::new()
        )
        .unwrap_err()
        .contains("escapes")
    );
}

#[test]
fn project_package_real_cargo_build_uses_independent_dependency_and_preserves_inputs() {
    let temp = temporary();
    let hook = hooks(&config()).unwrap().remove(0);
    independent_package(&temp.0, &hook);
    let mut reads = BTreeMap::new();
    let input = capture_input(
        &temp.0,
        &hook,
        read(&temp.0, &hook.source()).unwrap().unwrap(),
        &BTreeMap::new(),
        &[],
        &mut reads,
    )
    .unwrap();
    // No managed workspace exists: this package must not borrow its dependencies.
    let outputs = build(
        &temp.0.join("absent-workspace"),
        &temp.0,
        &json!({}),
        &[(hook.clone(), input.clone())],
        &crate::SystemProcessRunner,
    )
    .unwrap();
    verify_reads(&temp.0, &reads).unwrap();
    assert!(!temp.0.join(hook.binary()).exists());
    let executable = temp
        .0
        .join(format!("runtime{}", std::env::consts::EXE_SUFFIX));
    fs::write(&executable, &outputs[&temp.0.join(hook.binary())]).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();
    }
    let result = crate::SystemProcessRunner
        .run(&ProcessRequest::new(executable.into_os_string(), &temp.0))
        .unwrap();
    assert_eq!(result.code, 7);
    assert_eq!(
        String::from_utf8(result.stdout).unwrap().trim(),
        "independent:37"
    );
    let mut stale = input;
    stale
        .package
        .as_mut()
        .unwrap()
        .insert("Cargo.lock".into(), b"version = 4\n".to_vec());
    let error = build(
        &temp.0.join("absent-workspace"),
        &temp.0,
        &json!({}),
        &[(hook, stale)],
        &crate::SystemProcessRunner,
    )
    .unwrap_err();
    assert!(error.contains("project Rust hook build failed"), "{error}");
    verify_reads(&temp.0, &reads).unwrap();
}

#[test]
fn project_package_real_toml_dependency_needs_no_upstream_manifest_change() {
    let temp = temporary();
    let hook = hooks(&config()).unwrap().remove(0);
    let directory = temp.0.join(".codex/hooks/project_demo");
    fs::create_dir_all(&directory).unwrap();
    fs::write(directory.join("Cargo.toml"), b"[package]\nname=\"toml-hook\"\nversion=\"0.1.0\"\nedition=\"2021\"\n[workspace]\n[dependencies]\ntoml_edit=\"=0.23.7\"\n[[bin]]\nname=\"toml-hook\"\npath=\"entrypoint.rs\"\n").unwrap();
    fs::write(directory.join("entrypoint.rs"), b"pub fn run(_:Vec<String>)->i32{let doc=\"answer=42\".parse::<toml_edit::DocumentMut>().unwrap();println!(\"{}\",doc[\"answer\"].as_integer().unwrap());0}").unwrap();
    // Dependency authoring is confined to this fixture; production builds never
    // generate a lock. The crate is already part of the factory's locked cache.
    let mut resolve = ProcessRequest::new("cargo", &directory);
    resolve.args = vec!["generate-lockfile".into(), "--offline".into()];
    let resolved = crate::SystemProcessRunner.run(&resolve).unwrap();
    assert_eq!(
        resolved.code,
        0,
        "{}",
        String::from_utf8_lossy(&resolved.stderr)
    );
    let mut reads = BTreeMap::new();
    let input = capture_input(
        &temp.0,
        &hook,
        read(&temp.0, &hook.source()).unwrap().unwrap(),
        &BTreeMap::new(),
        &[],
        &mut reads,
    )
    .unwrap();
    let outputs = build(
        &temp.0.join("no-upstream-workspace"),
        &temp.0,
        &json!({}),
        &[(hook.clone(), input)],
        &crate::SystemProcessRunner,
    )
    .unwrap();
    verify_reads(&temp.0, &reads).unwrap();
    let executable = temp
        .0
        .join(format!("toml-runtime{}", std::env::consts::EXE_SUFFIX));
    fs::write(&executable, &outputs[&temp.0.join(hook.binary())]).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();
    }
    let result = crate::SystemProcessRunner
        .run(&ProcessRequest::new(executable.into_os_string(), &temp.0))
        .unwrap();
    assert_eq!(result.code, 0);
    assert_eq!(String::from_utf8(result.stdout).unwrap().trim(), "42");
}

#[test]
fn project_package_build_script_generated_source_runs_but_external_include_is_rejected() {
    let temp = temporary();
    let hook = hooks(&config()).unwrap().remove(0);
    independent_package(&temp.0, &hook);
    let directory = temp.0.join(".codex/hooks/project_demo");
    fs::write(directory.join("build.rs"), br#"fn main(){let out=std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());std::fs::write(out.join("generated.rs"),"pub fn generated_value()->i32{42}").unwrap();}"#).unwrap();
    fs::write(directory.join("entrypoint.rs"), br#"include!(concat!(env!("OUT_DIR"),"/generated.rs"));pub fn run(_:Vec<String>)->i32{println!("{}",generated_value());0}"#).unwrap();
    let mut reads = BTreeMap::new();
    let input = capture_input(
        &temp.0,
        &hook,
        read(&temp.0, &hook.source()).unwrap().unwrap(),
        &BTreeMap::new(),
        &[],
        &mut reads,
    )
    .unwrap();
    let outputs = build(
        &temp.0.join("absent-workspace"),
        &temp.0,
        &json!({}),
        &[(hook.clone(), input.clone())],
        &crate::SystemProcessRunner,
    )
    .unwrap();
    verify_reads(&temp.0, &reads).unwrap();
    assert!(!temp.0.join(hook.binary()).exists());
    let executable = temp
        .0
        .join(format!("generated-runtime{}", std::env::consts::EXE_SUFFIX));
    fs::write(&executable, &outputs[&temp.0.join(hook.binary())]).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();
    }
    let result = crate::SystemProcessRunner
        .run(&ProcessRequest::new(executable.into_os_string(), &temp.0))
        .unwrap();
    assert_eq!(result.code, 0);
    assert_eq!(String::from_utf8(result.stdout).unwrap().trim(), "42");

    struct ChangeGeneratedAfterSelfTest(std::sync::Mutex<Option<PathBuf>>);
    impl ProcessRunner for ChangeGeneratedAfterSelfTest {
        fn run(&self, request: &ProcessRequest) -> std::io::Result<crate::ProcessOutput> {
            let result = crate::SystemProcessRunner.run(request)?;
            if request.program == "cargo" {
                let position = request
                    .args
                    .iter()
                    .position(|arg| arg == "--target-dir")
                    .unwrap();
                *self.0.lock().unwrap() = Some(PathBuf::from(&request.args[position + 1]));
            } else {
                let output = self.0.lock().unwrap().clone().unwrap();
                let generated = package::generated_sources(
                    &output,
                    &output.join("release/project_demo.d"),
                    &request.cwd,
                )
                .unwrap();
                fs::write(
                    generated.keys().next().unwrap(),
                    b"modified after self-test",
                )?;
            }
            Ok(result)
        }
    }
    let drift = build(
        &temp.0.join("absent-workspace"),
        &temp.0,
        &json!({}),
        &[(hook.clone(), input.clone())],
        &ChangeGeneratedAfterSelfTest(std::sync::Mutex::new(None)),
    )
    .unwrap_err();
    assert!(drift.contains("generated source drifted"), "{drift}");

    let external = temp.0.join("outside.rs");
    fs::write(&external, b"pub fn generated_value()->i32{99}").unwrap();
    let mut escaped = input;
    let source = format!(
        "include!({:?});pub fn run(_:Vec<String>)->i32{{generated_value()}}",
        external.to_str().unwrap()
    )
    .into_bytes();
    escaped.source = source.clone();
    escaped
        .package
        .as_mut()
        .unwrap()
        .insert("entrypoint.rs".into(), source);
    let error = build(
        &temp.0.join("absent-workspace"),
        &temp.0,
        &json!({}),
        &[(hook.clone(), escaped)],
        &crate::SystemProcessRunner,
    )
    .unwrap_err();
    assert!(error.contains("uncaptured dependency"), "{error}");
    assert!(!temp.0.join(hook.binary()).exists());
    verify_reads(&temp.0, &reads).unwrap();
}

#[test]
fn project_package_generated_sources_are_limited_to_this_build_out_directory() {
    let temp = temporary();
    let snapshot = temp.0.join("package");
    let output = temp.0.join("output");
    fs::create_dir_all(&snapshot).unwrap();
    let good = output.join("release/build/demo/out/nested/generated.rs");
    let wrong = output.join("release/build/demo/wrong.rs");
    let neighbor = temp
        .0
        .join("output-neighbor/release/build/demo/out/external.rs");
    for path in [&good, &wrong, &neighbor] {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, b"source").unwrap();
    }
    let depfile = temp.0.join("project_demo.d");
    for (path, accepted) in [(&good, true), (&wrong, false), (&neighbor, false)] {
        fs::write(
            &depfile,
            format!("binary: {}\n", path.to_string_lossy().replace(' ', "\\ ")),
        )
        .unwrap();
        let generated = package::generated_sources(&output, &depfile, &snapshot).unwrap();
        assert_eq!(generated.len(), usize::from(accepted));
        assert_eq!(
            verify_dependencies_with_generated(&depfile, &snapshot, &BTreeMap::new(), &generated)
                .is_ok(),
            accepted
        );
        assert!(
            verify_dependencies(&depfile, &snapshot, &BTreeMap::new()).is_err(),
            "legacy mode must not accept generated sources"
        );
    }
}

#[cfg(windows)]
#[test]
fn project_package_generated_sources_reject_out_directory_junctions() {
    let temp = temporary();
    let output = temp.0.join("output");
    let external = temp.0.join("external");
    let link = output.join("release/build/demo/out");
    fs::create_dir_all(link.parent().unwrap()).unwrap();
    fs::create_dir_all(&external).unwrap();
    fs::write(external.join("generated.rs"), b"external source").unwrap();
    let mut request = ProcessRequest::new("cmd.exe", &temp.0);
    request.args = vec![
        "/c".into(),
        "mklink".into(),
        "/J".into(),
        link.to_string_lossy().replace('/', "\\").into(),
        external.to_string_lossy().replace('/', "\\").into(),
    ];
    let result = crate::SystemProcessRunner.run(&request).unwrap();
    assert_eq!(
        result.code,
        0,
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let depfile = temp.0.join("project_demo.d");
    fs::write(
        &depfile,
        format!(
            "binary: {}\n",
            link.join("generated.rs")
                .to_string_lossy()
                .replace(' ', "\\ ")
        ),
    )
    .unwrap();
    let result = package::generated_sources(&output, &depfile, &temp.0);
    fs::remove_dir(&link).unwrap();
    assert!(result.unwrap_err().contains("link"));
    assert_eq!(
        fs::read(external.join("generated.rs")).unwrap(),
        b"external source"
    );
}
