use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use std::fs;

fn okf() -> Command {
    Command::new(assert_cmd::cargo::cargo_bin!("okf"))
}
fn rows(output: &[u8]) -> Vec<Value> {
    String::from_utf8_lossy(output)
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .filter(|record: &Value| {
            !matches!(
                record["kind"].as_str(),
                Some("operation-start" | "operation-complete")
            )
        })
        .collect()
}

#[test]
fn plan_apply_and_stale_preview_guard() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("old.md"), "---\ntype: Note\n---\n# Old\n").unwrap();
    fs::write(
        dir.path().join("ref.md"),
        "---\ntype: Note\n---\n[old](/old.md)\n",
    )
    .unwrap();
    let plan = "version: 1\noperations:\n- op: move\n  from: old\n  to: nested/new\n";
    let output = okf()
        .args(["changeset", "plan", "--from", plan, "--json"])
        .arg(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let records = rows(&output);
    let summary = records.last().unwrap();
    assert_eq!(summary["applied"], false);
    assert_eq!(summary["files"], 3);
    let digest = summary["base_digest"].as_str().unwrap();
    assert!(dir.path().join("old.md").exists());
    assert!(!dir.path().join("nested").exists());
    okf()
        .args([
            "changeset",
            "apply",
            "--from",
            plan,
            "--expect",
            digest,
            "--json",
        ])
        .arg(dir.path())
        .assert()
        .success();
    assert!(!dir.path().join("old.md").exists());
    assert!(dir.path().join("nested/new.md").exists());
    assert!(fs::read_to_string(dir.path().join("ref.md"))
        .unwrap()
        .contains("/nested/new.md"));
    let edit = "version: 1\noperations:\n- op: edit\n  concept: nested/new\n  body: changed\n";
    okf()
        .args(["changeset", "apply", "--from", edit, "--expect", digest])
        .arg(dir.path())
        .assert()
        .code(2)
        .stderr(predicate::str::contains("digest differs"));
    assert!(!dir.path().join(".okf-transaction").exists());
}

#[test]
fn failed_plan_does_not_publish_earlier_operations() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("old.md"), "---\ntype: Note\n---\n# Old\n").unwrap();
    let plan = "version: 1\noperations:\n- op: move\n  from: old\n  to: new\n- op: remove\n  path: absent.md\n";
    okf()
        .args(["changeset", "apply", "--from", plan])
        .arg(dir.path())
        .assert()
        .failure();
    assert!(dir.path().join("old.md").exists());
    assert!(!dir.path().join("new.md").exists());
}

