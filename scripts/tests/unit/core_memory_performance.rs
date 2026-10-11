//! Explicit, offline release benchmarks. Production code is unchanged.
use super::*;
use serde_json::{Value, json};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

fn output_directory() -> PathBuf {
    let path = PathBuf::from(
        std::env::var_os("BRIDGEFORGE_R06_OUTPUT").expect("explicit output directory required"),
    );
    assert!(path.is_absolute());
    fs::create_dir_all(&path).unwrap();
    path
}

fn summary(samples: &[f64]) -> Value {
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    let n = sorted.len();
    let median = if n % 2 == 0 {
        (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
    } else {
        sorted[n / 2]
    };
    json!({"samples_ms": samples, "median_ms": median, "min_ms": sorted[0], "max_ms": sorted[n - 1]})
}

#[cfg(windows)]
fn process_memory() -> Value {
    // ABI matches the installed windows-sys PROCESS_MEMORY_COUNTERS definition.
    // Kept test-local to avoid changing product dependency features for a probe.
    #[repr(C)]
    struct Counters {
        cb: u32,
        faults: u32,
        peak_working_set: usize,
        working_set: usize,
        quota_peak_paged: usize,
        quota_paged: usize,
        quota_peak_nonpaged: usize,
        quota_nonpaged: usize,
        pagefile: usize,
        peak_pagefile: usize,
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn K32GetProcessMemoryInfo(
            process: *mut std::ffi::c_void,
            counters: *mut Counters,
            size: u32,
        ) -> i32;
    }
    let mut counters: Counters = unsafe { std::mem::zeroed() };
    counters.cb = std::mem::size_of::<Counters>() as u32;
    let ok = unsafe {
        K32GetProcessMemoryInfo(
            windows_sys::Win32::System::Threading::GetCurrentProcess(),
            &mut counters,
            std::mem::size_of::<Counters>() as u32,
        )
    };
    assert_ne!(ok, 0, "{}", std::io::Error::last_os_error());
    json!({"working_set_bytes":counters.working_set,"peak_working_set_bytes":counters.peak_working_set})
}

#[cfg(not(windows))]
fn process_memory() -> Value {
    json!({"unavailable":"Windows working-set probe"})
}

#[test]
#[ignore = "R06 benchmark subprocess helper; explicit fixture and output required"]
fn r06_snapshot_child() {
    assert!(!cfg!(debug_assertions), "use --release");
    let snapshot = PathBuf::from(std::env::var_os("BRIDGEFORGE_R06_SNAPSHOT").unwrap());
    let mode = std::env::var("BRIDGEFORGE_R06_MODE").unwrap();
    let manifest: SnapshotManifest =
        serde_json::from_slice(&fs::read(snapshot.join("snapshot-manifest.json")).unwrap())
            .unwrap();
    let total: usize = std::env::var("BRIDGEFORGE_R06_BYTES")
        .unwrap()
        .parse()
        .unwrap();
    let initial_memory = process_memory();
    let measure = || {
        let started = Instant::now();
        match mode.as_str() {
            "verify" => {
                verify_snapshot(&snapshot, &manifest).unwrap();
                started.elapsed().as_secs_f64() * 1000.0
            }
            "load" => {
                let files = snapshot_files(&snapshot).unwrap();
                let elapsed = started.elapsed().as_secs_f64() * 1000.0;
                // Assertions and dropping the returned buffers are outside the timer.
                assert_eq!(files.len(), manifest.files.len());
                assert_eq!(files.values().map(Vec::len).sum::<usize>(), total);
                for entry in &manifest.files {
                    assert_eq!(crate::memory::sha256_hex(&files[&entry.path]), entry.sha256);
                }
                elapsed
            }
            _ => panic!("unknown benchmark mode"),
        }
    };
    let first_ms = measure();
    let _warmup = measure();
    let samples = (0..10).map(|_| measure()).collect::<Vec<_>>();
    let result = json!({"schema":1,"mode":mode,"first_ms":first_ms,"warm":summary(&samples),
        "files":manifest.files.len(),"bytes":total,"initial_memory":initial_memory,"final_memory":process_memory()});
    atomic_write_json(&output_directory().join("child.json"), &result).unwrap();
}

#[test]
#[ignore = "explicit R06 offline release snapshot benchmark"]
fn r06_snapshot_baseline() {
    assert!(!cfg!(debug_assertions), "use --release");
    let output = output_directory();
    let executable = std::env::current_exe().unwrap();
    let mut cases = Vec::new();
    for (label, count, size) in [
        ("small", 100usize, 4096usize),
        ("many-small", 2000, 8192),
        ("few-large", 20, 1048576),
    ] {
        let fixture = RestoreFixture::new();
        let source = fixture.0.join("source");
        let snapshot = fixture.0.join("snapshot");
        let payload = vec![0x5a; size];
        for i in 0..count {
            write_memory(&source, &format!("d{:02}/note_{i:05}.md", i % 20), &payload);
        }
        build_snapshot(&source, &snapshot, 1).unwrap();
        for mode in ["verify", "load"] {
            let child_output = output.join(format!("{label}-{mode}"));
            let mut request = ProcessRequest::new(executable.as_os_str(), &fixture.0);
            request.args = [
                "--ignored",
                "--exact",
                "memory::remote::tests::performance::r06_snapshot_child",
                "--test-threads=1",
            ]
            .iter()
            .map(Into::into)
            .collect();
            request.env.insert(
                "BRIDGEFORGE_R06_SNAPSHOT".into(),
                snapshot.as_os_str().into(),
            );
            request.env.insert(
                "BRIDGEFORGE_R06_OUTPUT".into(),
                child_output.as_os_str().into(),
            );
            request
                .env
                .insert("BRIDGEFORGE_R06_MODE".into(), mode.into());
            request.env.insert(
                "BRIDGEFORGE_R06_BYTES".into(),
                (count * size).to_string().into(),
            );
            request.timeout = Duration::from_secs(180);
            let result = crate::SystemProcessRunner.run(&request).unwrap();
            assert_eq!(
                result.code,
                0,
                "{}\n{}",
                String::from_utf8_lossy(&result.stdout),
                String::from_utf8_lossy(&result.stderr)
            );
            assert!(!result.timed_out);
            let mut result: Value =
                serde_json::from_slice(&fs::read(child_output.join("child.json")).unwrap())
                    .unwrap();
            result["label"] = json!(label);
            cases.push(result);
        }
    }
    let report = json!({"schema":1,"build":"release","platform":std::env::consts::OS,
        "arch":std::env::consts::ARCH,"binary_sha256":crate::memory::sha256_hex(&fs::read(&executable).unwrap()),
        "sampling":"first + 1 warmup + 10 samples; no OS cache purge; construction/assertions/drop excluded",
        "cases":cases});
    atomic_write_json(&output.join("snapshot-baseline.json"), &report).unwrap();
    println!("{report}");
}

struct TimedLocalGit {
    runner: LocalGit,
    nanos: AtomicU64,
    calls: AtomicU64,
}
impl ProcessRunner for TimedLocalGit {
    fn run(&self, request: &ProcessRequest) -> std::io::Result<crate::ProcessOutput> {
        let started = Instant::now();
        let result = self.runner.run(request);
        self.nanos
            .fetch_add(started.elapsed().as_nanos() as u64, Ordering::Relaxed);
        self.calls.fetch_add(1, Ordering::Relaxed);
        result
    }
}

#[test]
#[ignore = "explicit R06 release local Git restore benchmark; no external network"]
fn r06_local_restore_baseline() {
    assert!(!cfg!(debug_assertions), "use --release");
    let output = output_directory();
    let fixture = RestoreFixture::new();
    let runner = TimedLocalGit {
        runner: LocalGit::new(&fixture.0),
        nanos: AtomicU64::new(0),
        calls: AtomicU64::new(0),
    };
    let source = fixture.0.join("source");
    for i in 0..100 {
        let mut payload = vec![0x5a; 4096];
        payload[..8].copy_from_slice(&(i as u64).to_le_bytes());
        write_memory(&source, &format!("note_{i:05}.md"), &payload);
    }
    assert_eq!(
        reconcile(
            &source,
            &fixture.0.join("source-state"),
            TEST_REMOTE,
            &runner
        )
        .unwrap(),
        "push"
    );
    let mut samples = Vec::new();
    let mut details = Vec::new();
    for i in 0..3 {
        let target = fixture.0.join(format!("restore-{i}"));
        let state = fixture.0.join(format!("state-{i}"));
        runner.nanos.store(0, Ordering::Relaxed);
        runner.calls.store(0, Ordering::Relaxed);
        let started = Instant::now();
        assert_eq!(
            reconcile(&target, &state, TEST_REMOTE, &runner).unwrap(),
            "restore"
        );
        let elapsed = started.elapsed().as_secs_f64() * 1000.0;
        let runner_ms = runner.nanos.load(Ordering::Relaxed) as f64 / 1_000_000.0;
        samples.push(elapsed);
        details.push(json!({"total_ms":elapsed,"process_runner_ms":runner_ms,"runner_calls":runner.calls.load(Ordering::Relaxed)}));
        for i in 0..100 {
            let mut expected = vec![0x5a; 4096];
            expected[..8].copy_from_slice(&(i as u64).to_le_bytes());
            assert_eq!(
                fs::read(target.join(format!("note_{i:05}.md"))).unwrap(),
                expected
            );
        }
    }
    let result = json!({"schema":1,"build":"release","files":100,"bytes":409600,
        "scenario":"local bare remote to fresh destination; initial push excluded; assertions after timing",
        "total":summary(&samples),"details":details});
    atomic_write_json(&output.join("local-restore-baseline.json"), &result).unwrap();
    println!("{result}");
}
