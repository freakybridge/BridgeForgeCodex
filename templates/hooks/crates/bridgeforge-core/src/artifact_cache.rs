//! Optional, project-local cache of verified generated artifacts.

use crate::{ProcessRequest, ProcessRunner, file_lock::FileLock};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub(crate) const DIRECTORY: &str = ".runtime/bridgeforge-codex/build-artifact-cache";
const OWNER: &str = "bridgeforge-generated-artifact-cache-v1";
const MAX_BYTES: u64 = 256 * 1024 * 1024;
const KEEP: usize = 2;

#[derive(Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Owner {
    schema: u32,
    owner: String,
    project: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    schema: u32,
    owner: String,
    project: String,
    identity: Value,
    key: String,
    binary_sha256: String,
    receipt_sha256: String,
    used: u128,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Operation {
    schema: u32,
    owner: String,
    project: String,
    key: String,
    binary: String,
    nonce: u128,
    kind: String,
}

pub(crate) struct Cache {
    root: PathBuf,
    directory: PathBuf,
    project: String,
    environment: String,
    build_context: PathBuf,
    protected: BTreeSet<String>,
    _lock: FileLock,
}

fn hash(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn now() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}

fn safe(path: &Path) -> Result<(), String> {
    for ancestor in path.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(_)
                if crate::persistence::is_link_or_reparse(ancestor)
                    .map_err(|e| e.to_string())? =>
            {
                return Err("artifact cache traverses a linked path".into());
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(())
}

fn plain(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    safe(path)?;
    let metadata = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if !metadata.is_file() || metadata.len() > limit {
        return Err("artifact cache file is not a bounded regular file".into());
    }
    let file = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > limit {
        return Err("artifact cache file grew beyond its bound".into());
    }
    safe(path)?;
    Ok(bytes)
}

fn parse<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, String> {
    let value = crate::baseline::parse_unique_json(bytes, "artifact cache")?;
    serde_json::from_value(value).map_err(|e| e.to_string())
}

fn platform() -> &'static str {
    if cfg!(windows) {
        "windows-x86_64"
    } else if cfg!(target_os = "linux") {
        "linux-x86_64"
    } else {
        "macos-x86_64"
    }
}

fn environment(root: &Path, runner: &dyn ProcessRunner) -> Option<String> {
    // Unmodelled compiler/native overrides make the optional cache unavailable.
    for (key, _) in std::env::vars_os() {
        let key = key.to_string_lossy();
        if matches!(
            key.as_ref(),
            "RUSTFLAGS"
                | "CARGO_ENCODED_RUSTFLAGS"
                | "RUSTC"
                | "RUSTC_WRAPPER"
                | "RUSTC_WORKSPACE_WRAPPER"
                | "CC"
                | "CXX"
                | "CFLAGS"
                | "CXXFLAGS"
                | "AR"
                | "LD"
                | "RUSTC_LINKER"
        ) || key.starts_with("CARGO_BUILD_")
            || key.starts_with("CARGO_TARGET_")
            || key.starts_with("CARGO_PROFILE_")
        {
            return None;
        }
    }
    let mut versions = Vec::new();
    for program in ["cargo", "rustc"] {
        let mut request = ProcessRequest::new(program, root);
        request.args = vec!["--version".into()];
        request.timeout = Duration::from_secs(15);
        let output = runner.run(&request).ok()?;
        if output.timed_out || output.code != 0 {
            return None;
        }
        let text = String::from_utf8(output.stdout).ok()?;
        let mut parts = text.split_whitespace();
        if parts.next() != Some(program) {
            return None;
        }
        let version = parts.next()?.parse::<crate::release::SemVer>().ok()?;
        let minimum = format!("{}.0", env!("CARGO_PKG_RUST_VERSION"))
            .parse::<crate::release::SemVer>()
            .ok()?;
        if version < minimum {
            return None;
        }
        versions.push(text.trim().to_string());
    }
    let home = std::env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
                .map(|p| PathBuf::from(p).join(".cargo"))
        });
    let mut configs = BTreeMap::new();
    let mut locations = Vec::new();
    for (index, parent) in root.ancestors().enumerate() {
        locations.push((format!("ancestor-{index}"), parent.join(".cargo")));
    }
    if let Some(home) = home {
        locations.push(("cargo-home".into(), home));
    }
    for (label, directory) in locations {
        for name in ["config", "config.toml"] {
            let path = directory.join(name);
            match fs::symlink_metadata(&path) {
                Ok(_) => {
                    let bytes = plain(&path, 1024 * 1024).ok()?;
                    let text = std::str::from_utf8(&bytes).ok()?;
                    let document = text.parse::<toml_edit::DocumentMut>().ok()?;
                    if document.iter().any(|(name, _)| {
                        !matches!(name, "net" | "http" | "registries" | "registry" | "source")
                    }) {
                        return None;
                    }
                    if document
                        .get("source")
                        .and_then(|s| s.as_table_like())
                        .is_some_and(|sources| {
                            sources.iter().any(|(_, source)| {
                                source.get("directory").is_some()
                                    || source.get("local-registry").is_some()
                            })
                        })
                    {
                        return None;
                    }
                    configs.insert(format!("{label}:{name}"), hash(&bytes));
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(_) => return None,
            }
        }
    }
    crate::manifest::canonical_sha(&json!({"versions":versions,"configs":configs})).ok()
}

