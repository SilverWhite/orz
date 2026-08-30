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
FIXTURE_DIR = ROOT / "runtime" / "fixtures" / "run-event-v0.2" / "payloads"

# 0k 审查处理：枚举必须包含两个新增值——缺任何一个都是 schema 漂移。
REQUIRED_REASONS = {"subagent_timeout", "auto_close"}
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
}

POSITIVE_FIXTURES = (
    "retrieval-close-record.subagent-timeout.valid.json",
    "retrieval-close-record.auto-close.valid.json",
    "retrieval-close-record.minimal.valid.json",
)

# RETRIEVAL-ORCHESTRATION-MECHANICAL 0k 第二批 (2026-08-30)：close-record
# 可选 `effort` 档枚举——缺任一档都是 schema 漂移；未知档必须被严格拒绝。
ALL_EFFORTS = {"standard", "extended", "deep"}


def load_json(path: Path) -> object:
    return json.loads(path.read_text(encoding="utf-8"))


def errors_for(instance_path: Path) -> list[str]:
    validator = Draft202012Validator(
        load_json(CLOSE_RECORD_SCHEMA), format_checker=FormatChecker()
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


if __name__ == "__main__":
    unittest.main()
