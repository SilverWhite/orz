#!/usr/bin/env node
/*
 * frontend_smoke.mjs —— 前端最低冒烟门（审查处理批 2026-09-25 新增）。
 *
 * 背景：0br S2 验证止步 HTTP/WS 协议层，main.js 的严格模式 ReferenceError
 * （对未声明变量赋值）直到本门建立才可被机器捕获。本脚本不引入任何依赖、
 * 不做打包：用最小 DOM/浏览器 stub 依次 import 全部应用模块（任何求值期
 * 异常＝失败），并对可纯逻辑验证的面（state 去重窗口/内存上限、keymap
 * 动作分发、dialogs 权限应答结算）跑断言。浏览器内观感/渲染验证仍在
 * S3 真机首读（判据①③④）。
 *
 * 运行：node tests/frontend_smoke.mjs   （退出码 0＝通过）
 */

import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

process.on('unhandledRejection', (e) => {
  console.error('frontend smoke: 未处理的 Promise 拒绝（boot 链）→', e);
  process.exit(1);
});

/* ── 浏览器 stub ── */

const registry = [];

function makeEl(tag) {
  const listeners = {};
  const el = {
    tagName: String(tag).toUpperCase(),
    children: [],
    className: '',
    textContent: '',
    innerHTML: '',
    value: '',
    title: '',
    hidden: false,
    disabled: false,
    tabIndex: 0,
    attributes: {},
    style: {},
    offsetParent: null,
    classList: {
      _s: new Set(),
      add(c) { this._s.add(c); },
      remove(c) { this._s.delete(c); },
      toggle(c, f) { f ? this._s.add(c) : this._s.delete(c); },
      contains(c) { return this._s.has(c); },
    },
    appendChild(c) { this.children.push(c); return c; },
    replaceChildren(...cs) { this.children = cs; },
    insertBefore(n) { this.children.push(n); return n; },
    removeChild(c) { this.children = this.children.filter((x) => x !== c); },
    get firstChild() { return this.children[0] ?? null; },
    addEventListener(t, fn) { (listeners[t] ??= []).push(fn); },
    removeEventListener() {},
    dispatch(t, ev = {}) {
      for (const fn of listeners[t] ?? []) fn({ preventDefault() {}, stopPropagation() {}, target: el, ...ev });
    },
    click() { this.dispatch('click'); },
    setAttribute(k, v) { this.attributes[k] = v; },
    getAttribute(k) { return this.attributes[k] ?? null; },
    removeAttribute(k) { delete this.attributes[k]; },
    focus() {},
    scrollIntoView() {},
    contains() { return false; },
    querySelector() { return null; },
    querySelectorAll() { return []; },
    scrollHeight: 0,
    scrollTop: 0,
    clientHeight: 0,
  };
  registry.push(el);
  return el;
}

const byId = new Map();
const windowListeners = {};
const socketInstances = [];

/* ── ACP 测试替身（318 行为化门）────────────────────────────────────────
 * boot 的 acp.connect 走这里的 WebSocket；send() 把 JSON-RPC 交给
 * mockAcpSend，测试可逐方法应答/挂起（session/load 代次必须可延迟）。
 * 默认：initialize 立即成功；session/new、session/load 未装配时如实报错
 * （避免测试之间静默创建/切换绑定）；其它方法立即成功。 */
const acpMock = {
  calls: [],
  loadHandler: null,
  newHandler: null,
  promptHandler: null,
  queueHandler: null,
  recallPreviewHandler: null,
  recallHandler: null,
  closeHandler: null,
  stateHandler: null,
  deleteHandler: null,
};

function wsRespond(ws, id, result) {
  ws.onmessage?.({ data: JSON.stringify({ jsonrpc: '2.0', id, result }) });
}
function wsRespondError(ws, id, message) {
  ws.onmessage?.({ data: JSON.stringify({ jsonrpc: '2.0', id, error: { code: -32603, message } }) });
}

function mockAcpSend(msg, ws) {
  acpMock.calls.push(msg);
  if (msg.id === undefined) return; // notification（session/cancel）
  const fail = () => wsRespondError(ws, msg.id, `mock: ${msg.method} not armed`);
  switch (msg.method) {
    case 'initialize':
      wsRespond(ws, msg.id, { protocolVersion: 1, agentCapabilities: { loadSession: true } });
      break;
    case 'session/new':
      (acpMock.newHandler ?? fail)(msg, ws);
      break;
    case 'session/load':
      (acpMock.loadHandler ?? fail)(msg, ws);
      break;
    case 'session/prompt':
      (acpMock.promptHandler ?? ((m, w) => wsRespond(w, m.id, { stopReason: 'end_turn' })))(msg, ws);
      break;
    case '_orz/queue_append':
      (acpMock.queueHandler ??
        ((m, w) =>
          wsRespond(w, m.id, {
            queued: true,
            depth: 1,
            capacity: 16,
            remaining: 15,
            per_message_cap: 262144,
            scope: 'session_process',
            persists_across_pages: false,
          })))(msg, ws);
      break;
    case '_orz/recall_preview':
      (acpMock.recallPreviewHandler ?? ((m, w) => wsRespond(w, m.id, { messages: [] })))(msg, ws);
      break;
    case '_orz/recall':
      (acpMock.recallHandler ?? ((m, w) => wsRespond(w, m.id, { recalled: true, recall_text: '' })))(msg, ws);
      break;
    case '_orz/session_close':
      (acpMock.closeHandler ??
        ((m, w) =>
          wsRespond(w, m.id, {
            sessionId: m.params?.sessionId,
            state: 'closed',
            closed: true,
            already: false,
          })))(msg, ws);
      break;
    case '_orz/session_state':
      (acpMock.stateHandler ??
        ((m, w) =>
          wsRespond(w, m.id, {
            sessionId: m.params?.sessionId,
            state: 'idle',
            closed: false,
            already: false,
          })))(msg, ws);
      break;
    case '_orz/session_delete':
      (acpMock.deleteHandler ??
        ((m, w) =>
          wsRespond(w, m.id, {
            sessionId: m.params?.sessionId,
            state: 'deleted',
            deleted: true,
            already: false,
          })))(msg, ws);
      break;
    default:
      fail();
  }
}

globalThis.document = {
  getElementById(id) {
    if (!byId.has(id)) byId.set(id, makeEl('div'));
    return byId.get(id);
  },
  createElement: makeEl,
  createTextNode(text) {
    const node = makeEl('#text');
    node.textContent = String(text ?? '');
    return node;
  },
  addEventListener() {},
  querySelector() { return null; },
  querySelectorAll() { return []; },
  activeElement: null,
  body: makeEl('body'),
};
globalThis.window = globalThis;
globalThis.window.marked = { parse: (s) => `<p>${s}</p>` };
globalThis.addEventListener = (type, fn) => {
  (windowListeners[type] ??= []).push(fn);
};
globalThis.location = {
  protocol: 'http:',
  host: '127.0.0.1:21487',
  href: 'http://127.0.0.1:21487/#token=abc',
  hash: '#token=abc',
};
/* 334（§4.5）：main.js 用 sessionStorage 记忆绑定——浏览器 stub 没有，补
 * 一个内存版（Node 无原生 sessionStorage）。 */
const memoryStorage = new Map();
globalThis.sessionStorage = {
  getItem(key) {
    return memoryStorage.has(key) ? memoryStorage.get(key) : null;
  },
  setItem(key, value) {
    memoryStorage.set(key, String(value));
  },
  removeItem(key) {
    memoryStorage.delete(key);
  },
  clear() {
    memoryStorage.clear();
  },
};
globalThis.WebSocket = class {
  static OPEN = 1;
  constructor(url) {
    this.url = url;
    this.readyState = 0;
    this.sent = [];
    this.onopen = null;
    this.onmessage = null;
    this.onclose = null;
    socketInstances.push(this);
    queueMicrotask(() => {
      this.readyState = 1;
      this.onopen?.();
    });
  }
  send(data) {
    let msg;
    try {
      msg = JSON.parse(data);
    } catch {
      return;
    }
    this.sent.push(msg);
    if (this.url.includes('/ws/acp')) mockAcpSend(msg, this);
  }
  close() {
    this.readyState = 3;
    this.onclose?.();
  }
};
globalThis.fetch = async () => ({ ok: true, json: async () => ({ runs: [], conversations: [] }) });
globalThis.requestAnimationFrame = (fn) => setTimeout(fn, 0);
globalThis.setInterval = () => 0;
globalThis.confirm = () => false;
globalThis.alert = () => {};

/* ── 1. 全模块导入（求值期异常＝失败；回归：main.js 未声明赋值 P0-1） ── */

await import('../assets/app/state.js');
await import('../assets/app/md.js');
await import('../assets/app/api.js');
await import('../assets/app/acp.js');
await import('../assets/app/journal.js');
await import('../assets/app/projection.js');
await import('../assets/app/widgets.js');
await import('../assets/app/dialogs.js');
await import('../assets/app/keymap.js');
await import('../assets/app/main.js');
console.log('ok  全部 10 个应用模块导入无求值期异常');

/* ── 2. state.js 纯逻辑钉子 ── */

const { state, vm, MAX_CONTENT_ITEMS, MAX_TOOL_ENTRIES } = await import('../assets/app/state.js');

vm.addUserMessage('继续');
vm.addUserMessage('继续'); // 相邻重复＝journal 双投递 → 去重
assert.equal(state.content.items.length, 1, '相邻重复用户消息必须去重');
vm.addUserMessage('继续，第二轮');
vm.addUserMessage('继续'); // 隔轮重复是合法行为 → 必须显示（F-2 回归）
assert.equal(state.content.items.length, 3, '隔轮相同消息不得被全历史去重吞掉');
console.log('ok  用户消息去重窗口＝最近一条');

state.content.items = [];
state.content.droppedItems = 0;
for (let i = 0; i < MAX_CONTENT_ITEMS + 10; i++) vm.addSystemMessage(`n${i}`, false);
assert.equal(state.content.items.length, MAX_CONTENT_ITEMS, '内容条目必须封顶');
assert.equal(state.content.droppedItems, 10, '裁掉的最旧条目必须计数');
console.log('ok  内容条目上限与裁剪计数');

const traceLine = state.content.items.find((it) => it.kind === 'tool' && it.toolName === 'bash') ?? null;
vm.addOrUpdateToolTrace('smoke_tool', 't', 'd');
for (let i = 0; i < MAX_TOOL_ENTRIES + 10; i++) vm.addOrUpdateToolTrace('smoke_tool', `t${i}`, 'd');
const line = state.content.items.find((it) => it.kind === 'tool' && it.toolName === 'smoke_tool');
assert.equal(line.entries.length, MAX_TOOL_ENTRIES, '工具清单必须封顶');
assert.equal(line.droppedEntries, 11, '工具清单裁剪必须计数（含首条）');
assert.ok(traceLine === null, '前提：无同名工具行干扰');
console.log('ok  工具清单上限与裁剪计数');

/* ── 3. keymap.js 动作分发（main.js 已绑定真实 actions；stub 事件不抛错） ── */

const { handleKey } = await import('../assets/app/keymap.js');
for (const ev of [
  { key: 'Escape', altKey: false, ctrlKey: false, shiftKey: false, target: { tagName: 'BODY' } },
  { key: 'Tab', altKey: false, ctrlKey: false, shiftKey: false, target: { tagName: 'BODY' } },
  { key: 'Tab', altKey: false, ctrlKey: false, shiftKey: true, target: { tagName: 'BODY' } },
  { key: 'd', altKey: true, ctrlKey: false, shiftKey: false, target: { tagName: 'BODY' } },
  { key: 'F6', altKey: false, ctrlKey: false, shiftKey: false, target: { tagName: 'BODY' } },
  { key: 'l', altKey: false, ctrlKey: true, shiftKey: false, target: { tagName: 'BODY' } },
]) {
  handleKey({ ...ev, preventDefault() {} });
}
console.log('ok  键盘事件分发不抛错（Esc/Tab/Shift+Tab/Alt+字母/F6/Ctrl+L）');

/* ── 4. widgets 菜单按名打开（Alt+字母 承载面） ── */

const widgets = await import('../assets/app/widgets.js');
const { ui } = widgets;
ui.menubar = globalThis.document.getElementById('menubar');
ui.toolbar = globalThis.document.getElementById('toolbar');
ui.explorer = globalThis.document.getElementById('explorer');
ui.content = globalThis.document.getElementById('content');
ui.contentScroll = globalThis.document.getElementById('contentScroll');
ui.toolPin = globalThis.document.getElementById('toolPin');
ui.marker = globalThis.document.getElementById('marker');
ui.statusbar = globalThis.document.getElementById('statusbar');
widgets.renderMenubar();
assert.equal(ui.menubar.children.length, state.menu.length, '菜单项数量');
assert.ok(state.menu.includes('编辑模式'), '菜单词表含「编辑模式」（补充稿 §13 冻结转换）');
console.log('ok  MenuBar 渲染与「编辑模式」词表');

/* ── 4b. 工具栏「新建会话」居「后退」之前（用户令 2026-09-25） ── */

widgets.renderToolbar();
const toolbarBtns = ui.toolbar.children.filter((n) => n.tagName === 'BUTTON').map((n) => n.textContent);
assert.equal(toolbarBtns[0], '新建会话', '「新建会话」必须居工具栏首位');
assert.equal(toolbarBtns[1], '后退', '「后退」必须紧随「新建会话」之后（用户令「放在后退的前面」）');
const selfcheckBtn = ui.toolbar.children.find((n) => n.textContent === '反例自查');
assert.ok(selfcheckBtn, '工具栏必须常驻「反例自查」入口');
assert.equal(selfcheckBtn.disabled, true, '空闲时「反例自查」必须禁用（0dm）');
assert.equal(typeof ui.newSession, 'function', '新建会话动作已接线');
// 0dk S2（332）：关闭会话入口存在，且运行中/无绑定时禁用。
const closeToolbarBtn = ui.toolbar.children.find((n) => n.textContent === '关闭会话');
assert.ok(closeToolbarBtn, '工具栏必须常驻「关闭会话」入口（0dk S2）');
assert.equal(closeToolbarBtn.disabled, true, '无绑定会话时「关闭会话」必须禁用');
vm.setRunning(true);
vm.syncToolbar();
widgets.renderToolbar();
const closeRunningBtn = ui.toolbar.children.find((n) => n.textContent === '关闭会话');
assert.equal(closeRunningBtn.disabled, true, '运行中「关闭会话」必须禁用（先停止再关闭）');
vm.setRunning(false);
vm.syncToolbar();
console.log('ok  工具栏「新建会话」居「后退」之前＋关闭会话入口（运行中禁用）');

/* ── 5. dialogs 权限应答结算（审查处理批 C-1 回归） ── */

const dialogs = await import('../assets/app/dialogs.js');
const before = registry.length;
const req = (name) => ({ toolCall: { title: 'bash', kind: 'exec' }, options: [{ optionId: 'allow_once', name }] });

const p1 = dialogs.showPermission(req('允许一'));
const p2 = dialogs.showPermission(req('允许二')); // 并发第二请求：第一请求被顶掉
const r1 = await p1;
assert.equal(r1.outcome.outcome, 'cancelled', '被顶掉的权限请求必须如实回 cancelled');
assert.equal((await Promise.race([p2, Promise.resolve('pending')])), 'pending', '第二请求仍待应答');
dialogs.closeModal(); // Esc 路径
const r2 = await p2;
assert.equal(r2.outcome.outcome, 'cancelled', 'Esc 关闭权限弹窗必须回 cancelled（不再永挂）');

