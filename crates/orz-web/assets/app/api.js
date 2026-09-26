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

/* 0bv ②：归属工作区根（旧会话跨区读面）——只经 query 传递，桥侧对
 * 「当前 ∪ 已信任」集做 fail-closed 校验（未知根 403）。 */
function rootParam(root) {
  return root ? `&root=${encodeURIComponent(root)}` : '';
}

export function fetchRuns(root) {
  return getJson('/api/runs' + rootParam(root));
}

export function fetchBoot() {
  return getJson('/api/boot');
}

export function fetchRunEvents(runId, from, root) {
  return getJson(`/api/run/${encodeURIComponent(runId)}/events?from=${from || 0}${rootParam(root)}`);
}

export function fetchConversations(root) {
  return getJson('/api/conversations' + rootParam(root));
}

/* 会话归档只读投影（0br S3 新增面，用户令 2026-09-25）。 */
export function fetchArchives(root) {
  return getJson('/api/archives' + rootParam(root));
}

export function fetchArchive(session8, root) {
  return getJson(`/api/archives/${encodeURIComponent(session8)}${root ? `?root=${encodeURIComponent(root)}` : ''}`);
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

/* 工作区切换（0bv ②，B 形态：服务随启动而立＋信任清单全局共享＋服务内
 * 点击切换）：POST /api/workspace/switch —— 机械切换（模型零感知）；
 * 未信任工作区桥侧 403（fail-closed，桥回错误消息）；「运行中禁切」由
 * 前端 state.running 主判（载荷＝目标路径，来自信任清单投影）。 */
export function switchWorkspace(path) {
  return sendJsonBody('POST', '/api/workspace/switch', { path });
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
