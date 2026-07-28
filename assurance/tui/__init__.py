"""Retro desktop-style Terminal UI prototype — GAK-UI-001.

The TUI is organised as a spatial, object-oriented workspace browser
rather than a chat-like scrollback.  It reuses mature desktop
interaction grammar (menus, toolbar, address bar, explorer pane,
properties, dialogs) adapted to agent workflows.

This first prototype is static and disconnected from the assurance
core.  All widgets render to plain string buffers; prompt_toolkit
integration follows in a later iteration.

Exports
-------
- :class:`TuiPrototype` — main application compositor
- :func:`render_screen` — single-call rendering entry point
- All widget classes for direct use or extension
"""

from .app import TuiPrototype, render_screen
from .commands import CommandDef, CommandRegistry, get_builtin_registry
from .pt_app import create_pt_application, run_tui_demo
from .view_models import (
    SAMPLE_ADDRESS_URI,
    SAMPLE_CLAIM_DISPOSITION,
    SAMPLE_COMMANDS,
    SAMPLE_DIALOG,
    SAMPLE_DISPOSITION_REASON,
    SAMPLE_MENUS,
    SAMPLE_NEXT_ACTIONS,
    SAMPLE_PROPERTIES,
    SAMPLE_SHORTCUTS,
    SAMPLE_SOURCE_TABLE,
    SAMPLE_SOURCE_TREE,
    SAMPLE_STATUS_ITEMS,
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
    box_double_horizontal,
    box_horizontal,
    box_t_junction,
    box_vertical,
)

__all__ = [
    "AddressBar",
    "CommandDef",
    "CommandPalette",
    "CommandRegistry",
    "ContentMarker",
    "ContentPane",
    "Dialog",
    "ExplorerPane",
    "FindBar",
    "MenuBar",
    "PropertiesSheet",
    "StatusBar",
    "Toolbar",
    "TuiPrototype",
    "box_bottom",
    "box_double_horizontal",
    "box_horizontal",
    "box_t_junction",
    "box_vertical",
    "create_pt_application",
    "get_builtin_registry",
    "render_screen",
    "run_tui_demo",
    "SAMPLE_ADDRESS_URI",
    "SAMPLE_CLAIM_DISPOSITION",
    "SAMPLE_COMMANDS",
    "SAMPLE_DIALOG",
    "SAMPLE_DISPOSITION_REASON",
    "SAMPLE_MENUS",
    "SAMPLE_NEXT_ACTIONS",
    "SAMPLE_PROPERTIES",
    "SAMPLE_SHORTCUTS",
    "SAMPLE_SOURCE_TABLE",
    "SAMPLE_SOURCE_TREE",
    "SAMPLE_STATUS_ITEMS",
]
