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

Use this guide when related records must move or disappear together. You need the CLI on
PATH and the checked-in changeset artifact. The example has three starting records:

| Record | Starting role | Final state |
| --- | --- | --- |
| `plans/current` | Maintained plan | Moved to `plans/revised` |
| `plans/obsolete` | Superseded plan | Removed after references are retargeted |
| `notes/consumer` | Links to the superseded plan | Links to `plans/revised` |

## Prepare the fixture and inspect a plan

The [worked changeset](../references/changesets/reorganize.yaml) moves a current plan,
retargets a consumer of an obsolete plan, and removes the obsolete record. Create the
small fixture below in a disposable directory, from the repository root:

```sh
demo_root=$(mktemp -d)
okf init "$demo_root/bundle"
okf add plans/current "$demo_root/bundle" --type Note --title "Current plan" --description "The current plan" --body "The maintained plan."
okf add plans/obsolete "$demo_root/bundle" --type Note --title "Obsolete plan" --description "A superseded plan" --body "The old plan."
okf add notes/consumer "$demo_root/bundle" --type Note --title "Plan consumer" --description "A plan dependency" --body '[Plan](../plans/obsolete.md)'
okf changeset plan "$demo_root/bundle" --from @knowledge/references/changesets/reorganize.yaml --json
```

## Review and apply the exact plan

Expect the preview to include the moved plan, removal of the obsolete plan, and a rewritten
consumer link. The fixture must still be unchanged after the preview.

Inspect the file diffs and returned `base_digest`. Copy that exact value into `base_digest`
below, then apply the unchanged document against the same fixture:

```sh
base_digest=REPLACE_WITH_REVIEWED_BASE_DIGEST
okf changeset apply "$demo_root/bundle" --from @knowledge/references/changesets/reorganize.yaml --expect "$base_digest"
okf show notes/consumer "$demo_root/bundle"
okf links notes/consumer "$demo_root/bundle"
okf validate "$demo_root/bundle"
okf lint "$demo_root/bundle" --fail-on error
```

The consumer should now link to `plans/revised`; `plans/current` and `plans/obsolete` should
be absent. If another change invalidates the baseline, review a new plan rather than dropping
`--expect`. Publication is journaled per file, not an atomic snapshot for concurrent readers.
[Recovery](../reference/troubleshooting.md#recover-an-interrupted-write) handles interrupted writes.

For a single move use [mv](../commands/mv.md). Splits require explicit consumer mappings;
automatic cross-bundle move orchestration is not provided. Curated indexes need editorial
attention: [docs](../commands/docs.md) index generation replaces their prose.

## Resolve a conflict or unexpected result

| Result | Next step |
| --- | --- |
| Baseline no longer matches | Inspect the intervening changes and create a fresh plan; keep `--expect`. |
| Destination already exists | Choose a free destination or explicitly reconcile the two records. |
| Dangling-reference check fails | Add the missing retarget or retain the referenced record until its consumers are handled. |
| Publication was interrupted | Inspect the journal and use [changeset recover](../commands/changeset-recover.md); do not delete coordination files to force a retry. |

For your own bundle, start with an explicit old-to-new mapping and inspect incoming links.
The tool can apply that mapping; it cannot infer which destination each consumer of a split
record should use. Review curated navigation after a move, as well as concept-to-concept links.
