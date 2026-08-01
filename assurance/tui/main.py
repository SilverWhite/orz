"""Entry point for the GAK-UI-001 TUI prototype.

Usage::

    python -m assurance.tui.main              # render at 100×30
    python -m assurance.tui.main -w 140 -h 40 # render at 140×40
    python -m assurance.tui.main --demo       # run interactive prompt_toolkit demo
    python -m assurance.tui.main --demo --live-retrieval
        # run interactive demo with live browser retrieval wired
"""

from __future__ import annotations

from typing import Any, Callable

from .events import RetrievalOutcome, RetrievalProgress


# ── retrieval handler wiring (LBR-001) ────────────────────────────────────────


def build_retrieval_handler(
) -> Callable[[str, str, Callable[[RetrievalProgress], None]], RetrievalOutcome]:
    """Build a retrieval handler wired to the real assurance workflow.

    This function is the **only** place where the TUI layer touches
    ``assurance.retrieval_workflow``.  It lives in the application entry
    point (:mod:`assurance.tui.main`) rather than in the TUI rendering
    layer, satisfying the architectural constraint that TUI widgets
    must not import from the assurance core.

    Returns a callable with signature::

        (mode: str, target: str, on_progress: Callable) -> RetrievalOutcome

    where *mode* is ``"search"`` or ``"retrieve"``.
    """
    from assurance.retrieval_workflow import (
        RetrievalProgress as AssuranceRetrievalProgress,
        RetrievalResult,
        WebRetrievalResult,
        retrieve_search,
        run_retrieval,
    )

    def _adapt_progress(ap: AssuranceRetrievalProgress) -> RetrievalProgress:
        """Map the assurance progress type to the TUI-owned type."""
        return RetrievalProgress(
            stage=ap.stage,
            message=ap.message,
            detail=ap.detail,
            timestamp=ap.timestamp,
        )

    def _handler(
        mode: str,
        target: str,
        on_progress: Callable[[RetrievalProgress], None],
    ) -> RetrievalOutcome:
        if mode == "search":
            raw: WebRetrievalResult = retrieve_search(
                target,
                launch_browser=True,
                headless=False,
                on_progress=lambda ap: on_progress(_adapt_progress(ap)),
                max_results=3,
            )
            return RetrievalOutcome(
                ok=True,
                title="SEARCH DONE",
                detail=f"{len(raw.pages)} pages, {raw.total_chars} chars",
                summary=f"{len(raw.pages)} pages from web search",
            )
        else:
            raw_result: RetrievalResult = run_retrieval(
                target,
                launch_browser=True,
                headless=False,
                on_progress=lambda ap: on_progress(_adapt_progress(ap)),
            )
            if raw_result.error:
                return RetrievalOutcome(
                    ok=False,
                    title="FAILED",
                    error=raw_result.error,
                    detail=raw_result.error,
                )
            return RetrievalOutcome(
                ok=True,
                title="RETRIEVED",
                detail=(
                    f"{raw_result.title[:80]} — "
                    f"{raw_result.page_count}pp, {raw_result.total_chars} chars"
                ),
                summary=raw_result.title,
            )

    return _handler


def _load_mcp_config(path: str) -> list[dict[str, Any]]:
    """Load MCP server configurations from a JSON file.

    Expected format: a JSON array of objects, each with:
    ``name`` (str), ``command`` (str), ``args`` (list[str], optional),
    ``env`` (dict[str, str], optional).
    """
    import json as _json
    config_path = Path(path)
    if not config_path.is_file():
        raise FileNotFoundError(f"MCP config file not found: {path}")
    with config_path.open("r", encoding="utf-8") as handle:
        data = _json.load(handle)
    if not isinstance(data, list):
        raise ValueError("MCP config must be a JSON array of server objects")
    for i, entry in enumerate(data):
        if not isinstance(entry, dict):
            raise ValueError(f"MCP config entry {i} must be an object")
        if "name" not in entry or "command" not in entry:
            raise ValueError(
                f"MCP config entry {i} must have 'name' and 'command' fields"
            )
    return data


