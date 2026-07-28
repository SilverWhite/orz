# CLI UI Interaction Model

## Purpose

This document records the proposed UI direction for the Windows-only scientific assurance CLI.

The goal is not to copy a nostalgic color theme. The goal is to reuse a mature desktop interaction grammar from the Windows 95/98, Windows 2000, and early Internet Explorer era, then adapt it to modern agent workflows.

Modern agent CLIs often collapse many different concepts into one vertical text stream:

- model output
- tool calls
- permission requests
- source visibility
- logs
- errors
- current plan
- verification state
- artifacts

That makes the user keep too much state in their head. The proposed UI treats the CLI as an object-oriented workspace browser instead of a chat-like scrollback.

## Design Thesis

Use an Explorer / old-IE style terminal UI as the spatial language for agent work.

The interface should make these questions continuously visible:

- Where am I?
- What object am I operating on?
- What sources are visible?
- What state is the run in?
- Which actions are available?
- What is blocking progress?
- What has been verified?

The retro desktop style is useful because it already has mature answers for command discovery, object navigation, status display, modal decisions, and properties inspection.

## Global Regions

### MenuBar

Purpose: full command discovery.

Example menus:

- File
- Edit
- View
- Sources
- Agent
- Run
- Verify
- Help

Menus should contain the complete command set. A command should not exist only as a toolbar button.

### Toolbar

Purpose: frequent immediate actions.

Candidate actions:

- Back
- Forward
- Stop
- Refresh
- Open
- Run
- Verify
- Properties

The toolbar should stay small. New features can start in menus and only move to the toolbar after they prove frequent enough.

### AddressBar

Purpose: current object location.

The address bar should not be limited to filesystem paths. It can represent agent-native objects:

```text
workspace://LIF/current-index
source://R211/mem-outlier
run://2026-07-27/0004
claim://mem-channel-z-information
adapter://deepseek/observation
artifact://results/R211_summary.json
```

Back and Forward should navigate object history, not merely file history.

The address row should also be the direct-command entry point for commands that intentionally bypass AI interpretation.

Preferred layout:

```text
Address: command://run/current-task
Find:    [ source visibility              ] in [ Conversation v]
```

Semantic split:

- Address / Command changes system state or navigates to an object.
- Find searches content.
- Scope limits the search range.

Direct commands such as `/run`, `/kill`, `/verify`, `/open`, and `/back` should map to explicit `command://...` addresses. They should not be mixed into the search scope selector.

Recommended search scopes:

- Conversation
- Project Docs
- Current File
- Runs / Artifacts
- Indexed Sources

### ExplorerPane

Purpose: stable navigation over workspaces, sources, runs, artifacts, and verification objects.

Candidate root groups:

- Workspaces
- Tasks
- Sources
- INDEX / MAP
- R Series
- Runs
- Artifacts
- Adapters
- Verifier Receipts

The pane should help the user move through the evidence graph without turning the main area into a long log stream.

Structured events should live here instead of overloading the right-side content scroll marker. Candidate groups:

- User Inputs
- Decisions
- Errors
- Permissions
- Verifier
- Artifacts
- Sources

### ContentPane

Purpose: the currently selected object or active task.

Examples:

- active task view
- source preview
- claim status
- run summary
- artifact viewer
- verifier comparison
- permission request explanation

Long raw logs should not dominate this pane by default. They belong in a dedicated log/history view.

### StatusBar

Purpose: passive but important runtime state.

Examples:

```text
GUARDED | NET OFF | SBX STRICT | SOURCES 3/4 | DEEPSEEK | RUNNING
```

Suitable status bar items:

- mode: guarded / strict / dry-run
- sandbox state
- network state
- active model or adapter
- source visibility count
- current run state
- token or budget state

Urgent errors must not appear only in the status bar, because status bars are easy to miss.

### Dialog

Purpose: blocking decisions only.

Dialogs are appropriate for:

- permission approval
- source provenance failure
- verifier disagreement
- strict sandbox unavailable
- irreversible or high-risk action
- conflict that cannot be resolved automatically

Dialogs should not be used for ordinary logs, non-blocking status, or informational noise.

### Properties

Purpose: structured metadata for the selected object.

