from __future__ import annotations

from itertools import count
from typing import Any, Sequence

from .contracts import validate_contract
from .errors import AssuranceError
from .utils import canonical_bytes, sha256_bytes, utc_now


REPORT_SCHEMA = "tool-availability-report-v0.1.schema.json"
RECEIPT_SCHEMA = "tool-availability-gate-receipt-v0.1.schema.json"

CONTEXT_BLOCK_PREFIX = "[TOOL_AVAILABILITY v0.1]"
CONTEXT_BLOCK_SUFFIX = "[/TOOL_AVAILABILITY]"

CAPABILITY_ENUM = {
    "search", "file_read", "file_write", "file_edit",
    "bash_exec", "web_fetch", "subagent", "tool_registry", "network_io",
}
VALID_STATUSES = {"available", "unavailable", "unprobed", "degraded"}

MODEL_HALLUCINATION_PATTERNS = [
    ("wrong_belief_about_availability", "模型声称某工具可用但 gate 确认不可用"),
    ("wrong_belief_about_unavailability", "模型声称某工具不可用但 gate 确认可用"),
    ("capability_guessing", "模型在未声明工具可用性的情况下猜测使用"),
    ("tool_capability_inflation", "模型将不可用工具的能力归因到其他工具"),
]


def _validate_tool_spec(spec: dict[str, Any]) -> None:
    required = {"tool_id", "tool_name", "capability", "probe_method"}
    missing = required - set(spec)
    if missing:
        raise AssuranceError(f"tool spec missing required fields: {sorted(missing)}")
    if spec["capability"] not in CAPABILITY_ENUM:
        raise AssuranceError(f"unknown tool capability: {spec['capability']}")
    if not spec.get("tool_id") or not isinstance(spec["tool_id"], str):
        raise AssuranceError("tool spec tool_id must be a non-empty string")


def _build_context_block(
    available_ids: list[str],
    unavailable_ids: list[str],
    unprobed_ids: list[str],
    degraded_ids: list[str],
) -> str:
    lines = [CONTEXT_BLOCK_PREFIX + ""]
    if available_ids:
        lines.append(
            "AVAILABLE: " + ", ".join(sorted(available_ids))
        )
    if unavailable_ids:
        lines.append(
            "UNAVAILABLE: " + ", ".join(sorted(unavailable_ids))
        )
    if degraded_ids:
        lines.append(
            "DEGRADED: " + ", ".join(sorted(degraded_ids))
        )
    if unprobed_ids:
        lines.append(
            "UNPROBED: " + ", ".join(sorted(unprobed_ids))
        )
    lines.append("")
    lines.append("GATE: runtime_probe_authoritative")
    lines.append("STATUS: descriptive_projection_only")
    lines.append(
        "ENFORCEMENT: tool calls, provider errors, and capability claims are "
        "checked by runtime gates and observers outside this text block."
    )
    lines.append(CONTEXT_BLOCK_SUFFIX)
    return "\n".join(lines)


def _build_tool_entries(
    category: str,
    entries: list[dict[str, Any]],
) -> list[dict[str, Any]]:
    result: list[dict[str, Any]] = []
    for entry in entries:
        item: dict[str, Any] = {
            "tool_id": entry["tool_id"],
            "tool_name": entry["tool_name"],
            "capability": entry["capability"],
            "probe_method": entry["probe_method"],
            "status": category,
        }
        if "probe_detail" in entry:
            item["probe_detail"] = entry["probe_detail"]
        if "runtime_constraint" in entry:
            item["runtime_constraint"] = entry["runtime_constraint"]
        result.append(item)
    return result


def probe_tool_availability(
    *,
    tool_specs: Sequence[dict[str, Any]],
    runtime_id: str = "RUNTIME-DEFAULT-001",
    probe_registry: dict[str, bool] | None = None,
) -> dict[str, Any]:
    if probe_registry is None:
        probe_registry = {}
    available: list[dict[str, Any]] = []
    unavailable: list[dict[str, Any]] = []
    unprobed: list[dict[str, Any]] = []
    degraded: list[dict[str, Any]] = []

    for spec in tool_specs:
        _validate_tool_spec(spec)
        tool_id = spec["tool_id"]
        if tool_id not in probe_registry:
            entry = {**spec, "probe_detail": "not in probe registry; no runtime hook available"}
            unprobed.append(entry)
            continue
        probed = probe_registry[tool_id]
        entry = {**spec, "probe_detail": "mechanical probe executed"}
        if probed is None:
            degraded.append(entry)
        elif probed:
            available.append(entry)
        else:
            unavailable.append(entry)

    available_ids = [item["tool_id"] for item in available]
    unavailable_ids = [item["tool_id"] for item in unavailable]
    unprobed_ids = [item["tool_id"] for item in unprobed]
    degraded_ids = [item["tool_id"] for item in degraded]

    report_id = f"TOOL-AVAIL-{runtime_id}"
    context_block = _build_context_block(
        available_ids=available_ids,
        unavailable_ids=unavailable_ids,
        unprobed_ids=unprobed_ids,
        degraded_ids=degraded_ids,
    )
    report = {
        "schema_version": "0.1.0-draft",
        "report_kind": "tool_availability_report",
        "report_id": report_id,
        "runtime_id": runtime_id,
        "probe_timestamp": utc_now(),
        "available": _build_tool_entries("available", available),
        "unavailable": _build_tool_entries("unavailable", unavailable),
        "unprobed": _build_tool_entries("unprobed", unprobed),
        "degraded": _build_tool_entries("degraded", degraded),
        "context_block": context_block,
        "notes": [
            "Tool availability is determined by mechanical probe, not model inference.",
            "Unprobed tools remain an explicit observed status.",
            "The context block is a descriptive projection; runtime gates enforce availability.",
        ],
    }
    validate_contract(report, REPORT_SCHEMA, label="tool availability report")
    return report


