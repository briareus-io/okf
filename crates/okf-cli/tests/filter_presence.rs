//! Presence-aware query unions retain their complete machine-readable evidence.
use serde_json::Value;
use std::{fs, process::Command};

#[test]
fn search_unions_absent_status_with_nonimplemented_and_reports_full_filter() {
    let directory = tempfile::tempdir().unwrap();
    for (name, status) in [
        ("absent", ""),
        ("proposed", "impl_status: proposed\n"),
        ("implemented", "impl_status: implemented\n"),
        ("null", "impl_status: null\n"),
    ] {
        fs::write(
            directory.path().join(format!("{name}.md")),
            format!("---\ntype: Note\ntitle: {name}\n{status}---\nBody\n"),
        )
        .unwrap();
    }
    let expression = "impl_status not in [\"implemented\"] OR impl_status absent";
    let output = Command::new(env!("CARGO_BIN_EXE_okf"))
        .current_dir(directory.path())
        .env_remove("OKF_BUNDLE")
        .env_remove("OKF_CATALOG_OVERRIDES")
        .args([
            "search",
            ".",
            "--json",
            "--full-scan",
            "--facet-filter",
            expression,
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let rows: Vec<Value> = std::str::from_utf8(&output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let mut ids: Vec<_> = rows.iter().filter_map(|row| row["id"].as_str()).collect();
    ids.sort();
    assert_eq!(ids, vec!["/absent", "/proposed"]);
    let summary = rows
        .iter()
        .find(|row| row["kind"] == "query-summary")
        .unwrap();
    assert_eq!(summary["filters"], serde_json::json!([expression]));
    assert_eq!(summary["total_matches"], 2);
    assert_eq!(summary["scan_complete"], true);
    let completion = rows.last().unwrap();
    assert_eq!(completion["kind"], "operation-complete");
    assert_eq!(completion["outcome"], "ok");
    assert_eq!(completion["completeness"]["matching"], "complete");
}
