---
name: retrieval
description: Answer questions from OKF bundles using scoped structured queries, progressive disclosure, source lineage, and explicit completeness/trust evidence. Use before reading bundle files directly; this skill is read-only.
---

# Retrieve knowledge from an OKF bundle

## Shared operating protocol

Read and follow the [shared OKF protocol](../../references/bundle-context.md) before CLI-dependent
work. It defines compatibility checks, bundle context, scope, output evidence, validation profiles,
and authoring conventions. Use `references/cli.md` for exact command syntax as needed.

Query the bundle via the CLI rather than bulk-reading it. Use `OKF_BUNDLE` for multi-step work;
explicit examples are `okf show <concept-id> <bundle>` and `okf links <concept-id> <bundle> --json`.
Before using an unfamiliar command or flag, read its entry in `references/cli.md`. If the installed
version differs or rejects documented syntax, use that command's `--help` output as the runtime
authority.

Search text must follow `--text`; the positional argument to search is the bundle path. For example,
`okf search <bundle> --text "travel policy" --limit 10`. Structured-only searches are valid, such as
`okf search <bundle> --type Note --tag policy --limit 10`. Without filters, search lists concepts;
use `okf list <bundle> --json` for an intended inventory. Use `okf browse <bundle> --directory
<directory>` to descend; its positional is also the bundle.

## Progressive disclosure

For JSON operations, require the versioned `operation-start` and
`operation-complete` records and check the actual process exit before treating
the payload as complete. An absent trailer after interruption proves neither
an empty result nor a complete scan. Use the trailer's examination and matching
dimensions alongside query-summary evidence; a bounded successful read can
still be partial. The trailer's `outcome` is an operation result, never a
concept's authored lifecycle `status`.

1. Establish the primary bundle and the scope needed for the question before querying. For catalog
   work, inspect registrations once and pass the selected `--bundle-id` on subsequent commands;
   add only the required `--scope-bundle` identities. A request spanning named bundles supplies
   that scope; it does not require another permission question. Reinspect settings only if the
   selection, configuration, or revision changes.
   Use the shortest route that resolves the question: open a named concept directly, search a known
   topic, or use `okf browse <bundle>` when orientation is needed. Descend with `--directory`;
   search with `okf search <bundle> --text "query terms" --limit 10`. Narrow noisy results with
   `--in title,description`; adding `frontmatter` expands metadata coverage. The default matches
   reader-visible phrases. Consult the search reference for repeatable phrases, token modes,
   literal Markdown matching, and sorting.
2. Choose the query from the evidence needed. Use typed filters for known metadata, facets when
   discovering a vocabulary/distribution, and `--project` for selected metadata. Inspect the
   effective ontology with `okf ontology show <name> <bundle>` for relationship rule names, then
   use `--expand` with `--target-field` for
   one-hop outbound metadata instead of opening each target. Use `links --details` for authored
   outgoing assertions and `backlinks --details` for incoming assertions; open bodies only for evidence missing from metadata. For worked decisions, scan
   recovery, and correlated-record pitfalls, read [query planning](references/query-planning.md).
   Check query summaries and warnings before using results: the default scan budget is 1,000
   eligible documents across scope, independent of result `--limit`. Incomplete scans cannot
   establish absence, exact totals, complete inventories, or a globally sorted page. Narrow the
   examination scope when appropriate, raise `--scan-limit`, or use `--full-scan` when exhaustive
   coverage is needed. Never use offset continuation to repair an incomplete scan.
3. For text searches with `--json`, use each result's `search.score` and `search.matches` to choose candidates. Match
   evidence identifies the field, bounded snippet, and a serialized document line when applicable;
   use that line to request a small `show --lines` window. Treat snippets as navigation evidence,
   not enough context to answer from.
4. Read a small candidate directly with `okf show <concept-id> <bundle>`. For large concepts, use
   `okf show <concept-id> <bundle> --outline`, then `--lines <START:END>` around relevant sections.
   Plain `show --json` contains metadata, not the body. For a small concept's prose alone, use
   `okf show <concept-id> <bundle> --body`; `--body --json` provides an explicit body record.
   `okf show <concept-id> <bundle> --numbered` (short form `-n`) uses the same document numbering
   as `--lines`, including frontmatter and excluding the plain display header. Keep outline/line windows for large documents.
