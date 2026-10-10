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
    /// `ORZ_MODEL_FACE_GUARD_TOKENS`，默认 700K）。越线 ⇒ 强制截断到线上。
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
        let entries = self.whitelist.lock().unwrap_or_else(|e| e.into_inner());
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

    /// 0cz S2（2026-10-11，设计 §5/§9）：`context_manage mode=clear` 的执行核
    /// （上下文管理第十一主工具的清零面）。顺序＝边界计算 → 防裸清校验 →
    /// 交接写入黑板（fail-closed 前置）→ 回放落盘（块级＋现场整段，best-effort
    /// 如实标注）→ 边界 marker 插入（保留窗起点；本地面**只插入不覆盖**，
    /// 不变量 I3；I1 显式例外＝模型主动＋交接前置＋边界可回放）→ 软水位键
    /// 重置。`Err`＝中性说明/拒绝（模型面文案，调用方原样回传、exit 0/1）。
    ///
    /// 0da S2（设计 §5/§6.3）：
    /// - **D2 归一**：marker 插入点＝保留窗第一轮起点 `ranges[cur-keep].0`、
    ///   边界参数 `cur-keep`（marker 文本 r ＝ boundary_round1）——物理隐藏
    ///   `[0, marker_idx)` ＝ 事件/信封边界 ＝ marker 文本 ＝ clear.md 覆盖
    ///   四方一致（239 批审查 D2：旧插入点恒当前轮起点使 keep>0 三方分裂）。
    /// - **D5 单调守卫**：既有清零边界 ≥ 新边界 ⇒ 中性 exit 0（新 marker 落
    ///   已隐藏区会被静默埋掉而信封谎报成功）；dump 资格补「旧边界已覆盖 ⇒
    ///   跳过」（档案零重写）。
    /// - **`clear_board`（mode=clear_all，设计 §6.3）**：先清黑板三分区
    ///   （journal 留痕）**后**写 handover（成为板面唯一条目——锚点保序），
    ///   再执行清零核。
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn apply_context_clear(
        &self,
        messages: &mut Vec<Message>,
        host: &dyn LoopHost,
        run_id: &str,
        handover: Option<&str>,
        keep_recent_rounds: u32,
        clear_board: bool,
    ) -> Result<ContextClearOutcome, (i64, String)> {
        use crate::model_face::BlockState;
        let ranges = crate::action_ledger::round_ranges(messages);
        // 边界＝当前轮（最后一个声明区间；本次清零调用就在其中，0 基 C）。
        if ranges.is_empty() {
            return Err((
                0,
                "无可清零内容：当前会话还没有已完成的工具轮次。".to_string(),
            ));
        }
        let cur = ranges.len() - 1; // 0 基当前轮号
        // 保留当前轮 ＋ keep 个前置完整轮 ⇒ 隐藏 0..=(cur-1-keep)。
        let Some(&(_, hidden_end)) = cur
            .checked_sub(1 + keep_recent_rounds as usize)
            .and_then(|i| ranges.get(i))
        else {
            return Err((
                0,
                format!(
                    "无可清零内容：保留轮数（keep_recent_rounds={keep_recent_rounds}）已覆盖全部历史。"
                ),
            ));
        };
        if hidden_end == 0 {
            return Err((
                0,
                "无可清零内容：当前轮之前没有需要移出的历史。".to_string(),
            ));
        }
        // 1 基边界轮号（marker 的「清零边界」行同值；＝保留窗第一轮）。
        let boundary_round1 = cur + 1 - keep_recent_rounds as usize;
        // D5 单调守卫（0da S2 设计 §5.2）：既有清零边界 ≥ 新边界 ⇒ 本轮无可清
        // （新 marker 会落已隐藏区被埋掉）。中性 exit 0，非护栏拒绝。
        // 242 批处置（P3）：守卫先于 handover 校验——「无可清」比「缺交接」
        // 更基本：二次清零重试即便漏带 handover 也应得到中性说明而非护栏拒绝。
        let markers_before = crate::model_face::face_markers(messages);
        if markers_before
            .cleared_before_round
            .is_some_and(|existing| (boundary_round1 as u64) <= existing as u64)
        {
            return Err((
                0,
                format!(
                    "无可清零内容：现有清零边界（第 {} 轮）已覆盖目标区间（第 {boundary_round1} 轮）。",
                    markers_before.cleared_before_round.unwrap()
                ),
            ));
        }
        // 防裸清护栏（用户裁决⑥；设计 §9）：handover 必填非空 ≤8K。
        let Some(handover) = handover.map(str::trim).filter(|s| !s.is_empty()) else {
            return Err((
                1,
                "清零被拒绝（防裸清护栏）：mode=clear 需要 `handover` 交接摘要（纯文本 ≤8K；\
                 目标／已完成／关键决策／未决问题／下一步）。请把工作现场固化到 handover 与黑板后再清零。"
                    .to_string(),
            ));
        };
        let handover_chars = handover.chars().count();
        if handover_chars > crate::blackboard::MODEL_NOTE_MAX_CHARS {
            return Err((
                1,
                format!(
                    "清零被拒绝：handover 超出单次上限（{handover_chars} > {} 字符）。请精炼后重试。",
                    crate::blackboard::MODEL_NOTE_MAX_CHARS
                ),
            ));
        }
        // 信封读数的近真参数（D4/静态开销不含——读数为内容近似；阶梯量尺
        // 在 loop 侧按真实 face_params 计，不受影响）。
        let envelope_params = crate::model_face::ModelFaceParams {
            slider_tokens: self.context_compact.slider_window_tokens,
            block_tokens: self.context_compact.model_face_block_tokens,
            ledger_path: Some(crate::action_ledger::ledger_file_path(&host.session_cwd())),
            archive_tag: Some(crate::model_face::archive_tag(
                self.session_id.as_deref(),
                run_id,
            )),
            run_id: run_id.to_string(),
            d4_block: None,
            static_overhead_tokens: 0,
        };
        let before = crate::model_face::estimate_model_face_tokens(messages, &envelope_params);
        // ⓪（mode=clear_all，设计 §6.3）：黑板三分区清空（journal 留痕面在
        // ContextCompressed 载荷；此处先清板、后写 handover ⇒ 板面最终恰一条
        // 【交接摘要】——锚点保序）。
        let board_entries_removed = if clear_board {
            self.blackboard.write().clear_model_note_sections(&[
                crate::blackboard::ModelNoteSection::Plan,
                crate::blackboard::ModelNoteSection::Notes,
                crate::blackboard::ModelNoteSection::Findings,
            ])
        } else {
            0
        };
        // ① 交接写入黑板（notes 分区【交接摘要】条目；(round, domain) 盖章）。
        let (round, domain) = self.effective_blackboard_stamp();
        self.blackboard.write().push_model_note(
            crate::blackboard::ModelNoteSection::Notes,
            crate::blackboard::NoteEntry {
                round,
                domain: Some(domain),
                timestamp: chrono_utc_now(),
                content: handover.to_string(),
                op: Some("交接摘要".to_string()),
            },
        );
        let handover_note = format!(
            "黑板 notes r{}@{}（【交接摘要】，{} 字符）",
            round,
            domain.as_str(),
            handover_chars
        );
        // ② 回放落盘（best-effort；失败如实标注，不阻塞清零——同 T1 截断口径）。
        let tag = crate::model_face::archive_tag(self.session_id.as_deref(), run_id);
        let session = self.session_id.as_deref();
        let blocks = crate::model_face::blocks_outside_slider(
            messages,
            self.context_compact.slider_window_tokens,
            self.context_compact.model_face_block_tokens,
        );
        let mut numbers: Vec<u32> = Vec::new();
        let mut replay: Vec<String> = Vec::new();
        for b in &blocks {
            // 只补dump「已闭合 ∧ Live ∧ 整块在边界前 ∧ 旧边界未覆盖」的块——
            // 已压缩/已截断块的按块档案在原动作时已落盘、由原 marker 指引；
            // 旧清零边界已覆盖的块已随上一清零落盘（D5：档案零重写）。
            if b.closed
                && markers_before.state(b.number) == BlockState::Live
                && (b.last_round as u64) + 1 < boundary_round1 as u64
                && !markers_before.is_round_cleared(b.first_round)
            {
                let path =
                    crate::model_face::block_archive_path(&host.session_cwd(), &tag, b.number);
                let markdown =
                    crate::model_face::block_archive_markdown(b, messages, run_id, session);
                let dir = path.parent().map(|p| p.to_path_buf()).unwrap_or_default();
                if !crate::summary::write_archive_retry(dir.as_path(), &path, &markdown) {
                    tracing::warn!(path = %path.display(), "context clear: block archive write failed (best-effort)");
                }
                numbers.push(b.number);
                replay.push(crate::model_face::replay_line(
                    b,
                    &crate::model_face::relative_block_archive(&tag, b.number),
                ));
            }
        }
        // 现场整段留档（边界前 `[0, hidden_end)` 逐字——含主滑块面，块级
        // 档案不覆盖的部分）。0da S2（D2 归一）：档头轮次标签同步改传
        // `cur-keep`（＝boundary_round1-1，覆盖止于保留窗前——0cz 传 `cur`
        // 在 keep>0 时档头虚大；keep=0 恰好重合故未暴露）。
        let clear_path = crate::model_face::clear_archive_path(&host.session_cwd(), &tag);
        let clear_markdown = crate::model_face::clear_transcript_markdown(
            messages,
            hidden_end,
            cur - keep_recent_rounds as usize,
            run_id,
            session,
        );
        let clear_dir = clear_path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_default();
        let archive_write_failed =
            !crate::summary::write_archive_retry(clear_dir.as_path(), &clear_path, &clear_markdown);
        // ③ 软水位键重置（设计 §5.3：软档＋first_block 复位；硬档沿越线
        // 重武装不重置。生效时点＝下一 run——ContextScaleState 每 run 自
        // controller 键集重建，当前 run 内已驻留 fired 集不受影响，如实登记）。
        self.reset_context_scale_soft_keys();
        // ④ 边界 marker 插入（**保留窗第一轮起点** `ranges[cur-keep].0`＝
        // hidden_end——0da S2 D2 归一：marker 下标即物理隐藏终点，marker
        // 文本边界＝事件/信封边界；**只插入**——I3 本地面零覆盖）。
        let rounds_cleared = hidden_end_size(&ranges, hidden_end);
        let insert_at = ranges
            .get(cur - keep_recent_rounds as usize)
            .map(|&(s, _)| s)
            .unwrap_or(hidden_end);
        let marker = crate::model_face::clear_marker(
            cur - keep_recent_rounds as usize,
            &numbers,
            rounds_cleared,
            &clear_path,
            session,
            &replay.join("\n"),
            &handover_note,
        );
        messages.insert(
            insert_at,
            Message {
                role: Role::User,
                content: marker,
                tool_call_id: None,
                tool_calls: Vec::new(),
                reasoning_content: None,
                round: None,
            },
        );
        let after = crate::model_face::estimate_model_face_tokens(messages, &envelope_params);
        let freed = before.saturating_sub(after);
        let messages_hidden = messages[..hidden_end]
            .iter()
            .filter(|m| !m.content.starts_with(crate::prompt::WHITELIST_PREFIX))
            .count();
        let failure_note = if archive_write_failed {
            "\n⚠ 回放档案写入失败（.gsa/compaction/blocks/ 落盘未成功）——被清内容的逐字原文\
             仍在本会话档案（sidecar）与 run journal 中，可按 journal 回读。"
        } else {
            ""
        };
        let board_note = if clear_board {
            format!(
                "黑板已一键清空（plan/notes/findings 共移出 {board_entries_removed} 条，\
                 全量留档于 journal）。"
            )
        } else {
            String::new()
        };
        let numbers_text = if numbers.is_empty() {
            "（无分块）".to_string()
        } else {
            crate::model_face::render_block_numbers(&numbers)
        };
        let envelope = format!(
            "清零已生效：边界＝第 {boundary_round1} 轮；已移出 {numbers_text}\
             （共 {rounds_cleared} 轮 ≈{freed}tk token，模型面估算）。\
             {board_note}\
             交接摘要已写入黑板（notes）。\
             清零存档：{}（边界前全部轮次逐字留档；read_file offset/limit 分页）。\
             用 blackboard_read 回读工作计划与结论。\
             清零后当前读数 ≈{after}（内容估算，未含常驻框架段与静态开销）。{failure_note}",
            crate::model_face::relative_clear_archive(&tag),
        );
        Ok(ContextClearOutcome {
            envelope,
            mode_label: if clear_board { "clear_all" } else { "clear" }.to_string(),
            boundary_round1,
            blocks_cleared: numbers,
            rounds_cleared,
            freed_tokens: freed,
            archive_write_failed,
            handover_note,
            messages_hidden,
            board_entries_removed,
            before_tokens: before,
            after_tokens: after,
        })
    }
}

