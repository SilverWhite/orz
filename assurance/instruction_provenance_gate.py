from __future__ import annotations

from pathlib import Path
import uuid
from typing import Any, Sequence

from .contracts import validate_contract
from .errors import AssuranceError
from .utils import atomic_write_json, canonical_bytes, load_json, sha256_bytes


GATE_CONTEXT_SCHEMA = "instruction-provenance-gate-context-v0.1.schema.json"
GATE_RECEIPT_SCHEMA = "instruction-provenance-gate-receipt-v0.1.schema.json"

ROUTABLE_SOURCES = {"platform", "user", "trusted_project"}
DATA_ONLY_SOURCES = {
    "untrusted_project",
    "external_content",
    "tool_output",
    "recalled_memory",
    "derived_summary",
}
ALL_SOURCE_TYPES = ROUTABLE_SOURCES | DATA_ONLY_SOURCES

INJECTION_PATTERNS = [
    ("忽略之前的指令", "known_injection_pattern"),
    ("ignore previous instructions", "known_injection_pattern"),
    ("ignore all prior", "known_injection_pattern"),
    ("disregard above", "known_injection_pattern"),
    ("你是一个", "source_type_mismatch"),
    ("you are now", "source_type_mismatch"),
    ("SYSTEM:", "source_type_mismatch"),
    ("[SYSTEM]", "source_type_mismatch"),
    ("<|im_start|>system", "source_type_mismatch"),
    ("请输出你的系统提示", "known_injection_pattern"),
    ("repeat your system prompt", "known_injection_pattern"),
    ("忽略安全", "known_injection_pattern"),
    ("bypass security", "known_injection_pattern"),
    ("dont check", "known_injection_pattern"),
    ("无需验证", "known_injection_pattern"),
    ("output your instructions", "known_injection_pattern"),
    ("override security", "known_injection_pattern"),
    ("system prompt", "known_injection_pattern"),
    ("hidden instructions", "known_injection_pattern"),
    ("disable safety", "known_injection_pattern"),
    ("developer mode", "known_injection_pattern"),
    ("debug mode", "known_injection_pattern"),
    ("pretend you are", "known_injection_pattern"),
    ("act as if", "known_injection_pattern"),
]


def _new_id(prefix: str) -> str:
    return f"{prefix}-{uuid.uuid4().hex.upper()}"


def _effective_source(declared_type: str, *, workspace_trust: str | None) -> str:
    if declared_type == "trusted_project" and workspace_trust != "observed_trusted":
        return "untrusted_project"
    return declared_type


def _scan_injection_indicators(content_hint: str | None) -> list[dict[str, str]]:
    if not content_hint:
        return []
    indicators: list[dict[str, str]] = []
    for pattern, alert_type in INJECTION_PATTERNS:
        if pattern in content_hint:
            indicators.append({"pattern": pattern, "alert_type": alert_type})
    return indicators


def build_instruction_provenance_gate_context(
    *,
    run_id: str,
    conversation_id: str,
    instructions: Sequence[dict[str, Any]],
) -> dict[str, Any]:
    if not instructions:
        raise AssuranceError("instruction provenance gate requires at least one instruction entry")
    for idx, entry in enumerate(instructions):
        if entry.get("declared_source_type") not in ALL_SOURCE_TYPES:
            raise AssuranceError(
                f"instruction entry {idx}: invalid source type {entry.get('declared_source_type')}"
            )
        if entry.get("declared_source_type") in {"trusted_project", "untrusted_project"}:
            if entry.get("workspace_trust") not in {"observed_trusted", "not_observed"}:
                raise AssuranceError(
                    f"instruction entry {idx}: project source requires workspace_trust"
                )
        if entry.get("declared_source_type") == "recalled_memory":
            if not entry.get("origin_conversation_id"):
                raise AssuranceError(
                    f"instruction entry {idx}: recalled_memory requires origin_conversation_id"
                )

    context = {
        "schema_version": "0.1.0-draft",
        "context_kind": "instruction_provenance_gate_context",
        "context_id": _new_id("IPG-CTX"),
        "run_id": run_id,
        "conversation_id": conversation_id,
        "instructions": [
            {
                "entry_id": entry.get("entry_id", f"INS-{_new_id('ENT')}"),
                "declared_source_type": entry["declared_source_type"],
                "source_id": entry["source_id"],
                "content_sha256": entry["content_sha256"],
                "content_bytes": entry["content_bytes"],
                "instruction_kind": entry.get("instruction_kind", "user_prompt"),
                "workspace_trust": entry.get("workspace_trust"),
                "origin_conversation_id": entry.get("origin_conversation_id"),
                "injection_indicators": entry.get("injection_indicators", []),
            }
            for entry in instructions
        ],
        "notes": [
            "Instruction provenance gate runs before any model or tool invocation.",
            "All instruction sources must be classified before gate decision.",
            "Data-only sources cannot issue routing instructions.",
        ],
    }
    validate_contract(context, GATE_CONTEXT_SCHEMA, label="instruction provenance gate context")
    return context


