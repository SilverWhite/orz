"""Diagnostic Coverage Check — neutral progressive pullback during debug stagnation.

Design constraint from ``CN_AGENT_BASE_GRAFT_VALUE_REVIEW_2026-07-30.md`` §7.3.

This module belongs to the **neutral inquiry** family (not counterexample gate,
not global review).  It provides:

- :class:`DiagnosticCoverageState` — per-bug state machine
- :func:`build_diagnostic_coverage_check` — build the trigger checkpoint
- :func:`evaluate_diagnostic_coverage_response` — verify the response
"""

from __future__ import annotations

from typing import Any, Sequence

from .contracts import validate_contract
from .errors import AssuranceError
from .utils import canonical_bytes, sha256_bytes, utc_now


RECEIPT_SCHEMA = "diagnostic-coverage-check-receipt-v0.1.schema.json"
VERIFICATION_SCHEMA = "diagnostic-coverage-check-verification-v0.1.schema.json"

# ── hard-signal type taxonomy (from CN §7.3) ──

HARD_SIGNAL_TYPES = frozenset({
    "consecutive_same_failure",
    "repeated_pattern",
    "same_module_no_evidence",
    "unabsorbed_new_evidence",
    "large_scope_low_diag",
    "key_surface_unexamined",
})

# Hard-signal descriptions used in message blocks
_HARD_SIGNAL_LABELS: dict[str, str] = {
    "consecutive_same_failure": "同一问题连续失败",
    "repeated_pattern": "同一测试或命令失败形态重复",
    "same_module_no_evidence": "同一模块连续修改但无新证据",
    "unabsorbed_new_evidence": "新错误类别/stack trace 出现但当前计划未吸收",
    "large_scope_low_diag": "准备大范围改动且当前诊断证据不足 2 类",
    "key_surface_unexamined": "关键日志/trace/source/边界条件仍未查看",
}

# ── thresholds (from CN §7.3) ──

INITIAL_THRESHOLD = 2
MAX_THRESHOLD = 5

# ── response fields ──

ALLOWED_DIAGNOSTIC_RESPONSE_FIELDS = [
    "decision",
    "covered_diagnostic_surfaces",
    "missing_diagnostic_surfaces",
    "next_step",
]

FORBIDDEN_DIAGNOSTIC_RESPONSE_FIELDS = [
    "counterexample_candidate",
    "claim_disposition",
    "claim_promotion",
    "root_cause_certain",
    "global_review_requested",
]

DIAGNOSTIC_MESSAGE_BLOCK = """\
[DIAGNOSTIC_COVERAGE_CHECK v0.1]
继续沿当前路线前，关键诊断面是否已经覆盖到足以选择下一步？

已记录的硬信号:
{signal_summary}

已覆盖的诊断面:
{covered_summary}

仍缺失的诊断面:
{missing_summary}

请回答 yes / no / uncertain，并列出已覆盖诊断面、仍缺诊断面和下一步（最多一个最小补诊断动作）。
[/DIAGNOSTIC_COVERAGE_CHECK]"""


# ═══════════════════════════════════════════════════════════════════
# DiagnosticCoverageState — per-bug state machine
# ═══════════════════════════════════════════════════════════════════


