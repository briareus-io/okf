use okf_core::bundle::context::{resolve_context, Context};
use okf_core::fingerprint::{Engine, Fingerprinter};
use okf_core::graph::catalog::build_catalog_graph;
use okf_core::model::link::LogicalReference;
use okf_core::model::source::Source;
use okf_core::query::artifact::{list_artifacts, resolve_artifact_in_context, show_artifact};
use std::fs;

fn fixture() -> (tempfile::TempDir, Context) {
    let temp = tempfile::tempdir().unwrap();
    fs::create_dir(temp.path().join("a")).unwrap();
    fs::create_dir(temp.path().join("b")).unwrap();
    fs::write(temp.path().join("okf-catalog.yaml"), "catalog_version: 2\ndefault_bundle: acme.a\nbundles:\n  acme.a:\n    location: {type: directory, path: a}\n  Acme.B:\n    location: {type: directory, path: b}\n").unwrap();
    fs::write(temp.path().join("a/source.md"), "---\ntype: Note\nsources:\n- resource: okf://Acme.B/target.md\n  kind: bundle-resource\n---\n[Target](okf://Acme.B/target.md#heading)\n").unwrap();
    fs::write(
        temp.path().join("b/target.md"),
        "---\ntype: Note\n---\n# Target\n",
    )
    .unwrap();
    fs::write(temp.path().join("b/data.txt"), "data\n").unwrap();
    let context = resolve_context(None, None, None, temp.path()).unwrap();
    (temp, context)
}

#[test]
fn parses_once_and_preserves_identity_case() {
    let r = LogicalReference::parse("okf://Acme.B/a%20file.md#part")
        .unwrap()
        .unwrap();
    assert_eq!(r.bundle, "Acme.B");
    assert_eq!(r.path, "a file.md");
    assert_eq!(r.fragment.as_deref(), Some("part"));
    for raw in [
        "okf://acme.b/../x",
        "okf://acme.b/%2e%2e/x",
        "okf://acme.b/%252e%252e/x",
        "okf://acme.b/a%2fb",
        "okf://user@acme.b/a",
        "okf://acme.b:9/a",
        "okf://acme.b/a?q=1",
        "okf://acme.b//a",
        "okf://acme.b/a\\b",
    ] {
        assert!(LogicalReference::parse(raw).is_err(), "{raw}");
    }
    assert!(LogicalReference::parse("https://example.com/x")
        .unwrap()
        .is_none());
}

#[test]
fn graph_scope_does_not_fetch_or_invent_local_targets() {
    let (_temp, context) = fixture();
    let graph = build_catalog_graph(&context, &[], false, None).unwrap();
    assert_eq!(graph.edges.len(), 2);
    assert!(graph.edges.iter().all(|edge| edge.status == "out-of-scope"));
    let graph = build_catalog_graph(&context, &["Acme.B".into()], false, None).unwrap();
    assert!(graph.edges.iter().all(|edge| edge.status == "resolved"));
    assert_eq!(
        graph.target(&context, "okf://Acme.B/target.md").bundle,
        "Acme.B"
    );
    assert_eq!(
        graph.target(&context, "okf://Acme.B/target.md").id,
        "/target"
    );
}

#[test]
fn logical_artifacts_and_fingerprints_require_explicit_scope() {
    let (_temp, context) = fixture();
    assert!(resolve_artifact_in_context(&context, &[], None, "okf://Acme.B/data.txt").is_err());
    let scope = ["Acme.B".into()];
    let (root, resolved) =
        resolve_artifact_in_context(&context, &scope, None, "okf://Acme.B/data.txt").unwrap();
    assert!(resolved.exists);
    assert!(root.ends_with("b"));
    let value =
        serde_yaml::from_str("resource: okf://Acme.B/data.txt\nkind: bundle-resource\n").unwrap();
    let source = Source::from_value(&value).unwrap();
    let fs = okf_core::ports::fs::RealFs;
    let git = okf_core::ports::git::RealGit;
    assert!(Engine::new(&context.primary.root, &fs, &git, None)
        .fingerprint(&source)
        .is_err());
    let fp = Engine::new(&context.primary.root, &fs, &git, None)
        .with_bundle_context(&context, &scope)
        .fingerprint(&source)
        .unwrap();
    assert_eq!(fp.get("bundle"), Some("Acme.B"));
    assert_eq!(fp.get("path"), Some("data.txt"));
    assert!(fp.get("commit").is_none());
    assert!(fp.get("sha256").is_some());
}

