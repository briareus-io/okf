---
type: Guide
audience: user
uses:
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
from this repository's root with a compatible CLI on PATH. These commands are read-only.

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

## Know when the answer is complete

For a question such as “which documents matched this phrase?”, check the reported scan
scope and completeness. A limit of ten bounds results, not examination work. Do not infer
absence from an incomplete scan, an unavailable bundle, or a truncated related view.

Use [bounded reads](bounded-reads.md) to control those costs,
[selectors](../reference/selectors.md) for structured filtering, and
[JSON automation](automate-with-json.md) when a program must enforce completeness.
