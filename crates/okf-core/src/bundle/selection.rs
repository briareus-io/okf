//! Recovery-safe selection receipts. Receipts are freshness assertions, not credentials.
//!
//! Checks hold the bundle's cooperative transaction lock during publication/recovery.
//! Catalog files and arbitrary filesystem writers do not participate in that lock; an
//! uncooperative concurrent replacement can therefore still race a check.
use super::context::{self, Context};
use crate::error::{OkfError, Result};
use crate::fingerprint::canonicalize::sha256_hex;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::path::{Path, PathBuf};

fn invalid(message: impl std::fmt::Display) -> OkfError {
    OkfError::classified(
        crate::error::FailureReason::ContextInvalid,
        format!("selection changed or invalid: {message}; select again"),
    )
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileState {
    path: PathBuf,
    canonical: Option<PathBuf>,
    digest: Option<String>,
}
fn file_state(path: PathBuf) -> Result<FileState> {
    match std::fs::read(&path) {
        Ok(bytes) => Ok(FileState {
            canonical: Some(path.canonicalize()?),
            path,
            digest: Some(sha256_hex(&bytes)),
        }),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(FileState {
            path,
            canonical: None,
            digest: None,
        }),
        Err(e) => Err(e.into()),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct State {
    config: Option<FileState>,
    catalog: Option<FileState>,
    lock: Option<FileState>,
    overrides: Option<FileState>,
    markers: Vec<FileState>,
    root_identity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelectionReceipt {
    pub version: u32,
    pub root: PathBuf,
    pub id: Option<String>,
    pub source: String,
    pub access: String,
    cwd: PathBuf,
    path: Option<String>,
    selector_id: Option<String>,
    env_bundle: Option<String>,
    env_catalog: Option<String>,
    env_overrides: Option<String>,
    cache_root: Option<PathBuf>,
    state: State,
}

fn root_identity(root: &Path) -> Result<String> {
    let metadata = std::fs::metadata(root)?;
    if !metadata.is_dir() {
        return Err(invalid("selected root is not a directory"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        Ok(format!("{}:{}", metadata.dev(), metadata.ino()))
    }
    #[cfg(not(unix))]
    {
        Ok(format!("{:?}", metadata.created().ok()))
    }
}

fn capture(
    ctx: &Context,
    cwd: &Path,
    overrides_env: Option<&str>,
) -> Result<(State, String, String)> {
    let root = ctx.primary.root.canonicalize()?;
    let config = ctx.config_path.clone().map(file_state).transpose()?;
    let catalog = ctx
        .catalog
        .as_ref()
        .map(|c| file_state(c.path.clone()))
        .transpose()?;
    let lock = ctx
        .catalog
        .as_ref()
        .map(|c| file_state(super::remote::lock_path(&c.path)))
        .transpose()?;
    let override_path = overrides_env
        .filter(|s| !s.is_empty())
        .map(|p| cwd.join(p))
        .or_else(|| {
            ctx.config.catalog_overrides.as_ref().map(|p| {
                ctx.config_path
                    .as_ref()
                    .and_then(|p| p.parent())
                    .unwrap_or(cwd)
                    .join(p)
            })
        })
        .or_else(|| {
            ctx.catalog.as_ref().and_then(|c| {
                std::fs::read(&c.path)
                    .ok()
                    .and_then(|b| serde_yaml::from_slice::<super::catalog::CatalogFile>(&b).ok())
                    .and_then(|f| {
                        f.catalog_overrides
                            .map(|p| c.path.parent().unwrap_or(cwd).join(p))
                    })
            })
        });
    let overrides = override_path.map(file_state).transpose()?;
    let mut markers = Vec::new();
    let mut managed = false;
    for ancestor in root.ancestors() {
        // A managed marker applies only to a root inside the sibling `tree`.
        // Never read a same-named file inside the selected bundle.
        if !root.starts_with(ancestor.join("tree")) {
            continue;
        }
        let marker = file_state(ancestor.join("snapshot.yaml"))?;
        if marker.digest.is_some()
            && ancestor.join("tree").is_dir()
            && root.starts_with(ancestor.join("tree"))
        {
            managed = true;
        }
        markers.push(marker);
    }
    let entry = ctx
        .primary
        .id
        .as_ref()
        .and_then(|id| ctx.catalog.as_ref().and_then(|c| c.entries.get(id)));
    if entry.is_some_and(|e| !e.available) {
        return Err(invalid("selected registration is unavailable"));
    }
    let remote = entry.is_some_and(|e| e.settings.contains_key("remote"));
    let source = if remote {
        if entry.is_some_and(|e| e.overridden) {
            "git-override"
        } else {
            "git-snapshot"
        }
    } else if entry.is_some() {
        "directory"
    } else {
        "standalone"
    };
    let access = if remote || managed {
        "read-only"
    } else {
        "read-write"
    };
    Ok((
        State {
            config,
            catalog,
            lock,
            overrides,
            markers,
            root_identity: root_identity(&root)?,
        },
        source.into(),
        access.into(),
    ))
}

impl SelectionReceipt {
    pub fn issue(path: Option<&str>, id: Option<&str>) -> Result<Self> {
        let cwd = std::env::current_dir()?.canonicalize()?;
        let env_bundle = std::env::var(super::resolve::ENV_BUNDLE)
            .ok()
            .filter(|s| !s.is_empty());
        let env_catalog = std::env::var("OKF_CATALOG").ok().filter(|s| !s.is_empty());
        let env_overrides = std::env::var("OKF_CATALOG_OVERRIDES")
            .ok()
            .filter(|s| !s.is_empty());
        Self::issue_at(path, id, cwd, env_bundle, env_catalog, env_overrides)
    }
    fn issue_at(
        path: Option<&str>,
        id: Option<&str>,
        cwd: PathBuf,
        env_bundle: Option<String>,
        env_catalog: Option<String>,
        env_overrides: Option<String>,
    ) -> Result<Self> {
        let ctx = context::resolve_selection_context_with(
            path,
            id,
            env_bundle.as_deref(),
            &cwd,
            env_catalog.as_deref(),
            env_overrides.as_deref(),
        )?;
        let root = ctx.primary.root.canonicalize()?;
        let (state, source, access) = capture(&ctx, &cwd, env_overrides.as_deref())?;
        let cache_root = if ctx.catalog.as_ref().is_some_and(|catalog| {
            catalog
                .entries
                .values()
                .any(|entry| entry.settings.contains_key("remote"))
        }) {
            Some(super::remote::cache_root()?)
        } else {
            None
        };
        Ok(Self {
            version: 1,
            root,
            id: ctx.primary.id,
            source,
            access,
            cwd,
            path: path.map(str::to_owned),
            selector_id: id.map(str::to_owned),
            env_bundle,
            env_catalog,
            env_overrides,
            cache_root,
            state,
        })
    }
    pub fn encode(&self) -> Result<String> {
        serde_json::to_string(self).map_err(invalid)
    }
    pub fn parse(token: &str) -> Result<Self> {
        if token.len() > 1024 * 1024 {
            return Err(invalid("receipt exceeds 1 MiB"));
        }
        let receipt: Self = serde_json::from_str(token).map_err(invalid)?;
        if receipt.version != 1 || !receipt.cwd.is_absolute() || !receipt.root.is_absolute() {
            return Err(invalid("unsupported receipt"));
        }
        Ok(receipt)
    }
    /// Explicit ambient selectors must agree; absence does not discard the saved origin.
    pub fn check_explicit_environment(&self) -> Result<()> {
        let cwd = std::env::current_dir()?;
        for (name, expected, captured) in [
            (
                "OKF_CATALOG",
                self.state
                    .catalog
                    .as_ref()
                    .and_then(|s| s.canonical.as_ref()),
                self.env_catalog.as_deref(),
            ),
            (
                "OKF_CATALOG_OVERRIDES",
                self.state
                    .overrides
                    .as_ref()
                    .and_then(|s| s.canonical.as_ref()),
                self.env_overrides.as_deref(),
            ),
        ] {
            if let Some(value) = std::env::var_os(name).filter(|s| !s.is_empty()) {
                // An unchanged relative value still refers to its issuance cwd.
                if captured.is_some_and(|old| value == std::ffi::OsStr::new(old)) {
                    continue;
                }
                let actual = cwd.join(value).canonicalize().map_err(invalid)?;
                if Some(&actual) != expected {
                    return Err(invalid(format!("{name} conflicts with receipt")));
                }
            }
        }
        // An explicit path or ID took precedence over OKF_BUNDLE at issuance.
        // A receipt itself supplies a selector, so a newly introduced default is
        // likewise immaterial. Only check the environment when it selected the root.
        if self.path.is_none()
            && self.selector_id.is_none()
            && self.env_bundle.is_some()
            && let Some(value) =
                std::env::var_os(super::resolve::ENV_BUNDLE).filter(|s| !s.is_empty())
            && !self
                .env_bundle
                .as_deref()
                .is_some_and(|old| value == std::ffi::OsStr::new(old))
            && cwd.join(value).canonicalize().map_err(invalid)? != self.root
        {
            return Err(invalid("OKF_BUNDLE conflicts with receipt"));
        }
        Ok(())
    }
    pub fn check(&self) -> Result<Context> {
        if self.version != 1 {
            return Err(invalid("unsupported receipt version"));
        }
        if self
            .cache_root
            .as_ref()
            .is_some_and(|old| super::remote::cache_root().ok().as_ref() != Some(old))
        {
            return Err(invalid("remote cache changed"));
        }
        let ctx = context::resolve_selection_context_with(
            self.path.as_deref(),
            self.selector_id.as_deref(),
            self.env_bundle.as_deref(),
            &self.cwd,
            self.env_catalog.as_deref(),
            self.env_overrides.as_deref(),
        )
        .map_err(invalid)?;
        let root = ctx.primary.root.canonicalize().map_err(invalid)?;
        let (state, source, access) =
            capture(&ctx, &self.cwd, self.env_overrides.as_deref()).map_err(invalid)?;
        if root != self.root
            || ctx.primary.id != self.id
            || source != self.source
            || access != self.access
            || state != self.state
        {
            return Err(invalid("identity, access, or selection inputs differ"));
        }
        Ok(ctx)
    }
}

thread_local! { static ACTIVE: RefCell<Option<(SelectionReceipt, bool, Context)>> = const { RefCell::new(None) }; }
thread_local! { static STAGING: RefCell<usize> = const { RefCell::new(0) }; }
/// Internal temporary staging must not be confused with publication into another bundle.
#[must_use]
pub struct StagingGuard;
impl Drop for StagingGuard {
    fn drop(&mut self) {
        STAGING.with(|s| *s.borrow_mut() -= 1);
    }
}
pub fn suppress_for_staging() -> StagingGuard {
    STAGING.with(|s| *s.borrow_mut() += 1);
    StagingGuard
}
#[must_use]
pub struct SelectionGuard {
    previous: Option<(SelectionReceipt, bool, Context)>,
}
impl Drop for SelectionGuard {
    fn drop(&mut self) {
        ACTIVE.with(|a| *a.borrow_mut() = self.previous.take());
    }
}
pub fn activate(receipt: SelectionReceipt) -> Result<SelectionGuard> {
    activate_inner(receipt, false)
}
pub fn activate_for_recovery(receipt: SelectionReceipt) -> Result<SelectionGuard> {
    activate_inner(receipt, true)
}
fn activate_inner(receipt: SelectionReceipt, recovery: bool) -> Result<SelectionGuard> {
    let context = receipt.check()?;
    let previous = ACTIVE.with(|a| a.replace(Some((receipt, recovery, context))));
    Ok(SelectionGuard { previous })
}
pub fn active_context(path: Option<&str>, id: Option<&str>) -> Result<Option<Context>> {
    let receipt = ACTIVE.with(|a| a.borrow().clone());
    let Some((receipt, recovery, _)) = receipt else {
        return Ok(None);
    };
    let mut context = receipt.check()?;
    if STAGING.with(|s| *s.borrow() > 0) {
        if let Some(path) = path {
            let root = std::fs::canonicalize(path).map_err(invalid)?;
            if root != receipt.root {
                context.primary = super::context::SelectedBundle { root, id: None };
            }
        }
        if id.is_some_and(|id| context.primary.id.as_deref() != Some(id)) {
            return Err(invalid("staging ID conflicts with receipt"));
        }
        return Ok(Some(context));
    }
    if !recovery {
        context = context::resolve_context_with(
            receipt.path.as_deref(),
            receipt.selector_id.as_deref(),
            receipt.env_bundle.as_deref(),
            &receipt.cwd,
            receipt.env_catalog.as_deref(),
            receipt.env_overrides.as_deref(),
        )
        .map_err(invalid)?;
        if context.primary.id != receipt.id
            || context.primary.root.canonicalize().map_err(invalid)? != receipt.root
        {
            return Err(invalid("fully validated selection differs"));
        }
    }
    if let Some(path) = path {
        let root = std::fs::canonicalize(path).map_err(invalid)?;
        if root != receipt.root {
            let entry = context
                .catalog
                .as_ref()
                .and_then(|c| c.entries.values().find(|e| e.available && e.root == root))
                .ok_or_else(|| invalid("path is outside receipt catalog"))?;
            context.primary = super::context::SelectedBundle {
                root,
                id: Some(entry.id.clone()),
            };
        }
    }
    if id.is_some_and(|id| context.primary.id.as_deref() != Some(id)) {
        return Err(invalid("explicit ID conflicts with receipt"));
    }
    Ok(Some(context))
}
/// Revalidate selected identity and access while the native transaction guard is held.
pub fn check_active(root: &Path) -> Result<()> {
    if STAGING.with(|s| *s.borrow() > 0) {
        return Ok(());
    }
    let receipt = ACTIVE.with(|a| a.borrow().clone());
    if let Some((receipt, _, _)) = receipt {
        if root.canonicalize().map_err(invalid)? != receipt.root {
            return Err(invalid("mutation root conflicts with receipt"));
        }
        receipt.check()?;
        if receipt.access != "read-write" {
            return Err(OkfError::Policy("selected bundle is read-only".into()));
        }
    }
    Ok(())
}

/// Under a binding, use its consumer catalog instead of rediscovering one from cwd.
pub fn bound_catalog_access(path: &Path) -> Option<Result<()>> {
    let (_, _, context) = ACTIVE.with(|a| a.borrow().clone())?;
    Some((|| {
        if context.catalog.as_ref().is_some_and(|catalog| {
            catalog
                .entries
                .values()
                .any(|entry| entry.settings.contains_key("remote") && path.starts_with(&entry.root))
        }) {
            return Err(OkfError::Policy(
                "destination belongs to a read-only remote registration".into(),
            ));
        }
        Ok(())
    })())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mutate::transaction::{self, FileChange, RecoveryOutcome};
    use std::fs;

    fn fixture() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let cwd = dir.path().canonicalize().unwrap();
        fs::create_dir(cwd.join("bundle")).unwrap();
        fs::write(cwd.join("okf-catalog.yaml"), "catalog_version: 2\ndefault_bundle: test.main\nbundles:\n  test.main: {location: {type: directory, path: bundle}}\n").unwrap();
        (dir, cwd)
    }
    fn select(cwd: &Path) -> SelectionReceipt {
        SelectionReceipt::issue_at(None, None, cwd.to_path_buf(), None, None, None).unwrap()
    }
    #[test]
    fn selection_recovers_real_journal_and_binds_after_lock() {
        let (_dir, cwd) = fixture();
        let root = cwd.join("bundle");
        let journal = root.join(transaction::JOURNAL);
        fs::create_dir(&journal).unwrap();
        fs::write(root.join("note.md"), "after").unwrap();
        let change = FileChange {
            path: "note.md".into(),
            before: Some(b"before".to_vec()),
            after: Some(b"after".to_vec()),
        };
        fs::write(
            journal.join("manifest.json"),
            serde_json::to_vec(&vec![change]).unwrap(),
        )
        .unwrap();
        let receipt = select(&cwd);
        assert_eq!(receipt.id.as_deref(), Some("test.main"));
        let _active =
            activate(SelectionReceipt::parse(&receipt.encode().unwrap()).unwrap()).unwrap();
        assert_eq!(
            transaction::recover_with_outcome(&root).unwrap(),
            RecoveryOutcome::RolledBack(vec!["note.md".into()])
        );
        assert_eq!(fs::read(root.join("note.md")).unwrap(), b"before");
        let lock = transaction::writer_lock(&root).unwrap();
        fs::write(cwd.join("okf-catalog.yaml"), "catalog_version: 2\ndefault_bundle: test.main\nbundles:\n  test.main: {location: {type: directory, path: bundle}}\n# changed\n").unwrap();
        let change = FileChange {
            path: "new.md".into(),
            before: None,
            after: Some(b"new".to_vec()),
        };
        assert!(transaction::commit_locked(&root, &lock, vec![change]).is_err());
        assert!(!root.join("new.md").exists());
    }
    #[test]
    fn detects_override_lock_and_root_drift() {
        let (_dir, cwd) = fixture();
        fs::write(cwd.join("overrides.yaml"), "bundles: {test.main: bundle}\n").unwrap();
        let receipt = SelectionReceipt::issue_at(
            None,
            None,
            cwd.clone(),
            None,
            None,
            Some("overrides.yaml".into()),
        )
        .unwrap();
        fs::write(
            cwd.join("overrides.yaml"),
            "bundles: {test.main: bundle}\n# changed\n",
        )
        .unwrap();
        assert!(receipt.check().is_err());
        let receipt = select(&cwd);
        fs::write(cwd.join("okf-catalog.lock"), "new").unwrap();
        assert!(receipt.check().is_err());
        fs::remove_file(cwd.join("okf-catalog.lock")).unwrap();
        let receipt = select(&cwd);
        fs::rename(cwd.join("bundle"), cwd.join("old")).unwrap();
        fs::create_dir(cwd.join("bundle")).unwrap();
        assert!(receipt.check().is_err());
    }

    #[test]
    fn unrelated_unavailable_remote_does_not_block_local_recovery() {
        let (_dir, cwd) = fixture();
        fs::write(cwd.join("okf-catalog.yaml"), "catalog_version: 2\ndefault_bundle: test.main\nbundles:\n  test.main: {location: {type: directory, path: bundle}}\n  test.remote: {location: {type: git, url: 'https://example.invalid/repo.git', ref: refs/heads/main, path: docs}}\n").unwrap();
        let root = cwd.join("bundle");
        fs::create_dir(root.join(transaction::JOURNAL)).unwrap();
        let receipt = select(&cwd);
        let _active = activate_for_recovery(receipt).unwrap();
        assert_eq!(
            transaction::recover_with_outcome(&root).unwrap(),
            RecoveryOutcome::RolledBack(vec![])
        );
    }

    #[test]
    fn publication_can_change_its_own_selection_config() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        fs::write(root.join("okf.toml"), "bundle = '.'\n").unwrap();
        let receipt =
            SelectionReceipt::issue_at(None, None, root.clone(), None, None, None).unwrap();
        let _active = activate(receipt).unwrap();
        let lock = transaction::writer_lock(&root).unwrap();
        let changes = vec![
            FileChange {
                path: "okf.toml".into(),
                before: Some(b"bundle = '.'\n".to_vec()),
                after: Some(b"bundle = '.'\n# changed\n".to_vec()),
            },
            FileChange {
                path: "note.md".into(),
                before: None,
                after: Some(b"written".to_vec()),
            },
        ];
        transaction::commit_locked(&root, &lock, changes).unwrap();
        assert_eq!(fs::read(root.join("note.md")).unwrap(), b"written");
    }
}
