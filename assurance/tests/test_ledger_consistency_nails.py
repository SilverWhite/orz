"""Negative-case nails for the ledger consistency & slimming gate (0ab).

`check_repository.py` cross-checks the living ledgers (BACKLOG / TODO /
index §8) via structured anchors.  These nails inject synthetic ledgers
with one defect at a time and assert the gate detects it, plus one
resident check that the real repository currently passes.
"""

from __future__ import annotations

import importlib.util
from datetime import date
from pathlib import Path
import tempfile
import unittest


_ROOT = Path(__file__).resolve().parents[2]
_SCRIPT = _ROOT / "scripts" / "check_repository.py"

_spec = importlib.util.spec_from_file_location("check_repository_under_test", _SCRIPT)
check_repository = importlib.util.module_from_spec(_spec)
assert _spec.loader is not None
_spec.loader.exec_module(check_repository)

TODAY = date(2026, 9, 15)


def _backlog(
    total: int = 2,
    p0_line: str = "开放项：0b / 0c。",
    p0_row: str = "| P0 | 工作集 | 甲（0b）；乙（0c） |",
    p1_line: str = "开放项：4。",
    p1_row: str = "| P1 | 并行 | 丙（4） |",
    p2_line: str = "开放项：7。",
    p2_row: str = "| P2 | 决策门 | 丁（7） |",
    count_section: str = "## 未闭合计数\n\n- 未闭合总数：**{total} 项**（口径 2026-09-15）。\n",
) -> str:
    return (
        "# BACKLOG\n\n"
        + count_section.format(total=total)
        + "\n## 优先级总览\n\n"
        + "| 优先级 | 含义 | 开放项（入口小节） |\n|---|---|---|\n"
        + p0_row
        + "\n"
        + p1_row
        + "\n"
        + p2_row
        + "\n"
        + f"\n## P0 — 当前工作集\n\n{p0_line}\n### 0b. 甲\n- 内容\n### 0c. 乙\n- 内容\n"
        + f"\n## P1 — 并行\n\n{p1_line}\n### 4. 丙\n- 内容\n"
        + f"\n## P2 — 决策门\n\n{p2_line}\n### 7. 丁\n- 内容\n"
    )


def _todo(
    total: int = 2,
    p0_route: str = "- P0：0b 剩余；0c 余项。",
    p1_route: str = "- P1：丙别名（4）。",
    p2_route: str = "- P2：丁别名（P2-7）。",
    extra_sections: str = "",
) -> str:
    return (
        "# TODO\n\n"
        "## 开放项路由（同步）\n\n"
        f"- 未闭合总数：**{total} 项**（BACKLOG 计数口径）。\n"
        + p0_route
        + "\n"
        + p1_route
        + "\n"
        + p2_route
        + "\n"
        + "\n## P0 — 当前工作集\n\n"
        "### P0-F 甲（pending）\n\n> 入口：BACKLOG 0b。\n\n- [ ] 剩余实施\n"
        "### P0-0c 乙（open）\n\n> 入口：BACKLOG 0c。\n\n- [ ] 余项\n"
        + extra_sections
    )


def _index(
    section8: str = "- `pending`：GAP-A、GAP-B。\n- `implemented`：GAP-C。\n",
    entries: str = "- **GAP-A** (`pending`; 2026-09-15)：定义。入口：[x](x.md)。\n",
) -> str:
    return (
        "# INDEX\n\n"
        + entries
        + "\n## 8. 状态速查\n\n"
        + section8
        + "\n## 9. 使用红线\n\n- 正文。\n"
    )


