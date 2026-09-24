//! Context-compaction parameter surface — batch B3 of the controller split
//! (CONTROLLER_SPLIT_DESIGN_2026-08-29).
//!
//! Moved verbatim from `controller.rs`; mechanical extraction only —
//! behavior, events and journal chain unchanged.

use crate::controller::{AgentLoopController, chrono_utc_now};
use crate::gateway::model::{Message, Role};
use crate::host::LoopHost;

/// 上下文压缩参数面——**滑块上下文 v8**（2026-09-16 勘误批，
/// [`CONTEXT_SLIDER_V8_DESIGN_2026-09-16`] §2 §3 §8）。
///
/// v8 把模型面改为**投影层**（`model_face`）：主滑块 x（最近 x K 连续完整轮，
/// **永不被压缩/截断**）＋主滑块以外按 y K 切的分块（**仅逻辑分块、留在模型面
/// 内累积**）＋机械摘要行。**减少模型面的动作只有两个**：模型主动压缩
/// （`[SEMANTIC_SUMMARY]` 摘要块 ⇒ 按块压缩）与 T1 硬截断；本地面
/// （`messages`）**永不因模型面动作被覆盖写入**（不变量 I1–I4）。
///
/// **随勘误退役（不是「翻转」——v7 是记录错误）**：常驻滑窗的**机械驱逐／
/// 推进**（`advance_fold` 前移、驻留带预算 L、H−L 参数体系）、**rhythm
/// ＝ H＋缓冲（192K，视图尺）** 机械压缩触发、**视图兜底（`safety_tokens`
/// 256K ＋ `compact_messages` 截断至 `recovery_target_tokens`）** 整体截断、
/// **实际上下文 ≥950K 强制压一次**（由 T1 500K 硬截断取代）。即：机械层
/// **不再以「总量超线」为由**压缩或截断模型面——模型面总量只由「模型自压」与
/// 「H1/T1」管。
///
/// **实现批追加退役（2026-09-16，审查 R-3）**：主车道的**收尾压缩**
/// （`session_end` 阈值 → `run_template_compact` drain）与 **D2-2 恢复预检**
/// （`recovery_*` → `compact_messages`）一并退役——本地面自此**全程逐字全量**
/// （会话归档随黑板一起打包），模型面的体积约束全部由投影层 ＋ H1/T1 ＋ 守卫
/// 在 loop-top（请求装配之前）承担。⇒ `target_tokens`／`min_rounds`（旧死字段）、
/// `recovery_trigger_tokens`／`recovery_target_tokens`／`with_recovery_compact`
/// 与 `compact_messages` 全部下线；`session_end_trigger_tokens` **保留**（检索／
/// grill 车道各自的一次性历史仍走该路径，它们不写回本会话侧车）。
///
/// 仍然存活（**结构化轨＝机械压缩内容策略本身**，按既有设计工作）：命令／
/// 动作／结果类内容压成台账摘要行＋指针＋compaction 存档＋`context_compressed`
/// 事件，并作用于模型面（`run_template_compact` 的检索车道／收尾路径）；
/// `recent_tail_rounds`／`min_compactable`／`max_reduction_ratio` 三项守卫与
/// 收尾／恢复预检参数（`recovery_*`／`session_end_trigger_tokens`）**未变**。
#[derive(Debug, Clone, Copy)]
pub struct ContextCompactConfig {
    /// **主滑块 x**（v8 设计 §2）：最近 x K 估算（chars/2，与既有滑块同尺）的
    /// **连续完整轮**——模型始终携带的基础段，**永不被压缩、永不被截断**
    /// （不变量 I1），也是注意力应当停留的位置。默认
    /// `DEFAULT_MODEL_FACE_SLIDER_TOKENS` = 160K 估算 ≈123K 真实（落在
    /// ≤128K 真实＝普遍稳定区内）；env `ORZ_MODEL_FACE_SLIDER_TOKENS` 覆盖。
    /// **取代** v7 的 `ORZ_SLIDER_WINDOW_TOKENS`（H，滑窗上限；语义已随勘误
    /// 作废，沿用旧名会误导读者）。
    pub slider_window_tokens: u64,
    /// **分块 y**（v8 设计 §2）：主滑块以外的内容按 y K 估算切块——**仅逻辑
    /// 分块，不从模型面流出**；块是可寻址单位（供压缩指定与截断枚举），
    /// **不是驱逐单位**。默认 `DEFAULT_MODEL_FACE_BLOCK_TOKENS` = 32K；env
    /// `ORZ_MODEL_FACE_BLOCK_TOKENS` 覆盖。
    pub model_face_block_tokens: u64,
    /// **上限守卫**（**异常保险**，模型面估算刻度）：默认
    /// `DEFAULT_MODEL_FACE_GUARD_TOKENS` = **700K**（2026-09-16 实现批下调，
    /// 原 1.10M）——见常量注释（＝1.4× T1 线；仍远低于 1M 真实窗口）。
    /// 常态不可达（T1 在 500K 就把主滑块以外的已闭合分块清零，且**按越线重新
    /// 武装**）；只有「T1 压不动」（溢出体量在主滑块内／窗口期抑制）或单轮暴涨
    /// 才触及，越线行为＝**强制截断到线上**（v7 的「不开窗降级」随勘误作废）。
    /// env `ORZ_MODEL_FACE_GUARD_TOKENS` 覆盖。
    pub model_face_guard_tokens: u64,
    /// **注意力阶梯**（v8 设计 §3；0bh ④ 定稿 2026-09-22＝软档 **192/256K**
    /// 64K 步距、取消 224K）：默认
    /// `[192/256K 软提醒 → 320K 硬打断 → 500K 硬截断]`（模型面估算
    /// 刻度）。**生产固定**——刻度值进文案，改值即改语义；
    /// `with_context_scale_ladder` 仅供测试用极小值驱动。
    pub context_scale_ladder: [crate::context_scale::LadderStep; 4],
    /// P0-D review fix (2026-08-14, ADR-0010 v1.14): the end-of-session
    /// compaction gate — **v8 实现批后只在检索／grill 车道生效**（主车道
    /// 收尾压缩已退役，见结构体头注）。
    pub session_end_trigger_tokens: u64,
    /// P0-D S2/S3 (2026-08-14, ADR-0010 v1.10): bounded recent tail kept
    /// verbatim in the model-visible collapsed view / after a summary.
    pub recent_tail_rounds: usize,
    /// P0-D S3: minimum droppable content (tokens) for a template summary
    /// (reuse of orz-compaction `min_compactable` guard).
    pub min_compactable: u64,
    /// P0-D S3: maximum kept/before ratio — the summary must reduce by at
    /// least 40% (`max_reduction_ratio` 0.6; reuse of orz-compaction).
    pub max_reduction_ratio: f64,
}

