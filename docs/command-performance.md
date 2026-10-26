# Performance and bounded reads

OKF can limit the material returned to people and programs while keeping examination scope
and incompleteness visible. Use those controls to keep reads manageable without mistaking a
small response for a complete inventory.

This guide describes available CLI 0.5.0 capabilities. See
[consistent command behavior](consistent-command-contracts.md) for input and write semantics,
and [checking results](command-contracts-validation.md) for accepting bounded results safely.

## Choose controls for the cost you need to limit

| Need | Available control | Important distinction |
| --- | --- | --- |
| Fewer primary results | `--limit` on list/search | Limits returned results, not the work needed to examine and rank candidates. |
| Bound concept examination | `--scan-limit` | May make matching counts and ordering incomplete; check the query summary. |
| Complete inventory | `--full-scan` | Removes the document scan budget; output pagination and expansion bounds remain separate. |
| Smaller metadata payload | `--project`; human `--columns` or `--view` | Reduces delivered fields, not necessarily examination work. |
| Read part of a document | `show --outline` or `show --lines` | An outline or slice is not the complete document. |
| Bound related knowledge | Relationship expansion edge, target, and byte limits | Expansion can be incomplete even when the primary query is complete. |
| Bound artifact content | `artifact show` byte/line controls | Binary artifacts may return metadata instead of text; text may be truncated. |
| Restrict graph exploration | Root, direction, and depth controls | A neighborhood is not the entire graph. |

Use `okf schema` or the [CLI reference](okf-cli-reference.md) to check which controls apply to
a particular command. Commands do not share every option simply because they share result
conventions.

## Explore first, then request an inventory deliberately

For interactive discovery, start with a directory index or a small relevant search:

```sh
okf browse knowledge
okf search knowledge --text "retention" --limit 10 --json
okf show policies/retention knowledge --outline
```

The default list/search scan budget is 1,000 eligible documents across the selected scope.
A result limit of ten does not promise only ten document reads. Exact matching and ranking
can require examining the full permitted scope.

For a workflow that requires a complete inventory:

```sh
okf list knowledge --full-scan --project '$id' --project 'type' --json
```

Confirm completeness in the returned evidence. A requested scope member can still be
unavailable, and removing the document budget does not remove every other bound. `--full-scan`
and `--scan-limit` are alternative choices, not cumulative options.

## Keep pagination and freshness separate

Offset pagination controls delivery:

```sh
okf search knowledge --text "policy" --full-scan --limit 20 --offset 0 --json
okf search knowledge --text "policy" --full-scan --limit 20 --offset 20 --json
```

Each invocation examines its scope again. There are no persistent query-result handles or
shared query caches across invocations. Concurrent working-tree edits can change ordering
or page membership; pages are not an immutable snapshot. Where supported, an immutable Git
revision provides a fixed historical input instead.

For a program that needs all results once, consider one complete query with suitable
projections and consume its NDJSON incrementally. This avoids repeated page scans but does
not mean every query operation has constant memory use or streams results before ranking.

## Keep related reads explicit

Project metadata before fetching full documents. Use declared relationship expansion for a
bounded related view, and inspect its completion evidence independently from the primary hits.
If a target is omitted because of an edge, target, or byte limit, follow up with a specific
read rather than treating omission as absence.

Catalog scope adds examination work. Registering a bundle alone does not add that work to
every query; only request extra scope when it is relevant. The current interface has no
batch body-link resolution API, so do not assume that requesting a projection resolves every
link in every returned concept.

## Understand write costs

A preview prepares and checks the proposed state; it is not just a text diff. Coordinated
changesets currently use an isolated copy, and guarded baseline acquisition must read the
inputs covered by the token. `changeset digest` avoids preparing a dummy mutation, but it
is not a constant-time metadata lookup.

CLI writes use journaled publication and writer exclusion. An enabled ontology gate may
examine the bundle to evaluate new violations and affected references. Shared-sidecar checks
can add explicit consumer examination. These checks trade additional work for a reliable
publication decision; pagination must never be used to silently reduce mutation targets.

Ordinary queries do not calculate a whole-bundle changeset digest automatically. Some Git
and fingerprint work is reused within an invocation, including repeated source computations;
this is distinct from a cache that serves later invocations. Single-concept refresh is the
available refresh interface; batch refresh is not currently exposed.

## Measure a representative workload

Latency depends on document sizes, nested metadata, reference density, source kinds, artifact
volume, storage, and scope—not only concept count. Start with the commands your integration
actually uses and retain its completeness requirements while measuring.

The repository includes a local measurement helper:

```sh
python3 scripts/benchmark_contracts.py --binary target/release/okf \
  --build-profile release-all-features --output /tmp/okf-performance.json
```

Use that build-profile label only for a binary actually built with that profile/features.
The helper creates disposable fixtures at 100, 1,000, and 10,000 concepts and measures search,
inventory, projected reads, source checking, and edit preview. It records wall time and
output sizes for fresh processes. It does not flush filesystem caches or measure peak memory,
actual I/O, lock duration, or subprocess counts.

Keep generated measurements with your experiment rather than treating them as supported
capacity limits. Compare like build profiles and feature sets, note background activity and
cache conditions, and repeat runs before choosing a local performance budget. There is no
universal latency guarantee or published maximum bundle size implied by these examples.
