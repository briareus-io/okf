//! Evaluate fixed manifest read chains against one bounded graph and its parsed documents.
use super::query::{cli_filter, field_matches, parse_field_filters, print_cells};
use crate::{
    cli::{SavedQueryArgs, SearchSortArg},
    output,
};
use okf_core::{
    bundle::loader::{Bundle, load_concept},
    model::concept::{Concept, ConceptId},
};
use okf_core::{
    bundle::{
        context::Context,
        settings::{BundleSettings, SavedQuery, SavedStart, SavedStep},
    },
    error::{OkfError, Result},
    graph::{
        catalog::{
            CatalogGraph, DocumentExamination, Edge, EdgeStatus, ExaminedBundle, LocalOutgoing,
            Node, ScopedBundles,
        },
        filter::{CompiledFilters, MatchOutcome},
    },
    query::{
        metadata::{Condition, project, validate_projection_field},
        search::{SearchFilter, search_detailed},
    },
};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

fn matching(
    bundles: &ScopedBundles,
    filter: &SearchFilter,
    field: &[String],
    facet: &[String],
    candidates: Option<&BTreeSet<Node>>,
) -> Result<BTreeMap<Node, u32>> {
    let literals = parse_field_filters(field)?;
    let conditions = facet
        .iter()
        .map(|s| Condition::parse(s))
        .collect::<Result<Vec<_>>>()?;
    let mut matches = BTreeMap::new();
    for (id, (bundle, version)) in bundles {
        for hit in search_detailed(bundle, filter) {
            let c = hit.concept;
            let node = Node {
                bundle: id.clone(),
                version: version.clone(),
                id: c.id.0.clone(),
            };
            if candidates.is_some_and(|set| !set.contains(&node))
                || !literals.iter().all(|(k, v)| field_matches(c, k, v))
            {
                continue;
            }
            let mut accepted = true;
            for condition in &conditions {
                if !condition.matches(c)? {
                    accepted = false;
                    break;
                }
            }
            if accepted {
                matches.insert(node, hit.score);
            }
        }
    }
    Ok(matches)
}

pub fn run(
    args: &SavedQueryArgs,
    settings: &BundleSettings,
    query: &SavedQuery,
    context: &Context,
    graph: &CatalogGraph,
    bundles: &ScopedBundles,
    json_output: bool,
) -> Result<i32> {
    let concept = |node: &Node| {
        bundles
            .get(&node.bundle)
            .filter(|(_, v)| v == &node.version)
            .and_then(|(b, _)| b.get(&node.id))
    };
    let examination = graph
        .examination
        .as_ref()
        .expect("bounded graph provides examination");
    let mut stages = Vec::new();
    let mut nodes: BTreeSet<Node> = match (&query.from, &args.concept) {
        (SavedStart::Concept(_), Some(id)) => {
            let parsed = okf_core::model::concept::ConceptId::parse(id)?;
            let node = graph.target(context, &parsed.0);
            if concept(&node).is_none() {
                return Err(OkfError::Usage(format!(
                    "starting concept unavailable: {id}"
                )));
            }
            BTreeSet::from([node])
        }
        (SavedStart::List { list: start } | SavedStart::Search { search: start }, None) => {
            let preset = settings.query_presets[&start.preset]
                .query
                .clone()
                .unwrap_or_default();
            let filter = SearchFilter {
                type_: preset.type_,
                tag: preset.tag,
                text: preset.text,
                ..Default::default()
            };
            matching(bundles, &filter, &preset.field, &preset.facet_filter, None)?
                .into_keys()
                .collect()
        }
        _ => {
            return Err(OkfError::Usage(
                "supply one starting concept for from: concept, and none for a preset start".into(),
            ));
        }
    };
    let mut unresolved = 0;
    for step in &query.steps {
        let incoming = matches!(step, SavedStep::Backlinks { .. });
        let filter = step.filters().compile()?;
        let mut next = BTreeSet::new();
        for edge in &graph.edges {
            // A provenance snapshot may name a different version than the current candidate.
            let target = match &edge.snapshot {
                Some(snapshot) => snapshot.resolved.as_ref(),
                None => edge.target.as_ref(),
            };
            let (anchor, end) = if incoming {
                (target, Some(&edge.source))
            } else {
                (Some(&edge.source), target)
            };
            if !anchor.is_some_and(|a| nodes.contains(a)) {
                continue;
            }
            let endpoint = end.and_then(concept);
            accept_edge(&filter, edge, end, endpoint, &mut next, &mut unresolved)?;
        }
        stages.push(json!({"operation":if incoming{"backlinks"}else{"links"},"inputs":nodes.len(),"outputs":next.len()}));
        nodes = next;
    }
    finish(
        args,
        query,
        bundles,
        &Traversal {
            nodes,
            stages,
            unresolved,
            examination: examination.clone(),
            strategy: "scope",
        },
        json_output,
    )
}

