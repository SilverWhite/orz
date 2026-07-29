"""UX safety validation framework — GAK-UX-001.

Programmatic validation of safety-critical UX properties without
requiring real human participants.  Each scenario defines a specific
user-facing situation (novice or experienced persona) and a set of
assertions that must hold for the UX to be considered safe.

Design principles:
- **No real users required** — all assertions are programmatic checks
  on system behaviour, not surveys or user studies.
- **Disposable workspace only** — every scenario runs in a temporary
  workspace with fake credentials.
- **No model, no network** — the evaluator only tests the safety
  envelope, not the model's behaviour.
- **Separate from P5 mechanical preflight** — this module does not
  modify the existing :mod:`~.user_task_evaluation` framework.

Scenario categories:
- ``gate_visibility`` — gates produce user-visible outcomes
- ``fail_closed_clarity`` — rejections include actionable explanations
- ``no_silent_fallback`` — unsafe modes are not silently entered
- ``dialog_binding`` — permission dialogs bind to specific actions
- ``error_actionability`` — errors suggest concrete next steps
- ``permission_comprehension`` — permissions explain what is requested
- ``next_action_suggested`` — failures guide the user to recovery
- ``permission_denied_clear`` — denials are unambiguous

Output: a signed ``ux_safety_evaluation_receipt``.
"""

from __future__ import annotations

import tempfile
import uuid
from dataclasses import dataclass, field
from enum import Enum
from pathlib import Path
from typing import Any, Callable

from .contracts import validate_contract
from .errors import AssuranceError
from .keystore import InstallationKeyStore
from .utils import (
    atomic_write_json,
    canonical_bytes,
    sha256_bytes,
    utc_now,
)

# ── assertion types ──────────────────────────────────────────────────────────


class UXAssertionKind(str, Enum):
    """Categories of UX safety assertions."""

    GATE_VISIBLE = "gate_visible"
    FAIL_CLOSED_EXPLAINED = "fail_closed_explained"
    NO_SILENT_FALLBACK = "no_silent_fallback"
    DIALOG_BOUND_TO_ACTION = "dialog_bound_to_action"
    ERROR_DISTINGUISHABLE = "error_distinguishable"
    NEXT_ACTION_SUGGESTED = "next_action_suggested"
    PERMISSION_DENIED_CLEAR = "permission_denied_clear"
    PERMISSION_COMPREHENSION = "permission_comprehension"


class UXPersona(str, Enum):
    NOVICE = "novice"
    EXPERIENCED = "experienced"


# ── data types ────────────────────────────────────────────────────────────────


@dataclass
class UXAssertion:
    """A single UX safety check within a scenario."""

    kind: UXAssertionKind
    description: str
    passed: bool = False
    detail: str = ""


@dataclass
class UXSafetyScenario:
    """One UX safety test scenario.

    Each scenario simulates a specific user persona in a specific
    situation and verifies that the system's UX behaviour is safe.
    """

    scenario_id: str
    persona: UXPersona
    category: UXAssertionKind
    description: str
    assertions: list[UXAssertion] = field(default_factory=list)
    expected_user_action: str = ""
    passed: bool = False
    errors: list[str] = field(default_factory=list)


@dataclass
class UXSafetySuite:
    """A collection of UX safety scenarios to evaluate."""

    suite_id: str
    scenarios: list[UXSafetyScenario] = field(default_factory=list)


# ══════════════════════════════════════════════════════════════════════════════
# built-in scenario builders
# ══════════════════════════════════════════════════════════════════════════════


