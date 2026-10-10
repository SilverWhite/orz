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
//! | R1–R2 | 192 / 256K | ≈148 / 197K | 软提醒（**64K 步距**；0bh ④ 定稿取消 224K） |
//! | **H1** | **320K** | ≈246K | **硬提醒：打断（开压缩窗口）** |
//! | **T1** | **500K** | ≈385K | **必定压缩（2026-09-24 补足，用户裁决）：强制开窗两级询问，第三次机械截断** |
//!
//! 换算：`估算 ≈ 真实 ÷ 0.77`（设计 §3.1 换算纪律）。每档**每会话一次**；
//! 0bh ④（2026-09-22，用户定稿「保留 192／256 双档」）——软梯由 32K 步距拉成
//! 64K 步距，方向是**少提醒、拉带宽**而非抬阈值。**承重件推论**（随 0bh S1
//! 文档化）：真机 0bg 轮 7 次压缩**全部** `mode=model_summary／reason=model_selected`
//! （机械 RHYTHM 线 0 次）⇒ 软提醒是「让压缩保持模型自撰」的承重件，故只拉
//! 步距、不砍档。
//! H1／T1 各一次/会话；950K 取消（它只是 provider 1M 窗口的安全上限、不是
//! 质量许可额度）；700K 估算守卫降为**异常保险**（单轮暴涨／换算漂移），
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
use orz_assurance::tool_names::CONTEXT_MANAGE_TOOL_NAME;

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

/// 全角冒号变体（0bk S1：模型常写 `压缩块：`，不再使区间行漏识别——漏识别
/// 曾导致静默走缺省兜底）。
pub const MODEL_SUMMARY_BLOCK_LABEL_FULLWIDTH: &str = "压缩块：";

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
    /// **T1 必定压缩线**（2026-09-24 补足，用户裁决）：越线先强制开压缩窗口
    /// （两级询问），第三次仍不产出 ⇒ 机械截断兜底（仅留主滑块，可回查存档）。
    /// 枚举名保留 `HardTruncate`（审计键／水位键随档位刻度命名，不做破坏性改名）。
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
///
/// 0bh ④（2026-09-22 用户定稿）：软档 **192／256**（64K 步距），取消 224K；
/// 320（H1 硬提醒）／500（T1 必定压缩）不动。每一软档恰贴在一条机械线之前
/// （192K≈RHYTHM／256K≈FALLBACK），单档含义唯一。
pub const DEFAULT_LADDER: [LadderStep; 4] = [
    LadderStep {
        tokens: 192_000,
        tier: LadderTier::Soft,
    },
    LadderStep {
        tokens: 256_000,
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
/// - **软提醒（192/256K）＝每会话一次**（水位随侧车持久化、恢复不重发）；
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
         （逐字原文仍全量留档、可按块回放；**至少命中 2 个小节**才被机械层识别）：\n{}",
        summary_block_skeleton()
    )
}

/// f15（0bv，2026-09-26）：语义摘要块**可照抄骨架**——软提醒／硬提醒／窗口块
/// 与 `context_compress` 压缩回执共用（单一来源）。c 轮实证：回执只教「产出
/// 摘要块」而没给落点与骨架，前两次投递落空；回执现在直接内嵌本骨架。
fn summary_block_skeleton() -> String {
    format!(
        "{MODEL_SUMMARY_PREFIX}]\n\
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
         压缩交给你自选、可延后：{}\n\
         {}",
        summary_block_guide(),
        context_permission_line()
    )
}

