from __future__ import annotations

from pathlib import Path
from typing import Any

from .contracts import validate_contract
from .utils import load_json, sha256_file, utc_now


TASK_CONTRACT_SCHEMA = "task-contract-v0.1.schema.json"
DEFAULT_TASK_ID = "TASK-CANONICAL-CLI-FAKE-001"


def build_task_contract_from_ask(
    *,
    ask: str,
    source_ledger_path: Path,
    task_id: str = DEFAULT_TASK_ID,
    created_at: str | None = None,
) -> dict[str, Any]:
    resolved_source_ledger_path = source_ledger_path.resolve()
    contract = {
        "schema_version": "0.1.0-draft",
        "contract_kind": "canonical_cli_task_contract",
        "task_id": task_id,
        "created_at": created_at or utc_now(),
        "entry_mode": "ask",
        "user_request": {
            "raw_text": ask,
            "normalized_intent": ask.strip(),
        },
        "source_ledger": {
            "path": str(resolved_source_ledger_path),
            "sha256": sha256_file(resolved_source_ledger_path),
            "required": True,
        },
        "output_contract": {
            "format": "canonical_cli_answer_packet",
            "must_include_source_visibility_summary": True,
            "must_include_claim_boundaries": True,
            "must_include_next_actions": True,
        },
        "permissions": {
            "network_allowed": False,
            "real_model_allowed": False,
            "tool_calls_allowed": False,
            "workspace_writes_allowed": False,
            "incremental_retrieval_allowed": True,
        },
        "claim_policy": {
            "max_claim_strength": "observed_fragment_only",
            "fulltext_required_for_mechanism": True,
            "defer_on_insufficient_visibility": True,
        },
        "run_policy": {
            "adapter_mode": "fake_offline",
            "verify_after_run": True,
            "resume_allowed": False,
        },
        "notes": [
            "Generated from --ask for the current offline canonical CLI path.",
            "This freezes the run contract without constraining future user questions.",
        ],
    }
    validate_contract(contract, TASK_CONTRACT_SCHEMA, label="task contract")
    return contract


def load_task_contract(path: Path) -> dict[str, Any]:
    contract = load_json(path)
    validate_contract(contract, TASK_CONTRACT_SCHEMA, label="task contract")
    return contract


def task_contract_source_ledger_path(contract: dict[str, Any]) -> Path:
    return Path(contract["source_ledger"]["path"])


def verify_task_contract_source_ledger(
    contract: dict[str, Any],
    *,
    source_ledger_path: Path,
) -> None:
    observed = sha256_file(source_ledger_path)
    expected = contract["source_ledger"]["sha256"]
    if observed != expected:
        from .errors import AssuranceError

        raise AssuranceError(
            f"task contract source ledger digest mismatch: expected {expected}, observed {observed}"
        )
