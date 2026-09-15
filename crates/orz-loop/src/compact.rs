//! Context-compaction parameter surface — batch B3 of the controller split
//! (CONTROLLER_SPLIT_DESIGN_2026-08-29).
//!
//! Moved verbatim from `controller.rs`; mechanical extraction only —
//! behavior, events and journal chain unchanged.

use crate::controller::{AgentLoopController, chrono_utc_now};
use crate::gateway::model::{Message, Role};
use crate::host::LoopHost;

/// P0-D (2026-08-14, ADR-0010 v1.10 / CONTEXT_COMPACTION_DESIGN §2-§6):
/// context-compaction parameters for the redesigned mechanism — the A6
/// parameter set (90K target / 20-round cooldown / 250K fallback) is
/// revoked.
///
/// - Template summary (S3): fires at any loop-top gap when the previous
///   round's MEASURED prompt tokens exceed `trigger_tokens` (192K —
///   2026-08-18 adjudication, ADR-0010 §14.26) with a ≥`min_rounds`
///   (2 model rounds — review fix 2026-08-14) cooldown, or exceed
///   `safety_tokens` (256K fallback — 2026-08-18 adjudication, ADR-0010
///   §14.26) regardless of cooldown.
///   Reduction guards: removable content ≥ `min_compactable` (5K) and kept
///   ≤ `max_reduction_ratio` (0.6) of before. A guard that cannot be
///   satisfied retries across trigger rounds and forces one compaction
///   after `GUARD_RETRY_LIMIT` failures (`guard_failed`). The summary
///   output is a five-section template (≤17K chars), archived under
///   `.gsa/compaction/` with a digest, and the rolling single marker
///   carries the pointer.
/// - Mechanical collapse (S2): every completed OLD tool round collapses
///   into a deterministic action-ledger row in the MODEL-VISIBLE request
///   (zero model calls, `recent_tail_rounds` kept verbatim); the
///   persisted conversation keeps the full records. FUS-LEDGER-FOLD-STATE
///   (2026-08-18, ADR-0010 §14.26): the collapse is stateful — the fold
///   point advances only when the estimated request view reaches the
///   window cap, and between advances the request view prefix is
///   byte-stable (pure append), restoring the v1.9 prefix-cache
///   discipline that the old per-request stateless recomputation broke.
///   动态上下文滑块 S1（2026-09-15，CONTEXT_DYNAMIC_SLIDER_DESIGN §3.1/§3.2
///   ＋ §3.3 成本律，用户裁定 R1/R2/R4）**把「锯齿折叠」换成「常驻滑窗」**：
///   触发＝视图估算 ≥ `slider_window_tokens`（H），动作＝自最旧驻留轮起
///   收集**连续完整轮**至累计估算 ≥ H−L（L ＝ `slider_resident_tokens`），
///   一次性移出视图；视图＝`preamble ＋ 固定指针 ＋ D4 机械段 ＋ 驻留带
///   ＋ 逐字尾部`，谷值恒 ≥L（锯齿形态的折后谷值实测仅 9,600 token ⇒
///   死亡螺旋根因）。参数 **L/H 同比例放大成本不变**（R≈0.925/(1−L/H)），
///   故「处理巨大/高压长任务」的正解是抬 H。**旧 `fold_tail_tokens`（8K
///   桥预算）与 `fold_trigger_tokens`（128K 触发）及其 `ORZ_FOLD_*` env
///   随之退役**：桥＝驻留带的尾部投影，语义并入 L。
/// - Recovery pre-check (D2-2): a restored conversation estimated over
///   `recovery_trigger_tokens` (200K conservative) is mechanically
///   truncated toward `recovery_target_tokens` (160K) before the first
///   request, with the full sidecar copied into the run journal as the
///   audit copy.
///
/// `target_tokens` (legacy A6 90K) is retained only for the pure
/// `compact_messages` unit surface; the loop no longer uses it.
#[derive(Debug, Clone, Copy)]
pub struct ContextCompactConfig {
    pub target_tokens: u64,
    pub min_rounds: u32,
    pub safety_tokens: u64,
    /// 动态上下文滑块 S1（2026-09-15，设计 §3.2/§3.3）：常驻滑窗上限 **H**
    /// ——当**估算的请求视图**（折叠后视图，chars/2）≥ H 时，循环在
    /// loop-top 安全间隙一次性驱逐最旧的连续完整轮。默认
    /// `DEFAULT_SLIDER_WINDOW_TOKENS` = 160K（保守档，用户裁定 R4）；env
    /// `ORZ_SLIDER_WINDOW_TOKENS` 覆盖。取代旧 `fold_trigger_tokens`
    /// （128K）与 `ORZ_FOLD_TRIGGER_TOKENS`。
    pub slider_window_tokens: u64,
    /// 滑块驻留带 **L**：驱逐后视图保留的连续完整轮预算（估计口径
    /// chars/2，与 H 同尺）。默认 `DEFAULT_SLIDER_RESIDENT_TOKENS` = 64K
    /// ⇒ 驱逐深度 = H−L = 96K、成本比 R ≈ ×1.54（设计 §3.3 表）；env
    /// `ORZ_SLIDER_RESIDENT_TOKENS` 覆盖。取代旧 8K 桥预算
    /// （`fold_tail_tokens` / `ORZ_FOLD_TAIL_TOKENS`）。
    ///
    /// 纪律：**L 与 H 不得进指针文案**（改 env 即一次性前缀失效；配置
    /// 固定性优先于精确表述，设计 §3.1）。
    pub slider_resident_tokens: u64,
    /// 机械压缩 rhythm 缓冲动量：**rhythm 阈值 = H ＋ 本缓冲**（视图刻度
    /// ——滑块把视图恒压在上限内，故 rhythm 退化为「驱逐停滞兜底」而非
    /// 周期性打断；用户裁定 R1 第 4 条＝打断频率上限）。默认
    /// `DEFAULT_SLIDER_RHYTHM_BUFFER_TOKENS` = 32K ⇒ 保守档 192K（与旧
    /// 192K rhythm 同值、新推导）；env `ORZ_SLIDER_RHYTHM_BUFFER_TOKENS`
    /// 覆盖。
    pub slider_rhythm_buffer_tokens: u64,
    /// rhythm 阈值的显式覆盖（**测试缝隙**；`None` = H ＋ 缓冲）。
    /// 生产路径不设；`with_context_compact` 用它维持旧测试口径。
    pub rhythm_tokens_override: Option<u64>,
    /// 机械压缩硬兜底（**实际上下文刻度**，非视图刻度）：全量会话估算
    /// ≥ 本值时无条件压缩一次（绕过冷却）。默认
    /// `DEFAULT_CONTEXT_SCALE_HARD_TOKENS` = 950K（设计 §3.4「硬兜底改挂
    /// 实际上下文」——滑窗下视图刻度不可达，兜底必须换尺）；env
    /// `ORZ_CONTEXT_SCALE_HARD_TOKENS` 覆盖。
    pub hard_context_tokens: u64,
    /// **压缩窗口的上传上限**（实际上下文估算刻度；审查修正批 2026-09-15）：
    /// v7 V3 的窗口轮要上传「滑块之外的携带内容 ＋ 滑块」＝`messages` 全量
    /// （`agent_loop` `window_loads_pending_region`）——**上传面无上限时，
    /// 实际上下文一旦越过 provider 单请求窗口，该轮请求就是硬失败**（run
    /// 直接报错，而非降级）。故开窗前先比对本阈值：越线则**不开窗**（按
    /// `context_scale` 走机械强制压缩 ＋ `mechanical_audit_update` 异常如实
    /// 落账），上传视图仍由常驻滑窗压在上限内 ⇒ 请求恒安全。
    ///
    /// 默认 `DEFAULT_WINDOW_UPLOAD_CAP_TOKENS` = 1.10M：provider 窗口 1M
    /// 真实 token × 实测换算系数 0.77（设计 §3.3.2：真实 token ≈ 0.77 ×
    /// 估算）⇒ 该上限对应 ≈0.85M 真实 token＋系统提示／工具面／输出预算的
    /// 余量。env `ORZ_CONTEXT_SCALE_WINDOW_CAP_TOKENS` 覆盖。
    /// **设计「过大时分段」仍是备选（S1 未实现）**：本阈值是 fail-soft
    /// 降级（不开必定失败的窗口），不是分段。
    pub window_upload_cap_tokens: u64,
    /// v7 压缩档位表（S1 修订批，2026-09-15，设计 §3.4.1「档位分工」；
    /// 用户裁定）：**实际上下文估算**（chars/2，全量会话）的两级刻度。
    /// 语义＝**除最后一档外全是纯提醒**（默认 500K：不打断、不开窗、
    /// 不强制，模型可延后）；**最后一档＝必须压缩一次**（默认 900K：
    /// 提醒 ＋ 开压缩窗口，窗口轮把「滑块之外的携带内容」连同滑块一并
    /// 上传给模型产出语义摘要）。**生产固定**（用户裁定值，无 env——
    /// 刻度值进文案，改值即改语义）；`with_context_scale_milestones`
    /// 仅供测试用极小值驱动。
    pub context_scale_milestones: [u64; 2],
    /// D2-2 (2026-08-14, ADR-0010 v1.10 / CONTEXT_COMPACTION_DESIGN §6):
    /// a restored conversation is pre-checked before the first request;
    /// when the ESTIMATE exceeds this conservative threshold (min of the
    /// 224K effective input budget and the 200K fallback = 200K), old
    /// whole rounds are mechanically dropped toward `recovery_target_tokens`.
    pub recovery_trigger_tokens: u64,
    /// D2-2: recovery truncation target (160K — under the ordinary summary
    /// trigger, so the first measured round may then drive a template
    /// summary instead of another raw truncation).
    pub recovery_target_tokens: u64,
    /// P0-D review fix (2026-08-14, ADR-0010 v1.14): the end-of-session
    /// compaction gate — a successful run whose FULL conversation estimate
    /// exceeds this threshold compacts once before the sidecar write-back,
    /// pinning the summary marker into the persisted conversation (the
    /// restore pre-check remains the fallback for older sidecars).
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
            target_tokens: 90_000,
            min_rounds: 2,
            safety_tokens: 256_000,
            slider_window_tokens: DEFAULT_SLIDER_WINDOW_TOKENS,
            slider_resident_tokens: DEFAULT_SLIDER_RESIDENT_TOKENS,
            slider_rhythm_buffer_tokens: DEFAULT_SLIDER_RHYTHM_BUFFER_TOKENS,
            rhythm_tokens_override: None,
            hard_context_tokens: DEFAULT_CONTEXT_SCALE_HARD_TOKENS,
            window_upload_cap_tokens: DEFAULT_WINDOW_UPLOAD_CAP_TOKENS,
            context_scale_milestones: DEFAULT_CONTEXT_SCALE_MILESTONES,
            recovery_trigger_tokens: 200_000,
            recovery_target_tokens: 160_000,
            session_end_trigger_tokens: 160_000,
            recent_tail_rounds: 2,
            min_compactable: 5_000,
            max_reduction_ratio: 0.6,
        }
    }
}

