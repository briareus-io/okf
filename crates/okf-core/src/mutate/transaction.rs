//! Recoverable publication of a prevalidated set of files within one bundle.
//!
//! Each file is atomically replaced; the set is journaled, not a filesystem-wide atomic
//! snapshot. Cooperating writers refuse a pending journal. Recovery rolls back only when
//! every file still matches its before/after image, never overwriting intervening edits.
use crate::error::{OkfError, Result};
use serde::{Deserialize, Serialize};
use std::cell::Cell;
use std::io::Write;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublicationEvidence {
    None,
    NotPublished,
    Committed,
    RecoveryRequired { commit_reached: bool },
}
thread_local! { static PUBLICATION_EVIDENCE: Cell<PublicationEvidence> = const { Cell::new(PublicationEvidence::None) }; }
thread_local! { static REQUIRE_ONTOLOGY_GATE: Cell<bool> = const { Cell::new(false) }; }
thread_local! { static SUPPRESS_ONTOLOGY_GATE: Cell<bool> = const { Cell::new(false) }; }
pub struct GateSuppression(bool);
impl Drop for GateSuppression {
    fn drop(&mut self) {
        SUPPRESS_ONTOLOGY_GATE.with(|cell| cell.set(self.0));
    }
}
pub fn suppress_ontology_gate() -> GateSuppression {
    GateSuppression(SUPPRESS_ONTOLOGY_GATE.with(|cell| {
        let old = cell.get();
        cell.set(true);
        old
    }))
}
pub fn ontology_gate_suppressed() -> bool {
    SUPPRESS_ONTOLOGY_GATE.with(Cell::get)
}
pub struct GateStrengthening(bool);
impl Drop for GateStrengthening {
    fn drop(&mut self) {
        REQUIRE_ONTOLOGY_GATE.with(|cell| cell.set(self.0));
    }
}
pub fn strengthen_ontology_gate() -> GateStrengthening {
    GateStrengthening(REQUIRE_ONTOLOGY_GATE.with(|cell| {
        let old = cell.get();
        cell.set(true);
        old
    }))
}
pub fn ontology_gate_strengthened() -> bool {
    REQUIRE_ONTOLOGY_GATE.with(Cell::get)
}
pub fn check_ontology_snapshot(
    root: &Path,
    supplied: Option<&crate::ontology::schema::Ontology>,
) -> Result<()> {
    let Some(supplied) = supplied else {
        return Ok(());
    };
    let (settings, current) = crate::bundle::settings::load_for(root)?;
    if settings.ontology_path.is_some() && current.as_ref() != Some(supplied) {
        return Err(usage(
            "baseline-conflict: ontology changed during write preparation; preview again",
        ));
    }
    Ok(())
}
pub fn publication_evidence() -> PublicationEvidence {
    PUBLICATION_EVIDENCE.with(Cell::get)
}
pub fn reset_publication_evidence() {
    PUBLICATION_EVIDENCE.with(|cell| cell.set(PublicationEvidence::None));
}
fn set_evidence(value: PublicationEvidence) {
    PUBLICATION_EVIDENCE.with(|cell| cell.set(value));
}
pub struct PublicationEvidenceScope(PublicationEvidence);
impl Drop for PublicationEvidenceScope {
    fn drop(&mut self) {
        set_evidence(self.0);
    }
}
/// Isolate journal writes to a temporary workspace from the selected bundle outcome.
pub fn suppress_publication_evidence() -> PublicationEvidenceScope {
    let previous = publication_evidence();
    set_evidence(PublicationEvidence::None);
    PublicationEvidenceScope(previous)
}