const p3 = dialogs.showPermission(req('允许'));
// 挂载回归钉（实机截图抓到的 P0：body 游离 → 真浏览器只渲染空标题栏）。
const layerEl = globalThis.document.getElementById('modalLayer');
const winEl = layerEl.children[0];
const bodyEl = winEl?.children?.find((c) => c.className === 'window-body');
assert.ok(bodyEl, '弹窗正文 window-body 必须挂进窗口节点');
assert.equal(layerEl.classList.contains('open'), true, '模态层必须处于打开态');
const btn = registry.slice(before).find((e) => e.tagName === 'BUTTON' && e.textContent === '允许');
btn.click();
const r3 = await p3;
assert.equal(r3.outcome.outcome, 'selected', '选项按钮应答 selected');
assert.equal(r3.outcome.optionId, 'allow_once', 'optionId 透传');
// 标题栏关闭钮（走查反馈四 2026-09-25）：每个弹窗必须有关闭 affordance，
// 语义＝取消（权限弹窗如实回传 cancelled）。
const p4 = dialogs.showPermission(req('允许四'));
const closeBtn = registry
  .filter((e) => e.tagName === 'BUTTON' && e.attributes['aria-label'] === 'Close')
  .at(-1);
assert.ok(closeBtn, '弹窗标题栏必须有关闭钮');
closeBtn.click();
const r4 = await p4;
assert.equal(r4.outcome.outcome, 'cancelled', '标题栏关闭钮＝取消语义');
console.log('ok  权限弹窗 Esc/并发/选项三条应答路径＋正文挂载');

/* ── 6. md.js 管线（stub DOM 下仅验不抛错与返回形态） ── */

const md = await import('../assets/app/md.js');
const html = md.renderMarkdown('# 标题\n\n<b>字面</b>');
assert.equal(typeof html, 'string');
console.log('ok  renderMarkdown 求值通过（净空扫描的 DOM 语义归 S3 浏览器复验）');

/* ── 7. 走查反馈批钉子（2026-09-25）：空白卡根除＋输入/输出锚点 ── */

state.content = { items: [], currentModelIndex: null, droppedItems: 0 };
state.marker = [];
state.turnCounter = 0;
const { applyJournalEvent } = await import('../assets/app/projection.js');

applyJournalEvent({ event_type: 'prompt_submitted', payload: { prompt: '问题一' } });
assert.equal(state.marker[0]?.kind, 'user', '用户输入必须有标记锚点');
assert.match(state.marker[0].label, /T1 输入/);

// 纯工具调用、无正文轮：不得再造空白对话卡（实机走查实证的空白根因）。
applyJournalEvent({
  event_type: 'model_output',
  payload: { text: '', tool_calls: [{ name: 'bash', arguments: 'ls', call_id: 'c1' }] },
});
assert.equal(
  state.content.items.some((it) => it.kind === 'message' && it.content === ''),
  false,
  '纯工具轮不得留空白模型卡',
);

// 中间轮/流式段不产输出锚点；终态才回溯定「最终输出」锚。
applyJournalEvent({ event_type: 'model_output', payload: { text: '答案正文' } });
applyJournalEvent({ event_type: 'model_output', payload: { text: '第二段（最终）' } });
assert.equal(state.marker.some((m) => m.kind === 'model'), false, '运行中不得有输出锚点（只标最终输出）');
applyJournalEvent({ event_type: 'run_finished', payload: { status: 'completed' } });
const modelAnchors = state.marker.filter((m) => m.kind === 'model');
assert.equal(modelAnchors.length, 1, '终态后恰一个最终输出锚');
assert.match(modelAnchors[0].label, /T1 最终输出/);
assert.equal(state.content.items[modelAnchors[0].itemIndex].content, '第二段（最终）', '锚必须指向最后一张有正文的模型卡');

// 合并会话视图（走查反馈六）：第二个运行的终态不清除上一运行的锚——
// 每次对话各自保留正式输出定位。
applyJournalEvent({ event_type: 'prompt_submitted', payload: { prompt: '问题二' } });
applyJournalEvent({ event_type: 'model_output', payload: { text: '第二个回答' } });
applyJournalEvent({ event_type: 'run_finished', payload: {} });
const anchors2 = state.marker.filter((m) => m.kind === 'model');
assert.equal(anchors2.length, 2, '合并视图下各运行的最终输出锚并存');
assert.match(anchors2[1].label, /T2 最终输出/);
assert.equal(state.content.items[anchors2[0].itemIndex].content, '第二段（最终）', '第一运行锚不被第二运行挤掉');
assert.equal(state.content.items[anchors2[1].itemIndex].content, '第二个回答');

// clearHits 只清命中、保留锚点。
vm.addSearchHit('L2', 1);
vm.clearHits();
assert.equal(state.marker.some((m) => m.kind === 'hit'), false, '命中已清');
assert.equal(state.marker.filter((m) => m.kind === 'user' || m.kind === 'model').length, state.marker.length, '锚点必须保留');
console.log('ok  空白卡根除＋输入/最终输出锚点＋clearHits 保留锚点');

// 2026-10-21 恢复修复：会话恢复后 tail 重放最新 run，turn 号必须取 run
// 后缀（prompt_number+1），不能从本地 0 重新累加——否则 RUN-…-18 会被
// 标成 T1，新输入也整体错位。
state.content = { items: [], currentModelIndex: null, droppedItems: 0 };
state.marker = [];
state.turnCounter = 0;
applyJournalEvent({
  run_id: 'RUN-abcdef12-17',
  event_type: 'prompt_submitted',
  payload: { prompt: '恢复后的续接问题' },
});
assert.equal(state.turnCounter, 18, 'turn 必须取 run 序号 17＋1');
assert.match(state.marker[0]?.label || '', /T18 输入/, '恢复后标记栏不得从 T1 重排');
console.log('ok  恢复后 turn 号按 run 序号锚定（标记栏）');

/* ── 8. 归档投影钉子（0br S3 新增面 2026-09-25）：API 面／只读转写／
 *    探索器三组重构＋归档动作键＋进行中/未完成语义 ── */

const api = await import('../assets/app/api.js');
assert.equal(typeof api.fetchArchives, 'function', '归档清单 API 客户端');
assert.equal(typeof api.fetchArchive, 'function', '归档详情 API 客户端');
assert.equal(typeof api.postArchive, 'function', '归档动作 API 客户端');
assert.equal(typeof api.unarchiveSession, 'function', '回档动作 API 客户端（批六）');
assert.equal(typeof api.deleteSession, 'function', '删除动作 API 客户端（批六）');
assert.equal(typeof api.grantTrust, 'function', '信任授信 API 客户端（批七）');
assert.equal(typeof api.switchWorkspace, 'function', '工作区切换 API 客户端（0bv ②）');

// REV-083-22 回归钉（F-2）：switchWorkspace 必须真实发出带 JSON 载荷的
// POST——修复前引用未定义符号 sendJsonBody ⇒ ReferenceError，UI 点击切换
// 从未发出请求（「函数存在」断言抓不到，必须走真实 fetch 链路）。
{
  const calls = [];
  const realFetch = globalThis.fetch;
  globalThis.fetch = async (url, init) => {
    calls.push({ url: String(url), init: init ?? {} });
    return { ok: true, json: async () => ({ ok: true, cwd: 'D:\\proj-b' }) };
  };
  try {
    api.setToken('tok-smoke');
    await api.switchWorkspace('D:\\proj-b');
  } finally {
    globalThis.fetch = realFetch;
  }
  assert.equal(calls.length, 1, 'switchWorkspace 必须恰好发出一次请求（F-2：此前为零次）');
  assert.ok(calls[0].url.startsWith('/api/workspace/switch?'), `切换端点：${calls[0].url}`);
  assert.ok(calls[0].url.includes('token=tok-smoke'), 'token 随 query 携带');
  assert.equal(calls[0].init.method, 'POST', 'POST 方法');
  assert.equal(calls[0].init.headers['Content-Type'], 'application/json', 'JSON 载荷头');
  assert.deepEqual(JSON.parse(calls[0].init.body), { path: 'D:\\proj-b' }, '载荷含目标路径');
}
console.log('ok  工作区切换真实 fetch 链路（REV-083-22 回归钉）');

state.content = { items: [], currentModelIndex: null, droppedItems: 0 };
state.marker = [];
state.turnCounter = 0;
vm.addArchivedMessage('用户', '归档里的问题', 1);
vm.addArchivedMessage('模型', '归档里的回答', 1);
assert.equal(state.content.items.length, 2, '归档转写逐条入内容区');
assert.equal(state.marker.length, 1, '归档用户输入产锚（模型卡不逐轮产锚）');
assert.equal(state.marker[0].kind, 'user');
assert.match(state.marker[0].label, /T1 输入/);
vm.anchorFinalOutput();
assert.equal(state.marker.length, 2, '归档终态回溯定最终输出锚');
assert.equal(state.marker[1].kind, 'model');
assert.equal(state.marker[1].itemIndex, 1, '输出锚指向模型卡');
console.log('ok  归档转写装填＋标记栏锚点（批四）');

// 探索器三组（用户令 2026-09-25）：工作区／活跃会话／归档会话——
// 归档内容不混入活跃组；活跃组行带归档动作键；仅存归档者入归档组。
state.runs = [
  { run_id: 'RUN-6aab1234-1', status: 'completed' },
  { run_id: 'RUN-6aabffff-1', status: 'running' }, // journal 无终态＝未完成
];
state.conversations = [];
state.archives = [{ session8: '6aacdead', size_bytes: 1, modified_ms: 1 }];
state.liveRunId = null;
state.running = false;
widgets.renderExplorer();
let tree = JSON.stringify(ui.explorer.children);
assert.ok(tree.includes('工作区'), '工作区组头');
assert.ok(tree.includes('当前:'), '当前工作区子级');
assert.ok(tree.includes('已信任工作区（0）'), '已信任工作区子级');
assert.ok(tree.includes('活跃会话（2）'), '活跃会话组头');
assert.ok(tree.includes('归档会话（1）'), '归档会话组头');
assert.ok(tree.includes('6aacdead'), '仅存归档的会话入归档组');
assert.ok(
  tree.includes('6aabffff') && tree.includes('（1 次运行 · 未完成）'),
  'journal 无终态旧 run 标「未完成」（ID 与状态双列分体渲染）',
);
assert.ok(!tree.includes('进行中'), '非实时窗口不得标「进行中」');

// 「进行中」唯一判据＝实时窗口正在运行（liveRunId＋running）。
state.running = true;
state.liveRunId = 'RUN-6aab1234-1';
widgets.renderExplorer();
tree = JSON.stringify(ui.explorer.children);
assert.ok(
  tree.includes('6aab1234') && tree.includes('（1 次运行 · 进行中）'),
  '实时窗口标「进行中」',
);
assert.ok(
  !tree.includes('（1 次运行 · 进行中）') === false &&
    [...tree.matchAll(/（1 次运行 · 进行中）/g)].length === 1,
  '其它中断会话不得跟着标「进行中」（进行中标记全局唯一）',
);
state.running = false;
state.liveRunId = null;

// 分组规则（批三修正）：归档包 mtime ≥ 最新运行 mtime ⇒ 归档组——
// 归档成功即移组；归档后又继续（run 更新）则回活跃组。
state.runs = [{ run_id: 'RUN-6aab1234-1', status: 'completed', modified_ms: 100 }];
state.archives = [{ session8: '6aab1234', size_bytes: 1, modified_ms: 200, archived_at: '2026-09-25T10:00:00Z' }];
widgets.renderExplorer();
tree = JSON.stringify(ui.explorer.children);
assert.ok(tree.includes('活跃会话（0）') && tree.includes('归档会话（1）'), '归档成功后会话移入归档组');
assert.ok(
  tree.includes('6aab1234') && tree.includes(' · 2026-09-25 10:00'),
  '归档行带归档时间（ID 与时间双列分体渲染）',
);

state.archives = [{ session8: '6aab1234', size_bytes: 1, modified_ms: 50, archived_at: '2026-09-25T09:00:00Z' }];
widgets.renderExplorer();
tree = JSON.stringify(ui.explorer.children);
assert.ok(tree.includes('活跃会话（1）') && tree.includes('归档会话（0）'), '归档后又继续的会话回活跃组');

// 分组规则补钉（新建会话批 2026-09-25）：ARC- 审计 journal 由归档动作
// 自身写入（恒晚于归档包落盘），不得计入活动水位——否则每次新归档都被
// 顶回活跃组（用户令「刷新后也没有将活跃会话归档」；实证 6ab6275c/6ab6570c）。
state.runs = [
  { run_id: 'RUN-6aab1234-1', status: 'completed', modified_ms: 100 },
  { run_id: 'ARC-6aab1234-1', status: 'completed', modified_ms: 300 },
];
state.archives = [{ session8: '6aab1234', size_bytes: 1, modified_ms: 200, archived_at: '2026-09-25T11:00:00Z' }];
widgets.renderExplorer();
tree = JSON.stringify(ui.explorer.children);
assert.ok(
  tree.includes('活跃会话（0）') && tree.includes('归档会话（1）'),
  'ARC 审计 journal 不得把新归档顶回活跃组',
);
// 反向：归档后真实新 run（mtime 更晚）仍如实回活跃组。
state.runs = [
  { run_id: 'RUN-6aab1234-2', status: 'completed', modified_ms: 400 },
  { run_id: 'ARC-6aab1234-1', status: 'completed', modified_ms: 300 },
];
widgets.renderExplorer();
tree = JSON.stringify(ui.explorer.children);
assert.ok(tree.includes('活跃会话（1）') && tree.includes('归档会话（0）'), '归档后真实新 run 仍如实回活跃组');

// 340：conversation 侧车继续存在不等于活跃；`.closed` 的 API 投影必须
// 单列为可恢复会话，避免 UI 把已关闭会话冒充当前活跃绑定。
state.runs = [
  { run_id: 'RUN-6aab1111-1', status: 'completed', modified_ms: 100 },
  { run_id: 'RUN-c23e0aaa-26', status: 'completed', modified_ms: 200 },
];
state.conversations = [
  {
    session8: '6aab1111',
    session_id: '6aab1111-1111-2222-3333-444455556666',
    closed: false,
  },
  {
    session8: 'c23e0aaa',
    session_id: 'c23e0aaa-e752-4215-9a4f-0e435ff09b6a',
    closed: true,
    legacy_short_id: true,
  },
];
state.archives = [];
widgets.renderExplorer();
tree = JSON.stringify(ui.explorer.children);
assert.ok(tree.includes('活跃会话（1）'), '只有未关闭侧车可计入活跃会话');
assert.ok(tree.includes('已关闭会话（1）'), 'closed 侧车必须保留独立可恢复入口');
assert.ok(tree.includes('c23e0aaa') && tree.includes('可恢复'), '关闭会话行必须可见且如实标注');
state.conversations = [];
console.log('ok  closed 会话不再冒充活跃，会保留独立恢复入口');

/* ── 8b. 组级功能键＋选中模型（批六用户令 2026-09-25）：
 *    「归档」与「活跃会话」组头同行、「回档」「删除」与「归档会话」
 *    组头同行；单击选中、双击打开；未选中＝禁用。 ── */

state.runs = [
  { run_id: 'RUN-6aab1111-1', status: 'completed', modified_ms: 100 },
  { run_id: 'RUN-6aab2222-1', status: 'completed', modified_ms: 300 },
];
state.archives = [{ session8: '6aab2222', size_bytes: 1, modified_ms: 400, archived_at: '2026-09-25T12:00:00Z' }];
state.selected = null;

const groupActions = () =>
  ui.explorer.children
    .filter((n) => n.className === 'explorer-group')
    .flatMap((n) => n.children || [])
    .filter((c) => (c.className || '').startsWith('group-action'))
    // stub 的 classList 与 className 字符串不同步——按字符串解析禁用态。
    .map((c) => ({ label: c.textContent, disabled: (c.className || '').split(' ').includes('disabled') }));

