//! Explicit, single-bundle change plans evaluated in isolation before publication.
use super::transaction::{self, FileChange};
use crate::error::{OkfError, Result};
use crate::model::concept::ConceptId;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChangeSet {
    pub version: u32,
    pub operations: Vec<Operation>,
}

#[derive(Debug)]
pub enum Operation {
    Create {
        path: String,
        content: String,
    },
    CreateStructured {
        path: String,
        frontmatter: yaml_serde::Mapping,
        body: String,
    },
    Replace {
        path: String,
        content: String,
    },
    Edit {
        concept: String,
        patch: Vec<super::structured::PatchOperation>,
        body: Option<String>,
    },
    Move {
        from: String,
        to: String,
    },
    Retarget {
        from: String,
        to: String,
        within: Option<Vec<String>>,
    },
    Remove {
        path: String,
    },
    RemoveAllowDangling {
        path: String,
    },
    PutArtifact {
        path: String,
        content: String,
        replace: bool,
    },
}

#[derive(Deserialize)]
#[serde(tag = "op", rename_all = "kebab-case", deny_unknown_fields)]
enum OperationInput {
    Create {
        path: String,
        content: Option<String>,
        frontmatter: Option<yaml_serde::Mapping>,
        body: Option<String>,
    },
    Replace {
        path: String,
        content: String,
    },
    Edit {
        concept: String,
        #[serde(default)]
        patch: Vec<super::structured::PatchOperation>,
        body: Option<String>,
    },
    Move {
        from: String,
        to: String,
    },
    Retarget {
        from: String,
        to: String,
        within: Option<Vec<String>>,
    },
    Remove {
        path: String,
        #[serde(default)]
        allow_dangling: bool,
    },
    PutArtifact {
        path: String,
        content: String,
        #[serde(default)]
        replace: bool,
    },
}

impl<'de> Deserialize<'de> for Operation {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        use serde::de::Error;
        Ok(match OperationInput::deserialize(deserializer)? {
            OperationInput::Create {
                path,
                content: Some(content),
                frontmatter: None,
                body: None,
            } => Self::Create { path, content },
            OperationInput::Create {
                path,
                content: None,
                frontmatter: Some(frontmatter),
                body,
            } => Self::CreateStructured {
                path,
                frontmatter,
                body: body.unwrap_or_default(),
            },
            OperationInput::Create { .. } => {
                return Err(D::Error::custom(
                    "create requires either content or frontmatter and optional body",
                ));
            }
            OperationInput::Replace { path, content } => Self::Replace { path, content },
            OperationInput::Edit {
                concept,
                patch,
                body,
            } => Self::Edit {
                concept,
                patch,
                body,
            },
            OperationInput::Move { from, to } => Self::Move { from, to },
            OperationInput::Retarget { from, to, within } => Self::Retarget { from, to, within },
            OperationInput::Remove {
                path,
                allow_dangling: true,
            } => Self::RemoveAllowDangling { path },
            OperationInput::Remove {
                path,
                allow_dangling: false,
            } => Self::Remove { path },
            OperationInput::PutArtifact {
                path,
                content,
                replace,
            } => Self::PutArtifact {
                path,
                content,
                replace,
            },
        })
    }
}

pub struct Plan {
    pub base_digest: String,
    pub changes: Vec<FileChange>,
    pub allowed_dangling: Vec<String>,
    before: BTreeMap<PathBuf, Vec<u8>>,
    interpretation: String,
    scope: Option<ScopeBaseline>,
    require_gate: bool,
}

