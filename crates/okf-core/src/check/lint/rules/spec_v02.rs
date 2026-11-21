//! Advisory validation of optional/recommended OKF v0.2 families.

use std::collections::HashSet;

use pulldown_cmark::{Event, Options, Parser, Tag};
use regex::Regex;
use yaml_serde::Value;

use crate::check::lint::{Finding, RuleContext};
use crate::model::standard::{LIFECYCLE_STATUSES, parse_timestamp, valid_actor};

pub const RULE: &str = "okf-v02";

pub fn run(ctx: &RuleContext<'_>) -> Vec<Finding> {
    let Some(severity) = ctx.config.spec_v02 else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for concept in &ctx.bundle.concepts {
        let fm = &concept.frontmatter;
        let mut add = |subject: &str, message: String| {
            out.push(Finding {
                comparison_detail: Some(subject.into()),
                code: None,
                field_path: None,
                rule: RULE.to_string(),
                severity,
                concept: Some(concept.id.0.clone()),
                message,
            });
        };

        if fm.get("tags").is_some_and(|v| !v.is_sequence()) {
            add("tags", "`tags` should be a YAML list".to_string());
        }
        if concept
            .description()
            .is_some_and(|d| d.contains(['\n', '\r']))
        {
            add(
                "description",
                "`description` should be one line".to_string(),
            );
        }
        if let Some(status) = fm.get_str("status")
            && !LIFECYCLE_STATUSES.contains(&status)
        {
            add(
                "status",
                format!("invalid status {status:?}: expected draft, stable, or deprecated"),
            );
        }
        check_timestamp(fm.get("stale_after"), "stale_after", &mut add);
        if let Some(generated) = fm.get("generated") {
            match generated.as_mapping() {
                Some(map) => {
                    match map.get("by").and_then(Value::as_str) {
                        Some(by) if valid_actor(by) => {}
                        _ => add(
                            "generated.by",
                            "`generated.by` must use the OKF actor convention".to_string(),
                        ),
                    }
                    check_timestamp(map.get("at"), "generated.at", &mut add);
                }
                None => add("generated", "`generated` must be a mapping".to_string()),
            }
        }
        if let Some(verified) = fm.get("verified") {
            let entries: Vec<&Value> = match verified {
                Value::Sequence(seq) => seq.iter().collect(),
                Value::Mapping(_) => vec![verified],
                _ => {
                    add(
                        "verified",
                        "`verified` must be a mapping or list of mappings".to_string(),
                    );
                    Vec::new()
                }
            };
            for (i, entry) in entries.into_iter().enumerate() {
                let Some(map) = entry.as_mapping() else {
                    add(
                        &format!("verified[{i}]"),
                        format!("verified[{i}] must be a mapping"),
                    );
                    continue;
                };
                match map.get("by").and_then(Value::as_str) {
                    Some(by) if valid_actor(by) => {}
                    _ => add(
                        &format!("verified[{i}].by"),
                        format!("verified[{i}].by must use the OKF actor convention"),
                    ),
                }
                check_required_timestamp(map.get("at"), &format!("verified[{i}].at"), &mut add);
            }
        }

        let mut source_ids = HashSet::new();
        let mut any_usage_count = false;
        if let Some(sources) = fm.get("sources") {
            match sources.as_sequence() {
                Some(entries) => {
                    for (i, entry) in entries.iter().enumerate() {
                        let Some(map) = entry.as_mapping() else {
                            add(
                                &format!("sources[{i}]"),
                                format!("sources[{i}] must be a mapping"),
                            );
                            continue;
                        };
                        if map
                            .get("resource")
                            .and_then(Value::as_str)
                            .is_none_or(|s| s.trim().is_empty())
                        {
                            add(
                                &format!("sources[{i}].resource"),
                                format!("sources[{i}].resource is required"),
                            );
                        }
                        if let Some(id) = map.get("id").and_then(Value::as_str)
                            && !source_ids.insert(id.to_string())
                        {
                            add(
                                &format!("duplicate-source:{id}"),
                                format!("duplicate sources[].id {id:?}"),
                            );
                        }
                        check_timestamp(
                            map.get("last_modified"),
                            &format!("sources[{i}].last_modified"),
                            &mut add,
                        );
                        if map.get("usage_count").is_some() {
                            any_usage_count = true;
                            if !map.get("usage_count").is_some_and(|v| v.as_u64().is_some()) {
                                add(
                                    &format!("sources[{i}].usage_count"),
                                    format!(
                                        "sources[{i}].usage_count must be a non-negative integer"
                                    ),
                                );
                            }
                        }
                        if let Some(window) = map.get("usage_window") {
                            check_window(window, &format!("sources[{i}].usage_window"), &mut add);
                        }
                    }
                }
                None => add("sources", "`sources` must be a list".to_string()),
            }
        }
        if let Some(window) = fm.get("usage_window") {
            check_window(window, "usage_window", &mut add);
        } else if any_usage_count {
            add(
                "usage-window-missing",
                "usage_count should be framed by a shared or per-source usage_window".to_string(),
            );
        }
        for id in defined_footnote_references(&concept.body) {
            if !source_ids.contains(&id) {
                add(
                    &format!("footnote:{id}"),
                    format!("footnote [^{id}] has no matching sources[].id"),
                );
            }
        }

        if concept.concept_type() == Some("Attested Computation") {
            check_computation(concept, &mut add);
        }
    }
    out
}

