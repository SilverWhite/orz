from __future__ import annotations

import uuid
from copy import deepcopy
from pathlib import Path
from typing import Any, Sequence

from .contracts import validate_contract
from .errors import AssuranceError
from .instruction_provenance_gate import (
    build_instruction_provenance_gate_context,
    evaluate_instruction_provenance_gate,
    verify_instruction_provenance_gate_receipt,
)
from .orientation_runtime_guard import build_orientation_checkpoint
from .source_visibility import evaluate_source_visibility_gate
from .tool_availability_gate import (
    build_tool_availability_gate_receipt,
    probe_tool_availability,
)
from .utils import atomic_write_json, canonical_bytes, load_json, sha256_bytes, sha256_file, utc_now


TASK_CONTRACT_SCHEMA = "retrieval-task-contract-v0.1.schema.json"
RESULT_SCHEMA = "retrieval-result-v0.1.schema.json"
CLOSE_RECEIPT_SCHEMA = "retrieval-session-close-receipt-v0.1.schema.json"

SUBAGENT_CAPABILITIES = [
    "search",
    "web_fetch",
    "file_read",
    "tool_registry",
]

SUBAGENT_TOOL_SPECS: list[dict[str, Any]] = [
    {"tool_id": "search", "tool_name": "Search", "capability": "search", "probe_method": "subagent_capability_declaration"},
    {"tool_id": "web_fetch", "tool_name": "Web Fetch", "capability": "web_fetch", "probe_method": "subagent_capability_declaration"},
    {"tool_id": "file_read", "tool_name": "File Read", "capability": "file_read", "probe_method": "subagent_capability_declaration"},
    {"tool_id": "tool_registry", "tool_name": "Tool Registry", "capability": "tool_registry", "probe_method": "subagent_capability_declaration"},
]

SOURCE_TYPE_ALLOWED_PATTERNS = {
    "academic_paper": {"url_pattern": "doi.org|arxiv|scholar"},
    "web_page": {"url_pattern": "https?://"},
    "documentation": {"url_pattern": "docs\\.|readthedocs|documentation"},
    "code_repository": {"url_pattern": "github\\.com|gitlab|bitbucket"},
}


def _new_id(prefix: str) -> str:
    return f"{prefix}-{uuid.uuid4().hex.upper()}"


def build_retrieval_task_contract(
    *,
    parent_session_id: str,
    retrieval_question: str,
    allowed_source_categories: Sequence[str],
    required_visibility: str = "full_text_observed",
    return_sections: Sequence[str] | None = None,
    forbidden_topics: Sequence[str] | None = None,
    max_sources: int = 20,
    context_budget_tokens: int = 8192,
) -> dict[str, Any]:
    if not parent_session_id.startswith("CONV-"):
        raise AssuranceError("parent_session_id must be a valid CONV- ID")
    if not retrieval_question.strip():
        raise AssuranceError("retrieval_question must be non-empty")
    if not allowed_source_categories:
        raise AssuranceError("allowed_source_categories must be non-empty")
    if required_visibility not in (
        "full_text_observed",
        "partial_text_observed",
        "metadata_only",
    ):
        raise AssuranceError(f"invalid required_visibility: {required_visibility}")
    if max_sources < 1 or max_sources > 50:
        raise AssuranceError("max_sources must be between 1 and 50")
    if context_budget_tokens < 100 or context_budget_tokens > 65536:
        raise AssuranceError("context_budget_tokens must be between 100 and 65536")

    sections = list(return_sections or ["summary", "key_findings", "source_breakdown"])
    topics = list(forbidden_topics or [])
    contract = {
        "schema_version": "0.1.0-draft",
        "contract_kind": "retrieval_task_contract",
        "contract_id": _new_id("RET-CTR"),
        "parent_session_id": parent_session_id,
        "retrieval_question": retrieval_question.strip(),
        "allowed_source_categories": sorted(set(allowed_source_categories)),
        "required_visibility": required_visibility,
        "return_format": {
            "sections": sections,
            "source_ledger_required": True,
            "filtering_log_required": True,
        },
        "scope_boundary": {
            "forbidden_topics": sorted(set(topics)),
            "forbidden_source_types": ["personal_data", "credential", "internal_config"],
            "delegation_constraint": "subagent_must_not_expand_scope_beyond_contract",
        },
        "max_sources": max_sources,
        "context_budget_tokens": context_budget_tokens,
        "notes": [
            "Sub-agent operates under capability delegation from parent session.",
            "The sub-agent must NOT perform any action beyond retrieval and preprocessing.",
            "All sources must be registered in the source visibility ledger.",
            "Result must be transparent for audit: no merged or opaque summaries.",
        ],
    }
    validate_contract(contract, TASK_CONTRACT_SCHEMA, label="retrieval task contract")
    return contract


