# okf artifact resolve

Resolve any OKF path-valued resource with document context

```sh
okf artifact resolve references/schema.json knowledge
```

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<resource>` | positional | yes | Resource path, URL, or scope descriptor |
| `<bundle>` | positional | no | Bundle directory (explicit, then $OKF_BUNDLE, nearest okf.toml, or current directory) |
| `--from <value>` | string | no | Resolve a relative resource against this declaring concept id |

Output stream: `artifact-resolution`.

NDJSON starts with [operation-start](../records/operation-start.md) and ends with [operation-complete](../records/operation-complete.md). Check the trailer and process exit.

[Global options](../globals.md) supported here: `--bundle-id`, `--bundle-revision`, `--catalog`, `--catalog-scope`, `--context`, `--guidance`, `--json`, `--revision`, `--scope-bundle`, `--selection`, `--workspace-revision`.

**Path namespaces:** a leading `/` means bundle-root-relative, not an operating-system absolute path. Other local paths resolve against the declaring concept's directory when `--from` is supplied, otherwise the bundle root. Keep `--from` on the subsequent read too. JSON uses `artifact_kind` (concept, artifact, reserved, support, external, scope, missing, or blocked), `path`, `exists`, `size`, and `message`. Missing/blocked resolution exits 1. A scope descriptor is provenance, not a missing file.

**Repository sources:** with repo `/work/app`, bundle `/work/app/knowledge`, and declaring concept `notes/service`, `/references/spec.txt` resolves to `/work/app/knowledge/references/spec.txt`. A `kind=git-path` or `git-commit` source `src/service.rs` is instead fingerprinted from the Git worktree root as `/work/app/src/service.rs`; the artifact resolver does not reinterpret paths by source kind. File/line-range/markdown-heading sources use bundle-root-relative paths in fingerprinting, doctor, and graph resolution; kind-less provenance and body links remain document-relative. Generic artifact commands cannot infer a source kind from the resource string: omit `--from` for a typed bundle-root source path. Correct typed source paths that previously relied on graph's document-relative interpretation. Ordinary cross-bundle paths and logical references require a registered target in explicit examination scope. Record the actual source location and convention; do not assume fingerprint and artifact paths coincide. Inspect external repository evidence with normal source tools within existing read authorization. Do not rewrite provenance, bypass containment, or mirror files merely to make artifact resolution succeed.
