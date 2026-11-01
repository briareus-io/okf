use okf_core::{
    mutate::{
        init::{init, InitOptions},
        transaction::{self, FileChange},
    },
    ontology,
};
use std::path::PathBuf;

fn snapshot() -> (tempfile::TempDir, PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let tree = temp.path().join("tree");
    std::fs::create_dir_all(&tree).unwrap();
    std::fs::write(temp.path().join("snapshot.yaml"), "profile: 1\nurl: https://example.invalid/upstream.git\ncommit: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\nobject_format: sha1\nmembers: {}\n").unwrap();
    (temp, tree)
}
#[test]
fn direct_managed_path_rejects_mutation_before_lock_or_directory_creation() {
    let (_temp, tree) = snapshot();
    let original = b"---\ntype: Note\n---\nOriginal\n";
    std::fs::write(tree.join("note.md"), original).unwrap();
    let change = FileChange {
        path: PathBuf::from("note.md"),
        before: Some(original.to_vec()),
        after: Some(b"changed".to_vec()),
    };
    assert!(transaction::commit(&tree, vec![change])
        .unwrap_err()
        .to_string()
        .contains("read-only"));
    assert_eq!(std::fs::read(tree.join("note.md")).unwrap(), original);
    assert!(!tree.join(".okf-transaction.lock").exists());
    assert!(init(&tree.join("new"), &InitOptions::default()).is_err());
    assert!(!tree.join("new").exists());
    assert!(transaction::recover_with_outcome(&tree).is_err());
}
#[test]
fn standalone_ontology_writer_cannot_bypass_managed_policy() {
    let (_temp, tree) = snapshot();
    let model = ontology::load::parse_ontology("okf_ontology: '0.1'\nconcepts: {}\n").unwrap();
    assert!(
        ontology::edit::save_ontology(&tree.join("ontology.yaml"), &model)
            .unwrap_err()
            .to_string()
            .contains("read-only")
    );
    assert!(!tree.join("ontology.yaml").exists());
}
#[cfg(unix)]
#[test]
fn symlink_alias_does_not_make_managed_snapshot_writable() {
    let (_temp, tree) = snapshot();
    let alias = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(&tree, alias.path().join("alias")).unwrap();
    assert!(transaction::writer_lock(&alias.path().join("alias")).is_err());
    assert!(!tree.join(".okf-transaction.lock").exists());
}
