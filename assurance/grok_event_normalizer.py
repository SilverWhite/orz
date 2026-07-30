from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator, FormatChecker

from .errors import AssuranceError
from .utils import atomic_write_bytes, canonical_bytes, sha256_bytes, utc_now


ROOT = Path(__file__).resolve().parents[1]
RUNTIME_SCHEMA = ROOT / "runtime" / "run-event-v0.1.schema.json"
SUPPORTED_EVENT_TYPES = {
    "run_preflight",
    "run_started",
    "artifact_registered",
    "run_finished",
    "run_failed",
    "prompt_submitted",
    "model_response_received",
    "acp_initialize",
    "acp_session_created",
    "tool_proposal",
    "permission_requested",
    "permission_decision",
}


def _validate_runtime_event(event: dict[str, Any]) -> None:
    schema = json.loads(RUNTIME_SCHEMA.read_text(encoding="utf-8"))
    errors = sorted(
        Draft202012Validator(schema, format_checker=FormatChecker()).iter_errors(event),
        key=lambda item: list(item.absolute_path),
    )
    if errors:
        rendered = [
            f"runtime event#/{'/'.join(map(str, item.absolute_path))}: {item.message}"
            for item in errors
        ]
        raise AssuranceError("; ".join(rendered))


def _event_hash(event: dict[str, Any]) -> str:
    return sha256_bytes(
        canonical_bytes({key: value for key, value in event.items() if key != "event_sha256"})
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
    redaction: str = "metadata_only",
) -> dict[str, Any]:
    if event_type not in SUPPORTED_EVENT_TYPES:
        raise AssuranceError(f"unsupported Grok normalized event type: {event_type}")
    event = {
        "schema_version": "0.1.0-draft",
        "run_id": run_id,
        "event_id": f"EVT-{run_id[4:]}-{sequence:03d}",
        "sequence": sequence,
        "timestamp": timestamp,
        "event_type": event_type,
        "run_manifest_sha256": manifest_sha256,
        "previous_event_sha256": previous_event_sha256,
        "payload_schema": "grok-runtime-normalized-v0.1",
        "payload": payload,
        "payload_sha256": sha256_bytes(canonical_bytes(payload)),
        "redaction": redaction,
        "event_sha256": "0" * 64,
    }
    event["event_sha256"] = _event_hash(event)
    _validate_runtime_event(event)
    return event


