"""Quick credential isolation audit."""
from assurance.retrieval_subagent import (
    DEFAULT_EXTERNAL_RETRIEVAL_CREDENTIAL_TARGET,
    dispatch_retrieval_subagent,
)
from assurance.deepseek_adapter import DEFAULT_CREDENTIAL_TARGET
import inspect, re

print("=== CREDENTIAL ISOLATION ===")
print(f"Main CLI:             {DEFAULT_CREDENTIAL_TARGET}")
print(f"External Retrieval:   {DEFAULT_EXTERNAL_RETRIEVAL_CREDENTIAL_TARGET}")

# Find the project doc subagent credential target from source
src = inspect.getsource(dispatch_retrieval_subagent)
for i, line in enumerate(src.split("\n")):
    if "credential_target" in line:
        print(f"Project Doc Retrieval: (line {i}) {line.strip()}")

# Verify all three are different
targets = {DEFAULT_CREDENTIAL_TARGET, DEFAULT_EXTERNAL_RETRIEVAL_CREDENTIAL_TARGET}
print(f"\nDistinct credential targets: {len(targets)} known from constants")

# Check credential scrub sites
from assurance.credential_scrub import audit_credential_scrub_sites
scrub_result = audit_credential_scrub_sites()
print(f"\nCredential scrub audit: {scrub_result}")
