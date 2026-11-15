use assert_cmd::Command;
use predicates::prelude::*;
use std::{fs, path::Path};

fn command(root: &Path) -> Command {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("okf"));
    cmd.current_dir(root)
        .env_remove("OKF_BUNDLE")
        .env_remove("OKF_CATALOG")
        .env_remove("OKF_CATALOG_OVERRIDES");
    cmd
}

fn fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("a")).unwrap();
    fs::write(
        dir.path().join("a/n.md"),
        "---\ntype: Note\ntitle: N\n---\nBody\n",
    )
    .unwrap();
    fs::write(dir.path().join("okf-catalog.yaml"), "catalog_version: 2\ndefault_bundle: demo.a\nbundles:\n  demo.a: {location: {type: directory, path: a}}\n").unwrap();
    dir
}

fn select(root: &Path, args: &[&str]) -> String {
    let output = command(root)
        .args(["bundle", "select"])
        .args(args)
        .arg("--json")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let record = String::from_utf8(output)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .find(|row| row["kind"] == "bundle-selection")
        .unwrap();
    record["token"].as_str().unwrap().to_owned()
}

#[test]
fn pending_recovery_can_be_selected_by_path_id_or_default() {
    for args in [vec!["a"], vec!["--bundle-id", "demo.a"], vec![]] {
        let dir = fixture();
        fs::create_dir(dir.path().join("a/.okf-transaction")).unwrap();
        let token = select(dir.path(), &args);
        command(dir.path())
            .args(["--selection", &token, "list"])
            .assert()
            .failure()
            .stderr(predicate::str::contains("pending transaction"));
        command(dir.path())
            .args(["--selection", &token, "changeset", "recover"])
            .assert()
            .success();
        command(dir.path())
            .args(["--selection", &token, "list"])
            .assert()
            .success();
    }
}

#[test]
fn selection_file_remains_bound_after_changing_working_directory() {
    let dir = fixture();
    let token = select(dir.path(), &["--bundle-id", "demo.a"]);
    let file = dir.path().join("selection.json");
    fs::write(&file, token).unwrap();
    let elsewhere = tempfile::tempdir().unwrap();
    command(elsewhere.path())
        .args([
            "--selection",
            &format!("@{}", file.display()),
            "list",
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"title\":\"N\""));
}

#[test]
fn changed_catalog_refuses_a_previously_selected_identity() {
    let dir = fixture();
    let token = select(dir.path(), &["--bundle-id", "demo.a"]);
    fs::create_dir(dir.path().join("b")).unwrap();
    fs::write(dir.path().join("okf-catalog.yaml"), "catalog_version: 2\ndefault_bundle: demo.a\nbundles:\n  demo.a: {location: {type: directory, path: b}}\n").unwrap();
    command(dir.path())
        .args(["--selection", &token, "list"])
        .assert()
        .failure();
}

#[test]
fn standalone_selection_allows_writes() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("n.md"), "---\ntype: Note\ntitle: N\n---\n").unwrap();
    let token = select(dir.path(), &[dir.path().to_str().unwrap()]);
    command(dir.path())
        .args(["--selection", &token, "edit", "n", "--set", "title=Changed"])
        .assert()
        .success();
    assert!(
        fs::read_to_string(dir.path().join("n.md"))
            .unwrap()
            .contains("Changed")
    );
}

#[test]
fn selected_recovery_rolls_back_interrupted_publication_despite_bad_manifest() {
    let dir = fixture();
    let bundle = dir.path().join("a");
    let before = b"---\ntype: Note\ntitle: N\n---\nBody\n";
    let after = b"---\ntype: Note\ntitle: Changed\n---\nBody\n";
    fs::write(bundle.join("n.md"), after).unwrap();
    fs::write(bundle.join("okf-bundle.yaml"), "not: [valid").unwrap();
    let journal = bundle.join(".okf-transaction");
    fs::create_dir(&journal).unwrap();
    fs::write(
        journal.join("manifest.json"),
        serde_json::to_vec(&serde_json::json!({
            "changes": [{"path":"n.md", "before":before.to_vec(), "after":after.to_vec()}],
            "permissions": {}, "created_dirs": []
        }))
        .unwrap(),
    )
    .unwrap();
    let token = select(dir.path(), &["--bundle-id", "demo.a"]);
    command(dir.path())
        .args(["--selection", &token, "changeset", "recover"])
        .assert()
        .success();
    assert_eq!(fs::read(bundle.join("n.md")).unwrap(), before);
    assert!(!journal.exists());
}

