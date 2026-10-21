//! Legacy configuration is read here to create one immutable, language-neutral starting point.
use super::*;
use crate::CommandOutcome;

struct Prepared {
    value: Value,
    fingerprint: String,
    inputs: BTreeMap<PathBuf, Option<Vec<u8>>>,
    inventory: Vec<String>,
    status: String,
    head: String,
}

/// During the handover window the new baseline has not yet become a formal
/// schema-4 release. Rebuild its native facts from immutable Git, not from an
/// editable deserialized version field. This is migration integrity checking;
/// it never invokes a legacy severity/propagation algorithm.
pub(super) fn verify_baseline(
    root: &Path,
    cfg: &Config,
    boundary: &str,
    runner: &dyn ProcessRunner,
) -> Result<(), String> {
    if configuration::configuration_hash(&cfg.baseline.initial_configuration)?
        != cfg.baseline.initial_configuration_hash
    {
        return Err("migration initial configuration proof changed".into());
    }
    if boundary != cfg.baseline.revision {
        let bytes = revision_payload(root, boundary, CONFIG, runner)?
            .ok_or("published schema 4 baseline missing")?;
        let published = parse(&bytes)?.ok_or("published release must use schema 4")?;
        if serde_json::to_value(&published.baseline).map_err(|e| e.to_string())?
            != serde_json::to_value(&cfg.baseline).map_err(|e| e.to_string())?
        {
            return Err("published migration baseline cannot be changed".into());
        }
        return Ok(());
    }
    let mut seed = cfg.baseline.initial_configuration.clone();
    seed["baseline"] = serde_json::to_value(&cfg.baseline).map_err(|e| e.to_string())?;
    let seed = parse(&serde_json::to_vec(&seed).map_err(|e| e.to_string())?)?
        .ok_or("migration proof must describe schema 4")?;
    let baseline = polyglot::historical(root, &cfg.baseline.revision, &seed.native, runner)?;
    let scope = baseline.config.as_ref().unwrap_or(&seed.native);
    let mut hashes = baseline
        .inputs
        .iter()
        .map(|(p, b)| (p.clone(), b.as_deref().map(digest)))
        .collect::<BTreeMap<_, _>>();
    for path in ["VERSION", "CHANGELOG.md", CONFIG] {
        hashes.insert(
            path.into(),
            revision_payload(root, &cfg.baseline.revision, path, runner)?
                .as_deref()
                .map(digest),
        );
    }
    let mut retired = BTreeSet::new();
    for t in &scope.lifecycle {
        for p in &t.from {
            if !t.to.iter().any(|n| n.id == p.id) {
                retired.insert(p.id.clone());
            }
        }
    }
    let mut lineage = Vec::new();
    for revision in history::git(
        root,
        runner,
        &[
            "rev-list",
            "--reverse",
            "--first-parent",
            &format!("{}..{}", cfg.baseline.revision, cfg.baseline.migration_head),
        ],
    )?
    .lines()
    .filter(|s| !s.is_empty())
    {
        lineage.push(configuration::LineageState {
            revision: revision.into(),
            packages: polyglot::historical(root, revision, &seed.native, runner)?.packages,
        });
    }
    let expected = json!({"packages":baseline.packages,"dependencies":baseline.dependencies,"retired_ids":retired,"input_hashes":hashes,"product_inputs":scope.product_inputs,"shared_inputs":scope.shared_inputs,"lifecycle":scope.lifecycle,"lineage":lineage});
    let actual = json!({"packages":cfg.baseline.packages,"dependencies":cfg.baseline.dependencies,"retired_ids":cfg.baseline.retired_ids,"input_hashes":cfg.baseline.input_hashes,"product_inputs":cfg.baseline.product_inputs,"shared_inputs":cfg.baseline.shared_inputs,"lifecycle":cfg.baseline.lifecycle,"lineage":cfg.baseline.lineage});
    if actual != expected {
        return Err("migration baseline facts disagree with immutable native history".into());
    }
    Ok(())
}