widgets.renderExplorer();
assert.ok(
  !JSON.stringify(ui.explorer.children).includes('◆归档'),
  '行内「◆归档」动作键必须退役（改组级功能键）',
);
let actions = groupActions();
assert.deepEqual(actions.map((a) => a.label), ['归档', '回档', '删除'], '三个功能键与各自组头同行');
assert.ok(actions.every((a) => a.disabled), '未选中时三个功能键全部禁用');

// 选中活跃会话 ⇒ 仅「归档」生效。
state.selected = { group: 'active', s8: '6aab1111' };
widgets.renderExplorer();
actions = groupActions();
assert.equal(actions.find((a) => a.label === '归档').disabled, false, '选中活跃会话后「归档」可用');
assert.ok(
  actions.filter((a) => a.label !== '归档').every((a) => a.disabled),
  '选中活跃会话不得点亮回档/删除',
);
const activeRow = ui.explorer.children.find(
  (n) => (n.className || '').includes('explorer-row') && n.children?.[0]?.textContent === '6aab1111',
);
assert.ok(activeRow.classList.contains('explorer-selected'), '选中行必须有高亮类');
// 批七修正（用户报告「点击其他对话，主窗口不会切换了」）：单击＝打开
// （主窗口切换）＋选中——不再有只选不开的中间态。
activeRow.dispatch('click');
assert.equal(state.nav.current, 'conversation://6aab1111', '单击即打开（主窗口切换）');
assert.ok(state.selected?.s8 === '6aab1111', '单击同时保持选中');

// 选中归档会话 ⇒ 「回档」「删除」生效、「归档」禁用。
state.selected = { group: 'archived', s8: '6aab2222' };
widgets.renderExplorer();
actions = groupActions();
assert.equal(actions.find((a) => a.label === '回档').disabled, false, '选中归档会话后「回档」可用');
assert.equal(actions.find((a) => a.label === '删除').disabled, false, '选中归档会话后「删除」可用');
assert.equal(actions.find((a) => a.label === '归档').disabled, true, '选中归档会话不得点亮「归档」');

state.selected = null;
state.archives = [];
state.runs = [];
console.log('ok  组级功能键（归档/回档/删除与组头同行）＋单击选中模型');

/* ── 9. 新建会话动作（新建会话批 2026-09-25）：归档收尾流程的入口 ── */

// 桥未连接（stub）路径：同步复位面先落地，ensureSession 走失败支路且
// 不残留旧会话绑定。
state.nav = { current: 'run://zz', back: ['workspace://live'], forward: [] };
ui.newSession();
await new Promise((r) => setTimeout(r, 0));
assert.equal(state.nav.current, 'workspace://live', '新建会话复位导航栈');
assert.equal(state.nav.back.length, 0, '新建会话清空后退历史');
assert.equal(state.sessionId, null, '桥不可达时不得残留旧会话绑定');
assert.equal(state.running, false, '新建会话不改运行态');
console.log('ok  新建会话复位导航/内容区并解绑旧会话');

/* ── 10. 0de N5（2026-10-16，277 批设计 §4/§7，D6）：压缩事件投影双字段兼容 ── */

// clear 形 payload 只带 `target_tokens`（无 `estimated_tokens_after`）——
// 模板固定读后者曾把实际 171K 渲成「估 ~0K」（0de 事发 F3）。
state.content = { items: [], currentModelIndex: null, droppedItems: 0 };
applyJournalEvent({
  event_type: 'context_compressed',
  payload: { trigger_tokens: 382271, target_tokens: 171261, rounds_dropped: 87, mode: 'clear' },
});
const clearCard = state.content.items.filter((it) => it.kind === 'message').pop();
assert.match(clearCard.content, /估 ~171K/, 'clear 事件须回退 target_tokens 渲染实际读数');
assert.doesNotMatch(clearCard.content, /估 ~0K/, '不得再渲染「估 ~0K」字段漂移');

// model_summary 形两字段并存（恒等值）⇒ 取值不变。
applyJournalEvent({
  event_type: 'context_compressed',
  payload: {
    trigger_tokens: 537319,
    target_tokens: 239724,
    estimated_tokens_after: 239724,
    rounds_dropped: 32,
    mode: 'model_summary',
  },
});
const summaryCard = state.content.items.filter((it) => it.kind === 'message').pop();
assert.match(summaryCard.content, /估 ~240K/, 'model_summary 事件取值不变');
console.log('ok  0de N5 压缩事件投影双字段兼容（clear 回退 target_tokens）');

/* ── 11. 285 批（2026-10-16 真机走查处置）：机械审查嵌套载荷＋反例门静默
 *     折叠＋收尾只折机器行（画图轮 journal 复现的三条渲染事实） ── */

// F3：本事件 payload 是 `{kind, payload:{key,round,summary,anomaly}}`——比其余
// 事件多一层。按平铺读会渲出「[机械审查] （轮 0）」幽灵行（真机走查实证：
// 一串无键无轮号的机械审查行）。
state.content = { items: [], currentModelIndex: null, droppedItems: 0 };
state.marker = [];
state.turnCounter = 0;
applyJournalEvent({
  event_type: 'mechanical_audit_update',
  payload: {
    kind: 'tool_result',
    payload: { key: 'cmd:call-1', round: 7, summary: 'exit 1', anomaly: 'exit 1' },
  },
});
const auditLine = state.content.items.filter((it) => it.kind === 'message').pop();
assert.match(auditLine.content, /\[机械审查\] cmd:call-1（轮 7）：exit 1/, '嵌套载荷须读到真键与真轮号');
assert.doesNotMatch(auditLine.content, /（轮 0）/, '不得再渲「（轮 0）」幽灵行');
// 无异常形（键＋轮号即可，摘要照旧随行）。
applyJournalEvent({
  event_type: 'mechanical_audit_update',
  payload: { kind: 'tool_result', payload: { key: 'file:out.txt', round: 2, summary: '120 行', anomaly: null } },
});
const auditLine2 = state.content.items.filter((it) => it.kind === 'message').pop();
assert.match(auditLine2.content, /\[机械审查\] file:out\.txt（轮 2）120 行/, '无异常形带摘要');
console.log('ok  285 F3 机械审查嵌套载荷（真键真轮号，幽灵行根除）');

// F2：反例门静默折叠——门前的草稿折起、门后正式输出保持展开；用户卡与
// 上一轮输出不动。F1：收尾（run_finished）不再折对话卡，只折机器行。
state.content = { items: [], currentModelIndex: null, droppedItems: 0 };
state.marker = [];
state.turnCounter = 0;
const findMsg = (text) => state.content.items.find((it) => it.kind === 'message' && it.content === text);

applyJournalEvent({ event_type: 'prompt_submitted', payload: { prompt: '上一轮问题' } });
applyJournalEvent({ event_type: 'model_output', payload: { text: '上一轮答案' } });
applyJournalEvent({ event_type: 'run_finished', payload: { status: 'completed' } });
assert.equal(findMsg('上一轮答案').collapsed, false, 'F1：收尾不得折对话卡（模型卡保持可读）');
assert.equal(findMsg('上一轮问题').collapsed, false, 'F1：收尾不得折用户卡');

applyJournalEvent({ event_type: 'prompt_submitted', payload: { prompt: '本轮问题' } });
applyJournalEvent({ event_type: 'tool_started', payload: { tool: 'run_terminal_cmd', call_id: 'c1' } });
const gateTool = state.content.items.find((it) => it.kind === 'tool');
gateTool.expanded = true;
applyJournalEvent({ event_type: 'model_output', payload: { text: '门前草稿' } });
applyJournalEvent({ event_type: 'counterexample_gate', payload: { position: 'final_answer', once_only: true } });
assert.equal(findMsg('门前草稿').collapsed, true, 'F2：门到即静默折叠门前草稿');
assert.equal(gateTool.expanded, false, 'F2：门到即折机器行');
assert.equal(findMsg('本轮问题').collapsed, false, 'F2：用户卡不折（提问要看得见）');
assert.equal(findMsg('上一轮答案').collapsed, false, 'F2：前几轮输出不受本轮门影响');
assert.equal(state.content.currentModelIndex, null, 'F2：流式指针复位，门后正式输出自成新卡');

applyJournalEvent({ event_type: 'model_output', payload: { text: '正式输出' } });
applyJournalEvent({ event_type: 'run_finished', payload: { status: 'completed' } });
assert.equal(findMsg('正式输出').collapsed, false, 'F1：正式输出收尾后保持展开');
assert.equal(findMsg('门前草稿').collapsed, true, 'F2：门前草稿保持折起');
assert.equal(gateTool.expanded, false, 'F1：收尾折机器行');
const modelAnchors3 = state.marker.filter((m) => m.kind === 'model');
assert.equal(modelAnchors3.length, 2, '两轮各一个最终输出锚');
assert.equal(
  state.content.items[modelAnchors3[1].itemIndex].content,
  '正式输出',
  '本轮锚指向门后正式输出',
);
console.log('ok  285 F1/F2 反例门静默折叠＋收尾只折机器行（对话卡保持可读）');

/* ── 12. 0dg（2026-10-18）：对话卡 pre-wrap＋Ctrl+Z 输入框豁免与闲置反馈 ── */

// ①用户/系统卡纯文本面带 .plain（pre-wrap 白名单只作用此面；模型卡
//   markdown 面不加——\n 由段落化消费）；textContent 保留源换行；
//   app.css 须含 .plain 与 .system-line 两条规则。
state.content = { items: [], currentModelIndex: null, droppedItems: 0 };
state.marker = [];
state.turnCounter = 0;
vm.addUserMessage('第一行\n第二行');
applyJournalEvent({ event_type: 'model_output', payload: { text: '模型回答' } });
applyJournalEvent({ event_type: 'run_finished', payload: {} });
widgets.renderContent();
const plainBody = registry.filter((e) => (e.className || '') === 'msg-body plain').at(-1);
const mdBody = registry.filter((e) => (e.className || '') === 'msg-body').at(-1);
assert.ok(plainBody, '非模型卡 body 必须带 .plain（pre-wrap 作用面）');
assert.ok(plainBody.textContent.includes('\n'), 'textContent 保留源换行（折叠发生在 CSS 层）');
assert.ok(mdBody, '模型卡 body 存在（对照组）');
assert.equal(mdBody.textContent, '', '模型卡 body 不带 .plain（markdown 走 innerHTML）');
const css = readFileSync(new URL('../assets/app.css', import.meta.url), 'utf8');
assert.match(css, /\.msg-card \.msg-body\.plain \{ white-space: pre-wrap; \}/, 'app.css 必须含 .plain pre-wrap 规则');
assert.match(css, /\.system-line \{[^}]*white-space: pre-wrap/s, '系统行同面补 pre-wrap');
console.log('ok  0dg ① 对话卡 pre-wrap（.plain 作用面限定＋system-line 同面）');

// ②输入框内 Ctrl+Z＝放行原生文本撤销（不调 stopRun、不 preventDefault）；
//   输入框外仍取消运行；闲置期按下给「当前无运行中」反馈（不再静默 return）。
const connBanner = globalThis.document.getElementById('connBanner');
const bannerBefore = connBanner.textContent;
let prevented = 0;
handleKey({
  key: 'z',
  ctrlKey: true,
  altKey: false,
  shiftKey: false,
  target: { tagName: 'TEXTAREA' },
  preventDefault() {
    prevented++;
  },
});
assert.equal(prevented, 0, '输入框内 Ctrl+Z 不得被全局键位抢占');
assert.equal(connBanner.textContent, bannerBefore, '输入框内 Ctrl+Z 不得触发取消运行');
handleKey({
  key: 'z',
  ctrlKey: true,
  altKey: false,
  shiftKey: false,
  target: { tagName: 'BODY' },
  preventDefault() {
    prevented++;
  },
});
assert.equal(prevented, 1, '输入框外 Ctrl+Z 保持取消运行语义（preventDefault）');
assert.equal(connBanner.textContent, '当前无运行中', '闲置期按下必须有反馈（不再静默）');
console.log('ok  0dg ② Ctrl+Z 输入框豁免＋闲置期反馈');

/* ── 13. 0di（2026-10-18）：中途补充排队＋撤回（编辑式重发） ── */

// ①supplement_delivered 投递卡——运行中排队的用户输入在 loop-top 生效后
//   经 journal 事件渲染为用户卡（投递即可见确认；turn 计数递增）。
state.content = { items: [], currentModelIndex: null, droppedItems: 0 };
state.marker = [];
state.turnCounter = 0;
const turnBefore = state.turnCounter;
applyJournalEvent({
  event_type: 'supplement_delivered',
  payload: { supplement_seq: 1, character_count: 9, text: '补充：改用中文输出' },
});
const suppCard = state.content.items.at(-1);
assert.ok(suppCard, 'supplement_delivered 必须产出内容项');
assert.equal(suppCard.role, '用户', '补充投递卡＝用户卡（与 prompt_submitted 同面）');
assert.equal(suppCard.content, '补充：改用中文输出');
assert.equal(state.turnCounter, turnBefore + 1, '投递卡计一 turn');
assert.equal(state.deliveredSupplements, 1, '本 run 已投递补充计数必须递增');
applyJournalEvent({ event_type: 'run_cancelled', payload: { reason: 'user_cancelled' } });
assert.equal(state.deliveredSupplements, 0, '非成功终态必须清零已投递计数');
assert.ok(
  state.content.items.some((item) => item.role === '系统' && item.content.includes('1 条补充未写回会话')),
  '失败/取消必须明确提示已投递补充未写回',
);
console.log('ok  0di ① supplement_delivered 投递卡（用户面渲染）');

// ②ext 线格式与接线——acp.js 三个 `_orz/*` 方法字面（`_` 前缀＝ACP 扩展
//   方法线格式，acp 库剥前缀路由 ExtRequest）；main.js 运行中排队臂调
//   sessionQueueAppend（flash 拒绝臂退役）；/rewind 命令与对话框接线。
const acpSrc = readFileSync(new URL('../assets/app/acp.js', import.meta.url), 'utf8');
assert.ok(acpSrc.includes("'_orz/queue_append'"), 'acp.js 必须含 queue_append ext 线格式');
assert.ok(acpSrc.includes("'_orz/recall_preview'"), 'acp.js 必须含 recall_preview ext 线格式');
assert.ok(acpSrc.includes("'_orz/recall'"), 'acp.js 必须含 recall ext 线格式');
const mainSrc = readFileSync(new URL('../assets/app/main.js', import.meta.url), 'utf8');
assert.ok(mainSrc.includes('sessionQueueAppend'), '运行中排队臂必须调 sessionQueueAppend');
const submitHead = mainSrc.slice(mainSrc.indexOf('async function submitPrompt'), mainSrc.indexOf('async function submitPrompt') + 600);
assert.ok(!submitHead.includes('当前有运行在进行'), 'submitPrompt 运行中 flash 拒绝臂必须退役（排队臂取代）');
assert.ok(mainSrc.includes("case '/rewind':"), '/rewind 命令必须接线');
assert.ok(mainSrc.includes("flash('当前有运行在进行，结束或取消后才能撤回')"), '/rewind 必须有运行态前端守卫');
const dialogsSrc = readFileSync(new URL('../assets/app/dialogs.js', import.meta.url), 'utf8');
assert.ok(dialogsSrc.includes('export function openRewind()'), 'dialogs 必须含 openRewind');
assert.ok(dialogsSrc.includes('rewindRecall'), '撤回确认必须走 rewindRecall 动作');
assert.ok(dialogsSrc.includes('fillInput(r.recall_text'), '撤回后原文必须回输入框');
assert.ok(dialogsSrc.includes('Array.from(entry.preview).length'), '省略号判定必须按 Unicode 字符而非 UTF-16 码元');
console.log('ok  0di ② ext 线格式与排队/撤回接线字面');

