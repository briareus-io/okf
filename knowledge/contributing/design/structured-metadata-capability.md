---
type: Reference
audience: contributor
title: Structured metadata interpretation
description: Selector, nested validation, occurrence pairing, index coverage, and query aggregation invariants.
---
# Structured metadata interpretation

The CLI interprets declared bundle metadata at read and lint time. This document records the
invariants shared by the selector walker, ontology validator, graph builder, and query pipeline.
For the ontology and documents used by the capability scenarios, see
[Query structured records and their relationships](../../guides/query-structured-records.md).
[Structured authoring](../decisions/structured-authoring.md) covers mutation semantics.

## Configuration and conformance

OKF concept IDs, standard source resources and exact Attested Computation semantics remain
independent of the ontology. Unknown fields survive parsing and mutation. Reading, linting and
querying leave frontmatter and verification events intact. Ontology and index findings belong to
local policy. A configuration error prevents interpretation under that configuration.

The bundle manifest owns the ontology path, views, query budget, facet settings, and lint policy.
Manifest paths resolve from the bundle root. Catalog registration owns identity and location.
An unregistered bundle retains its own settings. An explicit unreadable ontology is an error.
Otherwise the loader can discover `ontology.yaml` at the root. Selection and migration details
are in [Bundle manifest and guidance](bundle-manifest.md).

Each examined bundle supplies its own type definitions. Historical reads resolve an in-repository
ontology and configuration at the examined revision. External sidecars use available local bytes
and report the effective path and digest. Return the interpretation with the results so callers
can distinguish immutable document bytes from changing external definitions.

## Shared selector grammar

The selector parser walks `yaml_serde::Value` directly. Filter operands use the existing JSON
parser. The same selector syntax applies to declared references, relationship kind and attribute
selectors, display columns, projections, facets, and selector-aware filters:

| Selector | Meaning |
| --- | --- |
| `title` | Top-level property |
| `deadline.within` | Nested object property |
| `tags[]` | Each item of a list |
| `norms[].deadline.from` | Deadline anchor within each norm record |
| `["policy.status"]` | Literal top-level key containing a dot |
| `norms[]["action.name"]` | Literal dotted key within each norm record |

There is no leading JSONPath `$` and no `[*]` spelling. List traversal requires explicit
`[]`; `norms.deadline.from` does not implicitly traverse `norms`. Selecting a list itself,
such as `tags` for a display cell or projection, remains distinct from selecting its items.
Bracket-quoted keys use JSON string escaping. Computed display/projection names such as
`$id` and `$bundle` remain separate from authored-property selectors.

The subset excludes recursive descent, indexing/slices, selector unions, embedded predicates,
functions, pipes, and executable expressions. Malformed or unsupported selector syntax is
diagnosed rather than delegated to another expression engine. The walker retains concrete
occurrence paths and container presence so same-record relationship pairing and missing,
null, and empty-list states are not lost by flattening selected values.
Existing literal-key `--field` behavior and reference rules without a selector remain
compatible.

## Reusable types and nested linting

In the worked procedure, `Norm` composes `Modality` and `RelativeDeadline`.
`ClosedRelation` inherits `Relation`.
Inherited constraints use most-derived precedence. Explicit `fields`, `values`, or `item`
settings replace inherited counterparts rather than silently merging field maps. Cycles
across inheritance and nested references are configuration errors. Referencing a field type
does not itself create a concept reference or relationship edge.

Replacement can remove inherited properties: a derived `fields: {note: {type: string}}`
replaces all inherited fields, including mandatory ones. In a closed derived object, only
the replacement fields are allowed. Authors must repeat inherited fields they wish to keep.
`extends` does not promise substitutability. Changing an inherited base type is a
configuration error. Validate the complete effective definition after inheritance resolution;
for example, inherited `min: 5` combined with derived `max: 3` is invalid.

