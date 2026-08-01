"""TUI application compositor — GAK-UI-001.

:class:`TuiPrototype` holds all widgets, manages layout and focus, and
produces the full-screen render.  :func:`render_screen` is a convenience
entry point that builds a prototype with sample data and renders it.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any

from .events import RetrievalOutcome, RetrievalProgress
from .view_models import (
    SAMPLE_ADDRESS_URI,
    SAMPLE_CLAIM_DISPOSITION,
    SAMPLE_COMMAND_URI,
    SAMPLE_COMMANDS,
    SAMPLE_CONTENT_MARKER,
    SAMPLE_DIALOG,
    SAMPLE_DISPOSITION_REASON,
    SAMPLE_EVENT_GROUPS,
    SAMPLE_FIND_QUERY,
    SAMPLE_FIND_SCOPE,
    SAMPLE_FIND_SCOPES,
    SAMPLE_MENUS,
    SAMPLE_NEXT_ACTIONS,
    SAMPLE_PROPERTIES,
    SAMPLE_SOURCE_TABLE,
    SAMPLE_SOURCE_TREE,
    SAMPLE_STATUS_ITEMS,
    EventGroup,
)
from .widgets import (
    AddressBar,
    AddressDialog,
    AnnouncementStrip,
    ChatInput,
    ChatMessage,
    CommandPalette,
    ContentMarker,
    ContentPane,
    Dialog,
    ExplorerPane,
    FindBar,
    FindDialog,
    HelpOverlay,
    MenuBar,
    PropertiesSheet,
    StatusBar,
    Toolbar,
    ToolTraceLine,
    box_bottom,
    box_horizontal,
    box_t_junction,
)


# Layout constants
EXPLORER_WIDTH = 22   # fixed-width explorer pane (20 + 2 borders)
MARKER_WIDTH = 16     # right-side content marker (14 + 2 borders)
FIXED_HEIGHT = 1      # each fixed row consumes 1 line


@dataclass
class TuiPrototype:
    """Full retro-desktop terminal UI compositor.

    Manages widget instances, layout regions, focus cycling, and screen
    composition.  Call :meth:`render` to produce a full-screen string.

    Keyboard dispatch:
      - ``f6`` — cycle focus through major panes
      - ``tab`` / ``s-tab`` — forward/back in current pane
      - ``esc`` — running→cancel run → clear buffer → close menu → dismiss dialog
      - ``enter`` — activate focused item
      - ``f5`` — toggle run start/stop (demo simulation only)
      - ``alt+<letter>`` — activate menu
    """

    # ── widgets ──
    menu_bar: MenuBar = field(default_factory=MenuBar)
    toolbar: Toolbar = field(default_factory=Toolbar)
    address_bar: AddressBar = field(default_factory=AddressBar)
    find_bar: FindBar = field(default_factory=FindBar)
    explorer_pane: ExplorerPane = field(default_factory=ExplorerPane)
    content_pane: ContentPane = field(default_factory=ContentPane)
    content_marker: ContentMarker = field(default_factory=ContentMarker)
    status_bar: StatusBar = field(default_factory=StatusBar)
    dialog: Dialog = field(default_factory=Dialog)
    properties: PropertiesSheet = field(default_factory=PropertiesSheet)
    command_palette: CommandPalette = field(default_factory=CommandPalette)
    help_overlay: HelpOverlay = field(default_factory=HelpOverlay.with_defaults)
    find_dialog: FindDialog = field(default_factory=FindDialog)
    address_dialog: AddressDialog = field(default_factory=AddressDialog)
    announcement_strip: AnnouncementStrip = field(default_factory=AnnouncementStrip)
    chat_input: ChatInput = field(default_factory=lambda: ChatInput(focused=True))

    # ── sidebar visibility (GAK-UI-001 P2) ──
    show_explorer: bool = True
    show_events: bool = True
    show_markers: bool = True

    # ── navigation history (toolbar back/forward) ──
    _nav_history: list[str] = field(default_factory=list)
    _nav_index: int = -1

    # ── event-driven state (Phase 1) ──
    event_source: Any | None = None   # EventSource | None (typed Any to avoid circular import)
    _event_log: list[Any] = field(default_factory=list)  # list[TuiEvent]
    _status_messages: list[str] = field(default_factory=list)

    # ── retrieval handler (LBR-001) ──
    # Callable[[str, str, Callable[[RetrievalProgress], None]], RetrievalOutcome] | None
    # Wired at the application boundary (main.py) to avoid assurance imports in TUI.
    retrieval_handler: Any | None = None

    # ── terminal title (P0 extension) ──
    _title_prefix: str = "GSA"

    # ── permission bridge (D1.13) ──
    # True when the visible dialog is a pending ACP permission request.
    # The key handler routes Enter/Esc decisions back to the event source.
    _pending_permission: bool = False

    # ── runtime state ──
    running: bool = False       # True while agent is executing a run
    _last_sent: str = ""        # preserved when run is cancelled (refill bar)

    # ── focus ──
    _focusable_panes: tuple[str, ...] = (
        "explorer", "checklist", "content", "marker", "chat",
    )
    _active_pane_index: int = 4  # default: chat input
    _last_esc_time: float = field(default=0.0)  # P3.4: double-Esc session list

    # ── public API ──────────────────────────────────────────────────────────

    def render(self, width: int, height: int) -> str:
        """Render the full TUI screen at *width* × *height*."""
        if width < 80 or height < 24:
            return f"Viewport too small ({width}x{height}). Minimum 80x24 required."

        inner_w = width - 2  # inside │ borders
        lines: list[str] = []

        # ── visibility flags shorthand ──
        se = self.show_explorer
        sm = self.show_markers

        # ── top border ──
        lines.append(box_horizontal("", width))

        # ── Row 0: MenuBar (two-row: menus + toolbar) ──
        menu = self.menu_bar.render(inner_w, 1)
        lines.append("│" + menu[0] + "│")

        # ── Row 1: Toolbar (P3.1: address/find folded in as right buttons) ──
        tool = self.toolbar.render(inner_w, 1)
        lines.append("│" + tool[0] + "│")
        lines.append(box_t_junction(width))

        # ── Row 2: AnnouncementStrip (L1 checklist bar) — only when active ──
        if self.announcement_strip.items:
            chk_extra = self.announcement_strip.expanded_height
            chk_height = 1 + chk_extra
            chk = self.announcement_strip.render(inner_w, chk_height)
            for chk_line in chk:
                lines.append("│" + chk_line + "│")
            lines.append(box_t_junction(width))

        # ── explorer / content / marker 3-column split ──
        exp_w = (EXPLORER_WIDTH - 2) if se else 0
        marker_w = (MARKER_WIDTH - 2) if sm else 0
        # Count visible separators
        sep_count = (1 if se else 0) + (1 if sm else 0)
        content_w = inner_w - exp_w - marker_w - sep_count

        # Separator line
        seps: list[str] = []
        if se:
            seps.append("─" * exp_w)
            seps.append("┬")
        seps.append("─" * content_w)
        if sm:
            seps.append("┬")
            seps.append("─" * marker_w)
        sep_line = "├" + "".join(seps) + "┤"
        lines.append(sep_line)

        # ── Body (3-column with ChatInput below ContentPane) ──
        # Dynamic: menu(1) + toolbar(1) + status(1) + top(1)/bottom(1) border + 2 internal junctions
        fixed = 1 + 1 + 1 + 1 + 1 + 2
        if self.announcement_strip.items:
            fixed += 1 + chk_extra + 1
        body_height = height - fixed
        if body_height < 4:
            body_height = 4  # need at least 3 for content + 1 for chat input

        chat_height = 1
        content_height = body_height - chat_height

        explorer_inner = self._render_pane_inner(self.explorer_pane, exp_w, body_height) if se else []
        content_inner = self._render_pane_inner(self.content_pane, content_w, content_height)
        chat_inner = self.chat_input.render(content_w, chat_height)
        marker_inner = self._render_pane_inner(self.content_marker, marker_w, body_height) if sm else []

        for i in range(body_height):
            row_parts: list[str] = ["│"]
            if se:
                el = explorer_inner[i] if i < len(explorer_inner) else " " * exp_w
                row_parts.append(pad_to_width(el, exp_w) + "│")
            if i < content_height:
                cl = content_inner[i] if i < len(content_inner) else " " * content_w
                row_parts.append(pad_to_width(cl, content_w))
            else:
                # ChatInput row — aligned under ContentPane
                ci = chat_inner[i - content_height] if (i - content_height) < len(chat_inner) else " " * content_w
                row_parts.append(pad_to_width(ci, content_w))
            if sm:
                row_parts.append("│")
                ml = marker_inner[i] if i < len(marker_inner) else " " * marker_w
                row_parts.append(pad_to_width(ml, marker_w))
            row_parts.append("│")
            lines.append("".join(row_parts))

        # ── body bottom separator ──
        bot_seps: list[str] = []
        if se:
            bot_seps.append("─" * exp_w)
            bot_seps.append("┴")
        bot_seps.append("─" * content_w)
        if sm:
            bot_seps.append("┴")
            bot_seps.append("─" * marker_w)
        bot_sep = "├" + "".join(bot_seps) + "┤"
        lines.append(bot_sep)

        # ── StatusBar ──
        if self.status_bar.items and self.status_bar.items[-1][0] in ("空闲", "运行中", "IDLE", "RUNNING"):
            fixed = list(self.status_bar.items)
            fixed[-1] = ("运行中", True) if self.running else ("空闲", True)
            self.status_bar.items = fixed
        status = self.status_bar.render(inner_w, 1)
        lines.append("│" + status[0] + "│")

        # ── bottom border ──
        lines.append(box_bottom(width))

        result = "\n".join(lines)

        # ── Overlays (priority: help > find > palette > dialog > properties) ──
        if self.help_overlay.visible:
            overlay = self.help_overlay.render_overlay(width, height)
            if overlay:
                result = self._blend_overlay(result, overlay, height)
            return result  # help blocks everything else

        if self.address_dialog.visible:
            overlay = self.address_dialog.render_overlay(width, height)
            if overlay:
                result = self._blend_overlay(result, overlay, height)
            return result

        if self.find_dialog.visible:
            overlay = self.find_dialog.render_overlay(width, height)
            if overlay:
                result = self._blend_overlay(result, overlay, height)
            return result

        if self.address_bar.focused and self.address_bar._show_autocomplete:
            self.command_palette.candidates = self.address_bar.autocomplete_candidates
            self.command_palette.selected_index = self.address_bar._selected_index
            self.command_palette.visible = True
            overlay = self.command_palette.render_overlay(width, height)
            if overlay:
                result = self._blend_overlay(result, overlay, height)
        else:
            self.command_palette.visible = False

        if self.dialog.visible and not self.command_palette.visible:
            overlay = self.dialog.render_overlay(width, height)
            if overlay:
                result = self._blend_overlay(result, overlay, height)
        elif self.properties.visible and not self.command_palette.visible:
            overlay = self.properties.render_overlay(width, height)
            if overlay:
                result = self._blend_overlay(result, overlay, height)

        return result

    @staticmethod
    def _render_pane_inner(pane: object, width: int, height: int) -> list[str]:
        """Render a pane's inner content (no outer borders).

        Strips the outer │ borders from box_vertical output, or renders
        the widget at (width, height) and returns raw lines.
        """
        raw = pane.render(width, height)
        # If the pane used box_vertical, unwrap the borders
        stripped: list[str] = []
        for line in raw:
            if line and line[0] in ("│", "║") and line[-1] in ("│", "║"):
                stripped.append(line[1:-1])
            else:
                stripped.append(line)
        # Trim or pad
        if len(stripped) > height:
            stripped = stripped[:height]
        while len(stripped) < height:
            stripped.append(" " * width)
        # Ensure each line fits
        return [pad_to_width(s, width) for s in stripped]

    def handle_key(self, key: str) -> str | None:
        """Dispatch *key* and return an optional status message."""
        # Checklist L2/L3 consumes nav keys (↑↓ Esc Enter) when expanded
        if self.announcement_strip.l2_expanded or self.announcement_strip.l3_expanded:
            if key in ("up", "down", "enter", "esc"):
                self.announcement_strip.handle_key(key)
                return None

        # HelpOverlay consumes all keys while visible
        if self.help_overlay.visible:
            self.help_overlay.handle_key(key)
            return None

        # FindDialog consumes all keys while visible
        if self.find_dialog.visible:
            self.find_dialog.handle_key(key)
            return None

        # AddressDialog consumes all keys while visible
        if self.address_dialog.visible:
            self.address_dialog.handle_key(key)
            if not self.address_dialog.visible:
                # Dialog just closed — dispatch the command
                sent = self.address_dialog.buffer.strip()
                if sent:
                    self.address_dialog.history.insert(0, sent)
                    return self._dispatch_command(sent)
            return None

        # Dialogs/properties consume all keys while visible
        if self.dialog.visible:
            consumed = self.dialog.handle_key(key)
            if key == "enter" and self.dialog.visible:
                action_idx = self.dialog._selected_action
                if action_idx < len(self.dialog.actions):
                    action_value = self.dialog.actions[action_idx][1]
                    self.dialog.visible = False
                    # ── Permission dialog (D1.13) ──
                    if getattr(self, "_pending_permission", False):
                        self._pending_permission = False
                        if self.event_source and hasattr(self.event_source, "respond_to_permission"):
                            self.event_source.respond_to_permission(action_value)
                        return f"Permission: {action_value}"
                    if action_value == "approve":
                        self._execute_retrieval()
                        return "检索已启动"
                    return "已取消"
            if key == "esc":
                self.dialog.visible = False
                # Esc on a permission dialog = cancel
                if getattr(self, "_pending_permission", False):
                    self._pending_permission = False
                    if self.event_source and hasattr(self.event_source, "respond_to_permission"):
                        self.event_source.respond_to_permission("cancelled")
                    return "Permission: cancelled"
                return "已取消"
            return None
        if self.properties.visible:
            self.properties.handle_key(key)
            return None

        # Menu activation (Alt+letter for Chinese menus)
        if key.startswith("alt+"):
            self.menu_bar.handle_key(key)
            return None

        # F1 or Alt+H → Help overlay
        if key in ("f1", "alt+h"):
            self.help_overlay.visible = not self.help_overlay.visible
            return "帮助" if self.help_overlay.visible else None

        # Ctrl+F → Find dialog
        if key == "c-f":
            self._wire_find_callback()
            self.find_dialog.visible = not self.find_dialog.visible
            if self.find_dialog.visible:
                self.toolbar.highlighted.add("查找...")
            else:
                self.toolbar.highlighted.discard("查找...")
            return "查找" if self.find_dialog.visible else None

        if key == "c-l":
            current = self.address_bar.uri or ""
            self.address_dialog.open_dialog(current)
            self.toolbar.highlighted.add("命令...")
            return "命令…"

        if key == "esc":
            # Clear toolbar highlights when dialogs close.
            self.toolbar.highlighted.discard("命令...")
            self.toolbar.highlighted.discard("查找...")
            # 1. If help is visible, close it
            if self.help_overlay.visible:
                self.help_overlay.visible = False
                return None
            # 2. If address bar has auto-complete or buffer, clear it
            if self.address_bar.focused and (
                self.address_bar._show_autocomplete or self.address_bar._buffer
            ):
                self.address_bar.handle_key("esc")
                return None
            # 3. If menu is open, close it
            if self.menu_bar.active_menu:
                self.menu_bar.handle_key("esc")
                return None
            # 4. If session list is open in explorer, close it
            if self.explorer_pane.in_session_mode:
                self._toggle_session_list()
                return "来源树"
            # 5. Double-Esc (P3.4): nothing to dismiss → check for double-tap
            import time as _t
            now = _t.monotonic()
            if now - self._last_esc_time < 0.5:
                self._last_esc_time = 0.0
                return self._toggle_session_list()
            self._last_esc_time = now
            return None

        # ── Session list navigation (P3.4) ──
        if self.explorer_pane.in_session_mode and key in ("up", "down", "enter"):
            return self._navigate_session_list(key)

        if key == "backspace":
            if self.active_pane == "address" and self.address_bar._buffer:
                self.address_bar.handle_key("backspace")
                return None
            return None

        if key == "c-z":
            if self.running:
                self._cancel_run()
                return "已取消当前运行"
            return None

        if key == "f5":
            return self._toggle_run()

        if key == "f6":
            return self._cycle_focus()

        # ── Chat input (P4) ──────────────────────────────────────────────
        if self.active_pane == "chat":
            result = self.chat_input.handle_key(key)
            if result is not None:
                # Enter pressed with text → send as prompt.
                self.content_pane.add_user_message(result)
                if self.event_source is not None and hasattr(self.event_source, "send_prompt"):
                    self.event_source.send_prompt(result)
                    self.status_bar.update_item("Prompt", True)
                    return f"发送: {result[:60]}"
                else:
                    self._start_run(result)
                    return f"运行: {result[:60]}"
            return None

        # ── toolbar button keyboard shortcuts ──
        if key == "a-left":
            return self._trigger_toolbar("后退")
        if key == "a-right":
            return self._trigger_toolbar("前进")
        if key == "c-c" and not self.running:
            pass  # Ctrl+C = no-op (don't quit)
        if key == "c-break":
            if self.running:
                self._cancel_run()
                return "已强制停止"
            return None

        # ── ContentPane tool expand/collapse (P2.3) ──
        if key == "e" and self.active_pane == "content":
            names = self.content_pane.tool_trace_names()
            if names:
                self.content_pane.toggle_tool_expand(names[-1])
                return f"切换: {names[-1]}"
            return None

        # Printable characters + editing keys → route to focused address bar.
        if self.active_pane == "address":
            if key == "enter" and self.address_bar._buffer:
                self._last_sent = self.address_bar._buffer
            if self.address_bar.handle_key(key):
                if key == "enter" and self._last_sent:
                    self.address_bar._push_history(self._last_sent)
                    return self._dispatch_command(self._last_sent)
                return None

        # Delegate to active pane
        pane = self._get_active_pane()
        if pane and pane.handle_key(key):
            return None

        return None

    # ── internal ────────────────────────────────────────────────────────────

    @property
    def active_pane(self) -> str:
        return self._focusable_panes[self._active_pane_index]

    def _get_active_pane(self) -> Any:
        mapping = {
            "explorer": self.explorer_pane,
            "checklist": self.announcement_strip,
            "chat": self.chat_input,
            "content": self.content_pane,
            "marker": self.content_marker,
            "address": self.address_bar,
            "find": self.find_bar,
        }
        return mapping.get(self.active_pane)

    def _dispatch_command(self, text: str) -> str | None:
        """Route a slash command or URI entered in the address bar."""
        text = text.strip()

        # /help — show Help overlay
        if text == "/help":
            self.help_overlay.visible = not self.help_overlay.visible
            return "帮助" if self.help_overlay.visible else None

        # /search <query> — browser-based web search
        if text.startswith("/search "):
            query = text[len("/search "):].strip()
            if not query:
                return "用法: /search <关键词>"
            self._show_retrieval_dialog("search", query)
            return f"搜索: {query}"

        # /retrieve <url> — browser-based paper retrieval
        if text.startswith("/retrieve "):
            url = text[len("/retrieve "):].strip()
            if not url:
                return "用法: /retrieve <url>"
            self._show_retrieval_dialog("retrieve", url)
            return f"获取: {url}"

        # /plan — show plan state or enter plan mode
        if text == "/plan":
            if self.announcement_strip.items:
                self.announcement_strip.l2_expanded = True
                return "Plan 展开"
            return "Plan: 无活跃计划。直接输入任务开始。"

        # /verify — trigger verification on current content
        if text == "/verify":
            cp = self.content_pane
            source_count = len(cp.source_table)
            msg_count = len(cp._items)
            return f"验证: {source_count} sources, {msg_count} messages"

        # /stop or /kill — force cancel
        if text in ("/stop", "/kill"):
            if self.running:
                self._cancel_run()
                return "已强制停止"
            return "没有运行中的任务"

        # /open <file> — open in PropertiesSheet
        if text.startswith("/open "):
            target = text[len("/open "):].strip()
            if target:
                self.properties.visible = True
                self.properties._active_tab = 0
                return f"打开: {target}"
            return "用法: /open <文件路径>"

        # Sidebar toggle commands
        if text == "/toggle-explorer":
            self.show_explorer = not self.show_explorer
            return f"资源管理器: {'显示' if self.show_explorer else '隐藏'}"
        if text == "/toggle-events":
            self.show_events = not self.show_events
            self.explorer_pane.show_events = self.show_events
            return f"事件: {'显示' if self.show_events else '隐藏'}"
        if text == "/toggle-markers":
            self.show_markers = not self.show_markers
            return f"标记: {'显示' if self.show_markers else '隐藏'}"

        # Other slash commands — acknowledge
        if text.startswith("/"):
            return f"已激活: {text}"

        # D1.10: Non-command text in AddressBar falls through.
        # The ChatInput widget is the primary prompt entry point.
        # AddressBar is for command:// URIs and slash commands only.
        return None

    def _show_retrieval_dialog(self, mode: str, target: str) -> None:
        """Show a permission dialog for browser retrieval."""
        if mode == "search":
            title = "Browser Search"
            message = (
                f"The CLI will open your browser to search for:\n"
                f"\n"
                f"  {target}\n"
                f"\n"
                f"Google search → open results → extract content.\n"
                f"No credentials are sent to the model.\n"
                f"\n"
                f"Allow this search?"
            )
        else:
            title = "Paper Retrieval"
            message = (
                f"The CLI will open your browser to retrieve:\n"
                f"\n"
                f"  {target}\n"
                f"\n"
                f"Open page → detect PDF → download → store in evidence store.\n"
                f"No credentials are sent to the model.\n"
                f"\n"
                f"Allow this retrieval?"
            )

        self.dialog = Dialog(
            title=title,
            message=message,
            actions=[
                ("Approve", "approve"),
                ("Reject", "reject"),
            ],
            visible=True,
        )
        self.dialog._selected_action = 0
        # Store retrieval params for execution after approval
        self._pending_retrieval: dict = {"mode": mode, "target": target}

    def _execute_retrieval(self) -> None:
        """Run the approved retrieval in a background thread.

        Delegates to :attr:`retrieval_handler` which is wired at the
        application boundary (:mod:`assurance.tui.main`) so that the TUI
        layer never imports from ``assurance.*`` directly.

        If no handler is configured (demo / static render), logs a warning.
        """
        params = getattr(self, "_pending_retrieval", None)
        if not params:
            return
        self._pending_retrieval = {}

        if self.retrieval_handler is None:
            self._status_messages.append(
                "WARNING: retrieval_handler not wired — retrieval is a no-op in static/demo mode"
            )
            self.status_bar.update_item("空闲", True)
            self.content_pane.set_disposition(
                "未接线",
                "检索处理器未接线。请从 main.py 运行以启用实时检索。",
            )
            return

        import threading
        mode = params["mode"]
        target = params["target"]

        # Build a progress callback owned by the TUI layer — uses TUI's own
        # RetrievalProgress type, NOT the assurance type.
        def _on_progress(evt: RetrievalProgress) -> None:
            msg = f"[{evt.stage}] {evt.message}"
            self._status_messages.append(msg)
            self.explorer_pane.add_event_entry("Retrieval", msg)
            stage_labels: dict[str, str] = {
                "searching": "搜索中", "navigating": "获取中",
                "reading": "读取中", "downloading": "下载中",
                "storing": "存储中", "indexing": "索引中",
                "done": "空闲", "failed": "错误",
                "launching": "启动中", "connecting": "连接中",
                "finding_pdf": "查找PDF", "validating": "验证中",
                "retrieving": "检索中",
            }
            label = stage_labels.get(evt.stage, evt.stage.upper())
            self.status_bar.update_item(label, evt.stage != "failed")

        def _run() -> None:
            try:
                outcome: RetrievalOutcome = self.retrieval_handler(
                    mode, target, _on_progress,
                )
                if outcome.ok:
                    self.status_bar.update_item("空闲", True)
                    self.content_pane.set_disposition(outcome.title, outcome.detail)
                else:
                    self.status_bar.update_item("错误", False)
                    self.content_pane.set_disposition(
                        outcome.title or "失败",
                        outcome.error or outcome.detail,
                    )
            except Exception as exc:
                self._status_messages.append(f"错误: {exc}")
                self.status_bar.update_item("错误", False)

        threading.Thread(target=_run, daemon=True).start()

    def _cancel_run(self) -> None:
        """Cancel the current agent run, restore the last sent input to the
        address bar, and focus it so the user can edit and resend."""
        self.running = False
        self.address_bar._buffer = self._last_sent
        self.address_bar._show_autocomplete = self._last_sent.startswith("/")
        self.address_bar._selected_index = 0
        # Focus the address bar so the user can immediately edit
        while self.active_pane != "address":
            self._active_pane_index = (
                (self._active_pane_index + 1) % len(self._focusable_panes)
            )
        self.address_bar.focused = True

    def _toggle_run(self) -> str:
        """Demo-only: toggle agent run state for testing Esc-cancel."""
        self.running = not self.running
        return "模拟运行已启动" if self.running else "模拟运行已停止"

    def _cycle_focus(self) -> str:
        # Unfocus current
        current = self._get_active_pane()
        if current:
            current.focused = False
        # Advance
        self._active_pane_index = (
            (self._active_pane_index + 1) % len(self._focusable_panes)
        )
        # Focus new
        new_pane = self._get_active_pane()
        if new_pane:
            new_pane.focused = True
        return f"Focus: {self.active_pane}"

    def _start_run(self, prompt_text: str) -> None:
        """Start a new Grok ACP session with *prompt_text*.

        Creates a temporary run root, builds the live run function via the
        bridge, and attaches it as the active event source.
        """
        import tempfile
        from pathlib import Path as _Path
        run_root = _Path(tempfile.mkdtemp(prefix="gsa-run-"))
        workspace = run_root / "workspace"
        workspace.mkdir(parents=True, exist_ok=True)

        try:
            from .bridge import build_grok_acp_live_run_fn
            result = build_grok_acp_live_run_fn(
                run_root=str(run_root),
                workspace=str(workspace),
                prompt_text=prompt_text,
                interactive=True,
            )
        except Exception as exc:
            self._status_messages.append(f"运行失败: {exc}")
            return

        if isinstance(result, tuple):
            run_fn, perm_q, prompt_q = result
        else:
            run_fn, perm_q, prompt_q = result, None, None

        from .event_source import LiveRunEventSource
        source = LiveRunEventSource(
            run_fn=run_fn,
            permission_queue=perm_q,
            prompt_queue=prompt_q,
        )
        self.event_source = source
        source.start()
        self.running = True
        self.status_bar.update_item("运行中", True)
        self._status_messages.append(f"已开始: {prompt_text[:60]}")

    def update_terminal_title(self, status: str) -> None:
        """Set terminal window/tab title via OSC escape sequence.

        Call after state transitions (run start, permission wait, gate
        block, completion, idle).  No-op outside a full-screen TUI session.
        """
        title = f"{self._title_prefix} — {status}"
        _set_terminal_title(title)

    def _wire_find_callback(self) -> None:
        """Lazily wire the FindDialog's search callback to ContentPane."""
        if self.find_dialog.on_search is not None:
            return  # already wired
        def _search(query: str, scope: str) -> None:
            case = self.find_dialog.case_sensitive
            results = self.content_pane.search(query, case_sensitive=case)
            # Show results in status bar.
            if results:
                self.status_bar.update_item("查找", True)
                self.status_bar.update_item(
                    "查找结果", True,
                )
                self._status_messages.append(
                    f"查找: {query} — {len(results)} matches"
                )
            else:
                self._status_messages.append(f"查找: {query} — 未找到")
        self.find_dialog.on_search = _search

    def handle_mouse_click(self, col: int, row: int) -> str | None:
        """Handle a mouse click at terminal coordinates (col, row).

        Maps click position to clickable regions in the layout:
        Row 1 = Toolbar, Row 3+ = body (Explorer/Content/Marker split).
        """
        # Row 1: Toolbar buttons
        if row == 1:
            for btn_name in self.toolbar.buttons + list(self.toolbar.right_buttons):
                pos = self._find_button_position(btn_name)
                if pos is not None and pos <= col < pos + len(btn_name) + 2:
                    return self._trigger_toolbar(btn_name)
        # Row 2: AnnouncementStrip (if visible)
        if row == 2 and self.announcement_strip.items:
            self.announcement_strip.l2_expanded = not self.announcement_strip.l2_expanded
            return "切换 checklist"
        # ContentPane area (approximate — clicks on [▸ 展开] / [关闭] regions)
        if 22 <= col <= 84 and row >= 3:
            return self._handle_content_click(col, row)
        return None

    def _find_button_position(self, name: str) -> int | None:
        """Return the starting x-position of a toolbar button, or None.

        Left-group buttons start at column 1 (inside border) and grow right.
        Right-group buttons are right-aligned within the toolbar width.
        """
        sep = "  "
        # Check left group.
        x = 1
        for btn in self.toolbar.buttons:
            if btn == name:
                return x
            x += len(btn) + len(sep)
        # Check right group — need approximate toolbar width.
        # The TUI renders at a known width; use 100 as a reasonable default
        # (mouse click coordinates come from the actual terminal width).
        left_text = sep.join(self.toolbar.buttons) if self.toolbar.buttons else ""
        right_text = sep.join(self.toolbar.right_buttons)
        # Right-aligned: gap = width - left_width - right_width.
        # We don't know the exact terminal width here, so approximate
        # using a conservative value and the rightmost positions.
        approx_width = 100
        left_w = sum(len(b) for b in self.toolbar.buttons) + len(sep) * max(0, len(self.toolbar.buttons) - 1)
        right_w = sum(len(b) for b in self.toolbar.right_buttons) + len(sep) * max(0, len(self.toolbar.right_buttons) - 1)
        r_start = approx_width - right_w - 1  # -1 for right border
        rx = r_start
        for btn in self.toolbar.right_buttons:
            if btn == name:
                return rx
            rx += len(btn) + len(sep)
        return None

    def _handle_content_click(self, col: int, row: int) -> str | None:
        """Handle click in ContentPane area.

        Maps *row* to a specific tool trace line and toggles its
        expand/collapse state.  Each tool line in the conversation
        occupies one row.
        """
        cp = self.content_pane
        names = cp.tool_trace_names()
        if not names:
            return None

        # Determine which tool trace row was clicked.
        # The conversation layout is (starting from row 3 in the terminal):
        #   row 3: source visibility compact bar (if present)
        #   row 4+: conversation items
        base_row = 4 if cp.source_table else 3
        item_idx = row - base_row
        # Account for expanded tool (pin-to-top, takes extra rows).
        for i, item in enumerate(cp._items):
            if isinstance(item, ToolTraceLine) and item.expanded:
                expanded_rows = 2 + len(item.entries)  # header + entries
                if row < base_row + expanded_rows:
                    # Click within expanded area — toggle to collapse.
                    cp.toggle_tool_expand(item.tool_name)
                    return f"关闭: {item.tool_name}"
                base_row += expanded_rows
                break

        if 0 <= item_idx < len(names):
            cp.toggle_tool_expand(names[item_idx])
            return f"切换: {names[item_idx]}"
        # Fallback: toggle the last tool trace.
        cp.toggle_tool_expand(names[-1])
        return f"切换: {names[-1]}"

    # ── toolbar actions (P4) ──────────────────────────────────────────────

    def _trigger_toolbar(self, action: str) -> str | None:
        """Execute a toolbar button action.  Returns a status message."""
        # Record navigation before executing.
        current = self.address_bar.uri
        self._nav_push(current)

        if action == "后退":
            return self._nav_back()
        elif action == "前进":
            return self._nav_forward()
        elif action == "刷新":
            if self.running:
                self._cancel_run()
            self.content_pane._items.clear()
            return "已刷新"
        elif action == "停止":
            if self.running:
                self._cancel_run()
                return "已停止"
            return "没有运行中的任务"
        elif action == "打开":
            self.address_dialog.open_dialog(self.address_bar.uri)
            return "打开地址..."
        elif action == "验证":
            self.address_bar.uri = "command://verify/current-task"
            return "验证: 当前任务"
        elif action == "属性":
            self.properties.visible = True
            return "属性"
        elif action == "命令...":
            self.address_dialog.open_dialog("")
            self.toolbar.highlighted.add("命令...")
            return "命令..."
        elif action == "查找...":
            self._wire_find_callback()
            self.find_dialog.visible = not self.find_dialog.visible
            if self.find_dialog.visible:
                self.toolbar.highlighted.add("查找...")
            else:
                self.toolbar.highlighted.discard("查找...")
            return "查找..."
        return f"未知操作: {action}"

    # ── navigation history ────────────────────────────────────────────────

    def _nav_push(self, uri: str) -> None:
        """Push *uri* onto the navigation stack."""
        if self._nav_index >= 0 and self._nav_history[self._nav_index] == uri:
            return  # already at this location
        # Truncate forward history.
        self._nav_history = self._nav_history[:self._nav_index + 1]
        self._nav_history.append(uri)
        self._nav_index = len(self._nav_history) - 1

    def _nav_back(self) -> str | None:
        if self._nav_index > 0:
            self._nav_index -= 1
            prev = self._nav_history[self._nav_index]
            self.address_bar.uri = prev
            return f"后退: {prev[:40]}"
        return "已是最早位置"

    def _nav_forward(self) -> str | None:
        if self._nav_index < len(self._nav_history) - 1:
            self._nav_index += 1
            nxt = self._nav_history[self._nav_index]
            self.address_bar.uri = nxt
            return f"前进: {nxt[:40]}"
        return "已是最新位置"

    # ── session list (P3.4) ───────────────────────────────────────────────

    def _toggle_session_list(self) -> str:
        """Toggle ExplorerPane between source-tree and session-list modes.

        Session data is discovered by scanning ``.gsa/runs/`` for Grok ACP
        session directories that contain an ``events.jsonl`` artifact.
        Grok owns session persistence; the TUI only reads its output.
        """
        pane = self.explorer_pane
        if pane.in_session_mode:
            pane.toggle_session_mode()
            return "来源树"
        sessions = _discover_sessions()
        pane.load_sessions(sessions)
        pane.toggle_session_mode()
        return f"会话列表 ({len(sessions)})"

    def _navigate_session_list(self, direction: str) -> str | None:
        """Handle Up/Down in session list mode.

        Enter on a session opens its run_root for inspection (read-only);
        full session resume uses ``grok session resume`` (Grok-owned).
        """
        pane = self.explorer_pane
        if not pane.in_session_mode:
            return None
        if direction == "up":
            pane.session_selection_up()
        elif direction == "down":
            pane.session_selection_down()
        elif direction == "enter":
            sid = pane.selected_session_id()
            if sid:
                pane.toggle_session_mode()
                return f"会话: {sid}"
        return None

    @staticmethod
    def _blend_overlay(original: str, overlay: list[str], height: int) -> str:
        """Composite *overlay* lines on top of *original* using a
        rectangular cut-out approach: for each overlay line, find the
        first and last non-space character, and replace the original
        content within that span.  Space cells outside the span are
        transparent.
        """
        orig_lines = original.split("\n")
        if not orig_lines:
            return original
        line_width = max(len(l) for l in orig_lines)
        if len(orig_lines) < height:
            orig_lines += [" " * line_width] * (height - len(orig_lines))

        result: list[str] = []
        for i in range(min(height, len(orig_lines))):
            og = orig_lines[i]
            ov = overlay[i] if i < len(overlay) else ""
            if not ov or not ov.strip():
                result.append(og)
                continue
            # Find the non-space span in the overlay
            stripped = ov.rstrip()
            left = len(stripped) - len(stripped.lstrip())
            right = len(stripped)
            # Build: [left part of og] + [overlay span] + [right part of og]
            og_padded = og + " " * max(0, right - len(og))
            blended = og_padded[:left] + stripped[left:right] + og_padded[right:]
            result.append(blended)
        # Pad remaining
        while len(result) < height:
            result.append(" " * line_width)
        return "\n".join(result[:height])

    # ── event-driven API ───────────────────────────────────────────────────

    def poll_events(self) -> list[str]:
        """Drain available events from :attr:`event_source` and project them.

        Called from the prompt_toolkit background drain coroutine.
        Returns status messages produced by the projector.
        """
        if not self.event_source:
            return []
        events = self.event_source.poll()
        if not events:
            return []
        from .projector import apply_event
        messages: list[str] = []
        for evt in events:
            self._event_log.append(evt)
            msgs = apply_event(self, evt)
            messages.extend(msgs)
        return messages

    def _ensure_event_groups(self) -> None:
        """Populate ExplorerPane with the standard event group categories."""
        if self.explorer_pane.event_groups:
            return  # already initialised
        defaults = [
            EventGroup("Run", 0, True, []),
            EventGroup("Decisions", 0, True, []),
            EventGroup("Errors", 0, False, []),
            EventGroup("Permissions", 0, False, []),
            EventGroup("Tool Calls", 0, False, []),
            EventGroup("Artifacts", 0, False, []),
            EventGroup("Sources", 0, True, []),
        ]
        self.explorer_pane.event_groups = defaults

    @classmethod
    def with_event_source(cls, source: Any) -> TuiPrototype:
        """Build a prototype wired to *source* with full base UI.

        Menus, toolbar, status bar, and address bar are populated from
        sample data so the TUI looks functional immediately.  Dynamic
        content (event groups, source table, claim disposition) is
        created lazily on the first projected event.
        """
        from .view_models import (
            SAMPLE_COMMAND_URI, SAMPLE_FIND_QUERY, SAMPLE_FIND_SCOPE,
            SAMPLE_FIND_SCOPES, SAMPLE_MENUS, SAMPLE_SOURCE_TREE,
            SAMPLE_STATUS_ITEMS,
        )
        app = cls(
            menu_bar=MenuBar(menus=dict(SAMPLE_MENUS)),
            toolbar=Toolbar(
                buttons=["后退", "前进", "刷新", "停止", "打开", "验证", "属性"],
                right_buttons=["命令...", "查找..."],
            ),
            address_bar=AddressBar(uri=SAMPLE_COMMAND_URI),
            find_bar=FindBar(
                query=SAMPLE_FIND_QUERY, scope=SAMPLE_FIND_SCOPE,
                scopes=list(SAMPLE_FIND_SCOPES),
            ),
            explorer_pane=ExplorerPane(tree=list(SAMPLE_SOURCE_TREE)),
            content_pane=ContentPane(title="当前任务"),
            status_bar=StatusBar(items=list(SAMPLE_STATUS_ITEMS)),
        )
        app.event_source = source
        app._ensure_event_groups()
        return app

    # ── convenience builders ────────────────────────────────────────────────

    @classmethod
    def with_sample_data(cls) -> TuiPrototype:
        """Build a prototype populated with hard-coded sample data."""
        return cls(
            menu_bar=MenuBar(menus=dict(SAMPLE_MENUS)),
            toolbar=Toolbar(
                buttons=["后退", "前进", "刷新", "停止", "打开", "验证", "属性"],
                right_buttons=["命令...", "查找..."],
            ),
            address_bar=AddressBar(uri=SAMPLE_COMMAND_URI),
            find_bar=FindBar(
                query=SAMPLE_FIND_QUERY,
                scope=SAMPLE_FIND_SCOPE,
                scopes=list(SAMPLE_FIND_SCOPES),
            ),
            explorer_pane=ExplorerPane(
                tree=list(SAMPLE_SOURCE_TREE),
                event_groups=list(SAMPLE_EVENT_GROUPS),
            ),
            content_pane=ContentPane(
                title="当前任务",
                source_table=list(SAMPLE_SOURCE_TABLE),
                claim_disposition=SAMPLE_CLAIM_DISPOSITION,
                disposition_reason=SAMPLE_DISPOSITION_REASON,
                next_actions=list(SAMPLE_NEXT_ACTIONS),
            ),
            content_marker=ContentMarker(markers=list(SAMPLE_CONTENT_MARKER)),
            status_bar=StatusBar(items=list(SAMPLE_STATUS_ITEMS)),
            dialog=Dialog(
                title=SAMPLE_DIALOG["title"],
                message=SAMPLE_DIALOG["message"],
                actions=list(SAMPLE_DIALOG["actions"]),
                visible=False,
            ),
            properties=PropertiesSheet(
                title=SAMPLE_PROPERTIES["title"],
                tabs=list(SAMPLE_PROPERTIES["tabs"]),
                visible=False,
            ),
            command_palette=CommandPalette(
                candidates=list(SAMPLE_COMMANDS),
                selected_index=0,
                visible=False,
            ),
        )


