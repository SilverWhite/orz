/*
 * main.js —— 装配（0br S2）。启动序：fragment 令牌 → REST/WS 探测 →
 * ACP initialize → session/new → journal 尾跟随；输入行/工具栏/菜单/
 * 键盘动作集中在此接线。投影数据双通道：ACP（流式文本＋权限）与
 * journal 事件（事实面，运行留痕）——去重语义照 orz-tui projection.rs。
 */

import { state, vm } from './state.js';
import { setToken, fetchBoot, fetchConversations, fetchArchives, fetchArchive, postArchive } from './api.js';
import * as acp from './acp.js';
import { refreshRuns, replayRun, tailRun, closeTail } from './journal.js';
import { applyJournalEvent, applyAcpUpdate } from './projection.js';
import { ui, renderAll, renderContent, renderMarker, renderStatus, renderToolbar, renderExplorer, openMenuByName, closeMenus } from './widgets.js';
import * as dialogs from './dialogs.js';
import * as keymap from './keymap.js';

/* ── DOM 引用挂载 ── */
ui.menubar = document.getElementById('menubar');
ui.toolbar = document.getElementById('toolbar');
ui.explorer = document.getElementById('explorer');
ui.content = document.getElementById('content');
ui.contentScroll = document.getElementById('contentScroll');
ui.toolPin = document.getElementById('toolPin');
ui.marker = document.getElementById('marker');
ui.inputRow = document.getElementById('inputRow');
ui.statusbar = document.getElementById('statusbar');

const chatInput = document.getElementById('chatInput');
const sendBtn = document.getElementById('sendBtn');
const connBanner = document.getElementById('connBanner');

function flash(msg) {
  connBanner.hidden = false;
  connBanner.textContent = msg;
  clearTimeout(flash._t);
  flash._t = setTimeout(() => {
    connBanner.hidden = true;
  }, 4000);
}
ui.flash = flash;

/* ── 会话与发送 ── */

let acpSessionId = null;
let currentRunId = null;
let liveMode = true; // workspace://live ＝ 跟随最新 run

async function ensureSession() {
  if (acpSessionId) return acpSessionId;
  await acp.initialize();
  // cwd 来自 /api/boot（前端无法得知工作区路径；桥 spawn 子进程时已用它）。
  let cwd;
  try {
    cwd = (await fetchBoot()).cwd;
  } catch {
    cwd = undefined;
  }
  const result = await acp.sessionNew(cwd);
  acpSessionId = result.sessionId;
  state.sessionId = acpSessionId;
  vm.addSystemMessage(`ACP 会话已创建（${acpSessionId}）`, false);
  renderAll();
  return acpSessionId;
}

async function submitPrompt(text) {
  if (state.running) {
    flash('当前有运行在进行（Ctrl+Z 可取消）');
    return;
  }
  try {
    const sessionId = await ensureSession();
    state.running = true;
    vm.syncToolbar();
    renderToolbar();
    liveMode = true;
    // 新 run 将由 journal 尾自动流入（见 startTail；此处先清空流式指针）
    state.content.currentModelIndex = null;
    const resp = await acp.sessionPrompt(sessionId, text);
    state.running = false;
    vm.syncToolbar();
    renderToolbar();
    void resp;
  } catch (e) {
    state.running = false;
    vm.syncToolbar();
    vm.addSystemMessage(`[错误] ${e.message}`, true);
    renderAll();
  }
}

function stopRun() {
  if (!acpSessionId || !state.running) return;
  acp.sessionCancel(acpSessionId);
  flash('已发送取消');
}

ui.stopRun = stopRun;

/* ── journal 尾：跟随最新 run ── */

/* 探索器数据（走查反馈五修复：/api/conversations 此前从未被调用，
 * "会话"恒为 0；数据到达即重渲，不再等 5s interval）。 */
