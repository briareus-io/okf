//! Explicit portable context and bounded support-resource inspection.
use crate::{
    cli::{BundleCmd, BundleContextArgs, BundleResourceCmd, BundleResourceShowArgs},
    output,
};
use okf_core::bundle::context_receipt::ContextReceipt;
use okf_core::{
    bundle::{manifest, resolve::resolve_bundle},
    error::{OkfError, Result},
};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    io::Read,
    path::{Component, Path, PathBuf},
};

pub fn read_selection(raw: &str) -> Result<okf_core::bundle::selection::SelectionReceipt> {
    let text = if let Some(path) = raw.strip_prefix('@') {
        let mut text = String::new();
        std::fs::File::open(path)?
            .take(1024 * 1024 + 1)
            .read_to_string(&mut text)?;
        if text.len() > 1024 * 1024 {
            return Err(OkfError::classified(
                okf_core::error::FailureReason::ContextInvalid,
                "selection token exceeds 1 MiB",
            ));
        }
        text
    } else {
        raw.to_owned()
    };
    okf_core::bundle::selection::SelectionReceipt::parse(text.trim())
}

pub fn select(args: &crate::cli::BundleArgs, id: Option<&str>, json_output: bool) -> Result<i32> {
    let receipt = okf_core::bundle::selection::SelectionReceipt::issue(args.bundle.as_deref(), id)?;
    let token = receipt.encode()?;
    if json_output {
        output::print_line(
            &json!({"kind":"bundle-selection", "root":receipt.root, "id":receipt.id, "source":receipt.source, "access":receipt.access, "token":token}),
        )?;
    } else {
        output::print_text_line(format_args!(
            "Bundle root: {}\nSelection token: {}",
            receipt.root.display(),
            token
        ))?;
    }
    Ok(0)
}

fn read_token(raw: &str) -> Result<ContextReceipt> {
    let text = if let Some(path) = raw.strip_prefix('@') {
        let mut text = String::new();
        std::fs::File::open(path)?
            .take(1024 * 1024 + 1)
            .read_to_string(&mut text)?;
        text
    } else {
        raw.to_owned()
    };
    ContextReceipt::parse(&text)
}

/// Tokens are optional. If supplied, every explicitly examined bundle needs one.
/// Never follow a root supplied by a token: resolve roots from the command first.
pub fn check_execution_context(cli: &mut crate::cli::Cli) -> Result<()> {
    if cli.context.is_empty() {
        return Ok(());
    }
    if cli.revision.is_some()
        || cli.workspace_revision.is_some()
        || !cli.bundle_revision.is_empty()
        || okf_core::bundle::snapshot_context::is_inspection_active()
    {
        return Err(OkfError::classified(
            okf_core::error::FailureReason::ContextInvalid,
            "execution tokens cannot be used for historical inspection",
        ));
    }
    let operation = crate::exit::command_name(&cli.command);
    let path = super::catalog::bundle_slot(&mut cli.command).and_then(|s| s.clone());
    let selected =
        okf_core::bundle::context::current_context(path.as_deref(), cli.bundle_id.as_deref())?;
    let scope =
        okf_core::graph::catalog::resolve_scope(&selected, &cli.scope_bundle, cli.catalog_scope)?;
    if !scope.unavailable.is_empty() {
        return Err(OkfError::classified(
            okf_core::error::FailureReason::ContextInvalid,
            format!("unavailable bundles: {}", scope.unavailable.join(", ")),
        ));
    }
    let mut receipts = BTreeMap::new();
    for raw in &cli.context {
        let receipt = read_token(raw)?;
        if receipts.insert(receipt.root.clone(), receipt).is_some() {
            return Err(OkfError::classified(
                okf_core::error::FailureReason::ContextInvalid,
                "duplicate bundle token",
            ));
        }
    }
    let roots = scope
        .examined
        .iter()
        .map(|m| std::fs::canonicalize(&m.root))
        .collect::<std::io::Result<Vec<_>>>()?;
    if receipts.keys().any(|root| !roots.contains(root)) {
        return Err(OkfError::classified(
            okf_core::error::FailureReason::ContextInvalid,
            "token bundle is outside requested scope",
        ));
    }
    for root in roots {
        let receipt = receipts.get(&root).ok_or_else(|| {
            OkfError::classified(
                okf_core::error::FailureReason::ContextInvalid,
                format!(
                    "missing token for {}; reload bundle context for the requested scope",
                    root.display()
                ),
            )
        })?;
        receipt.check(&root, std::slice::from_ref(&operation), &command_names())?;
    }
    Ok(())
}

