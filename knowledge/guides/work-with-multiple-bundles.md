---
type: Guide
audience: user
uses:
- /commands/query
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

A product overview refers to a retention rule owned by a policy team. You can keep each
team's knowledge in a separate bundle and still inspect the relationship. Start in an empty
working directory. Run each command from the directory that will contain the two bundles.

## Create two bundles and a local catalog

Create the product bundle.

```sh
okf init product
```

Create its sibling, the policy bundle.

```sh
okf init policy
```

Give each bundle a concept containing the word “retention”. That shared term lets you see
which bundles a search examined.

```sh
okf add notes/overview product --type Note --title "Product overview" --description "Product responsibilities" --body "Product teams follow the shared retention rule."
```

Add the rule to the policy team's bundle.

```sh
okf add policies/retention policy --type Policy --title "Retention rule" --description "Retention requirements" --body "Keep sample records for 30 days."
```

Register the two sibling directories in a workspace catalog.

**File: `okf-catalog.yaml`**

```yaml
catalog_version: 2
default_bundle: demo.product
bundles:
  demo.product:
    location: {type: directory, path: product}
  demo.policy:
    location: {type: directory, path: policy}
```

Inspect the registrations.

```sh
okf catalog --json
```

Expect two available registrations. The CLI discovers `okf-catalog.yaml` from the working
directory upward. Each location is relative to the catalog. Use distinct, non-overlapping
roots for separate registrations. For an older workspace configured with TOML, see the
[migration guide](../reference/migrations/0.6.0.md).

## Observe selection before extending scope

Search the product bundle. Its overview should be the only matching concept.

```sh
okf search --bundle-id demo.product --text retention --json
```

Now include the policy bundle in examination scope.

```sh
okf search --bundle-id demo.product --scope-bundle demo.policy --text retention --json
```

Expect a second result, the retention rule. The query summary and scope records identify the
examined bundles and the number of matches. To select by path instead of catalog ID, give
search the bundle directory as its positional argument.

## Add a cross-bundle link and inspect it

Use a logical authored reference when the link should follow catalog identity even if a checkout
moves. Update the overview to link to the policy by catalog ID.

```sh
okf edit notes/overview --bundle-id demo.product --set-body 'Product teams follow the [retention rule](okf://demo.policy/policies/retention.md).'
```

Inspect the overview’s outgoing link. Its target belongs to `demo.policy`.

```sh
okf links notes/overview --bundle-id demo.product --scope-bundle demo.policy --details --json
```

Starting from that policy, find its referring concepts.

```sh
okf backlinks demo.policy:/policies/retention --bundle-id demo.product --scope-bundle demo.policy --details --json
```

Expect an incoming reference from the overview. The stored Markdown uses `okf://demo.policy/policies/retention.md`. The qualified target in the
last command is CLI syntax, not an alternative spelling for the authored URI.

An ordinary relative link, `../../policy/policies/retention.md`, is also valid from the overview
and works in repository Markdown viewers when the sibling layout is preserved. Logical links
remove that layout dependency for OKF-aware readers, but still require the catalog registration
and available target. See [remote dependencies](remote-dependencies.md) for locked Git registrations.

Copying product alone cannot make its policy target available. Registering a missing directory
preserves diagnostic evidence; the CLI does not clone or fetch it, and an unavailable member cannot
support a claim of complete coverage.

## Run a saved query across bundles

To repeat the policy lookup, define the query in the product bundle. Its single outgoing hop
keeps concepts of type `Policy` and returns their titles.

**File: `product/okf-bundle.yaml`**

```yaml
manifest_version: 1
settings:
  saved_queries:
    referenced-policies:
      from: concept
      steps:
        - links: {related_type: [Policy]}
      select: [title]
```

Start at the product overview and include the policy bundle in scope:

```sh
okf query referenced-policies notes/overview --bundle-id demo.product --scope-bundle demo.policy --json
```

Expect one projection with bundle `demo.policy`, ID `/policies/retention`, and title
`Retention rule`. The primary bundle supplies the definition and starting ID. The scope flag
allows the hop to read the target in the second bundle.

## Record the policy as a source

A source can carry both an ordinary resource path and the catalog identity it is intended
to address. From the same scratch directory, save this mapping. An ordinary relative resource and a qualified target point to the same file.

**File: `retention-source.yaml`**

```yaml
id: retention
resource: ../../policy/policies/retention.md
bundle_ref:
  id: demo.policy
  path: policies/retention.md
```

Attach this source to the overview.

```sh
okf edit notes/overview --bundle-id demo.product --add-source-json @retention-source.yaml
```

Inspect the source edge alongside the body link.

```sh
okf links notes/overview --bundle-id demo.product --scope-bundle demo.policy --details --json
```

The relative resource starts at `product/notes/`. The `bundle_ref.path` starts at the
policy bundle root. They should resolve to the same file. Inspect the source edge's
`status`, `evidence`, and qualified target. If the paths disagree, correct the mapping
before citing it. For a URL with a declared local mapping, an unchecked-equivalence result
means you still need independent evidence that the two resources correspond.

For a version-specific source, add a `snapshot` mapping under `bundle_ref`. An exact
`commit` identifies a Git object and a SHA-256 `digest` identifies the referenced file's
raw bytes. A `ref` or `label` can help locate the intended version. Inspect the snapshot's
status, resolved identity, and separately labeled current candidate before opening the
evidence. A missing local revision or mismatched digest means you cannot claim a match.
Scope records identify any historical target examined separately. An ordinary Markdown
link in the same document still follows its own destination.

## Apply this to a real workspace

Use [configuration](../reference/configuration.md) to set per-bundle ontology and display
settings. Machine-local catalog overrides change checkout locations without changing bundle
identity. Inspect effective registrations after changing them.

Only request additional scope where the question needs it. Use
[selection and scope](../concepts/selection-and-scope.md) for the model and the
[command reference](../commands/index.md) to check which operations accept scope.
[Sources and provenance](../concepts/sources-and-provenance.md) explains snapshot expectations;
a live candidate does not automatically satisfy an expected snapshot or verify a claim.
