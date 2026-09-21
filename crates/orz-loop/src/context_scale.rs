//! 滑块上下文 **v8**（2026-09-16 勘误批）——注意力阶梯、硬提醒与硬截断的
//! 非对话面实现：阶梯状态、提醒／告知文案、模型语义摘要块的机械识别、
//! 会话级水位键。
//!
//! ## 量尺（本次勘误的实质处，设计 §3.1）
//!
//! 本模块所有判定吃**模型面估算**＝投影层装配出的请求视图估算
//! （`model_face::model_face_estimate`，chars/2）——**不是**本地全量估算、
//! **不是** provider 实测 prompt。v7 把刻度挂在**本地全量**上（0ai 轮 130＝
//! 501,845 估算，而该轮真实上传仅 73,006 real token）⇒ 提醒对着模型看不见
//! 的量发，属**记录错误 → 实现错误**的勘误对象。
//!
//! ## 阶梯（设计 §3，按 1M 上下文模型普遍注意力水平定稿）
//!
//! | 档 | 模型面估算 | ≈真实 token | 形态 |
//! |---|---|---|---|
//! | R1–R4 | 192 / 224 / 256 / 288K | ≈148 / 172 / 197 / 222K | 软提醒 |
//! | **H1** | **320K** | ≈246K | **硬提醒：打断（开压缩窗口）** |
//! | **T1** | **500K** | ≈385K | **硬截断（主滑块以外的全部分块）** |
//!
//! 换算：`估算 ≈ 真实 ÷ 0.77`（设计 §3.1 换算纪律）。每档**每会话一次**；
//! H1／T1 各一次/会话；950K 取消（它只是 provider 1M 窗口的安全上限、不是
//! 质量许可额度）；1.10M 估算守卫降为**异常保险**（单轮暴涨／换算漂移），
//! 越线**强制截断到线上**（v7 的「不开窗」行为随之作废）。
//!
//! ## 会话级水位
//!
//! 每档每会话一次——已提醒键随会话侧车持久化
//! （`StoredConversation.context_scale_notified`），prompt 起始注入 loop、
//! fire 时回写；新会话独立、恢复不重发。
//!
//! 注入文本纪律：提醒块以 [`REMINDER_INJECTED_PREFIX`] 开头、窗口机械提示以
//! [`WINDOW_NOTICE_PREFIX`] 开头，二者均注册进
//! `prompt::is_injected_block_text` ⇒ 绝不写回持久化会话。模型面术语统一用
//! 「当前上下文窗口」「工作现场（你最近工作的连续轮次）」「分块」，不使用
//! 「滑块／主滑块」「驻留带」「锯齿折叠」等
//! 内部沿革词。

use std::collections::HashSet;

use crate::blackboard::BLACKBOARD_WRITE_TOOL_NAME;
use orz_assurance::tool_names::CONTEXT_COMPRESS_TOOL_NAME;

/// 提醒块注入前缀（注册进 `prompt::is_injected_block_text`，绝不持久化）。
pub const REMINDER_INJECTED_PREFIX: &str = "[CONTEXT_SCALE";

/// 压缩窗口内的机械提示前缀（注册进 `prompt::is_injected_block_text`）。
pub const WINDOW_NOTICE_PREFIX: &str = "[模型参与压缩";

/// **模型语义摘要块的机械可识别前缀**（压缩分工的载体）。
///
/// 模型在回复中输出该块 ⇒ 机械层在下一个 loop-top 安全间隙执行一次
/// 「按块压缩」（`mode=model_summary`）。零新增工具面（8 工具面冻结不动）
/// ——识别面是**回复文本**，不是工具。
pub const MODEL_SUMMARY_PREFIX: &str = "[SEMANTIC_SUMMARY";

/// 语义摘要块结束标记（缺失时按「到文本末尾」容错截取）。
pub const MODEL_SUMMARY_END: &str = "[/SEMANTIC_SUMMARY]";

