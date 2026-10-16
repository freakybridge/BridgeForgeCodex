//! Pure Rust release adapter. All filesystem access is supplied by the caller.
use super::{CommitInfo, SemVer, bump};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;
use toml_edit::{DocumentMut, Item, value};

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RustConfig {
    pub workspaces: Vec<String>,
    #[serde(default = "patch")]
    pub dependency_bump: String,
    #[serde(default)]
    pub shared_inputs: Vec<SharedInput>,
    #[serde(default)]
    pub product_inputs: Vec<String>,
}

fn patch() -> String {
    "patch".into()
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SharedInput {
    pub path: String,
    pub packages: Vec<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct PackageChange {
    pub manifest: String,
    pub name: String,
    pub version_before: String,
    pub version_after: String,
    pub reason: String,
    pub dependencies: Vec<String>,
}

#[derive(Clone, Debug)]
pub(super) struct AdapterPlan {
    pub inputs: BTreeMap<String, Option<Vec<u8>>>,
    pub writes: BTreeMap<String, Vec<u8>>,
    pub packages: Vec<PackageChange>,
}

#[derive(Clone, Debug)]
struct Package {
    name: String,
    version: SemVer,
    workspace: String,
    inherited: bool,
    features: BTreeMap<String, Vec<String>>,
}

#[derive(Clone, Debug)]
struct Dependency {
    owner: String,
    target: String,
    kind: DependencyKind,
    alias: String,
    optional: bool,
    features: Vec<String>,
    default_features: bool,
    document: String,
    keys: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DependencyKind {
    Normal,
    Build,
    Dev,
}

#[derive(Clone, Debug)]
pub(super) struct Workspace {
    config: RustConfig,
    inputs: BTreeMap<String, Option<Vec<u8>>>,
    documents: BTreeMap<String, DocumentMut>,
    packages: BTreeMap<String, Package>,
    dependencies: Vec<Dependency>,
    roots: BTreeMap<String, BTreeSet<String>>,
    locks: BTreeSet<String>,
}

fn parse(path: &str, bytes: &[u8]) -> Result<DocumentMut, String> {
    let text = std::str::from_utf8(bytes).map_err(|e| format!("{path}: invalid UTF-8: {e}"))?;
    text.parse()
        .map_err(|e| format!("{path}: invalid TOML: {e}"))
}

// Paths are repository-relative, normalized lexically; the caller rejects links.
fn normalize(base: &str, raw: &str) -> Result<String, String> {
    if raw.is_empty() || raw.contains('\\') || raw.contains(':') || raw.starts_with('/') {
        return Err(format!(
            "Rust release requires a repository-relative slash path: {raw}"
        ));
    }
    let mut parts = base
        .split('/')
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>();
    for part in raw.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if parts.pop().is_none() {
                    return Err(format!("Rust path escapes repository: {raw}"));
                }
            }
            other => parts.push(other),
        }
    }
    Ok(parts.join("/"))
}

fn directory(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(dir, _)| dir)
}

fn manifest_at(dir: &str) -> String {
    if dir.is_empty() {
        "Cargo.toml".into()
    } else {
        format!("{dir}/Cargo.toml")
    }
}

fn configured_path(raw: &str) -> Result<String, String> {
    let trailing = raw.ends_with('/');
    let normalized = normalize("", raw)?;
    if normalized.is_empty() {
        return Err("Rust input path cannot be repository root".into());
    }
    if raw.trim_end_matches('/') != normalized {
        return Err(format!("Rust config path must be normalized: {raw}"));
    }
    Ok(if trailing {
        format!("{normalized}/")
    } else {
        normalized
    })
}

fn matches_input(pattern: &str, path: &str) -> bool {
    if pattern.ends_with('/') {
        path.starts_with(pattern)
    } else {
        path == pattern
    }
}

fn strings(item: Option<&Item>, context: &str) -> Result<Vec<String>, String> {
    let Some(item) = item else {
        return Ok(Vec::new());
    };
    let array = item
        .as_array()
        .ok_or_else(|| format!("{context} must be an array"))?;
    array
        .iter()
        .map(|v| {
            v.as_str()
                .map(String::from)
                .ok_or_else(|| format!("{context} must contain strings"))
        })
        .collect()
}

fn member_pattern(raw: &str) -> Result<Regex, String> {
    if raw.contains('[') || raw.contains('{') || raw.contains('\\') || raw.contains("**") {
        return Err(format!("unsupported Cargo member glob: {raw}"));
    }
    let mut pattern = String::from("^");
    for c in raw.chars() {
        match c {
            '*' => pattern.push_str("[^/]*"),
            '?' => pattern.push_str("[^/]"),
            _ => pattern.push_str(&regex::escape(&c.to_string())),
        }
    }
    pattern.push('$');
    Regex::new(&pattern).map_err(|e| e.to_string())
}

fn read_document(
    workspace: &mut Workspace,
    path: &str,
    read: &mut impl FnMut(&str) -> Result<Option<Vec<u8>>, String>,
) -> Result<(), String> {
    if workspace.documents.contains_key(path) {
        return Ok(());
    }
    let bytes = read(path)?.ok_or_else(|| format!("Rust manifest missing: {path}"))?;
    let document = parse(path, &bytes)?;
    workspace.inputs.insert(path.into(), Some(bytes));
    workspace.documents.insert(path.into(), document);
    Ok(())
}

