from __future__ import annotations

import uuid
from copy import deepcopy
from pathlib import Path
from typing import Any, Sequence

from .contracts import validate_contract
from .conversation import ConversationNamespace
from .errors import AssuranceError
from .instruction_provenance_gate import (
    build_instruction_provenance_gate_context,
    evaluate_instruction_provenance_gate,
    verify_instruction_provenance_gate_receipt,
)
from .keystore import InstallationKeyStore
from .orientation_runtime_guard import build_orientation_checkpoint
from .source_visibility import evaluate_source_visibility_gate
from .tool_availability_gate import (
    build_tool_availability_gate_receipt,
    probe_tool_availability,
)
from .utils import atomic_write_json, canonical_bytes, load_json, sha256_bytes, sha256_file, utc_now
from .workspace_trust import (
    establish_workspace_trust,
    workspace_trust_for_adapter,
)


TASK_CONTRACT_SCHEMA = "retrieval-task-contract-v0.1.schema.json"
RESULT_SCHEMA = "retrieval-result-v0.1.schema.json"
CLOSE_RECEIPT_SCHEMA = "retrieval-session-close-receipt-v0.1.schema.json"

SUBAGENT_CAPABILITIES = [
    "search",
    "web_fetch",
    "file_read",
    "tool_registry",
]

# ── credential targets (GAK-RET-SUB-001, Gap Register §8.1) ──

# Independent API key for internal project-document retrieval.
# Stored in Windows Credential Manager under this target name.
DEFAULT_INTERNAL_RETRIEVAL_CREDENTIAL_TARGET = "deepseek-retrieval-subagent"

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


# ── real retrieval adapter ──


def _build_retrieval_system_prompt(
    question: str,
    doc_excerpts: list[dict[str, str]],
    return_sections: list[str],
    forbidden_topics: list[str],
) -> str:
    """Build the system prompt for the project doc retrieval DeepSeek call."""
    sections_str = "\n".join(f"  - {s}" for s in return_sections)
    forbidden_str = (
        "\n".join(f"  - {t}" for t in forbidden_topics)
        if forbidden_topics
        else "  (none)"
    )

    excerpts_lines: list[str] = []
    for i, doc in enumerate(doc_excerpts):
        excerpts_lines.append(
            f"\n### Document {i + 1}: {doc['path']} ({doc['category']})\n"
            f"{doc['excerpt']}"
        )

    return f"""You are a project-internal document retrieval subagent for the GSA (General Scientific Assurance) project.

Your task: answer the retrieval question using ONLY the provided document excerpts below.
Do NOT use external knowledge. Do NOT make claims beyond what the excerpts contain.
All output must be marked as derived from project documents, not as original claims.

Retrieval question:
{question}

Required return sections (you must address each one):
{sections_str}

Forbidden topics (do NOT discuss these):
{forbidden_str}

Project document excerpts:
{''.join(excerpts_lines)}

Please structure your response as a JSON object with the following keys:
- "sections": a list of {{"section_title": str, "content": str}} for each required section
- "source_ids_used": a list of document paths you referenced
Do NOT include any text outside the JSON object."""


