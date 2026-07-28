"""prompt_toolkit integration for GAK-UI-001 TUI prototype.

Wraps :class:`TuiPrototype` in a :mod:`prompt_toolkit` Application
that provides alternate-screen management, key bindings, and redraw
scheduling.

Architecture boundary (from :file:`architecture/CLI_UI_INTERACTION_MODEL_v0.1.md`):

    prompt_toolkit owns terminal mechanics.
    GSA retro widgets own interaction semantics.
    Assurance core owns behaviour and emits structured events.

This module is the bridge between the first two layers.  It translates
prompt_toolkit key events into the string-based dispatch interface of
:class:`TuiPrototype` and triggers re-renders on every state change.

Alt+letter handling
-------------------
Terminal emulators send Alt+<letter> as ESC followed by the letter
character (e.g. Alt+F → ``\\x1b f``).  prompt_toolkit represents this
as a two-key sequence ``('escape', '<letter>')``.  To avoid an
annoying delay on single-Escape (dismiss / back), we set
``ttimeoutlen`` to 100 ms — short enough to feel instantaneous, long
enough for terminals to deliver the second byte of an Alt sequence.
"""

from __future__ import annotations

from typing import Any

from prompt_toolkit import Application
from prompt_toolkit.key_binding import KeyBindings
from prompt_toolkit.layout import Layout, Window
from prompt_toolkit.layout.controls import FormattedTextControl

# ── helpers ──────────────────────────────────────────────────────────────────


def _modal_active(tui_app: Any) -> bool:
    """Return True if a modal overlay is consuming keyboard input."""
    return bool(tui_app.dialog.visible or tui_app.properties.visible)


# ── Application builder ──────────────────────────────────────────────────────