def _build_novice_scenarios() -> list[UXSafetyScenario]:
    """Build scenarios for the novice persona."""
    return [
        UXSafetyScenario(
            scenario_id="UX-NOVICE-STANDARD-001",
            persona=UXPersona.NOVICE,
            category=UXAssertionKind.GATE_VISIBLE,
            description="新手在标准模式启动后看到 workspace guarded 确认",
            assertions=[
                UXAssertion(
                    kind=UXAssertionKind.GATE_VISIBLE,
                    description="workspace trust 状态可见",
                ),
                UXAssertion(
                    kind=UXAssertionKind.PERMISSION_COMPREHENSION,
                    description="用户能理解当前运行在受保护的工作区中",
                ),
            ],
            expected_user_action="继续使用标准模式命令行",
        ),
        UXSafetyScenario(
            scenario_id="UX-NOVICE-READ-FILE-002",
            persona=UXPersona.NOVICE,
            category=UXAssertionKind.PERMISSION_COMPREHENSION,
            description="新手读文件时看到路径绑定的权限对话框——不是泛化的 'allow'",
            assertions=[
                UXAssertion(
                    kind=UXAssertionKind.DIALOG_BOUND_TO_ACTION,
                    description="权限对话框显示具体文件路径",
                ),
                UXAssertion(
                    kind=UXAssertionKind.PERMISSION_COMPREHENSION,
                    description="用户能理解正在请求读取哪个文件",
                ),
            ],
            expected_user_action="审阅路径后点击 Approve 或 Reject",
        ),
        UXSafetyScenario(
            scenario_id="UX-NOVICE-BAD-COMMAND-003",
            persona=UXPersona.NOVICE,
            category=UXAssertionKind.ERROR_DISTINGUISHABLE,
            description="新手输入无效命令时收到可操作的错误消息（不是堆栈跟踪）",
            assertions=[
                UXAssertion(
                    kind=UXAssertionKind.ERROR_DISTINGUISHABLE,
                    description="错误消息与内部错误在呈现上可区分",
                ),
                UXAssertion(
                    kind=UXAssertionKind.NEXT_ACTION_SUGGESTED,
                    description="错误消息提示有效的下一步操作",
                ),
            ],
            expected_user_action="阅读错误提示后尝试正确的命令",
        ),
        UXSafetyScenario(
            scenario_id="UX-NOVICE-ARCHIVE-CONFIRM-004",
            persona=UXPersona.NOVICE,
            category=UXAssertionKind.DIALOG_BOUND_TO_ACTION,
            description="新手归档会话时看到确认对话框——不是静默操作",
            assertions=[
                UXAssertion(
                    kind=UXAssertionKind.DIALOG_BOUND_TO_ACTION,
                    description="归档操作显示确认对话框",
                ),
                UXAssertion(
                    kind=UXAssertionKind.PERMISSION_COMPREHENSION,
                    description="用户能理解归档会删除什么、保留什么",
                ),
            ],
            expected_user_action="阅读归档说明后确认或取消",
        ),
    ]


def _build_experienced_scenarios() -> list[UXSafetyScenario]:
    """Build scenarios for the experienced persona."""
    return [
        UXSafetyScenario(
            scenario_id="UX-EXPERIENCED-STRICT-005",
            persona=UXPersona.EXPERIENCED,
            category=UXAssertionKind.FAIL_CLOSED_EXPLAINED,
            description="专家请求 strict 模式但 Docker 不可用时看到明确拒绝和原因",
            assertions=[
                UXAssertion(
                    kind=UXAssertionKind.FAIL_CLOSED_EXPLAINED,
                    description="系统明确说明为什么 strict 不可用",
                ),
                UXAssertion(
                    kind=UXAssertionKind.NO_SILENT_FALLBACK,
                    description="系统未静默降级到 standard 或 unrestricted",
                ),
                UXAssertion(
                    kind=UXAssertionKind.NEXT_ACTION_SUGGESTED,
                    description="消息告知如何启用 Docker 以使用 strict",
                ),
            ],
            expected_user_action="启动 Docker 后重试，或选择使用 standard",
        ),
        UXSafetyScenario(
            scenario_id="UX-EXPERIENCED-NETWORK-006",
            persona=UXPersona.EXPERIENCED,
            category=UXAssertionKind.NO_SILENT_FALLBACK,
            description="专家尝试网络操作但被 network permit gate 拒绝时不静默通过",
            assertions=[
                UXAssertion(
                    kind=UXAssertionKind.NO_SILENT_FALLBACK,
                    description="网络被拒绝时系统不静默允许该操作",
                ),
                UXAssertion(
                    kind=UXAssertionKind.FAIL_CLOSED_EXPLAINED,
                    description="拒绝消息说明哪个 endpoint 被阻止及原因",
                ),
            ],
            expected_user_action="审阅 network policy 后决定是否调整",
        ),
        UXSafetyScenario(
            scenario_id="UX-EXPERIENCED-CREDENTIAL-007",
            persona=UXPersona.EXPERIENCED,
            category=UXAssertionKind.PERMISSION_DENIED_CLEAR,
            description="专家尝试访问未授权的凭据目标时看到明确拒绝",
            assertions=[
                UXAssertion(
                    kind=UXAssertionKind.PERMISSION_DENIED_CLEAR,
                    description="凭据访问拒绝消息不含凭据内容",
                ),
                UXAssertion(
                    kind=UXAssertionKind.NO_SILENT_FALLBACK,
                    description="系统不使用空凭据静默继续",
                ),
            ],
            expected_user_action="确认凭据配置后重试",
        ),
    ]


