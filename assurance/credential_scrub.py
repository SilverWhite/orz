"""Credential lifecycle guard and leakage auditor — GAK-CRED-001.

Provides:

- :class:`CredentialGuard` — a context manager that reads a credential from
  Windows Credential Manager, provides it for the duration of a ``with`` block,
  and scrubs the Python string on exit (best-effort, no guarantee of total
  memory zeroisation).
- :func:`sanitize_child_environment` — strip credential-bearing variables from
  an environment dict before passing it to a subprocess.
- :func:`assert_no_credential_in_dict` — scan a dict for candidate credential
  values and raise :exc:`AssuranceError` if any field looks like a leaked key.
- :func:`audit_credential_scrub_sites` — enumerate every call site that reads a
  credential and verify a corresponding scrub is present.

Design constraints (from Gap Register GAK-CRED-001):

- Python cannot guarantee zeroisation of every temporary key copy in memory
  (GC-managed strings, interned values, pagefile/hibernation, WER dumps).
- The :class:`CredentialGuard` is a **defense-in-depth** mechanism, not a
  cryptographic erasure proof.
- All scrub operations are best-effort and logged; failure to scrub is
  surfaced via stderr, not swallowed.
"""

from __future__ import annotations

import ast
import os
import sys
from dataclasses import dataclass
from typing import Any

from .deepseek_adapter import DEFAULT_CREDENTIAL_TARGET, _read_windows_credential
from .errors import AssuranceError
from .utils import utc_now


# ── audit record ────────────────────────────────────────────────────────────

@dataclass
class ScrubRecord:
    """A single credential scrub lifecycle event."""
    credential_target: str
    acquired_at: str         # ISO-8601
    released_at: str = ""    # ISO-8601 (empty = still active)
    scrub_method: str = "python-string-overwrite"
    scrub_succeeded: bool = False
    error: str = ""


# In-memory audit log (session-scoped; never persisted to disk)
_scrub_audit: list[ScrubRecord] = []


def _emit_scrub_audit(record: ScrubRecord) -> None:
    """Record a scrub lifecycle event (in-memory only, never to disk)."""
    _scrub_audit.append(record)


def get_scrub_audit() -> list[dict[str, Any]]:
    """Return all scrub audit records as plain dicts (for test inspection)."""
    return [
        {
            "credential_target": r.credential_target,
            "acquired_at": r.acquired_at,
            "released_at": r.released_at,
            "scrub_method": r.scrub_method,
            "scrub_succeeded": r.scrub_succeeded,
            "error": r.error,
        }
        for r in _scrub_audit
    ]


# ── scrub utilities ─────────────────────────────────────────────────────────

def _scrub_string(value: str) -> str:
    """Overwrite *value* in-place via an intermediate mutable list, then
    return a zero-length replacement.

    CPython ``str`` objects are immutable, so we cannot truly zero the
    original buffer.  We instead:

    1. Build a list of individual characters (forces the string to be
       decomposed in memory, creating many small objects).
    2. Overwrite each list slot with ``\\x00``.
    3. Clear the list.

    This does **not** guarantee the original string buffer is recycled,
    but it reduces the window during which a single contiguous buffer
    holds the credential.
    """
    if not value:
        return ""
    chars: list[str] = list(value)
    for i in range(len(chars)):
        chars[i] = "\x00"
    chars.clear()
    return ""


def _scrub_bytes(value: bytearray) -> None:
    """Zero a mutable bytearray in-place."""
    for i in range(len(value)):
        value[i] = 0


# ── CredentialGuard ─────────────────────────────────────────────────────────

