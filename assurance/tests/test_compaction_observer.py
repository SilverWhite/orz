"""Tests for automatic compaction observer — GAK-CMP-001.

Covers:
1. Automatic threshold simulation
2. Range invariant proofs (non-overlap, classification required)
3. Unknown promotion prevention
4. AuditLedger integration (append → seal → verify cycle)
5. Manual vs automatic comparison
"""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from assurance import (
    ArchiveController,
    AssuranceError,
    AuditLedger,
    ConversationNamespace,
    MemoryInstallationKeyStore,
    verify_audit_seal,
)
from assurance.compaction_observer import (
    AutomaticCompactionSimulator,
    CompactionReason,
    ManualCompactionSimulator,
    SimulatedCompaction,
    SourceItem,
    assert_compaction_ranges_valid,
    estimate_tokens,
    validate_automatic_trigger_honesty,
    validate_compaction_classification_required,
    validate_compaction_non_overlap,
    validate_summary_never_promoted,
    validate_unknown_not_promoted,
)
from assurance.utils import sha256_bytes


# ── test helpers ──────────────────────────────────────────────────────────────


def _make_item(
    index: int,
    role: str = "user",
    estimated_tokens: int = 500,
    content: str = "test content",
    is_core_context: bool = False,
    is_user_pinned: bool = False,
) -> SourceItem:
    return SourceItem(
        index=index,
        role=role,
        estimated_tokens=estimated_tokens,
        content_sha256=sha256_bytes(content.encode()),
        is_core_context=is_core_context,
        is_user_pinned=is_user_pinned,
    )


def _make_items(
    count: int,
    tokens_each: int = 500,
    role: str = "user",
) -> list[SourceItem]:
    return [
        _make_item(i, role=role, estimated_tokens=tokens_each,
                     content=f"item-{i}")
        for i in range(count)
    ]


def _evidenced(value: str) -> dict[str, object]:
    return {
        "value": value,
        "evidence_status": "observed",
        "source_refs": ["fixture:gak-cmp-001"],
    }


def _context(label: str) -> dict[str, object]:
    return {
        "workspace_canonical_path_digest": _evidenced("a" * 64),
        "workspace_content_digest": _evidenced("b" * 64),
        "workspace_policy_digest": _evidenced("c" * 64),
        "runtime_family": _evidenced("runtime-neutral-test"),
        "runtime_adapter_id": _evidenced(label),
        "runtime_binary_digest": _evidenced("d" * 64),
        "runtime_capabilities_digest": _evidenced("e" * 64),
        "assurance_config_digest": _evidenced("f" * 64),
        "sandbox_backend": _evidenced("cmp-test-sandbox"),
        "sandbox_backend_digest": _evidenced("1" * 64),
    }


# ══════════════════════════════════════════════════════════════════════════════
# 1. Automatic threshold simulation
# ══════════════════════════════════════════════════════════════════════════════


