/*
 * dialogs.js —— 模态面（自研登记：0br S1 §4.2 R-3/R-6）。
 * 权限弹窗对接 ACP session/request_permission（呈现层自研、判定在
 * permission gate —— ADR-0010 §2.4 条 8）；Help 为分页 modal overlay
 * （补充稿 §12，六冻结页：快捷键/命令/模型/审批/终端/来源，默认中文）；
 * 查找/属性/命令弹窗照补充稿 §5/§6 弹窗化。
 */

import { state, vm } from './state.js';

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

function settlePermissionCancelled() {
  if (!activePermission) return;
  const resolve = activePermission;
  activePermission = null;
  resolve({ outcome: { outcome: 'cancelled' } });
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
    settlePermissionCancelled();
    activePermission = resolve;
    openModal((win, body) => {
      const title = modalTitle('权限请求');
      win.appendChild(title);

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