// ③撤回对话框行为——mock 动作面（bindDialogActions 重绑）：列表渲染、
//   确认调 rewindRecall、原文回输入框、回执 flash。
let recallOrdinal = null;
let filledText = null;
let flashed = null;
dialogs.bindDialogActions({
  rewindPreview: async () => ({
    messages: [
      { ordinal: 1, preview: '第一个问题', character_count: 5 },
      { ordinal: 2, preview: '第二个问题', character_count: 5 },
    ],
    queue_depth: 3,
  }),
  rewindRecall: async (ordinal) => {
    recallOrdinal = ordinal;
    return { recalled: true, recall_text: '第二个问题', removed_messages: 3, dropped_supplements: 1 };
  },
  fillInput: (text) => {
    filledText = text;
  },
  flash: (m) => {
    flashed = m;
  },
});
dialogs.openRewind();
for (let i = 0; i < 5; i++) await Promise.resolve();
const modalRoot = globalThis.document.getElementById('modalLayer');
assert.equal(modalRoot.classList.contains('open'), true, 'openRewind 必须打开弹窗');
const queueNotice = registry
  .filter((e) => e.tagName === 'DIV' && (e.textContent || '').includes('未投递补充'))
  .at(-1);
assert.ok(
  queueNotice && queueNotice.textContent.includes('3 条未投递补充'),
  '撤回预览必须显示 queue_depth（将一并丢弃的补充条数）',
);
// stub DOM 的 textContent 不聚合子节点——行断言走 registry 的行元素。
const rewindRows = registry.filter((e) => e.className === 'rewind-row');
assert.equal(rewindRows.length, 2, '列表必须渲染两行');
assert.ok(rewindRows.some((r) => r.textContent.includes('#1')), '列表须含 #1 行');
assert.ok(rewindRows.some((r) => r.textContent.includes('#2')), '列表须含 #2 行');
const rewindBtn = registry.filter((e) => e.tagName === 'BUTTON' && e.textContent === '撤回').at(-1);
assert.ok(rewindBtn, '必须存在撤回确认按钮');
rewindBtn.click();
for (let i = 0; i < 8; i++) await Promise.resolve();
assert.equal(recallOrdinal, 2, '确认必须以选中行的 ordinal 调 rewindRecall');
assert.equal(filledText, '第二个问题', '原文必须回输入框');
assert.ok(flashed && flashed.includes('已撤回 3 条'), '撤回回执必须有 flash 提示');
assert.ok(flashed.includes('1 条未投递补充'), '丢弃的未投递补充必须回显');
console.log('ok  0di ③ 撤回对话框行为（列表/确认/原文回填/回执）');

/* ── 14. 318 P1 Web/TUI 行为化门（2026-10-19）：只读回看、显式恢复、
 *      load 代次、运行态单一 owner、RUN-CLI 实时尾、长行、删会话解绑、
 *      刷新/关闭风险提示。 ── */

const flush = async (n = 8) => {
  for (let i = 0; i < n; i++) await new Promise((r) => setTimeout(r, 0));
};
const jsonResponse = (obj) => ({ ok: true, json: async () => obj });
async function withFetch(handler, fn) {
  const old = globalThis.fetch;
  globalThis.fetch = handler;
  try {
    return await fn();
  } finally {
    globalThis.fetch = old;
  }
}
const fallbackApi = (extra = {}) => async (url) => {
  const u = String(url);
  for (const [key, value] of Object.entries(extra)) {
    if (u.includes(key)) return jsonResponse(typeof value === 'function' ? value(u) : value);
  }
  if (u.includes('/api/run/')) return jsonResponse({ lines: [], next_offset: 0 });
  if (u.includes('/api/runs')) return jsonResponse({ runs: [] });
  if (u.includes('/api/conversations')) return jsonResponse({ conversations: [] });
  if (u.includes('/api/archives')) return jsonResponse({ archives: [] });
  return jsonResponse({});
};
const methodCalls = (method) => acpMock.calls.filter((c) => c.method === method);
const runStateLabel = () =>
  state.status.find((s) =>
    ['空闲', '预检', '运行中', '等待审批', '完成', '失败', '已取消', '无效', '恢复中'].includes(s.label),
  )?.label;
const lastJournalSocket = () =>
  socketInstances.filter((s) => s.url.includes('/ws/journal/')).at(-1);
const resetView = () => {
  state.content = { items: [], currentModelIndex: null, droppedItems: 0 };
  state.marker = [];
  state.turnCounter = 0;
  state.nav = { current: 'workspace://live', back: [], forward: [] };
  state.navEpoch++;
  state.resumeTarget = null;
  state.selected = null;
};
const inputEl = globalThis.document.getElementById('chatInput');
async function sendFromInput(text, n = 3) {
  inputEl.value = text;
  inputEl.dispatch('keydown', { key: 'Enter', shiftKey: false });
  await flush(n);
}

await flush();
const acpSocket = socketInstances.find((s) => s.url.includes('/ws/acp'));
assert.ok(acpSocket, 'ACP WebSocket 必须已连接（测试替身）');
// host（2026-10-19）要求 session/load 的 cwd 为绝对路径且会 canonicalize。
state.workspaceCwd = 'D:\\CLI\\orz';

/* 14.1 openConversation 默认只读：不自动 session/load、不切绑定。 */
resetView();
state.sessionId = null;
state.boundSession8 = null;
state.loadingSession = false;
state.conversations = [
  { session8: '6aab1234', session_id: '6aab1234-1111-2222-3333-444455556666' },
];
state.runs = [
  { run_id: 'RUN-6aab1234-1', status: 'completed' },
  { run_id: 'RUN-CLI-6aab1234', status: 'completed' },
  { run_id: 'ARC-6aab1234-1', status: 'completed' },
];
const loadBefore = methodCalls('session/load').length;
const newBefore = methodCalls('session/new').length;
await withFetch(fallbackApi(), async () => {
  await ui.openUri('conversation://6aab1234');
});
assert.equal(methodCalls('session/load').length, loadBefore, '打开旧会话不得自动 session/load');
assert.equal(methodCalls('session/new').length, newBefore, '打开旧会话不得静默创建绑定会话');
assert.equal(state.sessionId, null, '浏览动作不得改变活跃绑定');
assert.equal(state.boundSession8, null, '绑定 s8 保持空');
assert.equal(state.resumeTarget?.s8, '6aab1234', '只读视图必须留下显式恢复目标');
assert.ok(
  state.content.items.some((it) => it.role === '系统' && it.content.includes('只读回看')),
  '文案必须如实说只读回看（不再谎称已可续接）',
);
widgets.renderToolbar();
const resumeBtn = ui.toolbar.children.find((n) => n.textContent === '恢复并继续');
assert.ok(resumeBtn && !resumeBtn.disabled, '只读会话必须提供可点的「恢复并继续」');
console.log('ok  14.1 openConversation 只读回看：零 load/零绑定漂移＋显式恢复入口');

/* 14.2 显式「恢复并继续」：唯一 load 入口，成功后绑定并回实时。 */
acpMock.loadHandler = (msg, ws) => wsRespond(ws, msg.id, {});
await withFetch(fallbackApi(), async () => {
  await ui.resumeConversation();
  await flush();
});
const loadCall = methodCalls('session/load').at(-1);
assert.equal(loadCall?.params?.sessionId, '6aab1234-1111-2222-3333-444455556666', '恢复必须显式传完整 session id');
assert.ok(
  /^(?:[A-Za-z]:[\\/]|\\\\|\/)/.test(loadCall?.params?.cwd || ''),
  'session/load 必须传绝对 cwd（host 会 canonicalize）',
);
assert.equal(state.sessionId, '6aab1234-1111-2222-3333-444455556666', '恢复成功才切换绑定');
assert.equal(state.boundSession8, '6aab1234');
assert.equal(state.nav.current, 'workspace://live', '恢复成功后回到实时视图');
assert.ok(
  state.content.items.some((it) => it.role === '系统' && it.content.includes('下一条输入将续接')),
  '恢复成功文案与状态一致',
);
console.log('ok  14.2 显式「恢复并继续」唯一执行 session/load 并切换绑定');

/* 14.2s 真切换：新目标 load 成功后关闭旧 host 附着；顺序必须是 load→bind
 * →close(switch)，从而 load 失败时保留旧绑定，旧 closed 通知也不会解绑新目标。 */
resetView();
const switchFrom = state.sessionId;
const switchTo = 'deadbeef-aaaa-bbbb-cccc-ddddeeeeffff';
state.conversations = [{ session8: 'deadbeef', session_id: switchTo, closed: true }];
state.runs = [{ run_id: 'RUN-deadbeef-1', status: 'completed' }];
await withFetch(fallbackApi(), async () => {
  await ui.openUri('conversation://deadbeef');
});
acpMock.loadHandler = (msg, ws) => wsRespond(ws, msg.id, {});
const switchCallStart = acpMock.calls.length;
await withFetch(fallbackApi(), async () => {
  await ui.resumeConversation();
  await flush();
});
const switchCalls = acpMock.calls.slice(switchCallStart);
const switchLoadIndex = switchCalls.findIndex((call) => call.method === 'session/load');
const switchCloseIndex = switchCalls.findIndex((call) => call.method === '_orz/session_close');
assert.ok(switchLoadIndex >= 0 && switchCloseIndex > switchLoadIndex, '切换必须先 load 新目标再 close 旧目标');
assert.equal(switchCalls[switchCloseIndex]?.params?.sessionId, switchFrom, 'switch close 必须命中旧绑定');
assert.equal(switchCalls[switchCloseIndex]?.params?.reason, 'switch');
assert.equal(state.sessionId, switchTo, '旧会话收尾不得解绑新目标');
// 反例：新目标 load 失败时不得关闭／丢失现绑定。
state.conversations = [
  { session8: 'bad0cafe', session_id: 'bad0cafe-1111-2222-3333-444455556666', closed: true },
];
state.runs = [{ run_id: 'RUN-bad0cafe-1', status: 'completed' }];
await withFetch(fallbackApi(), async () => {
  await ui.openUri('conversation://bad0cafe');
});
acpMock.loadHandler = (msg, ws) => wsRespondError(ws, msg.id, 'load refused');
const failedCloseBefore = methodCalls('_orz/session_close').length;
await withFetch(fallbackApi(), async () => {
  await ui.resumeConversation();
  await flush();
});
assert.equal(state.sessionId, switchTo, '新目标 load 失败必须保留旧绑定');
assert.equal(
  methodCalls('_orz/session_close').length,
  failedCloseBefore,
  '新目标 load 失败不得关闭旧绑定',
);
console.log('ok  14.2s 恢复即真切换：成功 load 后 close(switch)，失败保留旧绑定');

/* 14.2a 归档视图直接恢复：不再要求先手动「回档」——归档详情带 session_id
 * 时工具栏即可恢复，host 侧 load 自动回档。 */
resetView();
state.sessionId = null;
state.boundSession8 = null;
state.running = false;
state.loadingSession = false;
state.workspaceCwd = 'D:\\CLI\\orz';
const archiveDoc = {
  session8: 'cafebabe',
  session_id: 'cafebabe-1111-2222-3333-444455556666',
  archived_at: '2026-10-21T09:00:00Z',
  message_count: 1,
  messages: [{ role: 'user', content: '归档里的问题' }],
  archive_keys: {},
};
acpMock.loadHandler = (msg, ws) =>
  wsRespond(ws, msg.id, { session_id: archiveDoc.session_id, status: 'loaded' });
await withFetch(fallbackApi({ '/api/archives/cafebabe': archiveDoc }), async () => {
  await ui.openUri('archive://cafebabe');
  assert.equal(state.resumeTarget?.s8, 'cafebabe', '归档视图必须留下恢复目标');
  assert.equal(
    state.toolbar.find((b) => b.id === 'resume')?.enabled,
    true,
    '归档视图的「恢复并继续」必须可点（不再要求先回档）',
  );
  const before = methodCalls('session/load').length;
  await ui.resumeConversation();
  await flush();
  assert.equal(methodCalls('session/load').length, before + 1, '归档恢复必须走唯一 load 入口');
});
assert.equal(state.sessionId, archiveDoc.session_id, '归档恢复成功后切换绑定');
assert.equal(state.nav.current, 'workspace://live', '归档恢复成功后回到实时视图');
console.log('ok  14.2a 归档视图直接「恢复并继续」（host 自动回档）');

/* 14.2b load 历史回放去重：host 在 load 返回前经 session/update 回放
 * 整段历史；恢复成功后必须先清空，再进入实时 journal 面——同一段历史
 * 不得渲染两遍。 */
resetView();
state.conversations = [
  { session8: 'feedface', session_id: 'feedface-1111-2222-3333-444455556666' },
];
state.runs = [{ run_id: 'RUN-feedface-1', status: 'completed' }];
await withFetch(fallbackApi(), async () => {
  await ui.openUri('conversation://feedface');
});
acpMock.loadHandler = (msg, ws) => {
  // ACP v1：响应前经 session/update 回放历史（host 2026-10-19 实现）。
  ws.onmessage({
    data: JSON.stringify({
      jsonrpc: '2.0',
      method: 'session/update',
      params: {
        sessionId: 'feedface-1111-2222-3333-444455556666',
        update: {
          sessionUpdate: 'agent_message_chunk',
          content: { type: 'text', text: 'LOAD历史回放' },
        },
      },
    }),
  });
  wsRespond(ws, msg.id, {});
};
await withFetch(fallbackApi(), async () => {
  await ui.resumeConversation();
  await flush();
});
assert.equal(
  state.content.items.filter((it) => String(it.content || '').includes('LOAD历史回放')).length,
  0,
  'load 回放必须先清空，不得与实时/后续回放重复渲染',
);
assert.equal(state.sessionId, 'feedface-1111-2222-3333-444455556666');
console.log('ok  14.2b load 历史回放不与实时面重复渲染（成功后清空回放面）');

/* 14.3 load 代次：加载期禁 prompt/禁切视图；取消后结果过期不写绑定；
 *       await 后运行态变化同样拒绝覆盖。 */