async function refreshExplorerData() {
  const runs = await refreshRuns();
  let conversations = [];
  try {
    conversations = (await fetchConversations()).conversations || [];
  } catch {
    conversations = state.conversations; // 拉取失败保留旧值
  }
  let archives = [];
  try {
    archives = (await fetchArchives()).archives || [];
  } catch {
    archives = state.archives; // 拉取失败保留旧值
  }
  state.runs = runs;
  state.conversations = conversations;
  state.archives = archives;
  renderExplorer();
  return runs;
}

async function startTail() {
  try {
    const runs = await refreshExplorerData();
    if (!runs.length) return;
    const runId = runs[0].run_id;
    if (runId === currentRunId) return;
    currentRunId = runId;
    state.liveRunId = runId;
    tailRun(runId, 0, (ev) => {
      applyJournalEvent(ev);
      scheduleRender();
    });
  } catch (e) {
    // 桥不可达/清单失败：如实显示，不留空白（错误路径修复 2026-09-25）。
    vm.addSystemMessage(`（实时跟随不可用：${e.message}）`, true);
    renderAll();
  }
}

/* 历史回放：把旧 run 事件整体重放进当前视图（run:// 导航）。
 * currentRunId 必须复位（审查处理批 C-2）：否则「后退」到实时时
 * startTail 早退，尾流永不恢复、页面停在回放内容上。 */
async function openRunReplay(runId) {
  closeTail();
  currentRunId = null;
  state.liveRunId = null;
  liveMode = false;
  state.content = { items: [], currentModelIndex: null, droppedItems: 0 };
  state.marker = [];
  state.turnCounter = 0;
  try {
    await replayRun(runId, (ev) => applyJournalEvent(ev));
  } catch (e) {
    vm.addSystemMessage(`（运行 ${runId} 读取失败：${e.message}）`, true);
  }
  vm.addSystemMessage(`（历史回放 ${runId} —— 只读；点「后退」返回实时）`, false);
  renderAll();
}

/* 回到实时（workspace://live）：清空回放残留并整体重灌活动 run。 */
async function returnToLive() {
  closeTail();
  currentRunId = null;
  liveMode = true;
  state.content = { items: [], currentModelIndex: null, droppedItems: 0 };
  state.marker = [];
  state.turnCounter = 0;
  renderAll();
  await startTail();
}

/* 会话合并视图（走查反馈六 2026-09-25）：同一会话的全部运行按时间序
 * 连续重放进同一内容区——一整个长窗口，不逐运行割裂。 */
async function openConversation(s8) {
  const re = new RegExp(`^(?:RUN-CLI-|ARC-|RUN-)${s8}`);
  const ordered = state.runs.filter((r) => re.test(r.run_id)).slice().reverse(); // 旧→新
  closeTail();
  currentRunId = null;
  state.liveRunId = null;
  liveMode = false;
  state.content = { items: [], currentModelIndex: null, droppedItems: 0 };
  state.marker = [];
  state.turnCounter = 0;
  try {
    for (const run of ordered) {
      await replayRun(run.run_id, (ev) => applyJournalEvent(ev));
    }
  } catch (e) {
    // 中途失败也要给出可见答复，不能留空白（错误路径修复 2026-09-25）。
    vm.addSystemMessage(`（会话 ${s8} 打开失败：${e.message}）`, true);
  }
  vm.addSystemMessage(
    ordered.length
      ? `（会话回放 ${s8} —— ${ordered.length} 次运行合并；只读；点「后退」返回实时）`
      : `（${s8} 暂无运行记录；侧车只读）`,
    false,
  );
  renderAll();
}

/* ── 归档只读浏览（0br S3 新增面，用户令 2026-09-25「会话的归档和
 * 查看归档会话都要做的」）：archive://{s8} → 桥 /api/archives/{s8}
 * 摘要＋有界转写，静态装填进内容区。零执行事实：读取、截断判定全在
 * 桥侧，前端只渲染。 ── */

const ARCHIVE_ROLE_CN = { user: '用户', assistant: '模型', system: '系统', tool: '工具' };

