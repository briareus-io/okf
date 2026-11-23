---
type: Guide
audience: user
uses:
- /commands/changeset-plan
- /commands/changeset-apply
- /commands/mv
- /commands/links
- /commands/validate
- /commands/lint
explains:
- /concepts/bundles-and-concepts
title: Reorganize a bundle
description: Preview, apply and verify a coordinated move, retarget and removal.
sources:
- resource: /references/changesets/reorganize.yaml
---
# Reorganize a bundle

Suppose `plans/current` is the maintained plan and `plans/obsolete` is ready to retire. A consumer still links to the obsolete plan. Move the maintained plan to its new name, redirect the consumer, and remove the obsolete plan in one reviewed changeset.

## Create the starting records

In an empty working directory, initialize a bundle.

```sh
okf init bundle
```

The starting bundle contains two plans and one note. Save each example under the indicated
path, creating its parent directory if needed.

**File: `bundle/plans/current.md`**

```markdown
---
type: Note
title: Current plan
description: The current plan
---
The maintained plan.
```

**File: `bundle/plans/obsolete.md`**

```markdown
---
type: Note
title: Obsolete plan
description: A superseded plan
---
The old plan.
```

**File: `bundle/notes/consumer.md`**

```markdown
---
type: Note
title: Plan consumer
description: A plan dependency
---
[Plan](../plans/obsolete.md)
```

## Describe and preview the changes

Save the [worked changeset](../references/changesets/reorganize.yaml) beside the bundle. The operation order puts the destination in place before redirecting references to it.

**File: `reorganize.yaml`**

```yaml
version: 1
operations:
  - op: move
    from: plans/current
    to: plans/revised
  - op: retarget
    from: plans/obsolete
    to: plans/revised
  - op: remove
    path: plans/obsolete.md
```

Preview the diffs.

```sh
okf changeset plan bundle --from @reorganize.yaml --json
```

Look for the move to `plans/revised`, the changed link in `notes/consumer`, and removal of `plans/obsolete.md`. The preview leaves all three records in their original state. Copy `base_digest` from the `changeset-summary` after reviewing the diffs.

## Apply and check the reviewed plan

Replace `REVIEWED_BASE_DIGEST` with the value you just copied. Use the same bundle and changeset file.

```sh
okf changeset apply bundle --from @reorganize.yaml --expect REVIEWED_BASE_DIGEST
```

Read the consumer.

```sh
okf show notes/consumer bundle --body
```

The note now points to the revised plan:

```markdown
[Plan](../plans/revised.md)
```

Confirm that the target resolves:

```sh
okf links notes/consumer bundle
```

The result should identify `/plans/revised` as an existing target. Check the final bundle.

```sh
okf validate bundle
```

```sh
okf lint bundle --fail-on error
```

## Handle a conflict or incomplete move

| Result | Action |
| --- | --- |
| Baseline changed since the preview | Inspect the intervening changes. Run the plan again and review the new diffs and digest before applying. |
| Destination already exists | Choose a free destination or reconcile the two records explicitly. |
| Dangling-reference check fails | Add the missing retarget or retain the old record until you have handled its consumers. |
| Publication was interrupted | Use [changeset recovery](../reference/troubleshooting.md#recover-an-interrupted-write) to inspect the journal and recover the per-file writes. |

For a single move, use [mv](../commands/mv.md). When splitting a record, decide which new concept each consumer needs. Inspect references across the relevant bundles and coordinate their edits separately. Review curated navigation after a move. The [docs](../commands/docs.md) index generator replaces the prose in indexes it regenerates.
