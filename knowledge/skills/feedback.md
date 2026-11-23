---
type: Skill
trigger: capture authoring feedback or a workflow lesson
uses:
- /commands/learn-draft
- /commands/learn-export
- /commands/learn-report
title: okf:feedback skill
description: Capture a private synthetic defect report, feature request, guidance gap, or lesson.
sources:
- resource: plugins/okf/skills/feedback/SKILL.md
  kind: git-path
---
# Capture authoring feedback

Invoke the feedback skill to turn a generalized account of an authoring experience into a
local draft. Collection is optional. Use invented inputs for a reproduction or worked
scenario and exclude real names and data. Review the complete text before any local export.
See [Capture local authoring feedback](../guides/capture-local-feedback.md) for draft, export, and optional counting examples.