pub fn command_names() -> Vec<String> {
    fn collect(cmd: &clap::Command, prefix: &str, out: &mut Vec<String>) {
        let name = if prefix.is_empty() {
            cmd.get_name().to_owned()
        } else {
            format!("{prefix} {}", cmd.get_name())
        };
        if cmd.has_subcommands() {
            for child in cmd.get_subcommands() {
                collect(child, &name, out);
            }
        } else {
            out.push(name);
        }
    }
    let tree = crate::cli::command_tree();
    let mut out = Vec::new();
    for child in tree.get_subcommands() {
        collect(child, "", &mut out);
    }
    out.push("catalog".into());
    out
}
pub fn json_input(raw: &str) -> Result<Value> {
    let text = if let Some(path) = raw.strip_prefix('@') {
        std::fs::read_to_string(path)?
    } else {
        raw.into()
    };
    serde_json::from_str(&text).map_err(|e| OkfError::Usage(format!("invalid JSON input: {e}")))
}
pub fn run(command: &BundleCmd, json_output: bool) -> Result<i32> {
    match command {
        BundleCmd::Select(_) => unreachable!("selection is dispatched before bundle reads"),
        BundleCmd::Context(args) => context(args, json_output),
        BundleCmd::Validate(args) => {
            let root = resolve_bundle(args.bundle.as_deref())?;
            let loaded = manifest::validate(&root, &command_names())?;
            if json_output {
                output::print_line(
                    &json!({"kind":"manifest-validation","root":root,"present":loaded.is_some(),"valid":true}),
                )?;
            } else {
                output::print_text_line(format_args!(
                    "manifest: {}",
                    if loaded.is_some() {
                        "valid"
                    } else {
                        "absent (optional)"
                    }
                ))?;
            }
            Ok(0)
        }
        BundleCmd::Resource(BundleResourceCmd::List(args)) => {
            let root = resolve_bundle(args.bundle.as_deref())?;
            for path in okf_core::bundle::walk::walk_files(&root)? {
                if path == Path::new(manifest::FILENAME)
                    || path.starts_with(manifest::SUPPORT_DIRECTORY)
                {
                    let abs = manifest::local_path(&root, &path.to_string_lossy())?;
                    let size = std::fs::metadata(&abs)?.len();
                    if json_output {
                        output::print_line(
                            &json!({"kind":"bundle-resource","path":path,"size":size}),
                        )?;
                    } else {
                        output::print_text_line(format_args!("{}\t{}", path.display(), size))?;
                    }
                }
            }
            Ok(0)
        }
        BundleCmd::Resource(BundleResourceCmd::Show(args)) => resource_show(args, json_output),
    }
}
fn context(args: &BundleContextArgs, json_output: bool) -> Result<i32> {
    let root = resolve_bundle(args.bundle.as_deref())?;
    let selected = okf_core::bundle::context::current_context(Some(&root.to_string_lossy()), None)?;
    let scope = okf_core::graph::catalog::resolve_scope(
        &selected,
        &super::catalog::examination_scope(),
        false,
    )?;
    if let Some(raw) = &args.check_token {
        if scope.examined.len() != 1 || !scope.unavailable.is_empty() {
            return Err(OkfError::Usage(
                "--check-token requires one available selected bundle".into(),
            ));
        }
        let receipt = read_token(raw)?;
        let checked = receipt.check(&root, &args.operation, &command_names())?;
        if json_output {
            output::print_line(
                &json!({"kind":"context-check","valid":true,"root":checked.root,"context_digest":checked.context_digest,"state_digest":checked.state_digest,"operations":checked.selection.operations}),
            )?;
        } else {
            output::print_text_line(format_args!(
                "Context is current; covered operations: {}",
                checked.selection.operations.join(", ")
            ))?;
        }
        return Ok(0);
    }
    if args.token && !scope.unavailable.is_empty() {
        return Err(OkfError::Usage(
            "cannot issue tokens for incomplete context scope".into(),
        ));
    }
    if json_output {
        output::print_line(
            &json!({"kind":"context-scope","requested":scope.requested,"examined":scope.examined.iter().map(|e|json!({"bundle":e.id,"root":e.root,"version":e.version})).collect::<Vec<_>>(),"unavailable":scope.unavailable,"complete":scope.unavailable.is_empty()}),
        )?;
    } else {
        if args.format.is_none() {
            output::print_text_line(format_args!(
                "Context scope: {}",
                scope.requested.join(", ")
            ))?;
        }
        if !scope.unavailable.is_empty() {
            output::print_text_line(format_args!(
                "Unavailable: {}",
                scope.unavailable.join(", ")
            ))?;
        }
    }
    if !scope.unavailable.is_empty() {
        output::mark_partial("examination", "context-scope-unavailable");
    }
    for (index, member) in scope.examined.iter().enumerate() {
        if args.format.is_some() && scope.requested.len() > 1 {
            output::print_text_line(format_args!("Bundle: {}", member.id))?;
        }
        context_one(
            args,
            &member.root,
            index == 0 && member.root == selected.primary.root,
            json_output,
        )?;
    }
    if args.format.is_some() {
        let status = if scope.unavailable.is_empty() {
            "complete"
        } else {
            "partial"
        };
        output::print_text_line(format_args!("Context: {status}"))?;
    }
    Ok(if scope.unavailable.is_empty() { 0 } else { 1 })
}
fn context_one(
    args: &BundleContextArgs,
    root: &Path,
    primary: bool,
    json_output: bool,
) -> Result<()> {
    let inputs = match args.inputs.as_ref().filter(|_| primary) {
        Some(raw) => serde_json::from_value::<BTreeMap<String, Value>>(json_input(raw)?)
            .map_err(|e| OkfError::Usage(format!("inputs must be an object: {e}")))?,
        None => BTreeMap::new(),
    };
    if !inputs.is_empty() && args.workflow.is_empty() {
        return Err(OkfError::Usage("--inputs requires --workflow".into()));
    }
    let request = manifest::ContextRequest {
        operations: args.operation.clone(),
        rules: args.rule.clone(),
        workflows: if primary {
            args.workflow.clone()
        } else {
            Vec::new()
        },
        all: args.all,
        inspection: okf_core::bundle::snapshot_context::is_inspection_active(),
        inputs,
        query: if primary {
            args.query.as_deref().map(json_input).transpose()?
        } else {
            None
        },
        ..Default::default()
    };
    let mut effective_queries = Vec::new();
    if let Some(query) = &request.query {
        if request.operations.is_empty() {
            return Err(OkfError::Usage("--query requires --operation".into()));
        }
        for operation in &request.operations {
            if matches!(operation.as_str(), "search" | "list") {
                let mut raw = query.clone();
                let preset = raw
                    .as_object_mut()
                    .and_then(|m| m.remove("preset"))
                    .map(|v| {
                        v.as_str()
                            .map(str::to_owned)
                            .ok_or_else(|| OkfError::Usage("preset must be a string".into()))
                    })
                    .transpose()?;
                let clear = raw
                    .as_object_mut()
                    .and_then(|m| m.remove("clear_filter"))
                    .map(serde_json::from_value::<Vec<String>>)
                    .transpose()
                    .map_err(|e| OkfError::Usage(e.to_string()))?
                    .unwrap_or_default();
                let explicit =
                    serde_json::from_value::<okf_core::bundle::settings::PrimaryQueryPreset>(raw)
                        .map_err(|e| OkfError::Usage(format!("invalid primary query: {e}")))?;
                let resolved = super::filters::resolve_primary(
                    root,
                    operation,
                    preset.as_deref(),
                    &explicit,
                    &clear,
                )?;
                effective_queries
                    .push(json!({"kind":"effective-query","operation":operation,"query":resolved}));
            } else {
                if !matches!(operation.as_str(), "links" | "backlinks" | "graph") {
                    return Err(OkfError::Usage(format!(
                        "typed query options are not supported for {operation}"
                    )));
                }
                let filters = super::filters::from_json(query)?;
                let (resolved, _, details) = super::filters::resolve(root, operation, &filters)?;
                effective_queries.push(json!({"kind":"effective-query","operation":operation,"filters":resolved,"details":details}));
            }
        }
    }
    let mut context = manifest::context(root, request, &command_names())?;
    context.records.extend(effective_queries);
    let bundle = context.effective_settings.bundle.clone();
    for record in &mut context.records {
        annotate_origin(record, root, bundle.as_deref());
        add_selection_metadata(record, &context.selection, root);
    }

    let token = if args.token {
        Some(ContextReceipt::issue(&context)?)
    } else {
        None
    };
    let (settings, omitted) = settings_view(
        &context,
        if args.format.is_some() {
            "compact"
        } else {
            &args.settings_detail
        },
        !args.workflow.is_empty(),
    )?;

    let mandatory_bytes = share_instruction_files(&mut context.records);

    if json_output {
        let mut head =
            serde_json::to_value(&context).map_err(|e| OkfError::Internal(e.to_string()))?;
        if let Some(object) = head.as_object_mut() {
            object.remove("records");
            object.remove("state_components");
        }
        head["effective_settings"] = settings.clone();
        if args.settings_detail == "compact" {
            head["setting_origins"]
                .as_object_mut()
                .expect("invariant: setting_origins serializes as an object")
                .retain(|path, _| {
                    let mut value = &settings["settings"];
                    for part in path.split('.') {
                        value = &value[part];
                    }
                    !value.is_null()
                });
        }
        head["settings_projection"] = json!({"detail":args.settings_detail,"omitted":omitted});
        head["kind"] = json!("bundle-context");
        head["bundle"] = json!(bundle);
        head["primary"] = json!(primary);
        output::print_line(&head)?;
        for record in &context.records {
            output::print_line(record)?;
        }
    } else if args.format.is_some() {
        print_agent_context(&context, settings)?;
    } else {
        output::print_text_line(format_args!(
            "Bundle root: {}\nContext digest: {}\nManifest: {}",
            root.display(),
            context.context_digest,
            context.manifest.as_ref().map_or_else(
                || "absent".into(),
                |m| format!(
                    "{} (digest: {})",
                    manifest::FILENAME,
                    m["digest"].as_str().unwrap_or_default()
                )
            )
        ))?;
        if context.records.iter().any(|r| r["content"] == "summary") {
            output::print_text_line(format_args!(
                "Expand entries with: okf bundle context {} --json",
                shell_word(&root.to_string_lossy())
            ))?;
        }
        let mut settings = compact_query_steps(settings);
        if args.settings_detail == "compact"
            && let Some(queries) = settings["settings"]
                .as_object_mut()
                .and_then(|map| map.remove("saved_queries"))
        {
            print_query_definitions(&queries)?;
        }
        output::print_text_line(format_args!(
            "Effective settings:\n{}",
            yaml_serde::to_string(&text_settings(settings))
                .map_err(|e| OkfError::Internal(e.to_string()))?
        ))?;
        if args.settings_detail == "full" {
            output::print_text_line(format_args!(
                "Setting origins:\n{}",
                yaml_serde::to_string(&context.setting_origins)
                    .map_err(|e| OkfError::Internal(e.to_string()))?
            ))?;
        } else if !omitted.is_empty() {
            output::print_text_line(format_args!(
                "Settings omitted: {} (use --settings-detail full)",
                omitted.join(", ")
            ))?;
        }
        for r in &context.records {
            print_context_record(r)?;
        }
        output::print_text_line(format_args!(
            "Delivery: all applicable mandatory instructions delivered; other rules and workflows listed for discovery. Inspection-only bodies do not activate tasks."
        ))?;
    }
    if args.stats || args.guidance_budget.is_some() {
        print_stats(&context, mandatory_bytes, args.guidance_budget, json_output)?;
    }
    // Emit the receipt only after the instruction records have been written successfully.
    if let Some(token) = token {
        if json_output {
            output::print_line(
                &json!({"kind":"context-token","root":root,"bundle":bundle,"token":token}),
            )?;
        } else {
            output::print_text_line(format_args!("Context token: {token}"))?;
        }
    }
    Ok(())
}
fn annotate_origin(record: &mut Value, root: &Path, bundle: Option<&str>) {
    record["bundle"] = json!(bundle);
    record["root"] = json!(root);
    if !record["origin"].is_object() {
        record["origin"] = json!({});
    }
    record["origin"]["bundle"] = json!(bundle);
    record["origin"]["root"] = json!(root);
}

