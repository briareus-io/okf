//! Portable bundle metadata and deterministic agent-context delivery.
use super::settings::{BundleSettings, EffectiveSettings};
use crate::{
    error::{OkfError, Result},
    fingerprint::canonicalize::sha256_hex,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Component, Path, PathBuf},
};

thread_local! {static COMMANDS:std::cell::RefCell<Option<Vec<String>>>=const {std::cell::RefCell::new(None)};}
/// Installs the executable's canonical command tree for schema validation across core services.
/// No parallel registry of command names is maintained by the core library.
pub struct CommandRegistryGuard {
    previous: Option<Vec<String>>,
}
impl Drop for CommandRegistryGuard {
    fn drop(&mut self) {
        COMMANDS.with(|commands| *commands.borrow_mut() = self.previous.take());
    }
}
pub fn command_registry(commands: Vec<String>) -> CommandRegistryGuard {
    let previous = COMMANDS.with(|active| active.replace(Some(commands)));
    CommandRegistryGuard { previous }
}
pub const FILENAME: &str = "okf-bundle.yaml";
pub const SUPPORT_DIRECTORY: &str = ".okf-bundle";
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub manifest_version: u32,
    pub description: Option<String>,
    #[serde(default)]
    pub settings: BundleSettings,
    #[serde(default)]
    pub guidance: Guidance,
    #[serde(default)]
    pub workflows: BTreeMap<String, Workflow>,
    #[serde(default)]
    pub extensions: BTreeMap<String, serde_yaml::Value>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Guidance {
    #[serde(default)]
    pub provenance: BTreeMap<String, Value>,
    pub instructions: Option<String>,
    pub instructions_file: Option<String>,
    #[serde(default)]
    pub display: DisplaySettings,
    #[serde(default)]
    pub rules: Vec<Rule>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DisplaySettings {
    #[serde(default = "full")]
    pub json: String,
    #[serde(default = "summary")]
    pub text: String,
}
fn full() -> String {
    "full".into()
}
fn summary() -> String {
    "summary".into()
}
impl Default for DisplaySettings {
    fn default() -> Self {
        Self {
            json: full(),
            text: summary(),
        }
    }
}
/// Existing rules remain mandatory; reference bodies are explicitly expanded.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GuidanceTier {
    #[default]
    Mandatory,
    Reference,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rule {
    #[serde(default)]
    pub tier: GuidanceTier,
    #[serde(default)]
    pub provenance: BTreeMap<String, Value>,
    pub id: String,
    pub description: Option<String>,
    pub operations: Option<Vec<String>>,
    pub instructions: Option<String>,
    pub instructions_file: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Workflow {
    #[serde(default)]
    pub provenance: BTreeMap<String, Value>,
    pub description: String,
    pub instructions: Option<String>,
    pub instructions_file: Option<String>,
    #[serde(default)]
    pub inputs: BTreeMap<String, WorkflowInput>,
    #[serde(default)]
    pub resources: Vec<String>,
    #[serde(default)]
    pub outputs: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowInput {
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(default)]
    pub required: bool,
    pub description: Option<String>,
    pub default: Option<Value>,
    pub values: Option<Vec<String>>,
}
#[derive(Debug, Clone)]
pub struct LoadedManifest {
    pub manifest: Manifest,
    pub path: PathBuf,
    pub digest: String,
    pub settings_value: serde_yaml::Value,
}
fn usage(message: impl Into<String>) -> OkfError {
    OkfError::Usage(message.into())
}
fn body_shape(inline: &Option<String>, file: &Option<String>, required: bool) -> Result<()> {
    match (inline, file) {
        (Some(_), Some(_)) => Err(usage(
            "exactly one of instructions and instructions_file is allowed",
        )),
        (None, None) if required => Err(usage("instructions or instructions_file is required")),
        (Some(v), _) | (_, Some(v)) if v.trim().is_empty() => {
            Err(usage("instruction body/path must not be empty"))
        }
        _ => Ok(()),
    }
}
pub fn local_path(root: &Path, resource: &str) -> Result<PathBuf> {
    let rel = Path::new(resource);
    if resource.is_empty()
        || rel.is_absolute()
        || rel
            .components()
            .any(|c| !matches!(c, Component::Normal(_) | Component::CurDir))
    {
        return Err(usage(format!(
            "resource must be bundle-relative without traversal: {resource}"
        )));
    }
    let mut current = root.to_path_buf();
    for part in rel.components() {
        current.push(part);
        if root.as_os_str().is_empty() {
            continue;
        }
        if let Ok(meta) = std::fs::symlink_metadata(&current) {
            if meta.file_type().is_symlink() {
                return Err(usage(format!(
                    "support resource contains symlink: {resource}"
                )));
            }
        }
    }
    Ok(current)
}
fn check_input(root: &Path, input: &WorkflowInput, value: &Value) -> Result<()> {
    let valid = match input.type_.as_str() {
        "string" => value.is_string(),
        "boolean" => value.is_boolean(),
        "enum" => value.as_str().is_some_and(|v| {
            input
                .values
                .as_ref()
                .is_some_and(|vs| vs.iter().any(|x| x == v))
        }),
        "path" => value.as_str().is_some_and(|v| local_path(root, v).is_ok()),
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(usage(format!(
            "invalid workflow input value for type {}",
            input.type_
        )))
    }
}
pub fn parse(text: &str) -> Result<Manifest> {
    let manifest: Manifest =
        serde_yaml::from_str(text).map_err(|e| usage(format!("invalid {FILENAME}: {e}")))?;
    if manifest.manifest_version != 1 {
        return Err(usage(format!(
            "unsupported manifest_version {}; supported: 1",
            manifest.manifest_version
        )));
    }
    super::settings::validate_settings(&manifest.settings)?;
    body_shape(
        &manifest.guidance.instructions,
        &manifest.guidance.instructions_file,
        false,
    )?;
    for mode in [
        &manifest.guidance.display.json,
        &manifest.guidance.display.text,
    ] {
        if !matches!(mode.as_str(), "full" | "summary" | "off") {
            return Err(usage("guidance display must be full, summary, or off"));
        }
    }
    let mut ids = BTreeSet::new();
    for rule in &manifest.guidance.rules {
        if rule.id.trim().is_empty() || rule.id.starts_with('$') || !ids.insert(&rule.id) {
            return Err(usage(format!(
                "invalid or duplicate guidance rule ID {:?}",
                rule.id
            )));
        }
        body_shape(&rule.instructions, &rule.instructions_file, true)?;
        if rule
            .operations
            .as_ref()
            .is_some_and(|ops| ops.is_empty() || ops.iter().any(|o| o.trim().is_empty()))
        {
            return Err(usage(
                "operations must be omitted or a nonempty command list",
            ));
        }
    }
    for (id, w) in &manifest.workflows {
        if id.trim().is_empty() || w.description.trim().is_empty() {
            return Err(usage("workflow ID and description must be nonempty"));
        }
        body_shape(&w.instructions, &w.instructions_file, true)?;
        for (name, input) in &w.inputs {
            if name.is_empty()
                || !matches!(input.type_.as_str(), "string" | "boolean" | "enum" | "path")
            {
                return Err(usage(format!("invalid workflow input {name:?}")));
            }
            if input.required && input.default.is_some() {
                return Err(usage("required workflow input cannot have a default"));
            }
            if input.type_ == "enum" && input.values.as_ref().is_none_or(|v| v.is_empty()) {
                return Err(usage("enum input requires nonempty values"));
            }
            if input.type_ != "enum" && input.values.is_some() {
                return Err(usage("values is only supported for enum inputs"));
            }
            if let Some(v) = &input.default {
                check_input(Path::new(""), input, v)?;
            }
        }
    }
    for key in manifest.extensions.keys() {
        if !key.contains('.') {
            return Err(usage("extension keys must be namespaced"));
        }
    }
    COMMANDS.with(|commands| {
        if let Some(commands) = commands.borrow().as_ref() {
            validate_commands(&manifest, commands)
        } else {
            Ok(())
        }
    })?;
    Ok(manifest)
}
pub fn load(root: &Path) -> Result<Option<LoadedManifest>> {
    let path = local_path(root, FILENAME)?;
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(usage(format!("cannot read {}: {e}", path.display()))),
    };
    let manifest = parse(&text)?;
    let raw: serde_yaml::Value = serde_yaml::from_str(&text).map_err(|e| usage(e.to_string()))?;
    Ok(Some(LoadedManifest {
        manifest,
        path,
        digest: sha256_hex(text.as_bytes()),
        settings_value: raw
            .get("settings")
            .cloned()
            .unwrap_or_else(|| serde_yaml::Value::Mapping(Default::default())),
    }))
}
pub fn validate(root: &Path, commands: &[String]) -> Result<Option<LoadedManifest>> {
    let Some(loaded) = load(root)? else {
        return Ok(None);
    };
    validate_commands(&loaded.manifest, commands)?;
    validate_resources(root)
}
pub fn validate_resources(root: &Path) -> Result<Option<LoadedManifest>> {
    let Some(loaded) = load(root)? else {
        return Ok(None);
    };
    let g = &loaded.manifest.guidance;
    let instruction_files = g
        .instructions_file
        .iter()
        .chain(g.rules.iter().filter_map(|r| r.instructions_file.as_ref()))
        .chain(
            loaded
                .manifest
                .workflows
                .values()
                .filter_map(|w| w.instructions_file.as_ref()),
        );
    for reference in instruction_files {
        let path = local_path(root, reference)?;
        std::fs::read_to_string(&path).map_err(|e| {
            usage(format!(
                "cannot read required UTF-8 instruction file {}: {e}",
                path.display()
            ))
        })?;
    }
    for reference in loaded
        .manifest
        .workflows
        .values()
        .flat_map(|w| w.resources.iter())
    {
        let path = local_path(root, reference)?;
        if !path.is_file() {
            return Err(usage(format!(
                "declared support resource missing: {}",
                path.display()
            )));
        }
        std::fs::File::open(&path).map_err(|e| {
            usage(format!(
                "cannot read declared support resource {}: {e}",
                path.display()
            ))
        })?;
    }
    Ok(Some(loaded))
}
pub fn validate_commands(manifest: &Manifest, commands: &[String]) -> Result<()> {
    for operation in manifest
        .guidance
        .rules
        .iter()
        .flat_map(|r| r.operations.iter().flatten())
        .chain(
            manifest
                .settings
                .query_presets
                .values()
                .flat_map(|p| p.operations.iter()),
        )
    {
        if !commands.iter().any(|c| c == operation) {
            return Err(usage(format!(
                "unknown or unsupported operation {operation:?}"
            )));
        }
    }
    Ok(())
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ContextRequest {
    pub operations: Vec<String>,
    pub rules: Vec<String>,
    pub workflows: Vec<String>,
    pub all: bool,
    pub inputs: BTreeMap<String, Value>,
    pub inspection: bool,
    pub revision: Option<String>,
    pub query: Option<Value>,
}
#[derive(Debug, Clone, Serialize)]
pub struct BundleContext {
    pub root: PathBuf,
    pub manifest: Option<Value>,
    pub snapshot: Option<Value>,
    pub context_digest: String,
    pub context_digest_version: u32,
    pub state_digest: String,
    pub state_components: BTreeMap<String, String>,
    pub selection: ContextRequest,
    pub effective_settings: EffectiveSettings,
    pub setting_origins: BTreeMap<String, String>,
    pub records: Vec<Value>,
}
fn read_body(
    root: &Path,
    inline: &Option<String>,
    file: &Option<String>,
    digests: &mut BTreeMap<String, String>,
) -> Result<Option<String>> {
    if let Some(file) = file {
        let path = local_path(root, file)?;
        let body = std::fs::read_to_string(&path).map_err(|e| {
            usage(format!(
                "cannot read instruction file {}: {e}",
                path.display()
            ))
        })?;
        let digest = sha256_hex(body.as_bytes());
        if digests.get(file).is_some_and(|old| old != &digest) {
            return Err(usage(format!(
                "instruction file changed during context read: {file}"
            )));
        }
        digests.insert(file.clone(), digest);
        Ok(Some(body))
    } else {
        Ok(inline.clone())
    }
}
pub fn context(
    root: &Path,
    mut request: ContextRequest,
    commands: &[String],
) -> Result<BundleContext> {
    request.operations.sort();
    request.operations.dedup();
    request.rules.sort();
    request.rules.dedup();
    request.workflows.sort();
    request.workflows.dedup();
    request.inspection |= super::snapshot_context::is_inspection_active();
    let canonical_root = std::fs::canonicalize(root)?;
    let root = canonical_root.as_path();
    if request.all
        && (!request.operations.is_empty()
            || !request.workflows.is_empty()
            || !request.rules.is_empty())
    {
        return Err(usage(
            "--all cannot be combined with operation/workflow/rule selectors",
        ));
    }
    for op in &request.operations {
        if !commands.contains(op) {
            return Err(usage(format!("unknown operation {op:?}")));
        }
    }
    if request.revision.is_some() {
        return Err(usage(
            "historical manifest context requires snapshot materialization",
        ));
    }
    let selected_context = super::context::current_context(Some(&root.to_string_lossy()), None)?;
    let before_manifest = load(root)?.map(|m| m.digest);
    let effective = super::settings::for_context(&selected_context, root)?;
    let loaded = load(root)?;
    if loaded.as_ref().map(|m| &m.digest) != before_manifest.as_ref() {
        return Err(usage("manifest changed during context read"));
    }
    let snapshot = super::remote::origin(root)?.map(|origin| json!(origin));
    fn origins(
        value: &serde_json::Value,
        prefix: &str,
        source: &str,
        out: &mut BTreeMap<String, String>,
    ) {
        if let Some(map) = value.as_object() {
            for (key, value) in map {
                let path = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                origins(value, &path, source, out);
            }
        } else {
            out.insert(prefix.to_owned(), source.to_owned());
        }
    }
    let mut setting_origins = BTreeMap::new();
    origins(
        &serde_json::to_value(&effective.settings).map_err(|e| usage(e.to_string()))?,
        "",
        "default",
        &mut setting_origins,
    );
    let key = effective.bundle.as_deref().unwrap_or("default");
    if !(effective.bundle.is_none() && selected_context.catalog.is_some()) {
        if let Some(value) = selected_context
            .config
            .settings
            .get("bundle_settings")
            .and_then(|v| v.get(key))
        {
            origins(
                &serde_json::to_value(value).map_err(|e| usage(e.to_string()))?,
                "",
                "legacy",
                &mut setting_origins,
            );
        }
    }
    if let Some(loaded) = &loaded {
        origins(
            &serde_json::to_value(&loaded.settings_value).map_err(|e| usage(e.to_string()))?,
            "",
            "manifest",
            &mut setting_origins,
        );
    }
    let mut records = Vec::new();
    let mut digests = BTreeMap::new();
    let mut resource_digests = BTreeMap::new();
    let role = if request.inspection || request.all {
        "inspected"
    } else {
        "task-guidance"
    };
    if let Some(l) = &loaded {
        validate_commands(&l.manifest, commands)?;
        let g = &l.manifest.guidance;
        if let Some(body) = read_body(root, &g.instructions, &g.instructions_file, &mut digests)? {
            records.push(json!({"kind":"guidance","id":"$global","matches_selection":true,"instruction_role":role,"content":"full","instructions":body,"origin":{"manifest":l.path,"manifest_digest":l.digest,"instructions_file":g.instructions_file,"provenance":g.provenance}}));
        }
        for id in &request.rules {
            if !g.rules.iter().any(|r| &r.id == id) {
                return Err(usage(format!("unknown guidance rule {id:?}")));
            }
        }
        for r in &g.rules {
            let matches = r
                .operations
                .as_ref()
                .map(|ops| request.operations.iter().any(|o| ops.contains(o)))
                .unwrap_or(true);
            let mandatory = r.tier == GuidanceTier::Mandatory;
            let full = request.all || (matches && mandatory) || request.rules.contains(&r.id);
            let mut row = json!({"kind":"guidance","id":r.id,"description":r.description,"operations":r.operations,"matches_selection":if r.operations.is_some()&&request.operations.is_empty(){None}else{Some(matches)},"tier":r.tier,"instruction_role":if matches && mandatory{role}else{"inspected"},"content":if full{"full"}else{"summary"},"origin":{"manifest":l.path,"manifest_digest":l.digest,"instructions_file":r.instructions_file,"provenance":r.provenance}});
            if full {
                row["instructions"] = json!(read_body(
                    root,
                    &r.instructions,
                    &r.instructions_file,
                    &mut digests
                )?);
            } else {
                row["omission_reason"] = json!(if matches {
                    "reference-on-demand"
                } else {
                    "outside-selection"
                });
            }
            records.push(row);
        }
        for name in &request.workflows {
            if !l.manifest.workflows.contains_key(name) {
                return Err(usage(format!("unknown workflow {name:?}")));
            }
        }
        for (name, w) in &l.manifest.workflows {
            let selected = request.workflows.contains(name);
            let full = request.all || selected;
            let mut row = json!({"kind":"workflow","id":name,"description":w.description,"selected":selected,"content":if full{"full"}else{"summary"},"origin":{"manifest":l.path,"manifest_digest":l.digest,"instructions_file":w.instructions_file,"provenance":w.provenance}});
            if full {
                row["instructions"] = json!(read_body(
                    root,
                    &w.instructions,
                    &w.instructions_file,
                    &mut digests
                )?);
                row["inputs"] = json!(w.inputs);
                row["resources"] = json!(w.resources);
                row["outputs"] = json!(w.outputs);
            }
            if full {
                let mut resources = Vec::new();
                for resource in &w.resources {
                    let path = local_path(root, resource)?;
                    let bytes = std::fs::read(&path).map_err(|e| {
                        usage(format!(
                            "cannot read workflow resource {}: {e}",
                            path.display()
                        ))
                    })?;
                    resource_digests.insert(resource.clone(), sha256_hex(&bytes));
                    resources.push(
                        json!({"path":resource,"digest":sha256_hex(&bytes),"size":bytes.len()}),
                    );
                }
                row["resource_metadata"] = json!(resources);
            }
            if !full {
                row["omission_reason"] = json!("workflow-not-selected");
            }
            if selected {
                for k in request.inputs.keys() {
                    if !w.inputs.contains_key(k) {
                        return Err(usage(format!("unknown input {k:?} for workflow {name}")));
                    }
                }
                let mut values = BTreeMap::new();
                let mut origins = BTreeMap::new();
                let mut missing = Vec::new();
                for (key, input) in &w.inputs {
                    if let Some(value) = request.inputs.get(key).or(input.default.as_ref()) {
                        check_input(root, input, value)?;
                        values.insert(key.clone(), value.clone());
                        origins.insert(
                            key.clone(),
                            if request.inputs.contains_key(key) {
                                "supplied"
                            } else {
                                "default"
                            },
                        );
                    } else if input.required {
                        missing.push(key.clone());
                    }
                }
                row["effective_inputs"] = json!(values);
                row["input_origins"] = json!(origins);
                row["inputs_complete"] = json!(missing.is_empty());
                row["missing_required"] = json!(missing);
            }
            records.push(row);
        }
    } else if !request.workflows.is_empty() || !request.rules.is_empty() {
        return Err(usage("bundle has no guidance rules or workflows"));
    }
    if !request.inputs.is_empty() && request.workflows.is_empty() {
        return Err(usage("workflow inputs require a selected workflow"));
    }
    let manifest = loaded
        .as_ref()
        .map(|l| json!({"path":l.path,"version":l.manifest.manifest_version,"digest":l.digest}));
    let hash = |value: Value| -> Result<String> {
        Ok(sha256_hex(
            &serde_json::to_vec(&value).map_err(|e| usage(e.to_string()))?,
        ))
    };
    let mut state_components = BTreeMap::from([
        (
            "identity".into(),
            hash(json!({"root":root,"bundle":effective.bundle}))?,
        ),
        ("manifest".into(), hash(json!(manifest))?),
        (
            "settings".into(),
            hash(json!({"effective":effective,"origins":setting_origins}))?,
        ),
        ("snapshot".into(), hash(json!(snapshot))?),
    ]);
    state_components.extend(
        digests
            .iter()
            .map(|(p, d)| (format!("instruction:{p}"), d.clone())),
    );
    state_components.extend(
        resource_digests
            .into_iter()
            .map(|(p, d)| (format!("resource:{p}"), d)),
    );
    let state_digest = hash(json!(state_components))?;
    let digest = hash(json!({"version":2,"state":state_digest,"selection":request}))?;
    for record in &mut records {
        record["context_digest"] = json!(digest);
        if record["kind"] == "workflow" {
            record["instruction_role"] = json!(if request.inspection
                || request.all
                || record["selected"] != true
            {
                "inspected"
            } else {
                "task-guidance"
            });
        }
        if let Some(file) = record["origin"]["instructions_file"]
            .as_str()
            .map(str::to_owned)
        {
            if let Some(digest) = digests.get(&file) {
                record["origin"]["instruction_digest"] = json!(digest);
            }
        }
    }
    Ok(BundleContext {
        root: root.to_path_buf(),
        manifest,
        snapshot,
        context_digest: digest,
        context_digest_version: 2,
        state_digest,
        state_components,
        selection: request,
        effective_settings: effective,
        setting_origins,
        records,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn write(root: &Path, text: &str) {
        std::fs::write(root.join(FILENAME), text).unwrap();
    }
    #[test]
    fn rejects_ambiguous_bodies_unknown_keys_and_versions() {
        for text in ["manifest_version: 2", "manifest_version: 1\nunknown: true", "manifest_version: 1\nguidance:\n  instructions: hi\n  instructions_file: x", "manifest_version: 1\nguidance:\n  rules:\n  - id: r\n    operations: []\n    instructions: hi"] {assert!(parse(text).is_err(),"{text}");}
    }
    #[test]
    fn instruction_file_change_changes_context_identity() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(
            root,
            "manifest_version: 1\nguidance:\n  instructions_file: .okf-bundle/spec.txt\n",
        );
        std::fs::create_dir(root.join(SUPPORT_DIRECTORY)).unwrap();
        std::fs::write(root.join(".okf-bundle/spec.txt"), "first").unwrap();
        let first = context(root, ContextRequest::default(), &[]).unwrap();
        std::fs::write(root.join(".okf-bundle/spec.txt"), "second").unwrap();
        let second = context(root, ContextRequest::default(), &[]).unwrap();
        assert_eq!(first.manifest, second.manifest);
        assert_ne!(first.context_digest, second.context_digest);
        assert_eq!(second.records[0]["instructions"], "second");
    }
    #[test]
    fn scoped_rules_and_inspection_do_not_select_workflows() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(),"manifest_version: 1\nguidance:\n  rules:\n  - id: graph\n    operations: [links]\n    instructions: edges\nworkflows:\n  import:\n    description: Import\n    instructions: Do it\n");
        let commands = vec!["links".into(), "backlinks".into()];
        let ctx = context(
            dir.path(),
            ContextRequest {
                operations: vec!["backlinks".into()],
                ..Default::default()
            },
            &commands,
        )
        .unwrap();
        assert_eq!(ctx.records[0]["content"], "summary");
        let all = context(
            dir.path(),
            ContextRequest {
                all: true,
                ..Default::default()
            },
            &commands,
        )
        .unwrap();
        assert_eq!(all.records[0]["instruction_role"], "inspected");
        assert_eq!(all.records[1]["selected"], false);
        assert_eq!(all.records[1]["instructions"], "Do it");
    }
    #[test]
    fn workflow_inputs_validate_and_report_missing() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(),"manifest_version: 1\nworkflows:\n  import:\n    description: Import\n    instructions: Do it\n    inputs:\n      source: {type: string, required: true}\n      destination: {type: path, default: requirements}\n");
        let request = ContextRequest {
            workflows: vec!["import".into()],
            ..Default::default()
        };
        let ctx = context(dir.path(), request.clone(), &[]).unwrap();
        assert_eq!(ctx.records[0]["inputs_complete"], false);
        assert_eq!(
            ctx.records[0]["effective_inputs"]["destination"],
            "requirements"
        );
        let mut bad = request;
        bad.inputs.insert("destination".into(), json!("../escape"));
        assert!(context(dir.path(), bad, &[]).is_err());
    }
    #[test]
    fn executable_command_registry_validates_normal_parse_and_restores() {
        let text="manifest_version: 1\nguidance:\n  rules:\n  - id: bad\n    operations: [not-a-command]\n    instructions: nope\n";
        assert!(parse(text).is_ok());
        {
            let _registry = command_registry(vec!["links".into()]);
            assert!(parse(text).is_err());
        }
        assert!(parse(text).is_ok());
    }
    #[test]
    fn primary_and_relationship_presets_are_not_interchangeable() {
        assert!(parse("manifest_version: 1\nsettings:\n  query_presets:\n    defects:\n      operations: [search, list]\n      query: {type: Defect, tag: active}\n").is_ok());
        assert!(parse("manifest_version: 1\nsettings:\n  query_presets:\n    defects:\n      operations: [search]\n      filters: {kind: [blocks]}\n").is_err());
    }
    #[test]
    fn instruction_files_must_be_utf8_while_resources_may_be_binary() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("data.bin"), [0xff, 0xfe]).unwrap();
        write(
            dir.path(),
            "manifest_version: 1\nguidance:\n  instructions_file: data.bin\n",
        );
        assert!(validate_resources(dir.path()).is_err());
        write(dir.path(),"manifest_version: 1\nworkflows:\n  inspect:\n    description: Inspect data\n    instructions: Read metadata\n    resources: [data.bin]\n");
        assert!(validate_resources(dir.path()).is_ok());
    }
    #[test]
    fn support_paths_cannot_escape() {
        assert!(local_path(Path::new("/tmp"), "../secret").is_err());
        assert!(local_path(Path::new("/tmp"), "/secret").is_err());
    }
    #[test]
    fn manifest_settings_are_portable_and_conflicts_fail() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(
            root,
            "manifest_version: 1\nsettings:\n  query: {scan_limit: 42}\n",
        );
        let ctx =
            super::super::context::resolve_context(Some(root.to_str().unwrap()), None, None, root)
                .unwrap();
        assert_eq!(
            super::super::settings::for_context(&ctx, root)
                .unwrap()
                .settings
                .query
                .scan_limit,
            Some(42)
        );
        std::fs::write(
            root.join("okf.toml"),
            "[bundle_settings.default.query]\nscan_limit=43\n",
        )
        .unwrap();
        let ctx =
            super::super::context::resolve_context(Some(root.to_str().unwrap()), None, None, root)
                .unwrap();
        assert!(super::super::settings::for_context(&ctx, root).is_err());
    }
}
