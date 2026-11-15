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

fn logical_request(reference: &str) -> ChangeSet {
    let mut set = request();
    if let Operation::Create { content, .. } = &mut set.operations[0] {
        *content = content.replace("../product/spec.md", reference);
    }
    set
}

#[test]
fn scoped_logical_concept_references_plan_and_publish() {
    for (reference, catalog_scope) in [
        ("okf://product.main/spec", false),
        ("okf://product.main/spec.md", true),
    ] {
        let (temp, _context) = fixture();
        let root = temp.path().join("planning");
        let ids = if catalog_scope { vec![] } else { scope() };
        let digest = changeset::digest_read_only_scoped(&root, &ids, catalog_scope).unwrap();
        let _gate = transaction::strengthen_ontology_gate();
        let plan =
            changeset::prepare_scoped(&root, &logical_request(reference), &ids, catalog_scope)
                .unwrap();
        assert_eq!(plan.base_digest, digest);
        changeset::apply(&root, plan, Some(&digest)).unwrap();
        assert!(root.join("increment.md").is_file());
    }
}

#[test]
fn scoped_logical_references_refuse_missing_unexamined_wrong_type_and_drift() {
    for case in ["missing", "unexamined", "wrong-type", "drift"] {
        let (temp, _context) = fixture();
        let root = temp.path().join("planning");
        let target = temp.path().join("product/spec.md");
        if case == "missing" {
            fs::remove_file(&target).unwrap();
        }
        if case == "wrong-type" {
            fs::write(&target, "---\ntype: Other\n---\n").unwrap();
        }
        let _gate = transaction::strengthen_ontology_gate();
        let ids = if case == "unexamined" {
            vec!["planning.main".into()]
        } else {
            scope()
        };
        let result = changeset::prepare_scoped(
            &root,
            &logical_request("okf://product.main/spec.md"),
            &ids,
            false,
        );
        if case == "drift" {
            let plan = result.unwrap();
            fs::write(&target, "---\ntype: Specification\n---\nChanged\n").unwrap();
            assert!(
                changeset::apply(&root, plan, None)
                    .unwrap_err()
                    .to_string()
                    .contains("baseline-conflict")
            );
        } else {
            let message = result.err().expect(case).to_string();
            if case == "unexamined" {
                assert!(message.contains("out-of-scope bundle"), "{message}");
            }
            if case == "missing" {
                assert!(message.contains("missing or blocked target"), "{message}");
            }
            if case == "wrong-type" {
                assert!(message.contains("write-gate-rejected"), "{message}");
            }
        }
        assert!(!root.join("increment.md").exists(), "{case}");
    }
}

#[test]
fn logical_primary_reference_uses_prospective_state() {
    let (temp, _context) = fixture();
    let root = temp.path().join("planning");
    let set = ChangeSet {
        version: 1,
        operations: vec![
            Operation::Create {
                path: "target.md".into(),
                content: "---\ntype: Increment\n---\n".into(),
            },
            Operation::Create {
                path: "source.md".into(),
                content: "---\ntype: Increment\n---\n[Target](okf://planning.main/target.md)\n"
                    .into(),
            },
        ],
    };
    let plan = changeset::prepare_scoped(&root, &set, &scope(), false).unwrap();
    changeset::apply(&root, plan, None).unwrap();
    assert!(root.join("source.md").is_file());
}

#[test]
fn logical_artifacts_are_literal_and_unsafe_paths_are_rejected() {
    for (reference, allowed) in [
        ("okf://product.main/data", true),
        ("okf://product.main/data.bin", true),
        ("okf://product.main/.okf-bundle/support.md", true),
        ("okf://product.main/data.md", false),
        ("okf://product.main/%2e%2e/spec.md", false),
    ] {
        let (temp, _context) = fixture();
        fs::write(temp.path().join("product/data"), "artifact").unwrap();
        fs::write(temp.path().join("product/data.bin"), [0, 255]).unwrap();
        fs::create_dir(temp.path().join("product/.okf-bundle")).unwrap();
        fs::write(
            temp.path().join("product/.okf-bundle/support.md"),
            "support",
        )
        .unwrap();
        let root = temp.path().join("planning");
        let set = ChangeSet {
            version: 1,
            operations: vec![Operation::Create {
                path: "source.md".into(),
                content: format!("---\ntype: Increment\n---\n[Data]({reference})\n"),
            }],
        };
        let result = changeset::prepare_scoped(&root, &set, &scope(), false);
        assert_eq!(result.is_ok(), allowed, "{reference}");
        assert!(!root.join("source.md").exists());
    }
}

#[test]
fn existing_wrong_typed_logical_reference_does_not_block_unrelated_create() {
    let (temp, _context) = fixture();
    let root = temp.path().join("planning");
    fs::write(
        temp.path().join("product/spec.md"),
        "---\ntype: Other\n---\n",
    )
    .unwrap();
    let Operation::Create { content, .. } = logical_request("okf://product.main/spec.md")
        .operations
        .remove(0)
    else {
        unreachable!()
    };
    fs::write(root.join("existing.md"), content).unwrap();
    let set = ChangeSet {
        version: 1,
        operations: vec![Operation::Create {
            path: "unrelated.md".into(),
            content: "---\ntype: Increment\n---\n".into(),
        }],
    };
    let _gate = transaction::strengthen_ontology_gate();
    let plan = changeset::prepare_scoped(&root, &set, &scope(), false).unwrap();
    changeset::apply(&root, plan, None).unwrap();
    assert!(root.join("unrelated.md").is_file());
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