fn shell_word(value: &str) -> String {
    if value.chars().any(char::is_control) {
        // Octal quoting keeps the command on one line and preserves the exact UTF-8 bytes.
        let bytes: String = value.bytes().map(|b| format!("\\{b:03o}")).collect();
        return format!("$'{bytes}'");
    }
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn expansion_flag(record: &Value) -> &'static str {
    if record["kind"] == "workflow" {
        "--workflow"
    } else {
        "--rule"
    }
}

// Keep text labels on one line and distinguish an escaped newline from a literal backslash-n.
fn display_id(id: &str) -> String {
    let mut label = String::with_capacity(id.len());
    for ch in id.chars() {
        if ch.is_control() || matches!(ch, '\\' | '\u{2028}' | '\u{2029}') {
            label.extend(ch.escape_debug());
        } else {
            label.push(ch);
        }
    }
    label
}

fn add_selection_metadata(record: &mut Value, selection: &manifest::ContextRequest, root: &Path) {
    let id = record["id"].as_str().unwrap_or("").to_owned();
    let reason = if selection.all || selection.inspection {
        "inspection only".to_owned()
    } else if record["kind"] == "workflow" {
        if record["selected"] == true {
            "named workflow selected"
        } else {
            "workflow not selected"
        }
        .into()
    } else if id == "$global" || record["operations"].is_null() {
        "always (bundle-wide guidance)".into()
    } else {
        let matched: Vec<_> = selection
            .operations
            .iter()
            .filter(|op| {
                record["operations"]
                    .as_array()
                    .is_some_and(|ops| ops.iter().any(|v| v == op.as_str()))
            })
            .cloned()
            .collect();
        if matched.is_empty() {
            "outside requested operations".into()
        } else {
            format!("manifest operation rule: {}", matched.join(", "))
        }
    };
    record["selection_reason"] = json!(reason);
    if record["content"] == "summary" {
        let flag = expansion_flag(record);
        let argv = vec![
            "okf".to_owned(),
            "bundle".into(),
            "context".into(),
            root.to_string_lossy().into_owned(),
            flag.into(),
            id,
            "--json".into(),
        ];
        record["expand_command"] = json!(
            argv.iter()
                .map(|a| shell_word(a))
                .collect::<Vec<_>>()
                .join(" ")
        );
        record["expand_argv"] = json!(argv);
    }
}

