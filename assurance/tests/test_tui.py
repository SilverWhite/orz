"""Screenshot / snapshot tests for GAK-UI-001 TUI first prototype.

All tests render the TUI to string buffers at various viewport sizes
and verify structural invariants: every region is present, keyboard
navigation works, and overlays render correctly.
"""

from __future__ import annotations

import json
import sys
import unittest
from io import StringIO
from pathlib import Path

# Verify tui/ module is importable
from assurance.tui.app import TuiPrototype, render_screen
from assurance.tui.widgets import (
    AddressBar,
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
    box_vertical,
    display_width,
    pad_to_width,
)
from assurance.tui.view_models import (
    SAMPLE_ADDRESS_URI,
    SAMPLE_COMMAND_URI,
    SAMPLE_CONTENT_MARKER,
    SAMPLE_EVENT_GROUPS,
    SAMPLE_FIND_QUERY,
    SAMPLE_FIND_SCOPE,
    SAMPLE_FIND_SCOPES,
    SAMPLE_MENUS,
    SAMPLE_SOURCE_TABLE,
    SAMPLE_SOURCE_TREE,
    SAMPLE_STATUS_ITEMS,
)

ROOT = Path(__file__).resolve().parents[2]


# ── import isolation ────────────────────────────────────────────────────────


class TuiImportIsolationTests(unittest.TestCase):
    """GAK-UI-001: The TUI package must not import from assurance core."""

    def test_no_assurance_core_imports_in_tui_package(self) -> None:
        """Verify tui/ modules don't import from the assurance kernel."""
        tui_dir = ROOT / "assurance" / "tui"
        self.assertTrue(tui_dir.is_dir(), "tui/ directory must exist")

        assurance_core_modules = {
            "assurance.canonical_cli",
            "assurance.contracts",
            "assurance.conversation",
            "assurance.deepseek_adapter",
            "assurance.envelope",
            "assurance.guarded_execution",
            "assurance.instruction_provenance_gate",
            "assurance.keystore",
            "assurance.sandbox",
            "assurance.windows_sandbox",
        }

        violations: list[str] = []
        for py_file in sorted(tui_dir.glob("*.py")):
            source = py_file.read_text(encoding="utf-8")
            for mod in assurance_core_modules:
                short = mod.split(".")[-1]
                # Check for "from .X import" where X is a core module
                if f"from ..{short}" in source or f"import ..{short}" in source:
                    violations.append(f"{py_file.name}: imports {mod}")
                # "from assurance.X import": check that the import is not a tui-local
                if f"from assurance.{short}" in source and short not in (
                    "tui", "__init__"
                ):
                    violations.append(f"{py_file.name}: possible import of {mod}")

        if violations:
            self.fail(
                "GAK-UI-001: TUI package must not import from assurance core:\n"
                + "\n".join(violations)
            )


# ── box-drawing helpers ─────────────────────────────────────────────────────


class BoxDrawingTests(unittest.TestCase):
    def test_box_horizontal_produces_border(self) -> None:
        result = box_horizontal("Test", 20)
        self.assertIn("╭", result)
        self.assertIn("╮", result)
        self.assertIn("Test", result)
        self.assertEqual(display_width(result), 20)

    def test_box_horizontal_focused_produces_double_border(self) -> None:
        result = box_horizontal("Test", 20, focused=True)
        self.assertIn("╒", result)
        self.assertIn("╕", result)
        self.assertIn("═", result)

    def test_box_bottom_produces_border(self) -> None:
        result = box_bottom(20)
        self.assertIn("╰", result)
        self.assertIn("╯", result)
        self.assertEqual(display_width(result), 20)

    def test_box_t_junction_produces_t(self) -> None:
        result = box_t_junction(20)
        self.assertIn("├", result)
        self.assertIn("┤", result)

    def test_box_vertical_wraps_lines(self) -> None:
        lines = ["hello", "world"]
        result = box_vertical(lines, 10)
        self.assertEqual(len(result), 2)
        self.assertTrue(result[0].startswith("│"))
        self.assertTrue(result[0].endswith("│"))

    def test_box_horizontal_empty_title(self) -> None:
        result = box_horizontal("", 10)
        self.assertNotIn("None", result)
        self.assertEqual(display_width(result), 10)


# ── display width ───────────────────────────────────────────────────────────


class DisplayWidthTests(unittest.TestCase):
    def test_ascii_is_1_wide(self) -> None:
        self.assertEqual(display_width("hello"), 5)
        self.assertEqual(display_width("A"), 1)

    def test_chinese_is_2_wide(self) -> None:
        self.assertEqual(display_width("中文"), 4)
        self.assertEqual(display_width("测试"), 4)

    def test_mixed_is_correct(self) -> None:
        # "Hello测试" = 5 + 4 = 9 cells
        self.assertEqual(display_width("Hello测试"), 9)

    def test_empty_string(self) -> None:
        self.assertEqual(display_width(""), 0)

    def test_box_drawing_is_1_wide(self) -> None:
        self.assertEqual(display_width("╭─╮"), 3)


# ── pad_to_width ────────────────────────────────────────────────────────────


class PadToWidthTests(unittest.TestCase):
    def test_left_pad(self) -> None:
        result = pad_to_width("hi", 5)
        self.assertEqual(len(result), 5)
        self.assertTrue(result.startswith("hi"))

    def test_right_pad(self) -> None:
        result = pad_to_width("hi", 5, align="right")
        self.assertEqual(len(result), 5)
        self.assertTrue(result.endswith("hi"))

    def test_center_pad(self) -> None:
        result = pad_to_width("hi", 6, align="center")
        self.assertEqual(len(result), 6)

    def test_truncation(self) -> None:
        result = pad_to_width("hello world", 5)
        self.assertEqual(len(result), 5)


# ── widget rendering (individual) ───────────────────────────────────────────


