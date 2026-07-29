"""Automatic compaction observer — GAK-CMP-001.

Provides simulation and validation of real-world automatic compaction
scenarios.  Distinguishes between:

1. **Manual compaction** — user-initiated, exact range boundaries known
2. **Automatic compaction** — threshold-triggered by the runtime when
   context exceeds a token budget; boundary classification is
   *inherently uncertain* for items near the threshold

The critical invariant (see Gap Register §9, GAK-CMP-001): automatic
threshold detection **must not silently promote unknown state** to
retained or discarded.  Items near the boundary that cannot be
definitively classified must remain ``unknown``.

All compaction metadata produced here is compatible with the
:class:`~.audit.AuditLedger` compaction schema used by
:func:`~.audit._normalize_compaction`.

Design reference: P4 audit compaction projection (§4),
Grok compaction provenance probe (integration/grok/).
"""

from __future__ import annotations

import math
import re
from dataclasses import dataclass, field
from typing import Any

from .errors import AssuranceError
from .utils import sha256_bytes, canonical_bytes

# ── safe identifiers (mirrors audit.py patterns) ──────────────────────────────

_SAFE_REASON: re.Pattern[str] = re.compile(r"^[A-Z][A-Z0-9_]{2,63}$")


# ── reason codes for automatic compaction ─────────────────────────────────────

class CompactionReason:
    """Canonical reason codes for compaction range disposition.

    These map to the ``reason_code`` field in :func:`~.audit._normalize_ranges`.
    """

    # ── retained ──
    PINNED_SOURCE_REFERENCE = "PINNED_SOURCE_REFERENCE"
    WITHIN_THRESHOLD_BOUNDARY = "WITHIN_THRESHOLD_BOUNDARY"
    CORE_CONTEXT_ITEM = "CORE_CONTEXT_ITEM"
    USER_EXPLICITLY_RETAINED = "USER_EXPLICITLY_RETAINED"

    # ── discarded ──
    BEYOND_THRESHOLD_BOUNDARY = "BEYOND_THRESHOLD_BOUNDARY"
    RAW_PAYLOAD_ARCHIVE_DELETE = "RAW_PAYLOAD_ARCHIVE_DELETE"
    DUPLICATE_CONTEXT = "DUPLICATE_CONTEXT"

    # ── unknown (automatic threshold uncertainty) ──
    AUTOMATIC_THRESHOLD_NOT_OBSERVED = "AUTOMATIC_THRESHOLD_NOT_OBSERVED"
    AUTOMATIC_THRESHOLD_OBSERVED = "AUTOMATIC_THRESHOLD_OBSERVED"
    THRESHOLD_BOUNDARY_UNCERTAIN = "THRESHOLD_BOUNDARY_UNCERTAIN"
    TOKEN_ESTIMATION_IMPRECISE = "TOKEN_ESTIMATION_IMPRECISE"


# ── data types ────────────────────────────────────────────────────────────────


@dataclass
class SourceItem:
    """A single item in a compaction source span.

    Represents one conversation turn / tool result / system message
    that may be retained, discarded, or fall into unknown during
    automatic compaction.
    """

    index: int
    role: str  # "system", "user", "assistant", "tool"
    estimated_tokens: int
    content_sha256: str
    is_core_context: bool = False          # system prompt / workspace rules
    is_user_pinned: bool = False           # explicitly saved by user


@dataclass
class CompactionRange:
    """One classified range within a compaction.

    Mirrors the ``ranges`` structure in
    :func:`~.audit._normalize_ranges`.
    """

    first_index: int
    last_index: int
    reason_code: str


@dataclass
class SimulatedCompaction:
    """Result of simulating a compaction event.

    Compatible with the ``compaction`` dict accepted by
    :meth:`AuditLedger.append`.
    """

    trigger: str                          # "automatic" | "manual"
    threshold_token_count: int | None     # None for manual triggers
    source_span: dict[str, Any]
    summary: dict[str, Any]
    ranges: dict[str, list[dict[str, Any]]]
    classification_confidence: float      # 0.0–1.0
    source_items: list[SourceItem] = field(default_factory=list)

    def to_audit_compaction(self) -> dict[str, Any]:
        """Produce the compaction dict for :meth:`AuditLedger.append`.

        The output passes :func:`~.audit._normalize_compaction` validation.
        """
        return {
            "summary_status": self.summary["status"],
            "summary_sha256": self.summary["sha256"],
            "source_span_sha256": self.source_span["sha256"],
            "ranges": {
                "retained": self.ranges.get("retained", []),
                "discarded": self.ranges.get("discarded", []),
                "unknown": self.ranges.get("unknown", []),
            },
        }

    @property
    def has_unknown_ranges(self) -> bool:
        """Return ``True`` if any items could not be definitively classified."""
        return len(self.ranges.get("unknown", [])) > 0


