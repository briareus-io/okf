#!/usr/bin/env python3
"""Check manual navigation and exercise the actual published examples. Stdlib only."""
import collections
import json
import os
from pathlib import Path
import re
import shlex
import subprocess
import tempfile
from urllib.parse import unquote, urlsplit

ROOT = Path(__file__).resolve().parents[1]
BUNDLE = ROOT / "knowledge"
BINARY = Path(os.environ.get("OKF_DOCS_BIN", ROOT / "target/debug/okf")).resolve()
ENV = {k: v for k, v in os.environ.items() if k not in ("OKF_BUNDLE", "OKF_CATALOG_OVERRIDES")}


def run(*args, expected=0, cwd=ROOT):
    result = subprocess.run([str(BINARY), *map(str, args)], cwd=cwd, env=ENV,
                            text=True, capture_output=True)
    if result.returncode != expected:
        raise AssertionError(f"{args}: exit {result.returncode}\n{result.stdout}\n{result.stderr}")
    return result.stdout


def records(*args, cwd=ROOT):
    rows = [json.loads(line) for line in run(*args, "--json", cwd=cwd).splitlines()]
    assert rows[0]["kind"] == "operation-start", rows
    assert rows[-1]["kind"] == "operation-complete", rows
    assert rows[-1]["exit_code"] == 0 and rows[-1]["outcome"] == "ok", rows[-1]
    for row in rows:
        if row.get("kind") == "query-summary":
            assert row["scan_complete"] and not row["partial"], row
    return rows


def prose(path):
    text = path.read_text()
    text = re.sub(r"\A---\n.*?\n---\n", "", text, flags=re.S)
    return re.sub(r"^```[^\n]*\n.*?^```\s*$", "", text, flags=re.M | re.S)


def anchors(path):
    result = set()
    counts = collections.Counter()
    for heading in re.findall(r"^#{1,6}\s+(.+?)\s*#*\s*$", prose(path), re.M):
        slug = re.sub(r"[^\w\- ]", "", heading.lower()).replace(" ", "-")
        suffix = f"-{counts[slug]}" if counts[slug] else ""
        result.add(slug + suffix)
        counts[slug] += 1
    return result


def check_navigation():
    errors = []
    pages = [ROOT / "README.md", *BUNDLE.rglob("*.md")]
    for page in pages:
        for raw in re.findall(r"\[[^\]\n]*\]\(([^\s)]+)\)", prose(page)):
            link = urlsplit(raw)
            if link.scheme or link.netloc:
                continue
            path = unquote(link.path)
            target = ((BUNDLE / path.lstrip("/")) if path.startswith("/") else
                      page.parent / path).resolve() if path else page
            if not target.exists():
                errors.append(f"{page.relative_to(ROOT)}: missing {raw}")
            elif link.fragment and target.suffix == ".md" and unquote(link.fragment) not in anchors(target):
                errors.append(f"{page.relative_to(ROOT)}: missing anchor {raw}")
    assert not errors, "\n".join(errors)
    print(f"Navigation: {len(pages)} pages, local targets and anchors resolved")


def blocks(page):
    return re.findall(r"^```sh\n(.*?)^```", (BUNDLE / page).read_text(), re.M | re.S)


def shell_example(script, demo_root):
    # Execute the published snippet, substituting only its disposable root and binary.
    script = script.replace("demo_root=$(mktemp -d)", "demo_root=" + shlex.quote(str(demo_root)))
    script = re.sub(r"^okf\b", shlex.quote(str(BINARY)), script, flags=re.M)
    script = script.replace('["okf",', '[' + json.dumps(str(BINARY)) + ',')
    result = subprocess.run(["bash", "-eu", "-c", script], cwd=ROOT, env=ENV,
                            text=True, capture_output=True)
    assert result.returncode == 0, result.stdout + result.stderr
    return result.stdout


