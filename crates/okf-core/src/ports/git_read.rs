//! Scoped local Git reads. Nested operations share discovery, object reads and failures;
//! dropping the outer session discards everything so later commands observe fresh state.
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, ExitStatus, Output, Stdio};
use std::rc::{Rc, Weak};

type CachedOutput = Result<Output, (io::ErrorKind, String)>;
#[derive(Default)]
struct Cache {
    commands: HashMap<(PathBuf, Vec<String>), CachedOutput>,
    blobs: HashMap<(PathBuf, String), Option<Vec<u8>>>,
    readers: HashMap<PathBuf, Option<Batch>>,
}
thread_local! {
    static ACTIVE: RefCell<Weak<RefCell<Cache>>> = const { RefCell::new(Weak::new()) };
}
/// Own this guard for one read operation; nested guards reuse the same cache.
pub struct ReadSession(Rc<RefCell<Cache>>);
impl Default for ReadSession {
    fn default() -> Self {
        Self::new()
    }
}
impl ReadSession {
    pub fn new() -> Self {
        Self(ACTIVE.with(|active| {
            if let Some(cache) = active.borrow().upgrade() {
                return cache;
            }
            let cache = Rc::new(RefCell::new(Cache::default()));
            *active.borrow_mut() = Rc::downgrade(&cache);
            cache
        }))
    }
}
fn command(root: &Path) -> Command {
    let mut cmd = Command::new("git");
    cmd.arg("-C")
        .arg(root)
        .env("GIT_NO_LAZY_FETCH", "1")
        .env("GIT_TERMINAL_PROMPT", "0");
    cmd
}
/// Read-only commands only; never use this cache for Git mutations.
pub(crate) fn output(root: &Path, args: &[&str]) -> io::Result<Output> {
    let cache = ACTIVE.with(|active| active.borrow().upgrade());
    let Some(cache) = cache else {
        return command(root).args(args).output();
    };
    let key = (
        root.to_path_buf(),
        args.iter().map(std::string::ToString::to_string).collect(),
    );
    let mut cache = cache.borrow_mut();
    cache
        .commands
        .entry(key)
        .or_insert_with(|| {
            command(root)
                .args(args)
                .output()
                .map_err(|error| (error.kind(), error.to_string()))
        })
        .clone()
        .map_err(|(kind, message)| io::Error::new(kind, message))
}
/// Read one immutable object through a persistent cat-file process, caching missing objects too.
pub(crate) fn blob(root: &Path, revision: &str, path: &Path) -> Option<Vec<u8>> {
    let session = ReadSession::new();
    let spec = format!("{revision}:{}", path.to_str()?);
    let key = (root.to_path_buf(), spec.clone());
    let mut cache = session.0.borrow_mut();
    if let Some(bytes) = cache.blobs.get(&key) {
        return bytes.clone();
    }
    // The line-based batch protocol cannot represent these names. Keep Git's argv semantics.
    let bytes = if !Batch::representable(&spec) {
        show(root, &spec)
    } else {
        let reader = cache
            .readers
            .entry(root.to_path_buf())
            .or_insert_with(|| Batch::new(root).ok());
        match reader.as_mut().map(|reader| reader.read(&spec)) {
            Some(Ok(bytes)) => bytes,
            Some(Err(_)) => {
                // A failed read can leave the stream mid-object; a reused reader would then
                // return another object's bytes. Drop it (killing the process) and do not
                // cache the failure, so a later read starts a fresh reader.
                cache.readers.remove(root);
                return None;
            }
            None => None,
        }
    };
    cache.blobs.insert(key, bytes.clone());
    bytes
}
/// `git show <spec>` for object names the batch protocol cannot carry.
pub(crate) fn show(root: &Path, spec: &str) -> Option<Vec<u8>> {
    command(root)
        .args(["show", "--end-of-options", spec])
        .output()
        .ok()
        .filter(|out| out.status.success())
        .map(|out| out.stdout)
}

