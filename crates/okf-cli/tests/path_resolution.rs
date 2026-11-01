use assert_cmd::prelude::*;
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

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
fn rows(out: &Output) -> Vec<serde_json::Value> {
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect()
}

#[test]
fn doctor_catalog_scope_reaches_source_aware_handler() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fs::create_dir_all(root.join("a/notes")).unwrap();
    fs::create_dir_all(root.join("b/specs")).unwrap();
    fs::write(root.join("okf-catalog.yaml"), "catalog_version: 2\nbundles:\n  demo.a: {location: {type: directory, path: a}}\n  demo.b: {location: {type: directory, path: b}}\ndefault_bundle: demo.a\n").unwrap();
    fs::write(
        root.join("a/notes/n.md"),
        "---\ntype: Note\nsources:\n- {resource: ../../b/specs/x.md}\n---\n",
    )
    .unwrap();
    fs::write(root.join("b/specs/x.md"), "---\ntype: Note\n---\n").unwrap();
    fs::write(root.join("b/specs/check.py"), "print(1)\n").unwrap();
    let unscoped = run(root, &["doctor", "--bundle-id", "demo.a", "--json"]);
    assert_eq!(
        unscoped.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&unscoped.stderr)
    );
    assert!(rows(&unscoped).iter().any(|r| r["kind"] == "doctor-finding"
        && r["message"].as_str().unwrap().contains("out-of-scope")));
    for scope in [vec!["--scope-bundle", "demo.b"], vec!["--catalog-scope"]] {
        let mut args = vec!["doctor", "--bundle-id", "demo.a", "--json"];
        args.extend(scope);
        let scoped = run(root, &args);
        assert!(
            scoped.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&scoped.stdout),
            String::from_utf8_lossy(&scoped.stderr)
        );
        assert!(rows(&scoped)
            .iter()
            .any(|r| r["kind"] == "doctor-summary" && r["ready"] == true));
        assert!(rows(&scoped)
            .iter()
            .any(|r| r["kind"] == "operation-complete" && r["outcome"] == "ok"));
    }
    let artifact = run(
        root,
        &[
            "artifact",
            "show",
            "../../b/specs/check.py",
            "--from",
            "/notes/n",
            "--bundle-id",
            "demo.a",
            "--scope-bundle",
            "demo.b",
        ],
    );
    assert!(
        artifact.status.success(),
        "{}",
        String::from_utf8_lossy(&artifact.stderr)
    );
}

#[test]
fn doctor_relative_link_normalization_is_explicit_preview_then_apply() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fs::create_dir(root.join("notes")).unwrap();
    let original = "---\ntype: Note\nverified:\n- {by: 'human:reviewer', at: '2026-11-01T00:00:00Z'}\n---\nSee [target](b.md#heading).\n";
    fs::write(root.join("notes/a.md"), original).unwrap();
    fs::write(root.join("notes/b.md"), "---\ntype: Note\n---\n# Heading\n").unwrap();
    let preview = run(root, &["doctor", "--normalize-relative-links", "--json"]);
    assert_eq!(
        preview.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&preview.stderr)
    );
    let preview_rows = rows(&preview);
    let mapping = preview_rows
        .iter()
        .find(|r| r["kind"] == "doctor-normalization")
        .unwrap();
    assert_eq!(mapping["applied"], false);
    assert!(mapping["diff"]
        .as_str()
        .unwrap()
        .contains("/notes/b.md#heading"));
    assert_eq!(
        fs::read_to_string(root.join("notes/a.md")).unwrap(),
        original
    );
    let applied = run(
        root,
        &["doctor", "--normalize-relative-links", "--yes", "--json"],
    );
    assert!(
        applied.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&applied.stdout),
        String::from_utf8_lossy(&applied.stderr)
    );
    let text = fs::read_to_string(root.join("notes/a.md")).unwrap();
    assert!(text.contains("human:reviewer"));
    assert!(text.contains("[target](/notes/b.md#heading)"));
    assert!(rows(&applied)
        .iter()
        .any(|r| r["kind"] == "doctor-normalization" && r["applied"] == true));
}