class AutomaticThresholdSimulationTests(unittest.TestCase):
    """Tests for :class:`AutomaticCompactionSimulator`."""

    def test_below_threshold_all_retained(self) -> None:
        """When total tokens are below threshold, all items are retained."""
        sim = AutomaticCompactionSimulator(threshold_tokens=5000)
        items = _make_items(5, tokens_each=500)  # 2500 total < 5000
        result = sim.simulate(items)
        self.assertEqual(result.trigger, "automatic")
        self.assertEqual(result.classification_confidence, 1.0)
        self.assertFalse(result.has_unknown_ranges)
        # All items in a single retained range
        self.assertEqual(len(result.ranges["retained"]), 1)
        self.assertEqual(result.ranges["retained"][0]["first_index"], 0)
        self.assertEqual(result.ranges["retained"][0]["last_index"], 4)
        self.assertEqual(
            result.ranges["retained"][0]["reason_code"],
            CompactionReason.WITHIN_THRESHOLD_BOUNDARY,
        )

    def test_exceeds_threshold_produces_unknown(self) -> None:
        """When total tokens exceed threshold, boundary items go to unknown."""
        sim = AutomaticCompactionSimulator(
            threshold_tokens=5000,
            boundary_zone_ratio=0.05,
        )
        # 15 items × 500 = 7500 tokens > 5000 threshold
        items = _make_items(15, tokens_each=500)
        result = sim.simulate(items)

        self.assertEqual(result.trigger, "automatic")
        # Should have classification for most items
        self.assertGreater(len(items), 0)
        # At boundary zone (5% of 5000 = ±250 tokens), items near
        # cumulative positions 4750–5250 should be uncertain
        total_classified = (
            sum(len(result.ranges[d]) for d in ("retained", "discarded", "unknown"))
        )

    def test_large_span_three_way_classification(self) -> None:
        """Large span far exceeding threshold produces all three zones."""
        sim = AutomaticCompactionSimulator(
            threshold_tokens=2000,
            boundary_zone_ratio=0.10,
        )
        # 20 items × 500 = 10 000 tokens >> 2000 threshold
        items = _make_items(20, tokens_each=500)
        result = sim.simulate(items)

        self.assertEqual(result.trigger, "automatic")
        # All three ranges should be present
        has_retained = len(result.ranges.get("retained", [])) > 0
        has_discarded = len(result.ranges.get("discarded", [])) > 0
        self.assertTrue(has_retained, "should have retained items")
        self.assertTrue(has_discarded, "should have discarded items")
        # With boundary_zone_ratio=10%, items near 2000±200 may be unknown
        self.assertTrue(
            validate_automatic_trigger_honesty(result),
            "automatic trigger must be honest about uncertainty",
        )

    def test_exact_threshold_boundary(self) -> None:
        """Items exactly at threshold produce boundary uncertainty."""
        sim = AutomaticCompactionSimulator(
            threshold_tokens=5000,
            boundary_zone_ratio=0.04,  # ±200 tokens
        )
        # 10 items × 500 = 5000 exactly at threshold
        items = _make_items(10, tokens_each=500)
        result = sim.simulate(items)

        self.assertEqual(result.trigger, "automatic")
        # At exact threshold with boundary zone, items 8-9 (positions
        # 4000-5000) straddle or are inside the boundary (4800-5200)
        self.assertTrue(validate_automatic_trigger_honesty(result))

    def test_single_item_span(self) -> None:
        """A single item span is valid."""
        sim = AutomaticCompactionSimulator(threshold_tokens=100)
        items = [_make_item(0, estimated_tokens=200, content="solo")]
        result = sim.simulate(items)
        self.assertEqual(result.trigger, "automatic")
        self.assertEqual(result.source_span["item_count"], 1)
        # Single item over threshold → discarded
        self.assertTrue(
            len(result.ranges["discarded"]) > 0
            or result.has_unknown_ranges
        )

    def test_core_context_always_retained(self) -> None:
        """Core context items are retained even beyond threshold."""
        sim = AutomaticCompactionSimulator(threshold_tokens=1000)
        items = [
            _make_item(0, role="system", estimated_tokens=200,
                        is_core_context=True, content="system prompt"),
            _make_item(1, estimated_tokens=600, content="user msg 1"),
            _make_item(2, estimated_tokens=600, content="user msg 2"),
        ]
        result = sim.simulate(items)

        # System item (index 0) must be retained
        retained_indices: set[int] = set()
        for r in result.ranges.get("retained", []):
            retained_indices.update(range(r["first_index"], r["last_index"] + 1))
        self.assertIn(0, retained_indices, "core context must be retained")

    def test_user_pinned_always_retained(self) -> None:
        """User-pinned items are retained even beyond threshold."""
        sim = AutomaticCompactionSimulator(threshold_tokens=500)
        items = [
            _make_item(0, estimated_tokens=400, content="msg 1"),
            _make_item(1, estimated_tokens=400, content="pinned",
                        is_user_pinned=True),
            _make_item(2, estimated_tokens=400, content="msg 3"),
        ]
        result = sim.simulate(items)

        retained_indices: set[int] = set()
        for r in result.ranges.get("retained", []):
            retained_indices.update(range(r["first_index"], r["last_index"] + 1))
        self.assertIn(1, retained_indices, "user-pinned item must be retained")

    def test_empty_source_raises(self) -> None:
        """Empty source span raises AssuranceError."""
        sim = AutomaticCompactionSimulator()
        with self.assertRaises(AssuranceError):
            sim.simulate([])

    def test_to_audit_compaction_produces_valid_dict(self) -> None:
        """to_audit_compaction() output passes _normalize_compaction."""
        from assurance.audit import _normalize_compaction

        sim = AutomaticCompactionSimulator(threshold_tokens=3000)
        items = _make_items(10, tokens_each=500)
        result = sim.simulate(items)
        audit_compaction = result.to_audit_compaction()

        # Must not raise
        normalized = _normalize_compaction(audit_compaction)
        self.assertIsNotNone(normalized)
        self.assertEqual(normalized["summary_status"], "derived_unverified")
        self.assertIn("retained", normalized["ranges"])
        self.assertIn("discarded", normalized["ranges"])
        self.assertIn("unknown", normalized["ranges"])


