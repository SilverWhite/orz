"""Real API call test — canonical CLI with DeepSeek adapter + on_event streaming."""
import json, sys, tempfile, time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from assurance.canonical_cli import run_canonical_guarded_cli_real
from assurance.tool_availability_gate import probe_tool_availability, build_tool_availability_gate_receipt
from assurance.canonical_cli import _build_canonical_tool_specs

# First, show tool availability status
print("=" * 60)
print("TOOL AVAILABILITY GATE — Pre-Model Check")
print("=" * 60)
tools = _build_canonical_tool_specs()
report = probe_tool_availability(tool_specs=tools, runtime_id="RUNTIME-LIVE-001")
receipt = build_tool_availability_gate_receipt(report)
print(f"Gate decision: {receipt['decisions']['gate_decision']}")
print(f"Context injected into model prompt: {receipt['decisions']['context_injected']}")
print(f"Model must not guess: {receipt['decisions']['model_must_not_guess']}")
print(f"Tools probed: {len(tools)}, available: {receipt['available_count']}, "
      f"unavailable: {receipt['unavailable_count']}, unprobed: {receipt['unprobed_count']}")
print()

# Run real CLI with API
print("=" * 60)
print("REAL DEEPSEEK API CALL — Live Event Stream")
print("=" * 60)

root = Path(tempfile.mkdtemp(prefix="gsa-live-"))
root.rmdir()  # must not exist

# Use project's source visibility fixture
import assurance
ledger = Path(assurance.__file__).parent / "fixtures" / "source_visibility" / "mixed-visibility-ledger.json"

events = []

def on_event(evt: dict) -> None:
    events.append(evt["event_type"])
    payload = evt.get("payload", {})
    ts = evt.get("timestamp", "")[:19]
    etype = evt["event_type"]
    # Print key info per event
    if etype == "run_preflight":
        print(f"[{ts}] PREFLIGHT  model={payload.get('model_id','?')}  "
              f"network={payload.get('real_network_allowed',False)}")
    elif etype == "instruction_provenance_gate":
        print(f"[{ts}] IPG         receipt={payload.get('receipt_sha256','')[:12]}…")
    elif etype == "tool_availability_check":
        print(f"[{ts}] TOOL AVAIL  avail={payload.get('available_count',0)}  "
              f"unavail={payload.get('unavailable_count',0)}  injected={payload.get('context_block_injected',False)}")
    elif etype == "run_started":
        print(f"[{ts}] RUN STARTED task={payload.get('task_id','?')}")
    elif etype == "gate_decision":
        print(f"[{ts}] SRC VIS     decision={payload.get('decision','?')}  "
              f"refs={payload.get('reference_count',0)}")
    elif etype == "model_request":
        print(f"[{ts}] MODEL REQ   provider={payload.get('provider','?')}  "
              f"model={payload.get('model_id','?')}  network={payload.get('real_network_used',False)}")
    elif etype == "model_output":
        api_err = payload.get("api_error", "")
        print(f"[{ts}] MODEL OUT   sha256={payload.get('answer_packet_sha256','')[:12]}…  "
              f"valid={payload.get('structured_output_valid',False)}"
              f"{'  API_ERROR=' + api_err if api_err else ''}")
    elif etype == "run_finished":
        print(f"[{ts}] RUN DONE    status={payload.get('status','?')}")
    elif etype == "run_failed":
        print(f"[{ts}] RUN FAILED  status={payload.get('status','?')}")

print()
try:
    receipt = run_canonical_guarded_cli_real(
        run_root=root,
        ask="What is spiking neural network (SNN)? Answer in 2-3 sentences. Do NOT use any tools — just answer from your knowledge.",
        source_ledger_path=ledger,
        on_event=on_event,
        api_timeout_seconds=30,
    )
    print(f"\n{'='*60}")
    print(f"RESULT: valid={receipt.get('valid')}")
    print(f"Journal events: {len(events)}")
    print(f"Journal file: {root / 'events.jsonl'}")
    print(f"{'='*60}")
except Exception as exc:
    print(f"\nERROR: {exc}")
    import traceback
    traceback.print_exc()
