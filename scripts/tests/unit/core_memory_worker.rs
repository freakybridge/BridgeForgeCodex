use super::*;
use std::sync::{Arc, Barrier};

struct Fixture(std::path::PathBuf);
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("bf-worker-{}", unique_token()));
        fs::create_dir_all(&root).unwrap();
        Self(root)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn corrupt_worker_reservation_is_preserved_and_recovered_once_concurrently() {
    for payload in [b"".as_slice(), b"{\"schemaVersion\":", b"{}"] {
        let temp = Fixture::new();
        fs::write(temp.0.join("worker.json"), payload).unwrap();
        let barrier = Arc::new(Barrier::new(8));
        let results = (0..8)
            .map(|_| {
                let root = temp.0.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    reserve_worker(&root).unwrap()
                })
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|thread| thread.join().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            results
                .iter()
                .filter(|item| matches!(item, WorkerReservation::Acquired(_)))
                .count(),
            1
        );
        let current = read_worker_state(&temp.0).unwrap().unwrap();
        for result in results {
            let (WorkerReservation::Acquired(value) | WorkerReservation::Reused(value)) = result;
            assert_eq!(value.token, current.token);
        }
        let backups = fs::read_dir(&temp.0)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("worker.invalid-")
            })
            .collect::<Vec<_>>();
        assert_eq!(backups.len(), 1);
        assert_eq!(fs::read(backups[0].path()).unwrap(), payload);
    }
}

#[test]
fn worker_recovery_refuses_a_directory_at_the_reservation_path() {
    let temp = Fixture::new();
    fs::create_dir(temp.0.join("worker.json")).unwrap();
    assert!(
        reserve_worker(&temp.0)
            .unwrap_err()
            .to_string()
            .contains("plain file")
    );
    assert!(temp.0.join("worker.json").is_dir());
}

#[test]
fn new_event_recovers_corrupt_pending_without_discarding_evidence() {
    let temp = Fixture::new();
    fs::write(temp.0.join("pending.json"), b"{truncated").unwrap();
    let pending = mark_pending(&temp.0, "sessionstart").unwrap();
    assert_eq!(pending.triggers, ["sessionstart"]);
    let backup = fs::read_dir(&temp.0)
        .unwrap()
        .filter_map(Result::ok)
        .find(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with("pending.invalid-")
        })
        .unwrap();
    assert_eq!(fs::read(backup.path()).unwrap(), b"{truncated");
}

#[test]
#[cfg(windows)]
fn worker_reuse_checks_process_birth_identity_and_legacy_launch_interval() {
    let temp = Fixture::new();
    let WorkerReservation::Acquired(lease) = reserve_worker(&temp.0).unwrap() else {
        panic!()
    };
    assert!(mark_worker_started(&temp.0, &lease.token, std::process::id()).unwrap());
    let mut current = read_worker_state(&temp.0).unwrap().unwrap();
    assert!(current.process_identity.is_some());
    assert!(worker_is_live(&current));
    current.process_identity = Some(current.process_identity.unwrap() + 1);
    assert!(
        !worker_is_live(&current),
        "a reused PID is not the original worker"
    );
    current.process_identity = None;
    current.started_utc = "2000-01-01T00:00:00Z".into();
    assert!(
        !worker_is_live(&current),
        "legacy state must not pin an unrelated live process"
    );
    atomic_write_json(&temp.0.join("worker.json"), &current).unwrap();
    assert!(matches!(
        reserve_worker(&temp.0).unwrap(),
        WorkerReservation::Acquired(_)
    ));
    assert!(!release_worker(&temp.0, &lease.token).unwrap());
}
