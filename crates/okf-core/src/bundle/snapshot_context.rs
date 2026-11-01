//! Frozen consumer context for offline historical reads.
//!
//! Workspace history selects the consumer's catalog/lock, while explicit bundle history
//! is an inspection override. Installed contexts prevent rediscovery of upstream settings.
use super::catalog::{absolute, load_catalog, normalize};
use super::context::Context;
use crate::error::{OkfError, Result};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SnapshotSelection {
    pub workspace_revision: Option<String>,
    #[serde(default)]
    pub bundle_revisions: BTreeMap<String, String>,
}
impl SnapshotSelection {
    pub fn parse(workspace_revision: Option<String>, values: &[String]) -> Result<Self> {
        let mut bundle_revisions = BTreeMap::new();
        for value in values {
            let (id, commit) = value.split_once('=').ok_or_else(|| {
                OkfError::Usage("bundle revision must be <bundle-id>=<full-commit>".into())
            })?;
            if !super::catalog::valid_id(id) || !full_commit(commit) {
                return Err(OkfError::Usage(format!("invalid bundle revision {value:?}; expected namespaced ID and full hexadecimal commit")));
            }
            if bundle_revisions
                .insert(id.to_owned(), commit.to_ascii_lowercase())
                .is_some()
            {
                return Err(OkfError::Usage(format!(
                    "duplicate bundle revision selector for {id}"
                )));
            }
        }
        if workspace_revision
            .as_ref()
            .is_some_and(|s| s.is_empty() || s.starts_with('-') || s.contains('\0'))
        {
            return Err(OkfError::Usage("invalid workspace revision".into()));
        }
        Ok(Self {
            workspace_revision,
            bundle_revisions,
        })
    }
    pub fn is_empty(&self) -> bool {
        self.workspace_revision.is_none() && self.bundle_revisions.is_empty()
    }
}
fn full_commit(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && value.bytes().all(|b| b.is_ascii_hexdigit())
}

/// A complete ordinary-file tree materialized exclusively from available local objects.
#[derive(Debug, Clone)]
pub struct SnapshotTree {
    /// Original repository root (or managed repository snapshot root) containing the input.
    pub original_repository: PathBuf,
    pub root: PathBuf,
    pub commit: String,
    /// Stable repository identity for rejecting conflicting per-repository selectors.
    pub repository_identity: String,
}
/// The backend must never fetch or execute checkout filters/hooks.
pub trait SnapshotMaterializer {
    fn materialize(&self, root: &Path, revision: &str) -> Result<SnapshotTree>;
}
/// Production adapter uses the shared managed object-store reader, never checkout/fetch.
pub struct LocalObjects;
impl SnapshotMaterializer for LocalObjects {
    fn materialize(&self, root: &Path, revision: &str) -> Result<SnapshotTree> {
        let source = std::fs::canonicalize(root)?;
        let snapshot = super::remote::materialize_local(&source, revision)?;
        let mut original_repository = source;
        for _ in snapshot.bundle_relative.components() {
            original_repository.pop();
        }
        Ok(SnapshotTree {
            original_repository,
            root: snapshot.root,
            commit: snapshot.commit,
            repository_identity: snapshot.repository.to_string_lossy().into_owned(),
        })
    }
}
/// Prepare offline snapshot context using locally available repository objects.
pub fn prepare(
    current: &Context,
    selection: SnapshotSelection,
    examined_ids: &[String],
) -> Result<SnapshotContext> {
    prepare_with(current, selection, examined_ids, &LocalObjects)
}
#[derive(Debug, Clone, Serialize)]
pub struct SnapshotEvidence {
    pub bundle: Option<String>,
    pub source_root: PathBuf,
    pub root: PathBuf,
    pub commit: Option<String>,
    pub selection: String,
    pub lock_divergence: bool,
}
#[derive(Debug, Clone)]
pub struct SnapshotContext {
    pub context: Context,
    pub selection: SnapshotSelection,
    pub evidence: Vec<SnapshotEvidence>,
    mappings: BTreeMap<PathBuf, PathBuf>,
}

fn source_path(path: &Path) -> Result<PathBuf> {
    let absolute = absolute(path)?;
    Ok(std::fs::canonicalize(&absolute).unwrap_or(absolute))
}

