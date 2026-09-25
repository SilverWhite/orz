/*
 * widgets.js —— 各区域渲染（自研登记：0br S1 §4.2 R-3/R-4/R-5）。
 * 控件质感来自 98.css（照搬件）；交互语义移植 orz-tui widgets.rs/app.rs。
 * ContentPane：有边框卡片＝对话（Markdown 框内展开）、无边框行＝机器动作、
 * 展开工具清单 pin-to-top（CONTENT_PANE_CONVERSATION_RENDERING_v0.1 §2–§5）。
 */

import { state, vm } from './state.js';
import { renderMarkdown } from './md.js';

export const ui = {};

function el(tag, cls, text) {
  const node = document.createElement(tag);
  if (cls) node.className = cls;
  if (text !== undefined) node.textContent = text;
  return node;
}

/* ── MenuBar ── */

let openMenu = null;

export function closeMenus() {
  document.querySelectorAll('.menu-dropdown').forEach((n) => n.remove());
  document.querySelectorAll('.region-menubar .menu-item.open').forEach((n) => n.classList.remove('open'));
  openMenu = null;
}

const MENUS = {
  文件: [
    { label: '工作区信息', action: () => ui.openProperties() },
    { label: '探索器 开/关（切换）', action: () => ui.toggleExplorer() },
  ],
  事件: [{ label: '事件栏 开/关', action: () => ui.toggleEvents() }],
  标记: [{ label: '标记栏 开/关', action: () => ui.toggleMarkers() }],
  编辑模式: [{ label: '（v1：编辑行为随输入框原生能力）', disabled: true }],
  模型: [{ label: '查看当前模型（状态栏）', action: () => ui.flashStatus() }],
  来源: [{ label: '来源可见性说明', action: () => ui.openHelp('来源') }],
  运行: [
    { label: '运行历史', action: () => ui.focusExplorer() },
    { label: '停止当前运行', action: () => ui.stopRun() },
  ],
  验证: [{ label: '（v1 未接线）', disabled: true }],
  帮助: [{ label: '帮助…', action: () => ui.openHelp() }],
};

// 按名打开菜单（Alt+字母 访问键入口；menuRefs 随每次渲染重建）。
const menuRefs = {};

export function openMenuByName(name) {
  menuRefs[name]?.open();
}

export function renderMenubar() {
  const root = ui.menubar;
  root.replaceChildren();
  for (const name of Object.keys(menuRefs)) delete menuRefs[name];
  for (const name of state.menu) {
    const item = el('span', 'menu-item', name);
    item.tabIndex = 0;
    item.setAttribute('role', 'menuitem');
    const openDropdown = () => {
      closeMenus();
      item.classList.add('open');
      openMenu = name;
      const dd = el('div', 'menu-dropdown window');
      const body = el('div', 'window-body');
      body.style.padding = '2px';
      for (const entry of MENUS[name] ?? []) {
        if (entry.disabled) {
          body.appendChild(el('div', 'menu-entry disabled', entry.label));
        } else {
          const row = el('div', 'menu-entry', entry.label);
          row.addEventListener('click', () => {
            closeMenus();
            entry.action();
          });
          body.appendChild(row);
        }
      }
      dd.appendChild(body);
      item.appendChild(dd);
    };
    menuRefs[name] = { item, open: openDropdown };
    item.addEventListener('click', (e) => {
      e.stopPropagation();
      if (openMenu === name) closeMenus();
      else openDropdown();
    });
    item.addEventListener('keydown', (e) => {
      if (e.key === 'Enter' || e.key === ' ') {
        e.preventDefault();
        if (openMenu === name) closeMenus();
        else openDropdown();
      }
    });
    root.appendChild(item);
  }
}
document.addEventListener('click', () => closeMenus());

/* ── Toolbar ── */

const TOOLBAR_ACTIONS = {
  back: () => ui.navAction('back'),
  forward: () => ui.navAction('forward'),
  refresh: () => ui.refresh(),
  stop: () => ui.stopRun(),
  open: () => ui.openCommand(),
  verify: () => {},
  properties: () => ui.openProperties(),
};

