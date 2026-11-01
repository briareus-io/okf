//! Explicit portable context and bounded support-resource inspection.
use crate::{
    cli::{BundleCmd, BundleContextArgs, BundleResourceCmd, BundleResourceShowArgs},
    output,
};
use okf_core::{
    bundle::{manifest, resolve::resolve_bundle},
    error::{OkfError, Result},
};
use serde_json::{json, Value};
use std::{collections::BTreeMap, io::Read, path::Path};

pub fn command_names() -> Vec<String> {
    fn collect(cmd: &clap::Command, prefix: &str, out: &mut Vec<String>) {
        let name = if prefix.is_empty() {
            cmd.get_name().to_owned()
        } else {
            format!("{prefix} {}", cmd.get_name())
        };
        if cmd.has_subcommands() {
            for child in cmd.get_subcommands() {
                collect(child, &name, out)
            }
        } else {
            out.push(name)
        }
    }
    let tree = crate::cli::command_tree();
    let mut out = Vec::new();
    for child in tree.get_subcommands() {
        collect(child, "", &mut out)
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
        BundleCmd::Context(args) => context(args, json_output),
        BundleCmd::Validate(args) => {
            let root = resolve_bundle(args.bundle.as_deref())?;
            let loaded = manifest::validate(&root, &command_names())?;
            if json_output {
                output::print_line(
                    &json!({"kind":"manifest-validation","root":root,"present":loaded.is_some(),"valid":true}),
                )?
            } else {
                output::print_text_line(format_args!(
                    "manifest: {}",
                    if loaded.is_some() {
                        "valid"
                    } else {
                        "absent (optional)"
                    }
                ))?
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
                        )?
                    } else {
                        output::print_text_line(format_args!("{}\t{}", path.display(), size))?
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
    if json_output {
        output::print_line(
            &json!({"kind":"context-scope","requested":scope.requested,"examined":scope.examined.iter().map(|e|json!({"bundle":e.id,"root":e.root,"version":e.version})).collect::<Vec<_>>(),"unavailable":scope.unavailable,"complete":scope.unavailable.is_empty()}),
        )?;
    } else {
        output::print_text_line(format_args!(
            "Context scope: {}; unavailable: {}",
            scope.requested.join(", "),
            scope.unavailable.join(", ")
        ))?;
    }
    if !scope.unavailable.is_empty() {
        output::mark_partial("examination", "context-scope-unavailable");
    }
    for (index, member) in scope.examined.iter().enumerate() {
        context_one(
            args,
            &member.root,
            index == 0 && member.root == selected.primary.root,
            json_output,
        )?;
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
    }

    if json_output {
        let mut head =
            serde_json::to_value(&context).map_err(|e| OkfError::Internal(e.to_string()))?;
        head.as_object_mut().unwrap().remove("records");
        head["kind"] = json!("bundle-context");
        head["bundle"] = json!(bundle);
        head["primary"] = json!(primary);
        output::print_line(&head)?;
        for record in &context.records {
            output::print_line(record)?
        }
    } else {
        output::print_text_line(format_args!(
            "Bundle root: {}\nContext digest: {}\nManifest: {}",
            root.display(),
            context.context_digest,
            context
                .manifest
                .as_ref()
                .map(|m| m.to_string())
                .unwrap_or("absent".into())
        ))?;
        output::print_text_line(format_args!(
            "Effective settings:\n{}",
            serde_yaml::to_string(&context.effective_settings)
                .map_err(|e| OkfError::Internal(e.to_string()))?
        ))?;
        for r in &context.records {
            print_context_record(r)?
        }
        output::print_text_line(format_args!("Delivery: complete for requested view"))?;
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
fn print_context_record(r: &Value) -> Result<()> {
    output::print_text_line(format_args!(
        "{}: {} [{}]",
        r["kind"].as_str().unwrap_or("context"),
        r["id"].as_str().unwrap_or(""),
        r["content"].as_str().unwrap_or("")
    ))?;
    if let Some(d) = r["description"].as_str() {
        output::print_text_line(format_args!("  {d}"))?
    }
    if let Some(body) = r["instructions"].as_str() {
        output::print_text_line(format_args!("{body}"))?
    }
    if r.get("inputs_complete").is_some() {
        output::print_text_line(format_args!(
            "  inputs complete: {}; missing: {}",
            r["inputs_complete"], r["missing_required"]
        ))?
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
    std::fs::File::open(&path)?
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
        output::mark_partial("content", "resource-truncated")
    }
    if json_output {
        output::print_line(
            &json!({"kind":"bundle-resource","path":path,"resource":args.resource,"sha256":digest,"content":body,"truncated":truncated,"bytes":bytes.len()}),
        )?
    } else if let Some(body) = body {
        output::print_text_line(format_args!("{body}"))?
    } else {
        output::print_text_line(format_args!(
            "binary resource: {} ({} bytes inspected)",
            path.display(),
            bytes.len()
        ))?
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
            record.as_object_mut().unwrap().remove("instructions");
            record["content"] = json!("summary");
            record["omission_reason"] = json!("display-summary")
        }
        if json_output {
            output::print_line(&record)?
        } else {
            eprintln!(
                "guidance [{}] {}: {}",
                context
                    .effective_settings
                    .bundle
                    .as_deref()
                    .map(str::to_owned)
                    .unwrap_or_else(|| root.display().to_string()),
                record["id"].as_str().unwrap_or(""),
                record["description"]
                    .as_str()
                    .unwrap_or("bundle instructions")
            );
            if mode == "full" {
                if let Some(body) = record["instructions"].as_str() {
                    eprintln!("{body}")
                }
            } else {
                eprintln!(
                    "  Read: okf bundle context '{}' --operation '{}'",
                    root.display(),
                    command
                )
            }
        }
    }
    Ok(())
}
