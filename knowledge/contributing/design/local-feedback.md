---
type: Reference
audience: contributor
title: Local authoring feedback
description: Consent, observation, drafting, storage, export, and compatibility contracts for authoring feedback.
sources:
- resource: crates/okf-cli/src/learn.rs
  kind: git-path
- resource: plugins/okf/skills/feedback/SKILL.md
  kind: git-path
---
# Local authoring feedback

Authors can capture a defect, feature request, guidance gap, or workflow lesson even when every
command succeeds. The feedback skill helps an author express the goal, friction, workaround, and
acceptance scenario using invented data. An optional local report supplies evidence about recurring
CLI outcomes. The user decides whether to retain a draft or export it to a file.

## Ownership and compatibility

Feedback is a CLI and plugin capability. It requires no change to the format, ontology, catalog,
[manifest](bundle-manifest.md), or bundle. The CLI dispatches feedback commands before bundle
discovery. Their only applicable global option is `--json`. JSON uses the normal operation envelope
with a `feedback` payload containing the action and its data. Command metadata, arguments, and
effects are discoverable through the schema and [command reference](../../commands/index.md).

Installation and ordinary use create no feedback files by default. Only the explicit enable
command asks for consent, on terminal input and terminal stderr. A person must type `enable`.
The skill must never enable counting. A terminal is a consent interface and cannot authenticate
a human against another process with equivalent access. Older CLIs can use the plugin's bundled
template manually. Existing integrations can continue to use their original commands, stdout,
stderr and exit codes.

## State ownership and retention

The store belongs to one OS user and aggregates across checkouts. Its root is
`~/Library/Application Support/okf/feedback` on macOS, `%LOCALAPPDATA%/okf/feedback` on Windows,
and `$XDG_STATE_HOME/okf/feedback` on other systems with an absolute state base, defaulting to
`~/.local/state/okf/feedback`. The status command returns the resolved root, consent, draft count,
and number of managed data bytes. POSIX directories are owner-only and draft and state files use
private temporary files. On Windows the user profile's access controls apply.

| Entry | Contents and lifetime |
| --- | --- |
| `state.json` | Version 1 consent flag and aggregates keyed by CLI version and registered command family. Disable preserves them. |
| `drafts/draft-XXXXXX.md` | Individually editable local items. Creating a draft does not grant consent to count. |
| `lock` | Stable coordination file shared by observers and explicit commands. Its continued existence after purge prevents a second lock domain. |

State changes read, modify, and atomically replace the aggregate file under the same lock.
Observation at command completion takes a nonblocking lock and reads current consent while
holding it. A busy, invalid, or unwritable store costs a sample. Feedback failures never replace
the observed command's result. Explicit commands surface errors. An unknown state schema is
rejected on read, while purge can remove an unreadable aggregate file. Purge removes managed
drafts and aggregate state, implicitly disabling collection. Independently exported copies belong
to the user. Counts and drafts persist across invocations until purged.

## Observation contract

The observer accepts a closed family and closed pattern identifiers. It never serializes a
finding, error message, custom field, concept type, identifier, command line, path, or hash of
project data. A recognized command contributes one denominator increment for a successful command
result or a recognized terminal outcome. Unclassified failures are excluded. A pattern contributes
at most once per invocation. Multiple patterns may describe the same invocation.

| Family | Qualifying observations |
| --- | --- |
| Ordinary lint | A built-in broken-link finding, even below the failure threshold. Exit 1 also records the threshold outcome. Fix and baseline modes are excluded. |
| Validate | Exit 1 records an invalid result. |
| Move | The scoped cross-bundle referrer guard records a specific outcome alongside the policy refusal. |
| Changeset apply and ontology apply | Recognized baseline conflicts, write-gate rejections, or policy refusals when returned by the handler. Ontology apply can also record a declaration finding on a successful run. |

Reports rank affected invocations and show their qualifying invocation denominator for the same
version and family. Families without an observed pattern have no ranked row. Counts are local
evidence about commands, with no claim about distinct users, sessions, retries, time lost, or
the importance of a request. Adding a signal requires a fixed identifier, an identified emission
site or typed result, a denominator definition, and a privacy regression check.

## Draft and export protocol

1. Choose a kind and optionally a built-in pattern from the report. The CLI creates a private
   template and prints its path and contents. The template contains neither actual counts nor
   source material. A selected pattern supplies a fictional starting scenario to verify. The plugin asset `skills/feedback/assets/template.md` is the authoritative Markdown template.
   The docs generator copies it to `crates/okf-cli/templates/feedback.md` for inclusion in the
   standalone binary. The generated-document check detects drift.
2. Write a fresh fictional scenario. A defect needs a synthetic reproduction, a request needs
   a desired outcome, and a lesson needs the conditions where the practice works. Exclude actual
   product names, personal details, paths, identifiers, logs, usage counts and real data. An agent
   that cannot preserve the lesson safely should ask for a generalized account.
3. Pass the managed filename and a chosen destination to the export command. The first call
   returns the exact text, resolved destination and a SHA-256 confirmation over the destination
   bytes, a zero separator and the UTF-8 draft. Review all three locally. A free-text draft
   requires human judgment about identifying details.
4. Repeat with the confirmation digest after approval. The CLI recomputes the digest, rejects a
   changed draft or destination, and atomically creates a new file without overwriting an existing
   one. Export has no network transport. The skill must never file or upload feedback.

## Using the commands

Start a manual request even if counting has never been enabled.

```sh
okf learn draft feature-request
```

Edit the returned Markdown file with the generalized account. For a pattern-based draft,
first inspect the local report and use a matching registered identifier.

```sh
okf learn report
okf learn draft defect --pattern broken-link
```

In this fictional example, replace the draft basename with the one the CLI returned.
The preview writes no destination file. Read the entire text and verify the destination
before using the digest from that preview.

```sh
okf learn export draft-AbCd12.md --output reviewed-feedback.md
okf learn export draft-AbCd12.md --output reviewed-feedback.md --confirm "DIGEST_FROM_REVIEWED_PREVIEW"
```

## Validation

Exercise collection disabled, enabled and purged in an isolated user state directory. Compare
ordinary command results and inspect aggregates with distinctive synthetic input values. Verify
that a successful workflow can produce a manual request without enabling collection. Edit a draft
between preview and confirmation, try a different destination, and attempt a repeated export.
Verify that purging leaves the exported file intact. See the
[feedback skill](../../skills/feedback.md) and [commands](../../commands/index.md).
