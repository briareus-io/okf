---
type: Reference
audience: contributor
title: Bundle manifest and guidance
description: Design of portable bundle configuration, agent guidance, workflows, support resources, and explicit query presets.
---
# Bundle manifest and guidance

An optional `okf-bundle.yaml` gives a bundle portable settings and task guidance. Direct users of
OKF skills and higher-level skills consume the same instructions, avoiding copies in agent memory
or wrapper skills. The CLI resolves structured configuration and delivers context; agents interpret
the instructions within the user's task. Authors remain responsible for maintaining that content.

This design defines the configuration and agent-context architecture introduced in CLI 0.6. For
operational examples, use [Customize a bundle](../../guides/customize-a-bundle.md); for field and
command details, use [Configuration](../../reference/configuration.md) and `okf schema`.
[Remote dependencies](remote-bundles.md) extend the catalog without moving consumer policy into
upstream manifests.

## Configuration ownership

```text
workspace/
  okf-catalog.yaml
  knowledge/
    okf-bundle.yaml
    ontology.yaml
    .okf-bundle/
      scripts/
      templates/
      references/
    index.md
    ...concepts...
```

| Location | Responsibility | Path base |
| --- | --- | --- |
| `okf-bundle.yaml` | Bundle description, portable settings, guidance, workflows, query presets | Bundle root |
| `ontology.yaml` | Concept types, fields, references, and relationship semantics | Its documented ontology rules |
| `okf-catalog.yaml` | Registered bundle IDs, locations, default selection, local override pointer, consumer dependency policy | Catalog directory |
| Local override file | Machine-specific registered locations | Override-file directory |
| Explicit invocation | Selected bundle, examination scope, supported temporary options | Invocation directory for CLI paths |

A manifest does not declare an intrinsic bundle ID. A standalone bundle remains usable by path;
a catalog assigns its registered identity. Ontology remains a separate file because modeling rules
have their own structure and authoring operations. A catalog is optional, as is the manifest.

YAML unifies configuration syntax without forcing all configuration into one file. Bundle behavior
travels with the bundle; workspace locations and defaults remain with the workspace. An external
ontology path retained for local compatibility is nonportable and must not be silently copied or
rebased to different content.

## Schema and compatibility

`manifest_version: 1` identifies the schema and its semantics independently of the CLI release.
Several CLI releases can support the same schema, and a CLI may retain multiple readers. `okf
schema` advertises supported manifest, catalog, and lock versions. A schema version changes when an
older reader could not safely interpret the new contract, not for documentation changes or edits
to instruction prose.

The manifest contains optional `description`, `settings`, `guidance`, `workflows`, and `extensions`
sections. Settings include ontology location, query limits, lint severities, write policy, views,
facets, and query presets. Unknown defined-schema fields, duplicate YAML keys, duplicate rule IDs,
unsupported versions, invalid selectors, and invalid typed values are errors. Namespaced
`extensions` hold opaque metadata; they cannot smuggle required behavior past an older reader.

An absent manifest uses supported legacy configuration and built-in defaults. A present but
malformed, unreadable, or unsupported manifest fails normal manifest-dependent work. Failure does
not silently reactivate TOML as a substitute. Diagnostic and raw-file inspection paths remain
available so users can repair configuration.

The manifest is a tooling extension. Its absence does not invalidate a core OKF bundle, and its
instructions cannot redefine core conformance.

## Example manifest settings

A documentation bundle can carry lint policy, reusable views, and explicit query presets together:

```yaml
manifest_version: 1
description: Product documentation and executable CLI examples.
settings:
  ontology: ontology.yaml
  query:
    scan_limit: 1000
  lint:
    source_unrecorded: off
    broken_link: error
  views:
    commands:
      columns: ["$id", title, group, mutates, description]
    guides:
      columns: ["$id", title, audience, description]
      expand: [uses]
  query_presets:
    writing-commands:
      description: Commands that declare authored mutations.
      operations: [search, list]
      query:
        type: Command
        facet_filter: ["mutates=true"]
```

The `commands` view changes presentation; the `writing-commands` preset narrows results only when
explicitly selected. The `guides` view uses the bundle ontology's declared `uses` relationship for
expansion. JSON view projections/expansion require the view's explicit `extended_output: true`
setting; human views do not silently alter ordinary machine results.