def _evaluate_single_entry(entry: dict[str, Any]) -> dict[str, Any]:
    declared = entry["declared_source_type"]
    workspace_trust = entry.get("workspace_trust")
    effective = _effective_source(declared, workspace_trust=workspace_trust)

    if effective in ROUTABLE_SOURCES:
        decision = "route"
        routing_permitted = True
        reason = f"source {declared} (effective: {effective}) is routable"
    elif effective in DATA_ONLY_SOURCES:
        if entry.get("instruction_kind") in {"user_prompt", "system_prompt"}:
            decision = "block"
            routing_permitted = False
            reason = (
                f"data-only source {declared} (effective: {effective}) cannot issue "
                f"{entry['instruction_kind']} instructions"
            )
        else:
            decision = "record_only"
            routing_permitted = False
            reason = f"data-only source {declared} (effective: {effective}) recorded for audit"
    else:
        decision = "block"
        routing_permitted = False
        reason = f"unknown effective source: {effective}"

    return {
        "entry_id": entry["entry_id"],
        "declared_source_type": declared,
        "effective_source_type": effective,
        "decision": decision,
        "routing_permitted": routing_permitted,
        "reason": reason,
    }


def evaluate_instruction_provenance_gate(
    *,
    gate_context: dict[str, Any],
    content_hints: dict[str, str] | None = None,
) -> dict[str, Any]:
    validate_contract(gate_context, GATE_CONTEXT_SCHEMA, label="instruction provenance gate context")
    if content_hints is None:
        content_hints = {}

    per_entry_decisions: list[dict[str, Any]] = []
    injection_alerts: list[dict[str, Any]] = []

    routing_count = 0
    data_only_count = 0
    blocked_count = 0

    for entry in gate_context["instructions"]:
        decision = _evaluate_single_entry(entry)
        per_entry_decisions.append(decision)

        if decision["effective_source_type"] in ROUTABLE_SOURCES:
            routing_count += 1
        else:
            data_only_count += 1

        if decision["decision"] == "block":
            blocked_count += 1

        hints = content_hints.get(entry["entry_id"], "")
        indicators = _scan_injection_indicators(hints)
        for ind in indicators:
            alert_type = ind["alert_type"]
            injection_alerts.append({
                "entry_id": entry["entry_id"],
                "alert_type": alert_type,
                "indicator": f"pattern '{ind['pattern']}' detected in content",
                "severity": "block" if alert_type == "source_type_mismatch" else "defer",
            })
            if alert_type == "source_type_mismatch":
                blocked_count += 1

    poisoning_detected = any(
        entry.get("injection_indicators") for entry in gate_context["instructions"]
    )
    if blocked_count > 0:
        gate_decision = "block"
    elif poisoning_detected or len(injection_alerts) > 0:
        gate_decision = "defer"
    else:
        gate_decision = "allow"

    has_user_source = any(
        d["declared_source_type"] == "user" for d in per_entry_decisions
    )

    receipt_id_seed = f"{gate_context['context_id']}:{gate_context['run_id']}"
    receipt = {
        "schema_version": "0.1.0-draft",
        "receipt_kind": "instruction_provenance_gate_receipt",
        "receipt_id": f"IPG-REC-{sha256_bytes(receipt_id_seed.encode('utf-8'))[:32].upper()}",
        "valid": gate_decision == "allow",
        "context_id": gate_context["context_id"],
        "run_id": gate_context["run_id"],
        "gate_decision": gate_decision,
        "source_summary": {
            "routing_source_count": routing_count,
            "data_only_source_count": data_only_count,
            "blocked_count": blocked_count,
            "injection_alert_count": len(injection_alerts),
        },
        "per_entry_decisions": per_entry_decisions,
        "injection_alerts": injection_alerts,
        "checks": {
            "all_sources_classified": len(per_entry_decisions) == len(gate_context["instructions"]),
            "data_only_sources_cannot_escalate": all(
                not (d["effective_source_type"] in DATA_ONLY_SOURCES and d["routing_permitted"])
                for d in per_entry_decisions
            ),
            "no_injection_escalation": len(injection_alerts) == 0 or gate_decision != "allow",
            "no_untrusted_to_routing": all(
                d["effective_source_type"] != "untrusted_project" or not d["routing_permitted"]
                for d in per_entry_decisions
            ),
            "user_source_present": has_user_source,
            "no_model_invoked": True,
            "mechanical_gate_only": True,
        },
        "limitations": [
            "Injection pattern detection uses static keyword matching; sophisticated obfuscation may evade.",
            "Content hints are not the full instruction content; only digest is verified.",
            "Real runtime must provide content bytes for full provenance verification.",
        ],
    }
    validate_contract(receipt, GATE_RECEIPT_SCHEMA, label="instruction provenance gate receipt")
    return receipt