#[test]
fn selection_refuses_default_remap_conflicts_and_invalid_tokens() {
    let dir = fixture();
    let token = select(dir.path(), &[]);
    fs::create_dir(dir.path().join("b")).unwrap();
    fs::write(dir.path().join("okf-catalog.yaml"), "catalog_version: 2\ndefault_bundle: demo.b\nbundles:\n  demo.a: {location: {type: directory, path: a}}\n  demo.b: {location: {type: directory, path: b}}\n").unwrap();
    command(dir.path())
        .args(["--selection", &token, "list"])
        .assert()
        .failure();
    let fresh = select(dir.path(), &["--bundle-id", "demo.a"]);
    command(dir.path())
        .args(["--selection", &fresh, "list", "--bundle-id", "demo.b"])
        .assert()
        .failure();
    command(dir.path())
        .args(["--selection", &fresh, "list", "b"])
        .assert()
        .failure();
    for invalid in ["not-json".to_owned(), "{}".to_owned(), "x".repeat(100_000)] {
        command(dir.path())
            .args(["--selection", &invalid, "list"])
            .assert()
            .failure();
    }
    let mut altered: serde_json::Value = serde_json::from_str(&fresh).unwrap();
    altered["version"] = 999.into();
    command(dir.path())
        .args(["--selection", &altered.to_string(), "list"])
        .assert()
        .failure();
}

#[cfg(unix)]
#[test]
fn replacing_a_selected_symlink_root_is_rejected() {
    let dir = fixture();
    fs::create_dir(dir.path().join("b")).unwrap();
    std::os::unix::fs::symlink("a", dir.path().join("link")).unwrap();
    let token = select(dir.path(), &["link"]);
    fs::remove_file(dir.path().join("link")).unwrap();
    std::os::unix::fs::symlink("b", dir.path().join("link")).unwrap();
    command(dir.path())
        .args(["--selection", &token, "list"])
        .assert()
        .failure();
}

