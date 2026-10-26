use okf_core::mutate::{gate, transaction::FileChange};
use std::{fs, path::Path};

const ONTOLOGY: &str = "okf_ontology: '0.1'\nconcepts:\n  Note:\n    fields:\n      count: {type: int}\n      items: {type: list, item: int}\n    references:\n      links: {target: Note, cardinality: 0..n}\n";

fn setup() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("ontology.yaml"), ONTOLOGY).unwrap();
    root
}
fn change(root: &Path, name: &str, content: Option<&str>) -> FileChange {
    FileChange {
        path: name.into(),
        before: fs::read(root.join(name)).ok(),
        after: content.map(|s| s.as_bytes().to_vec()),
    }
}

#[test]
fn gate_grandfathers_unchanged_violation_but_not_a_different_invalid_value() {
    let root = setup();
    fs::write(
        root.path().join("a.md"),
        "---\ntype: Note\ncount: wrong\n---\nBefore\n",
    )
    .unwrap();
    gate::check_changes(
        root.path(),
        &[change(
            root.path(),
            "a.md",
            Some("---\ntype: Note\ncount: wrong\n---\nAfter\n"),
        )],
        true,
    )
    .unwrap();
    let error = gate::check_changes(
        root.path(),
        &[change(
            root.path(),
            "a.md",
            Some("---\ntype: Note\ncount: different\n---\nAfter\n"),
        )],
        true,
    )
    .unwrap_err();
    assert_eq!(error.exit_code(), 1);
}

#[test]
fn list_reordering_is_not_a_new_violation_but_added_invalid_occurrences_are() {
    let root = setup();
    fs::write(
        root.path().join("a.md"),
        "---\ntype: Note\nitems: [wrong, 1]\n---\n",
    )
    .unwrap();
    gate::check_changes(
        root.path(),
        &[change(
            root.path(),
            "a.md",
            Some("---\ntype: Note\nitems: [1, 2, wrong]\n---\n"),
        )],
        true,
    )
    .unwrap();
    assert!(gate::check_changes(
        root.path(),
        &[change(
            root.path(),
            "a.md",
            Some("---\ntype: Note\nitems: [wrong, wrong]\n---\n")
        )],
        true
    )
    .is_err());
}

#[test]
fn unambiguous_move_preserves_existing_findings_and_reference_identities() {
    let root = setup();
    fs::write(
        root.path().join("a.md"),
        "---\ntype: Note\ncount: wrong\n---\n",
    )
    .unwrap();
    let changes = vec![
        change(root.path(), "a.md", None),
        change(
            root.path(),
            "new.md",
            Some("---\ntype: Note\ncount: wrong\n---\n"),
        ),
    ];
    gate::check_changes(root.path(), &changes, true).unwrap();
}

#[test]
fn deleting_a_typed_reference_target_is_a_new_gate_failure() {
    let root = setup();
    fs::write(
        root.path().join("a.md"),
        "---\ntype: Note\nlinks: [/b]\n---\n",
    )
    .unwrap();
    fs::write(root.path().join("b.md"), "---\ntype: Note\n---\n").unwrap();
    assert!(gate::check_changes(root.path(), &[change(root.path(), "b.md", None)], true).is_err());
}

#[test]
fn stricter_ontology_is_checked_against_all_existing_concepts() {
    let root = setup();
    fs::write(root.path().join("a.md"), "---\ntype: Note\ncount: 2\n---\n").unwrap();
    let before = okf_core::ontology::load::parse_ontology(ONTOLOGY).unwrap();
    let after = okf_core::ontology::load::parse_ontology(
        &ONTOLOGY.replace("count: {type: int}", "count: {type: int, min: 5}"),
    )
    .unwrap();
    assert!(gate::check_ontology_change(root.path(), Some(&before), &after, true).is_err());
}

#[test]
fn unsupported_constraints_cannot_be_reported_as_a_passing_gate() {
    let root = setup();
    fs::write(
        root.path().join("ontology.yaml"),
        ONTOLOGY.replace(
            "count: {type: int}",
            "count: {type: int, custom_constraint: true}",
        ),
    )
    .unwrap();
    fs::write(root.path().join("a.md"), "---\ntype: Note\ncount: 2\n---\n").unwrap();
    let result = gate::check_changes(
        root.path(),
        &[change(
            root.path(),
            "a.md",
            Some("---\ntype: Note\ncount: 3\n---\n"),
        )],
        true,
    );
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("write-gate-incomplete"));
}
