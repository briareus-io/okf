---
type: Command
group: query
mutates: false
title: okf links
description: Lists the normalized direct concept links defined by one concept and reports whether each target exists.
sources:
- resource: crates/okf-cli/src/commands/schema.rs
  kind: git-path
- resource: crates/okf-cli/src/cli.rs
  kind: git-path
- resource: crates/okf-cli/src/commands/query.rs
  kind: git-path
- resource: crates/okf-core/src/model/link.rs
  kind: git-path
last_modified: 2026-10-19T03:58:59Z
---
# okf links

Lists one concept's direct normalized outbound edges from ontology-declared frontmatter
references, internal Markdown `sources[].resource` values, and Markdown body links. External and
self links are excluded, duplicate targets are collapsed, and missing concept targets remain
visible. Use [backlinks](backlinks.md) for the inverse one-hop query or a bounded
[graph](graph.md) for multi-hop context.

Use `okf links <concept> <bundle> --details` for authored semantic occurrences, including
multiple relationships to the same target. Each carries its kind, configured inverse label,
concrete field path, attributes, and status. Semantic self-edges and external or unknown-kind
occurrences remain inspectable; the compact adjacency rules above apply to ordinary links.
Inverse labels describe the incoming view and do not add outgoing edges.

Local `--details --json` emits `relationship` records with `schema_version: 1`,
`direction: outgoing`, bundle/version identity and an `edge` payload. Remaining ordinary targets
use `link` records with `source`, `target`, and `exists`; they have no invented semantic kind.
Catalog queries retain `scope` and `bundle-edge` records, with semantic metadata nested under
`relationship`. Detailed catalog text displays kinds, inverse labels, paths and attributes.

Declared nested reference selectors extract individual metadata occurrences. Semantic rules pair
targets with kinds and attributes in the same list record; opaque path-like strings do not create
edges. Catalog graph inspection retains qualified identity and resource/snapshot evidence.

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

Output stream: `link,relationship,scope,bundle-edge`.

Supported global options: `--bundle-id`, `--bundle-revision`, `--catalog`, `--catalog-scope`, `--context`, `--guidance`, `--json`, `--revision`, `--scope-bundle`, `--workspace-revision`.
