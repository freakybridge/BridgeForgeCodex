use std::io::{Read, Write};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    assert_eq!(args.len(), 7);
    assert_eq!(
        &args[1..6],
        [
            "memory-sync",
            "hook-run",
            "--event",
            "SessionStart",
            "--codex-home"
        ]
    );
    assert_eq!(args[6], std::env::var("BF_PROBE_HOME").unwrap());
    let mut input = Vec::new();
    std::io::stdin().read_to_end(&mut input).unwrap();
    std::io::stdout().write_all(&input).unwrap();
    std::io::stderr().write_all(b"probe-stderr\n").unwrap();
    std::process::exit(std::env::var("BF_PROBE_EXIT").unwrap().parse().unwrap());
}
