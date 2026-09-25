/*
 * keymap.js —— 键盘模型（补充稿 §10：F6 切大区、Tab/Shift+Tab 区内移动、
 * Esc 关弹窗、Ctrl+L 命令、Ctrl+F 查找、Alt+字母 开菜单、Alt+H/F1 帮助、
 * Ctrl+Z 取消运行、双 Esc 运行历史、← → 切 Help 标签）。
 * （审查处理批 D3：补 Tab 区内移动与 Alt+字母菜单访问键。）
 */

let actions = {};

export function bindKeyActions(a) {
  actions = a;
}

let lastEsc = 0;

// Alt+字母 → 菜单访问键（九菜单；Alt+H 例外——直达帮助内容页）。
const MENU_ACCESS = {
  f: '文件',
  e: '事件',
  k: '标记',
  d: '编辑模式',
  o: '模型',
  s: '来源',
  r: '运行',
  v: '验证',
};

export function handleKey(e) {
  // 输入框内优先保留原生编辑键；提交/换行由 input 行自己处理。
  const inInput = e.target && (e.target.tagName === 'TEXTAREA' || e.target.tagName === 'INPUT');

  if (e.key === 'Escape') {
    const now = Date.now();
    actions.closeModal?.();
    if (now - lastEsc < 500) {
      // 双 Esc：运行历史列表（中立焦点 → 探索器）。
      actions.focusExplorer?.();
      lastEsc = 0;
    } else {
      lastEsc = now;
    }
    e.preventDefault();
    return;
  }
  if (e.key === 'F6') {
    actions.cycleFocus?.();
    e.preventDefault();
    return;
  }
  if (e.key === 'Tab' && !inInput) {
    // 区内焦点移动（补充稿 §10）；Shift+Tab 反向。
    actions.cycleFocusInRegion?.(e.shiftKey ? -1 : 1);
    e.preventDefault();
    return;
  }
  if (e.key === 'F1' || (e.altKey && (e.key === 'h' || e.key === 'H'))) {
    actions.openHelp?.();
    e.preventDefault();
    return;
  }
  if (e.altKey && !inInput) {
    const menu = MENU_ACCESS[e.key.toLowerCase()];
    if (menu) {
      actions.openMenu?.(menu);
      e.preventDefault();
      return;
    }
  }
  if ((e.ctrlKey || e.metaKey) && (e.key === 'l' || e.key === 'L')) {
    actions.openCommand?.();
    e.preventDefault();
    return;
  }
  if ((e.ctrlKey || e.metaKey) && (e.key === 'f' || e.key === 'F')) {
    actions.openFind?.();
    e.preventDefault();
    return;
  }
  if ((e.ctrlKey || e.metaKey) && (e.key === 'z' || e.key === 'Z')) {
    actions.stopRun?.();
    e.preventDefault();
    return;
  }
  if (!inInput && e.key === '/') {
    actions.openCommand?.('/');
    e.preventDefault();
    return;
  }
  // 弹窗内 ← → 切换 Help 标签
  if (e.key === 'ArrowRight' || e.key === 'ArrowLeft') {
    actions.cycleHelpTab?.(e.key === 'ArrowRight' ? 1 : -1);
  }
}

export function install() {
  document.addEventListener('keydown', handleKey);
}
