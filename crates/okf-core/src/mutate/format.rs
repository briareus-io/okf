//! Explicit presentation-only formatting; verification and authored values are retained.
use super::transaction::{self, FileChange};
use crate::{
    bundle::{settings, walk::walk_markdown},
    error::{OkfError, Result},
    model::concept::ConceptId,
    parse::{markdown::split_frontmatter, yaml},
};
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use std::path::Path;

pub fn format(root: &Path, concept: Option<&str>, apply: bool) -> Result<Vec<FileChange>> {
    let guard = transaction::writer_lock(root)?;
    let (effective, _) = settings::load_for(root)?;
    let settings = &effective.settings.formatting;
    let paths = if let Some(id) = concept {
        let id = ConceptId::parse(id)?;
        vec![super::edit::id_to_path(root, &id)?
            .strip_prefix(root)
            .map_err(|e| OkfError::Internal(e.to_string()))?
            .to_path_buf()]
    } else {
        walk_markdown(root)?
    };
    let mut changes = Vec::new();
    for path in paths {
        let full = transaction::checked_path(root, &path)?;
        let before = std::fs::read_to_string(full)?;
        let after = format_document(&before, settings)?;
        if before == after {
            continue;
        }
        let violations = crate::check::validate::validate_document(&path.to_string_lossy(), &after);
        if !violations.is_empty() {
            return Err(OkfError::Usage(format!(
                "refusing to format nonconformant document {}",
                path.display()
            )));
        }
        changes.push(FileChange {
            path,
            before: Some(before.into_bytes()),
            after: Some(after.into_bytes()),
        });
    }
    transaction::preflight(root, &changes)?;
    super::gate::check_changes(root, &changes, false)?;
    if apply && !changes.is_empty() {
        transaction::commit_locked(root, &guard, changes.clone())?;
    }
    Ok(changes)
}

pub fn format_document(input: &str, settings: &settings::FormattingSettings) -> Result<String> {
    let (fm, body) = split_frontmatter(input);
    let mut prefix = input[..input.len() - body.len()].to_string();
    if let Some(fm) = fm {
        let map = yaml::parse_frontmatter(&fm)?;
        let mut formatted = yaml::serialize_formatted(&map, &settings.yaml)?;
        if crlf_only(&prefix) {
            formatted = formatted.replace('\n', "\r\n");
        }
        let start = input.find('\n').unwrap() + 1;
        prefix.replace_range(start..start + fm.len(), &formatted);
    }
    let body = if settings.markdown.wrap_prose && settings.markdown.line_width > 0 {
        wrap_markdown_with_newline(
            &body,
            settings.markdown.line_width,
            if crlf_only(&body) || (!body.contains('\n') && crlf_only(input)) {
                "\r\n"
            } else {
                "\n"
            },
        )
    } else {
        body
    };
    Ok(prefix + &body)
}

fn crlf_only(s: &str) -> bool {
    s.contains("\r\n") && !s.replace("\r\n", "").contains('\n')
}

/// Top-level text-only paragraphs are deliberately the first supported subset.
/// Source slices preserve every byte outside eligible paragraphs. Parsed event
/// equivalence prevents wrapping from introducing headings, lists, or other syntax.
pub fn wrap_markdown(body: &str, width: usize) -> String {
    wrap_markdown_with_newline(body, width, if crlf_only(body) { "\r\n" } else { "\n" })
}

fn wrap_markdown_with_newline(body: &str, width: usize, newline: &str) -> String {
    // Unrecognized block dialects can give ordinary-looking following paragraphs
    // special meaning. Without their grammar, retain the entire body, not just
    // the marker line. This intentionally favors preservation over coverage.
    if width == 0
        || body.lines().any(|line| {
            let line = line.trim_start();
            [":::", "!!!", "???", "+++", "%", "{%"]
                .iter()
                .any(|prefix| line.starts_with(prefix))
        })
    {
        return body.to_string();
    }
    let mut depth = 0;
    let mut candidate = None;
    let mut edits = Vec::new();
    for (event, range) in Parser::new_ext(body, Options::all()).into_offset_iter() {
        match event {
            Event::Start(Tag::Paragraph) if depth == 0 => {
                candidate = Some(range);
                depth += 1;
            }
            Event::Start(_) => {
                candidate = None;
                depth += 1;
            }
            Event::End(TagEnd::Paragraph) => {
                depth -= 1;
                if let Some(range) = candidate.take() {
                    let original = &body[range.clone()];
                    // A file can mix frontmatter/body conventions. Honor each
                    // paragraph's existing convention, skipping internally mixed ones.
                    if original.contains("\r\n") && !crlf_only(original) {
                        continue;
                    }
                    let newline = if crlf_only(original) {
                        "\r\n"
                    } else if original.contains('\n') {
                        "\n"
                    } else {
                        newline
                    };
                    let end = original.trim_end_matches(['\r', '\n']);
                    // Unknown extension syntax and nontrivial whitespace stay untouched.
                    if end.contains(['[', ']', '<', '>', '`', '$', '{', '}', '|', '\\', '\t'])
                        || end.lines().any(|l| {
                            l.starts_with(char::is_whitespace) || l.ends_with(char::is_whitespace)
                        })
                    {
                        continue;
                    }
                    let flat = end.replace("\r\n", "\n").replace('\n', " ");
                    if flat.contains("  ") || flat.chars().any(|c| c.is_whitespace() && c != ' ') {
                        continue;
                    }
                    let wrapped = yaml::wrap_words(&flat, width).join(newline);
                    let replacement = wrapped + &original[end.len()..];
                    edits.push((range, replacement));
                }
            }
            Event::End(_) => {
                depth -= 1;
            }
            Event::Text(_) | Event::SoftBreak => {}
            _ => {
                candidate = None;
            }
        }
    }
    let mut result = body.to_string();
    for (range, replacement) in edits.into_iter().rev() {
        result.replace_range(range, &replacement);
    }
    if signature(body) == signature(&result) {
        result
    } else {
        body.to_string()
    }
}

fn signature(s: &str) -> Vec<Event<'static>> {
    let mut events = Vec::new();
    let mut text = String::new();
    for event in Parser::new_ext(s, Options::all()) {
        match event {
            Event::Text(t) => text.push_str(&t),
            Event::SoftBreak => text.push(' '),
            other => {
                if !text.is_empty() {
                    events.push(Event::Text(std::mem::take(&mut text).into()));
                }
                events.push(other.into_static());
            }
        }
    }
    if !text.is_empty() {
        events.push(Event::Text(text.into()));
    }
    events
}
