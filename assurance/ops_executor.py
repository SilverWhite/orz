#!/usr/bin/env python3
"""Structured Operation Protocol v0.1 -- cross-platform reference executor.

Reads one operation JSON from --op-file or stdin, validates it strictly, applies
the mechanical safety policy (scope, cache classification, recycle-bin capacity,
trash-by-default), executes or rejects, and appends one audit JSONL record.

Exit codes:
  0  executed or dry-run plan produced
  1  invalid input or execution error
  2  policy rejection (no mutation performed)
"""

from __future__ import annotations

import argparse
import datetime as _dt
import json
import os
import platform
import re
import shutil
import subprocess
import sys
import tempfile
import time
import uuid

PROTO = 1
DEFAULT_MAX_READ_BYTES = 10 * 1024 * 1024
TOTAL_READ_LIMIT_BYTES = 64 * 1024 * 1024
DEFAULT_TRASH_MAX_BYTES = 5 * 1024 * 1024 * 1024
DEFAULT_PROCESS_TIMEOUT_MS = 120_000
OUTPUT_TAIL_BYTES = 64 * 1024
MAX_TARGETS = 1000
MAX_TARGET_LEN = 4096
MAX_CONTENT_LEN = 10_485_760
MAX_EXE_LEN = 2048
MAX_ARG_LEN = 4096
MAX_ARGS = 512
MAX_DEST_LEN = 4096
MAX_TOTAL_DIR_ENTRIES = 1_000_000

COMMON_FIELDS = {"proto", "id", "op", "targets", "reason"}
OP_SPECS = {
    "file.read": {"targets": True, "fields": {"max_bytes"}},
    "file.write": {"targets": True, "fields": {"content"}},
    "file.move": {"targets": True, "fields": {"dest"}},
    "file.copy": {"targets": True, "fields": {"dest"}},
    "file.delete": {"targets": True, "fields": {"dry_run"}},
    "dir.list": {"targets": True, "fields": {"depth", "max_entries"}},
    "process.run": {"targets": False, "fields": {"exe", "args", "cwd", "timeout_ms"}},
    "env.info": {"targets": False, "fields": set()},
}


class InvalidOp(Exception):
    pass


class PolicyError(Exception):
    def __init__(self, message, detail=None):
        super().__init__(message)
        self.detail = detail or {}


class ExecutionError(Exception):
    pass


def validate_op(raw):
    if not isinstance(raw, dict):
        raise InvalidOp("envelope must be a JSON object")
    for field in ("proto", "op", "targets"):
        if field not in raw:
            raise InvalidOp("missing required field: %s" % field)
    if raw["proto"] != PROTO:
        raise InvalidOp("unsupported proto: %r" % raw["proto"])
    op = raw["op"]
    if op not in OP_SPECS:
        raise InvalidOp("unknown op: %r" % op)
    spec = OP_SPECS[op]
    unknown = set(raw) - COMMON_FIELDS - spec["fields"]
    if unknown:
        raise InvalidOp("unknown fields: %s" % ", ".join(sorted(unknown)))
    targets = raw["targets"]
    if not isinstance(targets, list):
        raise InvalidOp("targets must be an array")
    if len(targets) > MAX_TARGETS:
        raise InvalidOp("targets exceeds %d items" % MAX_TARGETS)
    if spec["targets"] and not targets:
        raise InvalidOp("targets must be a non-empty array")
    if any(not isinstance(t, str) or not t.strip() for t in targets):
        raise InvalidOp("targets entries must be non-empty strings")
    if any(len(t) > MAX_TARGET_LEN for t in targets):
        raise InvalidOp("targets entries exceed %d chars" % MAX_TARGET_LEN)
    if "id" in raw and not (isinstance(raw["id"], str) and re.fullmatch(r"[A-Za-z0-9._-]{1,80}", raw["id"])):
        raise InvalidOp("invalid id")
    if "reason" in raw and not (isinstance(raw["reason"], str) and 1 <= len(raw["reason"]) <= 500):
        raise InvalidOp("reason must be a string of 1..500 chars")

    if op == "file.read" and "max_bytes" in raw and not (isinstance(raw["max_bytes"], int) and 1 <= raw["max_bytes"] <= 1 << 30):
        raise InvalidOp("max_bytes must be an integer in [1, 2**30]")
    if op == "file.write":
        if len(targets) != 1:
            raise InvalidOp("file.write requires exactly one target")
        if not isinstance(raw.get("content"), str):
            raise InvalidOp("file.write requires string content")
        if len(raw["content"]) > MAX_CONTENT_LEN:
            raise InvalidOp("content exceeds %d chars" % MAX_CONTENT_LEN)
    if op in ("file.move", "file.copy"):
        if len(targets) != 1:
            raise InvalidOp("%s requires exactly one target" % op)
        if not isinstance(raw.get("dest"), str) or not raw["dest"].strip():
            raise InvalidOp("%s requires dest" % op)
        if len(raw["dest"]) > MAX_DEST_LEN:
            raise InvalidOp("dest exceeds %d chars" % MAX_DEST_LEN)
    if op == "file.delete" and "dry_run" in raw and not isinstance(raw["dry_run"], bool):
        raise InvalidOp("dry_run must be boolean")
    if op == "dir.list":
        if "depth" in raw and not (isinstance(raw["depth"], int) and 1 <= raw["depth"] <= 32):
            raise InvalidOp("depth must be an integer in [1, 32]")
        if "max_entries" in raw and not (isinstance(raw["max_entries"], int) and 1 <= raw["max_entries"] <= 1_000_000):
            raise InvalidOp("max_entries must be an integer in [1, 1000000]")
    if op == "process.run":
        if not isinstance(raw.get("exe"), str) or not raw["exe"].strip():
            raise InvalidOp("process.run requires exe")
        if len(raw["exe"]) > MAX_EXE_LEN:
            raise InvalidOp("exe exceeds %d chars" % MAX_EXE_LEN)
        if "args" in raw:
            if not isinstance(raw["args"], list) or len(raw["args"]) > MAX_ARGS:
                raise InvalidOp("args must be an array of at most %d strings" % MAX_ARGS)
            if not all(isinstance(a, str) and len(a) <= MAX_ARG_LEN for a in raw["args"]):
                raise InvalidOp("args must be strings of at most %d chars" % MAX_ARG_LEN)
        if "cwd" in raw and not (isinstance(raw["cwd"], str) and raw["cwd"].strip()):
            raise InvalidOp("cwd must be a non-empty string")
        if "timeout_ms" in raw and not (isinstance(raw["timeout_ms"], int) and 1000 <= raw["timeout_ms"] <= 3_600_000):
            raise InvalidOp("timeout_ms must be an integer in [1000, 3600000]")
    return raw