impl ContextCompactConfig {
    /// mechanical rhythm 阈值（视图刻度）＝ H ＋ 缓冲；测试缝隙可显式覆盖
    /// （`with_context_compact`）。
    pub fn rhythm_tokens(&self) -> u64 {
        self.rhythm_tokens_override.unwrap_or_else(|| {
            self.slider_window_tokens
                .saturating_add(self.slider_rhythm_buffer_tokens)
        })
    }
}

impl AgentLoopController {
    /// A6 (2026-08-08): override the explicit context-compaction parameters
    /// (tests use tiny values; production keeps the design §5 A6 defaults).
    ///
    /// 动态上下文滑块 S1（2026-09-15）：本缝隙设置的是 **rhythm** 阈值
    /// （视图刻度）＋ safety 兜底；**滑块两参数 H/L 不在本缝隙内**（保留
    /// 调用者已设的值，理由同 2026-08-19 对 `fold_tail_tokens` 的修复：
    /// 结构更新不得重置别处设好的参数），滑块测试用
    /// `with_slider_window_tokens` / `with_slider_resident_tokens`。
    pub fn with_context_compact(
        mut self,
        rhythm_tokens: u64,
        target_tokens: u64,
        min_rounds: u32,
        safety_tokens: u64,
    ) -> Self {
        self.context_compact = ContextCompactConfig {
            target_tokens,
            min_rounds,
            safety_tokens,
            rhythm_tokens_override: Some(rhythm_tokens),
            // 滑块两参数＋硬兜底：保留既有值（调用顺序无关）。
            slider_window_tokens: self.context_compact.slider_window_tokens,
            slider_resident_tokens: self.context_compact.slider_resident_tokens,
            slider_rhythm_buffer_tokens: self.context_compact.slider_rhythm_buffer_tokens,
            hard_context_tokens: self.context_compact.hard_context_tokens,
            window_upload_cap_tokens: self.context_compact.window_upload_cap_tokens,
            context_scale_milestones: self.context_compact.context_scale_milestones,
            ..ContextCompactConfig::default()
        };
        self
    }

    /// D2-2 (2026-08-14): override the recovery pre-check threshold/target
    /// (tests use tiny values; production keeps 200K/160K).
    pub fn with_recovery_compact(mut self, trigger_tokens: u64, target_tokens: u64) -> Self {
        self.context_compact.recovery_trigger_tokens = trigger_tokens;
        self.context_compact.recovery_target_tokens = target_tokens;
        self
    }

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

    /// 动态上下文滑块 S1（2026-09-15，设计 §3.2）：pin 滑窗上限 H（测试
    /// 缝隙；生产在构造时读 `ORZ_SLIDER_WINDOW_TOKENS`，默认 160K）。
    pub fn with_slider_window_tokens(mut self, tokens: u64) -> Self {
        self.context_compact.slider_window_tokens = tokens.max(1);
        self
    }

    /// 动态上下文滑块 S1：pin 驻留带 L（测试缝隙；生产在构造时读
    /// `ORZ_SLIDER_RESIDENT_TOKENS`，默认 64K）。驱逐后视图保留的完整轮
    /// 预算＝L（估计口径 chars/2），驱逐深度＝H−L。
    pub fn with_slider_resident_tokens(mut self, tokens: u64) -> Self {
        self.context_compact.slider_resident_tokens = tokens.max(1);
        self
    }

    /// 动态上下文滑块 S1：pin 实际上下文硬兜底阈值（测试缝隙；生产读
    /// `ORZ_CONTEXT_SCALE_HARD_TOKENS`，默认 950K）。
    pub fn with_hard_context_tokens(mut self, tokens: u64) -> Self {
        self.context_compact.hard_context_tokens = tokens.max(1);
        self
    }

    /// 审查修正批（2026-09-15，P2⑤「900K 窗口无上限守卫」）：pin 压缩窗口的
    /// 上传上限（测试缝隙；生产读 `ORZ_CONTEXT_SCALE_WINDOW_CAP_TOKENS`，
    /// 默认 1.10M）。窗口轮全量上传越线 ⇒ 不开窗、走机械强制压缩＋异常落账。
    pub fn with_window_upload_cap_tokens(mut self, tokens: u64) -> Self {
        self.context_compact.window_upload_cap_tokens = tokens.max(1);
        self
    }

