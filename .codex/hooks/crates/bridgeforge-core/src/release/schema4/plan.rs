use super::analysis::Analysis;
use super::*;
use crate::CommandOutcome;

fn product(version: SemVer) -> NativePackage {
    NativePackage {
        manifest: "VERSION".into(),
        name: "product".into(),
        version: version.to_string(),
        language: Language::Rust,
        workspace: String::new(),
        source_roots: vec![],
    }
}

fn render(
    root: &Path,
    cfg: &Config,
    capture: &Capture,
    versions: &BTreeMap<String, String>,
) -> Result<model::Writes, String> {
    let mut writes = BTreeMap::new();
    if let Some(rust_cfg) = &cfg.native.adapters.rust {
        let targets = versions
            .iter()
            .filter(|(path, _)| {
                capture
                    .current
                    .packages
                    .values()
                    .any(|p| p.language == Language::Rust && &p.manifest == *path)
            })
            .map(|(p, v)| (p.clone(), v.clone()))
            .collect();
        let rendered = rust::render_versions(
                rust_cfg,
                &capture.inventory,
                &mut |p| config::read_current(root, p),
                &targets,
            )?;
        polyglot::add_writes(&mut writes, rendered)?;
    }
    if let Some(python_cfg) = &cfg.native.adapters.python {
        let targets = versions
            .iter()
            .filter(|(path, _)| {
                capture
                    .current
                    .packages
                    .values()
                    .any(|p| p.language == Language::Python && &p.manifest == *path)
            })
            .map(|(p, v)| (p.clone(), v.clone()))
            .collect();
        polyglot::add_writes(
            &mut writes,
            python::render_versions(
                python_cfg,
                &capture.inventory,
                &mut |p| config::read_current(root, p),
                &targets,
            )?,
        )?;
    }
    Ok(writes)
}

fn level(analysis: &Analysis, id: &str, fingerprint: &str) -> Result<u8, String> {
    analysis.packages.get(id).ok_or_else(||format!("{id}: package analysis missing (including any effective dependency requirement changes)"))?.level(id,fingerprint)
}

fn choose(
    package: &NativePackage,
    impact: u8,
    target: Option<&String>,
    first: bool,
) -> Result<String, String> {
    let minimum = if first {
        package.version.clone()
    } else {
        match polyglot::bump_native(package, impact) {
            Ok(v) => v,
            Err(_) if package.language == Language::Python && target.is_some() => {
                let target = target.unwrap();
                polyglot::validate_target(package, target)?;
                return Ok(target.clone());
            }
            Err(e) => return Err(e),
        }
    };
    if let Some(target) = target {
        polyglot::validate_target(package, target)?;
        let enough = match package.language {
            Language::Rust => target.parse::<SemVer>()? >= minimum.parse::<SemVer>()?,
            Language::Python => {
                target
                    .parse::<pep440_rs::Version>()
                    .map_err(|e| e.to_string())?
                    >= minimum
                        .parse::<pep440_rs::Version>()
                        .map_err(|e| e.to_string())?
            }
        };
        if !enough {
            return Err(format!(
                "{}: explicit target {target} is below suggested {minimum}",
                package.name
            ));
        }
        Ok(target.clone())
    } else {
        Ok(minimum)
    }
}

pub(super) fn affected(cfg: &Config, capture: &Capture) -> BTreeSet<String> {
    let mut affected = capture
        .changes
        .iter()
        .filter(|c| c.role != "record")
        .flat_map(|c| c.packages.iter().cloned())
        .collect::<BTreeSet<_>>();
    for transition in cfg
        .native
        .lifecycle
        .iter()
        .filter(|t| t.base == capture.boundary)
    {
        let endpoints = transition
            .from
            .iter()
            .chain(&transition.to)
            .map(|p| p.id.clone())
            .collect::<BTreeSet<_>>();
        let net_identity_change = endpoints.iter().any(|id| {
            match (
                capture.baseline.packages.get(id),
                capture.current.packages.get(id),
            ) {
                (Some(before), Some(after)) => {
                    before.manifest != after.manifest
                        || before.name != after.name
                        || before.language != after.language
                }
                (None, None) => false,
                _ => true,
            }
        });
        if net_identity_change {
            affected.extend(endpoints);
        }
    }
    affected
}

