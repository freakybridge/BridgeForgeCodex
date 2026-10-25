use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProposalReport {
    pub schema: u32,
    pub healthy: bool,
    pub files: usize,
    pub issues: Vec<String>,
}

pub fn validate(root: &Path) -> ProposalReport {
    let mut issues = Vec::new();
    let mut files = 0;
    for entry in WalkDir::new(root).follow_links(false) {
        let Ok(entry) = entry else {
            issues.push("proposal tree cannot be traversed".into());
            break;
        };
        if entry.file_type().is_symlink() {
            issues.push(format!(
                "proposal contains linked path: {}",
                entry.path().display()
            ));
            continue;
        }
        if !entry.file_type().is_file() {
            continue;
        }
        files += 1;
        let extension = entry
            .path()
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        if extension == "py" {
            issues.push(format!(
                "proposal validator still depends on Python: {}",
                entry.path().display()
            ));
        }
        if extension == "md" {
            match fs::read_to_string(entry.path()) {
                Ok(text) if text.trim().is_empty() => issues.push(format!(
                    "proposal document is empty: {}",
                    entry.path().display()
                )),
                Err(error) => issues.push(format!(
                    "proposal document is not readable UTF-8: {}: {error}",
                    entry.path().display()
                )),
                _ => {}
            }
        }
    }
    ProposalReport {
        schema: 1,
        healthy: issues.is_empty(),
        files,
        issues,
    }
}
