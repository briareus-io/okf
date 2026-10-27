---
type: Reference
audience: contributor
title: Implementation modules
description: Find the core and CLI modules by responsibility without mirroring Rust API types.
sources:
- resource: crates/okf-core/src/bundle/settings.rs
  kind: git-path
- resource: crates/okf-core/src/bundle/catalog.rs
  kind: git-path
- resource: crates/okf-core/src/bundle/context.rs
  kind: git-path
- resource: crates/okf-core/src/bundle/mod.rs
  kind: git-path
- resource: crates/okf-core/src/bundle/resolve.rs
  kind: git-path
- resource: crates/okf-core/src/bundle/config.rs
  kind: git-path
- resource: crates/okf-core/src/bundle/loader.rs
  kind: git-path
- resource: crates/okf-core/src/check/lint/mod.rs
  kind: git-path
- resource: crates/okf-core/src/check/lint/rules/ontology_violation.rs
  kind: git-path
- resource: crates/okf-core/src/check/lint/rules/index_coverage.rs
  kind: git-path
- resource: crates/okf-core/src/check/validate.rs
  kind: git-path
- resource: crates/okf-core/src/check/stale.rs
  kind: git-path
- resource: crates/okf-core/src/fingerprint/mod.rs
  kind: git-path
- resource: crates/okf-core/src/graph/build.rs
  kind: git-path
- resource: crates/okf-core/src/graph/catalog.rs
  kind: git-path
- resource: crates/okf-core/src/graph/relationships.rs
  kind: git-path
- resource: crates/okf-core/src/graph/mod.rs
  kind: git-path
- resource: crates/okf-core/src/graph/render.rs
  kind: git-path
- resource: crates/okf-core/src/model/mod.rs
  kind: git-path
- resource: crates/okf-core/src/mutate/mod.rs
  kind: git-path
- resource: crates/okf-core/src/mutate/structured.rs
  kind: git-path
- resource: crates/okf-core/src/mutate/transaction.rs
  kind: git-path
- resource: crates/okf-core/src/mutate/gate.rs
  kind: git-path
- resource: crates/okf-core/src/mutate/refresh.rs
  kind: git-path
- resource: crates/okf-cli/src/main.rs
  kind: git-path
- resource: crates/okf-cli/src/structured.rs
  kind: git-path
- resource: crates/okf-cli/src/cli.rs
  kind: git-path
- resource: crates/okf-cli/src/output.rs
  kind: git-path
- resource: crates/okf-cli/src/exit.rs
  kind: git-path
- resource: crates/okf-core/src/lib.rs
  kind: git-path
- resource: crates/okf-core/src/ontology/field_types.rs
  kind: git-path
- resource: crates/okf-core/src/ontology/schema.rs
  kind: git-path
- resource: crates/okf-core/src/ontology/load.rs
  kind: git-path
- resource: crates/okf-core/src/ontology/mod.rs
  kind: git-path
- resource: crates/okf-core/src/ontology/edit.rs
  kind: git-path
- resource: crates/okf-core/src/output/mod.rs
  kind: git-path
- resource: crates/okf-core/src/parse/mod.rs
  kind: git-path
- resource: crates/okf-core/src/ports/git.rs
  kind: git-path
- resource: crates/okf-core/src/ports/mod.rs
  kind: git-path
- resource: crates/okf-core/src/query/metadata.rs
  kind: git-path
- resource: crates/okf-core/src/query/selector.rs
  kind: git-path
- resource: crates/okf-core/src/query/mod.rs
  kind: git-path
- resource: crates/okf-core/src/query/browse.rs
  kind: git-path
- resource: crates/okf-core/src/render/index.rs
  kind: git-path
---
# Implementation modules

Use this map to find implementation boundaries. Rust types and signatures belong in `cargo doc --workspace --no-deps`.

## bundle module

Loads concepts while honoring reserved files, and keeps bundle selection separate from graph
examination scope. Catalog identity qualifies ordinary concept IDs without changing their paths.
Paths and namespaced IDs are distinct selectors; `OKF_BUNDLE` retains path semantics. Typed
`default_bundle` replaces the deprecated path-only `bundle` setting.

