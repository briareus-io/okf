---
type: Topic
audience: user
title: Bundles and concepts
description: Understand concept identities, navigation, artifacts, and the bundle support namespace.
sources:
- resource: crates/okf-core/src/model/concept.rs
  kind: git-path
- resource: crates/okf-core/src/bundle/walk.rs
  kind: git-path
- resource: crates/okf-core/src/check/validate.rs
  kind: git-path
---
# Bundles and concepts

A bundle is a directory of Markdown concepts. Each concept has parseable YAML frontmatter
with a non-empty `type`. Its ID is its bundle-relative path without `.md`; renaming that path
changes its identity. A title and description make discovery useful but are not additional
format conformance requirements.

`index.md` is navigation, not an ordinary typed concept. Use it to introduce tasks and link
readers to smaller pages. This manual's root offers user and contributor entry points.
Non-Markdown files can be opaque artifacts: the YAML and JSON files under `references/`
are usable examples. `ontology.yaml` is the optional local modeling sidecar;
`okf-bundle.yaml` carries portable settings, guidance, and workflows. A workspace catalog
registers bundle IDs and locations; a standalone bundle does not require one.

## Standard frontmatter fields

| Field or family | Meaning in OKF v0.2 |
|-----------------|---------------------|
| `type` | Required non-empty concept type. Bundles can define their own type names. |
| `title`, `description`, `tags`, `resource` | Optional discovery metadata for the display name, one-line summary, tags, and represented resource. |
| `status`, `stale_after` | Optional document lifecycle and freshness. Explicit status values are `draft`, `stable`, and `deprecated`. An absent status means `stable`. |
| `sources`, `usage_window` | Provenance entries identify sources by resource. A usage window can give the time interval for source usage counts. |
| `generated`, `verified` | Generation and verification evidence, including actors and timestamps. |
| `runtime`, `parameters`, `computation`, `executor`, `attester` | Fields of the exact `Attested Computation` contract. See [computation check](../commands/computation-check.md) for its requirements and inline computation form. |

A local ontology can require an explicit lifecycle status or narrow its values to a subset
such as `[draft, stable]`. For a workflow with states such as `open` and `closed`, use a custom
field such as `workflow_state`. This manual uses `decision_status` for `accepted` and
`superseded`. A nested field such as `ticket.status` can have its own vocabulary.

## Knowledge and support files

The optional root `.okf-bundle/` directory holds scripts, templates, longer instructions,
and operational reports. Profile-aware knowledge queries exclude this support tree and the
manifest from ordinary discovery. Use [bundle resource](../commands/bundle-resource-list.md)
for explicit inspection. Physical snapshots retain these files.

This exclusion is a tooling profile, not a change to core OKF v0.2. Default
`okf validate` uses the core file boundary, so support Markdown without concept frontmatter
may produce violations. `okf validate --profile bundle` checks knowledge documents under
the tooling boundary; [bundle validate](../commands/bundle-validate.md) checks the manifest.
Older readers may not understand the exclusion. A report intended for ordinary knowledge
search should be a concept outside the support directory.

## References and identities

Use ordinary relative Markdown links when the directory layout is part of the reference.
In OKF, a leading `/` means the bundle root, not the operating system root. An optional
`okf://<bundle-id>/<path>` reference instead resolves through a selected catalog; ordinary
Markdown readers may need exported relative or web links to navigate it. Neither a title
nor a matching filename establishes identity.

Metadata creates a custom graph edge only when the ontology declares a reference; a
path-looking string alone is opaque. [Selection and scope](selection-and-scope.md) explains
cross-bundle examination and snapshot selection. [Customize a bundle](../guides/customize-a-bundle.md)
shows how to add the optional manifest and support resources.

[Create a first bundle](../getting-started/first-bundle.md) or inspect this manual through
[browse](../commands/browse.md), [show](../commands/show.md), and [artifact show](../commands/artifact-show.md).
