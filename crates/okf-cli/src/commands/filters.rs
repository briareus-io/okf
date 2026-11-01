//! Explicit query presets and per-occurrence predicates; never used by integrity guards.
use crate::{cli::EdgeFilterArgs, output};
use okf_core::{
    error::{OkfError, Result},
    graph::filter::{CompiledFilters, EdgeFilters},
};
use serde_json::json;
use std::path::Path;
pub fn requested(a: &EdgeFilterArgs) -> bool {
    a.preset.is_some() || !a.clear_filter.is_empty() || !explicit(a).is_empty()
}
pub fn explicit(a: &EdgeFilterArgs) -> EdgeFilters {
    EdgeFilters {
        kind: a.kind.clone(),
        inverse_kind: a.inverse_kind.clone(),
        related_type: a.related_type.clone(),
        rule: a.rule.clone(),
        edge: a.edge_filter.clone(),
        related: a.related_filter.clone(),
    }
}
pub fn resolve(
    root: &Path,
    operation: &str,
    a: &EdgeFilterArgs,
) -> Result<(EdgeFilters, CompiledFilters, bool)> {
    let (settings, _) = okf_core::bundle::settings::load_for(root)?;
    let (base, details) = if let Some(name) = &a.preset {
        let preset = settings
            .settings
            .query_presets
            .get(name)
            .ok_or_else(|| OkfError::Usage(format!("unknown query preset {name:?}")))?;
        if !preset.operations.iter().any(|op| op == operation) {
            return Err(OkfError::Usage(format!(
                "preset {name:?} does not support {operation}"
            )));
        }
        (preset.filters.clone(), preset.details)
    } else {
        (EdgeFilters::default(), false)
    };
    let filters = base.overlay(&explicit(a), &a.clear_filter)?;
    let compiled = filters.compile()?;
    Ok((filters, compiled, details))
}
pub fn report(filters: &EdgeFilters, preset: Option<&str>, json_output: bool) -> Result<()> {
    if json_output {
        output::print_line(
            &json!({"kind":"query-filters","filters":filters,"preset":preset,"origin":if preset.is_some(){"explicit-preset-and-arguments"}else{"arguments"}}),
        )
    } else {
        eprintln!(
            "filters: {}{}",
            serde_json::to_string(filters).unwrap_or_default(),
            preset.map(|p| format!(" (preset {p})")).unwrap_or_default()
        );
        Ok(())
    }
}
pub fn from_json(v: &serde_json::Value) -> Result<EdgeFilterArgs> {
    #[derive(serde::Deserialize, Default)]
    #[serde(default, deny_unknown_fields)]
    struct Query {
        kind: Vec<String>,
        inverse_kind: Vec<String>,
        related_type: Vec<String>,
        rule: Vec<String>,
        edge_filter: Vec<String>,
        related_filter: Vec<String>,
        preset: Option<String>,
        clear_filter: Vec<String>,
    }
    let q: Query = serde_json::from_value(v.clone())
        .map_err(|e| OkfError::Usage(format!("invalid query options: {e}")))?;
    Ok(EdgeFilterArgs {
        kind: q.kind,
        inverse_kind: q.inverse_kind,
        related_type: q.related_type,
        rule: q.rule,
        edge_filter: q.edge_filter,
        related_filter: q.related_filter,
        preset: q.preset,
        clear_filter: q.clear_filter,
    })
}
pub fn resolve_primary(
    root: &Path,
    operation: &str,
    preset: Option<&str>,
    explicit: &okf_core::bundle::settings::PrimaryQueryPreset,
    clear: &[String],
) -> Result<okf_core::bundle::settings::PrimaryQueryPreset> {
    for category in clear {
        if !matches!(
            category.as_str(),
            "type" | "tag" | "text" | "field" | "facet-filter"
        ) {
            return Err(OkfError::Usage(format!(
                "unknown filter category {category:?}"
            )));
        }
    }
    let mut query = if let Some(name) = preset {
        let settings = okf_core::bundle::settings::load_for(root)?.0;
        let p = settings
            .settings
            .query_presets
            .get(name)
            .ok_or_else(|| OkfError::Usage(format!("unknown query preset {name:?}")))?;
        if !p.operations.iter().any(|op| op == operation) {
            return Err(OkfError::Usage(format!(
                "preset {name:?} does not support {operation}"
            )));
        }
        p.query.clone().unwrap_or_default()
    } else {
        Default::default()
    };
    for (name, input, out) in [
        ("type", &explicit.type_, &mut query.type_),
        ("tag", &explicit.tag, &mut query.tag),
    ] {
        if clear.iter().any(|s| s == name) {
            if input.is_some() {
                return Err(OkfError::Usage(format!(
                    "cannot supply and clear filter category {name}"
                )));
            }
            *out = None;
        } else if input.is_some() {
            *out = input.clone();
        }
    }
    for (name, input, out) in [
        ("text", &explicit.text, &mut query.text),
        ("field", &explicit.field, &mut query.field),
        (
            "facet-filter",
            &explicit.facet_filter,
            &mut query.facet_filter,
        ),
    ] {
        if clear.iter().any(|s| s == name) {
            if !input.is_empty() {
                return Err(OkfError::Usage(format!(
                    "cannot supply and clear filter category {name}"
                )));
            }
            out.clear();
        } else if !input.is_empty() {
            *out = input.clone();
        }
    }
    query.validate()?;
    Ok(query)
}
pub fn apply_primary(
    root: &Path,
    operation: &str,
    args: &mut crate::cli::SearchArgs,
    json_output: bool,
) -> Result<()> {
    let explicit = okf_core::bundle::settings::PrimaryQueryPreset {
        type_: args.type_.clone(),
        tag: args.tag.clone(),
        text: args.text.clone(),
        field: args.field.clone(),
        facet_filter: args.facet_filter.clone(),
    };
    let q = resolve_primary(
        root,
        operation,
        args.preset.as_deref(),
        &explicit,
        &args.clear_filter,
    )?;
    if args.preset.is_some() || !args.clear_filter.is_empty() {
        if json_output {
            output::print_line(
                &json!({"kind":"query-filters","query":q,"preset":args.preset,"origin":"explicit-preset-and-arguments"}),
            )?
        } else {
            eprintln!(
                "primary filters: {}",
                serde_json::to_string(&q).unwrap_or_default()
            )
        }
    }
    args.type_ = q.type_;
    args.tag = q.tag;
    args.text = q.text;
    args.field = q.field;
    args.facet_filter = q.facet_filter;
    Ok(())
}
