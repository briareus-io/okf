---
type: Guide
audience: user
uses:
- /commands/affected
- /commands/artifact-put
- /commands/artifact-show
- /commands/add
- /commands/edit
- /commands/show
- /commands/refresh
- /commands/verify
- /commands/validate
- /commands/lint
explains:
- /concepts/sources-and-provenance
- /concepts/verification-and-freshness
title: Maintain source-backed knowledge
description: Find impacted documentation, compare current sources, and reconcile claims without stored fingerprints.
---
# Maintain source-backed knowledge

Use this workflow after a source changes: identify affected documents, compare their claims
with the current source, edit what changed, and validate the result. Source relationships
identify candidates for review; they do not decide which claims are wrong.

## Find affected documentation in this repository

Run these read-only commands from the repository root:

```sh
okf affected knowledge --changed crates/okf-core/src/mutate/refresh.rs --changed-base repo
okf show commands/refresh knowledge
```

Expect the refresh command page among the dependents. `--changed-base repo` means the input
path starts at the Git worktree root. It does not change the meaning of stored source records.
Read the current source file and the relevant prose before deciding whether an edit is needed.

This manual stores current source paths and their kinds, but no fingerprints. It disables
only the missing-baseline lint warning. `stale` can report missing/unavailable resources and
unrecorded baselines, but cannot establish fingerprint drift without a baseline. Known changes
and `affected` provide the maintenance starting point here.

## Practice a source change without editing the manual

The example starts with a service that retries twice. Its documentation cites a text artifact.
Run from the repository root, in one shell; all writes stay under a temporary directory.

```sh
demo_root=$(mktemp -d)
okf init "$demo_root/bundle"
printf 'retry_limit=2\n' > "$demo_root/service.txt"
okf artifact put references/service.txt "@$demo_root/service.txt" "$demo_root/bundle"
okf add notes/service "$demo_root/bundle" --type Note --title "Service retries" --description "Retry behavior from the service configuration" --body "The service retries twice."
okf edit notes/service "$demo_root/bundle" --add-source 'resource=references/service.txt,kind=file'
```

The source uses `kind=file`, whose resource resolves from the bundle root. Adding a source
records attribution and resolution semantics; it does not create a fingerprint or a review.

Now change the source and inspect the resulting review candidate:

```sh
printf 'retry_limit=3\n' > "$demo_root/service.txt"
okf artifact put references/service.txt "@$demo_root/service.txt" "$demo_root/bundle" --replace
okf affected "$demo_root/bundle" --changed references/service.txt
okf artifact show references/service.txt "$demo_root/bundle"
okf show notes/service "$demo_root/bundle"
```

Expect `/notes/service` in the affected result. The source now says `retry_limit=3`, while the
body still says twice. The contradiction is visible by reading both; no hash comparison was
needed to identify this known change.

## Reconcile the claim, then check it

```sh
okf edit notes/service "$demo_root/bundle" --set-body "The service retries three times." --dry-run
okf edit notes/service "$demo_root/bundle" --set-body "The service retries three times."
okf show notes/service "$demo_root/bundle"
okf validate "$demo_root/bundle"
okf lint "$demo_root/bundle" --fail-on error
```

Review the preview before applying the same edit. The final document should keep its source
entry and say three times. Validation checks format; lint checks local conventions. Neither
check verifies the service's real runtime behavior. Depending on the surrounding configuration,
lint may report an unrecorded fingerprint as an advisory warning.

## Choose the next action deliberately

| Observation | Action |
| --- | --- |
| Source changed but the documented behavior did not | Keep the prose; record your review through your team's process. |
| Source and prose disagree | Correct the claim and inspect the resulting diff. |
| Source is missing or inaccessible | Restore or replace the source evidence; do not invent its contents. |
| Your own bundle tracks fingerprint baselines | After review, use [refresh](../commands/refresh.md) to record supported current fingerprints. |
| A real document review has been performed | Use [verify](../commands/verify.md) with the actual reviewer and only the scope reviewed. |

The checked-in manual deliberately does not track fingerprint baselines; do not run refresh
against it merely to remove a diagnostic. For agent-assisted reconciliation, use the
[update skill](../skills/update.md). See [provenance](../concepts/sources-and-provenance.md),
[verification](../concepts/verification-and-freshness.md), and
[source-health troubleshooting](../reference/troubleshooting.md) for the distinctions.
