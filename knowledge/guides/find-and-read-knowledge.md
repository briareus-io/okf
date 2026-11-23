---
type: Guide
audience: user
uses:
- /commands/browse
- /commands/search
- /commands/show
- /commands/list
- /commands/backlinks
- /commands/links
- /commands/graph
explains:
- /concepts/bundles-and-concepts
- /concepts/ontology-and-relationships
title: Find and read knowledge
description: Navigate, search, inspect and follow relationships in this manual.
---
# Find and read knowledge

After a source file changes, which workflow should you follow? The examples below read the
documentation bundle in this repository. Run each command from the repository root.

## Find the relevant explanation

```sh
okf browse knowledge --directory guides
```

The guide index groups tasks and links to **Maintain source-backed knowledge**. To search by
topic when you know a phrase from the title or description, try:

```sh
okf search knowledge --text source --in title,description --limit 10
```

The result table includes `/concepts/sources-and-provenance` and
`/guides/maintain-source-backed-knowledge`. Read the workflow you found:

```sh
okf show guides/maintain-source-backed-knowledge knowledge --body
```

The returned guide explains how to use `affected` to find documents citing a changed source,
compare their claims with the current evidence, and reconcile the documents.

## Choose fields or a passage

For one metadata field, request a projection.

```sh
okf show commands/affected knowledge --project group
```

```text
check
```

Plain `show --json` includes authored frontmatter and computed metadata. For prose from a long
document, first inspect its heading hierarchy.

```sh
okf show reference/command-contracts knowledge --outline
```

The outline includes **Consistent command behavior → Find and inspect knowledge → Selection and
scope**. Use that complete heading path to read the subsection.

```sh
okf show reference/command-contracts knowledge --section '["Consistent command behavior","Find and inspect knowledge","Selection and scope"]'
```

The excerpt starts at `### Selection and scope` and explains how to select a bundle by path or
catalog ID. Use `--lines START:END` for a numbered window or `--body` for the complete prose.

## Reuse filters and add criteria

The manual defines a `writing-commands` preset for both `list` and `search`. It selects Command
documents with `mutates=true`. Add text to keep that condition and find commands about fingerprints.

```sh
okf search knowledge --preset writing-commands --text fingerprint --in title,description
```

The result is `/commands/refresh`. The `text` category and the preset's `facet-filter` category
both apply. Supplying a new facet filter replaces that category for this call.

```sh
okf list knowledge --preset writing-commands --facet-filter 'title="okf show"'
```

This returns `/commands/show`. The effective filter requires a Command with the requested title.
To also require the preset's original mutation condition, repeat it alongside the new condition.

```sh
okf list knowledge --preset writing-commands --facet-filter 'mutates=true' --facet-filter 'title="okf show"'
```

```text
(no concepts)
```

`show` is a read operation. The same category replacement rule applies independently to type,
tag, text, field, and facet-filter. Use `--clear-filter facet-filter` to drop that category.
To select documents with a particular field, use a condition such as `mutates exists` in
`query.facet_filter`. See [customize a bundle](customize-a-bundle.md) for the manifest format.

## Follow a relationship across two hops

Which topics explain the work behind `refresh`? Start at the command. The manual's
`guides-using-command` preset follows incoming `uses` relationships and keeps Guides as the
other endpoint.

```sh
okf backlinks commands/refresh knowledge --preset guides-using-command --details
```

The matching source is `/guides/maintain-source-backed-knowledge`. The guide authors `uses`
toward the command. A filter on the inverse label `--inverse-kind used-by` selects the same
relationship when reading it in this direction. The preset excludes ordinary Markdown mentions.

From the guide, follow its outbound `explains` relationships to Topic targets.

```sh
okf links guides/maintain-source-backed-knowledge knowledge --kind explains --related-type Topic
```

The two targets are `/concepts/sources-and-provenance` and
`/concepts/verification-and-freshness`. When the first hop returns several guides, run the second
hop for each one. To name a chain, filter its intermediate concepts or read one subsection per
match, see [save and run a query](save-and-run-queries.md).

For a picture of every incoming path within two hops, root a graph at the command.

```sh
okf graph knowledge --root commands/refresh --direction incoming --depth 2
```

The Mermaid graph includes the maintenance guide and other referrers. Depth bounds the distance
from the root. A graph filter applies at every hop. On `list` and `search`, expansion filters
limit displayed outbound relationships while retaining the primary hits.

## Know when the answer is complete

A limit of ten bounds the results displayed. Before reporting absence or exact totals, check the
scope, scan completeness, and delivery. For machine consumers, inspect the query summary, final
operation record, and process exit. See [bounded reads](bounded-reads.md),
[selectors](../reference/selectors.md), and [JSON automation](automate-with-json.md).