def get_allow_roots(cli_roots):
    roots = list(cli_roots or [])
    env = os.environ.get("OPS_ALLOW_ROOTS", "")
    if env:
        roots.extend(p for p in env.split(os.pathsep) if p.strip())
    return [os.path.realpath(os.path.abspath(p)) for p in roots]


def _inside(path, roots):
    norm = os.path.normcase(os.path.realpath(path))
    for r in roots:
        rn = os.path.normcase(os.path.realpath(r))
        if norm == rn or norm.startswith(rn.rstrip("\\/") + os.sep):
            return True
    return False


def resolve_scoped(path, roots, require_inside=True):
    full = os.path.realpath(os.path.abspath(os.path.expanduser(path)))
    if require_inside and not _inside(full, roots):
        raise PolicyError("target outside allow roots: %s" % full)
    return full


def ensure_not_system_root(path):
    norm = os.path.normcase(os.path.realpath(path))
    fs_root = os.path.normcase(os.path.abspath(os.sep))
    if norm == fs_root or re.match(r"^[a-z]:[\\/]?$", norm):
        raise PolicyError("refusing to target a filesystem root: %s" % path)


def ensure_not_allow_root(path, roots):
    norm = os.path.normcase(os.path.realpath(path))
    for r in roots:
        if norm == os.path.normcase(os.path.realpath(r)):
            raise PolicyError("refusing to target an allow root itself: %s" % path)


def protected_paths():
    paths = set()
    try:
        exe = os.path.realpath(os.path.abspath(sys.argv[0]))
        if exe:
            paths.add(os.path.normcase(exe))
    except Exception:
        pass
    try:
        paths.add(os.path.normcase(os.path.realpath(os.path.abspath(audit_path()))))
    except Exception:
        pass
    for p in os.environ.get("OPS_PROTECTED", "").split(os.pathsep):
        if p.strip():
            try:
                paths.add(os.path.normcase(os.path.realpath(os.path.abspath(p))))
            except Exception:
                pass
    return paths


def ensure_not_protected(path):
    if os.path.basename(os.path.normpath(path)).lower() == ".ops.json":
        raise PolicyError("refusing to mutate .ops.json (model cannot declare disposable/protected): %s" % path)
    norm = os.path.normcase(os.path.realpath(path))
    if norm in protected_paths():
        raise PolicyError("refusing to mutate a protected executor/audit/config file: %s" % path)


