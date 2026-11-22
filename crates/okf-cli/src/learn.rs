//! User-local feedback. Only closed outcome identifiers cross the observer boundary.
use crate::cli::{ChangeSetCmd, Command};
use clap::{Subcommand, ValueEnum};
use okf_core::error::{FailureReason, OkfError, Result};
use okf_core::fingerprint::canonicalize::sha256_hex;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;

#[derive(Debug, Subcommand)]
pub enum LearnCmd {
    /// Enable local counting after interactive consent.
    Enable,
    /// Stop counting and retain local feedback.
    Disable,
    /// Inspect the managed location, consent, draft count, and data size.
    Status,
    /// Rank locally counted outcomes by affected invocations.
    Report,
    /// Remove managed feedback and disable counting.
    Purge,
    /// Create a private editable Markdown draft from synthetic guidance.
    Draft {
        #[arg(value_enum)]
        kind: FeedbackKind,
        /// Optional built-in outcome from the local report.
        #[arg(long)]
        pattern: Option<Pattern>,
    },
    /// Preview a draft and destination, then export with its confirmation digest.
    Export {
        /// Managed draft filename returned by learn draft.
        draft: String,
        /// Explicit new file to create.
        #[arg(long)]
        output: PathBuf,
        /// Digest from the reviewed preview, binding content and destination.
        #[arg(long)]
        confirm: Option<String>,
    },
}