fn interface_impacts(
    capture: &Capture,
    analysis: &Analysis,
    affected: &mut BTreeSet<String>,
    strict: bool,
) -> Result<bool, String> {
    let before = affected.len();
    let mut pending = analysis
        .interface_impacts
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    for (id, impact) in &analysis.interface_impacts {
        if !capture.current.packages.contains_key(id)
            || impact.reason.trim().is_empty()
            || impact.dependencies.is_empty()
            || impact.dependencies.iter().collect::<BTreeSet<_>>().len()
                != impact.dependencies.len()
        {
            return Err(format!("{id}: public interface impact requires an active owner, reason and unique dependencies"));
        }
        for dependency in &impact.dependencies {
            if dependency == id
                || !capture
                    .current
                    .dependencies
                    .iter()
                    .chain(&capture.baseline.dependencies)
                    .any(|edge| &edge.owner == id && &edge.target == dependency)
            {
                return Err(format!(
                    "{id}: public interface impact has no declared dependency on {dependency}"
                ));
            }
        }
    }
    while !pending.is_empty() {
        let resolved = pending
            .iter()
            .filter(|id| {
                analysis.interface_impacts[*id]
                    .dependencies
                    .iter()
                    .all(|d| affected.contains(d))
            })
            .cloned()
            .collect::<Vec<_>>();
        if resolved.is_empty() {
            if strict {
                return Err(
                    "public interface impact is not rooted in actual final package changes".into(),
                );
            }
            break;
        }
        for id in resolved {
            pending.remove(&id);
            affected.insert(id);
        }
    }
    Ok(affected.len() != before)
}