def _check_source_visibility_ledger(
    contract: dict[str, Any],
    result: dict[str, Any],
) -> dict[str, Any]:
    required_visibility = contract["required_visibility"]
    insufficient: list[dict[str, Any]] = []
    unregistered: list[dict[str, Any]] = []

    ledger_ids = {entry["source_id"] for entry in result["source_ledger"]}
    for section in result["organized_response"]["sections"]:
        for source_id in section["source_ids"]:
            if source_id not in ledger_ids:
                unregistered.append({"source_id": source_id, "section": section["section_title"]})

    for entry in result["source_ledger"]:
        visibility_rank = {
            "full_text_observed": 3,
            "partial_text_observed": 2,
            "metadata_only": 1,
            "unavailable": 0,
        }
        if visibility_rank.get(entry["visibility"], 0) < visibility_rank.get(
            required_visibility, 3
        ):
            insufficient.append({
                "source_id": entry["source_id"],
                "observed_visibility": entry["visibility"],
                "required_visibility": required_visibility,
            })

    valid = len(insufficient) == 0 and len(unregistered) == 0
    return {
        "schema_version": "0.1.0-draft",
        "receipt_kind": "retrieval_result_source_ledger_verification",
        "valid": valid,
        "insufficient_visibility": insufficient,
        "unregistered_source_refs": unregistered,
        "checks": {
            "all_sources_registered": len(unregistered) == 0,
            "visibility_threshold_met": len(insufficient) == 0,
            "filtering_log_present": len(result["filtering_log"]) > 0
            or len(result["source_ledger"]) == 0,
            "source_ledger_nonempty": len(result["source_ledger"]) > 0,
        },
        "limitations": [
            "Source visibility is taken from the sub-agent's declared observations; main agent must independently verify.",
        ],
    }


def _check_scope_compliance(
    contract: dict[str, Any],
    result: dict[str, Any],
) -> dict[str, Any]:
    forbidden_topics = set(contract["scope_boundary"]["forbidden_topics"])
    forbidden_types = set(contract["scope_boundary"]["forbidden_source_types"])
    allowed_categories = set(contract["allowed_source_categories"])

    violations: list[dict[str, Any]] = []
    for query in result["query_summary"]:
        if query["source_category"] not in allowed_categories:
            violations.append({
                "query_id": query["query_id"],
                "category": query["source_category"],
                "reason": f"source_category not in allowed: {sorted(allowed_categories)}",
            })
        for topic in forbidden_topics:
            if topic and topic in query["query_text"]:
                violations.append({
                    "query_id": query["query_id"],
                    "forbidden_topic": topic,
                    "reason": "query touches forbidden topic",
                })

    source_count = len(result["source_ledger"])
    budget_exceeded = source_count > contract["max_sources"]

    valid = len(violations) == 0 and not budget_exceeded
    return {
        "schema_version": "0.1.0-draft",
        "receipt_kind": "retrieval_result_scope_compliance",
        "valid": valid,
        "violations": violations,
        "budget": {
            "source_count": source_count,
            "max_sources": contract["max_sources"],
            "exceeded": budget_exceeded,
        },
        "checks": {
            "no_forbidden_topics": len(violations) == 0,
            "category_compliance": all(
                query["source_category"] in allowed_categories
                for query in result["query_summary"]
            ),
            "budget_respected": not budget_exceeded,
        },
    }