impl Default for ContextCompactConfig {
    fn default() -> Self {
        Self {
            slider_window_tokens: DEFAULT_MODEL_FACE_SLIDER_TOKENS,
            model_face_block_tokens: DEFAULT_MODEL_FACE_BLOCK_TOKENS,
            model_face_guard_tokens: DEFAULT_MODEL_FACE_GUARD_TOKENS,
            context_scale_ladder: crate::context_scale::DEFAULT_LADDER,
            session_end_trigger_tokens: 160_000,
            recent_tail_rounds: 2,
            min_compactable: 5_000,
            max_reduction_ratio: 0.6,
        }
    }
}

impl AgentLoopController {
    /// P0-D review fix (2026-08-14): override the end-of-session compaction
    /// gate (tests use tiny values; production keeps 160K).
    pub fn with_session_end_trigger(mut self, tokens: u64) -> Self {
        self.context_compact.session_end_trigger_tokens = tokens;
        self
    }

    /// P0-D S2/S3: override the recent-tail length (tests use small values;
    /// production keeps 2).
    pub fn with_recent_tail(mut self, rounds: usize) -> Self {
        self.context_compact.recent_tail_rounds = rounds;
        self
    }

    /// 滑块上下文 v8（2026-09-16 勘误批，设计 §2）：pin **主滑块 x**
    /// （测试缝隙；生产在构造时读 `ORZ_MODEL_FACE_SLIDER_TOKENS`，默认 160K）。
    pub fn with_slider_window_tokens(mut self, tokens: u64) -> Self {
        self.context_compact.slider_window_tokens = tokens.max(1);
        self
    }

    /// 滑块上下文 v8：pin **分块 y**（测试缝隙；生产读
    /// `ORZ_MODEL_FACE_BLOCK_TOKENS`，默认 32K）。
    pub fn with_model_face_block_tokens(mut self, tokens: u64) -> Self {
        self.context_compact.model_face_block_tokens = tokens.max(1);
        self
    }

    /// 滑块上下文 v8：pin **上限守卫**（异常保险；生产读
    /// `ORZ_MODEL_FACE_GUARD_TOKENS`，默认 1.10M）。越线 ⇒ 强制截断到线上。
    pub fn with_model_face_guard_tokens(mut self, tokens: u64) -> Self {
        self.context_compact.model_face_guard_tokens = tokens.max(1);
        self
    }

    /// v8 阶梯测试缝隙（设计 §3）：用极小刻度驱动软提醒／H1／T1 三形态
    /// （生产固定 192/256/320/500K——0bh ④ 定稿软档 192/256；不暴露 env
    /// ——刻度值进文案，改值即改语义）。传入表原样采用（长度须为 4：
    /// 2 软 + H1 + T1）。
    ///
    /// 2026-09-16 实现批（审查 R-9）：测试刻度是「会话面」量级 ⇒ 同时把
    /// **静态开销读数 pin 0**（否则系统提示词＋工具定义会把小刻度一步顶穿）。
    /// 生产不 pin（按上一轮请求实测；见 `with_model_face_static_overhead`）。
    pub fn with_context_scale_ladder(
        mut self,
        ladder: [crate::context_scale::LadderStep; 4],
    ) -> Self {
        self.context_compact.context_scale_ladder = ladder;
        self.model_face_static_overhead_pin = Some(0);
        self
    }

