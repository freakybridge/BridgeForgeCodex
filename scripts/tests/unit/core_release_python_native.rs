//! Native producer/freshness oracles. Deliberately ignored in ordinary offline
//! test runs; run explicitly with fixed tool environments. No install/sync/build.
use super::{model::Writes, python};
use serde_json::json;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

struct Fixture(PathBuf);
impl Fixture {
    fn new(kind: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "bridgeforge-python-native-{kind}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
    fn write(&self, path: &str, value: impl AsRef<[u8]>) {
        let target = self.0.join(path);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(target, value).unwrap();
    }
    fn paths(&self) -> Vec<String> {
        walkdir::WalkDir::new(&self.0)
            .into_iter()
            .map(Result::unwrap)
            .filter(|entry| entry.file_type().is_file())
            .map(|entry| {
                entry
                    .path()
                    .strip_prefix(&self.0)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect()
    }
    fn no_build_artifacts(&self) {
        for entry in walkdir::WalkDir::new(&self.0) {
            let entry = entry.unwrap();
            let name = entry.file_name().to_string_lossy();
            assert!(
                !matches!(name.as_ref(), "build" | "dist" | ".venv")
                    && !name.ends_with(".egg-info"),
                "Native lock oracle unexpectedly built/installed: {}",
                entry.path().display()
            );
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        // Constrain recursive cleanup to the exact generated direct temp child.
        if let (Ok(actual), Ok(temp)) = (self.0.canonicalize(), std::env::temp_dir().canonicalize())
        {
            if actual.parent() == Some(temp.as_path())
                && actual
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("bridgeforge-python-native-")
            {
                let _ = fs::remove_dir_all(actual);
            }
        }
    }
}

fn tool(variable: &str, name: &str) -> PathBuf {
    let scripts = std::env::var_os(variable).unwrap_or_else(|| {
        panic!("Native oracle requires {variable}; this test is intentionally ignored by default")
    });
    let executable = PathBuf::from(scripts).join(if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    });
    assert!(
        executable.is_file(),
        "Missing fixed native oracle tool: {}",
        executable.display()
    );
    executable
}

fn run(root: &Fixture, program: &Path, args: &[&str], scripts: &Path) -> Output {
    let mut process = Command::new(program);
    process
        .args(args)
        .current_dir(&root.0)
        .env("UV_CACHE_DIR", root.0.join(".oracle-cache/uv"))
        .env("UV_PYTHON_DOWNLOADS", "never")
        .env("POETRY_CACHE_DIR", root.0.join(".oracle-cache/poetry"))
        .env("POETRY_CONFIG_DIR", root.0.join(".oracle-config/poetry"))
        .env("POETRY_VIRTUALENVS_CREATE", "false")
        .env("PDM_CACHE_DIR", root.0.join(".oracle-cache/pdm"))
        .env("PDM_LOG_DIR", root.0.join(".oracle-logs/pdm"))
        .env("PDM_CONFIG_FILE", root.0.join(".oracle-config/pdm.toml"))
        .env("PDM_CHECK_UPDATE", "false")
        .env("PDM_IGNORE_SAVED_PYTHON", "1")
        .env("PDM_USE_VENV", "false")
        .env(
            "PDM_PYTHON",
            scripts.join(if cfg!(windows) {
                "python.exe"
            } else {
                "python"
            }),
        )
        .env("PIP_NO_INDEX", "1")
        .env("PYTHONDONTWRITEBYTECODE", "1");
    let output = process
        .output()
        .unwrap_or_else(|error| panic!("{} {args:?}: {error}", program.display()));
    assert!(
        output.status.success(),
        "{} {args:?}\nstdout: {}\nstderr: {}",
        program.display(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn adapter(root: &Fixture, format: &str, version: &str) -> python::Config {
    serde_json::from_value(json!({
        "projects":["pyproject.toml","dep/pyproject.toml"],
        "local_sources":{"pyproject.toml":{"bf-native-dep":"dep/pyproject.toml"}},
        "locks":[{"path":format!("{format}.lock"),"format":format,"members":["pyproject.toml","dep/pyproject.toml"],"tool_version":version}]
    })).unwrap_or_else(|error|panic!("{}: {error}",root.0.display()))
}

fn render_and_apply(root: &Fixture, cfg: &python::Config) -> Writes {
    let paths = root.paths();
    let read = |path: &str| {
        fs::read(root.0.join(path))
            .map(Some)
            .map_err(|error| error.to_string())
    };
    let diagnostic = |error: String| -> ! {
        let locks = cfg
            .locks
            .iter()
            .map(|lock| {
                format!(
                    "{}:\n{}",
                    lock.path,
                    fs::read_to_string(root.0.join(&lock.path)).unwrap()
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        panic!("Native producer/adapter mismatch: {error}\n{locks}");
    };
    let before = python::snapshot(cfg, &paths, &mut |path| read(path))
        .unwrap_or_else(|error| diagnostic(error));
    assert_eq!(before.packages.len(), 2);
    assert_eq!(before.dependencies.len(), 1);
    let versions = BTreeMap::from([
        ("pyproject.toml".into(), "1.0.1".into()),
        ("dep/pyproject.toml".into(), "1.1.0".into()),
    ]);
    let writes = python::render_versions(cfg, &paths, &mut |path| read(path), &versions)
        .unwrap_or_else(|error| diagnostic(error));
    assert!(writes.contains_key("pyproject.toml"));
    assert!(writes.contains_key("dep/pyproject.toml"));
    for (path, bytes) in &writes {
        root.write(path, bytes);
    }
    let after = python::snapshot(cfg, &root.paths(), &mut |path| read(path)).unwrap();
    assert_eq!(after.packages["pyproject.toml"].version, "1.0.1");
    assert_eq!(after.packages["dep/pyproject.toml"].version, "1.1.0");
    writes
}

#[test]
#[ignore = "requires explicitly provisioned uv 0.9.8; run with BRIDGEFORGE_PYTHON_LOCK_TOOLS"]
fn python_native_uv_real_lock_accepts_adapter_versions_without_relocking() {
    // https://docs.astral.sh/uv/reference/cli/#uv-lock
    let executable = tool("BRIDGEFORGE_PYTHON_LOCK_TOOLS", "uv");
    let scripts = executable.parent().unwrap();
    let root = Fixture::new("uv");
    let output = run(&root, &executable, &["--version"], scripts);
    assert!(String::from_utf8_lossy(&output.stdout).starts_with("uv 0.9.8"));
    root.write("pyproject.toml",b"[project]\nname='bf-native-app'\nversion='1.0.0'\nrequires-python='>=3.10'\ndependencies=['bf-native-dep==1.0.0']\n[tool.uv]\npackage=false\n[tool.uv.workspace]\nmembers=['dep']\n[tool.uv.sources]\nbf-native-dep={workspace=true}\n");
    root.write("dep/pyproject.toml",b"[project]\nname='bf-native-dep'\nversion='1.0.0'\nrequires-python='>=3.10'\n[tool.uv]\npackage=false\n");
    run(
        &root,
        &executable,
        &["lock", "--offline", "--no-build"],
        scripts,
    );
    let cfg = adapter(&root, "uv", "0.9.8");
    let writes = render_and_apply(&root, &cfg);
    assert!(writes.contains_key("uv.lock"));
    let lock = fs::read(root.0.join("uv.lock")).unwrap();
    run(
        &root,
        &executable,
        &["lock", "--check", "--offline", "--no-build"],
        scripts,
    );
    assert_eq!(lock, fs::read(root.0.join("uv.lock")).unwrap());
    root.no_build_artifacts();
}

#[test]
#[ignore = "requires explicitly provisioned uv 0.9.8; run with BRIDGEFORGE_PYTHON_LOCK_TOOLS"]
fn python_native_uv_real_marker_and_extras_preserve_conditions() {
    let executable = tool("BRIDGEFORGE_PYTHON_LOCK_TOOLS", "uv");
    let scripts = executable.parent().unwrap();
    let root = Fixture::new("uv-conditions");
    let output = run(&root, &executable, &["--version"], scripts);
    assert!(String::from_utf8_lossy(&output.stdout).starts_with("uv 0.9.8"));
    root.write("pyproject.toml",b"[project]\nname='bf-native-app'\nversion='1.0.0'\nrequires-python='>=3.10'\ndependencies=[\"bf-native-dep[fast]==1.0.0 ; sys_platform == 'win32'\"]\n[tool.uv]\npackage=false\n[tool.uv.workspace]\nmembers=['dep']\n[tool.uv.sources]\nbf-native-dep={workspace=true}\n");
    root.write("dep/pyproject.toml",b"[project]\nname='bf-native-dep'\nversion='1.0.0'\nrequires-python='>=3.10'\n[project.optional-dependencies]\nfast=[]\n[tool.uv]\npackage=false\n");
    run(
        &root,
        &executable,
        &["lock", "--offline", "--no-build"],
        scripts,
    );
    let cfg = adapter(&root, "uv", "0.9.8");
    let writes = render_and_apply(&root, &cfg);
    let doc = std::str::from_utf8(&writes["pyproject.toml"])
        .unwrap()
        .parse::<toml_edit::DocumentMut>()
        .unwrap();
    let requirement = doc["project"]["dependencies"]
        .as_array()
        .unwrap()
        .get(0)
        .unwrap()
        .as_str()
        .unwrap();
    assert_eq!(
        super::python_dependencies::requirement_identity(requirement).unwrap(),
        super::python_dependencies::requirement_identity(
            "bf-native-dep[fast]==1.1.0 ; sys_platform == 'win32'"
        )
        .unwrap()
    );
    assert!(requirement.contains("==1.1.0"));
    let lock = fs::read(root.0.join("uv.lock")).unwrap();
    run(
        &root,
        &executable,
        &["lock", "--check", "--offline", "--no-build"],
        scripts,
    );
    assert_eq!(lock, fs::read(root.0.join("uv.lock")).unwrap());
    root.no_build_artifacts();
}

#[test]
#[ignore = "requires explicitly provisioned Poetry 2.1.4; run with BRIDGEFORGE_PYTHON_LOCK_TOOLS"]
fn python_native_poetry_real_lock_accepts_adapter_versions_and_content_hash() {
    // https://python-poetry.org/docs/cli/#check
    let executable = tool("BRIDGEFORGE_PYTHON_LOCK_TOOLS", "poetry");
    let scripts = executable.parent().unwrap();
    let root = Fixture::new("poetry");
    let output = run(&root, &executable, &["--version"], scripts);
    assert!(String::from_utf8_lossy(&output.stdout).contains("2.1.4"));
    root.write("pyproject.toml",b"[project]\nname='bf-native-app'\nversion='1.0.0'\ndescription='Native lock oracle'\nrequires-python='>=3.10,<4.0'\ndependencies=['bf-native-dep==1.0.0']\n[tool.poetry]\npackage-mode=false\n[tool.poetry.dependencies]\nbf-native-dep={path='dep'}\n");
    root.write("dep/pyproject.toml",b"[tool.poetry]\nname='bf-native-dep'\nversion='1.0.0'\ndescription='Static dependency metadata'\nauthors=[]\n[tool.poetry.dependencies]\npython='>=3.10,<4.0'\n");
    run(&root, &executable, &["lock", "--no-interaction"], scripts);
    let cfg = adapter(&root, "poetry", "2.1.4");
    let writes = render_and_apply(&root, &cfg);
    assert!(writes.contains_key("poetry.lock"));
    let manifest = fs::read_to_string(root.0.join("pyproject.toml"))
        .unwrap()
        .parse::<toml_edit::DocumentMut>()
        .unwrap();
    assert_eq!(
        manifest["project"]["dependencies"]
            .as_array()
            .unwrap()
            .get(0)
            .unwrap()
            .as_str(),
        Some("bf-native-dep==1.1.0")
    );
    assert_eq!(
        manifest["tool"]["poetry"]["dependencies"]["bf-native-dep"]["path"].as_str(),
        Some("dep")
    );
    assert!(
        manifest["tool"]["poetry"]["dependencies"]["bf-native-dep"]
            .get("version")
            .is_none()
    );
    let lock = fs::read(root.0.join("poetry.lock")).unwrap();
    run(
        &root,
        &executable,
        &["check", "--lock", "--no-interaction"],
        scripts,
    );
    assert_eq!(lock, fs::read(root.0.join("poetry.lock")).unwrap());
    root.no_build_artifacts();
}

#[test]
#[ignore = "requires explicitly provisioned PDM 2.25.9; run with BRIDGEFORGE_PDM_LOCK_TOOLS"]
fn python_native_pdm_real_lock_accepts_adapter_versions_and_content_hash() {
    // https://pdm-project.org/en/stable/usage/lockfile/#check-the-lock-file
    let executable = tool("BRIDGEFORGE_PDM_LOCK_TOOLS", "pdm");
    let scripts = executable.parent().unwrap();
    let root = Fixture::new("pdm");
    let output = run(&root, &executable, &["--version"], scripts);
    assert!(String::from_utf8_lossy(&output.stdout).contains("2.25.9"));
    root.write("pyproject.toml",b"[project]\nname='bf-native-app'\nversion='1.0.0'\nrequires-python='>=3.10'\ndependencies=['bf-native-dep @ file:///${PROJECT_ROOT}/dep']\n[tool.pdm]\ndistribution=false\n");
    root.write("dep/pyproject.toml",b"[project]\nname='bf-native-dep'\nversion='1.0.0'\nrequires-python='>=3.10'\n[tool.pdm]\ndistribution=false\n");
    run(&root, &executable, &["lock"], scripts);
    let cfg = adapter(&root, "pdm", "2.25.9");
    let writes = render_and_apply(&root, &cfg);
    assert!(writes.contains_key("pdm.lock"));
    let lock = fs::read(root.0.join("pdm.lock")).unwrap();
    run(&root, &executable, &["lock", "--check"], scripts);
    assert_eq!(lock, fs::read(root.0.join("pdm.lock")).unwrap());
    root.no_build_artifacts();
}