/// A conservative projection: unknown operations and selected workflows keep all groups.
/// Operational defaults remain visible; optional null/empty leaves can be omitted explicitly.
fn settings_view(
    context: &manifest::BundleContext,
    detail: &str,
    workflow_requested: bool,
) -> Result<(Value, Vec<String>)> {
    let mut value = serde_json::to_value(&context.effective_settings)
        .map_err(|e| OkfError::Internal(e.to_string()))?;
    let mut omitted = Vec::new();
    if detail == "full" {
        return Ok((value, omitted));
    }
    let mut groups = std::collections::BTreeSet::from(["ontology"]);
    let mut all = context.selection.operations.is_empty()
        || !context.selection.workflows.is_empty()
        || context.selection.all;
    for operation in &context.selection.operations {
        let relevant: &[&str] = match operation.as_str() {
            "query" => &["saved_queries", "query_presets", "query"],
            "list" | "search" => &["views", "facets", "query_presets", "query"],
            "links" | "backlinks" | "graph" => &["query_presets", "query"],
            "show"
            | "browse"
            | "resolve"
            | "scan"
            | "stats"
            | "affected"
            | "diff"
            | "artifact list"
            | "artifact show"
            | "artifact resolve"
            | "computation check"
            | "bundle resource list"
            | "bundle resource show"
            | "bundle validate"
            | "ontology list"
            | "ontology show"
            | "ontology field-type list"
            | "ontology field-type show" => &[],
            "validate" => &["lint"],
            "lint" | "doctor" | "stale" => &["lint", "write_policy", "formatting"],
            "add"
            | "edit"
            | "format"
            | "mv"
            | "rm"
            | "verify"
            | "refresh"
            | "artifact put"
            | "docs"
            | "changeset plan"
            | "changeset apply"
            | "changeset digest"
            | "changeset recover"
            | "ontology add"
            | "ontology update"
            | "ontology remove"
            | "ontology apply"
            | "ontology field-type add"
            | "ontology field-type update"
            | "ontology field-type remove" => &["write_policy", "formatting"],
            _ => {
                all = true;
                &[]
            }
        };
        groups.extend(relevant.iter().copied());
    }
    if workflow_requested && !context.selection.operations.iter().any(|op| op == "query") {
        let settings = value["settings"]
            .as_object_mut()
            .expect("settings serialize as an object");
        if let Some(queries) = settings
            .get_mut("saved_queries")
            .and_then(Value::as_object_mut)
        {
            let declared: std::collections::BTreeSet<_> = context
                .records
                .iter()
                .filter(|record| record["kind"] == "workflow" && record["selected"] == true)
                .filter_map(|record| record["uses_queries"].as_array())
                .flatten()
                .filter_map(Value::as_str)
                .collect();
            let before = queries.len();
            queries.retain(|name, _| declared.contains(name.as_str()));
            if queries.len() < before {
                omitted.push("saved_queries: outside selected workflows".into());
            }
            if queries.is_empty() {
                settings.remove("saved_queries");
            }
        } else {
            omitted.push("saved_queries: unset or empty".into());
        }
    }
    if !all {
        value["settings"]
            .as_object_mut()
            .expect("invariant: settings view is an object")
            .retain(|key, _| {
                let retain = groups.contains(key.as_str());
                if !retain {
                    omitted.push(format!("{key}: outside selected operations"));
                }
                retain
            });
    }
    fn prune(value: &mut Value, path: &str, omitted: &mut Vec<String>) {
        if let Some(map) = value.as_object_mut() {
            map.retain(|key, value| {
                let path = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{path}.{key}")
                };
                prune(value, &path, omitted);
                let empty = value.is_null()
                    || value.as_object().is_some_and(serde_json::Map::is_empty)
                    || value.as_array().is_some_and(std::vec::Vec::is_empty);
                if empty {
                    omitted.push(format!("{path}: unset or empty"));
                }
                !empty
            });
        }
    }
    prune(&mut value["settings"], "", &mut omitted);
    Ok((value, omitted))
}

