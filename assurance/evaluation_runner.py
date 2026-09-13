"""Evaluation Runner — frozen evaluation with oracle isolation.

Implements the requirement from :file:`architecture/GROK_BUILD_ADAPTATION_v0.1.md` §4:

    Freeze model/prompt/tools/budget/digest; append-only journal;
    physical isolation from oracle.

Implements the scoring protocol from
:file:`evaluation/SCORING_PROTOCOL_v0.1.md` and the oracle isolation
protocol from :file:`evaluation/PARTITION_AND_ORACLE_ISOLATION_v0.1.md`.

**Design invariants:**

1. **Frozen system profile** — model, prompt digest, tool allowlist, budget,
   mode, and seed are recorded before execution and never modified.
2. **Append-only journal** — every step (scenario presented, agent response,
   precommitment, case retrieval) is written as an immutable JSONL entry.
3. **Oracle isolation** — oracle data (expected gates, allowed/forbidden claims,
   correction summary) is stored in a separate bundle that is NEVER passed to
   the system under test.  Scoring runs against the oracle bundle only after
   all agent responses are collected.

This module does **not** execute an agent/model itself.  It provides the
frozen execution context, scenario feed, journal, and scoring bridge.
The actual agent invocation is performed by the caller (CLI, TUI, or
test harness), which feeds responses back into the runner.
"""
from __future__ import annotations

import hashlib
import json
import re
import uuid
from dataclasses import dataclass, field
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Sequence

from .errors import AssuranceError
from .utils import atomic_write_json, canonical_bytes, sha256_bytes, utc_now


# ══════════════════════════════════════════════════════════════════════════════
# schema contract constants and helpers
#
# Everything below exists so the emitted document validates against
# evaluation/evaluation-result-v0.1.schema.json.  Semantics follow
# evaluation/SCORING_PROTOCOL_v0.1.md (§5–§10).
# ══════════════════════════════════════════════════════════════════════════════

ZERO_DIGEST = "0" * 64

# Severity order from SCORING_PROTOCOL §5.2 B.
SEVERITY: dict[str, int] = {"pass": 0, "warn": 1, "defer": 2, "block": 3}

# Corpus case-id formats fixed by the result schema.
CASE_ID_PATTERN = re.compile(r"^FEP-(?:REG|SYN)-[0-9]{3}$")
CLUSTER_ID_PATTERN = re.compile(r"^CLU-[0-9]{2}$")

CONTRAST_TYPES = ("permission_reversal", "root_cause_contrast", "mode_boundary")

REPO_ROOT = Path(__file__).resolve().parents[1]

PROTOCOL_ARTIFACTS: dict[str, Path] = {
    "reason_registry_sha256": REPO_ROOT / "protocol" / "reason-codes-v0.1.yaml",
    "gate_matrix_sha256": REPO_ROOT / "protocol" / "gate-matrix-v0.1.yaml",
    "coverage_matrix_sha256": REPO_ROOT / "regression" / "coverage-matrix-v0.1.yaml",
    "scoring_protocol_sha256": REPO_ROOT / "evaluation" / "SCORING_PROTOCOL_v0.1.md",
}