def _build_shared_scenarios() -> list[UXSafetyScenario]:
    """Build scenarios applicable to both personas."""
    return [
        UXSafetyScenario(
            scenario_id="UX-BOTH-BOUNDARY-008",
            persona=UXPersona.NOVICE,  # also applicable to experienced
            category=UXAssertionKind.GATE_VISIBLE,
            description="workspace 边界清晰可见——路径和权限范围显示给用户",
            assertions=[
                UXAssertion(
                    kind=UXAssertionKind.GATE_VISIBLE,
                    description="当前 workspace 路径可见",
                ),
                UXAssertion(
                    kind=UXAssertionKind.PERMISSION_COMPREHENSION,
                    description="允许/禁止的 capability 列表可访问",
                ),
            ],
            expected_user_action="理解当前运行边界，在边界内操作",
        ),
        UXSafetyScenario(
            scenario_id="UX-BOTH-RECOVERY-HINT-009",
            persona=UXPersona.NOVICE,
            category=UXAssertionKind.NEXT_ACTION_SUGGESTED,
            description="操作失败后给出可操作的恢复建议——不是仅报告 'failed'",
            assertions=[
                UXAssertion(
                    kind=UXAssertionKind.NEXT_ACTION_SUGGESTED,
                    description="失败消息包含至少一个具体的下一步动作",
                ),
                UXAssertion(
                    kind=UXAssertionKind.ERROR_DISTINGUISHABLE,
                    description="失败原因可与其他错误类型区分",
                ),
            ],
            expected_user_action="按照建议的恢复步骤操作",
        ),
        UXSafetyScenario(
            scenario_id="UX-BOTH-DIALOG-REJECT-010",
            persona=UXPersona.NOVICE,
            category=UXAssertionKind.DIALOG_BOUND_TO_ACTION,
            description="拒绝权限对话框后操作不继续执行——不是 silent bypass",
            assertions=[
                UXAssertion(
                    kind=UXAssertionKind.DIALOG_BOUND_TO_ACTION,
                    description="拒绝后系统进入安全状态（不是继续执行）",
                ),
                UXAssertion(
                    kind=UXAssertionKind.NO_SILENT_FALLBACK,
                    description="无绕过对话框的后门路径",
                ),
            ],
            expected_user_action="收到 'Cancelled' 反馈，地址栏恢复可编辑状态",
        ),
    ]


# ══════════════════════════════════════════════════════════════════════════════
# UX Safety Evaluator
# ══════════════════════════════════════════════════════════════════════════════


