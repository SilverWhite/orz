#!/usr/bin/env python3
"""TB 2.1 official 89-task corpus freeze manifest (P2-15 / EVALUATION-CORPUS-FREEZE S1).

The exam corpus of the TB 2.1 V4.1 generation round is the pinned Harbor
dataset ``terminal-bench/terminal-bench-2-1@sha256:7d7bdc1c...`` as checked
out at ``D:\\tb-eval\\terminal-bench-2-1``.  This tool freezes, from primary
sources and without re-typing any list by hand:

1. the dataset pin — parsed out of the leaderboard's own ``hub.py``;
2. the batch composition — parsed out of the runner script that the round
   actually drives (``run_official_2.1.sh``);
3. the local checkout commit + working-tree cleanliness;
4. a deterministic per-task content digest, split into three component
   digests: model-visible prompt / task environment / verifier-side oracle;
5. the harness-side artifacts that define the round's execution contract
   (runner script, ``tb_agents.orz`` adapter, orz carriers);
6. the corpus's own dataset registry (``tasks/dataset.toml``) and the
   Harbor-recorded per-task identity digests found in the existing official
   ledger (``jobs-official/**/lock.json`` and trial ``result.json``) as two
   independent cross-checks that the local corpus is the one that ran.

Digest scheme (deterministic, documented): for a file group ``S``,

    digest = SHA256( concat over paths sorted by POSIX relpath of
                     f"{sha256(file_bytes)}  {relpath}\\n" )

Read-only with respect to the corpus and the ledger; the only write is the
manifest produced at ``--out``.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
from pathlib import Path

SCHEMA = "tb21-corpus-freeze-v0.1"

# Groups are classified by task-relative path prefix.  The split matters:
# only ``prompt`` and ``environment`` can reach the model's execution
# context; ``oracle`` must never be reachable from it.
ORACLE_PREFIXES = ("tests/", "solution/")
ENVIRONMENT_PREFIXES = ("environment/",)
ENVIRONMENT_ROOT_FILES = {
    "Dockerfile",
    "docker-compose.yaml",
    "docker-compose.yml",
    ".dockerignore",
}


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as fh:
        for chunk in iter(lambda: fh.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def is_binary(data: bytes) -> bool:
    return b"\x00" in data[:8192]


def normalized_digest(path: Path, *, normalize_eol: bool) -> str:
    """File digest, optionally after CRLF/CR -> LF normalisation.

    The local checkout uses ``core.autocrlf=true`` while the registry serves
    LF content, so a raw byte comparison flags every text file.  Normalising
    line endings (only for non-binary files) isolates real content changes.
    """
    data = path.read_bytes()
    if normalize_eol and not is_binary(data):
        data = data.replace(b"\r\n", b"\n").replace(b"\r", b"\n")
    return hashlib.sha256(data).hexdigest()


def group_digest(entries: list[tuple[str, str]]) -> str:
    """entries = [(relpath, file_sha256)]; returns the group digest."""
    body = "".join(f"{digest}  {relpath}\n" for relpath, digest in sorted(entries))
    return hashlib.sha256(body.encode("utf-8")).hexdigest()


def classify(relpath: str) -> str:
    posix = relpath.replace("\\", "/")
    if posix.startswith(ORACLE_PREFIXES):
        return "oracle"
    if posix.startswith(ENVIRONMENT_PREFIXES) or posix in ENVIRONMENT_ROOT_FILES:
        return "environment"
    if posix == "instruction.md":
        return "prompt"
    return "task_metadata"


def iter_task_files(task_dir: Path):
    for root, dirs, files in os.walk(task_dir):
        dirs[:] = sorted(d for d in dirs if d != ".git")
        for name in sorted(files):
            full = Path(root) / name
            rel = full.relative_to(task_dir).as_posix()
            yield rel, full


def parse_batches(runner_script: Path) -> dict[str, list[str]]:
    """Parse `BATCH<n>=(a b c)` lines out of the runner shell script."""
    batches: dict[str, list[str]] = {}
    pattern = re.compile(r"^BATCH(\d+)=\((.*)\)\s*$")
    for line in runner_script.read_text(encoding="utf-8").splitlines():
        m = pattern.match(line.strip())
        if m:
            batches[f"BATCH{m.group(1)}"] = m.group(2).split()
    if not batches:
        raise SystemExit(f"no BATCH<n>=(...) lines found in {runner_script}")
    return batches


def parse_runner_pin(runner_script: Path) -> str | None:
    for line in runner_script.read_text(encoding="utf-8").splitlines():
        if line.strip().startswith("DATASET="):
            return line.split("=", 1)[1].strip().strip('"')
    return None


def parse_leaderboard_pin(hub_py: Path) -> dict[str, str | None]:
    pin: dict[str, str | None] = {"dataset": None, "dataset_ref": None}
    if not hub_py.is_file():
        return pin
    for line in hub_py.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if line.startswith("DATASET_REF") and "=" in line:
            pin["dataset_ref"] = line.split("=", 1)[1].strip().strip('"')
        elif line.startswith("DATASET") and "=" in line:
            pin["dataset"] = line.split("=", 1)[1].strip().strip('"')
    return pin


def git_probe(repo: Path) -> dict[str, object]:
    def run(*args: str) -> subprocess.CompletedProcess:
        return subprocess.run(
            ["git", "-c", f"safe.directory={repo.as_posix()}", "-C", str(repo), *args],
            capture_output=True,
            text=True,
        )

    info: dict[str, object] = {
        "repo": str(repo),
        "head": None,
        "clean": None,
        "error": None,
    }
    head = run("rev-parse", "HEAD")
    if head.returncode != 0:
        info["error"] = head.stderr.strip().splitlines()[:1]
        return info
    info["head"] = head.stdout.strip()
    status = run("status", "--porcelain")
    info["clean"] = status.returncode == 0 and not status.stdout.strip()
    return info


def parse_task_metadata(task_toml: Path) -> dict[str, object]:
    """Minimal task.toml reader (stdlib tomllib, py>=3.11)."""
    try:
        import tomllib
    except ModuleNotFoundError:  # pragma: no cover
        return {"parse_error": "tomllib unavailable"}
    try:
        data = tomllib.loads(task_toml.read_text(encoding="utf-8"))
    except Exception as exc:  # noqa: BLE001 - manifest tolerates parse failure
        return {"parse_error": f"{type(exc).__name__}: {exc}"}
    meta = data.get("metadata", {}) if isinstance(data.get("metadata"), dict) else {}
    task = data.get("task", {}) if isinstance(data.get("task"), dict) else {}
    verifier = data.get("verifier", {}) if isinstance(data.get("verifier"), dict) else {}
    agent = data.get("agent", {}) if isinstance(data.get("agent"), dict) else {}
    env = data.get("environment", {}) if isinstance(data.get("environment"), dict) else {}
    return {
        "schema_version": data.get("schema_version"),
        "name": task.get("name"),
        "difficulty": meta.get("difficulty"),
        "category": meta.get("category"),
        "tags": meta.get("tags"),
        "estimated_duration_sec": meta.get("estimated_duration_sec"),
        "expert_time_estimate_min": meta.get("expert_time_estimate_min"),
        "junior_time_estimate_min": meta.get("junior_time_estimate_min"),
        "agent_timeout_sec": agent.get("timeout_sec"),
        "verifier_timeout_sec": verifier.get("timeout_sec"),
        "environment_build_timeout_sec": env.get("build_timeout_sec"),
        "docker_image": env.get("docker_image"),
        "cpus": env.get("cpus"),
        "memory_mb": env.get("memory_mb"),
        "allow_internet": env.get("allow_internet"),
    }


def parse_dataset_registry(dataset_toml: Path) -> dict[str, object]:
    """Read the corpus's own registry: dataset identity + per-task digests."""
    try:
        import tomllib
    except ModuleNotFoundError:  # pragma: no cover
        return {"parse_error": "tomllib unavailable"}
    try:
        data = tomllib.loads(dataset_toml.read_text(encoding="utf-8"))
    except Exception as exc:  # noqa: BLE001
        return {"parse_error": f"{type(exc).__name__}: {exc}"}
    dataset = data.get("dataset", {}) if isinstance(data.get("dataset"), dict) else {}
    tasks = data.get("tasks", []) or []
    return {
        "name": dataset.get("name"),
        "description": dataset.get("description"),
        "keywords": dataset.get("keywords"),
        "task_count": len(tasks),
        "digests": {
            entry["name"]: entry.get("digest")
            for entry in tasks
            if isinstance(entry, dict) and entry.get("name")
        },
    }


