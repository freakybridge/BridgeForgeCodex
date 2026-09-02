use super::ownership::{
    ExpectedHooksState, MANAGED_ID_KEY, expected_groups, hooks_file_healthy, merge_hooks_file,
};
use super::{
    Authorization, MemoryResult, MemorySyncError, atomic_write, record_native_memories_consent,
};
use serde_json::{Map, Value, json};
use std::fs;
use std::path::Path;

pub const HOOK_ID: &str = "bridgeforge-codex.native-memory-sync.v1";
pub const HOOK_EVENTS: &[&str] = &["SessionStart", "Stop", "SessionEnd"];

struct UserHooksLock {
    _file: fs::File,
}

impl UserHooksLock {
    fn acquire(codex_home: &Path) -> MemoryResult<Self> {
        let directory = codex_home.join(".bridgeforge-codex");
        fs::create_dir_all(&directory)?;
        let path = directory.join("user-hooks.lock");
        super::worker::try_lock_file(&path)?
            .map(|file| Self { _file: file })
            .ok_or_else(|| {
                MemorySyncError::new("another native memory user-hooks update is running")
            })
    }
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn windows_quote(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\\\""))
}

pub fn expected_document(binary: &Path, codex_home: &Path) -> Value {
    let binary_text = binary.to_string_lossy();
    let home_text = codex_home.to_string_lossy();
    let mut hooks = Map::new();
    for event in HOOK_EVENTS {
        let mut handler = json!({
            "type": "command",
            "command": format!(
                "{} memory-sync hook-run --event {} --codex-home {}",
                shell_quote(&binary_text),
                event,
                shell_quote(&home_text),
            ),
            "commandWindows": format!(
                "{} memory-sync hook-run --event {} --codex-home {}",
                windows_quote(&binary_text),
                event,
                windows_quote(&home_text),
            ),
            MANAGED_ID_KEY: format!("{HOOK_ID}:{event}"),
        });
        if *event == "Stop" {
            handler["async"] = Value::Bool(true);
            handler["timeout"] = Value::Number(120.into());
        } else if *event == "SessionStart" {
            handler["timeout"] = Value::Number(120.into());
        } else {
            handler["timeout"] = Value::Number(3.into());
        }
        hooks.insert((*event).into(), json!([{"hooks": [handler]}]));
    }
    json!({"hooks": hooks})
}

fn managed_looking(handler: &Map<String, Value>) -> bool {
    handler
        .get(MANAGED_ID_KEY)
        .and_then(Value::as_str)
        .is_some_and(|value| value.starts_with(&format!("{HOOK_ID}:")))
}

pub fn merge_user_hooks(codex_home: &Path, binary: &Path) -> MemoryResult<bool> {
    fs::create_dir_all(codex_home)?;
    let _lock = UserHooksLock::acquire(codex_home)?;
    let path = codex_home.join("hooks.json");
    let before = fs::read(&path).ok();
    let expected = expected_groups(
        &expected_document(binary, codex_home),
        &format!("{HOOK_ID}:"),
    )?;
    merge_hooks_file(
        &path,
        &expected,
        &[&format!("{HOOK_ID}:")],
        Some(&managed_looking),
        match before.as_deref() {
            Some(payload) => ExpectedHooksState::Exact(payload),
            None => ExpectedHooksState::Missing,
        },
    )
}

pub fn user_hooks_healthy(codex_home: &Path, binary: &Path) -> bool {
    let prefix = format!("{HOOK_ID}:");
    expected_groups(&expected_document(binary, codex_home), &prefix).is_ok_and(|expected| {
        hooks_file_healthy(
            &codex_home.join("hooks.json"),
            &expected,
            &[&prefix],
            Some(&managed_looking),
        )
    })
}

fn parse_config(text: &str) -> MemoryResult<toml_edit::DocumentMut> {
    text.parse().map_err(|error| {
        MemorySyncError::new(format!(
            "invalid Codex config.toml; left unchanged: {error}"
        ))
    })
}

