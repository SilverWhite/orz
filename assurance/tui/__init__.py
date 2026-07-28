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
from .events import (
    ArtifactRegisteredEvent,
    ErrorEvent,
    GateDecisionEvent,
    ModelOutputEvent,
    ModelRequestEvent,
    PermissionDecisionEvent,
    RunCancelledEvent,
    RunFailedEvent,
    RunFinishedEvent,
    RunPreflightEvent,
    RunStartedEvent,
    SourceVisibilityEvent,
    StatusUpdateEvent,
    TuiEvent,
    TuiEventKind,
    is_terminal,
)
from .event_source import (
    EventSource,
    FakeEventSource,
    JsonlFileSource,
    build_gate_chain_demo,
    build_gate_defer_demo,
    build_run_failed_demo,
)
from .projector import apply_event
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
    "ArtifactRegisteredEvent",
    "CommandDef",
    "CommandPalette",
    "CommandRegistry",
    "ContentMarker",
    "ContentPane",
    "Dialog",
    "ErrorEvent",
    "EventSource",
    "ExplorerPane",
    "FakeEventSource",
    "FindBar",
    "GateDecisionEvent",
    "JsonlFileSource",
    "MenuBar",
    "ModelOutputEvent",
    "ModelRequestEvent",
    "PermissionDecisionEvent",
    "PropertiesSheet",
    "RunCancelledEvent",
    "RunFailedEvent",
    "RunFinishedEvent",
    "RunPreflightEvent",
    "RunStartedEvent",
    "SourceVisibilityEvent",
    "StatusBar",
    "StatusUpdateEvent",
    "Toolbar",
    "TuiEvent",
    "TuiEventKind",
    "TuiPrototype",
    "apply_event",
    "box_bottom",
    "box_double_horizontal",
    "box_horizontal",
    "box_t_junction",
    "box_vertical",
    "build_gate_chain_demo",
    "build_gate_defer_demo",
    "build_run_failed_demo",
    "create_pt_application",
    "get_builtin_registry",
    "is_terminal",
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