class MenuBarRenderTests(unittest.TestCase):
    def test_renders_menu_items(self) -> None:
        menus = {"File": ["Open", "Exit"], "Edit": ["Cut", "Copy"]}
        bar = MenuBar(menus=menus)
        result = bar.render(80, 1)
        self.assertEqual(len(result), 1)
        self.assertIn("File", result[0])
        self.assertIn("Edit", result[0])

    def test_empty_menus_produces_space(self) -> None:
        bar = MenuBar(menus={})
        result = bar.render(80, 1)
        self.assertEqual(len(result), 1)
        self.assertTrue(result[0].strip() == "" or " " * 80 in result[0])

    def test_active_menu_shows_dropdown(self) -> None:
        menus = {"File": ["Open", "Exit"]}
        bar = MenuBar(menus=menus, active_menu="File")
        result = bar.render(80, 3)
        self.assertGreater(len(result), 1)
        self.assertIn("Open", "\n".join(result))

    def test_alt_key_activates_menu(self) -> None:
        menus = {"File": ["Open"], "Edit": ["Cut"]}
        bar = MenuBar(menus=menus)
        self.assertTrue(bar.handle_key("alt+f"))
        self.assertEqual(bar.active_menu, "File")
        bar.handle_key("esc")
        self.assertEqual(bar.active_menu, "")


class ToolbarRenderTests(unittest.TestCase):
    def test_renders_buttons(self) -> None:
        bar = Toolbar(buttons=["Back", "Forward", "Stop"])
        result = bar.render(80, 1)
        self.assertIn("Back", result[0])
        self.assertIn("Forward", result[0])
        self.assertIn("Stop", result[0])

    def test_empty_buttons_produces_space(self) -> None:
        bar = Toolbar(buttons=[])
        result = bar.render(80, 1)
        self.assertTrue(result[0].strip() == "")


class AddressBarRenderTests(unittest.TestCase):
    def test_renders_uri(self) -> None:
        bar = AddressBar(uri="workspace://LIF/current-index")
        result = bar.render(100, 1)
        self.assertIn("workspace://LIF/current-index", result[0])

    def test_focused_shows_cursor(self) -> None:
        bar = AddressBar(uri="workspace://test", focused=True)
        result = bar.render(100, 1)
        # Focused bar shows cursor "█" at the end of the displayed content
        self.assertIn("█", result[0])

    def test_has_go_button(self) -> None:
        bar = AddressBar(uri="workspace://test")
        result = bar.render(100, 1)
        self.assertIn("[ Go ]", result[0])


class ExplorerPaneRenderTests(unittest.TestCase):
    def test_renders_tree(self) -> None:
        pane = ExplorerPane(tree=list(SAMPLE_SOURCE_TREE))
        result = pane.render(30, 15)
        text = "\n".join(result)
        self.assertIn("INDEX", text)
        self.assertIn("MAP", text)
        self.assertIn("R Series", text)

    def test_expanded_groups_show_children(self) -> None:
        pane = ExplorerPane(tree=list(SAMPLE_SOURCE_TREE))
        result = pane.render(30, 20)
        text = "\n".join(result)
        self.assertIn("MAP_MEM.md", text)
        self.assertIn("R211", text)

    def test_focused_has_double_border(self) -> None:
        pane = ExplorerPane(tree=list(SAMPLE_SOURCE_TREE), focused=True)
        result = pane.render(30, 15)
        self.assertTrue(
            any("║" in line for line in result),
            "focused explorer should use ║ vertical border",
        )


class ContentPaneRenderTests(unittest.TestCase):
    def test_renders_title(self) -> None:
        from assurance.tui.view_models import SAMPLE_SOURCE_TABLE as tbl
        from assurance.tui.view_models import (
            SAMPLE_CLAIM_DISPOSITION,
            SAMPLE_DISPOSITION_REASON,
            SAMPLE_NEXT_ACTIONS,
        )
        pane = ContentPane(
            title="Test Task",
            source_table=list(tbl),
            claim_disposition=SAMPLE_CLAIM_DISPOSITION,
            disposition_reason=SAMPLE_DISPOSITION_REASON,
            next_actions=list(SAMPLE_NEXT_ACTIONS),
        )
        result = pane.render(60, 20)
        text = "\n".join(result)
        self.assertIn("Test Task", text)
        self.assertIn("Source Visibility", text)

    def test_shows_claim_disposition(self) -> None:
        pane = ContentPane(
            title="Task",
            claim_disposition="DEFER",
            disposition_reason="Missing full text",
        )
        result = pane.render(60, 15)
        text = "\n".join(result)
        self.assertIn("DEFER", text)
        self.assertIn("Missing full text", text)

    def test_shows_next_actions(self) -> None:
        actions = ["Action one", "Action two"]
        pane = ContentPane(title="Task", next_actions=actions)
        result = pane.render(60, 15)
        text = "\n".join(result)
        self.assertIn("Action one", text)
        self.assertIn("Action two", text)


class StatusBarRenderTests(unittest.TestCase):
    def test_renders_items(self) -> None:
        bar = StatusBar(items=list(SAMPLE_STATUS_ITEMS))
        result = bar.render(100, 1)
        self.assertIn("GUARDED", result[0])
        self.assertIn("NET OFF", result[0])
        self.assertIn("IDLE", result[0])

    def test_empty_items_produces_space(self) -> None:
        bar = StatusBar(items=[])
        result = bar.render(100, 1)
        self.assertTrue(result[0].strip() == "")


class DialogRenderTests(unittest.TestCase):
    def test_not_visible_returns_none(self) -> None:
        dlg = Dialog(title="Test", message="hello", visible=False)
        self.assertIsNone(dlg.render_overlay(80, 24))

    def test_visible_renders_overlay(self) -> None:
        dlg = Dialog(
            title="Permission Required",
            message="Allow this action?",
            actions=[("Approve", True), ("Reject", False)],
            visible=True,
        )
        result = dlg.render_overlay(80, 24)
        self.assertIsNotNone(result)
        text = "\n".join(result or [])
        self.assertIn("Permission Required", text)
        self.assertIn("Allow this action?", text)
        self.assertIn("Approve", text)
        self.assertIn("Reject", text)

    def test_esc_dismisses_dialog(self) -> None:
        dlg = Dialog(title="Test", message="hello", visible=True)
        self.assertTrue(dlg.handle_key("esc"))
        self.assertFalse(dlg.visible)

    def test_tab_cycles_actions(self) -> None:
        dlg = Dialog(
            title="Test", message="hello",
            actions=[("A", 1), ("B", 2)], visible=True,
        )
        self.assertEqual(dlg._selected_action, 0)
        dlg.handle_key("tab")
        self.assertEqual(dlg._selected_action, 1)
        dlg.handle_key("tab")
        self.assertEqual(dlg._selected_action, 0)


