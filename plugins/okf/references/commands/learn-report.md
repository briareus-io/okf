# okf learn report

Rank locally counted outcomes by affected invocations

```sh
okf learn report --json
```

_No arguments._

Output stream: `feedback`.

NDJSON starts with [operation-start](../records/operation-start.md) and ends with [operation-complete](../records/operation-complete.md). Check the trailer and process exit.

Record shapes: [feedback](../records/feedback.md).

[Global options](../globals.md) supported here: `--json`.

Each row counts affected invocations within a command family and CLI version. Contention can cause a missed sample.