export function renderToolbar() {
  const root = ui.toolbar;
  root.replaceChildren();
  for (const btn of state.toolbar) {
    const b = el('button', null, btn.label);
    b.disabled = !btn.enabled;
    b.addEventListener('click', () => TOOLBAR_ACTIONS[btn.id]?.());
    root.appendChild(b);
  }
  const spacer = el('span', 'spacer');
  root.appendChild(spacer);
  // 第二行状态摘要（补充稿 §4：Plan/Manual | 模型 | 运行态）
  const running = state.status.find((s) => ['空闲', '预检', '运行中', '等待审批', '完成', '失败', '已取消', '无效', '恢复中'].includes(s.label));
  const model = state.modelLabel;
  root.appendChild(el('span', 'summary', `手动 | ${model} | ${running ? running.label : '空闲'}`));
  root.appendChild(el('span', 'summary', state.nav.current));
}

/* ── Explorer ── */

export function renderExplorer() {
  const root = ui.explorer;
  root.replaceChildren();
  const title = el('div', null, '探索器');
  title.style.fontWeight = 'bold';
  root.appendChild(title);

  const groupHeader = (label, key) => {
    const head = el('div', 'explorer-group', `${state.explorer[key] ? '▸' : '▾'} ${label}`);
    head.tabIndex = 0;
    head.setAttribute('role', 'button');
    const toggle = () => {
      state.explorer[key] = !state.explorer[key];
      renderExplorer();
    };
    head.addEventListener('click', toggle);
    head.addEventListener('keydown', (e) => {
      if (e.key === 'Enter' || e.key === ' ') {
        e.preventDefault();
        toggle();
      }
    });
    return head;
  };
  const subHeader = (label, key) => {
    const head = el('div', 'explorer-subgroup', `${state.explorer[key] ? '▸' : '▾'} ${label}`);
    head.tabIndex = 0;
    head.setAttribute('role', 'button');
    const toggle = () => {
      state.explorer[key] = !state.explorer[key];
      renderExplorer();
    };
    head.addEventListener('click', toggle);
    head.addEventListener('keydown', (e) => {
      if (e.key === 'Enter' || e.key === ' ') {
        e.preventDefault();
        toggle();
      }
    });
    return head;
  };

  /* ── 工作区（用户令 2026-09-25：总栏加粗；两级子级＝当前工作区＋
   * 全部已信任工作区。信任清单来自用户全局信任存储的只读投影
   * （/api/boot），展示面不动信任判定。） ── */
  root.appendChild(groupHeader('工作区', 'workspaceCollapsed'));
  if (!state.explorer.workspaceCollapsed) {
    const cur = el('div', 'marker-entry explorer-sub', `当前: ${state.workspaceCwd || state.nav.current}`);
    cur.title = 'workspace://live —— 点击回到实时';
    cur.addEventListener('click', () => ui.openUri('workspace://live'));
    root.appendChild(cur);

    const trusted = state.trustedWorkspaces || [];
    root.appendChild(subHeader(`已信任工作区（${trusted.length}）`, 'trustedCollapsed'));
    if (!state.explorer.trustedCollapsed) {
      if (!trusted.length) root.appendChild(el('div', 'explorer-sub muted', '（无记录）'));
      for (const t of trusted) {
        const row = el('div', 'explorer-sub muted', t.path);
        row.title = t.decided_at
          ? `信任于 ${new Date(t.decided_at * 1000).toLocaleString()}（只读展示；本工作台绑定当前工作区）`
          : '已信任（只读展示；本工作台绑定当前工作区）';
        root.appendChild(row);
      }
    }
  }

  /* ── 会话分组（批三修正 2026-09-25）：归档判定＝有归档包**且**包 mtime
   * 不早于该会话最新运行——归档后立即移入「归档会话」组；归档后又继续
   * 的会话（新 run 更新 journal）如实回到活跃组。 ── */
  const groups = new Map();
  for (const run of state.runs) {
    const m = /^(?:RUN-CLI-|ARC-|RUN-)([0-9a-f]{8})/.exec(run.run_id);
    const s8 = m ? m[1] : '未识别';
    if (!groups.has(s8)) groups.set(s8, []);
    groups.get(s8).push(run);
  }
  for (const c of state.conversations) {
    if (!groups.has(c.session8)) groups.set(c.session8, []); // 有侧车暂无运行
  }
  const latestRunMs = new Map();
  for (const [s8, runs] of groups) {
    latestRunMs.set(s8, Math.max(0, ...runs.map((r) => r.modified_ms || 0)));
  }
  const archiveFor = new Map((state.archives || []).map((a) => [a.session8, a]));
  const isArchived = (s8) => {
    const a = archiveFor.get(s8);
    return !!a && a.modified_ms >= (latestRunMs.get(s8) || 0);
  };

  const lm = /^(?:RUN-CLI-|ARC-|RUN-)([0-9a-f]{8})/.exec(state.liveRunId || '');
  const liveS8 = state.running && lm ? lm[1] : null;

  const activeSessions = [...groups.keys()].filter((s8) => !isArchived(s8));
  root.appendChild(groupHeader(`活跃会话（${activeSessions.length}）`, 'activeCollapsed'));
  if (!state.explorer.activeCollapsed) {
    for (const s8 of activeSessions) {
      const runs = groups.get(s8);
      // 「进行中」只属于真正在动作/流式输出的实时窗口；journal 无终态
      // 事件的旧 run 实为中断收场（用户令 2026-09-25），如实标「未完成」。
      const suffix = liveS8 === s8 ? ' · 进行中' : runs.some((r) => r.status === 'running') ? ' · 未完成' : '';
      const row = el('div', 'marker-entry');
      // 会话 ID 与时间用等宽字体双列（批四补：比例字体下 hex 宽窄不一
      // 导致列表参差）；ID 恒 8 字符、时间恒定长格式 ⇒ 两列严格对齐。
      row.appendChild(el('span', 'session-id', s8));
      row.appendChild(document.createTextNode(`（${runs.length} 次运行${suffix}）`));
      row.title = `conversation://${s8} —— 点击打开整段会话`;
      row.addEventListener('click', () => ui.openUri(`conversation://${s8}`));
      // ◆归档＝归档动作（打包为 .gsa/archives 归档包；执行在桥 spawn 的
      // agent 子命令，前端零执行事实）。
      const jump = el('span', 'archive-jump', '◆归档');
      if (liveS8 === s8) {
        jump.classList.add('disabled');
        jump.title = '运行进行中，暂不能归档';
      } else {
        jump.title = `归档会话 ${s8}（打包为归档包＋ARC 审计 journal）`;
        jump.addEventListener('click', (e) => {
          e.stopPropagation();
          ui.archiveSession?.(s8);
        });
      }
      row.appendChild(jump);
      root.appendChild(row);
    }
  }

  const archivedSessions = (state.archives || []).filter((a) => isArchived(a.session8));
  root.appendChild(groupHeader(`归档会话（${archivedSessions.length}）`, 'archivedCollapsed'));
  if (!state.explorer.archivedCollapsed) {
    if (!archivedSessions.length) root.appendChild(el('div', 'explorer-sub muted', '（无）'));
    for (const a of archivedSessions) {
      const when = a.archived_at ? ` · ${a.archived_at.slice(0, 16).replace('T', ' ')}` : '';
      const row = el('div', 'marker-entry');
      row.appendChild(el('span', 'session-id', a.session8));
      if (when) row.appendChild(el('span', 'session-when', when));
      row.title = `archive://${a.session8} —— 点击打开归档只读浏览`;
      row.addEventListener('click', () => ui.openUri(`archive://${a.session8}`));
      root.appendChild(row);
    }
  }
}

