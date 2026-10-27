---
type: Skill
trigger: answer questions from a bundle
uses:
- /commands/catalog
- /commands/browse
- /commands/search
- /commands/show
- /commands/links
- /commands/backlinks
- /commands/list
- /commands/graph
- /commands/resolve
- /commands/computation-check
- /commands/artifact-resolve
- /commands/artifact-show
title: okf:retrieval skill
description: Answers from a bundle through progressive disclosure, source lineage, bounded artifact retrieval, and explicit lifecycle, verification, and runtime caveats.
sources:
- resource: plugins/okf/skills/retrieval/references/cli.md
  kind: git-path
- resource: plugins/okf/skills/retrieval/SKILL.md
  kind: git-path
last_modified: 2026-10-19T03:58:59Z
---
# okf:retrieval skill

The primary read-only consumer skill. It follows concepts, claim sources, and optional artifacts
without bulk loading or confusing a verified computation definition with an attested run.

The workflow connects [browse](../commands/browse.md), [search](../commands/search.md),
[show](../commands/show.md), [links](../commands/links.md),
[backlinks](../commands/backlinks.md), [list](../commands/list.md),
[graph](../commands/graph.md), [resolve](../commands/resolve.md),
[computation check](../commands/computation-check.md),
[artifact resolve](../commands/artifact-resolve.md), and
[artifact show](../commands/artifact-show.md).

Use `links --details` when outgoing kinds, inverse labels, attributes and concrete field paths
matter; use `backlinks --details` for incoming assertions. Preserve repeated semantic occurrences.
Local detailed JSON combines `relationship` and ordinary `link` records; catalog queries use
`bundle-edge` records with optional relationship metadata. Inverse labels do not add edges.

Use [catalog](../commands/catalog.md) inspection to establish effective identity, local locations
and interpretation settings. Additional bundle examination is explicit; report unavailable members
and keep current candidates separate from requested snapshot evidence.