/// Owns a child process. Dropping it before [`KillOnDrop::wait`] succeeds kills and reaps the
/// process, so early returns never leave zombies or orphaned Git processes behind.
pub(crate) struct KillOnDrop {
    child: Child,
    reaped: bool,
}
impl KillOnDrop {
    pub(crate) fn new(child: Child) -> Self {
        Self {
            child,
            reaped: false,
        }
    }
    /// Wait for a normal exit; the guard no longer needs to kill the process.
    pub(crate) fn wait(&mut self) -> io::Result<ExitStatus> {
        let status = self.child.wait()?;
        self.reaped = true;
        Ok(status)
    }
}
impl std::ops::Deref for KillOnDrop {
    type Target = Child;
    fn deref(&self) -> &Child {
        &self.child
    }
}
impl std::ops::DerefMut for KillOnDrop {
    fn deref_mut(&mut self) -> &mut Child {
        &mut self.child
    }
}
impl Drop for KillOnDrop {
    fn drop(&mut self) {
        if !self.reaped {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

/// A persistent `git cat-file --batch` reader. Any `Err` from [`Batch::read`] means the stream
/// may be desynchronised; callers must drop the reader rather than reuse it.
pub(crate) struct Batch {
    // Field order matters: close our pipe ends before the guard kills and reaps the child.
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    _child: KillOnDrop,
}
impl Batch {
    pub(crate) fn new(root: &Path) -> io::Result<Self> {
        let mut child = KillOnDrop::new(
            command(root)
                .args(["cat-file", "--batch"])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()?,
        );
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| io::Error::other("git cat-file stdin unavailable"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| io::Error::other("git cat-file stdout unavailable"))?;
        Ok(Self {
            stdin,
            stdout: BufReader::new(stdout),
            _child: child,
        })
    }
    /// Whether `spec` fits on one request line of the batch protocol.
    pub(crate) fn representable(spec: &str) -> bool {
        !spec.contains(['\n', '\r', '\0'])
    }
    /// The blob bytes, or `None` when the object is missing or not a blob.
    pub(crate) fn read(&mut self, spec: &str) -> io::Result<Option<Vec<u8>>> {
        if !Self::representable(spec) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "object name cannot be sent to cat-file --batch",
            ));
        }
        writeln!(self.stdin, "{spec}")?;
        self.stdin.flush()?;
        let mut header = String::new();
        self.stdout.read_line(&mut header)?;
        let Some(header) = header.strip_suffix('\n') else {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "truncated cat-file response",
            ));
        };
        if header.ends_with(" missing") || header.ends_with(" ambiguous") {
            return Ok(None);
        }
        let fields: Vec<_> = header.split(' ').collect();
        let size = match fields.as_slice() {
            [_, _, size] => size.parse::<usize>().ok(),
            _ => None,
        }
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid cat-file response"))?;
        let mut bytes = vec![0; size];
        self.stdout.read_exact(&mut bytes)?;
        let mut newline = [0];
        self.stdout.read_exact(&mut newline)?;
        if newline[0] != b'\n' {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid cat-file response framing",
            ));
        }
        if fields[1] != "blob" {
            return Ok(None);
        }
        Ok(Some(bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn git(root: &Path, args: &[&str]) {
        let out = Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .output()
            .unwrap();
        assert!(out.status.success(), "{out:?}");
    }
    fn commit(root: &Path) {
        git(root, &["add", "."]);
        git(
            root,
            &[
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.test",
                "commit",
                "-qm",
                "fixture",
            ],
        );
    }
    #[test]
    fn nested_sessions_share_reads_but_later_operations_observe_new_state() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        git(root, &["init", "-q"]);
        std::fs::write(root.join("a"), b"one\0\n").unwrap();
        commit(root);
        let session = ReadSession::new();
        let weak = Rc::downgrade(&session.0);
        {
            let nested = ReadSession::new();
            assert!(Rc::ptr_eq(&session.0, &nested.0));
            assert_eq!(
                blob(root, "HEAD", Path::new("a")),
                Some(b"one\0\n".to_vec())
            );
            assert_eq!(blob(root, "HEAD", Path::new("missing")), None);
        }
        // Failed reads and symbolic refs are stable within this read operation.
        std::fs::write(root.join("a"), b"two").unwrap();
        std::fs::write(root.join("missing"), b"present").unwrap();
        commit(root);
        assert_eq!(
            blob(root, "HEAD", Path::new("a")),
            Some(b"one\0\n".to_vec())
        );
        assert_eq!(blob(root, "HEAD", Path::new("missing")), None);
        drop(session);
        assert!(weak.upgrade().is_none());
        let _next = ReadSession::new();
        assert_eq!(blob(root, "HEAD", Path::new("a")), Some(b"two".to_vec()));
        assert_eq!(
            blob(root, "HEAD", Path::new("missing")),
            Some(b"present".to_vec())
        );
    }
    #[test]
    fn batch_reads_preserve_bytes_and_support_whitespace_names() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        git(root, &["init", "-q"]);
        for name in ["space name", "unicode-λ", "newline\nname"] {
            std::fs::write(root.join(name), [0, 255, b'\n', b'X']).unwrap();
        }
        commit(root);
        let _reads = ReadSession::new();
        for name in ["space name", "unicode-λ", "newline\nname"] {
            assert_eq!(
                blob(root, "HEAD", Path::new(name)),
                Some(vec![0, 255, b'\n', b'X'])
            );
        }
        assert_eq!(blob(root, "HEAD", Path::new("not-present")), None);
        assert_eq!(
            blob(root, "HEAD", Path::new("space name")),
            Some(vec![0, 255, b'\n', b'X'])
        );
    }
    #[test]
    fn failed_batch_reads_evict_the_reader_and_are_not_cached() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        git(root, &["init", "-q"]);
        std::fs::write(root.join("a"), b"one").unwrap();
        std::fs::write(root.join("b"), b"two").unwrap();
        commit(root);
        let session = ReadSession::new();
        assert_eq!(blob(root, "HEAD", Path::new("a")), Some(b"one".to_vec()));
        // Simulate a reader whose stream broke: its process is gone.
        {
            let mut cache = session.0.borrow_mut();
            let reader = cache.readers.get_mut(root).unwrap().as_mut().unwrap();
            reader._child.kill().unwrap();
            reader._child.wait().unwrap();
        }
        assert_eq!(blob(root, "HEAD", Path::new("b")), None);
        assert!(!session.0.borrow().readers.contains_key(root));
        // The failure was not cached and a fresh reader serves the next request.
        assert_eq!(blob(root, "HEAD", Path::new("b")), Some(b"two".to_vec()));
    }
    #[cfg(unix)]
    #[test]
    fn kill_on_drop_reaps_an_unwaited_child() {
        let child = Command::new("sleep").arg("30").spawn().unwrap();
        let pid = child.id().to_string();
        drop(KillOnDrop::new(child));
        // A zombie would still accept signal 0; a reaped process does not exist.
        let probe = Command::new("kill")
            .args(["-0", &pid])
            .stderr(Stdio::null())
            .status()
            .unwrap();
        assert!(!probe.success());
    }
}
