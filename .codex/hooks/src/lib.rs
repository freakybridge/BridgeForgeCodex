mod guards;
mod post;
mod project_map;
mod session;
mod util;

pub use guards::git_add_all_with_runner;

use serde_json::{Map, Value, json};
use std::io::Write;

use util::{
    EXIT_BLOCKED, ToolKind, parse_payload, read_stdin_bytes, tool_input, tool_kind, tool_name,
};

#[derive(Default)]
pub struct StepResult {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl StepResult {
    pub fn failed(operation: &str, error: impl std::fmt::Display) -> Self {
        Self {
            code: 1,
            stdout: String::new(),
            stderr: format!(
                "[{operation}] state operation failed: {error}; any completed tool action is unchanged.\n"
            ),
        }
    }
    pub fn ok() -> Self {
        Self::default()
    }
    pub fn blocked(stderr: String) -> Self {
        Self {
            code: EXIT_BLOCKED,
            stdout: String::new(),
            stderr,
        }
    }
    pub fn output(stdout: String) -> Self {
        Self {
            code: 0,
            stdout,
            stderr: String::new(),
        }
    }
}

#[derive(Default)]
struct HookOutput {
    contexts: Vec<String>,
    fields: Map<String, Value>,
}

impl HookOutput {
    fn absorb(&mut self, result: &StepResult) {
        if !result.stderr.is_empty() {
            let _ = std::io::stderr().write_all(result.stderr.as_bytes());
        }
        let raw = result.stdout.trim();
        if raw.is_empty() {
            return;
        }
        if let Ok(value) = serde_json::from_str::<Value>(raw)
            && let Some(specific) = value.get("hookSpecificOutput").and_then(Value::as_object)
        {
            if let Some(context) = specific.get("additionalContext").and_then(Value::as_str)
                && !context.is_empty()
            {
                self.contexts.push(context.to_string());
            }
            for (key, value) in specific {
                if key != "hookEventName" && key != "additionalContext" {
                    self.fields.insert(key.clone(), value.clone());
                }
            }
            return;
        }
        self.contexts.push(raw.to_string());
    }

    fn finish(self, event: &str, code: i32) -> i32 {
        if !self.contexts.is_empty() || !self.fields.is_empty() {
            let mut specific = self.fields;
            specific.insert("hookEventName".into(), Value::String(event.into()));
            if !self.contexts.is_empty() {
                specific.insert(
                    "additionalContext".into(),
                    Value::String(self.contexts.join("\n")),
                );
            }
            println!("{}", json!({"hookSpecificOutput": Value::Object(specific)}));
        }
        code
    }
}

fn validate_tool_payload(event: &str, payload: &Value) -> Result<Value, String> {
    let name = tool_name(payload);
    if name.is_empty() {
        return Err("tool_name is missing or is not a non-empty string".into());
    }
    let Some(input) = tool_input(payload) else {
        return Err("tool_input must be a JSON object".into());
    };
    let kind = tool_kind(&name).ok_or_else(|| format!("unsupported tool for {event}: {name}"))?;
    if (event == "post-shell" && kind != ToolKind::Shell)
        || (event == "post-edit" && kind == ToolKind::Shell)
    {
        return Err(format!("unsupported tool for {event}: {name}"));
    }
    if kind == ToolKind::Shell {
        if input
            .get("command")
            .and_then(Value::as_str)
            .is_none_or(|value| value.trim().is_empty())
        {
            return Err("shell tool_input.command must be a non-empty string".into());
        }
        return Ok(payload.clone());
    }
    if kind == ToolKind::Patch {
        let Some(command) = input
            .get("command")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
        else {
            return Err("apply_patch tool_input.command must be a non-empty string".into());
        };
        if virtual_edits(payload, command).is_empty() {
            return Err("apply_patch command contains no parseable target files".into());
        }
        return Ok(payload.clone());
    }
    let keys: &[&str] = if name == "NotebookEdit" {
        &["file_path", "path", "notebook_path"]
    } else {
        &["file_path", "path"]
    };
    let mut target: Option<&str> = None;
    for key in keys {
        if let Some(value) = input.get(*key) {
            let path = value
                .as_str()
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| format!("edit tool_input.{key} must be a non-empty string"))?;
            if target.is_some_and(|previous| previous != path) {
                return Err("edit tool_input contains conflicting target paths".into());
            }
            target = Some(path);
        }
    }
    let target = target.ok_or("edit tool_input target path is missing")?;
    let mut normalized = payload.clone();
    normalized["tool_input"]["file_path"] = Value::String(target.into());
    Ok(normalized)
}

fn virtual_edits(payload: &Value, command: &str) -> Vec<Value> {
    let mut items: Vec<(String, String)> = Vec::new();
    for line in command.lines() {
        if let Some(rest) = line.strip_prefix("*** Add File: ") {
            items.push(("Write".into(), rest.trim().into()));
        } else if let Some(rest) = line.strip_prefix("*** Update File: ") {
            items.push(("Edit".into(), rest.trim().into()));
        } else if let Some(rest) = line.strip_prefix("*** Delete File: ") {
            items.push(("Edit".into(), rest.trim().into()));
        } else if let Some(rest) = line.strip_prefix("*** Move to: ") {
            items.push(("Write".into(), rest.trim().into()));
        }
    }
    items
        .into_iter()
        .map(|(name, file)| {
            let mut value = payload.clone();
            value["tool_name"] = Value::String(name);
            value["tool_input"] = json!({"file_path": file});
            value
        })
        .collect()
}

fn edits(payload: &Value) -> Vec<Value> {
    if tool_name(payload) == "apply_patch" {
        let command = tool_input(payload)
            .and_then(|input| input.get("command"))
            .and_then(Value::as_str)
            .unwrap_or("");
        virtual_edits(payload, command)
    } else {
        vec![payload.clone()]
    }
}

