# okf learn purge

Remove managed feedback and disable counting

```sh
okf learn purge --json
```

_No arguments._

Output stream: `feedback`.

NDJSON starts with [operation-start](../records/operation-start.md) and ends with [operation-complete](../records/operation-complete.md). Check the trailer and process exit.

Record shapes: [feedback](../records/feedback.md).

[Global options](../globals.md) supported here: `--json`.

Purge removes managed drafts and counts. Copies previously exported to user-selected destinations remain there.