Properties should provide a consistent inspection surface across object types.

Suggested tabs:

- General
- Provenance
- Visibility
- Dependencies
- Verification
- History

This lets the user inspect a source, claim, run, artifact, adapter, or verifier receipt with one familiar interaction pattern.

## Object Types

The UI should be organized around objects, not transcript lines.

Primary object types:

- Workspace
- Task
- Source
- Claim
- Run
- Artifact
- Adapter
- JournalEvent
- VerifierReceipt
- PermissionRequest

Each object should support a small set of standard actions where applicable.

## Standard Actions

Common actions:

- Open
- Back
- Forward
- Refresh
- Stop
- Run
- Verify
- Approve
- Reject
- Properties
- Copy Path
- Open Source
- Show History
- Show Log

The same action should feel consistent across object types.

For example:

- Properties on a Source shows provenance and visibility.
- Properties on a Run shows lifecycle, adapter, artifacts, and verifier state.
- Properties on a Claim shows evidence, boundary status, and verification.

## Display Rules

- Background state goes to StatusBar.
- Recoverable errors go to ContentPane.
- User-required decisions go to Dialog.
- Evidence details go to Properties.
- Complete logs go to Log/History views.
- The main task view should show current state and next useful actions, not raw terminal noise.
- Severe failures must be visually distinct from ordinary log entries.
- Source visibility and verification state should remain inspectable without scrolling.
- The right-side content marker should stay simple: user input positions and current search hits only.
- Complex event categories belong in the ExplorerPane, not in a crowded scroll marker.

## Suggested Keyboard Model

Use old desktop conventions where they fit.

Candidate bindings:

```text
Alt+F        File menu
Alt+S        Sources menu
Alt+R        Run menu
Alt+V        Verify menu
F6           Cycle major panes
Tab          Move focus within current pane
Shift+Tab    Move focus backward within current pane
Enter        Activate focused item
Esc          Close dialog, cancel, or go back one UI layer
Ctrl+L       Focus address bar
Alt+Left     Back
Alt+Right    Forward
F5           Refresh or re-verify current object
Ctrl+Break   Stop active agent run
```

The goal is to reduce project-specific keybinding invention and reuse existing keyboard memory.

## Example Layout

```text
┌─ GSA ───────────────────────────────────────────────────────────┐
│ File  Edit  View  Sources  Agent  Run  Verify  Help             │
├─────────────────────────────────────────────────────────────────┤
│ Back  Forward  Stop  Refresh  Open  Verify  Properties           │
├─────────────────────────────────────────────────────────────────┤
│ Address: workspace://LIF/current                         [ Go ]  │
├──────────────────┬──────────────────────────────────────────────┤
│ Source Explorer  │ Current Task                                 │
│                  │                                              │
│ ▾ INDEX          │  Source Visibility                           │
│ ▾ MAP            │  - MAP_MEM.md                 VERIFIED       │
│ ▾ R Series       │  - R211.md                   PARTIAL         │
│ ▾ Artifacts      │  - run_2026_06_26.json       VERIFIED       │
│                  │                                              │
│ ▾ Adapters       │  Claim disposition: DEFER                    │
│ ▾ Verifier       │  Missing: orthogonal decoder final result     │
├──────────────────┴──────────────────────────────────────────────┤
│ GUARDED | NET OFF | SBX STRICT | SOURCES 3/4 | IDLE             │
└─────────────────────────────────────────────────────────────────┘
```

## Architecture Boundary

The UI must not become the assurance kernel.

Preferred flow:

```text
User input
  -> UI command
  -> canonical CLI / assurance core
  -> canonical events / state projection
  -> UI view model
  -> retro terminal renderer
```

Core modules should emit structured events. The UI should decide how to display them.

Example event:

```json
{
  "event_type": "gate_decision",
  "gate": "source_visibility",
  "decision": "defer",
  "reason_code": "SOURCE_FULLTEXT_MISSING"
}
```

Possible UI presentation:

```text
┌─ Source Warning ─────────────────────────┐
│ The selected source is only partially    │
│ visible. Claim promotion was deferred.   │
│                                          │
│       [ View Source ]   [ Close ]        │
└──────────────────────────────────────────┘
```

## Implementation Notes

