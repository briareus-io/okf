use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::json;
use std::fs;

fn fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    for name in ["planning", "knowledge", "unregistered"] {
        fs::create_dir(dir.path().join(name)).unwrap();
    }
    fs::write(dir.path().join("okf.toml"), "catalog='catalog.yaml'\n").unwrap();
    fs::write(dir.path().join("catalog.yaml"), "catalog_version: 1\nbundles:\n  acme.planning: {location: {type: directory, path: planning}}\n  acme.knowledge: {location: {type: directory, path: knowledge}}\n").unwrap();
    fs::write(dir.path().join("planning/ontology.yaml"), "okf_ontology: '0.1'\nconcepts:\n  Task:\n    references:\n      concepts:\n        target: [Task, 'acme.knowledge:Component']\n        cardinality: 0..n\n").unwrap();
    fs::write(
        dir.path().join("knowledge/target.md"),
        "---\ntype: Component\n---\n# Target\n",
    )
    .unwrap();
    dir
}

fn task(reference: &str) -> String {
    format!("---\ntype: Task\nconcepts: ['{reference}']\n---\n[target]({reference})\n")
}

#[allow(
    clippy::needless_pass_by_value,
    reason = "test helper takes ownership of the operations"
)]
fn command(dir: &tempfile::TempDir, verb: &str, operations: serde_json::Value) -> Command {
    let mut command = Command::new(assert_cmd::cargo::cargo_bin!("okf"));
    command.current_dir(dir.path()).args([
        "changeset",
        verb,
        "--bundle-id",
        "acme.planning",
        "--json",
        "--from",
        &json!({"version": 1, "operations": operations}).to_string(),
    ]);
    command
}

#[test]
fn new_registered_sibling_reference_is_previewed_and_applied_without_sibling_writes() {
    let dir = fixture();
    let target = fs::read(dir.path().join("knowledge/target.md")).unwrap();
    let operations = json!([{"op":"create","path":"new","content":task("../knowledge/target.md")}]);
    command(&dir, "plan", operations.clone()).assert().success();
    assert!(!dir.path().join("planning/new.md").exists());
    command(&dir, "apply", operations).assert().success();
    assert!(
        fs::read_to_string(dir.path().join("planning/new.md"))
            .unwrap()
            .contains("../knowledge/target.md")
    );
    assert_eq!(
        fs::read(dir.path().join("knowledge/target.md")).unwrap(),
        target
    );
    assert_eq!(
        fs::read_dir(dir.path().join("knowledge")).unwrap().count(),
        1
    );
}

#[test]
fn move_rebases_registered_sibling_reference_without_aliasing_a_local_target() {
    let dir = fixture();
    fs::write(
        dir.path().join("planning/old.md"),
        task("../knowledge/target.md#section"),
    )
    .unwrap();
    fs::create_dir(dir.path().join("planning/knowledge")).unwrap();
    fs::write(
        dir.path().join("planning/knowledge/target.md"),
        "---\ntype: Task\n---\n# Local\n",
    )
    .unwrap();
    let operations = json!([{"op":"move","from":"old","to":"nested/new"}, {"op":"move","from":"knowledge/target","to":"local-renamed"}]);
    command(&dir, "apply", operations).assert().success();
    let moved = fs::read_to_string(dir.path().join("planning/nested/new.md")).unwrap();
    assert!(
        moved.contains("../../knowledge/target.md#section"),
        "{moved}"
    );
    assert!(!moved.contains("local-renamed"), "{moved}");
    assert!(!dir.path().join("planning/old.md").exists());
    assert!(dir.path().join("knowledge/target.md").exists());
}

#[test]
fn nonexistent_registered_target_is_rejected_without_publication() {
    let dir = fixture();
    command(
        &dir,
        "apply",
        json!([{"op":"create","path":"new","content":task("../knowledge/missing.md")}]),
    )
    .assert()
    .code(2)
    .stderr(predicate::str::contains("dangling/blocked"));
    assert!(!dir.path().join("planning/new.md").exists());
}

#[test]
fn unregistered_existing_target_and_standalone_escape_remain_blocked() {
    let dir = fixture();
    fs::write(
        dir.path().join("unregistered/target.md"),
        "---\ntype: Task\n---\n",
    )
    .unwrap();
    for reference in [
        "../unregistered/target.md",
        "../../outside.md",
        "C:/../../knowledge/target.md",
    ] {
        command(
            &dir,
            "apply",
            json!([{"op":"create","path":"new","content":task(reference)}]),
        )
        .assert()
        .code(2)
        .stderr(predicate::str::contains("dangling/blocked"));
    }
    fs::remove_file(dir.path().join("okf.toml")).unwrap();
    Command::new(assert_cmd::cargo::cargo_bin!("okf"))
        .current_dir(dir.path()).args(["changeset", "apply", "planning", "--from", &json!({"version":1,"operations":[{"op":"create","path":"new","content":task("../knowledge/target.md")}]}).to_string()])
        .assert().code(2).stderr(predicate::str::contains("dangling/blocked"));
    assert!(!dir.path().join("planning/new.md").exists());
}

