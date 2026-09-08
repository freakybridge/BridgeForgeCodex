use super::*;

#[derive(Clone, Debug)]
pub(crate) struct Input {
    pub source: Vec<u8>,
    pub package: Option<BTreeMap<String, Vec<u8>>>,
}

impl Input {
    pub fn from_files(source: Vec<u8>, files: BTreeMap<String, Vec<u8>>) -> Result<Self, String> {
        if files.contains_key("Cargo.toml") {
            validate(&files)?;
            Ok(Self {
                source,
                package: Some(files),
            })
        } else {
            Ok(Self {
                source,
                package: None,
            })
        }
    }
    pub fn identity(&self, hook: &Hook, contract: &Value) -> Result<String, String> {
        match &self.package {
            None => super::identity(hook, &self.source, contract),
            Some(files) => crate::manifest::canonical_sha(&json!({
                "schema_version": 1, "hook": hook, "wrapper_version": 1,
                "build_mode": "project-package-v2",
                "files": files.iter().map(|(path, bytes)| (path, sha(bytes))).collect::<BTreeMap<_, _>>(),
                "build": crate::manifest::generated_build_recipe(&format!("project_{}", hook.id))
            })),
        }
    }
}

fn inventory(root: &Path, prefix: &str) -> Result<BTreeMap<String, Vec<u8>>, String> {
    fn visit(
        root: &Path,
        directory: &str,
        files: &mut BTreeMap<String, Vec<u8>>,
    ) -> Result<(), String> {
        if crate::memory::is_link_or_reparse(&root.join(directory)).map_err(|e| e.to_string())? {
            return Err(format!(
                "project hook package traverses a link: {directory}"
            ));
        }
        for entry in fs::read_dir(root.join(directory)).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| "non-UTF-8 project hook path")?;
            if matches!(name.as_str(), "target" | ".git") {
                continue;
            }
            if reserved_package_component(&name) {
                return Err(format!("reserved project hook package path: {name}"));
            }
            let relative = format!("{directory}/{name}");
            if crate::memory::is_link_or_reparse(&entry.path()).map_err(|e| e.to_string())? {
                return Err(format!("project hook package traverses a link: {relative}"));
            }
            if entry.file_type().map_err(|e| e.to_string())?.is_dir() {
                visit(root, &relative, files)?;
            } else {
                files.insert(
                    relative.clone(),
                    read(root, &relative)?.ok_or("project hook file disappeared")?,
                );
            }
        }
        Ok(())
    }
    let mut files = BTreeMap::new();
    visit(root, prefix, &mut files)?;
    Ok(files)
}

pub(crate) fn capture_input(
    root: &Path,
    hook: &Hook,
    source: Vec<u8>,
    writes: &BTreeMap<PathBuf, Vec<u8>>,
    deletes: &[PathBuf],
    reads: &mut BTreeMap<String, Option<Vec<u8>>>,
) -> Result<Input, String> {
    let prefix = format!(".codex/hooks/project_{}", hook.id);
    let manifest = format!("{prefix}/Cargo.toml");
    let original = read(root, &manifest)?;
    reads.insert(manifest.clone(), original.clone());
    let effective = writes.get(&root.join(&manifest)).cloned().or(original);
    if effective.is_none() {
        return Ok(Input {
            source,
            package: None,
        });
    }
    // Capture original inventory even for a prospective migration. It is checked
    // again before/after building, including newly added files.
    let original_files = if root.join(&prefix).exists() {
        inventory(root, &prefix)?
    } else {
        BTreeMap::new()
    };
    for (path, payload) in &original_files {
        reads.insert(path.clone(), Some(payload.clone()));
    }
    let mut files = original_files;
    for (path, payload) in writes {
        if let Ok(relative) = path.strip_prefix(root.join(&prefix)) {
            let relative = relative
                .to_str()
                .ok_or("non-UTF-8 project hook path")?
                .replace('\\', "/");
            let key = format!("{prefix}/{relative}");
            if !reads.contains_key(&key) {
                reads.insert(key.clone(), read(root, &key)?);
            }
            files.insert(key, payload.clone());
        }
    }
    for path in deletes {
        if path.starts_with(root.join(&prefix)) {
            return Err("registered project hook package input is scheduled for deletion".into());
        }
    }
    files.insert(hook.source(), source.clone());
    let files = files
        .into_iter()
        .map(|(path, payload)| (path[prefix.len() + 1..].to_string(), payload))
        .collect::<BTreeMap<_, _>>();
    Input::from_files(source, files)
}