def decode_text(data):
    """Fixed decode chain: strip BOM -> UTF-8 strict -> GB18030 -> refined
    UTF-8 lossy (0as, 2026-09-19 — mirrors orz-tools `util/encoding.rs`).
    Returns (text, encoding_label). The refined lossy stage decodes per line
    with a minimal-replacement choice; its label carries the degraded
    fraction as `utf-8-lossy:<p>%` (units = placeholders + U+FFFD, over the
    post-BOM-strip input bytes, two decimals).

    Pinned table boundaries vs the Rust side (2026-09-19 full-space
    differential, see assurance/tests/test_ops_executor_decode_text.py and
    docs/audits/0AS_REVIEW_HANDLING_2026-09-19.md): CPython's gb18030 codec
    keeps GB18030-2000 PUA mappings where encoding_rs (WHATWG /
    GB18030-2005+) uses official characters (20 two-byte pairs + one
    four-byte slot decode differently), replaces over-range four-byte forms
    with two U+FFFD where encoding_rs uses one (can flip the lossy
    minimal-replacement choice), and treats the bare euro byte 0x80 as an
    error where encoding_rs decodes it cleanly — the one known label fork.
    Both sides pin these boundaries."""
    label = "utf-8"
    if data.startswith(b"\xef\xbb\xbf"):
        data = data[3:]
        label = "utf-8-sig"
    try:
        return data.decode("utf-8"), label
    except UnicodeDecodeError:
        pass
    try:
        return data.decode("gb18030"), "gb18030"
    except UnicodeDecodeError:
        pass
    text, units = _decode_lossy_segmented(data)
    ratio = units / max(len(data), 1) * 100
    return text, "utf-8-lossy:%.2f%%" % ratio


def _decode_lossy_segmented(data):
    """0as lossy stage: per-line ladder so one bad byte cannot degrade the
    whole block. Neither UTF-8 nor GB18030 encodes 0x0A inside a multi-byte
    sequence, so splitting on \\n cannot truncate a character."""
    parts = []
    units = 0
    lines = data.split(b"\n")
    for index, body in enumerate(lines):
        text, line_units = _decode_lossy_line(body)
        parts.append(text)
        if index + 1 < len(lines):
            parts.append("\n")
        units += line_units
    return "".join(parts), units


def _decode_lossy_line(line):
    """Same ladder per line: valid UTF-8 kept -> clean GB18030 kept ->
    minimal-replacement lossy (fewer units wins; tie keeps the ladder order,
    UTF-8). Units = placeholders + U+FFFD."""
    try:
        return line.decode("utf-8"), 0
    except UnicodeDecodeError:
        pass
    try:
        return line.decode("gb18030"), 0
    except UnicodeDecodeError:
        pass
    utf8_text, utf8_units = _utf8_lossy_placeholders(line)
    gb_text = line.decode("gb18030", errors="replace")
    gb_units = gb_text.count("\ufffd")
    if gb_units < utf8_units:
        return gb_text, gb_units
    return utf8_text, utf8_units


def _utf8_lossy_placeholders(data):
    """UTF-8 lossy walk with explicit byte placeholders (0as): each invalid
    subsequence renders as `⟨0x8F⟩` / `⟨0xF0 0x9E 0x81⟩` instead of U+FFFD.
    CPython's UnicodeDecodeError spans follow the same maximal-subpart
    granularity as Rust's Utf8Error, so unit counts match `encoding.rs`."""
    parts = []
    units = 0
    offset = 0
    while offset < len(data):
        try:
            parts.append(data[offset:].decode("utf-8"))
            break
        except UnicodeDecodeError as error:
            start = offset + error.start
            end = offset + max(error.end, error.start + 1)
            parts.append(data[offset:start].decode("utf-8"))
            placeholder = " ".join("0x%02X" % b for b in data[start:end])
            parts.append("⟨%s⟩" % placeholder)
            units += 1
            offset = end
    return "".join(parts), units


def get_process_allowlist():
    out = set()
    for p in os.environ.get("OPS_PROCESS_ALLOW", "").split(os.pathsep):
        p = p.strip()
        if p:
            out.add(p)
    return out


def _pattern_regex(pattern):
    p = pattern.replace("\\", "/")
    if not p.startswith("/"):
        p = "**/" + p
    out = []
    i = 0
    while i < len(p):
        c = p[i]
        if c == "*":
            if i + 1 < len(p) and p[i + 1] == "*":
                while i + 1 < len(p) and p[i + 1] == "*":
                    i += 1
                out.append(".*")
            else:
                out.append("[^/]*")
        elif c == "?":
            out.append("[^/]")
        else:
            out.append(re.escape(c))
        i += 1
    return re.compile("^" + "".join(out) + "$")


def _match_patterns(patterns, path):
    if isinstance(patterns, str):
        patterns = [patterns]
    p = os.path.normpath(path).replace("\\", "/")
    for pat in patterns or []:
        if isinstance(pat, str) and _pattern_regex(pat).fullmatch(p):
            return pat
    return None


