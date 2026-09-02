pub mod archive_scan;
pub mod asset_migration;
pub mod audit_user_allow;
pub mod baseline;
pub mod batch;
pub mod factory_version;
mod file_lock;
pub mod git_sync;
pub mod manifest;
pub mod memory;
mod process;
pub mod project_structure;
pub mod project_sync;
pub mod proposal_contract;
pub mod release;
pub mod runtime;
pub mod skill_metadata;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};

pub use process::{ProcessOutput, ProcessRequest, ProcessRunner, SystemProcessRunner};

pub const EXIT_OK: i32 = 0;
pub const EXIT_BLOCKED: i32 = 2;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CommandOutcome {
    pub code: i32,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub stdout: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub stderr: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receipt: Option<Value>,
}

impl CommandOutcome {
    pub fn ok() -> Self {
        Self::default()
    }

    pub fn blocked(message: impl Into<String>) -> Self {
        Self {
            code: EXIT_BLOCKED,
            stderr: message.into(),
            ..Self::default()
        }
    }

    pub fn with_receipt(receipt: Value) -> Self {
        Self {
            receipt: Some(receipt),
            ..Self::default()
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectContext {
    root: PathBuf,
    codex_root: PathBuf,
}

impl ProjectContext {
    pub fn discover(explicit_root: Option<&Path>) -> Result<Self, String> {
        let root = match explicit_root {
            Some(path) => path.to_path_buf(),
            None => std::env::current_dir()
                .map_err(|error| format!("cannot read current directory: {error}"))?,
        };
        let root = root
            .canonicalize()
            .map_err(|error| format!("project root is unavailable: {}: {error}", root.display()))?;
        if !root.is_dir() {
            return Err(format!(
                "project root is not a directory: {}",
                root.display()
            ));
        }
        Ok(Self {
            codex_root: root.join(".codex"),
            root,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn codex_root(&self) -> &Path {
        &self.codex_root
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BaselineState {
    Clean,
    Transition,
    Blocked,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BaselineReport {
    pub schema: u32,
    pub state: BaselineState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub incoming_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fingerprint: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reasons: Vec<String>,
    #[serde(default)]
    pub details: Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AssetContract {
    pub schema: u32,
    pub product: String,
    pub skeleton_version: String,
    #[serde(default)]
    pub document: Value,
}

#[cfg(all(test, bridgeforge_factory_tests))]
#[path = "../../../../../scripts/tests/unit/core_lib.rs"]
mod tests;
