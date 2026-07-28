"""Static view-model data for the GAK-UI-001 first prototype.

All data is hard-coded — the prototype is disconnected from the
assurance core and renders a single fixed scene.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any

# ── Explorer pane: sample source tree ───────────────────────────────────────


@dataclass
class TreeNode:
    label: str
    children: list[TreeNode] = field(default_factory=list)
    expanded: bool = True
    uri: str = ""


SAMPLE_SOURCE_TREE: list[TreeNode] = [
    TreeNode(
        label="INDEX",
        expanded=True,
        uri="workspace://INDEX",
        children=[
            TreeNode(label="MAP_MEM.md", uri="workspace://INDEX/MAP_MEM.md"),
            TreeNode(label="LIF_CURRENT_INDEX.md", uri="workspace://INDEX/LIF_CURRENT_INDEX.md"),
        ],
    ),
    TreeNode(
        label="MAP",
        expanded=True,
        uri="workspace://MAP",
        children=[
            TreeNode(label="Self-Check", uri="workspace://MAP/self-check"),
            TreeNode(label="Prior-Existence", uri="workspace://MAP/prior-existence"),
        ],
    ),
    TreeNode(
        label="R Series",
        expanded=True,
        uri="workspace://R-Series",
        children=[
            TreeNode(label="R211", uri="source://R211"),
            TreeNode(label="R212", uri="source://R212"),
            TreeNode(label="R213", uri="source://R213"),
        ],
    ),
    TreeNode(
        label="Artifacts",
        expanded=False,
        uri="workspace://Artifacts",
        children=[
            TreeNode(label="run_2026_06_26.json", uri="artifact://results/run_2026_06_26.json"),
            TreeNode(label="run_2026_07_27.json", uri="artifact://results/run_2026_07_27.json"),
        ],
    ),
    TreeNode(
        label="Adapters",
        expanded=False,
        uri="workspace://Adapters",
        children=[
            TreeNode(label="deepseek", uri="adapter://deepseek/observation"),
        ],
    ),
    TreeNode(
        label="Verifier",
        expanded=False,
        uri="workspace://Verifier",
        children=[
            TreeNode(label="Receipts", uri="verifier://receipts"),
        ],
    ),
]

# ── Content pane: task summary ──────────────────────────────────────────────


@dataclass
class SourceVisibilityRow:
    ref_id: str
    observed: str
    required: str
    decision: str
    claim: str


SAMPLE_SOURCE_TABLE: list[SourceVisibilityRow] = [
    SourceVisibilityRow("MAP_MEM.md", "FULL TEXT", "FULL TEXT", "ALLOW", "full_text"),
    SourceVisibilityRow("R211.md", "PARTIAL", "FULL TEXT", "DEFER", "fragment"),
    SourceVisibilityRow("run_2026_06_26", "FULL TEXT", "FULL TEXT", "ALLOW", "full_text"),
]

SAMPLE_CLAIM_DISPOSITION = "DEFER"
SAMPLE_DISPOSITION_REASON = "Missing: orthogonal decoder final result"

SAMPLE_NEXT_ACTIONS = [
    "Retrieve full text of R211.md for mechanism claim.",
    "Re-run source visibility gate after retrieval.",
    "Verify adapter gate chain integrity.",
]

# ── Address bar ─────────────────────────────────────────────────────────────

SAMPLE_ADDRESS_URI = "workspace://LIF/current-index"
SAMPLE_COMMAND_URI = "command://run/current-task"

# ── Find bar ────────────────────────────────────────────────────────────────

SAMPLE_FIND_QUERY = "source visibility"
SAMPLE_FIND_SCOPE = "Conversation"
SAMPLE_FIND_SCOPES = [
    "Conversation",
    "Project Docs",
    "Current File",
    "Runs / Artifacts",
    "Indexed Sources",
]

# ── Content marker (right-side column) ──────────────────────────────────────


@dataclass
class MarkerEntry:
    kind: str       # "input" or "hit"
    line: int       # line number in the conversation
    label: str = "" # optional short label


SAMPLE_CONTENT_MARKER: list[MarkerEntry] = [
    MarkerEntry(kind="input", line=4, label="ask: check source visibility"),
    MarkerEntry(kind="hit", line=5, label="MAP_MEM.md"),
    MarkerEntry(kind="hit", line=8, label="R211.md"),
    MarkerEntry(kind="input", line=12, label="ask: mechanism claim?"),
    MarkerEntry(kind="hit", line=15, label="caspase-3"),
    MarkerEntry(kind="input", line=27, label="ask: next action?"),
]

# ── Explorer pane: event groups ─────────────────────────────────────────────


@dataclass
class EventGroup:
    label: str
    count: int
    expanded: bool = False
    entries: list[str] = field(default_factory=list)


SAMPLE_EVENT_GROUPS: list[EventGroup] = [
    EventGroup(label="User Inputs", count=3, expanded=True, entries=[
        "check source visibility",
        "mechanism claim?",
        "next action?",
    ]),
    EventGroup(label="Decisions", count=2, expanded=True, entries=[
        "gate: allow (source fulltext)",
        "gate: defer (insufficient visibility)",
    ]),
    EventGroup(label="Errors", count=1, expanded=False, entries=[
        "network permit denied: web_fetch(example.com)",
    ]),
    EventGroup(label="Permissions", count=1, expanded=False, entries=[
        "granted: file_read(workspace/**)",
    ]),
    EventGroup(label="Verifier", count=1, expanded=False, entries=[
        "receipt: guard chain complete",
    ]),
    EventGroup(label="Artifacts", count=2, expanded=False, entries=[
        "answer packet: RUN-2026-07-28-004",
        "journal event: gate_decision",
    ]),
    EventGroup(label="Sources", count=3, expanded=True, entries=[
        "MAP_MEM.md (VERIFIED)",
        "R211.md (PARTIAL)",
        "run_2026_06_26.json (VERIFIED)",
    ]),
]

# ── Status bar ──────────────────────────────────────────────────────────────

SAMPLE_STATUS_ITEMS = [
    ("GUARDED", True),
    ("NET OFF", True),
    ("SBX STRICT", True),
    ("SOURCES 3/4", False),
    ("DEEPSEEK", True),
    ("IDLE", True),
]

# ── Dialog mock ─────────────────────────────────────────────────────────────

SAMPLE_DIALOG: dict[str, Any] = {
    "title": "Permission Required",
    "message": (
        "The agent wants to read a source outside the current workspace.\n"
        "\n"
        "Source: https://example.com/external-paper.pdf\n"
        "Action: web_fetch (requires network permit)\n"
        "Impact: external page content enters the evidence chain\n"
        "\n"
        "Allow this action?"
    ),
    "actions": [
        ("Approve", True),
        ("Reject", False),
        ("View Details", None),
    ],
}

# ── Properties mock ─────────────────────────────────────────────────────────

SAMPLE_PROPERTIES: dict[str, Any] = {
    "title": "Properties: R211.md",
    "tabs": [
        {
            "name": "General",
            "fields": [
                ("Name", "R211.md"),
                ("Type", "Source Reference"),
                ("URI", "source://R211/mem-outlier"),
                ("Size", "12.4 KB"),
                ("Modified", "2026-06-26 14:22 UTC"),
            ],
        },
        {
            "name": "Provenance",
            "fields": [
                ("Origin", "LIF Project Workspace"),
                ("Author", "Project Contributor"),
                ("SHA-256", "a1b2c3...d4e5f6"),
                ("Signed", "Yes (Envelope K-001)"),
            ],
        },
        {
            "name": "Visibility",
            "fields": [
                ("Status", "PARTIAL"),
                ("Full Text", "Not Available"),
                ("Metadata", "Available"),
                ("Fragments", "2 of 3 sections"),
                ("Gate Decision", "DEFER"),
            ],
        },
        {
            "name": "Dependencies",
            "fields": [
                ("References", "MAP_MEM.md"),
                ("Derived Claims", "mem-channel-z-information"),
                ("Required For", "mechanism claim"),
            ],
        },
        {
            "name": "Verification",
            "fields": [
                ("Verifier", "Not Run"),
                ("Last Check", "N/A"),
                ("Evidence Status", "derived_unverified"),
            ],
        },
        {
            "name": "History",
            "fields": [
                ("First Seen", "2026-06-26"),
                ("Last Modified", "2026-06-26"),
                ("Gate Evaluations", "3 (2 allow, 1 defer)"),
                ("Claims Promoted", "1 (bibliographic)"),
            ],
        },
    ],
}

# ── Menu definitions ────────────────────────────────────────────────────────

SAMPLE_MENUS = {
    "File": ["Open Workspace", "Save", "Save As...", "Exit"],
    "Edit": ["Cut", "Copy", "Paste", "Select All"],
    "View": ["Refresh", "Show Log", "Show History", "Properties"],
    "Sources": ["Add Source", "Remove Source", "Source Visibility", "Source Provenance"],
    "Agent": ["Start", "Stop", "Pause", "Resume", "Set Budget"],
    "Run": ["Run", "Verify", "Dry Run", "Replay"],
    "Verify": ["Verify All", "Verify Source", "Verify Claim", "Verify Adapter"],
    "Help": ["About GSA", "Keyboard Shortcuts", "Documentation"],
}
