//! Compatibility-first diagnostics for upgrading existing bundles to corrected OKF v0.2 behavior.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::fingerprint::canonicalize::sha256_hex;
use ignore::WalkBuilder;

use crate::bundle::loader::Bundle;
use crate::bundle::walk::walk_markdown;
use crate::check::lint::{lint_bundle, LintConfig};
use crate::check::validate::{validate_document, ValidateRule, Violation};
use crate::error::{OkfError, Result};
use crate::model::concept::ConceptId;
use crate::model::link::{classify, raw_links, resolve_link, LinkKind};
use crate::parse::{markdown::split_frontmatter, parse_concept};
use crate::query::artifact::{ArtifactKind, ResolvedArtifact};
use serde_yaml::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoctorSeverity {
    Blocker,
    BehaviorChange,
    Warning,
    Info,
}

impl DoctorSeverity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Blocker => "blocker",
            Self::BehaviorChange => "behavior-change",
            Self::Warning => "warning",
            Self::Info => "info",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepairClass {
    Safe,
    Review,
    Manual,
}

impl RepairClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Safe => "safe",
            Self::Review => "review",
            Self::Manual => "manual",
        }
    }
}

#[derive(Debug, Clone)]
pub struct DoctorFinding {
    pub id: String,
    pub rule: String,
    pub path: Option<String>,
    pub severity: DoctorSeverity,
    pub repair: RepairClass,
    pub message: String,
    pub current: Option<String>,
    pub target: Option<String>,
    pub proposed_action: Option<String>,
    pub applied: bool,
}

#[derive(Debug, Clone)]
pub struct DoctorReport {
    pub target: String,
    pub findings: Vec<DoctorFinding>,
    pub concepts_inspected: usize,
    pub files_inspected: usize,
}

impl DoctorReport {
    pub fn ready(&self) -> bool {
        !self.findings.iter().any(|f| {
            matches!(
                f.severity,
                DoctorSeverity::Blocker | DoctorSeverity::BehaviorChange | DoctorSeverity::Warning
            ) && !f.applied
        })
    }
}

pub fn doctor(root: &Path, target: &str, fix_safe: bool, apply: bool) -> Result<DoctorReport> {
    let context = crate::bundle::context::Context {
        primary: crate::bundle::context::SelectedBundle {
            id: None,
            root: root.canonicalize()?,
        },
        catalog: None,
        config: Default::default(),
        config_path: None,
    };
    doctor_in_context(&context, &[], target, fix_safe, apply)
}

