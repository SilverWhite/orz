/*
 * dialogs.js —— 模态面（自研登记：0br S1 §4.2 R-3/R-6）。
 * 权限弹窗对接 ACP session/request_permission（呈现层自研、判定在
 * permission gate —— ADR-0010 §2.4 条 8）；Help 为分页 modal overlay
 * （补充稿 §12，六冻结页：快捷键/命令/模型/审批/终端/来源，默认中文）；
 * 查找/属性/命令弹窗照补充稿 §5/§6 弹窗化。
 */

import { state, vm } from './state.js';
/* 329（2026-10-21，0dm S2）：交付状态只读投影——请求走 api.js，渲染在
 * delivery.js（纯函数）；本文件只管弹窗生命周期（加载/错误/空态）。 */
import { fetchDelivery, fetchContext } from './api.js';
import { renderDeliveryStatus } from './delivery.js';

const SLASH_COMMANDS = [
  ['/help', '帮助', '打开帮助弹窗'],
  ['/status', '状态', '查看当前状态摘要'],
  ['/stop', '停止', '取消当前运行'],
  ['/open', '打开', '打开命令/位置输入'],
  ['/toggle-explorer', '探索器', '显示/隐藏左侧探索器'],
  ['/toggle-events', '事件栏', '显示/隐藏事件栏'],
  ['/toggle-markers', '标记栏', '显示/隐藏标记栏'],
  ['/properties', '属性', '查看当前对象属性'],
  ['/snapshots', '快照', '快照选择器（v1 只读展示）'],
  ['/rewind', '撤回', '撤回会话中的用户消息并编辑重发'],
];

let actions = {}; // main.js 注入的宿主动作（发送/取消/导航…）

export function bindDialogActions(a) {
  actions = a;
}

function layer() {
  return document.getElementById('modalLayer');
}

/* 统一弹窗标题栏：标题文本 ＋ 98.css 关闭钮（走查反馈四 2026-09-25：
 * Web 端弹窗需要明确的关闭 affordance）。关闭＝Esc 同语义——权限弹窗
 * 按取消结算回传 agent。aria-label 固定英文 "Close"（98.css 属性选择
 * 锚点），中文提示走 title。 */
function modalTitle(text) {
  const title = document.createElement('div');
  title.className = 'title-bar';
  const textEl = document.createElement('div');
  textEl.className = 'title-bar-text';
  textEl.textContent = text;
  title.appendChild(textEl);
  const controls = document.createElement('div');
  controls.className = 'title-bar-controls';
  const close = document.createElement('button');
  close.setAttribute('aria-label', 'Close');
  close.setAttribute('title', '关闭');
  close.addEventListener('click', () => closeModal());
  controls.appendChild(close);
  title.appendChild(controls);
  return title;
}

/* ── 权限应答结算（审查处理批 C-1） ──
 * ACP session/request_permission 必须应答，而 ACP 无超时字段——弹窗被
 * Esc 关闭/被其他弹窗替换/被并发请求顶掉时，若不应答 agent 将永久等待。
 * 规则：活动权限请求离开弹窗（任何路径）一律结算为 cancelled，由桥如实
 * 回传 agent；选项按钮的显式选择优先于该兜底。 */

let activePermission = null; // showPermission 的 resolve
/// 333：被并发请求顶掉的历史计数（本页生命周期内）；新弹窗据此显示
/// 「上一条已取消」，不再让旧请求静默消失。
let permissionDisplaced = 0;

function settlePermissionCancelled() {
  if (!activePermission) return false;
  const resolve = activePermission;
  activePermission = null;
  resolve({ outcome: { outcome: 'cancelled' } });
  return true;
}

function finishPermission(result) {
  const resolve = activePermission;
  activePermission = null;
  closeModalNow();
  resolve?.(result);
}

function closeModal() {
  settlePermissionCancelled();
  closeModalNow();
}

function closeModalNow() {
  layer().replaceChildren();
  layer().classList.remove('open');
}

