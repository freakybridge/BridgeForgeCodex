use chrono::Local;
use regex::Regex;
use serde_json::{Value, json};
use sha1::{Digest as Sha1Digest, Sha1};
use sha2::Sha256;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::StepResult;
use crate::util::{
    atomic_write, json_string, normalize_path, path_is_inside, relative_display, repo_root,
    tool_input,
};

const TEXT_SUFFIXES: &[&str] = &[
    "md", "json", "py", "rs", "toml", "sh", "ps1", "yml", "yaml", "txt",
];

// Factory-test counters only; no production telemetry or persisted state.
#[cfg(all(test, bridgeforge_factory_tests))]
pub(crate) static ENCODING_SCANS: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);
#[cfg(all(test, bridgeforge_factory_tests))]
pub(crate) static INSTRUCTION_SCANS: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);
const TEXT_NAMES: &[&str] = &[
    "AGENTS.md",
    "CHANGELOG.md",
    "INSTALL.md",
    "README.md",
    "RETIRED.md",
    "SKILL.md",
    "VERSION",
    "pre-commit",
];
const TEXT_ROOTS: &[&str] = &[
    "templates",
    "skills",
    "scripts",
    ".githooks",
    ".codex",
    "doc",
    "README.md",
    "INSTALL.md",
    "SKILL.md",
    "CHANGELOG.md",
    "VERSION",
    "AGENTS.md",
];

fn is_text(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| TEXT_SUFFIXES.contains(&value.to_lowercase().as_str()))
        || path
            .file_name()
            .and_then(|value| value.to_str())
            .is_some_and(|value| TEXT_NAMES.contains(&value))
}

fn all_text_paths(root: &Path) -> Vec<PathBuf> {
    #[cfg(all(test, bridgeforge_factory_tests))]
    ENCODING_SCANS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let mut result = BTreeSet::new();
    for selected in TEXT_ROOTS {
        let path = root.join(selected);
        if path.is_file() && is_text(&path) {
            result.insert(path);
            continue;
        }
        if !path.is_dir() {
            continue;
        }
        for entry in WalkDir::new(path)
            .follow_links(false)
            .into_iter()
            .filter_map(Result::ok)
        {
            if entry.file_type().is_file() && is_text(entry.path()) {
                result.insert(entry.path().to_path_buf());
            }
        }
    }
    result.into_iter().collect()
}

fn garble_hits(root: &Path, paths: &[PathBuf]) -> Vec<String> {
    let re = Regex::new(r"\?{3,}|\u{fffd}").unwrap();
    let mut hits = Vec::new();
    for path in paths {
        let Ok(bytes) = fs::read(path) else {
            continue;
        };
        let text = String::from_utf8_lossy(&bytes);
        for (index, line) in text.lines().enumerate() {
            if !re.is_match(line) {
                continue;
            }
            let mut snippet = line.trim().to_string();
            if snippet.chars().count() > 120 {
                snippet = snippet.chars().take(117).collect::<String>() + "...";
            }
            hits.push(format!(
                "{}:{}: {}",
                relative_display(root, path),
                index + 1,
                snippet
            ));
        }
    }
    hits
}

