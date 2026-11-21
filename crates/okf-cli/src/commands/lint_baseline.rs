//! Persist and compare the findings produced by the ordinary lint engines.
use super::{catalog, check};
use crate::{cli::LintArgs, output};
use okf_core::{
    bundle::{
        context::Context,
        remote,
        snapshot_context::{self, SnapshotSelection},
    },
    check::lint::{FailOn, Severity},
    error::{OkfError, Result},
    graph::catalog::{build_catalog_graph, resolve_scope},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Write,
    path::Path,
    str::FromStr,
};

pub(super) type Emit<'a> = dyn FnMut(&Value, Option<&str>) -> Result<()> + 'a;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Identity {
    bundle: String,
    rule: String,
    concept: Option<String>,
    code: Option<String>,
    field_path: Option<String>,
    subject: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    identity: Identity,
    severity: Severity,
    count: u64,
    message: String,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Scope {
    Catalog,
    CatalogAndStandalone(String),
    Bundles(BTreeSet<String>),
    Standalone(String),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Baseline {
    baseline_version: u32,
    scope: Scope,
    bundles: BTreeMap<String, String>,
    findings: Vec<Entry>,
}

fn incomplete(message: impl std::fmt::Display) -> OkfError {
    output::mark_partial("examination", "lint-baseline-incomplete");
    OkfError::Environment(format!("incomplete lint baseline: {message}"))
}

fn portable_root(root: &Path) -> String {
    for parent in root.ancestors() {
        if parent.join(".git").exists() {
            let relative = root.strip_prefix(parent).unwrap_or(root);
            return format!("repo:/{}", relative.to_string_lossy().replace('\\', "/"));
        }
    }
    format!(
        "directory:{}",
        root.file_name().unwrap_or_default().to_string_lossy()
    )
}

fn capture(
    args: &LintArgs,
    context: &Context,
    ids: &[String],
    all: bool,
    standalone: &str,
) -> Result<Baseline> {
    let scope = resolve_scope(context, ids, all)?;
    if !scope.unavailable.is_empty() {
        return Err(incomplete(format!(
            "unavailable bundles: {}",
            scope.unavailable.join(", ")
        )));
    }
    let registered = context.primary.id.is_some();
    let name = |id: &str| {
        if id.starts_with("path:") {
            standalone.to_owned()
        } else {
            id.to_owned()
        }
    };
    let bundles: BTreeMap<_, _> = scope
        .examined
        .iter()
        .map(|bundle| {
            let pin = context
                .catalog
                .as_ref()
                .and_then(|c| c.entries.get(&bundle.id))
                .and_then(|e| e.settings.get("remote"))
                .and_then(|r| r.get("commit"))
                .and_then(yaml_serde::Value::as_str);
            (
                name(&bundle.id),
                pin.map_or_else(|| bundle.version.clone(), |commit| format!("git:{commit}")),
            )
        })
        .collect();
    let selection = if all && !registered {
        Scope::CatalogAndStandalone(standalone.into())
    } else if all {
        Scope::Catalog
    } else if !registered && ids.is_empty() {
        Scope::Standalone(standalone.into())
    } else {
        Scope::Bundles(bundles.keys().cloned().collect())
    };
    let mut rows = BTreeMap::<(Identity, Severity), (u64, String)>::new();
    let mut collect = |row: &Value, detail: Option<&str>| {
        let string = |k: &str| row[k].as_str().map(str::to_owned);
        if row["code"] == "incomplete-check"
            || row["rule"] == "unavailable-bundle"
            || row
                .pointer("/resolution/snapshot/status")
                .and_then(Value::as_str)
                == Some("unavailable")
            || matches!(
                row["code"].as_str(),
                Some("unavailable-root" | "unavailable-root-or-revision" | "unchecked-equivalence")
            )
        {
            return Err(incomplete(&row["message"]));
        }
        let severity = serde_json::from_value(row["severity"].clone())
            .map_err(|e| OkfError::Internal(format!("lint severity: {e}")))?;
        let bundle = row["bundle"]
            .as_str()
            .map_or_else(|| standalone.to_owned(), name);
        let field_path = string("field_path")
            .map(|p| {
                if p.starts_with("body:line:") {
                    "body".into()
                } else {
                    p
                }
            })
            .or_else(|| string("directory"));
        let key = Identity {
            bundle,
            rule: string("rule").unwrap_or_default(),
            concept: string("concept"),
            code: string("code"),
            field_path,
            subject: detail
                .map(str::to_owned)
                .or_else(|| string("missing_child")),
        };
        let mut message = string("message").unwrap_or_default();
        for b in &scope.examined {
            message = message.replace(
                b.root.to_string_lossy().as_ref(),
                &format!("<{}>", name(&b.id)),
            );
        }
        let entry = rows.entry((key, severity)).or_insert((0, message.clone()));
        entry.0 += 1;
        if message < entry.1 {
            entry.1 = message;
        }
        Ok(())
    };
    if context.catalog.is_some() || all || !ids.is_empty() {
        let graph = build_catalog_graph(context, ids, all, None)?;
        catalog::run_lint_scoped(args, true, context, &graph, &mut collect)?;
    } else {
        check::lint_in(args, true, context, &mut collect)?;
    }
    let findings = rows
        .into_iter()
        .map(|((identity, severity), (count, message))| Entry {
            identity,
            severity,
            count,
            message,
        })
        .collect();
    Ok(Baseline {
        baseline_version: 1,
        scope: selection,
        bundles,
        findings,
    })
}

fn read(path: &Path) -> Result<Baseline> {
    let bytes = fs::read(path)
        .map_err(|e| OkfError::Usage(format!("cannot read baseline {}: {e}", path.display())))?;
    let baseline: Baseline = serde_json::from_slice(&bytes)
        .map_err(|e| OkfError::Usage(format!("invalid baseline {}: {e}", path.display())))?;
    if baseline.baseline_version != 1 {
        return Err(OkfError::Usage(format!(
            "unsupported lint baseline version {}",
            baseline.baseline_version
        )));
    }
    let members: BTreeSet<_> = baseline.bundles.keys().cloned().collect();
    let valid_scope = match &baseline.scope {
        Scope::Catalog => true,
        Scope::CatalogAndStandalone(id) => members.contains(id),
        Scope::Bundles(ids) => !ids.is_empty() && ids == &members,
        Scope::Standalone(id) => members.len() == 1 && members.contains(id),
    };
    if !valid_scope {
        return Err(OkfError::Usage(
            "invalid lint baseline scope membership".into(),
        ));
    }
    let mut seen = BTreeSet::new();
    for row in &baseline.findings {
        if row.count == 0
            || !baseline.bundles.contains_key(&row.identity.bundle)
            || !seen.insert((&row.identity, row.severity))
        {
            return Err(OkfError::Usage(
                "invalid or duplicate lint baseline entry".into(),
            ));
        }
    }
    Ok(baseline)
}

fn write(path: &Path, baseline: &Baseline) -> Result<()> {
    remote::ensure_writable(path)?;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut temp = tempfile::NamedTempFile::new_in(parent)?;
    serde_json::to_writer_pretty(&mut temp, baseline)
        .map_err(|e| OkfError::Environment(e.to_string()))?;
    writeln!(temp)?;
    temp.as_file().sync_all()?;
    temp.persist(path)
        .map_err(|e| OkfError::Environment(format!("cannot publish {}: {e}", path.display())))?;
    Ok(())
}

struct Bucket<'a> {
    row: &'a Entry,
    left: u64,
}

fn compare(
    before: &Baseline,
    after: &Baseline,
    minimum: Option<Severity>,
    json_output: bool,
) -> Result<bool> {
    if before.scope != after.scope {
        return Err(OkfError::Usage(
            "lint baseline scope differs from requested scope".into(),
        ));
    }
    let mut groups = BTreeMap::<&Identity, (Vec<Bucket<'_>>, Vec<Bucket<'_>>)>::new();
    for row in &before.findings {
        groups.entry(&row.identity).or_default().0.push(Bucket {
            row,
            left: row.count,
        });
    }
    for row in &after.findings {
        groups.entry(&row.identity).or_default().1.push(Bucket {
            row,
            left: row.count,
        });
    }
    let mut failed = false;
    let mut totals = BTreeMap::from([
        ("introduced", 0_u128),
        ("worsened", 0),
        ("existing", 0),
        ("improved", 0),
        ("resolved", 0),
    ]);
    for (key, (mut old, mut new)) in groups {
        let mut report = |state: &'static str,
                          row: &Entry,
                          count: u64,
                          previous: Option<Severity>|
         -> Result<()> {
            if count == 0 {
                return Ok(());
            }
            *totals.entry(state).or_default() += u128::from(count);
            failed |= matches!(state, "introduced" | "worsened")
                && minimum.is_some_and(|s| row.severity >= s);
            if json_output {
                output::print_line(
                    &json!({"kind":"lint-comparison","classification":state,"identity":key,"severity":row.severity,"previous_severity":previous,"count":count,"message":row.message}),
                )
            } else if state == "existing" {
                Ok(())
            } else {
                output::print_text_line(format_args!(
                    "{state}\t{}\t{}\t{}:{}\t{} (x{count})",
                    row.severity.as_str(),
                    key.rule,
                    key.bundle,
                    key.concept.as_deref().unwrap_or("-"),
                    format_args!("{} {}", row.message, key.subject.as_deref().unwrap_or(""))
                ))
            }
        };
        // Match unchanged occurrences first. Only remaining counts can have changed severity.
        old.sort_by_key(|v| v.row.severity);
        new.sort_by_key(|v| v.row.severity);
        for n in &mut new {
            if let Some(o) = old.iter_mut().find(|o| o.row.severity == n.row.severity) {
                let count = o.left.min(n.left);
                report("existing", n.row, count, Some(o.row.severity))?;
                o.left -= count;
                n.left -= count;
            }
        }
        for n in &mut new {
            for o in &mut old {
                let count = o.left.min(n.left);
                report(
                    if n.row.severity > o.row.severity {
                        "worsened"
                    } else {
                        "improved"
                    },
                    n.row,
                    count,
                    Some(o.row.severity),
                )?;
                o.left -= count;
                n.left -= count;
            }
            report("introduced", n.row, n.left, None)?;
        }
        for o in old {
            report("resolved", o.row, o.left, Some(o.row.severity))?;
        }
    }
    if json_output {
        output::print_line(
            &json!({"kind":"lint-comparison-summary","counts":totals,"before_bundles":before.bundles,"after_bundles":after.bundles}),
        )?;
    } else {
        for (id, version) in &after.bundles {
            if let Some(previous) = before.bundles.get(id)
                && previous != version
            {
                output::print_text_line(format_args!("{id}: {previous} -> {version}"))?;
            }
        }
        output::print_text_line(format_args!(
            "{} introduced, {} worsened, {} existing, {} improved, {} resolved",
            totals["introduced"],
            totals["worsened"],
            totals["existing"],
            totals["improved"],
            totals["resolved"]
        ))?;
    }
    Ok(failed)
}

fn capture_revision(
    args: &LintArgs,
    context: &Context,
    ids: &[String],
    all: bool,
    standalone: &str,
    revision: String,
) -> Result<(Baseline, String)> {
    let selection = SnapshotSelection::parse(Some(revision), &[])?;
    let mut source = context.clone();
    if all && context.primary.id.is_some() {
        // The current primary may have been added since the base commit. Historical
        // catalog membership determines the examined bundles on that side.
        source.primary.id = None;
    }
    let mut snapshot = snapshot_context::prepare(&source, selection, ids).map_err(incomplete)?;
    let commit = snapshot
        .workspace_commit
        .clone()
        .ok_or_else(|| incomplete("unresolved workspace revision"))?;
    if all
        && context.primary.id.is_some()
        && let Some(catalog) = &snapshot.context.catalog
    {
        let Some(id) = catalog
            .default_bundle
            .as_ref()
            .or_else(|| catalog.entries.keys().next())
        else {
            return Ok((
                Baseline {
                    baseline_version: 1,
                    scope: Scope::Catalog,
                    bundles: BTreeMap::new(),
                    findings: Vec::new(),
                },
                commit,
            ));
        };
        snapshot.context.primary.id = Some(id.clone());
        snapshot.context.primary.root = catalog.entry(id)?.root.clone();
    }
    let _guard = snapshot.install();
    let scope = resolve_scope(&snapshot.context, ids, all).map_err(incomplete)?;
    for bundle in scope.examined {
        if remote::origin(&bundle.root)?.is_none() {
            return Err(incomplete(format!(
                "{} has no historical snapshot",
                bundle.id
            )));
        }
    }
    let before = capture(args, &snapshot.context, ids, all, standalone).map_err(incomplete)?;
    Ok((before, commit))
}

pub(super) fn run(
    args: &LintArgs,
    context: &Context,
    ids: &[String],
    all: bool,
    json_output: bool,
) -> Result<i32> {
    let standalone = portable_root(&context.primary.root);
    let current = capture(args, context, ids, all, &standalone)?;
    if let Some(path) = &args.write_baseline {
        write(path, &current)?;
        if json_output {
            output::print_line(
                &json!({"kind":"lint-baseline","path":path,"bundles":current.bundles,"entries":current.findings.len()}),
            )?;
        } else {
            output::print_text_line(format_args!("wrote lint baseline {}", path.display()))?;
        }
        return Ok(0);
    }
    let previous = if let Some(path) = &args.baseline {
        read(path)?
    } else {
        let revision = args
            .baseline_ref
            .clone()
            .ok_or_else(|| OkfError::Internal("missing lint baseline selector".into()))?;
        let (before, commit) = capture_revision(args, context, ids, all, &standalone, revision)?;
        if json_output {
            output::print_line(&json!({"kind":"lint-baseline","revision":commit}))?;
        } else {
            output::print_text_line(format_args!("baseline revision {commit}"))?;
        }
        before
    };
    let minimum = FailOn::from_str(args.fail_on.map_or("error", crate::cli::FailOnArg::as_str))?
        .min_severity();
    Ok(i32::from(compare(
        &previous,
        &current,
        minimum,
        json_output,
    )?))
}