def main(argv: list[str] | None = None) -> int:
    import argparse
    import sys

    parser = argparse.ArgumentParser(
        prog="gsa-tui-prototype",
        description="GAK-UI-001 retro desktop TUI first prototype",
    )
    parser.add_argument(
        "-w", "--width", type=int, default=100,
        help="Viewport width in cells (default: 100)",
    )
    parser.add_argument(
        "-H", "--height", type=int, default=30,
        help="Viewport height in rows (default: 30)",
    )
    parser.add_argument(
        "--demo", action="store_true",
        help="Run interactive prompt_toolkit demo (press 'q' to quit)",
    )
    parser.add_argument(
        "--with-dialog", action="store_true",
        help="Show the permission dialog overlay in static render",
    )
    parser.add_argument(
        "--with-properties", action="store_true",
        help="Show the properties sheet overlay in static render",
    )
    parser.add_argument(
        "--demo-events", type=str, default=None, metavar="SCENARIO",
        choices=["gate_chain", "gate_defer", "run_failed"],
        help="Run interactive demo with a live event scenario",
    )
    parser.add_argument(
        "--live-retrieval", action="store_true",
        help="Wire live browser retrieval handler (requires Chrome on port 9222)",
    )
    parser.add_argument(
        "--replay", type=str, default=None, metavar="JOURNAL_PATH",
        help="Replay events from an existing events.jsonl file",
    )
    parser.add_argument(
        "--run", type=str, default=None, metavar="ASK_TEXT",
        help="Run the selected runtime and display it in TUI",
    )
    parser.add_argument(
        "--runtime", type=str, default="canonical",
        choices=["canonical", "grok"],
        help="Runtime for --run (default: canonical)",
    )
    parser.add_argument(
        "--real", action="store_true",
        help="Use the real DeepSeek adapter for --run (requires API key "
             "in Windows Credential Manager)",
    )
    parser.add_argument(
        "--credential-target", type=str, default="FEP-Agent/DeepSeek",
        metavar="TARGET",
        help="Windows Credential Manager target name for DeepSeek API key "
             "(default: FEP-Agent/DeepSeek)",
    )
    parser.add_argument(
        "--run-root", type=str, default=None,
        help="Run root directory for --run (default: auto-generated temp dir)",
    )
    parser.add_argument(
        "--workspace", type=str, default=None,
        help="Workspace directory for Grok --run (default: isolated directory under run root)",
    )
    parser.add_argument(
        "--retrieval-mode",
        type=str,
        default=None,
        choices=["local_browser", "framework_fallback", "off"],
        help=(
            "Explicit retrieval mode for Grok prompt/tool runs. "
            "version-smoke records the selection but never performs retrieval."
        ),
    )
    parser.add_argument(
        "--mcp-config",
        type=str,
        default=None,
        metavar="PATH",
        help="Path to a JSON file containing MCP server configurations "
             "(array of {name, command, args, env} objects). "
             "Forwarded to Grok's session/new mcpServers parameter.",
    )

    args = parser.parse_args(argv)

    from .app import TuiPrototype

    # Choose construction path
    if args.demo_events:
        from .event_source import (
            FakeEventSource,
            build_gate_chain_demo,
            build_gate_defer_demo,
            build_run_failed_demo,
        )
        scenarios = {
            "gate_chain": build_gate_chain_demo,
            "gate_defer": build_gate_defer_demo,
            "run_failed": build_run_failed_demo,
        }
        events = scenarios[args.demo_events]()
        source = FakeEventSource(preload=events)
        app = TuiPrototype.with_event_source(source)
        # Also open dialog/properties if requested
        if args.with_dialog:
            app.dialog.visible = True
        if args.with_properties:
            app.properties.visible = True
    else:
        app = TuiPrototype.with_sample_data()
        if args.with_dialog:
            app.dialog.visible = True
        if args.with_properties:
            app.properties.visible = True

    # Wire live retrieval handler if requested
    if args.live_retrieval:
        handler = build_retrieval_handler()
        app.retrieval_handler = handler

    # ── --replay: replay events from a journal file ──────────────────────
    if args.replay:
        from .event_source import JsonlFileSource
        source = JsonlFileSource(args.replay)
        app = TuiPrototype.with_event_source(source)
        if args.with_dialog:
            app.dialog.visible = True
        if args.with_properties:
            app.properties.visible = True
        if args.live_retrieval:
            handler = build_retrieval_handler()
            app.retrieval_handler = handler
        return _run_demo(app, args.width, args.height)

    # ── --run: live CLI run ─────────────────────────────────────────────
    if args.run:
        import tempfile
        from pathlib import Path
        run_root = Path(args.run_root) if args.run_root else Path(
            tempfile.mkdtemp(prefix="gsa-run-")
        )
        from .bridge import build_grok_acp_live_run_fn, build_grok_live_run_fn, build_live_run_fn
        from .event_source import LiveRunEventSource
        extra_kwargs: dict[str, Any] = {}
        if args.runtime == "grok":
            if args.real:
                print("--real is only valid with --runtime canonical", file=sys.stderr)
                return 2
            if args.run in {"version-smoke", "version_smoke"}:
                run_fn = build_grok_live_run_fn(
                    run_root=str(run_root),
                    workspace=args.workspace,
                    retrieval_mode=args.retrieval_mode or "off",
                    retrieval_mode_explicit=args.retrieval_mode is not None,
                )
            else:
                # ACP live session — real-time event streaming from grok agent stdio
                # Interactive mode: user approves/denies tool calls via TUI dialog
                mcp_servers = _load_mcp_config(args.mcp_config) if args.mcp_config else None
                result = build_grok_acp_live_run_fn(
                    run_root=str(run_root),
                    workspace=args.workspace,
                    prompt_text=args.run,
                    retrieval_mode=args.retrieval_mode or "off",
                    retrieval_mode_explicit=args.retrieval_mode is not None,
                    interactive=True,
                    mcp_servers=mcp_servers,
                )
                if isinstance(result, tuple):
                    run_fn, permission_queue, prompt_queue = result
                else:
                    run_fn = result
                    permission_queue = None
                    prompt_queue = None
        else:
            if args.retrieval_mode is not None:
                print("--retrieval-mode is only valid with --runtime grok", file=sys.stderr)
                return 2
            if args.real:
                extra_kwargs["credential_target"] = args.credential_target
            run_fn = build_live_run_fn(
                run_root=str(run_root),
                ask=args.run,
                real_adapter=args.real,
                **extra_kwargs,
            )
        source = LiveRunEventSource(run_fn=run_fn, permission_queue=permission_queue, prompt_queue=prompt_queue)
        app = TuiPrototype.with_event_source(source)
        if args.with_dialog:
            app.dialog.visible = True
        if args.with_properties:
            app.properties.visible = True
        if args.live_retrieval:
            handler = build_retrieval_handler()
            app.retrieval_handler = handler
        source.start()
        return _run_demo(app, args.width, args.height)

    if args.demo or args.demo_events:
        return _run_demo(app, args.width, args.height)

    output = app.render(args.width, args.height)
    print(output)
    return 0