function openModal(buildBody, { keepPermission = false } = {}) {
  if (!keepPermission) settlePermissionCancelled();
  const root = layer();
  root.replaceChildren();
  root.classList.add('open');
  const win = document.createElement('div');
  win.className = 'window modal-window';
  const body = document.createElement('div');
  body.className = 'window-body';
  // buildBody 先把标题栏挂上 win，body 随后插到标题之下（挂载顺序即
  // 视觉顺序；body 游离问题同批已修，见审查处理档 §8）。
  buildBody(win, body);
  win.appendChild(body);
  root.appendChild(win);
  return win;
}

export function isModalOpen() {
  return layer().classList.contains('open');
}

export { closeModal };

/* ── 权限弹窗（R-6：呈现层自研；选项来自 ACP 请求） ── */

export function showPermission(request) {
  return new Promise((resolve) => {
    // 并发第二个权限请求：前一个结算为 cancelled（被顶掉，如实回传），
    // 再挂载本请求——不排队（v1 语义：一次只呈现一个，见 S2 §2 偏差 2）。
    const displaced = settlePermissionCancelled();
    if (displaced) permissionDisplaced += 1;
    activePermission = resolve;
    openModal((win, body) => {
      const title = modalTitle('权限请求');
      win.appendChild(title);

      // 333：顶掉上一条必须在**新弹窗里**可见——用户可能正要点旧选项，
      // 旧请求已被 cancelled，不能让它无声消失。
      if (displaced) {
        const warn = document.createElement('div');
        warn.className = 'perm-displaced';
        warn.style.background = 'rgba(255, 196, 0, 0.25)';
        warn.style.padding = '3px 5px';
        warn.style.margin = '2px 0 6px';
        warn.textContent =
          '注意：上一条权限请求已被本次请求顶掉（已回 cancelled）；请确认正在审批的是最新工具。' +
          `本页累计顶掉 ${permissionDisplaced} 次。`;
        body.appendChild(warn);
      }

      const tool = request?.toolCall?.title || request?.toolCall?.toolCallId || '未知工具';
      const info = document.createElement('div');
      info.className = 'permission-body';
      const head = document.createElement('div');
      head.className = 'perm-tool';
      head.textContent = `工具「${tool}」请求权限`;
      info.appendChild(head);
      const kind = document.createElement('div');
      kind.textContent = `风险类: ${request?.toolCall?.kind ?? '—'}`;
      info.appendChild(kind);
      const raw = document.createElement('div');
      raw.className = 'perm-raw';
      raw.textContent = JSON.stringify(request?.toolCall, null, 2);
      info.appendChild(raw);
      body.appendChild(info);

      const actionsRow = document.createElement('div');
      actionsRow.className = 'modal-actions';
      const options = request?.options ?? [];
      for (const opt of options) {
        const btn = document.createElement('button');
        btn.textContent = opt.name || opt.optionId;
        btn.addEventListener('click', () => {
          finishPermission({ outcome: { outcome: 'selected', optionId: opt.optionId } });
        });
        actionsRow.appendChild(btn);
      }
      const cancel = document.createElement('button');
      cancel.textContent = '取消';
      cancel.addEventListener('click', () => {
        finishPermission({ outcome: { outcome: 'cancelled' } });
      });
      actionsRow.appendChild(cancel);
      body.appendChild(actionsRow);
    }, { keepPermission: true });
  });
}

/* ── Help（六冻结页，modals.rs HelpOverlay 移植；默认中文） ── */

