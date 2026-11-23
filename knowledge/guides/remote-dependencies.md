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
description: >-
  Declare Git dependencies, review candidate revisions, and read accepted snapshots
  without implicit fetching.
---
# Consume remote bundles

A Git catalog registration describes an upstream bundle. A committed YAML lock records the accepted
revision. Ordinary reads resolve that revision from local storage. In this example an application
bundle needs a shared product definition. You need an existing application bundle in `knowledge/`
and read access to a Git repository containing a bundle at `bundles/products`. Substitute your
repository URL, branch, and bundle path in the catalog. Run each command from the directory
containing the catalog.

## Declare the dependency

**File: `okf-catalog.yaml`**

```yaml
catalog_version: 2
default_bundle: example.application
bundles:
  example.application:
    location: {type: directory, path: knowledge}
  acme.products:
    location:
      type: git
      url: ssh://git@code.example.org/shared-knowledge.git
      ref: refs/heads/main
      path: bundles/products
    access: read-only
    accepts:
      okf_spec: ["0.2"]
      manifest_versions: [1]
      manifest_required: false
```

Remote candidates must pass bundle-profile conformance and manifest/ontology validation. An
explicitly configured ontology must be contained in the remote bundle; acquisition does not read a
consumer-machine path named by upstream metadata.

One accepted revision per repository is supported in a catalog. Registrations sharing that
repository share the update unit; updating one reports the other affected members. Consumer
examination scope is separate and must be explicit.

## Plan, review, and accept

Resolve the branch to an exact candidate and save a reviewable plan. Include the application
bundle so the plan can examine its references.

```sh
okf catalog update --bundle-id acme.products --scope-bundle example.application --plan products-update.yaml
```

Open `products-update.yaml`. Review the candidate commit, compatibility, consumer scope, and
changes to concepts, ontology, manifests, instructions, and scripts. The first plan has no
previous accepted revision to compare. Planning can contact the upstream and leaves the
accepted lock unchanged.

After reviewing and accepting the candidate, apply that specific plan.

```sh
okf catalog update --apply products-update.yaml
```

The lock now records the reviewed commit. Apply checks for changes to the plan inputs and
uses the candidate identified during planning. Incomplete impact requires an explicit
`--allow-incomplete-impact` acknowledgment; compatibility, integrity, and stale-plan errors
still prevent application. Acceptance leaves source fingerprints, authored references and
verification history under the control of their own workflows.

Materialize the locked revision for reading.

```sh
okf catalog sync --bundle-id acme.products
```

The consumer can now read the local managed snapshot. Check it offline before using it as
evidence.

```sh
okf catalog sync --check --bundle-id acme.products
```

A successful check confirms local correspondence and integrity. Remote reachability and
credential validity require a separate online operation. Supply credentials through the
existing Git mechanism and keep secrets out of catalog URLs.

## Reference a bundle independently of its checkout path

If the upstream has a concept in `products/widget.md`, add an application note that links
to its catalog identity.

**File: `knowledge/notes/widget.md`**

```markdown
---
type: Note
title: Widget dependency
---
See the [product definition](okf://acme.products/products/widget.md).
```

Resolve the outgoing link with the upstream in examination scope.

```sh
okf links notes/widget --bundle-id example.application --scope-bundle acme.products --details --json
```

The target should identify `acme.products`, its widget concept, and the accepted version.
A logical URI uses a registered bundle ID and a bundle-relative path. The consumer catalog
and lock select the actual bytes. Ordinary Markdown viewers may require a relative or web
link. Existing paths and HTTPS URLs retain their usual meanings.

A source may also declare `bundle_ref.snapshot` when its claim requires a particular observation.
A conflict with the accepted lock remains visible; available content is not automatically an
acceptable substitute. Cross-bundle access still follows examination scope and never implicitly
acquires a dependency. Ordinary Markdown viewers may need exported relative or web links.

## Access, worktrees, and history

Managed Git dependencies are read-only to OKF mutations. Local overrides change location, not
accepted revision or access policy, and must match the accepted bundle content. Never reset or clean
an owner's checkout to make an override pass.

Immutable cache content can be shared across worktrees, while each worktree retains its own catalog
and lock. Read-only CLI checks do not isolate arbitrary scripts or shell redirection; stronger
protection requires filesystem permissions or a sandbox. Acquisition does not execute upstream
hooks, content filters, workflows, or scripts.

Historical inspection must identify the workspace catalog/lock revision separately from an
explicitly inspected remote revision. Retain the reported repository, commit, and path when opening
evidence. Historical reads do not fetch missing objects, substitute current files, or activate
historical instructions.

See [bundle customization](customize-a-bundle.md) for guidance and
[configuration](../reference/configuration.md) for catalog selection and overrides.
