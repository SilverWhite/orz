from __future__ import annotations

from dataclasses import dataclass, field
from datetime import datetime, timezone
import json
from pathlib import Path
import subprocess
import time
from typing import Any, Protocol, Sequence
import uuid

from .contracts import ASSURANCE_ROOT, validate_contract
from .conversation import ConversationNamespace
from .child_capability_enforcer import (
    ChildCapabilityEscalationError,
    enforce_child_capabilities,
)
from .envelope import verify_security_envelope
from .errors import AssuranceError
from .keystore import InstallationKeyStore
from .sandbox import _validate_disposable_workspace, load_docker_profile
from .sandbox_verifier import (
    verify_docker_observation,
    verify_sandbox_selection_receipt,
)
from .utils import (
    canonical_bytes,
    is_link_or_reparse,
    sha256_bytes,
    sha256_file,
    utc_now,
)


ACTION_CAPABILITY = "action.fixed_no_model"
ACTION_KIND = "fixed_workspace_roundtrip"
PROBE_FILE = ".p2-5-guarded-execution-probe"
PROBE_PAYLOAD = b"guarded-execution-ok\n"
CONTAINER_SCRIPT = r"""
import hashlib
import json
from pathlib import Path
import time

payload = b"guarded-execution-ok\n"
target = Path("/workspace/.p2-5-guarded-execution-probe")
target.write_bytes(payload)
time.sleep(3)
print(json.dumps({
    "action": "fixed_workspace_roundtrip",
    "model_invoked": False,
    "network_requested": False,
    "probe_sha256": hashlib.sha256(payload).hexdigest(),
}, sort_keys=True, separators=(",", ":")))
""".strip()

SUCCESS_ROLES = [
    "create",
    "start",
    "inspect_running",
    "top",
    "wait",
    "logs",
    "inspect_terminal",
    "remove",
    "inspect_absent",
]


class ProcessTracker(Protocol):
    records: list[dict[str, Any]]

    def run(
        self,
        role: str,
        command: Sequence[str],
        timeout_seconds: int,
        *,
        allow_nonzero: bool = False,
    ) -> subprocess.CompletedProcess[str]:
        ...


@dataclass
class DockerProcessTracker:
    records: list[dict[str, Any]] = field(default_factory=list)

    def run(
        self,
        role: str,
        command: Sequence[str],
        timeout_seconds: int,
        *,
        allow_nonzero: bool = False,
    ) -> subprocess.CompletedProcess[str]:
        selected = list(command)
        started_at = utc_now()
        started = time.perf_counter()
        try:
            process = subprocess.Popen(
                selected,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                encoding="utf-8",
                errors="replace",
                shell=False,
            )
        except OSError as exc:
            raise AssuranceError(
                f"cannot start tracked Docker command for {role}: {type(exc).__name__}"
            ) from exc
        timed_out = False
        try:
            stdout, stderr = process.communicate(timeout=timeout_seconds)
        except subprocess.TimeoutExpired:
            timed_out = True
            process.kill()
            stdout, stderr = process.communicate()
        completed_at = utc_now()
        duration_ms = (time.perf_counter() - started) * 1000
        stdout_bytes = stdout.encode("utf-8")
        stderr_bytes = stderr.encode("utf-8")
        self.records.append(
            {
                "sequence": len(self.records),
                "role": role,
                "executable": Path(selected[0]).stem.lower(),
                "argument_count": len(selected) - 1,
                "arguments_sha256": sha256_bytes(
                    canonical_bytes(selected[1:])
                ),
                "pid": process.pid,
                "started_at": started_at,
                "completed_at": completed_at,
                "duration_ms": duration_ms,
                "exit_code": process.returncode,
                "timed_out": timed_out,
                "stdout_bytes": len(stdout_bytes),
                "stdout_sha256": sha256_bytes(stdout_bytes),
                "stderr_bytes": len(stderr_bytes),
                "stderr_sha256": sha256_bytes(stderr_bytes),
            }
        )
        completed = subprocess.CompletedProcess(
            selected, process.returncode, stdout, stderr
        )
        if timed_out:
            raise AssuranceError(f"tracked Docker command timed out: {role}")
        if completed.returncode != 0 and not allow_nonzero:
            raise AssuranceError(
                f"tracked Docker command failed: {role} "
                f"(exit {completed.returncode})"
            )
        return completed