Here `source_unrecorded: off` suppresses the missing-baseline warning for a documentation bundle
that preserves provenance without maintaining every fingerprint. It does not disable missing-source
checks, stale checks, or verification requirements. The repository's real manifest preserves its
existing lint policy and both views; the [live-bundle tour](../../guides/tour-the-bundle.md) exercises
them. The [customization guide](../../guides/customize-a-bundle.md) creates a complete scratch bundle
with lint settings, a view, a preset, an instruction file, and a workflow.

## Discovery and effective settings

The selected catalog comes from explicit `--catalog`/`OKF_CATALOG`, otherwise the nearest upward
`okf-catalog.yaml`, otherwise the supported legacy TOML catalog pointer. Ancestor catalogs are not
merged. Conflicting discovered and legacy declarations are diagnosed rather than silently
combined. Explicit standalone access can bypass an unrelated invalid discovered catalog; an
explicitly selected catalog must be valid.

Bundle selection prefers an explicit path or ID, then `OKF_BUNDLE`, a containing registered bundle
or standalone manifest, a configured default, a sole catalog entry, and finally the legacy
working-directory fallback. A containing manifest must agree with registered bundle boundaries.
Equal or nested registered roots are invalid. Selection and examination scope are separate:
registration does not authorize catalog-wide traversal.

Structured settings resolve from built-in defaults, compatible legacy values, manifest settings,
and supported explicit command options. During TOML compatibility, equal old/new values may
coexist; conflicting values fail with their origins identified. Paths retain their original bases
until migration explicitly rebases them. Local overrides change registered locations, not guidance,
write gates, accepted snapshots, or ontology meaning. Explicit options retain their own documented
semantics; an output option is not permission to waive a write gate.

## Guidance model

```yaml
manifest_version: 1
description: Documentation catalog maintained from selected sources.
settings:
  ontology: ontology.yaml
  query:
    scan_limit: 1000
guidance:
  display:
    json: full
    text: summary
  instructions: |
    Preserve source URLs exactly as supplied.
    Fetch URL-valued fields only when explicitly requested.
  rules:
    - id: relationship-evidence
      description: Interpret references without inferring acceptance.
      operations: [links, backlinks]
      instructions: |
        A reference does not establish agreement or acceptance.
    - id: artifact-resolution
      operations: ["artifact resolve"]
      instructions_file: .okf-bundle/references/artifact-policy.md
```

Global guidance applies to work on the selected bundle. A rule's optional `operations` selects
canonical CLI command paths such as `links` or `artifact resolve`, excluding the executable,
arguments, and flags. Matching uses exact command paths; `artifact` is not a wildcard and skill
names are not command names. Any listed operation matches; an omitted list makes the rule global.
An empty list is invalid. Descriptions explain purpose but do not determine whether required
instructions are loaded.

Each instruction-bearing rule or workflow has exactly one of `instructions` and
`instructions_file`. The global body is optional. Instruction files are required readable UTF-8
content, resolved with bundle-relative containment and symlink checks. They are loaded fully when
the associated guidance is selected. Missing or unreadable required files block dependent work.
Ordinary links inside them do not recursively include instructions, and remote instruction
includes are outside this design.

Guidance is local to its declaring bundle. Ordinary concepts are evidence, not instructions merely
because they contain imperative text. Cross-bundle tasks retain the origin of every applicable
rule. Conflicts are surfaced rather than resolved by incidental load order. User instructions and
governing agent constraints retain authority; guidance neither grants additional permissions nor
silently overrides structured CLI settings.

## Context delivery and identity

```text
okf bundle context knowledge
okf bundle context knowledge --operation links --operation backlinks --json
okf bundle context knowledge --workflow import-vendor-spec --inputs @inputs.json --json
okf bundle context knowledge --operation backlinks --query @query.json --json
okf bundle context knowledge --all --json
```

| Request | Content |
| --- | --- |
| No operation/workflow | Effective settings, global instructions, scoped-rule/workflow inventory |
| `--operation` | Global instructions and the union of matching rules |
| `--workflow` | Selected procedure, input definitions/effective values, declared resources and outputs |
| `--query` with an operation | Validated effective query options, without executing the query |
| `--all` | Every instruction and workflow body for inspection |