5. Follow portable body links and standard internal `sources[].resource` lineage when needed to
   support the answer; these work without ontology. Start with `okf links <concept-id> <bundle>
   --json` for direct outbound targets and `okf backlinks <concept-id> <bundle>` for direct inbound
   concepts. Add `--details` to `links` when kinds, inverse labels, attributes, or concrete field paths
   matter. Local detailed JSON mixes semantic `relationship` records with ordinary `link` records;
   catalog output uses `bundle-edge` records with optional `relationship` metadata. Retain repeated
   semantic occurrences to the same target. Inverse labels describe the incoming view of an
   authored assertion; they do not add another outgoing edge. Render only the useful neighborhood with `okf graph <bundle> --root <concept-id>
   --direction <outgoing|incoming|both> --depth <N> --format mermaid`; omit the root only when the
   entire graph is genuinely needed. Use `okf resolve <link> <bundle> --from <concept-id>` for a
   particular raw link.
6. Join claim footnotes to `sources[].id`. Cite the declaring concept and relevant source or
   resolved artifact separately. Do not fill bundle gaps with assumptions.

If a search finds nothing, try a broader token query with `--match any`, then a relevant type, tag,
or directory if known. Stop when the question is answered or these bounded alternatives provide no
useful leads. Report the search scope and missing evidence; failed queries alone do not establish
that the entire bundle lacks an answer.

## Trust and lifecycle decision

Use the returned lifecycle/trust metadata for material claims; inspect further only where it could
change the answer. Consider:

1. effective `status` (`stable` when absent; distinguish draft and deprecated);
2. whether `stale_after` has expired at the current instant;
3. the latest valid `verified.at` and derived tier;
4. whether `generated.at` is later than that verification;
5. relevant source credibility/usage signals and per-claim source IDs;
6. for computed values, whether this actual run has a passing runtime attestation.

Trust is advisory: stale, deprecated, or unverified knowledge may still be useful, but report
limitations that materially affect the answer. An Attested Computation definition can exist and be
document-verified without any run having occurred. `okf computation check <concept-id> <bundle>` is
inspect-only and reports `execution: not-run`; never describe that as a passing attestation.

## Artifact retrieval and `references/`

`references/` is an optional convention, not required. Markdown files beneath it are ordinary
concepts; non-Markdown files are opaque artifacts. Path-valued fields may be URLs, bundle-relative,
or document-relative. Resolve with `okf artifact resolve <resource> <bundle> --from <concept-id>`
and report `concept`, `artifact`, `reserved`, `support`, `external`, `scope`, `missing`, and `blocked`
distinctly. Use `okf show` for concepts and bounded `okf artifact show <resource> <bundle> --from
<concept-id> --lines <START:END> --max-bytes <N> --json` for artifacts. Never force SQL/Python
through `okf show`.

For path namespaces, external sources, and truncated reads, consult the artifact sections of
`references/cli.md` when needed. Artifact reads stay inside the resolved target bundle and requested scope; existing user
authorization to inspect a named external source also counts. Reading code never authorizes
execution.

Answer with the concepts actually opened, source/artifact citations, precise caveats, and a precise
statement of what could not be established from the material inspected. Do not mutate the bundle.

## Structured queries and historical evidence

For facets, paging, projection, expansion, or nested relationship conditions, read
[query planning](references/query-planning.md). Its examples explain scan completeness and
same-record correlation; use `references/cli.md` for exact selector and flag syntax.

When a source has `bundle_ref`, distinguish its requested snapshot from the actual resolved
content and any separately labeled candidate. Open evidence using its returned bundle and revision.
A matching digest or commit does not verify claims; retrieval neither accepts a lock update nor
refreshes fingerprints. Follow the shared protocol for unavailable historical evidence.

If concept YAML fails to parse, report the named files and use `okf validate <bundle> --profile bundle --json`
for document diagnostics. Do not repair or normalize files during retrieval.
