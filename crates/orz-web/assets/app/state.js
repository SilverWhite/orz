/*
 * state.js —— 视图模型（自研登记：0br S1 §4.2 R-5）。
 * 交互语义移植自 orz-tui view_model.rs：
 *   ContentItem = Message（有边框卡片）| ToolTrace（无边框工具行）
 *   StatusBar 冻结五段（守护/网络关闭/工具 n/m/模型/运行态；「沙箱严格」段
 *   已随 2026-09-25 用户令去除——生产面无 OS 沙箱（orz-sandbox 未接线，
 *   真机 Job 只约束派生进程树），保留即失真假面；orz-tui 母本同步留待
 *   TUI 后补批）
 *   MenuBar/Toolbar 冻结中文词表；nav 对象历史栈（R-4）。
 * 纯数据 + 变更函数；渲染在 widgets.js。
 */

export const state = {
  // ── 视图模型 ──
  // 「编辑模式」：补充稿 §13 冻结转换项（Edit→编辑模式）；顶部中文菜单
  // 最终词汇仍在 §13 延期面，S3 词汇定稿收口。
  menu: ['文件', '事件', '标记', '编辑模式', '模型', '来源', '运行', '验证', '帮助'],
  toolbar: [
    // 「新建会话」居首（用户令 2026-09-25「放在后退的前面」）：归档当前
    // 会话后由它开启下一段对话——桥为单会话绑定，没有它归档流程无法收尾。
    { id: 'new', label: '新建会话', enabled: true },
    { id: 'back', label: '后退', enabled: false },
    { id: 'forward', label: '前进', enabled: false },
    { id: 'refresh', label: '刷新', enabled: true },
    { id: 'stop', label: '停止', enabled: false },
    { id: 'open', label: '打开', enabled: true },
    { id: 'verify', label: '验证', enabled: false },
    { id: 'properties', label: '属性', enabled: true },
  ],
  status: [
    { label: '守护', ok: true },
    { label: '网络关闭', ok: true },
    { label: '工具 0/0', ok: true },
    { label: '模型', ok: true },
    { label: '空闲', ok: true },
  ],
  // 内容条目：{kind:'message', role, content, turn, warning, collapsed}
  //        或 {kind:'tool', toolName, entries:[{target, detail}], expanded}
  // droppedItems>0 表示最旧条目已裁出内存（性能上限登记：完整事实在
  // journal，渲染面按上限折叠——设计豁免见综合稿 §8）。
  content: { items: [], currentModelIndex: null, droppedItems: 0 },
  marker: [], // {kind:'user'|'hit', label, itemIndex}
  explorerVisible: true,
  markerVisible: true,

  // ── 会话/运行 ──
  running: false,
  turnCounter: 0,
  sessionId: null,
  gateBlock: null,
  pendingPermission: null, // {request, respond} 由 acp.js 注入
  modelLabel: '模型',
  // 实时窗口正跟随的 run id（journal 尾通道）——「进行中」唯一判据：
  // 只有真正在动作/流式输出的窗口才算进行中（用户令 2026-09-25）。
  liveRunId: null,
  // 工作区事实（/api/boot）：当前 cwd＋用户全局信任存储的授予清单。
  workspaceCwd: '',
  trustedWorkspaces: [],

  // ── 对象导航历史（AddressBar 语义，R-4） ──
  nav: { current: 'workspace://live', back: [], forward: [] },

  // ── 运行历史（Explorer 数据源） ──
  runs: [],
  conversations: [],
  // 会话归档清单（0br S3 新增面：.gsa/archives 只读投影）。
  archives: [],
  // 探索器选中项（批六用户令 2026-09-25：组级功能键的操作对象）——
  // { group: 'active'|'archived', s8 }；单击选中、双击打开。
  selected: null,

  // ── 探索器（用户令 2026-09-25：三组＝工作区／活跃会话／归档会话；
  // 工作区组两级子级＝当前工作区＋已信任工作区） ──
  explorer: {
    workspaceCollapsed: false,
    trustedCollapsed: true,
    activeCollapsed: false,
    archivedCollapsed: false,
  },
};

const RUN_STATES = ['空闲', '预检', '运行中', '等待审批', '完成', '失败', '已取消', '无效', '恢复中'];

