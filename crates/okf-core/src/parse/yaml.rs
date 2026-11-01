//! Lossless YAML load (order-preserving mapping) and serialize.
use crate::error::{OkfError, Result};
use indexmap::IndexMap;
use serde_yaml::Value;

/// Parse a frontmatter block into an order-preserving map. An empty/blank block is an
/// empty map (not an error).
pub fn parse_frontmatter(yaml: &str) -> Result<IndexMap<String, Value>> {
    if yaml.trim().is_empty() {
        return Ok(IndexMap::new());
    }
    serde_yaml::from_str::<IndexMap<String, Value>>(yaml).map_err(|e| OkfError::Yaml(e.to_string()))
}

/// Serialize a frontmatter map back to YAML text, preserving key order. Returns an empty
/// string for an empty map (so no `{}` noise is emitted).
pub fn serialize_frontmatter(map: &IndexMap<String, Value>) -> Result<String> {
    if map.is_empty() {
        return Ok(String::new());
    }
    serde_yaml::to_string(map).map_err(|e| OkfError::Internal(e.to_string()))
}

/// Render readable scalars, with a whole-document value equivalence guard.
pub fn serialize_formatted(
    map: &IndexMap<String, Value>,
    settings: &crate::bundle::settings::YamlFormatting,
) -> Result<String> {
    use crate::bundle::settings::YamlStyle;
    let legacy = serialize_frontmatter(map)?;
    if map.is_empty() || settings.style == YamlStyle::Legacy {
        return Ok(legacy);
    }
    let value = serde_yaml::to_value(map).map_err(|e| OkfError::Internal(e.to_string()))?;
    let candidate = render_value(&value, settings.line_width, 0)?;
    if serde_yaml::from_str::<Value>(&candidate).ok().as_ref() == Some(&value) {
        Ok(candidate)
    } else {
        // Exotic keys/tags or whitespace must never change a value for presentation.
        Ok(legacy)
    }
}

fn render_value(value: &Value, width: usize, indent: usize) -> Result<String> {
    let mut out = String::new();
    match value {
        Value::Mapping(map) if !map.is_empty() => {
            for (key, value) in map {
                let Value::String(_) = key else {
                    return serde_yaml::to_string(value)
                        .map_err(|e| OkfError::Internal(e.to_string()));
                };
                let key =
                    serde_yaml::to_string(key).map_err(|e| OkfError::Internal(e.to_string()))?;
                append_value(
                    &mut out,
                    &format!("{}:", key.trim_end_matches('\n')),
                    value,
                    width,
                    indent,
                )?;
            }
        }
        Value::Sequence(items) if !items.is_empty() => {
            for value in items {
                append_value(&mut out, "-", value, width, indent)?;
            }
        }
        _ => {
            let scalar = scalar(
                value,
                if width == 0 {
                    0
                } else {
                    width.saturating_sub(indent).max(1)
                },
            )?;
            for line in scalar.split_inclusive('\n') {
                out.push_str(&" ".repeat(indent));
                out.push_str(line);
            }
        }
    }
    Ok(out)
}

fn append_value(
    out: &mut String,
    prefix: &str,
    value: &Value,
    width: usize,
    indent: usize,
) -> Result<()> {
    out.push_str(&" ".repeat(indent));
    out.push_str(prefix);
    if matches!(value, Value::Mapping(m) if !m.is_empty())
        || matches!(value, Value::Sequence(s) if !s.is_empty())
    {
        out.push('\n');
        let child_indent = if matches!(value, Value::Sequence(_)) && prefix != "-" {
            indent
        } else {
            indent + 2
        };
        let child = render_value(value, width, child_indent)?;
        if prefix == "-" {
            out.pop();
            out.push(' ');
            out.push_str(
                child
                    .strip_prefix(&" ".repeat(child_indent))
                    .unwrap_or(&child),
            );
        } else {
            out.push_str(&child);
        }
    } else {
        out.push(' ');
        let scalar = scalar(
            value,
            if width == 0 {
                0
            } else {
                width
                    .saturating_sub(indent + prefix.chars().count() + 1)
                    .max(1)
            },
        )?;
        for (i, line) in scalar.split_inclusive('\n').enumerate() {
            if i > 0 {
                out.push_str(&" ".repeat(indent));
            }
            out.push_str(line);
        }
    }
    Ok(())
}

fn scalar(value: &Value, available: usize) -> Result<String> {
    if let Value::String(s) = value {
        if available > 0 && s.chars().count() > available && safe_words(s) {
            let lines = wrap_words(s, available);
            if lines.len() > 1 {
                return Ok(format!(
                    ">-\n{}\n",
                    lines
                        .iter()
                        .map(|line| format!("  {line}"))
                        .collect::<Vec<_>>()
                        .join("\n")
                ));
            }
        }
    }
    serde_yaml::to_string(value).map_err(|e| OkfError::Internal(e.to_string()))
}

/// Only single ASCII-space boundaries are eligible for folding.
fn safe_words(s: &str) -> bool {
    !s.is_empty()
        && !s.starts_with(' ')
        && !s.ends_with(' ')
        && !s.contains("  ")
        && !s
            .chars()
            .any(|c| c.is_control() || (c.is_whitespace() && c != ' '))
}

pub(crate) fn wrap_words(s: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    let mut columns = 0;
    for word in s.split(' ') {
        let len = word.chars().count();
        if !line.is_empty() && columns + 1 + len > width {
            lines.push(std::mem::take(&mut line));
            columns = 0;
        }
        if !line.is_empty() {
            line.push(' ');
            columns += 1;
        }
        line.push_str(word);
        columns += len;
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}
