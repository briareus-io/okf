//! Optional local interpretation settings. Named settings never register or leak to other bundles.
use super::context::{Context, current_context};
use crate::{
    error::{OkfError, Result},
    ontology::{load, schema::Ontology},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuerySettings {
    pub scan_limit: Option<usize>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LintSettings {
    #[serde(default)]
    pub index_exclude: Vec<String>,
    pub finding_budget: Option<usize>,
    pub ontology_violation: Option<String>,
    pub index_coverage: Option<String>,
    pub source_unrecorded: Option<String>,
    pub broken_link: Option<String>,
    pub invalid_reference: Option<String>,
    pub missing_title: Option<String>,
    pub frontmatter_order: Option<String>,
    pub missing_description: Option<String>,
    pub orphan: Option<String>,
    pub spec_v02: Option<String>,
    pub source_missing: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_max_lines: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_max_lines_limit: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frontmatter_max_lines: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frontmatter_max_lines_limit: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_line_length: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_line_length_limit: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_line_length_scope: Option<LineScope>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_line_length_exclude_code_blocks: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_line_length_exclude_tables: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_line_length_exclude_links: Option<bool>,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LineScope {
    #[default]
    Body,
    Frontmatter,
    All,
}
impl LintSettings {
    fn size_rules(&self) -> [(&str, Option<&str>, Option<usize>); 3] {
        [
            (
                "body_max_lines",
                self.body_max_lines.as_deref(),
                self.body_max_lines_limit,
            ),
            (
                "frontmatter_max_lines",
                self.frontmatter_max_lines.as_deref(),
                self.frontmatter_max_lines_limit,
            ),
            (
                "max_line_length",
                self.max_line_length.as_deref(),
                self.max_line_length_limit,
            ),
        ]
    }
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WritePolicy {
    #[serde(default)]
    pub ontology_gate: bool,
    pub ontology_severity: Option<String>,
    pub blocking_threshold: Option<String>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ViewSettings {
    #[serde(default)]
    pub columns: Vec<String>,
    #[serde(default)]
    pub expand: Vec<String>,
    #[serde(default)]
    pub extended_output: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FacetSettings {
    #[serde(default = "default_fields")]
    pub fields: Vec<String>,
    #[serde(default = "threshold")]
    pub max_distinct_values: usize,
    #[serde(default)]
    pub include_high_cardinality: Vec<String>,
}
fn default_fields() -> Vec<String> {
    vec!["type".into(), "tags[]".into()]
}
fn threshold() -> usize {
    100
}
impl Default for FacetSettings {
    fn default() -> Self {
        Self {
            fields: default_fields(),
            max_distinct_values: 100,
            include_high_cardinality: Vec::new(),
        }
    }
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BundleSettings {
    #[serde(default)]
    pub formatting: FormattingSettings,
    pub ontology: Option<String>,
    #[serde(default)]
    pub query_presets: BTreeMap<String, QueryPreset>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub saved_queries: BTreeMap<String, SavedQuery>,
    #[serde(default)]
    pub query: QuerySettings,
    #[serde(default)]
    pub lint: LintSettings,
    #[serde(default)]
    pub write_policy: WritePolicy,
    #[serde(default)]
    pub views: BTreeMap<String, ViewSettings>,
    #[serde(default)]
    pub facets: FacetSettings,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QueryPreset {
    pub query: Option<PrimaryQueryPreset>,
    pub description: Option<String>,
    pub operations: Vec<String>,
    #[serde(default)]
    pub filters: crate::graph::filter::EdgeFilters,
    #[serde(default)]
    pub details: bool,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PrimaryQueryPreset {
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub tag: Option<String>,
    pub text: Vec<String>,
    pub field: Vec<String>,
    pub facet_filter: Vec<String>,
}
impl PrimaryQueryPreset {
    pub fn validate(&self) -> Result<()> {
        for value in self
            .type_
            .iter()
            .chain(self.tag.iter())
            .chain(self.text.iter())
        {
            if value.trim().is_empty() {
                return Err(OkfError::Usage(
                    "primary query filter values must not be empty".into(),
                ));
            }
        }
        for field in &self.field {
            let (key, _) = field
                .split_once('=')
                .ok_or_else(|| OkfError::Usage("field filter requires key=value".into()))?;
            if key.is_empty() {
                return Err(OkfError::Usage(
                    "field filter requires a nonempty key".into(),
                ));
            }
        }
        for filter in &self.facet_filter {
            crate::query::metadata::Condition::parse(filter)?;
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct EffectiveSettings {
    pub bundle: Option<String>,
    pub ontology_path: Option<PathBuf>,
    pub ontology_digest: Option<String>,
    pub settings: BundleSettings,
    #[serde(skip)]
    pub(crate) override_origins: BTreeMap<String, String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OverrideFile {
    settings: OverrideSettings,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OverrideSettings {
    lint: BTreeMap<String, yaml_serde::Value>,
}

// Apply only authored lint keys. Collections such as index_exclude are one setting.
fn apply_overrides(settings: &mut BundleSettings) -> Result<BTreeMap<String, String>> {
    let mut origins = BTreeMap::new();
    let Some(path) = std::env::var_os("OKF_SETTINGS_OVERRIDES") else {
        return Ok(origins);
    };
    let path = PathBuf::from(path);
    let error = |message: String| {
        OkfError::Usage(format!(
            "OKF_SETTINGS_OVERRIDES {}: {message}",
            path.display()
        ))
    };
    let bytes = std::fs::read(&path).map_err(|e| error(e.to_string()))?;
    let file: OverrideFile = yaml_serde::from_slice(&bytes).map_err(|e| error(e.to_string()))?;
    let mut merged = yaml_serde::to_value(&settings.lint).map_err(|e| error(e.to_string()))?;
    for (key, value) in file.settings.lint {
        if value.is_null() {
            return Err(error(format!("null override for settings.lint.{key}")));
        }
        merged[&key] = value;
        origins.insert(
            format!("lint.{key}"),
            format!("override:{}", path.display()),
        );
    }
    settings.lint = yaml_serde::from_value(merged).map_err(|e| error(e.to_string()))?;
    // LintSettings tolerates legacy unknown keys. Explicit overrides must take effect.
    let accepted = yaml_serde::to_value(&settings.lint).map_err(|e| error(e.to_string()))?;
    for key in origins.keys() {
        if accepted.get(&key["lint.".len()..]).is_none() {
            return Err(error(format!("unknown setting {key}")));
        }
    }
    validate_settings(settings).map_err(|e| error(e.to_string()))?;
    Ok(origins)
}

fn resolve_settings(context: &Context, root: &Path) -> Result<EffectiveSettings> {
    let abs = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    let id = context
        .catalog
        .as_ref()
        .and_then(|c| c.entries.values().find(|e| e.root == abs))
        .map(|e| e.id.clone());
    let table = context
        .config
        .settings
        .get("bundle_settings")
        .and_then(toml::Value::as_table);
    let key = id.as_deref().unwrap_or("default");
    let legacy = table.and_then(|t| {
        if id.is_none() && context.catalog.is_some() {
            None
        } else {
            t.get(key)
        }
    });
    let loaded = super::manifest::load(root)?;
    let mut legacy_value = legacy
        .map(yaml_serde::to_value)
        .transpose()
        .map_err(|e| OkfError::Usage(e.to_string()))?
        .unwrap_or_else(|| yaml_serde::Value::Mapping(Default::default()));
    let base = context
        .config_path
        .as_ref()
        .and_then(|p| p.parent())
        .unwrap_or(root);
    // Rebase the legacy ontology before comparing it with bundle-relative manifest paths.
    if let Some(old) = legacy_value.get_mut("ontology")
        && let Some(path) = old.as_str()
    {
        *old = yaml_serde::Value::String(
            super::catalog::absolute(&base.join(path))?
                .to_string_lossy()
                .into_owned(),
        );
    }
    if let Some(manifest) = &loaded {
        let mut portable = manifest.settings_value.clone();
        if let Some(value) = portable.get_mut("ontology")
            && let Some(path) = value.as_str()
        {
            *value = yaml_serde::Value::String(
                super::catalog::absolute(&root.join(path))?
                    .to_string_lossy()
                    .into_owned(),
            );
        }
        merge_settings(&mut legacy_value, portable, "settings")?;
    }
    let mut settings: BundleSettings = yaml_serde::from_value(legacy_value)
        .map_err(|e| OkfError::Usage(format!("invalid effective bundle settings: {e}")))?;
    validate_settings(&settings)?;
    let override_origins = apply_overrides(&mut settings)?;
    let base = context
        .config_path
        .as_ref()
        .and_then(|p| p.parent())
        .unwrap_or(root);
    let ontology_path = if let Some(path) = &settings.ontology {
        Some(base.join(path))
    } else {
        load::find_ontology(root)
    };
    Ok(EffectiveSettings {
        bundle: id,
        settings,
        ontology_path,
        ontology_digest: None,
        override_origins,
    })
}

/// Resolve the optional ontology path without reading it, so diagnostics can identify a
/// configured sidecar even when it is missing or unreadable.
pub fn ontology_path_for(context: &Context, root: &Path) -> Result<Option<PathBuf>> {
    resolve_settings(context, root).map(|effective| effective.ontology_path)
}

pub fn for_context(context: &Context, root: &Path) -> Result<EffectiveSettings> {
    let mut effective = resolve_settings(context, root)?;
    effective.ontology_digest = effective
        .ontology_path
        .as_ref()
        .map(|p| std::fs::read(p).map(|bytes| crate::fingerprint::canonicalize::sha256_hex(&bytes)))
        .transpose()
        .map_err(|e| OkfError::Environment(format!("cannot read configured ontology: {e}")))?;
    Ok(effective)
}
pub fn load_for(root: &Path) -> Result<(EffectiveSettings, Option<Ontology>)> {
    let context = current_context(Some(&root.to_string_lossy()), None)?;
    load_in(&context, root)
}
pub fn load_in(context: &Context, root: &Path) -> Result<(EffectiveSettings, Option<Ontology>)> {
    let effective = for_context(context, root)?;
    let ontology = effective
        .ontology_path
        .as_ref()
        .map(|p| load::load_ontology(p))
        .transpose()?;
    Ok((effective, ontology))
}

enum HistoricalFile {
    External,
    Missing,
    Bytes(Vec<u8>),
}
fn historical(root: &Path, path: &Path, revision: &str) -> Result<HistoricalFile> {
    let run = |args: &[&str]| crate::ports::git_read::output(root, args);
    let repo = run(&["rev-parse", "--show-toplevel"])?;
    if !repo.status.success() {
        return Err(OkfError::Environment(
            "historical settings require available local Git repository".into(),
        ));
    }
    let repo = PathBuf::from(String::from_utf8_lossy(&repo.stdout).trim());
    // Revision and top-relative inventory reads can be shared across bundles in one repo.
    let run = |args: &[&str]| crate::ports::git_read::output(&repo, args);
    let abs = super::catalog::absolute(path)?;
    let Ok(rel) = abs.strip_prefix(&repo) else {
        return Ok(HistoricalFile::External);
    };
    if !run(&["rev-parse", "--verify", &format!("{revision}^{{commit}}")])?
        .status
        .success()
    {
        return Err(OkfError::Environment(format!(
            "historical settings revision unavailable: {revision}"
        )));
    }
    let inventory = run(&[
        "ls-tree",
        "-z",
        revision,
        "--",
        &format!(":(top){}", rel.to_string_lossy()),
    ])?;
    if !inventory.status.success() {
        return Err(OkfError::Environment(format!(
            "cannot inspect historical settings {}",
            path.display()
        )));
    }
    if inventory.stdout.is_empty() {
        return Ok(HistoricalFile::Missing);
    }
    let bytes = crate::ports::git_read::blob(&repo, revision, rel).ok_or_else(|| {
        OkfError::Environment(format!(
            "historical interpretation file unavailable: {} at {revision}",
            path.display()
        ))
    })?;
    Ok(HistoricalFile::Bytes(bytes))
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedQuery {
    pub from: SavedStart,
    #[serde(default)]
    pub steps: Vec<SavedStep>,
    #[serde(default)]
    pub select: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum SavedStart {
    Concept(String),
    List { list: PresetStart },
    Search { search: PresetStart },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PresetStart {
    pub preset: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum SavedStep {
    Links {
        links: crate::graph::filter::EdgeFilters,
    },
    Backlinks {
        backlinks: crate::graph::filter::EdgeFilters,
    },
}
impl SavedStep {
    pub fn filters(&self) -> &crate::graph::filter::EdgeFilters {
        match self {
            Self::Links { links } => links,
            Self::Backlinks { backlinks } => backlinks,
        }
    }
}
pub fn validate_settings(settings: &BundleSettings) -> Result<()> {
    for (name, saved) in &settings.saved_queries {
        if name.trim().is_empty() {
            return Err(OkfError::Usage("saved query needs a name".into()));
        }
        let start = match &saved.from {
            SavedStart::Concept(s) if s == "concept" => None,
            SavedStart::Concept(_) => {
                return Err(OkfError::Usage(format!(
                    "invalid start for saved query {name}"
                )));
            }
            SavedStart::List { list } => Some(("list", list)),
            SavedStart::Search { search } => Some(("search", search)),
        };
        if let Some((op, start)) = start
            && !settings
                .query_presets
                .get(&start.preset)
                .is_some_and(|p| p.operations.iter().any(|x| x == op))
        {
            return Err(OkfError::Usage(format!(
                "saved query {name}: missing or incompatible {op} preset {}",
                start.preset
            )));
        }
        for step in &saved.steps {
            step.filters().compile()?;
        }
        for field in &saved.select {
            crate::query::metadata::validate_projection_field(field)?;
        }
    }
    for (name, preset) in &settings.query_presets {
        if name.trim().is_empty() || preset.operations.is_empty() {
            return Err(OkfError::Usage(
                "query preset requires a name and nonempty operations".into(),
            ));
        }
        preset.filters.compile()?;
        if preset.operations.iter().any(|op| {
            !matches!(
                op.as_str(),
                "links" | "backlinks" | "graph" | "search" | "list"
            )
        }) {
            return Err(OkfError::Usage(format!(
                "query preset {name:?} references an unsupported query operation"
            )));
        }
        if let Some(query) = &preset.query {
            query.validate()?;
            if preset
                .operations
                .iter()
                .any(|op| !matches!(op.as_str(), "search" | "list"))
            {
                return Err(OkfError::Usage(
                    "primary query presets apply only to search/list".into(),
                ));
            }
        }
        if !preset.filters.is_empty()
            && preset
                .operations
                .iter()
                .any(|op| !matches!(op.as_str(), "links" | "backlinks" | "graph"))
        {
            return Err(OkfError::Usage(
                "relationship filters cannot be applied to primary search/list results".into(),
            ));
        }
        if preset.details
            && preset
                .operations
                .iter()
                .any(|op| !matches!(op.as_str(), "links" | "backlinks"))
        {
            return Err(OkfError::Usage(
                "preset details is supported only by links/backlinks".into(),
            ));
        }
    }

    for (key, value) in [
        (
            "ontology_severity",
            &settings.write_policy.ontology_severity,
        ),
        (
            "blocking_threshold",
            &settings.write_policy.blocking_threshold,
        ),
    ] {
        if let Some(value) = value
            && !matches!(value.as_str(), "info" | "warn" | "error")
        {
            return Err(OkfError::Usage(format!(
                "invalid write_policy.{key} {value:?}; expected info, warn, or error"
            )));
        }
    }
    if settings.query.scan_limit == Some(0)
        || settings.lint.finding_budget == Some(0)
        || settings.facets.max_distinct_values == 0
    {
        return Err(OkfError::Usage(
            "configured query, lint and facet limits must be positive".into(),
        ));
    }
    for view in settings.views.values() {
        for field in &view.columns {
            crate::query::metadata::validate_projection_field(field)?;
        }
    }
    for field in settings
        .facets
        .fields
        .iter()
        .chain(&settings.facets.include_high_cardinality)
    {
        crate::query::selector::Selector::parse(field)?;
    }
    for glob in &settings.lint.index_exclude {
        globset::GlobBuilder::new(glob)
            .literal_separator(true)
            .build()
            .map_err(|e| OkfError::Usage(format!("invalid index exclusion {glob:?}: {e}")))?;
    }
    for (rule, severity, limit) in settings.lint.size_rules() {
        if limit == Some(0) || (severity.unwrap_or("off") != "off" && limit.is_none()) {
            return Err(OkfError::Usage(format!(
                "{rule} requires a positive {rule}_limit"
            )));
        }
    }
    for (rule, severity) in [
        ("body_max_lines", &settings.lint.body_max_lines),
        (
            "frontmatter_max_lines",
            &settings.lint.frontmatter_max_lines,
        ),
        ("max_line_length", &settings.lint.max_line_length),
        ("broken_link", &settings.lint.broken_link),
        ("invalid_reference", &settings.lint.invalid_reference),
        ("missing_title", &settings.lint.missing_title),
        ("frontmatter_order", &settings.lint.frontmatter_order),
        ("missing_description", &settings.lint.missing_description),
        ("orphan", &settings.lint.orphan),
        ("ontology_violation", &settings.lint.ontology_violation),
        ("spec_v02", &settings.lint.spec_v02),
        ("source_missing", &settings.lint.source_missing),
        ("source_unrecorded", &settings.lint.source_unrecorded),
        ("index_coverage", &settings.lint.index_coverage),
    ] {
        if let Some(value) = severity
            && !matches!(value.as_str(), "off" | "info" | "warn" | "error")
        {
            return Err(OkfError::Usage(format!(
                "invalid {rule} lint severity {value:?}; expected off, info, warn, or error"
            )));
        }
    }
    Ok(())
}
/// Historical in-repository settings are resolved at the examined revision. External sidecars stay explicit local context.
pub fn for_context_at(
    context: &Context,
    root: &Path,
    revision: &str,
) -> Result<(EffectiveSettings, Option<Ontology>)> {
    if let Some(origin) = super::remote::origin(root)? {
        let mut selected = context.clone();
        let selected_root = if origin.commit.as_deref() == Some(revision) {
            root.to_path_buf()
        } else {
            let snapshot = super::remote::materialize_local(root, revision)?;
            snapshot.root.join(snapshot.bundle_relative)
        };
        if let Some(catalog) = &mut selected.catalog {
            for entry in catalog.entries.values_mut() {
                if entry.root == root {
                    entry.root = selected_root.clone();
                }
            }
        }
        if selected.primary.root == root {
            selected.primary.root = selected_root.clone();
        }
        let effective = for_context(&selected, &selected_root)?;
        let ontology = effective
            .ontology_path
            .as_ref()
            .map(|p| load::load_ontology(p))
            .transpose()?;
        return Ok((effective, ontology));
    }
    let _reads = crate::ports::git_read::ReadSession::new();
    let mut historical_context = context.clone();
    if let Some(path) = &context.config_path {
        match historical(root, path, revision)? {
            HistoricalFile::Bytes(bytes) => {
                let text =
                    String::from_utf8(bytes).map_err(|e| OkfError::Environment(e.to_string()))?;
                historical_context.config = toml::from_str(&text)
                    .map_err(|e| OkfError::Usage(format!("invalid historical config: {e}")))?;
                historical_context
                    .config
                    .validate(path.parent().unwrap_or(root))?;
            }
            HistoricalFile::Missing => {
                return Err(OkfError::Environment(format!(
                    "historical configuration unavailable: {} at {revision}",
                    path.display()
                )));
            }
            HistoricalFile::External => {}
        }
    }
    // Determine paths without consulting working-tree ontology existence or bytes.
    let abs = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    let id = context
        .catalog
        .as_ref()
        .and_then(|c| c.entries.values().find(|e| e.root == abs))
        .map(|e| e.id.clone());
    let table = historical_context
        .config
        .settings
        .get("bundle_settings")
        .and_then(toml::Value::as_table);
    let legacy = table.and_then(|t| {
        if id.is_none() && context.catalog.is_some() {
            None
        } else {
            t.get(id.as_deref().unwrap_or("default"))
        }
    });
    let base = context
        .config_path
        .as_ref()
        .and_then(|p| p.parent())
        .unwrap_or(root);
    let mut merged = legacy
        .map(yaml_serde::to_value)
        .transpose()
        .map_err(|e| OkfError::Usage(e.to_string()))?
        .unwrap_or_else(|| yaml_serde::Value::Mapping(Default::default()));
    if let Some(old) = merged.get_mut("ontology")
        && let Some(path) = old.as_str()
    {
        *old = yaml_serde::Value::String(
            super::catalog::absolute(&base.join(path))?
                .to_string_lossy()
                .into_owned(),
        );
    }
    match historical(root, &root.join(super::manifest::FILENAME), revision)? {
        HistoricalFile::Bytes(bytes) => {
            let text = std::str::from_utf8(&bytes).map_err(|e| OkfError::Usage(e.to_string()))?;
            super::manifest::parse(text)?;
            let raw: yaml_serde::Value =
                yaml_serde::from_str(text).map_err(|e| OkfError::Usage(e.to_string()))?;
            let mut portable = raw
                .get("settings")
                .cloned()
                .unwrap_or_else(|| yaml_serde::Value::Mapping(Default::default()));
            if let Some(value) = portable.get_mut("ontology")
                && let Some(path) = value.as_str()
            {
                *value = yaml_serde::Value::String(
                    super::catalog::absolute(&root.join(path))?
                        .to_string_lossy()
                        .into_owned(),
                );
            }
            merge_settings(&mut merged, portable, "settings")?;
        }
        HistoricalFile::Missing => {}
        HistoricalFile::External => {
            return Err(OkfError::Usage(
                "historical manifest unavailable outside repository".into(),
            ));
        }
    }
    let mut settings: BundleSettings = yaml_serde::from_value(merged)
        .map_err(|e| OkfError::Usage(format!("invalid historical settings: {e}")))?;
    validate_settings(&settings)?;
    let override_origins = apply_overrides(&mut settings)?;
    let configured = settings.ontology.is_some();
    let base = context
        .config_path
        .as_ref()
        .and_then(|p| p.parent())
        .unwrap_or(root);
    let path = settings
        .ontology
        .as_ref()
        .map_or_else(|| load::ontology_path(root), |p| base.join(p));
    let bytes = match historical(root, &path, revision)? {
        HistoricalFile::Bytes(bytes) => Some(bytes),
        HistoricalFile::External => {
            if configured {
                Some(std::fs::read(&path)?)
            } else {
                None
            }
        }
        HistoricalFile::Missing => {
            if configured {
                return Err(OkfError::Environment(format!(
                    "configured historical ontology unavailable: {} at {revision}",
                    path.display()
                )));
            } else {
                None
            }
        }
    };
    let ontology = bytes
        .as_ref()
        .map(|b| {
            std::str::from_utf8(b)
                .map_err(|e| OkfError::Environment(e.to_string()))
                .and_then(load::parse_ontology)
                .map_err(|error| error.at_path(&path))
        })
        .transpose()?;
    Ok((
        EffectiveSettings {
            bundle: id,
            ontology_path: bytes.as_ref().map(|_| path),
            ontology_digest: bytes
                .as_ref()
                .map(|b| crate::fingerprint::canonicalize::sha256_hex(b)),
            settings,
            override_origins,
        },
        ontology,
    ))
}
pub fn load_for_at(root: &Path, version: &str) -> Result<(EffectiveSettings, Option<Ontology>)> {
    let context = current_context(Some(&root.to_string_lossy()), None)?;
    if let Some(revision) = version.strip_prefix("git:") {
        for_context_at(&context, root, revision)
    } else {
        load_for(root)
    }
}

/// Merge only disjoint/equal declarations; conflicting legacy values require migration review.
pub fn merge_settings(
    base: &mut yaml_serde::Value,
    new: yaml_serde::Value,
    path: &str,
) -> Result<()> {
    match (base, new) {
        (yaml_serde::Value::Mapping(old), yaml_serde::Value::Mapping(new)) => {
            for (key, value) in new {
                if let Some(prior) = old.get_mut(&key) {
                    merge_settings(
                        prior,
                        value,
                        &format!("{path}.{}", key.as_str().unwrap_or("?")),
                    )?;
                } else {
                    old.insert(key, value);
                }
            }
        }
        (old, new) if *old == new => {}
        (_, _) => {
            return Err(OkfError::Usage(format!(
                "legacy and manifest settings conflict at {path}"
            )));
        }
    }
    Ok(())
}

/// Presentation policy; Markdown is only applied by explicit formatting commands.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FormattingSettings {
    pub yaml: YamlFormatting,
    pub markdown: MarkdownFormatting,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum YamlStyle {
    #[default]
    Readable,
    Legacy,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct YamlFormatting {
    pub style: YamlStyle,
    /// Target columns; zero disables wrapping.
    pub line_width: usize,
}
impl Default for YamlFormatting {
    fn default() -> Self {
        Self {
            style: YamlStyle::Readable,
            line_width: 100,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MarkdownFormatting {
    pub wrap_prose: bool,
    /// Target columns; zero disables wrapping.
    pub line_width: usize,
}
impl Default for MarkdownFormatting {
    fn default() -> Self {
        Self {
            wrap_prose: false,
            line_width: 100,
        }
    }
}
