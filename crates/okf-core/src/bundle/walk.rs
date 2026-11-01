//! Deterministic traversal of the physical OKF bundle tree.
//!
//! Git ignore rules do not redefine a bundle. We exclude VCS and transaction control metadata and do not follow
//! directory symlinks.

use std::path::{Path, PathBuf};

use crate::error::{OkfError, Result};

const EXCLUDED_DIRS: &[&str] = &[".git", ".okf-transaction"];

/// Walk every regular file below `root`, in bundle-relative order.
pub fn walk_files(root: &Path) -> Result<Vec<PathBuf>> {
    if !root.is_dir() {
        return Err(OkfError::Environment(format!(
            "bundle path is not a directory: {}",
            root.display()
        )));
    }
    let mut out = Vec::new();
    walk_dir(root, root, &mut out, true, &|_| true)?;
    out.sort();
    Ok(out)
}

/// Walk ordinary concept/artifact content, pruning support resources before traversal.
/// Physical snapshots must use `walk_files` instead.
pub fn walk_content_files(root: &Path) -> Result<Vec<PathBuf>> {
    if !root.is_dir() {
        return Err(OkfError::Environment(format!(
            "bundle path is not a directory: {}",
            root.display()
        )));
    }
    let mut out = Vec::new();
    walk_dir(root, root, &mut out, false, &|path| !is_support_path(path))?;
    out.sort();
    Ok(out)
}

/// Walk knowledge Markdown, pruning the root support namespace before traversal.
/// Git ignore rules do not redefine knowledge membership.
pub fn walk_markdown(root: &Path) -> Result<Vec<PathBuf>> {
    if !root.is_dir() {
        return Err(OkfError::Environment(format!(
            "bundle path is not a directory: {}",
            root.display()
        )));
    }
    let mut out = Vec::new();
    walk_dir(root, root, &mut out, false, &|path| {
        !is_support_path(path) && path.extension().and_then(|e| e.to_str()) == Some("md")
    })?;
    out.sort();
    Ok(out)
}

fn walk_dir(
    root: &Path,
    dir: &Path,
    out: &mut Vec<PathBuf>,
    physical: bool,
    include: &dyn Fn(&Path) -> bool,
) -> Result<()> {
    let mut entries = std::fs::read_dir(dir)
        .map_err(|e| OkfError::Io(format!("{}: {e}", dir.display())))?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|e| OkfError::Io(format!("{}: {e}", dir.display())))?;
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let ty = entry
            .file_type()
            .map_err(|e| OkfError::Io(format!("{}: {e}", path.display())))?;
        if ty.is_dir() {
            if !EXCLUDED_DIRS.contains(&name.as_ref())
                && (physical || !is_support_path(path.strip_prefix(root).unwrap_or(&path)))
            {
                walk_dir(root, &path, out, physical, include)?;
            }
        } else if ty.is_file()
            && name != ".okf-transaction.lock"
            && include(path.strip_prefix(root).unwrap_or(&path))
        {
            out.push(
                path.strip_prefix(root)
                    .map_err(|e| OkfError::Internal(e.to_string()))?
                    .to_path_buf(),
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn excludes_transaction_control_files_from_bundle_discovery() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join(".okf-transaction.lock"), "").unwrap();
        std::fs::create_dir(root.path().join(".okf-transaction")).unwrap();
        std::fs::write(
            root.path().join(".okf-transaction/manifest.json"),
            "private before images",
        )
        .unwrap();
        std::fs::write(
            root.path().join(".okf-transaction/backup.md"),
            "private document",
        )
        .unwrap();
        std::fs::write(root.path().join(".authored.md"), "authored").unwrap();
        std::fs::write(root.path().join("note.md"), "note").unwrap();
        assert_eq!(
            walk_files(root.path()).unwrap(),
            vec![PathBuf::from(".authored.md"), PathBuf::from("note.md")]
        );
        assert_eq!(
            walk_markdown(root.path()).unwrap(),
            vec![PathBuf::from(".authored.md"), PathBuf::from("note.md")]
        );
    }

    #[test]
    fn includes_gitignored_files_but_not_git_metadata() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join(".gitignore"), "ignored.md\n").unwrap();
        std::fs::write(root.path().join("ignored.md"), "x").unwrap();
        std::fs::create_dir(root.path().join(".git")).unwrap();
        std::fs::write(root.path().join(".git/internal.md"), "x").unwrap();
        assert_eq!(
            walk_markdown(root.path()).unwrap(),
            vec![PathBuf::from("ignored.md")]
        );
    }
}

/// Root-scoped bundle tooling support namespace (not a physical exclusion).
pub fn is_support_path(relative: &Path) -> bool {
    relative == Path::new("okf-bundle.yaml")
        || relative
            .components()
            .next()
            .is_some_and(|c| c.as_os_str() == ".okf-bundle")
}

#[cfg(test)]
mod support_tests {
    use super::*;
    #[test]
    fn tooling_excludes_root_support_but_preserves_physical_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".okf-bundle")).unwrap();
        std::fs::create_dir_all(dir.path().join("ordinary/.okf-bundle")).unwrap();
        std::fs::write(dir.path().join(".okf-bundle/spec.md"), "instructions").unwrap();
        std::fs::write(
            dir.path().join("ordinary/.okf-bundle/concept.md"),
            "concept",
        )
        .unwrap();
        std::fs::write(dir.path().join("okf-bundle.yaml"), "manifest_version: 1").unwrap();
        assert_eq!(walk_files(dir.path()).unwrap().len(), 3);
        assert_eq!(
            walk_content_files(dir.path()).unwrap(),
            vec![PathBuf::from("ordinary/.okf-bundle/concept.md")]
        );
        assert!(
            crate::bundle::loader::load_concept(dir.path(), ".okf-bundle/spec")
                .unwrap()
                .is_none()
        );
        assert_eq!(
            walk_markdown(dir.path()).unwrap(),
            vec![PathBuf::from("ordinary/.okf-bundle/concept.md")]
        );
    }
}

#[cfg(all(test, unix))]
mod unreadable_support_test {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    #[test]
    fn knowledge_walk_never_enters_unreadable_support_directory() {
        let dir = tempfile::tempdir().unwrap();
        let support = dir.path().join(".okf-bundle");
        std::fs::create_dir(&support).unwrap();
        std::fs::write(dir.path().join("note.md"), "note").unwrap();
        std::fs::set_permissions(&support, std::fs::Permissions::from_mode(0)).unwrap();
        let result = walk_markdown(dir.path());
        std::fs::set_permissions(&support, std::fs::Permissions::from_mode(0o700)).unwrap();
        assert_eq!(result.unwrap(), vec![PathBuf::from("note.md")]);
    }
}
