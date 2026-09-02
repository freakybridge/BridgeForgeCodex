use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ArchiveCandidate {
    pub kind: String,
    pub topic: String,
    pub path: PathBuf,
    pub lifecycle: String,
}

fn frontmatter(text: &str) -> BTreeMap<String, String> {
    let mut fields = BTreeMap::new();
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut lines = normalized.lines();
    if lines.next() != Some("---") {
        return fields;
    }
    for line in lines {
        if line == "---" {
            break;
        }
        if let Some((key, value)) = line.split_once(':') {
            fields.insert(
                key.trim().to_string(),
                value.trim().trim_matches('"').to_string(),
            );
        }
    }
    fields
}

fn visible(entry: &DirEntry) -> bool {
    let name = entry.file_name().to_string_lossy();
    if entry.depth() == 0 {
        return true;
    }
    !name.starts_with('.') && name != "evidence" && name != "artifacts"
}

fn package_lifecycle(root: &Path) -> Result<Option<String>, String> {
    let mut states = Vec::new();
    for entry in WalkDir::new(root)
        .max_depth(2)
        .follow_links(false)
        .into_iter()
        .filter_entry(visible)
    {
        let entry = entry.map_err(|error| format!("cannot scan {}: {error}", root.display()))?;
        if entry.file_type().is_symlink() {
            return Err(format!(
                "linked archive candidate is forbidden: {}",
                entry.path().display()
            ));
        }
        if entry.file_type().is_file()
            && entry.path().extension().and_then(|x| x.to_str()) == Some("md")
        {
            let text = fs::read_to_string(entry.path())
                .map_err(|error| format!("cannot read {}: {error}", entry.path().display()))?;
            if let Some(value) = frontmatter(&text).get("lifecycle") {
                states.push(value.clone());
            }
        }
    }
    if states.iter().any(|value| value == "active") {
        return Ok(Some("active".into()));
    }
    let retired = states
        .iter()
        .filter(|value| matches!(value.as_str(), "completed" | "superseded"))
        .cloned()
        .collect::<Vec<_>>();
    if retired.is_empty() {
        Ok(None)
    } else if retired.iter().all(|value| value == &retired[0]) {
        Ok(Some(retired[0].clone()))
    } else {
        Ok(Some("completed".into()))
    }
}

fn scan_layer(root: &Path, layer: &str, kind: &str) -> Result<Vec<ArchiveCandidate>, String> {
    let base = root.join("doc").join(layer);
    if !base.is_dir() {
        return Ok(Vec::new());
    }
    let mut candidates = Vec::new();
    for entry in
        fs::read_dir(&base).map_err(|error| format!("cannot scan {}: {error}", base.display()))?
    {
        let entry = entry.map_err(|error| format!("cannot scan {}: {error}", base.display()))?;
        let file_type = entry.file_type().map_err(|error| error.to_string())?;
        if file_type.is_symlink() {
            return Err(format!(
                "linked archive package is forbidden: {}",
                entry.path().display()
            ));
        }
        if !file_type.is_dir() {
            continue;
        }
        let topic = entry.file_name().to_string_lossy().to_string();
        if topic.starts_with('.') {
            continue;
        }
        if let Some(lifecycle) = package_lifecycle(&entry.path())?
            && matches!(lifecycle.as_str(), "completed" | "superseded")
        {
            candidates.push(ArchiveCandidate {
                kind: kind.into(),
                topic,
                path: entry.path(),
                lifecycle,
            });
        }
    }
    candidates.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(candidates)
}

pub fn scan(root: &Path) -> Result<Vec<ArchiveCandidate>, String> {
    let mut values = scan_layer(root, "1_delivery", "delivery")?;
    values.extend(scan_layer(root, "2_bugs", "bug")?);
    Ok(values)
}

#[cfg(all(test, bridgeforge_factory_tests))]
#[path = "../../../../../scripts/tests/unit/core_archive_scan.rs"]
mod tests;
