//! 动态上下文滑块 S1 修订批（v7，2026-09-15，[`CONTEXT_DYNAMIC_SLIDER_DESIGN`]
//! §3.4.1／§3.5.1）——「实际上下文刻度 ＋ 压缩分工（机械结构化轨 ／ 模型
//! 语义轨）」的非对话面实现：刻度状态、提醒与窗口文案、模型语义摘要块的
//! 机械识别、会话级水位键。
//!
//! **取代 0ae `attention_ladder`（D2 注意力阶梯）**：128K 打断式 / 160K
//! 提醒式 / 300·500·600·700K 软提醒 / 800K 硬提醒 / 920K 开窗七级 ＋
//! `rearm()` 整体**退役**（用户 2026-09-15 裁定：滑块把上传视图恒压在上限
//! H 内，旧阶梯每一级都成了不可达阈值）。取而代之：
//!
//! 1. **实际上下文刻度**（默认 500K / 900K，`ContextCompactConfig`
//!    `.context_scale_milestones`；测试缝隙可改，生产固定）：
//!    - **500K ＝ 纯提醒**（不打断、不开窗、不强制，模型可延后；文案给出
//!      自选压缩方法＝产出语义摘要块）；
//!    - **900K ＝ 必须压缩一次**（提醒 ＋ 开压缩窗口：窗口轮把「滑块之外
//!      的携带内容」连同滑块一并上传，由模型产出**语义摘要**）；
//!    - **上传越上限 ⇒ 降级**（审查修正批 2026-09-15）：窗口轮全量上传的
//!      估算越过 `window_upload_cap_tokens`（默认 1.10M）时**不开窗**（该轮
//!      请求必硬失败），改注入 [`window_skipped_over_cap_block`] ＋ 机械层
//!      同迭代强制压一次，事实经 anomaly `window_upload_over_cap` 如实落账；
//!    - **压不动 ⇒ 硬截留 ＋ 明确告知**（用户 2026-09-15 裁定，取代 v7 的
//!      「anomaly 停手／强制开窗」）：硬线之上压不下来时，机械层把「滑块
//!      （驻留带）之外」的内容全部移出（视图只剩前置＋固定指针＋当前滑块），
//!      并注入 [`compaction_failed_truncation_block`]（headline
//!      [`TRUNCATION_FAILURE_HEADLINE`]）由模型自行决定下一步——**不进 NoOp、
//!      不永久停手**，冷却过后新累积的滑块外轮次仍可再截留一次；
//!    - **窗口内超大结果指针化**（用户 2026-09-15 裁定第二条；先例＝
//!      OUTPUT-DEGENERATION-GUARD）：溢出体量位于当前窗口内（单结果过大）时，
//!      把超过 `action_ledger::OVERSIZED_TOOL_RESULT_CAP_TOKENS` 的工具结果
//!      正文换成「原文头部＋回读指针」（配对字段不动、幂等），使窗口内也能降线；
//!
//! **模型面措辞纪律（用户 2026-09-15 裁定）**：注入块与指针文案一律说
//! 「**当前上下文窗口**」（不说「滑块／驻留带」——那是内部分区术语，模型未受
//! 该词汇教学）；且截留告知**不报实际上下文读数**（本地存量无上限，读数对该
//! 动作无指导意义）。
//! 2. **首次真实驱逐**的一次性固化提醒（继承 D2 128K 梯的文案语义）；
//! 3. **压缩窗口（D3）**保留（≤3 轮无工具轮）；窗口语义在 `checkpoint` /
//!    `agent_loop`（`PendingCheckpoint::ModelCompression` /
//!    `finalize_model_compression_close`）。
//!
//! **量尺（本批最大实现陷阱，设计 §9-3）**：本模块所有判定吃**实际上下文
//! 估算**＝`estimate_messages_tokens(&messages)`（chars/2，全量会话）——**不是**
//! 视图估算、**不是** provider 实测 `prompt_tokens`。滑块下三者在数值上彻底
//! 分离（视图恒 ≈H，实际上下文可长期增长），故提醒必须挂前者——这也正是
//! D2 旧量尺（视图）必然失灵的原因。
//!
//! **会话级水位（v7，DP-16）**：每级**每会话**一次——已提醒键随会话状态
//! 持久化（先例 `StoredConversation.fatigue_tiers_notified`），prompt 起始
//! 注入 loop、fire 时回写；取代 S1 的 per-run 语义（长单对话里同一刻度
//! 每个 prompt 重发一次＝噪声）。
//!
//! 参数与读数纪律：**参数不进文案**（改 env 即一次性前缀失效；配置固定性
//! 优先于精确表述，同设计 §3.1）；**读数必须进文案**（用户裁定）。
//!
//! 注入文本纪律：提醒块以 [`REMINDER_INJECTED_PREFIX`] 开头、窗口机械提示以
//! [`WINDOW_NOTICE_PREFIX`] 开头，二者均注册进 `prompt::is_injected_block_text`
//! ⇒ 绝不写回持久化会话（机械注入文本，固定文本不是模型输出）。