fn text_settings(mut value: Value) -> Value {
    if let Some(map) = value.as_object_mut() {
        for value in map.values_mut() {
            *value = text_settings(value.take());
        }
        map.retain(|_, v| !v.is_null() && !v.as_object().is_some_and(serde_json::Map::is_empty));
    }
    value
}

// Remove only empty filter lists. An empty links/backlinks hop still means traverse.
fn compact_query_steps(mut settings: Value) -> Value {
    if let Some(queries) = settings
        .pointer_mut("/settings/saved_queries")
        .and_then(Value::as_object_mut)
    {
        for query in queries.values_mut() {
            let Some(steps) = query.get_mut("steps").and_then(Value::as_array_mut) else {
                continue;
            };
            for step in steps.iter_mut().filter_map(Value::as_object_mut) {
                for filters in step.values_mut().filter_map(Value::as_object_mut) {
                    filters.retain(|_, values| !values.as_array().is_some_and(Vec::is_empty));
                }
            }
        }
    }
    settings
}

/// Project one bundle at a time. The first full record remains the body owner,
/// even when a later record applies the same bytes in a different role.
fn share_instruction_files(records: &mut [Value]) -> usize {
    let mut owners: BTreeMap<_, (Value, bool)> = BTreeMap::new();
    let mut mandatory_bytes = 0;
    for record in records {
        let Some(body) = record["instructions"].as_str() else {
            continue;
        };
        let size = body.len();
        let mandatory = record["instruction_role"] == "task-guidance";
        let Some(file) = record["origin"]["instructions_file"].as_str() else {
            if mandatory {
                mandatory_bytes += size;
            }
            continue;
        };
        let path: PathBuf = Path::new(file)
            .components()
            .filter(|part| *part != Component::CurDir)
            .collect();
        // A changed file must never be represented by a reference to different bytes.
        let key = (
            path,
            record["origin"]["instruction_digest"]
                .as_str()
                .expect("loaded instruction file has a digest")
                .to_owned(),
        );
        if let Some((owner, counted_as_mandatory)) = owners.get_mut(&key) {
            record
                .as_object_mut()
                .expect("instruction record is an object")
                .remove("instructions");
            record["content"] = json!("shared");
            record["instructions_ref"] = owner.clone();
            if mandatory && !*counted_as_mandatory {
                mandatory_bytes += size;
                *counted_as_mandatory = true;
            }
        } else {
            owners.insert(
                key,
                (json!({"kind":record["kind"], "id":record["id"]}), mandatory),
            );
            if mandatory {
                mandatory_bytes += size;
            }
        }
    }
    mandatory_bytes
}