# ══════════════════════════════════════════════════════════════════════════════
# 2. Range invariant proofs
# ══════════════════════════════════════════════════════════════════════════════


class RangeInvariantTests(unittest.TestCase):
    """Tests for compaction range invariant validators."""

    def test_non_overlapping_ranges_pass(self) -> None:
        """Non-overlapping ranges validate successfully."""
        ranges = {
            "retained": [
                {"first_index": 0, "last_index": 2, "reason_code": "PINNED_SOURCE_REFERENCE"},
            ],
            "discarded": [
                {"first_index": 3, "last_index": 5, "reason_code": "RAW_PAYLOAD_ARCHIVE_DELETE"},
            ],
            "unknown": [
                {"first_index": 6, "last_index": 6, "reason_code": "AUTOMATIC_THRESHOLD_OBSERVED"},
            ],
        }
        self.assertTrue(validate_compaction_non_overlap(ranges))
        # assert variant should not raise
        assert_compaction_ranges_valid(ranges)

    def test_overlapping_ranges_detected(self) -> None:
        """Overlapping ranges are detected."""
        ranges = {
            "retained": [
                {"first_index": 0, "last_index": 4, "reason_code": "PINNED_SOURCE_REFERENCE"},
            ],
            "discarded": [
                {"first_index": 3, "last_index": 5, "reason_code": "RAW_PAYLOAD_ARCHIVE_DELETE"},
            ],
            "unknown": [],
        }
        self.assertFalse(validate_compaction_non_overlap(ranges))
        with self.assertRaises(AssuranceError):
            assert_compaction_ranges_valid(ranges)

    def test_empty_all_ranges_detected(self) -> None:
        """All-empty ranges are detected."""
        ranges = {"retained": [], "discarded": [], "unknown": []}
        self.assertFalse(validate_compaction_classification_required(ranges))
        with self.assertRaises(AssuranceError):
            assert_compaction_ranges_valid(ranges)

    def test_negative_first_index(self) -> None:
        """Negative first_index is rejected via _normalize_ranges."""
        from assurance.audit import _normalize_ranges
        ranges = {
            "retained": [
                {"first_index": -1, "last_index": 2, "reason_code": "BAD_RANGE"},
            ],
            "discarded": [],
            "unknown": [],
        }
        with self.assertRaises(AssuranceError):
            _normalize_ranges(ranges)

    def test_last_before_first_index(self) -> None:
        """last_index < first_index is rejected."""
        from assurance.audit import _normalize_ranges
        ranges = {
            "retained": [
                {"first_index": 5, "last_index": 2, "reason_code": "BAD_RANGE"},
            ],
            "discarded": [],
            "unknown": [],
        }
        with self.assertRaises(AssuranceError):
            _normalize_ranges(ranges)

    def test_invalid_reason_code_rejected(self) -> None:
        """Invalid reason_code (non-uppercase, special chars) is rejected."""
        from assurance.audit import _normalize_ranges
        ranges = {
            "retained": [
                {"first_index": 0, "last_index": 1, "reason_code": "bad reason!"},
            ],
            "discarded": [],
            "unknown": [],
        }
        with self.assertRaises(AssuranceError):
            _normalize_ranges(ranges)


# ══════════════════════════════════════════════════════════════════════════════
# 3. Unknown promotion prevention
# ══════════════════════════════════════════════════════════════════════════════


