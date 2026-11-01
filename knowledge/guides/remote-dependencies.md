---
type: Guide
audience: user
uses:
- /commands/catalog
- /commands/catalog-update
- /commands/catalog-sync
- /commands/resolve
- /commands/links
title: Consume remote bundles
description: Declare Git dependencies, review candidate revisions, and read accepted snapshots without implicit fetching.
---
# Consume remote bundles

A Git catalog registration describes an upstream bundle. A committed YAML lock records the
accepted revision. Ordinary reads resolve available accepted content; they never fetch or follow a
branch to newer content. The consumer catalog owns bundle IDs and access policy.

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

Remote candidates must pass bundle-profile conformance and manifest/ontology validation.
An explicitly configured ontology must be contained in the remote bundle; acquisition does not
read a consumer-machine path named by upstream metadata.

One accepted revision per repository is supported in a catalog. Registrations sharing that
repository share the update unit; updating one reports the other affected members. Consumer
examination scope is separate and must be explicit.

## Plan, review, and accept

```text
okf catalog update --bundle-id acme.products --scope-bundle example.application --plan /tmp/products-update.yaml
okf catalog update --apply /tmp/products-update.yaml
okf catalog sync --bundle-id acme.products
okf catalog sync --check --bundle-id acme.products
```

Planning may access the upstream to obtain an exact candidate; it does not change the accepted
lock. Review changed concepts, ontology, manifests, instruction files, scripts, compatibility, and
examined consumer scope. Apply accepts the reviewed commit without re-resolving the branch.
Changed plan inputs or unavailable required evidence prevent application. A plan without consumer
examination reports incomplete impact; applying it requires an explicit
`--allow-incomplete-impact` acknowledgment. This cannot bypass compatibility, integrity, or stale-plan
failures. An initial acquisition
has no old snapshot; this differs from a failed comparison against a required old revision.

Sync materializes the accepted commit. A missing lock requires initial planning and acceptance,
not an implicit branch-tip choice. Offline `sync --check` checks local correspondence and integrity;
it cannot establish credential validity or remote reachability. Authentication remains with the
existing Git credential mechanism, and credentials must not appear in catalog URLs or reports.

Acceptance does not rewrite references, refresh fingerprints, verify concepts, or execute upstream
workflows. Reconcile affected local knowledge through separately selected authoring work.

## Reference a bundle independently of its checkout path

```markdown
See the [product definition](okf://acme.products/products/widget.md).
```

The authority is a registered bundle ID, and the remaining path is relative to its bundle root.
The selected catalog and lock choose actual bytes. The URI is not globally resolvable or
independently pinned. Ordinary relative paths and HTTPS URLs retain their meanings; no heuristic
rewrites them into cache paths.

A source may also declare `bundle_ref.snapshot` when its claim requires a particular observation.
A conflict with the accepted lock remains visible; available content is not automatically an
acceptable substitute. Cross-bundle access still follows examination scope and never implicitly
acquires a dependency. Ordinary Markdown viewers may need exported relative or web links.

## Access, worktrees, and history

Managed Git dependencies are read-only to OKF mutations. Local overrides change location, not
accepted revision or access policy, and must match the accepted bundle content. Never reset or
clean an owner's checkout to make an override pass.

Immutable cache content can be shared across worktrees, while each worktree retains its own
catalog and lock. Read-only CLI checks do not isolate arbitrary scripts or shell redirection;
stronger protection requires filesystem permissions or a sandbox. Acquisition does not execute
upstream hooks, content filters, workflows, or scripts.

Historical inspection must identify the workspace catalog/lock revision separately from an
explicitly inspected remote revision. Retain the reported repository, commit, and path when
opening evidence. Historical reads do not fetch missing objects, substitute current files, or
activate historical instructions.

See [bundle customization](customize-a-bundle.md) for guidance and
[configuration](../reference/configuration.md) for catalog selection and overrides.