/// 0cz S2：边界前轮数（隐藏区间覆盖的完整轮数；按区间端点计数）。
fn hidden_end_size(ranges: &[(usize, usize)], hidden_end: usize) -> usize {
    ranges.iter().take_while(|&&(_, e)| e <= hidden_end).count()
}

/// 0cz S2（设计 §5/§9）：`apply_context_clear` 的成功回执（信封文案＋事件
/// 载荷事实；调用方落 journal 与工具响应）。
pub(crate) struct ContextClearOutcome {
    pub envelope: String,
    /// 事件 `mode` 标签（"clear"｜"clear_all"；0da S2）。
    pub mode_label: String,
    /// 1 基边界轮号（marker 的「清零边界」行同值）。
    pub boundary_round1: usize,
    pub blocks_cleared: Vec<u32>,
    pub rounds_cleared: usize,
    pub freed_tokens: u64,
    pub archive_write_failed: bool,
    pub handover_note: String,
    pub messages_hidden: usize,
    /// mode=clear_all：黑板三分区移出条目总数（非 clear_all 恒 0；0da S2）。
    pub board_entries_removed: usize,
    pub before_tokens: u64,
    pub after_tokens: u64,
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
        let w = controller
            .whitelist
            .lock()
            .unwrap_or_else(|e| e.into_inner());
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
        let w = controller
            .whitelist
            .lock()
            .unwrap_or_else(|e| e.into_inner());
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
        let w = controller
            .whitelist
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        assert!(w.is_empty(), "sealed whitelist write must not land: {w:?}");
        drop(w);

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 0am FR4（2026-09-20 用户令）：压缩白名单并入 `context_compress`——
    /// 可选 `whitelist` 条目在**处理压缩的同一调用**里落盘：内存列表 +
    /// `{journal_dir}/whitelist.jsonl` best-effort 存档 + 常驻前言区
    /// `[压缩白名单` 消息（跨压缩保留、机械压缩跳过）；空条目/超累计上限
    /// 条目跳过并在响应中如实说明；全链 fail-soft（exit 0 信封、三态不变）。
    /// 0cz S2（2026-10-11，方案 A）：`context_manage` 吸收压缩通道——本测
    /// 随批改走 `context_manage`（mode=compress），管线/白名单机制零改动。
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
                name: "context_manage".to_string(),
                arguments: serde_json::json!({
                    "mode": "compress",
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
                e.event_type == EventType::ToolCompleted && e.payload["tool"] == "context_manage"
            })
            .collect();
        assert_eq!(cc.len(), 1, "{cc:?}");
        assert_eq!(cc[0].payload["exit_code"], 0);
        // ② 内存白名单 = 仅首条（空/超限条目未落地）。
        let w = controller
            .whitelist
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
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

