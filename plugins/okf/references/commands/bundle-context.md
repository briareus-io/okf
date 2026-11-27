# okf bundle context

Return effective settings and selected agent guidance without running a workflow

```sh
okf bundle context knowledge --settings-detail compact --json
```

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<bundle>` | positional | no | — |
| `--operation <value>` | list<string> | no | — |
| `--rule <value>` | list<string> | no | Expand a guidance rule for inspection, including optional reference material (repeatable) |
| `--workflow <value>` | list<string> | no | — |
| `--all` | bool | no |  (default: `false`) |
| `--format <value>` | string | no | Render concise guidance, workflow resources, outputs, and saved queries. Conflicts with --json, --settings-detail, and --check-token |
| `--settings-detail <value>` | string | no | Settings projection; full preserves the complete JSON contract, compact shows relevant values (default: `full`) |
| `--stats` | bool | no | Show delivered instruction bytes per section and totals (not estimated tokens) (default: `false`) |
| `--guidance-budget <value>` | int | no | Advisory byte budget for delivered mandatory instructions; never truncates instructions |
| `--token` | bool | no | Issue a stateless context token after complete applicable instruction delivery (default: `false`) |
| `--check-token <value>` | string | no | Check a token without executing an operation; optional --operation checks coverage too |
| `--inputs <value>` | string | no | JSON input object or @file; values remain typed data |
| `--query <value>` | string | no | JSON query object or @file; resolve without executing the query |

Output stream: `context-scope,bundle-context,guidance,workflow,effective-query,context-stats,context-token,context-check`.

NDJSON starts with [operation-start](../records/operation-start.md) and ends with [operation-complete](../records/operation-complete.md). Check the trailer and process exit.

Record shapes: [context-scope](../records/context-scope.md), [bundle-context](../records/bundle-context.md), [guidance](../records/guidance.md), [workflow](../records/workflow.md), [context-stats](../records/context-stats.md), [context-token](../records/context-token.md), [context-check](../records/context-check.md).

[Global options](../globals.md) supported here: `--bundle-id`, `--bundle-revision`, `--catalog`, `--catalog-scope`, `--guidance`, `--json`, `--revision`, `--scope-bundle`, `--selection`, `--workspace-revision`.

Batch known operations in the initial request to load global and mandatory scoped instructions together. Within each bundle, a shared instruction file is delivered once. Later records carry content=shared and instructions_ref identifying the earlier record by kind and id. Each record retains its own instruction_role. Reference-tier rules expand with --rule; --all and historical context are inspection only. --format agent delivers concise rules, workflow resources and declared outputs, labels inspected bodies, lists available rules and workflows, and reports scope completion. Agent and compact context include definitions from the selected workflows' uses_queries lists. --operation query includes every saved query. For structured compact settings, replace --format agent with --settings-detail compact --json. Use full JSON for workflow input schemas, including enum choices. Run --check-token separately. --settings-detail compact explicitly projects settings; full JSON remains the default. --stats reports delivered instruction bytes. --token emits a stateless receipt after delivery; pass it with --context (or @file) on covered commands, or check with --check-token. A stale supplied token fails before execution; tokens are optional and are not authorization or concept-content guards. Context digest version 2 covers loaded instructions and selected workflow resources as well as settings, ontology and snapshot identity. Incidental guidance never replaces pre-action loading. Use bundle select to discover and bind a recovery root while a pending transaction prevents normal context reads.