/// Only defined Markdown footnotes can attribute a claim. Parser events exclude code spans
/// and blocks; collecting labels first also avoids reporting the same missing source twice.
fn defined_footnote_references(body: &str) -> Vec<String> {
    let parser = Parser::new_ext(body, Options::ENABLE_FOOTNOTES);
    let mut definitions = HashSet::new();
    let mut references = Vec::new();
    let mut seen = HashSet::new();
    for event in parser {
        match event {
            Event::Start(Tag::FootnoteDefinition(label)) => {
                definitions.insert(label.to_string());
            }
            Event::FootnoteReference(label) if seen.insert(label.to_string()) => {
                references.push(label.to_string());
            }
            _ => {}
        }
    }
    references
        .into_iter()
        .filter(|label| definitions.contains(label))
        .collect()
}

/// Shared structural checks for an exact Attested Computation contract.
pub fn computation_issues(concept: &crate::model::concept::Concept) -> Vec<String> {
    let mut issues = Vec::new();
    check_computation(concept, &mut |_, message| issues.push(message));
    issues
}
fn check_computation(concept: &crate::model::concept::Concept, add: &mut impl FnMut(&str, String)) {
    let fm = &concept.frontmatter;
    if fm.get_str("runtime").is_none_or(|s| s.trim().is_empty()) {
        add(
            "runtime",
            "Attested Computation requires non-empty `runtime`".to_string(),
        );
    }
    if fm.get("parameters").is_some_and(|v| !v.is_sequence()) {
        add(
            "parameters",
            "Attested Computation `parameters` must be a list".to_string(),
        );
    }
    if let Some(parameters) = fm.get("parameters").and_then(Value::as_sequence) {
        let mut names = HashSet::new();
        for (i, parameter) in parameters.iter().enumerate() {
            let valid = parameter
                .get("name")
                .and_then(Value::as_str)
                .is_some_and(|s| !s.is_empty())
                && parameter
                    .get("type")
                    .and_then(Value::as_str)
                    .is_some_and(|s| !s.is_empty())
                && parameter.get("required").and_then(Value::as_bool).is_some();
            if !valid {
                add(
                    &format!("parameters[{i}]"),
                    format!("parameters[{i}] must contain name, type, and boolean required"),
                );
            }
            if let Some(name) = parameter.get("name").and_then(Value::as_str)
                && !names.insert(name.to_string())
            {
                add(
                    &format!("duplicate-parameter:{name}"),
                    format!("duplicate computation parameter {name:?}"),
                );
            }
        }
    }
    for family in ["executor", "attester"] {
        if let Some(value) = fm.get(family) {
            if !value.is_mapping() {
                add(family, format!("`{family}` must be a mapping"));
            } else if value
                .get("resource")
                .and_then(Value::as_str)
                .is_none_or(|s| s.trim().is_empty())
            {
                add(
                    &format!("{family}.resource"),
                    format!("`{family}.resource` must be a non-empty path or URI"),
                );
            }
        }
    }
    let (fence_count, fence_has_content) = computation_fences(&concept.body);
    let inline = fence_count > 0;
    let file = fm
        .get_str("computation")
        .is_some_and(|s| !s.trim().is_empty());
    if inline == file {
        add("computation-form", "Attested Computation must use exactly one of inline # Computation fence or `computation` path".to_string());
    }
    if fence_count > 1 {
        add(
            "computation-fence-count",
            "inline # Computation must contain a single fenced code block".to_string(),
        );
    }
    if inline && !fence_has_content {
        add(
            "computation-fence-empty",
            "inline computation fence is empty".to_string(),
        );
    }
}