    /// v8 实现批（2026-09-16，审查 R-9）：pin **模型面静态开销读数**
    /// （系统提示词 ＋ 工具定义；chars/2 同尺）。生产不 pin（＝按上一轮请求实测）。
    pub fn with_model_face_static_overhead(mut self, tokens: u64) -> Self {
        self.model_face_static_overhead_pin = Some(tokens);
        self
    }

    /// 必定压缩三步升级（2026-09-24）测试缝隙：种子化「强制窗收口未产出」
    /// 连续计数（生产恒 0 起步）——`2` ＝ T1 首火即达第三步（机械截断），
    /// 供既有 T1 截断钉以旧时序驱动新语义；三步全流程钉不用种子。
    pub fn with_t1_window_failures(mut self, failures: u32) -> Self {
        self.t1_window_failures_seed = failures;
        self
    }

    /// P0-D S3: override the summary reduction guards (tests relax them).
    pub fn with_summary_guards(mut self, min_compactable: u64, max_reduction_ratio: f64) -> Self {
        self.context_compact.min_compactable = min_compactable;
        self.context_compact.max_reduction_ratio = max_reduction_ratio;
        self
    }

    /// A6 §8 C.2 (2026-08-08): override the whitelist character cap
    /// (tests use small values; production keeps DEFAULT_WHITELIST_CAP).
    pub fn with_whitelist_cap(mut self, cap: usize) -> Self {
        self.whitelist_cap = cap;
        self
    }

    /// A6 §8 C.2: keep the resident whitelist message in the conversation's
    /// preamble zone (after the original prompt, before the first tool
    /// declaration) — the compaction mechanism's always-kept preamble then
    /// skips it automatically (user decision: 常驻被压缩机制跳过, never
    /// re-injected at compaction time). New entries update the existing
    /// whitelist message in place.
    pub(crate) fn upsert_whitelist_message(&self, messages: &mut Vec<Message>) {
        let entries = self.whitelist.lock().unwrap();
        if entries.is_empty() {
            return;
        }
        let content = crate::prompt::build_whitelist_block(&entries);
        if let Some(i) = messages
            .iter()
            .position(|m| m.content.starts_with(crate::prompt::WHITELIST_PREFIX))
        {
            messages[i].content = content;
            return;
        }
        let pos = messages
            .iter()
            .position(|m| m.role == Role::Assistant && !m.tool_calls.is_empty())
            .unwrap_or(messages.len());
        messages.insert(
            pos,
            Message {
                role: Role::User,
                content,
                tool_call_id: None,
                tool_calls: Vec::new(),
                reasoning_content: None,
                round: None,
            },
        );
    }

    /// A6 §8 C.2: mechanical best-effort archive of a whitelist write to
    /// `{journal_dir}/whitelist.jsonl` (JSONL: timestamp + content, plain
    /// text — user decision 明文存档). The run dir is covered by the A5
    /// retention sweep (7 days), so A5 needs no changes. Runtime compaction
    /// reads the in-memory list, never this file; an archive failure is
    /// logged and never blocks the write.
    ///
    /// Review decision (2026-08-08): credential-shaped content is scanned
    /// (`looks_like_api_key`, GAK-CRED-001's detector) before the archive
    /// append — a hit logs a warning as the audit trail but does NOT block
    /// the write (best-effort semantics unchanged; the whitelist is
    /// model-chosen task content, and the scan is a surfaced warning, not
    /// a gate).
    pub(crate) fn archive_whitelist_entry(&self, host: &dyn LoopHost, content: &str) {
        if orz_assurance::credential::looks_like_api_key(content) {
            tracing::warn!(
                "whitelist entry looks credential-shaped (archived anyway — \
                 .gsa is gitignored, retained 7 days by A5)"
            );
        }
        let line = serde_json::json!({
            "timestamp": chrono_utc_now(),
            "content": content,
        });
        let path = host.journal().journal_dir().join("whitelist.jsonl");
        let mut line = line.to_string();
        line.push('\n');
        if let Err(e) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .and_then(|mut f| std::io::Write::write_all(&mut f, line.as_bytes()))
        {
            tracing::warn!(
                error = %e,
                path = %path.display(),
                "whitelist archive append failed (best-effort)"
            );
        }
    }
}

/// 滑块上下文 v8（2026-09-16 勘误批，设计 §2 §8）：**主滑块 x** 默认 160K
/// **估算**（chars/2，与既有滑块同尺）≈123K 真实 token——恰好落在
/// 「≤128K 真实＝普遍稳定区」之内（MRCR v2 64K–128K 档顶档 ≈97%），
/// 故 x 沿用量级、改的是其后的阶梯与天花板。env
/// `ORZ_MODEL_FACE_SLIDER_TOKENS` 覆盖（缺省/非法/0 ＝默认）。
pub const DEFAULT_MODEL_FACE_SLIDER_TOKENS: u64 = 160_000;

