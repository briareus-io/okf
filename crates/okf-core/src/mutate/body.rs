//! Document-body editing: whole-body replace/append/clear and section-aware edits.
//!
//! All operations take and return the concept **body** (everything after the frontmatter),
//! so the frontmatter and its key order are untouched. Section edits locate a heading via
//! [`find_section`] (matched by GitHub-style slug, so `"Rates"` and `"rates"` are the same
//! target) and splice by `\n`-delimited line, which round-trips exactly for both LF and CRLF
//! bodies. Content is normalized to end with a single trailing newline.
use crate::error::{OkfError, Result};
use crate::parse::markdown::{FenceTracker, find_section, parse_atx, slugify};

/// Replace the entire body with `text` (normalized to a single trailing newline).
pub fn set_body(text: &str) -> String {
    with_trailing_nl(text)
}

/// Append `text` to the body as a new block, separated by a blank line.
pub fn append_body(old: &str, text: &str) -> String {
    let block = text.trim_matches('\n');
    if old.trim_matches('\n').is_empty() {
        return with_trailing_nl(block);
    }
    let mut out = old.trim_end_matches('\n').to_string();
    out.push_str("\n\n");
    out.push_str(block);
    out.push('\n');
    out
}

/// Replace the content of the section under `heading` (the heading line is kept). Errors if no
/// heading matches.
pub fn set_section(body: &str, heading: &str, text: &str) -> Result<String> {
    let span = find_section(body, heading)
        .ok_or_else(|| OkfError::Usage(format!("edit: heading not found: {heading:?}")))?;
    let lines: Vec<&str> = body.split('\n').collect();
    let mut out = lines[..span.content_start].join("\n");
    push_block(&mut out, text);
    push_lines(&mut out, &lines[span.end..]);
    Ok(out)
}

/// Append `text` to the end of the section under `heading`, separated by a blank line. Errors
/// if no heading matches.
pub fn append_section(body: &str, heading: &str, text: &str) -> Result<String> {
    let span = find_section(body, heading)
        .ok_or_else(|| OkfError::Usage(format!("edit: heading not found: {heading:?}")))?;
    let lines: Vec<&str> = body.split('\n').collect();
    // Trim trailing blank lines inside the section so the appended block sits right after
    // the existing content, then re-pad with one blank separator.
    let mut end = span.end;
    while end > span.content_start && lines[end - 1].trim().is_empty() {
        end -= 1;
    }
    let mut out = lines[..end].join("\n");
    push_block(&mut out, text);
    push_lines(&mut out, &lines[span.end..]);
    Ok(out)
}

/// Remove the section under `heading` entirely, including its heading line and nested
/// subsections. Errors if no heading matches.
pub fn remove_section(body: &str, heading: &str) -> Result<String> {
    let span = find_section(body, heading)
        .ok_or_else(|| OkfError::Usage(format!("edit: heading not found: {heading:?}")))?;
    let lines: Vec<&str> = body.split('\n').collect();
    Ok([&lines[..span.heading_line], &lines[span.end..]]
        .concat()
        .join("\n"))
}

/// Append `\n`, a blank line's worth of separation, the trimmed block and a trailing blank
/// line to `out` (the `join("\n")` of `[.., "", block lines.., ""]`).
fn push_block(out: &mut String, text: &str) {
    out.push_str("\n\n");
    out.push_str(text.trim_matches('\n'));
    out.push('\n');
}

/// Append `lines` after `out` as further `\n`-joined lines.
fn push_lines(out: &mut String, lines: &[&str]) {
    for line in lines {
        out.push('\n');
        out.push_str(line);
    }
}