fn validate_workspace_root(
    workspace: &mut Workspace,
    root: &str,
    read: &mut impl FnMut(&str) -> Result<Option<Vec<u8>>, String>,
) -> Result<(), String> {
    let document = &workspace.documents[root];
    if let Some(pointer) = document.get("package").and_then(|p| p.get("workspace")) {
        let pointer = pointer
            .as_str()
            .ok_or_else(|| format!("{root}: package.workspace must be a path string"))?;
        let actual = manifest_at(&normalize(directory(root), pointer)?);
        if actual != root {
            return Err(format!(
                "{root}: package.workspace identifies {actual}; rust.workspaces must register the actual workspace root"
            ));
        }
        if document.get("workspace").is_none() {
            return Err(format!(
                "{root}: package.workspace does not identify a manifest with [workspace]"
            ));
        }
    }
    // An explicit [workspace] makes a nested package independent of ancestors.
    if document.get("workspace").is_some() {
        return Ok(());
    }
    let root_dir = directory(root);
    if root_dir.is_empty() {
        return Ok(());
    }
    let mut ancestor_dir = directory(root_dir).to_string();
    loop {
        let ancestor = manifest_at(&ancestor_dir);
        if !workspace.inputs.contains_key(&ancestor) {
            let bytes = read(&ancestor)?;
            if let Some(bytes) = &bytes {
                workspace
                    .documents
                    .insert(ancestor.clone(), parse(&ancestor, bytes)?);
            }
            workspace.inputs.insert(ancestor.clone(), bytes);
        }
        if let Some(document) = workspace.documents.get(&ancestor) {
            if let Some(table) = document.get("workspace") {
                let exclusions = strings(
                    table.get("exclude"),
                    &format!("{ancestor}: workspace.exclude"),
                )?;
                let excluded = exclusions
                    .iter()
                    .map(|raw| member_pattern(&normalize(&ancestor_dir, raw)?))
                    .collect::<Result<Vec<_>, String>>()?
                    .iter()
                    .any(|pattern| pattern.is_match(root_dir));
                if excluded {
                    return Ok(());
                }
                // Cargo searches ancestor workspaces even for automatically
                // included path members. An unlisted, unexcluded nested package
                // is rejected by Cargo too, rather than becoming a new root.
                return Err(format!(
                    "{root}: ancestor workspace {ancestor} owns this package; rust.workspaces must register the actual workspace root (or explicitly exclude an independent package)"
                ));
            }
        }
        if ancestor_dir.is_empty() {
            break;
        }
        ancestor_dir = directory(&ancestor_dir).to_string();
    }
    Ok(())
}

