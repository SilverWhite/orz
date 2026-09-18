"""0k 审查处理 (P1-1, 2026-08-30)：close-record `terminal_reason` 枚举
防回归测试。

背景：`retrieval-close-record-event-payload-v0.2.schema.json` 的枚举此前
缺 `subagent_timeout`（RETRIEVAL-ORCHESTRATION-MECHANICAL 0k 子代理 run
级墙钟超时收口）与 `auto_close`（THIN-HARNESS-REDESIGN R1 每次派发即
闭环）——真实超时/正常 run 的 journal 经严格 jsonschema 校验会失败
（GAP-EVENT-SCHEMA-DRIFT 同类）。pytest 此前全绿是因为 fixtures 只覆盖
`normal_close`、verifier 对非 normal_close 不交叉检查。

本文件独立于 `test_run_event_conformance.py`（该文件处于 S5-1 工作树
修改中，不混入本批），用独立断言锁住两个新枚举值 + 新正例 fixture。

0ar S1 扩展（2026-09-19，RETRIEVAL_BATCH_HANDOFF_AND_ROUND_SEAT_DESIGN
v1.0 §7/§9）：检索批次回送契约再补三个**正常收尾** terminal_reason
（`evidence_threshold_met`／`dispatch_wallclock_bound`／
`subagent_early_delivery`，不受 `normal_close` 的 allOf 约束）＋其正例/
约束反例，以及 `information_sufficiency_assessment` 的**可用计数与缺口**
字段（宽口径；两字段可选、配对规则 gap ⇒ count）。
"""

from __future__ import annotations

import json
from pathlib import Path
import unittest

from jsonschema import Draft202012Validator, FormatChecker


ROOT = Path(__file__).resolve().parents[2]
CLOSE_RECORD_SCHEMA = (
    ROOT / "runtime" / "retrieval-close-record-event-payload-v0.2.schema.json"
)
ASSESSMENT_SCHEMA = (
    ROOT / "runtime" / "information-sufficiency-assessment-event-payload-v0.2.schema.json"
)
FIXTURE_DIR = ROOT / "runtime" / "fixtures" / "run-event-v0.2" / "payloads"

# 0ar S1：D1/D2 正常收尾三值——缺任何一个都是 schema 漂移。
BATCH_HANDOFF_REASONS = {
    "evidence_threshold_met",
    "dispatch_wallclock_bound",
    "subagent_early_delivery",
}

# 0k 审查处理：枚举必须包含两个新增值——缺任何一个都是 schema 漂移。
REQUIRED_REASONS = {"subagent_timeout", "auto_close"} | BATCH_HANDOFF_REASONS
ALL_REASONS = {
    "normal_close",
    "auto_close",
    "user_cancelled",
    "session_cancelled",
    "wallclock",
    "budget_exhausted",
    "subagent_failed",
    "subagent_cancelled",
    "subagent_timeout",
} | BATCH_HANDOFF_REASONS

POSITIVE_FIXTURES = (
    "retrieval-close-record.subagent-timeout.valid.json",
    "retrieval-close-record.auto-close.valid.json",
    "retrieval-close-record.minimal.valid.json",
    # 0ar S1：批次回送三形态（达标回送／到点交回／提前交付）。
    "retrieval-close-record.evidence-threshold-met.valid.json",
    "retrieval-close-record.dispatch-wallclock-bound.valid.json",
    "retrieval-close-record.subagent-early-delivery.valid.json",
)

# RETRIEVAL-ORCHESTRATION-MECHANICAL 0k 第二批 (2026-08-30)：close-record
# 可选 `effort` 档枚举——缺任一档都是 schema 漂移；未知档必须被严格拒绝。
ALL_EFFORTS = {"standard", "extended", "deep"}


def load_json(path: Path) -> object:
    return json.loads(path.read_text(encoding="utf-8"))


def errors_for(
    instance_path: Path, schema_path: Path = CLOSE_RECORD_SCHEMA
) -> list[str]:
    validator = Draft202012Validator(
        load_json(schema_path), format_checker=FormatChecker()
    )
    return [
        error.message
        for error in validator.iter_errors(load_json(instance_path))
    ]


