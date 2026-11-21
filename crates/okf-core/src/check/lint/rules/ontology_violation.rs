//! Ontology-violation rule: check each concept against its ontology-declared type (unknown
//! type, missing required built-in/custom fields, reference cardinality, wrong reference
//! target). Skipped entirely when the bundle has no `ontology.yaml`.
//!
//! Reference-target checks need the *type* of each linked concept, which lives in the graph /
//! bundle. We supply that to [`ontology::check_concept`] as a `resolve_link_type` closure:
//! resolve the raw link against the source concept, look the target up in the bundle, return
//! its `type`. Unresolvable links yield `None` and are skipped (keeping the check permissive).
//!
//! [`ontology::check_concept`]: crate::ontology::field_types::check_concept
use crate::check::lint::{Finding, RuleContext, Severity};
use crate::model::link::resolve_link;
use crate::model::standard::LIFECYCLE_STATUSES;
use crate::ontology::field_types::{check_concept_with_budget, resolve_field};
use crate::ontology::schema::{FieldType, Ontology};

/// Rule identifier.
pub const RULE: &str = "ontology-violation";

/// Check authored top-level fields once per declaration, independently of concept records.
pub fn declaration_findings(
    ontology: Option<&Ontology>,
    severity: Option<Severity>,
) -> crate::error::Result<Vec<Finding>> {
    let mut findings = Vec::new();
    let (Some(ontology), Some(severity)) = (ontology, severity) else {
        return Ok(findings);
    };
    for (name, concept) in &ontology.concepts {
        let Some(field) = concept.fields.get("status") else {
            continue;
        };
        let ty = resolve_field(ontology, field)?.ty;
        let declaration = match ty.base {
            FieldType::String | FieldType::Text => continue,
            FieldType::Enum
                if ty
                    .values
                    .iter()
                    .all(|value| LIFECYCLE_STATUSES.contains(&value.as_str())) =>
            {
                continue;
            }
            FieldType::Enum => format!("enum {:?}", ty.values),
            other => format!("type {other}"),
        };
        let path = format!("concepts.{name}.fields.status");
        findings.push(Finding { comparison_detail: None,
            rule: RULE.into(),
            code: Some("ontology-status-conflict".into()),
            message: format!("{path} declares {declaration}. OKF lifecycle status accepts {}. Use a separate field such as workflow_state for domain states.", LIFECYCLE_STATUSES.join(", ")),
            field_path: Some(path),
            severity,
            concept: None,
        });
    }
    Ok(findings)
}

/// One finding per ontology violation across all concepts. Empty when no ontology is present.
pub fn run(ctx: &RuleContext<'_>) -> Vec<Finding> {
    let Some(severity) = ctx.config.ontology_violation else {
        return Vec::new();
    };
    let Some(ontology) = ctx.ontology else {
        return Vec::new();
    };

    let mut out = Vec::new();
    for concept in &ctx.bundle.concepts {
        // Resolve a raw link (relative to this concept) to its target's `type`.
        let resolve_link_type = |link: &str| -> Option<String> {
            let target = resolve_link(&concept.id, link);
            ctx.bundle
                .get(&target.0)
                .and_then(|c| c.concept_type().map(str::to_string))
        };

        for violation in check_concept_with_budget(
            ontology,
            &concept.frontmatter,
            resolve_link_type,
            ctx.config.finding_budget,
        ) {
            out.push(Finding {
                comparison_detail: Some(violation.kind.baseline_key()),
                code: match &violation.kind {
                    crate::ontology::field_types::ViolationKind::Metadata { code, .. } => {
                        Some(code.clone())
                    }
                    _ => None,
                },
                field_path: match &violation.kind {
                    crate::ontology::field_types::ViolationKind::Metadata { path, .. } => {
                        Some(path.clone())
                    }
                    _ => None,
                },
                rule: RULE.to_string(),
                severity,
                concept: Some(concept.id.0.clone()),
                message: violation.message,
            });
        }
    }
    out
}
