use super::*;
use serde_json::json;
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

struct Fixture {
    root: PathBuf,
    home: PathBuf,
    transcript: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "bf-high-cost-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let home = root.join("home");
        let transcript = home.join("sessions/test.jsonl");
        fs::create_dir_all(transcript.parent().unwrap()).unwrap();
        fs::write(
            &transcript,
            format!(
                "{}\n",
                json!({"type":"session_meta","payload":{"id":"session-1","source":"cli"}})
            ),
        )
        .unwrap();
        Self {
            root,
            home,
            transcript,
        }
    }
    fn payload(&self, turn: &str) -> Value {
        json!({"hook_event_name":"Stop","session_id":"session-1","turn_id":turn,"model":"gpt-6-astra","transcript_path":self.transcript,"stop_hook_active":false,"last_assistant_message":"完成"})
    }
    fn append(&self, turn: &str, second: i64, model: &str, effort: &str) {
        let time = chrono::DateTime::from_timestamp(1_700_000_000 + second, 0)
            .unwrap()
            .to_rfc3339();
        writeln!(fs::OpenOptions::new().append(true).open(&self.transcript).unwrap(), "{}", json!({"timestamp":time,"type":"turn_context","payload":{"turn_id":turn,"model":model,"effort":effort}})).unwrap();
    }
    fn state_path(&self) -> PathBuf {
        self.root.join(format!(
            ".runtime/bridgeforge-codex/high-cost-reminder/{:x}.json",
            Sha256::digest(b"session-1")
        ))
    }
    fn state(&self) -> State {
        serde_json::from_slice(&fs::read(self.state_path()).unwrap()).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn high_cost_thresholds_resume_duplicates_and_old_events() {
    let f = Fixture::new();
    for i in 1..=31 {
        let turn = format!("turn-{i}");
        f.append(&turn, i, "gpt-6-astra", "high");
        let payload = f.payload(&turn);
        let result = observe(&f.root, &f.home, &payload).unwrap();
        assert_eq!(result.is_some(), i % 10 == 0, "{i}");
        if let Some(text) = result {
            assert!(text.contains(&format!(" {i} 轮")));
        }
        assert!(observe(&f.root, &f.home, &payload).unwrap().is_none());
    }
    assert_eq!(f.state().streak, 31);
    f.append("late-turn", 2, "gpt-6-astra", "high");
    assert!(
        observe(&f.root, &f.home, &f.payload("late-turn"))
            .unwrap()
            .is_none()
    );
    assert_eq!(f.state().streak, 31);
    assert!(
        observe(&f.root, &f.home, &f.payload("turn-10"))
            .unwrap()
            .is_none()
    );
    assert_eq!(f.state().streak, 31);
}

#[test]
fn high_cost_normal_and_unknown_break_continuity() {
    let mut state = State::new("s");
    for i in 1..=9 {
        assert!(!state.complete(&i.to_string(), Some(i), Setting::High));
    }
    assert!(!state.complete("normal", Some(10), Setting::Normal));
    assert_eq!(state.streak, 0);
    for i in 11..=19 {
        assert!(!state.complete(&i.to_string(), Some(i), Setting::High));
    }
    assert!(!state.complete("unknown", Some(20), Setting::Unknown));
    assert_eq!(state.streak, 0);
    let f = Fixture::new();
    f.append("one", 1, "gpt-6-astra", "high");
    observe(&f.root, &f.home, &f.payload("one")).unwrap();
    observe(&f.root, &f.home, &f.payload("missing-context")).unwrap();
    assert_eq!(f.state().streak, 0);
    assert_eq!(f.state().setting, Setting::Unknown);
    assert!(f.state().diagnostic.is_some());
}

#[test]
fn high_cost_boolean_or_preserves_unknown_semantics() {
    for effort in ["high", "xhigh", "max", "ultra"] {
        assert_eq!(astra_high("gpt-6-astra", Some(effort)), Some(true));
    }
    assert_eq!(astra_high("gpt-6-astra", Some("medium")), Some(false));
    assert_eq!(astra_high("gpt-6-astra", None), None);
    assert_eq!(astra_high("gpt-5.6-sol", Some("high")), Some(false));
    assert_eq!(classify(Some(true), None), Setting::High);
    assert_eq!(classify(None, Some(true)), Setting::High);
    assert_eq!(classify(Some(false), None), Setting::Unknown);
    assert_eq!(classify(None, Some(false)), Setting::Unknown);
    assert_eq!(classify(Some(false), Some(false)), Setting::Normal);
}

#[test]
fn high_cost_excludes_interrupts_children_empty_and_continuations() {
    let f = Fixture::new();
    for patch in [
        json!({"hook_event_name":"Interrupt"}),
        json!({"agent_id":"child"}),
        json!({"stop_hook_active":true}),
        json!({"last_assistant_message":""}),
    ] {
        let mut payload = f.payload("t");
        payload
            .as_object_mut()
            .unwrap()
            .extend(patch.as_object().unwrap().clone());
        assert!(observe(&f.root, &f.home, &payload).unwrap().is_none());
        assert!(!f.state_path().exists());
    }
    fs::write(
        &f.transcript,
        format!(
            "{}\n",
            json!({"type":"session_meta","payload":{"id":"child","source":{"subagent":{}}}})
        ),
    )
    .unwrap();
    assert!(
        observe(&f.root, &f.home, &f.payload("t"))
            .unwrap()
            .is_none()
    );
    assert!(!f.state_path().exists());
}

#[test]
fn high_cost_corrupt_state_and_lock_conflict_preserve_bytes() {
    let f = Fixture::new();
    f.append("t", 1, "gpt-6-astra", "high");
    observe(&f.root, &f.home, &f.payload("t")).unwrap();
    let path = f.state_path();
    fs::write(&path, b"broken").unwrap();
    assert!(observe(&f.root, &f.home, &f.payload("t2")).is_err());
    assert_eq!(fs::read(&path).unwrap(), b"broken");
    let lock = path.with_extension("lock");
    let _held = crate::file_lock::FileLock::acquire(&lock).unwrap();
    assert!(observe(&f.root, &f.home, &f.payload("t3")).is_err());
    assert_eq!(fs::read(&path).unwrap(), b"broken");
}

#[test]
fn high_cost_foreign_path_unknown_and_state_directory_file_fails_closed() {
    let f = Fixture::new();
    let mut payload = f.payload("t");
    payload["transcript_path"] = json!(f.root.join("private.txt"));
    fs::write(f.root.join("private.txt"), b"not a transcript").unwrap();
    observe(&f.root, &f.home, &payload).unwrap();
    assert_eq!(f.state().setting, Setting::Unknown);
    let other = Fixture::new();
    fs::write(other.root.join(".runtime"), b"preserve").unwrap();
    assert!(observe(&other.root, &other.home, &other.payload("t")).is_err());
    assert_eq!(fs::read(other.root.join(".runtime")).unwrap(), b"preserve");
}

fn row(id: &str, thread: &str, single: &str, mode: &str, text: &str) -> String {
    let text = serde_json::to_string(text).unwrap();
    format!(
        "session_loop{{thread_id=s}}: Submission sub=Submission {{ id: \"{id}\", op: TurnInput {{ request: TurnInputRequest {{ input: UserInput {{ text: {text} }}, thread_settings: ThreadSettingsOverrides {{ service_tier: {thread} }}, start: TurnStartOptions {{ service_tier: {single} }} }}, mode: {mode} }} }}"
    )
}

#[test]
fn high_cost_log_thread_and_single_turn_overrides_are_distinct() {
    let a = row(
        "a",
        "Some(Some(\"default\"))",
        "Some(\"priority\")",
        "NewTurn",
        "",
    );
    let b = row("b", "None", "None", "NewTurn", "");
    assert_eq!(fast_from_rows(&[b.clone(), a.clone()], "a"), Some(true));
    assert_eq!(fast_from_rows(&[b.clone(), a], "b"), Some(false));
    let a = row("a", "Some(Some(\"priority\"))", "None", "NewTurn", "");
    assert_eq!(fast_from_rows(&[b.clone(), a.clone()], "b"), Some(true));
    let c = row("c", "Some(None)", "None", "NewTurn", "");
    assert_eq!(fast_from_rows(&[c, b, a], "c"), Some(false));
    assert_eq!(
        fast_from_rows(&[row("x", "None", "None", "NewTurn", "")], "x"),
        None
    );
    assert_eq!(
        fast_from_rows(
            &[row("x", "Some(Some(\"future\"))", "None", "NewTurn", "")],
            "x"
        ),
        None
    );
}

#[test]
fn high_cost_log_quoted_spoofs_truncation_and_steering_are_conservative() {
    let spoof = "ThreadSettingsOverrides { service_tier: Some(Some(\"priority\")) }";
    assert_eq!(
        fast_from_rows(&[row("a", "Some(None)", "None", "NewTurn", spoof)], "a"),
        Some(false)
    );
    let mut cut = row("a", "Some(None)", "None", "NewTurn", spoof);
    cut.truncate(cut.len() / 2);
    assert_eq!(fast_from_rows(&[cut], "a"), None);
    let first = row("a", "Some(None)", "None", "NewTurn", "");
    let steer = row("b", "None", "None", "Steer { expected_turn_id: \"a\" }", "");
    assert_eq!(fast_from_rows(&[steer, first.clone()], "a"), Some(false));
    let steer = row(
        "b",
        "Some(Some(\"priority\"))",
        "None",
        "Steer { expected_turn_id: \"a\" }",
        "",
    );
    assert_eq!(fast_from_rows(&[steer, first], "a"), None);
}
