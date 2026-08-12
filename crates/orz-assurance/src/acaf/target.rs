//! ACAF action target resolution (Slice 2 first phase, ADR-0011 §4.2/§4.5) —
//! the *parsed real target object* for action tickets: the resolved file
//! path for `file_write_v1`, the credential target for `credential_read_v1`.
//!
//! `resolved_target_sha256` binds the digest of this parsed object, and the
//! consumption point re-parses the live object and re-derives the digest
//! (check 5 TOCTOU — never trusts the ticket's own values).
//!
//! The lexical rules mirror the observable semantics of
//! `resolve_model_path` (orz-tools `types/resources.rs`) with three
//! registered differences: (1) verbatim/device (`\\?\`, `\\?\UNC\`,
//! `\\.\`) prefixes are rejected outright (they bypass lexical
//! normalisation); (2) `..` components are collapsed lexically — the
//! reparse scan runs on the UN-FOLDED candidate first so a
//! `<junction>\..` spelling cannot hide the link (review D1-1 2026-08-12);
//! (3) `~user` is refused (review P1-2 — shellexpand expands it on Unix
//! but not Windows; the ambiguous form is never ticketable). The two
//! resolvers are semantically mirrored, not single-sourced
//! (orz-assurance must not depend on orz-tools) — a divergence surfaces
//! as a ticket `target_mismatch` in shadow mode (registered drift
//! surface).

use std::path::{Component, Path, PathBuf};

use crate::journal::sha256_hex;

/// Why a model-supplied target could not be resolved.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TargetResolveError {
    #[error("target path is empty")]
    Empty,
    #[error("verbatim/device (\\\\?\\, \\\\?\\UNC\\, \\\\.\\) paths are not supported for action tickets")]
    VerbatimUnsupported,
    #[error("target path contains a reparse point or symlink component")]
    ReparseComponent,
    #[error("~user-style paths are not supported for action tickets (ambiguous expansion)")]
    TildeUserUnsupported,
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
    let expanded = expand_tilde(sanitized, home)?;
    let input_path = Path::new(&expanded);
    let candidate = if input_path.has_root() {
        input_path.to_path_buf()
    } else {
        worktree.join(input_path)
    };
    reject_verbatim(&candidate)?;
    let folded = normalize_lexical(&candidate);
    Ok((candidate, folded))
}

/// IO check: does any component from the target itself up to the root carry a
/// reparse point or symlink? A new file's final component does not exist yet —
/// only its ancestors are checked (`symlink_metadata` on a missing path
/// yields Err → not a reparse). Mirrors `orz-host` `is_reparse_or_symlink`
/// (Windows junction/reparse-point 0x400 bit + symlink); registered duplicate
/// implementation, to be single-sourced when the host moves onto this crate.
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

/// `symlink_metadata`-based reparse check — does NOT follow the link (a
/// junction reports `is_dir() == true` with `is_symlink() == false`, so the
/// reliable Windows signal is the FILE_ATTRIBUTE_REPARSE_POINT (0x400) bit on
/// the entry's own metadata).
pub fn is_reparse_or_symlink(path: &Path) -> bool {
    let Ok(md) = std::fs::symlink_metadata(path) else {
        // Missing (or unreadable) → not a reparse (new-file scenario).
        return false;
    };
    if md.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        md.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        false
    }
}

fn sanitize_model_path_arg(input: &str) -> &str {
    let trimmed = input.trim();
    let quote_wrapped =
        trimmed.len() >= 2 && trimmed.starts_with(['"', '\'']) && trimmed.ends_with(['"', '\'']);
    let unquoted = trimmed.trim_matches(['"', '\'']).trim();
    if !quote_wrapped {
        return unquoted;
    }
    let mut result = unquoted;
    while let Some(stripped) = result
        .strip_suffix("\\n")
        .or_else(|| result.strip_suffix("\\r"))
        .or_else(|| result.strip_suffix("\\t"))
    {
        result = stripped.trim_end();
    }
    result
}

/// `~` / `~/` → home (like `shellexpand::tilde`). `~user` is REFUSED
/// (review P1-2 2026-08-12): shellexpand expands `~user` on Unix (getpwnam)
/// but leaves it literal on Windows — mirroring either side forks behaviour,
/// so the ticket side never binds the ambiguous form (fail-closed; a `~user`
/// target is never ticketable). `home == None` leaves `~`/`~/` in place
/// (treated as a relative path).
fn expand_tilde(input: &str, home: Option<&Path>) -> Result<String, TargetResolveError> {
    if let Some(home) = home {
        if input == "~" {
            return Ok(home.to_string_lossy().into_owned());
        }
        if let Some(rest) = input.strip_prefix("~/") {
            return Ok(home.join(rest).to_string_lossy().into_owned());
        }
    }
    if input.starts_with('~') && input != "~" && !input.starts_with("~/") {
        return Err(TargetResolveError::TildeUserUnsupported);
    }
    Ok(input.to_string())
}

/// Lexically resolve `.` / `..` components (mirror of orz-host
/// `permission::normalize_lexical`).
fn normalize_lexical(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
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
        assert!(!has_reparse_or_symlink_component(&real.join("new-file.txt")));
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
        let escaped = format!("link{}..{}..{}secret.txt", std::path::MAIN_SEPARATOR, std::path::MAIN_SEPARATOR, std::path::MAIN_SEPARATOR);
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
        assert!(a.chars().all(|ch| ch.is_ascii_hexdigit() && !ch.is_ascii_uppercase()));
    }
}