#[test]
fn unsupported_globals_hidden_and_machine_capabilities_explicit() {
    okf()
        .args(["show", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--revision"))
        .stdout(predicate::str::contains("--scope-bundle").not());
    okf()
        .args(["search", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--revision"));
    // Historical inspection is now supported; mutation commands still reject it.
    okf()
        .args(["edit", "x", "--revision", "HEAD"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("--revision is supported only"));
    let output = okf()
        .arg("schema")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let records = rows(&output);
    let command = |name| records.iter().find(|r| r["name"] == name).unwrap();
    assert_eq!(command("show")["supported_globals"]["revision"], true);
    assert_eq!(command("search")["supported_globals"]["revision"], true);
    let arg = |name, flag| {
        command(name)["args"]
            .as_array()
            .unwrap()
            .iter()
            .find(|a| a["name"] == flag)
            .unwrap()
    };
    assert_eq!(arg("resolve", "from")["stdin"], false);
    assert_eq!(arg("artifact show", "from")["stdin"], false);
    assert_eq!(arg("add", "body")["stdin"], true);
    assert_eq!(arg("edit", "set-section")["stdin"], true);
    assert_eq!(arg("changeset apply", "from")["stdin"], true);
    assert_eq!(
        arg("search", "offset")["requires"],
        serde_json::json!(["limit"])
    );
    assert!(arg("show", "outline")["conflicts_with"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("lines")));
    assert_eq!(command("changeset plan")["mutates"], false);
    assert_eq!(command("changeset apply")["mutates"], true);
    assert_eq!(command("ontology field-type show")["mutates"], false);
}

#[test]
fn pending_publication_blocks_unrelated_writers_until_recovery() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("old.md"), "---\ntype: Note\n---\n# Old\n").unwrap();
    fs::create_dir(dir.path().join(".okf-transaction")).unwrap();
    okf()
        .args(["edit", "old", "--set", "title=changed"])
        .arg(dir.path())
        .assert()
        .code(2)
        .stderr(predicate::str::contains("pending transaction"));
    okf()
        .args(["changeset", "recover", "--json"])
        .arg(dir.path())
        .assert()
        .success();
    okf()
        .args(["edit", "old", "--set", "title=changed"])
        .arg(dir.path())
        .assert()
        .success();
}

#[test]
fn support_markdown_uses_bundle_profile_and_remains_in_the_reviewed_baseline() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join(".okf-bundle")).unwrap();
    let support = dir.path().join(".okf-bundle/authoring.md");
    fs::write(&support, "Plain Markdown instructions, not a concept.\n").unwrap();
    fs::write(
        dir.path().join("okf-bundle.yaml"),
        "manifest_version: 1\nguidance:\n  instructions_file: .okf-bundle/authoring.md\n",
    )
    .unwrap();
    let original = "---\ntype: Instruction\nstatus: stable\n---\nOriginal body.\n";
    fs::write(dir.path().join("note.md"), original).unwrap();
    let noop = r#"{"version":1,"operations":[{"op":"edit","concept":"/note","patch":[{"op":"test","path":"/status","value":"stable"}]}]}"#;
    let output = okf()
        .args(["changeset", "plan", "--from", noop, "--json"])
        .arg(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let data = rows(&output);
    let summary = data
        .iter()
        .find(|r| r["kind"] == "changeset-summary")
        .unwrap();
    assert_eq!(summary["files"], 0);
    let digest = summary["base_digest"].as_str().unwrap();
    okf()
        .args(["changeset", "apply", "--from", noop, "--expect", digest])
        .arg(dir.path())
        .assert()
        .success();
    assert_eq!(
        fs::read_to_string(dir.path().join("note.md")).unwrap(),
        original
    );
    assert_eq!(
        fs::read_to_string(&support).unwrap(),
        "Plain Markdown instructions, not a concept.\n"
    );
    let edit = r#"{"version":1,"operations":[{"op":"edit","concept":"/note","body":"Updated body."},{"op":"create","path":"other.md","content":"---\ntype: Note\n---\nRelated concept.\n"}]}"#;
    // A support-file change must still invalidate the reviewed baseline.
    fs::write(&support, "Updated instructions.\n").unwrap();
    okf()
        .args(["changeset", "apply", "--from", edit, "--expect", digest])
        .arg(dir.path())
        .assert()
        .code(2)
        .stderr(predicate::str::contains("digest differs"));
    assert_eq!(
        fs::read_to_string(dir.path().join("note.md")).unwrap(),
        original
    );
    assert!(!dir.path().join("other.md").exists());
    okf()
        .args(["changeset", "apply", "--from", edit])
        .arg(dir.path())
        .assert()
        .success();
    assert!(fs::read_to_string(dir.path().join("note.md"))
        .unwrap()
        .contains("Updated body."));
    assert!(dir.path().join("other.md").exists());
    assert_eq!(
        fs::read_to_string(&support).unwrap(),
        "Updated instructions.\n"
    );
    // The exclusion applies only to root support files, never ordinary or nested Markdown.
    for path in ["broken.md", "nested/.okf-bundle/broken.md"] {
        let target = dir.path().join(path);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(&target, "Missing required frontmatter.\n").unwrap();
        okf()
            .args(["changeset", "plan", "--from", noop])
            .arg(dir.path())
            .assert()
            .code(2)
            .stderr(predicate::str::contains("nonconformant documents"));
        fs::remove_file(target).unwrap();
    }
}