resetView();
state.conversations = [
  { session8: '6aab1234', session_id: '6aab1234-1111-2222-3333-444455556666' },
  { session8: 'deadbeef', session_id: 'deadbeef-aaaa-bbbb-cccc-ddddeeeeffff' },
];
state.runs = [{ run_id: 'RUN-deadbeef-1', status: 'completed' }];
await withFetch(fallbackApi(), async () => {
  await ui.openUri('conversation://deadbeef');
});
let pendingLoad = null;
acpMock.loadHandler = (msg, ws) => {
  pendingLoad = { msg, ws };
};
const promptBefore = methodCalls('session/prompt').length;
const loadPromise = ui.resumeConversation();
await flush();
assert.equal(state.loadingSession, true, 'load 等待期必须置 loading 门');
const chatInput0 = globalThis.document.getElementById('chatInput');
chatInput0.value = '加载期间不得发送';
chatInput0.dispatch('keydown', { key: 'Enter', shiftKey: false });
await flush(3);
assert.equal(methodCalls('session/prompt').length, promptBefore, 'load 期间禁发 prompt');
assert.equal(chatInput0.value, '加载期间不得发送', 'load 期间拒绝输入必须保留草稿');
assert.ok(connBanner.textContent.includes('恢复中'), 'load 期间发输入必须如实拒绝');
await ui.openUri('workspace://live');
assert.equal(state.nav.current, 'conversation://deadbeef', 'load 期间禁切视图/回 live');
// 取消本次恢复：代次自增，随后到达的 load 结果不得写绑定。
await ui.resumeConversation();
await flush();
assert.equal(state.loadingSession, false, '取消后 loading 门解除');
wsRespond(pendingLoad.ws, pendingLoad.msg.id, {});
await loadPromise;
await flush();
assert.equal(state.sessionId, 'feedface-1111-2222-3333-444455556666', '被取消的 load 不得覆盖绑定');
assert.ok(
  state.content.items.some((it) => it.role === '系统' && /取消|过期/.test(it.content)),
  '取消/过期必须留下可见说明',
);
// 第二回合：await 返回时 state.running 已变化 → 同样不切换绑定。
resetView();
state.conversations = [
  { session8: 'deadbeef', session_id: 'deadbeef-aaaa-bbbb-cccc-ddddeeeeffff' },
];
state.runs = [{ run_id: 'RUN-deadbeef-1', status: 'completed' }];
await withFetch(fallbackApi(), async () => {
  await ui.openUri('conversation://deadbeef');
});
pendingLoad = null;
acpMock.loadHandler = (msg, ws) => {
  pendingLoad = { msg, ws };
};
const loadPromise2 = ui.resumeConversation();
await flush();
vm.setRunning(true); // 模拟等待期间运行态变化
wsRespond(pendingLoad.ws, pendingLoad.msg.id, {});
await loadPromise2;
await flush();
assert.equal(state.sessionId, 'feedface-1111-2222-3333-444455556666', 'await 后 running 变化必须拒绝切换绑定');
vm.setRunning(false);
console.log('ok  14.3 load 代次：禁发/禁切＋取消过期＋await 后运行态复检');

/* 14.4 只读 rewind：跨会话/归档视图拒绝；拒绝时保留草稿；不调
 *      ensureSession；放行时显式传绑定 session_id。 */
resetView();
state.nav = { current: 'conversation://cafebabe', back: [], forward: [] };
state.navEpoch++;
const previewBefore = methodCalls('_orz/recall_preview').length;
const chatInput = globalThis.document.getElementById('chatInput');
chatInput.value = '/rewind';
chatInput.dispatch('keydown', { key: 'Enter', shiftKey: false });
assert.equal(chatInput.value, '/rewind', '只读 rewind 拒绝时必须保留草稿（不得先清空）');
assert.equal(methodCalls('_orz/recall_preview').length, previewBefore, '只读 rewind 不得发预览请求');
assert.ok(connBanner.textContent.includes('只读'), '只读拒绝必须有可见原因');
await assert.rejects(() => ui.rewindPreview(), /只读|恢复并继续/, '只读视图 rewindPreview 必须抛错拒绝');
// 归档视图同样拒绝。
state.nav = { current: 'archive://cafebabe', back: [], forward: [] };
state.navEpoch++;
chatInput.value = '/rewind';
chatInput.dispatch('keydown', { key: 'Enter', shiftKey: false });
assert.equal(chatInput.value, '/rewind', '归档视图 rewind 拒绝时必须保留草稿');
// 放行路径：绑定会话 + 实时视图，显式 session_id（不调用 ensureSession）。
resetView();
acpMock.recallPreviewHandler = (msg, ws) =>
  wsRespond(ws, msg.id, { messages: [{ ordinal: 1, preview: '问题', character_count: 2 }] });
const recallPreview = await ui.rewindPreview();
assert.equal(recallPreview.messages.length, 1);
const previewCall = methodCalls('_orz/recall_preview').at(-1);
assert.equal(previewCall?.params?.session_id, 'feedface-1111-2222-3333-444455556666', '预览必须显式传绑定 session_id');
acpMock.recallHandler = (msg, ws) => wsRespond(ws, msg.id, { recalled: true, recall_text: '问题' });
await ui.rewindRecall(1);
assert.equal(methodCalls('_orz/recall').at(-1)?.params?.session_id, 'feedface-1111-2222-3333-444455556666', '撤回必须命中同一显式会话');
chatInput.value = '';
console.log('ok  14.4 只读/归档 rewind 拒绝保留草稿＋显式 session_id（零静默绑定）');

/* 14.5 运行态单一 owner：journal 不写状态栏；status 与 running 同源；
 *      Ctrl+Z 与停止按钮取同一 state.running，取消正确会话。 */
vm.setRunning(false);
state.serverBusy = false;
applyJournalEvent({ event_type: 'run_started', payload: {} });
applyJournalEvent({ event_type: 'model_output', payload: { text: '历史输出' } });
assert.equal(state.running, false, 'journal 回放不得改写 state.running');
assert.equal(runStateLabel(), '空闲', 'journal 回放不得把状态栏写成运行中');
applyJournalEvent({ event_type: 'run_finished', payload: { status: 'completed' } });
assert.equal(runStateLabel(), '空闲', '回放终态不得把状态栏写成完成（单一 owner）');
// 真实 prompt 生命周期（当前绑定＝14.2b 恢复的 feedface 会话）。
resetView();
let pendingPrompt = null;
acpMock.promptHandler = (msg, ws) => {
  pendingPrompt = { msg, ws };
};
const promptPromise = sendFromInput('运行态测试');
await flush(3);
assert.equal(state.running, true, 'prompt 发出后 running 必须为真');
assert.equal(runStateLabel(), '运行中', '状态栏必须与 running 同源');
widgets.renderToolbar();
const stopBtn = ui.toolbar.children.find((n) => n.textContent === '停止');
assert.ok(stopBtn && stopBtn.disabled === false, '运行时停止按钮不得灰');
let cancelled = null;
const realSend = acpSocket.send.bind(acpSocket);
acpSocket.send = (data) => {
  const msg = JSON.parse(data);
  if (msg.method === 'session/cancel') cancelled = msg.params;
  realSend(data);
};
handleKey({
  key: 'z',
  ctrlKey: true,
  altKey: false,
  shiftKey: false,
  target: { tagName: 'BODY' },
  preventDefault() {},
});
assert.equal(cancelled?.sessionId, 'feedface-1111-2222-3333-444455556666', 'Ctrl+Z 必须取消本 run 的会话');
acpSocket.send = realSend;
wsRespond(pendingPrompt.ws, pendingPrompt.msg.id, { stopReason: 'end_turn' });
await promptPromise;
await flush();
assert.equal(state.running, false, 'prompt 返回后 running 复位');
assert.equal(runStateLabel(), '空闲', '状态栏随 running 复位');
// 服务端 busy：错误→清单重同步为运行中；终态清单到达→复位。
acpMock.promptHandler = (msg, ws) =>
  wsRespondError(ws, msg.id, 'prompt rejected: a run or restore is already in flight for session x');
await withFetch(
  fallbackApi({ '/api/runs': { runs: [{ run_id: 'RUN-feedface-9', status: 'running' }] } }),
  async () => {
    await sendFromInput('busy 探针');
    await flush();
  },
);
assert.equal(state.running, true, '服务端 busy 必须重同步为运行中');
assert.equal(runStateLabel(), '运行中', 'busy 时状态栏不得留在空闲');
assert.equal(state.serverBusy, true);
await withFetch(
  fallbackApi({ '/api/runs': { runs: [{ run_id: 'RUN-feedface-9', status: 'completed' }] } }),
  async () => {
    await ui.refresh();
    await flush();
  },
);
assert.equal(state.serverBusy, false, '终态清单到达后外部 busy 标记复位');
assert.equal(state.running, false);
acpMock.promptHandler = null;
console.log('ok  14.5 运行态单一 owner＋Ctrl+Z/停止同源＋服务端 busy 重同步');

/* 14.6 RUN-CLI 实时尾：精确匹配 RUN-CLI-<s8>，排除 ARC/RST；实时窗不空白。 */
resetView();
await withFetch(
  fallbackApi({
    '/api/runs': {
      runs: [
        { run_id: 'ARC-feedface-9', status: 'completed' },
        { run_id: 'RST-feedface-8', status: 'completed' },
        { run_id: 'RUN-CLI-feedface', status: 'completed' },
        { run_id: 'RUN-6aab9999-1', status: 'completed' },
      ],
    },
  }),
  async () => {
    await ui.openUri('workspace://live');
    await flush();
  },
);
const journalWs = lastJournalSocket();
assert.ok(journalWs, '必须建立 journal 尾连接（CLI 侧车不得空白）');
assert.ok(
  journalWs.url.includes('/ws/journal/RUN-CLI-feedface?'),
  `实时尾必须精确选 RUN-CLI-feedface（实际 ${journalWs.url}）`,
);
assert.ok(!journalWs.url.includes('ARC-') && !journalWs.url.includes('RST-'), '维护 journal 不得被实时尾选中');
journalWs.onmessage({
  data: JSON.stringify({ event_type: 'prompt_submitted', payload: { prompt: 'CLI 实时事件' } }),
});
await flush(2);
assert.ok(
  state.content.items.some((it) => it.kind === 'message' && it.content === 'CLI 实时事件'),
  'CLI 侧车实时窗必须能渲染事件（不得空白）',
);
console.log('ok  14.6 RUN-CLI 实时尾行为（精确匹配＋ARC/RST 排除＋实时窗非空）');

/* 14.7 长行（>64 KiB / >256 KiB）事件经尾通道到投影不截断。 */
const journal = await import('../assets/app/journal.js');
let longEvent = null;
journal.tailRun('RUN-feedface-77', 0, (ev) => {
  longEvent = ev;
});
const longWs = lastJournalSocket();
const longText = '长'.repeat(100_000); // JSON 行 >256 KiB
longWs.onmessage({
  data: JSON.stringify({
    event_type: 'supplement_delivered',
    payload: { supplement_seq: 1, character_count: 100_000, text: longText },
  }),
});
assert.equal(longEvent?.event_type, 'supplement_delivered', '>64 KiB 单行必须完整到达前端');
assert.equal(longEvent.payload.text.length, 100_000, '长行文本不得被截断/丢弃');
state.deliveredSupplements = 0;
applyJournalEvent(longEvent);
assert.equal(state.deliveredSupplements, 1, '长行 supplement 必须计入已投递');
assert.equal(
  state.content.items.filter((it) => it.kind === 'message').at(-1)?.content.length,
  100_000,
  '补充卡必须渲染完整长文本',
);
journal.closeTail();
console.log('ok  14.7 >256 KiB 单行经实时尾/投影完整渲染（不丢/不截断）');

/* 14.8 删除已加载会话清空绑定；newSession 解绑旧会话并停 tail。 */
resetView();
state.conversations = [];
globalThis.confirm = () => true;
await withFetch(fallbackApi({ '/api/sessions/': { message: 'deleted' } }), async () => {
  await ui.deleteSession('feedface');
  await flush();
});
assert.equal(state.sessionId, null, '删除已加载会话后必须清空绑定');
assert.equal(state.boundSession8, null, '绑定 s8 必须清空（防下一条 prompt 写回 sidecar）');
assert.equal(state.nav.current, 'workspace://live');
// 下一条 prompt 必须新建会话，而不是复用已删除的旧 id。
acpMock.newHandler = (msg, ws) =>
  wsRespond(ws, msg.id, { sessionId: 'beefcafe-1111-2222-3333-444455556666' });
const promptsBefore = methodCalls('session/prompt').length;
await withFetch(fallbackApi(), async () => {
  await sendFromInput('删除后的问题');
  await flush();
});
assert.equal(methodCalls('session/prompt').length, promptsBefore + 1, '删除后仍应能新建会话发送');
assert.equal(methodCalls('session/prompt').at(-1)?.params?.sessionId, 'beefcafe-1111-2222-3333-444455556666', '下一条 prompt 不得复用已删除会话');
assert.equal(state.sessionId, 'beefcafe-1111-2222-3333-444455556666');
// newSession：0dk S2 起换会话＝`_orz/session_close(switch)`，再新建。
const oldSession = state.sessionId;
acpMock.newHandler = (msg, ws) =>
  wsRespond(ws, msg.id, { sessionId: 'cafef00d-1111-2222-3333-444455556666' });
const newCallsBefore = methodCalls('session/new').length;
const closeCallsBeforeNew = methodCalls('_orz/session_close').length;
await ui.newSession();
await flush();
assert.equal(methodCalls('session/new').length, newCallsBefore + 1, 'newSession 必须显式新建会话');
assert.equal(
  methodCalls('_orz/session_close').length,
  closeCallsBeforeNew + 1,
  '换会话必须先关闭旧会话（close/switch）',
);
assert.equal(methodCalls('_orz/session_close').at(-1)?.params?.sessionId, oldSession, '关闭目标＝旧绑定');
assert.equal(methodCalls('_orz/session_close').at(-1)?.params?.reason, 'switch');
assert.equal(state.sessionId, 'cafef00d-1111-2222-3333-444455556666', 'newSession 切换绑定');
assert.notEqual(state.sessionId, oldSession);

// 0dk S2：工具栏「关闭会话」显式发 `_orz/session_close(user_close)` 并解绑。
const boundForClose = state.sessionId;
const closeCallsBefore = methodCalls('_orz/session_close').length;
await ui.closeSession();
await flush();
const closeCall = methodCalls('_orz/session_close').at(-1);
assert.equal(closeCall?.params?.sessionId, boundForClose, '关闭必须命中当前绑定会话');
assert.equal(closeCall?.params?.reason, 'user_close', '工具栏关闭＝user_close（非 switch）');
assert.equal(methodCalls('_orz/session_close').length, closeCallsBefore + 1);
assert.equal(state.sessionId, null, '关闭成功后必须清空绑定');
assert.equal(state.boundSession8, null);
assert.equal(state.nav.current, 'workspace://live', '关闭后回到实时空视图');
console.log('ok  0dk S2 关闭会话：入口/运行中禁用＋close(user_close/switch)＋解绑');
// 15.x 前提：重新建立一个活跃绑定（上一步的关闭入口已按语义解绑）。
acpMock.newHandler = (msg, ws) =>
  wsRespond(ws, msg.id, { sessionId: 'cafef00d-1111-2222-3333-444455556666' });
await withFetch(fallbackApi(), async () => {
  await sendFromInput('关闭后再开');
  await flush();
});
assert.equal(state.sessionId, 'cafef00d-1111-2222-3333-444455556666');
console.log('ok  14.8 删除已加载会话清空绑定＋newSession 换会话 close(switch)＋关闭会话入口');

/* 14.9 刷页/关闭风险提示（329 口径：子进程保活，刷新可重连）：running/
 *      排队/后台任务都必须拦，文案说明保活窗口与排队补充/后台任务不恢复。 */
const unloadHandlers = windowListeners.beforeunload ?? [];
assert.ok(unloadHandlers.length, 'beforeunload 必须已注册');
vm.setRunning(true);
state.supplementQueueDepth = 2;
state.backgroundTasks = 1;
let unloadPrevented = false;
const unloadEvent = {
  returnValue: '',
  preventDefault() {
    unloadPrevented = true;
  },
};
unloadHandlers.at(-1)(unloadEvent);
assert.equal(unloadPrevented, true, '存在运行/队列/后台任务时必须拦刷新/关闭');
const warning = String(unloadEvent.returnValue);
assert.ok(warning.includes('保活'), '329：必须说明子进程保活/刷新可重连');
assert.ok(warning.includes('后台任务') && warning.includes('排队补充'), '必须点名后台任务/排队补充不恢复');
vm.setRunning(false);
state.supplementQueueDepth = 0;
state.backgroundTasks = 0;
console.log('ok  14.9 beforeunload 风险面＋「保活窗口/排队补充＋后台任务不恢复」文案');

/* ── 14.10 333a（2026-10-21）：Web 细节四项 ────────────────────────────── */