/// Additional registered bundles are read dependencies, never write destinations.
#[derive(Clone)]
struct ScopeRequest {
    bundles: Vec<String>,
    catalog_scope: bool,
}
struct ScopeBaseline {
    request: ScopeRequest,
    signature: String,
    examined: Vec<String>,
}
impl Plan {
    /// Registered bundles whose inputs are bound to this scoped plan.
    pub fn examined_bundles(&self) -> Vec<String> {
        self.scope
            .as_ref()
            .map(|s| s.examined.clone())
            .unwrap_or_default()
    }
}
struct ScopeCapture {
    gate: super::gate::Scope,
    files: BTreeMap<String, BTreeSet<PathBuf>>,
    signature: String,
    roots: Vec<PathBuf>,
}
fn incomplete(message: impl Into<String>) -> OkfError {
    OkfError::classified(crate::error::FailureReason::WriteGateIncomplete, message)
}
fn conflict(message: impl Into<String>) -> OkfError {
    OkfError::classified(crate::error::FailureReason::BaselineConflict, message)
}
fn scope_context(
    root: &Path,
    request: &ScopeRequest,
) -> Result<(crate::bundle::context::Context, Vec<String>)> {
    let context = crate::bundle::context::current_context(Some(&root.to_string_lossy()), None)?;
    let catalog = context
        .catalog
        .as_ref()
        .ok_or_else(|| incomplete("scoped changeset requires an available catalog"))?;
    if context.primary.id.is_none() {
        return Err(incomplete(
            "scoped changeset requires a registered primary bundle",
        ));
    }
    let ids = if request.catalog_scope {
        catalog.entries.keys().cloned().collect::<BTreeSet<_>>()
    } else {
        request.bundles.iter().cloned().collect()
    };
    for id in &ids {
        let entry = catalog
            .entries
            .get(id)
            .ok_or_else(|| incomplete(format!("unknown scoped bundle: {id}")))?;
        if !entry.available {
            return Err(incomplete(format!("scoped bundle unavailable: {id}")));
        }
    }
    Ok((context, ids.into_iter().collect()))
}
fn scope_roots(root: &Path, request: &ScopeRequest) -> Result<Vec<PathBuf>> {
    let (context, ids) = scope_context(root, request)?;
    let catalog = context.catalog.as_ref().unwrap();
    Ok(ids
        .iter()
        .filter(|id| Some(id.as_str()) != context.primary.id.as_deref())
        .map(|id| catalog.entries[id].root.clone())
        .collect())
}
fn capture_scope(root: &Path, request: &ScopeRequest) -> Result<ScopeCapture> {
    let (context, ids) = scope_context(root, request)?;
    let catalog = context.catalog.as_ref().unwrap();
    // Catalog bytes plus resolved entries cover registration and override changes.
    // Effective settings below cover external sidecar contents and configuration.
    let entries: Vec<_> = catalog.entries.values().map(|e| (e, &e.settings)).collect();
    let mut evidence = vec![
        serde_json::to_string(&(
            &catalog.path,
            std::fs::read(&catalog.path)?,
            entries,
            context
                .config_path
                .as_ref()
                .map(std::fs::read)
                .transpose()?,
            &context.config_path,
            &ids,
            request.catalog_scope,
        ))
        .map_err(|e| OkfError::Internal(e.to_string()))?,
    ];
    let mut targets = BTreeMap::new();
    let mut target_files = BTreeMap::new();
    let mut roots = Vec::new();
    for id in ids {
        if Some(id.as_str()) == context.primary.id.as_deref() {
            continue;
        }
        let target = &catalog.entries[&id].root;
        let files =
            snapshot(target).map_err(|e| incomplete(format!("cannot examine {id}: {e}")))?;
        let (settings, ontology) = crate::bundle::settings::load_for(target)
            .map_err(|e| incomplete(format!("cannot interpret {id}: {e}")))?;
        let interpretation =
            serde_json::to_string(&settings).map_err(|e| OkfError::Internal(e.to_string()))?;
        evidence.push(
            serde_json::to_string(&(&id, target, digest(&files, &interpretation)))
                .map_err(|e| OkfError::Internal(e.to_string()))?,
        );
        let bundle = snapshot_bundle(target, &files, None)
            .map_err(|e| incomplete(format!("cannot examine {id}: {e}")))?;
        roots.push(target.clone());
        target_files.insert(id.clone(), files.into_keys().collect());
        targets.insert(id, (bundle, ontology));
    }
    Ok(ScopeCapture {
        gate: super::gate::Scope {
            primary_id: context.primary.id.unwrap(),
            targets,
        },
        files: target_files,
        signature: serde_json::to_string(&evidence)
            .map_err(|e| OkfError::Internal(e.to_string()))?,
        roots,
    })
}
fn scoped_interpretation(interpretation: &str, scope: Option<&str>) -> String {
    scope.map_or_else(
        || interpretation.to_owned(),
        |scope| serde_json::to_string(&(interpretation, scope)).expect("strings serialize"),
    )
}

fn usage(s: impl Into<String>) -> OkfError {
    OkfError::Usage(s.into())
}

fn snapshot(root: &Path) -> Result<BTreeMap<PathBuf, Vec<u8>>> {
    let mut out = BTreeMap::new();
    for path in crate::bundle::walk::walk_files(root)? {
        if path == Path::new(transaction::LOCK) {
            continue;
        }
        // Preserve hidden authored files in the isolated view too. The walker excludes
        // symlinks and Git metadata; publication still forbids editing control paths.
        out.insert(path.clone(), std::fs::read(root.join(&path))?);
    }
    Ok(out)
}