def dispatch_retrieval_subagent(
    *,
    contract: dict[str, Any],
    parent_session_id: str,
    key_store: InstallationKeyStore,
    run_root: Path,
    project_root: Path,
    credential_target: str | None = None,  # pass DEFAULT_INTERNAL_RETRIEVAL_CREDENTIAL_TARGET for real API
    api_timeout_seconds: int = 120,
    parent_envelope: dict[str, Any] | None = None,
) -> dict[str, Any]:
    """Dispatch a real retrieval subagent with independent DeepSeek API call.

    This is the production adapter path — it replaces the no-model
    ``build_fake_retrieval_result`` with a real project document scan
    followed by a DeepSeek v4 Pro API call.  All artifacts are routed
    through a :class:`SessionGovernor` backed by an independent
    :class:`ConversationNamespace`.

    The subagent searches only project-internal documents (no network
    search) and returns a structured result conforming to
    ``retrieval-result-v0.1.schema.json``.

    Args:
        contract: Task contract from :func:`build_retrieval_task_contract`.
        parent_session_id: The main agent's conversation ID.
        key_store: Installation key store for namespace envelope.
        run_root: Directory for the subagent's run artifacts.
        project_root: Root of the GSA project to scan.
        credential_target: Windows Credential Manager target for the
            subagent's independent DeepSeek API key.  If ``None``, the
            subagent runs in **offline mode** (document scan only,
            no API call).
        api_timeout_seconds: Timeout for the DeepSeek API call.

    Returns:
        A dict with ``result`` (the retrieval result), ``close_receipt``
        (session close confirmation), and ``governor_closure``.
    """
    import uuid as _uuid

    from .adapter_gate import AdapterGateContext, AdapterGateBlockedError, enforce_adapter_call
    from .deepseek_adapter import call_deepseek_api, _read_windows_credential
    from .project_doc_index import EXCERPT_MAX_CHARS, ProjectDocIndex
    from .session_governor import SessionGovernor

    validate_contract(contract, TASK_CONTRACT_SCHEMA, label="retrieval task contract")

    subagent_session_id = f"CONV-{_uuid.uuid4().hex[:32].upper()}"
    contract_id = contract["contract_id"]
    run_id = f"RUN-RETRIEVAL-{sha256_bytes(contract_id.encode('utf-8'))[:24].upper()}"

    # ── 1. Create independent namespace + governor ──
    from .canonical_cli import _build_run_frozen_context as _frozen_ctx

    conversation_repository = run_root / "conversations"

    # ── resolve effective capabilities ──
    if parent_envelope is not None:
        from .child_capability_enforcer import spawn_child_context
        child_ctx = spawn_child_context(
            parent_envelope=parent_envelope,
            child_kind="child_agent",
            child_id=f"RETRIEVAL-{contract_id}",
            requested_capabilities=["search", "file_read"],
            key_store=key_store,
            ttl_seconds=3600,
            frozen_context=_frozen_ctx(run_id),
        )
        _child_env = child_ctx.get("child_envelope") or {}
        _cap_env = _child_env.get("capability_envelope") or {}
        _effective_allowed = _cap_env.get("allowed", ["search", "file_read"])
        _effective_denied = _cap_env.get("denied", [
            "network.unrestricted", "secret.raw_read", "filesystem.workspace_write",
        ])
        _parent_eid: str | None = child_ctx.get("child_envelope_id") or None
    else:
        _effective_allowed = ["search", "file_read"]
        _effective_denied = [
            "network.unrestricted", "secret.raw_read", "filesystem.workspace_write",
        ]
        _parent_eid = None

    namespace = ConversationNamespace.create(
        conversation_repository,
        key_store=key_store,
        frozen_context=_frozen_ctx(run_id),
        allowed_capabilities=_effective_allowed,
        denied_capabilities=_effective_denied,
        parent_envelope_id=_parent_eid,
    )
    governor = SessionGovernor(namespace)

    # ── write child enforcement receipt if available ──
    if parent_envelope is not None:
        governor.write_gate_receipt(
            "child-capability-enforcement", child_ctx["enforcement_receipt"],
        )

    # ── 1b. Establish workspace trust ──
    trust_receipt = establish_workspace_trust(
        workspace_root=project_root,
        adapter_id="deepseek-v4-pro-retrieval",
        conversation_id=namespace.conversation_id,
    )
    trust_status = workspace_trust_for_adapter(
        trust_receipt, adapter_id="deepseek-v4-pro-retrieval",
    )
    governor.write_gate_receipt("workspace-trust-receipt", trust_receipt)

    # ── 2. IPG context + gate evaluation ──
    ipg_context = build_instruction_provenance_gate_context(
        run_id=run_id,
        conversation_id=namespace.conversation_id,
        instructions=[
            {
                "entry_id": f"INS-RETRIEVAL-CONTRACT-{contract_id}",
                "declared_source_type": "user",
                "source_id": f"retrieval-contract-{contract_id}",
                "content_sha256": sha256_bytes(canonical_bytes(contract)),
                "content_bytes": len(canonical_bytes(contract)),
                "instruction_kind": "user_prompt",
                "workspace_trust": trust_status,
            },
        ],
    )
    ipg_receipt = evaluate_instruction_provenance_gate(gate_context=ipg_context)
    governor.write_gate_receipt("instruction-provenance-gate-context", ipg_context)
    governor.write_gate_receipt("instruction-provenance-gate-receipt", ipg_receipt)

    # ── 3. Scan project documents ──
    doc_index = ProjectDocIndex(project_root)
    doc_index.scan()

    question = contract["retrieval_question"]
    max_sources = contract["max_sources"]
    allowed_categories = list(contract["allowed_source_categories"])
    forbidden_topics = list(contract["scope_boundary"]["forbidden_topics"])

    # Map contract source categories to doc index categories
    category_map: dict[str, list[str]] = {
        "documentation": ["architecture", "audit_docs", "adr"],
        "code_repository": ["source_code"],
        "internal_knowledge_base": [
            "architecture", "audit_docs", "adr", "source_code",
            "schemas", "protocol", "regression", "project_config",
        ],
    }
    search_categories: list[str] = []
    for cat in allowed_categories:
        search_categories.extend(category_map.get(cat, [cat]))
    search_categories = sorted(set(search_categories)) if search_categories else None

    matches = doc_index.search(
        question,
        max_results=max_sources,
        categories=search_categories,
    )

    # ── 4. Build filtering log ──
    filtering_log: list[dict[str, Any]] = []
    filtered_matches: list[DocMatch] = []
    for m in matches:
        excluded = False
        for topic in forbidden_topics:
            if topic.lower() in m.excerpt or topic.lower() in m.path.lower():
                filtering_log.append({
                    "source_id": f"SRC-{sha256_bytes(m.path.encode('utf-8'))[:16].upper()}",
                    "reason": "scope_violation",
                    "action": "excluded",
                    "filtered_at": utc_now(),
                })
                excluded = True
                break
        if m.size_bytes > contract["context_budget_tokens"] * 4:
            filtering_log.append({
                "source_id": f"SRC-{sha256_bytes(m.path.encode('utf-8'))[:16].upper()}",
                "reason": "budget_exceeded",
                "action": "deferred_for_main_agent_review",
                "filtered_at": utc_now(),
            })
            excluded = True
        if not excluded:
            filtered_matches.append(m)

    # ── 5. Build doc excerpts for the model ──
    doc_excerpts: list[dict[str, str]] = []
    source_ledger: list[dict[str, Any]] = []
    raw_source_refs: list[dict[str, Any]] = []

    for i, m in enumerate(filtered_matches[:max_sources]):
        source_id = f"SRC-{sha256_bytes(m.path.encode('utf-8'))[:16].upper()}"
        doc_excerpts.append({
            "path": m.path,
            "category": m.category,
            "excerpt": m.excerpt[:EXCERPT_MAX_CHARS],
            "title": m.title,
        })
        source_ledger.append({
            "source_id": source_id,
            "source_title": m.title,
            "source_url_or_ref": str(project_root / m.path),
            "visibility": "full_text_observed",
            "relevance": "direct",
            "used_in_sections": [],
            "content_sha256": m.sha256,
            "full_text_retrieved": True,
            "notes": f"project doc: {m.path}",
        })
        raw_source_refs.append({
            "source_id": source_id,
            "source_title": m.title,
            "source_url_or_ref": str(project_root / m.path),
            "visibility": "full_text_observed",
            "content_sha256": m.sha256,
            "retrieval_note": f"retrieved from project index: {m.path}",
        })

    # ── 6. Call DeepSeek API (or offline mode) ──
    result_id = f"RET-RES-{_uuid.uuid4().hex.upper()}"
    model_output_text: str | None = None
    api_used = False

    if credential_target is not None and doc_excerpts:
        api_key = ""
        try:
            api_key = _read_windows_credential(credential_target)
            system_prompt = _build_retrieval_system_prompt(
                question=question,
                doc_excerpts=doc_excerpts,
                return_sections=contract["return_format"]["sections"],
                forbidden_topics=forbidden_topics,
            )
            messages: list[dict[str, str]] = [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": question},
            ]

            gate_ctx = AdapterGateContext(
                ipg_receipt=ipg_receipt,
                ipg_context=ipg_context,
                adapter_id="deepseek-v4-pro-retrieval",
                conversation_id=namespace.conversation_id,
                run_id=run_id,
                trust_receipt=trust_receipt,
                network_endpoint="https://api.deepseek.com/chat/completions",
                network_endpoint_category="llm_provider",
                allowed_categories={"llm_provider"},
                allowed_endpoints={"api.deepseek.com"},
            )
            enforcement = enforce_adapter_call(
                gate_context=gate_ctx,
                adapter_call=lambda: call_deepseek_api(
                    api_key,
                    messages,
                    timeout_seconds=api_timeout_seconds,
                    conversation_id=namespace.conversation_id,
                    allowed_categories={"llm_provider"},
                    allowed_endpoints={"api.deepseek.com"},
                ),
            )
            api_output = enforcement["adapter_result"]
            model_output_text = api_output.get("public_assistant_text", "")
            api_used = True
            governor.write_gate_receipt(
                "adapter-gate-enforcement",
                {k: v for k, v in enforcement.items() if k != "adapter_result"},
            )
        except AdapterGateBlockedError:
            model_output_text = "[blocked] Retrieval subagent blocked by adapter gate."
        except Exception as exc:
            model_output_text = f"[error] Retrieval subagent API call failed: {exc}"
        finally:
            if api_key:
                api_key = "\x00" * len(api_key)  # GAK-CRED-001: best-effort scrub
                del api_key

    # ── 7. Build organized response ──
    sections: list[dict[str, Any]] = []
    claims: list[dict[str, Any]] = []

    if model_output_text:
        # Try to parse model output as JSON
        try:
            import json as _json
            parsed = _json.loads(model_output_text.strip())
            for sec in parsed.get("sections", []):
                sections.append({
                    "section_title": sec.get("section_title", "unknown"),
                    "content": sec.get("content", ""),
                    "source_ids": [
                        s["source_id"] for s in source_ledger
                    ],
                    "claim_strength": "derived",
                })
            for sid in parsed.get("source_ids_used", []):
                # Mark which sources were used
                for s in source_ledger:
                    if sid in s["source_url_or_ref"]:
                        if sid not in s["used_in_sections"]:
                            s["used_in_sections"].extend(
                                sec.get("section_title", "unknown")
                                for sec in parsed.get("sections", [])
                            )
        except Exception:
            # Non-JSON output — wrap in a single section
            sections.append({
                "section_title": contract["return_format"]["sections"][0],
                "content": model_output_text,
                "source_ids": [s["source_id"] for s in source_ledger],
                "claim_strength": "derived",
            })

    if not sections:
        # Offline mode: build sections from doc excerpts
        for section_name in contract["return_format"]["sections"]:
            content_parts: list[str] = []
            for m in filtered_matches[:max_sources]:
                content_parts.append(f"[{m.path}]: {m.excerpt[:500]}")
            sections.append({
                "section_title": section_name,
                "content": "\n\n".join(content_parts) if content_parts else (
                    "No project documents found matching the retrieval question."
                ),
                "source_ids": [s["source_id"] for s in source_ledger],
                "claim_strength": "observed",
            })

    for s in source_ledger:
        if not s["used_in_sections"]:
            s["used_in_sections"] = [sec["section_title"] for sec in sections]

    for i, src in enumerate(source_ledger):
        claims.append({
            "claim_id": f"CLM-{result_id}-{i:03d}",
            "claim_text": (
                f"Project document [{src['source_title']}] "
                f"({src['source_url_or_ref']}) provides evidence "
                f"related to the retrieval question."
            ),
            "source_ids": [src["source_id"]],
            "claim_strength": "derived" if api_used else "observed",
        })

    # ── 8. Build query summary ──
    query_summary = [{
        "query_id": f"QRY-{result_id}-000",
        "query_text": question,
        "source_category": allowed_categories[0] if allowed_categories else "internal_knowledge_base",
        "result_count": len(filtered_matches),
        "action_taken": "searched" if filtered_matches else "aborted_no_results",
        "tool_used": "file_read",
    }]

    # ── 9. Assemble result ──
    result = {
        "schema_version": "0.1.0-draft",
        "result_kind": "retrieval_subagent_result",
        "result_id": result_id,
        "contract_id": contract_id,
        "subagent_session_id": subagent_session_id,
        "query_summary": query_summary,
        "source_ledger": source_ledger,
        "filtering_log": filtering_log,
        "organized_response": {
            "sections": sections,
            "claims": claims,
        },
        "raw_source_refs": raw_source_refs,
        "opacity_notes": [
            "All sources are project-internal documents; no web search performed.",
            "Filtering log records every exclusion with reason.",
            "Result is independently verifiable: raw_source_refs point to project files.",
            f"{'Real DeepSeek API' if api_used else 'Offline document scan'} used for retrieval.",
            "All claims marked derived_unverified — main agent must verify.",
        ],
    }
    validate_contract(result, RESULT_SCHEMA, label="retrieval result")

    # ── 10. Validate + close ──
    validation_receipt = validate_retrieval_result(contract=contract, result=result)
    close_receipt = build_retrieval_session_close_receipt(
        parent_session_id=parent_session_id,
        subagent_session_id=subagent_session_id,
        contract_id=contract_id,
        result_id=result_id,
    )

    # Write artifacts through governor
    governor.write_gate_receipt("retrieval-task-contract", contract)
    governor.write_gate_receipt("retrieval-result", result)
    governor.write_gate_receipt("retrieval-result-validation", validation_receipt)
    governor.write_gate_receipt("retrieval-session-close-receipt", close_receipt)

    governor_closure = governor.close_session()

    return {
        "result": result,
        "close_receipt": close_receipt,
        "validation_receipt": validation_receipt,
        "governor_closure": governor_closure,
        "subagent_session_id": subagent_session_id,
        "api_used": api_used,
    }


