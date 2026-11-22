# feedback

```json
{
  "action": "learn subcommand",
  "data": {
    "draft": "{file:path,text:string}",
    "enable|disable|purge": "{enabled:boolean}",
    "export": "{destination:path,text:string,confirmation:string} or {exported:path}",
    "report": "{enabled:boolean,patterns:array<{version,command,pattern,affected,invocations}>}",
    "status": "{directory:path,enabled:boolean,drafts:integer,bytes:integer}"
  }
}
```
