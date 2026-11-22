# okf artifact show

Retrieve a bounded local text artifact; binary files return metadata only

```sh
okf artifact show references/schema.json knowledge
```

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<resource>` | positional | yes | Artifact path relative to the bundle root, or to --from when provided |
| `<bundle>` | positional | no | Bundle directory (explicit, then $OKF_BUNDLE, nearest okf.toml, or current directory) |
| `--from <value>` | string | no | Resolve a relative resource against this declaring concept id |
| `--lines <value>` | string | no | Retrieve only an inclusive, one-based START:END line range |
| `--max-bytes <value>` | int | no | Maximum bytes read into output (default: `65536`) |
| `--fetch` | bool | no | Request remote retrieval; unavailable unless built with url-sources and allowed by policy (default: `false`) |

Output stream: `artifact-content`.

NDJSON starts with [operation-start](../records/operation-start.md) and ends with [operation-complete](../records/operation-complete.md). Check the trailer and process exit.

[Global options](../globals.md) supported here: `--bundle-id`, `--bundle-revision`, `--catalog`, `--catalog-scope`, `--context`, `--guidance`, `--json`, `--revision`, `--scope-bundle`, `--selection`, `--workspace-revision`.

For document-relative paths, pass the same `--from` used during resolution. For bundle-root-relative file/line-range/markdown-heading source resources, omit `--from`; this generic reader does not infer the source kind. JSON includes `text`, `binary`, `truncated`, `size`, `sha256`, and `path`. Binary files provide metadata only. Inspect `truncated` before treating a read as complete; human output alone does not expose this flag. Use bounded line windows and a sufficient byte budget for the needed range; do not infer absence from a truncated result. Use `show` for concepts and this command for opaque or reserved files.

Local artifact reads remain inside the resolved target bundle and requested examination scope. For an HTTP(S) URL, `--fetch` requires a network-enabled build and authorization covering that source. A user request to inspect a named source can supply that authorization; no separate OKF policy file is specified by this command. If unavailable, use an authorized external retrieval tool or report the evidence gap. Do not send ambient credentials. Reading computation, executor, or attester code never authorizes execution.
