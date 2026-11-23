---
type: Reference
audience: contributor
title: Catalog identity and cross-bundle resolution
description: Registration, path resolution, version evidence, and graph identity invariants for local catalogs.
---
# Catalog identity and cross-bundle resolution

The local catalog identifies independently maintained bundle roots. The resolver associates
each concept occurrence with its declaring bundle, original reference, examined version and
resolution evidence. For a workspace setup and example queries, see
[Work with multiple bundles](../../guides/work-with-multiple-bundles.md).

[Bundle manifests](bundle-manifest.md) own portable settings. [Remote dependencies](remote-bundles.md)
extend local catalog registration with acquisition, accepted locks and managed snapshots.

## Registration and portability

| Rule | Purpose and effect |
| --- | --- |
| Use namespaced stable IDs, such as `acme.finance` | Distinguishes bundles across repositories; no alias layer is introduced |
| Register one effective local root per bundle ID | Makes selection and reference resolution explicit; machine-local overrides can change that location |
| Reject conflicting registrations | Avoids silently choosing one bundle when identity or location is ambiguous |
| Do not register equal or nested roots | A bundle cannot be a subbundle of another; ordinary subdirectories inside a bundle remain valid |
| Keep reference extensions consistent with ordinary resources | Catalog metadata cannot silently redirect a reference to known different material |
| Make graph/check scope explicit | Results describe selected, available bundles rather than claiming global coverage |

Root overlap is checked after path canonicalization for available directories. Unavailable roots are
reported as unavailable, not treated as verified non-overlapping locations. Independently maintained
forks receive new IDs; mirrors retain the original ID. These are identity conventions, not proof of
publisher authenticity or a global registration system.

### Relationship to OKF

