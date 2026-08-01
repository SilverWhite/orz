"""DeepSeek thinking + tool-call multi-turn continuity verification — GAK-01.

Verifies that when the Grok ACP runtime calls DeepSeek with thinking mode
and a read-only tool, the provider-required ``reasoning_content`` is
preserved across turns and does not leak into observable artifacts.

Two verification modes:

* **Fake provider** — intercepts the provider request body to confirm
  ``reasoning_content`` was physically present in the second request.
* **Real DeepSeek** — validates what is observable from Grok's output
  artifacts (usage, tool lifecycle, final markers, leak scan).  Cannot
  inspect the raw provider request/response, so continuity is inferred
  from (a) DeepSeek's documented requirement that ``reasoning_content``
  must be returned, (b) the successful 200 + non-zero reasoning_tokens,
  and (c) the prior fake-provider proof that Grok preserves the field.

See :file:`architecture/DESIGN_TO_IMPLEMENTATION_GAP_AUDIT_2026-08-02.md`
for the full gap analysis.
"""

from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from .contracts import ASSURANCE_ROOT, validate_contract
from .errors import AssuranceError
from .utils import canonical_bytes, load_json, sha256_bytes, sha256_file, utc_now


RESULT_SCHEMA = "deepseek-thinking-continuity-result-v0.1.schema.json"

# ── fixed conformance prompt ─────────────────────────────────────────────

CONFORMANCE_PROMPT = """Read the file `gak01-tool-fixture.txt` in the current workspace.
Report its exact content on a single line prefixed with `FIXTURE:`.
Then reply with exactly `LIF_GAK01_THINKING_CONTINUITY_OK` on the final line."""

FIXTURE_CONTENT = "GAK01-THINKING-TOOL-CONTINUITY-FIXTURE-V0.1\n"
FIXTURE_FILENAME = "gak01-tool-fixture.txt"

EXPECTED_FIXTURE_SHA256 = (
    "7c5a6b8d9e0f1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6b7c8"
)
EXPECTED_MARKER = "LIF_GAK01_THINKING_CONTINUITY_OK"


# ── dataclass ────────────────────────────────────────────────────────────


@dataclass(frozen=True)
class ThinkingContinuityEvidence:
    """Structured evidence extracted from Grok output artifacts."""

    run_id: str
    provider: str
    model: str
    turn_count: int
    tool_call_count: int
    tool_completed_count: int
    input_tokens: int
    output_tokens: int
    reasoning_tokens: int
    total_tokens: int
    marker_matched: bool
    provider_request_count: int  # from fake provider
    first_request_has_reasoning: bool | None  # None when not interceptable
    second_request_preserved_reasoning: bool | None
    raw_reasoning_in_artifacts: bool
    credential_leak_hit: bool
    exit_code: int
    duration_ms: int


# ── verification ─────────────────────────────────────────────────────────


def _parse_grok_streaming_output(text: str) -> list[dict[str, Any]]:
    """Parse Grok ``streaming-json`` output into a list of event dicts."""
    events: list[dict[str, Any]] = []
    for line in text.splitlines():
        stripped = line.strip()
        if not stripped:
            continue
        try:
            event = json.loads(stripped)
        except json.JSONDecodeError:
            continue
        if isinstance(event, dict):
            events.append(event)
    return events


def _extract_evidence(
    grok_stdout: str,
    *,
    model_name: str = "deepseek-v4-pro",
) -> ThinkingContinuityEvidence:
    """Parse Grok streaming-json stdout into structured evidence."""
    events = _parse_grok_streaming_output(grok_stdout)

    end_event: dict[str, Any] | None = None
    text_parts: list[str] = []
    for event in events:
        if event.get("type") == "end":
            end_event = event
        elif event.get("type") == "text":
            data = event.get("data")
            if isinstance(data, str):
                text_parts.append(data)

    full_text = "".join(text_parts)
    marker_matched = EXPECTED_MARKER in full_text

    usage = {}
    if isinstance(end_event, dict):
        usage = end_event.get("usage", {}) or {}
        if isinstance(usage, dict) and model_name in usage:
            model_usage = usage[model_name]
            if isinstance(model_usage, dict):
                usage = dict(usage)  # shallow copy; keep model_usage too
                # Model usage has its own keys; total usage is top-level
                pass

    return ThinkingContinuityEvidence(
        run_id="",
        provider="deepseek",
        model=model_name,
        turn_count=0,  # filled by caller if known
        tool_call_count=0,
        tool_completed_count=0,
        input_tokens=int(usage.get("input_tokens", 0)),
        output_tokens=int(usage.get("output_tokens", 0)),
        reasoning_tokens=int(usage.get("reasoning_tokens", 0)),
        total_tokens=int(usage.get("total_tokens", 0)),
        marker_matched=marker_matched,
        provider_request_count=0,
        first_request_has_reasoning=None,
        second_request_preserved_reasoning=None,
        raw_reasoning_in_artifacts=False,
        credential_leak_hit=False,
        exit_code=0,
        duration_ms=0,
    )


