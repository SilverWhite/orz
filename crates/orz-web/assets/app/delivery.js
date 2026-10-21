/*
 * delivery.js —— 0dm S2「交付状态」面板渲染件（329，2026-10-21）。
 *
 * 边界（设计 §4.2）：`submit` 已从模型面退役，交付由 `[RUN_END]`
 * （intent=conclude/reason=completed）机械完成；`compute_delivery_status`
 * 是 loop 运行时函数（依赖进程内的计划批准基线），空闲 UI 不可复算。
 * 所以本模块只把 `/api/delivery` 的只读事实投影渲染成表格——不写 state、
 * 不发请求、不触发模型；请求与生命周期在 dialogs.js 的
 * `openDeliveryStatus()`。
 */

/* 步骤状态中文标注（键与 orz-loop blackboard::StepStatus 的 serde 输出同形）。 */
const STEP_STATES = {
  pending: '待办',
  in_progress: '进行中',
  done: '已完成',
  failed: '失败',
  blocked: '阻塞',
};

/* 0dm 终态来源闭集（run_finished.terminal_source）。 */
const TERMINAL_SOURCES = {
  run_end: '模型 [RUN_END] 声明',
  text_without_terminal: '纯文本终答，无声明（fail-open）',
  short_text: '短聊，声明豁免',
  budget_exhausted: '预算耗尽',
  gate_block: '门控阻断',
  close_round: '收束轮',
  unspecified: '未标注',
};

/* 最近 run 的 status（run_finished/run_failed/run_cancelled/…）。 */
const RUN_STATUS = {
  completed: '完成',
  failed: '失败',
  cancelled: '已取消',
  invalidated: '无效',
  terminated: '已终止',
  running: '未收尾',
};

function plain(value) {
  return value === null || value === undefined || value === '' ? '—' : String(value);
}

function yesNo(value) {
  if (value === null || value === undefined) return '—';
  return value ? '是' : '否';
}

/* 闭集码值带原文标注（面板是事实面，中文标签不得替代键名）。 */
function codeLabel(raw, labels) {
  if (raw === null || raw === undefined || raw === '') return '—';
  const label = labels[raw];
  return label ? `${raw}（${label}）` : String(raw);
}

function mutedLine(body, text) {
  const div = document.createElement('div');
  div.className = 'muted';
  div.style.margin = '2px 0';
  div.textContent = text;
  body.appendChild(div);
  return div;
}

function factsTable(body, rows) {
  const table = document.createElement('table');
  table.className = 'props-table';
  for (const [k, v] of rows) {
    const tr = document.createElement('tr');
    const tdKey = document.createElement('td');
    tdKey.textContent = k;
    const tdValue = document.createElement('td');
    tdValue.textContent = v;
    tr.appendChild(tdKey);
    tr.appendChild(tdValue);
    table.appendChild(tr);
  }
  body.appendChild(table);
  return table;
}

/* 0dm S2（2026-10-21）：交付状态事实渲染（纯函数：只读 data，不写 state）。
 * data ＝ `/api/delivery` 响应；字段缺失一律按「—」呈现，空态显式提示。 */
export function renderDeliveryStatus(body, data) {
  const d = data || {};
  const plan = d.plan || {};
  const steps = Array.isArray(plan.steps) ? plan.steps : [];
  const lastRun = d.last_run || null;
  const recall = d.recall || {};
  const recallCount = Number.isFinite(recall.count) ? recall.count : 0;

  const heading = document.createElement('div');
  heading.style.fontWeight = 'bold';
  heading.style.margin = '2px 0 6px';
  heading.textContent = '交付事实（只读投影，不重算变更清单）';
  body.appendChild(heading);

  const empty =
    (d.messages === null || d.messages === undefined) && !steps.length && !lastRun;
  if (empty) {
    mutedLine(body, '（该会话尚无可展示的交付事实：会话侧车与 run journal 均未找到）');
  }

  factsTable(body, [
    ['会话（session8）', plain(d.session8)],
    ['ACP 会话 ID', plain(d.session_id)],
    ['工作区根', plain(d.root)],
    ['消息数', plain(d.messages)],
    [
      'Plan 末步',
      plan.terminal_step
        ? `${plan.terminal_step}（${STEP_STATES[plan.terminal_state] || plain(plan.terminal_state)}）`
        : '—（无 plan，或末步不是 deliver/submit）',
    ],
    [
      '交付状态（侧车存档）',
      plan.delivery_status
        ? String(plan.delivery_status)
        : '—（未存档；本面板不重算变更清单基线）',
    ],
    [
      '撤回审计',
      recallCount > 0
        ? `${recallCount} 次（最后撤回序号 #${plain(recall.last_ordinal)}；只截断消息面，plan 不回滚）`
        : '无撤回记录',
    ],
  ]);

  const planCaption = document.createElement('div');
  planCaption.className = 'muted';
  planCaption.style.margin = '8px 0 2px';
  planCaption.textContent = steps.length ? `Plan 步骤（${steps.length}）` : 'Plan 步骤（无）';
  body.appendChild(planCaption);
  for (const [i, step] of steps.entries()) {
    const row = document.createElement('div');
    row.textContent = `${i + 1}. ${plain(step.id)} — ${STEP_STATES[step.state] || plain(step.state)}`;
    row.style.margin = '1px 0';
    body.appendChild(row);
  }

  const runCaption = document.createElement('div');
  runCaption.className = 'muted';
  runCaption.style.margin = '8px 0 2px';
  runCaption.textContent = lastRun
    ? `最近 run（${plain(lastRun.run_id)}）`
    : '最近 run（无 run journal）';
  body.appendChild(runCaption);
  if (lastRun) {
    factsTable(body, [
      ['status', codeLabel(lastRun.status, RUN_STATUS)],
      ['terminal_source', codeLabel(lastRun.terminal_source, TERMINAL_SOURCES)],
      ['terminal_nudged', yesNo(lastRun.terminal_nudged)],
      ['terminal_nudge_count', plain(lastRun.terminal_nudge_count)],
      ['self_check_called', yesNo(lastRun.self_check_called)],
      ['self_check 完成次数', plain(lastRun.self_check_tool_rounds)],
    ]);
  } else {
    mutedLine(body, '该会话尚无 run journal：交付事实只来自会话侧车（run 面为空）。');
  }

  const notes = Array.isArray(d.notes) ? d.notes : [];
  const notesCaption = document.createElement('div');
  notesCaption.className = 'muted';
  notesCaption.style.margin = '8px 0 2px';
  notesCaption.textContent = '边界与说明';
  body.appendChild(notesCaption);
  if (notes.length) {
    for (const note of notes) {
      mutedLine(body, `· ${note}`);
    }
  } else {
    mutedLine(body, '（无）');
  }
}