def _find_ops_config(target, roots):
    """Returns (config_dict_or_None, malformed_bool). A present-but-invalid
    .ops.json is treated as malformed (fail-closed)."""
    cur = os.path.dirname(os.path.realpath(target))
    for _ in range(64):
        cfg = os.path.join(cur, ".ops.json")
        if os.path.isfile(cfg):
            try:
                with open(cfg, "r", encoding="utf-8") as fh:
                    data = json.load(fh)
                if isinstance(data, dict):
                    return data, False
                return None, True
            except Exception:
                return None, True
        if any(os.path.normcase(cur) == os.path.normcase(r) for r in roots):
            break
        parent = os.path.dirname(cur)
        if parent == cur:
            break
        cur = parent
    return {}, False


def _all_files_match(root, regex):
    for dirpath, dirnames, filenames in os.walk(root):
        dirnames[:] = [d for d in dirnames if not os.path.islink(os.path.join(dirpath, d))]
        for fn in filenames:
            if not regex.search(fn):
                return False
    return True


def classify(path, roots):
    real = os.path.realpath(path)
    cfg, malformed = _find_ops_config(real, roots)
    if malformed:
        return "protected", "malformed .ops.json (fail-closed)"

    chain = [real]
    parent = os.path.dirname(real)
    while parent and parent != real:
        if any(os.path.normcase(parent) == os.path.normcase(r) for r in roots):
            break
        chain.append(parent)
        real_parent = parent
        parent = os.path.dirname(parent)
        if parent == real_parent:
            break

    if cfg:
        for p in chain:
            pat = _match_patterns(cfg.get("protected"), p)
            if pat:
                return "protected", "project config protected pattern: %s" % pat
        for p in chain:
            pat = _match_patterns(cfg.get("disposable"), p)
            if pat:
                return "cache", "project config disposable pattern: %s" % pat

    for p in chain:
        name = os.path.basename(p.rstrip("\\/")) or p
        parts = [x.lower() for x in p.replace("\\", "/").split("/")]
        rel = os.path.relpath(p, os.path.commonpath([p, roots[0]])) if roots else p
        if name == "__pycache__":
            if _all_files_match(p, re.compile(r"\.py[co]$", re.IGNORECASE)):
                return "cache", "builtin: %s __pycache__ contains only .pyc/.pyo" % rel
        if name == "target" and os.path.isdir(p):
            markers = [m for m in ("debug", "release", ".fingerprint", "CACHEDIR.TAG")
                       if os.path.exists(os.path.join(p, m))]
            if markers:
                return "cache", "builtin: %s target/%s" % (rel, ",".join(markers))
        if name == "node_modules" and os.path.isdir(p):
            if os.path.exists(os.path.join(p, ".package-lock.json")) or os.path.exists(os.path.join(p, ".bin")):
                return "cache", "builtin: %s node_modules/.package-lock.json or .bin" % rel
            parent = os.path.dirname(p)
            if os.path.isfile(os.path.join(parent, "package.json")) or os.path.isfile(os.path.join(parent, "package-lock.json")):
                return "cache", "builtin: %s node_modules with package manifests" % rel
        if ".cargo" in parts and "registry" in parts:
            return "cache", "builtin: %s .cargo/registry" % rel
        if ".gradle" in parts and "caches" in parts:
            return "cache", "builtin: %s .gradle/caches" % rel
        if name == ".cache" and os.path.exists(os.path.join(p, "CACHEDIR.TAG")):
            return "cache", "builtin: %s .cache/CACHEDIR.TAG" % rel
        if os.path.isdir(p) and os.path.exists(os.path.join(p, "CACHEDIR.TAG")):
            return "cache", "builtin: %s CACHEDIR.TAG" % rel
    return "unknown", "no classification rule matched"


def measure(path):
    if os.path.islink(path) or os.path.isfile(path):
        try:
            return os.path.getsize(path), 1, 0
        except OSError:
            return 0, 0, 0
    total = 0
    files = 0
    dirs = 0
    for dirpath, dirnames, filenames in os.walk(path):
        dirnames[:] = [d for d in dirnames if not os.path.islink(os.path.join(dirpath, d))]
        dirs += len(dirnames)
        for fn in filenames:
            fp = os.path.join(dirpath, fn)
            if os.path.islink(fp):
                continue
            try:
                total += os.path.getsize(fp)
                files += 1
            except OSError:
                pass
    return total, files, dirs


