use assert_cmd::prelude::*;
use serde_json::Value;
use std::{fs, path::Path, process::Command};

fn okf(root: &Path, args: &[&str], exit: i32) -> Vec<Value> {
    let output = Command::cargo_bin("okf")
        .unwrap()
        .current_dir(root)
        .env("OKF_CACHE_DIR", root.parent().unwrap().join("cache"))
        .env_remove("OKF_SETTINGS_OVERRIDES")
        .env_remove("OKF_BUNDLE")
        .env_remove("OKF_CATALOG")
        .env_remove("OKF_CATALOG_OVERRIDES")
        .args(args)
        .arg("--json")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(exit), "{args:?}: {output:?}");
    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect()
}
fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .current_dir(root)
        .args([
            "-c",
            "user.email=test@example.invalid",
            "-c",
            "user.name=Test",
        ])
        .args(args)
        .output()
        .unwrap();
    assert!(output.status.success(), "{args:?} {output:?}");
}
fn commit(root: &Path) {
    git(root, &["add", "."]);
    git(root, &["commit", "-qm", "fixture"]);
}
fn comparison(rows: &[Value]) -> Vec<&Value> {
    rows.iter()
        .filter(|r| r["kind"] == "lint-comparison")
        .collect()
}
fn workspace(dir: &tempfile::TempDir) -> std::path::PathBuf {
    let repo = dir.path().join("workspace");
    fs::create_dir(&repo).unwrap();
    git(&repo, &["init", "-qb", "main"]);
    repo
}

#[test]
fn threshold_round_trip_and_ref_parity() {
    let dir = tempfile::tempdir().unwrap();
    let root = workspace(&dir);
    fs::write(
        root.join("a.md"),
        "---\ntype: Note\n---\n[old](missing.md)\n",
    )
    .unwrap();
    commit(&root);
    let baseline = "lint.json";
    let write = ["lint", "--write-baseline", baseline];
    okf(&root, &write, 0);
    let original = fs::read(root.join(baseline)).unwrap();
    okf(&root, &write, 0);
    assert_eq!(original, fs::read(root.join(baseline)).unwrap());
    let read = ["lint", "--baseline", baseline, "--fail-on", "warn"];
    assert!(
        comparison(&okf(&root, &read, 0))
            .iter()
            .all(|r| r["classification"] == "existing")
    );
    fs::write(
        root.join("a.md"),
        "---\ntype: Note\n---\n\n[old](another.md)\n",
    )
    .unwrap();
    fs::write(
        root.join("okf-bundle.yaml"),
        "manifest_version: 1\nsettings: {lint: {missing_description: warn}}\n",
    )
    .unwrap();
    let file = okf(&root, &read, 1);
    let revision = okf(
        &root,
        &["lint", "--baseline-ref", "HEAD", "--fail-on", "warn"],
        1,
    );
    assert_eq!(comparison(&file), comparison(&revision));
    for (state, subject) in [("resolved", "/missing"), ("introduced", "/another")] {
        assert!(
            comparison(&file)
                .iter()
                .any(|r| r["classification"] == state && r["identity"]["subject"] == subject)
        );
    }
    assert!(
        comparison(&file)
            .iter()
            .any(|r| r["classification"] == "worsened"
                && r["previous_severity"] == "info"
                && r["severity"] == "warn")
    );
    okf(
        &root,
        &["lint", "--baseline", baseline, "--fail-on", "never"],
        0,
    );
    assert_eq!(original, fs::read(root.join(baseline)).unwrap());
    okf(&root, &write, 0);
    assert!(
        !fs::read_to_string(root.join(baseline))
            .unwrap()
            .contains("/missing\"")
    );
}

