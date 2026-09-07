use super::*;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
#[ignore = "explicit read-only live log cases required; no model calls"]
fn high_cost_live_logs_read_only() {
    let cases: Vec<(String, String, bool)> =
        serde_json::from_str(&std::env::var("BRIDGEFORGE_HIGH_COST_LIVE_CASES").unwrap()).unwrap();
    let home = super::super::codex_home().unwrap();
    for (session, turn, expected) in cases {
        let records = rows(&home.join("logs_2.sqlite"), &session).unwrap();
        assert_eq!(
            super::super::fast_from_rows(&records, &turn),
            Some(expected),
            "session={session}, turn={turn}"
        );
    }
}

#[test]
fn high_cost_windows_reads_only_matching_session_without_database_changes() {
    let directory = std::env::temp_dir().join(format!(
        "bf-high-cost-sql-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&directory).unwrap();
    let path = directory.join("logs.sqlite");
    let api = Api::load().unwrap();
    {
        let filename = CString::new(path.to_str().unwrap()).unwrap();
        let mut db = std::ptr::null_mut();
        assert_eq!(
            unsafe { (api.open)(filename.as_ptr(), &mut db, 6, std::ptr::null()) },
            0
        );
        let db = Connection(&api, db);
        type Exec = unsafe extern "C" fn(
            *mut c_void,
            *const c_char,
            *mut c_void,
            *mut c_void,
            *mut *mut c_char,
        ) -> c_int;
        let address = unsafe { GetProcAddress(api.library, c"sqlite3_exec".as_ptr()) };
        assert!(!address.is_null());
        let exec = unsafe { std::mem::transmute::<*mut c_void, Exec>(address) };
        let sql=c"CREATE TABLE logs(id INTEGER PRIMARY KEY,ts INTEGER,ts_nanos INTEGER,thread_id TEXT,target TEXT,feedback_log_body TEXT); CREATE INDEX idx_logs_thread_id_ts ON logs(thread_id,ts DESC,ts_nanos DESC,id DESC); INSERT INTO logs VALUES (1,1,0,'wanted','codex_core::session::handlers','op: TurnInput { wanted-older'),(2,2,0,'other','codex_core::session::handlers','op: TurnInput { secret'),(3,3,0,'wanted','codex_core::session::handlers','op: TurnInput { wanted-newer'),(4,4,0,'wanted','tool','op: TurnInput { wrong-target');";
        assert_eq!(
            unsafe {
                exec(
                    db.1,
                    sql.as_ptr(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                )
            },
            0
        );
    }
    let before = fs::read(&path).unwrap();
    assert_eq!(
        rows(&path, "wanted").unwrap(),
        vec![
            "op: TurnInput { wanted-newer",
            "op: TurnInput { wanted-older"
        ]
    );
    assert!(rows(&path, "wanted' OR 1=1 --").unwrap().is_empty());
    assert_eq!(fs::read(&path).unwrap(), before);
    let absent = directory.join("absent.sqlite");
    assert!(rows(&absent, "x").is_err());
    assert!(!absent.exists());
    drop(api);
    fs::remove_dir_all(directory).unwrap();
}
