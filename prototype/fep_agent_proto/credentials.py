from __future__ import annotations

import ctypes
from ctypes import wintypes
import os
from typing import Protocol

from .errors import PrototypeError


WINDOWS_DEEPSEEK_CREDENTIAL_TARGET = "FEP-Agent/DeepSeek"


class CredentialLease:
    """Best-effort, short-lived owner for an ASCII API key."""

    def __init__(self, secret: bytes, *, source_id: str) -> None:
        _validate_api_key(secret)
        if not source_id or any(character.isspace() for character in source_id):
            raise PrototypeError("credential source ID must be a non-empty opaque token")
        self._secret = bytearray(secret)
        self.source_id = source_id
        self._closed = False

    def authorization_value(self) -> str:
        if self._closed:
            raise PrototypeError("credential lease is already closed")
        return "Bearer " + bytes(self._secret).decode("ascii")

    def close(self) -> None:
        if not self._closed:
            for index in range(len(self._secret)):
                self._secret[index] = 0
            self._closed = True

    def __enter__(self) -> CredentialLease:
        if self._closed:
            raise PrototypeError("credential lease is already closed")
        return self

    def __exit__(self, exc_type: object, exc: object, traceback: object) -> None:
        self.close()

    def __repr__(self) -> str:
        return f"CredentialLease(source_id={self.source_id!r}, secret=<redacted>)"


class CredentialProvider(Protocol):
    source_id: str

    def acquire(self) -> CredentialLease:
        ...


class UnavailableCredentialProvider:
    """Fail-closed default used when no credential authority was configured."""

    source_id = "unavailable"

    def acquire(self) -> CredentialLease:
        raise PrototypeError("DeepSeek credential provider is not configured")


class InMemoryCredentialProvider:
    """Test-only provider; production CLI code must not accept raw API keys."""

    source_id = "test-memory"

    def __init__(self, secret: bytes) -> None:
        _validate_api_key(secret)
        self._secret = bytes(secret)

    def acquire(self) -> CredentialLease:
        return CredentialLease(self._secret, source_id=self.source_id)

    def __repr__(self) -> str:
        return "InMemoryCredentialProvider(secret=<redacted>)"


class WindowsCredentialManagerProvider:
    """Read one fixed generic credential from the current Windows user vault."""

    source_id = "windows-credential-manager-current-user"
    target_name = WINDOWS_DEEPSEEK_CREDENTIAL_TARGET

    def acquire(self) -> CredentialLease:
        if os.name != "nt":
            raise PrototypeError("Windows Credential Manager is unavailable on this platform")

        class CREDENTIALW(ctypes.Structure):
            _fields_ = [
                ("Flags", wintypes.DWORD),
                ("Type", wintypes.DWORD),
                ("TargetName", wintypes.LPWSTR),
                ("Comment", wintypes.LPWSTR),
                ("LastWritten", wintypes.FILETIME),
                ("CredentialBlobSize", wintypes.DWORD),
                ("CredentialBlob", ctypes.POINTER(ctypes.c_ubyte)),
                ("Persist", wintypes.DWORD),
                ("AttributeCount", wintypes.DWORD),
                ("Attributes", wintypes.LPVOID),
                ("TargetAlias", wintypes.LPWSTR),
                ("UserName", wintypes.LPWSTR),
            ]

        credential_pointer = ctypes.POINTER(CREDENTIALW)()
        advapi32 = ctypes.WinDLL("Advapi32.dll", use_last_error=True)
        cred_read = advapi32.CredReadW
        cred_read.argtypes = [
            wintypes.LPCWSTR,
            wintypes.DWORD,
            wintypes.DWORD,
            ctypes.POINTER(ctypes.POINTER(CREDENTIALW)),
        ]
        cred_read.restype = wintypes.BOOL
        cred_free = advapi32.CredFree
        cred_free.argtypes = [wintypes.LPVOID]
        cred_free.restype = None

        if not cred_read(self.target_name, 1, 0, ctypes.byref(credential_pointer)):
            error = ctypes.get_last_error()
            raise PrototypeError(
                f"cannot read the pinned Windows DeepSeek credential (WinError {error})"
            )
        try:
            credential = credential_pointer.contents
            blob = ctypes.string_at(
                credential.CredentialBlob,
                credential.CredentialBlobSize,
            )
            try:
                text = blob.decode("utf-16-le").rstrip("\x00")
                secret = text.encode("ascii")
            except (UnicodeDecodeError, UnicodeEncodeError) as exc:
                raise PrototypeError(
                    "pinned Windows DeepSeek credential must be an ASCII key stored as UTF-16LE"
                ) from exc
            return CredentialLease(secret, source_id=self.source_id)
        finally:
            cred_free(credential_pointer)

    def __repr__(self) -> str:
        return (
            "WindowsCredentialManagerProvider(target='FEP-Agent/DeepSeek', "
            "scope='current-user')"
        )


def _validate_api_key(secret: bytes) -> None:
    if not isinstance(secret, bytes):
        raise PrototypeError("DeepSeek API key must be supplied as bytes")
    if not 8 <= len(secret) <= 512:
        raise PrototypeError("DeepSeek API key length is outside the accepted local bound")
    if any(byte < 0x21 or byte > 0x7E for byte in secret):
        raise PrototypeError("DeepSeek API key must contain printable ASCII without whitespace")
