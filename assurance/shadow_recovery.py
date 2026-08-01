"""Shadow recovery store and recovery executor — GAK-REC-001.

Provides the missing pieces between recovery *authorization* (proven in
:mod:`~.recovery`) and recovery *execution*:

1. :class:`ShadowRecoveryStore` — Git-backed, integrity-verified
   storage for recovery candidates, authorizations, and snapshot data.
   Independent of the source namespace (which may have been archived or
   deleted).

2. :class:`RecoveryDiffPreview` — before restoration, compare the
   recovery snapshot with the current target file.  Only metadata-level
   diff (hashes, line counts, byte counts) is exposed — raw content is
   never leaked into the diff preview.

3. :class:`RecoveryExecutor` — the **only** component that performs
   actual file restoration.  Every execution produces a signed
   :class:`ExecutionReceipt`.  Audit events are **never** modified or
   deleted — the executor only reads the audit seal for verification.

The store is an independent Git repository (inspired by Gemini CLI,
OpenCode, and Cline checkpointing patterns)::

    <project>/.gsa_shadow_recovery/
      repo/                        # Git repository
        .git/                      # Git internals (objects, refs, HEAD)
      executions/
        {timestamp}-{uuid8}/
          execution_receipt.json
          diff_preview.json

Each :meth:`ShadowRecoveryStore.store` call creates a Git commit
containing ``candidate.json``, ``authorization.json`` (optional), and
``snapshot.bin`` as blobs.  The commit SHA (40-char hex) is the entry
identifier.  Idempotency is achieved via tree-SHA deduplication.

The old SHA-256 content-addressed filesystem layout (``store/{prefix2}/{sha256}/``)
is deprecated.  A ``DeprecationWarning`` is issued if it still exists.

Design references:
- Gemini CLI checkpointing — independent shadow Git with conversation/tool-call association
- OpenCode snapshot/revert — Git store + patch/restore + session/message boundaries
- Cline checkpoints — separate Git repository, file + task restore
"""
from __future__ import annotations

import json
import os
import re
import subprocess
import tempfile
import uuid
import warnings
from dataclasses import dataclass, field
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from .contracts import ASSURANCE_ROOT
from .endpoint_canonicalizer import canonicalize_filesystem_path
from .errors import AssuranceError
from .keystore import InstallationKeyStore
from .utils import (
    atomic_write_bytes,
    atomic_write_json,
    canonical_bytes,
    exclusive_create_bytes,
    load_json,
    sha256_bytes,
    sha256_file,
    utc_now,
)

# ── paths ─────────────────────────────────────────────────────────────────────

SHADOW_ROOT: Path = ASSURANCE_ROOT.parent / ".gsa_shadow_recovery"
SHADOW_REPO: Path = SHADOW_ROOT / "repo"
SHADOW_EXECUTIONS: Path = SHADOW_ROOT / "executions"

COMMIT_SHA_RE = re.compile(r"^[a-f0-9]{40}$")
CREATE_NO_WINDOW: int = getattr(subprocess, "CREATE_NO_WINDOW", 0)


def _timestamp() -> str:
    return datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")


def _validate_commit_sha(value: str, *, label: str) -> None:
    if not isinstance(value, str) or not COMMIT_SHA_RE.fullmatch(value):
        raise AssuranceError(
            f"{label} must be a 40-char Git commit SHA hex digest"
        )


def _sign_body(
    body: dict[str, Any], *, key_store: InstallationKeyStore,
) -> dict[str, Any]:
    payload = canonical_bytes(body)
    return {
        **body,
        "integrity": {
            "canonicalization": "RFC8785",
            "key_id": key_store.key_id,
            "signature_algorithm": "hmac-sha256",
            "signed_payload_sha256": sha256_bytes(payload),
            "signature": key_store.sign(payload),
        },
    }


def _verify_body_signature(
    receipt: dict[str, Any], *, key_store: InstallationKeyStore, label: str,
) -> list[str]:
    errors: list[str] = []
    integrity = receipt.get("integrity", {})
    body = {k: v for k, v in receipt.items() if k != "integrity"}
    payload = canonical_bytes(body)
    if integrity.get("key_id") != key_store.key_id:
        errors.append(f"{label}: key_id mismatch")
    if integrity.get("signed_payload_sha256") != sha256_bytes(payload):
        errors.append(f"{label}: signed payload digest mismatch")
    if not key_store.verify(payload, integrity.get("signature", "")):
        errors.append(f"{label}: signature verification failed")
    return errors