def _windows_recycle_capacity(path):
    try:
        import ctypes
        from ctypes import wintypes
        drive = os.path.splitdrive(os.path.abspath(path))[0] or os.path.splitdrive(os.getcwd())[0]
        mount = drive if drive.endswith("\\") else drive + "\\"
        buf = ctypes.create_unicode_buffer(256)
        if ctypes.windll.kernel32.GetVolumeNameForVolumeMountPointW(mount, buf, 256):
            m = re.search(r"\{([0-9A-Fa-f-]+)\}", buf.value)
            if m:
                import winreg
                key_path = r"Software\Microsoft\Windows\CurrentVersion\Explorer\BitBucket\Volume\{%s}" % m.group(1)
                with winreg.OpenKey(winreg.HKEY_CURRENT_USER, key_path) as key:
                    mb = winreg.QueryValueEx(key, "MaxCapacity")[0]
                    if mb:
                        return int(mb) * 1024 * 1024
    except Exception:
        pass
    try:
        drive = os.path.splitdrive(os.path.abspath(path))[0] or os.path.splitdrive(os.getcwd())[0]
        total = shutil.disk_usage(drive + os.sep).total
        return int(total * 0.10)
    except Exception:
        return DEFAULT_TRASH_MAX_BYTES


def recycle_capacity(path):
    if os.name == "nt":
        return _windows_recycle_capacity(path)
    env = os.environ.get("OPS_TRASH_MAX_BYTES")
    if env:
        try:
            return max(1, int(env))
        except ValueError:
            pass
    return DEFAULT_TRASH_MAX_BYTES


def _posix_trash(path):
    data_home = os.environ.get("XDG_DATA_HOME") or os.path.join(os.path.expanduser("~"), ".local", "share")
    files_dir = os.path.join(data_home, "Trash", "files")
    info_dir = os.path.join(data_home, "Trash", "info")
    os.makedirs(files_dir, exist_ok=True)
    os.makedirs(info_dir, exist_ok=True)
    base = os.path.basename(path.rstrip("\\/"))
    name = base
    n = 1
    while os.path.exists(os.path.join(files_dir, name)):
        n += 1
        name = "%s.%d" % (base, n)
    dest = os.path.join(files_dir, name)
    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y-%m-%dT%H:%M:%S")
    escaped = path.replace("\\", "\\\\").replace("\n", "\\n").replace("\r", "\\r")
    with open(os.path.join(info_dir, name + ".trashinfo"), "w", encoding="utf-8") as fh:
        fh.write("[Trash Info]\nPath=%s\nDeletionDate=%s\n" % (escaped, ts))
    try:
        os.rename(path, dest)
    except OSError:
        shutil.move(path, dest)


def _windows_trash(path):
    import ctypes
    from ctypes import wintypes

    class SHFILEOPSTRUCTW(ctypes.Structure):
        _fields_ = [
            ("hwnd", wintypes.HWND),
            ("wFunc", wintypes.UINT),
            ("pFrom", wintypes.LPCWSTR),
            ("pTo", wintypes.LPCWSTR),
            ("fFlags", wintypes.UINT),
            ("fAnyOperationsAborted", wintypes.BOOL),
            ("hNameMappings", ctypes.c_void_p),
            ("lpszProgressTitle", wintypes.LPCWSTR),
        ]

    FO_DELETE = 3
    FOF_ALLOWUNDO = 0x40
    FOF_NOCONFIRMATION = 0x10
    FOF_SILENT = 0x0004
    op = SHFILEOPSTRUCTW(0, FO_DELETE, path + "\0\0", None,
                         FOF_ALLOWUNDO | FOF_NOCONFIRMATION | FOF_SILENT,
                         0, None, None)
    res = ctypes.windll.shell32.SHFileOperationW(ctypes.byref(op))
    if res != 0:
        raise OSError("SHFileOperationW failed with code %d" % res)


def delete_paths(paths, mode, roots):
    deleted = []
    for p in paths:
        ensure_not_system_root(p)
        ensure_not_allow_root(p, roots)
        if mode == "trash":
            if os.name == "nt":
                _windows_trash(p)
            else:
                _posix_trash(p)
        else:
            if os.path.islink(p) or os.path.isfile(p):
                os.unlink(p)
            else:
                shutil.rmtree(p)
        deleted.append(p)
    return deleted


