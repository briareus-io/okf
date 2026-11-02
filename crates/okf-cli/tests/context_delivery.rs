//! Context receipts, progressive delivery, and explicit settings projection.
use assert_cmd::prelude::*;
use serde_json::Value;
use std::{
    path::Path,
    process::{Command, Output},
};

fn write(root: &Path, name: &str, text: &str) {
    let path = root.join(name);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}
fn run(root: &Path, args: &[&str]) -> Output {
    Command::cargo_bin("okf")
        .unwrap()
        .current_dir(root)
        .env_remove("OKF_BUNDLE")
        .env_remove("OKF_CATALOG")
        .env_remove("OKF_CATALOG_OVERRIDES")
        .args(args)
        .output()
        .unwrap()
}
fn rows(out: &Output) -> Vec<Value> {
    assert!(
        out.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect()
}
fn record<'a>(rows: &'a [Value], kind: &str) -> &'a Value {
    rows.iter().find(|r| r["kind"] == kind).unwrap()
}
fn token(root: &Path, extra: &[&str]) -> String {
    let mut args = vec![
        "bundle",
        "context",
        ".",
        "--operation",
        "edit",
        "--operation",
        "show",
        "--token",
        "--json",
    ];
    args.extend(extra);
    record(&rows(&run(root, &args)), "context-token")["token"]
        .as_str()
        .unwrap()
        .into()
}
fn fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "okf-bundle.yaml", "manifest_version: 1\nguidance:\n  instructions_file: .okf-bundle/global.md\n  provenance: {generator: example}\n  rules:\n  - id: model\n    operations: [edit]\n    instructions: Preserve ownership.\n  - id: details\n    tier: reference\n    description: Extended modeling procedures.\n    operations: [edit]\n    instructions_file: .okf-bundle/details.md\n  - id: create\n    operations: [add]\n    instructions: Pick a stable ID.\nworkflows:\n  review:\n    description: Review support\n    instructions: Inspect the supporting evidence.\n    resources: [.okf-bundle/support.txt]\n");
    write(
        dir.path(),
        ".okf-bundle/global.md",
        "<!-- Authored comment remains intact. -->\nGlobal guidance.\n",
    );
    write(
        dir.path(),
        ".okf-bundle/details.md",
        "Optional long discussion.\n",
    );
    write(dir.path(), ".okf-bundle/support.txt", "first");
    write(
        dir.path(),
        "note.md",
        "---\ntype: Note\n---\nOld wording.\n",
    );
    dir
}
#[test]
fn delivery_explains_selection_and_preserves_authored_content() {
    let dir = fixture();
    let data = rows(&run(
        dir.path(),
        &[
            "bundle",
            "context",
            ".",
            "--operation",
            "edit",
            "--stats",
            "--guidance-budget",
            "1",
            "--json",
        ],
    ));
    let global = data.iter().find(|r| r["id"] == "$global").unwrap();
    assert!(global["instructions"].as_str().unwrap().starts_with("<!--"));
    assert_eq!(global["origin"]["provenance"]["generator"], "example");
    let reference = data.iter().find(|r| r["id"] == "details").unwrap();
    assert_eq!(reference["content"], "summary");
    assert_eq!(reference["omission_reason"], "reference-on-demand");
    assert_eq!(reference["instruction_role"], "inspected");
    let expand: Vec<_> = reference["expand_argv"]
        .as_array()
        .unwrap()
        .iter()
        .skip(1)
        .map(|v| v.as_str().unwrap())
        .collect();
    let expanded = rows(&run(dir.path(), &expand));
    let detail = expanded.iter().find(|r| r["id"] == "details").unwrap();
    assert_eq!(detail["instructions"], "Optional long discussion.\n");
    assert_eq!(detail["instruction_role"], "inspected");
    assert_eq!(record(&data, "context-stats")["budget_exceeded"], true);
    let expected: usize = data
        .iter()
        .filter_map(|r| r["instructions"].as_str())
        .map(str::len)
        .sum();
    assert_eq!(
        record(&data, "context-stats")["delivered_instruction_bytes"],
        expected
    );
    let out = run(
        dir.path(),
        &["bundle", "context", ".", "--operation", "edit"],
    );
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    for expected in [
        "manifest operation rule: edit",
        "reference-on-demand",
        "outside-selection",
        "inspected",
        "expand:",
        "all applicable mandatory instructions delivered",
    ] {
        assert!(text.contains(expected), "{expected}: {text}");
    }
    assert!(!text.contains("unavailable: "));
    assert!(!text.contains("ontology: null"));
    assert!(!text.contains("query_presets: {}"));
}
#[test]
fn reference_files_are_loaded_only_when_expanded_and_existing_rules_stay_mandatory() {
    let dir = fixture();
    std::fs::remove_file(dir.path().join(".okf-bundle/details.md")).unwrap();
    let data = rows(&run(
        dir.path(),
        &["bundle", "context", ".", "--operation", "edit", "--json"],
    ));
    let rule = data.iter().find(|r| r["id"] == "model").unwrap();
    assert_eq!(rule["tier"], "mandatory");
    assert_eq!(rule["content"], "full");
    assert!(
        !run(dir.path(), &["bundle", "context", ".", "--rule", "details"])
            .status
            .success()
    );
    assert!(
        !run(dir.path(), &["bundle", "context", ".", "--rule", "unknown"])
            .status
            .success()
    );
}
#[test]
fn compact_settings_preserve_state_and_union_relevant_groups() {
    let dir = fixture();
    let manifest_path = dir.path().join("okf-bundle.yaml");
    let manifest = std::fs::read_to_string(&manifest_path).unwrap();
    std::fs::write(
        &manifest_path,
        format!("{manifest}settings:\n  query: {{scan_limit: 25}}\n"),
    )
    .unwrap();
    let relationships = rows(&run(
        dir.path(),
        &[
            "bundle",
            "context",
            ".",
            "--operation",
            "backlinks",
            "--settings-detail",
            "compact",
            "--json",
        ],
    ));
    assert_eq!(
        record(&relationships, "bundle-context")["effective_settings"]["settings"]["query"]
            ["scan_limit"],
        25
    );

    let full = rows(&run(
        dir.path(),
        &["bundle", "context", ".", "--operation", "edit", "--json"],
    ));
    let compact = rows(&run(
        dir.path(),
        &[
            "bundle",
            "context",
            ".",
            "--operation",
            "edit",
            "--settings-detail",
            "compact",
            "--json",
        ],
    ));
    let a = record(&full, "bundle-context");
    let b = record(&compact, "bundle-context");
    assert!(a["effective_settings"]["settings"].get("lint").is_some());
    assert!(b["effective_settings"]["settings"].get("lint").is_none());
    assert_eq!(
        b["effective_settings"]["settings"]["write_policy"]["ontology_gate"],
        false
    );
    assert!(b["effective_settings"]["settings"]["formatting"].is_object());
    assert!(b["setting_origins"].get("lint.broken_link").is_none());
    assert_eq!(a["context_digest"], b["context_digest"]);
    assert_eq!(a["state_digest"], b["state_digest"]);
    let combined = rows(&run(
        dir.path(),
        &[
            "bundle",
            "context",
            ".",
            "--operation",
            "edit",
            "--operation",
            "list",
            "--settings-detail",
            "compact",
            "--json",
        ],
    ));
    let settings = &record(&combined, "bundle-context")["effective_settings"]["settings"];
    assert!(settings["write_policy"].is_object());
    assert!(settings["facets"].is_object());
    let all = rows(&run(
        dir.path(),
        &[
            "bundle",
            "context",
            ".",
            "--all",
            "--settings-detail",
            "compact",
            "--json",
        ],
    ));
    assert!(all
        .iter()
        .filter(|r| r["kind"] == "guidance")
        .all(|r| r["instruction_role"] == "inspected"));
    assert!(!run(
        dir.path(),
        &["bundle", "context", ".", "--all", "--operation", "edit"]
    )
    .status
    .success());
}
#[test]
fn tokens_allow_subset_operations_and_concept_edits_but_refuse_stale_guidance_before_writes() {
    let dir = fixture();
    let receipt = token(dir.path(), &[]);
    let out = run(
        dir.path(),
        &[
            "edit",
            "note",
            ".",
            "--replace",
            "Old",
            "New",
            "--context",
            &receipt,
            "--json",
        ],
    );
    rows(&out);
    rows(&run(
        dir.path(),
        &["show", "note", ".", "--context", &receipt, "--json"],
    ));
    write(dir.path(), "token.txt", &receipt);
    rows(&run(
        dir.path(),
        &[
            "bundle",
            "context",
            ".",
            "--check-token",
            "@token.txt",
            "--operation",
            "edit",
            "--json",
        ],
    ));
    let historical = run(
        dir.path(),
        &["graph", ".", "--revision", "HEAD", "--context", &receipt],
    );
    assert!(!historical.status.success());
    assert!(String::from_utf8_lossy(&historical.stderr).contains("historical inspection"));
    let before = std::fs::read(dir.path().join("note.md")).unwrap();
    write(dir.path(), ".okf-bundle/global.md", "Changed rule.");
    let stale = run(
        dir.path(),
        &[
            "edit",
            "note",
            ".",
            "--replace",
            "New",
            "Broken",
            "--context",
            &receipt,
            "--json",
        ],
    );
    assert!(!stale.status.success());
    assert!(String::from_utf8_lossy(&stale.stderr).contains("instruction:.okf-bundle/global.md"));
    assert_eq!(std::fs::read(dir.path().join("note.md")).unwrap(), before);
    let terminal: Value = serde_json::from_str(
        String::from_utf8_lossy(&stale.stdout)
            .lines()
            .last()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(terminal["publication"], "not-published");
    assert!(terminal["reasons"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v == "context-invalid"));
    // Ordinary use remains available without opting into receipt checking.
    rows(&run(
        dir.path(),
        &["edit", "note", ".", "--replace", "New", "Current", "--json"],
    ));
}
#[test]
fn receipts_bind_root_operations_resources_and_ontology() {
    let dir = fixture();
    let other = fixture();
    let receipt = token(dir.path(), &["--workflow", "review"]);
    assert!(
        !run(other.path(), &["show", "note", ".", "--context", &receipt])
            .status
            .success()
    );
    let missing = run(dir.path(), &["rm", "note", ".", "--context", &receipt]);
    assert!(!missing.status.success());
    assert!(String::from_utf8_lossy(&missing.stderr).contains("was not covered"));
    assert!(dir.path().join("note.md").exists());
    write(dir.path(), ".okf-bundle/support.txt", "second");
    let changed = run(
        dir.path(),
        &["bundle", "context", ".", "--check-token", &receipt],
    );
    assert!(!changed.status.success());
    assert!(String::from_utf8_lossy(&changed.stderr).contains("resource:.okf-bundle/support.txt"));
    let receipt = token(dir.path(), &[]);
    write(
        dir.path(),
        "ontology.yaml",
        "okf_ontology: '0.1'\nconcepts: {}\n",
    );
    assert!(!run(
        dir.path(),
        &["bundle", "context", ".", "--check-token", &receipt]
    )
    .status
    .success());
}
#[test]
fn cannot_issue_execution_tokens_for_inspection_or_incomplete_workflows() {
    let dir = fixture();
    assert!(
        !run(dir.path(), &["bundle", "context", ".", "--all", "--token"])
            .status
            .success()
    );
    write(dir.path(), "okf-bundle.yaml", "manifest_version: 1\nworkflows:\n  import:\n    description: Import\n    instructions: Import the supplied source.\n    inputs:\n      source: {type: string, required: true}\n");
    let out = run(
        dir.path(),
        &[
            "bundle",
            "context",
            ".",
            "--workflow",
            "import",
            "--token",
            "--json",
        ],
    );
    assert!(!out.status.success());
    assert!(!String::from_utf8_lossy(&out.stdout).contains("\"kind\":\"context-token\""));
    assert!(
        !run(dir.path(), &["show", "note", ".", "--context", "malformed"])
            .status
            .success()
    );
    assert!(!run(dir.path(), &["version", "--context", "malformed"])
        .status
        .success());
}
#[test]
fn scoped_commands_require_receipts_for_each_examined_bundle() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "okf-catalog.yaml", "catalog_version: 2\ndefault_bundle: example.a\nbundles:\n  example.a:\n    location: {type: directory, path: a}\n  example.b:\n    location: {type: directory, path: b}\n");
    for bundle in ["a", "b"] {
        write(
            dir.path(),
            &format!("{bundle}/okf-bundle.yaml"),
            "manifest_version: 1\n",
        );
        write(
            dir.path(),
            &format!("{bundle}/note.md"),
            "---\ntype: Note\n---\n",
        );
    }
    let data = rows(&run(
        dir.path(),
        &[
            "bundle",
            "context",
            "--bundle-id",
            "example.a",
            "--scope-bundle",
            "example.b",
            "--operation",
            "list",
            "--token",
            "--json",
        ],
    ));
    let tokens: Vec<_> = data
        .iter()
        .filter(|r| r["kind"] == "context-token")
        .map(|r| r["token"].as_str().unwrap())
        .collect();
    assert_eq!(tokens.len(), 2);
    assert!(!run(
        dir.path(),
        &[
            "list",
            "--bundle-id",
            "example.a",
            "--scope-bundle",
            "example.b",
            "--context",
            tokens[0]
        ]
    )
    .status
    .success());
    rows(&run(
        dir.path(),
        &[
            "list",
            "--bundle-id",
            "example.a",
            "--scope-bundle",
            "example.b",
            "--context",
            tokens[0],
            "--context",
            tokens[1],
            "--json",
        ],
    ));
}
