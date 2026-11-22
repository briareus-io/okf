# okf artifact put

Create or replace a local artifact; citing source fingerprints remain unchanged

```sh
okf artifact put references/schema.json @schema.json knowledge --create-only
```

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<resource>` | positional | yes | Destination path relative to the bundle root (opaque or reserved artifacts only) |
| `<input>` | positional | yes | Read artifact bytes from @file (file path is relative to the current directory) |
| `<bundle>` | positional | no | Bundle directory (explicit, then $OKF_BUNDLE, nearest okf.toml, or current directory) |
| `--create-only` | bool | no | Refuse to overwrite an existing file (the default) (default: `false`) |
| `--replace` | bool | no | Allow replacement of an existing artifact, leaving source fingerprints for drift checks (default: `false`) |

Output stream: `artifact-write`.

NDJSON starts with [operation-start](../records/operation-start.md) and ends with [operation-complete](../records/operation-complete.md). Check the trailer and process exit.

[Global options](../globals.md) supported here: `--bundle-id`, `--catalog`, `--context`, `--guidance`, `--json`, `--selection`.

Write bytes from `@file` to a bundle-relative opaque or reserved artifact. The default (and `--create-only`) refuses an existing destination; `--replace` permits replacement. Parent directories are created as needed. Concept and configuration files are protected, and paths must remain inside the bundle. Recorded source fingerprints are preserved so `stale` detects changes; review citing concepts before `refresh`.
