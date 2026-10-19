//! End-to-end release planning over real Git histories; no remote or publication.
use super::tests::{Repo, config, git, plan, repo, write};
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
    let cfg = graph_config(language, graph, events);
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
    for (path, bytes) in &p.writes {
        fs::write(path, bytes).unwrap();
    }
    git(root, &["add", "-A"]);
    git(root, &["commit", "-m", "chore: release"]);
    assert!(
        plan(root).unwrap().is_none(),
        "consumed lifecycle must not replay or bump again"
    );
}

#[test]
fn polyglot_both_languages_modification_and_new_package_have_distinct_version_rules() {
    for language in ["python", "rust"] {
        let root = fixture(language);
        let source = if language == "rust" {
            "a/src/lib.rs"
        } else {
            "a/a.py"
        };
        write(
            &root.0,
            source,
            if language == "rust" {
                b"pub fn value() -> u8 { 2 }\n"
            } else {
                b"VALUE=3\n"
            },
        );
        let p = plan(&root.0).unwrap().unwrap();
        assert_versions(&p, &[("a", "1.2.4"), ("b", "2.0.1")]);
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
        assert_eq!(p.package_changes[0].reason, "first_release");
        assert!(
            p.package_events
                .iter()
                .any(|e| e.id == "c" && e.operation == "add")
        );
        finish_and_repeat(&root.0, &p);
    }
}

#[test]
fn polyglot_both_languages_rename_preserves_history_and_propagates_versions() {
    for language in ["python", "rust"] {
        let root = fixture(language);
        let base = git(&root.0, &["rev-parse", "HEAD"]);
        let before = nodes();
        let mut after = before.clone();
        after[0].name = "foundation";
        after[0].dir = "foundation";
        let event = transition(
            language,
            &base,
            "rename",
            "compatible",
            &before[..1],
            &after[..1],
        );
        write_graph(&root.0, language, &after, vec![event]);
        let p = plan(&root.0).unwrap().unwrap();
        assert_versions(&p, &[("foundation", "1.2.4"), ("b", "2.0.1")]);
        let event = p.package_events.iter().find(|e| e.id == "a").unwrap();
        assert_eq!(event.operation, "rename");
        assert_eq!(event.name_before.as_deref(), Some("a"));
        assert_eq!(event.name_after.as_deref(), Some("foundation"));
        assert_eq!(event.version_before.as_deref(), Some("1.2.3"));
        finish_and_repeat(&root.0, &p);
    }
}

#[test]
fn polyglot_both_languages_mechanical_move_does_not_bump_packages() {
    for language in ["python", "rust"] {
        let root = fixture(language);
        let base = git(&root.0, &["rev-parse", "HEAD"]);
        let before = nodes();
        let mut after = before.clone();
        after[0].dir = "moved";
        let event = transition(
            language,
            &base,
            "move",
            "mechanical",
            &before[..1],
            &after[..1],
        );
        write_graph(&root.0, language, &after, vec![event]);
        let p = plan(&root.0).unwrap().unwrap();
        assert!(
            p.package_changes.is_empty(),
            "{language}: pure move unexpectedly changed versions: {:?}",
            p.package_changes
        );
        assert_eq!(p.new_version.to_string(), "1.0.1");
        let event = p.package_events.iter().find(|e| e.id == "a").unwrap();
        assert_eq!(event.operation, "move");
        assert_eq!(event.version_after.as_deref(), Some("1.2.3"));
        finish_and_repeat(&root.0, &p);
    }
}

#[test]
fn polyglot_both_languages_removal_records_tombstone_and_changed_old_incoming_edge() {
    for language in ["python", "rust"] {
        let root = fixture(language);
        let base = git(&root.0, &["rev-parse", "HEAD"]);
        let before = nodes();
        let mut survivor = before[1].clone();
        survivor.dependencies.clear();
        let event = transition(language, &base, "remove", "compatible", &before[..1], &[]);
        write_graph(&root.0, language, &[survivor], vec![event]);
        let p = plan(&root.0).unwrap().unwrap();
        assert_versions(&p, &[("b", "2.0.1")]);
        let event = p.package_events.iter().find(|e| e.id == "a").unwrap();
        assert_eq!(event.operation, "retire");
        assert!(event.version_after.is_none());
        assert_eq!(event.version_before.as_deref(), Some("1.2.3"));
        finish_and_repeat(&root.0, &p);
    }
}

