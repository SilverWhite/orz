from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Any, Callable
import uuid

from .deepseek_adapter import DEFAULT_CREDENTIAL_TARGET, call_deepseek_api
from .errors import AssuranceError
from .grok_event_normalizer import _validate_runtime_event
from .utils import atomic_write_json, canonical_bytes, sha256_bytes, utc_now


ADAPTER_ID = "deepseek-runtime-adapter"
ADAPTER_VERSION = "0.1.0"
DEFAULT_MODEL = "deepseek-v4-pro"
DEFAULT_RETRIEVAL_MODE = "off"


@dataclass(frozen=True)
class DeepSeekRunRequest:
    run_root: Path
    run_id: str = "RUN-DEEPSEEK-RUNTIME-001"
    prompt_text: str = "Return exactly: GSA_ALPHA_REAL_CALL_OK"
    credential_target: str = DEFAULT_CREDENTIAL_TARGET
    model_id: str = DEFAULT_MODEL
    max_tokens: int = 64
    timeout_seconds: int = 60
    retrieval_mode: str = DEFAULT_RETRIEVAL_MODE
    retrieval_mode_explicit: bool = False


def _event_hash(event: dict[str, Any]) -> str:
    return sha256_bytes(
        canonical_bytes(
            {key: value for key, value in event.items() if key != "event_sha256"}
        )
    )


def _build_event(
    *,
    run_id: str,
    manifest_sha256: str,
    sequence: int,
    event_type: str,
    previous_event_sha256: str | None,
    payload: dict[str, Any],
    timestamp: str,
) -> dict[str, Any]:
    event = {
        "schema_version": "0.1.0-draft",
        "run_id": run_id,
        "event_id": f"EVT-{run_id[4:]}-{sequence:03d}",
        "sequence": sequence,
        "timestamp": timestamp,
        "event_type": event_type,
        "run_manifest_sha256": manifest_sha256,
        "previous_event_sha256": previous_event_sha256,
        "payload_schema": "deepseek-runtime-normalized-v0.1",
        "payload": payload,
        "payload_sha256": sha256_bytes(canonical_bytes(payload)),
        "redaction": "metadata_only",
        "event_sha256": "0" * 64,
    }
    event["event_sha256"] = _event_hash(event)
    _validate_runtime_event(event)
    return event


def _write_events_jsonl(path: Path, events: list[dict[str, Any]]) -> None:
    encoded = "".join(
        f"{_json_dumps(event)}\n" for event in events
    ).encode("utf-8")
    from .utils import atomic_write_bytes

    atomic_write_bytes(path, encoded)


def _json_dumps(value: dict[str, Any]) -> str:
    import json

    return json.dumps(value, ensure_ascii=False, sort_keys=True, allow_nan=False)