struct Traversal {
    nodes: BTreeSet<Node>,
    stages: Vec<Value>,
    examination: DocumentExamination,
    strategy: &'static str,
    unresolved: usize,
}

fn accept_edge(
    filter: &CompiledFilters,
    edge: &Edge,
    end: Option<&Node>,
    endpoint: Option<&Concept>,
    next: &mut BTreeSet<Node>,
    unresolved: &mut usize,
) -> Result<()> {
    match filter.matches(edge.relationship.as_ref(), endpoint)? {
        MatchOutcome::Excluded => {}
        MatchOutcome::Matched
            if endpoint.is_some()
                && matches!(
                    edge.status,
                    EdgeStatus::Resolved | EdgeStatus::UncheckedEquivalence
                ) =>
        {
            next.insert(end.expect("loaded endpoint has identity").clone());
        }
        _ if edge.resource_kind.is_none() => *unresolved += 1,
        _ => {}
    }
    Ok(())
}

pub(super) fn validate(args: &SavedQueryArgs) -> Result<()> {
    if args.offset.is_some() && args.limit.is_none_or(|n| n == 0) {
        return Err(OkfError::Usage(
            "--offset requires a positive --limit".into(),
        ));
    }
    for field in &args.project {
        validate_projection_field(field)?;
    }
    if args.scan_limit == Some(0) {
        return Err(OkfError::Usage("scan limit must be positive".into()));
    }
    Ok(())
}

/// Keep positive and negative lookups per invocation. Once the budget is reached, remember
/// distinct deferred endpoints so repeated occurrences cannot inflate examination evidence.
struct Documents<'a> {
    root: &'a std::path::Path,
    loaded: BTreeMap<String, Concept>,
    missing: BTreeSet<String>,
    deferred: BTreeSet<String>,
    limit: Option<usize>,
}
impl Documents<'_> {
    fn read(&mut self, id: &str) -> Result<Option<&Concept>> {
        if !self.loaded.contains_key(id) && !self.missing.contains(id) {
            let filename = format!("{id}.md");
            if okf_core::bundle::walk::is_excluded_directory_path(std::path::Path::new(
                filename.trim_start_matches('/'),
            )) {
                self.missing.insert(id.to_owned());
                return Ok(None);
            }
            if self.limit.is_some_and(|max| self.loaded.len() >= max) {
                self.deferred.insert(id.to_owned());
                return Ok(None);
            }
            match load_concept(self.root, &filename)? {
                Some(c) => {
                    self.loaded.insert(id.to_owned(), c);
                }
                None => {
                    self.missing.insert(id.to_owned());
                }
            }
        }
        Ok(self.loaded.get(id))
    }
}

