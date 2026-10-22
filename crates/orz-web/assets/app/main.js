/*
 * main.js —— 装配（0br S2）。启动序：fragment 令牌 → REST/WS 探测 →
 * ACP initialize → session/new → journal 尾跟随；输入行/工具栏/菜单/
 * 键盘动作集中在此接线。投影数据双通道：ACP（流式文本＋权限）与
 * journal 事件（事实面，运行留痕）——去重语义照 orz-tui projection.rs。
 */

import { state, vm } from './state.js';
import { setToken, fetchBoot, fetchConversations, fetchArchives, fetchArchive, postArchive, unarchiveSession, deleteSession, grantTrust, switchWorkspace, fetchRecall } from './api.js';
import * as acp from './acp.js';
import { refreshRuns, replayRun, tailRun, closeTail } from './journal.js';
import { applyJournalEvent, applyAcpUpdate } from './projection.js';
import { ui, renderAll, renderContent, renderMarker, renderStatus, renderToolbar, renderExplorer, openMenuByName, closeMenus } from './widgets.js';
import * as dialogs from './dialogs.js';
import * as keymap from './keymap.js';
import { describeError } from './copy.js';

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
  // 批六：横幅过 sanitize——子进程 stderr 透传的错误可能带 ANSI 转义码
  // （tracing 着色），横幅不走 md/内容面净化管道，需就地剥除。
  connBanner.hidden = false;
  connBanner.textContent = vm.sanitizeText(msg);
  clearTimeout(flash._t);
  flash._t = setTimeout(() => {
    connBanner.hidden = true;
  }, 4000);
}
/* 0bv ②：路径归一（Windows 大小写/分隔符不敏感比较——信任清单是 canonical
 * 存储键，前端比较须同口径，否则「当前工作区」判定与切换门会抖动）。 */
const pathNorm = (p) =>
  (p || '').replace(/\\/g, '/').replace(/\/+$/, '').toLowerCase();

ui.flash = flash;

/* ── 会话与发送 ── */

let acpSessionId = null;
let currentRunId = null;
let liveMode = true; // workspace://live ＝ 跟随最新 run
// 318 P1-B3：会话恢复代次。start 时取 token，每次 await 后复核 token/导航
// 代次/运行态/目标视图；加载期间禁发 prompt、禁切会话、禁回 live。
let sessionLoadToken = 0;
let pendingLoadSessionId = null;
// 318 P1-B1/B3：本地 run 的会话句柄快照。stopRun/Ctrl+Z 只取消「实际承载
// 本 run」的会话，不用漂移后的 acpSessionId 去取消别人。
let currentRunSessionId = null;
// 318 P1-B1：撤回预览/撤回必须作用于同一个显式会话（服务端按 session_id
// 校验）；绑定/视图变化后旧预览作废，必须重新打开 /rewind。
let rewindSessionId = null;

/* ── 334（§4.1/§4.5/§5.9）：重连、绑定记忆、取消兜底 ── */
let accessToken = '';
// 刷新后 sessionStorage 里的绑定（桥重启 / 新 child 时经 session/load 显式续接）。
const SESSION_ID_KEY = 'orz.sessionId';
const SESSION_ROOT_KEY = 'orz.sessionRoot';
let rememberedSessionId = null;
let rememberedSessionRoot = null;
// 1s/2s/4s 三次退避；成功后清零。页面主动退出（btnQuit）不再重连。
const RECONNECT_DELAYS = [1000, 2000, 4000];
let reconnectAttempt = 0;
let reconnectTimer = null;
let pageExiting = false;
// 334（§4.3）：服务端 `_orz/session_state(closing)` 的本地门——关闭期间禁发。
let closingSessionId = null;
// 334（§4.11）：`_orz/session_replay` 口径声明——load 回放后本地会清空历史
// 投影（避免重复渲染），这条说明要在恢复成功后补回，不能被一起清掉。
let lastReplayNotice = null;
// 334（§4.5）：工作区切换期间抑制「桥重启后按本地记忆 load」——旧会话属于
// 旧根，不能拿旧 sessionStorage 去新根 load（切换完成后绑定本就要丢弃）。
let attachFallbackSuspended = false;
// 334（§4.10）：取消后 30s 仍未收敛则本地复位（服务端可能仍在收尾）。
const CANCEL_FALLBACK_MS = 30000;
let cancelFallbackTimer = null;

function sessionStoreGet(key) {
  try {
    return sessionStorage.getItem(key);
  } catch {
    return null;
  }
}

function sessionStoreSet(key, value) {
  try {
    if (value == null || value === '') sessionStorage.removeItem(key);
    else sessionStorage.setItem(key, value);
  } catch {
    /* sessionStorage 不可用（隐私模式/测试 stub）：内存变量仍持有绑定。 */
  }
}

function clearCancelFallback() {
  if (cancelFallbackTimer != null) {
    clearTimeout(cancelFallbackTimer);
    cancelFallbackTimer = null;
  }
}

function armCancelFallback() {
  clearCancelFallback();
  cancelFallbackTimer = setTimeout(() => {
    cancelFallbackTimer = null;
    if (!state.running) return;
    state.serverBusy = false;
    currentRunSessionId = null;
    vm.setRunning(false);
    vm.addSystemMessage(
      '（取消后 30 秒仍未收敛，已复位为本地空闲；服务端可能仍在收尾，完整事实见 journal）',
      true,
    );
    renderAll();
  }, CANCEL_FALLBACK_MS);
}

function setClosing(sessionId) {
  closingSessionId = sessionId || null;
  state.closingSessionId = closingSessionId;
  sendBtn.disabled = !!closingSessionId;
  vm.syncToolbar();
}

const TERMINAL_EVENT_TYPES = new Set([
  'run_finished',
  'run_failed',
  'run_cancelled',
  'run_invalidated',
  'run_terminated',
]);
const isTerminalEvent = (ev) => TERMINAL_EVENT_TYPES.has(ev?.event_type);

/* 会话 run id 口径（318 P1-B5）：模型 run＝`RUN-<s8>-*`，CLI 侧车 run＝
 * 精确 `RUN-CLI-<s8>`（无尾分隔符——前缀匹配会误吞近似会话）。只认这两
 * 种，ARC/RST 维护 journal 永不跟随。 */
const isSessionRunId = (runId, s8) =>
  !!runId && (runId.startsWith(`RUN-${s8}-`) || runId === `RUN-CLI-${s8}`);

const escapeRegExp = (s) => String(s ?? '').replace(/[.*+?^${}()|[\]\\]/g, '\\$&');

/* 318 P2（host 侧 2026-10-19 变更）：`session/load` 要求绝对 cwd 且会
 * canonicalize。boot/cwd 正常是绝对路径；缺省时回退 /api/boot 一次，仍
 * 不是绝对路径则拒绝恢复（不把相对路径交给 host）。 */
const isAbsolutePath = (p) => /^(?:[A-Za-z]:[\\/]|\\\\|\/)/.test(p || '');

async function resolveLoadCwd(target) {
  // 2026-10-21：优先用会话的**归属工作区根**（ConversationSummary.root）。
  // 切换工作区后，旧会话仍打开在窗内；若用当前 cwd 去 load，host 会在
  // 新根的 `.gsa/conversations/` 找不到侧车并报 `session not found`。
  let cwd = target && isAbsolutePath(target.root) ? target.root : state.workspaceCwd;
  if (!isAbsolutePath(cwd)) {
    try {
      const b = await fetchBoot();
      if (isAbsolutePath(b.cwd)) {
        cwd = b.cwd;
        state.workspaceCwd = cwd;
      }
    } catch {
      /* boot 不可达：保持原值，由调用方拒绝。 */
    }
  }
  return cwd;
}

const viewedConversation8 = () => {
  const current = state.nav.current;
  if (current.startsWith('conversation://')) {
    return current.slice('conversation://'.length);
  }
  // 2026-10-21 恢复修复：归档视图本身就是「可恢复的只读会话」——不再要求
  // 用户先手动回档；工具栏「恢复并继续」据此可用（host 侧 load 自动回档）。
  if (current.startsWith('archive://')) {
    return current.slice('archive://'.length);
  }
  return null;
};
const boundSession8 = () => (acpSessionId ? acpSessionId.slice(0, 8) : null);

/* 只读视图门（318 P1-B1）：归档/运行回放/跨会话回看都是只读历史面。
 * submitPrompt 与 /rewind 共用；拒绝时调用方必须保留草稿。 */
function viewReadOnlyReason() {
  if (state.nav.current.startsWith('archive://')) {
    return '当前是归档浏览（只读）；不能发送或撤回——请点「恢复并继续」或返回实时窗口';
  }
  if (state.nav.current.startsWith('run://')) {
    return '当前是运行历史回放（只读）；不能发送或撤回——请返回实时窗口';
  }
  const viewed = viewedConversation8();
  if (viewed && viewed !== boundSession8()) {
    return `当前查看会话 ${viewed} 的历史（只读）；请先点工具栏「恢复并继续」显式恢复该会话，或返回实时窗口`;
  }
  return null;
}

function promptGateReason() {
  if (state.loadingSession) return '会话恢复中——加载完成前不能发送输入';
  // 334（§4.3）：服务端已宣告 closing——新 prompt 会被拒绝，先禁用发送。
  if (closingSessionId) return '会话正在关闭——请等待关闭完成后再发送';
  return viewReadOnlyReason();
}

function rewindGateReason() {
  if (state.loadingSession) return '会话恢复中——加载完成前不能撤回';
  const readonly = viewReadOnlyReason();
  if (readonly) return readonly;
  if (!acpSessionId) return '当前没有绑定会话；撤回只作用于已恢复的活跃会话';
  return null;
}

function setBinding(sessionId, root) {
  acpSessionId = sessionId;
  state.sessionId = sessionId;
  state.boundSession8 = sessionId ? sessionId.slice(0, 8) : null;
  state.sessionRoot = root || state.workspaceCwd;
  // 334（§4.5）：把绑定写进 sessionStorage——刷新后 boot 可把 root/session
  // 带回 WS，桥重启时也能据此显式 session/load 续接。
  rememberedSessionId = sessionId || null;
  rememberedSessionRoot = state.sessionRoot || null;
  if (sessionId) {
    sessionStoreSet(SESSION_ID_KEY, sessionId);
    sessionStoreSet(SESSION_ROOT_KEY, state.sessionRoot || '');
  }
  // 334（§4.10）：新会话/新绑定不继承旧读数（队列深度、剩余额度、后台计数）。
  state.supplementQueueDepth = 0;
  state.queueRemaining = null;
  state.queueScope = null;
  state.backgroundTasks = 0;
  setClosing(null);
  rewindSessionId = null;
  vm.syncToolbar();
}