// ① 保活窗口外不恢复：attach 前置帧带 keepalive_expired 时必须显式上屏。
acpSocket.onmessage({
  data: JSON.stringify({
    jsonrpc: '2.0',
    method: '_orz/bridge_attached',
    params: {
      session_id: null,
      reused: false,
      keepalive_expired: true,
      expired_session_id: 'deadbeef-1111-2222-3333-444455556666',
      expired_secs_ago: 42,
      cwd: 'D:\\CLI\\orz',
    },
  }),
});
await flush();
assert.ok(
  state.content.items.some(
    (it) => it.kind === 'message' && String(it.content).includes('保活窗口已过期'),
  ),
  '保活到期后必须显式提示「上一会话未恢复」',
);
assert.ok(
  state.content.items.some((it) => String(it.content).includes('deadbeef')),
  '提示必须带被回收的会话标识（证据）',
);
console.log('ok  14.10a 保活窗口过期：attach 前置帧触发显式「未恢复」提示');

// ② 队列剩余额度/不跨进程：入队回执必须带 depth/capacity/remaining 与作用域。
state.running = true;
await ui.runSelfCheck();
await flush();
assert.ok(
  state.content.items.some(
    (it) => String(it.content).includes('1/16') && String(it.content).includes('剩余 15'),
  ),
  '入队回执必须显示第 N/上限 与剩余额度',
);
assert.ok(
  state.content.items.some((it) => String(it.content).includes('仅本会话进程')),
  '入队回执必须说明队列不跨进程/刷新不保留',
);
state.running = false;

// ③ 审批并发可见：第二个请求顶掉第一个时，新弹窗必须提示旧请求已取消。
assert.ok(
  registry.some((e) => String(e.textContent).includes('上一条权限请求已被本次请求顶掉')),
  '并发审批顶掉旧请求必须在新弹窗里可见',
);

// ④ 上下文读数：工具栏入口＋弹窗读数非空（估算/最近压缩/存档）。
widgets.renderToolbar();
assert.ok(
  ui.toolbar.children.some((n) => n.textContent === '上下文'),
  '工具栏必须有「上下文」读数入口',
);
resetView();
state.sessionId = 'cafef00d-1111-2222-3333-444455556666';
state.boundSession8 = 'cafef00d';
state.sessionRoot = 'D:\\CLI\\orz';
state.navEpoch = 7;
await withFetch(
  fallbackApi({
    '/api/context': {
      session8: 'cafef00d',
      messages: 3,
      chars: 1200,
      estimated_tokens: 600,
      estimate_method: 'chars/2',
      watermarks: ['500K'],
      last_compaction: {
        run_id: 'RUN-cafef00d-2',
        trigger_tokens: 900000,
        estimated_tokens_after: 420000,
        mode: 'mechanical',
        reason: 'context_scale_window',
      },
      compaction_archives: 2,
      notes: ['只读投影'],
    },
  }),
  async () => {
    await ui.openContext();
    await flush();
  },
);
assert.ok(
  registry.some((e) => String(e.textContent).includes('600 tokens')),
  '上下文读数必须显示估算占用',
);
assert.ok(
  registry.some((e) => String(e.textContent).includes('RUN-cafef00d-2')),
  '上下文读数必须显示最近一次压缩的 run/前后读数',
);
assert.ok(
  registry.some((e) => String(e.textContent).includes('500K')),
  '上下文读数必须显示水位提醒',
);
console.log('ok  14.10b 333a：队列额度/审批顶掉/上下文读数（估算＋最近压缩）');

/* ── 14.11 333b（2026-10-21）：后台可见/交付空态/版本偏斜/错误四要素 ──── */

// ⑤ 后台任务：属性弹窗必须明确「仅模型可见」，队列深度带额度。
resetView();
state.supplementQueueDepth = 3;
state.queueCapacity = 16;
state.backgroundTasks = 2;
ui.openProperties();
assert.ok(
  registry.some((e) => String(e.textContent).includes('仅模型可见')),
  '属性面板必须说明后台任务仅模型可见（不虚构读不到的任务表）',
);
assert.ok(
  registry.some((e) => String(e.textContent).includes('3/16')),
  '属性面板必须显示队列深度/上限',
);

// ⑥ 交付面板：空态显式文案；有 terminal_source 时必须原样可见。
state.boundSession8 = 'cafef00d';
state.sessionId = 'cafef00d-1111-2222-3333-444455556666';
state.sessionRoot = 'D:\\CLI\\orz';
state.navEpoch += 1;
await withFetch(
  fallbackApi({
    '/api/delivery': {
      session8: 'cafef00d',
      session_id: state.sessionId,
      root: 'D:\\CLI\\orz',
      messages: null,
      plan: { steps: [] },
      last_run: null,
      recall: { count: 0 },
      notes: ['只读投影'],
    },
  }),
  async () => {
    await ui.openDeliveryStatus();
    await flush();
  },
);
assert.ok(
  registry.some((e) => String(e.textContent).includes('尚无可展示的交付事实')),
  '交付面板空态必须有显式文案',
);
dialogs.closeModal();
const deliveryWithRun = {
  session8: 'cafef00d',
  session_id: state.sessionId,
  root: 'D:\\CLI\\orz',
  messages: 2,
  plan: { steps: [], terminal_step: null, terminal_state: null },
  last_run: {
    run_id: 'RUN-cafef00d-2',
    status: 'completed',
    terminal_source: 'run_end',
    terminal_nudged: false,
    terminal_nudge_count: 0,
    self_check_called: true,
    self_check_tool_rounds: 1,
  },
  recall: { count: 0 },
  notes: [],
};
await withFetch(
  fallbackApi({ '/api/delivery': deliveryWithRun }),
  async () => {
    await ui.openDeliveryStatus();
    await flush();
  },
);
assert.ok(
  registry.some((e) => String(e.textContent).includes('run_end（模型 [RUN_END] 声明）')),
  '交付面板必须显示 terminal_source 原码与中文标注',
);
dialogs.closeModal();

// ⑧ 版本偏斜：boot 事实落地时横幅与系统行都必须显式。
ui.applyBootFacts({
  cwd: 'D:\\CLI\\orz',
  bridge_version: '0.9.8',
  child_version: '0.9.7',
  version_skew: true,
});
assert.ok(connBanner.textContent.includes('版本偏斜'), '版本偏斜必须上连接横幅');
assert.ok(
  connBanner.textContent.includes('0.9.8') && connBanner.textContent.includes('0.9.7'),
  '版本偏斜必须给出两侧版本号（证据）',
);
assert.ok(
  state.content.items.some((it) => String(it.content).includes('版本偏斜')),
  '版本偏斜必须同时进系统行',
);
connBanner.hidden = true;

// ⑨ 错误四要素：copy.js 合成「发生了什么＋分类＋下一步＋证据」。
const copy = await import('../assets/app/copy.js');
const four = copy.describeError({
  code: 'session_busy',
  class: 'session',
  hint: '等本轮结束',
  evidence: 'RUN-cafef00d-2',
});
assert.ok(four.includes('会话正忙'), '四要素缺「发生了什么」');
assert.ok(four.includes('session'), '四要素缺「分类」');
assert.ok(four.includes('等本轮结束'), '四要素缺「下一步」');
assert.ok(four.includes('RUN-cafef00d-2'), '四要素缺「证据」');
assert.ok(copy.describeError({ code: 'unknown_code' }).includes('internal'), '未知码必须 internal 兜底');
console.log('ok  14.11 333b：后台仅模型可见/交付空态＋terminal_source/版本偏斜/错误四要素');

/* ── 15. 329（2026-10-21）：/rewind 本地投影截断＋同文重发＋重放水位过滤 ──
 * 0.9.7 真机问题：撤回后标记栏显示/定位均错、旧消息无法再定位。服务端
 * `_orz/recall` 已正确截断侧车；本段钉子全在 Web 投影面。 */

const userTexts = () =>
  state.content.items
    .filter((it) => it.kind === 'message' && it.role === '用户')
    .map((it) => it.content);
const messageTexts = () =>
  state.content.items.filter((it) => it.kind === 'message').map((it) => it.content);
const markerInBounds = () =>
  state.marker.every((m) => m.itemIndex >= 0 && m.itemIndex < state.content.items.length);

assert.equal(
  state.sessionId,
  'cafef00d-1111-2222-3333-444455556666',
  '前提：15.x 需要 14.8 留下的活跃绑定（/rewind 只作用于绑定会话）',
);

/* 15.1 撤回成功：按 ordinal 数用户卡截断 items、marker、turnCounter。 */
resetView();
state.content = { items: [], currentModelIndex: null, droppedItems: 0 };
state.marker = [];
state.turnCounter = 0;
applyJournalEvent({ run_id: 'RUN-cafef00d-1', event_type: 'prompt_submitted', payload: { prompt: '问题一' } });
applyJournalEvent({ event_type: 'model_output', payload: { text: '回答一' } });
applyJournalEvent({ event_type: 'run_finished', payload: { status: 'completed' } });
applyJournalEvent({ run_id: 'RUN-cafef00d-2', event_type: 'prompt_submitted', payload: { prompt: '问题二' } });
applyJournalEvent({ event_type: 'model_output', payload: { text: '回答二' } });
applyJournalEvent({ event_type: 'run_finished', payload: { status: 'completed' } });
assert.deepEqual(userTexts(), ['问题一', '问题二'], '前提：本地视图有两张用户卡');
assert.equal(state.turnCounter, 3, '前提：第二 run 的 turn 为 3');
const itemsBeforeRecall = state.content.items.length;

acpMock.recallPreviewHandler = (msg, ws) => wsRespond(ws, msg.id, { messages: [] });
await ui.rewindPreview();
acpMock.recallHandler = (msg, ws) =>
  wsRespond(ws, msg.id, { recalled: true, recall_text: '问题二', removed_messages: 4, dropped_supplements: 0 });
const recallResult = await ui.rewindRecall(2);
assert.equal(recallResult?.recalled, true, '撤回必须把服务端结果原样返回（dialogs 回填依赖）');
assert.deepEqual(userTexts(), ['问题一'], '第 2 张用户卡及其后全部条目必须删除');
assert.ok(state.content.items.length < itemsBeforeRecall, 'items 必须缩短');
assert.equal(state.content.currentModelIndex, null, '截断后流式指针必须复位');
assert.equal(state.turnCounter, 2, 'turnCounter 必须取截断后最后一张用户卡的 turn');
assert.equal(markerInBounds(), true, '截断后 marker 下标不得越界');
assert.equal(state.marker.filter((m) => m.kind === 'user').length, 1, '只保留幸存用户卡的输入锚');
assert.equal(state.marker.filter((m) => m.kind === 'model').length, 1, '幸存运行的最终输出锚保留');
assert.equal(
  state.content.items[state.marker.find((m) => m.kind === 'model').itemIndex]?.content,
  '回答一',
  '最终输出锚必须指向幸存模型卡',
);
assert.ok(
  state.content.items.some((it) => it.role === '系统' && it.content.includes('已撤回 4 条消息')),
  '撤回回执仍须落在内容面（可回查）',
);

// 搜索命中锚：重建时有效下标保留、越界即丢（内容已不在视图）。
vm.addSearchHit('L0', 0);
state.marker.push({ kind: 'hit', label: '● 越界', itemIndex: state.content.items.length + 3 });
vm.rebuildMarkers();
assert.ok(
  state.marker.some((m) => m.kind === 'hit' && m.label === '● L0' && m.itemIndex === 0),
  '有效搜索命中锚重建后保留',
);
assert.equal(
  state.marker.some((m) => m.kind === 'hit' && m.label === '● 越界'),
  false,
  '越界搜索命中锚重建即丢',
);
vm.clearHits();
console.log('ok  15.1 撤回成功：items 截断＋marker 重建＋turnCounter 回退');

/* 15.2 撤回后同文重发不再被去重吞：被撤回的用户卡已删除，去重窗口只看
 * 最近一条用户消息（问题一），同文「问题二」必须重新入卡并产新输入锚。 */
vm.addUserMessage('问题二');
assert.deepEqual(userTexts(), ['问题一', '问题二'], '撤回后同文重发必须产生新用户卡');
const newUserAnchor = state.marker.filter((m) => m.kind === 'user').at(-1);
assert.match(newUserAnchor?.label || '', /T2 输入/, '重发必须产生新的输入锚');
assert.equal(
  state.content.items[newUserAnchor.itemIndex]?.content,
  '问题二',
  '新输入锚必须指向重发的卡',
);
console.log('ok  15.2 撤回后同文重发重新入卡并产新输入锚（去重窗口＝最近一条）');

/* 15.3 本地视图找不到第 ordinal 张用户卡：不瞎截、不抛错，如实提示并
 * 保留服务端结果（只读回放/本地已裁掉场景）。 */
const itemsBeforeMiss = state.content.items.length;
await ui.rewindPreview();
acpMock.recallHandler = (msg, ws) =>
  wsRespond(ws, msg.id, { recalled: true, recall_text: '', removed_messages: 1, dropped_supplements: 0 });
const missResult = await ui.rewindRecall(9);
assert.equal(missResult?.recalled, true, '找不到序号也必须保留服务端结果');
assert.deepEqual(userTexts(), ['问题一', '问题二'], '找不到第 ordinal 张用户卡时不得瞎截');
assert.equal(
  state.content.items.length,
  itemsBeforeMiss + 1,
  '未截断时只应新增一条说明系统行（不得删卡）',
);
assert.ok(
  messageTexts().some((t) => t.includes('本地视图不含第 9 条用户消息')),
  '必须如实提示「本地视图已不含该序号，未做本地截断」',
);
console.log('ok  15.3 本地缺该序号：不瞎截＋如实提示＋保留服务端结果');

/* 15.4 scrollToItem 越界：显式提示、不抛错、不读 undefined 崩溃。 */
connBanner.textContent = '';
assert.doesNotThrow(
  () => ui.scrollToItem(state.content.items.length + 5),
  '越界标记跳转不得抛错',
);
assert.ok(connBanner.textContent.includes('已不在视图中'), '越界必须显式提示（不静默）');
assert.doesNotThrow(() => ui.scrollToItem(0), '在界内的标记跳转保持可用');
console.log('ok  15.4 scrollToItem 越界显式提示（不静默/不崩溃）');

/* 15.4b 内存裁剪后的 ordinal 轴：被裁掉的用户卡计入序号，不得把撤回目标
 * 数到错误的本地卡（droppedUserCards=1 时，服务器第 2 张＝本地第 1 张）。 */
resetView();
state.content = { items: [], currentModelIndex: null, droppedItems: 1, droppedUserCards: 1 };
state.marker = [];
state.turnCounter = 0;
vm.addUserMessage('本地第二张');
vm.addUserMessage('本地第三张');
await ui.rewindPreview();
acpMock.recallHandler = (msg, ws) =>
  wsRespond(ws, msg.id, { recalled: true, recall_text: '本地第二张', removed_messages: 2, dropped_supplements: 0 });
await ui.rewindRecall(2);
assert.deepEqual(
  userTexts(),
  [],
  '服务器第 2 张用户卡＝本地第 1 张：必须把它及之后裁掉（不得数到本地第 2 张）',
);
assert.equal(state.turnCounter, 0, '本地已无用户卡时 turnCounter 归 0');
console.log('ok  15.4b 内存裁剪后 ordinal 轴仍对齐（droppedUserCards）');

/* 15.5 重放按 recall_log 过滤：第一条 run 被撤回裁掉尾部，第二条 run 在
 * 撤回之后 → 只保留前段＋第二条。 */
