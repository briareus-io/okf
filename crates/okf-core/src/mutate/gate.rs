//! Independent optional ontology policy. Findings are compared structurally, never by prose.
use super::transaction::FileChange;
use crate::bundle::loader::Bundle;
use crate::error::{OkfError, Result};
use crate::model::concept::{Concept, ConceptId};
use crate::model::link::{classify, resolve_link, LinkKind};
use crate::ontology::field_types::{
    check_concept_with_budget, concept_is_a, resolve_field, ViolationKind,
};
use crate::ontology::schema::Ontology;
use serde_yaml::Value;
use std::collections::BTreeMap;
use std::path::Path;

type Findings = BTreeMap<String, usize>;
fn rank(value: &str) -> u8 {
    match value {
        "info" => 1,
        "warn" => 2,
        _ => 3,
    }
}
fn enforced(root: &Path, strengthen: bool) -> Result<bool> {
    let (settings, _) = crate::bundle::settings::load_for(root)?;
    let p = &settings.settings.write_policy;
    Ok(strengthen
        || (p.ontology_gate
            && rank(p.ontology_severity.as_deref().unwrap_or("error"))
                >= rank(p.blocking_threshold.as_deref().unwrap_or("error"))))
}
// Sorted mapping entries keep presentation order out of a semantic signature. YAML types
// remain distinct; JSON conversion would lose non-string keys and tagged extension values.
fn canonical(value: &Value) -> String {
    match value {
        Value::Mapping(map) => {
            let mut entries: Vec<_> = map
                .iter()
                .map(|(k, v)| (canonical(k), canonical(v)))
                .collect();
            entries.sort();
            format!("map:{entries:?}")
        }
        Value::Sequence(items) => {
            format!("seq:{:?}", items.iter().map(canonical).collect::<Vec<_>>())
        }
        _ => format!("scalar:{value:?}"),
    }
}
fn serialized<T: serde::Serialize>(value: &T) -> Result<String> {
    Ok(canonical(
        &serde_yaml::to_value(value).map_err(|e| OkfError::Internal(e.to_string()))?,
    ))
}
// Build paths exactly as the field checker does. Ambiguous authored dotted keys are
// detected rather than silently selecting one of two values for a grandfathered finding.
fn values_at<'a>(
    value: &'a Value,
    path: &str,
    wanted: &str,
    out: &mut Vec<&'a Value>,
    depth: usize,
) {
    if path == wanted {
        out.push(value);
    }
    if depth > 64 {
        return;
    }
    match value {
        Value::Mapping(map) => {
            for (key, child) in map {
                if let Some(key) = key.as_str() {
                    let next = if path.is_empty() {
                        key.to_owned()
                    } else {
                        format!("{path}.{key}")
                    };
                    values_at(child, &next, wanted, out, depth + 1);
                }
            }
        }
        Value::Sequence(items) => {
            for (i, child) in items.iter().enumerate() {
                values_at(child, &format!("{path}[{i}]"), wanted, out, depth + 1);
            }
        }
        _ => {}
    }
}
fn logical_path(path: &str) -> String {
    // Only concrete array indices are removed, preserving the logical constraint location.
    static INDICES: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    INDICES
        .get_or_init(|| regex::Regex::new(r"\[\d+\]").unwrap())
        .replace_all(path, "[]")
        .into_owned()
}
fn record(out: &mut Findings, id: &str, code: &str, path: &str, constraint: &str, evidence: &str) {
    *out.entry(format!(
        "{id:?}|{code:?}|{}|{constraint}|{evidence}",
        logical_path(path)
    ))
    .or_default() += 1;
}
fn findings(
    bundle: &Bundle,
    ontology: &Ontology,
    identities: &BTreeMap<String, String>,
) -> Result<Findings> {
    let mut out = Findings::new();
    for c in &bundle.concepts {
        let id = identities.get(&c.id.0).unwrap_or(&c.id.0);
        let root = serde_yaml::to_value(&c.frontmatter.map)
            .map_err(|e| OkfError::Internal(e.to_string()))?;
        let definition = c
            .concept_type()
            .and_then(|name| ontology.concepts.get(name));
        let resolver = |raw: &str| {
            bundle
                .get(&resolve_link(&c.id, raw).0)
                .and_then(|t| t.concept_type().map(str::to_owned))
        };
        for v in check_concept_with_budget(ontology, &c.frontmatter, resolver, usize::MAX) {
            let (code, path) = match &v.kind {
                ViolationKind::Metadata { code, path } => {
                    if code == "incomplete-check" || code == "metadata-unenforced" {
                        return Err(OkfError::Policy(format!(
                            "write-gate-incomplete: {id} {path}: {code}"
                        )));
                    }
                    // Reference cardinality/target occurrences are evaluated below with actual
                    // identities/types, avoiding a rendered message or whole-list signature.
                    if code.starts_with("metadata-reference-")
                        || code == "metadata-relationship-kind"
                    {
                        continue;
                    }
                    (code.as_str(), path.as_str())
                }
                ViolationKind::UnknownType => ("unknown-type", "type"),
                ViolationKind::MissingRequiredBuiltin(path) => {
                    ("missing-required-builtin", path.as_str())
                }
                ViolationKind::MissingRequiredField(path) => {
                    ("missing-required-field", path.as_str())
                }
                ViolationKind::CardinalityViolation(_) | ViolationKind::WrongReferenceTarget(_) => {
                    continue
                }
            };
            let mut values = Vec::new();
            values_at(&root, "", path, &mut values, 0);
            if values.len() > 1 {
                return Err(OkfError::Policy(format!(
                    "write-gate-comparison-ambiguous: {id} {path}"
                )));
            }
            let evidence = values
                .first()
                .map(|v| canonical(v))
                .unwrap_or_else(|| "absent".into());
            let field = definition.and_then(|d| {
                d.fields.iter().find(|(key, _)| {
                    path == key.as_str()
                        || path.starts_with(&format!("{key}."))
                        || path.starts_with(&format!("{key}["))
                })
            });
            let constraint = if let Some((_, field)) = field {
                serialized(&resolve_field(ontology, field)?)?
            } else {
                format!("{code}:{}", c.concept_type().unwrap_or(""))
            };
            record(&mut out, id, code, path, &constraint, &evidence);
        }
        let Some(definition) = definition else {
            continue;
        };
        for (key, rule) in &definition.references {
            let constraint = serialized(rule)?;
            let mut occurrences: Vec<(String, &Value)> = Vec::new();
            if let Some(selector) = &rule.selector {
                occurrences.extend(
                    crate::query::selector::Selector::parse(selector)?
                        .select(&root)
                        .leaves
                        .into_iter()
                        .map(|o| (o.path, o.value)),
                );
            } else if let Some(value) = c.frontmatter.get(key) {
                if let Value::Sequence(items) = value {
                    occurrences.extend(
                        items
                            .iter()
                            .enumerate()
                            .map(|(i, v)| (format!("{key}[{i}]"), v)),
                    );
                } else {
                    occurrences.push((key.clone(), value));
                }
            }
            let count = occurrences
                .iter()
                .filter(|(_, v)| v.as_str().is_some_and(|s| !s.trim().is_empty()))
                .count();
            if !rule.cardinality.permits(count) {
                record(
                    &mut out,
                    id,
                    "reference-cardinality",
                    key,
                    &constraint,
                    &count.to_string(),
                );
            }
            for (path, value) in occurrences {
                let Some(raw) = value.as_str().filter(|s| !s.trim().is_empty()) else {
                    record(
                        &mut out,
                        id,
                        "reference-type",
                        &path,
                        &constraint,
                        &canonical(value),
                    );
                    continue;
                };
                if classify(raw) == LinkKind::External {
                    continue;
                }
                let local_root =
                    std::fs::canonicalize(&bundle.root).unwrap_or_else(|_| bundle.root.clone());
                if let Some(path) = crate::graph::catalog::ordinary_path(&local_root, &c.id.0, raw)
                {
                    if !path.starts_with(&local_root) {
                        return Err(OkfError::Policy(format!("write-gate-incomplete: reference outside examined bundle: {id} {path:?}")));
                    }
                }
                let target = resolve_link(&c.id, raw);
                let canonical_id = identities.get(&target.0).unwrap_or(&target.0);
                match bundle.get(&target.0) {
                    None => record(
                        &mut out,
                        id,
                        "reference-missing",
                        &path,
                        &constraint,
                        canonical_id,
                    ),
                    Some(t) => {
                        let actual = t.concept_type().unwrap_or("");
                        if !rule
                            .target
                            .types()
                            .iter()
                            .any(|expected| concept_is_a(ontology, actual, expected))
                        {
                            record(
                                &mut out,
                                id,
                                "reference-target",
                                &path,
                                &constraint,
                                &format!("{canonical_id}:{actual}"),
                            );
                        }
                    }
                }
            }
        }
        // Kind-specific target narrowing is separate from the reference's broad target union.
        for edge in crate::graph::relationships::relationships(c, ontology) {
            if edge.status == "unknown-kind" {
                record(
                    &mut out,
                    id,
                    "relationship-kind",
                    &edge.field_path,
                    &serialized(&definition.relationships.get(&edge.rule))?,
                    &serialized(&edge.authored_kind)?,
                );
            }
            if let Some(targets) = definition
                .relationships
                .get(&edge.rule)
                .and_then(|r| edge.authored_kind.as_ref().and_then(|k| r.kinds.get(k)))
                .and_then(|k| k.target.as_ref())
            {
                if let Some(target) = bundle.get(&resolve_link(&c.id, &edge.raw_reference).0) {
                    let actual = target.concept_type().unwrap_or("");
                    if !targets
                        .types()
                        .iter()
                        .any(|t| concept_is_a(ontology, actual, t))
                    {
                        record(
                            &mut out,
                            id,
                            "relationship-target",
                            &edge.field_path,
                            &serialized(targets)?,
                            &format!(
                                "{}:{actual}",
                                identities.get(&target.id.0).unwrap_or(&target.id.0)
                            ),
                        );
                    }
                }
            }
        }
    }
    Ok(out)
}
fn compare(old: Findings, new: Findings) -> Result<()> {
    let introduced: Vec<_> = new
        .iter()
        .filter(|(k, n)| **n > old.get(*k).copied().unwrap_or(0))
        .map(|(k, _)| k.as_str())
        .collect();
    if introduced.is_empty() {
        Ok(())
    } else {
        Err(OkfError::Policy(format!(
            "write-gate-rejected: {} new/worsened ontology finding(s): {}",
            introduced.len(),
            introduced
                .iter()
                .take(5)
                .map(|s| s.chars().take(240).collect::<String>())
                .collect::<Vec<_>>()
                .join("; ")
        )))
    }
}
fn identity_payload(c: &Concept, ontology: &Ontology) -> String {
    let mut map = c.frontmatter.map.clone();
    for key in ["sources", "generated", "verified"] {
        map.shift_remove(key);
    }
    if let Some(d) = c
        .concept_type()
        .and_then(|name| ontology.concepts.get(name))
    {
        for (key, rule) in &d.references {
            let first = rule
                .selector
                .as_deref()
                .unwrap_or(key)
                .split(['.', '['])
                .next()
                .unwrap_or(key);
            map.shift_remove(first);
        }
    }
    serialized(&map).unwrap_or_default()
}
/// Check a final metadata state; moves can preserve findings only with unambiguous identity.
pub fn check_changes(root: &Path, changes: &[FileChange], strengthen: bool) -> Result<()> {
    if !changes.iter().any(|c| {
        c.path.extension().and_then(|s| s.to_str()) == Some("md")
            && !matches!(
                c.path.file_name().and_then(|s| s.to_str()),
                Some("index.md" | "log.md")
            )
    }) {
        return Ok(());
    }
    if !enforced(root, strengthen)? {
        return Ok(());
    }
    let (_, ontology) = crate::bundle::settings::load_for(root)?;
    let ontology = ontology
        .ok_or_else(|| OkfError::Policy("write-gate-incomplete: ontology unavailable".into()))?;
    let before = crate::bundle::loader::load_bundle(root)?;
    let mut after = before.clone();
    for change in changes {
        if change.path.extension().and_then(|s| s.to_str()) != Some("md")
            || matches!(
                change.path.file_name().and_then(|s| s.to_str()),
                Some("index.md" | "log.md")
            )
        {
            continue;
        }
        let id = ConceptId::from_relative(change.path.to_string_lossy().trim_end_matches(".md"));
        after.concepts.retain(|c| c.id != id);
        if let Some(bytes) = &change.after {
            let text = std::str::from_utf8(bytes)
                .map_err(|e| OkfError::Usage(format!("invalid UTF-8: {e}")))?;
            after.concepts.push(crate::parse::parse_concept(id, text)?);
        }
    }
    after.concepts.sort_by(|a, b| a.id.0.cmp(&b.id.0));
    let deleted: Vec<_> = before
        .concepts
        .iter()
        .filter(|c| after.get(&c.id.0).is_none())
        .collect();
    let created: Vec<_> = after
        .concepts
        .iter()
        .filter(|c| before.get(&c.id.0).is_none())
        .collect();
    let mut identities = BTreeMap::new();
    for old in &deleted {
        let signature = identity_payload(old, &ontology);
        let matches: Vec<_> = created
            .iter()
            .filter(|c| identity_payload(c, &ontology) == signature)
            .collect();
        if matches.len() == 1
            && deleted
                .iter()
                .filter(|c| identity_payload(c, &ontology) == signature)
                .count()
                == 1
        {
            identities.insert(old.id.0.clone(), matches[0].id.0.clone());
        }
    }
    compare(
        findings(&before, &ontology, &identities)?,
        findings(&after, &ontology, &BTreeMap::new())?,
    )
}
/// Compare the whole consumer bundle under the prior and prospective sidecar definitions.
pub fn check_ontology_change(
    root: &Path,
    before: Option<&Ontology>,
    after: &Ontology,
    strengthen: bool,
) -> Result<()> {
    if !enforced(root, strengthen)? {
        return Ok(());
    }
    let bundle = crate::bundle::loader::load_bundle(root)?;
    let old = before
        .map(|o| findings(&bundle, o, &BTreeMap::new()))
        .transpose()?
        .unwrap_or_default();
    compare(old, findings(&bundle, after, &BTreeMap::new())?)
}