#[test]
fn selection_pins_plan_and_guarded_apply_across_directories() {
    let dir = fixture();
    let token = select(dir.path(), &["--bundle-id", "demo.a"]);
    let payload = serde_json::json!({"version":1,"operations":[{"op":"create","path":"created","content":"---\ntype: Note\n---\n"}]}).to_string();
    let output = command(dir.path())
        .args([
            "--selection",
            &token,
            "changeset",
            "plan",
            "--json",
            "--from",
            &payload,
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let digest = String::from_utf8(output)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .find(|row| row["kind"] == "changeset-summary")
        .unwrap()["base_digest"]
        .as_str()
        .unwrap()
        .to_owned();
    let elsewhere = tempfile::tempdir().unwrap();
    command(elsewhere.path())
        .args([
            "--selection",
            &token,
            "changeset",
            "apply",
            "--from",
            &payload,
            "--expect",
            &digest,
        ])
        .assert()
        .success();
    assert!(dir.path().join("a/created.md").exists());
}

#[cfg(unix)]
#[test]
fn selected_git_override_remains_read_only_after_cwd_change() {
    use std::os::unix::fs::PermissionsExt;
    let dir = fixture();
    let upstream = dir.path().join("upstream");
    fs::create_dir_all(upstream.join("bundle")).unwrap();
    fs::write(
        upstream.join("bundle/n.md"),
        "---\ntype: Note\ntitle: N\n---\n",
    )
    .unwrap();
    for args in [
        vec!["init", "-q", "-b", "main"],
        vec!["config", "user.email", "test@example.invalid"],
        vec!["config", "user.name", "Test"],
        vec!["add", "."],
        vec!["commit", "-qm", "initial"],
    ] {
        assert!(
            std::process::Command::new("git")
                .current_dir(&upstream)
                .args(args)
                .status()
                .unwrap()
                .success()
        );
    }
    let ssh = dir.path().join("ssh.sh");
    fs::write(
        &ssh,
        "#!/bin/sh\nexec git-upload-pack \"$OKF_TEST_UPSTREAM\"\n",
    )
    .unwrap();
    fs::set_permissions(&ssh, fs::Permissions::from_mode(0o700)).unwrap();
    fs::write(dir.path().join("okf-catalog.yaml"), "catalog_version: 2\nbundles:\n  demo.a: {location: {type: directory, path: a}}\n  demo.remote:\n    location: {type: git, url: 'ssh://fixture.invalid/repo', ref: refs/heads/main, path: bundle}\n").unwrap();
    let run = |args: &[&str]| {
        let mut cmd = command(dir.path());
        cmd.env("GIT_SSH_COMMAND", &ssh)
            .env("GIT_SSH_VARIANT", "ssh")
            .env("OKF_TEST_UPSTREAM", &upstream)
            .env("OKF_CACHE_DIR", dir.path().join("cache"))
            .args(args);
        cmd.assert().success();
    };
    run(&["catalog", "update", "--plan", "initial.yaml"]);
    run(&[
        "catalog",
        "update",
        "--apply",
        "initial.yaml",
        "--allow-incomplete-impact",
    ]);
    // A durable accepted snapshot exists, but its marker is damaged. Selection and
    // recovery for the unrelated local bundle must not require reading that snapshot.
    let snapshots = dir.path().join("cache/snapshots");
    let repo = fs::read_dir(snapshots)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let snapshot = fs::read_dir(repo)
        .unwrap()
        .map(Result::unwrap)
        .map(|entry| entry.path())
        .find(|path| path.is_dir())
        .unwrap();
    let marker = snapshot.join("snapshot.yaml");
    let marker_contents = fs::read(&marker).unwrap();
    fs::set_permissions(&marker, fs::Permissions::from_mode(0o600)).unwrap();
    fs::write(&marker, "corrupted: [").unwrap();
    fs::create_dir(dir.path().join("a/.okf-transaction")).unwrap();
    let mut selected = command(dir.path());
    let output = selected
        .env("OKF_CACHE_DIR", dir.path().join("cache"))
        .args(["bundle", "select", "--bundle-id", "demo.a", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let local_token = String::from_utf8(output)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .find(|row| row["kind"] == "bundle-selection")
        .unwrap()["token"]
        .as_str()
        .unwrap()
        .to_owned();
    command(dir.path())
        .env("OKF_CACHE_DIR", dir.path().join("cache"))
        .args(["--selection", &local_token, "changeset", "recover"])
        .assert()
        .success();
    command(dir.path())
        .env("OKF_CACHE_DIR", dir.path().join("cache"))
        .args(["--selection", &local_token, "bundle", "context"])
        .assert()
        .failure();
    fs::write(&marker, marker_contents).unwrap();
    fs::set_permissions(&marker, fs::Permissions::from_mode(0o444)).unwrap();
    fs::write(
        dir.path().join("overrides.yaml"),
        format!(
            "bundles:\n  demo.remote: {}\n",
            upstream.join("bundle").display()
        ),
    )
    .unwrap();
    let catalog = dir.path().join("okf-catalog.yaml");
    let mut select_cmd = command(dir.path());
    let output = select_cmd
        .env("OKF_CATALOG_OVERRIDES", dir.path().join("overrides.yaml"))
        .env("OKF_CACHE_DIR", dir.path().join("cache"))
        .args([
            "bundle",
            "select",
            "--catalog",
            catalog.to_str().unwrap(),
            "--bundle-id",
            "demo.remote",
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let token = String::from_utf8(output)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .find(|row| row["kind"] == "bundle-selection")
        .unwrap()["token"]
        .as_str()
        .unwrap()
        .to_owned();
    let elsewhere = tempfile::tempdir().unwrap();
    command(elsewhere.path())
        .env("OKF_CACHE_DIR", dir.path().join("cache"))
        .args(["--selection", &token, "list"])
        .assert()
        .success();
    command(elsewhere.path())
        .env("OKF_CACHE_DIR", dir.path().join("cache"))
        .args(["--selection", &token, "edit", "n", "--set", "title=Changed"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("read-only"));
    assert!(
        !fs::read_to_string(upstream.join("bundle/n.md"))
            .unwrap()
            .contains("Changed")
    );
    fs::write(dir.path().join("overrides.yaml"), "bundles: {}\n").unwrap();
    command(elsewhere.path())
        .env("OKF_CACHE_DIR", dir.path().join("cache"))
        .args(["--selection", &token, "list"])
        .assert()
        .failure();
}

#[test]
fn selected_changeset_edits_and_moves_with_scoped_ontology_reference() {
    let dir = fixture();
    fs::create_dir(dir.path().join("b")).unwrap();
    fs::write(dir.path().join("okf-catalog.yaml"), "catalog_version: 2\ndefault_bundle: demo.a\nbundles:\n  demo.a: {location: {type: directory, path: a}}\n  demo.b: {location: {type: directory, path: b}}\n").unwrap();
    fs::write(dir.path().join("a/ontology.yaml"), "okf_ontology: '0.1'\nconcepts:\n  Task:\n    references:\n      concepts:\n        target: ['demo.b:Component']\n        cardinality: 0..n\n").unwrap();
    fs::write(
        dir.path().join("b/ontology.yaml"),
        "okf_ontology: '0.1'\nconcepts:\n  Component: {}\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("a/old.md"),
        "---\ntype: Task\nconcepts: ['okf://demo.b/target.md']\n---\nOld body\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("b/target.md"),
        "---\ntype: Component\n---\nTarget\n",
    )
    .unwrap();
    let original = fs::read(dir.path().join("b/target.md")).unwrap();
    let token = select(dir.path(), &["--bundle-id", "demo.a"]);
    let payload = serde_json::json!({"version":1,"operations":[
        {"op":"edit", "concept":"/old", "body":"Updated [target](okf://demo.b/target.md)."},
        {"op":"move", "from":"old", "to":"nested/moved"}
    ]})
    .to_string();
    let args = [
        "--selection",
        &token,
        "changeset",
        "plan",
        "--json",
        "--from",
        &payload,
        "--scope-bundle",
        "demo.b",
        "--require-ontology-gate",
    ];
    let output = command(dir.path())
        .args(args)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let digest = String::from_utf8(output)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .find(|row| row["kind"] == "changeset-summary")
        .unwrap()["base_digest"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(dir.path().join("a/old.md").exists());
    let elsewhere = tempfile::tempdir().unwrap();
    fs::write(elsewhere.path().join("okf.toml"), "bad = [").unwrap();
    command(elsewhere.path())
        .args([
            "--selection",
            &token,
            "changeset",
            "apply",
            "--from",
            &payload,
            "--scope-bundle",
            "demo.b",
            "--require-ontology-gate",
            "--expect",
            &digest,
        ])
        .assert()
        .success();
    assert!(!dir.path().join("a/old.md").exists());
    let moved = fs::read_to_string(dir.path().join("a/nested/moved.md")).unwrap();
    assert!(
        moved.contains("Updated [target](okf://demo.b/target.md)"),
        "{moved}"
    );
    assert_eq!(fs::read(dir.path().join("b/target.md")).unwrap(), original);
}

#[test]
fn explicit_selector_overrides_environment_but_environment_selector_cannot_drift() {
    let dir = fixture();
    fs::create_dir(dir.path().join("b")).unwrap();
    fs::write(dir.path().join("okf-catalog.yaml"), "catalog_version: 2\nbundles:\n  demo.a: {location: {type: directory, path: a}}\n  demo.b: {location: {type: directory, path: b}}\n").unwrap();
    let extract = |output: Vec<u8>| -> String {
        String::from_utf8(output)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
            .find(|row| row["kind"] == "bundle-selection")
            .unwrap()["token"]
            .as_str()
            .unwrap()
            .to_owned()
    };
    let explicit = extract(
        command(dir.path())
            .env("OKF_BUNDLE", "b")
            .args(["bundle", "select", "--bundle-id", "demo.a", "--json"])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone(),
    );
    command(dir.path())
        .env("OKF_BUNDLE", "b")
        .args(["--selection", &explicit, "list"])
        .assert()
        .success();
    let env_token = extract(
        command(dir.path())
            .env("OKF_BUNDLE", "a")
            .args(["bundle", "select", "--json"])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone(),
    );
    command(dir.path())
        .env("OKF_BUNDLE", "b")
        .args(["--selection", &env_token, "list"])
        .assert()
        .failure();
    command(dir.path())
        .args(["--selection", &env_token, "list"])
        .assert()
        .success();
}

#[test]
fn selected_doctor_cannot_run_config_migration() {
    let dir = fixture();
    let token = select(dir.path(), &["--bundle-id", "demo.a"]);
    fs::write(dir.path().join("old.toml"), "bundle = 'a'\n").unwrap();
    for args in [
        vec!["doctor", "--migrate-config", "old.toml"],
        vec![
            "doctor",
            "--migrate-config",
            "old.toml",
            "--migration-plan",
            "plan.yaml",
        ],
    ] {
        command(dir.path())
            .args(["--selection", &token])
            .args(args)
            .assert()
            .failure();
    }
    assert!(!dir.path().join("plan.yaml").exists());
}

#[test]
fn relative_environment_paths_are_bound_to_selection_origin() {
    let dir = fixture();
    fs::write(dir.path().join("overrides.yaml"), "bundles: {demo.a: a}\n").unwrap();
    let output = command(dir.path())
        .env("OKF_BUNDLE", "a")
        .env("OKF_CATALOG", "okf-catalog.yaml")
        .env("OKF_CATALOG_OVERRIDES", "overrides.yaml")
        .args(["bundle", "select", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let token = String::from_utf8(output)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .find(|row| row["kind"] == "bundle-selection")
        .unwrap()["token"]
        .as_str()
        .unwrap()
        .to_owned();
    let elsewhere = tempfile::tempdir().unwrap();
    command(elsewhere.path())
        .env("OKF_BUNDLE", "a")
        .env("OKF_CATALOG", "okf-catalog.yaml")
        .env("OKF_CATALOG_OVERRIDES", "overrides.yaml")
        .args(["--selection", &token, "list"])
        .assert()
        .success();
    command(elsewhere.path())
        .env("OKF_BUNDLE", "other")
        .env("OKF_CATALOG", "okf-catalog.yaml")
        .env("OKF_CATALOG_OVERRIDES", "overrides.yaml")
        .args(["--selection", &token, "list"])
        .assert()
        .failure();
    command(elsewhere.path())
        .env("OKF_BUNDLE", "a")
        .env("OKF_CATALOG", "other.yaml")
        .env("OKF_CATALOG_OVERRIDES", "overrides.yaml")
        .args(["--selection", &token, "list"])
        .assert()
        .failure();
    command(elsewhere.path())
        .env("OKF_BUNDLE", "a")
        .env("OKF_CATALOG", "okf-catalog.yaml")
        .env("OKF_CATALOG_OVERRIDES", "other.yaml")
        .args(["--selection", &token, "list"])
        .assert()
        .failure();
}
