from __future__ import annotations

from pathlib import Path
from typing import Any

from .contracts import ASSURANCE_ROOT, validate_contract
from .utils import sha256_file


def verify_windows_native_observation(
    observation: dict[str, Any],
    *,
    profile: dict[str, Any],
    profile_path: Path | None = None,
    require_compliant: bool,
) -> dict[str, Any]:
    errors: list[str] = []
    try:
        validate_contract(
            profile,
            "windows-native-sandbox-profile-v0.1.schema.json",
            label="Windows native sandbox profile",
        )
        validate_contract(
            observation,
            "windows-native-sandbox-observation-v0.1.schema.json",
            label="Windows native sandbox observation",
        )
    except Exception as exc:
        return {
            "valid": False,
            "controls_compliant": False,
            "errors": [str(exc)],
        }
    expected_profile_sha = sha256_file(
        profile_path
        or ASSURANCE_ROOT / "windows-native-sandbox-profile-v0.1.json"
    )
    if observation["profile_sha256"] != expected_profile_sha:
        errors.append("Windows native profile digest mismatch")
    ac = observation["appcontainer"]
    if not ac["sid_derived"] and not ac["profile_created"]:
        errors.append("Windows native sandbox must derive or create AppContainer SID")
    if ac["profile_created"] and not ac["profile_deleted"]:
        errors.append("AppContainer profile was created but not deleted")
    if ac["capabilities"]:
        errors.append("AppContainer capabilities must be empty for network isolation")
    jo = observation["job_object"]
    if not jo["created"]:
        errors.append("Windows native sandbox requires Job Object creation")
    if jo["assigned"]:
        if jo["memory_limit_bytes"] != profile["resources"]["memory_bytes"]:
            errors.append("Job Object memory limit mismatch")
    else:
        errors.append("Job Object must be assigned to process for strict sandbox")
    if not jo["kill_on_close"]:
        errors.append("Job Object must be kill-on-close")
    proc = observation["process"]
    if proc["shell_used"]:
        errors.append("Windows native sandbox must not use shell")
    required_checks = {
        "non_admin",
        "system32_write_blocked",
        "workspace_write_succeeded",
        "temp_write_succeeded",
        "registry_protected_blocked",
        "probe_file_cleaned",
    }
    if not set(observation["checks"].keys()) >= required_checks:
        errors.append("Windows native observation missing required checks")
    check_failures: list[str] = []
    if set(observation["checks"].keys()) >= required_checks:
        if not all(
            v for k, v in observation["checks"].items()
            if k in required_checks and k != "probe_file_cleaned"
        ):
            check_failures.append(
                "one or more Windows native isolation checks failed"
            )
        elif not observation["checks"].get("probe_file_cleaned"):
            check_failures.append("probe file was not cleaned up")
    # Network isolation is provided by host firewall rule, not AppContainer
    fw = observation.get("firewall", {})
    if not fw.get("outbound_block_rule_created", False):
        check_failures.append(
            "host firewall outbound block rule was not created; "
            "raw TCP may leak through empty AppContainer capabilities"
        )
    controls_compliant = not errors and not check_failures
    # Structural integrity can be valid while isolation controls remain noncompliant.
    # Failed checks only make valid=false when compliance is required or claimed.
    report_errors = list(errors)
    if require_compliant or observation["outcome"] == "compliant":
        report_errors.extend(check_failures)
    if require_compliant and observation["outcome"] != "compliant":
        report_errors.append(
            "Windows native observation did not reach compliant outcome"
        )
    if observation["outcome"] == "compliant" and not controls_compliant:
        report_errors.append("Windows native observation overstates compliance")
    if not (require_compliant or observation["outcome"] == "compliant"):
        # Surface isolation failures for operators without failing structural validity.
        report_errors.extend(check_failures)
        return {
            "valid": not errors,
            "controls_compliant": controls_compliant,
            "errors": report_errors,
        }
    return {
        "valid": not report_errors,
        "controls_compliant": controls_compliant,
        "errors": report_errors,
    }


