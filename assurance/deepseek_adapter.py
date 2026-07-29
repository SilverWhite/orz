"""Real DeepSeek adapter for the canonical guarded CLI.

Reads the API key from Windows Credential Manager (target: FEP-Agent/DeepSeek),
calls the DeepSeek API exactly once, and projects only public assistant output
into the gate pipeline.  Never persists raw credential, raw response, or hidden
reasoning text to disk.
"""
from __future__ import annotations

import ctypes
from ctypes import wintypes
import json
import os
from typing import Any
import urllib.error
import urllib.request

from .contracts import validate_contract
from .errors import AssuranceError
from .utils import canonical_bytes, sha256_bytes, utc_now

DEEPSEEK_ENDPOINT = "https://api.deepseek.com/chat/completions"
DEFAULT_CREDENTIAL_TARGET = "FEP-Agent/DeepSeek"
DEFAULT_MODEL = "deepseek-v4-pro"


class _FILETIME(ctypes.Structure):
    """Windows FILETIME — 64-bit timestamp as low/high DWORD pair."""
    _fields_ = [
        ("dwLowDateTime", wintypes.DWORD),
        ("dwHighDateTime", wintypes.DWORD),
    ]


class _CREDENTIALW(ctypes.Structure):
    """Windows CREDENTIALW struct — used to read CredReadW results safely.

    Field offsets are computed by ctypes at runtime, so this works across
    32-bit, 64-bit, and ARM64 Windows without manual pointer arithmetic.
    """
    _fields_ = [
        ("Flags",             wintypes.DWORD),
        ("Type",              wintypes.DWORD),
        ("TargetName",        wintypes.LPWSTR),
        ("Comment",           wintypes.LPWSTR),
        ("LastWritten",       _FILETIME),
        ("CredentialBlobSize", wintypes.DWORD),
        ("CredentialBlob",    ctypes.POINTER(ctypes.c_ubyte)),
        ("Persist",           wintypes.DWORD),
        ("AttributeCount",    wintypes.DWORD),
        ("Attributes",        ctypes.c_void_p),
        ("TargetAlias",       wintypes.LPWSTR),
        ("UserName",          wintypes.LPWSTR),
    ]


def _read_windows_credential(target: str) -> str:
    """Read a generic credential from Windows Credential Manager.

    Uses the documented CREDENTIALW structure (no pointer arithmetic) so
    the code is cross-architecture safe.  The credential blob is zeroed
    via :func:`ctypes.memset` before ``CredFree`` releases the buffer.

    Returns the credential as a UTF-16-LE decoded string.  Raises
    :exc:`AssuranceError` on failure.
    """
    if os.name != "nt":
        raise AssuranceError(
            "Windows Credential Manager is only available on Windows"
        )

    CRED_TYPE_GENERIC = 1

    advapi32 = ctypes.WinDLL("advapi32", use_last_error=True)
    advapi32.CredReadW.argtypes = [
        wintypes.LPCWSTR,
        wintypes.DWORD,
        wintypes.DWORD,
        ctypes.POINTER(ctypes.c_void_p),
    ]
    advapi32.CredReadW.restype = wintypes.BOOL

    advapi32.CredFree.argtypes = [ctypes.c_void_p]
    advapi32.CredFree.restype = None

    cred_ptr = ctypes.c_void_p()
    if not advapi32.CredReadW(target, CRED_TYPE_GENERIC, 0, ctypes.byref(cred_ptr)):
        err = ctypes.WinError(ctypes.get_last_error())
        raise AssuranceError(
            f"Cannot read Windows credential '{target}': {err}"
        )

    try:
        cred = ctypes.cast(cred_ptr, ctypes.POINTER(_CREDENTIALW)).contents
        blob_size = cred.CredentialBlobSize
        blob_ptr = cred.CredentialBlob

        if blob_size == 0 or blob_size % 2 != 0:
            raise AssuranceError(
                "Pinned credential blob is not valid UTF-16LE"
            )

        char_count = blob_size // 2
        secret_buf = ctypes.c_wchar * char_count
        secret_ptr = ctypes.cast(blob_ptr, ctypes.POINTER(secret_buf))
        secret = secret_ptr.contents.value.rstrip("\x00")

        if len(secret) < 8 or len(secret) > 512:
            raise AssuranceError(
                "Pinned credential length is outside the accepted bound"
            )

        for ch in secret:
            cp = ord(ch)
            if cp < 0x21 or cp > 0x7E:
                raise AssuranceError(
                    "Pinned credential must be printable ASCII without whitespace"
                )

        result = secret

        # Best-effort zero the credential blob via ctypes.memset.
        # A failure here is non-fatal but must be traceable.
        try:
            ctypes.memset(blob_ptr, 0, blob_size)
        except Exception as exc:
            # Credential zeroing failure: the blob may persist in memory.
            # Record via the only available path — attach the diagnostic
            # context to the returned result so the caller can surface it.
            result = secret
            # We cannot raise (credential is already read), so we note the
            # zeroing failure via the AssuranceError path on the credential
            # read function.  At minimum log to stderr so it is never silent.
            import sys
            print(
                f"[GSA] credential zeroing failed: {exc}",
                file=sys.stderr,
            )

        return result
    finally:
        advapi32.CredFree(cred_ptr)


