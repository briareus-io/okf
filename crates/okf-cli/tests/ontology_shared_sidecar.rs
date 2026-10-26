use assert_cmd::Command;
use std::fs;

fn setup() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    for name in ["a", "b"] {
        fs::create_dir(dir.path().join(name)).unwrap();
    }
    fs::write(dir.path().join("okf.toml"), "catalog = 'catalog.yaml'\n[bundle_settings.\"acme.a\"]\nontology = 'shared.yaml'\n[bundle_settings.\"acme.b\"]\nontology = 'shared.yaml'\n[bundle_settings.\"acme.b\".write_policy]\nontology_gate = true\n").unwrap();
    fs::write(dir.path().join("catalog.yaml"), "catalog_version: 1\nbundles:\n  acme.a: {location: {type: directory, path: a}}\n  acme.b: {location: {type: directory, path: b}}\n").unwrap();
    fs::write(
        dir.path().join("shared.yaml"),
        "okf_ontology: '0.1'\nconcepts:\n  Note: {}\n",
    )
    .unwrap();
    fs::write(dir.path().join("b/note.md"), "---\ntype: Note\n---\n").unwrap();
    dir
}

fn run(dir: &tempfile::TempDir, args: &[&str]) -> std::process::Output {
    Command::new(assert_cmd::cargo::cargo_bin!("okf"))
        .current_dir(dir.path().join("a"))
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn shared_sidecar_requires_explicit_scope_and_journals_single_target() {
    let dir = setup();
    let old = fs::read(dir.path().join("shared.yaml")).unwrap();
    let refused = run(
        &dir,
        &["ontology", "update", "Note", "--description", "Changed"],
    );
    assert_eq!(refused.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&refused.stderr).contains("--check-dependent-bundles"));
    assert_eq!(fs::read(dir.path().join("shared.yaml")).unwrap(), old);
    let applied = run(
        &dir,
        &[
            "ontology",
            "update",
            "Note",
            "--description",
            "Changed",
            "--check-dependent-bundles",
            "--json",
        ],
    );
    assert!(
        applied.status.success(),
        "{}",
        String::from_utf8_lossy(&applied.stderr)
    );
    let record = String::from_utf8_lossy(&applied.stdout)
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .find(|row| row["kind"] == "change")
        .unwrap();
    assert_eq!(
        record["required_dependent_bundles"],
        serde_json::json!(["acme.b"])
    );
    assert_eq!(record["examined_bundles"].as_array().unwrap().len(), 2);
    assert!(fs::read_to_string(dir.path().join("shared.yaml"))
        .unwrap()
        .contains("Changed"));
    assert!(!dir.path().join("a/.okf-transaction").exists());
    assert_eq!(
        fs::read_to_string(dir.path().join("b/note.md")).unwrap(),
        "---\ntype: Note\n---\n"
    );
}

#[test]
fn dependent_gate_blocks_new_requirement_before_sidecar_publication() {
    let dir = setup();
    let before = fs::read(dir.path().join("shared.yaml")).unwrap();
    let output = run(
        &dir,
        &[
            "ontology",
            "update",
            "Note",
            "--from",
            "{requires: [title]}",
            "--check-dependent-bundles",
        ],
    );
    assert_eq!(
        output.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read(dir.path().join("shared.yaml")).unwrap(), before);
}

#[test]
fn unavailable_dependent_is_reported_without_reading_consumer_content() {
    let dir = setup();
    fs::remove_dir_all(dir.path().join("b")).unwrap();
    let no_scope = run(
        &dir,
        &["ontology", "update", "Note", "--description", "Changed"],
    );
    assert_eq!(no_scope.status.code(), Some(2));
    let text = String::from_utf8_lossy(&no_scope.stderr);
    assert!(text.contains("required=acme.b"), "{text}");
    assert!(text.contains("unavailable=acme.b"), "{text}");
    let explicit = run(
        &dir,
        &[
            "ontology",
            "update",
            "Note",
            "--description",
            "Changed",
            "--check-dependent-bundles",
        ],
    );
    assert_eq!(explicit.status.code(), Some(3));
    assert!(String::from_utf8_lossy(&explicit.stderr).contains("scope incomplete"));
}