/// Reuse parsed documents only when their captured bytes are exactly unchanged.
/// The cache lives for one preparation and never substitutes for the apply baseline.
fn snapshot_bundle(
    root: &Path,
    files: &BTreeMap<PathBuf, Vec<u8>>,
    previous: Option<(&BTreeMap<PathBuf, Vec<u8>>, crate::bundle::loader::Bundle)>,
) -> Result<crate::bundle::loader::Bundle> {
    let mut concepts = Vec::new();
    let mut failures = Vec::new();
    let (old_files, mut old_concepts) = match previous {
        Some((files, bundle)) => (
            Some(files),
            bundle
                .concepts
                .into_iter()
                .map(|concept| (concept.id.0.clone(), concept))
                .collect::<BTreeMap<_, _>>(),
        ),
        None => (None, BTreeMap::new()),
    };
    for (path, bytes) in files {
        if crate::bundle::walk::is_support_path(path)
            || path.extension().and_then(|extension| extension.to_str()) != Some("md")
            || matches!(
                path.file_name().and_then(|name| name.to_str()),
                Some("index.md" | "log.md")
            )
        {
            continue;
        }
        let relative = path.to_string_lossy();
        let Some(stem) = relative.strip_suffix(".md") else {
            continue;
        };
        let id = ConceptId::from_relative(stem);
        if old_files.is_some_and(|files| files.get(path) == Some(bytes))
            && let Some(concept) = old_concepts.remove(&id.0)
        {
            concepts.push(concept);
            continue;
        }
        let text = std::str::from_utf8(bytes).map_err(|e| {
            OkfError::io(format!(
                "invalid UTF-8 in {}: {e}",
                root.join(path).display()
            ))
        })?;
        match crate::parse::parse_concept_file(id, text, &root.join(path)) {
            Ok(concept) => concepts.push(concept),
            Err(OkfError::Yaml(message)) => failures.push(message),
            Err(error) => return Err(error),
        }
    }
    if !failures.is_empty() {
        return Err(OkfError::yaml_files(failures));
    }
    concepts.sort_by(|a, b| a.id.0.cmp(&b.id.0));
    Ok(crate::bundle::loader::Bundle {
        root: root.to_path_buf(),
        concepts,
    })
}

fn digest(files: &BTreeMap<PathBuf, Vec<u8>>, interpretation: &str) -> String {
    let mut hash = Sha256::new();
    hash.update(interpretation.as_bytes());
    for (path, content) in files {
        let path = path.to_string_lossy();
        hash.update((path.len() as u64).to_le_bytes());
        hash.update(path.as_bytes());
        hash.update((content.len() as u64).to_le_bytes());
        hash.update(content);
    }
    hash.finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

struct Workspace(PathBuf);
impl Workspace {
    fn new() -> Result<Self> {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        loop {
            let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!("okf-plan-{}-{n}", std::process::id()));
            match std::fs::create_dir(&path) {
                Ok(()) => return Ok(Self(path)),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(e.into()),
            }
        }
    }
}
impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn concept_path(path: &str) -> Result<(ConceptId, PathBuf)> {
    let id = ConceptId::parse(path)?;
    let relative = PathBuf::from(format!("{}.md", id.0.trim_start_matches('/')));
    if matches!(
        relative.file_name().and_then(|s| s.to_str()),
        Some("index.md" | "log.md")
    ) {
        return Err(usage(
            "create/replace require a concept; use put-artifact for structural files",
        ));
    }
    Ok((id, relative))
}

fn write_document(root: &Path, path: &str, content: &str, replace: bool) -> Result<()> {
    let (id, rel) = concept_path(path)?;
    let target = transaction::checked_path(root, &rel)?;
    if target.exists() != replace {
        return Err(usage(format!(
            "{} requires {}: {path}",
            if replace { "replace" } else { "create" },
            if replace {
                "an existing concept"
            } else {
                "an absent concept"
            }
        )));
    }
    let concept = crate::parse::parse_concept(id.clone(), content)?;
    if concept.frontmatter.get("verified").is_some() {
        return Err(usage(
            "change-set content cannot supply verification; use verify after review",
        ));
    }
    if replace {
        let old = super::edit::load_concept(root, &id)?;
        let mut spec = super::edit::EditSpec {
            unsets: old
                .frontmatter
                .map
                .keys()
                .filter(|k| !concept.frontmatter.map.contains_key(*k) && k.as_str() != "verified")
                .cloned()
                .collect(),
            ..Default::default()
        };
        spec.structured.sets = concept.frontmatter.map.into_iter().collect();
        spec.set_body = Some(concept.body);
        super::edit::edit(root, &id.0, &spec)?;
    } else {
        super::edit::save_new_concept(root, &concept)?;
    }
    Ok(())
}

fn write_structured_document(
    root: &Path,
    path: &str,
    frontmatter: &yaml_serde::Mapping,
    body: &str,
) -> Result<()> {
    let (id, rel) = concept_path(path)?;
    let target = transaction::checked_path(root, &rel)?;
    if target.exists() {
        return Err(usage(format!("create requires an absent concept: {path}")));
    }
    let mut map = indexmap::IndexMap::new();
    for (key, value) in frontmatter {
        let Some(key) = key.as_str() else {
            return Err(usage("frontmatter keys must be strings"));
        };
        if key == "verified" {
            return Err(usage(
                "change-set content cannot supply verification; use verify after review",
            ));
        }
        map.insert(key.to_owned(), value.clone());
    }
    let concept = crate::model::concept::Concept {
        id,
        frontmatter: crate::model::frontmatter::Frontmatter::from_map(map),
        body: body.to_owned(),
    };
    super::edit::save_new_concept(root, &concept)?;
    Ok(())
}

