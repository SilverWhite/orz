"""One-time setup: add --remote-debugging-port to your Chrome shortcut.

After running this, your normal Chrome will always have the CDP port open.
The CLI will connect to it automatically — no more closing/restarting.

Usage::

    python scripts/setup_chrome_cdp.py
"""

from __future__ import annotations

import sys
from pathlib import Path


def main() -> int:
    import os
    import win32com.client  # type: ignore

    # Find Chrome shortcut in Start Menu
    start_menu = Path(os.environ["APPDATA"]) / "Microsoft/Windows/Start Menu/Programs"
    candidates = [
        start_menu / "Google Chrome.lnk",
        Path(os.environ["USERPROFILE"]) / "Desktop/Google Chrome.lnk",
    ]

    shortcut_path = None
    for c in candidates:
        if c.is_file():
            shortcut_path = c
            break

    if shortcut_path is None:
        print("Cannot find Chrome shortcut.")
        print(f"Searched: {[str(c) for c in candidates]}")
        return 1

    print(f"Found Chrome shortcut: {shortcut_path}")

    try:
        shell = win32com.client.Dispatch("WScript.Shell")
        shortcut = shell.CreateShortcut(str(shortcut_path))
        current_args = shortcut.Arguments or ""
        target = shortcut.TargetPath

        if "--remote-debugging-port" in current_args:
            print("Shortcut already has --remote-debugging-port configured.")
            print(f"  Target: {target}")
            print(f"  Args: {current_args}")
            return 0

        new_args = f"{current_args} --remote-debugging-port=9222".strip()
        shortcut.Arguments = new_args
        shortcut.Save()

        print("Updated Chrome shortcut:")
        print(f"  Target: {target}")
        print(f"  Args: {new_args}")
        print()
        print("Done! Close all Chrome windows, then open Chrome normally.")
        print("The CLI will now auto-connect to your browser on port 9222.")

    except ImportError:
        print("pywin32 is required. Install with: pip install pywin32")
        return 1
    except Exception as exc:
        print(f"Failed to update shortcut: {exc}")
        print()
        print("Manual setup:")
        print('  1. Right-click Chrome shortcut → Properties')
        print('  2. In the Target field, add at the end:')
        print('     --remote-debugging-port=9222')
        print('  3. Example: "C:\\...\\chrome.exe" --remote-debugging-port=9222')
        return 1

    return 0


if __name__ == "__main__":
    sys.exit(main())
