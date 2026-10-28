//! Test-only Agent side of the public schema 4 contract. Claims are explicit,
//! derived from each fixture's intended change; this does not run in the CLI.
use bridgeforge_core::{ProcessRequest, ProcessRunner, SystemProcessRunner};
use serde_json::{Value, json};
use std::{fs, path::Path, time::Duration};

#[path = "../support/current_fixture.rs"]
mod current_fixture;

pub(super) fn invoke(cli: &Path, root: &Path, args: &[&str], env: &[(&str, &str)]) -> Value {
    let mut request = ProcessRequest::new(cli.as_os_str(), root);
    request.args = args.iter().map(Into::into).collect();
    request.timeout = Duration::from_secs(120);
    for (key, value) in env {
        request.env.insert((*key).into(), (*value).into());
    }
    let output = SystemProcessRunner.run(&request).unwrap();
    assert!(
        !output.timed_out && output.code == 0,
        "{args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
pub(super) fn migrate(cli: &Path, root: &Path) {
    let proposal = current_fixture::proposal(root).unwrap();
    let mut args = vec!["git-sync", "--migration-plan"];
    if let Some(path) = &proposal {
        args.extend(["--setup-config-file", path.to_str().unwrap()]);
    }
    let p = invoke(cli, root, &args, &[]);
    let mut args = vec![
        "git-sync",
        "--migration-apply",
        "--migration-fingerprint",
        p["fingerprint"].as_str().unwrap(),
        "--confirmed-choice",
    ];
    if let Some(path) = &proposal {
        args.extend(["--setup-config-file", path.to_str().unwrap()]);
    }
    invoke(cli, root, &args, &[]);
}
pub(super) fn analyze(
    cli: &Path,
    root: &Path,
    message: &str,
    level: &str,
    additional: &[(&str, &str)],
    env: &[(&str, &str)],
) {
    close_packages(cli, root, level, additional, env);
    let p = invoke(
        cli,
        root,
        &["git-sync", "--release-preview", "--message", message],
        env,
    );
    assert_eq!(p["status"], "analysis-required");
    let value = json!({"schema":1,"fingerprint":p["fingerprint"],"product":{"level":level,"reason":"fixture declares the actual program change","compatibility":"compatible"}});
    fs::create_dir_all(root.join(".runtime")).unwrap();
    fs::write(
        root.join(".runtime/schema4-analysis.json"),
        serde_json::to_vec(&value).unwrap(),
    )
    .unwrap();
}

fn close_packages(
    cli: &Path,
    root: &Path,
    level: &str,
    additional: &[(&str, &str)],
    env: &[(&str, &str)],
) {
    let first = invoke(cli, root, &["develop", "--preview"], env);
    if first["status"] == "nothing-to-close" {
        return;
    }
    let path = root.join(".codex/bridgeforge-version.json");
    let mut cfg: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    let mut policies = serde_json::Map::new();
    for p in cfg["packages"].as_array().unwrap() {
        let id = p["id"].as_str().unwrap();
        let manifest = p["manifest"].as_str().unwrap();
        let check = if p["language"] == "rust" {
            json!({"id":"native-test","program":"cargo","args":["test","--locked","--manifest-path",manifest,"--target-dir",".runtime/native-target"],"timeout_seconds":120})
        } else {
            // Real Python executes the changed module; no package installation.
            json!({"id":"native-test","program":"python","args":["-B","-S","-c","exec(compile(open('app.py',encoding='utf-8').read(),'app.py','exec')); assert VALUE == 2"],"timeout_seconds":30})
        };
        policies.insert(id.into(), json!({"checks":[check]}));
    }
    if cfg["adapters"].get("python").is_some() && cfg.get("python_environment").is_none() {
        let mut command = ProcessRequest::new("python", root);
        command.args = [
            "-B",
            "-S",
            "-c",
            "import sys,encodings; print(sys.executable); print(encodings.__path__[0])",
        ]
        .iter()
        .map(Into::into)
        .collect();
        let output = SystemProcessRunner.run(&command).unwrap();
        assert_eq!(output.code, 0);
        let text = String::from_utf8(output.stdout).unwrap();
        let mut lines = text.lines();
        let interpreter = lines.next().unwrap();
        let runtime = lines.next().unwrap();
        fs::create_dir_all(root.join(".runtime/python-dependencies")).unwrap();
        cfg["python_environment"] = json!({"interpreter":interpreter,"runtime_roots":[runtime],"dependency_roots":[".runtime/python-dependencies"]});
    }
    cfg["development"] = json!(policies);
    fs::write(path, serde_json::to_vec(&cfg).unwrap()).unwrap();
    let p = invoke(cli, root, &["develop", "--preview"], env);
    let mut packages = serde_json::Map::new();
    let mut validation = serde_json::Map::new();
    for id in p["affected_packages"].as_array().unwrap() {
        validation.insert(
            id.as_str().unwrap().into(),
            cfg["development"][id.as_str().unwrap()]["checks"].clone(),
        );
        packages.insert(id.as_str().unwrap().into(),json!({"level":level,"reason":"fixture declares the actual package change","compatibility":"compatible"}));
    }
    for (id, level) in additional {
        packages.insert((*id).into(),json!({"level":level,"reason":"fixture effective dependency requirement changes","compatibility":"compatible"}));
    }
    fs::create_dir_all(root.join(".runtime")).unwrap();
    fs::write(
        root.join(".runtime/develop-analysis.json"),
        serde_json::to_vec(&json!({"schema":1,"fingerprint":p["fingerprint"],"packages":packages,"validation":validation}))
            .unwrap(),
    )
    .unwrap();
    let result = invoke(
        cli,
        root,
        &[
            "develop",
            "--apply",
            "--analysis-file",
            ".runtime/develop-analysis.json",
            "--fingerprint",
            p["fingerprint"].as_str().unwrap(),
        ],
        env,
    );
    assert_eq!(result["status"], "development-closed");
}
