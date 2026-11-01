---
type: Reference
audience: contributor
title: Remote bundles and locked catalog dependencies
description: Design of Git dependency acquisition, accepted revisions, logical references, scoped impact review, and managed read-only access.
---
# Remote bundles and locked catalog dependencies

Remote registrations let a consumer read another team's bundles at a reviewed Git revision without
encoding checkout locations into authored references. The catalog declares dependencies; a lock
records accepted commits; local immutable snapshots supply their bytes. Acquisition and acceptance
are explicit operations. Ordinary knowledge reads remain offline.

This design extends [local catalogs](multi-bundle-capability.md) and shares configuration ownership
with [bundle manifests](bundle-manifest.md). It addresses the acquisition and reproducibility needs
in [issue 7](https://github.com/briareus-io/okf/issues/7). The operational guide is
[Consume remote bundles](../../guides/remote-dependencies.md).

## Ownership and invariants

| Artifact | Owner and responsibility |
| --- | --- |
| `okf-catalog.yaml` | Consumer declarations, registered IDs, locations, default selection, compatibility requirements, access policy |
| `okf-catalog.lock` | Consumer's accepted exact repository commits and observed bundle compatibility |
| Upstream `okf-bundle.yaml` | Bundle description, settings, guidance, workflows; no consumer permission grants |
| Local override file | Alternate checkout locations, preserving accepted revision and access policy |
| Managed cache | Local objects and snapshots, outside authored bundle content |
| Update plan/report | Review evidence and acceptance outcome, not executable instructions |

A catalog accepts one commit per conservatively identified repository. Several bundle subpaths may
share that snapshot. Updating one member updates the repository group and reports the expanded
selection. Repository grouping does not infer equivalence between different SSH/HTTPS spellings,
and does not authorize examination of unrelated consumers.

Bundle IDs belong to the consumer catalog, not intrinsic upstream metadata. Matching a commit or
digest proves correspondence to selected bytes, not factual accuracy, authorship, or human review.
Accepting a dependency revision does not verify concepts or reconcile per-source expectations.

## Registration and lock schemas

Catalog version 2 combines defaults and Git registrations with the manifest design. Version-1
directory catalogs remain supported. Catalog, lock, manifest, and CLI versions have separate
compatibility meanings.

```yaml
catalog_version: 2
default_bundle: example.application
bundles:
  example.application:
    location: {type: directory, path: knowledge}
  acme.products:
    location:
      type: git
      url: https://github.com/acme/shared-knowledge.git
      ref: refs/heads/main
      path: bundles/products
    access: read-only
    accepts:
      okf_spec: ["0.2"]
      manifest_versions: [1]
      manifest_required: false
```

`url` identifies the repository, `ref` declares update intent, and `path` identifies the bundle
within the repository. Use fully qualified branch/tag refs or a full commit ID. HTTPS and SSH are
supported acquisition transports; local authored dependencies use directory registrations. Git
inputs are passed as arguments, not shell interpolation. Credentials do not belong in catalog
URLs, locks, plans, or reports.

Git registrations are read-only. The first release does not support publishing upstream or accepting
multiple revisions of the same repository in one catalog. Members of a repository group must agree
on update intent and accepted revision.

Compatibility policy uses explicit accepted-version sets. Omitted sets permit versions supported
by the running CLI. A missing manifest is allowed unless required. An undeclared spec version is
reported as undeclared, not fabricated to satisfy an explicit accepted set. Candidate bundles must
pass bundle-profile conformance and manifest/ontology validation. An upstream-configured ontology
must remain inside the selected bundle; acquisition does not read consumer-machine paths named by
upstream metadata.

The committed YAML lock uses `lock_version: 1`, a dependency-declaration digest, repository records,
and per-bundle records. Repository records contain URL, full commit, and Git object format. Bundle
records map catalog IDs to repository keys/subpaths and observed spec/manifest version evidence.
Repository keys are lock-local identifiers. SHA-1 and SHA-256 Git object formats are distinguished.
Observed metadata is rechecked against materialized content, not trusted merely because it appears
in the lock.

The declaration digest covers dependency identity, update intent, path, access, and compatibility,
excluding unrelated presentation changes. Changes to that contract make the existing lock stale
and require a registration-change plan rather than silent reinterpretation.

## Lifecycle

```text
okf catalog update --bundle-id acme.products --scope-bundle example.application --plan /tmp/products-update.yaml
okf catalog update --apply /tmp/products-update.yaml
okf catalog sync --bundle-id acme.products
okf catalog sync --check --bundle-id acme.products
```

| Operation | Acquisition | Accepted lock | Local materialization |
| --- | --- | --- | --- |
| Knowledge read, context, resolve, lint | None | Unchanged | Reads available content |
| Update plan | May resolve/fetch selected update intent | Unchanged | May stage candidate objects/snapshots |
| Apply reviewed plan | No ref resolution or fetching | Replaced with reviewed candidate | Requires verified candidate content |
| Sync | May fetch exact accepted commits | Unchanged | Creates missing accepted snapshots |
| Sync check | None | Unchanged | Offline verification only |
| Sync rebuild | No branch update | Unchanged | Quarantines corruption and rebuilds from available objects |

An update without `--apply` only plans. A missing lock requires initial planning and acceptance;
sync cannot invent accepted state from a moving branch. Without a dependency selector, lifecycle
commands operate on declared Git registrations and report their selection. An explicit ID selects
its repository group. Consumer impact scope is independently selected with `--scope-bundle` or
`--catalog-scope`; it is not inferred from dependency ownership.

Explicit HTTP artifact fetching, where supported, is separate. `artifact show --fetch` cannot
acquire an `okf://` dependency; the user must select catalog acquisition instead.

## Update plans and impact evidence

| Plan kind | Baseline | Evidence |
| --- | --- | --- |
| `initial` | No accepted registration/lock entry | Candidate compatibility and explicitly scoped consumer checks; no old-tree comparison |
| `update` | Accepted registration and commit | Old/new endpoint trees, compatibility, scoped impact, input digests |
| `registration-change` | Changed catalog/lock relationship | Identity/path rebinding, added/removed registrations, and resulting impact |

Plans record exact old/new commits, selected registrations, catalog and old-lock digests, candidate
lock data, consumer baselines, changed files, examination completeness, and a plan digest. A hash
detects changes; it is not approval or trust. Saved plans contain data, not shell commands or
arbitrary write destinations. JSON identifies the envelope as `catalog-update-plan` and retains
classification separately as `plan_kind`.

Impact analysis compares concepts, ontology, manifests, instructions/scripts, and other support
files. It reports added/removed/changed content, scoped links, before/after consumer lint, affected
concepts, and snapshot/fingerprint evidence. Both sides use the same explicitly examined consumer
baseline and the respective dependency snapshots. Effective checkout overrides and registered
consumer identity must be preserved. Unexamined consumers and incomplete evidence remain visible.

Commit history is separate from endpoint-tree comparison. A branch rewrite need not prevent a
comparison of available trees; unavailable required old content does. Rename evidence must not be
invented from titles. An initial plan's absent old snapshot is not an unavailable required baseline.

Apply revalidates plan structure, declaration and old-lock digests, candidate objects, compatibility,
consumer baselines, and report evidence. The accepted commit is exactly the reviewed candidate even
if the branch advances after planning. Changed inputs require a new plan. Unavailable staged
content is an error, not permission to resolve a newer tip.

The following failures always block acceptance: unsupported/invalid schemas, incompatible selected
bundles, object-integrity failures, invalid paths, access-policy violations, stale input digests, and
unavailable required endpoint trees. `--allow-incomplete-impact` may acknowledge reported impact or
history gaps only; it cannot waive those failures. A plan without consumer examination requires
that explicit acknowledgment. The acceptance report records the omitted scope/reasons and does
not label incomplete checks as passed.

Acceptance does not refresh source fingerprints, rewrite references, alter verification history,
execute workflows, or accept a conflicting per-source snapshot. Those are separately selected
reconciliation tasks. Reports are local review artifacts with exact selections, digests,
completeness, and outcome; no automatic pruning or report-upload behavior is implied.

## Object acquisition and snapshot publication

Explicit acquisition uses trusted Git credential mechanisms while controlling Git invocation so
upstream hooks, filters, scripts, submodule updates, and workflows do not execute. Raw object reads
avoid checkout filters. Missing or inaccessible pinned objects are errors; acquisition never falls
back to a branch tip.

Materialization records repository membership and creates ordinary Git blobs with their paths and
executable modes. Its identity includes repository, object format, commit, and materialization
profile. Gitlinks, symlinks, and recognized LFS pointers are unsupported entries in this profile;
their metadata remains explicit rather than being replaced with misleading empty directories or
hydrated content. The snapshot is complete for supported entries, not a complete checkout of every
Git entry kind.

A selected bundle containing unsupported entries fails compatibility. Unsupported entries outside
selected bundles do not block unrelated acquisition; accessing one or registering its subpath
requires explicit validation. Membership checks normalize root and dot-prefixed paths. Equal/nested
registered roots are rejected, and symlink traversal cannot escape a selected root.

Before publication, verify bytes and modes against Git objects and metadata. Stage snapshots and
publish them by same-filesystem atomic rename into immutable identity-keyed directories. Never
rewrite accepted contents in place. Publish candidate snapshots before atomically replacing the
accepted lock: interruption may leave unreferenced cache content, not a partially written lock.
Multi-repository candidates must validate all members before the one lock replacement.

Process locks and compare-and-swap checks prevent concurrent publication from overwriting newer
accepted state. Lock inodes remain stable and OS-held locks are released on process exit, avoiding
manual stale-lock-file removal. Cache recovery is explicit (`catalog sync --rebuild`), quarantines
corrupt content for inspection, and does not overwrite an owner's checkout. Cleanup must respect
other worktrees and active readers; automatic garbage collection is outside the first release.

## Cache and local overrides

The cache is per-user, outside bundle roots, and reusable across worktrees. `OKF_CACHE_DIR` can
select a local cache root; otherwise the CLI uses its user-cache location. Paths are machine
configuration, not committed dependency identity. Private repository content is not a cross-user
cache or a public CI artifact.

Each worktree uses its own working-tree catalog and lock by default, normally committed for
reproducibility. Sharing bytes does not share accepted state. Output identifies repository, commit,
bundle path, availability, and correspondence instead of exposing cache paths as authored identity.

A Git-registration override must match the accepted commit and selected bundle's tracked bytes,
without extra ordinary/support files in that bundle. An override changes location only. A dirty or
mismatched checkout is not a successful locked dependency, and OKF does not reset, clean, or fetch
inside it automatically. Owner-side editing belongs in an independent directory registration.

A workspace-local cache is easier to inspect and delete but duplicates snapshots and can enter a
root-level bundle accidentally. Per-user storage favors sharing and keeps operational files out of
knowledge discovery; the explicit cache-root option supports CI and constrained environments.

## Managed read-only enforcement

The consumer catalog owns access policy. Upstream guidance cannot weaken it. Protected writes are
checked at shared mutation boundaries, including concept edits, verification/refresh, artifact and
ontology writes, lint fixes, doctor repairs, CLI-owned output destinations, and changeset recovery
or publication. Canonical roots preserve protection across IDs, explicit paths, symlink aliases,
and active-catalog overrides.

Managed snapshots carry origin/protection metadata outside upstream content. Direct cache-path
access remains protected even without a consumer catalog. The entire managed repository snapshot
is read-only, not just registered subdirectories. Cache management is a narrowly scoped exception
for acquisition/recovery, not general authoring permission.

The guarantee covers OKF-controlled writes and corruption detection. Shell redirection is opened
by the shell; arbitrary tools and the owning user require filesystem permissions or a sandbox for
stronger isolation. Disabling Git push does not prevent local mutation. A user-owned override
checkout outside its consumer context has no universal catalog policy. The CLI does not claim
operating-system-wide agent isolation.

## Logical references and source evidence

```markdown
See the [product definition](okf://acme.products/products/widget.md).
```

`okf://<bundle-id>/<bundle-relative-path>` names a registered bundle and a path inside it. The
authority is a bundle ID, not a catalog ID or DNS host. IDs remain case-sensitive. Decode paths
once, reject credentials/ports/query strings and traversal or ambiguous encoded separators, and
preserve optional fragments without changing file identity. Authored examples use explicit `.md`
paths; other resources resolve as artifacts with bounded explicit reads.

Resolve the URI through the selected consumer catalog and accepted snapshot. Support body links,
ontology-declared reference fields, and source resources; arbitrary strings do not create graph
edges. Preserve the authored URI, origin, examination scope, and distinct unknown/unavailable/
out-of-scope/missing/mismatch outcomes. The URI does not authorize acquisition or catalog-wide
examination.

CLI targets such as `acme.products:/products/widget` remain a separate selection notation.
Ordinary relative paths and HTTPS URLs retain meaning and are not heuristically rewritten into
cache references. Logical URIs are location-independent within a compatible catalog, but ordinary
Markdown readers may not navigate them. General export to ordinary links is a separate capability;
known target layouts can be handled explicitly without claiming unsupported equivalence.

A logical URI follows the selected lock; it is not independently pinned. A provenance source may
also declare `bundle_ref` with the same ID/path and a required snapshot. Disagreement remains a
mismatch or separately labeled historical candidate, never a silent substitute.

The optional `bundle-resource` fingerprint kind resolves a logical URI before hashing raw file
bytes with SHA-256. It records bundle/path and immutable repository/commit evidence when available;
mutable directory targets do not acquire fabricated commits. Fragments do not change hashed bytes.
Unsupported URI/source-kind combinations are diagnosed rather than silently reinterpreted or sent
to an HTTP client. Kind-less sources remain valid provenance without fingerprint opt-in.

`stale` distinguishes resolution, required snapshot satisfaction, content drift, and revision-only
change. A new lock commit with identical file bytes changes revision identity without changing the
content digest. `refresh` writes an authorized consumer baseline after reconciliation; it neither
fetches nor replaces conflicting authored snapshot requirements.

## Historical context

| Selector | Interpretation |
| --- | --- |
| `--workspace-revision <ref>` | Historical consumer catalog/lock and in-repository content; remote commits follow that lock |
| `--bundle-revision <id>=<commit>` | Explicit inspection of locally available immutable bundle content, labeled when it diverges from the lock |
| Legacy `--revision` | Supported historical command behavior; ambiguous mixed-repository use is rejected |

Selections for members of one repository must agree. Merely sharing repository objects does not
expand examination scope. External directory registrations remain identified as working-tree
context unless explicitly selected at a snapshot. A historical catalog alone does not make every
external input immutable.

A common snapshot reader addresses repository object IDs and commit/path, independent of `.git`
inside extracted caches. Historical concept IDs remain relative to the selected bundle subpath;
sibling bundles must not leak into that identity. Show, graph, ontology, context, resource reads,
diff, and update analysis use compatible snapshot identities. Missing local objects never trigger
lazy network fetches during inspection.

The consumer catalog anchors namespace and policy throughout the read. Upstream `okf.toml` or
`okf-catalog.yaml` files are not activated by entering a cached path. Direct cache inspection can
report origin/protection but cannot invent a consumer identity; explicit `--catalog` supplies that
context. Historical instructions are inspected content, not automatically active procedures.

## CI and verification boundaries

CI supplies read-only Git credentials, commits declarations and lock, explicitly syncs accepted
objects, then runs offline sync check and scoped validation/lint. Noninteractive execution does
not wait for credential prompts. An inaccessible private object may be indistinguishable from a
missing one; diagnostics must not claim certainty the server does not provide.

`sync --check` verifies local correspondence, integrity, accepted versions, and required paths. It
cannot prove remote reachability or credential validity. Core document validation, manifest
validation, dependency availability, and advisory lint remain distinct checks. NDJSON envelopes
and completeness evidence are retained across lifecycle and inspection commands.

Verification covers initial acquisition, exact-pin updates, branch movement after planning,
repository groups, registration changes, stale consumer/catalog/lock inputs, dirty caches,
overrides, root-level bundles, unsupported entries, interrupted publication, protected destinations,
logical scope, historical subpaths, and source snapshot conflicts. It must also verify negative
boundaries: no implicit fetch, no instruction execution, no lock acceptance during planning, and no
fingerprint refresh as an update side effect.

Multiple accepted revisions per repository, writable remote registrations, upstream publication,
LFS/submodule hydration, signature-based publisher trust, automatic garbage collection, development
mode for mismatched overrides, generic link export, and managed OS execution isolation are deferred.