/// Rename a section heading while preserving its level, indentation, and content. Errors if the
/// heading cannot be found or the replacement is empty.
pub fn rename_section(body: &str, heading: &str, new_heading: &str) -> Result<String> {
    if new_heading.trim().is_empty() || new_heading.contains(['\n', '\r']) {
        return Err(OkfError::Usage(
            "edit: replacement section heading must be non-empty and single-line".to_string(),
        ));
    }
    let span = find_section(body, heading)
        .ok_or_else(|| OkfError::Usage(format!("edit: heading not found: {heading:?}")))?;
    let mut lines: Vec<&str> = body.split('\n').collect();
    let old_line = lines[span.heading_line];
    let wanted = slugify(heading);
    let mut fence = FenceTracker::default();
    let matches = lines
        .iter()
        .filter(|line| {
            !fence.skip_line(line)
                && parse_atx(line).is_some_and(|(_, text)| slugify(text) == wanted)
        })
        .count();
    if matches > 1 {
        return Err(OkfError::Usage(format!(
            "edit: heading is ambiguous: {heading:?} matches {matches} sections"
        )));
    }
    let indent = old_line.len() - old_line.trim_start().len();
    let prefix = &old_line[..indent];
    let ending = if old_line.ends_with('\r') { "\r" } else { "" };
    let level = parse_atx(old_line).map_or(span.level, |(level, _)| level);
    let replacement = format!("{prefix}{} {new_heading}{ending}", "#".repeat(level));
    lines[span.heading_line] = &replacement;
    Ok(lines.join("\n"))
}

/// Ensure a single trailing newline (empty stays empty).
fn with_trailing_nl(s: &str) -> String {
    if s.is_empty() {
        String::new()
    } else {
        format!("{}\n", s.trim_end_matches('\n'))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = "# Title\n\nintro\n\n## Rates\n\nold body\n\n## Next\n\ntail\n";

    #[test]
    fn set_body_replaces_and_normalizes_trailing_newline() {
        assert_eq!(set_body("hello"), "hello\n");
        assert_eq!(set_body("hello\n\n\n"), "hello\n");
        assert_eq!(set_body(""), "");
    }

    #[test]
    fn append_body_adds_a_separated_block() {
        assert_eq!(append_body("intro\n", "more"), "intro\n\nmore\n");
        // Appending to an empty body just sets it.
        assert_eq!(append_body("", "first"), "first\n");
    }

    #[test]
    fn set_section_replaces_content_keeps_heading_and_siblings() {
        let out = set_section(DOC, "Rates", "new one\nnew two").unwrap();
        assert!(out.contains("## Rates\n\nnew one\nnew two\n\n## Next"));
        assert!(!out.contains("old body"));
        // Untouched sections survive.
        assert!(out.contains("# Title\n\nintro"));
        assert!(out.contains("## Next\n\ntail"));
    }

    #[test]
    fn set_section_matches_by_slug() {
        let out = set_section(DOC, "rates", "x").unwrap();
        assert!(out.contains("## Rates\n\nx\n"));
    }

    #[test]
    fn append_section_inserts_after_existing_content() {
        let out = append_section(DOC, "Rates", "added").unwrap();
        assert!(out.contains("## Rates\n\nold body\n\nadded\n\n## Next"));
    }

    #[test]
    fn remove_section_drops_heading_and_body() {
        let out = remove_section(DOC, "Rates").unwrap();
        assert!(!out.contains("## Rates"));
        assert!(!out.contains("old body"));
        assert!(out.contains("## Next\n\ntail"));
    }

    #[test]
    fn rename_section_preserves_level_crlf_and_ignores_fenced_headings() {
        let doc = "## Rates\r\n\r\nbody\r\n\r\n```md\r\n## Rates\r\n```\r\n";
        let renamed = rename_section(doc, "Rates", "Pricing").unwrap();
        assert!(renamed.starts_with("## Pricing\r\n"));
        assert!(renamed.contains("## Rates\r\n```"));
    }

    #[test]
    fn rename_section_rejects_ambiguous_headings_and_multiline_names() {
        let doc = "## Rates\n\nfirst\n\n## Rates\n\nsecond\n";
        assert!(rename_section(doc, "Rates", "Pricing").is_err());
        assert!(rename_section("## Rates\n", "Rates", "New\nHeading").is_err());
    }

    #[test]
    fn missing_heading_is_a_usage_error() {
        assert!(set_section(DOC, "nope", "x").is_err());
        assert!(append_section(DOC, "nope", "x").is_err());
        assert!(remove_section(DOC, "nope").is_err());
    }

    #[test]
    fn section_edit_includes_nested_subsections() {
        let doc = "## A\n\nbody\n\n### deep\n\nx\n\n## B\n\ny\n";
        let out = remove_section(doc, "A").unwrap();
        assert!(!out.contains("### deep"));
        assert!(out.contains("## B"));
    }
}
