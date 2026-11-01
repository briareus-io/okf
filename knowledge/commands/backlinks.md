---
type: Command
group: query
mutates: false
title: okf backlinks
description: Lists the concepts that link to a given concept, using the reverse adjacency from the graph.
sources:
- resource: crates/okf-cli/src/commands/query.rs
  kind: git-path
- resource: crates/okf-cli/src/commands/schema.rs
  kind: git-path
- resource: crates/okf-cli/src/cli.rs
  kind: git-path
last_modified: 2026-10-04T20:39:29Z
---
# okf backlinks

Lists the concepts that link to a given concept, using the reverse adjacency from the graph. Use
[links](links.md) for the direct outbound targets defined by a concept.

Semantic incoming occurrences expose configured inverse kinds and authored conditions. Those
views do not author acceptance by the target or prove fulfillment. Qualified catalog identities
and explicit examination scope prevent global completeness claims; missing or out-of-scope
consumers remain unknown.

## Filter relationship occurrences

`--kind` matches the authored kind in either direction; `--inverse-kind` matches the configured
inverse label. `--related-type` selects the other endpoint: target for links, source for backlinks.
Edge attribute predicates apply to one occurrence, preserving correlation across parallel edges.
Related metadata predicates apply to that occurrence's other concept. Ordinary links have no
invented semantic kind and do not match semantic-kind filters.

For a defect that authors `blocks` toward a feature, use:

```text
okf backlinks features/export knowledge --kind blocks --related-type Defect --details
```

Explicit `--preset` selects a manifest query preset. CLI-supplied categories replace that category
from the preset; `--clear-filter <category>` removes an inherited category. There are no automatic
semantic filter defaults. Filters apply before deduplication and pagination. Effective predicates
and examination completeness remain visible independently of incidental guidance output. A
partial scan or unavailable endpoint cannot establish that no blockers exist.

## Arguments

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<concept>` | positional | yes | — |
| `<bundle>` | positional | no | — |
| `--details` | bool | no |  (default: `false`) |
| `--kind <value>` | list<string> | no | — |
| `--inverse-kind <value>` | list<string> | no | — |
| `--related-type <value>` | list<string> | no | — |
| `--rule <value>` | list<string> | no | — |
| `--edge-filter <value>` | list<string> | no | — |
| `--related-filter <value>` | list<string> | no | — |
| `--preset <value>` | string | no | — |
| `--clear-filter <value>` | list<string> | no | — |
| `--limit <value>` | string | no | — |
| `--offset <value>` | string | no |  (default: `0`) |
| `--scan-limit <value>` | string | no | — |
| `--full-scan` | bool | no |  (default: `false`) |

Global `--json` requests NDJSON. The optional trailing `bundle` positional is a path; `--bundle-id` selects a catalog identity. Selection uses explicit path or ID, `$OKF_BUNDLE` (path), registered containing bundle, configured default, sole catalog entry, then uncataloged cwd. Selection and examination scope are separate.

Output stream: `concept,relationship,scope,bundle-backlink,bundle-edge`.

Supported global options: `--bundle-id`, `--bundle-revision`, `--catalog`, `--catalog-scope`, `--guidance`, `--json`, `--revision`, `--scope-bundle`, `--workspace-revision`.