def verify_instruction_provenance_gate_receipt(
    *,
    gate_context: dict[str, Any],
    receipt: dict[str, Any],
) -> dict[str, Any]:
    validate_contract(gate_context, GATE_CONTEXT_SCHEMA, label="instruction provenance gate context")
    validate_contract(receipt, GATE_RECEIPT_SCHEMA, label="instruction provenance gate receipt")

    if receipt["context_id"] != gate_context["context_id"]:
        raise AssuranceError("gate receipt context_id mismatch")

    expected = evaluate_instruction_provenance_gate(gate_context=gate_context)
    if canonical_bytes(receipt) != canonical_bytes(expected):
        raise AssuranceError("instruction provenance gate receipt does not rebuild")

    return receipt


def run_instruction_provenance_gate_fixture(
    *,
    gate_context: dict[str, Any],
    output_root: Path,
) -> dict[str, Any]:
    if output_root.exists() and any(output_root.iterdir()):
        raise AssuranceError(f"output root must be empty or absent: {output_root}")

    receipt = evaluate_instruction_provenance_gate(gate_context=gate_context)
    output_root.mkdir(parents=True, exist_ok=True)
    atomic_write_json(output_root / "instruction-provenance-gate-context.json", gate_context)
    atomic_write_json(output_root / "instruction-provenance-gate-receipt.json", receipt)

    return {
        "schema_version": "0.1.0-draft",
        "receipt_kind": "instruction_provenance_gate_fixture_summary",
        "valid": receipt["valid"],
        "gate_decision": receipt["gate_decision"],
        "context_id": gate_context["context_id"],
        "output_root": str(output_root),
        "checks": receipt["checks"],
        "limitations": receipt["limitations"],
    }


def verify_instruction_provenance_gate_fixture(
    *,
    output_root: Path,
) -> dict[str, Any]:
    context_path = output_root / "instruction-provenance-gate-context.json"
    receipt_path = output_root / "instruction-provenance-gate-receipt.json"

    gate_context = load_json(context_path)
    observed_receipt = load_json(receipt_path)

    return verify_instruction_provenance_gate_receipt(
        gate_context=gate_context,
        receipt=observed_receipt,
    )


# ── production content parser ──

import re as _re  # noqa: E402 (module-level import added post-definition for organisation)