`--all` cannot be combined with operation/workflow selection. It does not start workflows or mark
all rules applicable. Full context means configuration and guidance, not every concept, source,
script, template, or registered bundle. Ontology location and interpretation identity are exposed;
model definitions remain available through ontology inspection commands.

Scope flags request additional bundle contexts explicitly. Records identify their bundle and root;
unavailable requested members produce incomplete-scope evidence. Primary workflow, input, and
query selections do not automatically activate similarly named workflows in secondary bundles.
Secondary context supplies the selected command guidance and inventory.

Context records distinguish `matches_selection` from `instruction_role`. Current task guidance is
labeled `task-guidance`; full inspection and historical context are labeled `inspected`. Matching
is not authorization, and inspection is not activation. Omitted bodies are marked as summaries;
required instruction delivery must be complete or explicitly incomplete.

The `context_digest` covers the manifest, effective settings and origins, loaded instruction-file
digests, ontology interpretation, snapshot identity, and operation/workflow/query/input selection.
An instruction-file edit invalidates context even if the manifest is unchanged. The digest format
has its own identifier. Digests identify content, not compliance, authority, or factual truth.
Optional scripts and resources have separate content identities and must be checked when used.

## Output contract

Explicit context output is the command result and goes to stdout. Incidental guidance on other
commands uses the same applicability rules, but is separate from their result data.

| Mode | JSON | Text |
| --- | --- | --- |
| `full` | Typed guidance records with full bodies and origins | Full guidance on stderr |
| `summary` | Rule inventory with explicit body omission | Short notice on stderr |
| `off` | No incidental guidance records | No incidental notice |

Defaults are full JSON guidance and text summaries. `guidance.display` configures defaults;
`--guidance full|summary|off` overrides delivery for an invocation. Explicit context requests still
return requested instructions. Suppression does not bypass structured validation or pre-action
loading obligations.

NDJSON preserves `operation-start` first and `operation-complete` last. `bundle-context`,
`guidance`, `workflow`, scope, and effective-query records remain distinct from concept results.
Consumers check process exit, completion evidence, scope, and intentional omissions separately.
Raw artifact/export formats are not contaminated by JSON records. Meta commands such as schema
and version do not load incidental bundle guidance. Exact record shapes are published by `okf
schema` rather than duplicated here.

## Skill integration

Skills perform compatibility preflight, establish the selected bundle, then load global task
context before source access or substantive work. They request full guidance for planned commands
before executing them, batching known operations where useful. New operations, changed scope,
changed effective context, or lost instruction context require a new read. A complete cached match
may be reused only for the same effective context identity.

Command output arrives too late to guarantee pre-action compliance, even if guidance was emitted
before the command's mutation. Incidental guidance supports discovery and interpretation; it does
not replace the separate context step. Higher-level skills that perform work directly follow the
same protocol rather than assuming another skill loaded instructions.

Fresh initialization and repair need deliberate bootstrap paths. There is no manifest to read
before creating an empty bundle; load context once the target exists and before subsequent work.
Malformed configuration is diagnosed and repaired through available inspection paths rather than
silently ignored. Shared skill protocol owns these common rules, while each skill owns its task
routing and specialized decisions.

## Named workflows and support resources

A workflow is a named interpreted procedure, not a CLI hook or a new executable command. It can
compose existing skills and tools. Selection can follow the user's task; a matching command alone
does not start a workflow.

```yaml
workflows:
  import-vendor-spec:
    description: Import requirements from a supplied specification.
    inputs:
      source:
        type: string
        required: true
      destination:
        type: path
        default: requirements
      review_level:
        type: enum
        values: [standard, detailed]
        default: standard
    resources:
      - .okf-bundle/scripts/format.sh
      - .okf-bundle/templates/requirement.md
    outputs:
      - Requirements with provenance in the selected destination.
      - A report of created concepts, unresolved questions, and validation.
    instructions: |
      Establish the source and scope from the supplied inputs.
      Create requirements using the bundled template.
      From the bundle root, run sh .okf-bundle/scripts/format.sh.
      Inspect the changes and validate the result.
```

