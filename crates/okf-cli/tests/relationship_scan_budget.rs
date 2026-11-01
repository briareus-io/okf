use assert_cmd::prelude::*;
use serde_json::Value;
use std::{
    path::Path,
    process::{Command, Output},
};
fn write(root: &Path, path: &str, text: &str) {
    let path = root.join(path);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}
fn run(root: &Path, args: &[&str]) -> Output {
    Command::cargo_bin("okf")
        .unwrap()
        .current_dir(root)
        .env_remove("OKF_BUNDLE")
        .env_remove("OKF_CATALOG")
        .env_remove("OKF_CATALOG_OVERRIDES")
        .args(args)
        .output()
        .unwrap()
}
fn records(out: &Output) -> Vec<Value> {
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}
fn fixture(root: &Path, prefix: &str) {
    write(
        root,
        &format!("{prefix}a.md"),
        "---\ntype: Note\ntitle: A\n---\n[target](/target.md)\n",
    );
    write(root, &format!("{prefix}z.md"), "---\ntype: [broken\n---\n");
}
#[test]
fn local_budget_bounds_parsing_and_counts_nonmatching_documents() {
    let d = tempfile::tempdir().unwrap();
    fixture(d.path(), "");
    let out = records(&run(
        d.path(),
        &["backlinks", "target", ".", "--scan-limit", "1", "--json"],
    ));
    let summary = out
        .iter()
        .find(|r| r["kind"] == "relationship-query-summary")
        .unwrap();
    assert_eq!(summary["evidence"]["examined_documents"], 1);
    assert_eq!(summary["eligible_documents"], 2);
    assert_eq!(summary["evidence"]["scan_complete"], false);
    assert!(summary["total_matches"].is_null());
    assert!(!run(
        d.path(),
        &["backlinks", "target", ".", "--full-scan", "--json"]
    )
    .status
    .success());
    let out = records(&run(
        d.path(),
        &[
            "backlinks",
            "unreferenced",
            ".",
            "--scan-limit",
            "1",
            "--json",
        ],
    ));
    let summary = out
        .iter()
        .find(|r| r["kind"] == "relationship-query-summary")
        .unwrap();
    assert_eq!(summary["evidence"]["examined_documents"], 1);
    assert_eq!(summary["evidence"]["scan_complete"], false);
}
#[test]
fn outgoing_filtered_anchor_must_exist_and_is_prioritized() {
    let d = tempfile::tempdir().unwrap();
    fixture(d.path(), "");
    records(&run(
        d.path(),
        &["links", "a", ".", "--scan-limit", "1", "--json"],
    ));
    assert!(!run(
        d.path(),
        &[
            "links",
            "absent",
            ".",
            "--kind",
            "blocks",
            "--scan-limit",
            "1"
        ]
    )
    .status
    .success());
    // A malformed unrelated filename sorting before the named source is not examined first.
    write(d.path(), "0.md", "---\ntype: [broken\n---\n");
    records(&run(
        d.path(),
        &["links", "a", ".", "--scan-limit", "1", "--json"],
    ));
}
#[test]
fn catalog_budget_bounds_parsing_before_graph_construction() {
    let d = tempfile::tempdir().unwrap();
    fixture(d.path(), "bundle/");
    write(d.path(),"okf-catalog.yaml","catalog_version: 2\ndefault_bundle: app.docs\nbundles:\n  app.docs:\n    location: {type: directory, path: bundle}\n");
    let out = records(&run(
        d.path(),
        &[
            "backlinks",
            "unreferenced",
            "--bundle-id",
            "app.docs",
            "--scan-limit",
            "1",
            "--json",
        ],
    ));
    let summary = out
        .iter()
        .find(|r| r["kind"] == "relationship-query-summary")
        .unwrap();
    assert_eq!(summary["examined_documents"], 1);
    assert_eq!(summary["eligible_documents"], 2);
    assert_eq!(summary["scan_complete"], false);
    assert!(summary["total_matches"].is_null());
    records(&run(
        d.path(),
        &[
            "links",
            "a",
            "--bundle-id",
            "app.docs",
            "--scan-limit",
            "1",
            "--json",
        ],
    ));
    assert!(!run(
        d.path(),
        &[
            "links",
            "absent",
            "--bundle-id",
            "app.docs",
            "--scan-limit",
            "1",
            "--kind",
            "blocks"
        ]
    )
    .status
    .success());
    assert!(!run(
        d.path(),
        &[
            "backlinks",
            "unreferenced",
            "--bundle-id",
            "app.docs",
            "--full-scan"
        ]
    )
    .status
    .success());
}