# ── terminal title (OSC escape sequences) ──────────────────────────────────


def _set_terminal_title(title: str) -> None:
    """Set terminal window/tab title via OSC 0 escape sequence.

    ``OSC 0 ; <title> BEL`` sets both window title and icon name.
    Most modern terminals (Windows Terminal, iTerm2, Alacritty, Kitty,
    WezTerm) support this.  Silent no-op if stdout is not a TTY.
    """
    import sys as _sys
    if not _sys.stdout.isatty():
        return
    try:
        _sys.stdout.write(f"\x1b]0;{title}\x07")
        _sys.stdout.flush()
    except (OSError, BrokenPipeError):
        pass


_TITLE_RESTORED: bool = False

def _restore_terminal_title(title: str = "") -> None:
    """Restore terminal title on TUI exit (via atexit)."""
    global _TITLE_RESTORED
    if _TITLE_RESTORED:
        return
    _TITLE_RESTORED = True
    _set_terminal_title(title)


# ── helper ──────────────────────────────────────────────────────────────────


def pad_to_width(text: str, width: int) -> str:
    """Pad *text* to exactly *width* display cells."""
    from .widgets import display_width
    current = display_width(text)
    if current >= width:
        return text[:width]
    return text + " " * (width - current)


# ── session discovery (P3.4) ────────────────────────────────────────────────


