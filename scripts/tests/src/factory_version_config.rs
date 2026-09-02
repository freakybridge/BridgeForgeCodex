use bridgeforge_core::release::{apply_file_release_plan, build_file_release_plan};
use bridgeforge_core::{ProcessOutput, ProcessRequest, ProcessRunner};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

struct Temp(PathBuf);
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
struct NoProcesses;
impl ProcessRunner for NoProcesses {
    fn run(&self, _: &ProcessRequest) -> std::io::Result<ProcessOutput> {
        panic!("factory version planning must not run external commands")
    }
}
fn factory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf()
}

#[test]
fn factory_release_updates_all_three_cargo_manifests_and_locks() {
    let source = factory();
    let config = fs::read(source.join(".codex/bridgeforge-version.json")).unwrap();
    let document: Value = serde_json::from_slice(&config).unwrap();
    let paths = document["manifests"].as_array().unwrap();
    assert_eq!(
        paths,
        &vec![
            Value::from("templates/hooks/Cargo.toml"),
            Value::from(".codex/hooks/Cargo.toml"),
            Value::from("scripts/tests/Cargo.toml"),
        ]
    );
    let root = std::env::temp_dir().join(format!(
        "bf-factory-version-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let temp = Temp(root);
    fs::create_dir_all(temp.0.join(".codex")).unwrap();
    fs::create_dir_all(temp.0.join("templates")).unwrap();
    fs::write(temp.0.join(".codex/bridgeforge-version.json"), config).unwrap();
    fs::write(temp.0.join("templates/managed-skeleton.json"), b"{}").unwrap();
    for name in ["VERSION", "CHANGELOG.md"] {
        fs::copy(source.join(name), temp.0.join(name)).unwrap();
    }
    for path in paths {
        let relative = Path::new(path.as_str().unwrap());
        fs::create_dir_all(temp.0.join(relative).parent().unwrap()).unwrap();
        for name in ["Cargo.toml", "Cargo.lock"] {
            fs::copy(
                source.join(relative).with_file_name(name),
                temp.0.join(relative).with_file_name(name),
            )
            .unwrap();
        }
    }
    let old_version = fs::read(temp.0.join("VERSION")).unwrap();
    let plan = build_file_release_plan(
        &temp.0,
        "fix: exercise factory version sync",
        vec!["templates/hooks/src/lib.rs".into()],
        &NoProcesses,
    )
    .unwrap()
    .unwrap();
    assert_eq!(
        fs::read(temp.0.join("VERSION")).unwrap(),
        old_version,
        "planning must not write"
    );
    let next = plan.new_version.to_string();
    for path in paths {
        let target = temp.0.join(path.as_str().unwrap());
        let payload = String::from_utf8(plan.writes[&target].clone()).unwrap();
        assert!(
            payload.contains(&format!("version = \"{next}\"")),
            "{}",
            target.display()
        );
        let lock = target.with_file_name("Cargo.lock");
        assert!(plan.writes.contains_key(&lock), "{}", lock.display());
        if path == "scripts/tests/Cargo.toml" {
            let payload = String::from_utf8(plan.writes[&lock].clone()).unwrap();
            let package = payload
                .split("[[package]]")
                .find(|part| part.contains("name = \"bridgeforge-factory-tests\""))
                .unwrap();
            assert!(package.contains(&format!("version = \"{next}\"")));
        }
    }
    apply_file_release_plan(&plan).unwrap();
    for (path, payload) in &plan.writes {
        assert_eq!(&fs::read(path).unwrap(), payload);
    }
}

#[test]
fn obsolete_venv_contract_is_not_an_active_runtime_instruction() {
    let source = factory();
    let old = fs::read_to_string(source.join("doc/1_delivery/project-venv-hook-runtime-single-rule/requirements_2026-08-19_project-venv-hook-runtime-single-rule.md")).unwrap();
    assert!(
        old.starts_with("---\nlifecycle: superseded\n")
            || old.starts_with("---\r\nlifecycle: superseded\r\n")
    );
    assert!(old.contains("superseded_by: ../rust-only-bridgeforge/"));
    assert!(!old.contains("其他 Python 流程仍继续遵守本卡"));
}