def verify_docker_observation(
    observation: dict[str, Any],
    *,
    profile: dict[str, Any],
    profile_path: Path | None = None,
    require_compliant: bool,
) -> dict[str, Any]:
    errors: list[str] = []
    try:
        validate_contract(
            profile,
            "docker-sandbox-profile-v0.1.schema.json",
            label="Docker sandbox profile",
        )
        validate_contract(
            observation,
            "docker-sandbox-observation-v0.1.schema.json",
            label="Docker sandbox observation",
        )
    except Exception as exc:
        return {
            "valid": False,
            "controls_compliant": False,
            "errors": [str(exc)],
        }
    expected_profile_sha = sha256_file(
        profile_path or ASSURANCE_ROOT / "docker-sandbox-profile-v0.1.json"
    )
    if observation["profile_sha256"] != expected_profile_sha:
        errors.append("Docker profile digest mismatch")
    if observation["image"]["reference"] != profile["image"]["reference"]:
        errors.append("Docker image reference mismatch")
    if profile["image"]["reference"] not in observation["image"]["repo_digests"]:
        errors.append("Docker pinned repo digest mismatch")
    if observation["docker"]["server_os"] != "linux":
        errors.append("Docker server must be Linux")
    if not any(
        "seccomp" in option.lower()
        for option in observation["docker"]["security_options"]
    ):
        errors.append("Docker server did not report seccomp")
    container = observation["container"]
    if container["user"] != profile["identity"]["user"]:
        errors.append("container user mismatch")
    if not container["read_only_rootfs"]:
        errors.append("container root filesystem is writable")
    if container["network_mode"] != "none":
        errors.append("container network is not disabled")
    if container["cap_drop"] != ["ALL"] or container["cap_add"]:
        errors.append("container capabilities do not match deny-all profile")
    if not any(
        option.startswith("no-new-privileges")
        for option in container["security_opt"]
    ):
        errors.append("container lacks no-new-privileges")
    if container["memory_bytes"] != profile["resources"]["memory_bytes"]:
        errors.append("container memory limit mismatch")
    if container["nano_cpus"] != profile["resources"]["nano_cpus"]:
        errors.append("container CPU limit mismatch")
    if container["pids_limit"] != profile["resources"]["pids_limit"]:
        errors.append("container PID limit mismatch")
    if len(container["mounts"]) != 1:
        errors.append("container must have exactly one host mount")
    elif (
        container["mounts"][0]["target"] != "/workspace"
        or container["mounts"][0]["read_only"]
        or container["mounts"][0]["source_path_sha256"]
        != observation["workspace_path_sha256"]
    ):
        errors.append("container workspace mount projection mismatch")
    expected_tmpfs = {
        item["target"]: item["options"] for item in profile["filesystem"]["tmpfs"]
    }
    if container["tmpfs"] != expected_tmpfs:
        errors.append("container tmpfs configuration mismatch")
    if container["exit_code"] != 0:
        errors.append("container probe exited nonzero")
    if not container["removed"]:
        errors.append("container cleanup did not complete")
    required_checks = {
        "non_root",
        "rootfs_write_blocked",
        "workspace_write_succeeded",
        "tmpfs_write_succeeded",
        "network_connect_blocked",
        "docker_socket_absent",
        "host_home_mount_absent",
        "probe_file_cleaned",
    }
    if set(observation["checks"]) != required_checks:
        errors.append("Docker observation check coverage mismatch")
    elif not all(observation["checks"].values()):
        errors.append("one or more Docker negative checks failed")
    controls_compliant = not errors
    if require_compliant and observation["outcome"] != "compliant":
        errors.append("Docker observation did not reach compliant outcome")
    if observation["outcome"] == "compliant" and not controls_compliant:
        errors.append("Docker observation overstates compliance")
    return {
        "valid": not errors,
        "controls_compliant": controls_compliant,
        "errors": errors,
    }


def verify_sandbox_selection_receipt(
    receipt: dict[str, Any],
) -> dict[str, Any]:
    errors: list[str] = []
    try:
        validate_contract(
            receipt,
            "sandbox-selection-receipt-v0.1.schema.json",
            label="sandbox selection receipt",
        )
    except Exception as exc:
        return {"valid": False, "errors": [str(exc)]}
    candidates = receipt["candidates"]
    kinds = [candidate["backend_kind"] for candidate in candidates]
    if len(kinds) != len(set(kinds)):
        errors.append("sandbox candidate backend kinds are not unique")
    decision = receipt["decision"]
    selected = decision["selected_backend"]
    requested = receipt["requested_backend"]
    if decision["fallback_used"] or decision["fallback_authorization_digest"] is not None:
        errors.append("sandbox selection must not silently fall back")
    if decision["outcome"] == "allow":
        matches = [
            candidate
            for candidate in candidates
            if candidate["backend_id"] == selected["backend_id"]
        ]
        if len(matches) != 1:
            errors.append("selected sandbox backend is not a unique candidate")
        else:
            candidate = matches[0]
            projection = {
                "backend_id": candidate["backend_id"],
                "backend_kind": candidate["backend_kind"],
                "compliance_status": candidate["compliance_status"],
                "evidence_status": candidate["evidence_status"],
                "receipt_digest": candidate["receipt_digest"],
            }
            if selected != projection:
                errors.append("selected sandbox projection does not match candidate")
            if (
                candidate["availability"] != "available"
                or candidate["compliance_status"] != "compliant"
                or candidate["evidence_status"] != "observed"
            ):
                errors.append("selected sandbox candidate is not observed compliant")
            if requested != "auto" and candidate["backend_kind"] != requested:
                errors.append("selected sandbox does not match requested backend")
    else:
        if selected is not None:
            errors.append("fail-closed sandbox receipt contains a selection")
        if requested != "auto":
            requested_candidates = [
                candidate
                for candidate in candidates
                if candidate["backend_kind"] == requested
            ]
            if any(
                candidate["availability"] == "available"
                and candidate["compliance_status"] == "compliant"
                and candidate["evidence_status"] == "observed"
                for candidate in requested_candidates
            ):
                errors.append("fail-closed receipt ignored an observed compliant request")
        elif any(
            candidate["availability"] == "available"
            and candidate["compliance_status"] == "compliant"
            and candidate["evidence_status"] == "observed"
            for candidate in candidates
        ):
            errors.append("auto selection ignored an observed compliant backend")
    return {"valid": not errors, "errors": errors}


