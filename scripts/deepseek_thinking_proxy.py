"""DeepSeek Thinking-Disabled Passthrough Proxy.

Intercepts Grok → DeepSeek Chat Completions requests and injects
``"thinking": {"type": "disabled"}`` into the request body before
forwarding to ``api.deepseek.com``.  Streaming SSE responses are
relayed back verbatim.

This proxy exists because Grok's ``extra_body`` TOML mechanism
does not reliably add fields to the JSON body for the
``chat_completions`` backend (undocumented / unverified behaviour),
and DeepSeek v4 Pro rejects ``tool_choice`` when thinking mode is
active (even when it was configured as disabled).

Usage::

    python scripts/deepseek_thinking_proxy.py --port 19800

Then point Grok's ``base_url`` at ``http://127.0.0.1:19800``.
Credentials (``LIF_DEEPSEEK_API_KEY``) are read from the environment
and forwarded — they are **never** persisted to disk.
"""

from __future__ import annotations

import argparse
import atexit
import hashlib
from http.server import BaseHTTPRequestHandler, HTTPServer
import json
import os
from pathlib import Path
import re
import signal
import socket
import ssl
import subprocess
import sys
import time
from typing import Any
from urllib.request import Request, urlopen
from urllib.error import HTTPError, URLError


HOST = "127.0.0.1"
DEEPSEEK_URL = "https://api.deepseek.com/chat/completions"
DEFAULT_PORT = 19800
READY_FILE = "ready.json"
MAX_BODY_BYTES = 2 * 1024 * 1024

# ── Daemon paths ───────────────────────────────────────────────────────
DAEMON_HOME = Path(os.environ.get("GSA_HOME", str(Path.home() / ".gsa")))
DAEMON_DIR = DAEMON_HOME / "thinking-proxy"
DAEMON_PID_FILE = DAEMON_DIR / "daemon.pid"
DAEMON_READY_FILE = DAEMON_DIR / "daemon.json"
DAEMON_CAPTURE_FILE = DAEMON_DIR / "capture.json"
DAEMON_STDOUT = DAEMON_DIR / "daemon.stdout.log"
DAEMON_STDERR = DAEMON_DIR / "daemon.stderr.log"


def _sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def _atomic_write_json(path: Path, value: Any) -> None:
    tmp = path.with_name(f".{path.name}.{time.time_ns()}.tmp")
    tmp.write_text(
        json.dumps(value, ensure_ascii=False, indent=2, sort_keys=True, allow_nan=False) + "\n",
        encoding="utf-8",
    )
    tmp.replace(path)


