use super::*;
use std::time::{SystemTime, UNIX_EPOCH};

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("bf-persistence-{}-{nonce}", std::process::id()));
        fs::create_dir(&root).unwrap();
        Self(root)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn json_bytes_are_sorted_recursively_and_legacy_entry_remains_compatible() {
    let fixture = Fixture::new();
    let value = serde_json::json!({"z": [{"z": 2, "a": 1}], "a": true});
    let expected =
        b"{\n  \"a\": true,\n  \"z\": [\n    {\n      \"a\": 1,\n      \"z\": 2\n    }\n  ]\n}\n";
    for legacy in [false, true] {
        let path = fixture
            .0
            .join(if legacy { "legacy.json" } else { "common.json" });
        if legacy {
            let result: crate::memory::MemoryResult<()> =
                crate::memory::atomic_write_json(&path, &value);
            result.unwrap();
        } else {
            atomic_write_json(&path, &value).unwrap();
        }
        assert_eq!(fs::read(path).unwrap(), expected);
    }
}

#[test]
fn serialization_failure_preserves_existing_file_and_memory_error_text() {
    struct Refused;
    impl Serialize for Refused {
        fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
            Err(serde::ser::Error::custom("fixture serialization refused"))
        }
    }
    let fixture = Fixture::new();
    let path = fixture.0.join("state.json");
    fs::write(&path, b"unchanged").unwrap();
    let error = atomic_write_json(&path, &Refused).unwrap_err();
    let legacy: crate::memory::MemorySyncError =
        crate::memory::atomic_write_json(&path, &Refused).unwrap_err();
    assert_eq!(error.to_string(), "fixture serialization refused");
    assert_eq!(legacy.to_string(), "fixture serialization refused");
    assert_eq!(fs::read(&path).unwrap(), b"unchanged");
    assert_eq!(fs::read_dir(&fixture.0).unwrap().count(), 1);
}

#[test]
fn failed_replacement_cleans_stage_and_preserves_existing_directory() {
    let fixture = Fixture::new();
    let destination = fixture.0.join("directory");
    fs::create_dir(&destination).unwrap();
    fs::write(destination.join("kept"), b"original").unwrap();
    let error = atomic_write(&destination, b"replacement").unwrap_err();
    let legacy: crate::memory::MemorySyncError =
        crate::memory::atomic_write(&destination, b"replacement").unwrap_err();
    assert_eq!(legacy.to_string(), error.to_string());
    assert_eq!(fs::read(destination.join("kept")).unwrap(), b"original");
    assert_eq!(fs::read_dir(&fixture.0).unwrap().count(), 1);
}

#[test]
fn linked_parent_is_rejected_without_writing_through_it() {
    let fixture = Fixture::new();
    let real = fixture.0.join("real");
    let link = fixture.0.join("link");
    fs::create_dir(&real).unwrap();
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let output = std::process::Command::new("cmd.exe")
            .args(["/D", "/C", "mklink", "/J"])
            .arg(&link)
            .arg(&real)
            .creation_flags(0x08000000)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    #[cfg(unix)]
    std::os::unix::fs::symlink(&real, &link).unwrap();
    let destination = link.join("state.json");
    assert!(is_link_or_reparse(&link).unwrap());
    let error = atomic_write(&destination, b"blocked").unwrap_err();
    assert_eq!(
        error.to_string(),
        format!(
            "directory must exist and must not be a link: {}",
            link.display()
        )
    );
    let legacy: crate::memory::MemorySyncError =
        crate::memory::atomic_write(&destination, b"blocked").unwrap_err();
    assert_eq!(legacy.to_string(), error.to_string());
    assert_eq!(fs::read_dir(&real).unwrap().count(), 0);
}

#[test]
#[cfg(windows)]
fn public_write_preserves_windows_long_path_replacement() {
    let fixture = Fixture::new();
    let parent = fixture
        .0
        .join("a".repeat(90))
        .join("b".repeat(90))
        .join("nested");
    let path = parent.join("state-file-with-a-long-name.json");
    assert!(path.as_os_str().len() > 260);
    atomic_write(&path, b"original").unwrap();
    atomic_write(&path, b"replacement").unwrap();
    assert_eq!(fs::read(&path).unwrap(), b"replacement");
    assert_eq!(fs::read_dir(&parent).unwrap().count(), 1);
}
