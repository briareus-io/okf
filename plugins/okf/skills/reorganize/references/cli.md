# okf CLI — reorganize command reference

> **Generated** by `cargo xtask docs` from `okf schema --json` and curated usage notes (tool 0.6.1, OKF spec 0.2). Do not hand-edit; regenerate instead.

This focused reference contains only commands selected for the `reorganize` workflow. First pass the skill CLI compatibility preflight. Consult this reference when exact arguments or output shapes are needed. For a compatible CLI that rejects documented syntax, use that command's `--help` output as the runtime authority.

Commands with a human form accept global `--json` for NDJSON; `schema` is always NDJSON. Bundle-aware commands take an optional trailing `bundle` path or a separate `--bundle-id` selector. Selection uses explicit path or ID, `$OKF_BUNDLE` (path), registered containing bundle, configured default, sole catalog entry, then uncataloged cwd. Catalog registration alone does not extend examination scope. Meta commands have no bundle, and `source-scan` takes an explicit arbitrary directory.

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
  "bundle-backlink": {
    "bundle": "string",
    "id": "string",
    "version": "string"
  },
  "bundle-context": {
    "context_digest": "string",
    "context_digest_version": 1,
    "effective_settings": "object",
    "manifest": "object|null",
    "selection": "object",
    "setting_origins": "object"
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
  "context-scope": {
    "complete": "boolean",
    "examined": "array<object>",
    "requested": "array<string>",
    "unavailable": "array<string>"
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
  "guidance": {
    "content": "full|summary",
    "context_digest": "string",
    "id": "string",
    "instruction_role": "task-guidance|inspected",
    "instructions": "string (full only)",
    "origin": "object"
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
  "scope": {
    "examined": "array<{id:string,root:path,version:string,interpretation:interpretation-settings}>",
    "requested": "array<string>",
    "snapshot_examined": "array<qualified_identity> (optional)",
    "unavailable": "array<string>"
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

## meta

### `okf catalog`

List effective catalog registrations, locations, and availability.

_No arguments._

Output stream: `bundle-registration,effective-settings`.

Supported global options: `--catalog`, `--json`.

## query

### `okf artifact list`

List local artifacts and concepts under a bundle directory.

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<bundle>` | positional | no | Bundle root directory; use --directory to select a directory within the bundle |
| `--directory <value>` | string | no | Bundle-relative directory to inventory (default: `references`) |
| `--exclude-concepts` | bool | no | Exclude concepts, retaining opaque artifacts and reserved files (default: `false`) |
| `--digest` | bool | no | Compute SHA-256 digests (reads each file) (default: `false`) |

Output stream: `artifact`.

Supported global options: `--bundle-id`, `--bundle-revision`, `--catalog`, `--guidance`, `--json`, `--revision`, `--workspace-revision`.

The positional path selects a bundle root relative to the current directory. To filter within the selected bundle, use `--directory contracts/x`; printed paths remain relative to the bundle root. Use `--directory .` for the entire bundle and `--exclude-concepts` to retain only opaque artifacts and reserved files.

### `okf artifact resolve`

Resolve any OKF path-valued resource with document context.

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<resource>` | positional | yes | Resource path, URL, or scope descriptor |
| `<bundle>` | positional | no | Bundle directory (explicit, then $OKF_BUNDLE, nearest okf.toml, or current directory) |
| `--from <value>` | string | no | Resolve a relative resource against this declaring concept id |

Output stream: `artifact-resolution`.

Supported global options: `--bundle-id`, `--bundle-revision`, `--catalog`, `--catalog-scope`, `--guidance`, `--json`, `--revision`, `--scope-bundle`, `--workspace-revision`.

**Path namespaces:** a leading `/` means bundle-root-relative, not an operating-system absolute path. Other local paths resolve against the declaring concept's directory when `--from` is supplied, otherwise the bundle root. Keep `--from` on the subsequent read too. JSON uses `artifact_kind` (concept, artifact, reserved, support, external, scope, missing, or blocked), `path`, `exists`, `size`, and `message`. Missing/blocked resolution exits 1. A scope descriptor is provenance, not a missing file.

**Repository sources:** with repo `/work/app`, bundle `/work/app/knowledge`, and declaring concept `notes/service`, `/references/spec.txt` resolves to `/work/app/knowledge/references/spec.txt`. A `kind=git-path` or `git-commit` source `src/service.rs` is instead fingerprinted from the Git worktree root as `/work/app/src/service.rs`; the artifact resolver does not reinterpret paths by source kind. File/line-range/markdown-heading sources use bundle-root-relative paths in fingerprinting, doctor, and graph resolution; kind-less provenance and body links remain document-relative. Generic artifact commands cannot infer a source kind from the resource string: omit `--from` for a typed bundle-root source path. Correct typed source paths that previously relied on graph's document-relative interpretation. Ordinary cross-bundle paths and logical references require a registered target in explicit examination scope. Record the actual source location and convention; do not assume fingerprint and artifact paths coincide. Inspect external repository evidence with normal source tools within existing read authorization. Do not rewrite provenance, bypass containment, or mirror files merely to make artifact resolution succeed.

### `okf artifact show`

Retrieve a bounded local text artifact; binary files return metadata only.

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<resource>` | positional | yes | Artifact path relative to the bundle root, or to --from when provided |
| `<bundle>` | positional | no | Bundle directory (explicit, then $OKF_BUNDLE, nearest okf.toml, or current directory) |
| `--from <value>` | string | no | Resolve a relative resource against this declaring concept id |
| `--lines <value>` | string | no | Retrieve only an inclusive, one-based START:END line range |
| `--max-bytes <value>` | int | no | Maximum bytes read into output (default: `65536`) |
| `--fetch` | bool | no | Request remote retrieval; unavailable unless built with url-sources and allowed by policy (default: `false`) |

Output stream: `artifact-content`.

Supported global options: `--bundle-id`, `--bundle-revision`, `--catalog`, `--catalog-scope`, `--guidance`, `--json`, `--revision`, `--scope-bundle`, `--workspace-revision`.

For document-relative paths, pass the same `--from` used during resolution. For bundle-root-relative file/line-range/markdown-heading source resources, omit `--from`; this generic reader does not infer the source kind. JSON includes `text`, `binary`, `truncated`, `size`, `sha256`, and `path`. Binary files provide metadata only. Inspect `truncated` before treating a read as complete; human output alone does not expose this flag. Use bounded line windows and a sufficient byte budget for the needed range; do not infer absence from a truncated result. Use `show` for concepts and this command for opaque or reserved files.

Local artifact reads remain inside the resolved target bundle and requested examination scope. For an HTTP(S) URL, `--fetch` requires a network-enabled build and authorization covering that source. A user request to inspect a named source can supply that authorization; no separate OKF policy file is specified by this command. If unavailable, use an authorized external retrieval tool or report the evidence gap. Do not send ambient credentials. Reading computation, executor, or attester code never authorizes execution.

### `okf backlinks`

Concepts that link to a given concept.

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<concept>` | positional | yes | — |
| `<bundle>` | positional | no | — |
| `--details` | bool | no |  (default: `false`) |
| `--kind <value>` | list<string> | no | — |
| `--inverse-kind <value>` | list<string> | no | — |
| `--related-type <value>` | list<string> | no | — |
| `--rule <value>` | list<string> | no | — |
| `--edge-filter <value>` | list<string> | no | — |
| `--related-filter <value>` | list<string> | no | — |
| `--preset <value>` | string | no | — |
| `--clear-filter <value>` | list<string> | no | — |
| `--limit <value>` | string | no | — |
| `--offset <value>` | string | no |  (default: `0`) |
| `--scan-limit <value>` | string | no | — |
| `--full-scan` | bool | no |  (default: `false`) |

Output stream: `concept,relationship,scope,bundle-backlink,bundle-edge`.

Supported global options: `--bundle-id`, `--bundle-revision`, `--catalog`, `--catalog-scope`, `--guidance`, `--json`, `--revision`, `--scope-bundle`, `--workspace-revision`.

### `okf bundle context`

Return effective settings and selected agent guidance without running a workflow.

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<bundle>` | positional | no | — |
| `--operation <value>` | list<string> | no | — |
| `--workflow <value>` | list<string> | no | — |
| `--all` | bool | no |  (default: `false`) |
| `--inputs <value>` | string | no | JSON input object or @file; values remain typed data |
| `--query <value>` | string | no | JSON query object or @file; resolve without executing the query |

Output stream: `context-scope,bundle-context,guidance,workflow,effective-query`.

Supported global options: `--bundle-id`, `--bundle-revision`, `--catalog`, `--catalog-scope`, `--guidance`, `--json`, `--revision`, `--scope-bundle`, `--workspace-revision`.

Load global task context before source access, then full applicable command guidance before acting. Context includes effective settings and origins, instruction content/completeness, and an effective digest including loaded instruction files. --all and historical context are inspection, not workflow activation. Missing required instructions block dependent agent work. Incidental guidance output does not replace pre-action loading.

### `okf bundle resource list`

Inventory support files and the manifest without loading instruction bodies.

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<bundle>` | positional | no | Bundle directory (explicit, then $OKF_BUNDLE, nearest okf.toml, or current directory) |

Output stream: `bundle-resource`.

Supported global options: `--bundle-id`, `--bundle-revision`, `--catalog`, `--guidance`, `--json`, `--revision`, `--workspace-revision`.

Support files are excluded from ordinary knowledge discovery. Explicit inspection is bounded, preserves containment, and never executes scripts or treats support Markdown as concepts.

### `okf bundle resource show`

Read a bounded support file without executing it.

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<resource>` | positional | yes | — |
| `<bundle>` | positional | no | — |
| `--max-bytes <value>` | string | no |  (default: `65536`) |
| `--lines <value>` | string | no | — |

Output stream: `bundle-resource`.

Supported global options: `--bundle-id`, `--bundle-revision`, `--catalog`, `--guidance`, `--json`, `--revision`, `--workspace-revision`.

Support files are excluded from ordinary knowledge discovery. Explicit inspection is bounded, preserves containment, and never executes scripts or treats support Markdown as concepts.

### `okf changeset digest`

Acquire the guarded bundle baseline token without staging changes.

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<bundle>` | positional | no | Bundle directory |

Output stream: `baseline`.

Supported global options: `--bundle-id`, `--catalog`, `--guidance`, `--json`.

### `okf changeset plan`

Validate and preview a coordinated change document without writing bundle files.

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<bundle>` | positional | no | Bundle directory |
| `--from <value>` | string | yes | Version 1 change document: inline YAML/JSON, @file, or stdin (-) |
| `--expect <value>` | string | no | Require this base digest from a previous plan before applying |
| `--dry-run` | bool | no | Validate and preview without publishing authored files (also the behavior of changeset plan) (default: `false`) |
| `--require-ontology-gate` | bool | no | Require the ontology write gate even when bundle policy leaves it disabled (default: `false`) |

Output stream: `changeset-file,changeset-summary`.

Supported global options: `--bundle-id`, `--catalog`, `--guidance`, `--json`.

Supply a version 1 YAML/JSON document with an ordered `operations` list. Supported operations: `create`/`replace` with `path` and complete Markdown `content`; `edit` with `concept`, optional frontmatter RFC 6902 `patch` and optional `body`; `move` with `from`/`to` concept IDs; `retarget` with `from`/`to` and optional `within` concept-ID list; `remove` with an exact bundle-relative file `path`; `put-artifact` with `path`, text `content`, and optional `replace: true`. Splits and merges combine creates, explicit retarget mappings, and removal; no semantic mapping is inferred. Operations run on an isolated copy; the final bundle must be conformant and introduce no dangling supported local references. Ordering matters: editing a link to `/old.md` before moving `/old` to `/new` lets the move rewrite it; writing that old link after the move introduces a dangling reference and is rejected. Prefer final target paths when practical. Checks cover local Markdown/artifact links and file, line-range, and markdown-heading sources; Git, URL, and producer-specific sources are not checked. Existing unrelated broken references may remain. Concept replacement/edit invalidates verification; content cannot supply verification. Plan and `apply --dry-run` print all file diffs plus a base digest without publishing authored bundle content; coordination state may be created. Use `apply --expect <base-digest>` to guard a reviewed baseline, with the same reviewed change document. Apply without `--expect` already prepares, validates, and rechecks its publication baseline in one invocation; it cannot guard an application decision made before that invocation. Publication is journaled and atomic per file. Ordinary working-tree CLI reads hold shared publication guards through payload delivery and refuse an active publisher or pending recovery journal; arbitrary filesystem readers can still see intermediate files. Separate CLI calls are not one snapshot, and external edits bypass the guards. Managed immutable snapshots need no coordination writes; a read-only root without an existing lock is readable without a coordination guarantee against privileged or external writers. Cooperating CLI writers refuse a pending transaction. Cross-bundle writes and configuration changes are unsupported. On interrupted publication use `changeset recover`; do not remove the journal manually.

### `okf graph`

Render the link graph (or a bounded rooted neighborhood) as mermaid/dot/graphml.

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `--kind <value>` | list<string> | no | — |
| `--inverse-kind <value>` | list<string> | no | — |
| `--related-type <value>` | list<string> | no | — |
| `--rule <value>` | list<string> | no | — |
| `--edge-filter <value>` | list<string> | no | — |
| `--related-filter <value>` | list<string> | no | — |
| `--preset <value>` | string | no | — |
| `--clear-filter <value>` | list<string> | no | — |
| `<bundle>` | positional | no | Bundle directory (explicit, then $OKF_BUNDLE, nearest okf.toml, or current directory) |
| `--root <value>` | string | no | Optional concept at the neighborhood root; without one, render the entire graph |
| `--format <value>` | string | no | Output format: mermaid (default), dot, or graphml (default: `mermaid`) |
| `--direction <value>` | string | no | Edges to follow from the root: outgoing (default), incoming, or both (default: `outgoing`) |
| `--depth <value>` | int | no | Maximum neighbor distance from the root (0 = root only; default: unbounded) |

Output stream: `graph,scope,bundle-node,bundle-edge`.

Supported global options: `--bundle-id`, `--bundle-revision`, `--catalog`, `--catalog-scope`, `--guidance`, `--json`, `--revision`, `--scope-bundle`, `--workspace-revision`.

### `okf links`

List the direct concept links defined by one concept.

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<concept>` | positional | yes | — |
| `<bundle>` | positional | no | — |
| `--details` | bool | no |  (default: `false`) |
| `--kind <value>` | list<string> | no | — |
| `--inverse-kind <value>` | list<string> | no | — |
| `--related-type <value>` | list<string> | no | — |
| `--rule <value>` | list<string> | no | — |
| `--edge-filter <value>` | list<string> | no | — |
| `--related-filter <value>` | list<string> | no | — |
| `--preset <value>` | string | no | — |
| `--clear-filter <value>` | list<string> | no | — |
| `--limit <value>` | string | no | — |
| `--offset <value>` | string | no |  (default: `0`) |
| `--scan-limit <value>` | string | no | — |
| `--full-scan` | bool | no |  (default: `false`) |

Output stream: `link,relationship,scope,bundle-edge`.

Supported global options: `--bundle-id`, `--bundle-revision`, `--catalog`, `--catalog-scope`, `--guidance`, `--json`, `--revision`, `--scope-bundle`, `--workspace-revision`.

### `okf list`

List all concepts (search with no filter).

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `--preset <value>` | string | no | — |
| `--clear-filter <value>` | list<string> | no | — |
| `--expand-kind <value>` | list<string> | no | — |
| `--expand-inverse-kind <value>` | list<string> | no | — |
| `--expand-related-type <value>` | list<string> | no | — |
| `--expand-edge-filter <value>` | list<string> | no | — |
| `--expand-related-filter <value>` | list<string> | no | — |
| `<bundle>` | positional | no | Bundle directory (explicit, then $OKF_BUNDLE, nearest okf.toml, or current directory) |
| `--type <value>` | string | no | Filter by exact concept `type` |
| `--tag <value>` | string | no | Filter by membership in the concept's `tags` |
| `--text <value>` | list<string> | no | Text query (repeatable). Repeated phrases use AND semantics by default |
| `--match <value>` | string | no | Text matching: phrase (default), all tokens, any token, or literal source text (default: `phrase`) |
| `--in <value>` | list<string> | no | Text fields to search (comma-separated or repeatable) (default: `id,title,description,body`) |
| `--sort <value>` | string | no | Result order: deterministic relevance (default) or concept id (default: `relevance`) |
| `--limit <value>` | int | no | Return at most this many results after all filters and sorting |
| `--field <value>` | list<string> | no | Filter by a frontmatter field, `key=value` (repeatable; AND) |
| `--facet-filter <value>` | list<string> | no | Typed selector condition: =, !=, [not] in, exists, absent, is [not] null; join with OR (repeatable; AND) |
| `--facets` | bool | no | Emit JSON facets over all matches, before pagination (default: `false`) |
| `--facet <value>` | list<string> | no | Facet selector (repeatable); does not bypass high cardinality guard |
| `--offset <value>` | int | no | Start page at this nonnegative offset; requires a positive limit |
| `--scan-limit <value>` | int | no | Maximum examined documents across scope (positive) |
| `--full-scan` | bool | no | Examine the entire selected scope without a document scan budget (default: `false`) |
| `--project <value>` | list<string> | no | Explicit metadata projection selector (repeatable) |
| `--columns <value>` | list<string> | no | Human output column selector (repeatable) |
| `--view <value>` | string | no | Named bundle display view |
| `--expand <value>` | list<string> | no | Expand selected outbound relationship rule (repeatable) |
| `--no-expand` | bool | no | Disable configured human view expansion (default: `false`) |
| `--target-field <value>` | list<string> | no | Metadata selector for related targets (repeatable) |
| `--expansion-edges <value>` | int | no | Maximum edge occurrences per primary hit (1..100) (default: `10`) |
| `--expansion-targets <value>` | int | no | Maximum distinct target payloads per query (1..1000) (default: `100`) |
| `--expansion-bytes <value>` | int | no | Maximum serialized expansion bytes (1..1048576) (default: `262144`) |

Output stream: `concept,concept-identity,scope,projection,query-summary,facet,facet-excluded,relationship,related-concept,expansion-summary,warning`.

Supported global options: `--bundle-id`, `--bundle-revision`, `--catalog`, `--catalog-scope`, `--guidance`, `--json`, `--revision`, `--scope-bundle`, `--workspace-revision`.

JSON records contain frontmatter and computed lifecycle/trust metadata, not bodies. Aggregate field occurrence counts only after checking scan completeness and output warnings. The default scan budget is 1,000 eligible documents across scope; --limit bounds output, not scan work. For exhaustive inventory use a sufficient --scan-limit or deliberate --full-scan, and aggregate metadata locally rather than loading all records into context. Each offset invocation rescans; an incomplete scan has no authoritative continuation. Unprojected `id` is the computed path and `frontmatter_conflicts.id` preserves an authored ID; `--project '$id' --project id` selects these separately in projection records. Machine consumers can pass `--guidance off` or configure `guidance.display.json: off`; require both supported operation envelopes and successful process exit, not just a valid trailer when present.

### `okf show`

Show one concept's content, heading outline, or selected line range.

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<concept>` | positional | yes | Concept id (leading slash optional), e.g. `tables/customers` |
| `<bundle>` | positional | no | Bundle directory (explicit, then $OKF_BUNDLE, nearest okf.toml, or current directory) |
| `--outline` | bool | no | Show only the Markdown heading outline with 1-based document line numbers (default: `false`) |
| `--lines <value>` | string | no | Show only an inclusive 1-based document line range, `START:END` (or one line, `N`) |
| `--numbered` | bool | no | Print document line numbers (including frontmatter); excludes the display header (default: `false`) |
| `--body` | bool | no | Print only the raw Markdown body, without frontmatter or display headers (default: `false`) |
| `--project <value>` | list<string> | no | Explicit metadata projection selector (repeatable) |

Output stream: `concept,projection`.

Supported global options: `--bundle-id`, `--bundle-revision`, `--catalog`, `--guidance`, `--json`, `--revision`, `--workspace-revision`.

Without `--json`, show includes the serialized frontmatter and full Markdown body. Plain `show --json` returns metadata only: frontmatter plus `id`, `trust_tier`, `effective_status`, `effective_generated_at`, `latest_verified_at`, and `verification_current`. It does not include the body. `--outline --json` returns `headings` with `line`, `level`, and `text`; `--lines START:END --json` returns `start`, actual `end`, and `lines` containing `line` and `text`. Line numbers refer to the serialized document, including frontmatter, excluding the three-line display header. `show -n` (or `--numbered`) prints the full document with these numbers and no header; its JSON uses the same line-range record. `show --body` prints only raw Markdown; with `--json` it returns a body record containing `id` and `body`. An outline or selected slice does not establish complete document-review coverage. Unprojected `id` is the computed path; an authored frontmatter `id` is preserved in `frontmatter_conflicts.id`. `verification_current` compares a valid latest verification timestamp with the effective generation timestamp; null means the comparison is unavailable, not necessarily that the concept has never been reviewed.

## check

### `okf bundle validate`

Validate the manifest and explicitly declared local resources.

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<bundle>` | positional | no | Bundle directory (explicit, then $OKF_BUNDLE, nearest okf.toml, or current directory) |

Output stream: `manifest-validation`.

Supported global options: `--bundle-id`, `--catalog`, `--guidance`, `--json`.

Validate the optional manifest's structure and supported schema without treating prose instructions as executable checks. Manifest validation is distinct from core concept conformance and bundle-profile knowledge validation.

### `okf diff`

Concept-level diff of the working tree vs a git ref.

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<git_ref>` | positional | yes | Git ref to diff against (e.g. `HEAD`, a branch, or a commit) |
| `<bundle>` | positional | no | Bundle directory (explicit, then $OKF_BUNDLE, nearest okf.toml, or current directory) |
| `--fail-on <value>` | string | no | Fail (exit 1) on any change: never (default) | info | warn | error | any (default: `never`) |

Output stream: `diff`.

Supported global options: `--bundle-id`, `--catalog`, `--guidance`, `--json`.

### `okf lint` · _conditionally mutates_

Advisory checks (broken links, missing fields, orphans, ontology violations).

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<bundle>` | positional | no | Bundle directory (explicit, then $OKF_BUNDLE, nearest okf.toml, or current directory) |
| `--fix` | bool | no | Normalize frontmatter order bundle-wide for the enabled frontmatter-order rule (default: `false`) |
| `--fail-on <value>` | string | no | Severity threshold that makes the run fail (exit 1): never|info|warn|error|any (default: `error`) |

Output stream: `finding,scope,fix`.

Supported global options: `--bundle-id`, `--catalog`, `--catalog-scope`, `--guidance`, `--json`, `--scope-bundle`.

For sources with a fingerprint kind, `source-unrecorded` warns when no baseline exists. `source-missing` errors when file, line-range, or markdown-heading sources cannot be fingerprinted, even without a baseline. Lint does not fetch URLs or inspect Git sources. Configure `source_unrecorded: off` (or `info`, `warn`, `error`; default `warn`) under `settings.lint` in `okf-bundle.yaml`. Legacy TOML bundle settings remain compatibility input. Disabling this rule leaves `source-missing` and `stale` checks active. All lint settings accept `off`, `info`, `warn`, or `error`. Defaults: `broken_link` and `source_missing` are `error`; `missing_title`, `spec_v02` (finding rule `okf-v02`), `ontology_violation`, `source_unrecorded`, `invalid_reference`, and `index_coverage` are `warn`; `missing_description` and `orphan` are `info`. Omitted settings keep these defaults. `off` suppresses the selected lint rule only; validation and stale checks remain independent. `index_exclude` controls index coverage exclusions and `finding_budget` limits ontology findings (positive, default 1,000 per concept). Use `--fail-on warn` to fail on warnings as well as errors.

### `okf validate`

Conformance validation — the spec's three hard rules only.

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<bundle>` | positional | no | — |
| `--profile <value>` | string | no | Core preserves OKF v0.2 file boundaries; bundle excludes the support namespace (default: `core`) |

Output stream: `violation,validation-profile`.

Supported global options: `--bundle-id`, `--catalog`, `--guidance`, `--json`.

## mutate

### `okf changeset apply` · _conditionally mutates_

Validate then publish coordinated changes with a recoverable rollback journal.

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<bundle>` | positional | no | Bundle directory |
| `--from <value>` | string | yes | Version 1 change document: inline YAML/JSON, @file, or stdin (-) |
| `--expect <value>` | string | no | Require this base digest from a previous plan before applying |
| `--dry-run` | bool | no | Validate and preview without publishing authored files (also the behavior of changeset plan) (default: `false`) |
| `--require-ontology-gate` | bool | no | Require the ontology write gate even when bundle policy leaves it disabled (default: `false`) |

Output stream: `changeset-file,changeset-summary`.

Supported global options: `--bundle-id`, `--catalog`, `--guidance`, `--json`.

Supply a version 1 YAML/JSON document with an ordered `operations` list. Supported operations: `create`/`replace` with `path` and complete Markdown `content`; `edit` with `concept`, optional frontmatter RFC 6902 `patch` and optional `body`; `move` with `from`/`to` concept IDs; `retarget` with `from`/`to` and optional `within` concept-ID list; `remove` with an exact bundle-relative file `path`; `put-artifact` with `path`, text `content`, and optional `replace: true`. Splits and merges combine creates, explicit retarget mappings, and removal; no semantic mapping is inferred. Operations run on an isolated copy; the final bundle must be conformant and introduce no dangling supported local references. Ordering matters: editing a link to `/old.md` before moving `/old` to `/new` lets the move rewrite it; writing that old link after the move introduces a dangling reference and is rejected. Prefer final target paths when practical. Checks cover local Markdown/artifact links and file, line-range, and markdown-heading sources; Git, URL, and producer-specific sources are not checked. Existing unrelated broken references may remain. Concept replacement/edit invalidates verification; content cannot supply verification. Plan and `apply --dry-run` print all file diffs plus a base digest without publishing authored bundle content; coordination state may be created. Use `apply --expect <base-digest>` to guard a reviewed baseline, with the same reviewed change document. Apply without `--expect` already prepares, validates, and rechecks its publication baseline in one invocation; it cannot guard an application decision made before that invocation. Publication is journaled and atomic per file. Ordinary working-tree CLI reads hold shared publication guards through payload delivery and refuse an active publisher or pending recovery journal; arbitrary filesystem readers can still see intermediate files. Separate CLI calls are not one snapshot, and external edits bypass the guards. Managed immutable snapshots need no coordination writes; a read-only root without an existing lock is readable without a coordination guarantee against privileged or external writers. Cooperating CLI writers refuse a pending transaction. Cross-bundle writes and configuration changes are unsupported. On interrupted publication use `changeset recover`; do not remove the journal manually.

### `okf changeset recover` · _mutates_

Recover an interrupted publication without overwriting intervening edits.

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<bundle>` | positional | no | Bundle directory (explicit, then $OKF_BUNDLE, nearest okf.toml, or current directory) |

Output stream: `changeset-recovery`.

Supported global options: `--bundle-id`, `--catalog`, `--guidance`, `--json`.

Roll back an interrupted uncommitted publication, or finish cleanup of a committed one. Recovery refuses an active publisher and refuses to overwrite intervening edits. Preserve the journal if recovery reports a conflict; reconcile those files before retrying.

### `okf edit` · _conditionally mutates_

Edit a concept losslessly and invalidate its prior verification.

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<concept>` | positional | yes | Concept id to edit |
| `<bundle>` | positional | no | Bundle directory (explicit, then $OKF_BUNDLE, nearest okf.toml, or current directory) |
| `--set <value>` | list<string> | no | Set/update a scalar field, `key=value` (repeatable) |
| `--set-yaml <value>` | list<string> | no | Replace a field with YAML: `key=value`, `key=@file`, or `key=-` (repeatable) |
| `--set-path <value>` | list<string> | no | Set an object path with a YAML value, e.g. `deadline.within=72` (repeatable) |
| `--dry-run` | bool | no | Validate and show the resulting diff without writing (default: `false`) |
| `--unset <value>` | list<string> | no | Remove a field entirely, `key` (repeatable) |
| `--unset-path <value>` | list<string> | no | Delete an object path (repeatable); absent keys are a no-op |
| `--patch <value>` | string | no | Apply an RFC 6902 JSON Patch to frontmatter, supplied as YAML, `@file`, or `-` |
| `--add <value>` | list<string> | no | Append an item to a list field, `key=value` (repeatable, idempotent) |
| `--remove <value>` | list<string> | no | Remove matching item(s) from a list field, `key=value` (repeatable) |
| `--add-source <value>` | list<string> | no | Add a standard source, `resource=<path-or-uri>[,kind=<extension>][,id=...,...]` |
| `--add-source-json <value>` | list<string> | no | Add a full source mapping as JSON/YAML or `@file` (repeatable) |
| `--remove-source <value>` | list<string> | no | Remove sources matching `<path-or-uri>` or `resource=<path-or-uri>[,kind=<kind>]` (repeatable) |
| `--set-body <value>` | string | no | Replace the whole body. Use `@file` to read a file or `-` for stdin |
| `--append-body <value>` | string | no | Append a block to the body. Use `@file` or `-` (stdin) |
| `--replace <OLD> <NEW>` | list<string> | no | Replace body text, `<old> <new>` (repeatable; defaults to exactly one match) |
| `--all` | bool | no | Replace every occurrence matched by `--replace` (default: `false`) |
| `--clear-body` | bool | no | Empty the body (default: `false`) |
| `--set-section <HEADING> <TEXT>` | list<string> | no | Replace a section's content, `<heading> <text>` (repeatable). Text accepts `@file`/`-` |
| `--append-section <HEADING> <TEXT>` | list<string> | no | Append to a section, `<heading> <text>` (repeatable). Text accepts `@file`/`-` |
| `--remove-section <value>` | list<string> | no | Remove a section (heading + content), `<heading>` (repeatable) |
| `--rename-section <OLD> <NEW>` | list<string> | no | Rename a section heading, `<old> <new>` (repeatable) |
| `--require-ontology-gate` | bool | no | Require the ontology write gate even when bundle policy leaves it disabled (default: `false`) |

Output stream: `change`.

Supported global options: `--bundle-id`, `--catalog`, `--guidance`, `--json`.

`--set` keeps literal scalar keys; a dotted key is not traversal. `--set-yaml` replaces a complete named value. `--set-path` parses YAML and creates intermediate maps for object-only paths; `--unset-path` removes an object property. `--patch` accepts an RFC 6902 array with concrete JSON Pointer paths, including list edits and `test` guards. Inputs accept inline YAML/JSON, `@file`, or `-`; only one stdin consumer is permitted. Duplicate/non-string keys, tags, anchors, aliases, merge keys, nonfinite numbers and multiple documents fail. `--dry-run` previews without writes. Use `--add-source-json @file` for source mappings. Do not fabricate verification. Meaningful edits remove active `verified` events and update existing `generated.at`; preserve needed historical evidence separately. Body files contain Markdown only, without frontmatter. Section flags take heading and text as separate values. `--replace OLD NEW` replaces exactly one literal body match; `--all` replaces every match and still fails if none exist. `--rename-section OLD NEW` changes a uniquely matched heading without replacing its content. Malformed YAML must be repaired before this command can load it.

### `okf mv` · _mutates_

Move/rename a concept and rewrite every inbound link.

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<old>` | positional | yes | Existing concept id |
| `<new>` | positional | yes | New concept id |
| `<bundle>` | positional | no | Bundle directory (explicit, then $OKF_BUNDLE, nearest okf.toml, or current directory) |
| `--require-ontology-gate` | bool | no | Require the ontology write gate even when bundle policy leaves it disabled (default: `false`) |

Output stream: `change`.

Supported global options: `--bundle-id`, `--catalog`, `--catalog-scope`, `--guidance`, `--json`, `--scope-bundle`.

Moves rewrite inline/reference-style Markdown, self-links, declared simple/nested references, and curated index/log links within the selected bundle. File/text fingerprint sources retain bundle-root semantics; Git sources retain worktree-root semantics. Destinations must be absent. The complete write set is prevalidated and journaled for rollback. Use changeset plan/apply for coordinated moves, replacement, splits, or cleanup. Cross-bundle referrers are not rewritten.

## render

### `okf docs` · _conditionally mutates_

Generate documentation from a bundle.

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<bundle>` | positional | no | Bundle directory (explicit, then $OKF_BUNDLE, nearest okf.toml, or current directory) |
| `--format <value>` | string | no | Output format: md|html|graphml|obsidian|index; pdf is retained but unavailable (default: `md`) |

Output stream: `docs,change`.

Supported global options: `--bundle-id`, `--catalog`, `--guidance`, `--json`.

`--format index` writes indexes throughout the bundle, replacing their bodies; it does not merge curated prose. Use it only when all affected index bodies are generated or replacement is already authorized. Preserve curated indexes and edit only necessary links otherwise. Validate after writes. Other formats emit output rather than updating indexes; the default is `md`.