const HELP_TABS = () => [
  {
    name: '快捷键',
    rows: [
      ['F6', '切换焦点（聊天 / 探索器 / 内容区）'],
      ['Tab / Shift+Tab', '区内焦点移动'],
      ['Esc', '关闭弹窗 / 对话框'],
      ['双 Esc', '运行历史列表（探索器聚焦）'],
      ['Enter', '提交 / 激活选中项'],
      ['↑ ↓', '输入历史 / 树与列表移动'],
      ['← →', '弹窗标签切换'],
      ['Ctrl+L', '命令/位置输入'],
      ['Ctrl+F', '查找'],
      ['Ctrl+Z', '取消当前运行'],
      ['Alt+字母', '打开对应菜单'],
      ['Alt+H / F1', '帮助'],
    ],
  },
  { name: '命令', rows: SLASH_COMMANDS.map(([slash, name, desc]) => [slash, `${name} — ${desc}`]) },
  {
    name: '模型',
    rows: [
      ['当前适配器', state.modelLabel],
      ['可用模型', '—（v1 未接线）'],
      ['推理强度', '—（v1 未接线）'],
      ['Plan 模式', '—（v1 未接线）'],
    ],
  },
  {
    name: '审批',
    rows: [
      ['Action 审批', '工具权限对话框（按请求选项允许/取消）'],
      ['审批策略', '手动（权限门 fail-closed）'],
      ['判定归属', '权限判定由 permission gate 决定，UI 只呈现与回应'],
    ],
  },
  {
    name: '终端',
    rows: [
      ['形态', 'Web 工作台（本地回环服务）'],
      ['桥', 'orz web —— ACP-over-WebSocket，单会话'],
      ['标题', 'orz 工作台'],
    ],
  },
  {
    name: '来源',
    rows: [
      ['可见性等级', '工具 n/m（探针链路，状态栏）'],
      ['证据等级', '门控决定决定等级（journal 面呈现）'],
      ['检索命令', '（v1 未接线）'],
    ],
  },
];

export function openHelp(focusTab) {
  let active = Math.max(0, HELP_TABS().findIndex((t) => t.name === focusTab));
  openModal((win, body) => {
    const title = modalTitle('帮助');
    win.appendChild(title);
    const tabsRow = document.createElement('div');
    tabsRow.className = 'modal-tabs';
    const content = document.createElement('div');
    content.className = 'window-body';
    const render = () => {
      tabsRow.replaceChildren();
      HELP_TABS().forEach((tab, i) => {
        const b = document.createElement('button');
        b.textContent = tab.name;
        if (i === active) b.classList.add('active');
        b.addEventListener('click', () => {
          active = i;
          render();
        });
        tabsRow.appendChild(b);
      });
      content.replaceChildren();
      const tab = HELP_TABS()[active];
      // 六页统一两列表格（走查反馈四补：快捷键页原为等宽文本块，口径
      // 与其他页不一致）；help-table 首列随内容自适应宽度。
      const table = document.createElement('table');
      table.className = 'props-table help-table';
      for (const [k, v] of tab.rows ?? []) {
        const tr = document.createElement('tr');
        const td1 = document.createElement('td');
        td1.textContent = k;
        const td2 = document.createElement('td');
        td2.textContent = v;
        tr.appendChild(td1);
        tr.appendChild(td2);
        table.appendChild(tr);
      }
      content.appendChild(table);
    };
    body.appendChild(tabsRow);
    body.appendChild(content);
    win._cycleTab = (dir) => {
      const tabs = HELP_TABS().length;
      active = (active + dir + tabs) % tabs;
      render();
    };
    render();
  });
}

/* ── 查找（多行弹窗，scope 选择器冻结五项；v1 检索当前对话） ── */

const FIND_SCOPES = ['对话', '项目文档', '当前文件', '运行/制品', '索引来源'];

export function openFind() {
  openModal((win, body) => {
      const title = modalTitle('查找');
    win.appendChild(title);
    const scopeRow = document.createElement('div');
    scopeRow.style.marginBottom = '6px';
    const sel = document.createElement('select');
    for (const s of FIND_SCOPES) {
      const opt = document.createElement('option');
      opt.textContent = s;
      sel.appendChild(opt);
    }
    scopeRow.appendChild(sel);
    body.appendChild(scopeRow);
    const ta = document.createElement('textarea');
    ta.rows = 3;
    ta.style.width = '96%';
    ta.placeholder = '查找内容（字面匹配，不区分大小写）';
    body.appendChild(ta);
    const actionsRow = document.createElement('div');
    actionsRow.className = 'modal-actions';
    const doFind = document.createElement('button');
    doFind.textContent = '查找';
    doFind.addEventListener('click', () => {
      try {
        const n = actions.findInConversation(ta.value, sel.value);
        closeModal();
        actions.flash(`查找完成：${n} 处命中（标记栏）`);
      } catch (e) {
        alert(`查找失败: ${e.message}`);
      }
    });
    const cancel = document.createElement('button');
    cancel.textContent = '取消';
    cancel.addEventListener('click', closeModal);
    actionsRow.appendChild(doFind);
    actionsRow.appendChild(cancel);
    body.appendChild(actionsRow);
  });
}

