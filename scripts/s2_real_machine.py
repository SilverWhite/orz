#!/usr/bin/env python3
"""0dn S2 —— 真机十条场景（real Chrome + `orz web --fake-provider` 等价的
`ORZ_FAKE_*` 脚本替身；零 Playwright）。

设计权威：docs/audits/330_SURFACE_CONTRACT_BASELINE_2026-10-21.md §7
（S2＝10 条脚本化场景，仓内 CDP 客户端驱动真 Chrome；每次载体重建后必跑）。

用法：
    python scripts/s2_real_machine.py                 # 全部十条
    python scripts/s2_real_machine.py --only refresh,delivery
    python scripts/s2_real_machine.py --bin D:/tb-eval/orz-windows/orz.exe --out D:/tb-eval/s2-098

实现边界（如实）：
- CDP 客户端复用 `assurance/browser_retrieval.py::BrowserCDPClient`；本脚本
  只补 `Page.bringToFront` / focus emulation（headless 下 rAF 节流会让
  `scheduleRender` 不跑，属无头观察面而非产品缺陷）。
- 权限并发：权限门默认 yolo，真实工具批按顺序执行，第二个 agent 侧权限
  请求不会与第一个同时在途；场景先用真实 `browser_control script` 取得
  真实权限弹窗，再由页面内本尊 `dialogs.showPermission` 送第二个请求，
  以验证「顶掉」代码路径（333 落码面）。审计档明记该形态。
"""

from __future__ import annotations

import argparse
import base64
import hashlib
import json
import os
import re
import shutil
import socket
import subprocess
import sys
import threading
import time
import traceback
import urllib.request
from dataclasses import dataclass, field
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO))
from assurance.browser_retrieval import BrowserCDPClient  # noqa: E402

DEFAULT_BIN = Path(os.environ.get("ORZ_S2_BIN", r"C:\Users\1\AppData\Local\orz\bin\orz.exe"))
DEFAULT_OUT = Path(os.environ.get("ORZ_S2_OUT", r"D:\tb-eval\s2-098"))
ACAF_ROOT = Path(r"D:\tb-eval\orz-windows\acaf")

