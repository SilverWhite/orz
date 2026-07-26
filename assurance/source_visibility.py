from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from .contracts import validate_contract
from .errors import AssuranceError
from .utils import load_json


LEDGER_SCHEMA = "source-visibility-ledger-v0.1.schema.json"
RECEIPT_SCHEMA = "source-visibility-gate-receipt-v0.1.schema.json"

VISIBILITY_RANK = {
    "unavailable": 0,
    "metadata_only": 1,
    "partial_text_observed": 2,
    "full_text_observed": 3,
}

CLAIM_REQUIRED_VISIBILITY = {
    "bibliographic_presence": "metadata_only",
    "metadata_fact": "metadata_only",
    "abstract_or_snippet_summary": "partial_text_observed",
    "section_summary": "partial_text_observed",
    "full_text_summary": "full_text_observed",
    "mechanism_claim": "full_text_observed",
    "methods_claim": "full_text_observed",
    "limitations_claim": "full_text_observed",
    "comparative_synthesis": "full_text_observed",
    "author_position_claim": "full_text_observed",
    "thread_summary": "full_text_observed",
}

CLAIM_ALLOWED_BY_VISIBILITY = {
    "full_text_observed": "full_text_claim",
    "partial_text_observed": "observed_fragment_only",
    "metadata_only": "metadata_only",
    "unavailable": "locator_only",
    "unregistered": "none",
}


def _overall_decision(reference_decisions: list[dict[str, Any]]) -> str:
    decisions = {item["decision"] for item in reference_decisions}
    if "block" in decisions:
        return "block"
    if "defer" in decisions:
        return "defer"
    return "allow"


def _source_index(sources: list[dict[str, Any]]) -> tuple[dict[str, dict[str, Any]], list[str]]:
    errors: list[str] = []
    indexed: dict[str, dict[str, Any]] = {}
    for source in sources:
        source_id = source["source_id"]
        if source_id in indexed:
            errors.append(f"duplicate source_id: {source_id}")
            continue
        indexed[source_id] = source
    return indexed, errors


def _reference_index(
    references: list[dict[str, Any]],
) -> tuple[dict[str, dict[str, Any]], list[str]]:
    errors: list[str] = []
    indexed: dict[str, dict[str, Any]] = {}
    for reference in references:
        ref_id = reference["ref_id"]
        if ref_id in indexed:
            errors.append(f"duplicate ref_id: {ref_id}")
            continue
        indexed[ref_id] = reference
    return indexed, errors


def _context_tokens_estimate(sources: list[dict[str, Any]]) -> int:
    return sum(
        attempt["context_tokens_estimate"]
        for source in sources
        for attempt in source["retrieval_attempts"]
    )


def _budget_warnings(ledger: dict[str, Any], context_tokens: int) -> list[str]:
    policy = ledger["retrieval_policy"]
    warnings: list[str] = []
    if len(ledger["sources"]) > policy["max_sources_per_pass"]:
        warnings.append("source count exceeds max_sources_per_pass")
    fulltext_sources = [
        source
        for source in ledger["sources"]
        if source["visibility_status"] == "full_text_observed"
    ]
    if len(fulltext_sources) > policy["max_fulltext_sources_per_pass"]:
        warnings.append("full-text source count exceeds max_fulltext_sources_per_pass")
    if context_tokens > policy["max_context_tokens_per_pass"]:
        warnings.append("context token estimate exceeds max_context_tokens_per_pass")
    return warnings