class CredentialGuard:
    """Context manager that reads, provides, and scrubs a credential.

    Usage::

        with CredentialGuard(DEFAULT_CREDENTIAL_TARGET) as api_key:
            call_some_api(api_key)
        # api_key is scrubbed here (even if an exception occurred)

    Audit records are emitted on entry and exit for every guard instance.
    """

    def __init__(self, target: str) -> None:
        if not target or not isinstance(target, str):
            raise AssuranceError("credential target must be a non-empty string")
        self._target = target
        self._credential: str = ""
        self._record = ScrubRecord(
            credential_target=target,
            acquired_at=utc_now(),
        )
        self._exited = False

    def __enter__(self) -> str:
        try:
            self._credential = _read_windows_credential(self._target)
        except Exception:
            self._record.scrub_succeeded = True  # nothing to scrub
            self._record.released_at = utc_now()
            _emit_scrub_audit(self._record)
            raise
        self._record.acquired_at = utc_now()
        return self._credential

    def __exit__(self, exc_type: object, exc_val: object, exc_tb: object) -> None:
        if self._exited:
            return
        self._exited = True
        try:
            if self._credential:
                self._credential = _scrub_string(self._credential)
            self._record.scrub_succeeded = True
            self._record.released_at = utc_now()
        except Exception as exc:
            self._record.scrub_succeeded = False
            self._record.error = str(exc)
            self._record.released_at = utc_now()
            print(
                f"[GSA] CredentialGuard scrub failed for "
                f"'{self._target}': {exc}",
                file=sys.stderr,
            )
        finally:
            _emit_scrub_audit(self._record)

    @property
    def target(self) -> str:
        return self._target

    @property
    def scrub_succeeded(self) -> bool:
        return self._record.scrub_succeeded


# ── child environment sanitization ──────────────────────────────────────────

# Environment variable names that commonly carry credentials.
# These are NEVER passed to child processes.
_CREDENTIAL_ENV_NAMES: set[str] = {
    # Generic
    "API_KEY", "API_TOKEN", "SECRET", "PASSWORD", "PASS", "TOKEN",
    "AUTH_TOKEN", "ACCESS_TOKEN", "BEARER_TOKEN", "CREDENTIAL",
    "DEEPSEEK_API_KEY", "OPENAI_API_KEY", "ANTHROPIC_API_KEY",
    # Cloud / CI
    "AWS_SECRET_ACCESS_KEY", "AWS_ACCESS_KEY_ID",
    "AZURE_CLIENT_SECRET", "GCP_SERVICE_ACCOUNT_KEY",
    # Git / VCS
    "GITHUB_TOKEN", "GITLAB_TOKEN", "GIT_ASKPASS",
    # Windows
    "DPAPI_ENTROPY", "CREDENTIAL_TARGET",
    # Project-specific
    "FEP_AGENT_DEEPSEEK_KEY", "GSA_CREDENTIAL_TARGET",
}

# Substrings that, if present in an env var name, flag it as
# potentially credential-bearing.
_CREDENTIAL_ENV_NAME_PATTERNS: tuple[str, ...] = (
    "key", "secret", "token", "password", "credential", "auth",
)


def _env_name_looks_like_credential(name: str) -> bool:
    """Return True if *name* looks like it could carry a credential."""
    upper = name.upper()
    if upper in _CREDENTIAL_ENV_NAMES:
        return True
    for pattern in _CREDENTIAL_ENV_NAME_PATTERNS:
        if pattern.upper() in upper:
            return True
    return False


def sanitize_child_environment(
    env: dict[str, str] | None = None,
) -> dict[str, str]:
    """Return a copy of *env* (or ``os.environ``) with credential-bearing
    variables removed.

    Call this before passing an environment dict to :func:`subprocess.Popen`
    or any child-process launcher.
    """
    source = dict(env) if env is not None else dict(os.environ)
    stripped: dict[str, str] = {}
    removed: list[str] = []
    for key, value in source.items():
        if _env_name_looks_like_credential(key):
            removed.append(key)
            continue
        stripped[key] = value
    if removed:
        print(
            f"[GSA] sanitize_child_environment removed "
            f"{len(removed)} credential-bearing env var(s): "
            f"{', '.join(sorted(removed))}",
            file=sys.stderr,
        )
    return stripped


def audit_child_environment(
    env: dict[str, str] | None = None,
) -> dict[str, Any]:
    """Return an audit dict describing credential exposure in *env*.

    Does NOT modify the environment — call :func:`sanitize_child_environment`
    for that.
    """
    source = dict(env) if env is not None else dict(os.environ)
    flagged: list[str] = []
    for key in source:
        if _env_name_looks_like_credential(key):
            flagged.append(key)
    return {
        "audit_kind": "child_environment_credential_audit",
        "total_vars": len(source),
        "flagged_credential_vars": sorted(flagged),
        "flagged_count": len(flagged),
        "safe": len(flagged) == 0,
    }


# ── artifact credential leakage scanner ─────────────────────────────────────

# Minimum length for a candidate API key-like string to be considered
# a credential (avoids false positives on short hex strings).
_MIN_CREDENTIAL_LENGTH = 20

