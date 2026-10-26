//! scan, validate, lint, stale, affected, diff, stats.
use std::io::{BufRead, IsTerminal};

use crate::cli::{
    AffectedArgs, BundleArgs, ChangedBase, DiffArgs, DoctorArgs, FailOnArgs, LintArgs,
    SourceScanArgs, StaleArgs,
};
use crate::output;
use okf_core::bundle::loader::{load_bundle, load_bundle_metadata};
use okf_core::bundle::resolve::resolve_bundle;
use okf_core::bundle::walk::{walk_files, walk_markdown};
use okf_core::check::doctor::doctor;
use okf_core::check::lint::{lint_bundle, meets_threshold, FailOn, LintConfig};
use okf_core::check::stale::check_stale;
use okf_core::check::validate::validate_bundle;
use okf_core::error::{OkfError, Result};
use okf_core::graph::affected::{affected_with_sources, AffectedOptions};
use okf_core::graph::build::build_graph;
use okf_core::ports::git::Git;
use okf_core::ports::git::RealGit;
use okf_core::query::diff::diff;
use okf_core::query::stats::stats;
use serde_json::json;
use std::path::Path;
use std::str::FromStr;

/// `okf scan [bundle]` — report the candidate concept files the loader would analyze.
pub fn run_scan(args: &FailOnArgs, json: bool) -> Result<i32> {
    let root = resolve_bundle(args.bundle.as_deref())?;
    let paths = walk_markdown(&root)?;
    if json {
        for path in &paths {
            let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            output::print_line(&json!({
                "kind": "scan",
                "file": path.to_string_lossy(),
                "reserved": matches!(name, "index.md" | "log.md"),
            }))?;
        }
    } else {
        output::print_text_line(format_args!("{} Markdown file(s):", paths.len()))?;
        for path in &paths {
            output::print_text_line(format_args!("  {}", path.display()))?;
        }
    }
    discovery_exit(&args.fail_on, paths.is_empty())
}

pub fn run_source_scan(args: &SourceScanArgs, json_output: bool) -> Result<i32> {
    let root = Path::new(&args.directory);
    let files = walk_files(root)?;
    if json_output {
        for path in files {
            let abs = root.join(&path);
            let size = std::fs::metadata(&abs)
                .map_err(|e| OkfError::Io(format!("{}: {e}", abs.display())))?
                .len();
            output::print_line(&json!({
                "kind": "source-file",
                "file": path.to_string_lossy(),
                "extension": path.extension().and_then(|s| s.to_str()),
                "size": size,
            }))?;
        }
    } else {
        for path in files {
            output::print_text_line(format_args!("{}", path.display()))?;
        }
    }
    Ok(0)
}

/// `okf validate [bundle]` — conformance (three hard rules). Exit 1 on nonconformance.
pub fn run_validate(args: &BundleArgs, json: bool) -> Result<i32> {
    let root = resolve_bundle(args.bundle.as_deref())?;
    let report = validate_bundle(&root)?;
    if json {
        for v in &report.violations {
            output::print_line(&json!({
                "kind": "violation",
                "file": v.file,
                "rule": v.rule.as_str(),
                "message": v.message,
            }))?;
        }
    } else if report.is_conformant() {
        output::print_text_line(format_args!("conformant: no violations"))?;
    } else {
        for v in &report.violations {
            output::print_text_line(format_args!(
                "{}\t{}\t{}",
                v.file,
                v.rule.as_str(),
                v.message
            ))?;
        }
        eprintln!("{} violation(s)", report.violations.len());
    }
    Ok(if report.is_conformant() { 0 } else { 1 })
}