/// Check registered siblings without publishing there or reading unregistered targets.
/// Lexical containment is checked before the sibling resolver checks canonical containment.
fn registered_reference(
    context: &crate::bundle::context::Context,
    from: &str,
    raw: &str,
    bundle_relative: bool,
) -> bool {
    let Some(catalog) = &context.catalog else {
        return false;
    };
    let root = &context.primary.root;
    if !catalog
        .entries
        .values()
        .any(|entry| entry.available && entry.root == *root)
    {
        return false;
    }
    let clean = raw
        .trim()
        .split(['#', '?'])
        .next()
        .unwrap_or(raw)
        .replace('\\', "/");
    let base = if bundle_relative || clean.starts_with('/') {
        root.clone()
    } else {
        root.join(from.trim_start_matches('/'))
            .parent()
            .map_or_else(|| root.clone(), Path::to_path_buf)
    };
    let mut target = crate::bundle::catalog::normalize(&base.join(clean.trim_start_matches('/')));
    if !bundle_relative && target.extension().is_none() {
        target.set_extension("md");
    }
    let Some(entry) = catalog
        .entries
        .values()
        .find(|entry| entry.available && entry.root != *root && target.starts_with(&entry.root))
    else {
        return false;
    };
    let Ok(relative) = target.strip_prefix(&entry.root) else {
        return false;
    };
    let resolved = crate::query::artifact::ArtifactResolver::new(&entry.root)
        .resolve(None, &relative.to_string_lossy());
    resolved.exists && resolved.kind != crate::query::artifact::ArtifactKind::Blocked
}

// Explain only newly introduced findings; the comparison keys stay stable so an
// unchanged pre-existing finding does not become a new violation.
fn explain_missing(
    finding: &str,
    context: &crate::bundle::context::Context,
    scope: Option<&ScopeCapture>,
) -> String {
    let Some((_, raw)) = finding.split_once(": ") else {
        return finding.into();
    };
    if !crate::model::link::is_logical_reference(raw) {
        return finding.into();
    }
    let reason = match crate::model::link::LogicalReference::parse(raw) {
        Err(_) => "invalid logical reference",
        Ok(Some(reference)) => {
            let entry = context
                .catalog
                .as_ref()
                .and_then(|catalog| catalog.entries.get(&reference.bundle));
            if entry.is_some_and(|entry| !entry.available) {
                "unavailable bundle"
            } else if !scope.is_some_and(|scope| {
                reference.bundle == scope.gate.primary_id
                    || scope.gate.targets.contains_key(&reference.bundle)
            }) {
                "out-of-scope bundle"
            } else {
                "missing or blocked target"
            }
        }
        Ok(None) => return finding.into(),
    };
    format!("{finding} ({reason})")
}