/// 语义摘要块的六段结构（目标／已完成／关键决策／未决问题／下一步／关键文件）；
/// 机械识别要求至少命中 [`MODEL_SUMMARY_MIN_SECTIONS`] 段，避免正文里偶然
/// 出现前缀即被误判为摘要块。
pub const MODEL_SUMMARY_SECTIONS: [&str; 6] = [
    "目标",
    "已完成",
    "关键决策",
    "未决问题",
    "下一步",
    "关键文件",
];

/// 识别语义摘要块所需的最小段命中数。
pub const MODEL_SUMMARY_MIN_SECTIONS: usize = 2;

/// 摘要块内**可选**的块区间指令标签（模型按块区间指定压缩对象；缺省时机械层
/// 按「最旧闭合块优先」，设计 §4）。
pub const MODEL_SUMMARY_BLOCK_LABEL: &str = "压缩块:";

/// 模型实施压缩的有限轮数（H1 打断后 ≤3 轮；超轮未产出摘要 ⇒ 如实落账
/// `model_participated=false`，**不再机械兜底压缩**——机械层不替模型决定
/// 模型面收缩，设计 §4）。
pub const COMPRESSION_WINDOW_ROUNDS: u32 = 3;

/// 一次性字面水位键：首个分块固化提醒（会话级；2026-09-16 实现批由 per-run
/// 改为会话级——「一次性」指的是一次会话，不是一次 run）。
pub const FLAG_FIRST_BLOCK: &str = "first_block";

/// 阶梯档位形态（设计 §3）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LadderTier {
    /// 软提醒：只报读数与自选压缩方法，不打断、不开窗。
    Soft,
    /// **H1 硬提醒：打断**（注入 ＋ 开压缩窗口 ≤3 轮）。
    HardReminder,
    /// **T1 硬截断**：机械层把主滑块以外的全部分块移出模型面。
    HardTruncate,
}

/// 一档阶梯（模型面估算刻度）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LadderStep {
    pub tokens: u64,
    pub tier: LadderTier,
}

/// v8 默认阶梯（生产固定——刻度值进文案，改值即改语义；测试经
/// `with_context_scale_ladder` 用极小刻度驱动）。
pub const DEFAULT_LADDER: [LadderStep; 6] = [
    LadderStep {
        tokens: 192_000,
        tier: LadderTier::Soft,
    },
    LadderStep {
        tokens: 224_000,
        tier: LadderTier::Soft,
    },
    LadderStep {
        tokens: 256_000,
        tier: LadderTier::Soft,
    },
    LadderStep {
        tokens: 288_000,
        tier: LadderTier::Soft,
    },
    LadderStep {
        tokens: 320_000,
        tier: LadderTier::HardReminder,
    },
    LadderStep {
        tokens: 500_000,
        tier: LadderTier::HardTruncate,
    },
];

/// 一次越线的阶梯事件（文案由调用方按形态渲染——硬截断的文案需要截断后的
/// 事实读数）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LadderFire {
    /// 越线的刻度（模型面估算口径）。
    pub milestone_tokens: u64,
    /// `mechanical_audit_update` 的 key 尾缀（`192k` / `320k` / `500k`）。
    pub key: String,
    pub tier: LadderTier,
}

/// 阶梯状态（**会话级水位**）。
///
/// 2026-09-16 实现批（用户裁定「守卫降值 ＋ 重新武装，双管齐下」）：
///
/// - **软提醒（192/224/256/288K）＝每会话一次**（水位随侧车持久化、恢复不重发）；
/// - **H1／T1 ＝按越线重新武装**（`latched` 闩）：越过线发一次，落到线下即复位
///   ⇒ 每次再越线都会再提醒/再截断。理由＝T1 只响一次时，模型面可以在无任何
///   信号的情况下重新长到守卫线（实测口径），500K 天花板名存实亡；H1 随之重新
///   武装，保证**每一次** T1 之前都先有过一次硬提醒。
#[derive(Debug, Clone, Default)]
pub struct ContextScaleState {
    /// 已发出的档位（软提醒一次/会话；硬档记「发过」供审计与侧车水位）。
    fired: HashSet<u64>,
    /// 硬档闩（越过线为真，落到线下复位）。
    latched: HashSet<u64>,
    /// 字面水位键（`first_block` 等）。
    flags: HashSet<String>,
}

