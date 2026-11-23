//! Run the feedback lifecycle against an isolated user profile.
use serde_json::{Value, json};
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn run(home: &Path, args: &[&str]) -> Output {
    command(home).args(args).output().unwrap()
}

fn command(home: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_okf"));
    cmd.current_dir(home)
        .env("HOME", home)
        .env("LOCALAPPDATA", home)
        .env("XDG_STATE_HOME", home)
        .env_remove("OKF_BUNDLE")
        .env_remove("OKF_CATALOG")
        .env_remove("OKF_FEEDBACK_DIR");
    cmd
}

fn data(home: &Path, args: &[&str]) -> Value {
    let result = run(home, args);
    assert!(
        result.status.success(),
        "{:?}",
        String::from_utf8_lossy(&result.stderr)
    );
    let rows: Vec<Value> = String::from_utf8(result.stdout)
        .unwrap()
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect();
    assert_eq!(rows[0]["kind"], "operation-start");
    assert_eq!(rows.last().unwrap()["kind"], "operation-complete");
    rows[1]["data"].clone()
}

#[test]
fn feedback_directory_override_uses_the_exact_path() {
    let home = tempfile::tempdir().unwrap();
    let root = home.path().join("custom feedback");
    let mut cmd = command(home.path());
    let result = cmd
        .env("OKF_FEEDBACK_DIR", &root)
        .args(["learn", "draft", "workflow-lesson"])
        .output()
        .unwrap();
    assert!(result.status.success());
    assert_eq!(root.join("drafts").read_dir().unwrap().count(), 1);
    let status = data(home.path(), &["learn", "status", "--json"]);
    assert!(!Path::new(status["directory"].as_str().unwrap()).exists());
    for invalid in ["", "relative"] {
        let result = command(home.path())
            .env("OKF_FEEDBACK_DIR", invalid)
            .args(["learn", "status"])
            .output()
            .unwrap();
        assert!(!result.status.success());
        assert!(String::from_utf8_lossy(&result.stderr).contains("OKF_FEEDBACK_DIR"));
    }
}

#[test]
fn manual_request_and_reviewed_export_are_independent_of_collection() {
    let home = tempfile::tempdir().unwrap();
    let home = home.path();
    let status = data(home, &["learn", "status", "--json"]);
    let root = Path::new(status["directory"].as_str().unwrap());
    assert!(!root.exists());
    assert!(!run(home, &["learn", "enable"]).status.success());
    assert!(!root.exists());
    let draft = data(home, &["learn", "draft", "feature-request", "--json"]);
    let path = Path::new(draft["file"].as_str().unwrap());
    assert_eq!(fs::read_to_string(path).unwrap(), draft["text"]);
    let name = path.file_name().unwrap().to_str().unwrap();
    let args = ["learn", "export", name, "--output", "reviewed.md", "--json"];
    let preview = data(home, &args);
    assert!(!home.join("reviewed.md").exists());
    let mut confirmed = args.to_vec();
    confirmed.extend(["--confirm", preview["confirmation"].as_str().unwrap()]);
    let mut moved = confirmed.clone();
    moved[4] = "another.md";
    assert!(!run(home, &moved).status.success());
    fs::write(path, "A reader needs a stable inventory.\n").unwrap();
    assert!(!run(home, &confirmed).status.success());
    assert!(!home.join("reviewed.md").exists());
    let preview = data(home, &args);
    confirmed[7] = preview["confirmation"].as_str().unwrap();
    data(home, &confirmed);
    assert!(!run(home, &confirmed).status.success());
    data(home, &["learn", "purge", "--json"]);
    assert!(!path.exists());
    assert!(!root.join("state.json").exists());
    assert_eq!(
        fs::read_to_string(home.join("reviewed.md")).unwrap(),
        preview["text"]
    );
}