This is a CLI extension above [OKF v0.2](https://github.com/GoogleCloudPlatform/open-knowledge-format/blob/main/SPEC.md),
not a stricter replacement specification. OKF permits additional metadata, ordinary paths
and URLs, and broken links; indexes are optional. Concept IDs remain paths without `.md`.
See §§2–6 and §8.

| Addition | Effect on OKF compatibility |
| --- | --- |
| Catalog and TOML settings | Optional tool configuration; neither becomes a required bundle file |
| Bundle ID | Qualifies CLI targets without changing standard concept IDs |
| `bundle_ref` source metadata | Producer-defined extension; ordinary `resource` retains its meaning |
| Catalog registration and boundary checks | Tool configuration errors, not new concept-conformance rules |
| Missing cross-bundle targets | Warnings by default, separately configurable for CI; broken links remain consumable |
| Optional snapshot expectations | Extra resolution metadata; no new mandatory provenance or verification fields |

The design does not intentionally override an OKF normative requirement or impose additional
conformance requirements. It preserves optional metadata, unknown fields, and ordinary single-bundle
use. Rejecting a catalog configuration must not be reported as invalid OKF content; the underlying
bundle can still be addressed directly.

There is a portability limitation: OKF describes a bundle as a self-contained distribution
unit. Relative references to sibling bundles depend on the surrounding layout. They retain
ordinary path semantics, but distributing just one directory may leave broken references.
Consequently, this mode provides **layout-preserving portability**, not independently
self-contained distribution or a guarantee that every reader permits access outside a
selected bundle root. This limit is explicit rather than addressed by changing path meaning.
No stronger claim of universal reader interoperability is made.

## Selection and effective paths

Catalog paths resolve relative to their declaring configuration file, registered locations
relative to the catalog, and local overrides relative to the override file. ID and path
selectors occupy separate namespaces. The containing registration takes precedence over a
configured default. Conflicting registrations and overlapping canonical roots make qualified
identity ambiguous. Keep local-directory registration independent of lockfiles, managed caches
and ontology discovery. For precedence and legacy layout compatibility, see
[configuration](../../reference/configuration.md).

## Reference resolution

The CLI resolves an ordinary local path, identifies the registered root containing its target, and
assigns the qualified target. It must not collapse an above-root traversal into an unrelated concept
inside the source bundle. This extends current single-bundle graph resolution specifically for
registered cross-bundle concept targets; it does not grant general unrestricted artifact reads
outside a bundle.

The extension is optional. Ordinary links can become cross-bundle edges through their resolved
target location without additional metadata. Existing internal links retain their meaning. An
unresolved path is never matched to another bundle by filename or title alone.

Standard URLs remain usable references. An explicit declared mapping can associate one with local
material, but this feature does not fetch or verify the remote resource. It must report that
equivalence is unchecked when no local evidence establishes it.

### Resolution evidence and diagnostics

The resolver prefers the strongest readily available evidence:

| Evidence | What can be reported |
| --- | --- |
| Same canonical local file | Both references address the same available file |
| Same repository, commit, and path | Both address the same locally available Git object |
| Matching optional digest | The checked file's bytes match the recorded digest |
| Explicit declared mapping | A correspondence is configured, with equivalence unchecked |

A declared mapping is allowed without independent proof, but it cannot override a known mismatch.
Resource semantics take priority over contradictory catalog metadata. Resolution evidence does not
establish the factual correctness of a concept or create a verification event.

Resource resolution and snapshot satisfaction are separate results. A live ordinary path remains the
ordinary target. Finding different historical bytes does not silently replace that target:
historical content counts as a successful reference match only when the resource identifies that
content or an explicit mapping establishes correspondence without overriding a known mismatch.
Otherwise retain the ordinary target and historical result separately, with their evidence and
status.

Diagnostics distinguish unknown bundle IDs, unavailable local roots or revisions, missing targets,
registration conflicts, reference mismatches, unchecked equivalence, and changed source
fingerprints. These are separate dimensions: a target can exist while its source has changed.
Missing targets warn by default; CI may configure a stricter lint policy.

### Snapshot evidence

`bundle_ref.snapshot` supports `commit`, `ref`, `label` and `digest`. Commits are full immutable
Git object IDs; refs and labels are hints. Digests describe raw referenced file bytes as SHA-256
hex, optionally prefixed `sha256:`. Scope records also retain `snapshot_examined` for separately inspected historical target nodes.
Snapshot results retain requested metadata, status, evidence,
resolved identity and separately labeled candidate identity. Nodes use qualified bundle/concept
identity and version evidence: current content is `working-tree`, commit material is `git:<SHA>`,
and a digest-established snapshot is `sha256:<HEX>`. Explicit standalone subdirectories retain
their own boundary; combining one with an overlapping registered root in scope is rejected.

The resolver distinguishes a requested snapshot from a current candidate.

1. Attempt to establish the requested content from available local files or Git material.
2. Report whether the request is matched, unverified, unavailable, or mismatched.
3. If it cannot be established, allow current local content as a separately labeled candidate.
4. Keep the candidate distinct from the resolved request in human output, machine output,
   and graph results. A known mismatch never counts as a successful snapshot match.

A user or skill may inspect and reconcile a candidate. Merely returning it neither accepts it as a
substitute nor changes authored metadata, fingerprints, or verification history. Local Git
operations must not implicitly fetch missing objects, including in partial clones.

An extension-only snapshot expectation does not pin an ordinary reader to historical content. A
portable exact pin also needs a standard resource identifying that same content. A live relative
path plus a historical expectation remains a live path for other readers.

### Graphs and scope

Primary bundle selection and examination scope are separate. The default scope contains only the
selected bundle. Examining additional bundles requires an explicit bundle set or an explicit
catalog-wide scope; catalog registration alone does not authorize traversal. Cross-bundle references
outside that scope remain visible with an out-of-scope status. Results report requested scope,
actual examined bundle/version scope, and unavailable members.

Graph nodes are qualified by bundle identity, concept identity, and resolved version where
available. Multiple revisions can coexist. Working-tree content is identified as such, not
mislabeled as an immutable commit when local changes are present.

Edges retain the original reference, its source location, target, and resolution evidence.
Unresolved edges remain visible. Traversal supports cycles through visited-node tracking and bounded
scope. Backlinks and affected analysis report which bundles and versions were examined; consumers
outside that scope remain unknown.

Bundles in the same repository may be updated and committed together. When inspecting a specific
local commit, they use that commit unless a reference explicitly requests another version. Ordinary
working-tree reads do not promise an atomic snapshot across files.

## Ownership boundaries

A catalog coordinates local reads. Multiple repositories may need separate edits when a move
changes cross-bundle paths. A mutation in one bundle cannot claim to reconcile consumers outside
its examined scope. Resolution and retrieval leave source verification, fingerprint refresh and
acceptance of upstream revisions to their respective workflows.

The CLI's leaf-command capability registry drives the scope and revision options advertised in
[command contracts](../../reference/cli-contracts.md). Catalog registration errors are distinct
from OKF document conformance.
