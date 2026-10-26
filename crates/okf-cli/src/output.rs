//! Render core results as human text or NDJSON (`--json`).
//!
//! NDJSON records are `serde_json::Value` objects (built with `preserve_order`), one per
//! output line. A `concept` record mirrors the concept's frontmatter verbatim plus computed
//! `id`/`trust_tier` (via [`concept_record`]); other record types have documented shapes.
use okf_core::error::Result;
use okf_core::model::concept::Concept;
use okf_core::output::record::concept_record;
use okf_core::parse::writer;
use okf_core::query::search::SearchHit;
use serde_json::Value as Json;
use std::cell::RefCell;
use std::io::{BufWriter, Write};

thread_local! {
    static STDOUT: RefCell<BufWriter<std::io::Stdout>> =
        RefCell::new(BufWriter::with_capacity(64 * 1024, std::io::stdout()));
    static OPERATION: RefCell<Option<Operation>> = const { RefCell::new(None) };
}

#[derive(Debug)]
struct Operation {
    command: String,
    publication: &'static str,
    track_publication: bool,
    consistency: &'static str,
    examination: &'static str,
    matching: &'static str,
    delivery: &'static str,
    expansion: &'static str,
    content: &'static str,
    reasons: Vec<String>,
    commit_reached: Option<Option<bool>>,
}

/// Start one versioned JSON operation stream after argument parsing succeeds.
pub fn begin(command: &str, json: bool, write: bool, revision: bool) -> Result<()> {
    if !json {
        return Ok(());
    }
    let matching = if matches!(command, "list" | "search") {
        "complete"
    } else {
        "not-applicable"
    };
    let publication_applicable = write
        || command.starts_with("changeset ")
        || matches!(
            command,
            "init" | "add" | "edit" | "mv" | "rm" | "verify" | "refresh" | "artifact put"
        )
        || command.starts_with("ontology ")
            && !matches!(
                command,
                "ontology list"
                    | "ontology show"
                    | "ontology field-type list"
                    | "ontology field-type show"
            );
    OPERATION.with(|state| {
        *state.borrow_mut() = Some(Operation {
            command: command.to_string(),
            publication: if publication_applicable {
                "not-published"
            } else {
                "not-applicable"
            },
            track_publication: write,
            consistency: if publication_applicable {
                "guarded-baseline"
            } else if revision {
                "immutable-revision"
            } else {
                "working-tree"
            },
            examination: "complete",
            matching,
            delivery: "complete",
            expansion: "not-applicable",
            content: "not-applicable",
            reasons: Vec::new(),
            commit_reached: None,
        })
    });
    raw_line(
        &serde_json::json!({"kind":"operation-start","schema_version":1,"command":command,"stream_version":1}),
    )
}

pub fn mark_committed() {
    OPERATION.with(|state| {
        if let Some(op) = state.borrow_mut().as_mut() {
            op.publication = "committed";
        }
    });
}

pub fn mark_recovery_required(commit_reached: Option<bool>) {
    OPERATION.with(|state| {
        if let Some(op) = state.borrow_mut().as_mut() {
            op.publication = "recovery-required";
            op.commit_reached = Some(commit_reached);
        }
    });
}

pub fn mark_partial(dimension: &str, reason: &str) {
    OPERATION.with(|state| {
        if let Some(op) = state.borrow_mut().as_mut() {
            match dimension {
                "examination" => {
                    op.examination = "partial";
                    if op.matching == "complete" {
                        op.matching = "partial";
                    }
                }
                "matching" => op.matching = "partial",
                "delivery" => op.delivery = "partial",
                "expansion" => op.expansion = "partial",
                "content" => op.content = "partial",
                _ => return,
            }
            if !op.reasons.iter().any(|existing| existing == reason) {
                op.reasons.push(reason.into());
            }
        }
    });
}

pub fn mark_content_complete() {
    OPERATION.with(|state| {
        if let Some(op) = state.borrow_mut().as_mut() {
            op.content = "complete";
        }
    });
}

pub fn mark_failure(code: i32, reason: &str) {
    OPERATION.with(|state| {
        if let Some(op) = state.borrow_mut().as_mut() {
            if code >= 2 {
                op.examination = "not-applicable";
                op.matching = "not-applicable";
                op.expansion = "not-applicable";
                op.content = "not-applicable";
                if reason != "baseline-conflict" && op.consistency == "guarded-baseline" {
                    op.consistency = "working-tree";
                }
            }
            if !op.reasons.iter().any(|existing| existing == reason) {
                op.reasons.push(reason.into());
            }
        }
    });
}