def load_registry_digests(path: Path) -> dict[str, object]:
    data = json.loads(path.read_text(encoding="utf-8"))
    data.setdefault("tasks", {})
    return data


def collect_ledger_identity(jobs_root: Path) -> dict[str, dict[str, object]]:
    """Per-task Harbor identity digests recorded by the existing ledger.

    Two independent recorded sources are used:
      * job ``lock.json`` -> ``trials[].task.{name,digest}``
      * trial ``result.json`` -> ``task_id.ref`` and ``task_checksum``
    """
    observed: dict[str, dict[str, object]] = {}

    def normalize(task: str) -> str:
        return task.split("/", 1)[1] if task.startswith("terminal-bench/") else task

    def slot(task: str) -> dict[str, object]:
        return observed.setdefault(
            normalize(task),
            {
                "lock_job_count": 0,
                "lock_digests": set(),
                "result_trial_count": 0,
                "result_refs": set(),
                "result_checksums": set(),
                "sources": set(),
            },
        )

    if not jobs_root.is_dir():
        return {}

    for lock in jobs_root.rglob("lock.json"):
        try:
            data = json.loads(lock.read_text(encoding="utf-8"))
        except Exception:  # noqa: BLE001 - an unreadable lock is not a blocker
            continue
        for trial in data.get("trials", []) or []:
            task_spec = trial.get("task") or {}
            task_name = task_spec.get("name")
            if not task_name:
                continue
            rec = slot(task_name)
            rec["lock_job_count"] = int(rec["lock_job_count"]) + 1
            if task_spec.get("digest"):
                rec["lock_digests"].add(task_spec["digest"])

    for result in jobs_root.rglob("result.json"):
        try:
            data = json.loads(result.read_text(encoding="utf-8"))
        except Exception:  # noqa: BLE001
            continue
        tid = data.get("task_id") or {}
        task_name = tid.get("name") or result.parent.name.split("__", 1)[0]
        rec = slot(task_name)
        rec["result_trial_count"] = int(rec["result_trial_count"]) + 1
        if tid.get("ref"):
            rec["result_refs"].add(tid["ref"])
        if data.get("task_checksum"):
            rec["result_checksums"].add(data["task_checksum"])
        if data.get("source"):
            rec["sources"].add(data["source"])

    for rec in observed.values():
        for key in ("lock_digests", "result_refs", "result_checksums", "sources"):
            rec[key] = sorted(rec[key])
    return observed