/* ── ContentPane ── */

let expandedTool = null; // pin-to-top 的工具行名

export function renderContent() {
  const scroll = ui.contentScroll;
  const nearBottom = scroll.scrollHeight - scroll.scrollTop - scroll.clientHeight < 40;
  scroll.replaceChildren();
  if (state.content.droppedItems > 0) {
    scroll.appendChild(
      el(
        'div',
        'system-line',
        `（更早的 ${state.content.droppedItems} 条已折叠出内存——渲染性能上限；完整事实在 journal，可用 run:// 回放查看）`,
      ),
    );
  }
  for (const item of state.content.items) {
    if (item.kind === 'message') {
      if (item.role === '系统') {
        scroll.appendChild(el('div', `system-line${item.warning ? ' warning' : ''}`, item.content));
      } else {
        const card = el('div', `msg-card${item.warning ? ' warning' : ''}${item.collapsed ? ' collapsed' : ''}`);
        const head = el('div', 'msg-head', `[${item.role}]${item.turn ? ` · turn ${item.turn}` : ''}`);
        head.addEventListener('click', () => {
          item.collapsed = !item.collapsed;
          renderContent();
        });
        const body = el('div', 'msg-body');
        body.innerHTML = item.role === '模型' ? renderMarkdown(item.content) : '';
        if (item.role !== '模型') body.textContent = item.content;
        card.appendChild(head);
        card.appendChild(body);
        scroll.appendChild(card);
      }
    } else {
      // 工具行（无边框）：[工具名] target · detail ▸ 展开/关闭
      const line = el('div', 'tool-line');
      const tag = el('span', 'tool-tag', `[${item.toolName}]`);
      const latest = item.entries[item.entries.length - 1] ?? { target: '', detail: '' };
      line.appendChild(tag);
      line.appendChild(document.createTextNode(`  ${latest.target}  ${latest.detail}  `));
      const exp = el('span', 'tool-expand', item.expanded ? '[关闭]' : `[展开]`);
      exp.addEventListener('click', () => {
        item.expanded = !item.expanded;
        expandedTool = item.expanded ? item.toolName : null;
        renderContent();
      });
      line.appendChild(exp);
      scroll.appendChild(line);
    }
  }
  scroll.appendChild(el('div', 'system-gap'));
  renderToolPin();
  if (nearBottom) scroll.scrollTop = scroll.scrollHeight;
}

