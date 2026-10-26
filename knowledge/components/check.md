---
type: Component
layer: core
depends_on:
- /components/bundle
- /components/model
- /components/fingerprint
- /components/ontology
- /components/graph
- /components/parse
- /components/ports
- /components/query
title: check module
description: Conformance, optional-family, lifecycle, drift, and compatibility diagnostics.
last_modified: 2026-10-04T20:39:29Z
sources:
- resource: crates/okf-core/src/check/lint/mod.rs
  kind: git-path
  fingerprint:
    blob_sha: 4282f317c5e2124314442b8e3820a6fd353548d9
- resource: crates/okf-core/src/check/lint/rules/ontology_violation.rs
  kind: git-path
  fingerprint:
    blob_sha: 6962acf83c0e81a6fdece9489d06656d4d69bccd
- resource: crates/okf-core/src/check/lint/rules/index_coverage.rs
  kind: git-path
  fingerprint:
    blob_sha: f079c8ba19b4a42376bfd0382a7090dec277ee87
- resource: crates/okf-core/src/check/validate.rs
  kind: git-path
  fingerprint:
    blob_sha: 18eedacd345b0d45bd56718f8f8011799fcd62d4
- resource: crates/okf-core/src/check/stale.rs
  kind: git-path
  fingerprint:
    blob_sha: b8f20b40cee8ce839c7acd73c40e32b698844a41
---
# check module

Diagnostics. Validate enforces all three conformance rules including index/log structure and root version syntax; lint checks optional families advisorily; stale compares corrected instants and fingerprint extensions; doctor performs tolerant upgrade preflight and safe repair. The implementation composes [bundle loading](bundle.md), the [model](model.md), [fingerprinting](fingerprint.md), the [ontology](ontology.md), the [graph](graph.md), [parsing](parse.md), effect [ports](ports.md), and artifact [queries](query.md).

Location: `crates/okf-core/src/check`.

Recursive metadata lint adds concrete paths and codes while retaining advisory ontology rules.
A depth or finding budget stop reports an incomplete check. Index coverage checks immediate
concept and concept-containing-directory links, honors configured exclusions, and remains
read-only. Neither local navigation policy nor optional ontology constraints tighten portable
OKF conformance.

The 0.5.0 source-health contract keeps drift, absent baseline, established
missing resource, unavailable computation, unsupported fingerprint kind, and
expiry distinct. A kindless source remains provenance without a fingerprint
health assertion. Diagnostic display filters do not change examination or
failure policy; configured lint severities are independent of an optional
write gate.