def workspace_content_sha256(workspace: Path) -> str:
    resolved = workspace.resolve(strict=True)
    entries: list[dict[str, Any]] = []
    for path in sorted(resolved.rglob("*"), key=lambda item: item.as_posix()):
        if is_link_or_reparse(path):
            raise AssuranceError(
                f"guarded workspace contains linked or reparse-backed entry: {path}"
            )
        if path.is_dir():
            continue
        if not path.is_file():
            raise AssuranceError(f"guarded workspace contains non-file entry: {path}")
        entries.append(
            {
                "path": path.relative_to(resolved).as_posix(),
                "sha256": sha256_file(path),
                "bytes": path.stat().st_size,
            }
        )
    return sha256_bytes(canonical_bytes(entries))


def guarded_runtime_capabilities_sha256() -> str:
    return sha256_bytes(
        canonical_bytes(
            {
                "action": ACTION_KIND,
                "model_invoked": False,
                "network_requested": False,
                "shell": False,
                "workspace_mount_count": 1,
            }
        )
    )


def build_guarded_frozen_context(
    *,
    workspace: Path,
    observation: dict[str, Any],
    profile_path: Path | None = None,
) -> dict[str, Any]:
    profile_file = profile_path or ASSURANCE_ROOT / "docker-sandbox-profile-v0.1.json"
    profile = load_docker_profile(profile_file)
    resolved = _validate_disposable_workspace(workspace)
    verification = verify_docker_observation(
        observation,
        profile=profile,
        profile_path=profile_file,
        require_compliant=True,
    )
    if not verification["valid"]:
        raise AssuranceError(
            "cannot bind a noncompliant Docker observation: "
            + "; ".join(verification["errors"])
        )

    def observed(value: str, source: str) -> dict[str, Any]:
        return {
            "value": value,
            "evidence_status": "observed",
            "source_refs": [source],
        }

    def derived(value: str, source: str) -> dict[str, Any]:
        return {
            "value": value,
            "evidence_status": "derived",
            "source_refs": [source],
        }

    return {
        "workspace_canonical_path_digest": derived(
            sha256_bytes(str(resolved).encode("utf-8")),
            "assurance.guarded_execution:canonical-workspace",
        ),
        "workspace_content_digest": derived(
            workspace_content_sha256(resolved),
            "assurance.guarded_execution:workspace-manifest",
        ),
        "workspace_policy_digest": derived(
            sha256_file(resolved / ".assurance-p2-disposable.json"),
            "assurance.guarded_execution:disposable-marker",
        ),
        "runtime_family": observed(
            "runtime-neutral-no-model",
            "assurance.guarded_execution:fixed-action",
        ),
        "runtime_adapter_id": observed(
            "assurance-guarded-executor",
            "assurance.guarded_execution:fixed-action",
        ),
        "runtime_binary_digest": observed(
            observation["image"]["image_id"].removeprefix("sha256:"),
            f"docker-observation:{observation['observation_id']}",
        ),
        "runtime_capabilities_digest": derived(
            guarded_runtime_capabilities_sha256(),
            "assurance.guarded_execution:capability-projection",
        ),
        "assurance_config_digest": derived(
            sha256_file(profile_file),
            "assurance.guarded_execution:docker-profile",
        ),
        "sandbox_backend": observed(
            "docker",
            f"docker-observation:{observation['observation_id']}",
        ),
        "sandbox_backend_digest": derived(
            sha256_bytes(canonical_bytes(observation)),
            f"docker-observation:{observation['observation_id']}",
        ),
    }


def _container_name(execution_id: str) -> str:
    return f"lif-assurance-{execution_id.lower()}"