/// Emit the last record. A payload error still gets a trailer if normal handling can run.
pub fn complete(exit_code: i32) -> Result<()> {
    let Some(mut op) = OPERATION.with(|state| state.borrow_mut().take()) else {
        return Ok(());
    };
    if op.track_publication {
        use okf_core::mutate::transaction::{publication_evidence, PublicationEvidence};
        match publication_evidence() {
            PublicationEvidence::Committed => op.publication = "committed",
            PublicationEvidence::RecoveryRequired { commit_reached } => {
                op.publication = "recovery-required";
                op.commit_reached = Some(Some(commit_reached));
            }
            PublicationEvidence::None | PublicationEvidence::NotPublished => {}
        }
    }
    let partial = [
        op.examination,
        op.matching,
        op.delivery,
        op.expansion,
        op.content,
    ]
    .contains(&"partial");
    let outcome = if exit_code != 0 {
        "failed"
    } else if partial {
        "partial"
    } else {
        "ok"
    };
    let mut record = serde_json::json!({
        "kind":"operation-complete","schema_version":1,"outcome":outcome,"exit_code":exit_code,
        "publication":op.publication,
        "completeness":{"examination":op.examination,"matching":op.matching,"delivery":op.delivery,"expansion":op.expansion,"content":op.content,"consistency":op.consistency},
        "reasons":op.reasons,
    });
    if let Some(commit) = op.commit_reached {
        record["commit_reached"] = serde_json::json!(commit);
    }
    raw_line(&record)
}

fn raw_line(value: &Json) -> Result<()> {
    STDOUT.with(|output| {
        let mut output = output.borrow_mut();
        serde_json::to_writer(&mut *output, value)
            .map_err(|error| okf_core::error::OkfError::Internal(error.to_string()))?;
        output.write_all(b"\n").map_err(Into::into)
    })
}

#[derive(Debug)]
struct Heading<'a> {
    line: usize,
    level: usize,
    text: &'a str,
}

/// Print one NDJSON record (object per line).
pub fn print_line(value: &Json) -> Result<()> {
    if value.get("kind").and_then(Json::as_str) == Some("query-summary")
        && value.get("scan_complete").and_then(Json::as_bool) == Some(false)
    {
        mark_partial("examination", "scan-limit");
    }
    if value.get("kind").and_then(Json::as_str) == Some("expansion-summary")
        && value.get("truncated").and_then(Json::as_bool) == Some(true)
    {
        mark_partial("expansion", "expansion-limit");
    }
    if value.get("kind").and_then(Json::as_str) == Some("scope")
        && value
            .get("unavailable")
            .and_then(Json::as_array)
            .is_some_and(|a| !a.is_empty())
    {
        mark_partial("examination", "scope-unavailable");
    }
    raw_line(value)
}

pub fn flush() -> Result<()> {
    STDOUT.with(|output| output.borrow_mut().flush().map_err(Into::into))
}

pub fn print_text(args: std::fmt::Arguments<'_>) -> Result<()> {
    STDOUT.with(|output| output.borrow_mut().write_fmt(args).map_err(Into::into))
}

pub fn print_text_line(args: std::fmt::Arguments<'_>) -> Result<()> {
    STDOUT.with(|output| {
        let mut output = output.borrow_mut();
        output.write_fmt(args)?;
        output.write_all(b"\n").map_err(Into::into)
    })
}

/// Print a sequence of NDJSON records.
pub fn print_lines(values: &[Json]) -> Result<()> {
    for v in values {
        print_line(v)?;
    }
    Ok(())
}

/// Print a list of concepts as either NDJSON `concept` records or a human table.
pub fn print_concepts(concepts: &[&Concept], json: bool) -> Result<()> {
    if json {
        for c in concepts {
            print_line(&concept_record(c))?;
        }
        return Ok(());
    }

    if concepts.is_empty() {
        println!("(no concepts)");
        return Ok(());
    }

    let id_w = concepts
        .iter()
        .map(|c| c.id.0.len())
        .max()
        .unwrap_or(2)
        .max(2);
    let ty_w = concepts
        .iter()
        .map(|c| c.concept_type().unwrap_or("-").len())
        .max()
        .unwrap_or(4)
        .max(4);
    let tier_w = 17; // widest tier string, "machine-confirmed"
    let status_w = concepts
        .iter()
        .map(|c| c.effective_status().len())
        .max()
        .unwrap_or(6)
        .max(6);

    let stdout = std::io::stdout();
    let mut output = BufWriter::with_capacity(64 * 1024, stdout.lock());
    writeln!(
        output,
        "{:<id_w$}  {:<ty_w$}  {:<tier_w$}  {}",
        "ID",
        "TYPE",
        "TRUST",
        format_args!("{:<status_w$}  {}", "STATUS", "TITLE")
    )?;
    for c in concepts {
        writeln!(
            output,
            "{:<id_w$}  {:<ty_w$}  {:<tier_w$}  {}",
            c.id.0,
            c.concept_type().unwrap_or("-"),
            c.trust_tier().as_str(),
            format_args!(
                "{:<status_w$}  {}",
                c.effective_status(),
                c.title().unwrap_or("")
            ),
        )?;
    }
    output.flush().map_err(Into::into)
}