impl ContextScaleState {
    pub fn new() -> Self {
        Self::default()
    }

    /// 从会话侧车水位键（`["192k", "320k", "first_block"]`）恢复。
    pub fn from_notified_keys(keys: &[String]) -> Self {
        let mut fired = HashSet::new();
        let mut flags = HashSet::new();
        for key in keys {
            match parse_key(key) {
                Some(tokens) => {
                    fired.insert(tokens);
                }
                None => {
                    let key = key.trim();
                    // 只认已知字面水位键；其它脏数据忽略（防旧/手工侧车注入）。
                    if key == FLAG_FIRST_BLOCK {
                        flags.insert(key.to_string());
                    }
                }
            }
        }
        Self {
            fired,
            latched: HashSet::new(),
            flags,
        }
    }

    /// 当前水位键（升序、去重；随侧车持久化）。
    pub fn notified_keys(&self) -> Vec<String> {
        let mut out: Vec<(String, String)> = self
            .fired
            .iter()
            .map(|m| (format!("{:09}k", m / 1000), format!("{}k", m / 1000)))
            .collect();
        out.extend(
            self.flags
                .iter()
                .map(|flag| (format!("zz-{flag}"), flag.clone())),
        );
        out.sort();
        out.into_iter().map(|(_, k)| k).collect()
    }

    /// 当前模型面读数下新越线的档位。
    ///
    /// 软档：每会话一次（重复查询零返回）；硬档：按越线重新武装（落到线下后
    /// 再越线即再发）。
    pub fn due(&mut self, model_face_tokens: u64, ladder: &[LadderStep]) -> Vec<LadderFire> {
        let mut out = Vec::new();
        for step in ladder {
            if model_face_tokens < step.tokens {
                // 落到线下 ⇒ 硬档重新武装（软档水位不受影响）。
                self.latched.remove(&step.tokens);
                continue;
            }
            let fresh = match step.tier {
                LadderTier::Soft => self.fired.insert(step.tokens),
                LadderTier::HardReminder | LadderTier::HardTruncate => {
                    self.latched.insert(step.tokens)
                }
            };
            if !fresh {
                continue;
            }
            self.fired.insert(step.tokens);
            out.push(LadderFire {
                milestone_tokens: step.tokens,
                key: format!("{}k", step.tokens / 1000),
                tier: step.tier,
            });
        }
        out
    }

    /// 该刻度是否已提醒过（判据/A-B 读数口用）。
    pub fn has_fired(&self, milestone_tokens: u64) -> bool {
        self.fired.contains(&milestone_tokens)
    }

    /// 字面水位是否已置（`first_block` 等）。
    pub fn has_flag(&self, flag: &str) -> bool {
        self.flags.contains(flag)
    }

    /// 置字面水位（幂等）。
    pub fn mark_flag(&mut self, flag: &str) {
        self.flags.insert(flag.to_string());
    }

    /// **复位硬档闩位**（2026-09-16 实现批）：越线判定为真、但本轮**没有可执行
    /// 的动作**（无已闭合分块／无回放块／无超大结果 ⇒ `Ok(None)`）时调用——
    /// 不消费闩位，下一轮继续试，内容一旦可动就立刻压回线上。
    pub fn rearm(&mut self, milestone_tokens: u64) {
        self.latched.remove(&milestone_tokens);
    }
}

/// 水位键（`"320k"`）→ 刻度（`320_000`）；非法键忽略（向后兼容/防脏数据）。
fn parse_key(key: &str) -> Option<u64> {
    key.trim()
        .strip_suffix('k')
        .or_else(|| key.trim().strip_suffix('K'))
        .and_then(|k| k.trim().parse::<u64>().ok())
        .map(|k| k * 1000)
}

