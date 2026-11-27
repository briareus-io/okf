# guidance

```json
{
  "content": "full|summary|shared",
  "context_digest": "string",
  "expand_argv": "array<string> (summary in explicit context)",
  "id": "string",
  "instruction_role": "task-guidance|inspected",
  "instructions": "string (full only)",
  "instructions_ref": "{kind,id} of an earlier full record in this bundle (shared only)",
  "omission_reason": "string (summary)",
  "origin": "object",
  "selection_reason": "string (explicit context)",
  "tier": "mandatory|reference (rules)"
}
```
