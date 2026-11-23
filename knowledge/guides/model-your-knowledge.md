---
type: Guide
audience: user
uses:
- /commands/init
- /commands/ontology-add
- /commands/ontology-update
- /commands/add
- /commands/edit
- /commands/show
- /commands/links
- /commands/backlinks
- /commands/search
- /commands/validate
- /commands/lint
explains:
- /concepts/ontology-and-relationships
title: Model and edit structured knowledge
description: Define metadata, apply a guarded patch, and author a relationship you can query.
sources:
- resource: /references/authoring/guide.yaml
- resource: /references/authoring/concept-patch.json
---
# Model and edit structured knowledge

A guide can declare its intended audience and identify the commands it teaches. This example defines the `audience` field, changes its value with a guarded patch, and adds a typed `uses` relationship that can be queried from either end. Start in an empty working directory.

## Define the audience field

Create the bundle.

```sh
okf init bundle
```

Save the [Guide definition](../references/authoring/guide.yaml) beside it. This definition requires one of two audience values.

**File: `guide.yaml`**

```yaml
description: A task guide for a named audience.
fields:
  audience:
    type: enum
    values: [user, contributor]
    required: true
```

Apply the definition.

```sh
okf ontology add Guide bundle --from @guide.yaml
```

Create a Guide for the `user` audience.

```sh
okf add guides/example bundle --type Guide --title "Example guide" --description "A structured authoring example" --set audience=user --body "A guide for readers."
```

The new file is `bundle/guides/example.md`. Its frontmatter includes:

```yaml
type: Guide
title: Example guide
audience: user
```

Check the [standard frontmatter fields](../concepts/bundles-and-concepts.md#standard-frontmatter-fields) before choosing custom names. For instance, the CLI reports an incompatible top-level `status` declaration as an advisory. Text output sends it to stderr. JSON output includes advisories in the change record's `findings` array. An advisory configured as `error` still permits an otherwise valid ontology write.

## Change the audience with a guarded patch

Use the [example patch](../references/authoring/concept-patch.json) when changing a guide intended for users into a contributor guide. The first operation checks the current audience before the patch replaces it and adds tags.

**File: `concept-patch.json`**

```json
[
  {"op": "test", "path": "/audience", "value": "user"},
  {"op": "replace", "path": "/audience", "value": "contributor"},
  {"op": "add", "path": "/tags", "value": ["worked-example", "authoring"]}
]
```

Preview the metadata diff.

```sh
okf edit guides/example bundle --patch @concept-patch.json --dry-run
```

After reviewing it, apply the patch.

```sh
okf edit guides/example bundle --patch @concept-patch.json
```

Inspect the guide.

```sh
okf show guides/example bundle
```

Expect `audience: contributor` and the tags `worked-example` and `authoring`. If you apply the patch again, its first test fails and the file stays unchanged. Patch paths use JSON Pointer syntax. Here `/audience` addresses the top-level frontmatter key.

## Declare and query a relationship

Define the type of the target concept.

```sh
okf ontology add Command bundle --description "A command described by a guide"
```

Create that concept.

```sh
okf add commands/check bundle --type Command --title "Check" --description "A sample check command" --body "Inspect the sample bundle."
```

Declare that a Guide can reference zero or more Commands through `uses`. Give the relationship a forward kind and an inverse label.

```sh
okf ontology update Guide bundle --ref 'uses:Command:0..n' --relationship-yaml 'uses={reference: uses, kind: uses, inverse: used-by}'
```

Set the list of targets on the guide.

```sh
okf edit guides/example bundle --set-yaml 'uses=[/commands/check]'
```

Inspect the authored assertion.

```sh
okf links guides/example bundle --details
```

Expect a `uses` relationship from `/guides/example` to `/commands/check`. Inspect the incoming view from the command.

```sh
okf backlinks commands/check bundle --details
```

The incoming view reports `used-by` for the same assertion. The target document needs no reverse link. To read the command title alongside the guide, expand the declared relationship rule.

```sh
okf search bundle --type Guide --expand uses --target-field title --json
```

Look for `Check` in the related concept's returned fields. The [manual's ontology](../ontology.yaml) also declares Guide-to-Topic `explains` references. The [bundle tour](tour-the-bundle.md) shows these relationships on real concepts.

## Check the result and choose an edit form

```sh
okf validate bundle
```

Expect `conformant: no violations`. To check the authored concepts against the ontology, run:

```sh
okf lint bundle --fail-on error
```

Validation checks the document format. Lint checks local conventions and ontology findings. For future edits, choose the operation that matches the part you want to change.

| Change | Use | What to check |
| --- | --- | --- |
| Replace a whole list or object | `--set-yaml` | The supplied value replaces the entire field. |
| Change an object property | `--set-path` | An intermediate map may be created. |
| Edit a list item or require an old value | `--patch` | Supply concrete JSON Pointer paths and guard dependent edits with `test`. |
| Remove a metadata key | `--unset` or `--unset-path` | Deletion and assigning null have different effects. |
| Change the explanation | `--set-body` or section flags | Review the prose and its references. |

See [edit](../commands/edit.md) for mutation flags and [selectors](../reference/selectors.md) for field reads. For queries that combine relationship attributes, projections, and facet counts, continue to [Query structured records and their relationships](query-structured-records.md).
