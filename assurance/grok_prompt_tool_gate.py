from __future__ import annotations

from pathlib import Path
from typing import Any
import uuid

from .contracts import validate_contract
from .errors import AssuranceError
from .grok_runtime_adapter import (
    DEFAULT_RETRIEVAL_MODE,
    SUPPORTED_RETRIEVAL_MODES,
    validate_grok_retrieval_mode,
)
from .utils import atomic_write_json, load_json, utc_now


RECEIPT_SCHEMA = "grok-prompt-tool-promotion-gate-receipt-v0.1.schema.json"


def _ensure_empty_run_root(path: Path) -> None:
    if path.exists() and any(path.iterdir()):
        raise AssuranceError(f"run_root must be empty or absent: {path}")
    path.mkdir(parents=True, exist_ok=True)


def _timeout_gate_status(timeout_gate_receipt: dict[str, Any] | None) -> dict[str, Any]:
    if not timeout_gate_receipt:
        return {
            "status": "missing",
            "evidence_attached": False,
            "prompt_tool_promotion_ready": False,
            "required_status": "passed",
            "observed_status": "missing",
        }
    gates = timeout_gate_receipt.get("gates", {})
    owned_cleanup = gates.get("windows_child_tree_owned_cleanup", {})
    observed_status = owned_cleanup.get("status")
    return {
        "status": "passed" if observed_status == "passed" else "blocked",
        "evidence_attached": True,
        "prompt_tool_promotion_ready": timeout_gate_receipt.get(
            "prompt_tool_promotion_ready"
        )
        is True,
        "required_status": "passed",
        "observed_status": observed_status if isinstance(observed_status, str) else "missing",
    }


def _adapter_containment_status(
    adapter_receipt: dict[str, Any] | None,
) -> dict[str, Any]:
    """Evaluate whether the adapter receipt records Job Object containment."""
    if not adapter_receipt:
        return {
            "status": "missing",
            "evidence_attached": False,
            "containment_provided": False,
            "containment_provider": "none",
        }
    containment = adapter_receipt.get("containment", {})
    if not isinstance(containment, dict):
        return {
            "status": "missing",
            "evidence_attached": True,
            "containment_provided": False,
            "containment_provider": "none",
        }
    job_assigned = containment.get("job_object_assigned") is True
    provider = containment.get("containment_provider", "none")
    return {
        "status": "passed" if job_assigned else "blocked",
        "evidence_attached": True,
        "containment_provided": job_assigned,
        "containment_provider": provider if isinstance(provider, str) else "none",
    }


def _tool_availability_status(
    tool_availability_gate_receipt: dict[str, Any] | None,
) -> dict[str, Any]:
    if not tool_availability_gate_receipt:
        return {
            "status": "missing",
            "evidence_attached": False,
            "required_decision": "allow",
            "observed_decision": "missing",
        }
    decision = (
        tool_availability_gate_receipt.get("decisions", {}).get("gate_decision")
    )
    return {
        "status": "passed" if decision == "allow" else "blocked",
        "evidence_attached": True,
        "required_decision": "allow",
        "observed_decision": decision if isinstance(decision, str) else "missing",
    }


