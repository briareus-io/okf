//! Explicit Git dependency acquisition and immutable local snapshots.
//!
//! Resolution/checks never fetch. Only `plan_update` and `sync(check=false)` acquire objects.
use crate::error::{OkfError, Result};
use crate::fingerprint::canonicalize::sha256_hex;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

pub const LOCK_FILENAME: &str = "okf-catalog.lock";
const PROFILE: u32 = 1;
const META: &str = "snapshot.yaml";
static SERIAL: AtomicU64 = AtomicU64::new(0);

fn usage(message: impl Into<String>) -> OkfError {
    OkfError::Usage(message.into())
}
fn environment(message: impl Into<String>) -> OkfError {
    OkfError::Environment(message.into())
}
fn yaml<T: serde::de::DeserializeOwned>(bytes: &[u8], name: &str) -> Result<T> {
    serde_yaml::from_slice(bytes).map_err(|e| usage(format!("invalid {name}: {e}")))
}
fn encoded<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    serde_yaml::to_string(value)
        .map(String::into_bytes)
        .map_err(|e| usage(e.to_string()))
}
fn digest<T: Serialize>(value: &T) -> Result<String> {
    Ok(sha256_hex(
        &serde_json::to_vec(value).map_err(|e| usage(e.to_string()))?,
    ))
}
fn read_digest(path: &Path) -> Result<Option<String>> {
    match std::fs::read(path) {
        Ok(b) => Ok(Some(sha256_hex(&b))),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Accepts {
    #[serde(default)]
    pub okf_spec: Vec<String>,
    #[serde(default)]
    pub manifest_versions: Vec<u32>,
    #[serde(default)]
    pub manifest_required: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GitRegistration {
    pub url: String,
    pub reference: String,
    pub path: String,
    pub access: String,
    pub accepts: Accepts,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LockedRepository {
    pub url: String,
    pub commit: String,
    pub object_format: String,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Observed {
    pub okf_spec: Option<String>,
    pub manifest_version: Option<u32>,
    pub manifest_digest: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LockedBundle {
    pub repository: String,
    pub path: String,
    pub observed: Observed,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LockFile {
    pub lock_version: u32,
    pub declaration_digest: String,
    pub repositories: BTreeMap<String, LockedRepository>,
    pub bundles: BTreeMap<String, LockedBundle>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteOrigin {
    pub url: String,
    pub commit: Option<String>,
    pub bundle_path: String,
    pub access: String,
    pub state: String,
}
#[derive(Debug, Clone, Serialize)]
pub struct RemoteResolution {
    pub root: PathBuf,
    pub available: bool,
    pub origin: RemoteOrigin,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Member {
    mode: String,
    object: String,
    sha256: Option<String>,
    status: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Snapshot {
    profile: u32,
    url: String,
    commit: String,
    object_format: String,
    members: BTreeMap<String, Member>,
}
#[derive(Debug, Clone, Serialize)]
pub struct SyncReport {
    pub check: bool,
    pub bundles: BTreeMap<String, RemoteResolution>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepositoryChange {
    pub fast_forward: Option<bool>,
    pub url: String,
    pub old_url: Option<String>,
    pub old_commit: Option<String>,
    pub new_commit: String,
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub changed: Vec<String>,
    pub commits: Vec<String>,
    pub history_complete: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConsumerEvidence {
    pub root: PathBuf,
    pub digest: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdatePlan {
    pub created_at_unix: u64,
    pub plan_version: u32,
    pub kind: String,
    pub declaration_digest: String,
    pub old_lock_digest: Option<String>,
    pub selected_bundles: Vec<String>,
    pub candidate: LockFile,
    pub changes: Vec<RepositoryChange>,
    pub consumers: Vec<ConsumerEvidence>,
    pub impact: super::remote_impact::ImpactReport,
    pub impact_complete: bool,
    pub impact_reasons: Vec<String>,
    pub plan_digest: String,
}
#[derive(Debug, Clone, Serialize)]
pub struct ApplyReport {
    pub accepted_at_unix: u64,
    pub impact_reasons: Vec<String>,
    pub lock_path: PathBuf,
    pub plan_digest: String,
    pub accepted: bool,
    pub incomplete_impact_accepted: bool,
    pub report_path: PathBuf,
}

#[cfg(test)]
thread_local! {static TEST_CACHE:std::cell::RefCell<Option<PathBuf>>=const{std::cell::RefCell::new(None)};}
fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}
pub fn cache_root() -> Result<PathBuf> {
    #[cfg(test)]
    if let Some(root) = TEST_CACHE.with(|p| p.borrow().clone()) {
        return Ok(root);
    }
    if let Some(p) = std::env::var_os("OKF_CACHE_DIR") {
        return super::catalog::absolute(Path::new(&p));
    }
    let base = if let Some(p) = std::env::var_os("XDG_CACHE_HOME") {
        PathBuf::from(p)
    } else if let Some(p) = std::env::var_os("HOME") {
        PathBuf::from(p).join(".cache")
    } else {
        return Err(environment(
            "set OKF_CACHE_DIR: no user cache directory available",
        ));
    };
    Ok(base.join("okf"))
}
fn repo_key(url: &str) -> String {
    sha256_hex(url.as_bytes())
}
fn object_store(url: &str) -> Result<PathBuf> {
    Ok(cache_root()?.join("objects").join(repo_key(url)))
}
fn snapshot_dir(repo: &LockedRepository) -> Result<PathBuf> {
    Ok(cache_root()?
        .join("snapshots")
        .join(repo_key(&repo.url))
        .join(format!("{}-{}-p{PROFILE}", repo.object_format, repo.commit)))
}
pub fn lock_path(catalog: &Path) -> PathBuf {
    catalog.with_file_name(LOCK_FILENAME)
}
fn safe_relative(raw: &str) -> Result<PathBuf> {
    let p = Path::new(raw);
    if raw.is_empty()
        || raw.contains(['\\', '\0', '\n', '\r'])
        || p.is_absolute()
        || p.components()
            .any(|c| !matches!(c, Component::Normal(_) | Component::CurDir))
    {
        return Err(usage(format!("unsafe repository-relative path {raw:?}")));
    }
    if p.components().any(
        |c| matches!(c,Component::Normal(n) if n.to_string_lossy().eq_ignore_ascii_case(".git")),
    ) {
        return Err(usage("repository paths may not contain .git"));
    }
    Ok(p.components()
        .filter_map(|c| match c {
            Component::Normal(name) => Some(name),
            _ => None,
        })
        .collect())
}
fn validate_url(url: &str) -> Result<()> {
    #[cfg(test)]
    if Path::new(url).is_absolute() {
        return Ok(());
    }
    if url.contains(['\n', '\r', '\0']) {
        return Err(usage("invalid Git URL"));
    }
    if let Some(rest) = url.strip_prefix("https://") {
        if rest.split('/').next().unwrap_or("").contains('@') || rest.is_empty() {
            return Err(usage("Git HTTPS URLs must not contain credentials"));
        }
    } else if let Some(rest) = url.strip_prefix("ssh://") {
        if rest.is_empty()
            || rest
                .split('/')
                .next()
                .unwrap_or("")
                .split('@')
                .next()
                .unwrap_or("")
                .contains(':')
        {
            return Err(usage("invalid SSH URL or embedded credential"));
        }
    } else {
        return Err(usage("Git registrations require https:// or ssh:// URLs; use directory registrations for local paths"));
    }
    Ok(())
}
fn full_oid(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && value.bytes().all(|b| b.is_ascii_hexdigit())
}
fn validate_ref(reference: &str) -> Result<()> {
    if full_oid(reference) {
        return Ok(());
    }
    if !(reference.starts_with("refs/heads/") || reference.starts_with("refs/tags/"))
        || reference.contains(['\0', '\n', '\r', ':', ' ', '~', '^', '?', '*', '[', '\\'])
        || reference.contains("..")
        || reference.contains("@{")
        || reference.ends_with('/')
    {
        return Err(usage(
            "remote ref must be a full commit ID, refs/heads/<name>, or refs/tags/<name>",
        ));
    }
    Ok(())
}
/// Parse only catalog-owned remote declarations. No catalog recursion or acquisition.
pub fn registrations(catalog: &Path) -> Result<BTreeMap<String, GitRegistration>> {
    let value: serde_yaml::Value = yaml(&std::fs::read(catalog)?, "catalog")?;
    let version = value
        .get("catalog_version")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| usage("catalog_version is required"))?;
    if !matches!(version, 1 | 2) {
        return Err(usage("unsupported catalog_version"));
    }
    let bundles = value
        .get("bundles")
        .and_then(|v| v.as_mapping())
        .ok_or_else(|| usage("catalog bundles must be a mapping"))?;
    let mut result = BTreeMap::new();
    let mut refs = BTreeMap::new();
    for (id, entry) in bundles {
        let id = id
            .as_str()
            .ok_or_else(|| usage("catalog IDs must be strings"))?;
        if !super::catalog::valid_id(id) {
            return Err(usage(format!("invalid bundle ID {id:?}")));
        }
        let location = entry
            .get("location")
            .ok_or_else(|| usage("registration requires location"))?;
        if location.get("type").and_then(|v| v.as_str()) != Some("git") {
            continue;
        }
        if version < 2 {
            return Err(usage("Git registration requires catalog_version 2"));
        }
        let field = |name: &str| {
            location
                .get(name)
                .and_then(|v| v.as_str())
                .map(str::to_owned)
                .ok_or_else(|| usage(format!("Git registration {id} requires location.{name}")))
        };
        let url = field("url")?;
        validate_url(&url)?;
        let reference = field("ref")?;
        validate_ref(&reference)?;
        let path = field("path")?;
        safe_relative(&path)?;
        let access = entry
            .get("access")
            .and_then(|v| v.as_str())
            .unwrap_or("read-only")
            .to_string();
        if access != "read-only" {
            return Err(usage("Git registrations only support read-only access"));
        }
        let accepts = entry
            .get("accepts")
            .map(|v| {
                serde_yaml::from_value(v.clone())
                    .map_err(|e| usage(format!("invalid accepts: {e}")))
            })
            .transpose()?
            .unwrap_or_default();
        if refs
            .insert(url.clone(), reference.clone())
            .is_some_and(|old| old != reference)
        {
            return Err(usage(
                "registrations for one repository must use the same ref",
            ));
        }
        result.insert(
            id.to_string(),
            GitRegistration {
                url,
                reference,
                path,
                access,
                accepts,
            },
        );
    }
    let items: Vec<_> = result.iter().collect();
    for (i, (id, a)) in items.iter().enumerate() {
        for (other, b) in &items[i + 1..] {
            if a.url == b.url
                && (safe_relative(&a.path)?.starts_with(safe_relative(&b.path)?)
                    || safe_relative(&b.path)?.starts_with(safe_relative(&a.path)?))
            {
                return Err(usage(format!(
                    "equal or nested remote roots: {id}, {other}"
                )));
            }
        }
    }
    Ok(result)
}
fn checked_lock(
    catalog: &Path,
    declarations: &BTreeMap<String, GitRegistration>,
    required: bool,
) -> Result<Option<LockFile>> {
    let path = lock_path(catalog);
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound && !required => return Ok(None),
        Err(e) => {
            return Err(environment(format!(
                "cannot read {}: {e}; plan and apply the initial dependency lock",
                path.display()
            )))
        }
    };
    let lock: LockFile = yaml(&bytes, "catalog lock")?;
    validate_lock(&lock, declarations)?;
    Ok(Some(lock))
}
fn validate_lock(lock: &LockFile, declarations: &BTreeMap<String, GitRegistration>) -> Result<()> {
    if lock.lock_version != 1 {
        return Err(usage("unsupported lock_version"));
    }
    if lock.declaration_digest != digest(declarations)? {
        return Err(usage(
            "catalog dependency declarations differ from lock; create a registration-change plan",
        ));
    }
    if lock.bundles.len() != declarations.len() {
        return Err(usage("lock registration set differs from catalog"));
    }
    let mut repo_commits = BTreeMap::new();
    for repo in lock.repositories.values() {
        if repo_commits
            .insert(&repo.url, &repo.commit)
            .is_some_and(|old| old != &repo.commit)
        {
            return Err(usage("one repository has multiple locked revisions"));
        }
    }
    for (id, decl) in declarations {
        let b = lock
            .bundles
            .get(id)
            .ok_or_else(|| usage(format!("missing locked bundle {id}")))?;
        let r = lock
            .repositories
            .get(&b.repository)
            .ok_or_else(|| usage("missing locked repository"))?;
        if r.url != decl.url
            || b.path != decl.path
            || !full_oid(&r.commit)
            || !matches!(r.object_format.as_str(), "sha1" | "sha256")
            || (r.object_format == "sha1" && r.commit.len() != 40)
            || (r.object_format == "sha256" && r.commit.len() != 64)
        {
            return Err(usage(format!("invalid locked identity for {id}")));
        }
    }
    Ok(())
}
fn git_command(root: &Path) -> Command {
    let mut c = Command::new("git");
    c.arg("-C")
        .arg(root)
        .args([
            "-c",
            "core.hooksPath=/dev/null",
            "-c",
            "core.fsmonitor=false",
            "-c",
            "protocol.ext.allow=never",
            "-c",
            "fetch.recurseSubmodules=false",
        ])
        .env("GIT_NO_LAZY_FETCH", "1")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE");
    #[cfg(not(test))]
    c.args(["-c", "protocol.file.allow=never"]);
    c
}
fn git(root: &Path, args: &[&str]) -> Result<Vec<u8>> {
    let out = git_command(root)
        .args(args)
        .output()
        .map_err(|e| environment(format!("cannot run Git: {e}")))?;
    if !out.status.success() {
        return Err(environment(format!(
            "Git {} failed: {}",
            args.first().unwrap_or(&"operation"),
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    Ok(out.stdout)
}
fn git_text(root: &Path, args: &[&str]) -> Result<String> {
    Ok(String::from_utf8(git(root, args)?)
        .map_err(|_| environment("Git returned non-UTF8 metadata"))?
        .trim()
        .to_string())
}
fn private_dir(path: &Path) -> Result<()> {
    // Local managed state must never be placed inside an existing immutable snapshot.
    let abs = canonical_existing(path)?;
    for ancestor in abs.ancestors() {
        if ancestor.join(META).is_file() && abs.starts_with(ancestor.join("tree")) {
            return Err(OkfError::Policy(
                "cache destination is inside a read-only snapshot".into(),
            ));
        }
    }
    std::fs::create_dir_all(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}
struct Guard(std::fs::File);
impl Drop for Guard {
    fn drop(&mut self) {
        let _ = self.0.unlock();
    }
}
fn guard(path: &Path) -> Result<Guard> {
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p)?;
    }
    if let Ok(meta) = std::fs::symlink_metadata(path) {
        if !meta.is_file() || meta.file_type().is_symlink() {
            return Err(environment(format!(
                "lock must be a regular file: {}",
                path.display()
            )));
        }
    }
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .map_err(|e| environment(format!("cannot open lock {}: {e}", path.display())))?;
    file.try_lock().map_err(|e| {
        environment(format!(
            "cannot acquire {}: {e}; another operation may be running",
            path.display()
        ))
    })?;
    // Keep the inode stable: unlinking after unlock could create two independent locks.
    // The OS releases the advisory lock on process exit, including interruption.
    Ok(Guard(file))
}
fn ensure_store(url: &str, object_format: &str) -> Result<PathBuf> {
    let store = object_store(url)?;
    let _init_guard = guard(&store.with_extension("init.lock"))?;
    if !store.join("HEAD").is_file() {
        private_dir(&store)?;
        git(
            &store,
            &[
                "init",
                "--bare",
                "--quiet",
                &format!("--object-format={object_format}"),
            ],
        )?;
    }
    Ok(store)
}
fn fetch(url: &str, reference: &str) -> Result<LockedRepository> {
    validate_url(url)?;
    validate_ref(reference)?;
    let object_format = if full_oid(reference) {
        if reference.len() == 64 {
            "sha256"
        } else {
            "sha1"
        }
    } else {
        let output = git_text(
            Path::new("."),
            &["ls-remote", "--exit-code", "--", url, reference],
        )
        .map_err(|_| environment("remote ref lookup failed (access-or-not-found/transport)"))?;
        match output.split_whitespace().next().map(str::len) {
            Some(64) => "sha256",
            Some(40) => "sha1",
            _ => return Err(environment("unsupported remote Git object format")),
        }
    };
    let store = ensure_store(url, object_format)?;
    let _guard = guard(&store.with_extension("fetch.lock"))?;
    git(&store,&["fetch","--no-tags","--no-recurse-submodules","--",url,reference]).map_err(|_|environment("remote fetch failed (access-or-not-found/transport); verify credentials and the requested ref or locked commit"))?;
    let commit = git_text(&store, &["rev-parse", "--verify", "FETCH_HEAD^{commit}"])?;
    if full_oid(reference) && !commit.eq_ignore_ascii_case(reference) {
        return Err(environment("fetched commit differs from locked commit"));
    }
    let object_format = git_text(&store, &["rev-parse", "--show-object-format"])?;
    Ok(LockedRepository {
        url: url.to_owned(),
        commit,
        object_format,
    })
}
struct BlobReader {
    child: std::process::Child,
    input: std::process::ChildStdin,
    output: std::io::BufReader<std::process::ChildStdout>,
}
impl BlobReader {
    fn new(store: &Path) -> Result<Self> {
        let mut child = git_command(store)
            .args(["cat-file", "--batch"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()?;
        Ok(Self {
            input: child.stdin.take().unwrap(),
            output: std::io::BufReader::new(child.stdout.take().unwrap()),
            child,
        })
    }
    fn blob(&mut self, oid: &str) -> Result<Vec<u8>> {
        use std::io::{BufRead, Read, Write};
        if !full_oid(oid) {
            return Err(environment("invalid object ID in Git tree"));
        }
        writeln!(self.input, "{oid}")?;
        self.input.flush()?;
        let mut header = String::new();
        self.output.read_line(&mut header)?;
        let fields: Vec<_> = header.split_whitespace().collect();
        if fields.len() != 3 || fields[0] != oid || fields[1] != "blob" {
            return Err(environment("required Git blob is unavailable"));
        }
        let size = fields[2]
            .parse::<usize>()
            .map_err(|_| environment("invalid Git blob size"))?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(size)
            .map_err(|_| environment("Git blob is too large to materialize"))?;
        bytes.resize(size, 0);
        self.output.read_exact(&mut bytes)?;
        let mut newline = [0];
        self.output.read_exact(&mut newline)?;
        if newline[0] != b'\n' {
            return Err(environment("invalid Git blob response framing"));
        }
        Ok(bytes)
    }
}
impl Drop for BlobReader {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
fn tree(repo: &LockedRepository) -> Result<BTreeMap<String, Member>> {
    let store = object_store(&repo.url)?;
    let bytes = git(
        &store,
        &["ls-tree", "-r", "-z", "--full-tree", &repo.commit],
    )?;
    let mut result = BTreeMap::new();
    let mut reader = BlobReader::new(&store)?;
    for entry in bytes.split(|b| *b == 0).filter(|e| !e.is_empty()) {
        let tab = entry
            .iter()
            .position(|b| *b == b'\t')
            .ok_or_else(|| environment("invalid Git tree record"))?;
        let header = std::str::from_utf8(&entry[..tab])
            .map_err(|_| environment("invalid Git tree header"))?;
        let mut pieces = header.split(' ');
        let mode = pieces.next().unwrap_or("");
        let _kind = pieces.next();
        let oid = pieces.next().unwrap_or("");
        let path = std::str::from_utf8(&entry[tab + 1..])
            .map_err(|_| environment("non-UTF8 repository paths are unsupported"))?;
        safe_relative(path)?;
        let mut m = Member {
            mode: mode.to_owned(),
            object: oid.to_owned(),
            sha256: None,
            status: "unsupported-entry".into(),
        };
        if matches!(mode, "100644" | "100755") {
            let blob = reader.blob(oid)?;
            if blob.starts_with(b"version https://git-lfs.github.com/spec/v1\n") {
                m.status = "unsupported-lfs".into();
            } else {
                m.sha256 = Some(sha256_hex(&blob));
                m.status = "materialized".into();
            }
        }
        result.insert(path.to_owned(), m);
    }
    Ok(result)
}
fn materialize(repo: &LockedRepository) -> Result<PathBuf> {
    let dest = snapshot_dir(repo)?;
    let _guard = guard(&dest.with_extension("publish.lock"))?;
    if dest.exists() {
        verify_snapshot(repo)?;
        return Ok(dest.join("tree"));
    }
    let members = tree(repo)?;
    let stage = dest.with_extension(format!(
        "tmp-{}-{}",
        std::process::id(),
        SERIAL.fetch_add(1, Ordering::Relaxed)
    ));
    private_dir(&stage.join("tree"))?;
    let result = (|| {
        let store = object_store(&repo.url)?;
        let mut reader = BlobReader::new(&store)?;
        for (name, member) in &members {
            if member.status != "materialized" {
                continue;
            }
            let p = stage.join("tree").join(safe_relative(name)?);
            if let Some(parent) = p.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let bytes = reader.blob(&member.object)?;
            if Some(sha256_hex(&bytes)) != member.sha256 {
                return Err(environment("Git blob integrity mismatch"));
            }
            write_new(&p, &bytes)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(
                    &p,
                    std::fs::Permissions::from_mode(if member.mode == "100755" {
                        0o500
                    } else {
                        0o400
                    }),
                )?;
            }
        }
        let metadata = Snapshot {
            profile: PROFILE,
            url: repo.url.clone(),
            commit: repo.commit.clone(),
            object_format: repo.object_format.clone(),
            members,
        };
        std::fs::write(stage.join(META), encoded(&metadata)?)?;
        std::fs::rename(&stage, &dest)?;
        Ok(dest.join("tree"))
    })();
    if result.is_err() {
        let _ = std::fs::remove_dir_all(&stage);
    }
    result
}
fn filesystem_members(root: &Path) -> Result<BTreeSet<String>> {
    fn walk(base: &Path, path: &Path, out: &mut BTreeSet<String>) -> Result<()> {
        for e in std::fs::read_dir(path)? {
            let e = e?;
            let p = e.path();
            let ty = e.file_type()?;
            if ty.is_symlink() {
                return Err(environment(format!(
                    "unexpected symlink in snapshot: {}",
                    p.display()
                )));
            }
            if ty.is_dir() {
                walk(base, &p, out)?;
            } else if ty.is_file() {
                out.insert(p.strip_prefix(base).unwrap().to_string_lossy().to_string());
            } else {
                return Err(environment("unsupported filesystem entry in snapshot"));
            }
        }
        Ok(())
    }
    let mut out = BTreeSet::new();
    walk(root, root, &mut out)?;
    Ok(out)
}
fn verify_snapshot(repo: &LockedRepository) -> Result<Snapshot> {
    let dir = snapshot_dir(repo)?;
    let snap: Snapshot = yaml(
        &std::fs::read(dir.join(META))
            .map_err(|e| environment(format!("snapshot unavailable: {e}; run catalog sync")))?,
        "snapshot metadata",
    )?;
    if snap.profile != PROFILE
        || snap.url != repo.url
        || snap.commit != repo.commit
        || snap.object_format != repo.object_format
    {
        return Err(environment("snapshot identity mismatch"));
    }
    let expected = tree(repo)?;
    if encoded(&snap.members)? != encoded(&expected)? {
        return Err(environment(
            "snapshot membership metadata differs from Git objects",
        ));
    }
    let actual = filesystem_members(&dir.join("tree"))?;
    let names: BTreeSet<_> = snap
        .members
        .iter()
        .filter(|(_, m)| m.status == "materialized")
        .map(|(p, _)| p.clone())
        .collect();
    if actual != names {
        return Err(environment(
            "snapshot has missing or extra files; explicit recovery required",
        ));
    }
    for (name, m) in &snap.members {
        if m.status != "materialized" {
            continue;
        }
        let path = dir.join("tree").join(name);
        if Some(sha256_hex(&std::fs::read(&path)?)) != m.sha256 {
            return Err(environment(format!(
                "snapshot content changed: {name}; explicit recovery required"
            )));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let executable = std::fs::metadata(&path)?.permissions().mode() & 0o111 != 0;
            if executable != (m.mode == "100755") {
                return Err(environment(format!("snapshot mode changed: {name}")));
            }
        }
    }
    Ok(snap)
}
fn observe(root: &Path, decl: &GitRegistration, snapshot: &Snapshot) -> Result<Observed> {
    let path = safe_relative(&decl.path)?;
    if snapshot
        .members
        .iter()
        .any(|(p, m)| Path::new(p).starts_with(&path) && m.status != "materialized")
    {
        return Err(usage(format!(
            "bundle {} contains unsupported Git entries (symlink, submodule, or LFS)",
            decl.path
        )));
    }
    let root = root.join(path);
    if !root.is_dir() {
        return Err(usage(format!("missing remote bundle path {}", decl.path)));
    }
    let conformance = crate::check::validate::validate_bundle_profile(
        &root,
        crate::check::validate::ValidationProfile::Bundle,
    )?;
    if !conformance.violations.is_empty() {
        return Err(usage(format!(
            "remote bundle {} has {} conformance violation(s)",
            decl.path,
            conformance.violations.len()
        )));
    }
    let loaded_manifest = super::manifest::load(&root)?;
    let ontology = loaded_manifest
        .as_ref()
        .and_then(|m| m.manifest.settings.ontology.as_deref());
    let ontology_path = super::manifest::local_path(&root, ontology.unwrap_or("ontology.yaml"))?;
    if ontology.is_some() || ontology_path.exists() {
        crate::ontology::load::load_ontology(&ontology_path)?;
    }
    let mut observed = Observed::default();
    let index = root.join("index.md");
    if index.is_file() {
        let text = std::fs::read_to_string(index)?;
        if let Some(rest) = text.strip_prefix("---\n") {
            if let Some((front, _)) = rest.split_once("\n---") {
                let v: serde_yaml::Value = yaml(front.as_bytes(), "root index")?;
                observed.okf_spec = v
                    .get("okf_version")
                    .and_then(|v| v.as_str())
                    .map(str::to_owned);
            }
        }
    }
    if observed.okf_spec.as_deref().is_some_and(|v| v != "0.2") {
        return Err(usage("remote bundle declares unsupported OKF spec"));
    }
    if !decl.accepts.okf_spec.is_empty()
        && !observed
            .okf_spec
            .as_ref()
            .is_some_and(|v| decl.accepts.okf_spec.contains(v))
    {
        return Err(usage(
            "remote OKF spec does not meet consumer accepted versions (undeclared is not assumed)",
        ));
    }
    let manifest = root.join("okf-bundle.yaml");
    super::manifest::validate_resources(&root)?;
    if manifest.is_file() {
        let bytes = std::fs::read(&manifest)?;
        let v: serde_yaml::Value = yaml(&bytes, "remote manifest")?;
        let version = v
            .get("manifest_version")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| usage("remote manifest requires manifest_version"))?;
        if version != 1 {
            return Err(usage("unsupported remote manifest version"));
        }
        if !decl.accepts.manifest_versions.is_empty()
            && !decl.accepts.manifest_versions.contains(&(version as u32))
        {
            return Err(usage("remote manifest version rejected by consumer"));
        }
        observed.manifest_version = Some(version as u32);
        observed.manifest_digest = Some(sha256_hex(&bytes));
    } else if decl.accepts.manifest_required {
        return Err(usage("consumer requires a remote manifest"));
    }
    Ok(observed)
}
fn selected(
    declarations: &BTreeMap<String, GitRegistration>,
    id: Option<&str>,
) -> Result<BTreeSet<String>> {
    if let Some(id) = id {
        let decl = declarations
            .get(id)
            .ok_or_else(|| usage(format!("{id} is not a Git registration")))?;
        Ok(declarations
            .iter()
            .filter(|(_, d)| d.url == decl.url)
            .map(|(id, _)| id.clone())
            .collect())
    } else {
        Ok(declarations.keys().cloned().collect())
    }
}
pub fn resolve_registration(catalog: &Path, id: &str) -> Result<RemoteResolution> {
    let declarations = registrations(catalog)?;
    let decl = declarations
        .get(id)
        .ok_or_else(|| usage(format!("unknown Git registration {id}")))?;
    let lock = checked_lock(catalog, &declarations, false)?;
    let mut origin = RemoteOrigin {
        url: decl.url.clone(),
        commit: None,
        bundle_path: decl.path.clone(),
        access: "read-only".into(),
        state: "unlocked".into(),
    };
    let Some(lock) = lock else {
        return Ok(RemoteResolution {
            root: cache_root()?
                .join("unlocked")
                .join(repo_key(&decl.url))
                .join(&decl.path),
            available: false,
            origin,
        });
    };
    let b = &lock.bundles[id];
    let repo = &lock.repositories[&b.repository];
    origin.commit = Some(repo.commit.clone());
    let tree_root = snapshot_dir(repo)?.join("tree");
    let root = tree_root.join(&decl.path);
    if !tree_root.exists() {
        origin.state = "not-synced".into();
        return Ok(RemoteResolution {
            root,
            available: false,
            origin,
        });
    }
    let snapshot = verify_snapshot(repo)?;
    let observed = observe(&tree_root, decl, &snapshot)?;
    if observed != b.observed {
        return Err(environment(
            "remote observed metadata does not match accepted lock",
        ));
    }
    origin.state = "matched".into();
    Ok(RemoteResolution {
        root,
        available: true,
        origin,
    })
}
pub fn sync(catalog: &Path, id: Option<&str>, check: bool) -> Result<SyncReport> {
    let declarations = registrations(catalog)?;
    let lock = checked_lock(catalog, &declarations, true)?.unwrap();
    let ids = selected(&declarations, id)?;
    let mut bundles = BTreeMap::new();
    let mut done = BTreeSet::new();
    for id in ids {
        let b = &lock.bundles[&id];
        let repo = &lock.repositories[&b.repository];
        if done.insert(b.repository.clone()) {
            if !check {
                let store = object_store(&repo.url)?;
                let available = store.exists()
                    && git(
                        &store,
                        &["cat-file", "-e", &format!("{}^{{commit}}", repo.commit)],
                    )
                    .is_ok();
                if !available {
                    let fetched = fetch(&repo.url, &repo.commit)?;
                    if &fetched != repo {
                        return Err(environment("locked Git identity mismatch"));
                    }
                }
                materialize(repo)?;
            } else {
                verify_snapshot(repo)?;
            }
        }
        bundles.insert(id.clone(), resolve_registration(catalog, &id)?);
    }
    Ok(SyncReport { check, bundles })
}
fn consumer_digest(root: &Path) -> Result<String> {
    let mut files = BTreeMap::new();
    for path in super::walk::walk_files(root)? {
        files.insert(
            path.to_string_lossy().to_string(),
            sha256_hex(&std::fs::read(root.join(path))?),
        );
    }
    digest(&files)
}
fn plan_hash(plan: &UpdatePlan) -> Result<String> {
    let mut p = plan.clone();
    p.plan_digest.clear();
    digest(&p)
}
pub fn plan_update(
    catalog: &Path,
    id: Option<&str>,
    consumer_roots: &[PathBuf],
) -> Result<UpdatePlan> {
    let declarations = registrations(catalog)?;
    let ids = selected(&declarations, id)?;
    let old_bytes = match std::fs::read(lock_path(catalog)) {
        Ok(b) => Some(b),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(e.into()),
    };
    let old = old_bytes
        .as_ref()
        .map(|b| yaml::<LockFile>(b, "old catalog lock"))
        .transpose()?;
    if ids.is_empty() && old.as_ref().is_none_or(|l| l.bundles.is_empty()) {
        return Err(usage("catalog has no Git registrations to update"));
    }
    if old.as_ref().is_some_and(|l| l.lock_version != 1) {
        return Err(usage("unsupported old lock version"));
    }
    let declaration_digest = digest(&declarations)?;
    let registration_change = old
        .as_ref()
        .is_some_and(|l| l.declaration_digest != declaration_digest);
    if registration_change && id.is_some() {
        return Err(usage(
            "registration-change plan must include all Git registrations",
        ));
    }
    let mut candidate = old.clone().unwrap_or(LockFile {
        lock_version: 1,
        declaration_digest: declaration_digest.clone(),
        repositories: BTreeMap::new(),
        bundles: BTreeMap::new(),
    });
    candidate.declaration_digest = declaration_digest.clone();
    candidate
        .bundles
        .retain(|id, _| declarations.contains_key(id));
    let mut changes = Vec::new();
    let mut processed = BTreeSet::new();
    for id in &ids {
        let decl = &declarations[id];
        if !processed.insert(decl.url.clone()) {
            continue;
        }
        let new = fetch(&decl.url, &decl.reference)?;
        let tree_root = materialize(&new)?;
        let new_tree = verify_snapshot(&new)?;
        let old_repo = old
            .as_ref()
            .and_then(|l| {
                l.bundles
                    .get(id)
                    .and_then(|b| l.repositories.get(&b.repository))
            })
            .cloned();
        let old_tree = old_repo.as_ref().map(tree).transpose()?;
        let mut added = Vec::new();
        let mut removed = Vec::new();
        let mut changed = Vec::new();
        for (p, m) in &new_tree.members {
            match old_tree.as_ref().and_then(|t| t.get(p)) {
                None => added.push(p.clone()),
                Some(old) if old.object != m.object || old.mode != m.mode => {
                    changed.push(p.clone())
                }
                _ => {}
            }
        }
        if let Some(old) = &old_tree {
            for p in old.keys() {
                if !new_tree.members.contains_key(p) {
                    removed.push(p.clone());
                }
            }
        }
        let (commits, history_complete) = if let Some(old) = &old_repo {
            match git_text(
                &object_store(&new.url)?,
                &["rev-list", &format!("{}..{}", old.commit, new.commit)],
            ) {
                Ok(s) => (s.lines().map(str::to_owned).collect(), true),
                Err(_) => (Vec::new(), false),
            }
        } else {
            (Vec::new(), true)
        };
        for (member, member_decl) in &declarations {
            if member_decl.url == decl.url {
                let observed = observe(&tree_root, member_decl, &new_tree)?;
                candidate.bundles.insert(
                    member.clone(),
                    LockedBundle {
                        repository: repo_key(&decl.url),
                        path: member_decl.path.clone(),
                        observed,
                    },
                );
            }
        }
        candidate
            .repositories
            .insert(repo_key(&decl.url), new.clone());
        let fast_forward = old_repo
            .as_ref()
            .filter(|old| old.url == new.url)
            .and_then(|old| {
                git_command(&object_store(&new.url).ok()?)
                    .args(["merge-base", "--is-ancestor", &old.commit, &new.commit])
                    .status()
                    .ok()
                    .and_then(|status| match status.code() {
                        Some(0) => Some(true),
                        Some(1) => Some(false),
                        _ => None,
                    })
            });
        changes.push(RepositoryChange {
            fast_forward,
            url: new.url,
            old_url: old_repo.as_ref().map(|r| r.url.clone()),
            old_commit: old_repo.map(|r| r.commit),
            new_commit: new.commit,
            added,
            removed,
            changed,
            commits,
            history_complete,
        });
    }
    let used: BTreeSet<_> = candidate
        .bundles
        .values()
        .map(|b| b.repository.clone())
        .collect();
    candidate.repositories.retain(|id, _| used.contains(id));
    validate_lock(&candidate, &declarations)?;
    let consumers = consumer_roots
        .iter()
        .map(|root| {
            let root = std::fs::canonicalize(root)?;
            Ok(ConsumerEvidence {
                digest: consumer_digest(&root)?,
                root,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let canonical_consumers: Vec<_> = consumers.iter().map(|c| c.root.clone()).collect();
    let impact =
        super::remote_impact::report(catalog, old.as_ref(), &candidate, &canonical_consumers)?;
    let mut reasons = impact.reasons.clone();
    if changes.iter().any(|c| !c.history_complete) {
        reasons.push("commit-history-incomplete".into());
    }
    let impact_complete = impact.complete && changes.iter().all(|c| c.history_complete);
    let mut plan = UpdatePlan {
        created_at_unix: now_unix(),
        plan_version: 1,
        kind: if old.is_none() {
            "initial"
        } else if registration_change {
            "registration-change"
        } else {
            "update"
        }
        .into(),
        declaration_digest,
        old_lock_digest: old_bytes.as_ref().map(|b| sha256_hex(b)),
        selected_bundles: ids.into_iter().collect(),
        candidate,
        changes,
        consumers,
        impact,
        impact_complete,
        impact_reasons: reasons,
        plan_digest: String::new(),
    };
    plan.plan_digest = plan_hash(&plan)?;
    Ok(plan)
}
pub fn save_plan(path: &Path, plan: &UpdatePlan) -> Result<()> {
    ensure_writable(path)?;
    if path.exists() {
        return Err(usage("plan destination already exists"));
    }
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p)?;
    }
    write_new(path, &encoded(plan)?)?;
    Ok(())
}
pub fn load_plan(path: &Path) -> Result<UpdatePlan> {
    yaml(&std::fs::read(path)?, "update plan")
}
pub fn default_plan_path(plan: &UpdatePlan) -> Result<PathBuf> {
    Ok(cache_root()?.join("plans").join(format!(
        "{}-{}.yaml",
        plan.plan_digest,
        SERIAL.fetch_add(1, Ordering::Relaxed)
    )))
}
pub fn apply_plan(
    catalog: &Path,
    plan: &UpdatePlan,
    allow_incomplete: bool,
) -> Result<ApplyReport> {
    if plan.plan_version != 1 || plan.plan_digest != plan_hash(plan)? {
        return Err(usage("invalid or changed update plan"));
    }
    let declarations = registrations(catalog)?;
    if plan.declaration_digest != digest(&declarations)? {
        return Err(usage("catalog changed since planning"));
    }
    let path = lock_path(catalog);
    ensure_writable(&path)?;
    let _guard = guard(&path.with_extension("lock.guard"))?;
    if read_digest(&path)? != plan.old_lock_digest {
        return Err(usage("accepted lock changed since planning"));
    }
    validate_lock(&plan.candidate, &declarations)?;
    let accepted = match std::fs::read(&path) {
        Ok(bytes) => Some(yaml::<LockFile>(&bytes, "accepted lock")?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(e.into()),
    };
    let expected_kind = match &accepted {
        None => "initial",
        Some(accepted) if accepted.declaration_digest != plan.declaration_digest => {
            "registration-change"
        }
        Some(_) => "update",
    };
    if plan.kind != expected_kind {
        return Err(usage("plan kind does not match accepted baseline"));
    }
    let selected_ids: BTreeSet<_> = plan.selected_bundles.iter().cloned().collect();
    if selected_ids.len() != plan.selected_bundles.len()
        || selected_ids.iter().any(|id| !declarations.contains_key(id))
    {
        return Err(usage("invalid selected registration set"));
    }
    if plan.kind == "registration-change" && selected_ids != declarations.keys().cloned().collect()
    {
        return Err(usage(
            "registration-change plan must include all dependencies",
        ));
    }
    if let Some(old) = &accepted {
        for (id, bundle) in &plan.candidate.bundles {
            if !selected_ids.contains(id)
                && (old.bundles.get(id) != Some(bundle)
                    || old.repositories.get(&bundle.repository)
                        != plan.candidate.repositories.get(&bundle.repository))
            {
                return Err(usage("plan changes an unselected dependency"));
            }
        }
    }
    for id in &selected_ids {
        let decl = &declarations[id];
        if declarations
            .iter()
            .any(|(other, d)| d.url == decl.url && !selected_ids.contains(other))
        {
            return Err(usage(
                "plan must select the complete shared-repository group",
            ));
        }
    }
    for consumer in &plan.consumers {
        if consumer_digest(&consumer.root)? != consumer.digest {
            return Err(usage("consumer content changed since impact analysis"));
        }
    }
    let consumer_roots: Vec<_> = plan.consumers.iter().map(|c| c.root.clone()).collect();
    let actual_impact =
        super::remote_impact::report(catalog, accepted.as_ref(), &plan.candidate, &consumer_roots)?;
    if digest(&actual_impact)? != digest(&plan.impact)?
        || plan.impact_complete
            != (actual_impact.complete && plan.changes.iter().all(|c| c.history_complete))
    {
        return Err(usage("plan impact evidence changed or is inconsistent"));
    }
    if !plan.impact_complete && !allow_incomplete {
        return Err(usage("impact evidence is incomplete; review reported gaps and explicitly use --allow-incomplete-impact"));
    }
    for (id, b) in &plan.candidate.bundles {
        if !selected_ids.contains(id) {
            continue;
        }
        let repo = &plan.candidate.repositories[&b.repository];
        let snap = verify_snapshot(repo)?;
        let observed = observe(&snapshot_dir(repo)?.join("tree"), &declarations[id], &snap)?;
        if observed != b.observed {
            return Err(usage("candidate observed metadata changed"));
        }
    }
    for change in &plan.changes {
        if let Some(old) = &change.old_commit {
            let old_lock: LockFile = yaml(&std::fs::read(&path)?, "accepted lock")?;
            let repo = old_lock
                .repositories
                .values()
                .find(|r| &r.commit == old && Some(&r.url) == change.old_url.as_ref())
                .ok_or_else(|| usage("plan old commit does not match accepted lock"))?;
            tree(repo)?;
        }
    }
    let report_path = cache_root()?
        .join("reports")
        .join(format!("{}-accepted.yaml", plan.plan_digest));
    let report = ApplyReport {
        accepted_at_unix: now_unix(),
        impact_reasons: plan.impact_reasons.clone(),
        lock_path: path.clone(),
        plan_digest: plan.plan_digest.clone(),
        accepted: true,
        incomplete_impact_accepted: !plan.impact_complete,
        report_path: report_path.clone(),
    };
    ensure_writable(&report_path)?;
    private_dir(report_path.parent().unwrap())?;
    if report_path.exists() {
        return Err(usage(
            "acceptance report already exists; inspect previous acceptance before retrying",
        ));
    }
    let report_temp = report_path.with_extension(format!(
        "pending-{}-{}",
        std::process::id(),
        SERIAL.fetch_add(1, Ordering::Relaxed)
    ));
    write_new(&report_temp, &encoded(&report)?)?;
    let temp = path.with_extension(format!(
        "tmp-{}-{}",
        std::process::id(),
        SERIAL.fetch_add(1, Ordering::Relaxed)
    ));
    if let Err(error) = write_new(&temp, &encoded(&plan.candidate)?) {
        let _ = std::fs::remove_file(&report_temp);
        return Err(error);
    }
    if let Err(error) = std::fs::rename(&temp, &path) {
        let _ = std::fs::remove_file(&temp);
        let _ = std::fs::remove_file(&report_temp);
        return Err(error.into());
    }
    std::fs::rename(&report_temp, &report_path).map_err(|e| {
        environment(format!(
            "lock committed, but acceptance report publication failed: {e}; pending report {}",
            report_temp.display()
        ))
    })?;

    Ok(report)
}
/// Managed snapshots are protected even when addressed without a consumer catalog.
pub fn ensure_writable(path: &Path) -> Result<()> {
    let abs = canonical_existing(path)?;
    if PROTECTED.with(|p| p.borrow().iter().any(|root| abs.starts_with(root))) {
        return Err(OkfError::Policy(
            "destination belongs to a read-only remote registration".into(),
        ));
    }
    for ancestor in abs.ancestors() {
        let marker = ancestor.join(META);
        if marker.is_file()
            && ancestor.join("tree").is_dir()
            && abs.starts_with(ancestor.join("tree"))
        {
            let snap: Snapshot = yaml(&std::fs::read(&marker)?, "snapshot origin")?;
            return Err(OkfError::Policy(format!(
                "managed remote snapshot is read-only: {} at {}",
                snap.url, snap.commit
            )));
        }
    }
    let cwd = std::env::current_dir()?;
    let catalog = std::env::var_os("OKF_CATALOG")
        .map(PathBuf::from)
        .or_else(|| {
            cwd.ancestors()
                .map(|p| p.join("okf-catalog.yaml"))
                .find(|p| p.is_file())
        });
    if let Some(catalog) = catalog {
        let declarations = registrations(&catalog)?;
        // Protection uses declared ownership, not successful lock compatibility: a stale lock
        // must not prevent repairing local catalog configuration or unlock managed snapshots.
        if let Ok(bytes) = std::fs::read(lock_path(&catalog)) {
            if let Ok(lock) = yaml::<LockFile>(&bytes, "catalog lock") {
                for (id, decl) in &declarations {
                    if let Some(bundle) = lock.bundles.get(id) {
                        if let Some(repo) = lock.repositories.get(&bundle.repository) {
                            let root = snapshot_dir(repo)?
                                .join("tree")
                                .join(safe_relative(&decl.path)?);
                            if abs.starts_with(canonical_existing(&root)?) {
                                return Err(OkfError::Policy(format!(
                                    "remote bundle {id} is read-only ({})",
                                    decl.url
                                )));
                            }
                        }
                    }
                }
            }
        }
        let raw: serde_yaml::Value = yaml(&std::fs::read(&catalog)?, "catalog")?;
        let override_path = std::env::var_os("OKF_CATALOG_OVERRIDES")
            .map(PathBuf::from)
            .or_else(|| {
                raw.get("catalog_overrides")
                    .and_then(|v| v.as_str())
                    .map(|p| catalog.parent().unwrap_or(Path::new(".")).join(p))
            });
        if let Some(path) = override_path {
            let value: serde_yaml::Value = yaml(&std::fs::read(&path)?, "catalog overrides")?;
            if let Some(map) = value.get("bundles").and_then(|v| v.as_mapping()) {
                for (id, location) in map {
                    if id.as_str().is_some_and(|id| declarations.contains_key(id)) {
                        if let Some(location) = location.as_str() {
                            let root = canonical_existing(
                                &path.parent().unwrap_or(Path::new(".")).join(location),
                            )?;
                            if abs.starts_with(root) {
                                return Err(OkfError::Policy(
                                    "remote override is read-only".into(),
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
fn canonical_existing(path: &Path) -> Result<PathBuf> {
    let abs = super::catalog::absolute(path)?;
    let mut existing = abs.as_path();
    let mut tail = Vec::new();
    loop {
        match std::fs::symlink_metadata(existing) {
            Ok(_) => break,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        tail.push(
            existing
                .file_name()
                .ok_or_else(|| usage("invalid destination"))?
                .to_owned(),
        );
        existing = existing
            .parent()
            .ok_or_else(|| usage("invalid destination parent"))?;
    }
    let mut p = std::fs::canonicalize(existing)?;
    for name in tail.into_iter().rev() {
        p.push(name);
    }
    Ok(p)
}
/// Validate a user-owned checkout without resetting, cleaning, or fetching it.
pub fn validate_override(catalog: &Path, id: &str, root: &Path) -> Result<()> {
    let declarations = registrations(catalog)?;
    let lock = checked_lock(catalog, &declarations, true)?.unwrap();
    let decl = declarations
        .get(id)
        .ok_or_else(|| usage("override is not a Git registration"))?;
    let b = &lock.bundles[id];
    let repo = &lock.repositories[&b.repository];
    let current = git_text(root, &["rev-parse", "HEAD"])?;
    if current != repo.commit {
        return Err(usage("override checkout does not match locked commit"));
    }
    let repo_root = PathBuf::from(git_text(root, &["rev-parse", "--show-toplevel"])?);
    if std::fs::canonicalize(repo_root.join(&decl.path))? != std::fs::canonicalize(root)? {
        return Err(usage("override path does not match locked bundle subpath"));
    }
    let status = git(
        root,
        &[
            "status",
            "--porcelain",
            "--untracked-files=all",
            "--ignored",
            "--",
            &format!(":(top){}", decl.path),
        ],
    )?;
    if !status.is_empty() {
        return Err(usage(
            "override bundle contains modified, extra, or ignored files",
        ));
    }
    Ok(())
}
fn managed_metadata(directory: &Path) -> Result<Option<Snapshot>> {
    let namespace = canonical_existing(&cache_root()?.join("snapshots"))?;
    if directory.parent().and_then(Path::parent) != Some(namespace.as_path()) {
        return Ok(None);
    }
    let snap: Snapshot = yaml(
        &std::fs::read(directory.join(META))?,
        "managed snapshot origin",
    )?;
    let expected = canonical_existing(&snapshot_dir(&LockedRepository {
        url: snap.url.clone(),
        commit: snap.commit.clone(),
        object_format: snap.object_format.clone(),
    })?)?;
    if expected != directory || snap.profile != PROFILE {
        return Err(environment("managed snapshot origin identity mismatch"));
    }
    Ok(Some(snap))
}
/// Read an immutable object, including from managed snapshots lacking `.git` metadata.
pub fn read_snapshot(root: &Path, commit: &str, relative: &Path) -> Result<Vec<u8>> {
    if !full_oid(commit) {
        return Err(usage("snapshot read requires a full immutable commit ID"));
    }
    safe_relative(&relative.to_string_lossy())?;
    let root = std::fs::canonicalize(root)?;
    for ancestor in root.ancestors() {
        if let Some(snap) = managed_metadata(ancestor)? {
            let rel = root
                .strip_prefix(ancestor.join("tree"))
                .map_err(|_| usage("root is not inside managed tree"))?
                .join(relative);
            return git(
                &object_store(&snap.url)?,
                &["show", &format!("{}:{}", commit, rel.to_string_lossy())],
            );
        }
    }
    let top = PathBuf::from(git_text(&root, &["rev-parse", "--show-toplevel"])?);
    let rel = root
        .strip_prefix(&top)
        .map_err(|_| usage("bundle outside repository"))?
        .join(relative);
    git(
        &top,
        &["show", &format!("{}:{}", commit, rel.to_string_lossy())],
    )
}

/// Repository identity for a materialized path; does not load upstream configuration.
pub fn origin(root: &Path) -> Result<Option<RemoteOrigin>> {
    let abs = canonical_existing(root)?;
    for ancestor in abs.ancestors() {
        if let Some(snap) = managed_metadata(ancestor)? {
            if !abs.starts_with(ancestor.join("tree")) {
                continue;
            }
            verify_snapshot(&LockedRepository {
                url: snap.url.clone(),
                commit: snap.commit.clone(),
                object_format: snap.object_format.clone(),
            })?;
            let relative = abs
                .strip_prefix(ancestor.join("tree"))
                .unwrap()
                .to_string_lossy()
                .to_string();
            return Ok(Some(RemoteOrigin {
                url: snap.url,
                commit: Some(snap.commit),
                bundle_path: relative,
                access: "read-only".into(),
                state: "matched".into(),
            }));
        }
    }
    Ok(None)
}
#[derive(Debug, Clone)]
pub struct LocalSnapshot {
    pub root: PathBuf,
    pub repository: PathBuf,
    pub commit: String,
    pub bundle_relative: PathBuf,
}
/// Materialize locally available Git objects only. This never invokes fetch, hooks or filters.
pub fn materialize_local(root: &Path, revision: &str) -> Result<LocalSnapshot> {
    if revision.is_empty() || revision.starts_with('-') || revision.contains(['\0', '\n', '\r']) {
        return Err(usage("invalid local revision"));
    }
    let abs = std::fs::canonicalize(root)?;
    let (repository, url, bundle_relative) = if let Some(origin) = origin(&abs)? {
        (
            object_store(&origin.url)?,
            origin.url,
            PathBuf::from(origin.bundle_path),
        )
    } else {
        let top = PathBuf::from(git_text(&abs, &["rev-parse", "--show-toplevel"])?);
        let rel = abs
            .strip_prefix(&top)
            .map_err(|_| usage("bundle is outside repository"))?
            .to_path_buf();
        (top.clone(), format!("local:{}", top.display()), rel)
    };
    let commit = git_text(
        &repository,
        &[
            "rev-parse",
            "--verify",
            "--end-of-options",
            &format!("{revision}^{{commit}}"),
        ],
    )?;
    let object_format = git_text(&repository, &["rev-parse", "--show-object-format"])?;
    // Copy the exact locally available commit into a controlled bare store without fetch.
    let store = object_store(&url)?;
    if repository != store {
        private_dir(&store)?;
        if !store.join("HEAD").exists() {
            git(
                &store,
                &[
                    "init",
                    "--bare",
                    "--quiet",
                    &format!("--object-format={object_format}"),
                ],
            )?;
        }
        let _guard = guard(&store.with_extension("local.lock"))?;
        let mut pack = git_command(&repository)
            .args(["pack-objects", "--stdout", "--revs"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()?;
        {
            use std::io::Write;
            pack.stdin
                .take()
                .unwrap()
                .write_all(format!("{commit}\n").as_bytes())?;
        }
        let mut unpack = match git_command(&store)
            .args(["index-pack", "--stdin"])
            .stdin(std::process::Stdio::from(pack.stdout.take().unwrap()))
            .stdout(std::process::Stdio::null())
            .spawn()
        {
            Ok(child) => child,
            Err(error) => {
                let _ = pack.kill();
                let _ = pack.wait();
                return Err(error.into());
            }
        };
        let unpack_status = unpack.wait()?;
        let pack_status = pack.wait()?;
        if !unpack_status.success() || !pack_status.success() {
            return Err(environment("cannot materialize local snapshot objects"));
        }
    }
    let repo = LockedRepository {
        url,
        commit: commit.clone(),
        object_format,
    };
    let root = materialize(&repo)?;
    Ok(LocalSnapshot {
        root,
        repository: store,
        commit,
        bundle_relative,
    })
}
thread_local! {static PROTECTED:std::cell::RefCell<Vec<PathBuf>>=const{std::cell::RefCell::new(Vec::new())};}
/// Scoped roots preserve catalog access policy for override checkouts and explicit selection.
pub struct ProtectionGuard {
    previous: Vec<PathBuf>,
}
impl Drop for ProtectionGuard {
    fn drop(&mut self) {
        PROTECTED.with(|p| *p.borrow_mut() = std::mem::take(&mut self.previous));
    }
}
pub fn protect_catalog(catalog: &super::catalog::Catalog) -> Result<ProtectionGuard> {
    let roots = catalog
        .entries
        .values()
        .filter(|e| e.settings.contains_key("remote"))
        .map(|e| canonical_existing(&e.root))
        .collect::<Result<Vec<_>>>()?;
    let previous = PROTECTED.with(|p| std::mem::replace(&mut *p.borrow_mut(), roots));
    Ok(ProtectionGuard { previous })
}

/// Offline materialization for a locked repository already present in the object store.
pub fn snapshot_root(repo: &LockedRepository) -> Result<PathBuf> {
    materialize(repo)
}

#[cfg(test)]
mod tests {
    use super::*;
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    struct CacheEnv(Option<PathBuf>);
    impl CacheEnv {
        fn set(path: &Path) -> Self {
            let old = TEST_CACHE.with(|p| p.replace(Some(path.to_path_buf())));
            Self(old)
        }
    }
    impl Drop for CacheEnv {
        fn drop(&mut self) {
            TEST_CACHE.with(|p| *p.borrow_mut() = self.0.take());
        }
    }
    fn commit(repo: &Path, message: &str) -> String {
        git(repo, &["add", "."]).unwrap();
        git(
            repo,
            &[
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.invalid",
                "commit",
                "-qm",
                message,
            ],
        )
        .unwrap();
        git_text(repo, &["rev-parse", "HEAD"]).unwrap()
    }
    fn fixture(root: &Path) -> (PathBuf, PathBuf) {
        let upstream = root.join("upstream");
        std::fs::create_dir_all(upstream.join("bundles/docs")).unwrap();
        git(&upstream, &["init", "-q", "-b", "main"]).unwrap();
        std::fs::write(
            upstream.join("bundles/docs/index.md"),
            "---\nokf_version: '0.2'\n---\n# Docs\n",
        )
        .unwrap();
        std::fs::write(
            upstream.join("bundles/docs/a.md"),
            "---\ntype: Note\n---\n# A\n",
        )
        .unwrap();
        commit(&upstream, "first");
        let catalog = root.join("okf-catalog.yaml");
        std::fs::write(&catalog,format!("catalog_version: 2\nbundles:\n  test.docs:\n    location:\n      type: git\n      url: '{}'\n      ref: refs/heads/main\n      path: bundles/docs\n    accepts:\n      okf_spec: ['0.2']\n",upstream.display())).unwrap();
        (upstream, catalog)
    }
    #[test]
    fn plans_pin_exact_objects_detect_stale_inputs_and_preserve_read_only_snapshots() {
        let _serial = ENV_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let _env = CacheEnv::set(&dir.path().join("cache"));
        let (upstream, catalog) = fixture(dir.path());
        assert!(
            !resolve_registration(&catalog, "test.docs")
                .unwrap()
                .available
        );
        assert!(sync(&catalog, None, true).is_err());
        let plan = plan_update(&catalog, None, &[]).unwrap();
        assert_eq!(plan.kind, "initial");
        assert!(!lock_path(&catalog).exists());
        assert!(apply_plan(&catalog, &plan, false).is_err());
        std::fs::write(
            upstream.join("bundles/docs/a.md"),
            "---\ntype: Note\n---\n# Changed upstream\n",
        )
        .unwrap();
        let newer = commit(&upstream, "second");
        apply_plan(&catalog, &plan, true).unwrap();
        let accepted = resolve_registration(&catalog, "test.docs").unwrap();
        assert!(accepted.available);
        assert_ne!(accepted.origin.commit.as_deref(), Some(newer.as_str()));
        assert!(std::fs::read_to_string(accepted.root.join("a.md"))
            .unwrap()
            .contains("# A"));
        assert!(ensure_writable(&accepted.root.join("new.md")).is_err());
        assert!(sync(&catalog, None, true).is_ok());
        assert!(apply_plan(&catalog, &plan, true).is_err());
        let next = plan_update(&catalog, None, &[]).unwrap();
        assert_eq!(next.kind, "update");
        assert!(next.changes[0]
            .changed
            .contains(&"bundles/docs/a.md".into()));
        let old_origin = origin(&accepted.root).unwrap().unwrap();
        assert_eq!(
            read_snapshot(
                &accepted.root,
                old_origin.commit.as_deref().unwrap(),
                Path::new("a.md")
            )
            .unwrap(),
            std::fs::read(accepted.root.join("a.md")).unwrap()
        );
        let local = materialize_local(&upstream, "HEAD").unwrap();
        assert_eq!(local.commit, newer);
        assert!(local.root.join("bundles/docs/a.md").is_file());
        std::fs::write(accepted.root.join("extra.md"), "unexpected").unwrap();
        assert!(sync(&catalog, None, true).is_err());
        let recovery = recover_cache(&catalog, None).unwrap();
        assert_eq!(recovery.quarantined.len(), 1);
        assert!(recovery.quarantined[0]
            .join("tree/bundles/docs/extra.md")
            .is_file());
        assert!(sync(&catalog, None, true).is_ok());
    }
    #[test]
    fn consumer_baselines_and_unsupported_entries_are_not_waived() {
        let _serial = ENV_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let _env = CacheEnv::set(&dir.path().join("cache"));
        let (upstream, catalog) = fixture(dir.path());
        let consumer = dir.path().join("consumer");
        std::fs::create_dir(&consumer).unwrap();
        std::fs::write(consumer.join("a.md"), "---\ntype: Note\n---\n").unwrap();
        let plan = plan_update(&catalog, None, std::slice::from_ref(&consumer)).unwrap();
        std::fs::write(consumer.join("a.md"), "changed").unwrap();
        assert!(apply_plan(&catalog, &plan, true).is_err());
        std::fs::write(
            upstream.join("bundles/docs/lfs.bin"),
            "version https://git-lfs.github.com/spec/v1\noid sha256:abc\nsize 1\n",
        )
        .unwrap();
        commit(&upstream, "lfs");
        assert!(plan_update(&catalog, None, &[]).is_err());
    }
    #[test]
    fn complete_scoped_plan_and_registration_removal_preserve_accepted_state() {
        let _serial = ENV_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let _env = CacheEnv::set(&dir.path().join("cache"));
        let (_upstream, catalog) = fixture(dir.path());
        let consumer = dir.path().join("consumer");
        std::fs::create_dir(&consumer).unwrap();
        std::fs::write(
            consumer.join("note.md"),
            "---\ntype: Note\n---\nSee [A](okf://test.docs/a.md).\n",
        )
        .unwrap();
        let plan = plan_update(&catalog, None, std::slice::from_ref(&consumer)).unwrap();
        assert!(plan.impact_complete, "{:?}", plan.impact_reasons);
        apply_plan(&catalog, &plan, false).unwrap();
        let bad = std::fs::read_to_string(&catalog)
            .unwrap()
            .replace("refs/heads/main", "refs/heads/new");
        std::fs::write(&catalog, bad).unwrap();
        assert!(sync(&catalog, None, true).is_err());
        std::fs::write(&catalog, "catalog_version: 2\nbundles: {}\n").unwrap();
        let removal = plan_update(&catalog, None, &[consumer]).unwrap();
        assert_eq!(removal.kind, "registration-change");
        assert!(removal.candidate.bundles.is_empty());
        apply_plan(&catalog, &removal, false).unwrap();
        assert!(sync(&catalog, None, true).unwrap().bundles.is_empty());
    }
    #[test]
    fn repository_groups_update_together_and_reject_unsafe_overrides() {
        let _serial = ENV_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let _env = CacheEnv::set(&dir.path().join("cache"));
        let (upstream, catalog) = fixture(dir.path());
        std::fs::create_dir_all(upstream.join("bundles/other")).unwrap();
        std::fs::write(
            upstream.join("bundles/other/a.md"),
            "---\ntype: Note\n---\n",
        )
        .unwrap();
        commit(&upstream, "second bundle");
        let mut source = std::fs::read_to_string(&catalog).unwrap();
        source.push_str(&format!("  test.other:\n    location:\n      type: git\n      url: '{}'\n      ref: refs/heads/main\n      path: bundles/other\n",upstream.display()));
        std::fs::write(&catalog, &source).unwrap();
        let plan = plan_update(&catalog, Some("test.docs"), &[]).unwrap();
        assert_eq!(plan.selected_bundles, vec!["test.docs", "test.other"]);
        apply_plan(&catalog, &plan, true).unwrap();
        validate_override(&catalog, "test.docs", &upstream.join("bundles/docs")).unwrap();
        std::fs::write(upstream.join("bundles/docs/extra.txt"), "untracked").unwrap();
        assert!(validate_override(&catalog, "test.docs", &upstream.join("bundles/docs")).is_err());
        std::fs::write(
            &catalog,
            source.replacen("ref: refs/heads/main", "ref: refs/heads/other", 1),
        )
        .unwrap();
        assert!(registrations(&catalog).is_err());
    }
    #[test]
    fn acquisition_cannot_bypass_conformance_or_read_external_ontology_without_consumers() {
        let _serial = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let _env = CacheEnv::set(&dir.path().join("cache"));
        let (upstream, catalog) = fixture(dir.path());
        std::fs::write(
            upstream.join("bundles/docs/a.md"),
            "Missing concept frontmatter\n",
        )
        .unwrap();
        commit(&upstream, "invalid concept");
        assert!(plan_update(&catalog, None, &[])
            .unwrap_err()
            .to_string()
            .contains("conformance"));
        std::fs::write(upstream.join("bundles/docs/a.md"), "---\ntype: Note\n---\n").unwrap();
        std::fs::write(
            upstream.join("bundles/docs/okf-bundle.yaml"),
            "manifest_version: 1\nsettings:\n  ontology: ../../outside.yaml\n",
        )
        .unwrap();
        commit(&upstream, "external ontology");
        assert!(plan_update(&catalog, None, &[])
            .unwrap_err()
            .to_string()
            .contains("bundle-relative"));
        assert!(!lock_path(&catalog).exists());
    }

    #[test]
    fn root_and_dot_prefixed_bundles_reject_unsupported_entries() {
        let _serial = ENV_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let _env = CacheEnv::set(&dir.path().join("cache"));
        let (upstream, catalog) = fixture(dir.path());
        std::fs::write(
            upstream.join("bundles/docs/lfs.bin"),
            "version https://git-lfs.github.com/spec/v1\noid sha256:abc\nsize 1\n",
        )
        .unwrap();
        commit(&upstream, "unsupported");
        let base = std::fs::read_to_string(&catalog).unwrap();
        std::fs::write(
            &catalog,
            base.replace("path: bundles/docs", "path: ./bundles/docs"),
        )
        .unwrap();
        assert!(plan_update(&catalog, None, &[]).is_err());
        let root = base
            .replace("path: bundles/docs", "path: .")
            .replace("    accepts:\n      okf_spec: ['0.2']\n", "");
        std::fs::write(&catalog, root).unwrap();
        assert!(plan_update(&catalog, None, &[]).is_err());
    }
    #[test]
    fn strict_paths_refs_and_registration_groups() {
        assert!(safe_relative("../escape").is_err());
        assert!(safe_relative(".git/config").is_err());
        assert!(validate_ref("main").is_err());
        assert!(validate_ref("refs/heads/main:evil").is_err());
        assert!(validate_url("https://secret:password@example.invalid/a").is_err());
    }
}
/// Pure snapshot location; existence and integrity are deliberately not implied.
pub fn snapshot_location(repo: &LockedRepository) -> Result<PathBuf> {
    Ok(snapshot_dir(repo)?.join("tree"))
}

#[derive(Debug, Clone, Serialize)]
pub struct RecoveryReport {
    pub quarantined: Vec<PathBuf>,
    pub verification: SyncReport,
}
/// Explicit offline recovery preserves damaged snapshots for inspection. Never touches overrides.
pub fn recover_cache(catalog: &Path, id: Option<&str>) -> Result<RecoveryReport> {
    let declarations = registrations(catalog)?;
    let lock = checked_lock(catalog, &declarations, true)?.unwrap();
    let ids = selected(&declarations, id)?;
    let mut done = BTreeSet::new();
    let mut quarantined = Vec::new();
    for id in ids {
        let bundle = &lock.bundles[&id];
        if !done.insert(bundle.repository.clone()) {
            continue;
        }
        let repo = &lock.repositories[&bundle.repository];
        // Availability of pinned objects is checked before moving any existing snapshot.
        tree(repo)?;
        let dest = snapshot_dir(repo)?;
        if dest.exists() && verify_snapshot(repo).is_err() {
            let _guard = guard(&dest.with_extension("publish.lock"))?;
            let target = cache_root()?.join("quarantine").join(format!(
                "{}-{}-{}-{}",
                repo_key(&repo.url),
                repo.commit,
                std::process::id(),
                SERIAL.fetch_add(1, Ordering::Relaxed)
            ));
            private_dir(target.parent().unwrap())?;
            std::fs::rename(&dest, &target)?;
            quarantined.push(target);
        }
        materialize(repo)?;
    }
    Ok(RecoveryReport {
        quarantined,
        verification: sync(catalog, id, true)?,
    })
}

#[cfg(test)]
mod process_lock_tests {
    #[test]
    fn stable_lock_file_can_be_reacquired_after_owner_releases_it() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("operation.lock");
        let first = super::guard(&path).unwrap();
        assert!(super::guard(&path).is_err());
        drop(first);
        assert!(path.is_file());
        let _next = super::guard(&path).unwrap();
    }
}
