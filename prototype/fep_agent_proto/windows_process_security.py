from __future__ import annotations

import ctypes
from ctypes import wintypes
import faulthandler
import os
from typing import Any

from .errors import PrototypeError


WER_FAULT_REPORTING_FLAG_NOHEAP = 0x00000001


def configure_secret_process_security() -> dict[str, Any]:
    """Disable ordinary heap-bearing crash reporting before credential access."""

    if os.name != "nt":
        raise PrototypeError("secret process security requires Windows")

    kernel32 = ctypes.WinDLL("Kernel32.dll", use_last_error=True)
    wer_set_flags = kernel32.WerSetFlags
    wer_set_flags.argtypes = [wintypes.DWORD]
    wer_set_flags.restype = ctypes.c_long
    wer_get_flags = kernel32.WerGetFlags
    wer_get_flags.argtypes = [wintypes.HANDLE, ctypes.POINTER(wintypes.DWORD)]
    wer_get_flags.restype = ctypes.c_long
    get_current_process = kernel32.GetCurrentProcess
    get_current_process.argtypes = []
    get_current_process.restype = wintypes.HANDLE

    set_status = int(wer_set_flags(WER_FAULT_REPORTING_FLAG_NOHEAP))
    if set_status != 0:
        raise PrototypeError(
            f"cannot disable WER heap collection (HRESULT 0x{set_status & 0xFFFFFFFF:08x})"
        )
    verified_flags = wintypes.DWORD(0)
    get_status = int(
        wer_get_flags(get_current_process(), ctypes.byref(verified_flags))
    )
    if (
        get_status != 0
        or not verified_flags.value & WER_FAULT_REPORTING_FLAG_NOHEAP
    ):
        raise PrototypeError("WER heap-collection suppression could not be verified")

    faulthandler.disable()
    if faulthandler.is_enabled():
        raise PrototypeError("Python faulthandler could not be disabled")

    return {
        "windows_wer_noheap_verified": True,
        "python_faulthandler_disabled": True,
        "scope": "current-short-lived-cli-process",
        "limitations": [
            "This does not prevent an administrator, debugger, malware, pagefile, hibernation, or external dump tool from reading process memory."
        ],
    }
