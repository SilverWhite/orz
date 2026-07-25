from __future__ import annotations

from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator, FormatChecker

from .errors import AssuranceError
from .utils import load_json


ASSURANCE_ROOT = Path(__file__).resolve().parent


def validate_contract(instance: Any, schema_name: str, *, label: str) -> None:
    schema_path = ASSURANCE_ROOT / schema_name
    validator = Draft202012Validator(
        load_json(schema_path), format_checker=FormatChecker()
    )
    errors = sorted(validator.iter_errors(instance), key=lambda item: list(item.path))
    if not errors:
        return
    rendered: list[str] = []
    for error in errors:
        pointer = "/".join(str(part) for part in error.absolute_path)
        rendered.append(f"{label}#/{pointer}: {error.message}")
    raise AssuranceError("; ".join(rendered))