/* ── 属性（Properties：当前会话/对象元数据） ── */

export function openProperties() {
  openModal((win, body) => {
      const title = modalTitle('属性');
    win.appendChild(title);
    const rows = [
      ['当前对象', state.nav.current],
      ['ACP 会话', state.sessionId ?? '—'],
      ['模型', state.modelLabel],
      ['运行态', state.running ? '运行中' : '空闲'],
      ['轮数', String(state.turnCounter)],
      ['内容条目', String(state.content.items.length)],
      // 333（S5）：队列深度与剩余额度显式化——上限随 `_orz/queue_append`
      // 响应回填，作用域只在本会话进程（刷新/换页不保留）。
      [
        '补充队列',
        `${state.supplementQueueDepth}/${state.queueCapacity ?? 16} 条（剩余 ${
          (state.queueCapacity ?? 16) - state.supplementQueueDepth
        }；仅本会话进程，刷新/换页不保留）`,
      ],
      // 333（S7）：后台任务明确「仅模型可见」——本页不提供任务表，且刷新/
      // 关闭/子进程回收后不恢复（不虚构一个读不到的列表）。
      [
        '后台任务',
        state.backgroundTasks > 0
          ? `${state.backgroundTasks} 个（仅模型可见；本页不提供任务表，刷新/关闭不恢复）`
          : '仅模型可见（本页不提供任务表；刷新/关闭不恢复）',
      ],
      ['会话（conversation）', 'conversation://{session8}（侧车由宿主管理）'],
    ];
    const table = document.createElement('table');
    table.className = 'props-table';
    for (const [k, v] of rows) {
      const tr = document.createElement('tr');
      const td1 = document.createElement('td');
      td1.textContent = k;
      const td2 = document.createElement('td');
      td2.textContent = v;
      tr.appendChild(td1);
      tr.appendChild(td2);
      table.appendChild(tr);
    }
    body.appendChild(table);
  });
}

/* ── 命令/位置输入（AddressBar 弹窗化，补充稿 §5） ── */

/* ── 0di（2026-10-18，ADR-0010 §14.88）：撤回对话框（编辑式重发）──
 * 列表＝会话续接包的用户消息（`_orz/recall_preview`，侧车真相面）；选择
 * 后确认＝`_orz/recall` 截断＋原文回输入框。运行历史/journal 不动（旧卡
 * 保持为历史证据）；文件面 git 兜底；运行中由服务端 idle 门拒绝。 */
export function openRewind() {
  openModal(async (win, body) => {
    const title = modalTitle('撤回消息');
    win.appendChild(title);
    const status = document.createElement('div');
    status.style.margin = '6px 0';
    status.textContent = '正在读取可撤回消息…';
    body.appendChild(status);
    let entries = [];
    let queueDepth = 0;
    try {
      const d = await actions.rewindPreview();
      entries = d.messages || [];
      queueDepth = d.queue_depth || 0;
    } catch (e) {
      status.textContent = `读取失败：${e.message}`;
      return;
    }
    if (!entries.length) {
      status.textContent = '（会话尚无用户消息，无可撤回）';
      return;
    }
    status.textContent =
      `选择要撤回的用户消息——该条及其后的上下文将被移除，原文回输入框供编辑重发（运行历史不变；文件面 git 兜底）：${
        queueDepth > 0 ? ` 当前有 ${queueDepth} 条未投递补充，撤回会一并丢弃。` : ''
      }`;
    let selected = entries.length - 1; // 默认选最新一条（最常用）
    const list = document.createElement('div');
    list.style.maxHeight = '240px';
    list.style.overflow = 'auto';
    const renderRows = () => {
      list.replaceChildren();
      entries.forEach((entry, i) => {
        const row = document.createElement('div');
        row.className = 'rewind-row';
        row.style.padding = '2px 4px';
        row.style.cursor = 'pointer';
        row.style.background = i === selected ? 'rgba(0,0,0,0.18)' : '';
        row.textContent = `#${entry.ordinal}  ${entry.preview}${entry.character_count > Array.from(entry.preview).length ? '…' : ''}（${entry.character_count} 字符）`;
        row.addEventListener('click', () => {
          selected = i;
          renderRows();
        });
        list.appendChild(row);
      });
    };
    renderRows();
    body.appendChild(list);
    const actionsRow = document.createElement('div');
    actionsRow.className = 'modal-actions';
    const ok = document.createElement('button');
    ok.textContent = '撤回';
    ok.addEventListener('click', async () => {
      const entry = entries[selected];
      ok.disabled = true;
      try {
        const r = await actions.rewindRecall(entry.ordinal);
        closeModal();
        actions.fillInput(r.recall_text || '');
        const droppedNote = r.dropped_supplements > 0 ? `；${r.dropped_supplements} 条未投递补充已一并丢弃` : '';
        actions.flash(`已撤回 ${r.removed_messages} 条消息${droppedNote}——原文已回输入框`);
      } catch (e) {
        ok.disabled = false;
        actions.flash(`撤回失败：${e.message}`);
      }
    });
    const cancel = document.createElement('button');
    cancel.textContent = '取消';
    cancel.addEventListener('click', closeModal);
    actionsRow.appendChild(ok);
    actionsRow.appendChild(cancel);
    body.appendChild(actionsRow);
  });
}

