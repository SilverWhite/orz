//! ACAF action target resolution (Slice 2 first phase, ADR-0011 §4.2/§4.5) —
//! the *parsed real target object* for action tickets: the resolved file
//! path for `file_write_v1`, the credential target for `credential_read_v1`,
//! and the canonical URL for `network_v1` (command_exec binds its
//! argv/cwd/env triple in the caller — orz-loop canonical helpers).
//!
//! `resolved_target_sha256` binds the digest of this parsed object, and the
//! consumption point re-parses the live object and re-derives the digest
//! (check 5 TOCTOU — never trusts the ticket's own values).
//!
//! The lexical primitives are single-sourced in `orz-paths::resolve`
//! (2026-08-12 decision); this module keeps the ticket-side orchestration
//! and the registered differences from the lenient tool resolver
//! (`orz-tools` `resolve_model_path` — now a thin shell over the same
//! primitives):
//! (1) verbatim/device (`\\?\`, `\\?\UNC\`, `\\.\`) prefixes are rejected
//! outright (they bypass lexical normalisation);
//! (2) `..` components are collapsed lexically via
//! `orz_paths::normalize_lexically` — the reparse scan runs on the
//! UN-FOLDED candidate first so a `<junction>\..` spelling cannot hide the
//! link (review D1-1 2026-08-12). Registered consequence: the
//! drive-relative `C:..\x` spelling keeps its `..` (prefix-preserving
//! fold) instead of the old copy's prefix-drop — conservative direction,
//! locked by `fold_drive_relative_preserves_parent_prefix`;
//! (3) `~user` is refused (`orz-paths` `tilde_expand_strict`, review
//! P1-2 — shellexpand expands it on Unix but not Windows; the ambiguous
//! form is never ticketable);
//! (4) the ticket side is single-base (worktree) — display_cwd ≡ cwd
//! simplification.
//! A residual divergence surfaces as a ticket `target_mismatch` in shadow
//! mode (observable, not silent).

#[cfg(windows)]
use std::path::Component;
use std::path::{Path, PathBuf};

use crate::journal::sha256_hex;
use orz_paths::resolve::{TildeExpandError, sanitize_model_path_arg, tilde_expand_strict};

/// Why a model-supplied target could not be resolved.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TargetResolveError {
    #[error("target path is empty")]
    Empty,
    #[error(
        "verbatim/device (\\\\?\\, \\\\?\\UNC\\, \\\\.\\) paths are not supported for action tickets"
    )]
    VerbatimUnsupported,
    #[error("target path contains a reparse point or symlink component")]
    ReparseComponent,
    #[error("~user-style paths are not supported for action tickets (ambiguous expansion)")]
    TildeUserUnsupported,
    #[error("target URL is not a valid absolute URL")]
    InvalidUrl,
    #[error("target URL scheme is not http/https")]
    UnsupportedScheme,
    #[error("target URL carries embedded credentials (userinfo)")]
    UrlCredentialsUnsupported,
}

/// Resolve a model-supplied path argument to the parsed real target, mirroring
/// `resolve_model_path(cwd, None, input)` (display_cwd is production-equal to
/// cwd; the simplification is registered):
///
/// - strip whitespace / wrapping quotes / trailing literal `\n \r \t`
///   escapes of a quote-wrapped arg (`sanitize_model_path_arg` mirror);
/// - expand a leading `~` / `~/` to `home` (`~user` is NOT expanded);
/// - rooted inputs (`has_root()` — on Windows `/foo` is rooted but not
///   absolute) are returned as-is, exactly like the tool resolver;
/// - relative inputs join onto `worktree`;
/// - `.` / `..` components are collapsed lexically (registered difference —
///   the digest covers the real touched object).
pub fn resolve_action_path(
    worktree: &Path,
    input: &str,
    home: Option<&Path>,
) -> Result<PathBuf, TargetResolveError> {
    Ok(parse_target(worktree, input, home)?.1)
}

