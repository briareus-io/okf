//! Independent optional ontology policy. Findings are compared structurally, never by prose.
use super::transaction::FileChange;
use crate::bundle::loader::Bundle;
use crate::error::{OkfError, Result};
use crate::model::concept::{Concept, ConceptId};
use crate::model::link::{LinkKind, LogicalReference, classify, resolve_link};
use crate::ontology::field_types::{
    ViolationKind, check_concept_with_budget, concept_is_a, resolve_field,
};
use crate::ontology::schema::Ontology;
use std::collections::BTreeMap;
use std::path::Path;
use yaml_serde::Value;

/// Frozen, explicitly examined reference targets. The caller binds these views and
/// their catalog identities to the guarded publication; the gate never widens scope.
#[derive(Debug, Clone)]
pub struct Scope {
    pub primary_id: String,
    pub targets: BTreeMap<String, (Bundle, Option<Ontology>)>,
}

struct TargetView<'a> {
    bundle_id: Option<&'a str>,
    id: String,
    concept: Option<&'a Concept>,
    ontology: Option<&'a Ontology>,
    local: bool,
}

fn incomplete(message: impl Into<String>) -> OkfError {
    OkfError::classified(crate::error::FailureReason::WriteGateIncomplete, message)
}

fn is_concept_path(path: &Path, allow_extensionless: bool) -> bool {
    !crate::bundle::walk::is_support_path(path)
        && (path.extension().and_then(|s| s.to_str()) == Some("md")
            || (allow_extensionless && path.extension().is_none()))
        && !matches!(
            path.file_stem().and_then(|s| s.to_str()),
            Some("index" | "log")
        )
}

/// Resolve against the primary prospective state or a frozen examined sibling.
/// None means a genuinely external URI, not an unavailable OKF target.
fn reference_target<'a>(
    bundle: &'a Bundle,
    ontology: &'a Ontology,
    source: &ConceptId,
    raw: &str,
    scope: Option<&'a Scope>,
) -> Result<Option<TargetView<'a>>> {
    let logical = LogicalReference::parse(raw).map_err(|e| incomplete(e.to_string()))?;
    if logical.is_none() && classify(raw) == LinkKind::External {
        return Ok(None);
    }
    let local_root = std::fs::canonicalize(&bundle.root).unwrap_or_else(|_| bundle.root.clone());
    let (owner, target_bundle, target_ontology, id, local, concept_path) = if let Some(logical) =
        logical
    {
        let scope = scope.ok_or_else(|| {
            incomplete(format!(
                "logical reference requires explicit examination scope: {raw}"
            ))
        })?;
        if logical.bundle == scope.primary_id {
            (
                Some(scope.primary_id.as_str()),
                bundle,
                Some(ontology),
                ConceptId::from_relative(logical.path.strip_suffix(".md").unwrap_or(&logical.path))
                    .0,
                true,
                is_concept_path(Path::new(&logical.path), true),
            )
        } else {
            let (target, ontology) = scope.targets.get(&logical.bundle).ok_or_else(|| {
                incomplete(format!(
                    "reference to unexamined bundle: {}",
                    logical.bundle
                ))
            })?;
            (
                Some(
                    scope
                        .targets
                        .get_key_value(&logical.bundle)
                        .expect("examined target")
                        .0
                        .as_str(),
                ),
                target,
                ontology.as_ref(),
                ConceptId::from_relative(logical.path.strip_suffix(".md").unwrap_or(&logical.path))
                    .0,
                false,
                is_concept_path(Path::new(&logical.path), true),
            )
        }
    } else if let Some(path) = crate::graph::catalog::ordinary_path(&local_root, &source.0, raw)
        && !path.starts_with(&local_root)
    {
        let (owner, (target, ontology)) = scope
            .and_then(|scope| {
                scope
                    .targets
                    .iter()
                    .filter(|(_, (target, _))| path.starts_with(&target.root))
                    .max_by_key(|(_, (target, _))| target.root.components().count())
            })
            .ok_or_else(|| {
                incomplete(format!(
                    "reference outside examined bundle: {} {path:?}",
                    source.0
                ))
            })?;
        let relative = path.strip_prefix(&target.root).expect("target owns path");
        let relative = relative.to_string_lossy();
        (
            Some(owner.as_str()),
            target,
            ontology.as_ref(),
            ConceptId::from_relative(relative.strip_suffix(".md").unwrap_or(&relative)).0,
            false,
            is_concept_path(Path::new(relative.as_ref()), false),
        )
    } else {
        (
            scope.map(|scope| scope.primary_id.as_str()),
            bundle,
            Some(ontology),
            resolve_link(source, raw).0,
            true,
            scope.is_none()
                || crate::graph::catalog::ordinary_path(&local_root, &source.0, raw)
                    .is_some_and(|path| is_concept_path(&path, false)),
        )
    };
    Ok(Some(TargetView {
        bundle_id: owner,
        concept: concept_path.then(|| target_bundle.get(&id)).flatten(),
        id,
        ontology: target_ontology,
        local,
    }))
}

