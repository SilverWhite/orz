/*
 * projection.js —— journal RunEvent → 视图模型（自研登记：0br S1 §4.2 R-5）。
 * 事件分发语义移植 orz-tui bridge.rs（防御式取字段）＋ projection.rs（中文
 * 文案与状态迁移）；event_type 为 journal/event.rs serde snake_case 口径。
 * 未知事件降级为无操作（不进内容面，避免噪声）。
 */

import { state, vm } from './state.js';

function str(p, k) {
  const v = p?.[k];
  return typeof v === 'string' ? v : '';
}
function num(p, k) {
  const v = p?.[k];
  return typeof v === 'number' ? v : 0;
}
function optStr(p, k) {
  const v = p?.[k];
  return typeof v === 'string' ? v : null;
}
function toolCalls(p) {
  const arr = p?.tool_calls;
  if (!Array.isArray(arr)) return [];
  return arr.map((tc) => ({
    name: str(tc, 'name'),
    arguments: str(tc, 'arguments'),
    call_id: str(tc, 'call_id'),
  }));
}

/* 2026-10-21 恢复修复：会话恢复后标记栏的 turn 号必须用 run 的会话序号，
 * 不能从本地 0 重新累加——恢复后 tail 会重放“最新一个 run”，若从 0 起算，
 * 该 run 的用户输入会被标成 T1（真机 RUN-c23e0aaa-18 实际是 T19）。
 * run id 是会话级事实：`RUN-{s8}-{n}` 的 n 即 prompt_number，turn = n+1。
 * `RUN-CLI-{s8}` 等无序号形态回退本地累加（保持旧语义）。 */
function runTurn(ev) {
  const m = /^RUN-[0-9a-fA-F]{8}-(\d+)$/.exec(ev?.run_id || '');
  return m ? Number(m[1]) + 1 : null;
}

function discloseRolledBackSupplements() {
  const count = state.deliveredSupplements;
  state.deliveredSupplements = 0;
  if (count > 0) {
    vm.addSystemMessage(
      `[提示] 本次运行已投递的 ${count} 条补充未写回会话；请按需重新发送`,
      true,
    );
  }
}

