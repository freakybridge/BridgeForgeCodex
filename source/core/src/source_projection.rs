//! Deterministic, bounded Cargo source projections. No build or installation I/O.
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs;
use std::path::{Component, Path};
use toml_edit::{Array, DocumentMut, Item, value};

pub const PUBLIC: &str = "public-runtime-v1";
pub const FACTORY: &str = "factory-runtime-v1";

pub fn available(root: &Path) -> bool {
    root.join("source/core/Cargo.toml").is_file()
}

pub fn source_root(root: &Path) -> std::path::PathBuf {
    if available(root) {
        root.to_path_buf()
    } else {
        root.join("templates/hooks")
    }
}

pub fn tree_sha(files: &BTreeMap<String, Vec<u8>>) -> Result<String, String> {
    use sha2::{Digest, Sha256};
    let records = files.iter().filter(|(p, _)| p.as_str() != "Cargo.lock").map(|(p, bytes)| {
        let normalized = if bytes.contains(&0) { bytes.clone() } else {
            String::from_utf8_lossy(bytes).replace("\r\n", "\n").replace('\r', "\n").into_bytes()
        };
        serde_json::json!({"path":p,"sha256":format!("sha256:{:x}", Sha256::digest(normalized))})
    }).collect::<Vec<_>>();
    crate::manifest::canonical_sha(&serde_json::json!(records))
}

fn document(bytes: &[u8], label: &str) -> Result<DocumentMut, String> {
    std::str::from_utf8(bytes)
        .map_err(|e| format!("{label}: {e}"))?
        .parse()
        .map_err(|e| format!("{label}: {e}"))
}

fn read(root: &Path, relative: &str) -> Result<Vec<u8>, String> {
    if relative.is_empty()
        || relative.contains('\\')
        || Path::new(relative)
            .components()
            .any(|p| !matches!(p, Component::Normal(_)))
    {
        return Err(format!("unsafe projection input: {relative}"));
    }
    let path = root.join(relative);
    for ancestor in path.ancestors() {
        if crate::persistence::is_link_or_reparse(ancestor).map_err(|e| e.to_string())? {
            return Err(format!(
                "projection input traverses a linked path: {relative}"
            ));
        }
    }
    fs::read(&path).map_err(|e| format!("projection input {relative}: {e}"))
}

fn strings(item: &Item, label: &str) -> Result<Vec<String>, String> {
    item.as_array()
        .ok_or_else(|| format!("{label} must be an explicit array"))?
        .iter()
        .map(|v| {
            v.as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("{label} contains a non-string"))
        })
        .collect()
}

fn members(manifest: &DocumentMut, public: bool) -> Result<Vec<String>, String> {
    let workspace = manifest.get("workspace").ok_or("workspace is missing")?;
    let all = strings(
        workspace
            .get("members")
            .ok_or("workspace members missing")?,
        "workspace members",
    )?;
    let selected = if public {
        strings(
            workspace
                .get("metadata")
                .and_then(|m| m.get("bridgeforge"))
                .and_then(|m| m.get("public-members"))
                .ok_or("public members missing")?,
            "public members",
        )?
    } else {
        all.clone()
    };
    let mut seen = BTreeSet::new();
    for member in &selected {
        if !member.starts_with("source/")
            || member.split('/').count() != 2
            || Path::new(member)
                .components()
                .any(|p| !matches!(p, Component::Normal(_)))
            || member.contains(['*', '?', '[', '\\'])
            || !all.contains(member)
            || !seen.insert(member)
            || (public && member == "source/factory")
        {
            return Err(format!("invalid projection member: {member}"));
        }
    }
    if selected.is_empty() {
        return Err("projection has no members".into());
    }
    Ok(selected)
}

fn collect(
    root: &Path,
    relative: &str,
    files: &mut BTreeMap<String, Vec<u8>>,
) -> Result<(), String> {
    let directory = root.join(relative);
    if crate::persistence::is_link_or_reparse(&directory).map_err(|e| e.to_string())? {
        return Err(format!("projection directory is linked: {relative}"));
    }
    for entry in fs::read_dir(&directory).map_err(|e| format!("{relative}: {e}"))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "projection name is not UTF-8")?;
        if matches!(
            name.as_str(),
            "target" | ".git" | ".runtime" | ".deps" | ".build"
        ) {
            continue;
        }
        let child = format!("{relative}/{name}");
        if crate::persistence::is_link_or_reparse(&entry.path()).map_err(|e| e.to_string())? {
            return Err(format!("projection path is linked: {child}"));
        }
        if entry.file_type().map_err(|e| e.to_string())?.is_dir() {
            collect(root, &child, files)?;
        } else if entry.file_type().map_err(|e| e.to_string())?.is_file() {
            files.insert(child.clone(), read(root, &child)?);
        } else {
            return Err(format!("projection input is not a regular file: {child}"));
        }
    }
    Ok(())
}

