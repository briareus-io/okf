---
type: Guide
audience: user
uses:
- /commands/affected
- /commands/artifact-put
- /commands/artifact-show
- /commands/add
- /commands/edit
- /commands/show
- /commands/refresh
- /commands/verify
- /commands/validate
- /commands/lint
explains:
- /concepts/sources-and-provenance
- /concepts/verification-and-freshness
title: Maintain source-backed knowledge
description: >-
  Find impacted documentation, compare current sources, and reconcile claims without
  stored fingerprints.
---
# Maintain source-backed knowledge

When a source changes, use its recorded path to find documents that cite it. In this example a service changes from two retries to three. You will identify its dependent note, compare the source with the claim, and correct the note.

## Record a source and its claim

In an empty working directory, create a bundle named `bundle`.

```sh
okf init bundle
```

Save the service configuration beside the bundle.

**File: `service.txt`**

```text
retry_limit=2
```

Copy it to a stable location inside the bundle.

```sh
okf artifact put references/service.txt @service.txt bundle
```

Create the note with the initial claim.

```sh
okf add notes/service bundle --type Note --title "Service retries" --description "Retry behavior from the service configuration" --body "The service retries twice."
```

Record where the claim came from. A source with `kind=file` resolves from the bundle root.

```sh
okf edit notes/service bundle --add-source 'resource=references/service.txt,kind=file'
```

## Find the note after the configuration changes

Update the local configuration file.

**File: `service.txt`**

```text
retry_limit=3
```

Replace the bundled copy at the same path.

```sh
okf artifact put references/service.txt @service.txt bundle --replace
```

Find concepts that cite this resource.

```sh
okf affected bundle --changed references/service.txt
```

The result includes `/notes/service`. Read the current source and the claim it supports.

```sh
okf artifact show references/service.txt bundle
```

```sh
okf show notes/service bundle --body
```

The artifact contains `retry_limit=3`. The note still says “The service retries twice.”

## Correct and check the claim

Preview the edit and inspect the changed line.

```sh
okf edit notes/service bundle --set-body "The service retries three times." --dry-run
```

Apply the reviewed edit.

```sh
okf edit notes/service bundle --set-body "The service retries three times."
```

Check the resulting prose and frontmatter.

```sh
okf show notes/service bundle
```

The note should say “The service retries three times.” Its `sources` entry should still identify `references/service.txt`. Check document conformance and local conventions with the following commands.

```sh
okf validate bundle
```

```sh
okf lint bundle --fail-on error
```

An unrecorded fingerprint may appear as a lint warning. The known change and the two versions of the claim supply the evidence needed for this correction. If the source changes again but the claim still holds, keep the prose and record your review according to local practice. If the source is unavailable, restore or replace the evidence before revising the claim.

Bundles that track fingerprint baselines can record the current fingerprint after review with [refresh](../commands/refresh.md). A reviewer who checked the document against its sources can record that separate event with [verify](../commands/verify.md).

## Apply the same workflow to this manual

The manual records repository-relative source paths and their kinds. From the repository root, run `okf affected knowledge --changed crates/okf-core/src/mutate/refresh.rs --changed-base repo`. The result includes `/commands/refresh`. Read that page with `okf show commands/refresh knowledge` and compare its claims with the current Rust source. The flag sets the base of the supplied changed path. Each stored source keeps its own resolution rules.

The manual tracks provenance without storing fingerprint baselines, and suppresses the missing-baseline lint warning. For more about these choices see [provenance](../concepts/sources-and-provenance.md), [verification and freshness](../concepts/verification-and-freshness.md), and [source-health troubleshooting](../reference/troubleshooting.md). For agent-assisted reconciliation, use the [update skill](../skills/update.md).
