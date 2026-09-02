#!/usr/bin/env python3
"""Run a command tree inside the Windows hardened run environment.

P0-0l-②: spawns an arbitrary command line (e.g. enforcement_probe.ps1 or an
orz command tree) under the arm's native Windows wall:

  control    — current token, Job-contained.
  non-admin  — restricted token (Administrators disabled/deny-only, six
               privileges removed, TokenVirtualizationAllowed=0).
  high-nist  — non-admin + LOW integrity + AppContainer (empty capabilities)
               + Job Object + %%TEMP%% redirect + egress wall.

Writes a run observation JSON (windows-native-sandbox-run-v0.1.schema.json)
to --output (default <workspace>/windows-native-run-observation.json).
"""
from __future__ import annotations

import argparse
import json
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from assurance.windows_sandbox import (  # noqa: E402
    WINDOWS_RUN_ARMS,
    run_windows_native_sandbox,
)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--workspace",
        type=Path,
        default=None,
        help="Disposable workspace with .assurance-p2-disposable.json marker",
    )
    parser.add_argument(
        "--arm",
        choices=WINDOWS_RUN_ARMS,
        default="high-nist",
        help="Which hardened arm to spawn under (default: high-nist)",
    )
    parser.add_argument(
        "--command",
        nargs=argparse.REMAINDER,
        required=True,
        help=(
            "Command to spawn (argv list; no shell).  Must be the LAST "
            "option: every remaining argument is taken verbatim (allows "
            "flags like -NoProfile in the child command)."
        ),
    )
    parser.add_argument(
        "--cwd",
        type=Path,
        default=None,
        help="Working directory for the child (default: workspace)",
    )
    parser.add_argument(
        "--env",
        action="append",
        default=[],
        metavar="KEY=VALUE",
        help="Extra environment overrides (repeatable)",
    )
    parser.add_argument(
        "--env-file",
        type=Path,
        default=None,
        metavar="PATH",
        help=(
            "JSON object of environment overrides to inject (keeps secrets "
            "out of argv; file is deleted after a successful read)"
        ),
    )
    parser.add_argument(
        "--allowlist-ip",
        action="append",
        default=[],
        metavar="IP",
        help="Per-task egress allowlist IP (repeatable; high-nist)",
    )
    parser.add_argument(
        "--timeout",
        type=int,
        default=None,
        help="Wall-clock timeout seconds (default: profile value)",
    )
    parser.add_argument(
        "--memory",
        type=int,
        default=None,
        help="Job Object memory limit bytes (default: profile value)",
    )
    parser.add_argument(
        "--no-capture",
        action="store_true",
        help="Do not capture child stdout/stderr",
    )
    parser.add_argument(
        "--no-appcontainer",
        action="store_true",
        help=(
            "high-nist only: keep the strict wall (non-admin + LOW IL + Job + "
            "TEMP redirect + egress allowlist) without the AppContainer layer "
            "(2026-09-03 ruling for orz.exe loader compatibility)"
        ),
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=None,
        help="Run observation JSON path (default: <workspace>/windows-native-run-observation.json)",
    )
    args = parser.parse_args()

    if args.no_appcontainer and args.arm != "high-nist":
        parser.error("--no-appcontainer requires --arm high-nist")

    env_overrides: dict[str, str] = {}
    if args.env_file is not None:
        try:
            payload = json.loads(args.env_file.read_text(encoding="utf-8"))
        except (OSError, ValueError) as exc:
            parser.error(f"--env-file {args.env_file} unreadable/invalid: {exc}")
        if not isinstance(payload, dict) or not all(
            isinstance(k, str) and isinstance(v, str) for k, v in payload.items()
        ):
            parser.error("--env-file must map string keys to string values")
        env_overrides.update(payload)
        try:
            args.env_file.unlink(missing_ok=True)
        except OSError:
            pass
    for item in args.env:
        if "=" not in item:
            parser.error(f"--env requires KEY=VALUE, got {item!r}")
        key, value = item.split("=", 1)
        env_overrides[key] = value

    cleanup: tempfile.TemporaryDirectory[str] | None = None
    if args.workspace is None:
        cleanup = tempfile.TemporaryDirectory(prefix="w32-native-run-")
        workspace = Path(cleanup.name)
        marker = {
            "schema_version": "0.1.0-draft",
            "purpose": "windows-native-sandbox-probe",
            "allow_container_write_probe": True,
        }
        (workspace / ".assurance-p2-disposable.json").write_text(
            json.dumps(marker, indent=2),
            encoding="utf-8",
        )
    else:
        workspace = args.workspace

    try:
        observation = run_windows_native_sandbox(
            args.command,
            workspace,
            arm=args.arm,
            cwd=args.cwd,
            env=env_overrides,
            timeout_seconds=args.timeout,
            memory_limit_bytes=args.memory,
            allowlist_ips=args.allowlist_ip,
            capture_output=not args.no_capture,
            appcontainer=not args.no_appcontainer,
        )
        output = args.output or (
            workspace / "windows-native-run-observation.json"
        )
        output.write_text(
            json.dumps(observation, indent=2, sort_keys=True),
            encoding="utf-8",
        )
        summary = {
            "observation_path": str(output),
            "arm": observation["arm"],
            "outcome": observation["outcome"],
            "exit_code": observation["process"]["exit_code"],
            "timed_out": observation["process"]["timed_out"],
            "checks": observation["checks"],
            "stdout_bytes": observation["output"]["stdout_bytes"],
            "stderr_bytes": observation["output"]["stderr_bytes"],
        }
        print(json.dumps(summary, indent=2, sort_keys=True))
        return 0
    finally:
        if cleanup is not None:
            cleanup.cleanup()


if __name__ == "__main__":
    sys.exit(main())