pub(super) fn load(
    config: &RustConfig,
    inventory: &[String],
    read: &mut impl FnMut(&str) -> Result<Option<Vec<u8>>, String>,
) -> Result<Workspace, String> {
    if config.workspaces.is_empty() {
        return Err("rust.workspaces must list at least one Cargo.toml".into());
    }
    if config.dependency_bump != "patch" {
        return Err("rust.dependency_bump currently supports only patch".into());
    }
    let mut result = Workspace {
        config: config.clone(),
        inputs: BTreeMap::new(),
        documents: BTreeMap::new(),
        packages: BTreeMap::new(),
        dependencies: Vec::new(),
        roots: BTreeMap::new(),
        locks: BTreeSet::new(),
    };
    let mut seen = BTreeSet::new();
    for root in &config.workspaces {
        if configured_path(root)? != *root
            || root.rsplit('/').next() != Some("Cargo.toml")
            || !seen.insert(root.clone())
        {
            return Err(format!(
                "invalid or duplicate Rust workspace manifest: {root}"
            ));
        }
        read_document(&mut result, root, read)?;
        validate_workspace_root(&mut result, root, read)?;
        let document = &result.documents[root];
        if document.get("patch").is_some() || document.get("replace").is_some() {
            return Err(format!(
                "{root}: patch/replace require an explicit future Rust adapter policy"
            ));
        }
        let root_dir = directory(root);
        let members = strings(
            document.get("workspace").and_then(|w| w.get("members")),
            &format!("{root}: workspace.members"),
        )?;
        let excludes = strings(
            document.get("workspace").and_then(|w| w.get("exclude")),
            &format!("{root}: workspace.exclude"),
        )?;
        let excludes = excludes
            .iter()
            .map(|x| member_pattern(&normalize(root_dir, x)?))
            .collect::<Result<Vec<_>, String>>()?;
        let mut paths = BTreeSet::new();
        if document.get("package").is_some() {
            paths.insert(root.clone());
        }
        for member in members {
            let normalized = normalize(root_dir, &member)?;
            let pattern = member_pattern(&normalized)?;
            let found = inventory
                .iter()
                .filter(|path| {
                    path.rsplit('/').next() == Some("Cargo.toml")
                        && pattern.is_match(directory(path))
                        && !excludes.iter().any(|e| e.is_match(directory(path)))
                })
                .cloned()
                .collect::<Vec<_>>();
            if found.is_empty() {
                return Err(format!(
                    "{root}: Cargo member has no matching manifest: {member}"
                ));
            }
            paths.extend(found);
        }
        if paths.is_empty() {
            return Err(format!("{root}: Rust workspace has no packages"));
        }
        result.roots.insert(root.clone(), paths.clone());
        for path in paths {
            if result.packages.contains_key(&path) {
                return Err(format!(
                    "Rust package is registered in multiple workspaces: {path}"
                ));
            }
            read_document(&mut result, &path, read)?;
            let doc = &result.documents[&path];
            if doc.get("patch").is_some() || doc.get("replace").is_some() {
                return Err(format!("{path}: patch/replace are unsupported"));
            }
            let package = doc
                .get("package")
                .ok_or_else(|| format!("{path}: missing package"))?;
            if path != *root && doc.get("workspace").is_some() {
                return Err(format!(
                    "{path}: nested workspace cannot be a member of {root}; register its actual workspace root separately"
                ));
            }
            if let Some(pointer) = package.get("workspace") {
                let pointer = pointer
                    .as_str()
                    .ok_or_else(|| format!("{path}: package.workspace must be a path string"))?;
                let actual = manifest_at(&normalize(directory(&path), pointer)?);
                if actual != *root {
                    return Err(format!(
                        "{path}: package.workspace identifies {actual}, not configured member root {root}"
                    ));
                }
            }
            let name = package
                .get("name")
                .and_then(Item::as_str)
                .ok_or_else(|| format!("{path}: missing package.name"))?
                .to_owned();
            let version = package
                .get("version")
                .ok_or_else(|| format!("{path}: missing package.version"))?;
            let inherited = version.get("workspace").and_then(Item::as_bool) == Some(true);
            let raw = if inherited {
                result.documents[root]
                    .get("workspace")
                    .and_then(|w| w.get("package"))
                    .and_then(|p| p.get("version"))
                    .and_then(Item::as_str)
            } else {
                version.as_str()
            }
            .ok_or_else(|| format!("{path}: unsupported or missing package version"))?;
            let version = SemVer::from_str(raw)?;
            let mut features = BTreeMap::new();
            if let Some(table) = doc.get("features") {
                let table = table
                    .as_table_like()
                    .ok_or_else(|| format!("{path}: features must be a table"))?;
                for (name, values) in table.iter() {
                    features.insert(
                        name.to_string(),
                        strings(Some(values), &format!("{path}: features.{name}"))?,
                    );
                }
            }
            result.packages.insert(
                path,
                Package {
                    name,
                    version,
                    workspace: root.clone(),
                    inherited,
                    features,
                },
            );
        }
        let lock = if root_dir.is_empty() {
            "Cargo.lock".to_string()
        } else {
            format!("{root_dir}/Cargo.lock")
        };
        let bytes = read(&lock)?;
        if let Some(bytes) = &bytes {
            result.documents.insert(lock.clone(), parse(&lock, bytes)?);
        }
        result.inputs.insert(lock.clone(), bytes);
        result.locks.insert(lock);
    }
    for input in &config.shared_inputs {
        configured_path(&input.path)?;
        if input.packages.is_empty() {
            return Err(format!("shared Rust input has no packages: {}", input.path));
        }
        let mut packages = BTreeSet::new();
        for path in &input.packages {
            if !result.packages.contains_key(path) || !packages.insert(path) {
                return Err(format!(
                    "shared Rust input references unknown or duplicate package: {path}"
                ));
            }
        }
    }
    for path in &config.product_inputs {
        configured_path(path)?;
    }
    let mappings = config
        .shared_inputs
        .iter()
        .map(|x| &x.path)
        .chain(config.product_inputs.iter())
        .collect::<Vec<_>>();
    for (index, path) in mappings.iter().enumerate() {
        for other in &mappings[index + 1..] {
            if path == other
                || matches_input(path, other.trim_end_matches('/'))
                || matches_input(other, path.trim_end_matches('/'))
            {
                return Err(format!(
                    "overlapping Rust input mappings: {path} and {other}"
                ));
            }
        }
    }
    result.collect_dependencies()?;
    result.validate_locks()?;
    Ok(result)
}

impl Workspace {
    pub(super) fn inputs(&self) -> &BTreeMap<String, Option<Vec<u8>>> {
        &self.inputs
    }

    pub(super) fn validate_baseline(&self, old: &Workspace) -> Result<(), String> {
        if self.packages.keys().ne(old.packages.keys()) {
            return Err("Rust package addition/removal/rename requires an explicit migration; package manifest identities changed".into());
        }
        for (path, package) in &self.packages {
            let before = &old.packages[path];
            if package.name != before.name
                || package.version != before.version
                || package.workspace != before.workspace
            {
                return Err(format!(
                    "{path}: package identity or effective version changed outside the managed Rust release"
                ));
            }
        }
        Ok(())
    }