def check_examples():
    with tempfile.TemporaryDirectory(prefix="okf-docs-") as tmp:
        tmp = Path(tmp)
        model_root = tmp / "model"
        model_root.mkdir()
        shell_example("\n".join(blocks("guides/model-your-knowledge.md")), model_root)
        concept = model_root / "bundle/guides/example.md"
        rows = records("show", "guides/example", model_root / "bundle")
        authored = next(row for row in rows if row.get("id") == "/guides/example")
        assert authored["audience"] == "contributor" and "authoring" in authored["tags"]
        assert authored["uses"] == ["/commands/check"], authored
        assert "/guides/example" in json.dumps(records("backlinks", "commands/check", model_root / "bundle", "--details"))
        before = concept.read_bytes()
        run("edit", "guides/example", model_root / "bundle", "--patch",
            "@knowledge/references/authoring/concept-patch.json", expected=2)
        assert concept.read_bytes() == before, "A failed guarded patch changed the concept"

        move_root = tmp / "move"
        move_root.mkdir()
        setup, apply = blocks("guides/reorganize-a-bundle.md")
        plan_text = shell_example(setup, move_root)
        plan = [json.loads(line) for line in plan_text.splitlines() if line.startswith("{")]
        assert plan[-1]["outcome"] == "ok"
        digest = next(row["base_digest"] for row in plan if row.get("kind") == "changeset-summary")
        bundle = move_root / "bundle"
        assert (bundle / "plans/current.md").exists(), "Preview published the move"
        apply = "demo_root=" + shlex.quote(str(move_root)) + "\n" + apply
        shell_example(apply.replace("REPLACE_WITH_REVIEWED_BASE_DIGEST", digest), move_root)
        assert (bundle / "plans/revised.md").exists()
        assert not (bundle / "plans/current.md").exists()
        assert not (bundle / "plans/obsolete.md").exists()
        assert "../plans/revised.md" in (bundle / "notes/consumer.md").read_text()

        # The quickstart must work independently of this repository's default bundle.
        quick = blocks("getting-started/first-bundle.md")[0].replace("mybundle", shlex.quote(str(tmp / "first")))
        shell_example(quick, tmp)

        maintenance = tmp / "maintenance"
        maintenance.mkdir()
        shell_example("\n".join(blocks("guides/maintain-source-backed-knowledge.md")), maintenance)
        maintained = records("show", "notes/service", maintenance / "bundle")
        authored = next(row for row in maintained if row.get("id") == "/notes/service")
        assert authored["sources"] == [{"resource": "references/service.txt", "kind": "file"}], authored
        assert "three times" in run("show", "notes/service", maintenance / "bundle", "--body")

        catalog = tmp / "catalog"
        catalog.mkdir()
        shell_example("\n".join(blocks("guides/work-with-multiple-bundles.md")), catalog)
        selected = records("search", "--bundle-id", "demo.product", "--text", "retention", cwd=catalog)
        scoped = records("search", "--bundle-id", "demo.product", "--scope-bundle", "demo.policy", "--text", "retention", cwd=catalog)
        assert next(row for row in selected if row.get("kind") == "query-summary")["total_matches"] == 1
        assert next(row for row in scoped if row.get("kind") == "query-summary")["total_matches"] == 2
        back = records("backlinks", "demo.policy:/policies/retention", "--bundle-id", "demo.product", "--scope-bundle", "demo.policy", "--details", cwd=catalog)
        assert "/notes/overview" in json.dumps(back), back

        for guide in ["find-and-read-knowledge", "bounded-reads", "tour-the-bundle"]:
            shell_example("\n".join(blocks(f"guides/{guide}.md")), tmp)

        consumer = blocks("guides/automate-with-json.md")[0]
        negative_checks = '''
partial = subprocess.run(
    ["okf", "list", "knowledge", "--type", "Command", "--scan-limit", "1", "--json"],
    capture_output=True, text=True,
)
for label, output, exit_code in [
    ("missing completion", "\\n".join(result.stdout.splitlines()[:-1]), 0),
    ("failed process", result.stdout, 3),
    ("partial scan", partial.stdout, partial.returncode),
]:
    try:
        command_inventory(output, exit_code)
    except ValueError:
        pass
    else:
        raise AssertionError("Consumer accepted " + label)
'''
        shell_example(consumer.replace("\nPY\n", "\n" + negative_checks + "\nPY\n"), tmp)
    print("Worked examples: all eight guides plus quickstart passed; incomplete/failed JSON results rejected")


def check_bundle():
    records("validate", BUNDLE)
    lint = records("lint", BUNDLE, "--fail-on", "any")
    actionable = [row for row in lint if row.get("rule") in
                  ("broken-link", "ontology-violation", "index-coverage", "frontmatter-order")]
    assert not actionable, actionable
    notices = collections.Counter(row["rule"] for row in lint if row.get("kind") == "finding")
    print("Bundle: conformant; advisory findings:", dict(notices))

    schema = [json.loads(line) for line in run("schema").splitlines()]
    commands = {row["name"].replace(" ", "-"): row for row in schema if row.get("kind") == "command"}
    authored = {row["id"].split("/")[-1]: row for row in records("list", BUNDLE, "--type", "Command", "--full-scan") if row.get("type") == "Command"}
    assert commands.keys() == authored.keys(), "Command pages differ from the schema"
    for name, command in commands.items():
        assert authored[name]["group"] == command["group"], name
        assert authored[name]["mutates"] == command["mutates"], name

    inventory = records("list", BUNDLE, "--full-scan")
    for row in inventory:
        if not row.get("type"):
            continue
        for source in row.get("sources", []):
            assert "fingerprint" not in source, row["id"]
            assert not re.search(r"github\.com/[^/]+/[^/]+/blob/", source.get("resource", "")), row["id"]

    writers = records("search", BUNDLE, "--type", "Command", "--facet-filter", "mutates=true", "--full-scan")
    assert "/commands/refresh" in {row.get("id") for row in writers}
    run("search", BUNDLE, "--type", "Command", "--facet-filter", "mutates=true", "--view", "commands")
    back = records("backlinks", "commands/refresh", BUNDLE, "--details")
    assert "/guides/maintain-source-backed-knowledge" in json.dumps(back)
    assert "/skills/update" in json.dumps(back)
    edges = records("links", "guides/maintain-source-backed-knowledge", BUNDLE, "--details")
    assert any(row.get("kind") == "relationship" for row in edges), edges
    expanded = records("search", BUNDLE, "--type", "Guide", "--text", "Maintain source-backed knowledge",
                       "--in", "title", "--expand", "uses", "--target-field", "title")
    assert any(row.get("kind") == "related-concept" for row in expanded), expanded
    summary = next(row for row in expanded if row.get("kind") == "expansion-summary")
    assert not summary["truncated"] and not summary["reasons"], summary
    assert summary["emitted_edges"] == summary["total_edges"], summary
    impact = records("affected", BUNDLE, "--changed", "crates/okf-core/src/mutate/refresh.rs", "--changed-base", "repo")
    assert "/commands/refresh" in json.dumps(impact), impact
    for path in ["references/changesets/reorganize.yaml", "references/authoring/guide.yaml", "references/authoring/concept-patch.json"]:
        records("artifact", "show", path, BUNDLE)
    print(f"Showcase: {len(commands)} commands covered; views, relationships, expansion, artifacts and impact passed")


if __name__ == "__main__":
    assert BINARY.is_file(), "Build the CLI first: cargo build -p okf-cli --locked"
    check_navigation()
    check_bundle()
    check_examples()