class PropertiesSheetRenderTests(unittest.TestCase):
    def test_not_visible_returns_none(self) -> None:
        ps = PropertiesSheet(
            title="Properties",
            tabs=[{"name": "General", "fields": [("Name", "test")]}],
            visible=False,
        )
        self.assertIsNone(ps.render_overlay(80, 24))

    def test_visible_renders_overlay(self) -> None:
        ps = PropertiesSheet(
            title="Properties: R211.md",
            tabs=[
                {"name": "General", "fields": [("Name", "R211.md")]},
                {"name": "Provenance", "fields": [("Origin", "LIF")]},
            ],
            visible=True,
        )
        result = ps.render_overlay(80, 24)
        self.assertIsNotNone(result)
        text = "\n".join(result or [])
        self.assertIn("Properties: R211.md", text)
        self.assertIn("General", text)
        self.assertIn("Provenance", text)
        self.assertIn("R211.md", text)

    def test_tab_cycles_tabs(self) -> None:
        ps = PropertiesSheet(
            title="Test",
            tabs=[
                {"name": "TabA", "fields": []},
                {"name": "TabB", "fields": []},
            ],
            visible=True,
        )
        self.assertEqual(ps._active_tab, 0)
        ps.handle_key("tab")
        self.assertEqual(ps._active_tab, 1)
        ps.handle_key("tab")
        self.assertEqual(ps._active_tab, 0)


# ── full-screen rendering ───────────────────────────────────────────────────


# ── FindBar ──────────────────────────────────────────────────────────────────


class FindBarRenderTests(unittest.TestCase):
    def test_renders_query_and_scope(self) -> None:
        bar = FindBar(query="source visibility", scope="Conversation",
                       scopes=list(SAMPLE_FIND_SCOPES))
        result = bar.render(100, 1)
        line = result[0]
        self.assertIn("source visibility", line)
        self.assertIn("Conversation", line)
        self.assertIn("Find:", line)

    def test_renders_empty_query(self) -> None:
        bar = FindBar(query="", scope="Project Docs",
                       scopes=list(SAMPLE_FIND_SCOPES))
        result = bar.render(100, 1)
        self.assertIn("Project Docs", result[0])

    def test_tab_opens_scope_dropdown(self) -> None:
        bar = FindBar(query="test", scope="Conversation",
                       scopes=list(SAMPLE_FIND_SCOPES), focused=True)
        self.assertFalse(bar._scope_open)
        bar.handle_key("tab")
        self.assertTrue(bar._scope_open)

    def test_down_cycles_scope(self) -> None:
        bar = FindBar(query="test", scope="Conversation",
                       scopes=list(SAMPLE_FIND_SCOPES), focused=True)
        bar.handle_key("tab")  # open
        bar.handle_key("down")
        self.assertEqual(bar.scope, "Project Docs")
        bar.handle_key("down")
        self.assertEqual(bar.scope, "Current File")


# ── ContentMarker ────────────────────────────────────────────────────────────


class ContentMarkerRenderTests(unittest.TestCase):
    def test_renders_input_markers(self) -> None:
        marker = ContentMarker(markers=list(SAMPLE_CONTENT_MARKER))
        result = marker.render(16, 10)
        text = "\n".join(result)
        self.assertIn("▸", text)
        self.assertIn("L4", text)

    def test_renders_search_hit_markers(self) -> None:
        marker = ContentMarker(markers=list(SAMPLE_CONTENT_MARKER))
        result = marker.render(16, 10)
        text = "\n".join(result)
        self.assertIn("●", text)
        self.assertIn("L5", text)

    def test_empty_markers_shows_none(self) -> None:
        marker = ContentMarker(markers=[])
        result = marker.render(16, 10)
        text = "\n".join(result)
        self.assertIn("(none)", text)


# ── ExplorerPane event groups ────────────────────────────────────────────────


class ExplorerPaneEventTests(unittest.TestCase):
    def test_shows_event_groups(self) -> None:
        pane = ExplorerPane(
            tree=list(SAMPLE_SOURCE_TREE),
            event_groups=list(SAMPLE_EVENT_GROUPS),
        )
        result = pane.render(22, 30)
        text = "\n".join(result)
        self.assertIn("Events", text)
        self.assertIn("User Inputs", text)
        self.assertIn("Decisions", text)

    def test_collapsed_groups_dont_show_entries(self) -> None:
        pane = ExplorerPane(
            tree=[],
            event_groups=list(SAMPLE_EVENT_GROUPS),
        )
        result = pane.render(22, 20)
        text = "\n".join(result)
        # Errors group is collapsed (expanded=False)
        self.assertIn("Errors", text)
        self.assertNotIn("network permit denied", text)

    def test_mixed_tree_and_events(self) -> None:
        pane = ExplorerPane(
            tree=list(SAMPLE_SOURCE_TREE),
            event_groups=list(SAMPLE_EVENT_GROUPS),
        )
        result = pane.render(30, 40)
        text = "\n".join(result)
        # Tree items
        self.assertIn("INDEX", text)
        self.assertIn("MAP", text)
        # Event items
        self.assertIn("Events", text)
        self.assertIn("User Inputs", text)