    /// Shared inputs win over inferred package ownership. Root manifest changes
    /// conservatively affect every member; lock changes require an explicit map.
    pub(super) fn affected(&self, path: &str) -> Result<Vec<String>, String> {
        let path = normalize("", path)?;
        if let Some(mapping) = self
            .config
            .shared_inputs
            .iter()
            .find(|m| matches_input(&m.path, &path))
        {
            return Ok(mapping.packages.clone());
        }
        if self
            .config
            .product_inputs
            .iter()
            .any(|m| matches_input(m, &path))
        {
            return Ok(Vec::new());
        }
        if self.locks.contains(&path) {
            return Err(format!(
                "{path}: Cargo.lock changes require explicit shared_inputs or product_inputs attribution"
            ));
        }
        if let Some(members) = self.roots.get(&path) {
            return Ok(members.iter().cloned().collect());
        }
        let best = self
            .packages
            .keys()
            .filter(|manifest| {
                let dir = directory(manifest);
                path == **manifest || (!dir.is_empty() && path.starts_with(&format!("{dir}/")))
            })
            .max_by_key(|manifest| directory(manifest).len());
        if let Some(manifest) = best {
            return Ok(vec![manifest.clone()]);
        }
        // A root package owns ordinary root files, but never another unregistered
        // Cargo package or a lock/config file whose impact cannot be inferred.
        if self.packages.contains_key("Cargo.toml")
            && !path.ends_with("Cargo.toml")
            && !path.ends_with("Cargo.lock")
        {
            return Ok(vec!["Cargo.toml".into()]);
        }
        Err(format!(
            "unattributed Rust release input: {path}; configure shared_inputs or product_inputs"
        ))
    }

    pub(super) fn is_mechanical_change(
        &self,
        path: &str,
        before: Option<&[u8]>,
        after: Option<&[u8]>,
    ) -> Result<bool, String> {
        let (Some(before), Some(after)) = (before, after) else {
            return Ok(false);
        };
        if !self.packages.contains_key(path) && !self.roots.contains_key(path) {
            return Ok(false);
        }
        let old = parse(path, before)?;
        let new = parse(path, after)?;
        let mut old_value: serde_json::Value = toml_to_json(old.as_item())?;
        let mut new_value: serde_json::Value = toml_to_json(new.as_item())?;
        if let Some(package) = self.packages.get(path) {
            for value in [&mut old_value, &mut new_value] {
                let version = value.pointer_mut("/package/version");
                if let Some(version) = version {
                    let inherited = version
                        .get("workspace")
                        .and_then(serde_json::Value::as_bool)
                        == Some(true);
                    if inherited {
                        *version = serde_json::Value::String(package.version.to_string());
                    }
                }
            }
        }
        // Removing an unused workspace.package.version after all inherited
        // versions have been materialized is a mechanical migration as well.
        if self.roots.contains_key(path) {
            let old_version = old_value.pointer("/workspace/package/version").cloned();
            let new_version = new_value.pointer("/workspace/package/version").cloned();
            if old_version != new_version && (old_version.is_none() || new_version.is_none()) {
                let version = old_version
                    .as_ref()
                    .or(new_version.as_ref())
                    .and_then(serde_json::Value::as_str);
                if version.is_some_and(|v| {
                    self.roots[path]
                        .iter()
                        .all(|p| self.packages[p].version.to_string() == v)
                }) {
                    for value in [&mut old_value, &mut new_value] {
                        if let Some(table) = value
                            .pointer_mut("/workspace/package")
                            .and_then(serde_json::Value::as_object_mut)
                        {
                            table.remove("version");
                        }
                    }
                }
            }
        }
        Ok(old_value == new_value)
    }

