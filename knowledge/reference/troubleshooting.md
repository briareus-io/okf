---
type: Reference
audience: user
title: Checking results and recovering writes
description: Choose checks, interpret incomplete results, and recover interrupted writes.
---
# Checking results and recovering writes

OKF distinguishes format conformance, advisory quality checks, source health, and publication
outcomes. Choose the check that answers your question instead of treating every successful
command as proof that the bundle is complete, current, or verified.

This guide covers the 0.6 CLI contracts. See [consistent command behavior](command-contracts.md)
for capability examples and the [CLI reference](../commands/index.md) for exact options.

## Choose the right check

| Question | Command | What a clean result establishes |
| --- | --- | --- |
| Do knowledge documents satisfy the bundle tooling profile? | `validate --profile bundle` | Examined knowledge documents conform, excluding root support Markdown. |
| Does this directory satisfy the core format boundary? | `validate --profile core` | Physical Markdown files satisfy core rules, including files an older reader may see. |
| Is the bundle manifest usable? | `bundle validate` | Supported schema, declarations, and required resources pass validation. |
| Are accepted remote dependencies available locally? | `catalog sync --check` | Local snapshots match the accepted lock; this is offline, not a network/authentication check. |
| Does the bundle satisfy our advisory conventions? | `lint` | Findings are interpreted under configured rules and the selected failure threshold. |
| Have recorded source fingerprints changed, or has review expiry passed? | `stale` | The reported source checks and expiry observations; kindless provenance is not fingerprinted. |
| What depends on changes I already know about? | `affected` | Dependents found within the command's examination and traversal scope. |
| What changed relative to a Git baseline? | `diff` | Concept differences for the selected comparison. |
| Does an existing bundle need compatibility repairs? | `doctor` | Compatibility findings and available safe repairs; read findings before applying repairs. |
| Can this coordinated write proceed? | `changeset plan` | A prepared final state and baseline, not a guarantee that the baseline will remain unchanged. |

For example:

```sh
okf bundle validate knowledge
okf validate knowledge --profile bundle
okf lint knowledge --fail-on error --json
okf stale knowledge --fail-on-state drifted,missing,unavailable,unsupported,expired --json
```

Validation does not require optional metadata or a local ontology. It does not prove source
truth, claim verification, or absence of broken links. Lint rules and source-health policies
can fail independently of format conformance.

## Interpret a JSON operation

Read `--json` output as NDJSON, one object per line. Normal operation streams contain:

1. An `operation-start` record naming the command and stream version.
2. Command-specific payloads, such as concepts, guidance, findings, scope, query summaries, or file diffs.
3. An `operation-complete` record with outcome, exit code, publication, and completeness evidence.

Discover the contract through `okf schema`. Do not assume that the first or last payload is
a concept, or that every command emits identical payload records. Schema/version discovery
retain their separate formats. Argument parsing errors can occur before a stream starts.

For an automation that needs exhaustive results:

- Require a supported stream contract and its mandatory records.
- Read payloads through the terminal record; EOF alone is not completion evidence.
- Check the terminal `exit_code` against the actual process exit. A failed output flush can
  turn an otherwise successful operation into a process failure.
- Require complete examination and matching over the intended scope before using absence,
  exact totals, or global ordering. Inspect query summaries as well as the terminal record.
- Check delivery, content, and related-record expansion separately when those dimensions
  matter to the task. Preserve omission and unavailable-target evidence.

A caller requesting a brief excerpt may accept partial content. A caller deciding whether
something exists may not accept an incomplete scan. Choose acceptance criteria for the
actual task rather than rejecting every bounded response or accepting every exit zero.

| Exit code | Interpretation |
| --- | --- |
| `0` | Success under the selected command/failure policy; a bounded read may still be partial. |
| `1` | A check or explicit policy failed, or an operation such as resolution was unsuccessful. |
| `2` | Usage or precondition problem, including a conflicting baseline. |
| `3` | Environment, I/O, or YAML-read failure. |
| `4` | Internal error. |

`outcome` belongs to the operation. It must not be copied into a concept's lifecycle `status`.
A source-state diagnostic likewise describes a check, not a new concept status.

## Distinguish limits from empty results

A page can be fully delivered even when more matches exist. Conversely, a scan may finish
its output while examining only part of the requested scope. These cases have different
consequences:

