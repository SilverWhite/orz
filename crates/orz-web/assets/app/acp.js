/*
 * acp.js —— ACP 客户端（JSON-RPC over WebSocket，令牌经 ?token=）。
 * 客户端角色与 orz-tui acp_client.rs 一致：initialize → session/new →
 * session/prompt（+ session/cancel 通知）；接收 session/update 通知与
 * session/request_permission 请求（其余 agent→client 请求回 method-not-found）。
 * 桥为单会话（409 时如实提示）。
 */

let ws = null;
let nextId = 1;
const pending = new Map(); // id -> {resolve, reject}
const handlers = {
  update: null, // (sessionId, update) => void
  permission: null, // (request) => Promise<response>
  closed: null, // () => void
  open: null, // () => void
  // 329（2026-10-21）：桥的 attach 前置帧——刷新/切换视图后重连到仍在跑的
  // 会话（`{session_id, reused, prompt_in_flight, grace_secs}`）；null 表示
  // 新 child（无会话可绑）。
  attached: null, // (params) => void
  // 329：桥主动通知（child 退出 / 断线 backlog 截断）——如实上屏，不静默。
  notice: null, // (kind) => void
};

function send(obj) {
  if (!ws || ws.readyState !== WebSocket.OPEN) throw new Error('桥接未连接');
  ws.send(JSON.stringify(obj));
}

function request(method, params) {
  return new Promise((resolve, reject) => {
    const id = nextId++;
    pending.set(id, { resolve, reject });
    send({ jsonrpc: '2.0', id, method, params });
  });
}

function notify(method, params) {
  send({ jsonrpc: '2.0', method, params });
}

/* 错误细节透出（批七 2026-09-25）：agent 侧把真实原因放在 error.data
 * （如 ACAF fail-closed 的拒绝理由），只读 message 会把一切吞成
 * 「Internal error」。 */
function errorMessage(err) {
  const base = err.message || `JSON-RPC ${err.code}`;
  const data =
    typeof err.data === 'string' ? err.data : err.data != null ? JSON.stringify(err.data) : '';
  return data ? `${base}：${data}` : base;
}

export function on(evt, fn) {
  handlers[evt] = fn;
}

export function connect(token) {
  const proto = location.protocol === 'https:' ? 'wss:' : 'ws:';
  return new Promise((resolve, reject) => {
    let settled = false;
    ws = new WebSocket(`${proto}//${location.host}/ws/acp?token=${encodeURIComponent(token)}`);
    ws.onopen = () => {
      settled = true;
      handlers.open?.();
      resolve();
    };
    ws.onerror = () => {
      if (!settled) reject(new Error('无法连接桥（令牌无效或已有活动会话）'));
    };
    ws.onclose = () => {
      pending.forEach((p) => p.reject(new Error('桥接已关闭')));
      pending.clear();
      handlers.closed?.();
    };
    ws.onmessage = (ev) => {
      let msg;
      try {
        msg = JSON.parse(ev.data);
      } catch {
        return;
      }
      if (msg.id !== undefined && (msg.result !== undefined || msg.error !== undefined)) {
        const p = pending.get(msg.id);
        if (p) {
          pending.delete(msg.id);
          if (msg.error) p.reject(new Error(errorMessage(msg.error)));
          else p.resolve(msg.result);
        }
        return;
      }
      if (msg.method === 'session/update') {
        handlers.update?.(msg.params?.sessionId, msg.params?.update);
      } else if (msg.method === '_orz/bridge_attached') {
        // 329：桥先发此帧再补发断线期间的 backlog；绑定与重放由 main.js 决定。
        handlers.attached?.(msg.params || {});
      } else if (msg.method === '_orz/bridge_child_exited') {
        handlers.notice?.('child_exited');
      } else if (msg.method === '_orz/bridge_backlog_truncated') {
        handlers.notice?.('backlog_truncated');
      } else if (msg.method === 'session/request_permission') {
        // 必须回应（否则 agent 等待超时）：交给 dialogs 的权限弹窗。
        Promise.resolve(handlers.permission?.(msg.params)).then((result) => {
          send({ jsonrpc: '2.0', id: msg.id, result });
        }).catch(() => {
          send({
            jsonrpc: '2.0',
            id: msg.id,
            result: { outcome: { outcome: 'cancelled' } },
          });
        });
      } else if (msg.id !== undefined && msg.method) {
        // 未实现的 agent→client 请求：如实回 method-not-found（不静默）。
        send({
          jsonrpc: '2.0',
          id: msg.id,
          error: { code: -32601, message: `client does not implement ${msg.method}` },
        });
      }
    };
  });
}

export function isConnected() {
  return ws && ws.readyState === WebSocket.OPEN;
}

export async function initialize() {
  // ACP initialize：protocolVersion 1（agent-client-protocol 0.10.x 语义）。
  return request('initialize', {
    protocolVersion: 1,
    clientCapabilities: {
      fs: { readTextFile: false, writeTextFile: false },
      terminal: false,
    },
  });
}

export async function sessionNew(cwd) {
  return request('session/new', { cwd, mcpServers: [] });
}

export async function sessionLoad(sessionId, cwd) {
  return request('session/load', { sessionId, cwd, mcpServers: [] });
}

export async function sessionPrompt(sessionId, text) {
  return request('session/prompt', {
    sessionId,
    prompt: [{ type: 'text', text }],
  });
}

export function sessionCancel(sessionId) {
  notify('session/cancel', { sessionId });
}

/* ── 0di（2026-10-18，ADR-0010 §14.88）：ext 面（`_orz/*`）──
 * ACP 扩展方法走 `_` 前缀线格式（acp 库剥前缀路由为 ExtRequest）；桥为
 * 透明中继，零改动。queue_append＝运行中排队（下一次 LLM 调用边界投递，
 * 不打断）；recall_preview/recall＝撤回（编辑式重发）。 */
async function extRequest(method, params) {
  return request(method, params);
}

export function sessionQueueAppend(sessionId, text) {
  return extRequest('_orz/queue_append', { session_id: sessionId, text });
}

export function sessionRecallPreview(sessionId) {
  return extRequest('_orz/recall_preview', { session_id: sessionId });
}

export function sessionRecall(sessionId, ordinal) {
  return extRequest('_orz/recall', { session_id: sessionId, ordinal });
}
