//! End-to-end local-object history test; no remote server or fetch is involved.
use okf_core::bundle::{
    catalog::load_catalog,
    context::{Context, SelectedBundle},
    snapshot_context::{prepare, SnapshotSelection},
};
use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

fn git(root: &Path, args: &[&str]) -> String {
    let result = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    String::from_utf8(result.stdout).unwrap().trim().into()
}
#[test]
fn workspace_history_reads_historical_config_and_bytes_from_an_object_snapshot() {
    let repo = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let prior = std::env::var_os("OKF_CACHE_DIR");
    std::env::set_var("OKF_CACHE_DIR", cache.path());
    git(repo.path(), &["init", "-q"]);
    std::fs::create_dir(repo.path().join("docs")).unwrap();
    std::fs::write(repo.path().join("okf-catalog.yaml"),"catalog_version: 2\ndefault_bundle: acme.docs\nbundles:\n  acme.docs:\n    location: {type: directory, path: docs}\n").unwrap();
    std::fs::write(
        repo.path().join("docs/okf-bundle.yaml"),
        "manifest_version: 1\nguidance:\n  instructions: old instructions\n",
    )
    .unwrap();
    std::fs::write(
        repo.path().join("docs/note.md"),
        "---\ntype: Note\n---\nOld evidence\n",
    )
    .unwrap();
    std::fs::create_dir(repo.path().join("sibling")).unwrap();
    std::fs::write(
        repo.path().join("sibling/private.md"),
        "---\ntype: Note\n---\nOutside selected bundle\n",
    )
    .unwrap();
    git(repo.path(), &["add", "."]);
    git(
        repo.path(),
        &[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.test",
            "commit",
            "-qm",
            "old",
        ],
    );
    let old = git(repo.path(), &["rev-parse", "HEAD"]);
    std::fs::write(
        repo.path().join("docs/note.md"),
        "---\ntype: Note\n---\nCurrent evidence\n",
    )
    .unwrap();
    std::fs::write(
        repo.path().join("docs/okf-bundle.yaml"),
        "manifest_version: 1\nguidance:\n  instructions: current instructions\n",
    )
    .unwrap();
    let catalog = load_catalog(&repo.path().join("okf-catalog.yaml"), None).unwrap();
    let current = Context {
        primary: SelectedBundle {
            id: Some("acme.docs".into()),
            root: repo.path().join("docs"),
        },
        catalog: Some(catalog),
        config: Default::default(),
        config_path: None,
    };
    let snapshot = prepare(
        &current,
        SnapshotSelection {
            workspace_revision: Some(old.clone()),
            bundle_revisions: BTreeMap::new(),
        },
        &[],
    )
    .unwrap();
    assert!(snapshot.context.primary.root.starts_with(cache.path()));
    assert!(
        std::fs::read_to_string(snapshot.context.primary.root.join("note.md"))
            .unwrap()
            .contains("Old evidence")
    );
    assert!(
        std::fs::read_to_string(snapshot.context.primary.root.join("okf-bundle.yaml"))
            .unwrap()
            .contains("old instructions")
    );
    assert_eq!(snapshot.evidence[0].commit.as_deref(), Some(old.as_str()));
    // A managed snapshot represents the repository, but this query must retain
    // the selected bundle subpath and cannot expose sibling concepts.
    let (historical, _, count) =
        okf_core::graph::catalog::load_revision_limited(&snapshot.context.primary.root, &old, None)
            .unwrap()
            .unwrap();
    assert_eq!(count, 1);
    assert_eq!(historical.concepts[0].id.0, "/note");
    let _guard = snapshot.install();
    let resolved = okf_core::bundle::context::current_context(
        Some(current.primary.root.to_str().unwrap()),
        None,
    )
    .unwrap();
    assert_eq!(resolved.primary.root, snapshot.context.primary.root);
    if let Some(prior) = prior {
        std::env::set_var("OKF_CACHE_DIR", prior);
    } else {
        std::env::remove_var("OKF_CACHE_DIR");
    }
}
