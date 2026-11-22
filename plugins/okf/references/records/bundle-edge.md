# bundle-edge

```json
{
  "evidence": "string",
  "fingerprint_status": "string|null",
  "location": "string",
  "relationship": "{source:string,rule:string,field_path:string,raw_reference:string,target:string,authored_kind:string|null,inverse:string|null,attributes:object,status:string}|null",
  "resource": "string",
  "resource_bundle": "string (non-concept resources only)",
  "resource_kind": "reserved|artifact|support (non-concept resources only)",
  "snapshot": "{requested:object,status:string,evidence:string,resolved:qualified_identity|null,candidate:qualified_identity|null}|null",
  "source": "qualified_identity",
  "status": "string",
  "target": "qualified_identity|null"
}
```
