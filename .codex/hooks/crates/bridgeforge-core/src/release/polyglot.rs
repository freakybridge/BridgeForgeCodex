//! Snapshot-driven orchestration shared by the Rust and Python adapters.
use super::lifecycle::{Identity, Transition};
use super::model::{Language, LanguageSnapshot, NativeDependency, NativePackage};
use super::*;

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Adapters {
    pub rust: Option<rust::RustConfig>,
    pub python: Option<python::Config>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SharedInput {
    pub path: String,
    pub packages: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CrossDependency {
    pub owner: String,
    pub target: String,
    pub kind: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Config {
    pub schema_version: u32,
    pub release_policy: String,
    pub strategy: String,
    pub git_sync_setup: Option<setup::Confirmation>,
    pub adapters: Adapters,
    /// An explicit adapter contract for revisions predating schema 3. This
    /// avoids interpreting old Python files with current dependency mappings.
    pub bootstrap_adapters: Option<Adapters>,
    pub packages: Vec<Identity>,
    #[serde(default)]
    pub lifecycle: Vec<Transition>,
    #[serde(default)]
    pub shared_inputs: Vec<SharedInput>,
    #[serde(default)]
    pub product_inputs: Vec<String>,
    #[serde(default)]
    pub dependencies: Vec<CrossDependency>,
    #[serde(default)]
    pub target_versions: BTreeMap<String, String>,
    #[serde(default)]
    pub target_levels: BTreeMap<String, u8>,
}

const CONFIG: &str = ".codex/bridgeforge-version.json";

pub(super) fn parse(payload: &[u8]) -> Result<Option<Config>, String> {
    let value = crate::json::parse_unique_json(payload, "version sync config")?;
    if value["schema_version"].as_u64() != Some(3) {
        return Ok(None);
    }
    setup::confirmation(&value)?;
    let mut cfg: Config = serde_json::from_value(value)
        .map_err(|e| format!("invalid schema 3 release config: {e}"))?;
    for p in cfg.packages.iter_mut().chain(
        cfg.lifecycle
            .iter_mut()
            .flat_map(|t| t.from.iter_mut().chain(t.to.iter_mut())),
    ) {
        if p.language == Language::Python {
            p.name = python_dependencies::normalize_name(&p.name)?;
        }
    }
    if cfg.schema_version != 3
        || cfg.release_policy != "explicit_release"
        || cfg.strategy != "independent"
    {
        return Err("schema 3 requires explicit_release and strategy=independent".into());
    }
    let mut ids = BTreeSet::new();
    let mut manifests = BTreeSet::new();
    for p in &cfg.packages {
        validate_identity(p)?;
        if !ids.insert(&p.id) || !manifests.insert(&p.manifest) {
            return Err("schema 3 has duplicate package id or manifest".into());
        }
    }
    let mut events = BTreeSet::new();
    for t in &cfg.lifecycle {
        t.validate()?;
        if !events.insert(&t.id) {
            return Err("duplicate lifecycle event id".into());
        }
        for p in t.from.iter().chain(&t.to) {
            validate_identity(p)?;
        }
    }
    for p in &cfg.product_inputs {
        relative(p)?;
    }
    for mapping in &cfg.shared_inputs {
        relative(&mapping.path)?;
        if mapping.packages.is_empty() {
            return Err("shared input has no package owners".into());
        }
        if mapping.packages.iter().any(|id| !ids.contains(id)) {
            return Err("shared input references an unknown active package id".into());
        }
    }
    let patterns = cfg
        .product_inputs
        .iter()
        .chain(cfg.shared_inputs.iter().map(|m| &m.path))
        .collect::<Vec<_>>();
    for (i, a) in patterns.iter().enumerate() {
        if patterns[i + 1..].iter().any(|b| owns(a, b) || owns(b, a)) {
            return Err("overlapping shared/product ownership".into());
        }
    }
    for d in &cfg.dependencies {
        if !ids.contains(&d.owner)
            || !ids.contains(&d.target)
            || d.owner == d.target
            || !matches!(d.kind.as_str(), "build" | "runtime" | "dev")
        {
            return Err("invalid explicit package dependency".into());
        }
    }
    if cfg.target_versions.keys().any(|id| !ids.contains(id)) {
        return Err("target version has an unknown package id".into());
    }
    if cfg
        .target_levels
        .iter()
        .any(|(id, level)| !cfg.target_versions.contains_key(id) || !(1..=3).contains(level))
    {
        return Err("explicit target level must bind a target version and be 1..3".into());
    }
    let _ = &cfg.git_sync_setup;
    validate_version_ownership(&cfg)?;
    Ok(Some(cfg))
}

fn validate_identity(p: &Identity) -> Result<(), String> {
    if p.id.is_empty()
        || !p
            .id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"-_.:".contains(&c))
        || p.name.is_empty()
    {
        return Err("package identity requires a stable ASCII id and native name".into());
    }
    if let Some(root) = &p.workspace {
        relative(root)?;
    }
    relative(&p.manifest)
}

fn relative(path: &str) -> Result<(), String> {
    let path = path.strip_suffix('/').unwrap_or(path);
    if path.is_empty()
        || path.contains('\\')
        || Path::new(path).is_absolute()
        || Path::new(path)
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
        || path
            .split('/')
            .any(|p| matches!(p, ".git" | ".runtime" | ".venv" | "target"))
    {
        return Err(format!("unsafe release input path: {path}"));
    }
    Ok(())
}

pub(super) fn load_config(root: &Path) -> Result<Option<Config>, String> {
    config::read_current(root, CONFIG)?
        .map(|p| parse(&p))
        .transpose()
        .map(Option::flatten)
}

#[derive(Clone, Debug, Default)]
struct Snapshot {
    packages: BTreeMap<String, NativePackage>,
    dependencies: Vec<NativeDependency>,
    inputs: model::Inputs,
    inventory: Vec<String>,
    config: Option<Config>,
}

fn owns(prefix: &str, path: &str) -> bool {
    prefix == path || prefix.ends_with('/') && path.starts_with(prefix)
}

fn combine(target: &mut LanguageSnapshot, source: LanguageSnapshot) -> Result<(), String> {
    for (p, native) in source.packages {
        if target.packages.insert(p.clone(), native).is_some() {
            return Err(format!("duplicate adapter ownership: {p}"));
        }
    }
    for (p, bytes) in source.inputs {
        if let Some(old) = target.inputs.insert(p.clone(), bytes.clone()) {
            if old != bytes {
                return Err(format!("adapters disagree on input: {p}"));
            }
        }
    }
    target.dependencies.extend(source.dependencies);
    target.roots.extend(source.roots);
    Ok(())
}

fn snapshot(
    cfg: &Config,
    paths: Vec<String>,
    historical: bool,
    read: &mut impl FnMut(&str) -> Result<Option<Vec<u8>>, String>,
) -> Result<Snapshot, String> {
    let payload = read(CONFIG)?;
    let at = payload.as_deref().map(parse).transpose()?.flatten();
    let mut adapters = if historical {
        at.as_ref()
            .map(|c| c.adapters.clone())
            .or_else(|| cfg.bootstrap_adapters.clone())
            .unwrap_or_else(|| cfg.adapters.clone())
    } else {
        cfg.adapters.clone()
    };
    // Old Rust revisions retain their own scope and lock-validation contract.
    let old_rust = if historical && at.is_none() {
        payload.as_deref().map(config::parse).transpose()?.flatten()
    } else {
        None
    };
    if let Some(old) = &old_rust {
        adapters.rust = Some(old.clone());
    }
    let mut native = LanguageSnapshot::default();
    if let Some(mut rust_cfg) = adapters.rust {
        if historical && at.is_none() && old_rust.is_none() && cfg.bootstrap_adapters.is_none() {
            rust_cfg.workspaces.retain(|p| paths.contains(p));
        }
        if !rust_cfg.workspaces.is_empty() {
            let loaded = if historical && at.is_none() {
                let value = payload
                    .as_deref()
                    .map(|p| crate::json::parse_unique_json(p, "historical config"))
                    .transpose()?;
                let scope = if value
                    .as_ref()
                    .is_some_and(|v| v["schema_version"].as_u64() == Some(1))
                {
                    let value = value.as_ref().unwrap();
                    Some(if config::auto_manifests(value)? {
                        ["Cargo.toml", "package.json"]
                            .iter()
                            .filter(|p| paths.iter().any(|x| x == **p))
                            .map(|p| p.to_string())
                            .collect::<BTreeSet<_>>()
                    } else {
                        value["manifests"]
                            .as_array()
                            .ok_or("historical manifests missing")?
                            .iter()
                            .map(|p| {
                                p.as_str()
                                    .map(str::to_string)
                                    .ok_or("invalid historical manifest".to_string())
                            })
                            .collect::<Result<BTreeSet<_>, _>>()?
                    })
                } else {
                    None
                };
                let workspace = if let Some(scope) = scope {
                    rust::load_legacy_history(&rust_cfg, &paths, &scope, read)?
                } else {
                    rust::load(&rust_cfg, &paths, read)?
                };
                rust::from_workspace(workspace)?
            } else {
                rust::snapshot(&rust_cfg, &paths, read)?
            };
            combine(&mut native, loaded)?;
        }
    }
    if let Some(mut python_cfg) = adapters.python {
        if historical && at.is_none() && cfg.bootstrap_adapters.is_none() {
            python_cfg.projects.retain(|p| paths.contains(p));
            python_cfg
                .version_sources
                .retain(|p, _| python_cfg.projects.contains(p));
            python_cfg
                .import_roots
                .retain(|p, _| python_cfg.projects.contains(p));
            python_cfg
                .local_sources
                .retain(|p, _| python_cfg.projects.contains(p));
            if python_cfg
                .local_sources
                .values()
                .any(|mapping| mapping.values().any(|p| !python_cfg.projects.contains(p)))
            {
                return Err("historical Python sources require explicit bootstrap_adapters".into());
            }
            if python_cfg
                .locks
                .iter()
                .any(|spec| !paths.contains(&spec.path))
            {
                return Err(
                    "historical Python lock scope requires explicit bootstrap_adapters".into(),
                );
            }
        }
        if !python_cfg.projects.is_empty() {
            combine(&mut native, python::snapshot(&python_cfg, &paths, read)?)?;
        }
    }
    let candidates = cfg
        .packages
        .iter()
        .chain(
            cfg.lifecycle
                .iter()
                .flat_map(|t| t.from.iter().chain(&t.to)),
        )
        .chain(at.iter().flat_map(|c| c.packages.iter()))
        .collect::<Vec<_>>();
    let mut manifests = BTreeMap::new();
    let mut result = Snapshot {
        inventory: paths.clone(),
        config: if historical {
            at.clone()
        } else {
            Some(cfg.clone())
        },
        ..Snapshot::default()
    };
    for (manifest, package) in native.packages {
        let declared = if historical {
            at.as_ref().map(|c| &c.packages)
        } else {
            Some(&cfg.packages)
        };
        let mut ids = declared
            .into_iter()
            .flatten()
            .filter(|p| lifecycle::matches(p, &package))
            .map(|p| p.id.clone())
            .collect::<BTreeSet<_>>();
        if ids.is_empty() {
            ids = candidates
                .iter()
                .filter(|p| lifecycle::matches(p, &package))
                .map(|p| p.id.clone())
                .collect();
        }
        if ids.len() != 1 {
            return Err(format!(
                "{manifest}: package needs one unambiguous registered identity"
            ));
        }
        let id = ids.into_iter().next().unwrap();
        if !paths.contains(&manifest) {
            return Err(format!("{manifest}: package is outside Git inventory"));
        }
        manifests.insert(manifest, id.clone());
        if result.packages.insert(id, package).is_some() {
            return Err("two live packages share an identity".into());
        }
    }
    if !historical
        && cfg.packages.iter().any(|p| {
            !result
                .packages
                .get(&p.id)
                .is_some_and(|n| lifecycle::matches(p, n))
        })
    {
        return Err("registered package is missing from current adapter discovery".into());
    }
    for dependency in native.dependencies {
        result.dependencies.push(NativeDependency {
            owner: manifests
                .get(&dependency.owner)
                .ok_or("dependency owner not registered")?
                .clone(),
            target: manifests
                .get(&dependency.target)
                .ok_or("dependency target not registered")?
                .clone(),
            kind: dependency.kind,
        });
    }
    let edges = result
        .config
        .as_ref()
        .map(|c| &c.dependencies)
        .unwrap_or(&cfg.dependencies);
    for edge in edges {
        if result.packages.contains_key(&edge.owner) && result.packages.contains_key(&edge.target) {
            result.dependencies.push(NativeDependency {
                owner: edge.owner.clone(),
                target: edge.target.clone(),
                kind: edge.kind.clone(),
            });
        } else if !historical {
            return Err("cross-language dependency refers to a missing package".into());
        }
    }
    result.inputs = native.inputs;
    result.inputs.insert(CONFIG.into(), payload);
    if !historical {
        validate_version_ownership(cfg)?;
    }
    Ok(result)
}

fn historical(
    root: &Path,
    revision: &str,
    cfg: &Config,
    runner: &dyn ProcessRunner,
) -> Result<Snapshot, String> {
    let paths = history::paths(&history::git(
        root,
        runner,
        &["ls-tree", "-r", "--name-only", "-z", revision],
    )?);
    snapshot(cfg, paths, true, &mut |p| {
        revision_payload(root, revision, p, runner)
    })
}

pub(super) fn verify_historical(
    root: &Path,
    revision: &str,
    cfg: &Config,
    runner: &dyn ProcessRunner,
) -> Result<(), String> {
    historical(root, revision, cfg, runner).map(|_| ())
}

pub(super) fn validate_current(
    root: &Path,
    boundary: &str,
    cfg: &Config,
    runner: &dyn ProcessRunner,
) -> Result<(), String> {
    let head = history::git(root, runner, &["rev-parse", "HEAD"])?;
    let reader = history_cache::Reader::new(root, runner);
    build(
        root,
        "chore: inspect release history",
        vec![],
        head.trim(),
        boundary,
        cfg,
        &reader,
    )?;
    reader.verify_unchanged()
}

pub(super) fn configuration_inputs(
    root: &Path,
    cfg: &Config,
    runner: &dyn ProcessRunner,
) -> Result<BTreeMap<PathBuf, Option<Vec<u8>>>, String> {
    let snapshot = snapshot(
        cfg,
        independent::inventory(root, runner)?,
        false,
        &mut |p| config::read_current(root, p),
    )?;
    Ok(snapshot
        .inputs
        .into_iter()
        .map(|(p, b)| (root.join(p), b))
        .collect())
}

fn level(commit: &CommitInfo) -> u8 {
    if commit.breaking {
        3
    } else if commit.kind == "feat" {
        2
    } else {
        1
    }
}

fn owner(snapshot: &Snapshot, path: &str, cfg: &Config) -> Result<Vec<String>, String> {
    let scope = snapshot.config.as_ref().unwrap_or(cfg);
    if scope.product_inputs.iter().any(|p| owns(p, path)) {
        return Ok(vec![]);
    }
    if let Some(mapping) = scope.shared_inputs.iter().find(|p| owns(&p.path, path)) {
        return Ok(mapping.packages.clone());
    }
    let best = snapshot
        .packages
        .iter()
        .filter_map(|(id, p)| {
            let size = p
                .source_roots
                .iter()
                .filter(|r| {
                    r.is_empty()
                        || path == r.as_str()
                        || path.starts_with(&format!("{}/", r.trim_end_matches('/')))
                })
                .map(String::len)
                .max();
            if p.manifest == path {
                Some((id, usize::MAX))
            } else {
                size.map(|n| (id, n))
            }
        })
        .collect::<Vec<_>>();
    if let Some(max) = best.iter().map(|(_, size)| *size).max() {
        let ids = best
            .into_iter()
            .filter(|(_, size)| *size == max)
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>();
        if ids.len() == 1 {
            return Ok(ids);
        }
    }
    if snapshot.inputs.contains_key(path) {
        return Err(format!(
            "unattributed adapter input: {path}; configure shared_inputs or product_inputs"
        ));
    }
    Err(format!(
        "unattributed release input: {path}; configure ownership"
    ))
}

fn bump_native(package: &NativePackage, impact: u8) -> Result<String, String> {
    match package.language {
        Language::Python => python::bump_version(&package.version, impact),
        Language::Rust => {
            let v: SemVer = package.version.parse()?;
            let n = match impact {
                3 => SemVer {
                    major: v.major.checked_add(1).ok_or("version overflow")?,
                    minor: 0,
                    patch: 0,
                },
                2 => SemVer {
                    major: v.major,
                    minor: v.minor.checked_add(1).ok_or("version overflow")?,
                    patch: 0,
                },
                _ => SemVer {
                    patch: v.patch.checked_add(1).ok_or("version overflow")?,
                    ..v
                },
            };
            Ok(n.to_string())
        }
    }
}

fn add_writes(target: &mut model::Writes, source: model::Writes) -> Result<(), String> {
    for (path, bytes) in source {
        if let Some(old) = target.insert(path.clone(), bytes.clone()) {
            if old != bytes {
                return Err(format!("conflicting adapter writes: {path}"));
            }
        }
    }
    Ok(())
}

pub(super) fn build(
    root: &Path,
    message: &str,
    changed: Vec<String>,
    head: &str,
    boundary: &str,
    cfg: &Config,
    runner: &history_cache::Reader<'_>,
) -> Result<Option<FileReleasePlan>, String> {
    let initial_status = independent::status(root, runner)?;
    let inventory = independent::inventory(root, runner)?;
    let current = snapshot(cfg, inventory.clone(), false, &mut |p| {
        config::read_current(root, p)
    })?;
    let baseline = historical(root, boundary, cfg, runner)?;
    if let Some(old) = &baseline.config {
        if cfg.lifecycle.len() < old.lifecycle.len()
            || cfg.lifecycle[..old.lifecycle.len()] != old.lifecycle
        {
            return Err("published lifecycle history must be preserved verbatim".into());
        }
    }
    let transitions = cfg
        .lifecycle
        .iter()
        .filter(|t| t.base == boundary)
        .cloned()
        .collect::<Vec<_>>();
    let known_old_events = baseline
        .config
        .as_ref()
        .map(|c| c.lifecycle.iter().map(|t| &t.id).collect::<BTreeSet<_>>())
        .unwrap_or_default();
    if cfg
        .lifecycle
        .iter()
        .any(|t| t.base != boundary && !known_old_events.contains(&t.id))
    {
        return Err("new lifecycle event must be bound to the current release baseline".into());
    }
    let mut consumed = BTreeSet::new();
    let mut ordered_transitions = Vec::new();
    let mut previous = baseline.clone();
    let mut infos = Vec::new();
    let mut paths_all = BTreeSet::new();
    let mut impacts = BTreeMap::<String, u8>::new();
    let mut raw_impacts = Vec::new();
    let mut first_versions = baseline
        .packages
        .iter()
        .map(|(id, p)| (id.clone(), p.version.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut retired = BTreeSet::new();
    if let Some(old) = &baseline.config {
        for t in &old.lifecycle {
            for source in &t.from {
                if !t.to.iter().any(|p| p.id == source.id) {
                    retired.insert(source.id.clone());
                }
            }
        }
    }
    if current.packages.keys().any(|id| retired.contains(id)) {
        return Err("published retired identity cannot be reused".into());
    }
    let factory = root.join("templates/managed-skeleton.json").is_file();
    let commits = history::git(
        root,
        runner,
        &[
            "rev-list",
            "--reverse",
            "--topo-order",
            &format!("{boundary}..{head}"),
        ],
    )?;
    for revision in commits.lines().filter(|s| !s.is_empty()) {
        let parents = history::git(
            root,
            runner,
            &["rev-list", "--parents", "-n", "1", revision],
        )?;
        let parts = parents.split_whitespace().collect::<Vec<_>>();
        if parts.len() != 2 {
            return Err(format!(
                "release history requires review of merge/root commit {revision}"
            ));
        }
        let before = parts[1];
        let touched = history::paths(&history::git(
            root,
            runner,
            &[
                "diff",
                "--name-only",
                "--no-renames",
                "-z",
                before,
                revision,
                "--",
            ],
        )?);
        let next = historical(root, revision, cfg, runner)?;
        let used_before = consumed.clone();
        let transition_start = ordered_transitions.len();
        let matched = lifecycle::validate_step(
            &previous.packages,
            &next.packages,
            &transitions,
            &mut consumed,
        )?;
        ordered_transitions.extend(
            matched
                .iter()
                .map(|id| transitions.iter().find(|t| &t.id == id).unwrap().clone()),
        );
        let mechanical = mechanical_paths(
            &previous,
            &next,
            cfg,
            &transitions,
            &used_before,
            &consumed,
            &mut |p| revision_payload(root, before, p, runner),
            &mut |p| revision_payload(root, revision, p, runner),
        )?;
        record_identities(&previous, &next, &mut first_versions, &mut retired)?;
        let classifier = if factory {
            None
        } else {
            let payload = revision_payload(root, revision, ".codex/managed-skeleton.json", runner)?
                .ok_or("release history has no ownership contract")?;
            Some(PayloadClassifier::new(&payload, |p| {
                revision_payload(root, before, p, runner)
            })?)
        };
        let mut business = Vec::new();
        for path in &touched {
            if !factory
                && classifier.as_ref().unwrap().classify(
                    std::slice::from_ref(path),
                    |p| revision_payload(root, before, p, runner),
                    |p| revision_payload(root, revision, p, runner),
                )? == ReleaseKind::SkeletonOnly
            {
                continue;
            }
            business.push(path.clone());
        }
        if !business.is_empty() {
            let info = parse_commit_message(&history::git(
                root,
                runner,
                &["show", "-s", "--format=%B", revision],
            )?)?;
            attribute(
                &previous,
                &next,
                &business,
                &info,
                cfg,
                transition_start,
                &mechanical,
                &mut raw_impacts,
            )?;
            infos.push(info);
        }
        paths_all.extend(touched);
        previous = next;
    }
    let used_before = consumed.clone();
    let transition_start = ordered_transitions.len();
    let matched = lifecycle::validate_step(
        &previous.packages,
        &current.packages,
        &transitions,
        &mut consumed,
    )?;
    ordered_transitions.extend(
        matched
            .iter()
            .map(|id| transitions.iter().find(|t| &t.id == id).unwrap().clone()),
    );
    let mechanical = mechanical_paths(
        &previous,
        &current,
        cfg,
        &transitions,
        &used_before,
        &consumed,
        &mut |p| revision_payload(root, head, p, runner),
        &mut |p| config::read_current(root, p),
    )?;
    record_identities(&previous, &current, &mut first_versions, &mut retired)?;
    let mut actual = changed.into_iter().collect::<BTreeSet<_>>();
    actual.extend(history::paths(&history::git(
        root,
        runner,
        &["diff", "--name-only", "--no-renames", "-z", "HEAD", "--"],
    )?));
    actual.extend(history::paths(&history::git(
        root,
        runner,
        &["ls-files", "--others", "--exclude-standard", "-z"],
    )?));
    let mut business = Vec::new();
    for path in &actual {
        if factory
            || classify(root, std::slice::from_ref(path), runner)? != ReleaseKind::SkeletonOnly
        {
            business.push(path.clone());
        }
    }
    if !business.is_empty() {
        let info = parse_commit_message(message)?;
        attribute(
            &previous,
            &current,
            &business,
            &info,
            cfg,
            transition_start,
            &mechanical,
            &mut raw_impacts,
        )?;
        infos.push(info);
    }
    paths_all.extend(actual);
    if transitions.iter().any(|t| !consumed.contains(&t.id)) {
        return Err("lifecycle declaration has no matching observed transition".into());
    }
    if infos.is_empty() {
        return Ok(None);
    }
    for (id, impact, start) in raw_impacts {
        for live in lifecycle::next_ids(&id, &ordered_transitions[start..]) {
            impacts
                .entry(live)
                .and_modify(|level| *level = (*level).max(impact))
                .or_insert(impact);
        }
    }
    let mut product_level = infos.iter().map(level).max().unwrap_or(1);
    for (index, transition) in ordered_transitions.iter().enumerate() {
        let severity = if transition.compatibility == "breaking" {
            3
        } else {
            1
        };
        product_level = product_level.max(severity);
        if transition.compatibility != "mechanical" {
            for p in &transition.to {
                for id in lifecycle::next_ids(&p.id, &ordered_transitions[index + 1..]) {
                    impacts
                        .entry(id)
                        .and_modify(|v| *v = (*v).max(severity))
                        .or_insert(severity);
                }
            }
        }
    }
    // Include removed/replaced incoming edges, which no longer occur in the
    // final reverse graph, before propagating through current live edges.
    for edge in &baseline.dependencies {
        if current.packages.contains_key(&edge.owner) && !current.dependencies.contains(edge) {
            impacts.entry(edge.owner.clone()).or_insert(1);
        }
    }
    let additions = current
        .packages
        .keys()
        .filter(|id| !baseline.packages.contains_key(*id))
        .cloned()
        .collect::<BTreeSet<_>>();
    for id in &additions {
        impacts.entry(id.clone()).or_insert(1);
    }
    loop {
        let mut progress = false;
        for edge in &current.dependencies {
            if impacts.contains_key(&edge.target) && !impacts.contains_key(&edge.owner) {
                impacts.insert(edge.owner.clone(), 1);
                progress = true;
            }
        }
        if !progress {
            break;
        }
    }
    let mut versions = BTreeMap::new();
    let mut changes = Vec::new();
    let mut events = Vec::new();
    for (id, package) in &current.packages {
        if let Some(impact) = impacts.get(id) {
            let next = if additions.contains(id) {
                package.version.clone()
            } else if let Some(explicit) = cfg.target_versions.get(id) {
                validate_target(package, explicit)?;
                if let Ok(minimum) = bump_native(package, *impact) {
                    validate_minimum(package, explicit, &minimum)?;
                } else if cfg.target_levels.get(id).is_none_or(|level| level < impact) {
                    return Err(format!(
                        "{id}: nonstandard version target requires an explicit impact policy at least {impact}"
                    ));
                }
                explicit.clone()
            } else {
                bump_native(package, *impact)?
            };
            versions.insert(package.manifest.clone(), next.clone());
            changes.push(PackageChange {
                manifest: package.manifest.clone(),
                name: package.name.clone(),
                version_before: package.version.clone(),
                version_after: next,
                reason: if additions.contains(id) {
                    "first_release"
                } else {
                    "direct_or_dependency_change"
                }
                .into(),
                dependencies: current
                    .dependencies
                    .iter()
                    .filter(|e| e.owner == *id && impacts.contains_key(&e.target))
                    .map(|e| current.packages[&e.target].manifest.clone())
                    .collect(),
            });
        }
    }
    for id in baseline
        .packages
        .keys()
        .chain(current.packages.keys())
        .collect::<BTreeSet<_>>()
    {
        let old = baseline.packages.get(id);
        let new = current.packages.get(id);
        if old.is_none()
            || new.is_none()
            || old
                .zip(new)
                .is_some_and(|(a, b)| a.manifest != b.manifest || a.name != b.name)
            || impacts.contains_key(id)
        {
            let operation = if old.is_none() {
                "add"
            } else if new.is_none() {
                "retire"
            } else if old.unwrap().name != new.unwrap().name {
                "rename"
            } else if old.unwrap().manifest != new.unwrap().manifest {
                "move"
            } else {
                "modify"
            };
            events.push(PackageEvent {
                id: id.clone(),
                language: format!("{:?}", old.or(new).unwrap().language).to_lowercase(),
                operation: operation.into(),
                manifest_before: old.map(|p| p.manifest.clone()),
                manifest_after: new.map(|p| p.manifest.clone()),
                name_before: old.map(|p| p.name.clone()),
                name_after: new.map(|p| p.name.clone()),
                version_before: old.map(|p| p.version.clone()),
                version_after: new.map(|p| versions.get(&p.manifest).unwrap_or(&p.version).clone()),
                related: transitions
                    .iter()
                    .filter(|t| t.from.iter().chain(&t.to).any(|p| p.id == *id))
                    .flat_map(|t| t.from.iter().chain(&t.to).map(|p| p.id.clone()))
                    .filter(|p| p.as_str() != id.as_str())
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect(),
            });
        }
    }
    let mut writes = BTreeMap::new();
    if let Some(rust_cfg) = &cfg.adapters.rust {
        let targets = versions
            .iter()
            .filter(|(m, _)| {
                current
                    .packages
                    .values()
                    .any(|p| p.manifest.as_str() == m.as_str() && p.language == Language::Rust)
            })
            .map(|(a, b)| (a.clone(), b.clone()))
            .collect();
        add_writes(
            &mut writes,
            rust::render_versions(
                rust_cfg,
                &inventory,
                &mut |p| config::read_current(root, p),
                &targets,
            )?,
        )?;
    }
    if let Some(python_cfg) = &cfg.adapters.python {
        let targets = versions
            .iter()
            .filter(|(m, _)| {
                current
                    .packages
                    .values()
                    .any(|p| p.manifest.as_str() == m.as_str() && p.language == Language::Python)
            })
            .map(|(a, b)| (a.clone(), b.clone()))
            .collect();
        add_writes(
            &mut writes,
            python::render_versions(
                python_cfg,
                &inventory,
                &mut |p| config::read_current(root, p),
                &targets,
            )?,
        )?;
    }
    let old_version = fs::read_to_string(root.join("VERSION"))
        .map_err(|e| e.to_string())?
        .trim()
        .parse::<SemVer>()?;
    let product = NativePackage {
        manifest: "VERSION".into(),
        name: "product".into(),
        version: old_version.to_string(),
        language: Language::Rust,
        workspace: String::new(),
        source_roots: vec![],
    };
    let new_version = bump_native(&product, product_level)?.parse::<SemVer>()?;
    let kind = if factory {
        ReleaseKind::Factory
    } else {
        ReleaseKind::Business
    };
    let paths = paths_all.into_iter().collect::<Vec<_>>();
    let mut log = String::from_utf8(render_changelog(root, new_version, &infos, &kind, &paths)?)
        .map_err(|e| e.to_string())?;
    let marker = format!("## [{new_version}]");
    let start = log.find(&marker).ok_or("release heading missing")?;
    let end = log[start + marker.len()..]
        .find("\n## [")
        .map(|n| start + marker.len() + n)
        .unwrap_or(log.len());
    let mut section = "\n### Package lifecycle\n\n".to_string();
    for event in &events {
        section.push_str(&format!(
            "- `{}` [{}]: {} -> {} ({})\n",
            event.id,
            event.language,
            event.version_before.as_deref().unwrap_or("none"),
            event.version_after.as_deref().unwrap_or("retired"),
            event.operation
        ));
    }
    for transition in &transitions {
        section.push_str(&format!(
            "- `{}`: {} [{}], {} -> {}\n",
            transition.id,
            transition.operation,
            transition.compatibility,
            transition
                .from
                .iter()
                .map(|p| p.id.as_str())
                .collect::<Vec<_>>()
                .join(", "),
            transition
                .to
                .iter()
                .map(|p| p.id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    log.insert_str(end, &section);
    writes.insert("VERSION".into(), format!("{new_version}\n").into_bytes());
    writes.insert("CHANGELOG.md".into(), log.into_bytes());
    let mut inputs = current
        .inputs
        .into_iter()
        .map(|(p, b)| (root.join(p), b))
        .collect::<BTreeMap<_, _>>();
    for p in paths
        .iter()
        .map(String::as_str)
        .chain([CONFIG, "VERSION", "CHANGELOG.md"])
    {
        inputs
            .entry(root.join(p))
            .or_insert(config::read_current(root, p)?);
    }
    verify_release_inputs(&inputs)?;
    if independent::status(root, runner)? != initial_status
        || independent::inventory(root, runner)? != inventory
        || history::git(root, runner, &["rev-parse", "HEAD"])?.trim() != head
    {
        return Err("repository changed during release planning".into());
    }
    Ok(Some(FileReleasePlan {
        inputs,
        old_version,
        new_version,
        kind,
        writes: writes.into_iter().map(|(p, b)| (root.join(p), b)).collect(),
        package_changes: changes,
        package_events: events,
        lifecycle_events: transitions
            .iter()
            .map(serde_json::to_value)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?,
        release_inventory: Some(inventory),
        release_status: Some(initial_status),
    }))
}

fn validate_target(package: &NativePackage, target: &str) -> Result<(), String> {
    match package.language {
        Language::Rust => {
            if target.parse::<SemVer>()? <= package.version.parse::<SemVer>()? {
                return Err("target version must increase".into());
            }
        }
        Language::Python => {
            let old = package
                .version
                .parse::<pep440_rs::Version>()
                .map_err(|e| e.to_string())?;
            let new = target
                .parse::<pep440_rs::Version>()
                .map_err(|e| e.to_string())?;
            if new <= old {
                return Err("target version must increase".into());
            }
        }
    }
    Ok(())
}

fn record_identities(
    before: &Snapshot,
    after: &Snapshot,
    versions: &mut BTreeMap<String, String>,
    retired: &mut BTreeSet<String>,
) -> Result<(), String> {
    for id in before
        .packages
        .keys()
        .filter(|id| !after.packages.contains_key(*id))
    {
        retired.insert(id.clone());
    }
    for (id, p) in &after.packages {
        if retired.contains(id) {
            return Err(format!("{id}: retired identity cannot be reused"));
        }
        if let Some(version) = versions.get(id) {
            if version != &p.version {
                return Err(format!("{id}: version changed outside managed release"));
            }
        } else {
            versions.insert(id.clone(), p.version.clone());
        }
    }
    Ok(())
}

// An exact current product-file declaration can fill a missing historical
// attribution. It never replaces a historical owner, broadens a directory
// declaration into a retroactive rule, or masks native adapter/package inputs.
fn recover_product_file(before: &Snapshot, after: &Snapshot, path: &str, cfg: &Config) -> bool {
    cfg.product_inputs
        .iter()
        .any(|p| p == path && !p.ends_with('/'))
        && [before, after].iter().all(|snapshot| {
            !snapshot.inputs.contains_key(path)
                && !snapshot.packages.values().any(|package| {
                    package.manifest == path
                        || package.source_roots.iter().any(|root| {
                            root.is_empty()
                                || path == root
                                || path.starts_with(&format!("{}/", root.trim_end_matches('/')))
                        })
                })
        })
}

fn attribute(
    before: &Snapshot,
    after: &Snapshot,
    paths: &[String],
    info: &CommitInfo,
    cfg: &Config,
    transition_start: usize,
    mechanical: &BTreeSet<String>,
    impacts: &mut Vec<(String, u8, usize)>,
) -> Result<(), String> {
    for path in paths {
        if matches!(path.as_str(), CONFIG | "VERSION" | "CHANGELOG.md") || mechanical.contains(path)
        {
            continue;
        }
        let mut ids = BTreeSet::new();
        let old = owner(before, path, cfg);
        let new = owner(after, path, cfg);
        if let Ok(value) = &old {
            ids.extend(value.iter().cloned());
        }
        if let Ok(value) = &new {
            ids.extend(value.iter().cloned());
        }
        if old.is_err() && new.is_err() && !recover_product_file(before, after, path, cfg) {
            return Err(format!("unattributed historical release input: {path}"));
        }
        for id in ids {
            impacts.push((id, level(info), transition_start));
        }
    }
    Ok(())
}

fn validate_minimum(package: &NativePackage, target: &str, minimum: &str) -> Result<(), String> {
    let lower = match package.language {
        Language::Rust => target.parse::<SemVer>()? < minimum.parse::<SemVer>()?,
        Language::Python => {
            target
                .parse::<pep440_rs::Version>()
                .map_err(|e| e.to_string())?
                < minimum
                    .parse::<pep440_rs::Version>()
                    .map_err(|e| e.to_string())?
        }
    };
    if lower {
        return Err(format!(
            "{}: target version is below required impact floor {minimum}",
            package.manifest
        ));
    }
    Ok(())
}

fn validate_version_ownership(cfg: &Config) -> Result<(), String> {
    if let Some(python) = &cfg.adapters.python {
        for (owner, source) in &python.version_sources {
            let path = source.path.to_ascii_lowercase();
            if matches!(path.as_str(), "version" | "changelog.md" | CONFIG)
                || path.starts_with(".codex/")
                || cfg
                    .packages
                    .iter()
                    .any(|p| p.manifest.to_ascii_lowercase() == path && p.manifest != *owner)
            {
                return Err(format!(
                    "{owner}: Python version source conflicts with another package or product output: {}",
                    source.path
                ));
            }
        }
    }
    Ok(())
}

fn mechanical_paths(
    before: &Snapshot,
    after: &Snapshot,
    cfg: &Config,
    transitions: &[Transition],
    used_before: &BTreeSet<String>,
    used_after: &BTreeSet<String>,
    old_read: &mut impl FnMut(&str) -> Result<Option<Vec<u8>>, String>,
    new_read: &mut impl FnMut(&str) -> Result<Option<Vec<u8>>, String>,
) -> Result<BTreeSet<String>, String> {
    let mut result = BTreeSet::new();
    for t in transitions.iter().filter(|t| {
        t.compatibility == "mechanical"
            && !used_before.contains(&t.id)
            && used_after.contains(&t.id)
    }) {
        let from = &t.from[0];
        let to = &t.to[0];
        if !before
            .packages
            .get(&from.id)
            .is_some_and(|p| lifecycle::matches(from, p))
            || !after
                .packages
                .get(&to.id)
                .is_some_and(|p| lifecycle::matches(to, p))
        {
            return Err(format!(
                "{}: cannot prove chained mechanical move from observed snapshots; declare compatible",
                t.id
            ));
        }
        let edges = |s: &Snapshot| {
            s.dependencies
                .iter()
                .filter(|e| e.owner == from.id)
                .map(|e| (e.target.clone(), e.kind.clone()))
                .collect::<BTreeSet<_>>()
        };
        if edges(before) != edges(after) {
            return Err(format!(
                "{}: mechanical move changes dependency identities",
                t.id
            ));
        }
        let collect = |s: &Snapshot,
                       p: &Identity,
                       read: &mut dyn FnMut(&str) -> Result<Option<Vec<u8>>, String>|
         -> Result<BTreeMap<String, Vec<u8>>, String> {
            let dir = p
                .manifest
                .rsplit_once('/')
                .map(|(d, _)| format!("{d}/"))
                .unwrap_or_default();
            let mut values = BTreeMap::new();
            for path in &s.inventory {
                if path == &p.manifest {
                    continue;
                }
                if owner(s, path, cfg).is_ok_and(|ids| ids == vec![p.id.clone()]) {
                    let suffix = path
                        .strip_prefix(&dir)
                        .map(|p| format!("package:{p}"))
                        .unwrap_or_else(|| format!("shared:{path}"));
                    values.insert(suffix, read(path)?.ok_or("mechanical source file missing")?);
                }
            }
            Ok(values)
        };
        if collect(before, from, old_read)? != collect(after, to, new_read)? {
            return Err(format!("{}: mechanical move changes source contents", t.id));
        }
        let old = old_read(&from.manifest)?.ok_or("mechanical manifest missing")?;
        let new = new_read(&to.manifest)?.ok_or("mechanical manifest missing")?;
        if native_semantics(&old)? != native_semantics(&new)? {
            return Err(format!("{}: mechanical move changes native metadata", t.id));
        }
        for (s, p) in [(before, from), (after, to)] {
            result.insert(p.manifest.clone());
            for path in &s.inventory {
                if owner(s, path, cfg).is_ok_and(|ids| ids == vec![p.id.clone()]) {
                    result.insert(path.clone());
                }
            }
        }
        for (id, old) in &before.packages {
            if let Some(new) = after.packages.get(id) {
                if old.manifest != new.manifest
                    || old.name != new.name
                    || old.version != new.version
                {
                    continue;
                }
                let old_edges = before
                    .dependencies
                    .iter()
                    .filter(|e| e.owner == *id)
                    .collect::<Vec<_>>();
                let new_edges = after
                    .dependencies
                    .iter()
                    .filter(|e| e.owner == *id)
                    .collect::<Vec<_>>();
                if old_edges != new_edges {
                    continue;
                }
                if let (Some(a), Some(b)) = (old_read(&old.manifest)?, new_read(&new.manifest)?) {
                    if a != b && native_semantics(&a)? == native_semantics(&b)? {
                        result.insert(new.manifest.clone());
                    }
                }
            }
        }
    }
    Ok(result)
}

fn native_semantics(bytes: &[u8]) -> Result<Value, String> {
    let text = std::str::from_utf8(bytes).map_err(|e| e.to_string())?;
    let doc = text.parse::<toml_edit::DocumentMut>().map_err(
        |_| "mechanical metadata proof requires TOML; declare compatible for legacy metadata",
    )?;
    fn convert(item: &toml_edit::Item, trail: &[String]) -> Result<Value, String> {
        if let Some(table) = item.as_table_like() {
            let mut values = serde_json::Map::new();
            for (k, v) in table.iter() {
                let keys = trail.iter().map(String::as_str).collect::<Vec<_>>();
                let dependency = match keys.as_slice() {
                    [section, _] => matches!(
                        *section,
                        "dependencies" | "dev-dependencies" | "build-dependencies"
                    ),
                    ["workspace", "dependencies", _] => true,
                    ["target", _, section, _] => matches!(
                        *section,
                        "dependencies" | "dev-dependencies" | "build-dependencies"
                    ),
                    ["tool", "poetry", section, _] => {
                        matches!(*section, "dependencies" | "dev-dependencies")
                    }
                    ["tool", "poetry", "group", _, "dependencies", _] => true,
                    _ => false,
                };
                let uv_source = trail.len() == 4 && trail[..3] == ["tool", "uv", "sources"];
                if k == "path" && (dependency || uv_source)
                    || k == "workspace" && trail == ["package"] && v.as_str().is_some()
                {
                    continue;
                }
                let mut next = trail.to_vec();
                next.push(k.into());
                values.insert(k.into(), convert(v, &next)?);
            }
            return Ok(Value::Object(values));
        }
        if let Some(array) = item.as_array_of_tables() {
            return array
                .iter()
                .map(|t| convert(&toml_edit::Item::Table(t.clone()), trail))
                .collect::<Result<Vec<_>, _>>()
                .map(Value::Array);
        }
        fn scalar(v: &toml_edit::Value) -> Result<Value, String> {
            Ok(match v {
                toml_edit::Value::String(s) => json!(s.value()),
                toml_edit::Value::Integer(n) => json!(n.value()),
                toml_edit::Value::Boolean(b) => json!(b.value()),
                toml_edit::Value::Array(a) => {
                    Value::Array(a.iter().map(scalar).collect::<Result<Vec<_>, _>>()?)
                }
                toml_edit::Value::InlineTable(t) => {
                    let mut m = serde_json::Map::new();
                    for (k, v) in t.iter() {
                        m.insert(k.into(), scalar(v)?);
                    }
                    Value::Object(m)
                }
                _ => return Err("unsupported mechanical metadata value".into()),
            })
        }
        item.as_value().map(scalar).unwrap_or(Ok(Value::Null))
    }
    convert(doc.as_item(), &[])
}

#[cfg(all(test, bridgeforge_factory_tests))]
#[path = "../../../../../../scripts/tests/unit/core_release_polyglot_scenarios.rs"]
mod scenario_tests;
#[cfg(all(test, bridgeforge_factory_tests))]
#[path = "../../../../../../scripts/tests/unit/core_release_polyglot.rs"]
mod tests;
