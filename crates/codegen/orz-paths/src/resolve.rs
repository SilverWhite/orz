//! Single-source model path-resolution primitives (design doc §4.5 — the
//! ACAF target-resolution mirror; ADR-0011 决策 3).
//!
//! Three consumers share these primitives (2026-08-12 single-sourcing
//! decision):
//!
//! - [`resolve_lexical`] + [`tilde_expand`] + [`sanitize_model_path_arg`]:
//!   the lenient tool-side semantics — `orz-tools` `resolve_model_path`
//!   becomes a thin shell over these three, byte-for-byte unchanged;
//! - [`tilde_expand_strict`]: the ticket-side tilde semantics —
//!   `orz-assurance` `acaf::target` maps its `TildeUserUnsupported` error
//!   onto this; it deliberately does NOT use `shellexpand` (the join must
//!   use `PathBuf` separator semantics so the digest covers the exact bytes
//!   the ticket binds);
//! - [`is_reparse_or_symlink`]: shared by the ticket-side reparse scan
//!   (`acaf::target`) and the host-side run-tests workspace-delta walk
//!   (previously two copies: `orz-host` lib.rs and `orz-assurance`).
//!
//! Deliberately NOT here (registered boundaries): `..` folding
//! ([`crate::normalize_lexically`] is used by consumers who fold, with the
//! drive-relative `C:..\x` prefix-preserving difference registered against
//! the old `acaf::target` copy), verbatim/device-prefix rejection, and the
//! resolved-target digest (SHA-256 is a ticket concern, stays in
//! `orz-assurance`).

use std::borrow::Cow;
use std::path::{Path, PathBuf};

/// Strip surrounding whitespace (e.g. a trailing newline from block-form
/// tool args) and quotes that models occasionally emit around path args.
///
/// When the arg was quote-wrapped, the model emitted a *string literal* (e.g.
/// a JSON-style `"/path/file.ts\n"` pasted into a block-form arg where no
/// JSON unescaping ever runs). In that case also strip trailing **literal**
/// escape sequences (`\n`, `\r`, `\t` as two characters) left at the end of
/// the unquoted value — `str::trim` only removes real whitespace, so the
/// resolved path would otherwise end in a literal backslash-n and miss the
/// file. Escape stripping requires the trimmed arg to both *start and end*
/// with a quote character (true quote-wrapping): a stray unbalanced quote is
/// still stripped, but does not enable escape stripping, so backslashes in
/// otherwise-unquoted real paths (e.g. Windows `dir\n ame`) are never eaten.
pub fn sanitize_model_path_arg(input: &str) -> &str {
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

/// Lenient tilde expansion: `shellexpand::tilde` semantics — leading `~` /
/// `~/` expands to the current user's home directory; `~user` is expanded on
/// Unix (getpwnam) and left literal on Windows.
pub fn tilde_expand(input: &str) -> Cow<'_, str> {
    shellexpand::tilde(input)
}

/// Error from [`tilde_expand_strict`]: a `~user`-style path was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TildeExpandError {
    #[error("~user-style paths are not supported (ambiguous expansion)")]
    TildeUserUnsupported,
}

/// Strict tilde expansion — the ticket-side semantics (`orz-assurance`
/// `acaf::target` mirror, byte-identical):
///
/// - `home = Some` and input `~` → the home path itself;
/// - `home = Some` and input `~/<rest>` → `home.join(rest)` (`PathBuf`
///   separator semantics — deliberately NOT `shellexpand`, which
///   string-concatenates and can mix `/` and `\` on Windows; the digest
///   covers the exact bytes bound);
/// - `~user` (any other leading-`~` spelling) is REFUSED in all cases —
///   `shellexpand` expands `~user` on Unix (getpwnam) but leaves it literal
///   on Windows, so mirroring either side forks behaviour; a `~user` target
///   is never ticketable (fail-closed);
/// - `home = None` leaves `~` / `~/` in place (treated as a relative path).
pub fn tilde_expand_strict(
    input: &str,
    home: Option<&Path>,
) -> Result<String, TildeExpandError> {
    if let Some(home) = home {
        if input == "~" {
            return Ok(home.to_string_lossy().into_owned());
        }
        if let Some(rest) = input.strip_prefix("~/") {
            return Ok(home.join(rest).to_string_lossy().into_owned());
        }
    }
    if input.starts_with('~') && input != "~" && !input.starts_with("~/") {
        return Err(TildeExpandError::TildeUserUnsupported);
    }
    Ok(input.to_string())
}

