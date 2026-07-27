from __future__ import annotations

from copy import deepcopy
import json
import os
from pathlib import Path
import subprocess
from typing import Any, Callable, Sequence
import uuid

from .contracts import ASSURANCE_ROOT, validate_contract
from .errors import AssuranceError
from .utils import (
    canonical_bytes,
    is_link_or_reparse,
    load_json,
    sha256_bytes,
    sha256_file,
    utc_now,
)


CommandRunner = Callable[[Sequence[str], int], subprocess.CompletedProcess[str]]
DISPOSABLE_MARKER = ".assurance-p2-disposable.json"
PROBE_FILE = ".p2-container-write-probe"

CONTAINER_PROBE = r"""
import json
import os
from pathlib import Path
import socket

checks = {}
checks["non_root"] = os.geteuid() != 0
try:
    Path("/etc/lif-assurance-write-probe").write_text("forbidden", encoding="utf-8")
    checks["rootfs_write_blocked"] = False
except OSError:
    checks["rootfs_write_blocked"] = True
try:
    Path("/workspace/.p2-container-write-probe").write_text("workspace-ok", encoding="utf-8")
    checks["workspace_write_succeeded"] = True
except OSError:
    checks["workspace_write_succeeded"] = False
try:
    Path("/tmp/p2-tmpfs-probe").write_text("tmpfs-ok", encoding="utf-8")
    checks["tmpfs_write_succeeded"] = True
except OSError:
    checks["tmpfs_write_succeeded"] = False
sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
sock.settimeout(0.5)
try:
    sock.connect(("1.1.1.1", 443))
    checks["network_connect_blocked"] = False
except OSError:
    checks["network_connect_blocked"] = True
finally:
    sock.close()
checks["docker_socket_absent"] = not (
    Path("/var/run/docker.sock").exists() or Path("/run/docker.sock").exists()
)
checks["host_home_mount_absent"] = not Path("/host-home").exists()
print(json.dumps(checks, sort_keys=True, separators=(",", ":")))
raise SystemExit(0 if all(checks.values()) else 7)
""".strip()


def _default_runner(
    command: Sequence[str], timeout_seconds: int
) -> subprocess.CompletedProcess[str]:
    try:
        return subprocess.run(
            list(command),
            check=False,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=timeout_seconds,
            shell=False,
        )
    except (OSError, subprocess.TimeoutExpired) as exc:
        raise AssuranceError(
            f"cannot execute fixed Docker probe command {command[0]!r}: "
            f"{type(exc).__name__}"
        ) from exc


def _run_json(
    runner: CommandRunner,
    command: Sequence[str],
    *,
    timeout_seconds: int,
    label: str,
) -> Any:
    completed = runner(command, timeout_seconds)
    if completed.returncode != 0:
        raise AssuranceError(
            f"{label} failed with exit code {completed.returncode}"
        )
    try:
        return json.loads(completed.stdout)
    except json.JSONDecodeError as exc:
        raise AssuranceError(f"{label} did not return valid JSON") from exc


def _validate_disposable_workspace(workspace: Path) -> Path:
    if not workspace.is_dir() or is_link_or_reparse(workspace):
        raise AssuranceError("Docker probe workspace must be a non-linked directory")
    resolved = workspace.resolve(strict=True)
    if "," in str(resolved) or "\n" in str(resolved) or "\r" in str(resolved):
        raise AssuranceError("Docker probe workspace path contains unsafe mount syntax")
    marker_path = resolved / DISPOSABLE_MARKER
    marker = load_json(marker_path)
    if marker != {
        "schema_version": "0.1.0-draft",
        "purpose": "docker-sandbox-probe",
        "allow_container_write_probe": True,
    }:
        raise AssuranceError("Docker probe workspace marker is missing or invalid")
    probe_file = resolved / PROBE_FILE
    if probe_file.exists():
        raise AssuranceError("refusing to overwrite an existing Docker probe file")
    return resolved


