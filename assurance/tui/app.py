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
    CommandPalette,
    ContentMarker,
    ContentPane,
    Dialog,
    ExplorerPane,
    FindBar,
    MenuBar,
    PropertiesSheet,
    StatusBar,
    Toolbar,
    box_bottom,
    box_horizontal,
    box_t_junction,
)


# Layout constants
EXPLORER_WIDTH = 22   # fixed-width explorer pane (20 + 2 borders)
MARKER_WIDTH = 16     # right-side content marker (14 + 2 borders)
FIXED_HEIGHT = 1      # menu, toolbar, address, find, status bars
TOTAL_FIXED_ROWS = 5  # menu + toolbar + address + find + status


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

    # ── event-driven state (Phase 1) ──
    event_source: Any | None = None   # EventSource | None (typed Any to avoid circular import)
    _event_log: list[Any] = field(default_factory=list)  # list[TuiEvent]
    _status_messages: list[str] = field(default_factory=list)

    # ── retrieval handler (LBR-001) ──
    # Callable[[str, str, Callable[[RetrievalProgress], None]], RetrievalOutcome] | None
    # Wired at the application boundary (main.py) to avoid assurance imports in TUI.
    retrieval_handler: Any | None = None

    # ── runtime state ──
    running: bool = False       # True while agent is executing a run
    _last_sent: str = ""        # preserved when run is cancelled (refill bar)

    # ── focus ──
    _focusable_panes: tuple[str, ...] = (
        "explorer", "content", "marker", "address", "find",
    )
    _active_pane_index: int = 0

    # ── public API ──────────────────────────────────────────────────────────

    def render(self, width: int, height: int) -> str:
        """Render the full TUI screen at *width* × *height*."""
        if width < 80 or height < 24:
            return f"Viewport too small ({width}x{height}). Minimum 80x24 required."

        inner_w = width - 2  # inside │ borders
        lines: list[str] = []

        # ── top border ──
        lines.append(box_horizontal("", width))

        # ── Row 0: MenuBar ──
        menu = self.menu_bar.render(inner_w, 1)
        lines.append("│" + menu[0] + "│")
        lines.append(box_t_junction(width))

        # ── Row 1: Toolbar ──
        tool = self.toolbar.render(inner_w, 1)
        lines.append("│" + tool[0] + "│")
        lines.append(box_t_junction(width))

        # ── Row 2: AddressBar (variable height for multi-line input) ──
        addr_lines_count = self.address_bar._line_count
        addr = self.address_bar.render(inner_w, addr_lines_count)
        for i, addr_line in enumerate(addr):
            lines.append("│" + addr_line + "│")
        # Separator below the last address bar row
        lines.append(box_t_junction(width))

        # ── Row 3: FindBar ──
        find = self.find_bar.render(inner_w, 1)
        lines.append("│" + find[0] + "│")

        # ── explorer / content / marker 3-column split ──
        exp_w = EXPLORER_WIDTH - 2   # inner explorer width
        marker_w = MARKER_WIDTH - 2  # inner marker width
        content_w = inner_w - exp_w - marker_w - 2  # minus two │ separators
        sep_line = (
            "├" + "─" * exp_w + "┬" + "─" * content_w + "┬" + "─" * marker_w + "┤"
        )
        lines.append(sep_line)

        # ── Body: ExplorerPane | ContentPane | ContentMarker ──
        # Account for extra address bar rows beyond the first
        extra_addr_rows = addr_lines_count - 1
        body_height = height - TOTAL_FIXED_ROWS - 5 - extra_addr_rows
        if body_height < 3:
            body_height = 3

        explorer_inner = self._render_pane_inner(
            self.explorer_pane, exp_w, body_height
        )
        content_inner = self._render_pane_inner(
            self.content_pane, content_w, body_height
        )
        marker_inner = self._render_pane_inner(
            self.content_marker, marker_w, body_height
        )

        for i in range(body_height):
            el = explorer_inner[i] if i < len(explorer_inner) else " " * exp_w
            cl = content_inner[i] if i < len(content_inner) else " " * content_w
            ml = marker_inner[i] if i < len(marker_inner) else " " * marker_w
            lines.append(
                "│" + pad_to_width(el, exp_w)
                + "│" + pad_to_width(cl, content_w)
                + "│" + pad_to_width(ml, marker_w)
                + "│"
            )

        # ── body bottom separator ──
        bot_sep = (
            "├" + "─" * exp_w + "┴" + "─" * content_w + "┴" + "─" * marker_w + "┤"
        )
        lines.append(bot_sep)

        # ── StatusBar (dynamic: IDLE ↔ RUNNING) ──
        # Update the last status item to reflect current run state
        if self.status_bar.items and self.status_bar.items[-1][0] in ("IDLE", "RUNNING"):
            fixed = list(self.status_bar.items)
            fixed[-1] = ("RUNNING", True) if self.running else ("IDLE", True)
            self.status_bar.items = fixed
        status = self.status_bar.render(inner_w, 1)
        lines.append("│" + status[0] + "│")

        # ── bottom border ──
        lines.append(box_bottom(width))

        result = "\n".join(lines)

        # Overlays (command palette → dialog → properties, lowest priority first)
        if (
            self.address_bar.focused
            and self.address_bar._show_autocomplete
        ):
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
        # Dialogs/properties consume all keys while visible
        if self.dialog.visible:
            consumed = self.dialog.handle_key(key)
            if key == "enter" and self.dialog.visible:
                # User confirmed via Enter on the selected action
                action_idx = self.dialog._selected_action
                if action_idx < len(self.dialog.actions):
                    action_value = self.dialog.actions[action_idx][1]
                    self.dialog.visible = False
                    if action_value == "approve":
                        self._execute_retrieval()
                        return "Retrieval started"
                    return "Cancelled"
            if key == "esc":
                self.dialog.visible = False
                return "Cancelled"
            return None
        if self.properties.visible:
            self.properties.handle_key(key)
            return None

        # Menu activation
        if key.startswith("alt+"):
            self.menu_bar.handle_key(key)
            return None

        if key == "esc":
            # 1. If address bar has auto-complete or buffer, clear it
            if self.address_bar.focused and (
                self.address_bar._show_autocomplete or self.address_bar._buffer
            ):
                self.address_bar.handle_key("esc")
                return None
            # 2. If menu is open, close it
            if self.menu_bar.active_menu:
                self.menu_bar.handle_key("esc")
                return None
            return None

        if key == "backspace":
            # If address bar has buffer, delete last character
            if self.active_pane == "address" and self.address_bar._buffer:
                self.address_bar.handle_key("backspace")
                return None
            return None

        if key == "c-z":
            # If agent is running, cancel the current run (retract sent input).
            # Ctrl+Z = universal undo/retract, works on all keyboard form factors.
            if self.running:
                self._cancel_run()
                return "已取消当前运行"
            return None

        if key == "f5":
            return self._toggle_run()

        if key == "f6":
            return self._cycle_focus()

        # Printable characters + editing keys → route to focused address bar.
        if self.active_pane == "address":
            # Save buffer BEFORE routing (widget may clear it on Enter)
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
            "content": self.content_pane,
            "marker": self.content_marker,
            "address": self.address_bar,
            "find": self.find_bar,
        }
        return mapping.get(self.active_pane)

    def _dispatch_command(self, text: str) -> str | None:
        """Route a slash command or URI entered in the address bar."""
        text = text.strip()

        # /search <query> — browser-based web search
        if text.startswith("/search "):
            query = text[len("/search "):].strip()
            if not query:
                return "Usage: /search <query>"
            self._show_retrieval_dialog("search", query)
            return f"Search: {query}"

        # /retrieve <url> — browser-based paper retrieval
        if text.startswith("/retrieve "):
            url = text[len("/retrieve "):].strip()
            if not url:
                return "Usage: /retrieve <url>"
            self._show_retrieval_dialog("retrieve", url)
            return f"Retrieve: {url}"

        # Other slash commands — just acknowledge
        if text.startswith("/"):
            return f"Activated: {text}"

        # URI navigation
        return f"Go to: {text}"

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
            self.status_bar.update_item("IDLE", True)
            self.content_pane.set_disposition(
                "NO HANDLER",
                "Retrieval handler not wired. Run from main.py for live retrieval.",
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
                "searching": "SEARCHING", "navigating": "FETCHING",
                "reading": "READING", "downloading": "DOWNLOADING",
                "storing": "STORING", "indexing": "INDEXING",
                "done": "IDLE", "failed": "ERROR",
                "launching": "LAUNCHING", "connecting": "CONNECTING",
                "finding_pdf": "FIND PDF", "validating": "VALIDATING",
                "retrieving": "RETRIEVING",
            }
            label = stage_labels.get(evt.stage, evt.stage.upper())
            self.status_bar.update_item(label, evt.stage != "failed")

        def _run() -> None:
            try:
                outcome: RetrievalOutcome = self.retrieval_handler(
                    mode, target, _on_progress,
                )
                if outcome.ok:
                    self.status_bar.update_item("IDLE", True)
                    self.content_pane.set_disposition(outcome.title, outcome.detail)
                else:
                    self.status_bar.update_item("ERROR", False)
                    self.content_pane.set_disposition(
                        outcome.title or "FAILED",
                        outcome.error or outcome.detail,
                    )
            except Exception as exc:
                self._status_messages.append(f"ERROR: {exc}")
                self.status_bar.update_item("ERROR", False)

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
        """Build a prototype wired to *source* with empty initial state.

        The returned prototype has no sample data — event groups are
        created lazily on the first projected event.
        """
        app = cls()
        app.event_source = source
        app._ensure_event_groups()
        return app

    # ── convenience builders ────────────────────────────────────────────────

    @classmethod
    def with_sample_data(cls) -> TuiPrototype:
        """Build a prototype populated with hard-coded sample data."""
        return cls(
            menu_bar=MenuBar(menus=dict(SAMPLE_MENUS)),
            toolbar=Toolbar(buttons=[
                "Back", "Forward", "Stop", "Refresh", "Open", "Verify", "Properties",
            ]),
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
                title="Current Task",
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


# ── helper ──────────────────────────────────────────────────────────────────


def pad_to_width(text: str, width: int) -> str:
    """Pad *text* to exactly *width* characters."""
    if len(text) >= width:
        return text[:width]
    return text + " " * (width - len(text))


# ── convenience entry point ─────────────────────────────────────────────────


def render_screen(width: int = 100, height: int = 30) -> str:
    """Render a full-screen TUI prototype with sample data.

    >>> output = render_screen()
    >>> assert "MenuBar" not in output  # MenuBar renders as " File Edit ..."
    >>> assert "╭" in output
    """
    app = TuiPrototype.with_sample_data()
    return app.render(width, height)