def normalize_deepseek_runtime_receipt(
    receipt: dict[str, Any],
    *,
    created_at: str | None = None,
) -> list[dict[str, Any]]:
    run_id = str(receipt["run_id"])
    timestamp = created_at or str(receipt.get("created_at") or utc_now())
    manifest = {
        "schema_version": "0.1.0-draft",
        "manifest_kind": "deepseek_runtime_adapter_projection",
        "run_id": run_id,
        "adapter_id": receipt["adapter"]["adapter_id"],
        "runtime_owner": receipt["adapter"]["runtime_owner"],
        "mode": receipt["request"]["mode"],
        "retrieval_mode": receipt["retrieval"]["mode"],
        "no_residue_required": receipt["containment"]["no_residue_required"],
    }
    manifest_sha256 = sha256_bytes(canonical_bytes(manifest))
    prompt = receipt["prompt"]
    execution = receipt["execution"]
    terminal_type = "run_finished" if receipt.get("valid") is True else "run_failed"
    terminal_payload = {
        "status": "completed" if receipt.get("valid") is True else "failed",
        "exit_code": 0 if receipt.get("valid") is True else 1,
        "no_residue_observed": receipt["containment"]["no_residue_observed"],
        "external_cleanup_required": receipt["containment"][
            "external_cleanup_required"
        ],
    }
    specs: list[tuple[str, dict[str, Any]]] = [
        (
            "run_preflight",
            {
                "adapter_id": receipt["adapter"]["adapter_id"],
                "provider": "deepseek",
                "model_id": prompt["model_id"],
                "real_network_allowed": True,
                "binary_valid": True,
                "workspace_trust_granted": True,
                "retrieval_mode": receipt["retrieval"]["mode"],
                "retrieval_active_for_current_mode": False,
                "retrieval_mode_explicit": receipt["retrieval"]["selected_explicitly"],
            },
        ),
        (
            "run_started",
            {
                "task_id": f"TASK-{run_id[4:]}",
                "run_root": receipt["request"]["run_root"],
                "mode": receipt["request"]["mode"],
            },
        ),
        (
            "prompt_submitted",
            {
                "prompt_sha256": prompt["prompt_sha256"],
                "prompt_bytes": prompt["prompt_bytes"],
                "model_id": prompt["model_id"],
                "max_turns": 1,
            },
        ),
    ]
    if prompt.get("response_sha256"):
        specs.extend(
            [
                (
                    "model_response_received",
                    {
                        "response_sha256": prompt["response_sha256"],
                        "response_summary_sha256": "",
                        "finish_reason": prompt["response_finish_reason"],
                        "token_count": prompt["response_token_count"],
                        "output_format": "chat_completions",
                    },
                ),
                (
                    "model_output",
                    {
                        "turn_count": 1,
                        "response_sha256": prompt["response_sha256"],
                        "tool_call_count": 0,
                        "stop_reason": prompt["response_finish_reason"],
                        "structured_output_valid": True,
                    },
                ),
            ]
        )
    specs.append(
        (
            "artifact_registered",
            {
                "artifact_path": receipt["artifacts"]["receipt_path"],
                "artifact_kind": "deepseek_runtime_adapter_receipt",
                "metadata_only": True,
            },
        )
    )
    specs.append((terminal_type, terminal_payload))
    events: list[dict[str, Any]] = []
    previous: str | None = None
    for sequence, (event_type, payload) in enumerate(specs):
        event = _build_event(
            run_id=run_id,
            manifest_sha256=manifest_sha256,
            sequence=sequence,
            event_type=event_type,
            previous_event_sha256=previous,
            payload=payload,
            timestamp=timestamp,
        )
        events.append(event)
        previous = event["event_sha256"]
    return events