export function applyJournalEvent(ev) {
  const p = ev.payload ?? {};
  switch (ev.event_type) {
    case 'run_preflight':
      // 318 P1-B4：journal 只给历史事实，不写状态栏运行态（单一 owner 在
      // main.js 的 prompt 生命周期；这里写「预检/运行中」会与 state.running
      // 分叉，回放旧 run 时尤其会留下永久「运行中」）。
      vm.statusUpdate('守护', true);
      vm.addSystemMessage('预检通过', false);
      break;

    case 'run_started':
      state.deliveredSupplements = 0;
      state.gateBlock = null;
      break;

    case 'prompt_submitted': {
      const turn = runTurn(ev);
      state.turnCounter = turn ?? state.turnCounter + 1;
      vm.addUserMessage(str(p, 'prompt'));
      break;
    }

    /* 0di：中途补充投递——运行中排队的用户输入在主车道 loop-top 生效；
     * 渲染为用户卡（与 prompt_submitted 同面，投递即可见确认）。 */
    case 'supplement_delivered':
      state.turnCounter++;
      state.deliveredSupplements++;
      if (state.supplementQueueDepth > 0) state.supplementQueueDepth--;
      vm.addUserMessage(str(p, 'text'));
      break;

    case 'model_request':
      vm.setModel(str(p, 'model_id') || '模型');
      break;

    case 'model_response_received':
      break; // 与 model_output 的工具行面合并，避免双计

    case 'model_output': {
      const turn = state.turnCounter;
      const calls = toolCalls(p);
      for (const tc of calls) {
        const target = vm.summarizeArguments(tc.arguments);
        vm.addOrUpdateToolTrace(tc.name, target, tc.name === 'bash' ? target : '...');
      }
      // 流式去重（projection.rs P1-3）：卡片内容与 journal 全文精确一致
      // ⇒ 只复位流式指针；不同文本 ⇒ 新卡片（工具轮后的下一段输出）。
      const text = str(p, 'text');
      const idx = state.content.currentModelIndex;
      const streamed =
        idx != null &&
        state.content.items[idx]?.kind === 'message' &&
        state.content.items[idx].role === '模型' &&
        state.content.items[idx].content === text;
      if (streamed) {
        state.content.currentModelIndex = null;
      } else if (text) {
        vm.addModelMessage(text, turn, false);
      } else {
        // 纯工具调用、无正文的轮次：不留空白对话卡（走查反馈 2026-09-25
        // 实证：journal 中空正文 model_output 与空白卡一一对应）。
        state.content.currentModelIndex = null;
      }
      break;
    }

    case 'request_header_change': {
      const reason = str(p, 'reason');
      const label = reason === 'first_observed' ? '首次记录' : '变化';
      const short = str(p, 'header_sha256').slice(0, 8);
      vm.addSystemMessage(`请求头${label}: ${num(p, 'tool_count')} 个工具，摘要 ${short}…`, false);
      break;
    }

    case 'tool_proposal':
      vm.addOrUpdateToolTrace(str(p, 'tool_name'), str(p, 'input_summary') || str(p, 'tool_name'), '...');
      break;

    case 'permission_requested':
      break;

    case 'permission_decision': {
      const tool = str(p, 'tool');
      const decision = str(p, 'decision');
      if (decision === 'deny' || decision === 'defer') vm.addSystemMessage(`[权限] ${tool}: 拒绝`, true);
      else vm.addSystemMessage(`[权限] ${tool}: 允许`, false);
      break;
    }

    case 'tool_started': {
      const tool = str(p, 'tool');
      const target = optStr(p, 'target') ?? '';
      vm.addOrUpdateToolTrace(tool, target, target || '运行中');
      break;
    }

    case 'tool_completed': {
      const tool = str(p, 'tool');
      const err = optStr(p, 'error');
      vm.updateToolLatest(tool, err ? `· 错误: ${err}` : '· 成功');
      break;
    }

    case 'tool_running': {
      const tool = str(p, 'tool');
      const callId = str(p, 'call_id');
      const secs = (num(p, 'wall_ms') / 1000).toFixed(0);
      const pid = p?.pid ? `，PID ${p.pid}` : '';
      // 334（§4.10）：本页累计转入后台次数（非当前任务数，不回减）——
      // beforeunload 风险面据此提示；真实任务表不在本页（仅模型可见）。
      state.backgroundTasks += 1;
      vm.addSystemMessage(
        `[工具运行中] ${tool} ${callId} 已运行 ${secs}s${pid}——命令继续运行，终态随后续工具结果返回`,
        false,
      );
      break;
    }

    case 'budget_cue_injected':
      vm.addSystemMessage(`[预算提示] 剩余 ${num(p, 'remaining_seconds')}s`, false);
      break;

    case 'orientation_checkpoint':
      vm.addSystemMessage(`[检查点] step ${num(p, 'step_index')}`, false);
      break;

    case 'tool_availability_check':
      vm.setToolProbe(num(p, 'complete'), num(p, 'complete') + num(p, 'incomplete'));
      vm.addSystemMessage(`工具探针: ${num(p, 'complete')} 链路完整`, false);
      break;

    case 'tool_belief_stagnation':
      vm.addSystemMessage(`[工具信念停滞] ${str(p, 'tool')}`, true);
      break;

    case 'instruction_provenance_gate': {
      const decision = str(p, 'decision');
      if (decision === 'block') {
        state.gateBlock = 'IPG';
      }
      vm.addSystemMessage(`[门控] IPG: ${decision}`, decision === 'block');
      break;
    }

    case 'gate_decision': {
      const gate = str(p, 'gate');
      const decision = str(p, 'decision');
      if (decision === 'block') {
        state.gateBlock = gate;
      }
      const reason = optStr(p, 'reason');
      vm.addSystemMessage(`[门控] ${gate}: ${decision}${reason ? `（${reason}）` : ''}`, decision === 'block');
      break;
    }

    case 'counterexample_gate': {
      const resp = optStr(p, 'model_response');
      // 285 批 F2（2026-10-16 用户令）：门前的草稿不是正式输出——门到即静默
      // 折叠本运行此前内容，只留门后正式输出（真机走查：门前整篇照常在渲染）。
      vm.foldSupersededBeforeGate();
      vm.addSystemMessage(`[反例询问] ${str(p, 'position')}${resp ? `：${resp}` : ''}`, false);
      break;
    }

    case 'mechanical_audit_update': {
      // 285 批 F3：本事件 payload 是 `{kind, payload:{key,round,summary,anomaly}}`
      // ——比其余事件多一层嵌套（变体 kind 决定内层形状）。此前按平铺读 ⇒ 键
      // 空串、轮号缺省 0，整片渲成「[机械审查] （轮 0）」幽灵行（2026-10-16
      // 真机走查实证）。内层与 orz-tui `/payload/key` 指针同源。
      const q = p?.payload ?? {};
      const key = str(q, 'key');
      const round = num(q, 'round');
      const anomaly = optStr(q, 'anomaly');
      const summary = optStr(q, 'summary');
      const line = anomaly
        ? `[机械审查] ${key}（轮 ${round}）：${anomaly}`
        : summary
          ? `[机械审查] ${key}（轮 ${round}）${summary}`
          : `[机械审查] ${key}（轮 ${round}）`;
      vm.addSystemMessage(line, false);
      break;
    }

    case 'context_compressed':
      // 0de D6（2026-10-16，设计 §4）：双字段兼容——clear 事件 payload 只带
      // `target_tokens`（无 `estimated_tokens_after`），模板固定读后者曾把
      // 实际 171K 渲成「估 ~0K」。两字段语义同＝压缩后模型面估算
      // （model_summary 事件两字段恒等值，取值不变）。
      vm.addSystemMessage(
        `[上下文压缩] 触发于 ${Math.round(num(p, 'trigger_tokens') / 1000)}K tokens，压掉 ${num(p, 'rounds_dropped')} 轮（估 ~${Math.round((num(p, 'estimated_tokens_after') || num(p, 'target_tokens')) / 1000)}K）`,
        true,
      );
      break;

    case 'context_recovery_truncated':
      vm.addSystemMessage(
        `[上下文恢复截断] 恢复对话估 ${Math.round(num(p, 'before_estimate_tokens') / 1000)}K，机械截断 ${num(p, 'rounds_dropped')} 轮（估 ~${Math.round(num(p, 'after_estimate_tokens') / 1000)}K）`,
        true,
      );
      break;

    case 'snapshot_created': {
      const tool = str(p, 'tool');
      const err = optStr(p, 'snapshot_error');
      const hash = optStr(p, 'snapshot_hash');
      if (err) vm.addSystemMessage(`[快照] ${tool}: 失败（${err}）`, true);
      else if (hash) vm.addSystemMessage(`[快照] ${tool} ${hash.slice(0, 8)}`, false);
      break;
    }

    case 'snapshot_restored': {
      const err = optStr(p, 'snapshot_error');
      vm.addSystemMessage(err ? `[快照恢复] 失败（${err}）` : '[快照恢复] 完成', !!err);
      break;
    }

    case 'session_archive': {
      const status = str(p, 'status');
      vm.addSystemMessage(`[会话归档] ${str(p, 'path')}（${status || 'ok'}）`, false);
      break;
    }

    case 'control_ticket_issued':
      vm.addSystemMessage(`[票据签发] ${str(p, 'ticket_id')}: ${str(p, 'ticket_kind')}（${str(p, 'capability_scope')}）`, false);
      break;
    case 'control_ticket_consumed':
      vm.addSystemMessage(`[票据消费] ${str(p, 'ticket_id')}: ${str(p, 'ticket_kind')}`, false);
      break;
    case 'control_ticket_rejected':
      vm.addSystemMessage(`[票据拒绝] ${str(p, 'ticket_id')}: ${str(p, 'reject_code')}`, true);
      break;

    case 'run_finished': {
      state.deliveredSupplements = 0;
      vm.collapseNonWarnings();
      vm.anchorFinalOutput();
      vm.addSystemMessage(`运行完成（${str(p, 'status') || 'completed'}）`, false);
      break;
    }

    case 'run_failed':
      vm.collapseNonWarnings();
      vm.anchorFinalOutput();
      vm.addSystemMessage(`[错误] ${str(p, 'error')}`, true);
      discloseRolledBackSupplements();
      break;

    case 'run_cancelled':
      vm.collapseNonWarnings();
      vm.anchorFinalOutput();
      vm.addSystemMessage(`运行已取消（${str(p, 'reason')}）`, false);
      discloseRolledBackSupplements();
      break;

    case 'run_invalidated':
      vm.collapseNonWarnings();
      vm.anchorFinalOutput();
      vm.addSystemMessage(`运行无效（${str(p, 'status')}）`, true);
      discloseRolledBackSupplements();
      break;

    case 'run_terminated':
      vm.collapseNonWarnings();
      vm.anchorFinalOutput();
      vm.addSystemMessage(`运行终止（${str(p, 'reason') || str(p, 'kind')}）`, true);
      discloseRolledBackSupplements();
      break;

    default:
      // 其余机制事件（检索族／充分性／ACAF 细节等）v1 不进内容面。
      break;
  }
}

/** ACP session/update → 视图模型（流式文本 + 工具卡，语义照 acp_client.rs）。 */
export function applyAcpUpdate(_sessionId, update) {
  const kind = update?.sessionUpdate;
  if (kind === 'agent_message_chunk') {
    const block = update?.content;
    const text = block?.type === 'text' ? block.text : '';
    if (text) vm.appendTextDelta(text);
  }
  // tool_call / tool_call_update 等其余 update v1 交由 journal 面承载。
}
