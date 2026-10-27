---
type: Guide
audience: user
uses:
- /commands/search
- /commands/backlinks
- /commands/links
- /commands/graph
- /commands/artifact-show
- /commands/affected
explains:
- /concepts/ontology-and-relationships
- /concepts/sources-and-provenance
title: Tour this live knowledge bundle
description: Use the manual itself to explore metadata, relationships, artifacts and source impact.
---
# Tour this live knowledge bundle

Run from this checkout's root. These commands query the actual manual; there is no synthetic
showcase bundle to keep in sync.

## Find commands that can write

```sh
okf search knowledge --type Command --facet-filter 'mutates=true' --full-scan --view commands
```

Expect entries including `edit` and `refresh`. `mutates` denotes write capability, not that every
invocation writes; command arguments describe conditions such as dry-run and index output.
The named view comes from the repository's `okf.toml` and shows compact human columns.

## Find workflows that use refresh

```sh
okf backlinks commands/refresh knowledge --details
okf links guides/maintain-source-backed-knowledge knowledge --details
okf graph knowledge --root guides/maintain-source-backed-knowledge --direction outgoing --depth 1
```

Expect the maintenance guide and update skill among incoming uses. Outbound guide edges connect
to its commands and explanations. The ontology labels `uses` edges and their incoming `used-by`
view. Body links and declared metadata are distinct evidence.

## Expand related command metadata

```sh
okf search knowledge --type Guide --text "Maintain source-backed knowledge" --in title --expand uses --target-field title --json
```

Expect the guide plus related command metadata and expansion-completeness evidence. The read is
bounded; a small primary result does not guarantee all expansions were delivered.

## Inspect a real artifact

```sh
okf artifact show references/changesets/reorganize.yaml knowledge
```

This reads the file; it does not execute the changeset. Follow the
[reorganization guide](reorganize-a-bundle.md) to exercise it on a disposable fixture.
The [modeling guide](model-your-knowledge.md) uses the two authoring artifacts.

## Trace source impact

```sh
okf affected knowledge --changed crates/okf-core/src/mutate/refresh.rs --changed-base repo
```

Expect `commands/refresh` among affected documentation. This demonstrates source lineage;
it does not decide whether the prose needs changing. Follow the
[maintenance guide](maintain-source-backed-knowledge.md) to make that judgment.
