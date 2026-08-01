from __future__ import annotations

import tempfile
from pathlib import Path
import unittest

from assurance import AssuranceError
from assurance.workspace_trust import (
    establish_workspace_trust,
    verify_workspace_trust,
    workspace_trust_for_adapter,
)
from assurance.child_capability_enforcer import (
    ChildCapabilityEscalationError,
    enforce_child_capabilities,
    verify_child_capability_enforcement,
)
from assurance.network_permit_gateway import (
    NetworkPermitBlockedError,
    build_network_permit_policy,
    evaluate_network_permit,
    verify_network_permit_receipt,
)

CONV_ID = "CONV-TRUST-TEST"


def _parent_envelope(
    allowed: list[str] | None = None,
    denied: list[str] | None = None,
) -> dict:
    if allowed is None:
        allowed = [
            "filesystem.workspace_read",
            "filesystem.workspace_write",
            "network.restricted",
        ]
    if denied is None:
        denied = [
            "network.unrestricted",
            "secret.raw_read",
        ]
    return {
        "envelope_id": "ENV-PARENT-001",
        "conversation_id": CONV_ID,
        "capability_envelope": {
            "allowed": allowed,
            "denied": denied,
        },
    }


class WorkspaceTrustTests(unittest.TestCase):
    """Unified workspace trust receipt tests."""

    def test_establishes_trust_on_directory(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "README.md").write_text("# Test", encoding="utf-8")
            (root / "src").mkdir()
            (root / "src" / "main.py").write_text("print(1)", encoding="utf-8")

            receipt = establish_workspace_trust(
                workspace_root=root,
                adapter_id="test-adapter",
                conversation_id=CONV_ID,
            )
            self.assertTrue(receipt["trust_established"])
            self.assertEqual(receipt["workspace"]["file_count"], 2)
            self.assertTrue(len(receipt["workspace"]["aggregate_sha256"]), 64)

    def test_trust_fails_on_missing_directory(self) -> None:
        receipt = establish_workspace_trust(
            workspace_root=Path("/nonexistent/path"),
            adapter_id="test-adapter",
            conversation_id=CONV_ID,
        )
        self.assertFalse(receipt["trust_established"])

    def test_verify_trust_detects_file_change(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "data.txt").write_text("original", encoding="utf-8")

            receipt = establish_workspace_trust(
                workspace_root=root,
                adapter_id="test-adapter",
                conversation_id=CONV_ID,
            )

            # Verify while unchanged
            result = verify_workspace_trust(receipt, workspace_root=root)
            self.assertTrue(result["trust_still_valid"])

            # Modify a file
            (root / "data.txt").write_text("modified", encoding="utf-8")
            result2 = verify_workspace_trust(receipt, workspace_root=root)
            self.assertFalse(result2["trust_still_valid"])

    def test_workspace_trust_for_adapter_matches(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "f.txt").write_text("x", encoding="utf-8")
            receipt = establish_workspace_trust(
                workspace_root=root,
                adapter_id="deepseek-adapter",
                conversation_id=CONV_ID,
            )
            self.assertEqual(
                workspace_trust_for_adapter(receipt, adapter_id="deepseek-adapter"),
                "observed_trusted",
            )
            self.assertEqual(
                workspace_trust_for_adapter(receipt, adapter_id="other-adapter"),
                "not_observed",
            )


class ChildCapabilityEnforcerTests(unittest.TestCase):
    """Child capability enforcement tests."""

    def test_child_subset_allowed(self) -> None:
        parent = _parent_envelope()
        receipt = enforce_child_capabilities(
            parent_envelope=parent,
            child_kind="child_process",
            child_id="proc-001",
            requested_capabilities=["filesystem.workspace_read"],
        )
        self.assertTrue(receipt["capability_enforced"])
        self.assertEqual(receipt["outcome"], "enforced")

    def test_escalation_blocked(self) -> None:
        parent = _parent_envelope()
        with self.assertRaises(ChildCapabilityEscalationError) as cm:
            enforce_child_capabilities(
                parent_envelope=parent,
                child_kind="child_agent",
                child_id="agent-001",
                requested_capabilities=["network.unrestricted"],
            )
        self.assertIn("not in parent envelope", str(cm.exception).lower())

    def test_denied_capability_blocked(self) -> None:
        parent = _parent_envelope()
        with self.assertRaises(ChildCapabilityEscalationError):
            enforce_child_capabilities(
                parent_envelope=parent,
                child_kind="child_process",
                child_id="proc-002",
                requested_capabilities=["secret.raw_read", "filesystem.workspace_read"],
            )

    def test_remote_mcp_blocked_from_local_capabilities(self) -> None:
        parent = _parent_envelope(
            allowed=["network.restricted", "credential.read", "filesystem.workspace_read"]
        )
        with self.assertRaises(ChildCapabilityEscalationError):
            enforce_child_capabilities(
                parent_envelope=parent,
                child_kind="remote_mcp",
                child_id="mcp-001",
                requested_capabilities=["credential.read", "network.restricted"],
            )

    def test_verifier_detects_invalid_receipt(self) -> None:
        parent = _parent_envelope()
        receipt = enforce_child_capabilities(
            parent_envelope=parent,
            child_kind="child_process",
            child_id="proc-003",
            requested_capabilities=["filesystem.workspace_read"],
        )
        verified = verify_child_capability_enforcement(receipt, parent_envelope=parent)
        self.assertTrue(verified["valid"], verified["errors"])

        # Verify with stricter parent
        parent2 = _parent_envelope(allowed=[])
        verified2 = verify_child_capability_enforcement(receipt, parent_envelope=parent2)
        self.assertFalse(verified2["valid"])


