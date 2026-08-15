"""Generate / verify the `orz/` source integrity manifest.

P0-C S3 前置全面审查修复 (F6, 2026-08-15): `orz/` is deliberately outside
git tracking (`.gitignore`), so its sources have no version-control recovery
path. This script maintains a committed SHA-256 manifest
(`orz_source_manifest.sha256`) so corruption or unrecorded
modification of the untracked runtime is detectable by the repository gate
(`python scripts/check_repository.py`) and by the script itself.

Usage:
    python scripts/generate_orz_source_manifest.py            # regenerate
    python scripts/generate_orz_source_manifest.py --check    # verify

The manifest is a recovery ledger, not a content backup: it detects changes
but does not restore deleted content. Full version control of `orz/` remains
a deliberate repository boundary pending user decision.
"""

from __future__ import annotations

import argparse
import hashlib
from pathlib import Path, PurePosixPath
import sys


ROOT = Path(__file__).resolve().parents[1]
ORZ_ROOT = ROOT / "orz"
MANIFEST = ROOT / "orz_source_manifest.sha256"
EXCLUDED_DIRS = {"target", ".git", ".pytest_cache", "__pycache__"}
HEADER = "# orz source integrity manifest (untracked runtime) — regenerate with:"
HEADER_CMD = "#   python scripts/generate_orz_source_manifest.py"


def _relative_posix(path: Path) -> str:
    return path.relative_to(ORZ_ROOT).as_posix()


def _is_excluded(relative_parts: tuple[str, ...]) -> bool:
    return any(part in EXCLUDED_DIRS for part in relative_parts)


def _source_files() -> list[Path]:
    if not ORZ_ROOT.is_dir():
        return []
    files: list[Path] = []
    for path in ORZ_ROOT.rglob("*"):
        if not path.is_file():
            continue
        relative_parts = path.relative_to(ORZ_ROOT).parts
        if _is_excluded(relative_parts):
            continue
        files.append(path)
    return sorted(files, key=lambda p: _relative_posix(p))


def _sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def generate() -> None:
    files = _source_files()
    lines = [HEADER, HEADER_CMD, ""]
    for path in files:
        lines.append(f"{_sha256(path)}  {_relative_posix(path)}")
    MANIFEST.write_text("\n".join(lines) + "\n", encoding="utf-8")
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
    actual: dict[str, Path] = {}
    for path in _source_files():
        actual[_relative_posix(path)] = path
    for rel, digest in sorted(expected.items()):
        path = actual.get(rel)
        if path is None:
            errors.append(f"orz source missing: {rel}")
            continue
        if _sha256(path) != digest:
            errors.append(f"orz source digest mismatch: {rel}")
    for rel in sorted(set(actual) - set(expected)):
        errors.append(
            f"orz source not in manifest (regenerate {MANIFEST.relative_to(ROOT)}): {rel}"
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
