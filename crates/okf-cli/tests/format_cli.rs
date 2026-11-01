use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
fn okf() -> Command {
    Command::new(assert_cmd::cargo::cargo_bin!("okf"))
}
fn manifest(dir: &std::path::Path, extra: &str) {
    fs::write(
        dir.join("okf-bundle.yaml"),
        format!("manifest_version: 1\nsettings:\n  formatting:\n{extra}"),
    )
    .unwrap();
}
#[test]
fn check_preview_apply_and_verification_preservation() {
    let dir = tempfile::tempdir().unwrap();
    manifest(
        dir.path(),
        "    yaml: {line_width: 40}\n    markdown: {wrap_prose: true, line_width: 35}\n",
    );
    let original = "---\ntype: Note\nnotes:\n- 'A long explanation: many ordinary words describing the reason for this decision and its consequences.'\nverified:\n- at: '2026-11-01T00:00:00Z'\n  by: human:test\n---\n# Keep this long heading exactly as originally authored\n\nThis is a paragraph with sufficient ordinary words to wrap across multiple lines safely.\n";
    fs::write(dir.path().join("a.md"), original).unwrap();
    fs::create_dir(dir.path().join(".okf-bundle")).unwrap();
    fs::write(dir.path().join(".okf-bundle/support.md"), original).unwrap();
    okf()
        .args(["format", "--check", "--bundle"])
        .arg(dir.path())
        .assert()
        .code(1)
        .stdout(predicate::str::contains("a.md"));
    assert_eq!(
        fs::read_to_string(dir.path().join("a.md")).unwrap(),
        original
    );
    okf()
        .args(["format", "a", "--dry-run", "--bundle"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(">-"));
    assert_eq!(
        fs::read_to_string(dir.path().join("a.md")).unwrap(),
        original
    );
    let result = okf()
        .args(["format", "--json", "--bundle"])
        .arg(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let rows: Vec<serde_json::Value> = String::from_utf8(result)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert!(rows
        .iter()
        .any(|r| r["kind"] == "format-summary" && r["files"] == 1));
    assert_eq!(rows.last().unwrap()["publication"], "committed");
    let formatted = fs::read_to_string(dir.path().join("a.md")).unwrap();
    assert!(formatted.contains(">-\n"));
    assert!(formatted.contains("by: human:test"));
    assert!(formatted.contains("# Keep this long heading exactly as originally authored\n"));
    assert_eq!(
        fs::read_to_string(dir.path().join(".okf-bundle/support.md")).unwrap(),
        original
    );
    okf()
        .args(["format", "--check", "--bundle"])
        .arg(dir.path())
        .assert()
        .success();
    okf()
        .args(["format", "--bundle"])
        .arg(dir.path())
        .assert()
        .success();
    assert_eq!(
        fs::read_to_string(dir.path().join("a.md")).unwrap(),
        formatted
    );
}

#[test]
fn ordinary_edits_respect_yaml_configuration_and_do_not_wrap_markdown() {
    let dir = tempfile::tempdir().unwrap();
    let original = "---\ntype: Note\n---\n";
    fs::write(dir.path().join("a.md"), original).unwrap();
    manifest(dir.path(), "    yaml: {style: legacy, line_width: 20}\n    markdown: {wrap_prose: true, line_width: 20}\n");
    let note =
        "description=A description: with many ordinary words that could be wrapped across lines";
    let body = "A body with many ordinary words that must not wrap during an ordinary edit.";
    okf()
        .args(["edit", "a", "--set", note, "--set-body", body])
        .arg(dir.path())
        .assert()
        .success();
    let legacy = fs::read_to_string(dir.path().join("a.md")).unwrap();
    assert!(!legacy.contains(">-"));
    assert!(legacy.contains(body));
    manifest(dir.path(), "    yaml: {style: readable, line_width: 20}\n    markdown: {wrap_prose: true, line_width: 20}\n");
    okf()
        .args(["edit", "a", "--set", "title=Example"])
        .arg(dir.path())
        .assert()
        .success();
    let readable = fs::read_to_string(dir.path().join("a.md")).unwrap();
    assert!(readable.contains(">-"));
    assert!(readable.contains(body));
}

#[test]
fn malformed_config_and_paths_fail_without_writes() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("a.md"), "---\ntype: Note\n---\n").unwrap();
    manifest(dir.path(), "    yaml: {style: unknown}\n");
    okf()
        .args(["format", "--bundle"])
        .arg(dir.path())
        .assert()
        .failure();
    manifest(dir.path(), "    markdown: {wrap_prose: false}\n");
    okf()
        .args(["format", "../outside", "--bundle"])
        .arg(dir.path())
        .assert()
        .code(2);
    okf()
        .args(["format", "--check", "--dry-run", "--bundle"])
        .arg(dir.path())
        .assert()
        .code(2);
}

#[test]
fn schema_reports_format_as_a_conditional_mutation() {
    let output = okf()
        .args(["schema", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let rows: Vec<serde_json::Value> = String::from_utf8(output)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let format = rows
        .iter()
        .find(|row| row["kind"] == "command" && row["name"] == "format")
        .unwrap();
    assert_eq!(format["group"], "mutate");
    assert_eq!(format["mutates"], true);
    assert_eq!(
        format["mutates_when"],
        serde_json::json!({"check": false, "dry-run": false})
    );
    assert_eq!(format["output"]["stream"], "format-file,format-summary");
}