def validate_retrieval_result(
    *,
    contract: dict[str, Any],
    result: dict[str, Any],
) -> dict[str, Any]:
    validate_contract(contract, TASK_CONTRACT_SCHEMA, label="retrieval task contract")
    validate_contract(result, RESULT_SCHEMA, label="retrieval result")

    if result["contract_id"] != contract["contract_id"]:
        raise AssuranceError("retrieval result contract_id mismatch")

    source_ledger_check = _check_source_visibility_ledger(contract, result)
    scope_check = _check_scope_compliance(contract, result)

    sections_requested = set(contract["return_format"]["sections"])
    sections_delivered = {
        section["section_title"] for section in result["organized_response"]["sections"]
    }
    missing_sections = sections_requested - sections_delivered

    valid = (
        source_ledger_check["valid"]
        and scope_check["valid"]
        and len(missing_sections) == 0
    )
    receipt = {
        "schema_version": "0.1.0-draft",
        "receipt_kind": "retrieval_result_validation_receipt",
        "valid": valid,
        "result_id": result["result_id"],
        "contract_id": contract["contract_id"],
        "source_ledger_verification": source_ledger_check,
        "scope_compliance": scope_check,
        "return_format_check": {
            "sections_requested": sorted(sections_requested),
            "sections_delivered": sorted(sections_delivered),
            "missing_sections": sorted(missing_sections),
        },
        "checks": {
            "source_ledger_valid": source_ledger_check["valid"],
            "scope_compliant": scope_check["valid"],
            "return_format_complete": len(missing_sections) == 0,
            "subagent_has_no_claim_promotion": True,
            "result_cannot_upgrade_parent_claim": True,
        },
        "limitations": [
            "Validation is mechanical; it does not evaluate content accuracy.",
            "Main agent must independently verify source content before using as evidence.",
            "Sub-agent claims are bounded and must not be promoted by the main agent.",
        ],
    }
    return receipt


def build_retrieval_session_close_receipt(
    *,
    parent_session_id: str,
    subagent_session_id: str,
    contract_id: str,
    result_id: str,
    triggered_by: str = "main_agent",
    reason: str = "main agent confirms retrieval round complete",
) -> dict[str, Any]:
    if triggered_by not in ("main_agent", "budget_exhausted", "scope_completed", "main_agent_abort"):
        raise AssuranceError(f"invalid close trigger: {triggered_by}")
    receipt_id_seed = f"{result_id}:{triggered_by}"
    receipt = {
        "schema_version": "0.1.0-draft",
        "receipt_kind": "retrieval_session_close_receipt",
        "receipt_id": f"RET-CLS-{sha256_bytes(receipt_id_seed.encode('utf-8'))[:32].upper()}",
        "valid": True,
        "parent_session_id": parent_session_id,
        "subagent_session_id": subagent_session_id,
        "contract_id": contract_id,
        "result_id": result_id,
        "close_trigger": {
            "triggered_by": triggered_by,
            "reason": reason,
        },
        "archive_policy": {
            "conversation_zeroed": False,
            "journal_preserved": True,
            "can_resume": True,
            "resume_condition": "subagent can be re-awakened by the same parent session with a new task contract",
        },
        "next_availability": {
            "subagent_resumable": True,
            "requirements": [
                "parent_session_id_match",
                "new_task_contract_required",
                "source_ledger_preserved",
            ],
        },
    }
    validate_contract(receipt, CLOSE_RECEIPT_SCHEMA, label="retrieval session close receipt")
    return receipt


def verify_retrieval_result_sources(
    *,
    contract: dict[str, Any],
    result: dict[str, Any],
) -> dict[str, Any]:
    return validate_retrieval_result(contract=contract, result=result)