def normalize_grok_runtime_receipt(
    receipt: dict[str, Any],
    *,
    created_at: str | None = None,
) -> list[dict[str, Any]]:
    """Project a Grok adapter receipt into canonical runtime events.

    The first slice deliberately uses only existing runtime event types.
    It records metadata, artifact paths, terminal status, and the no-residue
    boundary without exposing raw prompts, hidden reasoning, auth headers, or
    session-private data.
    """
    run_id = str(receipt["run_id"])
    timestamp = created_at or str(receipt.get("created_at") or utc_now())
    is_prompt = receipt["request"]["mode"] == "prompt_smoke"
    is_acp = receipt["request"]["mode"] == "acp_smoke"
    manifest = {
        "schema_version": "0.1.0-draft",
        "manifest_kind": "grok_runtime_adapter_projection",
        "run_id": run_id,
        "adapter_id": receipt["adapter"]["adapter_id"],
        "runtime_owner": receipt["adapter"]["runtime_owner"],
        "mode": receipt["request"]["mode"],
        "retrieval_mode": receipt["retrieval"]["mode"],
        "retrieval_active_for_current_mode": receipt["retrieval"]["active_for_current_mode"],
        "binary_sha256": receipt["binary"]["sha256"],
        "workspace_trust_sha256": receipt["workspace_trust"]["aggregate_sha256"],
        "no_residue_required": receipt["containment"]["no_residue_required"],
    }
    manifest_sha256 = sha256_bytes(canonical_bytes(manifest))
    terminal_type = "run_finished" if receipt.get("valid") is True else "run_failed"
    terminal_payload = {
        "status": "completed" if receipt.get("valid") is True else "failed",
        "exit_code": receipt["execution"]["exit_code"],
        "no_residue_observed": receipt["containment"]["no_residue_observed"],
        "external_cleanup_required": receipt["containment"]["external_cleanup_required"],
    }
    specs: list[tuple[str, dict[str, Any]]] = [
        (
            "run_preflight",
            {
                "adapter_id": receipt["adapter"]["adapter_id"],
                "provider": "grok",
                "model_id": "grok-runtime",
                "real_network_allowed": False,
                "binary_valid": receipt["binary"]["valid"],
                "workspace_trust_granted": receipt["workspace_trust"]["trust_granted"],
                "retrieval_mode": receipt["retrieval"]["mode"],
                "retrieval_active_for_current_mode": receipt["retrieval"]["active_for_current_mode"],
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
    ]
    # Prompt-specific events: prompt_submitted + model_response_received.
    if is_prompt and receipt.get("prompt") is not None:
        prompt_block = receipt["prompt"]
        specs.append((
            "prompt_submitted",
            {
                "prompt_sha256": prompt_block["prompt_sha256"],
                "prompt_bytes": prompt_block["prompt_bytes"],
                "model_id": prompt_block["model_id"],
                "max_turns": prompt_block["max_turns"],
            },
        ))
        specs.append((
            "model_response_received",
            {
                "response_sha256": prompt_block["response_sha256"],
                "response_summary_sha256": sha256_bytes(
                    prompt_block["response_summary"].encode("utf-8")
                ) if prompt_block["response_summary"] else "",
                "finish_reason": prompt_block["response_finish_reason"],
                "token_count": prompt_block["response_token_count"],
                "output_format": prompt_block["output_format"],
            },
        ))
    # ACP-specific events: session lifecycle + tool/permission observability.
    if is_acp and receipt.get("acp") is not None:
        acp_block = receipt["acp"]
        specs.append((
            "acp_initialize",
            {
                "protocol_version": acp_block["protocol_version"],
            },
        ))
        specs.append((
            "acp_session_created",
            {
                "session_id_hash": acp_block["session_id_hash"],
            },
        ))
        if acp_block.get("tool_call_count", 0) > 0:
            specs.append((
                "tool_proposal",
                {
                    "tool_call_count": acp_block["tool_call_count"],
                },
            ))
        if acp_block.get("permission_requests_count", 0) > 0:
            specs.append((
                "permission_requested",
                {
                    "permission_requests_count": acp_block["permission_requests_count"],
                    "permission_outcomes": acp_block["permission_outcomes"],
                },
            ))
            specs.append((
                "permission_decision",
                {
                    "permission_requests_count": acp_block["permission_requests_count"],
                    "permission_outcomes": acp_block["permission_outcomes"],
                },
            ))
    specs.append(
        ("artifact_registered",
         {
             "artifact_path": receipt["artifacts"]["receipt_path"],
             "artifact_kind": "grok_runtime_adapter_receipt",
             "metadata_only": True,
         }),
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


def write_grok_events_jsonl(path: Path, events: list[dict[str, Any]]) -> None:
    content = b"".join(
        json.dumps(
            event,
            ensure_ascii=False,
            sort_keys=True,
            separators=(",", ":"),
            allow_nan=False,
        ).encode("utf-8")
        + b"\n"
        for event in events
    )
    atomic_write_bytes(path, content)


def project_grok_events_for_tui(events: list[dict[str, Any]]) -> list[dict[str, Any]]:
    """Return a TUI-ready metadata projection without importing the TUI layer."""
    return [
        {
            "kind": event["event_type"],
            "timestamp": event["timestamp"],
            "payload": event["payload"],
            "redaction": event["redaction"],
        }
        for event in events
    ]
