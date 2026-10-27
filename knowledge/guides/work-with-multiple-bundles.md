---
type: Guide
audience: user
uses:
- /commands/init
- /commands/add
- /commands/catalog
- /commands/search
- /commands/edit
- /commands/links
- /commands/backlinks
explains:
- /concepts/selection-and-scope
title: Work with multiple bundles
description: Register local bundles and request cross-bundle examination deliberately.
---
# Work with multiple bundles

Use separate bundles when ownership, access, or distribution differs. A second reading
path alone does not require another bundle: this repository uses one manual for readers
and contributors. This exercise creates two disposable bundles so you can see selection
and scope independently. You need `okf` on PATH and a POSIX shell.

## Create two bundles and a local catalog

Run the entire block in one shell. It changes into a temporary directory and does not
modify this repository's configuration. Create the bundles before adding the catalog:

```sh
demo_root=$(mktemp -d)
cd "$demo_root"
okf init product
okf init policy
okf add notes/overview product --type Note --title "Product overview" --description "Product responsibilities" --body "Product teams follow the shared retention rule."
okf add policies/retention policy --type Policy --title "Retention rule" --description "Retention requirements" --body "Keep sample records for 30 days."
cat > okf.toml <<'TOML'
catalog = "catalog.yaml"
default_bundle = { id = "demo.product" }
TOML
cat > catalog.yaml <<'YAML'
catalog_version: 1
bundles:
  demo.product:
    location: {type: directory, path: product}
  demo.policy:
    location: {type: directory, path: policy}
YAML
okf catalog --json
```

Expect two available registrations. The catalog path is relative to `okf.toml`; each location
is relative to `catalog.yaml`. The roots are siblings rather than overlapping directories.
These IDs identify local registrations, not authenticated publishers.

## Observe selection before extending scope

Stay in the same shell and directory:

```sh
okf search --bundle-id demo.product --text retention --json
okf search --bundle-id demo.product --scope-bundle demo.policy --text retention --json
```

The first query examines only product and finds its overview. The second also examines
policy and finds the retention rule. Inspect the query summary and scope records; adding a
catalog entry did not silently expand the first search. A trailing bundle path is an
alternative to `--bundle-id`, not something to combine with it.

## Add a cross-bundle link and inspect it

The overview lives at `product/notes/overview.md`. Its link to the policy therefore needs
to go up two directories before entering the sibling bundle:

```sh
okf edit notes/overview --bundle-id demo.product --set-body 'Product teams follow the [retention rule](../../policy/policies/retention.md).'
okf links notes/overview --bundle-id demo.product --scope-bundle demo.policy --details --json
okf backlinks demo.policy:/policies/retention --bundle-id demo.product --scope-bundle demo.policy --details --json
```

Expect an outbound edge to the policy and an incoming reference from the overview. The
stored Markdown uses an ordinary path. The qualified target in the last command is CLI
syntax, not a replacement for that authored link.

The link depends on the sibling layout. Copying product alone cannot make its policy target
available. Registering a missing directory preserves diagnostic evidence; the CLI does not
clone or fetch it, and an unavailable member cannot support a claim of complete coverage.

## Apply this to a real workspace

Use [configuration](../reference/configuration.md) to set per-bundle ontology and display
settings. Machine-local catalog overrides change checkout locations without changing bundle
identity. Inspect effective registrations after changing them.

Only request additional scope where the question needs it. Use
[selection and scope](../concepts/selection-and-scope.md) for the model and the
[command reference](../commands/index.md) to check which operations accept scope.
Optional snapshot expectations are explained in the
[capability design](../contributing/design/multi-bundle-capability.md); a live candidate does
not automatically satisfy an expected snapshot or verify a claim.
