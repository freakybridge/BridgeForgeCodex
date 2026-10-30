use super::*;
use std::os::windows::process::CommandExt;
use std::{
    fs,
    io::{BufRead, BufReader, Write},
    process::{Command, Stdio},
};

#[test]
fn native_process_context_matches_current_directory_and_sdk_layout() {
    assert_eq!(size_of::<Parameters<usize>>(), 128);
    assert_eq!(size_of::<Parameters<u32>>(), 72);
    assert_eq!(size_of::<Peb32>(), 20);
    assert_eq!(
        offset_of!(Parameters<usize>, command_line),
        offset_of!(RTL_USER_PROCESS_PARAMETERS, CommandLine)
    );
    let handle = open(
        std::process::id(),
        PROCESS_QUERY_INFORMATION | PROCESS_VM_READ,
    )
    .unwrap();
    let (cwd, command) = context(handle.as_raw_handle()).unwrap();
    assert_eq!(
        cwd.canonicalize().unwrap(),
        std::env::current_dir().unwrap().canonicalize().unwrap()
    );
    assert!(!command.is_empty());
}

#[test]
fn external_interpreter_cwd_blocks_but_unrelated_cwd_is_allowed() {
    let base = test_artifacts::root(env!("CARGO_MANIFEST_DIR"), "process-context")
        .join(format!("{}", std::process::id()));
    let root = base.join("project 空格");
    let unrelated = base.join("project-other");
    fs::create_dir_all(&root).unwrap();
    fs::create_dir_all(&unrelated).unwrap();
    let root = root.canonicalize().unwrap();
    let system = PathBuf::from(std::env::var_os("SystemRoot").unwrap());
    let interpreters = [
        system.join("System32/WindowsPowerShell/v1.0/powershell.exe"),
        system.join("SysWOW64/WindowsPowerShell/v1.0/powershell.exe"),
    ];
    for interpreter in interpreters {
        assert!(
            interpreter.is_file(),
            "required native/WOW64 fixture interpreter missing: {}",
            interpreter.display()
        );
        for (cwd, blocked) in [(&root, true), (&unrelated, false)] {
            let mut child = Command::new(&interpreter)
                .creation_flags(0x08000000)
                .args([
                    "-NoProfile",
                    "-NonInteractive",
                    "-Command",
                    "[Console]::WriteLine('ready'); [Console]::In.ReadLine() | Out-Null",
                ])
                .current_dir(cwd)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
                .unwrap();
            let mut line = String::new();
            BufReader::new(child.stdout.take().unwrap())
                .read_line(&mut line)
                .unwrap();
            assert_eq!(line.trim(), "ready");
            let result = project_quiet(&root);
            child
                .stdin
                .as_mut()
                .unwrap()
                .write_all(b"finish\n")
                .unwrap();
            assert!(child.wait().unwrap().success());
            if blocked {
                let error = result.unwrap_err();
                assert!(error.contains("references project path"), "{error}");
            } else {
                result.unwrap();
            }
        }
    }
    fs::remove_dir_all(base).unwrap();
}

#[test]
fn command_path_arguments_preserve_spaces_and_match_only_absolute_paths() {
    let args = command_paths(std::ffi::OsStr::new(
        "tool.exe --root=\"D:\\project 空格\" relative.txt \"D:\\other\\file.txt\"",
    ))
    .unwrap();
    assert_eq!(
        args,
        vec![
            PathBuf::from("D:\\project 空格"),
            PathBuf::from("D:\\other\\file.txt")
        ]
    );
}

#[test]
fn missing_process_output_paths_match_case_insensitively_without_prefix_confusion() {
    let base = test_artifacts::root(env!("CARGO_MANIFEST_DIR"), "process-paths")
        .join(std::process::id().to_string());
    let root = base.join("Project=Data");
    fs::create_dir_all(&root).unwrap();
    let canonical = root.canonicalize().unwrap();
    let missing = root.join(".build/future/result.bin");
    assert!(reference_within(&missing, &canonical).unwrap());
    assert!(
        reference_within(
            &PathBuf::from(missing.to_string_lossy().to_lowercase()),
            &canonical
        )
        .unwrap()
    );
    assert!(
        !reference_within(
            &base.join("Project=Data-other/.build/result.bin"),
            &canonical
        )
        .unwrap()
    );
    let args = command_paths(missing.as_os_str()).unwrap();
    assert_eq!(args, vec![missing]);
    fs::remove_dir_all(base).unwrap();
}

#[allow(dead_code)]
#[path = "../support/artifact_paths.rs"]
mod test_artifacts;
