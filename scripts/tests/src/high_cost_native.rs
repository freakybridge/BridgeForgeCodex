//! Optional native Stop contract smoke using a loopback-only Responses fixture.
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[test]
#[ignore = "requires BRIDGEFORGE_CODEX_EXE and BRIDGEFORGE_TEST_HOOK; no paid model"]
fn high_cost_native_stop_reaches_tenth_round_without_extra_request() {
    let exe = std::env::var_os("BRIDGEFORGE_CODEX_EXE").expect("native Codex path");
    let hook = std::env::var("BRIDGEFORGE_TEST_HOOK").expect("built Hook path");
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("bf-cost-native-{}-{stamp}", std::process::id()));
    let home = root.join("home");
    let project = root.join("project");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&project).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let port = listener.local_addr().unwrap().port();
    fs::write(
        home.join("config.toml"),
        format!(
            r#"
model = "gpt-6-astra"
model_reasoning_effort = "high"
model_provider = "fixture"
[features]
hooks = true
[model_providers.fixture]
name = "offline cost fixture"
base_url = "http://127.0.0.1:{port}/v1"
wire_api = "responses"
requires_openai_auth = false
supports_websockets = false
request_max_retries = 0
stream_max_retries = 0
"#
        ),
    )
    .unwrap();
    fs::write(
        home.join("hooks.json"),
        serde_json::to_vec(&json!({"hooks":{"Stop":[{"hooks":[{
        "type":"command","command":format!("\"{hook}\" stop"),
        "commandWindows":format!("& '{}' stop", hook.replace('\'', "''")),"timeout":10
        }]}]}}))
        .unwrap(),
    )
    .unwrap();
    let mut command = Command::new(exe);
    command
        .arg("app-server")
        .current_dir(&project)
        .env("CODEX_HOME", &home)
        .env("BRIDGEFORGE_HOOK_ROOT", &project)
        .env_remove("OPENAI_API_KEY")
        .env_remove("CODEX_API_KEY")
        .env_remove("OPENAI_BASE_URL")
        .env_remove("OPENAI_ORG_ID")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(fs::File::create(root.join("stderr.txt")).unwrap());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = command.spawn().unwrap();
    let mut input = child.stdin.take().unwrap();
    let output = child.stdout.take().unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    let output_path = root.join("stdout.jsonl");
    let reader = std::thread::spawn(move || {
        let mut receipt = fs::File::create(output_path).unwrap();
        for line in BufReader::new(output).lines().map_while(Result::ok) {
            writeln!(receipt, "{line}").unwrap();
            if let Ok(event) = serde_json::from_str::<Value>(&line) {
                let _ = tx.send(event);
            }
        }
    });
    writeln!(input,"{}",json!({"id":1,"method":"initialize","params":{"clientInfo":{"name":"bridgeforge_fixture","version":"1"},"capabilities":{"experimentalApi":true}}})).unwrap();
    let started = Instant::now();
    let mut count = 0;
    let mut state_path = None;
    let mut completed = false;
    while started.elapsed() < Duration::from_secs(45) {
        for event in rx.try_iter() {
            assert!(event.get("error").is_none(), "app-server error: {event}");
            if event["id"] == 1 {
                writeln!(input, "{}", json!({"method":"initialized","params":{}})).unwrap();
                writeln!(
                    input,
                    "{}",
                    json!({"id":9,"method":"hooks/list","params":{"cwds":[project]}})
                )
                .unwrap();
            } else if event["id"] == 9 {
                let hooks = event["result"]["data"][0]["hooks"]
                    .as_array()
                    .expect("fixture hooks listed");
                assert_eq!(
                    hooks.len(),
                    1,
                    "isolated home must contain exactly our Stop hook: {event}"
                );
                let key = hooks[0]["key"].as_str().unwrap();
                let hash = hooks[0]["currentHash"].as_str().unwrap();
                assert!(!key.contains('\''));
                writeln!(
                    fs::OpenOptions::new()
                        .append(true)
                        .open(home.join("config.toml"))
                        .unwrap(),
                    "\n[hooks.state.'{key}']\ntrusted_hash = \"{hash}\""
                )
                .unwrap();
                writeln!(input,"{}",json!({"id":2,"method":"thread/start","params":{"cwd":project,"model":"gpt-6-astra","approvalPolicy":"never","sandbox":"read-only"}})).unwrap();
            } else if event["id"] == 2 {
                writeln!(input,"{}",json!({"id":3,"method":"turn/start","params":{"threadId":event["result"]["thread"]["id"],"input":[{"type":"text","text":"Reply with fixture complete."}]}})).unwrap();
            } else if event["method"] == "turn/completed" {
                assert_eq!(event["params"]["turn"]["status"], "completed", "{event}");
                completed = true;
            }
        }
        if completed {
            break;
        }
        match listener.accept() {
            Ok((mut stream, _)) => {
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut data = Vec::new();
                let end = loop {
                    let mut buffer = [0; 8192];
                    let size = stream.read(&mut buffer).unwrap();
                    assert!(size > 0 && data.len() < 4_000_000);
                    data.extend_from_slice(&buffer[..size]);
                    if let Some(i) = data.windows(4).position(|w| w == b"\r\n\r\n") {
                        break i + 4;
                    }
                };
                let headers = String::from_utf8_lossy(&data[..end]).to_lowercase();
                let size: usize = headers
                    .lines()
                    .find_map(|s| s.strip_prefix("content-length:"))
                    .unwrap()
                    .trim()
                    .parse()
                    .unwrap();
                while data.len() - end < size {
                    let mut buffer = [0; 8192];
                    let size = stream.read(&mut buffer).unwrap();
                    assert!(size > 0);
                    data.extend_from_slice(&buffer[..size]);
                }
                let request: Value = serde_json::from_slice(&data[end..end + size]).unwrap();
                assert_eq!(request["model"], "gpt-6-astra");
                assert!(request["service_tier"].is_null() || request["service_tier"] == "default");
                count += 1;
                if state_path.is_none() {
                    let transcript = transcript(&home.join("sessions"))
                        .expect("native transcript exists before response");
                    let head = fs::read_to_string(transcript).unwrap();
                    let meta: Value = serde_json::from_str(head.lines().next().unwrap()).unwrap();
                    let session = meta["payload"]["id"].as_str().unwrap();
                    let path = project.join(format!(
                        ".runtime/bridgeforge-codex/high-cost-reminder/{:x}.json",
                        Sha256::digest(session.as_bytes())
                    ));
                    fs::create_dir_all(path.parent().unwrap()).unwrap();
                    fs::write(&path, serde_json::to_vec(&json!({"schema":1,"session_id":session,"streak":9,"last_time":0,"processed":[],"setting":"High","diagnostic":null})).unwrap()).unwrap();
                    state_path = Some(path);
                }
                let item = json!({"type":"message","id":"msg_fixture","role":"assistant","status":"completed","content":[{"type":"output_text","text":"fixture complete","annotations":[]}]});
                let events = [
                    json!({"type":"response.created","response":{"id":"resp_fixture","status":"in_progress"}}),
                    json!({"type":"response.output_item.added","output_index":0,"item":item}),
                    json!({"type":"response.output_item.done","output_index":0,"item":item}),
                    json!({"type":"response.completed","response":{"id":"resp_fixture","status":"completed","output":[item],"usage":{"input_tokens":1,"output_tokens":1,"total_tokens":2}}}),
                ];
                let body: String = events
                    .iter()
                    .map(|e| format!("event: {}\ndata: {}\n\n", e["type"].as_str().unwrap(), e))
                    .collect();
                write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(error) => panic!("loopback failed: {error}"),
        }
        if child.try_wait().unwrap().is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let _ = child.kill();
    let _ = child.wait();
    reader.join().unwrap();
    eprintln!("native Stop receipt: {}", root.display());
    assert!(
        completed,
        "{}",
        fs::read_to_string(root.join("stderr.txt")).unwrap()
    );
    assert_eq!(
        count, 1,
        "reminder must not generate a continuation request"
    );
    let state: Value = serde_json::from_slice(&fs::read(state_path.unwrap()).unwrap()).unwrap();
    assert_eq!(
        state["streak"], 10,
        "actual native Stop must count once: {state}"
    );
    let output = fs::read_to_string(root.join("stdout.jsonl")).unwrap();
    let notifications: Vec<Value> = output
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let notification = notifications
        .iter()
        .find(|event| event["method"] == "hook/completed")
        .expect("native Hook completion notification");
    let run = &notification["params"]["run"];
    assert_eq!(run["status"], "completed", "{run}");
    assert!(
        run["entries"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["kind"] == "warning"
                && entry["text"]
                    .as_str()
                    .is_some_and(|text| text.contains("高耗能提醒") && text.contains(" 10 轮"))),
        "native notification must carry the tenth-round UI warning: {run}"
    );
}

fn transcript(directory: &Path) -> Option<std::path::PathBuf> {
    for entry in fs::read_dir(directory).ok()?.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = transcript(&path) {
                return Some(found);
            }
        } else if path.extension().is_some_and(|e| e == "jsonl") {
            return Some(path);
        }
    }
    None
}
