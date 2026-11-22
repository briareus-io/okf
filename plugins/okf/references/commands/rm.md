# okf rm

Remove a concept; refuse if backlinks would dangle unless `--force`

```sh
okf rm policies/obsolete knowledge
```

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<concept>` | positional | yes | Concept id to remove |
| `<bundle>` | positional | no | Bundle directory (explicit, then $OKF_BUNDLE, nearest okf.toml, or current directory) |
| `--force` | bool | no | Remove even if backlinks would dangle (default: `false`) |
| `--require-ontology-gate` | bool | no | Require the ontology write gate even when bundle policy leaves it disabled (default: `false`) |

Output stream: `change`.

NDJSON starts with [operation-start](../records/operation-start.md) and ends with [operation-complete](../records/operation-complete.md). Check the trailer and process exit.

[Global options](../globals.md) supported here: `--bundle-id`, `--catalog`, `--catalog-scope`, `--context`, `--guidance`, `--json`, `--scope-bundle`, `--selection`.

The backlink guard includes curated index/log navigation, declared structured references, and standard internal resources. --force deliberately permits dangling references. For coordinated cleanup, remove references and records in one changeset.
