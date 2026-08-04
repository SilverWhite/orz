//! IP5 pre-mutation snapshot store (content-addressed).
//!
//! Layout under the store root:
//!
//! ```text
//! <root>/
//!   objects/<sha256>              — file content, one object per distinct file
//!   manifests/<snapshot_hash>.json — canonical manifest of one snapshot
//! ```
//!
//! A snapshot hash is the SHA-256 of the canonical JSON of its manifest —
//! the analogue of the git tree hash in the v0.1 design. Each manifest entry
//! records the worktree-relative path (forward slashes), the object hash, the
//! size, and the pre-mutation mtime (nanos since epoch, 0 when unknown).
//!
//! Exclusions (from v0.1 §4.6): files larger than `max_file_bytes` (default
//! 2 MiB) and directory names in the exclusion list (default `.git`) are not
//! snapshotted. `track_worktree` walks the whole worktree with these rules;
//! `track` snapshots an explicit file list (the ToolDispatcher wrapper calls
//! this with the mutation tool's target paths).
//!
//! Symlink handling: `exists()`/`metadata()` follow links, so symlinks to
//! directories and dangling symlinks are skipped (not errors); symlinked
//! files are snapshotted by their target content.
//!
//! `mtime_nanos` is recorded for future incremental/dedup use; `verify` and
//! `restore` compare content hashes only.

use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::Semaphore;
use tracing::debug;

use crate::journal::{canonical_json, sha256_hex};

/// Default maximum snapshot file size (2 MiB, matches v0.1 design).
pub const DEFAULT_MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;

/// Default excluded directory names (never walked into).
pub const DEFAULT_EXCLUDED_DIRS: &[&str] = &[".git", "node_modules", "target"];

/// One manifest entry: a single snapshotted file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SnapshotEntry {
    /// Worktree-relative path, `/`-separated.
    pub path: String,
    /// SHA-256 of the file content (also the object key).
    pub object_sha256: String,
    /// File size in bytes at snapshot time.
    pub size: u64,
    /// Pre-mutation mtime in nanoseconds since Unix epoch (0 = unknown).
    pub mtime_nanos: u64,
}

/// Outcome of `track` — the snapshot handle plus what was captured.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotRecord {
    pub snapshot_hash: String,
    pub entries: Vec<SnapshotEntry>,
}

impl SnapshotRecord {
    /// True when no files were captured (e.g. all targets missing/excluded).
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Object hashes referenced by this snapshot.
    pub fn object_hashes(&self) -> impl Iterator<Item = &str> {
        self.entries.iter().map(|e| e.object_sha256.as_str())
    }
}

/// Result of `verify` — compares current worktree state against a snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnapshotVerifyOutcome {
    /// Every manifest entry matches the current file state.
    Clean,
    /// Entries that differ from the snapshot (path → object_sha256).
    Changed { changed: Vec<String> },
}

/// Result of `restore`/`revert` — what was written back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotRestoreOutcome {
    pub restored: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum SnapshotError {
    #[error("snapshot store root not a directory: {0}")]
    InvalidRoot(PathBuf),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("invalid snapshot hash: {0}")]
    InvalidHash(String),

    #[error("snapshot not found: {0}")]
    NotFound(String),

    #[error("file outside worktree: {0}")]
    OutsideWorktree(PathBuf),

    #[error("worktree path is a directory: {0}")]
    IsDirectory(PathBuf),

    #[error("snapshot manifest corrupt: {0}")]
    CorruptManifest(String),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("restore failed for {path}: {reason}")]
    RestoreFailed { path: String, reason: String },
}

/// IP5 pre-mutation snapshot store.
///
/// One instance per (session, worktree). All operations are serialised through
/// an internal semaphore (v0.1 design), so concurrent tool calls cannot
/// interleave a track with a restore of the same worktree.
#[derive(Debug)]
pub struct SnapshotStore {
    root: PathBuf,
    worktree: PathBuf,
    max_file_bytes: u64,
    excluded_dirs: Vec<String>,
    semaphore: Arc<Semaphore>,
}

