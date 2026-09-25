//! Trusted-workspace listing for the workbench explorer (0br S3 新增面,
//! user directive 2026-09-25: explorer regroups into 工作区 / 活跃会话 /
//! 归档会话 with the workspace group splitting into 当前工作区 + 已信任
//! 工作区). Reads the SAME user-global trust store the agent gates on
//! (`~/.grok/trusted_folders.toml` via `orz_workspace::trust::TrustStore`)
//! — no second implementation of the store semantics. Display-only: this
//! face never answers trust questions.

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct TrustedWorkspace {
    /// Folder key exactly as recorded in the store (canonical path).
    pub path: String,
    /// Unix timestamp (seconds) of the decision, when present.
    pub decided_at: Option<i64>,
}

/// Enumerate explicit trust GRANTS from the production store. Recorded
/// denies are skipped — the listing answers "which folders are trusted",
/// not "what did the user decide overall".
pub fn list_trusted_workspaces() -> Vec<TrustedWorkspace> {
    from_store(&orz_workspace::trust::TrustStore::load())
}

fn from_store(store: &orz_workspace::trust::TrustStore) -> Vec<TrustedWorkspace> {
    let mut out: Vec<TrustedWorkspace> = store
        .decisions()
        .filter(|(_, record)| record.trusted)
        .map(|(key, record)| TrustedWorkspace {
            path: key.to_string(),
            decided_at: record.decided_at,
        })
        .collect();
    out.sort_by_key(|w| w.path.to_lowercase());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn listing_reports_grants_and_skips_denies() {
        let tmp = tempfile_dir();
        let mut store =
            orz_workspace::trust::TrustStore::load_from(tmp.join("trusted_folders.toml"));
        let granted = tmp.join("repo-a");
        std::fs::create_dir_all(&granted).unwrap();
        store.set_trusted(&granted).unwrap();
        let denied = tmp.join("repo-b");
        std::fs::create_dir_all(&denied).unwrap();
        store.set_untrusted(&denied).unwrap();

        let listed = from_store(&store);
        assert_eq!(
            listed.len(),
            1,
            "denies never surface as trusted: {listed:?}"
        );
        assert!(
            listed[0].path.replace('/', "\\").ends_with("repo-a"),
            "grant listed with its stored key: {:?}",
            listed
        );
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn empty_store_lists_nothing() {
        let tmp = tempfile_dir();
        let store = orz_workspace::trust::TrustStore::load_from(tmp.join("trusted_folders.toml"));
        assert!(from_store(&store).is_empty());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    fn tempfile_dir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "orz-web-trust-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }
}