Input types are string, boolean, enum, and bundle-relative path. Defaults must validate; a required
input cannot also have a default. Unknown input names are errors. Missing optional values remain
distinct from empty values. Inputs are data, not shell interpolation or instructions. The CLI
returns effective values, their origins, missing-required names, and `inputs_complete`; the agent
obtains missing required values before dependent actions. Input completeness does not prove tool
availability, authorization, or workflow success.

`resources` declares inspectable local files; it does not make them instructions. `outputs`
describes expected deliverables, not machine-verified completion. Instruction files have stronger
loading semantics than optional resources. Scripts run through the agent's tools within existing
authorization, never because the CLI reads a manifest. An unfamiliar script is inspected enough to
understand its effects. Missing tools, failed scripts, or incomplete outputs remain visible. The
agent inspects mutations and reruns relevant checks.

Command selectors describe invocations, not workflow phases. `operations: [add, edit]` cannot mean
“after ingestion,” which may span many commands. Post-ingestion formatting belongs in a selected
workflow or explicitly conditional global guidance.

## Support namespace and conformance profiles

The root `.okf-bundle/` directory stores scripts, templates, instruction files, evaluation cases,
and operational reports. Under the bundle tooling profile its Markdown is support material, not
concept content. A nested content directory with the same name does not establish another support
root, and arbitrary dotfiles are not excluded by this rule.

| Surface | Treatment |
| --- | --- |
| Knowledge queries, graph, indexes, generated documentation | Exclude root manifest/support content |
| Default artifact inventory | Exclude root manifest/support content before traversal |
| `bundle resource list/show`, explicit artifact inspection | Read support resources explicitly, bounded and classified |
| Lint, doctor, bundle-profile validation | Exclude support Markdown from concept/index checks; validate manifest separately |
| Physical snapshots, copying, source inventory | Preserve authored support files and metadata |

Hidden-by-default is classification, not confidentiality. Discovery must prune support directories
before traversal so inaccessible or large support trees do not break ordinary knowledge queries.
Physical traversal remains separate so transactions and snapshots do not lose authored state.
Support Markdown is not routed through concept mutation/parsing APIs.

`okf validate --profile core` remains the default core OKF v0.2 boundary. Explicit `--profile bundle`
applies the tooling namespace exclusion and identifies that profile in results. Older core-only
readers may treat support Markdown as concepts and reject files without frontmatter. A
bundle-profile pass therefore does not imply compatibility with such readers. Excluding support
files from a distribution can restore a core-oriented layout but removes workflow resources; a
general export transformation is outside this feature.

## Validation and migration

| Command | Responsibility |
| --- | --- |
| `validate` | Document conformance under the selected profile |
| `bundle validate` | Manifest schema/version, command selectors, inputs, and declared resources |
| `lint` | Bundle health, including nonsuppressible manifest failures and advisory concept findings |
| `doctor` | Compatibility diagnosis and explicit migration/repair planning |

Required instruction files are validated as readable UTF-8; ordinary resources may be opaque
binary files. Checks operate on declared references, not guessed filenames extracted from prose.
Successful parsing does not prove that instructions were followed. `lint --fix` does not migrate
configuration, rewrite instructions, run scripts, or fill in workflow inputs.

```text
okf doctor --migrate-config okf.toml --default-bundle-id example.docs --migration-plan /tmp/okf-migration.yaml
okf doctor --migration-apply /tmp/okf-migration.yaml
```

Migration preserves effective settings, registration metadata, path meaning, unrelated legacy
content, and original-file backups. A path-only default needs an author-chosen registered ID.
Ontology paths are rebased from the TOML directory to the bundle root without copying targets or
silently making external resources portable. Existing conflicting manifests/catalogs and missing
author choices block application.

Apply checks plan inputs before writing, validates destinations, preserves originals and recovery
information, and rolls back failed publication. Multi-file migration is recoverable, not a claim
of filesystem-wide atomicity. The legacy file is removed only when no remaining content requires
it. Ordinary doctor and safe-fix modes do not implicitly migrate workspace configuration.

CLI 0.6 retains TOML compatibility and historical readers. A future removal boundary requires a
separate documented migration policy; this design does not assign an arbitrary removal release.
Historical reads use the selected revision's manifest, ontology, and instructions, report missing
evidence, and never substitute current files silently. Historical instruction inspection does not
authorize execution. Snapshot-selection semantics are defined in the remote design.

## Relationship filtering and presets