def create_pt_application(
    tui_app: Any,
    width: int = 100,
    height: int = 30,
) -> Application[Any]:
    """Build a prompt_toolkit :class:`Application` that hosts *tui_app*.

    Parameters
    ----------
    tui_app:
        A configured :class:`TuiPrototype` instance.
    width / height:
        Terminal viewport dimensions in cells.  These are *fixed* for the
        lifetime of the application (dynamic resize is deferred to a later
        iteration).

    Returns
    -------
    :
        A prompt_toolkit Application ready for :meth:`Application.run`.
    """

    # ── content provider ──────────────────────────────────────────────────

    def _get_content_text() -> str:
        return tui_app.render(width, height)

    content_control = FormattedTextControl(_get_content_text, focusable=True)

    # ── key bindings ──────────────────────────────────────────────────────

    kb = KeyBindings()

    # -- run toggle (F5, demo only) ----------------------------------------
    @kb.add("f5")
    def _f5(event: Any) -> None:
        result = tui_app.handle_key("f5")
        event.app.invalidate()

    # -- cancel run (Ctrl+Z, universal undo/retract) ------------------------
    @kb.add("c-z")
    def _cancel(event: Any) -> None:
        result = tui_app.handle_key("c-z")
        event.app.invalidate()

    # -- focus cycling (F6) -------------------------------------------------
    @kb.add("f6")
    def _f6(event: Any) -> None:
        tui_app.handle_key("f6")
        event.app.invalidate()

    # -- dismiss / back (Escape alone; fires after ttimeoutlen delay) -------
    @kb.add("escape")
    def _esc(event: Any) -> None:
        tui_app.handle_key("esc")
        event.app.invalidate()

    # -- focus movement within pane -----------------------------------------
    @kb.add("tab")
    def _tab(event: Any) -> None:
        tui_app.handle_key("tab")
        event.app.invalidate()

    @kb.add("s-tab")
    def _stab(event: Any) -> None:
        tui_app.handle_key("s-tab")
        event.app.invalidate()

    # -- activate -----------------------------------------------------------
    @kb.add("enter")
    def _enter(event: Any) -> None:
        tui_app.handle_key("enter")
        event.app.invalidate()

    @kb.add("s-enter")
    def _s_enter(event: Any) -> None:
        tui_app.handle_key("s-enter")
        event.app.invalidate()

    # -- dialog toggle (d) — only when address bar is NOT focused ---------
    @kb.add("d")
    def _dialog(event: Any) -> None:
        if tui_app.active_pane != "address":
            if not _modal_active(tui_app):
                tui_app.dialog.visible = not tui_app.dialog.visible
        else:
            tui_app.handle_key("d")
        event.app.invalidate()

    # -- properties toggle (p) — only when address bar is NOT focused ------
    @kb.add("p")
    def _props(event: Any) -> None:
        if tui_app.active_pane != "address":
            if not _modal_active(tui_app):
                tui_app.properties.visible = not tui_app.properties.visible
        else:
            tui_app.handle_key("p")
        event.app.invalidate()

    # -- arrow keys (delegate to active pane) -------------------------------
    @kb.add("up")
    def _up(event: Any) -> None:
        tui_app.handle_key("up")
        event.app.invalidate()

    @kb.add("down")
    def _down(event: Any) -> None:
        tui_app.handle_key("down")
        event.app.invalidate()

    @kb.add("left")
    def _left(event: Any) -> None:
        tui_app.handle_key("left")
        event.app.invalidate()

    @kb.add("right")
    def _right(event: Any) -> None:
        tui_app.handle_key("right")
        event.app.invalidate()

    # -- quit ---------------------------------------------------------------
    @kb.add("q")
    @kb.add("c-c")
    def _quit(event: Any) -> None:
        event.app.exit()

    # -- backspace ---------------------------------------------------------
    @kb.add("backspace")
    def _backspace(event: Any) -> None:
        tui_app.handle_key("backspace")
        event.app.invalidate()

    # -- space (for search queries etc.) -----------------------------------
    @kb.add(" ")
    def _space(event: Any) -> None:
        tui_app.handle_key("space")
        event.app.invalidate()

    # -- slash → open command palette ---------------------------------------
    @kb.add("/")
    def _slash(event: Any) -> None:
        tui_app.handle_key("/")
        event.app.invalidate()

    # -- printable letters + digits (common typing keys) --------------------
    _TYPING_KEYS: str = (
        "abcdefghijklmnopqrstuvwxyz"
        "ABCDEFGHIJKLMNOPQRSTUVWXYZ"
        "0123456789"
        "._-:="
    )

    def _bind_typing(char: str) -> None:
        @kb.add(char)
        def _handler(event: Any) -> None:
            tui_app.handle_key(char)
            event.app.invalidate()

    for _ch in _TYPING_KEYS:
        _bind_typing(_ch)

    # -- Alt+letter → menu activation ---------------------------------------
    # Terminal: Alt+<letter> is sent as ESC then <letter>.
    # We register each as a two-key sequence ('escape', '<letter>').
    # prompt_toolkit resolves the ambiguity between single 'escape' and
    # 'escape'+'<letter>' via a short timeout (ttimeoutlen).
    _ALT_LETTERS: dict[str, str] = {
        "f": "alt+f",
        "e": "alt+e",
        "s": "alt+s",
        "a": "alt+a",
        "r": "alt+r",
        "v": "alt+v",
        "h": "alt+h",
    }

    def _bind_alt_sequence(letter: str, key: str) -> None:
        """Register ('escape', 'letter') → key dispatch.

        A factory function ensures *letter* and *key* are captured by
        value via parameter binding, avoiding the classic loop-closure
        bug.
        """

        @kb.add("escape", letter)
        def _handler(event: Any) -> None:
            tui_app.handle_key(key)
            event.app.invalidate()

    for _letter, _key in _ALT_LETTERS.items():
        _bind_alt_sequence(_letter, _key)

    # -- ctrl+l → focus address bar (Explorer convention) -------------------
    @kb.add("c-l")
    def _focus_address(event: Any) -> None:
        while tui_app.active_pane != "address":
            tui_app.handle_key("f6")
        event.app.invalidate()

    # ── layout ────────────────────────────────────────────────────────────

    window = Window(content=content_control, always_hide_cursor=False)
    layout = Layout(window)

    # ── application ───────────────────────────────────────────────────────

    app: Application[Any] = Application(
        layout=layout,
        key_bindings=kb,
        full_screen=True,
        mouse_support=False,
    )
    # Minimise the delay before a bare Escape is recognised (default 1.0 s
    # is too sluggish for dialog-dismiss feedback).  100 ms is short enough
    # not to be noticed yet still lets terminals deliver ESC+letter as two
    # distinct bytes.
    app.ttimeoutlen = 0.1
    return app


# ── convenience entry point ──────────────────────────────────────────────────


def run_tui_demo(
    tui_app: Any,
    width: int = 100,
    height: int = 30,
) -> None:
    """Build and run the full-screen TUI demo.

    This is the primary interactive entry point for the first prototype.
    Press ``q`` or ``Ctrl+C`` to exit.
    """
    app = create_pt_application(tui_app, width=width, height=height)
    app.run()
