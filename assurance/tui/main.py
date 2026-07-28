"""Entry point for the GAK-UI-001 TUI prototype.

Usage::

    python -m assurance.tui.main              # render at 100×30
    python -m assurance.tui.main -w 140 -h 40 # render at 140×40
    python -m assurance.tui.main --demo       # run interactive prompt_toolkit demo
"""

from __future__ import annotations


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

    if args.demo or args.demo_events:
        return _run_demo(app, args.width, args.height)

    output = app.render(args.width, args.height)
    print(output)
    return 0


def _run_demo(app: object, width: int, height: int) -> int:
    """Run the interactive full-screen demo via prompt_toolkit.

    Falls back to the raw stdin loop (with a warning) if prompt_toolkit
    is not importable.
    """
    try:
        from .pt_app import run_tui_demo  # noqa: PLC0415
    except ImportError:
        print(
            "[GSA] WARNING: prompt_toolkit is not installed. "
            "Falling back to raw stdin demo.\n"
            "Install with: pip install prompt_toolkit>=3.0\n",
        )
        return _run_raw_demo(app, width, height)

    print("GAK-UI-001 TUI Prototype — Interactive Demo")
    print(f"Viewport: {width}×{height}")
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
