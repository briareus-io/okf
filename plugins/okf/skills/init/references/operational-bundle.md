# Maintain a lean operational bundle

Use a separate bundle when temporary execution records have a different owner and retention cycle
from durable knowledge. The example uses stable authored IDs and movable concept paths. Task
readiness, unique-ID allocation, cycle detection, and allowed state transitions remain application
policy: a typed reference and a successful OKF validation do not establish those properties.

Follow the [shared protocol](../../../references/bundle-context.md). Run this example in a new
scratch directory; adapt declarations narrowly for an existing bundle. The input files below live
outside the bundle. No routine fingerprints or verification records are needed for these records.

## Initialize and declare local rules

```sh
okf init planning
okf bundle context planning --operation 'ontology apply' --operation 'changeset plan' --operation 'changeset apply' --operation list --operation links --operation validate --operation lint --json
cat > operational-ontology.yaml <<'YAML'
concepts:
  Milestone:
    fields:
      id: {type: string, required: true}
      state: {type: enum, values: [queued, active, done], required: true}
  Defect:
    fields:
      id: {type: string, required: true}
      state: {type: enum, values: [open, resolved], required: true}
  Task:
    fields:
      id: {type: string, required: true}
      state: {type: enum, values: [queued, active, done], required: true}
    references:
      milestone: {target: Milestone, cardinality: '0..1'}
      depends_on: {target: Task, cardinality: '0..n'}
      resolves: {target: Defect, cardinality: '0..n'}
YAML
okf ontology apply planning --from @operational-ontology.yaml
```

`state` is an application field. It does not replace OKF's `status` lifecycle. Here, omitted
`status` has its normal effective value of `stable`; that does not mean a task is complete.
Optional kind-less `sources` can record provenance without opting into fingerprint refreshes.
Do not invent sources or review actors merely to fill fields.

## Create related records in one changeset

```sh
cat > initial-records.yaml <<'YAML'
version: 1
operations:
  - op: create
    path: milestones/M1.md
    frontmatter: {type: Milestone, id: M1, title: First release, description: Deliver the first release., state: queued}
    body: Coordinate the first release.
  - op: create
    path: defects/D1.md
    frontmatter: {type: Defect, id: D1, title: Retry duplicates, description: A repeated request creates duplicate work., state: open}
    body: Preserve the reproduction and acceptance evidence here.
  - op: create
    path: tasks/T1.md
    frontmatter:
      type: Task
      id: T1
      title: Make retries idempotent
      description: Reuse the original result for a repeated request.
      state: queued
      milestone: /milestones/M1.md
      resolves: [/defects/D1.md]
    body: Add the implementation and a regression for repeated requests.
  - op: create
    path: tasks/T2.md
    frontmatter:
      type: Task
      id: T2
      title: Exercise retry recovery
      description: Check recovery after an interrupted request.
      state: queued
      milestone: /milestones/M1.md
      depends_on: [/tasks/T1.md]
    body: Run the recovery scenario after T1 is complete.
YAML
okf changeset plan planning --from @initial-records.yaml --json --guidance off > plan.ndjson
```

Check the process exit and completion record, then review the diffs and `base_digest` in the
`changeset-summary`. Apply the same reviewed input with that exact digest:

```sh
okf changeset apply planning --from @initial-records.yaml --expect '<reviewed base_digest>'
okf validate planning --profile bundle
okf lint planning --fail-on never
okf links /tasks/T2 planning --details
```

The quoted digest is a placeholder for the reviewed value, not a special CLI token. For a
read-compute-write operation, capture `changeset digest` before the inventory used for the decision,
then supply it to apply. Do not replace an old digest with a newly captured one merely to make a
conflicting write pass. `apply` without `--expect` already validates and publishes in one invocation;
it cannot detect a stale application decision made before that invocation.

## Read identities and dependencies completely

```sh
okf list planning --full-scan --json --guidance off > inventory.ndjson
okf search planning --type Task --project '$id' --project id --project state --project depends_on --full-scan --json --guidance off
```

In an unprojected concept row, `id` is the computed path, such as `/tasks/T1`, and
`frontmatter_conflicts.id` is the authored stable ID `T1`. In projection records, `$id` selects the
computed path and `id` selects the authored value; projections are separate records, not a new
layout for every row. Preserve the bundle/revision identity when joining records. Do not infer
record identity from a basename or assume that moving a concept changes its authored ID.

Require the supported `operation-start`, terminal `operation-complete`, successful process exit,
and complete examination/delivery. Reject missing completion, warnings that invalidate the desired
inventory, and unexpected stream shapes. A result limit or partial scan cannot establish the full
dependency set. Read once and calculate readiness locally instead of rescanning for every page.
Use explicit dependency edges; do not interpret free prose as an executable dependency graph.

## Edit, order, and recover

Batch related changes using `edit` operations with guarded RFC 6902 patches. A `test` operation
can assert the expected authored ID or state before replacing a value. Source replacements should
likewise test the expected source object and replace it in one edit, rather than remove and add in
separate writes. Meaningful concept edits invalidate active verification; preserving history does
not make it current verification. There is no unrestricted `--keep-verified` escape hatch.

Operations run in their supplied order. If a changeset edits a body to mention `/tasks/T1.md` and
then moves T1 to `/tasks/retry.md`, the move can rewrite that link. If it moves T1 first and then
writes the old link, final validation rejects the newly introduced dangling reference. Prefer
writing final target paths when order should not affect the result. For historical bare body links,
`okf doctor planning --normalize-relative-links` previews normalization to checked current
document-relative targets; add `--yes` only after reviewing those mappings. Missing, blocked, or
ambiguous destination syntax is left unchanged. This opt-in choice is separate from `--fix-safe`:
a missing old target does not establish the author's original intent. When combined with safe
repairs, normalization and those repairs publish separately.

Publication uses a recovery journal and per-file replacement. Keep application read-compute-write
operations coordinated. Ordinary working-tree CLI reads hold shared publication guards through
payload delivery and refuse an active publisher or pending recovery journal. This does not make
several separate commands one snapshot and cannot stop editors or other tools that bypass the
locks. On interruption inspect
the completion/publication evidence and use `okf changeset recover planning`; do not delete the
journal by hand. Retain a baseline guard even when the harness has its own cooperative lock.

Managed immutable snapshots need no coordination writes. A read-only root without an existing
lock remains readable, but that fallback does not coordinate with privileged or external writers.

Changesets write one bundle. Durable bundles should not depend on disposable planning records.
For cross-bundle references, use ordinary relative paths or `okf://<bundle-id>/<path>`, include
required examination scope, and reconcile external consumers explicitly before a move. Scoped
move guards do not provide multi-bundle rewriting or atomic cross-bundle publication.