# ── automatic compaction simulator ────────────────────────────────────────────


class AutomaticCompactionSimulator:
    """Simulate automatic compaction triggered by a token threshold.

    The simulator models the real-world scenario where an agent runtime
    detects that the conversation context has exceeded a token budget
    (e.g. 8 000 tokens) and triggers automatic compaction.  Unlike
    manual compaction (where a user explicitly marks what to keep), the
    automatic threshold **cannot know with certainty** whether items
    near the boundary should be retained or discarded — those items are
    classified as ``unknown``.

    **Honesty invariant**: the simulator never promotes unknown state.
    Items within the *boundary zone* (near the threshold) that cannot
    be definitively classified are placed in ``unknown`` with
    reason code ``THRESHOLD_BOUNDARY_UNCERTAIN``.

    Parameters
    ----------
    threshold_tokens:
        Token budget that triggers automatic compaction.
    boundary_zone_ratio:
        Fraction of the threshold used as the boundary zone.  Items
        whose cumulative token position falls within
        ``threshold ± (threshold * boundary_zone_ratio)`` are
        classified as ``unknown``.  Default 0.05 (5 %).
    """

    def __init__(
        self,
        threshold_tokens: int = 8000,
        boundary_zone_ratio: float = 0.05,
    ) -> None:
        if threshold_tokens < 1:
            raise AssuranceError("threshold_tokens must be positive")
        if not (0.0 <= boundary_zone_ratio < 1.0):
            raise AssuranceError("boundary_zone_ratio must be in [0, 1)")
        self.threshold_tokens = threshold_tokens
        self.boundary_zone_ratio = boundary_zone_ratio
        self._boundary_half = int(threshold_tokens * boundary_zone_ratio)

    def simulate(
        self,
        source_items: list[SourceItem],
        *,
        core_context_roles: tuple[str, ...] = ("system",),
    ) -> SimulatedCompaction:
        """Simulate automatic compaction over *source_items*.

        Returns a :class:`SimulatedCompaction` with ranges classified
        by the automatic threshold.  Items near the boundary fall into
        ``unknown`` when their classification is uncertain.

        Parameters
        ----------
        source_items:
            Ordered list of conversation items to compact.
        core_context_roles:
            Roles treated as core context (always retained if within
            budget).  Default: ``("system",)``.

        Returns
        -------
        SimulatedCompaction
            The resulting compaction with audit-compatible ranges.
        """
        if not source_items:
            raise AssuranceError(
                "compaction source span must contain at least one item"
            )

        # Estimate whether threshold is exceeded
        total_tokens = sum(item.estimated_tokens for item in source_items)
        if total_tokens <= self.threshold_tokens:
            # Under threshold — no compaction needed; all retained
            return self._build_all_retained(source_items, total_tokens)

        # Over threshold — classify each item
        return self._classify_by_threshold(source_items, total_tokens)

    def _build_all_retained(
        self, items: list[SourceItem], total_tokens: int,
    ) -> SimulatedCompaction:
        """Build a compaction result where all items are retained."""
        return SimulatedCompaction(
            trigger="automatic",
            threshold_token_count=self.threshold_tokens,
            classification_confidence=1.0,
            source_span=self._build_source_span(items),
            summary=self._build_summary(items, total_tokens),
            ranges={
                "retained": [
                    {
                        "first_index": items[0].index,
                        "last_index": items[-1].index,
                        "reason_code": CompactionReason.WITHIN_THRESHOLD_BOUNDARY,
                    }
                ],
                "discarded": [],
                "unknown": [],
            },
            source_items=list(items),
        )

    def _classify_by_threshold(
        self, items: list[SourceItem], total_tokens: int,
    ) -> SimulatedCompaction:
        """Classify items relative to the automatic threshold."""
        retained: list[CompactionRange] = []
        discarded: list[CompactionRange] = []
        unknown: list[CompactionRange] = []

        cumulative = 0
        boundary_start = self.threshold_tokens - self._boundary_half
        boundary_end = self.threshold_tokens + self._boundary_half

        i = 0
        while i < len(items):
            item = items[i]
            item_start = cumulative
            item_end = cumulative + item.estimated_tokens
            cumulative = item_end

            # Core context is always retained when within budget
            if item.is_core_context or item.is_user_pinned:
                retained.append(CompactionRange(
                    first_index=item.index,
                    last_index=item.index,
                    reason_code=(
                        CompactionReason.USER_EXPLICITLY_RETAINED
                        if item.is_user_pinned
                        else CompactionReason.CORE_CONTEXT_ITEM
                    ),
                ))
                i += 1
                continue

            # Item straddles the boundary zone → unknown
            if (
                item_start < boundary_end
                and item_end > boundary_start
                and (
                    item_start >= boundary_start
                    or item_end <= boundary_end
                )
            ):
                # Check if we can definitively classify
                if item_end <= boundary_start:
                    # Entirely before boundary — definitively retained
                    retained.append(CompactionRange(
                        first_index=item.index,
                        last_index=item.index,
                        reason_code=CompactionReason.WITHIN_THRESHOLD_BOUNDARY,
                    ))
                elif item_start >= boundary_end:
                    # Entirely after boundary — definitively discarded
                    discarded.append(CompactionRange(
                        first_index=item.index,
                        last_index=item.index,
                        reason_code=CompactionReason.BEYOND_THRESHOLD_BOUNDARY,
                    ))
                else:
                    # Straddles or inside boundary — uncertain
                    unknown.append(CompactionRange(
                        first_index=item.index,
                        last_index=item.index,
                        reason_code=CompactionReason.THRESHOLD_BOUNDARY_UNCERTAIN,
                    ))
            elif item_end <= boundary_start:
                retained.append(CompactionRange(
                    first_index=item.index,
                    last_index=item.index,
                    reason_code=CompactionReason.WITHIN_THRESHOLD_BOUNDARY,
                ))
            else:
                discarded.append(CompactionRange(
                    first_index=item.index,
                    last_index=item.index,
                    reason_code=CompactionReason.BEYOND_THRESHOLD_BOUNDARY,
                ))
            i += 1

        # Merge adjacent ranges with the same reason code
        retained = _merge_adjacent_ranges(retained)
        discarded = _merge_adjacent_ranges(discarded)
        unknown = _merge_adjacent_ranges(unknown)

        all_classified = (
            sum(len(r) for r in [retained, discarded, unknown])
        )
        confidence = all_classified / len(items) if items else 1.0

        return SimulatedCompaction(
            trigger="automatic",
            threshold_token_count=self.threshold_tokens,
            classification_confidence=confidence,
            source_span=self._build_source_span(items),
            summary=self._build_summary(items, total_tokens),
            ranges={
                "retained": [
                    {"first_index": r.first_index, "last_index": r.last_index,
                     "reason_code": r.reason_code}
                    for r in retained
                ],
                "discarded": [
                    {"first_index": r.first_index, "last_index": r.last_index,
                     "reason_code": r.reason_code}
                    for r in discarded
                ],
                "unknown": [
                    {"first_index": r.first_index, "last_index": r.last_index,
                     "reason_code": r.reason_code}
                    for r in unknown
                ],
            },
            source_items=list(items),
        )

    def _build_source_span(
        self, items: list[SourceItem],
    ) -> dict[str, Any]:
        """Build the source_span metadata dict."""
        canonical = canonical_bytes([
            {"index": item.index, "role": item.role,
             "content_sha256": item.content_sha256}
            for item in items
        ])
        return {
            "kind": "conversation_context",
            "item_count": len(items),
            "first_index": items[0].index,
            "last_index": items[-1].index,
            "sha256": sha256_bytes(canonical),
        }

    def _build_summary(
        self, items: list[SourceItem], total_tokens: int,
    ) -> dict[str, Any]:
        """Build the summary metadata dict."""
        summary_text = (
            f"Automatic compaction: {len(items)} items, "
            f"~{total_tokens} tokens estimated. "
            f"Threshold: {self.threshold_tokens} tokens."
        )
        return {
            "status": "derived_unverified",
            "chars": len(summary_text),
            "sha256": sha256_bytes(summary_text.encode("utf-8")),
        }