fn binary_name(identity: &Value) -> Result<String, String> {
    let name = identity["binary_name"]
        .as_str()
        .ok_or("cache binary_name missing")?;
    if !matches!(name, "bridgeforge" | "bridgeforge-hook") {
        return Err("cache binary_name unsupported".into());
    }
    Ok(if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.into()
    })
}

fn footprint(path: &Path) -> Result<u64, String> {
    safe(path)?;
    let metadata = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if metadata.is_file() {
        return Ok(metadata.len());
    }
    if !metadata.is_dir() {
        return Err("cache unowned content cannot be bounded".into());
    }
    let mut bytes = 0u64;
    for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
        bytes = bytes
            .checked_add(footprint(&entry.map_err(|e| e.to_string())?.path())?)
            .ok_or("cache footprint overflow")?;
        if bytes > MAX_BYTES {
            return Err("cache unowned content exceeds capacity".into());
        }
    }
    Ok(bytes)
}

impl Cache {
    pub(crate) fn open(
        root: &Path,
        build_context: &Path,
        runner: &dyn ProcessRunner,
    ) -> Option<Self> {
        Self::try_open(root, build_context, runner).ok().flatten()
    }

    fn try_open(
        root: &Path,
        build_context: &Path,
        runner: &dyn ProcessRunner,
    ) -> Result<Option<Self>, String> {
        let mut ignored = ProcessRequest::new("git", root);
        ignored.args = vec!["check-ignore".into(), "--quiet".into(), DIRECTORY.into()];
        ignored.env_remove = std::env::vars_os()
            .map(|(key, _)| key)
            .filter(|key| key.to_string_lossy().starts_with("GIT_"))
            .collect();
        let output = runner.run(&ignored).map_err(|e| e.to_string())?;
        if output.timed_out || output.code != 0 {
            return Ok(None);
        }
        let Some(environment) = environment(build_context, runner) else {
            return Ok(None);
        };
        let project = root
            .canonicalize()
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .into_owned();
        let directory = root.join(DIRECTORY);
        safe(&directory)?;
        let owner = Owner {
            schema: 1,
            owner: OWNER.into(),
            project: project.clone(),
        };
        if !directory.exists() {
            fs::create_dir_all(directory.parent().ok_or("cache parent missing")?)
                .map_err(|e| e.to_string())?;
            fs::create_dir(&directory).map_err(|e| e.to_string())?;
            crate::persistence::atomic_write_json(&directory.join("owner.json"), &owner)
                .map_err(|e| e.to_string())?;
        }
        let actual: Owner = parse(&plain(&directory.join("owner.json"), 16 * 1024)?)?;
        if actual != owner {
            return Err("artifact cache root ownership mismatch".into());
        }
        let lock = FileLock::acquire(&directory.join("cache.lock"))?;
        let cache = Self {
            root: root.to_path_buf(),
            directory,
            project,
            environment,
            build_context: build_context.to_path_buf(),
            protected: BTreeSet::new(),
            _lock: lock,
        };
        cache.recover()?;
        Ok(Some(cache))
    }