pub(super) fn local_outgoing(
    args: &SavedQueryArgs,
    query: &SavedQuery,
    context: &Context,
    primary: &ExaminedBundle,
    scope: &okf_core::graph::catalog::Scope,
    limit: Option<usize>,
    json_output: bool,
) -> Result<Option<i32>> {
    let (SavedStart::Concept(_), Some(raw)) = (&query.from, &args.concept) else {
        return Ok(None);
    };
    if query
        .steps
        .iter()
        .any(|s| matches!(s, SavedStep::Backlinks { .. }))
    {
        return Ok(None);
    }
    let start = Node {
        bundle: primary.id.clone(),
        version: primary.version.clone(),
        id: ConceptId::parse(raw)?.0,
    };
    let mut docs = Documents {
        root: &primary.root,
        loaded: Default::default(),
        missing: Default::default(),
        deferred: Default::default(),
        limit,
    };
    if docs.read(&start.id)?.is_none() {
        return Err(OkfError::Usage(format!(
            "starting concept unavailable: {raw}"
        )));
    }
    let reader = LocalOutgoing::new(context, primary)?;
    let mut nodes = BTreeSet::from([start]);
    let mut unresolved = 0;
    let mut stages = Vec::new();
    for step in &query.steps {
        let filter = step.filters().compile()?;
        let mut next = BTreeSet::new();
        for node in &nodes {
            for mut edge in reader.edges(node, &docs.loaded[&node.id])? {
                // Snapshot evidence (including other versions) belongs to the scoped executor.
                if edge.snapshot.is_some() {
                    return Ok(None);
                }
                if matches!(
                    filter.matches(edge.relationship.as_ref(), None)?,
                    MatchOutcome::Excluded
                ) {
                    continue;
                }
                let end = edge.target.as_ref();
                let mut endpoint = None;
                if let Some(target) = end
                    && target.bundle == primary.id
                    && edge.status == EdgeStatus::MissingTarget
                {
                    endpoint = docs.read(&target.id)?;
                    if endpoint.is_some() {
                        edge.status = EdgeStatus::Resolved;
                    }
                }
                accept_edge(&filter, &edge, end, endpoint, &mut next, &mut unresolved)?;
            }
        }
        stages.push(json!({"operation":"links","inputs":nodes.len(),"outputs":next.len()}));
        nodes = next;
    }
    let examination = DocumentExamination {
        examined_documents: docs.loaded.len(),
        eligible_documents: docs.loaded.len() + docs.deferred.len(),
        scan_complete: docs.deferred.is_empty(),
    };
    let traversal = Traversal {
        nodes,
        stages,
        unresolved,
        examination,
        strategy: "outgoing",
    };
    let bundle = Bundle {
        root: primary.root.clone(),
        concepts: docs.loaded.into_values().collect(),
    };
    let bundles = BTreeMap::from([(primary.id.clone(), (bundle, primary.version.clone()))]);
    super::catalog::print_scope(scope, json_output)?;
    finish(args, query, &bundles, &traversal, json_output).map(Some)
}
fn finish(
    args: &SavedQueryArgs,
    query: &SavedQuery,
    bundles: &ScopedBundles,
    traversal: &Traversal,
    json_output: bool,
) -> Result<i32> {
    let Traversal {
        nodes,
        stages,
        unresolved,
        examination,
        strategy,
    } = traversal;
    let unresolved = *unresolved;
    let filter = cli_filter(
        &args.type_,
        &args.tag,
        &args.text,
        args.match_mode,
        &args.in_,
        args.sort,
    );
    let mut hits: Vec<_> = matching(
        bundles,
        &filter,
        &args.field,
        &args.facet_filter,
        Some(nodes),
    )?
    .into_iter()
    .collect();
    if matches!(args.sort, SearchSortArg::Relevance) {
        hits.sort_by_key(|(_, score)| std::cmp::Reverse(*score));
    }
    let total = hits.len();
    let projection = if args.project.is_empty() {
        &query.select
    } else {
        &args.project
    };
    let mut delivered = 0;
    let mut problems = 0;
    for (node, _) in hits
        .into_iter()
        .skip(args.offset.unwrap_or(0))
        .take(args.limit.unwrap_or(usize::MAX))
    {
        let c = bundles[&node.bundle]
            .0
            .get(&node.id)
            .expect("matched concept belongs to loaded scope");
        if !json_output {
            output::print_text_line(format_args!("{}", node.key()))?;
        }
        let mut record = if let Some(path) = &args.section {
            match output::section_record(c, &path.0) {
                Ok(section) => {
                    output::mark_partial("content", "section-only");
                    section
                }
                Err(OkfError::Usage(message)) => {
                    problems += 1;
                    if !json_output {
                        eprintln!("{message}");
                    }
                    json!({"kind":"warning","id":node.id,"path":path.0,"message":message})
                }
                Err(e) => return Err(e),
            }
        } else if args.body {
            output::mark_content_complete();
            json!({"kind":"body","id":node.id,"body":c.body})
        } else if args.outline {
            output::outline_record(c)?
        } else if !projection.is_empty() {
            project(c, projection, Some(&node.bundle), &node.version)?
        } else {
            okf_core::output::record::concept_record(c)
        };
        if args.section.is_some() || args.outline || args.body || !projection.is_empty() {
            record["bundle"] = json!(node.bundle);
            record["version"] = json!(node.version);
        } else if json_output {
            output::print_line(
                &json!({"kind":"concept-identity","schema_version":1,"bundle":node.bundle,"version":node.version,"id":node.id}),
            )?;
        }
        if json_output {
            output::print_line(&record)?;
        } else if args.body {
            output::print_text_line(format_args!("{}", c.body))?;
        } else if args.outline {
            output::print_concept_outline(c, false)?;
        } else if args.section.is_some() {
            if record["kind"] == "section" {
                output::print_section(&record, false)?;
            }
        } else if !projection.is_empty() {
            print_cells(c, projection, Some(&node.bundle), &node.version)?;
        } else {
            output::print_compact_concept(c, false)?;
        }
        delivered += 1;
    }
    if !examination.scan_complete {
        output::mark_partial("examination", "scan-incomplete");
    }
    if unresolved > 0 {
        output::mark_partial("matching", "unresolved-endpoint");
    }
    if delivered < total {
        output::mark_partial("delivery", "pagination");
    }
    if problems > 0 {
        output::mark_partial("content", "section-unavailable");
        output::mark_partial("delivery", "section-unavailable");
    }
    let complete = examination.scan_complete && unresolved == 0;
    if json_output {
        output::print_line(
            &json!({"kind":"saved-query-summary","name":args.name,"strategy":strategy,"stages":stages,"examination":examination,"unresolved":unresolved,"matching_complete":complete,"observed_matches":total,"delivery_complete":delivered==total&&problems==0,"section_errors":problems}),
        )?;
    } else {
        output::print_text_line(format_args!(
            "matched {total} concepts; matching complete: {complete}; section errors: {problems}"
        ))?;
    }
    Ok(0)
}