/// `okf lint [bundle] [--fix] [--fail-on <sev>]`.
pub fn run_lint(args: &LintArgs, json: bool) -> Result<i32> {
    let fail_on = match &args.fail_on {
        Some(s) => FailOn::from_str(s)?,
        None => FailOn::default(), // error
    };
    let root = resolve_bundle(args.bundle.as_deref())?;
    let bundle = load_bundle(&root)?;
    let (effective, ontology) = okf_core::bundle::settings::load_for(&root)?;
    let config = lint_config(&effective.settings)?;
    if args.fix && config.frontmatter_order.is_some() {
        let fixed = okf_core::check::lint::rules::frontmatter_order::fix(
            &bundle,
            &root,
            ontology.as_ref(),
        )?;
        if json {
            output::print_line(
                &json!({"kind": "fix", "rule": "frontmatter-order", "fixed": fixed}),
            )?;
        } else {
            eprintln!("--fix: normalized frontmatter order in {fixed} concepts");
        }
    }
    let bundle = if args.fix {
        load_bundle(&root)?
    } else {
        bundle
    };
    let findings = lint_bundle(&bundle, ontology.as_ref(), &config);
    let index_severity = lint_severity(
        effective.settings.lint.index_coverage.as_deref(),
        okf_core::check::lint::Severity::Warn,
    )?;
    let index_findings = if index_severity.is_some() {
        okf_core::check::lint::rules::index_coverage::check_indexes(
            &bundle,
            &effective.settings.lint.index_exclude,
        )?
    } else {
        Vec::new()
    };

    if json {
        for f in &findings {
            output::print_line(&json!({
                "kind": "finding",
                "rule": f.rule,
                "severity": f.severity.as_str(),
                "concept": f.concept,
                "message": f.message,
                "code": f.code,
                "field_path": f.field_path,
                "bundle": effective.bundle,
            }))?;
        }
    } else {
        if findings.is_empty() && index_findings.is_empty() {
            output::print_text_line(format_args!("clean: no findings"))?;
        }
        for f in &findings {
            output::print_text_line(format_args!(
                "{}\t{}\t{}\t{}",
                f.severity.as_str(),
                f.rule,
                f.concept.as_deref().unwrap_or("-"),
                f.message
            ))?;
        }
    }

    for finding in &index_findings {
        let Some(index_severity) = index_severity else {
            continue;
        };
        if json {
            let mut record =
                serde_json::to_value(finding).map_err(|e| OkfError::Internal(e.to_string()))?;
            record["kind"] = json!("finding");
            record["rule"] = json!("index-coverage");
            record["severity"] = json!(index_severity.as_str());
            record["bundle"] = json!(effective.bundle);
            output::print_line(&record)?;
        } else {
            output::print_text_line(format_args!(
                "{}\tindex-coverage\t{}\t{}",
                index_severity.as_str(),
                finding.directory,
                finding.message
            ))?;
        }
    }

    Ok(
        if meets_threshold(&findings, fail_on)
            || (!index_findings.is_empty()
                && index_severity
                    .is_some_and(|severity| fail_on.min_severity().is_some_and(|s| severity >= s)))
        {
            1
        } else {
            0
        },
    )
}

/// `okf stale [bundle] [--fail-on <sev>]` — informational unless `--fail-on` is set.
pub fn run_stale(args: &StaleArgs, json: bool) -> Result<i32> {
    let root = resolve_bundle(args.bundle.as_deref())?;
    let bundle = load_bundle_metadata(&root)?;
    let report = check_stale(&bundle);
    let provenance_only = bundle
        .concepts
        .iter()
        .flat_map(|concept| {
            concept
                .frontmatter
                .get("sources")
                .and_then(|value| value.as_sequence())
                .into_iter()
                .flatten()
        })
        .filter(|entry| entry.as_mapping().is_some_and(|mapping| {
            matches!(mapping.get(serde_yaml::Value::String("resource".into())), Some(serde_yaml::Value::String(resource)) if !resource.trim().is_empty())
                && !mapping.contains_key(serde_yaml::Value::String("kind".into()))
        }))
        .count();

    let examined = report
        .concepts
        .iter()
        .map(|c| c.sources.len() + usize::from(c.expired.is_some()))
        .sum::<usize>();
    let matches_state = |state: &str, states: &[crate::cli::SourceState]| {
        states.is_empty() || states.iter().any(|s| s.as_str() == state)
    };
    let displayed = report
        .concepts
        .iter()
        .map(|c| {
            c.sources
                .iter()
                .filter(|s| matches_state(s.drift.as_str(), &args.only))
                .count()
                + usize::from(c.expired.is_some() && matches_state("expired", &args.only))
        })
        .sum::<usize>();
    if json {
        for cd in &report.concepts {
            let sources: Vec<_> = cd
                .sources
                .iter()
                .filter(|s| matches_state(s.drift.as_str(), &args.only))
                .map(|s| json!({"resource": s.resource, "kind": s.kind, "state": s.drift.as_str(), "drift": s.drift.as_str(), "code": format!("source-{}", s.drift.as_str()), "message": s.message}))
                .collect();
            let expired = if matches_state("expired", &args.only) {
                cd.expired.clone()
            } else {
                None
            };
            if sources.is_empty() && expired.is_none() {
                continue;
            }
            output::print_line(&json!({
                "kind": "drift",
                "schema_version": 2,
                "concept": cd.concept,
                "expired": expired,
                "sources": sources,
            }))?;
        }
    } else if displayed == 0 {
        output::print_text_line(format_args!(
            "{}",
            if report.is_empty() {
                "in sync: nothing drifted"
            } else {
                "no displayed findings (filtered)"
            }
        ))?;
    } else {
        for cd in &report.concepts {
            if let Some(exp) = cd
                .expired
                .as_ref()
                .filter(|_| matches_state("expired", &args.only))
            {
                output::print_text_line(format_args!(
                    "{}\texpired\tstale_after {exp}",
                    cd.concept
                ))?;
            }
            for s in &cd.sources {
                if !matches_state(s.drift.as_str(), &args.only) {
                    continue;
                }
                output::print_text_line(format_args!(
                    "{}\t{}\t{}\t{}",
                    cd.concept,
                    s.drift.as_str(),
                    s.resource,
                    s.message
                ))?;
            }
        }
    }
    if args.summary {
        if json {
            output::print_line(
                &json!({"kind":"diagnostic-summary","schema_version":1,"examined":examined,"displayed":displayed,"omitted":examined-displayed,"provenance_only":provenance_only}),
            )?;
        } else {
            output::print_text_line(format_args!("examined={examined} displayed={displayed} omitted={} provenance_only={provenance_only}", examined-displayed))?;
        }
    }
    if !args.fail_on_state.is_empty() {
        Ok(i32::from(report.concepts.iter().any(|c| {
            (c.expired.is_some() && matches_state("expired", &args.fail_on_state))
                || c.sources
                    .iter()
                    .any(|s| matches_state(s.drift.as_str(), &args.fail_on_state))
        })))
    } else {
        discovery_exit(&args.fail_on, report.is_empty())
    }
}

