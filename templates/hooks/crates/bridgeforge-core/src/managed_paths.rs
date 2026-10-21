//! Managed asset path policy, shared by planning and isolated generated builds.
//! Other path policies remain with their owners; do not broaden this policy here.
use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

/// Bounded metadata reads must not follow a leaf reparse point or allocate from
/// an untrusted file length. Missing is distinct from unreadable/invalid.
pub(crate) fn read_metadata(path: &Path, limit: u64) -> Result<Option<Vec<u8>>, String> {
    for ancestor in path.ancestors().filter(|p| p.exists()) {
        if crate::persistence::is_link_or_reparse(ancestor).map_err(|e| e.to_string())? {
            return Err(format!(
                "metadata traverses a linked path: {}",
                path.display()
            ));
        }
    }
    let metadata = match fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.to_string()),
    };
    if !metadata.is_file() || metadata.len() > limit {
        return Err(format!(
            "metadata is not a bounded regular file: {}",
            path.display()
        ));
    }
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > limit
        || crate::persistence::is_link_or_reparse(path).map_err(|e| e.to_string())?
    {
        return Err("metadata changed during capture".into());
    }
    Ok(Some(bytes))
}

pub(crate) fn safe_join(root: &Path, raw: &str, label: &str) -> Result<PathBuf, String> {
    if raw.is_empty() || raw.contains('\\') || raw.contains(['*', '?', '[']) {
        return Err(format!("{label} is unsafe: {raw}"));
    }
    let relative = Path::new(raw);
    if relative.is_absolute()
        || relative.components().any(|part| {
            matches!(
                part,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(format!("{label} is unsafe: {raw}"));
    }
    let target = root.join(relative);
    let mut cursor = target.parent();
    while let Some(path) = cursor {
        if path == root {
            break;
        }
        if path.exists()
            && fs::symlink_metadata(path)
                .map_err(|error| error.to_string())?
                .file_type()
                .is_symlink()
        {
            return Err(format!("{label} traverses a linked path: {raw}"));
        }
        cursor = path.parent();
    }
    Ok(target)
}
