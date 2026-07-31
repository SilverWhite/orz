/**
 * ACP Bridge — thin wrapper over Grok ``agent stdio`` child process.
 *
 * Manages a node-pty-backed Grok process that speaks ACP JSON-RPC.
 * Events are parsed from stdout and forwarded to consumers.  The
 * bridge is owned by the extension host; the webview receives
 * structured status updates, not raw ACP events.
 *
 * Architecture note: This bridge does NOT implement its own model
 * loop, session manager, or dialogue renderer.  Grok owns those.
 * The bridge only translates ACP events → assurance panel state.
 */

import * as vscode from "vscode";

// ── types ─────────────────────────────────────────────────────────────────

export interface AcpEvent {
  jsonrpc: "2.0";
  method?: string;
  params?: Record<string, unknown>;
  result?: unknown;
  id?: string | number;
}

export interface AssuranceSnapshot {
  gates: GateSummary[];
  sources: SourceSummary[];
  tools: ToolSummary[];
  session: SessionSummary;
}

interface GateSummary {
  name: string;
  decision: string;
  reason?: string;
}

interface SourceSummary {
  id: string;
  decision: string;
}

interface ToolSummary {
  name: string;
  available: boolean;
}

interface SessionSummary {
  id?: string;
  turns?: number;
  status?: string;
}

// ── bridge ────────────────────────────────────────────────────────────────

export class AcpBridge {
  private _pty: any = null;
  private _buffer = "";
  private _snapshot: AssuranceSnapshot = {
    gates: [],
    sources: [],
    tools: [],
    session: {},
  };
  private _onUpdate?: (snap: AssuranceSnapshot) => void;

  constructor(onUpdate?: (snap: AssuranceSnapshot) => void) {
    this._onUpdate = onUpdate;
  }

  /** Start a Grok ACP child process via node-pty and begin listening. */
  start(prompt: string, cwd: string): void {
    try {
      // Dynamic require so the extension loads even if node-pty is unavailable.
      const pty = require("node-pty");
      this._pty = pty.spawn("grok", ["agent", "stdio"], {
        name: "grok-acp",
        cwd: cwd || process.cwd(),
        env: process.env,
      });

      this._pty.onData((data: string) => {
        this._buffer += data;
        this._processLines();
      });

      this._pty.onExit(() => {
        this._snapshot.session.status = "closed";
        this._notify();
      });

      // Send initialize + session/new + first prompt.
      setTimeout(() => {
        this._send({
          jsonrpc: "2.0",
          method: "initialize",
          params: {
            protocolVersion: 1,
            clientInfo: { name: "gsa-vscode", version: "0.1.0" },
          },
          id: 1,
        });
        setTimeout(() => {
          this._send({
            jsonrpc: "2.0",
            method: "session/new",
            params: {
              modelId: "lif-fake-deepseek",
              maxTurns: 5,
              mcpServers: [],
            },
            id: 2,
          });
          setTimeout(() => {
            this._send({
              jsonrpc: "2.0",
              method: "session/prompt",
              params: { prompt },
              id: 3,
            });
          }, 200);
        }, 200);
      }, 200);
    } catch (err) {
      vscode.window.showErrorMessage(
        `GSA: Failed to start Grok ACP — ${String(err)}`,
      );
    }
  }

  /** Send a raw JSON-RPC message to the Grok process. */
  send(method: string, params: Record<string, unknown> = {}): void {
    this._send({ jsonrpc: "2.0", method, params, id: Date.now() });
  }

  dispose(): void {
    try {
      this._pty?.kill();
    } catch {
      // Best-effort.
    }
    this._pty = null;
  }

  // ── internals ────────────────────────────────────────────────────────

  private _send(msg: AcpEvent): void {
    if (!this._pty) return;
    this._pty.write(JSON.stringify(msg) + "\n");
  }

  private _processLines(): void {
    while (true) {
      const idx = this._buffer.indexOf("\n");
      if (idx === -1) break;
      const line = this._buffer.slice(0, idx).trim();
      this._buffer = this._buffer.slice(idx + 1);
      if (!line) continue;
      try {
        const evt = JSON.parse(line) as AcpEvent;
        this._handleEvent(evt);
      } catch {
        // Non-JSON line (stderr, startup banner) — ignore.
      }
    }
  }

  private _handleEvent(evt: AcpEvent): void {
    const method = evt.method || "";
    const params = evt.params || {};

    // Map ACP events to assurance snapshot updates.
    if (method === "session/update") {
      const update = (params.update || params) as Record<string, unknown>;
      const updateType = update.sessionUpdate || update.type || "";
      if (updateType === "assistant_message" || updateType === "message") {
        const msg = (update.message || update) as Record<string, unknown>;
        const text = (msg.content || msg.text || "") as string;
        if (text && !this._snapshot.session.id) {
          this._snapshot.session = {
            id: String(params.sessionId || "").slice(0, 16),
            turns: 1,
            status: "active",
          };
          this._notify();
        }
      } else if (updateType === "error") {
        this._snapshot.gates.push({
          name: "acp_error",
          decision: "block",
          reason: String(update.message || update.error || "ACP error"),
        });
        this._notify();
      }
    } else if (method === "notification") {
      const notifType = (params.type || params.notificationType || "") as string;
      if (notifType === "status") {
        this._snapshot.session.status = String(params.message || "connected");
        this._notify();
      }
    }
  }

  private _notify(): void {
    if (this._onUpdate) {
      this._onUpdate({ ...this._snapshot });
    }
  }
}