export function openCommand(initial) {
  openModal((win, body) => {
      const title = modalTitle('命令 / 位置');
    win.appendChild(title);
    const input = document.createElement('input');
    input.style.width = '96%';
    input.value = initial || 'command://';
    input.placeholder = 'command://… / workspace://… / run://… / /help 等';
    body.appendChild(input);
    const hint = document.createElement('div');
    hint.style.color = '#555';
    hint.style.marginTop = '4px';
    hint.textContent = 'Enter 确认，Esc 取消；对象 URI 进入地址栏历史。';
    body.appendChild(hint);
    const actionsRow = document.createElement('div');
    actionsRow.className = 'modal-actions';
    const ok = document.createElement('button');
    ok.textContent = '执行';
    ok.addEventListener('click', () => {
      const v = input.value.trim();
      closeModal();
      if (v) actions.runAddress(v);
    });
    const cancel = document.createElement('button');
    cancel.textContent = '取消';
    cancel.addEventListener('click', closeModal);
    actionsRow.appendChild(ok);
    actionsRow.appendChild(cancel);
    body.appendChild(actionsRow);
    setTimeout(() => input.focus(), 0);
  });
}

/* ── 0dm S2（2026-10-21）：交付状态（只读事实投影） ──
 * 设计 §4.2：`submit` 已从模型面退役，交付由 `[RUN_END]`（intent=conclude /
 * reason=completed）机械完成；`compute_delivery_status` 依赖 loop 运行时内存
 * 基线，空闲 UI 不可复算。本面板只读 `/api/delivery`（侧车 plan ＋ 最近 run
 * 的 run_finished/tool_completed 事实）：不写状态、不发 prompt、不触发模型。
 * 加载/错误/空态显式呈现；切会话或切视图后迟到的结果作废（同 openRewind）。 */
export function openDeliveryStatus() {
  openModal(async (win, body) => {
    const title = modalTitle('交付状态');
    win.appendChild(title);
    const status = document.createElement('div');
    status.style.margin = '6px 0';
    body.appendChild(status);

    const s8 = state.boundSession8;
    if (!s8) {
      status.textContent =
        '尚未绑定会话——先打开或恢复一个会话，再查询交付状态（本面板只读，不触发模型）。';
      return;
    }
    status.textContent = `正在读取会话 ${s8} 的交付事实…`;
    const epoch = state.navEpoch;
    const root = state.sessionRoot || state.workspaceCwd || '';
    let data;
    try {
      data = await fetchDelivery(s8, root);
    } catch (e) {
      status.textContent = `交付状态读取失败：${e.message}（只读投影，未触发模型）`;
      return;
    }
    if (state.boundSession8 !== s8 || state.navEpoch !== epoch) {
      status.textContent = '会话绑定已变化——本次查询结果作废；请重新打开交付状态。';
      return;
    }
    // 浏览器有 Element.remove；smoke 的 DOM stub 没有——两种形态都收。
    if (typeof status.remove === 'function') status.remove();
    else body.removeChild(status);
    renderDeliveryStatus(body, data);

    const actionsRow = document.createElement('div');
    actionsRow.className = 'modal-actions';
    const close = document.createElement('button');
    close.textContent = '关闭';
    close.addEventListener('click', closeModal);
    actionsRow.appendChild(close);
    body.appendChild(actionsRow);
    close.focus();
  });
}

