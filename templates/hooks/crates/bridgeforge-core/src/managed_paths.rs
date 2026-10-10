//! Managed asset path policy, shared by planning and isolated generated builds.
//! Other path policies remain with their owners; do not broaden this policy here.
use std::fs;
use std::path::{Component, Path, PathBuf};

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