impl LearnCmd {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Enable => "enable",
            Self::Disable => "disable",
            Self::Status => "status",
            Self::Report => "report",
            Self::Purge => "purge",
            Self::Draft { .. } => "draft",
            Self::Export { .. } => "export",
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum FeedbackKind {
    Defect,
    FeatureRequest,
    GuidanceGap,
    WorkflowLesson,
}

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum Pattern {
    ThresholdReached,
    InvalidBundle,
    BrokenLink,
    DeclarationFinding,
    ScopedReferrers,
    BaselineConflict,
    WriteGateRejected,
    PolicyRefusal,
}

impl Pattern {
    fn synthetic_example(self) -> &'static str {
        match self {
            Self::BrokenLink => {
                "Give a fictional note a link to a missing note. Run lint and inspect the finding."
            }
            Self::DeclarationFinding => {
                "Declare a fictional type with a reserved status field and inspect the declaration findings during a preview."
            }
            Self::InvalidBundle => {
                "Create a fictional note without a required title. Validate the bundle and inspect the outcome."
            }
            Self::ThresholdReached => {
                "Create a fictional note with a dangling link. Run lint with a threshold that includes its severity."
            }
            Self::ScopedReferrers => {
                "Create two fictional bundles. Link from a note in one to a note in the other, then attempt a move with both in scope."
            }
            Self::BaselineConflict => {
                "Plan a change in a fictional bundle, edit one input, then apply using the earlier baseline."
            }
            Self::WriteGateRejected => {
                "Enable a write gate for a fictional bundle and preview a change that introduces an invalid field."
            }
            Self::PolicyRefusal => {
                "Describe a fictional operation, the relevant policy, and the point where the operation stops."
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Family {
    Lint,
    Validate,
    Move,
    ChangesetApply,
    OntologyApply,
}

impl Family {
    pub fn for_command(command: &Command) -> Option<Self> {
        match command {
            Command::Lint(a)
                if !a.fix
                    && a.baseline.is_none()
                    && a.baseline_ref.is_none()
                    && a.write_baseline.is_none() =>
            {
                Some(Self::Lint)
            }
            Command::Validate(_) => Some(Self::Validate),
            Command::Mv(_) => Some(Self::Move),
            Command::Changeset(ChangeSetCmd::Apply(_)) => Some(Self::ChangesetApply),
            Command::Ontology(crate::cli::OntologyCmd::Apply(_)) => Some(Self::OntologyApply),
            _ => None,
        }
    }
}

#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Counts {
    invocations: u64,
    affected: BTreeMap<Pattern, u64>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct State {
    schema_version: u8,
    enabled: bool,
    versions: BTreeMap<String, BTreeMap<Family, Counts>>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            schema_version: 1,
            enabled: false,
            versions: BTreeMap::new(),
        }
    }
}

fn location() -> Result<PathBuf> {
    let env_path = |key| {
        std::env::var_os(key)
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
    };
    let base = if cfg!(target_os = "macos") {
        env_path("HOME").map(|p| p.join("Library/Application Support"))
    } else if cfg!(windows) {
        env_path("LOCALAPPDATA")
    } else {
        env_path("XDG_STATE_HOME").or_else(|| env_path("HOME").map(|p| p.join(".local/state")))
    };
    base.filter(|p| p.to_str().is_some())
        .map(|p| p.join("okf/feedback"))
        .ok_or_else(|| OkfError::io("no absolute user state directory"))
}

fn private_dir(path: &Path) -> Result<()> {
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.recursive(true).create(path)?;
    let meta = fs::symlink_metadata(path)?;
    if !meta.is_dir() {
        return Err(OkfError::io("feedback directory must be a directory"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if meta.permissions().mode() & 0o077 != 0 {
            return Err(OkfError::io("feedback directory must be private"));
        }
    }
    Ok(())
}

fn load(root: &Path) -> Result<State> {
    let bytes = match fs::read(root.join("state.json")) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(State::default()),
        Err(e) => return Err(e.into()),
    };
    let state: State =
        serde_json::from_slice(&bytes).map_err(|_| OkfError::io("invalid feedback state"))?;
    if state.schema_version != 1 {
        return Err(OkfError::io("unsupported feedback state version"));
    }
    Ok(state)
}

fn save(root: &Path, state: &State) -> Result<()> {
    let mut tmp = tempfile::Builder::new()
        .prefix("pending-state-")
        .tempfile_in(root)?;
    serde_json::to_writer(&mut tmp, state).map_err(|e| OkfError::io(e.to_string()))?;
    tmp.as_file().sync_all()?;
    tmp.persist(root.join("state.json")).map_err(|e| e.error)?;
    Ok(())
}

thread_local! {
    static SIGNALS: std::cell::RefCell<BTreeSet<Pattern>> = const {std::cell::RefCell::new(BTreeSet::new())};
}

pub fn signal(pattern: Pattern) {
    SIGNALS.with_borrow_mut(|s| {
        s.insert(pattern);
    });
}

pub fn observe(family: Family, result: &Result<i32>) {
    let pattern = match result {
        Ok(0) => None,
        Ok(1) if matches!(family, Family::Validate) => Some(Pattern::InvalidBundle),
        Ok(1) if matches!(family, Family::Lint) => Some(Pattern::ThresholdReached),
        Err(e) if e.reason() == Some(FailureReason::BaselineConflict) => {
            Some(Pattern::BaselineConflict)
        }
        Err(e) if e.reason() == Some(FailureReason::WriteGateRejected) => {
            Some(Pattern::WriteGateRejected)
        }
        Err(OkfError::Policy(_)) => Some(Pattern::PolicyRefusal),
        _ => return,
    };
    let _ = (|| -> Result<()> {
        let root = location()?;
        let lock = File::options()
            .read(true)
            .write(true)
            .open(root.join("lock"))?;
        lock.try_lock()
            .map_err(|_| OkfError::io("feedback store busy"))?;
        let mut state = load(&root)?;
        if state.enabled {
            let c = state
                .versions
                .entry(env!("CARGO_PKG_VERSION").into())
                .or_default()
                .entry(family)
                .or_default();
            c.invocations = c.invocations.saturating_add(1);
            let mut signals = SIGNALS.take();
            signals.extend(pattern);
            for p in signals {
                let n = c.affected.entry(p).or_default();
                *n = n.saturating_add(1);
            }
            save(&root, &state)?;
        }
        Ok(())
    })();
}

fn managed_name(name: &str) -> bool {
    name.strip_prefix("draft-")
        .and_then(|n| n.strip_suffix(".md"))
        .is_some_and(|n| n.len() == 6 && n.bytes().all(|b| b.is_ascii_alphanumeric()))
}

fn inventory(root: &Path) -> Result<Vec<PathBuf>> {
    let dir = root.join("drafts");
    if !dir.try_exists()? {
        return Ok(Vec::new());
    }
    private_dir(&dir)?;
    let mut paths = Vec::new();
    for item in fs::read_dir(dir)? {
        let item = item?;
        if item.file_name().to_str().is_some_and(managed_name) {
            paths.push(item.path());
        }
    }
    Ok(paths)
}

fn execute(cmd: &LearnCmd, root: &Path) -> Result<Value> {
    // A missing store is a complete snapshot. Do not look again without a lock:
    // a concurrent enable or draft may create the store immediately afterwards.
    if !root.try_exists()? {
        match cmd {
            LearnCmd::Enable | LearnCmd::Draft { .. } => {}
            LearnCmd::Disable | LearnCmd::Purge => return Ok(json!({"enabled": false})),
            LearnCmd::Status => {
                return Ok(json!({"directory": root, "enabled": false, "drafts": 0, "bytes": 0}));
            }
            LearnCmd::Report => return Ok(json!({"enabled": false, "patterns": []})),
            LearnCmd::Export { .. } => return Err(OkfError::usage("no managed drafts")),
        }
    }
    private_dir(root)?;
    let lock = File::options()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(root.join("lock"))?;
    lock.lock()?;
    if matches!(cmd, LearnCmd::Purge) {
        let state = root.join("state.json");
        if state.try_exists()? {
            fs::remove_file(state)?;
        }
        for path in inventory(root)? {
            fs::remove_file(path)?;
        }
        if root.try_exists()? {
            for entry in fs::read_dir(root)? {
                let entry = entry?;
                if entry
                    .file_name()
                    .to_str()
                    .is_some_and(|name| name.starts_with("pending-state-"))
                    && entry.file_type()?.is_file()
                {
                    fs::remove_file(entry.path())?;
                }
            }
        }
        return Ok(json!({"enabled":false}));
    }
    match cmd {
        LearnCmd::Enable | LearnCmd::Disable => {
            let mut state = load(root)?;
            let enabled = matches!(cmd, LearnCmd::Enable);
            if state.enabled != enabled {
                state.enabled = enabled;
                save(root, &state)?;
            }
            Ok(json!({"enabled":enabled}))
        }
        LearnCmd::Status => {
            let files = inventory(root)?;
            let mut bytes = 0;
            for path in files.iter().cloned().chain([root.join("state.json")]) {
                if path.try_exists()? {
                    bytes += fs::symlink_metadata(path)?.len();
                }
            }
            Ok(
                json!({"directory":root,"enabled":load(root)?.enabled,"drafts":files.len(),"bytes":bytes}),
            )
        }
        LearnCmd::Report => {
            let state = load(root)?;
            let mut rows = Vec::new();
            for (version, families) in state.versions {
                for (family, counts) in families {
                    for (pattern, n) in counts.affected {
                        rows.push((n,json!({"version":version,"command":family,"pattern":pattern,"affected":n,"invocations":counts.invocations})));
                    }
                }
            }
            rows.sort_by_key(|r| std::cmp::Reverse(r.0));
            Ok(
                json!({"enabled":state.enabled,"patterns":rows.into_iter().map(|r|r.1).collect::<Vec<_>>()}),
            )
        }
        LearnCmd::Draft { kind, pattern } => {
            let dir = root.join("drafts");
            private_dir(&dir)?;
            let mut file = tempfile::Builder::new()
                .prefix("draft-")
                .suffix(".md")
                .tempfile_in(&dir)?;
            let kind = kind.to_possible_value().unwrap();
            let mut body = format!("# Feedback ({})\n\n", kind.get_name());
            if let Some(p) = pattern {
                body.push_str("Pattern: ");
                body.push_str(p.to_possible_value().unwrap().get_name());
                body.push_str(
                    "\n\nSynthetic starting point (verify before claiming reproduction): ",
                );
                body.push_str(p.synthetic_example());
                body.push_str("\n\n");
            }
            body.push_str(include_str!("../templates/feedback.md"));
            file.write_all(body.as_bytes())?;
            let (_, path) = file.keep().map_err(|e| e.error)?;
            Ok(json!({"file":path,"text":body}))
        }
        LearnCmd::Export {
            draft,
            output,
            confirm,
        } => {
            if !managed_name(draft) {
                return Err(OkfError::usage("expected a managed draft filename"));
            }
            let dir = root.join("drafts");
            if !dir.try_exists()? {
                return Err(OkfError::usage("no managed drafts"));
            }
            private_dir(&dir)?;
            let source = dir.join(draft);
            if !fs::symlink_metadata(&source)?.is_file() {
                return Err(OkfError::usage("expected a regular draft file"));
            }
            let body = fs::read_to_string(source)?;
            let abs = std::path::absolute(output)?;
            let parent = abs
                .parent()
                .ok_or_else(|| OkfError::usage("expected a destination filename"))?
                .canonicalize()?;
            let dest = parent.join(
                abs.file_name()
                    .ok_or_else(|| OkfError::usage("expected a destination filename"))?,
            );
            if dest.to_str().is_none() {
                return Err(OkfError::usage(
                    "export destination must be representable in the preview as UTF-8",
                ));
            }
            if dest.starts_with(root.canonicalize()?) || dest.try_exists()? {
                return Err(OkfError::usage(
                    "choose a new file outside the managed feedback store",
                ));
            }
            let mut bytes = dest.as_os_str().as_encoded_bytes().to_vec();
            bytes.push(0);
            bytes.extend(body.as_bytes());
            let digest = sha256_hex(&bytes);
            if let Some(expected) = confirm {
                if expected != &digest {
                    return Err(OkfError::usage(
                        "preview changed; review the draft and destination again",
                    ));
                }
                let mut tmp = NamedTempFile::new_in(parent)?;
                tmp.write_all(body.as_bytes())?;
                tmp.persist_noclobber(&dest).map_err(|e| e.error)?;
                Ok(json!({"exported":dest}))
            } else {
                Ok(json!({"destination":dest,"text":body,"confirmation":digest}))
            }
        }
        LearnCmd::Purge => unreachable!(),
    }
}

pub fn run(cmd: &LearnCmd, json_output: bool) -> Result<i32> {
    if matches!(cmd, LearnCmd::Enable) {
        if !std::io::stdin().is_terminal() || !std::io::stderr().is_terminal() {
            return Err(OkfError::usage("enable requires interactive consent"));
        }
        eprint!("Count fixed command outcomes locally? Type enable to consent: ");
        std::io::stderr().flush()?;
        let mut answer = String::new();
        std::io::stdin().read_line(&mut answer)?;
        if answer.trim() != "enable" {
            return Err(OkfError::usage("consent was not given"));
        }
    }
    let data = execute(cmd, &location()?)?;
    if json_output {
        crate::output::print_line(&json!({"kind":"feedback","action":cmd.name(),"data":data}))?;
    } else if matches!(cmd, LearnCmd::Report) {
        let rows = data["patterns"].as_array().expect("ranked report rows");
        if rows.is_empty() {
            crate::output::print_text_line(format_args!("No outcomes recorded."))?;
        }
        for row in rows {
            crate::output::print_text_line(format_args!(
                "{}  {}  {}  {} of {} qualifying invocations",
                row["version"].as_str().unwrap(),
                row["command"].as_str().unwrap(),
                row["pattern"].as_str().unwrap(),
                row["affected"],
                row["invocations"]
            ))?;
        }
    } else {
        for (key, value) in data.as_object().expect("feedback object") {
            if let Some(s) = value.as_str() {
                crate::output::print_text_line(format_args!("{key}: {s}"))?;
            } else {
                crate::output::print_text_line(format_args!("{key}: {value:#}"))?;
            }
        }
    }
    Ok(0)
}
