# okf catalog

List effective catalog registrations, locations, and availability

```sh
okf catalog --json
```

_No arguments._

Output stream: `bundle-registration,effective-settings`.

NDJSON starts with [operation-start](../records/operation-start.md) and ends with [operation-complete](../records/operation-complete.md). Check the trailer and process exit.

Record shapes: [bundle-registration](../records/bundle-registration.md), [effective-settings](../records/effective-settings.md).

[Global options](../globals.md) supported here: `--catalog`, `--json`.


