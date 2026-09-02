"""Friction probe: real pip --user install under the arm wall (ASCII)."""

import argparse
import glob
import os
import subprocess
import sys
import traceback

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from _tasklib import write_outcome  # noqa: E402


PKG = "orz-s4-probe"
BLOCKED_MARKERS = (
    "user_cache_dir",
    "appdirs",
    "platformdirs",
    "shell folders",
    "currentversion\\explorer",
    "filenotfounderror",
)


def _classify_text(text):
    low = text.lower()
    denied_markers = (
        "access is denied",
        "access denied",
        "permission denied",
        "winerror 5",
        "[errno 13]",
        "permissionerror",
    )
    for marker in denied_markers:
        if marker in low:
            return "denied"
    return ""


def _stdin_file(workdir):
    path = os.path.join(workdir, ".empty-stdin")
    with open(path, "wb"):
        pass
    return path


def _run_piped(args, stdin_path, timeout):
    with open(stdin_path, "rb") as stdin_fh:
        proc = subprocess.run(
            args,
            stdin=stdin_fh,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            timeout=timeout,
            text=True,
            errors="replace",
        )
    return proc


def _user_site(stdin_path):
    proc = _run_piped(
        [sys.executable, "-c", "import site; print(site.USER_SITE)"],
        stdin_path,
        60,
    )
    if proc.returncode != 0:
        return ""
    return (proc.stdout or "").strip()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("-Arm", required=True)
    ap.add_argument("-Workdir", required=True)
    ap.add_argument("-OutcomePath", required=True)
    args = ap.parse_args()

    task = "probe-pip-user-install"
    error = ""
    attempt = "failed"
    extra = {}
    pip_log = os.path.join(args.Workdir, "pip-install.log")
    stdin_path = _stdin_file(args.Workdir)
    try:
        wheels = sorted(glob.glob(os.path.join(args.Workdir, "*.whl")))
        if not wheels:
            raise RuntimeError("wheel seed missing in workdir")
        wheel = wheels[0]
        proc = _run_piped(
            [
                sys.executable,
                "-m",
                "pip",
                "install",
                "--user",
                "--no-index",
                "--no-deps",
                "--no-cache-dir",
                "--disable-pip-version-check",
                "--no-input",
                "--log",
                pip_log,
                wheel,
            ],
            stdin_path,
            240,
        )
        combined = proc.stdout or ""
        if proc.returncode == 0:
            site_dir = _user_site(stdin_path)
            pkg_init = os.path.join(site_dir, "orz_s4_probe", "__init__.py")
            if site_dir and os.path.isfile(pkg_init):
                attempt = "success"
                extra = {"site_file": pkg_init}
            else:
                attempt = "failed"
                error = "pip rc=0 but package file missing (site=%r)" % site_dir
        else:
            low = combined.lower()
            attempt = "blocked" if any(m in low for m in BLOCKED_MARKERS) else (
                _classify_text(combined) or "failed"
            )
            error = "pip rc=%s: %s" % (proc.returncode, combined.strip()[-1200:])
    except OSError as exc:
        attempt = "denied" if getattr(exc, "winerror", None) == 5 else "failed"
        error = "OSError winerror=%s: %s | %s" % (
            getattr(exc, "winerror", ""),
            exc,
            traceback.format_exc(limit=3).replace("\r", " ").replace("\n", " | "),
        )
    except subprocess.TimeoutExpired:
        attempt = "failed"
        error = "pip install timed out"
    except Exception as exc:  # noqa: BLE001
        attempt = "failed"
        error = "%s: %s | %s" % (
            exc.__class__.__name__,
            exc,
            traceback.format_exc(limit=3).replace("\r", " ").replace("\n", " | "),
        )
    finally:
        if attempt == "success":
            try:
                proc = _run_piped(
                    [
                        sys.executable,
                        "-m",
                        "pip",
                        "uninstall",
                        "-y",
                        "--disable-pip-version-check",
                        "--no-input",
                        PKG,
                    ],
                    stdin_path,
                    120,
                )
            except Exception:  # noqa: BLE001
                pass

    write_outcome(args.OutcomePath, task, args.Arm, attempt, error, extra=extra)
    print("OUTCOME task=%s arm=%s attempt=%s" % (task, args.Arm, attempt))
    return 0


if __name__ == "__main__":
    sys.exit(main())