def load_docker_profile(path: Path | None = None) -> dict[str, Any]:
    selected = path or ASSURANCE_ROOT / "docker-sandbox-profile-v0.1.json"
    profile = load_json(selected)
    validate_contract(
        profile,
        "docker-sandbox-profile-v0.1.schema.json",
        label="Docker sandbox profile",
    )
    if profile["image"]["digest"] not in profile["image"]["reference"]:
        raise AssuranceError("Docker image reference does not match pinned digest")
    return profile


def run_docker_sandbox_probe(
    workspace: Path,
    *,
    profile_path: Path | None = None,
    runner: CommandRunner | None = None,
) -> dict[str, Any]:
    selected_runner = runner or _default_runner
    profile = load_docker_profile(profile_path)
    resolved_workspace = _validate_disposable_workspace(workspace)
    timeout_seconds = profile["resources"]["wall_time_seconds"]

    version = _run_json(
        selected_runner,
        ["docker", "version", "--format", "{{json .}}"],
        timeout_seconds=timeout_seconds,
        label="Docker version probe",
    )
    docker_info = _run_json(
        selected_runner,
        ["docker", "info", "--format", "{{json .}}"],
        timeout_seconds=timeout_seconds,
        label="Docker security-options probe",
    )
    image_result = _run_json(
        selected_runner,
        ["docker", "image", "inspect", profile["image"]["reference"]],
        timeout_seconds=timeout_seconds,
        label="pinned Docker image inspection",
    )
    if not isinstance(image_result, list) or len(image_result) != 1:
        raise AssuranceError("pinned Docker image inspection returned an invalid shape")
    image = image_result[0]
    if profile["image"]["reference"] not in (image.get("RepoDigests") or []):
        raise AssuranceError("local Docker image does not expose the pinned repo digest")

    container_name = f"lif-assurance-p2-{uuid.uuid4().hex}"
    mount_spec = (
        f"type=bind,source={resolved_workspace},"
        f"target={profile['filesystem']['workspace_target']}"
    )
    tmpfs = profile["filesystem"]["tmpfs"][0]
    tmpfs_spec = f"{tmpfs['target']}:{tmpfs['options']}"
    create_command = [
        "docker",
        "create",
        "--pull",
        "never",
        "--name",
        container_name,
        "--read-only",
        "--network",
        "none",
        "--user",
        profile["identity"]["user"],
        "--cap-drop",
        "ALL",
        "--security-opt",
        "no-new-privileges=true",
        "--pids-limit",
        str(profile["resources"]["pids_limit"]),
        "--memory",
        str(profile["resources"]["memory_bytes"]),
        "--cpus",
        str(profile["resources"]["nano_cpus"] / 1_000_000_000),
        "--tmpfs",
        tmpfs_spec,
        "--mount",
        mount_spec,
        "--workdir",
        profile["filesystem"]["workspace_target"],
        profile["image"]["reference"],
        "python",
        "-c",
        CONTAINER_PROBE,
    ]
    container_id: str | None = None
    inspect: dict[str, Any] | None = None
    probe_checks: dict[str, bool] = {}
    start_return_code = -1
    removed = False
    created_container = False
    probe_path = resolved_workspace / PROBE_FILE
    probe_file_cleaned = False
    try:
        created = selected_runner(create_command, timeout_seconds)
        if created.returncode != 0:
            raise AssuranceError(
                f"Docker create failed with exit code {created.returncode}"
            )
        created_container = True
        container_id = created.stdout.strip()
        if len(container_id) != 64 or any(
            character not in "0123456789abcdef" for character in container_id
        ):
            raise AssuranceError("Docker create returned an invalid container ID")

        started = selected_runner(
            ["docker", "start", "--attach", container_id], timeout_seconds
        )
        start_return_code = started.returncode
        try:
            parsed_checks = json.loads(started.stdout)
        except json.JSONDecodeError as exc:
            raise AssuranceError("container probe did not return valid JSON") from exc
        if not isinstance(parsed_checks, dict):
            raise AssuranceError("container probe checks must be an object")
        probe_checks = {
            key: value for key, value in parsed_checks.items() if isinstance(value, bool)
        }
        inspected = _run_json(
            selected_runner,
            ["docker", "inspect", container_id],
            timeout_seconds=timeout_seconds,
            label="Docker container inspection",
        )
        if not isinstance(inspected, list) or len(inspected) != 1:
            raise AssuranceError("Docker container inspection returned an invalid shape")
        inspect = inspected[0]
    finally:
        if created_container:
            removed_result = selected_runner(
                ["docker", "rm", "--force", container_name], timeout_seconds
            )
            removed = removed_result.returncode == 0
        if probe_path.exists() or probe_path.is_symlink():
            try:
                probe_path.unlink()
                probe_file_cleaned = not probe_path.exists()
            except OSError:
                probe_file_cleaned = False
    probe_checks["probe_file_cleaned"] = probe_file_cleaned
    if inspect is None or container_id is None:
        raise AssuranceError("Docker probe lacks container inspection evidence")

    host_config = inspect["HostConfig"]
    config = inspect["Config"]
    mounts = inspect.get("Mounts", [])
    mount_projection = [
        {
            "type": item.get("Type"),
            "target": item.get("Destination"),
            "read_only": not bool(item.get("RW")),
            "source_path_sha256": sha256_bytes(
                str(resolved_workspace).encode("utf-8")
            ),
        }
        for item in mounts
    ]
    observation: dict[str, Any] = {
        "schema_version": "0.1.0-draft",
        "observation_kind": "docker_strict_sandbox_observation",
        "observation_id": f"DSO-{uuid.uuid4().hex.upper()}",
        "created_at": utc_now(),
        "profile_sha256": sha256_file(
            profile_path or ASSURANCE_ROOT / "docker-sandbox-profile-v0.1.json"
        ),
        "workspace_path_sha256": sha256_bytes(
            str(resolved_workspace).encode("utf-8")
        ),
        "docker": {
            "client_version": version["Client"]["Version"],
            "server_version": version["Server"]["Version"],
            "server_os": version["Server"]["Os"],
            "server_arch": version["Server"]["Arch"],
            "security_options": sorted(docker_info.get("SecurityOptions") or []),
        },
        "image": {
            "reference": profile["image"]["reference"],
            "image_id": image["Id"],
            "repo_digests": sorted(image.get("RepoDigests") or []),
        },
        "container": {
            "container_id": container_id,
            "user": config.get("User", ""),
            "read_only_rootfs": bool(host_config.get("ReadonlyRootfs")),
            "network_mode": host_config.get("NetworkMode"),
            "cap_drop": sorted(host_config.get("CapDrop") or []),
            "cap_add": sorted(host_config.get("CapAdd") or []),
            "security_opt": sorted(host_config.get("SecurityOpt") or []),
            "memory_bytes": int(host_config.get("Memory") or 0),
            "nano_cpus": int(host_config.get("NanoCpus") or 0),
            "pids_limit": int(host_config.get("PidsLimit") or 0),
            "mounts": mount_projection,
            "tmpfs": host_config.get("Tmpfs") or {},
            "exit_code": int(inspect["State"]["ExitCode"]),
            "removed": removed,
        },
        "checks": probe_checks,
        "outcome": "noncompliant",
        "evidence_status": "observed",
        "limitations": [
            "Docker daemon privilege and Docker Desktop/WSL2 boundaries are outside the container profile.",
            "The probe covers one disposable workspace and does not prove every host path or kernel escape impossible."
        ],
    }
    from .sandbox_verifier import verify_docker_observation

    pre_verification = verify_docker_observation(
        observation,
        profile=profile,
        profile_path=profile_path,
        require_compliant=False,
    )
    if pre_verification["controls_compliant"] and start_return_code == 0:
        observation["outcome"] = "compliant"
    validate_contract(
        observation,
        "docker-sandbox-observation-v0.1.schema.json",
        label="Docker sandbox observation",
    )
    final_verification = verify_docker_observation(
        observation,
        profile=profile,
        profile_path=profile_path,
        require_compliant=True,
    )
    if not final_verification["valid"]:
        raise AssuranceError(
            "Docker sandbox observation failed independent verification: "
            + "; ".join(final_verification["errors"])
        )
    return observation