# ── manual compaction simulator ───────────────────────────────────────────────


class ManualCompactionSimulator:
    """Simulate manual (user-initiated) compaction with explicit ranges.

    Manual compaction occurs when a user or operator explicitly decides
    to compact context.  Unlike automatic compaction, manual compaction
    can specify exact classification for every item — there are no
    boundary uncertainties.
    """

    @staticmethod
    def simulate(
        source_items: list[SourceItem],
        *,
        retained_indices: set[int] | None = None,
        discarded_indices: set[int] | None = None,
        unknown_indices: set[int] | None = None,
    ) -> SimulatedCompaction:
        """Simulate manual compaction with explicit index sets.

        Every source item index must appear in exactly one of the three
        sets.  Items not in any set are placed in ``unknown`` with
        reason ``AUTOMATIC_THRESHOLD_NOT_OBSERVED`` (manual trigger
        means we are not observing automatic behaviour here — a
        separate automatic simulation must cover that).

        Parameters
        ----------
        source_items:
            Ordered list of conversation items.
        retained_indices:
            Indices explicitly retained.
        discarded_indices:
            Indices explicitly discarded.
        unknown_indices:
            Indices left unknown.

        Returns
        -------
        SimulatedCompaction
            The resulting compaction with exact ranges.
        """
        if not source_items:
            raise AssuranceError(
                "compaction source span must contain at least one item"
            )

        retained = retained_indices or set()
        discarded = discarded_indices or set()
        unknown_idx = unknown_indices or set()

        all_indices = {item.index for item in source_items}
        classified = retained | discarded | unknown_idx

        # Items not explicitly classified go to unknown
        unclassified = all_indices - classified
        unknown_idx = unknown_idx | unclassified

        # Verify no overlap
        if retained & discarded or retained & unknown_idx or discarded & unknown_idx:
            raise AssuranceError(
                "manual compaction ranges overlap — each index must be in "
                "exactly one of retained/discarded/unknown"
            )

        total_tokens = sum(item.estimated_tokens for item in source_items)

        return SimulatedCompaction(
            trigger="manual",
            threshold_token_count=None,
            classification_confidence=(
                1.0 if not unclassified else
                len(classified) / len(all_indices)
            ),
            source_span=_build_span_from_items(source_items),
            summary={
                "status": "derived_unverified",
                "chars": 0,
                "sha256": sha256_bytes(b"manual-compaction-summary"),
            },
            ranges={
                "retained": _indices_to_ranges(
                    sorted(retained & all_indices),
                    CompactionReason.USER_EXPLICITLY_RETAINED,
                ),
                "discarded": _indices_to_ranges(
                    sorted(discarded & all_indices),
                    CompactionReason.RAW_PAYLOAD_ARCHIVE_DELETE,
                ),
                "unknown": _indices_to_ranges(
                    sorted(unknown_idx & all_indices),
                    CompactionReason.AUTOMATIC_THRESHOLD_NOT_OBSERVED,
                ),
            },
            source_items=list(source_items),
        )


