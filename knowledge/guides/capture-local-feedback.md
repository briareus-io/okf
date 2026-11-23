---
type: Guide
audience: user
uses:
- /commands/learn-draft
- /commands/learn-export
- /commands/learn-status
- /commands/learn-report
- /commands/learn-enable
- /commands/learn-disable
- /commands/learn-purge
title: Capture local authoring feedback
description: Turn a defect, feature request, guidance gap, or workflow lesson into a private draft and export a reviewed copy.
sources:
- resource: crates/okf-cli/src/learn.rs
  kind: git-path
- resource: plugins/okf/skills/feedback/SKILL.md
  kind: git-path
---
# Capture local authoring feedback

If you encounter a problem or find a useful workaround, you can write it down while the
details are fresh. Drafting works outside a bundle, with counting disabled, and after a
successful command. The [feedback skill](../skills/feedback.md) can help turn the experience
into a fictional example that preserves the behavior you want to describe.

## Create a draft

Choose `defect`, `feature-request`, `guidance-gap`, or `workflow-lesson`. For example, to request a
simpler way to repeat a query, run:

```sh
okf learn draft feature-request
```

The command prints a path and a Markdown template. Open that draft in your editor. Describe
the task in invented terms, the steps the author takes today, and the result you want. For
example, a fictional team might run two backlink commands to find notes about blocked features
and want to save the sequence under a name. Replace real project names, paths, identifiers,
logs, personal details and usage counts with fictional details. For a defect, include the
smallest synthetic reproduction. For a lesson, explain when the workaround applies.

## Preview and export the reviewed text

The CLI accepts the basename of a managed draft and the path of a new destination file.
Substitute the basename from the previous command:

```sh
okf learn export draft-AbCd12.md --output reviewed-feedback.md
```

Read the complete text and resolved destination printed by the preview. Once you approve both,
use the confirmation digest from that preview:

```sh
okf learn export draft-AbCd12.md --output reviewed-feedback.md --confirm "DIGEST_FROM_REVIEWED_PREVIEW"
```

The second command writes a local copy. If the draft or destination changes, preview again.
Choose a new destination if a file already exists. You decide separately whether to send the
exported file to anyone.

## Optionally count recurring CLI outcomes

Use `okf learn status` to see the active store, consent state, draft count, and managed bytes.
To opt into aggregate counting, run `okf learn enable` yourself in a terminal and type
`enable` at its prompt. The report covers recognized outcomes for a small set of command
families, including lint, validate, move, changeset apply, and ontology apply.

```sh
okf learn report
```

The report ranks known patterns by affected invocations. With collection disabled and no prior observations, the report can be empty. To start a draft for a specific registered pattern:

```sh
okf learn draft defect --pattern broken-link
```

Choose a registered pattern relevant to your experience. A report row gives the number of
affected invocations and the qualifying invocation count for that CLI version and command
family. Repeated attempts can contribute separate observations. The counters contain fixed
outcome identifiers. Edit the draft's fictional scenario to describe the issue you want
reviewed.

Use `okf learn disable` to stop collecting while retaining counts and drafts. The state is
shared across checkouts for the current OS user. To choose a different store, set
`OKF_FEEDBACK_DIR` to an absolute UTF-8 directory path. Existing drafts stay in their
original store. When you want to delete all managed drafts and counters in the active store,
run `okf learn purge`. Purge also disables counting. Copies exported elsewhere remain yours.