/* 解绑本地会话（318 P1-B2/P2）。ACP 0.10.4 无 session/close，服务端会话
 * 对象只能随进程结束；至少清本地绑定并停 tail——下一条 prompt 必须显式
 * 新建/恢复，不能把已删除/旧会话的 sidecar 再写回。 */
function detachBinding() {
  closeTail();
  currentRunId = null;
  state.liveRunId = null;
  acpSessionId = null;
  state.sessionId = null;
  state.boundSession8 = null;
  state.sessionRoot = null;
  rememberedSessionId = null;
  rememberedSessionRoot = null;
  // 334（§4.5）：显式解绑后不得再被 boot/attach 的 remembered 值复活。
  sessionStoreSet(SESSION_ID_KEY, null);
  sessionStoreSet(SESSION_ROOT_KEY, null);
  currentRunSessionId = null;
  pendingLoadSessionId = null;
  state.loadingSession = false;
  sessionLoadToken += 1;
  rewindSessionId = null;
  state.resumeTarget = null;
  // 334（§4.10）：队列/额度/后台任务全部清零；取消兜底 timer 一并撤回。
  clearCancelFallback();
  state.serverBusy = false;
  state.supplementQueueDepth = 0;
  state.queueRemaining = null;
  state.queueScope = null;
  state.backgroundTasks = 0;
  setClosing(null);
  lastReplayNotice = null;
  vm.syncToolbar();
}

function clearTranscript() {
  state.content = { items: [], currentModelIndex: null, droppedItems: 0 };
  state.marker = [];
  state.turnCounter = 0;
}

async function ensureSession() {
  if (acpSessionId) return acpSessionId;
  if (state.loadingSession) throw new Error('会话恢复中——请等待或取消恢复');
  await acp.initialize();
  // cwd 来自 /api/boot（前端无法得知工作区路径；桥 spawn 子进程时已用它）。
  let cwd;
  try {
    cwd = (await fetchBoot()).cwd;
  } catch {
    cwd = undefined;
  }
  const result = await acp.sessionNew(cwd);
  setBinding(result.sessionId, cwd || state.workspaceCwd);
  // 0bv ②：记录本会话的**归属工作区根**——切换工作区后本会话不迁移
  // （对话只认初始工作区），其尾/回放按该根解析（旧工作区会话＝回看）。
  vm.addSystemMessage(`ACP 会话已创建（${result.sessionId}）`, false);
  renderAll();
  return result.sessionId;
}

async function queueSupplement(text) {
  try {
    const sessionId = await ensureSession();
    if (state.loadingSession || promptGateReason()) {
      flash('视图或会话已变化，补充未入队');
      return;
    }
    const r = await acp.sessionQueueAppend(sessionId, text);
    // 334（§4.10）：深度/剩余额度直接用服务端读数——不再 Math.max（切会话
    // 后的旧高值会永远压着新会话），也不在本地减额度。
    state.supplementQueueDepth = Number.isFinite(r.depth) ? r.depth : state.supplementQueueDepth;
    if (Number.isFinite(r.capacity)) state.queueCapacity = r.capacity;
    state.queueRemaining = Number.isFinite(r.remaining) ? r.remaining : null;
    state.queueScope = r.scope || null;
    // 333：剩余额度与作用域一并说明——队列只在本会话进程内，刷新/换页/
    // 子进程回收都不保留（此前只报“第 N 条”，用户无从知道上限与生命周期）。
    const quota = Number.isFinite(r.capacity)
      ? `第 ${r.depth}/${r.capacity} 条，剩余 ${Number.isFinite(r.remaining) ? r.remaining : '—'} 个额度`
      : `第 ${r.depth} 条`;
    vm.addSystemMessage(
      `已排队（${quota}）——将在可用的下一次模型调用边界投递；仅本会话进程，刷新/换页不保留`,
      false,
    );
    renderAll();
  } catch (e) {
    vm.addSystemMessage(`[错误] 补充入队失败：${e.message}`, true);
    renderAll();
  }
}

/* 318 P1-B4：服务端 busy 时状态重同步。客户端可能在刷新后失去本地
 * prompt 句柄，但 agent 仍在跑——状态栏/停止键必须与真实运行同源。
 * 334（§4.1）：旧实现首行 `if (state.running) return` 让 attach 后的
 * 「永久运行中」无法被清单纠正；现在先查清单再决定，且只在「服务端
 * 承认 busy」或「本次来自 attach」时复位，不误清本地真实 prompt。 */
async function resyncBusyState({ fromAttach = false } = {}) {
  try {
    await refreshExplorerData();
  } catch {
    /* 清单不可达时保留错误回显，不另行造状态。 */
  }
  const s8 = boundSession8();
  if (!s8) return;
  const active = state.runs.some(
    (r) => isSessionRunId(r.run_id, s8) && r.status === 'running',
  );
  if (active) {
    state.serverBusy = true;
    currentRunSessionId = acpSessionId;
    vm.setRunning(true);
    vm.addSystemMessage('（服务端报告会话正忙：状态已重同步为「运行中」；终态到达后自动复位）', true);
    renderAll();
    return;
  }
  // 清单里没有本会话的运行中 run：服务端 busy 标记（可能来自旧 prompt 的
  // 本地句柄）或 attach 期旧状态需要复位；本地真实 prompt 不在本路径清。
  if (!state.serverBusy && !fromAttach) return;
  const wasRunning = state.running;
  state.serverBusy = false;
  currentRunSessionId = null;
  vm.setRunning(false);
  if (wasRunning) {
    vm.addSystemMessage('（服务端清单中未找到运行中的 run：状态已复位为空闲）', false);
  }
  renderAll();
}


async function submitPrompt(text, isRetry = false) {
  const gate = promptGateReason();
  if (gate) {
    flash(gate);
    return;
  }
  if (state.running) {
    // 0di（2026-10-18，ADR-0010 §14.88）：运行中不拒绝——入会话级补充队列，
    // 可用的下一次模型调用边界投递（不打断当前轮；CC #50246 deferred
    // 语义）。队列跨 run 存活；取消运行不清队。
    await queueSupplement(text);
    return;
  }
  // 318 P1-B3：await ensureSession 期间导航/绑定可能变化——用代次与视图
  // 快照复核，防把输入暗送到另一个会话。
  const epoch = state.navEpoch;
  const viewAtStart = state.nav.current;
  let sessionId = null;
  try {
    sessionId = await ensureSession();
    if (state.loadingSession || state.navEpoch !== epoch || state.nav.current !== viewAtStart) {
      flash('视图或会话已变化，本次输入未发送');
      return;
    }
    if (state.running) {
      await queueSupplement(text);
      return;
    }
    // A resumed conversation is first shown as a merged historical replay.
    // Before its next prompt, move to the live projection so new output cannot
    // be painted into a stale replay surface.
    if (!liveMode) {
      vm.navGo('workspace://live');
      await returnToLive();
    }
    currentRunSessionId = sessionId;
    vm.setRunning(true);
    renderStatus();
    renderToolbar();
    liveMode = true;
    // 新 run 目录由 agent 在 prompt 处理开始时创建：运行期每秒刷新一次
    // 清单并对准尾随（见 syncLiveTail；对准后自停）——prompt_submitted 等
    // 事实经 /ws/journal 从文件头补发，不丢事件。此处只清流式指针。
    state.content.currentModelIndex = null;
    const follow = setInterval(() => {
      if (!state.running) {
        clearInterval(follow);
        return;
      }
      refreshExplorerData().catch(() => {});
    }, 1000);
    const resp = await acp.sessionPrompt(sessionId, text);
    if (currentRunSessionId === sessionId) currentRunSessionId = null;
    if (!state.serverBusy) vm.setRunning(false);
    // 336（0dn S2）：极短回合（<1 s）可能整轮落在 1 s 跟随轮询之间，
    // 尾随从未挂上 ⇒ 用户卡/工具行/终态事实全缺（真机 background 场景
    // 实证）。prompt 返回后补一次清单刷新，让 journal 尾从 0 重放。
    refreshExplorerData().catch(() => {});
    renderStatus();
    renderToolbar();
    void resp;
  } catch (e) {
    if (sessionId && currentRunSessionId === sessionId) currentRunSessionId = null;
    if (!state.serverBusy) vm.setRunning(false);
    const message = e.message || '';
    // 318 P1-B4：服务端 busy（本客户端无本地句柄）→ 不把状态栏留在空闲。
    if (/already in flight|run or restore is already in flight/i.test(message)) {
      await resyncBusyState();
    }
    // 引导期失败的 run 没有 run_started/终态事件（本次走查实证：信任门
    // fail-closed 只留 run_preflight）——状态栏标签须就地回「空闲」，
    // 不得残留「预检/运行中」。
    vm.setRunState(state.running ? '运行中' : '空闲', true);
    // 336（0dn S2）：失败/极短回合同样补一次尾随（journal 可能已有终态）。
    refreshExplorerData().catch(() => {});
    // 信任窗（批七）：桥链为非交互（is_interactive=false），未授信工作区
    // 被 fail-closed 拒绝且没有 TUI 那样的信任窗——在此补上同语义流程：
    // 确认 → 桥授信（POST /api/trust）→ 自动重发一次。
    if (!isRetry && /workspace not trusted/.test(message) && confirm(`工作区尚未被信任（${state.workspaceCwd}）。信任它并重试？`)) {
      try {
        const r = await grantTrust();
        flash(r.message || '已信任工作区');
        // ⑬（0bs，2026-09-26）：授信后**清单就地刷新**——boot 快照里
        // `trusted_workspaces` 是信任面的单一读数源，授信成功必须让它
        // 立即反映（否则探索器的信任状态停在旧值直到下次手动刷新）。
        try {
          const b = await fetchBoot();
          state.workspaceCwd = b.cwd || state.workspaceCwd;
          state.trustedWorkspaces = b.trusted_workspaces || [];
          renderExplorer();
        } catch {
          /* boot 不可达：授信本身已完成，列表刷新失败不阻断重试。 */
        }
      } catch (trustError) {
        vm.addSystemMessage(`[错误] 授信失败：${trustError.message}`, true);
        renderAll();
        return;
      }
      return submitPrompt(text, true);
    }
    vm.addSystemMessage(`[错误] ${message}`, true);
    renderAll();
  } finally {
    // 334（§4.10）：prompt 真正收敛（成功/失败/信任重试）后取消兜底 timer。
    clearCancelFallback();
  }
}

