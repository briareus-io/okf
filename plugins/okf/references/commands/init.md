# okf init

Create a new empty OKF bundle

```sh
okf init knowledge --title 'Product knowledge'
```

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<bundle>` | positional | no | Bundle directory to create (explicit, then $OKF_BUNDLE, nearest okf.toml, or current directory) |
| `--title <value>` | string | no | Title for the scaffolded root `index.md` |
| `--no-index` | bool | no | Do not scaffold a root `index.md` (default: `false`) |
| `--no-ontology` | bool | no | Do not scaffold an `ontology.yaml` (default: `false`) |

Output stream: `change`.

NDJSON starts with [operation-start](../records/operation-start.md) and ends with [operation-complete](../records/operation-complete.md). Check the trailer and process exit.

[Global options](../globals.md) supported here: `--bundle-id`, `--catalog`, `--guidance`, `--json`.