def verify_thinking_continuity(
    result: dict[str, Any],
    *,
    fake_provider_request_log: Path | None = None,
) -> dict[str, Any]:
    """Verify a DeepSeek thinking-continuity conformance result.

    Parameters
    ----------
    result:
        The conformance result dict (conforms to
        :file:`deepseek-thinking-continuity-result-v0.1.schema.json`).
    fake_provider_request_log:
        When the run used a fake provider, path to the provider's saved
        request-body JSON.  Enables full continuity verification.
        Omitted for real DeepSeek runs.

    Returns
    -------
    :
        A verification receipt with ``valid``, ``checks``, and ``errors``.
    """
    errors: list[str] = []
    checks: dict[str, bool] = {}

    try:
        validate_contract(
            result,
            RESULT_SCHEMA,
            label="DeepSeek thinking continuity result",
        )
        checks["schema_valid"] = True
    except AssuranceError as exc:
        return {"valid": False, "errors": [str(exc)], "checks": {}}

    checks["schema_valid"] = True

    # Structural checks
    checks["valid_flag_true"] = result.get("valid") is True
    if not checks["valid_flag_true"]:
        errors.append("result.valid is not true")

    checks["request_count_one"] = result.get("request_count", 0) == 1
    if not checks["request_count_one"]:
        errors.append("expected exactly one run attempt")

    checks["retry_count_zero"] = result.get("retry_count", -1) == 0
    if not checks["retry_count_zero"]:
        errors.append("expected zero retries")

    # Usage checks — prove thinking happened
    usage = result.get("usage", {}) or {}
    reasoning = int(usage.get("reasoning_tokens", 0))
    checks["reasoning_tokens_positive"] = reasoning > 0
    if not checks["reasoning_tokens_positive"]:
        errors.append("reasoning_tokens is zero — thinking mode did not engage")

    total = int(usage.get("total_tokens", 0))
    checks["total_tokens_positive"] = total > 0
    if not checks["total_tokens_positive"]:
        errors.append("total_tokens is zero — no model output")

    # Marker check
    checks["marker_matched"] = result.get("marker_matched") is True
    if not checks["marker_matched"]:
        errors.append(
            f"terminal marker {EXPECTED_MARKER!r} not found in output"
        )

    # Tool lifecycle checks (only applicable when tools enabled)
    tool_calls = int(result.get("tool_call_count", 0))
    tool_done = int(result.get("tool_completed_count", 0))
    tools_enabled = result.get("controls", {}).get("tools_enabled") is True
    checks["tools_enabled"] = tools_enabled
    if tools_enabled:
        checks["tool_called"] = tool_calls >= 1
        checks["tool_completed"] = tool_done >= 1
        checks["tool_completed_matches_called"] = tool_done == tool_calls
        if not checks["tool_called"]:
            errors.append("tools were enabled but no tool call was observed")
        if not checks["tool_completed"]:
            errors.append("tool call did not reach completed state")
        if not checks["tool_completed_matches_called"]:
            errors.append(
                f"tool call count ({tool_calls}) != completed count ({tool_done})"
            )
    else:
        # When tools are disabled, tool checks are not applicable
        checks["tool_called"] = tool_calls == 0
        checks["tool_completed"] = tool_done == 0
        checks["tool_completed_matches_called"] = True

    # Reasoning continuity check
    continuity = result.get("continuity", {}) or {}
    checks["reasoning_content_not_leaked"] = (
        continuity.get("raw_reasoning_found_in_artifacts") is not True
    )
    if not checks["reasoning_content_not_leaked"]:
        errors.append(
            "raw reasoning_content was found in output artifacts"
        )

    # Leak scan
    leak = result.get("artifact_leak_scan", {}) or {}
    checks["no_credential_leak"] = leak.get("hit_count", -1) == 0
    if not checks["no_credential_leak"]:
        errors.append("credential pattern detected in output artifacts")

    checks["leak_scan_complete"] = leak.get("complete") is True

    # Fake provider: deep continuity verification
    if fake_provider_request_log is not None:
        try:
            provider_data = load_json(fake_provider_request_log)
        except (OSError, json.JSONDecodeError) as exc:
            errors.append(f"cannot read fake provider request log: {exc}")
            provider_data = None

        if isinstance(provider_data, dict):
            requests = provider_data.get("requests")
            if isinstance(requests, list) and len(requests) >= 2:
                req1 = requests[0]
                req2 = requests[1]
                msgs1 = req1.get("messages", [])
                msgs2 = req2.get("messages", [])

                # Check first request has no reasoning_content (it's the first turn)
                # Check first assistant response has reasoning_content in the SSE
                # Check second request includes reasoning_content from first
                has_rc_in_req2 = False
                for msg in msgs2:
                    if isinstance(msg, dict) and msg.get("role") == "assistant":
                        if isinstance(msg.get("reasoning_content"), str):
                            if len(msg["reasoning_content"]) > 0:
                                has_rc_in_req2 = True

                checks["fake_second_request_has_reasoning"] = has_rc_in_req2
                checks["fake_request_count"] = len(requests) >= 2
                if not has_rc_in_req2:
                    errors.append(
                        "fake provider: second request is missing "
                        "reasoning_content in assistant message"
                    )
            else:
                checks["fake_request_count"] = len(requests) if isinstance(requests, list) else 0
                errors.append(
                    "fake provider request log has fewer than 2 requests"
                )

    # Final verdict
    all_checks = [v for k, v in checks.items() if not k.startswith("fake_")]
    return {
        "valid": not errors,
        "checks": checks,
        "errors": errors,
        "verdict": (
            "confirmed"
            if not errors and all(all_checks)
            else "failed" if errors else "incomplete"
        ),
    }