class NetworkPermitGatewayTests(unittest.TestCase):
    """Network permit gateway tests for all endpoint categories."""

    def test_permits_llm_provider_endpoint(self) -> None:
        receipt = evaluate_network_permit(
            endpoint="https://api.deepseek.com/chat/completions",
            category="llm_provider",
            conversation_id=CONV_ID,
            attempt=1,
            turn=1,
        )
        self.assertTrue(receipt["permit_granted"])
        self.assertEqual(receipt["decision"], "allow")

    def test_permits_web_fetch_endpoint(self) -> None:
        receipt = evaluate_network_permit(
            endpoint="https://arxiv.org/abs/2401.00001",
            category="web_fetch",
            conversation_id=CONV_ID,
            attempt=1,
            turn=1,
        )
        self.assertTrue(receipt["permit_granted"])

    def test_permits_git_remote_endpoint(self) -> None:
        receipt = evaluate_network_permit(
            endpoint="https://github.com/user/repo.git",
            category="git_remote",
            conversation_id=CONV_ID,
            attempt=1,
            turn=1,
        )
        self.assertTrue(receipt["permit_granted"])

    def test_permits_mcp_server_endpoint(self) -> None:
        receipt = evaluate_network_permit(
            endpoint="https://mcp.example.com/tools",
            category="mcp_server",
            conversation_id=CONV_ID,
            attempt=1,
            turn=1,
        )
        self.assertTrue(receipt["permit_granted"])

    def test_blocks_exceeded_attempt_budget(self) -> None:
        with self.assertRaises(NetworkPermitBlockedError):
            evaluate_network_permit(
                endpoint="https://api.deepseek.com/chat/completions",
                category="llm_provider",
                conversation_id=CONV_ID,
                attempt=4,
                turn=1,
                max_attempts_per_turn=3,
            )

    def test_blocks_first_attempt_with_previous_status(self) -> None:
        with self.assertRaises(NetworkPermitBlockedError):
            evaluate_network_permit(
                endpoint="https://api.deepseek.com/chat/completions",
                category="llm_provider",
                conversation_id=CONV_ID,
                attempt=1,
                turn=1,
                previous_http_status=503,
            )

    def test_requires_previous_status_on_retry(self) -> None:
        with self.assertRaises(NetworkPermitBlockedError):
            evaluate_network_permit(
                endpoint="https://api.deepseek.com/chat/completions",
                category="llm_provider",
                conversation_id=CONV_ID,
                attempt=2,
                turn=1,
                previous_http_status=None,
            )

    def test_retry_with_previous_status_allowed(self) -> None:
        receipt = evaluate_network_permit(
            endpoint="https://api.deepseek.com/chat/completions",
            category="llm_provider",
            conversation_id=CONV_ID,
            attempt=2,
            turn=1,
            previous_http_status=503,
        )
        self.assertTrue(receipt["permit_granted"])

    def test_category_allowlist_blocks_unauthorized(self) -> None:
        with self.assertRaises(NetworkPermitBlockedError):
            evaluate_network_permit(
                endpoint="https://mcp.evil.com/tools",
                category="mcp_server",
                conversation_id=CONV_ID,
                attempt=1,
                turn=1,
                allowed_categories={"llm_provider", "web_fetch"},
            )

    def test_endpoint_allowlist_blocks_already_canonical_disallowed_endpoint(self) -> None:
        with self.assertRaises(NetworkPermitBlockedError):
            evaluate_network_permit(
                endpoint="https://evil.example/api",
                category="llm_provider",
                conversation_id=CONV_ID,
                attempt=1,
                turn=1,
                allowed_categories={"llm_provider"},
                allowed_endpoints={"api.deepseek.com"},
            )

    def test_endpoint_denylist_blocks_even_when_category_allowed(self) -> None:
        with self.assertRaises(NetworkPermitBlockedError):
            evaluate_network_permit(
                endpoint="https://api.deepseek.com/chat/completions",
                category="llm_provider",
                conversation_id=CONV_ID,
                attempt=1,
                turn=1,
                allowed_categories={"llm_provider"},
                denied_endpoints={"api.deepseek.com"},
            )

    def test_policy_builder_modes(self) -> None:
        strict = build_network_permit_policy(mode="strict")
        self.assertIn("llm_provider", strict["allowed_categories"])
        self.assertNotIn("web_fetch", strict["allowed_categories"])

        guarded = build_network_permit_policy(mode="guarded")
        self.assertIn("web_fetch", guarded["allowed_categories"])
        self.assertIn("git_remote", guarded["allowed_categories"])

        discussion = build_network_permit_policy(mode="discussion")
        self.assertIn("web_fetch", discussion["allowed_categories"])
        self.assertNotIn("llm_provider", discussion["allowed_categories"])

    def test_verifier_detects_invalid_permit(self) -> None:
        receipt = evaluate_network_permit(
            endpoint="https://api.deepseek.com/chat/completions",
            category="llm_provider",
            conversation_id=CONV_ID,
            attempt=1,
            turn=1,
        )
        verified = verify_network_permit_receipt(receipt)
        self.assertTrue(verified["valid"], verified["errors"])

    def test_verifier_detects_policy_endpoint_allowlist_mismatch(self) -> None:
        receipt = evaluate_network_permit(
            endpoint="https://api.deepseek.com/chat/completions",
            category="llm_provider",
            conversation_id=CONV_ID,
            attempt=1,
            turn=1,
        )
        policy = build_network_permit_policy(
            mode="strict",
            allowed_endpoints={"other.example"},
        )
        verified = verify_network_permit_receipt(receipt, policy=policy)
        self.assertFalse(verified["valid"])
        self.assertIn("not in policy allowed set", "\n".join(verified["errors"]))

    def test_verifier_detects_policy_endpoint_denylist_mismatch(self) -> None:
        receipt = evaluate_network_permit(
            endpoint="https://api.deepseek.com/chat/completions",
            category="llm_provider",
            conversation_id=CONV_ID,
            attempt=1,
            turn=1,
        )
        policy = build_network_permit_policy(
            mode="strict",
            denied_endpoints={"api.deepseek.com"},
        )
        verified = verify_network_permit_receipt(receipt, policy=policy)
        self.assertFalse(verified["valid"])
        self.assertIn("policy denied set", "\n".join(verified["errors"]))

    def test_verifier_detects_policy_attempt_budget_mismatch(self) -> None:
        receipt = evaluate_network_permit(
            endpoint="https://api.deepseek.com/chat/completions",
            category="llm_provider",
            conversation_id=CONV_ID,
            attempt=4,
            turn=1,
            previous_http_status=503,
            max_attempts_per_turn=10,
        )
        policy = build_network_permit_policy(
            mode="strict",
            max_attempts_per_turn=3,
        )
        verified = verify_network_permit_receipt(receipt, policy=policy)
        self.assertFalse(verified["valid"])
        self.assertIn("exceeds policy max", "\n".join(verified["errors"]))