use std::collections::HashSet;

/// 提醒块注入前缀（注册进 `prompt::is_injected_block_text`，绝不持久化）。
pub const REMINDER_INJECTED_PREFIX: &str = "[CONTEXT_SCALE";

/// 压缩窗口内的机械提示前缀（注册进 `prompt::is_injected_block_text`）。
/// 0AE 遗留缺口（S1 修订批补）：窗口轮的「仅 blackboard_write 可执行」提示
/// 与「窗口剩余 N 轮」提示在修复前**未注册** ⇒ 会被写回持久化会话。
pub const WINDOW_NOTICE_PREFIX: &str = "[模型参与压缩";

/// **模型语义摘要块的机械可识别前缀**（v7 压缩分工的载体，DP-14）。
///
/// 模型在回复中输出该块 ⇒ 机械层在下一个 loop-top 安全间隙执行一次
/// 「语义摘要替换被压区」（`mode=model_summary`）。零新增工具面（8 工具面
/// 冻结不动）——识别面是**回复文本**，不是工具。
pub const MODEL_SUMMARY_PREFIX: &str = "[SEMANTIC_SUMMARY";

/// 语义摘要块结束标记（缺失时按「到文本末尾」容错截取）。
pub const MODEL_SUMMARY_END: &str = "[/SEMANTIC_SUMMARY]";

/// 语义摘要块的六段结构（设计 §3.4.1「目标／已完成／关键决策／未决问题／
/// 下一步／关键文件」）；机械识别要求至少命中 [`MODEL_SUMMARY_MIN_SECTIONS`]
/// 段，避免正文里偶然出现前缀即被误判为摘要块。
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

/// D3：模型实施压缩的有限轮数（DP-7：≤3 轮；超轮未完成 ⇒ 机械层按既定
/// 兜底收口，`model_participated` 如实落账）。
pub const COMPRESSION_WINDOW_ROUNDS: u32 = 3;

/// 950K 最后防线的冷却（模型轮）：一次强制之后至少隔这么多轮才允许再强制
/// （v7 V4「once ＋ 冷却」；压不动则落 anomaly 并停手，不逐轮重压）。
pub const HARD_CONTEXT_COOLDOWN_ROUNDS: u32 = 4;

/// 一次里程碑提醒（刻度 + 机械读数位 + 注入块文案）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextScaleFire {
    /// 越线的里程碑刻度（token，实际上下文估算口径）。
    pub milestone_tokens: u64,
    /// `mechanical_audit_update` 的 key 尾缀（`500k` / `900k`）。
    pub key: String,
    /// 该刻度是否**开压缩窗口**（v7：最后一档＝必须压缩一次；其余＝纯提醒）。
    pub opens_window: bool,
    pub block: String,
}

/// 里程碑状态（**会话级水位**，DP-16；每级恰好一次、**不 rearm**——机械
/// 压缩不重置提醒面，否则长单对话里同一级会反复提醒）。
#[derive(Debug, Clone, Default)]
pub struct ContextScaleState {
    fired: HashSet<u64>,
}

impl ContextScaleState {
    pub fn new() -> Self {
        Self::default()
    }