class DiagnosticCoverageState:
    """Per-bug progressive diagnostic-coverage state machine.

    Scope: a single bug / single debug episode.
    Maintains a hard-signal counter and a progressive threshold.

    Initial threshold: 2
    Threshold sequence: 2 → 3 → 4 → 5 (capped)

    When ``record_hard_signal`` causes the counter to reach the
    threshold the caller should invoke :func:`build_diagnostic_coverage_check`
    and then call :meth:`acknowledge_trigger` to advance the threshold.

    ADR-0010 §4.6.3 (V11-IMPL-002): the 0.5 partial-signal weight is
    abolished — it was a subjective noise-reduction that could not be
    stably replayed. New evidence enters as a NEW hard signal bound to an
    evidence identity / failure fingerprint. ``record_soft_signal`` is
    retained for audit history only (weight 0, never counts — "absorbed
    into plan" is not mechanically verifiable and must not participate).
    """

    def __init__(self, bug_id: str) -> None:
        if not bug_id or not bug_id.strip():
            raise AssuranceError("bug_id must be non-empty")
        self._bug_id = bug_id.strip()
        self._hard_signal_count: float = 0.0
        self._current_threshold: int = INITIAL_THRESHOLD
        self._trigger_count: int = 0
        self._signals: list[dict[str, Any]] = []
        self._covered_surfaces: list[str] = []
        self._missing_surfaces: list[str] = []

    # ── read-only properties ──

    @property
    def bug_id(self) -> str:
        return self._bug_id

    @property
    def hard_signal_count(self) -> float:
        return self._hard_signal_count

    @property
    def current_threshold(self) -> int:
        return self._current_threshold

    @property
    def trigger_count(self) -> int:
        return self._trigger_count

    @property
    def signals(self) -> list[dict[str, Any]]:
        return list(self._signals)

    @property
    def covered_surfaces(self) -> list[str]:
        return list(self._covered_surfaces)

    @property
    def missing_surfaces(self) -> list[str]:
        return list(self._missing_surfaces)

    # ── signal recording ──

    def record_hard_signal(
        self, *, signal_type: str, description: str,
    ) -> bool:
        """Record a hard signal (weight 1.0).

        Returns ``True`` when the cumulative count reaches or exceeds
        the current threshold — the caller should trigger a diagnostic
        coverage check.
        """
        if signal_type not in HARD_SIGNAL_TYPES:
            raise AssuranceError(
                f"unknown hard_signal type: {signal_type}; "
                f"must be one of {sorted(HARD_SIGNAL_TYPES)}"
            )
        return self._record(signal_type, description, weight=1.0)

    def record_soft_signal(
        self, *, description: str,
    ) -> bool:
        """Record a soft signal (weight 0 — audit history only).

        ADR-0010 §4.6.3 (V11-IMPL-002): whether new evidence was
        "absorbed into the plan" cannot be mechanically verified, so it
        must NOT participate in the counter. The signal is still appended
        to the history for audit. New evidence that IS pointable enters
        as a new hard signal (bound to an evidence identity / failure
        fingerprint). "I thought about it again" is NOT eligible.

        Always returns ``False`` (never triggers on its own).
        """
        self._signals.append({
            "signal_type": "new_evidence_acknowledged",
            "description": description.strip(),
            "weight": 0.0,
            "recorded_at": utc_now(),
        })
        return False

    # ── diagnostic surfaces ──

    def set_diagnostic_surfaces(
        self,
        *,
        covered: Sequence[str],
        missing: Sequence[str],
    ) -> None:
        """Declare which diagnostic surfaces have been covered / are still missing.

        These are free-form labels chosen by the operator / calling
        code — e.g. ``["logs", "trace", "source inspection"]``.
        """
        self._covered_surfaces = list(covered)
        self._missing_surfaces = list(missing)

    # ── life-cycle ──

    def acknowledge_trigger(self) -> None:
        """Advance the threshold after a check fires.

        - Resets the hard-signal counter to 0.
        - Increments *current_threshold* (capped at 5).
        - Increments *trigger_count*.
        """
        self._hard_signal_count = 0.0
        self._trigger_count += 1
        if self._current_threshold < MAX_THRESHOLD:
            self._current_threshold += 1

    def resolve(self) -> None:
        """Reset the state machine — bug is resolved.

        - Hard-signal counter → 0.
        - Threshold → 2 (initial).
        - Signal history cleared.
        - Trigger count → 0.
        - Diagnostic surfaces cleared.
        """
        self._hard_signal_count = 0.0
        self._current_threshold = INITIAL_THRESHOLD
        self._signals.clear()
        self._trigger_count = 0
        self._covered_surfaces.clear()
        self._missing_surfaces.clear()

    def is_threshold_reached(self) -> bool:
        """Return ``True`` when a check should fire."""
        return self._hard_signal_count >= self._current_threshold

    def snapshot(self) -> dict[str, Any]:
        """Return a serialisable snapshot of the current state."""
        return {
            "bug_id": self._bug_id,
            "hard_signal_count": self._hard_signal_count,
            "current_threshold": self._current_threshold,
            "trigger_count": self._trigger_count,
            "signals": list(self._signals),
            "covered_surfaces": list(self._covered_surfaces),
            "missing_surfaces": list(self._missing_surfaces),
        }

    # ── internal ──

    def _record(
        self, signal_type: str, description: str, *, weight: float,
    ) -> bool:
        self._signals.append({
            "signal_type": signal_type,
            "description": description.strip(),
            "weight": weight,
            "recorded_at": utc_now(),
        })
        self._hard_signal_count += weight
        return self.is_threshold_reached()


