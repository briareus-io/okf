use crate::{cli::FormatArgs, output, structured};
use okf_core::{bundle::resolve::resolve_bundle, error::Result, mutate::format};
use serde_json::json;

pub fn run(args: &FormatArgs, json_output: bool) -> Result<i32> {
    let root = resolve_bundle(args.bundle.as_deref())?;
    let apply = !args.check && !args.dry_run;
    let changes = format::format(&root, args.concept.as_deref(), apply)?;
    for change in &changes {
        let before = std::str::from_utf8(change.before.as_deref().unwrap()).unwrap();
        let after = std::str::from_utf8(change.after.as_deref().unwrap()).unwrap();
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
