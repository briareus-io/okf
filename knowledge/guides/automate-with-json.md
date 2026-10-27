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

Use this guide when a program needs a complete command inventory. The example runs against
this repository's `knowledge/` bundle, using Python 3's standard library and `okf` on PATH.
It is read-only and prints the number and IDs of command concepts.

## Accept an inventory only after checking its evidence

NDJSON is one JSON object per line. Operation control records, query summaries, concept
payloads and extension records share the stream. Concept payloads preserve authored metadata;
do not assume every record has `kind` or every line is a concept.

This runnable consumer checks the process exit, stream version, terminal record, and query
completeness before returning any command records:

```sh
python3 - <<'PY'
import json
import subprocess


def command_inventory(stdout, exit_code):
    rows = [json.loads(line) for line in stdout.splitlines() if line.strip()]
    if not rows or rows[0].get("kind") != "operation-start":
        raise ValueError("Missing operation start")
    if rows[0].get("stream_version") != 1 or rows[0].get("schema_version") != 1:
        raise ValueError("Unsupported stream version")
    end = rows[-1]
    if end.get("kind") != "operation-complete" or end.get("schema_version") != 1:
        raise ValueError("Missing or unsupported completion record")
    if exit_code != 0 or end.get("exit_code") != exit_code or end.get("outcome") != "ok":
        raise ValueError("Operation did not complete successfully")
    completeness = end.get("completeness", {})
    if any(completeness.get(key) != "complete" for key in ("examination", "matching", "delivery")):
        raise ValueError("Inventory is incomplete")
    summaries = [row for row in rows if row.get("kind") == "query-summary"]
    if len(summaries) != 1:
        raise ValueError("Expected one query summary")
    summary = summaries[0]
    if not summary.get("scan_complete") or summary.get("partial") or summary.get("has_more"):
        raise ValueError("Scan or page is incomplete")
    commands = [row for row in rows if row.get("type") == "Command" and row.get("id")]
    if len(commands) != summary.get("returned") or len(commands) != summary.get("total_matches"):
        raise ValueError("Payload count does not match the query evidence")
    return commands


result = subprocess.run(
    ["okf", "list", "knowledge", "--type", "Command", "--full-scan", "--json"],
    capture_output=True, text=True,
)
commands = command_inventory(result.stdout, result.returncode)
print(f"Accepted {len(commands)} commands")
for command in commands:
    print(command["id"])
PY
```

For this release, expect `Accepted 48 commands`, followed by IDs including `/commands/add`
and `/commands/refresh`. Derive the count from the result rather than hard-coding it in an
integration. If the CLI cannot start, Python raises an OS error; if the CLI fails or the
stream is truncated, the consumer raises instead of reporting a successful empty inventory.

## Adapt acceptance to the question

This consumer is deliberately for a complete, unpaginated, single-bundle command inventory.
It does not support a projected payload or related expansion. Adapt the checks when changing
the invocation:

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
