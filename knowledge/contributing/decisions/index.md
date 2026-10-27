# Design decisions

Nine choices record context, alternatives, consequences and reconsideration criteria. Boundary decisions were consolidated; preservation now explains both frontmatter and ontology guarantees. The pages describe current choices; source references point to current repository files.

- [Separate core, CLI and judgment](architectural-boundaries.md) — Keep deterministic logic reusable and effects testable.
- [Derive CLI documentation from the command contract](derived-schema.md) — Keep discovery and generated reference tied to the implementation.
- [Derive trust tiers from verification evidence](derived-trust-tiers.md) — Expose recorded review actors without treating labels as proof of truth.
- [Use Git through its CLI](git-cli-not-library.md) — Keep Git an optional runtime dependency behind an effect port.
- [Catalog identity and explicit scope](multi-bundle-scope.md) — Catalog coordination preserves ordinary paths and separates selection, examination and snapshot evidence.
- [Preserve authored data through edits](preserve-authored-data.md) — Retain unknown metadata and distinguish semantic preservation from YAML presentation.
- [Explicit structured mutation inputs](structured-authoring.md) — YAML assignments, object paths and concrete patches extend authoring while preserving literal scalar flags.
- [Declared structured metadata interpretation](structured-metadata.md) — Nested constraints and semantic references remain optional local interpretation rather than OKF conformance.
- [Separate conformance from local quality policy](validate-vs-lint.md) — Keep valid OKF portable across different local modeling choices.
