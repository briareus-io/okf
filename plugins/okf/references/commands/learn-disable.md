# okf learn disable

Stop counting and retain local feedback

```sh
okf learn disable --json
```

_No arguments._

Output stream: `feedback`.

NDJSON starts with [operation-start](../records/operation-start.md) and ends with [operation-complete](../records/operation-complete.md). Check the trailer and process exit.

Record shapes: [feedback](../records/feedback.md).

[Global options](../globals.md) supported here: `--json`.

Disabling collection preserves existing aggregates and drafts.
