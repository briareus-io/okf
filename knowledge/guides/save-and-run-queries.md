---
type: Guide
audience: user
uses:
- /commands/init
- /commands/bundle-context
- /commands/bundle-validate
- /commands/query
- /commands/show
explains:
- /concepts/ontology-and-relationships
title: Save and run a query
description: >-
  Save a recurring relationship query, choose its starting concepts, and read fields or
  sections from its matches.
---
# Save and run a query

Suppose each feature review starts with the same question: which owned notes describe open
defects blocking this feature? A saved query names the sequence of reads. Each reviewer supplies
a feature ID and chooses which properties or prose to read. The manifest keeps the direction and
criteria for each hop together.

## Model the example

Use a directory named `product` for this example. Each file below has a path relative to its
parent directory. Use CLI 0.6.12 or newer for every reader of this example. CLI 0.6.11 supports
saved queries but rejects the workflow field `uses_queries`.
Run the commands from the parent directory.

```sh
okf init product
```

The command creates `product/index.md` and a starter ontology. Replace the starter ontology
with this one. When adapting the example to an existing bundle, merge the definitions into
its existing ontology and manifest.

**File: `product/ontology.yaml`**

```yaml
okf_ontology: '0.1'
concepts:
  Feature: {}
  Note:
    fields:
      owner: {type: string}
  Defect:
    fields:
      state: {type: string}
    references:
      blocks: {target: Feature, cardinality: '0..1'}
      triage: {target: Note, cardinality: '0..1'}
    relationships:
      blocking: {reference: blocks, kind: blocks, inverse: is-blocked-by}
      triage-note: {reference: triage, kind: has-triage-note, inverse: documents-defect}
```

Add these three documents. The defect **blocks** a feature and **has a triage note**. The note
links back to the defect in Markdown. An ordinary link participates in a hop that accepts Note
endpoints. A `kind` predicate requires an ontology relationship carrying that kind.

**File: `product/features/export.md`**

```markdown
---
type: Feature
title: Export
description: Export feature
---
Export records.
```

**File: `product/defects/csv.md`**

```markdown
---
type: Defect
title: CSV failure
description: Failure on an empty CSV row
state: open
blocks: /features/export.md
triage: /notes/triage.md
---
Export stops on an empty row.
```

**File: `product/notes/triage.md`**

```markdown
---
type: Note
title: Export triage
description: Steps for unblocking export
tags: [urgent]
owner: Marin
---
# Operations

See the [CSV failure](/defects/csv.md).

## Recovery
Retry the export after clearing the queue.

## Escalation
Contact the owner.
```

## Give each hop a direction and a filter

Save the following manifest. In `notes-for-open-blockers`, the first backlink hop finds an open
defect that blocks the feature. The second finds owned notes linking to that defect.
`triage-notes` changes direction at the defect and follows its declared triage relationship
outward.

**File: `product/okf-bundle.yaml`**

```yaml
manifest_version: 1
workflows:
  review-feature:
    description: Review the triage note before editing a feature.
    instructions: Read the triage note and check the proposed change with its owner.
    uses_queries: [triage-notes]
  copy-edit:
    description: Copy edit a feature.
    instructions: Preserve the meaning of its requirements.
settings:
  query_presets:
    features:
      operations: [list]
      query: {type: Feature}
  saved_queries:
    notes-for-open-blockers:
      from: concept
      steps:
        - backlinks:
            kind: [blocks]
            related_type: [Defect]
            related_filter: ['state="open"']
        - backlinks:
            related_type: [Note]
            related_filter: ['owner exists']
      select: [title, owner]
    triage-notes:
      from: concept
      steps:
        - backlinks: {kind: [blocks], related_type: [Defect]}
        - links: {kind: [has-triage-note], related_type: [Note]}
      select: [title, owner]
    all-open-blockers:
      from: {list: {preset: features}}
      steps:
        - backlinks: {kind: [blocks], related_type: [Defect], related_filter: ['state="open"']}
    triage-for-defect:
      from: concept
      steps:
        - links: {kind: [has-triage-note], related_type: [Note]}
      select: [owner]
```

```sh
okf bundle validate product
```

The command prints `manifest: valid`. If validation reports an unknown field named
`uses_queries`, upgrade that reader before using the manifest. The same parser handles
both text and JSON output. Run the two-backlink query with the feature ID.

```sh
okf query notes-for-open-blockers features/export --bundle product
```

The projection for `/notes/triage` ends with the title and owner, followed by the summary.

```text
Export triage	Marin
matched 1 concepts; matching complete: true; section errors: 0
```

Run the mixed-direction query on the same feature.

```sh
okf query triage-notes features/export --bundle product
```

You get the same note and owner. Closing the defect would remove the note from the first query's
results. The second would still reach it through the declared triage relationship.

Use `kind`, `inverse_kind`, `rule`, or `edge_filter` to narrow relationship occurrences. Use
`related_type` and `related_filter` for the reached concept. A `links` hop reaches the target,
and a `backlinks` hop reaches the author. To follow ordinary Markdown links from a defect to
notes, use `links: {related_type: [Note]}` and link to the note in the defect's body.
`links: {}` and `backlinks: {}` follow every direct link in their respective directions.
See [relationship filters](../commands/links.md).