The optional catalog registers local directories, canonicalizes available roots, rejects equal or
nested registrations, and reports unavailable material. Config paths are relative to `okf.toml`;
locations are relative to the catalog. Per-bundle settings supply ontology, lint, query, view, and
facet interpretation. Registration does not imply remote acquisition or catalog-wide traversal.

See the [multi-bundle decision](../decisions/multi-bundle-scope.md). Location:
`crates/okf-core/src/bundle`.

File parse errors retain the document path. Full bundle loads report every malformed YAML concept in deterministic file order, while direct reads report only the requested file. No partial bundle is returned after parse failures.

## check module

Diagnostics. Validate enforces all three conformance rules including index/log structure and root version syntax; lint checks optional families advisorily; stale compares corrected instants and fingerprint extensions; doctor performs tolerant upgrade preflight and safe repair. The implementation composes [bundle loading](modules.md), the [model](modules.md), [fingerprinting](modules.md), the [ontology](modules.md), the [graph](modules.md), [parsing](modules.md), effect [ports](modules.md), and artifact [queries](modules.md).

Location: `crates/okf-core/src/check`.

Recursive metadata lint adds concrete paths and codes while retaining advisory ontology rules.
A depth or finding budget stop reports an incomplete check. Index coverage checks immediate
concept and concept-containing-directory links, honors configured exclusions, and remains
read-only. Neither local navigation policy nor optional ontology constraints tighten portable
OKF conformance.

The 0.5.0 source-health contract keeps drift, absent baseline, established
missing resource, unavailable computation, unsupported fingerprint kind, and
expiry distinct. A kindless source remains provenance without a fingerprint
health assertion. Diagnostic display filters do not change examination or
failure policy; configured lint severities are independent of an optional
write gate.

## fingerprint module

Source fingerprinting, dispatched on `SourceKind`: git-commit and git-path through effect [ports](modules.md), line-range and markdown-heading through Markdown [parsing](modules.md), and optional URL fingerprints behind a feature flag. Canonicalization normalizes line endings and whitespace so `Fingerprint` values stay stable.

Location: `crates/okf-core/src/fingerprint`.

## graph module

Builds forward and reverse adjacency from [bundle](modules.md) concepts, body links, standard
`sources[].resource` lineage, and explicitly declared [ontology](modules.md) reference selectors.
Path-looking opaque metadata does not become an edge merely because it resembles a link.
Nested selectors retain occurrence paths; semantic kind and attribute selectors pair with the
same list record. `links --details` exposes authored outgoing semantic occurrences without
collapsing repeated targets; `backlinks --details` presents their incoming view with inverse
labels. Both preserve record attributes. Inverse backlinks display incoming assertions rather
than target acceptance, and inverse labels do not synthesize additional outgoing edges.

Cross-bundle results retain qualified identity, original reference, resolution evidence and
examined scope. Broken and out-of-scope targets remain visible; a catalog is not a global backlink
inventory. Requested source snapshots and current candidates remain distinct. Resolution does
not create verification events or accept changed source fingerprints.

Powers [links](../../commands/links.md), [backlinks](../../commands/backlinks.md),
[affected](../../commands/affected.md), and bounded [graph](../../commands/graph.md) rendering.
Automatic cross-bundle moves and rewriting declared nested-reference occurrences remain deferred.

Location: `crates/okf-core/src/graph`.

## model module

In-memory domain types with no I/O: `Concept`, `ConceptId`, the order-preserving `Frontmatter`, trust types (`Actor`, `Verified`, and `TrustTier`), source types (`Source`, `SourceKind`, and `Fingerprint`), and `Link` parsing. Link extraction consults the tool-local [ontology](modules.md) for declared reference fields. The linchpin of the crate, because everything round-trips through Frontmatter.

Location: `crates/okf-core/src/model`.

## mutate module

Atomic, self-validating concept writes preserve unknown YAML meaning and body text while applying init, add, edit, mv, rm, verify, and refresh. They compose [bundle](modules.md), [check](modules.md), [fingerprint](modules.md), [model](modules.md), [ontology](modules.md), [graph](modules.md), [parse](modules.md), and effect [port](modules.md) services. YAML comments and scalar presentation may normalize.

Location: `crates/okf-core/src/mutate`.