# ═══════════════════════════════════════════════════════════════════
# Checkpoint builder
# ═══════════════════════════════════════════════════════════════════


def build_diagnostic_coverage_check(
    *,
    bug_id: str,
    hard_signal_count: float,
    current_threshold: int,
    trigger_count: int,
    signals: Sequence[dict[str, Any]],
    covered_surfaces: Sequence[str],
    missing_surfaces: Sequence[str],
) -> dict[str, Any]:
    """Build a diagnostic coverage check receipt for the given bug state.

    This is the trigger artifact — it should be presented to the
    operator / agent when :meth:`DiagnosticCoverageState.record_hard_signal`
    returns ``True``.

    Args:
        bug_id: Identifier for the current bug / debug episode.
        hard_signal_count: Current weighted hard-signal count.
        current_threshold: Threshold that was just reached.
        trigger_count: Number of times this check has already fired.
        signals: The recorded signal history.
        covered_surfaces: Diagnostic surfaces already examined.
        missing_surfaces: Diagnostic surfaces still unexamined.
    """
    if current_threshold < 2 or current_threshold > 5:
        raise AssuranceError(
            f"current_threshold must be between 2 and 5, got {current_threshold}"
        )
    if trigger_count < 0:
        raise AssuranceError("trigger_count must be non-negative")

    signal_lines: list[str] = []
    for i, sig in enumerate(signals):
        label = _HARD_SIGNAL_LABELS.get(sig["signal_type"], sig["signal_type"])
        signal_lines.append(
            f"  {i + 1}. [{sig['weight']}] {label}: {sig['description']}"
        )
    signal_summary = (
        "\n".join(signal_lines) if signal_lines else "  (无已记录硬信号)"
    )

    covered_list = list(covered_surfaces)
    covered_summary = (
        "\n".join(f"  - {s}" for s in covered_list)
        if covered_list
        else "  (无已覆盖诊断面)"
    )

    missing_list = list(missing_surfaces)
    missing_summary = (
        "\n".join(f"  - {s}" for s in missing_list)
        if missing_list
        else "  (未声明缺失诊断面)"
    )

    message_block = DIAGNOSTIC_MESSAGE_BLOCK.format(
        signal_summary=signal_summary,
        covered_summary=covered_summary,
        missing_summary=missing_summary,
    )

    checkpoint_id = f"DIAG-COV-{bug_id}-{trigger_count}"
    checkpoint = {
        "schema_version": "0.1.0-draft",
        "checkpoint_kind": "diagnostic_coverage_check",
        "checkpoint_id": checkpoint_id,
        "bug_id": bug_id,
        "trigger": {
            "hard_signal_count": hard_signal_count,
            "current_threshold": current_threshold,
            "trigger_count": trigger_count,
            "signals": [
                {
                    "signal_type": s["signal_type"],
                    "description": s["description"],
                    "weight": s["weight"],
                    "recorded_at": s["recorded_at"],
                }
                for s in signals
            ],
        },
        "message_block": message_block,
        "diagnostic_surfaces": {
            "covered": covered_list,
            "missing": missing_list,
        },
        "allowed_response_fields": ALLOWED_DIAGNOSTIC_RESPONSE_FIELDS,
        "forbidden_response_fields": FORBIDDEN_DIAGNOSTIC_RESPONSE_FIELDS,
        "claim_policy": {
            "may_generate_counterexample_candidate": False,
            "may_set_claim_disposition": False,
            "claim_strength_effect": "none",
            "may_enter_global_review": False,
        },
        "notes": [
            "Neutral diagnostic coverage check only — does not evaluate correctness.",
            "No/uncertain responses must propose at most one minimal next diagnostic action.",
            "This check must not expand into a large re-audit or global review.",
            "User 'continue' does not clear or suppress the underlying trigger state.",
        ],
    }
    validate_contract(
        checkpoint,
        RECEIPT_SCHEMA,
        label="diagnostic coverage check",
    )
    return checkpoint