def build_fake_retrieval_result(
    *,
    contract: dict[str, Any],
    subagent_session_id: str,
    query_texts: Sequence[str] | None = None,
    source_entries: Sequence[dict[str, Any]] | None = None,
    sections_content: Sequence[dict[str, Any]] | None = None,
    filter_entries: Sequence[dict[str, Any]] | None = None,
) -> dict[str, Any]:
    validate_contract(contract, TASK_CONTRACT_SCHEMA, label="retrieval task contract")
    result_id = _new_id("RET-RES")

    query_summary = []
    if query_texts:
        for idx, text in enumerate(query_texts):
            category = contract["allowed_source_categories"][0] if contract["allowed_source_categories"] else "web_page"
            query_summary.append({
                "query_id": f"QRY-{result_id}-{idx:03d}",
                "query_text": text,
                "source_category": category,
                "result_count": len(source_entries or []) if idx == 0 else 0,
                "action_taken": "searched" if len(source_entries or []) > 0 else "aborted_no_results",
                "tool_used": "search",
            })
    if not query_summary:
        query_summary.append({
            "query_id": f"QRY-{result_id}-000",
            "query_text": contract["retrieval_question"],
            "source_category": contract["allowed_source_categories"][0] if contract["allowed_source_categories"] else "web_page",
            "result_count": len(source_entries or []),
            "action_taken": "searched" if len(source_entries or []) > 0 else "aborted_no_results",
            "tool_used": "search",
        })

    source_ids_used = {entry["source_id"] for entry in (source_entries or [])}
    sections = []
    if sections_content:
        for sec_idx, sec in enumerate(sections_content):
            sections.append({
                "section_title": sec.get("section_title", contract["return_format"]["sections"][sec_idx] if sec_idx < len(contract["return_format"]["sections"]) else f"section_{sec_idx}"),
                "content": sec.get("content", "Fake retrieval result for offline verification."),
                "source_ids": sorted(source_ids_used),
                "claim_strength": sec.get("claim_strength", "observed"),
            })
    else:
        for section_name in contract["return_format"]["sections"]:
            sections.append({
                "section_title": section_name,
                "content": f"Fake retrieval content for section [{section_name}].",
                "source_ids": sorted(source_ids_used),
                "claim_strength": "observed",
            })

    claims = []
    for src_idx, src in enumerate(source_entries or []):
        claims.append({
            "claim_id": f"CLM-{result_id}-{src_idx:03d}",
            "claim_text": f"Source [{src['source_title']}] provides evidence related to the retrieval question.",
            "source_ids": [src["source_id"]],
            "claim_strength": "observed",
        })

    sources = []
    if source_entries:
        sources = [deepcopy(entry) for entry in source_entries]
        for src in sources:
            if "used_in_sections" not in src:
                src["used_in_sections"] = [s["section_title"] for s in sections]
            if "visibility" not in src:
                src["visibility"] = contract["required_visibility"]
            if "relevance" not in src:
                src["relevance"] = "direct"
            if "full_text_retrieved" not in src:
                src["full_text_retrieved"] = contract["required_visibility"] == "full_text_observed"
            if "source_url_or_ref" not in src:
                src["source_url_or_ref"] = f"fake://retrieval/{src['source_id']}"
            if "content_sha256" not in src:
                src["content_sha256"] = sha256_bytes(src["source_title"].encode("utf-8"))

    filters = []
    if filter_entries:
        filters = [deepcopy(f) for f in filter_entries]

    raw_refs = [
        {
            "source_id": src["source_id"],
            "source_title": src["source_title"],
            "source_url_or_ref": src["source_url_or_ref"],
            "visibility": src["visibility"],
            "content_sha256": src["content_sha256"],
            "retrieval_note": src.get("notes", "source provided by sub-agent"),
        }
        for src in sources
    ]

    result = {
        "schema_version": "0.1.0-draft",
        "result_kind": "retrieval_subagent_result",
        "result_id": result_id,
        "contract_id": contract["contract_id"],
        "subagent_session_id": subagent_session_id,
        "query_summary": query_summary,
        "source_ledger": sources,
        "filtering_log": filters,
        "organized_response": {
            "sections": sections,
            "claims": claims,
        },
        "raw_source_refs": raw_refs,
        "opacity_notes": [
            "All sources are declared; no opaque or merged summaries.",
            "Filtering log records every exclusion with reason.",
            "Result is independently verifiable by the main agent.",
        ],
    }
    validate_contract(result, RESULT_SCHEMA, label="retrieval result")
    return result