Structured frontmatter mutations operate on YAML values before persistence. Object paths create
missing maps and reject list traversal; RFC 6902 patches use concrete JSON Pointer addresses.
All patch operations and concept conformance checks complete before writing. Dry-run applies the
same preflight without publishing authored content or lifecycle changes; coordination locks
and temporary staging may be created. Meaningful writes preserve
existing verification invalidation and generated timestamp semantics.

In 0.5.0, CLI direct and changeset writers prepare a final state and publish
through the journaled single-bundle protocol. An enabled ontology gate
compares new or worsened findings independently from advisory lint display.
Baseline acquisition can produce a guarded token without an operation plan;
it may create coordination state. Single refresh computes all required
fingerprints before publication and leaves the concept unchanged if a
required source fails. Recovery restores the recorded transaction state
without treating an interrupted output stream as proof of rollback.
The 0.5 write gate blocks with `write-gate-incomplete` when an out-of-bundle
typed reference target cannot be checked. Explicit dependent-bundle
examination for a shared ontology does not imply ordinary cross-bundle
reference traversal.

## okf-cli crate

The thin binary named okf. Responsible only for clap argument parsing, rendering [okf-core](modules.md) results as human text or NDJSON, and mapping results and errors to process exit codes. It holds no domain logic; every command is a thin function that parses args, calls the core, and hands the result to output.

## Command surface

- Meta: [schema](../../commands/schema.md), [version](../../commands/version.md), [catalog](../../commands/catalog.md)
- Query: [list](../../commands/list.md), [search](../../commands/search.md), [show](../../commands/show.md), [browse](../../commands/browse.md), [links](../../commands/links.md), [backlinks](../../commands/backlinks.md), [graph](../../commands/graph.md), [resolve](../../commands/resolve.md), [artifact list](../../commands/artifact-list.md), [artifact resolve](../../commands/artifact-resolve.md), [artifact show](../../commands/artifact-show.md), [computation check](../../commands/computation-check.md)
- Check: [scan](../../commands/scan.md), [source-scan](../../commands/source-scan.md), [validate](../../commands/validate.md), [lint](../../commands/lint.md), [stale](../../commands/stale.md), [affected](../../commands/affected.md), [diff](../../commands/diff.md), [stats](../../commands/stats.md), [doctor](../../commands/doctor.md)
- Mutate: [artifact put](../../commands/artifact-put.md), [init](../../commands/init.md), [add](../../commands/add.md), [edit](../../commands/edit.md), [mv](../../commands/mv.md), [rm](../../commands/rm.md), [verify](../../commands/verify.md), [refresh](../../commands/refresh.md), [ontology add](../../commands/ontology-add.md), [ontology update](../../commands/ontology-update.md), [ontology remove](../../commands/ontology-remove.md), [ontology apply](../../commands/ontology-apply.md), [field-type add](../../commands/ontology-field-type-add.md), [field-type update](../../commands/ontology-field-type-update.md), [field-type remove](../../commands/ontology-field-type-remove.md)
- Ontology queries: [ontology list](../../commands/ontology-list.md), [ontology show](../../commands/ontology-show.md)
- Render: [docs](../../commands/docs.md)
- Guarded baseline: [changeset digest](../../commands/changeset-digest.md)

Location: `crates/okf-cli`.

Global identity, scope and revision flags remain distinct. Schema-derived references document
the supported command combinations, computed record contracts, and optional interpretation
settings. Query projections and semantic relationship payloads preserve legacy authored metadata.

Structured input loading accepts one YAML/JSON document, inline, from a file or from stdin.
It rejects ambiguous keys, unsupported YAML constructs and multiple stdin consumers before
calling core mutation logic. This is input interpretation; object-path and patch semantics remain
in the core mutation module.

CLI 0.5 JSON operations declare a stream version and frame payload records
with one start record and, when normal completion is possible, one terminal
record. The terminal record carries outcome, publication and applicable
completeness dimensions; the process exit remains authoritative. Argument
parsing may fail before a stream begins. The CLI also resolves explicit
changed-resource input before deciding whether stdin is requested, and gives
filesystem, bundle and repository path namespaces explicit context.

## okf-core crate

