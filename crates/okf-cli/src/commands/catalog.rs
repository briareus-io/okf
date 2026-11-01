//! Catalog selection and scoped query dispatch; ordinary artifact operations stay confined.
use crate::cli::{
    ArtifactCmd, ChangeSetCmd, Cli, Command, ComputationCmd, OntFieldTypeCmd, OntologyCmd,
};
use crate::output;
use okf_core::bundle::{
    catalog::load_catalog,
    config::{find_config, load_config},
    context::{current_context, Context},
};
use okf_core::error::{OkfError, Result};
use okf_core::graph::catalog::{
    build_catalog_graph, resolve_reference, resolve_scope, resolve_scope_at, CatalogGraph, Edge,
    Node,
};
use serde_json::json;
use std::path::PathBuf;

thread_local! { static EXAMINATION_SCOPE: std::cell::RefCell<Vec<String>> = const {std::cell::RefCell::new(Vec::new())}; }
pub fn examination_scope() -> Vec<String> {
    EXAMINATION_SCOPE.with(|v| v.borrow().clone())
}
pub fn bundle_slot(command: &mut Command) -> Option<&mut Option<String>> {
    Some(match command {
        Command::Bundle(sub) => match sub {
            crate::cli::BundleCmd::Context(a) => &mut a.bundle,
            crate::cli::BundleCmd::Validate(a) => &mut a.bundle,
            crate::cli::BundleCmd::Resource(crate::cli::BundleResourceCmd::List(a)) => {
                &mut a.bundle
            }
            crate::cli::BundleCmd::Resource(crate::cli::BundleResourceCmd::Show(a)) => {
                &mut a.bundle
            }
        },
        Command::List(a) => &mut a.bundle,
        Command::Validate(a) => &mut a.bundle,
        Command::Search(a) => &mut a.bundle,
        Command::Show(a) => &mut a.bundle,
        Command::Browse(a) => &mut a.bundle,
        Command::Backlinks(a) | Command::Links(a) => &mut a.bundle,
        Command::Graph(a) => &mut a.bundle,
        Command::Resolve(a) => &mut a.bundle,
        Command::Scan(a) | Command::Stats(a) => &mut a.bundle,
        Command::Stale(a) => &mut a.bundle,
        Command::Lint(a) => &mut a.bundle,
        Command::Affected(a) => &mut a.bundle,
        Command::Diff(a) => &mut a.bundle,
        Command::Doctor(a) => &mut a.bundle,
        Command::Init(a) => &mut a.bundle,
        Command::Add(a) => &mut a.bundle,
        Command::Edit(a) => &mut a.bundle,
        Command::Mv(a) => &mut a.bundle,
        Command::Rm(a) => &mut a.bundle,
        Command::Verify(a) => &mut a.bundle,
        Command::Refresh(a) => &mut a.bundle,
        Command::Docs(a) => &mut a.bundle,
        Command::Changeset(cmd) => match cmd {
            ChangeSetCmd::Plan(a) | ChangeSetCmd::Apply(a) => &mut a.bundle,
            ChangeSetCmd::Digest(a) => &mut a.bundle,
            ChangeSetCmd::Recover(a) => &mut a.bundle,
        },
        Command::Artifact(cmd) => match cmd {
            ArtifactCmd::List(a) => &mut a.bundle,
            ArtifactCmd::Resolve(a) => &mut a.bundle,
            ArtifactCmd::Show(a) => &mut a.bundle,
            ArtifactCmd::Put(a) => &mut a.bundle,
        },
        Command::Computation(ComputationCmd::Check(a)) => &mut a.bundle,
        Command::Ontology(cmd) => match cmd {
            OntologyCmd::List(a) => &mut a.bundle,
            OntologyCmd::Show(a) => &mut a.bundle,
            OntologyCmd::Add(a) | OntologyCmd::Update(a) => &mut a.bundle,
            OntologyCmd::Remove(a) => &mut a.bundle,
            OntologyCmd::Apply(a) => &mut a.bundle,
            OntologyCmd::FieldType(cmd) => match cmd {
                OntFieldTypeCmd::List(a) => &mut a.bundle,
                OntFieldTypeCmd::Show(a) => &mut a.bundle,
                OntFieldTypeCmd::Add(a) | OntFieldTypeCmd::Update(a) => &mut a.bundle,
                OntFieldTypeCmd::Remove(a) => &mut a.bundle,
            },
        },
        _ => return None,
    })
}
fn print_edge(edge: &Edge, json_output: bool) -> Result<()> {
    if json_output {
        let mut value =
            serde_json::to_value(edge).map_err(|e| OkfError::Internal(e.to_string()))?;
        value["kind"] = json!("bundle-edge");
        output::print_line(&value)?;
    } else {
        output::print_text(format_args!("{}", edge_text(edge)))?;
    }
    Ok(())
}
fn edge_text(edge: &Edge) -> String {
    let mut text = format!(
        "{}\t{}\t{}\t{}\t{}\n",
        edge.source.key(),
        edge.resource,
        edge.target
            .as_ref()
            .map(Node::key)
            .unwrap_or_else(|| "?".into()),
        edge.status,
        edge.evidence
    );
    if let Some(snapshot) = &edge.snapshot {
        text.push_str(&format!(
            "  snapshot={} evidence={} requested={} candidate={}\n",
            snapshot.status,
            snapshot.evidence,
            snapshot
                .resolved
                .as_ref()
                .map(Node::key)
                .unwrap_or_else(|| "unresolved".into()),
            snapshot
                .candidate
                .as_ref()
                .map(Node::key)
                .unwrap_or_else(|| "none".into())
        ));
    }
    text
}
fn print_node(node: &Node, kind: &str, json_output: bool) -> Result<()> {
    if json_output {
        output::print_line(
            &json!({"kind":kind,"bundle":node.bundle,"id":node.id,"version":node.version}),
        )?;
    } else {
        output::print_text_line(format_args!("{}", node.key()))?;
    }
    Ok(())
}
fn print_scope(graph: &CatalogGraph, json_output: bool) -> Result<()> {
    if json_output {
        output::print_line(
            &json!({"kind":"scope","requested":graph.scope.requested,"examined":graph.scope.examined,"unavailable":graph.scope.unavailable,"snapshot_examined":graph.scope.snapshot_examined}),
        )?;
    } else {
        output::print_text_line(format_args!("{}", scope_text(graph)))?;
    }
    Ok(())
}
fn scope_text(graph: &CatalogGraph) -> String {
    format!(
        "scope: requested={} examined={} unavailable={}",
        graph.scope.requested.join(","),
        graph
            .scope
            .examined
            .iter()
            .map(|e| format!("{}@{}", e.id, e.version))
            .collect::<Vec<_>>()
            .join(","),
        graph.scope.unavailable.join(",")
    )
}
pub fn catalog_path() -> Result<PathBuf> {
    let cwd = std::env::current_dir()?;
    if let Some(path) = std::env::var_os("OKF_CATALOG") {
        return Ok(cwd.join(path));
    }
    if let Some(path) = okf_core::bundle::catalog::discover(&cwd) {
        return Ok(path);
    }
    if let Some(path) = find_config(&cwd) {
        let config = load_config(&path)?;
        if let Some(catalog) = config.catalog {
            return Ok(path.parent().unwrap_or(&cwd).join(catalog));
        }
    }
    Err(OkfError::Usage(
        "no catalog found; use --catalog or create okf-catalog.yaml".into(),
    ))
}
pub fn run_catalog(json_output: bool) -> Result<i32> {
    let path = catalog_path()?;
    let overrides = std::env::var_os("OKF_CATALOG_OVERRIDES")
        .map(PathBuf::from)
        .or_else(|| {
            let config_path = find_config(&std::env::current_dir().ok()?)?;
            let config = load_config(&config_path).ok()?;
            config.catalog_overrides.map(|p| {
                config_path
                    .parent()
                    .unwrap_or(std::path::Path::new("."))
                    .join(p)
            })
        });
    let catalog = load_catalog(&path, overrides.as_deref())?;
    for entry in catalog.entries.values() {
        if json_output {
            let mut value =
                serde_json::to_value(entry).map_err(|e| OkfError::Internal(e.to_string()))?;
            value["kind"] = json!("bundle-registration");
            output::print_line(&value)?
        } else {
            output::print_text_line(format_args!(
                "{}\t{}\t{}",
                entry.id,
                entry.root.display(),
                if entry.available {
                    "available"
                } else {
                    "unavailable"
                }
            ))?
        }
        if entry.available {
            let effective = okf_core::bundle::settings::load_for(&entry.root)?.0;
            if json_output {
                output::print_line(
                    &json!({"kind":"effective-settings","bundle":entry.id,"settings":effective}),
                )?
            }
        }
    }
    Ok(0)
}
pub fn run_catalog_command(args: &crate::cli::CatalogArgs, cli: &Cli) -> Result<i32> {
    use crate::cli::CatalogCmd;
    let Some(command) = &args.command else {
        return run_catalog(cli.json);
    };
    if cli.revision.is_some() || cli.workspace_revision.is_some() || !cli.bundle_revision.is_empty()
    {
        return Err(OkfError::Usage(
            "catalog lifecycle commands do not accept historical selectors".into(),
        ));
    }
    let path = catalog_path()?;
    match command {
        CatalogCmd::Sync(a) => {
            if !cli.scope_bundle.is_empty() || cli.catalog_scope {
                return Err(OkfError::Usage(
                    "sync scope uses --bundle-id; consumer scope applies only to update plans"
                        .into(),
                ));
            }
            if a.rebuild {
                let report =
                    okf_core::bundle::remote::recover_cache(&path, cli.bundle_id.as_deref())?;
                output_report("catalog-cache-recovery", &report, cli.json)?;
                return Ok(0);
            }
            let report = okf_core::bundle::remote::sync(&path, cli.bundle_id.as_deref(), a.check)?;
            output_report("catalog-sync", &report, cli.json)?;
        }
        CatalogCmd::Update(a) => {
            if let Some(plan_path) = &a.apply {
                if cli.bundle_id.is_some() || !cli.scope_bundle.is_empty() || cli.catalog_scope {
                    return Err(OkfError::Usage(
                        "apply uses the saved plan selection; do not supply new selectors".into(),
                    ));
                }
                let plan = okf_core::bundle::remote::load_plan(std::path::Path::new(plan_path))?;
                let report =
                    okf_core::bundle::remote::apply_plan(&path, &plan, a.allow_incomplete_impact)?;
                output::mark_committed();
                output_report("catalog-update-applied", &report, cli.json)?;
            } else {
                let mut roots = Vec::new();
                if !cli.scope_bundle.is_empty() || cli.catalog_scope {
                    let file: okf_core::bundle::catalog::CatalogFile =
                        serde_yaml::from_slice(&std::fs::read(&path)?)
                            .map_err(|e| OkfError::Usage(format!("invalid catalog: {e}")))?;
                    let ids = if cli.catalog_scope {
                        file.bundles
                            .iter()
                            .filter(|(_, r)| r.location.type_ == "directory")
                            .map(|(id, _)| id.clone())
                            .collect::<Vec<_>>()
                    } else {
                        cli.scope_bundle.clone()
                    };
                    let directory_entries =
                        okf_core::bundle::catalog::declared_directory_entries(&path)?;
                    for id in ids {
                        let registration = file.bundles.get(&id).ok_or_else(|| {
                            OkfError::Usage(format!("unknown consumer bundle {id}"))
                        })?;
                        let root = if registration.location.type_ == "directory" {
                            directory_entries
                                .get(&id)
                                .ok_or_else(|| {
                                    OkfError::Usage(format!("unknown consumer bundle {id}"))
                                })?
                                .root
                                .clone()
                        } else {
                            okf_core::bundle::remote::resolve_registration(&path, &id)?.root
                        };
                        roots.push(root);
                    }
                }
                let plan =
                    okf_core::bundle::remote::plan_update(&path, cli.bundle_id.as_deref(), &roots)?;
                let dest = a
                    .plan
                    .as_ref()
                    .map(PathBuf::from)
                    .map(Ok)
                    .unwrap_or_else(|| okf_core::bundle::remote::default_plan_path(&plan))?;
                okf_core::bundle::remote::save_plan(&dest, &plan)?;
                output_report("catalog-update-plan", &plan, cli.json)?;
                if cli.json {
                    output::print_line(&json!({"kind":"plan-file","path":dest}))?
                } else {
                    output::print_text_line(format_args!("Plan saved: {}", dest.display()))?
                }
            }
        }
    }
    Ok(0)
}
fn output_report<T: serde::Serialize>(kind: &str, value: &T, json_output: bool) -> Result<()> {
    if json_output {
        let mut row = serde_json::to_value(value).map_err(|e| OkfError::Internal(e.to_string()))?;
        if let Some(plan_kind) = row.get("kind").cloned() {
            row["plan_kind"] = plan_kind;
        }
        row["kind"] = json!(kind);
        output::print_line(&row)
    } else {
        output::print_text_line(format_args!(
            "{}",
            serde_yaml::to_string(value).map_err(|e| OkfError::Internal(e.to_string()))?
        ))
    }
}
pub fn writes_command(command: &Command) -> bool {
    match command {
        Command::Init(_)
        | Command::Mv(_)
        | Command::Rm(_)
        | Command::Verify(_)
        | Command::Refresh(_)
        | Command::Artifact(ArtifactCmd::Put(_)) => true,
        Command::Lint(a) => a.fix,
        Command::Add(a) => !a.dry_run,
        Command::Edit(a) => !a.dry_run,
        Command::Doctor(a) => a.migration_apply.is_some() || a.fix_safe && a.yes && !a.dry_run,
        Command::Catalog(a) => {
            matches!(&a.command,Some(crate::cli::CatalogCmd::Update(u)) if u.apply.is_some())
        }
        Command::Docs(a) => a.format == "index",
        Command::Ontology(OntologyCmd::Add(a) | OntologyCmd::Update(a)) => !a.dry_run,
        Command::Ontology(OntologyCmd::Remove(a)) => !a.dry_run,
        Command::Ontology(OntologyCmd::Apply(a)) => !a.dry_run,
        Command::Ontology(OntologyCmd::FieldType(
            OntFieldTypeCmd::Add(a) | OntFieldTypeCmd::Update(a),
        )) => !a.dry_run,
        Command::Ontology(OntologyCmd::FieldType(OntFieldTypeCmd::Remove(a))) => !a.dry_run,
        _ => false,
    }
}