/// `okf affected [bundle] --changed <..> [--transitive] [--depth N]` (also reads stdin).
pub fn run_affected(args: &AffectedArgs, json: bool) -> Result<i32> {
    let root = resolve_bundle(args.bundle.as_deref())?;
    let bundle = load_bundle(&root)?;
    let (_, ontology) = okf_core::bundle::settings::load_for(&root)?;

    let changed = changed_inputs(args, &root)?;

    let graph = build_graph(&bundle, ontology.as_ref());
    let opts = AffectedOptions {
        transitive: args.transitive,
        depth: args.depth,
    };
    let ids = affected_with_sources(&bundle, &graph, &changed, &opts);

    if json {
        for id in &ids {
            output::print_line(&json!({"kind": "affected", "concept": id.0}))?;
        }
    } else if ids.is_empty() {
        output::print_text_line(format_args!("no affected concepts"))?;
    } else {
        for id in &ids {
            output::print_text_line(format_args!("{}", id.0))?;
        }
    }
    discovery_exit(&args.fail_on, ids.is_empty())
}

/// Resolve the one declared input namespace before either standalone or catalog analysis.
pub fn changed_inputs(args: &AffectedArgs, bundle_root: &Path) -> Result<Vec<String>> {
    let explicit_stdin = args.changed.iter().filter(|s| s.as_str() == "-").count();
    if explicit_stdin > 1 {
        return Err(OkfError::Usage("--changed - may occur only once".into()));
    }
    let mut raw: Vec<String> = args
        .changed
        .iter()
        .filter(|s| s.as_str() != "-")
        .cloned()
        .collect();
    if explicit_stdin == 1 || (args.changed.is_empty() && !std::io::stdin().is_terminal()) {
        for line in std::io::stdin().lock().lines() {
            let line = line.map_err(|e| OkfError::Io(e.to_string()))?;
            let value = line.trim();
            if !value.is_empty() {
                raw.push(value.to_string());
            }
        }
    }
    if raw.is_empty() {
        return Err(OkfError::Usage(
            "affected requires --changed targets or explicit stdin".into(),
        ));
    }
    if args.changed_base == ChangedBase::Bundle {
        return Ok(raw);
    }
    let base = match args.changed_base {
        ChangedBase::Repo => RealGit.worktree_root(bundle_root).map_err(|e| {
            OkfError::Usage(format!(
                "repository root unavailable for --changed-base repo: {e}"
            ))
        })?,
        ChangedBase::Filesystem => std::env::current_dir()?,
        ChangedBase::Bundle => unreachable!(),
    };
    let bundle_root = okf_core::bundle::catalog::absolute(bundle_root)?;
    let base = okf_core::bundle::catalog::absolute(&base)?;
    raw.into_iter()
        .map(|input| {
            if input.contains(":/") {
                return Err(OkfError::Usage(format!(
                    "qualified changed target requires --changed-base bundle: {input}"
                )));
            }
            let (path, suffix) = split_changed_suffix(&input);
            if path.is_empty() {
                return Err(OkfError::Usage(format!("empty changed path: {input}")));
            }
            let absolute = okf_core::bundle::catalog::normalize(&base.join(path));
            let relative = lexical_relative(&bundle_root, &absolute);
            Ok(format!("{}{}", relative.display(), suffix))
        })
        .collect()
}

