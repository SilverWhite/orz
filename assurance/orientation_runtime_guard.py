from __future__ import annotations

from collections import Counter
import re
from typing import Any, Sequence

from .contracts import validate_contract
from .errors import AssuranceError
from .utils import canonical_bytes, sha256_bytes


ORIENTATION_CHECKPOINT_SCHEMA = "orientation-checkpoint-v0.1.schema.json"
ORIENTATION_VERIFICATION_SCHEMA = "orientation-checkpoint-verification-v0.1.schema.json"
STAGNATION_RECEIPT_SCHEMA = "runtime-stagnation-guard-receipt-v0.1.schema.json"

ORIENTATION_BLOCK = """[ORIENTATION_CHECKPOINT v0.1]
当前正在做什么？
当前任务定位是什么？
下一步输出应该服务哪个用户目标？
[/ORIENTATION_CHECKPOINT]"""

ALLOWED_ORIENTATION_FIELDS = [
    "orientation_summary",
    "current_task_position",
    "next_output_target",
]
FORBIDDEN_ORIENTATION_FIELDS = [
    "counterexample_candidate",
    "claim_disposition",
    "claim_promotion",
    "negative_fixture_candidate",
]


def build_orientation_checkpoint(
    *,
    task_id: str,
    trigger_step: int,
    task_contract_sha256: str | None = None,
) -> dict[str, Any]:
    if trigger_step < 0:
        raise AssuranceError("orientation trigger_step must be non-negative")
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
        "message_block": ORIENTATION_BLOCK,
        "allowed_response_fields": ALLOWED_ORIENTATION_FIELDS,
        "forbidden_response_fields": FORBIDDEN_ORIENTATION_FIELDS,
        "claim_policy": {
            "may_generate_counterexample_candidate": False,
            "may_set_claim_disposition": False,
            "claim_strength_effect": "none",
        },
        "notes": [
            "Neutral task orientation only.",
            "This checkpoint must not ask whether the current action is correct or biased.",
        ],
    }
    validate_contract(
        checkpoint,
        ORIENTATION_CHECKPOINT_SCHEMA,
        label="orientation checkpoint",
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


def _normalize_text(value: str) -> str:
    return " ".join(re.findall(r"[\w\u4e00-\u9fff]+", value.casefold()))


def _tokenize(value: str) -> list[str]:
    return re.findall(r"[\w\u4e00-\u9fff]+", value.casefold())


def _max_consecutive_repeated(values: Sequence[str]) -> int:
    best = 0
    previous: str | None = None
    current = 0
    for value in values:
        if not value:
            continue
        if value == previous:
            current += 1
        else:
            previous = value
            current = 1
        best = max(best, current)
    return best


def _max_ngram_repeat(tokens: Sequence[str], *, min_n: int = 3, max_n: int = 8) -> int:
    best = 0
    for ngram_size in range(min_n, max_n + 1):
        if len(tokens) < ngram_size:
            continue
        counts = Counter(
            tuple(tokens[index : index + ngram_size])
            for index in range(0, len(tokens) - ngram_size + 1)
        )
        if counts:
            best = max(best, max(counts.values()))
    return best


def evaluate_runtime_stagnation_guard(
    *,
    public_outputs: Sequence[str],
    retry_count: int = 0,
    retry_budget: int = 1,
    repeated_content_threshold: int = 10,
    ngram_repeat_threshold: int = 10,
    progress_markers: Sequence[str] | None = None,
) -> dict[str, Any]:
    if retry_count < 0 or retry_budget < 0:
        raise AssuranceError("retry count and budget must be non-negative")
    if repeated_content_threshold < 1 or ngram_repeat_threshold < 1:
        raise AssuranceError("stagnation thresholds must be positive")

    normalized_outputs = [_normalize_text(output) for output in public_outputs]
    tokens = _tokenize("\n".join(public_outputs))
    max_consecutive = _max_consecutive_repeated(normalized_outputs)
    max_ngram = _max_ngram_repeat(tokens)
    marker_count = len(set(progress_markers or []))
    reason_codes: list[str] = []
    if max_consecutive > repeated_content_threshold:
        reason_codes.append("STAGNATION-CONSECUTIVE-REPEAT")
    if max_ngram > ngram_repeat_threshold:
        reason_codes.append("STAGNATION-NGRAM-REPEAT")

    triggered = bool(reason_codes)
    if not triggered:
        decision = "continue"
        action = "none"
        restart_packet = None
    elif retry_count < retry_budget:
        decision = "restart_requested"
        action = "stop_and_restart"
        restart_packet = {
            "packet_kind": "terminal_safe_restart_packet",
            "included_state": [
                "task_contract",
                "verified_artifact_ledger",
                "unresolved_questions",
                "last_valid_checkpoint_digest",
            ],
            "runaway_suffix_retained": False,
        }
    else:
        decision = "handoff_required"
        action = "stop_and_handoff"
        restart_packet = None

    receipt = {
        "schema_version": "0.1.0-draft",
        "receipt_kind": "runtime_stagnation_guard_receipt",
        "valid": True,
        "decision": decision,
        "action": action,
        "reason_codes": reason_codes,
        "metrics": {
            "public_output_count": len(public_outputs),
            "token_count": len(tokens),
            "max_consecutive_repeated_content": max_consecutive,
            "max_ngram_repeat": max_ngram,
            "progress_marker_count": marker_count,
        },
        "thresholds": {
            "repeated_content_threshold": repeated_content_threshold,
            "ngram_repeat_threshold": ngram_repeat_threshold,
            "retry_count": retry_count,
            "retry_budget": retry_budget,
        },
        "restart_packet": restart_packet,
        "checks": {
            "metadata_only": True,
            "hidden_chain_of_thought_saved": False,
            "asks_model_if_stuck": False,
            "retry_budget_bounded": retry_count <= retry_budget,
        },
    }
    validate_contract(
        receipt,
        STAGNATION_RECEIPT_SCHEMA,
        label="runtime stagnation guard receipt",
    )
    return receipt
