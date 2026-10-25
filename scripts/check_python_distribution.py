"""Build and smoke-test the install layout of the Python wheel."""

from __future__ import annotations

import argparse
import os
from shutil import copy2, copytree
import subprocess
import sys
import tempfile
import zipfile
from pathlib import Path


SMOKE_CODE = r"""
from pathlib import Path

import assurance
from assurance.cli import main
from assurance.evaluation_runner import protocol_ref_block
from assurance.layout import EVALUATION_ROOT, PROTOCOL_ROOT, REGRESSION_ROOT, RUNTIME_ROOT
from assurance.profile_registry import load_profile_registry

package_root = Path(assurance.__file__).resolve().parent
for resource_root in (RUNTIME_ROOT, PROTOCOL_ROOT, EVALUATION_ROOT, REGRESSION_ROOT):
    assert package_root in resource_root.parents, (package_root, resource_root)
assert (RUNTIME_ROOT / "run-event-v0.1.schema.json").is_file()
assert (PROTOCOL_ROOT / "reason-codes-v0.1.yaml").is_file()
assert (EVALUATION_ROOT / "SCORING_PROTOCOL_v0.1.md").is_file()
assert (REGRESSION_ROOT / "coverage-matrix-v0.1.yaml").is_file()
assert load_profile_registry()["profiles"]
assert all(value != "0" * 64 for key, value in protocol_ref_block().items() if key.endswith("sha256"))
try:
    main(["--help"])
except SystemExit as exc:
    assert exc.code == 0
"""


def stage_build_source(repository_root: Path, target: Path) -> None:
    """Copy only wheel build inputs so the smoke never dirties the repository."""

    target.mkdir(parents=True)
    for directory in ("assurance", "runtime", "protocol", "evaluation", "regression"):
        copytree(repository_root / directory, target / directory)
    for filename in (
        "pyproject.toml",
        "setup.py",
        "MANIFEST.in",
        "LICENSE",
        "NOTICE",
        "README.md",
    ):
        copy2(repository_root / filename, target / filename)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("wheel", nargs="?", type=Path)
    args = parser.parse_args()
    repository_root = Path(__file__).resolve().parents[1]
    with tempfile.TemporaryDirectory(prefix="gsa-wheel-smoke-") as temporary:
        temporary_root = Path(temporary)
        wheel = args.wheel
        if wheel is None:
            source_root = temporary_root / "source"
            stage_build_source(repository_root, source_root)
            subprocess.run(
                [sys.executable, "-m", "build", "--wheel", "--outdir", str(temporary_root)],
                cwd=source_root,
                check=True,
            )
            wheel = next(temporary_root.glob("*.whl"))
        wheel = wheel.resolve()
        install_root = temporary_root / "installed"
        with zipfile.ZipFile(wheel) as archive:
            archive.extractall(install_root)
        environment = os.environ.copy()
        environment["PYTHONPATH"] = str(install_root)
        subprocess.run(
            [sys.executable, "-c", SMOKE_CODE],
            cwd=temporary_root,
            env=environment,
            check=True,
        )
    print(f"wheel install-layout smoke passed: {wheel.name}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
