"""Generate / verify the `orz/` submodule source integrity manifest.

P0-C S3 前置全面审查修复 (F6, 2026-08-15): `orz/` was originally an
untracked runtime directory, so a committed SHA-256 manifest
(`orz_source_manifest.sha256`) was introduced to detect corruption or
unrecorded modification. `orz/` is now registered as a git submodule, so
the manifest covers the submodule's tracked files using canonical
repository bytes (independent of local line-ending settings and untracked
local artifacts), and the repository gate
(`python scripts/check_repository.py`) verifies the pinned submodule
content.

Usage:
    python scripts/generate_orz_source_manifest.py            # regenerate
    python scripts/generate_orz_source_manifest.py --check    # verify

The manifest is a recovery ledger, not a content backup: it detects changes
but does not restore deleted content.
"""

from __future__ import annotations

import argparse
import hashlib
import io
from pathlib import Path, PurePosixPath
import subprocess
import sys


ROOT = Path(__file__).resolve().parents[1]
ORZ_ROOT = ROOT / "orz"
MANIFEST = ROOT / "orz_source_manifest.sha256"
EXCLUDED_DIRS = {"target", ".git", ".pytest_cache", "__pycache__"}
HEADER = (
    "# orz source integrity manifest (tracked submodule content) "
    "— regenerate with:"
)
HEADER_CMD = "#   python scripts/generate_orz_source_manifest.py"


def _is_excluded(relative_parts: tuple[str, ...]) -> bool:
    return any(part in EXCLUDED_DIRS for part in relative_parts)


def _tracked_files() -> list[str]:
    """Posix-relative paths of files tracked in the `orz` submodule."""
    proc = subprocess.run(
        ["git", "-C", str(ORZ_ROOT), "ls-files", "-z"],
        capture_output=True,
        check=False,
    )
    if proc.returncode != 0:
        raise RuntimeError(
            "cannot list orz submodule files (is `orz/` checked out?): "
            f"{proc.stderr.decode('utf-8', 'replace').strip()}"
        )
    return [
        path
        for path in proc.stdout.decode("utf-8", "surrogateescape").split("\0")
        if path and not _is_excluded(PurePosixPath(path).parts)
    ]


def _canonical_sha256s(paths: list[str]) -> dict[str, str]:
    """SHA-256 of each path's blob at the submodule HEAD commit.

    Hashing canonical git blob bytes keeps the manifest independent of
    local line-ending settings (core.autocrlf) and untracked artifacts.
    """
    if not paths:
        return {}
    requests = "".join(f"HEAD:{path}\n" for path in paths).encode(
        "utf-8", "surrogateescape"
    )
    proc = subprocess.run(
        ["git", "-C", str(ORZ_ROOT), "cat-file", "--batch"],
        input=requests,
        capture_output=True,
        check=False,
    )
    if proc.returncode != 0:
        raise RuntimeError(
            "git cat-file --batch failed for the orz submodule: "
            f"{proc.stderr.decode('utf-8', 'replace').strip()}"
        )
    stream = io.BytesIO(proc.stdout)
    digests: dict[str, str] = {}
    for path in paths:
        header = stream.readline()
        parts = header.decode("ascii", "replace").split()
        if len(parts) != 3 or parts[1] != "blob":
            raise RuntimeError(
                f"cannot read canonical content for orz/{path}"
            )
        size = int(parts[2])
        content = stream.read(size)
        trailing = stream.read(1)
        if trailing != b"\n":
            raise RuntimeError(
                f"malformed cat-file output for orz/{path}"
            )
        digests[path] = hashlib.sha256(content).hexdigest()
    return digests


def generate() -> None:
    files = _tracked_files()
    digests = _canonical_sha256s(files)
    lines = [HEADER, HEADER_CMD, ""]
    for rel in sorted(files):
        lines.append(f"{digests[rel]}  {rel}")
    # R-2 (GLM 2026-09-04)：Windows 文本模式默认 CRLF 会令 POSIX sha256sum -c
    # 无法整文件校验；显式以 LF 写出。
    with MANIFEST.open("w", encoding="utf-8", newline="\n") as fh:
        fh.write("\n".join(lines) + "\n")
    print(f"wrote {len(files)} entries to {MANIFEST.relative_to(ROOT)}")


def verify() -> tuple[bool, list[str]]:
    errors: list[str] = []
    if not MANIFEST.is_file():
        return False, [f"manifest missing: {MANIFEST.relative_to(ROOT)}"]
    expected: dict[str, str] = {}
    for line_number, raw in enumerate(
        MANIFEST.read_text(encoding="utf-8").splitlines(), start=1
    ):
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        parts = line.split(maxsplit=1)
        if len(parts) != 2 or len(parts[0]) != 64:
            errors.append(
                f"{MANIFEST.relative_to(ROOT)}:{line_number}: malformed manifest line"
            )
            continue
        digest, rel = parts
        try:
            int(digest, 16)
        except ValueError:
            errors.append(
                f"{MANIFEST.relative_to(ROOT)}:{line_number}: digest is not hex"
            )
            continue
        try:
            PurePosixPath(rel)
        except ValueError:
            errors.append(
                f"{MANIFEST.relative_to(ROOT)}:{line_number}: bad relative path {rel!r}"
            )
            continue
        expected[rel] = digest
    try:
        files = _tracked_files()
        digests = _canonical_sha256s(files)
    except RuntimeError as exc:
        return False, [str(exc)]
    for rel, digest in sorted(expected.items()):
        actual_digest = digests.get(rel)
        if actual_digest is None:
            errors.append(f"orz source missing: {rel}")
            continue
        if actual_digest != digest:
            errors.append(f"orz source digest mismatch: {rel}")
    for rel in sorted(set(digests) - set(expected)):
        errors.append(
            "orz source not in manifest "
            f"(regenerate {MANIFEST.relative_to(ROOT)}): {rel}"
        )
    return not errors, errors


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--check",
        action="store_true",
        help="verify the manifest instead of regenerating it",
    )
    args = parser.parse_args()
    if args.check:
        ok, errors = verify()
        for error in errors:
            print(error)
        print("valid" if ok else f"{len(errors)} error(s)")
        return 0 if ok else 1
    generate()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