def run_deepseek_direct_once(
    request: DeepSeekRunRequest,
    *,
    credential_reader: Callable[[str], str] | None = None,
    api_caller: Callable[..., dict[str, Any]] = call_deepseek_api,
) -> dict[str, Any]:
    from .deepseek_adapter import _read_windows_credential

    run_root = request.run_root
    if run_root.exists() and any(run_root.iterdir()):
        raise AssuranceError(f"run_root must be empty or absent: {run_root}")
    run_root.mkdir(parents=True, exist_ok=True)
    created_at = utc_now()
    receipt_path = run_root / "deepseek-runtime-receipt.json"
    events_path = run_root / "events.jsonl"
    reader = credential_reader or _read_windows_credential
    api_key = reader(request.credential_target)
    model_output = api_caller(
        api_key,
        [{"role": "user", "content": request.prompt_text}],
        model=request.model_id,
        max_tokens=request.max_tokens,
        temperature=0.0,
        timeout_seconds=request.timeout_seconds,
    )
    response_text = str(model_output.get("public_assistant_text", ""))
    usage = model_output.get("usage", {})
    if not isinstance(usage, dict):
        usage = {}
    prompt_sha256 = sha256_bytes(request.prompt_text.encode("utf-8"))
    response_sha256 = (
        sha256_bytes(response_text.encode("utf-8")) if response_text else ""
    )
    checks = {
        "credential_read_succeeded": bool(api_key),
        "http_status_ok": model_output.get("http_status_code") == 200,
        "response_received": bool(response_text.strip()),
        "finish_reason_observed": bool(model_output.get("finish_reason")),
        "no_tool_calls_requested": True,
        "retrieval_off": request.retrieval_mode == "off",
        "no_child_process_started": True,
        "no_secret_serialized": True,
    }
    receipt = {
        "schema_version": "0.1.0",
        "receipt_kind": "deepseek_runtime_adapter_receipt",
        "run_id": request.run_id,
        "created_at": created_at,
        "valid": False,
        "adapter": {
            "adapter_id": ADAPTER_ID,
            "adapter_version": ADAPTER_VERSION,
            "runtime_owner": "deepseek",
        },
        "request": {
            "mode": "chat_completions",
            "run_root": str(run_root.resolve()),
            "credential_target": request.credential_target,
            "retrieval_mode": request.retrieval_mode,
            "retrieval_mode_explicit": request.retrieval_mode_explicit,
        },
        "retrieval": {
            "mode": request.retrieval_mode,
            "selected_explicitly": request.retrieval_mode_explicit,
            "active_for_current_mode": False,
            "runtime_tool_retrieval_allowed": False,
            "assurance_receipts_required": False,
            "valid_modes": [DEFAULT_RETRIEVAL_MODE],
        },
        "execution": {
            "command_kind": "deepseek_chat_completions",
            "http_status_code": model_output.get("http_status_code"),
            "timeout_seconds": request.timeout_seconds,
            "network_permit_id": model_output.get("network_permit_id", ""),
        },
        "prompt": {
            "prompt_sha256": prompt_sha256,
            "prompt_bytes": len(request.prompt_text.encode("utf-8")),
            "model_id": request.model_id,
            "max_tokens": request.max_tokens,
            "response_sha256": response_sha256,
            "response_finish_reason": model_output.get("finish_reason", ""),
            "response_token_count": int(usage.get("completion_tokens", 0) or 0),
            "usage": {
                "prompt_tokens": int(usage.get("prompt_tokens", 0) or 0),
                "completion_tokens": int(usage.get("completion_tokens", 0) or 0),
                "total_tokens": int(usage.get("total_tokens", 0) or 0),
            },
            "private_reasoning_content_sha256": model_output.get(
                "private_reasoning_content_sha256"
            ),
            "output_format": "chat_completions",
        },
        "containment": {
            "no_residue_required": True,
            "no_residue_observed": True,
            "root_process_exited": True,
            "external_cleanup_required": False,
            "residue_scan_scope": "no_child_process",
            "containment_provider": "direct_https_no_child_process",
            "containment_available": True,
        },
        "artifacts": {
            "receipt_path": str(receipt_path.resolve()),
            "events_path": str(events_path.resolve()),
        },
        "checks": checks,
        "limitations": [
            "Direct DeepSeek runtime bypasses Grok ACP and proves provider/API availability only.",
            "Raw prompt and response content are hashed in the receipt; private reasoning is digest-only.",
            "No tool calls, ACP permission prompts, or Grok-native UI state are exercised.",
        ],
    }
    checks["no_secret_serialized"] = api_key not in _json_dumps(receipt)
    receipt["valid"] = all(checks.values())
    events = normalize_deepseek_runtime_receipt(receipt, created_at=created_at)
    _write_events_jsonl(events_path, events)
    response_path = run_root / "response.txt"
    from .utils import atomic_write_bytes

    atomic_write_bytes(response_path, response_text.encode("utf-8"))
    receipt["artifacts"]["response_path"] = str(response_path.resolve())
    atomic_write_json(receipt_path, receipt)
    return receipt


def run_deepseek_direct_smoke(
    *,
    run_root: Path,
    prompt_text: str | None = None,
    credential_target: str = DEFAULT_CREDENTIAL_TARGET,
    run_id: str | None = None,
    timeout_seconds: int = 60,
) -> dict[str, Any]:
    chosen_run_id = run_id or f"RUN-DEEPSEEK-DIRECT-{uuid.uuid4().hex[:8].upper()}"
    return run_deepseek_direct_once(
        DeepSeekRunRequest(
            run_root=run_root,
            run_id=chosen_run_id,
            prompt_text=prompt_text or "Return exactly: GSA_ALPHA_REAL_CALL_OK",
            credential_target=credential_target,
            timeout_seconds=timeout_seconds,
        )
    )


def run_deepseek_ask(
    *,
    run_root: Path,
    prompt_text: str,
    credential_target: str = DEFAULT_CREDENTIAL_TARGET,
    run_id: str | None = None,
    max_tokens: int = 4096,
    timeout_seconds: int = 120,
) -> tuple[dict[str, Any], str]:
    chosen_run_id = run_id or f"RUN-GSA-ASK-{uuid.uuid4().hex[:8].upper()}"
    receipt = run_deepseek_direct_once(
        DeepSeekRunRequest(
            run_root=run_root,
            run_id=chosen_run_id,
            prompt_text=prompt_text,
            credential_target=credential_target,
            max_tokens=max_tokens,
            timeout_seconds=timeout_seconds,
        )
    )
    response_path = Path(receipt["artifacts"]["response_path"])
    response_text = response_path.read_text(encoding="utf-8") if response_path.exists() else ""
    return receipt, response_text