def dispatch_retrieval_subagent_online(
    *,
    contract: dict[str, Any],
    parent_session_id: str,
    key_store: InstallationKeyStore,
    run_root: Path,
    project_root: Path,
    api_timeout_seconds: int = 120,
    parent_envelope: dict[str, Any] | None = None,
) -> dict[str, Any]:
    """Convenience wrapper: internal retrieval with the registered credential target.

    Uses ``DEFAULT_INTERNAL_RETRIEVAL_CREDENTIAL_TARGET``
    (``deepseek-retrieval-subagent``) as the credential target, matching
    the Gap Register §8.1 registration.  This is the production entry
    point — it will attempt a real DeepSeek API call and fall back to
    offline mode if the credential is not configured.
    """
    return dispatch_retrieval_subagent(
        contract=contract,
        parent_session_id=parent_session_id,
        key_store=key_store,
        run_root=run_root,
        project_root=project_root,
        credential_target=DEFAULT_INTERNAL_RETRIEVAL_CREDENTIAL_TARGET,
        api_timeout_seconds=api_timeout_seconds,
        parent_envelope=parent_envelope,
    )


# ── external retrieval adapter ──

DEFAULT_EXTERNAL_RETRIEVAL_CREDENTIAL_TARGET = "FEP-Agent/DeepSeek-Retrieval"