/// Resolve AND IO-check the model-supplied target in one call — the
/// controller's consumption-point function (both the issue and the verify
/// side).
///
/// Review D1-1 (2026-08-12): the reparse scan runs on the UN-FOLDED
/// candidate. A `<junction>\..` spelling (e.g. `link\..\..\secret.txt`)
/// collapses lexically to a path that hides the junction — yet the OS
/// resolves `..` component-by-component, so the real write follows the
/// link first and lands elsewhere. Scanning the un-folded chain catches
/// the junction (its parent() chain still carries `link`); the folded
/// form is what gets bound and digested.
pub fn resolve_action_path_checked(
    worktree: &Path,
    input: &str,
    home: Option<&Path>,
) -> Result<PathBuf, TargetResolveError> {
    let (candidate, folded) = parse_target(worktree, input, home)?;
    if has_reparse_or_symlink_component(&candidate) {
        return Err(TargetResolveError::ReparseComponent);
    }
    Ok(folded)
}

/// Parse (no IO) into the raw candidate (pre-fold) and its folded form.
/// `resolve_action_path` returns the folded form; the checked variant scans
/// the raw candidate first (D1-1).
fn parse_target(
    worktree: &Path,
    input: &str,
    home: Option<&Path>,
) -> Result<(PathBuf, PathBuf), TargetResolveError> {
    let sanitized = sanitize_model_path_arg(input);
    if sanitized.is_empty() {
        return Err(TargetResolveError::Empty);
    }
    let expanded = tilde_expand_strict(sanitized, home).map_err(|e| match e {
        TildeExpandError::TildeUserUnsupported => TargetResolveError::TildeUserUnsupported,
    })?;
    let input_path = Path::new(&expanded);
    let candidate = if input_path.has_root() {
        input_path.to_path_buf()
    } else {
        // Mirror the tool resolver's "forgot leading slash" recovery
        // (orz-paths `resolve_lexical`, review D2-1 2026-08-12): a
        // relative spelling whose components repeat the worktree path
        // (`data/user/...` for worktree `/data/user/...`) binds the REAL
        // object, not a doubled phantom — the tool side would resolve the
        // same input to the real path, so a phantom binding would be
        // self-consistent on both ticket sides and escape target_mismatch
        // detection.
        let as_absolute = PathBuf::from(format!("/{expanded}"));
        if as_absolute.starts_with(worktree)
            && let Ok(suffix) = as_absolute.strip_prefix(worktree)
        {
            worktree.join(suffix)
        } else {
            worktree.join(input_path)
        }
    };
    reject_verbatim(&candidate)?;
    let folded = orz_paths::normalize_lexically(&candidate);
    Ok((candidate, folded))
}

/// IO check: does any component from the target itself up to the root carry a
/// reparse point or symlink? A new file's final component does not exist yet —
/// only its ancestors are checked (`symlink_metadata` on a missing path
/// yields Err → not a reparse). Single-sourced with `orz-host`
/// (2026-08-12): both consume `orz-paths::resolve::is_reparse_or_symlink`.
pub fn has_reparse_or_symlink_component(target: &Path) -> bool {
    let mut cur = target.to_path_buf();
    loop {
        if is_reparse_or_symlink(&cur) {
            return true;
        }
        let Some(parent) = cur.parent() else { break };
        if parent == cur {
            break;
        }
        cur = parent.to_path_buf();
    }
    false
}

/// The digest bound as `resolved_target_sha256`: the resolved path string
/// with separators unified to `/` (Windows may mix `\` and `/` after joins),
/// then SHA-256 hex. Case is preserved — Windows case normalisation is the
/// caller's job via `dunce::canonicalize` before digesting.
pub fn resolved_target_digest(path: &Path) -> String {
    let unified = path.to_string_lossy().replace('\\', "/");
    sha256_hex(unified.as_bytes())
}

