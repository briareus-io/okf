use serde_json::Value;
use std::{path::Path, process::Command};

fn run(root: &Path, args: &[&str]) -> Vec<Value> {
    let output = Command::new(env!("CARGO_BIN_EXE_okf"))
        .current_dir(root)
        .env_remove("OKF_BUNDLE")
        .env_remove("OKF_CATALOG")
        .env_remove("OKF_CATALOG_OVERRIDES")
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

#[test]
fn schema_advertises_profile_and_migration_payloads_and_effects() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::create_dir(temp.path().join("knowledge")).unwrap();
    std::fs::write(temp.path().join("okf.toml"), "bundle = 'knowledge'\n").unwrap();
    let schema = run(temp.path(), &["schema"]);
    let command = |name: &str| {
        schema
            .iter()
            .find(|row| row["kind"] == "command" && row["name"] == name)
            .unwrap()
    };
    let families = |name: &str| {
        command(name)["stream_contract"]["payload_families"]
            .as_array()
            .unwrap()
            .clone()
    };
    for kind in ["violation", "validation-profile"] {
        assert!(families("validate").contains(&Value::from(kind)));
    }
    for kind in [
        "doctor-finding",
        "doctor-summary",
        "migration-plan",
        "migration-applied",
    ] {
        assert!(families("doctor").contains(&Value::from(kind)));
    }
    let validation = run(
        temp.path(),
        &["validate", "knowledge", "--profile", "bundle", "--json"],
    );
    assert!(validation
        .iter()
        .any(|row| row["kind"] == "validation-profile"));
    let plan = run(
        temp.path(),
        &[
            "doctor",
            "--migrate-config",
            "okf.toml",
            "--default-bundle-id",
            "test.docs",
            "--migration-plan",
            "plan.yaml",
            "--json",
        ],
    );
    assert!(plan.iter().any(|row| row["kind"] == "migration-plan"));
    assert_eq!(
        command("doctor")["effects"]["external_output_file_writes"]["when"]["migration-plan"],
        "present"
    );
    assert_eq!(
        command("catalog update")["effects"]["network_access"]["when"]["apply"],
        "absent"
    );
    assert_eq!(
        command("catalog sync")["effects"]["network_access"]["when"]["check"],
        false
    );
}

#[test]
fn catalog_lint_does_not_invent_concept_edges_for_repository_markdown_sources() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    assert!(Command::new("git")
        .args(["init", "-q"])
        .current_dir(root)
        .status()
        .unwrap()
        .success());
    std::fs::create_dir(root.join("knowledge")).unwrap();
    std::fs::create_dir(root.join("source-docs")).unwrap();
    std::fs::write(
        root.join("source-docs/reference.md"),
        "Repository documentation, not an OKF concept.\n",
    )
    .unwrap();
    std::fs::write(root.join("okf-catalog.yaml"), "catalog_version: 2\ndefault_bundle: test.docs\nbundles:\n  test.docs:\n    location: {type: directory, path: knowledge}\n").unwrap();
    std::fs::write(root.join("knowledge/note.md"), "---\ntype: Note\nsources:\n- {resource: source-docs/reference.md, kind: git-path}\n- {resource: source-docs/reference.md, kind: git-commit}\n---\nRepository-derived note.\n").unwrap();
    let lint = run(root, &["lint", "knowledge", "--fail-on", "never", "--json"]);
    assert!(
        !lint
            .iter()
            .any(|row| row["rule"] == "cross-bundle-reference"),
        "{lint:?}"
    );
    let links = run(root, &["links", "note", "knowledge", "--json"]);
    assert!(
        !links.iter().any(|row| row["kind"] == "bundle-edge"),
        "{links:?}"
    );
    // Resource impact still tracks repository sources even though they are not
    // authored concept-graph edges.
    let affected = run(
        root,
        &[
            "affected",
            "knowledge",
            "--changed",
            "source-docs/reference.md",
            "--changed-base",
            "repo",
            "--json",
        ],
    );
    assert!(
        affected.iter().any(|row| row.to_string().contains("/note")),
        "{affected:?}"
    );
}

#[test]
fn catalog_lint_accepts_reserved_and_artifact_links_without_creating_concepts() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir(root.join("knowledge")).unwrap();
    std::fs::write(root.join("okf-catalog.yaml"), "catalog_version: 2\ndefault_bundle: test.docs\nbundles:\n  test.docs:\n    location: {type: directory, path: knowledge}\n").unwrap();
    std::fs::write(
        root.join("knowledge/ontology.yaml"),
        "okf_ontology: '0.1'\nconcepts:\n  Note: {}\n",
    )
    .unwrap();
    std::fs::write(root.join("knowledge/data.json"), "{}\n").unwrap();
    std::fs::write(
        root.join("knowledge/index.md"),
        "# Index\n\n[Reader](reader.md)\n[Navigation](navigation.md)\n",
    )
    .unwrap();
    std::fs::write(root.join("knowledge/reader.md"), "---\ntype: Note\ntitle: Reader\ndescription: Reads schema and data.\n---\n[Index](index.md) [Schema](ontology.yaml) [Data](data.json)\n").unwrap();
    std::fs::write(root.join("knowledge/navigation.md"), "---\ntype: Note\ntitle: Navigation\ndescription: Has only a structural navigation link.\n---\n[Index](index.md)\n").unwrap();
    let lint = run(root, &["lint", "knowledge", "--fail-on", "any", "--json"]);
    assert!(!lint.iter().any(|r| r["kind"] == "finding"), "{lint:?}");
    let edges = run(
        root,
        &["links", "reader", "knowledge", "--details", "--json"],
    );
    assert_eq!(
        edges.iter().filter(|r| r["kind"] == "bundle-edge").count(),
        3,
        "{edges:?}"
    );
    for row in edges.iter().filter(|r| r["kind"] == "bundle-edge") {
        assert_eq!(row["status"], "resolved", "{row}");
        assert!(
            row["target"].is_null(),
            "resource must not become a concept: {row}"
        );
        assert_eq!(row["resource_bundle"], "test.docs");
        assert!(matches!(
            row["resource_kind"].as_str(),
            Some("reserved" | "artifact")
        ));
    }
    std::fs::remove_file(root.join("knowledge/data.json")).unwrap();
    let broken = run(root, &["lint", "knowledge", "--fail-on", "never", "--json"]);
    assert!(
        broken
            .iter()
            .any(|r| r["rule"] == "broken-link" && r["severity"] == "error"),
        "{broken:?}"
    );
}
