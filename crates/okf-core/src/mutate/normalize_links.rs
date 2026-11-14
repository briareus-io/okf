//! Explicit normalization of body links to their current document-relative targets.
//!
//! This is an opt-in migration, not a safe repair inferred from a missing legacy target.
//! Only destination byte ranges change; YAML, labels, titles and surrounding Markdown
//! remain exact. Unknown or ambiguous destination syntax is left for review.

use std::borrow::Cow;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use pulldown_cmark::{Event, LinkType, Parser, Tag};

use crate::bundle::walk::walk_markdown;
use crate::error::Result;
use crate::model::link::{LinkKind, classify};
use crate::parse::markdown::split_frontmatter;
use crate::query::artifact::{ArtifactKind, ArtifactResolver};

use super::transaction::{FileChange, commit_locked, writer_lock};

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
            if let Some((start, end)) = find_definition_destination(span, &definition.dest)
                && let Some(to) = normalize_target(&resolver, &from, &definition.dest)
            {
                edits.push((
                    definition.span.start + start,
                    definition.span.start + end,
                    to,
                ));
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
            let span = &body[range.clone()];
            if let Some((start, end)) = find_inline_destination(span, &dest_url) {
                edits.push((range.start + start, range.start + end, to));
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
        encode_destination(&path.to_string_lossy().replace('\\', "/")),
        &raw[suffix_start..]
    );
    (replacement != raw).then_some(replacement)
}

/// Locate the raw destination of an inline link/image inside its full parsed `span`.
/// `dest` is the parser's (unescaped) destination. Checking the title/closing tail avoids
/// matching text inside a label or an inner image.
pub(super) fn find_inline_destination(span: &str, dest: &str) -> Option<(usize, usize)> {
    for (open, _) in span.match_indices("](") {
        let Some((start, end)) = destination_span(span, open + 2) else {
            continue;
        };
        if unescape_backslashes(&span[start..end]) != dest {
            continue;
        }
        let after = if span.as_bytes().get(end) == Some(&b'>') {
            end + 1
        } else {
            end
        };
        if valid_inline_tail(&span[after..]) {
            return Some((start, end));
        }
    }
    None
}

/// Locate the raw destination of a link reference definition within its `span`.
pub(super) fn find_definition_destination(span: &str, dest: &str) -> Option<(usize, usize)> {
    let colon = span.find("]:")?;
    let (start, end) = destination_span(span, colon + 2)?;
    (unescape_backslashes(&span[start..end]) == dest).then_some((start, end))
}

/// Resolve CommonMark backslash escapes and entity/numeric character references, as the
/// parser does for link destinations. Unknown entities stay literal.
fn unescape_backslashes(raw: &str) -> Cow<'_, str> {
    if !raw.contains(['\\', '&']) {
        return raw.into();
    }
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(c) = rest.chars().next() {
        let after = &rest[c.len_utf8()..];
        if c == '\\'
            && let Some(n) = after.chars().next().filter(char::is_ascii_punctuation)
        {
            out.push(n);
            rest = &after[n.len_utf8()..];
            continue;
        }
        if c == '&'
            && let Some((decoded, len)) = decode_entity(after)
        {
            out.push(decoded);
            rest = &after[len..];
            continue;
        }
        out.push(c);
        rest = after;
    }
    out.into()
}

/// Decode one reference body following `&` (e.g. `amp;`, `#38;`, `#x26;`); returns the
/// character and the number of bytes consumed after the ampersand.
fn decode_entity(after: &str) -> Option<(char, usize)> {
    let end = after.find(';')?;
    if end == 0 || end > 10 {
        return None;
    }
    let name = &after[..end];
    let decoded = if let Some(num) = name.strip_prefix('#') {
        let code = match num.strip_prefix(['x', 'X']) {
            Some(hex) if !hex.is_empty() && hex.len() <= 6 => u32::from_str_radix(hex, 16).ok()?,
            Some(_) => return None,
            None if !num.is_empty() && num.len() <= 7 => num.parse().ok()?,
            None => return None,
        };
        if code == 0 {
            '\u{FFFD}'
        } else {
            char::from_u32(code).unwrap_or('\u{FFFD}')
        }
    } else {
        match name {
            "amp" => '&',
            "lt" => '<',
            "gt" => '>',
            "quot" => '"',
            "apos" => '\'',
            "nbsp" => '\u{A0}',
            _ => return None,
        }
    };
    Some((decoded, end + 1))
}

/// Percent-encode only what would make a bare CommonMark destination invalid: whitespace,
/// control characters, `<`, `>` and unbalanced parentheses. Ordinary paths are unchanged.
pub(super) fn encode_destination(dest: &str) -> String {
    let mut depth = 0i32;
    let mut balanced = true;
    for c in dest.chars() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                balanced &= depth >= 0;
            }
            _ => {}
        }
    }
    balanced &= depth == 0;
    let mut out = String::with_capacity(dest.len());
    for c in dest.chars() {
        let escape = c.is_whitespace()
            || c.is_control()
            || matches!(c, '<' | '>')
            || (!balanced && matches!(c, '(' | ')'));
        if escape {
            let mut buf = [0; 4];
            for byte in c.encode_utf8(&mut buf).bytes() {
                let _ = write!(out, "%{byte:02X}");
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Decode `%XX` escapes; results that are not valid UTF-8 leave the input untouched.
pub(super) fn percent_decode(raw: &str) -> Cow<'_, str> {
    if !raw.contains('%') {
        return raw.into();
    }
    let bytes = raw.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = |b: u8| (b as char).to_digit(16);
            if let (Some(hi), Some(lo)) = (hex(bytes[i + 1]), hex(bytes[i + 2])) {
                out.push((hi * 16 + lo) as u8);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    match String::from_utf8(out) {
        Ok(decoded) => decoded.into(),
        Err(_) => raw.into(),
    }
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

#[cfg(test)]
mod entity_tests {
    use super::*;

    #[test]
    fn destinations_decode_entities_and_leave_unknown_literal() {
        assert_eq!(unescape_backslashes("a&amp;b.md"), "a&b.md");
        assert_eq!(unescape_backslashes("a&#38;b&#x26;c"), "a&b&c");
        assert_eq!(unescape_backslashes("&lt;&gt;&quot;&apos;"), "<>\"'");
        assert_eq!(unescape_backslashes("a&nbsp;b"), "a\u{A0}b");
        assert_eq!(unescape_backslashes("a&bogus;b&amp"), "a&bogus;b&amp");
        assert_eq!(unescape_backslashes("\\&amp;"), "&amp;");
        assert_eq!(unescape_backslashes("x\\_y"), "x_y");
    }
}