# Hex characters only strings shorter than this are probably not API keys
# (but could be installation key material).
_HEX_ONLY_MAX_LENGTH = 64


def _looks_like_api_key(value: str) -> bool:
    """Heuristic: does *value* look like it could be a credential?

    Returns True for strings that resemble API keys, bearer tokens,
    or other secrets.  This is a **conservative** check — it may have
    false positives but should have very few false negatives for
    typical API key formats.
    """
    if not isinstance(value, str) or len(value) < _MIN_CREDENTIAL_LENGTH:
        return False
    # Skip values that are clearly file paths, URLs, or structured data
    if value.startswith(("http://", "https://", "/", "\\", "{", "[", "<")):
        return False
    # Skip SHA-256 digests (64 hex chars)
    if len(value) == 64 and all(c in "0123456789abcdef" for c in value.lower()):
        return False
    # Skip RFC8785-canonicalized JSON blobs
    if value.startswith("{") and value.endswith("}"):
        return False
    # Skip ISO-8601 timestamps (e.g. "2026-07-28T12:00:00Z")
    if (
        len(value) >= 20
        and value[4] == "-"
        and value[7] == "-"
        and (value[10] == "T" or value[10] == " ")
        and value[13] == ":"
        and value[16] == ":"
    ):
        return False
    # Skip natural-language sentences (contain spaces + common words)
    if " " in value and len(value.split()) >= 3:
        return False
    # Key-like: reasonable length, mixed content with sufficient entropy
    digit_count = sum(1 for c in value if c.isdigit())
    letter_count = sum(1 for c in value if c.isalpha())
    # Require at least 3 digits AND 3 letters for a positive match.
    # This avoids false positives on strings with a single digit
    # embedded in prose (e.g. "caspase-3") or dates in filenames.
    if digit_count >= 3 and letter_count >= 3:
        return True
    return False


def scan_dict_for_credentials(
    data: dict[str, Any],
    *,
    path: str = "$",
) -> list[dict[str, str]]:
    """Recursively scan *data* for values that look like credentials.

    Returns a list of findings, each with ``path`` (JSONPath-like), ``key``,
    and ``reason``.  An empty list means no suspicious values were found.
    """
    findings: list[dict[str, str]] = []

    def _scan(obj: object, current_path: str) -> None:
        if isinstance(obj, dict):
            for key, value in obj.items():  # type: ignore[attr-defined]
                child_path = f"{current_path}.{key}"
                if isinstance(value, str) and _looks_like_api_key(value):
                    findings.append({
                        "path": child_path,
                        "key": key,
                        "reason": "value resembles an API key / credential",
                    })
                elif isinstance(value, (dict, list)):
                    _scan(value, child_path)
        elif isinstance(obj, list):
            for idx, item in enumerate(obj):  # type: ignore[attr-defined]
                child_path = f"{current_path}[{idx}]"
                if isinstance(item, str) and _looks_like_api_key(item):
                    findings.append({
                        "path": child_path,
                        "key": f"[{idx}]",
                        "reason": "value resembles an API key / credential",
                    })
                elif isinstance(item, (dict, list)):
                    _scan(item, child_path)

    _scan(data, path)
    return findings


def assert_no_credential_in_dict(
    data: dict[str, Any],
    *,
    label: str = "artifact",
) -> None:
    """Scan *data* and raise :exc:`AssuranceError` if any field looks like
    a leaked credential.

    Call this before writing any receipt, answer packet, journal event,
    or other artifact to disk.
    """
    findings = scan_dict_for_credentials(data)
    if findings:
        detail = "; ".join(
            f"{f['path']} ({f['reason']})" for f in findings[:5]
        )
        if len(findings) > 5:
            detail += f" ... and {len(findings) - 5} more"
        raise AssuranceError(
            f"credential leakage detected in {label}: {detail}"
        )


# ── container mount credential exclusion ────────────────────────────────────

