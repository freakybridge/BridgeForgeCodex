use super::*;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn common_issue_requires_two_distinct_projects() {
    let projects = vec![
        BatchProject {
            order: 1,
            project_root: "one".into(),
            status: "deferred".into(),
            fingerprint: None,
            safe_count: 0,
            risk_count: 0,
            blockers: vec![],
            git: None,
            skeleton_version: None,
            issue_signature: Some("bridgeforge:shared".into()),
            result: None,
        },
        BatchProject {
            order: 2,
            project_root: "two".into(),
            status: "deferred".into(),
            fingerprint: None,
            safe_count: 0,
            risk_count: 0,
            blockers: vec![],
            git: None,
            skeleton_version: None,
            issue_signature: Some("bridgeforge:shared".into()),
            result: None,
        },
    ];
    let mut counts = BTreeMap::<String, usize>::new();
    for signature in projects
        .iter()
        .filter_map(|item| item.issue_signature.as_ref())
    {
        *counts.entry(signature.clone()).or_default() += 1;
    }
    let common = counts
        .into_iter()
        .find_map(|(signature, count)| (count >= 2).then_some(signature));
    assert_eq!(common.as_deref(), Some("bridgeforge:shared"));
}

#[test]
fn state_machine_is_serial_and_stops_on_repeated_common_issue() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("bridgeforge-batch-{unique}"));
    let state_path = root.join("state.json");
    fs::create_dir_all(&root).unwrap();
    fs::create_dir_all(lock_path(&root).parent().unwrap()).unwrap();
    fs::write(lock_path(&root), b"batch-test").unwrap();
    let project = |order| BatchProject {
        order,
        project_root: format!("project-{order}"),
        status: "planned".into(),
        fingerprint: Some(format!("fingerprint-{order}")),
        safe_count: 1,
        risk_count: 0,
        blockers: vec![],
        git: None,
        skeleton_version: None,
        issue_signature: None,
        result: None,
    };
    let state = BatchState {
        schema: 1,
        batch_id: "batch-test".into(),
        generation: 1,
        status: "active".into(),
        template_root: root.display().to_string(),
        created_at: "now".into(),
        updated_at: "now".into(),
        projects: vec![project(1), project(2)],
        current_order: None,
        common_issue_signature: None,
        factory: FactoryWitness {
            git: GitIdentity {
                head: "head".into(),
                branch: "main".into(),
                upstream: "origin/main".into(),
                remote_url: "https://github.com/freakybridge/BridgeForgeCodex.git".into(),
                common_dir: "git".into(),
                dirty: false,
                dirty_fingerprint: sha([]),
                ahead: 0,
                behind: 0,
            },
            skeleton_fingerprint: "skeleton".into(),
            batch_controller_blob: "batch".into(),
        },
        confirmed_plan_fingerprint: "plan".into(),
    };
    write_state(&state_path, &state).unwrap();
    let guard = StateMutationLock::acquire(&state_path).unwrap();
    assert!(mutate(&state_path, |_| Ok(())).is_err());
    drop(guard);
    mutate(&state_path, |state| {
        state.projects[0].status = "running".into();
        state.current_order = Some(1);
        Ok(())
    })
    .unwrap();
    assert!(finish(&state_path, true, "unverified success".into(), None).is_err());
    let still_running = load(&state_path).unwrap();
    assert_eq!(still_running.current_order, Some(1));
    assert_eq!(still_running.projects[0].status, "running");
    assert!(
        mutate(&state_path, |state| {
            if state.current_order.is_some() {
                Err("a batch project is already running".into())
            } else {
                Ok(())
            }
        })
        .is_err()
    );
    finish(
        &state_path,
        false,
        "first failure".into(),
        Some("bridgeforge:shared".into()),
    )
    .unwrap();
    mutate(&state_path, |state| {
        state.projects[1].status = "running".into();
        state.current_order = Some(2);
        Ok(())
    })
    .unwrap();
    let stopped = finish(
        &state_path,
        false,
        "second failure".into(),
        Some("bridgeforge:shared".into()),
    )
    .unwrap();
    assert_eq!(
        stopped.common_issue_signature.as_deref(),
        Some("bridgeforge:shared")
    );
    let before_retry = fs::read(&state_path).unwrap();
    assert!(
        retry(&state_path, 1, "new-fingerprint")
            .unwrap_err()
            .contains("requires restart")
    );
    assert_eq!(fs::read(&state_path).unwrap(), before_retry);
    assert!(begin(&state_path).is_err());
    assert!(close(&state_path).is_err());
    let mut finished = load(&state_path).unwrap();
    for project in &mut finished.projects {
        project.status = "succeeded".into();
    }
    finished.common_issue_signature = None;
    write_state(&state_path, &finished).unwrap();
    fs::write(lock_path(&root), b"new-active-batch").unwrap();
    assert!(close(&state_path).unwrap_err().contains("different batch"));
    assert_eq!(load(&state_path).unwrap().status, "active");
    fs::write(lock_path(&root), b"batch-test").unwrap();
    assert_eq!(close(&state_path).unwrap().status, "completed");
    fs::write(lock_path(&root), b"new-active-batch").unwrap();
    assert_eq!(close(&state_path).unwrap().status, "completed");
    assert_eq!(
        fs::read_to_string(lock_path(&root)).unwrap(),
        "new-active-batch"
    );
    let factory_guard = factory_batch_lock(&root).unwrap();
    assert!(factory_batch_lock(&root.join(".")).is_err());
    drop(factory_guard);
    fs::remove_dir_all(root).unwrap();
}