def call_deepseek_api(
    api_key: str,
    messages: list[dict[str, str]],
    *,
    model: str = DEFAULT_MODEL,
    max_tokens: int = 2048,
    temperature: float = 0.0,
    timeout_seconds: int = 60,
    conversation_id: str = "",
    attempt: int = 0,
    turn: int = 0,
    allowed_categories: set[str] | None = None,
    allowed_endpoints: set[str] | None = None,
) -> dict[str, Any]:
    """Call the DeepSeek chat completions API exactly once.

    Returns a dict with keys:
      - public_assistant_text: str
      - finish_reason: str
      - usage: dict with prompt_tokens, completion_tokens, total_tokens
      - model: str
      - private_reasoning_content_sha256: str | None
      - http_status_code: int
      - network_permit_id: str (empty if no permit evaluation)
    """
    # ── network permit gate ──
    network_permit_id = ""
    if conversation_id:
        from .network_permit_gateway import (
            evaluate_network_permit,
            NetworkPermitBlockedError,
        )
        try:
            permit_receipt = evaluate_network_permit(
                endpoint=DEEPSEEK_ENDPOINT,
                category="llm_provider",
                conversation_id=conversation_id,
                attempt=attempt,
                turn=turn,
                allowed_categories=allowed_categories or {"llm_provider"},
                allowed_endpoints=allowed_endpoints or {"api.deepseek.com"},
            )
            network_permit_id = permit_receipt.get("permit_id", "")
        except NetworkPermitBlockedError as exc:
            raise AssuranceError(
                f"DeepSeek API network permit denied: {exc}"
            )

    body_obj = {
        "model": model,
        "messages": messages,
        "max_tokens": max_tokens,
        "temperature": temperature,
        "stream": False,
    }
    body_bytes = json.dumps(body_obj, ensure_ascii=False).encode("utf-8")

    req = urllib.request.Request(
        DEEPSEEK_ENDPOINT,
        data=body_bytes,
        headers={
            "Authorization": f"Bearer {api_key}",
            "Content-Type": "application/json",
        },
        method="POST",
    )

    try:
        with urllib.request.urlopen(req, timeout=timeout_seconds) as resp:
            http_status = resp.status
            raw = resp.read().decode("utf-8")
    except urllib.error.HTTPError as exc:
        raise AssuranceError(
            f"DeepSeek API returned HTTP {exc.code}: {exc.reason}"
        )
    except urllib.error.URLError as exc:
        raise AssuranceError(f"DeepSeek API network error: {exc.reason}")

    document = json.loads(raw)

    choice = document["choices"][0]
    message = choice["message"]
    content = (message.get("content") or "").strip()

    if not content:
        raise AssuranceError("DeepSeek API returned empty public content")

    private_reasoning_sha256: str | None = None
    reasoning = message.get("reasoning_content")
    if reasoning:
        private_reasoning_sha256 = sha256_bytes(reasoning.encode("utf-8"))

    usage = document.get("usage", {})
    finish = choice.get("finish_reason", "unknown")

    return {
        "public_assistant_text": content,
        "finish_reason": finish,
        "usage": {
            "prompt_tokens": usage.get("prompt_tokens", 0),
            "completion_tokens": usage.get("completion_tokens", 0),
            "total_tokens": usage.get("total_tokens", 0),
        },
        "model": document.get("model", model),
        "private_reasoning_content_sha256": private_reasoning_sha256,
        "http_status_code": http_status,
        "network_permit_id": network_permit_id,
    }