def plan_delete(targets, roots, dry_run):
    resolved = []
    for t in targets:
        full = resolve_scoped(t, roots)
        ensure_not_system_root(full)
        ensure_not_allow_root(full, roots)
        ensure_not_protected(full)
        cls, ev = classify(full, roots)
        size, files, dirs = measure(full)
        resolved.append({
            "input": t,
            "absolute": full,
            "class": cls,
            "evidence": ev,
            "size_bytes": size,
            "files": files,
            "dirs": dirs,
        })
    total = sum(r["size_bytes"] for r in resolved)
    classes = {r["class"] for r in resolved}
    cap = max((recycle_capacity(r["absolute"]) for r in resolved), default=0)
    if "protected" in classes:
        protected_evidence = next((r["evidence"] for r in resolved if r["class"] == "protected"),
                                  "protected pattern matched")
        decision = {"decision": "reject", "mode": None,
                    "reason": "protected: %s; refuse deletion" % protected_evidence}
    elif classes == {"cache"}:
        decision = {"decision": "permit", "mode": "permanent",
                    "reason": "all targets classified as cache; evidence recorded per target"}
    elif total <= cap:
        decision = {"decision": "permit", "mode": "trash",
                    "reason": "non-cache within capacity (%d <= %d)" % (total, cap)}
    else:
        decision = {"decision": "reject", "mode": None,
                    "reason": "non-cache over capacity (%d > %d); permanent delete refused" % (total, cap)}
    if decision["decision"] == "reject":
        raise PolicyError(decision["reason"], detail={"plan": resolved, "decision": decision})
    if dry_run:
        return {"plan": resolved, "decision": decision, "dry_run": True, "deleted": []}, decision
    deleted = delete_paths([r["absolute"] for r in resolved], decision["mode"], roots)
    return {"plan": resolved, "decision": decision, "dry_run": False, "deleted": deleted}, decision


def _file_read(raw, roots):
    max_bytes = raw.get("max_bytes", DEFAULT_MAX_READ_BYTES)
    out = []
    total = 0
    for t in raw["targets"]:
        full = resolve_scoped(t, roots)
        if not os.path.isfile(full):
            raise ExecutionError("not a file: %s" % full)
        if os.path.getsize(full) > max_bytes:
            raise ExecutionError("file exceeds max_bytes: %s" % full)
        total += os.path.getsize(full)
        if total > TOTAL_READ_LIMIT_BYTES:
            raise ExecutionError("aggregate read size exceeds %d bytes" % TOTAL_READ_LIMIT_BYTES)
        with open(full, "rb") as fh:
            data = fh.read()
        content, encoding = decode_text(data)
        out.append({"path": full, "content": content, "encoding": encoding})
    return {"files": out}


def _file_write(raw, roots):
    full = resolve_scoped(raw["targets"][0], roots)
    ensure_not_protected(full)
    parent = os.path.dirname(full) or "."
    os.makedirs(parent, exist_ok=True)
    fd, tmp = tempfile.mkstemp(prefix=".ops-write-", dir=parent)
    try:
        with os.fdopen(fd, "w", encoding="utf-8", newline="") as fh:
            fh.write(raw["content"])
        os.replace(tmp, full)
    finally:
        if os.path.exists(tmp):
            try:
                os.unlink(tmp)
            except OSError:
                pass
    return {"path": full, "bytes": len(raw["content"].encode("utf-8"))}


def _move_copy(raw, roots, copy_mode):
    src = resolve_scoped(raw["targets"][0], roots)
    dest = resolve_scoped(raw["dest"], roots)
    ensure_not_protected(src)
    ensure_not_protected(dest)
    if not os.path.exists(src):
        raise ExecutionError("source does not exist: %s" % src)
    if os.path.isdir(dest):
        dest = os.path.join(dest, os.path.basename(src))
    if copy_mode:
        if os.path.isdir(src):
            shutil.copytree(src, dest, dirs_exist_ok=True)
        else:
            shutil.copy2(src, dest)
    else:
        shutil.move(src, dest)
    return {"source": src, "dest": dest}


def _dir_list(raw, roots):
    depth = raw.get("depth", 1)
    max_entries = raw.get("max_entries", 10000)
    out = []
    global_entries = {"n": 0}
    for t in raw["targets"]:
        full = resolve_scoped(t, roots)
        if not os.path.isdir(full):
            raise ExecutionError("not a directory: %s" % full)
        entries = []

        def walk(d, level):
            if len(entries) >= max_entries or global_entries["n"] >= MAX_TOTAL_DIR_ENTRIES:
                return
            try:
                with os.scandir(d) as it:
                    for e in it:
                        if len(entries) >= max_entries or global_entries["n"] >= MAX_TOTAL_DIR_ENTRIES:
                            break
                        is_dir = False
                        try:
                            is_dir = e.is_dir(follow_symlinks=False)
                        except OSError:
                            pass
                        size = 0
                        if not is_dir:
                            try:
                                size = e.stat(follow_symlinks=False).st_size
                            except OSError:
                                pass
                        entries.append({"name": e.name, "type": "dir" if is_dir else "file", "size": size})
                        global_entries["n"] += 1
                        if is_dir and level < depth:
                            walk(e.path, level + 1)
            except OSError as ex:
                entries.append({"name": "<error: %s>" % ex, "type": "error", "size": 0})

        walk(full, 1)
        out.append({"path": full, "entries": entries,
                    "truncated": len(entries) >= max_entries or global_entries["n"] >= MAX_TOTAL_DIR_ENTRIES})
    return {"dirs": out}