Named types support composition and inheritance at every nested level, including inline
objects whose children reference other named types and lists whose items reference named
types. Updating a shared definition changes the effective validation of all its users within
that ontology without rewriting their values. `ontology show` exposes resolved nested
structure, inherited constraints, and the named definitions they came from.

Recursive lint validates primitives without coercion, exact enum values, object children, list
items, patterns, bounds, and uniqueness:

| Declaration | Supported check |
| --- | --- |
| `string`, `text` | YAML string, without number/boolean coercion |
| `int` | Integral YAML number representable as a signed 64-bit integer |
| `bool` | YAML boolean |
| `date` | Valid Gregorian `YYYY-MM-DD` string |
| `datetime` | RFC 3339 string with explicit timezone |
| `uri` | Syntactically valid absolute URI; no network or existence check |
| `enum` | Exact string membership in resolved `values` |
| `object` | Mapping with string keys and recursively checked children |
| `list` | Sequence with recursively checked `item` values |
| `pattern` | Rust regex search semantics; anchors express full-string matching |
| `min`, `max` | Inclusive integer bounds, string Unicode-scalar length, or list length |
| `unique_items: true` | Structural YAML equality, ignoring mapping key order |
| `additional_properties: false` | Undeclared keys reported in this custom object |

Missing optional fields are skipped. A present null fails its declared type. For
`required: true`, absent/null values, whitespace-only strings, and empty lists fail;
empty objects count as present and their child constraints apply. Missing optional parents
do not make their children mandatory. Container type failures stop child checking at that
occurrence, avoiding misleading missing-child findings; independent failures remain visible
and duplicate required/null findings are suppressed. Root metadata remains open.

Malformed recognized constraints, incompatible constraint/type combinations, `min > max`,
missing item types, empty enum vocabularies, unknown named types, and cycles produce
configuration errors. Patterns are checked when loading the ontology. Unknown extension
constraints remain preserved with an unenforced notice.

Findings retain the existing `ontology-violation` rule alongside detailed codes such as
`metadata-required`, `metadata-type`, `metadata-enum`, `metadata-pattern`, `metadata-range`,
`metadata-duplicate`, and `metadata-unknown-property`. They identify the bundle/concept,
concrete occurrence path, expected constraint, and bounded actual value or type. Without an
ontology, ontology lint is skipped. Depth or finding-budget exhaustion produces an
incomplete-check diagnostic; the depth bound is 64, with a configurable finding
budget (1,000 findings per concept by default). Lint reports incomplete checks when it stops early.

## Reference extraction and edge identity

Only declared targets become custom concept references. `evidence-file` stays an ordinary
string. Leading `/` is source-bundle-root relative; bare and `../` paths resolve relative
to the containing document. An ordinary path crossing into another registered bundle uses
the shared multi-bundle resolver. Above-root traversal is preserved and never collapsed into
an unrelated local ID.
Missing, unavailable, wrong-type, and out-of-scope targets have distinct diagnostics.

Nested string references can contain ordinary paths or supported logical references. Snapshot
expectations come from provenance source entries with an explicit `bundle_ref.snapshot`. A body
link or metadata reference to the same target does not inherit the source entry’s expectation.

Target type restrictions are qualified by the bundle declaring the type. An unqualified
`target: Policy` denotes the source bundle's declaration; another bundle's `Policy` is not
equivalent merely because its name matches. Cross-bundle restrictions require an explicit
qualified declaration. A resolved target satisfies that restriction
through the exact declaration or its declared concept-type inheritance; field-type
inheritance alone does not establish concept subtyping. Use `<bundle-id>:<type>` for qualified
target types.

Reference rules support an optional `selector`. Without it, the rule key retains its
literal top-level-field meaning, including dotted keys, and accepts a scalar string or list
of strings. Explicit selectors support nested object keys, `[]` list traversal, and bracket
quoting such as `["a.b"][]`; selected target leaves are nonempty strings. Existing
extensionless concept references remain supported. Cardinality counts authored target
occurrences, including unresolved targets, rather than deduplicated neighbors. Missing
branches contribute no occurrences; nested mandatory values are checked by field declarations.

