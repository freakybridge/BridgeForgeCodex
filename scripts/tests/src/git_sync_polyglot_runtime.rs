//! The real managed CLI, with only a temporary local Git remote.
use bridgeforge_core::{ProcessRequest, ProcessRunner, SystemProcessRunner};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        if let (Ok(path), Ok(temp)) = (self.0.canonicalize(), std::env::temp_dir().canonicalize()) {
            if path.parent() == Some(temp.as_path())
                && path
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("bf-polyglot-cli-")
            {
                let _ = fs::remove_dir_all(path);
            }
        }
    }
}
fn git(root: &Path, args: &[&str]) -> String {
    let mut r = ProcessRequest::new("git", root);
    r.args = args.iter().map(Into::into).collect();
    let o = SystemProcessRunner.run(&r).unwrap();
    assert_eq!(o.code, 0, "{}", String::from_utf8_lossy(&o.stderr));
    String::from_utf8(o.stdout).unwrap().trim().into()
}
fn write(root: &Path, path: &str, bytes: &[u8]) {
    fs::create_dir_all(root.join(path).parent().unwrap()).unwrap();
    fs::write(root.join(path), bytes).unwrap();
}
fn call(cli: &Path, root: &Path, args: &[&str], success: bool) -> Value {
    let mut r = ProcessRequest::new(cli.as_os_str(), root);
    r.args = args.iter().map(Into::into).collect();
    r.timeout = Duration::from_secs(120);
    let o = SystemProcessRunner.run(&r).unwrap();
    assert!(!o.timed_out);
    if success {
        assert_eq!(o.code, 0, "{}", String::from_utf8_lossy(&o.stderr));
    } else {
        assert_ne!(o.code, 0);
    }
    serde_json::from_slice(&o.stdout)
        .unwrap_or_else(|_| json!({"stderr":String::from_utf8_lossy(&o.stderr)}))
}
#[test]
fn real_cli_python_lock_preview_release_rollback_and_repeat() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap();
    let cli = source.join(if cfg!(windows) {
        ".codex/bin/bridgeforge.exe"
    } else {
        ".codex/bin/bridgeforge"
    });
    let fixture = Fixture(
        std::env::temp_dir().join(format!(
            "bf-polyglot-cli-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )),
    );
    let root = fixture.0.join("project");
    fs::create_dir_all(&root).unwrap();
    write(&root, "VERSION", b"1.0.0\n");
    write(&root, "CHANGELOG.md", b"# Changelog\n");
    write(&root, "app.py", b"VALUE=1\n");
    write(&root, ".gitignore", b".runtime/\n");
    write(&root, "managed.txt", b"managed\n");
    write(&root, ".codex/.bridgeforge_codex_version", b"1.0.0\n");
    write(
        &root,
        "pyproject.toml",
        b"[project]\nname='app'\nversion='1.2.3'\nrequires-python='>=3.10'\n",
    );
    write(&root,"uv.lock",b"version = 1\nrevision = 3\nrequires-python = '>=3.10'\n[[package]]\nname = 'app'\nversion = '1.2.3'\nsource = { editable = '.' }\n");
    let cfg = json!({"schema_version":3,"release_policy":"explicit_release","strategy":"independent","git_sync_setup":{"schema":1,"confirmed":true,"selection":"independent"},"adapters":{"python":{"projects":["pyproject.toml"],"locks":[{"path":"uv.lock","format":"uv","members":["pyproject.toml"]}]}},"packages":[{"id":"app","language":"python","manifest":"pyproject.toml","name":"app"}],"product_inputs":["uv.lock",".gitignore"]});
    write(
        &root,
        ".codex/bridgeforge-version.json",
        &serde_json::to_vec(&cfg).unwrap(),
    );
    use sha2::{Digest, Sha256};
    let contract = json!({"schema_version":4,"release_version":"1.0.0","host":"codex","stamp":".codex/.bridgeforge_codex_version","contract_target":".codex/managed-skeleton.json","assets":[{"id":"managed.asset","source":"templates/managed.txt","target":"managed.txt","strategy":"whole","current_sha256":format!("sha256:{:x}",Sha256::digest(b"managed\n"))}],"generated_assets":[],"baseline_model":"current-only","compatibility_baseline":"1.0.0"});
    write(
        &root,
        ".codex/managed-skeleton.json",
        &serde_json::to_vec(&contract).unwrap(),
    );
    git(&root, &["init"]);
    git(&root, &["config", "user.name", "BridgeForge Test"]);
    git(&root, &["config", "user.email", "test@example.invalid"]);
    git(&root, &["config", "core.hooksPath", ".git/hooks"]);
    git(&root, &["add", "."]);
    git(&root, &["commit", "-m", "chore: baseline"]);
    let remote = fixture.0.join("origin.git");
    git(&root, &["init", "--bare", remote.to_str().unwrap()]);
    git(
        &root,
        &["remote", "add", "origin", remote.to_str().unwrap()],
    );
    git(&root, &["push", "-u", "origin", "HEAD"]);
    write(&root, "app.py", b"VALUE=2\n");
    let status = git(&root, &["status", "--porcelain=v1"]);
    let head = git(&root, &["rev-parse", "HEAD"]);
    let index = fs::read(root.join(".git/index")).unwrap();
    let before = ["VERSION", "pyproject.toml", "uv.lock", "CHANGELOG.md"]
        .into_iter()
        .map(|p| (p, fs::read(root.join(p)).unwrap()))
        .collect::<Vec<_>>();
    let preview = call(
        &cli,
        &root,
        &[
            "git-sync",
            "--release-preview",
            "--message",
            "fix: adjust app",
        ],
        true,
    );
    assert_eq!(preview["package_changes"][0]["version_after"], "1.2.4");
    assert_eq!(preview["version_strategy"], "independent");
    assert_eq!(fs::read(root.join(".git/index")).unwrap(), index);
    write(&root, ".git/hooks/pre-commit", b"#!/bin/sh\nexit 1\n");
    call(
        &cli,
        &root,
        &["git-sync", "--release", "--message", "fix: adjust app"],
        false,
    );
    for (p, b) in &before {
        assert_eq!(fs::read(root.join(p)).unwrap(), *b);
    }
    assert_eq!(git(&root, &["rev-parse", "HEAD"]), head);
    assert_eq!(git(&root, &["status", "--porcelain=v1"]), status);
    fs::remove_file(root.join(".git/hooks/pre-commit")).unwrap();
    let result = call(
        &cli,
        &root,
        &["git-sync", "--release", "--message", "fix: adjust app"],
        true,
    );
    assert_eq!(result["package_changes"], preview["package_changes"]);
    assert_eq!(result["version_after"], "1.0.1");
    assert_eq!(result["working_tree"], "clean");
    assert_eq!(result["ahead"], 0);
    assert_eq!(result["behind"], 0);
    assert!(
        String::from_utf8(fs::read(root.join("uv.lock")).unwrap())
            .unwrap()
            .contains("1.2.4")
    );
    let repeated = call(
        &cli,
        &root,
        &["git-sync", "--release", "--message", "feat!: ignored"],
        true,
    );
    assert_eq!(repeated["version_bumped"], false);
    assert_eq!(repeated["commit"], result["commit"]);
}