#[test]
fn complete_scope_required_for_capture_and_comparison() {
    let dir = tempfile::tempdir().unwrap();
    let root = workspace(&dir);
    for id in ["a", "b"] {
        fs::create_dir(root.join(id)).unwrap();
        fs::write(root.join(id).join("note.md"), "---\ntype: Note\n---\n").unwrap();
    }
    let catalog = "catalog_version: 2\ndefault_bundle: test.a\nbundles:\n  test.a: {location: {type: directory, path: a}}\n";
    fs::write(root.join("okf-catalog.yaml"), catalog).unwrap();
    commit(&root);
    let whole = ["lint", "--catalog-scope", "--write-baseline", "lint.json"];
    okf(&root, &whole, 0);
    fs::write(
        root.join("okf-catalog.yaml"),
        format!("{catalog}  test.b: {{location: {{type: directory, path: b}}}}\n"),
    )
    .unwrap();
    let current = okf(
        &root,
        &[
            "lint",
            "--baseline",
            "lint.json",
            "--catalog-scope",
            "--fail-on",
            "warn",
        ],
        1,
    );
    let historical = okf(
        &root,
        &[
            "lint",
            "--baseline-ref",
            "HEAD",
            "--catalog-scope",
            "--fail-on",
            "warn",
        ],
        1,
    );
    assert_eq!(comparison(&current), comparison(&historical));
    assert!(
        comparison(&current)
            .iter()
            .any(|r| r["classification"] == "introduced" && r["identity"]["bundle"] == "test.b")
    );
    okf(&root, &["lint", "--baseline", "lint.json"], 2);
    let original = fs::read(root.join("lint.json")).unwrap();
    fs::rename(root.join("b"), root.join("elsewhere")).unwrap();
    let output = okf(&root, &whole, 3);
    assert_eq!(
        output.last().unwrap()["completeness"]["examination"],
        "partial"
    );
    assert_eq!(original, fs::read(root.join("lint.json")).unwrap());
}

#[test]
fn historical_lock_reports_regression_in_untouched_consumer() {
    let dir = tempfile::tempdir().unwrap();
    let root = workspace(&dir);
    let upstream = dir.path().join("upstream");
    fs::create_dir_all(upstream.join("docs")).unwrap();
    git(&upstream, &["init", "-qb", "main"]);
    fs::write(
        upstream.join("docs/index.md"),
        "---\nokf_version: '0.2'\n---\n# Docs\n",
    )
    .unwrap();
    fs::write(
        upstream.join("docs/target.md"),
        "---\ntype: Note\n---\n# Target\n",
    )
    .unwrap();
    commit(&upstream);
    fs::create_dir(root.join("consumer")).unwrap();
    fs::write(
        root.join("consumer/use.md"),
        "---\ntype: Note\n---\n[target](okf://test.dep/target.md)\n",
    )
    .unwrap();
    // Seed a local object store for an HTTPS registration, so the test never needs a server.
    use okf_core::{bundle::remote, fingerprint::canonicalize::sha256_hex};
    let url = "https://fixture.invalid/upstream";
    let catalog = root.join("okf-catalog.yaml");
    fs::write(&catalog,format!("catalog_version: 2\ndefault_bundle: test.consumer\nbundles:\n  test.consumer: {{location: {{type: directory, path: consumer}}}}\n  test.dep:\n    location: {{type: git, url: '{url}', ref: refs/heads/main, path: docs}}\n")).unwrap();
    let store = dir
        .path()
        .join("cache/objects")
        .join(sha256_hex(url.as_bytes()));
    fs::create_dir_all(store.parent().unwrap()).unwrap();
    git(dir.path(), &["init", "--bare", store.to_str().unwrap()]);
    let declarations = remote::registrations(&catalog).unwrap();
    let pin = || {
        git(
            &upstream,
            &["push", store.to_str().unwrap(), "HEAD:refs/heads/main"],
        );
        let head = Command::new("git")
            .current_dir(&upstream)
            .args(["rev-parse", "HEAD"])
            .output()
            .unwrap();
        let oid = std::str::from_utf8(&head.stdout).unwrap().trim();
        let lock = serde_json::json!({"lock_version":1,"declaration_digest":sha256_hex(&serde_json::to_vec(&declarations).unwrap()),"repositories":{"dep":{"url":url,"commit":oid,"object_format":"sha1"}},"bundles":{"test.dep":{"repository":"dep","path":"docs","observed":{"okf_spec":"0.2"}}}});
        fs::write(
            remote::lock_path(&catalog),
            yaml_serde::to_string(&lock).unwrap(),
        )
        .unwrap();
        okf(&root, &["catalog", "sync"], 0);
    };
    pin();
    commit(&root);
    okf(
        &root,
        &["lint", "--catalog-scope", "--write-baseline", "lint.json"],
        0,
    );
    fs::rename(
        upstream.join("docs/target.md"),
        upstream.join("docs/moved.md"),
    )
    .unwrap();
    commit(&upstream);
    pin();
    let historical = okf(
        &root,
        &[
            "lint",
            "--baseline-ref",
            "HEAD",
            "--catalog-scope",
            "--fail-on",
            "warn",
        ],
        1,
    );
    let saved = okf(
        &root,
        &[
            "lint",
            "--baseline",
            "lint.json",
            "--catalog-scope",
            "--fail-on",
            "warn",
        ],
        1,
    );
    assert_eq!(comparison(&historical), comparison(&saved));
    assert!(
        comparison(&saved)
            .iter()
            .any(|r| r["classification"] == "introduced"
                && r["identity"]["bundle"] == "test.consumer"
                && r["identity"]["subject"] == "okf://test.dep/target.md")
    );
    // Removing only the old dependency snapshot must never turn its findings into resolutions.
    let pins = saved
        .iter()
        .find(|r| r["kind"] == "lint-comparison-summary")
        .unwrap();
    let old_pin = pins["before_bundles"]["test.dep"]
        .as_str()
        .unwrap()
        .strip_prefix("git:")
        .unwrap();
    let snapshots = dir
        .path()
        .join("cache/snapshots")
        .join(sha256_hex(url.as_bytes()));
    let old = fs::read_dir(snapshots)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.is_dir() && p.file_name().unwrap().to_string_lossy().contains(old_pin))
        .unwrap();
    fs::rename(old, dir.path().join("offline")).unwrap();
    let unavailable = okf(
        &root,
        &[
            "lint",
            "--baseline-ref",
            "HEAD",
            "--catalog-scope",
            "--fail-on",
            "never",
        ],
        3,
    );
    assert_eq!(
        unavailable.last().unwrap()["completeness"]["examination"],
        "partial"
    );
    assert!(comparison(&unavailable).is_empty());
}