def docker_candidate_from_observation(
    observation: dict[str, Any],
) -> dict[str, Any]:
    validate_contract(
        observation,
        "docker-sandbox-observation-v0.1.schema.json",
        label="Docker sandbox observation",
    )
    compliant = observation["outcome"] == "compliant"
    return {
        "backend_id": "BACKEND-DOCKER-PINNED-001",
        "backend_kind": "docker",
        "availability": "available",
        "compliance_status": "compliant" if compliant else "noncompliant",
        "evidence_status": "observed",
        "receipt_digest": sha256_bytes(canonical_bytes(observation)),
        "rejection_reasons": [] if compliant else ["docker_observation_noncompliant"],
    }


def windows_native_strict_candidate() -> dict[str, Any]:
    """Static candidate without a live observation.

    Always noncompliant: operators must run
    ``run_windows_native_sandbox_probe`` and build a candidate from the
    observation. A static PASS would overstate isolation.
    """
    availability = "available" if os.name == "nt" else "unavailable"
    reasons = (
        ["platform_not_windows"]
        if os.name != "nt"
        else ["windows_native_live_observation_required"]
    )
    projection = {
        "backend_kind": "windows_native_strict",
        "availability": availability,
        "compliance_status": "noncompliant",
        "rejection_reasons": reasons,
    }
    return {
        "backend_id": "BACKEND-WINDOWS-NATIVE-STRICT-001",
        "backend_kind": "windows_native_strict",
        "availability": availability,
        "compliance_status": "noncompliant",
        "evidence_status": "observed",
        "receipt_digest": sha256_bytes(canonical_bytes(projection)),
        "rejection_reasons": reasons,
    }