#[cfg(unix)]
#[test]
fn registered_sibling_symlink_escape_remains_blocked() {
    let dir = fixture();
    fs::write(
        dir.path().join("unregistered/secret.md"),
        "---\ntype: Component\n---\n",
    )
    .unwrap();
    std::os::unix::fs::symlink(
        dir.path().join("unregistered/secret.md"),
        dir.path().join("knowledge/escape.md"),
    )
    .unwrap();
    command(
        &dir,
        "apply",
        json!([{"op":"create","path":"new","content":task("../knowledge/escape.md")}]),
    )
    .assert()
    .code(2)
    .stderr(predicate::str::contains("dangling/blocked"));
    assert!(!dir.path().join("planning/new.md").exists());
}

#[test]
fn registered_sibling_typed_file_source_is_checked_from_bundle_root() {
    let dir = fixture();
    fs::write(dir.path().join("knowledge/data.csv"), "a,b\n").unwrap();
    let content =
        "---\ntype: Task\nsources:\n- kind: file\n  resource: ../knowledge/data.csv\n---\n# New\n";
    command(
        &dir,
        "apply",
        json!([{"op":"create","path":"nested/new","content":content}]),
    )
    .assert()
    .success();
}

#[test]
fn sibling_registration_does_not_authorize_cross_bundle_writes() {
    let dir = fixture();
    let original = fs::read(dir.path().join("knowledge/target.md")).unwrap();
    for operation in [
        json!({"op":"remove", "path":"../knowledge/target.md"}),
        json!({"op":"create", "path":"../knowledge/new", "content":task("/new")}),
        json!({"op":"put-artifact", "path":"../knowledge/data.csv", "content":"changed"}),
    ] {
        command(
            &dir,
            "apply",
            json!([{"op":"create","path":"new","content":"---\ntype: Task\n---\n"}, operation]),
        )
        .assert()
        .code(2);
        assert!(!dir.path().join("planning/new.md").exists());
        assert_eq!(
            fs::read(dir.path().join("knowledge/target.md")).unwrap(),
            original
        );
        assert_eq!(
            fs::read_dir(dir.path().join("knowledge")).unwrap().count(),
            1
        );
    }
}

#[test]
fn declared_metadata_reference_is_checked_without_a_matching_body_link() {
    let dir = fixture();
    for (path, reference, succeeds) in [
        ("valid", "../knowledge/target.md", true),
        ("invalid", "../knowledge/missing.md", false),
    ] {
        let content = format!("---\ntype: Task\nconcepts: ['{reference}']\n---\n# Task\n");
        let assertion = command(
            &dir,
            "apply",
            json!([{"op":"create","path":path,"content":content}]),
        )
        .assert();
        if succeeds {
            assertion.success();
        } else {
            assertion
                .code(2)
                .stderr(predicate::str::contains("dangling/blocked"));
        }
        assert_eq!(
            dir.path().join(format!("planning/{path}.md")).exists(),
            succeeds
        );
    }
}

fn gated_fixture() -> tempfile::TempDir {
    let dir = fixture();
    fs::write(
        dir.path().join("knowledge/ontology.yaml"),
        "okf_ontology: '0.1'\nconcepts:\n  Component: {}\n  Other: {}\n",
    )
    .unwrap();
    dir
}

fn create_sibling_reference() -> serde_json::Value {
    json!([{"op":"create","path":"new","content":task("../knowledge/target.md")}])
}

fn digest_from(output: &[u8], kind: &str) -> String {
    record_from(output, kind)["base_digest"]
        .as_str()
        .unwrap()
        .to_owned()
}

fn record_from(output: &[u8], kind: &str) -> serde_json::Value {
    String::from_utf8_lossy(output)
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .find(|record| record["kind"] == kind)
        .unwrap()
}

#[test]
fn explicit_scope_allows_typed_gated_plan_and_apply_with_single_bundle_writes() {
    for scope in [
        vec!["--scope-bundle", "acme.knowledge"],
        vec!["--catalog-scope"],
    ] {
        let dir = gated_fixture();
        let original = fs::read(dir.path().join("knowledge/target.md")).unwrap();
        let plan = command(&dir, "plan", create_sibling_reference())
            .arg("--require-ontology-gate")
            .args(&scope)
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        assert!(!dir.path().join("planning/new.md").exists());
        let digest = digest_from(&plan, "changeset-summary");
        assert_eq!(
            record_from(&plan, "changeset-summary")["scope"],
            json!({"requested":["acme.planning", "acme.knowledge"],"examined":["acme.planning", "acme.knowledge"],"unavailable":[]})
        );
        command(&dir, "apply", create_sibling_reference())
            .args(["--require-ontology-gate", "--expect", &digest])
            .args(&scope)
            .assert()
            .success();
        assert!(dir.path().join("planning/new.md").exists());
        assert_eq!(
            fs::read(dir.path().join("knowledge/target.md")).unwrap(),
            original
        );
    }
}