Ontology defines relationship meaning. Manifest presets define explicit reusable queries over that
model. Views control presentation; guidance explains when a query is useful. None of these makes a
reference establish acceptance or fulfillment.

For `Defect --blocks--> Feature`, these inspect the incoming defects:

```text
okf backlinks features/export knowledge --kind blocks --related-type Defect --details
okf backlinks features/export knowledge --inverse-kind is-blocked-by --related-type Defect
```

`--kind` always matches the authored kind; `--inverse-kind` matches its configured inverse label.
`--related-type` selects the target on links and source on backlinks. `--rule`, `--edge-filter`, and
`--related-filter` narrow the declaring rule, occurrence attributes, and other endpoint metadata.
All predicates apply to the same occurrence, preventing parallel edges from satisfying different
parts of one condition. Categories combine with AND; repeated categorical values use OR; repeated
predicate expressions use AND.

Semantic predicates exclude untyped ordinary links rather than inventing a kind. Endpoint-only
predicates may inspect ordinary links. Missing or unavailable endpoint metadata cannot establish a
match and remains explicit uncertainty. Filtering precedes deduplication, sorting, and pagination.
Scan budgets bound document parsing; result limits bound delivery. A partial examination cannot
establish exact totals, absence, or globally complete blockers. Outgoing missing anchors remain
errors whether filters are present or not.

```yaml
settings:
  query_presets:
    blocking-defects:
      description: Defects with authored blocking relationships to a feature.
      operations: [backlinks]
      filters:
        kind: [blocks]
        related_type: [Defect]
      details: true
```

Select a preset with `--preset blocking-defects`. The first release has no automatic semantic
filter defaults. Begin with no predicates, apply one compatible explicit preset, then replace each
category supplied on the command line. `--clear-filter` removes a preset category and conflicts
with supplying that same category. Presets do not inherit. Search/list primary presets use their
own typed `query` fields; expansion predicates are separate from primary-hit selection.

Effective filters and their origins appear in results independently of `--guidance off`. Integrity
checks, migration comparisons, and update-impact analysis never inherit narrowing presets.
Cross-bundle queries keep the selected query consistent while interpreting each occurrence under
its declaring bundle's ontology.

## Command consistency

A shared leaf-command capability registry drives option admission, help, schema, references, and
skill compatibility. Unsupported flags error instead of being silently ignored.

| Surface | Contract |
| --- | --- |
| `links`, `backlinks` | Shared occurrence predicates, presets, bounded examination, pagination, completeness |
| `graph` | Predicates filter traversal before depth expansion; retain the requested root |
| Search/list expansion | Expansion-specific predicates affect related output, not primary hits |
| `affected`, `rm`, `mv`, integrity lint | Structural dependencies without query presets; refuse unsafe/incomplete mutation |
| Logical resolve/artifact reads | Explicit scope, snapshot identity, no implicit acquisition |
| Show, ontology reads, context/resources | Reopen historical evidence through the shared snapshot reader |
| `stale`, `refresh` | Distinguish resolution, content drift, and source snapshot expectations |
| Managed writers | Central destination protection including fixes, output paths, and recovery |

One catalog anchors the consumer namespace; reading upstream content does not activate upstream
workspace configuration. Cross-bundle moves/removals cannot rewrite protected consumers and may
require explicit reconciliation instead of coordinated mutation. Unknown out-of-scope dependents
remain unknown; a local catalog is not proof of global backlink completeness.

## Rationale and boundaries

A structured sidecar was chosen over a single prompt because deterministic selectors and validation
make required instructions discoverable. Ordinary guidance concepts would offer knowledge-graph
features but require an instruction bootstrap and blur evidence with authority. Full bundled skill
packages would introduce installation, routing, and execution machinery beyond local customization.

A pointer to changing information can be modeled through ordinary fields and guidance. A URL alone
does not distinguish a live destination from a citation. Storing a pointer does not fetch it; an
answer based on consulting it records observation/version evidence. No built-in pointer type or
freshness guarantee is introduced.

Operational reports and registries can live in support files; queryable reports/issues belong in
ordinary concepts. Evaluation cases and scripts can be stored and invoked through authorized
workflows, but evaluation execution semantics, standardized registry schemas, scheduling, automatic
hooks, recursive instruction includes, and automatic query defaults require separate designs.