/// Diagnose the primary bundle while resolving resources only within explicit scope.
pub fn doctor_in_context(
    context: &crate::bundle::context::Context,
    scope_ids: &[String],
    target: &str,
    fix_safe: bool,
    apply: bool,
) -> Result<DoctorReport> {
    let root = &context.primary.root;
    if target != "0.2" {
        return Err(OkfError::Usage(format!(
            "doctor: unsupported target {target:?}; expected 0.2"
        )));
    }
    let guard = if fix_safe {
        Some(crate::mutate::transaction::writer_lock(root)?)
    } else {
        None
    };
    let mut changes = Vec::new();
    let paths = walk_markdown(root)?;
    let mut findings = Vec::new();
    if let Err(error) = crate::bundle::manifest::validate_resources(root) {
        push(
            &mut findings,
            "manifest-invalid",
            Some(Path::new("okf-bundle.yaml")),
            DoctorSeverity::Blocker,
            RepairClass::Review,
            &error.to_string(),
            None,
            Some("repair the manifest before manifest-dependent work"),
            None,
            false,
        );
    }
    if root.join(".okf-bundle").is_dir() {
        push(&mut findings, "bundle-profile-support", Some(Path::new(".okf-bundle")),
             DoctorSeverity::Info, RepairClass::Manual,
             "support files are excluded by the bundle tooling profile; core-only readers may treat support Markdown as concepts",
             None, None, Some("use validate --profile bundle for the tooling boundary"), false);
    }
    if let Some(config) = crate::bundle::config::find_config(root) {
        push(&mut findings, "legacy-configuration", Some(&config), DoctorSeverity::Info,
             RepairClass::Review, "legacy TOML configuration can be migrated through an explicit plan",
             None, None, Some("plan configuration migration; ordinary doctor safe fixes do not migrate configuration"), false);
    }
    let mut documents: HashMap<PathBuf, Option<String>> = HashMap::new();
    for path in &paths {
        let bytes = std::fs::read(root.join(path))
            .map_err(|error| OkfError::Io(format!("{}: {error}", root.join(path).display())))?;
        documents.insert(path.clone(), String::from_utf8(bytes).ok());
    }

    // The old loader honored Git ignore files; make newly visible documents explicit.
    let legacy: HashSet<PathBuf> = WalkBuilder::new(root)
        .hidden(false)
        .git_ignore(true)
        .git_exclude(true)
        .parents(true)
        .build()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_file())
        .filter_map(|entry| entry.path().strip_prefix(root).ok().map(Path::to_path_buf))
        .collect();
    for path in &paths {
        if !legacy.contains(path) {
            push(
                &mut findings,
                "newly-visible-markdown",
                Some(path),
                DoctorSeverity::BehaviorChange,
                RepairClass::Review,
                "corrected OKF traversal includes this Git-ignored Markdown file",
                Some("ignored by current loader"),
                Some("included in bundle tree"),
                Some(
                    "review the file as a concept/reserved document or move it outside the bundle",
                ),
                false,
            );
        }
    }

    // A whitespace-only index is a known safe repair: no authored body content is discarded.
    if fix_safe {
        for path in &paths {
            if path.file_name().and_then(|s| s.to_str()) != Some("index.md") {
                continue;
            }
            let Some(Some(content)) = documents.get(path) else {
                continue;
            };
            let (frontmatter, body) = split_frontmatter(content);
            if body.trim().is_empty() {
                let replacement = match frontmatter {
                    Some(fm) => format!(
                        "---\n{fm}---\n# Concepts\n\nNo concepts are present in this directory.\n"
                    ),
                    None => {
                        "# Concepts\n\nNo concepts are present in this directory.\n".to_string()
                    }
                };
                if apply {
                    changes.push(crate::mutate::transaction::FileChange {
                        path: path.clone(),
                        before: Some(content.as_bytes().to_vec()),
                        after: Some(replacement.as_bytes().to_vec()),
                    });
                    documents.insert(path.clone(), Some(replacement));
                }
                push(
                    &mut findings,
                    "empty-index-safe-fix",
                    Some(path),
                    DoctorSeverity::Info,
                    RepairClass::Safe,
                    "replace a whitespace-only index with a valid empty Concepts section",
                    Some("empty index body"),
                    Some("headed empty index"),
                    Some("okf doctor --fix-safe --yes"),
                    apply,
                );
            }
        }
    }

    let mut violations = Vec::new();
    for path in &paths {
        let rel = path.to_string_lossy().replace('\\', "/");
        match documents.get(path).and_then(Option::as_deref) {
            Some(content) => violations.extend(validate_document(&rel, content)),
            None => violations.push(Violation {
                file: rel,
                rule: ValidateRule::InvalidUtf8,
                message: "Markdown document is not valid UTF-8".to_string(),
            }),
        }
    }
    violations.sort_by(|a, b| {
        a.file
            .cmp(&b.file)
            .then_with(|| a.rule.as_str().cmp(b.rule.as_str()))
    });
    for violation in violations {
        let repair = match violation.rule {
            ValidateRule::InvalidIndex if violation.message.contains("at least one") => {
                RepairClass::Review
            }
            _ => RepairClass::Manual,
        };
        push(
            &mut findings,
            violation.rule.as_str(),
            Some(Path::new(&violation.file)),
            DoctorSeverity::Blocker,
            repair,
            &violation.message,
            Some("not conformant"),
            Some("OKF v0.2 conformant"),
            None,
            false,
        );
    }

    // Build a partial bundle from every independently parseable concept, even when siblings fail.
    let mut concepts = Vec::new();
    for path in &paths {
        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if matches!(name, "index.md" | "log.md") {
            continue;
        }
        let Some(Some(content)) = documents.get(path) else {
            continue;
        };
        let stem = path
            .to_string_lossy()
            .strip_suffix(".md")
            .unwrap_or_default()
            .to_string();
        if let Ok(concept) = parse_concept(ConceptId::from_relative(&stem), content) {
            concepts.push(concept);
        }
    }
    concepts.sort_by(|a, b| a.id.0.cmp(&b.id.0));
    let bundle = Bundle {
        root: root.to_path_buf(),
        concepts,
    };

    for finding in lint_bundle(&bundle, None, &LintConfig::default())
        .into_iter()
        .filter(|finding| finding.rule == "okf-v02")
    {
        push(
            &mut findings,
            "optional-family",
            finding
                .concept
                .as_deref()
                .map(|id| Path::new(id.trim_start_matches('/'))),
            DoctorSeverity::Warning,
            RepairClass::Review,
            &finding.message,
            Some("current optional metadata"),
            Some("well-formed OKF v0.2 optional metadata"),
            None,
            false,
        );
    }

    for concept in &bundle.concepts {
        let mut resources: Vec<(String, String, Option<&Value>)> = Vec::new();
        if let Some(resource) = concept.frontmatter.get_str("resource") {
            resources.push(("resource".to_string(), resource.to_string(), None));
        }
        if let Some(Value::Sequence(sources)) = concept.frontmatter.get("sources") {
            for (index, source) in sources.iter().enumerate() {
                let kind = source.get("kind").and_then(Value::as_str).unwrap_or("");
                // Git-root source kinds are explicitly tool-scoped and need not stay in bundle.
                if matches!(kind, "git-path" | "git-commit") {
                    continue;
                }
                if let Some(resource) = source.get("resource").and_then(Value::as_str) {
                    resources.push((
                        format!("sources[{index}].resource"),
                        resource.to_string(),
                        Some(source),
                    ));
                }
            }
        }
        if let Some(resource) = concept.frontmatter.get_str("computation") {
            resources.push(("computation".to_string(), resource.to_string(), None));
        }
        for family in ["executor", "attester"] {
            if let Some(resource) = concept
                .frontmatter
                .get(family)
                .and_then(|v| v.get("resource"))
                .and_then(Value::as_str)
            {
                resources.push((format!("{family}.resource"), resource.to_string(), None));
            }
        }
        for (field, resource, source) in resources {
            let resolution = if let Some(source) = source {
                crate::query::artifact::resolve_source_in_context(
                    context,
                    scope_ids,
                    &concept.id.0,
                    source,
                )
            } else {
                crate::query::artifact::resolve_artifact_in_context(
                    context,
                    scope_ids,
                    Some(&concept.id.0),
                    &resource,
                )
            };
            let resolved = match resolution {
                Ok((_, resolved)) => resolved,
                Err(error) => ResolvedArtifact {
                    resource: resource.clone(),
                    kind: ArtifactKind::Blocked,
                    path: None,
                    exists: false,
                    size: None,
                    message: Some(error.to_string()),
                },
            };
            if matches!(resolved.kind, ArtifactKind::Missing | ArtifactKind::Blocked) {
                push(
                    &mut findings,
                    "artifact-unavailable",
                    Some(Path::new(concept.id.0.trim_start_matches('/'))),
                    DoctorSeverity::Warning,
                    RepairClass::Review,
                    &format!(
                        "{field} {resource:?} resolves as {}{}",
                        resolved.kind.as_str(),
                        resolved
                            .message
                            .as_deref()
                            .map(|m| format!(": {m}"))
                            .unwrap_or_default()
                    ),
                    Some(&resource),
                    Some("retrievable artifact, external URI, or explicit scope descriptor"),
                    Some("confirm the declaring concept and repair the path without inventing provenance"),
                    false,
                );
            }
        }
        // Typed sources have their own namespace, not legacy Markdown link semantics.
        let mut markdown_links = concept.clone();
        if let Some(Value::Sequence(sources)) = markdown_links.frontmatter.map.get_mut("sources") {
            sources.retain(|source| source.get("kind").is_none());
        }
        for raw in raw_links(&markdown_links, None) {
            if classify(&raw) != LinkKind::Bare {
                continue;
            }
            let core = raw.split(['#', '?']).next().unwrap_or(&raw);
            let old = ConceptId::from_relative(core.strip_suffix(".md").unwrap_or(core));
            let new = resolve_link(&concept.id, &raw);
            if old != new {
                push(
                    &mut findings,
                    "relative-link-semantics",
                    Some(Path::new(concept.id.0.trim_start_matches('/'))),
                    DoctorSeverity::BehaviorChange,
                    RepairClass::Review,
                    &format!("bare relative link {raw:?} changes target"),
                    Some(&old.0),
                    Some(&new.0),
                    Some("rewrite the link explicitly after confirming intent"),
                    false,
                );
            }
        }
        if concept.frontmatter.get("last_modified").is_some() {
            push(
                &mut findings,
                "legacy-top-level-last-modified",
                Some(Path::new(concept.id.0.trim_start_matches('/'))),
                DoctorSeverity::Info,
                RepairClass::Review,
                "top-level last_modified is a preserved extension, not sources[].last_modified",
                None,
                None,
                None,
                false,
            );
        }
        if concept.frontmatter.get("generated").is_none()
            && concept.frontmatter.get("timestamp").is_some()
        {
            push(
                &mut findings,
                "legacy-timestamp",
                Some(Path::new(concept.id.0.trim_start_matches('/'))),
                DoctorSeverity::Info,
                RepairClass::Review,
                "v0.1 timestamp is consumed as a fallback; migrate only when producer identity is known",
                Some("timestamp"),
                Some("generated: {by, at}"),
                None,
                false,
            );
        }
        if concept
            .body
            .lines()
            .any(|line| line.trim() == "# Citations")
        {
            push(
                &mut findings,
                "legacy-citations",
                Some(Path::new(concept.id.0.trim_start_matches('/'))),
                DoctorSeverity::Info,
                RepairClass::Review,
                "v0.1 # Citations may be migrated to sources without inventing metadata",
                Some("# Citations"),
                Some("sources frontmatter and keyed footnotes"),
                None,
                false,
            );
        }
        if concept.concept_type() == Some("Computation")
            && ["runtime", "computation", "executor", "attester"]
                .iter()
                .any(|key| concept.frontmatter.get(key).is_some())
        {
            push(
                &mut findings,
                "custom-computation",
                Some(Path::new(concept.id.0.trim_start_matches('/'))),
                DoctorSeverity::Warning,
                RepairClass::Manual,
                "custom Computation remains valid but is not an OKF Attested Computation",
                Some("type: Computation"),
                Some("type: Attested Computation with reviewed contract, if intended"),
                None,
                false,
            );
        }
    }

    let mut ontology_path = root.join("ontology.yaml");
    let configured_ontology = (|| {
        let context = crate::bundle::context::current_context(Some(&root.to_string_lossy()), None)?;
        if let Some(path) = crate::bundle::settings::ontology_path_for(&context, root)? {
            ontology_path = path;
            crate::ontology::load::load_ontology(&ontology_path).map(Some)
        } else {
            Ok(None)
        }
    })();
    let ontology_finding_path = ontology_path.strip_prefix(root).unwrap_or(&ontology_path);
    match configured_ontology {
        Ok(Some(ontology)) => {
            for (name, concept_type) in ontology.concepts {
                if concept_type.attested && name != "Attested Computation" {
                    push(
                        &mut findings,
                        "custom-attested-ontology-type",
                        Some(ontology_finding_path),
                        DoctorSeverity::Warning,
                        RepairClass::Manual,
                        &format!("ontology type {name:?} is marked attested but is not the exact standard type"),
                        Some(&format!("{name}: attested: true")),
                        Some("custom type remains valid; convert only with a reviewed standard contract"),
                        None,
                        false,
                    );
                }
            }
        }
        Ok(None) => {}
        Err(error) => push(
            &mut findings,
            "ontology-unreadable",
            Some(ontology_finding_path),
            DoctorSeverity::Warning,
            RepairClass::Manual,
            &error.to_string(),
            None,
            Some("readable optional tool ontology or remove it from tool configuration"),
            None,
            false,
        ),
    }

    for finding in &mut findings {
        let identity = format!(
            "{}\0{}\0{}",
            finding.rule,
            finding.path.as_deref().unwrap_or(""),
            finding.message
        );
        let digest = sha256_hex(identity.as_bytes());
        finding.id = format!("DOC-{}", &digest[..12]);
    }
    if apply && fix_safe {
        crate::mutate::transaction::commit_locked(root, guard.as_ref().unwrap(), changes)?;
    }
    Ok(DoctorReport {
        target: target.to_string(),
        concepts_inspected: bundle.concepts.len(),
        files_inspected: paths.len(),
        findings,
    })
}