fn split_changed_suffix(value: &str) -> (&str, &str) {
    match value.find(['#', '?']) {
        Some(i) => (&value[..i], &value[i..]),
        None => (value, ""),
    }
}

fn lexical_relative(base: &Path, target: &Path) -> std::path::PathBuf {
    let base: Vec<_> = base.components().collect();
    let target: Vec<_> = target.components().collect();
    let shared = base.iter().zip(&target).take_while(|(a, b)| a == b).count();
    let mut result = std::path::PathBuf::new();
    for _ in shared..base.len() {
        result.push("..");
    }
    for component in &target[shared..] {
        result.push(component.as_os_str());
    }
    result
}

/// `okf diff <git-ref> [bundle] [--fail-on <sev>]`.
pub fn run_diff(args: &DiffArgs, json: bool) -> Result<i32> {
    let root = resolve_bundle(args.bundle.as_deref())?;
    let bundle = load_bundle(&root)?;
    let git = RealGit;
    let d = diff(&bundle, &git, &args.git_ref)?;

    if json {
        for (change, ids) in [
            ("added", &d.added),
            ("removed", &d.removed),
            ("modified", &d.modified),
        ] {
            for id in ids {
                output::print_line(&json!({"kind": "diff", "change": change, "concept": id.0}))?;
            }
        }
    } else if d.is_empty() {
        output::print_text_line(format_args!("no changes vs {}", args.git_ref))?;
    } else {
        for id in &d.added {
            output::print_text_line(format_args!("added\t{}", id.0))?;
        }
        for id in &d.removed {
            output::print_text_line(format_args!("removed\t{}", id.0))?;
        }
        for id in &d.modified {
            output::print_text_line(format_args!("modified\t{}", id.0))?;
        }
    }
    discovery_exit(&args.fail_on, d.is_empty())
}

/// `okf stats [bundle] [--fail-on <sev>]`.
pub fn run_stats(args: &FailOnArgs, json: bool) -> Result<i32> {
    let root = resolve_bundle(args.bundle.as_deref())?;
    let bundle = load_bundle(&root)?;
    let (_, ontology) = okf_core::bundle::settings::load_for(&root)?;
    let s = stats(&bundle, ontology.as_ref());

    if json {
        output::print_line(&json!({
            "kind": "stats",
            "total": s.total,
            "by_type": serde_json::to_value(&s.by_type).unwrap_or(serde_json::Value::Null),
            "trust": {
                "unverified": s.trust.unverified,
                "machine_confirmed": s.trust.machine_confirmed,
                "human_reviewed": s.trust.human_reviewed,
            },
            "stale": s.stale,
            "orphans": s.orphans,
        }))?;
    } else {
        println!("total: {}", s.total);
        println!("by type:");
        for (ty, n) in &s.by_type {
            println!("  {ty}: {n}");
        }
        println!("trust:");
        println!("  unverified: {}", s.trust.unverified);
        println!("  machine-confirmed: {}", s.trust.machine_confirmed);
        println!("  human-reviewed: {}", s.trust.human_reviewed);
        println!("orphans: {}", s.orphans);
        if let Some(stale) = s.stale {
            println!("stale: {stale}");
        }
    }
    // Stats is purely informational; only `--fail-on` (non-never) with a non-empty bundle fails.
    discovery_exit(&args.fail_on, s.total == 0)
}