fn print_instruction_body(record: &Value) -> Result<()> {
    if let Some(body) = record["instructions"].as_str() {
        output::print_text_line(format_args!("{body}"))?;
    } else if let Some(owner) = record.get("instructions_ref") {
        let kind = if owner["kind"] == "workflow" {
            "workflow"
        } else {
            "rule"
        };
        output::print_text_line(format_args!(
            "Same instructions as [{kind} {}]",
            display_id(owner["id"].as_str().unwrap_or_default())
        ))?;
    }
    Ok(())
}

fn print_agent_context(context: &manifest::BundleContext, settings: Value) -> Result<()> {
    output::print_text_line(format_args!("Context digest: {}", context.context_digest))?;
    for record in &context.records {
        if record["kind"] == "effective-query" {
            print_context_record(record)?;
            continue;
        }
        let kind = if record["kind"] == "workflow" {
            "workflow"
        } else {
            "rule"
        };
        let id = display_id(record["id"].as_str().unwrap_or_default());
        if record["content"] == "summary" {
            let description = record["description"].as_str().unwrap_or_default();
            output::print_text_line(format_args!(
                "[available {kind} {id}] {}",
                description.split_whitespace().collect::<Vec<_>>().join(" ")
            ))?;
            continue;
        }
        let role = if record["instruction_role"] == "inspected" {
            "inspect "
        } else {
            ""
        };
        output::print_text_line(format_args!("[{role}{kind} {id}]"))?;
        print_instruction_body(record)?;
        print_resources(record)?;
        if let Some(outputs) = record["outputs"].as_array().filter(|v| !v.is_empty()) {
            output::print_text_line(format_args!("Outputs:"))?;
            for output in outputs.iter().filter_map(Value::as_str) {
                output::print_text_line(format_args!("- {output}"))?;
            }
        }
        if record["effective_inputs"]
            .as_object()
            .is_some_and(|v| !v.is_empty())
        {
            output::print_text_line(format_args!("Inputs: {}", record["effective_inputs"]))?;
        }
        if record["inputs_complete"] == false {
            output::print_text_line(format_args!(
                "Missing inputs: {}",
                record["missing_required"]
            ))?;
        }
    }
    print_query_definitions(&compact_query_steps(settings)["settings"]["saved_queries"])
}

