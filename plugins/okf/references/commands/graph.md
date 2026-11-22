# okf graph

Render the link graph (or a bounded rooted neighborhood) as mermaid/dot/graphml

```sh
okf graph knowledge
```

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `--kind <value>` | list<string> | no | — |
| `--inverse-kind <value>` | list<string> | no | — |
| `--related-type <value>` | list<string> | no | — |
| `--rule <value>` | list<string> | no | — |
| `--edge-filter <value>` | list<string> | no | — |
| `--related-filter <value>` | list<string> | no | — |
| `--preset <value>` | string | no | — |
| `--clear-filter <value>` | list<string> | no | — |
| `<bundle>` | positional | no | Bundle directory (explicit, then $OKF_BUNDLE, nearest okf.toml, or current directory) |
| `--root <value>` | string | no | Optional concept at the neighborhood root; without one, render the entire graph |
| `--format <value>` | string | no | Output format: mermaid (default), dot, or graphml (default: `mermaid`) |
| `--direction <value>` | string | no | Edges to follow from the root: outgoing (default), incoming, or both (default: `outgoing`) |
| `--depth <value>` | int | no | Maximum neighbor distance from the root (0 = root only; default: unbounded) |

Output stream: `graph,scope,bundle-node,bundle-edge,query-filters,relationship-query-summary`.

NDJSON starts with [operation-start](../records/operation-start.md) and ends with [operation-complete](../records/operation-complete.md). Check the trailer and process exit.

Record shapes: [scope](../records/scope.md), [bundle-node](../records/bundle-node.md), [bundle-edge](../records/bundle-edge.md), [query-filters](../records/query-filters.md), [relationship-query-summary](../records/relationship-query-summary.md).

[Global options](../globals.md) supported here: `--bundle-id`, `--bundle-revision`, `--catalog`, `--catalog-scope`, `--context`, `--guidance`, `--json`, `--revision`, `--scope-bundle`, `--selection`, `--workspace-revision`.