// 渲染内存上限（审查处理批 F-1）：条目与单工具行清单各自封顶，超出裁最旧
// 并计数（UI 顶部提示）。这不是对话内容截断——完整事实始终在 journal。
export const MAX_CONTENT_ITEMS = 3000;
export const MAX_TOOL_ENTRIES = 2000;

function pushContentItem(item) {
  const items = state.content.items;
  items.push(item);
  if (items.length > MAX_CONTENT_ITEMS) {
    const drop = items.length - MAX_CONTENT_ITEMS;
    items.splice(0, drop);
    state.content.droppedItems += drop;
    if (state.content.currentModelIndex != null) {
      state.content.currentModelIndex -= drop;
      if (state.content.currentModelIndex < 0) state.content.currentModelIndex = null;
    }
    for (const m of state.marker) m.itemIndex = Math.max(0, m.itemIndex - drop);
  }
}

function sanitizeText(s) {
  // 边界净化（view_model.rs sanitize 语义）：去掉终端转义序列。
  // Web 侧另由 md.js 做 HTML 逃逸（安全由构造保证）。
  // eslint-disable-next-line no-control-regex
  return String(s ?? '').replace(/\x1b\[[0-9;]*[A-Za-z]/g, '').replace(/[\x00-\x08\x0b\x0c\x0e-\x1f]/g, '');
}

export const vm = {
  sanitizeText,

  // ── ContentPane（content 类方法照 orz-tui ContentPane impl） ──
  addUserMessage(text) {
    const items = state.content.items;
    // 去重窗口＝最近一条用户消息（journal 双投递防护；审查处理批 F-2：
    // 不再对全历史去重——隔轮发送相同消息是合法行为，必须显示）。
    for (let i = items.length - 1; i >= 0; i--) {
      const it = items[i];
      if (it.kind === 'message' && it.role === '用户') {
        if (it.content === text) return;
        break;
      }
    }
    pushContentItem({ kind: 'message', role: '用户', content: sanitizeText(text), turn: state.turnCounter, warning: false, collapsed: false });
    // 标记锚点：每条用户输入一个（走查反馈 2026-09-25：锚定输入与输出）。
    state.marker.push({
      kind: 'user',
      turn: state.turnCounter,
      label: `▸ T${state.turnCounter} 输入`,
      itemIndex: items.length - 1,
    });
  },

  addModelMessage(text, turn, warning) {
    pushContentItem({ kind: 'message', role: '模型', content: sanitizeText(text), turn, warning: !!warning, collapsed: false });
    state.content.currentModelIndex = state.content.items.length - 1;
    // 中间轮/流式段不产锚点（走查反馈二 2026-09-25：只标最终正式输出）；
    // 终态时由 anchorFinalOutput() 统一回溯定锚。
  },

  // 终态（run_finished/failed/cancelled/…）时调用：为本运行最后一张有
  // 正文的模型卡定「最终输出」锚。合并会话视图（走查反馈六）下各运行的
  // 锚并存——每次对话得到各自的正式输出定位；同卡重复终态去重。
  anchorFinalOutput() {
    for (let i = state.content.items.length - 1; i >= 0; i--) {
      const it = state.content.items[i];
      if (it.kind === 'message' && it.role === '模型' && it.content && it.content.trim()) {
        const last = state.marker[state.marker.length - 1];
        if (last && last.kind === 'model' && last.itemIndex === i) return;
        state.marker.push({ kind: 'model', turn: it.turn, label: `◆ T${it.turn} 最终输出`, itemIndex: i });
        return;
      }
    }
  },

  ensureCurrentModelCard() {
    if (state.content.currentModelIndex == null) {
      pushContentItem({ kind: 'message', role: '模型', content: '', turn: state.turnCounter, warning: false, collapsed: false });
      state.content.currentModelIndex = state.content.items.length - 1;
    }
  },

  appendTextDelta(text) {
    vm.ensureCurrentModelCard();
    const card = state.content.items[state.content.currentModelIndex];
    card.content = sanitizeText(card.content + text);
  },

  addSystemMessage(text, warning) {
    pushContentItem({ kind: 'message', role: '系统', content: sanitizeText(text), turn: state.turnCounter, warning: !!warning, collapsed: false });
  },

  // 归档转写卡（0br S3 新增面）：静态只读装填，无去重——转写是从归档包
  // 整体装填的投影，不是流式事实通道。
  // 批四（2026-09-25 用户令「归档会话中能也使标记栏可用吗」）：用户输入
  // 产 ▸ 锚与实时视图同语义；模型卡不逐轮产锚，由 anchorFinalOutput
  // 统一回溯定「最终输出」锚。
  addArchivedMessage(role, content, turn) {
    pushContentItem({ kind: 'message', role, content: sanitizeText(content), turn: turn || 0, warning: false, collapsed: false });
    if (role === '用户') {
      state.marker.push({
        kind: 'user',
        turn: turn || 0,
        label: `▸ T${turn || 0} 输入`,
        itemIndex: state.content.items.length - 1,
      });
    }
  },

  // 工具行：同工具追加条目，折叠态滚到最新（三稿 §3.2/§5）；清单按
  // MAX_TOOL_ENTRIES 封顶，裁最旧计数（展开区标题显示省略数）。
  addOrUpdateToolTrace(toolName, target, detail) {
    let line = state.content.items.find((it) => it.kind === 'tool' && it.toolName === toolName);
    if (!line) {
      line = { kind: 'tool', toolName, entries: [], expanded: false, droppedEntries: 0 };
      pushContentItem(line);
    }
    line.entries.push({ target: sanitizeText(target), detail: sanitizeText(detail) });
    if (line.entries.length > MAX_TOOL_ENTRIES) {
      const drop = line.entries.length - MAX_TOOL_ENTRIES;
      line.entries.splice(0, drop);
      line.droppedEntries += drop;
    }
  },

  updateToolLatest(toolName, detail) {
    const line = state.content.items.find((it) => it.kind === 'tool' && it.toolName === toolName);
    if (line && line.entries.length) line.entries[line.entries.length - 1].detail = sanitizeText(detail);
  },

  collapseNonWarnings() {
    for (const it of state.content.items) {
      if (it.kind === 'tool') it.expanded = false;
      else if (it.role !== '系统' || !it.warning) it.collapsed = true;
    }
    state.content.currentModelIndex = null;
  },

  summarizeArguments(args) {
    const compact = sanitizeText(args).replace(/[\n\r]/g, ' ');
    return compact.length > 60 ? compact.slice(0, 60) + '…' : compact;
  },

  addSearchHit(label, itemIndex) {
    state.marker.push({ kind: 'hit', label: `● ${label}`, itemIndex });
  },

  clearHits() {
    // 只清搜索命中；输入/输出锚点保留（走查反馈 2026-09-25）。
    state.marker = state.marker.filter((m) => m.kind !== 'hit');
  },

  // ── StatusBar ──
  statusUpdate(prefix, ok) {
    for (const it of state.status) if (it.label.startsWith(prefix)) it.ok = ok;
  },
  setToolProbe(complete, total) {
    for (const it of state.status) {
      if (it.label.startsWith('工具')) {
        it.label = `工具 ${complete}/${total}`;
        it.ok = complete === total;
      }
    }
  },
  setRunState(label, ok) {
    if (!RUN_STATES.includes(label)) return;
    for (const it of state.status) if (RUN_STATES.includes(it.label)) { it.label = label; it.ok = ok; }
  },
  setModel(label) {
    for (const it of state.status) if (it.label === state.modelLabel || ['模型', 'off'].includes(it.label)) it.label = label;
    state.modelLabel = label;
  },

  // ── 对象历史（AddressBar Back/Forward） ──
  navGo(uri) {
    if (uri === state.nav.current) return;
    state.nav.back.push(state.nav.current);
    state.nav.forward = [];
    state.nav.current = uri;
    vm.syncToolbar();
  },
  navBack() {
    if (!state.nav.back.length) return;
    state.nav.forward.push(state.nav.current);
    state.nav.current = state.nav.back.pop();
    vm.syncToolbar();
  },
  navForward() {
    if (!state.nav.forward.length) return;
    state.nav.back.push(state.nav.current);
    state.nav.current = state.nav.forward.pop();
    vm.syncToolbar();
  },
  syncToolbar() {
    const btn = (id) => state.toolbar.find((b) => b.id === id);
    btn('back').enabled = state.nav.back.length > 0;
    btn('forward').enabled = state.nav.forward.length > 0;
    btn('stop').enabled = state.running;
  },
};
