use assert_cmd::Command;
use okf_core::mutate::transaction::{self, JOURNAL, LOCK};
use predicates::prelude::*;
use std::fs;
use std::path::Path;

fn command(root: &Path) -> Command {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("okf"));
    cmd.current_dir(root)
        .env_remove("OKF_BUNDLE")
        .env_remove("OKF_CATALOG")
        .env_remove("OKF_CATALOG_OVERRIDES");
    cmd
}

fn bundle(root: &Path) {
    fs::create_dir_all(root).unwrap();
    fs::write(
        root.join("n.md"),
        "---\ntype: Note\ntitle: N\ndescription: d\n---\nBody\n",
    )
    .unwrap();
}

#[test]
fn working_tree_commands_refuse_a_writer_before_reading_payloads() {
    let root = tempfile::tempdir().unwrap();
    bundle(root.path());
    let guard = transaction::writer_lock(root.path()).unwrap();
    for args in [
        vec!["list"],
        vec!["search"],
        vec!["show", "/n"],
        vec!["links", "/n"],
        vec!["backlinks", "/n"],
        vec!["stats"],
        vec!["validate"],
        vec!["doctor"],
        vec!["bundle", "context"],
        vec!["artifact", "list"],
    ] {
        let result = command(root.path())
            .args(args)
            .args(["--json", "--guidance", "off"])
            .assert()
            .code(2)
            .stderr(predicate::str::contains("transaction is still active"));
        let rows: Vec<serde_json::Value> = String::from_utf8_lossy(&result.get_output().stdout)
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert!(rows
            .iter()
            .all(|row| row["kind"] == "operation-start" || row["kind"] == "operation-complete"));
        assert_eq!(rows.last().unwrap()["outcome"], "failed");
    }
    drop(guard);
    command(root.path())
        .args(["list", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"title\":\"N\""));
}

#[test]
fn pending_recovery_refuses_reads_but_recovery_remains_callable() {
    let root = tempfile::tempdir().unwrap();
    bundle(root.path());
    fs::create_dir(root.path().join(JOURNAL)).unwrap();
    command(root.path())
        .args(["list", "--json"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("pending transaction"));
    command(root.path())
        .args(["changeset", "recover"])
        .assert()
        .success();
    command(root.path()).arg("list").assert().success();
}

#[test]
fn scoped_read_refuses_busy_secondary_and_releases_primary() {
    let root = tempfile::tempdir().unwrap();
    let a = root.path().join("a");
    let b = root.path().join("b");
    bundle(&a);
    bundle(&b);
    fs::write(root.path().join("okf-catalog.yaml"), "catalog_version: 2\ndefault_bundle: demo.a\nbundles:\n  demo.a: {location: {type: directory, path: a}}\n  demo.b: {location: {type: directory, path: b}}\n").unwrap();
    let guard = transaction::writer_lock(&b).unwrap();
    command(root.path())
        .args(["list", "--scope-bundle", "demo.b", "--json"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("transaction is still active"));
    assert!(transaction::writer_lock(&a).is_ok());
    drop(guard);
    command(root.path())
        .args(["list", "--scope-bundle", "demo.b", "--json"])
        .assert()
        .success();
}

#[test]
fn reader_keeps_barrier_while_streaming_its_payload() {
    use std::io::{BufRead, BufReader, Read};
    use std::process::Stdio;
    let root = tempfile::tempdir().unwrap();
    // More output than the pipe can buffer makes the producer pause mid-stream.
    let description = "x".repeat(32 * 1024);
    for i in 0..64 {
        fs::write(
            root.path().join(format!("n{i}.md")),
            format!("---\ntype: Note\ntitle: N\ndescription: {description}\n---\n"),
        )
        .unwrap();
    }
    let mut child = std::process::Command::new(assert_cmd::cargo::cargo_bin!("okf"))
        .current_dir(root.path())
        .env_remove("OKF_BUNDLE")
        .env_remove("OKF_CATALOG")
        .env_remove("OKF_CATALOG_OVERRIDES")
        .args(["list", "--json", "--full-scan", "--guidance", "off"])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut reader = BufReader::new(child.stdout.take().unwrap());
    loop {
        let mut line = String::new();
        assert_ne!(reader.read_line(&mut line).unwrap(), 0);
        let row: serde_json::Value = serde_json::from_str(&line).unwrap();
        if row.get("id").is_some() && row.get("kind").is_none() {
            break;
        }
    }
    let blocked = transaction::writer_lock(root.path()).is_err();
    // Drain before asserting so failures cannot leave a blocked child behind.
    reader.read_to_end(&mut Vec::new()).unwrap();
    assert!(child.wait().unwrap().success());
    assert!(
        blocked,
        "writer entered while the reader was still delivering rows"
    );
    assert!(transaction::writer_lock(root.path()).is_ok());
}

#[cfg(unix)]
#[test]
fn read_only_bundle_without_lock_remains_readable_without_changes() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    bundle(root.path());
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o555)).unwrap();
    command(root.path())
        .args(["list", "--json"])
        .assert()
        .success();
    assert!(!root.path().join(LOCK).exists());
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o755)).unwrap();
}