def build_sandbox_selection_receipt(
    *,
    requested_backend: str,
    candidates: list[dict[str, Any]],
    conversation_id: str,
) -> dict[str, Any]:
    selected: dict[str, Any] | None = None
    if requested_backend == "auto":
        selected = next(
            (
                candidate
                for candidate in candidates
                if candidate["availability"] == "available"
                and candidate["compliance_status"] == "compliant"
                and candidate["evidence_status"] == "observed"
            ),
            None,
        )
    else:
        requested = [
            candidate
            for candidate in candidates
            if candidate["backend_kind"] == requested_backend
        ]
        if len(requested) > 1:
            raise AssuranceError("sandbox candidates contain duplicate backend kinds")
        if requested:
            candidate = requested[0]
            if (
                candidate["availability"] == "available"
                and candidate["compliance_status"] == "compliant"
                and candidate["evidence_status"] == "observed"
            ):
                selected = candidate

    outcome = "allow" if selected is not None else "fail_closed"
    selected_projection = None
    if selected is not None:
        selected_projection = {
            "backend_id": selected["backend_id"],
            "backend_kind": selected["backend_kind"],
            "compliance_status": selected["compliance_status"],
            "evidence_status": selected["evidence_status"],
            "receipt_digest": selected["receipt_digest"],
        }
    receipt = {
        "schema_version": "0.1.0-draft",
        "receipt_kind": "sandbox_selection_receipt",
        "receipt_id": f"SBX-{uuid.uuid4().hex.upper()}",
        "conversation_id": conversation_id,
        "created_at": utc_now(),
        "requested_backend": requested_backend,
        "candidates": deepcopy(candidates),
        "decision": {
            "outcome": outcome,
            "terminal_state": (
                "selected" if selected is not None else "no_compliant_backend"
            ),
            "selected_backend": selected_projection,
            "fallback_used": False,
            "fallback_authorization_digest": None,
            "rationale": (
                "Selected one observed compliant backend."
                if selected is not None
                else "Requested backend has no observed compliant candidate."
            ),
        },
    }
    validate_contract(
        receipt,
        "sandbox-selection-receipt-v0.1.schema.json",
        label="sandbox selection receipt",
    )
    from .sandbox_verifier import verify_sandbox_selection_receipt

    verification = verify_sandbox_selection_receipt(receipt)
    if not verification["valid"]:
        raise AssuranceError(
            "sandbox selection receipt failed verification: "
            + "; ".join(verification["errors"])
        )
    return receipt