/// Missing local references include authored concept links and structural navigation.
fn missing(
    root: &Path,
    files: &BTreeMap<PathBuf, Vec<u8>>,
    bundle: &crate::bundle::loader::Bundle,
    ontology: Option<&crate::ontology::schema::Ontology>,
    context: &crate::bundle::context::Context,
    scope: Option<&ScopeCapture>,
) -> Result<BTreeSet<String>> {
    let resolver = crate::query::artifact::ArtifactResolver::new(root);
    let mut out = BTreeSet::new();
    let mut check = |id: &str, raw: &str, bundle_relative: bool| {
        if crate::model::link::is_logical_reference(raw) {
            let found = crate::model::link::LogicalReference::parse(raw)
                .ok()
                .flatten()
                .and_then(|reference| {
                    let scope = scope?;
                    let (target_bundle, target_files) = if reference.bundle == scope.gate.primary_id
                    {
                        (bundle, None)
                    } else {
                        (
                            &scope.gate.targets.get(&reference.bundle)?.0,
                            Some(scope.files.get(&reference.bundle)?),
                        )
                    };
                    let exists = |path: &Path| match target_files {
                        Some(target_files) => target_files.contains(path),
                        None => files.contains_key(path),
                    };
                    let path = Path::new(&reference.path);
                    if exists(path) {
                        return Some(true);
                    }
                    // Concept links may omit .md, while artifact references are literal.
                    if !bundle_relative && path.extension().is_none() {
                        let id = ConceptId::from_relative(&reference.path);
                        let document = PathBuf::from(format!("{}.md", reference.path));
                        return Some(target_bundle.get(&id.0).is_some() && exists(&document));
                    }
                    Some(false)
                })
                .unwrap_or(false);
            if !found {
                out.insert(format!("{id}: {raw}"));
            }
            return;
        }
        let from = if bundle_relative {
            "/_bundle_sources"
        } else {
            id
        };
        let resolved = resolver.resolve(Some(from), raw);
        if resolved.kind == crate::query::artifact::ArtifactKind::Blocked
            && resolved.message.as_deref() == Some("usage error: artifact path escapes bundle")
            && registered_reference(context, id, raw, bundle_relative)
        {
            return;
        }
        if !bundle_relative
            && (resolved.kind == crate::query::artifact::ArtifactKind::Missing
                || (resolved.kind == crate::query::artifact::ArtifactKind::Blocked
                    && resolved.message.as_deref() == Some("resolved resource is not a file")))
        {
            let target = crate::model::link::resolve_link(&ConceptId::from_relative(id), raw);
            if bundle.get(&target.0).is_some() {
                // A sibling directory must not shadow a known suffix-free concept ID.
                // Re-resolve the actual Markdown path to retain containment checks.
                let document = resolver.resolve(None, &format!("{}.md", target.0));
                if document.kind == crate::query::artifact::ArtifactKind::Concept {
                    return;
                }
            }
        }
        if matches!(
            resolved.kind,
            crate::query::artifact::ArtifactKind::Missing
                | crate::query::artifact::ArtifactKind::Blocked
        ) {
            out.insert(format!("{id}: {raw}"));
        }
    };
    for concept in &bundle.concepts {
        // Typed source resources do not share Markdown's document-relative namespace.
        let mut links = concept.clone();
        links.frontmatter.map.shift_remove("sources");
        for raw in crate::model::link::raw_links(&links, ontology) {
            check(&concept.id.0, &raw, false);
        }
        if let Some(definition) = concept
            .concept_type()
            .and_then(|name| ontology.and_then(|o| o.concepts.get(name)))
        {
            fn strings<'a>(value: &'a yaml_serde::Value, out: &mut Vec<&'a str>) {
                match value {
                    yaml_serde::Value::String(s) if !s.trim().is_empty() => out.push(s),
                    yaml_serde::Value::Sequence(items) => {
                        for item in items {
                            strings(item, out);
                        }
                    }
                    _ => {}
                }
            }
            for (key, rule) in &definition.references {
                if rule.selector.is_none()
                    && let Some(value) = concept.frontmatter.get(key)
                {
                    let mut values = Vec::new();
                    strings(value, &mut values);
                    for raw in values {
                        check(&concept.id.0, raw, false);
                    }
                }
            }
        }
        for event in pulldown_cmark::Parser::new(&concept.body) {
            if let pulldown_cmark::Event::Start(pulldown_cmark::Tag::Image { dest_url, .. }) = event
            {
                check(&concept.id.0, &dest_url, false);
            }
        }
        if let Some(sources) = concept
            .frontmatter
            .get("sources")
            .and_then(yaml_serde::Value::as_sequence)
        {
            for source in sources {
                let Some(raw) = source.get("resource").and_then(yaml_serde::Value::as_str) else {
                    continue;
                };
                match source.get("kind").and_then(yaml_serde::Value::as_str) {
                    Some("file" | "line-range" | "markdown-heading") => {
                        check(&concept.id.0, raw, true);
                    }
                    None => check(&concept.id.0, raw, false),
                    _ => {} // URL, Git, and producer-specific resources need other resolvers.
                }
            }
        }
        if let Some(raw) = concept
            .frontmatter
            .get("computation")
            .and_then(yaml_serde::Value::as_str)
        {
            check(&concept.id.0, raw, false);
        }
        for key in ["executor", "attester"] {
            if let Some(raw) = concept
                .frontmatter
                .get(key)
                .and_then(|v| v.get("resource"))
                .and_then(yaml_serde::Value::as_str)
            {
                check(&concept.id.0, raw, false);
            }
        }
    }
    for (path, bytes) in files {
        if crate::bundle::walk::is_support_path(path)
            || !matches!(
                path.file_name().and_then(|s| s.to_str()),
                Some("index.md" | "log.md")
            )
        {
            continue;
        }
        let text = std::str::from_utf8(bytes)
            .map_err(|e| OkfError::io(format!("{}: {e}", root.join(path).display())))?;
        let id = format!("/{}", path.to_string_lossy().trim_end_matches(".md"));
        for event in pulldown_cmark::Parser::new(text) {
            if let pulldown_cmark::Event::Start(
                pulldown_cmark::Tag::Link { dest_url, .. }
                | pulldown_cmark::Tag::Image { dest_url, .. },
            ) = event
            {
                check(&id, &dest_url, false);
            }
        }
    }
    Ok(out)
}

