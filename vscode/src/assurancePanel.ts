/**
 * Sidebar webview — renders GSA assurance status as compact HTML cards.
 *
 * Communicates with the extension host via ``postMessage``.  Receives
 * structured status updates and renders them in a lightweight, dark-theme
 * friendly card layout.  No React / framework dependency — plain DOM.
 */

import * as vscode from "vscode";

// ── message types ─────────────────────────────────────────────────────────

interface StatusMessage {
  type: "verifyResult" | "reviewResult" | "runResult";
  result: string;
}

// ── HTML template ─────────────────────────────────────────────────────────

function getHtml(): string {
  return /* html */ `<!DOCTYPE html>
<html lang="zh">
<head>
<meta charset="utf-8"/>
<meta name="viewport" content="width=device-width, initial-scale=1"/>
<style>
  :root {
    --bg: var(--vscode-sideBar-background, #1e1e1e);
    --fg: var(--vscode-sideBar-foreground, #cccccc);
    --card-bg: var(--vscode-editor-background, #252526);
    --border: var(--vscode-panel-border, #3c3c3c);
    --green: #4ec9b0;
    --yellow: #dcdcaa;
    --red: #f44747;
    --blue: #569cd6;
  }
  body {
    font-family: var(--vscode-font-family, 'Segoe UI', sans-serif);
    font-size: 12px;
    color: var(--fg);
    background: var(--bg);
    padding: 8px;
    margin: 0;
  }
  .section { margin-bottom: 12px; }
  .section-title {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--blue);
    margin-bottom: 4px;
    padding-bottom: 2px;
    border-bottom: 1px solid var(--border);
  }
  .card {
    background: var(--card-bg);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 6px 8px;
    margin-bottom: 4px;
  }
  .row { display: flex; justify-content: space-between; align-items: center; padding: 2px 0; }
  .label { color: #888; }
  .allow { color: var(--green); }
  .block { color: var(--red); }
  .defer { color: var(--yellow); }
  .mono { font-family: var(--vscode-editor-font-family, 'Cascadia Code', monospace); font-size: 11px; }
  .empty { color: #555; font-style: italic; padding: 4px 0; }
  #status-line { font-size: 11px; padding: 4px 0; color: #888; }
  button {
    width: 100%;
    margin-top: 6px;
    padding: 4px;
    background: var(--vscode-button-background, #0e639c);
    color: var(--vscode-button-foreground, #fff);
    border: none;
    border-radius: 3px;
    cursor: pointer;
    font-size: 11px;
  }
  button:hover { background: var(--vscode-button-hoverBackground, #1177bb); }
</style>
</head>
<body>
  <div class="section">
    <div class="section-title">Gate Decisions</div>
    <div id="gates"><div class="empty">无活跃 gate 决策</div></div>
  </div>

  <div class="section">
    <div class="section-title">Source Visibility</div>
    <div id="sources"><div class="empty">无来源记录</div></div>
  </div>

  <div class="section">
    <div class="section-title">Tool Availability</div>
    <div id="tools"><div class="empty">无工具可用性数据</div></div>
  </div>

  <div class="section">
    <div class="section-title">Session</div>
    <div id="session"><div class="empty">无活跃会话</div></div>
  </div>

  <button onclick="refresh()">Refresh</button>
  <div id="status-line"></div>

<script>
  const vscodeApi = acquireVsCodeApi();

  window.addEventListener("message", (event) => {
    const msg = event.data;
    if (!msg) return;
    if (msg.type === "statusUpdate") {
      renderStatus(msg.gates, msg.sources, msg.tools, msg.session);
    } else if (msg.type === "verifyResult" || msg.type === "reviewResult") {
      document.getElementById("status-line").textContent = msg.result.slice(0, 200);
    }
  });

  function refresh() {
    vscodeApi.postMessage({ type: "refresh" });
  }

  function renderStatus(gates, sources, tools, session) {
    // Gates
    const gatesEl = document.getElementById("gates");
    if (!gates || !gates.length) {
      gatesEl.innerHTML = '<div class="empty">无活跃 gate 决策</div>';
    } else {
      gatesEl.innerHTML = gates.map(g =>
        '<div class="card"><div class="row">' +
        '<span>' + (g.name || "gate") + '</span>' +
        '<span class="' + (g.decision || "defer") + '">' + (g.decision || "?") + '</span>' +
        '</div>' +
        (g.reason ? '<div class="row"><span class="label">' + g.reason.slice(0, 60) + '</span></div>' : "") +
        '</div>'
      ).join("");
    }

    // Sources
    const srcEl = document.getElementById("sources");
    if (!sources || !sources.length) {
      srcEl.innerHTML = '<div class="empty">无来源记录</div>';
    } else {
      srcEl.innerHTML = sources.map(s =>
        '<div class="card"><div class="row">' +
        '<span class="mono">' + (s.id || "?").slice(0, 16) + '</span>' +
        '<span class="' + (s.decision || "defer") + '">' + (s.decision || "?") + '</span>' +
        '</div></div>'
      ).join("");
    }

    // Tools
    const toolEl = document.getElementById("tools");
    if (!tools || !tools.length) {
      toolEl.innerHTML = '<div class="empty">无工具可用性数据</div>';
    } else {
      toolEl.innerHTML = tools.map(t =>
        '<div class="card"><div class="row">' +
        '<span>' + (t.name || "tool") + '</span>' +
        '<span class="' + (t.available ? "allow" : "block") + '">' +
        (t.available ? "available" : "unavailable") + '</span>' +
        '</div></div>'
      ).join("");
    }

    // Session
    const sesEl = document.getElementById("session");
    if (!session || !session.id) {
      sesEl.innerHTML = '<div class="empty">无活跃会话</div>';
    } else {
      sesEl.innerHTML =
        '<div class="card">' +
        '<div class="row"><span>ID</span><span class="mono">' + (session.id || "").slice(0, 12) + '...</span></div>' +
        '<div class="row"><span>Turns</span><span>' + (session.turns || 0) + '</span></div>' +
        '<div class="row"><span>Status</span><span>' + (session.status || "active") + '</span></div>' +
        '</div>';
    }

    document.getElementById("status-line").textContent =
      "Updated " + new Date().toLocaleTimeString();
  }
</script>
</body>
</html>`;
}