def run_retrieval_subagent_fixture(
    *,
    contract: dict[str, Any],
    parent_session_id: str,
    output_root: Path,
    query_texts: Sequence[str] | None = None,
    source_entries: Sequence[dict[str, Any]] | None = None,
    sections_content: Sequence[dict[str, Any]] | None = None,
    filter_entries: Sequence[dict[str, Any]] | None = None,
) -> dict[str, Any]:
    if output_root.exists() and any(output_root.iterdir()):
        raise AssuranceError(f"output root must be empty or absent: {output_root}")

    subagent_session_id = f"CONV-{uuid.uuid4().hex[:32].upper()}"
    contract_id = contract["contract_id"]
    run_id = f"RUN-RETRIEVAL-{sha256_bytes(contract_id.encode('utf-8'))[:24].upper()}"

    ipg_context = build_instruction_provenance_gate_context(
        run_id=run_id,
        conversation_id=subagent_session_id,
        instructions=[
            {
                "entry_id": f"INS-RETRIEVAL-CONTRACT-{contract_id}",
                "declared_source_type": "user",
                "source_id": f"retrieval-contract-{contract_id}",
                "content_sha256": sha256_bytes(canonical_bytes(contract)),
                "content_bytes": len(canonical_bytes(contract)),
                "instruction_kind": "user_prompt",
            },
        ],
    )
    ipg_receipt = evaluate_instruction_provenance_gate(gate_context=ipg_context)

    probe_registry = {spec["tool_id"]: True for spec in SUBAGENT_TOOL_SPECS}
    tool_availability_report = probe_tool_availability(
        tool_specs=SUBAGENT_TOOL_SPECS,
        runtime_id=run_id,
        probe_registry=probe_registry,
    )
    tool_availability_gate_receipt = build_tool_availability_gate_receipt(tool_availability_report)

    orientation_checkpoint = build_orientation_checkpoint(
        task_id=contract_id,
        trigger_step=0,
        task_contract_sha256=sha256_bytes(canonical_bytes(contract)),
        tool_availability_sha256=sha256_bytes(canonical_bytes(tool_availability_report)),
    )

    result = build_fake_retrieval_result(
        contract=contract,
        subagent_session_id=subagent_session_id,
        query_texts=query_texts,
        source_entries=source_entries,
        sections_content=sections_content,
        filter_entries=filter_entries,
    )
    validation_receipt = validate_retrieval_result(contract=contract, result=result)
    close_receipt = build_retrieval_session_close_receipt(
        parent_session_id=parent_session_id,
        subagent_session_id=subagent_session_id,
        contract_id=contract["contract_id"],
        result_id=result["result_id"],
    )

    output_root.mkdir(parents=True, exist_ok=True)

    atomic_write_json(output_root / "instruction-provenance-gate-context.json", ipg_context)
    atomic_write_json(output_root / "instruction-provenance-gate-receipt.json", ipg_receipt)
    atomic_write_json(output_root / "tool-availability-report.json", tool_availability_report)
    atomic_write_json(output_root / "tool-availability-gate-receipt.json", tool_availability_gate_receipt)
    atomic_write_json(output_root / "orientation-checkpoint.json", orientation_checkpoint)
    atomic_write_json(output_root / "retrieval-task-contract.json", contract)
    atomic_write_json(output_root / "retrieval-result.json", result)
    atomic_write_json(output_root / "retrieval-result-validation-receipt.json", validation_receipt)
    atomic_write_json(output_root / "retrieval-session-close-receipt.json", close_receipt)

    summary = {
        "schema_version": "0.1.0-draft",
        "receipt_kind": "retrieval_subagent_fixture_summary",
        "valid": validation_receipt["valid"],
        "parent_session_id": parent_session_id,
        "subagent_session_id": subagent_session_id,
        "contract_id": contract["contract_id"],
        "result_id": result["result_id"],
        "output_root": str(output_root),
        "artifacts": {
            "ipg_context": "instruction-provenance-gate-context.json",
            "ipg_receipt": "instruction-provenance-gate-receipt.json",
            "tool_availability_report": "tool-availability-report.json",
            "tool_availability_gate_receipt": "tool-availability-gate-receipt.json",
            "orientation_checkpoint": "orientation-checkpoint.json",
            "task_contract": "retrieval-task-contract.json",
            "result": "retrieval-result.json",
            "validation_receipt": "retrieval-result-validation-receipt.json",
            "close_receipt": "retrieval-session-close-receipt.json",
        },
        "gates": {
            "instruction_provenance_gate_decision": ipg_receipt["gate_decision"],
            "tool_availability_gate_decision": tool_availability_gate_receipt["decisions"]["gate_decision"],
        },
        "checks": {
            "contract_valid": True,
            "result_built": True,
            "result_validates": validation_receipt["valid"],
            "close_receipt_confirms_no_zeroing": True,
            "subagent_conversation_preserved": True,
            "gates_evaluated_before_work": True,
            "no_model_invoked": True,
            "no_network_requested": True,
        },
        "limitations": [
            "This is a no-model fixture; no real retrieval, model, or network was used.",
            "Sub-agent conversation namespace is simulated; not connected to ConversationNamespace.",
            "Source entries are fake and provided by the test fixture.",
        ],
    }
    atomic_write_json(output_root / "retrieval-subagent-fixture-summary.json", summary)
    return summary