function archiveFactsLine(d) {
  const parts = [];
  if (d.archived_at) parts.push(`归档于 ${d.archived_at}`);
  if (d.archived_tokens) parts.push(`约 ${Math.round(d.archived_tokens / 1000)}K tokens`);
  parts.push(`消息 ${d.message_count ?? '?'} 条`);
  // LIF 跨度在三键段 archive_keys.lif（round_start/round_end）——
  // conversation.lif 用的是 sidecar 原生键（entry_round/round），不是一把尺。
  const klif = d.archive_keys?.lif || {};
  if (klif.round_start != null) parts.push(`LIF 轮 ${klif.round_start}→${klif.round_end}${klif.domain ? `（${klif.domain}）` : ''}`);
  const keys = d.archive_keys || {};
  if (keys.ledger?.exists) parts.push(`台账 seq ${keys.ledger.first_seq}→${keys.ledger.last_seq}`);
  if (Array.isArray(keys.journal?.runs) && keys.journal.runs.length) parts.push(`运行 ${keys.journal.runs.length} 个`);
  return `（归档 ${d.session8} —— ${parts.join(' · ')}）`;
}

function renderArchiveTranscript(d) {
  const byCallId = new Map();
  let turn = 0;
  for (const m of d.messages || []) {
    const role = ARCHIVE_ROLE_CN[m.role] || m.role || '未知';
    const text = (m.content || '') + (m.truncated ? '\n\n（…… 已截断，全文经归档回查）' : '');
    if (m.role === 'user') {
      turn++;
      vm.addArchivedMessage('用户', text, turn);
    } else if (m.role === 'assistant') {
      if (text.trim()) vm.addArchivedMessage('模型', text, turn);
      for (const c of m.tool_calls || []) {
        vm.addOrUpdateToolTrace(c.name || '工具', vm.summarizeArguments(c.arguments || ''), '');
        if (c.call_id) byCallId.set(c.call_id, c.name || '工具');
      }
    } else if (m.role === 'tool') {
      // 工具结果并入对应调用行的最新明细（call_id 对不上时如实丢弃）。
      const name = m.tool_call_id ? byCallId.get(m.tool_call_id) : null;
      if (name) vm.updateToolLatest(name, String(m.content || '').slice(0, 200));
    } else if (text.trim()) {
      vm.addArchivedMessage(role, text, turn);
    }
  }
}

async function openArchive(s8) {
  closeTail();
  currentRunId = null;
  state.liveRunId = null;
  liveMode = false;
  state.content = { items: [], currentModelIndex: null, droppedItems: 0 };
  state.marker = [];
  state.turnCounter = 0;
  let d;
  try {
    d = await fetchArchive(s8);
  } catch (e) {
    vm.addSystemMessage(`（归档 ${s8} 读取失败：${e.message}）`, true);
    renderAll();
    return;
  }
  vm.addSystemMessage(archiveFactsLine(d), false);
  renderArchiveTranscript(d);
  // 归档转写的「最终输出」锚（批四 2026-09-25：标记栏在归档浏览同样可用
  // ——用户锚在转写装填时逐条产生，输出锚按实时视图同规则只定正式输出）。
  vm.anchorFinalOutput();
  vm.addSystemMessage(
    `（归档浏览 ${s8} —— 只读${d.transcript_truncated ? `；转写超上限已截断（${d.messages_shown}/${d.messages_total} 条）` : ''}；点「后退」返回）`,
    false,
  );
  renderAll();
}

/* ── UI 动作接线 ── */