/// **H1 硬提醒（打断式，320K 估算 ≈246K 真实）**——宣告 T1 线的必定压缩
/// 语义（2026-09-24 补足：两次强制窗口 ⇒ 第三次机械截断），给出当前分块表与
/// 压缩方法（设计 §5＋§5 补足）。
///
/// 0bs ④（2026-09-25）：**不再内嵌整张分块表**——表在窗口尾部逐轮刷新，
/// 提醒块只给**单源指向**（此前内嵌副本与尾部表同渲染 ⇒ 同一「说明行」
/// 在一份请求里出现两次；判据＝单源一次）。
pub fn hard_reminder_block(
    milestone_tokens: u64,
    truncate_tokens: u64,
    model_face_tokens: u64,
) -> String {
    let k = milestone_tokens / 1000;
    let t1 = truncate_tokens / 1000;
    let reading = reading(model_face_tokens);
    format!(
        "{REMINDER_INJECTED_PREFIX} {k}K · 硬提醒] 当前上下文窗口 {reading}，已越过模型间开始分化的位置\
         （≈246K 真实 token）。到 **{t1}K 估算（≈385K 真实 token）** 时，机械层将**强制开压缩窗口**，\
         要求把工作现场以外的全部已闭合分块压缩；两次窗口内仍未产出 ⇒ 第三次机械层将**仅保留工作现场**，\
         把其余已闭合分块移出窗口（逐字原文全量留档，需要前置上下文时按块回查存档）。\n\
         不足一块的**残段**不参与，留在窗口内。**是否现在压缩、压缩哪些块由你判断**（压缩交给你自选、\
         可延后；{}）——若决定压缩：{}\n\
         {}\n\
         {declaration}\n\
         {permission}",
        target_tier_advice(),
        summary_block_guide(),
        crate::model_face::BLOCK_TABLE_POINTER_LINE,
        declaration = crate::model_face::MODEL_FACE_DECLARATION,
        permission = context_permission_line()
    )
}

/// **T1 第三步机械截断告知块**（设计 §5＋2026-09-24 必定压缩补足）：① 已截断
/// N 块／约 M token；② 可按块回放（给块表与回放口径）；③ 工作现场与残段未动、
/// **需要前置上下文时回查存档**（状况陈述）；`mandatory_window_fact` 非空时
/// 如实记录「已两次强制开窗未产出」（仅三步升级路径携带）。
///
/// 2026-09-16 审查 R-12③ 处置补录：一轮内只注入最高档 ⇒ T1 那轮的软／硬提醒
/// 被压掉，告知块必须自己带上**截断后的当前读数**（否则该轮模型看不到任何读数）。
/// 0bh ⑯ 子项（2026-09-22，用户批准「去判断而非去建议」，处置优先级 删＞保留＞改）：
/// 删除原③句「任务无需中止：…继续即可」——「该不该继续」是模型的判断，
/// 机械层只留事实（「工作现场与残段逐字未动」）与回放指引。
///
/// 0bs ④（2026-09-25）：不再内嵌整表（同 H1；判据＝单源一次）。
pub fn truncation_notice_block(
    truncated_blocks: usize,
    freed_tokens: u64,
    model_face_tokens: u64,
    replay: &str,
    archive_write_failed: bool,
    mandatory_window_fact: &str,
) -> String {
    let failure = if archive_write_failed {
        "\n4. **回放档案写入失败**（`.gsa/compaction/blocks/` 落盘未成功）——\
         被截断分块的逐字原文仍在本会话档案（sidecar）与 run journal 中；\
         请按下方指针或检索 journal `call_id` 回读。\n"
    } else {
        ""
    };
    let fact = if mandatory_window_fact.is_empty() {
        ""
    } else {
        mandatory_window_fact
    };
    format!(
        "{REMINDER_INJECTED_PREFIX} 硬截断] 已把**工作现场以外**的 {truncated_blocks} 个**已闭合分块**\
         （≈{freed_tokens}tk token）移出当前上下文窗口（截断后当前读数 {}）：\n\
         1. 已截断 {truncated_blocks} 块／≈{freed_tokens}tk token；\n\
         2. **可按块回放**——逐字原文全量留档（会话档案 ＋ 按块档案 ＋ journal），\
         用 read_file offset/limit 分页读回：\n{replay}\n\
         3. 工作现场（最近若干完整轮）与残段逐字未动；**需要前置上下文时请回查存档**\
         ——按上方指针分页读回即可。{failure}\n\
         {fact}\n\
         {}\n\
         {declaration}\n\
         {permission}",
        reading(model_face_tokens),
        crate::model_face::BLOCK_TABLE_POINTER_LINE,
        declaration = crate::model_face::MODEL_FACE_DECLARATION,
        permission = context_permission_line()
    )
}