class ThinkingProxyHandler(BaseHTTPRequestHandler):
    """Inject ``thinking: disabled`` and forward to DeepSeek."""

    def do_POST(self) -> None:
        if self.path not in ("/chat/completions", "/v1/chat/completions"):
            self._send_error(404, "Not Found")
            return

        content_length = int(self.headers.get("Content-Length", 0))
        if content_length <= 0 or content_length > MAX_BODY_BYTES:
            self._send_error(400, "Bad Request: invalid Content-Length")
            return

        raw = self.rfile.read(content_length)
        try:
            body = json.loads(raw)
        except json.JSONDecodeError:
            self._send_error(400, "Bad Request: invalid JSON")
            return

        if not isinstance(body, dict):
            self._send_error(400, "Bad Request: body must be a JSON object")
            return

        # ── Inject thinking: disabled ──────────────────────────────────
        body["thinking"] = {"type": "disabled"}
        self.server.capture["requests"].append({
            "timestamp": time.time(),
            "original_tool_choice": (
                body.get("tool_choice") if "tool_choice" in body else None
            ),
            "model": body.get("model", ""),
            "body_sha256": _sha256_bytes(
                json.dumps(body, ensure_ascii=False, sort_keys=True, allow_nan=False).encode("utf-8")
            ),
        })
        # Persist capture after every request so we don't lose it on kill.
        _atomic_write_json(
            self.server.output_dir / "thinking-proxy-capture.json",
            self.server.capture,
        )

        # ── Read authorization from incoming request ───────────────────
        auth_header = self.headers.get("Authorization", "")
        if not auth_header:
            env_key = os.environ.get("LIF_DEEPSEEK_API_KEY", "")
            if env_key:
                auth_header = f"Bearer {env_key}"

        # ── Forward to DeepSeek ────────────────────────────────────────
        try:
            req = Request(
                DEEPSEEK_URL,
                data=json.dumps(body, ensure_ascii=False, allow_nan=False).encode("utf-8"),
                headers={
                    "Content-Type": "application/json",
                    "Authorization": auth_header,
                    "Accept": "text/event-stream, application/json",
                },
                method="POST",
            )
            resp = urlopen(req, timeout=120)
            self.send_response(resp.status)
            content_type = resp.headers.get("Content-Type", "application/json")
            self.send_header("Content-Type", content_type)
            # Forward any SSE-related headers
            for key in ("Cache-Control", "X-Request-Id"):
                val = resp.headers.get(key)
                if val:
                    self.send_header(key, val)
            self.end_headers()

            # Stream the response body
            while True:
                chunk = resp.read(65536)
                if not chunk:
                    break
                self.wfile.write(chunk)
                self.wfile.flush()

        except HTTPError as exc:
            error_body = exc.read().decode("utf-8", errors="replace")
            self.server.capture["errors"].append({
                "timestamp": time.time(),
                "status": exc.code,
                "body": error_body[:2000],
            })
            self.send_response(exc.code)
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            self.wfile.write(error_body.encode("utf-8"))
        except URLError as exc:
            self.server.capture["errors"].append({
                "timestamp": time.time(),
                "reason": str(exc.reason),
            })
            self._send_error(502, f"Bad Gateway: {exc.reason}")
        except Exception as exc:
            self.server.capture["errors"].append({
                "timestamp": time.time(),
                "reason": f"{type(exc).__name__}: {exc}",
            })
            self._send_error(502, f"Bad Gateway: {exc}")

    def _send_error(self, code: int, message: str) -> None:
        self.send_response(code)
        self.send_header("Content-Type", "application/json")
        self.end_headers()
        self.wfile.write(
            json.dumps({"error": message}, ensure_ascii=False).encode("utf-8")
        )

    def log_message(self, format: str, *args: Any) -> None:
        sys.stderr.write(
            f"[proxy] {self.address_string()} — {format % args}\n"
        )


class ThinkingProxyServer(HTTPServer):
    allow_reuse_address = True

    def __init__(self, port: int, output_dir: Path) -> None:
        self.capture: dict[str, list[dict[str, Any]]] = {
            "requests": [],
            "errors": [],
        }
        self.output_dir = output_dir
        self.output_dir.mkdir(parents=True, exist_ok=True)
        super().__init__((HOST, port), ThinkingProxyHandler)


def _daemon_running() -> int | None:
    """Return the daemon PID if it is running, or None."""
    # Primary check: is the port listening?
    if not _port_listening(DEFAULT_PORT):
        return None
    # Secondary: check the ready file
    if not DAEMON_READY_FILE.exists():
        return None
    try:
        ready = json.loads(DAEMON_READY_FILE.read_text())
        if ready.get("ready") and ready.get("host") == HOST:
            return ready.get("pid")
    except (json.JSONDecodeError, OSError):
        pass
    # Fallback: check PID file
    if DAEMON_PID_FILE.exists():
        try:
            return int(DAEMON_PID_FILE.read_text().strip())
        except (ValueError, OSError):
            pass
    return None


def _port_listening(port: int) -> bool:
    """Check if something is listening on *port*."""
    s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    s.settimeout(0.2)
    try:
        s.connect((HOST, port))
        s.close()
        return True
    except (OSError, ConnectionRefusedError):
        return False


def _start_daemon(port: int) -> int:
    """Start the proxy as a background daemon.  Returns the PID."""
    DAEMON_DIR.mkdir(parents=True, exist_ok=True)
    stdout = DAEMON_STDOUT.open("ab")
    stderr = DAEMON_STDERR.open("ab")
    proc = subprocess.Popen(
        [sys.executable, __file__, "--port", str(port), "--output-dir", str(DAEMON_DIR)],
        stdout=stdout,
        stderr=stderr,
        stdin=subprocess.DEVNULL,
        creationflags=subprocess.CREATE_NO_WINDOW if sys.platform == "win32" else 0,
    )
    # Write PID immediately
    DAEMON_PID_FILE.write_text(str(proc.pid))
    # Wait for ready
    deadline = time.monotonic() + 10
    while time.monotonic() < deadline:
        if DAEMON_READY_FILE.exists():
            try:
                ready = json.loads(DAEMON_READY_FILE.read_text())
                if ready.get("ready") and ready.get("port") == port:
                    return proc.pid
            except (json.JSONDecodeError, OSError):
                pass
        if proc.poll() is not None:
            raise RuntimeError(f"daemon exited early (code {proc.returncode})")
        time.sleep(0.1)
    proc.kill()
    raise RuntimeError("daemon did not become ready within 10 s")