ui.toggleExplorer = () => {
  state.explorerVisible = !state.explorerVisible;
  ui.explorer.classList.toggle('hidden', !state.explorerVisible);
};
ui.toggleEvents = () => ui.toggleExplorer(); // v1：事件与探索器同栏
ui.toggleMarkers = () => {
  state.markerVisible = !state.markerVisible;
  ui.marker.classList.toggle('hidden', !state.markerVisible);
};
ui.openHelp = (tab) => dialogs.openHelp(tab);
ui.openProperties = () => dialogs.openProperties();
ui.openCommand = (initial) => dialogs.openCommand(initial);
ui.openFind = () => dialogs.openFind();
ui.focusExplorer = () => {
  if (!state.explorerVisible) ui.toggleExplorer();
  ui.explorer.classList.add('focus-ring');
  ui.explorer.focus();
  setTimeout(() => ui.explorer.classList.remove('focus-ring'), 1200);
};
ui.refresh = async () => {
  await refreshExplorerData();
  flash('运行历史已刷新');
};
ui.flashStatus = () => flash(`当前模型: ${state.modelLabel}`);

/* 归档活跃会话（0br S3 用户令 2026-09-25）：POST 递单，执行在桥 spawn
 * 的 `orz archive <s8>` 子命令（复用 agent 关闭归档原语）。成功后刷新
 * 探索器——会话随即移入「归档会话」组。 */
ui.archiveSession = async (s8) => {
  if (!confirm(`归档会话 ${s8}？（打包为 .gsa/archives 归档包＋ARC 审计 journal；侧车保留至保留期清扫）`)) return;
  try {
    const r = await postArchive(s8);
    flash(r.message || `已归档 ${s8}`);
  } catch (e) {
    flash(`归档失败：${e.message}`);
  }
  await refreshExplorerData();
};

ui.navAction = async (dir) => {
  if (dir === 'back') {
    const prev = state.nav.back[state.nav.back.length - 1];
    vm.navBack();
    if (prev === 'workspace://live') {
      await returnToLive();
    }
  } else {
    vm.navForward();
    renderAll();
  }
};

// 对象 URI 路由（AddressBar 语义，R-4）。v1 投影面：run://（单运行回放）、
// conversation://（会话合并视图，走查反馈六）、archive://（归档只读浏览，
// 0br S3 新增面）、workspace://live；其余 scheme 显式给出「无投影」答复，
// 不静默。
ui.openUri = async (uri) => {
  if (uri.startsWith('run://')) {
    vm.navGo(uri);
    await openRunReplay(uri.slice('run://'.length));
  } else if (uri === 'workspace://live') {
    vm.navGo(uri);
    await returnToLive();
  } else if (uri.startsWith('conversation://')) {
    vm.navGo(uri);
    await openConversation(uri.slice('conversation://'.length));
  } else if (uri.startsWith('archive://')) {
    // 归档浏览（0br S3 新增面）：只读静态投影，不动实时尾。
    vm.navGo(uri);
    await openArchive(uri.slice('archive://'.length));
  } else if (/^(source|claim|adapter|artifact):\/\//.test(uri)) {
    vm.navGo(uri);
    vm.addSystemMessage(`（${uri} —— 该对象类型 v1 无投影面；事实经 journal 运行历史呈现）`, false);
    renderAll();
  }
};

ui.scrollToItem = (itemIndex) => {
  // 跳转顺带展开折叠卡（走查反馈 2026-09-25：锚点直达正文）。
  const item = state.content.items[itemIndex];
  if (item?.kind === 'message' && item.collapsed) {
    item.collapsed = false;
    renderContent();
  }
  // renderContent 会在截断提示存在时多渲染一个首行，DOM 序号需偏移。
  const offset = state.content.droppedItems > 0 ? 1 : 0;
  const nodes = ui.contentScroll.children;
  nodes[itemIndex + offset]?.scrollIntoView({ block: 'start' });
};

ui.findInConversation = (query, scope) => {
  vm.clearHits();
  renderMarker();
  if (scope !== '对话') throw new Error(`${scope} 范围 v1 未接线（仅「对话」可用）`);
  if (!query) return 0;
  // 字面匹配（审查处理批 F-4）：用户输入按正则元字符转义——既防 ReDoS，
  // 也避免输入 `(` 之类字符直接抛错。
  const escaped = query.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const re = new RegExp(escaped, 'i');
  let hits = 0;
  state.content.items.forEach((item, idx) => {
    if (item.kind !== 'message' || !item.content) return;
    if (re.test(item.content)) {
      hits++;
      vm.addSearchHit(`L${idx + 1}`, idx);
    }
  });
  renderMarker();
  return hits;
};