# ── compaction invariant validators ───────────────────────────────────────────


def validate_compaction_non_overlap(
    ranges: dict[str, list[dict[str, Any]]],
) -> bool:
    """Validate that retained/discarded/unknown ranges do not overlap.

    Returns ``True`` if valid, ``False`` if overlap is detected.

    This is a standalone validator — it does not raise.  Use
    :func:`assert_compaction_ranges_valid` for the raising variant
    (which is called by :func:`~.audit._normalize_ranges`).
    """
    seen: set[int] = set()
    for disposition in ("retained", "discarded", "unknown"):
        for r in ranges.get(disposition, []):
            indices = set(range(r["first_index"], r["last_index"] + 1))
            if seen & indices:
                return False
            seen.update(indices)
    return True


def validate_compaction_classification_required(
    ranges: dict[str, list[dict[str, Any]]],
) -> bool:
    """Validate that at least one range is classified (non-empty)."""
    return any(ranges.get(d) for d in ("retained", "discarded", "unknown"))


def validate_summary_never_promoted(summary_status: str) -> bool:
    """Validate that the compaction summary is ``derived_unverified``.

    A derived summary can **never** be promoted to ``direct`` or
    ``observed`` — it is always at least one step removed from source
    evidence.
    """
    return summary_status == "derived_unverified"


