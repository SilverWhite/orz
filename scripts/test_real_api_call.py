"""Opt-in manual DeepSeek API smoke for the canonical CLI.

Importing this historical ``test_`` script is inert. Run it with ``--live``
to explicitly allow the real request.
"""

from __future__ import annotations

import argparse
import tempfile
from pathlib import Path
from typing import Any


def run_live_smoke() -> int:
    import assurance
    from assurance.canonical_cli import (
        _build_canonical_tool_specs,
        run_canonical_guarded_cli_real,
    )
    from assurance.tool_availability_gate import (
        build_tool_availability_gate_receipt,
        probe_tool_availability,
    )

    print("=" * 60)
    print("TOOL AVAILABILITY GATE — Pre-Model Check")
    print("=" * 60)
    tools = _build_canonical_tool_specs()
    report = probe_tool_availability(tool_specs=tools, runtime_id="RUNTIME-LIVE-001")
    gate = build_tool_availability_gate_receipt(report)
    print(f"Gate decision: {gate['decisions']['gate_decision']}")
    print(f"Context injected: {gate['decisions']['context_injected']}")
    print(f"Model must not guess: {gate['decisions']['model_must_not_guess']}")
    print(
        f"Tools probed: {len(tools)}, available: {gate['available_count']}, "
        f"unavailable: {gate['unavailable_count']}, unprobed: {gate['unprobed_count']}"
    )

    print("\n" + "=" * 60)
    print("REAL DEEPSEEK API CALL — Live Event Stream")
    print("=" * 60)
    root = Path(tempfile.mkdtemp(prefix="gsa-live-"))
    root.rmdir()  # The guarded CLI requires an absent run root.
    ledger = (
        Path(assurance.__file__).parent
        / "fixtures"
        / "source_visibility"
        / "mixed-visibility-ledger.json"
    )
    events: list[str] = []

    def on_event(event: dict[str, Any]) -> None:
        events.append(event["event_type"])
        payload = event.get("payload", {})
        timestamp = event.get("timestamp", "")[:19]
        event_type = event["event_type"]
        if event_type == "run_preflight":
            detail = (
                f"PREFLIGHT  model={payload.get('model_id', '?')}  "
                f"network={payload.get('real_network_allowed', False)}"
            )
        elif event_type == "instruction_provenance_gate":
            detail = f"IPG         receipt={payload.get('receipt_sha256', '')[:12]}…"
        elif event_type == "tool_availability_check":
            detail = (
                f"TOOL AVAIL  avail={payload.get('available_count', 0)}  "
                f"unavail={payload.get('unavailable_count', 0)}  "
                f"injected={payload.get('context_block_injected', False)}"
            )
        elif event_type == "run_started":
            detail = f"RUN STARTED task={payload.get('task_id', '?')}"
        elif event_type == "gate_decision":
            detail = (
                f"SRC VIS     decision={payload.get('decision', '?')}  "
                f"refs={payload.get('reference_count', 0)}"
            )
        elif event_type == "model_request":
            detail = (
                f"MODEL REQ   provider={payload.get('provider', '?')}  "
                f"model={payload.get('model_id', '?')}  "
                f"network={payload.get('real_network_used', False)}"
            )
        elif event_type == "model_output":
            api_error = payload.get("api_error", "")
            detail = (
                f"MODEL OUT   sha256={payload.get('answer_packet_sha256', '')[:12]}…  "
                f"valid={payload.get('structured_output_valid', False)}"
                f"{'  API_ERROR=' + api_error if api_error else ''}"
            )
        elif event_type == "run_finished":
            detail = f"RUN DONE    status={payload.get('status', '?')}"
        elif event_type == "run_failed":
            detail = f"RUN FAILED  status={payload.get('status', '?')}"
        else:
            return
        print(f"[{timestamp}] {detail}")

    receipt = run_canonical_guarded_cli_real(
        run_root=root,
        ask=(
            "What is spiking neural network (SNN)? Answer in 2-3 sentences. "
            "Do NOT use any tools — just answer from your knowledge."
        ),
        source_ledger_path=ledger,
        on_event=on_event,
        api_timeout_seconds=30,
    )
    print("\n" + "=" * 60)
    print(f"RESULT: valid={receipt.get('valid')}")
    print(f"Journal events: {len(events)}")
    print(f"Journal file: {root / 'events.jsonl'}")
    print("=" * 60)
    return 0 if receipt.get("valid") else 1


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--live",
        action="store_true",
        help="confirm that one real DeepSeek API request may be sent",
    )
    args = parser.parse_args()
    if not args.live:
        print(
            "Refusing to send a real API request without --live. "
            "This smoke may use credentials, network access, and paid quota."
        )
        return 2
    try:
        return run_live_smoke()
    except Exception as exc:
        print(f"\nERROR: {exc}")
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