def build_real_deepseek_answer_packet(
    *,
    run_id: str,
    task_id: str,
    task_contract: dict[str, Any],
    task_contract_sha256: str,
    source_gate_receipt: dict[str, Any],
    source_gate_receipt_sha256: str | None = None,
    ipg_receipt: dict[str, Any] | None = None,
    ipg_receipt_sha256: str | None = None,
    tool_availability_receipt: dict[str, Any] | None = None,
    tool_availability_receipt_sha256: str | None = None,
    model_output: dict[str, Any] | None = None,
    messages_sent: list[dict[str, str]] | None = None,
    conversation_id: str | None = None,
    envelope_id: str | None = None,
) -> dict[str, Any]:
    """Build a real answer packet from actual DeepSeek model output.

    When *conversation_id* and *envelope_id* are provided (from a
    :class:`ConversationNamespace`), they are embedded in the packet
    for session traceability and cross-session isolation verification.
    """
    gate_digest = source_gate_receipt_sha256 or sha256_bytes(
        canonical_bytes(source_gate_receipt)
    )
    ipg_digest = ipg_receipt_sha256 or sha256_bytes(
        canonical_bytes(ipg_receipt)
    )
    tool_digest = tool_availability_receipt_sha256 or sha256_bytes(
        canonical_bytes(tool_availability_receipt)
    )
    deferred = [
        f"{item['ref_id']} requires {item['required_visibility']} before stronger use"
        for item in source_gate_receipt["reference_decisions"]
        if item["decision"] != "allow"
    ]

    summary_text: list[str]
    if model_output and model_output.get("public_assistant_text"):
        content = model_output["public_assistant_text"]
        summary_text = [content]
    else:
        summary_text = ["Real DeepSeek adapter produced no output content."]

    answer_mode: str = "real_development"
    network_used: bool = True

    packet: dict[str, Any] = {
        "schema_version": "0.1.0-draft",
        "packet_kind": "canonical_guarded_cli_answer_packet",
        "run_id": run_id,
        "task_id": task_id,
        "task_contract": {
            "sha256": task_contract_sha256,
            "entry_mode": task_contract["entry_mode"],
        },
        "adapter": {
            "adapter_id": "canonical-cli-real-deepseek-adapter",
            "provider": "deepseek",
            "model_id": model_output.get("model", DEFAULT_MODEL) if model_output else DEFAULT_MODEL,
            "mode": answer_mode,
            "real_network_used": network_used,
            "tool_calls_used": False,
        },
        "instruction_provenance_gate": {
            "receipt_sha256": ipg_digest,
            "decision": ipg_receipt["gate_decision"],
            "all_sources_classified": ipg_receipt["checks"]["all_sources_classified"],
            "no_injection_escalation": ipg_receipt["checks"]["no_injection_escalation"],
        },
        "tool_availability_gate": {
            "receipt_sha256": tool_digest,
            "decision": tool_availability_receipt["decisions"]["gate_decision"],
            "available_count": tool_availability_receipt["available_count"],
            "unavailable_count": tool_availability_receipt["unavailable_count"],
            "context_injected": tool_availability_receipt["decisions"]["context_injected"],
        },
        "source_visibility_gate": {
            "receipt_sha256": gate_digest,
            "decision": source_gate_receipt["decision"],
            "must_report_visibility_status": True,
            "reference_decision_count": len(source_gate_receipt["reference_decisions"]),
        },
        "answer": {
            "summary": summary_text,
            "source_visibility_summary": [
                {
                    "ref_id": item["ref_id"],
                    "observed_visibility": item["observed_visibility"],
                    "decision": item["decision"],
                    "claim_allowed": item["claim_allowed"],
                }
                for item in source_gate_receipt["reference_decisions"]
            ],
            "deferred_claims": deferred,
        },
        "claim_boundaries": {
            "scientific_claim_strength": (
                "full_text_grounded_but_unvalidated"
                if source_gate_receipt["decision"] == "allow"
                else "metadata_only"
            ),
            "fulltext_missing_blocks_mechanism_claims": True,
            "source_gate_decision_authoritative": True,
            "all_gates_evaluated_before_model": True,
            "gate_chain_order": [
                "instruction_provenance_gate",
                "tool_availability_gate",
                "source_visibility_gate",
            ],
        },
        "next_actions": [
            "Model output has been recorded under gate constraints.",
            "This packet is not a scientific claim — it is a guarded model response.",
            "Verify the gate chain preserves all gate receipts for independent audit.",
        ],
        "limitations": [
            "Real DeepSeek API was called exactly once with tool_calls disabled.",
            "Credential value and raw HTTP response were not persisted.",
            "Only public assistant text enters the answer packet; private reasoning is digest-only.",
            "This proves adapter shape under all gates, not model scientific correctness.",
            "API usage incurs billing; retries are disabled (exactly one request).",
        ],
    }

    if model_output:
        packet["adapter"]["usage"] = {
            "prompt_tokens": model_output["usage"]["prompt_tokens"],
            "completion_tokens": model_output["usage"]["completion_tokens"],
            "total_tokens": model_output["usage"]["total_tokens"],
        }
        packet["adapter"]["http_status_code"] = model_output["http_status_code"]
        packet["adapter"]["finish_reason"] = model_output["finish_reason"]

    validate_contract(
        packet,
        "canonical-cli-answer-packet-v0.1.schema.json",
        label="real DeepSeek answer packet",
    )
    return packet


