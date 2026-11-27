#!/usr/bin/env python3
"""Check manual navigation and exercise the actual published examples. Stdlib only."""
import collections
import json
import os
from pathlib import Path
import re
import runpy
import shlex
import subprocess
import tempfile
from doc_examples import run_guide
from urllib.parse import unquote, urlsplit

ROOT = Path(__file__).resolve().parents[1]
BUNDLE = ROOT / "knowledge"
BINARY = Path(os.environ.get("OKF_DOCS_BIN", ROOT / "target/debug/okf")).resolve()
ENV = {k: v for k, v in os.environ.items() if k not in ("OKF_BUNDLE", "OKF_CATALOG", "OKF_CATALOG_OVERRIDES", "OKF_SETTINGS_OVERRIDES")}


def run(*args, expected=0, cwd=ROOT):
    result = subprocess.run([str(BINARY), *map(str, args)], cwd=cwd, env=ENV,
                            text=True, capture_output=True)
    if result.returncode != expected:
        raise AssertionError(f"{args}: exit {result.returncode}\n{result.stdout}\n{result.stderr}")
    return result.stdout


def run_cli(args, directory):
    return run(*args, cwd=directory)


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


def check_structured_guide(directory):
    outputs = run_guide(BUNDLE / "guides/query-structured-records.md", directory, run_cli)
    rows = [json.loads(line) for args, text in outputs if "--json" in args
            for line in text.splitlines()]
    assert any(row.get("kind") == "projection" and row["id"] == "/procedures/onboarding" for row in rows)
    assert any(row.get("kind") == "related-concept" and row["id"] == "/teams/platform" for row in rows)
    assert any(row.get("kind") == "facet" for row in rows)


def check_remote_guide(directory):
    # Exercise Git transport against an isolated local upstream; never contact the sample host.
    upstream = directory / "upstream"
    upstream.mkdir(parents=True)
    consumer = directory / "consumer"
    run("init", consumer / "knowledge")
    run("init", upstream / "bundles/products")
    run("add", "products/widget", upstream / "bundles/products", "--type", "Product", "--title", "Widget")
    for args in [["init", "-q", "-b", "main"], ["add", "."],
                 ["-c", "user.name=Guide", "-c", "user.email=guide@example.invalid",
                  "-c", "commit.gpgsign=false", "commit", "-qm", "Product bundle"]]:
        subprocess.run(["git", *args], cwd=upstream, check=True)
    env = {**ENV, "OKF_CACHE_DIR": str(directory / "cache"), "OKF_GUIDE_UPSTREAM": str(upstream),
           "GIT_SSH_VARIANT": "ssh",
           "GIT_SSH_COMMAND": """sh -c 'exec git-upload-pack "$OKF_GUIDE_UPSTREAM"' --"""}

    def remote(args, cwd):
        completed = subprocess.run([str(BINARY), *args], cwd=cwd, env=env, capture_output=True,
                                   text=True, check=True)
        return completed.stdout

    outputs = run_guide(BUNDLE / "guides/remote-dependencies.md", consumer, remote)
    edges = [json.loads(line) for args, text in outputs if args[0] == "links"
             for line in text.splitlines()]
    widget = next(row for row in edges if row.get("kind") == "bundle-edge")
    assert widget["target"]["bundle"] == "acme.products" and widget["target"]["id"] == "/products/widget"
    assert widget["status"] == "resolved" and widget["target"]["version"].startswith("git:")


def run_customization_step(args, cwd):
    text = run_cli(args, cwd)
    if "--token" in args:
        rows = [json.loads(line) for line in text.splitlines()]
        assert rows[-1]["kind"] == "operation-complete" and rows[-1]["exit_code"] == 0
        receipt = next(row["token"] for row in rows if row["kind"] == "context-token")
        (cwd / "context-token.json").write_text(receipt)
    return text