def _create_command(
    *,
    execution_id: str,
    workspace: Path,
    profile: dict[str, Any],
) -> list[str]:
    command = [
        "docker",
        "create",
        "--pull",
        "never",
        "--name",
        _container_name(execution_id),
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
    ]
    for tmpfs in profile["filesystem"]["tmpfs"]:
        command.extend(
            ["--tmpfs", f"{tmpfs['target']}:{tmpfs['options']}"]
        )
    command.extend(
        [
            "--mount",
            (
                f"type=bind,source={workspace},"
                f"target={profile['filesystem']['workspace_target']}"
            ),
            "--workdir",
            profile["filesystem"]["workspace_target"],
            "--label",
            f"local.fep.guarded-execution={execution_id}",
            profile["image"]["reference"],
            "python",
            "-I",
            "-c",
            CONTAINER_SCRIPT,
        ]
    )
    return command


def _commands(
    *,
    execution_id: str,
    workspace: Path,
    profile: dict[str, Any],
    container_id: str,
) -> list[tuple[str, list[str]]]:
    return [
        (
            "create",
            _create_command(
                execution_id=execution_id,
                workspace=workspace,
                profile=profile,
            ),
        ),
        ("start", ["docker", "start", container_id]),
        ("inspect_running", ["docker", "inspect", container_id]),
        ("top", ["docker", "top", container_id, "-eo", "pid,ppid,comm"]),
        ("wait", ["docker", "wait", container_id]),
        ("logs", ["docker", "logs", container_id]),
        ("inspect_terminal", ["docker", "inspect", container_id]),
        ("remove", ["docker", "rm", "--force", container_id]),
        ("inspect_absent", ["docker", "inspect", container_id]),
    ]


def _json_list(stdout: str, *, label: str) -> dict[str, Any]:
    try:
        value = json.loads(stdout)
    except json.JSONDecodeError as exc:
        raise AssuranceError(f"{label} did not return valid JSON") from exc
    if not isinstance(value, list) or len(value) != 1 or not isinstance(value[0], dict):
        raise AssuranceError(f"{label} returned an invalid shape")
    return value[0]


def _parse_top(stdout: str) -> list[dict[str, Any]]:
    lines = [line.strip() for line in stdout.splitlines() if line.strip()]
    if len(lines) < 2:
        raise AssuranceError("Docker top did not report a running process")
    rows: list[dict[str, Any]] = []
    for line in lines[1:]:
        parts = line.split(maxsplit=2)
        if len(parts) != 3:
            raise AssuranceError("Docker top row has an invalid shape")
        try:
            pid = int(parts[0])
            ppid = int(parts[1])
        except ValueError as exc:
            raise AssuranceError("Docker top row has a non-integer PID") from exc
        rows.append({"pid": pid, "ppid": ppid, "command": parts[2]})
    return rows


def _parse_timestamp(value: str) -> datetime:
    try:
        return datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError as exc:
        raise AssuranceError(f"invalid receipt timestamp: {value}") from exc


