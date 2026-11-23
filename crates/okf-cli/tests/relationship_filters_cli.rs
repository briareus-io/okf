use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;
fn setup() -> tempfile::TempDir {
    let d = tempdir().unwrap();
    std::fs::write(d.path().join("index.md"), "# Bundle\n").unwrap();
    std::fs::write(
        d.path().join("feature.md"),
        "---\ntype: Feature\n---\n# Feature\n",
    )
    .unwrap();
    std::fs::write(
        d.path().join("defect.md"),
        "---\ntype: Defect\nblocks: /feature.md\n---\n# Defect\n",
    )
    .unwrap();
    std::fs::write(
        d.path().join("note.md"),
        "---\ntype: Note\n---\n[Feature](/feature.md)\n",
    )
    .unwrap();
    std::fs::write(d.path().join("ontology.yaml"),"okf_ontology: '0.1'\nconcepts:\n  Feature: {}\n  Defect:\n    references:\n      blocks: {target: Feature, cardinality: '0..1'}\n    relationships:\n      blocking: {reference: blocks, kind: blocks, inverse: is-blocked-by}\n").unwrap();
    std::fs::write(d.path().join("okf-bundle.yaml"),"manifest_version: 1\nsettings:\n  query_presets:\n    blockers:\n      operations: [backlinks, graph]\n      filters:\n        kind: [blocks]\n        related_type: [Defect]\n").unwrap();
    d
}
#[test]
fn local_preset_filters_semantic_occurrences() {
    let d = setup();
    Command::cargo_bin("okf")
        .unwrap()
        .args([
            "backlinks",
            "feature",
            d.path().to_str().unwrap(),
            "--preset",
            "blockers",
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("query-filters"))
        .stdout(predicate::str::contains("\"id\":\"/defect\""))
        .stdout(predicate::str::contains("\"id\":\"/note\"").not());
}
#[test]
fn scoped_filters_match_inverse_and_other_endpoint() {
    let d = setup();
    let cat = d.path().join("catalog.yaml");
    std::fs::write(&cat,format!("catalog_version: 2\nbundles:\n  test.bundle:\n    location: {{type: directory, path: '{}' }}\n",d.path().display())).unwrap();
    Command::cargo_bin("okf")
        .unwrap()
        .args([
            "backlinks",
            "feature",
            "--catalog",
            cat.to_str().unwrap(),
            "--bundle-id",
            "test.bundle",
            "--inverse-kind",
            "is-blocked-by",
            "--related-type",
            "Defect",
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("bundle-backlink"))
        .stdout(predicate::str::contains("\"id\":\"/note\"").not());
}
#[test]
fn preset_query_context_uses_execution_resolver() {
    let d = setup();
    Command::cargo_bin("okf")
        .unwrap()
        .args([
            "bundle",
            "context",
            d.path().to_str().unwrap(),
            "--operation",
            "backlinks",
            "--query",
            r#"{"preset":"blockers"}"#,
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("effective-query"))
        .stdout(predicate::str::contains("blocks"));
}
#[test]
fn invalid_clear_and_supplied_category_is_error() {
    let d = setup();
    Command::cargo_bin("okf")
        .unwrap()
        .args([
            "backlinks",
            "feature",
            d.path().to_str().unwrap(),
            "--kind",
            "blocks",
            "--clear-filter",
            "kind",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot supply and clear"));
}

#[test]
fn saved_queries_compose_relationships_then_select_endpoint_sections() {
    let d = setup();
    let write = |path: &str, contents: &str| std::fs::write(d.path().join(path), contents).unwrap();
    write(
        "okf-bundle.yaml",
        r#"manifest_version: 1
settings:
  query_presets:
    features:
      operations: [list]
      query: {type: Feature}
  saved_queries:
    notes:
      from: {list: {preset: features}}
      steps:
        - backlinks: {kind: [blocks], related_type: [Defect]}
        - links: {related_type: [Note]}
      select: [owner]
    direct:
      from: concept
"#,
    );
    write(
        "defect.md",
        "---\ntype: Defect\nblocks: /feature.md\n---\n[Note](/note.md) [Other](/other.md)",
    );
    write(
        "note.md",
        "---\ntype: Note\ntags: [urgent]\nowner: example\n---\n# Operations\n## Recovery\nRestart.\n### Details\nCheck.\n## Another\nSkip.\n",
    );
    write(
        "other.md",
        "---\ntype: Note\n---\n# Operations\n## Recovery\n## Recovery\n",
    );
    let rows = |args: &[&str]| -> Vec<serde_json::Value> {
        let output = Command::cargo_bin("okf")
            .unwrap()
            .env("OKF_BUNDLE", d.path())
            .current_dir(d.path())
            .args(args)
            .arg("--json")
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        String::from_utf8(output)
            .unwrap()
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect()
    };
    let path = r#"["Operations","Recovery"]"#;
    let show = rows(&["show", "note", "--section", path]);
    let section = show.iter().find(|r| r["kind"] == "section").unwrap();
    assert_eq!(
        section["text"],
        "## Recovery\nRestart.\n### Details\nCheck.\n"
    );
    let query = rows(&["query", "notes", "--tag", "urgent", "--section", path]);
    let actual = query.iter().find(|r| r["kind"] == "section").unwrap();
    assert_eq!(actual["text"], section["text"]);
    assert_eq!(actual["id"], "/note");
    let all = rows(&["query", "notes", "--section", path]);
    assert!(
        all.iter()
            .any(|r| r["kind"] == "warning" && r["id"] == "/other")
    );
    assert_eq!(all.last().unwrap()["completeness"]["delivery"], "partial");
    let limited = rows(&["query", "direct", "other", "--scan-limit", "1"]);
    assert!(limited.iter().any(|r| r["id"] == "/other"));
    assert_eq!(
        limited.last().unwrap()["completeness"]["examination"],
        "complete"
    );
    let projected = rows(&["query", "notes", "--limit", "1"]);
    assert_eq!(
        projected
            .iter()
            .filter(|r| r["kind"] == "projection")
            .count(),
        1
    );
    assert!(
        projected
            .iter()
            .any(|r| r["kind"] == "saved-query-summary" && r["observed_matches"] == 2)
    );
}

#[test]
fn outbound_reads_only_reached_concepts_and_preserves_scoped_results() {
    let d = setup();
    let write = |name: &str, text: &str| std::fs::write(d.path().join(name), text).unwrap();
    write(
        "okf-catalog.yaml",
        "catalog_version: 2\nbundles:\n  test.bundle: {location: {type: directory, path: .}}\n",
    );
    write(
        "okf-bundle.yaml",
        r#"manifest_version: 1
settings:
  saved_queries:
    chain:
      from: concept
      steps:
        - links: {kind: [blocks], related_type: [Feature]}
        - links: {related_filter: ['owner exists']}
        - links: {}
      select: [title]
"#,
    );
    write(
        "defect.md",
        "---\ntype: Defect\nblocks: /feature.md\n---\n[irrelevant](/unreached.md)\n",
    );
    write(
        "feature.md",
        "---\ntype: Feature\n---\n[one](/one.md) [two](/two.md) [missing](/absent.md)\n",
    );
    for note in ["one.md", "two.md"] {
        write(
            note,
            "---\ntype: Note\nowner: team\n---\n[start](/defect.md)\n",
        );
    }
    let cat = d.path().join("okf-catalog.yaml");
    let run = |extra: &[&str]| {
        let out = Command::cargo_bin("okf")
            .unwrap()
            .current_dir(d.path())
            .args([
                "query",
                "chain",
                "defect",
                "--catalog",
                cat.to_str().unwrap(),
                "--bundle-id",
                "test.bundle",
                "--json",
            ])
            .args(extra)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8(out.stdout)
            .unwrap()
            .lines()
            .map(|l| serde_json::from_str::<serde_json::Value>(l).unwrap())
            .collect::<Vec<_>>()
    };
    let summary = |rows: &[serde_json::Value]| {
        rows.iter()
            .find(|r| r["kind"] == "saved-query-summary")
            .unwrap()
            .clone()
    };
    let full = run(&["--scope-bundle", "test.bundle", "--full-scan"]);
    write("unreached.md", "---\ntype: [broken\n---\n");
    let local = run(&["--full-scan"]);
    let left = summary(&local);
    let right = summary(&full);
    assert_eq!(right["strategy"], "scope");
    assert_eq!(left["strategy"], "outgoing");
    assert_eq!(left["examination"]["examined_documents"], 4);
    assert_eq!(left["unresolved"], 1);
    for key in [
        "stages",
        "unresolved",
        "observed_matches",
        "matching_complete",
        "delivery_complete",
    ] {
        assert_eq!(left[key], right[key]);
    }
    let projection = |rows: &[serde_json::Value]| {
        rows.iter()
            .find(|r| r["kind"] == "projection")
            .unwrap()
            .clone()
    };
    assert_eq!(projection(&local), projection(&full));
    assert_eq!(projection(&local)["id"], "/defect");
    let bounded = summary(&run(&["--scan-limit", "1"]));
    assert_eq!(bounded["examination"]["examined_documents"], 1);
    assert_eq!(bounded["examination"]["eligible_documents"], 2);
    assert_eq!(bounded["matching_complete"], false);
    assert_eq!(bounded["observed_matches"], 0);
    write("feature.md", "---\ntype: [broken\n---\n");
    Command::cargo_bin("okf")
        .unwrap()
        .current_dir(d.path())
        .args([
            "query",
            "chain",
            "defect",
            "--catalog",
            cat.to_str().unwrap(),
            "--bundle-id",
            "test.bundle",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("feature.md"));
}