/// Active interpretation and selection files may live inside a bundle, but remain
/// tool configuration rather than artifacts available to coordinated content edits.
fn control_paths(root: &Path, ontology: Option<&Path>) -> Result<BTreeSet<PathBuf>> {
    let context = crate::bundle::context::current_context(Some(&root.to_string_lossy()), None)?;
    let mut paths = Vec::new();
    if let Some(config_path) = &context.config_path {
        paths.push(config_path.clone());
        let base = config_path.parent().unwrap_or(root);
        if let Some(catalog) = &context.config.catalog {
            paths.push(base.join(catalog));
        }
        let overrides = std::env::var("OKF_CATALOG_OVERRIDES")
            .ok()
            .filter(|path| !path.is_empty())
            .map(PathBuf::from)
            .or_else(|| {
                context
                    .config
                    .catalog_overrides
                    .as_ref()
                    .map(|path| base.join(path))
            });
        if let Some(overrides) = overrides {
            paths.push(overrides);
        }
    }
    if let Some(ontology) = ontology {
        paths.push(ontology.to_path_buf());
    }
    let canonical_root = root.canonicalize()?;
    let mut relative = BTreeSet::new();
    for path in paths {
        let absolute = crate::bundle::catalog::absolute(&path)?;
        let canonical = absolute.canonicalize().unwrap_or(absolute);
        if let Ok(path) = canonical.strip_prefix(&canonical_root) {
            relative.insert(path.to_path_buf());
        }
    }
    Ok(relative)
}

/// Preview and apply share this exact preparation. No bundle files are changed here.
pub fn prepare(root: &Path, set: &ChangeSet) -> Result<Plan> {
    prepare_scoped(root, set, &[], false)
}