CHROME_CANDIDATES = [
    Path(r"C:\Program Files\Google\Chrome\Application\chrome.exe"),
    Path(r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe"),
]


def find_chrome() -> Path:
    for p in CHROME_CANDIDATES:
        if p.is_file():
            return p
    raise SystemExit("Chrome not found")


def free_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


def kill_tree(pid: int) -> None:
    if pid <= 0:
        return
    try:
        subprocess.run(
            ["taskkill", "/PID", str(pid), "/T", "/F"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            timeout=15,
        )
    except Exception:
        pass


def wait_http(url: str, timeout: float = 20.0) -> bool:
    deadline = time.time() + timeout
    while time.time() < deadline:
        try:
            with urllib.request.urlopen(url, timeout=1) as r:
                if r.status == 200:
                    return True
        except Exception:
            time.sleep(0.2)
    return False


class CheckFail(RuntimeError):
    pass


@dataclass
class Check:
    name: str
    passed: bool
    evidence: str = ""


@dataclass
class Ctx:
    name: str
    title: str
    out: Path
    binary: Path
    scenario_file: Path | None = None
    keepalive: int = 300
    extra_env: dict = field(default_factory=dict)
    started_at: float = 0.0
    server: "Server | None" = None
    chrome: "Chrome | None" = None
    client: BrowserCDPClient | None = None
    tab: str | None = None
    checks: list[Check] = field(default_factory=list)
    artifacts: list[str] = field(default_factory=list)

    @property
    def ws(self) -> Path:
        return self.out / "ws"

    @property
    def token(self) -> str:
        assert self.server is not None
        return self.server.token

    # ---------- assertions ----------
    def check(self, name: str, ok: bool, evidence: str = "") -> bool:
        self.checks.append(Check(name, bool(ok), evidence))
        return bool(ok)

    def require(self, name: str, ok: bool, evidence: str = "") -> None:
        if not self.check(name, ok, evidence):
            raise CheckFail(f"{name} :: {evidence}")

    def eq(self, name: str, got, want) -> None:
        self.require(name, got == want, f"got={got!r} want={want!r}")

    def contains(self, name: str, hay: str, needle: str) -> None:
        self.require(name, needle in (hay or ""), f"missing={needle!r} hay={ (hay or '')[:400]!r}")

    # ---------- page primitives ----------
    def eval(self, js: str, timeout: float = 30.0) -> object:
        assert self.client is not None and self.tab is not None
        r = self.client._send(
            self.tab,
            "Runtime.evaluate",
            {
                "expression": js,
                "returnByValue": True,
                "awaitPromise": True,
                "userGesture": True,
            },
        )
        if r.get("exceptionDetails"):
            det = r["exceptionDetails"]
            desc = (
                det.get("exception", {}).get("description")
                or det.get("text")
                or json.dumps(det, ensure_ascii=False)
            )
            raise RuntimeError(f"JS exception: {desc}")
        return r.get("result", {}).get("value")

    def wait(self, js: str, timeout: float = 20.0, desc: str = "condition", interval: float = 0.25):
        deadline = time.time() + timeout
        last = None
        while time.time() < deadline:
            try:
                last = self.eval(js)
                if last:
                    return last
            except Exception as e:  # page reload window / transient
                last = f"<err {e}>"
            time.sleep(interval)
        raise CheckFail(f"timeout waiting {desc!r} ({timeout}s); last={last!r}")

    def text(self, sel: str) -> str:
        return self.eval(f"(document.querySelector({json.dumps(sel)})||{{}}).innerText||''") or ""

    def modal_text(self) -> str:
        return self.text("#modalLayer")

    def content_text(self) -> str:
        return self.text("#contentScroll")

    def toolbar_labels(self) -> list[str]:
        return self.eval("[...document.querySelectorAll('#toolbar button')].map(b=>b.textContent)") or []

    def click_toolbar(self, label: str) -> None:
        ok = self.eval(
            "(()=>{const b=[...document.querySelectorAll('#toolbar button')].find(x=>x.textContent===%s);"
            "if(!b||b.disabled)return false;b.click();return true})()" % json.dumps(label)
        )
        self.require(f"toolbar[{label}] clickable", ok is True, f"labels={self.toolbar_labels()}")

    def fill(self, text: str) -> None:
        self.eval(
            "(()=>{const t=document.getElementById('chatInput');t.value=%s;"
            "t.dispatchEvent(new Event('input',{bubbles:true}));return true})()" % json.dumps(text)
        )

    def send(self) -> None:
        self.eval("document.getElementById('sendBtn').click(); true")

    def state(self, expr: str = "m.state") -> object:
        return self.eval(
            f"(async()=>{{const m=await import('/assets/app/state.js');return ({expr});}})()"
        )

    def session_id(self) -> str | None:
        return self.state("m.state.sessionId")

    def run_statuses(self) -> list[str]:
        return self.state("m.state.runs.map(r=>r.run_id+':'+r.status)") or []

    def run_dir_names(self) -> set[str]:
        runs = self.ws / ".gsa" / "runs"
        if not runs.is_dir():
            return set()
        return {p.name for p in runs.iterdir() if p.is_dir() and p.name.startswith("RUN-")}

    def terminal_run_names(self) -> set[str]:
        out = set()
        runs = self.ws / ".gsa" / "runs"
        if not runs.is_dir():
            return out
        for d in runs.iterdir():
            if not d.is_dir() or not d.name.startswith("RUN-"):
                continue
            ev = d / "events.jsonl"
            if not ev.is_file():
                continue
            try:
                text = ev.read_text(encoding="utf-8", errors="replace")
            except Exception:
                continue
            if re.search(r'"event_type":"run_(finished|failed|cancelled|invalidated|terminated)"', text):
                out.add(d.name)
        return out

    def wait_py(self, fn, timeout: float = 30.0, desc: str = "condition", interval: float = 0.25):
        deadline = time.time() + timeout
        last = None
        while time.time() < deadline:
            try:
                last = fn()
                if last:
                    return last
            except Exception as e:
                last = f"<err {e}>"
            time.sleep(interval)
        raise CheckFail(f"timeout waiting {desc!r} ({timeout}s); last={last!r}")

    def prompt(self, text: str, timeout: float = 90.0) -> None:
        before = self.run_dir_names()
        self.fill(text)
        self.send()
        self.wait(
            "(async()=>{const m=await import('/assets/app/state.js');return m.state.running===false})()",
            timeout,
            "run finished",
        )
        # 清单跟随是 1s 轮询，极短回合可能整轮落在两次轮询之间；以磁盘
        # journal 的新 run 目录 + 终态事件为完成事实（与 UI 同源事实面）。
        self.wait_py(
            lambda: bool((self.terminal_run_names() - before)),
            30,
            "new run journal terminal",
        )
        time.sleep(0.4)

    def new_tab(self, url: str | None = None) -> str:
        assert self.client is not None and self.server is not None
        tab = self.client.new_tab("about:blank")
        self.client._send(tab, "Page.enable")
        self.client._send(
            tab,
            "Page.addScriptToEvaluateOnNewDocument",
            {
                "source": "window.__s2errors=[];addEventListener('error',e=>window.__s2errors.push('err:'+e.message+' @'+e.filename+':'+e.lineno));"
                "addEventListener('unhandledrejection',e=>window.__s2errors.push('rej:'+String(e.reason&&e.reason.stack||e.reason)));"
            },
        )
        self.client._send(tab, "Page.navigate", {"url": url or self.server.url})
        self.client._send(tab, "Page.bringToFront")
        try:
            self.client._send(tab, "Emulation.setFocusEmulationEnabled", {"enabled": True})
        except Exception:
            pass
        return tab

    def wait_page(self, tab: str, timeout: float = 25.0) -> None:
        deadline = time.time() + timeout
        while time.time() < deadline:
            try:
                r = self.client._send(
                    tab,
                    "Runtime.evaluate",
                    {
                        "expression": "document.readyState==='complete' && !!document.getElementById('toolbar')",
                        "returnByValue": True,
                    },
                )
                if r.get("result", {}).get("value") is True:
                    return
            except Exception:
                pass
            time.sleep(0.2)
        raise CheckFail("page did not become ready")

    def open(self) -> None:
        assert self.client is not None and self.server is not None
        self.tab = self.new_tab(self.server.url)
        self.wait_page(self.tab)
        self.wait(
            "document.getElementById('connBanner') && document.getElementById('connBanner').innerText.length>0",
            15,
            "boot banner",
        )
        time.sleep(0.4)

    def screenshot(self, tag: str) -> str:
        assert self.client is not None and self.tab is not None
        data = self.client._send(
            self.tab, "Page.captureScreenshot", {"format": "png", "captureBeyondViewport": False}
        )
        path = self.out / f"{tag}.png"
        path.write_bytes(base64.b64decode(data["data"]))
        self.artifacts.append(str(path))
        return str(path)

    def close_modal(self) -> None:
        try:
            self.eval(
                "(()=>{const b=[...document.querySelectorAll('#modalLayer button')].find(x=>x.textContent==='取消'||x.textContent==='关闭');if(b)b.click();return true})()"
            )
        except Exception:
            pass

    # ---------- delete / direct protocol ----------
    def delete_session_via_api(self, s8: str) -> dict:
        js = (
            "fetch('/api/sessions/%s?token=%s',{method:'DELETE'})"
            ".then(async r=>({status:r.status,body:await r.text()}))"
        ) % (s8, self.token)
        raw = self.eval(js)
        try:
            body = json.loads(raw.get("body") or "{}")
        except Exception:
            body = {"raw": raw.get("body")}
        return {"status": raw.get("status"), "body": body}


class Server:
    def __init__(self, binary: Path, cwd: Path, port: int, env: dict, log_path: Path):
        self.binary = binary
        self.cwd = cwd
        self.port = port
        self.env = env
        self.log_path = log_path
        self.proc: subprocess.Popen | None = None
        self.url = ""
        self.token = ""
        self._url_evt = threading.Event()
        self._lines: list[str] = []
        self._reader: threading.Thread | None = None

    def start(self, timeout: float = 30.0) -> None:
        self.log_path.parent.mkdir(parents=True, exist_ok=True)
        self._log = self.log_path.open("w", encoding="utf-8")
        self.proc = subprocess.Popen(
            [str(self.binary), "web", "--addr", f"127.0.0.1:{self.port}", "--cwd", str(self.cwd)],
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            encoding="utf-8",
            errors="replace",
            env=self.env,
            creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0,
        )
        self._reader = threading.Thread(target=self._read, daemon=True)
        self._reader.start()
        if not self._url_evt.wait(timeout):
            raise CheckFail(f"server did not print URL within {timeout}s; log={self.log_path}")
        if not wait_http(f"http://127.0.0.1:{self.port}/", 20):
            raise CheckFail("server URL printed but HTTP not ready")

    def _read(self) -> None:
        assert self.proc is not None and self.proc.stdout is not None
        for line in self.proc.stdout:
            self._lines.append(line)
            try:
                self._log.write(line)
                self._log.flush()
            except Exception:
                pass
            m = re.search(r"(http://\S+?#token=([0-9a-f]+))", line)
            if m and not self.url:
                self.url = m.group(1)
                self.token = m.group(2)
                self._url_evt.set()

    @property
    def text(self) -> str:
        return "".join(self._lines)

    def stop(self) -> None:
        if self.proc and self.proc.poll() is None:
            try:
                self.proc.terminate()
                self.proc.wait(timeout=8)
            except Exception:
                kill_tree(self.proc.pid)
        try:
            self._log.close()
        except Exception:
            pass


class Chrome:
    def __init__(self, port: int, profile: Path):
        self.port = port
        self.profile = profile
        self.proc: subprocess.Popen | None = None

    def start(self) -> None:
        self.profile.mkdir(parents=True, exist_ok=True)
        exe = find_chrome()
        args = [
            str(exe),
            f"--remote-debugging-port={self.port}",
            "--headless=new",
            "--disable-gpu",
            "--no-first-run",
            "--no-default-browser-check",
            "--disable-extensions",
            "--disable-background-networking",
            "--disable-sync",
            "--disable-translate",
            "--disable-features=TranslateUI,CalculateNativeWinOcclusion",
            "--disable-background-timer-throttling",
            "--disable-backgrounding-occluded-windows",
            "--disable-renderer-backgrounding",
            "--metrics-recording-only",
            "--remote-allow-origins=*",
            f"--user-data-dir={self.profile}",
            "about:blank",
        ]
        self.proc = subprocess.Popen(
            args,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            stdin=subprocess.DEVNULL,
            creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0,
        )
        if not wait_http(f"http://127.0.0.1:{self.port}/json/version", 20):
            raise CheckFail("Chrome CDP did not become ready")

    def stop(self) -> None:
        if self.proc and self.proc.poll() is None:
            kill_tree(self.proc.pid)


def base_env(ctx: Ctx) -> dict:
    env = os.environ.copy()
    env.update(
        {
            "ORZ_ACAF_KEYSTORE": str(ACAF_ROOT / "keystore"),
            "ORZ_ACAF_MANIFEST": str(ACAF_ROOT / "signer-manifest.json"),
            "ORZ_ACAF_BINARY": str(Path(r"D:\tb-eval\orz-windows\orz-signer.exe")),
            "ORZ_WEB_KEEPALIVE_SECS": str(ctx.keepalive),
            "RUST_LOG": "info",
            "NO_COLOR": "1",
        }
    )
    if ctx.scenario_file is not None:
        env["ORZ_FAKE_SCENARIO"] = str(ctx.scenario_file)
    env.update({k: str(v) for k, v in ctx.extra_env.items()})
    return env


def setup(ctx: Ctx) -> None:
    ctx.out.mkdir(parents=True, exist_ok=True)
    ctx.ws.mkdir(parents=True, exist_ok=True)
    ctx.server = Server(ctx.binary, ctx.ws, free_port(), base_env(ctx), ctx.out / "server.log")
    ctx.server.start()
    cport = free_port()
    ctx.chrome = Chrome(cport, ctx.out / "chrome-profile")
    ctx.chrome.start()
    ctx.client = BrowserCDPClient(port=cport)
    if not ctx.client.connect():
        raise CheckFail("CDP connect failed")
    # 给页面内 fetch/WS 一个稳定时钟。version-skew 场景必须在打开页面前
    # 替换磁盘二进制，故由其自行调用 open()。
    if ctx.name != "version-skew":
        ctx.open()


def teardown(ctx: Ctx) -> None:
    try:
        if ctx.client is not None:
            ctx.client.disconnect()
    except Exception:
        pass
    try:
        if ctx.chrome is not None:
            ctx.chrome.stop()
    except Exception:
        pass
    try:
        if ctx.server is not None:
            ctx.server.stop()
    except Exception:
        pass


# ───────────────────────── scenarios ─────────────────────────


def s_refresh(ctx: Ctx) -> None:
    ctx.prompt("第一轮：刷新重连")
    sid1 = ctx.session_id()
    ctx.require("session bound", bool(sid1), str(sid1))
    ctx.contains("first response rendered", ctx.content_text(), "已收到请求")
    first_runs = len(ctx.run_statuses())

    assert ctx.client is not None and ctx.tab is not None
    ctx.client._send(ctx.tab, "Page.reload", {"ignoreCache": True})
    time.sleep(0.8)
    ctx.wait_page(ctx.tab)
    ctx.wait(
        "(async()=>{const m=await import('/assets/app/state.js');return m.state.sessionId===%s})()" % json.dumps(sid1),
        25,
        "session rebound after reload",
    )
    ctx.wait("document.getElementById('contentScroll').innerText.includes('已重连到')", 20, "reconnect notice")
    recon = ctx.content_text()
    ctx.contains("reconnect says keepalive child", recon, "子进程保活")
    ctx.contains("history replayed after reload", recon, "已收到请求")
    ctx.eq("same session id", ctx.session_id(), sid1)
    ctx.require(
        "server logged reused attach",
        "reused=true" in (ctx.server.text if ctx.server else ""),
        (ctx.server.text[-800:] if ctx.server else ""),
    )

    ctx.prompt("第二轮：刷新后继续")
    ctx.contains("second response after reload", ctx.content_text(), "已收到请求")
    ctx.require("second run recorded", len(ctx.run_statuses()) >= first_runs + 1, str(ctx.run_statuses()))
    ctx.screenshot("refresh-reconnected")


def s_two_tabs(ctx: Ctx) -> None:
    ctx.prompt("双标签占用测试")
    assert ctx.server is not None and ctx.client is not None

    tab2 = ctx.new_tab(ctx.server.url)
    ctx.wait_page(tab2)
    deadline = time.time() + 12
    banner2 = ""
    while time.time() < deadline:
        banner2 = ctx.client._send(
            tab2,
            "Runtime.evaluate",
            {
                "expression": "document.getElementById('connBanner').innerText",
                "returnByValue": True,
            },
        ).get("result", {}).get("value") or ""
        if "连不上本地桥" in banner2:
            break
        time.sleep(0.3)
    ctx.contains("second tab gets occupied copy", banner2, "连不上本地桥")
    ctx.contains("second tab mentions other window", banner2, "另一个 orz 窗口")
    ctx.screenshot("two-tabs-409")

    # 直接 WS 握手：断言 HTTP 409 + 四要素 body（浏览器 JS 看不到 body）
    status, body = raw_ws_status(ctx.server.port, ctx.server.token)
    ctx.eq("raw WS upgrade status", status, 409)
    ctx.contains("409 body workspace_occupied", body, "workspace_occupied")
    for key in ("code", "class", "hint", "evidence"):
        ctx.contains(f"409 body has {key}", body, f'"{key}"')
    ctx.client.close_tab(tab2)


def raw_ws_status(port: int, token: str) -> tuple[int, str]:
    key = base64.b64encode(os.urandom(16)).decode()
    req = (
        f"GET /ws/acp?token={token} HTTP/1.1\r\n"
        f"Host: 127.0.0.1:{port}\r\n"
        "Upgrade: websocket\r\n"
        "Connection: Upgrade\r\n"
        f"Sec-WebSocket-Key: {key}\r\n"
        "Sec-WebSocket-Version: 13\r\n"
        f"Origin: http://127.0.0.1:{port}\r\n"
        "\r\n"
    ).encode()
    with socket.create_connection(("127.0.0.1", port), timeout=8) as s:
        s.sendall(req)
        data = b""
        deadline = time.time() + 8
        while time.time() < deadline:
            chunk = s.recv(4096)
            if not chunk:
                break
            data += chunk
            if b"\r\n\r\n" in data:
                head, _, rest = data.partition(b"\r\n\r\n")
                if b"Content-Length:" in head:
                    try:
                        n = int(re.search(rb"Content-Length:\s*(\d+)", head).group(1))
                        if len(rest) >= n:
                            break
                    except Exception:
                        pass
                elif b"Transfer-Encoding: chunked" not in head:
                    break
        head, _, body = data.partition(b"\r\n\r\n")
        status = int(head.split(b" ")[1]) if head.startswith(b"HTTP/") else 0
        return status, body.decode("utf-8", "replace")


def s_rewind(ctx: Ctx) -> None:
    ctx.prompt("第一轮：请记住 42")
    ctx.contains("first response", ctx.content_text(), "已收到请求")
    ctx.fill("/rewind")
    ctx.send()
    ctx.wait("document.getElementById('modalLayer').innerText.includes('撤回消息')", 15, "rewind dialog")
    ctx.wait("document.getElementById('modalLayer').innerText.includes('#1')", 15, "rewind ordinal list")
    ctx.screenshot("rewind-dialog")
    ok = ctx.eval(
        "(()=>{const b=[...document.querySelectorAll('#modalLayer button')].find(x=>x.textContent==='撤回');if(!b)return false;b.click();return true})()"
    )
    ctx.require("rewind confirm clickable", ok is True)
    ctx.wait("document.getElementById('modalLayer').innerText===''", 15, "rewind dialog closed")
    inp = ctx.eval("document.getElementById('chatInput').value")
    ctx.contains("original text returned to input", inp, "第一轮")
    ctx.wait("document.getElementById('contentScroll').innerText.includes('已撤回')", 15, "recall system line")
    recall = ctx.eval(
        "(async()=>{const m=await import('/assets/app/state.js');const r=await fetch('/api/recall?session8='+m.state.boundSession8+'&root='+encodeURIComponent(m.state.sessionRoot||m.state.workspaceCwd)+'&token=%s').then(x=>x.json());return r})()"
        % ctx.token
    )
    ctx.require("recall_log recorded", isinstance(recall, dict) and len(recall.get("recall_log") or []) >= 1, json.dumps(recall, ensure_ascii=False)[:500])

    # 原文回输入框后直接重发
    ctx.send()
    ctx.wait(
        "(async()=>{const m=await import('/assets/app/state.js');return m.state.running===true})()",
        20,
        "resend started",
    )
    ctx.wait(
        "(async()=>{const m=await import('/assets/app/state.js');return m.state.running===false})()",
        90,
        "resend finished",
    )
    time.sleep(0.4)
    ctx.contains("resend produced response", ctx.content_text(), "已收到请求")
    sid = ctx.session_id()
    ctx.require("resend kept same session", bool(sid), str(sid))
    ctx.screenshot("rewind-resend")


def s_delete(ctx: Ctx) -> None:
    ctx.prompt("删除后不复活测试")
    sid = ctx.session_id()
    ctx.require("session bound", bool(sid), str(sid))
    s8 = sid[:8]
    # 删除入口只对已关闭/归档会话开放（0dk：lease 持有中先 close）。
    ctx.click_toolbar("关闭会话")
    # 「关闭会话」是直接动作（无确认弹窗）；等绑定解除与系统事实行。
    ctx.wait(
        "(async()=>{const m=await import('/assets/app/state.js');return m.state.sessionId===null})()",
        20,
        "session closed/unbound",
    )
    closed = ctx.ws / ".gsa" / "sessions" / f"{s8}.closed"
    ctx.require("closed marker written", closed.is_file(), str(closed))
    ctx.screenshot("delete-closed")

    del_res = ctx.delete_session_via_api(s8)
    ctx.eq("DELETE /api/sessions status", del_res["status"], 200)
    ctx.require("delete response ok", del_res["body"].get("ok") is True, json.dumps(del_res, ensure_ascii=False)[:500])
    tomb = ctx.ws / ".gsa" / "sessions" / f"{s8}.deleted"
    ctx.require("deleted tombstone written", tomb.is_file(), str(tomb))
    conv = ctx.ws / ".gsa" / "conversations" / f"{s8}.json"
    time.sleep(0.6)
    ctx.require("conversation sidecar removed", not conv.exists(), str(conv))
    ctx.screenshot("delete-tombstone")

    # 旧 session id 直接 prompt：用户面应是四要素「会话不存在或已被删除」。
    err = ctx.eval(
        "(async()=>{const a=await import('/assets/app/acp.js');try{await a.sessionPrompt(%s,'复活？');return 'NO_ERROR'}catch(e){return String(e.message||e)}})()"
        % json.dumps(sid)
    )
    ctx.require("old session prompt rejected", "NO_ERROR" not in err, str(err))
    ctx.contains("old session rejection names deletion", err, "会话不存在或已被删除")
    ctx.contains("old session rejection has evidence", err, "证据：session=")
    time.sleep(0.8)
    ctx.require("sidecar not resurrected", not conv.exists(), str(conv))

    # UI 下一条输入落新会话（非旧会话）
    ctx.prompt("删除后的新会话")
    sid2 = ctx.session_id()
    ctx.require("new session id differs", bool(sid2) and sid2 != sid, f"old={sid} new={sid2}")
    ctx.screenshot("delete-new-session")


def s_keepalive(ctx: Ctx) -> None:
    ctx.prompt("保活窗口测试")
    sid = ctx.session_id()
    ctx.require("session bound", bool(sid), str(sid))
    s8 = sid[:8]
    assert ctx.client is not None and ctx.tab is not None
    ctx.client.close_tab(ctx.tab)
    ctx.tab = None
    # REAPER_TICK=15s；grace=2s。等待 reaper 记录 expired。
    time.sleep(19)
    log = ctx.server.text if ctx.server else ""
    ctx.contains("reaper logged grace reclaim", log, "ACP agent child reclaimed (grace/dead)")
    ctx.contains("expired record in log", log, "reason=\"grace\"")

    ctx.tab = ctx.new_tab(ctx.server.url)
    ctx.wait_page(ctx.tab)
    ctx.wait("document.getElementById('contentScroll').innerText.includes('保活窗口已过期')", 20, "expired notice")
    txt = ctx.content_text()
    ctx.contains("expired names old session", txt, s8)
    ctx.contains("expired says new session next", txt, "下一条输入将新建会话")
    ctx.screenshot("keepalive-expired")

    ctx.prompt("过期后的新会话")
    sid2 = ctx.session_id()
    ctx.require("new session after expiry", bool(sid2) and sid2 != sid, f"old={sid} new={sid2}")


def s_delivery(ctx: Ctx) -> None:
    ctx.prompt("交付面板测试")
    s8 = (ctx.session_id() or "")[:8]
    ctx.require("session bound", bool(s8), s8)
    ctx.click_toolbar("交付状态")
    ctx.wait("document.getElementById('modalLayer').innerText.includes('交付事实')", 20, "delivery panel")
    modal = ctx.modal_text()
    ctx.contains("delivery shows session8", modal, s8)
    ctx.contains("delivery shows run status", modal, "completed（完成）")
    ctx.contains("delivery shows terminal_source", modal, "terminal_source")
    ctx.contains("delivery shows short text label", modal, "短聊")
    ctx.screenshot("delivery-panel")
    ctx.close_modal()

    data = ctx.eval(
        "(async()=>{const m=await import('/assets/app/state.js');return fetch('/api/delivery?session8='+m.state.boundSession8+'&root='+encodeURIComponent(m.state.sessionRoot||m.state.workspaceCwd)+'&token=%s').then(r=>r.json())})()"
        % ctx.token
    )
    ctx.require("delivery API last_run present", isinstance(data, dict) and bool(data.get("last_run")), json.dumps(data, ensure_ascii=False)[:600])
    ctx.eq("delivery API run status", data["last_run"].get("status"), "completed")
    ctx.contains("delivery API terminal_source", str(data["last_run"].get("terminal_source")), "short_text")


def s_approval(ctx: Ctx) -> None:
    # 真实权限弹窗：browser_control script 是唯一 force_prompt 动作面之一
    ctx.fill("审批并发测试")
    ctx.send()
    ctx.wait("document.getElementById('modalLayer').innerText.includes('权限请求')", 25, "real permission modal")
    real_modal = ctx.modal_text()
    ctx.contains("real permission is browser_control", real_modal, "browser_control")
    ctx.screenshot("approval-real")

    # 第二个权限请求（本尊 dialogs.showPermission）：验证 333 的「顶掉」路径
    ctx.eval(
        "(async()=>{const d=await import('/assets/app/dialogs.js');"
        "d.showPermission({toolCall:{title:'second-tool',kind:'edit'},options:[{optionId:'allow',name:'允许'}]});"
        "return true})()"
    )
    ctx.wait("document.getElementById('modalLayer').innerText.includes('顶掉')", 10, "displaced warning")
    modal2 = ctx.modal_text()
    ctx.contains("displacement warning text", modal2, "上一条权限请求已被本次请求顶掉")
    ctx.contains("displacement counter", modal2, "本页累计顶掉 1 次")
    ctx.contains("new modal shows second tool", modal2, "second-tool")
    ctx.screenshot("approval-displaced")
    ctx.close_modal()
    # 被顶掉的第一条已回 cancelled；真实 run 在无应答下 fail-closed 收尾。
    ctx.wait(
        "(async()=>{const m=await import('/assets/app/state.js');return m.state.running===false})()",
        60,
        "approval run finished",
    )
    ctx.contains("approval run reached terminal copy", ctx.content_text(), "审批场景结束")


def s_context(ctx: Ctx) -> None:
    ctx.prompt("上下文读数测试")
    s8 = (ctx.session_id() or "")[:8]
    ctx.require("session bound", bool(s8), s8)
    ctx.click_toolbar("上下文")
    ctx.wait("document.getElementById('modalLayer').innerText.includes('上下文与压缩')", 20, "context panel")
    modal = ctx.modal_text()
    ctx.contains("context shows estimate", modal, "估算占用")
    ctx.contains("context shows chars/2 method", modal, "chars/2")
    ctx.contains("context shows messages", modal, "消息数")
    ctx.contains("context shows compaction row", modal, "最近压缩")
    ctx.screenshot("context-panel")
    ctx.close_modal()

    data = ctx.eval(
        "(async()=>{const m=await import('/assets/app/state.js');return fetch('/api/context?session8='+m.state.boundSession8+'&root='+encodeURIComponent(m.state.sessionRoot||m.state.workspaceCwd)+'&token=%s').then(r=>r.json())})()"
        % ctx.token
    )
    ctx.require("context API chars>0", isinstance(data, dict) and int(data.get("chars") or 0) > 0, json.dumps(data, ensure_ascii=False)[:500])
    ctx.contains("context API estimate method", str(data.get("estimate_method")), "chars/2")
    ctx.require(
        "context API per-message tokens bounded by chars/2",
        0 < int(data.get("estimated_tokens") or 0) <= int(data.get("chars") or 0) // 2,
        f"chars={data.get('chars')} tokens={data.get('estimated_tokens')}",
    )


def s_background(ctx: Ctx) -> None:
    ctx.prompt("后台任务可见性测试")
    bg = ctx.state("m.state.backgroundTasks")
    ctx.require("background task counted", int(bg or 0) >= 1, f"backgroundTasks={bg}")
    ctx.click_toolbar("属性")
    ctx.wait("document.getElementById('modalLayer').innerText.includes('后台任务')", 15, "properties panel")
    modal = ctx.modal_text()
    ctx.contains("background row", modal, "后台任务")
    ctx.contains("background count", modal, "累计转入后台")
    ctx.contains("background model-only", modal, "仅模型可见")
    ctx.screenshot("background-properties")
    ctx.close_modal()


def s_version_skew(ctx: Ctx) -> None:
    assert ctx.server is not None
    # 运行中的 exe 被 Windows 锁定——重命名是允许的，再用 patch 过的同名探针占位。
    run_exe = ctx.binary
    real = run_exe.with_name("bridge-real.exe")
    if real.exists():
        real.unlink()
    run_exe.rename(real)
    patched = ctx.out / "bridge-patched.exe"
    shutil.copy2(real, patched)
    raw = bytearray(patched.read_bytes())
    if b"0.9.8" not in raw:
        raise CheckFail("version string 0.9.8 not found in binary")
    raw = raw.replace(b"0.9.8", b"0.9.9")
    patched.write_bytes(bytes(raw))
    shutil.copy2(patched, run_exe)

    try:
        ctx.open()
        # /api/boot 为只读探针面；不需要 prompt。
        boot = urllib.request.urlopen(
            f"http://127.0.0.1:{ctx.server.port}/api/boot?token={ctx.server.token}", timeout=15
        ).read().decode()
        b = json.loads(boot)
        ctx.eq("bridge version", b.get("bridge_version"), "0.9.8")
        ctx.eq("child probe", b.get("child_probe"), "ok")
        ctx.eq("child version", b.get("child_version"), "0.9.9")
        ctx.eq("version_skew", b.get("version_skew"), True)
        ctx.wait("document.getElementById('contentScroll').innerText.includes('版本偏斜')", 20, "skew system line")
        ctx.contains("skew system line bridge", ctx.content_text(), "桥 0.9.8")
        ctx.contains("skew system line child", ctx.content_text(), "agent 0.9.9")
        ctx.check(
            "persistent skew banner (334 §3.1)",
            "版本偏斜" in ctx.text("#connBanner"),
            f"banner={ctx.text('#connBanner')!r}",
        )
        ctx.screenshot("version-skew")
    finally:
        # 还原真实 exe（运行中的 bridge 进程继续用已映射 image）
        try:
            if run_exe.exists():
                run_exe.unlink()
            real.rename(run_exe)
        except Exception:
            pass


SCENARIOS = {
    "refresh": ("刷新重连", s_refresh, {}),
    "two-tabs": ("双标签 409", s_two_tabs, {"keepalive": 300}),
    "rewind": ("撤回后重发", s_rewind, {}),
    "delete": ("删除后不复活", s_delete, {}),
    "keepalive": ("保活过期提示", s_keepalive, {"keepalive": 2}),
    "delivery": ("交付面板", s_delivery, {}),
    "approval": ("审批并发", s_approval, {"scenario": "approval"}),
    "context": ("上下文读数", s_context, {}),
    "background": ("后台可见", s_background, {"scenario": "background"}),
    "version-skew": ("版本偏斜", s_version_skew, {"special": "skew"}),
}


def scenario_file(kind: str | None, out: Path) -> Path | None:
    if not kind:
        return None
    out.mkdir(parents=True, exist_ok=True)
    p = out / f"scenario-{kind}.json"
    if kind == "approval":
        data = [
            {
                "tool_calls": [
                    {
                        "name": "browser_control",
                        "arguments": {"action": "script", "script": "1+1"},
                        "call_id": "perm-1",
                    }
                ]
            },
            {"text": "审批场景结束（fake）。"},
        ]
    elif kind == "background":
        data = [
            {
                "tool_calls": [
                    {
                        "name": "run_terminal_cmd",
                        "arguments": {
                            "command": 'python -c "import time; time.sleep(20)"',
                            "description": "s2 background sleep",
                            "is_background": True,
                        },
                        "call_id": "bg-1",
                    }
                ]
            },
            {"text": "后台任务已启动（fake）。"},
            {"text": "后台任务仍可经 get_task_output 回查（fake）。"},
            {"text": "后台任务场景收尾（fake）。"},
            {"text": "后台任务场景收尾（fake）。"},
        ]
    else:
        raise SystemExit(f"unknown scenario kind {kind}")
    p.write_text(json.dumps(data, ensure_ascii=False, indent=2), encoding="utf-8")
    return p


def run_one(name: str, binary: Path, out_root: Path, keep: bool) -> dict:
    title, fn, opts = SCENARIOS[name]
    out = out_root / name
    if out.exists():
        shutil.rmtree(out, ignore_errors=True)
    ctx = Ctx(name=name, title=title, out=out, binary=binary)
    ctx.keepalive = int(opts.get("keepalive", 300))
    ctx.scenario_file = scenario_file(opts.get("scenario"), out)
    if opts.get("special") == "skew":
        # 在独立目录跑桥，才可替换运行中的同名 exe。
        skew_dir = out / "bin"
        skew_dir.mkdir(parents=True, exist_ok=True)
        shutil.copy2(binary, skew_dir / "orz.exe")
        ctx.binary = skew_dir / "orz.exe"
    started = time.time()
    err = ""
    try:
        setup(ctx)
        fn(ctx)
    except CheckFail as e:
        err = str(e)
    except Exception:
        err = traceback.format_exc()
    finally:
        try:
            ctx.screenshot("final")
        except Exception:
            pass
        teardown(ctx)
        ctx.started_at = time.time() - started
        # 迟到失败：任何 check 已红都算失败
        if not err:
            failed = [c for c in ctx.checks if not c.passed]
            if failed:
                err = "; ".join(f"{c.name}: {c.evidence}" for c in failed)
        result = {
            "name": name,
            "title": title,
            "passed": not err,
            "error": err,
            "seconds": round(ctx.started_at, 1),
            "checks": [{"name": c.name, "passed": c.passed, "evidence": c.evidence} for c in ctx.checks],
            "artifacts": ctx.artifacts,
        }
        (out / "result.json").write_text(json.dumps(result, ensure_ascii=False, indent=2), encoding="utf-8")
        if not keep:
            # 保留日志/结果/截图；删掉 Chrome profile 与工作区（节省磁盘）
            shutil.rmtree(out / "chrome-profile", ignore_errors=True)
    return result


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--bin", default=str(DEFAULT_BIN))
    ap.add_argument("--out", default=str(DEFAULT_OUT))
    ap.add_argument("--only", default="")
    ap.add_argument("--keep", action="store_true", help="keep chrome profile / workspace")
    args = ap.parse_args()
    binary = Path(args.bin)
    if not binary.is_file():
        raise SystemExit(f"binary not found: {binary}")
    out_root = Path(args.out)
    names = [n for n in SCENARIOS if not args.only or n in {x.strip() for x in args.only.split(",")}]
    if not names:
        raise SystemExit("no scenarios selected")
    out_root.mkdir(parents=True, exist_ok=True)
    sha = hashlib.sha256(binary.read_bytes()).hexdigest()
    try:
        ver = subprocess.run(
            [str(binary), "--version"], capture_output=True, text=True, timeout=10
        ).stdout.strip()
    except Exception as e:
        ver = f"<probe failed: {e}>"
    meta = {"binary": str(binary), "sha256": sha, "version": ver, "out": str(out_root)}
    print(f"binary: {binary}\nsha256: {sha}\nversion: {ver}", flush=True)
    results = []
    for n in names:
        print(f"=== {n} ({SCENARIOS[n][0]}) ===", flush=True)
        r = run_one(n, binary, out_root, args.keep)
        r["binary_sha256"] = sha
        results.append(r)
        print(f"--- {n}: {'PASS' if r['passed'] else 'FAIL'} ({r['seconds']}s)", flush=True)
        if r["error"]:
            print(r["error"][:2000], flush=True)
    (out_root / "results.json").write_text(
        json.dumps({"meta": meta, "results": results}, ensure_ascii=False, indent=2), encoding="utf-8"
    )
    lines = ["# 0dn S2 真机十条读数", ""]
    for r in results:
        lines.append(f"- {'PASS' if r['passed'] else 'FAIL'} **{r['name']}**（{r['title']}）{r['seconds']}s")
        if r["error"]:
            lines.append(f"  - {r['error'][:500]}")
    (out_root / "SUMMARY.md").write_text("\n".join(lines) + "\n", encoding="utf-8")
    print("\n".join(lines))
    return 0 if all(r["passed"] for r in results) else 1


if __name__ == "__main__":
    raise SystemExit(main())
