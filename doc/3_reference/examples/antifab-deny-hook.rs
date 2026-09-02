//! C1 "confirmed missing => deny" reference Hook. This file is not registered by the factory.
//! A downstream project may copy the pure `evaluate` logic into its own Rust Hook bundle after
//! filling `EXEMPT_PREFIXES` and `REAL_SOURCE_HINT`.

use serde_json::{Value, json};
use std::io::Read;
use std::path::{Path, PathBuf};

const READ_TOOLS: &[&str] = &["Read"];
const EXEMPT_PREFIXES: &[&str] = &[];
const REAL_SOURCE_HINT: &str =
    "若这是项目数据，请改用项目约定的权威数据源，而非凭空假设的本地文件。";

fn evaluate(tool_name: &str, input: &Value) -> Option<String> {
    if !READ_TOOLS.contains(&tool_name) {
        return None;
    }
    let raw = input.get("file_path")?.as_str()?;
    if raw.is_empty()
        || raw.contains("://")
        || raw.contains(['*', '?', '[', ']'])
        || raw.ends_with(['/', '\\'])
        || raw.contains(['$', '%'])
    {
        return None;
    }
    let expanded = if let Some(tail) = raw.strip_prefix("~/") {
        std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(PathBuf::from)?
            .join(tail)
    } else {
        PathBuf::from(raw)
    };
    let absolute = if expanded.is_absolute() {
        expanded
    } else {
        std::env::current_dir().ok()?.join(expanded)
    };
    if absolute.exists() || absolute.is_dir() {
        return None;
    }
    let normalized = absolute.to_string_lossy().replace('\\', "/");
    if EXEMPT_PREFIXES.iter().any(|prefix| {
        Path::new(prefix)
            .canonicalize()
            .ok()
            .is_some_and(|path| normalized.starts_with(&path.to_string_lossy().replace('\\', "/")))
    }) {
        return None;
    }
    Some(format!(
        "该路径确证不存在。通道或路径错误不等于资源缺失；不要编造数据源。{REAL_SOURCE_HINT}（路径：{}）",
        absolute.display()
    ))
}

fn main() {
    let mut raw = String::new();
    let _ = std::io::stdin().read_to_string(&mut raw);
    let payload: Value = serde_json::from_str(&raw).unwrap_or(Value::Null);
    let tool_name = payload.get("tool_name").and_then(Value::as_str).unwrap_or("");
    let input = payload.get("tool_input").unwrap_or(&Value::Null);
    if let Some(reason) = evaluate(tool_name, input) {
        println!(
            "{}",
            json!({"hookSpecificOutput": {
                "hookEventName": "PreToolUse",
                "permissionDecision": "deny",
                "permissionDecisionReason": reason
            }})
        );
    }
}