fn finish(
    root: &Path,
    message: &str,
    cfg: &Config,
    mut capture: Capture,
    analysis: Analysis,
    runner: &dyn ProcessRunner,
) -> Result<Option<FileReleasePlan>, String> {
    let inspection = InspectionRunner(runner);
    let runner: &dyn ProcessRunner = &inspection;
    let meaningful = capture
        .changes
        .iter()
        .filter(|c| c.role != "record")
        .cloned()
        .collect::<Vec<_>>();
    if meaningful.is_empty() {
        if !analysis.targets.is_empty() {
            return Err("version targets require actual release changes".into());
        }
        return Ok(None);
    }
    for id in analysis.targets.keys() {
        if id != "product" && !capture.current.packages.contains_key(id) {
            return Err(format!(
                "explicit target references unknown active package: {id}"
            ));
        }
    }
    let mut affected = affected(cfg, &capture);
    interface_impacts(&capture, &analysis, &mut affected, false)?;
    let mut actual = affected.clone();
    let program = analysis
        .product
        .as_ref()
        .ok_or("product analysis is required for a release")?;
    let breaking_ids = cfg
        .native
        .lifecycle
        .iter()
        .filter(|t| t.base == capture.boundary && t.compatibility == "breaking")
        .flat_map(|t| t.from.iter().chain(&t.to).map(|p| p.id.clone()))
        .collect::<BTreeSet<_>>();
    if !breaking_ids.is_empty() && program.compatibility != "breaking" {
        return Err("product: breaking lifecycle transition requires breaking analysis and explicit user acceptance".into());
    }
    let program_level = program.level("product", &capture.fingerprint)?;
    let old = capture
        .inputs
        .get(&root.join("VERSION"))
        .and_then(Option::as_ref)
        .ok_or("root VERSION missing")?;
    let old_version = std::str::from_utf8(old)
        .map_err(|e| e.to_string())?
        .trim()
        .parse::<SemVer>()?;
    let mut versions = BTreeMap::new();
    let mut impacts = BTreeMap::new();
    let mut new_version = old_version;
    let mut suggested = old_version;
    let mut writes = BTreeMap::new();
    for round in 0..=capture.current.packages.len() + 1 {
        impacts.clear();
        let mut maximum = program_level;
        for id in &affected {
            let impact = level(&analysis, id, &capture.fingerprint)?;
            if breaking_ids.contains(id) && analysis.packages[id].compatibility != "breaking" {
                return Err(format!(
                    "{id}: breaking lifecycle transition requires breaking analysis and explicit user acceptance"
                ));
            }
            impacts.insert(id.clone(), impact);
            maximum = maximum.max(impact);
        }
        if program_level < maximum {
            return Err("product analysis must cover the highest actual package impact".into());
        }
        suggested = polyglot::bump_native(&product(old_version), maximum)?.parse()?;
        new_version = choose(
            &product(old_version),
            maximum,
            analysis.targets.get("product"),
            false,
        )?
        .parse()?;
        versions.clear();
        for (id, impact) in &impacts {
                if let Some(p) = capture.current.packages.get(id) {
                    versions.insert(
                        p.manifest.clone(),
                        choose(
                            p,
                            *impact,
                            analysis.targets.get(id),
                            !capture.baseline.packages.contains_key(id),
                        )?,
                    );
                }
        }
        writes = render(root, cfg, &capture, &versions)?;
        let mut requirement_owners = BTreeSet::new();
        if let Some(rust_cfg) = &cfg.native.adapters.rust {
            requirement_owners.extend(rust::requirement_changes(
                rust_cfg,
                &capture.inventory,
                &mut |p| config::read_current(root, p),
                &writes,
            )?);
        }
        for p in capture.current.packages.values().filter(|p| {
            p.language == Language::Python
                && writes.contains_key(&p.manifest)
                && !versions.contains_key(&p.manifest)
        }) {
            requirement_owners.insert(p.manifest.clone());
        }
        let mut changed = false;
        for manifest in requirement_owners {
            let id = capture
                .current
                .packages
                .iter()
                .find(|(_, p)| p.manifest == manifest)
                .map(|(id, _)| id.clone())
                .ok_or("dependency owner is not registered")?;
            actual.insert(id.clone());
            changed |= affected.insert(id);
        }
        changed |= interface_impacts(&capture, &analysis, &mut actual, false)?;
        affected.extend(actual.iter().cloned());
        if !changed {
            break;
        }
        if round == capture.current.packages.len() + 1 {
            return Err("effective dependency requirement closure did not converge".into());
        }
    }
    interface_impacts(&capture, &analysis, &mut actual, true)?;
    if analysis.targets.keys().any(|id| id != "product" && !affected.contains(id)) {
        return Err("version targets cannot upgrade an unchanged package".into());
    }
    if analysis.packages.keys().any(|id| !affected.contains(id)) {
        return Err(
            "analysis attempts to upgrade a package with no final change or explicit target".into(),
        );
    }
    let mut evidence_scope = affected.clone();
    evidence_scope.insert("product".into());
    analysis::validate_checks(root, cfg, &mut capture, &analysis, &evidence_scope)?;
    if !cfg.product_manifests.is_empty() {
        polyglot::add_writes(
            &mut writes,
            rust::render_product_mappings(
                &cfg.product_manifests,
                &capture.inventory,
                &mut |p| config::read_current(root, p),
                old_version,
                new_version,
            )?,
        )?;
    }
    let kind = if root.join("templates/managed-skeleton.json").is_file() {
        ReleaseKind::Factory
    } else {
        ReleaseKind::Business
    };
    let paths = meaningful
        .iter()
        .map(|c| c.path.clone())
        .collect::<Vec<_>>();
    let mut info = parse_commit_message(message)?;
    info.breaking = program_level == 3;
    info.kind = if program_level == 2 {
        "feat"
    } else if program_level == 3 {
        "feat"
    } else {
        "chore"
    }
    .into();
    let mut log = String::from_utf8(render_changelog(root, new_version, &[info], &kind, &paths)?)
        .map_err(|e| e.to_string())?;
    let mut changes = Vec::new();
    let mut events = Vec::new();
    let mut all_packages = Vec::new();
    for (id, p) in &capture.current.packages {
        let after = versions.get(&p.manifest).unwrap_or(&p.version);
        let reason = if affected.contains(id) {
            analysis
                .packages
                .get(id)
                .map(|a| a.reason.as_str())
                .unwrap_or("first release")
        } else {
            "no_final_package_change"
        };
        all_packages.push(json!({"id":id,"language":p.language,"manifest":p.manifest,"version_before":p.version,"version_after":after,"reason":reason}));
        if versions.contains_key(&p.manifest) {
            changes.push(PackageChange {
                manifest: p.manifest.clone(),
                name: p.name.clone(),
                version_before: p.version.clone(),
                version_after: after.clone(),
                reason: reason.into(),
                dependencies: vec![],
            });
        }
    }
    for id in capture
        .baseline
        .packages
        .keys()
        .chain(capture.current.packages.keys())
        .collect::<BTreeSet<_>>()
    {
        let before = capture.baseline.packages.get(id);
        let after = capture.current.packages.get(id);
        let operation = if before.is_none() {
            "add"
        } else if after.is_none() {
            "retire"
        } else if before.unwrap().name != after.unwrap().name {
            "rename"
        } else if before.unwrap().manifest != after.unwrap().manifest {
            "move"
        } else {
            "modify"
        };
        if operation != "modify" || affected.contains(id) {
            let p = after.or(before).unwrap();
            events.push(PackageEvent {
                id: id.clone(),
                language: format!("{:?}", p.language).to_lowercase(),
                operation: operation.into(),
                manifest_before: before.map(|p| p.manifest.clone()),
                manifest_after: after.map(|p| p.manifest.clone()),
                name_before: before.map(|p| p.name.clone()),
                name_after: after.map(|p| p.name.clone()),
                version_before: before.map(|p| p.version.clone()),
                version_after: after
                    .map(|p| versions.get(&p.manifest).unwrap_or(&p.version).clone()),
                related: cfg
                    .native
                    .lifecycle
                    .iter()
                    .filter(|t| {
                        t.base == capture.boundary
                            && t.from.iter().chain(&t.to).any(|p| p.id == *id)
                    })
                    .flat_map(|t| t.from.iter().chain(&t.to).map(|p| p.id.clone()))
                    .filter(|p| p != id.as_str())
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect(),
            });
        }
    }
    let heading = format!("## [{new_version}]");
    let start = log
        .find(&heading)
        .ok_or("release changelog heading missing")?;
    let end = log[start + heading.len()..]
        .find("\n## [")
        .map(|i| start + heading.len() + i)
        .unwrap_or(log.len());
    let mut section = format!("\n### Release analysis\n\n- Product: {}\n", program.reason);
    for p in &all_packages {
        section.push_str(&format!(
            "- `{}`: {} -> {} ({})\n",
            p["id"].as_str().unwrap(),
            p["version_before"].as_str().unwrap(),
            p["version_after"].as_str().unwrap(),
            p["reason"].as_str().unwrap()
        ));
    }
    for e in &events {
        if e.operation == "retire" {
            section.push_str(&format!("- `{}`: retired\n", e.id));
        }
    }
    log.insert_str(end, &section);
    writes.insert("VERSION".into(), format!("{new_version}\n").into_bytes());
    writes.insert("CHANGELOG.md".into(), log.into_bytes());
    for path in writes.keys() {
        capture
            .inputs
            .entry(root.join(path))
            .or_insert(config::read_current(root, path)?);
    }
    verify_release_inputs(&capture.inputs)?;
    if let Some(environment) = &capture.environment_guard {
        environment.verify(root)?;
    }
    if independent::status(root, runner)? != capture.status
        || independent::inventory(root, runner)? != capture.inventory
        || history::git(root, runner, &["rev-parse", "HEAD"])?.trim() != capture.head
    {
        return Err("repository changed before release plan completion".into());
    }
    let details = json!({"schema_version":4,"fingerprint":capture.fingerprint,"validation_fingerprint":capture.validation_fingerprint,"python_environment_fingerprint":capture.environment_guard.as_ref().map(|e|e.fingerprint.clone()),"strategy":cfg.strategy,"suggested_version":suggested.to_string(),"version_after":new_version.to_string(),"packages":all_packages,"required_checks":cfg.required_checks,"check_scopes":cfg.check_scopes,"interface_impacts":analysis.interface_impacts.iter().map(|(id,impact)|json!({"id":id,"dependencies":impact.dependencies,"reason":impact.reason})).collect::<Vec<_>>(),"compatibility_validation":"agent_evidence_verified_not_automatically_proven"});
    Ok(Some(FileReleasePlan {
        inputs: capture.inputs,
        old_version,
        new_version,
        kind,
        writes: writes.into_iter().map(|(p, b)| (root.join(p), b)).collect(),
        package_changes: changes,
        package_events: events,
        lifecycle_events: cfg
            .native
            .lifecycle
            .iter()
            .filter(|t| t.base == capture.boundary)
            .map(serde_json::to_value)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?,
        release_inventory: Some(capture.inventory),
        release_status: Some(capture.status),
        details: Some(details),
        environment: capture.environment_guard,
    }))
}