const TS_BASE = Date.parse('2026-10-21T00:00:00Z');
const at = (sec) => new Date(TS_BASE + sec * 1000).toISOString();
const journalEvent = (runId, sec, type, payload = {}) => ({
  run_id: runId,
  timestamp: at(sec),
  event_type: type,
  payload,
});
const runOneEvents = [
  journalEvent('RUN-bcc0ffee-1', 1, 'prompt_submitted', { prompt: '旧问题一' }),
  journalEvent('RUN-bcc0ffee-1', 2, 'model_output', { text: '旧回答一' }),
  journalEvent('RUN-bcc0ffee-1', 3, 'prompt_submitted', { prompt: '旧问题二' }),
  journalEvent('RUN-bcc0ffee-1', 4, 'model_output', { text: '旧回答二' }),
  journalEvent('RUN-bcc0ffee-1', 5, 'run_finished', { status: 'completed' }),
];
const runTwoEvents = [
  journalEvent('RUN-bcc0ffee-2', 7, 'prompt_submitted', { prompt: '新问题三' }),
  journalEvent('RUN-bcc0ffee-2', 8, 'model_output', { text: '新回答三' }),
  journalEvent('RUN-bcc0ffee-2', 9, 'run_finished', { status: 'completed' }),
];
const recallEventsApi = (eventsByRun, recallLog) => async (url) => {
  const u = String(url);
  if (u.includes('/api/recall')) return jsonResponse({ session8: 'bcc0ffee', recall_log: recallLog });
  if (u.includes('/api/run/')) {
    const m = /\/api\/run\/([^/?]+)\/events/.exec(u);
    const runId = m ? decodeURIComponent(m[1]) : '';
    return jsonResponse({ lines: eventsByRun[runId] || [], next_offset: 0 });
  }
  if (u.includes('/api/runs')) return jsonResponse({ runs: [] });
  if (u.includes('/api/conversations')) return jsonResponse({ conversations: [] });
  if (u.includes('/api/archives')) return jsonResponse({ archives: [] });
  return jsonResponse({});
};

resetView();
state.conversations = [
  { session8: 'bcc0ffee', session_id: 'bcc0ffee-1111-2222-3333-444455556666', root: 'D:\\CLI\\orz' },
];
state.runs = [
  { run_id: 'RUN-bcc0ffee-2', status: 'completed' },
  { run_id: 'RUN-bcc0ffee-1', status: 'completed' },
];
await withFetch(
  recallEventsApi(
    { 'RUN-bcc0ffee-1': runOneEvents, 'RUN-bcc0ffee-2': runTwoEvents },
    [{ ordinal: 2, recalled_at: TS_BASE / 1000 + 6, removed_messages: 3 }],
  ),
  async () => {
    await ui.openUri('conversation://bcc0ffee');
  },
);
assert.deepEqual(
  userTexts(),
  ['旧问题一', '新问题三'],
  '撤回水位：第 2 张用户卡及其后旧 tail 被裁，recall 之后的新 run 保留',
);
assert.ok(messageTexts().includes('旧回答一'), '撤回点之前的回答保留');
assert.equal(messageTexts().includes('旧回答二'), false, '被撤回的旧回答不得在重放中复活');
assert.equal(messageTexts().includes('旧问题二'), false, '被撤回的旧问题不得在重放中复活');
assert.ok(messageTexts().includes('新回答三'), 'recall 之后的 run 正常重放');
assert.equal(state.turnCounter, 3, '过滤重放后 turnCounter 取最后一个保留 run 的序号');
assert.equal(markerInBounds(), true, '重放后 marker 下标不得越界');
assert.equal(
  messageTexts().some((t) => t.includes('无终态')),
  false,
  '撤回裁掉终态不得误报「历史未完成」',
);
assert.equal(state.marker.filter((m) => m.kind === 'model').length, 1, '只保留 recall 之后 run 的最终输出锚');
assert.equal(
  state.content.items[state.marker.find((m) => m.kind === 'model').itemIndex]?.content,
  '新回答三',
  '重放后的最终输出锚必须指向保留的模型卡',
);
console.log('ok  15.5 合并重放按 recall_log 过滤（前段保留＋撤回后 run 保留）');

/* 15.6 多条 recall 级联：按 recalled_at 升序依次应用（乱序日志也必须按
 * 时间序）；只应用最后一条会把「旧回答一」一起裁掉，这里正是反例钉。 */
resetView();
state.runs = [
  { run_id: 'RUN-bcc0ffee-2', status: 'completed' },
  { run_id: 'RUN-bcc0ffee-1', status: 'completed' },
];
await withFetch(
  recallEventsApi(
    { 'RUN-bcc0ffee-1': runOneEvents, 'RUN-bcc0ffee-2': runTwoEvents },
    [
      { ordinal: 2, recalled_at: TS_BASE / 1000 + 10, removed_messages: 3 }, // 后发生（日志乱序）
      { ordinal: 2, recalled_at: TS_BASE / 1000 + 6, removed_messages: 3 }, // 先发生
    ],
  ),
  async () => {
    await ui.openUri('conversation://bcc0ffee');
  },
);
assert.deepEqual(userTexts(), ['旧问题一'], '两条 recall 级联后只剩第一段');
assert.ok(
  messageTexts().includes('旧回答一'),
  '第一条 recall 裁掉的是旧问题二之后；旧回答一必须存活（只取最后一条的反例）',
);
assert.equal(messageTexts().includes('旧回答二'), false, '第一条 recall 裁掉旧回答二');
assert.equal(messageTexts().includes('新问题三'), false, '第二条 recall 裁掉 recall 之后的新 run');
assert.equal(messageTexts().includes('新回答三'), false, '第二条 recall 裁掉新回答');
console.log('ok  15.6 多条 recall 按时间序级联（非只取最后一条）');

/* 15.6b 撤回后没有新事件：游标扫描不会触发，收尾仍须按序应用水位——
 * 否则「只撤回、未重发」的会话重开会把被撤回的旧 tail 带回来。 */
resetView();
state.runs = [{ run_id: 'RUN-bcc0ffee-1', status: 'completed' }];
await withFetch(
  recallEventsApi(
    { 'RUN-bcc0ffee-1': runOneEvents },
    [{ ordinal: 2, recalled_at: TS_BASE / 1000 + 10, removed_messages: 3 }],
  ),
  async () => {
    await ui.openUri('conversation://bcc0ffee');
  },
);
assert.deepEqual(userTexts(), ['旧问题一'], 'recall 之后无新事件时仍须应用水位（裁掉旧 tail）');
assert.ok(messageTexts().includes('旧回答一'), '撤回点之前的内容保留');
assert.equal(messageTexts().includes('旧回答二'), false, '无新事件也不得复活被撤回内容');
console.log('ok  15.6b 撤回后无新事件：收尾仍应用水位');

/* 15.7 水位端点不可用：catch 成「无水位」，打开会话不失败、按原全量重放。 */
resetView();
state.runs = [{ run_id: 'RUN-bcc0ffee-1', status: 'completed' }];
await withFetch(
  async (url) => {
    const u = String(url);
    if (u.includes('/api/recall')) {
      return { ok: false, status: 500, json: async () => ({ error: { message: 'recall endpoint unavailable' } }) };
    }
    if (u.includes('/api/run/')) return jsonResponse({ lines: runOneEvents, next_offset: 0 });
    if (u.includes('/api/runs')) return jsonResponse({ runs: [] });
    if (u.includes('/api/conversations')) return jsonResponse({ conversations: [] });
    if (u.includes('/api/archives')) return jsonResponse({ archives: [] });
    return jsonResponse({});
  },
  async () => {
    await ui.openUri('conversation://bcc0ffee');
  },
);
assert.deepEqual(
  userTexts(),
  ['旧问题一', '旧问题二'],
  '水位端点失败必须按无水位继续（不得让打开会话失败）',
);
assert.ok(messageTexts().includes('旧回答二'), '无水位时保持原全量重放（旧行为不变）');
console.log('ok  15.7 /api/recall 不可用：按无水位继续＋原全量重放');

/* ── 16. 334（2026-10-21）：绑定读数归零/原型键兜底/attach 运行态/
 *      session_root 绑定/撤回窗口缺口 ── */

// 16.1 切绑定/解绑必须清零队列深度、剩余额度、后台计数（334 §4.10）——
// 旧实现用 Math.max 且解绑不清，切会话后属性面板仍显示旧会话读数，
// beforeunload 也会为不存在的队列弹确认。
const mainModule = await import('../assets/app/main.js');
const hooks = mainModule.__testHooks;
assert.ok(hooks?.setBinding && hooks?.detachBinding, 'main.js 必须导出绑定测试钩子（冒烟可注入）');
resetView();
state.supplementQueueDepth = 5;
state.queueRemaining = 9;
state.queueScope = 'session_process';
state.backgroundTasks = 3;
hooks.setBinding('facefeed-1111-2222-3333-444455556666', 'D:\\CLI\\orz');
assert.equal(state.supplementQueueDepth, 0, '新绑定不得继承旧队列深度');
assert.equal(state.queueRemaining, null, '新绑定不得继承旧剩余额度');
assert.equal(state.backgroundTasks, 0, '新绑定不得继承旧后台计数');
assert.equal(
  memoryStorage.get('orz.sessionId'),
  'facefeed-1111-2222-3333-444455556666',
  '绑定必须写入 sessionStorage（刷新后 attach 续接）',
);
assert.equal(memoryStorage.get('orz.sessionRoot'), 'D:\\CLI\\orz', '归属根必须写入 sessionStorage');
state.supplementQueueDepth = 2;
state.queueRemaining = 1;
state.backgroundTasks = 4;
state.serverBusy = true;
hooks.detachBinding();
assert.equal(state.supplementQueueDepth, 0, '解绑必须清零队列深度');
assert.equal(state.queueRemaining, null, '解绑必须清零剩余额度');
assert.equal(state.backgroundTasks, 0, '解绑必须清零后台计数');
assert.equal(state.serverBusy, false, '解绑必须清服务端 busy 标记');
assert.equal(memoryStorage.has('orz.sessionId'), false, '解绑必须清除 sessionStorage 的会话键');
assert.equal(memoryStorage.has('orz.sessionRoot'), false, '解绑必须清除 sessionStorage 的根键');
console.log('ok  16.1 queue depth resets on binding change（setBinding/detachBinding 清零＋存储同步）');

// 16.2 copy.js 原型键兜底（334 §5.10）：code:"constructor" 不得命中原型上的
// Object 构造器（旧实现输出 `undefined（undefined）`）。
const copyMod = await import('../assets/app/copy.js');
const ctorFour = copyMod.describeError({ code: 'constructor' });
assert.equal(ctorFour.includes('undefined'), false, 'constructor 不得命中原型键渲染 undefined');
assert.ok(ctorFour.includes('内部错误'), 'constructor 必须回退 internal「内部错误」');
console.log('ok  16.2 copy.js constructor falls back to internal（原型键不命中）');

// 16.3 attach 顺序（334 §4.1）：prompt_in_flight=false 时必须清掉残留的
// 假运行态（此前 onBridgeAttached 先 setRunning(true)、resync 又首行早退）。
const currentAcpSocket = () =>
  socketInstances.filter((s) => s.url.includes('/ws/acp')).at(-1);
resetView();
state.serverBusy = false;
vm.setRunning(true);
assert.equal(state.running, true, '前提：先置一个残留运行态');
currentAcpSocket().onmessage({
  data: JSON.stringify({
    jsonrpc: '2.0',
    method: '_orz/bridge_attached',
    params: {
      session_id: 'deadc0de-1111-2222-3333-444455556666',
      reused: true,
      prompt_in_flight: false,
      session_root: 'D:\\CLI\\orz',
      cwd: 'D:\\CLI\\orz',
      pending_permission_count: 0,
    },
  }),
});
await flush();
assert.equal(state.running, false, 'prompt_in_flight=false 必须把假运行态清为空闲');
assert.equal(state.serverBusy, false, 'prompt_in_flight=false 必须清服务端 busy');
assert.equal(state.sessionId, 'deadc0de-1111-2222-3333-444455556666', 'attach 必须绑定服务端会话');
console.log('ok  16.3 bridge_attached with prompt_in_flight false clears running（先 resync 再置态）');

// 16.4 绑定根（334 §4.5）：preamble 的 session_root 优先于 child 启动 cwd。
const ATTACHED_ROOT = 'D:\\CLI\\orz-old-root';
currentAcpSocket().onmessage({
  data: JSON.stringify({
    jsonrpc: '2.0',
    method: '_orz/bridge_attached',
    params: {
      session_id: 'f00dbabe-1111-2222-3333-444455556666',
      reused: true,
      prompt_in_flight: false,
      session_root: ATTACHED_ROOT,
      cwd: 'D:\\CLI\\orz',
      pending_permission_count: 0,
    },
  }),
});
await flush();
assert.equal(state.sessionRoot, ATTACHED_ROOT, 'session_root 必须用于绑定（不得退回 child cwd）');
assert.equal(state.sessionId, 'f00dbabe-1111-2222-3333-444455556666');
console.log('ok  16.4 attached session_root is used for binding（归属根优先于 cwd）');

// 16.5 撤回窗口缺口（334 §4.11）：重放窗口起点（ordered[0].modified_ms）晚于
// 撤回时刻时，撤回前段不在窗口内——必须跳过该水位的过滤并如实提示，不能把
// 窗口内更晚的用户事件当 ordinal 目标错裁。
resetView();
state.sessionId = 'bcc0ffee-1111-2222-3333-444455556666';
state.boundSession8 = 'bcc0ffee';
state.sessionRoot = 'D:\\CLI\\orz';
state.conversations = [
  { session8: 'bcc0ffee', session_id: 'bcc0ffee-1111-2222-3333-444455556666', root: 'D:\\CLI\\orz' },
];
state.runs = [
  // 窗口起点（最早 run 的 mtime）晚于撤回时刻 TS_BASE+6s ⇒ 撤回前段缺失。
  { run_id: 'RUN-bcc0ffee-1', status: 'completed', modified_ms: TS_BASE + 60000 },
];
const gapPromptsBefore = state.content.items.length;
await withFetch(
  recallEventsApi(
    { 'RUN-bcc0ffee-1': runOneEvents },
    [{ ordinal: 2, recalled_at: TS_BASE / 1000 + 6, removed_messages: 3 }],
  ),
  async () => {
    await ui.openUri('conversation://bcc0ffee');
    await flush();
  },
);
assert.deepEqual(
  userTexts(),
  ['旧问题一', '旧问题二'],
  '窗口缺撤回前段时必须按原样重放（不得跨撤回点错裁）',
);
assert.ok(messageTexts().includes('旧回答二'), '窗口不完整时被撤回 tail 保持原样（诚实降级）');
assert.ok(
  messageTexts().some((t) => t.includes('journal 窗口缺撤回前段')),
  '窗口缺口必须给出一次系统行提示',
);
assert.ok(state.content.items.length > gapPromptsBefore, '重放事件与提示必须落在内容面');
console.log('ok  16.5 recall window gap skips filtering（缺撤回前段：跳过过滤＋如实提示）');

// 16.6 334 通知面：`_orz/session_replay` 的「只回放正文」声明与
// `_orz/bridge_ingress_truncated` 的当前页丢帧都必须上屏（不静默）。
resetView();
currentAcpSocket().onmessage({
  data: JSON.stringify({
    jsonrpc: '2.0',
    method: '_orz/session_replay',
    params: {
      sessionId: 'f00dbabe-1111-2222-3333-444455556666',
      mode: 'text_only',
      tool_calls_skipped: 3,
      policy: 'load 只回放用户/助手正文',
    },
  }),
});
await flush(2);
assert.ok(
  messageTexts().some((t) => t.includes('跳过工具调用 3 条')),
  'session_replay 必须说明工具面未回放（跳过条数）',
);
currentAcpSocket().onmessage({
  data: JSON.stringify({
    jsonrpc: '2.0',
    method: '_orz/bridge_ingress_truncated',
    params: { dropped: 'attached_ingress_full', detail: '浏览器接收过慢' },
  }),
});
await flush(2);
assert.ok(
  messageTexts().some((t) => t.includes('接收缓冲已满')),
  'ingress_truncated 必须显式提示当前页丢帧',
);
console.log('ok  16.6 session_replay/ingress_truncated 通知如实上屏');