def artifact_entry(path: Path, *, hash_it: bool = True) -> dict[str, object]:
    if not path.exists():
        return {"path": str(path), "present": False}
    entry: dict[str, object] = {
        "path": str(path),
        "present": True,
        "bytes": path.stat().st_size,
    }
    if hash_it:
        entry["sha256"] = sha256_file(path)
    return entry


def task_tree(root: Path, name: str, *, normalize_eol: bool = False) -> dict[str, object]:
    """Per-file digests for one task directory (relpath -> sha256)."""
    task_dir = root / name
    files = {
        rel: normalized_digest(full, normalize_eol=normalize_eol)
        for rel, full in iter_task_files(task_dir)
    }
    return {
        "tree_sha256": group_digest(list(files.items())),
        "files": files,
        "bytes": sum((task_dir / rel.replace("/", os.sep)).stat().st_size for rel in files),
    }


def compare_roots(
    corpus_root: Path,
    other_root: Path,
    frozen: dict[str, str],
    *,
    normalize_eol: bool,
    ignore: set[str],
) -> dict[str, object]:
    """Compare task content between the local checkout and a second root.

    Typical use: a freshly downloaded copy of the published dataset version
    (``harbor download <dataset>@<ref>``).
    """
    tasks_root = other_root / "tasks" if (other_root / "tasks").is_dir() else other_root
    other_names = sorted(p.name for p in tasks_root.iterdir() if p.is_dir())

    only_other = sorted(set(other_names) - set(frozen))
    only_frozen = sorted(set(frozen) - set(other_names))
    mismatched: list[str] = []
    detail: dict[str, object] = {}

    self_root = corpus_root / "tasks"
    for name in sorted(set(other_names) & set(frozen)):
        mine = task_tree(self_root, name, normalize_eol=normalize_eol)
        theirs = task_tree(tasks_root, name, normalize_eol=normalize_eol)
        if ignore:
            mine = {**mine, "files": {k: v for k, v in mine["files"].items() if k not in ignore}}
            theirs = {**theirs, "files": {k: v for k, v in theirs["files"].items() if k not in ignore}}
            mine["tree_sha256"] = group_digest(list(mine["files"].items()))
            theirs["tree_sha256"] = group_digest(list(theirs["files"].items()))
        if mine["tree_sha256"] == theirs["tree_sha256"]:
            continue
        mismatched.append(name)
        mine_files, their_files = mine["files"], theirs["files"]
        differing = sorted(
            rel
            for rel in mine_files.keys() & their_files.keys()
            if mine_files[rel] != their_files[rel]
        )
        detail[name] = {
            "files_only_local": sorted(mine_files.keys() - their_files.keys()),
            "files_only_other": sorted(their_files.keys() - mine_files.keys()),
            "files_differing": differing,
        }

    matched = len(set(other_names) & set(frozen)) - len(mismatched)
    return {
        "other_root": str(other_root),
        "mode": "raw_bytes" if not normalize_eol else "eol_normalised",
        "ignored_names": sorted(ignore),
        "tasks_compared": len(set(other_names) & set(frozen)),
        "tasks_matched": matched,
        "content_mismatches": mismatched,
        "only_in_other_root": only_other,
        "only_in_frozen_set": only_frozen,
        "detail": detail,
    }


