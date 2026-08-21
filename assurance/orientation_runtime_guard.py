from __future__ import annotations

import re
from typing import Any, Sequence

from .contracts import validate_contract
from .errors import AssuranceError
from .utils import canonical_bytes, sha256_bytes


ORIENTATION_CHECKPOINT_SCHEMA = "orientation-checkpoint-v0.1.schema.json"
ORIENTATION_VERIFICATION_SCHEMA = "orientation-checkpoint-verification-v0.1.schema.json"

ORIENTATION_BLOCK = """[ORIENTATION_CHECKPOINT v0.1]
当前正在做什么？
当前任务定位是什么？
下一步输出应该服务哪个用户目标？
[/ORIENTATION_CHECKPOINT]"""

CHECKLIST_CONTEXT_TEMPLATE = (
    "[CHECKLIST_CONTEXT v0.1]\n"
    "当前步骤: {step_id} — {title}\n"
    "任务位置: 步骤 {position} / 共 {total} 项\n"
    "步骤状态: {status}\n"
    "[/CHECKLIST_CONTEXT]"
)

ALLOWED_ORIENTATION_FIELDS = [
    "orientation_summary",
    "current_task_position",
    "next_output_target",
    "available_tools_acknowledged",
]
FORBIDDEN_ORIENTATION_FIELDS = [
    "counterexample_candidate",
    "claim_disposition",
    "claim_promotion",
    "negative_fixture_candidate",
    "tool_belief_mismatch_statement",
]


def build_orientation_checkpoint(
    *,
    task_id: str,
    trigger_step: int,
    task_contract_sha256: str | None = None,
    tool_availability_sha256: str | None = None,
    checklist_context: dict[str, Any] | None = None,
) -> dict[str, Any]:
    if trigger_step < 0:
        raise AssuranceError("orientation trigger_step must be non-negative")

    # Build message block with optional checklist context injection.
    message_block = ORIENTATION_BLOCK
    if checklist_context:
        ctx_block = CHECKLIST_CONTEXT_TEMPLATE.format(
            step_id=checklist_context.get("step_id", ""),
            title=checklist_context.get("title", ""),
            position=checklist_context.get("position", "?"),
            total=checklist_context.get("total", "?"),
            status=checklist_context.get("status", "todo"),
        )
        message_block = ctx_block + "\n\n" + ORIENTATION_BLOCK

    checkpoint = {
        "schema_version": "0.1.0-draft",
        "checkpoint_kind": "orientation_checkpoint",
        "checkpoint_id": f"ORIENT-{task_id}-{trigger_step:04d}",
        "task_id": task_id,
        "trigger": {
            "trigger_type": "fixed_step_interval",
            "step_index": trigger_step,
            "task_contract_sha256": task_contract_sha256,
        },
        "message_block": message_block,
        "allowed_response_fields": ALLOWED_ORIENTATION_FIELDS,
        "forbidden_response_fields": FORBIDDEN_ORIENTATION_FIELDS,
        "claim_policy": {
            "may_generate_counterexample_candidate": False,
            "may_set_claim_disposition": False,
            "claim_strength_effect": "none",
        },
        "tool_availability_sha256": tool_availability_sha256,
        "notes": [
            "Neutral task orientation only.",
            "This checkpoint must not ask whether the current action is correct or biased.",
            "Checklist context (if present) is neutral positional data only.",
        ],
    }
    validate_contract(
        checkpoint,
        ORIENTATION_CHECKPOINT_SCHEMA,
        label="orientation checkpoint",
    )
    return checkpoint


def build_tool_availability_infused_orientation_block(
    *,
    tool_availability_context_block: str,
) -> str:
    return tool_availability_context_block + "\n\n" + ORIENTATION_BLOCK


def build_tool_availability_infused_orientation_checkpoint(
    *,
    task_id: str,
    trigger_step: int,
    tool_availability_context_block: str,
    task_contract_sha256: str | None = None,
    tool_availability_sha256: str | None = None,
) -> dict[str, Any]:
    if trigger_step < 0:
        raise AssuranceError("orientation trigger_step must be non-negative")
    infused_block = build_tool_availability_infused_orientation_block(
        tool_availability_context_block=tool_availability_context_block,
    )
    checkpoint = {
        "schema_version": "0.1.0-draft",
        "checkpoint_kind": "orientation_checkpoint",
        "checkpoint_id": f"ORIENT-{task_id}-{trigger_step:04d}",
        "task_id": task_id,
        "trigger": {
            "trigger_type": "fixed_step_interval",
            "step_index": trigger_step,
            "task_contract_sha256": task_contract_sha256,
        },
        "message_block": infused_block,
        "allowed_response_fields": ALLOWED_ORIENTATION_FIELDS,
        "forbidden_response_fields": FORBIDDEN_ORIENTATION_FIELDS,
        "claim_policy": {
            "may_generate_counterexample_candidate": False,
            "may_set_claim_disposition": False,
            "claim_strength_effect": "none",
        },
        "tool_availability_sha256": tool_availability_sha256,
        "notes": [
            "Neutral task orientation with tool availability context.",
            "This checkpoint must not ask whether the current action is correct or biased.",
            "Tool availability is declared by the runtime, not inferred by the model.",
        ],
    }
    validate_contract(
        checkpoint,
        ORIENTATION_CHECKPOINT_SCHEMA,
        label="tool-availability-infused orientation checkpoint",
    )
    return checkpoint


