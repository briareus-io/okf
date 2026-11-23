# okf show

Show one concept's content, heading outline, or selected line range

```sh
okf show policies/review knowledge --lines 1:5 --json
```

| JSON mode | Payload |
|---|---|
| `--body` | `body` |
| `--lines` | `line-range` |
| `--numbered` | `line-range` |
| `--outline` | `outline` |
| `--project` | `projection` |
| `--section` | `section` |
| `otherwise` | `concept` |

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `--section <value>` | string | no | Select a Markdown section by its full JSON heading path |
| `<concept>` | positional | yes | Concept id (leading slash optional), e.g. `tables/customers` |
| `<bundle>` | positional | no | Bundle directory (explicit, then $OKF_BUNDLE, nearest okf.toml, or current directory) |
| `--outline` | bool | no | Show only the Markdown heading outline with 1-based document line numbers (default: `false`) |
| `--lines <value>` | string | no | Show only an inclusive 1-based document line range, `START:END` (or one line, `N`) |
| `--numbered` | bool | no | Print document line numbers (including frontmatter); excludes the display header (default: `false`) |
| `--body` | bool | no | Print only the raw Markdown body, without frontmatter or display headers (default: `false`) |
| `--project <value>` | list<string> | no | Explicit metadata projection selector (repeatable). Computed fields: $id, $bundle, $version, $trust_tier, $effective_status |

Output stream: `concept,projection,body,outline,line-range,section`.

NDJSON starts with [operation-start](../records/operation-start.md) and ends with [operation-complete](../records/operation-complete.md). Check the trailer and process exit.

Record shapes: [concept](../records/concept.md), [projection](../records/projection.md), [body](../records/body.md), [outline](../records/outline.md), [line-range](../records/line-range.md), [section](../records/section.md).

[Global options](../globals.md) supported here: `--bundle-id`, `--bundle-revision`, `--catalog`, `--context`, `--guidance`, `--json`, `--revision`, `--selection`, `--workspace-revision`.

Without `--json`, show includes the serialized frontmatter and full Markdown body. Use `--section '["Operations","Recovery"]'` for a unique full heading path. Plain `show --json` returns metadata only: frontmatter plus `id`, `trust_tier`, `effective_status`, `effective_generated_at`, `latest_verified_at`, and `verification_current`. It does not include the body. `--outline --json` returns `headings` with `line`, `level`, and `text`; `--lines START:END --json` returns `start`, actual `end`, and `lines` containing `line` and `text`. Line numbers refer to the serialized document, including frontmatter, excluding the three-line display header. `show -n` (or `--numbered`) prints the full document with these numbers and no header; its JSON uses the same line-range record. `show --body` prints only raw Markdown; with `--json` it returns a body record containing `id` and `body`. An outline or selected slice does not establish complete document-review coverage. Unprojected `id` is the computed path; an authored frontmatter `id` is preserved in `frontmatter_conflicts.id`. `verification_current` compares a valid latest verification timestamp with the effective generation timestamp; null means the comparison is unavailable, not necessarily that the concept has never been reviewed.