class EndToEndTrustAndPermitTests(unittest.TestCase):
    """End-to-end: establish trust → verify child capabilities → evaluate network permit."""

    def test_full_trust_capability_permit_chain(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "config.json").write_text('{"mode":"guarded"}', encoding="utf-8")

            # 1. Establish workspace trust
            trust = establish_workspace_trust(
                workspace_root=root,
                adapter_id="deepseek-adapter",
                conversation_id=CONV_ID,
            )
            self.assertTrue(trust["trust_established"])

            # 2. Enforce child capabilities
            parent = _parent_envelope(
                allowed=["filesystem.workspace_read", "network.restricted"]
            )
            child = enforce_child_capabilities(
                parent_envelope=parent,
                child_kind="child_agent",
                child_id="sub-agent-001",
                requested_capabilities=["filesystem.workspace_read"],
            )
            self.assertTrue(child["capability_enforced"])

            # 3. Evaluate network permit for model API
            permit = evaluate_network_permit(
                endpoint="https://api.deepseek.com/chat/completions",
                category="llm_provider",
                conversation_id=CONV_ID,
                attempt=1,
                turn=1,
            )
            self.assertTrue(permit["permit_granted"])

            # 4. Trust verification against workspace
            trust_check = verify_workspace_trust(trust, workspace_root=root)
            self.assertTrue(trust_check["trust_still_valid"])


