---
type: Guide
audience: user
uses:
- /commands/list
- /commands/schema
explains:
- /concepts/selection-and-scope
title: Automate with JSON
description: Accept structured results using process exit, operation completion and scope evidence.
---
# Automate with JSON

Suppose an integration needs every command documented in this repository. Run the examples
from the repository root. Each line on standard output is a separate JSON object.

## Request the inventory

```sh
okf list knowledge --type Command --full-scan --json
```

The first record identifies the command and stream version. The stream then includes guidance,
scope, and one concept record per matching command. For example, the record for `okf add`
contains these fields (excerpt):

```json
{"id": "/commands/add", "type": "Command", "title": "okf add", "mutates": true}
```

Collect records with an ID and `type: Command`. A concept can have authored fields of its own,
so assuming every record has a `kind` field would miss ordinary concepts.

## Check whether every match arrived

Near the end, the `query-summary` record reports what the CLI examined and delivered. For
example, an uninterrupted run against a bundle containing 64 command concepts would include
these summary fields:

```json
{"kind": "query-summary", "scan_complete": true, "partial": false, "has_more": false, "total_matches": 64, "returned": 64}
```

Accept a complete inventory when all three checks pass:

1. The stream starts with a supported `operation-start` schema and stream version and ends with
   a supported `operation-complete` record. The process exit and trailer exit code are both zero.
2. The trailer reports `outcome: ok` and complete examination, matching, and delivery.
3. Exactly one query summary reports a complete scan and no remaining page. The number of
   collected commands equals both `returned` and `total_matches`. Read these counts from
   the response each time.

A [worked Python consumer](../references/command_inventory.py) implements these checks and
prints the accepted IDs.

## See what an incomplete scan returns

```sh
okf list knowledge --type Command --scan-limit 1 --json
```

The command examines at most one document. It can exit successfully while reporting partial
examination. Reject that output when the task requires every command. Likewise, a missing
trailer leaves a read incomplete, even if some valid command records arrived.

## Adapt acceptance to the question

This consumer is deliberately for a complete, unpaginated, single-bundle command inventory. It does
not support a projected payload or related expansion. Adapt the checks when changing the invocation:

| Change | Evidence to inspect |
| --- | --- |
| Add `--limit` or `--offset` | A delivered page is not the complete inventory; inspect `has_more` and the requested page. |
| Add `--scan-limit` | Exit zero can accompany partial examination; rerun with suitable scope or budget. |
| Add `--expand` | Check `expansion-summary`, its truncation/reasons, and related records independently. |
| Request catalog scope | Check requested and examined bundles, unavailable members, and qualified identities. |
| Read a bounded artifact | Check `truncated` and content completeness rather than query counts. |

A terminal record with outcome `partial` is useful evidence, but this inventory consumer
rejects it because its job requires exhaustiveness. A UI showing a preview may choose to
accept it and display the limitation.

## Keep read and write recovery separate

A missing completion record on a read means the result is incomplete. On a write, it can
mean the publication outcome is unknown. Do not retry an append or verification event
blindly: inspect the expected post-state and follow
[write recovery](../reference/troubleshooting.md#recover-an-interrupted-write).

Use [schema](../commands/schema.md) for the binary's current contract, the generated
[CLI record reference](../reference/cli-contracts.md) for extension shapes, and
[command contracts](../reference/command-contracts.md) for common behavior.
