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
  const resp = await fetch(`/api/archives/${encodeURIComponent(session8)}?token=${encodeURIComponent(TOKEN)}`, { method: 'POST' });
  const body = await resp.json().catch(() => ({}));
  if (!resp.ok) {
    throw new Error(body?.error?.message || `HTTP ${resp.status}`);
  }
  return body;
}