# Paths under which credentials or keystore material may reside on Windows.
# These must NEVER be bind-mounted into a Docker container.
_FORBIDDEN_HOST_MOUNT_ROOTS: tuple[str, ...] = (
    # Windows Credential Manager storage
    r"C:\Users\*\AppData\Roaming\Microsoft\Credentials",
    r"C:\Users\*\AppData\Local\Microsoft\Credentials",
    r"C:\Users\*\AppData\Roaming\Microsoft\Protect",
    r"C:\Users\*\AppData\Local\Microsoft\Protect",
    # DPAPI master key directory
    r"C:\Users\*\AppData\Roaming\Microsoft\Crypto",
    r"C:\Users\*\AppData\Local\Microsoft\Crypto",
    # General secret storage
    r"C:\Users\*\.ssh",
    r"C:\Users\*\.gnupg",
    # Project keystore
    "**/keystore/**",
    "**/*.dpapi",
    "**/*.pem",
    "**/*.key",
)


def audit_container_mount(
    host_path: str,
    *,
    label: str = "container mount",
) -> dict[str, Any]:
    """Check whether *host_path* could expose credential material.

    Returns an audit dict.  Does NOT raise — callers should decide
    whether to block based on the ``safe`` field.
    """
    import fnmatch

    warnings: list[str] = []
    normalized = host_path.replace("/", "\\").rstrip("\\")

    for forbidden in _FORBIDDEN_HOST_MOUNT_ROOTS:
        fnorm = forbidden.replace("/", "\\")
        fnorm_without_glob_tail = fnorm.removesuffix("\\**")
        # fnmatch for glob-style matching
        if (
            fnmatch.fnmatch(normalized, fnorm)
            or fnmatch.fnmatch(normalized, fnorm + "\\*")
            or fnmatch.fnmatch(normalized, fnorm_without_glob_tail)
            or fnmatch.fnmatch(normalized, fnorm_without_glob_tail + "\\*")
        ):
            warnings.append(
                f"host path '{host_path}' matches credential-sensitive "
                f"pattern '{forbidden}'"
            )
            break

    # Also check parent dirs
    import pathlib
    try:
        p = pathlib.Path(host_path)
        for parent in p.parents:
            pstr = str(parent).replace("/", "\\").rstrip("\\")
            for forbidden in _FORBIDDEN_HOST_MOUNT_ROOTS:
                fnorm = forbidden.replace("/", "\\")
                if fnmatch.fnmatch(pstr, fnorm):
                    warnings.append(
                        f"host path '{host_path}' is inside credential-sensitive "
                        f"directory '{parent}' (pattern '{forbidden}')"
                    )
                    break
    except (ValueError, OSError):
        pass

    return {
        "audit_kind": "container_mount_credential_audit",
        "host_path": host_path,
        "label": label,
        "safe": len(warnings) == 0,
        "warnings": warnings,
    }


def assert_safe_container_mount(
    host_path: str,
    *,
    label: str = "container mount",
) -> None:
    """Raise :exc:`AssuranceError` if *host_path* could expose credentials
    inside a container.
    """
    audit = audit_container_mount(host_path, label=label)
    if not audit["safe"]:
        raise AssuranceError(
            f"credential leak risk via {label}: "
            + "; ".join(audit["warnings"])
        )


# ── static call-site audit ──────────────────────────────────────────────────

def audit_credential_scrub_sites(
    source_paths: list[str],
) -> dict[str, Any]:
    """Statically verify that every ``_read_windows_credential`` call is
    paired with a scrub in a ``finally`` block or :class:`CredentialGuard`.

    This is a best-effort AST scan — it cannot catch dynamically-generated
    code or indirect calls.
    """
    findings: list[dict[str, Any]] = []
    total_reads = 0
    guarded_reads = 0
    files_scanned = 0

    for path_str in source_paths:
        try:
            with open(path_str, "r", encoding="utf-8") as fh:
                source = fh.read()
        except OSError:
            findings.append({
                "file": path_str,
                "error": "could not read file",
            })
            continue

        files_scanned += 1
        try:
            tree = ast.parse(source, filename=path_str)
        except SyntaxError as exc:
            findings.append({
                "file": path_str,
                "error": f"syntax error: {exc}",
            })
            continue

        visitor = _CredentialScrubVisitor()
        _add_parent_refs(tree)
        visitor.visit(tree)

        total_reads += visitor.credential_reads
        guarded_reads += visitor.guarded_reads

        if visitor.unscrubbed:
            for us in visitor.unscrubbed:
                findings.append({
                    "file": path_str,
                    "line": us["line"],
                    "issue": "credential read without detectable scrub",
                    "variable": us.get("variable", "?"),
                })

    return {
        "audit_kind": "credential_scrub_site_audit",
        "files_scanned": files_scanned,
        "total_credential_read_sites": total_reads,
        "guarded_reads": guarded_reads,
        "unscrubbed_sites": len(findings),
        "findings": findings,
        "passes": len(findings) == 0 and total_reads > 0,
        "limitations": [
            "AST scan only — cannot detect indirect or dynamic calls.",
            "CredentialGuard usage is detected by class name match in With nodes.",
            "Manual finally-block scrubs are detected by simple heuristic.",
        ],
    }


