from __future__ import annotations

import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]


def test_live_api_script_is_inert_without_explicit_opt_in() -> None:
    completed = subprocess.run(
        [sys.executable, str(ROOT / "scripts" / "test_real_api_call.py")],
        cwd=ROOT,
        capture_output=True,
        text=True,
        timeout=10,
        check=False,
    )
    assert completed.returncode == 2
    assert "without --live" in completed.stdout


def test_default_pytest_configuration_excludes_manual_scripts() -> None:
    pytest_configuration = (ROOT / "pytest.ini").read_text(encoding="utf-8")
    norecursedirs = next(
        line for line in pytest_configuration.splitlines() if line.startswith("norecursedirs")
    )
    excluded = norecursedirs.split("=", 1)[1].split()
    assert "scripts" in excluded
    assert ".tmp-*" in excluded
