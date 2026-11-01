use okf_core::{
    bundle::settings::{FormattingSettings, YamlStyle},
    mutate::format::{format_document, wrap_markdown},
    parse::{markdown::split_frontmatter, yaml},
};

#[test]
fn yaml_roundtrip_wrapping_and_idempotence() {
    let strings = [
        "REFINEMENT 2026-11-01 (package P6 wave; coordinator applied): refiner proposed readiness 'ready'; held at needs-refinement pending advisor review and independent verification.",
        "true", "null", "007", "123", "a: b", "a # b", "", " leading", "trailing ",
        "two  spaces are retained even when wrapping very long descriptions of a concept",
        "line one\nline two", "  indented\n  code\n", "one\n\n", "tab\there", "a\rb", "a\0b", "emoji 🦀 and 中文 and more words to wrap across several lines",
        "https://example.com/a/very/long/unbroken/url/with/no/space", "x\u{a0}y", "---", "!tag", "&alias",
    ];
    let mut settings = FormattingSettings::default();
    settings.yaml.line_width = 35;
    for s in strings {
        let value = serde_json::json!({"type":"Note", "notes":[s], "nested":{"value":s}});
        let input = format!(
            "---\n{}---\n# Body\n",
            serde_yaml::to_string(&value).unwrap()
        );
        let output = format_document(&input, &settings).unwrap();
        assert_eq!(
            yaml::parse_frontmatter(&split_frontmatter(&input).0.unwrap()).unwrap(),
            yaml::parse_frontmatter(&split_frontmatter(&output).0.unwrap()).unwrap(),
            "{s:?}"
        );
        assert_eq!(
            format_document(&output, &settings).unwrap(),
            output,
            "{s:?}"
        );
        if s.starts_with("REFINEMENT") {
            assert!(output.contains(">-\n"), "{output}");
            assert!(!output.contains("''ready''"));
        }
    }
}

#[test]
fn yaml_disabled_wrapping_and_legacy() {
    let input = "---\ntype: Note\nnotes:\n- 'long description: many words with a quoted ''word'' and sufficient length to wrap'\n---\nbody\n";
    let mut settings = FormattingSettings::default();
    settings.yaml.line_width = 0;
    assert_eq!(format_document(input, &settings).unwrap(), input);
    settings.yaml.style = YamlStyle::Legacy;
    settings.yaml.line_width = 10;
    assert_eq!(format_document(input, &settings).unwrap(), input);
}

#[test]
fn markdown_safe_subset_and_structural_guards() {
    let ordinary = "This is ordinary prose with enough words to wrap across several lines and a https://example.com/very/long/unbroken/url.\n";
    let wrapped = wrap_markdown(ordinary, 30);
    assert_ne!(wrapped, ordinary);
    assert!(wrapped.contains("https://example.com/very/long/unbroken/url."));
    assert_eq!(wrap_markdown(&wrapped, 30), wrapped);
    for untouched in [
        "# A heading with many words that should stay on a single line\n",
        "```rust\nlet code = a_very_long_expression_that_stays_unchanged();\n```\n",
        "    an indented code block with many words to leave alone\n",
        "- a list item with many words which remains untouched\n",
        "> a blockquote with many words which remains untouched\n",
        "| first | second |\n| --- | --- |\n| long value here | other value here |\n",
        "A paragraph with `inline code with spaces` and many more words.\n",
        "A paragraph with [a long link label](https://example.com) and more words.\n",
        "<div>Some long HTML with many words that should be left alone.</div>\n",
        "A line with a hard break  \nand another line with many words\n",
        "A line with a hard break\\\nand another line with many words\n",
        "[ref]: https://example.com/some/long/path \"a title with spaces\"\n",
        "A paragraph with {{ unfamiliar template syntax }} and more words.\n",
    ] {
        assert_eq!(wrap_markdown(untouched, 20), untouched);
    }
    // Reflow must not introduce a new list item at a line boundary.
    let syntax = "ordinary words 1. item after this stays a paragraph\n";
    assert_eq!(wrap_markdown(syntax, 14), syntax);
}

#[test]
fn opt_in_crlf_and_missing_final_newline() {
    let input = "---\r\ntype: Note\r\n---\r\nThis is a long paragraph containing many ordinary words to be wrapped across lines without a final newline";
    let mut settings = FormattingSettings::default();
    assert_eq!(format_document(input, &settings).unwrap(), input);
    settings.markdown.wrap_prose = true;
    settings.markdown.line_width = 30;
    let output = format_document(input, &settings).unwrap();
    assert!(!output.ends_with('\n'));
    assert!(!output.replace("\r\n", "").contains('\n'));
    assert!(output.starts_with("---\r\ntype: Note\r\n---\r\n"));
    assert_eq!(format_document(&output, &settings).unwrap(), output);
}

#[test]
fn unknown_block_dialects_preserve_the_entire_markdown_body() {
    for body in [
        "+++ some long tab title that should be kept on the same line +++\n",        "!!! note \"A very long directive title that must stay on the directive line\"\n\n    A long admonition paragraph which must stay exactly as authored.\n",
        "??? note \"A very long collapsible title that must stay on the directive line\"\n",
        "::: warning A long title with many words which should stay in its directive\n\nA long paragraph inside the directive with many words which must remain untouched.\n:::\n",
        "% A long pandoc title with many words\n% Author\n% Date\n\nA long paragraph following the metadata which is conservatively preserved.\n",
        "{% custom_block %}\n\nOrdinary looking prose inside an unknown template block must remain unchanged.\n{% endcustom_block %}\n",
    ] {
        assert_eq!(wrap_markdown(body, 25), body);
    }
}

#[test]
fn paragraph_local_newline_conventions_are_preserved() {
    let mut settings = FormattingSettings::default();
    settings.markdown.wrap_prose = true;
    settings.markdown.line_width = 25;
    let body = "A long paragraph with many words to wrap while preserving its local newline convention.\r\n";
    let input = format!("---\ntype: Note\n---\n{body}");
    let output = format_document(&input, &settings).unwrap();
    let (_, formatted_body) = split_frontmatter(&output);
    assert!(!formatted_body.replace("\r\n", "").contains('\n'));
    assert_eq!(format_document(&output, &settings).unwrap(), output);
    let mixed = "A long paragraph with many ordinary words and mixed endings\r\nthat must stay untouched because there is no single convention.\n";
    assert_eq!(wrap_markdown(mixed, 25), mixed);
}
