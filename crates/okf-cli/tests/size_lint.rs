use assert_cmd::prelude::*;
use serde_json::Value;
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

fn run(root: &Path, args: &[&str], ci: bool) -> Output {
    let mut cmd = Command::cargo_bin("okf").unwrap();
    cmd.current_dir(root)
        .args(args)
        .arg("--json")
        .env_remove("OKF_SETTINGS_OVERRIDES");
    if ci {
        cmd.env("OKF_SETTINGS_OVERRIDES", "ci.yaml");
    }
    cmd.output().unwrap()
}

fn records(output: &Output, expected: i32) -> Vec<Value> {
    assert_eq!(output.status.code(), Some(expected), "{output:?}");
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

#[test]
fn physical_lines_and_partial_ci_policy() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::write(root.join("note.md"),"---\r\ntitle: hello\r\n# comment\r\n---\r\néééé\r\nééééé\r\n\r\n```\r\n123456\r\n```\r\n\r\n[link](https://example.org)\r\n\r\n| abcd |\r\n| ---- |\r\n| data |\r\n\r\n").unwrap();
    fs::write(root.join("okf-bundle.yaml"),"manifest_version: 1\nsettings:\n  lint:\n    body_max_lines: warn\n    body_max_lines_limit: 12\n    frontmatter_max_lines: warn\n    frontmatter_max_lines_limit: 2\n    max_line_length: warn\n    max_line_length_limit: 4\n").unwrap();
    let local = records(&run(root, &["lint"], false), 0);
    assert!(
        !local
            .iter()
            .any(|v| v["rule"] == "body-max-lines" || v["rule"] == "frontmatter-max-lines")
    );
    assert!(local.iter().any(|v| v["rule"] == "max-line-length"
        && v["message"] == "1 line exceeds 4 characters (first: 6)"));
    fs::write(root.join("ci.yaml"),"settings:\n  lint:\n    body_max_lines_limit: 11\n    frontmatter_max_lines_limit: 1\n    max_line_length: error\n    max_line_length_scope: all\n").unwrap();
    let ci = records(&run(root, &["lint"], true), 1);
    for (rule, message) in [
        ("body-max-lines", "body has 12 lines (max 11)"),
        ("frontmatter-max-lines", "frontmatter has 2 lines (max 1)"),
        (
            "max-line-length",
            "3 lines exceed 4 characters (first: 2, 3, 6)",
        ),
    ] {
        assert!(
            ci.iter()
                .any(|v| v["rule"] == rule && v["message"] == message),
            "{ci:?}"
        );
    }
    let context = records(&run(root, &["bundle", "context", "."], true), 0);
    let head = context
        .iter()
        .find(|row| row["kind"] == "bundle-context")
        .unwrap();
    assert_eq!(head["setting_origins"]["lint.body_max_lines"], "manifest");
    assert_eq!(
        head["setting_origins"]["lint.max_line_length"],
        "override:ci.yaml"
    );
}

#[test]
fn catalog_inherits_each_bundles_limit() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::write(root.join("okf-catalog.yaml"),"catalog_version: 2\ndefault_bundle: test.a\nbundles:\n  test.a: {location: {type: directory, path: a}}\n  test.b: {location: {type: directory, path: b}}\n").unwrap();
    for (id, max) in [("a", 1), ("b", 2)] {
        let bundle = root.join(id);
        fs::create_dir(&bundle).unwrap();
        fs::write(bundle.join("note.md"), "one\ntwo\n").unwrap();
        fs::write(
            bundle.join("okf-bundle.yaml"),
            format!("manifest_version: 1\nsettings:\n  lint:\n    body_max_lines_limit: {max}\n"),
        )
        .unwrap();
    }
    let local = records(&run(root, &["lint", "--catalog-scope"], false), 0);
    assert!(!local.iter().any(|v| v["rule"] == "body-max-lines"));
    fs::write(
        root.join("ci.yaml"),
        "settings: {lint: {body_max_lines: error}}",
    )
    .unwrap();
    let ci = records(&run(root, &["lint", "--catalog-scope"], true), 1);
    let findings: Vec<_> = ci
        .iter()
        .filter(|v| v["rule"] == "body-max-lines")
        .collect();
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0]["bundle"], "test.a");
}

#[test]
fn invalid_overrides_fail_even_with_never_threshold() {
    let dir = tempfile::tempdir().unwrap();
    for value in [
        "body_max_lines: warn",
        "max_line_length_limit: 0",
        "typo: warn",
        "orphan: null",
    ] {
        fs::write(
            dir.path().join("ci.yaml"),
            format!("settings: {{lint: {{{value}}}}}"),
        )
        .unwrap();
        let output = run(dir.path(), &["lint", "--fail-on", "never"], true);
        assert!(!output.status.success(), "{value}");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("OKF_SETTINGS_OVERRIDES"),
            "{output:?}"
        );
    }
}

#[test]
fn override_cannot_hide_a_legacy_manifest_conflict() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::write(
        root.join("okf.toml"),
        "[bundle_settings.default.lint]\norphan='warn'\n",
    )
    .unwrap();
    fs::write(
        root.join("okf-bundle.yaml"),
        "manifest_version: 1\nsettings: {lint: {orphan: info}}\n",
    )
    .unwrap();
    fs::write(root.join("ci.yaml"), "settings: {lint: {orphan: error}}\n").unwrap();
    let output = run(root, &["lint"], true);
    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("conflict"),
        "{output:?}"
    );
}
