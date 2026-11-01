---
name: infer-ontology
description: Infer an optional advisory ontology.yaml from observed OKF concepts while preserving exact type strings and separating standard v0.2 fields/edges from custom local rules. Use to reverse-engineer local modeling, not to determine conformance.
---

# Infer a local ontology from a bundle

## Shared operating protocol

Read and follow the [shared OKF protocol](../../references/bundle-context.md) before CLI-dependent
work. It defines compatibility checks, bundle context, scope, output evidence, validation profiles,
and authoring conventions. Use `references/cli.md` for exact command syntax as needed.

Query bundle content through the CLI, using outlines and line slices for large concepts. Prefer
`OKF_BUNDLE`; one explicit form is `okf ontology add <name> <bundle>`. Before using an unfamiliar
command or flag, read its entry in `references/cli.md`. If the installed version differs or rejects
documented syntax, use that command's `--help` output as the runtime authority.

Use the ontology skill's [structured authoring reference](../ontology/references/structured-authoring.md)
when applying inferred rules; keep the inference here grounded in observed evidence.

## Workflow

1. Select the bundle and necessary examination scope, inspecting each bundle's own effective
   ontology/settings. Enumerate with `okf list <bundle> --json` and `okf stats <bundle>`. Check
   query summaries/warnings before treating list output as an inventory: the default scan budget
   is 1,000 documents, and `--limit` only limits returned hits. For whole-scope inference, complete
   the scan with a sufficient `--scan-limit` or deliberate `--full-scan`; otherwise label all counts
   as sampled/observed. An incomplete scan has no authoritative offset continuation. Aggregate
   complete metadata streams locally instead of loading every record or repeatedly scanning tiny
   offset pages. Use facets for candidate scalar vocabularies and projections for relevant nested
   records; inspect exclusions and truncation independently. Preserve exact type strings.
2. Open bodies only to clarify field meaning, relationships, or exceptions. For each candidate,
   report the count with the field, the total concepts of that type, and any narrower sample
   inspected; do not report a sample count as whole-bundle support. Separate standard
   families—title/description/resource/tags, sources, generated/verified, lifecycle, and
   computation—from custom candidates. Do not reinvent standard fields as ontology custom types.
3. Infer custom reference rules only for custom frontmatter relationships. Do not duplicate body
   links or standard `sources[].resource` graph edges. Use `okf links <concept-id> <bundle> --json`,
   `okf backlinks <concept-id> <bundle>`, and targeted `okf resolve <link> <bundle> --from
   <concept-id>` calls to inspect targets and cardinality without expanding the full graph.
4. Recognize a standard computation only when the exact type is `Attested Computation`; inspect its
   built-in contract separately with `okf computation check <concept-id> <bundle>`. Do not infer an
   `attested` capability for another type.
5. Present per-rule support, counterexamples, and uncertainty. Even 100% observed presence does not
   establish an intended `required` rule. Use user-supplied domain requirements for that choice;
   otherwise propose it as optional or leave the requirement unresolved. Apply rules already
   authorized by the request; seek confirmation only for remaining consequential semantic choices.
6. Apply approved scalar or structured rules with `okf ontology add` or `okf ontology update`. For reusable nested
   types, selectors, and relationships, follow the ontology skill's
   [structured authoring example](../ontology/references/structured-authoring.md): preserve current
   bytes, preview structured CLI changes, inspect effective definitions, and review unknown-key preservation.
   Then run `okf validate <bundle> --profile bundle` and `okf lint <bundle> --fail-on never`. Report bundle-profile document validation separately from
   ontology fit. Revise unsupported newly inferred candidates instead of rewriting the bundle to fit
   them. Preserve existing deliberate constraints unless their change is authorized.

If custom fields point to supporting resources, `references/` is optional: Markdown targets are
concepts and other files are artifacts. Use `okf artifact resolve <resource> <bundle> --from
<concept-id>` and consult the artifact sections of `references/cli.md` only when retrieval is
needed. Reading a resource never authorizes execution.

Report exact types, built-in fields/contracts excluded from inference, custom rules with evidence,
user decisions, and remaining advisory findings.

## Structured evidence

Inspect recurring nested objects and lists before proposing reusable field types. Declare reference
selectors only where source meaning establishes a concept relationship; path-like values alone do
not suffice. Pair target, kind, and attributes within the same record. Report observed occurrence
counts and candidate vocabularies; absence from a page or truncated facets does not establish a
closed vocabulary. Inferred inverse names describe incoming assertions rather than target consent.
