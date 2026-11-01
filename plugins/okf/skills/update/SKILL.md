---
name: update
description: Reconcile OKF concepts after source or concept changes using lifecycle, source metadata, fingerprint drift, affected analysis, and git diff. Use for documentation sync or requested presentation formatting; refresh fingerprints only after review and route meaningful rewrites to re-verification.
---

# Update concepts after changes

## Shared operating protocol

Read and follow the [shared OKF protocol](../../references/bundle-context.md) before CLI-dependent
work. It defines compatibility checks, bundle context, scope, output evidence, validation profiles,
and authoring conventions. Use `references/cli.md` for exact command syntax as needed.

Prefer `OKF_BUNDLE` for a workflow. Fully qualified examples include `okf diff <git-ref> <bundle>`
and `okf affected <bundle> --changed <resource>`. Before using an unfamiliar command or flag, read
its entry in `references/cli.md`. If the installed version differs or rejects documented syntax, use
that command's `--help` output as the runtime authority.

## Detect and classify

Use explicit `--changed -` to combine a changed-target argument
with piped changed targets. Use `--changed-base repo` for repository-worktree
paths or `--changed-base filesystem` for explicit filesystem paths; the default
remains bundle-relative. `--depth` requires `--transitive`. For JSON output,
check the versioned operation completion record and actual process exit before
acting on a result. Display-only diagnostic filters do not change the checked
population or failure policy.

Use the relevant detectors and keep their meanings separate:

- `okf stale <bundle> --fail-on any` reports expiry and source findings: `unrecorded` means no
  baseline, `missing` means an established absent resource, `unavailable` means computation
  could not complete, `unsupported` means no implementation for the declared kind, and
  `drifted` means a recorded baseline changed. Without `--fail-on any`, findings still exit
  successfully unless an explicit state policy applies. Standard kindless provenance and
  top-level resources are not automatically fingerprinted.
- Standard source `last_modified`, credibility, and usage signals inform review but are not the same
  as local fingerprint drift.
- `okf affected <bundle> --changed <link-or-resource>` follows corrected body and standard source
  graph edges; add `--transitive --depth <N>` when needed.
- `okf diff <git-ref> <bundle>` reports concept-level change versus a Git baseline.

Triage findings as lifecycle expiry, standard source evidence change, tool fingerprint drift, or
actual concept-content change. A concept may have more than one.

For `revision-changed`, review the changed revision and authored snapshot expectations even when
content bytes match; do not assume content drift or refresh automatically. Other states follow the
shared source-health definitions.

## Reconcile

1. Resolve bundle identity and the requested consumer scope before affected analysis; use detailed
   semantic occurrences for nested relationships and retain unknown consumers outside scope. Inspect the concept with targeted `okf show` reads and the changed evidence. For a path-valued
   source, use `okf artifact resolve <resource> <bundle> --from <concept-id>` and bounded `okf
   artifact show`; use `okf show` when it resolves to a concept. Use `okf links <concept-id>
   <bundle> --details --json` when the concept's direct outbound relationships may have changed.
   Compare occurrence paths, kinds, inverse labels and attributes, not just deduplicated targets.
2. If meaning changed, supply an explicit edit operation: `okf edit <concept-id> <bundle> --set
   "description=Updated summary"` for a scalar, `okf edit <concept-id> <bundle> --set-body
   @/tmp/revised-body.md` for the whole Markdown body, or `okf edit <concept-id> <bundle>
   --set-section "Heading" @/tmp/section.md` for a section. Section edits take two values;
   body/section files contain prose, not frontmatter. A bare edit supplies no content change. The
   CLI updates existing `generated.at` and invalidates verification for meaningful edits; report the
   need for `review-verify`. For nested data use `okf edit <concept-id> <bundle> --set-yaml
   "details=@/tmp/details.yaml"`, `okf edit <concept-id> <bundle> --set-path "deadline.within=72"`,
   or `okf edit <concept-id> <bundle> --patch @/tmp/changes.yaml --dry-run`. Follow the ontology
   skill's [structured authoring example](../ontology/references/structured-authoring.md); review
   replacement versus nested-edit semantics and preserve provenance.

   To replace a source path or metadata, prefer one guarded `--patch` over separate remove/add
   edits. For example, after inspecting source index 0, use
   `[{"op":"test","path":"/sources/0/resource","value":"old.py"},{"op":"replace","path":"/sources/0/resource","value":"new.py"}]`.
   Test the full source mapping when other metadata must also match the reviewed baseline. This
   preserves unrelated source fields and refuses a changed target; overlapping `--add-source`
   or `--add-source-json` and `--remove-source` selectors are rejected. Preview with `--dry-run`.

   Meaningful edits remove active verification; no-op edits preserve it. Inspect `trust_tier`
   and valid verification events before bulk curation, preserve needed history separately, and
   report the resulting review work. Applied JSON edits include an `invalidate-verification`
   change when records are removed. `verification_current` compares the latest valid verification
   timestamp with `generated.at` (or the legacy `timestamp` when `generated` is absent): `null`
   means that comparison is unavailable, including when either timestamp is missing or invalid.
   It is not a verification-presence flag; a reviewed `trust_tier` alone does not establish currency.