fn print_query_definitions(queries: &Value) -> Result<()> {
    if let Some(queries) = queries.as_object() {
        for (name, query) in queries {
            output::print_text_line(format_args!("[saved query {}] {query}", display_id(name)))?;
        }
    }
    Ok(())
}

fn print_resources(record: &Value) -> Result<()> {
    if let Some(resources) = record["resources"].as_array().filter(|v| !v.is_empty()) {
        output::print_text_line(format_args!("Resources: {}", json!(resources)))?;
    }
    Ok(())
}

fn print_stats(
    context: &manifest::BundleContext,
    mandatory: usize,
    budget: Option<usize>,
    json_output: bool,
) -> Result<()> {
    let sections: Vec<_> = context.records.iter().filter(|r| matches!(r["kind"].as_str(), Some("guidance" | "workflow")))
        .map(|r| json!({"id":r["id"],"kind":r["kind"],"delivered_bytes":r["instructions"].as_str().map_or(0, str::len),"mandatory":r["instruction_role"] == "task-guidance"})).collect();
    let total: u64 = sections
        .iter()
        .filter_map(|r| r["delivered_bytes"].as_u64())
        .sum();
    let exceeded = budget.is_some_and(|b| mandatory > b);
    if json_output {
        output::print_line(
            &json!({"kind":"context-stats","root":context.root,"sections":sections,"delivered_instruction_bytes":total,"mandatory_instruction_bytes":mandatory,"mandatory_budget":budget,"budget_exceeded":exceeded}),
        )?;
    } else {
        for section in sections {
            output::print_text_line(format_args!(
                "Instruction bytes: {} {} = {}",
                section["kind"].as_str().unwrap_or_default(),
                display_id(section["id"].as_str().unwrap_or_default()),
                section["delivered_bytes"]
            ))?;
        }
        output::print_text_line(format_args!(
            "Delivered instruction bytes: {total}; mandatory: {mandatory}; budget exceeded: {exceeded}"
        ))?;
    }
    Ok(())
}
fn print_context_record(r: &Value) -> Result<()> {
    if r["kind"] == "effective-query" {
        let mut effective = json!({"operation": r["operation"]});
        for key in ["query", "filters"] {
            if let Some(filters) = r[key].as_object() {
                let mut filters = filters.clone();
                filters.retain(|_, v| !v.is_null() && !v.as_array().is_some_and(Vec::is_empty));
                effective[key] = Value::Object(filters);
            }
        }
        if let Some(details) = r.get("details") {
            effective["details"] = details.clone();
        }
        return output::print_text_line(format_args!("Effective query: {effective}"));
    }
    output::print_text_line(format_args!(
        "{}: {} [{}]",
        r["kind"].as_str().unwrap_or("context"),
        display_id(r["id"].as_str().unwrap_or("")),
        r["content"].as_str().unwrap_or("")
    ))?;
    for key in [
        "tier",
        "instruction_role",
        "selection_reason",
        "omission_reason",
    ] {
        if let Some(value) = r[key].as_str() {
            output::print_text_line(format_args!("  {key}: {value}"))?;
        }
    }
    if let Some(origin) = r["origin"].as_object() {
        let mut local = origin.clone();
        for common in ["root", "bundle", "manifest", "manifest_digest"] {
            local.remove(common);
        }
        let details = text_settings(Value::Object(local));
        if details.as_object().is_some_and(|m| !m.is_empty()) {
            output::print_text_line(format_args!("  origin: {details}"))?;
        }
    }
    if r["content"] == "summary" {
        output::print_text_line(format_args!(
            "  expand arguments: {} {}",
            expansion_flag(r),
            shell_word(r["id"].as_str().unwrap_or_default())
        ))?;
    }
    if let Some(d) = r["description"].as_str() {
        output::print_text_line(format_args!("  {d}"))?;
    }
    print_instruction_body(r)?;
    print_resources(r)?;
    if r.get("inputs_complete").is_some() {
        output::print_text_line(format_args!(
            "  inputs complete: {}; missing: {}",
            r["inputs_complete"], r["missing_required"]
        ))?;
    }
    Ok(())
}
fn resource_show(args: &BundleResourceShowArgs, json_output: bool) -> Result<i32> {
    if args.max_bytes == 0 {
        return Err(OkfError::Usage("--max-bytes must be positive".into()));
    }
    let root = resolve_bundle(args.bundle.as_deref())?;
    let relative = Path::new(&args.resource);
    if relative != Path::new(manifest::FILENAME)
        && !relative.starts_with(manifest::SUPPORT_DIRECTORY)
    {
        return Err(OkfError::Usage(
            "bundle resource show requires the manifest or .okf-bundle/ resource".into(),
        ));
    }
    let path = manifest::local_path(&root, &args.resource)?;
    let mut bytes = Vec::new();
    manifest::open_regular_nofollow(&path)?
        .take(args.max_bytes as u64 + 1)
        .read_to_end(&mut bytes)?;
    let truncated = bytes.len() > args.max_bytes;
    bytes.truncate(args.max_bytes);
    let digest = if truncated {
        None
    } else {
        Some(okf_core::fingerprint::canonicalize::sha256_hex(&bytes))
    };
    let body = std::str::from_utf8(&bytes).ok().map(str::to_owned);
    let body = if let Some(range) = &args.lines {
        let (a, b) = range
            .split_once(':')
            .ok_or_else(|| OkfError::Usage("--lines requires START:END".into()))?;
        let start: usize = a
            .parse()
            .map_err(|_| OkfError::Usage("invalid start line".into()))?;
        let end: usize = b
            .parse()
            .map_err(|_| OkfError::Usage("invalid end line".into()))?;
        if start == 0 || end < start {
            return Err(OkfError::Usage("invalid line range".into()));
        }
        body.map(|s| {
            s.lines()
                .skip(start - 1)
                .take(end - start + 1)
                .collect::<Vec<_>>()
                .join("\n")
        })
    } else {
        body
    };
    if truncated {
        output::mark_partial("content", "resource-truncated");
    }
    if json_output {
        output::print_line(
            &json!({"kind":"bundle-resource","path":path,"resource":args.resource,"sha256":digest,"content":body,"truncated":truncated,"bytes":bytes.len()}),
        )?;
    } else if let Some(body) = body {
        output::print_text_line(format_args!("{body}"))?;
    } else {
        output::print_text_line(format_args!(
            "binary resource: {} ({} bytes inspected)",
            path.display(),
            bytes.len()
        ))?;
    }
    Ok(0)
}
/// Guidance is a separate payload, never merged into a concept or raw artifact body.
pub fn incidental(
    root: &Path,
    command: &str,
    json_output: bool,
    requested: Option<&str>,
) -> Result<()> {
    let Some(loaded) = manifest::load(root)? else {
        return Ok(());
    };
    let mode = requested.unwrap_or(if json_output {
        &loaded.manifest.guidance.display.json
    } else {
        &loaded.manifest.guidance.display.text
    });
    manifest::validate_commands(&loaded.manifest, &command_names())?;
    if mode == "off" {
        return Ok(());
    }
    let context = manifest::context(
        root,
        manifest::ContextRequest {
            operations: vec![command.into()],
            inspection: okf_core::bundle::snapshot_context::is_inspection_active(),
            ..Default::default()
        },
        &command_names(),
    )?;
    for mut record in context
        .records
        .into_iter()
        .filter(|r| r["kind"] == "guidance" && r["matches_selection"] == true)
    {
        record["context_digest"] = json!(context.context_digest);
        annotate_origin(
            &mut record,
            root,
            context.effective_settings.bundle.as_deref(),
        );
        if mode == "summary" {
            if let Some(object) = record.as_object_mut() {
                object.remove("instructions");
            }
            record["content"] = json!("summary");
            record["omission_reason"] = json!("display-summary");
        }
        if json_output {
            output::print_line(&record)?;
        } else {
            eprintln!(
                "guidance [{}] {}: {}",
                context
                    .effective_settings
                    .bundle
                    .as_deref()
                    .map_or_else(|| root.display().to_string(), str::to_owned),
                record["id"].as_str().unwrap_or(""),
                record["description"]
                    .as_str()
                    .unwrap_or("bundle instructions")
            );
            if mode == "full" {
                if let Some(body) = record["instructions"].as_str() {
                    eprintln!("{body}");
                }
            } else {
                eprintln!(
                    "  Read: okf bundle context '{}' --operation '{}'",
                    root.display(),
                    command
                );
            }
        }
    }
    Ok(())
}
