# okf learn draft

Create a private editable Markdown draft from synthetic guidance

```sh
okf learn draft feature-request
```

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<kind>` | positional | yes | — |
| `--pattern <value>` | string | no | Optional built-in outcome from the local report |

Output stream: `feedback`.

NDJSON starts with [operation-start](../records/operation-start.md) and ends with [operation-complete](../records/operation-complete.md). Check the trailer and process exit.

Record shapes: [feedback](../records/feedback.md).

[Global options](../globals.md) supported here: `--json`.

Choose a feedback kind even when collection is off. Use invented data and exclude actual identifiers and usage counts.