def _process_run(raw, roots):
    if not roots:
        raise PolicyError("process.run requires allow roots (OPS_ALLOW_ROOTS or --allow-root)")
    exe = raw["exe"]
    args = raw.get("args", [])
    cwd = resolve_scoped(raw["cwd"], roots) if "cwd" in raw else roots[0]
    timeout = raw.get("timeout_ms", DEFAULT_PROCESS_TIMEOUT_MS) / 1000.0
    resolved_exe = exe if os.path.isabs(exe) else shutil.which(exe)
    if not resolved_exe:
        raise ExecutionError("executable not found: %s" % exe)
    real_exe = os.path.realpath(os.path.abspath(resolved_exe))
    allowlist = get_process_allowlist()
    allowed = _inside(real_exe, roots)
    if not allowed:
        base = os.path.basename(real_exe)
        stem = os.path.splitext(base)[0]
        for entry in allowlist:
            if os.path.isabs(entry):
                if os.path.normcase(os.path.realpath(os.path.abspath(entry))) == os.path.normcase(real_exe):
                    allowed = True
                    break
            else:
                entry_norm = os.path.normcase(entry)
                if entry_norm in (os.path.normcase(base), os.path.normcase(stem)):
                    allowed = True
                    break
    if not allowed:
        raise PolicyError("process.run executable not authorized: %s (must be inside allow roots or OPS_PROCESS_ALLOW)" % real_exe)
    try:
        env = os.environ.copy()
        env["PYTHONIOENCODING"] = "utf-8"
        env["PYTHONUTF8"] = "1"
        cp = subprocess.run([resolved_exe] + list(args), cwd=cwd,
                            capture_output=True, timeout=timeout, shell=False, env=env)
    except subprocess.TimeoutExpired:
        raise ExecutionError("process timed out after %gs" % timeout)
    stdout_text, stdout_enc = decode_text(cp.stdout)
    stderr_text, stderr_enc = decode_text(cp.stderr)
    return {
        "exe": resolved_exe,
        "exit_code": cp.returncode,
        "stdout_tail": stdout_text[-OUTPUT_TAIL_BYTES:],
        "stderr_tail": stderr_text[-OUTPUT_TAIL_BYTES:],
        "stdout_encoding": stdout_enc,
        "stderr_encoding": stderr_enc,
    }


def env_info(roots):
    return {
        "host": "windows" if os.name == "nt" else "posix",
        "platform": platform.platform(),
        "python": platform.python_version(),
        "shell": "powershell" if os.name == "nt" else os.environ.get("SHELL", "sh"),
        "path_style": "windows" if os.name == "nt" else "posix",
        "eol": "crlf" if os.name == "nt" else "lf",
        "encoding": "utf-8",
        "cwd": os.getcwd(),
        "allow_roots": roots,
        "audit_log": audit_path(),
    }


def audit_path():
    return os.environ.get("OPS_AUDIT_LOG") or os.path.join(os.getcwd(), "ops-audit.jsonl")


def execute(op, raw, roots):
    if op == "file.delete":
        return plan_delete(raw["targets"], roots, bool(raw.get("dry_run")))
    if op == "file.read":
        return _file_read(raw, roots), None
    if op == "file.write":
        return _file_write(raw, roots), None
    if op == "file.move":
        return _move_copy(raw, roots, copy_mode=False), None
    if op == "file.copy":
        return _move_copy(raw, roots, copy_mode=True), None
    if op == "dir.list":
        return _dir_list(raw, roots), None
    if op == "process.run":
        return _process_run(raw, roots), None
    if op == "env.info":
        return env_info(roots), None
    raise InvalidOp("unknown op: %s" % op)


def write_audit(entry):
    path = audit_path()
    parent = os.path.dirname(path)
    if parent:
        os.makedirs(parent, exist_ok=True)
    with open(path, "a", encoding="utf-8") as fh:
        fh.write(json.dumps(entry, ensure_ascii=False) + "\n")


