//! Optional checkpointing for explicitly reusable development checks.
use super::*;

fn executable(root: &Path, program: &str) -> Option<PathBuf> {
    if program == "{bridgeforge}" {
        return Some(root.join(if cfg!(windows) {
            ".codex/bin/bridgeforge.exe"
        } else {
            ".codex/bin/bridgeforge"
        }));
    }
    let file = Path::new(program);
    if file.is_absolute() || file.components().count() > 1 {
        return Some(if file.is_absolute() {
            file.into()
        } else {
            root.join(file)
        });
    }
    let extensions = if cfg!(windows) {
        std::env::var("PATHEXT")
            .ok()?
            .split(';')
            .map(str::to_string)
            .collect::<Vec<_>>()
    } else {
        vec![String::new()]
    };
    // An implicit cwd override would make PATH resolution ambiguous. Disable
    // optional reuse rather than claiming to identify a different executable.
    if cfg!(windows)
        && extensions
            .iter()
            .any(|extension| root.join(format!("{program}{extension}")).is_file())
    {
        return None;
    }
    for directory in std::env::split_paths(&std::env::var_os("PATH")?) {
        for extension in std::iter::once("").chain(extensions.iter().map(String::as_str)) {
            let candidate = directory.join(format!("{program}{extension}"));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

pub(super) fn environment(root: &Path, cfg: &Config, runner: &dyn ProcessRunner) -> Option<String> {
    if !cfg.checks.iter().any(|check| check.reusable) {
        return None;
    }
    let toolchain = crate::artifact_cache::environment(root, runner)?;
    let mut programs = BTreeMap::new();
    for check in &cfg.checks {
        let file = executable(root, &check.program)?.canonicalize().ok()?;
        let key = file.to_string_lossy().into_owned();
        if !programs.contains_key(&key) {
            programs.insert(key, payload_sha(&fs::read(file).ok()?));
        }
    }
    // Hash values only; never persist credentials or the environment itself.
    // Invocation telemetry changes on every Codex tool call, independently of
    // check inputs. Opted-in checks must not depend on this trace identifier.
    let vars = std::env::vars_os()
        .filter(|(key, _)| key.as_os_str() != std::ffi::OsStr::new("CODEX_TOOL_CALL_ID"))
        .map(|(key, value)| {
            (
                payload_sha(key.as_encoded_bytes()),
                payload_sha(value.as_encoded_bytes()),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let git = Git { root, runner };
    let git_config = git
        .required(
            &["config", "--null", "--list", "--show-origin"],
            Duration::from_secs(30),
        )
        .ok()?;
    let history = crate::release::planning_history_state(root, runner).ok()?;
    let mut attributes = BTreeMap::new();
    for args in [
        vec!["var", "GIT_ATTR_SYSTEM"],
        vec!["var", "GIT_ATTR_GLOBAL"],
        vec![
            "rev-parse",
            "--path-format=absolute",
            "--git-path",
            "info/attributes",
        ],
    ] {
        let file = PathBuf::from(git.required(&args, Duration::from_secs(30)).ok()?);
        let file = if file.is_absolute() {
            file
        } else {
            root.join(file)
        };
        let payload = match fs::read(&file) {
            Ok(bytes) => Some(payload_sha(&bytes)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(_) => return None,
        };
        attributes.insert(file.to_string_lossy().into_owned(), payload);
    }
    crate::manifest::canonical_sha(&json!({"toolchain":toolchain,"programs":programs,"vars":vars,"git_config":payload_sha(git_config.as_bytes()),"history":history,"attributes":attributes,"os":std::env::consts::OS,"arch":std::env::consts::ARCH})).ok()
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Progress {
    schema: u32,
    context: String,
    checks: Vec<CheckResult>,
    #[serde(skip)]
    enabled: bool,
}

impl Progress {
    pub(super) fn load(
        root: &Path,
        key: &str,
        identity: &RepositoryIdentity,
        index: &str,
        environment: Option<&str>,
    ) -> Result<Self, String> {
        let context = crate::manifest::canonical_sha(&json!({
            "inputs":key,"root":root.canonicalize().map_err(|e| e.to_string())?,
            "head":identity.head_oid,"branch":identity.symbolic_head,"git_dir":identity.git_dir,
            "common_dir":identity.common_dir,"git_config":identity.common_config_sha256,
            "index":payload_sha(index.as_bytes()),"environment":environment
        }))?;
        let mut result = Self {
            schema: 1,
            context,
            checks: Vec::new(),
            enabled: environment.is_some(),
        };
        if result.enabled {
            if let Ok(saved) = read_json::<Self>(&path(root, &format!("{CACHE}/checks.json"))?) {
                if saved.schema == 1 && saved.context == result.context {
                    result.checks = saved.checks;
                }
            }
        }
        Ok(result)
    }

    pub(super) fn reuse(&self, root: &Path, check: &Check) -> Option<CheckResult> {
        if !self.enabled {
            return None;
        }
        self.checks
            .iter()
            .find(|result| {
                result.exit_code == 0
                    && serde_json::to_value(&result.check).ok() == serde_json::to_value(check).ok()
                    && load_blob(root, &result.log_sha256).is_ok()
            })
            .cloned()
    }

    pub(super) fn save(&mut self, root: &Path, checks: &[CheckResult]) -> Result<(), String> {
        if !self.enabled {
            return Ok(());
        }
        for result in checks.iter().filter(|result| result.check.reusable) {
            self.checks.retain(|old| old.check.id != result.check.id);
            self.checks.push(result.clone());
        }
        crate::persistence::atomic_write_json(
            &path(root, &format!("{CACHE}/checks.json"))?,
            &serde_json::to_value(&self).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())
    }
}