/// 滑块上下文 v8：**分块 y** 默认 32K 估算（设计 §2「仅逻辑分块，不从模型面
/// 流出」）。块是压缩指定与截断枚举的可寻址单位，不是驱逐单位。env
/// `ORZ_MODEL_FACE_BLOCK_TOKENS` 覆盖。
pub const DEFAULT_MODEL_FACE_BLOCK_TOKENS: u64 = 32_000;

/// 滑块上下文 v8：**上限守卫**默认 **700K 估算 ≈539K 真实**（**异常保险**；
/// 2026-09-16 实现批按用户裁定「守卫降值 ＋ T1 重新武装，双管齐下」由 1.10M
/// 下调）。取值依据：
///
/// - **必须 > T1 线**（500K 估算）才不会抢在 T1 之前触发；留 **200K 估算
///   （≈73 轮 ≈154K 真实）** 的间距，既容得下 H1 窗口期的 ≤3 轮抑制，也容得下
///   一次单轮暴涨后再由守卫兜底；
/// - **远低于 provider 1M 真窗口**（≈539K 真实 ≈ 窗口的 54%），并覆盖「T1 压
///   不动」的情形（溢出体量在主滑块内／窗口期抑制）；
/// - T1 **按越线重新武装**后，日常天花板＝500K 估算 ≈385K 真实，本线只在
///   异常路径出现（真机实测前属**待校准值**，env 可覆盖）。
///
/// 越线＝**强制截断到线上**（v7 的「不开窗降级」随勘误作废）；env
/// `ORZ_MODEL_FACE_GUARD_TOKENS` 覆盖。
pub const DEFAULT_MODEL_FACE_GUARD_TOKENS: u64 = 700_000;

/// Env override 名（单一源；测试缝隙不经进程 env——`parse_*` 纯函数）。
///
/// **v7 五 env 随勘误退役**（构造时**不再读取**；沿用旧名会误导读者——
/// 语义已被 v8 取代，故不给新名而是直接作废，同 `ORZ_LADDER_*` 先例）：
/// `ORZ_SLIDER_WINDOW_TOKENS`（H＝滑窗上限）、`ORZ_SLIDER_RESIDENT_TOKENS`
/// （L＝驻留带）、`ORZ_SLIDER_RHYTHM_BUFFER_TOKENS`（rhythm 缓冲）、
/// `ORZ_CONTEXT_SCALE_HARD_TOKENS`（950K 硬兜底）、
/// `ORZ_CONTEXT_SCALE_WINDOW_CAP_TOKENS`（窗口上传上限）。
pub const ENV_MODEL_FACE_SLIDER_TOKENS: &str = "ORZ_MODEL_FACE_SLIDER_TOKENS";
pub const ENV_MODEL_FACE_BLOCK_TOKENS: &str = "ORZ_MODEL_FACE_BLOCK_TOKENS";
pub const ENV_MODEL_FACE_GUARD_TOKENS: &str = "ORZ_MODEL_FACE_GUARD_TOKENS";

/// Pure parse rule for every slider env value (tested without env mutation):
/// trimmed, positive integer; absent/invalid/zero → None (＝默认).
pub(crate) fn parse_slider_tokens(s: &str) -> Option<u64> {
    s.trim().parse().ok().filter(|v| *v > 0)
}

/// Env override for the model-face main slider x. Parsed at controller
/// construction; absent/invalid/zero = the default.
pub fn model_face_slider_tokens_override() -> Option<u64> {
    std::env::var(ENV_MODEL_FACE_SLIDER_TOKENS)
        .ok()
        .and_then(|s| parse_slider_tokens(&s))
}

/// Env override for the model-face block size y.
pub fn model_face_block_tokens_override() -> Option<u64> {
    std::env::var(ENV_MODEL_FACE_BLOCK_TOKENS)
        .ok()
        .and_then(|s| parse_slider_tokens(&s))
}

/// Env override for the model-face guard（异常保险线；越线强制截断到线上）。
pub fn model_face_guard_tokens_override() -> Option<u64> {
    std::env::var(ENV_MODEL_FACE_GUARD_TOKENS)
        .ok()
        .and_then(|s| parse_slider_tokens(&s))
}

/// A6 §8 C.2 (2026-08-08): default cumulative character cap for the
/// compaction whitelist (16K — user decision; ≈8K tokens ≈ ~9% of the
/// 90K compacted target, small enough not to squeeze the kept rounds).
pub const DEFAULT_WHITELIST_CAP: usize = 16 * 1024;

/// A6: estimated tokens of one message — chars/2 (a conservative CJK-aware
/// guess: CJK ≈ 2 chars/token, English would be ≈ 4 — over-estimating is
/// the safe direction; the real next-round usage measurement is what the
/// trigger uses).
pub(crate) fn estimate_message_tokens(m: &Message) -> u64 {
    let mut chars = m.content.chars().count() as u64;
    if let Some(r) = &m.reasoning_content {
        chars += r.chars().count() as u64;
    }
    for tc in &m.tool_calls {
        chars += tc.name.chars().count() as u64;
        chars += serde_json::to_string(&tc.arguments)
            .map(|s| s.chars().count() as u64)
            .unwrap_or(0);
    }
    chars / 2
}