    /// 从会话侧车水位键（`["500k", "900k"]`）恢复（跨 prompt 延续、新会话
    /// 从零开始、恢复会话不重发——与黑板同族语义）。
    pub fn from_notified_keys(keys: &[String]) -> Self {
        let fired = keys.iter().filter_map(|k| parse_key(k)).collect();
        Self { fired }
    }

    /// 当前水位键（升序、去重；随侧车持久化）。
    pub fn notified_keys(&self) -> Vec<String> {
        let mut out: Vec<String> = self
            .fired
            .iter()
            .map(|m| format!("{}k", m / 1000))
            .collect();
        out.sort();
        out
    }

    /// 当前实际上下文读数下新越线的里程碑（每级恰好一次；重复查询零返回）。
    ///
    /// `milestones` 升序（默认 `[500_000, 900_000]`，见
    /// `ContextCompactConfig::context_scale_milestones`）：**最后一档**
    /// 开压缩窗口（900K ＝ 必须压缩一次），之前的档位是**纯提醒**
    /// （500K ＝ 可延后，不打断、不开窗）。
    pub fn due(&mut self, actual_tokens: u64, milestones: &[u64]) -> Vec<ContextScaleFire> {
        let mut out = Vec::new();
        let forced_window_from = milestones.last().copied();
        for &milestone in milestones {
            if actual_tokens < milestone || !self.fired.insert(milestone) {
                continue;
            }
            let opens_window = forced_window_from == Some(milestone);
            out.push(ContextScaleFire {
                milestone_tokens: milestone,
                key: format!("{}k", milestone / 1000),
                opens_window,
                block: if opens_window {
                    reminder_with_window_block(milestone, actual_tokens)
                } else {
                    reminder_block(milestone, actual_tokens)
                },
            });
        }
        out
    }

    /// 该刻度是否已提醒过（判据/A-B 读数口用）。
    pub fn has_fired(&self, milestone_tokens: u64) -> bool {
        self.fired.contains(&milestone_tokens)
    }
}

/// 水位键（`"500k"`）→ 刻度（`500_000`）；非法键忽略（向后兼容/防脏数据）。
fn parse_key(key: &str) -> Option<u64> {
    key.trim()
        .strip_suffix('k')
        .or_else(|| key.trim().strip_suffix('K'))
        .and_then(|k| k.trim().parse::<u64>().ok())
        .map(|k| k * 1000)
}

/// 机械读数渲染（文案必含实际读数，且同时给 M 记法与原始 token 数）。
fn reading(tokens: u64) -> String {
    format!(
        "≈{:.2}M token（{tokens} token）",
        tokens as f64 / 1_000_000.0
    )
}

/// 语义摘要块的输出说明（500K 纯提醒与 900K 窗口块共用——单一来源，
/// 避免两处文案漂移）。
fn summary_block_guide() -> String {
    format!(
        "在回复中输出一个语义摘要块（机械层据此把**当前上下文窗口之外**的旧内容替换成该摘要，\
         原文仍留在本地档案、可按定位指针回读；窗口内的工作现场不动）：\n\
         {MODEL_SUMMARY_PREFIX}]\n\
         目标: …\n已完成: …\n关键决策: …\n未决问题: …\n下一步: …\n关键文件: …\n\
         {MODEL_SUMMARY_END}"
    )
}

/// **500K ＝ 纯提醒**（v7：不打断、不开窗、不强制，模型可延后）。
pub fn reminder_block(milestone_tokens: u64, actual_tokens: u64) -> String {
    let k = milestone_tokens / 1000;
    let reading = reading(actual_tokens);
    format!(
        "{REMINDER_INJECTED_PREFIX} {k}K] 当前实际上下文 {reading}。\
         发送给你的内容仍被**当前上下文窗口**限制在上限内（更早轮次的逐字本体只在本地档案与 journal），\
         但**压缩交给你自选、可延后**：若判断早期内容已可清理，{}\
         \n延后不会打断你（窗口随新内容继续推进）；到最高刻度会强制要求一次。",
        summary_block_guide()
    )
}