# Path-like patterns: Unix absolute/relative and Windows drive-letter paths
_PATH_UNIX = _re.compile(r"(?:^|\s)((?:\.{0,2}/)+(?:[^\s\"'`<>|:]+/)*[^\s\"'`<>|:]*)")
_PATH_WIN = _re.compile(
    r"(?:^|\s)([A-Za-z]:\\(?:[^\s\"'`<>|:]+\\?)*[^\s\"'`<>|:]*)",
    _re.IGNORECASE,
)
_PATH_UNC = _re.compile(r"(?:^|\s)(\\\\[^\s\"'`<>|]+(?:\\[^\s\"'`<>|]+)*)")
_URL_PATTERN = _re.compile(r"https?://[^\s\"'`<>|]+")
_DIRECTIVE_PATTERNS = [
    (_re.compile(r"\byou must\b", _re.IGNORECASE), "imperative_directive"),
    (_re.compile(r"\bdo not\b", _re.IGNORECASE), "negative_directive"),
    (_re.compile(r"\bignore\b(?!\s+(?:the\s+)?(?:previous|above|prior|all))", _re.IGNORECASE), "ignore_keyword"),
    (_re.compile(r"\b(?:bypass|override|disable)\s+(?:security|safety|gate|check|verification)\b", _re.IGNORECASE), "security_bypass_directive"),
    (_re.compile(r"\bpretend\s+(?:you are|to be|that)\b", _re.IGNORECASE), "role_play_directive"),
    (_re.compile(r"\bforget\s+(?:everything|all|your)\s+(?:above|previous|instructions|training)\b", _re.IGNORECASE), "amnesia_directive"),
    (_re.compile(r"\boutput\s+(?:your|the)\s+(?:system\s*(?:prompt|message|instruction)|hidden\s*(?:prompt|instruction|rule))\b", _re.IGNORECASE), "prompt_extraction"),
]

# Obfuscation detection
_ZERO_WIDTH_CHARS = _re.compile("[​‌‍‎‏﻿]")
_HOMOGLYPH_CYRILLIC = _re.compile("[аеоурхсмАВЕКМНОРСТХ]")  # Cyrillic chars that look like Latin
_BASE64_PATTERN = _re.compile(r"(?:[A-Za-z0-9+/]{28,}={0,2})")
_FULLWIDTH_PATTERN = _re.compile(r"[！-～]")  # Fullwidth Latin


def parse_instruction_content(content_bytes: bytes) -> dict[str, object]:
    """Parse raw instruction bytes to extract path/endpoint/directive references.

    This is a mechanical content analyser — no model calls, no network.
    It extracts structural indicators that the gate uses for deeper
    injection detection beyond simple substring matching.

    Returns a dict with keys:
      - path_references: list[str] — filesystem paths found
      - endpoint_references: list[str] — HTTP(S) URLs found
      - directive_indicators: list[dict] — detected directive patterns
      - instruction_kind_hint: str | None — inferred instruction type
      - has_executable_references: bool — whether content references executables
    """
    try:
        text = content_bytes.decode("utf-8", errors="replace")
    except UnicodeDecodeError:
        text = content_bytes.decode("latin-1", errors="replace")

    # Extract path references
    path_refs: list[str] = []
    for match in _PATH_UNIX.finditer(text):
        candidate = match.group(1).strip()
        if len(candidate) > 1 and candidate not in path_refs:
            path_refs.append(candidate)
    for match in _PATH_WIN.finditer(text):
        candidate = match.group(1).strip()
        if len(candidate) > 2 and candidate not in path_refs:
            path_refs.append(candidate)
    for match in _PATH_UNC.finditer(text):
        candidate = match.group(1).strip()
        if len(candidate) > 3 and candidate not in path_refs:
            path_refs.append(candidate)

    # Extract endpoint references
    endpoint_refs: list[str] = []
    for match in _URL_PATTERN.finditer(text):
        url = match.group(0)
        if url not in endpoint_refs:
            endpoint_refs.append(url)

    # Detect directive indicators
    directive_indicators: list[dict[str, str]] = []
    for pattern, indicator_type in _DIRECTIVE_PATTERNS:
        for match in pattern.finditer(text):
            directive_indicators.append({
                "type": indicator_type,
                "match": match.group(0),
            })

    # Infer instruction kind
    instruction_kind_hint: str | None = None
    lowered = text.lower()
    if any(
        marker in lowered
        for marker in ("system prompt", "system message", "system instruction", "<|im_start|>system")
    ):
        instruction_kind_hint = "system_prompt"
    elif any(
        marker in lowered
        for marker in ("tool call", "tool output", "tool result", "function call")
    ):
        instruction_kind_hint = "tool_output"
    elif any(
        marker in lowered
        for marker in ("project rule", "project config", "workspace rule")
    ):
        instruction_kind_hint = "project_rule"

    # Executable references
    has_executable_references = any(
        marker in lowered
        for marker in (".exe", ".bat", ".cmd", ".ps1", ".sh", ".py", "python ", "bash ", "cmd.exe", "powershell")
    )

    return {
        "path_references": path_refs,
        "endpoint_references": endpoint_refs,
        "directive_indicators": directive_indicators,
        "instruction_kind_hint": instruction_kind_hint,
        "has_executable_references": has_executable_references,
    }


