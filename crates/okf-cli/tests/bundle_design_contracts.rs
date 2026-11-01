use assert_cmd::prelude::*;
use serde_json::Value;
use std::{
    path::Path,
    process::{Command, Output},
};

fn write(root: &Path, path: &str, text: &str) {
    let path = root.join(path);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}
fn run(root: &Path, args: &[&str]) -> Output {
    Command::cargo_bin("okf")
        .unwrap()
        .current_dir(root)
        .env_remove("OKF_BUNDLE")
        .env_remove("OKF_CATALOG")
        .env_remove("OKF_CATALOG_OVERRIDES")
        .args(args)
        .output()
        .unwrap()
}
fn rows(out: &Output) -> Vec<Value> {
    assert!(
        out.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}
fn context_digest(rows: &[Value]) -> String {
    rows.iter()
        .find_map(|r| r.get("context_digest").and_then(Value::as_str))
        .expect("context digest")
        .into()
}
#[test]
fn instruction_file_changes_invalidate_context_and_inspection_does_not_activate() {
    let t = tempfile::tempdir().unwrap();
    write(t.path(), "okf-bundle.yaml", "manifest_version: 1\nguidance:\n  instructions_file: .okf-bundle/instructions.md\nworkflows:\n  inspect:\n    description: Inspect evidence\n    instructions: Read the selected concept.\n");
    write(t.path(), ".okf-bundle/instructions.md", "Preserve URLs.\n");
    let first = rows(&run(t.path(), &["bundle", "context", ".", "--json"]));
    write(
        t.path(),
        ".okf-bundle/instructions.md",
        "Preserve URLs and timestamps.\n",
    );
    let second = rows(&run(t.path(), &["bundle", "context", ".", "--json"]));
    assert_ne!(context_digest(&first), context_digest(&second));
    let inspected = rows(&run(
        t.path(),
        &["bundle", "context", ".", "--all", "--json"],
    ));
    assert!(inspected
        .iter()
        .filter(|r| r["kind"] == "guidance")
        .all(|r| r["instruction_role"] == "inspected"));
    assert!(inspected
        .iter()
        .any(|r| r["kind"] == "workflow" && r["selected"] == false && r["content"] == "full"));
}
#[test]
fn support_files_are_hidden_from_knowledge_but_core_validation_keeps_its_boundary() {
    let t = tempfile::tempdir().unwrap();
    write(t.path(), "okf-bundle.yaml", "manifest_version: 1\n");
    write(t.path(), "visible.md", "---\ntype: Note\n---\nVisible\n");
    write(
        t.path(),
        ".okf-bundle/template.md",
        "Unstructured template\n",
    );
    let listed = rows(&run(t.path(), &["list", ".", "--json"]));
    assert!(listed.iter().any(|r| r["id"] == "/visible"));
    assert!(!listed.iter().any(|r| r["id"]
        .as_str()
        .is_some_and(|id| id.contains(".okf-bundle"))));
    assert!(!run(t.path(), &["validate", ".", "--profile", "core"])
        .status
        .success());
    assert!(run(t.path(), &["validate", ".", "--profile", "bundle"])
        .status
        .success());
}
#[test]
fn yaml_catalog_selects_bundle_without_legacy_toml() {
    let t = tempfile::tempdir().unwrap();
    write(t.path(), "okf-catalog.yaml", "catalog_version: 2\ndefault_bundle: example.docs\nbundles:\n  example.docs:\n    location: {type: directory, path: docs}\n");
    write(t.path(), "docs/okf-bundle.yaml", "manifest_version: 1\n");
    write(t.path(), "docs/note.md", "---\ntype: Note\n---\n");
    let out = rows(&run(t.path(), &["list", "--json"]));
    assert!(out.iter().any(|r| r["id"] == "/note"));
}
#[test]
fn invalid_manifest_cannot_be_silently_ignored_but_core_diagnosis_remains_available() {
    let t = tempfile::tempdir().unwrap();
    write(t.path(), "okf-bundle.yaml", "manifest_version: 999\n");
    write(t.path(), "note.md", "---\ntype: Note\n---\n");
    assert!(!run(t.path(), &["list", "."]).status.success());
    assert!(!run(t.path(), &["bundle", "validate", "."]).status.success());
    assert!(run(t.path(), &["validate", ".", "--profile", "core"])
        .status
        .success());
}

#[test]
fn expansion_filters_before_edge_limit_without_filtering_primary_hits() {
    let t = tempfile::tempdir().unwrap();
    write(t.path(), "ontology.yaml", "okf_ontology: '0.1'\nconcepts:\n  Note:\n    references:\n      targets: {selector: 'relations[].target', target: Note, cardinality: 0..n}\n    relationships:\n      relations:\n        reference: targets\n        kind_selector: 'relations[].kind'\n        kinds:\n          cites: {inverse: cited-by}\n          blocks: {inverse: is-blocked-by}\n");
    write(t.path(), "a.md", "---\ntype: Note\nrole: anchor\nrelations:\n- {target: /b, kind: cites}\n- {target: /c, kind: blocks}\n---\n");
    write(t.path(), "b.md", "---\ntype: Note\n---\n");
    write(t.path(), "c.md", "---\ntype: Note\n---\n");
    let out = rows(&run(
        t.path(),
        &[
            "list",
            ".",
            "--field",
            "role=anchor",
            "--expand",
            "relations",
            "--expand-kind",
            "blocks",
            "--expansion-edges",
            "1",
            "--json",
        ],
    ));
    assert!(out.iter().any(|r| r["id"] == "/a"));
    let edges: Vec<_> = out.iter().filter(|r| r["kind"] == "relationship").collect();
    assert_eq!(edges.len(), 1, "{out:?}");
    assert_eq!(edges[0]["edge"]["authored_kind"], "blocks");
    assert_eq!(edges[0]["edge"]["target"], "/c");
}

#[test]
fn migration_plan_output_cannot_overwrite_managed_snapshot() {
    let t = tempfile::tempdir().unwrap();
    write(t.path(), "okf.toml", "bundle = 'docs'\n");
    write(t.path(), "docs/note.md", "---\ntype: Note\n---\n");
    write(t.path(), "cache/snapshot.yaml", "profile: 1\nurl: https://example.invalid/upstream.git\ncommit: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\nobject_format: sha1\nmembers: {}\n");
    write(t.path(), "cache/tree/report.yaml", "original");
    let out = run(
        t.path(),
        &[
            "doctor",
            "--migrate-config",
            "okf.toml",
            "--default-bundle-id",
            "example.docs",
            "--migration-plan",
            "cache/tree/report.yaml",
        ],
    );
    assert!(!out.status.success(), "{out:?}");
    assert_eq!(
        std::fs::read_to_string(t.path().join("cache/tree/report.yaml")).unwrap(),
        "original"
    );
    assert!(String::from_utf8_lossy(&out.stderr).contains("read-only"));
}

#[test]
fn context_and_execution_reject_ineffective_filter_options() {
    let t = tempfile::tempdir().unwrap();
    write(
        t.path(),
        "okf-bundle.yaml",
        "manifest_version: 1\nsettings:\n  views:\n    plain:\n      columns: [title]\n",
    );
    write(t.path(), "note.md", "---\ntype: Note\n---\n");
    let context = run(
        t.path(),
        &[
            "bundle",
            "context",
            ".",
            "--operation",
            "list",
            "--query",
            r#"{"clear_filter":["nonsense"]}"#,
            "--json",
        ],
    );
    assert!(!context.status.success());
    assert!(String::from_utf8_lossy(&context.stderr).contains("unknown filter category"));
    let expansion = run(
        t.path(),
        &[
            "list",
            ".",
            "--view",
            "plain",
            "--expand-kind",
            "blocks",
            "--json",
        ],
    );
    assert!(!expansion.status.success());
    assert!(String::from_utf8_lossy(&expansion.stderr).contains("effective expansion"));
}