/// **最高刻度 ＝ 必须压缩一次**（v7：提醒 ＋ 开压缩窗口）。
pub fn escalated_reminder_block(milestone_tokens: u64, actual_tokens: u64) -> String {
    let k = milestone_tokens / 1000;
    let reading = reading(actual_tokens);
    format!(
        "{REMINDER_INJECTED_PREFIX} {k}K · 必须压缩] 当前实际上下文 {reading}，已到强制线。\
         本轮起开压缩窗口（≤{COMPRESSION_WINDOW_ROUNDS} 轮）：窗口内已把**当前上下文窗口之外的携带内容**\
         连同窗口内内容一并上传给你，{}\
         \n窗口内未完成 ⇒ 机械层按既定兜底收口并如实落账（`model_participated=false`）。",
        summary_block_guide()
    )
}

/// 首次真实驱逐的一次性固化提醒（A4，0ae D2 128K 梯文案语义复用）。
pub fn first_fold_reminder_block() -> String {
    format!(
        "{REMINDER_INJECTED_PREFIX} 首次驱逐] 上传视图已按常驻滑窗移出最旧若干**完整轮**\
         （视图＝前置＋固定指针＋当前上下文窗口；被移出轮次的逐字本体仍在本地档案与 journal，\
         模型面只剩台账摘要行）。请把接线结论/关键读数固化到黑板\
         （blackboard_write section=plan|notes）——黑板不属于上下文窗口，跨驱逐不失效。"
    )
}

/// 最高刻度 ＋ 压缩窗口任务块的合并注入（一个越线事件＝一次提醒 ＋ 一次
/// 开窗，避免同一刻度在视图里留下两块重复文本）。
pub fn reminder_with_window_block(milestone_tokens: u64, actual_tokens: u64) -> String {
    format!(
        "{}\n{}",
        escalated_reminder_block(milestone_tokens, actual_tokens),
        compression_window_block(milestone_tokens)
    )
}

/// 审查修正批（2026-09-15，审查 P2⑤「900K 窗口无上限守卫」）：**窗口上传
/// 越上限 ⇒ 降级块**——最高刻度必须压缩一次，但窗口轮要上传 `messages`
/// 全量，若该上传估算越过 [`crate::compact::ContextCompactConfig::window_upload_cap_tokens`]
/// （默认 1.10M，对应 provider 1M 窗口 ÷ 实测换算 0.77 ＋ 余量），开窗只会
/// 让该轮请求硬失败。故**不开窗**：如实告知读数与降级事实、给出语义摘要块
/// 方法（自选压缩通路仍在），机械层同步走一次强制压缩兜底；事实另经
/// 同一条 fire 的 `mechanical_audit_update` 落账（`key=context_scale:<档位>`，
/// `summary` 带 `window_skipped_over_cap=true`，`anomaly=window_upload_over_cap`）。
/// 前缀同 [`REMINDER_INJECTED_PREFIX`] ⇒ 绝不写回持久化会话。
pub fn window_skipped_over_cap_block(
    milestone_tokens: u64,
    actual_tokens: u64,
    cap_tokens: u64,
) -> String {
    let k = milestone_tokens / 1000;
    let reading = reading(actual_tokens);
    format!(
        "{REMINDER_INJECTED_PREFIX} {k}K · 窗口降级] 当前实际上下文 {reading}，已超过压缩窗口的\
         上传上限 ≈{} token——窗口轮要把「当前上下文窗口之外的携带内容」连同窗口内内容一并上传，\
         越线即超出单请求上限（硬失败），故**本轮不开压缩窗口**（设计「过大时分段」为备选、尚未实现）。\
         发送给你的视图照旧被上下文窗口压在上限内，请求安全；机械层本轮就地强制压一次可指针化的\
         结构化内容（无可压内容则如实落账）。语义层（任务线／决策／未决项）仍只能由你压缩：{}",
        reading_only(cap_tokens),
        summary_block_guide()
    )
}

/// 上限读数的轻量渲染（只给 M 记法，避免与**实际读数**混淆——实际读数由
/// [`reading`] 给全）。
fn reading_only(tokens: u64) -> String {
    format!("{:.2}M", tokens as f64 / 1_000_000.0)
}

