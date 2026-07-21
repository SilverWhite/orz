from __future__ import annotations

import ctypes
from ctypes import wintypes
import json
import os
from pathlib import Path
from typing import Any

from .errors import PrototypeError
from .io_utils import (
    atomic_write_bytes,
    canonical_bytes,
    sha256_bytes,
    sha256_file,
    utc_now,
)


MAGIC = b"FEP-DPAPI-TRANSCRIPT\x00\x01"
ENTROPY = b"fep-agent-private-transcript-v0.1"
CRYPTPROTECT_UI_FORBIDDEN = 0x00000001


class _DataBlob(ctypes.Structure):
    _fields_ = [
        ("cbData", wintypes.DWORD),
        ("pbData", ctypes.POINTER(ctypes.c_ubyte)),
    ]


def _require_windows() -> None:
    if os.name != "nt":
        raise PrototypeError("DPAPI private transcript storage is available only on Windows")


def _input_blob(data: bytes) -> tuple[_DataBlob, ctypes.Array[ctypes.c_char]]:
    buffer = ctypes.create_string_buffer(data, len(data))
    blob = _DataBlob(
        len(data), ctypes.cast(buffer, ctypes.POINTER(ctypes.c_ubyte))
    )
    return blob, buffer


def _protect(plaintext: bytes) -> bytes:
    _require_windows()
    crypt32 = ctypes.WinDLL("crypt32", use_last_error=True)
    kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
    crypt32.CryptProtectData.argtypes = [
        ctypes.POINTER(_DataBlob),
        wintypes.LPCWSTR,
        ctypes.POINTER(_DataBlob),
        wintypes.LPVOID,
        wintypes.LPVOID,
        wintypes.DWORD,
        ctypes.POINTER(_DataBlob),
    ]
    crypt32.CryptProtectData.restype = wintypes.BOOL
    kernel32.LocalFree.argtypes = [wintypes.HLOCAL]
    kernel32.LocalFree.restype = wintypes.HLOCAL
    input_blob, input_buffer = _input_blob(plaintext)
    entropy_blob, entropy_buffer = _input_blob(ENTROPY)
    output_blob = _DataBlob()
    succeeded = crypt32.CryptProtectData(
        ctypes.byref(input_blob),
        "FEP Agent provider-private transcript v0.1",
        ctypes.byref(entropy_blob),
        None,
        None,
        CRYPTPROTECT_UI_FORBIDDEN,
        ctypes.byref(output_blob),
    )
    _ = input_buffer, entropy_buffer
    if not succeeded:
        raise ctypes.WinError(ctypes.get_last_error())
    try:
        return ctypes.string_at(output_blob.pbData, output_blob.cbData)
    finally:
        kernel32.LocalFree(ctypes.cast(output_blob.pbData, wintypes.HLOCAL))


def _unprotect(ciphertext: bytes) -> bytes:
    _require_windows()
    crypt32 = ctypes.WinDLL("crypt32", use_last_error=True)
    kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
    crypt32.CryptUnprotectData.argtypes = [
        ctypes.POINTER(_DataBlob),
        ctypes.POINTER(wintypes.LPWSTR),
        ctypes.POINTER(_DataBlob),
        wintypes.LPVOID,
        wintypes.LPVOID,
        wintypes.DWORD,
        ctypes.POINTER(_DataBlob),
    ]
    crypt32.CryptUnprotectData.restype = wintypes.BOOL
    kernel32.LocalFree.argtypes = [wintypes.HLOCAL]
    kernel32.LocalFree.restype = wintypes.HLOCAL
    input_blob, input_buffer = _input_blob(ciphertext)
    entropy_blob, entropy_buffer = _input_blob(ENTROPY)
    output_blob = _DataBlob()
    description = wintypes.LPWSTR()
    succeeded = crypt32.CryptUnprotectData(
        ctypes.byref(input_blob),
        ctypes.byref(description),
        ctypes.byref(entropy_blob),
        None,
        None,
        CRYPTPROTECT_UI_FORBIDDEN,
        ctypes.byref(output_blob),
    )
    _ = input_buffer, entropy_buffer
    if not succeeded:
        raise ctypes.WinError(ctypes.get_last_error())
    try:
        return ctypes.string_at(output_blob.pbData, output_blob.cbData)
    finally:
        if description:
            kernel32.LocalFree(ctypes.cast(description, wintypes.HLOCAL))
        kernel32.LocalFree(ctypes.cast(output_blob.pbData, wintypes.HLOCAL))