#[allow(clippy::too_many_arguments)]
fn push(
    out: &mut Vec<DoctorFinding>,
    rule: &str,
    path: Option<&Path>,
    severity: DoctorSeverity,
    repair: RepairClass,
    message: &str,
    current: Option<&str>,
    target: Option<&str>,
    action: Option<&str>,
    applied: bool,
) {
    out.push(DoctorFinding {
        id: String::new(),
        rule: rule.to_string(),
        path: path.map(|p| p.to_string_lossy().into_owned()),
        severity,
        repair,
        message: message.to_string(),
        current: current.map(str::to_string),
        target: target.map(str::to_string),
        proposed_action: action.map(str::to_string),
        applied,
    });
}

#[cfg(test)]
mod manifest_tests {
    use super::*;
    #[test]
    fn malformed_manifest_remains_diagnosable_and_is_not_fixed() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(
            root.path().join("note.md"),
            "---\ntype: Note\n---\n# Note\n",
        )
        .unwrap();
        let invalid = "manifest_version: unsupported\n";
        std::fs::write(root.path().join("okf-bundle.yaml"), invalid).unwrap();
        let report = doctor(root.path(), "0.2", false, false).unwrap();
        assert!(report
            .findings
            .iter()
            .any(|f| f.rule == "manifest-invalid" && f.severity == DoctorSeverity::Blocker));
        assert!(!report.ready());
        assert_eq!(
            std::fs::read_to_string(root.path().join("okf-bundle.yaml")).unwrap(),
            invalid
        );
    }
    #[test]
    fn support_instructions_are_not_doctor_concepts() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join(".okf-bundle")).unwrap();
        std::fs::write(root.path().join(".okf-bundle/guide.md"), "Plain guidance").unwrap();
        let report = doctor(root.path(), "0.2", false, false).unwrap();
        assert_eq!(report.concepts_inspected, 0);
        assert!(report
            .findings
            .iter()
            .any(|f| f.rule == "bundle-profile-support"));
        assert!(!report.findings.iter().any(|f| f.rule == "missing-type"));
    }
}
