---
type: Skill
trigger: reverse-engineer an ontology from a bundle
uses:
- /commands/ontology-apply
- /commands/ontology-field-type-add
- /commands/ontology-field-type-update
- /commands/ontology-field-type-remove
- /commands/catalog
- /commands/list
- /commands/stats
- /commands/show
- /commands/resolve
- /commands/links
- /commands/backlinks
- /commands/computation-check
- /commands/ontology-add
- /commands/ontology-update
- /commands/validate
- /commands/lint
- /commands/artifact-resolve
title: okf:infer-ontology skill
description: Infers advisory custom fields and references from support counts while excluding standard OKF families and preserving exact unknown types.
sources:
- resource: plugins/okf/skills/infer-ontology/references/cli.md
  kind: git-path
- resource: plugins/okf/skills/infer-ontology/SKILL.md
  kind: git-path
- resource: plugins/okf/skills/ontology/references/structured-authoring.md
  kind: git-path
last_modified: 2026-10-05T10:16:06Z
---
# okf:infer-ontology skill

Reports evidence and confidence, recognizes the exact built-in computation contract, and never
infers required rules from a small sample.

The workflow connects [list](../commands/list.md), [stats](../commands/stats.md),
[show](../commands/show.md), [resolve](../commands/resolve.md),
[links](../commands/links.md), [backlinks](../commands/backlinks.md),
[computation check](../commands/computation-check.md),
[ontology add](../commands/ontology-add.md), [ontology update](../commands/ontology-update.md),
[validate](../commands/validate.md), [lint](../commands/lint.md), and
[artifact resolve](../commands/artifact-resolve.md).

Use [catalog](../commands/catalog.md) inspection to establish effective identity, local locations
and interpretation settings. Additional bundle examination is explicit; report unavailable members
and keep current candidates separate from requested snapshot evidence.

Structured authoring follows the ontology skill's shared example. Use structured declaration
YAML/file inputs and bulk apply for dependent schemas; use concept `--set-yaml`, object paths,
or RFC 6902 patches for frontmatter. Review dry-run and actual diffs, preserve source lineage,
and retain meaningful-change verification consequences. Model validation does not attest claims.