Missing target warnings for declared references remain configurable for CI; body-link rules
retain their existing severity. One occurrence does not produce duplicate broken-link and
nested-reference findings. Out-of-scope resolution remains distinct from a proven missing file, and
candidate content does not satisfy requested-version target-type checks.

Relationship mappings pair kind, target, and attributes within the same list record.
List occurrence anchors retain that pairing instead of independently flattening values.
Fixed-kind relationships, such as a declared `depends-on` list, are also supported. Distinct
occurrences and kinds to one target remain distinct semantic edges.

For example, alongside an appropriate top-level reference rule:

```yaml
dependencies:
  reference: depends-on
  kind: depends-on
  inverse: dependency-of
```

Mappings support either a fixed `kind` or a `kind_selector` with a `kinds` map.
Ambiguous/mismatched list anchors and contradictory mappings produce configuration errors.
Dynamic kind vocabularies agree with declared enums; per-kind target restrictions can narrow
the reference rule's allowed target types. Unknown dynamic kinds produce a finding and an
unresolved semantic occurrence while the underlying declared target remains visible. Each nested relationship
record has one scalar target. Top-level fixed-kind references support scalar and list targets.

Each edge retains source identity/version, rule, metadata path, raw reference, kind, inverse,
attributes, and resolution evidence. Semantic self-edges remain available even where legacy
adjacency excludes self-links; concept-neighbor views can continue deduplicating targets.

Detailed local queries retain distinct occurrences and inverse labels in versioned relationship
records. Catalog results carry the semantic occurrence on the qualified edge. Ordinary links
carry no invented kind. An inverse label describes the incoming view of the authored assertion.

Cross-bundle backlinks report examined bundles and versions. Unexamined consumers remain unknown.
Matching kind labels from different bundle ontologies do not establish equivalent vocabularies.
Machine output identifies the declaring bundle/rule so cross-bundle queries can select the intended
declaration. Affected analysis includes declared nested targets. Relationship-rule selectors,
including expansion options, are likewise qualified by declaring bundle. Short forms select rules in
the primary source bundle; multi-bundle queries require explicit qualification where declarations
differ.

Supported single-bundle moves rewrite declared structured-reference occurrences. Mutation must
preflight exact occurrences, preserve surrounding values and attributes, and reject unsupported
rewrites before publication. Scoped integrity checks can require explicit reconciliation instead of silently leaving dangling links.

## Index coverage

The check compares each eligible directory's immediate children with links in its actual
`index.md`. Coverage includes concepts, their ancestor directories, and the root. It excludes
reserved `index.md`/`log.md`, opaque artifacts, `.git`, artifact-only directories, and configured
index exclusions. Hidden and Git-ignored concepts follow the physical-tree walker. Exclusions
affect this check only, not bundle boundaries, search, graphs, or validation.

Index exclusions use bundle-relative POSIX globs. Excluding a directory excludes its subtree from
coverage; eligible ancestors are recomputed afterward so excluded-only subtrees do not produce
parent omissions. Symlinks are not followed. Coverage uses the generator's concept and ancestor
inventory rather than comparing generated and authored text byte for byte.

```text
policies/
  index.md             links only to access-control.md
  access-control.md
  retention.md         omitted
  exceptions/
    temporary.md       makes exceptions/ an eligible directory
```

```sh
okf lint knowledge/product --json
```

Illustrative coverage findings, among other applicable lint findings:

```json
{"kind":"finding","code":"index-missing-entry","severity":"warn","directory":"policies","missing_child":"retention.md"}
{"kind":"finding","code":"index-missing-entry","severity":"warn","directory":"policies","missing_child":"exceptions/"}
```