#[test]
fn required_gate_keeps_out_of_scope_target_incomplete() {
    let dir = gated_fixture();
    for verb in ["plan", "apply"] {
        command(&dir, verb, create_sibling_reference())
            .arg("--require-ontology-gate")
            .assert()
            .code(1)
            .stderr(predicate::str::contains("write-gate-incomplete"));
        assert!(!dir.path().join("planning/new.md").exists());
    }
}

#[test]
fn scoped_gate_rejects_actual_wrong_type_and_unavailable_target_without_publication() {
    let dir = gated_fixture();
    fs::write(
        dir.path().join("knowledge/target.md"),
        "---\ntype: Other\n---\n",
    )
    .unwrap();
    command(&dir, "apply", create_sibling_reference())
        .args([
            "--require-ontology-gate",
            "--scope-bundle",
            "acme.knowledge",
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("write-gate-rejected"));
    assert!(!dir.path().join("planning/new.md").exists());
    fs::remove_dir_all(dir.path().join("knowledge")).unwrap();
    command(&dir, "apply", create_sibling_reference())
        .args([
            "--require-ontology-gate",
            "--scope-bundle",
            "acme.knowledge",
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("write-gate-incomplete"));
    assert!(!dir.path().join("planning/new.md").exists());
}

#[test]
fn scoped_digest_matches_plan_and_target_edits_invalidate_expected_baseline() {
    let dir = gated_fixture();
    let output = Command::new(assert_cmd::cargo::cargo_bin!("okf"))
        .current_dir(dir.path())
        .args([
            "changeset",
            "digest",
            "--bundle-id",
            "acme.planning",
            "--scope-bundle",
            "acme.knowledge",
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let digest = digest_from(&output, "baseline");
    let digest_scope = record_from(&output, "baseline")["scope"].clone();
    let output = command(&dir, "plan", create_sibling_reference())
        .args([
            "--require-ontology-gate",
            "--scope-bundle",
            "acme.knowledge",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    assert_eq!(digest_from(&output, "changeset-summary"), digest);
    assert_eq!(
        record_from(&output, "changeset-summary")["scope"],
        digest_scope
    );
    fs::write(
        dir.path().join("knowledge/target.md"),
        "---\ntype: Component\n---\nChanged\n",
    )
    .unwrap();
    command(&dir, "apply", create_sibling_reference())
        .args([
            "--require-ontology-gate",
            "--scope-bundle",
            "acme.knowledge",
            "--expect",
            &digest,
        ])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("baseline-conflict"));
    assert!(!dir.path().join("planning/new.md").exists());
}

#[test]
fn scoped_gate_cannot_authorize_cross_bundle_write_paths() {
    let dir = gated_fixture();
    let original = fs::read(dir.path().join("knowledge/target.md")).unwrap();
    command(
        &dir,
        "apply",
        json!([{"op":"remove","path":"../knowledge/target.md"}]),
    )
    .args([
        "--require-ontology-gate",
        "--scope-bundle",
        "acme.knowledge",
    ])
    .assert()
    .code(2);
    assert_eq!(
        fs::read(dir.path().join("knowledge/target.md")).unwrap(),
        original
    );
}

#[test]
fn scoped_gate_missing_target_is_rejected_without_publication() {
    let dir = gated_fixture();
    fs::remove_file(dir.path().join("knowledge/target.md")).unwrap();
    command(&dir, "apply", create_sibling_reference())
        .args([
            "--require-ontology-gate",
            "--scope-bundle",
            "acme.knowledge",
        ])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("dangling/blocked")
                .or(predicate::str::contains("reference-missing")),
        );
    assert!(!dir.path().join("planning/new.md").exists());
}

#[test]
fn manifest_enabled_gate_uses_explicit_scope_without_cli_strengthening() {
    let dir = gated_fixture();
    fs::write(
        dir.path().join("planning/okf-bundle.yaml"),
        "manifest_version: 1\nsettings:\n  write_policy:\n    ontology_gate: true\n",
    )
    .unwrap();
    let original = fs::read(dir.path().join("knowledge/target.md")).unwrap();
    command(&dir, "apply", create_sibling_reference())
        .args(["--scope-bundle", "acme.knowledge"])
        .assert()
        .success();
    assert!(dir.path().join("planning/new.md").exists());
    assert_eq!(
        fs::read(dir.path().join("knowledge/target.md")).unwrap(),
        original
    );
}

#[test]
fn qualified_target_requires_registered_bundle_identity_as_well_as_type() {
    let dir = gated_fixture();
    fs::write(
        dir.path().join("planning/local.md"),
        "---\ntype: Component\n---\n",
    )
    .unwrap();
    // Both concepts have Component type, but only the sibling belongs to acme.knowledge.
    command(
        &dir,
        "apply",
        json!([{"op":"create","path":"new","content":task("/local.md")}]),
    )
    .args([
        "--require-ontology-gate",
        "--scope-bundle",
        "acme.knowledge",
    ])
    .assert()
    .code(1)
    .stderr(predicate::str::contains("write-gate-rejected"));
    assert!(!dir.path().join("planning/new.md").exists());
}
