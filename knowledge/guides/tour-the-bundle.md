---
type: Guide
audience: user
uses:
- /commands/catalog
- /commands/bundle-context
- /commands/bundle-resource-show
- /commands/bundle-validate
- /commands/list
- /commands/validate
- /commands/lint
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

Run from this checkout's root. These commands query the actual manual; destructive and remote
scenarios use the separate disposable fixtures described below.

## Inspect the bundle's own configuration

```sh
okf catalog --json
okf bundle context knowledge --json
okf bundle context knowledge --workflow check-documentation --json
```

The catalog supplies `okf.docs`; the manifest supplies lint policy, views, explicit query presets,
and guidance. The selected workflow returns its instructions and supporting-file origin without
executing checks. Its instruction file lives in the support directory, which ordinary concept
queries exclude. Inspect it deliberately:

```sh
okf bundle resource show .okf-bundle/workflows/check-documentation.md knowledge
```

## Find commands that can write

```sh
okf search knowledge --type Command --facet-filter 'mutates=true' --full-scan --view commands
```

Expect entries including `edit` and `refresh`. `mutates` denotes write capability, not that every
invocation writes; command arguments describe conditions such as dry-run and index output.
The named view comes from `knowledge/okf-bundle.yaml` and shows compact human columns.
The catalog registers the same directory as `okf.docs`, so the preset form is equivalent:

```sh
okf list --bundle-id okf.docs --preset writing-commands --view commands --full-scan
```

## Find workflows that use refresh

```sh
okf backlinks commands/refresh knowledge --details
okf links guides/maintain-source-backed-knowledge knowledge --details
okf graph knowledge --root guides/maintain-source-backed-knowledge --direction outgoing --depth 1
```

Expect the maintenance guide and update skill among incoming uses. Outbound guide edges connect
to its commands and explanations. The ontology labels `uses` edges and their incoming `used-by`
view. Body links and declared metadata are distinct evidence.

To select only guides with an authored `uses` relationship, rather than every ordinary backlink:

```sh
okf backlinks commands/refresh knowledge --preset guides-using-command
```

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

## Check the showcase

```sh
okf bundle validate knowledge
okf validate knowledge --profile bundle
okf lint knowledge --fail-on any --json
```

These check different contracts. The manifest preserves `source_unrecorded: off` because the manual
records provenance without maintaining every source fingerprint; missing-source checks remain active.
Core-profile validation additionally sees the support Markdown and is a separate interchange check.
Repository-wide `python3 scripts/check-docs.py` verifies this live configuration and runs disposable
examples for authoring, migration, views, workflow inputs, relationship presets, and recovery. The
plugin workflow checks exercise remote planning against a local Git fixture without fetching a real
upstream. Destructive examples and dependency acquisition belong in those isolated fixtures, not in
ordinary reads of this manual.

## Capability coverage

The live manual and disposable examples serve different purposes:

| Capability | Showcase | Executable evidence |
| --- | --- | --- |
| Portable configuration and default selection | This bundle's manifest and `okf.docs` catalog registration | Documentation checks assert the preserved lint policy and views. |
| Guidance, instruction files, and named workflows | This bundle's check workflow; customization scratch bundle | Context identity, supplied/defaulted inputs, inspection roles, and hidden support files are checked. |
| Views, typed queries, and presets | Commands and guides in this manual | Named writer/guide presets and projection records are checked. |
| Relationship kinds, inverses, and expansion | Real guide/skill `uses` edges; blocking-defects fixture | Exact semantic filters, ordinary-link contrast, and expansion completeness are checked. |
| Provenance and source impact | Command pages cite implementation paths | Affected analysis and source namespace behavior are checked without manufacturing verification. |
| Authoring, migration, and recovery | Disposable bundles in the worked guides | Example commands and resulting artifacts are checked without mutating the manual. |
| Remote acquisition and reviewed updates | Isolated local-Git fixtures | CLI/core tests check locks, plan/apply, integrity, scope, and read-only behavior; ordinary manual reads need no network. |

Keep this coverage tied to behavior, not merely the number of command pages. Add a runnable example
and an outcome assertion when a major feature is introduced. Live private-host authentication and
agent judgment require separate integration/evaluation environments; neither is established by a
successful local fixture or by a relationship declaring that a guide uses a command.
