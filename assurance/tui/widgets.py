"""Retro desktop widget layer — GAK-UI-001.

Every widget renders to a plain list of strings (no ANSI escapes,
no terminal control codes — the compositor handles presentation).

All box-drawing uses Unicode box-drawing characters (U+2500–U+257F).
Chinese / CJK characters are assumed to be 2 cells wide; ASCII and
box-drawing characters are 1 cell wide.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Sequence

# ── cell-width utilities ────────────────────────────────────────────────────

# Unicode ranges whose characters occupy 2 terminal cells.
_CJK_RANGES: tuple[tuple[int, int], ...] = (
    (0x1100, 0x115F),   # Hangul Jamo
    (0x2E80, 0xA4CF),   # CJK Radicals … Yi
    (0xAC00, 0xD7A3),   # Hangul Syllables
    (0xF900, 0xFAFF),   # CJK Compatibility Ideographs
    (0xFE30, 0xFE4F),   # CJK Compatibility Forms
    (0xFF01, 0xFF60),   # Fullwidth Forms
    (0xFFE0, 0xFFE6),   # Fullwidth Signs
    (0x1F300, 0x1F64F), # Misc Symbols (emoji; conservative 2-wide)
    (0x1F680, 0x1F6FF), # Transport (emoji)
    (0x1F900, 0x1F9FF), # Supplemental Symbols (emoji)
    (0x20000, 0x2FFFD), # CJK Unified Ideographs Extension B
    (0x30000, 0x3FFFD), # CJK Unified Ideographs Extension G
)


def cell_width(char: str) -> int:
    """Return the terminal cell width of *char* (1 or 2)."""
    cp = ord(char)
    for lo, hi in _CJK_RANGES:
        if lo <= cp <= hi:
            return 2
    return 1


def display_width(text: str) -> int:
    """Return the display width of *text* in terminal cells."""
    return sum(cell_width(ch) for ch in text)


def pad_to_width(text: str, width: int, align: str = "left") -> str:
    """Pad *text* to exactly *width* display cells."""
    current = display_width(text)
    if current >= width:
        return text[:width]  # truncation; caller should avoid this
    gap = width - current
    if align == "right":
        return " " * gap + text
    elif align == "center":
        left = gap // 2
        right = gap - left
        return " " * left + text + " " * right
    return text + " " * gap


# ── box-drawing helpers ─────────────────────────────────────────────────────

def box_horizontal(title: str, width: int, *, focused: bool = False) -> str:
    """Top border: ``╭─ title ──────────────╮`` or ``╒═ title ═══╕`` when focused."""
    lc = "╒" if focused else "╭"
    rc = "╕" if focused else "╮"
    fill = "═" if focused else "─"
    inner = width - 2  # left + right corners
    if title:
        title_str = f" {title} "
        if display_width(title_str) <= inner:
            title_str = fill + title_str + fill * (inner - display_width(title_str) - 1)
        else:
            title_str = title_str[:inner]
    else:
        title_str = fill * inner
    return lc + title_str + rc


def box_bottom(width: int, *, focused: bool = False) -> str:
    """Bottom border: ``╰──────────────────╯`` or ``╘══════╛`` when focused."""
    lc = "╘" if focused else "╰"
    rc = "╛" if focused else "╯"
    fill = "═" if focused else "─"
    return lc + fill * (width - 2) + rc


def box_vertical(lines: list[str], width: int, *, focused: bool = False) -> list[str]:
    """Wrap each line between ``│`` / ``║`` borders."""
    edge = "║" if focused else "│"
    return [edge + pad_to_width(line, width - 2) + edge for line in lines]


def box_t_junction(width: int, *, focused: bool = False) -> str:
    """T-junction: ``├──────────────────┤`` or ``╞══════╡`` when focused."""
    lc = "╞" if focused else "├"
    rc = "╡" if focused else "┤"
    fill = "═" if focused else "─"
    return lc + fill * (width - 2) + rc


def box_double_horizontal(width: int) -> str:
    """Double-line separator: ``╞══════════════╡``."""
    return "╞" + "═" * (width - 2) + "╡"


# ── widget base ─────────────────────────────────────────────────────────────


@dataclass
class Widget:
    """Base widget.  ``render`` returns lines filling (width x height)."""
    focusable: bool = False
    focused: bool = False

    def render(self, width: int, height: int) -> list[str]:
        raise NotImplementedError

    def handle_key(self, key: str) -> bool:
        """Return True if the key was consumed."""
        return False


# ── MenuBar ─────────────────────────────────────────────────────────────────


@dataclass
class MenuBar(Widget):
    menus: dict[str, list[str]] = field(default_factory=dict)
    active_menu: str = ""
    focusable: bool = True
    # Access key mapping: "alt+<letter>" → menu key (§10 keyboard model)
    _access_keys: dict[str, str] = field(default_factory=dict)

    # Default Chinese access key mapping (F=文件, S=事件, B=标记, E=编辑模式, M=模型, O=来源, R=运行, V=验证, H=帮助)
    DEFAULT_ACCESS_KEYS: dict[str, str] = field(default_factory=lambda: {
        "f": "文件", "s": "事件", "b": "标记", "e": "编辑模式",
        "m": "模型", "o": "来源", "r": "运行", "v": "验证", "h": "帮助",
    })

    def _resolve_access(self, letter: str) -> str | None:
        """Find the menu key for an access letter."""
        lower = letter.lower()
        # Try the explicit mapping first (only if the target exists in menus)
        if lower in self._access_keys:
            target = self._access_keys[lower]
            if target in self.menus:
                return target
        # Try the default Chinese mapping (only if the target exists in menus)
        if lower in self.DEFAULT_ACCESS_KEYS:
            target = self.DEFAULT_ACCESS_KEYS[lower]
            if target in self.menus:
                return target
        # Try first-letter match (for backwards compatibility with English menus)
        for name in self.menus:
            if name.upper().startswith(letter):
                return name
        return None

    def render(self, width: int, height: int) -> list[str]:
        if height < 1:
            return []
        names = list(self.menus.keys())
        if not names:
            return [" " * width]

        label_chunks: list[str] = []
        for name in names:
            marker = " " if (self.focused or self.active_menu) else " "
            label_chunks.append(f"{marker}{name}")

        # Build the menu bar line
        line = "  ".join(label_chunks)

        # If a menu is active, show its dropdown items
        if self.active_menu and self.active_menu in self.menus:
            return self._render_with_dropdown(width, height)
        return [pad_to_width(line, width)]

    def _render_with_dropdown(self, width: int, height: int) -> list[str]:
        """Render menu bar line + dropdown overlay."""
        result: list[str] = []
        # Menu bar line
        names = list(self.menus.keys())
        chunks = []
        for name in names:
            if name == self.active_menu:
                chunks.append(f"▼{name}")
            else:
                chunks.append(f" {name}")
        menu_line = "  ".join(chunks)
        result.append(pad_to_width(menu_line, width))

        # Dropdown items
        items = self.menus.get(self.active_menu, [])
        max_item_w = max(display_width(it) for it in items) if items else 10
        dropdown_w = max_item_w + 4  # padding + border
        for i, item in enumerate(items):
            prefix = "▸ " if i == 0 else "   "
            result.append(edge() + " " + prefix + item + " " * (dropdown_w - display_width(item) - len(prefix) - 3) + edge())
        # Bottom of dropdown
        if items:
            result.append(edge() + "-" * (dropdown_w - 2) + edge())

        # Pad all dropdown rows to full width
        return [pad_to_width(r, width) for r in result]

    def handle_key(self, key: str) -> bool:
        if key.startswith("alt+"):
            letter = key[4:].upper()
            name = self._resolve_access(letter)
            if name is not None and name in self.menus:
                self.active_menu = name
                return True
        if key == "esc" and self.active_menu:
            self.active_menu = ""
            return True
        return False


# ── Toolbar ─────────────────────────────────────────────────────────────────


@dataclass
class Toolbar(Widget):
    buttons: list[str] = field(default_factory=list)
    focusable: bool = True

    def render(self, width: int, height: int) -> list[str]:
        if height < 1:
            return []
        if not self.buttons:
            return [" " * width]
        sep = "  "
        line = sep.join(self.buttons)
        if self.focused:
            line = "[" + line + "]"
        return [pad_to_width(line, width)]


# ── AddressBar ──────────────────────────────────────────────────────────────


@dataclass
class AddressBar(Widget):
    """Address bar with text input, slash-command auto-complete, command
    history, and multi-line support.

    - Printable characters accumulate in an internal buffer.
    - ``/`` opens the command palette; prefix filtering as the user types.
    - ``↑`` / ``↓`` navigate command history when the buffer is empty and
      auto-complete is not active.
    - ``Shift+Enter`` inserts a newline into the buffer; the bar grows
      vertically to show all lines.  ``Enter`` sends the full content.
    """

    uri: str = ""
    focusable: bool = True
    _buffer: str = ""
    _show_autocomplete: bool = False
    _selected_index: int = 0

    # ── command history ──────────────────────────────────────────────────
    _history: list[str] = field(default_factory=list)
    _history_index: int = -1        # -1 = not navigating history
    _history_draft: str = ""        # saved draft while browsing history
    _browsing_history: bool = False  # True while ↑↓ has loaded a history entry

    def _push_history(self, text: str) -> None:
        """Record *text* in the command history (called externally on send)."""
        if text and (not self._history or self._history[-1] != text):
            self._history.append(text)
        self._history_index = -1
        self._history_draft = ""

    # ── line count helper ────────────────────────────────────────────────

    @property
    def _line_count(self) -> int:
        """Number of display rows: capped at 3, minimum 1 when not focused."""
        if not self.focused or not self._buffer:
            return 1
        total = self._buffer.count("\n") + 1
        return min(total, 3)

    def _current_line(self) -> str:
        """Return the text after the last newline (the line being edited)."""
        if "\n" not in self._buffer:
            return self._buffer
        return self._buffer.rsplit("\n", 1)[-1]

    # ── render ───────────────────────────────────────────────────────────

    def render(self, width: int, height: int) -> list[str]:
        if height < 1:
            return []
        addr_label = "Address: "
        go_button = "[ Go ]"
        available = width - display_width(addr_label) - display_width(go_button) - 2
        if available < 10:
            available = 10

        if not self.focused or not self._buffer:
            # Static URI display
            uri_display = self.uri if not self.focused else (
                self._buffer or self.uri
            )
            if self.focused:
                uri_display = (self._buffer or self.uri) + "█"
                if display_width(uri_display) > available:
                    uri_display = "…" + uri_display[-(available - 4):]
            else:
                if display_width(uri_display) > available:
                    uri_display = uri_display[:available - 3] + "..."
            filler = " " * max(0, available - display_width(uri_display))
            return [pad_to_width(addr_label + uri_display + filler + go_button, width)]

        # Multi-line buffer: show last N lines (capped at 3 visible rows).
        # Older lines scroll off the top and are only in the buffer.
        all_lines = self._buffer.split("\n")
        shown = all_lines[-3:]  # last 3 lines
        result: list[str] = []
        for i, line in enumerate(shown):
            is_last = (i == len(shown) - 1)
            if is_last and self.focused:
                line = line + "█"
            if display_width(line) > available:
                if is_last:
                    line = "…" + line[-(available - 4):]
                else:
                    line = line[:available - 3] + "..."
            filler = " " * max(0, available - display_width(line))
            result.append(pad_to_width(addr_label + line + filler + go_button, width))
        return result

    # ── input handling ───────────────────────────────────────────────────

    def handle_key(self, key: str) -> bool:
        if not self.focused:
            return False

        # -- history navigation (empty buffer, or already browsing) ------
        if key == "up" and not self._show_autocomplete:
            if not self._buffer or self._browsing_history:
                return self._history_up()
        if key == "down" and not self._show_autocomplete:
            if not self._buffer or self._browsing_history:
                return self._history_down()

        # -- auto-complete navigation (only when palette is active) -----
        if key == "up" and self._show_autocomplete and self._buffer:
            self._selected_index -= 1
            return True
        if key == "down" and self._show_autocomplete and self._buffer:
            self._selected_index += 1
            return True

        # -- printable characters -----------------------------------------
        if len(key) == 1 and key.isprintable() and key not in ("\x1b", "\t", "\r", "\n"):
            self._buffer += key
            self._browsing_history = False
            self._show_autocomplete = self._buffer.startswith("/") and "\n" not in self._buffer
            self._selected_index = 0
            return True
        if key == "space":
            self._buffer += " "
            self._show_autocomplete = False
            return True

        # -- editing -------------------------------------------------------
        if key == "backspace":
            if self._buffer:
                self._buffer = self._buffer[:-1]
            self._show_autocomplete = self._buffer.startswith("/") and "\n" not in self._buffer
            self._selected_index = 0
            return True

        # -- newline (Shift+Enter) -----------------------------------------
        if key == "s-enter" or key == "\n":
            self._buffer += "\n"
            self._show_autocomplete = False
            return True

        # -- send (Enter) --------------------------------------------------
        if key == "enter":
            if self._show_autocomplete:
                self._buffer = ""
                self._show_autocomplete = False
                self._selected_index = 0
                return True
            return True  # caller reads _buffer and sends

        # -- dismiss (Esc) ------------------------------------------------
        if key == "esc":
            self._buffer = ""
            self._show_autocomplete = False
            self._selected_index = 0
            return True

        # -- tab-complete (palette) ----------------------------------------
        if key == "tab" and self._show_autocomplete:
            from .commands import get_builtin_registry
            registry = get_builtin_registry()
            matches = registry.search(self._buffer)
            if matches:
                idx = max(0, min(self._selected_index, len(matches) - 1))
                self._buffer = matches[idx].slash
                self._selected_index = 0
            return True

        return False

    # ── history internals ────────────────────────────────────────────────

    def _history_up(self) -> bool:
        if not self._history:
            return False
        if self._history_index == -1:
            self._history_draft = self._buffer  # save draft before browsing
            self._history_index = len(self._history) - 1
        elif self._history_index > 0:
            self._history_index -= 1
        else:
            return True  # at oldest entry
        self._buffer = self._history[self._history_index]
        self._show_autocomplete = False
        self._browsing_history = True
        return True

    def _history_down(self) -> bool:
        if self._history_index == -1:
            return False
        if self._history_index < len(self._history) - 1:
            self._history_index += 1
            self._buffer = self._history[self._history_index]
        else:
            self._buffer = self._history_draft
            self._history_index = -1
            self._history_draft = ""
            self._browsing_history = False
        self._show_autocomplete = False
        return True

    # ── autocomplete query ───────────────────────────────────────────────

    @property
    def autocomplete_candidates(self) -> list:
        """Return the current set of autocomplete matches (for the palette)."""
        if not self._show_autocomplete:
            return []
        from .commands import get_builtin_registry
        registry = get_builtin_registry()
        return registry.search(self._buffer)


# ── CommandPalette (slash-command auto-complete dropdown) ───────────────────


@dataclass
class CommandPalette(Widget):
    """Auto-complete dropdown for slash-commands.

    Renders as a bordered overlay listing matching commands with their
    Chinese name and description.  Call :meth:`render_overlay` to produce
    a rectangle that the compositor blends on top of the main screen.
    """

    candidates: list = field(default_factory=list)
    selected_index: int = 0
    visible: bool = False

    # ── helpers ────────────────────────────────────────────────────────────

    @staticmethod
    def _cmd_width(cmd) -> int:
        """Display width of one palette row: ``▸ /new  新对话  创建新的...``"""
        return display_width(cmd.slash) + display_width(cmd.name_zh) + 4

    # ── overlay render ─────────────────────────────────────────────────────

    def render_overlay(self, screen_w: int, screen_h: int) -> list[str] | None:
        """Return overlay lines for the command palette, positioned just
        below the address bar (roughly row 5 in the standard layout).

        Returns ``None`` when ``visible`` is ``False`` or there are no
        candidates.
        """
        if not self.visible or not self.candidates:
            return None

        # Clamp selected index
        idx = max(0, min(self.selected_index, len(self.candidates) - 1))

        # Calculate palette dimensions
        max_cmd_w = max(self._cmd_width(c) for c in self.candidates)
        # Add room for description — cap at a reasonable width
        palette_w = max_cmd_w + 18  # padding + borders + description preview
        palette_w = max(palette_w, 24)
        palette_w = min(palette_w, screen_w - 4)

        inner_w = palette_w - 2
        title = " 指令 "

        result: list[str] = []
        # Top border
        result.append(box_horizontal(title, palette_w, focused=True))
        # Candidate rows
        for i, cmd in enumerate(self.candidates):
            is_selected = bool(i == idx)
            marker = "▸ " if is_selected else "  "
            desc = cmd.description_zh
            # Truncate description to fit
            row_content = f"{marker}{cmd.slash}  {cmd.name_zh}"
            row_w = display_width(row_content)
            desc_avail = inner_w - row_w - 1
            if desc_avail > 8:
                if display_width(desc) > desc_avail:
                    desc = desc[:desc_avail - 1] + "…"
                row_content = row_content + "  " + desc
            if is_selected:
                row_content = "\033[7m" + row_content + "\033[0m"
            result.append(edge() + pad_to_width(row_content, inner_w) + edge())
        # Bottom
        result.append(box_bottom(palette_w, focused=True))

        # Position: row 5 (right below the AddressBar row in the TUI layout)
        top_offset = 4
        left_offset = 4  # align under "Address: " prefix

        # Build overlay as screen-sized transparent lines
        overlay: list[str] = []
        for i in range(screen_h):
            if i >= top_offset and i < top_offset + len(result):
                line = result[i - top_offset]
                # left-pad
                overlay.append(" " * left_offset + pad_to_width(line, palette_w) + " " * (screen_w - left_offset - palette_w))
            else:
                overlay.append(" " * screen_w)
        return overlay[:screen_h]


# ── FindBar ─────────────────────────────────────────────────────────────────


@dataclass
class FindBar(Widget):
    query: str = ""
    scope: str = "Conversation"
    scopes: list[str] = field(default_factory=list)
    focusable: bool = True
    _scope_open: bool = False

    def render(self, width: int, height: int) -> list[str]:
        if height < 1:
            return []
        inner_w = width  # already inside │ borders from compositor
        scope_text = f"in [ {self.scope} v ]"
        find_label = "Find: [ "
        available = (
            inner_w
            - display_width(find_label)
            - display_width(scope_text)
            - 3  # closing ] + spacing
        )
        if available < 8:
            available = 8
        query_display = self.query
        if display_width(query_display) > available:
            query_display = query_display[: available - 3] + "..."
        filler = " " * (available - display_width(query_display))
        line = find_label + query_display + filler + " ] " + scope_text
        return [pad_to_width(line, inner_w)]

    def handle_key(self, key: str) -> bool:
        if key == "tab" and self.focused:
            self._scope_open = not self._scope_open
            return True
        if key in ("up", "down") and self._scope_open:
            idx = self.scopes.index(self.scope) if self.scope in self.scopes else 0
            if key == "down":
                idx = (idx + 1) % len(self.scopes)
            else:
                idx = (idx - 1) % len(self.scopes)
            self.scope = self.scopes[idx]
            return True
        if key == "enter" and self._scope_open:
            self._scope_open = False
            return True
        if key == "esc" and self._scope_open:
            self._scope_open = False
            return True
        return False


# ── ExplorerPane ────────────────────────────────────────────────────────────


@dataclass
class ExplorerPane(Widget):
    tree: list[Any] = field(default_factory=list)  # list[TreeNode]
    event_groups: list[Any] = field(default_factory=list)  # list[EventGroup]
    focusable: bool = True

    def render(self, width: int, height: int) -> list[str]:
        inner_w = width - 2
        lines: list[str] = []
        # Tree section
        lines.extend(self._render_tree(self.tree, inner_w, "", 0))
        # Separator between tree and events
        if self.tree and self.event_groups:
            lines.append("-" * inner_w)
        # Event groups
        lines.extend(self._render_events(self.event_groups, inner_w))
        # Truncate/pad
        if len(lines) < height:
            lines += [" " * inner_w] * (height - len(lines))
        else:
            lines = lines[:height]
        return box_vertical(lines, width, focused=self.focused)

    def _render_tree(
        self, nodes: list[Any], width: int, indent: str, depth: int
    ) -> list[str]:
        result: list[str] = []
        for node in nodes:
            marker = "▾ " if node.expanded else "▸ "
            line = indent + marker + node.label
            result.append(pad_to_width(line, width))
            if node.expanded and node.children:
                child_lines = self._render_tree(
                    node.children, width, indent + "  ", depth + 1
                )
                result.extend(child_lines)
        return result

    def _render_events(self, groups: list[Any], width: int) -> list[str]:
        result: list[str] = []
        if not groups:
            return result
        result.append(pad_to_width("Events", width))
        for g in groups:
            marker = "▾ " if g.expanded else "▸ "
            line = "  " + marker + f"{g.label} ({g.count})"
            result.append(pad_to_width(line, width))
            if g.expanded and g.entries:
                for entry in g.entries:
                    result.append(pad_to_width("    " + entry[: width - 6], width))
        return result

    # ── mutation helpers (event-driven updates) ───────────────────────────

    def add_event_entry(self, group_label: str, entry_text: str) -> None:
        """Append *entry_text* to the event group named *group_label*.

        Creates the group if it does not already exist.
        """
        for g in self.event_groups:
            if g.label == group_label:
                g.entries.append(entry_text)
                g.count = len(g.entries)
                return
        from .view_models import EventGroup
        self.event_groups.append(EventGroup(
            label=group_label, count=1, expanded=True,
            entries=[entry_text],
        ))

    def add_event_group(self, group_label: str) -> None:
        """Ensure an event group named *group_label* exists (no-op if present)."""
        for g in self.event_groups:
            if g.label == group_label:
                return
        from .view_models import EventGroup
        self.event_groups.append(EventGroup(
            label=group_label, count=0, expanded=False, entries=[],
        ))


# ── ContentPane ─────────────────────────────────────────────────────────────


@dataclass
class ContentPane(Widget):
    title: str = "Current Task"
    source_table: list[Any] = field(default_factory=list)  # list[SourceVisibilityRow]
    claim_disposition: str = ""
    disposition_reason: str = ""
    next_actions: list[str] = field(default_factory=list)
    focusable: bool = True

    def render(self, width: int, height: int) -> list[str]:
        inner_w = width - 2
        lines: list[str] = []

        # Title
        lines.append(pad_to_width(self.title, inner_w, align="center"))
        lines.append("-" * inner_w)
        lines.append("")

        # Source Visibility table
        lines.append("  Source Visibility")
        lines.append("  " + "-" * (inner_w - 4))
        if self.source_table:
            # Header
            header = "  {:<14} {:<10} {:<10} {:<8} {}".format(
                "Source", "Observed", "Required", "Decision", "Claim"
            )
            lines.append(header[:inner_w])
            lines.append("  " + "-" * (min(inner_w - 4, 54)))
            for row in self.source_table:
                rline = "  {:<14} {:<10} {:<10} {:<8} {}".format(
                    row.ref_id[:12],
                    row.observed[:8],
                    row.required[:8],
                    row.decision[:6],
                    row.claim[:10],
                )
                lines.append(rline[:inner_w])
        else:
            lines.append("  (no sources loaded)")
        lines.append("")

        # Claim disposition
        lines.append(f"  Claim disposition: {self.claim_disposition}")
        if self.disposition_reason:
            lines.append(f"  {self.disposition_reason}")
        lines.append("")

        # Next actions
        if self.next_actions:
            lines.append("  Next Actions")
            lines.append("  " + "-" * (inner_w - 4))
            for i, action in enumerate(self.next_actions, 1):
                lines.append(f"  {i}. {action}")

        # Pad to height
        if len(lines) < height:
            lines += [""] * (height - len(lines))
        return box_vertical(lines[:height], width, focused=self.focused)

    # ── mutation helpers (event-driven updates) ───────────────────────────

    def add_or_update_source_row(
        self, ref_id: str, observed: str, required: str,
        decision: str, claim: str,
    ) -> None:
        """Insert or update a source-visibility row identified by *ref_id*."""
        for row in self.source_table:
            if row.ref_id == ref_id:
                row.observed = observed
                row.required = required
                row.decision = decision
                row.claim = claim
                return
        from .view_models import SourceVisibilityRow
        self.source_table.append(SourceVisibilityRow(
            ref_id=ref_id, observed=observed, required=required,
            decision=decision, claim=claim,
        ))

    def set_disposition(self, disposition: str, reason: str = "") -> None:
        """Set the claim disposition and optional reason text."""
        self.claim_disposition = disposition
        if reason:
            self.disposition_reason = reason

    def set_next_actions(self, actions: list[str]) -> None:
        """Replace the next-actions list."""
        self.next_actions = list(actions)


# ── StatusBar ───────────────────────────────────────────────────────────────


@dataclass
class StatusBar(Widget):
    items: list[tuple[str, bool]] = field(default_factory=list)

    def render(self, width: int, height: int) -> list[str]:
        if height < 1:
            return []
        if not self.items:
            return [" " * width]
        chunks: list[str] = []
        for label, _ok in self.items:
            chunks.append(label)
        line = " | ".join(chunks)
        return [pad_to_width(line, width)]

    # ── mutation helpers (event-driven updates) ───────────────────────────

    def update_item(self, label: str, ok: bool) -> None:
        """Update or append a status item.

        Items are matched by their label text (exact match).  If no item
        with *label* exists, the pair ``(label, ok)`` is appended.
        """
        for i, (existing_label, _) in enumerate(self.items):
            if existing_label == label:
                self.items[i] = (label, ok)
                return
        self.items.append((label, ok))


# ── ContentMarker (right-side column) ──────────────────────────────────────


@dataclass
class ContentMarker(Widget):
    """Narrow right-side column showing conversation position markers.

    ▸ = user input position, ● = search hit position.
    The marker is simple by design: complex event categories belong
    in the ExplorerPane, not here.
    """
    markers: list[Any] = field(default_factory=list)  # list[MarkerEntry]

    def render(self, width: int, height: int) -> list[str]:
        inner_w = width - 2
        lines: list[str] = []
        lines.append(pad_to_width("Markers", inner_w, align="center"))
        lines.append("-" * inner_w)
        if not self.markers:
            lines.append(pad_to_width("(none)", inner_w))
        else:
            for m in self.markers:
                glyph = "●" if m.kind == "hit" else "▸"
                label = m.label[: inner_w - 8] if m.label else ""
                line_text = f" {glyph} L{m.line:<4}"
                if label:
                    line_text += label
                lines.append(pad_to_width(line_text, inner_w))
        # Pad
        if len(lines) < height:
            lines += [" " * inner_w] * (height - len(lines))
        return box_vertical(lines[:height], width)

    # ── mutation helpers (event-driven updates) ───────────────────────────

    def add_marker(self, kind: str, line: int, label: str = "") -> None:
        """Append a marker entry to the right-side marker column."""
        # Import locally to avoid circular dependency at module level
        from .view_models import MarkerEntry
        self.markers.append(MarkerEntry(kind=kind, line=line, label=label))


# ── Dialog (modal overlay) ──────────────────────────────────────────────────


@dataclass
class Dialog(Widget):
    title: str = "Dialog"
    message: str = ""
    actions: list[tuple[str, Any]] = field(default_factory=list)
    visible: bool = False
    _selected_action: int = 0

    def render_overlay(self, width: int, height: int) -> list[str] | None:
        """Return lines for a centered overlay, or None if not visible."""
        if not self.visible:
            return None

        msg_lines = self.message.split("\n")
        max_content_w = max(
            max(display_width(line) for line in msg_lines) if msg_lines else 10,
            display_width(self.title),
            40,
        )
        dialog_w = min(max_content_w + 6, width - 4)
        dialog_w = max(dialog_w, 20)

        result: list[str] = []
        # Top border
        result.append(box_horizontal(self.title, dialog_w, focused=self.focused))
        # Message
        for line in msg_lines:
            inner = dialog_w - 2
            if display_width(line) > inner:
                line = line[: inner - 3] + "..."
            result.append(edge() + pad_to_width(line, inner) + edge())
        # Separator
        result.append(box_t_junction(dialog_w))
        # Action buttons
        action_line = "  ".join(
            f"[{label}]" if i == self._selected_action else f" {label} "
            for i, (label, _) in enumerate(self.actions)
        )
        inner = dialog_w - 2
        result.append(edge() + pad_to_width(action_line, inner, align="center") + edge())
        # Bottom
        result.append(box_bottom(dialog_w, focused=self.focused))

        # Center vertically.  Padding uses spaces (transparent in blend);
        # the dialog background inside the border uses ░ to prevent
        # content bleed-through.
        BG = " "
        dialog_h = len(result)
        top_pad = max(0, (height - dialog_h) // 2)
        left_pad = max(0, (width - dialog_w) // 2)
        padded: list[str] = []
        for i in range(top_pad):
            padded.append(" " * width)
        for line in result:
            padded.append(" " * left_pad + pad_to_width(line, dialog_w) + " " * (width - left_pad - dialog_w))
        for i in range(height - len(padded)):
            padded.append(" " * width)
        return padded[:height]

    def handle_key(self, key: str) -> bool:
        if not self.visible:
            return False
        if key == "esc":
            self.visible = False
            return True
        if key == "tab":
            self._selected_action = (self._selected_action + 1) % max(1, len(self.actions))
            return True
        if key == "s-tab":
            self._selected_action = (self._selected_action - 1) % max(1, len(self.actions))
            return True
        if key == "enter":
            self.visible = False
            return True
        if key == "left":
            self._selected_action = max(0, self._selected_action - 1)
            return True
        if key == "right":
            self._selected_action = min(len(self.actions) - 1, self._selected_action + 1)
            return True
        return False


# ── PropertiesSheet (modal overlay) ─────────────────────────────────────────


@dataclass
class PropertiesSheet(Widget):
    title: str = "Properties"
    tabs: list[dict[str, Any]] = field(default_factory=list)
    visible: bool = False
    _active_tab: int = 0

    def render_overlay(self, width: int, height: int) -> list[str] | None:
        if not self.visible or not self.tabs:
            return None

        tab_names = [t["name"] for t in self.tabs]
        tab = self.tabs[self._active_tab]
        fields = tab.get("fields", [])

        # Calculate dialog dimensions
        max_field_w = max(
            max(display_width(k) + display_width(v) + 4 for k, v in fields) if fields else 10,
            display_width(self.title),
            sum(display_width(n) + 2 for n in tab_names) + 2,
            40,
        )
        prop_w = min(max_field_w + 4, width - 4)
        prop_w = max(prop_w, 24)
        inner_w = prop_w - 2

        result: list[str] = []
        # Top
        result.append(box_horizontal(self.title, prop_w, focused=self.focused))
        # Tab row
        tab_line = "  ".join(
            f"▶{name}" if i == self._active_tab else f"  {name}"
            for i, name in enumerate(tab_names)
        )
        result.append(edge() + pad_to_width(tab_line, inner_w) + edge())
        # Separator
        result.append(box_t_junction(prop_w))
        # Fields
        for k, v in fields:
            field_line = f"  {k}: {v}"
            if display_width(field_line) > inner_w:
                field_line = field_line[: inner_w - 3] + "..."
            result.append(edge() + pad_to_width(field_line, inner_w) + edge())
        # Bottom
        result.append(box_bottom(prop_w, focused=self.focused))

        # Center overlay with space padding (transparent in blend)
        BG = " "
        prop_h = len(result)
        top_pad = max(0, (height - prop_h) // 2)
        left_pad = max(0, (width - prop_w) // 2)
        padded: list[str] = []
        for i in range(top_pad):
            padded.append(BG * width)
        for line in result:
            padded.append(BG * left_pad + pad_to_width(line, prop_w) + BG * (width - left_pad - prop_w))
        for i in range(height - len(padded)):
            padded.append(BG * width)
        return padded[:height]

    def handle_key(self, key: str) -> bool:
        if not self.visible:
            return False
        if key == "esc":
            self.visible = False
            return True
        if key in ("tab", "right"):
            self._active_tab = (self._active_tab + 1) % len(self.tabs)
            return True
        if key in ("s-tab", "left"):
            self._active_tab = (self._active_tab - 1) % len(self.tabs)
            return True
        return False


# ── HelpOverlay (modal overlay, GAK-UI-001 P2) ────────────────────────────


@dataclass
class HelpOverlay(Widget):
    """Help modal overlay with tabbed pages (§12).

    Reuses the same overlay pattern as :class:`PropertiesSheet`.  Tabs are:
    快捷键 | 命令 | 模型 | 审批 | 终端 | 来源

    Keyboard: Esc close, Tab/Shift+Tab move focus, ←/→ switch tabs.
    """

    title: str = "帮助"
    tabs: list[dict[str, Any]] = field(default_factory=list)
    visible: bool = False
    _active_tab: int = 0

    @classmethod
    def with_defaults(cls) -> HelpOverlay:
        """Build a HelpOverlay pre-populated with reference content."""
        from .view_models import SAMPLE_COMMANDS, SAMPLE_SHORTCUTS

        shortcuts_content = "\n".join(
            f"  {key:<16} {desc}"
            for key, desc in SAMPLE_SHORTCUTS
        )

        commands_content = "\n".join(
            f"  {cmd.slash:<16} {cmd.name_zh:<10} {cmd.description_zh}"
            for cmd in SAMPLE_COMMANDS[:12]
        )

        return cls(
            tabs=[
                {
                    "name": "快捷键",
                    "fields": [],
                    "content": shortcuts_content,
                },
                {
                    "name": "命令",
                    "fields": [],
                    "content": commands_content,
                },
                {
                    "name": "模型",
                    "fields": [
                        ("当前适配器", "DeepSeek v4 Pro"),
                        ("可用模型", "deepseek-chat, deepseek-reasoner"),
                        ("推理强度", "low / medium / high / xhigh / max"),
                        ("Plan 模式", "/plan — 进入只读计划阶段"),
                    ],
                },
                {
                    "name": "审批",
                    "fields": [
                        ("Plan 审批", "计划完成后需用户审批再进入执行"),
                        ("Action 审批", "敏感操作可设为手动/自动/混合"),
                        ("审批策略", "manual / auto / mixed"),
                        ("Ctrl+Z", "取消当前运行，回填指令"),
                    ],
                },
                {
                    "name": "终端",
                    "fields": [
                        ("系统终端", "状态栏实时显示 CPU/MEM/时间"),
                        ("VS Code", "终端标题 2 秒刷新资源占用"),
                        ("标题格式", "CPU 185% MEM 2.1G 14m"),
                        ("异常提示", "HIGH CPU / HIGH MEM / IDLE?"),
                    ],
                },
                {
                    "name": "来源",
                    "fields": [
                        ("可见性等级", "FULL TEXT / PARTIAL / NONE"),
                        ("证据等级", "A (论文PDF) / B (网页) / C (元数据)"),
                        ("Gate 检查", "来源全文可见性门控"),
                        ("检索命令", "/search 网络搜索 / /retrieve 论文获取"),
                    ],
                },
            ],
        )

    def render_overlay(self, width: int, height: int) -> list[str] | None:
        if not self.visible or not self.tabs:
            return None

        tab_names = [t["name"] for t in self.tabs]
        tab = self.tabs[self._active_tab]

        # Calculate dimensions
        prop_w = min(max(width - 8, 36), 68)
        prop_w = max(prop_w, 28)
        inner_w = prop_w - 2

        result: list[str] = []
        # Top border
        result.append(box_horizontal(f" {self.title} ", prop_w, focused=True))
        # Tab row
        tab_line = "  ".join(
            f"▸{name}" if i == self._active_tab else f" {name} "
            for i, name in enumerate(tab_names)
        )
        result.append(edge() + pad_to_width(tab_line, inner_w) + edge())
        result.append(box_t_junction(prop_w))

        # Content: either "fields" (key-value) or "content" (free text)
        fields = tab.get("fields", [])
        content_text = tab.get("content", "")
        if content_text:
            for line in content_text.split("\n"):
                if display_width(line) > inner_w:
                    line = line[: inner_w - 3] + "…"
                result.append(edge() + pad_to_width(line, inner_w) + edge())
        elif fields:
            for k, v in fields:
                field_line = f"  {k}: {v}"
                if display_width(field_line) > inner_w:
                    field_line = field_line[: inner_w - 3] + "…"
                result.append(edge() + pad_to_width(field_line, inner_w) + edge())
        else:
            result.append(edge() + pad_to_width("（暂无内容）", inner_w) + edge())

        # Footer hint
        result.append(box_t_junction(prop_w))
        result.append(edge() + pad_to_width(" Esc 关闭  ←→ 切换标签  ", inner_w, align="center") + edge())
        # Bottom
        result.append(box_bottom(prop_w, focused=True))

        # Center overlay
        BG = " "
        prop_h = len(result)
        top_pad = max(1, (height - prop_h) // 2)
        left_pad = max(0, (width - prop_w) // 2)
        padded: list[str] = []
        for _ in range(top_pad):
            padded.append(BG * width)
        for line in result:
            padded.append(BG * left_pad + pad_to_width(line, prop_w) + BG * (width - left_pad - prop_w))
        for _ in range(height - len(padded)):
            padded.append(BG * width)
        return padded[:height]

    def handle_key(self, key: str) -> bool:
        if not self.visible:
            return False
        if key == "esc":
            self.visible = False
            return True
        if key in ("tab", "right"):
            self._active_tab = (self._active_tab + 1) % len(self.tabs)
            return True
        if key in ("s-tab", "left"):
            self._active_tab = (self._active_tab - 1) % len(self.tabs)
            return True
        return False


# ── FindDialog (modal overlay, GAK-UI-001 P2) ────────────────────────────────


@dataclass
class FindDialog(Widget):
    """Modal overlay for multi-line / advanced search (§6).

    Provides a larger input area for complex search queries, regex, scope
    selection, and case-sensitivity toggles.
    """

    query: str = ""
    scope: str = "对话"
    scopes: list[str] = field(default_factory=lambda: [
        "对话", "项目文档", "当前文件", "运行/制品", "索引来源",
    ])
    case_sensitive: bool = False
    use_regex: bool = False
    visible: bool = False
    _focus_row: int = 0  # 0=query, 1=scope, 2=case, 3=regex, 4=buttons
    _scope_idx: int = 0

    def render_overlay(self, width: int, height: int) -> list[str] | None:
        if not self.visible:
            return None

        dialog_w = min(max(width - 8, 32), 56)
        inner_w = dialog_w - 2

        result: list[str] = []
        result.append(box_horizontal(" 查找 ", dialog_w, focused=True))

        # Query input area
        q_prefix = "▸ " if self._focus_row == 0 else "  "
        q_text = self.query or "（输入查找内容）"
        q_line = f"{q_prefix}查找: {q_text}"
        result.append(edge() + pad_to_width(q_line, inner_w) + edge())

        # Scope
        s_prefix = "▸ " if self._focus_row == 1 else "  "
        s_line = f"{s_prefix}范围: {self.scope}"
        result.append(edge() + pad_to_width(s_line, inner_w) + edge())

        # Options row
        case_mark = "▸ " if self._focus_row == 2 else "  "
        regex_mark = "▸ " if self._focus_row == 3 else "  "
        opts_line = (
            f"{case_mark}大小写: {'是' if self.case_sensitive else '否'}  "
            f"{regex_mark}正则: {'是' if self.use_regex else '否'}"
        )
        result.append(edge() + pad_to_width(opts_line, inner_w) + edge())

        # Separator
        result.append(box_t_junction(dialog_w))

        # Action buttons
        btn_line = "  [ 查找 ]    [ 取消 ]"
        if self._focus_row == 4:
            btn_line = "▸ [ 查找 ]    [ 取消 ]"
        result.append(edge() + pad_to_width(btn_line, inner_w, align="center") + edge())
        result.append(box_bottom(dialog_w, focused=True))

        # Center overlay
        BG = " "
        dialog_h = len(result)
        top_pad = max(0, (height - dialog_h) // 2)
        left_pad = max(0, (width - dialog_w) // 2)
        padded: list[str] = []
        for _ in range(top_pad):
            padded.append(BG * width)
        for line in result:
            padded.append(BG * left_pad + pad_to_width(line, dialog_w) + BG * (width - left_pad - dialog_w))
        for _ in range(height - len(padded)):
            padded.append(BG * width)
        return padded[:height]

    def handle_key(self, key: str) -> bool:
        if not self.visible:
            return False
        if key == "esc":
            self.visible = False
            return True
        if key in ("up", "s-tab"):
            self._focus_row = (self._focus_row - 1) % 5
            return True
        if key in ("down", "tab"):
            self._focus_row = (self._focus_row + 1) % 5
            return True
        if key == "enter" and self._focus_row == 4:
            self.visible = False  # execute find
            return True
        if key == "enter" and self._focus_row == 1:
            # Cycle scope
            idx = self.scopes.index(self.scope) if self.scope in self.scopes else 0
            self.scope = self.scopes[(idx + 1) % len(self.scopes)]
            self._scope_idx = (self._scope_idx + 1) % len(self.scopes)
            return True
        if key == "enter" and self._focus_row == 2:
            self.case_sensitive = not self.case_sensitive
            return True
        if key == "enter" and self._focus_row == 3:
            self.use_regex = not self.use_regex
            return True
        # Printable characters in query field
        if self._focus_row == 0 and len(key) == 1 and key.isprintable():
            self.query += key
            return True
        if self._focus_row == 0 and key == "backspace" and self.query:
            self.query = self.query[:-1]
            return True
        if self._focus_row == 0 and key == "space":
            self.query += " "
            return True
        return False


# ── helpers ─────────────────────────────────────────────────────────────────


def edge() -> str:
    """Return the vertical border character for the current focus state.

    This is a module-level helper so that compositors and widgets can
    use a consistent border style without passing state explicitly.
    """
    return "│"  # │
