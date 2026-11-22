# okf mv

Move/rename a concept and rewrite every inbound link

```sh
okf mv policies/old policies/review knowledge
```

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<old>` | positional | yes | Existing concept id |
| `<new>` | positional | yes | New concept id |
| `<bundle>` | positional | no | Bundle directory (explicit, then $OKF_BUNDLE, nearest okf.toml, or current directory) |
| `--require-ontology-gate` | bool | no | Require the ontology write gate even when bundle policy leaves it disabled (default: `false`) |

Output stream: `change`.

NDJSON starts with [operation-start](../records/operation-start.md) and ends with [operation-complete](../records/operation-complete.md). Check the trailer and process exit.

[Global options](../globals.md) supported here: `--bundle-id`, `--catalog`, `--catalog-scope`, `--context`, `--guidance`, `--json`, `--scope-bundle`, `--selection`.

Moves rewrite inline/reference-style Markdown, self-links, declared simple/nested references, and curated index/log links within the selected bundle. File/text fingerprint sources retain bundle-root semantics; Git sources retain worktree-root semantics. Destinations must be absent. The complete write set is prevalidated and journaled for rollback. Use changeset plan/apply for coordinated moves, replacement, splits, or cleanup. Cross-bundle referrers are not rewritten.