    /// 0cz S2（2026-10-11，设计 §5/§9）六钉之①②⑥：`context_manage
    /// mode=clear` 端到端——
    /// ① **防裸清负例**：无 handover ⇒ exit 1 拒绝、零 ContextCompressed、
    ///    零 marker、黑板零写入；
    /// ② **本地面零 diff（I3）**：清零仅插入 marker——会话消息数组逐字
    ///    保留全部原消息（不 drain、不覆盖）；
    /// ⑥ **字节单调（.gsa 落盘在案）**：清零现场存档 ＋ 按块回放档案落盘
    ///    `​.gsa/compaction/blocks/`，marker 内带回放指针（追加式，无重写）。
    #[tokio::test]
    async fn context_manage_clear_requires_handover_and_lands_boundary_marker() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let fat = "F".repeat(3_000);
        let host = TestHost {
            journal,
            tool_result: Some(ToolResult {
                output: fat.clone(),
                exit_code: Some(0),
                ..Default::default()
            }),
        };
        let clear_call = |call_id: &str, args: serde_json::Value| ToolCall {
            name: "context_manage".to_string(),
            arguments: args,
            call_id: call_id.to_string(),
        };
        let fake = Arc::new(FakeProvider::new(vec![
            // 第 1 轮：正常工作（肥胖结果在 slider=1K/block=1K 下形成闭合块）。
            ScriptedResponse::tool_calls(vec![tool_call_read("call-r1")]),
            // 第 2 轮：clear 无 handover ⇒ 防裸清拒绝（exit 1）。
            ScriptedResponse::tool_calls(vec![clear_call(
                "call-c1",
                serde_json::json!({ "mode": "clear" }),
            )]),
            // 第 3 轮：clear 带 handover ⇒ 生效（边界 marker ＋ 落盘 ＋ 重置）。
            ScriptedResponse::tool_calls(vec![clear_call(
                "call-c2",
                serde_json::json!({
                    "mode": "clear",
                    "handover": "目标: 完成接线\n已完成: 台账接线\n关键决策: 分块压缩\n未决问题: 无\n下一步: 从台账继续",
                }),
            )]),
            // 终答候选轮（无工具调用）⇒ 反例门一次性触发 ⇒ 再烧一轮。
            ScriptedResponse::text("终答"),
            ScriptedResponse::text("终答确认"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway)
            .with_slider_window_tokens(1_000)
            .with_model_face_block_tokens(1_000);
        let mut conversation: Vec<Message> = Vec::new();
        controller
            .run_turn(
                &host,
                "0cz 清零端到端",
                "RUN-CZ-CLEAR",
                MANIFEST,
                0,
                None,
                None,
                Some(&mut conversation),
            )
            .await
            .unwrap();

        let evs = events(&dir);
        // ① 防裸清负例：恰一次 exit 1 ToolCompleted（mode=clear），文案带
        // 护栏名；该轮**无** ContextCompressed（唯一一条清零事件来自第 3 轮）。
        let rejected: Vec<_> = evs
            .iter()
            .filter(|e| {
                e.event_type == EventType::ToolCompleted
                    && e.payload["mode"] == "clear"
                    && e.payload["exit_code"] == 1
            })
            .collect();
        assert_eq!(rejected.len(), 1, "{rejected:?}");
        assert!(
            rejected[0].payload["error"]
                .as_str()
                .unwrap()
                .contains("防裸清护栏")
        );
        let cleared: Vec<_> = evs
            .iter()
            .filter(|e| {
                e.event_type == EventType::ContextCompressed && e.payload["mode"] == "clear"
            })
            .collect();
        assert_eq!(cleared.len(), 1, "{cleared:?}");
        assert_eq!(cleared[0].payload["boundary_round"], 3);
        assert_eq!(cleared[0].payload["rounds_dropped"], 2);
        // 成功轮 ToolCompleted exit 0。
        assert!(evs.iter().any(|e| e.event_type == EventType::ToolCompleted
            && e.payload["mode"] == "clear"
            && e.payload["exit_code"] == 0));
        // ② I3 零 diff：会话消息数组＝原消息全量逐字 ＋ 恰一枚清零 marker
        // （插入不覆盖——原消息无一丢失、无一改写）。
        let marker_msgs: Vec<_> = conversation
            .iter()
            .filter(|m| {
                m.content
                    .starts_with(crate::model_face::CLEAR_MARKER_PREFIX)
            })
            .collect();
        assert_eq!(marker_msgs.len(), 1, "恰一枚清零 marker");
        assert!(marker_msgs[0].content.contains("清零边界: 第 3 轮]"));
        assert!(marker_msgs[0].content.contains("交接摘要"));
        // 原消息保真：肥胖工具结果与两枚工具声明逐字仍在。
        assert!(
            conversation
                .iter()
                .any(|m| m.role == crate::gateway::model::Role::Tool && m.content == fat),
            "被清轮次的本地面原文必须逐字保留（移出视野≠销毁）"
        );
        // ⑥ .gsa 落盘在案：清零现场存档 ＋ 按块回放档案（tag 取 run id 尾）。
        let blocks_dir = dir.join(".gsa").join("compaction").join("blocks");
        let mut clear_archives = Vec::new();
        let mut block_archives = Vec::new();
        for entry in std::fs::read_dir(&blocks_dir).expect("blocks dir") {
            let name = entry.unwrap().file_name().to_string_lossy().to_string();
            if name.ends_with("-clear.md") {
                clear_archives.push(name);
            } else if name.contains("-block-") {
                block_archives.push(name);
            }
        }
        assert_eq!(clear_archives.len(), 1, "{clear_archives:?}");
        assert!(!block_archives.is_empty(), "{block_archives:?}");
        let text =
            std::fs::read_to_string(blocks_dir.join(&clear_archives[0])).expect("clear archive");
        assert!(text.contains(&fat), "现场存档必须逐字含被清内容");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 0da S2（2026-10-11，设计 §5.3）D2/D5 钉：`keep_recent_rounds` 归一＋
    /// 二次清零单调守卫——
    /// ① keep=2 清零：物理隐藏/事件边界/marker 文本/clear.md 覆盖**四方归一**
    ///    （保留窗两轮在面逐字；被清轮不在面、恰在 clear.md）；
    /// ② keep 过大致边界倒退的二次清零 ⇒ **中性 exit 0**（现有清零边界已覆盖；
    ///    零新 marker、零事件——修复前：新 marker 被静默埋掉而信封谎报成功）；
    /// ③ 正常二次清零（边界前进）⇒ 生效，且只 dump 新增区间块。
    #[tokio::test]
    async fn context_clear_keep_recent_rounds_and_monotonic_second_clear() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let fat = |tag: char| "F".repeat(3_000) + &tag.to_string();
        let host = TestHost {
            journal,
            tool_result: Some(ToolResult {
                output: fat('a'),
                exit_code: Some(0),
                ..Default::default()
            }),
        };
        let clear_call = |call_id: &str, keep: u64| ToolCall {
            name: "context_manage".to_string(),
            arguments: serde_json::json!({
                "mode": "clear",
                "handover": "目标: 完成接线\n已完成: 台账接线\n下一步: 继续",
                "keep_recent_rounds": keep,
            }),
            call_id: call_id.to_string(),
        };
        let read_call = |call_id: &str| ToolCall {
            name: "read_file".to_string(),
            arguments: serde_json::json!({ "path": "src/f.rs" }),
            call_id: call_id.to_string(),
        };
        let fake = Arc::new(FakeProvider::new(vec![
            // 轮 1–3：正常工作（三次读取形成历史轮）。
            ScriptedResponse::tool_calls(vec![read_call("call-r1")]),
            ScriptedResponse::tool_calls(vec![read_call("call-r2")]),
            ScriptedResponse::tool_calls(vec![read_call("call-r3")]),
            // 轮 4：clear keep=2 ⇒ 边界＝第 2 轮（轮 1 清、轮 2–3 保留）。
            ScriptedResponse::tool_calls(vec![clear_call("call-c1", 2)]),
            // 轮 5：clear keep=2 ⇒ cur=4, 边界 5-2=3 > 既有边界 2 ⇒ 正常二次
            // 清零生效（边界前进到第 3 轮）。
            ScriptedResponse::tool_calls(vec![clear_call("call-c2", 2)]),
            // 轮 6：clear keep=3 ⇒ cur=5, 边界 6-3=3 ≤ 既有边界 3 ⇒ 单调守卫
            // 中性拒绝（修复前：静默埋 marker＋谎报成功）。
            ScriptedResponse::tool_calls(vec![clear_call("call-c3", 3)]),
            // 终答候选轮 ×2（反例门一次性触发后再烧一轮）。
            ScriptedResponse::text("终答"),
            ScriptedResponse::text("终答确认"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway)
            .with_slider_window_tokens(1_000)
            .with_model_face_block_tokens(1_000);
        let mut conversation: Vec<Message> = Vec::new();
        controller
            .run_turn(
                &host,
                "0da keep/单调守卫端到端",
                "RUN-CZ-KEEP",
                MANIFEST,
                0,
                None,
                None,
                Some(&mut conversation),
            )
            .await
            .unwrap();

        let evs = events(&dir);
        // ① keep=2：事件边界＝2、marker 文本同界、轮 2–3 声明在面、轮 1 结果在
        // clear.md。
        let first = &evs
            .iter()
            .filter(|e| {
                e.event_type == EventType::ContextCompressed
                    && e.payload["mode"] == "clear"
                    && e.payload["boundary_round"] == 2
            })
            .count();
        assert_eq!(*first, 1, "第一次清零边界＝2（keep=2）");
        let marker_msgs: Vec<_> = conversation
            .iter()
            .filter(|m| {
                m.content
                    .starts_with(crate::model_face::CLEAR_MARKER_PREFIX)
            })
            .collect();
        assert_eq!(
            marker_msgs.len(),
            2,
            "恰两枚清零 marker（keep 清零＋二次清零）"
        );
        assert!(
            marker_msgs[0].content.contains("清零边界: 第 2 轮]"),
            "marker 文本边界＝事件边界（D2 归一）：{}",
            &marker_msgs[0].content[..120]
        );
        // 物理面（投影装配，非原始数组——I3 原始消息全量保留）。D2 归一的
        // 机械语义：marker 恰插在**保留窗第一轮声明起点**——marker1＝轮 2
        // 声明起点（keep=2）、marker2＝轮 3 声明起点（二次清零 keep=2）。
        let face_params = crate::model_face::ModelFaceParams {
            slider_tokens: 1_000,
            block_tokens: 1_000,
            ledger_path: None,
            archive_tag: None,
            run_id: "RUN-CZ-KEEP".to_string(),
            d4_block: None,
            static_overhead_tokens: 0,
        };
        let face_in = |id: &str, convo: &[Message]| {
            crate::model_face::build_model_face(convo, &face_params)
                .unwrap()
                .iter()
                .any(|m| {
                    m.role == Role::Assistant && m.tool_calls.iter().any(|tc| tc.call_id == id)
                })
        };
        let decl_idx = |id: &str| {
            conversation
                .iter()
                .position(|m| {
                    m.role == Role::Assistant && m.tool_calls.iter().any(|tc| tc.call_id == id)
                })
                .expect("declaration present (I3)")
        };
        let m1 = conversation
            .iter()
            .position(|m| {
                m.content
                    .starts_with(crate::model_face::CLEAR_MARKER_PREFIX)
            })
            .unwrap();
        let m2 = conversation
            .iter()
            .rposition(|m| {
                m.content
                    .starts_with(crate::model_face::CLEAR_MARKER_PREFIX)
            })
            .unwrap();
        assert_eq!(
            m1 + 1,
            decl_idx("call-r2"),
            "marker1 紧邻保留窗第一轮（轮 2）声明之前＝物理隐藏终点 [0, marker)（keep=2 归一）"
        );
        assert_eq!(
            m2 + 1,
            decl_idx("call-r3"),
            "marker2 紧邻二次清零保留窗第一轮（轮 3）声明之前"
        );
        // clear.md 覆盖＝被清区间（第二次清零覆盖写同档：边界 3 ⇒ 档头「轮 1–2」
        // ——保留窗轮 3 不得入档，D2 归一）。
        let blocks_dir = dir.join(".gsa").join("compaction").join("blocks");
        let mut clear_archives = Vec::new();
        for entry in std::fs::read_dir(&blocks_dir).expect("blocks dir") {
            let name = entry.unwrap().file_name().to_string_lossy().to_string();
            if name.ends_with("-clear.md") {
                clear_archives.push(name);
            }
        }
        clear_archives.sort();
        let t1 = std::fs::read_to_string(blocks_dir.join(&clear_archives[0])).unwrap();
        assert!(
            t1.contains("清零现场留档（轮 1–2）"),
            "clear.md 覆盖恰为被清区间轮 1–2（保留窗轮 3 不入档）：{}",
            &t1[..200]
        );
        assert!(t1.contains(&fat('a')), "被清轮内容逐字在档");
        // ② 二次清零（keep=2，边界 3）：生效且 marker 文本同界；终态面＝
        // r1/r2 移出、r3 与当前轮在面。
        assert!(
            evs.iter().any(|e| {
                e.event_type == EventType::ContextCompressed
                    && e.payload["mode"] == "clear"
                    && e.payload["boundary_round"] == 3
            }),
            "正常二次清零生效（边界前进到 3）"
        );
        assert!(
            !face_in("call-r1", &conversation) && !face_in("call-r2", &conversation),
            "终态面：轮 1–2（边界 3 之前）声明移出面"
        );
        assert!(
            face_in("call-r3", &conversation),
            "轮 3（新边界轮）保持在面"
        );
        assert!(marker_msgs[1].content.contains("清零边界: 第 3 轮]"));
        // ③ 单调守卫：keep=3 ⇒ 中性拒绝——回执带「已覆盖目标区间」（中性
        // exit 0 不入事件 error 字段＝0cz 既有口径，断言打在工具回执上）＋
        // 零第三 marker＋零边界事件（修复前：静默埋 marker＋谎报成功）。
        let guard_reply = conversation
            .iter()
            .find(|m| {
                m.role == Role::Tool
                    && m.tool_call_id.as_deref() == Some("call-c3")
                    && m.content.contains("已覆盖目标区间")
            })
            .expect("单调守卫须中性说明一次（工具回执）");
        assert!(
            guard_reply.content.contains("现有清零边界（第 3 轮）"),
            "守卫回执须点名既有边界：{}",
            guard_reply.content
        );
        assert_eq!(
            marker_msgs.len(),
            2,
            "守卫拒绝后不得新增 marker（修复前会被静默埋掉）"
        );
        assert!(
            !evs.iter().any(|e| {
                e.event_type == EventType::ContextCompressed && e.payload["boundary_round"] == 1
            }),
            "守卫拒绝不得产生边界事件"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 242 批处置钉（P3）：D5 单调守卫**先于** handover 校验——已覆盖边界的
    /// 二次清零即便漏带 handover，也得到中性「已覆盖目标区间」说明（exit 0），
    /// 而非防裸清护栏拒绝（exit 1）；零新 marker、零新边界事件（零副作用
    /// 不变量不变，仅语义次序：无可清比缺交接更基本）。
    #[tokio::test]
    async fn monotonic_guard_precedes_handover_validation() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: Some(ToolResult {
                output: "F".repeat(3_000),
                exit_code: Some(0),
                ..Default::default()
            }),
        };
        let clear_call = |call_id: &str, args: serde_json::Value| ToolCall {
            name: "context_manage".to_string(),
            arguments: args,
            call_id: call_id.to_string(),
        };
        let fake = Arc::new(FakeProvider::new(vec![
            // 轮 1–2：正常工作。
            ScriptedResponse::tool_calls(vec![tool_call_read("call-r1")]),
            ScriptedResponse::tool_calls(vec![tool_call_read("call-r2")]),
            // 轮 3：clear（带 handover）⇒ 生效，边界＝第 3 轮（keep=0 仅留当前轮）。
            ScriptedResponse::tool_calls(vec![clear_call(
                "call-c1",
                serde_json::json!({ "mode": "clear", "handover": "目标: x\n下一步: y" }),
            )]),
            // 轮 4：keep=1 ⇒ 边界 4-1=3 ≤ 既有边界 3 ⇒ 守卫先命中；且本调用
            // **漏带 handover**（修复前次序：先吃防裸清 exit 1）。
            ScriptedResponse::tool_calls(vec![clear_call(
                "call-c2",
                serde_json::json!({ "mode": "clear", "keep_recent_rounds": 1 }),
            )]),
            // 终答候选轮 ×2。
            ScriptedResponse::text("终答"),
            ScriptedResponse::text("终答确认"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway)
            .with_slider_window_tokens(1_000)
            .with_model_face_block_tokens(1_000);
        let mut conversation: Vec<Message> = Vec::new();
        controller
            .run_turn(
                &host,
                "0da 守卫次序端到端",
                "RUN-0DA-GUARD",
                MANIFEST,
                0,
                None,
                None,
                Some(&mut conversation),
            )
            .await
            .unwrap();

        // 中性说明打在工具回执上：含「已覆盖目标区间」、不含护栏拒绝文案。
        let reply = conversation
            .iter()
            .find(|m| m.role == Role::Tool && m.tool_call_id.as_deref() == Some("call-c2"))
            .expect("second clear reply");
        assert!(
            reply.content.contains("已覆盖目标区间"),
            "守卫须先于 handover 校验给出中性说明：{}",
            reply.content
        );
        assert!(
            !reply.content.contains("防裸清护栏"),
            "不得以护栏拒绝替代中性说明：{}",
            reply.content
        );
        // 恰一枚清零 marker（第二次被守卫拦下）；全程零护栏拒绝事件。
        let marker_count = conversation
            .iter()
            .filter(|m| {
                m.content
                    .starts_with(crate::model_face::CLEAR_MARKER_PREFIX)
            })
            .count();
        assert_eq!(marker_count, 1, "守卫拦截不得落新 marker");
        let evs = events(&dir);
        assert!(
            !evs.iter().any(|e| {
                e.event_type == EventType::ToolCompleted
                    && e.payload["mode"] == "clear"
                    && e.payload["exit_code"] == 1
            }),
            "全程不得出现清零护栏拒绝事件"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 0da S2（2026-10-11，设计 §6.3/§6.5）replace 族钉：`mode=clear_all`
    /// 一键清理 e2e——黑板三分区清空（journal 留痕在事件载荷）→ handover
    /// 成为清空后板面**唯一条目**（先清板后写交接，锚点保序）→ 清零核生效
    /// （marker＋边界事件 mode=clear_all＋board_entries_removed 载荷）。
    #[tokio::test]
    async fn context_manage_clear_all_wipes_board_then_writes_handover() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: Some(ToolResult {
                output: "F".repeat(3_000),
                exit_code: Some(0),
                ..Default::default()
            }),
        };
        let fake = Arc::new(FakeProvider::new(vec![
            // 轮 1：写黑板两分区（制造板面旧条目）。
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "blackboard_write".to_string(),
                arguments: serde_json::json!({
                    "section": "notes",
                    "content": "旧笔记甲",
                }),
                call_id: "call-w1".to_string(),
            }]),
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "blackboard_write".to_string(),
                arguments: serde_json::json!({
                    "section": "findings",
                    "content": "旧结论乙",
                }),
                call_id: "call-w2".to_string(),
            }]),
            // 轮 2：read（形成历史轮）。
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "read_file".to_string(),
                arguments: serde_json::json!({ "path": "src/f.rs" }),
                call_id: "call-r1".to_string(),
            }]),
            // 轮 3：clear_all（一键清理）。
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "context_manage".to_string(),
                arguments: serde_json::json!({
                    "mode": "clear_all",
                    "handover": "目标: 一键清理验证\n已完成: 板面写入\n下一步: 从台账继续",
                }),
                call_id: "call-ca1".to_string(),
            }]),
            // 终答候选轮 ×2。
            ScriptedResponse::text("终答"),
            ScriptedResponse::text("终答确认"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway)
            .with_slider_window_tokens(1_000)
            .with_model_face_block_tokens(1_000);
        let mut conversation: Vec<Message> = Vec::new();
        controller
            .run_turn(
                &host,
                "0da clear_all 端到端",
                "RUN-CZ-CA",
                MANIFEST,
                0,
                None,
                None,
                Some(&mut conversation),
            )
            .await
            .unwrap();

