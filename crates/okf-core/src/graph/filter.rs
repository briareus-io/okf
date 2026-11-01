//! Shared relationship predicates. Filtering is per occurrence, before adjacency collapse.
//! These filters are for explicit queries, never mutation guards or integrity checks.
use crate::{
    bundle::loader::Bundle,
    error::{OkfError, Result},
    graph::{
        build::LinkGraph,
        relationships::{relationships, Relationship},
    },
    model::{
        concept::{Concept, ConceptId},
        frontmatter::Frontmatter,
        link::outbound_links,
    },
    ontology::schema::Ontology,
    query::metadata::Condition,
};
use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct EdgeFilters {
    pub kind: Vec<String>,
    pub inverse_kind: Vec<String>,
    pub related_type: Vec<String>,
    pub rule: Vec<String>,
    #[serde(alias = "edge_filter")]
    pub edge: Vec<String>,
    #[serde(alias = "related_filter")]
    pub related: Vec<String>,
}
impl EdgeFilters {
    pub fn is_empty(&self) -> bool {
        self.kind.is_empty()
            && self.inverse_kind.is_empty()
            && self.related_type.is_empty()
            && self.rule.is_empty()
            && self.edge.is_empty()
            && self.related.is_empty()
    }
    pub fn needs_relationship(&self) -> bool {
        !self.kind.is_empty()
            || !self.inverse_kind.is_empty()
            || !self.rule.is_empty()
            || !self.edge.is_empty()
    }
    pub fn needs_related(&self) -> bool {
        !self.related_type.is_empty() || !self.related.is_empty()
    }
    /// Replace explicitly supplied categories, then clear inherited categories. Supplying and
    /// clearing one category is rejected. Empty explicit vectors mean an omitted CLI option.
    pub fn overlay(&self, explicit: &Self, clear: &[String]) -> Result<Self> {
        let mut result = self.clone();
        for (name, input, output) in [
            ("kind", &explicit.kind, &mut result.kind),
            (
                "inverse-kind",
                &explicit.inverse_kind,
                &mut result.inverse_kind,
            ),
            (
                "related-type",
                &explicit.related_type,
                &mut result.related_type,
            ),
            ("rule", &explicit.rule, &mut result.rule),
            ("edge", &explicit.edge, &mut result.edge),
            ("related", &explicit.related, &mut result.related),
        ] {
            if clear.iter().any(|s| s == name) {
                if !input.is_empty() {
                    return Err(OkfError::Usage(format!(
                        "cannot supply and clear filter category {name}"
                    )));
                }
                output.clear();
            } else if !input.is_empty() {
                *output = input.clone();
            }
        }
        for name in clear {
            if !matches!(
                name.as_str(),
                "kind" | "inverse-kind" | "related-type" | "rule" | "edge" | "related"
            ) {
                return Err(OkfError::Usage(format!("unknown filter category {name:?}")));
            }
        }
        result.compile()?;
        Ok(result)
    }
    pub fn compile(&self) -> Result<CompiledFilters> {
        for value in self
            .kind
            .iter()
            .chain(&self.inverse_kind)
            .chain(&self.related_type)
            .chain(&self.rule)
        {
            if value.trim().is_empty() {
                return Err(OkfError::Usage(
                    "categorical relationship filters cannot be empty".into(),
                ));
            }
        }
        Ok(CompiledFilters {
            filters: self.clone(),
            edge: self
                .edge
                .iter()
                .map(|s| Condition::parse(s))
                .collect::<Result<_>>()?,
            related: self
                .related
                .iter()
                .map(|s| Condition::parse(s))
                .collect::<Result<_>>()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct CompiledFilters {
    pub filters: EdgeFilters,
    edge: Vec<Condition>,
    related: Vec<Condition>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum MatchOutcome {
    Matched,
    Excluded,
    Unresolved,
}
fn category(wanted: &[String], actual: Option<&str>) -> bool {
    wanted.is_empty() || actual.is_some_and(|v| wanted.iter().any(|w| w == v))
}
impl CompiledFilters {
    /// `related` is the target for outgoing inspection and the source for incoming inspection.
    /// None means unavailable/out-of-scope/missing. A known predicate failure is excluded even
    /// if some other metadata is unavailable; otherwise endpoint uncertainty stays explicit.
    pub fn matches(
        &self,
        edge: Option<&Relationship>,
        related: Option<&Concept>,
    ) -> Result<MatchOutcome> {
        if self.filters.needs_relationship() && edge.is_none() {
            return Ok(MatchOutcome::Excluded);
        }
        if let Some(edge) = edge {
            if (!self.filters.kind.is_empty() || !self.filters.inverse_kind.is_empty())
                && edge.status == "unknown-kind"
            {
                return Ok(MatchOutcome::Excluded);
            }
            if !category(&self.filters.kind, edge.authored_kind.as_deref())
                || !category(&self.filters.inverse_kind, edge.inverse.as_deref())
                || !category(&self.filters.rule, Some(&edge.rule))
            {
                return Ok(MatchOutcome::Excluded);
            }
            if !self.edge.is_empty() {
                // Attributes belong to this exact occurrence, never a union of parallel edges.
                let attributes = Concept {
                    id: ConceptId::from_relative("_edge"),
                    frontmatter: Frontmatter::from_map(edge.attributes.clone()),
                    body: String::new(),
                };
                for condition in &self.edge {
                    if !condition.matches(&attributes)? {
                        return Ok(MatchOutcome::Excluded);
                    }
                }
            }
        }
        if self.filters.needs_related() {
            let Some(related) = related else {
                return Ok(MatchOutcome::Unresolved);
            };
            if !category(&self.filters.related_type, related.concept_type()) {
                return Ok(MatchOutcome::Excluded);
            }
            for condition in &self.related {
                if !condition.matches(related)? {
                    return Ok(MatchOutcome::Excluded);
                }
            }
        }
        Ok(MatchOutcome::Matched)
    }
}

/// Inventory all eligible IDs but parse only the requested document budget. A named outgoing
/// source is first so an unrelated malformed document cannot precede the explicitly selected
/// concept. Inventory is not document examination.
pub fn load_bundle_limited(
    root: &std::path::Path,
    limit: Option<usize>,
    priority: Option<&str>,
) -> Result<(Bundle, usize)> {
    let mut paths = crate::bundle::walk::walk_markdown(root)?
        .into_iter()
        .filter(|p| {
            !matches!(
                p.file_name().and_then(|s| s.to_str()),
                Some("index.md" | "log.md")
            )
        })
        .collect::<Vec<_>>();
    let total = paths.len();
    if let Some(priority) = priority {
        let id = ConceptId::parse(priority)?;
        let path = std::path::PathBuf::from(format!("{}.md", id.0.trim_start_matches('/')));
        let at = paths
            .iter()
            .position(|p| p == &path)
            .ok_or_else(|| OkfError::Usage(format!("concept not found: {}", id.0)))?;
        let path = paths.remove(at);
        paths.insert(0, path);
    }
    let mut concepts = Vec::new();
    let mut failures = Vec::new();
    for path in paths.into_iter().take(limit.unwrap_or(usize::MAX)) {
        let id = ConceptId::from_relative(&path.with_extension("").to_string_lossy());
        let file = root.join(path);
        let text = std::fs::read_to_string(&file).map_err(|e| OkfError::from(e).at_path(&file))?;
        match crate::parse::parse_concept_file(id, &text, &file) {
            Ok(c) => concepts.push(c),
            Err(OkfError::Yaml(message)) => failures.push(message),
            Err(e) => return Err(e),
        }
    }
    if !failures.is_empty() {
        return Err(OkfError::yaml_files(failures));
    }
    Ok((
        Bundle {
            root: root.to_path_buf(),
            concepts,
        },
        total,
    ))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Outgoing,
    Incoming,
}
#[derive(Debug, Clone, Default)]
pub struct QueryOptions {
    pub scan_limit: Option<usize>,
    pub limit: Option<usize>,
    pub offset: usize,
    pub details: bool,
}
#[derive(Debug, Clone, Serialize)]
pub struct FilteredOccurrence {
    pub source: String,
    pub target: String,
    pub related: String,
    pub exists: bool,
    pub relationship: Option<Relationship>,
}
#[derive(Debug, Clone, Default, Serialize)]
pub struct FilterEvidence {
    pub examined_documents: usize,
    pub examined_occurrences: usize,
    pub matched_occurrences: usize,
    pub matched_concepts: usize,
    pub unresolved_candidates: usize,
    pub scan_complete: bool,
    pub delivery_complete: bool,
}
#[derive(Debug, Clone, Serialize)]
pub struct FilteredQuery {
    pub occurrences: Vec<FilteredOccurrence>,
    pub evidence: FilterEvidence,
}

/// Explicit local filtered query, preserving occurrence identity until after predicates.
/// An unfiltered compact caller should retain its legacy adjacency path.
pub fn query_local(
    bundle: &Bundle,
    ontology: Option<&Ontology>,
    anchor: &str,
    direction: Direction,
    filters: &CompiledFilters,
    options: &QueryOptions,
) -> Result<FilteredQuery> {
    if options.scan_limit == Some(0)
        || options.limit == Some(0)
        || (options.offset > 0 && options.limit.is_none())
    {
        return Err(OkfError::Usage(
            "query limits must be positive; offset requires limit".into(),
        ));
    }
    let anchor = ConceptId::from_relative(anchor).0;
    let concepts: IndexMap<&str, &Concept> = bundle
        .concepts
        .iter()
        .map(|c| (c.id.0.as_str(), c))
        .collect();
    let mut evidence = FilterEvidence {
        scan_complete: true,
        delivery_complete: true,
        ..Default::default()
    };
    let mut matches = Vec::new();
    for concept in &bundle.concepts {
        if direction == Direction::Outgoing && concept.id.0 != anchor {
            continue;
        }
        if options
            .scan_limit
            .is_some_and(|n| evidence.examined_documents >= n)
        {
            evidence.scan_complete = false;
            break;
        }
        evidence.examined_documents += 1;
        let semantic = ontology
            .map(|o| relationships(concept, o))
            .unwrap_or_default();
        let semantic_targets: IndexSet<&str> = semantic.iter().map(|r| r.target.as_str()).collect();
        let ordinary: Vec<_> = outbound_links(concept, ontology)
            .into_iter()
            .filter(|target| !semantic_targets.contains(target.0.as_str()))
            .collect();
        for (target, edge) in semantic
            .iter()
            .map(|r| (r.target.as_str(), Some(r)))
            .chain(ordinary.iter().map(|t| (t.0.as_str(), None)))
        {
            if direction == Direction::Incoming && target != anchor {
                continue;
            }
            evidence.examined_occurrences += 1;
            let related_id = if direction == Direction::Incoming {
                concept.id.0.as_str()
            } else {
                target
            };
            let related = concepts.get(related_id).copied();
            match filters.matches(edge, related)? {
                MatchOutcome::Excluded => {}
                MatchOutcome::Unresolved => evidence.unresolved_candidates += 1,
                MatchOutcome::Matched => matches.push(FilteredOccurrence {
                    source: concept.id.0.clone(),
                    target: target.into(),
                    related: related_id.into(),
                    exists: related.is_some(),
                    relationship: edge.cloned(),
                }),
            }
        }
    }
    matches.sort_by(|a, b| {
        (
            &a.related,
            &a.source,
            &a.target,
            a.relationship.as_ref().map(|r| (&r.rule, &r.field_path)),
        )
            .cmp(&(
                &b.related,
                &b.source,
                &b.target,
                b.relationship.as_ref().map(|r| (&r.rule, &r.field_path)),
            ))
    });
    evidence.matched_occurrences = matches.len();
    evidence.matched_concepts = matches
        .iter()
        .map(|r| &r.related)
        .collect::<IndexSet<_>>()
        .len();
    if !options.details {
        let mut seen = IndexSet::new();
        matches.retain(|r| seen.insert(r.related.clone()));
    }
    let total = matches.len();
    let end = options
        .limit
        .map(|n| options.offset.saturating_add(n))
        .unwrap_or(total)
        .min(total);
    evidence.delivery_complete = options.offset == 0 && end == total;
    let occurrences = matches
        .into_iter()
        .skip(options.offset)
        .take(options.limit.unwrap_or(usize::MAX))
        .collect();
    Ok(FilteredQuery {
        occurrences,
        evidence,
    })
}

/// Build traversal adjacency from matching occurrences, not from a rendered edge subset.
/// `direction` chooses which endpoint metadata is considered related during evaluation.
/// Callers retain the requested root even if its filtered adjacency is empty.
pub fn filtered_graph(
    bundle: &Bundle,
    ontology: Option<&Ontology>,
    filters: &CompiledFilters,
    direction: Direction,
) -> Result<(LinkGraph, FilterEvidence)> {
    let mut graph = LinkGraph::default();
    let mut evidence = FilterEvidence {
        scan_complete: true,
        delivery_complete: true,
        ..Default::default()
    };
    for concept in &bundle.concepts {
        graph.existing.insert(concept.id.0.clone());
    }
    let concepts: IndexMap<&str, &Concept> = bundle
        .concepts
        .iter()
        .map(|c| (c.id.0.as_str(), c))
        .collect();
    for concept in &bundle.concepts {
        evidence.examined_documents += 1;
        let semantic = ontology
            .map(|o| relationships(concept, o))
            .unwrap_or_default();
        let targets: IndexSet<&str> = semantic.iter().map(|r| r.target.as_str()).collect();
        let ordinary: Vec<_> = outbound_links(concept, ontology)
            .into_iter()
            .filter(|t| !targets.contains(t.0.as_str()))
            .collect();
        for (target, edge) in semantic
            .iter()
            .filter(|r| r.status != "external")
            .map(|r| (r.target.as_str(), Some(r)))
            .chain(ordinary.iter().map(|t| (t.0.as_str(), None)))
        {
            evidence.examined_occurrences += 1;
            let related = if direction == Direction::Incoming {
                Some(concept)
            } else {
                concepts.get(target).copied()
            };
            match filters.matches(edge, related)? {
                MatchOutcome::Excluded => {}
                MatchOutcome::Unresolved => evidence.unresolved_candidates += 1,
                MatchOutcome::Matched => {
                    evidence.matched_occurrences += 1;
                    graph
                        .forward
                        .entry(concept.id.0.clone())
                        .or_default()
                        .push(ConceptId(target.into()));
                    graph
                        .reverse
                        .entry(target.into())
                        .or_default()
                        .push(concept.id.clone());
                }
            }
        }
    }
    for values in graph.forward.values_mut().chain(graph.reverse.values_mut()) {
        values.sort_by(|a, b| a.0.cmp(&b.0));
        values.dedup();
    }
    graph.forward.sort_keys();
    graph.reverse.sort_keys();
    evidence.matched_concepts = graph
        .forward
        .iter()
        .flat_map(|(s, targets)| {
            std::iter::once(s.as_str()).chain(targets.iter().map(|t| t.0.as_str()))
        })
        .collect::<IndexSet<_>>()
        .len();
    Ok((graph, evidence))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraversalDirection {
    Outgoing,
    Incoming,
    Both,
}

/// Traverse from an anchor, applying predicates relative to the current node before enqueuing
/// a neighbor. For `Both`, an occurrence can match from one endpoint but not the other.
/// Returned adjacency contains only accepted traversal edges and can be rendered directly.
pub fn rooted_filtered_graph(
    bundle: &Bundle,
    ontology: Option<&Ontology>,
    filters: &CompiledFilters,
    anchor: &str,
    direction: TraversalDirection,
    depth: Option<usize>,
) -> Result<(LinkGraph, FilterEvidence)> {
    let anchor = ConceptId::from_relative(anchor).0;
    let mut graph = LinkGraph::default();
    if bundle.concepts.iter().any(|c| c.id.0 == anchor) {
        graph.existing.insert(anchor.clone());
    }
    graph.forward.insert(anchor.clone(), Vec::new());
    let mut evidence = FilterEvidence {
        scan_complete: true,
        delivery_complete: true,
        ..Default::default()
    };
    let mut visited = IndexSet::new();
    visited.insert(anchor.clone());
    let mut queue = std::collections::VecDeque::from([(anchor, 0usize)]);
    let mut edges = IndexSet::new();
    while let Some((current, level)) = queue.pop_front() {
        if depth.is_some_and(|max| level >= max) {
            continue;
        }
        let directions: &[Direction] = match direction {
            TraversalDirection::Outgoing => &[Direction::Outgoing],
            TraversalDirection::Incoming => &[Direction::Incoming],
            TraversalDirection::Both => &[Direction::Outgoing, Direction::Incoming],
        };
        for query_direction in directions {
            let query = query_local(
                bundle,
                ontology,
                &current,
                *query_direction,
                filters,
                &QueryOptions {
                    details: true,
                    ..Default::default()
                },
            )?;
            evidence.examined_documents += query.evidence.examined_documents;
            evidence.examined_occurrences += query.evidence.examined_occurrences;
            evidence.unresolved_candidates += query.evidence.unresolved_candidates;
            for occurrence in query.occurrences {
                if occurrence
                    .relationship
                    .as_ref()
                    .is_some_and(|r| r.status == "external")
                {
                    continue;
                }
                let key = (
                    occurrence.source.clone(),
                    occurrence.target.clone(),
                    occurrence
                        .relationship
                        .as_ref()
                        .map(|r| (r.rule.clone(), r.field_path.clone())),
                );
                if edges.insert(key) {
                    evidence.matched_occurrences += 1;
                    graph
                        .forward
                        .entry(occurrence.source.clone())
                        .or_default()
                        .push(ConceptId(occurrence.target.clone()));
                    graph
                        .reverse
                        .entry(occurrence.target.clone())
                        .or_default()
                        .push(ConceptId(occurrence.source.clone()));
                }
                if occurrence.exists {
                    graph.existing.insert(occurrence.related.clone());
                }
                if visited.insert(occurrence.related.clone()) {
                    queue.push_back((occurrence.related, level + 1));
                }
            }
        }
    }
    for values in graph.forward.values_mut().chain(graph.reverse.values_mut()) {
        values.sort_by(|a, b| a.0.cmp(&b.0));
        values.dedup();
    }
    graph.forward.sort_keys();
    graph.reverse.sort_keys();
    evidence.matched_concepts = visited.len().saturating_sub(1);
    Ok((graph, evidence))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_yaml::Value;
    fn concept(id: &str, yaml: &str, body: &str) -> Concept {
        Concept {
            id: ConceptId::from_relative(id),
            frontmatter: Frontmatter::from_map(serde_yaml::from_str(yaml).unwrap()),
            body: body.into(),
        }
    }
    fn fixture() -> (Bundle, Ontology) {
        let ontology = crate::ontology::load::parse_ontology(
            r#"
okf_ontology: '0.1'
concepts:
  Defect:
    references:
      targets: {target: Feature, cardinality: 0..n, selector: 'relations[].target'}
    relationships:
      blockers:
        reference: targets
        kind: blocks
        inverse: is-blocked-by
        attributes:
          state: 'relations[].state'
          severity: 'relations[].severity'
  Feature: {}
"#,
        )
        .unwrap();
        let bundle = Bundle { root: ".".into(), concepts: vec![
            concept("defects/a", "type: Defect\nstatus: stable\nrelations:\n- target: /features/a\n  state: open\n  severity: low\n- target: /features/a\n  state: closed\n  severity: high\n- target: /features/missing\n  state: open\n  severity: high\n", ""),
            concept("defects/b", "type: Defect\nrelations:\n- target: /features/a\n  state: open\n  severity: high\n", ""),
            concept("features/a", "type: Feature", ""),
            concept("notes/a", "type: Note", "[feature](/features/a.md)"),
        ]};
        (bundle, ontology)
    }
    #[test]
    fn incoming_kind_and_inverse_select_same_occurrences_before_collapse() {
        let (b, o) = fixture();
        let filters = EdgeFilters {
            kind: vec!["blocks".into()],
            inverse_kind: vec!["is-blocked-by".into()],
            related_type: vec!["Defect".into()],
            ..Default::default()
        }
        .compile()
        .unwrap();
        let detailed = query_local(
            &b,
            Some(&o),
            "features/a",
            Direction::Incoming,
            &filters,
            &QueryOptions {
                details: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(detailed.evidence.matched_occurrences, 3);
        assert_eq!(detailed.evidence.matched_concepts, 2);
        assert!(detailed
            .occurrences
            .iter()
            .all(|r| r.relationship.is_some()));
        let compact = query_local(
            &b,
            Some(&o),
            "features/a",
            Direction::Incoming,
            &filters,
            &QueryOptions {
                limit: Some(1),
                offset: 1,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(compact.occurrences[0].related, "/defects/b");
        assert!(compact.evidence.scan_complete);
        assert!(!compact.evidence.delivery_complete);
    }
    #[test]
    fn parallel_attributes_cannot_cross_match_and_filtering_precedes_limit() {
        let (b, o) = fixture();
        let filters = EdgeFilters {
            edge: vec!["state=\"open\"".into(), "severity=\"high\"".into()],
            ..Default::default()
        }
        .compile()
        .unwrap();
        let q = query_local(
            &b,
            Some(&o),
            "features/a",
            Direction::Incoming,
            &filters,
            &QueryOptions {
                limit: Some(1),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(q.occurrences.len(), 1);
        assert_eq!(q.occurrences[0].related, "/defects/b");
        assert_eq!(q.evidence.matched_occurrences, 1);
    }
    #[test]
    fn endpoint_missing_is_unknown_and_scan_budget_is_not_result_limit() {
        let (b, o) = fixture();
        let filters = EdgeFilters {
            related_type: vec!["Feature".into()],
            ..Default::default()
        }
        .compile()
        .unwrap();
        let q = query_local(
            &b,
            Some(&o),
            "defects/a",
            Direction::Outgoing,
            &filters,
            &QueryOptions::default(),
        )
        .unwrap();
        assert_eq!(q.evidence.unresolved_candidates, 1);
        assert_eq!(q.evidence.matched_occurrences, 2);
        let q = query_local(
            &b,
            Some(&o),
            "features/a",
            Direction::Incoming,
            &EdgeFilters::default().compile().unwrap(),
            &QueryOptions {
                scan_limit: Some(1),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(!q.evidence.scan_complete);
        assert_eq!(q.evidence.examined_documents, 1);
    }
    #[test]
    fn presets_replace_categories_and_clear_without_silent_accumulation() {
        let base = EdgeFilters {
            kind: vec!["blocks".into()],
            related_type: vec!["Defect".into()],
            ..Default::default()
        };
        let explicit = EdgeFilters {
            kind: vec!["depends-on".into()],
            ..Default::default()
        };
        let merged = base.overlay(&explicit, &["related-type".into()]).unwrap();
        assert_eq!(merged.kind, vec!["depends-on"]);
        assert!(merged.related_type.is_empty());
        assert!(base.overlay(&explicit, &["kind".into()]).is_err());
        assert!(base
            .overlay(&EdgeFilters::default(), &["typo".into()])
            .is_err());
        assert!(serde_yaml::from_str::<EdgeFilters>("kinds: [blocks]").is_err());
    }
    #[test]
    fn ordinary_edges_only_match_endpoint_filters_and_unknown_kinds_do_not_match() {
        let c = concept("d", "type: Defect\npriority: 3", "");
        let endpoint = EdgeFilters {
            related: vec!["priority=3".into()],
            ..Default::default()
        }
        .compile()
        .unwrap();
        assert_eq!(
            endpoint.matches(None, Some(&c)).unwrap(),
            MatchOutcome::Matched
        );
        let kind = EdgeFilters {
            kind: vec!["blocks".into()],
            ..Default::default()
        }
        .compile()
        .unwrap();
        assert_eq!(
            kind.matches(None, Some(&c)).unwrap(),
            MatchOutcome::Excluded
        );
        let edge = Relationship {
            source: "/d".into(),
            target: "/f".into(),
            rule: "r".into(),
            field_path: "refs[0]".into(),
            raw_reference: "/f".into(),
            authored_kind: Some("blocks".into()),
            inverse: None,
            attributes: IndexMap::<String, Value>::new(),
            status: "unknown-kind".into(),
        };
        assert_eq!(
            kind.matches(Some(&edge), Some(&c)).unwrap(),
            MatchOutcome::Excluded
        );
    }
    #[test]
    fn graph_adjacency_contains_only_matching_occurrences() {
        let (b, o) = fixture();
        let filters = EdgeFilters {
            edge: vec!["state=\"open\"".into(), "severity=\"high\"".into()],
            ..Default::default()
        }
        .compile()
        .unwrap();
        let (graph, evidence) =
            filtered_graph(&b, Some(&o), &filters, Direction::Outgoing).unwrap();
        assert_eq!(
            graph.outbound("/defects/a"),
            &[ConceptId("/features/missing".into())]
        );
        assert_eq!(
            graph.inbound("/features/a"),
            &[ConceptId("/defects/b".into())]
        );
        assert!(graph.exists("/notes/a"));
        assert_eq!(evidence.matched_occurrences, 2);
    }
    #[test]
    fn both_direction_traversal_filters_neighbor_before_expanding() {
        let (b, o) = fixture();
        let f = EdgeFilters {
            related_type: vec!["Defect".into()],
            kind: vec!["blocks".into()],
            ..Default::default()
        }
        .compile()
        .unwrap();
        let (g, e) = rooted_filtered_graph(
            &b,
            Some(&o),
            &f,
            "features/a",
            TraversalDirection::Both,
            Some(2),
        )
        .unwrap();
        assert_eq!(
            g.inbound("/features/a"),
            &[
                ConceptId("/defects/a".into()),
                ConceptId("/defects/b".into())
            ]
        );
        // Once a defect is visited, its outgoing Feature neighbor fails related-type Defect.
        assert!(g.inbound("/features/missing").is_empty());
        assert_eq!(e.matched_concepts, 2);
        let (g, e) = rooted_filtered_graph(
            &b,
            Some(&o),
            &f,
            "features/a",
            TraversalDirection::Both,
            Some(0),
        )
        .unwrap();
        assert!(g.exists("/features/a"));
        assert!(g.inbound("/features/a").is_empty());
        assert_eq!(e.matched_occurrences, 0);
    }
}