pub(crate) fn verify_reads(
    root: &Path,
    reads: &BTreeMap<String, Option<Vec<u8>>>,
) -> Result<(), String> {
    for (relative, expected) in reads {
        if read(root, relative)? != *expected {
            return Err(format!(
                "project Rust hook input changed after plan: {relative}"
            ));
        }
        if relative.ends_with("/Cargo.toml") {
            let prefix = relative.trim_end_matches("/Cargo.toml");
            if !prefix.starts_with(".codex/hooks/project_") || prefix.matches('/').count() != 2 {
                continue;
            }
            if expected.is_none() && !reads.contains_key(&format!("{prefix}/Cargo.lock")) {
                continue;
            }
            let expected_files = reads
                .iter()
                .filter(|(p, _)| p.starts_with(&format!("{prefix}/")))
                .filter_map(|(p, b)| b.as_ref().map(|b| (p.clone(), b.clone())))
                .collect();
            let actual_files = if root.join(prefix).exists() {
                inventory(root, prefix)?
            } else {
                BTreeMap::new()
            };
            if actual_files != expected_files {
                return Err(format!(
                    "project hook package inventory changed after plan: {prefix}"
                ));
            }
        }
    }
    Ok(())
}

fn validate(files: &BTreeMap<String, Vec<u8>>) -> Result<(), String> {
    for path in files.keys() {
        if path.split('/').any(reserved_package_component) {
            return Err(format!("reserved project hook package path: {path}"));
        }
    }
    if !files.contains_key("Cargo.lock") {
        return Err(
            "project hook package requires Cargo.lock; resolve dependencies in the project first"
                .into(),
        );
    }
    if !files.contains_key("entrypoint.rs") {
        return Err("project hook package requires entrypoint.rs".into());
    }
    for (path, bytes) in files.iter().filter(|(p, _)| {
        Path::new(p)
            .file_name()
            .is_some_and(|name| name == "Cargo.toml")
    }) {
        let doc = std::str::from_utf8(bytes)
            .map_err(|e| e.to_string())?
            .parse::<toml_edit::DocumentMut>()
            .map_err(|e| format!("invalid {path}: {e}"))?;
        if path == "Cargo.toml" && doc.get("package").is_none() {
            return Err("project hook Cargo.toml requires a package".into());
        }
        // Every local Cargo path must remain in the captured package. Cargo.lock
        // supplies registry/git dependency identity; no upstream allowlist applies.
        fn paths(value: &toml_edit::Item, base: &Path) -> Result<(), String> {
            if let Some(table) = value.as_table_like() {
                for (key, item) in table.iter() {
                    if matches!(key, "path" | "build") {
                        if let Some(path) = item.as_str() {
                            let candidate = Path::new(path);
                            let mut depth = base.components().count() as i32;
                            for component in candidate.components() {
                                match component {
                                    std::path::Component::Normal(_) => depth += 1,
                                    std::path::Component::CurDir => (),
                                    std::path::Component::ParentDir => {
                                        depth -= 1;
                                        if depth < 0 {
                                            return Err(
                                                "project hook Cargo path escapes package".into()
                                            );
                                        }
                                    }
                                    _ => {
                                        return Err(
                                            "project hook Cargo path must be relative".into()
                                        );
                                    }
                                }
                            }
                        }
                    }
                    paths(item, base)?;
                }
            }
            if let Some(tables) = value.as_array_of_tables() {
                for table in tables {
                    paths(&toml_edit::Item::Table(table.clone()), base)?;
                }
            }
            Ok(())
        }
        paths(doc.as_item(), Path::new(path).parent().unwrap())?;
    }
    Ok(())
}

// Only files named by Cargo's dependency receipt, under this build's fresh
// release/build/<package>/out directory, may supplement the captured sources.
pub(super) fn generated_sources(
    output: &Path,
    depfile: &Path,
    snapshot: &Path,
) -> Result<BTreeMap<PathBuf, Vec<u8>>, String> {
    use std::path::Component;
    let mut generated = BTreeMap::new();
    for path in super::dependency_paths(depfile, snapshot)? {
        let Ok(relative) = path.strip_prefix(output) else {
            continue;
        };
        let components = relative.components().collect::<Vec<_>>();
        if components.len() < 5
            || components
                .iter()
                .any(|part| !matches!(part, Component::Normal(_)))
            || components[0].as_os_str() != "release"
            || components[1].as_os_str() != "build"
            || components[3].as_os_str() != "out"
        {
            continue;
        }
        let relative = relative
            .to_str()
            .ok_or("non-UTF-8 generated source path")?
            .replace('\\', "/");
        // read checks every ancestor for symlinks/reparse points, including output.
        let bytes = read(output, &relative)?.ok_or("generated project hook source is missing")?;
        generated.insert(path, bytes);
    }
    Ok(generated)
}

pub(super) fn verify_generated_sources(
    output: &Path,
    generated: &BTreeMap<PathBuf, Vec<u8>>,
) -> Result<(), String> {
    for (path, bytes) in generated {
        let relative = path
            .strip_prefix(output)
            .map_err(|_| "generated source escaped output")?
            .to_str()
            .ok_or("non-UTF-8 generated source path")?
            .replace('\\', "/");
        if read(output, &relative)?.as_ref() != Some(bytes) {
            return Err(format!(
                "project hook generated source drifted: {}",
                path.display()
            ));
        }
    }
    Ok(())
}