    fn collect_dependencies(&mut self) -> Result<(), String> {
        for (manifest, package) in &self.packages {
            let doc = &self.documents[manifest];
            let mut sections = Vec::new();
            for kind in ["dependencies", "build-dependencies", "dev-dependencies"] {
                if let Some(table) = doc.get(kind) {
                    sections.push((vec![kind.to_owned()], table));
                }
            }
            if let Some(targets) = doc.get("target").and_then(Item::as_table_like) {
                for (target, item) in targets.iter() {
                    for kind in ["dependencies", "build-dependencies", "dev-dependencies"] {
                        if let Some(table) = item.get(kind) {
                            sections
                                .push((vec!["target".into(), target.into(), kind.into()], table));
                        }
                    }
                }
            }
            for (section, table) in sections {
                // Keep the member's declaration kind even when the actual
                // requirement is inherited from workspace.dependencies.
                let kind = match section.last().map(String::as_str) {
                    Some("dependencies") => DependencyKind::Normal,
                    Some("build-dependencies") => DependencyKind::Build,
                    Some("dev-dependencies") => DependencyKind::Dev,
                    _ => return Err(format!("{manifest}: unsupported dependency section")),
                };
                let table = table
                    .as_table_like()
                    .ok_or_else(|| format!("{manifest}: invalid dependency table"))?;
                for (name, declaration) in table.iter() {
                    let inherited =
                        declaration.get("workspace").and_then(Item::as_bool) == Some(true);
                    let (document, mut keys, dependency) = if inherited {
                        let root = &self.documents[&package.workspace];
                        let dependency = root
                            .get("workspace")
                            .and_then(|w| w.get("dependencies"))
                            .and_then(|d| d.get(name))
                            .ok_or_else(|| {
                                format!("{manifest}: inherited dependency {name} is missing")
                            })?;
                        (
                            package.workspace.clone(),
                            vec!["workspace".into(), "dependencies".into()],
                            dependency,
                        )
                    } else {
                        (manifest.clone(), section.clone(), declaration)
                    };
                    keys.push(name.into());
                    let Some(path) = dependency.get("path").and_then(Item::as_str) else {
                        continue;
                    };
                    let bool_field =
                        |item: &Item, key: &str, default: bool| -> Result<bool, String> {
                            item.get(key).map_or(Ok(default), |v| {
                                v.as_bool().ok_or_else(|| {
                                    format!("{manifest}: dependency {name}.{key} must be boolean")
                                })
                            })
                        };
                    if inherited && bool_field(dependency, "optional", false)? {
                        return Err(format!(
                            "{document}: workspace dependency {name} cannot be optional"
                        ));
                    }
                    let optional = bool_field(declaration, "optional", false)?;
                    if optional && kind == DependencyKind::Dev {
                        return Err(format!(
                            "{manifest}: dev dependency {name} cannot be optional"
                        ));
                    }
                    let mut default_features = bool_field(dependency, "default-features", true)?;
                    if inherited && declaration.get("default-features").is_some() {
                        let member_default = bool_field(declaration, "default-features", true)?;
                        if default_features && !member_default {
                            return Err(format!(
                                "{manifest}: cannot disable inherited default-features for {name}; declare false in workspace.dependencies"
                            ));
                        }
                        default_features |= member_default;
                    }
                    let mut features = strings(
                        dependency.get("features"),
                        &format!("{document}: dependency {name}.features"),
                    )?;
                    if inherited {
                        features.extend(strings(
                            declaration.get("features"),
                            &format!("{manifest}: dependency {name}.features"),
                        )?);
                    }
                    features.sort();
                    features.dedup();
                    if dependency.get("git").is_some() {
                        return Err(format!(
                            "{document}: local dependency {name} also declares git"
                        ));
                    }
                    let target = manifest_at(&normalize(directory(&document), path)?);
                    let target_package = self.packages.get(&target).ok_or_else(|| format!("{manifest}: local dependency {name} is not in registered workspaces: {target}"))?;
                    let expected = dependency
                        .get("package")
                        .and_then(Item::as_str)
                        .unwrap_or(name);
                    if expected != target_package.name {
                        return Err(format!(
                            "{document}: dependency {name} package does not match {target}"
                        ));
                    }
                    if let Some(requirement) = dependency.get("version") {
                        let requirement = requirement
                            .as_str()
                            .ok_or_else(|| format!("{document}: invalid version for {name}"))?;
                        if !accepts(requirement, target_package.version)? {
                            return Err(format!(
                                "{document}: dependency {name} requirement {requirement} does not accept {}",
                                target_package.version
                            ));
                        }
                    }
                    self.dependencies.push(Dependency {
                        owner: manifest.clone(),
                        target,
                        kind,
                        alias: name.to_string(),
                        optional,
                        features,
                        default_features,
                        document,
                        keys,
                    });
                }
            }
        }
        Ok(())
    }

    fn validate_locks(&self) -> Result<(), String> {
        for (root, members) in &self.roots {
            let lock = if directory(root).is_empty() {
                "Cargo.lock".to_owned()
            } else {
                format!("{}/Cargo.lock", directory(root))
            };
            let Some(document) = self.documents.get(&lock) else {
                continue;
            };
            if !matches!(
                document.get("version").and_then(Item::as_integer),
                Some(3 | 4)
            ) {
                return Err(format!(
                    "{lock}: Rust release supports only Cargo.lock format version 3 or 4"
                ));
            }
            let entries = document
                .get("package")
                .and_then(Item::as_array_of_tables)
                .ok_or_else(|| format!("{lock}: missing package entries"))?;
            let closure = self.required_lock_closure(members);
            for manifest in &closure {
                let package = &self.packages[manifest];
                let matches = entries
                    .iter()
                    .filter(|entry| {
                        entry.get("source").is_none()
                            && entry.get("name").and_then(Item::as_str)
                                == Some(package.name.as_str())
                    })
                    .collect::<Vec<_>>();
                if matches.len() != 1
                    || matches[0].get("version").and_then(Item::as_str)
                        != Some(package.version.to_string().as_str())
                {
                    return Err(format!(
                        "{lock}: local package {} has missing, ambiguous, or stale version",
                        package.name
                    ));
                }
            }
            for entry in entries {
                if entry.get("source").is_some() {
                    continue;
                }
                let name = entry
                    .get("name")
                    .and_then(Item::as_str)
                    .ok_or_else(|| format!("{lock}: local lock entry has no name"))?;
                let version = entry
                    .get("version")
                    .and_then(Item::as_str)
                    .ok_or_else(|| format!("{lock}: local lock entry has no version"))?;
                let matches = self
                    .packages
                    .values()
                    .filter(|p| p.name == name && p.version.to_string() == version)
                    .count();
                if matches != 1 {
                    return Err(format!(
                        "{lock}: local lock package {name} {version} has no unique registered identity"
                    ));
                }
            }
        }
        Ok(())
    }