fn mapped(path: &Path, tree: &SnapshotTree) -> PathBuf {
    path.strip_prefix(&tree.original_repository)
        .map(|rel| tree.root.join(rel))
        .unwrap_or_else(|_| path.to_path_buf())
}

fn historical_config(
    path: &Path,
    original: &Path,
    tree: &SnapshotTree,
) -> Result<super::config::Config> {
    let mut config = super::config::load_config(path)?;
    let base = original.parent().unwrap_or(Path::new("."));
    let rebase = |value: &mut String| {
        let authored = normalize(&base.join(&*value));
        *value = mapped(&authored, tree).to_string_lossy().into_owned();
    };
    if let Some(value) = &mut config.bundle {
        rebase(value);
    }
    if let Some(value) = config.default_bundle.as_mut().and_then(|s| s.path.as_mut()) {
        rebase(value);
    }
    if let Some(value) = &mut config.catalog {
        rebase(value);
    }
    if let Some(value) = &mut config.catalog_overrides {
        rebase(value);
    }
    if let Some(groups) = config
        .settings
        .get_mut("bundle_settings")
        .and_then(toml::Value::as_table_mut)
    {
        for (_, group) in groups.iter_mut() {
            if let Some(value) = group.as_table_mut().and_then(|t| t.get_mut("ontology")) {
                if let Some(text) = value.as_str() {
                    let mut text = text.to_owned();
                    rebase(&mut text);
                    *value = toml::Value::String(text);
                }
            }
        }
    }
    Ok(config)
}

/// Legacy single-primary historical reads use a fixed local commit without applying the
/// revision to unrelated catalog registrations. The caller must reject ambiguous multi-scope use.
pub fn prepare_primary_revision(current: &Context, revision: &str) -> Result<SnapshotContext> {
    let tree = LocalObjects.materialize(&current.primary.root, revision)?;
    if let Some(id) = &current.primary.id {
        struct PreparedTree(SnapshotTree);
        impl SnapshotMaterializer for PreparedTree {
            fn materialize(&self, _: &Path, _: &str) -> Result<SnapshotTree> {
                Ok(self.0.clone())
            }
        }
        return prepare_with(
            current,
            SnapshotSelection {
                workspace_revision: None,
                bundle_revisions: BTreeMap::from([(id.clone(), tree.commit.clone())]),
            },
            &[],
            &PreparedTree(tree),
        );
    }
    let source = source_path(&current.primary.root)?;
    let root = mapped(&source, &tree);
    let mut context = current.clone();
    context.primary.root = root.clone();
    if let Some(path) = &current.config_path {
        let historical = mapped(&source_path(path)?, &tree);
        if historical != *path {
            context.config = if historical.is_file() {
                historical_config(&historical, &source_path(path)?, &tree)?
            } else {
                Default::default()
            };
            context.config_path = historical.is_file().then_some(historical);
        }
    }
    Ok(SnapshotContext {
        context,
        selection: SnapshotSelection {
            workspace_revision: Some(tree.commit.clone()),
            bundle_revisions: BTreeMap::new(),
        },
        evidence: vec![SnapshotEvidence {
            bundle: None,
            source_root: source.clone(),
            root: root.clone(),
            commit: Some(tree.commit),
            selection: "primary-revision-inspection".into(),
            lock_divergence: false,
        }],
        mappings: BTreeMap::from([(source, root)]),
    })
}