/// Returns Some when a catalog-aware query handled the command; otherwise rewrites only
/// the selected root so existing authoring/query handlers retain their established contracts.
pub fn prepare(cli: &mut Cli) -> Result<Option<i32>> {
    if let Command::Catalog(args) = &cli.command {
        if args.command.is_none()
            && (cli.bundle_id.is_some()
                || !cli.scope_bundle.is_empty()
                || cli.catalog_scope
                || cli.revision.is_some())
        {
            return Err(OkfError::Usage(
                "catalog inspection does not take selectors or scope/revision flags".into(),
            ));
        }
        return run_catalog_command(args, cli).map(Some);
    }
    let scoped = !cli.scope_bundle.is_empty() || cli.catalog_scope;
    if scoped && matches!(&cli.command, Command::Lint(a) if a.fix) {
        return Err(OkfError::Usage(
            "lint --fix requires one selected bundle; scope flags are not supported".into(),
        ));
    }
    let revision = cli.revision.is_some();
    let scope_command = crate::exit::command_name(&cli.command);
    let supports_scope = crate::cli::supports_scope(&scope_command);
    if scoped && !supports_scope {
        return Err(OkfError::Usage("scope flags are supported only by graph, backlinks, links, resolve, affected, lint, search, and list".into()));
    }
    if revision && !crate::cli::supports_revision(&scope_command) {
        return Err(OkfError::Usage("--revision is supported only by graph, backlinks, links, resolve, affected, search, and list; this command reads the working tree".into()));
    }
    let command_name = crate::exit::command_name(&cli.command);
    let diagnostic = matches!(
        &cli.command,
        Command::Doctor(_)
            | Command::Validate(_)
            | Command::Bundle(crate::cli::BundleCmd::Validate(_))
            | Command::Bundle(crate::cli::BundleCmd::Resource(_))
    );
    let writes = writes_command(&cli.command);
    let Some(slot) = bundle_slot(&mut cli.command) else {
        if cli.bundle_id.is_some() {
            return Err(OkfError::Usage(
                "this command does not select an OKF bundle".into(),
            ));
        }
        return Ok(None);
    };
    let explicit = slot.clone();
    if cli.bundle_id.is_some() && explicit.is_some() {
        return Err(OkfError::Usage(
            "bundle path and --bundle-id are conflicting selectors".into(),
        ));
    }
    let context = current_context(explicit.as_deref(), cli.bundle_id.as_deref())?;
    let mut ids = cli.scope_bundle.clone();
    if cli.catalog_scope {
        if let Some(c) = &context.catalog {
            ids.extend(c.entries.keys().cloned())
        }
    }
    if let Some(id) = &context.primary.id {
        ids.push(id.clone())
    }
    ids.sort();
    ids.dedup();
    EXAMINATION_SCOPE.with(|v| *v.borrow_mut() = ids);

    if writes {
        okf_core::mutate::transaction::ensure_idle(&context.primary.root)?;
    }
    if scoped && context.catalog.is_none() {
        if let Some(catalog_path) = &context.config.catalog {
            let base = context
                .config_path
                .as_ref()
                .and_then(|p| p.parent())
                .unwrap_or(std::path::Path::new("."));
            let overrides = std::env::var("OKF_CATALOG_OVERRIDES")
                .ok()
                .filter(|s| !s.is_empty())
                .map(PathBuf::from)
                .or_else(|| {
                    context
                        .config
                        .catalog_overrides
                        .as_ref()
                        .map(|s| base.join(s))
                });
            load_catalog(&base.join(catalog_path), overrides.as_deref())?;
        }
        return Err(OkfError::Usage(
            "explicit bundle scope requires a valid catalog".into(),
        ));
    }
    if !diagnostic && command_name != "bundle context" {
        let guidance_scope = resolve_scope(&context, &cli.scope_bundle, cli.catalog_scope)?;
        if !guidance_scope.unavailable.is_empty() {
            output::mark_partial("examination", "guidance-scope-unavailable");
            if cli.json {
                output::print_line(
                    &json!({"kind":"guidance-scope","requested":guidance_scope.requested,"unavailable":guidance_scope.unavailable,"complete":false}),
                )?;
            } else {
                eprintln!(
                    "guidance unavailable for requested bundles: {}",
                    guidance_scope.unavailable.join(", ")
                );
            }
        }
        for member in &guidance_scope.examined {
            super::bundle::incidental(
                &member.root,
                &command_name,
                cli.json,
                cli.guidance.as_deref(),
            )?;
        }
    }
    if cli.bundle_id.is_some() || context.catalog.is_some() {
        *slot = Some(context.primary.root.to_string_lossy().into_owned());
    }
    match &mut cli.command {
        Command::Search(a) => {
            super::filters::apply_primary(&context.primary.root, "search", a, cli.json)?
        }
        Command::List(a) => {
            super::filters::apply_primary(&context.primary.root, "list", a, cli.json)?
        }
        _ => {}
    }
    if matches!(&cli.command, Command::Mv(_) | Command::Rm(_)) && context.catalog.is_some() {
        let graph = build_catalog_graph(&context, &cli.scope_bundle, cli.catalog_scope, None)?;
        if !graph.scope.unavailable.is_empty() {
            return Err(OkfError::Policy(format!(
                "cannot establish mutation safety: unavailable requested bundles: {}",
                graph.scope.unavailable.join(", ")
            )));
        }
        let concept = match &cli.command {
            Command::Mv(a) => &a.old,
            Command::Rm(a) => &a.concept,
            _ => unreachable!(),
        };
        let target = graph.target(&context, concept);
        let consumers = graph
            .edges
            .iter()
            .filter(|e| e.target.as_ref() == Some(&target) && e.source.bundle != target.bundle)
            .map(|e| e.source.key())
            .collect::<std::collections::BTreeSet<_>>();
        if !consumers.is_empty() {
            return Err(OkfError::Policy(format!(
                "cross-bundle references require explicit reconciliation before this mutation: {}",
                consumers.into_iter().collect::<Vec<_>>().join(", ")
            )));
        }
        if cli.json {
            output::print_line(
                &json!({"kind":"mutation-scope","examined":graph.scope.requested,"out_of_scope":"unknown"}),
            )?
        } else {
            eprintln!(
                "mutation scope: {}; out-of-scope consumers remain unknown",
                graph.scope.requested.join(", ")
            )
        }
        return Ok(None);
    }
    // Catalog-aware graph queries use selected-only scope even without an explicit scope flag.
    if supports_scope
        && !matches!(
            &cli.command,
            Command::Artifact(_) | Command::Stale(_) | Command::Refresh(_) | Command::Bundle(_)
        )
        && (context.catalog.is_some() || scoped || revision)
    {
        if let Command::Search(args) | Command::List(args) = &cli.command {
            let scope = resolve_scope_at(
                &context,
                &cli.scope_bundle,
                cli.catalog_scope,
                cli.revision.as_deref(),
            )?;
            let graph = CatalogGraph {
                scope,
                ..Default::default()
            };
            print_scope(&graph, cli.json)?;
            let roots = graph
                .scope
                .examined
                .iter()
                .map(|e| (e.id.clone(), e.root.clone(), e.version.clone()))
                .collect::<Vec<_>>();
            return super::query::run_search_in_scope(args, cli.json, &roots).map(Some);
        }
        let graph = if let Command::Links(a) | Command::Backlinks(a) = &cli.command {
            let bounded = super::filters::requested(&a.filters)
                || a.limit.is_some()
                || a.scan_limit.is_some()
                || a.full_scan;
            if a.limit == Some(0) || a.scan_limit == Some(0) {
                return Err(OkfError::Usage("query limits must be positive".into()));
            }
            if bounded {
                let limit = if a.full_scan {
                    None
                } else {
                    Some(
                        a.scan_limit
                            .or(okf_core::bundle::settings::for_context(
                                &context,
                                &context.primary.root,
                            )?
                            .settings
                            .query
                            .scan_limit)
                            .unwrap_or(1000),
                    )
                };
                let source = matches!(cli.command, Command::Links(_)).then_some(a.concept.as_str());
                okf_core::graph::catalog::build_catalog_graph_limited(
                    &context,
                    &cli.scope_bundle,
                    cli.catalog_scope,
                    cli.revision.as_deref(),
                    limit,
                    source,
                )?
            } else {
                build_catalog_graph(
                    &context,
                    &cli.scope_bundle,
                    cli.catalog_scope,
                    cli.revision.as_deref(),
                )?
            }
        } else {
            build_catalog_graph(
                &context,
                &cli.scope_bundle,
                cli.catalog_scope,
                cli.revision.as_deref(),
            )?
        };
        if matches!(cli.command, Command::Graph(_)) && !cli.json {
            use std::io::Write;
            writeln!(std::io::stderr().lock(), "{}", scope_text(&graph))?;
        } else {
            print_scope(&graph, cli.json)?;
        }
        let graph = filter_scoped(cli, &context, &graph)?;
        return run_scoped(cli, &context, &graph).map(Some);
    }
    Ok(None)
}
fn filter_scoped(cli: &mut Cli, context: &Context, graph: &CatalogGraph) -> Result<CatalogGraph> {
    use okf_core::graph::filter::{CompiledFilters, MatchOutcome};
    use std::collections::{BTreeMap, BTreeSet, VecDeque};
    let (args, operation) = match &cli.command {
        Command::Links(a) => (&a.filters, "links"),
        Command::Backlinks(a) => (&a.filters, "backlinks"),
        Command::Graph(a) => (&a.filters, "graph"),
        _ => return Ok(graph.clone()),
    };
    let pagination = matches!(&cli.command,Command::Links(a)|Command::Backlinks(a) if a.limit.is_some()||a.scan_limit.is_some()||a.full_scan);
    if !super::filters::requested(args) && !pagination {
        return Ok(graph.clone());
    }
    let (filters, compiled, preset_details) =
        super::filters::resolve(&context.primary.root, operation, args)?;
    super::filters::report(&filters, args.preset.as_deref(), cli.json)?;
    let mut concepts = BTreeMap::new();
    if filters.needs_related() {
        for node in &graph.nodes {
            if let Some(bundle) = graph.scope.examined.iter().find(|b| b.id == node.bundle) {
                let concept = if let Some(rev) = node.version.strip_prefix("git:") {
                    okf_core::graph::catalog::read_concept_at(&bundle.root, &node.id, rev)?
                } else {
                    okf_core::bundle::loader::load_concept(&bundle.root, &node.id)?
                };
                if let Some(c) = concept {
                    concepts.insert(node.clone(), c);
                }
            }
        }
    }
    let mut unknown = 0usize;
    let mut evaluate = |edge: &Edge, incoming: bool| -> Result<bool> {
        let other = if incoming {
            Some(&edge.source)
        } else {
            edge.target.as_ref()
        };
        match compiled.matches(
            edge.relationship.as_ref(),
            other.and_then(|n| concepts.get(n)),
        )? {
            MatchOutcome::Matched => Ok(true),
            MatchOutcome::Excluded => Ok(false),
            MatchOutcome::Unresolved => {
                unknown += 1;
                Ok(false)
            }
        }
    };
    let mut result = graph.clone();
    let mut matched = Vec::new();
    let scan_complete = graph.examination.as_ref().is_none_or(|e| e.scan_complete);
    let mut delivery_complete = true;
    let mut examined = 0usize;
    let mut matched_occurrences = 0usize;
    let mut observed_matches = 0usize;
    match &mut cli.command {
        Command::Links(a) | Command::Backlinks(a) => {
            a.details |= preset_details;
            if a.limit == Some(0) || a.scan_limit == Some(0) {
                return Err(OkfError::Usage("query limits must be positive".into()));
            }
            let incoming = operation == "backlinks";
            let target = graph.target(context, &a.concept);
            for edge in graph.edges.iter().filter(|e| {
                if incoming {
                    e.target.as_ref() == Some(&target)
                } else {
                    e.source == target
                }
            }) {
                examined += 1;
                if evaluate(edge, incoming)? {
                    matched.push(edge.clone());
                }
            }
            matched_occurrences = matched.len();
            if !a.details {
                let mut seen = BTreeSet::new();
                matched.retain(|e| {
                    seen.insert(if incoming {
                        Some(e.source.clone())
                    } else {
                        e.target.clone()
                    })
                });
            }
            let count = matched.len();
            observed_matches = count;
            let limit = a.limit.unwrap_or(usize::MAX);
            matched = matched.into_iter().skip(a.offset).take(limit).collect();
            delivery_complete = a.offset == 0 && matched.len() == count;
        }
        Command::Graph(a) => {
            let direction = a.direction.as_deref().unwrap_or("outgoing");
            if let Some(anchor) = &a.concept {
                let start = graph.target(context, anchor);
                let mut seen = BTreeSet::from([start.clone()]);
                let mut queue = VecDeque::from([(start, 0usize)]);
                let mut accepted = BTreeSet::new();
                while let Some((node, depth)) = queue.pop_front() {
                    if a.depth.is_some_and(|max| depth >= max) {
                        continue;
                    }
                    for (i, edge) in graph.edges.iter().enumerate() {
                        let incoming =
                            edge.target.as_ref() == Some(&node) && direction != "outgoing";
                        let outgoing = edge.source == node && direction != "incoming";
                        if !incoming && !outgoing {
                            continue;
                        }
                        examined += 1;
                        let pass = (outgoing && evaluate(edge, false)?)
                            || (incoming && evaluate(edge, true)?);
                        if pass {
                            accepted.insert(i);
                            let other = if outgoing {
                                edge.target.clone()
                            } else {
                                Some(edge.source.clone())
                            };
                            if let Some(other) = other {
                                if seen.insert(other.clone()) {
                                    queue.push_back((other, depth + 1));
                                }
                            }
                        }
                    }
                }
                result.nodes = seen;
                matched = accepted
                    .into_iter()
                    .map(|i| graph.edges[i].clone())
                    .collect();
            } else {
                if direction == "both" && filters.needs_related() {
                    return Err(OkfError::Usage(
                        "graph --direction both with related metadata filters requires --root"
                            .into(),
                    ));
                }
                for edge in &graph.edges {
                    examined += 1;
                    if evaluate(edge, direction == "incoming")? {
                        matched.push(edge.clone());
                    }
                }
            }
            matched_occurrences = matched.len();
            observed_matches = matched_occurrences;
        }
        _ => {}
    }
    result.edges = matched;
    if !scan_complete {
        output::mark_partial("examination", "relationship-scan-limit")
    }
    if !delivery_complete {
        output::mark_partial("delivery", "relationship-result-limit")
    }
    if unknown > 0 {
        output::mark_partial("matching", "unresolved-related-metadata")
    }
    if cli.json {
        output::print_line(
            &json!({"kind":"relationship-query-summary","examined_occurrences":examined,"matched_occurrences":matched_occurrences,"unresolved_candidates":unknown,"scan_complete":scan_complete,"delivery_complete":delivery_complete,"examined_documents":graph.examination.as_ref().map(|e|e.examined_documents),"eligible_documents":graph.examination.as_ref().map(|e|e.eligible_documents),"total_matches":if scan_complete{Some(observed_matches)}else{None}}),
        )?
    }
    Ok(result)
}

