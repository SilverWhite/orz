from __future__ import annotations

import os
from pathlib import Path, PurePosixPath, PureWindowsPath
from typing import Any
from urllib.parse import urlparse

from .errors import AssuranceError


def canonicalize_filesystem_path(raw: str | Path, *, base_root: Path | None = None) -> str:
    """Canonicalize a filesystem path for gate evaluation.

    Resolves symlinks, junctions, and ``..`` traversal.  Returns a normalised
    POSIX-style relative path when *base_root* is provided, otherwise an
    absolute path.  Raises :exc:`AssuranceError` on traversal escape or
    reparse-point ambiguity.
    """
    raw_text = os.fspath(raw)
    windows_path = PureWindowsPath(raw_text)
    if base_root is None and windows_path.drive and windows_path.is_absolute():
        return windows_path.as_posix()

    path = Path(raw)
    if not path.is_absolute() and base_root is not None:
        path = (base_root / path).resolve()
    else:
        try:
            path = path.resolve()
        except OSError as exc:
            raise AssuranceError(f"cannot resolve path: {raw}: {exc}") from exc

    if base_root is not None:
        try:
            base_root = Path(base_root).resolve()
        except OSError as exc:
            raise AssuranceError(
                f"cannot resolve base root: {base_root}: {exc}"
            ) from exc
        try:
            path.relative_to(base_root)
        except ValueError:
            raise AssuranceError(
                f"path traversal detected: {raw} escapes base root {base_root}"
            )

    return path.as_posix()


def canonicalize_network_endpoint(raw: str) -> str:
    """Canonicalize a network endpoint URL for gate evaluation.

    Normalises the scheme, hostname, port, and path components.  Strips
    fragment and userinfo.  Rejects non-HTTP(S) schemes.

    Returns a normalised ``scheme://host[:port]/path`` string.
    """
    try:
        parsed = urlparse(raw)
    except Exception as exc:
        raise AssuranceError(f"cannot parse URL: {raw}: {exc}") from exc

    if parsed.scheme not in {"http", "https"}:
        raise AssuranceError(
            f"network endpoint must use http or https scheme: {parsed.scheme}"
        )

    if parsed.hostname is None:
        raise AssuranceError(f"network endpoint must include a hostname: {raw}")

    hostname = parsed.hostname.lower()
    port = parsed.port
    path = parsed.path or "/"

    # Strip trailing slash unless root
    if path != "/" and path.endswith("/"):
        path = path[:-1]

    # Normalise port display
    default_port = 443 if parsed.scheme == "https" else 80
    if port is None or port == default_port:
        return f"{parsed.scheme}://{hostname}{path}"

    return f"{parsed.scheme}://{hostname}:{port}{path}"


def validate_endpoint_list(
    endpoints: list[str],
    *,
    allowed_schemes: set[str] | None = None,
    allowed_hosts: set[str] | None = None,
) -> list[dict[str, Any]]:
    """Validate a list of network endpoints against an allowlist.

    Returns a list of per-endpoint validation results.
    Each result has ``endpoint``, ``canonical``, ``allowed``, and ``reason``.
    """
    results: list[dict[str, Any]] = []
    for raw in endpoints:
        try:
            canonical = canonicalize_network_endpoint(raw)
        except AssuranceError as exc:
            results.append({
                "endpoint": raw,
                "canonical": None,
                "allowed": False,
                "reason": str(exc),
            })
            continue

        allowed = True
        reason = "endpoint allowed"

        if allowed_schemes is not None:
            scheme = urlparse(canonical).scheme
            if scheme not in allowed_schemes:
                allowed = False
                reason = f"scheme {scheme} not in allowlist"

        if allowed and allowed_hosts is not None:
            hostname = urlparse(canonical).hostname or ""
            if hostname not in allowed_hosts:
                allowed = False
                reason = f"host {hostname} not in allowlist"

        results.append({
            "endpoint": raw,
            "canonical": canonical,
            "allowed": allowed,
            "reason": reason,
        })

    return results


def validate_filesystem_targets(
    paths: list[str],
    *,
    base_root: Path | None = None,
    forbidden_prefixes: list[str] | None = None,
) -> list[dict[str, Any]]:
    """Validate a list of filesystem targets against a blocklist.

    Returns a list of per-path validation results.
    """
    results: list[dict[str, Any]] = []
    for raw in paths:
        try:
            canonical = canonicalize_filesystem_path(raw, base_root=base_root)
        except AssuranceError as exc:
            results.append({
                "path": raw,
                "canonical": None,
                "allowed": False,
                "reason": str(exc),
            })
            continue

        allowed = True
        reason = "path allowed"

        if forbidden_prefixes is not None:
            lower = canonical.lower()
            for prefix in forbidden_prefixes:
                if lower.startswith(prefix.lower()):
                    allowed = False
                    reason = f"path matches forbidden prefix: {prefix}"
                    break

        results.append({
            "path": raw,
            "canonical": canonical,
            "allowed": allowed,
            "reason": reason,
        })

    return results