    /// v7 压缩档位表测试缝隙（S1 修订批，2026-09-15）：用极小刻度驱动
    /// 「纯提醒／必须压缩」两级路径（生产固定 500K/900K，不暴露 env——刻度
    /// 值进文案，改值即改语义）。传入值自动升序化，避免测试写出倒序表。
    pub fn with_context_scale_milestones(mut self, first: u64, second: u64) -> Self {
        let (a, b) = (first.max(1), second.max(1));
        self.context_compact.context_scale_milestones = [a.min(b), a.max(b)];
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

/// 动态上下文滑块 S1（2026-09-15，CONTEXT_DYNAMIC_SLIDER_DESIGN §3.3 参数律，
/// 用户裁定 R2/R4）：滑窗上限 **H** 默认取**保守档 160K**——R ≈ 0.925/(1−L/H)
/// ＝0.925/0.6 ≈ ×1.54，平均工作点 (H+L)/2 ＝ 112K 仍在 128K 质量平台期内
/// ⇒ 本档**不需要质量读数**（DP-9 只在启用 192K/256K 长任务档时生效）。
/// 均衡档 192K/80K（R≈×1.59）、长任务档 256K/96K（R≈×1.48）仅巨大/高压
/// 长任务启用。env `ORZ_SLIDER_WINDOW_TOKENS` 覆盖（构造时解析；缺省/非法/0
/// ＝默认）。**取代** `ORZ_FOLD_TRIGGER_TOKENS`（128K）。
pub const DEFAULT_SLIDER_WINDOW_TOKENS: u64 = 160_000;

/// 动态上下文滑块 S1：驻留带 **L** 默认 64K（L/H ＝ 0.40 ⇒ R ≈ ×1.54，设计
/// §3.3 建议线）。谷值恒 ≥L（对比锯齿形态折后实测谷值 9,600 token——死亡
/// 螺旋根因）。env `ORZ_SLIDER_RESIDENT_TOKENS` 覆盖。**取代** 8K 桥预算
/// `ORZ_FOLD_TAIL_TOKENS`。
pub const DEFAULT_SLIDER_RESIDENT_TOKENS: u64 = 64_000;

/// 动态上下文滑块 S1：rhythm 缓冲（rhythm ＝ H ＋ 缓冲；视图刻度）。32K ⇒
/// 保守档 192K（与旧 192K rhythm 同值、新推导）。env
/// `ORZ_SLIDER_RHYTHM_BUFFER_TOKENS` 覆盖。
pub const DEFAULT_SLIDER_RHYTHM_BUFFER_TOKENS: u64 = 32_000;

/// 动态上下文滑块 S1：机械压缩硬兜底（**实际上下文刻度**）。默认 950K——
/// 设计 §3.4「硬兜底改挂实际上下文（建议 ≥950K 强制一次）」。env
/// `ORZ_CONTEXT_SCALE_HARD_TOKENS` 覆盖。
pub const DEFAULT_CONTEXT_SCALE_HARD_TOKENS: u64 = 950_000;

/// 审查修正批（2026-09-15；审查 P2⑤）：**压缩窗口上传上限**默认值——1.10M
/// （实际上下文估算刻度）。v7 V3 的窗口轮上传 `messages` 全量；provider 单
/// 请求窗口 1M 真实 token ÷ 实测换算 0.77（设计 §3.3.2）≈1.30M 估算，
/// 再留系统提示／工具面／输出预算余量 ⇒ 取 1.10M（对应 ≈0.85M 真实
/// token）。越线不开窗（fail-soft），上传视图照旧被常驻滑窗压在上限内。
/// env `ORZ_CONTEXT_SCALE_WINDOW_CAP_TOKENS` 覆盖。
pub const DEFAULT_WINDOW_UPLOAD_CAP_TOKENS: u64 = 1_100_000;

/// v7 压缩档位表默认值（S1 修订批，2026-09-15；用户裁定保留两级实际上下文
/// 提醒）：**500K ＝ 纯提醒**（不打断、不开窗、不强制，模型可延后）、
/// **900K ＝ 必须压缩一次**（提醒 ＋ 压缩窗口，模型产出语义摘要）。无 env
/// 覆盖——刻度值进文案（改值即改语义）；测试经
/// `with_context_scale_milestones`。
pub const DEFAULT_CONTEXT_SCALE_MILESTONES: [u64; 2] = [500_000, 900_000];

/// Env override 名（单一源；测试缝隙不经进程 env——`parse_*` 纯函数）。
pub const ENV_SLIDER_WINDOW_TOKENS: &str = "ORZ_SLIDER_WINDOW_TOKENS";
pub const ENV_SLIDER_RESIDENT_TOKENS: &str = "ORZ_SLIDER_RESIDENT_TOKENS";
pub const ENV_SLIDER_RHYTHM_BUFFER_TOKENS: &str = "ORZ_SLIDER_RHYTHM_BUFFER_TOKENS";
pub const ENV_CONTEXT_SCALE_HARD_TOKENS: &str = "ORZ_CONTEXT_SCALE_HARD_TOKENS";
pub const ENV_CONTEXT_SCALE_WINDOW_CAP_TOKENS: &str = "ORZ_CONTEXT_SCALE_WINDOW_CAP_TOKENS";

/// Pure parse rule for every slider env value (tested without env mutation):
/// trimmed, positive integer; absent/invalid/zero → None (＝默认).
pub(crate) fn parse_slider_tokens(s: &str) -> Option<u64> {
    s.trim().parse().ok().filter(|v| *v > 0)
}

/// Env override for the slider window cap H. Parsed at controller
/// construction; absent/invalid/zero = the default.
pub fn slider_window_tokens_override() -> Option<u64> {
    std::env::var(ENV_SLIDER_WINDOW_TOKENS)
        .ok()
        .and_then(|s| parse_slider_tokens(&s))
}

/// Env override for the resident band L（驱逐后保留的完整轮预算）。
pub fn slider_resident_tokens_override() -> Option<u64> {
    std::env::var(ENV_SLIDER_RESIDENT_TOKENS)
        .ok()
        .and_then(|s| parse_slider_tokens(&s))
}

/// Env override for the rhythm buffer（rhythm ＝ H ＋ 缓冲）。
pub fn slider_rhythm_buffer_tokens_override() -> Option<u64> {
    std::env::var(ENV_SLIDER_RHYTHM_BUFFER_TOKENS)
        .ok()
        .and_then(|s| parse_slider_tokens(&s))
}

/// Env override for the actual-context hard fallback（机械压缩兜底刻度）。
pub fn context_scale_hard_tokens_override() -> Option<u64> {
    std::env::var(ENV_CONTEXT_SCALE_HARD_TOKENS)
        .ok()
        .and_then(|s| parse_slider_tokens(&s))
}

/// Env override for the compression-window upload cap（审查修正批 2026-09-15）。
pub fn window_upload_cap_tokens_override() -> Option<u64> {
    std::env::var(ENV_CONTEXT_SCALE_WINDOW_CAP_TOKENS)
        .ok()
        .and_then(|s| parse_slider_tokens(&s))
}

/// A6 §8 C.2 (2026-08-08): default cumulative character cap for the
/// compaction whitelist (16K — user decision; ≈8K tokens ≈ ~9% of the
/// 90K compacted target, small enough not to squeeze the kept rounds).
pub const DEFAULT_WHITELIST_CAP: usize = 16 * 1024;

/// Result of one explicit context compaction (A6).
pub(crate) struct CompactionStats {
    pub(crate) rounds_dropped: u32,
    pub(crate) messages_dropped: usize,
    pub(crate) messages_kept: usize,
    pub(crate) estimated_tokens_after: u64,
    /// Index at which the caller must insert the compaction marker — the
    /// cut point: after the preamble, before the first kept round.
    pub(crate) marker_index: usize,
}

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

/// A6 (2026-08-08): explicit context compaction — drop complete OLDER tool
/// rounds so the remaining conversation (preamble + newest rounds) is
/// estimated under `target_tokens`.
///
/// Round = one assistant declaration message (with `tool_calls`) plus every
/// message up to the next declaration — the provider protocol requires each
/// surviving tool reply's `tool_call_id` to match a declaration in history
/// (a round split across the cut would 400 on the next request, 2026-08-06
/// design review D2-1), so compaction never splits a round. The preamble
/// (original user prompt, gate blocks) and at least the NEWEST round are
/// always kept verbatim (精确段保留 — design §5 A6: 最近 K 轮消息原文).
///
/// The caller inserts the marker (`context_compressed_marker`) at
/// `marker_index` and journals the `context_compressed` event.
pub(crate) fn compact_messages(messages: &mut Vec<Message>, target_tokens: u64) -> CompactionStats {
    let round_starts: Vec<usize> = messages
        .iter()
        .enumerate()
        .filter(|(_, m)| m.role == Role::Assistant && !m.tool_calls.is_empty())
        .map(|(i, _)| i)
        .collect();
    let noop = || CompactionStats {
        rounds_dropped: 0,
        messages_dropped: 0,
        messages_kept: messages.len(),
        estimated_tokens_after: estimate_messages_tokens(messages),
        marker_index: 0,
    };
    if round_starts.is_empty() {
        return noop();
    }
    let preamble_end = round_starts[0];
    let preamble_tokens = estimate_messages_tokens(&messages[..preamble_end]);
    let mut round_estimates: Vec<u64> = Vec::with_capacity(round_starts.len());
    for (k, &start) in round_starts.iter().enumerate() {
        let end = round_starts.get(k + 1).copied().unwrap_or(messages.len());
        round_estimates.push(estimate_messages_tokens(&messages[start..end]));
    }
    // Walk from the NEWEST round backward, keeping while the total fits the
    // target; the newest round is always kept even when it alone exceeds it
    // (recent context stays exact — the target is an estimate anyway).
    let mut kept_total = preamble_tokens;
    let mut kept_count = 0usize;
    for estimate in round_estimates.iter().rev() {
        if kept_count > 0 && kept_total + estimate > target_tokens {
            break;
        }
        kept_count += 1;
        kept_total += estimate;
    }
    let rounds_dropped = round_starts.len() - kept_count;
    if rounds_dropped == 0 {
        return noop();
    }
    let cut = round_starts[round_starts.len() - kept_count];
    let messages_dropped = messages.drain(preamble_end..cut).count();
    CompactionStats {
        rounds_dropped: rounds_dropped as u32,
        messages_dropped,
        messages_kept: messages.len(),
        estimated_tokens_after: kept_total,
        marker_index: preamble_end,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blackboard::EditRecord;
    use crate::controller_test_support::*;
    use crate::gateway::fake::{FakeProvider, ScriptedResponse};
    use crate::gateway::model::{FinishReason, ModelGateway, ModelRequest, ToolCall};
    use crate::host::{
        PermitDecision, PermitError, RiskClass, ToolError, ToolRegistry, ToolResult,
    };
    use async_trait::async_trait;
    use orz_assurance::{EventType, JournalRecorder, RunEvent};
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU64, Ordering};

    /// A6 (2026-08-08): the pure compaction function — drops only the
    /// OLDEST rounds, keeps the preamble + newest round(s), and every
    /// surviving tool reply's `tool_call_id` still matches a declaration
    /// (rounds are never split — the provider 400s on unmatched ids).
    #[test]
    fn compact_messages_preserves_pairing_and_drops_oldest_rounds() {
        let decl = |id: &str| Message {
            role: Role::Assistant,
            content: String::new(),
            tool_call_id: None,
            tool_calls: vec![ToolCall {
                name: "read_file".to_string(),
                arguments: serde_json::json!({}),
                call_id: id.to_string(),
            }],
            reasoning_content: None,
            round: None,
        };
        let tool_reply = |id: &str| Message {
            role: Role::Tool,
            content: "tool output".to_string(),
            tool_call_id: Some(id.to_string()),
            tool_calls: Vec::new(),
            reasoning_content: None,
            round: None,
        };
        let summary = |text: &str| Message {
            role: Role::Assistant,
            content: text.to_string(),
            tool_call_id: None,
            tool_calls: Vec::new(),
            reasoning_content: None,
            round: None,
        };
        let mut messages = vec![
            Message {
                role: Role::User,
                content: "原始提示词".to_string(),
                tool_call_id: None,
                tool_calls: Vec::new(),
                reasoning_content: None,
                round: None,
            },
            decl("call-1"),
            tool_reply("call-1"),
            summary("第一轮总结"),
            decl("call-2"),
            tool_reply("call-2"),
            summary("第二轮总结"),
            decl("call-3"),
            tool_reply("call-3"),
            summary("第三轮总结"),
        ];

        // Target small enough that only the newest round fits.
        let stats = compact_messages(&mut messages, 10);
        assert_eq!(stats.rounds_dropped, 2);
        assert_eq!(stats.messages_dropped, 6);
        assert_eq!(stats.marker_index, 1); // after the preamble

        // Preamble + marker slot + newest round (decl + reply + summary).
        assert_eq!(messages.len(), 4, "{messages:?}");
        assert_eq!(messages[0].content, "原始提示词");
        assert_eq!(messages[1].role, Role::Assistant);
        assert_eq!(messages[1].tool_calls[0].call_id, "call-3");
        assert_eq!(messages[2].tool_call_id.as_deref(), Some("call-3"));
        assert_eq!(messages[3].content, "第三轮总结");

        // Pairing invariant: every tool message's call_id has a declaration.
        let declared: Vec<&str> = messages
            .iter()
            .filter(|m| !m.tool_calls.is_empty())
            .flat_map(|m| m.tool_calls.iter().map(|t| t.call_id.as_str()))
            .collect();
        for m in &messages {
            if let Some(id) = &m.tool_call_id {
                assert!(declared.contains(&id.as_str()), "unmatched {id}");
            }
        }
    }

    /// A6: no tool rounds → compaction is a no-op (nothing droppable).
    #[test]
    fn compact_messages_noop_without_rounds() {
        let mut messages = vec![Message {
            role: Role::User,
            content: "hi".to_string(),
            tool_call_id: None,
            tool_calls: Vec::new(),
            reasoning_content: None,
            round: None,
        }];
        let stats = compact_messages(&mut messages, 0);
        assert_eq!(stats.rounds_dropped, 0);
        assert_eq!(stats.messages_dropped, 0);
        assert_eq!(messages.len(), 1);
    }

    /// P0-D (2026-08-14, ADR-0010 v1.10): the template summary fires at
    /// the first eligible loop-top gap after the cooldown when measured
    /// prompt tokens cross the trigger — INCLUDING mid-task tool gaps (the
    /// old A6 "final-answer gap only" rhythm is revoked). The summary call
    /// is a pure chat round; the marker carries the archive pointer, the
    /// archive is written under `.gsa/compaction/`, and the
    /// `context_compressed` event carries the v0.2 fields.
    #[tokio::test]
    async fn context_compact_summary_fires_at_mid_task_gap() {
        // Host returning a DIFFERENT fat output per call — so the kept
        // newest rounds are distinguishable from the dropped oldest round.
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
        // Fat per-round outputs (~600 chars each) — distinguishable A/B/C.
        let host = SeqHost {
            journal,
            outputs: vec!["A".repeat(600), "B".repeat(600), "C".repeat(600)],
            calls: AtomicU64::new(0),
            cwd: dir.clone(),
        };
        let tool_call = |id: &str| ScriptedResponse {
            text: None,
            tool_calls: vec![ToolCall {
                name: "read_file".to_string(),
                arguments: serde_json::json!({"target_file": "a.txt"}),
                call_id: id.to_string(),
            }],
            finish_reason: FinishReason::ToolCalls,
            reasoning_content: None,
            prompt_tokens: Some(50_000), // over the tiny test trigger
        };
        let fake = Arc::new(FakeProvider::new(vec![
            tool_call("call-a1"),
            tool_call("call-a2"),
            tool_call("call-a3"),
            // 机械模式（2026-08-18 B 定案）：压缩零模型调用——脚本不消费
            // 任何摘要项，收到的请求数即为主循环请求数。
            ScriptedResponse::text("候选答案").with_prompt_tokens(5_000),
            ScriptedResponse::text("最终答案").with_prompt_tokens(5_000),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway)
            .with_context_compact(1_000, 400, 2, 100_000)
            // v7（S1 修订批）：未折叠路径的切断点改以**驻留带 L** 为界
            // （不再按 `recent_tail_rounds` 轮数切）⇒ 本测试把 L 钉成
            // 「最新 2 轮」的估计量，复现既有切断点（每轮 ≈320 估计）。
            .with_slider_resident_tokens(800)
            .with_summary_guards(1, 1.0);
        controller
            .run_turn(
                &host,
                "压缩测试",
                "RUN-COMPACT",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        // The compaction event is journaled with the v0.2 template-summary
        // fields.
        let compact_events: Vec<serde_json::Value> = events(&dir)
            .into_iter()
            .filter(|e| e.event_type == EventType::ContextCompressed)
            .map(|e| e.payload)
            .collect();
        assert_eq!(compact_events.len(), 1, "exactly one summary");
        assert_eq!(compact_events[0]["trigger_tokens"], 50_000);
        // Three completed tool rounds before the summary.
        assert_eq!(compact_events[0]["rounds_since_last_compaction"], 3);
        assert_eq!(compact_events[0]["rounds_dropped"], 1);
        assert_eq!(compact_events[0]["mode"], "mechanical");
        assert_eq!(compact_events[0]["reason"], "rhythm");
        assert_eq!(compact_events[0]["summary_incomplete"], false);
        assert_eq!(compact_events[0]["retained_rounds"], 2);

        // The summary archive was written under .gsa/compaction/.
        let archive_dir = dir.join(".gsa").join("compaction");
        let archives: Vec<_> = std::fs::read_dir(&archive_dir)
            .expect("archive dir exists")
            .flatten()
            .collect();
        assert_eq!(archives.len(), 1, "{archives:?}");
        let archive_text = std::fs::read_to_string(archives[0].path()).unwrap();
        assert!(
            archive_text.contains("# ORZ 会话压缩摘要"),
            "{archive_text}"
        );
        assert!(archive_text.contains("机械模式"));
        // P2-14 S1：v0.3 存档含折叠视图快照各段（主车道）。
        assert!(
            archive_text.contains("## 近窗明细（round < r_keep，已排除保留尾）"),
            "{archive_text}"
        );
        assert!(archive_text.contains("## 查询指针"), "{archive_text}");

        // The MID-TASK gap (request 4, after tool round 3) carries the
        // marker — the old "final-answer gap only" semantics are revoked.
        let received = fake.received_requests();
        assert_eq!(
            received.len(),
            5,
            "机械压缩零模型调用：3 工具轮 + 候选答案 + 最终答案 = 5 请求，{received:?}"
        );
        let marker_idx = received
            .iter()
            .position(|r| {
                r.messages
                    .iter()
                    .any(|m| m.content.starts_with("[前文上下文已压缩"))
            })
            .expect("marker request present");
        assert!(
            marker_idx >= 3,
            "marker must appear only after the tool rounds (mid-task): \
             request {marker_idx}"
        );
        assert!(
            received[..marker_idx].iter().all(|r| r
                .messages
                .iter()
                .all(|m| !m.content.starts_with("[前文上下文已压缩"))),
            "no marker before the summary"
        );
        let round4 = &received[marker_idx].messages;
        assert!(
            round4
                .iter()
                .any(|m| m.content.starts_with("[前文上下文已压缩")),
            "marker present at the mid-task gap: {round4:?}"
        );
        // Oldest round dropped; the newest two kept verbatim (pairing
        // intact).
        assert!(
            round4.iter().all(|m| !m.content.contains(&"A".repeat(600))),
            "oldest round's output gone: {round4:?}"
        );
        assert!(
            round4.iter().any(|m| m.content.contains(&"B".repeat(600)))
                && round4.iter().any(|m| m.content.contains(&"C".repeat(600))),
            "newest rounds kept: {round4:?}"
        );
        let declared: Vec<&str> = round4
            .iter()
            .filter(|m| m.role == Role::Assistant)
            .flat_map(|m| m.tool_calls.iter().map(|tc| tc.call_id.as_str()))
            .collect();
        for m in round4.iter().filter(|m| m.role == Role::Tool) {
            assert!(
                m.tool_call_id
                    .as_deref()
                    .is_some_and(|id| declared.contains(&id)),
                "orphan tool result: {m:?}"
            );
        }
        // P2-14 S1（2026-09-04，ADR-0010 §14.54）：主车道压缩走 v0.3 折叠
        // 视图快照 marker——A–E 块齐全、r_keep 单边界行排除（既有 A 轮
        // 原文不在 marker/消息中即 r_keep 排除的等价断言），v0.2 五段槽
        // 不在主车道 marker 出现。
        let marker = round4
            .iter()
            .find(|m| m.content.starts_with("[前文上下文已压缩"))
            .expect("marker present");
        assert!(
            marker.content.contains("[前文上下文已压缩 v0.3]"),
            "main-lane compaction must emit the v0.3 marker: {}",
            marker.content
        );
        for head in [
            "保留尾首轮 r_keep=",
            "== 近窗明细（round < r_keep，已排除保留尾） ==",
            "== 旧段聚合",
            "== 失败目标聚合 ==",
            "== 查询指针 ==",
            "[/前文上下文已压缩]",
        ] {
            assert!(marker.content.contains(head), "missing {head}");
        }
        assert!(
            !marker.content.contains("目的: ") && !marker.content.contains("后续衔接: "),
            "v0.2 五段槽不得出现在主车道 v0.3 marker"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// THIN-HARNESS-REDESIGN R1 (§4.1 边界项, 2026-08-27)：compaction_
    /// whitelist_add 已封存——即使首轮批次调用也一律 sealed_tool_denied
    /// 结构化拒绝（无 ToolStarted、白名单不落盘、无 whitelist.jsonl
    /// 存档、零副作用），运行正常继续（旧 A6 常驻/压缩保真机制已退役，
    /// 代码休眠保留，R3 统一裁决）。
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
            .with_context_compact(1_000, 400, 1, 100_000)
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

    /// P0-D (2026-08-14): the summary requires a measured trigger — a run
    /// whose rounds report usage BELOW the trigger never compacts, even at
    /// the final-answer gap (the old A6 gap-only semantics are gone; the
    /// measured threshold is the gate).
    #[tokio::test]
    async fn context_compact_requires_measured_trigger() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: Some(ToolResult {
                output: "x".repeat(600),
                exit_code: Some(0),
                output_encoding: None,
                structured: None,
                ..Default::default()
            }),
        };
        let tool_call = |id: &str| ScriptedResponse {
            text: None,
            tool_calls: vec![ToolCall {
                name: "read_file".to_string(),
                arguments: serde_json::json!({"target_file": "a.txt"}),
                call_id: id.to_string(),
            }],
            finish_reason: FinishReason::ToolCalls,
            reasoning_content: None,
            prompt_tokens: Some(100), // below the tiny test trigger
        };
        let fake = Arc::new(FakeProvider::new(vec![
            tool_call("call-g1"),
            tool_call("call-g2"),
            tool_call("call-g3"),
            ScriptedResponse::text("候选答案").with_prompt_tokens(100),
            ScriptedResponse::text("最终答案").with_prompt_tokens(100),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway)
            .with_context_compact(1_000, 400, 1, 100_000)
            .with_summary_guards(1, 1.0);
        controller
            .run_turn(
                &host,
                "压缩测试",
                "RUN-COMPACT-NOTRIG",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let compact_events: Vec<serde_json::Value> = events(&dir)
            .into_iter()
            .filter(|e| e.event_type == EventType::ContextCompressed)
            .map(|e| e.payload)
            .collect();
        assert!(compact_events.is_empty(), "{compact_events:?}");
        for request in fake.received_requests() {
            assert!(
                request
                    .messages
                    .iter()
                    .all(|m| !m.content.contains("[前文上下文已压缩")),
                "no marker without trigger: {request:?}"
            );
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A6: the min-round cooldown suppresses compaction — a trigger token
    /// count alone is not enough.
    #[tokio::test]
    async fn context_compact_respects_min_rounds_cooldown() {
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
        let tool_call = |id: &str| ScriptedResponse {
            text: None,
            tool_calls: vec![ToolCall {
                name: "read_file".to_string(),
                arguments: serde_json::json!({"target_file": "a.txt"}),
                call_id: id.to_string(),
            }],
            finish_reason: FinishReason::ToolCalls,
            reasoning_content: None,
            prompt_tokens: Some(5_000),
        };
        let fake = Arc::new(FakeProvider::new(vec![
            tool_call("call-b1"),
            tool_call("call-b2"),
            ScriptedResponse::text("第一轮完成").with_prompt_tokens(5_000),
            ScriptedResponse::text("最终答案").with_prompt_tokens(5_000),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller =
            AgentLoopController::with_gateway(gateway).with_context_compact(1_000, 400, 5, 100_000); // cooldown longer than the run
        controller
            .run_turn(
                &host,
                "压缩测试",
                "RUN-COMPACT-NO",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let compact_events: Vec<serde_json::Value> = events(&dir)
            .into_iter()
            .filter(|e| e.event_type == EventType::ContextCompressed)
            .map(|e| e.payload)
            .collect();
        assert!(
            compact_events.is_empty(),
            "cooldown suppressed: {compact_events:?}"
        );
        // No marker in any request.
        for request in fake.received_requests() {
            assert!(
                request
                    .messages
                    .iter()
                    .all(|m| !m.content.contains("[前文上下文已压缩")),
                "no marker without compaction: {request:?}"
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// P0-D window guard (ADR-0010 v1.10 §2): measured tokens above
    /// `safety_tokens` (the 200K fallback) fire the template summary
    /// IMMEDIATELY — the cooldown is bypassed so a high-start task never
    /// approaches the provider window while waiting for the interval. The
    /// guard re-fires on every over-safety round that still has droppable
    /// content, journaling the honest (short) rounds_since_last_compaction.
    #[tokio::test]
    async fn context_compact_safety_trigger_bypasses_cooldown() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: Some(ToolResult {
                output: "x".repeat(600), // fat rounds — droppable content
                exit_code: Some(0),
                output_encoding: None,
                structured: None,
                ..Default::default()
            }),
        };
        let tool_call = |id: &str| ScriptedResponse {
            text: None,
            tool_calls: vec![ToolCall {
                name: "read_file".to_string(),
                arguments: serde_json::json!({"target_file": "a.txt"}),
                call_id: id.to_string(),
            }],
            finish_reason: FinishReason::ToolCalls,
            reasoning_content: None,
            prompt_tokens: Some(300_000), // over the safety gate
        };
        let fake = Arc::new(FakeProvider::new(vec![
            tool_call("call-s1"),
            tool_call("call-s2"),
            tool_call("call-s3"),
            tool_call("call-s4"),
            // 机械模式（2026-08-18 B 定案）：两次 fallback 压缩均零模型
            // 调用——round 3 后与 round 4 后各触发一次，脚本无摘要项。
            ScriptedResponse::text("候选答案"),
            ScriptedResponse::text("最终答案"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        // Cooldown 20 — far longer than the run — but the fallback trigger
        // (100_000) must fire regardless of the cooldown.
        let controller = AgentLoopController::with_gateway(gateway)
            .with_context_compact(150_000, 400, 20, 100_000)
            // v7（S1 修订批）：未折叠路径以驻留带 L 为界（不再按轮数切）——
            // 钉成「最新 2 轮」的估计量以复现既有切断点。
            .with_slider_resident_tokens(800)
            .with_summary_guards(1, 1.0);
        controller
            .run_turn(
                &host,
                "压缩测试",
                "RUN-COMPACT-SAFE",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let compact_events: Vec<serde_json::Value> = events(&dir)
            .into_iter()
            .filter(|e| e.event_type == EventType::ContextCompressed)
            .map(|e| e.payload)
            .collect();
        assert_eq!(compact_events.len(), 2, "fallback fired twice");
        // Every summary happened FAR before the 20-round cooldown — the
        // payload honestly reports the bypassed interval.
        for event in &compact_events {
            assert_eq!(event["reason"], "fallback");
            assert_eq!(event["trigger_tokens"], 300_000);
            assert!(
                event["rounds_since_last_compaction"].as_u64().unwrap() < 20,
                "cooldown bypassed: {event:?}"
            );
        }

        // The marker reached a request after the first fallback summary.
        let received = fake.received_requests();
        assert!(received.len() >= 3, "{received:?}");
        assert!(
            received.iter().any(|r| r
                .messages
                .iter()
                .any(|m| m.content.starts_with("[前文上下文已压缩"))),
            "marker present after fallback summary"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 2026-08-18 B 定案（ADR-0010 §14.29）：压缩为纯机械——零模型调用、
    /// 无 summary_incomplete 终止态；fallback 触发下存档恒写入、marker 恒
    /// 携带真实 digest/路径，会话机械截断不滞留窗口之上。
    #[tokio::test]
    async fn context_compact_mechanical_mode_zero_model_calls() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: Some(ToolResult {
                output: "x".repeat(600),
                exit_code: Some(0),
                output_encoding: None,
                structured: None,
                ..Default::default()
            }),
        };
        let tool_call = |id: &str| ScriptedResponse {
            text: None,
            tool_calls: vec![ToolCall {
                name: "read_file".to_string(),
                arguments: serde_json::json!({"target_file": "a.txt"}),
                call_id: id.to_string(),
            }],
            finish_reason: FinishReason::ToolCalls,
            reasoning_content: None,
            prompt_tokens: Some(300_000), // over the fallback gate
        };
        let fake = Arc::new(FakeProvider::new(vec![
            tool_call("call-f1"),
            tool_call("call-f2"),
            tool_call("call-f3"),
            // 机械模式：脚本即主循环请求数——压缩不消费任何模型项。
            ScriptedResponse::text("候选答案"),
            ScriptedResponse::text("最终答案"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway)
            .with_context_compact(150_000, 400, 20, 100_000)
            // v7（S1 修订批）：未折叠路径以驻留带 L 为界（不再按轮数切）——
            // 钉成「最新 2 轮」的估计量以复现既有切断点。
            .with_slider_resident_tokens(800)
            .with_summary_guards(1, 1.0);
        controller
            .run_turn(
                &host,
                "压缩测试",
                "RUN-COMPACT-FAIL",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let compact_events: Vec<serde_json::Value> = events(&dir)
            .into_iter()
            .filter(|e| e.event_type == EventType::ContextCompressed)
            .map(|e| e.payload)
            .collect();
        assert_eq!(compact_events.len(), 1, "one mechanical compaction");
        assert_eq!(compact_events[0]["mode"], "mechanical");
        assert_eq!(compact_events[0]["summary_incomplete"], false);
        assert_eq!(compact_events[0]["reason"], "fallback");
        assert!(compact_events[0]["summary_id"].as_str().is_some());
        assert!(compact_events[0]["summary_digest"].as_str().is_some());
        assert!(compact_events[0]["summary_path"].as_str().is_some());

        // 零模型调用：3 工具轮 + 候选答案 + 最终答案 = 5 请求。
        let received = fake.received_requests();
        assert_eq!(received.len(), 5, "{received:?}");
        // The marker reaches the model and carries the placeholder slots —
        // no `summary_incomplete` termination flag.
        let last = received.last().unwrap();
        assert!(
            last.messages
                .iter()
                .any(|m| m.content.starts_with("[前文上下文已压缩")),
            "marker present: {:?}",
            last.messages
        );
        assert!(
            !last
                .messages
                .iter()
                .any(|m| m.content.contains("summary_incomplete")),
            "机械模式 marker 不得携带 termination 标志: {:?}",
            last.messages
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// P0-D review fix (2026-08-14, ADR-0010 v1.14) + FUS-LEDGER-FOLD-STATE
    /// (2026-08-18, ADR-0010 §14.26): a reduction guard that cannot be
    /// satisfied retries across trigger rounds WITHOUT interrupting
    /// content or raw truncation; after GUARD_RETRY_LIMIT consecutive
    /// failures one compaction is forced and reported `guard_failed`.
    #[test]
    fn context_compact_defaults_follow_v1_14_review() {
        let cfg = ContextCompactConfig::default();
        // Cooldown is 2 MODEL rounds (review fix — avoids long-action
        // accumulation); the session-end gate defaults to the 160K rhythm
        // threshold; the recovery pre-check stays 200K/160K.
        assert_eq!(cfg.min_rounds, 2);
        assert_eq!(cfg.session_end_trigger_tokens, 160_000);
        assert_eq!(cfg.recovery_trigger_tokens, 200_000);
        assert_eq!(cfg.recovery_target_tokens, 160_000);
        assert_eq!(cfg.recent_tail_rounds, 2);
        // 动态上下文滑块 S1（2026-09-15，设计 §3.3，用户裁定 R4）：默认
        // **保守档 H=160K / L=64K**（R≈×1.54、工作点均值 112K 仍在 128K
        // 平台期内 ⇒ 不需要质量读数）；rhythm ＝ H＋缓冲 ＝ 192K（与旧
        // 192K 同值、新推导）；硬兜底挂**实际上下文** 950K。
        assert_eq!(cfg.slider_window_tokens, DEFAULT_SLIDER_WINDOW_TOKENS);
        assert_eq!(DEFAULT_SLIDER_WINDOW_TOKENS, 160_000);
        assert_eq!(cfg.slider_resident_tokens, DEFAULT_SLIDER_RESIDENT_TOKENS);
        assert_eq!(DEFAULT_SLIDER_RESIDENT_TOKENS, 64_000);
        assert_eq!(
            cfg.slider_rhythm_buffer_tokens,
            DEFAULT_SLIDER_RHYTHM_BUFFER_TOKENS
        );
        assert_eq!(DEFAULT_SLIDER_RHYTHM_BUFFER_TOKENS, 32_000);
        assert_eq!(cfg.rhythm_tokens(), 192_000, "rhythm ＝ H ＋ 缓冲");
        assert_eq!(cfg.hard_context_tokens, DEFAULT_CONTEXT_SCALE_HARD_TOKENS);
        assert_eq!(DEFAULT_CONTEXT_SCALE_HARD_TOKENS, 950_000);
        // 审查修正批（2026-09-15，P2⑤）：压缩窗口上传上限（fail-soft 降级线）。
        assert_eq!(
            cfg.window_upload_cap_tokens,
            DEFAULT_WINDOW_UPLOAD_CAP_TOKENS
        );
        assert_eq!(DEFAULT_WINDOW_UPLOAD_CAP_TOKENS, 1_100_000);
        // 驱逐深度 = H−L = 96K（成本律 R ≈ 0.925/(1−0.4) ≈ ×1.54）。
        assert_eq!(
            cfg.slider_window_tokens - cfg.slider_resident_tokens,
            96_000
        );
        // 2026-08-18 adjudication (ADR-0010 §14.26): 256K 视图刻度兜底
        // （S1 起常态不可达，保留为 S2 展开膨胀路径）。
        assert_eq!(cfg.safety_tokens, 256_000);
    }

    /// 动态上下文滑块 S1（2026-09-15，设计 §3.2/§3.3）：四个 `ORZ_SLIDER_*`
    /// /`ORZ_CONTEXT_SCALE_*` env 共用同一解析规则——trim 后正整数；缺省/
    /// 非法/0 ⇒ 默认（测试不经进程 env：纯函数缝隙）。
    #[test]
    fn slider_tokens_parse_rule() {
        assert_eq!(crate::compact::parse_slider_tokens("64000"), Some(64_000));
        assert_eq!(crate::compact::parse_slider_tokens(" 64000 "), Some(64_000));
        assert_eq!(crate::compact::parse_slider_tokens("1"), Some(1));
        assert_eq!(crate::compact::parse_slider_tokens("0"), None, "0 = 默认");
        assert_eq!(crate::compact::parse_slider_tokens("-1"), None);
        assert_eq!(crate::compact::parse_slider_tokens("abc"), None);
        assert_eq!(crate::compact::parse_slider_tokens(""), None);
        // env 名是契约面（文档/账本逐字引用）。
        assert_eq!(ENV_SLIDER_WINDOW_TOKENS, "ORZ_SLIDER_WINDOW_TOKENS");
        assert_eq!(ENV_SLIDER_RESIDENT_TOKENS, "ORZ_SLIDER_RESIDENT_TOKENS");
        assert_eq!(
            ENV_SLIDER_RHYTHM_BUFFER_TOKENS,
            "ORZ_SLIDER_RHYTHM_BUFFER_TOKENS"
        );
        assert_eq!(
            ENV_CONTEXT_SCALE_HARD_TOKENS,
            "ORZ_CONTEXT_SCALE_HARD_TOKENS"
        );
        assert_eq!(
            ENV_CONTEXT_SCALE_WINDOW_CAP_TOKENS,
            "ORZ_CONTEXT_SCALE_WINDOW_CAP_TOKENS"
        );
    }

    /// FUS-LEDGER-FOLD-STATE (2026-08-18, ADR-0010 §14.26): the fold point
    /// advances mechanically once the ESTIMATED request view reaches
    /// `fold_trigger_tokens` (loop-top gap, zero model calls). Before the
    /// advance every request view is `messages` verbatim (no per-request
    /// stateless collapse — byte-stable from the first round); between
    /// advances the request view is the anchored view + pure append
    /// (byte-identical prefix), and only a mechanical advance (a new
    /// frozen ledger version) rewrites the prefix — one accepted rewrite
    /// per fold window.
    #[tokio::test]
    async fn fold_state_advances_once_and_prefix_stays_stable() {
        struct SeqHost {
            journal: JournalRecorder,
            outputs: Vec<String>,
            calls: AtomicU64,
            cwd: PathBuf,
        }
        #[async_trait]
        impl LoopHost for SeqHost {
            fn journal(&self) -> &JournalRecorder {
                &self.journal
            }
            fn tools_registry(&self) -> &dyn ToolRegistry {
                &EmptyRegistry
            }
            fn session_cwd(&self) -> PathBuf {
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
            outputs: vec![
                "AAAA".repeat(200),
                "BBBB".repeat(200),
                "CCCC".repeat(200),
                "DDDD".repeat(200),
            ],
            calls: AtomicU64::new(0),
            cwd: dir.clone(),
        };
        let tool_call = |id: &str| ScriptedResponse {
            text: None,
            tool_calls: vec![ToolCall {
                name: "read_file".to_string(),
                arguments: serde_json::json!({"target_file": "a.txt"}),
                call_id: id.to_string(),
            }],
            finish_reason: FinishReason::ToolCalls,
            reasoning_content: None,
            prompt_tokens: Some(1_000),
        };
        let mut script = Vec::new();
        for k in 0..9 {
            script.push(tool_call(&format!("call-{k}")));
        }
        // The counterexample gate adds one text-only round before the
        // final answer — keep the script ahead of the loop.
        script.push(ScriptedResponse::text("反例自查通过").with_prompt_tokens(1_000));
        script.push(ScriptedResponse::text("最终答案").with_prompt_tokens(1_000));
        let fake = Arc::new(FakeProvider::new(script));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        // Compaction kept far away (huge triggers + long cooldown) — the
        // test isolates the fold mechanism.
        let controller = AgentLoopController::with_gateway(gateway)
            .with_context_compact(100_000_000, 400, 20, 100_000_000)
            // 阈值高于「preamble + 指针 + 桥（100 token → 100 估计）」的
            // 固定基线，使触发复位断言（推进后估算 < 阈值）真实成立。
            .with_slider_window_tokens(2_000)
            // FUS-LEDGER-FOLD-BRIDGE (2026-08-19, ADR-0010 §14.32): 测试轮
            // 输出 ~800 字符（估计 ~400），桥预算 100 token（估计 200）
            // 使推进时只保留最新 1 轮——否则 9 轮全在 8K 桥内、永不推进。
            .with_slider_resident_tokens(100);
        controller
            .run_turn(&host, "折叠测试", "RUN-FOLD", MANIFEST, 0, None, None, None)
            .await
            .unwrap();

        let received = fake.received_requests();
        assert!(received.len() >= 9, "{received:?}");
        let ledger_idx = received
            .iter()
            .position(|r| {
                r.messages.iter().any(|m| {
                    m.content
                        .starts_with(crate::action_ledger::LEDGER_FOLD_POINTER_PREFIX)
                })
            })
            .expect("a mechanical fold advance happened");
        assert!(ledger_idx > 0, "the first request must stay verbatim");
        for r in &received[..ledger_idx] {
            assert!(
                r.messages.iter().all(|m| !m
                    .content
                    .starts_with(crate::action_ledger::LEDGER_FOLD_POINTER_PREFIX)),
                "no ledger before the mechanical trigger: {r:?}"
            );
        }
        // FUS-LEDGER-FOLD-STATE external-file design (2026-08-18, ADR-0010
        // §14.28): after the first advance the [preamble + fixed pointer]
        // prefix is BYTE-IDENTICAL in every later request — the folded
        // rows live in the external file, so advances never rewrite the
        // view prefix (the cache-critical invariant; measured 81.9% →
        // ~91–93%).
        let pointer_of = |r: &ModelRequest| {
            r.messages
                .iter()
                .find(|m| {
                    m.content
                        .starts_with(crate::action_ledger::LEDGER_FOLD_POINTER_PREFIX)
                })
                .map(|m| m.content.clone())
        };
        let anchor_ptr = pointer_of(&received[ledger_idx]).unwrap();
        let anchor_prefix: Vec<Message> = received[ledger_idx]
            .messages
            .iter()
            .take_while(|m| {
                !m.content
                    .starts_with(crate::action_ledger::LEDGER_FOLD_POINTER_PREFIX)
            })
            .cloned()
            .collect();
        for (i, r) in received.iter().enumerate().skip(ledger_idx) {
            assert_eq!(
                pointer_of(r).as_deref(),
                Some(anchor_ptr.as_str()),
                "pointer message rewritten at request {i}"
            );
            assert!(
                r.messages
                    .iter()
                    .take(anchor_prefix.len())
                    .eq(anchor_prefix.iter()),
                "preamble rewritten at request {i}"
            );
        }
        // Every real fold advance journals `ledger_fold_advance` with the
        // new fold point and the triggering estimate — the mechanical
        // advances (and their miss cost) are attributable in the event
        // chain. Trigger reset: advances must be strictly fewer than the
        // post-fold requests (the view after an advance is pointer + 1
        // round < threshold — no per-request fold storm).
        let fold_events = events(&dir)
            .into_iter()
            .filter(|e| e.event_type == EventType::LedgerFoldAdvance)
            .collect::<Vec<_>>();
        assert!(!fold_events.is_empty(), "fold advance event missing");
        assert!(
            fold_events.len() < received.len() - ledger_idx,
            "advances must be strictly fewer than post-fold requests: {} / {}",
            fold_events.len(),
            received.len() - ledger_idx
        );
        for ev in &fold_events {
            let fold_start = ev.payload["fold_start"].as_u64().unwrap();
            let fold_cut = ev.payload["fold_cut"].as_u64().unwrap();
            let rounds_folded = ev.payload["rounds_folded"].as_u64().unwrap();
            assert!(fold_start < fold_cut, "{ev:?}");
            assert!(rounds_folded >= 1, "{ev:?}");
            assert!(
                ev.payload["view_estimate_tokens"].as_u64().unwrap() >= 2_000,
                "{ev:?}"
            );
            // 触发复位（2026-08-18 审查修复补足 S2 测试 #3 的直接口径）：
            // 推进后视图估算必须回落到阈值之下。
            assert!(
                ev.payload["view_estimate_after"].as_u64().unwrap() < 2_000,
                "post-advance estimate must reset below the trigger: {ev:?}"
            );
            assert_eq!(ev.payload["agent_role"], "main", "{ev:?}");
        }
        // Within a fold window fold_cut is strictly increasing (each
        // advance moves it forward); a reset only happens at compaction.
        let cuts: Vec<u64> = fold_events
            .iter()
            .map(|e| e.payload["fold_cut"].as_u64().unwrap())
            .collect();
        assert!(cuts.windows(2).all(|w| w[0] < w[1]), "{cuts:?}");
        // The folded rows land in the external append-only projection.
        let ledger_file = dir.join(".gsa").join("ledger").join("current.md");
        let text = std::fs::read_to_string(&ledger_file).expect("external ledger file written");
        let lines: Vec<&str> = text.lines().collect();
        assert!(!lines.is_empty(), "external ledger must hold folded rows");
        assert!(
            lines.iter().all(|l| l.starts_with('[')),
            "every external row carries a global seq: {text}"
        );
        assert!(
            lines
                .iter()
                .any(|l| l.contains("read_file") && l.contains("目标=")),
            "rows carry tool/target/pointer/reply: {text}"
        );
    }

    /// FUS-LEDGER-FOLD-STATE external-file design (2026-08-18, ADR-0010
    /// §14.28 审查修复): a persistent external-ledger append failure must
    /// NOT spin the loop — after FOLD_WRITE_FAILURE_LIMIT consecutive
    /// failures folding is disabled for the loop, the session keeps making
    /// model requests with the unfolded view, and every failure is
    /// journaled (`ledger_fold_write_failed`; 此前 `continue` 会在持久写
    /// 失败时形成无模型调用的空转).
    #[tokio::test]
    async fn fold_append_failure_disables_fold_and_keeps_session_going() {
        struct SeqHost {
            journal: JournalRecorder,
            outputs: Vec<String>,
            calls: AtomicU64,
            cwd: PathBuf,
        }
        #[async_trait]
        impl LoopHost for SeqHost {
            fn journal(&self) -> &JournalRecorder {
                &self.journal
            }
            fn tools_registry(&self) -> &dyn ToolRegistry {
                &EmptyRegistry
            }
            fn session_cwd(&self) -> PathBuf {
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
        // 用文件占住 `.gsa/ledger` 目录位：`create_dir_all` 必然失败，
        // 复现持久写失败路径。
        std::fs::create_dir_all(dir.join(".gsa")).unwrap();
        std::fs::write(dir.join(".gsa").join("ledger"), "不是目录").unwrap();
        let journal = JournalRecorder::new(dir.clone());
        let host = SeqHost {
            journal,
            outputs: vec![
                "AAAA".repeat(200),
                "BBBB".repeat(200),
                "CCCC".repeat(200),
                "DDDD".repeat(200),
            ],
            calls: AtomicU64::new(0),
            cwd: dir.clone(),
        };
        let tool_call = |id: &str| ScriptedResponse {
            text: None,
            tool_calls: vec![ToolCall {
                name: "read_file".to_string(),
                arguments: serde_json::json!({"target_file": "a.txt"}),
                call_id: id.to_string(),
            }],
            finish_reason: FinishReason::ToolCalls,
            reasoning_content: None,
            prompt_tokens: Some(1_000),
        };
        let mut script = Vec::new();
        for k in 0..9 {
            script.push(tool_call(&format!("call-{k}")));
        }
        script.push(ScriptedResponse::text("反例自查通过").with_prompt_tokens(1_000));
        script.push(ScriptedResponse::text("最终答案").with_prompt_tokens(1_000));
        let expected_requests = script.len();
        let fake = Arc::new(FakeProvider::new(script));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway)
            .with_context_compact(100_000_000, 400, 20, 100_000_000)
            .with_slider_window_tokens(1_000)
            // FUS-LEDGER-FOLD-BRIDGE (2026-08-19, ADR-0010 §14.32): 桥预算
            // 100 token（估计 200）——否则 9 轮全在桥内、推进永不发生、
            // 写失败路径无法复现。
            .with_slider_resident_tokens(100);
        controller
            .run_turn(
                &host,
                "折叠写失败",
                "RUN-FOLD-FAIL",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let received = fake.received_requests();
        assert_eq!(
            received.len(),
            expected_requests,
            "会话必须走完所有脚本请求（持久写失败不得空转）"
        );
        assert!(
            received.iter().all(|r| !r.messages.iter().any(|m| {
                m.content
                    .starts_with(crate::action_ledger::LEDGER_FOLD_POINTER_PREFIX)
            })),
            "折叠从未成功：任何请求都不含指针消息"
        );
        let failed = events(&dir)
            .into_iter()
            .filter(|e| e.event_type == EventType::LedgerFoldWriteFailed)
            .collect::<Vec<_>>();
        assert!(
            failed.len() >= crate::agent_loop::FOLD_WRITE_FAILURE_LIMIT as usize,
            "每次失败都要 journal 留痕: {}",
            failed.len()
        );
        assert!(
            failed.iter().all(|e| e.payload["agent_role"] == "main"),
            "{failed:?}"
        );
        assert!(
            failed
                .last()
                .map(|e| e.payload["disabled"].as_bool().unwrap())
                .unwrap_or(false),
            "预算耗尽后折叠必须被禁用: {failed:?}"
        );
        // 失败序列的 attempt 严格递增。
        let attempts: Vec<u64> = failed
            .iter()
            .map(|e| e.payload["attempt"].as_u64().unwrap())
            .collect();
        assert!(attempts.windows(2).all(|w| w[0] < w[1]), "{attempts:?}");
    }

    /// FUS-LEDGER-FOLD-STATE (2026-08-18, ADR-0010 §14.26) × compaction:
    /// the fold advances BEFORE the summary fires; the summary call's
    /// input is the SAME stateful folded view (摘要输入与主请求同源 — it
    /// carries the frozen ledger block); mechanical compaction (2026-08-18
    /// B 定案) then resets the fold state (marker request has no ledger)
    /// and the fold re-accumulates from the marker (a later request shows
    /// the ledger again). The compaction archive preserves the frozen
    /// ledger the model saw (设计 §3.5 第 1 步 — the archive is the only
    /// surviving ledger snapshot after the folded region is drained).
    #[tokio::test]
    async fn fold_state_resets_after_mechanical_compaction_and_archive_keeps_pointer() {
        struct SeqHost {
            journal: JournalRecorder,
            outputs: Vec<String>,
            calls: AtomicU64,
            cwd: PathBuf,
        }
        #[async_trait]
        impl LoopHost for SeqHost {
            fn journal(&self) -> &JournalRecorder {
                &self.journal
            }
            fn tools_registry(&self) -> &dyn ToolRegistry {
                &EmptyRegistry
            }
            fn session_cwd(&self) -> PathBuf {
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
            outputs: vec![
                "AAAA".repeat(300),
                "BBBB".repeat(300),
                "CCCC".repeat(300),
                "DDDD".repeat(300),
            ],
            calls: AtomicU64::new(0),
            cwd: dir.clone(),
        };
        let tool_call = |id: &str| ScriptedResponse {
            text: None,
            tool_calls: vec![ToolCall {
                name: "read_file".to_string(),
                arguments: serde_json::json!({"target_file": "a.txt"}),
                call_id: id.to_string(),
            }],
            finish_reason: FinishReason::ToolCalls,
            reasoning_content: None,
            // 20K keeps the first rhythm trigger (cooldown 5, round 6's
            // loop-top) on the intended mechanical compaction path.
            prompt_tokens: Some(20_000),
        };
        // Rhythm (cooldown 5) fires at round 6's loop-top (the counter
        // reaches 5 when round 5 completes): rounds 0..10 (11 tool rounds)
        // cross two rhythm triggers, both compactions are mechanical (no
        // model items consumed), and the two text rounds cover the
        // counterexample gate + final answer.
        let mut script = Vec::new();
        for k in 0..5 {
            script.push(tool_call(&format!("call-{k}")));
        }
        for k in 5..11 {
            script.push(tool_call(&format!("call-{k}")));
        }
        script.push(ScriptedResponse::text("反例自查通过").with_prompt_tokens(20_000));
        script.push(ScriptedResponse::text("最终答案").with_prompt_tokens(20_000));
        let fake = Arc::new(FakeProvider::new(script));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        // Rhythm cooldown 5 → the fold (trigger 1K) advances several
        // rounds BEFORE the summary (trigger 1K + cooldown 5) fires.
        let controller = AgentLoopController::with_gateway(gateway)
            .with_context_compact(1_000, 400, 5, 100_000_000)
            .with_summary_guards(1, 1.0)
            .with_slider_window_tokens(1_000)
            // FUS-LEDGER-FOLD-BRIDGE (2026-08-19, ADR-0010 §14.32): 桥预算
            // 100 token（估计 200）——否则 11 轮全在桥内、折叠不推进，
            // 「折叠先于压缩」的联动断言无法成立。
            .with_slider_resident_tokens(100);
        controller
            .run_turn(
                &host,
                "折叠压缩联动",
                "RUN-FOLD-COMPACT",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let received = fake.received_requests();
        let has_ledger = |r: &ModelRequest| {
            r.messages.iter().any(|m| {
                m.content
                    .starts_with(crate::action_ledger::LEDGER_FOLD_POINTER_PREFIX)
            })
        };
        let has_marker = |r: &ModelRequest| {
            r.messages
                .iter()
                .any(|m| m.content.starts_with("[前文上下文已压缩"))
        };
        let ledger_idx = received.iter().position(has_ledger).expect("fold advance");
        let marker_idx = received
            .iter()
            .position(has_marker)
            .expect("compaction marker");
        assert!(
            ledger_idx < marker_idx,
            "the fold must advance before the summary fires (fold {ledger_idx} / marker {marker_idx})"
        );
        // 机械模式（2026-08-18 B 定案）：压缩零模型调用——11 工具轮 +
        // 反例自查 + 最终答案 = 13 请求，脚本即主循环请求数。
        assert_eq!(received.len(), 13, "机械压缩零模型调用: {received:?}");
        // The fold re-accumulates from the marker — after the first
        // compaction only the tail rounds survive (no foldable round
        // outside the tail at first), so the ledger reappears once the
        // rounds grow again.
        assert!(
            received[marker_idx..].iter().any(has_ledger),
            "fold re-accumulates after compaction: {received:?}"
        );
        // FUS-LEDGER-FOLD-STATE review fix (2026-08-18, 设计 §3.5 第 1 步):
        // each compaction archive preserves the frozen ledger the model
        // saw — the fixed pointer message (the folded rows survive in the
        // append-only external ledger file, which compaction never drains;
        // ADR-0010 §14.28 external-file design).
        let archive_dir = dir.join(".gsa").join("compaction");
        let archives: Vec<_> = std::fs::read_dir(&archive_dir)
            .expect("archive dir exists")
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .collect();
        assert_eq!(archives.len(), 2, "{archives:?}");
        let ledger_file = dir.join(".gsa").join("ledger").join("current.md");
        let ledger_path_display = ledger_file.display().to_string();
        let pointer = crate::action_ledger::build_pointer_message(&ledger_file);
        for path in &archives {
            let archive = std::fs::read_to_string(path).unwrap();
            assert!(
                archive.contains("## 折叠视图（冻结快照：外挂指针）"),
                "frozen ledger section missing from archive: {archive}"
            );
            assert!(
                archive.contains(&pointer),
                "frozen ledger block missing from archive: {archive}"
            );
        }
        // The marker points a restored conversation at the surviving
        // append-only external ledger.
        let marker_req = &received[marker_idx];
        let marker = marker_req
            .messages
            .iter()
            .find(|m| m.content.starts_with("[前文上下文已压缩"))
            .map(|m| m.content.clone())
            .unwrap();
        assert!(
            marker.contains(&format!("历史摘要累积于 {ledger_path_display}")),
            "marker must carry the external ledger path: {marker}"
        );
        // The external ledger exists, holds rows, and its seqs continue
        // after the compaction (fold reset does not reset the file).
        let text = std::fs::read_to_string(&ledger_file).expect("external ledger file written");
        let lines: Vec<&str> = text.lines().collect();
        assert!(!lines.is_empty(), "external ledger must hold folded rows");
        assert!(lines.iter().all(|l| l.starts_with('[')), "{text}");
    }

    #[tokio::test]
    async fn context_compact_guard_failure_retries_then_forces() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: Some(ToolResult {
                output: "x".repeat(600),
                exit_code: Some(0),
                output_encoding: None,
                structured: None,
                ..Default::default()
            }),
        };
        let tool_call = |id: &str| ScriptedResponse {
            text: None,
            tool_calls: vec![ToolCall {
                name: "read_file".to_string(),
                arguments: serde_json::json!({"target_file": "a.txt"}),
                call_id: id.to_string(),
            }],
            finish_reason: FinishReason::ToolCalls,
            reasoning_content: None,
            prompt_tokens: Some(300_000),
        };
        let fake = Arc::new(FakeProvider::new(vec![
            tool_call("call-f1"),
            tool_call("call-f2"),
            tool_call("call-f3"),
            tool_call("call-f4"),
            tool_call("call-f5"),
            // Three guard-blocked rounds → the forced compaction fires on
            // the next loop-top (round 6's gap) and reports guard_failed.
            // 机械模式（2026-08-18 B 定案）：压缩零模型调用，无摘要项。
            ScriptedResponse {
                text: Some("第六轮".to_string()),
                tool_calls: vec![ToolCall {
                    name: "read_file".to_string(),
                    arguments: serde_json::json!({"target_file": "b.txt"}),
                    call_id: "call-f6".to_string(),
                }],
                finish_reason: FinishReason::ToolCalls,
                reasoning_content: None,
                prompt_tokens: Some(50_000),
            },
            ScriptedResponse::text("候选答案"),
            ScriptedResponse::text("最终答案"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway)
            .with_context_compact(150_000, 400, 20, 100_000)
            // The guard can never be satisfied — min_compactable is huge.
            // v7（S1 修订批）：未折叠路径以驻留带 L 为界（不再按轮数切）——
            // 钉成「最新 2 轮」的估计量，使守卫路径可达（否则无可压内容）。
            .with_slider_resident_tokens(800)
            .with_summary_guards(u64::MAX, 1.0);
        controller
            .run_turn(
                &host,
                "压缩测试",
                "RUN-COMPACT-GUARD",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let compact_events: Vec<serde_json::Value> = events(&dir)
            .into_iter()
            .filter(|e| e.event_type == EventType::ContextCompressed)
            .map(|e| e.payload)
            .collect();
        assert_eq!(compact_events.len(), 1, "{compact_events:?}");
        assert_eq!(compact_events[0]["guard_failed"], true);
        assert_eq!(compact_events[0]["reason"], "fallback");
        assert_eq!(compact_events[0]["summary_incomplete"], false);
        assert!(compact_events[0]["summary_digest"].as_str().is_some());

        // The failure report reaches the model in the marker.
        let received = fake.received_requests();
        assert!(
            received.iter().any(|r| r
                .messages
                .iter()
                .any(|m| { m.content.contains("机制失败：缩减守卫连续不满足") })),
            "guard failure must be explicitly reported: {received:?}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// P0-D review fix (2026-08-14, ADR-0010 v1.14): a successful run whose
    /// FULL conversation estimate crosses the session-end gate compacts once
    /// BEFORE the terminal event and the sidecar write-back — the marker is
    /// pinned into the persisted conversation (restore 治本) and the
    /// blackboard edit window is rolled.
    #[tokio::test]
    async fn session_end_compact_pins_marker_into_sidecar_and_keeps_edits() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse {
                text: Some("候选答案".to_string()),
                tool_calls: Vec::new(),
                finish_reason: FinishReason::Stop,
                reasoning_content: None,
                prompt_tokens: Some(100),
            },
            ScriptedResponse {
                text: Some("最终答案".to_string()),
                tool_calls: Vec::new(),
                finish_reason: FinishReason::Stop,
                reasoning_content: None,
                prompt_tokens: Some(100),
            },
        ]));
        let controller = AgentLoopController::with_gateway(fake.clone())
            .with_session_end_trigger(1)
            .with_summary_guards(1, 1.0);
        // Seed a fat conversation (three big tool rounds) and two edit
        // records — the epoch-scoped window the end-of-session compaction
        // must NOT touch (v1.15: compaction decoupled from the blackboard).
        let mut conversation = vec![conv_message(Role::User, "第一问")];
        conversation.extend(tool_round("call-r1", &"A".repeat(600)));
        conversation.extend(tool_round("call-r2", &"B".repeat(600)));
        conversation.extend(tool_round("call-r3", &"C".repeat(600)));
        {
            let mut bb = controller.blackboard().write();
            bb.edits.push(EditRecord {
                file: "a.py".into(),
                old_lines: 1,
                new_lines: 2,
                timestamp: "2026-08-14T00:00:00Z".into(),
                round: 0,
                domain: None,
            });
            bb.edits.push(EditRecord {
                file: "b.rs".into(),
                old_lines: 3,
                new_lines: 4,
                timestamp: "2026-08-14T00:00:00Z".into(),
                round: 0,
                domain: None,
            });
        }
        let _ = controller
            .run_turn(
                &host,
                "继续",
                "RUN-SE",
                MANIFEST,
                0,
                None,
                None,
                Some(&mut conversation),
            )
            .await
            .unwrap();

        let compact_events: Vec<serde_json::Value> = events(&dir)
            .into_iter()
            .filter(|e| e.event_type == EventType::ContextCompressed)
            .map(|e| e.payload)
            .collect();
        assert_eq!(compact_events.len(), 1, "{compact_events:?}");
        assert_eq!(compact_events[0]["reason"], "session_end");
        assert_eq!(compact_events[0]["summary_incomplete"], false);
        assert_eq!(compact_events[0]["guard_failed"], false);
        assert_eq!(compact_events[0]["archive_write_failed"], false);

        // The marker is pinned into the persisted conversation; the old
        // rounds are gone from it.
        assert!(
            conversation
                .iter()
                .any(|m| m.content.starts_with("[前文上下文已压缩")),
            "marker must ride the sidecar: {conversation:?}"
        );
        assert!(
            conversation
                .iter()
                .all(|m| !m.content.contains(&"A".repeat(600))),
            "old rounds must be drained from the sidecar"
        );
        // The archive was written under .gsa/compaction.
        let archives: Vec<_> = std::fs::read_dir(dir.join(".gsa").join("compaction"))
            .expect("archive dir exists")
            .flatten()
            .collect();
        assert_eq!(archives.len(), 1);
        // v1.15 (2026-08-14): compaction never clears the blackboard — the
        // edit records stay live for the current plan epoch.
        assert_eq!(controller.blackboard().read().edits.len(), 2);

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// D3-1 (2026-08-14, ADR-0010 v1.10 / CONTEXT_COMPACTION_DESIGN §6): the
    /// restore write-back filter keeps the compaction marker and the
    /// whitelist block (a restored prompt must see the compression notice
    /// and the task facts) while still filtering other mechanical injected
    /// blocks.
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

    // ── P2-14 S2（2026-09-04，ADR-0010 §14.54 / 设计稿 §7 第 8 项、§9）──

    /// S2 共用前置：一次主车道长 run，同一会话串行真实触发三类压缩——
    /// r1..r3 的 measured 50K 越过 rhythm（cooldown 2，r3 后首个可压轮），
    /// r4/r5 的 measured 300K 越过 fallback（绕冷却、r4/r5 后各一），r6
    /// 回落 50K 不再触发，run 收尾（conversation 模式）越过 session_end
    /// 恒压。压缩全机械零模型调用；返回写回会话（含滚动单 v0.3 marker）
    /// 与模型网关（请求观测）。
    async fn p2_14_s2_run_serial_v03(dir: &PathBuf) -> (Vec<Message>, Arc<FakeProvider>) {
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: Some(ToolResult {
                output: "x".repeat(600),
                exit_code: Some(0),
                output_encoding: None,
                structured: None,
                ..Default::default()
            }),
        };
        let tool_call = |id: &str, prompt: u64| ScriptedResponse {
            text: None,
            tool_calls: vec![ToolCall {
                name: "read_file".to_string(),
                arguments: serde_json::json!({"target_file": "a.txt"}),
                call_id: id.to_string(),
            }],
            finish_reason: FinishReason::ToolCalls,
            reasoning_content: None,
            prompt_tokens: Some(prompt),
        };
        let fake = Arc::new(FakeProvider::new(vec![
            tool_call("call-s1", 50_000),
            tool_call("call-s2", 50_000),
            tool_call("call-s3", 50_000),
            tool_call("call-s4", 300_000),
            tool_call("call-s5", 300_000),
            tool_call("call-s6", 50_000),
            ScriptedResponse::text("候选答案").with_prompt_tokens(100),
            ScriptedResponse::text("最终答案").with_prompt_tokens(100),
        ]));
        let controller = AgentLoopController::with_gateway(fake.clone())
            .with_context_compact(5_000, 400, 2, 100_000)
            // v7（S1 修订批）：未折叠路径以驻留带 L 为界（不再按轮数切）——
            // 钉成「最新 2 轮」的估计量以复现既有串行触发链。
            .with_slider_resident_tokens(800)
            .with_summary_guards(1, 1.0)
            .with_session_end_trigger(1);
        let mut conversation = vec![conv_message(Role::User, "第一问")];
        controller
            .run_turn(
                &host,
                "压缩测试",
                "RUN-S2-SERIAL",
                MANIFEST,
                0,
                None,
                None,
                Some(&mut conversation),
            )
            .await
            .unwrap();
        (conversation, fake)
    }

    /// P2-14 S2（2026-09-04，设计稿 §9）：压缩 e2e 全串行——同一主会话
    /// 一次 run 依序真实触发 rhythm → fallback（×2，绕冷却）→ session_end，
    /// 全机械零模型调用；每次压缩后下一模型请求携带滚动单 v0.3 折叠快照
    /// marker（A–E 块、无 v0.2 五段槽），收尾写回的会话只保留一个 v0.3
    /// marker（旧 marker 被替换）。
    #[tokio::test]
    async fn p2_14_s2_rhythm_fallback_session_end_serial_v03_marker() {
        let dir = test_dir();
        let (conversation, fake) = p2_14_s2_run_serial_v03(&dir).await;

        // 触发按序 journal：rhythm（r3 后）→ fallback（r4/r5 后各一）→
        // session_end（run 收尾）。
        let reasons: Vec<String> = events(&dir)
            .into_iter()
            .filter(|e| e.event_type == EventType::ContextCompressed)
            .map(|e| e.payload["reason"].as_str().unwrap().to_string())
            .collect();
        assert_eq!(
            reasons,
            ["rhythm", "fallback", "fallback", "session_end"],
            "serial trigger chain must fire rhythm → fallback → session_end"
        );

        // 8 个模型请求（6 工具轮 + 候选 + 最终）——机械压缩零模型调用。
        let reqs = fake.received_requests();
        assert_eq!(reqs.len(), 8, "{reqs:?}");
        for (idx, req) in reqs.iter().enumerate() {
            let markers: Vec<&str> = req
                .messages
                .iter()
                .filter(|m| {
                    m.content
                        .starts_with(crate::prompt::CONTEXT_COMPRESSED_PREFIX)
                })
                .map(|m| m.content.as_str())
                .collect();
            if idx < 3 {
                assert!(
                    markers.is_empty(),
                    "no marker before the first rhythm gap (request {idx}): {req:?}"
                );
            } else {
                assert_eq!(
                    markers.len(),
                    1,
                    "rolling single marker per post-compaction request {idx}: {req:?}"
                );
                assert!(
                    markers[0].starts_with("[前文上下文已压缩 v0.3]"),
                    "main-lane compaction must keep v0.3 through the serial chain: {}",
                    markers[0]
                );
                assert!(
                    !markers[0].contains("目的: ") && !markers[0].contains("后续衔接: "),
                    "v0.2 五段槽不得出现在串行链 v0.3 marker: {}",
                    markers[0]
                );
            }
        }

        // 收尾会话 = 滚动单 v0.3 marker + 保留尾；A–E 块齐全。
        let markers: Vec<&Message> = conversation
            .iter()
            .filter(|m| {
                m.content
                    .starts_with(crate::prompt::CONTEXT_COMPRESSED_PREFIX)
            })
            .collect();
        assert_eq!(
            markers.len(),
            1,
            "session-end write-back must carry exactly one rolling marker: {conversation:?}"
        );
        let marker = markers[0].content.as_str();
        assert!(marker.starts_with("[前文上下文已压缩 v0.3]"), "{marker}");
        for head in [
            "保留尾首轮 r_keep=",
            "== 近窗明细（round < r_keep，已排除保留尾） ==",
            "== 旧段聚合",
            "== 失败目标聚合 ==",
            "== 查询指针 ==",
            "[/前文上下文已压缩]",
        ] {
            assert!(marker.contains(head), "missing {head}: {marker}");
        }
        assert!(
            !marker.contains("目的: ") && !marker.contains("后续衔接: "),
            "v0.2 五段槽不得出现在收尾 v0.3 marker: {marker}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// P2-14 S2（2026-09-04，设计稿 §7 矩阵第 8 项）：恢复预检后压缩 marker
    /// 仍在、内容原样——前一 run 收尾写回的 v0.3 会话在下次 run 越过恢复
    /// 触发（测试化 10）时整轮机械截断；旧 v0.3 marker 消息位于 preamble
    /// 恒原样保留（逐字节相等，非仅前缀），截断 marker 追加其后，二者均经
    /// D3-1 write-back 存活。
    #[tokio::test]
    async fn p2_14_s2_restore_preflight_keeps_v03_marker_verbatim() {
        let dir1 = test_dir();
        let (conversation1, _) = p2_14_s2_run_serial_v03(&dir1).await;
        let marker_v03 = conversation1
            .iter()
            .find(|m| m.content.starts_with("[前文上下文已压缩 v0.3]"))
            .map(|m| m.content.clone())
            .expect("run1 must leave a v0.3 marker in the sidecar");
        assert_eq!(
            conversation1
                .iter()
                .filter(|m| m.content == marker_v03)
                .count(),
            1
        );

        let dir2 = test_dir();
        let journal2 = JournalRecorder::new(dir2.clone());
        let host2 = TestHost {
            journal: journal2,
            tool_result: None,
        };
        let fake2 = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::text("收到"),
            ScriptedResponse::text("收到"),
        ]));
        // 恢复预检触发/目标测试化：10 tokens 触发 → 只保留最新整轮（丢 ≥1
        // 轮）；session_end/rhythm 保持生产默认（截断后估计远低于门槛）。
        let controller2 =
            AgentLoopController::with_gateway(fake2.clone()).with_recovery_compact(10, 2);
        let mut conversation2 = conversation1;
        controller2
            .run_turn(
                &host2,
                "第二问",
                "RUN-S2-RESTORE",
                MANIFEST,
                0,
                None,
                None,
                Some(&mut conversation2),
            )
            .await
            .unwrap();

        // 截断确实发生（整轮 drop）且旧 marker 逐字节保留在首个模型请求。
        let evs2 = events(&dir2);
        let trunc: Vec<&RunEvent> = evs2
            .iter()
            .filter(|e| e.event_type == EventType::ContextRecoveryTruncated)
            .collect();
        assert_eq!(trunc.len(), 1, "{trunc:?}");
        assert!(
            trunc[0].payload["rounds_dropped"].as_u64().unwrap() >= 1,
            "{:?}",
            trunc[0].payload
        );
        let reqs = fake2.received_requests();
        assert_eq!(reqs.len(), 2, "{reqs:?}");
        let first = &reqs[0].messages;
        assert!(
            first.iter().any(|m| m.content == marker_v03),
            "v0.3 marker must reach the restored first request byte-identical: {first:?}"
        );
        assert!(
            first
                .iter()
                .any(|m| m.content.starts_with("[前文上下文已压缩 v0.1-恢复]")),
            "recovery truncation marker must follow: {first:?}"
        );
        let v03_pos = first.iter().position(|m| m.content == marker_v03).unwrap();
        let recovery_pos = first
            .iter()
            .position(|m| m.content.starts_with("[前文上下文已压缩 v0.1-恢复]"))
            .unwrap();
        assert!(
            v03_pos < recovery_pos,
            "old marker stays in the preamble before the truncation marker: {first:?}"
        );

        // D3-1 write-back 后会话仍只有一个 v0.3 marker，内容原样。
        assert_eq!(
            conversation2
                .iter()
                .filter(|m| m.content == marker_v03)
                .count(),
            1,
            "v0.3 marker must survive the restore write-back verbatim: {conversation2:?}"
        );
        assert_eq!(
            conversation2
                .iter()
                .filter(|m| m
                    .content
                    .starts_with(crate::prompt::CONTEXT_COMPRESSED_PREFIX))
                .count(),
            2,
            "sidecar must hold the retained v0.3 marker + the recovery marker: {conversation2:?}"
        );

        let _ = std::fs::remove_dir_all(&dir1);
        let _ = std::fs::remove_dir_all(&dir2);
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
