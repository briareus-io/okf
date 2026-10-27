---
type: Reference
audience: user
title: Consistent command behavior
description: Shared CLI input, output, scope, source-health and publication behavior.
---
# Consistent command behavior

OKF CLI 0.5.0 provides predictable input handling, structured results, guarded writes,
and source-health checks for people, agents, and programs. These capabilities operate
on ordinary OKF v0.2 bundles; they do not introduce a new document format.

Use this guide to choose commands and understand their shared behavior. The
[CLI reference](../commands/index.md) and `okf schema` describe the exact arguments
available in your binary. See [checking results and recovering writes](troubleshooting.md)
for automation and troubleshooting, and [performance and bounded reads](../guides/bounded-reads.md)
for larger bundles. Integrations upgrading from 0.4.x should first read the
[migration guide](migrations/0.5.0.md).

## Find and inspect knowledge

Start with a directory overview, search for relevant concepts, then read only the
metadata or content you need:

```sh
okf browse knowledge
okf search knowledge --text "review policy" --limit 10 --json
okf show policies/review knowledge --outline
okf show policies/review knowledge --body --json
```

`list` and `search` support the same metadata filters, projections, and display views.
The following examples select several types, exclude a metadata value, and request
specific fields without loading each result through separate `show` calls:

```sh
okf list knowledge --facet-filter 'type in ["Policy","Procedure"]' --json
okf list knowledge --facet-filter 'status not in ["deprecated"]' --json
okf list knowledge --columns '$id,title,status'
okf show policies/review knowledge --project '$id' --project 'owner' --json
```

Filters combine with AND. Membership within one filter expresses alternatives.
Missing or null metadata does not satisfy negative membership: the second example
selects explicitly authored, non-deprecated status values, not every concept whose
effective status is current. An absent concept status still means `stable` for OKF
lifecycle interpretation. Use a suitable local view or explicit metadata policy when
selecting current guidance; do not assume a filter fills in missing authored values.

Computed `$id` and authored metadata `id` are separate projection selectors. Existing
unknown metadata remains preserved. Columns affect human display; they do not silently
remove fields from JSON output.

### Selection and scope

A trailing bundle argument selects a directory. `--bundle-id` selects a registered
catalog identity; do not supply both. Without an explicit selector, configuration and
working-directory rules select the primary bundle.

Catalog registration does not automatically broaden a query. Commands supporting
additional scope accept `--scope-bundle` or deliberate `--catalog-scope`:

```sh
okf search --bundle-id acme.product --scope-bundle acme.policy --text "retention" --json
```

Inspect requested, examined, and unavailable scope in results before drawing conclusions.
A scoped query is not a global inventory of every possible consumer. See the
[multi-bundle capability](../contributing/design/multi-bundle-capability.md) for registration and reference rules.

### Which commands support which reads?

| Task | Commands | Scope and read controls |
| --- | --- | --- |
| Search or inventory concepts | `list`, `search` | Catalog scope, existing `--revision`, filters, projections, scan budgets, pagination |
| Read one concept or navigation page | `show`, `browse` | Selected bundle; `show` offers projections, outline, body, and line slices |
| Inspect references and impact | `links`, `backlinks`, `graph`, `resolve`, `affected` | Catalog scope and existing `--revision`; command-specific traversal controls |
| Check conformance or source health | `validate`, `stale`, `stats`, `doctor` | Selected bundle; diagnostics and applicable failure controls |
| Lint concepts | `lint` | Catalog scope for reads; fixes apply to one selected bundle |
| Inspect artifacts or computations | `artifact list/resolve/show`, `computation check` | Selected bundle and explicit resource context; bounded artifact reads; no code execution |
| Inspect configuration or source files | `catalog`, ontology inspection, `scan`, `source-scan` | Each command's declared configuration or directory scope; `source-scan` does not require a bundle |
| Compare content with Git | `diff` | Selected bundle and its explicit comparison revision |

Unsupported options are rejected rather than silently expanding capabilities. For example,
`show` does not acquire catalog scope or revision support merely because `search` has it.

## Supply changed resources explicitly

`affected` finds concepts that depend on changed concepts or source resources. Choose the
namespace of the input paths:

