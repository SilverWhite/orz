from __future__ import annotations

import base64
import ctypes
from ctypes import wintypes
import hmac
import os
from pathlib import Path
import secrets
from typing import Protocol

from .contracts import validate_contract
from .errors import AssuranceError
from .utils import (
    atomic_write_bytes,
    atomic_write_json,
    load_json,
    is_link_or_reparse,
    require_no_linked_ancestors,
    require_within,
    sha256_bytes,
    sha256_file,
    utc_now,
)


KEY_BYTES = 32
MAGIC = b"LIF-ASSURANCE-INSTALLATION-KEY\x00\x01"
ENTROPY = b"lif-assurance-installation-key-v0.1"
CRYPTPROTECT_UI_FORBIDDEN = 0x00000001


class InstallationKeyStore(Protocol):
    key_id: str
    storage_id: str

    def sign(self, payload: bytes) -> str:
        ...

    def verify(self, payload: bytes, signature: str) -> bool:
        ...

    def metadata(self) -> dict[str, object]:
        ...


def _key_id(secret: bytes | bytearray) -> str:
    return f"KEY-{sha256_bytes(bytes(secret))[:20].upper()}"


def _encode_signature(value: bytes) -> str:
    return base64.urlsafe_b64encode(value).rstrip(b"=").decode("ascii")


def _decode_signature(value: str) -> bytes:
    if not isinstance(value, str) or len(value) != 43:
        raise AssuranceError("HMAC signature has invalid encoding")
    try:
        return base64.urlsafe_b64decode(value + "=")
    except Exception as exc:
        raise AssuranceError("HMAC signature has invalid encoding") from exc


class MemoryInstallationKeyStore:
    """Test-only key store; never accepted as an OS keystore."""

    storage_id = "memory-test-only"

    def __init__(self, secret: bytes | None = None) -> None:
        selected = secret if secret is not None else secrets.token_bytes(KEY_BYTES)
        if len(selected) != KEY_BYTES:
            raise AssuranceError("installation key must be exactly 32 bytes")
        self._secret = bytearray(selected)
        self.key_id = _key_id(self._secret)
        self._created_at = utc_now()
        self._closed = False

    def sign(self, payload: bytes) -> str:
        if self._closed:
            raise AssuranceError("memory installation key store is closed")
        digest = hmac.new(bytes(self._secret), payload, "sha256").digest()
        return _encode_signature(digest)

    def verify(self, payload: bytes, signature: str) -> bool:
        if self._closed:
            raise AssuranceError("memory installation key store is closed")
        expected = hmac.new(bytes(self._secret), payload, "sha256").digest()
        try:
            supplied = _decode_signature(signature)
        except AssuranceError:
            return False
        return hmac.compare_digest(expected, supplied)

    def metadata(self) -> dict[str, object]:
        value: dict[str, object] = {
            "schema_version": "0.1.0-draft",
            "metadata_kind": "installation_key_metadata",
            "key_id": self.key_id,
            "created_at": self._created_at,
            "key_algorithm": "hmac-sha256",
            "key_bytes": KEY_BYTES,
            "storage": self.storage_id,
            "protected_blob_sha256": None,
            "secret_material_persisted_in_metadata": False,
            "limitations": [
                "This adapter is test-only and does not persist or protect the key."
            ],
        }
        validate_contract(
            value,
            "installation-key-metadata-v0.1.schema.json",
            label="memory installation key metadata",
        )
        return value

    def close(self) -> None:
        if not self._closed:
            for index in range(len(self._secret)):
                self._secret[index] = 0
            self._closed = True


class _DataBlob(ctypes.Structure):
    _fields_ = [
        ("cbData", wintypes.DWORD),
        ("pbData", ctypes.POINTER(ctypes.c_ubyte)),
    ]


def _input_blob(data: bytes) -> tuple[_DataBlob, ctypes.Array[ctypes.c_char]]:
    buffer = ctypes.create_string_buffer(data, len(data))
    blob = _DataBlob(len(data), ctypes.cast(buffer, ctypes.POINTER(ctypes.c_ubyte)))
    return blob, buffer


def _protect(plaintext: bytes) -> bytes:
    if os.name != "nt":
        raise AssuranceError("Windows DPAPI key storage is unavailable on this platform")
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
        "LIF Assurance installation key v0.1",
        ctypes.byref(entropy_blob),
        None,
        None,
        CRYPTPROTECT_UI_FORBIDDEN,
        ctypes.byref(output_blob),
    )
    _ = input_buffer, entropy_buffer
    if not succeeded:
        raise AssuranceError(
            f"DPAPI could not protect installation key (WinError {ctypes.get_last_error()})"
        )
    try:
        return ctypes.string_at(output_blob.pbData, output_blob.cbData)
    finally:
        kernel32.LocalFree(ctypes.cast(output_blob.pbData, wintypes.HLOCAL))