class AdapterGateContextTrustTests(unittest.TestCase):
    """AdapterGateContext carries trust receipt and reports trust status."""

    def test_gate_context_carries_trust_receipt(self) -> None:
        from assurance.adapter_gate import AdapterGateContext
        import tempfile
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "config.json").write_text("{}", encoding="utf-8")
            trust = establish_workspace_trust(
                workspace_root=root,
                adapter_id="test-adapter",
                conversation_id=CONV_ID,
            )
        ctx = AdapterGateContext(
            ipg_receipt={"gate_decision": "allow", "valid": True},
            ipg_context={"conversation_id": CONV_ID},
            adapter_id="test-adapter",
            conversation_id=CONV_ID,
            run_id="RUN-TEST",
            trust_receipt=trust,
        )
        self.assertEqual(ctx.trust_status, "observed_trusted")
        self.assertTrue(len(ctx.trust_receipt_id) > 0)
        self.assertTrue(len(ctx.trust_receipt_sha256) > 0)

    def test_gate_context_without_trust_receipt_reports_not_observed(self) -> None:
        from assurance.adapter_gate import AdapterGateContext
        ctx = AdapterGateContext(
            ipg_receipt={"gate_decision": "allow", "valid": True},
            ipg_context={"conversation_id": CONV_ID},
            adapter_id="test-adapter",
            conversation_id=CONV_ID,
            run_id="RUN-TEST",
        )
        self.assertEqual(ctx.trust_status, "not_observed")
        self.assertEqual(ctx.trust_receipt_id, "")
        self.assertEqual(ctx.trust_receipt_sha256, "")

    def test_gate_context_carries_network_policy_fields(self) -> None:
        from assurance.adapter_gate import AdapterGateContext
        policy = build_network_permit_policy(mode="guarded")
        ctx = AdapterGateContext(
            ipg_receipt={"gate_decision": "allow", "valid": True},
            ipg_context={"conversation_id": CONV_ID},
            adapter_id="test-adapter",
            conversation_id=CONV_ID,
            run_id="RUN-TEST",
            network_policy=policy,
            network_endpoint="https://api.deepseek.com/v1",
            network_endpoint_category="llm_provider",
            allowed_categories={"llm_provider"},
            allowed_endpoints={"api.deepseek.com"},
        )
        self.assertIsNotNone(ctx.network_policy)
        self.assertEqual(ctx.network_endpoint, "https://api.deepseek.com/v1")
        self.assertEqual(ctx.network_endpoint_category, "llm_provider")


class EntryPointAuditorTests(unittest.TestCase):
    """Entry point auditor validates all known entry points establish trust."""

    def test_validate_all_entry_points_establish_trust(self) -> None:
        from assurance.workspace_trust import validate_all_entry_points_establish_trust
        result = validate_all_entry_points_establish_trust()
        self.assertTrue(result["valid"], f"auditor errors: {result['errors']}")
        consumers = result["consumers"]
        self.assertIn("assurance/canonical_cli.py", consumers)
        self.assertIn("assurance/retrieval_subagent.py", consumers)
        for path, checks in consumers.items():
            self.assertTrue(checks["imports_trust"], f"{path}: missing import")
            self.assertTrue(checks["calls_trust"], f"{path}: missing call")


class DeepSeekNetworkPermitTests(unittest.TestCase):
    """Network permit evaluation in call_deepseek_api."""

    def test_call_deepseek_api_accepts_network_params(self) -> None:
        """call_deepseek_api accepts optional network permit params without error."""
        from assurance.deepseek_adapter import call_deepseek_api
        # Verify signature is importable
        import inspect
        sig = inspect.signature(call_deepseek_api)
        params = list(sig.parameters.keys())
        for p in ("conversation_id", "attempt", "turn", "allowed_categories",
                  "allowed_endpoints"):
            self.assertIn(p, params, f"missing parameter: {p}")

    def test_network_permit_blocked_with_disallowed_category(self) -> None:
        """evaluate_network_permit blocks a category not in allowlist."""
        with self.assertRaises(NetworkPermitBlockedError):
            evaluate_network_permit(
                endpoint="https://example.com/api",
                category="web_fetch",
                conversation_id=CONV_ID,
                attempt=1,
                turn=1,
                allowed_categories={"llm_provider"},
            )


if __name__ == "__main__":
    unittest.main()