// ── provider ──────────────────────────────────────────────────────────────

export class AssurancePanel implements vscode.WebviewViewProvider {
  private _view?: vscode.WebviewView;

  constructor(private readonly _ctx: vscode.ExtensionContext) {}

  resolveWebviewView(
    webviewView: vscode.WebviewView,
    _context: vscode.WebviewViewResolveContext<unknown>,
    _token: vscode.CancellationToken,
  ): void | Thenable<void> {
    this._view = webviewView;
    webviewView.webview.options = {
      enableScripts: true,
      localResourceRoots: [],
    };
    webviewView.webview.html = getHtml();

    // Handle refresh requests from webview.
    webviewView.webview.onDidReceiveMessage((msg) => {
      if (msg.type === "refresh") {
        // Extension host should call updateStatus() to push fresh data.
        vscode.commands.executeCommand("gsa.verify");
      }
    });
  }

  /** Push structured status data to the webview. */
  updateStatus(jsonResult: string): void {
    if (!this._view) return;
    let parsed: any = {};
    try {
      parsed = JSON.parse(jsonResult);
    } catch {
      parsed = {};
    }
    this._view.webview.postMessage({
      type: "statusUpdate",
      gates: parsed.gates || [],
      sources: parsed.sources || [],
      tools: parsed.tools || [],
      session: parsed.session || {},
    });
  }

  /** Send a generic message to the webview. */
  postMessage(msg: StatusMessage): void {
    this._view?.webview.postMessage(msg);
  }

  dispose(): void {
    this._view = undefined;
  }
}