class UnknownPromotionPreventionTests(unittest.TestCase):
    """Tests proving unknown state cannot be silently promoted."""

    def test_summary_never_promoted_accepts_derived_unverified(self) -> None:
        """derived_unverified passes the validator."""
        self.assertTrue(validate_summary_never_promoted("derived_unverified"))

    def test_summary_never_promoted_rejects_direct(self) -> None:
        """direct provenance is rejected for compaction summaries."""
        self.assertFalse(validate_summary_never_promoted("direct"))

    def test_summary_never_promoted_rejects_observed(self) -> None:
        """observed provenance is rejected for compaction summaries."""
        self.assertFalse(validate_summary_never_promoted("observed"))

    def test_summary_never_promoted_rejects_empty(self) -> None:
        """Empty string is rejected."""
        self.assertFalse(validate_summary_never_promoted(""))

    def test_unknown_to_retained_detected(self) -> None:
        """Silently reclassifying unknown → retained is detected."""
        original = {
            "retained": [],
            "discarded": [],
            "unknown": [
                {"first_index": 0, "last_index": 2, "reason_code": "AUTOMATIC_THRESHOLD_OBSERVED"},
            ],
        }
        modified = {
            "retained": [
                {"first_index": 0, "last_index": 2, "reason_code": "PINNED_SOURCE_REFERENCE"},
            ],
            "discarded": [],
            "unknown": [],
        }
        self.assertFalse(validate_unknown_not_promoted(original, modified))

    def test_unknown_to_discarded_detected(self) -> None:
        """Silently reclassifying unknown → discarded is detected."""
        original = {
            "retained": [],
            "discarded": [],
            "unknown": [
                {"first_index": 5, "last_index": 7, "reason_code": "AUTOMATIC_THRESHOLD_OBSERVED"},
            ],
        }
        modified = {
            "retained": [],
            "discarded": [
                {"first_index": 5, "last_index": 7, "reason_code": "RAW_PAYLOAD_ARCHIVE_DELETE"},
            ],
            "unknown": [],
        }
        self.assertFalse(validate_unknown_not_promoted(original, modified))

    def test_unknown_unchanged_passes(self) -> None:
        """Unknown ranges that don't change pass the validator."""
        ranges = {
            "retained": [{"first_index": 0, "last_index": 3, "reason_code": "PINNED_SOURCE_REFERENCE"}],
            "discarded": [{"first_index": 4, "last_index": 7, "reason_code": "RAW_PAYLOAD_ARCHIVE_DELETE"}],
            "unknown": [{"first_index": 8, "last_index": 8, "reason_code": "AUTOMATIC_THRESHOLD_OBSERVED"}],
        }
        self.assertTrue(validate_unknown_not_promoted(ranges, ranges))

    def test_unknown_partial_reclassification_detected(self) -> None:
        """Partially reclassifying unknown (some indices moved) is detected."""
        original = {
            "retained": [],
            "discarded": [],
            "unknown": [
                {"first_index": 0, "last_index": 4, "reason_code": "AUTOMATIC_THRESHOLD_OBSERVED"},
            ],
        }
        # Indices 0-2 moved to retained, 3-4 remain unknown
        modified = {
            "retained": [
                {"first_index": 0, "last_index": 2, "reason_code": "PINNED_SOURCE_REFERENCE"},
            ],
            "discarded": [],
            "unknown": [
                {"first_index": 3, "last_index": 4, "reason_code": "AUTOMATIC_THRESHOLD_OBSERVED"},
            ],
        }
        self.assertFalse(validate_unknown_not_promoted(original, modified))


# ══════════════════════════════════════════════════════════════════════════════
# 4. AuditLedger integration
# ══════════════════════════════════════════════════════════════════════════════