class FullScreenRenderTests(unittest.TestCase):
    def test_render_at_100x30_has_all_seven_regions(self) -> None:
        output = render_screen(100, 30)
        lines = output.split("\n")
        self.assertGreater(len(lines), 25, "Screen should have at least 25 lines")
        self.assertLessEqual(len(lines), 35, "Screen should not exceed 35 lines")

        # All seven regions (v2 adds FindBar + ContentMarker)
        self.assertIn("File", output, "MenuBar missing")
        self.assertIn("Back", output, "Toolbar missing")
        self.assertIn("Address:", output, "AddressBar missing")
        self.assertIn("Find:", output, "FindBar missing")
        self.assertIn("INDEX", output, "ExplorerPane missing")
        self.assertIn("Source Visibility", output, "ContentPane missing")
        self.assertIn("Markers", output, "ContentMarker missing")
        self.assertIn("GUARDED", output, "StatusBar missing")

    def test_render_at_140x40_wider_layout(self) -> None:
        output = render_screen(140, 40)
        lines = output.split("\n")
        self.assertGreater(len(lines), 35)
        self.assertLessEqual(len(lines), 45)
        self.assertIn("File", output)
        self.assertIn("Source Visibility", output)
        self.assertIn("Find:", output)
        self.assertIn("Markers", output)

    def test_command_uri_displayed(self) -> None:
        """v2: address bar should show command:// URI."""
        output = render_screen(100, 30)
        self.assertIn("command://run/current-task", output)

    def test_three_column_split_present(self) -> None:
        """v2: ├──┬──┬──┤ triple column split."""
        output = render_screen(100, 30)
        self.assertIn("┬", output)  # triple-split junction
        self.assertIn("┴", output)  # triple-join junction

    def test_render_at_minimum_viewport_80x24(self) -> None:
        """Minimum viewport should not crash."""
        output = render_screen(80, 24)
        self.assertIsInstance(output, str)
        self.assertTrue(len(output) > 0)

    def test_render_below_minimum_reports_error(self) -> None:
        output = render_screen(30, 10)
        self.assertIn("too small", output.lower())

    def test_box_drawing_chars_present(self) -> None:
        output = render_screen(100, 30)
        box_chars = "╭╮╰╯├┤│─═"
        found = [c for c in box_chars if c in output]
        self.assertGreater(len(found), 3, f"Box-drawing chars missing: {output[:200]}")

    def test_address_bar_shows_uri(self) -> None:
        output = render_screen(100, 30)
        self.assertIn("command://run/current-task", output)

    def test_toolbar_has_standard_actions(self) -> None:
        output = render_screen(100, 30)
        for btn in ["Back", "Forward", "Stop", "Refresh", "Verify", "Properties"]:
            self.assertIn(btn, output, f"Toolbar missing '{btn}'")

    def test_status_bar_shows_mode_indicators(self) -> None:
        output = render_screen(100, 30)
        for indicator in ["GUARDED", "NET OFF", "SBX STRICT", "SOURCES", "IDLE"]:
            self.assertIn(indicator, output, f"StatusBar missing '{indicator}'")

    def test_explorer_shows_source_tree_groups(self) -> None:
        output = render_screen(100, 30)
        for group in ["INDEX", "MAP", "R Series", "Artifacts", "Adapters", "Verifier"]:
            self.assertIn(group, output, f"ExplorerPane missing '{group}'")

    def test_content_shows_task_summary(self) -> None:
        output = render_screen(100, 30)
        self.assertIn("Claim disposition", output)
        self.assertIn("Next Actions", output)

    def test_menu_bar_shows_all_menus(self) -> None:
        output = render_screen(100, 30)
        for menu_name in SAMPLE_MENUS:
            self.assertIn(menu_name, output, f"MenuBar missing '{menu_name}'")

    def test_lines_are_consistent_width(self) -> None:
        output = render_screen(100, 30)
        for i, line in enumerate(output.split("\n")):
            self.assertEqual(
                len(line), 100,
                f"Line {i} has width {len(line)}, expected 100: {line[:40]}..."
            )


# ── keyboard navigation ─────────────────────────────────────────────────────


class KeyboardNavigationTests(unittest.TestCase):
    def setUp(self) -> None:
        self.app = TuiPrototype.with_sample_data()

    def test_f6_cycles_focus(self) -> None:
        self.assertEqual(self.app.active_pane, "explorer")
        result = self.app.handle_key("f6")
        self.assertIn("content", result)
        self.assertEqual(self.app.active_pane, "content")
        result = self.app.handle_key("f6")
        self.assertIn("marker", result)
        self.assertEqual(self.app.active_pane, "marker")
        result = self.app.handle_key("f6")
        self.assertIn("address", result)
        self.assertEqual(self.app.active_pane, "address")
        result = self.app.handle_key("f6")
        self.assertIn("find", result)
        self.assertEqual(self.app.active_pane, "find")
        result = self.app.handle_key("f6")
        self.assertEqual(self.app.active_pane, "explorer")

    def test_alt_key_activates_menu(self) -> None:
        self.app.handle_key("alt+f")
        self.assertEqual(self.app.menu_bar.active_menu, "File")

    def test_esc_closes_menu(self) -> None:
        self.app.handle_key("alt+f")
        self.app.handle_key("esc")
        self.assertEqual(self.app.menu_bar.active_menu, "")

    def test_dialog_blocks_focus_cycle(self) -> None:
        self.app.dialog.visible = True
        result = self.app.handle_key("f6")
        # Focus should NOT change while dialog is visible
        self.assertEqual(self.app.active_pane, "explorer")
        self.assertIsNone(result)

    def test_esc_dismisses_dialog(self) -> None:
        self.app.dialog.visible = True
        self.app.handle_key("esc")
        self.assertFalse(self.app.dialog.visible)

    def test_properties_block_focus_cycle(self) -> None:
        self.app.properties.visible = True
        result = self.app.handle_key("f6")
        self.assertIsNone(result)


# ── dialog overlay ──────────────────────────────────────────────────────────


class DialogOverlayRenderTests(unittest.TestCase):
    def test_dialog_overlay_in_full_screen(self) -> None:
        app = TuiPrototype.with_sample_data()
        app.dialog.visible = True
        output = app.render(100, 30)
        self.assertIn("Permission Required", output)

    def test_dialog_shows_action_buttons(self) -> None:
        app = TuiPrototype.with_sample_data()
        app.dialog.visible = True
        output = app.render(100, 30)
        self.assertIn("Approve", output)
        self.assertIn("Reject", output)

    def test_esc_dismisses_dialog_in_app(self) -> None:
        app = TuiPrototype.with_sample_data()
        app.dialog.visible = True
        app.handle_key("esc")
        self.assertFalse(app.dialog.visible)

    def test_dialog_does_not_appear_when_hidden(self) -> None:
        app = TuiPrototype.with_sample_data()
        app.dialog.visible = False
        output = app.render(100, 30)
        self.assertNotIn("Permission Required", output)


# ── properties overlay ──────────────────────────────────────────────────────


