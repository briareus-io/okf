//! Read-only before/after impact evidence for an exact candidate dependency snapshot.
//! Contexts are built in memory; neither the accepted catalog nor its lock is rewritten.
use super::{
    catalog::{Catalog, CatalogEntry, CatalogFile},
    context::{Context, SelectedBundle},
    loader::load_bundle,
    remote::LockFile,
};
use crate::{
    error::{OkfError, Result},
    fingerprint::canonicalize::sha256_hex,
    graph::catalog::{build_catalog_graph, CatalogGraph},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FileDelta {
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub changed: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleDelta {
    pub bundle: String,
    pub before_root: Option<PathBuf>,
    pub after_root: Option<PathBuf>,
    pub concepts: FileDelta,
    pub ontology: FileDelta,
    pub manifest: FileDelta,
    pub support: FileDelta,
    pub other: FileDelta,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LintEvidence {
    pub rule: String,
    pub severity: String,
    pub concept: Option<String>,
    pub message: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkEvidence {
    pub source: String,
    pub resource: String,
    pub location: String,
    pub target_bundle: Option<String>,
    pub target: Option<String>,
    pub status: String,
    pub snapshot_status: Option<String>,
    pub fingerprint_status: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsumerImpact {
    pub bundle: String,
    pub root: PathBuf,
    pub digest: String,
    pub before_links: Vec<LinkEvidence>,
    pub after_links: Vec<LinkEvidence>,
    pub before_lint: Vec<LintEvidence>,
    pub after_lint: Vec<LintEvidence>,
    pub affected_concepts: Vec<String>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ImpactReport {
    pub complete: bool,
    pub reasons: Vec<String>,
    pub requested_consumers: Vec<PathBuf>,
    pub examined_before: Vec<String>,
    pub examined_after: Vec<String>,
    pub unavailable_before: Vec<String>,
    pub unavailable_after: Vec<String>,
    pub bundles: Vec<BundleDelta>,
    pub consumers: Vec<ConsumerImpact>,
}
fn digest(root: &Path) -> Result<String> {
    let files = super::walk::walk_files(root)?
        .into_iter()
        .map(|p| {
            Ok((
                p.to_string_lossy().to_string(),
                sha256_hex(&std::fs::read(root.join(p))?),
            ))
        })
        .collect::<Result<BTreeMap<_, _>>>()?;
    Ok(sha256_hex(
        &serde_json::to_vec(&files).map_err(|e| OkfError::Internal(e.to_string()))?,
    ))
}
fn inventory(root: Option<&Path>) -> Result<BTreeMap<String, String>> {
    match root {
        None => Ok(BTreeMap::new()),
        Some(root) => super::walk::walk_files(root)?
            .into_iter()
            .map(|p| {
                Ok((
                    p.to_string_lossy().replace('\\', "/"),
                    sha256_hex(&std::fs::read(root.join(p))?),
                ))
            })
            .collect(),
    }
}
fn delta(id: &str, old: Option<&Path>, new: Option<&Path>) -> Result<BundleDelta> {
    let before = inventory(old)?;
    let after = inventory(new)?;
    let mut out = BundleDelta {
        bundle: id.into(),
        before_root: old.map(Path::to_path_buf),
        after_root: new.map(Path::to_path_buf),
        concepts: FileDelta::default(),
        ontology: FileDelta::default(),
        manifest: FileDelta::default(),
        support: FileDelta::default(),
        other: FileDelta::default(),
    };
    for path in before.keys().chain(after.keys()).collect::<BTreeSet<_>>() {
        if before.get(path) == after.get(path) {
            continue;
        }
        let is_concept = path.ends_with(".md")
            && !path.starts_with(".okf-bundle/")
            && !matches!(
                Path::new(path).file_name().and_then(|s| s.to_str()),
                Some("index.md" | "log.md")
            );
        let category = if path == "ontology.yaml" {
            &mut out.ontology
        } else if path == "okf-bundle.yaml" {
            &mut out.manifest
        } else if path.starts_with(".okf-bundle/") {
            &mut out.support
        } else if is_concept {
            &mut out.concepts
        } else {
            &mut out.other
        };
        let value = if is_concept {
            format!("/{}", path.trim_end_matches(".md"))
        } else {
            path.to_string()
        };
        if !before.contains_key(path) {
            category.added.push(value);
        } else if !after.contains_key(path) {
            category.removed.push(value);
        } else {
            category.changed.push(value);
        }
    }
    Ok(out)
}

/// Exact old/new locks are already verified by the lifecycle planner. Materialization is
/// offline: missing old objects fail rather than silently substituting current content.
pub fn report(
    catalog: &Path,
    old: Option<&LockFile>,
    new: &LockFile,
    consumers: &[PathBuf],
) -> Result<ImpactReport> {
    let identity = |lock: &LockFile, id: &str| -> Option<(String, String, String, String)> {
        let b = lock.bundles.get(id)?;
        let r = lock.repositories.get(&b.repository)?;
        Some((
            r.url.clone(),
            r.commit.clone(),
            r.object_format.clone(),
            b.path.clone(),
        ))
    };
    let changed: BTreeSet<String> = old
        .into_iter()
        .flat_map(|l| l.bundles.keys())
        .chain(new.bundles.keys())
        .filter(|id| old.and_then(|l| identity(l, id)) != identity(new, id))
        .cloned()
        .collect();
    let roots = |lock: &LockFile| -> Result<BTreeMap<String, PathBuf>> {
        lock.bundles
            .iter()
            .map(|(id, b)| {
                let repo = lock
                    .repositories
                    .get(&b.repository)
                    .ok_or_else(|| OkfError::Usage("lock refers to unknown repository".into()))?;
                let root = if changed.contains(id) {
                    super::remote::snapshot_root(repo)?
                } else {
                    super::remote::snapshot_location(repo)?
                };
                Ok((id.clone(), root.join(&b.path)))
            })
            .collect()
    };
    let before = old.map(roots).transpose()?.unwrap_or_default();
    let after = roots(new)?;
    report_snapshots(catalog, &before, &after, consumers)
}

/// Also usable by tests and callers comparing already materialized snapshot roots.
pub fn report_snapshots(
    catalog: &Path,
    before: &BTreeMap<String, PathBuf>,
    after: &BTreeMap<String, PathBuf>,
    consumer_roots: &[PathBuf],
) -> Result<ImpactReport> {
    let base = catalog.parent().unwrap_or(Path::new("."));
    let mut entries = super::catalog::declared_directory_entries(catalog)?;
    let changed: Vec<_> = before
        .keys()
        .chain(after.keys())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .filter(|id| before.get(*id) != after.get(*id))
        .cloned()
        .collect();
    let mut consumers = Vec::new();
    for root in consumer_roots {
        let root = std::fs::canonicalize(root)?;
        if consumers.iter().any(|(_, r)| r == &root) {
            continue;
        }
        let remote_id = before
            .iter()
            .chain(after)
            .find(|(_, p)| std::fs::canonicalize(p).unwrap_or_else(|_| (*p).clone()) == root)
            .map(|(id, _)| id.clone());
        // Updated dependencies are comparison targets, not unchanged consumer baselines.
        if remote_id.as_ref().is_some_and(|id| changed.contains(id)) {
            continue;
        }
        let id = entries
            .iter()
            .find(|(_, e)| e.root == root)
            .map(|(id, _)| id.clone())
            .or(remote_id)
            .unwrap_or_else(|| format!("impact.consumer{}", consumers.len() + 1));
        entries.entry(id.clone()).or_insert(CatalogEntry {
            id: id.clone(),
            configured_root: root.clone(),
            root: root.clone(),
            overridden: false,
            available: true,
            settings: BTreeMap::new(),
        });
        consumers.push((id, root));
    }
    let mut out = ImpactReport {
        complete: true,
        requested_consumers: consumer_roots.to_vec(),
        ..Default::default()
    };
    for id in &changed {
        out.bundles.push(delta(
            id,
            before.get(id).map(PathBuf::as_path),
            after.get(id).map(PathBuf::as_path),
        )?);
    }
    if consumers.is_empty() {
        out.complete = false;
        out.reasons.push("consumer-impact-not-examined".into());
        return Ok(out);
    }
    // Legacy consumer settings remain tied to the selected workspace, never upstream discovery.
    let config_path = super::config::find_config(base);
    let config = config_path
        .as_ref()
        .map(|p| super::config::load_config(p))
        .transpose()?
        .unwrap_or_default();
    let make_context = |roots: &BTreeMap<String, PathBuf>| {
        let mut members = entries.clone();
        for (id, root) in roots {
            let root = std::fs::canonicalize(root).unwrap_or_else(|_| root.clone());
            members.insert(
                id.clone(),
                CatalogEntry {
                    id: id.clone(),
                    configured_root: root.clone(),
                    root: root.clone(),
                    overridden: false,
                    available: root.is_dir(),
                    settings: BTreeMap::new(),
                },
            );
        }
        Context {
            primary: SelectedBundle {
                id: Some(consumers[0].0.clone()),
                root: consumers[0].1.clone(),
            },
            catalog: Some(Catalog {
                path: catalog.into(),
                default_bundle: None,
                entries: members,
            }),
            config: config.clone(),
            config_path: config_path.clone(),
        }
    };
    let consumer_baselines = consumers
        .iter()
        .map(|(id, root)| Ok((id.clone(), digest(root)?)))
        .collect::<Result<BTreeMap<_, _>>>()?;
    let old_context = make_context(before);
    let new_context = make_context(after);
    let scope = |roots: &BTreeMap<String, PathBuf>| {
        consumers
            .iter()
            .map(|(id, _)| id.clone())
            .chain(changed.iter().filter(|id| roots.contains_key(*id)).cloned())
            .collect::<Vec<_>>()
    };
    let old_graph = build_catalog_graph(&old_context, &scope(before), false, None)?;
    let new_graph = build_catalog_graph(&new_context, &scope(after), false, None)?;
    out.examined_before = old_graph
        .scope
        .examined
        .iter()
        .map(|e| e.id.clone())
        .collect();
    out.examined_after = new_graph
        .scope
        .examined
        .iter()
        .map(|e| e.id.clone())
        .collect();
    out.unavailable_before = old_graph.scope.unavailable.clone();
    out.unavailable_after = new_graph.scope.unavailable.clone();
    if !out.unavailable_before.is_empty() || !out.unavailable_after.is_empty() {
        out.reasons.push("requested-bundle-unavailable".into());
    }
    for (id, root) in consumers {
        let baseline = consumer_baselines[&id].clone();
        let old_links = links(&old_graph, &id);
        let new_links = links(&new_graph, &id);
        if old_links.iter().chain(&new_links).any(|e| {
            matches!(
                e.status.as_str(),
                "out-of-scope"
                    | "unavailable-root"
                    | "unavailable-revision"
                    | "unavailable-root-or-revision"
            )
        }) {
            out.reasons.push(format!("unresolved-consumer-scope:{id}"));
        }
        let before_lint = lint(&old_context, &old_graph, &id, &root)?;
        let after_lint = lint(&new_context, &new_graph, &id, &root)?;
        let mut affected = BTreeSet::new();
        for link in old_links.iter().chain(&new_links) {
            if link
                .target_bundle
                .as_ref()
                .is_some_and(|b| changed.contains(b))
                || !old_links.contains(link)
                || !new_links.contains(link)
            {
                affected.insert(link.source.clone());
            }
        }
        if digest(&root)? != baseline {
            return Err(OkfError::Environment(
                "consumer changed during impact analysis".into(),
            ));
        }
        out.consumers.push(ConsumerImpact {
            bundle: id,
            root,
            digest: baseline,
            before_links: old_links,
            after_links: new_links,
            before_lint,
            after_lint,
            affected_concepts: affected.into_iter().collect(),
        });
    }
    out.reasons.sort();
    out.reasons.dedup();
    out.complete = out.reasons.is_empty();
    Ok(out)
}
fn links(graph: &CatalogGraph, id: &str) -> Vec<LinkEvidence> {
    graph
        .edges
        .iter()
        .filter(|e| e.source.bundle == id)
        .map(|e| LinkEvidence {
            source: e.source.id.clone(),
            resource: e.resource.clone(),
            location: e.location.clone(),
            target_bundle: e.target.as_ref().map(|n| n.bundle.clone()),
            target: e.target.as_ref().map(|n| n.id.clone()),
            status: e.status.clone(),
            snapshot_status: e.snapshot.as_ref().map(|s| s.status.clone()),
            fingerprint_status: e.fingerprint_status.clone(),
        })
        .collect()
}
fn severity(
    raw: Option<&str>,
    fallback: crate::check::lint::Severity,
) -> Option<crate::check::lint::Severity> {
    use crate::check::lint::Severity::*;
    match raw {
        Some("off") => None,
        Some("info") => Some(Info),
        Some("warn") => Some(Warn),
        Some("error") => Some(Error),
        _ => Some(fallback),
    }
}
fn lint(
    context: &Context,
    graph: &CatalogGraph,
    id: &str,
    root: &Path,
) -> Result<Vec<LintEvidence>> {
    use crate::check::lint::{lint_bundle, LintConfig, Severity};
    let effective = super::settings::for_context(context, root)?;
    let s = &effective.settings.lint;
    let ontology = effective
        .ontology_path
        .as_ref()
        .map(|p| crate::ontology::load::load_ontology(p))
        .transpose()?;
    let bundle = load_bundle(root)?;
    let config = LintConfig {
        broken_link: None, // Replaced by exact scoped graph evidence below, avoiding local-only false positives.
        missing_title: severity(s.missing_title.as_deref(), Severity::Warn),
        frontmatter_order: severity(s.frontmatter_order.as_deref(), Severity::Warn),
        missing_description: severity(s.missing_description.as_deref(), Severity::Info),
        orphan: None,             // Replaced with scoped inbound evidence below.
        ontology_violation: None, // Resolve target types across the exact comparison context.
        spec_v02: severity(s.spec_v02.as_deref(), Severity::Warn),
        source_missing: severity(s.source_missing.as_deref(), Severity::Error),
        source_unrecorded: severity(s.source_unrecorded.as_deref(), Severity::Warn),
        invalid_reference: severity(s.invalid_reference.as_deref(), Severity::Warn),
        finding_budget: usize::MAX, // Impact analysis is complete, not a display-budget-limited query.
    };
    let mut result: Vec<_> = lint_bundle(&bundle, ontology.as_ref(), &config)
        .into_iter()
        .chain(crate::check::lint::rules::invalid_reference::run(
            &bundle,
            ontology.as_ref(),
            context.catalog.as_ref(),
            config.invalid_reference,
        ))
        .map(|f| LintEvidence {
            rule: f.rule,
            severity: f.severity.as_str().into(),
            concept: f.concept,
            message: f.message,
        })
        .collect();
    if let Some(level) = severity(s.orphan.as_deref(), Severity::Info) {
        for concept in &bundle.concepts {
            if !graph.edges.iter().any(|e| {
                e.target
                    .as_ref()
                    .is_some_and(|t| t.bundle == id && t.id == concept.id.0)
            }) {
                result.push(LintEvidence {
                    rule: "orphan".into(),
                    severity: level.as_str().into(),
                    concept: Some(concept.id.0.clone()),
                    message: "no inbound references in the examined scope".into(),
                });
            }
        }
    }
    if let (Some(ontology), Some(level)) = (
        ontology.as_ref(),
        severity(s.ontology_violation.as_deref(), Severity::Warn),
    ) {
        let mut types = BTreeMap::new();
        for examined in &graph.scope.examined {
            for c in load_bundle(&examined.root)?.concepts {
                if let Some(ty) = c.concept_type() {
                    types.insert((examined.id.clone(), c.id.0.clone()), ty.to_string());
                }
            }
        }
        for concept in &bundle.concepts {
            let target_type = |raw: &str| {
                graph
                    .edges
                    .iter()
                    .find(|e| {
                        e.source.bundle == id && e.source.id == concept.id.0 && e.resource == raw
                    })
                    .and_then(|e| e.target.as_ref())
                    .and_then(|t| types.get(&(t.bundle.clone(), t.id.clone())))
                    .cloned()
                    .or_else(|| {
                        bundle
                            .get(&crate::model::link::resolve_link(&concept.id, raw).0)
                            .and_then(|c| c.concept_type().map(str::to_owned))
                    })
            };
            for v in crate::ontology::field_types::check_concept_with_budget(
                ontology,
                &concept.frontmatter,
                target_type,
                usize::MAX,
            ) {
                result.push(LintEvidence {
                    rule: "ontology-violation".into(),
                    severity: level.as_str().into(),
                    concept: Some(concept.id.0.clone()),
                    message: v.message,
                });
            }
        }
    }
    for edge in graph.edges.iter().filter(|e| e.source.bundle == id) {
        if let Some(snapshot) = edge.snapshot.as_ref().filter(|s| s.status != "matched") {
            result.push(LintEvidence {
                rule: "source-snapshot".into(),
                severity: "warn".into(),
                concept: Some(edge.source.id.clone()),
                message: format!(
                    "{}: {} ({})",
                    edge.resource, snapshot.status, snapshot.evidence
                ),
            });
        }
        if let Some(state) = edge
            .fingerprint_status
            .as_ref()
            .filter(|s| s.as_str() != "matched" && s.as_str() != "unchanged")
        {
            result.push(LintEvidence {
                rule: "source-fingerprint".into(),
                severity: "warn".into(),
                concept: Some(edge.source.id.clone()),
                message: format!("{}: {}", edge.resource, state),
            });
        }
    }
    if let Some(level) = severity(s.broken_link.as_deref(), Severity::Error) {
        for edge in graph.edges.iter().filter(|e| e.source.bundle == id) {
            if !matches!(
                edge.status.as_str(),
                "resolved" | "external" | "artifact" | "reserved"
            ) {
                result.push(LintEvidence {
                    rule: "catalog-reference".into(),
                    severity: level.as_str().into(),
                    concept: Some(edge.source.id.clone()),
                    message: format!("{}: {} ({})", edge.resource, edge.status, edge.evidence),
                });
            }
        }
    }
    if let Some(level) = severity(s.index_coverage.as_deref(), Severity::Warn) {
        for f in crate::check::lint::rules::index_coverage::check_indexes(
            &bundle,
            &effective.settings.lint.index_exclude,
        )? {
            result.push(LintEvidence {
                rule: "index-coverage".into(),
                severity: level.as_str().into(),
                concept: None,
                message: f.message,
            });
        }
    }
    result
        .sort_by(|a, b| (&a.rule, &a.concept, &a.message).cmp(&(&b.rule, &b.concept, &b.message)));
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn write(root: &Path, path: &str, text: &str) {
        let p = root.join(path);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    fn fixture() -> tempfile::TempDir {
        let d = tempfile::tempdir().unwrap();
        write(d.path(),"okf-catalog.yaml","catalog_version: 2\nbundles:\n  app.local:\n    location: {type: directory, path: consumer}\n  acme.remote:\n    location: {type: git, url: 'https://example.invalid/repo', ref: refs/heads/main, path: knowledge}\n");
        write(
            d.path(),
            "consumer/a.md",
            "---\ntype: Note\ntitle: Consumer\n---\n[dependency](okf://acme.remote/target.md)\n",
        );
        write(
            d.path(),
            "old/target.md",
            "---\ntype: Note\ntitle: Old\n---\nold\n",
        );
        write(
            d.path(),
            "new/added.md",
            "---\ntype: Note\ntitle: New\n---\nnew\n",
        );
        d
    }
    #[test]
    fn reports_removed_target_scoped_lint_and_support_changes_without_mutation() {
        let d = fixture();
        write(d.path(), "old/.okf-bundle/scripts/check.sh", "old script");
        write(d.path(), "new/.okf-bundle/scripts/check.sh", "new script");
        let catalog = d.path().join("okf-catalog.yaml");
        let bytes = std::fs::read(&catalog).unwrap();
        let old = BTreeMap::from([("acme.remote".into(), d.path().join("old"))]);
        let new = BTreeMap::from([("acme.remote".into(), d.path().join("new"))]);
        let report = report_snapshots(&catalog, &old, &new, &[d.path().join("consumer")]).unwrap();
        assert!(report.complete, "{:?}", report.reasons);
        assert_eq!(report.bundles[0].concepts.removed, vec!["/target"]);
        assert_eq!(report.bundles[0].concepts.added, vec!["/added"]);
        assert_eq!(
            report.bundles[0].support.changed,
            vec![".okf-bundle/scripts/check.sh"]
        );
        let impact = &report.consumers[0];
        assert_eq!(impact.affected_concepts, vec!["/a"]);
        assert_eq!(impact.before_links[0].status, "resolved");
        assert_eq!(impact.after_links[0].status, "missing-target");
        assert!(impact
            .after_lint
            .iter()
            .any(|f| f.rule == "catalog-reference" && f.message.contains("missing-target")));
        assert_eq!(std::fs::read(&catalog).unwrap(), bytes);
        assert!(!d.path().join("okf-catalog.lock").exists());
    }
    #[test]
    fn initial_acquisition_has_no_missing_old_tree_and_absent_scope_is_incomplete() {
        let d = fixture();
        let catalog = d.path().join("okf-catalog.yaml");
        let new = BTreeMap::from([("acme.remote".into(), d.path().join("old"))]);
        let report = report_snapshots(
            &catalog,
            &BTreeMap::new(),
            &new,
            &[d.path().join("consumer")],
        )
        .unwrap();
        assert!(report.complete, "{:?}", report.reasons);
        assert_eq!(report.bundles[0].concepts.added, vec!["/target"]);
        assert_eq!(
            report.consumers[0].before_links[0].status,
            "unknown-bundle-id"
        );
        assert_eq!(report.consumers[0].after_links[0].status, "resolved");
        let empty = report_snapshots(&catalog, &BTreeMap::new(), &new, &[]).unwrap();
        assert!(!empty.complete);
        assert_eq!(empty.reasons, vec!["consumer-impact-not-examined"]);
    }
    #[test]
    fn compares_ontology_target_types_across_candidate_snapshot() {
        let d = fixture();
        write(d.path(),"consumer/ontology.yaml","okf_ontology: '0.1'\nconcepts:\n  Note:\n    references:\n      product: {target: Feature, cardinality: '1..1'}\n  Feature: {}\n  Defect: {}\n");
        write(
            d.path(),
            "consumer/a.md",
            "---\ntype: Note\nproduct: okf://acme.remote/target.md\n---\nConsumer\n",
        );
        write(d.path(), "old/target.md", "---\ntype: Feature\n---\nOld\n");
        write(d.path(), "new/target.md", "---\ntype: Defect\n---\nNew\n");
        let before = BTreeMap::from([("acme.remote".into(), d.path().join("old"))]);
        let after = BTreeMap::from([("acme.remote".into(), d.path().join("new"))]);
        let report = report_snapshots(
            &d.path().join("okf-catalog.yaml"),
            &before,
            &after,
            &[d.path().join("consumer")],
        )
        .unwrap();
        assert!(report.complete);
        assert!(!report.consumers[0]
            .before_lint
            .iter()
            .any(|f| f.rule == "ontology-violation"));
        assert!(report.consumers[0]
            .after_lint
            .iter()
            .any(|f| f.rule == "ontology-violation"));
    }
}