def _run_demo(app: object, width: int, height: int, banner: bool = True) -> int:
    """Run the interactive full-screen TUI via prompt_toolkit.

    Set *banner* to False to skip the startup message (production entry).
    """
    # Ensure Windows console uses UTF-8 so box-drawing characters
    # render correctly.  Without this, legacy code pages (CP437/CP936)
    # map these codepoints to missing-glyph placeholders.
    import sys as _sys
    if _sys.platform == "win32":
        try:
            import ctypes as _ctypes
            _ctypes.windll.kernel32.SetConsoleCP(65001)
            _ctypes.windll.kernel32.SetConsoleOutputCP(65001)
        except Exception:
            pass

    try:
        from .pt_app import run_tui_demo  # noqa: PLC0415
    except ImportError:
        print(
            "[GSA] WARNING: prompt_toolkit is not installed. "
            "Falling back to raw stdin demo.\n"
            "Install with: pip install prompt_toolkit>=3.0\n",
        )
        return _run_raw_demo(app, width, height)

    if banner:
        print("GSA TUI")
        print(f"Viewport: {width}x{height}")
        print("Keys: f6=cycle focus, alt+letter=menu, d=dialog, p=properties, q=quit")
        print("=" * min(width, 80))

    run_tui_demo(app, width=width, height=height)
    return 0


def _run_raw_demo(app: object, width: int, height: int) -> int:
    """Simple stdin-driven demo loop.  Press 'q' or Ctrl+C to exit.

    This is the fallback when prompt_toolkit is unavailable.
    """
    import sys

    while True:
        output = app.render(width, height)
        sys.stdout.write("\033[H\033[J")  # clear screen
        sys.stdout.write(output)
        sys.stdout.write(f"\n{'─' * width}\n")
        sys.stdout.write("Key? ")
        sys.stdout.flush()

        try:
            key = sys.stdin.readline().strip()
        except (EOFError, KeyboardInterrupt):
            break
        if key in ("q", "quit", "exit"):
            break
        if key == "d":
            app.dialog.visible = not app.dialog.visible
        elif key == "p":
            app.properties.visible = not app.properties.visible
        else:
            result = app.handle_key(key)
            if result:
                sys.stdout.write(f"  → {result}\n")

    return 0


if __name__ == "__main__":
    import sys
    sys.exit(main())