/// Resolve a (already sanitized + tilde-expanded) model-provided path,
/// rewriting absolute paths from conversation history when `display_cwd` is
/// set. This is the exact body of `orz-tools` `resolve_model_path` minus the
/// sanitize/tilde stages — lenient: `..` components are NOT folded, verbatim
/// paths are NOT rejected, no reparse check.
///
/// - If `display_cwd` is `None`, falls back to `cwd.join(input)`.
/// - If `input` starts with the `display_cwd` prefix, strips it and joins
///   the suffix onto `cwd` (the real worktree path).
/// - If `input` is absolute but doesn't match, returns it as-is.
/// - Relative paths are always joined onto `cwd`.
///
/// `has_root()` is used, not `is_absolute()`: on Windows `/foo` is rooted but
/// has no drive prefix, so `is_absolute()` is false — yet it is the model's
/// absolute/display form and must get the same display-strip treatment.
pub fn resolve_lexical(cwd: &Path, display_cwd: Option<&Path>, input: &str) -> PathBuf {
    let input_path = Path::new(input);
    if let Some(display) = display_cwd
        && input_path.has_root()
    {
        if let Ok(suffix) = input_path.strip_prefix(display) {
            return cwd.join(suffix);
        }
        return input_path.to_path_buf();
    }
    if !input_path.has_root() && !input.is_empty() {
        let as_absolute = PathBuf::from(format!("/{input}"));
        let effective_base = display_cwd.unwrap_or(cwd);
        if as_absolute.starts_with(effective_base)
            && let Ok(suffix) = as_absolute.strip_prefix(effective_base)
        {
            return cwd.join(suffix);
        }
    }
    // Rooted inputs are absolute-form: return as-is. On Windows `cwd.join`
    // would keep the cwd's drive prefix (`C:\etc\hosts` from `/etc/hosts`),
    // silently retargeting the path — a security-relevant miss for the
    // permission resolver.
    if input_path.has_root() {
        return input_path.to_path_buf();
    }
    cwd.join(input_path)
}

