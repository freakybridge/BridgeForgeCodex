use chrono::Local;
use regex::Regex;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use walkdir::WalkDir;

use crate::StepResult;
use crate::post::source_tree_hash;
use crate::util::{atomic_write, codex_root, read_utf8, repo_root, run_command};

fn git(args: &[String], timeout: Duration) -> Option<std::process::Output> {
    let root = repo_root();
    let mut safe_args = vec![
        "-c".to_string(),
        format!(
            "safe.directory={}",
            root.to_string_lossy().replace('\\', "/")
        ),
    ];
    safe_args.extend(args.iter().cloned());
    run_command("git", safe_args, &root, timeout).ok()
}

fn skeleton_version() -> String {
    fs::read_to_string(codex_root().join(".bridgeforge_codex_version"))
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "?".into())
}

fn active_work(root: &Path) -> String {
    let delivery = WalkDir::new(root.join("doc/1_delivery"))
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| {
            entry.file_type().is_file()
                && entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("requirements_")
                && entry.path().extension().and_then(|value| value.to_str()) == Some("md")
        })
        .count();
    let bugs = WalkDir::new(root.join("doc/2_bugs"))
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| {
            entry.file_type().is_file()
                && entry.file_name().to_string_lossy().starts_with("BUG-")
                && entry.path().extension().and_then(|value| value.to_str()) == Some("md")
        })
        .count();
    format!("delivery 确认卡: {delivery}；未归档 Bug: {bugs}")
}

fn snapshot_paths(directory: &Path) -> Vec<PathBuf> {
    let mut items: Vec<_> = fs::read_dir(directory)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("md"))
        .collect();
    items.sort_by_key(|path| {
        fs::metadata(path)
            .and_then(|value| value.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH)
    });
    items.reverse();
    items
}

pub fn snapshot(event: &str) -> StepResult {
    let root = repo_root();
    let directory = root.join(".runtime/session_state");
    if let Err(error) = fs::create_dir_all(&directory) {
        return StepResult::failed("session snapshot", error);
    }
    if event == "stop" {
        if let Some(latest) = snapshot_paths(&directory).first()
            && let Ok(modified) = fs::metadata(latest).and_then(|value| value.modified())
            && modified
                .elapsed()
                .is_ok_and(|age| age < Duration::from_secs(300))
        {
            return StepResult::ok();
        }
    }
    let timestamp = Local::now().format("%Y-%m-%d_%H%M%S").to_string();
    let branch = git(
        &["branch".into(), "--show-current".into()],
        Duration::from_secs(10),
    )
    .filter(|value| value.status.success())
    .map(|value| String::from_utf8_lossy(&value.stdout).trim().to_string())
    .unwrap_or_else(|| "?".into());
    let ahead = git(
        &[
            "rev-list".into(),
            "--left-right".into(),
            "--count".into(),
            "HEAD...@{u}".into(),
        ],
        Duration::from_secs(10),
    )
    .filter(|value| value.status.success())
    .map(|value| String::from_utf8_lossy(&value.stdout).trim().to_string())
    .unwrap_or_else(|| "no-upstream".into());
    let status = git(
        &["status".into(), "--short".into()],
        Duration::from_secs(10),
    )
    .filter(|value| value.status.success())
    .map(|value| String::from_utf8_lossy(&value.stdout).trim().to_string())
    .unwrap_or_default();
    let content = format!(
        "# Session State Snapshot\n\n**Timestamp**: {timestamp}\n**Event**: {event}\n**Branch**: {branch}\n**Ahead/Behind**: {ahead}\n**bridgeforge-codex Skeleton**: v{}\n\n## Uncommitted changes\n\n```\n{}\n```\n\n## 活跃交付与 Bug（唤起记忆）\n\n```\n{}\n```\n\n---\n\n由 `{event}` hook 自动生成。\n下次 session 接续可用 `$resume` 读最新一份。\n",
        skeleton_version(),
        if status.is_empty() {
            "(clean)"
        } else {
            &status
        },
        active_work(&root)
    );
    let path = directory.join(format!("{timestamp}.md"));
    if let Err(error) = atomic_write(&path, content.as_bytes()) {
        return StepResult::failed("session snapshot", error);
    }
    let paths = snapshot_paths(&directory);
    let trimmed = paths.len().saturating_sub(20);
    for old in paths.into_iter().skip(20) {
        if let Err(error) = fs::remove_file(old) {
            return StepResult::failed("session snapshot retention", error);
        }
    }
    if event == "stop" {
        StepResult::ok()
    } else {
        let suffix = if trimmed > 0 {
            format!(" (trimmed {trimmed} old)")
        } else {
            String::new()
        };
        StepResult::output(format!(
            "[session snapshot {event}] -> .runtime/session_state/{timestamp}.md{suffix}"
        ))
    }
}

