use assert_cmd::prelude::*;
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn write(root: &Path, path: &str, content: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn fixture() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    write(root.path(), "okf-catalog.yaml", "catalog_version: 2\ndefault_bundle: demo.a\nbundles:\n  demo.a: {location: {type: directory, path: a}}\n  demo.b: {location: {type: directory, path: b}}\n");
    write(root.path(), "a/ontology.yaml", "okf_ontology: '0.1'\nconcepts:\n  Note:\n    references:\n      informs: {target: 'demo.b:Note', cardinality: 0..n}\n      nested: {selector: 'records[].target', target: 'demo.b:Note', cardinality: 0..n}\n");
    write(
        root.path(),
        "b/x.md",
        "---\ntype: Note\ntitle: X\ndescription: Target\n---\n",
    );
    root
}

fn lint(root: &Path, scoped: bool) -> Output {
    let mut command = Command::cargo_bin("okf").unwrap();
    command
        .current_dir(root)
        .env_remove("OKF_BUNDLE")
        .env_remove("OKF_CATALOG")
        .env_remove("OKF_CATALOG_OVERRIDES");
    command.args(["lint", "--json", "--guidance", "off", "--fail-on", "never"]);
    if scoped {
        command.args(["--bundle-id", "demo.a", "--scope-bundle", "demo.b"]);
    } else {
        command.arg("a");
    }
    command.output().unwrap()
}

fn invalid(output: &Output) -> Vec<Value> {
    assert!(output.status.success(), "{output:?}");
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .filter(|row| row["rule"] == "invalid-reference")
        .collect()
}

#[test]
fn qualified_cli_targets_are_diagnosed_in_all_authored_reference_locations() {
    let root = fixture();
    write(root.path(), "a/n.md", "---\ntype: Note\ntitle: N\ndescription: Test\ninforms: [demo.b:/x.md]\nrecords: [{target: 'demo.b:/x.md#detail'}]\nsources: [{resource: 'demo.b:/evidence.json'}]\nordinary_text: 'demo.b:/not-a-reference.md'\n---\n[Target](demo.b:/x.md)\n![Image](demo.b:/image.png)\n");
    for scoped in [false, true] {
        let rows = invalid(&lint(root.path(), scoped));
        assert_eq!(rows.len(), 5, "{rows:#?}");
        assert!(rows
            .iter()
            .all(|row| row["severity"] == "warn" && row["code"] == "qualified-cli-target"));
        let paths: Vec<_> = rows
            .iter()
            .map(|row| row["field_path"].as_str().unwrap())
            .collect();
        assert!(paths.contains(&"informs[0]"), "{paths:?}");
        assert!(paths.contains(&"records[0].target"), "{paths:?}");
        assert!(paths.contains(&"sources[0].resource"), "{paths:?}");
        assert!(rows.iter().any(|row| row["message"]
            .as_str()
            .unwrap()
            .contains("okf://demo.b/x.md#detail")));
    }
}

#[test]
fn supported_references_and_external_uri_schemes_are_not_diagnosed() {
    let root = fixture();
    write(root.path(), "a/n.md", "---\ntype: Note\ntitle: N\ndescription: Test\ninforms: [okf://demo.b/x.md, ../b/x.md]\nsources:\n- {resource: 'git:deadbeef:path/to/file'}\n- {resource: 'file:/tmp/example'}\n- {resource: 'custom.scheme:/external'}\n- {resource: 'demo.b://legitimate-uri'}\n---\n[Mail](mailto:user@example.com)\n[Web](https://example.com)\n[Custom](custom:/path)\n[File](file:/tmp/example)\n[Logical](okf://demo.b/x.md)\n");
    for scoped in [false, true] {
        assert!(invalid(&lint(root.path(), scoped)).is_empty());
    }
}

#[test]
fn invalid_reference_severity_is_configurable_without_disabling_other_lints() {
    let root = fixture();
    write(
        root.path(),
        "a/n.md",
        "---\ntype: Note\ntitle: N\ndescription: Test\n---\n[Target](demo.b:/x.md)\n",
    );
    for severity in ["off", "info", "error"] {
        write(
            root.path(),
            "a/okf-bundle.yaml",
            &format!(
                "manifest_version: 1\nsettings:\n  lint:\n    invalid_reference: {severity}\n"
            ),
        );
        for scoped in [false, true] {
            let rows = invalid(&lint(root.path(), scoped));
            if severity == "off" {
                assert!(rows.is_empty());
            } else {
                assert_eq!(rows.len(), 1);
                assert_eq!(rows[0]["severity"], severity);
            }
        }
    }
}

#[test]
fn unregistered_cli_like_strings_remain_external_without_guessing_identity() {
    let root = fixture();
    write(root.path(), "a/n.md", "---\ntype: Note\ntitle: N\ndescription: Test\ninforms: [unknown.bundle:/x.md]\n---\n[Unknown](unknown.bundle:/x.md)\n");
    assert!(invalid(&lint(root.path(), false)).is_empty());
}