pub(super) fn build(
    root: &Path,
    hook: &Hook,
    input: &Input,
    files: &BTreeMap<String, Vec<u8>>,
    contract: &Value,
    runner: &dyn ProcessRunner,
) -> Result<BTreeMap<PathBuf, Vec<u8>>, String> {
    validate(files)?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let temporary = Temporary(std::env::temp_dir().join(format!(
        "bridgeforge-project-package-{}-{nonce}",
        std::process::id()
    )));
    let snapshot = temporary.0.join("package");
    fs::create_dir_all(&snapshot).map_err(|e| e.to_string())?;
    let fingerprint = input.identity(hook, contract)?;
    let response = json!({"id":hook.id,"input_sha256":fingerprint,"status":"ok"});
    let mut expected = files.clone();
    let mut manifest = std::str::from_utf8(&files["Cargo.toml"])
        .map_err(|e| e.to_string())?
        .parse::<toml_edit::DocumentMut>()
        .map_err(|e| e.to_string())?;
    manifest
        .entry("workspace")
        .or_insert(toml_edit::Item::Table(toml_edit::Table::new()));
    manifest["package"]["autobins"] = toml_edit::value(false);
    let mut bin = toml_edit::Table::new();
    bin["name"] = toml_edit::value(format!("project_{}", hook.id));
    bin["path"] = toml_edit::value(".bridgeforge-main.rs");
    let mut bins = toml_edit::ArrayOfTables::new();
    bins.push(bin);
    manifest["bin"] = toml_edit::Item::ArrayOfTables(bins);
    expected.insert("Cargo.toml".into(), manifest.to_string().into_bytes());
    expected.insert(".bridgeforge-main.rs".into(), format!(
        "#![cfg_attr(windows, windows_subsystem = \"windows\")]\nmod entrypoint;\nfn main() {{ let args: Vec<String> = std::env::args().skip(1).collect(); if args == [\"--bridgeforge-self-test\"] {{ println!(\"{{}}\", {:?}); return; }} std::process::exit(entrypoint::run(args)); }}\n", response.to_string()).into_bytes());
    for (path, bytes) in &expected {
        let target = snapshot.join(path);
        fs::create_dir_all(target.parent().unwrap()).map_err(|e| e.to_string())?;
        fs::write(target, bytes).map_err(|e| e.to_string())?;
    }
    let verify_snapshot = || -> Result<(), String> {
        for (path, bytes) in &expected {
            if read(&snapshot, path)?.as_ref() != Some(bytes) {
                return Err(format!("project hook build input drifted: {path}"));
            }
        }
        Ok(())
    };
    let output = temporary.0.join("output");
    let mut request = ProcessRequest::new("cargo", &snapshot);
    request.args = vec![
        "build".into(),
        "--locked".into(),
        "--profile".into(),
        "release".into(),
        "--manifest-path".into(),
        snapshot.join("Cargo.toml").into_os_string(),
        "--target-dir".into(),
        output.clone().into_os_string(),
        "--bin".into(),
        format!("project_{}", hook.id).into(),
    ];
    request.timeout = Duration::from_secs(900);
    let built = runner.run(&request).map_err(|e| e.to_string())?;
    if built.timed_out || built.code != 0 {
        return Err(format!(
            "project Rust hook build failed: {}",
            String::from_utf8_lossy(&built.stderr)
        ));
    }
    verify_snapshot()?;
    let depfile = output
        .join("release")
        .join(format!("project_{}.d", hook.id));
    let dependency_receipt = fs::read(&depfile).map_err(|e| e.to_string())?;
    let generated = generated_sources(&output, &depfile, &snapshot)?;
    super::verify_dependencies_with_generated(&depfile, &snapshot, &expected, &generated)?;
    let binary = output.join("release").join(format!(
        "project_{}{}",
        hook.id,
        std::env::consts::EXE_SUFFIX
    ));
    let bytes = fs::read(&binary).map_err(|e| e.to_string())?;
    if cfg!(windows) {
        verify_windows_gui(&bytes)?;
    }
    let mut test = ProcessRequest::new(binary.clone().into_os_string(), &snapshot);
    test.args = vec!["--bridgeforge-self-test".into()];
    test.timeout = Duration::from_secs(30);
    let result = runner.run(&test).map_err(|e| e.to_string())?;
    if result.timed_out
        || result.code != 0
        || crate::baseline::parse_unique_json(&result.stdout, "project hook self-test")? != response
        || fs::read(binary).map_err(|e| e.to_string())? != bytes
    {
        return Err(format!("project Rust hook self-test failed: {}", hook.id));
    }
    verify_snapshot()?;
    verify_generated_sources(&output, &generated)?;
    if fs::read(&depfile).map_err(|e| e.to_string())? != dependency_receipt {
        return Err("project hook dependency receipt drifted during self-test".into());
    }
    let receipt = json!({"schema_version":1,"id":hook.id,"input_sha256":fingerprint,
        "platform":std::env::consts::OS,"binary_sha256":sha(&bytes)});
    Ok(BTreeMap::from([
        (root.join(hook.binary()), bytes),
        (
            root.join(hook.receipt()),
            serde_json::to_vec_pretty(&receipt).map_err(|e| e.to_string())?,
        ),
    ]))
}
