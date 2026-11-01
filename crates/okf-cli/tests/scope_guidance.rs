use assert_cmd::Command;
use serde_json::Value;
use tempfile::TempDir;
fn fixture() -> TempDir {
    let d = tempfile::tempdir().unwrap();
    for name in ["a", "b"] {
        std::fs::create_dir(d.path().join(name)).unwrap();
        std::fs::write(
            d.path().join(name).join("note.md"),
            format!("---\ntype: Note\n---\n# {name}\n"),
        )
        .unwrap();
        std::fs::write(d.path().join(name).join("okf-bundle.yaml"),format!("manifest_version: 1\nguidance:\n  instructions: Guidance for {name}\n  rules:\n    - id: read-rule\n      operations: [list]\n      instructions: List rule for {name}\n")).unwrap();
    }
    std::fs::write(d.path().join("okf-catalog.yaml"),"catalog_version: 2\ndefault_bundle: test.a\nbundles:\n  test.a:\n    location: {type: directory, path: a}\n  test.b:\n    location: {type: directory, path: b}\n  test.missing:\n    location: {type: directory, path: missing}\n").unwrap();
    d
}
fn rows(output: &[u8]) -> Vec<Value> {
    String::from_utf8_lossy(output)
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect()
}
#[test]
fn scoped_list_delivers_each_bundles_guidance_with_origin() {
    let d = fixture();
    let out = Command::cargo_bin("okf")
        .unwrap()
        .current_dir(d.path())
        .args(["list", "--scope-bundle", "test.b", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let r = rows(&out);
    let guidance = r
        .iter()
        .filter(|r| r["kind"] == "guidance")
        .collect::<Vec<_>>();
    assert!(guidance
        .iter()
        .any(|r| r["bundle"] == "test.a" && r["id"] == "read-rule"));
    assert!(guidance
        .iter()
        .any(|r| r["bundle"] == "test.b" && r["id"] == "read-rule"));
    assert!(guidance.iter().all(|r| r["origin"]["root"].is_string()));
    assert!(!guidance.iter().any(|r| r["bundle"] == "test.missing"));
}
#[test]
fn explicit_context_scope_returns_both_bundles() {
    let d = fixture();
    let out = Command::cargo_bin("okf")
        .unwrap()
        .current_dir(d.path())
        .args([
            "bundle",
            "context",
            "--scope-bundle",
            "test.b",
            "--operation",
            "list",
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let r = rows(&out);
    let contexts = r
        .iter()
        .filter(|r| r["kind"] == "bundle-context")
        .collect::<Vec<_>>();
    assert_eq!(contexts.len(), 2);
    assert_eq!(contexts[0]["bundle"], "test.a");
    assert_eq!(contexts[1]["bundle"], "test.b");
    assert_eq!(contexts[0]["primary"], true);
    assert_eq!(contexts[1]["primary"], false);
}
#[test]
fn unavailable_context_scope_is_explicit_partial_and_nonzero() {
    let d = fixture();
    let out = Command::cargo_bin("okf")
        .unwrap()
        .current_dir(d.path())
        .args([
            "bundle",
            "context",
            "--scope-bundle",
            "test.missing",
            "--json",
        ])
        .assert()
        .code(1)
        .get_output()
        .stdout
        .clone();
    let r = rows(&out);
    let scope = r.iter().find(|r| r["kind"] == "context-scope").unwrap();
    assert_eq!(scope["complete"], false);
    assert_eq!(scope["unavailable"], serde_json::json!(["test.missing"]));
    assert_eq!(r.last().unwrap()["completeness"]["examination"], "partial");
}
#[test]
fn unknown_scope_does_not_silently_return_primary_context() {
    let d = fixture();
    Command::cargo_bin("okf")
        .unwrap()
        .current_dir(d.path())
        .args([
            "bundle",
            "context",
            "--scope-bundle",
            "test.unknown",
            "--json",
        ])
        .assert()
        .code(2);
}
#[test]
fn scope_inspection_never_activates_secondary_workflows() {
    let d = fixture();
    let out = Command::cargo_bin("okf")
        .unwrap()
        .current_dir(d.path())
        .args([
            "bundle",
            "context",
            "--scope-bundle",
            "test.b",
            "--all",
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let r = rows(&out);
    assert!(r
        .iter()
        .filter(|r| r["kind"] == "guidance")
        .all(|r| r["instruction_role"] == "inspected"));
}
