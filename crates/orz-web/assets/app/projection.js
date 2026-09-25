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

export function applyJournalEvent(ev) {
  const p = ev.payload ?? {};
  switch (ev.event_type) {
    case 'run_preflight':
      vm.statusUpdate('守护', true);
      vm.setRunState('预检', true);
      vm.addSystemMessage('预检通过', false);
      break;

    case 'run_started':
      state.running = true;
      state.gateBlock = null;
      vm.setRunState('运行中', true);
      vm.syncToolbar();
      break;

    case 'prompt_submitted':
      state.turnCounter++;
      vm.addUserMessage(str(p, 'prompt'));
      break;

    case 'model_request':
      vm.setModel(str(p, 'model_id') || '模型');
      break;

    case 'model_response_received':
      break; // 与 model_output 的工具行面合并，避免双计

    case 'model_output': {
      vm.setRunState('运行中', true);
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
      vm.setRunState('等待审批', true);
      break;

    case 'permission_decision': {
      vm.setRunState('运行中', true);
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
        vm.setRunState('失败', false);
        state.gateBlock = 'IPG';
      }
      vm.addSystemMessage(`[门控] IPG: ${decision}`, decision === 'block');
      break;
    }

    case 'gate_decision': {
      const gate = str(p, 'gate');
      const decision = str(p, 'decision');
      if (decision === 'block') {
        vm.setRunState('失败', false);
        state.gateBlock = gate;
      }
      const reason = optStr(p, 'reason');
      vm.addSystemMessage(`[门控] ${gate}: ${decision}${reason ? `（${reason}）` : ''}`, decision === 'block');
      break;
    }

    case 'counterexample_gate': {
      const resp = optStr(p, 'model_response');
      vm.addSystemMessage(`[反例询问] ${str(p, 'position')}${resp ? `：${resp}` : ''}`, false);
      break;
    }

    case 'mechanical_audit_update': {
      const key = str(p, 'key');
      const round = num(p, 'round');
      const anomaly = optStr(p, 'anomaly');
      const summary = optStr(p, 'summary');
      const line = anomaly
        ? `[机械审查] ${key}（轮 ${round}）：${anomaly}`
        : summary
          ? `[机械审查] ${key}（轮 ${round}）${summary}`
          : `[机械审查] ${key}（轮 ${round}）`;
      vm.addSystemMessage(line, false);
      break;
    }

    case 'context_compressed':
      vm.addSystemMessage(
        `[上下文压缩] 触发于 ${Math.round(num(p, 'trigger_tokens') / 1000)}K tokens，压掉 ${num(p, 'rounds_dropped')} 轮（估 ~${Math.round(num(p, 'estimated_tokens_after') / 1000)}K）`,
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
      state.running = false;
      vm.setRunState('完成', true);
      vm.collapseNonWarnings();
      vm.anchorFinalOutput();
      vm.addSystemMessage(`运行完成（${str(p, 'status') || 'completed'}）`, false);
      vm.syncToolbar();
      break;
    }

    case 'run_failed':
      state.running = false;
      vm.setRunState('失败', false);
      vm.collapseNonWarnings();
      vm.anchorFinalOutput();
      vm.addSystemMessage(`[错误] ${str(p, 'error')}`, true);
      vm.syncToolbar();
      break;

    case 'run_cancelled':
      state.running = false;
      vm.setRunState('已取消', true);
      vm.collapseNonWarnings();
      vm.anchorFinalOutput();
      vm.addSystemMessage(`运行已取消（${str(p, 'reason')}）`, false);
      vm.syncToolbar();
      break;

    case 'run_invalidated':
      state.running = false;
      vm.setRunState('无效', false);
      vm.collapseNonWarnings();
      vm.anchorFinalOutput();
      vm.addSystemMessage(`运行无效（${str(p, 'status')}）`, true);
      vm.syncToolbar();
      break;

    case 'run_terminated':
      state.running = false;
      vm.setRunState('失败', false);
      vm.collapseNonWarnings();
      vm.anchorFinalOutput();
      vm.addSystemMessage(`运行终止（${str(p, 'reason') || str(p, 'kind')}）`, true);
      vm.syncToolbar();
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
