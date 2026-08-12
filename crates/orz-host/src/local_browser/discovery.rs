//! Browser binary discovery for the `local_browser` lane.
//!
//! Resolution order (2026-08-10):
//! 1. `ORZ_BROWSER_PATH` — explicit path to chrome/msedge. Set but missing
//!    fails loudly with the searched path (never a silent fallback).
//! 2. Windows standard install paths (Chrome + Edge — `PATH` does not cover
//!    `Program Files`).
//! 3. `PATH` lookup (`chrome`, `google-chrome`, `chromium`, `msedge`).
//!
//! Launch args are owned here so the manager and the probe agree on a single
//! profile layout (`{workspace}/.gsa/chrome-profile-<session8>`, ADR-0009
//! A-class write point).

use std::path::{Path, PathBuf};

/// Env var for an explicit browser executable path.
pub const ORZ_BROWSER_PATH_ENV: &str = "ORZ_BROWSER_PATH";

/// Windows-standard candidates (checked before PATH — PATH doesn't cover
/// `Program Files`). Chrome's Channel/* dirs and Edge's version dirs make
/// these glob-ish: we probe `exe` at the well-known root and, for Edge, the
/// Application dir (single version there is the common case).
#[cfg(windows)]
const WINDOWS_CANDIDATES: &[&str] = &[
    r"C:\Program Files\Google\Chrome\Application\chrome.exe",
    r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
    r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
    r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
];

/// PATH names tried for non-Windows / as a last resort.
const PATH_NAMES: &[&str] = &[
    "chrome",
    "google-chrome",
    "google-chrome-stable",
    "chromium",
    "chromium-browser",
    "msedge",
];

/// Why discovery failed (surfaced in the `Degraded("browser_launch_failed: ...")`
/// reason so the model sees an explicit, actionable cause — ADR-0010 §3.7.2).
#[derive(Debug, thiserror::Error)]
pub enum DiscoveryError {
    #[error("no browser executable found (ORZ_BROWSER_PATH unset; searched: {paths})")]
    NotFound { paths: String },

    #[error("ORZ_BROWSER_PATH set to {path} but the file does not exist")]
    ExplicitPathMissing { path: String },
}