        let evs = events(&dir);
        // 清零事件：mode=clear_all＋板面载荷。
        let cleared: Vec<_> = evs
            .iter()
            .filter(|e| {
                e.event_type == EventType::ContextCompressed && e.payload["mode"] == "clear_all"
            })
            .collect();
        assert_eq!(cleared.len(), 1, "{cleared:?}");
        assert_eq!(
            cleared[0].payload["board_entries_removed"], 2,
            "旧板面恰两分区各一条"
        );
        assert_eq!(
            cleared[0].payload["boundary_round"], 4,
            "清零调用在第 4 轮（两次板写各占一轮）"
        );
        // 板面终态：notes 恰一条【交接摘要】、plan/findings 全空（先清板后写
        // 交接＝锚点保序）。
        let bb = controller.blackboard.read();
        assert_eq!(bb.notes.len(), 1, "清空后板面恰一条（handover）");
        assert!(
            bb.notes[0].content.contains("一键清理验证")
                && bb.notes[0].op.as_deref() == Some("交接摘要"),
            "唯一条目＝交接摘要：{:?}",
            bb.notes[0]
        );
        assert!(bb.plan.model_notes.is_empty() && bb.findings.is_empty());
        // marker 在场＋信封带一键清空回执。
        assert!(
            conversation.iter().any(|m| m
                .content
                .starts_with(crate::model_face::CLEAR_MARKER_PREFIX)),
            "clear_all 须落清零 marker"
        );
        assert!(
            conversation
                .iter()
                .any(|m| m.role == Role::Tool && m.content.contains("黑板已一键清空")),
            "信封须带一键清空回执"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 0da S2（2026-10-11，设计 §6.1/§6.5；D3 并入）replace 族钉：
    /// `blackboard_write op=clear`／section=all 组合约束／非法 op——
    /// ① op=clear 单分区清空（版本计 1＋journal op=clear＋信封回显条数）；
    /// ② section=all＋op=clear 三分区全清；
    /// ③ section=all＋append ⇒ 显式拒绝 exit 1；
    /// ④ 非法 op（replace）⇒ 拒绝 exit 1（D3）。
    #[tokio::test]
    async fn blackboard_write_op_clear_and_all_section_guardrails() {
        let dir = test_dir();
        let bb_call = |call_id: &str, args: serde_json::Value| ToolCall {
            name: "blackboard_write".to_string(),
            arguments: args,
            call_id: call_id.to_string(),
        };
        let fake = Arc::new(FakeProvider::new(vec![
            // ① 写两条 notes（供清空计数）。
            ScriptedResponse::tool_calls(vec![bb_call(
                "call-b1",
                serde_json::json!({ "section": "notes", "content": "笔记一" }),
            )]),
            ScriptedResponse::tool_calls(vec![bb_call(
                "call-b2",
                serde_json::json!({ "section": "notes", "content": "笔记二" }),
            )]),
            // ② op=clear 清空 notes。
            ScriptedResponse::tool_calls(vec![bb_call(
                "call-b3",
                serde_json::json!({ "section": "notes", "op": "clear" }),
            )]),
            // ④ 非法 op（D3：显式拒绝）。
            ScriptedResponse::tool_calls(vec![bb_call(
                "call-b4",
                serde_json::json!({ "section": "notes", "content": "x", "op": "replace" }),
            )]),
            // ③ section=all＋append ⇒ 拒绝。
            ScriptedResponse::tool_calls(vec![bb_call(
                "call-b5",
                serde_json::json!({ "section": "all", "content": "x" }),
            )]),
            // ②′ section=all＋op=clear 三分区全清（直接清空计数验证）。
            ScriptedResponse::tool_calls(vec![bb_call(
                "call-b6",
                serde_json::json!({ "section": "all", "op": "clear" }),
            )]),
            ScriptedResponse::text("终答"),
            ScriptedResponse::text("终答确认"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway);
        let host = TestHost {
            journal: JournalRecorder::new(dir.clone()),
            tool_result: None,
        };
        controller
            .run_turn(
                &host,
                "0da op=clear 守护栏端到端",
                "RUN-CZ-OPC",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let evs = events(&dir);
        // ① op=clear：exit 0＋信封回显条数＋journal op=clear。
        let clear_done: Vec<_> = evs
            .iter()
            .filter(|e| {
                e.event_type == EventType::ToolCompleted
                    && e.payload["op"] == "clear"
                    && e.payload["exit_code"] == 0
            })
            .collect();
        assert_eq!(
            clear_done.len(),
            2,
            "notes 清空＋all 清空恰两次：{clear_done:?}"
        );
        assert_eq!(
            clear_done[0].payload["entries_removed"], 2,
            "notes 旧条目恰两条"
        );
        let plan_writes: Vec<_> = evs
            .iter()
            .filter(|e| e.event_type == EventType::PlanWrite && e.payload["op"] == "clear")
            .collect();
        assert_eq!(plan_writes.len(), 2, "journal 恰两笔 op=clear");
        assert_eq!(plan_writes[0].payload["section"], "notes");
        assert_eq!(plan_writes[0].payload["content_chars"], 0);
        assert_eq!(plan_writes[1].payload["section"], "all");
        // ④ 非法 op 拒绝（D3）。
        let illegal: Vec<_> = evs
            .iter()
            .filter(|e| {
                e.event_type == EventType::ToolCompleted
                    && e.payload["exit_code"] == 1
                    && e.payload["error"]
                        .as_str()
                        .unwrap_or("")
                        .contains("只接受 append | replace_all | clear")
            })
            .collect();
        assert_eq!(illegal.len(), 1, "非法 op 须显式拒绝：{illegal:?}");
        // ③ section=all＋append 拒绝。
        let all_reject: Vec<_> = evs
            .iter()
            .filter(|e| {
                e.event_type == EventType::ToolCompleted
                    && e.payload["exit_code"] == 1
                    && e.payload["error"]
                        .as_str()
                        .unwrap_or("")
                        .contains("all 仅与 op=clear 组合合法")
            })
            .collect();
        assert_eq!(all_reject.len(), 1, "all+append 须显式拒绝：{all_reject:?}");
        // ②′ 板面终态全空＋版本计数随清空 bump（notes 经历写×2→清→(all)清）。
        let bb = controller.blackboard.read();
        assert!(bb.notes.is_empty() && bb.plan.model_notes.is_empty() && bb.findings.is_empty());
        assert_eq!(
            bb.revisions.notes, 4,
            "notes：写2＋clear＋all-clear＝4 次可见变化"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 0cz S2（设计 §5.3）六钉之⑤′：软水位键重置——软档键与 first_block
    /// 复位、硬档键（320k）保留（沿越线重武装）；生效时点＝下一 run。
    #[test]
    fn context_clear_resets_soft_watermark_keys_only() {
        let fake = Arc::new(FakeProvider::new(vec![ScriptedResponse::text("完成")]));
        let controller = AgentLoopController::with_gateway(fake);
        controller.mark_context_scale_notified("192k");
        controller.mark_context_scale_notified("first_block");
        controller.mark_context_scale_notified("320k");
        controller.reset_context_scale_soft_keys();
        let keys = controller.context_scale_notified_keys();
        assert_eq!(keys, vec!["320k".to_string()], "{keys:?}");
    }

    /// 测试局部：read_file 声明（闭块fixture）。
    fn tool_call_read(call_id: &str) -> ToolCall {
        ToolCall {
            name: "read_file".to_string(),
            arguments: serde_json::json!({ "path": "src/f0.rs" }),
            call_id: call_id.to_string(),
        }
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
        let w = controller
            .whitelist
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        assert!(w.is_empty(), "sealed whitelist write must not land: {w:?}");
        drop(w);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
