//! L1 (2026-08-08 write-placement ruling): redirect `$GROK_HOME` off the
//! user directory (C: on dev machines) to the orz install directory.
//!
//! Design: `docs/WRITE_PLACEMENT_AND_GRILL_DESIGN_2026-08-08.md` §1/§2.
//! B-class (own-body) writes — config/logs/credential metadata/trust files —
//! all route through `orz_config::grok_home()` (OnceLock + `GROK_HOME` env)
//! or the standalone `xai_fast_worktree::resolve_grok_home()` (worktree
//! paths, reads the same env), so a single env injection at process start
//! zeroes the whole B-class (write-point inventory: §1 ②).
//!
//! Degradation chain (§2 ruling): install dir writable → `{cwd}/.gsa/grok-home`
//! → user dir (last resort, caller must surface the warning).
//! Rulings honored: `$GROK_HOME` already set → respected verbatim (Q4, no
//! `ORZ_`-prefixed variant); cwd = process-start cwd (Q3); multi-instance
//! lock races accepted (Q6 — atomic write + exclusive-create precedent).

use std::path::{Path, PathBuf};

/// Where the redirect decided `$GROK_HOME` should point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GrokHomePlacement {
    /// `$GROK_HOME` was already set (user/CI) — respected verbatim (Q4 ruling).
    EnvRespected,
    /// `{install_dir}/grok-home` — default when the install dir is writable.
    InstallDir(PathBuf),
    /// `{cwd}/.gsa/grok-home` — install dir not writable (Program Files /
    /// eval container).
    WorkspaceFallback(PathBuf),
    /// No placement writable — the user dir stays the last resort.
    UserFallback,
}

/// Redirect `$GROK_HOME` to the install dir (or the degradation chain).
///
/// Must run **before any `orz_config::grok_home()` call** — that accessor
/// caches the value in a process-wide `OnceLock`, so injection after first
/// use is a no-op. Call at the very top of every entry (`orz-bin`/`orz-codex`
/// `main`, `orz-tui` `run`); the second call is idempotent (`EnvRespected`).
///
/// `cwd` is the process-start working directory (Q3 ruling — a later
/// `--run-root` switch is a view change, not a re-anchoring).
///
/// SAFETY contract: single-threaded call before any runtime starts (edition
/// 2024 `std::env::set_var` is `unsafe` — same precedent as orz-bin's
/// `--real`/`--allow-write` env flags).
pub fn redirect_grok_home(cwd: &Path) -> GrokHomePlacement {
    redirect_grok_home_with(cwd, install_dir().as_deref())
}

/// Test-injectable core: `install_dir` supplied explicitly instead of
/// derived from `current_exe()`.
fn redirect_grok_home_with(cwd: &Path, install_dir: Option<&Path>) -> GrokHomePlacement {
    if std::env::var_os("GROK_HOME").is_some() {
        return GrokHomePlacement::EnvRespected;
    }
    if let Some(dir) = install_dir {
        let target = dir.join("grok-home");
        if try_claim_dir(&target) {
            set_grok_home(&target);
            return GrokHomePlacement::InstallDir(target);
        }
    }
    let fallback = cwd.join(".gsa").join("grok-home");
    if try_claim_dir(&fallback) {
        set_grok_home(&fallback);
        return GrokHomePlacement::WorkspaceFallback(fallback);
    }
    GrokHomePlacement::UserFallback
}

/// The binary's own directory (`current_exe` parent) — the B-class home.
fn install_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()?
        .parent()
        .map(Path::to_path_buf)
}

/// Create the dir and prove writability with an exclusive probe file
/// (`create_dir_all` on an existing dir does not prove ACL-writable).
///
/// Self-healing (2026-08-08 review D2-1/P2-4): a stale `.orz-probe` from a
/// crashed or racing earlier process must not permanently demote a writable
/// install dir to a lower tier — on `AlreadyExists` the probe is removed and
/// retried once (two racing first-starts: one wins, the other clears and
/// reclaims; the probe window is transient).
fn try_claim_dir(dir: &Path) -> bool {
    if !std::fs::create_dir_all(dir).is_ok() {
        return false;
    }
    let probe = dir.join(".orz-probe");
    match std::fs::File::create_new(&probe) {
        Ok(_) => {
            let _ = std::fs::remove_file(&probe);
            true
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            // Stale/racing probe — clear and reclaim once.
            let _ = std::fs::remove_file(&probe);
            match std::fs::File::create_new(&probe) {
                Ok(_) => {
                    let _ = std::fs::remove_file(&probe);
                    true
                }
                Err(_) => false,
            }
        }
        Err(_) => false,
    }
}