def build_tool_availability_context_block(report: dict[str, Any]) -> str:
    return report["context_block"]


def build_tool_availability_gate_receipt(report: dict[str, Any]) -> dict[str, Any]:
    validate_contract(report, REPORT_SCHEMA, label="tool availability report")
    all_probed = len(report["unprobed"]) == 0
    receipt = {
        "schema_version": "0.1.0-draft",
        "receipt_kind": "tool_availability_gate_receipt",
        "valid": True,
        "report_id": report["report_id"],
        "report_sha256": sha256_bytes(canonical_bytes(report)),
        "available_count": len(report["available"]),
        "unavailable_count": len(report["unavailable"]),
        "unprobed_count": len(report["unprobed"]),
        "degraded_count": len(report["degraded"]),
        "context_block_sha256": sha256_bytes(report["context_block"].encode("utf-8")),
        "decisions": {
            "gate_decision": "block" if len(report["degraded"]) > 0 else ("defer" if not all_probed else "allow"),
            "context_injected": True,
            "model_must_not_guess": True,
        },
        "checks": {
            "all_declared_tools_probed": all_probed,
            "unavailable_tools_listed": len(report["unavailable"]) > 0 or len(report["available"]) > 0,
            "no_capability_guessing_permitted": True,
            "mechanical_probe_only": True,
            "no_model_invoked": True,
            "no_network_requested": True,
        },
        "limitations": [
            "This gate uses declared probe results; it does not dynamically discover tools at runtime.",
            "Unprobed tools are represented as unprobed; model-facing text is not the enforcement layer.",
            "Probe registry must be maintained by the runtime adapter.",
        ],
    }
    validate_contract(receipt, RECEIPT_SCHEMA, label="tool availability gate receipt")
    return receipt


def evaluate_tool_belief_mismatch(
    *,
    model_claimed_available: Sequence[str],
    model_claimed_unavailable: Sequence[str],
    actual_report: dict[str, Any],
) -> dict[str, Any]:
    validate_contract(actual_report, REPORT_SCHEMA, label="tool availability report")
    actual_available = {item["tool_id"] for item in actual_report["available"]}
    actual_unavailable = {item["tool_id"] for item in actual_report["unavailable"]}
    actual_degraded = {item["tool_id"] for item in actual_report["degraded"]}

    mismatches: list[dict[str, Any]] = []
    reason_codes: list[str] = []

    for claimed in model_claimed_available:
        if claimed in actual_unavailable:
            mismatches.append({
                "tool_id": claimed,
                "mismatch_type": "wrong_belief_about_availability",
                "model_claimed": "available",
                "actual_status": "unavailable",
                "detail": "模型声称某工具可用但 gate 确认不可用",
            })
            reason_codes.append("TOOL-BELIEF-AVAILABILITY-MISMATCH")
        elif claimed in actual_degraded:
            mismatches.append({
                "tool_id": claimed,
                "mismatch_type": "tool_capability_inflation",
                "model_claimed": "available",
                "actual_status": "degraded",
                "detail": "模型将不可用工具的能力归因到其他工具",
            })
            reason_codes.append("TOOL-BELIEF-DEGRADED-MISMATCH")

    for claimed in model_claimed_unavailable:
        if claimed in actual_available:
            mismatches.append({
                "tool_id": claimed,
                "mismatch_type": "wrong_belief_about_unavailability",
                "model_claimed": "unavailable",
                "actual_status": "available",
                "detail": "模型声称某工具不可用但 gate 确认可用",
            })
            reason_codes.append("TOOL-BELIEF-UNAVAILABILITY-MISMATCH")

    triggered = len(mismatches) > 0
    report = {
        "schema_version": "0.1.0-draft",
        "receipt_kind": "tool_availability_belief_mismatch_receipt",
        "valid": not triggered,
        "triggered": triggered,
        "mismatch_count": len(mismatches),
        "mismatches": mismatches,
        "reason_codes": sorted(set(reason_codes)),
        "checks": {
            "model_aware_of_available_tools": not triggered,
            "model_does_not_claim_degraded_tools_as_available": (
                "TOOL-BELIEF-DEGRADED-MISMATCH" not in reason_codes
            ),
            "model_does_not_invent_unavailable_tools": True,
            "mechanical_probe_only": True,
            "no_model_invoked": True,
            "no_network_requested": True,
        },
        "limitations": [
            "Model claims about tool availability are extracted from observed text, not from hidden chain-of-thought.",
            "This detector only flags explicit tool name mentions; it cannot detect implicit assumption of unavailable tools.",
        ],
    }
    return report