def check_feedback_guide(directory):
    directory.mkdir()
    env = {**ENV, "OKF_FEEDBACK_DIR": str(directory / "store")}
    draft, preview, confirm, *patterns = blocks("guides/capture-local-feedback.md")

    def feedback(command):
        cli, *args = shlex.split(command)
        assert cli == "okf"
        result = subprocess.run([str(BINARY), *args, "--json"], cwd=directory, env=env,
                                capture_output=True, text=True, check=True)
        rows = [json.loads(line) for line in result.stdout.splitlines()]
        assert rows[-1]["kind"] == "operation-complete" and rows[-1]["outcome"] == "ok"
        return next(row["data"] for row in rows if row.get("kind") == "feedback")

    managed = Path(feedback(draft)["file"])
    destination = directory / "reviewed-feedback.md"
    reviewed = feedback(preview.replace("draft-AbCd12.md", managed.name))
    assert Path(reviewed["destination"]) == destination.resolve() and not destination.exists()
    assert reviewed["text"] == managed.read_text()
    feedback(confirm.replace("draft-AbCd12.md", managed.name)
             .replace("DIGEST_FROM_REVIEWED_PREVIEW", reviewed["confirmation"]))
    assert destination.read_bytes() == managed.read_bytes()
    for command in patterns:
        feedback(command)


