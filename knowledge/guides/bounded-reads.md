---
type: Guide
audience: user
uses:
- /commands/search
- /commands/list
- /commands/show
- /commands/artifact-show
explains:
- /concepts/selection-and-scope
title: Performance and bounded reads
description: Choose scan, output and expansion limits without mistaking partial reads for absence.
---
# Performance and bounded reads

Use this guide when a query returns too much material or an automation needs to distinguish
a small response from a complete answer. Run the examples from the repository root. All
examples below inspect the live manual without modifying it.

## Choose which cost to bound

| Need | Control | What it does not guarantee |
| --- | --- | --- |
| Fewer displayed matches | `--limit` | Less examination work. |
| Fewer examined documents | `--scan-limit` | Complete matches, totals, or global ranking. |
| Exhaustive examination | `--full-scan` | Unbounded delivery, expansion, or available external scope. |
| Fewer metadata fields | `--project`, human `--columns` or `--view` | Changed filtering or a smaller scan. |
| Less prose | `show --outline` then `show --lines` | A complete document review. |
| Fewer related records | Expansion edge, target and byte limits | A complete relationship neighborhood. |
| Less artifact content | `artifact show --max-bytes` or `--lines` | A full file read. |

## Start with one page of a complete search

```sh
okf search knowledge --type Command --full-scan --sort id --limit 5 --json
```

Expect five command concepts and a query summary with complete examination, more than five
total matches, and `has_more: true`. The page is delivered successfully, but it is not the
entire inventory. `--limit 5` does not mean only five files were read.

For human navigation, an outline is often cheaper to consume than another broad query:

```sh
okf show reference/command-contracts knowledge --outline
okf show reference/command-contracts knowledge --lines 1:20
```

The line window is inclusive and includes serialized frontmatter. Use the heading numbers
from the outline to choose a relevant window rather than assuming the first lines contain
the answer.

## Deliberately trigger and repair an incomplete scan

```sh
okf search knowledge --type Command --scan-limit 1 --limit 1 --json
okf search knowledge --type Command --full-scan --limit 1 --json
```

The first invocation examines at most one eligible document. In this manual it reports a
scan-limit warning and partial examination even though the process exits zero. Its match
count cannot establish the total number of commands. The second invocation removes the scan
budget and reports complete examination while still displaying only one result.

For a complete unpaginated inventory:

```sh
okf list knowledge --type Command --full-scan --project '$id' --project 'type' --json
```

Inspect the query summary and terminal completion record. `--full-scan` and `--scan-limit`
are alternative choices; an unavailable requested bundle still limits what can be established.

## Page through a stable selection

```sh
okf search knowledge --type Command --full-scan --sort id --limit 5 --offset 0 --json
okf search knowledge --type Command --full-scan --sort id --limit 5 --offset 5 --json
```

The pages should contain different IDs while the working tree is unchanged. Each invocation
rescans its scope; offset is not a cached continuation token. Do not use offset to repair an
incomplete scan, and do not assume separate working-tree pages form an immutable snapshot.
A one-shot inventory is often simpler for an automation that needs all records.

## Inspect expansion and artifact bounds separately

```sh
okf search knowledge --type Guide --text "Maintain source-backed knowledge" --in title --expand uses --expansion-edges 1 --json
okf artifact show references/changesets/reorganize.yaml knowledge --max-bytes 32 --json
```

The guide's primary match is complete, but the expansion summary reports truncation because
it uses more than one command. The artifact result similarly sets `truncated: true`; 32 bytes
cannot contain the whole changeset. Neither omission means the missing material does not exist.

Retrieve the needed command directly, increase the relevant expansion bound, or read the
small artifact in full:

```sh
okf show commands/affected knowledge
okf artifact show references/changesets/reorganize.yaml knowledge --json
```

## Measure before choosing a performance budget

Write previews do real preparation and validation work. Guarded digests and changesets inspect
covered inputs; they are not constant-time metadata lookups. Views reduce reader output, not
necessarily CPU or I/O. Per-invocation Git reuse does not create a cache shared across later calls.

For local measurement, `scripts/benchmark_contracts.py` creates disposable fixtures at 100,
1,000, and 10,000 concepts. Build the intended binary and run the helper with its matching
`--binary`, `--build-profile`, and `--output` options. It measures elapsed time and output size,
not peak memory, actual I/O, or a universal capacity limit. Compare like builds and retain
completeness requirements when comparing results.

See [JSON automation](automate-with-json.md) for an executable completeness check,
[search](../commands/search.md) for exact options, and
[troubleshooting](../reference/troubleshooting.md) for unavailable scope and interrupted operations.
