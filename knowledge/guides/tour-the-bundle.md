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
```

The catalog identifies this manual as `okf.docs`. Inspect its effective configuration:

```sh
okf bundle context knowledge --json
```

The context includes lint policy, views, presets, and available workflows. To read the instructions for checking this manual, select its workflow:

```sh
okf bundle context knowledge --workflow check-documentation --json
```

The response includes the selected workflow’s instructions and supporting-file origin. Its instruction file lives in the support directory, which ordinary concept
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
```

Incoming relationships include the maintenance guide and update skill. Inspect the maintenance guide from the other end:

```sh
okf links guides/maintain-source-backed-knowledge knowledge --details
```

Its outgoing edges identify commands the guide uses and concepts it explains. To see these neighbors together, render the one-hop graph:

```sh
okf graph knowledge --root guides/maintain-source-backed-knowledge --direction outgoing --depth 1
```

The graph shows the guide and its direct neighbors. The ontology labels `uses` edges and their incoming `used-by` view.

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
```

Expect `manifest: valid`. To check the concepts with the bundle tooling profile, run:

```sh
okf validate knowledge --profile bundle
```

Expect `conformant: no violations`. To check the manual against its configured lint policy, run:

```sh
okf lint knowledge --fail-on any --json
```

The lint output reports any actionable findings, and `--fail-on any` makes them fail the
command. The manual records source provenance without requiring a fingerprint for every
source. Missing-source checks still apply. See [troubleshooting](../reference/troubleshooting.md)
for help interpreting a finding.