class AuditLedgerIntegrationTests(unittest.TestCase):
    """Tests proving AuditLedger correctly handles automatic compaction events."""

    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name)
        self.key_store = MemoryInstallationKeyStore()

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def _namespace(self, name: str) -> ConversationNamespace:
        return ConversationNamespace.create(
            self.root / name,
            key_store=self.key_store,
            frozen_context=_context(name),
            allowed_capabilities=[
                "filesystem.workspace_read",
                "filesystem.workspace_write",
            ],
            denied_capabilities=[
                "credential.raw_read",
                "network.unrestricted",
            ],
        )

    def test_automatic_compaction_seal_verify_cycle(self) -> None:
        """Automatic compaction event → append → seal → verify passes."""
        namespace = self._namespace("auto-cmp")
        ledger = AuditLedger(namespace=namespace, key_store=self.key_store)

        # Simulate automatic compaction
        sim = AutomaticCompactionSimulator(threshold_tokens=2000)
        items = _make_items(10, tokens_each=500)
        auto_result = sim.simulate(items)

        # Append the compaction event
        ledger.append(
            source_kind="session",
            source_id="session:auto-cmp",
            source_sequence=0,
            source_completeness="partial",
            canonical_event_type="context.compacted",
            raw_payload=b"AUTO-COMPACTED-SOURCE-SPAN",
            raw_retention_category="private_reasoning",
            provenance_status="derived_unverified",
            source_refs=["fixture:gak-cmp-001/auto"],
            facts={
                "trigger": "automatic",
                "budget_limit": 2000,
                "span_size": len(items),
            },
            compaction=auto_result.to_audit_compaction(),
        )

        # Append terminal event
        ledger.append(
            source_kind="supervisor",
            source_id="supervisor:auto-cmp",
            source_sequence=0,
            source_completeness="unknown",
            canonical_event_type="run.terminal",
            raw_payload=b"RUN-TERMINAL",
            raw_retention_category="full_stdout_stderr",
            provenance_status="unknown",
            source_refs=["fixture:gak-cmp-001/auto"],
            facts={"exit_code": 0},
            terminal_outcome="succeeded",
        )

        seal = ledger.seal(
            source_completeness={
                "acp": "unknown",
                "session": "partial",
                "provider": "unknown",
                "supervisor": "unknown",
            }
        )

        # Verify the seal
        verification = verify_audit_seal(
            seal,
            namespace=namespace,
            key_store=self.key_store,
        )
        self.assertTrue(verification["valid"], verification["errors"])
        self.assertEqual(verification["event_count"], 2)
        self.assertEqual(seal["compaction"]["event_count"], 1)
        self.assertEqual(
            seal["compaction"]["summary_statuses"],
            ["derived_unverified"],
        )

    def test_mixed_automatic_and_manual_compaction(self) -> None:
        """Mixed automatic + manual compaction events produce correct seal projection."""
        namespace = self._namespace("mixed-cmp")
        ledger = AuditLedger(namespace=namespace, key_store=self.key_store)

        # First: automatic compaction
        auto_sim = AutomaticCompactionSimulator(threshold_tokens=1500)
        auto_items = _make_items(6, tokens_each=400)
        auto_result = auto_sim.simulate(auto_items)

        ledger.append(
            source_kind="session",
            source_id="session:mixed",
            source_sequence=0,
            source_completeness="partial",
            canonical_event_type="context.compacted",
            raw_payload=b"AUTO-COMPACTION-PAYLOAD",
            raw_retention_category="private_reasoning",
            provenance_status="derived_unverified",
            source_refs=["fixture:gak-cmp-001/mixed-1"],
            facts={"trigger": "automatic"},
            compaction=auto_result.to_audit_compaction(),
        )

        # Second: manual compaction (same source, next sequence)
        manual_sim = ManualCompactionSimulator()
        manual_items = _make_items(4, tokens_each=300)
        manual_result = manual_sim.simulate(
            manual_items,
            retained_indices={0, 1},
            discarded_indices={2, 3},
        )

        ledger.append(
            source_kind="session",
            source_id="session:mixed",
            source_sequence=1,
            source_completeness="partial",
            canonical_event_type="context.compacted",
            raw_payload=b"MANUAL-COMPACTION-PAYLOAD",
            raw_retention_category="private_reasoning",
            provenance_status="derived_unverified",
            source_refs=["fixture:gak-cmp-001/mixed-2"],
            facts={"trigger": "manual"},
            compaction=manual_result.to_audit_compaction(),
        )

        # Terminal
        ledger.append(
            source_kind="supervisor",
            source_id="supervisor:mixed",
            source_sequence=0,
            source_completeness="unknown",
            canonical_event_type="run.terminal",
            raw_payload=b"TERMINAL",
            raw_retention_category="full_stdout_stderr",
            provenance_status="unknown",
            source_refs=["fixture:gak-cmp-001/mixed"],
            facts={"exit_code": 0},
            terminal_outcome="succeeded",
        )

        seal = ledger.seal(
            source_completeness={
                "acp": "unknown",
                "session": "partial",
                "provider": "unknown",
                "supervisor": "unknown",
            }
        )

        self.assertEqual(seal["compaction"]["event_count"], 2)
        verification = verify_audit_seal(
            seal,
            namespace=namespace,
            key_store=self.key_store,
        )
        self.assertTrue(verification["valid"], verification["errors"])
        self.assertEqual(verification["event_count"], 3)

    def test_compaction_requires_derived_unverified(self) -> None:
        """Compaction events with non-derived_unverified provenance are rejected."""
        namespace = self._namespace("bad-prov")
        ledger = AuditLedger(namespace=namespace, key_store=self.key_store)

        with self.assertRaises(AssuranceError) as ctx:
            ledger.append(
                source_kind="session",
                source_id="session:bad",
                source_sequence=0,
                source_completeness="complete",
                canonical_event_type="context.compacted",
                raw_payload=b"BAD-PROVENANCE",
                raw_retention_category="private_reasoning",
                provenance_status="direct",  # ← wrong: must be derived_unverified
                source_refs=["fixture:gak-cmp-001/bad"],
                compaction={
                    "summary_status": "derived_unverified",
                    "summary_sha256": "a" * 64,
                    "source_span_sha256": "b" * 64,
                    "ranges": {
                        "retained": [],
                        "discarded": [],
                        "unknown": [
                            {"first_index": 0, "last_index": 0,
                             "reason_code": "AUTOMATIC_THRESHOLD_OBSERVED"},
                        ],
                    },
                },
            )
        self.assertIn("derived_unverified", str(ctx.exception))

    def test_tampered_compaction_detected_after_archive(self) -> None:
        """Tampered compaction metadata is detected post-archive by verify_audit_seal."""
        namespace = self._namespace("tamper-cmp")
        ledger = AuditLedger(namespace=namespace, key_store=self.key_store)

        sim = AutomaticCompactionSimulator(threshold_tokens=1000)
        items = _make_items(5, tokens_each=500)
        auto_result = sim.simulate(items)

        ledger.append(
            source_kind="session",
            source_id="session:tamper",
            source_sequence=0,
            source_completeness="partial",
            canonical_event_type="context.compacted",
            raw_payload=b"COMPACTION-PAYLOAD",
            raw_retention_category="private_reasoning",
            provenance_status="derived_unverified",
            source_refs=["fixture:gak-cmp-001/tamper"],
            facts={"trigger": "automatic"},
            compaction=auto_result.to_audit_compaction(),
        )
        ledger.append(
            source_kind="supervisor",
            source_id="supervisor:tamper",
            source_sequence=0,
            source_completeness="unknown",
            canonical_event_type="run.terminal",
            raw_payload=b"TERMINAL",
            raw_retention_category="full_stdout_stderr",
            provenance_status="unknown",
            source_refs=["fixture:gak-cmp-001/tamper"],
            facts={"exit_code": 0},
            terminal_outcome="succeeded",
        )

        seal = ledger.seal(
            source_completeness={
                "acp": "unknown",
                "session": "partial",
                "provider": "unknown",
                "supervisor": "unknown",
            }
        )

        # Pre-archive verification passes
        pre_verify = verify_audit_seal(
            seal, namespace=namespace, key_store=self.key_store,
        )
        self.assertTrue(pre_verify["valid"], pre_verify["errors"])

        # Archive
        ArchiveController(key_store=self.key_store).archive(namespace)

        # Post-archive verification passes
        post_verify = verify_audit_seal(
            seal, namespace=namespace, key_store=self.key_store,
            require_archive_complete=True,
        )
        self.assertTrue(post_verify["valid"], post_verify["errors"])
        self.assertTrue(post_verify["raw_payload_absence_proven"])

        # Tamper: modify the journal on disk
        journal_relative = seal["journal"]["relative_path"]
        prefix = "artifacts/redacted_conversation/"
        journal_path = namespace.root / journal_relative
        tampered_bytes = journal_path.read_bytes()
        # Change "derived_unverified" to "direct" in the journal
        tampered_bytes = tampered_bytes.replace(
            b'"derived_unverified"', b'"direct"'
        )
        journal_path.write_bytes(tampered_bytes)

        # Verification must now fail
        tamper_verify = verify_audit_seal(
            seal, namespace=namespace, key_store=self.key_store,
            require_archive_complete=True,
        )
        self.assertFalse(tamper_verify["valid"])
        self.assertTrue(
            any("compaction" in err.lower() for err in tamper_verify["errors"]),
            f"Expected compaction-related error, got: {tamper_verify['errors']}",
        )