def verify_guarded_execution_receipt(
    receipt: dict[str, Any],
    *,
    namespace: ConversationNamespace,
    key_store: InstallationKeyStore,
    workspace: Path,
    selection_receipt: dict[str, Any],
    observation: dict[str, Any],
    profile_path: Path | None = None,
) -> dict[str, Any]:
    errors: list[str] = []
    profile_file = profile_path or ASSURANCE_ROOT / "docker-sandbox-profile-v0.1.json"
    try:
        validate_contract(
            receipt,
            "guarded-execution-receipt-v0.1.schema.json",
            label="guarded execution receipt",
        )
        profile = load_docker_profile(profile_file)
        resolved = _validate_disposable_workspace(workspace)
        envelope = namespace.load_active_envelope()
        envelope_verification = verify_security_envelope(
            envelope, key_store=key_store
        )
    except Exception as exc:
        return {"valid": False, "errors": [str(exc)]}

    integrity = receipt["integrity"]
    signed_body = {
        key: value for key, value in receipt.items() if key != "integrity"
    }
    signed_payload = canonical_bytes(signed_body)
    if integrity["key_id"] != key_store.key_id:
        errors.append("guarded execution integrity key mismatch")
    if integrity["signed_payload_sha256"] != sha256_bytes(signed_payload):
        errors.append("guarded execution signed payload digest mismatch")
    if not key_store.verify(signed_payload, integrity["signature"]):
        errors.append("guarded execution signature verification failed")

    selection_verification = verify_sandbox_selection_receipt(selection_receipt)
    if not selection_verification["valid"]:
        errors.extend(
            f"sandbox selection: {item}"
            for item in selection_verification["errors"]
        )
    observation_verification = verify_docker_observation(
        observation,
        profile=profile,
        profile_path=profile_file,
        require_compliant=True,
    )
    if not observation_verification["valid"]:
        errors.extend(
            f"Docker observation: {item}"
            for item in observation_verification["errors"]
        )

    if receipt["conversation_id"] != namespace.conversation_id:
        errors.append("receipt conversation does not match namespace")
    if receipt["envelope_id"] != envelope["envelope_id"]:
        errors.append("receipt envelope does not match active envelope")
    if selection_receipt["conversation_id"] != namespace.conversation_id:
        errors.append("sandbox selection conversation mismatch")
    selected = selection_receipt["decision"]["selected_backend"]
    if (
        selection_receipt["decision"]["outcome"] != "allow"
        or selected is None
        or selected["backend_kind"] != "docker"
    ):
        errors.append("guarded execution requires a selected Docker backend")

    expected_observation_sha = sha256_bytes(canonical_bytes(observation))
    if selected is not None and selected["receipt_digest"] != expected_observation_sha:
        errors.append("selected backend is not bound to the Docker observation")
    bindings = receipt["bindings"]
    expected_bindings = {
        "envelope_signed_payload_sha256": envelope_verification[
            "signed_payload_sha256"
        ],
        "workspace_path_sha256": sha256_bytes(
            str(resolved).encode("utf-8")
        ),
        "workspace_content_sha256": workspace_content_sha256(resolved),
        "workspace_policy_sha256": sha256_file(
            resolved / ".assurance-p2-disposable.json"
        ),
        "sandbox_selection_receipt_sha256": sha256_bytes(
            canonical_bytes(selection_receipt)
        ),
        "docker_observation_sha256": expected_observation_sha,
        "docker_profile_sha256": sha256_file(profile_file),
    }
    # Only compare child-capability fields if the receipt actually carries them
    for _cap_key in (
        "child_capability_enforced",
        "child_capability_enforcement_receipt_id",
    ):
        if _cap_key in bindings:
            expected_bindings[_cap_key] = bindings[_cap_key]
    if bindings != expected_bindings:
        errors.append("guarded execution binding projection mismatch")

    context = envelope["frozen_context"]
    expected_context = build_guarded_frozen_context(
        workspace=resolved,
        observation=observation,
        profile_path=profile_file,
    )
    for key, expected in expected_context.items():
        if context.get(key, {}).get("value") != expected["value"]:
            errors.append(f"security envelope frozen context mismatch: {key}")
    if ACTION_CAPABILITY not in envelope["capability_envelope"]["allowed"]:
        errors.append("security envelope lacks fixed no-model action capability")
    if envelope["lifecycle_state"] != "active":
        errors.append("security envelope is not active")
    completed_at = _parse_timestamp(receipt["completed_at"])
    if completed_at > _parse_timestamp(envelope["expires_at"]):
        errors.append("guarded execution completed after envelope expiry")

    expected_commands = _commands(
        execution_id=receipt["execution_id"],
        workspace=resolved,
        profile=profile,
        container_id=receipt["container"]["container_id"],
    )
    trace = receipt["process_trace"]
    if [item["role"] for item in trace] != SUCCESS_ROLES:
        errors.append("tracked Docker command role sequence mismatch")
    for index, ((role, command), record) in enumerate(
        zip(expected_commands, trace, strict=False)
    ):
        if record["sequence"] != index or record["role"] != role:
            errors.append(f"tracked Docker sequence mismatch at {index}")
            continue
        if record["arguments_sha256"] != sha256_bytes(
            canonical_bytes(command[1:])
        ):
            errors.append(f"tracked Docker arguments mismatch: {role}")
        if record["argument_count"] != len(command) - 1:
            errors.append(f"tracked Docker argument count mismatch: {role}")
        expected_exit = (
            record["exit_code"] != 0
            if role == "inspect_absent"
            else record["exit_code"] == 0
        )
        if not expected_exit:
            errors.append(f"tracked Docker exit semantics mismatch: {role}")
    if len(trace) != len(expected_commands):
        errors.append("tracked Docker command count mismatch")

    container = receipt["container"]
    if not any(
        row["pid"] == container["running_host_pid"]
        for row in container["process_rows"]
    ):
        errors.append("container running PID is absent from Docker top snapshot")
    if not any("python" in row["command"].lower() for row in container["process_rows"]):
        errors.append("Docker top snapshot lacks the fixed Python action")
    if receipt["result"]["probe_sha256"] != sha256_bytes(PROBE_PAYLOAD):
        errors.append("guarded workspace probe digest mismatch")
    if receipt["result"]["probe_bytes"] != len(PROBE_PAYLOAD):
        errors.append("guarded workspace probe length mismatch")
    expected_report = {
        "action": ACTION_KIND,
        "model_invoked": False,
        "network_requested": False,
        "probe_sha256": sha256_bytes(PROBE_PAYLOAD),
    }
    if receipt["result"]["container_report_sha256"] != sha256_bytes(
        canonical_bytes(expected_report)
    ):
        errors.append("guarded container report digest mismatch")
    if receipt["action"]["template_sha256"] != sha256_bytes(
        CONTAINER_SCRIPT.encode("utf-8")
    ):
        errors.append("guarded action template digest mismatch")
    return {"valid": not errors, "errors": errors}


