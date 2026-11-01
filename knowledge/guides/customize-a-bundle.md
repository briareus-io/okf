---
type: Guide
audience: user
uses:
- /commands/init
- /commands/add
- /commands/bundle-context
- /commands/bundle-validate
- /commands/validate
- /commands/lint
- /commands/list
- /commands/docs
- /commands/ontology-add
title: Customize a bundle
description: Keep portable settings, task guidance, workflows, and supporting resources with a bundle.
---
# Customize a bundle

Use a manifest when a bundle needs portable settings or task guidance. This example creates a
small bundle with a required instruction file and a named workflow, then inspects that workflow
without executing it. Run all blocks in one POSIX shell with a compatible `okf` on PATH.

## Create the bundle and its instructions

```sh
demo_root=$(mktemp -d)
cd "$demo_root"
okf init "$demo_root/bundle"
okf ontology add Note "$demo_root/bundle"
okf add notes/overview "$demo_root/bundle" --type Note --title "Vendor documentation" --description "Requirements collected from selected sources." --body "Record source observations separately from current facts."
mkdir -p "$demo_root/bundle/.okf-bundle/references"
cat > "$demo_root/bundle/.okf-bundle/references/import-specification.md" <<'MARKDOWN'
# Import a supplied specification

Establish which supplied document the source input identifies and the requested scope.
Create requirements with provenance. Preserve source URLs without following every URL
mentioned in the document. Report created concepts, unresolved questions, and validation.
Use the selected review_level when explaining the evidence review.
MARKDOWN
cat > "$demo_root/bundle/okf-bundle.yaml" <<'YAML'
manifest_version: 1
description: Requirements extracted from selected vendor documentation.
settings:
  ontology: ontology.yaml
  lint:
    source_unrecorded: warn
    missing_description: warn
  views:
    overview:
      columns: ["$id", title, description]
      extended_output: true
  query_presets:
    documented-notes:
      description: Notes in this documentation bundle.
      operations: [search, list]
      query:
        type: Note
guidance:
  display:
    json: full
    text: summary
  instructions: |
    Preserve supplied source URLs. Do not fetch URL-valued fields merely
    because they appear in a document; fetch only within the user's request.
  rules:
    - id: relationship-evidence
      operations: [links, backlinks]
      instructions: |
        A reference alone does not establish agreement or acceptance.
workflows:
  import-specification:
    description: Import a supplied specification under the authoring conventions.
    instructions_file: .okf-bundle/references/import-specification.md
    inputs:
      source:
        type: string
        required: true
        description: Identifier of the document supplied for this task.
      review_level:
        type: enum
        values: [standard, detailed]
        default: standard
YAML
okf bundle validate "$demo_root/bundle"
okf docs "$demo_root/bundle" --format index
okf validate "$demo_root/bundle" --profile bundle
```

The scratch bundle has only generated index prose, so the example regenerates its navigation.
The manifest is valid, and the knowledge documents pass bundle-profile validation. The instruction
file deliberately has no concept frontmatter. Default core validation includes that Markdown and
may report a violation; the bundle tooling profile excludes the support namespace. See
[bundles and concepts](../concepts/bundles-and-concepts.md) for the compatibility boundary.

`manifest_version` identifies the schema, independently of the CLI release. A missing manifest uses
defaults; an unsupported version or malformed manifest is an error. `ontology.yaml` remains
separate. The catalog owns workspace IDs and locations, while the manifest travels with the bundle.

## Use lint policy, views, and presets

```sh
okf list "$demo_root/bundle" --view overview --json
okf list "$demo_root/bundle" --preset documented-notes --view overview --json
okf bundle context "$demo_root/bundle" --operation list --query '{"preset":"documented-notes"}' --json
okf lint "$demo_root/bundle" --fail-on warn --json
```

The view selects useful display fields. Its `extended_output: true` explicitly enables separate JSON
projection records; callers still read the stream by record kind. The preset selects `Note` concepts
and remains inactive unless requested. Context shows the effective query before execution. Lint
uses the manifest's warning policy; warnings are advisory by default, while `--fail-on warn` turns
them into this command's failure threshold. These three controls have different jobs: presentation,
selection, and quality policy.

## Inspect and select context

```sh
okf bundle context "$demo_root/bundle" --json
okf bundle context "$demo_root/bundle" --operation links --operation backlinks --json
okf bundle context "$demo_root/bundle" --workflow import-specification --json
okf bundle context "$demo_root/bundle" --workflow import-specification --inputs '{"source":"supplied-vendor-specification","review_level":"detailed"}' --json
okf bundle context "$demo_root/bundle" --all --json
```

The first response contains global instructions and inventories of scoped rules and workflows.
Operation selection returns the full relationship rule. Selecting the workflow without inputs
reports `inputs_complete: false` and the missing `source`; supplying it produces complete inputs.
The optional `review_level` defaults to `standard` when omitted. Input completeness says nothing
about whether the source was inspected or the workflow succeeded: these commands only return
context. They do not import a document, fetch a URL, or run a script.

`--all` returns every instruction body for inspection. It does not activate workflows. Canonical
operation names include subcommands such as `artifact resolve`; omitted `operations` makes a rule
bundle-wide, while an empty list is invalid.

Skills load global guidance before source access and applicable command instructions before acting.
Incidental guidance in command output cannot replace that pre-action read. JSON defaults to full
guidance, and text output uses a short stderr notice. The effective context digest includes loaded
instruction files, so editing a required file invalidates cached context even if the manifest is
unchanged.

## Choose where supporting material belongs

Use exactly one of `instructions` and `instructions_file` for each instruction body. A selected
instruction file is mandatory guidance; optional workflow `resources` are supporting material to
inspect when needed. Paths are relative to the bundle root.

The `.okf-bundle/` directory can also contain scripts, templates, and operational reports. Ordinary
knowledge queries hide these files; [bundle resource](../commands/bundle-resource-list.md) provides
explicit inspection. Physical snapshots preserve them. Reports intended for ordinary knowledge
search belong outside the support tree as concepts.

A workflow can request a script, but reading its definition does not execute or authorize it.
An agent uses the declared working directory, treats inputs as data, checks unfamiliar actions
against the task, and reports missing tools or failed steps. Managed remote write protection still
applies to its support files.

See [configuration](../reference/configuration.md) for setting ownership and precedence,
[find and read knowledge](find-and-read-knowledge.md#find-defects-blocking-a-feature) for a query-preset
recipe, and [remote dependencies](remote-dependencies.md) for explicit acquisition and locked revisions.