/// Resolve selected snapshots without consulting a cached upstream repository's config.
/// `examined_ids` is the explicit consumer scope, not all registered bundles.
pub fn prepare_with(
    current: &Context,
    selection: SnapshotSelection,
    examined_ids: &[String],
    backend: &dyn SnapshotMaterializer,
) -> Result<SnapshotContext> {
    let normalized = SnapshotSelection::parse(
        selection.workspace_revision.clone(),
        &selection
            .bundle_revisions
            .iter()
            .map(|(id, commit)| format!("{id}={commit}"))
            .collect::<Vec<_>>(),
    )?;
    let selection = normalized;
    let mut context = current.clone();
    let mut mappings = BTreeMap::new();
    let mut evidence = Vec::new();
    let mut scope: BTreeSet<String> = examined_ids.iter().cloned().collect();
    if let Some(id) = &current.primary.id {
        scope.insert(id.clone());
    }
    if let Some(revision) = &selection.workspace_revision {
        let anchor = current
            .catalog
            .as_ref()
            .map(|c| c.path.as_path())
            .or(current.config_path.as_deref())
            .unwrap_or(&current.primary.root);
        let base = if anchor.is_file() {
            anchor.parent().unwrap()
        } else {
            anchor
        };
        let tree = backend.materialize(base, revision)?;
        let old_primary = source_path(&current.primary.root)?;
        context.primary.root = mapped(&old_primary, &tree);
        mappings.insert(old_primary.clone(), context.primary.root.clone());
        if let Some(old_catalog) = &current.catalog {
            let historical_path = mapped(&source_path(&old_catalog.path)?, &tree);
            if historical_path == old_catalog.path
                && !old_catalog.path.starts_with(&tree.original_repository)
            {
                return Err(OkfError::Usage(
                    "workspace catalog is outside the selected repository".into(),
                ));
            }
            let mut catalog = load_catalog(&historical_path, None)?;
            // Relative locations escaping the workspace remain external working-tree context.
            let authored: super::catalog::CatalogFile =
                serde_yaml::from_str(&std::fs::read_to_string(&historical_path)?)
                    .map_err(|e| OkfError::Usage(format!("invalid historical catalog: {e}")))?;
            for (id, declaration) in &authored.bundles {
                if declaration.location.type_ == "directory" {
                    let original = source_path(&normalize(
                        &old_catalog
                            .path
                            .parent()
                            .unwrap_or(Path::new("."))
                            .join(&declaration.location.path),
                    ))?;
                    let target = mapped(&original, &tree);
                    if let Some(entry) = catalog.entries.get_mut(id) {
                        entry.root = target.clone();
                        entry.configured_root = target.clone();
                        entry.available = target.is_dir();
                        entry.overridden = false;
                        mappings.insert(original, target);
                    }
                }
            }
            // Historical default/registrations are authoritative; never silently use a current ID.
            if let Some(id) = &current.primary.id {
                context.primary.root = catalog.entry(id)?.root.clone();
                mappings.insert(old_primary.clone(), context.primary.root.clone());
            }
            context.catalog = Some(catalog);
        }
        if let Some(old_config) = &current.config_path {
            let config_path = mapped(&source_path(old_config)?, &tree);
            if !config_path.is_file() {
                // A newer workspace can have no legacy config at the selected revision.
                context.config = super::config::Config::default();
                context.config_path = None;
            } else {
                context.config = historical_config(&config_path, &source_path(old_config)?, &tree)?;
                context.config_path = Some(config_path);
            }
        }
        let (commit, selection_name) = if context.primary.root.starts_with(&tree.root) {
            (Some(tree.commit), "workspace-revision")
        } else if let Some(origin) = super::remote::origin(&context.primary.root)? {
            (origin.commit, "workspace-lock")
        } else {
            (None, "external-working-tree")
        };
        evidence.push(SnapshotEvidence {
            bundle: current.primary.id.clone(),
            source_root: old_primary,
            root: context.primary.root.clone(),
            commit,
            selection: selection_name.into(),
            lock_divergence: false,
        });
    }
    let mut repositories = BTreeMap::<String, String>::new();
    for (id, commit) in &selection.bundle_revisions {
        if !scope.contains(id) {
            return Err(OkfError::Usage(format!(
                "bundle revision {id} is outside the explicitly examined scope"
            )));
        }
        let catalog = context
            .catalog
            .as_mut()
            .ok_or_else(|| OkfError::Usage("bundle revisions require a consumer catalog".into()))?;
        let entry = catalog.entry(id)?.clone();
        let tree = backend.materialize(&entry.root, commit)?;
        if let Some(previous) =
            repositories.insert(tree.repository_identity.clone(), tree.commit.clone())
        {
            if previous != tree.commit {
                return Err(OkfError::Usage(
                    "bundle revision selectors for one repository must agree".into(),
                ));
            }
        }
        // One inspection commit per repository: remap already-authorized siblings,
        // without widening the examination scope to other registrations.
        let members: Vec<_> = catalog
            .entries
            .iter()
            .filter(|(member_id, member)| {
                scope.contains(*member_id) && member.root.starts_with(&tree.original_repository)
            })
            .map(|(member_id, member)| (member_id.clone(), member.root.clone()))
            .collect();
        for (member_id, source_root) in members {
            let root = mapped(&source_root, &tree);
            mappings.insert(source_root.clone(), root.clone());
            if let Some(old) = current
                .catalog
                .as_ref()
                .and_then(|c| c.entries.get(&member_id))
            {
                mappings.insert(old.root.clone(), root.clone());
            }
            let changed = catalog.entries.get_mut(&member_id).unwrap();
            let lock_divergence = changed
                .settings
                .get("remote")
                .and_then(|origin| origin.get("commit"))
                .and_then(serde_yaml::Value::as_str)
                .is_some_and(|accepted| accepted != tree.commit);
            changed.root = root.clone();
            changed.available = root.is_dir();
            if context.primary.id.as_deref() == Some(&member_id) {
                context.primary.root = root.clone();
            }
            evidence.push(SnapshotEvidence {
                bundle: Some(member_id),
                source_root,
                root,
                commit: Some(tree.commit.clone()),
                selection: "bundle-revision-inspection".into(),
                lock_divergence,
            });
        }
    }
    Ok(SnapshotContext {
        context,
        selection,
        evidence,
        mappings,
    })
}