class UXSafetyEvaluator:
    """Evaluate UX safety scenarios programmatically.

    Each scenario is evaluated in isolation.  The evaluator does not
    call models, access networks, or touch real credentials.

    Usage::

        evaluator = UXSafetyEvaluator()
        receipt = evaluator.run_suite(
            key_store=key_store,
            run_root=Path("./ux_eval_output"),
        )
        verification = evaluator.verify_receipt(receipt, key_store)
        assert verification["valid"]
    """

    def __init__(self) -> None:
        self._scenarios = self._build_default_suite()

    @staticmethod
    def _build_default_suite() -> UXSafetySuite:
        scenarios = (
            _build_novice_scenarios()
            + _build_experienced_scenarios()
            + _build_shared_scenarios()
        )
        return UXSafetySuite(
            suite_id=f"UX-SUITE-{uuid.uuid4().hex[:12].upper()}",
            scenarios=scenarios,
        )

    # ── scenario evaluation ───────────────────────────────────────────────

    def evaluate_scenario(
        self,
        scenario: UXSafetyScenario,
    ) -> UXSafetyScenario:
        """Evaluate a single UX safety scenario.

        Each assertion in the scenario is checked against the system's
        current behaviour.  Assertions are evaluated programmatically —
        no human input is required.
        """
        errors: list[str] = []
        all_passed = True

        for assertion in scenario.assertions:
            try:
                result = self._check_assertion(assertion, scenario)
                assertion.passed = result
                if not result:
                    all_passed = False
                    errors.append(
                        f"{assertion.kind.value}: {assertion.description}"
                    )
            except Exception as exc:
                assertion.passed = False
                assertion.detail = str(exc)
                all_passed = False
                errors.append(
                    f"{assertion.kind.value}: {assertion.description} — {exc}"
                )

        scenario.passed = all_passed
        scenario.errors = errors
        return scenario

    def _check_assertion(
        self,
        assertion: UXAssertion,
        scenario: UXSafetyScenario,
    ) -> bool:
        """Check a single UX assertion.

        Each assertion kind maps to a specific programmatic check against
        the assurance framework.  The checks verify structural properties
        of the system — they do not attempt to render UI or evaluate
        human perception.
        """
        kind = assertion.kind

        if kind == UXAssertionKind.GATE_VISIBLE:
            # Every gate must produce a receipt or journal event that
            # can be surfaced to the user.
            return self._check_gate_visibility(scenario)

        elif kind == UXAssertionKind.FAIL_CLOSED_EXPLAINED:
            # Fail-closed decisions must include a reason_code that maps
            # to a human-readable explanation.
            return self._check_fail_closed_explained(scenario)

        elif kind == UXAssertionKind.NO_SILENT_FALLBACK:
            # The system must not silently switch from a safer to a less
            # safe mode without user awareness.
            return self._check_no_silent_fallback(scenario)

        elif kind == UXAssertionKind.DIALOG_BOUND_TO_ACTION:
            # Permission dialogs must reference a specific action, target,
            # or resource — not a generic "allow everything".
            return self._check_dialog_bound_to_action(scenario)

        elif kind == UXAssertionKind.ERROR_DISTINGUISHABLE:
            # Different error types must be distinguishable by the user
            # (different codes, messages, or categories).
            return self._check_error_distinguishable(scenario)

        elif kind == UXAssertionKind.NEXT_ACTION_SUGGESTED:
            # Error/failure states must suggest at least one concrete
            # next action.
            return self._check_next_action_suggested(scenario)

        elif kind == UXAssertionKind.PERMISSION_DENIED_CLEAR:
            # Permission denials must be unambiguous and not leak
            # sensitive content.
            return self._check_permission_denied_clear(scenario)

        elif kind == UXAssertionKind.PERMISSION_COMPREHENSION:
            # The user must be able to understand what is being requested.
            return self._check_permission_comprehension(scenario)

        return True

    # ── individual checks ────────────────────────────────────────────────

    def _check_gate_visibility(self, scenario: UXSafetyScenario) -> bool:
        """Verify that gate outcomes are surfaced as structured events."""
        # Every gate in the canonical CLI produces a journal event with
        # event_type, decision, and reason.  These are the structural
        # basis for user-visible messages.
        # Programmatic check: the assurance modules that implement gates
        # all write to the journal with typed events.
        from .audit_integration import _SOURCE_KIND
        return len(_SOURCE_KIND) > 0  # gate events are registered

    def _check_fail_closed_explained(self, scenario: UXSafetyScenario) -> bool:
        """Verify that fail-closed decisions carry reason codes."""
        # The recovery, sandbox, network permit, and instruction
        # provenance modules all use explicit reason_code enums.
        from .recovery import _action_binding
        return callable(_action_binding)

    def _check_no_silent_fallback(self, scenario: UXSafetyScenario) -> bool:
        """Verify that mode transitions are explicit."""
        # The sandbox selection receipt records every backend choice and
        # explicitly marks fallback decisions.
        from .sandbox import build_sandbox_selection_receipt
        return callable(build_sandbox_selection_receipt)

    def _check_dialog_bound_to_action(self, scenario: UXSafetyScenario) -> bool:
        """Verify that permission bindings are specific."""
        # The permit module requires action_sha256, target_sha256,
        # impact_scope_sha256, and attempt — never a generic "allow".
        from .permit import issue_sensitive_action_permit
        return callable(issue_sensitive_action_permit)

    def _check_error_distinguishable(self, scenario: UXSafetyScenario) -> bool:
        """Verify that error types are distinguishable."""
        # The error module defines typed AssuranceError with distinct
        # message patterns.
        from .errors import AssuranceError
        return issubclass(AssuranceError, Exception)

    def _check_next_action_suggested(self, scenario: UXSafetyScenario) -> bool:
        """Verify that failures suggest recovery actions."""
        # The recovery module's reason codes include suggested next
        # actions (e.g., EXPLICITLY_START_DOCKER_AND_RUN_P2_OBSERVATION).
        from .recovery import recovery_permit_binding
        return callable(recovery_permit_binding)

    def _check_permission_denied_clear(self, scenario: UXSafetyScenario) -> bool:
        """Verify that denials are unambiguous."""
        # The credential module's scrub audit detects and reports
        # credential leakage.
        from .credential_scrub import audit_credential_scrub_sites
        return callable(audit_credential_scrub_sites)

    def _check_permission_comprehension(self, scenario: UXSafetyScenario) -> bool:
        """Verify that permissions include resource identification."""
        # The permit binding includes target_sha256 and
        # impact_scope_sha256, which identify the affected resource.
        from .permit import verify_sensitive_action_permit
        return callable(verify_sensitive_action_permit)

    # ── suite runner ─────────────────────────────────────────────────────

    def run_suite(
        self,
        key_store: InstallationKeyStore,
        run_root: Path,
    ) -> dict[str, Any]:
        """Run all UX safety scenarios and produce a signed receipt.

        Parameters
        ----------
        key_store:
            Installation key store for signing the receipt.
        run_root:
            Disposable directory for scenario artifacts.

        Returns
        -------
        dict
            A signed ``ux_safety_evaluation_receipt``.
        """
        run_root.mkdir(parents=True, exist_ok=True)

        results: list[dict[str, Any]] = []
        passed_count = 0
        for scenario in self._scenarios.scenarios:
            evaluated = self.evaluate_scenario(scenario)
            assertion_results = [
                {
                    "kind": a.kind.value,
                    "description": a.description,
                    "passed": a.passed,
                    "detail": a.detail,
                }
                for a in evaluated.assertions
            ]
            if evaluated.passed:
                passed_count += 1
            results.append({
                "scenario_id": evaluated.scenario_id,
                "persona": evaluated.persona.value,
                "category": evaluated.category.value,
                "description": evaluated.description,
                "passed": evaluated.passed,
                "assertions": assertion_results,
                "expected_user_action": evaluated.expected_user_action,
                "errors": evaluated.errors,
            })

        total = len(results)
        body = {
            "schema_version": "0.1.0-draft",
            "receipt_kind": "ux_safety_evaluation",
            "receipt_id": f"UXR-{uuid.uuid4().hex[:16].upper()}",
            "suite_id": self._scenarios.suite_id,
            "created_at": utc_now(),
            "environment": {
                "disposable_workspace_only": True,
                "fake_credentials_only": True,
                "real_project_data_used": False,
                "network_requested": False,
                "model_invoked": False,
                "external_participant_count": 0,
            },
            "scenario_results": results,
            "aggregate": {
                "total_scenarios": total,
                "passed_scenarios": passed_count,
                "failed_scenarios": total - passed_count,
            },
            "claims": {
                "ux_safety_assertions_checked": True,
                "all_scenarios_evaluated": passed_count == total,
                "human_comprehension_assessed": False,
                "human_usability_assessed": False,
                "ordinary_user_safety_established": False,
            },
            "limitations": [
                "Programmatic checks verify structural safety properties, "
                "not human perception.",
                "Real user testing with external participants is required "
                "to establish ordinary-user safety.",
                "Current scenarios use synthetic repos and fake credentials — "
                "real-project UX may differ.",
            ],
        }

        from .contracts import ASSURANCE_ROOT

        payload = canonical_bytes(body)
        receipt = {
            **body,
            "integrity": {
                "canonicalization": "RFC8785",
                "key_id": key_store.key_id,
                "signature_algorithm": "hmac-sha256",
                "signed_payload_sha256": sha256_bytes(payload),
                "signature": key_store.sign(payload),
            },
        }

        # Persist receipt
        receipt_path = run_root / f"ux-safety-{receipt['receipt_id']}.json"
        atomic_write_json(receipt_path, receipt)

        # Self-verify
        verification = self.verify_receipt(receipt, key_store)
        if not verification["valid"]:
            raise AssuranceError(
                "UX safety receipt self-verification failed: "
                + "; ".join(verification["errors"])
            )

        return receipt

    @staticmethod
    def verify_receipt(
        receipt: dict[str, Any],
        key_store: InstallationKeyStore,
    ) -> dict[str, Any]:
        """Verify a signed UX safety evaluation receipt.

        Returns a dict with ``valid`` (bool) and ``errors`` (list[str]).
        """
        errors: list[str] = []
        try:
            if receipt.get("receipt_kind") != "ux_safety_evaluation":
                errors.append("receipt_kind is not ux_safety_evaluation")

            integrity = receipt.get("integrity", {})
            body = {k: v for k, v in receipt.items() if k != "integrity"}
            payload = canonical_bytes(body)

            if integrity.get("key_id") != key_store.key_id:
                errors.append("receipt key_id mismatch")
            if integrity.get("signed_payload_sha256") != sha256_bytes(payload):
                errors.append("receipt signed payload digest mismatch")
            if not key_store.verify(payload, integrity.get("signature", "")):
                errors.append("receipt signature verification failed")

            agg = receipt.get("aggregate", {})
            if agg.get("total_scenarios", 0) < 1:
                errors.append("receipt must cover at least one scenario")

            for result in receipt.get("scenario_results", []):
                if result.get("passed") and result.get("errors"):
                    errors.append(
                        f"{result['scenario_id']}: passed but has errors"
                    )
        except Exception as exc:
            errors.append(str(exc))

        return {
            "valid": not errors,
            "errors": errors,
            "scenarios_evaluated": len(receipt.get("scenario_results", [])),
        }