def check_examples():
    with tempfile.TemporaryDirectory(prefix="okf-docs-") as tmp:
        tmp = Path(tmp)
        model_root = tmp / "model"
        model = run_guide(BUNDLE / "guides/model-your-knowledge.md", model_root,
                          run_cli)
        assert (model_root / "guide.yaml").read_text() == (BUNDLE / "references/authoring/guide.yaml").read_text()
        assert json.loads((model_root / "concept-patch.json").read_text()) == json.loads(
            (BUNDLE / "references/authoring/concept-patch.json").read_text())
        concept = model_root / "bundle/guides/example.md"
        rows = records("show", "guides/example", model_root / "bundle")
        authored = next(row for row in rows if row.get("id") == "/guides/example")
        assert authored["audience"] == "contributor" and "authoring" in authored["tags"]
        assert authored["uses"] == ["/commands/check"], authored
        assert any(args[0] == "backlinks" and "/guides/example" in output for args, output in model)
        before = concept.read_bytes()
        run("edit", "guides/example", "bundle", "--patch", "@concept-patch.json", cwd=model_root, expected=2)
        assert concept.read_bytes() == before, "A failed guarded patch changed the concept"

        move_root = tmp / "move"
        bundle = move_root / "bundle"
        digest = None

        def exercise_move(args, directory):
            nonlocal digest
            if args[:2] == ["changeset", "apply"]:
                assert digest and args[-2:] == ["--expect", "REVIEWED_BASE_DIGEST"]
                args = [*args[:-1], digest]
            output = run(*args, cwd=directory)
            if args[:2] == ["changeset", "plan"]:
                plan = [json.loads(line) for line in output.splitlines()]
                assert plan[-1]["outcome"] == "ok"
                digest = next(row["base_digest"] for row in plan if row.get("kind") == "changeset-summary")
                assert (bundle / "plans/current.md").exists(), "Preview published the move"
            return output

        run_guide(BUNDLE / "guides/reorganize-a-bundle.md", move_root, exercise_move)
        assert (move_root / "reorganize.yaml").read_text() == (BUNDLE / "references/changesets/reorganize.yaml").read_text()
        assert (bundle / "plans/revised.md").exists()
        assert not (bundle / "plans/current.md").exists()
        assert not (bundle / "plans/obsolete.md").exists()
        assert "../plans/revised.md" in (bundle / "notes/consumer.md").read_text()

        # The quickstart must work independently of this repository's default bundle.
        run_guide(BUNDLE / "getting-started/first-bundle.md", tmp, run_cli)

        maintenance = tmp / "maintenance"
        steps = run_guide(BUNDLE / "guides/maintain-source-backed-knowledge.md", maintenance,
                          run_cli)
        assert any(args[0] == "affected" and "/notes/service" in output for args, output in steps)
        assert any(args[:2] == ["artifact", "show"] and "retry_limit=3" in output for args, output in steps)
        assert any(args[0] == "show" and "The service retries twice." in output for args, output in steps)
        maintained = records("show", "notes/service", maintenance / "bundle")
        authored = next(row for row in maintained if row.get("id") == "/notes/service")
        assert authored["sources"] == [{"resource": "references/service.txt", "kind": "file"}], authored
        assert "three times" in run("show", "notes/service", maintenance / "bundle", "--body")

        customization = tmp / "customization"
        run_guide(BUNDLE / "guides/customize-a-bundle.md", customization, run_customization_step)
        custom_bundle = customization / "bundle"
        missing = records("bundle", "context", custom_bundle, "--workflow", "import-specification", cwd=customization)
        missing_workflow = next(row for row in missing if row.get("kind") == "workflow")
        assert missing_workflow["inputs_complete"] is False
        assert missing_workflow["missing_required"] == ["source"]
        selected_workflow = records("bundle", "context", custom_bundle, "--workflow", "import-specification",
                                    "--inputs", '{"source":"supplied-vendor-specification"}', cwd=customization)
        workflow = next(row for row in selected_workflow if row.get("kind") == "workflow")
        assert workflow["inputs_complete"] is True
        assert workflow["effective_inputs"]["review_level"] == "standard"
        assert workflow["input_origins"]["review_level"] == "default"
        assert (custom_bundle / ".okf-bundle/references/import-specification.md").exists()
        inspected = records("bundle", "context", custom_bundle, "--all", cwd=customization)
        assert all(row["instruction_role"] == "inspected" for row in inspected
                   if row.get("kind") == "guidance")
        assert not next(row for row in inspected if row.get("kind") == "workflow")["selected"]
        viewed = records("list", custom_bundle, "--view", "overview", cwd=customization)
        assert any(row.get("kind") == "projection" for row in viewed), viewed
        preset_query = records("bundle", "context", custom_bundle, "--operation", "list",
                               "--query", '{"preset":"documented-notes"}', cwd=customization)
        assert next(row for row in preset_query if row.get("kind") == "effective-query")["query"]["type"] == "Note"
        inventory = records("list", custom_bundle, cwd=customization)
        assert [row["id"] for row in inventory if row.get("trust_tier") is not None] == ["/notes/overview"]
        run("validate", custom_bundle, "--profile", "bundle", cwd=customization)
        run("validate", custom_bundle, expected=1, cwd=customization)

        catalog = tmp / "catalog"
        steps = run_guide(BUNDLE / "guides/work-with-multiple-bundles.md", catalog, run_cli)
        saved = next(output for args, output in steps if args[0] == "query")
        endpoints = [row for row in map(json.loads, saved.splitlines()) if row.get("kind") == "projection"]
        assert [(row["bundle"], row["id"]) for row in endpoints] == [("demo.policy", "/policies/retention")]
        assert endpoints[0]["fields"][0]["occurrences"][0]["value"] == "Retention rule"
        check_remote_guide(tmp / "remote")
        selected = records("search", "--bundle-id", "demo.product", "--text", "retention", cwd=catalog)
        scoped = records("search", "--bundle-id", "demo.product", "--scope-bundle", "demo.policy", "--text", "retention", cwd=catalog)
        assert next(row for row in selected if row.get("kind") == "query-summary")["total_matches"] == 1
        assert next(row for row in scoped if row.get("kind") == "query-summary")["total_matches"] == 2
        back = records("backlinks", "demo.policy:/policies/retention", "--bundle-id", "demo.product", "--scope-bundle", "demo.policy", "--details", cwd=catalog)
        assert "/notes/overview" in json.dumps(back), back

        for guide in ["bounded-reads", "tour-the-bundle"]:
            run_guide(BUNDLE / f"guides/{guide}.md", ROOT, run_cli)

        find = run_guide(BUNDLE / "guides/find-and-read-knowledge.md", ROOT,
                         run_cli)
        filtered = [output for args, output in find if args[0] == "list"]
        assert "/commands/show" in filtered[0] and "(no concepts)" in filtered[1]
        topics = next(output for args, output in find if args[0] == "links")
        assert "/concepts/sources-and-provenance" in topics and "/concepts/verification-and-freshness" in topics
        saved = tmp / "saved"
        examples = run_guide(BUNDLE / "guides/save-and-run-queries.md", saved,
                             run_cli)
        contexts = [output for args, output in examples if args[:2] == ["bundle", "context"]]
        assert [output.count("[saved query ") for output in contexts] == [0, 1, 4]
        assert "[saved query triage-notes]" in contexts[1]
        assert any("/defects/csv" in output for args, output in examples if "all-open-blockers" in args)
        rows = [json.loads(line) for args, result in examples if "--json" in args
                for line in result.splitlines()]
        projections = [row for row in rows if row.get("kind") == "projection"]
        assert [row["id"] for row in projections] == ["/notes/triage", "/notes/triage"]
        summaries = {row["name"]: row for row in rows if row.get("kind") == "saved-query-summary"}
        assert summaries["triage-notes"]["strategy"] == "scope"
        direct = summaries["triage-for-defect"]
        assert direct["strategy"] == "outgoing" and direct["examination"]["examined_documents"] == 2
        assert all(row["matching_complete"] and row["delivery_complete"] for row in summaries.values())
        section = run("query", "notes-for-open-blockers", "features/export", "--bundle",
                      saved / "product", "--section", '["Operations","Recovery"]', "--json")
        section_rows = [json.loads(line) for line in section.splitlines()]
        excerpt = next(row for row in section_rows if row.get("kind") == "section")
        assert excerpt["id"] == "/notes/triage" and "Retry the export" in excerpt["text"]
        assert "Escalation" not in excerpt["text"]
        assert section_rows[-1]["completeness"]["delivery"] == "complete"

        check_feedback_guide(tmp / "feedback")
        check_structured_guide(tmp / "structured")

        inventory = runpy.run_path(str(BUNDLE / "references/command_inventory.py"))["command_inventory"]
        inventory_examples = run_guide(BUNDLE / "guides/automate-with-json.md", ROOT, run_cli)
        complete, partial = [output for _, output in inventory_examples]
        assert "/commands/refresh" in {row["id"] for row in inventory(complete, 0)}
        for label, output, code in [
            ("missing completion", "\n".join(complete.splitlines()[:-1]), 0),
            ("failed process", complete, 3),
            ("partial scan", partial, 0),
        ]:
            try:
                inventory(output, code)
            except ValueError:
                continue
            raise AssertionError("Consumer accepted " + label)
    print("Worked examples: guide and quickstart examples passed; saved-query endpoints and sections, reviewed feedback exports, and JSON completeness checked")


