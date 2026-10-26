//! Advisory health checks for fingerprinted sources. Standard kind-less provenance remains
//! valid without fingerprints. Local file/text sources are checked for readability even
//! before their first refresh; lint does not fetch URL sources or require a Git worktree.
use crate::check::lint::{Finding, RuleContext};
use crate::fingerprint::{Engine, Fingerprinter};
use crate::model::source::{Source, SourceKind};
use crate::ports::fs::RealFs;
use crate::ports::git::RealGit;

pub const UNRECORDED_RULE: &str = "source-unrecorded";
pub const MISSING_RULE: &str = "source-missing";
pub const UNAVAILABLE_RULE: &str = "source-unavailable";
pub const UNSUPPORTED_RULE: &str = "source-unsupported";

pub fn run(ctx: &RuleContext) -> Vec<Finding> {
    let fs = RealFs;
    let git = RealGit;
    let engine = Engine::new(&ctx.bundle.root, &fs, &git, None);
    let mut findings = Vec::new();
    for concept in &ctx.bundle.concepts {
        let Some(value) = concept.frontmatter.get("sources") else {
            continue;
        };
        let Some(entries) = value.as_sequence() else {
            if let Some(severity) = ctx.config.source_missing {
                findings.push(Finding {
                    code: Some("source-unavailable".into()),
                    field_path: Some("sources".into()),
                    rule: UNAVAILABLE_RULE.into(),
                    severity,
                    concept: Some(concept.id.0.clone()),
                    message: "sources is not a list".into(),
                });
            }
            continue;
        };
        for (index, entry) in entries.iter().enumerate() {
            if let Some(message) = crate::check::stale::malformed_source_entry(entry, index) {
                if let Some(severity) = ctx.config.source_missing {
                    findings.push(Finding {
                        code: Some("source-unavailable".into()),
                        field_path: Some(format!("sources[{index}]")),
                        rule: UNAVAILABLE_RULE.into(),
                        severity,
                        concept: Some(concept.id.0.clone()),
                        message,
                    });
                }
                continue;
            }
            let source = Source::from_value(entry).expect("shape checked above");
            if source.kind.as_kind_str().trim().is_empty() {
                continue;
            }
            if matches!(source.kind, SourceKind::Other(_)) {
                if let Some(severity) = ctx.config.source_missing {
                    findings.push(Finding {
                        code: Some("source-unsupported".into()),
                        field_path: Some("sources".into()),
                        rule: UNSUPPORTED_RULE.into(),
                        severity,
                        concept: Some(concept.id.0.clone()),
                        message: format!(
                            "source {} declares unsupported fingerprint kind {}",
                            source.resource,
                            source.kind.as_kind_str()
                        ),
                    });
                }
                continue;
            }
            if source.fingerprint.is_empty() {
                if let Some(severity) = ctx.config.source_unrecorded {
                    findings.push(Finding {
                        code: None,
                        field_path: None,
                        rule: UNRECORDED_RULE.to_string(),
                        severity,
                        concept: Some(concept.id.0.clone()),
                        message: format!(
                            "source {} has no recorded fingerprint; run refresh after reviewing the source",
                            source.resource
                        ),
                    });
                }
            }
            if matches!(
                source.kind,
                SourceKind::File | SourceKind::LineRange | SourceKind::MarkdownHeading
            ) {
                if let Some(severity) = ctx.config.source_missing {
                    if let Err(error) = engine.fingerprint(&source) {
                        let missing = error
                            .to_string()
                            .to_ascii_lowercase()
                            .contains("no such file");
                        findings.push(Finding {
                            code: Some(
                                if missing {
                                    "source-missing"
                                } else {
                                    "source-unavailable"
                                }
                                .into(),
                            ),
                            field_path: Some("sources".into()),
                            rule: if missing {
                                MISSING_RULE
                            } else {
                                UNAVAILABLE_RULE
                            }
                            .into(),
                            severity,
                            concept: Some(concept.id.0.clone()),
                            message: format!(
                                "cannot fingerprint source {}: {error}",
                                source.resource
                            ),
                        });
                    }
                }
            }
        }
    }
    findings
}