This direction is compatible with a Windows-only terminal application.

## Implementation Decision

Use `prompt_toolkit` plus a thin custom retro widget layer.

This is the preferred first implementation path because the project needs a Windows-only terminal UI with strong control over layout, focus, keyboard navigation, redraw behavior, subprocess output handling, and the retro desktop interaction model. The UI should feel like a character-cell Windows Explorer / old-IE workspace, not like a generic modern terminal dashboard.

`prompt_toolkit` should provide the terminal substrate:

- application lifecycle
- alternate-screen management
- input handling
- key bindings
- focus management
- mouse support where useful
- terminal output and redraw scheduling

The project-owned widget layer should provide the product language:

- MenuBar
- Toolbar
- AddressBar
- ExplorerPane
- ContentPane
- StatusBar
- Dialog
- Properties view
- old desktop border and focus behavior
- object navigation history

Textual remains a reasonable rapid-prototype alternative, but it is not the preferred implementation direction for this project. Its default style and component model are closer to a modern terminal dashboard, while this project needs a smaller, more controlled widget grammar that can preserve the Explorer / old-IE object-navigation model.

Implementation rule:

```text
prompt_toolkit owns terminal mechanics.
GSA retro widgets own interaction semantics.
Assurance core owns behavior and emits structured events.
```

The first usable UI should avoid coupling directly to model calls, subprocess streams, or assurance internals. It should render a view model projected from canonical events.

## Build Strategy

Do not start by cloning a specific old Internet Explorer, Windows 98, or Windows Explorer screen.

Instead, extract the mature interaction grammar from that era and build the GSA-specific skeleton directly.

Borrowed classic grammar:

- persistent menu bar for command discovery
- small toolbar for frequent immediate actions
- address row for object navigation and direct commands
- dedicated find row for scoped search
- left explorer pane for structured objects and event groups
- central content pane for the active task or selected object
- simple right-side content marker for user inputs and search hits
- status bar for passive runtime state
- dialogs for blocking decisions
- properties view for object metadata

GSA-specific model:

- Workspace
- Task
- Source
- Claim
- Run
- Artifact
- Adapter
- JournalEvent
- VerifierReceipt
- PermissionRequest

This avoids inheriting browser-specific baggage such as page loading, favorites, web history menus, or document metaphors that do not belong to the assurance workflow. The target is not a replica browser; it is an Explorer-style assurance workspace.

First implementation target:

```text
Classic desktop interaction grammar
+ GSA object model
+ prompt_toolkit terminal substrate
+ custom retro widgets
```

Recommended first implementation path:

- Use a terminal UI toolkit only for input, focus, terminal control, and screen drawing.
- Keep a thin custom widget layer for the retro desktop model.
- Render components from state rather than printing ad hoc text.
- Keep stdout/stderr from subprocesses captured and converted into events.
- Use an alternate screen buffer and restore terminal state on exit.
- Batch redraws instead of repainting every streamed token.

Important constraints:

- Account for Chinese character width with a real width calculator.
- Avoid unstable emoji-width assumptions.
- Prefer stable ASCII and box-drawing characters for core UI.
- Define a minimum viewport such as 100 x 30.
- Target Windows Terminal and Windows 11 first.

## Non-Goals

- Do not fully recreate a specific historical application screenshot.
- Do not make nostalgia override task clarity.
- Do not make color the core design dependency.
- Do not route core assurance behavior through UI code.
- Do not let every module print directly to the terminal.
- Do not turn logs, model text, and verification state back into one undifferentiated stream.

## First Prototype Scope

The first prototype can be static and disconnected from the assurance core.

Prototype goals:

- Main frame with menu, toolbar, address bar, explorer pane, content pane, and status bar.
- Object URI display in address bar.
- Direct command address example: `command://run/current-task`.
- Dedicated Find row with `Conversation` / `Project Docs` scope switching.
- A small fixed source tree.
- A right-side content marker showing only user inputs and current search hits.
- A task summary view.
- A properties dialog mock.
- A blocking permission dialog mock.
- Keyboard focus movement across major panes.
- Screenshot or snapshot tests at 100 x 30 and 140 x 40.

This is enough to validate the spatial model before wiring it to real agent state.