fn pre_tool(payload: &Value) -> i32 {
    let mut output = HookOutput::default();
    if tool_kind(&tool_name(payload)) == Some(ToolKind::Shell) {
        let handlers: [fn(&Value) -> StepResult; 4] = [
            guards::git_add_all,
            guards::non_ascii_shell,
            guards::cross_project_write,
            guards::user_config_write,
        ];
        for handler in handlers {
            let step = handler(payload);
            output.absorb(&step);
            if step.code != 0 {
                return output.finish("PreToolUse", step.code);
            }
        }
        return output.finish("PreToolUse", 0);
    }
    for edit in edits(payload) {
        let handlers: [fn(&Value) -> StepResult; 2] =
            [guards::cross_project_write, guards::user_config_write];
        for handler in handlers {
            let step = handler(&edit);
            output.absorb(&step);
            if step.code != 0 {
                return output.finish("PreToolUse", step.code);
            }
        }
    }
    output.finish("PreToolUse", 0)
}

fn post_edit(payload: &Value) -> i32 {
    let mut output = HookOutput::default();
    let virtuals = edits(payload);
    for edit in &virtuals {
        let step = post::encoding(edit);
        output.absorb(&step);
        if step.code != 0 {
            eprintln!("[hook-dispatch] encoding_check failed; dependent edit checks skipped.");
            return output.finish("PostToolUse", step.code);
        }
        let _ = project_map::mark_dirty(edit);
    }
    for edit in &virtuals {
        let handlers: [fn(&Value) -> StepResult; 4] = [
            post::instruction_source,
            post::requirements,
            post::cargo_default_run,
            post::fallback_smell,
        ];
        for handler in handlers {
            let step = handler(edit);
            output.absorb(&step);
            if step.code != 0 {
                return output.finish("PostToolUse", step.code);
            }
        }
    }
    output.finish("PostToolUse", 0)
}

fn post_shell(payload: &Value) -> i32 {
    let mut output = HookOutput::default();
    let step = post::test_receipt(payload);
    output.absorb(&step);
    output.finish("PostToolUse", step.code)
}

fn self_test() -> i32 {
    println!("{}", session::build_identity());
    0
}

fn lifecycle(event: &str) -> i32 {
    if matches!(event, "post-compact" | "stop") {
        if event == "stop" {
            let _ = project_map::ensure_if_dirty();
        }
        let step = session::snapshot(event);
        let mut output = HookOutput::default();
        output.absorb(&step);
        return output.finish(
            if event == "post-compact" {
                "PostCompact"
            } else {
                "Stop"
            },
            step.code,
        );
    }
    let _ = project_map::ensure_current();
    let mut output = HookOutput::default();
    let mut first = 0;
    let steps = [
        || session::config_health(false),
        session::enforce_no_effort,
        session::githooks_path,
        session::show_state,
        session::skill_sync,
    ];
    for handler in steps {
        let step = handler();
        output.absorb(&step);
        if first == 0 && step.code != 0 {
            first = step.code;
        }
    }
    output.finish("SessionStart", first)
}

pub fn run(args: Vec<String>) -> i32 {
    if args.as_slice() == ["self-test", "--json"] {
        return self_test();
    }
    if args.as_slice() == ["config-health", "--strict"] {
        let step = session::config_health(true);
        print!("{}", step.stdout);
        eprint!("{}", step.stderr);
        return step.code;
    }
    if args.as_slice() == ["encoding", "--pre-commit"] {
        return post::precommit_encoding();
    }
    if args.as_slice() == ["instruction-source", "--pre-commit"] {
        return post::precommit_instruction_source();
    }
    if args.as_slice() == ["snapshot", "manual"] {
        let step = session::snapshot("manual");
        print!("{}", step.stdout);
        eprint!("{}", step.stderr);
        return step.code;
    }
    if args.as_slice() == ["project-map", "ensure-current"] {
        let step = project_map::ensure_current();
        print!("{}", step.stdout);
        eprint!("{}", step.stderr);
        return step.code;
    }
    if args.len() != 1 {
        eprintln!("usage: bridgeforge-hook EVENT | snapshot manual | project-map ensure-current");
        return 2;
    }
    let event = &args[0];
    if !matches!(
        event.as_str(),
        "pre-tool" | "post-edit" | "post-shell" | "post-compact" | "stop" | "session-start"
    ) {
        eprintln!("unknown hook event route: {event}");
        return 2;
    }
    let raw = read_stdin_bytes();
    if matches!(event.as_str(), "pre-tool" | "post-edit" | "post-shell") {
        let payload = match parse_payload(&raw) {
            Ok(value) => value,
            Err(error) => {
                let outcome = if event == "pre-tool" {
                    "BLOCKED"
                } else {
                    "FAILED"
                };
                eprintln!("[hook-dispatch] {outcome}: {error}");
                return 2;
            }
        };
        let payload = match validate_tool_payload(event, &payload) {
            Ok(value) => value,
            Err(error) => {
                let outcome = if event == "pre-tool" {
                    "BLOCKED"
                } else {
                    "FAILED"
                };
                eprintln!("[hook-dispatch] {outcome}: {error}");
                return 2;
            }
        };
        if event == "pre-tool" {
            return pre_tool(&payload);
        }
        if event == "post-edit" {
            return post_edit(&payload);
        }
        return post_shell(&payload);
    }
    lifecycle(event)
}

#[cfg(all(test, bridgeforge_factory_tests))]
#[path = "../../../scripts/tests/unit/hook.rs"]
mod tests;