def evaluate_source_visibility_gate(ledger: dict[str, Any]) -> dict[str, Any]:
    validate_contract(ledger, LEDGER_SCHEMA, label="source visibility ledger")
    sources_by_id, source_errors = _source_index(ledger["sources"])
    references_by_id, reference_errors = _reference_index(ledger["references"])
    context_tokens = _context_tokens_estimate(ledger["sources"])
    warnings = _budget_warnings(ledger, context_tokens)
    reference_decisions: list[dict[str, Any]] = []

    for error in source_errors:
        reference_decisions.append(
            {
                "ref_id": "S0",
                "source_id": "SOURCE",
                "claim_type": "registry_integrity",
                "observed_visibility": "unregistered",
                "required_visibility": "metadata_only",
                "decision": "block",
                "claim_allowed": "none",
                "must_label_in_output": True,
                "reason_codes": ["SOURCE-VISIBILITY-DUPLICATE-ID"],
                "missing_scope": [error],
            }
        )
    for error in reference_errors:
        reference_decisions.append(
            {
                "ref_id": "S0",
                "source_id": "REFERENCE",
                "claim_type": "registry_integrity",
                "observed_visibility": "unregistered",
                "required_visibility": "metadata_only",
                "decision": "block",
                "claim_allowed": "none",
                "must_label_in_output": True,
                "reason_codes": ["SOURCE-VISIBILITY-DUPLICATE-ID"],
                "missing_scope": [error],
            }
        )

    for cited_ref_id in ledger["gate_request"]["cited_ref_ids"]:
        reference = references_by_id.get(cited_ref_id)
        if reference is None:
            reference_decisions.append(
                {
                    "ref_id": cited_ref_id,
                    "source_id": "MISSING",
                    "claim_type": "unregistered_reference",
                    "observed_visibility": "unregistered",
                    "required_visibility": "metadata_only",
                    "decision": "block",
                    "claim_allowed": "none",
                    "must_label_in_output": True,
                    "reason_codes": ["SOURCE-VISIBILITY-UNREGISTERED-REFERENCE"],
                    "missing_scope": ["reference was cited but not registered"],
                }
            )
            continue
        source = sources_by_id.get(reference["source_id"])
        required_visibility = CLAIM_REQUIRED_VISIBILITY[reference["claim_type"]]
        if source is None:
            reference_decisions.append(
                {
                    "ref_id": cited_ref_id,
                    "source_id": reference["source_id"],
                    "claim_type": reference["claim_type"],
                    "observed_visibility": "unregistered",
                    "required_visibility": required_visibility,
                    "decision": "block",
                    "claim_allowed": "none",
                    "must_label_in_output": True,
                    "reason_codes": ["SOURCE-VISIBILITY-UNREGISTERED-SOURCE"],
                    "missing_scope": ["source_id was referenced but not registered"],
                }
            )
            continue

        observed_visibility = source["visibility_status"]
        reason_codes: list[str] = []
        missing_scope = list(source["missing_scope"])
        if source["secondary_source"]:
            reason_codes.append("SOURCE-VISIBILITY-SECONDARY-SOURCE")
        if observed_visibility == "unavailable":
            reason_codes.append("SOURCE-VISIBILITY-UNAVAILABLE")
        elif observed_visibility == "metadata_only":
            reason_codes.append("SOURCE-VISIBILITY-METADATA-ONLY")
        if warnings:
            reason_codes.append("SOURCE-VISIBILITY-BUDGET-EXCEEDED")

        if VISIBILITY_RANK[observed_visibility] >= VISIBILITY_RANK[required_visibility]:
            decision = "allow"
            reason_codes.append("SOURCE-VISIBILITY-OK")
        elif observed_visibility == "unavailable":
            decision = "block"
        else:
            decision = "defer"
            reason_codes.append("SOURCE-VISIBILITY-FULLTEXT-REQUIRED")
            if not missing_scope:
                missing_scope.append("full text was not observed")

        if reference["requires_full_text_if_used_as_written"] and observed_visibility != "full_text_observed":
            if decision == "allow":
                decision = "defer"
            if "SOURCE-VISIBILITY-FULLTEXT-REQUIRED" not in reason_codes:
                reason_codes.append("SOURCE-VISIBILITY-FULLTEXT-REQUIRED")
            if not missing_scope:
                missing_scope.append("full text required by reference record")
        if warnings and decision == "allow":
            decision = "defer"

        reference_decisions.append(
            {
                "ref_id": cited_ref_id,
                "source_id": source["source_id"],
                "claim_type": reference["claim_type"],
                "observed_visibility": observed_visibility,
                "required_visibility": required_visibility,
                "decision": decision,
                "claim_allowed": CLAIM_ALLOWED_BY_VISIBILITY[observed_visibility],
                "must_label_in_output": True,
                "reason_codes": sorted(set(reason_codes)),
                "missing_scope": missing_scope,
            }
        )

    receipt = {
        "schema_version": "0.1.0-draft",
        "receipt_kind": "source_fulltext_visibility_gate_receipt",
        "request_id": ledger["gate_request"]["request_id"],
        "ledger_id": ledger["ledger_id"],
        "decision": _overall_decision(reference_decisions),
        "must_report_visibility_status": True,
        "retrieval_policy": {
            "pass_index": ledger["retrieval_policy"]["pass_index"],
            "max_sources_per_pass": ledger["retrieval_policy"]["max_sources_per_pass"],
            "max_fulltext_sources_per_pass": ledger["retrieval_policy"]["max_fulltext_sources_per_pass"],
            "max_context_tokens_per_pass": ledger["retrieval_policy"]["max_context_tokens_per_pass"],
            "allow_incremental_retrieval": True,
        },
        "source_count": len(ledger["sources"]),
        "reference_count": len(ledger["gate_request"]["cited_ref_ids"]),
        "context_tokens_estimate": context_tokens,
        "reference_decisions": reference_decisions,
        "warnings": warnings,
        "limitations": [
            "This gate verifies source visibility registration, not source truth.",
            "Full-text observation does not imply scientific correctness.",
            "Partial visibility must be surfaced in the answer and may require incremental retrieval.",
        ],
    }
    validate_contract(receipt, RECEIPT_SCHEMA, label="source visibility gate receipt")
    return receipt


def evaluate_source_visibility_ledger_file(path: Path) -> dict[str, Any]:
    return evaluate_source_visibility_gate(load_json(path))


def render_visibility_summary(receipt: dict[str, Any]) -> str:
    lines = [
        f"source visibility gate: {receipt['decision']}",
        f"context tokens estimate: {receipt['context_tokens_estimate']}",
    ]
    for item in receipt["reference_decisions"]:
        lines.append(
            f"{item['ref_id']} {item['observed_visibility']} "
            f"required={item['required_visibility']} decision={item['decision']} "
            f"claim_allowed={item['claim_allowed']}"
        )
    return "\n".join(lines)


def dumps_receipt(receipt: dict[str, Any]) -> str:
    return json.dumps(
        receipt,
        ensure_ascii=False,
        indent=2,
        sort_keys=True,
        allow_nan=False,
    )