/* 渲染节流（审查处理批 F-1）：journal 尾与 ACP 流式回调经 rAF 合帧，
 * 高频事件不再每条全量重绘。 */
let renderQueued = false;
function scheduleRender() {
  if (renderQueued) return;
  renderQueued = true;
  requestAnimationFrame(() => {
    renderQueued = false;
    renderContent();
    renderStatus();
    renderMarker();
    renderToolbar();
  });
}

/* Tab 区内焦点移动（补充稿 §10；F6 保持三大区切换）。 */
const REGION_CYCLE = () => [ui.menubar, ui.toolbar, ui.explorer, ui.contentScroll, chatInput].filter(Boolean);

const actions = {
  ...ui,
  // Esc 关弹窗时顺带收起打开中的菜单下拉（实机走查补笔：菜单开着又
  // 开弹窗/关弹窗，下拉会残留在模态层后面）。
  closeModal: () => {
    dialogs.closeModal();
    closeMenus();
  },
  isModalOpen: () => dialogs.isModalOpen(),
  openMenu: (name) => openMenuByName(name),
  cycleHelpTab: (dir) => {
    const win = document.querySelector('#modalLayer .modal-window');
    win?._cycleTab?.(dir);
  },
  cycleFocus: () => {
    const order = [chatInput, ui.explorer, ui.contentScroll];
    const active = document.activeElement;
    let i = order.findIndex((n) => n === active || n.contains?.(active));
    i = (i + 1) % order.length;
    order[i].focus();
    order[i].classList?.add?.('focus-ring');
    setTimeout(() => order[i].classList?.remove?.('focus-ring'), 1200);
  },
  cycleFocusInRegion: (dir) => {
    const active = document.activeElement;
    const region = REGION_CYCLE().find((r) => r === active || r.contains?.(active));
    if (!region) return;
    const focusables = Array.from(
      region.querySelectorAll('button:not([disabled]), [tabindex="0"], textarea, input, select'),
    ).filter((n) => n.tagName === 'TEXTAREA' || n.tagName === 'INPUT' || n.offsetParent !== null);
    if (!focusables.length) return;
    const i = focusables.indexOf(active);
    const next = i === -1 ? (dir > 0 ? 0 : focusables.length - 1) : (i + dir + focusables.length) % focusables.length;
    focusables[next].focus();
  },
  runAddress: (v) => {
    // slash 命令与对象 URI 的统一入口（AddressBar 语义，R-4）。
    if (v.startsWith('/')) {
      runSlash(v);
    } else if (v.startsWith('command://')) {
      runSlash('/' + v.slice('command://'.length).replace(/^\/*/, ''));
    } else if (v.includes('://')) {
      ui.openUri(v);
    } else if (v) {
      chatInput.value = v;
      chatInput.focus();
    }
  },
  sendPrompt: submitPrompt,
};

function runSlash(cmd) {
  const name = cmd.split(/\s+/)[0];
  switch (name) {
    case '/help':
      dialogs.openHelp();
      break;
    case '/status':
      ui.flashStatus();
      break;
    case '/stop':
      stopRun();
      break;
    case '/open':
      dialogs.openCommand('command://');
      break;
    case '/toggle-explorer':
      ui.toggleExplorer();
      break;
    case '/toggle-events':
      ui.toggleEvents();
      break;
    case '/toggle-markers':
      ui.toggleMarkers();
      break;
    case '/properties':
      dialogs.openProperties();
      break;
    case '/snapshots':
      flash('快照选择器 v1 为只读展示（恢复未接线）');
      break;
    case '/grill':
    case '/grill-finish':
      flash('设计拷问（grill）v1 未接线');
      break;
    default:
      flash(`未知命令 ${name}（见 /help）`);
  }
}

dialogs.bindDialogActions({ ...actions, findInConversation: ui.findInConversation, flash });
keymap.bindKeyActions(actions);
keymap.install();

/* ── 输入行 ── */

const inputHistory = [];
let historyIndex = null;

function submitFromInput() {
  const text = chatInput.value.trim();
  if (!text) return;
  if (text.startsWith('/')) {
    runSlash(text);
    chatInput.value = '';
    return;
  }
  inputHistory.push(text);
  historyIndex = null;
  chatInput.value = '';
  submitPrompt(text);
}

sendBtn.addEventListener('click', submitFromInput);
chatInput.addEventListener('keydown', (e) => {
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault();
    submitFromInput();
  } else if (e.key === 'ArrowUp' && !chatInput.value && inputHistory.length) {
    historyIndex = historyIndex == null ? inputHistory.length - 1 : Math.max(0, historyIndex - 1);
    chatInput.value = inputHistory[historyIndex];
    e.preventDefault();
  } else if (e.key === 'ArrowDown' && historyIndex != null) {
    historyIndex = Math.min(inputHistory.length - 1, historyIndex + 1);
    chatInput.value = inputHistory[historyIndex] ?? '';
    e.preventDefault();
    if (historyIndex === inputHistory.length - 1 && !inputHistory[historyIndex]) historyIndex = null;
  }
});

document.getElementById('btnQuit').addEventListener('click', () => {
  if (confirm('退出 orz 工作台？（桥接会话将一并结束）')) {
    window.close();
    closeTail();
    flash('请关闭浏览器标签页（服务随 Ctrl+C 退出）');
  }
});

// 刷新/关闭守卫（审查处理批 B2 登记）：v1 无断线重连——桥为单会话且
// 连接关闭即结束 `orz --stdio` 子进程。有活动连接时先经浏览器确认。
window.addEventListener('beforeunload', (e) => {
  if (acp.isConnected()) {
    e.preventDefault();
    e.returnValue = '';
  }
});

/* ── ACP 事件接线 ── */

acp.on('update', (sessionId, update) => {
  applyAcpUpdate(sessionId, update);
  scheduleRender();
});
acp.on('permission', (request) => dialogs.showPermission(request));
acp.on('closed', () => {
  flash('桥接已关闭（服务退出或会话结束）');
  state.running = false;
  renderStatus();
  renderToolbar();
});
acp.on('open', () => flash('桥接已连接'));

/* ── 启动 ── */

async function boot() {
  // 令牌来自启动控制台打印的 URL fragment（#token=…，lib.rs 打印同格式）。
  const hash = new URLSearchParams(location.hash.replace(/^#/, ''));
  const token = hash.get('token') || '';
  if (!token) {
    connBanner.hidden = false;
    connBanner.textContent = '缺少访问令牌：请从 orz 控制台打印的完整地址打开本页。';
    renderAll();
    return;
  }
  setToken(token);
  // 工作区事实（当前 cwd＋已信任工作区清单）先行拉取——探索器「工作区」
  // 组的数据源，不等首次会话创建。
  try {
    const b = await fetchBoot();
    state.workspaceCwd = b.cwd || '';
    state.trustedWorkspaces = b.trusted_workspaces || [];
    renderExplorer();
  } catch {
    /* boot 不可达：探索器仍显示 nav 当前值 */
  }
  try {
    await acp.connect(token);
  } catch (e) {
    connBanner.hidden = false;
    connBanner.textContent = `连接失败: ${e.message}`;
  }
  renderAll();
  await startTail();
  // 每 5 秒刷新探索器数据（journal 尾为实时事实通道）。
  setInterval(async () => {
    try {
      await refreshExplorerData();
    } catch {
      /* 服务未起/令牌变化：横幅已提示 */
    }
  }, 5000);
}

boot();
