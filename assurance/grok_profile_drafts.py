from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
AGENTS_DIR = ROOT / ".grok" / "agents"
WORKFLOWS_DIR = ROOT / ".grok" / "workflows"


@dataclass(frozen=True)
class GrokRetrievalProfileDraft:
    name: str
    path: Path
    description: str
    tools: tuple[str, ...]
    mcp_inheritance: str
    body: str


EXPECTED_PROFILES: dict[str, dict[str, Any]] = {
    "gsa-project-doc-retrieval.md": {
        "name": "gsa-project-doc-retrieval",
        "required_tools": {"Read", "Grep", "Glob"},
        "forbidden_tools": {
            "web_search",
            "web_fetch",
            "edit",
            "write",
            "bash",
            "run_terminal_command",
            "spawn_subagent",
            "workflow",
        },
        "required_body_markers": {
            "CLI_PROJECT_INDEX.md",
            "Subagent Retrieval Completion Check",
            "source visibility",
        },
    },
    "gsa-external-retrieval.md": {
        "name": "gsa-external-retrieval",
        "required_tools": {"Read", "web_search", "web_fetch"},
        "forbidden_tools": {
            "edit",
            "write",
            "bash",
            "run_terminal_command",
            "spawn_subagent",
            "workflow",
        },
        "required_body_markers": {
            "local_browser",
            "framework_fallback",
            "PDF evidence",
            "Subagent Retrieval Completion Check",
        },
    },
}


def _strip_quotes(value: str) -> str:
    value = value.strip()
    if len(value) >= 2 and value[0] == value[-1] and value[0] in {'"', "'"}:
        return value[1:-1]
    return value


def _parse_frontmatter(path: Path) -> tuple[dict[str, str], str]:
    text = path.read_text(encoding="utf-8")
    lines = text.splitlines()
    if not lines or lines[0].strip() != "---":
        raise ValueError(f"{path} is missing YAML frontmatter")
    try:
        end = next(index for index, line in enumerate(lines[1:], 1) if line.strip() == "---")
    except StopIteration as exc:
        raise ValueError(f"{path} has unterminated YAML frontmatter") from exc
    frontmatter: dict[str, str] = {}
    for line_number, raw_line in enumerate(lines[1:end], 2):
        if not raw_line.strip():
            continue
        if raw_line.startswith(" ") or raw_line.startswith("\t") or ":" not in raw_line:
            raise ValueError(f"{path}:{line_number} uses unsupported frontmatter syntax")
        key, raw_value = raw_line.split(":", 1)
        key = key.strip()
        value = _strip_quotes(raw_value)
        if not key or key in frontmatter:
            raise ValueError(f"{path}:{line_number} has duplicate or empty frontmatter key")
        frontmatter[key] = value
    body = "\n".join(lines[end + 1 :]).strip()
    return frontmatter, body


def _parse_tools(value: str) -> tuple[str, ...]:
    return tuple(item.strip() for item in value.split(",") if item.strip())


def load_grok_retrieval_profile_draft(path: Path) -> GrokRetrievalProfileDraft:
    frontmatter, body = _parse_frontmatter(path)
    return GrokRetrievalProfileDraft(
        name=frontmatter.get("name", ""),
        path=path,
        description=frontmatter.get("description", ""),
        tools=_parse_tools(frontmatter.get("tools", "")),
        mcp_inheritance=frontmatter.get("mcpInheritance", ""),
        body=body,
    )


def _validate_rhai_workflow_structure(path: Path) -> None:
    """Check that a Rhai workflow file has the required structure.

    Validates the file starts with ``let meta = #{`` and contains a
    ``name`` field.  This is a structural check, not a full Rhai
    compiler pass — full smoke-checking requires a live Grok session.
    """
    text = path.read_text(encoding="utf-8")
    stripped = text.lstrip()
    if not stripped.startswith("let meta = #{"):
        raise ValueError(
            f"{path.name}: workflow must start with 'let meta = #{'{'}...'}}'"
        )
    if "name:" not in text or "description:" not in text:
        raise ValueError(
            f"{path.name}: workflow meta must include name and description"
        )
    if "agent(" not in text and "parallel(" not in text:
        raise ValueError(
            f"{path.name}: workflow must contain at least one agent() or parallel() call"
        )