/// `dunce::canonicalize` (Windows case / 8.3 short-name normalisation) when
/// the target exists — a new file keeps its lexical spelling. As a side
/// effect a symlink swapped in at resolution time resolves through to its
/// real target, so the digest would differ from the ticket binding.
pub fn canonicalize_if_exists(path: &Path) -> PathBuf {
    match dunce::canonicalize(path) {
        Ok(c) => c,
        Err(_) => path.to_path_buf(),
    }
}

/// Resolve a model-supplied URL argument to the canonical network target
/// (`network_v1` — web_fetch / browser_read): absolute http(s) URL with the
/// scheme/host lowercased by the parser, explicit default ports removed,
/// the fragment dropped (it is never sent), and embedded userinfo rejected
/// (the URL gates already refuse credentials — a credential-bearing URL is
/// unticketable, not bindable). Percent-encoding and path/query bytes are
/// preserved as the parser normalises them — the digest covers the exact
/// canonical string the request is built from.
pub fn resolve_network_url(input: &str) -> Result<String, TargetResolveError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(TargetResolveError::Empty);
    }
    let parsed = url::Url::parse(trimmed).map_err(|_| TargetResolveError::InvalidUrl)?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(TargetResolveError::UnsupportedScheme);
    }
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err(TargetResolveError::UrlCredentialsUnsupported);
    }
    let mut normalized = parsed.clone();
    normalized.set_fragment(None);
    let default_port = match normalized.scheme() {
        "http" => Some(80),
        "https" => Some(443),
        _ => None,
    };
    if normalized.port() == default_port {
        let _ = normalized.set_port(None);
    }
    Ok(normalized.to_string())
}

/// The digest bound as the `network_v1` resolved target: SHA-256 over the
/// canonical URL bytes (the parsed real object being contacted).
pub fn network_target_digest(canonical_url: &str) -> String {
    sha256_hex(canonical_url.as_bytes())
}

/// `symlink_metadata`-based reparse check — does NOT follow the link (a
/// junction reports `is_dir() == true` with `is_symlink() == false`, so the
/// reliable Windows signal is the FILE_ATTRIBUTE_REPARSE_POINT (0x400) bit on
/// the entry's own metadata). Thin shell over the single-sourced
/// `orz-paths::resolve::is_reparse_or_symlink` (2026-08-12).
pub fn is_reparse_or_symlink(path: &Path) -> bool {
    orz_paths::resolve::is_reparse_or_symlink(path)
}

#[cfg(windows)]
fn reject_verbatim(p: &Path) -> Result<(), TargetResolveError> {
    use std::path::Prefix;
    if let Some(Component::Prefix(prefix)) = p.components().next()
        && matches!(
            prefix.kind(),
            // DeviceNS (`\\.\C:\...`) also bypasses lexical normalisation —
            // review P2-1 (2026-08-12): it is a device path, not a file path.
            Prefix::Verbatim(_)
                | Prefix::VerbatimUNC(_, _)
                | Prefix::VerbatimDisk(_)
                | Prefix::DeviceNS(_)
        )
    {
        return Err(TargetResolveError::VerbatimUnsupported);
    }
    Ok(())
}