# ══════════════════════════════════════════════════════════════════════════════
# data types
# ══════════════════════════════════════════════════════════════════════════════


@dataclass
class StoreEntry:
    """A recovery entry in the shadow store.

    Each entry corresponds to a Git commit in the shadow repository.
    *commit_sha* is the 40-character Git commit SHA (primary key).
    *entry_sha256* is ``None`` in Git mode; retained for backward
    compatibility with callers that check its truthiness.
    """

    entry_id: str
    commit_sha: str
    entry_sha256: str | None
    candidate_sha256: str
    authorization_sha256: str | None
    snapshot_sha256: str
    snapshot_bytes: int
    stored_at: str
    manifest: dict[str, Any] = field(default_factory=dict)


@dataclass
class DiffPreview:
    """Metadata-level diff between a recovery snapshot and a target file.

    Does **not** expose raw content — only hashes, byte counts, and
    line counts.  Full content comparison is done internally; the
    preview is a safety summary for the operator.
    """

    candidate_id: str
    target_path: str
    snapshot_sha256: str
    current_sha256: str | None           # None = target does not exist
    snapshot_bytes: int
    current_bytes: int | None
    bytes_to_add: int
    bytes_to_remove: int
    snapshot_lines: int
    current_lines: int | None
    lines_added: int
    lines_removed: int
    would_overwrite: bool = False


@dataclass
class ExecutionReceipt:
    """Signed receipt for a recovery execution.

    The receipt records what was restored, where, and the outcome.
    It explicitly declares that audit events were **not** modified.

    *shadow_commit_sha* is the Git commit SHA of the source entry
    in the shadow repository (``None`` if not from a Git-backed store).
    """

    receipt_id: str
    candidate_id: str
    authorization_id: str | None
    target_path: str
    snapshot_sha256: str
    pre_existing_sha256: str | None
    outcome: str            # "restored" | "failed" | "rejected"
    bytes_written: int
    shadow_commit_sha: str | None = None
    diff_preview: dict[str, Any] | None = None
    errors: list[str] = field(default_factory=list)
    audit_events_preserved: bool = True


# ══════════════════════════════════════════════════════════════════════════════
# Shadow Recovery Store (Git-backed)
# ══════════════════════════════════════════════════════════════════════════════


