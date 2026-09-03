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
    assert_eq!(render(&rendered).unwrap(), rendered);
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
        let result = build(
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