def save_private_transcript(
    path: Path,
    *,
    run_id: str,
    messages: list[dict[str, Any]],
    overwrite: bool = False,
) -> dict[str, Any]:
    if not run_id:
        raise PrototypeError("private transcript run_id cannot be empty")
    if not isinstance(messages, list):
        raise PrototypeError("private transcript messages must be a list")
    payload = {"messages": messages}
    envelope = {
        "schema_version": "0.1.0-prototype",
        "run_id": run_id,
        "updated_at": utc_now(),
        "payload": payload,
        "payload_sha256": sha256_bytes(canonical_bytes(payload)),
    }
    plaintext = canonical_bytes(envelope)
    ciphertext = _protect(plaintext)
    atomic_write_bytes(path, MAGIC + ciphertext, overwrite=overwrite)
    return {
        "schema_version": "0.1.0-prototype",
        "storage": "windows-dpapi-current-user",
        "path": str(path),
        "run_id": run_id,
        "message_count": len(messages),
        "ciphertext_sha256": sha256_file(path),
        "plaintext_sha256": sha256_bytes(plaintext),
        "plaintext_recorded_outside_dpapi": False,
        "updated_at": envelope["updated_at"],
        "limitations": [
            "DPAPI ciphertext is normally recoverable only by the same Windows user on the same machine.",
            "Python cannot guarantee immediate zeroization of all plaintext object copies in process memory.",
        ],
    }


def load_private_transcript(
    path: Path, *, expected_run_id: str | None = None
) -> dict[str, Any]:
    try:
        stored = path.read_bytes()
    except OSError as exc:
        raise PrototypeError(f"cannot read private transcript: {exc}") from exc
    if not stored.startswith(MAGIC) or len(stored) == len(MAGIC):
        raise PrototypeError("private transcript header is invalid")
    try:
        plaintext = _unprotect(stored[len(MAGIC) :])
    except OSError as exc:
        raise PrototypeError(f"DPAPI could not decrypt private transcript: {exc}") from exc
    try:
        envelope = json.loads(
            plaintext,
            parse_constant=lambda value: (_ for _ in ()).throw(
                ValueError(f"non-standard JSON constant {value}")
            ),
        )
    except (UnicodeDecodeError, json.JSONDecodeError, ValueError) as exc:
        raise PrototypeError(f"decrypted private transcript is invalid JSON: {exc}") from exc
    if not isinstance(envelope, dict):
        raise PrototypeError("decrypted private transcript envelope must be an object")
    required = {"schema_version", "run_id", "updated_at", "payload", "payload_sha256"}
    if set(envelope) != required or envelope["schema_version"] != "0.1.0-prototype":
        raise PrototypeError("decrypted private transcript envelope shape is invalid")
    if expected_run_id is not None and envelope["run_id"] != expected_run_id:
        raise PrototypeError("private transcript run_id does not match expected run")
    payload = envelope["payload"]
    if not isinstance(payload, dict) or set(payload) != {"messages"}:
        raise PrototypeError("private transcript payload shape is invalid")
    if not isinstance(payload["messages"], list):
        raise PrototypeError("private transcript messages must be an array")
    if envelope["payload_sha256"] != sha256_bytes(canonical_bytes(payload)):
        raise PrototypeError("private transcript application digest mismatch")
    return {
        "schema_version": envelope["schema_version"],
        "run_id": envelope["run_id"],
        "updated_at": envelope["updated_at"],
        "messages": payload["messages"],
        "message_count": len(payload["messages"]),
        "payload_sha256": envelope["payload_sha256"],
    }