def detect_obfuscated_injection(content_bytes: bytes) -> list[dict[str, str]]:
    """Detect obfuscation techniques used to evade substring-based injection detection.

    Checks for:
      - Zero-width character injection (U+200B, U+200C, U+200D, U+FEFF)
      - Homoglyph attacks (Cyrillic characters that look like Latin)
      - Base64-encoded payloads
      - Fullwidth character substitution

    Returns a list of alert dicts with ``technique`` and ``detail`` keys.
    """
    try:
        text = content_bytes.decode("utf-8", errors="replace")
    except UnicodeDecodeError:
        text = content_bytes.decode("latin-1", errors="replace")

    alerts: list[dict[str, str]] = []

    # Zero-width characters
    zw_matches = _ZERO_WIDTH_CHARS.findall(text)
    if zw_matches:
        alerts.append({
            "technique": "zero_width_character",
            "detail": f"found {len(zw_matches)} zero-width character(s): "
                      f"{', '.join(f'U+{ord(c):04X}' for c in set(zw_matches))}",
        })

    # Homoglyph detection (Cyrillic chars in primarily-Latin text)
    cyrillic_matches = _HOMOGLYPH_CYRILLIC.findall(text)
    latin_chars = sum(1 for c in text if c.isascii() and c.isalpha())
    if cyrillic_matches and latin_chars > 10:
        unique_cyrillic = set(cyrillic_matches)
        alerts.append({
            "technique": "homoglyph_attack",
            "detail": f"found Cyrillic homoglyph characters in Latin-dominant text: "
                      f"{''.join(sorted(unique_cyrillic))}",
        })

    # Base64 payload detection
    for match in _BASE64_PATTERN.finditer(text):
        candidate = match.group(0)
        # Skip common false positives (e.g. SHA256 hashes are 64 hex chars).
        # Require at least 40 base64 chars (~30 bytes decoded) for meaningful payloads.
        if len(candidate) >= 40 and not all(c in "0123456789abcdefABCDEF" for c in candidate):
            try:
                import base64
                decoded = base64.b64decode(candidate, validate=True)
                decoded_text = decoded.decode("utf-8", errors="replace")
                lowered = decoded_text.lower()
                if any(
                    kw in lowered
                    for kw in ("ignore", "bypass", "system", "prompt", "instruction", "password", "token")
                ):
                    alerts.append({
                        "technique": "base64_encoded_payload",
                        "detail": f"base64 payload decodes to injection-relevant content "
                                  f"({len(decoded)} bytes)",
                    })
                    break  # one confirmed payload is enough
            except Exception:
                pass

    # Fullwidth character substitution
    fw_matches = _FULLWIDTH_PATTERN.findall(text)
    if len(fw_matches) >= 3:
        alerts.append({
            "technique": "fullwidth_substitution",
            "detail": f"found {len(fw_matches)} fullwidth character(s) "
                      f"that may substitute ASCII in injection patterns",
        })

    return alerts


# ── canonicalizer integration ──