| Observation | Appropriate response |
| --- | --- |
| Scan budget reached | Narrow the question, increase `--scan-limit`, or deliberately use `--full-scan`; then recheck completeness. |
| Requested bundle unavailable | Restore that bundle or explicitly narrow the task; do not claim catalog-wide absence. |
| Related targets or content truncated | Request a smaller expansion or a targeted read of the omitted material. |
| No matching concepts after a complete scan | Conclude no matches for that query and scope, not that no related knowledge exists anywhere. |
| Source backend unavailable | Resolve the backend/resource problem; do not refresh a baseline to hide the finding. |
| Write gate incomplete | Correct the unsupported constraint or examination problem; do not interpret it as a passing gate. |

See [performance and bounded reads](../guides/bounded-reads.md) for choosing limits without
losing the evidence needed by your application.

## Diagnose manifest and dependency problems

| Symptom | Next action |
| --- | --- |
| Context fails on a malformed manifest | Run `bundle validate` or doctor and inspect the exact file. Repair the configuration before dependent work; do not silently substitute defaults. |
| Core validation flags support instructions | Check whether bundle-profile validation was intended. Do not add concept metadata to support files solely to silence core validation. |
| Instruction-file change is not reflected in an agent task | Reload context and compare `context_digest`, not only the manifest digest. |
| Dependency has no accepted lock | Create and review an initial update plan; sync never invents a branch-tip lock. |
| Accepted snapshot is absent locally | Explicitly run `catalog sync` within the requested dependency scope. Ordinary reads do not acquire it. |
| Managed cache content differs from its commit | Preserve the diagnostic; use `catalog sync --rebuild` for explicit quarantine and offline reconstruction from available objects. Do not edit the snapshot or reset an owner's override checkout. |
| Update plan is stale | Replan against current catalog, lock, and examined consumer content. |
| Impact was not fully examined | Inspect the reported gap; an explicit `--allow-incomplete-impact` acceptance does not waive integrity, compatibility, or stale-plan failures. |
| Write targets a managed dependency | Edit an owner-side working checkout instead; overrides do not weaken the consumer's access policy. |

See [remote dependency maintenance](../guides/remote-dependencies.md) for the acquisition lifecycle
and [configuration migration](migrations/0.6.0.md) for TOML plans and backups. Ordinary doctor diagnosis
is read-only, safe document repairs are explicit, and configuration migration has separate plan/apply
side effects. A pending configuration-migration backup is not a changeset journal.

## Recover an interrupted write

Use terminal publication evidence when it is available:

| Publication value | Meaning | Next action |
| --- | --- | --- |
| `not-applicable` | This command has no authored publication outcome. | Evaluate its read/check evidence. |
| `not-published` | The attempt left no authored changes, including successful previews or no-ops. | Correct the cause and obtain a fresh plan when retrying. |
| `committed` | The operation passed its commit point. | Do not repeat an append or verification event merely because later output handling failed. |
| `recovery-required` | A pending journal needs guarded recovery; committed cleanup may also need recovery. | Inspect `commit_reached` if present, then recover. |
| No terminal record | Publication outcome is unknown. | Inspect/recover pending state and reconcile actual content before retrying. |

Run recovery against the affected bundle:

```sh
okf changeset recover knowledge --json
```

Recovery can report rollback, committed cleanup, or no pending transaction. It refuses
unsafe recovery when files no longer match the recorded expectations. Preserve the journal
and investigate intervening edits; do not delete it to force a retry. A shared-sidecar journal
refers to the recorded target rather than permission to redirect recovery to another file.

No pending transaction does not prove that an earlier attempt never committed: it may have
committed and completed cleanup before output was lost. Inspect the expected post-state.
The CLI does not provide a persistent exactly-once receipt store. Appends and verification
events must not be blindly retried after an ambiguous outcome.

## Handle coordination and permission failures

Plans, digests, and mutation previews do not publish authored content, but they acquire
writer exclusion and may create coordination or temporary staging files. They can therefore
fail on storage that ordinary queries can read. Use a writable checkout when you need a
guarded baseline; do not treat an unguarded read as a substitute token.

Another cooperating writer can make the bundle temporarily busy. Wait for it to finish,
then re-read and replan as necessary. A stale baseline is a conflict to resolve, not a reason
to omit `--expect` from an already reviewed read-then-write workflow.

Ordinary queries do not hold these writer locks. Neither journaled publication nor a baseline
token makes multi-document working-tree reads atomic. External editors do not participate in
CLI writer exclusion. Use supported immutable revision reads where appropriate, and avoid
concurrent external edits when the workflow requires a stable working tree.
