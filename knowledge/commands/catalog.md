---
type: Command
group: meta
mutates: false
title: okf catalog
description: Inspects configured and effective catalog locations, availability and bundle settings.
sources:
- resource: crates/okf-cli/src/commands/catalog.rs
  kind: git-path
- resource: crates/okf-cli/src/commands/schema.rs
  kind: git-path
- resource: crates/okf-cli/src/cli.rs
  kind: git-path
- resource: crates/okf-core/src/bundle/catalog.rs
  kind: git-path
- resource: crates/okf-core/src/bundle/context.rs
  kind: git-path
last_modified: 2026-10-04T20:39:29Z
---
# okf catalog

Lists effective local registrations without selecting a primary bundle. IDs, configured and
overridden locations, availability and applicable tool settings are inspection context. Unknown
override IDs, conflicting registrations and malformed configuration are errors. Catalog
registration does not authorize traversal or prove publisher identity. The CLI does not acquire
remote bundles or accept different snapshots through a location override.

## Arguments

_No arguments._

Global `--json` requests NDJSON.

Output stream: `bundle-registration,effective-settings`.

Supported global options: `--json`.