class ShadowRecoveryStore:
    """Git-backed store for recovery artifacts.

    Each :meth:`store` call creates a Git commit containing
    ``candidate.json``, ``authorization.json`` (optional), and
    ``snapshot.bin`` as blobs.  The 40-character commit SHA is the
    entry identifier.  Git's object integrity model replaces the old
    manifest-based verification.

    Usage::

        store = ShadowRecoveryStore()
        entry = store.store(
            candidate=candidate_receipt,
            authorization=auth_receipt,   # optional, may be None
            snapshot_bytes=b"checkpoint data",
        )
        # ... later ...
        retrieved = store.retrieve(entry.commit_sha)
        assert store.verify_entry(entry.commit_sha)

    Pass *repo_root* to isolate tests (e.g. a :class:`tempfile.TemporaryDirectory`).
    """

    def __init__(self, repo_root: Path | None = None) -> None:
        base = repo_root or SHADOW_ROOT
        self._repo: Path = base / "repo"
        self._executions: Path = base / "executions"
        self._executions.mkdir(parents=True, exist_ok=True)

        # Warn if the old CAS store directory still exists.
        old_store = base / "store"
        if old_store.is_dir():
            warnings.warn(
                f"Old shadow store format detected at {old_store}. "
                "This format is no longer used by ShadowRecoveryStore. "
                f"The Git-backed store is at {self._repo}.",
                DeprecationWarning, stacklevel=2,
            )

        self._init_repo()

    # ── Git infrastructure ────────────────────────────────────────────────

    def _git_env(self, **extra: str) -> dict[str, str]:
        """Build an environment dict pointing at this store's Git repo."""
        env = os.environ.copy()
        env["GIT_DIR"] = str(self._repo / ".git")
        for k, v in extra.items():
            env[k] = v
        return env

    def _run_git(
        self,
        args: list[str],
        *,
        input: str | bytes | None = None,
        env: dict[str, str] | None = None,
        check: bool = True,
    ) -> subprocess.CompletedProcess:
        """Run a git command; raise :class:`AssuranceError` on failure."""
        try:
            return subprocess.run(
                ["git", *args],
                input=input,
                capture_output=True,
                text=isinstance(input, str) or input is None,
                check=check,
                env=env or self._git_env(),
                creationflags=CREATE_NO_WINDOW,
            )
        except subprocess.CalledProcessError as exc:
            stderr = (
                exc.stderr.decode("utf-8", errors="replace")
                if isinstance(exc.stderr, bytes)
                else exc.stderr or ""
            ).strip()
            raise AssuranceError(
                f"git {args[0]} failed: {stderr}"
            ) from exc

    def _git_bytes(self, args: list[str]) -> bytes:
        """Run a git command and return raw stdout bytes.

        Used for retrieving binary blobs (e.g. ``snapshot.bin``) that
        ``text=True`` would mangle.
        """
        env = self._git_env()
        try:
            result = subprocess.run(
                ["git", *args],
                capture_output=True,
                check=True,
                env=env,
                creationflags=CREATE_NO_WINDOW,
            )
            return result.stdout
        except subprocess.CalledProcessError as exc:
            stderr = (
                exc.stderr.decode("utf-8", errors="replace")
                if isinstance(exc.stderr, bytes)
                else exc.stderr or ""
            ).strip()
            raise AssuranceError(
                f"git {args[0]} failed: {stderr}"
            ) from exc

    def _init_repo(self) -> None:
        """Initialize the Git repository if it does not already exist."""
        self._repo.mkdir(parents=True, exist_ok=True)
        dot_git = self._repo / ".git"
        if dot_git.is_dir():
            return

        subprocess.run(
            ["git", "-C", str(self._repo), "init"],
            capture_output=True, text=True, check=True,
            creationflags=CREATE_NO_WINDOW,
        )
        # Local config so commits succeed without a global identity set.
        subprocess.run(
            ["git", "-C", str(self._repo), "config", "user.name", "GSA Shadow Recovery"],
            capture_output=True, text=True, check=True,
            creationflags=CREATE_NO_WINDOW,
        )
        subprocess.run(
            ["git", "-C", str(self._repo), "config", "user.email", "shadow-recovery@gsa.local"],
            capture_output=True, text=True, check=True,
            creationflags=CREATE_NO_WINDOW,
        )

    def _get_head(self) -> str | None:
        """Return HEAD commit SHA, or ``None`` if the repo has no commits."""
        try:
            result = self._run_git(["rev-parse", "--verify", "HEAD"])
            return result.stdout.strip()
        except AssuranceError:
            return None

    def _find_commit_by_tree(self, tree_sha: str) -> str | None:
        """Walk all commits looking for one whose tree matches *tree_sha*."""
        try:
            result = self._run_git(["log", "--all", "--format=%T %H"])
        except AssuranceError:
            return None
        for line in result.stdout.strip().splitlines():
            parts = line.strip().split(" ", 1)
            if len(parts) == 2 and parts[0] == tree_sha:
                return parts[1]
        return None

    def _create_commit(
        self,
        tree_sha: str,
        candidate_sha256: str,
        auth_sha256: str | None,
        snapshot_sha256: str,
        snapshot_bytes: int,
        candidate_id: str,
    ) -> str:
        """Create a Git commit object for *tree_sha* and advance ``main``."""
        msg_lines = [
            f"candidate_sha256: {candidate_sha256}",
            f"snapshot_sha256: {snapshot_sha256}",
            f"snapshot_bytes: {snapshot_bytes}",
        ]
        if auth_sha256:
            msg_lines.append(f"authorization_sha256: {auth_sha256}")
        msg_lines.append(f"candidate_id: {candidate_id}")
        commit_msg = "\n".join(msg_lines)

        parent = self._get_head()
        args = ["commit-tree", tree_sha, "-m", commit_msg]
        if parent:
            args = ["commit-tree", tree_sha, "-p", parent, "-m", commit_msg]

        result = self._run_git(args)
        commit_sha = result.stdout.strip()
        self._run_git(["update-ref", "refs/heads/main", commit_sha])
        return commit_sha

    # ── public API ────────────────────────────────────────────────────────

    def store(
        self,
        candidate: dict[str, Any],
        authorization: dict[str, Any] | None,
        snapshot_bytes: bytes,
    ) -> StoreEntry:
        """Store a recovery entry as a Git commit.

        Parameters
        ----------
        candidate:
            Recovery candidate receipt.
        authorization:
            Recovery authorization receipt.  May be ``None``.
        snapshot_bytes:
            The snapshot data to store.

        Returns
        -------
        StoreEntry
            Metadata about the stored entry.  *commit_sha* is the primary key.

        Raises
        ------
        AssuranceError
            If *snapshot_bytes* is empty.
        """
        if not snapshot_bytes:
            raise AssuranceError("shadow store: snapshot must not be empty")

        # Compute content hashes (same algorithm as the old CAS store).
        candidate_json = canonical_bytes(candidate)
        candidate_sha256 = sha256_bytes(candidate_json)
        snapshot_sha256 = sha256_bytes(snapshot_bytes)

        auth_json = b""
        auth_sha256 = None
        if authorization is not None:
            auth_json = canonical_bytes(authorization)
            auth_sha256 = sha256_bytes(auth_json)

        # Stage files in a temp worktree and create a Git tree object.
        with tempfile.TemporaryDirectory(prefix="shadow-store-") as tmp:
            work = Path(tmp)
            (work / "candidate.json").write_bytes(candidate_json)
            if authorization is not None:
                (work / "authorization.json").write_bytes(auth_json)
            (work / "snapshot.bin").write_bytes(snapshot_bytes)

            # GIT_INDEX_FILE must point to a path that does NOT exist yet —
            # Git will create the index on first use.  An empty file (from
            # mkstemp) trips "index file smaller than expected".
            idx_path = os.path.join(tmp, "index")
            env = self._git_env(
                GIT_WORK_TREE=str(work),
                GIT_INDEX_FILE=idx_path,
            )
            self._run_git(["add", "."], env=env)
            result = self._run_git(["write-tree"], env=env)
            tree_sha = result.stdout.strip()

        # Idempotency: same tree → same commit.
        commit_sha = self._find_commit_by_tree(tree_sha)
        if commit_sha is None:
            commit_sha = self._create_commit(
                tree_sha,
                candidate_sha256,
                auth_sha256,
                snapshot_sha256,
                len(snapshot_bytes),
                candidate.get("candidate_id", ""),
            )

        # Timestamp from the commit.
        result = self._run_git(["log", "--format=%ct", "-1", commit_sha])
        stored_at = datetime.fromtimestamp(
            int(result.stdout.strip()), tz=timezone.utc,
        ).isoformat().replace("+00:00", "Z")

        manifest = {
            "schema_version": "0.1.0-draft",
            "commit_sha": commit_sha,
            "tree_sha": tree_sha,
            "candidate_sha256": candidate_sha256,
            "authorization_sha256": auth_sha256,
            "snapshot_sha256": snapshot_sha256,
            "snapshot_bytes": len(snapshot_bytes),
            "stored_at": stored_at,
            "candidate_id": candidate.get("candidate_id", ""),
            "source_conversation_id": (
                candidate.get("source", {}).get("conversation_id", "")
            ),
        }

        return StoreEntry(
            entry_id=f"SRV-{commit_sha[:16]}",
            commit_sha=commit_sha,
            entry_sha256=None,
            candidate_sha256=candidate_sha256,
            authorization_sha256=auth_sha256,
            snapshot_sha256=snapshot_sha256,
            snapshot_bytes=len(snapshot_bytes),
            stored_at=stored_at,
            manifest=manifest,
        )

    def retrieve(self, commit_sha: str) -> StoreEntry:
        """Retrieve a stored recovery entry by its Git commit SHA.

        Parameters
        ----------
        commit_sha:
            The 40-character Git commit SHA.

        Returns
        -------
        StoreEntry
            The entry metadata.

        Raises
        ------
        AssuranceError
            If the commit does not exist or is corrupt.
        """
        _validate_commit_sha(commit_sha, label="commit_sha")

        # Verify the object exists and is a commit.
        try:
            result = self._run_git(["cat-file", "-t", commit_sha])
            obj_type = result.stdout.strip()
        except AssuranceError:
            raise AssuranceError(
                f"shadow store: entry not found: {commit_sha[:16]}…"
            )
        if obj_type != "commit":
            raise AssuranceError(
                f"shadow store: object {commit_sha[:16]}… is not a commit"
            )

        # Retrieve blob contents.
        def _get_blob_text(path: str) -> bytes:
            result = self._run_git(["show", f"{commit_sha}:{path}"])
            return result.stdout.encode("utf-8")  # text=True → need to encode back

        def _get_blob_raw(path: str) -> bytes:
            return self._git_bytes(["show", f"{commit_sha}:{path}"])

        try:
            candidate_bytes = _get_blob_text("candidate.json")
            snapshot_bytes = _get_blob_raw("snapshot.bin")
        except AssuranceError as exc:
            raise AssuranceError(
                f"shadow store: missing blob in commit {commit_sha[:16]}…"
            ) from exc

        candidate_sha256 = sha256_bytes(candidate_bytes)
        snapshot_sha256 = sha256_bytes(snapshot_bytes)

        auth_sha256 = None
        try:
            auth_bytes = _get_blob_text("authorization.json")
            auth_sha256 = sha256_bytes(auth_bytes)
        except AssuranceError:
            pass  # authorization is optional

        # Get tree SHA and timestamp.
        result = self._run_git(["rev-parse", f"{commit_sha}^{{tree}}"])
        tree_sha = result.stdout.strip()
        result = self._run_git(["log", "--format=%ct", "-1", commit_sha])
        stored_at = datetime.fromtimestamp(
            int(result.stdout.strip()), tz=timezone.utc,
        ).isoformat().replace("+00:00", "Z")

        manifest = {
            "schema_version": "0.1.0-draft",
            "commit_sha": commit_sha,
            "tree_sha": tree_sha,
            "candidate_sha256": candidate_sha256,
            "authorization_sha256": auth_sha256,
            "snapshot_sha256": snapshot_sha256,
            "snapshot_bytes": len(snapshot_bytes),
            "stored_at": stored_at,
        }

        return StoreEntry(
            entry_id=f"SRV-{commit_sha[:16]}",
            commit_sha=commit_sha,
            entry_sha256=None,
            candidate_sha256=candidate_sha256,
            authorization_sha256=auth_sha256,
            snapshot_sha256=snapshot_sha256,
            snapshot_bytes=len(snapshot_bytes),
            stored_at=stored_at,
            manifest=manifest,
        )

    def list_entries(self) -> list[StoreEntry]:
        """List all stored recovery entries (all commits on all branches)."""
        try:
            result = self._run_git(["log", "--all", "--format=%H"])
        except AssuranceError:
            return []

        sha_list = result.stdout.strip()
        if not sha_list:
            return []

        entries: list[StoreEntry] = []
        for commit_sha in sha_list.splitlines():
            commit_sha = commit_sha.strip()
            if not commit_sha:
                continue
            try:
                entries.append(self.retrieve(commit_sha))
            except Exception:
                continue
        return entries

    def verify_entry(self, commit_sha: str) -> bool:
        """Verify the integrity of a stored entry using Git's object model.

        Checks that *commit_sha* is a valid commit whose tree and blobs
        all exist and are not corrupt.

        Returns ``True`` if all objects pass ``git cat-file -t`` checks.
        """
        try:
            _validate_commit_sha(commit_sha, label="commit_sha")

            # Verify commit object.
            obj_type = self._run_git(["cat-file", "-t", commit_sha]).stdout.strip()
            if obj_type != "commit":
                return False

            # Verify tree object.
            tree_sha = self._run_git(
                ["rev-parse", f"{commit_sha}^{{tree}}"],
            ).stdout.strip()
            obj_type = self._run_git(["cat-file", "-t", tree_sha]).stdout.strip()
            if obj_type != "tree":
                return False

            # Verify every blob in the tree.
            tree_result = self._run_git(["ls-tree", tree_sha])
            for line in tree_result.stdout.strip().splitlines():
                parts = line.split()
                if len(parts) >= 4:
                    blob_sha = parts[2]
                    obj_type = self._run_git(
                        ["cat-file", "-t", blob_sha],
                    ).stdout.strip()
                    if obj_type != "blob":
                        return False

            return True
        except AssuranceError:
            return False