#[test]
fn enabled_collection_preserves_command_results_and_excludes_input() {
    let home = tempfile::tempdir().unwrap();
    let home = home.path();
    let bundle = home.join("private-sentinel-name");
    fs::create_dir(&bundle).unwrap();
    fs::write(
        bundle.join("private-sentinel.md"),
        "---\ntitle: private-sentinel\ntype: Note\n---\n[one](missing.md) [two](absent.md)\n",
    )
    .unwrap();
    let args = ["lint", bundle.to_str().unwrap(), "--fail-on", "any"];
    let off = run(home, &args);
    let status = data(home, &["learn", "status", "--json"]);
    let root = Path::new(status["directory"].as_str().unwrap());
    assert!(!root.exists());
    data(home, &["learn", "draft", "guidance-gap", "--json"]);
    // Seed the consent that the interactive terminal flow would persist.
    fs::write(
        root.join("state.json"),
        json!({"schema_version":1,"enabled":true,"versions":{}}).to_string(),
    )
    .unwrap();
    let on = run(home, &args);
    assert_eq!(
        (off.status, off.stdout, off.stderr),
        (on.status, on.stdout, on.stderr)
    );
    fs::write(
        bundle.join("invalid.md"),
        "---\ntitle: private-sentinel\n---\n",
    )
    .unwrap();
    assert_eq!(
        run(home, &["validate", bundle.to_str().unwrap()])
            .status
            .code(),
        Some(1)
    );
    let report = data(home, &["learn", "report", "--json"]);
    let patterns = report["patterns"].as_array().unwrap();
    for (family, pattern) in [("lint", "broken-link"), ("validate", "invalid-bundle")] {
        let row = patterns
            .iter()
            .find(|r| r["command"] == family && r["pattern"] == pattern)
            .unwrap();
        assert_eq!(row["affected"], 1);
        assert_eq!(row["invocations"], 1);
    }
    let store = fs::read_to_string(root.join("state.json")).unwrap();
    assert!(!store.contains("sentinel"));
    data(home, &["learn", "disable", "--json"]);
    run(home, &args);
    assert_eq!(
        data(home, &["learn", "report", "--json"])["patterns"],
        report["patterns"]
    );
    fs::write(root.join("state.json"), "invalid data").unwrap();
    assert_eq!(run(home, &args).status.code(), Some(1));
    assert!(!run(home, &["learn", "report"]).status.success());
    data(home, &["learn", "purge", "--json"]);
}

#[test]
fn busy_store_skips_observation_and_parallel_drafts_survive() {
    let profile = tempfile::tempdir().unwrap();
    let home = profile.path();
    let bundle = home.join("bundle");
    fs::create_dir(&bundle).unwrap();
    fs::write(
        bundle.join("example.md"),
        "---\ntype: Note\ntitle: Example\n---\n",
    )
    .unwrap();
    std::thread::scope(|scope| {
        let workers: Vec<_> = (0..8)
            .map(|_| scope.spawn(|| data(home, &["learn", "draft", "workflow-lesson", "--json"])))
            .collect();
        for w in workers {
            w.join().unwrap();
        }
    });
    let status = data(home, &["learn", "status", "--json"]);
    assert_eq!(status["drafts"], 8);
    let root = Path::new(status["directory"].as_str().unwrap());
    let path = root.join("state.json");
    let consent = r#"{"schema_version":1,"enabled":true,"versions":{}}"#;
    fs::write(&path, consent).unwrap();
    let lock = fs::File::options()
        .read(true)
        .write(true)
        .open(root.join("lock"))
        .unwrap();
    lock.lock().unwrap();
    assert!(
        run(home, &["validate", bundle.to_str().unwrap()])
            .status
            .success()
    );
    assert_eq!(fs::read_to_string(&path).unwrap(), consent);
    drop(lock);
    data(home, &["learn", "purge", "--json"]);
    assert!(
        run(home, &["validate", bundle.to_str().unwrap()])
            .status
            .success()
    );
    assert!(!path.exists());
}
