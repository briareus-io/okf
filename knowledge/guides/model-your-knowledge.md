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

Use this guide when you need to add structured metadata and make its relationships queryable.
You need the CLI on PATH and a checkout containing the example artifacts.

## Define and edit one guide

This exercise uses a [Guide definition](../references/authoring/guide.yaml) and a
[guarded JSON patch](../references/authoring/concept-patch.json). Run from the repository root.
`mktemp` creates a disposable destination; the checked-in manual is not edited.

```sh
demo_root=$(mktemp -d)
okf init "$demo_root/bundle"
okf ontology add Guide "$demo_root/bundle" --from @knowledge/references/authoring/guide.yaml
okf add guides/example "$demo_root/bundle" --type Guide --title "Example guide" --description "A structured authoring example" --set audience=user --body "A guide for readers."
okf edit guides/example "$demo_root/bundle" --patch @knowledge/references/authoring/concept-patch.json --dry-run
okf edit guides/example "$demo_root/bundle" --patch @knowledge/references/authoring/concept-patch.json
okf show guides/example "$demo_root/bundle"
okf validate "$demo_root/bundle"
okf lint "$demo_root/bundle" --fail-on error
```

The definition constrains `audience` to user or contributor. The patch first tests that it is
`user`, then changes it to `contributor` and adds tags. Repeating the patch should fail its test
without changing the document. Query selectors and concrete JSON Pointer patch paths are
different languages; `/audience` addresses one frontmatter property.

The small definition introduces fields without unrelated dependencies. The manual's
[actual ontology](../ontology.yaml) extends this idea with Guide-to-Command `uses` and
Guide-to-Topic `explains` references. Inspect those real edges in the [bundle tour](tour-the-bundle.md).
See [ontology and relationships](../concepts/ontology-and-relationships.md) for the model,
[edit](../commands/edit.md) for mutation flags, and [selectors](../reference/selectors.md) for reads.

## Add a relationship you can query

Continue in the same shell. Define a small command concept and declare that guides can use
commands. The relationship supplies readable forward and inverse labels:

```sh
okf ontology add Command "$demo_root/bundle" --description "A command described by a guide"
okf add commands/check "$demo_root/bundle" --type Command --title "Check" --description "A sample check command" --body "Inspect the sample bundle."
okf ontology update Guide "$demo_root/bundle" --ref 'uses:Command:0..n' --relationship-yaml 'uses={reference: uses, kind: uses, inverse: used-by}'
okf edit guides/example "$demo_root/bundle" --set-yaml 'uses=[/commands/check]'
okf links guides/example "$demo_root/bundle" --details
okf backlinks commands/check "$demo_root/bundle" --details
okf search "$demo_root/bundle" --type Guide --expand uses --target-field title --json
okf lint "$demo_root/bundle" --fail-on error
```

Expect a `uses` edge to `/commands/check`, an incoming `used-by` view from the guide, and
related command metadata in the expanded query. The reference declaration creates the edge;
a custom string that merely looks like a path would not do so by itself.

An inverse label presents the incoming view of the same assertion. It does not author a
second link in the command, grant consent, or establish that anyone ran the command.
The manual's own `Guide.uses` rule follows the same pattern.

## Choose an edit form

| Change | Use | Check before applying |
| --- | --- | --- |
| Replace a whole list or object | `--set-yaml` | The supplied value replaces the entire field. |
| Change an object property | `--set-path` | Intermediate maps may be created; lists need a concrete patch. |
| Edit a list item or require an old value | `--patch` | JSON Pointer addresses are concrete; a failed `test` prevents publication. |
| Remove a metadata key | `--unset` or `--unset-path` | Deletion differs from assigning null. |
| Change the explanation | `--set-body` or section flags | Review the prose and its references, not only schema validity. |

For this exercise the guarded patch changes `audience` from `user` to `contributor` and
adds `worked-example` and `authoring` tags. A second application fails its initial test
because the audience has already changed; the file should remain unchanged. A wrong audience
value is an ontology issue, not proof that the Markdown format itself is invalid.

Keep the temporary directory until you have inspected the results. None of these commands
creates a review event or modifies the checked-in manual.