/// Exit code for the informational discovery commands: `0` unless `--fail-on` is set to
/// something other than `never` and there is at least one result.
fn discovery_exit(fail_on: &Option<String>, empty: bool) -> Result<i32> {
    match fail_on {
        None => Ok(0),
        Some(s) => {
            let fo = FailOn::from_str(s)?;
            Ok(if !matches!(fo, FailOn::Never) && !empty {
                1
            } else {
                0
            })
        }
    }
}

/// Compatibility-first preflight and conservative repair for existing bundles.
pub fn run_doctor(args: &DoctorArgs, json_output: bool) -> Result<i32> {
    let root = resolve_bundle(args.bundle.as_deref())?;
    let apply = args.fix_safe && args.yes && !args.dry_run;
    let report = doctor(&root, &args.target, args.fix_safe, apply)?;
    if json_output {
        for finding in &report.findings {
            output::print_line(&json!({
                "kind": "doctor-finding",
                "id": finding.id,
                "rule": finding.rule,
                "path": finding.path,
                "severity": finding.severity.as_str(),
                "repair": finding.repair.as_str(),
                "message": finding.message,
                "current": finding.current,
                "target": finding.target,
                "proposed_action": finding.proposed_action,
                "applied": finding.applied,
            }))?;
        }
        output::print_line(&json!({
            "kind": "doctor-summary",
            "target": report.target,
            "ready": report.ready(),
            "files_inspected": report.files_inspected,
            "concepts_inspected": report.concepts_inspected,
            "findings": report.findings.len(),
            "applied": apply,
        }))?;
    } else {
        for finding in &report.findings {
            output::print_text_line(format_args!(
                "{}\t{}\t{}\t{}\t{}",
                finding.severity.as_str(),
                finding.id,
                finding.repair.as_str(),
                finding.path.as_deref().unwrap_or("-"),
                finding.message
            ))?;
        }
        output::print_text_line(format_args!(
            "doctor: {} file(s), {} concept(s), {} finding(s), ready={}",
            report.files_inspected,
            report.concepts_inspected,
            report.findings.len(),
            report.ready()
        ))?;
        if args.fix_safe && !apply {
            eprintln!("safe fixes were dry-run only; pass --fix-safe --yes to apply");
        }
    }
    Ok(if report.ready() { 0 } else { 1 })
}

pub fn lint_severity(
    value: Option<&str>,
    fallback: okf_core::check::lint::Severity,
) -> Result<Option<okf_core::check::lint::Severity>> {
    use okf_core::check::lint::Severity;
    match value {
        None => Ok(Some(fallback)),
        Some("off") => Ok(None),
        Some("info") => Ok(Some(Severity::Info)),
        Some("warn") => Ok(Some(Severity::Warn)),
        Some("error") => Ok(Some(Severity::Error)),
        Some(value) => Err(OkfError::Usage(format!("invalid lint severity {value:?}"))),
    }
}
pub fn lint_config(settings: &okf_core::bundle::settings::BundleSettings) -> Result<LintConfig> {
    let mut config = LintConfig::default();
    config.broken_link = lint_severity(
        settings.lint.broken_link.as_deref(),
        okf_core::check::lint::Severity::Error,
    )?;
    config.frontmatter_order = lint_severity(
        settings.lint.frontmatter_order.as_deref(),
        okf_core::check::lint::Severity::Warn,
    )?;
    config.missing_title = lint_severity(
        settings.lint.missing_title.as_deref(),
        okf_core::check::lint::Severity::Warn,
    )?;
    config.missing_description = lint_severity(
        settings.lint.missing_description.as_deref(),
        okf_core::check::lint::Severity::Info,
    )?;
    config.orphan = lint_severity(
        settings.lint.orphan.as_deref(),
        okf_core::check::lint::Severity::Info,
    )?;
    config.ontology_violation = lint_severity(
        settings.lint.ontology_violation.as_deref(),
        okf_core::check::lint::Severity::Warn,
    )?;
    config.spec_v02 = lint_severity(
        settings.lint.spec_v02.as_deref(),
        okf_core::check::lint::Severity::Warn,
    )?;
    config.source_missing = lint_severity(
        settings.lint.source_missing.as_deref(),
        okf_core::check::lint::Severity::Error,
    )?;
    config.source_unrecorded = lint_severity(
        settings.lint.source_unrecorded.as_deref(),
        okf_core::check::lint::Severity::Warn,
    )?;
    config.finding_budget = settings.lint.finding_budget.unwrap_or(1000);
    Ok(config)
}
