//! okf config file model (`okf.toml`).
//!
//! A tool-local, project-scoped config discovered by walking up from the current directory.
//! Its `bundle` key names a default bundle directory (relative to the config file, or absolute).
//! The bundle itself stays spec-pure — this file lives outside it.
use crate::error::{OkfError, Result};
use std::path::{Path, PathBuf};

/// The config filename searched for from the cwd upward.
pub const CONFIG_FILENAME: &str = "okf.toml";

/// Parsed `okf.toml`. Unknown keys are ignored so the format can grow compatibly.
///
/// `default_bundle` is filled in by [`Config::validate`], which every loader runs
/// after deserialization, so a selector with both or neither key reports the
/// usage error below rather than a TOML parse error.
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct Config {
    /// Default bundle directory, relative to the config file's directory (or absolute).
    pub bundle: Option<String>,
    pub catalog: Option<String>,
    pub catalog_overrides: Option<String>,
    #[serde(skip)]
    pub default_bundle: Option<BundleSelector>,
    /// The authored `default_bundle` table; `validate` converts it.
    #[serde(rename = "default_bundle")]
    raw_default_bundle: Option<RawBundleSelector>,
    #[serde(flatten)]
    pub settings: std::collections::BTreeMap<String, toml::Value>,
}

/// Explicit typed default: exactly one identity or directory path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BundleSelector {
    Id(String),
    Path(String),
}

impl BundleSelector {
    pub fn id(&self) -> Option<&str> {
        match self {
            Self::Id(id) => Some(id),
            Self::Path(_) => None,
        }
    }

    pub fn path(&self) -> Option<&str> {
        match self {
            Self::Path(path) => Some(path),
            Self::Id(_) => None,
        }
    }
}

/// `default_bundle = { id = … }` or `{ path = … }` as authored.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawBundleSelector {
    id: Option<String>,
    path: Option<String>,
}

impl Config {
    /// Check cross-field rules and resolve the typed `default_bundle`.
    pub fn validate(&mut self, base: &Path) -> Result<()> {
        if let Some(raw) = self.raw_default_bundle.take() {
            self.default_bundle = Some(match (raw.id, raw.path) {
                (Some(id), None) => BundleSelector::Id(id),
                (None, Some(path)) => BundleSelector::Path(path),
                _ => {
                    return Err(OkfError::Usage(
                        "default_bundle requires exactly one of id and path".into(),
                    ));
                }
            });
        }
        if let Some(selector) = &self.default_bundle
            && let Some(old) = &self.bundle
        {
            let equivalent = selector.path().is_some_and(|path| {
                let a = base.join(old);
                let b = base.join(path);
                match (std::fs::canonicalize(&a), std::fs::canonicalize(&b)) {
                    (Ok(a), Ok(b)) => a == b,
                    _ => super::catalog::normalize(&a) == super::catalog::normalize(&b),
                }
            });
            if !equivalent {
                return Err(OkfError::Usage("bundle and default_bundle conflict; only equivalent path selectors may coexist".into()));
            }
        }
        Ok(())
    }
}

/// Walk up from `start` (inclusive) to the filesystem root, returning the first `okf.toml` found.
pub fn find_config(start: &Path) -> Option<PathBuf> {
    let mut dir = Some(start);
    while let Some(d) = dir {
        let candidate = d.join(CONFIG_FILENAME);
        if candidate.is_file() {
            return Some(candidate);
        }
        dir = d.parent();
    }
    None
}

/// Read and parse a config file. A present-but-malformed file is a usage error.
pub fn load_config(path: &Path) -> Result<Config> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| OkfError::Environment(format!("cannot read {}: {e}", path.display())))?;
    let mut config: Config = toml::from_str(&text)
        .map_err(|e| OkfError::Usage(format!("invalid config {}: {e}", path.display())))?;
    config.validate(path.parent().unwrap_or(Path::new(".")))?;
    Ok(config)
}

/// Resolve the bundle a config file points at, relative to the config's own directory.
/// Returns `None` when no config is found or it has no `bundle` key.
pub fn config_bundle(start: &Path) -> Result<Option<PathBuf>> {
    let Some(cfg_path) = find_config(start) else {
        return Ok(None);
    };
    let cfg = load_config(&cfg_path)?;
    let Some(bundle) = cfg
        .default_bundle
        .and_then(|selector| match selector {
            BundleSelector::Path(path) => Some(path),
            BundleSelector::Id(_) => None,
        })
        .or(cfg.bundle)
    else {
        return Ok(None);
    };
    let base = cfg_path.parent().unwrap_or(start);
    // `Path::join` returns `bundle` as-is when it is absolute, else joins onto `base`.
    Ok(Some(base.join(bundle)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load(text: &str) -> Result<Config> {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(CONFIG_FILENAME);
        std::fs::write(&path, text).unwrap();
        load_config(&path)
    }

    #[test]
    fn default_bundle_selects_exactly_one_of_id_and_path() {
        assert_eq!(
            load("default_bundle = { id = 'acme.a' }\n")
                .unwrap()
                .default_bundle,
            Some(BundleSelector::Id("acme.a".into()))
        );
        assert_eq!(
            load("[default_bundle]\npath = 'a'\n")
                .unwrap()
                .default_bundle,
            Some(BundleSelector::Path("a".into()))
        );
        for text in [
            "default_bundle = { id = 'acme.a', path = 'a' }\n",
            "default_bundle = {}\n",
        ] {
            assert_eq!(
                load(text).unwrap_err().to_string(),
                "usage error: default_bundle requires exactly one of id and path"
            );
        }
        let unknown = load("default_bundle = { name = 'a' }\n").unwrap_err();
        assert!(
            unknown.to_string().contains("unknown field `name`"),
            "{unknown}"
        );
    }
}
