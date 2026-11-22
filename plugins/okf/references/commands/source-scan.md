# okf source-scan

Inventory every regular source file without parsing it as an OKF concept

```sh
okf source-scan src
```

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<directory>` | positional | yes | Source directory to inventory; it need not be an OKF bundle |

Output stream: `source-file`.

NDJSON starts with [operation-start](../records/operation-start.md) and ends with [operation-complete](../records/operation-complete.md). Check the trailer and process exit.

[Global options](../globals.md) supported here: `--json`.


