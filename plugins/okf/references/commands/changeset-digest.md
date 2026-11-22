# okf changeset digest

Acquire the guarded bundle baseline token without staging changes

```sh
okf changeset digest knowledge
```

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<bundle>` | positional | no | Bundle directory |

Output stream: `baseline`.

NDJSON starts with [operation-start](../records/operation-start.md) and ends with [operation-complete](../records/operation-complete.md). Check the trailer and process exit.

Record shapes: [baseline](../records/baseline.md).

[Global options](../globals.md) supported here: `--bundle-id`, `--catalog`, `--catalog-scope`, `--context`, `--guidance`, `--json`, `--scope-bundle`, `--selection`.

Returns the baseline token without an operation document. Use the same `--scope-bundle` or `--catalog-scope` as the later scoped plan/apply to cover its examined target inputs. The token does not establish ontology correctness or approve an operation document; apply validates the proposed changes and rechecks its covered baseline.
