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
globalThis.addEventListener = () => {};
globalThis.location = {
  protocol: 'http:',
  host: '127.0.0.1:21487',
  href: 'http://127.0.0.1:21487/#token=abc',
  hash: '#token=abc',
};
globalThis.WebSocket = class {
  static OPEN = 1;
  constructor(url) {
    this.url = url;
    this.readyState = 0;
    queueMicrotask(() => {
      this.readyState = 1;
      this.onopen?.();
    });
  }
  send() {}
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
assert.equal(typeof ui.newSession, 'function', '新建会话动作已接线');
console.log('ok  工具栏「新建会话」居「后退」之前');

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

console.log('\nfrontend smoke: 全部通过');
process.exit(0);
