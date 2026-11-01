//! Selection and catalog discovery, independent from examination scope.
use super::catalog::{absolute, load_catalog, Catalog};
use super::config::{find_config, load_config, Config};
use crate::error::{OkfError, Result};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct SelectedBundle {
    pub id: Option<String>,
    pub root: PathBuf,
}
#[derive(Debug, Clone)]
pub struct Context {
    pub primary: SelectedBundle,
    pub catalog: Option<Catalog>,
    pub config: Config,
    pub config_path: Option<PathBuf>,
}
pub fn resolve_context(
    path: Option<&str>,
    id: Option<&str>,
    env: Option<&str>,
    cwd: &Path,
) -> Result<Context> {
    if path.is_some() && id.is_some() {
        return Err(OkfError::Usage(
            "bundle path and --bundle-id are conflicting selectors".into(),
        ));
    }
    // Explicit path access is independent of catalog validity: bad tool configuration cannot
    // prohibit consuming otherwise valid OKF directly.
    let config_path = find_config(cwd);
    let config = config_path
        .as_ref()
        .map(|path| load_config(path))
        .transpose()?
        .unwrap_or_default();
    let base = config_path
        .as_ref()
        .and_then(|path| path.parent())
        .unwrap_or(cwd);
    let overrides = std::env::var("OKF_CATALOG_OVERRIDES")
        .ok()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .or_else(|| config.catalog_overrides.as_ref().map(|s| base.join(s)));
    let explicit_catalog = std::env::var("OKF_CATALOG")
        .ok()
        .filter(|s| !s.is_empty())
        .map(|s| cwd.join(s));
    let selected_catalog = explicit_catalog
        .clone()
        .or_else(|| super::catalog::discover(cwd))
        .or_else(|| config.catalog.as_ref().map(|p| base.join(p)));
    if explicit_catalog.is_none() {
        if let (Some(discovered), Some(old)) = (&selected_catalog, &config.catalog) {
            let old = base.join(old);
            if super::catalog::absolute(discovered)? != super::catalog::absolute(&old)? {
                return Err(OkfError::Usage(
                    "legacy and discovered catalog locations conflict".into(),
                ));
            }
        }
    }
    let catalog = match selected_catalog
        .as_ref()
        .map(|p| load_catalog(p, overrides.as_deref()))
        .transpose()
    {
        Ok(catalog) => catalog,
        Err(_) if explicit_catalog.is_none() && (path.is_some() || env.is_some()) => None,
        Err(error) => return Err(error),
    };
    if let (Some(catalog), Some(old)) = (&catalog, &config.default_bundle) {
        if let Some(new) = &catalog.default_bundle {
            let same = old.id.as_ref() == Some(new)
                || old.path.as_ref().is_some_and(|p| {
                    catalog.entries.get(new).is_some_and(|e| {
                        super::catalog::absolute(&base.join(p)).ok().as_ref() == Some(&e.root)
                    })
                });
            if !same {
                return Err(OkfError::Usage(
                    "legacy and catalog default_bundle conflict".into(),
                ));
            }
        }
    }
    if let Some(catalog) = &catalog {
        if let Some(default) = &catalog.default_bundle {
            if let Some(old) = &config.bundle {
                if let Some(entry) = catalog.entries.get(default) {
                    if super::catalog::absolute(&base.join(old))? != entry.root {
                        return Err(OkfError::Usage(
                            "legacy bundle and catalog default_bundle conflict".into(),
                        ));
                    }
                }
            }
        }
        if let (Some(entry), Some(manifest_root)) = (
            catalog.containing(cwd),
            cwd.ancestors()
                .find(|p| p.join(super::manifest::FILENAME).is_file()),
        ) {
            if std::fs::canonicalize(manifest_root).unwrap_or_else(|_| manifest_root.to_path_buf())
                != entry.root
            {
                return Err(OkfError::Usage(
                    "containing manifest contradicts registered bundle boundary".into(),
                ));
            }
        }
    }
    let path_selection = |path: PathBuf| -> Result<SelectedBundle> {
        let root = absolute(&path)?;
        let root = std::fs::canonicalize(&root).unwrap_or(root);
        let id = catalog
            .as_ref()
            .and_then(|c| c.entries.values().find(|entry| entry.root == root))
            .map(|entry| entry.id.clone());
        Ok(SelectedBundle { id, root })
    };
    let id_selection = |id: &str| -> Result<SelectedBundle> {
        let catalog = catalog
            .as_ref()
            .ok_or_else(|| OkfError::Usage("bundle ID requires a catalog".into()))?;
        let entry = catalog.entry(id)?;
        Ok(SelectedBundle {
            id: Some(id.into()),
            root: entry.root.clone(),
        })
    };
    let primary = if let Some(path) = path {
        path_selection(cwd.join(path))?
    } else if let Some(id) = id {
        id_selection(id)?
    } else if let Some(env) = env.filter(|s| !s.is_empty()) {
        path_selection(cwd.join(env))?
    } else if let Some(entry) = catalog.as_ref().and_then(|c| c.containing(cwd)) {
        SelectedBundle {
            id: Some(entry.id.clone()),
            root: entry.root.clone(),
        }
    } else if let Some(root) = cwd
        .ancestors()
        .find(|p| p.join(super::manifest::FILENAME).is_file())
    {
        path_selection(root.to_path_buf())?
    } else if let Some(default) = catalog.as_ref().and_then(|c| c.default_bundle.as_deref()) {
        id_selection(default)?
    } else if let Some(selector) = &config.default_bundle {
        if let Some(id) = &selector.id {
            id_selection(id)?
        } else {
            path_selection(base.join(selector.path.as_ref().unwrap()))?
        }
    } else if let Some(path) = &config.bundle {
        path_selection(base.join(path))?
    } else if let Some(catalog) = &catalog {
        if catalog.entries.len() == 1 {
            id_selection(catalog.entries.keys().next().unwrap())?
        } else {
            return Err(OkfError::Usage(format!(
                "select a bundle with --bundle-id or an explicit path; available IDs: {}",
                catalog
                    .entries
                    .keys()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ")
            )));
        }
    } else {
        path_selection(cwd.to_path_buf())?
    };
    Ok(Context {
        primary,
        catalog,
        config,
        config_path,
    })
}
pub fn current_context(path: Option<&str>, id: Option<&str>) -> Result<Context> {
    if let Some(context) = super::snapshot_context::active_context(path, id)? {
        return Ok(context);
    }
    let env = std::env::var(super::resolve::ENV_BUNDLE).ok();
    resolve_context(path, id, env.as_deref(), &std::env::current_dir()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for path in ["a/nested", "b"] {
            std::fs::create_dir_all(dir.path().join(path)).unwrap();
        }
        std::fs::write(
            dir.path().join("okf.toml"),
            "catalog='catalog.yaml'\ndefault_bundle={id='acme.b'}\n",
        )
        .unwrap();
        std::fs::write(dir.path().join("catalog.yaml"),"catalog_version: 1\nbundles:\n  acme.a: {location: {type: directory, path: a}}\n  acme.b: {location: {type: directory, path: b}}\n").unwrap();
        dir
    }
    #[test]
    fn typed_selection_and_cwd_precedence() {
        let dir = fixture();
        let cwd = dir.path().join("a/nested");
        assert_eq!(
            resolve_context(None, None, None, &cwd)
                .unwrap()
                .primary
                .id
                .as_deref(),
            Some("acme.a")
        );
        assert_eq!(
            resolve_context(None, Some("acme.b"), Some("../.."), &cwd)
                .unwrap()
                .primary
                .id
                .as_deref(),
            Some("acme.b")
        );
        assert_eq!(
            resolve_context(None, None, None, dir.path())
                .unwrap()
                .primary
                .id
                .as_deref(),
            Some("acme.b")
        );
        assert!(resolve_context(Some("a"), Some("acme.a"), None, dir.path()).is_err());
        assert!(resolve_context(None, Some("a"), None, dir.path()).is_err());
    }
    #[test]
    fn config_alias_accepts_only_equivalent_paths() {
        let dir = fixture();
        for config in [
            "bundle='a'\ndefault_bundle={id='a'}",
            "default_bundle={id='acme.a',path='a'}",
            "default_bundle={}",
            "bundle='a'\ndefault_bundle={path='b'}",
        ] {
            std::fs::write(dir.path().join("okf.toml"), config).unwrap();
            assert!(resolve_context(None, None, None, dir.path()).is_err());
        }
        std::fs::write(
            dir.path().join("okf.toml"),
            "bundle='a'\ndefault_bundle={path='./a'}",
        )
        .unwrap();
        assert!(resolve_context(None, None, None, dir.path())
            .unwrap()
            .primary
            .root
            .ends_with("a"));
    }
    #[test]
    fn invalid_catalog_does_not_prohibit_explicit_standalone_consumption() {
        let dir = fixture();
        std::fs::write(dir.path().join("catalog.yaml"), "not a catalog").unwrap();
        assert!(resolve_context(None, None, None, dir.path()).is_err());
        let context = resolve_context(Some("a"), None, None, dir.path()).unwrap();
        assert!(context.catalog.is_none());
        assert!(context.primary.root.ends_with("a"));
    }
}

