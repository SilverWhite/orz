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

    def render(self, width: int, height: int) -> list[str]:
        if height < 1:
            return []
        names = list(self.menus.keys())
        if not names:
            return [" " * width]

        label_chunks: list[str] = []
        for name in names:
            marker = "&" if self.focused or self.active_menu else " "
            # Underline the first letter hint for Alt+letter access
            first = name[0]
            label_chunks.append(f"{marker}{first}{name[1:]}")

        # Build the menu bar line: " File  Edit  View ..."
        line = ""
        sep = "  "
        for chunk in label_chunks:
            line += chunk + sep
        line = line.rstrip()

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
            first = name[0]
            if name == self.active_menu:
                chunks.append(f"▼{first}{name[1:]}")
            else:
                chunks.append(f" {first}{name[1:]}")
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
            for name in self.menus:
                if name.upper().startswith(letter):
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
    uri: str = ""
    focusable: bool = True

    def render(self, width: int, height: int) -> list[str]:
        if height < 1:
            return []
        addr_label = "Address: "
        go_button = "[ Go ]"
        available = width - display_width(addr_label) - display_width(go_button) - 2
        if available < 10:
            available = 10
        uri_display = self.uri
        if display_width(uri_display) > available:
            uri_display = uri_display[:available - 3] + "..."
        filler = " " * (available - display_width(uri_display))
        if self.focused:
            uri_display = "▶" + uri_display[1:] if uri_display else "▶"
        return [pad_to_width(addr_label + uri_display + filler + go_button, width)]

    def handle_key(self, key: str) -> bool:
        if key == "enter" and self.focused:
            return True  # would activate Go
        return False


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


# ── helpers ─────────────────────────────────────────────────────────────────


def edge() -> str:
    """Return the vertical border character for the current focus state.

    This is a module-level helper so that compositors and widgets can
    use a consistent border style without passing state explicitly.
    """
    return "│"  # │