class PropertiesOverlayRenderTests(unittest.TestCase):
    def test_properties_overlay_in_full_screen(self) -> None:
        app = TuiPrototype.with_sample_data()
        app.properties.visible = True
        output = app.render(100, 30)
        self.assertIn("Properties: R211.md", output)

    def test_properties_shows_tabs(self) -> None:
        app = TuiPrototype.with_sample_data()
        app.properties.visible = True
        output = app.render(100, 30)
        for tab in ["General", "Provenance", "Visibility"]:
            self.assertIn(tab, output, f"Properties missing tab '{tab}'")

    def test_properties_does_not_appear_when_hidden(self) -> None:
        app = TuiPrototype.with_sample_data()
        app.properties.visible = False
        output = app.render(100, 30)
        self.assertNotIn("Properties: R211.md", output)

    def test_tab_key_cycles_tabs(self) -> None:
        app = TuiPrototype.with_sample_data()
        app.properties.visible = True
        self.assertEqual(app.properties._active_tab, 0)
        app.handle_key("tab")
        self.assertEqual(app.properties._active_tab, 1)


# ── prototype with all widgets rendered ─────────────────────────────────────


class PrototypeIntegrationTests(unittest.TestCase):
    def test_prototype_builds_and_renders(self) -> None:
        app = TuiPrototype.with_sample_data()
        output = app.render(100, 30)
        self.assertIsInstance(output, str)
        self.assertGreater(len(output), 100)
        # Basic structure
        self.assertIn("╭", output)  # top border present
        self.assertIn("╰", output)  # bottom border present

    def test_render_is_deterministic(self) -> None:
        a = render_screen(100, 30)
        b = render_screen(100, 30)
        self.assertEqual(a, b, "render_screen must be deterministic")

    def test_render_screen_can_be_captured(self) -> None:
        """Verify render_screen output can be captured for CI."""
        output = render_screen(80, 24)
        self.assertTrue("\n".join(output.split("\n")[0:3]).strip() != "")


if __name__ == "__main__":
    unittest.main()


# ──────────────────────────────────────────────────────────────────────────────
# GAK-UI-001 / Slash-command system tests
# ──────────────────────────────────────────────────────────────────────────────


class CommandRegistryTests(unittest.TestCase):
    """GAK-UI-001: slash-command registry search and defaults."""

    def setUp(self) -> None:
        from assurance.tui.commands import CommandRegistry
        self.registry = CommandRegistry.with_builtins()

    def test_fifteen_commands_registered(self) -> None:
        self.assertEqual(len(self.registry), 15)

    def test_defaults_are_six(self) -> None:
        defaults = self.registry.get_defaults()
        self.assertEqual(len(defaults), 6)

    def test_defaults_exclude_help_and_non_default_commands(self) -> None:
        defaults = self.registry.get_defaults()
        slashes = [c.slash for c in defaults]
        non_default = [
            "/help", "/diff", "/verify", "/sources", "/plan",
            "/rewind", "/model", "/status", "/export",
        ]
        for s in non_default:
            self.assertNotIn(s, slashes, f"{s} should not be in defaults")
        self.assertIn("/new", slashes)
        self.assertIn("/run", slashes)
        self.assertIn("/kill", slashes)

    def test_search_slash_alone_returns_defaults(self) -> None:
        matches = self.registry.search("/")
        self.assertEqual(len(matches), 6)

    def test_search_slash_new_exact(self) -> None:
        matches = self.registry.search("/new")
        self.assertEqual(len(matches), 1)
        self.assertEqual(matches[0].slash, "/new")

    def test_search_slash_con_prefix(self) -> None:
        matches = self.registry.search("/con")
        self.assertEqual(len(matches), 1)
        self.assertEqual(matches[0].slash, "/context")

    def test_search_no_prefix_returns_empty(self) -> None:
        self.assertEqual(self.registry.search("new"), [])
        self.assertEqual(self.registry.search(""), [])

    def test_search_slash_d_returns_diff(self) -> None:
        matches = self.registry.search("/d")
        self.assertEqual(len(matches), 1)
        self.assertEqual(matches[0].slash, "/diff")

    def test_search_slash_v_returns_verify(self) -> None:
        matches = self.registry.search("/v")
        self.assertEqual(len(matches), 1)
        self.assertEqual(matches[0].slash, "/verify")

    def test_search_slash_s_returns_sources(self) -> None:
        matches = self.registry.search("/s")
        self.assertTrue(any(c.slash == "/sources" for c in matches))

    def test_search_slash_p_returns_plan(self) -> None:
        matches = self.registry.search("/p")
        self.assertTrue(any(c.slash == "/plan" for c in matches))

    def test_new_commands_have_correct_categories(self) -> None:
        self.assertEqual(self.registry.get("/diff").category, "变更")
        self.assertEqual(self.registry.get("/verify").category, "保证")
        self.assertEqual(self.registry.get("/sources").category, "保证")
        self.assertEqual(self.registry.get("/plan").category, "工作流")
        self.assertEqual(self.registry.get("/rewind").category, "恢复")
        self.assertEqual(self.registry.get("/model").category, "系统")
        self.assertEqual(self.registry.get("/status").category, "系统")

    def test_search_second_priority_commands(self) -> None:
        self.assertEqual(len(self.registry.search("/r")), 2)  # /run + /rewind
        self.assertTrue(any(c.slash == "/rewind" for c in self.registry.search("/re")))
        self.assertEqual(len(self.registry.search("/m")), 1)   # /model
        self.assertEqual(len(self.registry.search("/st")), 1)  # /status

    def test_every_command_has_chinese_name_and_description(self) -> None:
        for cmd in self.registry.all():
            self.assertTrue(cmd.name_zh, f"{cmd.slash} missing name_zh")
            self.assertTrue(cmd.description_zh, f"{cmd.slash} missing description_zh")
            self.assertTrue(cmd.slash.startswith("/"), f"{cmd.slash} bad prefix")
            self.assertTrue(cmd.uri.startswith("command://"), f"{cmd.slash} bad uri")

    def test_all_annotations_are_chinese_not_english(self) -> None:
        """Every name_zh and description_zh must contain CJK characters."""
        for cmd in self.registry.all():
            has_cjk = any(
                (0x4E00 <= ord(ch) <= 0x9FFF) or (0x3400 <= ord(ch) <= 0x4DBF)
                for ch in cmd.name_zh + cmd.description_zh
            )
            self.assertTrue(
                has_cjk,
                f"{cmd.slash}: name_zh='{cmd.name_zh}' "
                f"desc='{cmd.description_zh}' has no CJK chars",
            )

    def test_get_returns_none_for_unknown(self) -> None:
        self.assertIsNone(self.registry.get("/nonexistent"))


