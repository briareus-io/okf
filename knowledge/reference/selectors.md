---
type: Reference
audience: user
title: Selectors and structured queries
description: Select metadata fields, filter typed values and expand declared references.
sources:
- resource: crates/okf-core/src/query/selector.rs
  kind: git-path
---
# Selectors and structured queries

Dotted selectors traverse object properties; `[]` traverses explicit list items;
`["policy.status"]` names a literal dotted key. `$id` projects computed identity independently
of authored `id`. Matching is case-sensitive for field names.

```sh
okf search knowledge --type Command --facet-filter 'mutates=true' --full-scan --columns '$id,title,mutates'
okf search knowledge --type Guide --project '$id' --project 'uses[]' --json
okf search knowledge --type Guide --expand uses --target-field title --json
```

Filters combine with AND; `in` supplies alternatives within one condition. Operands are typed
JSON values. Missing/null fields do not satisfy negative membership. Separate nested conditions
can match different list records: inspect the projected whole record before asserting correlation.
Legacy `--field` filters retain literal-key semantics.

Expansion follows declared outbound references one hop and has separate edge, target and byte
limits. Inspect its summary independently of the primary query. Views and human columns change
display without rewriting metadata; ordinary JSON retains concept records. Explicit projections
produce separate records. Facets aggregate examined matches before pagination and can report
excluded fields or incomplete scans.

See [search](../commands/search.md), [list](../commands/list.md), and
[bounded reads](../guides/bounded-reads.md) for flags and completeness.
