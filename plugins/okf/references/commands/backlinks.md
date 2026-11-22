# okf backlinks

Concepts that link to a given concept

```sh
okf backlinks policies/review knowledge
```

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
| `--limit <value>` | int | no | — |
| `--offset <value>` | int | no |  (default: `0`) |
| `--scan-limit <value>` | int | no | — |
| `--full-scan` | bool | no |  (default: `false`) |
| `--no-align` | bool | no | Render concept tables with tab-separated cells and no padding (default: `false`) |

Output stream: `concept,relationship,scope,bundle-backlink,bundle-edge,query-filters,relationship-query-summary`.

NDJSON starts with [operation-start](../records/operation-start.md) and ends with [operation-complete](../records/operation-complete.md). Check the trailer and process exit.

Record shapes: [concept](../records/concept.md), [relationship](../records/relationship.md), [scope](../records/scope.md), [bundle-backlink](../records/bundle-backlink.md), [bundle-edge](../records/bundle-edge.md), [query-filters](../records/query-filters.md), [relationship-query-summary](../records/relationship-query-summary.md).

[Global options](../globals.md) supported here: `--bundle-id`, `--bundle-revision`, `--catalog`, `--catalog-scope`, `--context`, `--guidance`, `--json`, `--revision`, `--scope-bundle`, `--selection`, `--workspace-revision`.