A descriptive Markdown link counts; a filename mentioned in prose does not. A directory
link or link to its `index.md` covers that child directory; a link only to a deeper concept
does not. Inline and reference-style links count, including `.md`, accepted extensionless
spellings, fragments, and consistently decoded URL-escaped paths. Images, code blocks,
frontmatter strings, remote destinations, and links to another bundle do not establish
immediate-child coverage. Duplicate entries are not completeness failures; stale/broken
entries remain link-check concerns. Findings sort by bundle, directory, child, and code.

A missing eligible index produces one `index-missing` finding instead of one per child.
Unreadable indexes produce I/O diagnostics instead of successful empty coverage.
Synthesized browsing does not satisfy an on-disk index check. Lint does not regenerate
indexes, and existing generation can replace an authored index body.

## Query pipeline

### Matching and projection

Apply document conditions before ranking. Separate filters combine with AND. Uppercase OR
combines alternatives within one typed filter. Equality on nested selectors is existential.
Separate conditions can match different list items. Relationship predicates use occurrence
paths to keep edge attributes correlated. For an authored list, `in` accepts any candidate
value and `not in` excludes a match anywhere in the selected values. A selected empty list
passes negative membership. Missing or null fields fail both operations. Equality and membership
use scalar operands of compatible types. Presence and null tests inspect the occurrence itself.

Display and projection reuse the selector grammar. In human output a missing cell is marked
and truncation stays visible. A named view can supply human columns and expansion defaults.
Machine projection requires an explicit projection or an opted-in extended-output view. A
projection retains qualified identity, missing selectors, and authored nulls. Computed names
such as `$id` and `$version` have their own namespace. Separate identity records on ordinary
multi-bundle JSON reads preserve authored fields with colliding names.

### Aggregation and paging

Facets count matching primary documents **before** pagination. A repeated value counts once per
document. Missing branches, nulls and empty lists contribute no bucket. Empty strings are
authored scalar values, distinct scalar types remain distinct, and object-valued results raise a
diagnostic. Expanded neighbors contribute no facet counts. The default candidates are `type`
and `tags[]`. The default exclusion threshold is more than 100 distinct nonmissing values.
Explicit facet selection still respects it. Per-field high-cardinality inclusion remains subject
to the 1,000-value output cap. Emit the reason and threshold for exclusions, retain configured
field order, and rank buckets by descending document count with typed value order breaking ties.

The primary bundle supplies a single scan budget across scope. The default is 1,000 eligible
documents, with configuration and explicit command overrides. `--full-scan` removes that
document-count bound. Scope, availability, facet output bounds and expansion bounds continue to
apply. Exactly hitting the budget is complete when no eligible documents remain. Exhaustion
before that point marks observed matches partial, withholds exact totals and continuation, and
warns even when no matches have been found. A complete calculation can still have truncated
delivery.

Use qualified identity as the tie-breaker after relevance scoring, or as the key for ID sorting.
Apply offset and limit after all filters and sorting. With more matches, `next_offset` is the
offset plus the returned count. An offset past the last result yields an empty page and no
continuation. Every page request is an independent read. Reproducible paging requires the
same query, scope, ordering, document versions, and interpretation settings.

### Bounded relationship expansion

Expansion follows selected outgoing rules for primary hits on the returned page. Targets can lie
outside the matching population and consume no primary page slots. Retain each edge occurrence
and deduplicate target payloads by bundle, concept, and version. Account for identity, values
and status entries in the byte bound. Preserve raw references to missing, unavailable and
out-of-scope targets. The expansion summary reports emitted counts, truncation and reasons
independently of primary scan completeness. Ordinary reads do not acquire remote material.

Defaults are 10 edges per primary hit, 100 distinct targets and 262,144 serialized bytes per
query. The corresponding CLI maxima are 100 edges, 1,000 targets and 1,048,576 bytes.
`--no-expand` disables configured human expansion. The [search reference](../../commands/search.md)
owns the supported flags and output record names. See
[Performance and bounded reads](../../guides/bounded-reads.md) for user workflows.
