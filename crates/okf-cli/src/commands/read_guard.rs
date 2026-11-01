//! Whole-command barriers for cooperative working-tree readers.
use crate::cli::{ArtifactCmd, Cli, Command, OntFieldTypeCmd, OntologyCmd};
use okf_core::bundle::context::current_context;
use okf_core::error::{OkfError, Result};
use okf_core::mutate::transaction::{reader_locks, ReaderGuard};

/// Mutation families (including their previews) acquire their own writer guards;
/// nesting a read guard around them would conflict with those exclusive locks.
pub fn guards_command(command: &Command) -> bool {
    match command {
        Command::List(_)
        | Command::Search(_)
        | Command::Show(_)
        | Command::Browse(_)
        | Command::Backlinks(_)
        | Command::Links(_)
        | Command::Graph(_)
        | Command::Resolve(_)
        | Command::Computation(_)
        | Command::Scan(_)
        | Command::Validate(_)
        | Command::Stale(_)
        | Command::Affected(_)
        | Command::Diff(_)
        | Command::Stats(_)
        | Command::Bundle(_) => true,
        Command::Artifact(command) => !matches!(command, ArtifactCmd::Put(_)),
        Command::Lint(args) => !args.fix,
        Command::Doctor(args) => {
            args.migration_apply.is_none() && !args.fix_safe && !args.normalize_relative_links
        }
        Command::Docs(args) => args.format != "index",
        Command::Ontology(command) => matches!(
            command,
            OntologyCmd::List(_)
                | OntologyCmd::Show(_)
                | OntologyCmd::FieldType(OntFieldTypeCmd::List(_) | OntFieldTypeCmd::Show(_))
        ),
        _ => false,
    }
}

/// Called before guidance, filter interpretation, graph construction, or reads.
/// Selection discovers roots only; bundle content is consumed under the guards.
pub fn acquire(cli: &mut Cli) -> Result<Vec<ReaderGuard>> {
    if !guards_command(&cli.command) || cli.revision.is_some() {
        return Ok(Vec::new());
    }
    let path = super::catalog::bundle_slot(&mut cli.command).and_then(|slot| slot.clone());
    let context = current_context(path.as_deref(), cli.bundle_id.as_deref())?;
    let mut roots = vec![context.primary.root.clone()];
    if cli.catalog_scope || !cli.scope_bundle.is_empty() {
        let catalog = context.catalog.as_ref().ok_or_else(|| {
            OkfError::Usage("explicit bundle scope requires a valid catalog".into())
        })?;
        if cli.catalog_scope {
            roots.extend(
                catalog
                    .entries
                    .values()
                    .filter(|entry| entry.available)
                    .map(|entry| entry.root.clone()),
            );
        } else {
            for id in &cli.scope_bundle {
                let entry = catalog.entry(id)?;
                if entry.available {
                    roots.push(entry.root.clone());
                }
            }
        }
    }
    // Historical scope may mix materialized immutable roots with working trees.
    // reader_locks skips protected snapshots without skipping mutable members.
    roots.retain(|root| root.is_dir());
    reader_locks(&roots)
}