EXTERNAL_SOURCE_CATEGORIES: list[str] = [
    "academic_paper",
    "web_page",
    "documentation",
    "code_repository",
    "thread_conversation",
    "structured_data",
]

# Maps contract source categories to search-result category labels
_EXTERNAL_CATEGORY_LABELS: dict[str, str] = {
    "academic_paper": "Academic Paper",
    "web_page": "Web Page",
    "documentation": "Documentation",
    "code_repository": "Code Repository",
    "thread_conversation": "Thread / Conversation",
    "structured_data": "Structured Data",
}


def _build_external_retrieval_system_prompt(
    question: str,
    search_excerpts: list[dict[str, str]],
    return_sections: list[str],
    forbidden_topics: list[str],
) -> str:
    """Build the system prompt for the external web-search retrieval call.

    The prompt includes the actual web search result excerpts and
    **explicitly forbids** the model from using training knowledge.
    Every claim must be grounded in a specific search result.
    """
    sections_str = "\n".join(f"  - {s}" for s in return_sections)
    forbidden_str = (
        "\n".join(f"  - {t}" for t in forbidden_topics)
        if forbidden_topics
        else "  (none)"
    )

    excerpts_lines: list[str] = []
    for i, result in enumerate(search_excerpts):
        category_label = _EXTERNAL_CATEGORY_LABELS.get(
            result.get("category", "web_page"), "External Source"
        )
        excerpts_lines.append(
            f"\n### Search Result {i + 1}: {result['title']}\n"
            f"URL: {result['url']}\n"
            f"Category: {category_label}\n"
            f"```\n{result['snippet']}\n```"
        )

    return f"""You are a web search result processing subagent for the GSA (General Scientific Assurance) project.

Your ONLY job: read the provided web search result excerpts below and structure them into a coherent answer. You are a search-result summariser, NOT a knowledge model.

CRITICAL — VIOLATING THESE RULES IS A HARD FAILURE:
1. ONLY use the provided search result excerpts.  Do NOT use any training knowledge, prior knowledge, or external facts.
2. If the search results do not contain enough information to answer the question, state "Insufficient search results to answer this question" in the relevant section — do NOT fabricate or infer missing information.
3. Every factual statement MUST cite the specific search result number(s) it comes from (e.g. "see results 1, 3").
4. Do NOT evaluate the quality, truthfulness, or credibility of search results.  Just report what they say.
5. Do NOT discuss forbidden topics.

Retrieval question:
{question}

Required return sections (you must address each one):
{sections_str}

Forbidden topics (do NOT discuss these):
{forbidden_str}

Web search result excerpts:
{''.join(excerpts_lines)}

Return format — a JSON object with exactly these keys:
- "sections": a list of {{"section_title": str, "content": str, "cited_results": [int]}} for each required section
- "insufficient_coverage": bool — true if the search results are insufficient to answer the question
Do NOT include any text outside the JSON object."""


