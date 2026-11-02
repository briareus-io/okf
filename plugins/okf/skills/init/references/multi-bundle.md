# Set up a local multi-bundle catalog

Use separate bundles when ownership, update/release cycles, or distribution boundaries differ.
Use ordinary subdirectories when those boundaries are shared. Choose stable, namespaced IDs from
the user's domain; reuse existing IDs for the same registered material. Follow the
[shared operating protocol](../../../references/bundle-context.md) before authoring.

A catalog registers available directories and supplies workspace selection. Portable settings
belong in each bundle's `okf-bundle.yaml`. Registration does not clone or fetch; Git dependency
acquisition is a separate, explicitly requested catalog workflow.

## Catalog IDs and authored links

| Context | Reference form |
| --- | --- |
| CLI graph target, for example `backlinks` | `acme.finance:/policies/margin-standard` |
| Markdown body or declared metadata reference | Ordinary path, or `okf://acme.finance/policies/margin-standard.md` |
| Standard source entry | `resource` identifying the evidence, optionally paired with matching `bundle_ref.id` and bundle-root-relative `bundle_ref.path` |

Qualified CLI targets are not authored link syntax. A leading `/` in an ordinary link starts at
the declaring bundle's root, not another catalog entry. Relative links depend on checkout layout;
logical URIs resolve through the selected catalog and accepted lock without independently pinning
content. Include the target with `--scope-bundle acme.finance` when examination requires it.
Registration alone does not expand scope.

## Bundles in one workspace

This example has product material referencing a finance policy. Run commands from the workspace
root. Initialize only new roots; preserve existing configuration and concepts when adapting it:

```sh
okf init knowledge/product --no-ontology
okf init knowledge/finance --no-ontology
```

Create or merge the following YAML files using narrow configuration edits; there is no inferred
catalog-authoring command. Preserve unrelated entries and inspect the combined diff. Registered
roots must not be equal or nested. Load context after configuration is available and before
further concept authoring.

## `okf-catalog.yaml`

```yaml
catalog_version: 2
default_bundle: acme.product
bundles:
  acme.product:
    location: {type: directory, path: knowledge/product}
  acme.finance:
    location: {type: directory, path: knowledge/finance}
```

## `knowledge/product/okf-bundle.yaml`

```yaml
manifest_version: 1
settings:
  query:
    scan_limit: 1000
  views:
    compact:
      columns: ["$id", title, description]
  facets:
    fields: [type, "tags[]"]
    max_distinct_values: 100
```

Catalog locations resolve from the catalog file; manifest paths resolve from the bundle root.
Each bundle may use its own ontology. If a nondefault sidecar is needed, set
`settings.ontology: tooling/product-ontology.yaml` in that bundle's manifest after creating the
readable file. A manifest also works without a catalog when the bundle is selected by path.

## `knowledge/finance/policies/margin-standard.md`

```markdown
---
type: Policy
title: Margin standard
description: Defines the illustrative margin calculation.
---
Margin is revenue less direct costs in this example.
```

## `knowledge/product/metrics/revenue.md`

```markdown
---
type: Metric
title: Revenue reporting
description: Uses the finance margin definition.
sources:
  - id: margin-standard
    resource: ../../finance/policies/margin-standard.md
    bundle_ref:
      id: acme.finance
      path: policies/margin-standard.md
---
Use the [margin standard](../../finance/policies/margin-standard.md) for margin reporting.[^margin-standard]

[^margin-standard]: Finance's margin definition.
```

These complete concept files show the result. Normally use `okf add` with a body file and
`okf edit --add-source-json` for the source mapping. Here `resource` is document-relative and
`bundle_ref.path` is relative to the target bundle root; both identify the same material.
`sources[].id` joins claim footnotes and is distinct from the bundle ID.

Snapshots are optional; do not invent them. A live relative path or logical URI does not become
independently pinned by an accompanying snapshot expectation. Preserve exact source evidence when
it is known, and report mismatches separately from available current candidates.

## Inspect and check

