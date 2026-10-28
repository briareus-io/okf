# okf — a harness for the Open Knowledge Format

`okf` works with directories of Markdown knowledge: create and query concepts, follow
relationships, track source changes, and maintain documentation with explicit review evidence.
It provides readable CLI output for people and NDJSON for programs and agents. Agent skills
supply the judgment around its deterministic operations.

The [knowledge bundle](knowledge/index.md) is the user manual, CLI reference, contributor
guide, and a working example of OKF. [Tour the bundle](knowledge/guides/tour-the-bundle.md)
to query its real relationships, inspect artifacts, and trace source impact.

## Install

**Breaking change in CLI 0.5.0:** read the [migration guide](knowledge/reference/migrations/0.5.0.md)
before updating a 0.4.x installation or automation. Install CLI and plugin together;
the 0.5.2 plugin supports stable CLI versions `>=0.5.0, <0.6.0`. The document format remains OKF v0.2.

Download the installer from the published release, inspect it, then run it:

```sh
curl --fail --location --proto '=https' --proto-redir '=https' \
  https://github.com/abatyuk/okf/releases/download/v0.5.2/install.sh -o install.sh
# Review install.sh before running it.
sh install.sh 0.5.2
export PATH="$HOME/.local/bin:$PATH"
okf version
```

See [installation](knowledge/getting-started/installation.md) for archive downloads,
checksum verification, supported platforms, and updates. See
[agent plugins](knowledge/getting-started/agent-plugins.md) for Claude Code and Codex setup.

## Try it

Run in a scratch directory. Explicit bundle paths keep the example independent of local settings.

```sh
okf init mybundle
okf add notes/hello mybundle --type Note --title "Hello" --description "A first note" --body "Hello from OKF."
okf search mybundle --text Hello
okf show notes/hello mybundle
okf validate mybundle
```

## Find your next step

- [Getting started](knowledge/getting-started/index.md): installation and your first bundle.
- [Task guides](knowledge/guides/index.md): retrieve, model, maintain, reorganize, and automate.
- [Concepts](knowledge/concepts/index.md): provenance, verification, ontology, and scope.
- [CLI commands](knowledge/commands/index.md) and [shared reference](knowledge/reference/index.md).
- [Agent workflows](knowledge/skills/index.md): choose the skill for your task.
- [Contributing](knowledge/contributing/index.md): development, architecture, decisions, and releases.