function renderToolPin() {
  const pin = ui.toolPin;
  pin.replaceChildren();
  if (!expandedTool) {
    pin.hidden = true;
    return;
  }
  const line = state.content.items.find((it) => it.kind === 'tool' && it.toolName === expandedTool);
  if (!line) {
    expandedTool = null;
    pin.hidden = true;
    return;
  }
  pin.hidden = false;
  const omit = line.droppedEntries ? `（已省略最早 ${line.droppedEntries} 条）` : '';
  const title = el('div', 'tool-pin-title', `[${line.toolName}] 展开清单${omit}`);
  const close = el('span', 'tool-pin-close', '[关闭]');
  close.addEventListener('click', () => {
    line.expanded = false;
    expandedTool = null;
    renderContent();
  });
  title.appendChild(close);
  pin.appendChild(title);
  const ol = el('ol');
  line.entries.forEach((entry, i) => {
    ol.appendChild(el('li', null, `${i + 1}  ${entry.target}  ${entry.detail}`));
  });
  pin.appendChild(ol);
}

/* ── Marker ── */

export function renderMarker() {
  const root = ui.marker;
  root.replaceChildren();
  root.appendChild(el('div', null, '标记'));
  for (const m of state.marker) {
    // 锚点分级配色：▸ 输入（藏青）／◆ 输出（紫）／● 搜索命中（橙）。
    const row = el('div', `marker-entry mk-${m.kind}`, m.label);
    row.title = '点击跳转';
    row.addEventListener('click', () => ui.scrollToItem(m.itemIndex));
    root.appendChild(row);
  }
}

/* ── StatusBar ── */

export function renderStatus() {
  const root = ui.statusbar;
  root.replaceChildren();
  for (const s of state.status) {
    const span = el('span', `status-item${s.ok ? '' : ' bad'}`, s.label);
    root.appendChild(span);
  }
  if (state.gateBlock) {
    root.appendChild(el('span', 'status-item bad', `⚠ ${state.gateBlock}`));
  }
}

/* ── 全量重绘 ── */

export function renderAll() {
  renderMenubar();
  renderToolbar();
  renderExplorer();
  renderContent();
  renderMarker();
  renderStatus();
}
