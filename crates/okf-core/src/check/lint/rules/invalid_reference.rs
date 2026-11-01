//! Diagnose CLI-qualified targets accidentally authored as portable references.
//!
//! URI syntax alone cannot establish intent. Only catalog-registered bundle identities
//! are diagnosed, without reading those bundles or widening examination scope.
use crate::bundle::{catalog::Catalog, loader::Bundle};
use crate::check::lint::{Finding, Severity};
use crate::ontology::schema::Ontology;
use pulldown_cmark::{Event, Parser, Tag};
use serde_yaml::Value;

pub const RULE: &str = "invalid-reference";

pub fn run(
    bundle: &Bundle,
    ontology: Option<&Ontology>,
    catalog: Option<&Catalog>,
    severity: Option<Severity>,
) -> Vec<Finding> {
    let (Some(catalog), Some(severity)) = (catalog, severity) else {
        return vec![];
    };
    let mut findings = Vec::new();
    for concept in &bundle.concepts {
        let mut inspect = |raw: &str, field_path: String| {
            let Some((id, path)) = raw.trim().split_once(":/") else {
                return;
            };
            // Ordinary hierarchical URIs use ://. Names that are not registered may
            // be legitimate custom schemes; do not guess that they are bundle IDs.
            if path.is_empty() || path.starts_with('/') || !catalog.entries.contains_key(id) {
                return;
            }
            findings.push(Finding {
                rule: RULE.into(),
                code: Some("qualified-cli-target".into()),
                field_path: Some(field_path),
                severity,
                concept: Some(concept.id.0.clone()),
                message: format!(
                    "{raw:?} uses qualified CLI target syntax, which is not an authored reference; use okf://{id}/{path} or a document-relative path"
                ),
            });
        };
        if let Some(sources) = concept
            .frontmatter
            .get("sources")
            .and_then(Value::as_sequence)
        {
            for (index, source) in sources.iter().enumerate() {
                if let Some(resource) = source.get("resource").and_then(Value::as_str) {
                    inspect(resource, format!("sources[{index}].resource"));
                }
            }
        }
        if let Some(ct) = concept
            .concept_type()
            .and_then(|name| ontology.and_then(|o| o.concepts.get(name)))
        {
            let root = serde_yaml::to_value(&concept.frontmatter.map).unwrap_or_default();
            for (key, rule) in &ct.references {
                if let Ok(selector) =
                    crate::graph::relationships::reference_selector(key, rule.selector.as_deref())
                {
                    for occurrence in selector.select(&root).leaves {
                        inspect_value(occurrence.value, occurrence.path, &mut inspect);
                    }
                }
            }
        }
        for (event, range) in Parser::new(&concept.body).into_offset_iter() {
            if let Event::Start(Tag::Link { dest_url, .. } | Tag::Image { dest_url, .. }) = event {
                let line = concept.body[..range.start]
                    .bytes()
                    .filter(|b| *b == b'\n')
                    .count()
                    + 1;
                inspect(&dest_url, format!("body:line:{line}"));
            }
        }
    }
    findings
}

fn inspect_value(value: &Value, path: String, inspect: &mut impl FnMut(&str, String)) {
    match value {
        Value::String(raw) => inspect(raw, path),
        Value::Sequence(values) => {
            for (index, value) in values.iter().enumerate() {
                inspect_value(value, format!("{path}[{index}]"), inspect);
            }
        }
        _ => {}
    }
}
