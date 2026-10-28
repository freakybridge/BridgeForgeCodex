//! End-to-end release planning over real Git histories; no remote or publication.
use super::tests::{Repo, config, git, plan, repo_uninitialized as repo, write};
use super::*;

#[derive(Clone)]
struct Node<'a> {
    id: &'a str,
    name: &'a str,
    dir: &'a str,
    version: &'a str,
    dependencies: Vec<&'a str>,
}

fn nodes() -> Vec<Node<'static>> {
    vec![
        Node {
            id: "a",
            name: "a",
            dir: "a",
            version: "1.2.3",
            dependencies: vec![],
        },
        Node {
            id: "b",
            name: "b",
            dir: "b",
            version: "2.0.0",
            dependencies: vec!["a"],
        },
    ]
}

fn manifest(language: &str, dir: &str) -> String {
    format!(
        "{dir}/{}",
        if language == "rust" {
            "Cargo.toml"
        } else {
            "pyproject.toml"
        }
    )
}

fn endpoint(language: &str, node: &Node<'_>) -> Value {
    json!({"id":node.id,"language":language,"manifest":manifest(language,node.dir),"name":node.name})
}

fn transition(
    language: &str,
    base: &str,
    operation: &str,
    compatibility: &str,
    from: &[Node<'_>],
    to: &[Node<'_>],
) -> Value {
    json!({"id":format!("{operation}-packages"),"base":base,"operation":operation,"compatibility":compatibility,
        "from":from.iter().map(|n|endpoint(language,n)).collect::<Vec<_>>(),
        "to":to.iter().map(|n|endpoint(language,n)).collect::<Vec<_>>()})
}

fn graph_config(language: &str, graph: &[Node<'_>], events: Vec<Value>) -> Value {
    let mut cfg = config();
    cfg["packages"] = json!(
        graph
            .iter()
            .map(|n| endpoint(language, n))
            .collect::<Vec<_>>()
    );
    cfg["lifecycle"] = json!(events);
    if language == "rust" {
        cfg["adapters"] = json!({"rust":{"workspaces":["Cargo.toml"]}});
        // A virtual root and its lock belong to the product. Individual package
        // changes are inferred from their actual manifests and source files.
        cfg["product_inputs"] = json!(["README.md", "Cargo.toml", "Cargo.lock"]);
    } else {
        let mut sources = serde_json::Map::new();
        for node in graph {
            let mut mapping = serde_json::Map::new();
            for target in &node.dependencies {
                let target = graph.iter().find(|n| n.id == *target).unwrap();
                mapping.insert(target.name.into(), json!(manifest(language, target.dir)));
            }
            if !mapping.is_empty() {
                sources.insert(manifest(language, node.dir), Value::Object(mapping));
            }
        }
        cfg["adapters"] = json!({"python":{"projects":graph.iter().map(|n|manifest(language,n.dir)).collect::<Vec<_>>(),"local_sources":sources}});
    }
    cfg
}

fn write_graph(root: &Path, language: &str, graph: &[Node<'_>], events: Vec<Value>) -> Value {
    // Every directory in this list is created solely by this temporary fixture.
    for dir in ["a", "b", "c", "d", "moved", "foundation", "merged"] {
        let path = root.join(dir);
        if path.exists() {
            fs::remove_dir_all(path).unwrap();
        }
    }
    for node in graph {
        let mut body = if language == "rust" {
            format!(
                "[package]\nname='{}'\nversion='{}'\nedition='2024'\n",
                node.name, node.version
            )
        } else {
            format!(
                "[project]\nname='{}'\nversion='{}'\n",
                node.name, node.version
            )
        };
        if !node.dependencies.is_empty() {
            if language == "rust" {
                body.push_str("[dependencies]\n");
                for target in &node.dependencies {
                    let target = graph.iter().find(|n| n.id == *target).unwrap();
                    body.push_str(&format!(
                        "{}={{path='../{}',version='>={}'}}\n",
                        target.name, target.dir, target.version
                    ));
                }
            } else {
                body.push_str("dependencies=[");
                for target in &node.dependencies {
                    let target = graph.iter().find(|n| n.id == *target).unwrap();
                    body.push_str(&format!("'{}>={}',", target.name, target.version));
                }
                body.push_str("]\n");
            }
        }
        write(root, &manifest(language, node.dir), body.as_bytes());
        // Stable source contents prove that a move preserved the payload.
        if language == "rust" {
            write(
                root,
                &format!("{}/src/lib.rs", node.dir),
                b"pub fn value() -> u8 { 1 }\n",
            );
        } else {
            write(
                root,
                &format!("{}/{}.py", node.dir, node.id),
                if node.id == "b" {
                    b"VALUE=2\n"
                } else {
                    b"VALUE=1\n"
                },
            );
        }
    }
    if language == "rust" {
        let members = graph
            .iter()
            .map(|n| format!("'{}'", n.dir))
            .collect::<Vec<_>>()
            .join(",");
        write(
            root,
            "Cargo.toml",
            format!("[workspace]\nmembers=[{members}]\nresolver='3'\n").as_bytes(),
        );
        let mut lock = String::from("version=4\n");
        for node in graph {
            lock.push_str(&format!(
                "[[package]]\nname='{}'\nversion='{}'\n",
                node.name, node.version
            ));
            if !node.dependencies.is_empty() {
                let dependencies = node
                    .dependencies
                    .iter()
                    .map(|id| format!("'{}'", graph.iter().find(|n| n.id == *id).unwrap().name))
                    .collect::<Vec<_>>()
                    .join(",");
                lock.push_str(&format!("dependencies=[{dependencies}]\n"));
            }
        }
        write(root, "Cargo.lock", lock.as_bytes());
    }
    let mut cfg = graph_config(language, graph, events);
    if let Ok(bytes) = fs::read(root.join(CONFIG)) {
        let old: Value = serde_json::from_slice(&bytes).unwrap();
        if old["schema_version"] == 4 {
            cfg["schema_version"] = json!(4);
            cfg["baseline"] = old["baseline"].clone();
            cfg["git_sync_setup"] = old["git_sync_setup"].clone();
            for key in [
                "release_model",
                "product_manifests",
                "development",
                "python_environment",
            ] {
                if let Some(value) = old.get(key) {
                    cfg[key] = value.clone();
                }
            }
        }
    }
    write(root, CONFIG, &serde_json::to_vec(&cfg).unwrap());
    cfg
}

fn fixture(language: &str) -> Repo {
    let root = repo();
    write_graph(&root.0, language, &nodes(), vec![]);
    // Replace the temporary repository's initial fixture snapshot, keeping one
    // real initial release boundary for both language variants.
    git(&root.0, &["add", "-A"]);
    git(&root.0, &["commit", "--amend", "--no-edit"]);
    super::super::schema4::fixture::migrate(&root.0, &crate::SystemProcessRunner).unwrap();
    root
}

fn assert_versions(p: &FileReleasePlan, expected: &[(&str, &str)]) {
    let actual = p
        .package_changes
        .iter()
        .map(|c| (c.name.as_str(), c.version_after.as_str()))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(actual, expected.iter().copied().collect());
}

fn finish_and_repeat(root: &Path, p: &FileReleasePlan) {
    super::super::apply_file_release_plan(p).unwrap();
    git(root, &["add", "."]);
    git(root, &["commit", "-m", "chore: schema4 release"]);
    assert!(plan(root).unwrap().is_none());
}

#[test]
fn schema4_both_languages_own_source_and_new_packages_have_distinct_versions() {
    for language in ["rust", "python"] {
        let root = fixture(language);
        write(
            &root.0,
            if language == "rust" {
                "a/src/lib.rs"
            } else {
                "a/a.py"
            },
            if language == "rust" {
                b"pub fn value()->u8{2}\n"
            } else {
                b"VALUE=3\n"
            },
        );
        let p = plan(&root.0).unwrap().unwrap();
        assert_versions(&p, &[("a", "1.2.4")]);
        finish_and_repeat(&root.0, &p);
        let root = fixture(language);
        let mut graph = nodes();
        graph.push(Node {
            id: "c",
            name: "c",
            dir: "c",
            version: "0.1.0",
            dependencies: vec![],
        });
        write_graph(&root.0, language, &graph, vec![]);
        let p = plan(&root.0).unwrap().unwrap();
        assert_versions(&p, &[("c", "0.1.0")]);
        assert!(
            p.package_events
                .iter()
                .any(|e| e.id == "c" && e.operation == "add")
        );
        finish_and_repeat(&root.0, &p);
    }
}

fn lifecycle_case(language: &str, operation: &str, breaking: bool) -> Repo {
    let root = fixture(language);
    let base = super::super::schema4::load(&root.0)
        .unwrap()
        .unwrap()
        .baseline
        .revision;
    let before = nodes();
    let mut after = before.clone();
    let (from, to) = match operation {
        "move" => {
            after[0].dir = "moved";
            (vec![before[0].clone()], vec![after[0].clone()])
        }
        "rename" => {
            after[0].name = "foundation";
            after[0].dir = "foundation";
            (vec![before[0].clone()], vec![after[0].clone()])
        }
        "remove" => {
            after.remove(0);
            after[0].dependencies.clear();
            (vec![before[0].clone()], vec![])
        }
        "split" => {
            after.push(Node {
                id: "c",
                name: "c",
                dir: "c",
                version: "0.1.0",
                dependencies: vec![],
            });
            after[1].dependencies.push("c");
            (
                vec![before[0].clone()],
                vec![after[0].clone(), after[2].clone()],
            )
        }
        "merge" => {
            after = vec![before[0].clone()];
            (before.clone(), after.clone())
        }
        _ => unreachable!(),
    };
    let event = transition(
        language,
        &base,
        operation,
        if breaking { "breaking" } else { "compatible" },
        &from,
        &to,
    );
    write_graph(&root.0, language, &after, vec![event]);
    root
}

#[test]
fn schema4_both_languages_lifecycle_changes_keep_identity_and_tombstones() {
    for language in ["rust", "python"] {
        for operation in ["move", "rename", "remove", "split", "merge"] {
            let root = lifecycle_case(language, operation, false);
            let index = fs::read(root.0.join(".git/index")).unwrap();
            let p = plan(&root.0).unwrap().unwrap();
            assert_eq!(p.new_version.to_string(), "1.0.1");
            assert_eq!(fs::read(root.0.join(".git/index")).unwrap(), index);
            let expected: Vec<(&str, &str)> = match operation {
                "move" => {
                    if language == "rust" {
                        vec![("a", "1.2.4"), ("b", "2.0.1")]
                    } else {
                        vec![("a", "1.2.4")]
                    }
                }
                "rename" => vec![("foundation", "1.2.4"), ("b", "2.0.1")],
                "remove" => vec![("b", "2.0.1")],
                "split" => vec![("a", "1.2.4"), ("b", "2.0.1"), ("c", "0.1.0")],
                "merge" => vec![("a", "1.2.4")],
                _ => unreachable!(),
            };
            assert_versions(&p, &expected);
            if matches!(operation, "remove" | "merge") {
                assert!(
                    p.package_events
                        .iter()
                        .any(|e| e.operation == "retire" && e.version_after.is_none())
                );
            }
            assert_eq!(p.lifecycle_events.len(), 1);
            finish_and_repeat(&root.0, &p);
        }
    }
}

#[test]
fn schema4_all_breaking_lifecycle_operations_require_explicit_acceptance() {
    for language in ["rust", "python"] {
        for operation in ["rename", "remove", "split", "merge"] {
            let root = lifecycle_case(language, operation, true);
            let request = super::super::schema4::fixture::development_analysis(
                &root.0,
                &crate::SystemProcessRunner,
                &[],
                false,
            )
            .unwrap();
            let result = super::super::schema4::development::preview(
                &root.0,
                &request,
                &crate::SystemProcessRunner,
            );
            assert!(
                result.stderr.contains("breaking lifecycle"),
                "{}",
                result.stderr
            );
            let p = accepted_lifecycle_plan(&root.0);
            assert_eq!(p.new_version.to_string(), "2.0.0");
            finish_and_repeat(&root.0, &p);
        }
    }
}

fn accepted_lifecycle_plan(root: &Path) -> FileReleasePlan {
    super::super::schema4::fixture::close_packages_accepted(
        root,
        &crate::SystemProcessRunner,
        &[],
        true,
    )
    .unwrap();
    let request =
        super::super::schema4::fixture::analysis(root, &crate::SystemProcessRunner, "major", &[])
            .unwrap();
    let path = root.join(request.analysis_file.as_ref().unwrap());
    let mut value: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    let acceptance = json!({"fingerprint":value["fingerprint"],"scope":["product"],"reason":"fixture accepts cumulative lifecycle migration","source":"existing fixture user decision"});
    value["product"] = json!({"level":"major","reason":"cumulative breaking lifecycle","compatibility":"breaking","acceptance":acceptance});
    fs::write(path, serde_json::to_vec(&value).unwrap()).unwrap();
    super::super::schema4::build(
        root,
        "feat!: accepted lifecycle",
        &request,
        &crate::SystemProcessRunner,
    )
    .unwrap()
    .unwrap()
}
