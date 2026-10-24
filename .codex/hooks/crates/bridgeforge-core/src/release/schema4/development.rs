//! Package development closure. No Git/index/ref writes and no product version writes.
use super::*;
use crate::CommandOutcome;

pub(super) const RECORD: &str = ".codex/package-closures.json";
const JOURNAL: &str = ".runtime/bridgeforge-codex/develop-transaction.json";

pub(crate) fn require_completed_transaction(root: &Path) -> Result<(), String> {
    if root.join(JOURNAL).exists() {
        Err(
            "unfinished development transaction; run develop --recover before Git synchronization"
                .into(),
        )
    } else {
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CheckCommand {
    id: String,
    program: String,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default = "default_timeout")]
    timeout_seconds: u64,
    #[serde(default)]
    artifacts: Vec<String>,
    #[serde(default)]
    inputs: Vec<String>,
    #[serde(default)]
    kind: String,
}
fn default_timeout() -> u64 {
    300
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Stability {
    contracts: Vec<String>,
    checks: Vec<CheckCommand>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(in crate::release) struct Policy {
    checks: Vec<CheckCommand>,
    #[serde(default)]
    stability: Option<Stability>,
}
impl Policy {
    pub(super) fn validate(&self, id: &str) -> Result<(), String> {
        if id.trim().is_empty() || self.checks.is_empty() {
            return Err(format!(
                "{id}: development requires a nonempty mandatory check list"
            ));
        }
        let mut ids = BTreeSet::new();
        for check in self
            .checks
            .iter()
            .chain(self.stability.iter().flat_map(|s| &s.checks))
        {
            if check.id.trim().is_empty()
                || !ids.insert(&check.id)
                || check.program.trim().is_empty()
                || !(1..=14400).contains(&check.timeout_seconds)
            {
                return Err(format!(
                    "{id}: invalid/duplicate development check or timeout"
                ));
            }
            for path in check.artifacts.iter().chain(&check.inputs) {
                safe_relative(Path::new("."), path)?;
            }
        }
        if let Some(stability) = &self.stability {
            for path in &stability.contracts {
                safe_relative(Path::new("."), path)?;
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Entry {
    pub id: String,
    pub before: Option<String>,
    pub after: Option<String>,
    pub content: String,
    pub evidence: String,
    pub level: u8,
    pub compatibility: String,
    pub acceptance: Option<Value>,
    pub reason: String,
    pub stability: String,
    pub checks: Vec<Value>,
    pub artifacts: BTreeMap<String, String>,
    pub previous: String,
    pub hash: String,
}
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Ledger {
    pub entries: Vec<Entry>,
}
impl Ledger {
    pub(super) fn parse(bytes: Option<&[u8]>) -> Result<Self, String> {
        let ledger: Self = match bytes {
            None => Self::default(),
            Some(bytes) => {
                serde_json::from_value(crate::json::parse_unique_json(bytes, "package closures")?)
                    .map_err(|e| format!("invalid package closure ledger: {e}"))?
            }
        };
        let mut previous = String::new();
        let mut versions = BTreeMap::new();
        for entry in &ledger.entries {
            let mut unhashed = entry.clone();
            unhashed.hash.clear();
            if entry.previous != previous
                || entry.hash != hash(&unhashed)?
                || entry.reason.trim().is_empty()
                || entry.level > 3
                || entry.checks.is_empty()
                || entry.checks.iter().any(|c| c["status"] != "passed")
            {
                return Err(format!(
                    "{}: invalid closure chain or validation evidence",
                    entry.id
                ));
            }
            if let Some(version) = versions.get(&entry.id) {
                if version != &entry.before {
                    return Err(format!("{}: discontinuous closure versions", entry.id));
                }
            }
            if let (Some(before), Some(after)) = (&entry.before, &entry.after) {
                let before = before
                    .parse::<pep440_rs::Version>()
                    .map_err(|e| e.to_string())?;
                let after = after
                    .parse::<pep440_rs::Version>()
                    .map_err(|e| e.to_string())?;
                if after < before || (after == before && entry.level != 0) {
                    return Err(format!(
                        "{}: closure version must increase for actual changes",
                        entry.id
                    ));
                }
            }
            if entry.compatibility == "breaking"
                && entry.acceptance.as_ref().is_none_or(|a| {
                    a["source"].as_str().is_none_or(|s| s.trim().is_empty())
                        || a["reason"].as_str().is_none_or(|s| s.trim().is_empty())
                        || !a["scope"]
                            .as_array()
                            .is_some_and(|s| s.iter().any(|id| id == &entry.id))
                })
            {
                return Err(format!("{}: breaking closure acceptance missing", entry.id));
            }
            versions.insert(entry.id.clone(), entry.after.clone());
            previous = entry.hash.clone();
        }
        Ok(ledger)
    }
    pub(super) fn load(root: &Path) -> Result<Self, String> {
        Self::parse(config::read_current(root, RECORD)?.as_deref())
    }
    pub(super) fn latest(&self, id: &str) -> Option<&Entry> {
        self.entries.iter().rev().find(|e| e.id == id)
    }
    fn append(&mut self, mut entry: Entry) -> Result<(), String> {
        entry.previous = self
            .entries
            .last()
            .map(|e| e.hash.clone())
            .unwrap_or_default();
        entry.hash = hash(&entry)?;
        self.entries.push(entry);
        Ok(())
    }
    pub(super) fn extends(&self, old: &Self) -> Result<(), String> {
        if self.entries.len() < old.entries.len()
            || self
                .entries
                .iter()
                .zip(&old.entries)
                .any(|(a, b)| a.hash != b.hash)
        {
            return Err("committed package closure history must be preserved verbatim".into());
        }
        Ok(())
    }
}
fn hash(value: &impl Serialize) -> Result<String, String> {
    Ok(digest(
        &serde_json::to_vec(value).map_err(|e| e.to_string())?,
    ))
}
fn uses_build(cfg: &Config, capture: &Capture, id: &str, path: &str) -> bool {
    if cfg.build_inputs.iter().any(|p| polyglot::owns(p, path)) {
        return true;
    }
    let Some(package) = capture.current.packages.get(id) else {
        return false;
    };
    if package.language == Language::Rust {
        return Path::new(&package.workspace)
            .with_file_name("Cargo.lock")
            .to_string_lossy()
            .replace('\\', "/")
            == path;
    }
    cfg.native.adapters.python.as_ref().is_some_and(|a| {
        a.locks.iter().any(|l| {
            l.path == path && (l.members.is_empty() || l.members.contains(&package.manifest))
        })
    })
}
fn is_build(cfg: &Config, path: &str) -> bool {
    Path::new(path).file_name().and_then(|s| s.to_str()) == Some("Cargo.lock")
        || cfg.build_inputs.iter().any(|p| polyglot::owns(p, path))
        || cfg
            .native
            .adapters
            .python
            .as_ref()
            .is_some_and(|a| a.locks.iter().any(|l| l.path == path))
}
pub(super) fn version_recorded(root: &Path, id: &str, version: &str) -> Result<bool, String> {
    Ok(Ledger::load(root)?
        .latest(id)
        .is_some_and(|e| e.after.as_deref() == Some(version)))
}

// Content determines whether to bump; environment/build/dependency evidence determines
// whether to revalidate. Unrelated package changes do not force another version bump.
fn fingerprints(
    root: &Path,
    cfg: &Config,
    capture: &mut Capture,
) -> Result<BTreeMap<String, (String, String)>, String> {
    let ids = capture
        .current
        .packages
        .keys()
        .chain(capture.baseline.packages.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    if ids.is_empty() {
        return Ok(BTreeMap::new());
    }
    let mut files = BTreeMap::<String, Option<String>>::new();
    for path in &capture.inventory {
        if path == RECORD
            || cfg.record_inputs.contains(path)
            || matches!(path.as_str(), "VERSION" | "CHANGELOG.md")
        {
            continue;
        }
        let bytes = config::read_current(root, path)?;
        files.insert(path.clone(), bytes.as_deref().map(digest));
        capture.inputs.entry(root.join(path)).or_insert(bytes);
    }
    // Include declared native inputs outside Git inventory, notably ignored native locks.
    for (path, bytes) in &capture.current.inputs {
        files.insert(path.clone(), bytes.as_deref().map(digest));
        capture
            .inputs
            .entry(root.join(path))
            .or_insert(bytes.clone());
    }
    let mut contents = BTreeMap::new();
    for id in &ids {
        let mut owned = BTreeMap::new();
        for (path, digest) in &files {
            if digest.is_none() || is_build(cfg, path) {
                continue;
            }
            let owners = polyglot::owner(&capture.current, path, &cfg.native).unwrap_or_default();
            // The content identity uses current ownership. Historical ownership
            // belongs to impact detection, and must not change this hash when a
            // newly extracted package becomes part of the next release baseline.
            if owners.contains(id) {
                owned.insert(path, json!({"hash":digest,"mode":capture.modes.get(path).map(String::as_str).unwrap_or("100644")}));
            }
        }
        let identity = capture.current.packages.get(id);
        contents.insert(
            id.clone(),
            hash(&json!({"package":identity,"files":owned}))?,
        );
    }
    let build = files
        .iter()
        .filter(|(p, _)| {
            Path::new(p).file_name().and_then(|s| s.to_str()) == Some("Cargo.lock")
                || cfg.build_inputs.iter().any(|r| polyglot::owns(r, p))
                || cfg
                    .native
                    .adapters
                    .python
                    .as_ref()
                    .is_some_and(|a| a.locks.iter().any(|l| &l.path == *p))
        })
        .collect::<BTreeMap<_, _>>();
    let mut result = BTreeMap::new();
    for id in ids {
        let mut dependencies = BTreeSet::from([id.clone()]);
        loop {
            let before = dependencies.len();
            for edge in &capture.current.dependencies {
                if dependencies.contains(&edge.owner) {
                    dependencies.insert(edge.target.clone());
                }
            }
            if before == dependencies.len() {
                break;
            }
        }
        let dependency_inputs = dependencies
            .iter()
            .filter_map(|id| contents.get(id).map(|v| (id, v)))
            .collect::<BTreeMap<_, _>>();
        let mut contracts = BTreeMap::new();
        let mut check_inputs = BTreeMap::new();
        if let Some(policy) = cfg.development.get(&id) {
            for check in policy
                .checks
                .iter()
                .chain(policy.stability.iter().flat_map(|s| &s.checks))
            {
                for path in &check.inputs {
                    let bytes = config::read_current(root, path)?;
                    check_inputs.insert(path, bytes.as_deref().map(digest));
                    capture.inputs.entry(root.join(path)).or_insert(bytes);
                }
            }
        }
        if let Some(stability) = cfg.development.get(&id).and_then(|p| p.stability.as_ref()) {
            for path in &stability.contracts {
                let bytes = config::read_current(root, path)?;
                contracts.insert(path, bytes.as_deref().map(digest));
                capture.inputs.entry(root.join(path)).or_insert(bytes);
            }
        }
        let scoped_build = build
            .iter()
            .filter(|(path, _)| {
                dependencies
                    .iter()
                    .any(|id| uses_build(cfg, capture, id, path))
            })
            .collect::<BTreeMap<_, _>>();
        let evidence = hash(
            &json!({"content":contents[&id],"dependencies":dependency_inputs,
            "build":scoped_build,"environment":capture.environment,"policy":cfg.development.get(&id),
            "contracts":contracts,"check_inputs":check_inputs,"adapters":format!("{:?}",cfg.native.adapters)}),
        )?;
        result.insert(id.clone(), (contents[&id].clone(), evidence));
    }
    Ok(result)
}

fn pending(
    cfg: &Config,
    capture: &Capture,
    ledger: &Ledger,
    hashes: &BTreeMap<String, (String, String)>,
) -> BTreeSet<String> {
    let affected = plan::affected(cfg, capture);
    hashes
        .iter()
        .filter(|(id, (content, evidence))| match ledger.latest(id) {
            Some(entry) => &entry.content != content || &entry.evidence != evidence,
            None => affected.contains(*id),
        })
        .map(|(id, _)| id.clone())
        .collect()
}

pub(super) fn release_gate(
    root: &Path,
    cfg: &Config,
    capture: &mut Capture,
) -> Result<Ledger, String> {
    if root.join(JOURNAL).exists() {
        return Err(
            "unfinished development transaction; run develop --recover before release".into(),
        );
    }
    let ledger = Ledger::load(root)?;
    let hashes = fingerprints(root, cfg, capture)?;
    let pending = pending(cfg, capture, &ledger, &hashes);
    if !pending.is_empty() {
        return Err(format!(
            "package development closure required: {}; complete validation and managed version closure before release (release does not run develop)",
            pending.into_iter().collect::<Vec<_>>().join(", ")
        ));
    }
    for id in plan::affected(cfg, capture) {
        if let Some(entry) = ledger.latest(&id) {
            for (path, expected) in &entry.artifacts {
                let bytes = config::read_current(root, path)?.ok_or_else(|| {
                    format!("{id}: development artifact missing: {path}; run develop")
                })?;
                if digest(&bytes) != *expected {
                    return Err(format!(
                        "{id}: development artifact changed: {path}; run develop"
                    ));
                }
                capture.inputs.insert(root.join(path), Some(bytes));
            }
        }
    }
    Ok(ledger)
}

struct Prepared {
    cfg: Config,
    capture: Capture,
    ledger: Ledger,
    analysis: analysis::Analysis,
    affected: BTreeSet<String>,
    versions: BTreeMap<String, String>,
    fallback: BTreeMap<String, String>,
    stability: BTreeMap<String, String>,
    levels: BTreeMap<String, u8>,
    direct_changes: BTreeSet<String>,
    writes: model::Writes,
    receipt: Value,
}
fn unstable(package: &NativePackage) -> bool {
    unstable_version(package, &package.version)
}
fn unstable_version(package: &NativePackage, version: &str) -> bool {
    match package.language {
        Language::Rust => version.parse::<SemVer>().is_ok_and(|v| v.major == 0),
        Language::Python => version
            .parse::<pep440_rs::Version>()
            .is_ok_and(|v| v.release().first() == Some(&0)),
    }
}

fn requirement_owners(
    root: &Path,
    cfg: &Config,
    capture: &Capture,
    writes: &model::Writes,
) -> Result<BTreeSet<String>, String> {
    let mut owners = BTreeSet::new();
    if let Some(rust) = &cfg.native.adapters.rust {
        owners.extend(rust::requirement_changes(
            rust,
            &capture.inventory,
            &mut |p| config::read_current(root, p),
            writes,
        )?);
    }
    for package in capture
        .current
        .packages
        .values()
        .filter(|p| p.language == Language::Python)
    {
        if let Some(after) = writes.get(&package.manifest) {
            let before = config::read_current(root, &package.manifest)?
                .ok_or("Python manifest disappeared")?;
            let before = super::super::python_metadata::parse(&before, &package.manifest)?;
            let after = super::super::python_metadata::parse(after, &package.manifest)?;
            if before
                .dependencies
                .iter()
                .map(|d| &d.requirement)
                .collect::<Vec<_>>()
                != after
                    .dependencies
                    .iter()
                    .map(|d| &d.requirement)
                    .collect::<Vec<_>>()
            {
                owners.insert(package.manifest.clone());
            }
        }
    }
    Ok(owners)
}
fn stability_gap(
    root: &Path,
    cfg: &Config,
    ledger: &Ledger,
    capture: &Capture,
    id: &str,
    analysis: &analysis::ObjectAnalysis,
) -> Result<Option<String>, String> {
    if analysis.compatibility != "compatible" {
        return Ok(Some("current change is incompatible".into()));
    }
    if ledger.latest(id).is_none() && !capture.baseline.packages.contains_key(id) {
        return Ok(Some(
            "no previous effective development or published baseline for compatibility comparison"
                .into(),
        ));
    }
    let Some(policy) = cfg.development.get(id).and_then(|p| p.stability.as_ref()) else {
        return Ok(Some("stability contract is not configured".into()));
    };
    if policy.contracts.is_empty()
        || policy.checks.is_empty()
        || !policy.checks.iter().any(|c| c.kind == "core")
        || !policy.checks.iter().any(|c| c.kind == "integration")
    {
        return Ok(Some(
            "stability requires contract baselines plus core and real-caller integration checks"
                .into(),
        ));
    }
    for path in &policy.contracts {
        if config::read_current(root, path)?.is_none_or(|b| b.is_empty()) {
            return Ok(Some(format!("contract baseline missing or empty: {path}")));
        }
    }
    Ok(None)
}

fn prepare(
    root: &Path,
    request: &Request,
    runner: &dyn ProcessRunner,
) -> Result<Result<Prepared, Value>, String> {
    require_ignored_runtime(root, runner)?;
    if root.join(JOURNAL).exists() {
        return Err("unfinished development transaction; run develop --recover".into());
    }
    let cfg = load(root)?.ok_or("develop requires schema 4; migrate configuration first")?;
    let mut capture = capture(root, &cfg, runner)?;
    let ledger = Ledger::load(root)?;
    let hashes = fingerprints(root, &cfg, &mut capture)?;
    // Declared ignored inputs are discovered above and must bind the preview too.
    capture.fingerprint = hash(&json!({"capture":capture.fingerprint,"packages":hashes,
        "inputs":capture.inputs.iter().map(|(p,b)|(p.to_string_lossy().into_owned(),b.as_deref().map(digest))).collect::<BTreeMap<_,_>>()}))?;
    let mut affected = pending(&cfg, &capture, &ledger, &hashes);
    // Lost or modified generated artifacts require revalidation, never a second bump.
    for (id, _) in &hashes {
        if let Some(entry) = ledger.latest(id) {
            if entry.artifacts.iter().any(|(p, h)| {
                config::read_current(root, p)
                    .ok()
                    .flatten()
                    .as_deref()
                    .map(digest)
                    .as_ref()
                    != Some(h)
            }) {
                affected.insert(id.clone());
            }
        }
    }
    let mut receipt = json!({"schema":1,"status":"development-analysis-required","fingerprint":capture.fingerprint,
        "affected_packages":affected,"packages":capture.current.packages,"writes":[],"product_version":capture.product_version});
    if affected.is_empty() {
        receipt["status"] = json!("nothing-to-close");
        return Ok(Err(receipt));
    }
    receipt["python_environment_required"] = json!(
        capture.environment_guard.is_none()
            && affected.iter().any(|id| capture
                .current
                .packages
                .get(id)
                .or_else(|| capture.baseline.packages.get(id))
                .is_some_and(|p| p.language == Language::Python))
    );
    let Some(analysis) = analysis::read(root, request, &mut capture)? else {
        return Ok(Err(receipt));
    };
    if receipt["python_environment_required"] == true {
        return Err("Python development checks require explicit python_environment interpreter/runtime/dependency roots".into());
    }
    if analysis.product.is_some() || analysis.targets.contains_key("product") {
        return Err("develop cannot analyze or upgrade product VERSION".into());
    }
    let mut direct_changes = affected
        .iter()
        .filter(|id| ledger.latest(id).is_none_or(|e| e.content != hashes[*id].0))
        .cloned()
        .collect::<BTreeSet<_>>();
    plan::interface_impacts(&capture, &analysis, &mut direct_changes, true)?;
    affected.extend(direct_changes.iter().cloned());
    let mut force_bump = analysis
        .interface_impacts
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut mechanical = BTreeSet::new();
    let mut versions = BTreeMap::new();
    let mut fallback = BTreeMap::new();
    let mut stability = BTreeMap::new();
    let mut levels = BTreeMap::new();
    let mut writes = BTreeMap::new();
    for round in 0..=capture.current.packages.len() + 1 {
        for id in &affected {
            let a = analysis.packages.get(id).ok_or_else(|| {
                format!(
                    "{id}: development analysis missing (including effective dependency changes)"
                )
            })?;
            let policy = cfg
                .development
                .get(id)
                .ok_or_else(|| format!("{id}: mandatory development checks must be configured"))?;
            policy.validate(id)?;
            let package = capture
                .current
                .packages
                .get(id)
                .or_else(|| capture.baseline.packages.get(id))
                .ok_or("unknown development object")?;
            let impact = a.version_level(id, &capture.fingerprint, unstable(package))?;
            if cfg.native.lifecycle.iter().any(|t| {
                t.base == capture.boundary
                    && t.compatibility == "breaking"
                    && t.from.iter().chain(&t.to).any(|p| &p.id == id)
            }) && a.compatibility != "breaking"
            {
                return Err(format!(
                    "{id}: breaking lifecycle transition requires breaking analysis and explicit user acceptance"
                ));
            }
            levels.insert(id.clone(), impact);
            if !capture.current.packages.contains_key(id) {
                continue;
            }
            let revalidation = !force_bump.contains(id)
                && (mechanical.contains(id)
                    || ledger.latest(id).is_some_and(|e| e.content == hashes[id].0));
            if revalidation {
                levels.insert(id.clone(), 0);
            }
            let first = !capture.baseline.packages.contains_key(id) && ledger.latest(id).is_none();
            if unstable(package)
                && analysis
                    .targets
                    .get(id)
                    .is_some_and(|v| !unstable_version(package, v))
            {
                return Err(format!(
                    "{id}: stable graduation is automatic; explicit stable targets are not accepted"
                ));
            }
            let target = if revalidation {
                package.version.clone()
            } else {
                plan::choose(package, impact, analysis.targets.get(id), first)?
            };
            if revalidation && analysis.targets.contains_key(id) {
                return Err(format!("{id}: unchanged content cannot be upgraded"));
            }
            fallback.insert(package.manifest.clone(), target.clone());
            let gap = stability_gap(root, &cfg, &ledger, &capture, id, a)?;
            let promote = unstable(package) && !revalidation && gap.is_none();
            if unstable(package) && !promote && !unstable_version(package, &target) {
                return Err(format!(
                    "{id}: entry to 1.0.0 is automatic and requires the stability contract"
                ));
            }
            versions.insert(
                package.manifest.clone(),
                if promote { "1.0.0".into() } else { target },
            );
            stability.insert(
                id.clone(),
                if promote {
                    "eligible; final-version stability checks required".into()
                } else {
                    gap.unwrap_or_else(|| "already stable or no version change".into())
                },
            );
        }
        writes = plan::render(root, &cfg, &capture, &versions)?;
        let owners = requirement_owners(root, &cfg, &capture, &writes)?;
        let before = (affected.len(), force_bump.len());
        for manifest in owners {
            let id = capture
                .current
                .packages
                .iter()
                .find(|(_, p)| p.manifest == manifest)
                .map(|(id, _)| id)
                .ok_or("dependency owner is unregistered")?;
            force_bump.insert(id.clone());
            affected.insert(id.clone());
        }
        // Native adapters may materialize an unchanged inherited version. Verify
        // and record that final manifest without inventing a business bump.
        for (id, package) in &capture.current.packages {
            if writes.contains_key(&package.manifest) && !affected.contains(id) {
                mechanical.insert(id.clone());
                affected.insert(id.clone());
            }
        }
        // Shared lock changes invalidate existing checks, but not package content.
        if writes.keys().any(|path| {
            Path::new(path).file_name().and_then(|s| s.to_str()) == Some("Cargo.lock")
                || cfg
                    .native
                    .adapters
                    .python
                    .as_ref()
                    .is_some_and(|a| a.locks.iter().any(|l| &l.path == path))
        }) {
            affected.extend(
                ledger
                    .entries
                    .iter()
                    .filter(|e| {
                        writes
                            .keys()
                            .any(|path| uses_build(&cfg, &capture, &e.id, path))
                    })
                    .map(|e| e.id.clone()),
            );
        }
        if (affected.len(), force_bump.len()) == before {
            break;
        }
        if round == capture.current.packages.len() + 1 {
            return Err("development dependency closure did not converge".into());
        }
    }
    if analysis
        .packages
        .keys()
        .chain(analysis.targets.keys())
        .any(|id| !affected.contains(id))
    {
        return Err("development analysis includes unchanged or unknown packages".into());
    }
    for path in writes.keys() {
        capture
            .inputs
            .entry(root.join(path))
            .or_insert(config::read_current(root, path)?);
    }
    verify_release_inputs(&capture.inputs)?;
    receipt["status"] = json!("development-planned");
    receipt["affected_packages"] = json!(affected);
    receipt["versions"] = json!(versions);
    receipt["stability"] = json!(stability);
    receipt["writes"] = json!(
        writes
            .keys()
            .chain(std::iter::once(&RECORD.to_string()))
            .collect::<Vec<_>>()
    );
    Ok(Ok(Prepared {
        cfg,
        capture,
        ledger,
        analysis,
        affected,
        versions,
        fallback,
        stability,
        levels,
        direct_changes,
        writes,
        receipt,
    }))
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Journal {
    root: String,
    head: String,
    before: model::Inputs,
    after: model::Writes,
}
fn atomic(root: &Path, path: &str, bytes: &[u8]) -> Result<(), String> {
    let path = safe_relative(root, path)?;
    crate::persistence::atomic_write(&path, bytes).map_err(|e| e.to_string())
}
fn rollback(root: &Path, journal: &Journal) -> Result<(), String> {
    let mut conflicts = Vec::new();
    for (path, after) in &journal.after {
        let before = journal
            .before
            .get(path)
            .ok_or("invalid development journal")?;
        let current = config::read_current(root, path)?;
        if &current == before {
            continue;
        }
        if current.as_ref() != Some(after) {
            conflicts.push(path.clone());
            continue;
        }
        match before {
            Some(bytes) => atomic(root, path, bytes)?,
            None => {
                fs::remove_file(safe_relative(root, path)?).map_err(|e| e.to_string())?;
            }
        }
    }
    if !conflicts.is_empty() {
        return Err(format!(
            "rollback preserved external edits; resolve journal conflicts: {}",
            conflicts.join(", ")
        ));
    }
    fs::remove_file(root.join(JOURNAL)).map_err(|e| e.to_string())
}
fn save_journal(root: &Path, journal: &Journal) -> Result<(), String> {
    atomic(
        root,
        JOURNAL,
        &serde_json::to_vec_pretty(journal).map_err(|e| e.to_string())?,
    )
}
pub fn recover(root: &Path) -> CommandOutcome {
    let result: Result<Value, String> = (|| {
        let runner = crate::SystemProcessRunner;
        require_ignored_runtime(root, &runner)?;
        let _sync_lock = crate::git_sync::lock_factory_runtime(root, &runner)?;
        let _lock = crate::project_sync::ProjectLock::acquire(root)?;
        let bytes =
            config::read_current(root, JOURNAL)?.ok_or("no unfinished development transaction")?;
        let journal: Journal = serde_json::from_value(crate::json::parse_unique_json(
            &bytes,
            "development journal",
        )?)
        .map_err(|e| e.to_string())?;
        if journal.root
            != root
                .canonicalize()
                .map_err(|e| e.to_string())?
                .to_string_lossy()
            || journal.head
                != history::git(root, &InspectionRunner(&runner), &["rev-parse", "HEAD"])?.trim()
        {
            return Err("development journal belongs to another checkout or HEAD; preserve it for manual diagnosis".into());
        }
        for path in journal.before.keys().chain(journal.after.keys()) {
            safe_relative(root, path)?;
            if matches!(path.as_str(), "VERSION" | "CHANGELOG.md" | CONFIG)
                || path.starts_with(".git/")
            {
                return Err("invalid development journal write boundary".into());
            }
        }
        if journal
            .after
            .get(RECORD)
            .is_some_and(|b| config::read_current(root, RECORD).ok().flatten().as_ref() == Some(b))
        {
            fs::remove_file(root.join(JOURNAL)).map_err(|e| e.to_string())?;
        } else {
            rollback(root, &journal)?;
        }
        Ok(json!({"status":"development-recovered","git_written":false}))
    })();
    outcome(result)
}

fn run_check(
    root: &Path,
    check: &CheckCommand,
    runner: &dyn ProcessRunner,
) -> Result<(Value, BTreeMap<String, String>), String> {
    for path in &check.inputs {
        if config::read_current(root, path)?.is_none() {
            return Err(format!(
                "{}: required validation input missing: {path}",
                check.id
            ));
        }
    }
    let mut command = ProcessRequest::new(&check.program, root);
    command.args = check.args.iter().map(Into::into).collect();
    command.timeout = Duration::from_secs(check.timeout_seconds);
    let output = runner
        .run(&command)
        .map_err(|e| format!("{}: cannot run check: {e}", check.id))?;
    if output.code != 0 || output.timed_out {
        return Err(format!(
            "{}: development check failed (exit {}, timeout {}): {}",
            check.id,
            output.code,
            output.timed_out,
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let mut artifacts = BTreeMap::new();
    for path in &check.artifacts {
        let bytes = config::read_current(root, path)?
            .ok_or_else(|| format!("{}: required final artifact missing: {path}", check.id))?;
        artifacts.insert(path.clone(), digest(&bytes));
    }
    Ok((
        json!({"id":check.id,"status":"passed","command":check,"stdout_sha256":digest(&output.stdout),"stderr_sha256":digest(&output.stderr)}),
        artifacts,
    ))
}

fn execute(root: &Path, mut p: Prepared, runner: &dyn ProcessRunner) -> Result<Value, String> {
    let mut journal = Journal {
        root: root
            .canonicalize()
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .into(),
        head: p.capture.head.clone(),
        before: BTreeMap::new(),
        after: p.writes.clone(),
    };
    for path in journal.after.keys() {
        journal
            .before
            .insert(path.clone(), config::read_current(root, path)?);
    }
    verify_release_inputs(&p.capture.inputs)?;
    save_journal(root, &journal)?;
    let result: Result<Value, String> = (|| {
        let mut proofs = BTreeMap::<String, Vec<Value>>::new();
        let mut artifacts = BTreeMap::<String, BTreeMap<String, String>>::new();
        // Each failed stability contract downgrades only that package. Re-render and
        // rerun checks for the final dependency graph; successful packages may graduate.
        for round in 0..=p.affected.len() {
            for (path, bytes) in &p.writes {
                atomic(root, path, bytes)?;
            }
            proofs.clear();
            artifacts.clear();
            let mut failures = BTreeMap::new();
            for id in &p.affected {
                let policy = &p.cfg.development[id];
                for check in &policy.checks {
                    let (proof, files) = run_check(root, check, runner)?;
                    proofs.entry(id.clone()).or_default().push(proof);
                    artifacts.entry(id.clone()).or_default().extend(files);
                }
                if let Some(package) = p.capture.current.packages.get(id) {
                    if p.versions.get(&package.manifest) != p.fallback.get(&package.manifest) {
                        for check in &policy
                            .stability
                            .as_ref()
                            .ok_or("stability policy missing")?
                            .checks
                        {
                            match run_check(root, check, runner) {
                                Ok((proof, files)) => {
                                    proofs.entry(id.clone()).or_default().push(proof);
                                    artifacts.entry(id.clone()).or_default().extend(files);
                                }
                                Err(error) => {
                                    failures.insert(id.clone(), error);
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            if failures.is_empty() {
                for id in &p.affected {
                    if let Some(package) = p.capture.current.packages.get(id) {
                        if p.versions.get(&package.manifest) != p.fallback.get(&package.manifest) {
                            p.stability.insert(
                                id.clone(),
                                "stable contract and core/integration checks passed on final 1.0.0"
                                    .into(),
                            );
                        }
                    }
                }
                break;
            }
            if round == p.affected.len() {
                return Err("stability fallback did not converge".into());
            }
            rollback(root, &journal)?;
            for (id, error) in failures {
                let manifest = &p.capture.current.packages[&id].manifest;
                p.versions
                    .insert(manifest.clone(), p.fallback[manifest].clone());
                p.stability
                    .insert(id, format!("remains in development: {error}"));
            }
            p.writes = plan::render(root, &p.cfg, &p.capture, &p.versions)?;
            // A candidate graduation may have required a caller bump. If the final
            // fallback no longer changes that requirement, retain the caller version.
            for _ in 0..=p.affected.len() {
                let owners = requirement_owners(root, &p.cfg, &p.capture, &p.writes)?;
                let mut changed = false;
                for id in &p.affected {
                    if let Some(package) = p.capture.current.packages.get(id) {
                        if !p.direct_changes.contains(id)
                            && !owners.contains(&package.manifest)
                            && p.versions.get(&package.manifest) != Some(&package.version)
                        {
                            p.versions
                                .insert(package.manifest.clone(), package.version.clone());
                            p.fallback
                                .insert(package.manifest.clone(), package.version.clone());
                            p.levels.insert(id.clone(), 0);
                            changed = true;
                        }
                    }
                }
                if !changed {
                    break;
                }
                p.writes = plan::render(root, &p.cfg, &p.capture, &p.versions)?;
            }
            journal = Journal {
                root: journal.root.clone(),
                head: journal.head.clone(),
                before: BTreeMap::new(),
                after: p.writes.clone(),
            };
            for path in journal.after.keys() {
                journal
                    .before
                    .insert(path.clone(), config::read_current(root, path)?);
            }
            save_journal(root, &journal)?;
        }
        let mut expected = p.capture.inputs.clone();
        for (path, bytes) in &p.writes {
            expected.insert(root.join(path), Some(bytes.clone()));
        }
        for files in artifacts.values() {
            for (path, hash) in files {
                if p.capture.inventory.contains(path)
                    && !p.cfg.build_inputs.iter().any(|p| polyglot::owns(p, path))
                {
                    return Err(format!(
                        "tracked artifact must be declared as a build input: {path}"
                    ));
                }
                let bytes = config::read_current(root, path)?
                    .ok_or_else(|| format!("final artifact disappeared: {path}"))?;
                if digest(&bytes) != *hash {
                    return Err(format!("final artifact changed after its check: {path}"));
                }
                expected.insert(root.join(path), Some(bytes));
            }
        }
        verify_release_inputs(&expected)?;
        if history::git(root, &InspectionRunner(runner), &["rev-parse", "HEAD"])?.trim()
            != p.capture.head
        {
            return Err("HEAD changed during development validation".into());
        }
        if inputs::modes(root, &InspectionRunner(runner))? != p.capture.modes {
            return Err("file modes changed during development validation".into());
        }
        if let Some(environment) = &p.capture.environment_guard {
            environment.verify(root)?;
        }
        if inputs::environment(root, &p.cfg, p.capture.environment_guard.as_ref(), runner)?
            != p.capture.environment
        {
            return Err("validation environment changed during development checks".into());
        }
        let inventory = independent::inventory(root, &InspectionRunner(runner))?;
        if inventory != p.capture.inventory {
            return Err("project file inventory changed during development checks".into());
        }
        p.capture.current = polyglot::snapshot(
            &p.cfg.native,
            p.capture.inventory.clone(),
            false,
            &mut |path| config::read_current(root, path),
        )?;
        p.capture.inputs = expected;
        let hashes = fingerprints(root, &p.cfg, &mut p.capture)?;
        let mut changes = Vec::new();
        for files in artifacts.values() {
            for (path, expected) in files {
                let bytes = config::read_current(root, path)?
                    .ok_or_else(|| format!("final artifact disappeared: {path}"))?;
                if digest(&bytes) != *expected {
                    return Err(format!("final artifact changed after its check: {path}"));
                }
                p.capture.inputs.insert(root.join(path), Some(bytes));
            }
        }
        for id in &p.affected {
            let before = p
                .ledger
                .latest(id)
                .and_then(|e| e.after.clone())
                .or_else(|| {
                    p.capture
                        .baseline
                        .packages
                        .get(id)
                        .map(|p| p.version.clone())
                });
            let after = p
                .capture
                .current
                .packages
                .get(id)
                .map(|p| p.version.clone());
            let a = &p.analysis.packages[id];
            let (content, evidence) = hashes.get(id).ok_or("missing package fingerprint")?;
            let stability = p
                .stability
                .get(id)
                .cloned()
                .unwrap_or_else(|| "retired".into());
            changes.push(json!({"id":id,"version_before":before,"version_after":after,"reason":a.reason,"stability":stability}));
            p.ledger.append(Entry{id:id.clone(),before,after,content:content.clone(),evidence:evidence.clone(),
                level:p.levels[id],compatibility:a.compatibility.clone(),acceptance:a.acceptance.as_ref().map(|v|json!({"fingerprint":v.fingerprint,"scope":v.scope,"reason":v.reason,"source":v.source})),reason:a.reason.clone(),stability,
                checks:proofs.remove(id).unwrap_or_default(),artifacts:artifacts.remove(id).unwrap_or_default(),previous:String::new(),hash:String::new()})?;
        }
        let bytes = serde_json::to_vec_pretty(&p.ledger).map_err(|e| e.to_string())?;
        Ledger::parse(Some(&bytes))?;
        journal
            .before
            .insert(RECORD.into(), config::read_current(root, RECORD)?);
        journal.after.insert(RECORD.into(), bytes.clone());
        save_journal(root, &journal)?;
        verify_release_inputs(&p.capture.inputs)?;
        atomic(root, RECORD, &bytes)?;
        fs::remove_file(root.join(JOURNAL)).map_err(|e| e.to_string())?;
        Ok(
            json!({"schema":1,"status":"development-closed","product_version":p.capture.product_version,"package_changes":changes,"git_written":false}),
        )
    })();
    match result {
        Ok(value) => Ok(value),
        Err(error) => match rollback(root, &journal) {
            Ok(()) => Err(format!("{error}; native version writes rolled back")),
            Err(recovery) => Err(format!("{error}; {recovery}")),
        },
    }
}
fn outcome(result: Result<Value, String>) -> CommandOutcome {
    match result {
        Ok(value) => CommandOutcome::with_receipt(value),
        Err(error) => CommandOutcome::blocked(format!("[package-development] {error}\n")),
    }
}
pub fn preview(root: &Path, request: &Request, runner: &dyn ProcessRunner) -> CommandOutcome {
    outcome(prepare(root, request, runner).map(|p| match p {
        Ok(p) => p.receipt,
        Err(receipt) => receipt,
    }))
}
pub fn apply(
    root: &Path,
    request: &Request,
    expected: &str,
    runner: &dyn ProcessRunner,
) -> CommandOutcome {
    outcome((|| {
        require_ignored_runtime(root, runner)?;
        let _sync_lock = crate::git_sync::lock_factory_runtime(root, runner)?;
        let _lock = crate::project_sync::ProjectLock::acquire(root)?;
        match prepare(root, request, runner)? {
            Err(receipt) if receipt["status"] == "nothing-to-close" => Ok(receipt),
            Err(_) => Err("development analysis is required".into()),
            Ok(p) => {
                if expected.is_empty() || expected != p.capture.fingerprint {
                    return Err("development plan changed; preview again before applying".into());
                }
                execute(root, p, runner)
            }
        }
    })())
}
