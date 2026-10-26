use assert_cmd::prelude::*;
use serde_json::{json, Value};
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

fn write(root: &Path, path: &str, text: &str) {
    let path = root.join(path);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn rows(output: &[u8]) -> Vec<Value> {
    std::str::from_utf8(output)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

fn run(root: &Path, args: &[&str]) -> std::process::Output {
    Command::cargo_bin("okf")
        .unwrap()
        .current_dir(root)
        .env_remove("OKF_BUNDLE")
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn list_stream_has_exact_control_records_and_partial_scan_evidence() {
    let temp = tempfile::tempdir().unwrap();
    write(temp.path(), "a.md", "---\ntype: Note\ntitle: A\n---\n");
    write(temp.path(), "b.md", "---\ntype: Note\ntitle: B\n---\n");
    let complete = run(temp.path(), &["list", "--json"]);
    assert_eq!(complete.status.code(), Some(0), "{complete:?}");
    let complete = rows(&complete.stdout);
    assert!(complete.iter().any(|row| row["kind"] == "query-summary"
        && row["scan_complete"] == true
        && row["total_matches"] == 2));
    assert_eq!(
        complete.first().unwrap(),
        &json!({"kind":"operation-start","schema_version":1,"command":"list","stream_version":1})
    );
    assert_eq!(
        complete.last().unwrap(),
        &json!({"kind":"operation-complete","schema_version":1,"outcome":"ok","exit_code":0,"publication":"not-applicable","completeness":{"examination":"complete","matching":"complete","delivery":"complete","expansion":"not-applicable","content":"not-applicable","consistency":"working-tree"},"reasons":[]})
    );
    let partial = run(temp.path(), &["list", "--json", "--scan-limit", "1"]);
    assert_eq!(partial.status.code(), Some(0), "{partial:?}");
    let partial = rows(&partial.stdout);
    assert!(partial.iter().any(|row| row["kind"] == "query-summary"
        && row["scan_complete"] == false
        && row["total_matches"].is_null()));
    assert_eq!(partial.first().unwrap()["kind"], "operation-start");
    assert_eq!(
        partial.last().unwrap(),
        &json!({"kind":"operation-complete","schema_version":1,"outcome":"partial","exit_code":0,"publication":"not-applicable","completeness":{"examination":"partial","matching":"partial","delivery":"complete","expansion":"not-applicable","content":"not-applicable","consistency":"working-tree"},"reasons":["scan-limit"]})
    );
}

#[test]
fn affected_explicit_targets_do_not_read_open_inherited_stdin() {
    let temp = tempfile::tempdir().unwrap();
    write(temp.path(), "a.md", "---\ntype: Note\n---\n[x](/b)\n");
    write(temp.path(), "b.md", "---\ntype: Note\n---\n");
    let mut child = Command::cargo_bin("okf")
        .unwrap()
        .current_dir(temp.path())
        .args(["affected", "--changed", "b.md", "--json"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut exited = false;
    for _ in 0..40 {
        if child.try_wait().unwrap().is_some() {
            exited = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    if !exited {
        child.kill().unwrap();
    }
    let output = child.wait_with_output().unwrap();
    assert!(exited, "affected waited for unused stdin");
    assert_eq!(output.status.code(), Some(0));
    assert!(rows(&output.stdout)
        .iter()
        .any(|row| row["kind"] == "affected" && row["concept"] == "/a"));
}

#[test]
fn changed_stdin_is_explicit_and_path_bases_are_distinct() {
    let temp = tempfile::tempdir().unwrap();
    write(temp.path(), "bundle/a.md", "---\ntype: Note\n---\n");
    write(temp.path(), "bundle/data.csv", "x\n");
    write(
        temp.path(),
        "bundle/b.md",
        "---\ntype: Note\nsources:\n- {resource: data.csv, kind: file}\n---\n",
    );
    let mut command = Command::cargo_bin("okf").unwrap();
    let mut child = command
        .current_dir(temp.path())
        .args([
            "affected",
            "bundle",
            "--changed",
            "-",
            "--changed-base",
            "filesystem",
            "--json",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"bundle/data.csv\n")
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(rows(&output.stdout)
        .iter()
        .any(|row| row["kind"] == "affected" && row["concept"] == "/b"));
    let duplicate = run(
        temp.path(),
        &[
            "affected",
            "bundle",
            "--changed",
            "-",
            "--changed",
            "-",
            "--json",
        ],
    );
    assert_eq!(duplicate.status.code(), Some(2));
    let missing_repo = run(
        temp.path(),
        &[
            "affected",
            "bundle",
            "--changed",
            "data.csv",
            "--changed-base",
            "repo",
            "--json",
        ],
    );
    assert_eq!(missing_repo.status.code(), Some(2));
}

#[test]
fn repo_base_resolves_from_worktree_root_for_nested_bundle() {
    let temp = tempfile::tempdir().unwrap();
    let git = Command::new("git")
        .current_dir(temp.path())
        .args(["init", "-q"])
        .output()
        .unwrap();
    assert!(git.status.success(), "{git:?}");
    write(temp.path(), "src/data.csv", "x\n");
    write(
        temp.path(),
        "knowledge/note.md",
        "---\ntype: Note\nsources:\n- {resource: src/data.csv, kind: git-path}\n---\n",
    );
    let output = run(
        temp.path(),
        &[
            "affected",
            "knowledge",
            "--changed",
            "src/data.csv",
            "--changed-base",
            "repo",
            "--json",
        ],
    );
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(rows(&output.stdout)
        .iter()
        .any(|row| row["kind"] == "affected" && row["concept"] == "/note"));
}

#[test]
fn catalog_affected_uses_the_same_explicit_filesystem_namespace() {
    let temp = tempfile::tempdir().unwrap();
    write(temp.path(), "a/data.csv", "x\n");
    write(
        temp.path(),
        "a/note.md",
        "---\ntype: Note\nsources:\n- {resource: data.csv, kind: file}\n---\n",
    );
    write(
        temp.path(),
        "okf.toml",
        "catalog='catalog.yaml'\ndefault_bundle={id='acme.a'}\n",
    );
    write(
        temp.path(),
        "catalog.yaml",
        "catalog_version: 1\nbundles:\n  acme.a: {location: {type: directory, path: a}}\n",
    );
    let output = run(
        temp.path(),
        &[
            "affected",
            "--changed",
            "a/data.csv",
            "--changed-base",
            "filesystem",
            "--catalog-scope",
            "--json",
        ],
    );
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(rows(&output.stdout)
        .iter()
        .any(|row| row["kind"] == "bundle-affected"
            && row["bundle"] == "acme.a"
            && row["id"] == "/note"));
}

#[test]
fn digest_stream_reports_guarded_baseline_without_publication() {
    let temp = tempfile::tempdir().unwrap();
    write(temp.path(), "note.md", "---\ntype: Note\n---\n");
    let output = run(temp.path(), &["changeset", "digest", "--json"]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let records = rows(&output.stdout);
    assert_eq!(records.len(), 3, "{records:?}");
    assert_eq!(
        records[0],
        json!({"kind":"operation-start","schema_version":1,"command":"changeset digest","stream_version":1})
    );
    assert_eq!(records[1]["kind"], "baseline");
    assert_eq!(records[1]["schema_version"], 1);
    assert_eq!(
        records[1]["bundle"],
        std::fs::canonicalize(temp.path())
            .unwrap()
            .to_string_lossy()
            .to_string()
    );
    assert!(records[1]["base_digest"]
        .as_str()
        .is_some_and(|s| !s.is_empty()));
    assert_eq!(
        records[2],
        json!({"kind":"operation-complete","schema_version":1,"outcome":"ok","exit_code":0,"publication":"not-published","completeness":{"examination":"complete","matching":"not-applicable","delivery":"complete","expansion":"not-applicable","content":"not-applicable","consistency":"guarded-baseline"},"reasons":[]})
    );
}

#[test]
fn configured_gate_is_independent_of_lint_display_and_rejects_before_publication() {
    let temp = tempfile::tempdir().unwrap();
    write(temp.path(), "note.md", "---\ntype: Note\nage: 2\n---\n");
    write(
        temp.path(),
        "ontology.yaml",
        "okf_ontology: '0.1'\nconcepts:\n  Note:\n    fields:\n      age: {type: int}\n",
    );
    write(temp.path(), "okf.toml", "[bundle_settings.default.lint]\nontology_violation = 'off'\n[bundle_settings.default.write_policy]\nontology_gate = true\n");
    let before = std::fs::read(temp.path().join("note.md")).unwrap();
    let output = run(
        temp.path(),
        &["edit", "note", "--set-yaml", "age=wrong", "--json"],
    );
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert_eq!(std::fs::read(temp.path().join("note.md")).unwrap(), before);
    let records = rows(&output.stdout);
    let complete = records.last().unwrap();
    assert_eq!(complete["outcome"], "failed");
    assert_eq!(complete["publication"], "not-published");
    assert_eq!(complete["reasons"], json!(["write-gate-rejected"]));
}

#[test]
fn required_refresh_failure_and_changeset_preview_report_no_publication() {
    let temp = tempfile::tempdir().unwrap();
    write(
        temp.path(),
        "note.md",
        "---\ntype: Note\nsources:\n- {resource: missing.txt, kind: file}\n---\n",
    );
    let before = std::fs::read(temp.path().join("note.md")).unwrap();
    let failed = run(temp.path(), &["refresh", "note", "--json"]);
    assert_eq!(failed.status.code(), Some(2), "{failed:?}");
    assert_eq!(std::fs::read(temp.path().join("note.md")).unwrap(), before);
    assert_eq!(
        rows(&failed.stdout).last().unwrap()["publication"],
        "not-published"
    );
    let preview_root = tempfile::tempdir().unwrap();
    write(preview_root.path(), "other.md", "---\ntype: Note\n---\n");
    let plan = run(
        preview_root.path(),
        &[
            "changeset",
            "apply",
            "--from",
            "version: 1\noperations:\n- op: remove\n  path: other.md\n",
            "--dry-run",
            "--json",
        ],
    );
    assert_eq!(plan.status.code(), Some(0), "{plan:?}");
    assert_eq!(std::fs::read(temp.path().join("note.md")).unwrap(), before);
    assert_eq!(
        rows(&plan.stdout).last().unwrap()["publication"],
        "not-published"
    );
}

#[test]
fn ineffective_flags_and_scoped_fix_are_usage_errors() {
    let temp = tempfile::tempdir().unwrap();
    for args in [
        vec!["affected", "--changed", "x", "--depth", "2"],
        vec!["computation", "check", "/x", "--details"],
        vec!["init", "--title", "X", "--no-index"],
        vec!["lint", "--fix", "--catalog-scope"],
    ] {
        let result = run(temp.path(), &args);
        assert_eq!(result.status.code(), Some(2), "{args:?}: {result:?}");
    }
}
