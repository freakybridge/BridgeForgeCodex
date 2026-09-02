use serde_json::{Map, Value};
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::process::{ExitStatus, Output};
use std::time::Duration;

pub const EXIT_BLOCKED: i32 = 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ToolKind {
    Shell,
    Edit,
    Patch,
}

// All dispatcher and guard decisions must use this classification.
pub fn tool_kind(name: &str) -> Option<ToolKind> {
    match name {
        "Bash" | "PowerShell" | "shell_command" => Some(ToolKind::Shell),
        "Edit" | "Write" | "MultiEdit" | "NotebookEdit" => Some(ToolKind::Edit),
        "apply_patch" => Some(ToolKind::Patch),
        _ => None,
    }
}

pub fn repo_root() -> PathBuf {
    if let Some(value) = env::var_os("BRIDGEFORGE_HOOK_ROOT") {
        return PathBuf::from(value);
    }
    let executable =
        env::current_exe().unwrap_or_else(|_| PathBuf::from(".codex/bin/bridgeforge-hook"));
    executable
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

pub fn codex_root() -> PathBuf {
    repo_root().join(".codex")
}

pub fn read_stdin_bytes() -> Vec<u8> {
    let mut bytes = Vec::new();
    let _ = std::io::stdin().read_to_end(&mut bytes);
    bytes
}

pub fn parse_payload(raw: &[u8]) -> Result<Value, String> {
    if raw.iter().all(u8::is_ascii_whitespace) {
        return Err("hook input is empty".into());
    }
    let text = std::str::from_utf8(raw).map_err(|_| "hook input is not valid UTF-8 JSON")?;
    let value: Value =
        serde_json::from_str(text).map_err(|_| "hook input is not valid UTF-8 JSON")?;
    if !value.is_object() {
        return Err("hook input must be a JSON object".into());
    }
    Ok(value)
}

pub fn tool_name(payload: &Value) -> String {
    let primary = payload
        .get("tool_name")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();
    if primary.is_empty() {
        payload
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_string()
    } else {
        primary.to_string()
    }
}

pub fn tool_input(payload: &Value) -> Option<&Map<String, Value>> {
    payload.get("tool_input").and_then(Value::as_object)
}

pub fn json_string(map: &Map<String, Value>, key: &str) -> String {
    map.get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

pub fn normalize_path(root: &Path, raw: &str) -> PathBuf {
    let stripped = raw.trim().trim_matches(['\'', '"']);
    let path = PathBuf::from(stripped);
    let joined = if path.is_absolute() {
        path
    } else {
        root.join(path)
    };
    let lexical = lexical_normalize(&joined);
    canonicalize_existing_ancestor(&lexical).unwrap_or(lexical)
}

fn canonicalize_existing_ancestor(path: &Path) -> Option<PathBuf> {
    let mut current = path;
    let mut suffix = Vec::new();
    while !current.exists() {
        suffix.push(current.file_name()?.to_os_string());
        current = current.parent()?;
    }
    let mut resolved = fs::canonicalize(current).ok()?;
    for part in suffix.into_iter().rev() {
        resolved.push(part);
    }
    Some(lexical_normalize(&resolved))
}

pub fn lexical_normalize(path: &Path) -> PathBuf {
    let mut result = PathBuf::new();
    for part in path.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => {
                if !result.pop() {
                    result.push(part.as_os_str());
                }
            }
            _ => result.push(part.as_os_str()),
        }
    }
    result
}

pub fn path_is_inside(root: &Path, candidate: &Path) -> bool {
    let root = canonicalize_existing_ancestor(root).unwrap_or_else(|| lexical_normalize(root));
    let candidate =
        canonicalize_existing_ancestor(candidate).unwrap_or_else(|| lexical_normalize(candidate));
    #[cfg(windows)]
    {
        let root = root.to_string_lossy().replace('/', "\\").to_lowercase();
        let candidate = candidate
            .to_string_lossy()
            .replace('/', "\\")
            .to_lowercase();
        candidate == root || candidate.starts_with(&(root + "\\"))
    }
    #[cfg(not(windows))]
    {
        candidate == root || candidate.starts_with(root)
    }
}

pub fn relative_display(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

pub fn read_utf8(path: &Path) -> std::io::Result<String> {
    let bytes = fs::read(path)?;
    let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(&bytes);
    Ok(String::from_utf8_lossy(bytes)
        .replace("\r\n", "\n")
        .replace('\r', "\n"))
}

pub fn run_command<I, S>(
    program: &str,
    args: I,
    cwd: &Path,
    timeout: Duration,
) -> std::io::Result<Output>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    use bridgeforge_core::{ProcessRequest, ProcessRunner, SystemProcessRunner};
    let mut request = ProcessRequest::new(program, cwd);
    request.args = args
        .into_iter()
        .map(|value| value.as_ref().to_os_string())
        .collect();
    request.timeout = timeout;
    let output = SystemProcessRunner.run(&request)?;
    if output.timed_out {
        return Err(std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            "hook command timed out",
        ));
    }
    #[cfg(windows)]
    let status = {
        use std::os::windows::process::ExitStatusExt;
        ExitStatus::from_raw(output.code as u32)
    };
    #[cfg(unix)]
    let status = {
        use std::os::unix::process::ExitStatusExt;
        ExitStatus::from_raw(output.code << 8)
    };
    Ok(Output {
        status,
        stdout: output.stdout,
        stderr: output.stderr,
    })
}

pub fn atomic_write(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    bridgeforge_core::memory::atomic_write(path, bytes).map_err(std::io::Error::other)
}
