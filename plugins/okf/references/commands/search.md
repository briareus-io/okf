# okf search

Search concepts by type, tag, text, and/or frontmatter field

```sh
okf search --bundle-id acme.a --scope-bundle acme.b --type Policy --sort id --full-scan --limit 10 --json
```

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `--preset <value>` | string | no | — |
| `--clear-filter <value>` | list<string> | no | — |
| `--expand-kind <value>` | list<string> | no | — |
| `--expand-inverse-kind <value>` | list<string> | no | — |
| `--expand-related-type <value>` | list<string> | no | — |
| `--expand-edge-filter <value>` | list<string> | no | — |
| `--expand-related-filter <value>` | list<string> | no | — |
| `<bundle>` | positional | no | Bundle directory (explicit, then $OKF_BUNDLE, nearest okf.toml, or current directory) |
| `--type <value>` | string | no | Filter by exact concept `type` |
| `--tag <value>` | string | no | Filter by membership in the concept's `tags` |
| `--text <value>` | list<string> | no | Text query (repeatable). Repeated phrases use AND semantics by default |
| `--match <value>` | string | no | Text matching: phrase (default), all tokens, any token, or literal source text (default: `phrase`) |
| `--in <value>` | list<string> | no | Text fields to search (comma-separated or repeatable) (default: `id,title,description,body`) |
| `--sort <value>` | string | no | Result order: deterministic relevance (default) or concept id (default: `relevance`) |
| `--limit <value>` | int | no | Return at most this many results after all filters and sorting |
| `--field <value>` | list<string> | no | Filter by a frontmatter field, `key=value` (repeatable; AND) |
| `--facet-filter <value>` | list<string> | no | Typed selector condition: =, !=, [not] in, exists, absent, is [not] null; join with OR (repeatable; AND) |
| `--facets` | bool | no | Emit JSON facets over all matches, before pagination (default: `false`) |
| `--facet <value>` | list<string> | no | Facet selector (repeatable); does not bypass high cardinality guard |
| `--offset <value>` | int | no | Start page at this nonnegative offset; requires a positive limit |
| `--scan-limit <value>` | int | no | Maximum examined documents across scope (positive) |
| `--full-scan` | bool | no | Examine the entire selected scope without a document scan budget (default: `false`) |
| `--project <value>` | list<string> | no | Explicit metadata projection selector (repeatable). Computed fields: $id, $bundle, $version, $trust_tier, $effective_status |
| `--no-align` | bool | no | Render concept tables with tab-separated cells and no padding (default: `false`) |
| `--columns <value>` | list<string> | no | Human output column selector (repeatable) |
| `--view <value>` | string | no | Named bundle display view |
| `--expand <value>` | list<string> | no | Expand selected outbound relationship rule (repeatable) |
| `--no-expand` | bool | no | Disable configured human view expansion (default: `false`) |
| `--target-field <value>` | list<string> | no | Metadata selector for related targets (repeatable) |
| `--expansion-edges <value>` | int | no | Maximum edge occurrences per primary hit (1..100) (default: `10`) |
| `--expansion-targets <value>` | int | no | Maximum distinct target payloads per query (1..1000) (default: `100`) |
| `--expansion-bytes <value>` | int | no | Maximum serialized expansion bytes (1..1048576) (default: `262144`) |

Output stream: `query-filters,concept,concept-identity,scope,projection,query-summary,facet,facet-excluded,relationship,related-concept,expansion-summary,warning`.

NDJSON starts with [operation-start](../records/operation-start.md) and ends with [operation-complete](../records/operation-complete.md). Check the trailer and process exit.

Record shapes: [query-filters](../records/query-filters.md), [concept](../records/concept.md), [concept-identity](../records/concept-identity.md), [scope](../records/scope.md), [projection](../records/projection.md), [query-summary](../records/query-summary.md), [facet](../records/facet.md), [facet-excluded](../records/facet-excluded.md), [relationship](../records/relationship.md), [related-concept](../records/related-concept.md), [expansion-summary](../records/expansion-summary.md), [warning](../records/warning.md).

[Global options](../globals.md) supported here: `--bundle-id`, `--bundle-revision`, `--catalog`, `--catalog-scope`, `--context`, `--guidance`, `--json`, `--revision`, `--scope-bundle`, `--selection`, `--workspace-revision`.

The positional argument is the bundle, never query text. Text requires `--text`; structured filters work without it. No filters means inventory. Text-search JSON adds `search.score` and bounded `search.matches` to metadata records, not full bodies. Structured-only search has no text-match evidence. `--in title,description` narrows the default fields; adding `frontmatter` broadens them. Empty results exit successfully and establish only that this query found no matches. Check query-summary and warning records: after a complete scan, repeat with the returned `next_offset` while `has_more` is true. Keep scope, filters, sort, revisions and interpretation consistent across pages. Incomplete scans cannot establish absence, exact totals, or globally ordered pages. --limit does not reduce scan work; offset requests rescan without shared query caches. Separate nested filters may match different list records; inspect concrete occurrences before attributing their combined conditions to one relationship. Expansion completeness and facet truncation are independent of primary scan completeness.