/// `symlink_metadata`-based reparse check — does NOT follow the link (a
/// junction reports `is_dir() == true` with `is_symlink() == false`, so the
/// reliable Windows signal is the FILE_ATTRIBUTE_REPARSE_POINT (0x400) bit on
/// the entry's own metadata). Missing (or unreadable) path → false
/// (new-file scenario). Single implementation shared by the ACAF ticket-side
/// reparse scan and the host-side run-tests workspace-delta walk
/// (single-sourced 2026-08-12; the orz-host copy additionally skipped a
/// `symlink_metadata` call on Unix by reusing the `read_dir` entry type — the
/// extra syscall per entry is semantically equivalent, registered).
pub fn is_reparse_or_symlink(path: &Path) -> bool {
    let Ok(md) = std::fs::symlink_metadata(path) else {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_strips_quotes_and_literal_escapes() {
        assert_eq!(sanitize_model_path_arg("\"a.rs\""), "a.rs");
        assert_eq!(sanitize_model_path_arg("'a.rs'"), "a.rs");
        assert_eq!(sanitize_model_path_arg("\"a.rs\\n\""), "a.rs");
        assert_eq!(sanitize_model_path_arg("\"a.rs\\r\\n\""), "a.rs");
        assert_eq!(sanitize_model_path_arg("\"a.rs\\t\""), "a.rs");
        assert_eq!(sanitize_model_path_arg("  a.rs  "), "a.rs");
    }

    #[test]
    fn sanitize_unbalanced_quote_keeps_literal_escapes() {
        // A stray leading quote is stripped but does NOT enable escape
        // stripping — backslashes in real paths are never eaten.
        assert_eq!(sanitize_model_path_arg("\"a.rs\\n"), "a.rs\\n");
        assert_eq!(sanitize_model_path_arg("dir\\name"), "dir\\name");
    }

    #[test]
    fn tilde_expand_plain_passthrough() {
        assert_eq!(tilde_expand("plain/path"), "plain/path");
        assert_eq!(tilde_expand(""), "");
    }

    #[cfg(windows)]
    #[test]
    fn tilde_expand_strict_home_expansion() {
        let home = Path::new(r"C:\Users\test");
        assert_eq!(
            tilde_expand_strict("~", Some(home)).unwrap(),
            r"C:\Users\test"
        );
        // PathBuf join semantics: `\` between joined components, `/` inside
        // the joined string preserved — the exact bytes the ticket digest
        // covers (shellexpand string-concats to `C:\Users\test/proj/a.rs` —
        // deliberately different).
        assert_eq!(
            tilde_expand_strict("~/proj/a.rs", Some(home)).unwrap(),
            r"C:\Users\test\proj/a.rs"
        );
    }

    #[cfg(not(windows))]
    #[test]
    fn tilde_expand_strict_home_expansion() {
        let home = Path::new("/home/test");
        assert_eq!(tilde_expand_strict("~", Some(home)).unwrap(), "/home/test");
        assert_eq!(
            tilde_expand_strict("~/a/b.rs", Some(home)).unwrap(),
            "/home/test/a/b.rs"
        );
    }

    #[test]
    fn tilde_expand_strict_user_refused() {
        let home = Path::new("/home/test");
        assert_eq!(
            tilde_expand_strict("~other/a.rs", Some(home)),
            Err(TildeExpandError::TildeUserUnsupported)
        );
        // Also refused when no home is available (fail-closed).
        assert_eq!(
            tilde_expand_strict("~other/a.rs", None),
            Err(TildeExpandError::TildeUserUnsupported)
        );
    }

    #[test]
    fn tilde_expand_strict_no_home_leaves_tilde() {
        // `~` / `~/` with home=None are treated as a relative path.
        assert_eq!(tilde_expand_strict("~", None).unwrap(), "~");
        assert_eq!(tilde_expand_strict("~/x", None).unwrap(), "~/x");
    }

    #[cfg(windows)]
    #[test]
    fn tilde_expand_strict_join_keeps_native_separator() {
        let home = Path::new(r"C:\Users\test");
        assert_eq!(
            tilde_expand_strict("~/proj/a.rs", Some(home)).unwrap(),
            r"C:\Users\test\proj/a.rs"
        );
    }

    #[test]
    fn resolve_lexical_relative_joins() {
        let cwd = Path::new("/work");
        assert_eq!(
            resolve_lexical(cwd, None, "src/main.rs"),
            PathBuf::from("/work/src/main.rs")
        );
        // Dot-dot components are NOT folded (lenient tool semantics).
        assert_eq!(
            resolve_lexical(cwd, None, "a/../b.rs"),
            PathBuf::from("/work/a/../b.rs")
        );
    }

    #[test]
    fn resolve_lexical_empty_joins_cwd() {
        assert_eq!(resolve_lexical(Path::new("/work"), None, ""), PathBuf::from("/work"));
    }

    #[test]
    fn resolve_lexical_rooted_passthrough() {
        let cwd = Path::new("/work");
        assert_eq!(
            resolve_lexical(cwd, None, "/etc/hosts"),
            PathBuf::from("/etc/hosts")
        );
        assert_eq!(
            resolve_lexical(cwd, Some(Path::new("/display")), "/etc/hosts"),
            PathBuf::from("/etc/hosts")
        );
    }

    #[test]
    fn resolve_lexical_display_strip() {
        let cwd = Path::new("/real/work");
        let display = Path::new("/display/proj");
        assert_eq!(
            resolve_lexical(cwd, Some(display), "/display/proj/src/a.rs"),
            PathBuf::from("/real/work/src/a.rs")
        );
        // Absolute but not under display → as-is.
        assert_eq!(
            resolve_lexical(cwd, Some(display), "/elsewhere/a.rs"),
            PathBuf::from("/elsewhere/a.rs")
        );
    }

    #[test]
    fn resolve_lexical_forgot_leading_slash_recovery() {
        // The model dropped the leading `/` of the cwd path itself —
        // compare the slash-prefixed spelling against the base and strip it.
        let cwd = Path::new("/data/user/workspace/repo/project");
        assert_eq!(
            resolve_lexical(cwd, None, "data/user/workspace/repo/project"),
            cwd
        );
        assert_eq!(
            resolve_lexical(cwd, None, "data/user/workspace/repo/project/src/main.rs"),
            PathBuf::from("/data/user/workspace/repo/project/src/main.rs")
        );
        // Same pattern with display_cwd set.
        let display = Path::new("/home/user/project");
        assert_eq!(
            resolve_lexical(Path::new("/worktree/abc"), Some(display), "home/user/project/src/main.rs"),
            PathBuf::from("/worktree/abc/src/main.rs")
        );
        // Non-matching relative stays cwd-joined (no recovery).
        assert_eq!(
            resolve_lexical(cwd, None, "src/main.rs"),
            PathBuf::from("/data/user/workspace/repo/project/src/main.rs")
        );
    }

    #[test]
    fn resolve_lexical_bare_colon_and_colon_prefixed() {
        // `:` (bare) and `:/display/...` (colon before display path) are NOT
        // absolute — treated as relative and joined (Kimi colon idiom).
        let cwd = Path::new("/worktree/abc");
        assert_eq!(resolve_lexical(cwd, None, ":"), PathBuf::from("/worktree/abc/:"));
        assert_eq!(
            resolve_lexical(cwd, None, ":/testbed/cache/cache.go"),
            PathBuf::from("/worktree/abc/:/testbed/cache/cache.go")
        );
    }

    #[test]
    fn is_reparse_or_symlink_plain_dir_and_missing() {
        let dir = std::env::temp_dir().join("orz-paths-resolve-test-dir");
        std::fs::create_dir_all(&dir).unwrap();
        assert!(!is_reparse_or_symlink(&dir));
        std::fs::remove_dir_all(&dir).unwrap();
        // Missing path → false (new-file scenario).
        assert!(!is_reparse_or_symlink(&dir));
    }

    #[cfg(unix)]
    #[test]
    fn is_reparse_or_symlink_detects_symlink() {
        use std::os::unix::fs::symlink;
        let dir = std::env::temp_dir().join("orz-paths-resolve-test-link");
        let target = dir.with_extension("target");
        std::fs::create_dir_all(&target).unwrap();
        symlink(&target, &dir).unwrap();
        assert!(is_reparse_or_symlink(&dir));
        std::fs::remove_file(&dir).unwrap();
        std::fs::remove_dir_all(&target).unwrap();
    }
}