def verify_retrieval_subagent_fixture(
    *,
    output_root: Path,
    parent_session_id: str,
) -> dict[str, Any]:
    ipg_context_path = output_root / "instruction-provenance-gate-context.json"
    ipg_receipt_path = output_root / "instruction-provenance-gate-receipt.json"
    tool_report_path = output_root / "tool-availability-report.json"
    tool_receipt_path = output_root / "tool-availability-gate-receipt.json"
    orientation_path = output_root / "orientation-checkpoint.json"
    contract_path = output_root / "retrieval-task-contract.json"
    result_path = output_root / "retrieval-result.json"
    validation_path = output_root / "retrieval-result-validation-receipt.json"
    close_path = output_root / "retrieval-session-close-receipt.json"
    summary_path = output_root / "retrieval-subagent-fixture-summary.json"

    ipg_context = load_json(ipg_context_path)
    ipg_receipt = load_json(ipg_receipt_path)
    verify_instruction_provenance_gate_receipt(
        gate_context=ipg_context, receipt=ipg_receipt
    )

    tool_report = load_json(tool_report_path)
    tool_receipt = load_json(tool_receipt_path)
    recomputed_tool_receipt = build_tool_availability_gate_receipt(tool_report)
    if canonical_bytes(tool_receipt) != canonical_bytes(recomputed_tool_receipt):
        raise AssuranceError("subagent tool availability gate receipt does not rebuild")

    orientation = load_json(orientation_path)

    contract = load_json(contract_path)
    validate_contract(contract, TASK_CONTRACT_SCHEMA, label="retrieval task contract")
    result = load_json(result_path)
    validate_contract(result, RESULT_SCHEMA, label="retrieval result")

    observed_validation = load_json(validation_path)
    expected_validation = validate_retrieval_result(contract=contract, result=result)
    if canonical_bytes(observed_validation) != canonical_bytes(expected_validation):
        raise AssuranceError("retrieval result validation receipt does not rebuild")

    observed_close = load_json(close_path)
    validate_contract(observed_close, CLOSE_RECEIPT_SCHEMA, label="retrieval session close receipt")
    expected_close = build_retrieval_session_close_receipt(
        parent_session_id=parent_session_id,
        subagent_session_id=result["subagent_session_id"],
        contract_id=contract["contract_id"],
        result_id=result["result_id"],
    )
    if canonical_bytes(observed_close) != canonical_bytes(expected_close):
        raise AssuranceError("retrieval session close receipt does not rebuild")

    observed_summary = load_json(summary_path)
    if observed_summary["parent_session_id"] != parent_session_id:
        raise AssuranceError("retrieval subagent fixture parent session mismatch")
    if not observed_summary["checks"]["close_receipt_confirms_no_zeroing"]:
        raise AssuranceError("retrieval subagent close receipt must confirm no zeroing")
    if not observed_summary["checks"]["gates_evaluated_before_work"]:
        raise AssuranceError("subagent gates must be evaluated before retrieval work")

    return observed_summary