/// 机械读数渲染（文案必含当前读数，且同时给 M 记法与原始 token 数）。
fn reading(tokens: u64) -> String {
    format!(
        "≈{:.2}M token（{tokens} token）",
        tokens as f64 / 1_000_000.0
    )
}

/// 语义摘要块的输出说明（软提醒／硬提醒／窗口块共用——单一来源，避免漂移）。
fn summary_block_guide() -> String {
    format!(
        "在回复中输出一个语义摘要块，机械层据此把**工作现场之外**的旧分块替换成该摘要\
         （逐字原文仍全量留档、可按块回放）：\n\
         {MODEL_SUMMARY_PREFIX}]\n\
         {label} 1-4（可选：不给则由机械层按最旧闭合块优先）\n\
         目标: …\n已完成: …\n关键决策: …\n未决问题: …\n下一步: …\n关键文件: …\n\
         {MODEL_SUMMARY_END}",
        label = MODEL_SUMMARY_BLOCK_LABEL,
    )
}

/// **R1–R4 软提醒**（不打断、不开窗、不强制；压缩交还模型自选、可延后）。
pub fn soft_reminder_block(milestone_tokens: u64, model_face_tokens: u64) -> String {
    let k = milestone_tokens / 1000;
    let reading = reading(model_face_tokens);
    format!(
        "{REMINDER_INJECTED_PREFIX} {k}K] 当前上下文窗口 {reading}。除你**最近工作的\
         连续轮次**（工作现场）以外，更早的内容按**分块**累积在你的窗口里\
         （分块表在窗口**尾部**、逐轮刷新），机械层不会替你删除它们。\
         压缩交给你自选、可延后：{}",
        summary_block_guide()
    )
}

/// **H1 硬提醒（打断式，320K 估算 ≈246K 真实）**——宣告 T1 时将硬性截断
/// 主滑块以外的全部分块，给出当前分块表与压缩方法，并明确「不压缩也可以，
/// 但到时这些块只能靠回查」（设计 §5）。
pub fn hard_reminder_block(
    milestone_tokens: u64,
    truncate_tokens: u64,
    model_face_tokens: u64,
    block_table: &str,
) -> String {
    let k = milestone_tokens / 1000;
    let t1 = truncate_tokens / 1000;
    let reading = reading(model_face_tokens);
    format!(
        "{REMINDER_INJECTED_PREFIX} {k}K · 硬提醒] 当前上下文窗口 {reading}，已越过模型间开始分化的位置\
         （≈246K 真实 token）。到 **{t1}K 估算（≈385K 真实 token）** 时，机械层将\
         **硬性截断工作现场以外的全部已闭合分块**：此后这些内容只能按块回放（read_file 分页）。\
         此后**每再越线一次都会再截断一次**（该线按越线重新武装）——每次截断前都会先收到\
         这条硬提醒。不足一块的**残段**不参与截断，留在窗口内。\n\
         不压缩也可以——但到时这些块只能靠回查。现在就压：{}\n\
         {block_table}\n\
         {declaration}",
        summary_block_guide(),
        declaration = crate::model_face::MODEL_FACE_DECLARATION,
    )
}

/// **T1 硬截断告知块**（设计 §5）：① 已截断 N 块／约 M token；② 可按块回放
/// （给块表与回放口径）；③ 任务无需中止。
///
/// 2026-09-16 审查 R-12③ 处置补录：一轮内只注入最高档 ⇒ T1 那轮的软／硬提醒
/// 被压掉，告知块必须自己带上**截断后的当前读数**（否则该轮模型看不到任何读数）。
pub fn truncation_notice_block(
    truncated_blocks: usize,
    freed_tokens: u64,
    model_face_tokens: u64,
    replay: &str,
    block_table: &str,
    archive_write_failed: bool,
) -> String {
    let failure = if archive_write_failed {
        "\n4. **回放档案写入失败**（`.gsa/compaction/blocks/` 落盘未成功）——\
         被截断分块的逐字原文仍在本会话档案（sidecar）与 run journal 中；\
         请按下方指针或检索 journal `call_id` 回读。\n"
    } else {
        ""
    };
    format!(
        "{REMINDER_INJECTED_PREFIX} 硬截断] 已把**工作现场以外**的 {truncated_blocks} 个**已闭合分块**\
         （≈{freed_tokens}tk token）移出当前上下文窗口（截断后当前读数 {}）：\n\
         1. 已截断 {truncated_blocks} 块／≈{freed_tokens}tk token；\n\
         2. **可按块回放**——逐字原文全量留档（会话档案 ＋ 按块档案 ＋ journal），\
         用 read_file offset/limit 分页读回：\n{replay}\n\
         3. **任务无需中止**：工作现场（最近若干完整轮）与残段逐字未动，继续即可。{failure}\n\
         {block_table}\n\
         {declaration}",
        reading(model_face_tokens),
        declaration = crate::model_face::MODEL_FACE_DECLARATION,
    )
}

