//! Explicit release-binary benchmark. All generated projects are temporary.
use bridgeforge_core::{ProcessRequest, ProcessRunner, SystemProcessRunner};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn invoke(binary: &Path, root: &Path, args: &[&str]) -> f64 {
    let mut request = ProcessRequest::new(binary, root);
    request.args = args.iter().map(Into::into).collect();
    request.timeout = Duration::from_secs(60);
    request
        .env
        .insert("BRIDGEFORGE_HOOK_ROOT".into(), root.as_os_str().into());
    request.env.insert(
        "CODEX_HOME".into(),
        root.join(".runtime/home").into_os_string(),
    );
    let started = Instant::now();
    let result = SystemProcessRunner.run(&request).unwrap();
    let ms = started.elapsed().as_secs_f64() * 1000.0;
    assert_eq!(
        result.code,
        0,
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(!result.timed_out);
    assert!(result.stdout.is_empty());
    assert!(result.stderr.is_empty());
    ms
}

fn maps(root: &Path) -> Vec<Vec<u8>> {
    ["find-doc.map.md", "sync-docs.map.md"]
        .iter()
        .map(|name| fs::read(root.join(".runtime/bridgeforge-codex").join(name)).unwrap())
        .collect()
}

fn mark(root: &Path) {
    fs::write(
        root.join(".runtime/bridgeforge-codex/project-map-dirty"),
        b"dirty\n",
    )
    .unwrap();
}

#[test]
#[ignore = "explicit R04 offline release Hook index benchmark"]
fn project_map_refresh_performance() {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let binary = repository.join(if cfg!(windows) {
        ".codex/bin/bridgeforge-hook.exe"
    } else {
        ".codex/bin/bridgeforge-hook"
    });
    let binary_hash = hash(&fs::read(&binary).unwrap());
    let mut measurements = Vec::new();
    for (files, docs) in [(1000usize, 50usize), (5000, 200)] {
        let root = std::env::temp_dir().join(format!(
            "bf-map-bench-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let fixture = Fixture(root);
        let root = &fixture.0;
        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir_all(root.join("doc/0_architecture")).unwrap();
        fs::write(
            root.join("AGENTS.md"),
            b"# Index benchmark\n- `Domain` entry\n",
        )
        .unwrap();
        fs::write(
            root.join("Cargo.toml"),
            b"[package]\nname=\"benchmark\"\nversion=\"0.1.0\"\n",
        )
        .unwrap();
        for file in 0..files {
            fs::write(
                root.join(format!("src/code_{file:05}.rs")),
                b"pub fn original() {}\n",
            )
            .unwrap();
        }
        let mut references = String::from("# Design\n");
        for file in 0..20 {
            references.push_str(&format!("`src/code_{file:05}.rs:12`\n"));
        }
        references.push_str("`src/added.rs`\n");
        for doc in 0..docs {
            fs::write(
                root.join(format!("doc/0_architecture/design_{doc:04}.md")),
                &references,
            )
            .unwrap();
        }
        invoke(&binary, root, &["project-map", "ensure-current"]);
        let original = maps(root);
        let target = root.join("src/code_00000.rs");
        let added = root.join("src/added.rs");
        let doc = root.join("doc/0_architecture/design_0000.md");
        for scenario in [
            "noop",
            "source_body",
            "add_file",
            "delete_file",
            "doc_reference",
        ] {
            let mut times = Vec::new();
            let mut expected_hashes = None;
            for _ in 0..5 {
                match scenario {
                    "source_body" => fs::write(&target, b"pub fn changed() {}\n").unwrap(),
                    "add_file" => fs::write(&added, b"pub fn added() {}\n").unwrap(),
                    "delete_file" => {
                        fs::remove_file(&target).unwrap();
                    }
                    "doc_reference" => {
                        fs::write(&doc, "# Changed reference\n`src/code_00020.rs:9`\n").unwrap()
                    }
                    _ => {}
                }
                if scenario != "noop" {
                    mark(root);
                }
                let ms = invoke(&binary, root, &["stop"]);
                assert!(
                    !root
                        .join(".runtime/bridgeforge-codex/project-map-dirty")
                        .exists()
                );
                let actual = maps(root);
                assert_eq!(actual[0], original[0]);
                if matches!(scenario, "noop" | "source_body") {
                    assert_eq!(actual, original);
                } else {
                    assert_ne!(actual[1], original[1]);
                    let sync = String::from_utf8_lossy(&actual[1]);
                    match scenario {
                        "add_file" => assert!(sync.contains("| `src/added.rs` |")),
                        "delete_file" => assert!(!sync.contains("| `src/code_00000.rs` |")),
                        "doc_reference" => assert!(sync.contains("| `src/code_00020.rs` |")),
                        _ => unreachable!(),
                    }
                }
                let hashes = actual.iter().map(|bytes| hash(bytes)).collect::<Vec<_>>();
                if let Some(expected) = &expected_hashes {
                    assert_eq!(&hashes, expected);
                }
                expected_hashes = Some(hashes);
                times.push(ms);
                match scenario {
                    "source_body" | "delete_file" => {
                        fs::write(&target, b"pub fn original() {}\n").unwrap()
                    }
                    "add_file" => fs::remove_file(&added).unwrap(),
                    "doc_reference" => fs::write(&doc, &references).unwrap(),
                    _ => {}
                }
                // Restore outside the timed region with the same strict route.
                if scenario != "noop" {
                    invoke(&binary, root, &["project-map", "ensure-current"]);
                }
                assert_eq!(maps(root), original);
            }
            times.sort_by(f64::total_cmp);
            measurements.push(json!({"code_files":files,"design_docs":docs,"references_per_doc":21,
                "scenario":scenario,"samples_ms":times,"median_ms":times[2],"min_ms":times[0],"max_ms":times[4],
                "map_hashes":expected_hashes.unwrap()}));
        }
    }
    let report = json!({"schema":1,"binary_sha256":binary_hash,"profile":"release","samples":5,"timing":"Stop child process including index refresh; setup excluded","measurements":measurements});
    if let Some(output) = std::env::var_os("BRIDGEFORGE_MAP_BENCH_OUTPUT") {
        fs::write(output, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    }
    println!("{report}");
}