def check_bundle():
    records("bundle", "validate", BUNDLE)
    records("validate", BUNDLE, "--profile", "bundle")
    lint = records("lint", BUNDLE, "--fail-on", "any")
    actionable = [row for row in lint if row.get("rule") in
                  ("broken-link", "ontology-violation", "index-coverage", "frontmatter-order")]
    assert not actionable, actionable
    notices = collections.Counter(row["rule"] for row in lint if row.get("kind") == "finding")
    print("Bundle tooling profile: conformant; manifest valid; advisory findings:", dict(notices))

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

    context = records("bundle", "context", BUNDLE)
    settings = next(row for row in context if row.get("kind") == "bundle-context")["effective_settings"]
    assert settings["bundle"] == "okf.docs", settings
    assert settings["settings"]["lint"]["source_unrecorded"] == "off"
    assert set(settings["settings"]["views"]) >= {"commands", "guides"}
    preset_writers = records("list", BUNDLE, "--preset", "writing-commands", "--full-scan")
    assert any(row.get("id") == "/commands/refresh" for row in preset_writers)
    assert all(row.get("mutates") is True for row in preset_writers if row.get("type") == "Command")
    guide_users = records("backlinks", "commands/refresh", BUNDLE, "--preset", "guides-using-command")
    assert "/guides/maintain-source-backed-knowledge" in json.dumps(guide_users)
    workflow = records("bundle", "context", BUNDLE, "--workflow", "check-documentation")
    assert any(row.get("kind") == "workflow" and row.get("selected") is True for row in workflow)
    writers = records("search", BUNDLE, "--type", "Command", "--facet-filter", "mutates=true", "--full-scan")
    assert "/commands/refresh" in {row.get("id") for row in writers}
    run("search", BUNDLE, "--type", "Command", "--facet-filter", "mutates=true", "--view", "commands")
    back = records("backlinks", "commands/refresh", BUNDLE, "--details")
    assert "/guides/maintain-source-backed-knowledge" in json.dumps(back)
    assert "/skills/update" in json.dumps(back)
    edges = records("links", "guides/maintain-source-backed-knowledge", BUNDLE, "--details")
    assert any(row.get("kind") == "relationship" or
               row.get("kind") == "bundle-edge" and row.get("relationship") for row in edges), edges
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
    print(f"Showcase: {len(commands)} commands covered; manifest/catalog, lint policy, views, presets, workflow context, relationships, expansion, artifacts and impact passed")


if __name__ == "__main__":
    assert BINARY.is_file(), "Build the CLI first: cargo build -p okf-cli --locked"
    check_navigation()
    check_bundle()
    check_examples()
