"""Quick check: tool availability gate status."""
import json
from assurance.tool_availability_gate import probe_tool_availability, build_tool_availability_gate_receipt
from assurance.canonical_cli import _build_canonical_tool_specs

tools = _build_canonical_tool_specs()
report = probe_tool_availability(tool_specs=tools, runtime_id="RUNTIME-SMOKE-001")
receipt = build_tool_availability_gate_receipt(report)

print("Available  :", receipt["available_count"])
print("Unavailable:", receipt["unavailable_count"])
print("Unprobed   :", receipt["unprobed_count"])
print("Degraded   :", receipt["degraded_count"])
print("GATE DECISION:", receipt["decisions"]["gate_decision"])
print("Context injected:", receipt["decisions"]["context_injected"])
print()
for t in report["available"]:
    print(f"  [OK] {t['tool_name']} ({t['capability']})")
for t in report["unavailable"]:
    print(f"  [--] {t['tool_name']} ({t['capability']})")
for t in report["unprobed"]:
    print(f"  [??] {t['tool_name']} ({t['capability']})")
print()
print("=== CONTEXT BLOCK (injected into model system prompt) ===")
print(report["context_block"][:2000])