pub const JOURNAL: &str = ".okf-transaction";
pub const LOCK: &str = ".okf-transaction.lock";

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    changes: Vec<FileChange>,
    #[serde(default)]
    external_target: Option<PathBuf>,
    permissions: std::collections::BTreeMap<PathBuf, SavedPermissions>,
    created_dirs: Vec<PathBuf>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SavedPermissions {
    readonly: bool,
    #[cfg(unix)]
    mode: u32,
}
impl SavedPermissions {
    fn capture(path: &Path) -> Result<Self> {
        let permissions = std::fs::metadata(path)?.permissions();
        Ok(Self {
            readonly: permissions.readonly(),
            #[cfg(unix)]
            mode: {
                use std::os::unix::fs::PermissionsExt;
                permissions.mode()
            },
        })
    }
    fn restore(&self, path: &Path) -> Result<()> {
        let mut permissions = std::fs::metadata(path)?.permissions();
        permissions.set_readonly(self.readonly);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            permissions.set_mode(self.mode);
        }
        std::fs::set_permissions(path, permissions)?;
        std::fs::File::open(path)?.sync_all()?;
        Ok(())
    }
}

// This stable inode is never unlinked: removing a lock file allows another caller
// to lock a different inode while an earlier holder still owns the old one.
/// Unlock explicitly before closing the descriptor: another thread can fork while a
/// lock is held, temporarily retaining the descriptor until that child execs.
#[derive(Debug)]
pub struct WriterGuard {
    file: std::fs::File,
    root: PathBuf,
}
impl Drop for WriterGuard {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}