Each stage passes distinct endpoints to the next. Two matching routes to one note produce one
result for its bundle, version, and concept ID. A later hop may revisit a concept. The length of
the declared sequence bounds the read. To see individual occurrences, use `links --details` or
`backlinks --details`. For a drawing of the neighborhood, use [graph](../commands/graph.md).

## Give a workflow its saved queries

Suppose a reviewer needs the triage note before editing a feature, while a copy editor only
needs the feature text. The manifest above declares `uses_queries: [triage-notes]` for the reviewer. The copy edit omits
the field. Compare their context before starting the edit.

```sh
okf bundle context product --workflow copy-edit --operation edit --format agent
```

```sh
okf bundle context product --workflow review-feature --operation edit --format agent
```

The copy edit shows zero saved queries. The feature review includes one line beginning
`[saved query triage-notes]`, with its starting point, hops, and selected fields. Listing
both workflows together shows that definition once. A typo in `uses_queries` fails validation
with the workflow and missing query name. Use a CLI that recognizes this manifest field on
each reader before sharing the change.

To run the lookup during the review, include the query operation when loading guidance, then
invoke the query with the feature ID.

```sh
okf bundle context product --workflow review-feature --operation edit --operation query --format agent
```

```sh
okf query triage-notes features/export --bundle product
```

The explicit query operation exposes all four definitions. Adding `--token` to that context
request produces a receipt covering both edit and query. Declaring a query in a workflow
controls discovery and leaves execution to the caller.

## Start from a group of concepts

The manifest’s `features` preset selects Feature concepts. The `all-open-blockers` query starts
from that set and follows incoming `blocks` relationships to open defects. Run it without a
starting ID to find `/defects/csv`, the only open blocker in this bundle.

```sh
okf query all-open-blockers --bundle product
```

Use `from: {search: {preset: name}}` for a preset that supports `search`. A saved query can omit
`steps` to read its starting concepts directly. An existing `query_presets` entry supplies
filters to the named operation. The sequence of hops belongs in `saved_queries`.

## Select fields or prose from the final matches

```sh
okf query notes-for-open-blockers features/export --bundle product --tag urgent --project owner
```

The urgent note's projection has a single value, `Marin`, for `/notes/triage`.

```sh
okf query notes-for-open-blockers features/export --bundle product --outline
```

The heading lines show the two sibling subsections.

```text
9: # Operations
13: ## Recovery
16: ## Escalation
```

```sh
okf query notes-for-open-blockers features/export --bundle product --section '["Operations","Recovery"]'
```

The output identifies `/notes/triage` and includes this excerpt.

```text
## Recovery
Retry the export after clearing the queue.
```

To read the same excerpt from a known document, use:

```sh
okf show notes/triage product --section '["Operations","Recovery"]'
```

This prints the Recovery heading and the retry instruction. Runtime `--type`,
`--tag`, `--text`, `--field`, and `--facet-filter` narrow final concepts. To change the conditions
on an intermediate defect, edit that hop in the manifest. Text matches use search ordering.
Otherwise results follow concept identity order. `--offset` and `--limit` apply to the final
distinct matches.

Without an explicit output option, `select` supplies the default metadata fields. Omitting
`select` gives the ordinary concept listing. Repeat `--project` to replace the entire default
selection, or choose one prose mode. `--body` returns the full body and `--outline` shows the
heading hierarchy. Both `query` and `show` accept a full heading path with `--section`, a JSON
array from outermost heading to innermost. Here the section includes the Recovery heading and
the retry instruction, ending before Escalation. A missing or ambiguous path reports the concept
ID. A query continues with other matches and marks the affected delivery incomplete.

## Check what the query examined

```sh
okf query triage-notes features/export --bundle product --json
```

Read `saved-query-summary` for the input and distinct output counts of each stage, unresolved
endpoints, and matching and delivery completeness. Check the final `operation-complete` and
the process exit as well. Missing endpoints or a scan limit can make matching partial even if the
last stage returns zero concepts. Pagination can deliver only part of a complete match set.

This example reports `strategy: scope` because finding incoming links requires examining possible
authors. Preset starts, explicit examination scope, historical reads, and traversed provenance
snapshots also use the scoped executor. For cross-bundle reads, select a primary bundle and request
the other bundles you intend to examine. See [Run a saved query across bundles](work-with-multiple-bundles.md#run-a-saved-query-across-bundles).

If you already know the defect ID, read its triage note directly:

```sh
okf query triage-for-defect defects/csv --bundle product --json
```

The projection returns Marin. These fields from its summary describe the two examined documents,
the defect and its triage note.

```json
{
  "strategy": "outgoing",
  "examination": {"examined_documents": 2, "eligible_documents": 2, "scan_complete": true},
  "observed_matches": 1
}
```
The `kind` filter excludes the blocking relationship before the executor
reads its target.

A known starting ID in a single working-tree bundle with only outgoing hops can report
`strategy: outgoing`. It reads reached documents on demand, including the start, and reuses
document and missing-target lookups for that invocation. A zero-hop read also qualifies.
`examined_documents` counts distinct parsed documents. `eligible_documents` adds reached
candidate IDs deferred by the scan budget. Their descendants remain unknown. Broad fan-out
increases the number of reads, and different strategies can return different partial results
under the same tight budget.

A complete outbound result establishes reachability along the requested chain. Unrelated
documents with malformed YAML may remain unread. Malformed YAML in a reached document fails
the read. Use `okf validate product --profile bundle` to check bundle-wide health.
For all flags and output records, see [okf query](../commands/query.md).
