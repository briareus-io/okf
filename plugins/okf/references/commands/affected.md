# okf affected

Impact query: concepts needing review given a set of changed links

```sh
okf affected knowledge --changed policies/review
```

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<bundle>` | positional | no | Bundle directory (explicit, then $OKF_BUNDLE, nearest okf.toml, or current directory) |
| `--changed <value>` | list<string> | no | A changed link/concept-id/resource (repeatable; use - to read stdin) |
| `--changed-base <value>` | string | no | Interpret changed paths relative to the bundle, repository, or invocation directory (default: `bundle`) |
| `--transitive` | bool | no | Follow the cascade past direct dependents (default: `false`) |
| `--depth <value>` | int | no | Cap hops with --transitive |
| `--fail-on <value>` | string | no | Fail (exit 1) on any affected concept: never (default) \| info \| warn \| error \| any (default: `never`) |

Output stream: `affected,scope,bundle-affected`.

NDJSON starts with [operation-start](../records/operation-start.md) and ends with [operation-complete](../records/operation-complete.md). Check the trailer and process exit.

Record shapes: [scope](../records/scope.md), [bundle-affected](../records/bundle-affected.md).

[Global options](../globals.md) supported here: `--bundle-id`, `--bundle-revision`, `--catalog`, `--catalog-scope`, `--context`, `--guidance`, `--json`, `--revision`, `--scope-bundle`, `--selection`, `--workspace-revision`.