fn acquire_lock(root: &Path) -> Result<WriterGuard> {
    let path = root.join(LOCK);
    let file = match std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(&path)
    {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            let meta = std::fs::symlink_metadata(&path)?;
            if !meta.is_file() || meta.file_type().is_symlink() {
                return Err(usage("transaction lock must be a regular file"));
            }
            std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(path)?
        }
        Err(e) => return Err(e.into()),
    };
    file.try_lock().map_err(|e| {
        usage(format!(
            "transaction is still active; operation refused: {e}"
        ))
    })?;
    Ok(WriterGuard {
        file,
        root: root.canonicalize()?,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileChange {
    pub path: PathBuf,
    pub before: Option<Vec<u8>>,
    pub after: Option<Vec<u8>>,
}

fn usage(message: impl Into<String>) -> OkfError {
    OkfError::Usage(message.into())
}

/// Reject symlink components, special files, traversal, and reserved transaction paths.
pub fn checked_path(root: &Path, relative: &Path) -> Result<PathBuf> {
    if relative.as_os_str().is_empty()
        || relative
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err(usage(format!(
            "transaction path must be relative without traversal: {}",
            relative.display()
        )));
    }
    if relative
        .components()
        .any(|c| c.as_os_str().to_string_lossy().starts_with('.'))
    {
        return Err(usage(format!(
            "transaction cannot edit hidden/control path: {}",
            relative.display()
        )));
    }
    let mut path = root.to_path_buf();
    let count = relative.components().count();
    for (i, component) in relative.components().enumerate() {
        path.push(component);
        match std::fs::symlink_metadata(&path) {
            Ok(m)
                if m.file_type().is_symlink()
                    || (i + 1 < count && !m.is_dir())
                    || (i + 1 == count && !m.is_file()) =>
            {
                return Err(usage(format!(
                    "transaction requires regular files and real directories: {}",
                    path.display()
                )));
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
    }
    Ok(path)
}

pub fn read_optional(path: &Path) -> Result<Option<Vec<u8>>> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

fn sync_dir(path: &Path) -> Result<()> {
    std::fs::File::open(path)?.sync_all()?;
    Ok(())
}

fn target_path(root: &Path, change: &FileChange, external: Option<&Path>) -> Result<PathBuf> {
    if let Some(external) = external {
        if change.path != Path::new("ontology.yaml") || !external.is_absolute() {
            return Err(usage("invalid external ontology journal target"));
        }
        let parent = external
            .parent()
            .ok_or_else(|| usage("external ontology target has no parent"))?;
        if parent.canonicalize()? != parent || external.file_name().is_none() {
            return Err(usage("external ontology journal target is not canonical"));
        }
        if let Ok(meta) = std::fs::symlink_metadata(external) {
            if !meta.is_file() || meta.file_type().is_symlink() {
                return Err(usage("external ontology target must be a regular file"));
            }
        }
        Ok(external.to_path_buf())
    } else {
        checked_path(root, &change.path)
    }
}

fn publish(
    root: &Path,
    change: &FileChange,
    bytes: &Option<Vec<u8>>,
    external: Option<&Path>,
) -> Result<()> {
    let path = target_path(root, change, external)?;
    if let Some(bytes) = bytes {
        let parent = path.parent().unwrap();
        std::fs::create_dir_all(parent)?;
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let (temp, mut file) = loop {
            let seq = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let temp = parent.join(format!(".okf-publish-{}-{seq}", std::process::id()));
            match std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temp)
            {
                Ok(f) => break (temp, f),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(e.into()),
            }
        };
        let result = (|| -> Result<()> {
            if let Ok(meta) = std::fs::metadata(&path) {
                file.set_permissions(meta.permissions())?;
            }
            file.write_all(bytes)?;
            file.sync_all()?;
            drop(file);
            std::fs::rename(&temp, &path)?;
            sync_dir(parent)
        })();
        let _ = std::fs::remove_file(&temp);
        result?;
    } else if path.exists() {
        std::fs::remove_file(&path)?;
        sync_dir(path.parent().unwrap())?;
    }
    Ok(())
}

pub fn ensure_idle(root: &Path) -> Result<()> {
    if std::fs::symlink_metadata(root.join(JOURNAL)).is_ok() {
        return Err(usage(
            "pending transaction; run `okf changeset recover` before writing",
        ));
    }
    Ok(())
}

/// Hold across an ordinary mutation's complete read/modify/write operation.
/// Publish with `commit_locked` while holding this guard. An absent root is an error;
/// bundle initialization creates its root before acquiring this guard.
pub fn writer_lock(root: &Path) -> Result<WriterGuard> {
    let lock = acquire_lock(root)?;
    ensure_idle(root)?;
    Ok(lock)
}

/// Read-only publication checks shared by previews and commits. File/directory
/// type transitions are unsupported, even if another change removes the blocker.
pub fn preflight(root: &Path, changes: &[FileChange]) -> Result<()> {
    ensure_idle(root)?;
    let mut paths = std::collections::BTreeSet::new();
    for c in changes {
        if !paths.insert(c.path.clone()) {
            return Err(usage("duplicate transaction path"));
        }
        let path = checked_path(root, &c.path)?;
        if read_optional(&path)? != c.before {
            return Err(usage(format!(
                "file changed since planning: {}",
                c.path.display()
            )));
        }
    }
    Ok(())
}

/// Publish guarded before/after images. On a normal failure roll back; on interruption
/// retain the durable journal for an explicit `recover` invocation.
pub fn commit(root: &Path, changes: Vec<FileChange>) -> Result<()> {
    commit_with_hook(root, changes, &mut |_| Ok(()))
}

/// Publish using a guard acquired before preparation. The guard must belong to `root`.
pub fn commit_locked(root: &Path, guard: &WriterGuard, changes: Vec<FileChange>) -> Result<()> {
    if guard.root != root.canonicalize()? {
        return Err(usage("writer guard belongs to a different bundle"));
    }
    if !ontology_gate_suppressed() {
        super::gate::check_changes(root, &changes, ontology_gate_strengthened())?;
    }
    commit_under_lock(root, changes, None, &mut |_| Ok(()))
}

/// Publish one configured ontology sidecar outside the root with its canonical target
/// recorded in the selected bundle's recovery journal.
pub fn commit_external_ontology_locked(
    root: &Path,
    guard: &WriterGuard,
    target: &Path,
    before: Option<Vec<u8>>,
    after: Vec<u8>,
) -> Result<()> {
    if guard.root != root.canonicalize()? {
        return Err(usage("writer guard belongs to a different bundle"));
    }
    let parent = target
        .parent()
        .ok_or_else(|| usage("ontology target has no parent"))?
        .canonicalize()?;
    let filename = target
        .file_name()
        .ok_or_else(|| usage("ontology target has no name"))?;
    let canonical = parent.join(filename);
    if canonical.starts_with(root.canonicalize()?) {
        return Err(usage("external ontology target is inside selected bundle"));
    }
    let change = FileChange {
        path: PathBuf::from("ontology.yaml"),
        before,
        after: Some(after),
    };
    commit_under_lock(root, vec![change], Some(canonical), &mut |_| Ok(()))
}

fn commit_with_hook(
    root: &Path,
    changes: Vec<FileChange>,
    after_publish: &mut dyn FnMut(usize) -> Result<()>,
) -> Result<()> {
    let _root_lock = acquire_lock(root)?;
    commit_under_lock(root, changes, None, after_publish)
}

fn commit_under_lock(
    root: &Path,
    changes: Vec<FileChange>,
    external_target: Option<PathBuf>,
    after_publish: &mut dyn FnMut(usize) -> Result<()>,
) -> Result<()> {
    set_evidence(PublicationEvidence::NotPublished);
    ensure_idle(root)?;
    let changes: Vec<_> = changes
        .into_iter()
        .filter(|c| c.before != c.after)
        .collect();
    if changes.is_empty() {
        return Ok(());
    }
    if external_target.is_none() {
        preflight(root, &changes)?;
    } else {
        if changes.len() != 1 {
            return Err(usage("external ontology transaction requires one file"));
        }
        let path = target_path(root, &changes[0], external_target.as_deref())?;
        if read_optional(&path)? != changes[0].before {
            return Err(usage("external ontology changed since planning"));
        }
    }
    let mut permissions = std::collections::BTreeMap::new();
    let mut created_dirs = std::collections::BTreeSet::new();
    for change in &changes {
        if change.before.is_some() {
            permissions.insert(
                change.path.clone(),
                SavedPermissions::capture(&target_path(root, change, external_target.as_deref())?)?,
            );
        }
        if change.after.is_some() && external_target.is_none() {
            let mut parent = change.path.parent();
            while let Some(path) = parent.filter(|p| !p.as_os_str().is_empty()) {
                if !root.join(path).exists() {
                    created_dirs.insert(path.to_path_buf());
                }
                parent = path.parent();
            }
        }
    }
    let manifest = Manifest {
        changes: changes.clone(),
        external_target: external_target.clone(),
        permissions,
        created_dirs: created_dirs.into_iter().collect(),
    };
    let journal = root.join(JOURNAL);
    std::fs::create_dir(&journal)
        .map_err(|e| usage(format!("cannot acquire transaction journal: {e}")))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&journal, std::fs::Permissions::from_mode(0o700))?;
    }
    let lock = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(journal.join("lock"))?;
    lock.try_lock()
        .map_err(|e| usage(format!("transaction is active: {e}")))?;
    let setup = (|| -> Result<()> {
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(journal.join("manifest.tmp"))?;
        serde_json::to_writer(&mut f, &manifest).map_err(|e| OkfError::Internal(e.to_string()))?;
        f.write_all(b"\n")?;
        f.sync_all()?;
        drop(f);
        std::fs::rename(journal.join("manifest.tmp"), journal.join("manifest.json"))?;
        sync_dir(&journal)?;
        sync_dir(root)
    })();
    if let Err(e) = setup {
        let _ = std::fs::remove_dir_all(&journal);
        return Err(e);
    }
    let result = (|| -> Result<()> {
        // Recheck every expectation after acquiring the journal, before any publication.
        for c in &changes {
            if read_optional(&target_path(root, c, external_target.as_deref())?)? != c.before {
                return Err(usage(format!(
                    "file changed since planning: {}",
                    c.path.display()
                )));
            }
        }
        for (index, c) in changes.iter().enumerate() {
            if read_optional(&target_path(root, c, external_target.as_deref())?)? != c.before {
                return Err(usage(format!(
                    "concurrent file change: {}",
                    c.path.display()
                )));
            }
            publish(root, c, &c.after, external_target.as_deref())?;
            after_publish(index)?;
        }
        // Persist directory entries for newly created ancestor chains before commit.
        for directory in manifest.created_dirs.iter().rev() {
            sync_dir(&root.join(directory))?;
        }
        sync_dir(root)?;
        Ok(())
    })();
    if let Err(error) = result {
        return match recover_locked(root) {
            Ok(_) => Err(error),
            Err(recovery) => {
                set_evidence(PublicationEvidence::RecoveryRequired {
                    commit_reached: false,
                });
                Err(OkfError::Io(format!(
                    "{error}; rollback requires attention: {recovery}; journal retained"
                )))
            }
        };
    }
    let mark = (|| -> Result<()> {
        let marker = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(journal.join("committed"))?;
        marker.sync_all()?;
        sync_dir(&journal)
    })();
    if let Err(error) = mark {
        set_evidence(PublicationEvidence::RecoveryRequired {
            commit_reached: false,
        });
        return Err(error);
    }
    set_evidence(PublicationEvidence::Committed);
    if let Err(error) = cleanup(&journal).and_then(|_| sync_dir(root)) {
        set_evidence(PublicationEvidence::RecoveryRequired {
            commit_reached: true,
        });
        return Err(error);
    }
    Ok(())
}