```sh
okf affected knowledge --changed policies/review.md
okf affected knowledge --changed src/service.rs --changed-base repo
okf affected knowledge --changed /work/project/src/service.rs --changed-base filesystem
```

| Base | Interpretation |
| --- | --- |
| `bundle` (default) | Existing bundle-relative concept/resource interpretation |
| `repo` | Paths relative to the selected bundle's Git worktree root |
| `filesystem` | Absolute filesystem paths or paths relative to the invocation directory |

The base applies to every changed input in the invocation. Deleted paths can still identify
source dependencies. Repository mode requires an available repository context.
These CLI input modes do not reinterpret stored Markdown links: a leading `/` in an
ordinary authored concept link remains bundle-relative.

Explicit changed arguments do not consume an unrelated stdin pipe. Request stdin with `-`:

```sh
git diff --name-only HEAD~1 | okf affected knowledge --changed - --changed-base repo
printf '%s\n' policies/other.md | okf affected knowledge --changed policies/review.md --changed -
```

The stdin-only fallback also works when no changed arguments are supplied. Only one input
may consume stdin. `--depth` requires `--transitive`; use both for bounded cascading impact.

## Consume structured results

Use `--json` for newline-delimited JSON. Operation commands normally emit an
`operation-start`, their command-specific payloads, and an `operation-complete` record.
Schema and version discovery retain their own output formats.

The completion record distinguishes:

- `outcome`: whether execution succeeded, returned a partial read, or failed.
- `exit_code`: the declared process result, which must agree with the actual exit.
- `publication`: whether authored changes were published or require recovery.
- `completeness`: applicable examination, matching, delivery, expansion, and content bounds,
  together with read consistency.
- `reasons`: machine-readable reasons for limits or failure.

