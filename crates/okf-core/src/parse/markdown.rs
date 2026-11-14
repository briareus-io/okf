//! Split frontmatter/body; build the heading tree; extract sections.
//!
//! Beyond the frontmatter/body split, this module hosts the shared ATX-heading primitives
//! (`parse_atx`, `slugify`) used by both `markdown-heading` fingerprints and section-aware
//! `okf edit`, plus [`find_section`], which locates a heading's line span for lossless
//! splicing, and the shared [`FenceTracker`] for fenced code blocks.

/// Split a concept file's leading `---\n … \n---\n` frontmatter block from its body.
///
/// Returns `(Some(frontmatter_text), body)` when a well-formed leading block is present,
/// otherwise `(None, whole_content)`. The body is preserved verbatim (everything after the
/// closing delimiter line's newline). Frontmatter text excludes both delimiter lines.
/// A leading UTF-8 BOM is tolerated when detecting the block; it belongs to neither the
/// frontmatter nor the body (callers that rewrite the file must carry it over themselves).
pub fn split_frontmatter(content: &str) -> (Option<String>, String) {
    let text = content.strip_prefix('\u{feff}').unwrap_or(content);
    // The block must open on the very first line.
    let first_line_end = text.find('\n').map_or(text.len(), |i| i + 1);
    let first_line = &text[..first_line_end];
    if !is_delimiter(first_line) {
        return (None, content.to_string());
    }

    let open_end = first_line_end;
    let mut pos = open_end;
    while pos < text.len() {
        let nl = text[pos..].find('\n').map_or(text.len(), |i| pos + i + 1);
        let line = &text[pos..nl];
        if is_delimiter(line) {
            let fm = text[open_end..pos].to_string();
            let body = text[nl..].to_string();
            return (Some(fm), body);
        }
        pos = nl;
    }

    // No closing delimiter — be permissive and treat everything as body.
    (None, content.to_string())
}

fn is_delimiter(line: &str) -> bool {
    let line = line.strip_suffix('\n').unwrap_or(line);
    line.strip_suffix('\r').unwrap_or(line) == "---"
}

/// A located ATX-heading section within a body, expressed in `body.split('\n')` line
/// indices so callers can splice losslessly (rejoining with `\n` is exact).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SectionSpan {
    /// ATX level (1–6) of the matched heading.
    pub level: usize,
    /// Index of the heading line itself.
    pub heading_line: usize,
    /// Index of the first content line after the heading (`heading_line + 1`).
    pub content_start: usize,
    /// Exclusive end index: the next equal-or-shallower heading, or the line count.
    pub end: usize,
}

/// Tracks fenced code blocks line by line, per CommonMark: an opener is 3+ backticks or
/// tildes (indented at most 3 spaces; a backtick opener's info string has no backticks), and
/// the closer must use the same character, be at least as long, and carry no info string.
#[derive(Debug, Default)]
pub(crate) struct FenceTracker {
    open: Option<(u8, usize)>,
}

impl FenceTracker {
    /// Feed the next line. Returns `true` when the line is a fence delimiter or lies inside
    /// a fenced block (i.e. it must not be interpreted as Markdown structure).
    pub(crate) fn skip_line(&mut self, line: &str) -> bool {
        let line = line.strip_suffix('\r').unwrap_or(line);
        let indent = line.len() - line.trim_start_matches(' ').len();
        let rest = &line[indent..];
        let run = fence_run(rest);
        match self.open {
            Some((ch, len)) => {
                if indent <= 3
                    && run.is_some_and(|(c, n)| c == ch && n >= len)
                    && rest[run.map_or(0, |(_, n)| n)..].trim().is_empty()
                {
                    self.open = None;
                }
                true
            }
            None => match run {
                Some((ch, n)) if indent <= 3 && !(ch == b'`' && rest[n..].contains('`')) => {
                    self.open = Some((ch, n));
                    true
                }
                _ => false,
            },
        }
    }
}

/// Leading run of 3+ identical fence characters, as `(char, length)`.
fn fence_run(text: &str) -> Option<(u8, usize)> {
    let first = *text.as_bytes().first()?;
    if first != b'`' && first != b'~' {
        return None;
    }
    let len = text.bytes().take_while(|&b| b == first).count();
    (len >= 3).then_some((first, len))
}

/// Locate the first heading whose slug equals `slugify(heading)`, honoring fenced code blocks
/// (ATX headings inside ```` ``` ````/`~~~` fences are ignored). Returns line indices into
/// `body.split('\n')`; `None` if no heading matches. Nested subsections are part of the span.
pub(crate) fn find_section(body: &str, heading: &str) -> Option<SectionSpan> {
    let want = slugify(heading);
    let lines: Vec<&str> = body.split('\n').collect();
    let mut fence = FenceTracker::default();
    for (i, line) in lines.iter().enumerate() {
        if fence.skip_line(line) {
            continue;
        }
        if let Some((level, htext)) = parse_atx(line)
            && slugify(htext) == want
        {
            let end = section_end(&lines, i + 1, level);
            return Some(SectionSpan {
                level,
                heading_line: i,
                content_start: i + 1,
                end,
            });
        }
    }
    None
}

