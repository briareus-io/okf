use okf_core::{
    bundle::loader::load_bundle,
    mutate::{gate, transaction::FileChange},
    ontology::load::parse_ontology,
};
use std::{collections::BTreeMap, fs};

fn check_reference(raw: &str, files: &[(&str, &str)]) -> okf_core::error::Result<()> {
    let workspace = tempfile::tempdir().unwrap();
    let primary = workspace.path().join("planning");
    let target = workspace.path().join("product");
    fs::create_dir(&primary).unwrap();
    fs::create_dir(&target).unwrap();
    fs::write(
        primary.join("ontology.yaml"),
        "okf_ontology: '0.1'\nconcepts:\n  Increment:\n    references:\n      specification: {target: 'product.main:Specification', cardinality: '1'}\n",
    )
    .unwrap();
    for (name, text) in files {
        fs::write(target.join(name), text).unwrap();
    }
    let target_ontology =
        parse_ontology("okf_ontology: '0.1'\nconcepts:\n  Specification: {}\n  Other: {}\n")
            .unwrap();
    let scope = gate::Scope {
        primary_id: "planning.main".into(),
        targets: BTreeMap::from([(
            "product.main".into(),
            (load_bundle(&target).unwrap(), Some(target_ontology)),
        )]),
    };
    gate::check_changes_scoped(
        &primary,
        &[FileChange {
            path: "increment.md".into(),
            before: None,
            after: Some(format!("---\ntype: Increment\nspecification: {raw}\n---\n").into_bytes()),
        }],
        true,
        &scope,
    )
}

#[test]
fn ordinary_artifact_does_not_alias_a_different_concept_file() {
    let result = check_reference(
        "../product/spec.txt",
        &[
            ("spec.txt", "untyped artifact"),
            ("spec.txt.md", "---\ntype: Specification\n---\n"),
        ],
    );
    assert!(
        result.is_err(),
        "an artifact must not borrow another file's type"
    );
}

#[test]
fn logical_reference_strips_only_one_markdown_suffix() {
    let result = check_reference(
        "okf://product.main/spec.md.md",
        &[
            ("spec.md", "---\ntype: Specification\n---\n"),
            ("spec.md.md", "---\ntype: Other\n---\n"),
        ],
    );
    assert!(result.is_err(), "the named target has type Other");
}

#[test]
fn unexamined_logical_target_cannot_bypass_the_required_gate() {
    let primary = tempfile::tempdir().unwrap();
    fs::write(
        primary.path().join("ontology.yaml"),
        "okf_ontology: '0.1'\nconcepts:\n  Increment:\n    references:\n      specification: {target: 'product.main:Specification', cardinality: '1'}\n",
    )
    .unwrap();
    let result = gate::check_changes(
        primary.path(),
        &[FileChange {
            path: "increment.md".into(),
            before: None,
            after: Some(
                b"---\ntype: Increment\nspecification: okf://product.main/spec.md\n---\n".to_vec(),
            ),
        }],
        true,
    );
    assert!(
        result.is_err(),
        "a logical OKF reference requires examination, unlike an external URL"
    );
}