fn run_scoped(cli: &Cli, context: &Context, graph: &CatalogGraph) -> Result<i32> {
    match &cli.command {
        Command::Search(a) | Command::List(a) => {
            let scope = graph
                .scope
                .examined
                .iter()
                .map(|e| (e.id.clone(), e.root.clone(), e.version.clone()))
                .collect::<Vec<_>>();
            super::query::run_search_in_scope(a, cli.json, &scope)
        }
        Command::Links(a) => {
            let target = graph.target(context, &a.concept);
            if !graph.nodes.contains(&target) {
                return Err(OkfError::Usage(format!(
                    "concept not found in examined scope: {}",
                    a.concept
                )));
            }
            for edge in graph.edges.iter().filter(|e| e.source == target) {
                print_edge(edge, cli.json)?;
                if a.details && !cli.json {
                    if let Some(relationship) = &edge.relationship {
                        output::print_text_line(format_args!(
                            "  kind: {}",
                            relationship.authored_kind.as_deref().unwrap_or("unknown")
                        ))?;
                        output::print_text_line(format_args!(
                            "  source field: {}",
                            relationship.field_path
                        ))?;
                        if let Some(inverse) = &relationship.inverse {
                            output::print_text_line(format_args!("  inverse: {inverse}"))?;
                        }
                        for (key, value) in &relationship.attributes {
                            output::print_text_line(format_args!(
                                "  {}: {}",
                                key,
                                serde_json::to_string(value).unwrap_or_default()
                            ))?;
                        }
                    }
                }
            }
            Ok(0)
        }
        Command::Backlinks(a) => {
            let target = graph.target(context, &a.concept);
            let nodes: std::collections::BTreeSet<_> = graph
                .edges
                .iter()
                .filter(|e| e.target.as_ref() == Some(&target))
                .map(|e| e.source.clone())
                .collect();
            if a.details {
                for edge in graph
                    .edges
                    .iter()
                    .filter(|e| e.target.as_ref() == Some(&target))
                {
                    print_edge(edge, cli.json)?;
                }
            } else {
                for node in nodes {
                    print_node(&node, "bundle-backlink", cli.json)?;
                }
            }
            Ok(0)
        }
        Command::Resolve(a) => {
            let edge = resolve_reference(
                context,
                graph,
                a.from.as_deref().unwrap_or("/context"),
                &a.link,
            );
            print_edge(&edge, cli.json)?;
            Ok(if matches!(edge.status.as_str(), "resolved" | "external") {
                0
            } else {
                1
            })
        }
        Command::Affected(a) => {
            let raw = super::check::changed_inputs(a, &context.primary.root)?;
            let nodes = graph.affected_resources(context, &raw, a.transitive, a.depth);
            for node in &nodes {
                print_node(node, "bundle-affected", cli.json)?;
            }
            Ok(
                if !nodes.is_empty() && a.fail_on.as_deref().is_some_and(|s| s != "never") {
                    1
                } else {
                    0
                },
            )
        }
        Command::Graph(a) => {
            let selected = a.concept.as_ref().map(|s| {
                graph.neighborhood(
                    &graph.target(context, s),
                    a.direction.as_deref().unwrap_or("outgoing"),
                    a.depth,
                )
            });
            let edges = graph.edges.iter().filter(|e| {
                selected.as_ref().is_none_or(|n| {
                    n.contains(&e.source) && e.target.as_ref().is_none_or(|t| n.contains(t))
                })
            });
            if cli.json {
                for node in graph
                    .nodes
                    .iter()
                    .filter(|n| selected.as_ref().is_none_or(|s| s.contains(n)))
                {
                    print_node(node, "bundle-node", true)?;
                }
                for edge in edges {
                    print_edge(edge, true)?;
                }
            } else {
                let mut content = String::new();
                let mut rendered_nodes = graph.nodes.clone();
                for edge in &graph.edges {
                    if let Some(target) = &edge.target {
                        rendered_nodes.insert(target.clone());
                    }
                }
                let nodes: Vec<_> = rendered_nodes
                    .iter()
                    .filter(|n| selected.as_ref().is_none_or(|s| s.contains(n)))
                    .collect();
                let index = nodes
                    .iter()
                    .enumerate()
                    .map(|(i, n)| (n.key(), i))
                    .collect::<std::collections::BTreeMap<_, _>>();
                match a.format.as_str() {
                    "dot" => {
                        content.push_str("digraph okf {\n");
                        for (i, node) in nodes.iter().enumerate() {
                            content.push_str(&format!(
                                "  n{i} [label={}];\n",
                                serde_json::to_string(&node.key()).unwrap()
                            ));
                        }
                        for edge in edges {
                            if let (Some(i), Some(j)) = (
                                index.get(&edge.source.key()),
                                edge.target.as_ref().and_then(|n| index.get(&n.key())),
                            ) {
                                content.push_str(&format!(
                                    "  n{i} -> n{j} [label={}];\n",
                                    serde_json::to_string(&edge.status).unwrap()
                                ));
                            }
                        }
                        content.push_str("}\n");
                    }
                    "graphml" => {
                        content.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<graphml xmlns=\"http://graphml.graphdrawing.org/xmlns\"><key id=\"label\" for=\"node\" attr.name=\"label\" attr.type=\"string\"/><graph edgedefault=\"directed\">\n");
                        for (i, node) in nodes.iter().enumerate() {
                            content.push_str(&format!(
                                "<node id=\"n{i}\"><data key=\"label\">{}</data></node>\n",
                                xml(&node.key())
                            ));
                        }
                        for (edge_id, edge) in edges.enumerate() {
                            if let (Some(i), Some(j)) = (
                                index.get(&edge.source.key()),
                                edge.target.as_ref().and_then(|n| index.get(&n.key())),
                            ) {
                                content.push_str(&format!(
                                    "<edge id=\"e{edge_id}\" source=\"n{i}\" target=\"n{j}\"/>\n"
                                ));
                            }
                        }
                        content.push_str("</graph></graphml>\n");
                    }
                    _ => {
                        content.push_str("graph TD\n");
                        for (i, node) in nodes.iter().enumerate() {
                            content.push_str(&format!(
                                "  n{i}[\"{}\"]\n",
                                node.key().replace('&', "&amp;").replace('"', "&quot;")
                            ));
                        }
                        for edge in edges {
                            if let (Some(i), Some(j)) = (
                                index.get(&edge.source.key()),
                                edge.target.as_ref().and_then(|n| index.get(&n.key())),
                            ) {
                                content.push_str(&format!("  n{i} -->|{}| n{j}\n", edge.status));
                            }
                        }
                    }
                }
                output::print_text(format_args!("{content}"))?;
                // Keep unresolved references and snapshot candidates visible in human output.
                for edge in graph.edges.iter().filter(|e| {
                    (e.status != "resolved" || e.snapshot.is_some())
                        && selected
                            .as_ref()
                            .is_none_or(|nodes| nodes.contains(&e.source))
                }) {
                    use std::io::Write;
                    write!(std::io::stderr().lock(), "{}", edge_text(edge))?;
                }
            }
            Ok(0)
        }
        Command::Lint(a) => run_lint_scoped(a, cli.json, graph),
        _ => Err(OkfError::Internal("unsupported scoped dispatch".into())),
    }
}
fn xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
fn run_lint_scoped(
    args: &crate::cli::LintArgs,
    json_output: bool,
    graph: &CatalogGraph,
) -> Result<i32> {
    use okf_core::check::lint::{lint_bundle, meets_threshold, FailOn, Finding, Severity};
    use std::str::FromStr;
    let fail_on = FailOn::from_str(args.fail_on.as_deref().unwrap_or("error"))?;
    let mut failed = false;
    let mut broken_link_severities = std::collections::BTreeMap::new();
    let mut ontology_severities = std::collections::BTreeMap::new();
    let mut target_types = std::collections::BTreeMap::new();
    let mut target_ontologies = std::collections::BTreeMap::new();
    // Only the qualified graph can distinguish cross-bundle targets from local IDs.
    // Keep broken concept edges and structural navigation, but exclude self-links
    // and opaque artifact resources.
    let mut connected = std::collections::BTreeSet::new();
    for edge in &graph.edges {
        if let Some(target) = &edge.target {
            if edge.source != *target {
                connected.insert(edge.source.clone());
                connected.insert(target.clone());
            }
        } else if edge.resource_kind.as_deref() == Some("reserved") {
            // Index/log navigation is a supported outgoing document reference,
            // without treating the structural document as a concept graph node.
            connected.insert(edge.source.clone());
        }
    }
    for examined in &graph.scope.examined {
        okf_core::check::lint::validate_configuration(&examined.root)?;
        if let Some(ontology) = okf_core::bundle::settings::load_for(&examined.root)?.1 {
            target_ontologies.insert(examined.id.clone(), ontology);
        }
        for concept in okf_core::bundle::loader::load_bundle(&examined.root)?.concepts {
            if let Some(type_name) = concept.concept_type() {
                target_types.insert(
                    Node {
                        bundle: examined.id.clone(),
                        id: concept.id.0.clone(),
                        version: examined.version.clone(),
                    },
                    format!("{}:{type_name}", examined.id),
                );
            }
        }
    }
    for examined in &graph.scope.examined {
        let bundle = okf_core::bundle::loader::load_bundle(&examined.root)?;
        let (effective, ontology) = okf_core::bundle::settings::load_for(&examined.root)?;
        let config = super::check::lint_config(&effective.settings)?;
        let bundle = if args.fix && config.frontmatter_order.is_some() {
            let fixed = okf_core::check::lint::rules::frontmatter_order::fix(
                &bundle,
                &examined.root,
                ontology.as_ref(),
            )?;
            if json_output {
                output::print_line(
                    &json!({"kind":"fix","rule":"frontmatter-order","bundle":examined.id,"fixed":fixed}),
                )?;
            } else {
                eprintln!(
                    "--fix: normalized frontmatter order in {fixed} concepts ({})",
                    examined.id
                );
            }
            okf_core::bundle::loader::load_bundle(&examined.root)?
        } else {
            bundle
        };
        broken_link_severities.insert(examined.id.clone(), config.broken_link);
        ontology_severities.insert(examined.id.clone(), config.ontology_violation);
        let document_config = okf_core::check::lint::LintConfig {
            orphan: None,
            frontmatter_order: None,
            ..config
        };
        let mut findings = lint_bundle(&bundle, None, &document_config);
        // Ordering uses the bundle's ontology; reference validation below uses catalog identity.
        let local_graph = okf_core::graph::build::build_graph(&bundle, None);
        findings.extend(okf_core::check::lint::rules::frontmatter_order::run(
            &okf_core::check::lint::RuleContext {
                bundle: &bundle,
                graph: &local_graph,
                ontology: ontology.as_ref(),
                config: &config,
            },
        ));
        if let Some(severity) = config.orphan {
            for concept in &bundle.concepts {
                let node = Node {
                    bundle: examined.id.clone(),
                    id: concept.id.0.clone(),
                    version: examined.version.clone(),
                };
                if !connected.contains(&node) {
                    findings.push(Finding {
                        rule: "orphan".into(),
                        code: None,
                        field_path: None,
                        severity,
                        concept: Some(concept.id.0.clone()),
                        message: "orphan: no inbound or outbound links".into(),
                    });
                }
            }
        }
        if let (Some(ontology), Some(severity)) = (&ontology, config.ontology_violation) {
            let mut qualified = ontology.clone();
            for ct in qualified.concepts.values_mut() {
                for rule in ct.references.values_mut() {
                    rule.target = okf_core::ontology::schema::Target::Many(
                        rule.target
                            .types()
                            .iter()
                            .map(|target| {
                                if target.contains(':') {
                                    target.to_string()
                                } else {
                                    format!("{}:{target}", examined.id)
                                }
                            })
                            .collect(),
                    );
                }
            }
            for concept in &bundle.concepts {
                let resolver = |resource: &str| {
                    graph
                        .edges
                        .iter()
                        .find(|edge| {
                            edge.source.bundle == examined.id
                                && edge.source.id == concept.id.0
                                && edge.resource == resource
                                && edge.status == "resolved"
                        })
                        .and_then(|edge| edge.target.as_ref())
                        .and_then(|node| target_types.get(node))
                        .cloned()
                };
                for violation in okf_core::ontology::field_types::check_concept_with_budget(
                    &qualified,
                    &concept.frontmatter,
                    resolver,
                    config.finding_budget,
                ) {
                    use okf_core::ontology::field_types::ViolationKind;
                    let (code, field_path) = match &violation.kind {
                        ViolationKind::Metadata { code, path } => {
                            (Some(code.clone()), Some(path.clone()))
                        }
                        ViolationKind::WrongReferenceTarget(rule) => {
                            let edge = graph.edges.iter().find(|edge| {
                                edge.source.bundle == examined.id
                                    && edge.source.id == concept.id.0
                                    && edge.reference_rule.as_ref() == Some(rule)
                                    && violation.message.contains(&format!("{:?}", edge.resource))
                            });
                            (
                                Some("metadata-reference-target-type".into()),
                                edge.map(|e| e.location.clone()),
                            )
                        }
                        _ => (None, None),
                    };
                    if !matches!(violation.kind, ViolationKind::WrongReferenceTarget(_))
                        && !matches!(
                            code.as_deref(),
                            Some("metadata-reference-missing" | "metadata-reference-target")
                        )
                    {
                        findings.push(okf_core::check::lint::Finding {
                            rule: "ontology-violation".into(),
                            code,
                            field_path,
                            severity,
                            concept: Some(concept.id.0.clone()),
                            message: violation.message,
                        });
                    }
                }
                if let Some(ct) = concept
                    .concept_type()
                    .and_then(|name| ontology.concepts.get(name))
                {
                    for edge in graph.edges.iter().filter(|edge| {
                        edge.source.bundle == examined.id
                            && edge.source.id == concept.id.0
                            && edge.reference_rule.is_some()
                            && edge.status == "resolved"
                    }) {
                        let Some(target) = &edge.target else {
                            continue;
                        };
                        let Some(actual) = target_types.get(target).and_then(|qualified| {
                            qualified.strip_prefix(&format!("{}:", target.bundle))
                        }) else {
                            continue;
                        };
                        let Some(reference) = edge
                            .reference_rule
                            .as_ref()
                            .and_then(|key| ct.references.get(key))
                        else {
                            continue;
                        };
                        let allows = |expected: &okf_core::ontology::schema::Target| {
                            expected.types().iter().any(|expected| {
                                let (namespace, name) =
                                    expected.split_once(':').unwrap_or((&examined.id, expected));
                                namespace == target.bundle
                                    && (actual == name
                                        || target_ontologies.get(&target.bundle).is_some_and(
                                            |ontology| {
                                                okf_core::ontology::field_types::concept_is_a(
                                                    ontology, actual, name,
                                                )
                                            },
                                        ))
                            })
                        };
                        let kind_target = edge.relationship.as_ref().and_then(|relationship| {
                            ct.relationships
                                .get(&relationship.rule)
                                .and_then(|rule| {
                                    relationship
                                        .authored_kind
                                        .as_ref()
                                        .and_then(|kind| rule.kinds.get(kind))
                                })
                                .and_then(|kind| kind.target.as_ref())
                        });
                        if !allows(&reference.target)
                            || kind_target.is_some_and(|expected| !allows(expected))
                        {
                            findings.push(okf_core::check::lint::Finding{rule:"ontology-violation".into(),code:Some("metadata-reference-target-type".into()),field_path:Some(edge.location.clone()),severity,concept:Some(concept.id.0.clone()),message:format!("{}: reference {:?} targets {}:{actual}, outside its declared target namespace/types",edge.location,edge.resource,target.bundle)});
                        }
                    }
                }
                let selected = findings
                    .iter()
                    .enumerate()
                    .filter(|(_, finding)| {
                        finding.rule == "ontology-violation"
                            && finding.concept.as_ref() == Some(&concept.id.0)
                    })
                    .map(|(index, _)| index)
                    .collect::<Vec<_>>();
                if selected.len() > config.finding_budget {
                    let remove = selected
                        .into_iter()
                        .skip(config.finding_budget)
                        .collect::<std::collections::BTreeSet<_>>();
                    let mut index = 0usize;
                    findings.retain(|_| {
                        let keep = !remove.contains(&index);
                        index += 1;
                        keep
                    });
                    if !findings.iter().any(|finding| {
                        finding.concept.as_ref() == Some(&concept.id.0)
                            && finding.code.as_deref() == Some("incomplete-check")
                    }) {
                        findings.push(okf_core::check::lint::Finding {
                            rule: "ontology-violation".into(),
                            code: Some("incomplete-check".into()),
                            field_path: None,
                            severity,
                            concept: Some(concept.id.0.clone()),
                            message: "ontology finding budget exhausted; check incomplete".into(),
                        });
                    }
                }
            }
        }
        // The qualified resolver owns all broken-reference diagnostics; the single-root
        // graph would normalize above-root references into unrelated local IDs.
        findings.retain(|f| {
            f.rule != "broken-link" && f.code.as_deref() != Some("metadata-reference-missing")
        });
        failed |= meets_threshold(&findings, fail_on);
        for finding in findings {
            if json_output {
                output::print_line(
                    &json!({"kind":"finding","rule":finding.rule,"code":finding.code,"field_path":finding.field_path,"severity":finding.severity.as_str(),"concept":finding.concept,"bundle":examined.id,"message":finding.message}),
                )?;
            } else {
                output::print_text_line(format_args!(
                    "{}\t{}\t{}:{}\t{}",
                    finding.severity.as_str(),
                    finding.rule,
                    examined.id,
                    finding.concept.as_deref().unwrap_or("-"),
                    finding.message
                ))?;
            }
        }
        let Some(index_severity) = super::check::lint_severity(
            effective.settings.lint.index_coverage.as_deref(),
            Severity::Warn,
        )?
        else {
            continue;
        };
        for finding in okf_core::check::lint::rules::index_coverage::check_indexes(
            &bundle,
            &effective.settings.lint.index_exclude,
        )? {
            failed |= fail_on
                .min_severity()
                .is_some_and(|threshold| index_severity >= threshold);
            if json_output {
                let mut value = serde_json::to_value(&finding)
                    .map_err(|e| OkfError::Internal(e.to_string()))?;
                value["kind"] = json!("finding");
                value["rule"] = json!("index-coverage");
                value["severity"] = json!(index_severity.as_str());
                value["bundle"] = json!(examined.id);
                output::print_line(&value)?;
            } else {
                output::print_text_line(format_args!(
                    "{}\tindex-coverage\t{}:{}\t{}",
                    index_severity.as_str(),
                    examined.id,
                    finding.directory,
                    finding.message
                ))?;
            }
        }
    }
    let threshold = args.fail_on.as_deref().unwrap_or("error");
    for edge in &graph.edges {
        let issue = if !matches!(edge.status.as_str(), "resolved" | "out-of-scope") {
            Some(edge.status.as_str())
        } else if edge
            .snapshot
            .as_ref()
            .is_some_and(|s| s.status != "matched")
        {
            Some("snapshot-expectation")
        } else if edge.fingerprint_status.as_deref() == Some("changed") {
            Some("changed-source-fingerprint")
        } else {
            None
        };
        if let Some(issue) = issue {
            let local_missing = edge.status == "missing-target"
                && (edge
                    .target
                    .as_ref()
                    .is_some_and(|target| target.bundle == edge.source.bundle)
                    || edge.resource_bundle.as_deref() == Some(edge.source.bundle.as_str()));
            let (rule, code, severity) = if local_missing && edge.reference_rule.is_some() {
                let Some(Some(configured)) = ontology_severities.get(&edge.source.bundle) else {
                    continue;
                };
                (
                    "ontology-violation",
                    "metadata-reference-missing",
                    configured.as_str(),
                )
            } else if local_missing {
                let Some(Some(configured)) = broken_link_severities.get(&edge.source.bundle) else {
                    continue;
                };
                ("broken-link", issue, configured.as_str())
            } else {
                ("cross-bundle-reference", issue, "warn")
            };
            if json_output {
                output::print_line(
                    &json!({"kind":"finding","rule":rule,"code":code,"severity":severity,"concept":edge.source.id,"bundle":edge.source.bundle,"field_path":edge.location,"message":edge.evidence,"resolution":edge}),
                )?;
            } else {
                output::print_text_line(format_args!(
                    "{severity}\t{code}\t{}\t{}",
                    edge.source.key(),
                    edge.resource
                ))?;
            }
            let severity = match severity {
                "error" => Severity::Error,
                "warn" => Severity::Warn,
                _ => Severity::Info,
            };
            failed |= fail_on
                .min_severity()
                .is_some_and(|minimum| severity >= minimum);
        }
    }
    for unavailable in &graph.scope.unavailable {
        if json_output {
            output::print_line(
                &json!({"kind":"finding","rule":"unavailable-bundle","severity":"warn","bundle":unavailable,"message":"requested scope member unavailable"}),
            )?;
        } else {
            output::print_text_line(format_args!("warn\tunavailable-bundle\t{unavailable}"))?;
        }
        failed |= matches!(threshold, "info" | "warn" | "any");
    }
    Ok(if failed { 1 } else { 0 })
}