#[test]
fn polyglot_both_languages_split_retains_one_version_line_and_starts_another() {
    for language in ["python", "rust"] {
        let root = fixture(language);
        let base = git(&root.0, &["rev-parse", "HEAD"]);
        let before = nodes();
        let mut after = before.clone();
        after.push(Node {
            id: "c",
            name: "c",
            dir: "c",
            version: "0.1.0",
            dependencies: vec![],
        });
        after[1].dependencies.push("c");
        let destinations = vec![after[0].clone(), after[2].clone()];
        let event = transition(
            language,
            &base,
            "split",
            "compatible",
            &before[..1],
            &destinations,
        );
        write_graph(&root.0, language, &after, vec![event]);
        let p = plan(&root.0).unwrap().unwrap();
        assert_versions(&p, &[("a", "1.2.4"), ("b", "2.0.1"), ("c", "0.1.0")]);
        assert!(
            p.package_events
                .iter()
                .find(|e| e.id == "a")
                .unwrap()
                .related
                .contains(&"c".into())
        );
        assert!(
            p.package_events
                .iter()
                .any(|e| e.id == "c" && e.operation == "add")
        );
        finish_and_repeat(&root.0, &p);
    }
}

#[test]
fn polyglot_both_languages_split_can_retire_the_old_line_and_start_two_new_ones() {
    for language in ["python", "rust"] {
        let root = fixture(language);
        let base = git(&root.0, &["rev-parse", "HEAD"]);
        let before = nodes();
        let mut consumer = before[1].clone();
        consumer.dependencies = vec!["c", "d"];
        let destinations = vec![
            Node {
                id: "c",
                name: "c",
                dir: "c",
                version: "0.1.0",
                dependencies: vec![],
            },
            Node {
                id: "d",
                name: "d",
                dir: "d",
                version: "0.2.0",
                dependencies: vec![],
            },
        ];
        let event = transition(
            language,
            &base,
            "split",
            "breaking",
            &before[..1],
            &destinations,
        );
        let mut after = destinations;
        after.push(consumer);
        write_graph(&root.0, language, &after, vec![event]);
        let p = plan(&root.0).unwrap().unwrap();
        assert_versions(&p, &[("b", "2.0.1"), ("c", "0.1.0"), ("d", "0.2.0")]);
        assert_eq!(p.new_version.to_string(), "2.0.0");
        let retired = p.package_events.iter().find(|e| e.id == "a").unwrap();
        assert_eq!(retired.operation, "retire");
        assert_eq!(
            retired.related,
            BTreeSet::from(["c".to_string(), "d".to_string()])
                .into_iter()
                .collect::<Vec<_>>()
        );
        finish_and_repeat(&root.0, &p);
    }
}

#[test]
fn polyglot_both_languages_breaking_rename_raises_inherited_line_and_product_major() {
    for language in ["python", "rust"] {
        let root = fixture(language);
        let base = git(&root.0, &["rev-parse", "HEAD"]);
        let before = nodes();
        let mut after = before.clone();
        after[0].name = "foundation";
        after[0].dir = "foundation";
        let event = transition(
            language,
            &base,
            "rename",
            "breaking",
            &before[..1],
            &after[..1],
        );
        write_graph(&root.0, language, &after, vec![event]);
        let p = plan(&root.0).unwrap().unwrap();
        assert_versions(&p, &[("foundation", "2.0.0"), ("b", "2.0.1")]);
        assert_eq!(p.new_version.to_string(), "2.0.0");
        finish_and_repeat(&root.0, &p);
    }
}