fn local_dependency_paths(
    table: &Item,
    inherited: Option<&Item>,
    mut visit: impl FnMut(&str, bool) -> Result<(), String>,
) -> Result<(), String> {
    if let Some(table) = table.as_table_like() {
        for (name, item) in table.iter() {
            if item.get("workspace").and_then(Item::as_bool) == Some(true) {
                let dependency = inherited
                    .and_then(|t| t.get(name))
                    .ok_or_else(|| format!("workspace dependency missing: {name}"))?;
                if let Some(path) = dependency.get("path").and_then(Item::as_str) {
                    visit(path, true)?;
                }
            } else if let Some(path) = item.get("path").and_then(Item::as_str) {
                visit(path, false)?;
            }
        }
    }
    Ok(())
}

fn validate_dependencies(
    manifest: &DocumentMut,
    workspace: &DocumentMut,
    owner: &str,
    selected: &[String],
) -> Result<(), String> {
    let visit = |raw: &str, inherited: bool| {
        if raw.contains('\\') {
            return Err("dependency path must use forward slashes".into());
        }
        let mut parts = if inherited {
            Vec::new()
        } else {
            owner.split('/').map(str::to_owned).collect::<Vec<_>>()
        };
        for component in Path::new(raw).components() {
            match component {
                Component::ParentDir => {
                    parts.pop().ok_or("dependency escapes source root")?;
                }
                Component::Normal(part) => {
                    parts.push(part.to_str().ok_or("dependency is not UTF-8")?.into())
                }
                Component::CurDir => {}
                _ => return Err("dependency path is absolute".into()),
            }
        }
        let path = parts.join("/");
        if !selected.contains(&path) {
            return Err(format!("{owner}: dependency is outside projection: {raw}"));
        }
        Ok(())
    };
    let inherited = workspace
        .get("workspace")
        .and_then(|w| w.get("dependencies"));
    for kind in ["dependencies", "build-dependencies", "dev-dependencies"] {
        if let Some(table) = manifest.get(kind) {
            local_dependency_paths(table, inherited, &visit)?;
        }
        if let Some(targets) = manifest.get("target").and_then(Item::as_table_like) {
            for (_, target) in targets.iter() {
                if let Some(table) = target.get(kind) {
                    local_dependency_paths(table, inherited, &visit)?;
                }
            }
        }
    }
    Ok(())
}

pub fn lock_dependency_closure(bytes: &[u8], names: &BTreeSet<String>) -> Result<Vec<u8>, String> {
    let mut lock = document(bytes, "Cargo.lock")?;
    let entries = lock
        .get("package")
        .and_then(Item::as_array_of_tables)
        .ok_or("Cargo.lock package entries missing")?;
    let key = |entry: &toml_edit::Table| -> Result<(String, String, Option<String>), String> {
        Ok((
            entry
                .get("name")
                .and_then(Item::as_str)
                .ok_or("lock package name missing")?
                .into(),
            entry
                .get("version")
                .and_then(Item::as_str)
                .ok_or("lock package version missing")?
                .into(),
            entry
                .get("source")
                .map(|v| {
                    v.as_str()
                        .map(str::to_owned)
                        .ok_or("lock package source must be a string")
                })
                .transpose()?,
        ))
    };
    let mut queue = VecDeque::new();
    for (index, entry) in entries.iter().enumerate() {
        let (name, _, source) = key(entry)?;
        if source.is_none() && names.contains(&name) {
            queue.push_back(index);
        }
    }
    if queue.len() != names.len() {
        return Err("public lock lacks an unambiguous local package identity".into());
    }
    let mut retained = BTreeSet::new();
    while let Some(index) = queue.pop_front() {
        if !retained.insert(index) {
            continue;
        }
        if let Some(dependencies) = entries
            .get(index)
            .unwrap()
            .get("dependencies")
            .and_then(Item::as_array)
        {
            for dependency in dependencies {
                let raw = dependency.as_str().ok_or("invalid lock dependency")?;
                let parts = raw.split_whitespace().collect::<Vec<_>>();
                if parts.is_empty() || parts.len() > 3 {
                    return Err(format!("invalid projected lock dependency: {raw}"));
                }
                let candidates = entries
                    .iter()
                    .enumerate()
                    .filter_map(|(i, e)| {
                        let (name, version, source) = key(e).ok()?;
                        (parts.first().copied() == Some(name.as_str())
                            && (parts.len() < 2 || parts[1] == version)
                            && (parts.len() < 3
                                || source
                                    .as_deref()
                                    .is_some_and(|s| parts[2] == format!("({s})"))))
                        .then_some(i)
                    })
                    .collect::<Vec<_>>();
                let local = candidates
                    .iter()
                    .copied()
                    .filter(|i| entries.get(*i).unwrap().get("source").is_none())
                    .collect::<Vec<_>>();
                let selected = if parts.len() < 3 && local.len() == 1 {
                    local[0]
                } else if candidates.len() == 1 {
                    candidates[0]
                } else {
                    return Err(format!("ambiguous projected lock dependency: {raw}"));
                };
                queue.push_back(selected);
            }
        }
    }
    let entries = lock["package"].as_array_of_tables_mut().unwrap();
    for index in (0..entries.len()).rev() {
        if !retained.contains(&index) {
            entries.remove(index);
        }
    }
    Ok(lock.to_string().into_bytes())
}