thread_local! { static ACTIVE: RefCell<Option<SnapshotContext>> = const { RefCell::new(None) }; }
/// Whether the current operation explicitly inspects a historical snapshot.
pub fn is_inspection_active() -> bool {
    ACTIVE.with(|slot| {
        slot.borrow()
            .as_ref()
            .is_some_and(|ctx| !ctx.selection.is_empty())
    })
}
pub fn active_evidence() -> Vec<SnapshotEvidence> {
    ACTIVE.with(|slot| {
        slot.borrow()
            .as_ref()
            .map(|ctx| ctx.evidence.clone())
            .unwrap_or_default()
    })
}
pub struct SnapshotContextGuard {
    previous: Option<SnapshotContext>,
}
impl Drop for SnapshotContextGuard {
    fn drop(&mut self) {
        ACTIVE.with(|slot| *slot.borrow_mut() = self.previous.take());
    }
}
impl SnapshotContext {
    pub fn install(&self) -> SnapshotContextGuard {
        SnapshotContextGuard {
            previous: ACTIVE.with(|slot| slot.borrow_mut().replace(self.clone())),
        }
    }
}
/// Used by normal context resolution while a CLI read retains its snapshot guard.
pub fn active_context(path: Option<&str>, id: Option<&str>) -> Result<Option<Context>> {
    ACTIVE.with(|slot| {
        let borrowed = slot.borrow();
        let Some(snapshot) = borrowed.as_ref() else {
            return Ok(None);
        };
        if path.is_some() && id.is_some() {
            return Err(OkfError::Usage("bundle path and ID conflict".into()));
        }
        let mut context = snapshot.context.clone();
        if let Some(id) = id {
            let entry = context
                .catalog
                .as_ref()
                .ok_or_else(|| OkfError::Usage("bundle ID requires a catalog".into()))?
                .entry(id)?;
            context.primary = super::context::SelectedBundle {
                id: Some(id.into()),
                root: entry.root.clone(),
            };
        } else if let Some(path) = path {
            let original = source_path(Path::new(path))?;
            let root = snapshot
                .mappings
                .get(&original)
                .cloned()
                .unwrap_or(original);
            let id = context
                .catalog
                .as_ref()
                .and_then(|c| c.entries.values().find(|e| e.root == root))
                .map(|e| e.id.clone());
            context.primary = super::context::SelectedBundle { id, root };
        }
        Ok(Some(context))
    })
}

#[cfg(test)]
mod tests {
    use super::super::catalog::{Catalog, CatalogEntry};
    use super::super::context::SelectedBundle;
    use super::*;
    use std::cell::Cell;