#[cfg(not(windows))]
fn reject_verbatim(_p: &Path) -> Result<(), TargetResolveError> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn worktree() -> PathBuf {
        PathBuf::from(r"C:\worktree")
    }

    fn home() -> PathBuf {
        PathBuf::from(r"C:\Users\test")
    }

    #[test]
    fn resolve_relative_joins_worktree() {
        assert_eq!(
            resolve_action_path(&worktree(), "src/main.rs", None).unwrap(),
            PathBuf::from(r"C:\worktree\src\main.rs")
        );
    }

    #[test]
    fn forgot_leading_slash_recovery_binds_real_object() {
        // Review D2-1 (2026-08-12): mirror the tool resolver's
        // forgot-leading-slash recovery (`orz-paths` `resolve_lexical`) — a
        // relative spelling whose components repeat the worktree path binds
        // the REAL object, not a doubled phantom (which would be
        // self-consistent on both ticket sides and escape target_mismatch
        // detection). Unix-style paths: the component-wise comparison holds
        // on both platforms.
        let worktree = Path::new("/data/user/workspace/repo/project");
        assert_eq!(
            resolve_action_path(
                worktree,
                "data/user/workspace/repo/project/src/main.rs",
                None
            )
            .unwrap(),
            PathBuf::from("/data/user/workspace/repo/project/src/main.rs")
        );
        // Exact worktree spelling (no suffix) → the worktree itself.
        assert_eq!(
            resolve_action_path(worktree, "data/user/workspace/repo/project", None).unwrap(),
            PathBuf::from("/data/user/workspace/repo/project")
        );
        // Unrelated relative paths are unaffected (normal join).
        assert_eq!(
            resolve_action_path(worktree, "src/main.rs", None).unwrap(),
            PathBuf::from("/data/user/workspace/repo/project/src/main.rs")
        );
    }

    #[test]
    fn resolve_dotted_and_escaped_inputs() {
        assert_eq!(
            resolve_action_path(&worktree(), "src/./lib/../main.rs", None).unwrap(),
            PathBuf::from(r"C:\worktree\src\main.rs")
        );
        // Quote-wrapped with a trailing literal \n escape — stripped.
        assert_eq!(
            resolve_action_path(&worktree(), r#""src/main.rs\n""#, None).unwrap(),
            PathBuf::from(r"C:\worktree\src\main.rs")
        );
        // Bare whitespace only.
        assert_eq!(
            resolve_action_path(&worktree(), "   ", None),
            Err(TargetResolveError::Empty)
        );
    }

    #[test]
    fn parent_dir_escapes_collapse() {
        // `..` collapses lexically — the digest covers the real touched
        // object (registered difference vs resolve_model_path, which keeps
        // the literal `..` component).
        assert_eq!(
            resolve_action_path(&worktree(), "../secret.txt", None).unwrap(),
            PathBuf::from(r"C:\secret.txt")
        );
    }

    #[cfg(windows)]
    #[test]
    fn fold_drive_relative_preserves_parent_prefix() {
        // Single-sourced fold (`orz_paths::normalize_lexically`,
        // 2026-08-12): a drive-relative `C:..\x` spelling KEEPS its `..`
        // (prefix-preserving) — the old local copy popped the prefix and
        // produced `C:x` (drive-relative). Conservative direction, locked
        // here; rooted escapes still collapse to the drive root.
        assert_eq!(
            resolve_action_path(&worktree(), r"C:..\outside.rs", None).unwrap(),
            PathBuf::from(r"C:..\outside.rs")
        );
        assert_eq!(
            resolve_action_path(&worktree(), r"C:\worktree\..\..\secret.txt", None).unwrap(),
            PathBuf::from(r"C:\secret.txt")
        );
    }

    #[test]
    fn tilde_expansion() {
        assert_eq!(
            resolve_action_path(&worktree(), "~/proj/a.rs", Some(&home())).unwrap(),
            PathBuf::from(r"C:\Users\test\proj\a.rs")
        );
        assert_eq!(
            resolve_action_path(&worktree(), "~", Some(&home())).unwrap(),
            home()
        );
        // No home known → literal tilde stays a relative path.
        assert_eq!(
            resolve_action_path(&worktree(), "~/x.rs", None).unwrap(),
            PathBuf::from(r"C:\worktree\~\x.rs")
        );
    }

    #[test]
    fn tilde_user_rejected() {
        // `~user` is ambiguous (shellexpand expands it on Unix only) — the
        // ticket side never binds it (review P1-2 2026-08-12, fail-closed).
        assert_eq!(
            resolve_action_path(&worktree(), "~other/x.rs", Some(&home())),
            Err(TargetResolveError::TildeUserUnsupported)
        );
        assert_eq!(
            resolve_action_path(&worktree(), "~other/x.rs", None),
            Err(TargetResolveError::TildeUserUnsupported)
        );
        assert_eq!(
            resolve_action_path_checked(&worktree(), "~other/x.rs", Some(&home())),
            Err(TargetResolveError::TildeUserUnsupported)
        );
    }

    #[cfg(windows)]
    #[test]
    fn windows_root_slash_semantics() {
        // `/etc/hosts` is rooted-but-not-absolute: returned as-is, exactly
        // like the tool resolver (registered simplification).
        assert_eq!(
            resolve_action_path(&worktree(), "/etc/hosts", None).unwrap(),
            PathBuf::from(r"/etc/hosts")
        );
        // Absolute Windows path passes through.
        assert_eq!(
            resolve_action_path(&worktree(), r"D:\other\file.txt", None).unwrap(),
            PathBuf::from(r"D:\other\file.txt")
        );
    }

    #[cfg(windows)]
    #[test]
    fn verbatim_prefix_rejected() {
        assert_eq!(
            resolve_action_path(&worktree(), r"\\?\C:\any", None),
            Err(TargetResolveError::VerbatimUnsupported)
        );
        assert_eq!(
            resolve_action_path(&worktree(), r"\\?\UNC\srv\share\f", None),
            Err(TargetResolveError::VerbatimUnsupported)
        );
        // DeviceNS (`\\.\C:\...`) also bypasses lexical normalisation
        // (review P2-1 2026-08-12).
        assert_eq!(
            resolve_action_path(&worktree(), r"\\.\C:\any", None),
            Err(TargetResolveError::VerbatimUnsupported)
        );
    }

    #[test]
    fn empty_input_rejected() {
        assert_eq!(
            resolve_action_path(&worktree(), "", None),
            Err(TargetResolveError::Empty)
        );
        assert_eq!(
            resolve_action_path(&worktree(), r#""""#, None),
            Err(TargetResolveError::Empty)
        );
    }

    #[cfg(not(windows))]
    #[test]
    fn unix_absolute_passthrough() {
        assert_eq!(
            resolve_action_path(Path::new("/srv/work"), "/etc/hosts", None).unwrap(),
            PathBuf::from("/etc/hosts")
        );
    }

    #[test]
    fn reparse_component_detected() {
        let tmp = tempfile::tempdir().unwrap();
        let real = tmp.path().join("real");
        std::fs::create_dir_all(&real).unwrap();
        let link = tmp.path().join("link");
        #[cfg(windows)]
        let link_ok = std::os::windows::fs::symlink_dir(&real, &link).is_ok();
        #[cfg(not(windows))]
        let link_ok = std::os::unix::fs::symlink(&real, &link).is_ok();
        if !link_ok {
            // Windows symlink_dir needs developer mode / privileges — skip.
            eprintln!("symlink creation unsupported, skipping");
            return;
        }
        // The link itself and anything beneath it is a reparse surface.
        assert!(has_reparse_or_symlink_component(&link.join("file.txt")));
        assert!(has_reparse_or_symlink_component(&link));
        // A plain directory is not.
        assert!(!has_reparse_or_symlink_component(&real.join("file.txt")));
        // A missing final component under a plain dir has no reparse surface.
        assert!(!has_reparse_or_symlink_component(
            &real.join("new-file.txt")
        ));
    }

    #[test]
    fn checked_resolve_scans_unfolded_candidate() {
        // Review D1-1 (2026-08-12): a `<link>\..` spelling collapses to a
        // path that hides the link — the CHECKED resolver scans the
        // un-folded chain and must catch it, while the lexical resolver
        // still folds (its digest covers the collapsed spelling).
        let tmp = tempfile::tempdir().unwrap();
        let real = tmp.path().join("real");
        std::fs::create_dir_all(&real).unwrap();
        let link = tmp.path().join("link");
        #[cfg(windows)]
        let link_ok = std::os::windows::fs::symlink_dir(&real, &link).is_ok();
        #[cfg(not(windows))]
        let link_ok = std::os::unix::fs::symlink(&real, &link).is_ok();
        if !link_ok {
            eprintln!("symlink creation unsupported, skipping");
            return;
        }
        // `link\..\..\secret.txt` folds to `parent\secret.txt` (the link is
        // hidden) — the checked resolver refuses, the lexical one folds.
        let escaped = format!(
            "link{}..{}..{}secret.txt",
            std::path::MAIN_SEPARATOR,
            std::path::MAIN_SEPARATOR,
            std::path::MAIN_SEPARATOR
        );
        assert_eq!(
            resolve_action_path_checked(tmp.path(), &escaped, None),
            Err(TargetResolveError::ReparseComponent)
        );
        // The lexical resolver still folds: the `link` component is gone and
        // the result escapes to the temp dir's parent (normalize_lexical
        // pops past the join base — same behaviour as the permission
        // resolver's lexical normalisation).
        let folded = resolve_action_path(tmp.path(), &escaped, None).unwrap();
        assert_eq!(folded.file_name().unwrap(), "secret.txt");
        assert!(
            !folded.components().any(|c| c.as_os_str() == "link"),
            "the link component must be folded away: {folded:?}"
        );
    }

    #[test]
    fn target_digest_stable_and_sensitive() {
        let a = resolved_target_digest(Path::new(r"C:\worktree\src\main.rs"));
        let b = resolved_target_digest(Path::new(r"C:/worktree/src/main.rs"));
        let c = resolved_target_digest(Path::new(r"C:\worktree\src\other.rs"));
        // Separator-unified digest: `\` and `/` spell the same target.
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_eq!(a.len(), 64);
        assert!(
            a.chars()
                .all(|ch| ch.is_ascii_hexdigit() && !ch.is_ascii_uppercase())
        );
    }

    #[test]
    fn network_url_canonicalisation() {
        // Default ports are removed, the fragment is dropped, the host is
        // lowercased, and an explicit non-default port survives.
        assert_eq!(
            resolve_network_url("HTTP://Example.COM:80/a/b?q=1#frag").unwrap(),
            "http://example.com/a/b?q=1"
        );
        assert_eq!(
            resolve_network_url("https://example.com:8443/x").unwrap(),
            "https://example.com:8443/x"
        );
        // The empty input and invalid URLs are unticketable.
        assert_eq!(resolve_network_url("   "), Err(TargetResolveError::Empty));
        assert_eq!(
            resolve_network_url("not a url"),
            Err(TargetResolveError::InvalidUrl)
        );
        // Only http(s) is a network-tool target.
        assert_eq!(
            resolve_network_url("file:///etc/passwd"),
            Err(TargetResolveError::UnsupportedScheme)
        );
        // Embedded credentials are refused — the URL gates already reject
        // them, so the ticket side never binds a credential-bearing URL.
        assert_eq!(
            resolve_network_url("https://user:pass@example.com/"),
            Err(TargetResolveError::UrlCredentialsUnsupported)
        );
        assert_eq!(
            resolve_network_url("https://user@example.com/"),
            Err(TargetResolveError::UrlCredentialsUnsupported)
        );
    }

    #[test]
    fn network_target_digest_is_stable_and_sensitive() {
        let a = network_target_digest("https://example.com/a");
        let b = network_target_digest("https://example.com/a");
        let c = network_target_digest("https://example.com/b");
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_eq!(a.len(), 64);
        assert!(
            a.chars()
                .all(|ch| ch.is_ascii_hexdigit() && !ch.is_ascii_uppercase())
        );
    }
}