/// 0bk ②（2026-09-24）：`压缩块:` 区间行存在但解析不出合法区间 ⇒ 如实回报
/// （引用原行＋给出当前可压区间与正确语法），**不压缩、不静默退化**。
pub fn block_selection_unrecognized_notice(line: &str, compressible: &str) -> String {
    // 0bm 复审补口（2026-09-24）：可压集合为空时渲染「（无）」，不给「区间：」
    // 后接空串的残缺句。
    let compressible = if compressible.is_empty() {
        "（无）"
    } else {
        compressible
    };
    format!(
        "{WINDOW_NOTICE_PREFIX} · 区间未识别] 你的语义摘要块中的区间行未能解析出任何合法区间，\
         本次**未执行压缩**：\n原行：{line}\n\
         当前可压区间：{compressible}（只含「已闭合且仍为原文」的分块）。\n\
         正确写法：`{MODEL_SUMMARY_BLOCK_LABEL} 1-4, 6`（可带行内说明，\
         如 `{MODEL_SUMMARY_BLOCK_LABEL} 1-62（全部已闭合块）`）。\n\
         {declaration}",
        declaration = crate::model_face::MODEL_FACE_DECLARATION,
    )
}

/// 0bs ⑥（2026-09-25）：**摘要未落地回执**——F12 观测性缺口：语义摘要被识别、
/// 机械层尝试落地但**未压缩任何分块**（指定区间无命中／当前无可压分块／台账
/// 写失败）时，旧行为只在审计与 tracing 落账、模型侧静默——模型无法判断
/// 「握手是否完成」。本回执把未落地做成与 `block_selection_unrecognized_notice`
/// 同形的如实告知（不判罚、不改语义、不阻断动作）。落地成功的回执＝压缩
/// marker（既有告知面），本函数不出。
pub fn summary_not_landed_notice(reason: &str, compressible: &str) -> String {
    let compressible = if compressible.is_empty() {
        "（无）"
    } else {
        compressible
    };
    format!(
        "{WINDOW_NOTICE_PREFIX} · 压缩回执] 语义摘要**已收到，但未压缩任何分块**（未落地）：\
         {reason}。\n\
         当前可压区间：{compressible}（只含「已闭合且仍为原文」的分块；未闭合的残段\
         与工作现场不参与）。\n\
         动作不阻断：照常继续即可；如需压缩，在摘要块里写明可压区间内的块号重投\
         （等目标块闭合后再投同效）。\n\
         {declaration}",
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
         工作现场与残段逐字未动；需要更早内容时按上表回放。{failure}\n\
         {}",
        reading_only(guard_tokens),
        reading(model_face_tokens),
        context_permission_line()
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

/// 0bh ④（2026-09-22，门二＝A「只做目标档建议」）：压缩**目标档**建议——
/// 建议把模型面读数压到最低软档以下（贴线复压会立刻再来一轮；真机实测连号
/// 压缩仅隔 52 个事件）。**只告知、不做机械护栏**：不加折叠下限、不动
/// `max_reduction_ratio`，压缩仍交模型自选。
pub const COMPRESSION_TARGET_TIER_TOKENS: u64 = 192_000;

/// 目标档建议行（单一来源；`context_compress` 响应与 H1 窗口块共用）。
fn target_tier_advice() -> String {
    format!(
        "目标档：≤{}K（R1 软档——压到最低软档以下，避免贴线复压）",
        COMPRESSION_TARGET_TIER_TOKENS / 1000
    )
}

/// 0cz S2（2026-10-11，设计 §7）：**兜底联动一步权限提醒**——单一来源句，
/// 全部压缩/截断提醒块共用（用户裁决④「跟着现有上下文压缩提醒走＝改叙述、
/// 不加新注入点」；钉子＝本函数被六块引用、句面单源一次）。
/// 0da S2（设计 §6.4）：句面扩**四选择**（压缩／清零续台账／一键清理／
/// 清黑板）——清理操作多样化（用户裁决②）的提醒面落地。
pub fn context_permission_line() -> String {
    format!(
        "你拥有上下文管理权限：可随时调用 {CONTEXT_MANAGE_TOOL_NAME} 主动压缩\
         （mode=compress）；或在把必要结论固化到黑板（{BLACKBOARD_WRITE_TOOL_NAME}）后\
         直接清零当前窗口仅凭台账继续工作（mode=clear，需先写交接摘要 handover）；\
         也可一键清理上下文＋黑板（mode=clear_all，handover 成为清空后板面唯一条目）；\
         黑板分区本身可随时清空（{BLACKBOARD_WRITE_TOOL_NAME} op=clear，历史全量在 \
         journal 可回查）。"
    )
}

/// H1 的压缩窗口任务块（打断式提醒注入——FR-3：不锁工具面；量尺＝模型面阶梯）。
pub fn compression_window_block(milestone_tokens: u64) -> String {
    let k = milestone_tokens / 1000;
    format!(
        "{WINDOW_NOTICE_PREFIX} · 窗口 · 模型面 {k}K] 打断式提醒（不锁工具面、动作照常）。请在窗口内完成：\n\
         1. 产出语义摘要块（见上）——机械层用它替换**工作现场之外**的分块（可按块区间指定）；\
         {}。\n\
         2. 若有关键结论需要跨压缩长期留存，一并固化到黑板\
         （{BLACKBOARD_WRITE_TOOL_NAME} section=plan|notes；黑板不受上下文窗口影响）。\n\
         （读数与再发起可随时调用 {CONTEXT_MANAGE_TOOL_NAME}（mode=compress）：窗口在程中时它只返回当前\
         读数，不会重复开窗。）\n\
         窗口结束仍未产出摘要块 ⇒ 如实落账 `model_participated=false`\
         （机械层不替你压缩；到必定压缩线时机械层将强制再次开窗）。\n\
         {}",
        target_tier_advice(),
        context_permission_line()
    )
}

/// T1 线（必定压缩）的强制压缩窗口任务块——两级询问（2026-09-24 用户裁决的
/// 三步升级）：`attempt`＝第几次询问（1＝首问；2＝升级再询问，明示质量衰减
/// 与最后机会）。FR-3 口径不变：不锁工具面；量尺＝模型面阶梯。
/// 0bn R2（2026-09-24 复审补口，v8 §15）：**摘要格式模板自嵌块本体**——
/// 不再依赖任何早前注入的「见上」指称（H1 块可被移出／单轮暴涨被最高档抑制
/// ／恢复会话不回放注入块），块随窗口注入即自带说明。
///
/// 0bs ④（2026-09-25）：不再内嵌整表——摘要模板仍自嵌（R2 不动），
/// 分块索引改单源指向窗口尾部表（判据＝单源一次）。
pub fn mandatory_compression_window_block(
    milestone_tokens: u64,
    attempt: u32,
    model_face_tokens: u64,
) -> String {
    let k = milestone_tokens / 1000;
    let reading = reading(model_face_tokens);
    let (head, after) = if attempt <= 1 {
        (
            format!(
                "{WINDOW_NOTICE_PREFIX} · 窗口 · 必定压缩 · {k}K] 当前上下文窗口 {reading}，\
                 已越过**必定压缩线**（{k}K 估算 ≈385K 真实 token）。机械层不替你截断——请你现在压缩："
            ),
            "窗口收口仍未产出 ⇒ 水位越线期间会再次开窗询问（共两次窗口机会）；\
             第三次仍未产出 ⇒ 机械层将**仅保留工作现场**，把其余已闭合分块移出窗口。",
        )
    } else {
        (
            format!(
                "{WINDOW_NOTICE_PREFIX} · 窗口 · 必定压缩 · {k}K · 第二次询问] 当前上下文窗口 {reading}，\
                 仍在必定压缩线（{k}K 估算 ≈385K 真实 token）之上。**上下文质量已严重衰减**\
                 ——≈385K 真实 token 已过模型普遍可靠下沿，压缩是解决这一问题的途径。\
                 这是**最后一次压缩窗口**："
            ),
            "窗口收口仍未产出 ⇒ 机械层将**仅保留工作现场**，把其余已闭合分块全部移出窗口\
             （逐字原文全量留档，需要前置上下文时按块回查存档）。",
        )
    };
    format!(
        "{head}\n\
         1. 产出语义摘要块，覆盖**工作现场以外的全部已闭合分块**\
         （不给 `{MODEL_SUMMARY_BLOCK_LABEL}` 行 ＝ 按全部处理；也可写区间收窄）。\
         摘要块格式（自嵌本块，不依赖早前注入）：\n\
         {guide}\n\
         2. 若有关键结论需要跨压缩长期留存，一并固化到黑板\
         （{BLACKBOARD_WRITE_TOOL_NAME} section=plan|notes；黑板不受上下文窗口影响）。\n\
         {after}\n\
         （读数与再发起可随时调用 {CONTEXT_MANAGE_TOOL_NAME}（mode=compress）：窗口在程中时它只返回当前读数。）\n\
         {pointer}\n{declaration}\n\
         {permission}",
        guide = summary_block_guide(),
        pointer = crate::model_face::BLOCK_TABLE_POINTER_LINE,
        declaration = crate::model_face::MODEL_FACE_DECLARATION,
        permission = context_permission_line()
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
    // f15（0bv，2026-09-26）：回执直接给出**落点与可照抄骨架**——摘要块必须
    // 写在回复文本里（写黑板/工具参数不触发折叠），c 轮前两次投递落空即此因。
    let head = match state {
        CompressRequestState::Requested => format!(
            "压缩窗口已请求：下一个安全边界将开启模型参与压缩窗口（≤3 轮）。\
             窗口轮请产出语义摘要块（机械层据以折叠主滑块外的已闭合分块），\
             必要时用 {BLACKBOARD_WRITE_TOOL_NAME} 固化关键结论。{}\
             落点＝**回复文本**（写黑板或工具参数不会触发折叠）；可照抄骨架：\n{}",
            target_tier_advice(),
            summary_block_skeleton()
        ),
        CompressRequestState::InProgress => format!(
            "压缩窗口已在程中（in_progress）：本轮即窗口轮，请直接产出语义摘要块或固化黑板；无需重复发起。\
             落点＝**回复文本**（写黑板或工具参数不会触发折叠）；可照抄骨架：\n{}",
            summary_block_skeleton()
        ),
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

/// `压缩块:` 区间指令的三态解析结果（0bk S1，2026-09-24）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockSelection {
    /// 摘要块没有 `压缩块:` 行——走缺省语义（模型自选窗＝最旧一块；强制窗＝
    /// 全部已闭合分块），回执如实标注属缺省行为（0bk ③）。
    NotSpecified,
    /// 解析成功（容忍行内注解与全角冒号）。
    Specified(Vec<u32>),
    /// 有 `压缩块:` 行但解析不出任何合法区间——非法段仍拒 ⇒ 如实回报，
    /// 不得静默退化为缺省兜底（0bk ②）。
    Unrecognized(String),
}

/// 摘要块内的**块区间指令**（0bk S1 放宽解析）：`压缩块: 1-4, 6` →
/// `Specified([1,2,3,4,6])`。带注解区间不再使解析失败——每段取**数字核心**
/// （前导 数字/`-`/空白 连续段），其后内容视为行内注解忽略（`1-62（全部已
/// 闭合块；工作现场保留）` ⇒ `[1..62]`）；全角冒号 `压缩块：` 同样识别。
/// 某段连数字核心都没有（`abc`）或区间非法 ⇒ 整条 `Unrecognized`（非法段仍
/// 拒，携带原文行供回执如实引用）；没有 `压缩块:` 行 ⇒ `NotSpecified`。
pub fn parse_block_selection(summary: &str) -> BlockSelection {
    let Some(line) = summary.lines().map(str::trim_start).find(|l| {
        l.starts_with(MODEL_SUMMARY_BLOCK_LABEL)
            || l.starts_with(MODEL_SUMMARY_BLOCK_LABEL_FULLWIDTH)
    }) else {
        return BlockSelection::NotSpecified;
    };
    let spec = line
        .strip_prefix(MODEL_SUMMARY_BLOCK_LABEL)
        .or_else(|| line.strip_prefix(MODEL_SUMMARY_BLOCK_LABEL_FULLWIDTH))
        .unwrap_or(line);
    let mut out: Vec<u32> = Vec::new();
    // 0bm 复审补口（2026-09-24）：`、` 是模型写多段区间的常见分隔（如
    // `压缩块: 1-4、6`）——不识别会把「6」当注解静默截断，与 0bk ②「如实
    // 回报」相悖；与 `,`/`，` 同列分隔集。
    for part in spec.split([',', '，', '、']).map(str::trim) {
        if part.is_empty() {
            continue;
        }
        let core: String = part
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '-' || c.is_whitespace())
            .collect();
        let (a, b) = match core.trim().split_once('-') {
            Some((a, b)) => (a.trim(), b.trim()),
            None => (core.trim(), core.trim()),
        };
        let (Ok(a), Ok(b)) = (a.parse::<u32>(), b.parse::<u32>()) else {
            return BlockSelection::Unrecognized(line.to_string());
        };
        if a == 0 || b < a || b > 100_000 {
            return BlockSelection::Unrecognized(line.to_string());
        }
        for n in a..=b {
            if out.len() >= 512 {
                return BlockSelection::Specified(out);
            }
            out.push(n);
        }
    }
    if out.is_empty() {
        // `压缩块:` 行存在但没给出任何区间本体——无区间可执行，同非法处理。
        BlockSelection::Unrecognized(line.to_string())
    } else {
        BlockSelection::Specified(out)
    }
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
        assert!(requested.contains("目标档：≤192K"), "{requested}");
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

    /// 0bh ④ 钉（2026-09-22，门二＝A）：目标档建议**只做告知、不做机械护栏**
    /// ——`context_compress` 响应与 H1 窗口块共用同一行建议（单一来源）；
    /// 建议目标＝最低软档（192K）；不加折叠下限、不动 `max_reduction_ratio`。
    #[test]
    fn compression_target_tier_advice_is_shared_and_advisory() {
        let readout = crate::model_face::SliderReadout {
            compressible_blocks: 2,
            compressible_estimate_tokens: 102_400,
            total_blocks: 3,
        };
        let requested = context_compress_response(CompressRequestState::Requested, &readout);
        assert!(requested.contains("目标档：≤192K"), "{requested}");
        let window = compression_window_block(320_000);
        assert!(window.contains("目标档：≤192K"), "{window}");
        assert_eq!(COMPRESSION_TARGET_TIER_TOKENS, 192_000);
        // 「只告知」：建议文本不得携带机械扣留语义（护栏词）。
        assert!(!requested.contains("必须"), "{requested}");
        assert!(!window.contains("必须"), "{window}");
    }

    #[test]
    fn ladder_fires_each_tier_exactly_once_and_in_policy_order() {
        let mut state = ContextScaleState::new();
        assert!(state.due(100_000, &DEFAULT_LADDER).is_empty());
        let fires = state.due(330_000, &DEFAULT_LADDER);
        assert_eq!(
            fires.iter().map(|f| f.milestone_tokens).collect::<Vec<_>>(),
            vec![192_000, 256_000, 320_000]
        );
        assert_eq!(fires[0].tier, LadderTier::Soft);
        assert_eq!(fires[2].tier, LadderTier::HardReminder);
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
            vec![256_000, 320_000]
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
            vec!["192k", "256k", "320k", "500k", FLAG_FIRST_BLOCK]
        );
    }

    #[test]
    fn reminder_blocks_carry_the_model_face_reading_and_state_the_truncation_line() {
        let soft = soft_reminder_block(192_000, 194_321);
        assert!(soft.starts_with(REMINDER_INJECTED_PREFIX));
        assert!(soft.contains("194321 token"));
        assert!(soft.contains("工作现场"));
        assert!(!soft.contains("已打断"));
        let hard = hard_reminder_block(320_000, 500_000, 322_000);
        assert!(hard.contains("320K · 硬提醒"));
        assert!(hard.contains("500K"));
        // 2026-09-24 必定压缩补足：H1 宣告 T1 线的新语义——强制开压缩窗口、
        // 两次窗口未产出 ⇒ 第三次机械截断（仅留工作现场、回查存档）。
        assert!(hard.contains("强制开压缩窗口"));
        assert!(hard.contains("仅保留工作现场"));
        assert!(hard.contains("按块回查存档"));
        assert!(hard.contains(crate::model_face::MODEL_FACE_DECLARATION));
        let notice = truncation_notice_block(3, 41_000, 460_000, "- 块#1 …", false, "");
        assert!(notice.contains("已截断 3 块"));
        // 0bh ⑯ 子项：判断句已删——机械层不再替模型断言「该不该继续」，
        // 只留状况陈述（工作现场未动）与回放指引。
        assert!(!notice.contains("任务无需中止"), "{notice}");
        assert!(
            notice.contains("工作现场（最近若干完整轮）与残段逐字未动"),
            "{notice}"
        );
        // 2026-09-24 必定压缩补足：截断告知块明确「需要前置上下文时回查存档」。
        assert!(notice.contains("需要前置上下文时请回查存档"), "{notice}");
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
            hard_reminder_block(320_000, 500_000, 320_000),
            truncation_notice_block(1, 1_000, 460_000, "- 块#1 …", false, ""),
            guard_truncation_notice_block(1, 1_000, 460_000, 700_000, "- 块#1 …", false),
            first_block_reminder_block(),
            compression_window_block(320_000),
            mandatory_compression_window_block(500_000, 1, 500_000),
            mandatory_compression_window_block(500_000, 2, 500_000),
            block_selection_unrecognized_notice("压缩块: 全部", "1-3"),
        ] {
            assert!(
                crate::prompt::is_injected_block_text(&block),
                "injected block not registered: {block}"
            );
        }
        // 两级询问语义：首问＝规则告知；第二次＝明示质量衰减与最后机会。
        let first = mandatory_compression_window_block(500_000, 1, 500_000);
        let second = mandatory_compression_window_block(500_000, 2, 500_000);
        assert!(first.contains("机械层不替你截断"), "{first}");
        assert!(!first.contains("第二次询问"), "{first}");
        assert!(second.contains("第二次询问"), "{second}");
        assert!(second.contains("上下文质量已严重衰减"), "{second}");
        assert!(second.contains("最后一次压缩窗口"), "{second}");
        assert!(second.contains("按块回查存档"), "{second}");
    }

    /// 0bs ④ 钉（2026-09-25）：**通知块不再内嵌整表**——分块索引单源＝窗口
    /// 尾部表（逐轮刷新）；H1／截断告知／必定窗口三个通知块不得含表体或
    /// 「说明行」，但须带单源指向行（同一「说明行」在一份请求里只出现一次）。
    #[test]
    fn notices_point_to_the_tail_table_instead_of_embedding_it() {
        let cases = [
            hard_reminder_block(320_000, 500_000, 322_000),
            truncation_notice_block(3, 41_000, 460_000, "- 块#1 …", false, ""),
            mandatory_compression_window_block(500_000, 1, 500_000),
            mandatory_compression_window_block(500_000, 2, 500_000),
        ];
        for text in &cases {
            assert!(!text.contains("[上下文分块表"), "{text}");
            assert!(!text.contains("说明: 工作现场"), "{text}");
            assert!(
                text.contains(crate::model_face::BLOCK_TABLE_POINTER_LINE),
                "{text}"
            );
        }
    }

    /// 0bs ⑥ 钉（2026-09-25）：**未落地回执**——F12：摘要被识别但未压缩任何
    /// 分块时模型侧不再静默；回执含未落地原因、可压区间与重投指引，且属注册
    /// 注入文本（不写回持久化会话）；空集合渲染「（无）」而非残句。
    #[test]
    fn summary_not_landed_receipt_is_informative_and_registered() {
        let notice = summary_not_landed_notice("指定区间在可压集合中无命中", "2-3");
        assert!(notice.starts_with(WINDOW_NOTICE_PREFIX));
        assert!(notice.contains("未压缩任何分块"), "{notice}");
        assert!(notice.contains("指定区间在可压集合中无命中"), "{notice}");
        assert!(notice.contains("2-3"), "{notice}");
        assert!(notice.contains("重投"), "{notice}");
        assert!(crate::prompt::is_injected_block_text(&notice));
        let empty = summary_not_landed_notice("当前没有「已闭合且仍为原文」的可压分块", "");
        assert!(empty.contains("（无）"), "{empty}");
    }

    /// 0bn R2 钉②（2026-09-24 复审补口，v8 §15）：**强制窗块自嵌摘要模板**
    /// ——模板（含「至少命中 2 个小节」识别门槛）必须在两级询问块文本内可见，
    /// 不再依赖任何早前注入的「（见上）」指称；否则 H1 块被移出／单轮暴涨被
    /// 最高档抑制／恢复会话不回放时模型从未见过格式 ⇒ 产出形状错判「未产出」
    /// ⇒ 直推升级/截断。
    #[test]
    fn mandatory_window_block_embeds_the_summary_template_self_contained() {
        for ask in [
            mandatory_compression_window_block(500_000, 1, 500_000),
            mandatory_compression_window_block(500_000, 2, 500_000),
        ] {
            assert!(ask.contains(MODEL_SUMMARY_PREFIX), "{ask}");
            assert!(ask.contains(MODEL_SUMMARY_END), "{ask}");
            assert!(ask.contains(MODEL_SUMMARY_BLOCK_LABEL), "{ask}");
            assert!(ask.contains("至少命中 2 个小节"), "{ask}");
            assert!(ask.contains("目标: …"), "{ask}");
            // 模板就地成立：不得残留依赖早前注入的指称。
            assert!(!ask.contains("（见上）"), "{ask}");
        }
    }

    #[test]
    fn summary_block_is_extracted_with_optional_block_selection() {
        let text =
            "先说明。\n[SEMANTIC_SUMMARY]\n压缩块: 2-4\n目标: x\n已完成: y\n[/SEMANTIC_SUMMARY]\n";
        let summary = extract_model_summary(text).expect("summary extracted");
        assert!(summary.starts_with(MODEL_SUMMARY_PREFIX));
        assert_eq!(
            parse_block_selection(&summary),
            BlockSelection::Specified(vec![2, 3, 4])
        );
        assert!(
            extract_model_summary("[SEMANTIC_SUMMARY]\n目标: 只有一个段\n[/SEMANTIC_SUMMARY]")
                .is_none()
        );
        assert_eq!(
            parse_block_selection("[SEMANTIC_SUMMARY]\n目标: x\n"),
            BlockSelection::NotSpecified
        );
    }

    /// 0bk S2 钉①（2026-09-24）：**带注解区间解析**——行内注解（中/英括号、
    /// 说明文字）与全角冒号不再使解析失败（0bi 轮 `压缩块: 1-62（全部已闭合
    /// 块；工作现场保留）` 曾整体解析失败 ⇒ 静默只压最旧一块）；**非法段仍
    /// 拒**（⇒ `Unrecognized` 如实回报，不静默缺省）。
    #[test]
    fn annotated_block_ranges_parse_leniently_and_illegal_ones_stay_rejected() {
        assert_eq!(
            parse_block_selection("压缩块: 1-62（全部已闭合块；工作现场保留）"),
            BlockSelection::Specified((1..=62).collect())
        );
        assert_eq!(
            parse_block_selection("压缩块: 1-4 (already closed)"),
            BlockSelection::Specified(vec![1, 2, 3, 4])
        );
        assert_eq!(
            parse_block_selection("压缩块：2, 6（已压过的不重复）"),
            BlockSelection::Specified(vec![2, 6])
        );
        // 0bm 复审补口（2026-09-24）：顿号是模型写多段区间的常见分隔，
        // 不识别会把尾段当注解静默截断。
        assert_eq!(
            parse_block_selection("压缩块: 1-4、6"),
            BlockSelection::Specified(vec![1, 2, 3, 4, 6])
        );
        // 非法区间（b < a）同样整条拒（0bm 复审补钉：此前无直接断言）。
        assert_eq!(
            parse_block_selection("压缩块: 5-3"),
            BlockSelection::Unrecognized("压缩块: 5-3".to_string())
        );
        assert_eq!(
            parse_block_selection("压缩块：7（工作现场）"),
            BlockSelection::Specified(vec![7])
        );
        assert_eq!(
            parse_block_selection("压缩块: 1-4, abc"),
            BlockSelection::Unrecognized("压缩块: 1-4, abc".to_string())
        );
        assert_eq!(
            parse_block_selection("压缩块: 全部"),
            BlockSelection::Unrecognized("压缩块: 全部".to_string())
        );
        assert_eq!(
            parse_block_selection("目标: x\n已完成: y"),
            BlockSelection::NotSpecified
        );
    }

    /// 0cz S2（2026-10-11，设计 §7）钉⑤：兜底联动一步权限提醒——单源句
    /// 被**全部**压缩/截断提醒块携带（改叙述、不加注入点；句面单源一次）。
    /// 0da S2（设计 §6.4）：句面四选择断言（压缩/清零/一键清理/清黑板）。
    #[test]
    fn context_permission_line_is_single_sourced_across_reminder_blocks() {
        let line = context_permission_line();
        assert!(
            line.contains("context_manage")
                && line.contains("mode=compress")
                && line.contains("mode=clear")
                && line.contains("mode=clear_all")
                && line.contains("op=clear")
                && line.contains("handover")
                && line.contains("blackboard_write"),
            "权限句必须点名四选择与交接前置：{line}"
        );
        let blocks = [
            soft_reminder_block(192_000, 200_000),
            hard_reminder_block(320_000, 500_000, 330_000),
            compression_window_block(320_000),
            mandatory_compression_window_block(500_000, 1, 510_000),
            mandatory_compression_window_block(500_000, 2, 510_000),
            truncation_notice_block(3, 40_000, 200_000, "- 回放行", false, ""),
            guard_truncation_notice_block(2, 30_000, 100_000, 700_000, "- 回放行", false),
        ];
        for (i, block) in blocks.iter().enumerate() {
            assert!(
                block.contains(line.as_str()),
                "提醒块 #{i} 必须携带单源权限句：{block}"
            );
        }
    }
}
