# Format YAML and Markdown presentation

Use `okf format` for requested presentation changes. It preserves parsed frontmatter values,
verification, and generation metadata. Normal `edit` remains the route for meaningful content
changes. Formatting does not repair malformed YAML, establish conformance, refresh fingerprints,
or verify claims.

Pass the shared compatibility preflight and load context for operation `format` before using it.
For compatible older executables without this command, report that limitation instead of
substituting a broad text reformatter or installing software automatically.

```sh
okf format <concept-id> --bundle <bundle> --dry-run
okf format <concept-id> --bundle <bundle>
okf format --bundle <bundle> --check
```

Omit the concept ID for bundle-wide formatting. Unlike `edit`, `format` selects a directory with
`--bundle`; its positional argument is a concept ID. A registered `--bundle-id` also works.
`--dry-run` previews the complete diff. `--check` reports drift and exits 1 when files need
formatting, 0 when clean; it cannot be combined with `--dry-run`. Both avoid publishing authored
content, though coordination state may be created. JSON emits `format-file`, `format-summary`,
and the usual operation envelope. Review previews and apply within the user's requested scope.

## Configuration

The defaults in `okf-bundle.yaml` are:

```yaml
manifest_version: 1
settings:
  formatting:
    yaml:
      style: readable
      line_width: 100
    markdown:
      wrap_prose: false
      line_width: 100
```

Use `yaml.style: legacy` for the previous serializer. A width of `0` disables wrapping for that
format. Width is a target in Unicode characters; long indivisible tokens may exceed it. Settings
also work in legacy TOML under `bundle_settings.<bundle>.formatting`; conflicting manifest/TOML
values are errors, not overrides. Inspect effective settings and their origins with bundle context.

Readable YAML folds eligible long single-line strings using `>-`, retains actual multiline values
with literal blocks when possible, and keeps necessary quoting/escaping. Output is parsed again
and compared with the original values; unsafe readable candidates fall back to legacy rendering.
JSON input does not require quoted YAML output: parsed string content determines presentation.
YAML comments and authored scalar styles are not preserved. The YAML policy applies to ordinary
concept writes as well as explicit formatting.

Enable `markdown.wrap_prose: true` only when requested. Even then, Markdown wrapping runs only
through `format`, not ordinary edits. The supported subset is ordinary top-level text paragraphs.
Lists, blockquotes, headings, tables, code blocks, HTML, references, and paragraphs with inline
markup or unfamiliar extension syntax stay unchanged. Bodies with block markers `:::`, `!!!`,
`???`, `+++`, `%`, or `{%` are skipped entirely. URLs are indivisible, hard breaks and significant
whitespace remain intact, paragraph-local line endings are retained, and internally mixed-ending
paragraphs are skipped. Missing final newlines are retained. A parsed-event comparison rejects
reflow that changes structure or text apart from soft breaks. This is conservative support for
the CLI's Markdown parser, not a guarantee across arbitrary Markdown dialects.

Formatting is idempotent and publishes through guarded transactions. It preserves authored
field order; use the existing `frontmatter-order` lint fix for ontology ordering. Raw file
fingerprints can change even when values are equivalent: review citing sources before any
separately authorized refresh. There is no dedicated formatting lint rule; use `format --check`
when a presentation gate is requested. Do not add it as a mandatory check to unrelated workflows.
