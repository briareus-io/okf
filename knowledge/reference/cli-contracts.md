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

Generated from `okf schema` for CLI 0.6.11. See the [command index](../commands/index.md) for individual arguments and [command contracts](command-contracts.md) for shared behavior.

Commands with a human form accept global `--json` for NDJSON; `schema` is always NDJSON. Most bundle-aware commands take an optional trailing `bundle` path or a separate `--bundle-id` selector; `format` uses `--bundle <path>` and an optional concept ID positional. Selection uses explicit path or ID, `$OKF_BUNDLE` (path), registered containing bundle, configured default, sole catalog entry, then uncataloged cwd. Catalog registration alone does not extend examination scope. Meta commands have no bundle, and `source-scan` takes an explicit arbitrary directory.

Exit codes: 0 means success under the selected failure threshold, not necessarily no findings; 1 means findings or an unsuccessful resolution; 2 means usage errors; 3 means environment/I/O/YAML errors; 4 means an internal error. Inspect findings even with `--fail-on never`. NDJSON is one record per line, not a JSON array. In CLI 0.6, JSON operations begin with `operation-start` and normally end with `operation-complete`; require the terminal record and check the actual exit before accepting a complete result. A successful bounded read may still report partial examination or matching. Preview and dry-run operations do not publish authored content but may create coordination state.

## Global arguments

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

Scope defaults to the selected bundle. Each command advertises its supported scope, snapshot,
catalog, and guidance options below; unsupported options are errors. Catalog registration does not
authorize traversal. Explicit historical inspection never acquires objects or accepts a new lock.
Scope and effective settings are interpretation context, not authored concept metadata.

## Extension record contracts

```json
{
  "baseline": {
    "base_digest": "opaque string",
    "bundle": "path",
    "schema_version": 1,
    "scope": "{requested:array<string>,examined:array<string>,unavailable:array<string>} (optional; explicit scoped baseline, includes primary)"
  },
  "body": {
    "body": "string",
    "id": "string"
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
  "bundle-selection": {
    "access": "read-only|read-write",
    "id": "string|null (null for standalone selection)",
    "root": "canonical existing directory path",
    "source": "standalone|directory|git-override|git-snapshot",
    "token": "opaque versioned string"
  },
  "catalog-update-plan": {
    "impact": "object",
    "impact_complete": "boolean",
    "plan_digest": "string",
    "plan_kind": "initial|update|registration-change"
  },
  "changeset-dangling-override": {
    "occurrence": "string"
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
    "allowed_dangling": "integer",
    "applied": "boolean",
    "base_digest": "string",
    "files": "integer",
    "publication": "journaled-per-file",
    "scope": "{requested:array<string>,examined:array<string>,unavailable:array<string>} (optional; explicit scoped baseline, includes primary)"
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
    "provenance_only": "integer",
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
  "feedback": {
    "action": "learn subcommand",
    "data": {
      "draft": "{file:path,text:string}",
      "enable|disable|purge": "{enabled:boolean}",
      "export": "{destination:path,text:string,confirmation:string} or {exported:path}",
      "report": "{enabled:boolean,patterns:array<{version,command,pattern,affected,invocations}>}",
      "status": "{directory:path,enabled:boolean,drafts:integer,bytes:integer}"
    }
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
  "line-range": {
    "end": "integer",
    "id": "string",
    "lines": "array<{line:integer,text:string}>",
    "start": "integer",
    "trust_tier": "string"
  },
  "lint-baseline": {
    "bundles": "map<bundle,version> (file capture)",
    "entries": "integer (file capture)",
    "path": "path (file capture)",
    "revision": "commit (Git comparison)"
  },
  "lint-comparison": {
    "classification": "introduced|existing|worsened|improved|resolved",
    "count": "positive integer",
    "identity": "{bundle,rule,concept,code,field_path,subject}",
    "message": "string",
    "previous_severity": "info|warn|error|null",
    "severity": "info|warn|error"
  },
  "lint-comparison-summary": {
    "after_bundles": "map<bundle,version>",
    "before_bundles": "map<bundle,version>",
    "counts": "map<classification,count>"
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
  "outline": {
    "headings": "array<{line:integer,level:integer,text:string}>",
    "id": "string",
    "trust_tier": "string"
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
    "filters": "object (relationship filters)",
    "origin": "string",
    "preset": "string|null",
    "query": "object (primary query preset)"
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
    "delivery_complete": "boolean (scoped)",
    "eligible_documents": "integer|null",
    "evidence": "object (local)",
    "examined_documents": "integer|null (scoped)",
    "examined_occurrences": "integer (scoped)",
    "matched_occurrences": "integer (scoped)",
    "scan_complete": "boolean (scoped)",
    "total_matches": "integer|null",
    "unresolved_candidates": "integer (scoped)"
  },
  "saved-query-summary": {
    "delivery_complete": "boolean",
    "examination": "object",
    "matching_complete": "boolean",
    "name": "string",
    "observed_matches": "integer",
    "section_errors": "integer",
    "stages": "array",
    "strategy": "outgoing|scope",
    "unresolved": "integer"
  },
  "scope": {
    "examined": "array<{id:string,root:path,version:string,interpretation:interpretation-settings}>",
    "requested": "array<string>",
    "snapshot_examined": "array<qualified_identity> (optional)",
    "unavailable": "array<string>"
  },
  "section": {
    "end": "integer",
    "id": "string",
    "path": "array<string>",
    "start": "integer",
    "text": "string"
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
