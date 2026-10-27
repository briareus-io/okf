---
type: Reference
audience: user
title: Configuration
description: Bundle selection, catalog locations, settings, and lint policies.
---
# Configuration

Bundle paths and catalog IDs use distinct selectors. Selection follows this order:

1. An explicit trailing bundle path or `--bundle-id` (using both is an error).
2. `OKF_BUNDLE`, which always means a directory path.
3. The registered bundle containing the current directory.
4. `default_bundle` in the nearest `okf.toml`.
5. The sole catalog entry, if unambiguous.
6. The current directory when no catalog is configured; otherwise select a bundle explicitly.

The selected path must exist and be a directory. `init` permits a new target. The nearest
`okf.toml` is discovered by walking upward from the working directory. New configurations use
a typed default; the legacy `bundle = "knowledge"` remains a deprecated path-only alias. If both
are supplied, only equivalent path defaults are accepted.

```toml
# okf.toml
catalog = "okf-catalog.yaml"
default_bundle = { id = "acme.product" }
# For single-bundle use: default_bundle = { path = "knowledge" }

[bundle_settings."acme.product"]
ontology = "tooling/product-ontology.yaml"
```

```yaml
# okf-catalog.yaml
catalog_version: 1
bundles:
  acme.product:
    location: {type: directory, path: knowledge/product}
  acme.finance:
    location: {type: directory, path: knowledge/finance}
```

Catalog and configured ontology paths are relative to `okf.toml`; directory locations are
relative to the catalog. Available registered roots are canonicalized and must not overlap.
Unavailable directories remain visible diagnostics. Each bundle owns its interpretation settings;
`[bundle_settings.default]` applies to uncataloged use. The optional root `ontology.yaml` remains
the discovery fallback and never establishes a bundle boundary.

Local checkout overrides use `catalog_overrides` in `okf.toml`, or `OKF_CATALOG_OVERRIDES`
(the environment value takes precedence). The YAML override file maps `bundles` to paths:

```yaml
bundles:
  acme.finance: ../finance-checkout/knowledge
```

Override paths are relative to that file. Overrides cannot register new IDs or accept different
snapshot content. The CLI reads available local files and Git objects without cloning or fetching.

Git discovery, revision resolution, and historical object reads are shared within each CLI
command, including failed lookups. Historical concepts, settings, snapshot checks, and related
record expansion reuse a `git cat-file --batch` reader per repository. Caches are discarded
when the command finishes; later commands observe new repository state. Refresh and catalog
source-edge checks batch and deduplicate Git fingerprints while retaining each source occurrence
and its individual comparison or error. These optimizations preserve explicit examination scope.

Selection and examination scope are separate. Catalog registration alone does not include another
bundle in a query, graph or check. Use repeatable `--scope-bundle <id>` or deliberate `--catalog-scope` as described in the [CLI reference](../commands/index.md)
and retain reported examined scope and unavailable members. Cross-bundle ordinary paths preserve
the surrounding layout; copying one directory alone can break those references. Optional
`bundle_ref` source metadata adds target identity and snapshot expectations while retaining
ordinary `resource` meaning. Current candidates stay separate from requested snapshot matches;
resolving a reference never verifies its claims or refreshes its fingerprints.

`okf catalog --json` inspects effective registrations without selecting a primary. Graph targets
can use qualified IDs such as `acme.finance:/policies/margin-standard`. `--revision <ref>` is
available for graph, links, backlinks, resolve, affected, search and list; other reads and writes use
current local content. Additional scope is supported by those commands plus lint; other commands reject scope options.

See [multi-bundle design](../contributing/design/multi-bundle-capability.md) and
[structured metadata design](../contributing/design/structured-metadata-capability.md) for compatibility and limits.

## Source baseline lint setting

Sources with a fingerprint `kind` but no baseline produce `source-unrecorded` warnings by default.
This repository disables that finding in its `okf.toml`, while retaining all other lint checks.
To use the same local policy in another bundle, add:

```toml
[bundle_settings.default.lint]
source_unrecorded = "off"
```

Accepted values are `off`, `info`, `warn`, and `error` (default `warn`). For a named bundle,
use `[bundle_settings."<id>".lint]`. This setting leaves `source-missing` errors and
`okf stale` checks active; it does not record or refresh fingerprints.

All lint rules accept `off`, `info`, `warn`, or `error` under the same settings table.
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