    fn identity(&self, asset: &Value) -> Value {
        json!({"id":asset["id"],"platform":platform(),"binary_name":asset["build"]["binary_name"],
            "source_tree_sha256":asset["source_tree_sha256"],"lockfile_sha256":asset["lockfile_sha256"],
            "build_recipe_sha256":asset["build_recipe_sha256"],"self_test_sha256":asset["self_test_sha256"],
            "build":asset["build"],"self_test":asset["self_test"],"environment":self.environment})
    }

    fn key(identity: &Value) -> Result<String, String> {
        Ok(crate::manifest::canonical_sha(identity)?
            .trim_start_matches("sha256:")
            .into())
    }

    fn owned(&self, key: &str) -> Result<(Entry, Vec<u8>), String> {
        if key.len() != 64 || !key.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("cache key invalid".into());
        }
        let directory = self.directory.join(key);
        let meta = plain(&directory.join("entry.json"), 64 * 1024)?;
        let entry: Entry = parse(&meta)?;
        if entry.schema != 1
            || entry.owner != OWNER
            || entry.project != self.project
            || entry.key != key
            || Self::key(&entry.identity)? != key
        {
            return Err("cache entry ownership mismatch".into());
        }
        let expected = BTreeSet::from([
            "entry.json".to_string(),
            "receipt.json".into(),
            binary_name(&entry.identity)?,
        ]);
        let actual = fs::read_dir(&directory)
            .map_err(|e| e.to_string())?
            .map(|e| e.map(|e| e.file_name().to_string_lossy().into_owned()))
            .collect::<Result<BTreeSet<_>, _>>()
            .map_err(|e| e.to_string())?;
        if actual != expected {
            return Err("cache entry contains unowned files".into());
        }
        for name in expected {
            let path = directory.join(name);
            safe(&path)?;
            let metadata = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
            if !metadata.is_file() || metadata.len() > MAX_BYTES {
                return Err("cache entry file is not bounded and regular".into());
            }
        }
        Ok((entry, meta))
    }

    pub(crate) fn load(
        &mut self,
        asset: &Value,
        runner: &dyn ProcessRunner,
    ) -> Option<(Vec<u8>, Vec<u8>)> {
        self.try_load(asset, runner).ok()
    }

    fn try_load(
        &mut self,
        asset: &Value,
        runner: &dyn ProcessRunner,
    ) -> Result<(Vec<u8>, Vec<u8>), String> {
        let identity = self.identity(asset);
        let key = Self::key(&identity)?;
        let (mut entry, meta) = self.owned(&key)?;
        if entry.identity != identity {
            return Err("cache contract mismatch".into());
        }
        let directory = self.directory.join(&key);
        let binary = directory.join(binary_name(&identity)?);
        let payload = plain(&binary, MAX_BYTES)?;
        let receipt = plain(&directory.join("receipt.json"), 64 * 1024)?;
        if hash(&payload) != entry.binary_sha256 || hash(&receipt) != entry.receipt_sha256 {
            return Err("cache payload hash mismatch".into());
        }
        crate::baseline::verify_generated_payload(asset, &payload, &receipt)?;
        let mut request = ProcessRequest::new(binary.as_os_str(), &self.root);
        request.args = asset["self_test"]["args"]
            .as_array()
            .ok_or("cache self-test missing")?
            .iter()
            .map(|arg| {
                arg.as_str()
                    .map(Into::into)
                    .ok_or("cache self-test argument invalid")
            })
            .collect::<Result<_, _>>()?;
        request.timeout = Duration::from_secs(60);
        let output = runner.run(&request).map_err(|e| e.to_string())?;
        let actual: Value = serde_json::from_slice(&output.stdout).map_err(|e| e.to_string())?;
        let expected = asset["self_test"]["expected_json"]
            .as_object()
            .ok_or("cache self-test invalid")?;
        if output.timed_out
            || output.code != 0
            || !expected.iter().all(|(k, v)| actual.get(k) == Some(v))
            || plain(&binary, MAX_BYTES)? != payload
            || plain(&directory.join("receipt.json"), 64 * 1024)? != receipt
            || plain(&directory.join("entry.json"), 64 * 1024)? != meta
            || environment(&self.build_context, runner).as_deref()
                != Some(self.environment.as_str())
        {
            return Err("cache changed or self-test failed".into());
        }
        entry.used = self.next_used()?;
        let op = self.operation(&key, binary_name(&identity)?, "touch")?;
        let temp = directory.join(format!(".entry-meta-{}.tmp", op.nonce));
        let bytes = serde_json::to_vec_pretty(&entry).map_err(|e| e.to_string())?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(|e| e.to_string())?;
        file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .map_err(|e| e.to_string())?;
        drop(file);
        let written = crate::persistence::atomic_replace_file(&temp, &directory.join("entry.json"));
        if written.is_err() {
            let _ = self.finish_operation(&op);
            return Err("cache recency publication failed".into());
        }
        self.finish_operation(&op)?;
        self.protected.insert(key);
        Ok((payload, receipt))
    }

    fn operation_paths(&self, op: &Operation) -> Result<(PathBuf, PathBuf), String> {
        if op.schema != 1
            || op.owner != OWNER
            || op.project != self.project
            || op.key.len() != 64
            || !op.key.bytes().all(|b| b.is_ascii_hexdigit())
            || !matches!(op.kind.as_str(), "stage" | "retired" | "touch")
            || !matches!(
                op.binary.as_str(),
                "bridgeforge.exe" | "bridgeforge-hook.exe" | "bridgeforge" | "bridgeforge-hook"
            )
        {
            return Err("cache operation ownership invalid".into());
        }
        Ok((
            self.directory.join(format!("operation-{}.json", op.nonce)),
            self.directory
                .join(format!(".{}.{}.{}", op.key, op.nonce, op.kind)),
        ))
    }

    fn operation(&self, key: &str, binary: String, kind: &str) -> Result<Operation, String> {
        let op = Operation {
            schema: 1,
            owner: OWNER.into(),
            project: self.project.clone(),
            key: key.into(),
            binary,
            nonce: now(),
            kind: kind.into(),
        };
        let (journal, directory) = self.operation_paths(&op)?;
        safe(&directory)?;
        if directory.exists() || journal.exists() {
            return Err("cache operation collision".into());
        }
        let bytes = serde_json::to_vec(&op).map_err(|e| e.to_string())?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&journal)
            .map_err(|e| e.to_string())?;
        file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .map_err(|e| e.to_string())?;
        Ok(op)
    }

    fn finish_operation(&self, op: &Operation) -> Result<(), String> {
        let (journal, directory) = self.operation_paths(op)?;
        let owner_bytes = plain(&journal, 16 * 1024)?;
        let actual: Operation = parse(&owner_bytes)?;
        if serde_json::to_value(actual).unwrap() != serde_json::to_value(op).unwrap() {
            return Err("cache operation changed".into());
        }
        if op.kind == "touch" {
            let entry_directory = self.directory.join(&op.key);
            let entry: Entry = parse(&plain(&entry_directory.join("entry.json"), 64 * 1024)?)?;
            if entry.owner != OWNER
                || entry.project != self.project
                || entry.key != op.key
                || Self::key(&entry.identity)? != op.key
            {
                return Err("cache recency ownership mismatch".into());
            }
            let temp = entry_directory.join(format!(".entry-meta-{}.tmp", op.nonce));
            match fs::symlink_metadata(&temp) {
                Ok(_) => {
                    plain(&temp, 64 * 1024)?;
                    fs::remove_file(temp).map_err(|e| e.to_string())?;
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e.to_string()),
            }
            return fs::remove_file(journal).map_err(|e| e.to_string());
        }
        safe(&directory)?;
        if directory.exists() {
            let allowed = BTreeSet::from([
                op.binary.clone(),
                "receipt.json".into(),
                "entry.json".into(),
            ]);
            let actual = fs::read_dir(&directory)
                .map_err(|e| e.to_string())?
                .map(|e| e.map(|e| e.file_name().to_string_lossy().into_owned()))
                .collect::<Result<BTreeSet<_>, _>>()
                .map_err(|e| e.to_string())?;
            if !actual.is_subset(&allowed) {
                return Err("cache operation contains unowned files".into());
            }
            for name in [op.binary.as_str(), "receipt.json", "entry.json"] {
                let path = directory.join(name);
                match fs::symlink_metadata(&path) {
                    Ok(_) => {
                        plain(&path, MAX_BYTES)?;
                        #[cfg(windows)]
                        {
                            use std::os::windows::fs::OpenOptionsExt;
                            let _probe = OpenOptions::new()
                                .read(true)
                                .share_mode(0)
                                .open(&path)
                                .map_err(|e| e.to_string())?;
                        }
                        if plain(&journal, 16 * 1024)? != owner_bytes {
                            return Err("cache operation owner drift".into());
                        }
                        fs::remove_file(path).map_err(|e| e.to_string())?;
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                    Err(e) => return Err(e.to_string()),
                }
            }
            fs::remove_dir(&directory).map_err(|e| e.to_string())?;
        }
        fs::remove_file(journal).map_err(|e| e.to_string())
    }

    fn recover(&self) -> Result<(), String> {
        for item in fs::read_dir(&self.directory).map_err(|e| e.to_string())? {
            let item = item.map_err(|e| e.to_string())?;
            let name = item.file_name().to_string_lossy().into_owned();
            if name.starts_with("operation-") && name.ends_with(".json") {
                // A crash before a complete journal is synced has not claimed any artifact.
                // Preserve it as unowned capacity; it must not poison unrelated entries.
                if let Ok(bytes) = plain(&item.path(), 16 * 1024) {
                    if let Ok(op) = parse::<Operation>(&bytes) {
                        if self
                            .operation_paths(&op)
                            .is_ok_and(|paths| paths.0 == item.path())
                        {
                            let _ = self.finish_operation(&op);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn next_used(&self) -> Result<u128, String> {
        let mut maximum = 0u128;
        for item in fs::read_dir(&self.directory).map_err(|e| e.to_string())? {
            let item = item.map_err(|e| e.to_string())?;
            let key = item.file_name().to_string_lossy().into_owned();
            if let Ok((entry, _)) = self.owned(&key) {
                maximum = maximum.max(entry.used);
            }
        }
        maximum
            .checked_add(1)
            .ok_or("cache recency sequence exhausted".into())
    }

    fn remove(&self, entry: &Entry) -> Result<(), String> {
        if self.protected.contains(&entry.key) {
            return Err("cache entry is leased".into());
        }
        let (actual, meta) = self.owned(&entry.key)?;
        if serde_json::to_value(actual).unwrap() != serde_json::to_value(entry).unwrap() {
            return Err("cache entry changed before cleanup".into());
        }
        let directory = self.directory.join(&entry.key);
        let binary = binary_name(&entry.identity)?;
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            let _probe = OpenOptions::new()
                .read(true)
                .share_mode(0)
                .open(directory.join(&binary))
                .map_err(|e| e.to_string())?;
        }
        if plain(&directory.join("entry.json"), 64 * 1024)? != meta {
            return Err("cache owner drift".into());
        }
        let op = self.operation(&entry.key, binary, "retired")?;
        let (journal, retired) = self.operation_paths(&op)?;
        if let Err(error) = fs::rename(&directory, &retired) {
            let _ = fs::remove_file(journal);
            return Err(error.to_string());
        }
        self.finish_operation(&op)
    }

    fn reserve(&self, id: &Value, bytes: u64) -> Result<(), String> {
        if bytes > MAX_BYTES {
            return Err("cache artifact exceeds capacity".into());
        }
        let mut entries = Vec::new();
        let mut unknown_bytes = 0u64;
        for item in fs::read_dir(&self.directory).map_err(|e| e.to_string())? {
            let item = item.map_err(|e| e.to_string())?;
            let key = item.file_name().to_string_lossy().into_owned();
            if matches!(key.as_str(), "owner.json" | "cache.lock") {
                continue;
            }
            let (entry, _) = match self.owned(&key) {
                Ok(owned) => owned,
                Err(_) => {
                    unknown_bytes = unknown_bytes
                        .checked_add(footprint(&item.path())?)
                        .ok_or("cache footprint overflow")?;
                    continue;
                }
            };
            let directory = self.directory.join(&key);
            let size = [
                "entry.json".to_string(),
                "receipt.json".into(),
                binary_name(&entry.identity)?,
            ]
            .iter()
            .map(|n| fs::metadata(directory.join(n)).map(|m| m.len()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?
            .iter()
            .sum::<u64>();
            entries.push((entry, size));
        }
        entries.sort_by_key(|(entry, _)| entry.used);
        loop {
            let same = entries
                .iter()
                .filter(|(e, _)| {
                    e.identity["id"] == id["id"] && e.identity["platform"] == id["platform"]
                })
                .count();
            let base = fs::metadata(self.directory.join("owner.json"))
                .map_err(|e| e.to_string())?
                .len()
                + fs::metadata(self.directory.join("cache.lock"))
                    .map_err(|e| e.to_string())?
                    .len();
            let total =
                entries.iter().map(|(_, size)| size).sum::<u64>() + bytes + base + unknown_bytes;
            if same < KEEP && total <= MAX_BYTES {
                return Ok(());
            }
            let position = entries
                .iter()
                .position(|(e, _)| {
                    !self.protected.contains(&e.key)
                        && (total > MAX_BYTES
                            || (e.identity["id"] == id["id"]
                                && e.identity["platform"] == id["platform"]))
                })
                .ok_or("cache capacity is occupied")?;
            self.remove(&entries[position].0)?;
            entries.remove(position);
        }
    }

    pub(crate) fn store(
        &mut self,
        asset: &Value,
        payload: &[u8],
        receipt: &[u8],
        runner: &dyn ProcessRunner,
    ) -> Result<(), String> {
        crate::baseline::verify_generated_payload(asset, payload, receipt)?;
        if environment(&self.build_context, runner).as_deref() != Some(self.environment.as_str()) {
            return Err("cache compiler inputs changed".into());
        }
        let identity = self.identity(asset);
        let key = Self::key(&identity)?;
        let destination = self.directory.join(&key);
        if destination.exists() {
            let (entry, _) = self.owned(&key)?;
            self.remove(&entry)?;
        }
        let entry = Entry {
            schema: 1,
            owner: OWNER.into(),
            project: self.project.clone(),
            identity: identity.clone(),
            key: key.clone(),
            binary_sha256: hash(payload),
            receipt_sha256: hash(receipt),
            used: self.next_used()?,
        };
        let meta = serde_json::to_vec_pretty(&entry).map_err(|e| e.to_string())?;
        self.reserve(
            &identity,
            (payload.len() + receipt.len() + meta.len() + 1 + 16 * 1024) as u64,
        )?;
        let op = self.operation(&key, binary_name(&identity)?, "stage")?;
        let (_, stage) = self.operation_paths(&op)?;
        if let Err(error) = fs::create_dir(&stage) {
            let _ = fs::remove_file(self.operation_paths(&op)?.0);
            return Err(error.to_string());
        }
        let result = (|| {
            for (name, bytes) in [
                (binary_name(&identity)?, payload),
                ("receipt.json".into(), receipt),
            ] {
                let path = stage.join(name);
                let mut file = OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&path)
                    .map_err(|e| e.to_string())?;
                file.write_all(bytes)
                    .and_then(|_| file.sync_all())
                    .map_err(|e| e.to_string())?;
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    fs::set_permissions(&path, fs::Permissions::from_mode(0o700))
                        .map_err(|e| e.to_string())?;
                }
            }
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(stage.join("entry.json"))
                .map_err(|e| e.to_string())?;
            let mut encoded = meta.clone();
            encoded.push(b'\n');
            file.write_all(&encoded)
                .and_then(|_| file.sync_all())
                .map_err(|e| e.to_string())?;
            drop(file);
            safe(&destination)?;
            fs::rename(&stage, &destination).map_err(|e| e.to_string())?;
            self.protected.insert(key);
            Ok(())
        })();
        let cleanup = self.finish_operation(&op);
        if result.is_ok() {
            cleanup?;
        }
        result
    }
}

#[cfg(all(test, bridgeforge_factory_tests))]
#[path = "../../../../../scripts/tests/unit/core_artifact_cache.rs"]
mod tests;
