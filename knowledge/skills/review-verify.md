---
type: Skill
trigger: review and verify concepts
uses:
- /commands/catalog
- /commands/verify
- /commands/show
- /commands/artifact-resolve
- /commands/stale
- /commands/list
- /commands/stats
- /commands/artifact-show
- /commands/computation-check
title: okf:review-verify skill
description: Reviews concepts against declared evidence and appends document-level verified events with the real reviewer actor, never as a substitute for runtime attestation.
sources:
- resource: plugins/okf/skills/review-verify/references/cli.md
  kind: git-path
- resource: plugins/okf/skills/review-verify/SKILL.md
  kind: git-path
last_modified: 2026-10-04T20:39:29Z
---
# okf:review-verify skill

Separates source-backed document verification from execution attestation, validates reviewer
identity, and reports the exact verification event and remaining lifecycle limitations.

The workflow connects [verify](../commands/verify.md), [show](../commands/show.md),
[artifact resolve](../commands/artifact-resolve.md), [stale](../commands/stale.md),
[list](../commands/list.md), [stats](../commands/stats.md),
[artifact show](../commands/artifact-show.md), and
[computation check](../commands/computation-check.md).

Use [catalog](../commands/catalog.md) inspection to establish effective identity, local locations
and interpretation settings. Additional bundle examination is explicit; report unavailable members
and keep current candidates separate from requested snapshot evidence.
