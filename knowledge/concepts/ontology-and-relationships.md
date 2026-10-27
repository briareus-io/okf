---
type: Topic
audience: user
title: Ontology and relationships
description: Use optional local types and explicit references without changing OKF conformance.
sources:
- resource: crates/okf-core/src/ontology/schema.rs
  kind: git-path
---
# Ontology and relationships

An ontology describes local concept types, fields, reference targets and optional relationship
labels. It is a tool extension: unknown concept types remain valid OKF. `lint` checks local
rules; `validate` checks format conformance. An optional write gate can apply local policy.

This manual uses `Guide`, `Topic`, `Reference`, `Command`, `Skill`, and `DesignDecision`.
Guides declare `uses` references to commands and `explains` references to topics. Decisions
link to the topics they constrain. These relationships answer real navigation questions:
which guides use a command, and which decisions explain a behavior?

Relationship rules add a kind and inverse label to a declared reference. Backlinks are an
incoming view of the authored assertion; they do not invent another assertion or imply consent.
Nested selectors retain occurrence pairing when attributes and targets share a list item.

Inspect the [actual ontology](../ontology.yaml), follow the
[modeling guide](../guides/model-your-knowledge.md), and use
[selectors](../reference/selectors.md) for structured queries.