fn prepare(
    root: &Path,
    runner: &dyn ProcessRunner,
    proposal: Option<&str>,
    strategy: Option<&str>,
) -> Result<Prepared, String> {
    let inspection = InspectionRunner(runner);
    crate::git_sync::verify_readonly_git(root, &inspection)?;
    require_ignored_runtime(root, &inspection)?;
    let reader = history_cache::Reader::new(root, &inspection);
    let inventory = independent::inventory(root, &reader)?;
    let status = independent::status(root, &reader)?;
    let old = config::read_current(root, CONFIG)?;
    let old_value=old.as_deref().map(|b|crate::json::parse_unique_json(b,"migration source config")).transpose()?.unwrap_or_else(||json!({"schema_version":1,"manifest_policy":"auto","release_policy":"explicit_release"}));
    let source_schema = old_value["schema_version"]
        .as_u64()
        .ok_or("migration source schema is missing")? as u32;
    if source_schema == 4 && proposal.is_none() && strategy.is_none() {
        return Err("project already uses schema 4; use an explicit configuration proposal for reconfiguration".into());
    }
    if !(1..=4).contains(&source_schema) {
        return Err("unsupported migration source schema".into());
    }
    if old_value["target_versions"]
        .as_object()
        .is_some_and(|v| !v.is_empty())
        || old_value["target_levels"]
            .as_object()
            .is_some_and(|v| !v.is_empty())
    {
        return Err("migration cannot discard pending legacy targets; review them and supply per-release targets explicitly".into());
    }
    let (head, boundary) = history::validate_baseline(root, &reader, None, false)?;
    let mut value = if source_schema >= 3 {
        old_value.clone()
    } else {
        json!({"schema_version":3,"release_policy":"explicit_release","strategy":"independent","adapters":{},"packages":[]})
    };
    let mut inputs = BTreeMap::from([(root.join(CONFIG), old.clone())]);
    if source_schema == 2 {
        let rust_cfg = config::parse(old.as_deref().ok_or("schema 2 source is missing")?)?
            .ok_or("schema 2 Rust adapter missing")?;
        let native = rust::snapshot(&rust_cfg, &inventory, &mut |p| {
            config::read_current(root, p)
        })?;
        let mut identities = Vec::new();
        let mut manifest_ids = BTreeMap::new();
        let mut ids = BTreeSet::new();
        for (manifest, p) in &native.packages {
            if !ids.insert(p.name.clone()) {
                return Err("schema 2 has duplicate native names; provide a schema 4 proposal with explicit stable ids".into());
            }
            manifest_ids.insert(manifest.clone(), p.name.clone());
            identities
                .push(json!({"id":p.name,"name":p.name,"manifest":manifest,"language":"rust"}));
        }
        value["packages"] = json!(identities);
        value["adapters"]["rust"] = old_value["rust"].clone();
        value["product_inputs"] = json!(rust_cfg.product_inputs);
        value["shared_inputs"]=json!(rust_cfg.shared_inputs.iter().map(|s|Ok(json!({"path":s.path,"packages":s.packages.iter().map(|m|manifest_ids.get(m).cloned().ok_or_else(||format!("legacy shared input references unknown package: {m}"))).collect::<Result<Vec<_>,_>>()?}))).collect::<Result<Vec<Value>,String>>()?);
    }
    if source_schema == 1 {
        value["strategy"] = json!("synchronized");
        let manifests = if config::auto_manifests(&old_value)? {
            ["Cargo.toml", "package.json"]
                .into_iter()
                .filter(|p| inventory.iter().any(|x| x == p))
                .map(str::to_string)
                .collect::<Vec<_>>()
        } else {
            old_value["manifests"]
                .as_array()
                .ok_or("schema 1 manifests missing")?
                .iter()
                .map(|p| {
                    p.as_str()
                        .map(str::to_string)
                        .ok_or("invalid legacy manifest".into())
                })
                .collect::<Result<Vec<_>, String>>()?
        };
        if manifests.iter().any(|p| !p.ends_with("Cargo.toml")) {
            return Err(
                "non-Rust/Python native mappings require an explicit supported migration proposal"
                    .into(),
            );
        }
        value["synchronized_manifests"] = json!(manifests);
    }
    if let Some(path) = proposal {
        let path = if Path::new(path).is_absolute() {
            PathBuf::from(path)
        } else {
            root.join(path)
        };
        let bytes = release_input(&path)?.ok_or("migration configuration proposal missing")?;
        value = crate::json::parse_unique_json(&bytes, "schema 4 migration proposal")?;
        if value["schema_version"].as_u64() != Some(4) {
            return Err("migration proposal must use schema 4".into());
        }
        if value.get("baseline").is_some() {
            return Err(
                "migration baseline is generated from history, not accepted from a proposal".into(),
            );
        }
        inputs.insert(path, Some(bytes));
    }
    value["schema_version"] = json!(4);
    value["release_policy"] = json!("explicit_release");
    if source_schema != 1 && source_schema != 4 && proposal.is_none() {
        value["strategy"] = json!("independent");
    }
    if let Some(strategy) = strategy {
        if !matches!(strategy, "synchronized" | "independent") {
            return Err("schema 4 supports only synchronized or independent".into());
        }
        value["strategy"] = json!(strategy);
    }
    value["git_sync_setup"] = json!({"schema":1,"confirmed":true,"selection":value["strategy"].as_str().ok_or("migration proposal strategy missing")?});
    // Adapter DTOs are reused; no legacy bump algorithm is invoked during migration.
    let mut adapter_value = value.clone();
    let obj = adapter_value
        .as_object_mut()
        .ok_or("migration config must be an object")?;
    for key in [
        "baseline",
        "build_inputs",
        "record_inputs",
        "required_checks",
        "synchronized_manifests",
        "python_environment",
    ] {
        obj.remove(key);
    }
    obj.insert("schema_version".into(), json!(3));
    obj.insert("strategy".into(), json!("independent"));
    obj.entry("adapters").or_insert(json!({}));
    obj.entry("packages").or_insert(json!([]));
    let native = polyglot::parse(&serde_json::to_vec(&adapter_value).map_err(|e| e.to_string())?)?
        .ok_or("invalid migration adapter configuration")?;
    let current = polyglot::snapshot(&native, inventory.clone(), false, &mut |p| {
        config::read_current(root, p)
    })?;
    let baseline = polyglot::historical(root, &boundary, &native, &reader)?;
    let mut lineage = Vec::new();
    let commits = history::git(
        root,
        &reader,
        &[
            "rev-list",
            "--reverse",
            "--first-parent",
            &format!("{boundary}..{head}"),
        ],
    )?;
    for revision in commits.lines().filter(|s| !s.is_empty()) {
        lineage.push(configuration::LineageState {
            revision: revision.into(),
            packages: polyglot::historical(root, revision, &native, &reader)?.packages,
        });
    }
    let mut retired_ids = BTreeSet::new();
    for transition in baseline
        .config
        .as_ref()
        .map(|c| c.lifecycle.as_slice())
        .unwrap_or(&[])
    {
        for p in &transition.from {
            if !transition.to.iter().any(|n| n.id == p.id) {
                retired_ids.insert(p.id.clone());
            }
        }
    }
    if current.packages.keys().any(|id| retired_ids.contains(id)) {
        return Err("migration would reuse a retired package identity".into());
    }
    let mut input_hashes = baseline
        .inputs
        .iter()
        .map(|(p, b)| (p.clone(), b.as_deref().map(digest)))
        .collect::<BTreeMap<_, _>>();
    for path in ["VERSION", "CHANGELOG.md", CONFIG] {
        input_hashes.insert(
            path.into(),
            revision_payload(root, &boundary, path, &reader)?
                .as_deref()
                .map(digest),
        );
    }
    let baseline_scope = baseline.config.as_ref().unwrap_or(&native);
    value["baseline"] = serde_json::to_value(Baseline {
        revision: boundary.clone(),
        migration_head: head.clone(),
        product_version: fs::read_to_string(root.join("VERSION"))
            .map_err(|e| e.to_string())?
            .trim()
            .into(),
        source_schema,
        initial_configuration_hash: configuration::configuration_hash(&value)?,
        initial_configuration: {
            let mut initial = value.clone();
            initial
                .as_object_mut()
                .ok_or("invalid migration configuration")?
                .remove("baseline");
            initial
        },
        packages: baseline.packages.clone(),
        dependencies: baseline.dependencies.clone(),
        retired_ids,
        input_hashes,
        product_inputs: baseline_scope.product_inputs.clone(),
        shared_inputs: baseline_scope.shared_inputs.clone(),
        lifecycle: baseline_scope.lifecycle.clone(),
        lineage,
    })
    .map_err(|e| e.to_string())?;
    if source_schema == 4 {
        value["baseline"] = old_value["baseline"].clone();
    }
    let cfg = parse(&serde_json::to_vec(&value).map_err(|e| e.to_string())?)?
        .ok_or("migration did not produce schema 4")?;
    super::lineage::validate(root, &cfg, &baseline, &current, &head, &boundary, &reader)?;
    let current_product = fs::read_to_string(root.join("VERSION"))
        .map_err(|e| e.to_string())?
        .trim()
        .to_string();
    if cfg.strategy == "synchronized"
        && current
            .packages
            .values()
            .any(|p| p.version != current_product)
    {
        return Err("synchronized strategy requires registered packages to already match the product version; review an explicit transition before changing strategy".into());
    }
    for (id, p) in &current.packages {
        if baseline
            .packages
            .get(id)
            .is_some_and(|b| b.version != p.version)
        {
            return Err(format!(
                "{id}: migration must not conceal manual package version changes"
            ));
        }
    }
    inputs.extend(
        rust::product_mapping_inputs(
            &cfg.synchronized_manifests,
            &inventory,
            &mut |p| config::read_current(root, p),
            current_product.parse::<SemVer>()?,
        )?
        .into_iter()
        .map(|(p, b)| (root.join(p), b)),
    );
    inputs.extend(current.inputs.into_iter().map(|(p, b)| (root.join(p), b)));
    for path in ["VERSION", "CHANGELOG.md"] {
        inputs.insert(root.join(path), config::read_current(root, path)?);
    }
    verify_release_inputs(&inputs)?;
    reader.verify_unchanged()?;
    let hashes = inputs
        .iter()
        .map(|(p, b)| (p.to_string_lossy().into_owned(), b.as_deref().map(digest)))
        .collect::<BTreeMap<_, _>>();
    let fingerprint=digest(&serde_json::to_vec(&json!({"root":root.canonicalize().map_err(|e|e.to_string())?,"head":head,"history":planning_history_state(root,&reader)?,"status":status,"inventory":inventory,"inputs":hashes,"configuration":value})).map_err(|e|e.to_string())?);
    Ok(Prepared {
        value,
        fingerprint,
        inputs,
        inventory,
        status,
        head,
    })
}

