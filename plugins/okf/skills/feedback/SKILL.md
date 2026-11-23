---
name: feedback
description: Capture a requested defect report, feature request, guidance gap, or workflow lesson as a private synthetic feedback draft, including when commands succeeded or collection is off.
---

# Capture feedback locally

Start with the outcome the user wanted. Choose defect, feature request, guidance gap, or
workflow lesson. A single observation is enough. Identify the workaround and the observable
improvement. Distinguish an author's account from an agent observation and a synthetic reproduction.

Write from a blank fictional scenario. Exclude actual product and organization names, personal
details, paths, URLs, transcripts, logs, identifiers, usage counts and real project data.
Never copy raw material into a temporary draft to sanitize later. When generalization would
lose the point, ask for a safe abstraction. Use the [template](assets/template.md) and
[worked examples](references/examples.md). The optional skill-local `references/` contains
examples and the [command index](references/cli.md).

For CLI work consult the [shared protocol](../../references/bundle-context.md) for compatibility.
Feedback uses per-user state and needs no bundle context. Check availability of the
[draft command](../../references/commands/learn-draft.md) with its help. With an older CLI,
prepare the synthetic template in conversation. Never enable collection on behalf of the user.
Read a local report only when requested. Use a fixed pattern ID when relevant and keep all actual
counts out of drafts. Use `okf learn draft feature-request` for a successful but cumbersome
workflow. Edit only the created draft, using the sanitized account.

Before finishing, apply the template’s acceptance scenario check. Read the draft as a
maintainer who has never seen the session. Include enough detail for them to investigate
the defect, evaluate the request, or act on the guidance gap or lesson. State what
remains unknown when evidence is missing.

Export only on request to a user-selected destination. Show the full preview and destination
to the user and wait for approval before passing the returned confirmation digest.
Keep the draft local. Never post, upload, or open an external issue for this workflow.