fn set_grok_home(path: &Path) {
    // SAFETY: single-threaded, process start (see `redirect_grok_home`).
    unsafe {
        std::env::set_var("GROK_HOME", path.as_os_str());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;
    use std::sync::Mutex;

    /// Serialize env-mutating tests (shared process under `cargo test` —
    /// xai-fast-worktree GROK_HOME_ENV_LOCK precedent).
    static GROK_HOME_ENV_LOCK: Mutex<()> = Mutex::new(());

    /// Restore the previous `GROK_HOME` (or unset it) on drop.
    struct EnvVarGuard {
        old: Option<OsString>,
    }

    impl EnvVarGuard {
        /// Capture the current value; restore it on drop. Every test that
        /// sets the env (directly or via `redirect_grok_home_with`) must hold
        /// one — otherwise the value leaks into the next test (shared
        /// process) and that test sees `EnvRespected`.
        fn capture() -> Self {
            Self {
                old: std::env::var_os("GROK_HOME"),
            }
        }

        fn set(value: &str) -> Self {
            let guard = Self::capture();
            // SAFETY: test-only, single-threaded under the lock.
            unsafe {
                std::env::set_var("GROK_HOME", value);
            }
            guard
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            // SAFETY: test-only, single-threaded under the lock.
            unsafe {
                match &self.old {
                    Some(v) => std::env::set_var("GROK_HOME", v),
                    None => std::env::remove_var("GROK_HOME"),
                }
            }
        }
    }

    fn with_lock(f: impl FnOnce()) {
        let _guard = GROK_HOME_ENV_LOCK.lock().unwrap();
        f();
    }

    fn tmpdir(label: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("orz-grok-home-{label}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create tmp dir");
        dir
    }

    #[test]
    fn env_already_set_is_respected_verbatim() {
        with_lock(|| {
            let _guard = EnvVarGuard::set("C:/custom/grok-home");
            let tmp = tmpdir("env-set");
            let placement = redirect_grok_home_with(&tmp, Some(&tmp));
            assert_eq!(placement, GrokHomePlacement::EnvRespected);
            assert_eq!(std::env::var("GROK_HOME").unwrap(), "C:/custom/grok-home");
            // No claim performed — no dirs created.
            assert!(!tmp.join("grok-home").exists());
        });
    }

    #[test]
    fn writable_install_dir_claims_install_dir() {
        with_lock(|| {
            let _guard = EnvVarGuard::capture();
            let tmp = tmpdir("install-writable");
            let placement = redirect_grok_home_with(&tmp, Some(&tmp));
            let expected = tmp.join("grok-home");
            assert_eq!(placement, GrokHomePlacement::InstallDir(expected.clone()));
            assert_eq!(
                std::env::var("GROK_HOME").unwrap(),
                expected.to_str().unwrap()
            );
            assert!(expected.is_dir());
            // Probe file must not remain behind.
            assert!(!expected.join(".orz-probe").exists());
        });
    }

    #[test]
    fn unwritable_install_dir_falls_back_to_workspace() {
        with_lock(|| {
            let _guard = EnvVarGuard::capture();
            let tmp = tmpdir("install-unwritable");
            // A file where a directory is expected → create_dir_all fails.
            let blocker = tmp.join("not-a-dir");
            std::fs::write(&blocker, b"x").expect("write blocker");
            let cwd = tmpdir("fallback-cwd");
            let placement = redirect_grok_home_with(&cwd, Some(&blocker));
            let expected = cwd.join(".gsa").join("grok-home");
            assert_eq!(
                placement,
                GrokHomePlacement::WorkspaceFallback(expected.clone())
            );
            assert_eq!(
                std::env::var("GROK_HOME").unwrap(),
                expected.to_str().unwrap()
            );
        });
    }

    #[test]
    fn both_unwritable_leaves_user_dir() {
        with_lock(|| {
            let _guard = EnvVarGuard::capture();
            let tmp = tmpdir("both-unwritable");
            let blocker = tmp.join("not-a-dir");
            std::fs::write(&blocker, b"x").expect("write blocker");
            // cwd is also a file → `{cwd}/.gsa` cannot be created either.
            let placement = redirect_grok_home_with(&blocker, Some(&blocker));
            assert_eq!(placement, GrokHomePlacement::UserFallback);
            assert!(std::env::var_os("GROK_HOME").is_none());
        });
    }
}