pub fn migration_plan(
    root: &Path,
    runner: &dyn ProcessRunner,
    proposal: Option<&str>,
) -> CommandOutcome {
    configuration_plan(root, runner, proposal, None)
}

pub fn configuration_plan(
    root: &Path,
    runner: &dyn ProcessRunner,
    proposal: Option<&str>,
    strategy: Option<&str>,
) -> CommandOutcome {
    match prepare(root, runner, proposal, strategy) {
        Ok(p) => CommandOutcome::with_receipt(
            json!({"schema":1,"status":"migration-planned","fingerprint":p.fingerprint,"configuration":p.value,"writes":[CONFIG],"version_changes":false}),
        ),
        Err(e) => CommandOutcome::blocked(format!("[schema4-migration] {e}\n")),
    }
}

pub fn migration_apply(
    root: &Path,
    runner: &dyn ProcessRunner,
    proposal: Option<&str>,
    expected: &str,
    confirmed: bool,
) -> CommandOutcome {
    configuration_apply(root, runner, proposal, None, expected, confirmed)
}

pub fn configuration_apply(
    root: &Path,
    runner: &dyn ProcessRunner,
    proposal: Option<&str>,
    strategy: Option<&str>,
    expected: &str,
    confirmed: bool,
) -> CommandOutcome {
    let result = (|| {
        if !confirmed {
            return Err("migration requires a confirmed strategy choice".into());
        }
        let _lock = crate::project_sync::ProjectLock::acquire(root)?;
        let p = prepare(root, runner, proposal, strategy)?;
        if p.fingerprint != expected {
            return Err("migration inputs changed; plan again before applying".into());
        }
        let inspection = InspectionRunner(runner);
        verify_release_inputs(&p.inputs)?;
        verify_release_inventory(root, &inspection, &p.inventory)?;
        if independent::status(root, &inspection)? != p.status
            || history::git(root, &inspection, &["rev-parse", "HEAD"])?.trim() != p.head
        {
            return Err("repository changed before migration apply".into());
        }
        crate::persistence::atomic_write_json(&root.join(CONFIG), &p.value)
            .map_err(|e| e.to_string())?;
        Ok::<_, String>(
            json!({"schema":1,"status":"migrated","configuration":CONFIG,"strategy":p.value["strategy"],"sync_performed":false,"version_changes":false}),
        )
    })();
    match result {
        Ok(v) => CommandOutcome::with_receipt(v),
        Err(e) => CommandOutcome::blocked(format!("[schema4-migration] {e}\n")),
    }
}