/// Print ranked search hits. Human output retains the concept table; NDJSON adds bounded
/// evidence under a computed search object while preserving a conflicting frontmatter value.
pub fn print_search_hits(hits: &[SearchHit<'_>], json: bool) -> Result<()> {
    if json {
        for hit in hits {
            let matches: Vec<Json> = hit
                .matches
                .iter()
                .map(|evidence| {
                    serde_json::json!({
                        "query": evidence.query,
                        "field": evidence.field,
                        "line": evidence.line,
                        "snippet": evidence.snippet,
                    })
                })
                .collect();
            let mut record = concept_record(hit.concept);
            if let Json::Object(object) = &mut record {
                let search = serde_json::json!({
                    "score": hit.score,
                    "matches": matches,
                });
                if let Some(conflict) = object.insert("search".to_string(), search) {
                    let previous = object.remove("frontmatter_conflicts");
                    let mut conflicts = match previous {
                        Some(Json::Object(conflicts)) => conflicts,
                        Some(value) => {
                            let mut conflicts = serde_json::Map::new();
                            conflicts.insert("frontmatter_conflicts".to_string(), value);
                            conflicts
                        }
                        None => serde_json::Map::new(),
                    };
                    conflicts.insert("search".to_string(), conflict);
                    object.insert("frontmatter_conflicts".to_string(), Json::Object(conflicts));
                }
            }
            print_line(&record)?;
        }
        return Ok(());
    }

    let concepts: Vec<&Concept> = hits.iter().map(|hit| hit.concept).collect();
    print_concepts(&concepts, false)
}

/// Print a concept metadata record in JSON mode, or its full document in human mode.
pub fn print_concept(concept: &Concept, json: bool) -> Result<()> {
    if json {
        return print_line(&concept_record(concept));
    }
    println!("# {}", concept.id.0);
    println!("trust_tier: {}", concept.trust_tier().as_str());
    println!();
    print!("{}", writer::write_concept(concept)?);
    Ok(())
}

/// Print only the Markdown heading structure, with physical line numbers in the serialized
/// concept document. ATX-like text inside fenced code blocks is ignored.
pub fn print_concept_outline(concept: &Concept, json: bool) -> Result<()> {
    let source = writer::write_concept(concept)?;
    let headings = headings(&source);
    if json {
        mark_partial("content", "outline-only");
        let outline: Vec<Json> = headings
            .iter()
            .map(|h| serde_json::json!({"line": h.line, "level": h.level, "text": h.text}))
            .collect();
        return print_line(&serde_json::json!({
            "kind": "outline",
            "id": concept.id.0,
            "trust_tier": concept.trust_tier().as_str(),
            "headings": outline,
        }));
    }
    println!("# {}", concept.id.0);
    println!("trust_tier: {}", concept.trust_tier().as_str());
    println!();
    for h in headings {
        println!("{}: {} {}", h.line, "#".repeat(h.level), h.text);
    }
    Ok(())
}

/// Print an inclusive, 1-based slice of the serialized concept document.
pub fn print_concept_lines(concept: &Concept, start: usize, end: usize, json: bool) -> Result<()> {
    let source = writer::write_concept(concept)?;
    let total = source.lines().count();
    if start > total {
        return Err(okf_core::error::OkfError::Usage(format!(
            "--lines starts at {start}, but {} has only {total} lines",
            concept.id.0
        )));
    }
    let actual_end = end.min(total);
    let selected: Vec<(usize, &str)> = source
        .lines()
        .enumerate()
        .filter_map(|(i, line)| {
            let n = i + 1;
            (n >= start && n <= actual_end).then_some((n, line))
        })
        .collect();
    if json {
        if start > 1 || actual_end < total {
            mark_partial("content", "line-range");
        } else {
            mark_content_complete();
        }
        let lines: Vec<Json> = selected
            .iter()
            .map(|(line, text)| serde_json::json!({"line": line, "text": text}))
            .collect();
        return print_line(&serde_json::json!({
            "kind": "line-range",
            "id": concept.id.0,
            "trust_tier": concept.trust_tier().as_str(),
            "start": start,
            "end": actual_end,
            "lines": lines,
        }));
    }
    for (line, text) in selected {
        println!("{line}: {text}");
    }
    Ok(())
}

fn headings(source: &str) -> Vec<Heading<'_>> {
    let mut result = Vec::new();
    let mut in_frontmatter = false;
    let mut frontmatter_closed = false;
    let mut in_fence = false;
    for (i, line) in source.lines().enumerate() {
        if i == 0 && line.trim_end() == "---" {
            in_frontmatter = true;
            continue;
        }
        if in_frontmatter {
            if line.trim_end() == "---" {
                in_frontmatter = false;
                frontmatter_closed = true;
            }
            continue;
        }
        if !frontmatter_closed {
            continue;
        }
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        let level = trimmed.chars().take_while(|c| *c == '#').count();
        if level == 0 || level > 6 {
            continue;
        }
        let rest = &trimmed[level..];
        if !rest.is_empty() && !rest.starts_with([' ', '\t']) {
            continue;
        }
        result.push(Heading {
            line: i + 1,
            level,
            text: rest.trim().trim_end_matches('#').trim_end(),
        });
    }
    result
}