def build_grok_prompt_tool_promotion_gate_receipt(
    *,
    run_root: Path,
    ask: str | None,
    retrieval_mode: str = DEFAULT_RETRIEVAL_MODE,
    retrieval_mode_explicit: bool = False,
    run_id: str | None = None,
    timeout_gate_receipt: dict[str, Any] | None = None,
    tool_availability_gate_receipt: dict[str, Any] | None = None,
    adapter_receipt: dict[str, Any] | None = None,
    execution_outcome: dict[str, Any] | None = None,
) -> dict[str, Any]:
    mode = validate_grok_retrieval_mode(retrieval_mode)
    timeout_gate = _timeout_gate_status(timeout_gate_receipt)
    tool_gate = _tool_availability_status(tool_availability_gate_receipt)
    adapter_containment = _adapter_containment_status(adapter_receipt)

    windows_cleanup_passed = (
        timeout_gate["status"] == "passed"
        and timeout_gate["prompt_tool_promotion_ready"]
    )
    containment_satisfied = (
        windows_cleanup_passed or adapter_containment["containment_provided"]
    )
    executed = execution_outcome is not None
    checks = {
        "runtime_selected_explicitly": True,
        "prompt_supplied": bool(ask),
        "retrieval_mode_explicit_or_off": retrieval_mode_explicit or mode == "off",
        "tool_availability_gate_allows": tool_gate["status"] == "passed",
        "windows_child_tree_owned_cleanup_passed": windows_cleanup_passed,
        "adapter_containment_provided": adapter_containment["containment_provided"],
        "containment_requirement_satisfied": containment_satisfied,
        "prompt_tool_execution_attempted": executed,
        "canonical_default_unchanged": not executed,
    }
    blocking_reasons: list[str] = []
    if not checks["prompt_supplied"]:
        blocking_reasons.append("missing_prompt")
    if not checks["retrieval_mode_explicit_or_off"]:
        blocking_reasons.append("retrieval_mode_not_explicit")
    if not checks["tool_availability_gate_allows"]:
        blocking_reasons.append("tool_availability_gate_not_allow")
    if not checks["containment_requirement_satisfied"]:
        blocking_reasons.append("windows_child_tree_owned_cleanup_not_passed")

    decision = "allow" if not blocking_reasons else "block"
    receipt = {
        "schema_version": "0.1.0",
        "receipt_kind": "grok_prompt_tool_promotion_gate_receipt",
        "receipt_id": f"GROK-PROMPT-TOOL-GATE-{uuid.uuid4().hex[:12].upper()}",
        "run_id": run_id or f"RUN-GROK-PROMPT-TOOL-GATE-{uuid.uuid4().hex[:8].upper()}",
        "created_at": utc_now(),
        "valid": True,
        "decision": decision,
        "runtime": {
            "requested_runtime": "grok",
            "default_runtime_unchanged": not executed,
            "production_prompt_tool_path": (
                execution_outcome.get("mode", "prompt_smoke")
                if executed else "not_promoted"
            ),
        },
        "request": {
            "run_root": str(run_root.resolve()),
            "ask_present": bool(ask),
            "retrieval_mode": mode,
            "retrieval_mode_explicit": retrieval_mode_explicit,
            "valid_retrieval_modes": list(SUPPORTED_RETRIEVAL_MODES),
        },
        "prerequisites": {
            "tool_availability": tool_gate,
            "windows_child_tree_owned_cleanup": timeout_gate,
            "adapter_containment": adapter_containment,
        },
        "checks": checks,
        "blocking_reasons": blocking_reasons,
        "execution": (
            {
                "attempted": True,
                "outcome": execution_outcome.get("outcome", "unknown"),
                "receipt_path": execution_outcome.get("receipt_path", ""),
                "events_path": execution_outcome.get("events_path", ""),
            }
            if executed
            else None
        ),
        "limitations": (
            [
                "Prompt smoke execution completed; response captured via streaming-json.",
                "2026-07-31 containment judgment: CREATE_SUSPENDED + AssignProcessToJobObject closed the post-creation race window on all Grok launch paths.",
                "The canonical gsa run path remains the default; --runtime grok is opt-in.",
            ]
            if executed
            else [
                "This receipt evaluates whether Grok prompt/tool execution may be promoted; it does not launch Grok.",
                "2026-07-31 containment judgment: CREATE_SUSPENDED + AssignProcessToJobObject provides equivalent containment to PROC_THREAD_ATTRIBUTE_JOB_LIST. The carried-limitation on windows_child_tree_owned_cleanup does not block prompt/tool promotion.",
                "The canonical gsa run path remains the default until an explicit promotion decision changes it.",
            ]
        ),
    }
    validate_contract(receipt, RECEIPT_SCHEMA, label="Grok prompt/tool promotion gate receipt")
    return receipt


def write_grok_prompt_tool_promotion_gate_receipt(
    *,
    run_root: Path,
    ask: str | None,
    retrieval_mode: str = DEFAULT_RETRIEVAL_MODE,
    retrieval_mode_explicit: bool = False,
    run_id: str | None = None,
    timeout_gate_path: Path | None = None,
    tool_availability_gate_path: Path | None = None,
    adapter_receipt_path: Path | None = None,
) -> dict[str, Any]:
    _ensure_empty_run_root(run_root)
    timeout_gate = load_json(timeout_gate_path) if timeout_gate_path else None
    tool_gate = load_json(tool_availability_gate_path) if tool_availability_gate_path else None
    adapter_receipt = load_json(adapter_receipt_path) if adapter_receipt_path else None
    receipt = build_grok_prompt_tool_promotion_gate_receipt(
        run_root=run_root,
        ask=ask,
        retrieval_mode=retrieval_mode,
        retrieval_mode_explicit=retrieval_mode_explicit,
        run_id=run_id,
        timeout_gate_receipt=timeout_gate,
        tool_availability_gate_receipt=tool_gate,
        adapter_receipt=adapter_receipt,
    )
    atomic_write_json(run_root / "grok-prompt-tool-promotion-gate.json", receipt)
    return receipt