fn hooks_table(text: &str) -> Result<bool, String> {
    let header = Regex::new(r#"(?m)^\s*\[+\s*(?P<body>[^\]\r\n]+)\]+\s*(?:#.*)?$"#).unwrap();
    for line in text.lines() {
        if line.trim_start().starts_with('[') && !line.contains(']') {
            return Err("unclosed table header".into());
        }
    }
    for captures in header.captures_iter(text) {
        let body = captures
            .name("body")
            .unwrap()
            .as_str()
            .trim()
            .replace(['\'', '"', ' '], "");
        if body == "hooks" || body.starts_with("hooks.") {
            return Ok(true);
        }
    }
    let dotted = Regex::new(r"(?m)^\s*hooks(?:\.[A-Za-z0-9_-]+)+\s*=").unwrap();
    let inline = Regex::new(r"(?m)^\s*hooks\s*=\s*\{").unwrap();
    Ok(dotted.is_match(text) || inline.is_match(text))
}

pub fn config_health(strict: bool) -> StepResult {
    let root = repo_root();
    let mut failures: Vec<(&str, String)> = Vec::new();
    let home = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .unwrap_or_default();
    for (label, path) in [
        ("~/.codex/settings.json", home.join(".codex/settings.json")),
        (".codex/settings.json", root.join(".codex/settings.json")),
        (
            ".codex/settings.local.json",
            root.join(".codex/settings.local.json"),
        ),
    ] {
        if path.is_file()
            && fs::read_to_string(&path)
                .ok()
                .and_then(|text| serde_json::from_str::<Value>(&text).ok())
                .is_none()
        {
            failures.push((
                "settings-json-valid",
                format!("settings.json invalid JSON: {label}. FIX: repair the JSON syntax."),
            ));
        }
    }
    let mut source = Vec::new();
    for (label, path) in [
        (".codex/settings.json", root.join(".codex/settings.json")),
        (
            ".codex/settings.local.json",
            root.join(".codex/settings.local.json"),
        ),
    ] {
        if let Ok(text) = fs::read_to_string(path)
            && serde_json::from_str::<Value>(&text)
                .ok()
                .and_then(|value| value.get("hooks").cloned())
                .is_some()
        {
            source.push(format!("{label} contains hooks"));
        }
    }
    let config = root.join(".codex/config.toml");
    if let Ok(text) = fs::read_to_string(config) {
        match hooks_table(&text) {
            Ok(true) => source.push(".codex/config.toml contains a hooks table".into()),
            Err(error) => source.push(format!(".codex/config.toml table header invalid ({error})")),
            _ => {}
        }
    }
    let hooks = root.join(".codex/hooks.json");
    if fs::read_to_string(hooks)
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .and_then(|value| value.get("hooks").cloned())
        .and_then(|value| value.as_object().cloned())
        .is_none()
    {
        source.push(".codex/hooks.json invalid JSON or has no hooks object".into());
    }
    if !source.is_empty() {
        failures.push(("single-hook-source", format!("Codex hook registration is not single-source: {}. FIX: merge project hooks into .codex/hooks.json and remove all other hook blocks.", source.join("; "))));
    }
    if failures.is_empty() {
        return StepResult::ok();
    }
    let mut stdout = format!(
        "[health-check] {} skeleton setting(s) need attention (check-only, nothing changed):\n",
        failures.len()
    );
    for (_, message) in &failures {
        stdout.push_str(&format!("  - {message}\n"));
    }
    let code = if strict
        && failures.iter().any(|(name, _)| {
            matches!(
                *name,
                "project-runtime" | "settings-json-valid" | "single-hook-source"
            )
        }) {
        2
    } else {
        0
    };
    StepResult {
        code,
        stdout,
        stderr: String::new(),
    }
}

pub fn enforce_no_effort() -> StepResult {
    let path = repo_root().join(".codex/settings.json");
    let Ok(text) = fs::read_to_string(&path) else {
        return StepResult::ok();
    };
    let Ok(mut value) = serde_json::from_str::<Value>(&text) else {
        return StepResult::ok();
    };
    let Some(map) = value.as_object_mut() else {
        return StepResult::ok();
    };
    let Some(removed) = map.shift_remove("effortLevel") else {
        return StepResult::ok();
    };
    if let Err(error) = atomic_write(&path.with_extension("json.bak"), text.as_bytes()) {
        return StepResult::failed("enforce-no-effortlevel backup", error);
    }
    let mut bytes = match serde_json::to_vec_pretty(&value) {
        Ok(bytes) => bytes,
        Err(error) => return StepResult::failed("enforce-no-effortlevel serialization", error),
    };
    bytes.push(b'\n');
    if let Err(error) = atomic_write(&path, &bytes) {
        return StepResult::failed("enforce-no-effortlevel", error);
    }
    StepResult::output(format!(
        "[enforce-no-effortlevel] removed project-level effortLevel={} from .codex/settings.json (effort is governed at user-global level only).",
        removed
    ))
}

pub fn githooks_path() -> StepResult {
    let root = repo_root();
    if !root.join(".git").exists() || !root.join(".githooks/pre-commit").is_file() {
        return StepResult::ok();
    }
    let current = git(
        &["config".into(), "--local".into(), "core.hooksPath".into()],
        Duration::from_secs(10),
    )
    .map(|value| {
        String::from_utf8_lossy(&value.stdout)
            .trim()
            .replace('\\', "/")
    })
    .unwrap_or_default();
    if current.trim_end_matches('/') == ".githooks" {
        return StepResult::ok();
    }
    let Some(result) = git(
        &[
            "config".into(),
            "--local".into(),
            "core.hooksPath".into(),
            ".githooks".into(),
        ],
        Duration::from_secs(10),
    ) else {
        return StepResult::failed("githooks", "Git could not start or timed out");
    };
    if result.status.success() {
        StepResult::output("[githooks] 已设 core.hooksPath=.githooks（提交前闸已生效）".into())
    } else {
        StepResult::failed("githooks", String::from_utf8_lossy(&result.stderr).trim())
    }
}

fn git_state() -> (String, usize, String) {
    let output = git(
        &["status".into(), "--porcelain=v2".into(), "--branch".into()],
        Duration::from_secs(5),
    );
    let text = output
        .filter(|value| value.status.success())
        .map(|value| String::from_utf8_lossy(&value.stdout).to_string())
        .unwrap_or_default();
    let mut branch = "?".to_string();
    let mut dirty = 0usize;
    let mut ahead = "no-upstream".to_string();
    for line in text.lines() {
        if let Some(value) = line.strip_prefix("# branch.head ") {
            if value != "(detached)" && !value.is_empty() {
                branch = value.into();
            }
        } else if let Some(value) = line.strip_prefix("# branch.ab ") {
            if let Some(caps) = Regex::new(r"^\+(\d+) -(\d+)$").unwrap().captures(value) {
                ahead = format!("{}/{}", &caps[1], &caps[2]);
            }
        } else if !line.starts_with("# ") && !line.is_empty() {
            dirty += 1;
        }
    }
    (branch, dirty, ahead)
}

fn archive_count(root: &Path) -> usize {
    let delivery = root.join("doc/1_delivery");
    let bugs = root.join("doc/2_bugs");
    let ready = Regex::new(r"(?im)^lifecycle:\s*(?:completed|superseded)\s*$").unwrap();
    let lifecycle =
        Regex::new(r"(?im)^lifecycle:\s*(?:active|completed|superseded|archived)\s*$").unwrap();
    let done = Regex::new(
        r"(?i)(?:状态|status)\s*[:：]\s*(?:已完成|已验收|已解决|done|accepted|resolved)",
    )
    .unwrap();

    let mut delivery_topics = std::collections::BTreeMap::<PathBuf, Vec<PathBuf>>::new();
    for entry in WalkDir::new(&delivery)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
    {
        if entry.file_type().is_file()
            && entry
                .file_name()
                .to_str()
                .is_some_and(|name| name.starts_with("requirements_") && name.ends_with(".md"))
        {
            if let Some(parent) = entry.path().parent() {
                delivery_topics
                    .entry(parent.to_path_buf())
                    .or_default()
                    .push(entry.path().to_path_buf());
            }
        }
    }
    let mut ready_topics = std::collections::BTreeSet::new();
    for (topic, cards) in delivery_topics {
        if cards.iter().all(|card| {
            read_utf8(card).is_ok_and(|text| {
                let frontmatter = text
                    .strip_prefix("---\n")
                    .and_then(|body| body.split_once("\n---\n"))
                    .map(|(head, _)| head)
                    .unwrap_or("");
                lifecycle.is_match(frontmatter) && ready.is_match(frontmatter)
            })
        }) {
            ready_topics.insert(topic);
        }
    }
    let mut count = ready_topics.len();
    for entry in WalkDir::new(&delivery)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
    {
        if entry.file_type().is_file() && entry.file_name() == "acceptance.md" {
            let Some(topic) = entry.path().parent() else {
                continue;
            };
            if ready_topics.contains(topic) {
                continue;
            }
            if read_utf8(entry.path()).is_ok_and(|text| {
                done.is_match(&text.lines().take(30).collect::<Vec<_>>().join("\n"))
            }) {
                count += 1;
            }
        }
    }
    if let Ok(entries) = fs::read_dir(&bugs) {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.starts_with("BUG-") {
                continue;
            }
            let evidence = if path.is_file()
                && path.extension().and_then(|value| value.to_str()) == Some("md")
            {
                path
            } else if path.is_dir() && path.join("README.md").is_file() {
                path.join("README.md")
            } else {
                continue;
            };
            if read_utf8(&evidence).is_ok_and(|text| {
                let frontmatter = text
                    .strip_prefix("---\n")
                    .and_then(|body| body.split_once("\n---\n"))
                    .map(|(head, _)| head)
                    .unwrap_or("");
                if lifecycle.is_match(frontmatter) {
                    ready.is_match(frontmatter)
                } else {
                    done.is_match(&text.lines().take(30).collect::<Vec<_>>().join("\n"))
                }
            }) {
                count += 1;
            }
        }
    }
    count
}