def verify_orientation_response(
    *,
    checkpoint: dict[str, Any],
    response: dict[str, Any],
) -> dict[str, Any]:
    validate_contract(
        checkpoint,
        ORIENTATION_CHECKPOINT_SCHEMA,
        label="orientation checkpoint",
    )
    allowed = set(checkpoint["allowed_response_fields"])
    forbidden = set(checkpoint["forbidden_response_fields"])
    present = set(response)
    forbidden_present = sorted(present & forbidden)
    unexpected_present = sorted(present - allowed - forbidden)
    valid = not forbidden_present and not unexpected_present
    receipt = {
        "schema_version": "0.1.0-draft",
        "receipt_kind": "orientation_checkpoint_verification",
        "valid": valid,
        "checkpoint_id": checkpoint["checkpoint_id"],
        "checkpoint_sha256": sha256_bytes(canonical_bytes(checkpoint)),
        "response_sha256": sha256_bytes(canonical_bytes(response)),
        "allowed_fields_observed": sorted(present & allowed),
        "forbidden_fields_observed": forbidden_present,
        "unexpected_fields_observed": unexpected_present,
        "checks": {
            "neutral_orientation_only": valid,
            "no_counterexample_candidate": "counterexample_candidate" not in present,
            "no_claim_disposition": "claim_disposition" not in present,
            "no_claim_strength_effect": True,
        },
    }
    validate_contract(
        receipt,
        ORIENTATION_VERIFICATION_SCHEMA,
        label="orientation checkpoint verification",
    )
    return receipt


def _extract_tool_names_from_text(text: str, *, known_tool_names: Sequence[str]) -> list[str]:
    found: list[str] = []
    for name in known_tool_names:
        if name in text:
            found.append(name)
    return found


def _guess_tool_usage_from_text(text: str) -> bool:
    patterns = [
        r"(?:使用|use|调用|call)\s*(?:了|的|the|a)?\s*(?:工具|tool|搜索|检索|bash|命令|function)",
        r"(?:搜索|search)\s*(?:到|了|结果|result|the|for|online|一下|这个|那个|一下|关于|related|.*(?:tool|ability|capability))",
        r"(?:读取|read)\s*(?:了|文件|the|file)",
        r"(?:执行|运行|run)\s*(?:了|命令|command|bash)",
        r"(?:获取|fetch|抓取)\s*(?:了|网页|the|web|url|page|data)",
        r"(?:浏览|browse|look\s*up)\s*(?:网页|web|online|the|结果)",
        r"(?:write|edit|写入|编辑)\s*(?:文件|the|file|to)",
        r"(?:我.*(?:搜索|查找|search|look\s*up|检索))",
        r"(?:I.*(?:search|look\s*up|browse|fetch|retrieve))",
    ]
    return any(re.search(pattern, text, re.IGNORECASE) for pattern in patterns)


def evaluate_tool_belief_stagnation(
    *,
    public_outputs: Sequence[str],
    available_tool_names: Sequence[str],
    unavailable_tool_names: Sequence[str],
) -> dict[str, Any]:
    reason_codes: list[str] = []
    observed_mentions: list[dict[str, Any]] = []

    for turn_index, text in enumerate(public_outputs):
        found_available = _extract_tool_names_from_text(
            text, known_tool_names=available_tool_names
        )
        found_unavailable = _extract_tool_names_from_text(
            text, known_tool_names=unavailable_tool_names
        )
        usage_guessed = _guess_tool_usage_from_text(text)

        if found_unavailable:
            reason_codes.append("TOOL-BELIEF-UNAVAILABLE-TOOL-MENTIONED")
            for name in found_unavailable:
                observed_mentions.append({
                    "turn_index": turn_index,
                    "tool_name": name,
                    "status": "unavailable",
                    "detection": "explicit_name_mention",
                })
        if usage_guessed:
            if not found_available:
                reason_codes.append("TOOL-BELIEF-CAPABILITY-GUESSING")
            observed_mentions.append({
                "turn_index": turn_index,
                "tool_name": None,
                "status": "guessing",
                "detection": "tool_usage_language_pattern",
            })

    explicit_mismatch = "TOOL-BELIEF-UNAVAILABLE-TOOL-MENTIONED" in reason_codes
    triggered = explicit_mismatch
    receipt = {
        "schema_version": "0.1.0-draft",
        "receipt_kind": "tool_availability_belief_stagnation_receipt",
        "valid": not triggered,
        "decision": "restart_requested" if triggered else "continue",
        "action": "stop_and_restart" if triggered else "none",
        "reason_codes": sorted(set(reason_codes)),
        "observed_mentions": observed_mentions,
        "metrics": {
            "public_output_count": len(public_outputs),
            "available_tool_count": len(available_tool_names),
            "unavailable_tool_count": len(unavailable_tool_names),
            "tool_belief_mismatch_count": len(observed_mentions),
        },
        "checks": {
            "no_unavailable_tool_mentioned": not explicit_mismatch,
            "no_capability_guessing": "TOOL-BELIEF-CAPABILITY-GUESSING" not in reason_codes,
            "mechanical_probe_only": True,
            "asks_model_if_stuck": False,
            "hidden_chain_of_thought_saved": False,
        },
        "limitations": [
            "Tool name detection is based on substring matching of declared tool names; unknown tool aliases may be missed.",
            "Capability guessing detection uses keyword patterns and may produce false positives for hypothetical discussions.",
            "This detector does not read hidden chain-of-thought.",
        ],
    }
    return receipt