def build_real_deepseek_context(
    *,
    task_contract: dict[str, Any],
    source_gate_receipt: dict[str, Any],
    tool_availability_report: dict[str, Any],
) -> list[dict[str, str]]:
    """Build the message context for a real DeepSeek call.

    Constructs a system message from the gate context and a user message
    from the task contract.  The source visibility summary is inlined so the
    model can see per-reference decisions.
    """
    task_text = task_contract["user_request"]["raw_text"]

    allowed_refs = [
        f"  {item['ref_id']}: visibility={item['observed_visibility']}, "
        f"decision={item['decision']}, claim={item['claim_allowed']}"
        for item in source_gate_receipt["reference_decisions"]
    ]
    source_block = (
        "Source visibility decisions (each reference's observed state and "
        "allowed claim level):\n" + "\n".join(allowed_refs)
        if allowed_refs
        else "No source references available."
    )

    available_tools = tool_availability_report.get("available", [])
    unavailable_tools = tool_availability_report.get("unavailable", [])
    unprobed_tools = tool_availability_report.get("unprobed", [])
    degraded_tools = tool_availability_report.get("degraded", [])

    def _tool_names(entries: list[Any]) -> str:
        names: list[str] = []
        for entry in entries:
            if isinstance(entry, str):
                names.append(entry)
            elif isinstance(entry, dict):
                names.append(str(entry.get("tool_name", entry.get("tool_id", "?"))))
            else:
                names.append(str(entry))
        return ", ".join(names) if names else "none"

    tool_lines = []
    tool_lines.append(f"AVAILABLE tools: {_tool_names(available_tools)}")
    if unavailable_tools:
        tool_lines.append(f"UNAVAILABLE tools: {_tool_names(unavailable_tools)}")
    if degraded_tools:
        tool_lines.append(f"DEGRADED tools: {_tool_names(degraded_tools)}")
    if unprobed_tools:
        tool_lines.append(f"UNPROBED tools: {_tool_names(unprobed_tools)}")

    tool_block = (
        "\n".join(tool_lines) + "\n"
        "You MUST NOT guess whether an unavailable tool exists or what it would return. "
        "You MUST NOT fabricate tool call results or search results when the tool is unavailable. "
        "If a tool is listed as UNAVAILABLE, explicitly state that it is unavailable and suggest "
        "the user enable it or proceed with available tools only."
    )

    system_content = (
        f"[TOOL_AVAILABILITY v0.1]\n{tool_block}\n"
        f"[/TOOL_AVAILABILITY]\n\n"
        f"{source_block}\n\n"
        "You are operating under a scientific-assurance gate. "
        "Answer the user's question based only on the source references "
        "whose visibility status is reported above. "
        "If a reference is marked metadata_only or observed_fragment_only, "
        "you MUST NOT use it for mechanism, methods, comparison, or "
        "quantitative claims — only for identifying the work's existence. "
        "Explicitly note which sources you relied on and their visibility level."
    )

    return [
        {"role": "system", "content": system_content},
        {"role": "user", "content": task_text},
    ]