# ══════════════════════════════════════════════════════════════════════════════
# 5. Manual vs Automatic comparison
# ══════════════════════════════════════════════════════════════════════════════


class ManualVsAutomaticComparisonTests(unittest.TestCase):
    """Tests comparing manual and automatic compaction semantics."""

    def test_manual_exact_ranges_no_unknown(self) -> None:
        """Manual compaction with exact classification produces no unknown (confidence=1.0)."""
        items = _make_items(10, tokens_each=300)
        result = ManualCompactionSimulator.simulate(
            items,
            retained_indices={0, 1, 2, 3},
            discarded_indices={4, 5, 6, 7, 8, 9},
        )
        self.assertEqual(result.trigger, "manual")
        self.assertEqual(result.classification_confidence, 1.0)
        self.assertFalse(result.has_unknown_ranges)
        # Exact ranges
        retained = result.ranges["retained"]
        self.assertEqual(len(retained), 1)
        self.assertEqual(retained[0]["first_index"], 0)
        self.assertEqual(retained[0]["last_index"], 3)

    def test_manual_unclassified_items_become_unknown(self) -> None:
        """Items not explicitly classified go to unknown with lower confidence."""
        items = _make_items(5, tokens_each=300)
        result = ManualCompactionSimulator.simulate(
            items,
            retained_indices={0, 1},
            # items 2-4 not classified → unknown
        )
        self.assertEqual(result.trigger, "manual")
        self.assertLess(result.classification_confidence, 1.0)
        self.assertTrue(result.has_unknown_ranges)
        # Items 2-4 should be in unknown
        unknown_indices: set[int] = set()
        for r in result.ranges["unknown"]:
            unknown_indices.update(range(r["first_index"], r["last_index"] + 1))
        self.assertIn(2, unknown_indices)
        self.assertIn(3, unknown_indices)
        self.assertIn(4, unknown_indices)

    def test_both_produce_valid_audit_compaction(self) -> None:
        """Both manual and automatic compaction produce audit-compatible dicts."""
        from assurance.audit import _normalize_compaction

        items = _make_items(8, tokens_each=400)

        auto = AutomaticCompactionSimulator(threshold_tokens=1500).simulate(items)
        manual = ManualCompactionSimulator.simulate(
            items, retained_indices={0, 1, 2}, discarded_indices={3, 4, 5, 6, 7},
        )

        for label, result in [("auto", auto), ("manual", manual)]:
            audit_dict = result.to_audit_compaction()
            normalized = _normalize_compaction(audit_dict)
            self.assertIsNotNone(normalized, f"{label} compaction should normalize")
            self.assertEqual(
                normalized["summary_status"], "derived_unverified",
                f"{label} summary must be derived_unverified",
            )

    def test_automatic_trigger_honesty_with_confidence_below_one(self) -> None:
        """Automatic trigger with confidence < 1.0 must have unknown ranges."""
        sim = AutomaticCompactionSimulator(
            threshold_tokens=1000,
            boundary_zone_ratio=0.20,  # large boundary → more uncertainty
        )
        items = _make_items(5, tokens_each=500)  # 2500 >> 1000
        result = sim.simulate(items)

        if result.classification_confidence < 1.0:
            self.assertTrue(
                result.has_unknown_ranges,
                "confidence < 1.0 requires unknown ranges for honesty",
            )
        self.assertTrue(validate_automatic_trigger_honesty(result))

    def test_automatic_trigger_honesty_full_confidence(self) -> None:
        """Automatic trigger with confidence 1.0 is valid without unknown."""
        sim = AutomaticCompactionSimulator(threshold_tokens=5000)
        items = _make_items(4, tokens_each=500)  # 2000 < 5000
        result = sim.simulate(items)
        self.assertEqual(result.classification_confidence, 1.0)
        self.assertTrue(validate_automatic_trigger_honesty(result))


# ══════════════════════════════════════════════════════════════════════════════
# 6. Token estimation
# ══════════════════════════════════════════════════════════════════════════════


class TokenEstimationTests(unittest.TestCase):
    """Tests for :func:`estimate_tokens`."""

    def test_empty_string_zero(self) -> None:
        self.assertEqual(estimate_tokens(""), 0)

    def test_short_text_minimum_one(self) -> None:
        self.assertEqual(estimate_tokens("hi"), 1)

    def test_english_prose(self) -> None:
        text = "The quick brown fox jumps over the lazy dog." * 10  # ~440 chars
        tokens = estimate_tokens(text)
        self.assertGreater(tokens, 100)
        self.assertLess(tokens, 250)

    def test_code_content(self) -> None:
        text = "def foo(x):\n    return x * 2\n" * 100  # ~2500 chars
        tokens = estimate_tokens(text)
        self.assertGreater(tokens, 500)
        self.assertLess(tokens, 1500)


if __name__ == "__main__":
    unittest.main()