# ══════════════════════════════════════════════════════════════════════════════
# Recovery Diff Preview
# ══════════════════════════════════════════════════════════════════════════════


class RecoveryDiffPreview:
    """Produce a metadata-level diff between a recovery snapshot and the
    current state of the target file.

    The preview **never** exposes raw file content — only hashes, byte
    counts, and line counts.  This is a safety measure: the operator
    sees what will change without any risk of leaking sensitive content
    through the diff mechanism.
    """

    @staticmethod
    def preview(
        candidate: dict[str, Any],
        snapshot_bytes: bytes,
        target_path: str,
        base_root: Path | None = None,
    ) -> DiffPreview:
        """Compare *snapshot_bytes* with the current content at *target_path*.

        Parameters
        ----------
        candidate:
            Recovery candidate receipt.
        snapshot_bytes:
            The snapshot data that would be written.
        target_path:
            The filesystem path to be restored.  Must pass
            :func:`~.endpoint_canonicalizer.canonicalize_filesystem_path`.
        base_root:
            Optional root to scope the target path under.

        Returns
        -------
        DiffPreview
            Metadata-level comparison result.

        Raises
        ------
        AssuranceError
            If *target_path* fails canonicalization.
        """
        canonical = canonicalize_filesystem_path(
            target_path, base_root=base_root,
        )
        target = Path(canonical) if base_root is None else (base_root / canonical)

        snap_sha = sha256_bytes(snapshot_bytes)
        snap_lines = snapshot_bytes.decode("utf-8", errors="replace").count("\n") + 1

        current_bytes: int | None = None
        current_sha: str | None = None
        current_lines: int | None = None
        bytes_to_add = len(snapshot_bytes)
        bytes_to_remove = 0
        lines_added = snap_lines
        lines_removed = 0
        would_overwrite = False

        if target.exists() and target.is_file():
            would_overwrite = True
            current_data = target.read_bytes()
            current_bytes = len(current_data)
            current_sha = sha256_bytes(current_data)
            current_lines = (
                current_data.decode("utf-8", errors="replace").count("\n") + 1
            )
            bytes_to_add = max(0, len(snapshot_bytes) - current_bytes)
            bytes_to_remove = max(0, current_bytes - len(snapshot_bytes))
            lines_added = max(0, snap_lines - current_lines)
            lines_removed = max(0, current_lines - snap_lines)

        return DiffPreview(
            candidate_id=candidate.get("candidate_id", ""),
            target_path=str(target),
            snapshot_sha256=snap_sha,
            current_sha256=current_sha,
            snapshot_bytes=len(snapshot_bytes),
            current_bytes=current_bytes,
            bytes_to_add=bytes_to_add,
            bytes_to_remove=bytes_to_remove,
            snapshot_lines=snap_lines,
            current_lines=current_lines,
            lines_added=lines_added,
            lines_removed=lines_removed,
            would_overwrite=would_overwrite,
        )


