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

```sh
okf init mybundle
okf add notes/hello mybundle --type Note --title "Hello" --description "A first note" --body "Hello from OKF."
okf browse mybundle
okf list mybundle
okf search mybundle --text Hello
okf show notes/hello mybundle
okf validate mybundle
```

The concept is `mybundle/notes/hello.md`: YAML frontmatter followed by Markdown. Validation
should report no conformance violations. An unknown `Note` type is valid; an optional ontology
adds local modeling and advisory checks. The initial index predates the new note; `browse`
can synthesize a directory index where one is absent.

Next, [find and read knowledge](../guides/find-and-read-knowledge.md),
[model your knowledge](../guides/model-your-knowledge.md), or
[tour this repository's live bundle](../guides/tour-the-bundle.md).