fn matches_target(target: &TargetView<'_>, expected: &[&str]) -> Result<bool> {
    let actual = target.concept.and_then(Concept::concept_type).unwrap_or("");
    let mut unknown_inheritance = false;
    for expected in expected {
        let expected = if target.bundle_id.is_some()
            && let Some((bundle, name)) = expected.split_once(':')
            && crate::bundle::catalog::valid_id(bundle)
            && !name.is_empty()
        {
            if Some(bundle) != target.bundle_id {
                continue;
            }
            name
        } else {
            expected
        };
        if actual == expected {
            return Ok(true);
        }
        if let Some(ontology) = target.ontology {
            if concept_is_a(ontology, actual, expected) {
                return Ok(true);
            }
        } else if !actual.is_empty() {
            unknown_inheritance = true;
        }
    }
    if unknown_inheritance {
        return Err(incomplete(format!(
            "target ontology unavailable for inheritance check: {}:{}",
            target.bundle_id.unwrap_or(""),
            target.id
        )));
    }
    Ok(false)
}

fn target_identity(target: &TargetView<'_>, identities: &BTreeMap<String, String>) -> String {
    if target.local {
        identities.get(&target.id).unwrap_or(&target.id).clone()
    } else {
        format!("{}:{}", target.bundle_id.unwrap_or(""), target.id)
    }
}

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
        &yaml_serde::to_value(value).map_err(|e| OkfError::Internal(e.to_string()))?,
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
    scope: Option<&Scope>,
) -> Result<Findings> {
    let mut out = Findings::new();
    for c in &bundle.concepts {
        let id = identities.get(&c.id.0).unwrap_or(&c.id.0);
        let root = yaml_serde::to_value(&c.frontmatter.map)
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
                        return Err(OkfError::classified(
                            crate::error::FailureReason::WriteGateIncomplete,
                            format!("{id} {path}: {code}"),
                        ));
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
                    continue;
                }
            };
            let mut values = Vec::new();
            values_at(&root, "", path, &mut values, 0);
            if values.len() > 1 {
                return Err(OkfError::classified(
                    crate::error::FailureReason::WriteGateComparisonAmbiguous,
                    format!("{id} {path}"),
                ));
            }
            let evidence = values
                .first()
                .map_or_else(|| "absent".into(), |v| canonical(v));
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
                let Some(target) = reference_target(bundle, ontology, &c.id, raw, scope)? else {
                    continue;
                };
                let canonical_id = target_identity(&target, identities);
                match target.concept {
                    None => record(
                        &mut out,
                        id,
                        "reference-missing",
                        &path,
                        &constraint,
                        &canonical_id,
                    ),
                    Some(t) => {
                        let actual = t.concept_type().unwrap_or("");
                        if !matches_target(&target, &rule.target.types())? {
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
                && let Some(target) =
                    reference_target(bundle, ontology, &c.id, &edge.raw_reference, scope)?
                && let Some(concept) = target.concept
            {
                let actual = concept.concept_type().unwrap_or("");
                if !matches_target(&target, &targets.types())? {
                    record(
                        &mut out,
                        id,
                        "relationship-target",
                        &edge.field_path,
                        &serialized(targets)?,
                        &format!("{}:{actual}", target_identity(&target, identities)),
                    );
                }
            }
        }
    }
    Ok(out)
}
fn compare(old: &Findings, new: &Findings) -> Result<()> {
    let introduced: Vec<_> = new
        .iter()
        .filter(|(k, n)| **n > old.get(*k).copied().unwrap_or(0))
        .map(|(k, _)| k.as_str())
        .collect();
    if introduced.is_empty() {
        Ok(())
    } else {
        Err(OkfError::classified(
            crate::error::FailureReason::WriteGateRejected,
            format!(
                "{} new/worsened ontology finding(s): {}",
                introduced.len(),
                introduced
                    .iter()
                    .take(5)
                    .map(|s| s.chars().take(240).collect::<String>())
                    .collect::<Vec<_>>()
                    .join("; ")
            ),
        ))
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
    check_changes_inner(root, changes, strengthen, None)
}

/// Check using frozen target bundles captured under the caller's publication guards.
pub fn check_changes_scoped(
    root: &Path,
    changes: &[FileChange],
    strengthen: bool,
    scope: &Scope,
) -> Result<()> {
    check_changes_inner(root, changes, strengthen, Some(scope))
}

fn check_changes_inner(
    root: &Path,
    changes: &[FileChange],
    strengthen: bool,
    scope: Option<&Scope>,
) -> Result<()> {
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
    let ontology = ontology.ok_or_else(|| {
        OkfError::classified(
            crate::error::FailureReason::WriteGateIncomplete,
            "ontology unavailable",
        )
    })?;
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
            let text = std::str::from_utf8(bytes).map_err(|e| {
                OkfError::Usage(format!("invalid UTF-8 in {}: {e}", change.path.display()))
            })?;
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
        &findings(&before, &ontology, &identities, scope)?,
        &findings(&after, &ontology, &BTreeMap::new(), scope)?,
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
        .map(|o| findings(&bundle, o, &BTreeMap::new(), None))
        .transpose()?
        .unwrap_or_default();
    compare(&old, &findings(&bundle, after, &BTreeMap::new(), None)?)
}