def validate_unknown_not_promoted(
    original: dict[str, list[dict[str, Any]]],
    modified: dict[str, list[dict[str, Any]]],
) -> bool:
    """Validate that no unknown range has been silently reclassified.

    Every index that was ``unknown`` in *original* must still be
    ``unknown`` in *modified* — it cannot be silently promoted to
    ``retained`` or ``discarded``.

    Returns ``True`` if the invariant holds, ``False`` if promotion
    is detected.
    """
    original_unknown: set[int] = set()
    for r in original.get("unknown", []):
        original_unknown.update(range(r["first_index"], r["last_index"] + 1))

    modified_unknown: set[int] = set()
    for r in modified.get("unknown", []):
        modified_unknown.update(range(r["first_index"], r["last_index"] + 1))

    # Every originally-unknown index must still be unknown
    return original_unknown <= modified_unknown


def validate_automatic_trigger_honesty(
    compaction: SimulatedCompaction,
) -> bool:
    """Validate that an automatic trigger does not fake precision.

    When ``classification_confidence < 1.0``, the compaction MUST have
    at least one ``unknown`` range — otherwise it is claiming certainty
    it does not possess.

    When ``classification_confidence == 1.0``, having no unknown ranges
    is acceptable (the simulator was able to definitively classify
    every item).
    """
    if compaction.trigger != "automatic":
        return True  # manual triggers have different semantics

    if compaction.classification_confidence < 1.0:
        # Must have at least one unknown range or explicit unknown reason
        return compaction.has_unknown_ranges

    # confidence == 1.0 → no unknown required
    return True


def assert_compaction_ranges_valid(
    ranges: dict[str, list[dict[str, Any]]],
) -> None:
    """Assert all compaction range invariants hold, raising on failure.

    This is a convenience wrapper that calls the three main validators
    and raises :class:`AssuranceError` with a descriptive message on
    failure.  Use this in test assertions and in production validation
    paths.
    """
    if not validate_compaction_non_overlap(ranges):
        raise AssuranceError(
            "compaction ranges overlap — each index must be classified "
            "in exactly one of retained/discarded/unknown"
        )
    if not validate_compaction_classification_required(ranges):
        raise AssuranceError(
            "compaction must classify at least one source range"
        )


# ── helpers ───────────────────────────────────────────────────────────────────


def _build_span_from_items(
    items: list[SourceItem],
) -> dict[str, Any]:
    """Build a source_span dict from SourceItems."""
    canonical = canonical_bytes([
        {"index": item.index, "role": item.role,
         "content_sha256": item.content_sha256}
        for item in items
    ])
    return {
        "kind": "conversation_context",
        "item_count": len(items),
        "first_index": items[0].index,
        "last_index": items[-1].index,
        "sha256": sha256_bytes(canonical),
    }


def _indices_to_ranges(
    sorted_indices: list[int],
    reason_code: str,
) -> list[dict[str, Any]]:
    """Convert a sorted list of indices to contiguous range dicts."""
    if not sorted_indices:
        return []
    ranges: list[dict[str, Any]] = []
    start = sorted_indices[0]
    prev = start
    for idx in sorted_indices[1:]:
        if idx == prev + 1:
            prev = idx
        else:
            ranges.append({
                "first_index": start,
                "last_index": prev,
                "reason_code": reason_code,
            })
            start = idx
            prev = idx
    ranges.append({
        "first_index": start,
        "last_index": prev,
        "reason_code": reason_code,
    })
    return ranges


def _merge_adjacent_ranges(
    ranges: list[CompactionRange],
) -> list[CompactionRange]:
    """Merge adjacent ranges that share the same reason code."""
    if not ranges:
        return []
    merged: list[CompactionRange] = [ranges[0]]
    for r in ranges[1:]:
        last = merged[-1]
        if (
            r.reason_code == last.reason_code
            and r.first_index == last.last_index + 1
        ):
            merged[-1] = CompactionRange(
                first_index=last.first_index,
                last_index=r.last_index,
                reason_code=last.reason_code,
            )
        else:
            merged.append(r)
    return merged


def estimate_tokens(text: str) -> int:
    """Estimate token count for *text* using a simple heuristic.

    This is a rough approximation (≈ characters / 3.5 for English,
    ≈ characters / 2.0 for code-heavy content).  Real tokenizers
    produce different counts; the simulator uses this estimate and
    **declares the imprecision** via the boundary zone mechanism.

    The estimate is deliberately conservative — it errs on the side of
    over-estimation to avoid falsely claiming an item is within budget.
    """
    if not text:
        return 0
    # Conservative: assume ~2.5 chars per token (blend of code + English)
    return max(1, math.ceil(len(text) / 2.5))
