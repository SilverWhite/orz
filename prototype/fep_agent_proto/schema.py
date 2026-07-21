from __future__ import annotations

from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator, FormatChecker

from .errors import PrototypeError
from .io_utils import load_json


def validate_instance(instance: Any, schema_path: Path, *, label: str) -> None:
    schema = load_json(schema_path)
    try:
        Draft202012Validator.check_schema(schema)
    except Exception as exc:
        raise PrototypeError(f"invalid schema {schema_path}: {exc}") from exc
    validator = Draft202012Validator(schema, format_checker=FormatChecker())
    errors = sorted(validator.iter_errors(instance), key=lambda error: list(error.absolute_path))
    if errors:
        details = "; ".join(
            f"/{'/'.join(map(str, error.absolute_path))}: {error.message}" for error in errors[:8]
        )
        raise PrototypeError(f"{label} does not satisfy {schema_path.name}: {details}")
