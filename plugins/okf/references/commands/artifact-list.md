# okf artifact list

List local artifacts and concepts under a bundle directory

```sh
okf artifact list knowledge
```

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<bundle>` | positional | no | Bundle root directory; use --directory to select a directory within the bundle |
| `--directory <value>` | string | no | Bundle-relative directory to inventory (default: `references`) |
| `--exclude-concepts` | bool | no | Exclude concepts, retaining opaque artifacts and reserved files (default: `false`) |
| `--digest` | bool | no | Compute SHA-256 digests (reads each file) (default: `false`) |

Output stream: `artifact`.

NDJSON starts with [operation-start](../records/operation-start.md) and ends with [operation-complete](../records/operation-complete.md). Check the trailer and process exit.

[Global options](../globals.md) supported here: `--bundle-id`, `--bundle-revision`, `--catalog`, `--context`, `--guidance`, `--json`, `--revision`, `--selection`, `--workspace-revision`.

The positional path selects a bundle root relative to the current directory. To filter within the selected bundle, use `--directory contracts/x`; printed paths remain relative to the bundle root. Use `--directory .` for the entire bundle and `--exclude-concepts` to retain only opaque artifacts and reserved files.