class AddressBarTextInputTests(unittest.TestCase):
    """GAK-UI-001: AddressBar text input buffer and auto-complete triggers."""

    def test_buffer_starts_empty(self) -> None:
        from assurance.tui.widgets import AddressBar
        bar = AddressBar(uri="command://run/status")
        self.assertEqual(bar._buffer, "")
        self.assertFalse(bar._show_autocomplete)

    def test_typing_slash_opens_autocomplete(self) -> None:
        from assurance.tui.widgets import AddressBar
        bar = AddressBar(uri="command://run/status", focused=True)
        bar.handle_key("/")
        self.assertEqual(bar._buffer, "/")
        self.assertTrue(bar._show_autocomplete)

    def test_typing_slash_new_shows_one_match(self) -> None:
        from assurance.tui.widgets import AddressBar
        bar = AddressBar(uri="command://run/status", focused=True)
        bar.handle_key("/")
        bar.handle_key("n")
        bar.handle_key("e")
        bar.handle_key("w")
        self.assertEqual(bar._buffer, "/new")
        self.assertTrue(bar._show_autocomplete)
        self.assertEqual(len(bar.autocomplete_candidates), 1)

    def test_escape_clears_buffer(self) -> None:
        from assurance.tui.widgets import AddressBar
        bar = AddressBar(uri="command://run/status", focused=True)
        bar.handle_key("/")
        bar.handle_key("c")
        self.assertTrue(bar._show_autocomplete)
        bar.handle_key("esc")
        self.assertEqual(bar._buffer, "")
        self.assertFalse(bar._show_autocomplete)

    def test_backspace_removes_last_char(self) -> None:
        from assurance.tui.widgets import AddressBar
        bar = AddressBar(uri="command://run/status", focused=True)
        bar.handle_key("/")
        bar.handle_key("c")
        self.assertEqual(bar._buffer, "/c")
        bar.handle_key("backspace")
        self.assertEqual(bar._buffer, "/")
        bar.handle_key("backspace")
        self.assertEqual(bar._buffer, "")

    def test_up_down_select_autocomplete_index(self) -> None:
        from assurance.tui.widgets import AddressBar
        bar = AddressBar(uri="command://run/status", focused=True)
        bar.handle_key("/")
        self.assertEqual(bar._selected_index, 0)
        bar.handle_key("down")
        self.assertEqual(bar._selected_index, 1)
        bar.handle_key("down")
        self.assertEqual(bar._selected_index, 2)
        bar.handle_key("up")
        self.assertEqual(bar._selected_index, 1)
        # Wrap-around not enforced by AddressBar; CommandPalette clamps

    def test_tab_completes_first_match(self) -> None:
        from assurance.tui.widgets import AddressBar
        bar = AddressBar(uri="command://run/status", focused=True)
        bar.handle_key("/")
        bar.handle_key("c")
        bar.handle_key("o")
        self.assertEqual(bar._buffer, "/co")
        bar.handle_key("tab")
        self.assertEqual(bar._buffer, "/context")

    def test_enter_with_buffer_returns_activated(self) -> None:
        from assurance.tui.widgets import AddressBar
        bar = AddressBar(uri="command://run/status", focused=True)
        bar.handle_key("/")
        bar.handle_key("n")
        bar.handle_key("e")
        bar.handle_key("w")
        result = bar.handle_key("enter")
        self.assertTrue(result)
        # Buffer is cleared after command activation
        self.assertEqual(bar._buffer, "")

    def test_ignores_keys_when_not_focused(self) -> None:
        from assurance.tui.widgets import AddressBar
        bar = AddressBar(uri="command://run/status", focused=False)
        result = bar.handle_key("/")
        self.assertFalse(result)
        self.assertEqual(bar._buffer, "")

    def test_render_shows_buffer_when_focused(self) -> None:
        from assurance.tui.widgets import AddressBar
        bar = AddressBar(uri="command://run/status", focused=True)
        bar.handle_key("/")
        output = bar.render(100, 1)
        self.assertIn("/", output[0])
        self.assertIn("█", output[0])  # cursor present

    def test_render_shows_uri_when_not_focused(self) -> None:
        from assurance.tui.widgets import AddressBar
        bar = AddressBar(uri="command://run/status", focused=False)
        output = bar.render(100, 1)
        self.assertIn("command://run/status", output[0])
        self.assertNotIn("█", output[0])  # no cursor


class CommandHistoryTests(unittest.TestCase):
    """GAK-UI-001: ↑↓ command history navigation."""

    def setUp(self) -> None:
        from assurance.tui.widgets import AddressBar
        self.bar = AddressBar(uri="command://run/status", focused=True)

    def _send(self, text: str) -> None:
        for ch in text:
            self.bar.handle_key(ch)
        self.bar._push_history(self.bar._buffer)
        self.bar._buffer = ""
        self.bar._show_autocomplete = False

    def test_empty_history_up_does_nothing(self) -> None:
        self.assertFalse(self.bar.handle_key("up"))

    def test_push_and_recall_one_entry(self) -> None:
        self._send("/run")
        # Buffer is empty; ↑ shows /run
        self.assertTrue(self.bar.handle_key("up"))
        self.assertEqual(self.bar._buffer, "/run")

    def test_up_then_down_restores_draft(self) -> None:
        self._send("/new")
        self._send("/run")
        self.bar.handle_key("up")  # → /run
        self.assertEqual(self.bar._buffer, "/run")
        self.bar.handle_key("up")  # → /new
        self.assertEqual(self.bar._buffer, "/new")
        self.bar.handle_key("down")  # → /run
        self.assertEqual(self.bar._buffer, "/run")
        self.bar.handle_key("down")  # → draft (empty)
        self.assertEqual(self.bar._buffer, "")

    def test_history_does_not_trigger_when_buffer_has_content(self) -> None:
        self._send("/run")
        self.bar.handle_key("x")  # type something
        self.assertEqual(self.bar._buffer, "x")
        self.bar.handle_key("up")  # should navigate palette, not history
        self.assertEqual(self.bar._buffer, "x")

    def test_dup_entry_not_pushed(self) -> None:
        self._send("/run")
        self._send("/run")  # duplicate — should not re-push
        self.assertEqual(len(self.bar._history), 1)


