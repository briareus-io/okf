use crate::{cli::FormatArgs, output, structured};
use okf_core::{bundle::resolve::resolve_bundle, error::Result, mutate::format};
use serde_json::json;

pub fn run(args: &FormatArgs, json_output: bool) -> Result<i32> {
    let root = resolve_bundle(args.bundle.as_deref())?;
    let apply = !args.check && !args.dry_run;
    let changes = format::format(&root, args.concept.as_deref(), apply)?;
    for change in &changes {
        let (Some(before), Some(after)) = (
            change
                .before
                .as_deref()
                .and_then(|b| std::str::from_utf8(b).ok()),
            change
                .after
                .as_deref()
                .and_then(|a| std::str::from_utf8(a).ok()),
        ) else {
            return Err(okf_core::error::OkfError::Internal(format!(
                "format change for {} lacks UTF-8 before/after content",
                change.path.display()
            )));
        };
        if json_output {
            output::print_line(
                &json!({"kind":"format-file","schema_version":1,"path":change.path,"applied":apply,"diff":if args.check {None} else {Some(structured::preview_diff(&change.path,before,after))}}),
            )?;
        } else if args.check {
            output::print_text_line(format_args!("needs formatting: {}", change.path.display()))?;
        } else if args.dry_run {
            output::print_text(format_args!(
                "{}",
                structured::preview_diff(&change.path, before, after)
            ))?;
        } else {
            output::print_text_line(format_args!("formatted: {}", change.path.display()))?;
        }
    }
    if json_output {
        output::print_line(
            &json!({"kind":"format-summary","schema_version":1,"files":changes.len(),"applied":apply}),
        )?;
    } else {
        output::print_text_line(format_args!(
            "{} file(s) {}",
            changes.len(),
            if apply {
                "formatted"
            } else {
                "need formatting"
            }
        ))?;
    }
    Ok(i32::from(args.check && !changes.is_empty()))
}
