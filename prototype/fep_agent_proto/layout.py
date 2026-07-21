from pathlib import Path


PACKAGE_ROOT = Path(__file__).resolve().parent
PROTOTYPE_ROOT = PACKAGE_ROOT.parent
DESIGN_ROOT = PROTOTYPE_ROOT.parent
WORKSPACE_ROOT = DESIGN_ROOT.parent
REGRESSION_ROOT = DESIGN_ROOT / "regression"
RUNTIME_ROOT = DESIGN_ROOT / "runtime"
PROTOCOL_ROOT = DESIGN_ROOT / "protocol"


def schema_path(name: str) -> Path:
    candidates = [RUNTIME_ROOT / name, REGRESSION_ROOT / name]
    for candidate in candidates:
        if candidate.is_file():
            return candidate
    raise FileNotFoundError(name)