impl SnapshotStore {
    /// Create a store rooted at `root` (created if missing) for `worktree`.
    ///
    /// `worktree` must be an absolute path; it is canonicalised if it exists.
    pub fn new(root: PathBuf, worktree: PathBuf) -> Result<Self, SnapshotError> {
        let worktree = if worktree.is_absolute() {
            dunce::canonicalize(&worktree).unwrap_or(worktree)
        } else {
            std::env::current_dir()?.join(worktree)
        };
        Ok(Self {
            root,
            worktree,
            max_file_bytes: DEFAULT_MAX_FILE_BYTES,
            excluded_dirs: DEFAULT_EXCLUDED_DIRS.iter().map(|s| s.to_string()).collect(),
            semaphore: Arc::new(Semaphore::new(1)),
        })
    }

    /// Override the per-file size cap (bytes).
    pub fn with_max_file_bytes(mut self, max: u64) -> Self {
        self.max_file_bytes = max;
        self
    }

    /// Override the excluded directory names.
    pub fn with_excluded_dirs(mut self, dirs: &[&str]) -> Self {
        self.excluded_dirs = dirs.iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn worktree(&self) -> &Path {
        &self.worktree
    }

    /// Snapshot an explicit file list (relative to the worktree).
    pub async fn track(&self, paths: &[PathBuf]) -> Result<SnapshotRecord, SnapshotError> {
        let _guard = self.semaphore.acquire().await.map_err(std::io::Error::other)?;
        let mut entries = Vec::new();
        for path in paths {
            let full = self.resolve(path)?;
            match self.capture_file(&full, path) {
                Ok(Some(entry)) => entries.push(entry),
                Ok(None) => {} // missing / excluded / too large
                Err(e) => return Err(e),
            }
        }
        self.persist(entries)
    }

    /// Snapshot the whole worktree (with exclusion rules).
    pub async fn track_worktree(&self) -> Result<SnapshotRecord, SnapshotError> {
        let _guard = self.semaphore.acquire().await.map_err(std::io::Error::other)?;
        let mut entries = Vec::new();
        let mut stack = vec![self.worktree.clone()];
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir)? {
                let entry = entry?;
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().into_owned();
                let rel = self.relative(&path);
                if entry.file_type()?.is_dir() {
                    if self.excluded_dirs.contains(&name) {
                        continue;
                    }
                    stack.push(path);
                } else if let Some(snapshot_entry) = self.capture_file(&path, Path::new(&rel))? {
                    entries.push(snapshot_entry);
                }
            }
        }
        self.persist(entries)
    }

    /// Verify the worktree state against a snapshot hash.
    pub async fn verify(&self, snapshot_hash: &str) -> Result<SnapshotVerifyOutcome, SnapshotError> {
        let manifest = self.load_manifest(snapshot_hash)?;
        let mut changed = Vec::new();
        for entry in manifest {
            let full = self.resolve(Path::new(&entry.path))?;
            match self.hash_file(&full) {
                Ok(Some(hash)) if hash == entry.object_sha256 => {}
                _ => changed.push(entry.path),
            }
        }
        if changed.is_empty() {
            Ok(SnapshotVerifyOutcome::Clean)
        } else {
            Ok(SnapshotVerifyOutcome::Changed { changed })
        }
    }

    /// Restore every file in a snapshot back into the worktree.
    pub async fn restore(&self, snapshot_hash: &str) -> Result<SnapshotRestoreOutcome, SnapshotError> {
        let _guard = self.semaphore.acquire().await.map_err(std::io::Error::other)?;
        let manifest = self.load_manifest(snapshot_hash)?;
        let mut restored = Vec::new();
        for entry in manifest {
            let full = self.resolve(Path::new(&entry.path))?;
            let object = self.read_object(&entry.object_sha256)?;
            write_atomic(&full, &object)?;
            restored.push(entry.path);
        }
        Ok(SnapshotRestoreOutcome { restored })
    }

    /// Restore only the listed paths from a snapshot (selective revert).
    ///
    /// Paths not present in the snapshot are skipped (not an error).
    pub async fn revert(
        &self,
        snapshot_hash: &str,
        paths: &[PathBuf],
    ) -> Result<SnapshotRestoreOutcome, SnapshotError> {
        let _guard = self.semaphore.acquire().await.map_err(std::io::Error::other)?;
        let manifest = self.load_manifest(snapshot_hash)?;
        let mut wanted = Vec::new();
        for p in paths {
            wanted.push(self.relative(&self.resolve(p)?));
        }
        let mut restored = Vec::new();
        for entry in manifest {
            if wanted.contains(&entry.path) {
                let full = self.resolve(Path::new(&entry.path))?;
                let object = self.read_object(&entry.object_sha256)?;
                write_atomic(&full, &object)?;
                restored.push(entry.path);
            }
        }
        Ok(SnapshotRestoreOutcome { restored })
    }

    /// All snapshot hashes currently stored (for GC / inventory).
    pub fn list_snapshots(&self) -> Result<Vec<String>, SnapshotError> {
        let dir = self.root.join("manifests");
        let mut hashes = Vec::new();
        if dir.is_dir() {
            for entry in std::fs::read_dir(dir)? {
                let entry = entry?;
                if entry.file_type()?.is_file()
                    && let Some(name) = entry.file_name().to_str()
                    && let Some(hash) = name.strip_suffix(".json")
                {
                    hashes.push(hash.to_string());
                }
            }
        }
        hashes.sort();
        Ok(hashes)
    }

    // ── internals ──────────────────────────────────────────────────────────

    /// Resolve a worktree-relative path, rejecting `..` escape and absolute
    /// paths that leave the worktree.
    fn resolve(&self, rel: &Path) -> Result<PathBuf, SnapshotError> {
        if rel.is_absolute() {
            return Err(SnapshotError::OutsideWorktree(rel.to_path_buf()));
        }
        let mut clean = PathBuf::new();
        for component in rel.components() {
            match component {
                Component::Normal(part) => clean.push(part),
                Component::CurDir => {}
                Component::ParentDir => return Err(SnapshotError::OutsideWorktree(rel.to_path_buf())),
                _ => return Err(SnapshotError::OutsideWorktree(rel.to_path_buf())),
            }
        }
        Ok(self.worktree.join(clean))
    }

    /// Worktree-relative, `/`-separated string for a path inside the worktree.
    fn relative(&self, path: &Path) -> String {
        match path.strip_prefix(&self.worktree) {
            Ok(rel) => rel.components().map(|c| c.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/"),
            Err(_) => path.to_string_lossy().replace('\\', "/"),
        }
    }

    /// Capture one file: content-addressed object + manifest entry.
    /// Returns `None` for missing files, excluded sizes, or directories.
    fn capture_file(&self, full: &Path, rel: &Path) -> Result<Option<SnapshotEntry>, SnapshotError> {
        if !full.exists() {
            return Ok(None);
        }
        let meta = full.metadata()?;
        if meta.is_dir() {
            return Ok(None);
        }
        if meta.len() > self.max_file_bytes {
            debug!(path = %rel.display(), len = meta.len(), "snapshot: file exceeds max size, excluded");
            return Ok(None);
        }
        let bytes = std::fs::read(full)?;
        let hash = sha256_hex(&bytes);
        self.write_object(&hash, &bytes)?;
        let mtime = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        Ok(Some(SnapshotEntry {
            path: rel.components().map(|c| c.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/"),
            object_sha256: hash,
            size: meta.len(),
            mtime_nanos: mtime,
        }))
    }

    /// Persist entries as a manifest; returns the snapshot hash.
    fn persist(&self, mut entries: Vec<SnapshotEntry>) -> Result<SnapshotRecord, SnapshotError> {
        // Deterministic order — canonical manifest independent of walk order.
        let mut by_path: BTreeMap<String, SnapshotEntry> = BTreeMap::new();
        for entry in entries.drain(..) {
            by_path.insert(entry.path.clone(), entry);
        }
        let ordered: Vec<SnapshotEntry> = by_path.into_values().collect();
        let bytes = canonical_json(&ordered)?;
        let hash = sha256_hex(&bytes);
        let manifest_dir = self.root.join("manifests");
        std::fs::create_dir_all(&manifest_dir)?;
        std::fs::write(manifest_dir.join(format!("{hash}.json")), &bytes)?;
        debug!(snapshot = %hash, files = ordered.len(), "snapshot recorded");
        Ok(SnapshotRecord {
            snapshot_hash: hash,
            entries: ordered,
        })
    }

    fn load_manifest(&self, snapshot_hash: &str) -> Result<Vec<SnapshotEntry>, SnapshotError> {
        if !snapshot_hash
            .chars()
            .all(|c| c.is_ascii_hexdigit())
            || snapshot_hash.len() != 64
        {
            return Err(SnapshotError::InvalidHash(snapshot_hash.to_string()));
        }
        let manifest_path = self.root.join("manifests").join(format!("{snapshot_hash}.json"));
        let bytes = std::fs::read(&manifest_path)
            .map_err(|_| SnapshotError::NotFound(snapshot_hash.to_string()))?;
        let entries: Vec<SnapshotEntry> =
            serde_json::from_slice(&bytes).map_err(|e| SnapshotError::CorruptManifest(e.to_string()))?;
        Ok(entries)
    }

    fn write_object(&self, hash: &str, bytes: &[u8]) -> Result<(), SnapshotError> {
        let objects_dir = self.root.join("objects");
        std::fs::create_dir_all(&objects_dir)?;
        let target = objects_dir.join(hash);
        if !target.exists() {
            // Content-addressed: identical content is stored once.
            write_atomic(&target, bytes)?;
        }
        Ok(())
    }

    fn read_object(&self, hash: &str) -> Result<Vec<u8>, SnapshotError> {
        let target = self.root.join("objects").join(hash);
        std::fs::read(&target)
            .map_err(|_| SnapshotError::CorruptManifest(format!("missing object {hash}")))
    }

    fn hash_file(&self, path: &Path) -> Result<Option<String>, SnapshotError> {
        if !path.exists() {
            return Ok(None);
        }
        let bytes = std::fs::read(path)?;
        Ok(Some(sha256_hex(&bytes)))
    }
}

/// Write bytes to `path` atomically (temp file + rename), creating parents.
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), SnapshotError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("snapshot");
    let tmp = path.with_file_name(format!(".{file_name}.snap-tmp"));
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store_in(dir: &std::path::Path) -> SnapshotStore {
        SnapshotStore::new(dir.join("store"), dir.join("worktree")).unwrap()
    }

    #[tokio::test]
    async fn track_captures_content_and_returns_hash() {
        let dir = tempfile::tempdir().unwrap();
        let worktree = dir.path().join("worktree");
        std::fs::create_dir_all(&worktree).unwrap();
        std::fs::write(worktree.join("a.txt"), "hello").unwrap();
        std::fs::create_dir_all(worktree.join("sub")).unwrap();
        std::fs::write(worktree.join("sub/b.txt"), "world").unwrap();

        let store = store_in(dir.path());
        let record = store.track(&[PathBuf::from("a.txt"), PathBuf::from("sub/b.txt")]).await.unwrap();
        assert_eq!(record.entries.len(), 2);
        assert_eq!(record.snapshot_hash.len(), 64);

        // Deterministic: same content → same snapshot hash, and manifests dir has it.
        let again = store.track(&[PathBuf::from("sub/b.txt"), PathBuf::from("a.txt")]).await.unwrap();
        assert_eq!(again.snapshot_hash, record.snapshot_hash);
        assert!(store.list_snapshots().unwrap().contains(&record.snapshot_hash));
    }

    #[tokio::test]
    async fn verify_detects_change_and_restore_reverts() {
        let dir = tempfile::tempdir().unwrap();
        let worktree = dir.path().join("worktree");
        std::fs::create_dir_all(&worktree).unwrap();
        let file = worktree.join("a.txt");
        std::fs::write(&file, "original").unwrap();

        let store = store_in(dir.path());
        let record = store.track(&[PathBuf::from("a.txt")]).await.unwrap();
        assert_eq!(store.verify(&record.snapshot_hash).await.unwrap(), SnapshotVerifyOutcome::Clean);

        std::fs::write(&file, "mutated").unwrap();
        match store.verify(&record.snapshot_hash).await.unwrap() {
            SnapshotVerifyOutcome::Changed { changed } => assert_eq!(changed, vec!["a.txt".to_string()]),
            SnapshotVerifyOutcome::Clean => panic!("expected change"),
        }

        let outcome = store.restore(&record.snapshot_hash).await.unwrap();
        assert_eq!(outcome.restored, vec!["a.txt".to_string()]);
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "original");
        assert_eq!(store.verify(&record.snapshot_hash).await.unwrap(), SnapshotVerifyOutcome::Clean);
    }

    #[tokio::test]
    async fn revert_is_selective() {
        let dir = tempfile::tempdir().unwrap();
        let worktree = dir.path().join("worktree");
        std::fs::create_dir_all(&worktree).unwrap();
        std::fs::write(worktree.join("a.txt"), "a0").unwrap();
        std::fs::write(worktree.join("b.txt"), "b0").unwrap();

        let store = store_in(dir.path());
        let record = store.track(&[PathBuf::from("a.txt"), PathBuf::from("b.txt")]).await.unwrap();
        std::fs::write(worktree.join("a.txt"), "a1").unwrap();
        std::fs::write(worktree.join("b.txt"), "b1").unwrap();

        let outcome = store.revert(&record.snapshot_hash, &[PathBuf::from("a.txt")]).await.unwrap();
        assert_eq!(outcome.restored, vec!["a.txt".to_string()]);
        assert_eq!(std::fs::read_to_string(worktree.join("a.txt")).unwrap(), "a0");
        assert_eq!(std::fs::read_to_string(worktree.join("b.txt")).unwrap(), "b1");
    }

    #[tokio::test]
    async fn track_worktree_walks_recursively_and_excludes() {
        let dir = tempfile::tempdir().unwrap();
        let worktree = dir.path().join("worktree");
        std::fs::create_dir_all(worktree.join(".git")).unwrap();
        std::fs::create_dir_all(worktree.join("src")).unwrap();
        std::fs::write(worktree.join("src/main.rs"), "fn main() {}").unwrap();
        std::fs::write(worktree.join(".git/config"), "secret git config").unwrap();

        let store = store_in(dir.path());
        let record = store.track_worktree().await.unwrap();
        assert_eq!(record.entries.len(), 1);
        assert_eq!(record.entries[0].path, "src/main.rs");
        assert!(!record.snapshot_hash.is_empty());
    }

    #[tokio::test]
    async fn excludes_large_files() {
        let dir = tempfile::tempdir().unwrap();
        let worktree = dir.path().join("worktree");
        std::fs::create_dir_all(&worktree).unwrap();
        std::fs::write(worktree.join("big.bin"), vec![0u8; 300]).unwrap();

        let store = store_in(dir.path()).with_max_file_bytes(100);
        let record = store.track(&[PathBuf::from("big.bin")]).await.unwrap();
        assert!(record.is_empty());
    }

    #[tokio::test]
    async fn rejects_path_escape() {
        let dir = tempfile::tempdir().unwrap();
        let worktree = dir.path().join("worktree");
        std::fs::create_dir_all(&worktree).unwrap();
        let store = store_in(dir.path());
        let err = store.track(&[PathBuf::from("../escape.txt")]).await.unwrap_err();
        assert!(matches!(err, SnapshotError::OutsideWorktree(_)));
    }

    #[tokio::test]
    async fn missing_file_is_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let worktree = dir.path().join("worktree");
        std::fs::create_dir_all(&worktree).unwrap();
        let store = store_in(dir.path());
        let record = store.track(&[PathBuf::from("nope.txt")]).await.unwrap();
        assert!(record.is_empty());
    }

    #[tokio::test]
    async fn restore_is_atomic_and_creates_parents() {
        let dir = tempfile::tempdir().unwrap();
        let worktree = dir.path().join("worktree");
        std::fs::create_dir_all(&worktree).unwrap();
        let file = worktree.join("deep/nested/a.txt");
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, "v1").unwrap();

        let store = store_in(dir.path());
        let record = store.track(&[PathBuf::from("deep/nested/a.txt")]).await.unwrap();

        // Delete the file (simulates destructive mutation) then restore.
        std::fs::remove_file(&file).unwrap();
        std::fs::remove_dir(file.parent().unwrap()).unwrap();
        let outcome = store.restore(&record.snapshot_hash).await.unwrap();
        assert_eq!(outcome.restored, vec!["deep/nested/a.txt".to_string()]);
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "v1");
    }

    #[tokio::test]
    async fn invalid_or_missing_hash_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let worktree = dir.path().join("worktree");
        std::fs::create_dir_all(&worktree).unwrap();
        let store = store_in(dir.path());

        let err = store.verify("not-a-hash").await.unwrap_err();
        assert!(matches!(err, SnapshotError::InvalidHash(_)));

        let err = store.verify("a".repeat(64).as_str()).await.unwrap_err();
        assert!(matches!(err, SnapshotError::NotFound(_)));
    }
}