3. After checking changed sources against the final content, run `okf refresh <concept-id> <bundle>
   --fail-on skipped` if local fingerprints are in use. This applies after a substantive rewrite as
   well as after confirming unchanged content. Never refresh an unreviewed source merely to clear
   drift; refresh operates on the concept's supported sources. A missing,
   unavailable, or unsupported required source blocks the whole single-concept refresh
   before publication. Inspect skipped/error results and keep unresolved sources in the
   report. Refresh updates the fingerprint extension only. It does
   not alter standard `sources[].last_modified`, regenerate content, change status, or cure an
   expired `stale_after`.
4. Change `stale_after` or lifecycle status only from evidence and an authorized lifecycle decision;
   an existing user instruction can supply that decision. These are distinct from refresh.
5. If a concept moved, use `okf mv <old-id> <new-id> <bundle>`. When an opaque artifact moves,
   update its declaring standard path field and preserve provenance; never execute it.
6. Re-run stale/affected checks, `okf validate <bundle> --profile bundle`, and advisory `okf lint <bundle> --fail-on
   never`. Regenerate indexes with `okf docs <bundle> --format index` only when needed and their
   bodies are generated or replacement is already authorized; preserve curated prose otherwise.
   Validate after index generation. Inspect lint findings despite its reporting-only exit; use
   `okf lint <bundle> --fail-on warn` for a source-health gate. Missing local sources are errors,
   and unrecorded fingerprints are warnings by default; neither is repaired by conformance validation.

For a phrase correction, use `okf edit <concept-id> <bundle> --replace "Old phrase" "New phrase"`;
exactly one literal body match is required. Use `--all` only when every occurrence should change.
Rename a uniquely matched heading with `okf edit <concept-id> <bundle> --rename-section
"Old heading" "New heading"`; its content and level are preserved. Failed matching leaves the
file unchanged. Use `okf show <concept-id> <bundle> --body` if a full body is needed for editing,
without splitting display headers or frontmatter delimiters.

## Source access

`references/` is optional; Markdown files are concepts and other files are artifacts. Keep declaring
context on artifact reads. For repository-relative fingerprints, external source access, or
truncation, use the artifact sections of `references/cli.md`. Existing authorization to inspect a
source counts; do not bypass bundle containment. Reading code does not authorize execution.

For an authorized artifact copy or authored schema, use `okf artifact put
contracts/x/references/schema.json @/tmp/schema.json <bundle>`. The destination is bundle-relative;
the input file is relative to cwd. Creation is the default; use `--replace` only for an intended
replacement. Writes preserve recorded fingerprints, so review citing concepts before refreshing
them. This command does not execute artifacts or grant permission to copy external material.

Report changes, validation, and remaining evidence gaps. Include fingerprint-only refreshes,
unchanged expiry, and verification consequences when relevant to this update.

## Scoped reconciliation

Use explicit examination scope for cross-bundle affected analysis. Record which consumers remain
outside scope; a clean result is not a global backlink inventory. Check both ordinary source
resources and optional `bundle_ref` metadata, retaining known mismatches rather than redirecting
resources. Review a labeled current candidate before intentionally changing an expectation;
retrieval alone never accepts it. Snapshot resolution and fingerprint drift are separate findings.

Re-run nested metadata and index-coverage lint after metadata or layout changes. Lint and query
operations do not rewrite curated indexes. Use the declared selectors to locate affected records;
use object paths or concrete JSON Pointer patches for writes, rather than list query selectors. Review the combined repository
diff when changes span bundles; there is no automatic cross-bundle rename transaction.

## Presentation-only formatting

For requested quoting cleanup or prose wrapping, use the shared [formatting guide](../../references/formatting.md).
Preview with `okf format --bundle <bundle> --dry-run`; apply within the requested scope, then use
`okf format --bundle <bundle> --check` for drift. Use the optional concept ID to limit the change.
Formatting retains verification and generation metadata; meaningful content edits still follow
this skill's review workflow. Do not enable Markdown wrapping or format unrelated concepts merely
because a source or document changed.
