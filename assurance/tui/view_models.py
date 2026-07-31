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

# ── Built-in slash commands (seven) ──────────────────────────────────────────

# Import the shared registry so the TUI palette and tests share the same
# command definitions.
try:
    from .commands import get_builtin_registry

    _registry = get_builtin_registry()
    SAMPLE_COMMANDS = _registry.all()
except ImportError:
    SAMPLE_COMMANDS = []

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
    ("守护", True),
    ("网络关闭", True),
    ("沙箱严格", True),
    ("来源 3/4", False),
    ("DeepSeek", True),
    ("空闲", True),
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

# ── Keyboard shortcuts reference ─────────────────────────────────────────────

# Displayed when the user invokes /help.
# Format: (shortcut, description_zh)
SAMPLE_SHORTCUTS: list[tuple[str, str]] = [
    ("/", "指令面板 — 显示常用指令"),
    ("Alt+字母", "菜单激活 — 例如 Alt+W 文件"),
    ("F6", "焦点循环 — 在主要区域之间切换"),
    ("Tab", "焦点内移动 / 指令面板中切换选项"),
    ("Shift+Tab", "焦点内反向移动"),
    ("Ctrl+Z", "撤回已发送输入 — 取消当前运行并回填指令"),
    ("Ctrl+L", "打开命令/位置输入弹窗"),
    ("Esc", "关闭 — 清空输入 / 关闭菜单 / 关闭弹窗"),
    ("Enter", "激活选中项 / 发送指令"),
    ("↑↓←→", "方向键 — 在面板内移动选择"),
    ("Backspace", "输入框删除上一个字符"),
]

# ── Menu definitions ────────────────────────────────────────────────────────

SAMPLE_MENUS = {
    "文件": ["打开工作区", "保存", "另存为...", "退出"],
    "事件": ["显示事件", "隐藏事件", "事件过滤..."],
    "标记": ["显示标记", "隐藏标记", "标记过滤..."],
    "编辑": ["插入/覆盖", "半角/全角", "自动换行", "多行输入"],
    "模型": ["切换模型", "推理强度", "Plan/Manual/Auto", "适配器状态"],
    "来源": ["添加来源", "移除来源", "来源可见性", "来源溯源"],
    "运行": ["运行", "验证", "试运行", "重放"],
    "验证": ["验证全部", "验证来源", "验证声明", "验证适配器"],
    "帮助": ["快捷键", "命令列表", "模型说明", "审批说明", "终端说明", "来源说明"],
}