/// Restore before-images after interrupted publication, refusing intervening user edits.
pub fn recover(root: &Path) -> Result<Vec<PathBuf>> {
    let _root_lock = acquire_lock(root)?;
    recover_pending_locked(root)
}

fn recover_pending_locked(root: &Path) -> Result<Vec<PathBuf>> {
    let journal = root.join(JOURNAL);
    let meta = std::fs::symlink_metadata(&journal)?;
    if !meta.is_dir() || meta.file_type().is_symlink() {
        return Err(usage("transaction journal must be a real directory"));
    }
    let lock_path = journal.join("lock");
    let _lock = match std::fs::symlink_metadata(&lock_path) {
        Ok(meta) => {
            if !meta.is_file() || meta.file_type().is_symlink() {
                return Err(usage("invalid transaction lock"));
            }
            let lock = std::fs::OpenOptions::new().write(true).open(lock_path)?;
            lock.try_lock().map_err(|e| {
                usage(format!(
                    "transaction is still active; recovery refused: {e}"
                ))
            })?;
            Some(lock)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(e.into()),
    };
    recover_locked(root)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryOutcome {
    RolledBack(Vec<PathBuf>),
    CommittedCleanup,
    NoPendingTransaction,
}

pub fn recover_with_outcome(root: &Path) -> Result<RecoveryOutcome> {
    let _guard = acquire_lock(root)?;
    if !root.join(JOURNAL).exists() {
        return Ok(RecoveryOutcome::NoPendingTransaction);
    }
    let committed = root.join(JOURNAL).join("committed").exists();
    let paths = recover_pending_locked(root)?;
    if committed {
        Ok(RecoveryOutcome::CommittedCleanup)
    } else {
        Ok(RecoveryOutcome::RolledBack(paths))
    }
}

fn recover_locked(root: &Path) -> Result<Vec<PathBuf>> {
    let journal = root.join(JOURNAL);
    let meta = std::fs::symlink_metadata(&journal)?;
    if !meta.is_dir() || meta.file_type().is_symlink() {
        return Err(usage("transaction journal must be a real directory"));
    }
    if let Ok(marker) = std::fs::symlink_metadata(journal.join("committed")) {
        if !marker.is_file() || marker.file_type().is_symlink() {
            return Err(usage("invalid transaction commit marker"));
        }
        cleanup(&journal)?;
        sync_dir(root)?;
        return Ok(Vec::new());
    }
    let manifest = journal.join("manifest.json");
    let meta = match std::fs::symlink_metadata(&manifest) {
        Ok(meta) => meta,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // Publication cannot begin until the complete manifest has been renamed.
            cleanup(&journal)?;
            sync_dir(root)?;
            return Ok(Vec::new());
        }
        Err(e) => return Err(e.into()),
    };
    if !meta.is_file() || meta.file_type().is_symlink() {
        return Err(usage("transaction manifest must be a regular file"));
    }
    let bytes = std::fs::read(&manifest)?;
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum JournalData {
        Current(Manifest),
        Legacy(Vec<FileChange>),
    }
    let Manifest {
        changes,
        external_target,
        permissions,
        created_dirs,
    } = match serde_json::from_slice(&bytes)
        .map_err(|e| usage(format!("invalid recovery journal: {e}")))?
    {
        JournalData::Current(data) => data,
        JournalData::Legacy(changes) => Manifest {
            changes,
            external_target: None,
            permissions: Default::default(),
            created_dirs: Vec::new(),
        },
    };
    let mut paths = std::collections::BTreeSet::new();
    for c in &changes {
        if !paths.insert(c.path.clone()) {
            return Err(usage("duplicate recovery path"));
        }
        let current = read_optional(&target_path(root, c, external_target.as_deref())?)?;
        if current != c.before && current != c.after {
            return Err(usage(format!(
                "recovery refuses intervening edit: {}",
                c.path.display()
            )));
        }
    }
    for directory in &created_dirs {
        checked_path(root, &directory.join("_okf_directory_check"))?;
    }
    for c in changes.iter().rev() {
        publish(root, c, &c.before, external_target.as_deref())?;
        if c.before.is_some() {
            if let Some(saved) = permissions.get(&c.path) {
                saved.restore(&target_path(root, c, external_target.as_deref())?)?;
            }
        }
    }
    // Only remove directories created by this transaction, and only while empty.
    let mut created_dirs = created_dirs;
    created_dirs.sort_by_key(|p| std::cmp::Reverse(p.components().count()));
    for directory in created_dirs {
        // Validate directory components using a synthetic regular-file child.
        checked_path(root, &directory.join("_okf_directory_check"))?;
        match std::fs::remove_dir(root.join(&directory)) {
            Ok(()) => {}
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::NotFound | std::io::ErrorKind::DirectoryNotEmpty
                ) => {}
            Err(e) => return Err(e.into()),
        }
    }
    cleanup(&journal)?;
    sync_dir(root)?;
    Ok(changes.into_iter().map(|c| c.path).collect())
}