```sh
okf catalog --json
okf bundle context --bundle-id acme.product --scope-bundle acme.finance --operation add --operation edit --operation "bundle validate" --operation validate --operation links --operation resolve --operation backlinks --operation lint --json
okf bundle validate --bundle-id acme.product
okf bundle validate --bundle-id acme.finance
okf validate --bundle-id acme.product --profile bundle
okf validate --bundle-id acme.finance --profile bundle
okf links metrics/revenue --bundle-id acme.product --scope-bundle acme.finance --json
okf resolve ../../finance/policies/margin-standard.md --from metrics/revenue --bundle-id acme.product --scope-bundle acme.finance --json
okf backlinks acme.finance:/policies/margin-standard --bundle-id acme.product --scope-bundle acme.finance --details --json
okf lint --bundle-id acme.product --scope-bundle acme.finance --fail-on never --json
```

Check effective roots, requested/examined scope, availability, and source resolution. Without the
added scope, a target remains out-of-scope rather than established missing. Validate each bundle
separately. Supported scope flags on `mv`/`rm` enable integrity guards and can refuse changes that
would leave external consumers stale; they do not orchestrate cross-bundle rewrites. Inspect the
command's supported options rather than assuming every mutation accepts scope.

Use explicit `--bundle-id` throughout catalog work. `OKF_BUNDLE` stays path-only, and an explicit
bundle path conflicts with `--bundle-id`. Use core-profile validation separately when assessing
interchange with readers that do not understand the support namespace.

## Separate repositories

The same catalog model works with existing sibling checkouts:

```text
work/
  product-repo/
    okf-catalog.yaml
    knowledge/
      okf-bundle.yaml
      metrics/revenue.md
  finance-repo/
    knowledge/policies/margin-standard.md
```

From `product-repo`, register the existing roots below. Put product settings in
`knowledge/okf-bundle.yaml`, using the manifest structure above:

```yaml
catalog_version: 2
default_bundle: acme.product
bundles:
  acme.product:
    location: {type: directory, path: knowledge}
  acme.finance:
    location: {type: directory, path: ../finance-repo/knowledge}
```

A layout-preserving source mapping in `knowledge/metrics/revenue.md` is:

```yaml
sources:
  - id: margin-standard
    resource: ../../../finance-repo/knowledge/policies/margin-standard.md
    bundle_ref:
      id: acme.finance
      path: policies/margin-standard.md
```

```markdown
See the [margin standard](../../../finance-repo/knowledge/policies/margin-standard.md).
```

Verify from `product-repo`:

```sh
okf catalog --json
okf resolve ../../../finance-repo/knowledge/policies/margin-standard.md --from metrics/revenue --bundle-id acme.product --scope-bundle acme.finance --json
okf backlinks acme.finance:/policies/margin-standard --bundle-id acme.product --scope-bundle acme.finance --details --json
```

These relative paths must match the checkout arrangement. When location independence is wanted,
explicitly author `okf://acme.finance/policies/margin-standard.md` in the body and applicable source
resource instead. Keep any `bundle_ref` identity/path consistent and preserve snapshot expectations.
Do not heuristically convert existing provenance or claim that the logical URI is a historical pin.

## Optional machine-local override

For an existing finance checkout at `../finance-checkout/knowledge`, a local override file can be:

```yaml
bundles:
  acme.finance: ../finance-checkout/knowledge
```

Select it with `catalog_overrides: okf-catalog.local.yaml` in the catalog, or
`OKF_CATALOG_OVERRIDES` (its filename resolves from cwd and takes precedence). Paths inside the
override resolve from that file. Use the repository's local-ignore convention for machine-specific
files and inspect `okf catalog --json` after the change.

An override changes location, not snapshot acceptance, access policy, or ordinary link meaning.
Relative paths may need reviewed edits if layout changes; logical URIs continue using the registered
identity. A known conflicting ordinary resource cannot be redirected by `bundle_ref`. Git overrides
must match their accepted locked content. This directory-catalog setup neither acquires checkouts
nor establishes a global consumer inventory.

## Existing TOML workspaces

Retain valid legacy configuration when the task does not request migration. For example, a
workspace may still have:

```toml
catalog = "okf-catalog.yaml"
default_bundle = { id = "acme.product" }

[bundle_settings."acme.product".query]
scan_limit = 1000
```

Legacy catalog pointers, ontology paths, and settings retain their configuration-file base.
Do not copy paths unchanged into manifests when that changes their target. Conflicting legacy
and manifest settings are errors; do not choose a winner silently. For requested configuration
migration, use the repair skill's explicit doctor plan/apply workflow and preserve recovery data.
Do not introduce TOML into a new workspace merely to reproduce a legacy example.
