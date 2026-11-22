# operation-complete

```json
{
  "commit_reached": "boolean|null (when recovery-required)",
  "completeness": "{examination,matching,delivery,expansion,content: complete|partial|not-applicable; consistency: working-tree|immutable-revision|guarded-baseline}",
  "exit_code": "integer",
  "outcome": "ok|partial|failed",
  "publication": "not-applicable|not-published|committed|recovery-required",
  "reasons": "array<diagnostic-code>",
  "schema_version": 1
}
```