function stopRun() {
  if (!state.running) {
    // 0dg ②：闲置期按下不再静默——与运行中「已发送取消」同通道反馈。
    flash('当前无运行中');
    return;
  }
  // 318 P1-B3：只取消承载本 run 的会话快照——acpSessionId 可能已被（或
  // 未来再被）其它动作改写，绝不能拿漂移后的绑定去取消别人。
  const target = currentRunSessionId || acpSessionId;
  if (!target) {
    flash('本 run 的会话绑定已丢失，无法发送取消');
    return;
  }
  if (!acp.isConnected()) {
    flash('桥接未连接：取消未发送；请等自动重连完成或刷新页面后再试');
    return;
  }
  acp.sessionCancel(target);
  // 334（§4.10）：取消不是终态——30s 未收敛则本地复位，别让「运行中」永锁。
  armCancelFallback();
  flash('已发送取消');
}

ui.stopRun = stopRun;

/* ── journal 尾：实时窗跟随 ── */

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
  // 318 P1-B4：外部 busy 重同步后，用清单终态复位状态（journal 尾也会在
  // 终态事件到达时复位；两条路径都以真实运行事实为准）。
  if (state.serverBusy && acpSessionId) {
    const s8 = boundSession8();
    const stillRunning = runs.some(
      (r) => isSessionRunId(r.run_id, s8) && r.status === 'running',
    );
    if (!stillRunning) {
      state.serverBusy = false;
      currentRunSessionId = null;
      vm.setRunning(false);
      renderStatus();
      renderToolbar();
    }
  }
  renderExplorer();
  await syncLiveTail();
  return runs;
}

/* 实时尾随对准（新建会话批 2026-09-25）：把 journal 尾切到应跟随的
 * run。每轮 prompt 一个新 run 目录（RUN-{s8}-{n}），尾随必须随 run 切换
 * ——此前只在 boot 时尾随 runs[0]，新 run 的事实（工具行/门控/终态）永
 * 不流入实时窗。已建会话时只跟本会话的最新 run（新建会话后不得再灌旧
 * 会话内容）；尚无会话时跟全局最新（boot 连续性）。切换从 offset 0 重灌
 * ——journal_tail 自文件头补发，事实不丢。 */
async function syncLiveTail() {
  if (!liveMode) return;
  let wanted = null;
  let tailRoot;
  if (acpSessionId) {
    const s8 = acpSessionId.slice(0, 8);
    // 0bv ②：会话只认初始工作区——切换后本会话的 run 列表按其归属根取
    // （跨工作区投影按归属根解析；防切区后尾断/404）。
    const ownsOld =
      state.sessionRoot && pathNorm(state.sessionRoot) !== pathNorm(state.workspaceCwd);
    tailRoot = ownsOld ? state.sessionRoot : undefined;
    let runs = state.runs;
    if (tailRoot) {
      try {
        runs = await refreshRuns(tailRoot);
      } catch {
        runs = state.runs;
        tailRoot = undefined;
      }
    }
    // ARC/RST journals are maintenance facts, never the active model turn.
    // 318 P1-B5：CLI 侧车 run 是精确 `RUN-CLI-<s8>`（无尾分隔符），模型 run
    // 是 `RUN-<s8>-*`；只认这两种，ARC/RST 永不跟随。
    const mine = runs.find((r) => isSessionRunId(r.run_id, s8));
    if (!mine) return; // 新会话首 prompt 前：空实时窗等待
    // 329（2026-10-21）：撤回后实时窗不得把被撤回的旧 run 从头重放。
    // 只有「本会话最新 run 在最近一次撤回时刻之前已结束」时跳过它；撤回后
    // 新起的 run（或仍在跑的 run）照常尾随。
    if (mine.status !== 'running') {
      let recalledAt = 0;
      try {
        const recall = await fetchRecall(s8, tailRoot);
        for (const record of recall?.recall_log || []) {
          recalledAt = Math.max(recalledAt, Number(record?.recalled_at) || 0);
        }
      } catch {
        /* 无水位（旧载体/端点不可用）：按原样尾随，不阻断实时面 */
      }
      if (recalledAt > 0 && Number(mine.modified_ms) <= recalledAt * 1000) {
        currentRunId = mine.run_id; // 标记已处理：别每 5 秒重复提示/重放
        state.liveRunId = null;
        vm.addSystemMessage(
          '（撤回后实时窗从下一个 run 开始；被撤回的旧运行不再重放，历史经会话回看）',
          false,
        );
        renderAll();
        return;
      }
    }
    wanted = mine.run_id;
  } else {
    wanted = state.runs[0]?.run_id ?? null;
  }
  if (!wanted || wanted === currentRunId) return;
  currentRunId = wanted;
  state.liveRunId = wanted;
  tailRun(
    wanted,
    0,
    onLiveJournalEvent,
    undefined,
    tailRoot,
  );
}

function onLiveJournalEvent(ev) {
  applyJournalEvent(ev);
  // 334（§4.10）：终态事实到达＝本轮已收敛，撤销 30s 取消兜底。
  if (isTerminalEvent(ev)) clearCancelFallback();
  // 318 P1-B4：外部 busy 运行的终态事实到达 → 状态栏/停止键复位。
  if (state.serverBusy && isTerminalEvent(ev)) {
    state.serverBusy = false;
    currentRunSessionId = null;
    vm.setRunning(false);
    vm.addSystemMessage('（服务端运行已到达终态：状态复位为空闲）', false);
    renderAll();
    return;
  }
  scheduleRender();
}

