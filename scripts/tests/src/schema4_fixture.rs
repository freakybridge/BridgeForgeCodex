//! Test-only Agent side of the public schema 4 contract. Claims are explicit,
//! derived from each fixture's intended change; this does not run in the CLI.
use bridgeforge_core::{ProcessRequest, ProcessRunner, SystemProcessRunner};
use serde_json::{Value, json};
use std::{fs, path::Path, time::Duration};

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
    let p = invoke(cli, root, &["git-sync", "--migration-plan"], &[]);
    invoke(
        cli,
        root,
        &[
            "git-sync",
            "--migration-apply",
            "--migration-fingerprint",
            p["fingerprint"].as_str().unwrap(),
            "--confirmed-choice",
        ],
        &[],
    );
}
pub(super) fn analyze(
    cli: &Path,
    root: &Path,
    message: &str,
    level: &str,
    additional: &[(&str, &str)],
    env: &[(&str, &str)],
) {
    let p = invoke(
        cli,
        root,
        &["git-sync", "--release-preview", "--message", message],
        env,
    );
    assert_eq!(p["status"], "analysis-required");
    let mut packages = serde_json::Map::new();
    for id in p["affected_packages"].as_array().unwrap() {
        packages.insert(id.as_str().unwrap().into(),json!({"level":level,"reason":"fixture declares the actual package change","compatibility":"compatible"}));
    }
    for (id, level) in additional {
        packages.insert((*id).into(),json!({"level":level,"reason":"fixture effective dependency requirement changes","compatibility":"compatible"}));
    }
    let value = json!({"schema":1,"fingerprint":p["fingerprint"],"product":{"level":level,"reason":"fixture declares the actual program change","compatibility":"compatible"},"packages":packages});
    fs::create_dir_all(root.join(".runtime")).unwrap();
    fs::write(
        root.join(".runtime/schema4-analysis.json"),
        serde_json::to_vec(&value).unwrap(),
    )
    .unwrap();
}
