from __future__ import annotations

import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

from assurance.grok_profile_drafts import (
    ROOT,
    load_grok_retrieval_profile_draft,
    verify_grok_retrieval_profile_drafts,
)


class GrokProfileDraftTests(unittest.TestCase):
    def test_repository_has_exactly_two_retrieval_profile_drafts(self) -> None:
        receipt = verify_grok_retrieval_profile_drafts()

        self.assertTrue(receipt["valid"], receipt["errors"])
        self.assertEqual(receipt["agent_profile_count"], 2)
        self.assertEqual(receipt["workflow_draft_count"], 0)
        self.assertEqual(
            {profile["name"] for profile in receipt["profiles"]},
            {"gsa-project-doc-retrieval", "gsa-external-retrieval"},
        )

    def test_project_doc_profile_excludes_external_retrieval_tools(self) -> None:
        draft = load_grok_retrieval_profile_draft(
            ROOT / ".grok" / "agents" / "gsa-project-doc-retrieval.md"
        )

        self.assertEqual(draft.name, "gsa-project-doc-retrieval")
        self.assertEqual(draft.mcp_inheritance, "none")
        self.assertNotIn("web_search", draft.tools)
        self.assertNotIn("web_fetch", draft.tools)

    def test_external_profile_declares_web_tools_without_scheduler_tools(self) -> None:
        draft = load_grok_retrieval_profile_draft(
            ROOT / ".grok" / "agents" / "gsa-external-retrieval.md"
        )

        self.assertEqual(draft.name, "gsa-external-retrieval")
        self.assertIn("web_search", draft.tools)
        self.assertIn("web_fetch", draft.tools)
        self.assertNotIn("workflow", {tool.lower() for tool in draft.tools})
        self.assertNotIn("spawn_subagent", {tool.lower() for tool in draft.tools})

    def test_workflow_drafts_are_rejected_until_syntax_is_verified(self) -> None:
        with tempfile.TemporaryDirectory(dir=ROOT) as temporary:
            root = Path(temporary)
            agents = root / ".grok" / "agents"
            workflows = root / ".grok" / "workflows"
            agents.mkdir(parents=True)
            workflows.mkdir(parents=True)
            for source in (ROOT / ".grok" / "agents").glob("*.md"):
                (agents / source.name).write_text(source.read_text(encoding="utf-8"), encoding="utf-8")
            (workflows / "gsa-project-doc-retrieval.rhai").write_text(
                "let meta = #{ name: \"gsa-project-doc-retrieval\" };\n",
                encoding="utf-8",
            )

            receipt = verify_grok_retrieval_profile_drafts(root)

        self.assertFalse(receipt["valid"])
        self.assertIn("premature workflows", "\n".join(receipt["errors"]))

    def test_locked_grok_inspect_discovers_project_agent_profiles(self) -> None:
        binary = ROOT / ".tools" / "grok" / "0.2.112" / "grok.exe"
        if not binary.is_file():
            self.skipTest("locked Grok binary is not installed")
        with tempfile.TemporaryDirectory(dir=ROOT) as temporary:
            env = os.environ.copy()
            env["GROK_HOME"] = str(Path(temporary) / "grok-home")
            env["GROK_MEMORY"] = "0"
            env["GROK_WEB_FETCH"] = "0"
            completed = subprocess.run(
                [str(binary), "inspect", "--json"],
                cwd=ROOT,
                stdin=subprocess.DEVNULL,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                encoding="utf-8",
                errors="replace",
                env=env,
                timeout=20,
                check=False,
            )

        self.assertEqual(completed.returncode, 0, completed.stderr)
        report = json.loads(completed.stdout)
        project_agents = {
            item["name"]: item
            for item in report.get("agents", [])
            if item.get("source", {}).get("type") == "project"
        }
        self.assertEqual(
            set(project_agents),
            {"gsa-project-doc-retrieval", "gsa-external-retrieval"},
        )
        for name, agent in project_agents.items():
            self.assertIn(f".grok/agents\\{name}.md", agent["source"]["path"])


if __name__ == "__main__":
    unittest.main()