The deterministic core library. Its public module map comprises [bundle](modules.md), [check](modules.md), [fingerprint](modules.md), [graph](modules.md), [model](modules.md), [mutate](modules.md), [ontology](modules.md), [output](modules.md), [parse](modules.md), [ports](modules.md), [query](modules.md), and [render](modules.md). It takes no argv and writes no stdout; it exposes typed results that the CLI renders. This is the brain half of the hands-vs-brain split.

Location: `crates/okf-core`.

## ontology module

Optional tool-local definitions: exact concept types, reusable nested field types, recursive
constraints, declared reference selectors and semantic relationship rules. Composition resolves
named references at every nested level; inheritance retains most-derived constraints and explicit
fields, values or item settings replace inherited counterparts. Invalid schemas and cycles are
configuration errors. Unknown extensions remain preserved and unenforced constraints remain visible.

Loads ontology YAML and supplies effective definition inspection. Scalar and structured add/update/remove writes are available. YAML declaration inputs replace
named entries, reusable field-type commands replace full definitions, and bulk apply merges
changes and explicit removals before validating and writing the combined result once. [Lint](../../commands/lint.md) checks
declarations advisorily, without changing OKF conformance or proving relationship truth.

Location: `crates/okf-core/src/ontology`.

Concept-type ancestry supports relationship target restrictions. A dynamic kind may narrow its
reference rule's allowed target union; this local ancestry does not grant standard computation
semantics to arbitrary types. Each bundle owns those definitions.

Ontology parse and schema errors identify the effective ontology file, including configured sidecars and historical interpretation files.

Structured declaration edits preserve unrelated ontology extensions. Concept updates merge by
named declaration; reusable field-type updates replace complete definitions. Bulk change documents
contain `field_types`, `concepts`, and explicit `remove` maps rather than a complete sidecar.
Conflicting operations fail, and dry-run previews the same resulting ontology without writing.

## output module

NDJSON record types and their serializer. `Concept records` preserve frontmatter values and add collision-safe identity, trust, lifecycle, generation, and verification views; `Finding`, `Change`, and `Affected` records back other machine outputs.

Location: `crates/okf-core/src/output`.

## parse module

Semantic markdown and YAML handling: split frontmatter from body, build the heading tree, extract sections, retain unknown keys/values and top-level order, and serialize back. Body text is preserved; YAML comments and presentation may normalize.

Location: `crates/okf-core/src/parse`.

`parse_concept_file` adds the document path to parser errors without changing their exit class; text-only `parse_concept` remains available for in-memory parsing.

## ports module

Effect boundaries expressed as traits: fs, git, clock, and net. Real implementations in production, fakes in tests, so fingerprint/stale/diff stay hermetically testable. The git port is a thin wrapper over the git CLI, not a git library.

Location: `crates/okf-core/src/ports`.

Local Git reads disable implicit partial-clone object fetching and terminal prompts. Catalog
snapshot inspection does not acquire missing material or authorize execution.

## query module

Read-only lookups: browse reads or synthesizes structural directory indexes through [render](modules.md); search/list/show retrieve concepts from a [bundle](modules.md); artifact commands safely resolve path-valued resources; computation check inspects contracts without execution; and stats/diff compose [check](modules.md), [graph](modules.md), [model](modules.md), [ontology](modules.md), [parse](modules.md), and effect [port](modules.md) services.

Location: `crates/okf-core/src/query`.

Structured queries share explicit YAML selectors with graph rules: nested properties, `[]` list
traversal, and bracket-quoted literal keys. The walker retains occurrence paths. Typed conditions,
views, projections, JSON facets, pagination and bounded related expansion compute reader output
without changing authored frontmatter. Results retain scan completeness and examination scope;
facets concern examined matches before the displayed page.

Parse failures identify the actual files examined by the query. Bounded scans collect failures within the scan budget; historical reads include revision context and distinguish malformed content from unavailable Git material.

## render module

Derived output artifacts: progressive-disclosure index.md generation and the optional html/md/pdf/graphml/obsidian document renderers behind feature flags. Rendering composes [bundle](modules.md), [check](modules.md), [model](modules.md), [graph](modules.md), [ontology](modules.md), and [parse](modules.md) services.

Location: `crates/okf-core/src/render`.
