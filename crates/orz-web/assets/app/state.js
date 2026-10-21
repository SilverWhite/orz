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
    // 318 P1-B2（2026-10-19）：打开旧会话默认只读回看，不再自动 load。
    // 只有点这个显式动作才恢复 ACP 会话并切换绑定（加载中变为「取消恢复」）。
    { id: 'resume', label: '恢复并继续', enabled: false },
    // 0dm（2026-10-21）：反例自查 PULL 入口——运行中经补充队列投递三问；
    // 空闲禁用（由 syncToolbar 按 state.running 同步）。
    { id: 'selfcheck', label: '反例自查', enabled: false },
    // 0dm S2（2026-10-21）：交付状态只读投影——按需查询侧车 plan 与最近
    // run 的终态/自查事实（不重算变更清单、不触发模型）。
    { id: 'delivery', label: '交付状态', enabled: true },
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
  content: { items: [], currentModelIndex: null, droppedItems: 0, droppedUserCards: 0 },
  marker: [], // {kind:'user'|'hit', label, itemIndex}
  explorerVisible: true,
  markerVisible: true,

  // ── 会话/运行 ──
  // 运行态单一 owner（318 P1-B4）：只有 main.js 的 prompt 生命周期与
  // 服务端 busy 重同步写 `running`；journal/projection 只产历史事实，绝不
  // 写状态栏的「运行中/空闲」。状态栏标签由 vm.setRunning() 同源派生。
  running: false,
  // 服务端报告「已有运行在途」但本客户端没有本地 prompt 句柄时的重同步标记
  // （318 P1-B4：status 不得与真实 busy 分叉）。
  serverBusy: false,
  // 会话恢复（session/load）加载门（318 P1-B3）：加载期间禁发 prompt、
  // 禁切会话/回 live；`navEpoch` 是导航/绑定代次，await 后必须复核。
  loadingSession: false,
  navEpoch: 0,
  // 显式「恢复并继续」目标（318 P1-B2）：{ s8, sessionId }；只在只读
  // conversation 视图且侧车带完整 ACP id 时非空。工具栏动作据此启停。
  resumeTarget: null,
  // 当前绑定会话 s8（main.js 维护；只读门与工具栏用；不代表可静默续接）。
  boundSession8: null,
  turnCounter: 0,
  // 已进入当前 run 模型面的补充数；失败/取消时提示 SUCCESS-ONLY 回滚。
  deliveredSupplements: 0,
  // 刷新/关闭风险面（318 P1-B7）：队列深度与后台任务事实。beforeunload
  // 与启动提示据此说明「仅文本上下文恢复，后台任务/队列不恢复」。
  supplementQueueDepth: 0,
  backgroundTasks: 0,
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
  // 2026-10-21：本桥会话内打开过的工作区（含启动 cwd 与切换历史）。
  // 不是持久授信；用于「切出去还能切回来」与旧工作区会话回看。
  recentWorkspaces: [],

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
    recentCollapsed: true,
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
    // 329（2026-10-21）：/rewind 的 ordinal 是「会话第几张用户卡」；内存裁剪
    // 后本地计数要加上被裁掉的用户卡数，否则可能把撤回目标数到错误的卡。
    let droppedUserCards = 0;
    for (let i = 0; i < drop; i++) {
      const it = items[i];
      if (it.kind === 'message' && it.role === '用户') droppedUserCards++;
    }
    items.splice(0, drop);
    state.content.droppedItems += drop;
    state.content.droppedUserCards = (state.content.droppedUserCards || 0) + droppedUserCards;
    if (state.content.currentModelIndex != null) {
      state.content.currentModelIndex -= drop;
      if (state.content.currentModelIndex < 0) state.content.currentModelIndex = null;
    }
    // 329（2026-10-21）：裁掉的条目其锚点必须一并移除——旧写法把越界下标
    // 钳到 0，标记栏会跳到最后也指向错卡；存活锚点统一下移 drop。
    state.marker = state.marker.filter((m) => m.itemIndex >= drop);
    for (const m of state.marker) m.itemIndex -= drop;
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
    // 329（2026-10-21）：锚描述同时挂到条目上（item.anchor），截断/重放后
    // 可由 vm.rebuildMarkers() 按当前 items 下标重建标记栏。
    const item = items[items.length - 1];
    item.anchor = {
      kind: 'user',
      turn: state.turnCounter,
      label: `▸ T${state.turnCounter} 输入`,
    };
    state.marker.push({ ...item.anchor, itemIndex: items.length - 1 });
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
        // 329（2026-10-21）：锚描述随条目存一份，供标记栏重建复用。
        it.anchor = { kind: 'model', turn: it.turn, label: `◆ T${it.turn} 最终输出` };
        state.marker.push({ ...it.anchor, itemIndex: i });
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
      const item = state.content.items[state.content.items.length - 1];
      item.anchor = {
        kind: 'user',
        turn: turn || 0,
        label: `▸ T${turn || 0} 输入`,
      };
      state.marker.push({
        ...item.anchor,
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

  // 运行终态折叠（CONTENT_PANE「折叠策略」：只折机器行）——2026-10-16 用户令
  // 修正（285 批 F1）：此前连对话卡一并折起，真机走查实证「一整段会话被折起、
  // 要逐张手动展开」。对话卡（用户/模型）不再折；只有机器行（工具清单）
  // 收尾折叠，阻塞/告警面本就不折。
  collapseNonWarnings() {
    for (const it of state.content.items) {
      if (it.kind === 'tool') it.expanded = false;
    }
    state.content.currentModelIndex = null;
  },

  // 反例门静默折叠（2026-10-16 用户令，285 批 F2）：反例问询前的模型输出是
  // 待自查草稿、不是正式输出——门到即把本运行（本 turn）此前的模型卡与全部
  // 机器行静默折叠（无提示行、不留痕），只留门后的正式输出；用户输入卡不折
  // （提问要看得见）。真机走查实证：门前的整篇草稿此前照常展开渲染。
  foldSupersededBeforeGate() {
    const turn = state.turnCounter;
    for (const it of state.content.items) {
      if (it.kind === 'tool') it.expanded = false;
      else if (it.kind === 'message' && it.role === '模型' && !it.warning && it.turn === turn) {
        it.collapsed = true;
      }
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

  /* 329（2026-10-21）：按当前 items 重建标记栏下标。撤回截断/合并重放后
   * items 已变，历史 marker.itemIndex 是数组下标、会漂移或越界（0.9.7
   * 真机「标记栏显示与定位均错、旧消息无法定位」的直接原因）。锚语义保持
   * 不变：用户输入锚按用户卡恢复；最终输出锚随条目存 item.anchor；搜索
   * 命中锚按下标保留，越界即丢（内容已不在视图）。 */
  rebuildMarkers() {
    const rebuilt = [];
    state.content.items.forEach((item, idx) => {
      // 用户输入锚可从条目角色直接恢复（旧投影/手工装填的条目也能重建）；
      // 最终输出锚只能沿用终态时记在条目上的描述。
      const anchor =
        item.anchor ||
        (item.kind === 'message' && item.role === '用户'
          ? { kind: 'user', turn: item.turn || 0, label: `▸ T${item.turn || 0} 输入` }
          : null);
      if (!anchor) return;
      rebuilt.push({ ...anchor, itemIndex: idx });
    });
    for (const m of state.marker) {
      if (m.kind !== 'hit') continue;
      if (m.itemIndex >= 0 && m.itemIndex < state.content.items.length) rebuilt.push({ ...m });
    }
    state.marker = rebuilt;
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
  // 运行态单一写入口（318 P1-B4）：state.running 与状态栏标签同源。
  // journal 回放/终态事件只经 addSystemMessage 给历史事实，不调用本函数。
  setRunning(running) {
    state.running = !!running;
    vm.setRunState(state.running ? '运行中' : '空闲', true);
    vm.syncToolbar();
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
    state.navEpoch++;
    vm.syncToolbar();
  },
  navBack() {
    if (!state.nav.back.length) return;
    state.nav.forward.push(state.nav.current);
    state.nav.current = state.nav.back.pop();
    state.navEpoch++;
    vm.syncToolbar();
  },
  navForward() {
    if (!state.nav.forward.length) return;
    state.nav.back.push(state.nav.current);
    state.nav.current = state.nav.forward.pop();
    state.navEpoch++;
    vm.syncToolbar();
  },
  syncToolbar() {
    const btn = (id) => state.toolbar.find((b) => b.id === id);
    btn('back').enabled = state.nav.back.length > 0 && !state.loadingSession;
    btn('forward').enabled = state.nav.forward.length > 0 && !state.loadingSession;
    btn('stop').enabled = state.running;
    btn('selfcheck').enabled = state.running;
    btn('new').enabled = !state.running && !state.loadingSession;
    // 只在「查看某会话且该会话不是当前绑定」且侧车/归档带完整 id 时可恢复。
    // 2026-10-21 恢复修复：archive:// 视图同样可恢复（host 侧 load 自动回档）。
    const viewed = state.nav.current.startsWith('conversation://')
      ? state.nav.current.slice('conversation://'.length)
      : state.nav.current.startsWith('archive://')
        ? state.nav.current.slice('archive://'.length)
        : null;
    const rt = state.resumeTarget;
    btn('resume').enabled =
      state.loadingSession ||
      (!state.running && !!rt?.sessionId && rt.s8 === viewed);
  },
};
