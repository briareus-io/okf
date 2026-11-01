//! Explicit, reviewable migration of repository TOML into portable YAML configuration.
//! Planning does not modify files. Applying checks all inputs before publication and
//! retains the exact original bytes in a recovery directory.
use crate::error::{OkfError, Result};
use crate::fingerprint::canonicalize::sha256_hex;
use serde::{Deserialize, Serialize};
use serde_yaml::{Mapping, Value};
use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MigrationOptions {
    /// Required author choice when a path-only default has no existing registration.
    pub default_bundle_id: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MigrationChange {
    pub path: PathBuf,
    pub before_digest: Option<String>,
    pub after: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MigrationInput {
    pub path: PathBuf,
    pub digest: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MigrationPlan {
    pub plan_version: u32,
    pub config: PathBuf,
    pub options: MigrationOptions,
    pub inputs: Vec<MigrationInput>,
    pub changes: Vec<MigrationChange>,
    pub unresolved: Vec<String>,
    pub notes: Vec<String>,
    pub digest: String,
}
#[derive(Debug, Clone, Serialize)]
pub struct MigrationReport {
    pub applied: bool,
    pub changed: Vec<PathBuf>,
    pub backup_directory: PathBuf,
}
fn usage(s: impl Into<String>) -> OkfError {
    OkfError::Usage(s.into())
}
fn key(s: &str) -> Value {
    Value::String(s.into())
}
fn read(path: &Path) -> Result<Option<Vec<u8>>> {
    match std::fs::read(path) {
        Ok(v) => Ok(Some(v)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(OkfError::Io(format!("{}: {e}", path.display()))),
    }
}
fn digest(path: &Path) -> Result<Option<String>> {
    Ok(read(path)?.map(|b| sha256_hex(&b)))
}
fn absolute(path: &Path) -> Result<PathBuf> {
    super::catalog::absolute(path)
}
fn no_symlinks(path: &Path) -> Result<()> {
    let mut current = PathBuf::new();
    for part in path.components() {
        current.push(part);
        match std::fs::symlink_metadata(&current) {
            Ok(m) if m.file_type().is_symlink() => {
                return Err(usage(format!(
                    "migration path contains a symlink: {}",
                    current.display()
                )))
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}
fn relative(from: &Path, to: &Path) -> PathBuf {
    let a: Vec<_> = from.components().collect();
    let b: Vec<_> = to.components().collect();
    let shared = a.iter().zip(&b).take_while(|(a, b)| a == b).count();
    if shared == 0 {
        return to.to_path_buf();
    }
    let mut result = PathBuf::new();
    for part in &a[shared..] {
        if matches!(part, Component::Normal(_)) {
            result.push("..");
        }
    }
    for part in &b[shared..] {
        result.push(part);
    }
    if result.as_os_str().is_empty() {
        result.push(".");
    }
    result
}
fn add_input(plan: &mut MigrationPlan, path: &Path) -> Result<()> {
    if !plan.inputs.iter().any(|i| i.path == path) {
        plan.inputs.push(MigrationInput {
            path: path.into(),
            digest: digest(path)?,
        });
    }
    Ok(())
}
fn merge(target: &mut Value, source: Value, label: &str) -> Result<()> {
    match (target, source) {
        (Value::Mapping(a), Value::Mapping(b)) => {
            for (k, v) in b {
                if let Some(old) = a.get_mut(&k) {
                    merge(old, v, &format!("{label}.{}", k.as_str().unwrap_or("?")))?;
                } else {
                    a.insert(k, v);
                }
            }
            Ok(())
        }
        (a, b) if *a == b => Ok(()),
        _ => Err(usage(format!(
            "conflicting old and new configuration at {label}"
        ))),
    }
}
fn yaml_at(path: &Path) -> Result<Value> {
    match read(path)? {
        Some(b) => serde_yaml::from_slice(&b)
            .map_err(|e| usage(format!("invalid YAML {}: {e}", path.display()))),
        None => Ok(Value::Mapping(Mapping::new())),
    }
}
fn stage(plan: &mut MigrationPlan, path: PathBuf, after: Option<String>) -> Result<()> {
    add_input(plan, &path)?;
    no_symlinks(&path)?;
    if read(&path)?.as_deref() != after.as_ref().map(|s| s.as_bytes()) {
        plan.changes.push(MigrationChange {
            before_digest: digest(&path)?,
            path,
            after,
        });
    }
    Ok(())
}
pub fn plan(config: &Path) -> Result<MigrationPlan> {
    plan_with_options(config, MigrationOptions::default())
}
pub fn plan_with_options(config: &Path, options: MigrationOptions) -> Result<MigrationPlan> {
    let config = absolute(config)?;
    let parent = config
        .parent()
        .ok_or_else(|| usage("configuration has no parent"))?
        .canonicalize()?;
    let config = parent.join(
        config
            .file_name()
            .ok_or_else(|| usage("configuration has no filename"))?,
    );
    no_symlinks(&config)?;
    let bytes = read(&config)?.ok_or_else(|| {
        usage(format!(
            "legacy configuration is missing: {}",
            config.display()
        ))
    })?;
    let text = std::str::from_utf8(&bytes).map_err(|e| usage(e.to_string()))?;
    let parsed: super::config::Config =
        toml::from_str(text).map_err(|e| usage(format!("invalid legacy configuration: {e}")))?;
    let base = config
        .parent()
        .ok_or_else(|| usage("configuration has no parent"))?;
    parsed.validate(base)?;
    let mut legacy: toml::Value = toml::from_str(text).map_err(|e| usage(e.to_string()))?;
    let mut out = MigrationPlan {
        plan_version: 1,
        config: config.clone(),
        options: options.clone(),
        inputs: vec![],
        changes: vec![],
        unresolved: vec![],
        notes: vec![],
        digest: String::new(),
    };
    add_input(&mut out, &config)?;
    let target_catalog = base.join("okf-catalog.yaml");
    let old_catalog = parsed
        .catalog
        .as_ref()
        .map(|s| absolute(&base.join(s)))
        .transpose()?;
    let mut catalog = if let Some(path) = &old_catalog {
        add_input(&mut out, path)?;
        yaml_at(path)?
    } else {
        Value::Mapping(Mapping::new())
    };
    let cm = catalog
        .as_mapping_mut()
        .ok_or_else(|| usage("catalog must be a mapping"))?;
    cm.insert(key("catalog_version"), Value::Number(2.into()));
    let registrations = cm
        .entry(key("bundles"))
        .or_insert_with(|| Value::Mapping(Mapping::new()))
        .as_mapping_mut()
        .ok_or_else(|| usage("catalog bundles must be a mapping"))?;
    let mut roots = BTreeMap::<String, PathBuf>::new();
    let old_base = old_catalog
        .as_deref()
        .and_then(Path::parent)
        .unwrap_or(base);
    for (id, registration) in registrations.iter_mut() {
        let id = id
            .as_str()
            .ok_or_else(|| usage("catalog IDs must be strings"))?
            .to_string();
        if !super::catalog::valid_id(&id) {
            return Err(usage(format!("invalid catalog bundle ID: {id}")));
        }
        let loc = registration
            .get_mut("location")
            .and_then(Value::as_mapping_mut)
            .ok_or_else(|| usage("registration lacks location"))?;
        if loc.get(key("type")).and_then(Value::as_str) != Some("directory") {
            out.unresolved.push(format!(
                "registration {id} is not a legacy directory registration"
            ));
            continue;
        }
        let raw = loc
            .get(key("path"))
            .and_then(Value::as_str)
            .ok_or_else(|| usage("directory location needs path"))?;
        let root = absolute(&old_base.join(raw))?;
        loc.insert(
            key("path"),
            Value::String(relative(base, &root).to_string_lossy().into()),
        );
        roots.insert(id, root);
    }
    let default_path = parsed
        .default_bundle
        .as_ref()
        .and_then(|s| s.path.as_ref())
        .or(parsed.bundle.as_ref())
        .map(|s| absolute(&base.join(s)))
        .transpose()?;
    let mut default_id = parsed.default_bundle.as_ref().and_then(|s| s.id.clone());
    if let Some(path) = &default_path {
        default_id = roots
            .iter()
            .find(|(_, root)| *root == path)
            .map(|(id, _)| id.clone());
        if default_id.is_none() {
            if let Some(id) = &options.default_bundle_id {
                if !super::catalog::valid_id(id) {
                    return Err(usage("default bundle ID must be namespaced"));
                }
                if roots.contains_key(id) {
                    return Err(usage(
                        "chosen default bundle ID already refers to a different location",
                    ));
                }
                let reg = serde_yaml::to_value(
                    serde_json::json!({"location":{"type":"directory","path":relative(base,path)}}),
                )
                .map_err(|e| usage(e.to_string()))?;
                registrations.insert(key(id), reg);
                roots.insert(id.clone(), path.clone());
                default_id = Some(id.clone());
            } else {
                out.unresolved.push(format!(
                    "choose a stable default_bundle_id for {}",
                    path.display()
                ));
            }
        }
    }
    let cm = catalog.as_mapping_mut().unwrap();
    if let Some(id) = &default_id {
        if !roots.contains_key(id) {
            out.unresolved
                .push(format!("default bundle ID {id} is not registered"));
        }
        if let Some(old) = cm.get(key("default_bundle")) {
            if old != &key(id) {
                out.unresolved
                    .push("catalog and TOML default_bundle conflict".into());
            }
        }
        cm.insert(key("default_bundle"), key(id));
    }
    if let Some(overrides) = &parsed.catalog_overrides {
        let path = absolute(&base.join(overrides))?;
        cm.insert(
            key("catalog_overrides"),
            key(&relative(base, &path).to_string_lossy()),
        );
        add_input(&mut out, &path)?;
    }
    if let Some(groups) = parsed
        .settings
        .get("bundle_settings")
        .and_then(toml::Value::as_table)
    {
        for (id, settings) in groups {
            let root = if id == "default" && old_catalog.is_none() {
                default_path.as_ref()
            } else {
                roots.get(id)
            };
            let Some(root) = root else {
                out.unresolved.push(format!(
                    "bundle_settings.{id} has no unambiguous bundle destination"
                ));
                continue;
            };
            if !root.is_dir() {
                out.unresolved.push(format!(
                    "bundle destination unavailable: {}",
                    root.display()
                ));
                continue;
            }
            let mut value = serde_yaml::to_value(settings).map_err(|e| usage(e.to_string()))?;
            if let Some(ontology) = value.get_mut("ontology") {
                let raw = ontology
                    .as_str()
                    .ok_or_else(|| usage("ontology must be a path string"))?;
                let target = absolute(&base.join(raw))?;
                if !target.starts_with(root) {
                    out.notes.push(format!(
                        "nonportable external ontology retained for {id}: {}",
                        target.display()
                    ));
                }
                *ontology = key(&relative(root, &target).to_string_lossy());
            }
            let path = root.join("okf-bundle.yaml");
            if let Err(error) = super::manifest::validate_resources(root) {
                out.unresolved.push(format!("{}: {error}", path.display()));
                continue;
            }
            let mut manifest = yaml_at(&path)?;
            // Equivalent ontology locations may use different relative spellings.
            if let (Some(existing), Some(incoming)) = (
                manifest
                    .get_mut("settings")
                    .and_then(|s| s.get_mut("ontology")),
                value.get("ontology"),
            ) {
                if let (Some(old), Some(new)) = (existing.as_str(), incoming.as_str()) {
                    if absolute(&root.join(old))? == absolute(&root.join(new))? {
                        *existing = incoming.clone();
                    }
                }
            }
            let map = manifest
                .as_mapping_mut()
                .ok_or_else(|| usage("manifest must be a mapping"))?;
            if map
                .get(key("manifest_version"))
                .is_some_and(|v| v.as_u64() != Some(1))
            {
                out.unresolved.push(format!(
                    "unsupported existing manifest version: {}",
                    path.display()
                ));
                continue;
            }
            map.insert(key("manifest_version"), Value::Number(1.into()));
            let old = map
                .entry(key("settings"))
                .or_insert_with(|| Value::Mapping(Mapping::new()));
            if let Err(e) = merge(old, value, "settings") {
                out.unresolved.push(format!("{}: {e}", path.display()));
                continue;
            }
            let serialized = serde_yaml::to_string(&manifest).map_err(|e| usage(e.to_string()))?;
            if let Err(e) = super::manifest::parse(&serialized) {
                out.unresolved.push(format!("{}: {e}", path.display()));
                continue;
            }
            stage(&mut out, path, Some(serialized))?;
        }
    }
    if target_catalog != old_catalog.clone().unwrap_or_default() && target_catalog.exists() {
        let mut existing = yaml_at(&target_catalog)?;
        if let Err(e) = merge(&mut existing, catalog.clone(), "catalog") {
            out.unresolved.push(e.to_string());
        } else {
            catalog = existing;
        }
    }
    stage(
        &mut out,
        target_catalog,
        Some(serde_yaml::to_string(&catalog).map_err(|e| usage(e.to_string()))?),
    )?;
    let table = legacy
        .as_table_mut()
        .ok_or_else(|| usage("legacy config must be a table"))?;
    for k in [
        "bundle",
        "default_bundle",
        "catalog",
        "catalog_overrides",
        "bundle_settings",
    ] {
        table.remove(k);
    }
    let remaining = if table.is_empty() {
        None
    } else {
        out.notes.push(
            "Unknown legacy keys retained in okf.toml; exact original bytes retained in backup."
                .into(),
        );
        Some(toml::to_string_pretty(&legacy).map_err(|e| usage(e.to_string()))?)
    };
    stage(&mut out, config, remaining)?;
    out.inputs.sort_by(|a, b| a.path.cmp(&b.path));
    out.changes.sort_by(|a, b| a.path.cmp(&b.path));
    out.digest = sha256_hex(&serde_json::to_vec(&out).map_err(|e| usage(e.to_string()))?);
    Ok(out)
}
pub fn save(plan: &MigrationPlan, path: &Path) -> Result<()> {
    use std::io::Write;
    super::remote::ensure_writable(path)?;
    let bytes = serde_yaml::to_string(plan).map_err(|e| usage(e.to_string()))?;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| {
            usage(format!(
                "cannot create migration plan {}: {e}",
                path.display()
            ))
        })?;
    if let Err(error) = file
        .write_all(bytes.as_bytes())
        .and_then(|_| file.sync_all())
    {
        drop(file);
        let _ = std::fs::remove_file(path);
        return Err(error.into());
    }
    Ok(())
}
pub fn load(path: &Path) -> Result<MigrationPlan> {
    serde_yaml::from_slice(&std::fs::read(path)?)
        .map_err(|e| usage(format!("invalid migration plan: {e}")))
}
pub fn apply(plan: &MigrationPlan) -> Result<MigrationReport> {
    if plan.plan_version != 1 {
        return Err(usage("unsupported migration plan version"));
    }
    if !plan.unresolved.is_empty() {
        return Err(usage(format!(
            "migration requires review: {}",
            plan.unresolved.join("; ")
        )));
    }
    // Recompute the complete plan: a saved plan cannot introduce arbitrary write destinations.
    let expected = plan_with_options(&plan.config, plan.options.clone())?;
    if serde_json::to_value(&expected).unwrap() != serde_json::to_value(plan).unwrap() {
        return Err(usage(
            "migration plan changed or inputs are stale; regenerate and review",
        ));
    }
    let base = plan.config.parent().unwrap();
    let lock_path = base.join(".okf-migration.lock");
    super::remote::ensure_writable(&lock_path)?;
    for change in &plan.changes {
        super::remote::ensure_writable(&change.path)?;
    }
    let _lock = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&lock_path)
        .map_err(|e| usage(format!("cannot acquire migration lock: {e}")))?;
    struct Guard(PathBuf);
    impl Drop for Guard {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }
    let _guard = Guard(lock_path);
    for input in &plan.inputs {
        if digest(&input.path)? != input.digest {
            return Err(usage("migration inputs changed while acquiring lock"));
        }
    }
    let backup = base.join(".okf-migration").join(&plan.digest);
    super::remote::ensure_writable(&backup)?;
    for change in &plan.changes {
        super::remote::ensure_writable(&change.path)?;
    }
    no_symlinks(&backup)?;
    std::fs::create_dir_all(&backup)?;
    save(plan, &backup.join("plan.yaml"))?;
    let mut originals = Vec::new();
    for (i, change) in plan.changes.iter().enumerate() {
        no_symlinks(&change.path)?;
        let before = read(&change.path)?;
        if let Some(bytes) = &before {
            std::fs::write(backup.join(format!("{i}.before")), bytes)?;
        }
        if let Some(after) = &change.after {
            std::fs::write(backup.join(format!("{i}.after")), after)?;
        }
        originals.push(before);
    }
    std::fs::write(backup.join("state"), "prepared\n")?;
    let mut applied = 0;
    let result = (|| -> Result<()> {
        for (i, change) in plan.changes.iter().enumerate() {
            if digest(&change.path)? != change.before_digest {
                return Err(usage(format!(
                    "migration target changed: {}",
                    change.path.display()
                )));
            }
            if let Some(after) = &change.after {
                let parent = change.path.parent().unwrap();
                std::fs::create_dir_all(parent)?;
                let staging = parent.join(format!(".okf-migrate-{}-{i}", &plan.digest[..12]));
                let mut file = std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&staging)?;
                use std::io::Write;
                file.write_all(after.as_bytes())?;
                file.sync_all()?;
                drop(file);
                std::fs::rename(staging, &change.path)?;
            } else if change.path.exists() {
                std::fs::remove_file(&change.path)?;
            }
            applied = i + 1;
            std::fs::write(backup.join("state"), format!("applied {applied}\n"))?;
        }
        Ok(())
    })();
    if let Err(error) = result {
        let mut rollback_errors = Vec::new();
        for i in (0..applied).rev() {
            let path = &plan.changes[i].path;
            let r = if let Some(bytes) = &originals[i] {
                std::fs::write(path, bytes)
            } else {
                std::fs::remove_file(path)
            };
            if let Err(e) = r {
                rollback_errors.push(e.to_string());
            }
        }
        std::fs::write(
            backup.join("state"),
            if rollback_errors.is_empty() {
                "rolled-back\n"
            } else {
                "recovery-required\n"
            },
        )?;
        return Err(OkfError::Environment(format!(
            "migration failed: {error}; backup {}; rollback errors: {}",
            backup.display(),
            rollback_errors.join("; ")
        )));
    }
    std::fs::write(backup.join("state"), "complete\n")?;
    Ok(MigrationReport {
        applied: true,
        changed: plan.changes.iter().map(|c| c.path.clone()).collect(),
        backup_directory: backup,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("knowledge")).unwrap();
        std::fs::write(dir.path().join("okf.toml"),"default_bundle = { path = 'knowledge' }\n[bundle_settings.default.lint]\nsource_unrecorded = 'off'\n").unwrap();
        dir
    }
    #[test]
    fn author_identity_choice_is_required_and_plan_is_read_only() {
        let dir = fixture();
        let p = plan(&dir.path().join("okf.toml")).unwrap();
        assert_eq!(p.unresolved.len(), 1);
        assert!(!dir.path().join("okf-catalog.yaml").exists());
        assert!(apply(&p).is_err());
        assert!(dir.path().join("okf.toml").exists());
    }
    #[test]
    fn migration_preserves_exact_original_and_rebases_ontology() {
        let dir = fixture();
        let original="# preserved in backup\ndefault_bundle={path='knowledge'}\n[bundle_settings.default]\nontology='knowledge/ontology.yaml'\n[bundle_settings.default.lint]\nsource_unrecorded='off'\n";
        std::fs::write(dir.path().join("okf.toml"), original).unwrap();
        let p = plan_with_options(
            &dir.path().join("okf.toml"),
            MigrationOptions {
                default_bundle_id: Some("example.docs".into()),
            },
        )
        .unwrap();
        assert!(p.unresolved.is_empty(), "{:?}", p.unresolved);
        let report = apply(&p).unwrap();
        assert!(report.applied);
        assert!(!dir.path().join("okf.toml").exists());
        let m = std::fs::read_to_string(dir.path().join("knowledge/okf-bundle.yaml")).unwrap();
        assert!(m.contains("ontology: ontology.yaml"));
        let index = p
            .changes
            .iter()
            .position(|c| c.path == dir.path().canonicalize().unwrap().join("okf.toml"))
            .unwrap();
        assert_eq!(
            std::fs::read_to_string(report.backup_directory.join(format!("{index}.before")))
                .unwrap(),
            original
        );
    }
    #[test]
    fn stale_and_modified_plans_cannot_write() {
        let dir = fixture();
        let mut p = plan_with_options(
            &dir.path().join("okf.toml"),
            MigrationOptions {
                default_bundle_id: Some("example.docs".into()),
            },
        )
        .unwrap();
        p.changes[0].after = Some("injected".into());
        assert!(apply(&p).is_err());
        let p = plan_with_options(
            &dir.path().join("okf.toml"),
            MigrationOptions {
                default_bundle_id: Some("example.docs".into()),
            },
        )
        .unwrap();
        std::fs::write(
            dir.path().join("knowledge/okf-bundle.yaml"),
            "manifest_version: 1\ndescription: changed\n",
        )
        .unwrap();
        assert!(apply(&p).is_err());
        assert!(dir.path().join("okf.toml").exists());
    }
    #[test]
    fn conflicting_manifest_does_not_get_overwritten() {
        let dir = fixture();
        std::fs::write(
            dir.path().join("knowledge/okf-bundle.yaml"),
            "manifest_version: 1\nsettings:\n  lint:\n    source_unrecorded: error\n",
        )
        .unwrap();
        let p = plan_with_options(
            &dir.path().join("okf.toml"),
            MigrationOptions {
                default_bundle_id: Some("example.docs".into()),
            },
        )
        .unwrap();
        assert!(!p.unresolved.is_empty());
        assert!(apply(&p).is_err());
    }
    #[test]
    fn unknown_legacy_keys_survive() {
        let dir = fixture();
        std::fs::write(
            dir.path().join("okf.toml"),
            "default_bundle={path='knowledge'}\ncustom='keep'\n",
        )
        .unwrap();
        let p = plan_with_options(
            &dir.path().join("okf.toml"),
            MigrationOptions {
                default_bundle_id: Some("example.docs".into()),
            },
        )
        .unwrap();
        apply(&p).unwrap();
        assert!(std::fs::read_to_string(dir.path().join("okf.toml"))
            .unwrap()
            .contains("keep"));
    }
}
