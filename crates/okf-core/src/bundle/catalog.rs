//! Optional tool-local catalog. Identities do not change standard concept IDs.
use crate::error::{OkfError, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, Deserialize)]
pub struct CatalogFile {
    pub catalog_version: u32,
    pub default_bundle: Option<String>,
    pub catalog_overrides: Option<String>,
    pub bundles: BTreeMap<String, Registration>,
}
#[derive(Debug, Clone, Deserialize)]
pub struct Registration {
    pub location: Location,
    #[serde(flatten)]
    pub settings: BTreeMap<String, serde_yaml::Value>,
}
#[derive(Debug, Clone, Deserialize)]
pub struct Location {
    #[serde(rename = "type")]
    pub type_: String,
    pub path: String,
    pub url: Option<String>,
    #[serde(rename = "ref")]
    pub ref_: Option<String>,
}
#[derive(Debug, Clone, Serialize)]
pub struct CatalogEntry {
    pub id: String,
    pub configured_root: PathBuf,
    pub root: PathBuf,
    pub overridden: bool,
    pub available: bool,
    #[serde(skip)]
    pub settings: BTreeMap<String, serde_yaml::Value>,
}
#[derive(Debug, Clone)]
pub struct Catalog {
    pub path: PathBuf,
    pub default_bundle: Option<String>,
    pub entries: BTreeMap<String, CatalogEntry>,
}
/// Resolve directory registrations without requiring remote locks or acquiring objects.
/// Used by initial update planning and impact analysis; shares override precedence with
/// ordinary context (environment, legacy workspace pointer, catalog pointer).
pub fn declared_directory_entries(path: &Path) -> Result<BTreeMap<String, CatalogEntry>> {
    let file: CatalogFile = serde_yaml::from_slice(&std::fs::read(path)?)
        .map_err(|e| OkfError::Usage(format!("invalid catalog: {e}")))?;
    if !matches!(file.catalog_version, 1 | 2) {
        return Err(OkfError::Usage("unsupported catalog_version".into()));
    }
    let base = path.parent().unwrap_or(Path::new("."));
    let legacy = super::config::find_config(base)
        .map(|p| {
            super::config::load_config(&p).map(|c| {
                c.catalog_overrides
                    .map(|v| p.parent().unwrap_or(base).join(v))
            })
        })
        .transpose()?
        .flatten();
    let override_path = std::env::var_os("OKF_CATALOG_OVERRIDES")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or(legacy)
        .or_else(|| file.catalog_overrides.as_ref().map(|v| base.join(v)));
    let overrides: BTreeMap<String, PathBuf> = if let Some(p) = override_path {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Overrides {
            bundles: BTreeMap<String, String>,
        }
        let overrides: Overrides = serde_yaml::from_slice(&std::fs::read(&p)?)
            .map_err(|e| OkfError::Usage(format!("invalid catalog overrides: {e}")))?;
        let mut resolved = BTreeMap::new();
        for (id, location) in overrides.bundles {
            if !file.bundles.contains_key(&id) {
                return Err(OkfError::Usage(format!(
                    "override names unknown bundle ID: {id}"
                )));
            }
            resolved.insert(
                id,
                absolute(&p.parent().unwrap_or(Path::new(".")).join(location))?,
            );
        }
        resolved
    } else {
        BTreeMap::new()
    };
    let mut result = BTreeMap::new();
    for (id, registration) in file.bundles {
        if !valid_id(&id) {
            return Err(OkfError::Usage(format!("invalid bundle ID: {id}")));
        }
        if registration.location.type_ != "directory" {
            continue;
        }
        let configured_root = absolute(&base.join(registration.location.path))?;
        let root = overrides
            .get(&id)
            .cloned()
            .unwrap_or_else(|| configured_root.clone());
        let root = std::fs::canonicalize(&root).unwrap_or(root);
        result.insert(
            id.clone(),
            CatalogEntry {
                overridden: overrides.contains_key(&id),
                available: root.is_dir(),
                id,
                configured_root,
                root,
                settings: registration.settings,
            },
        );
    }
    let available: Vec<_> = result.values().filter(|entry| entry.available).collect();
    for (index, first) in available.iter().enumerate() {
        for second in &available[index + 1..] {
            if first.root.starts_with(&second.root) || second.root.starts_with(&first.root) {
                return Err(OkfError::Usage(format!(
                    "conflicting consumer roots: {} and {} are equal or nested",
                    first.id, second.id
                )));
            }
        }
    }
    Ok(result)
}
/// Machine-local override file: `bundles: {acme.finance: /local/checkout}`.
/// Paths are relative to the override file. Only registered IDs can be overridden.
pub fn load_catalog(path: &Path, overrides: Option<&Path>) -> Result<Catalog> {
    let text = std::fs::read_to_string(path)?;
    let value: serde_yaml::Value = serde_yaml::from_str(&text)
        .map_err(|e| OkfError::Usage(format!("invalid catalog {}: {e}", path.display())))?;
    let file: CatalogFile = serde_yaml::from_value(value)
        .map_err(|e| OkfError::Usage(format!("invalid catalog {}: {e}", path.display())))?;
    if !matches!(file.catalog_version, 1 | 2) {
        return Err(OkfError::Usage(
            "unsupported catalog_version (expected 1 or 2)".into(),
        ));
    }
    if file.catalog_version == 1 && file.default_bundle.is_some() {
        return Err(OkfError::Usage(
            "default_bundle requires catalog_version 2".into(),
        ));
    }
    let configured_override = file
        .catalog_overrides
        .as_ref()
        .map(|p| path.parent().unwrap_or(Path::new(".")).join(p));
    let overrides = overrides.or(configured_override.as_deref());
    let mut entries = BTreeMap::new();
    for (id, registration) in file.bundles {
        if !valid_id(&id) {
            return Err(OkfError::Usage(format!(
                "bundle ID must be namespaced: {id:?}"
            )));
        }
        let mut settings = registration.settings;
        let root = match registration.location.type_.as_str() {
            "directory" => absolute(
                &path
                    .parent()
                    .unwrap_or(Path::new("."))
                    .join(&registration.location.path),
            )?,
            "git" => {
                let resolved = super::remote::resolve_registration(path, &id)?;
                settings.insert(
                    "remote".into(),
                    serde_yaml::to_value(&resolved.origin)
                        .map_err(|e| OkfError::Usage(e.to_string()))?,
                );
                resolved.root
            }
            _ => {
                return Err(OkfError::Usage(format!(
                    "unsupported location type for {id}"
                )))
            }
        };
        entries.insert(
            id.clone(),
            CatalogEntry {
                id,
                configured_root: root.clone(),
                root,
                overridden: false,
                available: false,
                settings,
            },
        );
    }
    if let Some(overrides) = overrides {
        #[derive(Deserialize)]
        struct Overrides {
            bundles: BTreeMap<String, String>,
        }
        let override_text = std::fs::read_to_string(overrides)?;
        let value: serde_yaml::Value = serde_yaml::from_str(&override_text)
            .map_err(|e| OkfError::Usage(format!("invalid catalog overrides: {e}")))?;
        let file: Overrides = serde_yaml::from_value(value)
            .map_err(|e| OkfError::Usage(format!("invalid catalog overrides: {e}")))?;
        for (id, root) in file.bundles {
            let entry = entries.get_mut(&id).ok_or_else(|| {
                OkfError::Usage(format!("override names unknown bundle ID: {id}"))
            })?;
            entry.root = absolute(&overrides.parent().unwrap_or(Path::new(".")).join(root))?;
            if entry.settings.contains_key("remote") {
                super::remote::validate_override(path, &id, &entry.root)?;
            }
            entry.overridden = true;
        }
    }
    for entry in entries.values_mut() {
        entry.available = entry.root.is_dir()
            && entry
                .settings
                .get("remote")
                .map(|origin| {
                    entry.overridden
                        || origin.get("state").and_then(|v| v.as_str()) == Some("matched")
                })
                .unwrap_or(true);
        if entry.available {
            entry.root = std::fs::canonicalize(&entry.root)?;
        }
    }
    let available: Vec<_> = entries.values().filter(|e| e.available).collect();
    for (index, first) in available.iter().enumerate() {
        for second in &available[index + 1..] {
            if first.root.starts_with(&second.root) || second.root.starts_with(&first.root) {
                return Err(OkfError::Usage(format!(
                    "conflicting bundle roots: {} ({}) and {} ({}) are equal or nested",
                    first.id,
                    first.root.display(),
                    second.id,
                    second.root.display()
                )));
            }
        }
    }
    if let Some(id) = &file.default_bundle {
        if !entries.contains_key(id) {
            return Err(OkfError::Usage(format!(
                "default_bundle names unknown ID: {id}"
            )));
        }
    }
    Ok(Catalog {
        path: path.to_path_buf(),
        default_bundle: file.default_bundle,
        entries,
    })
}
impl Catalog {
    pub fn entry(&self, id: &str) -> Result<&CatalogEntry> {
        self.entries
            .get(id)
            .ok_or_else(|| OkfError::Usage(format!("unknown bundle ID: {id}")))
    }
    pub fn containing(&self, path: &Path) -> Option<&CatalogEntry> {
        let path = std::fs::canonicalize(path).unwrap_or_else(|_| normalize(path));
        self.entries
            .values()
            .find(|entry| entry.available && path.starts_with(&entry.root))
    }
}
pub fn valid_id(id: &str) -> bool {
    let segments: Vec<_> = id.split('.').collect();
    segments.len() >= 2
        && segments.iter().all(|s| {
            !s.is_empty()
                && s.chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        })
}
pub fn absolute(path: &Path) -> Result<PathBuf> {
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    Ok(normalize(&path))
}
/// Normalize actual filesystem paths without clamping traversal to a bundle root.
pub fn normalize(path: &Path) -> PathBuf {
    let mut result = PathBuf::new();
    for part in path.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => {
                result.pop();
            }
            part => result.push(part.as_os_str()),
        }
    }
    result
}

