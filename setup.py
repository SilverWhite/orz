"""Setuptools build customizations for bundled assurance resources."""

from __future__ import annotations

from pathlib import Path
from shutil import copy2, rmtree

from setuptools import setup
from setuptools.command.build_py import build_py as _build_py


class build_py(_build_py):
    """Copy repository-owned contract resources into the installed package."""

    def run(self) -> None:
        super().run()
        repository_root = Path(__file__).resolve().parent
        package_root = Path(self.build_lib) / "assurance"
        # An incremental build must not retain packages/resources removed by
        # the current configuration.
        rmtree(package_root / "tests", ignore_errors=True)
        resource_suffixes = {".json", ".jsonl", ".md", ".txt", ".yaml"}
        for directory in ("runtime", "protocol", "evaluation", "regression"):
            source_root = repository_root / directory
            target_root = package_root / directory
            rmtree(target_root, ignore_errors=True)
            for source in source_root.rglob("*"):
                if not source.is_file() or source.suffix not in resource_suffixes:
                    continue
                relative = source.relative_to(source_root)
                if relative.parts and relative.parts[0] in {"tests", "exports"}:
                    continue
                target = target_root / relative
                target.parent.mkdir(parents=True, exist_ok=True)
                copy2(source, target)


setup(cmdclass={"build_py": build_py})
