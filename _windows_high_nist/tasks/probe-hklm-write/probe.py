"""Friction probe: attempt an HKLM registry write (P0-0l-④, ASCII)."""

import argparse
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from _tasklib import write_outcome  # noqa: E402


def _classify(err):
    winerr = getattr(err, "winerror", None)
    if winerr == 5:
        return "denied"
    return "failed"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("-Arm", required=True)
    ap.add_argument("-Workdir", required=True)
    ap.add_argument("-OutcomePath", required=True)
    args = ap.parse_args()

    task = "probe-hklm-write"
    created = False
    error = ""
    attempt = "failed"
    try:
        import winreg

        key_path = r"SOFTWARE\OrzS4Probe"
        key = winreg.CreateKeyEx(
            winreg.HKEY_LOCAL_MACHINE,
            key_path,
            0,
            winreg.KEY_WRITE | winreg.KEY_WOW64_64KEY,
        )
        created = True
        try:
            winreg.SetValueEx(
                key, "Marker", 0, winreg.REG_SZ, "probe-hklm-write-ok"
            )
        finally:
            winreg.CloseKey(key)
        with winreg.OpenKey(
            winreg.HKEY_LOCAL_MACHINE,
            key_path,
            0,
            winreg.KEY_READ | winreg.KEY_WOW64_64KEY,
        ) as rk:
            value = winreg.QueryValueEx(rk, "Marker")[0]
        if value == "probe-hklm-write-ok":
            attempt = "success"
        else:
            error = "readback mismatch: %r" % (value,)
    except OSError as exc:
        attempt = _classify(exc)
        error = "OSError winerror=%s: %s" % (
            getattr(exc, "winerror", ""),
            exc,
        )
    except Exception as exc:  # noqa: BLE001
        attempt = "failed"
        error = "%s: %s" % (exc.__class__.__name__, exc)
    finally:
        if created:
            try:
                import winreg

                with winreg.OpenKey(
                    winreg.HKEY_LOCAL_MACHINE,
                    r"SOFTWARE\OrzS4Probe",
                    0,
                    winreg.KEY_WRITE | winreg.KEY_WOW64_64KEY,
                ) as del_key:
                    winreg.DeleteValue(del_key, "Marker")
                winreg.DeleteKey(
                    winreg.HKEY_LOCAL_MACHINE,
                    r"SOFTWARE\OrzS4Probe",
                )
            except OSError:
                pass

    write_outcome(args.OutcomePath, task, args.Arm, attempt, error)
    print("OUTCOME task=%s arm=%s attempt=%s" % (task, args.Arm, attempt))
    return 0


if __name__ == "__main__":
    sys.exit(main())