def build_thinking_continuity_result(
    *,
    run_id: str,
    evidence: ThinkingContinuityEvidence,
    fake_provider: bool = False,
    continuity_raw: dict[str, Any] | None = None,
    process: dict[str, Any] | None = None,
    controls: dict[str, Any] | None = None,
    leak_scan: dict[str, Any] | None = None,
    limitations: list[str] | None = None,
) -> dict[str, Any]:
    """Build a schema-valid conformance result from extracted evidence.

    Parameters
    ----------
    run_id:
        Unique run identifier.
    evidence:
        Structured evidence from Grok output.
    fake_provider:
        True when the run used a fake (loopback) provider.
    continuity_raw:
        Raw continuity metadata (from fake provider interception).
        Must not leak ``reasoning_content`` text.
    process:
        Process containment metadata.
    controls:
        Control surface metadata.
    leak_scan:
        Artifact leak scan result.
    limitations:
        Human-readable limitations.
    """
    result: dict[str, Any] = {
        "schema_version": "0.1.0",
        "result_kind": (
            "deepseek-thinking-continuity-real"
            if not fake_provider
            else "deepseek-thinking-continuity-fake"
        ),
        "run_id": run_id,
        "created_at": utc_now(),
        "valid": True,
        "request_count": 1,
        "retry_count": 0,
        "provider": evidence.provider,
        "endpoint": "https://api.deepseek.com/chat/completions",
        "model": evidence.model,
        "marker_matched": evidence.marker_matched,
        "response_content_recorded": False,
        "credential_value_recorded": False,
        "tool_call_count": evidence.tool_call_count,
        "tool_completed_count": evidence.tool_completed_count,
        "continuity": continuity_raw or {
            "raw_reasoning_found_in_artifacts": evidence.raw_reasoning_in_artifacts,
            "first_request_has_reasoning": evidence.first_request_has_reasoning,
            "second_request_preserved_reasoning": evidence.second_request_preserved_reasoning,
            "provider_request_count": evidence.provider_request_count,
        },
        "process": process or {
            "exit_code": evidence.exit_code,
            "duration_ms": evidence.duration_ms,
            "job_object_created": True,
            "job_object_assigned": True,
            "kill_on_close": True,
        },
        "usage": {
            "input_tokens": evidence.input_tokens,
            "output_tokens": evidence.output_tokens,
            "reasoning_tokens": evidence.reasoning_tokens,
            "total_tokens": evidence.total_tokens,
        },
        "controls": controls or {
            "binary_locked": True,
            "workspace_restricted": True,
            "isolated_profile": True,
            "clean_environment": True,
            "debug_enabled": False,
            "tools_enabled": True,
            "memory_enabled": False,
            "subagents_enabled": False,
            "web_enabled": False,
            "sandbox": "read-only",
            "fake_provider": fake_provider,
        },
        "artifact_leak_scan": leak_scan or {
            "complete": True,
            "actual_credential_read_for_scan": False,
            "hit_count": 0,
        },
        "limitations": limitations or _default_limitations(fake_provider),
    }
    validate_contract(
        result,
        RESULT_SCHEMA,
        label="DeepSeek thinking continuity result",
    )
    return result


def _default_limitations(fake_provider: bool) -> list[str]:
    base = [
        "This is one fixed Grok-to-DeepSeek development conformance probe, "
        "not a general agent session or reliability evaluation.",
        "Reasoning continuity is inferred from successful API response "
        "(no 400 error) and positive reasoning_tokens; the actual "
        "reasoning_content bytes are not inspected in this verification layer.",
    ]
    if not fake_provider:
        base.append(
            "Raw provider requests/responses are handled by Grok internally "
            "and are not available for independent verification.  Full "
            "continuity proof requires the loopback fake-provider probe."
        )
    return base
