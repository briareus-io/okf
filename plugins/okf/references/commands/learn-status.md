# okf learn status

Inspect the managed location, consent, draft count, and data size

```sh
okf learn status --json
```

_No arguments._

Output stream: `feedback`.

NDJSON starts with [operation-start](../records/operation-start.md) and ends with [operation-complete](../records/operation-complete.md). Check the trailer and process exit.

Record shapes: [feedback](../records/feedback.md).

[Global options](../globals.md) supported here: `--json`.

The directory is per OS user. Status works before the first draft or opt-in.
