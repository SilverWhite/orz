from pathlib import Path


PACKAGE_ROOT = Path(__file__).resolve().parent
DESIGN_ROOT = PACKAGE_ROOT.parent
WORKSPACE_ROOT = DESIGN_ROOT
REGRESSION_ROOT = DESIGN_ROOT / "regression"
RUNTIME_ROOT = DESIGN_ROOT / "runtime"
PROTOCOL_ROOT = DESIGN_ROOT / "protocol"


def schema_path(name: str) -> Path:
    candidates = [RUNTIME_ROOT / name, REGRESSION_ROOT / name, PACKAGE_ROOT / name]
    for candidate in candidates:
        if candidate.is_file():
            return candidate
    raise FileNotFoundError(name)