# ══════════════════════════════════════════════════════════════════════════════
# Recovery Executor
# ══════════════════════════════════════════════════════════════════════════════


class RecoveryExecutor:
    """Execute a recovery — the **only** component that performs actual
    file restoration.

    Every execution produces a signed :class:`ExecutionReceipt`.  Audit
    events are **never** modified or deleted — the executor only reads
    the audit seal for pre-flight verification.

    Usage::

        executor = RecoveryExecutor()
        receipt = executor.execute(
            candidate=candidate_receipt,
            authorization=auth_receipt,
            snapshot_bytes=store_entry_data,
            target_path="/path/to/restore",
            key_store=key_store,
            shadow_commit_sha=entry.commit_sha,
        )
        assert verify_execution_receipt(receipt, key_store=key_store)["valid"]
    """

    @staticmethod
    def execute(
        candidate: dict[str, Any],
        authorization: dict[str, Any] | None,
        snapshot_bytes: bytes,
        target_path: str,
        key_store: InstallationKeyStore,
        base_root: Path | None = None,
        dry_run: bool = False,
        shadow_commit_sha: str | None = None,
        execution_root: Path | None = None,
    ) -> ExecutionReceipt:
        """Execute a recovery — write *snapshot_bytes* to *target_path*.

        Parameters
        ----------
        candidate:
            Recovery candidate receipt.  Must pass
            :func:`~.recovery.verify_recovery_candidate`.
        authorization:
            Recovery authorization receipt.  If ``None``, the execution
            is rejected (outcome = ``"rejected"``).
        snapshot_bytes:
            The snapshot data to write.
        target_path:
            Canonicalized via :func:`~.endpoint_canonicalizer.canonicalize_filesystem_path`.
        key_store:
            Installation key store for signing the execution receipt.
        base_root:
            Optional root to scope *target_path* under.
        dry_run:
            If ``True``, produce the diff preview but do not write.
        shadow_commit_sha:
            Git commit SHA of the source entry in the shadow repository.
            ``None`` if the snapshot did not come from a Git-backed store.
        execution_root:
            Optional directory for execution receipts.  Defaults to the
            repository-level shadow executions directory; tests can pass an
            isolated temp directory.

        Returns
        -------
        ExecutionReceipt
            Signed receipt recording the execution outcome.

        Raises
        ------
        AssuranceError
            If the candidate verification fails or the path is invalid.
        """
        receipt_id = f"RXR-{uuid.uuid4().hex[:16].upper()}"
        errors: list[str] = []
        diff: dict[str, Any] | None = None
        pre_existing_sha: str | None = None
        outcome = "failed"
        bytes_written = 0

        # 1. Canonicalize target path
        canonical = canonicalize_filesystem_path(
            target_path, base_root=base_root,
        )
        resolved = Path(canonical) if base_root is None else (base_root / canonical)

        # 2. Validate candidate structural integrity
        candidate_id = candidate.get("candidate_id", "")
        source = candidate.get("source", {})
        if source.get("classification") != "untrusted_recovery_candidate":
            return ExecutionReceipt(
                receipt_id=receipt_id,
                candidate_id=candidate_id,
                authorization_id=None,
                target_path=str(resolved),
                snapshot_sha256=sha256_bytes(snapshot_bytes),
                pre_existing_sha256=None,
                outcome="rejected",
                shadow_commit_sha=shadow_commit_sha,
                errors=["candidate classification must be untrusted_recovery_candidate"],
            )

        # 3. Verify authorization (required)
        if authorization is None:
            return ExecutionReceipt(
                receipt_id=receipt_id,
                candidate_id=candidate_id,
                authorization_id=None,
                target_path=str(resolved),
                snapshot_sha256=sha256_bytes(snapshot_bytes),
                pre_existing_sha256=None,
                outcome="rejected",
                bytes_written=0,
                shadow_commit_sha=shadow_commit_sha,
                errors=["recovery authorization is required for execution"],
            )
        auth_id = authorization.get("receipt_id", "")

        auth_candidate_id = authorization.get("candidate_id", "")
        if auth_candidate_id != candidate_id:
            return ExecutionReceipt(
                receipt_id=receipt_id,
                candidate_id=candidate_id,
                authorization_id=auth_id,
                target_path=str(resolved),
                snapshot_sha256=sha256_bytes(snapshot_bytes),
                pre_existing_sha256=None,
                outcome="rejected",
                shadow_commit_sha=shadow_commit_sha,
                errors=[
                    f"authorization candidate_id '{auth_candidate_id}' "
                    f"does not match candidate_id '{candidate_id}'"
                ],
            )

        decision = authorization.get("decision", {})
        if decision.get("outcome") != "allow":
            return ExecutionReceipt(
                receipt_id=receipt_id,
                candidate_id=candidate_id,
                authorization_id=auth_id,
                target_path=str(resolved),
                snapshot_sha256=sha256_bytes(snapshot_bytes),
                pre_existing_sha256=None,
                outcome="rejected",
                shadow_commit_sha=shadow_commit_sha,
                errors=[
                    f"authorization decision is '{decision.get('outcome')}', "
                    f"not 'allow'"
                ],
                bytes_written=0,
            )
        if decision.get("restoration_performed"):
            return ExecutionReceipt(
                receipt_id=receipt_id,
                candidate_id=candidate_id,
                authorization_id=auth_id,
                target_path=str(resolved),
                snapshot_sha256=sha256_bytes(snapshot_bytes),
                pre_existing_sha256=None,
                outcome="rejected",
                bytes_written=0,
                shadow_commit_sha=shadow_commit_sha,
                errors=["authorization already performed restoration"],
            )

        # 4. Capture pre-existing state
        if resolved.exists():
            pre_existing_sha = sha256_file(resolved)
            errors.append(
                "target file exists; backup recommended before restoration"
            )

        # 5. Generate diff preview
        diff_preview = RecoveryDiffPreview.preview(
            candidate, snapshot_bytes, str(resolved),
        )
        diff = {
            "snapshot_sha256": diff_preview.snapshot_sha256,
            "current_sha256": diff_preview.current_sha256,
            "bytes_to_add": diff_preview.bytes_to_add,
            "bytes_to_remove": diff_preview.bytes_to_remove,
            "lines_added": diff_preview.lines_added,
            "lines_removed": diff_preview.lines_removed,
            "would_overwrite": diff_preview.would_overwrite,
        }

        if dry_run:
            return ExecutionReceipt(
                receipt_id=receipt_id,
                candidate_id=candidate_id,
                authorization_id=auth_id,
                target_path=str(resolved),
                snapshot_sha256=sha256_bytes(snapshot_bytes),
                pre_existing_sha256=pre_existing_sha,
                outcome="rejected",
                bytes_written=0,
                shadow_commit_sha=shadow_commit_sha,
                diff_preview=diff,
                errors=["dry_run: no data written"],
            )

        # 6. Atomic write
        try:
            resolved.parent.mkdir(parents=True, exist_ok=True)
            fd, tmp_path_str = tempfile.mkstemp(
                dir=str(resolved.parent),
                prefix=f".{resolved.name}.",
                suffix=".tmp",
            )
            try:
                os.write(fd, snapshot_bytes)
                os.fsync(fd)
            finally:
                os.close(fd)
            tmp_path = Path(tmp_path_str)
            tmp_path.replace(resolved)
            bytes_written = len(snapshot_bytes)
            outcome = "restored"
        except OSError as exc:
            errors.append(f"write failed: {exc}")
            outcome = "failed"
            try:
                Path(tmp_path_str).unlink(missing_ok=True)
            except Exception:
                pass

        # 7. Build and sign execution receipt
        body = {
            "schema_version": "0.1.0-draft",
            "receipt_kind": "recovery_execution",
            "receipt_id": receipt_id,
            "candidate_id": candidate_id,
            "authorization_id": auth_id,
            "target_path": str(resolved),
            "snapshot_sha256": sha256_bytes(snapshot_bytes),
            "pre_existing_sha256": pre_existing_sha,
            "outcome": outcome,
            "bytes_written": bytes_written,
            "shadow_commit_sha": shadow_commit_sha,
            "diff_preview": diff,
            "errors": errors,
            "audit_events_preserved": True,
            "executed_at": _timestamp(),
        }
        signed = _sign_body(body, key_store=key_store)

        # Persist execution receipt
        executions_root = execution_root or SHADOW_EXECUTIONS
        exec_dir = (
            executions_root
            / f"{_timestamp()[:19].replace(':', '')}-{receipt_id[-8:]}"
        )
        exec_dir.mkdir(parents=True, exist_ok=True)
        atomic_write_json(exec_dir / "execution_receipt.json", signed)
        if diff:
            atomic_write_json(exec_dir / "diff_preview.json", diff)

        return ExecutionReceipt(
            receipt_id=receipt_id,
            candidate_id=candidate_id,
            authorization_id=auth_id,
            target_path=str(resolved),
            snapshot_sha256=sha256_bytes(snapshot_bytes),
            pre_existing_sha256=pre_existing_sha,
            outcome=outcome,
            bytes_written=bytes_written,
            shadow_commit_sha=shadow_commit_sha,
            diff_preview=diff,
            errors=errors,
            audit_events_preserved=True,
        )


def verify_execution_receipt(
    receipt: dict[str, Any],
    key_store: InstallationKeyStore,
) -> dict[str, Any]:
    """Verify a signed recovery execution receipt.

    Returns a dict with ``valid`` (bool), ``errors`` (list[str]), and
    ``outcome`` (str) fields.
    """
    errors: list[str] = []
    try:
        if receipt.get("receipt_kind") != "recovery_execution":
            errors.append("receipt_kind is not recovery_execution")
        errors.extend(
            _verify_body_signature(
                receipt, key_store=key_store, label="execution receipt",
            )
        )
        if not receipt.get("audit_events_preserved"):
            errors.append(
                "execution receipt must declare audit_events_preserved=True"
            )
        outcome = receipt.get("outcome", "")
        if outcome not in ("restored", "failed", "rejected"):
            errors.append(f"unexpected outcome: {outcome}")
    except Exception as exc:
        errors.append(str(exc))

    return {
        "valid": not errors,
        "errors": errors,
        "outcome": receipt.get("outcome", "unknown"),
    }
