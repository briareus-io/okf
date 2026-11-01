---
type: Guide
audience: user
uses:
- /commands/init
- /commands/add
- /commands/browse
- /commands/search
- /commands/show
- /commands/backlinks
- /commands/links
explains:
- /concepts/bundles-and-concepts
- /concepts/ontology-and-relationships
title: Find and read knowledge
description: Navigate, search, inspect and follow relationships in this manual.
---
# Find and read knowledge

Use this workflow when you have a question rather than a known concept ID. The worked
question is: **which workflow should I use after a source file changes?** Run the examples
from this repository's root with a compatible CLI on PATH. The manual queries are read-only; the optional relationship recipe writes only to a disposable bundle.

## Find the relevant explanation

Start with navigation, then search titles and descriptions:

```sh
okf browse knowledge
okf browse knowledge --directory guides
okf search knowledge --text "source" --in title,description --limit 10
```

Expect the source-maintenance guide and provenance topic among the results. The root index
separates reader and contributor entry points; the guide index groups tasks. Search results
help select a document, but snippets are not enough to establish its meaning.

Read the selected explanation and workflow:

```sh
okf show concepts/sources-and-provenance knowledge
okf show guides/maintain-source-backed-knowledge knowledge
```

The topic explains what source records mean. The guide explains which commands to run and
what decisions remain yours. If the initial phrase finds nothing, broaden it with
`--match any` or try the relevant type before concluding that documentation is absent.

## Choose metadata, an outline, or the body

```sh
okf show commands/affected knowledge --json
okf show reference/command-contracts knowledge --outline
okf show commands/affected knowledge --body --json
```

The first command returns frontmatter and computed views, not prose. The outline reports
numbered headings; choose an inclusive line window with `show --lines START:END` when a
large reference has only one relevant section. The last command explicitly returns the
body. Use the full body when reviewing a claim whose surrounding qualifications matter.

## Follow the actual workflow relationships

```sh
okf links guides/maintain-source-backed-knowledge knowledge --details
okf backlinks commands/refresh knowledge --details
okf search knowledge --type Command --facet-filter 'mutates=true' --view commands --full-scan
```

The guide's declared `uses` edges identify its commands. Incoming references to `refresh`
include the maintenance guide and the update skill. The final query identifies commands
with write capability; that does not mean every invocation writes. For example, dry-run
and fix flags matter. Read the individual command page before using it.

At this point you can answer the worked question: use `affected` to find documents citing
a known changed file, read those sources and documents, and apply the maintenance workflow.
The graph identifies relationships; it does not decide whether the prose is correct.

## Find defects blocking a feature

For a domain question, filter relationship occurrences rather than treating every backlink as the
same assertion. This independent scratch example declares `Defect --blocks--> Feature`, its inverse
label, and a named query preset:

```sh
demo_root=$(mktemp -d)
cd "$demo_root"
okf init "$demo_root/relationships"
cat > "$demo_root/relationships/ontology.yaml" <<'YAML'
okf_ontology: '0.1'
concepts:
  Feature: {}
  Defect:
    references:
      blocks: {target: Feature, cardinality: '0..1'}
    relationships:
      blocking: {reference: blocks, kind: blocks, inverse: is-blocked-by}
YAML
cat > "$demo_root/relationships/okf-bundle.yaml" <<'YAML'
manifest_version: 1
settings:
  query_presets:
    blocking-defects:
      description: Defects with authored blocking relationships to the selected feature.
      operations: [backlinks]
      filters:
        kind: [blocks]
        related_type: [Defect]
YAML
okf add features/export "$demo_root/relationships" --type Feature --title "Export" --description "Export feature" --body "Export records."
okf add defects/csv "$demo_root/relationships" --type Defect --title "CSV defect" --description "Blocks export" --set blocks=/features/export.md --body "CSV serialization fails."
okf add notes/discussion "$demo_root/relationships" --type Note --title "Discussion" --description "Context only" --body '[Export discussion](/features/export.md)'
okf backlinks features/export "$demo_root/relationships" --preset blocking-defects --json
okf backlinks features/export "$demo_root/relationships" --inverse-kind is-blocked-by --related-type Defect --details --json
```

Both queries identify `defects/csv`; the ordinary discussion link is not a blocking relationship.
`--kind` always matches the authored kind, even for backlinks. `--inverse-kind` matches the configured
inverse label of the same occurrence, and `--related-type` matches the other endpoint. For backlinks,
that endpoint is the source defect. Detailed output preserves individual relationship occurrences.

The preset is selected explicitly and does not narrow ordinary backlink queries or integrity checks.
A query establishes declared blockers within its examined scope, not that a defect remains active or
that the feature cannot be implemented. Add explicit lifecycle or edge-attribute predicates when the
model requires them, and check scan completeness before saying “all blockers.” See
[backlinks](../commands/backlinks.md) for the supported filters and [bounded reads](bounded-reads.md)
for scan versus output limits.

## Know when the answer is complete

For a question such as “which documents matched this phrase?”, check the reported scan
scope and completeness. A limit of ten bounds results, not examination work. Do not infer
absence from an incomplete scan, an unavailable bundle, or a truncated related view.

Use [bounded reads](bounded-reads.md) to control those costs,
[selectors](../reference/selectors.md) for structured filtering, and
[JSON automation](automate-with-json.md) when a program must enforce completeness.