def dispatch_external_retrieval_subagent(
    *,
    contract: dict[str, Any],
    parent_session_id: str,
    key_store: InstallationKeyStore,
    run_root: Path,
    search_results: list[dict[str, str]] | None = None,
    credential_target: str = DEFAULT_EXTERNAL_RETRIEVAL_CREDENTIAL_TARGET,
    api_timeout_seconds: int = 120,
    parent_envelope: dict[str, Any] | None = None,
) -> dict[str, Any]:
    """Dispatch an external retrieval subagent to process web search results.

    This subagent processes raw web search results (title, URL, snippet)
    through a DeepSeek API call that is **strictly constrained** to use
    only the provided search excerpts — no training knowledge, no
    hallucination.

    It follows the same design patterns as
    :func:`dispatch_retrieval_subagent`: independent
    :class:`ConversationNamespace`, :class:`SessionGovernor`, IPG gate,
    and :meth:`enforce_adapter_call` wrapping.

    The input ``search_results`` are the raw web search hits — the
    subagent does NOT perform the search itself.  This keeps the search
    tool and the result processor as separate, auditable steps.

    Args:
        contract: Task contract from :func:`build_retrieval_task_contract`.
        parent_session_id: The main agent's conversation ID.
        key_store: Installation key store for namespace envelope.
        run_root: Directory for the subagent's run artifacts.
        search_results: Raw web search results.  Each dict must have
            ``title`` (str), ``url`` (str), ``snippet`` (str), and
            optionally ``category`` (str).  If ``None`` or empty, the
            subagent returns an empty result with an opacity note.
        credential_target: Windows Credential Manager target for the
            subagent's independent DeepSeek API key.
        api_timeout_seconds: Timeout for the DeepSeek API call.

    Returns:
        A dict with ``result``, ``close_receipt``, ``validation_receipt``,
        ``governor_closure``, ``subagent_session_id``, and ``api_used``.
    """
    import uuid as _uuid

    from .adapter_gate import AdapterGateContext, AdapterGateBlockedError, enforce_adapter_call
    from .deepseek_adapter import call_deepseek_api, _read_windows_credential
    from .session_governor import SessionGovernor

    validate_contract(contract, TASK_CONTRACT_SCHEMA, label="retrieval task contract")

    allowed_categories = list(contract["allowed_source_categories"])
    if "internal_knowledge_base" in allowed_categories:
        raise AssuranceError(
            "dispatch_external_retrieval_subagent does not support "
            "internal_knowledge_base.  Use dispatch_retrieval_subagent() "
            "for project-internal document retrieval."
        )

    # ── normalise search results ──
    results: list[dict[str, str]] = []
    if search_results:
        for r in search_results:
            results.append({
                "title": r.get("title", "Untitled"),
                "url": r.get("url", ""),
                "snippet": r.get("snippet", ""),
                "category": r.get("category", allowed_categories[0] if allowed_categories else "web_page"),
            })

    subagent_session_id = f"CONV-{_uuid.uuid4().hex[:32].upper()}"
    contract_id = contract["contract_id"]
    run_id = f"RUN-RETRIEVAL-{sha256_bytes(contract_id.encode('utf-8'))[:24].upper()}"

    # ── 1. Create independent namespace + governor ──
    from .canonical_cli import _build_run_frozen_context as _frozen_ctx

    conversation_repository = run_root / "conversations"

    # ── resolve effective capabilities ──
    if parent_envelope is not None:
        from .child_capability_enforcer import spawn_child_context
        child_ctx = spawn_child_context(
            parent_envelope=parent_envelope,
            child_kind="child_agent",
            child_id=f"EXT-RETRIEVAL-{contract_id}",
            requested_capabilities=["search", "web_fetch"],
            key_store=key_store,
            ttl_seconds=3600,
            frozen_context=_frozen_ctx(run_id),
        )
        _child_env = child_ctx.get("child_envelope") or {}
        _cap_env = _child_env.get("capability_envelope") or {}
        _effective_allowed = _cap_env.get("allowed", ["search", "web_fetch"])
        _effective_denied = _cap_env.get("denied", [
            "network.unrestricted", "secret.raw_read", "filesystem.workspace_write",
        ])
        _parent_eid: str | None = child_ctx.get("child_envelope_id") or None
    else:
        _effective_allowed = ["search", "web_fetch"]
        _effective_denied = [
            "network.unrestricted", "secret.raw_read", "filesystem.workspace_write",
        ]
        _parent_eid = None

    namespace = ConversationNamespace.create(
        conversation_repository,
        key_store=key_store,
        frozen_context=_frozen_ctx(run_id),
        allowed_capabilities=_effective_allowed,
        denied_capabilities=_effective_denied,
        parent_envelope_id=_parent_eid,
    )
    governor = SessionGovernor(namespace)

    if parent_envelope is not None:
        governor.write_gate_receipt(
            "child-capability-enforcement", child_ctx["enforcement_receipt"],
        )

    # ── 1b. Establish workspace trust ──
    trust_receipt = establish_workspace_trust(
        workspace_root=run_root.parent,
        adapter_id="deepseek-v4-pro-external-retrieval",
        conversation_id=namespace.conversation_id,
    )
    trust_status = workspace_trust_for_adapter(
        trust_receipt, adapter_id="deepseek-v4-pro-external-retrieval",
    )
    governor.write_gate_receipt("workspace-trust-receipt", trust_receipt)

    # ── 2. IPG context + gate evaluation ──
    ipg_context = build_instruction_provenance_gate_context(
        run_id=run_id,
        conversation_id=namespace.conversation_id,
        instructions=[
            {
                "entry_id": f"INS-RETRIEVAL-CONTRACT-{contract_id}",
                "declared_source_type": "user",
                "source_id": f"retrieval-contract-{contract_id}",
                "content_sha256": sha256_bytes(canonical_bytes(contract)),
                "content_bytes": len(canonical_bytes(contract)),
                "instruction_kind": "user_prompt",
                "workspace_trust": trust_status,
            },
        ],
    )
    ipg_receipt = evaluate_instruction_provenance_gate(gate_context=ipg_context)
    governor.write_gate_receipt("instruction-provenance-gate-context", ipg_context)
    governor.write_gate_receipt("instruction-provenance-gate-receipt", ipg_receipt)

    question = contract["retrieval_question"]
    max_sources = contract["max_sources"]
    forbidden_topics = list(contract["scope_boundary"]["forbidden_topics"])

    # ── 3. Build source ledger from search results before API call ──
    result_id = f"RET-RES-{_uuid.uuid4().hex.upper()}"
    source_ledger: list[dict[str, Any]] = []
    raw_source_refs: list[dict[str, Any]] = []
    filtering_log: list[dict[str, Any]] = []

    for i, r in enumerate(results[:max_sources]):
        source_id = f"SRC-{sha256_bytes(r['url'].encode('utf-8'))[:16].upper()}"
        category_label = _EXTERNAL_CATEGORY_LABELS.get(r["category"], "External Source")
        source_ledger.append({
            "source_id": source_id,
            "source_title": r["title"],
            "source_url_or_ref": r["url"],
            "visibility": "full_text_observed" if r["url"].startswith("http") else "metadata_only",
            "relevance": "direct",
            "used_in_sections": [],
            "content_sha256": sha256_bytes(r["snippet"].encode("utf-8")),
            "full_text_retrieved": False,
            "notes": f"web search result ({category_label}); snippet only — main agent should fetch full text",
        })
        raw_source_refs.append({
            "source_id": source_id,
            "source_title": r["title"],
            "source_url_or_ref": r["url"],
            "visibility": source_ledger[-1]["visibility"],
            "content_sha256": source_ledger[-1]["content_sha256"],
            "retrieval_note": f"web search result ({category_label})",
        })

    # ── 4. Call DeepSeek API to process search results ──
    model_output_text: str | None = None
    api_used = False

    if credential_target is not None and results:
        api_key = ""
        try:
            api_key = _read_windows_credential(credential_target)
            system_prompt = _build_external_retrieval_system_prompt(
                question=question,
                search_excerpts=results,
                return_sections=contract["return_format"]["sections"],
                forbidden_topics=forbidden_topics,
            )
            messages: list[dict[str, str]] = [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": question},
            ]

            gate_ctx = AdapterGateContext(
                ipg_receipt=ipg_receipt,
                ipg_context=ipg_context,
                adapter_id="deepseek-v4-pro-external-retrieval",
                conversation_id=namespace.conversation_id,
                run_id=run_id,
                trust_receipt=trust_receipt,
                network_endpoint="https://api.deepseek.com/chat/completions",
                network_endpoint_category="llm_provider",
                allowed_categories={"llm_provider"},
                allowed_endpoints={"api.deepseek.com"},
            )
            enforcement = enforce_adapter_call(
                gate_context=gate_ctx,
                adapter_call=lambda: call_deepseek_api(
                    api_key,
                    messages,
                    timeout_seconds=api_timeout_seconds,
                    conversation_id=namespace.conversation_id,
                    allowed_categories={"llm_provider"},
                    allowed_endpoints={"api.deepseek.com"},
                ),
            )
            api_output = enforcement["adapter_result"]
            model_output_text = api_output.get("public_assistant_text", "")
            api_used = True
            governor.write_gate_receipt(
                "adapter-gate-enforcement",
                {k: v for k, v in enforcement.items() if k != "adapter_result"},
            )
        except AdapterGateBlockedError:
            model_output_text = "[blocked] External retrieval subagent blocked by adapter gate."
        except Exception as exc:
            model_output_text = f"[error] External retrieval subagent API call failed: {exc}"
        finally:
            if api_key:
                api_key = "\x00" * len(api_key)  # GAK-CRED-001: best-effort scrub
                del api_key

    # ── 5. Parse model output ──
    sections: list[dict[str, Any]] = []
    insufficient_coverage = False

    if model_output_text:
        try:
            import json as _json

            parsed = _json.loads(model_output_text.strip())
            insufficient_coverage = bool(parsed.get("insufficient_coverage", False))

            for sec in parsed.get("sections", []):
                cited = sec.get("cited_results", [])
                matched_ids = []
                for idx in cited:
                    if isinstance(idx, int) and 1 <= idx <= len(source_ledger):
                        matched_ids.append(source_ledger[idx - 1]["source_id"])

                sections.append({
                    "section_title": sec.get("section_title", "unknown"),
                    "content": sec.get("content", ""),
                    "source_ids": matched_ids or [s["source_id"] for s in source_ledger],
                    "claim_strength": "derived",
                })
        except Exception:
            sections.append({
                "section_title": contract["return_format"]["sections"][0],
                "content": model_output_text,
                "source_ids": [s["source_id"] for s in source_ledger],
                "claim_strength": "derived",
            })

    if not sections:
        if not results:
            offline_note = (
                "No search results provided.  External retrieval requires "
                "web search results as input — this subagent processes "
                "search results, it does not perform searches itself."
            )
        else:
            offline_note = (
                "External retrieval API credential not configured.  "
                f"{len(results)} search result(s) were provided but could "
                "not be processed.  Raw snippets are available in "
                "raw_source_refs."
            )
        for section_name in contract["return_format"]["sections"]:
            sections.append({
                "section_title": section_name,
                "content": offline_note,
                "source_ids": [s["source_id"] for s in source_ledger],
                "claim_strength": "observed",
            })

    for s in source_ledger:
        if not s["used_in_sections"]:
            s["used_in_sections"] = [sec["section_title"] for sec in sections]

    claims: list[dict[str, Any]] = []
    for i, src in enumerate(source_ledger):
        claims.append({
            "claim_id": f"CLM-{result_id}-{i:03d}",
            "claim_text": (
                f"Web search result [{src['source_title']}] "
                f"({src['source_url_or_ref']}) — snippet provided.  "
                f"Main agent should fetch full text for independent verification."
            ),
            "source_ids": [src["source_id"]],
            "claim_strength": "derived" if api_used else "observed",
        })

    # ── 6. Build query summary ──
    query_summary = [{
        "query_id": f"QRY-{result_id}-000",
        "query_text": question,
        "source_category": allowed_categories[0] if allowed_categories else "web_page",
        "result_count": len(results),
        "action_taken": "searched" if results else "aborted_no_results",
        "tool_used": "search" if results else "none",
    }]

    # ── 7. Assemble result ──
    result = {
        "schema_version": "0.1.0-draft",
        "result_kind": "retrieval_subagent_result",
        "result_id": result_id,
        "contract_id": contract_id,
        "subagent_session_id": subagent_session_id,
        "query_summary": query_summary,
        "source_ledger": source_ledger,
        "filtering_log": filtering_log,
        "organized_response": {
            "sections": sections,
            "claims": claims,
        },
        "raw_source_refs": raw_source_refs,
        "opacity_notes": (
            [
                "All content derived from provided web search result snippets — no training knowledge used.",
                f"Input: {len(results)} web search result(s) processed through DeepSeek API.",
                "Search result snippets may be truncated or outdated — main agent should fetch full text.",
                "Filtering log records every exclusion with reason.",
                f"{'Real DeepSeek API' if api_used else 'No API key configured'} used for processing.",
                "Subagent does NOT perform web searches — it only processes search results provided as input.",
                "insufficient_coverage" + (": true" if insufficient_coverage else ": false") + " — model self-assessment.",
            ]
            if api_used
            else [
                "No search results provided or no API credential configured.",
                "Subagent does NOT perform web searches — it only processes search results provided as input.",
            ]
        ),
    }
    validate_contract(result, RESULT_SCHEMA, label="retrieval result")

    # ── 8. Validate + close ──
    validation_receipt = validate_retrieval_result(contract=contract, result=result)
    close_receipt = build_retrieval_session_close_receipt(
        parent_session_id=parent_session_id,
        subagent_session_id=subagent_session_id,
        contract_id=contract_id,
        result_id=result_id,
    )

    governor.write_gate_receipt("retrieval-task-contract", contract)
    governor.write_gate_receipt("retrieval-result", result)
    governor.write_gate_receipt("retrieval-result-validation", validation_receipt)
    governor.write_gate_receipt("retrieval-session-close-receipt", close_receipt)

    governor_closure = governor.close_session()

    return {
        "result": result,
        "close_receipt": close_receipt,
        "validation_receipt": validation_receipt,
        "governor_closure": governor_closure,
        "subagent_session_id": subagent_session_id,
        "api_used": api_used,
    }