pub fn show_state() -> StepResult {
    let root = repo_root();
    let (branch, dirty, ahead) = git_state();
    let mut lines = vec![format!(
        "[session-start] branch={branch} | dirty={dirty} | ahead/behind={ahead} | skeleton=v{}",
        skeleton_version()
    )];
    let snapshots = snapshot_paths(&root.join(".runtime/session_state"));
    if let Some(latest) = snapshots
        .first()
        .and_then(|path| path.file_name())
        .and_then(|value| value.to_str())
    {
        lines.push(format!(
            "[snapshot] 最新存档: {latest} — 输入 $resume 可接续上下文"
        ));
    }
    let count = archive_count(&root);
    if count > 0 {
        lines.push(format!(
            "[archive] delivery / bugs 有 {count} 个候选可归档 — 输入 $archive-scan 查看"
        ));
    }
    StepResult::output(lines.join("\n"))
}

fn normalize_hash(value: &str) -> Option<String> {
    let mut normalized = value.trim().to_lowercase();
    if let Some(stripped) = normalized.strip_prefix("sha256:") {
        normalized = stripped.to_string();
    }
    (normalized.len() == 64 && normalized.chars().all(|value| value.is_ascii_hexdigit()))
        .then_some(normalized)
}

pub fn skill_sync() -> StepResult {
    let home = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .unwrap_or_default();
    let platform = home.join(".codex");
    let shelf = platform.join("skills");
    let entry = shelf.join("bridgeforge-codex/SKILL.md");
    let ledger = platform.join("bridgeforge-codex-managed.json");
    if !entry.is_file() && !ledger.exists() {
        return StepResult::ok();
    }
    let warning = |detail: &str| {
        StepResult::output(format!(
            "[skill-sync] {detail}。请运行无参 $bridgeforge-codex 重新同步。"
        ))
    };
    if !ledger.is_file() {
        return warning("bridgeforge-codex 托管账本缺失");
    }
    let Ok(value) = fs::read_to_string(&ledger)
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .ok_or(())
    else {
        return warning("bridgeforge-codex 托管账本无法读取");
    };
    if value.get("schema_version").and_then(Value::as_i64) != Some(1)
        || value.get("platform").and_then(Value::as_str) != Some("codex")
    {
        return warning("bridgeforge-codex 托管账本版本或平台不匹配");
    }
    let Some(records) = value
        .get("records")
        .and_then(Value::as_object)
        .filter(|records| !records.is_empty())
    else {
        return warning("bridgeforge-codex 托管账本没有有效记录");
    };
    let mut stale = Vec::new();
    for (name, record) in records {
        if name.is_empty() || name.contains(['/', '\\']) || matches!(name.as_str(), "." | "..") {
            return warning("bridgeforge-codex 托管账本包含无效 skill 记录");
        }
        let Some(expected) = record
            .get("content_hash")
            .and_then(Value::as_str)
            .and_then(normalize_hash)
        else {
            return warning("bridgeforge-codex 托管账本包含无效 skill 记录");
        };
        let root = shelf.join(name);
        if !root.is_dir() || source_tree_hash(&root) != expected {
            stale.push(name.clone());
        }
    }
    if stale.is_empty() {
        StepResult::ok()
    } else {
        warning(&format!(
            "{} 个托管 skill 缺失或内容漂移（{}）",
            stale.len(),
            stale.join("、")
        ))
    }
}

pub fn build_identity() -> Value {
    let mut hasher = Sha256::new();
    hasher.update(include_bytes!("../Cargo.lock"));
    json!({"schema":1,"name":"bridgeforge-hook","status":"ok","platform":std::env::consts::OS,"arch":std::env::consts::ARCH,"lock_sha256":format!("{:x}",hasher.finalize())})
}
