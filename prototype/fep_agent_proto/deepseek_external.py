from __future__ import annotations

import os
import ssl
from typing import Any

from .credentials import WINDOWS_DEEPSEEK_CREDENTIAL_TARGET
from .deepseek_adapter import validate_deepseek_profile
from .deepseek_https import create_deepseek_tls_context
from .external_network import DEEPSEEK_CHAT_ENDPOINT
from .layout import RUNTIME_ROOT
from .schema import validate_instance


def inspect_deepseek_external_readiness(profile: dict[str, Any]) -> dict[str, Any]:
    """Offline-only readiness report; never reads a credential or opens a socket."""

    profile_report = validate_deepseek_profile(profile)
    context = create_deepseek_tls_context()
    report = {
        "schema_version": "0.1.0-prototype",
        "valid": profile_report["valid"],
        "ready_for_network": False,
        "provider": "deepseek",
        "endpoint": DEEPSEEK_CHAT_ENDPOINT,
        "transport": "deepseek-direct-https-v0.1",
        "model_requested": profile["model"],
        "model_resolved": profile_report["model_resolved"],
        "profile_valid": profile_report["valid"],
        "tls": {
            "certificate_verification": context.verify_mode == ssl.CERT_REQUIRED,
            "hostname_verification": context.check_hostname,
            "minimum_version": "TLSv1.2",
            "system_trust": True,
        },
        "credential": {
            "provider": "windows-credential-manager-current-user",
            "target": WINDOWS_DEEPSEEK_CREDENTIAL_TARGET,
            "provider_implementation_available": os.name == "nt",
            "accessed": False,
            "raw_key_cli_argument_supported": False,
        },
        "approval": {
            "issued": False,
            "one_shot": True,
            "request_digest_bound": True,
            "endpoint_bound": True,
            "maximum_ttl_seconds": 300,
        },
        "network_request_performed": False,
        "blockers": [
            "No one-shot permit has been issued for an exact request digest.",
            "The pinned Windows credential has not been accessed.",
            "No user-facing command is authorized to execute this external transport.",
        ],
        "limitations": [
            "This report checks construction-time policy only and performs no DNS, TLS, credential, quota, or model discovery probe.",
            "A capability permit narrows authority but does not prove that a person understood the request content.",
            "Successful provider I/O would not establish evidence or claim eligibility.",
        ],
    }
    validate_instance(
        report,
        RUNTIME_ROOT / "deepseek-external-readiness-v0.1.schema.json",
        label="DeepSeek external readiness report",
    )
    return report
