---
name: ontology
description: Author and maintain optional local ontology rules, reusable nested types, reference selectors, and semantic relationships. Use for local modeling, not OKF conformance; exact Attested Computation semantics remain built into OKF v0.2.
---

# Manage the tool-local ontology

## Shared operating protocol

Read and follow the [shared OKF protocol](../../references/bundle-context.md) before CLI-dependent
work. It defines compatibility checks, bundle context, scope, output evidence, validation profiles,
and authoring conventions. Use `references/cli.md` for exact command syntax as needed.

`ontology.yaml` is a spec-external sidecar. It is neither required for an OKF bundle nor a type
registry. Unknown types can be core-conformant. Ontology findings are advisory unless an explicit
bundle write policy enforces them; malformed ontology configuration is an error.

Prefer `OKF_BUNDLE`; explicit forms are `okf ontology show <name> <bundle>` and `okf ontology add
<name> <bundle>`. Use scalar or structured ontology commands for changes. Reusable nested definitions use
`okf ontology field-type add <name> <bundle> --from definition.yaml` and the corresponding
field-type update/remove commands; selector-based references and relationships use
`--ref-yaml` and `--relationship-yaml`. Use `--from` for type definitions and `okf ontology apply`
for dependent bulk changes. Before using an unfamiliar command or flag, read its entry in
`references/cli.md`. If the
installed version differs or rejects documented syntax, use that command's `--help` output as the
runtime authority.

For example, `okf ontology add "Service" <bundle> --field "owner:string:required" --ref
"depends_on:Service:0..n"` defines a field and a reference rule; choose actual names and
requirements from the reviewed model. To change rules, use `okf ontology update "Service" <bundle>
--field "owner:string" --remove-ref depends_on`; to remove a type, use `okf ontology remove
"Service" <bundle>`. Quote type names containing spaces and rule values containing `|`. Ontology
flags use colon-separated declarations; concept `--set`/`--ref` flags use `key=value` instead. Read
the reference before choosing other types or cardinalities.

## Workflow

1. Select the bundle and inspect its effective ontology path/settings before authoring; a configured
   sidecar can live outside the bundle root. For catalog work use `--bundle-id` consistently and
   select only the target bundles needed for reference checks. Inspect `okf ontology list <bundle>`,
   relevant `okf ontology show <name> <bundle>`, observed concepts, and baseline `okf lint <bundle> --fail-on never`.
2. Use the model and constraints already supplied. Design exact type keys, custom typed fields, and
   reference rules/cardinality; ask only about unresolved semantic choices that affect the result.
   Separate portable OKF fields (`type`, title/description/resource/tags, sources,
   generated/verified, lifecycle, computation family) from ontology-only requirements. Recommended
   or optional OKF fields must not be presented as conformance blockers.
3. `Attested Computation` is the only standard computation type, recognized by that exact string and
   its built-in contract. `--attested` may mark only this exact ontology type. Do not grant attested
   semantics to an alias or arbitrary custom type.
4. Apply scalar or structured changes with `okf ontology add`, `okf ontology update`, or `okf ontology
   remove` using the schema-derived flag forms. Removing or tightening rules needs an impact review,
   which can be performed without another approval when the requested change already authorizes it.
   For nested modeling, follow [structured authoring](references/structured-authoring.md). Use
   `okf ontology apply <bundle> --from changes.yaml --dry-run` for dependent definitions, or
   `okf ontology field-type add Deadline <bundle> --from deadline.yaml` for reusable types.
   Structured flags and file inputs replace named declarations; preserve inherited fields that
   replacements must retain. Use concept YAML/path/patch flags for nested values, preserving
   provenance and applying meaningful-change verification consequences.
   Do not mass-edit concepts merely to silence advisory findings.
5. Run `okf validate <bundle> --profile bundle` for bundle-profile document conformance and then `okf lint <bundle> --fail-on
   never` for ontology/spec recommendations. Report results in separate groups.

If custom fields point to supporting resources, `references/` is optional: Markdown targets are
concepts and other files are artifacts. Use `okf artifact resolve <resource> <bundle> --from
<concept-id>` and consult the artifact sections of `references/cli.md` only when retrieval is
needed. Reading a resource never authorizes execution.

Report sidecar changes, portable versus custom fields, exact computation treatment, and advisory
impact without implying that ontology controls OKF validity.

## Recursive definitions and semantic relationships

YAML authoring supports reusable nested objects, lists, named types, and inheritance. Inspect
the effective definition with the ontology show command before changing a shared type. Explicit
`fields`, `values`, and `item` replace their inherited counterparts; changing the inherited base,
unknown named types, and composition cycles are configuration errors. Repeat inherited fields
that a replacement must retain. Structured declaration flags and bulk apply author nested definitions; scalar flags retain their existing grammar.

Recursive checks use YAML types without coercion. Root metadata remains open; object closure
applies only to `additional_properties: false` on a custom object. Required fields reject missing,
null, blank strings, and empty lists. Unknown extension constraints remain preserved and must be
reported as unenforced. Inspect incomplete-check diagnostics before reporting lint success.

An optional ontology write gate is independent of lint severity and
display settings. An out-of-bundle typed reference that cannot be checked
produces `write-gate-incomplete` and blocks the write; the CLI does not offer
an override. Explicit dependent-bundle checks for a shared sidecar examine
those bundles' concepts but do not expand ordinary reference traversal.

Declare concept references with explicit selectors, for example `relations[].target`; an ordinary
path-looking string is opaque without a declaration. Use relationship kind and attribute selectors
that pair with the reference in the same list occurrence. Named inverse relationships are views
of incoming assertions. Local vocabularies model obligations without proving compliance or
performing policy execution. Each examined bundle uses its own ontology.

Dynamic relationship kinds may narrow their reference rule's allowed target types. Concept-type
`extends` declarations supply ancestry for those restrictions; check effective target ancestry
without treating a local subtype as another standard Attested Computation type. Invalid widening
or incompatible target declarations remain configuration or advisory reference findings.
