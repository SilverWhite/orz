from __future__ import annotations

import unittest

from assurance.contracts import validate_contract
from assurance.grok_tool_permission_observer import (
    build_grok_tool_availability_projection,
    build_grok_tool_permission_observation_bundle,
    build_grok_tool_permission_observation_receipt,
)


HELP_TEXT = """
Usage: grok [OPTIONS]
  --allow <TOOLS>
  --deny <TOOLS>
  --permission-mode <MODE>
  --always-approve
  --disable-web-search
  --tools <TOOLS>
  --disallowed-tools <TOOLS>
"""

AGENT_HELP_TEXT = """
Usage: grok agent [OPTIONS]
  --agent-profile <PATH>
  --always-approve
"""


def _inspect_report() -> dict[str, object]:
    return {
        "grokVersion": "0.2.112",
        "projectTrusted": False,
        "permissions": {
            "sources": [],
            "loaded": 0,
            "skipped": [],
            "managedSettingsActive": False,
        },
        "agents": [
            {
                "name": "general-purpose",
                "description": "General purpose agent.",
                "source": {"type": "builtin"},
            },
            {
                "name": "explore",
                "description": "Read-only exploration agent.",
                "source": {"type": "builtin"},
            },
            {
                "name": "gsa-project-doc-retrieval",
                "description": "Project docs.",
                "source": {"type": "project", "path": ".grok/agents/project.md"},
            },
            {
                "name": "gsa-external-retrieval",
                "description": "External retrieval.",
                "source": {"type": "project", "path": ".grok/agents/external.md"},
            },
        ],
        "mcpServers": [],
    }


def _acp_verification(valid: bool = True) -> dict[str, object]:
    return {
        "schema_version": "0.1.0",
        "verification_kind": "grok-acp-fake-tool-probe-verification",
        "valid": valid,
        "probe_id": "ACPTOOL-0123456789abcdef0123456789abcdef",
        "result_sha256": "a" * 64,
        "checks": {
            "result_schema_valid": True,
            "result_valid_flag": True,
            "binary_lock_matches": True,
            "artifact_digests_match": True,
            "transcript_projection_matches": True,
            "provider_capture_matches": True,
            "session_evidence_matches": True,
            "workspace_receipts_match": True,
            "scenario_semantics_match": valid,
            "safety_checks_all_true": valid,
        },
        "errors": [] if valid else ["fixture failed"],
    }


class GrokToolPermissionObserverTests(unittest.TestCase):
    def test_builds_deferred_static_observation_without_acp_probe(self) -> None:
        receipt = build_grok_tool_permission_observation_receipt(
            inspect_report=_inspect_report(),
            grok_help_text=HELP_TEXT,
            agent_help_text=AGENT_HELP_TEXT,
            observed_at="2026-07-30T00:00:00Z",
        )

        validate_contract(
            receipt,
            "grok-tool-permission-observation-receipt-v0.1.schema.json",
            label="grok tool permission observation receipt",
        )
        self.assertTrue(receipt["valid"])
        self.assertEqual(receipt["decision"], "defer")
        self.assertFalse(receipt["checks"]["acp_permission_probe_attached"])
        self.assertEqual(
            receipt["surface"]["project_agents"],
            ["gsa-external-retrieval", "gsa-project-doc-retrieval"],
        )

        report = build_grok_tool_availability_projection(receipt)
        self.assertEqual(report["degraded"][0]["tool_id"], "grok_acp_permission_probe")
        gate = build_grok_tool_permission_observation_bundle(receipt)[
            "tool_availability_gate_receipt"
        ]
        self.assertEqual(gate["decisions"]["gate_decision"], "block")

    def test_attached_valid_acp_probe_allows_projection(self) -> None:
        receipt = build_grok_tool_permission_observation_receipt(
            inspect_report=_inspect_report(),
            grok_help_text=HELP_TEXT,
            agent_help_text=AGENT_HELP_TEXT,
            acp_verifications=[_acp_verification()],
            observed_at="2026-07-30T00:00:00Z",
        )

        self.assertTrue(receipt["valid"])
        self.assertEqual(receipt["decision"], "allow")
        bundle = build_grok_tool_permission_observation_bundle(receipt)
        self.assertEqual(
            bundle["tool_availability_gate_receipt"]["decisions"]["gate_decision"],
            "allow",
        )
        self.assertEqual(bundle["tool_availability_report"]["degraded"], [])

    def test_missing_permission_flag_blocks_observation(self) -> None:
        receipt = build_grok_tool_permission_observation_receipt(
            inspect_report=_inspect_report(),
            grok_help_text=HELP_TEXT.replace("--permission-mode <MODE>", ""),
            agent_help_text=AGENT_HELP_TEXT,
            observed_at="2026-07-30T00:00:00Z",
        )

        self.assertFalse(receipt["valid"])
        self.assertEqual(receipt["decision"], "block")
        self.assertFalse(receipt["checks"]["permission_controls_observed"])

    def test_invalid_attached_acp_probe_blocks_observation(self) -> None:
        receipt = build_grok_tool_permission_observation_receipt(
            inspect_report=_inspect_report(),
            grok_help_text=HELP_TEXT,
            agent_help_text=AGENT_HELP_TEXT,
            acp_verifications=[_acp_verification(valid=False)],
            observed_at="2026-07-30T00:00:00Z",
        )

        self.assertFalse(receipt["valid"])
        self.assertEqual(receipt["decision"], "block")
        self.assertFalse(receipt["checks"]["acp_permission_probe_valid_when_attached"])


if __name__ == "__main__":
    unittest.main()