fn cleanup(journal: &Path) -> Result<()> {
    for name in ["manifest.tmp", "manifest.json", "committed", "lock"] {
        match std::fs::remove_file(journal.join(name)) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
    }
    std::fs::remove_dir(journal)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn change(path: &str, before: Option<&str>, after: Option<&str>) -> FileChange {
        FileChange {
            path: path.into(),
            before: before.map(|s| s.as_bytes().to_vec()),
            after: after.map(|s| s.as_bytes().to_vec()),
        }
    }

    #[test]
    fn external_ontology_target_is_journaled_and_guardedly_rolled_back() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let target = outside.path().canonicalize().unwrap().join("ontology.yaml");
        fs::write(&target, "old\n").unwrap();
        let guard = writer_lock(root.path()).unwrap();
        let error = commit_under_lock(
            root.path(),
            vec![change("ontology.yaml", Some("old\n"), Some("new\n"))],
            Some(target.clone()),
            &mut |_| Err(OkfError::Io("injected external failure".into())),
        )
        .unwrap_err();
        assert!(error.to_string().contains("injected external failure"));
        assert_eq!(fs::read_to_string(&target).unwrap(), "old\n");
        assert!(!root.path().join(JOURNAL).exists());
        commit_external_ontology_locked(
            root.path(),
            &guard,
            &target,
            Some(b"old\n".to_vec()),
            b"new\n".to_vec(),
        )
        .unwrap();
        assert_eq!(fs::read_to_string(&target).unwrap(), "new\n");
        assert!(!root.path().join(JOURNAL).exists());
    }

    #[test]
    fn external_recovery_uses_recorded_target_and_refuses_intervening_edit() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let target = outside.path().canonicalize().unwrap().join("ontology.yaml");
        let redirected = outside
            .path()
            .canonicalize()
            .unwrap()
            .join("redirected.yaml");
        fs::write(&target, "after").unwrap();
        fs::write(&redirected, "untouched").unwrap();
        let journal = root.path().join(JOURNAL);
        fs::create_dir(&journal).unwrap();
        let manifest = Manifest {
            changes: vec![change("ontology.yaml", Some("before"), Some("after"))],
            external_target: Some(target.clone()),
            permissions: Default::default(),
            created_dirs: vec![],
        };
        fs::write(
            journal.join("manifest.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        fs::write(&target, "external editor").unwrap();
        assert!(recover(root.path())
            .unwrap_err()
            .to_string()
            .contains("intervening edit"));
        assert!(journal.exists());
        assert_eq!(fs::read_to_string(&target).unwrap(), "external editor");
        fs::write(&target, "after").unwrap();
        assert_eq!(
            recover_with_outcome(root.path()).unwrap(),
            RecoveryOutcome::RolledBack(vec![PathBuf::from("ontology.yaml")])
        );
        assert_eq!(fs::read_to_string(&target).unwrap(), "before");
        assert_eq!(fs::read_to_string(&redirected).unwrap(), "untouched");
    }

    #[test]
    fn injected_publish_failure_restores_deleted_files_and_created_directories() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("old.txt"), "old bytes").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(
                root.path().join("old.txt"),
                fs::Permissions::from_mode(0o640),
            )
            .unwrap();
        }
        let mut published = 0;
        let error = commit_with_hook(
            root.path(),
            vec![
                change("old.txt", Some("old bytes"), None),
                change("new/deep/file.txt", None, Some("created")),
            ],
            &mut |index| {
                published += 1;
                if index == 1 {
                    Err(OkfError::Io("injected publication failure".into()))
                } else {
                    Ok(())
                }
            },
        )
        .unwrap_err();
        assert!(error.to_string().contains("injected publication failure"));
        assert_eq!(published, 2);
        assert_eq!(
            fs::read_to_string(root.path().join("old.txt")).unwrap(),
            "old bytes"
        );
        assert!(!root.path().join("new").exists());
        assert!(!root.path().join(JOURNAL).exists());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(root.path().join("old.txt"))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o640
            );
        }
    }

    #[test]
    fn recovery_and_competing_commit_are_refused_during_publication() {
        let root = tempfile::tempdir().unwrap();
        commit_with_hook(
            root.path(),
            vec![change("new.txt", None, Some("new"))],
            &mut |_| {
                assert!(recover(root.path())
                    .unwrap_err()
                    .to_string()
                    .contains("active"));
                assert!(
                    commit(root.path(), vec![change("other.txt", None, Some("other"))]).is_err()
                );
                assert_eq!(
                    fs::read_to_string(root.path().join("new.txt")).unwrap(),
                    "new"
                );
                Ok(())
            },
        )
        .unwrap();
        assert!(!root.path().join("other.txt").exists());
        assert!(!root.path().join(JOURNAL).exists());
    }

    #[test]
    fn recovery_cannot_remove_an_initializing_journal() {
        let root = tempfile::tempdir().unwrap();
        let lock = acquire_lock(root.path()).unwrap();
        fs::create_dir(root.path().join(JOURNAL)).unwrap();
        assert!(recover(root.path()).is_err());
        assert!(root.path().join(JOURNAL).exists());
        drop(lock);
        recover(root.path()).unwrap();
        assert!(!root.path().join(JOURNAL).exists());
    }

    #[test]
    fn incomplete_initialization_is_recoverable_without_touching_bundle_bytes() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("existing.txt"), "keep").unwrap();
        let journal = root.path().join(JOURNAL);
        fs::create_dir(&journal).unwrap();
        fs::write(journal.join("manifest.tmp"), "{partial").unwrap();
        recover(root.path()).unwrap();
        assert_eq!(
            fs::read_to_string(root.path().join("existing.txt")).unwrap(),
            "keep"
        );
        assert!(!journal.exists());
    }

    #[test]
    fn committed_cleanup_preserves_published_and_subsequently_edited_bytes() {
        let root = tempfile::tempdir().unwrap();
        let journal = root.path().join(JOURNAL);
        fs::create_dir(&journal).unwrap();
        fs::write(
            journal.join("manifest.json"),
            serde_json::to_vec(&vec![change("a.txt", Some("old"), Some("new"))]).unwrap(),
        )
        .unwrap();
        fs::write(journal.join("committed"), "").unwrap();
        fs::write(root.path().join("a.txt"), "subsequent edit").unwrap();
        assert!(recover(root.path()).unwrap().is_empty());
        assert_eq!(
            fs::read_to_string(root.path().join("a.txt")).unwrap(),
            "subsequent edit"
        );
        assert!(!journal.exists());
    }

    #[test]
    fn ordinary_writer_guard_excludes_transactions_and_other_writers() {
        let root = tempfile::tempdir().unwrap();
        let guard = writer_lock(root.path()).unwrap();
        assert!(writer_lock(root.path()).is_err());
        assert!(commit(root.path(), vec![change("new.txt", None, Some("new"))]).is_err());
        assert!(!root.path().join("new.txt").exists());
        drop(guard);
        commit(root.path(), vec![change("new.txt", None, Some("new"))]).unwrap();
    }

    #[test]
    fn dropping_guard_unlocks_even_if_a_duplicate_descriptor_remains() {
        let root = tempfile::tempdir().unwrap();
        let guard = writer_lock(root.path()).unwrap();
        let duplicate = guard.file.try_clone().unwrap();
        drop(guard);
        commit(root.path(), vec![change("new.txt", None, Some("new"))]).unwrap();
        drop(duplicate);
    }

    #[test]
    fn ordinary_writer_guard_refuses_pending_journal_and_absent_root() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join(JOURNAL)).unwrap();
        assert!(writer_lock(root.path())
            .unwrap_err()
            .to_string()
            .contains("pending transaction"));
        recover(root.path()).unwrap();
        assert!(writer_lock(root.path()).is_ok());
        let absent = root.path().join("absent");
        assert!(writer_lock(&absent).is_err());
        assert!(!absent.exists());
    }

    #[test]
    fn stale_before_image_never_creates_a_journal() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("a.txt"), "current").unwrap();
        assert!(commit(
            root.path(),
            vec![change("a.txt", Some("stale"), Some("new"))]
        )
        .is_err());
        assert_eq!(
            fs::read_to_string(root.path().join("a.txt")).unwrap(),
            "current"
        );
        assert!(!root.path().join(JOURNAL).exists());
    }

    #[cfg(unix)]
    #[test]
    fn replacing_a_file_preserves_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("a.txt"), "old").unwrap();
        fs::set_permissions(root.path().join("a.txt"), fs::Permissions::from_mode(0o600)).unwrap();
        commit(root.path(), vec![change("a.txt", Some("old"), Some("new"))]).unwrap();
        assert_eq!(
            fs::metadata(root.path().join("a.txt"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
}
