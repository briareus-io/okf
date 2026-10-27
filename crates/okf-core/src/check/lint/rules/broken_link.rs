//! Broken-link rule: a concept links (via frontmatter refs or body markdown) at an internal
//! target that is not a loaded concept. Broken links are spec-legal, so this is advisory only.
use crate::check::lint::{Finding, RuleContext};
use crate::model::link::{classify, raw_links, resolve_link, LinkKind};
use crate::query::artifact::{ArtifactKind, ArtifactResolver};

/// Rule identifier.
pub const RULE: &str = "broken-link";

/// One finding per outbound edge whose target does not exist in the bundle.
pub fn run(ctx: &RuleContext) -> Vec<Finding> {
    let Some(severity) = ctx.config.broken_link else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let resolver = ArtifactResolver::new(&ctx.bundle.root);
    for concept in &ctx.bundle.concepts {
        let raw = raw_links(concept, ctx.ontology);
        for target in ctx.graph.outbound(&concept.id.0) {
            if ctx.graph.exists(&target.0) {
                continue;
            }
            let explicit_nested = ctx
                .ontology
                .and_then(|o| concept.concept_type().and_then(|t| o.concepts.get(t)))
                .is_some_and(|ct| {
                    ct.references
                        .iter()
                        .filter_map(|(_, r)| r.selector.as_ref())
                        .any(|raw| {
                            let root =
                                serde_yaml::to_value(&concept.frontmatter.map).unwrap_or_default();
                            crate::query::selector::Selector::parse(raw)
                                .ok()
                                .is_some_and(|s| {
                                    s.select(&root)
                                        .leaves
                                        .iter()
                                        .filter_map(|o| o.value.as_str())
                                        .any(|raw| {
                                            crate::model::link::resolve_link(&concept.id, raw)
                                                == *target
                                        })
                                })
                        })
                });
            // Graph IDs discard .md and include non-concept body links. Resolve the
            // original spelling so real indexes/artifacts do not become false findings.
            // Every occurrence must resolve: a valid alias cannot hide a missing or
            // escaping path which normalized to the same graph ID.
            let mut occurrences = raw
                .iter()
                .filter(|raw| {
                    classify(raw) != LinkKind::External && resolve_link(&concept.id, raw) == *target
                })
                .peekable();
            let existing_artifact = occurrences.peek().is_some()
                && occurrences.all(|raw| {
                    let resolved = resolver.resolve(Some(&concept.id.0), raw);
                    resolved.exists
                        && matches!(
                            resolved.kind,
                            ArtifactKind::Reserved | ArtifactKind::Artifact
                        )
                });
            if !explicit_nested && !existing_artifact {
                out.push(Finding {
                    code: None,
                    field_path: None,
                    rule: RULE.to_string(),
                    severity,
                    concept: Some(concept.id.0.clone()),
                    message: format!("links to missing concept {}", target.0),
                });
            }
        }
    }
    out
}