/* 333（2026-10-21，S9）：上下文占用与最近压缩读数。只读投影（`/api/context`
 * → 侧车消息估算＋最近 run 的 context_compressed 事件），不触发模型；切会话
 * 或切视图后迟到的结果作废（同 openDeliveryStatus）。 */
export function openContext() {
  openModal(async (win, body) => {
    const title = modalTitle('上下文');
    win.appendChild(title);
    const status = document.createElement('div');
    status.style.margin = '6px 0';
    body.appendChild(status);

    const s8 = state.boundSession8;
    if (!s8) {
      status.textContent =
        '尚未绑定会话——先打开或恢复一个会话，再查看上下文读数（只读投影，不触发模型）。';
      return;
    }
    status.textContent = `正在读取会话 ${s8} 的上下文读数…`;
    const epoch = state.navEpoch;
    const root = state.sessionRoot || state.workspaceCwd || '';
    let data;
    try {
      data = await fetchContext(s8, root);
    } catch (e) {
      status.textContent = `上下文读数读取失败：${e.message}（只读投影，未触发模型）`;
      return;
    }
    if (state.boundSession8 !== s8 || state.navEpoch !== epoch) {
      status.textContent = '会话绑定已变化——本次读数作废；请重新打开上下文。';
      return;
    }
    // 浏览器有 Element.remove；smoke 的 DOM stub 没有——两种形态都收。
    if (typeof status.remove === 'function') status.remove();
    else body.removeChild(status);

    const heading = document.createElement('div');
    heading.style.fontWeight = 'bold';
    heading.style.margin = '2px 0 6px';
    heading.textContent = '上下文与压缩（只读读数）';
    body.appendChild(heading);

    const compaction = data.last_compaction;
    const watermarks = Array.isArray(data.watermarks) ? data.watermarks : [];
    const rows = [
      [
        '估算占用',
        `${data.estimated_tokens ?? 0} tokens（${data.chars ?? 0} 字符，` +
          `${data.estimate_method || 'chars/2'} 近似；非模型 tokenizer 计数）`,
      ],
      ['消息数', data.messages === null || data.messages === undefined ? '—' : String(data.messages)],
      ['水位提醒', watermarks.length ? watermarks.join('、') : '无（会话内未触发刻度提醒）'],
      [
        '最近压缩',
        compaction
          ? `${compaction.run_id}：trigger ${compaction.trigger_tokens ?? '—'} → after ${
              compaction.estimated_tokens_after ?? '—'
            }（${compaction.mode || '—'}/${compaction.reason || '—'}）`
          : '无（未压缩，或事件早于扫描窗口）',
      ],
      ['压缩存档', `${data.compaction_archives ?? 0} 件（含 blocks 回放档）`],
    ];
    const table = document.createElement('table');
    table.className = 'props-table';
    for (const [k, v] of rows) {
      const tr = document.createElement('tr');
      const tdKey = document.createElement('td');
      tdKey.textContent = k;
      const tdVal = document.createElement('td');
      tdVal.textContent = v;
      tr.appendChild(tdKey);
      tr.appendChild(tdVal);
      table.appendChild(tr);
    }
    body.appendChild(table);

    const notes = Array.isArray(data.notes) ? data.notes : [];
    if (notes.length) {
      const caption = document.createElement('div');
      caption.className = 'muted';
      caption.style.margin = '8px 0 2px';
      caption.textContent = '边界与说明';
      body.appendChild(caption);
      for (const note of notes) {
        const line = document.createElement('div');
        line.className = 'muted';
        line.style.margin = '2px 0';
        line.textContent = `· ${note}`;
        body.appendChild(line);
      }
    }

    const actionsRow = document.createElement('div');
    actionsRow.className = 'modal-actions';
    const close = document.createElement('button');
    close.textContent = '关闭';
    close.addEventListener('click', closeModal);
    actionsRow.appendChild(close);
    body.appendChild(actionsRow);
    close.focus();
  });
}