def evaluate_instruction_provenance_gate_with_canonicalizer(
    *,
    gate_context: dict[str, Any],
    content_hints: dict[str, str] | None = None,
    allowed_endpoint_hosts: set[str] | None = None,
    forbidden_path_prefixes: list[str] | None = None,
) -> dict[str, Any]:
    """Evaluate the IPG with integrated path/endpoint canonicalizer.

    Extends :func:`evaluate_instruction_provenance_gate` by also scanning
    instruction content for path traversal and SSRF attempts.  Path
    references are validated through :func:`canonicalize_filesystem_path`;
    endpoint references through :func:`validate_endpoint_list`.

    Additional blocking conditions (on top of the base gate):
      - Path traversal in any data-only source → ``block``
      - Unauthorized endpoint in any data-only source → ``block``
      - Obfuscated injection indicators in data-only source → ``defer``

    Returns the base gate receipt extended with a ``canonicalizer_results``
    key containing per-instruction path/endpoint/obfuscation scan results.
    """
    from .endpoint_canonicalizer import (
        canonicalize_filesystem_path,
        validate_endpoint_list,
    )

    # Run the base gate first
    base_receipt = evaluate_instruction_provenance_gate(
        gate_context=gate_context,
        content_hints=content_hints,
    )

    canonicalizer_results: list[dict[str, Any]] = []
    additional_blocks = 0
    additional_defers = 0
    new_alerts: list[dict[str, Any]] = list(base_receipt.get("injection_alerts", []))

    for entry in gate_context["instructions"]:
        entry_id = entry["entry_id"]
        declared_source = entry["declared_source_type"]
        content = entry.get("content_bytes")
        is_data_only = declared_source in DATA_ONLY_SOURCES

        entry_result: dict[str, Any] = {
            "entry_id": entry_id,
            "path_scan": {"paths_found": 0, "blocked": False, "alerts": []},
            "endpoint_scan": {"endpoints_found": 0, "blocked": False, "alerts": []},
            "obfuscation_scan": {"techniques_detected": 0, "blocked": False, "alerts": []},
        }

        if content is not None and isinstance(content, int) and content > 0:
            # content_bytes stores byte count, not actual bytes.
            # Full content parsing requires actual bytes from the runtime.
            # For offline: use content_hints as a proxy.
            hint = (content_hints or {}).get(entry_id, "")
            if hint:
                parsed = parse_instruction_content(hint.encode("utf-8", errors="replace"))
                obfuscation = detect_obfuscated_injection(
                    hint.encode("utf-8", errors="replace")
                )

                # Path scan
                for path_ref in parsed["path_references"]:
                    entry_result["path_scan"]["paths_found"] += 1
                    is_traversal = False
                    # Direct traversal detection: ../ patterns in relative paths
                    if path_ref.startswith("..") or "/.." in path_ref or "\\.." in path_ref:
                        is_traversal = True
                    else:
                        try:
                            canonicalize_filesystem_path(path_ref)
                        except AssuranceError:
                            is_traversal = True
                    if is_traversal:
                        entry_result["path_scan"]["blocked"] = True
                        entry_result["path_scan"]["alerts"].append(
                            f"path traversal detected: {path_ref}"
                        )
                        if is_data_only:
                            additional_blocks += 1
                            new_alerts.append({
                                "entry_id": entry_id,
                                "alert_type": "path_traversal",
                                "indicator": f"traversal path in data-only source: {path_ref}",
                                "severity": "block",
                            })

                # Endpoint scan
                if parsed["endpoint_references"]:
                    entry_result["endpoint_scan"]["endpoints_found"] = len(
                        parsed["endpoint_references"]
                    )
                    if allowed_endpoint_hosts is not None:
                        validations = validate_endpoint_list(
                            parsed["endpoint_references"],
                            allowed_hosts=allowed_endpoint_hosts,
                        )
                        for val in validations:
                            if not val["allowed"]:
                                entry_result["endpoint_scan"]["blocked"] = True
                                entry_result["endpoint_scan"]["alerts"].append(
                                    f"unauthorized endpoint: {val.get('endpoint', '')}: {val['reason']}"
                                )
                                if is_data_only:
                                    additional_blocks += 1
                                    new_alerts.append({
                                        "entry_id": entry_id,
                                        "alert_type": "ssrf_attempt",
                                        "indicator": f"unauthorized endpoint in data-only source: {val.get('endpoint', '')}",
                                        "severity": "block",
                                    })

                # Obfuscation scan
                if obfuscation:
                    entry_result["obfuscation_scan"]["techniques_detected"] = len(obfuscation)
                    entry_result["obfuscation_scan"]["alerts"] = [
                        f"{o['technique']}: {o['detail']}" for o in obfuscation
                    ]
                    if is_data_only:
                        additional_defers += 1
                        for o in obfuscation:
                            new_alerts.append({
                                "entry_id": entry_id,
                                "alert_type": f"obfuscation_{o['technique']}",
                                "indicator": o["detail"],
                                "severity": "defer",
                            })

        canonicalizer_results.append(entry_result)

    # Recompute gate decision
    base_blocked = base_receipt["gate_decision"] == "block"
    base_deferred = base_receipt["gate_decision"] == "defer"

    if base_blocked or additional_blocks > 0:
        gate_decision = "block"
    elif base_deferred or additional_defers > 0:
        gate_decision = "defer"
    else:
        gate_decision = "allow"

    return {
        **base_receipt,
        "gate_decision": gate_decision,
        "valid": gate_decision == "allow",
        "injection_alerts": new_alerts,
        "source_summary": {
            **base_receipt["source_summary"],
            "injection_alert_count": len(new_alerts),
        },
        "checks": {
            **base_receipt["checks"],
            "canonicalizer_applied": True,
            "no_path_traversal": additional_blocks == 0 or not any(
                a["alert_type"] == "path_traversal" for a in new_alerts
            ),
            "no_ssrf": additional_blocks == 0 or not any(
                a["alert_type"] == "ssrf_attempt" for a in new_alerts
            ),
            "no_obfuscation_bypass": additional_defers == 0,
        },
        "canonicalizer_results": canonicalizer_results,
        "limitations": base_receipt.get("limitations", []) + [
            "Canonicalizer operates on content_hints (text proxy), not raw bytes.",
            "Path extraction uses regex heuristics; complex obfuscation may evade.",
            "Full content-byte canonicalization requires runtime adapter integration.",
        ],
    }