/// **700K 估算守卫**（异常保险：单轮暴涨／换算漂移）：越线即**强制截断到
/// 线上**（v7 的「不开窗」行为随勘误作废）。文案如实报三条事实。
pub fn guard_truncation_notice_block(
    truncated_blocks: usize,
    freed_tokens: u64,
    model_face_tokens: u64,
    guard_tokens: u64,
    replay: &str,
    archive_write_failed: bool,
) -> String {
    let failure = if archive_write_failed {
        "\n（提示：回放档案写入失败，逐字原文仍在会话档案与 run journal 中。）"
    } else {
        ""
    };
    format!(
        "{REMINDER_INJECTED_PREFIX} 上限守卫] 当前上下文窗口已越过单请求上限 ≈{} token\
         （异常保险线：单轮暴涨或换算漂移才会触及）。机械层已强制截断工作现场以外的\
         {truncated_blocks} 个**已闭合分块**（≈{freed_tokens}tk token）以把请求压回线上\
         （截断后当前读数 {}）：\n\
         {replay}\n\
         任务无需中止（工作现场与残段逐字未动）；需要更早内容时按上表回放。{failure}",
        reading_only(guard_tokens),
        reading(model_face_tokens),
    )
}

/// 上限读数的轻量渲染（只给 M 记法，避免与**当前读数**混淆）。
fn reading_only(tokens: u64) -> String {
    format!("{:.2}M", tokens as f64 / 1_000_000.0)
}

/// **首次出现主滑块之外的分块**时的一次性固化提醒（A4 文案语义保留，触发
/// 从 v7 的「首次真实驱逐」改锚为「首个分块形成」——v8 没有驱逐）。
pub fn first_block_reminder_block() -> String {
    format!(
        "{REMINDER_INJECTED_PREFIX} 首个分块] 你的当前上下文窗口里首次出现了**工作现场之外**的分块\
         （分块表见下）。这只是一个索引事实：内容仍在你的窗口里，机械层不会替你删除。\
         请把接线结论/关键读数固化到黑板（{BLACKBOARD_WRITE_TOOL_NAME} section=plan|notes）\
         ——黑板不属于上下文窗口，跨压缩与截断都不失效。"
    )
}

/// H1 的压缩窗口任务块（打断式提醒注入——FR-3：不锁工具面；量尺＝模型面阶梯）。
pub fn compression_window_block(milestone_tokens: u64) -> String {
    let k = milestone_tokens / 1000;
    format!(
        "{WINDOW_NOTICE_PREFIX} · 窗口 · 模型面 {k}K] 打断式提醒（不锁工具面、动作照常）。请在窗口内完成：\n\
         1. 产出语义摘要块（见上）——机械层用它替换**工作现场之外**的分块（可按块区间指定）；\n\
         2. 若有关键结论需要跨压缩长期留存，一并固化到黑板\
         （{BLACKBOARD_WRITE_TOOL_NAME} section=plan|notes；黑板不受上下文窗口影响）。\n\
         （读数与再发起可随时调用 {CONTEXT_COMPRESS_TOOL_NAME}：窗口在程中时它只返回当前\
         读数，不会重复开窗。）\n\
         窗口结束仍未产出摘要块 ⇒ 机械层**不做压缩兜底**（模型面总量只由模型自压与\
         H1/T1 管），如实落账 `model_participated=false`。"
    )
}

