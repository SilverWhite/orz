/*
 * api.js —— 只读 REST 访问（桥面：/api/runs、/api/run/{id}/events、
 * /api/conversations；令牌来自 URL fragment，S1 §6 分发面）。
 */

let TOKEN = '';

export function setToken(t) {
  TOKEN = t || '';
}

export function getToken() {
  return TOKEN;
}

async function getJson(url) {
  const sep = url.includes('?') ? '&' : '?';
  const resp = await fetch(url + sep + 'token=' + encodeURIComponent(TOKEN));
  if (!resp.ok) {
    const body = await resp.json().catch(() => ({}));
    throw new Error(body?.error?.message || `HTTP ${resp.status}`);
  }
  return resp.json();
}

export function fetchRuns() {
  return getJson('/api/runs');
}

export function fetchBoot() {
  return getJson('/api/boot');
}

export function fetchRunEvents(runId, from) {
  return getJson(`/api/run/${encodeURIComponent(runId)}/events?from=${from || 0}`);
}

export function fetchConversations() {
  return getJson('/api/conversations');
}

/* 会话归档只读投影（0br S3 新增面，用户令 2026-09-25）。 */
export function fetchArchives() {
  return getJson('/api/archives');
}

export function fetchArchive(session8) {
  return getJson(`/api/archives/${encodeURIComponent(session8)}`);
}

/* 归档动作（0br S3 用户令「归档活跃会话」）：POST 递单，执行在桥侧
 * spawn 的 agent 子命令（orz archive <s8>）——前端零执行事实。 */
export async function postArchive(session8) {
  return sendJson('POST', `/api/archives/${encodeURIComponent(session8)}`);
}

/* 信任授信（0br S3 批七）：POST /api/trust —— TUI 信任窗的 Web 等价物。
 * 工作区路径由桥自带（state.cwd），浏览器零路径输入。 */
export function grantTrust() {
  return sendJson('POST', '/api/trust');
}

/* 回档（0br S3 批六用户令）：DELETE 递单 → 桥 spawn `orz unarchive <s8>`
 * ——移除归档包＋水位，会话回活跃组；数据保留。 */
export async function unarchiveSession(session8) {
  return sendJson('DELETE', `/api/archives/${encodeURIComponent(session8)}`);
}

/* 删除（0br S3 批六用户令）：DELETE 递单 → 桥 spawn
 * `orz delete-session <s8>`——彻底移除归档包/水位/侧车/运行 journal，
 * 不可恢复；UI 侧必须先经确认弹窗。 */
export async function deleteSession(session8) {
  return sendJson('DELETE', `/api/sessions/${encodeURIComponent(session8)}`);
}

async function sendJson(method, url) {
  const sep = url.includes('?') ? '&' : '?';
  const resp = await fetch(url + sep + 'token=' + encodeURIComponent(TOKEN), { method });
  const body = await resp.json().catch(() => ({}));
  if (!resp.ok) {
    throw new Error(body?.error?.message || `HTTP ${resp.status}`);
  }
  return body;
}
