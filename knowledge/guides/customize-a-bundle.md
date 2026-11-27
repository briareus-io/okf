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
description: >-
  Keep portable settings, task guidance, workflows, and supporting resources with a
  bundle.
---
# Customize a bundle

A requirements team wants every author to preserve source URLs and wants a standard procedure
for importing a specification. The bundle manifest can carry both. This example also defines
a filtered inventory of notes. Start in an empty working directory and run commands from that
directory throughout.

## Give the bundle one note

Create a bundle named `bundle`.

```sh
okf init bundle
```

Declare the note type. This creates or updates `bundle/ontology.yaml`.

```sh
okf ontology add Note bundle
```

Add a note that the inventory can find.

```sh
okf add notes/overview bundle --type Note --title "Vendor documentation" --description "Requirements collected from selected sources." --body "Record source observations separately from current facts."
```

## Define instructions, a workflow, and a view

Create the following two files, including the directories in their paths. The first file gives
an author the procedure to follow when someone requests a specification import.

**File: `bundle/.okf-bundle/references/import-specification.md`**

```markdown
# Import a supplied specification

Establish which supplied document the source input identifies and the requested scope.
Create requirements with provenance. Preserve source URLs without following every URL
mentioned in the document. Report created concepts, unresolved questions, and validation.
Use the selected review_level when explaining the evidence review.
```

The manifest below makes the URL-preservation rule bundle-wide and adds a rule specific to
relationship reads. The named workflow takes a required `source` and an optional `review_level`.
The `overview` view selects fields for display and the `documented-notes` preset selects concepts.

**File: `bundle/okf-bundle.yaml`**

```yaml
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
    outputs:
      - Imported requirement records
      - A list of unresolved questions
    inputs:
      source:
        type: string
        required: true
        description: Identifier of the document supplied for this task.
      review_level:
        type: enum
        values: [standard, detailed]
        default: standard
```

Check the manifest. A successful validation confirms its schema and configured resources.

```sh
okf bundle validate bundle
```

Generate navigation for this new example bundle. In a bundle with curated indexes this
command replaces authored index prose.

```sh
okf docs bundle --format index
```

Validate the knowledge documents using the bundle profile. Its scope excludes the workflow
instructions in `.okf-bundle/`, which have no concept frontmatter.

```sh
okf validate bundle --profile bundle
```

The manifest travels with the bundle. `manifest_version` names the schema version. A catalog
owns workspace IDs and locations. [Configuration](../reference/configuration.md) describes
the precedence of these settings.

## Display the inventory and check its quality

Apply the preset and view together.

```sh
okf list bundle --preset documented-notes --view overview --json
```

Expect one `projection` record for `/notes/overview` with the selected ID, title and
description. The manifest’s `extended_output: true` enables this view for JSON. To inspect
the filters that the preset contributes before running a query, request its context.

```sh
okf bundle context bundle --operation list --query '{"preset":"documented-notes"}' --json
```

The `effective-query` record contains the `Note` type filter. A preset can also select the
starting concepts for a [saved query](save-and-run-queries.md).

Check the manifest’s lint policy separately. The example should pass; a warning reaches the
failure threshold specified here.

```sh
okf lint bundle --fail-on warn --json
```

## Select instructions for a task

Request both planned operations together when checking relationship evidence.

```sh
okf bundle context bundle --operation links --operation backlinks --json
```

Look for the global instruction and the matching `relationship-evidence` rule. The rule's
applicability comes from its `operations` list. Plain `bundle context` shows the inventory,
and `--all` returns bodies for inspection.

Read the import instructions and expected deliverables before selecting a review level.

```sh
okf bundle context bundle --workflow import-specification --format agent
```

The workflow lists `Imported requirement records` and `A list of unresolved questions` under
`Outputs`. It also reports the missing `source`. For the input schema, request the structured
workflow record. Its `inputs.review_level.values` lists `standard` and `detailed`.

```sh
okf bundle context bundle --workflow import-specification --json
```

Supply a fictional document identifier and choose the detailed review.

```sh
okf bundle context bundle --workflow import-specification --inputs '{"source":"supplied-vendor-specification","review_level":"detailed"}' --json
```

The workflow record reports `inputs_complete: true` and the effective values. Omit
`review_level` to use `standard`. Omit the required `source` and the workflow record lists
the missing input. Selecting the workflow loads instructions for an author. The author still
needs to inspect the document, perform the requested import, and verify the result.

### Reuse an instruction file

A rule and a workflow can point to the same bundle-relative `instructions_file`. When both are
delivered, the first prints the file and the second identifies the earlier rule or workflow.
For example, if global guidance and the import workflow share a file, the workflow shows
`Same instructions as [rule $global]` along with its own inputs and outputs. A summary alone
does not deliver the file. In JSON, resolve `content: shared` through `instructions_ref.kind`
and `instructions_ref.id` within the same bundle. The referring record supplies its own role.

## Check context freshness before using it

If a later read or edit depends on exactly these instructions, request a token covering the
planned operations.

```sh
okf bundle context bundle --operation show --operation edit --operation validate --token --stats --json
```

Check the process exit and the terminal `operation-complete` record. Save the **value of the
`token` field** from the `context-token` record as `context-token.json` in the working
directory. Save its decoded string contents, including the JSON braces, without the outer
string quotes. Check the token for a planned edit.

```sh
okf bundle context bundle --check-token @context-token.json --operation edit --json
```

A current token covers this edit. Supply the same token on a covered read.

```sh
okf show notes/overview bundle --context @context-token.json --json
```

The read returns the note metadata. A stale token or uncovered operation fails before execution.
Reload context after changing settings, ontology, instructions, workflow resources, or scope.
Editing ordinary concept content alone leaves context current. Multi-bundle calls take one
`--context` token per examined bundle. Tokens check freshness at command entry; writes still
use their ordinary validation, baseline and authorization checks.

## Keep guidance and supporting materials usable

Use short mandatory rules for requirements that apply to every matching operation. Give an
optional explanatory rule `tier: reference` and a useful `description` so authors can request
its body with `--rule ID`. Named workflows hold procedures. `--stats` reports delivered bytes
per section; `--guidance-budget N` compares mandatory instruction bytes with an advisory budget.
An optional `provenance` mapping on instructions, rules or workflows records publisher
metadata in `origin.provenance`.

Choose one of `instructions` and `instructions_file` for each body. File and resource paths
are relative to the bundle root. Keep templates, scripts and operational reports under
`.okf-bundle/` when they need explicit inspection. Use an ordinary concept for a report
readers should find through knowledge search. Reading a workflow definition leaves its
scripts inert. The author decides which actions the task authorizes.

For a preset applied to a relationship, see [Follow a relationship across two hops](find-and-read-knowledge.md#follow-a-relationship-across-two-hops).

When a workflow depends on a recurring lookup, declare its saved-query names in
`uses_queries`. See the [feature review example](save-and-run-queries.md#give-a-workflow-its-saved-queries).