/// **压缩失败 ⇒ 硬截留**（用户 2026-09-15 裁定，取代 v7「压不动 ⇒ anomaly
/// 停手／只开窗」）：950K 最后防线压不动时**不进 NoOp、不永久停手**——机械层
/// 强硬把「**滑块（驻留带）之外**」的内容全部移出模型上下文（视图只留
/// 前置＋固定指针＋当前滑块；被移出的轮次行入台账、逐字原文留 run journal 与
/// 本地档案），并把该事实**明确告知模型**，由模型自行决定下一步。
pub const TRUNCATION_FAILURE_HEADLINE: &str = "上一轮上下文压缩失败，已机械截留";

/// 压缩失败告知块（注入文本；headline ＝ [`TRUNCATION_FAILURE_HEADLINE`]）。
///
/// 事实三项（按实况任意组合，全部如实）：
/// - `dropped_rounds > 0`：**窗口之外**确有轮次被截留（行入台账、原文留 journal）；
/// - `pointerized_results > 0`：**窗口之内**的超大工具结果**正文已换成指针**
///   （用户 2026-09-15 裁定第二条；先例＝OUTPUT-DEGENERATION-GUARD）；
/// - 两项皆 0：窗口外已无可截留、窗口内也无可指针化的超大结果 ⇒ 机械层到此
///   为止（溢出体量位于前置/窗口框架内）。
///
/// **不再报实际上下文读数**（用户 2026-09-15 裁定：本地存量没有上限，读数对
/// 该动作无指导意义，只留「已截留」的事实与回读指针）。
/// 语义＝「让模型自己决定下一步、任务还能继续」：给出可执行的低成本选项
/// （按指针回读／避免重复整读超大文件／把必须长期保留的结论固化到黑板），
/// 并显式声明**任务无需中止**。
pub fn compaction_failed_truncation_block(
    dropped_rounds: u32,
    pointerized_results: u32,
    ledger_hint: Option<&str>,
) -> String {
    let mut facts: Vec<String> = Vec::new();
    if dropped_rounds > 0 {
        let ledger = ledger_hint
            .map(|p| format!("台账摘要行见 {p}（按 `轮次`/工具名 grep）"))
            .unwrap_or_else(|| "台账摘要行见会话外挂台账文件".to_string());
        facts.push(format!(
            "**当前上下文窗口之外**的 {dropped_rounds} 轮已移出模型上下文（{ledger}；\
             逐字原文在 run journal 与本地档案，可按定位指针回读）"
        ));
    }
    if pointerized_results > 0 {
        facts.push(format!(
            "**窗口之内**的 {pointerized_results} 个超大工具结果**正文已换成指针**\
             （原文仍在本地档案、逐字可回读：按结果里的 `call_id=` 检索 run journal，\
             或用 read_file 分页读原文件）"
        ));
    }
    let detail = if facts.is_empty() {
        "窗口之外已无可截留内容、窗口之内也没有值得指针化的超大结果——溢出体量位于\
         前置/窗口框架内，机械层到此为止"
            .to_string()
    } else {
        facts.join("；")
    };
    format!(
        "{REMINDER_INJECTED_PREFIX} 压缩失败已截留] {TRUNCATION_FAILURE_HEADLINE}：{detail}。\
         当前发送给你的视图只保留「前置＋固定指针＋当前上下文窗口」。\
         下一步由你决定：需要原文时按上面的指针回读（read_file 分页）、避免重复整读超大文件、\
         把必须长期保留的结论固化到黑板（blackboard_write section=plan|notes）。**任务无需中止**。"
    )
}

/// D3：压缩窗口任务块（打断全部动作后的无工具轮注入）——产出语义摘要块
/// （可选：另把必须留存的结论固化到黑板）。量尺＝实际上下文里程碑。
///
/// v7 更正：不再要求模型「标注可弃范围」——标注在 S1 实现里从未被消费
/// （审查发现 P1⑤），语义侧改由**摘要块**承载（机械层识别后替换被压区）。
pub fn compression_window_block(milestone_tokens: u64) -> String {
    let k = milestone_tokens / 1000;
    format!(
        "{WINDOW_NOTICE_PREFIX} · 窗口 · 实际上下文 {k}K] 已打断全部动作。请在窗口内完成：\n\
         1. 产出语义摘要块（见上）——机械层用它替换当前上下文窗口之外的旧内容；\n\
         2. 若有关键结论需要跨压缩长期留存，一并固化到黑板\
         （blackboard_write section=plan|notes；黑板不受上下文窗口影响）。\n\
         窗口结束仍未产出摘要块 ⇒ 机械层按既定兜底收口（结构化轨照常压，\
         语义层未压如实落账 `model_participated=false`）。"
    )
}