def print_comparison(result: dict[str, object]) -> int:
    other_root = result["other_root"]
    print(f"compared against      : {other_root}")
    print(f"mode                  : {result['mode']}"
          + (f", ignoring {result['ignored_names']}" if result["ignored_names"] else ""))
    print(f"tasks compared        : {result['tasks_compared']}")
    print(f"matched               : {result['tasks_matched']}")
    print(f"content mismatches    : {result['content_mismatches'] or 'none'}")
    print(f"only in other root    : {result['only_in_other_root'] or 'none'}")
    print(f"only in frozen set    : {result['only_in_frozen_set'] or 'none'}")
    for name, info in result["detail"].items():
        print(f"  - {name}: {json.dumps(info, ensure_ascii=False)}")
    differs = bool(
        result["content_mismatches"] or result["only_in_other_root"] or result["only_in_frozen_set"]
    )
    return 1 if differs else 0


def build_manifest(args) -> dict[str, object]:
    corpus_root: Path = args.corpus_root
    tasks_root = corpus_root / "tasks"
    runner_script: Path = args.runner_script
    hub_py = corpus_root / "leaderboard" / "src" / "leaderboard" / "core" / "hub.py"
    dataset_toml = tasks_root / "dataset.toml"

    batches = parse_batches(runner_script)
    ordered = [task for batch in sorted(batches) for task in batches[batch]]
    duplicates = sorted({t for t in ordered if ordered.count(t) > 1})
    batch_set = set(ordered)

    local_tasks = sorted(p.name for p in tasks_root.iterdir() if p.is_dir())
    local_set = set(local_tasks)
    ledger = collect_ledger_identity(args.jobs_root)
    registry = parse_dataset_registry(dataset_toml) if dataset_toml.is_file() else {}
    registry_names = {
        name.split("/", 1)[1] for name in (registry.get("digests") or {}) if "/" in name
    }
    canonical = load_registry_digests(args.registry_digests) if args.registry_digests else None
    canonical_digests: dict[str, str] = (canonical or {}).get("tasks", {})
    canonical_names = {
        name.split("/", 1)[1] for name in canonical_digests if "/" in name
    }

    task_entries = []
    for name in local_tasks:
        task_dir = tasks_root / name
        groups: dict[str, list[tuple[str, str]]] = {
            "prompt": [],
            "environment": [],
            "oracle": [],
            "task_metadata": [],
        }
        bytes_by_group = {k: 0 for k in groups}
        for rel, full in iter_task_files(task_dir):
            digest = sha256_file(full)
            group = classify(rel)
            groups[group].append((rel, digest))
            bytes_by_group[group] += full.stat().st_size

        all_entries = [e for g in groups.values() for e in g]
        metadata: dict[str, object] = {}
        task_toml = task_dir / "task.toml"
        if task_toml.is_file():
            metadata = parse_task_metadata(task_toml)

        ledger_rec = ledger.get(name, {})
        dataset_digest = (registry.get("digests") or {}).get(f"terminal-bench/{name}")
        canonical_digest = canonical_digests.get(f"terminal-bench/{name}")
        lock_digests = ledger_rec.get("lock_digests", [])
        result_refs = ledger_rec.get("result_refs", [])
        task_entries.append(
            {
                "task": name,
                "task_qualified_name": f"terminal-bench/{name}",
                "batches": sorted(b for b, tasks in batches.items() if name in tasks),
                "file_count": len(all_entries),
                "bytes": sum(bytes_by_group.values()),
                "tree_sha256": group_digest(all_entries),
                "prompt_sha256": group_digest(groups["prompt"]),
                "environment_sha256": group_digest(groups["environment"]),
                "oracle_sha256": group_digest(groups["oracle"]),
                "task_metadata_sha256": group_digest(groups["task_metadata"]),
                "group_file_counts": {k: len(v) for k, v in groups.items()},
                "group_bytes": bytes_by_group,
                "metadata": metadata,
                "dataset_digest": dataset_digest,
                "registry_digest": canonical_digest,
                "ledger_identity": {
                    "lock_job_count": ledger_rec.get("lock_job_count", 0),
                    "lock_digests": lock_digests,
                    "result_trial_count": ledger_rec.get("result_trial_count", 0),
                    "result_refs": result_refs,
                    "result_checksums": ledger_rec.get("result_checksums", []),
                    "sources": ledger_rec.get("sources", []),
                },
                "identity_agrees": {
                    "dataset_registry_vs_ledger_lock": (
                        bool(dataset_digest) and lock_digests == [dataset_digest]
                    ),
                    "dataset_registry_vs_ledger_ref": (
                        bool(dataset_digest) and result_refs == [dataset_digest]
                    ),
                    "registry_vs_ledger_lock": (
                        bool(canonical_digest) and lock_digests == [canonical_digest]
                    ),
                    "registry_vs_ledger_ref": (
                        bool(canonical_digest) and result_refs == [canonical_digest]
                    ),
                },
            }
        )

    corpus_digest = hashlib.sha256(
        "".join(f"{t['task']}\t{t['tree_sha256']}\n" for t in task_entries).encode("utf-8")
    ).hexdigest()

    drifted = [
        t["task"]
        for t in task_entries
        if len(t["ledger_identity"]["lock_digests"]) > 1
        or len(t["ledger_identity"]["result_refs"]) > 1
        or len(t["ledger_identity"]["result_checksums"]) > 1
    ]
    registry_mismatch = [
        t["task"]
        for t in task_entries
        if t["dataset_digest"]
        and t["ledger_identity"]["lock_digests"]
        and t["ledger_identity"]["lock_digests"] != [t["dataset_digest"]]
    ]
    ref_mismatch = [
        t["task"]
        for t in task_entries
        if t["dataset_digest"]
        and t["ledger_identity"]["result_refs"]
        and t["ledger_identity"]["result_refs"] != [t["dataset_digest"]]
    ]
    canonical_vs_lock = [
        t["task"]
        for t in task_entries
        if t["registry_digest"]
        and t["ledger_identity"]["lock_digests"]
        and t["ledger_identity"]["lock_digests"] != [t["registry_digest"]]
    ]
    canonical_vs_ref = [
        t["task"]
        for t in task_entries
        if t["registry_digest"]
        and t["ledger_identity"]["result_refs"]
        and t["ledger_identity"]["result_refs"] != [t["registry_digest"]]
    ]
    canonical_vs_local_registry = [
        t["task"]
        for t in task_entries
        if t["registry_digest"] and t["dataset_digest"] and t["registry_digest"] != t["dataset_digest"]
    ]

    artifacts = [artifact_entry(p) for p in args.artifact]
    env_record = None
    if args.env_file:
        env_record = artifact_entry(args.env_file, hash_it=False)
        if env_record.get("present"):
            env_record["note"] = (
                "credential-bearing env file; only presence/size recorded, value "
                "deliberately not hashed (key rotation must not invalidate the "
                "corpus freeze)"
            )

    coverage: dict[str, object] = {
        "tasks": len(task_entries),
        "batches": len(batches),
        "batch_sizes": {b: len(t) for b, t in sorted(batches.items())},
        "files": sum(t["file_count"] for t in task_entries),
        "bytes": sum(t["bytes"] for t in task_entries),
        "difficulty": {},
        "category": {},
    }
    for t in task_entries:
        meta = t["metadata"] if isinstance(t["metadata"], dict) else {}
        for field in ("difficulty", "category"):
            key = meta.get(field) or "unknown"
            bucket = coverage[field]
            bucket[key] = bucket.get(key, 0) + 1

    pin = parse_leaderboard_pin(hub_py)
    return {
        "schema": SCHEMA,
        "frozen_at": args.frozen_at,
        "purpose": (
            "P2-15 (EVALUATION-CORPUS-FREEZE) S1 corpus freeze for the TB 2.1 "
            "V4.1 generation round: make the round reproducible and later "
            "auditable by pinning exactly which tasks, task content and "
            "harness artifacts produced the data."
        ),
        "dataset": {
            "name": pin.get("dataset"),
            "pin": pin.get("dataset_ref"),
            "pin_source": str(hub_py),
            "runner_invocation_pin": parse_runner_pin(runner_script),
        },
        "local_checkout": git_probe(corpus_root),
        "corpus_root": str(corpus_root),
        "runner_script": artifact_entry(runner_script),
        "dataset_registry": {
            "path": str(dataset_toml),
            "present": dataset_toml.is_file(),
            "sha256": sha256_file(dataset_toml) if dataset_toml.is_file() else None,
            "name": registry.get("name"),
            "description": registry.get("description"),
            "keywords": registry.get("keywords"),
            "task_count": registry.get("task_count"),
            "tasks_readme_sha256": (
                sha256_file(tasks_root / "README.md")
                if (tasks_root / "README.md").is_file()
                else None
            ),
            "role": (
                "repo-local manifest that `harbor publish`/`harbor sync` keep in "
                "step with the registry; it can lag the published version "
                "(see consistency.registry_vs_local_dataset_manifest_mismatch)"
            ),
        },
        "canonical_registry": (
            {
                "snapshot_path": str(args.registry_digests),
                "dataset": canonical.get("dataset") if canonical else None,
                "ref": canonical.get("ref") if canonical else None,
                "fetched_at": canonical.get("fetched_at") if canonical else None,
                "task_count": len(canonical_digests),
                "role": (
                    "authoritative per-task identity of the pinned dataset ref; "
                    "task content the harness resolves at run time"
                ),
            }
            if canonical
            else None
        ),
        "batches": {b: tasks for b, tasks in sorted(batches.items())},
        "consistency": {
            "batch_union_size": len(batch_set),
            "batch_duplicates": duplicates,
            "tasks_missing_from_batches": sorted(local_set - batch_set),
            "tasks_not_in_checkout": sorted(batch_set - local_set),
            "registry_tasks_missing_from_batches": sorted(registry_names - batch_set),
            "registry_tasks_not_in_checkout": sorted(registry_names - local_set),
            "canonical_tasks_missing_from_batches": sorted(canonical_names - batch_set),
            "canonical_tasks_not_in_checkout": sorted(canonical_names - local_set),
            "registry_tasks_without_digest": sorted(
                t["task"] for t in task_entries if not t["dataset_digest"]
            ),
            "canonical_registry_vs_ledger_lock_mismatch": canonical_vs_lock,
            "canonical_registry_vs_ledger_ref_mismatch": canonical_vs_ref,
            "registry_vs_local_dataset_manifest_mismatch": canonical_vs_local_registry,
            "registry_digest_vs_ledger_lock_mismatch": registry_mismatch,
            "registry_digest_vs_ledger_ref_mismatch": ref_mismatch,
            "tasks_with_ledger_digest_drift": drifted,
            "tasks_without_ledger_identity": sorted(
                t["task"]
                for t in task_entries
                if not t["ledger_identity"]["lock_digests"]
                and not t["ledger_identity"]["result_refs"]
            ),
        },
        "coverage": coverage,
        "corpus_digest": corpus_digest,
        "harness_artifacts": artifacts,
        "env_file": env_record,
        "tasks": task_entries,
    }