class MultiLineInputTests(unittest.TestCase):
    """GAK-UI-001: Shift+Enter multi-line input."""

    def setUp(self) -> None:
        from assurance.tui.widgets import AddressBar
        self.bar = AddressBar(uri="command://run/status", focused=True)

    def test_shift_enter_inserts_newline(self) -> None:
        self.bar.handle_key("h")
        self.bar.handle_key("i")
        self.bar.handle_key("s-enter")
        self.assertEqual(self.bar._buffer, "hi\n")

    def test_multi_line_line_count(self) -> None:
        self.bar.handle_key("a")
        self.bar.handle_key("s-enter")
        self.bar.handle_key("b")
        self.bar.handle_key("s-enter")
        self.bar.handle_key("c")
        self.assertEqual(self.bar._line_count, 3)

    def test_multi_line_render_shows_last_three(self) -> None:
        def _type(text: str) -> None:
            for ch in text:
                self.bar.handle_key(ch)
        _type("line1")
        self.bar.handle_key("s-enter")
        _type("line2")
        result = self.bar.render(100, 3)
        self.assertEqual(len(result), 2)  # 2 lines shown
        text = "\n".join(result)
        self.assertIn("line1", text)
        self.assertIn("line2", text)

    def test_multi_line_over_three_shows_only_last_three(self) -> None:
        def _type(text: str) -> None:
            for ch in text:
                self.bar.handle_key(ch)
        # Build 5 lines — only last 3 should be visible
        _type("第1行")
        self.bar.handle_key("s-enter")
        _type("第2行")
        self.bar.handle_key("s-enter")
        _type("第3行")
        self.bar.handle_key("s-enter")
        _type("第4行")
        self.bar.handle_key("s-enter")
        _type("第5行")
        self.assertEqual(self.bar._buffer.count("\n"), 4)  # 5 total lines
        self.assertEqual(self.bar._line_count, 3)  # capped at 3
        result = self.bar.render(100, 3)
        self.assertEqual(len(result), 3)
        text = "\n".join(result)
        # 第1,2行 scrolled off; 第3,4,5 visible
        self.assertNotIn("第1行", text)
        self.assertNotIn("第2行", text)
        self.assertIn("第3行", text)
        self.assertIn("第4行", text)
        self.assertIn("第5行", text)

    def test_autocomplete_disabled_in_multi_line(self) -> None:
        """Slash at start of a non-first line should not open palette."""
        self.bar.handle_key("/")
        self.assertTrue(self.bar._show_autocomplete)
        self.bar.handle_key("s-enter")
        self.assertFalse(self.bar._show_autocomplete)


class CommandPaletteRenderTests(unittest.TestCase):
    """GAK-UI-001: CommandPalette overlay rendering."""

    def test_hidden_returns_none(self) -> None:
        from assurance.tui.widgets import CommandPalette
        cp = CommandPalette(candidates=[], visible=False)
        self.assertIsNone(cp.render_overlay(80, 24))

    def test_no_candidates_returns_none(self) -> None:
        from assurance.tui.widgets import CommandPalette
        cp = CommandPalette(candidates=[], visible=True)
        self.assertIsNone(cp.render_overlay(80, 24))

    def test_renders_chinese_annotations(self) -> None:
        from assurance.tui.widgets import CommandPalette
        from assurance.tui.commands import get_builtin_registry
        cmds = get_builtin_registry().search("/")
        cp = CommandPalette(candidates=cmds, selected_index=0, visible=True)
        overlay = cp.render_overlay(120, 30)
        self.assertIsNotNone(overlay)
        text = "\n".join(overlay or [])
        self.assertIn("新对话", text)
        self.assertIn("上下文可视", text)
        self.assertIn("强制终止", text)
        self.assertIn("/new", text)
        self.assertIn("/context", text)
        self.assertIn("/kill", text)

    def test_renders_command_uris(self) -> None:
        from assurance.tui.widgets import CommandPalette
        from assurance.tui.commands import get_builtin_registry
        cmds = get_builtin_registry().search("/")
        cp = CommandPalette(candidates=cmds, selected_index=0, visible=True)
        overlay = cp.render_overlay(120, 30)
        self.assertIsNotNone(overlay)
        text = "\n".join(overlay or [])
        self.assertIn("指令", text)  # title

    def test_selected_index_clamped(self) -> None:
        from assurance.tui.widgets import CommandPalette
        from assurance.tui.commands import get_builtin_registry
        cmds = get_builtin_registry().search("/")
        # Index -1 → clamped to 0
        cp = CommandPalette(candidates=cmds, selected_index=-1, visible=True)
        overlay = cp.render_overlay(120, 30)
        self.assertIsNotNone(overlay)
        # Index 999 → clamped to last
        cp2 = CommandPalette(candidates=cmds, selected_index=999, visible=True)
        overlay2 = cp2.render_overlay(120, 30)
        self.assertIsNotNone(overlay2)


