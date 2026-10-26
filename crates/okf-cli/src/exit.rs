//! Map a command result (and lint/stale findings) to an exit code.
//!
//! 0 ok · 1 findings at/above `--fail-on` (later) · 2 usage · 3 env/IO · 4 internal.
use crate::cli::Cli;
use crate::commands;
use clap::error::ErrorKind;
use clap::FromArgMatches;

/// Parse argv, dispatch, and return the process exit code.
pub fn run() -> i32 {
    let cli = match crate::cli::command_tree()
        .try_get_matches()
        .and_then(|m| Cli::from_arg_matches(&m))
    {
        Ok(cli) => cli,
        Err(err) => {
            // clap prints help/version/usage itself.
            let _ = err.print();
            return match err.kind() {
                ErrorKind::DisplayHelp
                | ErrorKind::DisplayVersion
                | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand => 0,
                _ => 2,
            };
        }
    };

    okf_core::mutate::transaction::reset_publication_evidence();
    let stream = !matches!(
        &cli.command,
        crate::cli::Command::Schema | crate::cli::Command::Version
    );
    if stream {
        let name = command_name(&cli.command);
        let writes = crate::commands::catalog::writes_command(&cli.command)
            || matches!(&cli.command, crate::cli::Command::Changeset(crate::cli::ChangeSetCmd::Apply(a)) if !a.dry_run)
            || matches!(
                &cli.command,
                crate::cli::Command::Changeset(crate::cli::ChangeSetCmd::Recover(_))
            );
        if let Err(err) = crate::output::begin(&name, cli.json, writes, cli.revision.is_some()) {
            eprintln!("okf: {err}");
            return err.exit_code();
        }
    }
    let code = match commands::run(cli) {
        Ok(code) => code,
        Err(err) => {
            eprintln!("okf: {err}");
            let reason = match &err {
                okf_core::error::OkfError::Policy(message)
                    if message.starts_with("write-gate-incomplete:") =>
                {
                    "write-gate-incomplete"
                }
                okf_core::error::OkfError::Policy(message)
                    if message.starts_with("write-gate-rejected:") =>
                {
                    "write-gate-rejected"
                }
                okf_core::error::OkfError::Policy(message)
                    if message.starts_with("write-gate-comparison-ambiguous:") =>
                {
                    "write-gate-incomplete"
                }
                okf_core::error::OkfError::Usage(message)
                    if message.starts_with("baseline-conflict:") =>
                {
                    "baseline-conflict"
                }
                _ => match err.exit_code() {
                    1 => "policy-rejected",
                    2 => "usage-error",
                    3 => "environment-error",
                    _ => "internal-error",
                },
            };
            crate::output::mark_failure(err.exit_code(), reason);
            if reason == "write-gate-incomplete" {
                crate::output::mark_partial("examination", reason);
            }
            err.exit_code()
        }
    };
    if let Err(err) = crate::output::complete(code) {
        eprintln!("okf: {err}");
        return err.exit_code();
    }
    if let Err(err) = crate::output::flush() {
        eprintln!("okf: {err}");
        return err.exit_code();
    }
    code
}

fn command_name(command: &crate::cli::Command) -> String {
    use crate::cli::{
        ArtifactCmd, ChangeSetCmd, Command, ComputationCmd, OntFieldTypeCmd, OntologyCmd,
    };
    match command {
        Command::Artifact(sub) => format!(
            "artifact {}",
            match sub {
                ArtifactCmd::List(_) => "list",
                ArtifactCmd::Resolve(_) => "resolve",
                ArtifactCmd::Show(_) => "show",
                ArtifactCmd::Put(_) => "put",
            }
        ),
        Command::Computation(ComputationCmd::Check(_)) => "computation check".into(),
        Command::Ontology(sub) => format!(
            "ontology {}",
            match sub {
                OntologyCmd::List(_) => "list",
                OntologyCmd::Show(_) => "show",
                OntologyCmd::Add(_) => "add",
                OntologyCmd::Update(_) => "update",
                OntologyCmd::Remove(_) => "remove",
                OntologyCmd::Apply(_) => "apply",
                OntologyCmd::FieldType(field) => match field {
                    OntFieldTypeCmd::List(_) => "field-type list",
                    OntFieldTypeCmd::Show(_) => "field-type show",
                    OntFieldTypeCmd::Add(_) => "field-type add",
                    OntFieldTypeCmd::Update(_) => "field-type update",
                    OntFieldTypeCmd::Remove(_) => "field-type remove",
                },
            }
        ),
        Command::Changeset(sub) => format!(
            "changeset {}",
            match sub {
                ChangeSetCmd::Digest(_) => "digest",
                ChangeSetCmd::Plan(_) => "plan",
                ChangeSetCmd::Apply(_) => "apply",
                ChangeSetCmd::Recover(_) => "recover",
            }
        ),
        Command::Schema => "schema".into(),
        Command::Catalog => "catalog".into(),
        Command::Version => "version".into(),
        Command::List(_) => "list".into(),
        Command::Search(_) => "search".into(),
        Command::Show(_) => "show".into(),
        Command::Browse(_) => "browse".into(),
        Command::Backlinks(_) => "backlinks".into(),
        Command::Links(_) => "links".into(),
        Command::Graph(_) => "graph".into(),
        Command::Resolve(_) => "resolve".into(),
        Command::Scan(_) => "scan".into(),
        Command::SourceScan(_) => "source-scan".into(),
        Command::Validate(_) => "validate".into(),
        Command::Lint(_) => "lint".into(),
        Command::Stale(_) => "stale".into(),
        Command::Affected(_) => "affected".into(),
        Command::Diff(_) => "diff".into(),
        Command::Stats(_) => "stats".into(),
        Command::Doctor(_) => "doctor".into(),
        Command::Init(_) => "init".into(),
        Command::Add(_) => "add".into(),
        Command::Edit(_) => "edit".into(),
        Command::Mv(_) => "mv".into(),
        Command::Rm(_) => "rm".into(),
        Command::Verify(_) => "verify".into(),
        Command::Refresh(_) => "refresh".into(),
        Command::Docs(_) => "docs".into(),
    }
}
