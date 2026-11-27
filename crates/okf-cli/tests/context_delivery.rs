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
    write(
        dir.path(),
        "okf-bundle.yaml",
        "manifest_version: 1\nguidance:\n  instructions_file: .okf-bundle/global.md\n  provenance: {generator: example}\n  rules:\n  - id: model\n    operations: [edit]\n    instructions: Preserve ownership.\n  - id: details\n    tier: reference\n    description: Extended modeling procedures.\n    operations: [edit]\n    instructions_file: .okf-bundle/details.md\n  - id: create\n    operations: [add]\n    instructions: Pick a stable ID.\nworkflows:\n  review:\n    description: Review support\n    instructions: Inspect the supporting evidence.\n    resources: [.okf-bundle/support.txt]\n",
    );
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
        "expand arguments:",
        "Expand entries with: okf bundle context",
        "all applicable mandatory instructions delivered",
    ] {
        assert!(text.contains(expected), "{expected}: {text}");
    }
    assert_eq!(text.matches("Expand entries with:").count(), 1);
    assert!(text.contains("expand arguments: --rule 'details'"));
    assert!(text.contains(r#""provenance":{"generator":"example"}"#));
    assert!(text.contains(r#""instructions_file":".okf-bundle/details.md""#));
    assert!(!text.contains("manifest_digest"));
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
        record(&relationships, "bundle-context")["effective_settings"]["settings"]["query"]["scan_limit"],
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
    assert!(
        all.iter()
            .filter(|r| r["kind"] == "guidance")
            .all(|r| r["instruction_role"] == "inspected")
    );
    assert!(
        !run(
            dir.path(),
            &["bundle", "context", ".", "--all", "--operation", "edit"]
        )
        .status
        .success()
    );
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
    assert!(
        terminal["reasons"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == "context-invalid")
    );
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
    assert!(
        !run(
            dir.path(),
            &["bundle", "context", ".", "--check-token", &receipt]
        )
        .status
        .success()
    );
}
#[test]
fn cannot_issue_execution_tokens_for_inspection_or_incomplete_workflows() {
    let dir = fixture();
    assert!(
        !run(dir.path(), &["bundle", "context", ".", "--all", "--token"])
            .status
            .success()
    );
    write(
        dir.path(),
        "okf-bundle.yaml",
        "manifest_version: 1\nworkflows:\n  import:\n    description: Import\n    instructions: Import the supplied source.\n    inputs:\n      source: {type: string, required: true}\n",
    );
    let agent = run(
        dir.path(),
        &[
            "bundle",
            "context",
            ".",
            "--workflow",
            "import",
            "--format",
            "agent",
        ],
    );
    assert!(agent.status.success());
    let text = String::from_utf8(agent.stdout).unwrap();
    assert!(text.contains(r#"Missing inputs: ["source"]"#));
    assert!(text.ends_with("Context: complete\n"));
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
    assert!(
        !run(dir.path(), &["version", "--context", "malformed"])
            .status
            .success()
    );
}
#[test]
fn scoped_commands_require_receipts_for_each_examined_bundle() {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        "okf-catalog.yaml",
        "catalog_version: 2\ndefault_bundle: example.a\nbundles:\n  example.a:\n    location: {type: directory, path: a}\n  example.b:\n    location: {type: directory, path: b}\n",
    );
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
    assert!(
        !run(
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
        .success()
    );
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

#[test]
fn agent_context_delivers_guidance_resources_queries_and_a_checkable_receipt() {
    let dir = fixture();
    let root = dir.path();
    let path = root.join("okf-bundle.yaml");
    let mut manifest = std::fs::read_to_string(&path).unwrap();
    manifest.push_str("settings:\n  saved_queries:\n    owners:\n      from: concept\n      steps: [{links: {}}, {backlinks: {inverse_kind: [owns], related_type: [Person]}}]\n      select: [title]\n");
    std::fs::write(path, manifest).unwrap();
    let base = [
        "bundle",
        "context",
        ".",
        "--operation",
        "edit",
        "--workflow",
        "review",
    ];
    let render = |extra: &[&str]| {
        let out = run(root, &[base.as_slice(), extra].concat());
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8(out.stdout).unwrap()
    };
    let text = render(&["--format", "agent"]);
    for expected in [
        "<!-- Authored comment remains intact. -->\nGlobal guidance.",
        "Preserve ownership.",
        "[available rule details]",
        "[workflow review]",
        "Inspect the supporting evidence.",
        ".okf-bundle/support.txt",
    ] {
        assert!(text.contains(expected), "{text}");
    }
    assert!(text.ends_with("Context: complete\n"));
    assert!(text.len() < 600);
    assert!(!text.contains("[saved query"));
    assert!(!text.contains(root.to_str().unwrap()));
    assert!(render(&[]).contains(".okf-bundle/support.txt"));
    let json = rows(&run(root, &[base.as_slice(), &["--json"]].concat()));
    assert!(
        text.contains(
            record(&json, "bundle-context")["context_digest"]
                .as_str()
                .unwrap()
        )
    );
    let with_token = render(&["--format", "agent", "--token"]);
    let token = with_token
        .lines()
        .find_map(|line| line.strip_prefix("Context token: "))
        .unwrap();
    assert!(
        run(
            root,
            &[
                "bundle",
                "context",
                ".",
                "--operation",
                "edit",
                "--check-token",
                token
            ]
        )
        .status
        .success()
    );
    let query = render(&["--operation", "query", "--format", "agent"]);
    let line = query
        .lines()
        .find(|line| line.starts_with("[saved query owners]"))
        .unwrap();
    assert!(line.contains(r#""steps":[{"links":{}},{"backlinks":{"inverse_kind":["owns"],"related_type":["Person"]}}]"#));
    assert!(line.contains(r#""select":["title"]"#));
    let inspect = render(&["--rule", "details", "--format", "agent"]);
    assert!(inspect.contains("[inspect rule details]\nOptional long discussion."));
}

#[test]
fn workflows_project_the_union_of_declared_query_definitions() {
    let dir = fixture();
    let root = dir.path();
    let manifest = r#"manifest_version: 1
settings:
  saved_queries:
    plants: {from: concept, select: [title]}
    tasks: {from: concept, steps: [{links: {}}]}
    harvests: {from: concept}
workflows:
  log: {description: Log harvest, instructions: Record weight., uses_queries: []}
  prune: {description: Schedule pruning, instructions: Check tasks., uses_queries: [tasks]}
  review: {description: Review bed, instructions: Check plants and tasks., uses_queries: [plants, tasks]}
"#;
    write(root, "okf-bundle.yaml", manifest);
    let base = ["bundle", "context", ".", "--operation", "edit"];
    for (extra, expected) in [
        (vec!["--workflow", "log"], vec![]),
        (vec!["--workflow", "prune"], vec!["tasks"]),
        (
            vec!["--workflow", "prune", "--workflow", "review"],
            vec!["plants", "tasks"],
        ),
        (
            vec!["--workflow", "prune", "--operation", "query"],
            vec!["harvests", "plants", "tasks"],
        ),
    ] {
        let args = [base.as_slice(), &extra].concat();
        for format in [["--format", "agent"], ["--settings-detail", "compact"]] {
            let out = run(root, &[args.as_slice(), &format].concat());
            assert!(
                out.status.success(),
                "{}",
                String::from_utf8_lossy(&out.stderr)
            );
            let text = String::from_utf8(out.stdout).unwrap();
            let names: Vec<_> = text
                .lines()
                .filter_map(|line| line.strip_prefix("[saved query "))
                .map(|line| {
                    let (name, definition) = line.split_once("] ").unwrap();
                    assert_eq!(
                        serde_json::from_str::<Value>(definition).unwrap()["from"],
                        "concept"
                    );
                    name
                })
                .collect();
            assert_eq!(names, expected);
        }
        let compact = rows(&run(
            root,
            &[args.as_slice(), &["--settings-detail", "compact", "--json"]].concat(),
        ));
        if expected.is_empty() {
            let omitted = &record(&compact, "bundle-context")["settings_projection"]["omitted"];
            let reasons: Vec<_> = omitted
                .as_array()
                .unwrap()
                .iter()
                .filter_map(Value::as_str)
                .filter(|reason| reason.starts_with("saved_queries:"))
                .collect();
            assert_eq!(reasons, ["saved_queries: outside selected workflows"]);
        }
        let queries =
            &record(&compact, "bundle-context")["effective_settings"]["settings"]["saved_queries"];
        assert_eq!(
            queries
                .as_object()
                .into_iter()
                .flat_map(|q| q.keys())
                .collect::<Vec<_>>(),
            expected
        );
    }
    let full = rows(&run(
        root,
        &[base.as_slice(), &["--workflow", "prune", "--json"]].concat(),
    ));
    assert_eq!(
        record(&full, "bundle-context")["effective_settings"]["settings"]["saved_queries"]
            .as_object()
            .unwrap()
            .len(),
        3
    );
    let prune = full.iter().find(|row| row["id"] == "prune").unwrap();
    assert_eq!(prune["uses_queries"], serde_json::json!(["tasks"]));

    let receipt = token(root, &["--workflow", "prune"]);
    assert!(
        !run(
            root,
            &[
                "bundle",
                "context",
                ".",
                "--operation",
                "query",
                "--check-token",
                &receipt
            ]
        )
        .status
        .success()
    );

    write(
        root,
        "okf-catalog.yaml",
        "catalog_version: 2\ndefault_bundle: garden.main\nbundles:\n  garden.main: {location: {type: directory, path: main}}\n  garden.other: {location: {type: directory, path: other}}\n",
    );
    write(root, "main/okf-bundle.yaml", manifest);
    write(
        root,
        "other/okf-bundle.yaml",
        "manifest_version: 1\nsettings:\n  saved_queries:\n    unrelated: {from: concept}\n",
    );
    let scoped = run(
        root,
        &[
            "bundle",
            "context",
            "--bundle-id",
            "garden.main",
            "--scope-bundle",
            "garden.other",
            "--workflow",
            "prune",
            "--format",
            "agent",
        ],
    );
    assert!(
        scoped.status.success(),
        "{}",
        String::from_utf8_lossy(&scoped.stderr)
    );
    let text = String::from_utf8(scoped.stdout).unwrap();
    assert!(text.contains("[saved query tasks]"));
    assert!(!text.contains("[saved query unrelated]"));

    write(
        root,
        "okf-bundle.yaml",
        &manifest.replace("uses_queries: [tasks]", "uses_queries: [typo]"),
    );
    let validate = run(root, &["bundle", "validate", "."]);
    assert!(!validate.status.success());
    assert!(
        format!(
            "{}{}",
            String::from_utf8_lossy(&validate.stderr),
            String::from_utf8_lossy(&validate.stdout)
        )
        .contains("workflow \"prune\" uses unknown saved query \"typo\"")
    );
}

#[test]
fn effective_query_text_preserves_filters_without_root_dependent_framing() {
    let dir = tempfile::tempdir().unwrap();
    let manifest = "manifest_version: 1\nsettings:\n  query_presets:\n    plants: {operations: [list], query: {type: Plant}}\n    detailed-links: {operations: [links], details: true}\n";
    for name in ["garden".to_owned(), "garden".repeat(25)] {
        let root = dir.path().join(name);
        write(&root, "okf-bundle.yaml", manifest);
        for (op, input, expected) in [
            (
                "list",
                "{}",
                serde_json::json!({"operation":"list","query":{}}),
            ),
            (
                "list",
                r#"{"preset":"plants","tag":"food"}"#,
                serde_json::json!({"operation":"list","query":{"type":"Plant","tag":"food"}}),
            ),
            (
                "links",
                r#"{"related_type":["Plant"],"preset":"detailed-links"}"#,
                serde_json::json!({"operation":"links","filters":{"related_type":["Plant"]},"details":true}),
            ),
        ] {
            let out = run(
                &root,
                &[
                    "bundle",
                    "context",
                    ".",
                    "--operation",
                    op,
                    "--query",
                    input,
                    "--format",
                    "agent",
                ],
            );
            assert!(
                out.status.success(),
                "{}",
                String::from_utf8_lossy(&out.stderr)
            );
            let text = String::from_utf8(out.stdout).unwrap();
            let query = text
                .lines()
                .find_map(|s| s.strip_prefix("Effective query: "))
                .unwrap();
            assert_eq!(serde_json::from_str::<Value>(query).unwrap(), expected);
            assert!(!text.contains(root.to_str().unwrap()));
        }
    }
}

#[test]
fn text_escapes_identifiers_and_delivers_declared_outputs() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(
        root,
        "okf-bundle.yaml",
        r#"manifest_version: 1
guidance:
  rules:
    - id: "plant\tcare"
      instructions: Keep records current.
workflows:
  "bed\nreview":
    description: Review a bed.
    instructions: Check the bed record.
    inputs:
      level: {type: enum, required: true, values: [quick, thorough]}
    outputs: [Updated bed record, List of follow-up tasks]
settings:
  saved_queries:
    "weekly\nnotes": {from: concept}
    'weekly\nnotes': {from: concept}
"#,
    );
    let base = [
        "bundle",
        "context",
        ".",
        "--workflow",
        "bed\nreview",
        "--operation",
        "query",
    ];
    let out = run(root, &[base.as_slice(), &["--format", "agent"]].concat());
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8(out.stdout).unwrap();
    for line in [
        r#"[workflow bed\nreview]"#,
        r#"[rule plant\tcare]"#,
        r#"[saved query weekly\nnotes] {"from":"concept"}"#,
        r#"[saved query weekly\\nnotes] {"from":"concept"}"#,
        "Outputs:",
        "- Updated bed record",
        "- List of follow-up tasks",
        r#"Missing inputs: ["level"]"#,
    ] {
        assert!(text.lines().any(|s| s == line), "{text}");
    }
    let structured = rows(&run(root, &[base.as_slice(), &["--json"]].concat()));
    let workflow = record(&structured, "workflow");
    assert_eq!(workflow["id"], "bed\nreview");
    assert_eq!(
        workflow["inputs"]["level"]["values"],
        serde_json::json!(["quick", "thorough"])
    );
    write(
        root,
        "okf-bundle.yaml",
        "manifest_version: 1\nworkflows:\n  log: {description: Log, instructions: Record date.}\n",
    );
    let out = rows(&run(
        root,
        &[
            "bundle",
            "context",
            ".",
            "--workflow",
            "log",
            "--settings-detail",
            "compact",
            "--json",
        ],
    ));
    let omitted = record(&out, "bundle-context")["settings_projection"]["omitted"]
        .as_array()
        .unwrap();
    assert!(omitted.iter().any(|v| v == "saved_queries: unset or empty"));
    assert!(
        !omitted
            .iter()
            .any(|v| v == "saved_queries: outside selected workflows")
    );
}

#[test]
fn shared_instruction_files_are_delivered_once_with_each_records_own_role() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let body = "Preserve the planting date.";
    write(root, "shared.md", body);
    write(
        root,
        "okf-bundle.yaml",
        r#"manifest_version: 1
guidance:
  rules:
    - {id: inventory, tier: reference, description: Inventory, instructions_file: shared.md}
    - {id: example, tier: reference, description: Example, instructions_file: shared.md}
    - {id: planting, operations: [edit], instructions_file: ./shared.md}
workflows:
  planting:
    description: Planting
    instructions_file: shared.md
    outputs: [Planting record]
"#,
    );
    let args = [
        "bundle",
        "context",
        ".",
        "--operation",
        "edit",
        "--rule",
        "example",
        "--workflow",
        "planting",
        "--token",
        "--stats",
        "--guidance-budget",
        "1",
    ];
    let data = rows(&run(root, &[args.as_slice(), &["--json"]].concat()));
    let matching = |kind: &str, id: &str| {
        data.iter()
            .find(|r| r["kind"] == kind && r["id"] == id)
            .unwrap()
    };
    let summary = matching("guidance", "inventory");
    assert_eq!(summary["content"], "summary");
    assert!(summary.get("instructions_ref").is_none());
    let first = matching("guidance", "example");
    assert_eq!(first["instructions"], body);
    assert_eq!(first["instruction_role"], "inspected");
    for kind in ["guidance", "workflow"] {
        let shared = matching(kind, "planting");
        assert_eq!(shared["content"], "shared");
        assert_eq!(shared["instruction_role"], "task-guidance");
        assert_eq!(
            shared["instructions_ref"],
            serde_json::json!({"kind":"guidance","id":"example"})
        );
        assert!(shared.get("instructions").is_none());
    }
    let stats = record(&data, "context-stats");
    assert_eq!(stats["mandatory_instruction_bytes"], body.len());
    assert_eq!(stats["delivered_instruction_bytes"], body.len());
    assert_eq!(stats["budget_exceeded"], true);
    for format in [&[][..], &["--format", "agent"][..]] {
        let out = run(root, &[args.as_slice(), format].concat());
        assert!(out.status.success());
        let text = String::from_utf8(out.stdout).unwrap();
        assert_eq!(text.matches(body).count(), 1);
        assert_eq!(
            text.matches("Same instructions as [rule example]").count(),
            2
        );
    }
    let token = record(&data, "context-token")["token"].as_str().unwrap();
    let check = [
        "bundle",
        "context",
        ".",
        "--check-token",
        token,
        "--operation",
        "edit",
    ];
    assert!(run(root, &check).status.success());
    write(root, "shared.md", "Record the bed as well.");
    assert!(!run(root, &check).status.success());
}

#[test]
fn instruction_file_references_are_local_to_each_bundle_and_path() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(
        root,
        "okf-catalog.yaml",
        "catalog_version: 2\ndefault_bundle: garden.main\nbundles:\n  garden.main: {location: {type: directory, path: main}}\n  garden.other: {location: {type: directory, path: other}}\n",
    );
    let body = "Record the planting date.";
    for name in ["main", "other"] {
        let bundle = root.join(name);
        write(&bundle, "one.md", body);
        write(&bundle, "two.md", body);
        write(
            &bundle,
            "okf-bundle.yaml",
            r#"manifest_version: 1
guidance:
  instructions_file: one.md
  rules:
    - {id: same-file, instructions_file: one.md}
    - {id: separate-file, instructions_file: two.md}
    - {id: inline, instructions: Record the planting date.}
"#,
        );
    }
    let data = rows(&run(
        root,
        &[
            "bundle",
            "context",
            "--bundle-id",
            "garden.main",
            "--scope-bundle",
            "garden.other",
            "--all",
            "--json",
        ],
    ));
    for bundle in ["garden.main", "garden.other"] {
        let rules: Vec<_> = data
            .iter()
            .filter(|r| r["bundle"] == bundle && r["kind"] == "guidance")
            .collect();
        assert_eq!(
            rules.iter().filter(|r| r["instructions"] == body).count(),
            3
        );
        let shared = rules.iter().find(|r| r["id"] == "same-file").unwrap();
        assert_eq!(
            shared["instructions_ref"],
            serde_json::json!({"kind":"guidance","id":"$global"})
        );
    }
}