async function startTail() {
  try {
    await refreshExplorerData();
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
  state.resumeTarget = null;
  liveMode = false;
  state.content = { items: [], currentModelIndex: null, droppedItems: 0 };
  state.marker = [];
  state.turnCounter = 0;
  // 0bv ②：本会话（acpSessionId 的 s8）的 run 按归属根回放——切区后旧会话
  // 的历史仍可读（跨工作区投影按归属根解析）。
  const lm = /^(?:RUN-CLI-|ARC-|RUN-)([0-9a-f]{8})/.exec(runId);
  const replayRoot =
    acpSessionId &&
    lm &&
    lm[1] === acpSessionId.slice(0, 8) &&
    state.sessionRoot &&
    pathNorm(state.sessionRoot) !== pathNorm(state.workspaceCwd)
      ? state.sessionRoot
      : undefined;
  let sawTerminal = false;
  try {
    await replayRun(runId, (ev) => {
      if (isTerminalEvent(ev)) sawTerminal = true;
      applyJournalEvent(ev);
    }, undefined, replayRoot);
  } catch (e) {
    vm.addSystemMessage(`（运行 ${runId} 读取失败：${e.message}）`, true);
  }
  // 329（2026-10-21）：重放后按 items 重建标记栏下标（单 run 回放同样适用）。
  vm.rebuildMarkers();
  // 318 P1-B4：无终态的旧 run 是历史未完成，绝不冒充「运行中」。
  if (!sawTerminal) {
    vm.addSystemMessage(`（运行 ${runId} 无终态事件——历史未完成；不代表当前运行中）`, true);
  }
  vm.addSystemMessage(`（历史回放 ${runId} —— 只读；点「后退」返回实时）`, false);
  renderAll();
}

/* 回到实时（workspace://live）：清空回放残留并整体重灌活动 run。 */
async function returnToLive() {
  closeTail();
  currentRunId = null;
  liveMode = true;
  state.resumeTarget = null;
  rewindSessionId = null;
  state.content = { items: [], currentModelIndex: null, droppedItems: 0 };
  state.marker = [];
  state.turnCounter = 0;
  vm.syncToolbar();
  renderAll();
  await startTail();
}

/* 329（2026-10-21）：撤回水位重放过滤（/api/recall 的 recall_log）。
 * 服务端 /rewind 只截断侧车；journal 历史不动。合并重放若按全量 journal
 * 铺开，被撤回的旧 tail 会复活。因此有 recall_log 时才缓冲事件，按撤回
 * 时刻把「第 ordinal 个用户消息事件」及其后内容裁掉；无水位仍走逐 run
 * 流式重放（性能不变）。口径与服务端一致：ordinal 数用户消息事件，
 * prompt_submitted 与 supplement_delivered 都算。 */
const RECALL_FILTER_MAX_EVENTS = 200000;

const isUserMessageEvent = (ev) =>
  ev?.event_type === 'prompt_submitted' || ev?.event_type === 'supplement_delivered';

/* 按 recalled_at 升序应用水位；同一时刻多条 recall 按日志顺序依次应用。
 * 事件缺/坏 timestamp 视为「早于 recall」：不触发比较，随所在前段保留/
 * 被裁；只 console.debug 记一次，不刷屏。
 * 334（§4.11）：journal 有窗口上限（200 run/64 MiB），重放窗口起点可能晚
 * 于撤回时刻——此时 ordinal 会把窗口内更晚的用户事件当目标（跨撤回点
 * 错裁）。检测到该缺口就整体跳过该水位并显式提示，不假装投影完整。 */
function filterEventsByRecall(events, recallLog, windowStartMs) {
  const windowStart = Number(windowStartMs);
  let gapWarned = false;
  const recalls = (recallLog || [])
    .map((r, order) => ({ at: Number(r?.recalled_at), ordinal: Number(r?.ordinal), order }))
    .filter((r) => Number.isFinite(r.at) && Number.isInteger(r.ordinal) && r.ordinal > 0)
    .filter((r) => {
      if (Number.isFinite(windowStart) && windowStart > 0 && r.at * 1000 < windowStart) {
        if (!gapWarned) {
          gapWarned = true;
          console.debug(
            '[rewind] journal 窗口起点晚于撤回时刻：跳过该水位的整体过滤（撤回前段不在窗口内）',
            { window_start_ms: windowStart, recalled_at: r.at },
          );
          vm.addSystemMessage(
            '（journal 窗口缺撤回前段，已按原样重放，撤回投影可能不完整）',
            true,
          );
        }
        return false;
      }
      return true;
    })
    .sort((a, b) => a.at - b.at || a.order - b.order);
  const kept = [];
  let cursor = 0;
  let badTimestampLogged = false;
  let ordinalSkippedWarned = false;
  const applyRecall = (recall) => {
    let seen = 0;
    for (let i = 0; i < kept.length; i++) {
      if (!isUserMessageEvent(kept[i])) continue;
      seen++;
      if (seen === recall.ordinal) {
        kept.length = i;
        return;
      }
    }
    // 334（§4.11）：数不到就跳过该水位（绝不静默/错裁别的 run），并留一条
    // 可见告警——重放窗口可能已不含该 ordinal 的前段。
    console.debug(
      `[rewind] 撤回水位 ordinal ${recall.ordinal} 在重放窗口事件中数不到对应用户消息，跳过该水位`,
    );
    if (!ordinalSkippedWarned) {
      ordinalSkippedWarned = true;
      vm.addSystemMessage(
        `（撤回水位 ordinal ${recall.ordinal} 在当前重放窗口内数不到对应用户消息——已跳过该水位，未做过滤）`,
        true,
      );
    }
  };
  for (const ev of events) {
    const ts = Date.parse(ev?.timestamp);
    if (!Number.isFinite(ts) && !badTimestampLogged) {
      badTimestampLogged = true;
      console.debug('[rewind] 事件缺/坏 timestamp，按早于撤回处理', ev?.event_id ?? ev?.event_type ?? '');
    }
    while (cursor < recalls.length && Number.isFinite(ts) && ts >= recalls[cursor].at * 1000) {
      applyRecall(recalls[cursor]);
      cursor++;
    }
    kept.push(ev);
  }
  // 撤回后没有新事件的会话：上面的循环不会触发，收尾仍须按序应用剩余
  // 水位——否则「只撤回、未重发」的会话重放时仍会带回被撤回的旧 tail。
  // 多条水位继续按时间序级联（后一条的 ordinal 以前一条裁完后的会话为准）。
  while (cursor < recalls.length) {
    applyRecall(recalls[cursor]);
    cursor++;
  }
  return kept;
}

/* 329（2026-10-21）：合并重放入口。无 recall_log 时保持逐 run 流式重放
 * （不改变性能）；有水位时缓冲全部事件，过滤后逐事件重放。事件数超过
 * 上限则放弃过滤：已缓冲前缀原样重放、其余恢复流式，并如实提示。返回
 * 无终态 run 数（按 journal 原始事件统计，撤回裁掉终态不算「未完成」）。 */
async function replayConversationMerged(s8, ordered, recallLog, windowStartMs) {
  let unfinished = 0;
  if (!recallLog.length) {
    for (const run of ordered) {
      let sawTerminal = false;
      await replayRun(run.run_id, (ev) => {
        if (isTerminalEvent(ev)) sawTerminal = true;
        applyJournalEvent(ev);
      });
      if (!sawTerminal) unfinished++;
    }
    return unfinished;
  }
  const events = [];
  let overflow = false;
  try {
    for (const run of ordered) {
      let sawTerminal = false;
      await replayRun(run.run_id, (ev) => {
        if (isTerminalEvent(ev)) sawTerminal = true;
        if (overflow) {
          applyJournalEvent(ev);
          return;
        }
        events.push(ev);
        if (events.length > RECALL_FILTER_MAX_EVENTS) {
          overflow = true;
          for (const buffered of events) applyJournalEvent(buffered);
          events.length = 0;
          vm.addSystemMessage(
            `（会话 ${s8} 的 journal 事件超过 ${RECALL_FILTER_MAX_EVENTS} 条——已放弃按撤回水位过滤，按原样重放）`,
            true,
          );
        }
      });
      if (!sawTerminal) unfinished++;
    }
  } catch (e) {
    // 某个 run 读取失败：已缓冲的前缀仍须落地（与旧流式路径同样「不留
    // 空白」），过滤后重放已拿到的部分，错误由调用方提示。
    if (!overflow) {
      for (const ev of filterEventsByRecall(events, recallLog, windowStartMs)) applyJournalEvent(ev);
    }
    throw e;
  }
  if (!overflow) {
    for (const ev of filterEventsByRecall(events, recallLog, windowStartMs)) applyJournalEvent(ev);
  }
  return unfinished;
}

/* 会话合并视图（走查反馈六 2026-09-25）：同一会话的全部运行按时间序
 * 连续重放进同一内容区——一整个长窗口，不逐运行割裂。
 * 318 P1-B2（2026-10-19）：打开旧会话＝**只读回看**，绝不自动
 * session/load 改变活跃绑定；只有工具栏「恢复并继续」才显式恢复。 */
async function openConversation(s8) {
  if (state.loadingSession) {
    flash('会话恢复中——加载完成前不能切换视图');
    return;
  }
  // 318 P1-B1/P2：s8 来自 URI，进 RegExp 前必须转义（`.`/`(` 等元字符
  // 会让构造抛错或跨会话误配；会话 id 只应是 8 hex，防御仍须在）。
  const re = new RegExp(`^(?:RUN-CLI-|ARC-|RUN-)${escapeRegExp(s8)}`);
  const ordered = state.runs.filter((r) => re.test(r.run_id)).slice().reverse(); // 旧→新
  closeTail();
  currentRunId = null;
  state.liveRunId = null;
  liveMode = false;
  state.content = { items: [], currentModelIndex: null, droppedItems: 0 };
  state.marker = [];
  state.turnCounter = 0;
  state.resumeTarget = null;
  const bound = boundSession8();
  const summary = state.conversations.find((c) => c.session8 === s8);
  if (s8 === bound) {
    // 当前绑定会话的历史面：不是只读回看；继续输入会先回实时再续接。
    vm.addSystemMessage(`（会话 ${s8} 历史回放——当前活跃会话；继续输入将回到实时并续接）`, false);
  } else if (summary?.session_id) {
    state.resumeTarget = { s8, sessionId: summary.session_id, root: summary.root };
    vm.addSystemMessage(
      `（会话 ${s8} —— 只读回看；点工具栏「恢复并继续」显式恢复后才可续接）`,
      false,
    );
    if (state.running) {
      vm.addSystemMessage('（当前有运行在进行；结束后才能恢复其他会话）', false);
    }
  } else {
    vm.addSystemMessage(`（会话 ${s8} 缺少完整 ACP 标识，只读打开，不能续接）`, true);
  }
  vm.syncToolbar();
  // 329（2026-10-21）：撤回水位——端点不可用（旧载体/后端未起/跨根）时按
  // 「无水位」继续，绝不让打开会话失败。根取会话归属根（跨工作区回看）。
  let recallLog = [];
  try {
    const recallRoot = s8 === boundSession8() ? state.sessionRoot : summary?.root;
    const recall = await fetchRecall(s8, recallRoot);
    recallLog = Array.isArray(recall?.recall_log) ? recall.recall_log : [];
  } catch (e) {
    console.debug(`[rewind] 撤回水位不可用（${s8}）：${e.message}`);
  }
  let unfinished = 0;
  try {
    // 334（§4.11）：把重放窗口起点（最早一条 run 的 mtime）交给水位过滤，
    // 用于识别「窗口不含撤回前段」。
    unfinished = await replayConversationMerged(s8, ordered, recallLog, ordered[0]?.modified_ms);
  } catch (e) {
    // 中途失败也要给出可见答复，不能留空白（错误路径修复 2026-09-25）。
    vm.addSystemMessage(`（会话 ${s8} 打开失败：${e.message}）`, true);
  }
  // 329（2026-10-21）：重放后按当前 items 重建标记栏——撤回过滤会裁掉旧
  // 事件，事件期间记下的数组下标不得沿用到重放后的内容面。
  vm.rebuildMarkers();
  const unfinishedNote = unfinished
    ? `；其中 ${unfinished} 次运行无终态（历史未完成，不代表当前运行中）`
    : '';
  vm.addSystemMessage(
    ordered.length
      ? `（会话回放 ${s8} —— ${ordered.length} 次运行合并；只读；点「后退」返回实时${unfinishedNote}）`
      : `（${s8} 暂无运行记录；侧车只读）`,
    !!unfinished,
  );
  renderAll();
}

/* 318 P1-B2/P1-B3：显式「恢复并继续」。唯一会执行 session/load 并切换
 * acpSessionId/sessionId/sessionRoot 的入口；await 前后复核 load token、
 * navEpoch、运行态与目标视图，加载期间再次点击＝取消本次恢复。 */
async function resumeViewedConversation() {
  if (state.loadingSession) {
    cancelSessionLoad();
    return;
  }
  const s8 = viewedConversation8();
  if (!s8) {
    flash('当前视图不是会话回看；请先打开一个活跃会话');
    return;
  }
  if (state.running) {
    flash('当前有运行在进行，结束或取消后才能恢复其他会话');
    return;
  }
  const target = state.resumeTarget;
  if (!target || target.s8 !== s8 || !target.sessionId) {
    if (s8 === boundSession8()) {
      vm.navGo('workspace://live');
      await returnToLive();
      flash('已回到实时会话');
      return;
    }
    flash('该会话缺少完整 ACP 标识（或不是可恢复的会话回看），不能续接');
    return;
  }
  const token = ++sessionLoadToken;
  const epoch = state.navEpoch;
  state.loadingSession = true;
  pendingLoadSessionId = target.sessionId;
  vm.syncToolbar();
  renderAll();
  vm.addSystemMessage(`（正在恢复 ACP 会话 ${s8}……）`, false);
  renderAll();
  const stillCurrent = () =>
    token === sessionLoadToken &&
    epoch === state.navEpoch &&
    !state.running &&
    viewedConversation8() === s8;
  try {
    await acp.initialize();
    if (!stillCurrent()) {
      vm.addSystemMessage(`（会话 ${s8} 恢复已取消：绑定未切换）`, true);
      return;
    }
    const cwd = await resolveLoadCwd(target);
    if (!stillCurrent()) {
      vm.addSystemMessage(`（会话 ${s8} 恢复已取消：绑定未切换）`, true);
      return;
    }
    if (!isAbsolutePath(cwd)) {
      vm.addSystemMessage(
        `（会话 ${s8} 恢复失败：工作区路径不是绝对路径（${cwd || '空'}），session/load 拒绝执行）`,
        true,
      );
      return;
    }
    await acp.sessionLoad(target.sessionId, cwd);
    if (!stillCurrent()) {
      vm.addSystemMessage(`（会话 ${s8} 恢复结果已过期：绑定未切换）`, true);
      return;
    }
    // Host（2026-10-19）在 load 返回前经 session/update 回放整段历史。
    // 恢复成功后立即切回实时投影：先清掉 load 回放，否则随后 journal 回放
    // 会把同一段历史渲染第二遍（B2/B3 重复渲染回归）。
    clearTranscript();
    setBinding(target.sessionId, cwd);
    state.loadingSession = false;
    pendingLoadSessionId = null;
    state.resumeTarget = null;
    vm.navGo('workspace://live');
    await returnToLive();
    // 334（§4.11）：session/load 的「只回放正文」声明在 load 返回前到达，
    // 随后的 clearTranscript/returnToLive 会清掉历史投影——恢复成功后补回，
    // 别让用户误以为工具历史丢失。
    if (lastReplayNotice && lastReplayNotice.sessionId === target.sessionId) {
      addSessionReplayNotice(lastReplayNotice);
      lastReplayNotice = null;
    }
    vm.addSystemMessage(`（已恢复 ACP 会话 ${s8}；下一条输入将续接此会话）`, false);
    flash(`已恢复会话 ${s8}`);
  } catch (e) {
    vm.addSystemMessage(`（会话 ${s8} 恢复失败，仍为只读回看：${e.message}）`, true);
  } finally {
    if (token === sessionLoadToken) {
      state.loadingSession = false;
      pendingLoadSessionId = null;
    }
    vm.syncToolbar();
    renderAll();
  }
}

function cancelSessionLoad() {
  if (!state.loadingSession) return;
  // 代次自增＝作废在途 load；返回时不写绑定。
  sessionLoadToken += 1;
  state.loadingSession = false;
  pendingLoadSessionId = null;
  vm.syncToolbar();
  vm.addSystemMessage('（已取消会话恢复；绑定未切换，仍为只读回看）', false);
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
  state.resumeTarget = null;
  liveMode = false;
  state.content = { items: [], currentModelIndex: null, droppedItems: 0 };
  state.marker = [];
  state.turnCounter = 0;
  vm.syncToolbar();
  let d;
  try {
    d = await fetchArchive(s8);
  } catch (e) {
    vm.addSystemMessage(`（归档 ${s8} 读取失败：${e.message}）`, true);
    renderAll();
    return;
  }
  // 2026-10-21 恢复修复：归档详情携带原会话 id 时留下显式恢复目标——
  // 「恢复并继续」由 host 自动回档（只补缺、不覆盖），不必先手动回档。
  if (d.session_id && s8 !== boundSession8() && !state.running) {
    state.resumeTarget = { s8, sessionId: d.session_id, root: state.workspaceCwd };
  }
  vm.syncToolbar();
  vm.addSystemMessage(archiveFactsLine(d), false);
  renderArchiveTranscript(d);
  // 归档转写的「最终输出」锚（批四 2026-09-25：标记栏在归档浏览同样可用
  // ——用户锚在转写装填时逐条产生，输出锚按实时视图同规则只定正式输出）。
  vm.anchorFinalOutput();
  const resumeHint = state.resumeTarget
    ? '；点「恢复并继续」回档并续接'
    : state.running
      ? '（当前有运行在进行；结束后才能恢复）'
      : '（缺少完整会话标识，不能续接）';
  vm.addSystemMessage(
    `（归档浏览 ${s8} —— 只读${d.transcript_truncated ? `；转写超上限已截断（${d.messages_shown}/${d.messages_total} 条）` : ''}${resumeHint}；点「后退」返回）`,
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
// 0dm S2（2026-10-21）：交付状态按需查询（只读投影，不触发模型）。
ui.openDeliveryStatus = () => dialogs.openDeliveryStatus();
// 333（2026-10-21，S9）：上下文占用/最近压缩只读读数（/api/context）。
ui.openContext = () => dialogs.openContext();
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
/* 0dm（2026-10-21）：工具栏「反例自查」——PULL 入口。运行中经中途补充
 * 队列投递三问；空闲/只读视图不凭空起一轮。 */
ui.runSelfCheck = async () => {
  if (!state.running) {
    flash('反例自查只在任务运行中可用；先发起任务');
    return;
  }
  if (state.loadingSession || promptGateReason()) {
    flash('当前视图不可补充；请回到实时会话');
    return;
  }
  await queueSupplement(
    '请对当前结论执行反例自查（三问），然后按需继续或输出 [RUN_END]。',
  );
};
ui.flashStatus = () => flash(`当前模型: ${state.modelLabel}`);
// 333（S12）：boot 事实落地（工作区/版本偏斜）——冒烟可直接注入载荷。
ui.applyBootFacts = applyBootFacts;

/* 0di：撤回面动作（dialogs.openRewind 经 actions 调用）。
 * 318 P1-B1：不再用 ensureSession() 静默绑定另一个会话——只对当前绑定
 * 会话显式传 session_id（服务端按 id 校验）；只读/跨会话视图直接拒绝。
 * 预览与撤回还必须命中同一个显式目标，绑定/视图变化后旧预览作废。 */
ui.rewindPreview = async () => {
  const reason = rewindGateReason();
  if (reason) throw new Error(reason);
  const target = acpSessionId;
  const preview = await acp.sessionRecallPreview(target);
  if (acpSessionId !== target || rewindGateReason()) {
    throw new Error('撤回目标已变化（会话绑定或视图已切换），请重新打开 /rewind');
  }
  rewindSessionId = target;
  return preview;
};
ui.rewindRecall = async (ordinal) => {
  const reason = rewindGateReason();
  if (reason) throw new Error(reason);
  const target = acpSessionId;
  if (!target || rewindSessionId !== target) {
    throw new Error('撤回目标已变化；请重新打开 /rewind 再确认');
  }
  try {
    const r = await acp.sessionRecall(target, ordinal);
    const dropped = r.dropped_supplements > 0 ? `；${r.dropped_supplements} 条未投递补充一并丢弃` : '';
    // 329（2026-10-21）：服务端已截断侧车，本地投影必须同步截断——旧实现
    // 只加一条系统行，被撤回的卡与其 marker 留在窗内，重发还会被去重吞。
    // ordinal 是「可撤回用户消息」序号（1-based），本地按用户卡计数；内存
    // 裁剪掉的用户卡也计入 ordinal 轴，避免把目标数到错误的卡。
    const droppedUsers = state.content.droppedUserCards || 0;
    const localOrdinal = Number(ordinal) - droppedUsers;
    let seen = 0;
    let cut = -1;
    if (localOrdinal >= 1) {
      for (let i = 0; i < state.content.items.length; i++) {
        const it = state.content.items[i];
        if (it.kind !== 'message' || it.role !== '用户') continue;
        seen++;
        if (seen === localOrdinal) {
          cut = i;
          break;
        }
      }
    }
    if (cut < 0) {
      // 只读回放/本地已裁掉/序号漂移：不瞎截，保留服务端结果并如实说明。
      vm.addSystemMessage(
        `（服务端已撤回 ${r.removed_messages ?? '?'} 条消息${dropped}，但本地视图不含第 ${ordinal} 条用户消息——未做本地截断；请重新打开会话查看）`,
        true,
      );
      renderAll();
      return r;
    }
    state.content.items.splice(cut);
    // 契约：先过滤越界锚点，再按 items 重建（保留输入锚/最终输出锚/搜索命中）。
    state.marker = state.marker.filter((m) => m.itemIndex < cut);
    let lastUserTurn = 0;
    for (let i = state.content.items.length - 1; i >= 0; i--) {
      const it = state.content.items[i];
      if (it.kind === 'message' && it.role === '用户') {
        lastUserTurn = it.turn || 0;
        break;
      }
    }
    state.turnCounter = lastUserTurn;
    state.content.currentModelIndex = null;
    vm.rebuildMarkers();
    // P2（318）：撤回不能只有 4 秒 flash——留一条持久系统行作为可回查的事实。
    vm.addSystemMessage(`（已撤回 ${r.removed_messages ?? '?'} 条消息${dropped}；原文已回输入框）`, false);
    renderAll();
    return r;
  } finally {
    rewindSessionId = null;
  }
};
ui.fillInput = (text) => {
  chatInput.value = text;
  chatInput.focus();
};
// 318 P1-B2/B3：唯一显式恢复入口（工具栏按钮）。加载中再次调用＝取消。
ui.resumeConversation = resumeViewedConversation;

/* 会话维护三动作（执行都在桥 spawn 的 agent 子命令，前端零执行事实）。
 * 批六（2026-09-25 第七用户令）：归档/回档/删除改为组级功能键——先在
 * 组内选中会话再点功能键，动作完成后清除选中并刷新探索器。 */
const isLiveSession = (s8) => {
  return !!(state.running && state.sessionId?.slice(0, 8) === s8);
};

ui.archiveSession = async (s8) => {
  if (!s8) return;
  if (state.loadingSession) {
    flash('会话恢复中——加载完成前不能归档');
    return;
  }
  if (isLiveSession(s8)) {
    flash('该会话正在运行，暂不能归档');
    return;
  }
  if (!confirm(`归档会话 ${s8}？（打包为 .gsa/archives 归档包＋ARC 审计 journal；侧车保留至保留期清扫）`)) return;
  try {
    const r = await postArchive(s8);
    flash(r.message || `已归档 ${s8}`);
  } catch (e) {
    flash(`归档失败：${e.message}`);
  }
  state.selected = null;
  await refreshExplorerData();
};

ui.unarchiveSession = async (s8) => {
  if (!s8) return;
  if (state.loadingSession) {
    flash('会话恢复中——加载完成前不能回档');
    return;
  }
  if (!confirm(`回档会话 ${s8}？（移除归档包，会话回到「活跃会话」组；运行与侧车数据全部保留）`)) return;
  try {
    const r = await unarchiveSession(s8);
    flash(r.message || `已回档 ${s8}`);
  } catch (e) {
    flash(`回档失败：${e.message}`);
  }
  state.selected = null;
  await refreshExplorerData();
};

ui.deleteSession = async (s8) => {
  if (!s8) return;
  if (state.loadingSession) {
    flash('会话恢复中——加载完成前不能删除');
    return;
  }
  if (isLiveSession(s8)) {
    flash('该会话正在运行，暂不能删除');
    return;
  }
  if (!confirm(`彻底删除会话 ${s8}？（移除归档包＋对话侧车＋全部运行 journal——不可恢复！）`)) return;
  let confirmed = false;
  try {
    const r = await deleteSession(s8);
    confirmed = true;
    flash(r.message || `已删除 ${s8}`);
  } catch (e) {
    flash(`删除失败：${e.message}`);
  }
  // 318 P2：已加载（绑定）会话被删除后必须解绑并停 tail——否则下一条
  // prompt 会用旧 agent 会话把 sidecar 写回，「彻底删除」失效。
  // 仅删除当前正在只读浏览的会话时，只退回实时，不动仍在用的绑定。
  const wasBound = confirmed && state.boundSession8 === s8;
  const wasViewed = viewedConversation8() === s8;
  if (wasBound) {
    detachBinding();
    vm.addSystemMessage(`（会话 ${s8} 已删除：本地绑定已解绑；后续输入将创建/恢复新会话）`, false);
  }
  if (wasViewed) {
    liveMode = true;
    state.nav = { current: 'workspace://live', back: [], forward: [] };
    state.navEpoch++;
    state.resumeTarget = null;
    clearTranscript();
    vm.syncToolbar();
  }
  state.selected = null;
  await refreshExplorerData();
  renderAll();
};

/* 新建会话（用户令 2026-09-25「需要一个明确的新建会话按键」，工具栏
 * 居「后退」之前）：归档当前会话后由此开启下一段对话。桥为单 ACP 会话
 * 绑定，此前没有任何换会话入口——旧会话一经绑定终身跟随，归档流程无法
 * 收尾。ACP 0.10.4 无 session/close（服务端会话对象随进程结束），因此
 * 至少清本地绑定并停 tail（318 P2）；数据已留在 .gsa，探索器可随时回看；
 * 运行中拒绝换会话。 */
ui.newSession = async () => {
  if (state.loadingSession) {
    flash('会话恢复中——请等待完成或点「取消恢复」');
    return;
  }
  if (state.running) {
    flash('当前有运行在进行（Ctrl+Z 可取消）');
    return;
  }
  // 0dk S2（332）：换会话＝close(switch)——旧会话不再留在 agent 内存里空转
  // （有界表容量有限）。关闭失败不阻塞新建：至少解绑本地并如实提示。
  const oldSession = acpSessionId;
  if (oldSession) {
    try {
      await acp.sessionClose(oldSession, 'switch');
    } catch (e) {
      vm.addSystemMessage(
        `（旧会话关闭失败：${e.message}；已仅解绑本地，历史仍保留在 .gsa）`,
        true,
      );
    }
  }
  detachBinding();
  liveMode = true;
  // 导航历史属于旧会话的浏览足迹，一并归零——「后退」不得回到旧会话回放。
  state.nav = { current: 'workspace://live', back: [], forward: [] };
  state.navEpoch++;
  vm.syncToolbar();
  clearTranscript();
  // 运行态与状态栏一并复位——上一会话回放残留的「完成」不得带进新会话。
  vm.setRunning(false);
  renderAll();
  try {
    await ensureSession();
  } catch (e) {
    vm.addSystemMessage(`（新会话创建失败：${e.message}）`, true);
  }
  flash('已新建会话（旧会话可经探索器回看）');
};

/* 0dk S2（332）：关闭当前会话（工具栏「关闭会话」）。服务端先 cancel 在途
 * run（运行中按钮已禁用），再释放 lease、移出内存；侧车保留，历史可回看。
 * 成败都以服务端事实为准：already=true 表示此前已关闭。 */
ui.closeSession = async () => {
  if (state.loadingSession) {
    flash('会话恢复中——加载完成前不能关闭');
    return;
  }
  if (state.running) {
    flash('运行中不能关闭：先点「停止」或等本轮结束');
    return;
  }
  const id = acpSessionId;
  if (!id) {
    flash('当前没有绑定会话可关闭');
    return;
  }
  try {
    const out = await acp.sessionClose(id, 'user_close');
    // 通知可能先到并已解绑/清场；只在仍绑定同一会话时补做本地收尾。
    if (acpSessionId === id) {
      detachBinding();
      liveMode = true;
      state.nav = { current: 'workspace://live', back: [], forward: [] };
      state.navEpoch++;
      clearTranscript();
      vm.setRunning(false);
    }
    renderAll();
    flash(
      out?.already
        ? '该会话此前已关闭（历史保留在 .gsa）'
        : '会话已关闭（历史保留在 .gsa，可从探索器回看）',
    );
  } catch (e) {
    vm.addSystemMessage(
      `关闭没成功：${e.message}。→ 等当前操作结束后重试；若仍失败，关闭其他 orz 窗口后刷新`,
      true,
    );
    renderAll();
  }
};

ui.navAction = async (dir) => {
  if (state.loadingSession) {
    flash('会话恢复中——加载完成前不能切换视图');
    return;
  }
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

/* 工作区机械切换（0bv ②，B 形态：服务随启动而立；服务内点击「已信任
 * 工作区」行）。门禁：运行中禁切＝前端 state.running 主判（活跃 run 不吃
 * 切换）；桥侧信任门 fail-closed（未信任 403 原样回显），桥有 run 在跑时
 * 服务端 409（334 §5.5）。334（§4.5）：成功后 WS 必须重连到新根；原绑定
 * 会话留在旧工作区（可从探索器回看/「恢复并继续」）。 */
async function switchWorkspaceTo(path) {
  if (pathNorm(path) === pathNorm(state.workspaceCwd)) return; // 已在该工作区
  if (state.loadingSession) {
    flash('会话恢复中——加载完成前不能切换工作区');
    return;
  }
  if (state.running) {
    flash('运行中禁切：等本轮结束（或 Ctrl+Z 取消）后再切换工作区');
    return;
  }
  try {
    const b = await switchWorkspace(path);
    // 334（§4.5）：切换工作区后 WS 仍停在旧槽——必须重连到新根，否则后续
    // session/new 会在旧 child 里以新 cwd 建会话，刷新即失联。
    const hadBinding = !!acpSessionId;
    const oldRoot = state.sessionRoot;
    state.workspaceCwd = b.cwd || path;
    state.trustedWorkspaces = b.trusted_workspaces || state.trustedWorkspaces;
    state.recentWorkspaces = b.recent_workspaces || state.recentWorkspaces;
    const trustedNow = (state.trustedWorkspaces || []).some(
      (t) => pathNorm(t.path) === pathNorm(state.workspaceCwd),
    );
    // 334（§4.5）：切换后 WS 必须重连到新根（不论此前是否绑定——旧槽的
    // child 不能继续承接新工作区的新会话）。切换期间抑制 remembered load。
    let reconnectError = '';
    try {
      attachFallbackSuspended = true;
      await acp.reconnect(accessToken || acp.rememberedToken(), state.workspaceCwd, null);
    } catch (e) {
      reconnectError = e.message;
    }
    if (hadBinding) {
      detachBinding();
      liveMode = true;
      state.nav = { current: 'workspace://live', back: [], forward: [] };
      state.navEpoch++;
      clearTranscript();
      vm.setRunning(false);
    }
    attachFallbackSuspended = false;
    // 系统行必须落在清场之后，否则会被 clearTranscript 一并抹掉。
    if (!trustedNow) {
      vm.addSystemMessage(
        `（工作区 ${state.workspaceCwd} 尚未持久授信：本次会话内可随时切回；` +
          '要刷新后仍保留，请在该工作区发一条消息，信任提示里点确认。）',
        false,
      );
    }
    if (hadBinding) {
      vm.addSystemMessage(
        `（旧工作区会话（回看）：本对话归属 ${oldRoot || '旧工作区'}；工作区已切换到 ` +
          `${state.workspaceCwd}——本对话不迁移，留在原上下文；可从探索器回看或「恢复并继续」）`,
        false,
      );
    }
    if (reconnectError) {
      vm.addSystemMessage(
        `（工作区已切换，但桥接重连到新工作区失败：${reconnectError}；刷新本页可重连）`,
        true,
      );
    }
    await refreshExplorerData();
    flash(`已切换工作区：${state.workspaceCwd}`);
  } catch (e) {
    // 334（§5.5）：服务端 409（有 run 在跑）必须如实显示 message/hint。
    // 注：api.js 只上浮 error.message（不在本批改动文件内），hint 按服务端
    // 409 契约同文补齐，避免用户只看到一句「切换失败」。
    const message = e.message || '未知错误';
    // api.js 334 起把桥 JSON 的 hint/evidence 挂在 Error 上；优先用服务端
    // 原文，旧载体无字段时才按文案兜底。
    const hint =
      e.hint ||
      (/运行在进行|不能切换工作区|already in flight/i.test(message)
        ? '等本轮结束或点「停止」后再切换；切换会改变新会话的归属根。'
        : '');
    flash(`工作区切换失败：${message}${hint ? `（${hint}）` : ''}`);
    if (hint) vm.addSystemMessage(`[错误] ${message}（${hint}）`, true);
  }
  renderAll();
}

// 对象 URI 路由（AddressBar 语义，R-4）。v1 投影面：run://（单运行回放）、
// conversation://（会话合并视图，走查反馈六）、archive://（归档只读浏览，
// 0br S3 新增面）、workspace://live；其余 scheme 显式给出「无投影」答复，
// 不静默。
ui.openUri = async (uri) => {
  // 318 P1-B3：session/load 等待期间禁切会话/回 live（也禁开其它回放）——
  // 否则 await 返回时绑定可能已漂移，load 结果会覆盖用户的新视图。
  if (state.loadingSession) {
    flash('会话恢复中——加载完成前不能切换视图（可点「取消恢复」）');
    return;
  }
  if (uri.startsWith('run://')) {
    vm.navGo(uri);
    await openRunReplay(uri.slice('run://'.length));
  } else if (uri === 'workspace://live') {
    vm.navGo(uri);
    await returnToLive();
  } else if (uri.startsWith('workspace://switch/')) {
    // 0bv ②：工作区机械切换（不是内容导航——不进 nav 栈；运行中禁切回显）。
    await switchWorkspaceTo(decodeURIComponent(uri.slice('workspace://switch/'.length)));
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
  // 329（2026-10-21）：标记锚可能来自失效的旧投影（撤回截断/重放/内存裁剪
  // 之前）。越界必须显式提示，不静默失败、不读 undefined 崩溃。
  if (!Number.isInteger(itemIndex) || itemIndex < 0 || itemIndex >= state.content.items.length) {
    flash(
      `标记指向的条目已不在视图中（下标 ${itemIndex}，当前 ${state.content.items.length} 条）；请刷新或重新打开会话`,
    );
    return;
  }
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
    case '/rewind': {
      // 0di：撤回（编辑式重发）——对话框列会话用户消息，选择后截断并把
      // 原文回输入框。
      // 318 P1-B1：/rewind 与 submitPrompt 共用只读/跨会话门；拒绝时返回
      // false 让调用方保留草稿（清空必须发生在此门之后）。
      if (state.running) {
        flash('当前有运行在进行，结束或取消后才能撤回');
        return false;
      }
      const reason = rewindGateReason();
      if (reason) {
        flash(reason);
        return false;
      }
      dialogs.openRewind();
      break;
    }
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
  return true;
}

dialogs.bindDialogActions({
  ...actions,
  findInConversation: ui.findInConversation,
  // 334：权限取消/顶掉/关闭的四要素系统行（函数声明已提升）。
  onPermissionCancelled,
  flash,
});
keymap.bindKeyActions(actions);
keymap.install();

/* ── 输入行 ── */

const inputHistory = [];
let historyIndex = null;

function submitFromInput() {
  const text = chatInput.value.trim();
  if (!text) return;
  if (text.startsWith('/')) {
    if (runSlash(text)) chatInput.value = '';
    return;
  }
  // 318 P1-B1：只读门在清空输入之前——拒绝时草稿原样保留。
  const reason = promptGateReason();
  if (reason) {
    flash(reason);
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
    // 334（§5.9）：主动退出不触发页面内重连。
    pageExiting = true;
    clearTimeout(reconnectTimer);
    window.close();
    closeTail();
    flash('请关闭浏览器标签页（服务随 Ctrl+C 退出）');
  }
});

// 刷新/关闭守卫（审查处理批 B2 登记；318 P1-B7 强化；329 改口径）：
// 桥按 `ORZ_WEB_KEEPALIVE_SECS`（默认 1800s）保活 `orz --stdio` 子进程——
// 刷新/短时断线后重开页面会经 `_orz/bridge_attached` 重连到同一会话，排队
// 补充与后台任务随 child 存活；只有桥进程退出或超过保活窗口才回收。仍须
// 提示的真实边界：关掉页面超过保活窗口，运行中的任务会被回收。
const UNLOAD_WARNING =
  '刷新可重连续接（子进程默认保活 30 分钟）；关闭页面超过保活窗口会回收子进程，排队补充与后台任务将不恢复。';

function unloadRiskReasons() {
  const reasons = [];
  if (state.running) reasons.push('运行进行中');
  if (state.supplementQueueDepth > 0) reasons.push(`已排队 ${state.supplementQueueDepth} 条补充`);
  if (state.backgroundTasks > 0) reasons.push(`${state.backgroundTasks} 个后台任务`);
  return reasons;
}

window.addEventListener('beforeunload', (e) => {
  // 334（§5.9）：空闲刷新不应弹确认——只有真实的运行/队列/后台风险才拦。
  // （连接是否仍开着不再作为拦截条件：刷新/短断线可经保活 attach 续接。）
  if (unloadRiskReasons().length === 0) return;
  e.preventDefault();
  e.returnValue = UNLOAD_WARNING;
});

/* ── ACP 事件接线 ── */

acp.on('update', (sessionId, update) => {
  // 318 P1-B3：session/load 等待期间，目标会话的 update 不能因绑定尚未
  // 切换而被过滤器丢弃（恢复回放/首段输出会丢字）。只接受「当前绑定」或
  // 「本次显式恢复目标」；加载取消/过期后 pendingLoadSessionId 清空即恢复
  // 原来的严格跨会话隔离。
  const accepted =
    sessionId === acpSessionId ||
    (state.loadingSession && sessionId === pendingLoadSessionId);
  if (!accepted) return;
  const viewed = viewedConversation8();
  if (!liveMode && viewed !== sessionId.slice(0, 8)) return;
  applyAcpUpdate(sessionId, update);
  scheduleRender();
});
acp.on('permission', (request) => dialogs.showPermission(request));

/* 334（§4.6）：权限弹窗被顶掉/关闭/Esc 时已如实回 cancelled，main 补一条
 * 可回查的四要素系统行；用户主动点「取消」也如实说明但不夸张（不标红）。 */
function onPermissionCancelled(info = {}) {
  const reason = info.reason || 'cancelled';
  let hint;
  if (reason === 'displaced') {
    hint = '上一条审批已被新的审批请求顶掉（已回传 cancelled）；请确认正在审批的是最新工具';
  } else if (reason === 'user_cancel') {
    hint = '你主动取消了本次审批（已回传 cancelled）；模型会按拒绝继续或终止该工具';
  } else {
    hint = '审批弹窗已关闭（已回传 cancelled）；若仍需该工具，请让模型重试';
  }
  vm.addSystemMessage(
    `[权限] ${describeError({
      code: 'permission_denied',
      class: 'permission',
      hint,
      evidence: 'ACP session/request_permission（cancelled）',
    })}`,
    reason !== 'user_cancel',
  );
  renderAll();
}

/* 334（§5.9）：页面内重连——1s/2s/4s，最多 3 次；成功后清零并 flash。
 * 主动退出（btnQuit）或未拿到令牌时不重连。 */
function scheduleReconnect() {
  if (pageExiting) return;
  const token = accessToken || acp.rememberedToken();
  if (!token) return;
  if (reconnectTimer != null) return;
  if (reconnectAttempt >= RECONNECT_DELAYS.length) {
    vm.addSystemMessage(
      '（自动重连 3 次均失败：请刷新页面重试；若仍失败请重启 orz web（证据：ws /ws/acp））',
      true,
    );
    renderAll();
    return;
  }
  const delay = RECONNECT_DELAYS[reconnectAttempt];
  reconnectAttempt += 1;
  reconnectTimer = setTimeout(async () => {
    reconnectTimer = null;
    try {
      await acp.reconnect(token, state.workspaceCwd, state.sessionId);
      reconnectAttempt = 0;
      flash('已重连');
    } catch {
      scheduleReconnect();
    }
  }, delay);
}

acp.on('closed', (facts) => {
  flash('桥接连接已断开，正在尝试重连…');
  if (state.loadingSession) {
    sessionLoadToken += 1;
    state.loadingSession = false;
    pendingLoadSessionId = null;
  }
  // 334（§4.1）：不在这里把运行态直接清成空闲（服务端可能仍在跑，重连后
  // 由 attach 前置帧/清单校正）；但必须把四要素断开事实说清楚。标记
  // serverBusy 还可阻止在途 prompt 的 reject 路径把状态误清成空闲。
  if (state.running) state.serverBusy = true;
  vm.addSystemMessage(
    `[错误] ${describeError(facts || { code: 'connection_closed' }, 'connection_closed')}`,
    true,
  );
  scheduleReconnect();
  renderStatus();
  renderToolbar();
  renderAll();
});
acp.on('open', () => {
  reconnectAttempt = 0;
  flash('桥接已连接');
  // 334：偏斜横幅必须压过连接 flash（否则 boot 后 4 s 内消失）。
  renderVersionSkewBanner();
});

/* 329（2026-10-21）：会话保活——刷新/断线重连后的 attach 前置帧。
 * 桥在 new WS 上先发 `_orz/bridge_attached`（session_id/reused/
 * prompt_in_flight/cwd/session_root），本页据此绑定仍在跑的会话：不再
 * session/new，也不再要求用户先点「恢复并继续」。 */
acp.on('attached', (params) => {
  void onBridgeAttached(params);
});
acp.on('notice', (kind, params) => {
  if (kind === 'child_exited') {
    vm.addSystemMessage(
      '（桥接的 agent 子进程已退出：本轮若未收尾则已终止；文本上下文可经「恢复并继续」续接）',
      true,
    );
    vm.setRunning(false);
  } else if (kind === 'backlog_truncated') {
    vm.addSystemMessage(
      '（断线期间输出超过桥缓存上限，部分流式帧已丢弃；完整事实以 journal 为准）',
      true,
    );
  } else if (kind === 'ingress_truncated') {
    // 334（§4.12）：attached 期有界接收缓冲溢出——当前页直接丢帧。
    vm.addSystemMessage(
      `（本页连接接收缓冲已满（${params?.dropped || 'attached_ingress_full'}）：部分 agent 输出帧已丢弃；完整事实以 journal 为准）`,
      true,
    );
  }
  renderAll();
});

/* 334（§4.11）：load 只回放正文——工具调用/系统消息不回放，必须显式说明
 * 「工具事实在 journal」，避免用户以为历史缺失。 */
function addSessionReplayNotice(params) {
  const skipped = Number(params?.tool_calls_skipped) || 0;
  vm.addSystemMessage(
    `（load 只回放用户/助手正文；工具调用与系统消息不回放，工具事实见运行历史 journal；跳过工具调用 ${skipped} 条）`,
    false,
  );
}

acp.on('replay', (params) => {
  lastReplayNotice = {
    sessionId: params?.sessionId || null,
    tool_calls_skipped: Number(params?.tool_calls_skipped) || 0,
  };
  addSessionReplayNotice(lastReplayNotice);
  renderAll();
});

/* 0dk S2（332）/334：服务端会话状态转移（`_orz/session_state`）。
 * closing → 禁用发送；closed → 解绑（含非本页发起的驱逐）；deleted →
 * 已删除并解绑。 */
acp.on('state', (params) => {
  const sessionId = params?.sessionId;
  const next = params?.state;
  if (!sessionId) return;
  if (next === 'closing') {
    if (acpSessionId === sessionId) {
      setClosing(sessionId);
      vm.addSystemMessage(
        `（会话 ${sessionId.slice(0, 8)} 正在关闭：发送已禁用，等待关闭完成）`,
        false,
      );
      renderAll();
    }
    return;
  }
  if (next !== 'closed' && next !== 'deleted') return;
  const isBound = acpSessionId === sessionId;
  const s8 = sessionId.slice(0, 8);
  if (next === 'deleted') {
    // 334（§4.9）：删除是终局事实——即便不是本页发起，也要解绑并如实提示。
    if (isBound) detachBinding();
    const wasViewed = viewedConversation8() === s8;
    if (wasViewed) {
      liveMode = true;
      state.nav = { current: 'workspace://live', back: [], forward: [] };
      state.navEpoch++;
      clearTranscript();
      vm.setRunning(false);
    }
    if (isBound || wasViewed) {
      vm.addSystemMessage(
        `（会话 ${s8} 已删除：本地绑定已解除；数据已从磁盘移除，不可恢复）`,
        true,
      );
      renderAll();
    }
    return;
  }
  if (!isBound) return;
  const reason = params?.reason;
  detachBinding();
  liveMode = true;
  state.nav = { current: 'workspace://live', back: [], forward: [] };
  state.navEpoch++;
  clearTranscript();
  vm.setRunning(false);
  vm.addSystemMessage(
    `（会话 ${s8} 已关闭${
      reason === 'evict' ? '（容量驱逐：内存会话上限 32）' : ''
    }；历史保留在 .gsa，可从探索器回看或「恢复并继续」）`,
    false,
  );
  renderAll();
});

async function onBridgeAttached(params) {
  let sid = params?.session_id;
  // 334（§4.5）：绑定根以会话归属根为准（load 参数），cwd 只是 child 启动
  // 目录；两者都没有时才回退当前工作区。
  let bindRoot = params?.session_root || params?.cwd || state.workspaceCwd;
  // 333：保活窗口到期回收的显式事实——刷新/重开页面后必须先说「上一会话
  // 没恢复」，不能让用户把新 child 误当重连成功（排队补充/后台任务已失效）。
  // 334（§5.6）：dead child 与 grace 到期分开表述，别把崩溃说成「等太久」。
  if (params?.keepalive_expired) {
    const old = params.expired_session_id
      ? params.expired_session_id.slice(0, 8)
      : '（未知）';
    const reason = params.expired_reason;
    const why =
      reason === 'child_exited'
        ? `（子进程已退出：上一会话 ${old} 随 agent 子进程退出而未恢复；排队补充与后台任务已失效。下一条输入将新建会话；历史可从探索器回看，或点「恢复并继续」显式恢复。）`
        : `（保活窗口已过期：上一会话 ${old} 已随子进程回收，未自动恢复；排队补充与后台任务已失效。下一条输入将新建会话；历史可从探索器回看，或点「恢复并继续」显式恢复。）`;
    vm.addSystemMessage(why, true);
    // 同页重连到已死 child 时本地仍持有旧绑定——必须解绑，否则下一条输入
    // 会拿死会话 id 去撞 not_found（刷新页本就没有绑定，此分支自然跳过）。
    if (!sid && params.expired_session_id && params.expired_session_id === acpSessionId) {
      detachBinding();
      liveMode = true;
      state.nav = { current: 'workspace://live', back: [], forward: [] };
      state.navEpoch++;
      clearTranscript();
      vm.setRunning(false);
    }
    renderAll();
  }
  // 334（§4.5）：桥重启后没有 attach 到旧 child——sessionStorage 里的绑定
  // 还在时显式 initialize + session/load 续接；失败如实提示，不假装恢复。
  // 保活/子进程已过期时不尝试（旧会话确定已不在，load 只会造新的等待）。
  if (!sid && !params?.keepalive_expired && !attachFallbackSuspended) {
    const rememberedId = sessionStoreGet(SESSION_ID_KEY) || rememberedSessionId;
    const rememberedRoot =
      sessionStoreGet(SESSION_ROOT_KEY) || rememberedSessionRoot || state.workspaceCwd;
    if (rememberedId) {
      vm.addSystemMessage(
        `（桥未 attach 到旧会话：正在从本地记忆恢复会话 ${rememberedId.slice(0, 8)}……）`,
        false,
      );
      renderAll();
      // 334：load 的 session/update 回放必须被 on('update') 接收——用它
      // 已有的 loadingSession/pendingLoadSessionId 通道，否则首屏历史被
      // 过滤器丢掉，只剩 journal 尾部。
      pendingLoadSessionId = rememberedId;
      state.loadingSession = true;
      try {
        await acp.initialize();
        await acp.sessionLoad(rememberedId, rememberedRoot || state.workspaceCwd);
        sid = rememberedId;
        bindRoot = rememberedRoot || bindRoot;
        // 与「恢复并继续」同口径：load 回放先落屏，随后清掉并交给 journal
        // 重放，避免同一段历史渲染两遍（B2/B3 重复渲染回归）。
        clearTranscript();
      } catch (e) {
        vm.addSystemMessage(
          `（会话 ${rememberedId.slice(0, 8)} 自动恢复失败：${e.message}；下一条输入将新建会话，历史可从探索器回看）`,
          true,
        );
        renderAll();
        return;
      } finally {
        pendingLoadSessionId = null;
        state.loadingSession = false;
      }
    }
  }
  if (!sid) return;
  const same = sid === acpSessionId;
  // 顺序修正（334 §4.1）：先绑定，再 resync（此时 running 未被强行置 true），
  // 最后才按服务端 prompt_in_flight 决定运行态。
  setBinding(sid, bindRoot);
  liveMode = true;
  currentRunId = null;
  await resyncBusyState({ fromAttach: true });
  if (params?.prompt_in_flight === true) {
    // 服务端事实：本轮仍在跑。状态栏/停止键与真实运行同源（318 P1-B4）。
    state.serverBusy = true;
    currentRunSessionId = sid;
    vm.setRunning(true);
  } else if (params?.prompt_in_flight === false) {
    // 334：preamble 明确空闲——清掉可能残留的假运行态。
    state.serverBusy = false;
    currentRunSessionId = null;
    vm.setRunning(false);
  }
  if (!same) {
    const pendingPermissions = Number(params?.pending_permission_count) || 0;
    vm.addSystemMessage(
      `（已重连到${params?.prompt_in_flight ? '仍在运行的' : '空闲'}会话 ${sid.slice(0, 8)}${
        params?.reused ? '（子进程保活）' : ''
      }${params?.prompt_in_flight ? '；本轮仍在运行，输出继续' : '；当前空闲'}${
        pendingPermissions > 0 ? `；${pendingPermissions} 条未应答审批已重放` : ''
      }）`,
      false,
    );
  }
  // 334：load 回放口径声明在 load 返回前到达，自动恢复路径的清屏会把它
  // 一起清掉——此处补回，确保用户知道工具面未回放。
  if (lastReplayNotice && lastReplayNotice.sessionId === sid) {
    addSessionReplayNotice(lastReplayNotice);
    lastReplayNotice = null;
  }
  // 从 journal 头补齐该 run（断线期间的事实不丢）；在跑时随后由
  // session/update 续流。置 null 强制 syncLiveTail 重挂。
  await startTail();
  renderAll();
}

/* ── 启动 ── */

/* 334（§3.1）：版本偏斜横幅是**持久**事实，不能被连接成功的 4 s flash
 * 覆盖（333 落码后真机 S2 实测：boot 横幅先出，WS open 的「桥接已连接」
 * 把它顶掉）。boot 与每次 open 都重绘；有偏斜时取消 flash 的自动隐藏。 */
function renderVersionSkewBanner() {
  if (state.versionSkew !== true) return;
  const probeNote =
    state.childProbe && state.childProbe !== 'ok'
      ? `child 探针失败（${state.childProbe}）：磁盘二进制版本未能读出，偏斜未能确认；`
      : '';
  clearTimeout(flash._t);
  connBanner.hidden = false;
  connBanner.textContent =
    `版本偏斜：桥 ${state.bridgeVersion || '未知'} ≠ agent 子进程 ${state.childVersion || '未知'}。` +
    probeNote +
    '→ 用同一版本重新换装后重启 orz web；混跑可能造成协议/字段不一致（证据：/api/boot version_skew）。';
}

/* 333（2026-10-21，S12）：boot 事实落地——工作区清单＋桥/child 版本偏斜。
 * 独立函数便于单测/冒烟直接注入 boot 载荷（不重跑整条连接链）。 */
function applyBootFacts(b) {
  const facts = b || {};
  state.workspaceCwd = facts.cwd || '';
  state.trustedWorkspaces = facts.trusted_workspaces || [];
  state.recentWorkspaces = facts.recent_workspaces || [];
  state.bridgeVersion = facts.bridge_version ?? null;
  state.childVersion = facts.child_version ?? null;
  state.childProbe = facts.child_probe ?? null;
  state.versionSkew = facts.version_skew ?? null;
  if (facts.version_skew === true) {
    // 334（§3.1）：桥版本与磁盘 child 版本不一致时必须同时交代探针结果
    // ——探针失败时 child_version 为 null，「偏斜」的证据也随之弱化。
    renderVersionSkewBanner();
    const probeNote =
      facts.child_probe && facts.child_probe !== 'ok'
        ? `child 探针失败（${facts.child_probe}）：磁盘二进制版本未能读出，偏斜未能确认；`
        : '';
    vm.addSystemMessage(
      `（版本偏斜：桥 ${facts.bridge_version || '?'}，agent ${facts.child_version || '?'}${
        probeNote ? `；${probeNote}` : ''
      }；建议重装同版本后再用）`,
      true,
    );
  }
  renderExplorer();
}

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
  accessToken = token;
  acp.rememberToken(token);
  // 工作区事实（当前 cwd＋已信任工作区清单）先行拉取——探索器「工作区」
  // 组的数据源，不等首次会话创建。
  try {
    const b = await fetchBoot();
    applyBootFacts(b);
  } catch {
    /* boot 不可达：探索器仍显示 nav 当前值 */
  }
  // 334（§4.5）：刷新后把归属根与会话 id 带回 WS（root 决定桥槽；session
  // 让保活中的旧 child 直接 attach）。sessionStorage 读不到时按无绑定连接。
  rememberedSessionId = sessionStoreGet(SESSION_ID_KEY);
  rememberedSessionRoot = sessionStoreGet(SESSION_ROOT_KEY);
  try {
    await acp.connect(token, state.workspaceCwd, rememberedSessionId || null);
  } catch (e) {
    connBanner.hidden = false;
    connBanner.textContent = `连接失败: ${e.message}`;
  }
  renderAll();
  await startTail();
  // 318 P1-B7：刷新/断线恢复边界必须在 UI 里有字——不是只在 beforeunload
  // 的浏览器原生提示里（原生文案现代浏览器不可定制，且未必显示）。
  vm.addSystemMessage(
    '（提示：刷新/短时断线后重开页面会重连到同一会话（子进程保活）；桥进程退出或超过保活窗口后，排队补充与后台任务不恢复）',
    false,
  );
  renderAll();
  // 每 5 秒刷新探索器数据（journal 尾为实时事实通道）。
  setInterval(async () => {
    try {
      await refreshExplorerData();
    } catch {
      /* 服务未起/令牌变化：横幅已提示 */
    }
  }, 5000);
}

/* 334：冒烟测试钩子（不新增 UI 面）——绑定读数归零/attach 顺序可直接注入。 */
export const __testHooks = { setBinding, detachBinding, resyncBusyState, onBridgeAttached };

boot();