def build_diagnostic_coverage_check_from_state(
    state: DiagnosticCoverageState,
) -> dict[str, Any]:
    """Convenience wrapper that builds a check from a :class:`DiagnosticCoverageState`.

    Use this when the state machine is directly available; otherwise
    call :func:`build_diagnostic_coverage_check` with explicit values.
    """
    return build_diagnostic_coverage_check(
        bug_id=state.bug_id,
        hard_signal_count=state.hard_signal_count,
        current_threshold=state.current_threshold,
        trigger_count=state.trigger_count,
        signals=state.signals,
        covered_surfaces=state.covered_surfaces,
        missing_surfaces=state.missing_surfaces,
    )


# ═══════════════════════════════════════════════════════════════════
# Response evaluator
# ═══════════════════════════════════════════════════════════════════


def evaluate_diagnostic_coverage_response(
    *,
    check: dict[str, Any],
    response: dict[str, Any],
) -> dict[str, Any]:
    """Evaluate a response to a diagnostic coverage check.

    Validates that the response:

    - Contains only allowed fields (no counterexample / claim / global-review).
    - Has a ``decision`` of ``"yes"``, ``"no"``, or ``"uncertain"``.
    - If ``"no"`` or ``"uncertain"``, proposes at most **one** minimal
      next diagnostic action.

    Args:
        check: The checkpoint receipt from :func:`build_diagnostic_coverage_check`.
        response: The operator / agent response dict.

    Returns:
        A verification receipt (schema:
        ``diagnostic-coverage-check-verification-v0.1.schema.json``).
    """
    validate_contract(
        check,
        RECEIPT_SCHEMA,
        label="diagnostic coverage check",
    )

    allowed = set(check["allowed_response_fields"])
    forbidden = set(check["forbidden_response_fields"])
    present = set(response)

    forbidden_present = sorted(present & forbidden)
    unexpected_present = sorted(present - allowed - forbidden)

    decision = response.get("decision")
    _raw_decision = decision
    if decision not in ("yes", "no", "uncertain"):
        # Invalid or missing decision makes the response invalid, but
        # "decision" is an *allowed* field name — it cannot appear in
        # forbidden_fields_observed.  We report the raw value via
        # unexpected_fields_observed instead.
        if decision is not None:
            unexpected_present.append(f"decision:{decision!r}")
        else:
            unexpected_present.append("decision:missing")
        # Normalise to a schema-acceptable value so the receipt passes
        # its own schema validation even when valid=False.
        decision = "uncertain"

    # Count next steps — at most 1 when decision is no / uncertain.
    next_step_raw = response.get("next_step")
    next_step_count = 0
    if isinstance(next_step_raw, str) and next_step_raw.strip():
        next_step_count = 1
    elif isinstance(next_step_raw, list):
        next_step_count = len(next_step_raw)
    elif isinstance(next_step_raw, dict):
        next_step_count = 1  # single structured step

    next_step_bounded = next_step_count <= 1

    valid = (
        not forbidden_present
        and not unexpected_present
        and decision in ("yes", "no", "uncertain")
    )

    receipt = {
        "schema_version": "0.1.0-draft",
        "receipt_kind": "diagnostic_coverage_check_verification",
        "valid": valid,
        "checkpoint_id": check["checkpoint_id"],
        "checkpoint_sha256": sha256_bytes(canonical_bytes(check)),
        "response_sha256": sha256_bytes(canonical_bytes(response)),
        "response_decision": decision,
        "allowed_fields_observed": sorted(present & allowed),
        "forbidden_fields_observed": forbidden_present,
        "unexpected_fields_observed": unexpected_present,
        "next_step_count": next_step_count,
        "checks": {
            "neutral_diagnostic_only": valid,
            "no_counterexample_candidate": "counterexample_candidate" not in present,
            "no_claim_disposition": "claim_disposition" not in present,
            "no_claim_strength_effect": True,
            "no_global_review": "global_review_requested" not in present,
            "next_step_bounded": next_step_bounded,
        },
    }
    validate_contract(
        receipt,
        VERIFICATION_SCHEMA,
        label="diagnostic coverage check verification",
    )
    return receipt