def execute_guarded_no_model_action(
    *,
    namespace: ConversationNamespace,
    key_store: InstallationKeyStore,
    workspace: Path,
    selection_receipt: dict[str, Any],
    observation: dict[str, Any],
    profile_path: Path | None = None,
    tracker: ProcessTracker | None = None,
) -> dict[str, Any]:
    profile_file = profile_path or ASSURANCE_ROOT / "docker-sandbox-profile-v0.1.json"
    profile = load_docker_profile(profile_file)
    resolved = _validate_disposable_workspace(workspace)
    envelope = namespace.load_active_envelope()
    envelope_verification = verify_security_envelope(envelope, key_store=key_store)
    if selection_receipt["conversation_id"] != namespace.conversation_id:
        raise AssuranceError("sandbox selection conversation mismatch")
    selection_verification = verify_sandbox_selection_receipt(selection_receipt)
    if not selection_verification["valid"]:
        raise AssuranceError(
            "sandbox selection verification failed: "
            + "; ".join(selection_verification["errors"])
        )
    selected = selection_receipt["decision"]["selected_backend"]
    if (
        selection_receipt["decision"]["outcome"] != "allow"
        or selected is None
        or selected["backend_kind"] != "docker"
    ):
        raise AssuranceError("guarded execution requires an allowed Docker selection")
    observation_verification = verify_docker_observation(
        observation,
        profile=profile,
        profile_path=profile_file,
        require_compliant=True,
    )
    if not observation_verification["valid"]:
        raise AssuranceError(
            "Docker observation verification failed: "
            + "; ".join(observation_verification["errors"])
        )
    observation_sha = sha256_bytes(canonical_bytes(observation))
    if selected["receipt_digest"] != observation_sha:
        raise AssuranceError("selected Docker backend does not match observation")
    expected_context = build_guarded_frozen_context(
        workspace=resolved,
        observation=observation,
        profile_path=profile_file,
    )
    for key, expected in expected_context.items():
        if envelope["frozen_context"].get(key, {}).get("value") != expected["value"]:
            raise AssuranceError(f"security envelope context mismatch: {key}")
    if ACTION_CAPABILITY not in envelope["capability_envelope"]["allowed"]:
        raise AssuranceError("security envelope lacks fixed no-model capability")
    if envelope["lifecycle_state"] != "active":
        raise AssuranceError("security envelope is not active")
    if datetime.now(timezone.utc) >= _parse_timestamp(envelope["expires_at"]):
        raise AssuranceError("security envelope is expired")

    selected_tracker = tracker or DockerProcessTracker()
    if selected_tracker.records:
        raise AssuranceError("guarded execution requires an empty process tracker")
    execution_id = f"GEX-{uuid.uuid4().hex.upper()}"
    # ── child capability enforcement before Docker spawn ──
    child_enforcement_receipt = enforce_child_capabilities(
        parent_envelope=envelope,
        child_kind="child_process",
        child_id=execution_id,
        requested_capabilities=[ACTION_CAPABILITY, "filesystem.workspace_read"],
    )
    container_name = _container_name(execution_id)
    probe_path = resolved / PROBE_FILE
    if probe_path.exists() or probe_path.is_symlink():
        raise AssuranceError("refusing to overwrite guarded execution probe")
    created_at = utc_now()
    container_id: str | None = None
    container_created = False
    container_absent = False
    probe_removed = False
    running_pid = 0
    process_rows: list[dict[str, Any]] = []
    container_report: dict[str, Any] | None = None
    terminal_exit_code = -1
    probe_bytes = b""
    try:
        created = selected_tracker.run(
            "create",
            _create_command(
                execution_id=execution_id,
                workspace=resolved,
                profile=profile,
            ),
            profile["resources"]["wall_time_seconds"],
        )
        container_created = True
        container_id = created.stdout.strip()
        if len(container_id) != 64 or any(
            character not in "0123456789abcdef" for character in container_id
        ):
            raise AssuranceError("Docker create returned an invalid container ID")

        selected_tracker.run(
            "start",
            ["docker", "start", container_id],
            profile["resources"]["wall_time_seconds"],
        )
        running = _json_list(
            selected_tracker.run(
                "inspect_running",
                ["docker", "inspect", container_id],
                profile["resources"]["wall_time_seconds"],
            ).stdout,
            label="running container inspection",
        )
        if not running["State"]["Running"]:
            raise AssuranceError("guarded container was not observed running")
        running_pid = int(running["State"]["Pid"])
        if running_pid < 1:
            raise AssuranceError("guarded container has no running host PID")
        process_rows = _parse_top(
            selected_tracker.run(
                "top",
                ["docker", "top", container_id, "-eo", "pid,ppid,comm"],
                profile["resources"]["wall_time_seconds"],
            ).stdout
        )
        waited = selected_tracker.run(
            "wait",
            ["docker", "wait", container_id],
            profile["resources"]["wall_time_seconds"],
        )
        try:
            wait_exit_code = int(waited.stdout.strip())
        except ValueError as exc:
            raise AssuranceError("Docker wait returned an invalid exit code") from exc
        logs = selected_tracker.run(
            "logs",
            ["docker", "logs", container_id],
            profile["resources"]["wall_time_seconds"],
        )
        try:
            container_report = json.loads(logs.stdout)
        except json.JSONDecodeError as exc:
            raise AssuranceError("guarded container log is not valid JSON") from exc
        terminal = _json_list(
            selected_tracker.run(
                "inspect_terminal",
                ["docker", "inspect", container_id],
                profile["resources"]["wall_time_seconds"],
            ).stdout,
            label="terminal container inspection",
        )
        terminal_exit_code = int(terminal["State"]["ExitCode"])
        if terminal["State"]["Running"] or terminal_exit_code != wait_exit_code:
            raise AssuranceError("guarded container terminal state is inconsistent")
        if terminal_exit_code != 0:
            raise AssuranceError("guarded container action failed")
        if is_link_or_reparse(probe_path) or not probe_path.is_file():
            raise AssuranceError("guarded workspace probe is not a regular file")
        probe_bytes = probe_path.read_bytes()
        if probe_bytes != PROBE_PAYLOAD:
            raise AssuranceError("guarded workspace probe content mismatch")
        expected_report = {
            "action": ACTION_KIND,
            "model_invoked": False,
            "network_requested": False,
            "probe_sha256": sha256_bytes(PROBE_PAYLOAD),
        }
        if container_report != expected_report:
            raise AssuranceError("guarded container report mismatch")

        selected_tracker.run(
            "remove",
            ["docker", "rm", "--force", container_id],
            profile["resources"]["wall_time_seconds"],
        )
        absent = selected_tracker.run(
            "inspect_absent",
            ["docker", "inspect", container_id],
            profile["resources"]["wall_time_seconds"],
            allow_nonzero=True,
        )
        container_absent = absent.returncode != 0
        if not container_absent:
            raise AssuranceError("guarded container remains after removal")
    finally:
        cleanup_diagnostics: list[str] = []
        if container_created and not container_absent:
            try:
                selected_tracker.run(
                    "cleanup_remove",
                    ["docker", "rm", "--force", container_name],
                    profile["resources"]["wall_time_seconds"],
                    allow_nonzero=True,
                )
            except Exception as exc:
                cleanup_diagnostics.append(
                    f"docker rm --force {container_name} failed: {exc}"
                )
        if probe_path.exists() or probe_path.is_symlink():
            try:
                probe_path.unlink()
            except OSError as exc:
                cleanup_diagnostics.append(
                    f"probe file unlink {probe_path} failed: {exc}"
                )
        probe_removed = not probe_path.exists() and not probe_path.is_symlink()

    completed_at = utc_now()
    body: dict[str, Any] = {
        "schema_version": "0.1.0-draft",
        "receipt_kind": "guarded_no_model_docker_execution",
        "execution_id": execution_id,
        "conversation_id": namespace.conversation_id,
        "envelope_id": envelope["envelope_id"],
        "created_at": created_at,
        "completed_at": completed_at,
        "action": {
            "kind": ACTION_KIND,
            "required_capability": ACTION_CAPABILITY,
            "template_sha256": sha256_bytes(CONTAINER_SCRIPT.encode("utf-8")),
            "model_invoked": False,
            "network_requested": False,
        },
        "bindings": {
            "envelope_signed_payload_sha256": envelope_verification[
                "signed_payload_sha256"
            ],
            "workspace_path_sha256": sha256_bytes(
                str(resolved).encode("utf-8")
            ),
            "workspace_content_sha256": workspace_content_sha256(resolved),
            "workspace_policy_sha256": sha256_file(
                resolved / ".assurance-p2-disposable.json"
            ),
            "sandbox_selection_receipt_sha256": sha256_bytes(
                canonical_bytes(selection_receipt)
            ),
            "docker_observation_sha256": observation_sha,
            "docker_profile_sha256": sha256_file(profile_file),
            "child_capability_enforced": child_enforcement_receipt["capability_enforced"],
            "child_capability_enforcement_receipt_id": child_enforcement_receipt["receipt_id"],
        },
        "process_trace": selected_tracker.records,
        "container": {
            "container_id": container_id,
            "running_host_pid": running_pid,
            "running_observed": True,
            "process_rows": process_rows,
            "terminal_exit_code": terminal_exit_code,
            "state_sequence": [
                "created",
                "running",
                "exited",
                "removed",
                "absent",
            ],
        },
        "result": {
            "probe_sha256": sha256_bytes(probe_bytes),
            "probe_bytes": len(probe_bytes),
            "container_report_sha256": sha256_bytes(
                canonical_bytes(container_report)
            ),
        },
        "cleanup": {
            "container_removed": container_absent,
            "container_absent": container_absent,
            "workspace_probe_removed": probe_removed,
            "diagnostics": cleanup_diagnostics,
        },
        "outcome": "completed_verified",
        "evidence_status": "observed",
        "limitations": [
            "This receipt covers one fixed no-model action in one disposable workspace.",
            "Docker daemon, Docker Desktop/WSL2, host-kernel and storage-remanence boundaries are outside this receipt.",
        ],
    }
    signed_payload = canonical_bytes(body)
    receipt: dict[str, Any] = {
        **body,
        "integrity": {
            "canonicalization": "RFC8785",
            "key_id": key_store.key_id,
            "signature_algorithm": "hmac-sha256",
            "signed_payload_sha256": sha256_bytes(signed_payload),
            "signature": key_store.sign(signed_payload),
        },
    }
    verification = verify_guarded_execution_receipt(
        receipt,
        namespace=namespace,
        key_store=key_store,
        workspace=resolved,
        selection_receipt=selection_receipt,
        observation=observation,
        profile_path=profile_file,
    )
    if not verification["valid"]:
        raise AssuranceError(
            "guarded execution receipt failed verification: "
            + "; ".join(verification["errors"])
        )
    validate_contract(
        receipt,
        "guarded-execution-receipt-v0.1.schema.json",
        label="guarded execution receipt",
    )
    namespace.write_artifact(
        "temporary_lifecycle_receipt",
        f"{execution_id}.json",
        canonical_bytes(receipt),
    )
    return receipt