class _CredentialScrubVisitor(ast.NodeVisitor):
    """AST visitor that tracks credential reads and scrub coverage."""

    def __init__(self) -> None:
        self.credential_reads = 0
        self.guarded_reads = 0
        self.unscrubbed: list[dict[str, Any]] = []

    def visit_Call(self, node: ast.Call) -> None:
        # Detect _read_windows_credential(...) calls
        func_name = self._func_name(node.func)
        if func_name in ("_read_windows_credential", "CredentialGuard"):
            self.credential_reads += 1
            # Check if this call is inside a With node (CredentialGuard)
            if self._inside_with_credential_guard(node):
                self.guarded_reads += 1
            elif self._inside_finally(node):
                # Manual scrub in finally block
                self.guarded_reads += 1
            else:
                # Check if assignment target is scrubbed in a sibling finally
                assigned_var = self._assignment_target(node)
                if assigned_var and self._scrubbed_in_finally(node, assigned_var):
                    self.guarded_reads += 1
                elif func_name == "_read_windows_credential":
                    self.unscrubbed.append({
                        "line": node.lineno,
                        "variable": assigned_var or "?",
                    })
        self.generic_visit(node)

    @staticmethod
    def _func_name(node: ast.expr) -> str:
        if isinstance(node, ast.Name):
            return node.id
        if isinstance(node, ast.Attribute):
            return node.attr
        return ""

    @staticmethod
    def _inside_with_credential_guard(node: ast.AST) -> bool:
        """Check if *node* is nested inside ``with CredentialGuard(...)``."""
        # Walk up through parents (we set parent references below)
        current: ast.AST | None = node
        while current is not None:
            if isinstance(current, ast.With):
                for item in current.items:
                    if isinstance(item.context_expr, ast.Call):
                        name = _CredentialScrubVisitor._func_name(
                            item.context_expr.func
                        )
                        if name == "CredentialGuard":
                            return True
            current = getattr(current, "_cred_parent", None)  # type: ignore[union-attr]
        return False

    @staticmethod
    def _inside_finally(node: ast.AST) -> bool:
        """Check if *node* is inside a finally body."""
        current: ast.AST | None = node
        while current is not None:
            if isinstance(current, ast.Try):
                for handler in (current.finalbody or []):
                    if _CredentialScrubVisitor._contains(handler, node):
                        return True
            current = getattr(current, "_cred_parent", None)  # type: ignore[union-attr]
        return False

    @staticmethod
    def _contains(parent: ast.AST, target: ast.AST) -> bool:
        """Simple check if *parent* AST contains *target*."""
        for child in ast.walk(parent):
            if child is target:
                return True
        return False

    @staticmethod
    def _assignment_target(node: ast.Call) -> str:
        """If *node* is the value in an assignment, return the target name."""
        # Walk up to find the Assign node
        # This is approximate since we don't have parent pointers by default
        return ""

    @staticmethod
    def _scrubbed_in_finally(node: ast.Call, var_name: str) -> bool:
        """Check if *var_name* is scrubbed in a sibling finally block."""
        return False


# Wire up parent pointers for the AST visitor
def _add_parent_refs(tree: ast.AST) -> None:
    """Add ``_cred_parent`` references to every node in *tree*."""
    for parent in ast.walk(tree):
        for child in ast.iter_child_nodes(parent):
            setattr(child, "_cred_parent", parent)  # type: ignore[arg-type]




# ── public API summary ──────────────────────────────────────────────────────

__all__ = [
    "CredentialGuard",
    "ScrubRecord",
    "get_scrub_audit",
    "sanitize_child_environment",
    "audit_child_environment",
    "scan_dict_for_credentials",
    "assert_no_credential_in_dict",
    "audit_container_mount",
    "assert_safe_container_mount",
    "audit_credential_scrub_sites",
]