class LedgerConsistencyNails(unittest.TestCase):
    def _run_checks(self, backlog: str, todo: str, index: str) -> list[str]:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "docs").mkdir()
            (root / "docs" / "BACKLOG_AND_PRIORITIES.md").write_text(
                backlog, encoding="utf-8"
            )
            (root / "TODO.md").write_text(todo, encoding="utf-8")
            (root / "CLI_PROJECT_INDEX.md").write_text(index, encoding="utf-8")
            errors = list(check_repository._check_ledger_consistency(root))
            slimming, _covered = check_repository._check_ledger_slimming(
                root, today=TODAY
            )
            errors.extend(slimming)
            return errors

    def test_consistent_synthetic_ledger_passes(self) -> None:
        self.assertEqual(self._run_checks(_backlog(), _todo(), _index()), [])

    def test_total_mismatch_between_backlog_and_todo_is_detected(self) -> None:
        errors = self._run_checks(
            _backlog(total=3), _todo(total=2), _index()
        )
        self.assertTrue(
            any("未闭合总数不一致" in error for error in errors), errors
        )

    def test_backlog_open_line_missing_token_vs_table_is_detected(self) -> None:
        errors = self._run_checks(
            _backlog(p0_line="开放项：0b。"), _todo(), _index()
        )
        self.assertTrue(
            any("P0 开放项清单不一致" in error and "优先级总览表" in error
                for error in errors),
            errors,
        )

    def test_todo_route_missing_token_is_detected(self) -> None:
        errors = self._run_checks(
            _backlog(), _todo(p0_route="- P0：0b 剩余。"), _index()
        )
        self.assertTrue(
            any("P0 开放项清单不一致" in error and "TODO 路由" in error
                for error in errors),
            errors,
        )

    def test_p2_route_missing_numeric_marker_is_detected(self) -> None:
        errors = self._run_checks(
            _backlog(), _todo(p2_route="- P2：丁别名。"), _index()
        )
        self.assertTrue(
            any("P2 开放项清单不一致" in error for error in errors), errors
        )

    def test_closed_todo_section_listed_open_in_backlog_is_detected(self) -> None:
        extra = "### P0-0z 残留（closed）\n\n> 入口：BACKLOG 0z。\n\n- [x] 已闭合\n"
        errors = self._run_checks(
            _backlog(p0_line="开放项：0b / 0c / 0z。",
                     p0_row="| P0 | 工作集 | 甲（0b）；乙（0c）；残（0z） |"),
            _todo(extra_sections=extra),
            _index(),
        )
        self.assertTrue(
            any("TODO 全部勾选但 BACKLOG 仍记开放" in error and "0z" in error
                for error in errors),
            errors,
        )

    def test_open_todo_section_absent_from_backlog_lists_is_detected(self) -> None:
        extra = "### P0-0q 未登记（open）\n\n> 入口：BACKLOG 0q。\n\n- [ ] 待办\n"
        errors = self._run_checks(_backlog(), _todo(extra_sections=extra), _index())
        self.assertTrue(
            any("TODO 存在未勾选节但 BACKLOG 已记闭合" in error and "0q" in error
                for error in errors),
            errors,
        )

    def test_index_status_id_in_two_buckets_is_detected(self) -> None:
        errors = self._run_checks(
            _backlog(),
            _todo(),
            _index(section8="- `pending`：GAP-A。\n- `partial`：GAP-A。\n"),
        )
        self.assertTrue(
            any("同时出现在" in error and "GAP-A" in error for error in errors),
            errors,
        )

    def test_index_duplicate_id_within_bucket_is_detected(self) -> None:
        errors = self._run_checks(
            _backlog(),
            _todo(),
            _index(section8="- `pending`：GAP-A、GAP-A。\n- `implemented`：GAP-C。\n"),
        )
        self.assertTrue(
            any("桶内重复 ID" in error for error in errors), errors
        )

    def test_index_illegal_entry_status_is_detected(self) -> None:
        errors = self._run_checks(
            _backlog(),
            _todo(),
            _index(entries="- **GAP-A** (`in_progress`; 2026-09-15)：定义。\n"),
        )
        self.assertTrue(
            any("非法状态" in error and "in_progress" in error for error in errors),
            errors,
        )

    def test_embedded_second_heading_is_detected(self) -> None:
        backlog = _backlog().replace(
            "## P2 — 决策门", "## P2 — 决策门## P2 — 决策门", 1
        )
        errors = self._run_checks(backlog, _todo(), _index())
        self.assertTrue(
            any("标题行内嵌第二个标题记号" in error for error in errors), errors
        )

    def test_overlong_count_line_is_flagged_for_archiving(self) -> None:
        filler = "；" .join(f"2026-09-15 第{i}次流水" for i in range(200))
        backlog = _backlog(
            count_section=(
                f"## 未闭合计数\n\n- 未闭合总数：**2 项**（{filler}）。\n"
            )
        )
        errors = self._run_checks(backlog, _todo(), _index())
        self.assertTrue(
            any("头部台账/计数行超长" in error for error in errors), errors
        )

    def test_aged_count_line_is_flagged_for_archiving(self) -> None:
        backlog = _backlog(
            count_section=(
                "## 未闭合计数\n\n"
                "- 未闭合总数：**2 项**（2026-06-01 定档，长期未动）。\n"
            )
        )
        errors = self._run_checks(backlog, _todo(), _index())
        self.assertTrue(
            any("行龄超限" in error for error in errors), errors
        )


class ResidentRepositoryLedgerGate(unittest.TestCase):
    """The gate must stay green on the real ledgers (deterministic)."""

    def test_real_repository_ledgers_are_consistent(self) -> None:
        self.assertEqual(
            check_repository._check_ledger_consistency(_ROOT), []
        )

    def test_real_repository_header_lines_are_within_limits(self) -> None:
        errors, _covered = check_repository._check_ledger_slimming(_ROOT)
        self.assertEqual(errors, [])


if __name__ == "__main__":
    unittest.main()