fn check_timestamp<F>(value: Option<&Value>, name: &str, add: &mut F)
where
    F: FnMut(&str, String),
{
    if let Some(value) = value {
        match value.as_str() {
            Some(raw) if parse_timestamp(raw).is_some() => {}
            _ => add(
                name,
                format!("`{name}` must be an ISO 8601 datetime with explicit UTC offset"),
            ),
        }
    }
}

fn check_required_timestamp<F>(value: Option<&Value>, name: &str, add: &mut F)
where
    F: FnMut(&str, String),
{
    if value.is_none() {
        add(name, format!("`{name}` is required"));
    } else {
        check_timestamp(value, name, add);
    }
}

fn check_window<F>(value: &Value, name: &str, add: &mut F)
where
    F: FnMut(&str, String),
{
    match value.as_mapping() {
        Some(map) => {
            check_required_timestamp(map.get("from"), &format!("{name}.from"), add);
            check_required_timestamp(map.get("to"), &format!("{name}.to"), add);
            if let (Some(from), Some(to)) = (
                map.get("from")
                    .and_then(Value::as_str)
                    .and_then(parse_timestamp),
                map.get("to")
                    .and_then(Value::as_str)
                    .and_then(parse_timestamp),
            ) && from > to
            {
                add(name, format!("`{name}.from` must not be after `{name}.to`"));
            }
        }
        None => add(name, format!("`{name}` must be a {{from,to}} mapping")),
    }
}

pub(crate) fn computation_fences(body: &str) -> (usize, bool) {
    let mut under = false;
    let mut in_fence = false;
    let mut count = 0;
    let mut has_content = false;
    for line in body.lines() {
        let t = line.trim();
        if !in_fence && t.starts_with("# ") {
            under = t.trim_start_matches("# ").trim() == "Computation";
        } else if under && (t.starts_with("```") || t.starts_with("~~~")) {
            if !in_fence {
                count += 1;
            }
            in_fence = !in_fence;
        } else if under && in_fence && !t.is_empty() {
            has_content = true;
        }
    }
    (count, has_content)
}

#[cfg(test)]
mod tests {
    use super::defined_footnote_references;

    #[test]
    fn only_defined_markdown_references_are_source_attributions() {
        let body = "plain [^x] and /job/[^/]+/batch\n\n\
                    inline `from (x)+src/[^']*`\n\n\
                    ```\ngrep \"[^s]\"\n```\n\n\
                    claim[^proof] and another[^proof]\n\n\
                    [^proof]: Evidence.\n";
        assert_eq!(defined_footnote_references(body), vec!["proof"]);
    }

    #[test]
    fn definition_without_reference_is_not_an_attribution() {
        assert!(defined_footnote_references("[^unused]: Evidence.\n").is_empty());
    }
}
