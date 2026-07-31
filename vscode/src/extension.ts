/**
 * GSA Assurance — VS Code extension entry point.
 *
 * Architecture: thin bridge over Grok ACP.
 *   - Sidebar webview displays assurance status (gates, sources, tools).
 *   - Commands trigger gsa CLI actions.
 *   - ACP bridge connects to ``grok agent stdio`` for real-time state.
 *
 * Does NOT: render dialogue, own model loop, duplicate Grok session mgmt.
 */

import * as vscode from "vscode";
import { AssurancePanel } from "./assurancePanel";
import { AcpBridge } from "./acpBridge";

// ── global state ──────────────────────────────────────────────────────────

let assurancePanel: AssurancePanel | undefined;
let acpBridge: AcpBridge | undefined;
let statusBarItem: vscode.StatusBarItem;

// ── activate ──────────────────────────────────────────────────────────────

export function activate(ctx: vscode.ExtensionContext): void {
  // Status bar — shows gate summary.
  statusBarItem = vscode.window.createStatusBarItem(
    vscode.StatusBarAlignment.Right,
    100,
  );
  statusBarItem.text = "$(shield) GSA — 就绪";
  statusBarItem.tooltip = "GSA Assurance — gate decisions, source visibility";
  statusBarItem.command = "gsa.assurancePanel.focus";
  statusBarItem.show();
  ctx.subscriptions.push(statusBarItem);

  // Sidebar webview provider.
  assurancePanel = new AssurancePanel(ctx);
  ctx.subscriptions.push(
    vscode.window.registerWebviewViewProvider("gsa.assurancePanel", assurancePanel),
  );

  // ── commands ────────────────────────────────────────────────────────────

  ctx.subscriptions.push(
    vscode.commands.registerCommand("gsa.verify", async () => {
      const result = await runGsaCommand(["verify"]);
      assurancePanel?.postMessage({ type: "verifyResult", result });
    }),
  );

  ctx.subscriptions.push(
    vscode.commands.registerCommand("gsa.reviewGlobal", async () => {
      const result = await runGsaCommand(["review", "global"]);
      assurancePanel?.postMessage({ type: "reviewResult", result });
    }),
  );

  ctx.subscriptions.push(
    vscode.commands.registerCommand("gsa.runGrok", async () => {
      const prompt = await vscode.window.showInputBox({
        prompt: "Enter prompt for Grok session",
        placeHolder: "/run 审查当前项目的 assurance 层...",
      });
      if (prompt) {
        const result = await runGsaCommand([
          "run", "--runtime", "grok", "--grok-execute", "--ask", prompt,
        ]);
        assurancePanel?.postMessage({ type: "runResult", result });
      }
    }),
  );

  // ── ACP bridge (lazy — started when panel becomes visible) ──────────────
  // The bridge connects to ``grok agent stdio`` and streams ACP events.
  // For now, we start with a poll-based approach using the gsa CLI.
  updateAssuranceStatus();
}

// ── deactivate ────────────────────────────────────────────────────────────

export function deactivate(): void {
  acpBridge?.dispose();
  acpBridge = undefined;
  assurancePanel?.dispose();
  assurancePanel = undefined;
}

// ── helpers ───────────────────────────────────────────────────────────────

async function runGsaCommand(args: string[]): Promise<string> {
  return new Promise((resolve) => {
    const { execFile } = require("node:child_process");
    const proc = execFile(
      "python", ["-m", "assurance.cli", ...args],
      { cwd: vscode.workspace.workspaceFolders?.[0]?.uri.fsPath, timeout: 30000 },
      (err: Error | null, stdout: string, stderr: string) => {
        if (err) {
          resolve(`Error: ${stderr || err.message}`);
        } else {
          resolve(stdout.slice(0, 2000));
        }
      },
    );
  });
}

async function updateAssuranceStatus(): Promise<void> {
  // Poll gsa doctor for current status and push to webview.
  try {
    const result = await runGsaCommand(["doctor", "--json", "--quick"]);
    assurancePanel?.updateStatus(result);
    statusBarItem.text = "$(shield) GSA";
  } catch {
    statusBarItem.text = "$(warning) GSA — offline";
  }
  // Re-poll every 30s.
  setTimeout(updateAssuranceStatus, 30000);
}