// 16.7 `_orz/session_state` 的 closing/deleted：关闭期禁发、删除即解绑。
resetView();
state.sessionId = 'f00dbabe-1111-2222-3333-444455556666';
state.boundSession8 = 'f00dbabe';
state.sessionRoot = 'D:\\CLI\\orz';
currentAcpSocket().onmessage({
  data: JSON.stringify({
    jsonrpc: '2.0',
    method: '_orz/session_state',
    params: { sessionId: 'f00dbabe-1111-2222-3333-444455556666', state: 'closing', reason: 'user_close' },
  }),
});
await flush(2);
assert.equal(
  state.closingSessionId,
  'f00dbabe-1111-2222-3333-444455556666',
  'closing 必须置本地关闭门',
);
assert.equal(
  globalThis.document.getElementById('sendBtn').disabled,
  true,
  'closing 期间必须禁用发送按钮',
);
assert.ok(
  messageTexts().some((t) => t.includes('正在关闭')),
  'closing 必须如实提示',
);
currentAcpSocket().onmessage({
  data: JSON.stringify({
    jsonrpc: '2.0',
    method: '_orz/session_state',
    params: { sessionId: 'f00dbabe-1111-2222-3333-444455556666', state: 'deleted', reason: 'delete' },
  }),
});
await flush(2);
assert.equal(state.sessionId, null, 'deleted 必须解绑本地会话');
assert.equal(state.boundSession8, null);
assert.equal(state.closingSessionId, null, '解绑必须清除关闭门');
assert.equal(
  globalThis.document.getElementById('sendBtn').disabled,
  false,
  '解绑后发送按钮必须恢复',
);
assert.ok(
  messageTexts().some((t) => t.includes('已删除')),
  'deleted 必须如实提示「已删除」',
);
console.log('ok  16.7 session_state closing 禁发＋deleted 解绑');

// 16.8 空闲刷新不弹确认（334 §5.9）：无运行/队列/后台时 beforeunload 放行。
state.running = false;
state.serverBusy = false;
state.supplementQueueDepth = 0;
state.backgroundTasks = 0;
let idlePrevented = false;
const idleUnload = {
  returnValue: '',
  preventDefault() {
    idlePrevented = true;
  },
};
unloadHandlers.at(-1)(idleUnload);
assert.equal(idlePrevented, false, '空闲刷新不得 preventDefault（旧实现无条件拦截）');
assert.equal(idleUnload.returnValue, '', '空闲刷新不得设置确认文案');
console.log('ok  16.8 idle beforeunload 不拦截（仅有运行/队列/后台风险时提示）');

// 16.9 桥重启后的显式续接（334 §4.5）：attach 帧无 session_id 但
// sessionStorage 有绑定时，必须 initialize + session/load（带归属根），
// 成功后才 setBinding——不能静默当新 child。
resetView();
hooks.setBinding('feedbeef-1111-2222-3333-444455556666', 'D:\\CLI\\orz');
acpMock.loadHandler = (msg, ws) =>
  wsRespond(ws, msg.id, { session_id: 'feedbeef-1111-2222-3333-444455556666', status: 'loaded' });
const loadsBeforeFallback = methodCalls('session/load').length;
currentAcpSocket().onmessage({
  data: JSON.stringify({
    jsonrpc: '2.0',
    method: '_orz/bridge_attached',
    params: {
      session_id: null,
      reused: false,
      prompt_in_flight: false,
      keepalive_expired: false,
      cwd: 'D:\\CLI\\orz',
    },
  }),
});
await flush(12);
const fallbackLoad = methodCalls('session/load').at(-1);
assert.equal(
  methodCalls('session/load').length,
  loadsBeforeFallback + 1,
  'attach 无会话时必须显式 session/load 续接',
);
assert.equal(
  fallbackLoad?.params?.sessionId,
  'feedbeef-1111-2222-3333-444455556666',
  'session/load 必须带 sessionStorage 记住的会话 id',
);
assert.equal(fallbackLoad?.params?.cwd, 'D:\\CLI\\orz', 'session/load 必须带归属根（不是猜的新 cwd）');
assert.equal(state.sessionId, 'feedbeef-1111-2222-3333-444455556666', '续接成功后必须绑定该会话');
console.log('ok  16.9 attach 无会话：按 sessionStorage 显式 load 续接（带归属根）');

// 16.10 工作区切换（334 §4.5/§5.5）：成功切换后 WS 必须重连到新根，旧绑定
// 会话解绑并留在旧工作区（可回看）；409 时显示服务端 message/hint。
resetView();
state.workspaceCwd = 'D:\\CLI\\orz';
hooks.setBinding('feedbeef-1111-2222-3333-444455556666', 'D:\\CLI\\orz');
const socketsBeforeSwitch = socketInstances.length;
await withFetch(
  fallbackApi({
    '/api/workspace/switch': {
      ok: true,
      cwd: 'D:\\CLI\\orz2',
      trusted_workspaces: [{ path: 'D:\\CLI\\orz2' }],
      recent_workspaces: ['D:\\CLI\\orz', 'D:\\CLI\\orz2'],
    },
  }),
  async () => {
    await ui.openUri('workspace://switch/D%3A%5CCLI%5Corz2');
    await flush();
  },
);
const switchSocket = socketInstances.slice(socketsBeforeSwitch).find((s) => s.url.includes('/ws/acp'));
assert.ok(switchSocket, '工作区切换必须重连 WS（不得继续用旧槽）');
assert.equal(state.workspaceCwd, 'D:\\CLI\\orz2', '切换后 cwd 必须更新');
assert.equal(state.sessionId, null, '切换后旧绑定会话必须解绑');
assert.equal(state.sessionRoot, null, '切换后不得沿用旧会话根');
assert.equal(state.nav.current, 'workspace://live', '切换后回归实时视图');
assert.equal(memoryStorage.has('orz.sessionId'), false, '切换后旧绑定不得留在 sessionStorage');
assert.ok(
  messageTexts().some((t) => t.includes('本对话不迁移') && t.includes('D:\\CLI\\orz')),
  '必须提示旧会话仍在旧工作区（可从探索器恢复）',
);
// 409：服务端有 run 在跑——显示 error.message 与同义 hint。
await withFetch(
  async (url, init) => {
    if (String(url).includes('/api/workspace/switch')) {
      return {
        ok: false,
        status: 409,
        json: async () => ({
          error: {
            code: 'session_busy',
            class: 'session',
            message: '有运行在进行，不能切换工作区。',
            hint: '等本轮结束或点「停止」后再切换；切换会改变新会话的归属根。',
          },
        }),
      };
    }
    return jsonResponse({ runs: [], conversations: [], archives: [] });
  },
  async () => {
    await ui.openUri('workspace://switch/D%3A%5CCLI%5Corz');
    await flush();
  },
);
assert.ok(connBanner.textContent.includes('有运行在进行'), '409 必须显示服务端 message');
assert.ok(connBanner.textContent.includes('等本轮结束'), '409 必须显示可行动 hint');
console.log('ok  16.10 工作区切换：重连新根＋旧会话留旧区＋409 显示 message/hint');

// 16.11 取消兜底（334 §4.10）：停止必须装配精确 30s timer；到点时本地
// 状态收敛为空闲并给出 journal 边界提示；若终态先到，timer 必须撤销，
// 不能在已收敛后再发一条误导性的超时提示。
assert.equal(typeof hooks.onLiveJournalEvent, 'function', '必须可从冒烟钩子注入实时 journal 终态');
const nativeSetTimeout = globalThis.setTimeout;
const nativeClearTimeout = globalThis.clearTimeout;
let nextFakeTimer = 1;
const cancelTimers = new Map();
globalThis.setTimeout = (fn, ms, ...args) => {
  if (ms !== 30000) return nativeSetTimeout(fn, ms, ...args);
  const id = { fakeCancelTimer: nextFakeTimer++ };
  cancelTimers.set(id, { fn, active: true });
  return id;
};
globalThis.clearTimeout = (id) => {
  const timer = cancelTimers.get(id);
  if (timer) {
    timer.active = false;
    return;
  }
  nativeClearTimeout(id);
};
try {
  resetView();
  hooks.setBinding('c0ffee00-1111-2222-3333-444455556666', 'D:\\CLI\\orz');
  state.serverBusy = true;
  vm.setRunning(true);
  const cancelsBefore = methodCalls('session/cancel').length;
  ui.stopRun();
  assert.equal(methodCalls('session/cancel').length, cancelsBefore + 1, '停止必须先发 session/cancel');
  const firstTimer = [...cancelTimers.values()].at(-1);
  assert.ok(firstTimer?.active, '停止必须装配 30s 取消兜底 timer');
  firstTimer.fn();
  assert.equal(state.running, false, '取消 30s 未收敛时必须复位本地 running');
  assert.equal(state.serverBusy, false, '取消 30s 未收敛时必须清 serverBusy');
  assert.ok(
    messageTexts().some((t) => t.includes('取消后 30 秒仍未收敛') && t.includes('journal')),
    '取消超时必须提示本地复位与 journal 真相边界',
  );

  resetView();
  state.serverBusy = true;
  vm.setRunning(true);
  ui.stopRun();
  const lateTimer = [...cancelTimers.values()].at(-1);
  assert.ok(lateTimer?.active, '第二次停止同样装配 timer');
  hooks.onLiveJournalEvent({ event_type: 'run_cancelled', payload: { status: 'cancelled' } });
  assert.equal(lateTimer.active, false, '终态先到时必须撤销 30s timer');
  const timeoutNotices = messageTexts().filter((t) => t.includes('取消后 30 秒仍未收敛')).length;
  if (lateTimer.active) lateTimer.fn();
  assert.equal(
    messageTexts().filter((t) => t.includes('取消后 30 秒仍未收敛')).length,
    timeoutNotices,
    '已撤销 timer 不得在晚到阶段追加超时提示',
  );
} finally {
  globalThis.setTimeout = nativeSetTimeout;
  globalThis.clearTimeout = nativeClearTimeout;
}
console.log('ok  16.11 cancel fallback 30s 收敛＋终态先到撤销 timer');

/* ── 17. 338（2026-10-22 真机 c23e0aaa）：回档压平 mtime 后的会话顺序 ── */
// 17.1 合并回放必须按 run 序号铺开，不得相信 /api/runs 的 mtime 顺序。
// 真机：24 个 journal 的 mtime 被「回档」压到 44 ms 内（16 个不同值），服务端
// 顺序退化为目录名字典序 0,1,10,11,…,23,3,4,5,6,7,8,9 ⇒ 回放末尾落在 run 8/9
// 的旧内容（T9/T10），标记栏随之乱序。
assert.ok(hooks?.orderedSessionRuns && hooks?.latestSessionRun, 'main.js 必须导出 run 排序钩子');
const flatMs = 1792671547700;
const lexicographic = [
  'RUN-c23e0aaa-0', 'RUN-c23e0aaa-1', 'RUN-c23e0aaa-10', 'RUN-c23e0aaa-11',
  'RUN-c23e0aaa-12', 'RUN-c23e0aaa-2', 'RUN-c23e0aaa-20', 'RUN-c23e0aaa-23',
  'RUN-c23e0aaa-3', 'RUN-c23e0aaa-9', 'ARC-c23e0aaa-24', 'RUN-d24eadd6-0',
].map((run_id) => ({ run_id, modified_ms: flatMs }));
const orderedIds = hooks.orderedSessionRuns(lexicographic, 'c23e0aaa').map((r) => r.run_id);
assert.deepEqual(
  orderedIds,
  [
    'RUN-c23e0aaa-0', 'RUN-c23e0aaa-1', 'RUN-c23e0aaa-2', 'RUN-c23e0aaa-3',
    'RUN-c23e0aaa-9', 'RUN-c23e0aaa-10', 'RUN-c23e0aaa-11', 'RUN-c23e0aaa-12',
    'RUN-c23e0aaa-20', 'RUN-c23e0aaa-23', 'ARC-c23e0aaa-24',
  ],
  '会话回放必须按 run 序号旧→新铺开（ARC 审计 run 按同一序号面就位）',
);
assert.ok(
  !orderedIds.includes('RUN-d24eadd6-0'),
  '其他会话的 run 不得混进本会话回放',
);
assert.equal(
  orderedIds.at(-1),
  'ARC-c23e0aaa-24',
  '回放末尾必须是序号最大者（真机：旧实现末尾落在 run 8/9 的 T9/T10 旧内容）',
);
console.log('ok  17.1 conversation replay ordered by run sequence（mtime 压平后不坍塌）');

// 17.2 「最新 run」＝序号最大者，同样不看清单顺序（回档后 state.runs[0] 可能是
// run 9；实时尾曾因此挂在 T9/T10 上，恢复并继续后标记栏只剩最后两条）。
const shuffled = [...lexicographic].reverse();
assert.equal(
  hooks.latestSessionRun(shuffled, 'c23e0aaa')?.run_id,
  'RUN-c23e0aaa-23',
  '最新 run 必须是 RUN-c23e0aaa-23（ARC- 维护 journal 不参与尾随）',
);
assert.equal(
  hooks.latestSessionRun([{ run_id: 'RUN-CLI-6ab7de01', modified_ms: flatMs }], '6ab7de01')?.run_id,
  'RUN-CLI-6ab7de01',
  '无序号形态（RUN-CLI-<s8>）仍须可选中（回退 mtime）',
);
assert.equal(hooks.runSequence('RUN-CLI-6ab7de01'), null, '无序号形态不得伪造序号');
console.log('ok  17.2 latest run by sequence（恢复并继续不再挂到 run 9）');

// 17.3 回档压平 mtime 后，撤回窗口起点必须取 journal 的 run_started 时间戳，
// 不能取 modified_ms——否则「窗口起点（回档时刻）晚于撤回时刻」会被误判成
// 「窗口缺撤回前段」，整条水位被跳过、被撤回内容复活（真机 c23e0aaa：mtime
// 20:19 > 撤回 17:14）。
resetView();
state.conversations = [
  { session8: 'bcc0ffee', session_id: 'bcc0ffee-1111-2222-3333-444455556666', root: 'D:\\CLI\\orz' },
];
const restoreMs = TS_BASE + 9_000_000; // 回档时刻（远晚于撤回）
state.runs = [
  // 服务端新序（新在前），但两条 mtime 都被回档压到同一刻
  { run_id: 'RUN-bcc0ffee-2', status: 'completed', modified_ms: restoreMs, started: at(7) },
  { run_id: 'RUN-bcc0ffee-1', status: 'completed', modified_ms: restoreMs, started: at(1) },
];
await withFetch(
  recallEventsApi(
    { 'RUN-bcc0ffee-1': runOneEvents, 'RUN-bcc0ffee-2': runTwoEvents },
    [{ ordinal: 2, recalled_at: TS_BASE / 1000 + 6, removed_messages: 3 }],
  ),
  async () => {
    await ui.openUri('conversation://bcc0ffee');
  },
);
assert.deepEqual(
  userTexts(),
  ['旧问题一', '新问题三'],
  'mtime 被回档压平后水位仍必须生效（窗口起点取 run_started）',
);
assert.equal(
  messageTexts().some((t) => t.includes('窗口缺撤回前段')),
  false,
  '不得因回档 mtime 误报「窗口缺撤回前段」',
);
assert.equal(messageTexts().includes('旧回答二'), false, '被撤回内容不得复活');
console.log('ok  17.3 recall window start uses run_started（回档 mtime 不影响撤回水位）');

console.log('\nfrontend smoke: 全部通过');
process.exit(0);