/// Discover the nearest workspace catalog without merging ancestor catalogs.
pub fn discover(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .map(|p| p.join("okf-catalog.yaml"))
        .find(|p| p.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(yaml: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("a")).unwrap();
        std::fs::create_dir(dir.path().join("b")).unwrap();
        std::fs::write(dir.path().join("catalog.yaml"), yaml).unwrap();
        dir
    }
    #[test]
    fn validates_namespaced_ids_and_duplicate_registrations() {
        for bundles in ["plain: {location: {type: directory, path: a}}","acme.a: {location: {type: directory, path: a}}\n  acme.a: {location: {type: directory, path: b}}"] {
            let dir=fixture(&format!("catalog_version: 1\nbundles:\n  {bundles}\n"));
            assert_eq!(load_catalog(&dir.path().join("catalog.yaml"),None).unwrap_err().exit_code(),2);
        }
    }
    #[test]
    fn rejects_equal_and_nested_roots_without_treating_unavailable_as_verified() {
        for path in ["a", "a/child"] {
            let dir=fixture(&format!("catalog_version: 1\nbundles:\n  acme.a: {{location: {{type: directory, path: a}}}}\n  acme.b: {{location: {{type: directory, path: {path}}}}}\n"));
            if path == "a/child" {
                std::fs::create_dir(dir.path().join(path)).unwrap();
            }
            assert!(load_catalog(&dir.path().join("catalog.yaml"), None).is_err());
        }
        let dir=fixture("catalog_version: 1\nbundles:\n  acme.a: {location: {type: directory, path: absent}}\n  acme.b: {location: {type: directory, path: absent/child}}\n");
        let catalog = load_catalog(&dir.path().join("catalog.yaml"), None).unwrap();
        assert!(catalog.entries.values().all(|e| !e.available));
    }
    #[test]
    fn overrides_preserve_configured_root_and_validate_effective_overlap() {
        let dir=fixture("catalog_version: 1\nbundles:\n  acme.a: {location: {type: directory, path: absent}}\n  acme.b: {location: {type: directory, path: b}}\n");
        let overrides = dir.path().join("local.yaml");
        std::fs::write(&overrides, "bundles: {acme.a: a}\n").unwrap();
        let catalog = load_catalog(&dir.path().join("catalog.yaml"), Some(&overrides)).unwrap();
        let entry = &catalog.entries["acme.a"];
        assert!(entry.available && entry.overridden);
        assert!(entry.configured_root.ends_with("absent"));
        assert!(entry.root.ends_with("a"));
        std::fs::write(&overrides, "bundles: {acme.a: b}\n").unwrap();
        assert!(load_catalog(&dir.path().join("catalog.yaml"), Some(&overrides)).is_err());
    }
}
