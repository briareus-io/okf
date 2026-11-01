//! Explicit normalization of body links to their current document-relative targets.
//!
//! This is an opt-in migration, not a safe repair inferred from a missing legacy target.
//! Only destination byte ranges change; YAML, labels, titles and surrounding Markdown
//! remain exact. Unknown or ambiguous destination syntax is left for review.

use std::path::{Path, PathBuf};

use pulldown_cmark::{Event, LinkType, Parser, Tag};

use crate::bundle::walk::walk_markdown;
use crate::error::Result;
use crate::model::link::{classify, LinkKind};
use crate::parse::markdown::split_frontmatter;
use crate::query::artifact::{ArtifactKind, ArtifactResolver};

use super::transaction::{commit_locked, writer_lock, FileChange};

#[derive(Debug, Clone)]
pub struct NormalizedLink {
    pub path: PathBuf,
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone)]
pub struct NormalizationResult {
    pub changes: Vec<FileChange>,
    pub links: Vec<NormalizedLink>,
    pub applied: bool,
}

/// Preview or publish explicit body-link normalization while holding the writer lock.
/// The caller must opt in to current Markdown semantics; no legacy intent is inferred.
pub fn normalize_relative_links(root: &Path, apply: bool) -> Result<NormalizationResult> {
    let guard = writer_lock(root)?;
    let resolver = ArtifactResolver::new(root);
    let mut result = NormalizationResult {
        changes: Vec::new(),
        links: Vec::new(),
        applied: false,
    };
    for path in walk_markdown(root)? {
        let before = std::fs::read(root.join(&path))?;
        let text = std::str::from_utf8(&before).map_err(|e| {
            crate::error::OkfError::Usage(format!("{}: invalid UTF-8: {e}", path.display()))
        })?;
        let (_, body) = split_frontmatter(text);
        let body_offset = text.len() - body.len();
        let from = format!("/{}", path.with_extension("").to_string_lossy());
        let mut edits = Vec::new();
        let parser = Parser::new(&body);
        for (_, definition) in parser.reference_definitions().iter() {
            let span = &body[definition.span.clone()];
            if let Some(colon) = span.find("]:") {
                if let Some((start, end)) = destination_span(span, colon + 2) {
                    if &span[start..end] == definition.dest.as_ref() {
                        if let Some(to) = normalize_target(&resolver, &from, &definition.dest) {
                            edits.push((
                                definition.span.start + start,
                                definition.span.start + end,
                                to,
                            ));
                        }
                    }
                }
            }
        }
        for (event, range) in parser.into_offset_iter() {
            let Event::Start(
                Tag::Link {
                    dest_url,
                    link_type: LinkType::Inline,
                    ..
                }
                | Tag::Image {
                    dest_url,
                    link_type: LinkType::Inline,
                    ..
                },
            ) = event
            else {
                continue;
            };
            let Some(to) = normalize_target(&resolver, &from, &dest_url) else {
                continue;
            };
            let span = &body[range.clone()];
            // Locate the destination using the parsed link's full span. Checking the
            // remaining title/closing delimiter avoids matching text inside a label.
            for (open, _) in span.match_indices("](") {
                let Some((start, end)) = destination_span(span, open + 2) else {
                    continue;
                };
                if &span[start..end] != dest_url.as_ref() {
                    continue;
                }
                let after = if span.as_bytes().get(end) == Some(&b'>') {
                    end + 1
                } else {
                    end
                };
                if !valid_inline_tail(&span[after..]) {
                    continue;
                }
                edits.push((range.start + start, range.start + end, to));
                break;
            }
        }
        if edits.is_empty() {
            continue;
        }
        edits.sort_by_key(|(start, _, _)| *start);
        edits.dedup_by(|a, b| a.0 == b.0 && a.1 == b.1);
        let mut after = text.to_owned();
        for (start, end, to) in edits.into_iter().rev() {
            result.links.push(NormalizedLink {
                path: path.clone(),
                from: body[start..end].to_owned(),
                to: to.clone(),
            });
            after.replace_range(body_offset + start..body_offset + end, &to);
        }
        let violations = crate::check::validate::validate_document(&path.to_string_lossy(), &after);
        if !violations.is_empty() {
            return Err(crate::error::OkfError::Usage(format!(
                "cannot normalize nonconformant document {}: {}",
                path.display(),
                violations
                    .iter()
                    .map(|v| v.message.as_str())
                    .collect::<Vec<_>>()
                    .join("; ")
            )));
        }
        result.changes.push(FileChange {
            path,
            before: Some(before),
            after: Some(after.into_bytes()),
        });
    }
    result
        .links
        .sort_by(|a, b| (&a.path, &a.from, &a.to).cmp(&(&b.path, &b.from, &b.to)));
    if apply && !result.changes.is_empty() {
        commit_locked(root, &guard, result.changes.clone())?;
        result.applied = true;
    }
    Ok(result)
}