An exit code of zero does not establish exhaustive results. Check query summaries and
completion evidence before treating a result as proof of absence or an exact count.
A process can stop without producing its terminal record. Follow the
[result-handling guide](troubleshooting.md#interpret-a-json-operation)
when embedding the CLI.

Operation `outcome` is not concept `status`. Concept lifecycle retains the standard meanings
`draft`, `stable`, and `deprecated`, with absent status interpreted as `stable`. Keep local
workflow states in separate metadata such as `workflow_state`. Unfamiliar authored status
values remain readable and are preserved; they are not silently normalized.

## Check source health and accept reviewed baselines

`stale` examines fingerprint extensions and expiry. Filtering presentation and selecting a
failure policy are separate actions:

```sh
okf stale knowledge --only drifted,missing --summary --json
okf stale knowledge --fail-on-state drifted,missing,unavailable,unsupported,expired --json
```

| Observation | Meaning and next step |
| --- | --- |
| `drifted` | A supported fingerprint changed. Review the source and the concept's claims. |
| `unrecorded` | A source requests fingerprinting but has no recorded baseline. Review before establishing one. |
| `missing` | The check established that a local artifact is absent. Correct the source location or restore the artifact. |
| `unavailable` | Computation could not be completed, including malformed source requests or backend failures. Inspect the diagnostic. |
| `unsupported` | The declared fingerprint kind has no implementation. Choose a supported kind only if it represents the intended evidence. |
| `expired` | The concept's `stale_after` instant has passed. Review its currency separately from fingerprints. |
| Provenance only | A valid source has no fingerprint kind. Its presence makes no fingerprint-health claim. |

Backends without enough evidence to establish absence report unavailable conservatively.
Provenance-only sources are not a seventh failure state; summaries can count them separately.
`--only` does not suppress findings from the exit policy. Summary counts describe findings,
not necessarily the number of source files or concepts examined.

After reviewing a concept against its sources:

```sh
okf refresh notes/service knowledge
```

Single-concept refresh computes every required fingerprint before publishing any of them.
Missing, unavailable, unsupported, or malformed required sources prevent the update.
Valid kindless provenance is left unchanged. The existing `--fail-on skipped` policy can
also reject provenance skips before publication; `--fail-on never` cannot allow required
fingerprint failures through.

Refresh does not verify claims, change standard source modification timestamps, or clear
expiry. Document verification is a separate `verify` operation following an actual review.

## Preview and apply coordinated changes

Use a changeset for related edits, moves, creates, or removals. Creation accepts either a
complete Markdown document or a frontmatter object plus body:

```yaml
version: 1
operations:
  - op: create
    path: notes/review.md
    frontmatter:
      type: Note
      title: Review procedure
    body: |
      Describe the reviewed procedure here.
```

Save that document as `plan.yaml`, then preview it:

```sh
okf changeset plan knowledge --from @plan.yaml --json
```

Inspect the file changes and returned `base_digest`. Apply the same reviewed operation
document with that digest:

```sh
okf changeset apply knowledge --from @plan.yaml --expect <base-digest>
```

The `<base-digest>` text is a placeholder for the returned token, not a literal argument.
Complete Markdown and structured creation inputs cannot be supplied together for one create.
The final state is checked before publication; temporary intermediate states do not need to
satisfy an enabled ontology gate independently.

For read-then-write integrations, obtain a token without preparing a dummy mutation:

```sh
okf changeset digest knowledge --json
```

A token guards the covered bundle and interpretation inputs. It is not a persistent read
snapshot, proof of review, or approval of a different operation document. Plans, digests,
and previews do not publish authored content, but can create coordination state and need
permission to acquire the writer lock.

### Deliberate deletion

`rm --force` permits deletion despite the ordinary backlink safeguard. A changeset removal
can express the same narrow intention with `allow_dangling: true`. This permits missing
references caused by that removal, not unrelated errors or a bypass of an enabled ontology
gate. Review the affected referrers before using either override.

## Apply a local ontology write policy

An optional write gate prevents new or worsened ontology violations. It is disabled by
default and does not change format conformance. For an uncataloged bundle, configure:

```toml
[bundle_settings.default.write_policy]
ontology_gate = true
ontology_severity = "error"
blocking_threshold = "error"
```

For a registered bundle, use its settings key, such as
`[bundle_settings."acme.product".write_policy]`. Severity and threshold accept `info`, `warn`,
or `error`. The gate's defaults are error-level violations and an error-level blocking
threshold. Lint display settings, including `off`, do not weaken this write policy.

Supported authoring commands offer `--require-ontology-gate` to require the check even if
configuration leaves it disabled. There is no command-line override that weakens an enabled
gate. The gate needs an available ontology and sufficiently complete checking. Existing
unrelated violations can remain; new violations cannot be offset by fixing different ones.

Shared ontology changes can affect several bundles. A refusal identifies dependent bundles;
use `--check-dependent-bundles` on the ontology command to examine them explicitly:

```sh
okf ontology update Note knowledge --description "A maintained note" --check-dependent-bundles
```

The operation publishes the sidecar, not consumer concept files. Unavailable required
consumers block the update. Typed targets outside a consumer's examined bundle produce
`write-gate-incomplete`; dependent-sidecar examination does not broaden ordinary reference
traversal. This is a current capability limit, not an invalid-OKF verdict.

## Understand write guarantees

CLI authored writes use guarded, journaled publication. Multi-file fixes and generated
indexes prepare their write set before publication. Cooperating CLI writers share exclusion;
other filesystem editors are outside that protocol. Publication is atomic per file, not an
atomic multi-document view for readers.

`lint --fix` operates on one selected bundle. Run separate invocations for separate bundles;
there is no rollback across those invocations. Index generation replaces generated navigation
content, so use it only where that replacement is intended. Initialization preserves existing
authored files; a failed attempt can leave empty directories or coordination files.

After interruption, use the publication evidence and
[recovery guidance](troubleshooting.md#recover-an-interrupted-write).
Do not infer that a write was rolled back merely because output was interrupted.

These guarantees describe the CLI. The low-level library `save_ontology(path, ...)` primitive
has no selected-bundle policy context and is not a substitute for the guarded CLI path.

## Current boundaries

The current interface does not provide batch refresh, batch link resolution, map-value
wildcard selectors, persistent query-result handles, general immutable bundle snapshots,
cross-bundle content transactions, or a selective export pipeline. Use existing single-record
commands, supported graph queries, and explicit scope; do not rely on flags from older
planning material. Existing typed filters, projections, and nested list-reference selectors
remain available as described in the [structured metadata capability](../contributing/design/structured-metadata-capability.md).

Unknown types and metadata, optional families, and broken links remain consumable under
OKF v0.2. A local write policy, unsupported operation, or unavailable resource is reported
separately from `validate` conformance.