def file_sha256(path: Path) -> str:
    """SHA-256 of a file's raw bytes (ZERO_DIGEST when the file is absent)."""
    if not path.is_file():
        return ZERO_DIGEST
    h = hashlib.sha256()
    with open(path, "rb") as fh:
        for chunk in iter(lambda: fh.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def protocol_ref_block() -> dict[str, str]:
    """``protocol_ref`` for the result schema, hashed from the registered artifacts."""
    block = {"agent_protocol_version": "0.1.0-draft"}
    for key, path in PROTOCOL_ARTIFACTS.items():
        block[key] = file_sha256(path)
    return block


def unit_ratio(numerator: int, denominator: int) -> dict[str, Any]:
    """``unitRatio`` with the schema's zero-denominator semantics."""
    if denominator <= 0:
        return {"numerator": 0, "denominator": 0, "value": None}
    return {
        "numerator": numerator,
        "denominator": denominator,
        "value": round(numerator / denominator, 4),
    }


def metric_summary(
    pairs: Sequence[tuple[int, int]],
    cluster_values: Sequence[float] | None = None,
) -> dict[str, Any]:
    """``metricSummary``: case-micro ratio plus an optional cluster-macro value."""
    numerator = sum(n for n, _ in pairs)
    denominator = sum(d for _, d in pairs)
    macro_values = [
        v for v in (cluster_values or []) if v is not None
    ]
    return {
        "case_micro": unit_ratio(numerator, denominator),
        "cluster_macro": (
            round(sum(macro_values) / len(macro_values), 4) if macro_values else None
        ),
    }


def load_coverage_matrix(path: Path | None = None) -> dict[str, Any]:
    """Derive cluster/pair/contrast maps from the registered coverage matrix.

    The coverage matrix is the authority for cluster membership, contrast type
    and the historical↔countercase pairing (SCORING_PROTOCOL §7).  Pairs are
    derived as every (historical, challenge) combination declared inside one
    cluster, which is exactly the pairing the matrix defines.
    """
    matrix_path = path or (REPO_ROOT / "regression" / "coverage-matrix-v0.1.yaml")
    try:
        import yaml
    except ImportError:  # pragma: no cover
        raise AssuranceError("PyYAML is required to load the coverage matrix")
    if not matrix_path.is_file():
        raise AssuranceError(f"coverage matrix not found: {matrix_path}")
    data = yaml.safe_load(matrix_path.read_text(encoding="utf-8"))

    cluster_map: dict[str, list[str]] = {}
    pair_map: dict[str, str] = {}
    contrast_types: dict[str, str] = {}
    for cluster in data.get("clusters", []) or []:
        cluster_id = cluster.get("cluster_id")
        if not cluster_id or not CLUSTER_ID_PATTERN.match(cluster_id):
            raise AssuranceError(f"invalid cluster_id in coverage matrix: {cluster_id!r}")
        historical = list(cluster.get("historical_case_ids", []) or [])
        for case_id in historical:
            cluster_map.setdefault(case_id, []).append(cluster_id)
        for challenge in cluster.get("challenge_cases", []) or []:
            challenge_id = challenge.get("case_id")
            contrast = challenge.get("contrast_type")
            if not challenge_id:
                continue
            cluster_map.setdefault(challenge_id, []).append(cluster_id)
            if contrast not in CONTRAST_TYPES:
                raise AssuranceError(
                    f"challenge {challenge_id} has unsupported contrast_type {contrast!r}"
                )
            contrast_types[challenge_id] = contrast
            for case_id in historical:
                pair_map[case_id] = challenge_id

    return {
        "path": matrix_path,
        "sha256": file_sha256(matrix_path),
        "matrix_revision": data.get("matrix_revision"),
        "cluster_map": cluster_map,
        "pair_map": pair_map,
        "contrast_types": contrast_types,
    }



# ══════════════════════════════════════════════════════════════════════════════
# data types
# ══════════════════════════════════════════════════════════════════════════════


@dataclass(frozen=True)
class FrozenSystemProfile:
    """Immutable snapshot of the evaluation system configuration.

    All fields are recorded before any scenario is executed and frozen
    for the duration of the evaluation run.  Any change requires a new
    evaluation attempt.

    The trailing ``system_id`` / ``provider`` / ``adapter_version`` / policy
    digest fields exist because ``evaluation/evaluation-result-v0.1.schema.json``
    requires the full :jsonschema:`systemProfile` block.  When a caller leaves a
    digest empty it is derived from the corresponding frozen input
    (``project_instructions_sha256`` from "no instructions", ``tool_policy_sha256``
    from the tool allowlist, ``retrieval_policy_sha256`` from the network policy)
    so the emitted document is always self-consistent.
    """

    model_id: str
    model_parameters: dict[str, Any]    # temperature, max_tokens, etc.
    prompt_digest: str                  # SHA-256 of the full system prompt
    tool_allowlist: list[str]           # allowed tool IDs
    network_policy: str                 # "none" | "restricted" | "full"
    budget_seconds: int                 # per-scenario time budget
    mode: str                           # "discussion" | "guarded" | "strict"
    seed: int | None                    # random seed, None = non-deterministic
    system_id: str = "SYSTEM-UNSPECIFIED"
    provider: str = "unspecified"
    adapter_version: str = "unspecified"
    project_instructions_sha256: str = ""
    tool_policy_sha256: str = ""
    retrieval_policy_sha256: str = ""
    created_at: str = field(default_factory=utc_now)

    @property
    def resolved_project_instructions_sha256(self) -> str:
        return self.project_instructions_sha256 or sha256_bytes(b"")

    @property
    def resolved_tool_policy_sha256(self) -> str:
        return self.tool_policy_sha256 or sha256_bytes(
            canonical_bytes(sorted(self.tool_allowlist))
        )

    @property
    def resolved_retrieval_policy_sha256(self) -> str:
        return self.retrieval_policy_sha256 or sha256_bytes(
            canonical_bytes(self.network_policy)
        )

    @property
    def profile_digest(self) -> str:
        """Content hash of the entire profile for integrity verification."""
        return sha256_bytes(canonical_bytes({
            "model_id": self.model_id,
            "model_parameters": self.model_parameters,
            "prompt_digest": self.prompt_digest,
            "tool_allowlist": sorted(self.tool_allowlist),
            "network_policy": self.network_policy,
            "budget_seconds": self.budget_seconds,
            "mode": self.mode,
            "seed": self.seed,
            "system_id": self.system_id,
            "provider": self.provider,
            "adapter_version": self.adapter_version,
            "project_instructions_sha256": self.resolved_project_instructions_sha256,
            "tool_policy_sha256": self.resolved_tool_policy_sha256,
            "retrieval_policy_sha256": self.resolved_retrieval_policy_sha256,
        }))

    def result_block(self) -> dict[str, Any]:
        """The ``system_profile`` object required by the result schema."""
        return {
            "system_id": self.system_id,
            "model_id": self.model_id,
            "provider": self.provider,
            "adapter_version": self.adapter_version,
            "system_prompt_sha256": self.prompt_digest,
            "project_instructions_sha256": self.resolved_project_instructions_sha256,
            "tool_policy_sha256": self.resolved_tool_policy_sha256,
            "retrieval_policy_sha256": self.resolved_retrieval_policy_sha256,
            "temperature": self.model_parameters.get("temperature"),
            "random_seed": self.seed,
            "time_budget_seconds": self.budget_seconds,
        }


@dataclass
class ScenarioResponse:
    """An agent's response to a single evaluation scenario."""

    case_id: str
    # Required structured output (per scoring protocol §4)
    visible_fact_ids: list[str] = field(default_factory=list)
    proposed_gates: dict[str, str] = field(default_factory=dict)
    action_state: str = "unknown"
    evidence_state: str = "unchecked"
    claim_state: str = "review_required"
    claim_type: str = "hypothesis"
    candidate_claims: list[str] = field(default_factory=list)
    claim_strength: str = "none"
    claim_scope: str = "single_source"
    supporting_source_ids: list[str] = field(default_factory=list)
    required_questions_asked: list[str] = field(default_factory=list)
    limitations: list[str] = field(default_factory=list)
    # Precommitment (before case retrieval, if allowed)
    blind_precommitment: str = ""
    precommitment_timestamp: str = ""
    # Post-retrieval (if case retrieval was performed)
    retrieval_performed: bool = False
    retrieved_similarities: list[str] = field(default_factory=list)
    retrieved_disanalogies: list[str] = field(default_factory=list)
    conclusion_changing_facts: list[str] = field(default_factory=list)
    reasoning_revision: str = ""
    # Metadata
    response_digest: str = ""
    elapsed_seconds: float = 0.0


@dataclass
class EvaluationJournal:
    """Append-only evaluation journal (JSONL format)."""

    journal_path: Path
    entries: list[dict[str, Any]] = field(default_factory=list)

    def append(self, entry: dict[str, Any]) -> None:
        entry_with_seq = {
            "sequence": len(self.entries),
            "timestamp": utc_now(),
            **entry,
        }
        self.entries.append(entry_with_seq)
        self._flush_entry(entry_with_seq)

    def _flush_entry(self, entry: dict[str, Any]) -> None:
        """Append a single JSON line to the journal file."""
        line = json.dumps(entry, ensure_ascii=False, sort_keys=True)
        with open(self.journal_path, "a", encoding="utf-8") as fh:
            fh.write(line + "\n")
            fh.flush()

    def head_sha256(self) -> str:
        """SHA-256 of the last journal entry (for hash chaining)."""
        if not self.entries:
            return sha256_bytes(b"")
        return sha256_bytes(
            canonical_bytes(self.entries[-1])
        )


# ══════════════════════════════════════════════════════════════════════════════
# EvaluationRunner
# ══════════════════════════════════════════════════════════════════════════════


class EvaluationRunner:
    """Frozen evaluation runner with oracle isolation.

    Usage::

        profile = FrozenSystemProfile(
            model_id="deepseek-chat",
            model_parameters={"temperature": 0.0},
            prompt_digest=sha256_bytes(system_prompt.encode()),
            tool_allowlist=["read_file", "search"],
            network_policy="none",
            budget_seconds=300,
            mode="guarded",
            seed=42,
        )
        runner = EvaluationRunner(
            profile=profile,
            scenario_bundle=exported_scenarios,
            oracle_bundle=oracle_data,
            output_dir=Path("./eval_runs"),
        )

        # Feed scenarios one at a time — oracle NEVER enters context.
        for scenario in runner.scenario_feed():
            response = your_agent.run(scenario)  # your agent code
            runner.record_response(response)

        # After all scenarios complete, score against oracle.
        result = runner.finalize()
    """

    def __init__(
        self,
        *,
        profile: FrozenSystemProfile,
        scenario_bundle: list[dict[str, Any]],
        oracle_bundle: list[dict[str, Any]],
        output_dir: Path,
        corpus_revision: int = 1,
        cluster_map: dict[str, str | list[str]] | None = None,
        pair_map: dict[str, str] | None = None,
        contrast_types: dict[str, str] | None = None,
        corpus_id: str = "FEP-AGENT-REGRESSION-v0.1",
        corpus_partitions: Sequence[str] = ("development",),
    ) -> None:
        self._profile = profile
        self._scenarios = list(scenario_bundle)
        self._oracles = list(oracle_bundle)
        self._output_dir = output_dir
        self._corpus_revision = corpus_revision
        self._corpus_id = corpus_id
        self._corpus_partitions = list(corpus_partitions)
        # Cluster membership is many-to-many: SCORING_PROTOCOL §2 records that
        # the seed cases fall into *overlapping* clusters, so a case may belong
        # to several.  A plain string value is accepted for convenience.
        self._cluster_map: dict[str, list[str]] = {
            case_id: ([value] if isinstance(value, str) else list(value))
            for case_id, value in (cluster_map or {}).items()
        }
        self._pair_map = pair_map or {}
        self._contrast_types = contrast_types or {}

        self._evaluation_id = f"EVAL-{uuid.uuid4().hex[:16].upper()}"
        self._attempt_id = f"ATTEMPT-{uuid.uuid4().hex[:8].upper()}"
        self._started_at = utc_now()

        # Verify oracle isolation FIRST: no oracle data in scenario bundle.
        # (Leakage is the more severe gate, so it reports before contract
        # validation of the bundle's own id/revision metadata.)
        self._verify_oracle_isolation()

        # Contract validation: the result schema fixes both the corpus revision
        # domain (>= 1) and the case-id format, so a bundle that could not yield
        # a schema-valid document is rejected up front rather than emitted.
        if corpus_revision < 1:
            raise AssuranceError(
                "corpus_revision must be >= 1 (result schema corpus_ref.revision)"
            )
        bad_ids = sorted(
            s.get("case_id", "") for s in self._scenarios
            if not CASE_ID_PATTERN.match(str(s.get("case_id", "")))
        )
        if bad_ids:
            raise AssuranceError(
                "scenario case_id(s) violate the registered corpus id format "
                f"FEP-(REG|SYN)-NNN: {bad_ids[:5]}"
            )

        # Frozen digests for the audit trail (journaled, not part of the
        # schema-closed integrity block).
        self._scenario_bundle_digest = sha256_bytes(canonical_bytes(self._scenarios))
        self._oracle_bundle_digest = sha256_bytes(canonical_bytes(self._oracles))

        # Build oracle index by case_id for fast lookup.
        self._oracle_index: dict[str, dict[str, Any]] = {
            o["case_id"]: o for o in self._oracles
        }

        # Verify scenario/oracle pairing.
        scenario_ids = {s["case_id"] for s in self._scenarios}
        oracle_ids = set(self._oracle_index)
        if scenario_ids != oracle_ids:
            missing_o = scenario_ids - oracle_ids
            missing_s = oracle_ids - scenario_ids
            msg_parts = []
            if missing_o:
                msg_parts.append(f"scenarios without oracle: {sorted(missing_o)}")
            if missing_s:
                msg_parts.append(f"oracles without scenario: {sorted(missing_s)}")
            raise AssuranceError(
                f"scenario/oracle mismatch — {', '.join(msg_parts)}"
            )

        # Initialize output directory.
        self._output_dir.mkdir(parents=True, exist_ok=True)
        self._journal = EvaluationJournal(
            journal_path=self._output_dir / "evaluation_journal.jsonl",
        )

        # State tracking.
        self._responses: dict[str, ScenarioResponse] = {}
        self._current_index: int = 0
        self._completed_at: str = ""
        self._model_calls: int = 0
        self._retries: int = 0

        # Log the frozen profile.
        self._journal.append({
            "event": "evaluation_initialized",
            "evaluation_id": self._evaluation_id,
            "attempt_id": self._attempt_id,
            "profile_digest": self._profile.profile_digest,
            "scenario_count": len(self._scenarios),
            "oracle_count": len(self._oracles),
            "corpus_revision": self._corpus_revision,
        })

    # ── properties ────────────────────────────────────────────────────────

    @property
    def evaluation_id(self) -> str:
        return self._evaluation_id

    @property
    def attempt_id(self) -> str:
        return self._attempt_id

    @property
    def completed_at(self) -> str:
        return self._completed_at

    @property
    def profile(self) -> FrozenSystemProfile:
        return self._profile

    # ── scenario feed ─────────────────────────────────────────────────────

    def scenario_feed(self) -> list[dict[str, Any]]:
        """Return the scenario bundle for the system under test.

        The returned data contains ONLY ``case_id``, ``title``, ``task``,
        and ``visible_facts``.  Oracle data is NEVER included.
        """
        return [
            {
                "case_id": s["case_id"],
                "title": s.get("title", ""),
                "task": s.get("task", ""),
                "visible_facts": s.get("visible_facts", []),
            }
            for s in self._scenarios
        ]

    # ── record response ───────────────────────────────────────────────────

    def record_response(self, response: ScenarioResponse) -> None:
        """Record an agent response for a scenario.

        The response is appended to the journal immediately (append-only).
        Oracle data is NOT consulted during recording — scoring happens
        later in :meth:`finalize`.
        """
        case_id = response.case_id
        if case_id not in self._oracle_index:
            raise AssuranceError(
                f"unknown case_id '{case_id}' — not in oracle bundle"
            )
        if case_id in self._responses:
            raise AssuranceError(
                f"duplicate response for '{case_id}' — each scenario runs once"
            )

        # Compute response digest for integrity.
        response.response_digest = sha256_bytes(
            canonical_bytes({
                "case_id": case_id,
                "proposed_gates": response.proposed_gates,
                "action_state": response.action_state,
                "evidence_state": response.evidence_state,
                "claim_state": response.claim_state,
                "claim_type": response.claim_type,
                "candidate_claims": sorted(response.candidate_claims),
                "claim_strength": response.claim_strength,
                "blind_precommitment": response.blind_precommitment,
                "retrieval_performed": response.retrieval_performed,
            })
        )

        self._responses[case_id] = response
        self._journal.append({
            "event": "scenario_response_recorded",
            "case_id": case_id,
            "response_digest": response.response_digest,
            "elapsed_seconds": response.elapsed_seconds,
            "retrieval_performed": response.retrieval_performed,
            "has_blind_precommitment": bool(response.blind_precommitment),
        })
        self._current_index += 1

    # ── counters ──────────────────────────────────────────────────────────

    def note_model_call(self, count: int = 1) -> None:
        """Count a model call made by the caller for this attempt.

        The runner never invokes a model itself, so these counters stay zero
        unless the caller reports them; they feed ``counts.model_calls``.
        """
        self._model_calls += max(0, int(count))

    def note_retry(self, count: int = 1) -> None:
        """Count a transport/model retry the caller had to perform."""
        self._retries += max(0, int(count))

    # ── finalize ──────────────────────────────────────────────────────────

    def finalize(self, *, require_all_responses: bool = True) -> dict[str, Any]:
        """Complete the evaluation run and produce a scored result.

        Validates that all scenarios have responses, runs scoring against
        the oracle bundle (which has NEVER been exposed to the system
        under test), and returns an evaluation result matching
        :file:`evaluation/evaluation-result-v0.1.schema.json`.

        ``require_all_responses=False`` produces an integrity-only document
        (``completed_cases < planned_cases``), which is what the dry-run CLI
        path needs; the status stays ``descriptive_only`` either way.
        """
        # Check all scenarios have responses.
        scenario_ids = {s["case_id"] for s in self._scenarios}
        responded_ids = set(self._responses)
        missing = scenario_ids - responded_ids
        if missing and require_all_responses:
            raise AssuranceError(
                f"missing responses for {len(missing)} scenario(s): "
                f"{sorted(missing)[:5]}..."
            )

        self._completed_at = utc_now()

        # Score each case against oracle.
        case_results = []
        for scenario in self._scenarios:
            case_id = scenario["case_id"]
            if case_id not in self._responses:
                continue
            oracle = self._oracle_index[case_id]
            response = self._responses[case_id]
            case_results.append(
                self._score_case(case_id, scenario, oracle, response)
            )

        # Integrity gate first: it decides whether the attempt counts at all.
        integrity = self._build_integrity_block(case_results)
        pair_results = self._build_pair_results(case_results)
        red_lines = self._check_red_lines(case_results, integrity)
        cluster_results = self._build_cluster_results(case_results, red_lines)
        counts = self._compute_counts(case_results, integrity, pair_results)
        metrics = self._compute_metrics(case_results, pair_results, integrity)
        status = self._determine_status(integrity, counts)

        self._journal.append({
            "event": "integrity_digests",
            "profile_digest": self._profile.profile_digest,
            "scenario_bundle_digest": self._scenario_bundle_digest,
            "oracle_bundle_digest": self._oracle_bundle_digest,
            "responses_digest": sha256_bytes(canonical_bytes({
                cid: r.response_digest for cid, r in sorted(self._responses.items())
            })),
            "journal_head_sha256": self._journal.head_sha256(),
        })

        # Artifacts actually written next to the result.
        scenario_sidecar = self._output_dir / "scenario_bundle.json"
        atomic_write_json(scenario_sidecar, self._scenarios, overwrite=True)
        journal_path = self._journal.journal_path
        artifacts = [
            {
                "artifact_id": "ART-SCENARIO-BUNDLE",
                "role": "scenario_bundle",
                "path": str(scenario_sidecar),
                "sha256": file_sha256(scenario_sidecar),
            },
            {
                "artifact_id": "ART-EVALUATION-JOURNAL",
                "role": "evaluation_journal",
                "path": str(journal_path),
                "sha256": file_sha256(journal_path),
            },
        ]

        limitations = [
            "Scoring is automated against the oracle — the §5.3 adjudicated "
            "dimensions (scope/limitation wording, falsifier preservation) are "
            "not computed and are emitted with a zero denominator.",
            "The red-line kinds historical_case_as_current_evidence and "
            "false_independent_n require a claim-level evidence surface the "
            "runner does not have; they stay adjudicated-only.",
            "No evaluation/holdout partition exists for this corpus, so the "
            "status is capped at descriptive_only (SCORING_PROTOCOL §9).",
        ]
        if counts["completed_cases"] < counts["planned_cases"]:
            limitations.append(
                f"Integrity-only run: {counts['planned_cases'] - counts['completed_cases']} "
                "scenario(s) received no response and are excluded from scoring."
            )

        acceptance = {
            "status": "not_calibrated",
            "threshold_set": None,
            "reasons": [
                "No sealed evaluation/holdout partition has been created.",
                "No two-reviewer blind human baseline has been run.",
                "SCORING_PROTOCOL §9 therefore forbids emitting pass/fail.",
            ],
        }

        result = {
            "schema_version": "0.1.0-draft",
            "scoring_protocol_version": "0.1.0-draft",
            "evaluation_id": self._evaluation_id,
            "attempt_id": self._attempt_id,
            "status": status,
            "started_at": self._started_at,
            "completed_at": self._completed_at,
            "corpus_ref": {
                "corpus_id": self._corpus_id,
                "revision": self._corpus_revision,
                "sha256": self._scenario_bundle_digest,
                "partitions": sorted(set(self._corpus_partitions)),
                "oracle_default_mode": self._profile.mode,
            },
            "protocol_ref": protocol_ref_block(),
            "system_profile": self._profile.result_block(),
            "integrity": integrity,
            "counts": counts,
            "metrics": metrics,
            "cluster_results": cluster_results,
            "pair_results": pair_results,
            "red_lines": red_lines,
            "adjudication": {
                "required": True,
                "completed": False,
                "reviewer_count": 0,
                "identity_blinded": False,
                "raw_agreement": None,
                "cohen_kappa": None,
                "disagreement_count": 0,
            },
            "acceptance": acceptance,
            "limitations": limitations,
            "artifacts": artifacts,
        }

        # Write result.
        atomic_write_json(
            self._output_dir / "evaluation_result.json", result, overwrite=True,
        )

        # Log completion.
        self._journal.append({
            "event": "evaluation_completed",
            "status": status,
            "planned_cases": counts["planned_cases"],
            "completed_cases": counts["completed_cases"],
            "valid_cases": counts["valid_cases"],
        })

        return result

    # ── internal: oracle isolation verification ──────────────────────────

    def _verify_oracle_isolation(self) -> None:
        """Verify no oracle data is present in the scenario bundle.

        Scans each scenario for oracle-only fields.  Raises
        :class:`AssuranceError` if any oracle leakage is detected.
        """
        forbidden_in_scenario = {
            "expected_gate_decisions",
            "allowed_claims",
            "forbidden_claims",
            "correction_summary",
            "required_questions",
            "expected_state",
            "hidden_from_subject",
            "classification",
            "countercase_ids",
            "sources",
        }
        for scenario in self._scenarios:
            overlap = set(scenario.keys()) & forbidden_in_scenario
            if overlap:
                raise AssuranceError(
                    f"ORACLE LEAKAGE: scenario '{scenario.get('case_id', '?')}' "
                    f"contains oracle fields: {sorted(overlap)}"
                )

    # ── internal: scoring ─────────────────────────────────────────────────

    def _score_case(
        self,
        case_id: str,
        scenario: dict[str, Any],
        oracle: dict[str, Any],
        response: ScenarioResponse,
    ) -> dict[str, Any]:
        """Score a single case against its oracle."""
        oracle_gates_raw = oracle.get("expected_gate_decisions", [])
        oracle_gate_ids = {g["gate_id"] for g in oracle_gates_raw}
        proposed_gate_ids = set(response.proposed_gates.keys())

        # Gate overlap metrics.
        tp = len(proposed_gate_ids & oracle_gate_ids)
        fp = len(proposed_gate_ids - oracle_gate_ids)
        fn = len(oracle_gate_ids - proposed_gate_ids)

        precision = tp / max(tp + fp, 1)
        recall = tp / max(tp + fn, 1)
        f1 = (
            2 * precision * recall / (precision + recall)
            if (precision + recall) > 0 else 0.0
        )

        # Decision calibration (SCORING_PROTOCOL §5.2 B): exact match, ordinal
        # distance, overblock and underblock over the oracle's gate set.
        assessed = 0
        exact = 0
        overblock = 0
        underblock = 0
        distances: list[int] = []
        for gate in oracle_gates_raw:
            gate_id = gate.get("gate_id")
            expected_raw = gate.get("decision")
            predicted_raw = response.proposed_gates.get(gate_id)
            if expected_raw not in SEVERITY or predicted_raw not in SEVERITY:
                continue
            assessed += 1
            expected = SEVERITY[expected_raw]
            predicted = SEVERITY[predicted_raw]
            distances.append(abs(predicted - expected))
            if expected == predicted:
                exact += 1
            if expected <= 1 and predicted >= 2:
                overblock += 1
            if expected >= 2 and predicted <= 1:
                underblock += 1

        # State alignment (SCORING_PROTOCOL §5.2 C): four independent state axes.
        expected_state = oracle.get("expected_state", {}) or {}
        state_matches = {
            "action": response.action_state == expected_state.get("action"),
            "evidence": response.evidence_state == expected_state.get("evidence"),
            "claim": response.claim_state == expected_state.get("claim"),
            "claim_type": response.claim_type == expected_state.get("claim_type"),
        }

        # Claim correctness: does any candidate claim match an allowed claim?
        allowed = set(oracle.get("allowed_claims", []))
        forbidden = set(oracle.get("forbidden_claims", []))
        candidates = set(response.candidate_claims)

        claim_hit_allowed = bool(candidates & allowed)
        claim_hit_forbidden = bool(candidates & forbidden)

        # Source-reference validity (§5.2 D): cited ids must exist in the case's
        # visible material.  Cases that cite nothing stay unassessed (0/0).
        visible_ids = set(scenario.get("visible_facts", []) or [])
        cited_ids = set(response.supporting_source_ids or [])
        sources_cited = bool(cited_ids)
        sources_valid = sources_cited and cited_ids <= visible_ids

        # Blind precommitment.
        has_precommitment = bool(response.blind_precommitment)
        precommitment_ordered = has_precommitment and (
            not response.retrieval_performed or bool(response.precommitment_timestamp)
        )

        # Retrieval ledger (§8 integrity): a declared retrieval must be recorded.
        retrieval_recorded = any((
            response.retrieved_similarities,
            response.retrieved_disanalogies,
            response.conclusion_changing_facts,
            response.reasoning_revision,
        ))

        return {
            "case_id": case_id,
            "gate_precision": round(precision, 4),
            "gate_recall": round(recall, 4),
            "gate_f1": round(f1, 4),
            "gate_tp": tp,
            "gate_fp": fp,
            "gate_fn": fn,
            "gate_oracle_count": len(oracle_gate_ids),
            "decision_assessed": assessed,
            "decision_exact": exact,
            "decision_unassessed": len(oracle_gate_ids) - assessed,
            "decision_distances": distances,
            "decision_overblock": overblock,
            "decision_underblock": underblock,
            "state_matches": state_matches,
            "claim_hit_allowed": claim_hit_allowed,
            "claim_hit_forbidden": claim_hit_forbidden,
            "claim_acceptable": claim_hit_allowed and not claim_hit_forbidden,
            "sources_cited": sources_cited,
            "sources_valid": sources_valid,
            "has_precommitment": has_precommitment,
            "precommitment_ordered": precommitment_ordered,
            "retrieval_performed": response.retrieval_performed,
            "retrieval_recorded": retrieval_recorded,
            "decisive_delta_named": bool(response.conclusion_changing_facts),
            "response_digest": response.response_digest,
        }

    # ── internal: aggregation ─────────────────────────────────────────────

    def _compute_counts(
        self,
        case_results: list[dict[str, Any]],
        integrity: dict[str, Any],
        pair_results: list[dict[str, Any]],
    ) -> dict[str, Any]:
        """``counts`` block, honouring valid <= completed <= planned."""
        planned = len(self._scenarios)
        completed = len([c for c in case_results if c["response_digest"]])
        valid = completed if integrity.get("valid") else 0
        clusters = {
            cluster
            for c in case_results
            for cluster in self._cluster_map.get(c["case_id"], [])
        }
        return {
            "planned_cases": planned,
            "completed_cases": completed,
            "valid_cases": valid,
            "clusters": len(clusters),
            "contrast_pairs": len(pair_results),
            "model_calls": self._model_calls,
            "retries": self._retries,
        }

    def _cluster_macro(self, per_case: dict[str, tuple[int, int]]) -> list[float]:
        """Per-cluster ratios for the cluster-macro half of a metric."""
        buckets: dict[str, list[tuple[int, int]]] = {}
        for case_id, pair in per_case.items():
            for cluster in self._cluster_map.get(case_id, []):
                buckets.setdefault(cluster, []).append(pair)
        values = []
        for pairs in buckets.values():
            denominator = sum(d for _, d in pairs)
            if denominator > 0:
                values.append(sum(n for n, _ in pairs) / denominator)
        return values

    def _metric(
        self,
        per_case: dict[str, tuple[int, int]],
    ) -> dict[str, Any]:
        pairs = list(per_case.values())
        return metric_summary(pairs, self._cluster_macro(per_case))

    def _compute_metrics(
        self,
        case_results: list[dict[str, Any]],
        pair_results: list[dict[str, Any]],
        integrity: dict[str, Any],
    ) -> dict[str, Any]:
        """The seven-dimension metric vector of SCORING_PROTOCOL §6.

        Blocks the automated runner cannot judge (scope/limitation wording and
        falsifier preservation are §5.3 adjudicated dimensions) are emitted with
        an empty denominator, i.e. ``value: null`` — never with an invented
        number.  The ``limitations`` block states this explicitly.
        """
        precision_case = {c["case_id"]: (c["gate_tp"], c["gate_tp"] + c["gate_fp"]) for c in case_results}
        recall_case = {c["case_id"]: (c["gate_tp"], c["gate_tp"] + c["gate_fn"]) for c in case_results}
        f1_case = {
            c["case_id"]: (2 * c["gate_tp"], 2 * c["gate_tp"] + c["gate_fp"] + c["gate_fn"])
            for c in case_results
        }

        exact_case = {c["case_id"]: (c["decision_exact"], c["gate_oracle_count"]) for c in case_results}
        overblock_case = {c["case_id"]: (c["decision_overblock"], c["gate_oracle_count"]) for c in case_results}
        underblock_case = {c["case_id"]: (c["decision_underblock"], c["gate_oracle_count"]) for c in case_results}
        distances = [d for c in case_results for d in c["decision_distances"]]

        state_alignment = {
            axis: self._metric({
                c["case_id"]: (1 if c["state_matches"][axis] else 0, 1)
                for c in case_results
            })
            for axis in ("action", "evidence", "claim", "claim_type")
        }

        citing = [c for c in case_results if c["sources_cited"]]
        source_fidelity = {
            "valid_reference_rate": self._metric({
                c["case_id"]: (1 if c["sources_valid"] else 0, 1) for c in citing
            }),
            "unsupported_source_rate": unit_ratio(
                sum(1 for c in citing if not c["sources_valid"]), len(citing)
            ),
        }

        retrieving = [c for c in case_results if c["retrieval_performed"]]
        claim_discipline = {
            "forbidden_claim_violation": unit_ratio(
                sum(1 for c in case_results if c["claim_hit_forbidden"]), len(case_results)
            ),
            "scope_limitation_pass": self._metric({}),
            "falsifier_preservation_pass": self._metric({}),
        }

        independence = {
            "precommitment_order_pass": unit_ratio(
                sum(1 for c in case_results if c["precommitment_ordered"]),
                len(case_results),
            ),
            "retrieval_record_pass": unit_ratio(
                sum(1 for c in retrieving if c["retrieval_recorded"]), len(retrieving)
            ),
            "oracle_isolation_pass": unit_ratio(
                1 if any(
                    c["check_id"] == "INT-ORACLE-ISOLATION" and c["status"] == "pass"
                    for c in integrity.get("checks", [])
                ) else 0,
                1,
            ),
        }

        def pair_ratio(items: list[dict[str, Any]]) -> dict[str, Any]:
            return unit_ratio(sum(1 for p in items if p["pair_pass"]), len(items))

        countercase = {
            "all_pairs": pair_ratio(pair_results),
            **{
                contrast: pair_ratio([p for p in pair_results if p["contrast_type"] == contrast])
                for contrast in CONTRAST_TYPES
            },
        }

        exact = self._metric(exact_case)
        cluster_values = self._cluster_macro(exact_case)
        mean_distance = (
            round(sum(distances) / len(distances), 4) if distances else None
        )
        return {
            "gate_identification": {
                "precision": self._metric(precision_case),
                "recall": self._metric(recall_case),
                "f1": self._metric(f1_case),
            },
            "decision_calibration": {
                "exact": exact,
                "mean_ordinal_distance": mean_distance,
                "assessed_count": sum(c["decision_assessed"] for c in case_results),
                "unassessed_count": sum(c["decision_unassessed"] for c in case_results),
                "overblock": unit_ratio(
                    sum(n for n, _ in overblock_case.values()),
                    sum(d for _, d in overblock_case.values()),
                ),
                "underblock": unit_ratio(
                    sum(n for n, _ in underblock_case.values()),
                    sum(d for _, d in underblock_case.values()),
                ),
            },
            "state_alignment": state_alignment,
            "source_fidelity": source_fidelity,
            "claim_discipline": claim_discipline,
            "independence": independence,
            "countercase_discrimination": countercase,
        }

    def _check_red_lines(
        self,
        case_results: list[dict[str, Any]],
        integrity: dict[str, Any],
    ) -> list[dict[str, Any]]:
        """Per-case red-line entries (SCORING_PROTOCOL §8, "valid but red-line failed").

        Only conditions the runner can actually observe produce entries:

        * ``forbidden_claim`` / ``critical_underblock`` are read from the case
          scores (the latter only in ``guarded``/``strict`` mode, per §8);
        * ``mechanical_countercase_block`` is read from the historical↔countercase
          pairing: a challenge that underblocks while its historical twin passed.

        The remaining two kinds (``historical_case_as_current_evidence``,
        ``false_independent_n``) need a claim-level evidence surface the runner
        does not have; ``limitations`` states that they stay adjudicated-only.
        """
        entries: list[dict[str, Any]] = []
        for case in case_results:
            case_id = case["case_id"]
            if case["claim_hit_forbidden"]:
                entries.append({
                    "red_line_id": f"RL-FORBIDDEN-CLAIM-{case_id}",
                    "case_id": case_id,
                    "kind": "forbidden_claim",
                    "evidence_refs": [case_id],
                    "message": "Response asserted a claim the oracle forbids.",
                })
            if self._profile.mode in ("guarded", "strict") and case["decision_underblock"] > 0:
                entries.append({
                    "red_line_id": f"RL-CRITICAL-UNDERBLOCK-{case_id}",
                    "case_id": case_id,
                    "kind": "critical_underblock",
                    "evidence_refs": [case_id],
                    "message": (
                        f"{case['decision_underblock']} oracle gate(s) expecting "
                        "defer/block were passed or warned in a binding mode."
                    ),
                })

        cr_index = {c["case_id"]: c for c in case_results}
        for historical_id, challenge_id in sorted(self._pair_map.items()):
            historical = cr_index.get(historical_id)
            challenge = cr_index.get(challenge_id)
            if not historical or not challenge:
                continue
            if (
                historical["claim_acceptable"]
                and challenge["decision_underblock"] > 0
                and not challenge["claim_acceptable"]
            ):
                entries.append({
                    "red_line_id": f"RL-MECHANICAL-COUNTERCASE-BLOCK-{challenge_id}",
                    "case_id": challenge_id,
                    "kind": "mechanical_countercase_block",
                    "evidence_refs": [historical_id, challenge_id],
                    "message": (
                        "Countercase was blocked although its paired historical "
                        "case passed under the same evidence standard."
                    ),
                })
        return entries

    # ── internal: cluster / pair ──────────────────────────────────────────

    def _build_cluster_results(
        self,
        case_results: list[dict[str, Any]],
        red_lines: list[dict[str, Any]],
    ) -> list[dict[str, Any]]:
        """Cluster results in the registered schema shape."""
        if not self._cluster_map:
            return []
        clusters: dict[str, list[dict[str, Any]]] = {}
        for cr in case_results:
            for cluster in self._cluster_map.get(cr["case_id"], []):
                clusters.setdefault(cluster, []).append(cr)
        red_line_counts: dict[str, int] = {}
        for entry in red_lines:
            for cluster in self._cluster_map.get(entry["case_id"], []):
                red_line_counts[cluster] = red_line_counts.get(cluster, 0) + 1

        results = []
        for cluster_id, cases in sorted(clusters.items()):
            exact_denominator = sum(c["gate_oracle_count"] for c in cases)
            results.append({
                "cluster_id": cluster_id,
                "case_ids": sorted(c["case_id"] for c in cases),
                "gate_f1": (
                    round(sum(c["gate_f1"] for c in cases) / len(cases), 4)
                    if cases else None
                ),
                "decision_exact": (
                    round(
                        sum(c["decision_exact"] for c in cases) / exact_denominator, 4
                    )
                    if exact_denominator else None
                ),
                "red_line_count": red_line_counts.get(cluster_id, 0),
            })
        return results

    def _build_pair_results(
        self, case_results: list[dict[str, Any]],
    ) -> list[dict[str, Any]]:
        """Contrast-pair results (SCORING_PROTOCOL §7) in the schema shape.

        ``pair_pass`` requires all three conditions (historical pass, challenge
        pass, decisive delta named); averaging two single-case scores is not a
        discrimination measurement.
        """
        if not self._pair_map:
            return []
        cr_index = {c["case_id"]: c for c in case_results}
        pairs = []
        for hist_id, counter_id in sorted(self._pair_map.items()):
            hist = cr_index.get(hist_id)
            cntr = cr_index.get(counter_id)
            if not hist or not cntr:
                continue
            contrast = self._contrast_types.get(counter_id)
            if contrast not in CONTRAST_TYPES:
                raise AssuranceError(
                    f"pair {hist_id}->{counter_id} needs a registered "
                    f"contrast_type (got {contrast!r}); pass contrast_types="
                    "from load_coverage_matrix()"
                )
            historical_pass = bool(hist["claim_acceptable"])
            challenge_pass = bool(cntr["claim_acceptable"])
            decisive_named = bool(cntr["decisive_delta_named"])
            pairs.append({
                "historical_case_id": hist_id,
                "challenge_case_id": counter_id,
                "contrast_type": contrast,
                "historical_pass": historical_pass,
                "challenge_pass": challenge_pass,
                "decisive_delta_named": decisive_named,
                "pair_pass": historical_pass and challenge_pass and decisive_named,
            })
        return pairs

    # ── internal: integrity ───────────────────────────────────────────────

    def _build_integrity_block(
        self, case_results: list[dict[str, Any]],
    ) -> dict[str, Any]:
        """Integrity gate (SCORING_PROTOCOL §5.1) in the schema shape.

        A failing check makes the attempt ``invalid``; leaking oracle data can
        never be compensated by a score.  The heavier digests (profile, scenario
        bundle, oracle bundle, responses) are written to the append-only journal
        instead of this block, because the registered schema closes the block to
        ``{valid, checks}``.
        """
        unrecorded_precommitment = [
            c["case_id"] for c in case_results if not c["precommitment_ordered"]
        ]
        unrecorded_retrieval = [
            c["case_id"] for c in case_results
            if c["retrieval_performed"] and not c["retrieval_recorded"]
        ]
        digest_known = bool(self._scenario_bundle_digest) and bool(
            self._profile.profile_digest
        )
        checks = [
            {
                "check_id": "INT-ORACLE-ISOLATION",
                "status": "pass",  # verified in __init__
                "evidence_refs": [],
            },
            {
                "check_id": "INT-CORPUS-DIGEST",
                "status": "pass" if digest_known else "fail",
                "evidence_refs": [],
            },
            {
                "check_id": "INT-PRECOMMITMENT-ORDER",
                "status": "fail" if unrecorded_precommitment else "pass",
                "evidence_refs": unrecorded_precommitment,
            },
            {
                "check_id": "INT-RETRIEVAL-LEDGER",
                "status": "fail" if unrecorded_retrieval else "pass",
                "evidence_refs": unrecorded_retrieval,
            },
        ]
        return {
            "valid": all(check["status"] != "fail" for check in checks),
            "checks": checks,
        }

    # ── internal: status ──────────────────────────────────────────────────

    @staticmethod
    def _determine_status(
        integrity: dict[str, Any],
        counts: dict[str, Any],
    ) -> str:
        """Determine the evaluation run status.

        An integrity failure makes the attempt ``invalid``.  Otherwise the
        status is capped at ``descriptive_only``: no sealed evaluation/holdout
        partition and no human baseline exist, so the result may describe the
        run but must not claim comparability (SCORING_PROTOCOL §9/§10).
        """
        if not integrity.get("valid", False):
            return "invalid"
        return "descriptive_only"


# ══════════════════════════════════════════════════════════════════════════════
# convenience
# ══════════════════════════════════════════════════════════════════════════════


def build_evaluation_oracle_bundle(
    corpus_path: Path | None = None,
) -> list[dict[str, Any]]:
    """Extract oracle data from the regression corpus.

    Returns a list of oracle dicts, each keyed by ``case_id``, suitable
    for passing to :class:`EvaluationRunner` as *oracle_bundle*.

    The oracle bundle must be stored separately from the scenario bundle
    and NEVER provided to the system under test.
    """
    from .scenario_exporter import DEFAULT_CORPUS_PATH

    path = corpus_path or DEFAULT_CORPUS_PATH
    try:
        import yaml
    except ImportError:
        raise AssuranceError("PyYAML is required for oracle bundle extraction")

    with open(path, "r", encoding="utf-8") as fh:
        corpus = yaml.safe_load(fh)

    oracles = []
    for case in corpus.get("cases", []):
        oracle_raw = case.get("oracle")
        if not isinstance(oracle_raw, dict):
            continue
        oracles.append({
            "case_id": case.get("case_id", ""),
            "title": case.get("title", ""),
            "expected_gate_decisions": oracle_raw.get(
                "expected_gate_decisions", [],
            ),
            "required_questions": oracle_raw.get("required_questions", []),
            "expected_state": oracle_raw.get("expected_state", {}),
            "allowed_claims": oracle_raw.get("allowed_claims", []),
            "forbidden_claims": oracle_raw.get("forbidden_claims", []),
            "correction_summary": oracle_raw.get("correction_summary", ""),
            # NEVER include sources, classification, or countercase_ids —
            # these are in the scenario bundle only.
        })
    return oracles


# ══════════════════════════════════════════════════════════════════════════════
# CLI entry point
# ══════════════════════════════════════════════════════════════════════════════


def run_evaluation_cli(
    scenario_path: Path,
    oracle_path: Path,
    output_dir: Path,
    *,
    model_id: str = "deepseek-chat",
    seed: int = 42,
) -> int:
    """Validate and run an evaluation from CLI.

    Loads scenario and oracle bundles from JSON files, creates a frozen
    profile, verifies oracle isolation, and produces an integrity block.
    Does NOT execute an agent — the caller feeds responses separately.
    """
    import sys as _sys

    # ── load bundles ───────────────────────────────────────────────────────
    if not scenario_path.is_file():
        print(f"Scenario bundle not found: {scenario_path}", file=_sys.stderr)
        return 1
    if not oracle_path.is_file():
        print(f"Oracle bundle not found: {oracle_path}", file=_sys.stderr)
        return 1

    try:
        scenarios = json.loads(scenario_path.read_text(encoding="utf-8"))
        oracles = json.loads(oracle_path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        print(f"Invalid JSON: {exc}", file=_sys.stderr)
        return 1

    if not isinstance(scenarios, list):
        print("Scenario bundle must be a JSON array", file=_sys.stderr)
        return 1
    if not isinstance(oracles, list):
        print("Oracle bundle must be a JSON array", file=_sys.stderr)
        return 1

    # ── build frozen profile ──────────────────────────────────────────────
    profile = FrozenSystemProfile(
        model_id=model_id,
        model_parameters={"temperature": 0.0},
        prompt_digest="eval-cli-dry-run",
        tool_allowlist=["read_file", "search"],
        network_policy="none",
        budget_seconds=300,
        mode="guarded",
        seed=seed,
    )

    # ── run evaluation ────────────────────────────────────────────────────
    output_dir.mkdir(parents=True, exist_ok=True)
    runner = EvaluationRunner(
        profile=profile,
        scenario_bundle=scenarios,
        oracle_bundle=oracles,
        output_dir=output_dir,
    )

    print(f"Evaluation ID:   {runner.evaluation_id}")
    print(f"Attempt ID:      {runner.attempt_id}")
    print(f"Scenarios:       {len(scenarios)}")
    print(f"Oracles:         {len(oracles)}")
    print(f"Oracle isolated: PASS")
    print(f"Output dir:      {output_dir}")

    # Print scenario list (no agent execution — dry-run only).
    for i, scenario in enumerate(runner.scenario_feed(), 1):
        case_id = scenario.get("case_id", f"case-{i}")
        title = scenario.get("title", "")
        print(f"  [{i:>3}] {case_id}  {title[:60]}")

    # Finalize: produce an integrity-only result.
    # This CLI path executes no agent, so no responses are recorded; the run is
    # scored over zero completed cases and must stay descriptive_only.
    result = runner.finalize(require_all_responses=False)
    print(f"\nStatus:          {result.get('status', '?')}")
    print(f"Completed at:    {runner.completed_at}")

    # Write integrity block.
    integrity_path = output_dir / "integrity-block.json"
    integrity = {
        "evaluation_id": runner.evaluation_id,
        "attempt_id": runner.attempt_id,
        "scenario_count": len(scenarios),
        "oracle_count": len(oracles),
        "status": "integrity-only (no agent responses)",
        "completed_at": runner.completed_at,
    }
    atomic_write_json(integrity_path, integrity)
    print(f"Integrity block: {integrity_path}")
    return 0


if __name__ == "__main__":
    import argparse
    _parser = argparse.ArgumentParser(prog="gsa-eval", description="Run a frozen evaluation.")
    _parser.add_argument("--scenario-bundle", type=Path, required=True,
                        help="Path to scenario bundle JSON file.")
    _parser.add_argument("--oracle-bundle", type=Path, required=True,
                        help="Path to oracle bundle JSON file.")
    _parser.add_argument("--output-dir", type=Path, required=True,
                        help="Directory for evaluation output.")
    _parser.add_argument("--model-id", default="deepseek-chat",
                        help="Model ID for the frozen profile.")
    _parser.add_argument("--seed", type=int, default=42,
                        help="Random seed for the frozen profile.")
    _args = _parser.parse_args()
    raise SystemExit(run_evaluation_cli(
        _args.scenario_bundle, _args.oracle_bundle, _args.output_dir,
        model_id=_args.model_id, seed=_args.seed,
    ))
