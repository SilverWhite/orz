"""Nails for the gate's own-tree ROOT anchor (0al).

`check_repository.py` cross-checks the repository it lives in.  Because a bare
`python scripts/check_repository.py` puts `scripts/` (not the repo root) on
`sys.path`, its `assurance` imports used to resolve through whatever
*editable install* the environment carried — in a frozen clone that is the
original tree, so the gate either crashed on `relative_to` or (same-shaped
paths) silently validated the wrong tree (dogfood run RUN-CLI-6aa999d6,
agent report F2).

These nails pin the fix: the gate anchors its own tree, the reference module
comes from that tree, and a foreign reference module fails closed.
"""

from __future__ import annotations

import importlib.util
from pathlib import Path
import sys
import tempfile
import unittest


_ROOT = Path(__file__).resolve().parents[2]
_SCRIPT = _ROOT / "scripts" / "check_repository.py"
_REFERENCE_PACKAGE = "assurance"
_REFERENCE_MODULE = "assurance.run_event_journal_validation"


def _load_gate(script: Path, name: str):
    spec = importlib.util.spec_from_file_location(name, script)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def _poison_sys_path() -> None:
    """Front-load a directory that is not the tree under test — the shape a
    stale editable install takes when the gate runs inside a clone."""
    sys.path.insert(0, str(Path(tempfile.gettempdir()) / "not-the-tree"))


class GateRootAnchorNails(unittest.TestCase):
    def test_gate_anchors_its_own_tree_first_on_sys_path(self) -> None:
        """Resident nail: after loading, this tree is sys.path[0]."""
        original_path = list(sys.path)
        try:
            sys.path.insert(0, str(Path(tempfile.gettempdir()) / "not-the-tree"))
            gate = _load_gate(_SCRIPT, "check_repository_anchor_resident")
            self.assertEqual(gate.ROOT.resolve(), _ROOT.resolve())
            self.assertEqual(sys.path[0], str(_ROOT))
        finally:
            sys.path[:] = original_path

    def test_reference_module_resolves_to_the_scripts_own_tree(self) -> None:
        """Resident nail: the imported reference module belongs to this tree."""
        original_path = list(sys.path)
        original_package = sys.modules.pop(_REFERENCE_PACKAGE, None)
        original_module = sys.modules.pop(_REFERENCE_MODULE, None)
        try:
            gate = _load_gate(_SCRIPT, "check_repository_anchor_reference")
            errors: list[str] = []
            gate._check_reference_root_anchor(errors)
            self.assertEqual(errors, [], f"resident tree must self-validate: {errors}")
        finally:
            sys.path[:] = original_path
            sys.modules.pop(_REFERENCE_MODULE, None)
            if original_module is not None:
                sys.modules[_REFERENCE_MODULE] = original_module
            if original_package is not None:
                sys.modules[_REFERENCE_PACKAGE] = original_package

    def test_foreign_reference_module_fails_closed(self) -> None:
        """Negative nail: a reference module from another tree is an error."""
        original_path = list(sys.path)
        import assurance

        original_module = sys.modules.get(_REFERENCE_MODULE)
        try:
            gate = _load_gate(_SCRIPT, "check_repository_anchor_negative")
            with tempfile.TemporaryDirectory() as tmp:
                foreign_root = Path(tmp) / "other-tree"
                package = foreign_root / "assurance"
                package.mkdir(parents=True)
                reference = package / "run_event_journal_validation.py"
                reference.write_text(
                    "from pathlib import Path\n"
                    "ROOT = Path(__file__).resolve().parents[1]\n",
                    encoding="utf-8",
                )
                # Simulate the editable-install shadow: the imported module
                # object reports a different tree than the gate's own ROOT.
                class _ForeignModule:
                    __file__ = str(reference)

                # `from assurance import …` prefers the package attribute, so
                # the shadow must be installed there (a plain sys.modules
                # entry would be ignored once the package was imported).
                setattr(assurance, "run_event_journal_validation", _ForeignModule())
                errors: list[str] = []
                gate._check_reference_root_anchor(errors)
                self.assertEqual(len(errors), 1, errors)
                self.assertIn("wrong tree", errors[0])
        finally:
            sys.path[:] = original_path
            if original_module is not None:
                setattr(assurance, "run_event_journal_validation", original_module)
            elif hasattr(assurance, "run_event_journal_validation"):
                delattr(assurance, "run_event_journal_validation")

    def test_clone_shape_resolves_to_the_clone(self) -> None:
        """The friction's real shape: a clone with its own assurance package.

        `python scripts/check_repository.py` inside that clone must import the
        clone's reference module (a shadowing editable install must lose), so
        the gate validates the clone's tree rather than the original one.
        """
        original_path = list(sys.path)
        original_package = sys.modules.pop(_REFERENCE_PACKAGE, None)
        original_module = sys.modules.pop(_REFERENCE_MODULE, None)
        try:
            with tempfile.TemporaryDirectory() as tmp:
                clone = Path(tmp) / "clone"
                (clone / "scripts").mkdir(parents=True)
                package = clone / "assurance"
                package.mkdir()
                (package / "__init__.py").write_text("", encoding="utf-8")
                (package / "run_event_journal_validation.py").write_text(
                    "from pathlib import Path\n"
                    "ROOT = Path(__file__).resolve().parents[1]\n",
                    encoding="utf-8",
                )
                gate_script = clone / "scripts" / "check_repository.py"
                gate_script.write_text(
                    _SCRIPT.read_text(encoding="utf-8"), encoding="utf-8"
                )
                # Poison sys.path the way the environment does: an editable
                # install of the ORIGINAL tree sits ahead of everything.
                _poison_sys_path()
                gate = _load_gate(gate_script, "check_repository_anchor_clone")
                self.assertEqual(gate.ROOT.resolve(), clone.resolve())
                import importlib

                reference = importlib.import_module(_REFERENCE_MODULE)
                self.assertEqual(
                    Path(reference.__file__).resolve().parents[1],
                    clone.resolve(),
                    "the clone's own reference module must win over the install",
                )
                errors: list[str] = []
                gate._check_reference_root_anchor(errors)
                self.assertEqual(errors, [], errors)
        finally:
            sys.path[:] = original_path
            sys.modules.pop(_REFERENCE_MODULE, None)
            if original_module is not None:
                sys.modules[_REFERENCE_MODULE] = original_module
            if original_package is not None:
                sys.modules[_REFERENCE_PACKAGE] = original_package


if __name__ == "__main__":
    unittest.main()
