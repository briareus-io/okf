---
type: Command
group: mutate
mutates: true
title: okf format
description: >-
  Format YAML and opt-in Markdown prose while preserving values and verification
  metadata.
sources:
- resource: crates/okf-core/src/mutate/format.rs
  kind: git-path
- resource: crates/okf-core/src/parse/yaml.rs
  kind: git-path
- resource: crates/okf-cli/src/commands/format.rs
  kind: git-path
- resource: crates/okf-core/src/bundle/settings.rs
  kind: git-path
---
# okf format

Format one concept or every Markdown document in the selected bundle without changing parsed
frontmatter values, verification events, or generation metadata. YAML formatting is enabled by
default; Markdown prose wrapping is opt-in and only runs through this command.

```sh
okf format --bundle knowledge --dry-run
okf format notes/example --bundle knowledge
okf format --bundle knowledge --check
okf format --bundle knowledge
```

The optional positional argument is a concept ID. Unlike most commands, the directory selector
is `--bundle <path>`, not a trailing positional. `--bundle-id` selects a registered bundle instead.
Omitting the concept ID selects the whole bundle, excluding tooling support files and opaque
artifacts. Existing bundle resolution applies when neither selector is supplied.

`--dry-run` previews a complete diff without publishing. `--check` lists files needing formatting
and exits 1 for drift or 0 when clean; the two flags are mutually exclusive. Neither publishes
authored content, but coordination state may be created. An ordinary invocation applies all
prepared changes through guarded transactions. No-op runs leave content untouched.

## Configuration

Set presentation policy in the root `okf-bundle.yaml`. These are the defaults:

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

`yaml.style: legacy` retains the previous YAML serializer. A width of `0` disables wrapping
for that format. Width counts Unicode characters and is a target, not a hard limit: indivisible
tokens may exceed it. Legacy TOML supports the same settings under
`bundle_settings.<bundle>.formatting`. See [configuration](../reference/configuration.md) for
settings origins and conflicts; inspect effective policy with `okf bundle context knowledge`.

## Readable YAML

Eligible long single-line strings use folded blocks such as `>-`. Actual multiline values keep
their line breaks and use literal blocks where possible. Necessary quoting and escaping remain:
for example, a single-line scalar containing colon-space cannot safely be emitted as plain YAML.
Inside single quotes, doubled apostrophes represent one apostrophe in the parsed string.

Readable output is parsed again and compared with the original values. Unsafe candidates fall
back to the legacy serializer. JSON versus YAML input does not determine output style; both are
parsed into values before serialization. YAML comments and authored scalar styles are not retained.
This policy applies to normal concept writes as well as explicit formatting. Formatting itself
preserves authored mapping order; use the existing `frontmatter-order` lint fix for ontology order.

## Conservative Markdown wrapping

Set `markdown.wrap_prose: true` to enable wrapping of ordinary top-level text paragraphs. Even
when enabled, ordinary concept edits do not wrap Markdown. Only explicit `format` runs do so.

The current formatter leaves lists, blockquotes, headings, fenced and indented code, tables, HTML,
reference definitions, and paragraphs containing inline markup or unfamiliar extension syntax
unchanged. Bodies containing block markers `:::`, `!!!`, `???`, `+++`, `%`, or `{%` are skipped
entirely. URLs are never split. Hard breaks, significant whitespace, local paragraph line-ending
conventions, and missing final newlines are preserved; internally mixed-ending paragraphs are skipped.

Parsing uses pulldown-cmark with its extensions enabled. Parsed-event equivalence rejects reflow
that changes structure or text apart from soft breaks. This conservative subset does not promise
compatibility with every Markdown dialect. Repeated formatting is idempotent.

## Evidence and boundaries

Formatting preserves verification and generation metadata but does not verify claims, repair
malformed YAML, or refresh source fingerprints. Raw file fingerprints can change when presentation
changes; review citing concepts before any separate [refresh](refresh.md). Meaningful rewrites
still belong to [edit](edit.md) and require the normal review workflow.

Use `--check` as an optional presentation gate. There is no dedicated formatting lint rule;
[lint](lint.md) and [validate](validate.md) retain their separate purposes.

JSON output uses `format-file` records (path, applied status, diff or null for check mode), a
`format-summary` (changed-file count and applied mode), and the standard operation envelope.
Check the process exit and terminal publication evidence; applied mode alone does not establish
that a nonempty transaction was committed.

## Arguments

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<concept>` | positional | no | One concept ID; omit to format the entire bundle |
| `--bundle <value>` | path | no | Bundle directory (otherwise use the current context) |
| `--check` | bool | no | Report drift without writing; exit 1 when formatting is needed (default: `false`) |
| `--dry-run` | bool | no | Preview the complete diff without writing (default: `false`) |

Global `--json` requests NDJSON. `--bundle <path>` selects a directory; the optional positional is a concept ID, not a bundle path. `--bundle-id` selects a catalog identity. Without either selector, normal bundle-context resolution applies.

Output stream: `format-file,format-summary`.

Supported global options: `--bundle-id`, `--catalog`, `--context`, `--guidance`, `--json`.

Presentation-only formatting preserves verification and generation metadata. `--check` does not publish and exits 1 for formatting drift; `--dry-run` previews the diff. Omit the concept ID for the whole selected bundle. YAML uses `settings.formatting.yaml` (readable by default, target width 100; legacy retains the previous serializer). Markdown wrapping is disabled by default and runs only here when `settings.formatting.markdown.wrap_prose` is true. A width of 0 disables wrapping. Unsupported Markdown structures remain unchanged. YAML comments and scalar styles may normalize; file fingerprints may change. Formatting is not conformance repair, fingerprint refresh, or claim verification.