/// Render a self-contained source tree, retaining pinned dependencies and no factory sources.
pub fn render(root: &Path, public: bool) -> Result<BTreeMap<String, Vec<u8>>, String> {
    render_with_overlay(root, public, &BTreeMap::new())
}

/// Apply candidate native versions in memory, before a transaction writes them.
pub fn render_with_overlay(
    root: &Path,
    public: bool,
    overlay: &BTreeMap<String, Vec<u8>>,
) -> Result<BTreeMap<String, Vec<u8>>, String> {
    let root_bytes = overlay
        .get("Cargo.toml")
        .cloned()
        .map(Ok)
        .unwrap_or_else(|| read(root, "Cargo.toml"))?;
    let mut manifest = document(&root_bytes, "Cargo.toml")?;
    let selected = members(&manifest, public)?;
    let mut files = BTreeMap::new();
    let mut names = BTreeSet::new();
    for member in &selected {
        collect(root, member, &mut files)?;
        for (path, bytes) in &mut files {
            if let Some(candidate) = overlay.get(path) {
                *bytes = candidate.clone();
            }
        }
        let relative = format!("{member}/Cargo.toml");
        let package = document(
            files
                .get(&relative)
                .ok_or("projection package manifest missing")?,
            &relative,
        )?;
        validate_dependencies(&package, &manifest, member, &selected)?;
        let name = package
            .get("package")
            .and_then(|p| p.get("name"))
            .and_then(Item::as_str)
            .ok_or("projection package name missing")?;
        if !names.insert(name.to_owned()) {
            return Err("projection has duplicate package names".into());
        }
    }
    if public {
        manifest["workspace"]["members"] =
            value(selected.iter().map(String::as_str).collect::<Array>());
        manifest["workspace"]
            .as_table_mut()
            .ok_or("workspace is not a table")?
            .remove("default-members");
        manifest["workspace"]
            .as_table_mut()
            .unwrap()
            .remove("exclude");
        let dependencies = manifest
            .get_mut("workspace")
            .and_then(|w| w.get_mut("dependencies"))
            .and_then(Item::as_table_like_mut);
        if let Some(dependencies) = dependencies {
            let remove = dependencies
                .iter()
                .filter_map(|(name, item)| {
                    item.get("path")
                        .and_then(Item::as_str)
                        .filter(|p| !selected.iter().any(|s| s == *p))
                        .map(|_| name.to_owned())
                })
                .collect::<Vec<_>>();
            for name in remove {
                dependencies.remove(&name);
            }
        }
    }
    files.insert("Cargo.toml".into(), manifest.to_string().into_bytes());
    let ignore = if root.join("templates/hooks/.gitignore").is_file() {
        Some("templates/hooks/.gitignore")
    } else if root.join(".gitignore").is_file() {
        Some(".gitignore")
    } else {
        None
    };
    if let Some(ignore) = ignore {
        files.insert(".gitignore".into(), read(root, ignore)?);
    }
    let lock = overlay
        .get("Cargo.lock")
        .cloned()
        .map(Ok)
        .unwrap_or_else(|| read(root, "Cargo.lock"))?;
    files.insert(
        "Cargo.lock".into(),
        if public {
            lock_dependency_closure(&lock, &names)?
        } else {
            lock
        },
    );
    Ok(files)
}

pub fn projected(root: &Path, mode: &str) -> Result<BTreeMap<String, Vec<u8>>, String> {
    match mode {
        PUBLIC => render(root, true),
        FACTORY => render(root, false),
        _ => Err("unknown source projection".into()),
    }
}

#[cfg(all(test, bridgeforge_factory_tests))]
#[path = "../../../scripts/tests/unit/core_source_projection.rs"]
mod tests;