/// The exclusive end of a section that starts after `from`: the first equal-or-shallower ATX
/// heading (fences honored), or `lines.len()`.
fn section_end(lines: &[&str], from: usize, level: usize) -> usize {
    let mut fence = FenceTracker::default();
    for (j, line) in lines.iter().enumerate().skip(from) {
        if fence.skip_line(line) {
            continue;
        }
        if let Some((lvl, _)) = parse_atx(line)
            && lvl <= level
        {
            return j;
        }
    }
    lines.len()
}

/// Parse an ATX heading line, returning `(level, text)`. Requires 1–6 leading `#` followed by
/// whitespace (or an otherwise-empty heading); the trailing ATX closing `#` run is trimmed.
pub(crate) fn parse_atx(line: &str) -> Option<(usize, &str)> {
    let trimmed = line.trim_start();
    let hashes = trimmed.chars().take_while(|&c| c == '#').count();
    if hashes == 0 || hashes > 6 {
        return None;
    }
    let rest = &trimmed[hashes..];
    if rest.is_empty() {
        return Some((hashes, ""));
    }
    if !rest.starts_with([' ', '\t']) {
        return None; // e.g. "#hashtag" is not a heading.
    }
    let text = rest.trim().trim_end_matches('#').trim_end();
    Some((hashes, text))
}

/// GitHub-style heading slug: lowercase; alphanumerics kept; spaces/`-`/`_` → `-`; other
/// punctuation dropped; consecutive/edge hyphens collapsed.
pub(crate) fn slugify(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        if c.is_alphanumeric() {
            for lc in c.to_lowercase() {
                out.push(lc);
            }
        } else if c == ' ' || c == '-' || c == '_' {
            out.push('-');
        }
    }
    let mut slug = String::with_capacity(out.len());
    let mut prev_dash = false;
    for c in out.chars() {
        if c == '-' {
            if !prev_dash {
                slug.push('-');
            }
            prev_dash = true;
        } else {
            slug.push(c);
            prev_dash = false;
        }
    }
    slug.trim_matches('-').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_frontmatter_and_body() {
        let (fm, body) = split_frontmatter("---\ntype: table\n---\nhello\n");
        assert_eq!(fm.as_deref(), Some("type: table\n"));
        assert_eq!(body, "hello\n");
    }

    #[test]
    fn no_frontmatter() {
        let (fm, body) = split_frontmatter("just body\n");
        assert!(fm.is_none());
        assert_eq!(body, "just body\n");
    }

    #[test]
    fn delimiter_must_be_on_its_own_line() {
        assert!(split_frontmatter("---  \ntype: T\n---\n").0.is_none());
        assert!(split_frontmatter("---\r\ntype: T\r\n---\r\n").0.is_some());
    }

    const DOC: &str =
        "# Title\n\nintro\n\n## Rates\n\nbody one\n\n### Nested\n\ndeep\n\n## Next\n\ntail\n";

    #[test]
    fn find_section_spans_nested_stops_at_equal_level() {
        let s = find_section(DOC, "Rates").unwrap();
        assert_eq!(s.level, 2);
        let lines: Vec<&str> = DOC.split('\n').collect();
        assert_eq!(lines[s.heading_line], "## Rates");
        // Section content runs through the nested block up to (excluding) "## Next".
        assert_eq!(lines[s.end], "## Next");
        assert!(lines[s.content_start..s.end].contains(&"### Nested"));
    }

    #[test]
    fn find_section_matches_by_slug_and_misses_cleanly() {
        assert_eq!(find_section(DOC, "rates").unwrap().level, 2);
        assert!(find_section(DOC, "does-not-exist").is_none());
    }

    #[test]
    fn find_section_ignores_headings_in_fences() {
        let doc = "## Sec\n\n```\n## fake\n```\n\n## Real\n";
        let s = find_section(doc, "Real").unwrap();
        let lines: Vec<&str> = doc.split('\n').collect();
        assert_eq!(lines[s.heading_line], "## Real");
    }

    #[test]
    fn slugify_matches_github_style() {
        assert_eq!(slugify("Reimbursement Rates"), "reimbursement-rates");
        assert_eq!(slugify("Foo: Bar (baz)!"), "foo-bar-baz");
    }
}

#[cfg(test)]
mod fence_and_bom_tests {
    use super::*;

    #[test]
    fn bom_before_frontmatter_is_tolerated() {
        let (fm, body) = split_frontmatter("\u{feff}---\ntype: T\n---\nhi\n");
        assert_eq!(fm.as_deref(), Some("type: T\n"));
        assert_eq!(body, "hi\n");
        let (fm, body) = split_frontmatter("\u{feff}plain\n");
        assert!(fm.is_none());
        assert_eq!(body, "\u{feff}plain\n");
    }

    #[test]
    fn fence_closer_must_match_char_and_length() {
        let doc = "## A\n\n````\n```\n## fake\n````\n\n## Real\n";
        assert!(find_section(doc, "fake").is_none());
        assert!(find_section(doc, "Real").is_some());
        let doc = "~~~\n```\n## fake\n~~~\n## Real\n";
        assert!(find_section(doc, "fake").is_none());
        assert!(find_section(doc, "Real").is_some());
        // A closer with an info string does not close.
        let doc = "```\n``` x\n## fake\n```\n## Real\n";
        assert!(find_section(doc, "fake").is_none());
    }
}