class RetrievalCloseReasonEnumTests(unittest.TestCase):
    def test_enum_contains_subagent_timeout_and_auto_close(self) -> None:
        schema = load_json(CLOSE_RECORD_SCHEMA)
        enum = set(schema["properties"]["terminal_reason"]["enum"])
        self.assertTrue(
            REQUIRED_REASONS <= enum,
            f"close-record terminal_reason enum missing: "
            f"{sorted(REQUIRED_REASONS - enum)}",
        )
        self.assertEqual(enum, ALL_REASONS)

    def test_effort_enum_contains_all_tiers(self) -> None:
        schema = load_json(CLOSE_RECORD_SCHEMA)
        effort = set(schema["properties"]["effort"]["enum"])
        self.assertEqual(
            effort,
            ALL_EFFORTS,
            f"close-record effort enum drift: {sorted(ALL_EFFORTS - effort)}",
        )

    def test_bad_effort_fails_closed(self) -> None:
        errors = errors_for(FIXTURE_DIR / "retrieval-close-record.bad-effort.invalid.json")
        self.assertTrue(
            errors,
            "unknown effort tier must be rejected by the close-record schema",
        )

    def test_new_reason_fixtures_validate(self) -> None:
        for name in POSITIVE_FIXTURES:
            with self.subTest(fixture=name):
                self.assertEqual(
                    errors_for(FIXTURE_DIR / name),
                    [],
                    f"{name} must validate against the close-record schema",
                )

    def test_unknown_reason_fails_closed(self) -> None:
        """枚举之外的 terminal_reason 必须被严格 schema 拒绝（防枚举回退）。"""
        payload = {
            "close_record_id": "CLOSE-0001",
            "parent_session_id": "sess-main-1",
            "subagent_session_id": "sess-ext-2",
            "activation_id": "ACT-EXT-0001",
            "contract_id": "CONTRACT-EXT-0001",
            "contract_revision": 0,
            "result_digest": None,
            "assessment_id": None,
            "validated_disposition_id": None,
            "terminal_reason": "not_a_real_reason",
            "resumable": True,
            "live_state_reset": True,
            "archive_ref": "run-journal:RUN-0001",
        }
        validator = Draft202012Validator(
            load_json(CLOSE_RECORD_SCHEMA), format_checker=FormatChecker()
        )
        self.assertTrue(
            [e for e in validator.iter_errors(payload)],
            "unknown terminal_reason must be rejected",
        )


class RetrievalBatchHandoffContractTests(unittest.TestCase):
    """0ar S1（2026-09-19，RETRIEVAL_BATCH_HANDOFF_AND_ROUND_SEAT_DESIGN
    v1.0 §7／§8 判据 1、§4.2）：批次回送契约面的机械锁。

    - 到点交回（`dispatch_wallclock_bound`）的「已得计数＋缺口」只存在于
      `information_sufficiency_assessment` ⇒ close record 必带 assessment 链；
    - 提前交付（`subagent_early_delivery`）须带证据指针（result_digest ＋
      archive_ref）；
    - assessment 可用计数/缺口字段为**可选**（存量刊物与生产者零行为变更），
      配对规则 gap ⇒ count 由约束反例锁住。
    """

    def test_wallclock_bound_close_requires_assessment_chain(self) -> None:
        validator = Draft202012Validator(
            load_json(CLOSE_RECORD_SCHEMA), format_checker=FormatChecker()
        )
        instance = load_json(
            FIXTURE_DIR
            / "retrieval-close-record.wallclock-missing-assessment.constraint.invalid.json"
        )
        failures = list(validator.iter_errors(instance))
        self.assertTrue(
            failures,
            "dispatch_wallclock_bound without an assessment chain must be "
            "rejected by the close-record schema",
        )
        paths = {tuple(error.absolute_path) for error in failures}
        self.assertIn(
            ("assessment_id",),
            paths,
            f"refusal must point at assessment_id: {sorted(paths)}",
        )

    def test_early_delivery_fixture_keeps_evidence_pointer(self) -> None:
        payload = load_json(
            FIXTURE_DIR / "retrieval-close-record.subagent-early-delivery.valid.json"
        )
        self.assertEqual(payload["terminal_reason"], "subagent_early_delivery")
        self.assertTrue(payload["result_digest"], "early delivery needs a digest")
        self.assertTrue(payload["archive_ref"], "early delivery needs a pointer")

    def test_new_reasons_are_not_bound_by_the_normal_close_allof(self) -> None:
        """三个新值均带 validated_disposition_id: null 且仍合法 ⇒ 不受
        `normal_close` 的 allOf（必须引用已裁决 close）约束。"""
        for name in (
            "retrieval-close-record.evidence-threshold-met.valid.json",
            "retrieval-close-record.dispatch-wallclock-bound.valid.json",
            "retrieval-close-record.subagent-early-delivery.valid.json",
        ):
            with self.subTest(fixture=name):
                payload = load_json(FIXTURE_DIR / name)
                self.assertIn(payload["terminal_reason"], BATCH_HANDOFF_REASONS)
                self.assertIsNone(payload["validated_disposition_id"])
                self.assertEqual(errors_for(FIXTURE_DIR / name), [])

    def test_assessment_usable_count_and_gap_are_optional_but_paired(self) -> None:
        schema = load_json(ASSESSMENT_SCHEMA)
        properties = schema["properties"]
        self.assertIn("usable_source_count", properties)
        self.assertIn("sufficiency_gap", properties)
        for optional in ("usable_source_count", "sufficiency_gap"):
            self.assertNotIn(
                optional,
                schema["required"],
                f"{optional} must stay optional (zero behaviour change)",
            )
        self.assertEqual(
            errors_for(
                FIXTURE_DIR
                / "information-sufficiency-assessment.partial-report.valid.json",
                ASSESSMENT_SCHEMA,
            ),
            [],
            "partial-report assessment must validate",
        )
        self.assertTrue(
            errors_for(
                FIXTURE_DIR
                / "information-sufficiency-assessment.gap-without-count.constraint.invalid.json",
                ASSESSMENT_SCHEMA,
            ),
            "sufficiency_gap without usable_source_count must be rejected",
        )


if __name__ == "__main__":
    unittest.main()
