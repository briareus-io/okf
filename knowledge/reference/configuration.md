---
type: Reference
audience: user
title: Configuration
description: Bundle selection, catalog locations, settings, and lint policies.
---
# Configuration

Bundle paths and catalog IDs use distinct selectors. The workspace catalog owns registered IDs,
locations, and the default bundle. Each optional root `okf-bundle.yaml` owns its portable settings,
guidance, and workflows. `ontology.yaml` remains a separate file.

```yaml
# okf-catalog.yaml
catalog_version: 2
default_bundle: acme.product
bundles:
  acme.product:
    location: {type: directory, path: knowledge/product}
  acme.finance:
    location: {type: directory, path: knowledge/finance}
```

```yaml
# knowledge/product/okf-bundle.yaml
manifest_version: 1
description: Product knowledge and requirements.
settings:
  ontology: ontology.yaml
  lint:
    source_unrecorded: warn
  views:
    summary:
      columns: ["$id", title, description]
guidance:
  instructions: |
    Preserve source URLs and distinguish observations from current facts.
```

Manifest schema versions are independent of CLI versions. Supported schema versions are
machine-discoverable; malformed or unsupported manifests fail rather than disappearing from
interpretation. Inspect effective values and their origins with `okf bundle context <bundle>`.

Selection follows explicit bundle path or `--bundle-id`, `OKF_BUNDLE`, a containing bundle,
configured default, an unambiguous sole catalog entry, then the legacy uncataloged working-directory
fallback. Path and ID selectors cannot be combined. `OKF_BUNDLE` always means a path. Use explicit
`--catalog <path>` to select a catalog instead of upward discovery. Catalog discovery does not merge
arbitrary ancestor catalogs or make every registered bundle part of examination scope.

Catalog-relative locations and bundle-root-relative manifest paths have different bases. Local
override paths resolve relative to the override file. Available registered roots are canonicalized
and cannot be equal or nested. The standalone manifest can identify a containing bundle without
requiring a catalog. A standalone bundle has no invented registration ID.

## Legacy configuration and migration

Supported legacy `okf.toml` remains readable during compatibility. Its catalog/override pointers,
`default_bundle`, and `bundle_settings` have the old path bases: configured ontology paths are
relative to the TOML file, not the bundle. Configuration migration must rebase those paths while
preserving their targets. Use doctor diagnosis and its explicit migration plan/apply mode; ordinary
diagnosis and safe fixes do not silently migrate workspace configuration.

When old and new configuration coexist, equal effective values are redundant; conflicting values
are errors with both origins reported. Legacy-only values remain visible during compatibility.
A broken manifest does not trigger a silent fallback to TOML. Migration preserves unrelated legacy
content and reports values or identity choices it cannot safely convert.

## Local overrides and remote dependencies

`OKF_CATALOG_OVERRIDES` can select a YAML file mapping registered IDs to locations:

```yaml
bundles:
  acme.finance: ../finance-checkout/knowledge
```

Overrides cannot register new IDs, weaken access policy, or accept different locked content. A
Git registration resolves through its accepted lock; explicit sync materializes that revision.
Ordinary reads inspect local material only. See [remote dependencies](../guides/remote-dependencies.md)
for planning, acceptance, acquisition, and offline checks.

A logical authored reference such as `okf://acme.finance/policies/margin-standard.md` resolves by
catalog identity and bundle-relative path. It is independent of a cache location but follows the
selected lock. Ordinary relative paths and web URLs retain their original meaning. Optional
`bundle_ref.snapshot` expectations remain separate: a current candidate does not satisfy a known
mismatch. Resolution does not verify claims or refresh fingerprints.

Selection and examination scope remain separate. Use supported `--scope-bundle` options or
deliberate `--catalog-scope` for additional examination, and retain completeness/unavailability
records. Historical workspace selection identifies the consumer catalog and lock at a revision;
explicit bundle revision selection inspects available historical objects without accepting a new
lock. The [generated command contracts](cli-contracts.md) show which commands admit each selector.

See [bundle customization](../guides/customize-a-bundle.md) for guidance and support resources.

## Source baseline lint setting

Sources with a fingerprint `kind` but no baseline produce `source-unrecorded` warnings by default.
This repository uses `knowledge/okf-bundle.yaml` and catalog ID `okf.docs`. Its manifest preserves
the documentation policy with:

```yaml
settings:
  lint:
    source_unrecorded: "off"
```

Accepted values are `off`, `info`, `warn`, and `error` (default `warn`). The selected bundle
manifest owns its policy. This setting leaves `source-missing` errors and
`okf stale` checks active; it does not record or refresh fingerprints.

All lint rules accept `off`, `info`, `warn`, or `error` under the same settings mapping.
Omitted settings retain these defaults:

| Setting | Finding rule | Default |
|---------|--------------|---------|
| `broken_link` | broken-link | error |
| `missing_title` | missing-title | warn |
| `missing_description` | missing-description | info |
| `orphan` | orphan | info |
| `ontology_violation` | ontology-violation | warn |
| `spec_v02` | okf-v02 | warn |
| `source_unrecorded` | source-unrecorded | warn |
| `source_missing` | source-missing | error |
| `index_coverage` | index-coverage | warn |

`off` suppresses only the selected lint rule. Conformance validation and `stale` checks
remain independent. `index_exclude` accepts glob patterns for index coverage exclusions.
`finding_budget` is a positive ontology finding limit (default 1,000 per concept).
`--fail-on` controls only the exit threshold; it does not change finding severities.

## Presentation formatting

The bundle manifest accepts `settings.formatting.yaml` and `settings.formatting.markdown`:

```yaml
settings:
  formatting:
    yaml: {style: readable, line_width: 100}
    markdown: {wrap_prose: false, line_width: 100}
```

These are the defaults. YAML style accepts `readable` or `legacy`; `line_width: 0` disables
wrapping for either format. Width is a target, so indivisible tokens can exceed it. Markdown
wrapping is opt-in and occurs only through explicit [format](../commands/format.md) invocations;
normal concept edits only apply the YAML policy. See that command for the supported Markdown
subset, value-preservation checks, and check/preview/apply examples.

Legacy TOML accepts `[bundle_settings.default.formatting.yaml]` and
`[bundle_settings.default.formatting.markdown]`, or the corresponding named-bundle tables.
Existing conflict rules apply when TOML and manifest settings coexist. `bundle context` exposes
effective formatting settings and their origins. Formatting policy is a local tool setting,
not an additional OKF conformance requirement or a mandate to reformat every bundle.
