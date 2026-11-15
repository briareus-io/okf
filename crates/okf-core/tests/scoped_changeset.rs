use okf_core::{
    bundle::{context, snapshot_context},
    mutate::{
        changeset::{self, ChangeSet, Operation},
        transaction,
    },
};
use std::fs;

fn fixture() -> (tempfile::TempDir, snapshot_context::SnapshotContextGuard) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    for name in ["planning", "product"] {
        fs::create_dir(root.join(name)).unwrap();
    }
    fs::write(root.join("okf-catalog.yaml"), "catalog_version: 2\nbundles:\n  planning.main:\n    location: {type: directory, path: planning}\n  product.main:\n    location: {type: directory, path: product}\n").unwrap();
    fs::write(root.join("planning/ontology.yaml"), "okf_ontology: '0.1'\nconcepts:\n  Increment:\n    references:\n      specifications:\n        selector: checkpoint.specifications[]\n        target: product.main:Specification\n        cardinality: 0..n\n").unwrap();
    fs::write(
        root.join("product/ontology.yaml"),
        "okf_ontology: '0.1'\nconcepts:\n  Specification: {}\n",
    )
    .unwrap();
    fs::write(
        root.join("product/spec.md"),
        "---\ntype: Specification\n---\n",
    )
    .unwrap();
    let context = context::resolve_context(Some("planning"), None, None, root).unwrap();
    let snapshot = snapshot_context::prepare(&context, Default::default(), &[]).unwrap();
    let guard = snapshot.install();
    (temp, guard)
}
fn request() -> ChangeSet {
    ChangeSet { version: 1, operations: vec![Operation::Create {
        path: "increment.md".into(),
        content: "---\ntype: Increment\ncheckpoint:\n  specifications:\n    - ../product/spec.md\n---\n".into(),
    }] }
}
fn scope() -> Vec<String> {
    vec!["product.main".into()]
}

#[test]
fn scoped_digest_matches_plan_and_guarded_apply_preserves_target() {
    let (temp, _context) = fixture();
    let root = temp.path().join("planning");
    let before = fs::read(temp.path().join("product/spec.md")).unwrap();
    let (digest, examined) =
        changeset::digest_read_only_scoped_with_evidence(&root, &scope(), false).unwrap();
    assert_eq!(examined, vec!["planning.main", "product.main"]);
    let gate = transaction::strengthen_ontology_gate();
    let plan = changeset::prepare_scoped(&root, &request(), &scope(), false).unwrap();
    assert_eq!(digest, plan.base_digest);
    assert_eq!(examined, plan.examined_bundles());
    drop(gate); // A strengthened plan retains its policy requirement at publication.
    changeset::apply(&root, plan, Some(&digest)).unwrap();
    assert!(root.join("increment.md").is_file());
    assert_eq!(
        fs::read(temp.path().join("product/spec.md")).unwrap(),
        before
    );
}

#[test]
fn scoped_apply_rejects_changed_target_and_catalog_inputs() {
    for changed in [
        "product/spec.md",
        "product/ontology.yaml",
        "okf-catalog.yaml",
    ] {
        let (temp, _context) = fixture();
        let root = temp.path().join("planning");
        let _gate = transaction::strengthen_ontology_gate();
        let plan = changeset::prepare_scoped(&root, &request(), &scope(), false).unwrap();
        let path = temp.path().join(changed);
        let mut content = fs::read_to_string(&path).unwrap();
        content.push_str("\n# changed\n");
        fs::write(path, content).unwrap();
        let error = changeset::apply(&root, plan, None).unwrap_err();
        assert!(
            error.to_string().contains("baseline-conflict"),
            "{changed}: {error}"
        );
        assert!(!root.join("increment.md").exists());
    }
}

#[test]
fn scoped_apply_rejects_removed_target_or_active_target_writer() {
    for remove in [false, true] {
        let (temp, _context) = fixture();
        let root = temp.path().join("planning");
        let _gate = transaction::strengthen_ontology_gate();
        let plan = changeset::prepare_scoped(&root, &request(), &scope(), false).unwrap();
        let _writer = if remove {
            fs::remove_file(temp.path().join("product/spec.md")).unwrap();
            None
        } else {
            Some(transaction::writer_lock(&temp.path().join("product")).unwrap())
        };
        let error = changeset::apply(&root, plan, None).unwrap_err();
        assert!(error.to_string().contains("baseline-conflict"), "{error}");
        assert!(!root.join("increment.md").exists());
    }
}
