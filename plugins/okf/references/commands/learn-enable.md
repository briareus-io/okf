# okf learn enable

Enable local counting after interactive consent

```sh
okf learn enable --json
```

_No arguments._

Output stream: `feedback`.

NDJSON starts with [operation-start](../records/operation-start.md) and ends with [operation-complete](../records/operation-complete.md). Check the trailer and process exit.

Record shapes: [feedback](../records/feedback.md).

[Global options](../globals.md) supported here: `--json`.

Type enable at the terminal prompt to begin counting qualifying outcomes across checkouts.