pub fn prepare_scoped(
    root: &Path,
    set: &ChangeSet,
    scope_bundle: &[String],
    catalog_scope: bool,
) -> Result<Plan> {
    let _guard = transaction::writer_lock(root)?;
    let request = (!scope_bundle.is_empty() || catalog_scope).then(|| ScopeRequest {
        bundles: scope_bundle.to_vec(),
        catalog_scope,
    });
    let roots = request
        .as_ref()
        .map(|r| scope_roots(root, r))
        .transpose()?
        .unwrap_or_default();
    let _readers = transaction::reader_locks(&roots)
        .map_err(|e| incomplete(format!("cannot lock scoped examination: {e}")))?;
    let scope = request
        .as_ref()
        .map(|r| capture_scope(root, r))
        .transpose()?;
    if scope.as_ref().is_some_and(|s| s.roots != roots) {
        return Err(conflict(
            "scope mapping changed while acquiring read barriers; preview again",
        ));
    }
    let require_gate = transaction::ontology_gate_strengthened();
    if set.version != 1 || set.operations.is_empty() {
        return Err(usage(
            "change set requires version: 1 and nonempty operations",
        ));
    }
    let (settings, ontology) = crate::bundle::settings::load_for(root)?;
    let interpretation =
        serde_json::to_string(&settings).map_err(|e| OkfError::Internal(e.to_string()))?;
    let controls = control_paths(root, settings.ontology_path.as_deref())?;
    let before = snapshot(root)?;
    let base_digest = digest(
        &before,
        &scoped_interpretation(
            &interpretation,
            scope.as_ref().map(|s| s.signature.as_str()),
        ),
    );
    let workspace = Workspace::new()?;
    let _staging_evidence = transaction::suppress_publication_evidence();
    let _selection_staging = crate::bundle::selection::suppress_for_staging();
    let stage = &workspace.0;
    let mut directories = BTreeSet::new();
    for (path, content) in &before {
        let target = stage.join(path);
        if directories.insert(target.parent().unwrap().to_path_buf()) {
            std::fs::create_dir_all(target.parent().unwrap())?;
        }
        std::fs::write(target, content)?;
    }
    let context = crate::bundle::context::current_context(Some(&root.to_string_lossy()), None)?;
    let before_bundle = snapshot_bundle(stage, &before, None)?;
    let previous_missing = missing(
        stage,
        &before,
        &before_bundle,
        ontology.as_ref(),
        &context,
        scope.as_ref(),
    )?;
    let mut allowed_removed = BTreeSet::new();
    let stage_gate = transaction::suppress_ontology_gate();
    for op in &set.operations {
        match op {
            Operation::Create { path, content } => write_document(stage, path, content, false)?,
            Operation::CreateStructured {
                path,
                frontmatter,
                body,
            } => write_structured_document(stage, path, frontmatter, body)?,
            Operation::Replace { path, content } => write_document(stage, path, content, true)?,
            Operation::Edit {
                concept,
                patch,
                body,
            } => {
                let mut spec = super::edit::EditSpec::default();
                spec.structured.patch = patch.clone();
                spec.set_body = body.clone();
                super::edit::edit(stage, concept, &spec)?;
            }
            Operation::Move { from, to } => {
                super::mv::mv_with_ontology(stage, from, to, ontology.as_ref())?;
            }
            Operation::Retarget { from, to, within } => {
                let ids = within
                    .as_ref()
                    .map(|ids| {
                        ids.iter()
                            .map(|id| {
                                let relative = id
                                    .trim_start_matches('/')
                                    .strip_suffix(".md")
                                    .unwrap_or(id.trim_start_matches('/'));
                                let path = PathBuf::from(format!("{relative}.md"));
                                let target = transaction::checked_path(stage, &path)?;
                                if !target.is_file() {
                                    return Err(usage(format!(
                                        "retarget referrer does not exist: {id}"
                                    )));
                                }
                                Ok(ConceptId::from_relative(relative))
                            })
                            .collect::<Result<Vec<_>>>()
                    })
                    .transpose()?;
                super::mv::retarget(stage, from, to, ontology.as_ref(), ids.as_deref())?;
            }
            Operation::Remove { path } => {
                if matches!(
                    Path::new(path).file_name().and_then(|s| s.to_str()),
                    Some("okf.toml" | "ontology.yaml")
                ) {
                    return Err(usage("change sets cannot remove tool configuration"));
                }
                std::fs::remove_file(transaction::checked_path(stage, Path::new(path))?)?;
            }
            Operation::RemoveAllowDangling { path } => {
                let (id, relative) = concept_path(path)?;
                allowed_removed.insert(id.0);
                std::fs::remove_file(transaction::checked_path(stage, &relative)?)?;
            }
            Operation::PutArtifact {
                path,
                content,
                replace,
            } => {
                crate::query::artifact::put_artifact(stage, path, content.as_bytes(), *replace)?;
            }
        }
    }
    drop(stage_gate);
    drop(_selection_staging);
    let after = snapshot(stage)?;
    // Coordinated writes use the bundle tooling profile, like concept discovery.
    // Root support Markdown is not a concept. Keep it in the captured snapshot
    // and reviewed baseline, but exclude it from concept conformance checks.
    // Validate captured bytes so conformance and publication share one image.
    let report = crate::check::validate::ValidateReport {
        violations: after
            .iter()
            .filter(|(path, _)| path.extension().and_then(|ext| ext.to_str()) == Some("md"))
            .filter(|(path, _)| !crate::bundle::walk::is_support_path(path))
            .flat_map(|(path, bytes)| {
                crate::check::validate::validate_document_bytes(&path.to_string_lossy(), bytes)
            })
            .collect(),
    };
    if !report.is_conformant() {
        return Err(usage(format!(
            "change set leaves nonconformant documents: {}",
            report
                .violations
                .iter()
                .map(|v| format!("{}: {}", v.file, v.message))
                .collect::<Vec<_>>()
                .join("; ")
        )));
    }
    let after_bundle = snapshot_bundle(stage, &after, Some((&before, before_bundle)))?;
    let after_missing = missing(
        stage,
        &after,
        &after_bundle,
        ontology.as_ref(),
        &context,
        scope.as_ref(),
    )?;
    let mut allowed_dangling = Vec::new();
    let introduced = after_missing
        .difference(&previous_missing)
        .filter(|finding| {
            let Some((from, raw)) = finding.split_once(": ") else {
                return true;
            };
            let from = ConceptId::from_relative(from);
            if allowed_removed.contains(&crate::model::link::resolve_link(&from, raw).0) {
                allowed_dangling.push((*finding).clone());
                false
            } else {
                true
            }
        })
        .cloned()
        .collect::<Vec<_>>();
    if !introduced.is_empty() {
        return Err(usage(format!(
            "change set introduces dangling/blocked references: {}",
            introduced
                .iter()
                .map(|finding| explain_missing(finding, &context, scope.as_ref()))
                .collect::<Vec<_>>()
                .join("; ")
        )));
    }
    for path in controls {
        if before.get(&path) != after.get(&path) {
            return Err(usage(format!(
                "change sets cannot modify active tool configuration: {}",
                path.display()
            )));
        }
    }
    let paths: BTreeSet<_> = before.keys().chain(after.keys()).cloned().collect();
    let changes: Vec<_> = paths
        .into_iter()
        .filter_map(|path| {
            let old = before.get(&path);
            let new = after.get(&path);
            (old != new).then(|| FileChange {
                path,
                before: old.cloned(),
                after: new.cloned(),
            })
        })
        .collect();
    transaction::preflight(root, &changes)?;
    if let Some(scope) = &scope {
        super::gate::check_changes_scoped(root, &changes, require_gate, &scope.gate)?;
        if capture_scope(root, request.as_ref().unwrap())?.signature != scope.signature {
            return Err(conflict(
                "scoped examination inputs changed during planning; preview again",
            ));
        }
    } else {
        super::gate::check_changes(root, &changes, require_gate)?;
    }
    Ok(Plan {
        base_digest,
        changes,
        allowed_dangling,
        before,
        interpretation,
        scope: scope.map(|scope| ScopeBaseline {
            request: request.unwrap(),
            signature: scope.signature,
            examined: std::iter::once(scope.gate.primary_id)
                .chain(scope.gate.targets.into_keys())
                .collect(),
        }),
        require_gate,
    })
}