class SlashCommandIntegrationTests(unittest.TestCase):
    """GAK-UI-001: end-to-end slash-command integration in TuiPrototype."""

    def setUp(self) -> None:
        from assurance.tui.app import TuiPrototype
        self.app = TuiPrototype.with_sample_data()

    def test_address_bar_f6_then_type_slash(self) -> None:
        """Focus the address bar, type '/', verify auto-complete opens."""
        # Cycle to address pane
        while self.app.active_pane != "address":
            self.app.handle_key("f6")
        self.assertEqual(self.app.active_pane, "address")
        # Type '/'
        self.app.handle_key("/")
        self.assertTrue(self.app.address_bar._show_autocomplete)
        # Palette visibility is updated only at render() time;
        # verify it becomes visible after a render cycle.
        self.app.render(100, 30)
        self.assertTrue(self.app.command_palette.visible)

    def test_render_includes_palette_when_typing_slash(self) -> None:
        """After typing '/', palette overlay appears in the rendered output."""
        while self.app.active_pane != "address":
            self.app.handle_key("f6")
        self.app.handle_key("/")
        output = self.app.render(100, 30)
        self.assertIn("指令", output)
        self.assertIn("/new", output)
        self.assertIn("新对话", output)

    def test_escape_clears_buffer_and_hides_palette(self) -> None:
        while self.app.active_pane != "address":
            self.app.handle_key("f6")
        self.app.handle_key("/")
        self.app.handle_key("esc")
        self.assertFalse(self.app.address_bar._show_autocomplete)
        self.assertFalse(self.app.command_palette.visible)

    def test_dialog_and_palette_dont_overlap(self) -> None:
        """When palette is open, dialog should not render on top."""
        while self.app.active_pane != "address":
            self.app.handle_key("f6")
        self.app.handle_key("/")
        # Try to open dialog — should be blocked while palette is visible
        self.app.dialog.visible = True
        output = self.app.render(100, 30)
        self.assertIn("指令", output)  # palette present
        self.assertNotIn("Permission Required", output)  # dialog suppressed


class EscCancelRunTests(unittest.TestCase):
    """GAK-UI-001: Esc during active run cancels the run (retract sent input)."""

    def setUp(self) -> None:
        from assurance.tui.app import TuiPrototype
        self.app = TuiPrototype.with_sample_data()

    def test_f5_toggles_running_state(self) -> None:
        self.assertFalse(self.app.running)
        result = self.app.handle_key("f5")
        self.assertTrue(self.app.running)
        self.assertIn("模拟运行已启动", result)
        result = self.app.handle_key("f5")
        self.assertFalse(self.app.running)
        self.assertIn("模拟运行已停止", result)

    def test_ctrl_z_cancels_running_returns_idle(self) -> None:
        self.app.handle_key("f5")  # start run
        self.assertTrue(self.app.running)
        result = self.app.handle_key("c-z")
        self.assertFalse(self.app.running)
        self.assertIn("已取消当前运行", result)

    def test_ctrl_z_when_not_running_does_nothing(self) -> None:
        self.assertFalse(self.app.running)
        result = self.app.handle_key("c-z")
        self.assertIsNone(result)

    def test_status_bar_shows_running_when_active(self) -> None:
        self.app.handle_key("f5")  # start run
        output = self.app.render(100, 30)
        self.assertIn("RUNNING", output)
        self.assertNotIn("IDLE", output)

    def test_status_bar_shows_idle_after_ctrl_z_cancel(self) -> None:
        self.app.handle_key("f5")
        self.app.handle_key("c-z")
        output = self.app.render(100, 30)
        self.assertIn("IDLE", output)
        self.assertNotIn("RUNNING", output)

    def test_ctrl_z_with_buffer_and_running_restores_last_sent(self) -> None:
        """Running takes priority — Ctrl+Z cancels run, restores last sent."""
        # First send a command (this sets _last_sent)
        while self.app.active_pane != "address":
            self.app.handle_key("f6")
        self.app.handle_key("/")
        self.app.handle_key("r")
        self.app.handle_key("u")
        self.app.handle_key("n")
        self.app.handle_key("enter")  # sends /run, sets _last_sent="/run"
        # Now type something new in the buffer while running
        self.app.handle_key("f5")  # start run
        self.app.handle_key("/")
        self.app.handle_key("h")
        self.assertEqual(self.app.address_bar._buffer, "/h")
        # Ctrl+Z should cancel the run and restore _last_sent="/run"
        self.app.handle_key("c-z")
        self.assertFalse(self.app.running)
        self.assertEqual(self.app.address_bar._buffer, "/run")

    def test_cancel_run_refills_last_sent_into_bar(self) -> None:
        """After Ctrl+Z cancels, the last sent command reappears in the bar."""
        # Type and send a command
        while self.app.active_pane != "address":
            self.app.handle_key("f6")
        self.app.handle_key("/")
        self.app.handle_key("n")
        self.app.handle_key("e")
        self.app.handle_key("w")
        self.assertEqual(self.app.address_bar._buffer, "/new")
        # Send it
        self.app.handle_key("enter")
        # Simulate a run starting
        self.app.handle_key("f5")
        self.assertTrue(self.app.running)
        # Cancel — buffer should refill with /new
        self.app.handle_key("c-z")
        self.assertFalse(self.app.running)
        self.assertEqual(self.app.address_bar._buffer, "/new")

    def test_cancel_run_focuses_address_bar(self) -> None:
        """After cancel, the address bar is focused for immediate editing."""
        # Navigate away from address bar
        while self.app.active_pane != "explorer":
            self.app.handle_key("f6")
        self.assertEqual(self.app.active_pane, "explorer")
        # Send something
        while self.app.active_pane != "address":
            self.app.handle_key("f6")
        self.app.handle_key("/")
        self.app.handle_key("r")
        self.app.handle_key("enter")
        # Navigate away again
        self.app.handle_key("f6")
        self.app.handle_key("f5")  # start run
        self.assertTrue(self.app.running)
        # Ctrl+Z — address bar should be focused
        self.app.handle_key("c-z")
        self.assertEqual(self.app.active_pane, "address")
        self.assertTrue(self.app.address_bar.focused)

    def test_refilled_slash_command_shows_autocomplete(self) -> None:
        """If the restored input starts with /, auto-complete reopens."""
        while self.app.active_pane != "address":
            self.app.handle_key("f6")
        self.app.handle_key("/")
        self.app.handle_key("v")
        self.app.handle_key("enter")
        self.app.handle_key("f5")
        self.app.handle_key("c-z")
        self.assertEqual(self.app.address_bar._buffer, "/v")
        self.assertTrue(self.app.address_bar._show_autocomplete)

    def test_backspace_deletes_char_when_not_running(self) -> None:
        """When not running, backspace deletes last character in address bar."""
        while self.app.active_pane != "address":
            self.app.handle_key("f6")
        self.app.handle_key("/")
        self.app.handle_key("h")
        self.assertEqual(self.app.address_bar._buffer, "/h")
        self.app.handle_key("backspace")
        self.assertEqual(self.app.address_bar._buffer, "/")