#[test]
fn counts_occurrences_and_ignores_message_edits_and_body_line_shifts() {
    let temp = tempfile::tempdir().unwrap();
    let root = workspace(&temp);
    fs::create_dir(root.join("a")).unwrap();
    fs::write(root.join("okf-catalog.yaml"),"catalog_version: 2\ndefault_bundle: test.a\nbundles:\n  test.a: {location: {type: directory, path: a}}\n").unwrap();
    let note = root.join("a/test.md");
    fs::write(&note, "---\ntype: Note\n---\n[first](missing.md)\n").unwrap();
    okf(&root, &["lint", "--write-baseline", "baseline.json"], 0);
    let path = root.join("baseline.json");
    let mut baseline: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    for row in baseline["findings"].as_array_mut().unwrap() {
        row["message"] = "earlier CLI wording".into();
    }
    fs::write(&path, serde_json::to_vec(&baseline).unwrap()).unwrap();
    fs::write(&note, "---\ntype: Note\n---\n\n\n[first](missing.md)\n").unwrap();
    let read = ["lint", "--baseline", "baseline.json"];
    okf(&root, &read, 0);
    fs::write(
        &note,
        "---\ntype: Note\n---\n\n[first](missing.md)\n[second](missing.md)\n",
    )
    .unwrap();
    let out = okf(&root, &read, 1);
    assert!(
        comparison(&out)
            .iter()
            .any(|r| r["classification"] == "introduced"
                && r["identity"]["subject"] == "missing.md"
                && r["count"] == 1)
    );
}

#[test]
fn merge_base_ignores_independent_main_fix_and_incomplete_capture_preserves_baseline() {
    let temp = tempfile::tempdir().unwrap();
    let root = workspace(&temp);
    let note = root.join("a.md");
    fs::write(&note, "---\ntype: Note\n---\n[missing](gone.md)\n").unwrap();
    commit(&root);
    git(&root, &["branch", "feature"]);
    fs::write(&note, "---\ntype: Note\n---\nFixed\n").unwrap();
    commit(&root);
    git(&root, &["checkout", "-q", "feature"]);
    let base = Command::new("git")
        .current_dir(&root)
        .args(["merge-base", "HEAD", "main"])
        .output()
        .unwrap();
    let base = std::str::from_utf8(&base.stdout).unwrap().trim();
    okf(&root, &["lint", "--baseline-ref", base], 0);
    okf(&root, &["lint", "--baseline-ref", "main"], 1);
    okf(
        &root,
        &[
            "lint",
            "--baseline-ref",
            "unknown-revision",
            "--fail-on",
            "never",
        ],
        3,
    );
    let capture = [
        "lint",
        "--write-baseline",
        "lint.json",
        "--fail-on",
        "never",
    ];
    okf(&root, &capture, 0);
    let saved = fs::read(root.join("lint.json")).unwrap();
    fs::write(root.join("ontology.yaml"),"okf_ontology: '0.1'\nconcepts:\n  Note:\n    fields:\n      a: {type: bool, required: true}\n      b: {type: bool, required: true}\n").unwrap();
    fs::write(
        root.join("okf-bundle.yaml"),
        "manifest_version: 1\nsettings: {lint: {finding_budget: 1}}\n",
    )
    .unwrap();
    let output = okf(&root, &capture, 3);
    assert_eq!(
        output.last().unwrap()["completeness"]["examination"],
        "partial"
    );
    assert_eq!(fs::read(root.join("lint.json")).unwrap(), saved);
}