#[test]
fn support_files_are_hidden_but_explicitly_readable() {
    let (temp, context) = fixture();
    let root = temp.path().join("a");
    fs::create_dir(root.join(".okf-bundle")).unwrap();
    fs::write(
        root.join(".okf-bundle/instructions.md"),
        "No concept metadata needed",
    )
    .unwrap();
    fs::write(root.join("okf-bundle.yaml"), "manifest_version: 1\n").unwrap();
    assert!(list_artifacts(&root, Some(".okf-bundle"), false)
        .unwrap()
        .is_empty());
    assert!(!list_artifacts(&root, None, false)
        .unwrap()
        .iter()
        .any(|e| e.path.starts_with(".okf-bundle") || e.path.to_str() == Some("okf-bundle.yaml")));
    let read = show_artifact(
        &context.primary.root,
        None,
        ".okf-bundle/instructions.md",
        100,
        None,
    )
    .unwrap();
    assert_eq!(read.resolved.kind.as_str(), "support");
}

#[test]
fn local_logical_links_guard_remove_and_are_rewritten_on_move() {
    let (temp, _context) = fixture();
    let root = temp.path().join("b");
    fs::write(
        root.join("referrer.md"),
        "---\ntype: Note\n---\n[Target](okf://Acme.B/target.md#heading)\n",
    )
    .unwrap();
    assert!(okf_core::mutate::rm::rm(&root, "target", false).is_err());
    okf_core::mutate::mv::mv(&root, "target", "renamed").unwrap();
    assert!(fs::read_to_string(root.join("referrer.md"))
        .unwrap()
        .contains("okf://Acme.B/renamed.md#heading"));
}

#[test]
fn uri_bundle_ref_conflicts_are_not_equivalence() {
    let (temp, context) = fixture();
    fs::write(temp.path().join("a/source.md"), "---\ntype: Note\nsources:\n- resource: okf://Acme.B/target.md\n  bundle_ref: {id: acme.a, path: target.md}\n---\n").unwrap();
    let graph = build_catalog_graph(&context, &["Acme.B".into()], false, None).unwrap();
    assert_eq!(graph.edges[0].status, "reference-mismatch");
}

#[test]
fn snapshot_constrained_logical_source_blocks_automatic_move() {
    let (temp, _context) = fixture();
    let root = temp.path().join("b");
    fs::write(root.join("referrer.md"), "---\ntype: Note\nsources:\n- resource: okf://Acme.B/target.md\n  kind: bundle-resource\n  bundle_ref:\n    id: Acme.B\n    path: target.md\n    snapshot: {label: accepted}\n---\n").unwrap();
    assert!(okf_core::mutate::mv::mv(&root, "target", "renamed").is_err());
    assert!(root.join("target.md").exists());
    assert!(!root.join("renamed.md").exists());
}

#[test]
fn semantic_logical_occurrences_share_resolved_endpoint_and_scope_status() {
    let (temp, context) = fixture();
    fs::write(temp.path().join("a/ontology.yaml"), "okf_ontology: '0.1'\nconcepts:\n  Note:\n    references:\n      target: {target: 'Acme.B:Note', cardinality: 0..1}\n    relationships:\n      blocks: {reference: target, kind: blocks, inverse: is-blocked-by}\n").unwrap();
    let raw = "okf://Acme.B/target.md";
    fs::write(
        temp.path().join("a/source.md"),
        format!("---\ntype: Note\ntarget: {raw}\n---\n"),
    )
    .unwrap();
    for (scope, expected_status) in [
        (Vec::new(), "out-of-scope"),
        (vec!["Acme.B".into()], "resolved"),
    ] {
        let graph = build_catalog_graph(&context, &scope, false, None).unwrap();
        let edge = graph
            .edges
            .iter()
            .find(|e| e.relationship.is_some())
            .unwrap();
        let relationship = edge.relationship.as_ref().unwrap();
        assert_eq!(edge.target.as_ref().unwrap().id, "/target");
        assert_eq!(relationship.target, "/target");
        assert_eq!(relationship.raw_reference, raw);
        assert_eq!(relationship.status, expected_status);
        assert_eq!(relationship.status, edge.status);
    }
}