def _rhai_meta_name(path: Path) -> str:
    """Extract the ``name`` field from a Rhai workflow meta map."""
    text = path.read_text(encoding="utf-8")
    for line in text.splitlines():
        stripped_line = line.strip()
        if stripped_line.startswith("name:"):
            raw = stripped_line[len("name:"):].strip()
            return _strip_quotes(raw.rstrip(","))
    return path.stem


def verify_grok_retrieval_profile_drafts(root: Path = ROOT) -> dict[str, Any]:
    agents_dir = root / ".grok" / "agents"
    workflows_dir = root / ".grok" / "workflows"
    errors: list[str] = []
    drafts: list[GrokRetrievalProfileDraft] = []

    if not agents_dir.is_dir():
        errors.append("missing .grok/agents directory")
        agent_files: list[Path] = []
    else:
        agent_files = sorted(agents_dir.glob("*.md"))

    observed_names = {path.name for path in agent_files}
    expected_names = set(EXPECTED_PROFILES)
    if observed_names != expected_names:
        errors.append(
            "retrieval profile drafts must be exactly "
            f"{sorted(expected_names)}; observed {sorted(observed_names)}"
        )

    for path in agent_files:
        try:
            draft = load_grok_retrieval_profile_draft(path)
        except ValueError as exc:
            errors.append(str(exc))
            continue
        drafts.append(draft)
        expected = EXPECTED_PROFILES.get(path.name)
        if expected is None:
            continue
        if draft.name != expected["name"]:
            errors.append(f"{path.name} frontmatter name mismatch: {draft.name!r}")
        if not draft.description:
            errors.append(f"{path.name} description must be non-empty")
        if draft.mcp_inheritance != "none":
            errors.append(f"{path.name} must set mcpInheritance: none")
        tools = set(draft.tools)
        missing_tools = sorted(expected["required_tools"] - tools)
        if missing_tools:
            errors.append(f"{path.name} missing required tools: {missing_tools}")
        lower_tools = {tool.lower() for tool in tools}
        forbidden = sorted(
            tool for tool in expected["forbidden_tools"] if tool.lower() in lower_tools
        )
        if forbidden:
            errors.append(f"{path.name} declares forbidden tools: {forbidden}")
        for marker in expected["required_body_markers"]:
            if marker not in draft.body:
                errors.append(f"{path.name} body missing marker: {marker}")

    workflow_files = sorted(workflows_dir.glob("*.rhai")) if workflows_dir.is_dir() else []
    for wf_path in workflow_files:
        try:
            _validate_rhai_workflow_structure(wf_path)
        except ValueError as exc:
            errors.append(str(exc))

    return {
        "schema_version": "0.1.0",
        "receipt_kind": "grok_retrieval_profile_draft_verification",
        "valid": not errors,
        "agent_profile_count": len(drafts),
        "workflow_draft_count": len(workflow_files),
        "profiles": [
            {
                "name": draft.name,
                "path": str(draft.path.relative_to(root)),
                "tools": list(draft.tools),
                "mcpInheritance": draft.mcp_inheritance,
            }
            for draft in drafts
        ],
        "workflows": [
            {
                "name": _rhai_meta_name(wf_path),
                "path": str(wf_path.relative_to(root)),
            }
            for wf_path in workflow_files
        ],
        "errors": errors,
        "limitations": [
            "This verifies documented Grok agent-profile draft shape and basic Rhai workflow structure.",
            "No prompt/tool execution is promoted by these profiles or workflows.",
            "Workflow Rhai syntax verified against Grok 0.2.112 bundled create-workflow skill reference.",
            "Full smoke-check (validate_only) requires a live Grok prompt/tool session.",
        ],
    }