def summarize(manifest: dict[str, object], out: Path) -> None:
    c = manifest["coverage"]
    cons = manifest["consistency"]
    print(f"tasks                 : {c['tasks']}")
    print(f"batches               : {c['batches']} {c['batch_sizes']}")
    print(f"files / bytes         : {c['files']} / {c['bytes']:,}")
    print(f"difficulty            : {c['difficulty']}")
    print(f"category              : {c['category']}")
    print(f"corpus digest         : {manifest['corpus_digest']}")
    print(f"dataset pin           : {manifest['dataset']['pin']}")
    print(f"dataset registry      : {manifest['dataset_registry']['task_count']} tasks, "
          f"sha256={str(manifest['dataset_registry']['sha256'])[:16]}...")
    canon = manifest.get("canonical_registry")
    if canon:
        print(f"canonical registry    : {canon['task_count']} tasks @ {canon['ref']}")
    print(f"checkout HEAD         : {manifest['local_checkout'].get('head')}")
    print(f"checkout clean        : {manifest['local_checkout'].get('clean')}")
    print(f"batch duplicates      : {cons['batch_duplicates'] or 'none'}")
    print(f"missing from batches  : {cons['tasks_missing_from_batches'] or 'none'}")
    print(f"not in checkout       : {cons['tasks_not_in_checkout'] or 'none'}")
    print(f"registry vs batches   : {(cons['registry_tasks_missing_from_batches'] or cons['registry_tasks_not_in_checkout']) or 'consistent'}")
    print(f"canonical vs batches  : {(cons['canonical_tasks_missing_from_batches'] or cons['canonical_tasks_not_in_checkout']) or 'consistent'}")
    print(f"canonical vs lock     : {cons['canonical_registry_vs_ledger_lock_mismatch'] or 'all match'}")
    print(f"canonical vs ledger ref: {cons['canonical_registry_vs_ledger_ref_mismatch'] or 'all match'}")
    print(f"canonical vs local toml: {cons['registry_vs_local_dataset_manifest_mismatch'] or 'all match'}")
    print(f"registry vs lock      : {cons['registry_digest_vs_ledger_lock_mismatch'] or 'all match'}")
    print(f"registry vs ledger ref: {cons['registry_digest_vs_ledger_ref_mismatch'] or 'all match'}")
    print(f"ledger digest drift   : {cons['tasks_with_ledger_digest_drift'] or 'none'}")
    print(f"no ledger identity    : {cons['tasks_without_ledger_identity'] or 'none'}")
    print(f"manifest              : {out}")


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--corpus-root", type=Path, default=Path("D:/tb-eval/terminal-bench-2-1"))
    ap.add_argument("--runner-script", type=Path, default=Path("D:/tb-eval/run_official_2.1.sh"))
    ap.add_argument("--jobs-root", type=Path, default=Path("D:/tb-eval/jobs-official"))
    ap.add_argument("--artifact", type=Path, action="append", default=[])
    ap.add_argument("--env-file", type=Path, default=None)
    ap.add_argument(
        "--registry-digests",
        type=Path,
        default=None,
        help="snapshot produced by scripts/fetch_tb21_registry_digests.py",
    )
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--frozen-at", default="2026-09-13")
    ap.add_argument(
        "--verify-against",
        type=Path,
        default=None,
        help=(
            "second content root (e.g. a freshly downloaded copy of the "
            "published dataset version) to compare task content against"
        ),
    )
    ap.add_argument(
        "--raw-bytes",
        action="store_true",
        help="with --verify-against: compare raw bytes (no EOL normalisation)",
    )
    ap.add_argument(
        "--ignore-name",
        action="append",
        default=[],
        help="with --verify-against: task-relative file names to ignore",
    )
    ap.add_argument(
        "--check",
        action="store_true",
        help="compare against the existing manifest at --out; fail on drift",
    )
    args = ap.parse_args(argv)

    manifest = build_manifest(args)
    comparison = None
    if args.verify_against:
        comparison = compare_roots(
            Path(args.corpus_root),
            args.verify_against,
            {t["task"]: t["tree_sha256"] for t in manifest["tasks"]},
            normalize_eol=not args.raw_bytes,
            ignore=set(args.ignore_name or []),
        )
        manifest["published_copy_comparison"] = comparison
    rendered = json.dumps(manifest, indent=2, ensure_ascii=False) + "\n"

    if args.check:
        if not args.out.is_file():
            print(f"no existing manifest at {args.out}", file=sys.stderr)
            return 2
        existing = json.loads(args.out.read_text(encoding="utf-8"))
        if existing.get("corpus_digest") != manifest["corpus_digest"]:
            print(
                "DRIFT: corpus digest changed\n"
                f"  recorded: {existing.get('corpus_digest')}\n"
                f"  current : {manifest['corpus_digest']}",
                file=sys.stderr,
            )
            return 1
        old = {t["task"]: t["tree_sha256"] for t in existing.get("tasks", [])}
        new = {t["task"]: t["tree_sha256"] for t in manifest["tasks"]}
        changed = sorted(k for k in old.keys() & new.keys() if old[k] != new[k])
        added = sorted(new.keys() - old.keys())
        removed = sorted(old.keys() - new.keys())
        if changed or added or removed:
            print(
                f"DRIFT: changed={changed} added={added} removed={removed}",
                file=sys.stderr,
            )
            return 1
        print(f"OK: corpus matches frozen manifest ({manifest['corpus_digest']})")
        return 0

    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(rendered, encoding="utf-8")
    summarize(manifest, args.out)
    return print_comparison(comparison) if comparison else 0


if __name__ == "__main__":
    raise SystemExit(main())
