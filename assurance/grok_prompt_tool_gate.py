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
) -> dict[str, Any]:
    mode = validate_grok_retrieval_mode(retrieval_mode)
    timeout_gate = _timeout_gate_status(timeout_gate_receipt)
    tool_gate = _tool_availability_status(tool_availability_gate_receipt)
    checks = {
        "runtime_selected_explicitly": True,
        "prompt_supplied": bool(ask),
        "retrieval_mode_explicit_or_off": retrieval_mode_explicit or mode == "off",
        "tool_availability_gate_allows": tool_gate["status"] == "passed",
        "windows_child_tree_owned_cleanup_passed": timeout_gate["status"] == "passed"
        and timeout_gate["prompt_tool_promotion_ready"],
        "prompt_tool_execution_attempted": False,
        "canonical_default_unchanged": True,
    }
    blocking_reasons: list[str] = []
    if not checks["prompt_supplied"]:
        blocking_reasons.append("missing_prompt")
    if not checks["retrieval_mode_explicit_or_off"]:
        blocking_reasons.append("retrieval_mode_not_explicit")
    if not checks["tool_availability_gate_allows"]:
        blocking_reasons.append("tool_availability_gate_not_allow")
    if not checks["windows_child_tree_owned_cleanup_passed"]:
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
            "default_runtime_unchanged": True,
            "production_prompt_tool_path": "not_promoted",
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
        },
        "checks": checks,
        "blocking_reasons": blocking_reasons,
        "limitations": [
            "This receipt evaluates whether Grok prompt/tool execution may be promoted; it does not launch Grok.",
            "A blocked decision is expected while windows_child_tree_owned_cleanup remains a carried limitation.",
            "The canonical gsa run path remains the default until an explicit promotion decision changes it.",
        ],
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
) -> dict[str, Any]:
    _ensure_empty_run_root(run_root)
    timeout_gate = load_json(timeout_gate_path) if timeout_gate_path else None
    tool_gate = load_json(tool_availability_gate_path) if tool_availability_gate_path else None
    receipt = build_grok_prompt_tool_promotion_gate_receipt(
        run_root=run_root,
        ask=ask,
        retrieval_mode=retrieval_mode,
        retrieval_mode_explicit=retrieval_mode_explicit,
        run_id=run_id,
        timeout_gate_receipt=timeout_gate,
        tool_availability_gate_receipt=tool_gate,
    )
    atomic_write_json(run_root / "grok-prompt-tool-promotion-gate.json", receipt)
    return receipt