def verify_windows_native_run_observation(
    observation: dict[str, Any],
    *,
    profile: dict[str, Any],
    profile_path: Path | None = None,
    require_compliant: bool,
) -> dict[str, Any]:
    """Independent verifier for windows-native-sandbox-run observations.

    Cross-checks the spawn-time facts against the profile and the per-arm
    required checks.  Refuses overstated (compliant with failing checks) and
    understated (all checks true but noncompliant) outcomes.
    """
    errors: list[str] = []
    try:
        validate_contract(
            profile,
            "windows-native-sandbox-profile-v0.1.schema.json",
            label="Windows native sandbox profile",
        )
        validate_contract(
            observation,
            "windows-native-sandbox-run-v0.1.schema.json",
            label="Windows native sandbox run observation",
        )
    except Exception as exc:
        return {
            "valid": False,
            "controls_compliant": False,
            "errors": [str(exc)],
        }

    expected_profile_sha = sha256_file(
        profile_path
        or ASSURANCE_ROOT / "windows-native-sandbox-profile-v0.1.json"
    )
    if observation["profile_sha256"] != expected_profile_sha:
        errors.append("Windows native run profile digest mismatch")

    arm = observation["arm"]
    if arm not in ("control", "non-admin", "high-nist"):
        errors.append(f"unknown arm {arm!r}")

    from .windows_sandbox import run_observation_checks_for_arm

    required_checks = set(run_observation_checks_for_arm(arm))
    appcontainer_mode = observation.get("appcontainer_mode", "enabled")
    if arm == "high-nist" and appcontainer_mode == "disabled":
        required_checks.discard("appcontainer_token")
    observed_keys = set(observation["checks"].keys())
    if not observed_keys >= required_checks:
        missing = sorted(required_checks - observed_keys)
        errors.append(f"run observation missing required checks: {missing}")
    all_required_true = observed_keys >= required_checks and all(
        observation["checks"].get(name) is True for name in required_checks
    )

    if arm != "control":
        token = observation["token"]
        if require_compliant:
            if not token["restricted"]:
                errors.append("compliant run requires a restricted token spawn")
            if token["virtualization_allowed"]:
                errors.append("compliant run requires token virtualization disabled")
            if not token["privileges_removed"]:
                errors.append("compliant run requires restricted privileges")
        if arm == "high-nist" and require_compliant:
            if not token["low_integrity"]:
                errors.append("compliant high-nist run requires LOW integrity")
            if appcontainer_mode == "enabled":
                if not token["appcontainer"]:
                    errors.append(
                        "compliant high-nist run requires AppContainer token"
                    )
            elif token["appcontainer"]:
                errors.append(
                    "high-nist appcontainer_mode=disabled must not carry an "
                    "AppContainer token"
                )
    elif require_compliant and observation["token"]["restricted"]:
        errors.append("control arm must not spawn with a restricted token")

    ac = observation["appcontainer"]
    if arm == "high-nist":
        if appcontainer_mode == "enabled":
            if not ac["sid_derived"] and not ac["profile_created"]:
                errors.append("high-nist run must derive or create AppContainer SID")
            if ac["profile_created"] and not ac["profile_deleted"]:
                errors.append("AppContainer profile was created but not deleted")
        elif ac["sid_derived"] or ac["profile_created"]:
            errors.append(
                "high-nist appcontainer_mode=disabled must not derive or create "
                "an AppContainer SID/profile"
            )
    if ac["capabilities"]:
        errors.append("AppContainer capabilities must be empty for network isolation")

    jo = observation["job_object"]
    if not jo["created"]:
        errors.append("run environment requires Job Object creation")
    if not jo["assigned"]:
        errors.append("Job Object must be assigned to the spawned process")
    if not jo["kill_on_close"]:
        errors.append("Job Object must be kill-on-close")
    if jo["memory_limit_bytes"] != profile["resources"]["memory_bytes"]:
        errors.append("Job Object memory limit mismatch")
    if jo["active_process_limit"] != profile["resources"]["pids_limit"]:
        errors.append("Job Object active-process limit mismatch")

    firewall = observation["firewall"]
    if arm == "high-nist":
        if not firewall["outbound_block_rule_created"]:
            errors.append(
                "high-nist run requires an egress wall "
                "(block rule, or per-task allowlist rules that were "
                "actually created)"
            )

    proc = observation["process"]
    if proc["shell_used"]:
        errors.append("run environment must not use shell")

    if require_compliant and not all_required_true:
        errors.append("run observation overstates compliance (checks not all true)")
    if not require_compliant and all_required_true:
        errors.append("run observation understates compliance (checks all true)")

    return {
        "valid": not errors,
        "controls_compliant": all_required_true,
        "errors": errors,
    }