pub fn enable_memories(codex_home: &Path, confirmed: bool) -> MemoryResult<bool> {
    if !confirmed {
        return Err(MemorySyncError::new(
            "native memories remain unchanged without --confirmed-enable",
        ));
    }
    let path = codex_home.join("config.toml");
    let before = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(error.into()),
    };
    let mut document = parse_config(&before)?;
    let mut changed = false;
    for (section, key) in [
        ("features", "memories"),
        ("memories", "generate_memories"),
        ("memories", "use_memories"),
    ] {
        let item = document
            .entry(section)
            .or_insert(toml_edit::Item::Table(toml_edit::Table::new()));
        let table = item.as_table_like_mut().ok_or_else(|| {
            MemorySyncError::new(format!(
                "Codex config.toml {section} must be a table; left unchanged"
            ))
        })?;
        if table.get(key).and_then(toml_edit::Item::as_bool) == Some(true) {
            continue;
        }
        let mut value = toml_edit::value(true);
        if let Some(previous) = table.get(key).and_then(toml_edit::Item::as_value) {
            *value.as_value_mut().unwrap().decor_mut() = previous.decor().clone();
        }
        table.insert(key, value);
        changed = true;
    }
    if !changed {
        return Ok(false);
    }
    let after = document.to_string();
    parse_config(&after)?;
    fs::create_dir_all(codex_home)?;
    atomic_write(&path, after.as_bytes())?;
    Ok(true)
}

pub fn memories_enabled(codex_home: &Path) -> bool {
    let Ok(text) = fs::read_to_string(codex_home.join("config.toml")) else {
        return false;
    };
    let Ok(document) = parse_config(&text) else {
        return false;
    };
    [
        ("features", "memories"),
        ("memories", "generate_memories"),
        ("memories", "use_memories"),
    ]
    .iter()
    .all(|(section, key)| {
        document
            .get(section)
            .and_then(toml_edit::Item::as_table_like)
            .and_then(|table| table.get(key))
            .and_then(toml_edit::Item::as_bool)
            == Some(true)
    })
}

pub fn configure(
    codex_home: &Path,
    binary: &Path,
    remote: &str,
    confirmed_enable: bool,
) -> MemoryResult<Authorization> {
    let state_dir = codex_home.join(".bridgeforge-codex/native-memory-sync");
    let ledger = codex_home.join("bridgeforge-codex-managed.json");
    let targets = [
        codex_home.join("config.toml"),
        codex_home.join("hooks.json"),
        state_dir.join("remote.txt"),
        ledger.clone(),
    ];
    let snapshots = targets
        .iter()
        .map(|path| (path.clone(), fs::read(path).ok()))
        .collect::<Vec<_>>();
    let applied = (|| {
        enable_memories(codex_home, confirmed_enable)?;
        merge_user_hooks(codex_home, binary)?;
        fs::create_dir_all(&state_dir)?;
        atomic_write(
            &state_dir.join("remote.txt"),
            format!("{remote}\n").as_bytes(),
        )?;
        record_native_memories_consent(&ledger, "approved", true, Some(remote))?;
        super::native_memories_authorization(&ledger)?
            .ok_or_else(|| MemorySyncError::new("native memories authorization was not recorded"))
    })();
    if applied.is_ok() {
        return applied;
    }
    let original = applied.unwrap_err();
    let mut rollback = Vec::new();
    for (path, before) in snapshots.into_iter().rev() {
        let restored = match before {
            Some(payload) => atomic_write(&path, &payload),
            None if path.exists() => fs::remove_file(&path).map_err(MemorySyncError::from),
            None => Ok(()),
        };
        if let Err(error) = restored {
            rollback.push(format!("{}: {error}", path.display()));
        }
    }
    if rollback.is_empty() {
        Err(MemorySyncError::new(format!(
            "native memories setup rolled back: {original}"
        )))
    } else {
        Err(MemorySyncError::new(format!(
            "native memories setup failed: {original}; rollback incomplete: {}",
            rollback.join("; ")
        )))
    }
}

#[cfg(all(test, bridgeforge_factory_tests))]
#[path = "../../../../../../scripts/tests/unit/core_memory_user_config.rs"]
mod tests;