    struct FakeBackend {
        from: PathBuf,
        to: PathBuf,
        calls: Cell<usize>,
    }
    impl SnapshotMaterializer for FakeBackend {
        fn materialize(&self, _root: &Path, revision: &str) -> Result<SnapshotTree> {
            self.calls.set(self.calls.get() + 1);
            Ok(SnapshotTree {
                original_repository: self.from.clone(),
                root: self.to.clone(),
                commit: revision.into(),
                repository_identity: "repo".into(),
            })
        }
    }
    fn context(root: &Path) -> Context {
        let entry = CatalogEntry {
            id: "acme.docs".into(),
            configured_root: root.join("docs"),
            root: root.join("docs"),
            overridden: false,
            available: true,
            settings: BTreeMap::new(),
        };
        Context {
            primary: SelectedBundle {
                id: Some("acme.docs".into()),
                root: root.join("docs"),
            },
            catalog: Some(Catalog {
                path: root.join("okf-catalog.yaml"),
                default_bundle: Some("acme.docs".into()),
                entries: BTreeMap::from([("acme.docs".into(), entry)]),
            }),
            config: Default::default(),
            config_path: None,
        }
    }
    #[test]
    fn selectors_require_full_commit_and_unique_names() {
        assert!(SnapshotSelection::parse(None, &["acme.docs=main".into()]).is_err());
        assert!(SnapshotSelection::parse(None, &[format!("plain={}", "a".repeat(40))]).is_err());
        assert!(SnapshotSelection::parse(
            None,
            &[
                format!("acme.docs={}", "a".repeat(40)),
                format!("acme.docs={}", "b".repeat(40))
            ]
        )
        .is_err());
        assert!(SnapshotSelection::parse(Some("-bad".into()), &[]).is_err());
        assert!(SnapshotSelection::parse(None, &[format!("acme.docs={}", "a".repeat(40))]).is_ok());
    }
    #[test]
    fn explicit_revision_requires_examination_scope() {
        let dir = tempfile::tempdir().unwrap();
        let backend = FakeBackend {
            from: dir.path().into(),
            to: dir.path().join("old"),
            calls: Cell::new(0),
        };
        let request =
            SnapshotSelection::parse(None, &[format!("other.docs={}", "a".repeat(40))]).unwrap();
        assert!(prepare_with(&context(dir.path()), request, &[], &backend).is_err());
        assert_eq!(backend.calls.get(), 0);
    }
    #[test]
    fn installed_context_remaps_original_paths_and_restores_after_drop() {
        let dir = tempfile::tempdir().unwrap();
        let old = tempfile::tempdir().unwrap();
        std::fs::create_dir(old.path().join("docs")).unwrap();
        let ctx = context(dir.path());
        let backend = FakeBackend {
            from: dir.path().into(),
            to: old.path().into(),
            calls: Cell::new(0),
        };
        let request =
            SnapshotSelection::parse(None, &[format!("acme.docs={}", "a".repeat(40))]).unwrap();
        let resolved = prepare_with(&ctx, request, &[], &backend).unwrap();
        assert_eq!(resolved.context.primary.root, old.path().join("docs"));
        {
            let _guard = resolved.install();
            let active = active_context(Some(ctx.primary.root.to_str().unwrap()), None)
                .unwrap()
                .unwrap();
            assert_eq!(active.primary.root, old.path().join("docs"));
            assert_eq!(active.catalog.unwrap().path, ctx.catalog.unwrap().path);
            assert!(!resolved.evidence[0].lock_divergence);
        }
        assert!(active_context(None, None).unwrap().is_none());
    }
    #[test]
    fn workspace_uses_historical_registration_without_upstream_discovery() {
        let current = tempfile::tempdir().unwrap();
        let history = tempfile::tempdir().unwrap();
        std::fs::create_dir(history.path().join("old-docs")).unwrap();
        std::fs::write(history.path().join("okf-catalog.yaml"),"catalog_version: 2\ndefault_bundle: acme.docs\nbundles:\n  acme.docs:\n    location: {type: directory, path: old-docs}\n").unwrap();
        let ctx = context(current.path());
        let backend = FakeBackend {
            from: current.path().into(),
            to: history.path().into(),
            calls: Cell::new(0),
        };
        let result = prepare_with(
            &ctx,
            SnapshotSelection {
                workspace_revision: Some("a".repeat(40)),
                bundle_revisions: BTreeMap::new(),
            },
            &[],
            &backend,
        )
        .unwrap();
        assert_eq!(result.context.primary.root, history.path().join("old-docs"));
        assert_eq!(
            result.context.catalog.unwrap().path,
            history.path().join("okf-catalog.yaml")
        );
    }
}
