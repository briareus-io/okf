# Global CLI arguments

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `--json` | bool | no | Emit NDJSON instead of human text or a bare artifact; schema is always NDJSON (default: `false`) |
| `--selection <value>` | string | no | Bind execution to a previously issued bundle selection token (@file supported) |
| `--bundle-id <value>` | string | no | Select an explicitly registered bundle identity (paths keep separate meanings) |
| `--scope-bundle <value>` | list<string> | no | Add a registered bundle to the examination scope (repeatable) |
| `--catalog-scope` | bool | no | Examine every locally available registered bundle (default: `false`) |
| `--revision <value>` | string | no | Inspect a locally available Git revision; never fetches missing objects |
| `--catalog <value>` | string | no | Select a catalog explicitly; relative paths are resolved from the invocation directory |
| `--guidance <value>` | string | no | Incidental guidance delivery; explicit context requests always return selected instructions |
| `--context <value>` | list<string> | no | Check a previously loaded context token before execution (repeat per scoped bundle; @file supported) |
| `--workspace-revision <value>` | string | no | Inspect the consumer repository and its catalog/lock at this revision |
| `--bundle-revision <value>` | list<string> | no | Inspect a registered bundle at an exact local commit, ID=COMMIT (repeatable) |

[Bundle selection, scope, and completion](bundle-context.md)
