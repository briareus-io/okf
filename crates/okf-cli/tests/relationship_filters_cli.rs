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