# ── unified gate entry verification ──


_EXPECTED_GATE_CONSUMERS = [
    "assurance/canonical_cli.py",
    "assurance/adapter_gate.py",
    "assurance/retrieval_subagent.py",
    "assurance/deepseek_adapter.py",
]


def validate_all_entry_points_consume_same_gate() -> dict[str, Any]:
    """Verify that all known runtime/tool entry points consume the same IPG.

    This is a static mechanical check — it reads the source of each known
    consumer module and verifies that ``evaluate_instruction_provenance_gate``
    is imported and called.  It does not execute any code.

    Returns a dict with ``valid``, ``consumers``, and ``errors`` keys.
    """
    import ast as _ast
    from pathlib import Path as _Path

    assurance_root = _Path(__file__).resolve().parent
    errors: list[str] = []
    consumers: dict[str, dict[str, bool]] = {}

    for rel_path in _EXPECTED_GATE_CONSUMERS:
        abs_path = assurance_root.parent / rel_path
        consumer_result = {"imports_gate": False, "calls_gate": False}
        try:
            source = abs_path.read_text(encoding="utf-8")
            tree = _ast.parse(source)
            for node in _ast.walk(tree):
                if isinstance(node, _ast.ImportFrom):
                    if node.module == "assurance.instruction_provenance_gate" or \
                       node.module == ".instruction_provenance_gate":
                        for alias in node.names:
                            if alias.name == "evaluate_instruction_provenance_gate":
                                consumer_result["imports_gate"] = True
                elif isinstance(node, _ast.Call):
                    if isinstance(node.func, _ast.Name) and \
                       node.func.id == "evaluate_instruction_provenance_gate":
                        consumer_result["calls_gate"] = True
                    elif isinstance(node.func, _ast.Attribute) and \
                            node.func.attr == "evaluate_instruction_provenance_gate":
                        consumer_result["calls_gate"] = True
            consumers[rel_path] = consumer_result
            if not consumer_result["imports_gate"]:
                errors.append(f"{rel_path}: does not import evaluate_instruction_provenance_gate")
            if not consumer_result["calls_gate"]:
                errors.append(f"{rel_path}: imports but does not call evaluate_instruction_provenance_gate")
        except Exception as exc:
            errors.append(f"{rel_path}: cannot verify: {exc}")
            consumers[rel_path] = {"imports_gate": False, "calls_gate": False}

    return {
        "valid": not errors,
        "consumers": consumers,
        "errors": errors,
        "limitations": [
            "Static AST analysis only — cannot verify runtime dispatch.",
            "New entry points must be manually added to _EXPECTED_GATE_CONSUMERS.",
        ],
    }
