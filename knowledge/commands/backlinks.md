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

## Arguments

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<concept>` | positional | yes | Concept id (leading slash optional), e.g. `tables/customers` |
| `<bundle>` | positional | no | Bundle directory (explicit, then $OKF_BUNDLE, nearest okf.toml, or current directory) |
| `--details` | bool | no | Show individual semantic relationship occurrences and configured inverse labels (default: `false`) |

Global `--json` requests NDJSON. The optional trailing `bundle` positional is a path; `--bundle-id` selects a catalog identity. Selection uses explicit path or ID, `$OKF_BUNDLE` (path), registered containing bundle, configured default, sole catalog entry, then uncataloged cwd. Selection and examination scope are separate.

Output stream: `concept,relationship,scope,bundle-backlink,bundle-edge`.

Supported global options: `--bundle-id`, `--catalog-scope`, `--json`, `--revision`, `--scope-bundle`.