    /// Resolve only the local feature implications needed to prove lock entries
    /// mandatory. Registry feature resolution remains Cargo's responsibility.
    /// Workspace members are locked with all features; external path packages
    /// receive only defaults and features enabled by their incoming dependencies.
    fn required_lock_closure(&self, members: &BTreeSet<String>) -> BTreeSet<String> {
        let mut closure = members.clone();
        let mut features = members
            .iter()
            .map(|manifest| {
                (
                    manifest.clone(),
                    self.packages[manifest]
                        .features
                        .keys()
                        .cloned()
                        .collect::<BTreeSet<_>>(),
                )
            })
            .collect::<BTreeMap<_, _>>();
        loop {
            let mut changed = false;
            for owner in closure.clone() {
                let empty = BTreeSet::new();
                let (optional, requests) =
                    self.local_feature_requests(&owner, features.get(&owner).unwrap_or(&empty));
                for dependency in self.dependencies.iter().filter(|d| d.owner == owner) {
                    if dependency.kind == DependencyKind::Dev && !members.contains(&owner) {
                        continue;
                    }
                    if dependency.optional
                        && !members.contains(&owner)
                        && !optional.contains(&dependency.alias)
                    {
                        continue;
                    }
                    changed |= closure.insert(dependency.target.clone());
                    let enabled = features.entry(dependency.target.clone()).or_default();
                    for feature in &dependency.features {
                        changed |= enabled.insert(feature.clone());
                    }
                    if dependency.default_features {
                        changed |= enabled.insert("default".into());
                    }
                    if let Some(requests) = requests.get(&dependency.alias) {
                        for feature in requests {
                            changed |= enabled.insert(feature.clone());
                        }
                    }
                }
            }
            if !changed {
                break;
            }
        }
        closure
    }

    fn local_feature_requests(
        &self,
        owner: &str,
        requested: &BTreeSet<String>,
    ) -> (BTreeSet<String>, BTreeMap<String, BTreeSet<String>>) {
        let package = &self.packages[owner];
        let mut enabled = requested.clone();
        let mut optional = BTreeSet::new();
        let mut dependency_features = BTreeMap::<String, BTreeSet<String>>::new();
        loop {
            let mut changed = false;
            for feature in enabled.clone() {
                if let Some(expressions) = package.features.get(&feature) {
                    for expression in expressions {
                        if let Some(alias) = expression.strip_prefix("dep:") {
                            changed |= optional.insert(alias.to_string());
                        } else if let Some((alias, target_feature)) = expression.split_once('/') {
                            let weak = alias.ends_with('?');
                            let alias = alias.trim_end_matches('?');
                            if !weak {
                                changed |= optional.insert(alias.to_string());
                            }
                            let required = self
                                .dependencies
                                .iter()
                                .any(|d| d.owner == owner && d.alias == alias && !d.optional);
                            if !weak || required || optional.contains(alias) {
                                changed |= dependency_features
                                    .entry(alias.to_string())
                                    .or_default()
                                    .insert(target_feature.to_string());
                            }
                        } else {
                            changed |= enabled.insert(expression.clone());
                        }
                    }
                } else if self
                    .dependencies
                    .iter()
                    .any(|d| d.owner == owner && d.alias == feature && d.optional)
                {
                    // Cargo's implicit feature named after an optional dependency.
                    changed |= optional.insert(feature);
                }
            }
            if !changed {
                break;
            }
        }
        (optional, dependency_features)
    }

