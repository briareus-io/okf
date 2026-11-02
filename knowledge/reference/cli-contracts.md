---
type: Reference
audience: user
title: CLI globals and record contracts
description: Generated global arguments and extension record shapes.
sources:
- resource: xtask/src/main.rs
  kind: git-path
- resource: crates/okf-cli/src/commands/schema.rs
  kind: git-path
---
# CLI globals and record contracts

> **Generated** by `cargo xtask docs` from `okf schema --json` and curated usage notes (tool 0.6.3, OKF spec 0.2). Do not hand-edit; regenerate instead.

See the [command index](../commands/index.md) for individual arguments and [command contracts](command-contracts.md) for shared behavior. Skills carry focused generated references with their workflows.

Commands with a human form accept global `--json` for NDJSON; `schema` is always NDJSON. Most bundle-aware commands take an optional trailing `bundle` path or a separate `--bundle-id` selector; `format` uses `--bundle <path>` and an optional concept ID positional. Selection uses explicit path or ID, `$OKF_BUNDLE` (path), registered containing bundle, configured default, sole catalog entry, then uncataloged cwd. Catalog registration alone does not extend examination scope. Meta commands have no bundle, and `source-scan` takes an explicit arbitrary directory.

Exit codes: 0 means success under the selected failure threshold, not necessarily no findings; 1 means findings or an unsuccessful resolution; 2 means usage errors; 3 means environment/I/O/YAML errors; 4 means an internal error. Inspect findings even with `--fail-on never`. NDJSON is one record per line, not a JSON array. In CLI 0.6, JSON operations begin with `operation-start` and normally end with `operation-complete`; require the terminal record and check the actual exit before accepting a complete result. A successful bounded read may still report partial examination or matching. Preview and dry-run operations do not publish authored content but may create coordination state.

## Global arguments

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `--json` | bool | no | Emit NDJSON instead of human text or a bare artifact; schema is always NDJSON (default: `false`) |
| `--bundle-id <value>` | string | no | Select an explicitly registered bundle identity (paths keep separate meanings) |
| `--scope-bundle <value>` | list<string> | no | Add a registered bundle to the examination scope (repeatable) |
| `--catalog-scope` | bool | no | Examine every locally available registered bundle (default: `false`) |
| `--revision <value>` | string | no | Inspect a locally available Git revision; never fetches missing objects |
| `--catalog <value>` | string | no | Select a catalog explicitly; relative paths are resolved from the invocation directory |
| `--guidance <value>` | string | no | Incidental guidance delivery; explicit context requests always return selected instructions |
| `--context <value>` | list<string> | no | Check a previously loaded context token before execution (repeat per scoped bundle; @file supported) |
| `--workspace-revision <value>` | string | no | Inspect the consumer repository and its catalog/lock at this revision |
| `--bundle-revision <value>` | list<string> | no | Inspect a registered bundle at an exact local commit, ID=COMMIT (repeatable) |

Scope defaults to the selected bundle. Each command advertises its supported scope, snapshot, catalog, and guidance options below; unsupported options are errors. Catalog registration does not authorize traversal. Explicit historical inspection never acquires objects or accepts a new lock. Scope and effective settings are interpretation context, not authored concept metadata.

## Extension record contracts

Authored concept metadata remains open. These computed records describe scope, identity, interpretation, and explicit query extensions. Missing and null remain distinct.

