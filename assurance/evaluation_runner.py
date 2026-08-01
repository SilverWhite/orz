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

import json
import uuid
from dataclasses import dataclass, field
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Sequence

from .errors import AssuranceError
from .utils import atomic_write_json, canonical_bytes, sha256_bytes, utc_now


# ══════════════════════════════════════════════════════════════════════════════
# data types
# ══════════════════════════════════════════════════════════════════════════════


@dataclass(frozen=True)
class FrozenSystemProfile:
    """Immutable snapshot of the evaluation system configuration.

    All fields are recorded before any scenario is executed and frozen
    for the duration of the evaluation run.  Any change requires a new
    evaluation attempt.
    """

    model_id: str
    model_parameters: dict[str, Any]    # temperature, max_tokens, etc.
    prompt_digest: str                  # SHA-256 of the full system prompt
    tool_allowlist: list[str]           # allowed tool IDs
    network_policy: str                 # "none" | "restricted" | "full"
    budget_seconds: int                 # per-scenario time budget
    mode: str                           # "discussion" | "guarded" | "strict"
    seed: int | None                    # random seed, None = non-deterministic
    created_at: str = field(default_factory=utc_now)

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
        }))


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
        corpus_revision: int = 0,
        cluster_map: dict[str, str] | None = None,
        pair_map: dict[str, str] | None = None,
    ) -> None:
        self._profile = profile
        self._scenarios = list(scenario_bundle)
        self._oracles = list(oracle_bundle)
        self._output_dir = output_dir
        self._corpus_revision = corpus_revision
        self._cluster_map = cluster_map or {}
        self._pair_map = pair_map or {}

        self._evaluation_id = f"EVAL-{uuid.uuid4().hex[:16].upper()}"
        self._attempt_id = f"ATTEMPT-{uuid.uuid4().hex[:8].upper()}"
        self._started_at = utc_now()

        # Verify oracle isolation FIRST: no oracle data in scenario bundle.
        self._verify_oracle_isolation()

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

    # ── finalize ──────────────────────────────────────────────────────────

    def finalize(self) -> dict[str, Any]:
        """Complete the evaluation run and produce a scored result.

        Validates that all scenarios have responses, runs scoring against
        the oracle bundle (which has NEVER been exposed to the system
        under test), and returns an evaluation result matching
        :file:`evaluation/evaluation-result-v0.1.schema.json`.
        """
        # Check all scenarios have responses.
        scenario_ids = {s["case_id"] for s in self._scenarios}
        responded_ids = set(self._responses)
        missing = scenario_ids - responded_ids
        if missing:
            raise AssuranceError(
                f"missing responses for {len(missing)} scenario(s): "
                f"{sorted(missing)[:5]}..."
            )

        self._completed_at = utc_now()

        # Build integrity block.
        integrity = self._build_integrity_block()

        # Score each case against oracle.
        case_results = []
        for scenario in self._scenarios:
            case_id = scenario["case_id"]
            oracle = self._oracle_index[case_id]
            response = self._responses[case_id]
            case_results.append(
                self._score_case(case_id, scenario, oracle, response)
            )

        # Aggregate metrics.
        counts = self._compute_counts(case_results)
        metrics = self._compute_metrics(case_results)
        red_lines = self._check_red_lines(case_results, integrity)

        # Cluster-level results.
        cluster_results = self._build_cluster_results(case_results)

        # Pair (countercase) results.
        pair_results = self._build_pair_results(case_results)

        # Determine status.
        status = self._determine_status(integrity, red_lines, counts)

        # Build result.
        result = {
            "schema_version": "0.1.0-draft",
            "scoring_protocol_version": "0.1.0-draft",
            "evaluation_id": self._evaluation_id,
            "attempt_id": self._attempt_id,
            "status": status,
            "started_at": self._started_at,
            "completed_at": self._completed_at,
            "corpus_ref": {
                "corpus_id": "FEP-AGENT-REGRESSION-v0.1",
                "revision": self._corpus_revision,
                "digest": sha256_bytes(
                    canonical_bytes(self._scenarios)
                ),
            },
            "protocol_ref": {
                "scoring": "SCORING_PROTOCOL_v0.1",
                "oracle_isolation": "PARTITION_AND_ORACLE_ISOLATION_v0.1",
            },
            "system_profile": {
                "model_id": self._profile.model_id,
                "profile_digest": self._profile.profile_digest,
                "mode": self._profile.mode,
                "seed": self._profile.seed,
            },
            "integrity": integrity,
            "counts": counts,
            "metrics": metrics,
            "cluster_results": cluster_results,
            "pair_results": pair_results,
            "red_lines": red_lines,
            "adjudication": {
                "adjudicator": "automated",  # no human adjudication yet
                "divergence_count": 0,
                "notes": ["Automated scoring only — no human review performed."],
            },
            "acceptance": {
                "status": (
                    "accepted" if status == "eligible_for_comparison"
                    else "requires_review"
                ),
                "limiting_factors": [
                    r["rule"] for r in red_lines if r["triggered"]
                ] if any(r["triggered"] for r in red_lines) else [],
            },
            "limitations": [
                "Scoring is automated against oracle — no human calibration.",
                "Cluster-level aggregation is preliminary.",
                "No evaluation/holdout partitions exist — status capped at descriptive_only.",
            ],
            "artifacts": {
                "journal_sha256": sha256_bytes(
                    str(self._journal.journal_path).encode("utf-8")
                ),
                "journal_path": str(self._journal.journal_path),
                "output_dir": str(self._output_dir),
            },
        }

        # Write result.
        atomic_write_json(
            self._output_dir / "evaluation_result.json", result, overwrite=True,
        )

        # Log completion.
        self._journal.append({
            "event": "evaluation_completed",
            "status": status,
            "total_cases": counts["total"],
            "completed_cases": counts["completed"],
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

        # Claim correctness: does any candidate claim match an allowed claim?
        allowed = set(oracle.get("allowed_claims", []))
        forbidden = set(oracle.get("forbidden_claims", []))
        candidates = set(response.candidate_claims)

        claim_hit_allowed = bool(candidates & allowed)
        claim_hit_forbidden = bool(candidates & forbidden)

        # Blind precommitment.
        has_precommitment = bool(response.blind_precommitment)

        return {
            "case_id": case_id,
            "gate_precision": round(precision, 4),
            "gate_recall": round(recall, 4),
            "gate_f1": round(f1, 4),
            "claim_hit_allowed": claim_hit_allowed,
            "claim_hit_forbidden": claim_hit_forbidden,
            "claim_acceptable": claim_hit_allowed and not claim_hit_forbidden,
            "has_precommitment": has_precommitment,
            "retrieval_performed": response.retrieval_performed,
            "response_digest": response.response_digest,
        }

    # ── internal: aggregation ─────────────────────────────────────────────

    def _compute_counts(self, case_results: list[dict[str, Any]]) -> dict[str, Any]:
        total = len(case_results)
        completed = len([c for c in case_results if c["response_digest"]])
        acceptable = len([c for c in case_results if c["claim_acceptable"]])
        with_precommitment = len(
            [c for c in case_results if c["has_precommitment"]]
        )
        return {
            "total": total,
            "completed": completed,
            "acceptable_claims": acceptable,
            "with_precommitment": with_precommitment,
        }

    def _compute_metrics(self, case_results: list[dict[str, Any]]) -> dict[str, Any]:
        if not case_results:
            return {"gate_f1_mean": 0.0, "claim_acceptance_rate": 0.0}
        f1_mean = sum(c["gate_f1"] for c in case_results) / len(case_results)
        claim_rate = (
            sum(1 for c in case_results if c["claim_acceptable"])
            / len(case_results)
        )
        return {
            "gate_f1_mean": round(f1_mean, 4),
            "claim_acceptance_rate": round(claim_rate, 4),
        }

    def _check_red_lines(
        self,
        case_results: list[dict[str, Any]],
        integrity: dict[str, Any],
    ) -> list[dict[str, Any]]:
        """Check red-line rules."""
        lines = []
        # RL-001: oracle isolation must be verified.
        lines.append({
            "rule": "RL-ORACLE-ISOLATION",
            "triggered": not integrity.get("oracle_isolated", False),
            "detail": "Oracle data must not be present in scenario bundle.",
        })
        # RL-002: precommitment required for all cases.
        missing_pre = [
            c["case_id"] for c in case_results if not c["has_precommitment"]
        ]
        lines.append({
            "rule": "RL-PRECOMMITMENT",
            "triggered": len(missing_pre) > 0,
            "detail": f"{len(missing_pre)} case(s) missing precommitment.",
        })
        # RL-003: no forbidden claims allowed.
        forbidden_hits = [
            c["case_id"] for c in case_results if c["claim_hit_forbidden"]
        ]
        lines.append({
            "rule": "RL-NO-FORBIDDEN-CLAIMS",
            "triggered": len(forbidden_hits) > 0,
            "detail": f"{len(forbidden_hits)} case(s) made forbidden claims.",
        })
        # RL-004: digest must be known.
        lines.append({
            "rule": "RL-DIGEST-KNOWN",
            "triggered": not self._profile.profile_digest,
            "detail": "System profile digest is required.",
        })
        return lines

    # ── internal: cluster / pair ──────────────────────────────────────────

    def _build_cluster_results(
        self, case_results: list[dict[str, Any]],
    ) -> list[dict[str, Any]]:
        """Aggregate case results by cluster."""
        if not self._cluster_map:
            return []
        clusters: dict[str, list[dict[str, Any]]] = {}
        for cr in case_results:
            cluster = self._cluster_map.get(cr["case_id"], "unclustered")
            clusters.setdefault(cluster, []).append(cr)
        return [
            {
                "cluster": name,
                "case_count": len(cases),
                "gate_f1_mean": round(
                    sum(c["gate_f1"] for c in cases) / len(cases), 4,
                ),
                "claim_acceptable": len(
                    [c for c in cases if c["claim_acceptable"]]
                ),
            }
            for name, cases in sorted(clusters.items())
        ]

    def _build_pair_results(
        self, case_results: list[dict[str, Any]],
    ) -> list[dict[str, Any]]:
        """Build contrast-pair results (historical vs countercase)."""
        if not self._pair_map:
            return []
        cr_index = {c["case_id"]: c for c in case_results}
        pairs = []
        for hist_id, counter_id in self._pair_map.items():
            hist = cr_index.get(hist_id)
            cntr = cr_index.get(counter_id)
            if hist and cntr:
                pairs.append({
                    "historical": hist_id,
                    "countercase": counter_id,
                    "historical_acceptable": hist["claim_acceptable"],
                    "countercase_acceptable": cntr["claim_acceptable"],
                    "contrast_correct": (
                        hist["claim_acceptable"] and not cntr["claim_acceptable"]
                    ),
                })
        return pairs

    # ── internal: integrity ───────────────────────────────────────────────

    def _build_integrity_block(self) -> dict[str, Any]:
        """Build the integrity verification block."""
        return {
            "oracle_isolated": True,  # verified in __init__
            "profile_digest": self._profile.profile_digest,
            "scenario_bundle_digest": sha256_bytes(
                canonical_bytes(self._scenarios)
            ),
            "oracle_bundle_digest": sha256_bytes(
                canonical_bytes(self._oracles)
            ),
            "journal_head_sha256": self._journal.head_sha256(),
            "responses_digest": sha256_bytes(
                canonical_bytes({
                    cid: r.response_digest
                    for cid, r in sorted(self._responses.items())
                })
            ),
        }

    # ── internal: status ──────────────────────────────────────────────────

    @staticmethod
    def _determine_status(
        integrity: dict[str, Any],
        red_lines: list[dict[str, Any]],
        counts: dict[str, Any],
    ) -> str:
        """Determine the evaluation run status."""
        if not integrity.get("oracle_isolated", False):
            return "invalid"
        if any(r["triggered"] and r["rule"] in (
            "RL-ORACLE-ISOLATION", "RL-DIGEST-KNOWN",
        ) for r in red_lines):
            return "invalid"
        if counts["completed"] < counts["total"]:
            return "descriptive_only"
        # Cap at descriptive_only — no human calibration yet.
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

    # Finalize: produce integrity block.
    # Since we haven't recorded any agent responses, scoring is incomplete.
    result = runner.finalize()
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