def _discover_sessions() -> list[dict[str, object]]:
    """Scan ``.gsa/runs/`` for Grok ACP session directories.

    Each session directory contains Grok artifacts (``events.jsonl``,
    ``acp_transcript.jsonl``) and optionally a ``session.json`` summary
    written by the bridge.

    Returns a list of session summary dicts, newest first.  This is a
    read-only view over Grok-owned session data — no separate index is
    maintained.
    """
    import json as _json
    from pathlib import Path as _Path

    runs_dir = _Path(".gsa/runs")
    if not runs_dir.is_dir():
        return []

    sessions: list[dict[str, object]] = []
    for child in sorted(runs_dir.iterdir(), reverse=True):
        if not child.is_dir():
            continue
        events_path = child / "events.jsonl"
        if not events_path.is_file():
            continue
        meta_path = child / "session.json"
        if meta_path.is_file():
            try:
                meta = _json.loads(meta_path.read_text(encoding="utf-8"))
                sessions.append({
                    "session_id": child.name,
                    "created_at": meta.get("created_at", ""),
                    "last_active_at": meta.get("last_active_at", ""),
                    "first_prompt": meta.get("first_prompt", ""),
                    "prompt_preview": meta.get("first_prompt", "")[:80],
                    "turn_count": meta.get("turn_count", 0),
                    "run_root": str(child),
                })
            except (_json.JSONDecodeError, OSError):
                pass
    return sessions


# ── convenience entry point ─────────────────────────────────────────────────


def render_screen(width: int = 100, height: int = 30) -> str:
    """Render a full-screen TUI prototype with sample data.

    >>> output = render_screen()
    >>> assert "MenuBar" not in output  # MenuBar renders as " File Edit ..."
    >>> assert "╭" in output
    """
    app = TuiPrototype.with_sample_data()
    return app.render(width, height)
