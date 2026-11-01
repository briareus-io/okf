use okf_core::{
    bundle::{
        catalog::load_catalog,
        context::{Context, SelectedBundle},
    },
    check::doctor::{doctor, doctor_in_context},
    mutate::changeset::{self, ChangeSet, Operation},
    query::artifact::{resolve_artifact_in_context, show_artifact_in_context, ArtifactKind},
};
use std::{fs, path::Path};

fn write(root: &Path, path: &str, text: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

fn catalog(root: &Path) -> Context {
    fs::create_dir_all(root.join("a")).unwrap();
    fs::create_dir_all(root.join("b")).unwrap();
    write(root, "okf-catalog.yaml", "catalog_version: 2\nbundles:\n  demo.a: {location: {type: directory, path: a}}\n  demo.b: {location: {type: directory, path: b}}\ndefault_bundle: demo.a\n");
    let catalog = load_catalog(&root.join("okf-catalog.yaml"), None).unwrap();
    Context {
        primary: SelectedBundle {
            id: Some("demo.a".into()),
            root: catalog.entries["demo.a"].root.clone(),
        },
        catalog: Some(catalog),
        config: Default::default(),
        config_path: None,
    }
}

#[test]
fn doctor_preserves_typed_source_namespaces_and_reports_missing_files() {
    let root = tempfile::tempdir().unwrap();
    write(root.path(), "refs/code.py", "print(1)\n");
    write(
        root.path(),
        "refs/note.md",
        "---\ntype: Note\n---\n# Evidence\nproof\n",
    );
    write(root.path(), "nested/concept.md", "---\ntype: Note\nsources:\n- {resource: refs/code.py, kind: file}\n- {resource: 'refs/code.py#L1-1', kind: line-range}\n- {resource: 'refs/note.md#evidence', kind: markdown-heading}\n---\n");
    let report = doctor(root.path(), "0.2", false, false).unwrap();
    assert!(report.ready(), "{:?}", report.findings);
    fs::remove_file(root.path().join("refs/code.py")).unwrap();
    let report = doctor(root.path(), "0.2", false, false).unwrap();
    assert_eq!(
        report
            .findings
            .iter()
            .filter(|f| f.rule == "artifact-unavailable")
            .count(),
        2
    );
}

#[test]
fn typed_file_sources_do_not_become_scope_descriptors_or_uri_schemes() {
    let root = tempfile::tempdir().unwrap();
    write(root.path(), "refs/file with spaces.py", "print(1)\n");
    write(root.path(), "data:part.csv", "1,2\n");
    write(root.path(), "nested/concept.md", "---\ntype: Note\nsources:\n- {resource: 'refs/file with spaces.py', kind: file}\n- {resource: 'data:part.csv', kind: file}\n---\n");
    assert!(doctor(root.path(), "0.2", false, false).unwrap().ready());
    fs::remove_file(root.path().join("refs/file with spaces.py")).unwrap();
    fs::remove_file(root.path().join("data:part.csv")).unwrap();
    let report = doctor(root.path(), "0.2", false, false).unwrap();
    assert_eq!(
        report
            .findings
            .iter()
            .filter(|f| f.rule == "artifact-unavailable")
            .count(),
        2
    );
}

#[test]
fn refreshed_file_source_has_consistent_doctor_stale_and_graph_resolution() {
    let root = tempfile::tempdir().unwrap();
    let context = catalog(root.path());
    write(
        root.path(),
        "a/refs/evidence.md",
        "---\ntype: Note\n---\nEvidence\n",
    );
    write(
        root.path(),
        "a/nested/concept.md",
        "---\ntype: Note\nsources:\n- {resource: refs/evidence.md, kind: file}\n---\n",
    );
    let engine = okf_core::fingerprint::Engine::new(
        &context.primary.root,
        &okf_core::ports::fs::RealFs,
        &okf_core::ports::git::RealGit,
        None,
    );
    okf_core::mutate::refresh::refresh(
        &context.primary.root,
        "/nested/concept",
        &engine,
        &okf_core::ports::clock::FixedClock("2026-11-01T00:00:00Z".into()),
    )
    .unwrap();
    assert!(doctor_in_context(&context, &[], "0.2", false, false)
        .unwrap()
        .ready());
    let bundle = okf_core::bundle::loader::load_bundle(&context.primary.root).unwrap();
    assert!(okf_core::check::stale::check_stale(&bundle)
        .concepts
        .is_empty());
    let graph = okf_core::graph::catalog::build_catalog_graph(&context, &[], false, None).unwrap();
    let edge = graph
        .edges
        .iter()
        .find(|e| e.location == "sources[0].resource")
        .unwrap();
    assert_eq!(edge.status, "resolved");
    assert_eq!(edge.fingerprint_status.as_deref(), Some("unchanged"));
}

#[test]
fn ordinary_cross_bundle_artifacts_require_scope_and_keep_target_root_for_reads() {
    let root = tempfile::tempdir().unwrap();
    let context = catalog(root.path());
    write(root.path(), "b/refs/code.py", "print(2)\n");
    let resource = "../../b/refs/code.py";
    assert!(
        resolve_artifact_in_context(&context, &[], Some("/notes/n"), resource)
            .unwrap_err()
            .to_string()
            .contains("out-of-scope")
    );
    let scope = vec!["demo.b".into()];
    let (target, resolved) =
        resolve_artifact_in_context(&context, &scope, Some("/notes/n"), resource).unwrap();
    assert_eq!(
        target,
        context.catalog.as_ref().unwrap().entries["demo.b"].root
    );
    assert_eq!(resolved.kind, ArtifactKind::Artifact);
    let content =
        show_artifact_in_context(&context, &scope, Some("/notes/n"), resource, 1024, None).unwrap();
    assert_eq!(content.text.as_deref(), Some("print(2)\n"));
    write(root.path(), "outside/code.py", "secret");
    assert!(resolve_artifact_in_context(
        &context,
        &scope,
        Some("/notes/n"),
        "../../outside/code.py"
    )
    .is_err());
    let (_, missing) = resolve_artifact_in_context(
        &context,
        &scope,
        Some("/notes/n"),
        "../../b/refs/missing.py",
    )
    .unwrap();
    assert_eq!(missing.kind, ArtifactKind::Missing);
}

#[test]
fn doctor_resolves_scoped_sources_and_enforces_bundle_mapping_and_snapshot() {
    let root = tempfile::tempdir().unwrap();
    let context = catalog(root.path());
    write(
        root.path(),
        "b/specs/x.md",
        "---\ntype: Note\n---\nEvidence\n",
    );
    let scope = vec!["demo.b".into()];
    for resource in ["../../b/specs/x.md", "okf://demo.b/specs/x.md"] {
        write(root.path(), "a/notes/n.md", &format!("---\ntype: Note\nsources:\n- resource: {resource}\n  bundle_ref: {{id: demo.b, path: specs/x.md}}\n---\n"));
        let report = doctor_in_context(&context, &[], "0.2", false, false).unwrap();
        assert!(!report.ready());
        let report = doctor_in_context(&context, &scope, "0.2", false, false).unwrap();
        assert!(report.ready(), "{:?}", report.findings);
    }
    write(root.path(), "a/notes/n.md", "---\ntype: Note\nsources:\n- resource: okf://demo.b/specs/x.md\n  bundle_ref: {id: demo.b, path: specs/other.md}\n---\n");
    let report = doctor_in_context(&context, &scope, "0.2", false, false).unwrap();
    assert!(report
        .findings
        .iter()
        .any(|f| f.message.contains("reference-mismatch")));
    write(root.path(), "a/notes/n.md", &format!("---\ntype: Note\nsources:\n- resource: okf://demo.b/specs/x.md\n  bundle_ref:\n    id: demo.b\n    path: specs/x.md\n    snapshot: {{digest: '{}'}}\n---\n", "0".repeat(64)));
    let report = doctor_in_context(&context, &scope, "0.2", false, false).unwrap();
    assert!(report
        .findings
        .iter()
        .any(|f| f.message.contains("snapshot mismatched")));
}

#[cfg(unix)]
#[test]
fn scoped_resolution_does_not_follow_escaping_symlinks() {
    let root = tempfile::tempdir().unwrap();
    let context = catalog(root.path());
    write(root.path(), "outside.txt", "secret");
    std::os::unix::fs::symlink(
        root.path().join("outside.txt"),
        root.path().join("b/secret.txt"),
    )
    .unwrap();
    let (_, resolved) = resolve_artifact_in_context(
        &context,
        &["demo.b".into()],
        Some("/notes/n"),
        "../../b/secret.txt",
    )
    .unwrap();
    assert_eq!(resolved.kind, ArtifactKind::Blocked);
    write(root.path(), "b/specs/x.md", "---\ntype: Note\n---\n");
    std::os::unix::fs::symlink(
        root.path().join("outside.txt"),
        root.path().join("a/escape.txt"),
    )
    .unwrap();
    write(root.path(), "a/note.md", "---\ntype: Note\nsources:\n- resource: escape.txt\n  bundle_ref: {id: demo.b, path: specs/x.md}\n---\n");
    let report = doctor_in_context(&context, &["demo.b".into()], "0.2", false, false).unwrap();
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.message.contains("escapes bundle")),
        "{:?}",
        report.findings
    );
}

#[test]
fn changeset_concept_id_is_not_shadowed_by_same_named_directory() {
    let root = tempfile::tempdir().unwrap();
    write(root.path(), "m/M1.md", "---\ntype: Note\n---\n");
    write(root.path(), "m/M1/child.md", "---\ntype: Note\n---\n");
    let changes = ChangeSet {
        version: 1,
        operations: vec![Operation::Create {
            path: "n.md".into(),
            content: "---\ntype: Note\n---\n[Milestone](/m/M1)\n".into(),
        }],
    };
    changeset::prepare(root.path(), &changes).unwrap();
    fs::remove_file(root.path().join("m/M1.md")).unwrap();
    assert!(changeset::prepare(root.path(), &changes)
        .err()
        .unwrap()
        .to_string()
        .contains("dangling/blocked"));
}