pub(crate) fn estimate_messages_tokens(messages: &[Message]) -> u64 {
    messages.iter().map(estimate_message_tokens).sum()
}

// A6 (2026-08-08) 的 `compact_messages`（把旧轮 drain 出会话本体的显式机械
// 压缩）**随 v8 实现批退役**（2026-09-16，审查 R-3）：它唯一的调用者是不再存在
// 的 D2-2 恢复预检，语义与「本地面＝单对话全量、不因模型面动作丢失」冲突。
// 仍需要「按总量收缩」的唯一车道＝检索／grill 的 session-end 模板压缩，走
// `agent_loop::run_template_compact`（`collapsed_cut` ＋ drain ＋ marker），
// 与本函数无关。历史 journal 的 `context_recovery_truncated` 事件保留在闭枚举
// 里仅供回放（生产零写入）。

#[cfg(test)]
mod tests {
    use super::*;
    use crate::controller_test_support::*;
    use crate::gateway::fake::{FakeProvider, ScriptedResponse};
    use crate::gateway::model::{FinishReason, ModelGateway, ToolCall};
    use crate::host::{
        PermitDecision, PermitError, RiskClass, ToolError, ToolRegistry, ToolResult,
    };
    use orz_assurance::{EventType, JournalRecorder};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU64, Ordering};

    #[tokio::test]
    async fn whitelist_first_batch_sealed_and_never_lands() {
        // Host returning a DIFFERENT fat output per call — so the dropped
        // oldest round is distinguishable from the kept newest round.
        struct SeqHost {
            journal: JournalRecorder,
            outputs: Vec<String>,
            calls: AtomicU64,
            cwd: std::path::PathBuf,
        }
        #[async_trait::async_trait]
        impl LoopHost for SeqHost {
            fn journal(&self) -> &JournalRecorder {
                &self.journal
            }
            fn tools_registry(&self) -> &dyn ToolRegistry {
                &EmptyRegistry
            }
            fn session_cwd(&self) -> std::path::PathBuf {
                self.cwd.clone()
            }
            async fn call_tool(
                &self,
                _name: &str,
                _arguments: serde_json::Value,
                _call_id: &str,
            ) -> Result<ToolResult, ToolError> {
                let n = self.calls.fetch_add(1, Ordering::SeqCst) as usize;
                Ok(ToolResult {
                    output: self.outputs[n % self.outputs.len()].clone(),
                    exit_code: Some(0),
                    output_encoding: None,
                    structured: None,
                    ..Default::default()
                })
            }
            async fn request_permission(
                &self,
                _risk: RiskClass,
                _tool: &str,
                _arguments: &serde_json::Value,
            ) -> Result<PermitDecision, PermitError> {
                Ok(PermitDecision::AllowOnce)
            }
        }
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = SeqHost {
            journal,
            outputs: vec!["A".repeat(600), "B".repeat(600)],
            calls: AtomicU64::new(0),
            cwd: dir.clone(),
        };
        // Two whitelist writes in the SAME first batch — append semantics
        // in the resident message AND two archive lines (JSONL append).
        let whitelist_calls = ScriptedResponse::tool_calls(vec![
            ToolCall {
                name: "compaction_whitelist_add".to_string(),
                arguments: serde_json::json!({"content": "任务背景：修复缓存回归；约束：不改 schema"}),
                call_id: "call-w1".to_string(),
            },
            ToolCall {
                name: "compaction_whitelist_add".to_string(),
                arguments: serde_json::json!({"content": "关键路径：src/controller.rs"}),
                call_id: "call-w1b".to_string(),
            },
        ]);
        let tool_call = |id: &str| ScriptedResponse {
            text: None,
            tool_calls: vec![ToolCall {
                name: "read_file".to_string(),
                arguments: serde_json::json!({"target_file": "a.txt"}),
                call_id: id.to_string(),
            }],
            finish_reason: FinishReason::ToolCalls,
            reasoning_content: None,
            prompt_tokens: Some(50_000),
        };
        let fake = Arc::new(FakeProvider::new(vec![
            whitelist_calls,
            tool_call("call-w2"),
            tool_call("call-w3"),
            // 机械模式（2026-08-18 B 定案）：压缩零模型调用，无摘要项。
            ScriptedResponse::text("候选答案").with_prompt_tokens(5_000),
            ScriptedResponse::text("最终答案").with_prompt_tokens(5_000),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway)
            .with_slider_window_tokens(100_000_000)
            .with_summary_guards(1, 1.0);
        controller
            .run_turn(
                &host,
                "修复任务",
                "RUN-WHITELIST",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        // 同批次两次 whitelist 调用均被 sealed_tool_denied 拒绝。
        let sealed: Vec<_> = events(&dir)
            .into_iter()
            .filter(|e| e.event_type == EventType::ToolCompleted)
            .filter(|e| e.payload["tool"] == "compaction_whitelist_add")
            .filter(|e| {
                e.payload.get("error").and_then(|v| v.as_str()) == Some("sealed_tool_denied")
            })
            .collect();
        assert_eq!(sealed.len(), 2, "both first-batch writes sealed");
        // 无 ToolStarted（零副作用前置条件）。
        assert!(
            events(&dir).into_iter().all(|e| {
                !(e.event_type == EventType::ToolStarted
                    && e.payload.get("tool").and_then(|t| t.as_str())
                        == Some("compaction_whitelist_add"))
            }),
            "sealed tool must never reach ToolStarted"
        );
        // 白名单恒空、无存档文件。
        let w = controller.whitelist.lock().unwrap();
        assert!(w.is_empty(), "sealed whitelist write must not land: {w:?}");
        drop(w);
        let archive = dir.join("whitelist.jsonl");
        assert!(!archive.exists(), "no whitelist archive under sealing");
        // 主对话不出现 [压缩白名单] 注入块。
        let received = fake.received_requests();
        assert!(
            received
                .iter()
                .flat_map(|r| r.messages.iter())
                .all(|m| !m.content.starts_with("[压缩白名单")),
            "no whitelist message under sealing"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// THIN-HARNESS-REDESIGN R1 (§4.1 边界项, 2026-08-27)：compaction_
    /// whitelist_add 已封存——跨批次的两条写入一律 sealed_tool_denied
    /// 结构化拒绝（无白名单消息、零副作用、完整 ToolCompleted 事件链）。
    #[tokio::test]
    async fn whitelist_later_batch_refused() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: Some(ToolResult {
                output: "ok".to_string(),
                exit_code: Some(0),
                output_encoding: None,
                structured: None,
                ..Default::default()
            }),
        };
        let whitelist_call = |id: &str, content: &str| {
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "compaction_whitelist_add".to_string(),
                arguments: serde_json::json!({"content": content}),
                call_id: id.to_string(),
            }])
        };
        let fake = Arc::new(FakeProvider::new(vec![
            whitelist_call("call-x1", "首轮条目"),
            whitelist_call("call-x2", "次轮条目"),
            ScriptedResponse::text("候选答案"),
            ScriptedResponse::text("最终答案"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway);
        controller
            .run_turn(
                &host,
                "任务",
                "RUN-WL-REFUSE",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        // 两条写入均被 sealed_tool_denied 拒绝——无 [压缩白名单] 消息。
        let received = fake.received_requests();
        assert!(
            received
                .iter()
                .flat_map(|r| r.messages.iter())
                .all(|m| !m.content.starts_with("[压缩白名单")),
            "no whitelist message under sealing: {received:?}"
        );
        // 两条拒绝均 journaled 为 sealed_tool_denied 的 ToolCompleted。
        let failed_tool_completed: Vec<_> = events(&dir)
            .into_iter()
            .filter(|e| e.event_type == EventType::ToolCompleted)
            .filter(|e| e.payload["tool"] == "compaction_whitelist_add")
            .filter(|e| {
                e.payload.get("error").and_then(|v| v.as_str()) == Some("sealed_tool_denied")
            })
            .collect();
        assert!(
            failed_tool_completed.len() >= 2,
            "both writes sealed-journaled: {failed_tool_completed:?}"
        );
        // 白名单恒空。
        let w = controller.whitelist.lock().unwrap();
        assert!(w.is_empty(), "sealed whitelist write must not land: {w:?}");
        drop(w);

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// THIN-HARNESS-REDESIGN R1 (§4.1 边界项, 2026-08-27)：compaction_
    /// whitelist_add 已封存——内容校验（容量/空值）已不可达，调用一律
    /// sealed_tool_denied（cap 配置不再生效；白名单恒空）。
    #[tokio::test]
    async fn whitelist_cap_refused() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "compaction_whitelist_add".to_string(),
                arguments: serde_json::json!({"content": "0123456789ABCDEFGHIJ"}), // 20 chars > cap 10
                call_id: "call-y1".to_string(),
            }]),
            ScriptedResponse::text("候选答案"),
            ScriptedResponse::text("最终答案"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway).with_whitelist_cap(10);
        controller
            .run_turn(&host, "任务", "RUN-WL-CAP", MANIFEST, 0, None, None, None)
            .await
            .unwrap();

        let received = fake.received_requests();
        let round2 = &received[1].messages;
        assert!(
            round2
                .iter()
                .filter(|m| m.role == Role::Tool)
                .any(|m| m.content.contains("已封存") && m.content.contains("不再可用")),
            "sealed refusal: {round2:?}"
        );
        // No whitelist message was created.
        assert!(
            round2.iter().all(|m| !m.content.starts_with("[压缩白名单")),
            "no whitelist message on refusal: {round2:?}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// THIN-HARNESS-REDESIGN R1 (§4.1 边界项, 2026-08-27)：compaction_
    /// whitelist_add 已封存——空/空白内容校验已不可达，调用一律
    /// sealed_tool_denied，无白名单消息、完整 ToolCompleted 事件链。
    #[tokio::test]
    async fn whitelist_empty_content_refused() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "compaction_whitelist_add".to_string(),
                arguments: serde_json::json!({"content": "   "}),
                call_id: "call-z1".to_string(),
            }]),
            ScriptedResponse::text("候选答案"),
            ScriptedResponse::text("最终答案"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway);
        controller
            .run_turn(&host, "任务", "RUN-WL-EMPTY", MANIFEST, 0, None, None, None)
            .await
            .unwrap();

        let received = fake.received_requests();
        let round2 = &received[1].messages;
        assert!(
            round2
                .iter()
                .filter(|m| m.role == Role::Tool)
                .any(|m| m.content.contains("已封存") && m.content.contains("不再可用")),
            "sealed refusal: {round2:?}"
        );
        assert!(
            round2.iter().all(|m| !m.content.starts_with("[压缩白名单")),
            "no whitelist message on empty refusal: {round2:?}"
        );
        let failed: Vec<_> = events(&dir)
            .into_iter()
            .filter(|e| e.event_type == EventType::ToolCompleted)
            .filter(|e| e.payload["tool"] == "compaction_whitelist_add")
            .filter(|e| {
                e.payload.get("error").and_then(|v| v.as_str()) == Some("sealed_tool_denied")
            })
            .collect();
        assert_eq!(failed.len(), 1, "sealed refusal journaled");
        let w = controller.whitelist.lock().unwrap();
        assert!(w.is_empty(), "sealed whitelist write must not land: {w:?}");
        drop(w);

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 0am FR4（2026-09-20 用户令）：压缩白名单并入 `context_compress`——
    /// 可选 `whitelist` 条目在**处理压缩的同一调用**里落盘：内存列表 +
    /// `{journal_dir}/whitelist.jsonl` best-effort 存档 + 常驻前言区
    /// `[压缩白名单` 消息（跨压缩保留、机械压缩跳过）；空条目/超累计上限
    /// 条目跳过并在响应中如实说明；全链 fail-soft（exit 0 信封、三态不变）。
    #[tokio::test]
    async fn context_compress_whitelist_entries_land_and_respect_cap() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "context_compress".to_string(),
                arguments: serde_json::json!({
                    "whitelist": ["任务背景：甲", "关键路径：src/x.rs", "   "],
                }),
                call_id: "call-cc-w1".to_string(),
            }]),
            ScriptedResponse::text("候选答案"),
            ScriptedResponse::text("最终答案"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        // cap 取 12：首条（6 字符）落地；次条（13 字符）超累计上限被跳过。
        let controller = AgentLoopController::with_gateway(gateway).with_whitelist_cap(12);
        controller
            .run_turn(&host, "任务", "RUN-CC-WL", MANIFEST, 0, None, None, None)
            .await
            .unwrap();

        // ① 工具仍 exit 0（fail-soft 信封）。
        let cc: Vec<_> = events(&dir)
            .into_iter()
            .filter(|e| {
                e.event_type == EventType::ToolCompleted && e.payload["tool"] == "context_compress"
            })
            .collect();
        assert_eq!(cc.len(), 1, "{cc:?}");
        assert_eq!(cc[0].payload["exit_code"], 0);
        // ② 内存白名单 = 仅首条（空/超限条目未落地）。
        let w = controller.whitelist.lock().unwrap().clone();
        assert_eq!(w, vec!["任务背景：甲".to_string()], "{w:?}");
        // ③ 存档：whitelist.jsonl 恰一行（best-effort JSONL append）。
        let archive = dir.join("whitelist.jsonl");
        let text = std::fs::read_to_string(&archive).expect("whitelist archive");
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 1, "{lines:?}");
        assert!(lines[0].contains("任务背景：甲"), "{lines:?}");
        // ④ 常驻前言区消息 + 响应回执（含跳过说明）。
        let received = fake.received_requests();
        let round2 = &received[1].messages;
        assert!(
            round2
                .iter()
                .any(|m| m.content.starts_with("[压缩白名单") && m.content.contains("任务背景：甲")),
            "resident whitelist message: {round2:?}"
        );
        assert!(
            round2.iter().filter(|m| m.role == Role::Tool).any(|m| m
                .content
                .contains("白名单：保存 1 条")
                && m.content.contains("超累计上限跳过")
                && m.content.contains("空条目跳过")),
            "whitelist receipt with skip notes: {round2:?}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// v8 实现批（2026-09-16，审查 R-3）：**本地面全量零覆盖**——主车道 run
    /// 收尾不再有任何机械压缩：会话侧车（进而归档包）逐字保留全部轮次，且
    /// **不得**出现 `context_compressed{reason=session_end}`（该路径已退役；
    /// 检索／grill 车道各自的一次性历史仍按自己的收口路径走）。
    #[tokio::test]
    async fn main_lane_session_end_keeps_the_local_face_verbatim() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::text("收到"),
            ScriptedResponse::text("收到"),
        ]));
        // 收尾阈值钉到 1（旧行为下必然触发一次 session_end 压缩）。
        let controller = AgentLoopController::with_gateway(fake.clone())
            .with_session_end_trigger(1)
            .with_summary_guards(1, 1.0);
        let fat = "A".repeat(600);
        let mut conversation = vec![conv_message(Role::User, "第一问")];
        conversation.extend(tool_round("call-r1", &fat));
        conversation.extend(tool_round("call-r2", &fat));
        let _ = controller
            .run_turn(
                &host,
                "继续",
                "RUN-V8-LOCAL",
                MANIFEST,
                0,
                None,
                None,
                Some(&mut conversation),
            )
            .await
            .unwrap();
        // ① 无 session_end 压缩事件（生产零写入）。
        let compact_events: Vec<serde_json::Value> = events(&dir)
            .into_iter()
            .filter(|e| e.event_type == EventType::ContextCompressed)
            .map(|e| e.payload)
            .collect();
        assert!(
            compact_events.iter().all(|p| p["reason"] != "session_end"),
            "主车道收尾压缩已退役: {compact_events:?}"
        );
        // ② 本地面逐字全量（fat 正文一条不少）＋ 无压缩 marker。
        assert_eq!(
            conversation.iter().filter(|m| m.content == fat).count(),
            2,
            "本地面逐字原文必须原样留在侧车: {conversation:?}"
        );
        assert!(
            conversation
                .iter()
                .all(|m| !m.content.starts_with("[前文上下文已压缩")),
            "未压缩则不得有压缩 marker: {conversation:?}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn conversation_writeback_retains_marker_and_whitelist() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::text("收到"),
            ScriptedResponse::text("收到"),
        ]));
        let controller = AgentLoopController::with_gateway(fake.clone());
        let mut conversation = vec![
            conv_message(Role::User, "第一问"),
            conv_message(
                Role::User,
                &crate::prompt::build_whitelist_block(&["任务背景：修复缓存回归".to_string()]),
            ),
            conv_message(
                Role::User,
                &crate::prompt::context_compressed_marker(3, 160_000, None),
            ),
            conv_message(Role::User, "[ORIENTATION v0.1] 当前任务是什么？"),
        ];
        let _ = controller
            .run_turn(
                &host,
                "第二问",
                "RUN-RETAIN",
                MANIFEST,
                0,
                None,
                None,
                Some(&mut conversation),
            )
            .await
            .unwrap();
        let kept: Vec<&str> = conversation.iter().map(|m| m.content.as_str()).collect();
        assert!(
            kept.iter()
                .any(|c| c.starts_with(crate::prompt::WHITELIST_PREFIX)),
            "whitelist must survive restore write-back: {kept:?}"
        );
        assert!(
            kept.iter()
                .any(|c| c.starts_with(crate::prompt::CONTEXT_COMPRESSED_PREFIX)),
            "marker must survive restore write-back: {kept:?}"
        );
        assert!(
            kept.iter().all(|c| !c.starts_with("[ORIENTATION")),
            "other injected blocks stay filtered: {kept:?}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// THIN-HARNESS-REDESIGN R1 (§4.1)：compaction_whitelist_add 已封存
    /// ——即使 plan_first 会话计划落板后（旧 P2-2 顺延窗口），调用仍被
    /// sealed_tool_denied 结构化拒绝、whitelist 不落盘。
    #[tokio::test]
    async fn whitelist_write_sealed_even_in_plan_first_session() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: Some(ToolResult {
                output: "ok".to_string(),
                exit_code: Some(0),
                output_encoding: None,
                structured: None,
                ..Default::default()
            }),
        };
        let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![plan_write_call("call-plan-1", valid_plan_json())]),
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "compaction_whitelist_add".to_string(),
                arguments: serde_json::json!({"content": "task fact"}),
                call_id: "call-wl-1".to_string(),
            }]),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let controller = AgentLoopController::with_gateway(gateway).with_plan_first_enabled(true);
        controller
            .run_turn(
                &host,
                "修复缓存回归",
                "RUN-PF-WL",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let events = events(&dir);
        let wl = events
            .iter()
            .find(|e| {
                e.event_type == EventType::ToolCompleted
                    && e.payload.get("tool").and_then(|v| v.as_str())
                        == Some("compaction_whitelist_add")
            })
            .expect("sealed whitelist call refused after the plan round");
        assert_eq!(
            wl.payload["exit_code"].as_u64(),
            Some(1),
            "sealed whitelist write must be refused: {:?}",
            wl.payload
        );
        assert_eq!(
            wl.payload["error"].as_str(),
            Some("sealed_tool_denied"),
            "{:?}",
            wl.payload
        );
        let w = controller.whitelist.lock().unwrap();
        assert!(w.is_empty(), "sealed whitelist write must not land: {w:?}");
        drop(w);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