def main(argv=None):
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    except Exception:
        pass
    parser = argparse.ArgumentParser(description="Structured Operation Protocol v0.1 executor")
    parser.add_argument("--op-file", help="read operation JSON from file instead of stdin ('-' = stdin)")
    parser.add_argument("--allow-root", action="append", default=[], help="allow root (repeatable)")
    parser.add_argument("--audit-log", help="audit JSONL path (overrides OPS_AUDIT_LOG)")
    args = parser.parse_args(argv)
    if args.audit_log:
        os.environ["OPS_AUDIT_LOG"] = args.audit_log

    started = time.monotonic()
    raw = None
    op = None
    request_id = "REQ-" + uuid.uuid4().hex[:12].upper()
    roots = []
    result = None
    policy = {"decision": "n/a", "mode": None, "reason": None}
    status = "error"
    detail = None
    resolved_audit = []
    output_encoding = None

    try:
        if args.op_file and args.op_file != "-":
            with open(args.op_file, "r", encoding="utf-8") as fh:
                raw = json.load(fh)
        else:
            raw = json.load(sys.stdin)
    except Exception as ex:
        status, detail = "invalid", "invalid JSON: %s" % ex

    if raw is not None:
        req = raw.get("id") if isinstance(raw, dict) else None
        if isinstance(req, str) and re.fullmatch(r"[A-Za-z0-9._-]{1,80}", req):
            request_id = req
        try:
            validate_op(raw)
            op = raw["op"]
            roots = get_allow_roots(args.allow_root)
            result, policy = execute(op, raw, roots)
            if policy is None:
                policy = {"decision": "n/a", "mode": None, "reason": None}
            if op == "file.delete":
                if result.get("dry_run"):
                    status = "dry-run"
                else:
                    status = "ok"
                resolved_audit = [
                    {k: r[k] for k in ("input", "absolute", "class", "evidence", "size_bytes", "files", "dirs")}
                    for r in result["plan"]
                ]
            else:
                status = "ok"
                for t in raw.get("targets", []):
                    try:
                        full = resolve_scoped(t, roots, require_inside=False)
                        resolved_audit.append({"input": t, "absolute": full, "class": "n/a", "evidence": "not a delete operation"})
                    except Exception:
                        resolved_audit.append({"input": t, "absolute": None, "class": "n/a", "evidence": "resolution failed"})
                if op == "file.read":
                    encs = [f.get("encoding") for f in result.get("files", []) if f.get("encoding")]
                    output_encoding = ",".join(dict.fromkeys(encs)) or None
                elif op == "process.run":
                    encs = [result.get("stdout_encoding"), result.get("stderr_encoding")]
                    encs = [e for e in encs if e]
                    output_encoding = ",".join(dict.fromkeys(encs)) or None
            exit_code = 0
        except InvalidOp as ex:
            status, detail, exit_code = "invalid", str(ex), 1
        except PolicyError as ex:
            status, detail, exit_code = "rejected", str(ex), 2
            if ex.detail:
                resolved_audit = [
                    {k: r[k] for k in ("input", "absolute", "class", "evidence", "size_bytes", "files", "dirs")}
                    for r in ex.detail.get("plan", [])
                ]
                policy = ex.detail.get("decision", policy)
        except (ExecutionError, OSError, subprocess.SubprocessError) as ex:
            status, detail, exit_code = "error", str(ex), 1
        except Exception as ex:
            status, detail, exit_code = "error", "%s: %s" % (type(ex).__name__, ex), 1
    else:
        exit_code = 1

    duration_ms = int((time.monotonic() - started) * 1000)
    audit_op = "?"
    audit_targets = []
    if isinstance(raw, dict):
        audit_op = raw["op"] if isinstance(raw.get("op"), str) and raw["op"] in OP_SPECS else "?"
        tg = raw.get("targets")
        if isinstance(tg, list):
            audit_targets = [t for t in tg if isinstance(t, str)]
    audit_entry = {
        "schema_version": "0.1.0-draft",
        "audit_kind": "structured_operation",
        "request_id": request_id,
        "timestamp": _dt.datetime.now(_dt.timezone.utc).strftime("%Y-%m-%dT%H:%M:%S.%f")[:-3] + "Z",
        "op": audit_op,
        "targets": audit_targets,
        "resolved": resolved_audit,
        "policy": policy,
        "result": {"status": status, "detail": detail, "exit_code": exit_code, "duration_ms": duration_ms},
        "env": env_info(roots),
        "source": os.environ.get("OPS_SOURCE", "local"),
        "output_encoding": output_encoding,
    }
    try:
        write_audit(audit_entry)
    except Exception as ex:
        detail = ((detail + "; ") if detail else "") + "audit write failed: %s" % ex
        status = "error"
        exit_code = 1

    if status == "rejected":
        ok = False
    elif status in ("ok", "dry-run"):
        ok = True
    else:
        ok = False
    out = {
        "ok": ok,
        "request_id": request_id,
        "op": audit_op,
        "status": status,
        "result": result if result is not None else {"detail": detail},
        "audit": audit_entry["result"],
    }
    print(json.dumps(out, ensure_ascii=False, indent=2))
    return exit_code


if __name__ == "__main__":
    sys.exit(main())