```json
{
  "baseline": {
    "base_digest": "opaque string",
    "bundle": "path",
    "schema_version": 1
  },
  "bundle-affected": {
    "bundle": "string",
    "id": "string",
    "version": "string"
  },
  "bundle-backlink": {
    "bundle": "string",
    "id": "string",
    "version": "string"
  },
  "bundle-context": {
    "context_digest": "string",
    "context_digest_version": 2,
    "effective_settings": "object",
    "manifest": "object|null",
    "selection": "object",
    "setting_origins": "object",
    "settings_projection": "{detail:full|compact,omitted:array<string>}",
    "state_digest": "string"
  },
  "bundle-edge": {
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
  },
  "bundle-node": {
    "bundle": "string",
    "id": "string",
    "version": "string"
  },
  "bundle-registration": {
    "available": "boolean",
    "configured_root": "path",
    "id": "string",
    "overridden": "boolean",
    "root": "path"
  },
  "bundle-resource": {
    "content": "string|null",
    "path": "path",
    "sha256": "string|null",
    "truncated": "boolean"
  },
  "catalog-update-plan": {
    "impact": "object",
    "impact_complete": "boolean",
    "plan_digest": "string",
    "plan_kind": "initial|update|registration-change"
  },
  "changeset-file": {
    "after_bytes": "integer|null",
    "before_bytes": "integer|null",
    "diff": "string|null",
    "operation": "create|replace|remove",
    "path": "path"
  },
  "changeset-recovery": {
    "paths": "array<path>"
  },
  "changeset-summary": {
    "applied": "boolean",
    "base_digest": "string",
    "files": "integer",
    "publication": "journaled-per-file"
  },
  "concept": {
    "discriminator": "no kind field is added; unprojected concept metadata",
    "frontmatter_conflicts": "optional object preserving authored values whose names collide with computed fields, including authored id",
    "id": "computed suffix-free bundle-relative concept path",
    "identity_projection": "--project '$id' selects computed identity; --project id selects authored frontmatter id",
    "trust_tier": "unverified|machine-confirmed|human-reviewed",
    "verification_current": "boolean|null; latest valid verification compared with generated.at (legacy timestamp fallback); null when either timestamp is unavailable"
  },
  "concept-identity": {
    "bundle": "string",
    "id": "string",
    "schema_version": 1,
    "version": "string"
  },
  "context-check": {
    "context_digest": "string",
    "operations": "array<string>",
    "root": "path",
    "state_digest": "string",
    "valid": "boolean"
  },
  "context-scope": {
    "complete": "boolean",
    "examined": "array<object>",
    "requested": "array<string>",
    "unavailable": "array<string>"
  },
  "context-stats": {
    "budget_exceeded": "boolean; advisory only",
    "delivered_instruction_bytes": "integer",
    "mandatory_budget": "integer|null",
    "mandatory_instruction_bytes": "integer",
    "root": "path",
    "sections": "array<{id,kind,delivered_bytes,mandatory}>"
  },
  "context-token": {
    "bundle": "string|null",
    "root": "path",
    "token": "string; opaque stateless receipt, not a credential"
  },
  "diagnostic-summary": {
    "displayed": "integer",
    "examined": "integer",
    "omitted": "integer",
    "schema_version": 1
  },
  "doctor-normalization": {
    "applied": "boolean",
    "diff": "string",
    "links": "array<{from:string,to:string}>",
    "path": "path"
  },
  "drift": {
    "concept": "string",
    "expired": "timestamp|null",
    "schema_version": 2,
    "sources": "array<{resource:string,kind:string,state:drifted|unrecorded|missing|unavailable|unsupported,code:string,message:string}>"
  },
  "effective-settings": {
    "bundle": "string",
    "settings": "interpretation-settings"
  },
  "expansion-summary": {
    "bounds": "{edges_per_hit:integer,targets:integer,bytes:integer}",
    "emitted_bytes": "integer",
    "emitted_edges": "integer",
    "emitted_targets": "integer",
    "reasons": "array<string>",
    "schema_version": 1,
    "total_edges": "integer|null",
    "truncated": "boolean"
  },
  "facet": {
    "basis": "all-matches|observed-matches",
    "complete": "boolean",
    "diagnostics": "array<object>",
    "field": "string",
    "omitted_values": "integer",
    "schema_version": 1,
    "truncated": "boolean",
    "values": "array<{value:scalar,count:integer}>"
  },
  "facet-excluded": {
    "complete": "boolean",
    "distinct_values": "integer",
    "field": "string",
    "reason": "string",
    "schema_version": 1,
    "threshold": "integer"
  },
  "format-file": {
    "applied": "boolean",
    "diff": "string|null",
    "path": "path",
    "schema_version": 1
  },
  "format-summary": {
    "applied": "boolean",
    "files": "integer",
    "schema_version": 1
  },
  "guidance": {
    "content": "full|summary",
    "context_digest": "string",
    "expand_argv": "array<string> (summary in explicit context)",
    "id": "string",
    "instruction_role": "task-guidance|inspected",
    "instructions": "string (full only)",
    "omission_reason": "string (summary)",
    "origin": "object",
    "selection_reason": "string (explicit context)",
    "tier": "mandatory|reference (rules)"
  },
  "guidance-scope": {
    "complete": "boolean",
    "requested": "array<string>",
    "unavailable": "array<string>"
  },
  "interpretation-settings": {
    "bundle": "string|null",
    "ontology_digest": "string|null",
    "ontology_path": "path|null",
    "settings": "object"
  },
  "migration-applied": {
    "report": "object"
  },
  "migration-plan": {
    "plan": "object",
    "saved": "path|null"
  },
  "ontology_field_type": {
    "definition": "object",
    "effective": "object",
    "name": "string"
  },
  "operation-complete": {
    "commit_reached": "boolean|null (when recovery-required)",
    "completeness": "{examination,matching,delivery,expansion,content: complete|partial|not-applicable; consistency: working-tree|immutable-revision|guarded-baseline}",
    "exit_code": "integer",
    "outcome": "ok|partial|failed",
    "publication": "not-applicable|not-published|committed|recovery-required",
    "reasons": "array<diagnostic-code>",
    "schema_version": 1
  },
  "operation-start": {
    "command": "string",
    "schema_version": 1,
    "stream_version": 1
  },
  "projection": {
    "bundle": "string|null",
    "fields": "array<{field:string,present:boolean,value?:any,occurrences?:array<{path:string,value:any}>,empty_lists?:integer}>",
    "id": "string",
    "schema_version": 1,
    "version": "string"
  },
  "qualified_identity": {
    "bundle": "string",
    "id": "string",
    "version": "string"
  },
  "query-filters": {
    "filters": "object",
    "origin": "string",
    "preset": "string|null"
  },
  "query-summary": {
    "examined_documents": "integer",
    "filters": "array<string>",
    "has_more": "boolean",
    "interpretation": "array<interpretation-settings>",
    "limit": "integer|null",
    "next_offset": "integer|null",
    "observed_matches": "integer",
    "offset": "integer",
    "partial": "boolean",
    "returned": "integer",
    "scan_complete": "boolean",
    "scan_limit": "integer|null",
    "schema_version": 1,
    "scope": "array<{bundle:string,root:path,version:string,mutable:boolean}>",
    "total_matches": "integer|null"
  },
  "related-concept": {
    "bundle": "string|null",
    "fields": "projection.fields",
    "id": "string",
    "identity": "string",
    "schema_version": 1,
    "version": "string"
  },
  "relationship": {
    "bundle": "string",
    "edge": "object",
    "primary": "string",
    "schema_version": 1,
    "target_identity": "string|null",
    "target_payload_status": "string (optional)",
    "version": "string"
  },
  "relationship-query-summary": {
    "evidence": "object (local)"
  },
  "scope": {
    "examined": "array<{id:string,root:path,version:string,interpretation:interpretation-settings}>",
    "requested": "array<string>",
    "snapshot_examined": "array<qualified_identity> (optional)",
    "unavailable": "array<string>"
  },
  "snapshot-context": {
    "evidence": "array<object>"
  },
  "validation-profile": {
    "conformant": "boolean",
    "profile": "bundle"
  },
  "warning": {
    "limit": "integer|null",
    "message": "string",
    "omitted": "integer|null",
    "reason": "string",
    "schema_version": 1
  },
  "workflow": {
    "content": "full|summary",
    "id": "string",
    "inputs_complete": "boolean (selected only)",
    "selected": "boolean"
  }
}
```