/// 窗口内模型未产出摘要块时的机械提示（单空格，0AE-C13 文案纪律）。
pub fn window_remaining_notice(rounds_left: u32) -> String {
    format!(
        "{WINDOW_NOTICE_PREFIX}] 窗口剩余 {rounds_left} 轮：尚未检测到语义摘要块。\
         请输出 `{MODEL_SUMMARY_PREFIX}] … {MODEL_SUMMARY_END}`（或把必要结论写入黑板）。"
    )
}

/// 0ap（2026-09-18，设计 §1/§0 表）：`context_compress` 调用的机械判定
/// 三态（防抖；纯内存压缩状态操作，fail-soft——三态都是 exit 0 信封，
/// 不报错不阻断）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressRequestState {
    /// 请求受理：下一个 loop-top 安全边界开 D3 模型参与压缩窗口。
    Requested,
    /// 窗口已在程中：no-op 防连点，只返回当前读数。
    InProgress,
    /// 主滑块外无可压缩分块：中性说明返回、不开窗（不虚构动作；沿
    /// Unknown 档文案纪律——如实说明而非报错）。
    NothingToCompress,
}

/// 0ap：`context_compress` 响应文本（防抖三态说明＋滑块读数表；设计 §3
/// 辅面——与 `blackboard_read` 头增段共用同一渲染）。只出数量与估算，
/// 分块内容不流出模型面。
pub fn context_compress_response(
    state: CompressRequestState,
    readout: &crate::model_face::SliderReadout,
) -> String {
    let table = crate::model_face::render_slider_readout_line(readout);
    let total = readout.total_blocks;
    let head = match state {
        CompressRequestState::Requested => format!(
            "压缩窗口已请求：下一个安全边界将开启模型参与压缩窗口（≤3 轮）。\
             窗口轮请产出语义摘要块（机械层据以折叠主滑块外的已闭合分块），\
             必要时用 {BLACKBOARD_WRITE_TOOL_NAME} 固化关键结论。"
        ),
        CompressRequestState::InProgress => {
            "压缩窗口已在程中（in_progress）：本轮即窗口轮，请直接产出语义摘要块或固化黑板；无需重复发起。"
                .to_string()
        }
        CompressRequestState::NothingToCompress => {
            "主滑块之外没有可压缩分块：无需压缩，未开窗。".to_string()
        }
    };
    format!("{head}\n滑块读数：{table}（主滑块外共 {total} 块）。")
}

/// **语义摘要块的机械识别**：从模型回复文本中抽取最后一块摘要（前缀到结束
/// 标记；结束标记缺失时取到文本末尾），至少命中
/// [`MODEL_SUMMARY_MIN_SECTIONS`] 个段标签才算数（防误识别）。返回去除首尾
/// 空白后的块文本（含首尾标记）。
pub fn extract_model_summary(text: &str) -> Option<String> {
    let start = text.rfind(MODEL_SUMMARY_PREFIX)?;
    let rest = &text[start..];
    let body = match rest.find(MODEL_SUMMARY_END) {
        Some(end) => &rest[..end + MODEL_SUMMARY_END.len()],
        None => rest,
    };
    let hits = MODEL_SUMMARY_SECTIONS
        .iter()
        .filter(|section| body.contains(**section))
        .count();
    if hits < MODEL_SUMMARY_MIN_SECTIONS {
        return None;
    }
    Some(body.trim().to_string())
}