def _stop_daemon() -> None:
    """Stop a running daemon."""
    pid = _daemon_running()
    if pid is None:
        print("No daemon running.")
        return
    print(f"Stopping daemon (pid {pid})…")
    try:
        os.kill(pid, signal.SIGTERM)
    except OSError:
        pass
    time.sleep(0.5)
    if _daemon_running():
        try:
            os.kill(pid, signal.SIGKILL)
        except OSError:
            pass
    DAEMON_PID_FILE.unlink(missing_ok=True)
    print("Daemon stopped.")


def _daemon_status() -> dict[str, Any]:
    """Return daemon status as a dict."""
    pid = _daemon_running()
    status = {
        "running": pid is not None,
        "pid": pid,
        "port": DEFAULT_PORT,
        "target": DEEPSEEK_URL,
        "proxy_type": "thinking_disabled_passthrough",
    }
    if DAEMON_READY_FILE.exists():
        try:
            status.update(json.loads(DAEMON_READY_FILE.read_text()))
        except (json.JSONDecodeError, OSError):
            pass
    return status


def main() -> None:
    parser = argparse.ArgumentParser(description="DeepSeek Thinking-Disabled Passthrough Proxy")
    parser.add_argument("--port", type=int, default=DEFAULT_PORT, help=f"Listen port (default: {DEFAULT_PORT})")
    parser.add_argument("--output-dir", type=Path, default=Path.cwd(), help="Output directory for capture artifacts")
    parser.add_argument("--daemon", action="store_true", help="Start as a persistent background daemon")
    parser.add_argument("--stop", action="store_true", help="Stop a running daemon")
    parser.add_argument("--status", action="store_true", help="Print daemon status and exit")
    args = parser.parse_args()

    # ── Daemon control commands ──────────────────────────────────────
    if args.stop:
        _stop_daemon()
        return
    if args.status:
        status = _daemon_status()
        print(json.dumps(status, ensure_ascii=False, indent=2, sort_keys=True))
        return
    if args.daemon:
        existing = _daemon_running()
        if existing is not None:
            print(f"Daemon already running (pid {existing}).")
            return
        try:
            pid = _start_daemon(args.port)
            print(f"Daemon started (pid {pid}) on {HOST}:{args.port}")
        except RuntimeError as exc:
            print(f"Error: {exc}", file=sys.stderr)
            sys.exit(1)
        return

    # ── Foreground mode (used by GrokAcpSession) ─────────────────────
    output_dir = args.output_dir.resolve()
    ready_path = output_dir / READY_FILE

    server = ThinkingProxyServer(args.port, output_dir)

    def _write_capture() -> None:
        capture_path = output_dir / "thinking-proxy-capture.json"
        try:
            _atomic_write_json(capture_path, server.capture)
        except Exception:
            pass

    atexit.register(_write_capture)
    signal.signal(signal.SIGTERM, lambda _signum, _frame: sys.exit(0))

    ready = {
        "ready": True,
        "host": HOST,
        "port": args.port,
        "target": DEEPSEEK_URL,
        "proxy_type": "thinking_disabled_passthrough",
        "pid": os.getpid(),
        "external_bind": False,
    }
    _atomic_write_json(ready_path, ready)
    # Also write daemon.json if we're running in the daemon output dir
    # so that _daemon_running() / _check_thinking_proxy_daemon() can
    # discover us.
    if output_dir == DAEMON_DIR:
        _atomic_write_json(DAEMON_READY_FILE, ready)

    sys.stderr.write(
        f"[proxy] DeepSeek thinking-disabled proxy on {HOST}:{args.port}\n"
        f"[proxy] Forwarding to {DEEPSEEK_URL}\n"
        f"[proxy] Ready file: {ready_path}\n"
        f"[proxy] Injecting 'thinking: disabled' into every request body\n"
    )

    try:
        server.serve_forever()
    except KeyboardInterrupt:
        sys.stderr.write("\n[proxy] Shutting down.\n")
    finally:
        server.server_close()
        capture_path = output_dir / "thinking-proxy-capture.json"
        _atomic_write_json(capture_path, server.capture)


if __name__ == "__main__":
    main()
