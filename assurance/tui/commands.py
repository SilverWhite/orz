"""Slash-command registry with Chinese annotations — GAK-UI-001.

Defines every slash-command available in the TUI, their Chinese-language
metadata, and a :class:`CommandRegistry` that supports prefix search and
default-command ranking.

Design rule:
    Annotations are Chinese-only.  No English descriptions appear in the
    command palette.  Commands are identified by their ``/name`` prefix;
    the palette filters as the user types.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any


# ── command definition ───────────────────────────────────────────────────────


@dataclass
class CommandDef:
    """A single slash-command known to the TUI.

    Attributes
    ----------
    slash:
        The slash-prefixed command name, e.g. ``"/new"``.
    name_zh:
        Short Chinese name shown in the palette, e.g. ``"新对话"``.
    description_zh:
        One-line Chinese description of what the command does.
    category:
        Logical group for future grouping in the palette:
        ``"会话"``, ``"上下文"``, ``"运行时"``, ``"帮助"``.
    uri:
        The ``command://`` URI this slash maps to.  Used by the
        address bar to navigate to the appropriate object or action.
    """

    slash: str
    name_zh: str
    description_zh: str
    category: str
    uri: str


# ── built-in command set ─────────────────────────────────────────────────────

_BUILTIN_COMMANDS: list[CommandDef] = [
    CommandDef(
        slash="/new",
        name_zh="新对话",
        description_zh="创建新的会话命名空间，重置上下文窗口",
        category="会话",
        uri="command://conversation/new",
    ),
    CommandDef(
        slash="/history",
        name_zh="历史对话",
        description_zh="查看已归档的会话列表及摘要",
        category="会话",
        uri="command://conversation/history",
    ),
    CommandDef(
        slash="/context",
        name_zh="上下文可视",
        description_zh="显示当前上下文窗口占比、token 数及分布",
        category="上下文",
        uri="command://context/status",
    ),
    CommandDef(
        slash="/compact",
        name_zh="手动压缩",
        description_zh="触发上下文压缩，保留关键信息，释放窗口空间",
        category="上下文",
        uri="command://context/compact",
    ),
    CommandDef(
        slash="/kill",
        name_zh="强制终止",
        description_zh="立即终止当前运行的 agent 进程及其子进程树",
        category="运行时",
        uri="command://run/kill",
    ),
    # ── first-priority additions: audit, verification, workflow ──────────
    CommandDef(
        slash="/diff",
        name_zh="差异查看",
        description_zh="显示当前工作区 git diff，标注受影响文件的 gate 评估状态",
        category="变更",
        uri="command://workspace/diff",
    ),
    CommandDef(
        slash="/verify",
        name_zh="独立验证",
        description_zh="对当前/指定 run-root 执行独立 verifier，返回通过/失败/缺失项",
        category="保证",
        uri="command://verify/run",
    ),
    CommandDef(
        slash="/sources",
        name_zh="来源可视",
        description_zh="显示当前上下文所有来源的可见性状态和门控决策",
        category="保证",
        uri="command://sources/visibility",
    ),
    CommandDef(
        slash="/plan",
        name_zh="执行计划",
        description_zh="进入只读计划模式——分析方案，等待确认后再执行变更",
        category="工作流",
        uri="command://plan/enter",
    ),
    # ── second-priority additions: recovery, model, system status ─────────
    CommandDef(
        slash="/rewind",
        name_zh="回退检查点",
        description_zh="回退对话和/或文件变更到前一个检查点，可分别恢复 task 或 files",
        category="恢复",
        uri="command://checkpoint/rewind",
    ),
    CommandDef(
        slash="/model",
        name_zh="模型切换",
        description_zh="查看/切换当前使用的模型适配器及推理配置",
        category="系统",
        uri="command://model/switch",
    ),
    CommandDef(
        slash="/status",
        name_zh="系统状态",
        description_zh="显示沙箱状态、网络许可、Gate 链路完整性和活跃 agent 进程",
        category="系统",
        uri="command://status",
    ),
    CommandDef(
        slash="/export",
        name_zh="导出对话",
        description_zh="将当前对话导出为审计兼容的 JSON/JSONL 格式",
        category="会话",
        uri="command://conversation/export",
    ),
    CommandDef(
        slash="/search",
        name_zh="网络搜索",
        description_zh="通过本地浏览器搜索网络，打开结果页面并提取内容（B 级 evidence）",
        category="检索",
        uri="command://retrieval/search",
    ),
    CommandDef(
        slash="/retrieve",
        name_zh="论文获取",
        description_zh="通过本地浏览器打开论文页，检测 PDF 并下载存入 evidence store（A 级 evidence）",
        category="检索",
        uri="command://retrieval/paper",
    ),
    CommandDef(
        slash="/help",
        name_zh="指令列表",
        description_zh="显示全部指令、中文说明及键盘快捷键参考",
        category="帮助",
        uri="command://help",
    ),
]

# The first six are shown by default when the user types "/".
_DEFAULT_SLASHES: list[str] = [
    "/new",
    "/history",
    "/context",
    "/compact",
    "/status",
    "/kill",
]


# ── registry ─────────────────────────────────────────────────────────────────


@dataclass
class CommandRegistry:
    """In-memory slash-command registry with prefix search.

    Usage::

        registry = CommandRegistry.with_builtins()
        matches = registry.search("/con")  # → [CommandDef("/context", ...), ...]
        defaults = registry.get_defaults()  # → first six
    """

    _commands: dict[str, CommandDef] = field(default_factory=dict)
    _default_order: list[str] = field(default_factory=list)

    # ── builders ──────────────────────────────────────────────────────────

    @classmethod
    def with_builtins(cls) -> CommandRegistry:
        """Return a registry pre-loaded with the built-in command set."""
        reg = cls()
        for cmd in _BUILTIN_COMMANDS:
            reg.register(cmd)
        for slash in _DEFAULT_SLASHES:
            if slash in reg._commands:
                reg._default_order.append(slash)
        return reg

    # ── mutations ─────────────────────────────────────────────────────────

    def register(self, cmd: CommandDef) -> None:
        """Add or replace a command definition."""
        if not cmd.slash.startswith("/"):
            raise ValueError(f"slash command must start with '/': {cmd.slash}")
        self._commands[cmd.slash] = cmd

    # ── queries ───────────────────────────────────────────────────────────

    def get(self, slash: str) -> CommandDef | None:
        """Return the command for *slash*, or ``None``."""
        return self._commands.get(slash)

    def search(self, prefix: str) -> list[CommandDef]:
        """Return commands whose slash starts with *prefix*, in default order.

        If *prefix* is just ``"/"`` (no further characters typed), return
        the default six.  Otherwise perform prefix matching against all
        registered commands.
        """
        if not prefix.startswith("/"):
            return []
        if prefix == "/":
            return self.get_defaults()
        results: list[CommandDef] = []
        for slash in self._default_order:
            cmd = self._commands.get(slash)
            if cmd and cmd.slash.startswith(prefix):
                results.append(cmd)
        # Append any non-default commands that also match
        for slash, cmd in self._commands.items():
            if slash not in self._default_order and cmd.slash.startswith(prefix):
                results.append(cmd)
        return results

    def get_defaults(self) -> list[CommandDef]:
        """Return the six default commands in priority order."""
        return [self._commands[s] for s in self._default_order if s in self._commands]

    def all(self) -> list[CommandDef]:
        """Return every registered command, in registration order."""
        return list(self._commands.values())

    def __len__(self) -> int:
        return len(self._commands)

    def __contains__(self, slash: str) -> bool:
        return slash in self._commands


# ── module-level convenience ─────────────────────────────────────────────────

_builtin_registry: CommandRegistry | None = None


def get_builtin_registry() -> CommandRegistry:
    """Return a shared, lazily-constructed built-in command registry."""
    global _builtin_registry
    if _builtin_registry is None:
        _builtin_registry = CommandRegistry.with_builtins()
    return _builtin_registry
