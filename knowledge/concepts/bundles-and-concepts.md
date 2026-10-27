---
type: Topic
audience: user
title: Bundles and concepts
description: Understand Markdown concepts, path identities, indexes and opaque artifacts.
sources:
- resource: crates/okf-core/src/model/concept.rs
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
are usable examples, while `ontology.yaml` is the optional local modeling sidecar.

Use ordinary relative Markdown links so pages also work in repository viewers. In OKF,
a leading `/` means the bundle root, not the operating system root. Metadata creates a
custom graph edge only when the ontology declares a reference; a path-looking string alone
is opaque. [Selection and scope](selection-and-scope.md) explain multi-bundle boundaries.

[Create a first bundle](../getting-started/first-bundle.md) or inspect this manual through
[browse](../commands/browse.md), [show](../commands/show.md), and [artifact show](../commands/artifact-show.md).
