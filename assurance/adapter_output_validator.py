from __future__ import annotations

import json
from typing import Any

from jsonschema import Draft202012Validator, FormatChecker

from .contracts import validate_contract
from .errors import AssuranceError
from .utils import load_json, sha256_bytes, utc_now


ANSWER_SCHEMA = "canonical-cli-answer-packet-v0.1.schema.json"
CREDENTIAL_PATTERNS = [
    "sk-", "sk-ant-", "sk-or-",  # Anthropic
    "sk-",  # OpenAI
    "Bearer ",  # generic
    "xai-",  # xAI
    "deepseek-",  # DeepSeek
]


def _detect_credential_leak(text: str) -> list[str]:
    """Heuristic scan for credential-like patterns in public output."""
    findings: list[str] = []
    lower = text.lower()
    for pattern in CREDENTIAL_PATTERNS:
        idx = lower.find(pattern)
        if idx >= 0:
            snippet = text[max(0, idx - 5): idx + len(pattern) + 30]
            findings.append(f"possible credential leak: pattern '{pattern}' near '{snippet[:80]}'")
    return findings


def validate_adapter_output(
    *,
    model_output: dict[str, Any],
    answer_packet: dict[str, Any],
    source_gate_receipt: dict[str, Any] | None = None,
    task_contract: dict[str, Any] | None = None,
) -> dict[str, Any]:
    """Validate a model adapter's output against the answer packet schema and gate constraints.

    Checks:
      1. Answer packet conforms to the canonical CLI answer schema.
      2. Claim boundaries align with source visibility gate decisions.
      3. No credential-like patterns in public assistant text.
      4. Source visibility annotations are present when required.
      5. No raw model internals (private reasoning, raw response) leaked to public fields.

    Returns a validation receipt.
    """
    errors: list[str] = []
    checks: dict[str, bool] = {}

    # 1. Schema conformance
    try:
        validate_contract(answer_packet, ANSWER_SCHEMA, label="adapter output answer packet")
        checks["schema_conforms"] = True
    except AssuranceError as exc:
        errors.append(f"answer packet schema validation failed: {exc}")
        checks["schema_conforms"] = False

    # 2. Claim boundaries
    if source_gate_receipt is not None:
        try:
            gate_decision = source_gate_receipt.get("decision", "block")
            claim_strength = answer_packet.get("claim_boundaries", {}).get("scientific_claim_strength", "none")
            if gate_decision == "block" and claim_strength != "none":
                errors.append("claim strength asserted despite source gate block")
                checks["source_gate_respected"] = False
            else:
                checks["source_gate_respected"] = True
        except Exception as exc:
            errors.append(
                f"source gate check failed with unexpected error: {exc}"
            )
            checks["source_gate_respected"] = False
    else:
        checks["source_gate_respected"] = True  # nothing to check

    # 3. Credential leak detection
    public_text = ""
    source_visibility_summary = answer_packet.get("answer", {}).get("source_visibility_summary", [])
    if source_visibility_summary:
        for item in source_visibility_summary:
            if isinstance(item, dict):
                for value in item.values():
                    if isinstance(value, str):
                        public_text += value + " "
    answer_summary = answer_packet.get("answer", {}).get("summary", [])
    if isinstance(answer_summary, list):
        for item in answer_summary:
            if isinstance(item, str):
                public_text += item + " "
    leak_findings = _detect_credential_leak(public_text)
    if leak_findings:
        for finding in leak_findings:
            errors.append(finding)
        checks["no_credential_leak"] = False
    else:
        checks["no_credential_leak"] = True

    # 4. Source visibility annotations present
    if source_gate_receipt is not None:
        ref_decisions = source_gate_receipt.get("reference_decisions", [])
        sv_annotations = answer_packet.get("answer", {}).get("source_visibility_summary", [])
        if ref_decisions and not sv_annotations:
            errors.append("source visibility annotations missing from answer")
            checks["visibility_annotated"] = False
        else:
            checks["visibility_annotated"] = True
    else:
        checks["visibility_annotated"] = True  # nothing to check

    # 5. No raw internals leaked
    raw_internals_keys = {"raw_response", "private_reasoning", "finish_reason_internal", "api_key"}
    public_answer = answer_packet.get("answer", {})
    leaked_internals = raw_internals_keys & set(public_answer.keys() if isinstance(public_answer, dict) else set())
    if leaked_internals:
        errors.append(f"raw model internals leaked to public answer: {sorted(leaked_internals)}")
        checks["no_raw_internals_leak"] = False
    else:
        checks["no_raw_internals_leak"] = True

    valid = not errors

    return {
        "schema_version": "0.1.0-draft",
        "receipt_kind": "adapter_output_validation_receipt",
        "valid": valid,
        "checks": checks,
        "errors": errors,
        "limitations": [
            "Credential leak detection uses heuristic pattern matching; false positives possible.",
            "Source visibility annotation check is structural, not semantic.",
            "Does not verify factual correctness or model safety.",
        ],
    }
