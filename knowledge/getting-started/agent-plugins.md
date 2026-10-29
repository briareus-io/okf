---
type: Guide
audience: user
title: Use OKF with an agent
description: Install the shared plugin and choose an agent workflow.
---
# Use OKF with an agent

The agent skills ship as an **`okf`** plugin for both Claude Code and Codex. This repository is a
single-plugin marketplace for each agent. Both marketplaces reference the same self-contained
package under `plugins/okf/`, whose `skills/` directory is the canonical source.

### Claude Code

Claude Code marketplace metadata lives in `.claude-plugin/marketplace.json`; the package manifest
lives in `plugins/okf/.claude-plugin/plugin.json`. Claude auto-discovers the package's shared
`skills/` directory.

Install:

```text
/plugin marketplace add briareus-io/okf   # this repo on GitHub, or a full git URL / local path
/plugin install okf
```

### Codex

Codex marketplace metadata lives in `.agents/plugins/marketplace.json`; its package manifest lives
in `plugins/okf/.codex-plugin/plugin.json`. It uses the same `plugins/okf/skills/` directory as
Claude Code.

Install this repository as a marketplace, then install the plugin:

```sh
codex plugin marketplace add briareus-io/okf   # or a full Git URL / local path
codex plugin add okf@okf-marketplace
```

Start a new Codex thread after installation so the skills are loaded.

The skills are namespaced by the plugin in both agents:

| Skill | Claude Code | Codex |
|-------|-------------|-------|
| Start a new bundle | `/okf:init` | `$okf:init` |
| Migrate existing docs | `/okf:migrate` | `$okf:migrate` |
| Ingest a repo | `/okf:ingest` | `$okf:ingest` |
| Answer questions (retrieval) | `/okf:retrieval` | `$okf:retrieval` |
| Manage the ontology | `/okf:ontology` | `$okf:ontology` |
| Infer an ontology | `/okf:infer-ontology` | `$okf:infer-ontology` |
| Sync docs after changes | `/okf:update` | `$okf:update` |
| Review & verify documents | `/okf:review-verify` | `$okf:review-verify` |
| Repair or upgrade a bundle | `/okf:repair` | `$okf:repair` |
| Reorganize the tree | `/okf:reorganize` | `$okf:reorganize` |

The skills drive the `okf` CLI, so install the binary too (`cargo install --path crates/okf-cli`,
or `cargo xtask install`). Each skill carries a generated `references/cli.md` containing only its
workflow's commands and reads it when exact flags or output shapes are needed. If the installed
binary version differs from the generated reference, command-specific `--help` is authoritative.
Run `cargo xtask skills` and `cargo xtask docs --check` in CI to validate authored command examples,
reference coverage, and generated documentation.

Start with [installation](installation.md) for a compatible CLI. Choose [retrieval](../skills/retrieval.md) for questions, [ingest](../skills/ingest.md) for repository research, or [update](../skills/update.md) after source changes. See the [workflow index](../skills/index.md) for all skills.
