use okf_core::bundle::{catalog::declared_directory_entries, remote_impact::report_snapshots};
use std::{collections::BTreeMap, fs};
#[test]
fn impact_uses_overridden_consumer_root_and_preserves_its_registered_identity() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    for dir in ["configured", "actual", "candidate"] {
        fs::create_dir(root.join(dir)).unwrap();
    }
    fs::write(
        root.join("configured/wrong.md"),
        "---\ntype: Note\n---\nWrong checkout\n",
    )
    .unwrap();
    fs::write(
        root.join("actual/right.md"),
        "---\ntype: Note\n---\n[Dependency](okf://test.remote/note.md)\n",
    )
    .unwrap();
    fs::write(
        root.join("candidate/note.md"),
        "---\ntype: Note\n---\nCandidate\n",
    )
    .unwrap();
    fs::write(
        root.join("overrides.yaml"),
        "bundles:\n  test.consumer: actual\n",
    )
    .unwrap();
    fs::write(root.join("okf-catalog.yaml"), "catalog_version: 2\ncatalog_overrides: overrides.yaml\nbundles:\n  test.consumer:\n    location: {type: directory, path: configured}\n  test.remote:\n    location: {type: git, url: 'https://example.invalid/repo.git', ref: refs/heads/main, path: bundle}\n").unwrap();
    let catalog = root.join("okf-catalog.yaml");
    let entries = declared_directory_entries(&catalog).unwrap();
    assert!(entries["test.consumer"].overridden);
    assert_eq!(
        entries["test.consumer"].root,
        root.join("actual").canonicalize().unwrap()
    );
    let after = BTreeMap::from([("test.remote".to_string(), root.join("candidate"))]);
    let report = report_snapshots(
        &catalog,
        &BTreeMap::new(),
        &after,
        &[entries["test.consumer"].root.clone()],
    )
    .unwrap();
    assert_eq!(report.consumers.len(), 1);
    assert_eq!(report.consumers[0].bundle, "test.consumer");
    assert!(report.consumers[0]
        .after_links
        .iter()
        .any(|e| e.source.contains("right")));
    assert!(!report.consumers[0]
        .after_links
        .iter()
        .any(|e| e.source.contains("wrong")));
}
