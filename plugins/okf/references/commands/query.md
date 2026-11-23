# okf query

Run a manifest-defined chain of read operations

```sh
okf query triage-notes features/export --bundle-id acme.product --project owner --json
```

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<name>` | positional | yes | Saved query name in the primary bundle manifest |
| `<concept>` | positional | no | Starting concept for a concept-start query |
| `--bundle <value>` | path | no | Path to the primary bundle |
| `--type <value>` | string | no | Filter by exact concept `type` |
| `--tag <value>` | string | no | Filter by membership in the concept's `tags` |
| `--text <value>` | list<string> | no | Text query (repeatable). Repeated phrases use AND semantics by default |
| `--match <value>` | string | no | Text matching: phrase (default), all tokens, any token, or literal source text (default: `phrase`) |
| `--in <value>` | list<string> | no | Text fields to search (comma-separated or repeatable) (default: `id,title,description,body`) |
| `--sort <value>` | string | no | Result order: deterministic relevance (default) or concept id (default: `relevance`) |
| `--limit <value>` | int | no | Return at most this many results after all filters and sorting |
| `--field <value>` | list<string> | no | Filter by a frontmatter field, `key=value` (repeatable; AND) |
| `--facet-filter <value>` | list<string> | no | Typed selector condition: =, !=, [not] in, exists, absent, is [not] null; join with OR (repeatable; AND) |
| `--offset <value>` | int | no | Start page at this nonnegative offset; requires a positive limit |
| `--scan-limit <value>` | int | no | Maximum examined documents across scope (positive) |
| `--full-scan` | bool | no | Examine the entire selected scope without a document scan budget (default: `false`) |
| `--project <value>` | list<string> | no | Explicit metadata projection selector (repeatable). Computed fields: $id, $bundle, $version, $trust_tier, $effective_status |
| `--body` | bool | no | Read the body of each matching concept (default: `false`) |
| `--outline` | bool | no | Read the heading outline of each matching concept (default: `false`) |
| `--section <value>` | string | no | Full heading path as a JSON array of titles |

Output stream: `scope,concept-identity,concept,projection,body,outline,section,warning,saved-query-summary`.

NDJSON starts with [operation-start](../records/operation-start.md) and ends with [operation-complete](../records/operation-complete.md). Check the trailer and process exit.

Record shapes: [scope](../records/scope.md), [concept-identity](../records/concept-identity.md), [concept](../records/concept.md), [projection](../records/projection.md), [body](../records/body.md), [outline](../records/outline.md), [section](../records/section.md), [warning](../records/warning.md), [saved-query-summary](../records/saved-query-summary.md).

[Global options](../globals.md) supported here: `--bundle-id`, `--bundle-revision`, `--catalog`, `--catalog-scope`, `--context`, `--guidance`, `--json`, `--revision`, `--scope-bundle`, `--selection`, `--workspace-revision`.


