/*
 * copy.js —— 上浮错误的四要素文案表（333，2026-10-21，S4/S11）。
 *
 * 设计依据：0dk S1 §9（每条上浮错误＝「发生了什么＋为什么（分类）＋下一步＋
 * 证据在哪」）；分类闭集＝connection/session/permission/provider/tool/internal。
 * 工具类错误**不上浮**（留在对话里由模型处理），本表因此不收录 tool。
 *
 * 宿主在 `error.data` 里带 `{code,class,hint,evidence}`；本模块优先用宿主
 * 的 hint/evidence（更具体），表只补「发生了什么/默认下一步」——未知 code
 * 一律走 internal 兜底，绝不吞成一句 "Internal error"。
 */

export const ERROR_COPY = {
  session_not_found: {
    what: '会话不存在或已被删除',
    class: 'session',
    hint: '新建会话，或从探索器选择归档后「恢复并继续」',
  },
  session_busy: {
    what: '会话正忙：上一轮回复尚未收束',
    class: 'session',
    hint: '等待当前回复结束（或点「停止」）后重试',
  },
  invalid_state: {
    what: '当前状态不允许这个操作',
    class: 'session',
    hint: '先结束当前操作（停止/等待回复），再重试',
  },
  lease_conflict: {
    what: '会话被另一个 orz 进程占用',
    class: 'session',
    hint: '关闭占用该会话的其他窗口/进程后重试',
  },
  attach_conflict: {
    what: '工作区已有另一个窗口在接收会话',
    class: 'connection',
    hint: '关闭其他 orz 窗口后刷新本页',
  },
  connection_closed: {
    what: '与本地桥的连接已断开',
    class: 'connection',
    hint: '刷新页面重连；保活窗口内可直接续接',
  },
  permission_denied: {
    what: '权限请求被拒绝或超时',
    class: 'permission',
    hint: '重试并在弹窗中允许；超时按 fail-closed 处理',
  },
  provider_error: {
    what: '模型请求失败（网络/额度/供应商）',
    class: 'provider',
    hint: '检查网络或余额后重试',
  },
  internal: {
    what: '内部错误',
    class: 'internal',
    hint: '重试；仍失败请记录证据并重启 orz web',
  },
};

/** 四要素合成：发生了什么＋为什么（分类）＋下一步＋证据。 */
export function describeError(data, fallback = 'internal') {
  const d = data && typeof data === 'object' ? data : {};
  const entry = ERROR_COPY[d.code] || ERROR_COPY[fallback] || ERROR_COPY.internal;
  const what = entry.what;
  const cls = d.class || entry.class;
  const hint = d.hint || entry.hint;
  const evidence = d.evidence || '（未提供）';
  return `${what}（${cls}）。→ ${hint}；证据：${evidence}`;
}