#[cfg(test)]
mod manifest_context_tests {
    use super::*;
    #[test]
    fn yaml_catalog_default_and_standalone_manifest_discovery() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("bundle/nested")).unwrap();
        std::fs::write(
            dir.path().join("bundle/okf-bundle.yaml"),
            "manifest_version: 1\n",
        )
        .unwrap();
        let context = resolve_context(None, None, None, &dir.path().join("bundle/nested")).unwrap();
        assert_eq!(
            context.primary.root,
            std::fs::canonicalize(dir.path().join("bundle")).unwrap()
        );
        std::fs::write(dir.path().join("okf-catalog.yaml"),"catalog_version: 2\ndefault_bundle: example.docs\nbundles:\n  example.docs:\n    location: {type: directory, path: bundle}\n").unwrap();
        let context = resolve_context(None, None, None, dir.path()).unwrap();
        assert_eq!(context.primary.id.as_deref(), Some("example.docs"));
    }
    #[test]
    fn conflicting_legacy_default_is_visible() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("a")).unwrap();
        std::fs::create_dir(dir.path().join("b")).unwrap();
        std::fs::write(dir.path().join("okf-catalog.yaml"),"catalog_version: 2\ndefault_bundle: example.a\nbundles:\n  example.a:\n    location: {type: directory, path: a}\n").unwrap();
        std::fs::write(dir.path().join("okf.toml"), "bundle = 'b'\n").unwrap();
        assert!(resolve_context(None, None, None, dir.path()).is_err());
    }
    #[test]
    fn nested_manifest_cannot_redefine_registered_boundary() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("a/nested")).unwrap();
        std::fs::write(dir.path().join("okf-catalog.yaml"),"catalog_version: 2\nbundles:\n  example.a:\n    location: {type: directory, path: a}\n").unwrap();
        std::fs::write(
            dir.path().join("a/nested/okf-bundle.yaml"),
            "manifest_version: 1\n",
        )
        .unwrap();
        assert!(resolve_context(None, None, None, &dir.path().join("a/nested")).is_err());
    }
}
