//! Limits on the authored Markdown and YAML, including comments and blank lines.
use super::super::{Finding, Severity};
use crate::bundle::{
    loader::Bundle,
    manifest::{local_path, read_to_string_nofollow},
    settings::{LineScope, LintSettings},
};
use crate::error::Result;
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag};
use std::ops::Range;

fn limit(level: Option<&str>, max: Option<usize>) -> Option<(Severity, usize)> {
    let severity = match level? {
        "info" => Severity::Info,
        "warn" => Severity::Warn,
        "error" => Severity::Error,
        _ => return None,
    };
    Some((severity, max?))
}

pub fn run(bundle: &Bundle, settings: &LintSettings) -> Result<Vec<Finding>> {
    let body_limit = limit(
        settings.body_max_lines.as_deref(),
        settings.body_max_lines_limit,
    );
    let yaml_limit = limit(
        settings.frontmatter_max_lines.as_deref(),
        settings.frontmatter_max_lines_limit,
    );
    let width = limit(
        settings.max_line_length.as_deref(),
        settings.max_line_length_limit,
    );
    let mut findings = Vec::new();
    if body_limit.is_none() && yaml_limit.is_none() && width.is_none() {
        return Ok(findings);
    }
    for concept in &bundle.concepts {
        let source;
        let (yaml, body) = if yaml_limit.is_some() || width.is_some() {
            let relative = format!("{}.md", concept.id.0.trim_start_matches('/'));
            let path = local_path(&bundle.root, &relative)?;
            let text = read_to_string_nofollow(&path).map_err(|e| {
                crate::error::OkfError::io_with(format!("{}: {e}", path.display()), e)
            })?;
            source = crate::parse::markdown::split_frontmatter(&text);
            (source.0.as_deref(), source.1.as_str())
        } else {
            (None, concept.body.as_str())
        };
        let header = yaml.unwrap_or("");
        let header_lines = header.lines().count();
        let mut report = |rule: &str, severity, message| {
            findings.push(Finding {
                comparison_detail: None,
                rule: rule.into(),
                severity,
                message,
                concept: Some(concept.id.0.clone()),
                code: None,
                field_path: None,
            });
        };
        for (rule, section, count, limit) in [
            (
                "body-max-lines",
                "body",
                body.trim_end().lines().count(),
                body_limit,
            ),
            (
                "frontmatter-max-lines",
                "frontmatter",
                header_lines,
                yaml_limit,
            ),
        ] {
            if let Some((severity, max)) = limit
                && count > max
            {
                report(
                    rule,
                    severity,
                    format!("{section} has {count} lines (max {max})"),
                );
            }
        }
        if let Some((severity, max)) = width {
            let (count, first) = overlong_lines(yaml, body, max, settings);
            if count > 0 {
                let summary = if count == 1 {
                    "line exceeds"
                } else {
                    "lines exceed"
                };
                report(
                    "max-line-length",
                    severity,
                    format!(
                        "{count} {summary} {max} characters (first: {})",
                        first.join(", ")
                    ),
                );
            }
        }
    }
    Ok(findings)
}

fn overlong_lines(
    yaml: Option<&str>,
    body: &str,
    max: usize,
    settings: &LintSettings,
) -> (usize, Vec<String>) {
    let header = yaml.unwrap_or("");
    let scope = settings.max_line_length_scope.unwrap_or_default();
    let mut count = 0;
    let mut first = Vec::new();
    let mut record = |number: usize| {
        count += 1;
        if first.len() < 3 {
            first.push(number.to_string());
        }
    };
    if scope != LineScope::Body {
        for (index, line) in header.lines().enumerate() {
            if line.trim_end_matches('\r').chars().count() > max {
                record(index + 2);
            }
        }
    }
    if scope != LineScope::Frontmatter {
        let mut excluded = excluded_ranges(body, settings).into_iter().peekable();
        let first_body = if yaml.is_some() {
            header.lines().count() + 3
        } else {
            1
        };
        let mut offset = 0;
        for (index, raw) in body.split_inclusive('\n').enumerate() {
            let line = raw.trim_end_matches(['\n', '\r']);
            let end = offset + line.len();
            while excluded.peek().is_some_and(|range| range.end <= offset) {
                excluded.next();
            }
            let skip = excluded
                .peek()
                .is_some_and(|r| r.start < end && r.end > offset);
            if !skip
                && line.chars().count() > max
                && !(settings.max_line_length_exclude_links.unwrap_or(true) && bare_url(line))
            {
                record(first_body + index);
            }
            offset += raw.len();
        }
    }
    (count, first)
}

fn bare_url(line: &str) -> bool {
    let text = line.trim();
    (text.starts_with("https://") || text.starts_with("http://"))
        && !text.contains(char::is_whitespace)
}

fn excluded_ranges(body: &str, settings: &LintSettings) -> Vec<Range<usize>> {
    let mut ranges = Vec::new();
    for (event, range) in Parser::new_ext(body, Options::ENABLE_TABLES).into_offset_iter() {
        let skip = match event {
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(_))) => {
                settings.max_line_length_exclude_code_blocks.unwrap_or(true)
            }
            Event::Start(Tag::Table(_)) => settings.max_line_length_exclude_tables.unwrap_or(true),
            Event::Start(Tag::Link { .. })
                if settings.max_line_length_exclude_links.unwrap_or(true) =>
            {
                let before = body[..range.start].trim_end_matches([' ', '\t', '\r']);
                let after = body[range.end..].trim_start_matches([' ', '\t', '\r']);
                !body[range.clone()].contains('\n')
                    && (before.is_empty() || before.ends_with('\n'))
                    && (after.is_empty() || after.starts_with('\n'))
            }
            _ => false,
        };
        if skip {
            ranges.push(range);
        }
    }
    ranges
}