/// 摘要块内的**块区间指令**（可选）：`压缩块: 1-4, 6` → `[1,2,3,4,6]`。
/// 缺省／非法 ⇒ `None`（机械层按「最旧闭合块优先」）。
pub fn extract_block_selection(summary: &str) -> Option<Vec<u32>> {
    let line = summary
        .lines()
        .map(str::trim_start)
        .find(|l| l.starts_with(MODEL_SUMMARY_BLOCK_LABEL))?;
    let spec = line.trim_start_matches(MODEL_SUMMARY_BLOCK_LABEL);
    let mut out: Vec<u32> = Vec::new();
    for part in spec.split([',', '，']).map(str::trim) {
        if part.is_empty() {
            continue;
        }
        let (a, b) = match part.split_once('-') {
            Some((a, b)) => (a.trim(), b.trim()),
            None => (part, part),
        };
        let (Ok(a), Ok(b)) = (a.parse::<u32>(), b.parse::<u32>()) else {
            return None;
        };
        if a == 0 || b < a || b > 100_000 {
            return None;
        }
        for n in a..=b {
            if out.len() >= 512 {
                return Some(out);
            }
            out.push(n);
        }
    }
    (!out.is_empty()).then_some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 0ap S1 钉②（2026-09-18，设计 §6）：`context_compress` 响应信封
    /// 三态——Requested / InProgress（no-op 防抖）/ NothingToCompress
    /// （中性不开窗），全部携带滑块读数表（与 blackboard_read 头同渲染）。
    #[test]
    fn context_compress_response_covers_the_three_debounce_states() {
        let readout = crate::model_face::SliderReadout {
            compressible_blocks: 3,
            compressible_estimate_tokens: 153_600,
            total_blocks: 5,
        };
        let requested = context_compress_response(CompressRequestState::Requested, &readout);
        assert!(requested.contains("压缩窗口已请求"), "{requested}");
        assert!(requested.contains("blackboard_write"), "{requested}");
        assert!(
            requested.contains("滑块读数：滑块外可压缩 3 块 ≈ est 153K（主滑块外共 5 块）"),
            "{requested}"
        );

        let in_progress = context_compress_response(CompressRequestState::InProgress, &readout);
        assert!(
            in_progress.contains("压缩窗口已在程中（in_progress）"),
            "{in_progress}"
        );
        assert!(
            !in_progress.contains("已请求"),
            "in-progress 态不得再次宣请开窗: {in_progress}"
        );

        let nothing = context_compress_response(CompressRequestState::NothingToCompress, &readout);
        assert!(nothing.contains("没有可压缩分块"), "{nothing}");
        assert!(nothing.contains("未开窗"), "{nothing}");
        // 读数表三态同源（设计 §3：辅面自带同表）。
        for text in [requested, in_progress, nothing] {
            assert!(text.contains("滑块读数："), "{text}");
        }
    }

    #[test]
    fn ladder_fires_each_tier_exactly_once_and_in_policy_order() {
        let mut state = ContextScaleState::new();
        assert!(state.due(100_000, &DEFAULT_LADDER).is_empty());
        let fires = state.due(330_000, &DEFAULT_LADDER);
        assert_eq!(
            fires.iter().map(|f| f.milestone_tokens).collect::<Vec<_>>(),
            vec![192_000, 224_000, 256_000, 288_000, 320_000]
        );
        assert_eq!(fires[0].tier, LadderTier::Soft);
        assert_eq!(fires[4].tier, LadderTier::HardReminder);
        // 每档一次：重复查询零返回；T1 单独一次。
        assert!(state.due(330_000, &DEFAULT_LADDER).is_empty());
        let fires = state.due(501_000, &DEFAULT_LADDER);
        assert_eq!(fires.len(), 1);
        assert_eq!(fires[0].tier, LadderTier::HardTruncate);
        assert_eq!(fires[0].key, "500k");
        assert!(state.due(900_000, &DEFAULT_LADDER).is_empty());
    }

    #[test]
    fn session_watermark_round_trips_and_suppresses_already_notified_tiers() {
        let mut state = ContextScaleState::from_notified_keys(&[
            "192k".to_string(),
            "320k".to_string(),
            FLAG_FIRST_BLOCK.to_string(),
            "垃圾".to_string(),
        ]);
        // 软档＝水位压住；**硬档＝按越线重新武装**（恢复会话仍在线上 ⇒ H1 再提醒
        // 一次，这是设计意图：先警告再发请求）。
        let fires = state.due(330_000, &DEFAULT_LADDER);
        assert_eq!(
            fires.iter().map(|f| f.milestone_tokens).collect::<Vec<_>>(),
            vec![224_000, 256_000, 288_000, 320_000]
        );
        // 字面水位随侧车往返（first_block），不参与数值刻度。
        assert!(state.has_flag(FLAG_FIRST_BLOCK));
        assert!(
            state
                .notified_keys()
                .contains(&FLAG_FIRST_BLOCK.to_string())
        );
        // 落到线下 ⇒ 硬档复位；再越线 ⇒ H1/T1 各自**再发一次**。
        state.due(100_000, &DEFAULT_LADDER);
        let again = state.due(501_000, &DEFAULT_LADDER);
        assert_eq!(
            again.iter().map(|f| f.milestone_tokens).collect::<Vec<_>>(),
            vec![320_000, 500_000]
        );
        assert_eq!(
            state.notified_keys(),
            vec![
                "192k",
                "224k",
                "256k",
                "288k",
                "320k",
                "500k",
                FLAG_FIRST_BLOCK
            ]
        );
    }

    #[test]
    fn reminder_blocks_carry_the_model_face_reading_and_state_the_truncation_line() {
        let soft = soft_reminder_block(192_000, 194_321);
        assert!(soft.starts_with(REMINDER_INJECTED_PREFIX));
        assert!(soft.contains("194321 token"));
        assert!(soft.contains("工作现场"));
        assert!(!soft.contains("已打断"));
        let hard = hard_reminder_block(
            320_000,
            500_000,
            322_000,
            "[上下文分块表 v0.1]\n[/上下文分块表]",
        );
        assert!(hard.contains("320K · 硬提醒"));
        assert!(hard.contains("500K"));
        assert!(hard.contains("硬性截断"));
        assert!(hard.contains("不压缩也可以"));
        assert!(hard.contains(crate::model_face::MODEL_FACE_DECLARATION));
        let notice =
            truncation_notice_block(3, 41_000, 460_000, "- 块#1 …", "[上下文分块表 v0.1]", false);
        assert!(notice.contains("已截断 3 块"));
        assert!(notice.contains("任务无需中止"));
        // 2026-09-16（审查 R-12③ 处置）：一层内只注入最高档 ⇒ 告知块自带
        // **截断后读数**，压掉同轮软／硬提醒才是无损的。
        assert!(notice.contains("截断后当前读数"), "{notice}");
        assert!(notice.contains("≈0.46M token"), "{notice}");
        assert!(notice.contains(crate::model_face::MODEL_FACE_DECLARATION));
    }

    #[test]
    fn reminder_blocks_are_registered_injected_text() {
        for block in [
            soft_reminder_block(192_000, 192_000),
            hard_reminder_block(320_000, 500_000, 320_000, "表"),
            truncation_notice_block(1, 1_000, 460_000, "- 块#1 …", "表", false),
            guard_truncation_notice_block(1, 1_000, 460_000, 700_000, "- 块#1 …", false),
            first_block_reminder_block(),
            compression_window_block(320_000),
        ] {
            assert!(
                crate::prompt::is_injected_block_text(&block),
                "injected block not registered: {block}"
            );
        }
    }

    #[test]
    fn summary_block_is_extracted_with_optional_block_selection() {
        let text =
            "先说明。\n[SEMANTIC_SUMMARY]\n压缩块: 2-4\n目标: x\n已完成: y\n[/SEMANTIC_SUMMARY]\n";
        let summary = extract_model_summary(text).expect("summary extracted");
        assert!(summary.starts_with(MODEL_SUMMARY_PREFIX));
        assert_eq!(extract_block_selection(&summary), Some(vec![2, 3, 4]));
        assert!(
            extract_model_summary("[SEMANTIC_SUMMARY]\n目标: 只有一个段\n[/SEMANTIC_SUMMARY]")
                .is_none()
        );
        assert_eq!(
            extract_block_selection("[SEMANTIC_SUMMARY]\n目标: x\n"),
            None
        );
    }
}
