from pathlib import Path


PACKAGE_ROOT = Path(__file__).resolve().parent
DESIGN_ROOT = PACKAGE_ROOT.parent
WORKSPACE_ROOT = DESIGN_ROOT


def bundled_or_source_directory(name: str) -> Path:
    """Resolve wheel-bundled resources before the repository source layout."""

    bundled = PACKAGE_ROOT / name
    if bundled.is_dir():
        return bundled
    return DESIGN_ROOT / name


RUNTIME_ROOT = bundled_or_source_directory("runtime")
PROTOCOL_ROOT = bundled_or_source_directory("protocol")
EVALUATION_ROOT = bundled_or_source_directory("evaluation")
REGRESSION_ROOT = bundled_or_source_directory("regression")


def resource_path(relative: str | Path) -> Path:
    """Resolve a repository resource in either wheel or source layout."""

    relative_path = Path(relative)
    if relative_path.is_absolute():
        return relative_path
    bundled = PACKAGE_ROOT / relative_path
    if bundled.exists():
        return bundled
    return DESIGN_ROOT / relative_path


def schema_path(name: str) -> Path:
    candidates = [RUNTIME_ROOT / name, REGRESSION_ROOT / name, PACKAGE_ROOT / name]
    for candidate in candidates:
        if candidate.is_file():
            return candidate
    raise FileNotFoundError(name)
