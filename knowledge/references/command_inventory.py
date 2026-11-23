"""Read a complete OKF command inventory. Run from the repository root."""
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


if __name__ == "__main__":
    result = subprocess.run(
        ["okf", "list", "knowledge", "--type", "Command", "--full-scan", "--json"],
        capture_output=True, text=True,
    )
    commands = command_inventory(result.stdout, result.returncode)
    print(f"Accepted {len(commands)} commands")
    for command in commands:
        print(command["id"])
