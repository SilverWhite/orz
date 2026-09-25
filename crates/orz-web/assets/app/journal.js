/*
 * journal.js —— journal 投影通道（桥 /ws/journal/{run_id} 活动尾 ＋
 * /api/run/{id}/events 历史切片）。 RunEvent 为 JSONL 原样转发，前端按
 * event_type（snake_case，journal/event.rs serde 口径）分发到 projection。
 */

import { fetchRuns, fetchRunEvents } from './api.js';

let tailWs = null;

export async function refreshRuns() {
  const data = await fetchRuns();
  return data.runs || [];
}

/**
 * 拉取一个历史 run 的全部事件（分片循环到 EOF）。
 * onEvent(event) 逐事件回调；返回事件总数。
 * 字节上限 REPLAY_MAX_BYTES（审查处理批 F-1）：超大 run 回放到上限即止
 * ——渲染内存另有条目上限，超限部分经 run:// 分段回放查看。
 */
export const REPLAY_MAX_BYTES = 64 * 1024 * 1024;

export async function replayRun(runId, onEvent, onProgress) {
  let from = 0;
  let total = 0;
  for (;;) {
    const page = await fetchRunEvents(runId, from);
    for (const ev of page.lines || []) {
      total++;
      onEvent(ev);
    }
    onProgress?.(total);
    const next = page.next_offset;
    if (next == null || next <= from || next > REPLAY_MAX_BYTES) break;
    from = next;
  }
  return total;
}

/**
 * 订阅活动 run 的 journal 尾。onEvent 收到每个完整事件行（对象）；
 * 返回关闭函数。
 */
export function tailRun(runId, fromOffset, onEvent, onClosed) {
  closeTail();
  const proto = location.protocol === 'https:' ? 'wss:' : 'ws:';
  const token = new URLSearchParams(location.hash.replace(/^#/, '')).get('token') || '';
  tailWs = new WebSocket(
    `${proto}//${location.host}/ws/journal/${encodeURIComponent(runId)}?token=${encodeURIComponent(token)}&from=${fromOffset || 0}`,
  );
  tailWs.onmessage = (ev) => {
    try {
      onEvent(JSON.parse(ev.data));
    } catch {
      /* 非事件行（如 tail 控制行）忽略 */
    }
  };
  tailWs.onclose = () => onClosed?.();
  return closeTail;
}

export function closeTail() {
  if (tailWs) {
    tailWs.close();
    tailWs = null;
  }
}

/** 取最新 run id（活动跟随用；无 run 时返回 null）。 */
export async function latestRunId() {
  const runs = await refreshRuns();
  return runs.length ? runs[0].run_id : null;
}
