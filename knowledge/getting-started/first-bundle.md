---
type: Guide
audience: user
uses:
- /commands/init
- /commands/add
- /commands/show
- /commands/validate
explains:
- /concepts/bundles-and-concepts
title: Create your first bundle
description: Create, read, search and validate a small bundle using explicit paths.
---
# Create your first bundle

Install the [CLI](installation.md), then run these commands in a scratch directory.
Passing the bundle on every command keeps the example independent of local defaults.

## Create the bundle

```sh
okf init mybundle
```

This creates the bundle directory and its initial index.

## Add a concept

```sh
okf add notes/hello mybundle --type Note --title "Hello" --description "A first note" --body "Hello from OKF."
```

The new file is `mybundle/notes/hello.md`. Its frontmatter contains the type, title, and
description. Its Markdown body contains the greeting.

## Find and read the note

```sh
okf browse mybundle
```

This displays the root index, which was created before the note. To list the concepts that
currently exist, run:

```sh
okf list mybundle
```

The list includes `/notes/hello`. You can also find it by text:

```sh
okf search mybundle --text Hello
```

The same note matches by title and body. Read the whole concept by ID:

```sh
okf show notes/hello mybundle
```

The output includes the frontmatter and the line `Hello from OKF.`.

## Validate the bundle

```sh
okf validate mybundle
```

Expect no conformance violations. OKF allows the authored type `Note`. An optional ontology
can add modeling rules and advisory checks.

Next, [find and read knowledge](../guides/find-and-read-knowledge.md),
[model your knowledge](../guides/model-your-knowledge.md),
[customize its settings and guidance](../guides/customize-a-bundle.md), or
[tour this repository's live bundle](../guides/tour-the-bundle.md).