pub fn encoding(payloads: &[Value]) -> StepResult {
    let root = repo_root();
    let all = all_text_paths(&root);
    let bom: Vec<String> = all
        .iter()
        .filter_map(|path| {
            fs::read(path)
                .ok()
                .filter(|bytes| bytes.starts_with(&[0xef, 0xbb, 0xbf]))
                .map(|_| relative_display(&root, path))
        })
        .collect();
    if !bom.is_empty() {
        return StepResult::blocked(format!(
            "[encoding] hard gate: UTF-8 BOM is forbidden\n{}\n[encoding] Fix: save these files as UTF-8 without BOM.\n",
            bom.into_iter()
                .map(|item| format!("[encoding]   {item}"))
                .collect::<Vec<_>>()
                .join("\n")
        ));
    }
    let mut edited = BTreeSet::new();
    for payload in payloads {
        if let Some(input) = tool_input(payload) {
            for key in ["file_path", "path"] {
                let raw = json_string(input, key);
                if raw.is_empty() {
                    continue;
                }
                let path = normalize_path(&root, &raw);
                if path_is_inside(&root, &path) && path.is_file() && is_text(&path) {
                    edited.insert(path);
                }
            }
        }
    }
    let hits = garble_hits(&root, &edited.into_iter().collect::<Vec<_>>());
    if hits.is_empty() {
        StepResult::ok()
    } else {
        StepResult {
            code: 0,
            stdout: String::new(),
            stderr: format!(
                "[encoding] suspicious replacement text detected\n{}\n[encoding] Stop and confirm the original text; automatic repair is unsafe.\n",
                hits.into_iter()
                    .map(|item| format!("[encoding]   {item}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            ),
        }
    }
}

pub fn requirements(payload: &Value) -> StepResult {
    let Some(input) = tool_input(payload) else {
        return StepResult::ok();
    };
    let raw = json_string(input, "file_path");
    let path = normalize_path(&repo_root(), &raw);
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    if !name.starts_with("requirements")
        || path.extension().and_then(|value| value.to_str()) != Some("txt")
        || !path.is_file()
    {
        return StepResult::ok();
    }
    let Ok(bytes) = fs::read(&path) else {
        return StepResult::ok();
    };
    let mut violations = Vec::new();
    if std::str::from_utf8(&bytes).is_err() {
        violations.push("文件不是合法 UTF-8 — 清理非法字节".to_string());
    }
    let text = String::from_utf8_lossy(&bytes);
    let absolute = Regex::new(r"(?i)@\s*file://").unwrap();
    for (index, line) in text.lines().enumerate() {
        if absolute.is_match(line) {
            violations.push(format!("L{} 绝对路径 URL（`@ file://`）— 换机即 fail；改用 libs/ + 顶部 `--find-links libs/` + `name==version`", index + 1));
        }
        let non_ascii: Vec<char> = line
            .chars()
            .filter(|value| !value.is_ascii())
            .take(5)
            .collect();
        if !non_ascii.is_empty() {
            violations.push(format!("L{} 含非 ASCII 字符 {:?} — pip 在 Windows 按 GBK 解码会报错；注释只用英文，中文挪 README", index + 1, non_ascii));
        }
    }
    if violations.is_empty() {
        return StepResult::ok();
    }
    StepResult::output(format!(
        "[requirements-check] {name} 违反可移植性红线（AGENTS.md §1.3；codex-project-operating-guide.md）:\n{}",
        violations
            .into_iter()
            .map(|item| format!("[requirements-check]   - {item}"))
            .collect::<Vec<_>>()
            .join("\n")
    ))
}

pub fn cargo_default_run(payload: &Value) -> StepResult {
    let Some(input) = tool_input(payload) else {
        return StepResult::ok();
    };
    let raw = json_string(input, "file_path").replace('\\', "/");
    if !raw.ends_with("Cargo.toml") {
        return StepResult::ok();
    }
    let root = repo_root();
    let path = normalize_path(&root, &raw);
    let Ok(text) = fs::read_to_string(&path) else {
        return StepResult::ok();
    };
    let bin_count = Regex::new(r"(?m)^\s*\[\[bin\]\]")
        .unwrap()
        .find_iter(&text)
        .count();
    if bin_count < 2
        || Regex::new(r"(?m)^\s*default-run\s*=")
            .unwrap()
            .is_match(&text)
    {
        return StepResult::ok();
    }
    StepResult {
        code: 0,
        stdout: String::new(),
        stderr: format!(
            "[cargo-default-run] {} contains {bin_count} [[bin]] sections but no default-run.\n   Plain `cargo run` will fail with: could not determine which binary to run.\n   Fix: add `default-run = \"<main-bin-name>\"` under [package], or make launch scripts call `cargo run --bin <name>` explicitly.\n",
            relative_display(&root, &path)
        ),
    }
}

pub fn fallback_smell(payload: &Value) -> StepResult {
    let Some(input) = tool_input(payload) else {
        return StepResult::ok();
    };
    let raw = json_string(input, "file_path");
    let suffix = Path::new(&raw)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_lowercase();
    if !["py", "pyw", "js", "jsx", "ts", "tsx", "mjs", "cjs"].contains(&suffix.as_str()) {
        return StepResult::ok();
    }
    let mut parts = Vec::new();
    for key in ["new_string", "content"] {
        if let Some(value) = input.get(key).and_then(Value::as_str) {
            parts.push((value, None));
        }
    }
    if let Some(edits) = input.get("edits").and_then(Value::as_array) {
        for edit in edits {
            if let Some(value) = edit.get("new_string").and_then(Value::as_str) {
                parts.push((value, None));
            }
        }
    }
    if let Some(hunks) = input.get("patch_hunks").and_then(Value::as_array) {
        for hunk in hunks {
            if let (Some(text), Some(added)) = (
                hunk.get("new_string").and_then(Value::as_str),
                hunk.get("added_lines").and_then(Value::as_array),
            ) {
                parts.push((text, Some(added)));
            }
        }
    }
    if parts.is_empty() {
        return StepResult::ok();
    }
    let py = Regex::new(r"except\s*(?:\(?\s*(?:Exception|BaseException)\s*\)?(?:\s+as\s+\w+)?)?\s*:\s*(?:\n[ \t]*)?pass\b").unwrap();
    let js = Regex::new(r"catch\s*(?:\([^)]*\))?\s*\{\s*\}").unwrap();
    let mut hits = Vec::new();
    for (text, added_lines) in parts {
        for (pattern, label) in [
            (&py, "裸/宽 except 后直接 pass（静默吞异常）"),
            (&js, "空 catch 块（静默吞异常）"),
        ] {
            for item in pattern.find_iter(text) {
                // Context completes split constructs; only changed matches should warn.
                if let Some(added) = added_lines {
                    let first = text[..item.start()]
                        .bytes()
                        .filter(|byte| *byte == b'\n')
                        .count();
                    let last = text[..item.end()]
                        .trim_end_matches('\n')
                        .bytes()
                        .filter(|byte| *byte == b'\n')
                        .count();
                    if !added
                        .iter()
                        .filter_map(Value::as_u64)
                        .any(|line| (first..=last).contains(&(line as usize)))
                    {
                        continue;
                    }
                }
                hits.push(format!(
                    "{label}: {}",
                    item.as_str()
                        .split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" ")
                ));
            }
        }
    }
    if hits.is_empty() {
        return StepResult::ok();
    }
    StepResult::output(format!(
        "[fallback-smell] {} 疑似兜底坏味道（仅裸吞异常，软提醒不阻塞）:\n{}\n[fallback-smell] 静默吞异常会把真报错藏成假成功 → 先确认根因，别用 pass 掩盖",
        Path::new(&raw)
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or(&raw),
        hits.into_iter()
            .map(|item| format!("[fallback-smell]   - {item}"))
            .collect::<Vec<_>>()
            .join("\n")
    ))
}

const PUBLIC_BEGIN: &str = "<!-- BRIDGEFORGE:PUBLIC:BEGIN -->";
const PUBLIC_END: &str = "<!-- BRIDGEFORGE:PUBLIC:END -->";
const PROJECT_BEGIN: &str = "<!-- BRIDGEFORGE:PROJECT:BEGIN -->";
const PROJECT_END: &str = "<!-- BRIDGEFORGE:PROJECT:END -->";
const RETIRED_INSTRUCTION_RUNTIME: &[&str] = &[
    "skill-routing.json",
    "clarify_reminder.py",
    "focus_reminder.py",
    "project_memory_writer.py",
    "memory_rebuild_index.py",
    "memory_lint.py",
];

fn line_finish(text: &str, marker: usize) -> usize {
    text[marker..]
        .find('\n')
        .map(|offset| marker + offset + 1)
        .unwrap_or(text.len())
}

fn zone_parts(text: &str) -> Result<(&str, &str), String> {
    let markers = [PUBLIC_BEGIN, PUBLIC_END, PROJECT_BEGIN, PROJECT_END];
    if markers
        .iter()
        .any(|marker| text.matches(marker).count() != 1)
    {
        return Err("AGENTS zone markers must each appear exactly once".into());
    }
    let positions: Vec<_> = markers
        .iter()
        .map(|marker| text.find(marker).unwrap())
        .collect();
    if !positions.windows(2).all(|pair| pair[0] < pair[1]) {
        return Err("AGENTS zone markers are reversed or nested".into());
    }
    let public_finish = line_finish(text, positions[1]);
    let project_finish = line_finish(text, positions[3]);
    let outside = format!(
        "{}{}{}",
        &text[..positions[0]],
        &text[public_finish..positions[2]],
        &text[project_finish..]
    );
    if !outside.trim().is_empty() {
        return Err("AGENTS content exists outside the public/project zones".into());
    }
    Ok((
        &text[positions[0]..public_finish],
        &text[positions[2]..project_finish],
    ))
}

fn visible_heading_positions(
    text: &str,
    headings: &[&str],
) -> Result<BTreeMap<String, Vec<usize>>, String> {
    let mut result: BTreeMap<String, Vec<usize>> = headings
        .iter()
        .map(|heading| ((*heading).to_string(), Vec::new()))
        .collect();
    let mut fence: Option<(char, usize)> = None;
    let mut offset = 0usize;
    for line in text.split_inclusive('\n') {
        let stripped = line.trim_end_matches(['\r', '\n']);
        let leading = stripped.len() - stripped.trim_start_matches(' ').len();
        let visible = stripped.trim_start_matches(' ');
        let marker_len = visible
            .chars()
            .take_while(|value| *value == '`' || *value == '~')
            .count();
        if leading <= 3 && marker_len >= 3 {
            let marker = visible.chars().next().unwrap();
            match fence {
                Some((open, minimum))
                    if marker == open
                        && marker_len >= minimum
                        && visible[marker_len..].trim().is_empty() =>
                {
                    fence = None;
                }
                None => fence = Some((marker, marker_len)),
                _ => {}
            }
        } else if fence.is_none()
            && leading <= 3
            && let Some(positions) = result.get_mut(visible)
        {
            positions.push(offset);
        }
        offset += line.len();
    }
    if fence.is_some() {
        return Err("AGENTS contains an unclosed fenced code block".into());
    }
    Ok(result)
}

fn claims_positive_autoload(text: &str) -> bool {
    let positives = [
        Regex::new(r"(?i)详细规则按需加载自\s*[^\n]*rules").unwrap(),
        Regex::new(r"(?i)Markdown[^\n]{0,80}(?:paths:|path)[^\n]{0,80}(?:自动|按需|始终)加载")
            .unwrap(),
        Regex::new(r"(?i)(?:自动|按需|始终)加载[^\n]{0,80}Markdown[^\n]{0,80}(?:paths:|path)")
            .unwrap(),
        Regex::new(r"(?i)(?:paths:|path-rule)[^\n]{0,80}(?:自动|按需|始终)加载").unwrap(),
    ];
    let negated = Regex::new(
        r"(?i)不会|不能|不支持|并不|未被|禁止.{0,24}(?:宣称|建立)|does\s+not|not\s+(?:be\s+)?auto",
    )
    .unwrap();
    text.split(['。', '！', '？', '；', ';', '，', ',', '\n'])
        .any(|clause| {
            positives.iter().any(|item| item.is_match(clause)) && !negated.is_match(clause)
        })
}

fn managed_public_hash(root: &Path) -> Option<String> {
    for path in [
        root.join(".codex/managed-skeleton.json"),
        root.join("templates/managed-skeleton.json"),
    ] {
        let Ok(value) = fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str::<Value>(&text).ok())
            .ok_or(())
        else {
            continue;
        };
        if let Some(hash) = value["assets"]
            .as_array()
            .and_then(|assets| assets.iter().find(|asset| asset["id"] == "root.agents"))
            .and_then(|asset| asset["agents_zones"]["public"]["current_sha256"].as_str())
        {
            return Some(hash.to_string());
        }
    }
    None
}

fn public_hash(public: &str) -> String {
    let clone = Regex::new(
        r"(?m)^(git clone <repo_url> )(?:[A-Za-z0-9._-]+|\{\{PROJECT_NAME\}\})( && cd )(?:[A-Za-z0-9._-]+|\{\{PROJECT_NAME\}\})([ \t]*)$",
    )
    .unwrap();
    let normalized = clone.replace_all(public, "$1{{PROJECT_NAME}}$2{{PROJECT_NAME}}$3");
    let mut hasher = Sha256::new();
    hasher.update(normalized.as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}

fn instruction_text_issues(path: &Path, root: &Path, nested: bool) -> Vec<String> {
    let label = relative_display(root, path);
    let Ok(bytes) = fs::read(path) else {
        return vec![format!("cannot read {label}")];
    };
    if bytes.len() > 48 * 1024 {
        return vec![format!("{label} exceeds 49152 bytes")];
    }
    if bytes.starts_with(&[0xef, 0xbb, 0xbf]) {
        return vec![format!("{label} contains UTF-8 BOM")];
    }
    let Ok(text) = String::from_utf8(bytes) else {
        return vec![format!("cannot read {label}: invalid UTF-8")];
    };
    let lowered = text.to_lowercase();
    let mut issues = Vec::new();
    for retired in RETIRED_INSTRUCTION_RUNTIME {
        if lowered.contains(&retired.to_lowercase()) {
            issues.push(format!("{label} references retired runtime asset {retired}; keep retirement history outside active instructions"));
        }
    }
    if claims_positive_autoload(&text) {
        issues.push(format!(
            "{label} claims unsupported Markdown paths auto-loading"
        ));
    }
    if nested { issues } else { Vec::new() }
}

pub fn instruction_source(_payload: &Value) -> StepResult {
    #[cfg(all(test, bridgeforge_factory_tests))]
    INSTRUCTION_SCANS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = repo_root();
    let path = root.join("AGENTS.md");
    let Ok(bytes) = fs::read(&path) else {
        return StepResult {
            code: 0,
            stdout: String::new(),
            stderr: "[instruction-source] cannot read AGENTS.md\n".into(),
        };
    };
    let mut issues = Vec::new();
    if bytes.len() > 48 * 1024 {
        issues.push("AGENTS.md exceeds 49152 bytes".to_string());
    }
    if bytes.starts_with(&[0xef, 0xbb, 0xbf]) {
        issues.push("AGENTS.md contains UTF-8 BOM".into());
    }
    let text = match String::from_utf8(bytes) {
        Ok(text) => text,
        Err(error) => {
            issues.push("AGENTS.md is not valid UTF-8".into());
            String::from_utf8_lossy(error.as_bytes()).into_owned()
        }
    }
    .replace("\r\n", "\n")
    .replace('\r', "\n");
    let parts = zone_parts(&text);
    match &parts {
        Ok((public, project)) => {
            // The managed public hash below validates the complete public block.
            // Do not duplicate its editorial headings in the runtime contract.
            if let Err(error) = visible_heading_positions(public, &[]) {
                issues.push(format!("AGENTS.md: {error}"));
            }
            let project_headings = [
                "## 项目级专区",
                "### 项目架构红线",
                "### 项目业务与安全红线",
                "### 项目目录地图",
                "### 项目快速命令",
                "### 目录级 AGENTS 索引",
            ];
            let project_heads = visible_heading_positions(project, &project_headings);
            if let Err(error) = &project_heads {
                issues.push(format!("AGENTS.md: {error}"));
            }
            let mut ordered = Vec::new();
            for heading in project_headings {
                let positions = project_heads
                    .as_ref()
                    .ok()
                    .and_then(|items| items.get(heading));
                if positions.is_none_or(|items| items.len() != 1) {
                    issues.push(format!(
                        "AGENTS.md project zone must contain exactly one {heading}"
                    ));
                } else {
                    ordered.push(positions.unwrap()[0]);
                }
            }
            if !ordered.windows(2).all(|pair| pair[0] < pair[1]) {
                issues.push("AGENTS.md project zone headings are out of order".into());
            }
            match managed_public_hash(&root) {
                Some(expected) if public_hash(public) != expected => issues.push(
                    "AGENTS.md BridgeForge public zone was modified; move project constraints to the project zone and restore the official public block".into(),
                ),
                None => issues.push("AGENTS.md BridgeForge public zone cannot be verified because the managed contract is missing, invalid, or has no trusted public hash".into()),
                _ => {}
            }
            if text.contains("规则文件索引") || text.contains("Rule 文件索引") {
                issues.push("active Markdown rule index is forbidden".into());
            }
            if claims_positive_autoload(&text) {
                issues.push("AGENTS.md claims unsupported Markdown paths auto-loading".into());
            }
            if let Ok(template) = fs::read_to_string(root.join("templates/AGENTS.md"))
                && let Ok((template_public, _)) = zone_parts(&template)
            {
                let rendered = template_public.replace("{{PROJECT_NAME}}", "BridgeForgeCodex");
                if rendered != *public {
                    issues.push(
                        "factory AGENTS public regions drift from templates/AGENTS.md".into(),
                    );
                }
            }
        }
        Err(error) => issues.push(error.clone()),
    }
    for target in ["doc/README.md", "doc/3_reference/codex-hook-signals.md"] {
        if text.contains(target) && !root.join(target).is_file() {
            issues.push(format!(
                "AGENTS.md references missing managed instruction document: {target}"
            ));
        }
    }
    for retired in RETIRED_INSTRUCTION_RUNTIME {
        if text.to_lowercase().contains(&retired.to_lowercase()) {
            issues.push(format!("AGENTS.md references retired runtime asset {retired}; keep retirement history outside active instructions"));
        }
    }
    for entry in WalkDir::new(&root)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
    {
        if !entry.file_type().is_file() || entry.file_name() != "AGENTS.md" || entry.path() == path
        {
            continue;
        }
        let relative = entry.path().strip_prefix(&root).unwrap_or(entry.path());
        let parts: Vec<_> = relative
            .components()
            .map(|item| item.as_os_str().to_string_lossy())
            .collect();
        if parts
            .iter()
            .any(|item| item == ".git" || item == ".runtime")
            || (parts.first().is_some_and(|item| item == "doc")
                && parts.get(1).is_some_and(|item| item == "4_archive"))
        {
            continue;
        }
        issues.extend(instruction_text_issues(entry.path(), &root, true));
    }
    for directory in [root.join("templates/rules"), root.join(".codex/rules")] {
        if directory.is_dir()
            && WalkDir::new(&directory)
                .max_depth(1)
                .into_iter()
                .filter_map(Result::ok)
                .any(|entry| {
                    entry.path().extension().and_then(|value| value.to_str()) == Some("md")
                })
        {
            issues.push(format!(
                "factory Markdown rule directory must remain retired: {}",
                relative_display(&root, &directory)
            ));
        }
    }
    let stderr = issues
        .into_iter()
        .map(|item| format!("[instruction-source] {item}\n"))
        .collect();
    StepResult {
        code: 0,
        stdout: String::new(),
        stderr,
    }
}

pub fn precommit_instruction_source() -> i32 {
    let step = instruction_source(&Value::Null);
    eprint!("{}", step.stderr);
    if step.stderr.is_empty() { 0 } else { 2 }
}

fn response_text(value: &Value) -> String {
    if let Some(text) = value.as_str() {
        return text.into();
    }
    let mut result = Vec::new();
    if let Some(map) = value.as_object() {
        for key in ["stdout", "stderr", "output", "text", "error"] {
            if let Some(value) = map.get(key).and_then(Value::as_str) {
                result.push(value);
            }
        }
        if let Some(content) = map.get("content").and_then(Value::as_array) {
            for item in content {
                if let Some(value) = item.get("text").and_then(Value::as_str) {
                    result.push(value);
                }
            }
        }
    }
    result.join("\n")
}

pub fn test_receipt(payload: &Value) -> StepResult {
    let Some(input) = tool_input(payload) else {
        return StepResult::ok();
    };
    let command = json_string(input, "command");
    let matcher =
        Regex::new(r"(?i)\b(pytest|cargo\s+test|npm\s+(?:run\s+)?test|go\s+test|tsc|make)\b")
            .unwrap();
    let Some(found) = matcher.find(&command) else {
        return StepResult::ok();
    };
    let response = payload.get("tool_response").unwrap_or(&Value::Null);
    let interrupted = response
        .get("interrupted")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let mut exit = None;
    let mut source = "unknown";
    for key in [
        "exit_code",
        "exitCode",
        "returncode",
        "returnCode",
        "code",
        "status",
    ] {
        if let Some(value) = response.get(key) {
            if let Some(number) = value.as_i64() {
                exit = Some(number);
                source = "explicit";
                break;
            }
            if let Some(number) = value.as_str().and_then(|text| text.parse::<i64>().ok()) {
                exit = Some(number);
                source = "explicit";
                break;
            }
        }
    }
    let response_output = response_text(response);
    let running = ["session_id", "cell_id"]
        .iter()
        .any(|key| response.get(*key).is_some_and(|value| !value.is_null()))
        || response
            .get("status")
            .and_then(Value::as_str)
            .is_some_and(|status| matches!(status, "running" | "pending"))
        || response_output.contains("Process running with session ID")
        || response_output.contains("Script running with cell ID");
    if running {
        exit = None;
        source = "unknown";
    }
    if exit.is_none() && !running {
        let text = response_text(response);
        if let Some(number) = Regex::new(r"(?i)\bexit\s+(?:code|status)\s+(\d+)")
            .unwrap()
            .captures(&text)
            .and_then(|item| item.get(1))
            .and_then(|item| item.as_str().parse::<i64>().ok())
        {
            exit = Some(number);
            source = "text";
        }
    }
    let timestamp = Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();
    let mut hasher = Sha1::new();
    hasher.update(command.as_bytes());
    let sha1 = format!("{:x}", hasher.finalize());
    let receipt = json!({"ts":timestamp,"kind":found.as_str().split_whitespace().next().unwrap_or("test"),"cmd":command.chars().take(200).collect::<String>(),"sha1":&sha1[..12],"exit_code":exit,"source":source,"interrupted":interrupted});
    let root = repo_root();
    let directory = root.join(".runtime/test_receipts");
    if let Err(error) = fs::create_dir_all(&directory) {
        return StepResult::failed("test-receipt directory", error);
    }
    let mut file = match OpenOptions::new()
        .create(true)
        .append(true)
        .open(directory.join("receipts.jsonl"))
    {
        Ok(file) => file,
        Err(error) => return StepResult::failed("test-receipt open", error),
    };
    if let Err(error) = writeln!(file, "{}", receipt).and_then(|_| file.sync_all()) {
        return StepResult::failed("test-receipt write", error);
    }
    let samples = directory.join("payload_samples");
    let sample_result = (|| -> Result<(), Box<dyn std::error::Error>> {
        fs::create_dir_all(&samples)?;
        let sample = samples.join(format!("{}.json", timestamp.replace(':', "")));
        let bytes = serde_json::to_vec_pretty(&truncate_sample(payload))?;
        atomic_write(&sample, &bytes)?;
        let mut entries: Vec<_> = fs::read_dir(&samples)?
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .filter(|entry| {
                entry.path().extension().and_then(|value| value.to_str()) == Some("json")
            })
            .collect();
        entries.sort_by_key(|entry| entry.metadata().and_then(|value| value.modified()).ok());
        let remove_count = entries.len().saturating_sub(5);
        for entry in entries.into_iter().take(remove_count) {
            fs::remove_file(entry.path())?;
        }
        Ok(())
    })();
    if let Err(error) = sample_result {
        return StepResult::failed(
            "test-receipt sample/retention (receipt already appended)",
            error,
        );
    }
    StepResult::output(format!(
        "[test-receipt] {} exit={} source={} sha1={} -> .runtime/test_receipts/receipts.jsonl",
        found.as_str().split_whitespace().next().unwrap_or("test"),
        exit.map(|value| value.to_string())
            .unwrap_or_else(|| "None".into()),
        source,
        &sha1[..12]
    ))
}

fn truncate_sample(value: &Value) -> Value {
    match value {
        Value::String(text) => Value::String(text.chars().take(2000).collect()),
        Value::Array(items) => Value::Array(items.iter().map(truncate_sample).collect()),
        Value::Object(items) => Value::Object(
            items
                .iter()
                .map(|(key, value)| (key.clone(), truncate_sample(value)))
                .collect(),
        ),
        _ => value.clone(),
    }
}

fn index_encoding_issues(root: &Path) -> Result<Vec<String>, String> {
    use bridgeforge_core::{ProcessRequest, ProcessRunner, SystemProcessRunner};
    let query = |args: &[&str], stdin: Vec<u8>| -> Result<Vec<u8>, String> {
        let mut request = ProcessRequest::new("git", root);
        request.args = args.iter().map(|value| (*value).into()).collect();
        request.stdin = stdin;
        let output = SystemProcessRunner
            .run(&request)
            .map_err(|error| error.to_string())?;
        if output.timed_out || output.code != 0 {
            return Err(format!(
                "Git index query failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        Ok(output.stdout)
    };
    let changed_bytes = query(
        &[
            "diff",
            "--cached",
            "--name-only",
            "-z",
            "--diff-filter=ACMR",
        ],
        vec![],
    )?;
    let changed = std::str::from_utf8(&changed_bytes)
        .map_err(|error| error.to_string())?
        .split('\0')
        .filter(|name| !name.is_empty())
        .collect::<BTreeSet<_>>();
    let entries = query(&["ls-files", "--stage", "-z"], vec![])?;
    let entries = std::str::from_utf8(&entries).map_err(|error| error.to_string())?;
    let mut selected = Vec::new();
    for entry in entries.split('\0').filter(|entry| !entry.is_empty()) {
        let (metadata, name) = entry.split_once('\t').ok_or("invalid Git index entry")?;
        let fields: Vec<_> = metadata.split_whitespace().collect();
        if fields.len() != 3 || fields[2] != "0" {
            return Err("unmerged or invalid index entry".into());
        }
        let scoped = TEXT_ROOTS
            .iter()
            .any(|scope| Path::new(name).starts_with(scope));
        if fields[0].starts_with("100")
            && is_text(Path::new(name))
            && (scoped || changed.contains(name))
        {
            selected.push((fields[1], name));
        }
    }
    if selected.is_empty() {
        return Ok(vec![]);
    }
    let input = selected
        .iter()
        .map(|(oid, _)| format!("{oid}\n"))
        .collect::<String>();
    let data = query(&["cat-file", "--batch"], input.into_bytes())?;
    let mut cursor = 0usize;
    let mut hits = Vec::new();
    let garble = Regex::new(r"\?{3,}|\u{fffd}").unwrap();
    for (oid, name) in selected {
        let rest = data.get(cursor..).ok_or("truncated Git blob batch")?;
        let header_length = rest
            .iter()
            .position(|byte| *byte == b'\n')
            .ok_or("missing Git blob header")?;
        let header =
            std::str::from_utf8(&rest[..header_length]).map_err(|error| error.to_string())?;
        let fields: Vec<_> = header.split_whitespace().collect();
        if fields.len() != 3 || fields[0] != oid || fields[1] != "blob" {
            return Err("unexpected Git blob response".into());
        }
        let size: usize = fields[2].parse().map_err(|_| "invalid Git blob size")?;
        cursor += header_length + 1;
        let end = cursor.checked_add(size).ok_or("Git blob size overflow")?;
        let bytes = data.get(cursor..end).ok_or("truncated Git blob body")?;
        if data.get(end) != Some(&b'\n') {
            return Err("invalid Git blob terminator".into());
        }
        cursor = end + 1;
        if bytes.starts_with(&[0xef, 0xbb, 0xbf]) {
            hits.push(format!("{name}: UTF-8 BOM is forbidden in the index"));
        }
        if changed.contains(name) {
            for (index, line) in String::from_utf8_lossy(bytes).lines().enumerate() {
                if garble.is_match(line) {
                    hits.push(format!("{name}:{}: suspicious replacement text", index + 1));
                }
            }
        }
    }
    Ok(hits)
}

pub fn precommit_encoding() -> i32 {
    match index_encoding_issues(&repo_root()) {
        Ok(hits) if hits.is_empty() => 0,
        Ok(hits) => {
            eprintln!(
                "[encoding] staged/index content failed\n{}",
                hits.join("\n")
            );
            2
        }
        Err(error) => {
            eprintln!("[encoding] cannot verify index: {error}");
            2
        }
    }
}

pub fn source_tree_hash(root: &Path) -> String {
    let mut records = Vec::new();
    for entry in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| !matches!(entry.file_name().to_str(), Some(".git" | "__pycache__")))
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
    {
        let Ok(bytes) = fs::read(entry.path()) else {
            continue;
        };
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        records.push((
            relative_display(root, entry.path()),
            format!("{:x}", hasher.finalize()),
        ));
    }
    records.sort();
    let mut hasher = Sha256::new();
    for (path, digest) in records {
        hasher.update(format!("{path}\n{digest}\n"));
    }
    format!("{:x}", hasher.finalize())
}