pub fn build(
    root: &Path,
    message: &str,
    request: &Request,
    runner: &dyn ProcessRunner,
) -> Result<Option<FileReleasePlan>, String> {
    let cfg = load(root)?.ok_or("release requires schema 4 migration; run --migration-plan")?;
    let mut capture = capture(root, &cfg, runner)?;
    let analysis = analysis::read(root, request, &mut capture)?;
    if capture.changes.iter().all(|c| c.role == "record")
        && analysis.as_ref().is_none_or(|a| a.targets.is_empty())
    {
        return Ok(None);
    }
    let analysis=analysis.ok_or("schema 4 requires final-difference analysis; run --release-preview and supply --release-analysis-file")?;
    finish(root, message, &cfg, capture, analysis, runner)
}

pub fn preview(
    root: &Path,
    message: &str,
    request: &Request,
    runner: &dyn ProcessRunner,
) -> CommandOutcome {
    let result = (|| {
        let cfg = load(root)?.ok_or("release requires schema 4 migration; run --migration-plan")?;
        let mut capture = capture(root, &cfg, runner)?;
        let analysis = analysis::read(root, request, &mut capture)?;
        if capture.changes.iter().all(|c| c.role == "record")
            && analysis.as_ref().is_none_or(|a| a.targets.is_empty())
        {
            let mut value = analysis::requirements(&cfg, &capture);
            value["status"] = json!("nothing-to-release");
            value["package_changes"] = json!([]);
            return Ok(value);
        }
        let Some(analysis) = analysis else {
            return Ok(analysis::requirements(&cfg, &capture));
        };
        let p = finish(root, message, &cfg, capture, analysis, runner)?
            .ok_or("release plan unexpectedly empty")?;
        Ok::<_, String>(
            json!({"schema":1,"status":"release-planned","version_before":p.old_version.to_string(),"version_after":p.new_version.to_string(),"version_strategy":cfg.strategy,"package_changes":p.package_changes,"package_events":p.package_events,"lifecycle_events":p.lifecycle_events,"details":p.details,"kind":p.kind,"writes":p.writes.keys().map(|p|p.strip_prefix(root).unwrap_or(p).to_string_lossy().replace('\\',"/")).collect::<Vec<_>>()}),
        )
    })();
    match result {
        Ok(v) => CommandOutcome::with_receipt(v),
        Err(e) => CommandOutcome::blocked(format!("[schema4-release] {e}\n")),
    }
}
