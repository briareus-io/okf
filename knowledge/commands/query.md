---
type: Command
group: query
mutates: false
title: okf query
description: Run a named saved query and return matching endpoint concepts, properties, or prose.
sources:
- resource: crates/okf-cli/src/commands/saved_query.rs
  kind: git-path
---
# okf query

Run a definition from the primary bundle's `settings.saved_queries`. Supply the starting ID for
a concept start, or omit it for a list/search preset start. Each hop may independently follow
links or backlinks and filter typed relationships and related concepts. CLI filters and pagination
apply to the final endpoint concepts. Use `--project`, `--outline`, `--body`, or `--section`
to select the output. See [saved queries](../guides/save-and-run-queries.md) for the
ontology, manifest, examples, and completeness contract.

For a local working-tree concept start with only outgoing hops, the CLI reads reached documents
on demand. A complete outbound result can leave unrelated documents unread, including documents
with invalid frontmatter. The summary reports `strategy: outgoing` and the number actually
examined. Incoming hops, preset starts, explicit scope, and historical reads use the scoped graph.

## Arguments

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

Use `--bundle <path>` or `--bundle-id <id>` to select the primary bundle. Global `--json` requests NDJSON.

Output stream: `scope,concept-identity,concept,projection,body,outline,section,warning,saved-query-summary`.

Supported global options: `--bundle-id`, `--bundle-revision`, `--catalog`, `--catalog-scope`, `--context`, `--guidance`, `--json`, `--revision`, `--scope-bundle`, `--selection`, `--workspace-revision`.