#[test]
fn polyglot_both_languages_merge_can_retain_or_replace_the_version_line() {
    for language in ["python", "rust"] {
        for retain in [true, false] {
            let root = fixture(language);
            let base = git(&root.0, &["rev-parse", "HEAD"]);
            let before = nodes();
            let destination = if retain {
                before[0].clone()
            } else {
                Node {
                    id: "merged",
                    name: "merged",
                    dir: "merged",
                    version: "0.2.0",
                    dependencies: vec![],
                }
            };
            let event = transition(
                language,
                &base,
                "merge",
                "compatible",
                &before,
                std::slice::from_ref(&destination),
            );
            write_graph(
                &root.0,
                language,
                std::slice::from_ref(&destination),
                vec![event],
            );
            let p = plan(&root.0).unwrap().unwrap();
            assert_versions(
                &p,
                &[(destination.name, if retain { "1.2.4" } else { "0.2.0" })],
            );
            assert!(
                p.package_events
                    .iter()
                    .any(|e| e.id == "b" && e.operation == "retire")
            );
            if !retain {
                assert!(
                    p.package_events
                        .iter()
                        .any(|e| e.id == "a" && e.operation == "retire")
                );
            }
            let event = p
                .package_events
                .iter()
                .find(|e| e.id == destination.id)
                .unwrap();
            assert!(event.related.contains(&"b".into()));
            finish_and_repeat(&root.0, &p);
        }
    }
}

#[test]
fn polyglot_both_languages_accept_declaration_added_after_committed_refactor() {
    for language in ["python", "rust"] {
        let root = fixture(language);
        let base = git(&root.0, &["rev-parse", "HEAD"]);
        let before = nodes();
        let mut after = before.clone();
        after[0].name = "foundation";
        after[0].dir = "foundation";
        let mut cfg = write_graph(&root.0, language, &after, vec![]);
        git(&root.0, &["add", "-A"]);
        git(&root.0, &["commit", "-m", "fix: relocate foundation"]);
        let refactor = git(&root.0, &["rev-parse", "HEAD"]);
        cfg["lifecycle"] = json!([transition(
            language,
            &base,
            "rename",
            "compatible",
            &before[..1],
            &after[..1]
        )]);
        write(&root.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
        let p = plan(&root.0).unwrap().unwrap();
        assert_versions(&p, &[("foundation", "1.2.4"), ("b", "2.0.1")]);
        assert_eq!(
            git(&root.0, &["rev-parse", "HEAD"]),
            refactor,
            "planning must not rewrite the committed refactor"
        );
        finish_and_repeat(&root.0, &p);
    }
}

#[test]
fn polyglot_mixed_repo_propagates_rust_build_change_into_python_distribution() {
    let root = fixture("rust");
    let mut cfg = graph_config("rust", &nodes(), vec![]);
    cfg["adapters"]["python"] = json!({"projects":["py/pyproject.toml"]});
    cfg["packages"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"py","language":"python","manifest":"py/pyproject.toml","name":"py"}));
    cfg["dependencies"] = json!([{"owner":"py","target":"a","kind":"build"}]);
    write(
        &root.0,
        "py/pyproject.toml",
        b"[project]\nname='py'\nversion='3.0.0'\n",
    );
    write(&root.0, "py/package.py", b"VALUE=1\n");
    write(&root.0, CONFIG, &serde_json::to_vec(&cfg).unwrap());
    git(&root.0, &["add", "-A"]);
    git(&root.0, &["commit", "--amend", "--no-edit"]);
    write(&root.0, "a/src/lib.rs", b"pub fn value() -> u8 { 2 }\n");
    let p = plan(&root.0).unwrap().unwrap();
    assert_versions(&p, &[("a", "1.2.4"), ("b", "2.0.1"), ("py", "3.0.1")]);
    assert_eq!(
        p.package_changes
            .iter()
            .find(|c| c.name == "py")
            .unwrap()
            .dependencies,
        ["a/Cargo.toml"]
    );
    finish_and_repeat(&root.0, &p);
}