/// **950K 最后防线**注入块（v7 的原「强制开窗要求模型压缩」文案）。
///
/// **已退役（用户 2026-09-15 裁定）**：最后防线的失败处置改为**硬截留 ＋
/// 明确告知**（[`compaction_failed_truncation_block`]），不再从该档强制开窗，
/// 故本函数**生产零调用**；保留只为文案留档与将来可能的复用（同
/// `mechanical_audit::KIND_ATTENTION_LADDER` 的退役留值纪律）。
#[allow(dead_code)]
pub fn last_resort_block(actual_tokens: u64) -> String {
    let reading = reading(actual_tokens);
    format!(
        "{REMINDER_INJECTED_PREFIX} 最后防线] 当前实际上下文 {reading}，已越过硬兜底线。\
         机械层已压掉可指针化的结构化内容（工具／命令／结果）；**语义层（任务线／决策／\
         未决项）只能由你压缩**——请立即{}\
         \n语义层始终不压时，实际上下文不再下降（v7 显式接受的责任划分；最终防线＝\
         本地资源门）。",
        summary_block_guide()
    )
}

/// 窗口内模型未产出摘要块时的机械提示（单空格，0AE-C13 文案纪律）。
pub fn window_remaining_notice(rounds_left: u32) -> String {
    format!(
        "{WINDOW_NOTICE_PREFIX}] 窗口剩余 {rounds_left} 轮：尚未检测到语义摘要块。\
         请输出 `{MODEL_SUMMARY_PREFIX}] … {MODEL_SUMMARY_END}`（或把必要结论写入黑板）；\
         窗口结束即执行机械兜底收口。"
    )
}