def _unprotect(ciphertext: bytes) -> bytes:
    if os.name != "nt":
        raise AssuranceError("Windows DPAPI key storage is unavailable on this platform")
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
        raise AssuranceError(
            f"DPAPI could not unprotect installation key (WinError {ctypes.get_last_error()})"
        )
    try:
        return ctypes.string_at(output_blob.pbData, output_blob.cbData)
    finally:
        if description:
            kernel32.LocalFree(ctypes.cast(description, wintypes.HLOCAL))
        kernel32.LocalFree(ctypes.cast(output_blob.pbData, wintypes.HLOCAL))


class WindowsDpapiInstallationKeyStore:
    """Random installation key protected by current-user Windows DPAPI."""

    storage_id = "windows-dpapi-current-user"
    blob_name = "installation-key.dpapi"
    metadata_name = "installation-key.json"

    def __init__(self, root: Path) -> None:
        self.root = root
        self.blob_path = root / self.blob_name
        self.metadata_path = root / self.metadata_name
        if not root.is_dir() or is_link_or_reparse(root):
            raise AssuranceError("installation key root must be a non-linked directory")
        for path in (self.blob_path, self.metadata_path):
            require_within(path, root, must_exist=True)
            require_no_linked_ancestors(path, root)
            if not path.is_file():
                raise AssuranceError("installation key files must be regular files")
        metadata = load_json(self.metadata_path)
        validate_contract(
            metadata,
            "installation-key-metadata-v0.1.schema.json",
            label="installation key metadata",
        )
        if metadata["storage"] != self.storage_id:
            raise AssuranceError("installation key metadata uses the wrong storage adapter")
        self.key_id = metadata["key_id"]

    @classmethod
    def create(cls, root: Path) -> WindowsDpapiInstallationKeyStore:
        if os.name != "nt":
            raise AssuranceError("Windows DPAPI key storage is unavailable on this platform")
        root.mkdir(parents=True, exist_ok=True)
        if is_link_or_reparse(root):
            raise AssuranceError("installation key root cannot be linked or reparse-backed")
        blob_path = root / cls.blob_name
        metadata_path = root / cls.metadata_name
        if blob_path.exists() or metadata_path.exists():
            raise AssuranceError("refusing to overwrite an existing installation key")
        secret = bytearray(secrets.token_bytes(KEY_BYTES))
        try:
            key_id = _key_id(secret)
            protected = MAGIC + _protect(bytes(secret))
            atomic_write_bytes(blob_path, protected)
            metadata: dict[str, object] = {
                "schema_version": "0.1.0-draft",
                "metadata_kind": "installation_key_metadata",
                "key_id": key_id,
                "created_at": utc_now(),
                "key_algorithm": "hmac-sha256",
                "key_bytes": KEY_BYTES,
                "storage": cls.storage_id,
                "protected_blob_sha256": sha256_file(blob_path),
                "secret_material_persisted_in_metadata": False,
                "limitations": [
                    "DPAPI ciphertext is scoped to the current Windows user and machine context.",
                    "Python cannot guarantee zeroization of every temporary key copy in memory."
                ],
            }
            validate_contract(
                metadata,
                "installation-key-metadata-v0.1.schema.json",
                label="new installation key metadata",
            )
            atomic_write_json(metadata_path, metadata)
        except Exception as orig_exc:
            try:
                blob_path.unlink(missing_ok=True)
            except OSError as cleanup_exc:
                orig_exc.add_note(
                    f"blob cleanup also failed during key generation rollback: {cleanup_exc}"
                )
            raise
        finally:
            for index in range(len(secret)):
                secret[index] = 0
        return cls(root)

    def metadata(self) -> dict[str, object]:
        metadata = load_json(self.metadata_path)
        validate_contract(
            metadata,
            "installation-key-metadata-v0.1.schema.json",
            label="installation key metadata",
        )
        return metadata

    def _read_secret(self) -> bytearray:
        for path in (self.blob_path, self.metadata_path):
            require_within(path, self.root, must_exist=True)
            require_no_linked_ancestors(path, self.root)
            if not path.is_file():
                raise AssuranceError("installation key files must be regular files")
        metadata = self.metadata()
        if sha256_file(self.blob_path) != metadata["protected_blob_sha256"]:
            raise AssuranceError("installation key protected-blob digest mismatch")
        stored = self.blob_path.read_bytes()
        if not stored.startswith(MAGIC) or len(stored) == len(MAGIC):
            raise AssuranceError("installation key protected blob has an invalid header")
        secret = bytearray(_unprotect(stored[len(MAGIC) :]))
        if len(secret) != KEY_BYTES or _key_id(secret) != self.key_id:
            for index in range(len(secret)):
                secret[index] = 0
            raise AssuranceError("installation key identity check failed")
        return secret

    def sign(self, payload: bytes) -> str:
        secret = self._read_secret()
        try:
            digest = hmac.new(bytes(secret), payload, "sha256").digest()
            return _encode_signature(digest)
        finally:
            for index in range(len(secret)):
                secret[index] = 0

    def verify(self, payload: bytes, signature: str) -> bool:
        secret = self._read_secret()
        try:
            expected = hmac.new(bytes(secret), payload, "sha256").digest()
            try:
                supplied = _decode_signature(signature)
            except AssuranceError:
                return False
            return hmac.compare_digest(expected, supplied)
        finally:
            for index in range(len(secret)):
                secret[index] = 0
