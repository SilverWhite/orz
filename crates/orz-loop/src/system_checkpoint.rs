//! 0ds M1 v1 (2026-10-28, 366 charter §1 item 5): the minimal durable
//! checkpoint producer.
//!
//! M1 only needs a mechanically verifiable checkpoint on the state write
//! path. The full M4 recovery service stays out of scope: this module writes
//! an atomic JSON snapshot under `<session>/.gsa/checkpoints/` and returns
//! the identity/digest/path for the `CheckpointCreated` journal row.
//!
//! Safety: the checkpoint carries the already-rendered state document only;
//! no tool arguments, no secrets. Writes are temp-file + rename so a crash
//! never leaves a half checkpoint at the final path.

use std::io;
use std::path::{Path, PathBuf};

use orz_assurance::journal::sha256_hex;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckpointWrite {
    pub checkpoint_id: String,
    pub generation: u64,
    pub state_sha256: String,
    pub relative_path: String,
    pub absolute_path: PathBuf,
}

/// Write one checkpoint for `state_document`. `generation` is monotonic in
/// the caller's state revision; the id includes the digest prefix so two
/// revisions with the same generation still cannot overwrite each other.
pub fn write_checkpoint(
    session_root: &Path,
    generation: u64,
    state_document: &str,
) -> io::Result<CheckpointWrite> {
    let state_sha256 = sha256_hex(state_document.as_bytes());
    let checkpoint_id = format!("CKPT-{generation:08}-{}", &state_sha256[..12]);
    let relative_path = format!(".gsa/checkpoints/{checkpoint_id}.json");
    let dir = session_root.join(".gsa").join("checkpoints");
    std::fs::create_dir_all(&dir)?;
    let absolute_path = dir.join(format!("{checkpoint_id}.json"));
    let temp_path = dir.join(format!(".{checkpoint_id}.json.tmp"));
    let body = serde_json::json!({
        "checkpoint_id": checkpoint_id,
        "generation": generation,
        "state_sha256": state_sha256,
        "state": state_document,
    });
    let bytes = serde_json::to_vec_pretty(&body)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    std::fs::write(&temp_path, &bytes)?;
    std::fs::rename(&temp_path, &absolute_path)?;
    Ok(CheckpointWrite {
        checkpoint_id,
        generation,
        state_sha256,
        relative_path,
        absolute_path,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_dir() -> PathBuf {
        std::env::temp_dir().join(format!(
            "orz-checkpoint-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn writes_atomic_checkpoint_with_digest() {
        let dir = test_dir();
        let write = write_checkpoint(&dir, 7, "state v1\ngoal: x\n").unwrap();
        assert_eq!(write.generation, 7);
        assert_eq!(write.state_sha256.len(), 64);
        assert!(write.relative_path.starts_with(".gsa/checkpoints/"));
        let text = std::fs::read_to_string(&write.absolute_path).unwrap();
        let json: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(json["checkpoint_id"], write.checkpoint_id);
        assert_eq!(json["state_sha256"], write.state_sha256);
        assert_eq!(json["state"], "state v1\ngoal: x\n");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn same_generation_different_state_does_not_overwrite() {
        let dir = test_dir();
        let a = write_checkpoint(&dir, 1, "state a").unwrap();
        let b = write_checkpoint(&dir, 1, "state b").unwrap();
        assert_ne!(a.checkpoint_id, b.checkpoint_id);
        assert!(a.absolute_path.exists());
        assert!(b.absolute_path.exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