/// A discovered browser executable.
#[derive(Debug, Clone)]
pub struct BrowserBinary {
    pub path: PathBuf,
    /// Where it came from — used in Degraded reasons and logs.
    pub origin: DiscoveryOrigin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscoveryOrigin {
    Env,
    WindowsPath,
    PathLookup,
}

/// Find a browser executable.
///
/// `env_path` is the caller-provided `ORZ_BROWSER_PATH` value (`None` when
/// unset) so the function stays pure and testable.
pub fn find_browser(env_path: Option<&str>) -> Result<BrowserBinary, DiscoveryError> {
    if let Some(p) = env_path {
        let path = PathBuf::from(p);
        if path.is_file() {
            return Ok(BrowserBinary {
                path,
                origin: DiscoveryOrigin::Env,
            });
        }
        return Err(DiscoveryError::ExplicitPathMissing {
            path: p.to_string(),
        });
    }

    let mut searched: Vec<String> = Vec::new();

    #[cfg(windows)]
    for cand in WINDOWS_CANDIDATES {
        searched.push(cand.to_string());
        if Path::new(cand).is_file() {
            return Ok(BrowserBinary {
                path: PathBuf::from(cand),
                origin: DiscoveryOrigin::WindowsPath,
            });
        }
    }

    for name in PATH_NAMES {
        searched.push(name.to_string());
        if let Some(path) = find_on_path(name) {
            return Ok(BrowserBinary {
                path,
                origin: DiscoveryOrigin::PathLookup,
            });
        }
    }

    Err(DiscoveryError::NotFound {
        paths: searched.join(", "),
    })
}

/// Minimal `which`-style PATH lookup (no extra dependency): searches each
/// PATH entry for `name`, appending PATHEXT extensions on Windows so
/// `chrome` resolves to `chrome.exe`.
fn find_on_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    #[cfg(windows)]
    let exts: Vec<String> = std::env::var("PATHEXT")
        .map(|s| {
            s.split(';')
                .filter(|e| !e.is_empty())
                .map(|e| e.to_ascii_lowercase())
                .collect()
        })
        .unwrap_or_else(|_| vec![".exe".to_string()]);
    for dir in std::env::split_paths(&path) {
        let base = dir.join(name);
        if base.is_file() {
            return Some(base);
        }
        #[cfg(windows)]
        for ext in &exts {
            let candidate = dir.join(format!("{name}{ext}"));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

/// Env var forcing headless mode. User ruling (2026-08-10, after the
/// review batch): the browser runs HEADED by default — the visible window
/// lets the operator log in to sites (campus SSO, journal accounts) once
/// through the window; cookies persist in the session's isolated profile.
/// Set this in display-less environments (CI, containers) where headed
/// Chrome cannot start.
pub const ORZ_BROWSER_HEADLESS_ENV: &str = "ORZ_BROWSER_HEADLESS";

/// Command-line arguments for launching the browser on the local_browser
/// lane's isolated profile. The debug endpoint binds 127.0.0.1 only
/// (`--remote-debugging-port` on a loopback-only port) and `--remote-allow-
/// origins=*` is required by newer Chrome before it accepts non-local-origin
/// websocket handshakes.
///
/// `headless` is an explicit parameter (caller reads `ORZ_BROWSER_HEADLESS`)
/// so tests cover both modes without env manipulation.
pub fn browser_launch_args(profile_dir: &Path, headless: bool) -> Vec<String> {
    let mut args = Vec::new();
    if headless {
        args.push("--headless=new".to_string());
    }
    args.push("--remote-debugging-port=0".to_string());
    args.push("--remote-allow-origins=*".to_string());
    args.push(format!("--user-data-dir={}", profile_dir.display()));
    args.push("--no-first-run".to_string());
    args.push("--no-default-browser-check".to_string());
    args.push("--disable-extensions".to_string());
    args.push("--disable-background-networking".to_string());
    args.push("--disable-sync".to_string());
    args.push("--disable-translate".to_string());
    args
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_path_wins_when_file_exists() {
        let dir = std::env::temp_dir().join(format!(
            "orz-browser-disc-{:?}",
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let fake = dir.join("chrome.exe");
        std::fs::write(&fake, b"not really a browser").unwrap();

        let found = find_browser(Some(fake.to_str().unwrap())).unwrap();
        assert_eq!(found.origin, DiscoveryOrigin::Env);
        assert_eq!(found.path, fake);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn explicit_path_missing_fails_loudly() {
        let dir = std::env::temp_dir().join(format!(
            "orz-browser-disc-{:?}",
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let missing = dir.join("nope.exe");

        match find_browser(Some(missing.to_str().unwrap())) {
            Err(DiscoveryError::ExplicitPathMissing { path }) => {
                assert!(path.contains("nope.exe"), "{path}")
            }
            other => panic!("expected ExplicitPathMissing, got {other:?}"),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn no_browser_lists_searched_paths() {
        match find_browser(None) {
            Err(DiscoveryError::NotFound { paths }) => {
                // The message must be actionable — it names what was tried.
                assert!(!paths.is_empty());
            }
            Err(e) => panic!("no env override was set; only NotFound is reachable: {e:?}"),
            // A real machine may have Chrome/Edge — acceptable either way;
            // the error path is what we assert when there is none.
            Ok(_) => {}
        }
    }

    #[test]
    fn launch_args_are_isolated_and_mode_switchable() {
        let profile = PathBuf::from(r"C:\work\.gsa\chrome-profile-RUN123456");
        // Headed — the 2026-08-10 user ruling default (operator logs in
        // through the visible window).
        let headed = browser_launch_args(&profile, false);
        assert!(!headed.iter().any(|a| a == "--headless=new"));
        assert!(headed.iter().any(|a| a == "--remote-debugging-port=0"));
        assert!(headed.iter().any(|a| a == "--remote-allow-origins=*"));
        assert!(headed.iter().any(|a| a == "--disable-extensions"));
        assert!(
            headed.iter().any(
                |a| a.starts_with("--user-data-dir=") && a.contains("chrome-profile-RUN123456")
            )
        );
        // Headless fallback (ORZ_BROWSER_HEADLESS=1).
        let headless = browser_launch_args(&profile, true);
        assert!(headless.iter().any(|a| a == "--headless=new"));
        assert!(
            headless.iter().any(
                |a| a.starts_with("--user-data-dir=") && a.contains("chrome-profile-RUN123456")
            )
        );
    }
}
