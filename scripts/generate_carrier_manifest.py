#!/usr/bin/env python3
"""Generate the carrier release manifest (0bw②, 2026-09-27).

Writes ``carrier-manifest.json`` into the carrier directory: one entry per
top-level regular file (flat layout — the carrier bundle ships binaries and
support files side by side), keyed by file name, carrying sha256 + size.

The runtime self-check (`orz_host::carrier_integrity::verify_at_startup`)
verifies every entry at startup when the manifest is present; a missing
manifest (dev trees) is silently skipped and a mismatch is a loud stderr
warning — the audit leg of the 0bw write-control design (§6), never a
start-up blocker.

Usage:
    python scripts/generate_carrier_manifest.py <carrier-dir> [--version V]

Excludes: the manifest itself, symlinks/dirs (v1 flat-files only), and the
machine-local ``.bak`` backup chain (``orz.exe.0.7.3-bak`` /
``signer-manifest.json.bak-20260927``) — swap-site archaeology, not carrier
payload (2026-09-27 v0.8.0 rebuild friction: the naive scan enrolled 80 bak
files and a moved copy reported 79 bogus "missing" findings).
Re-run after ANY change to the carrier directory contents (e.g. swapping a
binary) — a stale manifest is a guaranteed false alarm at startup.
"""

from __future__ import annotations

import argparse
import datetime
import hashlib
import json
import os
import sys
from pathlib import Path

MANIFEST_FILENAME = "carrier-manifest.json"
MANIFEST_KIND = "orz-carrier-manifest"


def sha256_of(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def generate(carrier_dir: Path, version: str) -> Path:
    if not carrier_dir.is_dir():
        raise SystemExit(f"carrier dir not found: {carrier_dir}")
    entries: dict[str, dict[str, object]] = {}
    for item in sorted(carrier_dir.iterdir()):
        if item.name == MANIFEST_FILENAME:
            continue
        if ".bak" in item.name or item.name.endswith("-bak"):
            # machine-local backup chain — two on-disk forms: versioned
            # `orz.exe.0.7.3-bak` (dash form) and `signer-manifest.json.
            # bak-<date>` (dot form). Swap-site archaeology, not carrier
            # payload (v0.8.0 rebuild friction).
            continue
        if not item.is_file() or item.is_symlink():
            # v1: flat regular files only (dirs such as grok-home/ are
            # covered by the write-protection face, not this manifest).
            continue
        entries[item.name] = {
            "sha256": sha256_of(item),
            "size_bytes": item.stat().st_size,
        }
    manifest = {
        "kind": MANIFEST_KIND,
        "version": version,
        "generated_at": datetime.datetime.now(datetime.timezone.utc)
        .replace(microsecond=0)
        .isoformat()
        .replace("+00:00", "Z"),
        "entries": entries,
    }
    out_path = carrier_dir / MANIFEST_FILENAME
    # 显式 LF（2026-09-27 复审 P3）：仓内 manifest 面沿显式 LF 先例
    # （orz_source_manifest 同口径），Windows 上不落 CRLF。
    with out_path.open("w", encoding="utf-8", newline="\n") as handle:
        handle.write(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n")
    return out_path


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("carrier_dir", type=Path, help="carrier directory to scan")
    parser.add_argument(
        "--version",
        default="unknown",
        help="carrier version recorded informationally (e.g. 0.7.3)",
    )
    args = parser.parse_args()
    out_path = generate(args.carrier_dir, args.version)
    count = len(json.loads(out_path.read_text(encoding="utf-8"))["entries"])
    print(f"carrier manifest written: {out_path} ({count} entries)")


if __name__ == "__main__":
    sys.exit(main())
