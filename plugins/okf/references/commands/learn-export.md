# okf learn export

Preview a draft and destination, then export with its confirmation digest

```sh
okf learn export draft-AbCd12.md --output feedback.md
```

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<draft>` | positional | yes | Managed draft filename returned by learn draft |
| `--output <value>` | string | yes | Explicit new file to create |
| `--confirm <value>` | string | no | Digest from the reviewed preview, binding content and destination |

Output stream: `feedback`.

NDJSON starts with [operation-start](../records/operation-start.md) and ends with [operation-complete](../records/operation-complete.md). Check the trailer and process exit.

Record shapes: [feedback](../records/feedback.md).

[Global options](../globals.md) supported here: `--json`.

Preview the complete text and destination first. Pass the returned digest only after review. The destination must be a new file.