pub fn apply(root: &Path, plan: Plan, expected: Option<&str>) -> Result<()> {
    let guard = transaction::writer_lock(root)?;
    let _gate = plan
        .require_gate
        .then(transaction::strengthen_ontology_gate);
    let roots = plan
        .scope
        .as_ref()
        .map(|s| scope_roots(root, &s.request))
        .transpose()
        .map_err(|e| {
            conflict(format!(
                "scoped examination unavailable since planning: {e}"
            ))
        })?
        .unwrap_or_default();
    let _readers = transaction::reader_locks(&roots)
        .map_err(|e| conflict(format!("scoped examination cannot be guarded: {e}")))?;
    if expected.is_some_and(|value| value != plan.base_digest) {
        return Err(OkfError::classified(
            crate::error::FailureReason::BaselineConflict,
            "change-set base digest differs from --expect; preview again",
        ));
    }
    let interpretation = serde_json::to_string(&crate::bundle::settings::load_for(root)?.0)
        .map_err(|e| OkfError::Internal(e.to_string()))?;
    if snapshot(root)? != plan.before || interpretation != plan.interpretation {
        return Err(OkfError::classified(
            crate::error::FailureReason::BaselineConflict,
            "bundle or interpretation changed during planning; preview again",
        ));
    }
    if let Some(baseline) = &plan.scope {
        let scope = capture_scope(root, &baseline.request).map_err(|e| {
            conflict(format!(
                "scoped examination unavailable since planning: {e}"
            ))
        })?;
        if scope.roots != roots || scope.signature != baseline.signature {
            return Err(conflict(
                "scoped examination inputs changed since planning; preview again",
            ));
        }
        super::gate::check_changes_scoped(
            root,
            &plan.changes,
            transaction::ontology_gate_strengthened(),
            &scope.gate,
        )?;
        if capture_scope(root, &baseline.request)
            .map_err(|e| {
                conflict(format!(
                    "scoped examination changed before publication: {e}"
                ))
            })?
            .signature
            != baseline.signature
        {
            return Err(conflict(
                "scoped examination inputs changed before publication; preview again",
            ));
        }
        // The scoped check above replaces the single-bundle check; read barriers stay
        // alive until journaled publication completes.
        let _checked = transaction::suppress_ontology_gate();
        transaction::commit_locked(root, &guard, plan.changes)
    } else {
        transaction::commit_locked(root, &guard, plan.changes)
    }
}

/// Acquire the same covered baseline used by changeset plan and apply, without staging edits.
pub fn digest_read_only(root: &Path) -> Result<String> {
    digest_read_only_scoped(root, &[], false)
}
pub fn digest_read_only_scoped(
    root: &Path,
    scope_bundle: &[String],
    catalog_scope: bool,
) -> Result<String> {
    Ok(digest_read_only_scoped_with_evidence(root, scope_bundle, catalog_scope)?.0)
}

/// Return the digest and registered examination IDs from the same guarded capture.
pub fn digest_read_only_scoped_with_evidence(
    root: &Path,
    scope_bundle: &[String],
    catalog_scope: bool,
) -> Result<(String, Vec<String>)> {
    let _guard = transaction::writer_lock(root)?;
    let interpretation = serde_json::to_string(&crate::bundle::settings::load_for(root)?.0)
        .map_err(|e| OkfError::Internal(e.to_string()))?;
    let request = (!scope_bundle.is_empty() || catalog_scope).then(|| ScopeRequest {
        bundles: scope_bundle.to_vec(),
        catalog_scope,
    });
    let roots = request
        .as_ref()
        .map(|r| scope_roots(root, r))
        .transpose()?
        .unwrap_or_default();
    let _readers = transaction::reader_locks(&roots)
        .map_err(|e| incomplete(format!("cannot lock scoped examination: {e}")))?;
    let scope = request
        .as_ref()
        .map(|r| capture_scope(root, r))
        .transpose()?;
    if scope.as_ref().is_some_and(|s| s.roots != roots) {
        return Err(conflict(
            "scope mapping changed while acquiring read barriers; retry",
        ));
    }
    let baseline = digest(
        &snapshot(root)?,
        &scoped_interpretation(
            &interpretation,
            scope.as_ref().map(|s| s.signature.as_str()),
        ),
    );
    let examined = scope
        .map(|scope| {
            std::iter::once(scope.gate.primary_id)
                .chain(scope.gate.targets.into_keys())
                .collect()
        })
        .unwrap_or_default();
    Ok((baseline, examined))
}