/// 窗口内非白名单动作被丢弃时的机械提示（只报事实、不带建议）。
pub fn window_dropped_calls_notice(dropped: usize) -> String {
    format!(
        "{WINDOW_NOTICE_PREFIX}] 窗口内仅 blackboard_write 可执行，本轮其余动作已跳过（{dropped} 个）"
    )
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

#[cfg(test)]
mod tests {
    use super::*;

    const DEFAULT_MILESTONES: [u64; 2] = [500_000, 900_000];

    #[test]
    fn milestones_fire_exactly_once_each() {
        let mut state = ContextScaleState::new();
        assert!(
            state.due(499_999, &DEFAULT_MILESTONES).is_empty(),
            "未越线零返回"
        );
        let fires = state.due(500_001, &DEFAULT_MILESTONES);
        assert_eq!(fires.len(), 1);
        assert_eq!(fires[0].milestone_tokens, 500_000);
        assert_eq!(fires[0].key, "500k");
        assert!(!fires[0].opens_window, "500K ＝ 纯提醒（v7 V2）");
        assert!(
            state.due(500_001, &DEFAULT_MILESTONES).is_empty(),
            "同位重查不重复"
        );
        assert!(
            state.due(899_999, &DEFAULT_MILESTONES).is_empty(),
            "900K 未到"
        );
        let fires = state.due(950_000, &DEFAULT_MILESTONES);
        assert_eq!(fires.len(), 1, "900K 单级触发");
        assert_eq!(fires[0].key, "900k");
        assert!(fires[0].opens_window, "900K ＝ 必须压缩一次（v7 V3）");
        assert!(state.has_fired(500_000) && state.has_fired(900_000));
        assert_eq!(state.notified_keys(), vec!["500k", "900k"]);
    }

    #[test]
    fn both_milestones_fire_together_on_a_big_jump() {
        let mut state = ContextScaleState::new();
        let fires = state.due(1_200_000, &DEFAULT_MILESTONES);
        let keys: Vec<&str> = fires.iter().map(|f| f.key.as_str()).collect();
        assert_eq!(keys, vec!["500k", "900k"], "一次性越两级须两级都提醒");
        assert!(fires[1].opens_window, "最高档开窗");
    }

    /// DP-16：会话级水位——已提醒键随侧车恢复，跨 prompt 不重发。
    #[test]
    fn session_level_watermark_suppresses_already_notified_milestones() {
        let mut state =
            ContextScaleState::from_notified_keys(&["500k".to_string(), "900k".to_string()]);
        assert!(
            state.due(1_500_000, &DEFAULT_MILESTONES).is_empty(),
            "会话级不重发"
        );
        let mut partial = ContextScaleState::from_notified_keys(&["500k".to_string()]);
        let fires = partial.due(1_500_000, &DEFAULT_MILESTONES);
        assert_eq!(fires.len(), 1);
        assert_eq!(fires[0].key, "900k", "只补未提醒的档");
        // 脏键忽略（不 panic、不虚构刻度）。
        assert!(
            ContextScaleState::from_notified_keys(&["bogus".to_string()])
                .notified_keys()
                .is_empty()
        );
    }

    #[test]
    fn reminder_blocks_carry_the_actual_reading() {
        let block = reminder_block(500_000, 512_345);
        assert!(block.starts_with(REMINDER_INJECTED_PREFIX));
        assert!(block.contains("512345 token"), "实际读数入文案：{block}");
        assert!(
            block.contains(MODEL_SUMMARY_PREFIX),
            "500K 给出自选压缩方法：{block}"
        );
        assert!(block.contains("可延后"), "500K 允许延后：{block}");
        let escalated = escalated_reminder_block(900_000, 901_000);
        assert!(escalated.contains("必须压缩"), "900K 升级语气：{escalated}");
        let last = last_resort_block(951_000);
        assert!(
            last.contains("最后防线") && last.contains("951000 token"),
            "{last}"
        );
    }

    #[test]
    fn first_fold_reminder_reuses_the_d2_wording_and_is_injected_text() {
        let block = first_fold_reminder_block();
        assert!(block.starts_with(REMINDER_INJECTED_PREFIX));
        assert!(block.contains("blackboard_write section=plan|notes"));
        assert!(crate::prompt::is_injected_block_text(&block));
    }

    #[test]
    fn compression_window_block_carries_the_scale_and_the_summary_task() {
        let block = compression_window_block(900_000);
        assert!(block.contains("实际上下文 900K"));
        assert!(block.contains("语义摘要块"));
        assert!(block.contains("blackboard_write"));
        assert!(block.contains("model_participated"));
        assert_eq!(COMPRESSION_WINDOW_ROUNDS, 3);
    }

    #[test]
    fn window_notices_are_registered_as_injected_text() {
        for block in [
            window_remaining_notice(2),
            window_dropped_calls_notice(1),
            compression_window_block(900_000),
            reminder_with_window_block(900_000, 900_500),
            window_skipped_over_cap_block(900_000, 1_300_000, 1_100_000),
        ] {
            assert!(
                crate::prompt::is_injected_block_text(&block),
                "窗口/提醒文案必须注册为注入块: {block}"
            );
        }
    }

    /// 审查修正批（2026-09-15，审查 P2⑤）：窗口上传越上限的降级块——带实际
    /// 读数与上限读数、明示「不开窗」与分段未实现、仍给出自选压缩方法。
    #[test]
    fn over_cap_block_reports_the_degradation_and_keeps_the_self_selected_path() {
        let block = window_skipped_over_cap_block(900_000, 1_312_400, 1_100_000);
        assert!(block.starts_with(REMINDER_INJECTED_PREFIX));
        assert!(block.contains("窗口降级"), "{block}");
        assert!(block.contains("1312400 token"), "实际读数必给全: {block}");
        assert!(block.contains("1.10M"), "上限读数: {block}");
        assert!(block.contains("本轮不开压缩窗口"), "{block}");
        assert!(block.contains("尚未实现"), "分段未实现须如实: {block}");
        assert!(
            block.contains(MODEL_SUMMARY_PREFIX),
            "自选通路仍在: {block}"
        );
        assert!(
            !block.contains("窗口内已把"),
            "降级块不得复用窗口轮的「已上传待压区」措辞: {block}"
        );
    }

    /// 用户 2026-09-15 裁定：压缩失败 ⇒ 硬截留 ＋ 明确告知（不是 NoOp／停手）。
    /// 事实三项按实况组合；**不再报实际上下文读数**（本地存量无上限）；
    /// 模型面措辞用「当前上下文窗口」（不再说「滑块」）。
    #[test]
    fn truncation_failure_block_states_the_facts_without_a_capacity_reading() {
        // ① 只有窗口外轮次被截留。
        let rounds_only = compaction_failed_truncation_block(3, 0, Some(".gsa/ledger/current.md"));
        assert!(rounds_only.starts_with(REMINDER_INJECTED_PREFIX));
        assert!(rounds_only.contains(TRUNCATION_FAILURE_HEADLINE));
        assert!(rounds_only.contains("当前上下文窗口之外"), "{rounds_only}");
        assert!(rounds_only.contains("3 轮已移出"), "{rounds_only}");
        assert!(rounds_only.contains(".gsa/ledger/current.md"));
        assert!(rounds_only.contains("下一步由你决定"));
        assert!(rounds_only.contains("任务无需中止"));
        assert!(
            !rounds_only.contains("token"),
            "不得再报容量读数: {rounds_only}"
        );
        assert!(
            !rounds_only.contains("滑块"),
            "模型面不出现「滑块」: {rounds_only}"
        );
        assert!(crate::prompt::is_injected_block_text(&rounds_only));

        // ② 只有窗口内超大结果被指针化（第二条裁定）。
        let pointer_only = compaction_failed_truncation_block(0, 2, None);
        assert!(pointer_only.contains(TRUNCATION_FAILURE_HEADLINE));
        assert!(pointer_only.contains("2 个超大工具结果"), "{pointer_only}");
        assert!(pointer_only.contains("正文已换成指针"), "{pointer_only}");
        assert!(pointer_only.contains("call_id="), "{pointer_only}");
        assert!(
            !pointer_only.contains("轮已移出"),
            "无轮次截留不得虚报: {pointer_only}"
        );

        // ③ 两者皆无 ⇒ 如实说明机械层到此为止。
        let nothing = compaction_failed_truncation_block(0, 0, None);
        assert!(nothing.contains("已无可截留内容"), "{nothing}");
        assert!(nothing.contains("机械层到此为止"), "{nothing}");
        assert!(!nothing.contains("轮已移出"), "{nothing}");
        assert!(!nothing.contains("超大工具结果"), "{nothing}");
        assert!(crate::prompt::is_injected_block_text(&nothing));
    }

    #[test]
    fn model_summary_block_is_extracted_from_the_reply_text() {
        let reply = "先说明一句。\n[SEMANTIC_SUMMARY]\n目标: 修 A\n已完成: B\n\
                     关键决策: C\n未决问题: 无\n下一步: D\n关键文件: e.rs\n[/SEMANTIC_SUMMARY]\n收尾一句";
        let block = extract_model_summary(reply).expect("摘要块应被识别");
        assert!(block.starts_with(MODEL_SUMMARY_PREFIX));
        assert!(block.ends_with(MODEL_SUMMARY_END));
        assert!(!block.contains("收尾一句"), "块外文本不入摘要: {block}");
        // 结束标记缺失 → 取到文本末尾（容错）。
        let unterminated = "[SEMANTIC_SUMMARY]\n目标: x\n已完成: y\n";
        assert!(extract_model_summary(unterminated).is_some());
        // 段命中不足 / 无前缀 → 不识别（防误判）。
        assert!(extract_model_summary("[SEMANTIC_SUMMARY]\n随便一句\n").is_none());
        assert!(extract_model_summary("目标: x\n已完成: y\n").is_none());
        // 多块取最后一块（窗口内可能先叙述后产出）。
        let twice = format!("{unterminated}...{reply}");
        let picked = extract_model_summary(&twice).unwrap();
        assert!(picked.contains("关键决策"), "{picked}");
    }
}
