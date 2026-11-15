use okf_core::{
    bundle::loader::load_bundle,
    mutate::{
        gate::{self, Scope},
        transaction::FileChange,
    },
    ontology::load::try_load,
};
use std::{collections::BTreeMap, fs, path::Path};

fn fixture(
    target: &str,
    product_type: &str,
    product_ontology: Option<&str>,
) -> (tempfile::TempDir, Scope) {
    let temp = tempfile::tempdir().unwrap();
    let planning = temp.path().join("planning");
    let product = temp.path().join("product");
    fs::create_dir(&planning).unwrap();
    fs::create_dir(&product).unwrap();
    fs::write(planning.join("ontology.yaml"), format!("okf_ontology: '0.1'\nconcepts:\n  Increment:\n    references:\n      specifications:\n        selector: checkpoint.specifications[]\n        target: {target}\n        cardinality: 0..n\n")).unwrap();
    fs::write(
        product.join("spec.md"),
        format!("---\ntype: {product_type}\n---\n"),
    )
    .unwrap();
    if let Some(ontology) = product_ontology {
        fs::write(product.join("ontology.yaml"), ontology).unwrap();
    }
    let scope = Scope {
        primary_id: "planning.main".into(),
        targets: BTreeMap::from([(
            "product.main".into(),
            (
                load_bundle(&product.canonicalize().unwrap()).unwrap(),
                try_load(&product).unwrap(),
            ),
        )]),
    };
    (temp, scope)
}

fn check(root: &Path, scope: &Scope, reference: &str) -> okf_core::error::Result<()> {
    gate::check_changes_scoped(
        &root.join("planning"),
        &[FileChange {
            path: "increment.md".into(),
            before: None,
            after: Some(
                format!(
                    "---\ntype: Increment\ncheckpoint:\n  specifications:\n    - {reference}\n---\n"
                )
                .into_bytes(),
            ),
        }],
        true,
        scope,
    )
}

#[test]
fn scoped_gate_checks_actual_type_and_registered_owner() {
    let (temp, scope) = fixture("product.main:Specification", "Specification", None);
    check(temp.path(), &scope, "../product/spec.md").unwrap();
    check(temp.path(), &scope, "okf://product.main/spec.md").unwrap();
    let (temp, scope) = fixture("other.main:Specification", "Specification", None);
    assert!(
        check(temp.path(), &scope, "../product/spec.md")
            .unwrap_err()
            .to_string()
            .contains("write-gate-rejected")
    );
    let (temp, scope) = fixture(
        "product.main:Specification",
        "Other",
        Some("okf_ontology: '0.1'\nconcepts:\n  Other: {}\n"),
    );
    assert!(
        check(temp.path(), &scope, "../product/spec.md")
            .unwrap_err()
            .to_string()
            .contains("write-gate-rejected")
    );
}

#[test]
fn scoped_gate_uses_target_inheritance_and_distinguishes_unknown_hierarchy() {
    let ontology = "okf_ontology: '0.1'\nconcepts:\n  Specification: {}\n  Detailed:\n    extends: Specification\n";
    let (temp, scope) = fixture("product.main:Specification", "Detailed", Some(ontology));
    check(temp.path(), &scope, "../product/spec.md").unwrap();
    let (temp, scope) = fixture("Specification", "Detailed", Some(ontology));
    check(temp.path(), &scope, "../product/spec.md").unwrap();
    let (temp, scope) = fixture("product.main:Specification", "Detailed", None);
    assert!(
        check(temp.path(), &scope, "../product/spec.md")
            .unwrap_err()
            .to_string()
            .contains("write-gate-incomplete")
    );
    let (temp, scope) = fixture(
        "[product.main:Specification, product.main:Detailed]",
        "Detailed",
        None,
    );
    check(temp.path(), &scope, "../product/spec.md").unwrap();
}

#[test]
fn missing_concept_is_rejected_but_unexamined_bundle_is_incomplete() {
    let (temp, scope) = fixture("product.main:Specification", "Specification", None);
    assert!(
        check(temp.path(), &scope, "../product/missing.md")
            .unwrap_err()
            .to_string()
            .contains("write-gate-rejected")
    );
    assert!(
        check(temp.path(), &scope, "okf://other.main/spec.md")
            .unwrap_err()
            .to_string()
            .contains("write-gate-incomplete")
    );
    let empty = Scope {
        primary_id: scope.primary_id,
        targets: BTreeMap::new(),
    };
    assert!(
        check(temp.path(), &empty, "../product/spec.md")
            .unwrap_err()
            .to_string()
            .contains("write-gate-incomplete")
    );
}

#[test]
fn scoped_relationship_kind_narrows_the_cross_bundle_reference() {
    let ontology = "okf_ontology: '0.1'\nconcepts:\n  Specification: {}\n  Detailed:\n    extends: Specification\n";
    let (temp, scope) = fixture(
        "product.main:Specification",
        "Specification",
        Some(ontology),
    );
    fs::write(temp.path().join("planning/ontology.yaml"), "okf_ontology: '0.1'\nconcepts:\n  Increment:\n    references:\n      specifications: {target: ['product.main:Specification', 'product.main:Detailed'], cardinality: 0..n}\n    relationships:\n      obligation:\n        reference: specifications\n        kind_selector: kind\n        kinds:\n          implements: {inverse: implemented-by, target: 'product.main:Detailed'}\n").unwrap();
    let error = gate::check_changes_scoped(&temp.path().join("planning"), &[FileChange {
        path: "increment.md".into(), before: None,
        after: Some(b"---\ntype: Increment\nkind: implements\nspecifications: ../product/spec.md\n---\n".to_vec()),
    }], true, &scope).unwrap_err();
    assert!(error.to_string().contains("relationship-target"), "{error}");
}

#[test]
fn unchanged_cross_bundle_violation_is_grandfathered() {
    let (temp, scope) = fixture(
        "product.main:Specification",
        "Wrong",
        Some("okf_ontology: '0.1'\nconcepts:\n  Wrong: {}\n"),
    );
    let before = "---\ntype: Increment\ncheckpoint:\n  specifications:\n    - ../product/spec.md\n---\nOld body\n";
    fs::write(temp.path().join("planning/increment.md"), before).unwrap();
    gate::check_changes_scoped(
        &temp.path().join("planning"),
        &[FileChange {
            path: "increment.md".into(),
            before: Some(before.as_bytes().to_vec()),
            after: Some(before.replace("Old body", "New body").into_bytes()),
        }],
        true,
        &scope,
    )
    .unwrap();
}
