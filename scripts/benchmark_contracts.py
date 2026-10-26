#!/usr/bin/env python3
"""Reproducible CLI measurements; creates only disposable local fixtures.

Run against an explicit binary before/after a change. No timing assertion is imposed:
the JSON records reproducible inputs and measured variance, not an invented SLO.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import statistics
import subprocess
import tempfile
import time


def fixture(root, size):
    (root / "references").mkdir()
    (root / "references" / "shared.txt").write_text("source\n", encoding="utf-8")
    body = "A reproducible note about contract behavior.\n" * 8
    for i in range(size):
        target = f"/note-{(i + 1) % size}.md"
        text = (
            f"---\ntype: Note\ntitle: Note {i}\ntags: [contracts]\n"
            "details: {owner: example, nested: {enabled: true}}\n"
            "sources:\n  - {resource: references/shared.txt, kind: file}\n"
            f"---\n{body}\n[Next]({target})\n"
        )
        (root / f"note-{i}.md").write_text(text, encoding="utf-8")


def run(binary, args, cwd):
    started = time.perf_counter()
    result = subprocess.run(
        [binary, *args], cwd=cwd, stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False,
    )
    elapsed = time.perf_counter() - started
    if result.returncode:
        raise RuntimeError(f"{args}: exit {result.returncode}: {result.stderr.decode(errors='replace')}")
    return {"seconds": elapsed, "stdout_bytes": len(result.stdout), "stderr_bytes": len(result.stderr)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", required=True)
    parser.add_argument("--build-profile", default="unspecified")
    parser.add_argument("--sizes", default="100,1000,10000")
    parser.add_argument("--repetitions", type=int, default=3)
    parser.add_argument("--output", type=Path, required=True)
    options = parser.parse_args()
    binary = str(Path(options.binary).resolve(strict=True))
    sizes = [int(value) for value in options.sizes.split(",")]
    if options.repetitions < 2 or any(size < 1 for size in sizes):
        parser.error("positive sizes and at least two repetitions required")
    version = subprocess.check_output([binary, "version"], text=True).strip()
    records = []
    with tempfile.TemporaryDirectory(prefix="okf-benchmark-") as temporary:
        workspace = Path(temporary)
        for size in sizes:
            root = workspace / f"bundle-{size}"
            root.mkdir()
            fixture(root, size)
            workloads = {
                "bounded-search": ["search", str(root), "--text", "contract", "--limit", "10", "--json"],
                "full-inventory": ["list", str(root), "--full-scan", "--json"],
                "projected-read": ["show", "note-0", str(root), "--project", "$id", "--json"],
                "shared-source-stale": ["stale", str(root), "--json"],
                "single-edit-preview": ["edit", "note-0", str(root), "--set", "title=Revised", "--dry-run", "--json"],
            }
            for name, arguments in workloads.items():
                first = run(binary, arguments, workspace)
                repeated = [run(binary, arguments, workspace) for _ in range(options.repetitions)]
                durations = [sample["seconds"] for sample in repeated]
                records.append({
                    "concepts": size, "workload": name,
                    "first_process": first, "repeated_processes": repeated,
                    "median_seconds": statistics.median(durations),
                    "max_seconds": max(durations),
                })
    report = {
        "format_version": 1, "binary_version": version, "build_profile": options.build_profile,
        "binary_sha256": hashlib.sha256(Path(binary).read_bytes()).hexdigest(),
        "platform": platform.platform(), "machine": platform.machine(),
        "python": platform.python_version(), "cpu_count": os.cpu_count(),
        "fixture_version": 1, "fixture_seed": "deterministic-ring-v1",
        "sizes": sizes, "repetitions": options.repetitions,
        "cache_conditions": "Each sample is a new process; filesystem caches are not flushed. First sample is not claimed to be cold storage.",
        "metrics": ["wall_seconds", "stdout_bytes", "stderr_bytes"],
        "limitations": "No peak RSS, I/O, parse/staging counters, lock duration or Git subprocess instrumentation. This is a latency baseline, not the full performance matrix or a regression threshold.",
        "records": records,
    }
    options.output.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(f"Wrote {len(records)} workload measurements to {options.output}")


if __name__ == "__main__":
    main()