fn normalize_target(resolver: &ArtifactResolver, from: &str, raw: &str) -> Option<String> {
    if classify(raw) != LinkKind::Bare {
        return None;
    }
    let suffix_start = raw.find(['#', '?']).unwrap_or(raw.len());
    let core = &raw[..suffix_start];
    let mut target = resolver.resolve(Some(from), raw);
    // Suffix-free concept IDs may coexist with same-stem directories. Prefer the
    // actual Markdown document, but only after checking its containment separately.
    if (target.kind == ArtifactKind::Missing
        || (target.kind == ArtifactKind::Blocked
            && target.message.as_deref() == Some("resolved resource is not a file")))
        && Path::new(core).extension().is_none()
    {
        let document = resolver.resolve(Some(from), &format!("{core}.md"));
        if document.kind == ArtifactKind::Concept {
            target = document;
        }
    }
    if !matches!(
        target.kind,
        ArtifactKind::Concept
            | ArtifactKind::Artifact
            | ArtifactKind::Reserved
            | ArtifactKind::Support
    ) {
        return None;
    }
    let path = target.path?;
    let replacement = format!(
        "/{}{}",
        path.to_string_lossy().replace('\\', "/"),
        &raw[suffix_start..]
    );
    (replacement != raw).then_some(replacement)
}

/// Return the raw destination range, excluding optional angle brackets.
fn destination_span(span: &str, mut start: usize) -> Option<(usize, usize)> {
    let bytes = span.as_bytes();
    while bytes.get(start).is_some_and(u8::is_ascii_whitespace) {
        start += 1;
    }
    if bytes.get(start) == Some(&b'<') {
        start += 1;
        let end = span[start..].find('>')? + start;
        return Some((start, end));
    }
    let mut end = start;
    let mut depth = 0;
    while let Some(&byte) = bytes.get(end) {
        match byte {
            b'\\' => {
                end += 1;
                if end >= bytes.len() {
                    return None;
                }
            }
            b'(' => depth += 1,
            b')' if depth == 0 => break,
            b')' => depth -= 1,
            b if b.is_ascii_whitespace() => break,
            _ => {}
        }
        end += 1;
    }
    (end > start && depth == 0).then_some((start, end))
}

fn valid_inline_tail(tail: &str) -> bool {
    if tail == ")" {
        return true;
    }
    if !tail.starts_with(char::is_whitespace) {
        return false;
    }
    let tail = tail.trim_start();
    if tail == ")" {
        return true;
    }
    let Some(open) = tail.chars().next() else {
        return false;
    };
    let close = match open {
        '\'' | '"' => open,
        '(' => ')',
        _ => return false,
    };
    // Consume exactly one title. Looking only at its first and last quote can
    // mistake an inner image's destination for the enclosing link's destination.
    let bytes = tail.as_bytes();
    let mut index = 1;
    let mut depth = 0;
    while let Some(&byte) = bytes.get(index) {
        if byte == b'\\' {
            index += 2;
            continue;
        }
        if open == '(' && byte == b'(' {
            depth += 1;
        } else if byte == close as u8 {
            if depth == 0 {
                return tail[index + 1..].trim() == ")";
            }
            depth -= 1;
        }
        index += 1;
    }
    false
}