    pub(super) fn plan(&self, events: &[(CommitInfo, Vec<String>)]) -> Result<AdapterPlan, String> {
        let mut versions = BTreeMap::<String, SemVer>::new();
        let mut direct = BTreeSet::<String>::new();
        for (commit, paths) in events {
            for path in paths {
                for manifest in self.affected(path)? {
                    let old = self.packages[&manifest].version;
                    let component = if commit.breaking {
                        old.major
                    } else if commit.kind == "feat" {
                        old.minor
                    } else {
                        old.patch
                    };
                    if component == u64::MAX {
                        return Err(format!("{manifest}: Rust package version overflow"));
                    }
                    let next = bump(old, commit);
                    versions
                        .entry(manifest.clone())
                        .and_modify(|v| *v = (*v).max(next))
                        .or_insert(next);
                    direct.insert(manifest);
                }
            }
        }
        loop {
            let mut changed = false;
            for dependency in &self.dependencies {
                if versions.contains_key(&dependency.target)
                    && !versions.contains_key(&dependency.owner)
                {
                    let old = self.packages[&dependency.owner].version;
                    let next = SemVer {
                        patch: old
                            .patch
                            .checked_add(1)
                            .ok_or("Rust package version overflow")?,
                        ..old
                    };
                    versions.insert(dependency.owner.clone(), next);
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        let mut documents = self.documents.clone();
        let mut touched = BTreeSet::new();
        // Literal versions split inherited workspace versions without bumping
        // unaffected packages. Future releases cannot silently reunify them.
        for (manifest, package) in &self.packages {
            if package.inherited || versions.contains_key(manifest) {
                documents.get_mut(manifest).unwrap()["package"]["version"] = value(
                    versions
                        .get(manifest)
                        .unwrap_or(&package.version)
                        .to_string(),
                );
                touched.insert(manifest.clone());
            }
        }
        for dependency in &self.dependencies {
            let Some(next) = versions.get(&dependency.target) else {
                continue;
            };
            let document = documents.get_mut(&dependency.document).unwrap();
            let mut item = document.as_item_mut();
            for key in &dependency.keys {
                item = item
                    .get_mut(key)
                    .ok_or_else(|| format!("{}: dependency path changed", dependency.document))?;
            }
            if let Some(requirement) = item.get("version").and_then(Item::as_str) {
                if !accepts(requirement, *next)? {
                    let updated = updated_requirement(requirement, *next)?;
                    if let Some(inline) = item.as_inline_table_mut() {
                        inline.insert("version", updated.into());
                    } else if let Some(table) = item.as_table_mut() {
                        table.insert("version", value(updated));
                    } else {
                        return Err("local dependency must be a table".into());
                    }
                    touched.insert(dependency.document.clone());
                }
            }
        }
        self.render_locks(&versions, &mut documents, &mut touched)?;
        let mut writes = BTreeMap::new();
        for path in touched {
            let bytes = documents[&path].to_string().into_bytes();
            if self.inputs.get(&path).and_then(Option::as_ref) != Some(&bytes) {
                writes.insert(path, bytes);
            }
        }
        let packages = versions
            .iter()
            .map(|(manifest, next)| {
                let package = &self.packages[manifest];
                let dependencies = self
                    .dependencies
                    .iter()
                    .filter(|d| d.owner == *manifest && versions.contains_key(&d.target))
                    .map(|d| d.target.clone())
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect::<Vec<_>>();
                PackageChange {
                    manifest: manifest.clone(),
                    name: package.name.clone(),
                    version_before: package.version.to_string(),
                    version_after: next.to_string(),
                    reason: if direct.contains(manifest) {
                        "direct_change".into()
                    } else {
                        "dependency_change".into()
                    },
                    dependencies,
                }
            })
            .collect();
        Ok(AdapterPlan {
            inputs: self.inputs.clone(),
            writes,
            packages,
        })
    }

    fn render_locks(
        &self,
        versions: &BTreeMap<String, SemVer>,
        documents: &mut BTreeMap<String, DocumentMut>,
        touched: &mut BTreeSet<String>,
    ) -> Result<(), String> {
        for lock in &self.locks {
            let Some(document) = documents.get_mut(lock) else {
                continue;
            };
            let entries = document
                .get_mut("package")
                .and_then(Item::as_array_of_tables_mut)
                .ok_or_else(|| format!("{lock}: invalid package entries"))?;
            // Identity comes from a source-less lock entry, never a name-only
            // replacement; registry packages with identical versions survive.
            let mut replacements = BTreeMap::new();
            for entry in entries.iter_mut() {
                if entry.get("source").is_some() {
                    continue;
                }
                let name = entry
                    .get("name")
                    .and_then(Item::as_str)
                    .unwrap_or("")
                    .to_string();
                let old = entry
                    .get("version")
                    .and_then(Item::as_str)
                    .unwrap_or("")
                    .to_string();
                let manifest = self
                    .packages
                    .iter()
                    .find(|(_, p)| p.name == name && p.version.to_string() == old)
                    .map(|(m, _)| m);
                if let Some(next) = manifest.and_then(|m| versions.get(m)) {
                    replacements.insert((name, old), next.to_string());
                    entry.insert("version", value(next.to_string()));
                    touched.insert(lock.clone());
                }
            }
            for entry in entries.iter_mut() {
                if let Some(dependencies) =
                    entry.get_mut("dependencies").and_then(Item::as_array_mut)
                {
                    for dependency in dependencies.iter_mut() {
                        let raw = dependency
                            .as_str()
                            .ok_or_else(|| format!("{lock}: invalid dependency identifier"))?;
                        let parts = raw.split_whitespace().collect::<Vec<_>>();
                        // Source-qualified registry/git dependencies are external.
                        if parts.len() == 2 {
                            if let Some(next) =
                                replacements.get(&(parts[0].into(), parts[1].into()))
                            {
                                *dependency = format!("{} {next}", parts[0]).into();
                                touched.insert(lock.clone());
                            }
                        } else if parts.len() > 2 && !parts[2].starts_with('(') {
                            return Err(format!("{lock}: unsupported dependency identifier {raw}"));
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

// Canonical semantic comparison without depending on serialization of Item
// (which retains trivia and formatting rather than TOML values).
fn toml_to_json(item: &Item) -> Result<serde_json::Value, String> {
    if let Some(table) = item.as_table_like() {
        let mut result = serde_json::Map::new();
        for (key, value) in table.iter() {
            result.insert(key.into(), toml_to_json(value)?);
        }
        return Ok(serde_json::Value::Object(result));
    }
    if let Some(value) = item.as_value() {
        return value_json(value);
    }
    if let Some(array) = item.as_array_of_tables() {
        return Ok(serde_json::Value::Array(
            array
                .iter()
                .map(|t| toml_to_json(&Item::Table(t.clone())))
                .collect::<Result<Vec<_>, _>>()?,
        ));
    }
    Ok(serde_json::Value::Null)
}

fn value_json(value: &toml_edit::Value) -> Result<serde_json::Value, String> {
    if let Some(v) = value.as_str() {
        return Ok(v.into());
    }
    if let Some(v) = value.as_bool() {
        return Ok(v.into());
    }
    if let Some(v) = value.as_integer() {
        return Ok(v.into());
    }
    if let Some(v) = value.as_float() {
        return Ok(serde_json::json!(v));
    }
    if let Some(v) = value.as_array() {
        return Ok(serde_json::Value::Array(
            v.iter().map(value_json).collect::<Result<Vec<_>, _>>()?,
        ));
    }
    if let Some(v) = value.as_inline_table() {
        let mut result = serde_json::Map::new();
        for (k, v) in v.iter() {
            result.insert(k.into(), value_json(v)?);
        }
        return Ok(serde_json::Value::Object(result));
    }
    if let Some(v) = value.as_datetime() {
        return Ok(v.to_string().into());
    }
    Err("unsupported TOML value".into())
}

fn comparator(raw: &str, version: SemVer) -> Result<bool, String> {
    let raw = raw.trim();
    if raw == "*" {
        return Ok(true);
    }
    let (operator, number) = [">=", "<=", ">", "<", "=", "^", "~"]
        .into_iter()
        .find_map(|op| raw.strip_prefix(op).map(|n| (op, n.trim())))
        .unwrap_or(("^", raw));
    let parts = number.split('.').collect::<Vec<_>>();
    if parts.is_empty() || parts.len() > 3 {
        return Err(format!("unsupported Cargo version requirement: {raw}"));
    }
    let mut numbers = Vec::new();
    let mut wildcard = false;
    for part in &parts {
        if matches!(*part, "*" | "x" | "X") {
            wildcard = true;
            break;
        }
        if part.is_empty()
            || !part.bytes().all(|b| b.is_ascii_digit())
            || (part.len() > 1 && part.starts_with('0'))
        {
            return Err(format!("unsupported Cargo version requirement: {raw}"));
        }
        numbers.push(
            part.parse::<u64>()
                .map_err(|_| format!("unsupported Cargo version requirement: {raw}"))?,
        );
    }
    if wildcard
        && parts[numbers.len()..]
            .iter()
            .any(|p| !matches!(*p, "*" | "x" | "X"))
    {
        return Err(format!("unsupported Cargo version requirement: {raw}"));
    }
    if numbers.is_empty() {
        if operator != "^" && operator != "=" {
            return Err(format!("unsupported Cargo version requirement: {raw}"));
        }
        return Ok(true);
    }
    if wildcard && !matches!(operator, "^" | "=") {
        return Err(format!("unsupported Cargo version requirement: {raw}"));
    }
    let lower = SemVer {
        major: numbers[0],
        minor: *numbers.get(1).unwrap_or(&0),
        patch: *numbers.get(2).unwrap_or(&0),
    };
    let upper =
        if wildcard || operator == "~" || (matches!(operator, ">" | "<=") && numbers.len() < 3) {
            if numbers.len() == 1 {
                SemVer {
                    major: lower.major.checked_add(1).ok_or("version bound overflow")?,
                    minor: 0,
                    patch: 0,
                }
            } else {
                SemVer {
                    minor: lower.minor.checked_add(1).ok_or("version bound overflow")?,
                    patch: 0,
                    ..lower
                }
            }
        } else if lower.major > 0 || numbers.len() == 1 {
            SemVer {
                major: lower.major.checked_add(1).ok_or("version bound overflow")?,
                minor: 0,
                patch: 0,
            }
        } else if lower.minor > 0 || numbers.len() == 2 {
            SemVer {
                minor: lower.minor.checked_add(1).ok_or("version bound overflow")?,
                patch: 0,
                ..lower
            }
        } else {
            SemVer {
                patch: lower.patch.checked_add(1).ok_or("version bound overflow")?,
                ..lower
            }
        };
    Ok(match operator {
        ">=" => version >= lower,
        ">" => {
            if numbers.len() == 3 {
                version > lower
            } else {
                version >= upper
            }
        }
        "<" => version < lower,
        "<=" => {
            if numbers.len() == 3 {
                version <= lower
            } else {
                version < upper
            }
        }
        "=" if !wildcard => {
            version.major == lower.major
                && (numbers.len() < 2 || version.minor == lower.minor)
                && (numbers.len() < 3 || version.patch == lower.patch)
        }
        _ => version >= lower && version < upper,
    })
}

fn accepts(raw: &str, version: SemVer) -> Result<bool, String> {
    let mut accepted = true;
    for component in raw.split(',') {
        accepted &= comparator(component, version)?;
    }
    Ok(accepted)
}

fn updated_requirement(raw: &str, version: SemVer) -> Result<String, String> {
    if raw.contains(',') || raw.trim().starts_with(['>', '<']) {
        return Err(format!(
            "cannot safely widen internal Cargo requirement {raw}; configure an explicit supported requirement"
        ));
    }
    let prefix = if raw.trim().starts_with('=') {
        "="
    } else if raw.trim().starts_with('~') {
        "~"
    } else if raw.trim().starts_with('^') {
        "^"
    } else {
        ""
    };
    Ok(format!("{prefix}{version}"))
}

#[cfg(all(test, bridgeforge_factory_tests))]
#[path = "../../../../../../scripts/tests/unit/core_release_rust.rs"]
mod tests;
