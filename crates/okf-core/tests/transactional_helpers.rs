use okf_core::bundle::loader::load_bundle;
use okf_core::check::{doctor::doctor, lint::rules::frontmatter_order};
use okf_core::mutate::transaction::{self, JOURNAL};
use okf_core::query::artifact::put_artifact;
use okf_core::render::index::write_indexes;
use std::fs;

#[test]
fn index_preparation_failure_does_not_publish_earlier_indexes() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("index.md"), "# Curated\n").unwrap();
    fs::create_dir(root.path().join("nested")).unwrap();
    fs::write(root.path().join("nested/a.md"), "---\ntype: Note\n---\nA\n").unwrap();
    // A directory is not a legal publication target. The root index would otherwise change.
    fs::create_dir(root.path().join("nested/index.md")).unwrap();
    let bundle = load_bundle(root.path()).unwrap();
    assert!(write_indexes(&bundle).is_err());
    assert_eq!(
        fs::read_to_string(root.path().join("index.md")).unwrap(),
        "# Curated\n"
    );
    assert!(!root.path().join(JOURNAL).exists());
}

#[test]
fn artifact_preparation_has_no_parent_directory_side_effect_on_invalid_content() {
    let root = tempfile::tempdir().unwrap();
    assert!(put_artifact(
        root.path(),
        "new/index.md",
        b"---\ntype: Note\n---\n",
        false
    )
    .is_err());
    assert!(!root.path().join("new").exists());
    assert!(!root.path().join(JOURNAL).exists());
}

#[test]
fn helper_writers_honor_the_same_exclusion_and_leave_authored_content_alone() {
    let root = tempfile::tempdir().unwrap();
    let original = "---\ntitle: A\ntype: Note\n---\nText\n";
    fs::write(root.path().join("a.md"), original).unwrap();
    fs::write(root.path().join("index.md"), "\n").unwrap();
    let bundle = load_bundle(root.path()).unwrap();
    let guard = transaction::writer_lock(root.path()).unwrap();
    assert!(write_indexes(&bundle).is_err());
    assert!(put_artifact(root.path(), "data.txt", b"data", false).is_err());
    assert!(frontmatter_order::fix(&bundle, root.path(), None).is_err());
    assert!(doctor(root.path(), "0.2", true, true).is_err());
    // Ordinary diagnostic reads remain usable while a cooperating writer holds exclusion.
    assert!(doctor(root.path(), "0.2", false, false).is_ok());
    assert_eq!(
        fs::read_to_string(root.path().join("a.md")).unwrap(),
        original
    );
    assert_eq!(
        fs::read_to_string(root.path().join("index.md")).unwrap(),
        "\n"
    );
    drop(guard);
    assert_eq!(
        frontmatter_order::fix(&bundle, root.path(), None).unwrap(),
        1
    );
    assert!(put_artifact(root.path(), "data.txt", b"data", false).is_ok());
    assert!(!root.path().join(JOURNAL).exists());
}

#[test]
fn normalization_reloads_content_instead_of_overwriting_a_stale_inventory() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("a.md"),
        "---\ntitle: A\ntype: Note\n---\nBefore\n",
    )
    .unwrap();
    let old = load_bundle(root.path()).unwrap();
    fs::write(
        root.path().join("a.md"),
        "---\ntitle: A\ntype: Note\n---\nAfter\n",
    )
    .unwrap();
    frontmatter_order::fix(&old, root.path(), None).unwrap();
    assert!(fs::read_to_string(root.path().join("a.md"))
        .unwrap()
        .ends_with("After\n"));
}
