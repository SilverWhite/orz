//! 滑块上下文 v8（2026-09-16 勘误批）——**模型面投影层**、**分块表**与
//! **按块回放面**。设计权威：[`CONTEXT_SLIDER_V8_DESIGN_2026-09-16`]。
//!
//! ## 两个面（设计 §1）
//!
//! - **本地面**（Local）＝`messages` 逐字全量：v8 起**任何**机械／语义压缩
//!   都**不再 `drain` 会话本体**（v7 的 `messages.drain` 覆盖随之退役）。
//!   压缩只往会话里**追加**机械 marker（`[前文上下文已压缩 …]`，restore-
//!   retained），逐字内容一条不删——这是「可按块回放」与「会话归档随黑板
//!   一起打包、无实际上限」的物理前提（设计 §6 §7、不变量 I3）。
//! - **模型面**（Model）＝真正发往模型商的上下文＝
//!   `前置 ＋ 固定指针 ＋ 各分块（原文｜机械摘要行）＋ 主滑块 x ＋ 尾部 ＋
//!   D4 机械段 ＋ 分块表 ＋ 结束自述通道行（0bs ①，2026-09-25）`。
//!
//!   **常驻头自首个轮次即在（0bz S3，2026-09-28）**：固定指针的注入时点
//!   从「首个分块形成」提前到「首个轮次成形」——旧口径使开窗轮 face 前部
//!   整体重排（自发塌陷族机理：`RUN-CLI-6ab99969` r40＝135,754 tk 整窗
//!   miss）。注入点＝`round_ranges[0].0`，与开窗后 `preamble_end` 恒同值 ⇒
//!   开窗转换只在尾部追加，前缀字节稳定（I6 从「两次压缩之间」延伸到
//!   「会话全程」）。头部内容零改，只动时点。
//!
//!   **D4 机械段落尾部（0bz S3′，2026-10-04）**：D4 是逐 epoch 易变渲染
//!   （自编辑清单随编辑更新），原居头部（marker 之前）使每次重渲重价其后
//!   整段前缀——recli 四跑离线对账实证 9/9 压缩后 +2 请求在 D4 槽全前缀
//!   重价（「第 2 针」未消机理）。落尾部（块表之前）后重渲只重价尾部自身；
//!   内容零改，只动位置。
//!
//!   **分块表落尾部（2026-09-16 实现批，前缀纪律优先）**：表里末块那一行每
//!   新增一轮就变（区间／估算／计数都在长）⇒ 若像 v7 那样挂在固定指针之后
//!   （模型面第 2 条），每轮都会在表处打断前缀、把其后的主滑块整体推向缓存
//!   miss。表改为**尾部追加**后，两次压缩/截断之间前缀字节稳定（不变量 I6
//!   与设计 §9 的成本口径同时成立），表本身的变化只落在尾部。
//!
//! ## 主滑块与分块（设计 §2）
//!
//! - **主滑块 x**＝最近 x K 估算的**连续完整轮**（整轮对齐；与 v7 同一把
//!   `bridge_cut` 尺）——**永不被压缩、永不被截断**（不变量 I1）。
//! - **分块 y**＝主滑块**以外**的内容按 y K 估算切块，**仅逻辑分块、不流出
//!   模型面**（不变量 I2）；块的编号**锚在会话起点**，故跨轮次增长稳定
//!   （去重键＝块号，设计 §12「同一块在同一会话内不重复计数」）。
//! - **只有「已闭合」的块可被压缩**（其后已有更新块 ⇒ 不再增长）；仍在增长
//!   的末块留在原文面，避免摘要与正文漂移。
//!
//! ## 回放面（设计 §6，形态①：零新工具）
//!
//! 压缩/截断一个块时，机械层把该块逐字原文落盘到
//! `{cwd}/.gsa/compaction/blocks/<session8>-b<k>.md`（同
//! `.gsa/session/terminal/<id>.log` 先例），marker 与该块的分块表行都带
//! 「完整内容见〈路径〉，用 `read_file` offset/limit 分页」指针 ⇒ 模型用
//! 既有读工具即可按块拉回，**不动 8 工具面**。块头附陈旧性标注（内容截至
//! 轮次 N；文件可能已变更，编辑前须新鲜读取）。
//!
//! 注入文本纪律：分块表以 [`BLOCK_TABLE_PREFIX`] 开头并注册进
//! `prompt::is_injected_block_text` ⇒ 绝不写回持久化会话；压缩/截断 marker
//! 仍用 `prompt::CONTEXT_COMPRESSED_PREFIX` 前缀（既在注入文本注册表内、
//! 又是 restore-retained），故它**会**随侧车留存（恢复后分块状态由此重建、
//! 无需新的侧车字段）。

use crate::gateway::model::{Message, ToolCall};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// 分块表注入前缀（注册进 `prompt::is_injected_block_text`，绝不持久化）。
pub const BLOCK_TABLE_PREFIX: &str = "[上下文分块表";

/// 0cz S2（2026-10-11，设计 §5）：模型主动清零 marker 前缀（注册进
/// `prompt::is_injected_block_text`〔绝不持久化〕与
/// `prompt::is_restore_retained_block`〔恢复后边界由此重建——会话是唯一
/// 真源、零新侧车字段〕）。
pub const CLEAR_MARKER_PREFIX: &str = "[前文上下文已清零";

/// 0bs ④（2026-09-25）：分块表的**单源指向行**——通知块（H1 硬提醒／T1 截断
/// 告知／必定压缩窗口）不再内嵌整表，只给这一行；表本体在窗口**尾部**逐轮
/// 刷新（同一「说明行」在一份请求里只出现一次）。
pub const BLOCK_TABLE_POINTER_LINE: &str =
    "分块表见**窗口尾部**（逐轮刷新；压缩/回放均按块号指定）。";

/// marker 机器行标签：块号区间（供 `face_markers` 反解分块状态）。
pub const BLOCK_MARKER_RANGE_LABEL: &str = "已处理分块: ";

/// v8 分块压缩 marker 版本号（复用 `[前文上下文已压缩` 前缀）。
pub const BLOCK_MARKER_COMPRESSED_VERSION: &str = "v0.4-分块压缩";

/// v8 分块截断 marker 版本号（T1 硬截断；同上复用前缀）。
pub const BLOCK_MARKER_TRUNCATED_VERSION: &str = "v0.4-分块截断";

/// 0cz S2（2026-10-11，设计 §5）：模型主动清零 marker 版本号（自有前缀
/// `[前文上下文已清零`，注册进 `prompt::is_injected_block_text`（绝不持久化）
/// 与 `is_restore_retained_block`（恢复后边界由此重建））。
pub const CLEAR_MARKER_VERSION: &str = "v0.1-清零";

/// 清零 marker 机器行标签：边界轮号（1 基；轮 < R 的全部内容移出窗口）。
pub const CLEAR_MARKER_BOUNDARY_LABEL: &str = "清零边界: 第 ";

/// 单个 marker 允许声明的最大块数（防手写/脏数据构造超大集合）。
const MAX_MARKER_BLOCKS: usize = 512;

/// 块状态（**幂等、以块号为键**；设计 §12「块表状态幂等」）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BlockState {
    /// 原文留在模型面内累积（未压缩、未截断）。
    #[default]
    Live,
    /// 已被压缩：模型面里由机械摘要行／语义摘要替代（可按块回放）。
    Compressed,
    /// 已被 T1 硬截断：移出模型面（可按块回放）。
    Truncated,
    /// 已被模型主动清零（0cz S2）：随边界轮整体移出窗口（可按块回放）。
    /// 表态由 `FaceMarkers::is_round_cleared` 推导（清零以轮为界，不以块号
    /// 记录——边界后新建块不受影响）。
    Cleared,
}

/// 一个主滑块之外的逻辑分块。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextBlock {
    /// 1 基块号（锚在会话起点 ⇒ 稳定；回放去重键）。
    pub number: u32,
    /// 会话内 0 基轮序号（含）。
    pub first_round: usize,
    /// 会话内 0 基轮序号（含）。
    pub last_round: usize,
    /// 块首消息下标（＝首轮声明下标）。
    pub msg_start: usize,
    /// 块尾消息下标（不含；仅作展示，隐藏区间按轮现算）。
    pub msg_end: usize,
    /// 块内轮次的估算体量（chars/2 口径）。
    pub estimate_tokens: u64,
    /// 是否已闭合（其后已有更新块）——只有闭合块可被压缩。
    pub closed: bool,
    /// 主工具计数（TOP-n 展示用）。
    pub tools: Vec<(String, u32)>,
    /// 主要目标（TOP-n 展示用）。
    pub targets: Vec<String>,
}

/// 会话内已声明的分块状态（由 marker 反解；会话是唯一真源）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FaceMarkers {
    pub compressed: BTreeSet<u32>,
    pub truncated: BTreeSet<u32>,
    /// 0cz S2：清零边界（1 基轮号 R——轮 < R 的全部内容移出窗口）。
    /// 多枚清零 marker 时取**最后**一枚（边界单调前进；早前边界是其子集）。
    pub cleared_before_round: Option<u32>,
    /// 每块的**外挂台账 `[seq]` 闭区间**（marker 的「台账定位」行反解；
    /// 三元一致的第三键——2026-09-16 实现批补齐，设计 §2 §11）。
    pub ledger_seq: BTreeMap<u32, (u64, u64)>,
    /// 每块所属 marker 的 **journal seq 闭区间**（0bh ⑮ 2026-09-22：指针
    /// `r<轮>·b<块>·s<journal seq>` 的 `s` 来源——marker「原文定位（四项）」
    /// 的 `- journal: run=… seq=a–b` 行反解；同 marker 内各块共享区间）。
    pub journal_seq: BTreeMap<u32, (u64, u64)>,
}

impl FaceMarkers {
    pub fn state(&self, number: u32) -> BlockState {
        if self.truncated.contains(&number) {
            BlockState::Truncated
        } else if self.compressed.contains(&number) {
            BlockState::Compressed
        } else {
            BlockState::Live
        }
    }

    pub fn is_empty(&self) -> bool {
        self.compressed.is_empty()
            && self.truncated.is_empty()
            && self.cleared_before_round.is_none()
            && self.ledger_seq.is_empty()
            && self.journal_seq.is_empty()
    }

    /// 0cz S2：该 0 基轮是否已被清零（轮 1 基号 < 边界 R ⇒ 已清零）。
    pub fn is_round_cleared(&self, round0: usize) -> bool {
        match self.cleared_before_round {
            Some(r) => (round0 as u64) + 1 < r as u64,
            None => false,
        }
    }

    /// 0cz S2：清零隐藏区的消息终点（隐藏区间 `[0, end)`；＝边界轮的起点）。
    /// 轮次不足/无边界 ⇒ 0（无隐藏）。
    pub fn clear_hidden_end(&self, ranges: &[(usize, usize)]) -> usize {
        let Some(r) = self.cleared_before_round else {
            return 0;
        };
        if r <= 1 {
            return 0;
        }
        // 隐藏轮 1..=R-1（1 基）＝ 0 基 0..=R-2 ⇒ 终点＝轮 R-1（1 基）的
        // 区间尾＝轮 R（边界轮）的区间首（marker 插入点）。
        ranges.get(r as usize - 2).map(|&(_, e)| e).unwrap_or(0)
    }

    /// 242 批（0da S2 审查处置 P2）：**整块已清零**——first∧last 两端轮次
    /// 都在边界前。机制排除（可压集合/截断 live/T1 计数/块隐藏臂跳过）与
    /// 块表「已清零」标态的**单一判据**。跨边界块（first 清零、last 未清零）
    /// **不**算整块清零：其边界后段仍在窗口，必须照常参与滑块/压缩/截断
    /// 兜底——首轮端点单判据（`is_round_cleared(first_round)`）会把该段
    /// 永久排除在回收机制外（审查实证：清零后重生长分区不保证在边界处
    /// 切分块，跨边界块是可达形态）。
    pub fn is_block_cleared(&self, first_round: usize, last_round: usize) -> bool {
        self.is_round_cleared(first_round) && self.is_round_cleared(last_round)
    }

    /// 该块的外挂台账 `[seq]` 区间（未外挂 ⇒ `None`）。
    pub fn ledger_range(&self, number: u32) -> Option<(u64, u64)> {
        self.ledger_seq.get(&number).copied()
    }

    /// 该块的 journal seq 区间（0bh ⑮；旧 marker 缺行 ⇒ `None`）。
    pub fn journal_range(&self, number: u32) -> Option<(u64, u64)> {
        self.journal_seq.get(&number).copied()
    }

    /// 0bh ⑮：该块的定位符 `r<轮>·b<块>·s<seq>`（≤24 B；`s` 缺 ⇒ 用台账
    /// seq 顶位并在渲染处如实标注；两者都缺 ⇒ `None`——不虚构）。
    pub fn pointer_for(&self, number: u32, first_round: usize) -> Option<String> {
        let seq = self
            .journal_seq
            .get(&number)
            .map(|(from, _)| *from)
            .or_else(|| self.ledger_seq.get(&number).map(|(from, _)| *from))?;
        Some(format!("r{}·b{number}·s{seq}", first_round + 1))
    }
}

/// 模型面装配参数（每次装配由 loop 现取，无隐藏状态）。
///
/// 字段**自带所有权**（不借 `writer`／loop 局部状态）：装配点遍布 loop-top、
/// 请求装配与 run 尾，借用形态会与 `writer` 的 `&mut` 借用打架；每次装配只
/// 复制几个小字符串，代价可忽略。
#[derive(Debug, Clone)]
pub struct ModelFaceParams {
    /// 主滑块 x（估算口径 chars/2）。
    pub slider_tokens: u64,
    /// 分块 y（估算口径 chars/2）。
    pub block_tokens: u64,
    /// 外挂台账路径（固定指针文案；`None` = 不渲染指针行）。
    pub ledger_path: Option<PathBuf>,
    /// 分块回放档案的会话标签（`<tag>-block-<k>.md`；`None` = 表内不列回放路径）。
    pub archive_tag: Option<String>,
    /// 运行 id（分块表 run 列）。
    pub run_id: String,
    /// D4 机械段（run 基线＋自编辑清单＋编辑指纹；调用方按 epoch 冻结）。
    pub d4_block: Option<String>,
    /// **静态开销**（系统提示词 ＋ 工具定义的估算，chars/2 同尺；2026-09-16
    /// 实现批，审查 R-9）：只进**估算**（阶梯/守卫的量尺＝真正发往模型商的
    /// 请求体积），不进视图装配（系统提示词与工具面在请求装配处独立携带）。
    /// 生产＝上一轮请求的实测值（首轮 0）；测试缝隙可 pin 0 以保持既有刻度语义。
    pub static_overhead_tokens: u64,
}

/// **主滑块起点**＝最近 `slider_tokens` 估算的连续完整轮的首个声明下标。
/// 全部内容都放得下时返回 `None`（整段会话即主滑块、无分块）。
pub fn slider_start(messages: &[Message], slider_tokens: u64) -> Option<usize> {
    crate::action_ledger::bridge_cut(messages, slider_tokens)
}

/// 主滑块之外的逻辑分块（编号锚在会话起点，稳定可寻址）。
pub fn blocks_outside_slider(
    messages: &[Message],
    slider_tokens: u64,
    block_tokens: u64,
) -> Vec<ContextBlock> {
    let ranges = crate::action_ledger::round_ranges(messages);
    if ranges.is_empty() {
        return Vec::new();
    }
    let Some(start) = slider_start(messages, slider_tokens) else {
        return Vec::new();
    };
    let block_tokens = block_tokens.max(1);
    let mut blocks: Vec<ContextBlock> = Vec::new();
    let mut cur_first: Option<usize> = None;
    let mut cur_total: u64 = 0;
    let mut cur_last: usize = 0;
    let mut number: u32 = 0;
    for (idx, &(s, e)) in ranges.iter().enumerate() {
        if s >= start {
            break;
        }
        // 配对不变量：不完整轮不进分块（不完整轮及其后全部留在主滑块面）。
        if !crate::action_ledger::is_round_complete(messages, (s, e)) {
            break;
        }
        cur_first.get_or_insert(idx);
        cur_last = idx;
        cur_total =
            cur_total.saturating_add(crate::controller::estimate_messages_tokens(&messages[s..e]));
        if cur_total >= block_tokens {
            number += 1;
            let first = cur_first.take().unwrap_or(idx);
            blocks.push(make_block(
                number, first, idx, &ranges, messages, cur_total, true,
            ));
            cur_total = 0;
        }
    }
    if let Some(first) = cur_first {
        // 仍在增长的**残段**（未闭合）：留在原文面，不可压缩、不可截断。
        // 2026-09-16 实现批（P0 修复）：末轮必须记**它真正累计到的最后一轮**
        // （`cur_last`），不得写 `ranges.len() - 1`——后者等于「会话末轮」，
        // 会把工作现场及其后所有新轮次一起圈进隐藏区间（截断时表现为模型面
        // 当场失明，违反 I1）。
        number += 1;
        blocks.push(make_block(
            number,
            first,
            cur_last.max(first),
            &ranges,
            messages,
            cur_total,
            false,
        ));
    }
    blocks
}

fn make_block(
    number: u32,
    first_round: usize,
    last_round: usize,
    ranges: &[(usize, usize)],
    messages: &[Message],
    estimate_tokens: u64,
    closed: bool,
) -> ContextBlock {
    let mut counts: Vec<(String, u32)> = Vec::new();
    let mut targets: Vec<String> = Vec::new();
    for &(s, e) in &ranges[first_round..=last_round] {
        for m in &messages[s..e] {
            for tc in &m.tool_calls {
                match counts.iter_mut().find(|(name, _)| *name == tc.name) {
                    Some((_, n)) => *n += 1,
                    None => counts.push((tc.name.clone(), 1)),
                }
                let t = crate::action_ledger::target_of_call(tc);
                if !t.is_empty() && !targets.contains(&t) {
                    targets.push(t);
                }
            }
        }
    }
    counts.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    ContextBlock {
        number,
        first_round,
        last_round,
        msg_start: ranges[first_round].0,
        msg_end: ranges[last_round].1,
        estimate_tokens,
        closed,
        tools: counts,
        targets,
    }
}

/// 0ap（2026-09-18，设计 §2/§3）：滑块读数——主滑块以外**未压缩**分块数
/// N＋估算 token 合计（「可压缩量」）。数据源＝既有分块账
/// （`blocks_outside_slider` ＋ `face_markers` 的 marker 反解），**读时
/// 现算、零新增记账**；只暴露数量与估算，分块内容/原文不流出模型面（v8
/// 「仅分块、不流出模型面」不变）；advisory——不联动任何硬门（H1/T1/
/// 轮预算/资源门/orientation 全不接，设计 §2 纪律）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SliderReadout {
    /// 已闭合且仍为原文（Live）的分块数——可被语义摘要压缩的量
    /// （与 `compress_blocks_now` 的可压缩判定同源同尺：`closed ∧ Live`）。
    pub compressible_blocks: usize,
    /// 上述分块的估算体量合计（chars/2 口径）。
    pub compressible_estimate_tokens: u64,
    /// 主滑块以外全部分块数（含未闭合残段与已压缩/截断——上下文信息）。
    pub total_blocks: usize,
}

/// 读时现算滑块读数（纯函数：不取任何锁、不改任何状态）。
pub fn slider_readout(
    messages: &[Message],
    slider_tokens: u64,
    block_tokens: u64,
) -> SliderReadout {
    let blocks = blocks_outside_slider(messages, slider_tokens, block_tokens);
    let markers = face_markers(messages);
    let mut compressible_blocks = 0usize;
    let mut compressible_estimate_tokens = 0u64;
    for b in &blocks {
        // 0cz S2：已清零块不进可压集合（已移出窗口；compressible＝closed ∧
        // Live ∧ 未清零）。242 批（P2）：判据收窄为**整块已清零**——跨边界
        // 块的边界后段仍在窗口，照常可压（首轮端点判据会把它永久排除在
        // 兜底回收外）。
        if b.closed
            && markers.state(b.number) == BlockState::Live
            && !markers.is_block_cleared(b.first_round, b.last_round)
        {
            compressible_blocks += 1;
            compressible_estimate_tokens += b.estimate_tokens;
        }
    }
    SliderReadout {
        compressible_blocks,
        compressible_estimate_tokens,
        total_blocks: blocks.len(),
    }
}

/// 估算读数标签（与 v8 读数口径同族：≥1M 用 M 记法一位小数，≥1K 用 K，
/// 其余原值照出）。
pub fn estimate_tokens_label(tokens: u64) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}M", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{}K", tokens / 1_000)
    } else {
        format!("{tokens}")
    }
}

/// 滑块读数段文本（`blackboard_read` 响应头增段与 `context_compress` 响应
/// 辅面**共用同一渲染**——设计 §3「辅面自带同表」；形态＝
/// 「滑块外可压缩 N 块 ≈ est K」）。
pub fn render_slider_readout_line(readout: &SliderReadout) -> String {
    format!(
        "滑块外可压缩 {} 块 ≈ est {}",
        readout.compressible_blocks,
        estimate_tokens_label(readout.compressible_estimate_tokens),
    )
}

/// 反解会话内的分块状态（扫描 v8 marker；v0.1–v0.3 历史 marker 无块号 ⇒ 忽略）。
pub fn face_markers(messages: &[Message]) -> FaceMarkers {
    let mut out = FaceMarkers::default();
    for m in messages {
        // 0cz S2：清零 marker（自有前缀，与分块 marker 互斥）——边界取
        // 最后一次（单调前进；早前边界是其子集）。台账/journal 定位行照常
        // 反解（清零 marker 同样携带「台账定位」——由执行面写入）。
        if let Some(r) = parse_clear_boundary(&m.content) {
            out.cleared_before_round = Some(r);
        }
        let Some((state, numbers)) = parse_marker_blocks(&m.content) else {
            continue;
        };
        for n in numbers {
            match state {
                BlockState::Compressed => {
                    out.compressed.insert(n);
                }
                BlockState::Truncated => {
                    out.truncated.insert(n);
                }
                // 清零态不进块号集合——表态由边界轮推导（is_round_cleared）。
                BlockState::Live | BlockState::Cleared => {}
            }
        }
        for (number, range) in parse_marker_ledger_seqs(&m.content) {
            out.ledger_seq.insert(number, range);
        }
        // 0bh ⑮：journal seq 区间（指针 `s` 的权威来源）——同 marker 的
        // 各块共享该区间；旧 marker 缺行则如实缺（pointer 退用台账 seq）。
        if let Some(range) = parse_marker_journal_range(&m.content) {
            for n in parse_marker_blocks(&m.content)
                .map(|(_, numbers)| numbers)
                .unwrap_or_default()
            {
                out.journal_seq.insert(n, range);
            }
        }
    }
    out
}

/// 0cz S2：清零边界的**权威消息下标**（最后一枚 marker 的位置；隐藏区间
/// `[0, idx)`、marker 本体即存根留在窗口）。marker（role=User）不开新轮、
/// 会被轮区间吸进前一轮——轮端点推导（[`FaceMarkers::clear_hidden_end`]）
/// 只作无 marker 消息时的回退。
pub fn clear_marker_index(messages: &[Message]) -> Option<usize> {
    messages
        .iter()
        .rposition(|m| m.content.starts_with(CLEAR_MARKER_PREFIX))
}

/// 反解清零 marker 的边界轮号（`清零边界: 第 R 轮`；1 基）。脏数据 ⇒
/// `None`（边界不虚构）。多方读数一致：`face_markers` 据此维护
/// `cleared_before_round`。
pub fn parse_clear_boundary(content: &str) -> Option<u32> {
    if !content.starts_with(CLEAR_MARKER_PREFIX) {
        return None;
    }
    let line = content
        .lines()
        .map(str::trim_start)
        .find(|l| l.starts_with(CLEAR_MARKER_BOUNDARY_LABEL))?;
    let rest = line.trim_start_matches(CLEAR_MARKER_BOUNDARY_LABEL);
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    let r: u32 = digits.parse().ok()?;
    (r >= 1).then_some(r)
}

/// 反解 marker 的 journal seq 区间（`- journal: run=… seq=a–b（…）`；
/// 区间用 **en dash** `–`，与 `LocatorPointers::render_marker_lines` 同源）。
/// 缺失/脏数据 ⇒ `None`（指针不虚构）。
pub fn parse_marker_journal_range(content: &str) -> Option<(u64, u64)> {
    let line = content
        .lines()
        .map(str::trim_start)
        .find(|l| l.starts_with("- journal:"))?;
    let after = line.split_once("seq=")?.1;
    let end = after.find(['（', '）', ' ']).unwrap_or(after.len());
    let (a, b) = after[..end].split_once('–')?;
    Some((a.trim().parse().ok()?, b.trim().parse().ok()?))
}

/// 反解 marker 的「台账定位」行（`台账定位: 块#1 [seq] 12-25; 块#2 [seq] 26-40`）。
/// 缺失/脏数据 ⇒ 该块无台账键（分块表如实留空，不虚构）。
pub fn parse_marker_ledger_seqs(content: &str) -> Vec<(u32, (u64, u64))> {
    const LABEL: &str = "台账定位:";
    let Some(line) = content
        .lines()
        .map(str::trim_start)
        .find(|l| l.starts_with(LABEL))
    else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for part in line.trim_start_matches(LABEL).split(';') {
        let part = part.trim();
        let Some(rest) = part.strip_prefix("块#") else {
            continue;
        };
        let Some((number_text, seq_text)) = rest.split_once("[seq]") else {
            continue;
        };
        let Ok(number) = number_text.trim().parse::<u32>() else {
            continue;
        };
        let Some((from, to)) = seq_text.trim().split_once('-') else {
            continue;
        };
        let (Ok(from), Ok(to)) = (from.trim().parse::<u64>(), to.trim().parse::<u64>()) else {
            continue;
        };
        if from <= to {
            out.push((number, (from, to)));
        }
    }
    out
}

/// 解析一个 v8 marker 的「块状态 ＋ 块号集合」。非 v8 marker 返回 `None`。
pub fn parse_marker_blocks(content: &str) -> Option<(BlockState, Vec<u32>)> {
    if !content.starts_with(crate::prompt::CONTEXT_COMPRESSED_PREFIX) {
        return None;
    }
    let state = if content.contains(BLOCK_MARKER_TRUNCATED_VERSION) {
        BlockState::Truncated
    } else if content.contains(BLOCK_MARKER_COMPRESSED_VERSION) {
        BlockState::Compressed
    } else {
        return None;
    };
    let line = content
        .lines()
        .map(str::trim_start)
        .find(|l| l.starts_with(BLOCK_MARKER_RANGE_LABEL))?;
    let numbers = parse_block_numbers(line.trim_start_matches(BLOCK_MARKER_RANGE_LABEL))?;
    Some((state, numbers))
}

/// `"1-4, 6"` → `[1,2,3,4,6]`；非法段截断在最后一个合法段。
fn parse_block_numbers(spec: &str) -> Option<Vec<u32>> {
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
            break;
        };
        if a == 0 || b < a || b > 100_000 {
            break;
        }
        for n in a..=b {
            if out.len() >= MAX_MARKER_BLOCKS {
                return Some(out);
            }
            out.push(n);
        }
    }
    (!out.is_empty()).then_some(out)
}

/// `true` ＝ 该消息是 v8 分块 marker（**永不隐藏**：它就是压缩后留在模型面里
/// 的那一行摘要）。
pub fn is_block_marker(content: &str) -> bool {
    parse_marker_blocks(content).is_some()
}

/// 已压缩/已截断块的隐藏消息区间 ＋ 0cz 清零边界区间。
///
/// 双护栏（2026-09-16 实现批，P0 修复）：
/// ① **未闭合残段永不隐藏**（它仍在增长，隐藏它等于隐藏工作现场）；
/// ② 区间末端**硬钳到主滑块起点**（`slider_start`）——不变量 I1 的结构性
///    保证：任何路径都不能隐藏主滑块及其后的内容（现算区间即便因脏 marker
///    越界，也只会在滑块起点处被截断）。
/// 0cz S2（设计 §5.1）：清零区间（`[0, clear_hidden_end)`）**不受钳 ②**
/// ——I1 的登记显式例外（模型主动发起＋强制交接写入前置＋边界以 marker
/// 为界可回放，三条件由执行面保证）；护栏 ① 不适用（清零以轮为界，边界轮
/// 本身不在隐藏区）。
/// `clear_end` 由调用方下传（0da S2 前置段同一 gated 推导——「有 marker
/// 消息且边界可解析」才隐藏，脏 marker 不虚构边界；242 批改参数化：公开
/// 入口恰算一次，内部不再自查，前置段与主循环同源）。
fn hidden_message_ranges(
    messages: &[Message],
    ranges: &[(usize, usize)],
    blocks: &[ContextBlock],
    markers: &FaceMarkers,
    slider_start: Option<usize>,
    clear_end: usize,
) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    if clear_end > 0 {
        out.push((0, clear_end));
    }
    for block in blocks {
        if !block.closed {
            continue;
        }
        if markers.state(block.number) == BlockState::Live {
            continue;
        }
        let Some(&(start, _)) = ranges.get(block.first_round) else {
            continue;
        };
        let end = ranges
            .get(block.last_round)
            .map(|&(_, e)| e)
            .unwrap_or_else(|| messages.len());
        let end = slider_start.map_or(end, |s| end.min(s));
        // 242 批（P2）：整块已清零 ⇒ 已被 (0, clear_end) 臂覆盖，跳过；
        // **跨边界块不再被首轮端点判据排除**——其边界后段（压缩/截断后）
        // 必须由块臂隐藏，否则 marker 声明「已压缩」而正文仍在面。
        if end > start && !markers.is_block_cleared(block.first_round, block.last_round) {
            out.push((start, end));
        }
    }
    out
}

/// **模型面装配**（设计 §1 §2）。无分块时（整段会话即主滑块）除**常驻头
/// （指针，0bz S3）＋窗口尾 D4 机械段（0bz S3′）**外原样返回 `messages`
/// ——头部结构消息自首个轮次即在，开窗转换只剩尾部追加
/// （见 [`try_clone_messages_with_resident_head`]）。
///
/// **0bc S2④（2026-09-21）：可失败分配**。本函数是 S1 清单的「模型缓冲」
/// 巨量分配路径——逐条克隆改 `try_reserve(_exact)` 族；失败以
/// `ErrorKind::OutOfMemory` 上抛，由调用方进终态收口（不 abort、不 panic）。
/// 残余登记：`ToolCall::arguments`（`serde_json::Value`）深克隆无对应物，
/// 保留普通克隆。
pub fn build_model_face(
    messages: &[Message],
    params: &ModelFaceParams,
) -> Result<Vec<Message>, std::io::Error> {
    // 0da S2（2026-10-11，设计 §4.1，D1 修复）：清零 marker 在场**不再**短路
    // 独立装配——有块清零走**常规装配**（`hidden_message_ranges` 的
    // `(0, clear_end)` 前置臂已在位、不钳 slider_start），边界后内容照常受
    // 滑块/压缩/截断/700K 守卫钳制（0cz 设计 §5.3/§9「机制不退役」恢复成立；
    // 239 批审查实证短路使兜底收缩全链失效）。`build_cleared_face` 保留为
    // **无块清零面**专用（无块早退不应用 hidden 区间，见下）。
    // 清零隐藏区间有效起点：marker 在场取 marker 消息下标（0cz S2 权威），
    // 脏数据回退边界轮推导；未清零恒 0（各循环零行为变化）。
    let markers = face_markers(messages);
    let clear_end = if markers.cleared_before_round.is_some() {
        clear_marker_index(messages).unwrap_or_else(|| {
            markers.clear_hidden_end(&crate::action_ledger::round_ranges(messages))
        })
    } else {
        0
    };
    let blocks = blocks_outside_slider(messages, params.slider_tokens, params.block_tokens);
    if blocks.is_empty() {
        if clear_end > 0 {
            return build_cleared_face(messages, &markers, params);
        }
        // 0bz S3（2026-09-28）：**常驻头（指针；0bz S3′ 后 D4 落窗口尾）自
        // 首个轮次起即在**——旧
        // 口径「无分块原样返回」使指针/D4 拖到首个分块形成那一刻才整体插入
        // face 前部 ⇒ 开窗轮前缀全变、整窗 miss（`RUN-CLI-6ab99969` r40 实测
        // 135,754 tk，六轮狗粮「自发塌陷」同族的机理）。注入点＝
        // `round_ranges[0].0`，与开窗后 `preamble_end = blocks[0].msg_start`
        // 恒同值（首个分块必为首个完整轮）⇒ 开窗转换只剩尾部追加（块表／
        // RUN_END 行），前缀逐字节稳定。内容零改，只动时点。
        return try_clone_messages_with_resident_head(messages, params);
    }
    let ranges = crate::action_ledger::round_ranges(messages);
    let start = slider_start(messages, params.slider_tokens);
    let hidden = hidden_message_ranges(messages, &ranges, &blocks, &markers, start, clear_end);
    let preamble_end = blocks[0].msg_start;
    // 0bc S2④：一次可失败预留（上界＝len＋4：前置＋指针/D4＋尾部消息＋块表
    // ＋结束自述通道行），此后 push 不再触发不可失败的增长重分配。
    let mut view: Vec<Message> = Vec::new();
    view.try_reserve_exact(messages.len() + 4)
        .map_err(alloc_err_io)?;
    // 前置段（0da S2 设计 §4.2）：**清零区间内仅保留白名单**（跨清零保留，
    // 0cz 设计 §5.3；常规路径此前无此豁免＝D1 修复面），区间外原序全推
    // ——未清零（clear_end=0）时与旧口径逐条等价。
    for (i, m) in messages.iter().enumerate().take(preamble_end) {
        if i >= clear_end || m.content.starts_with(crate::prompt::WHITELIST_PREFIX) {
            view.push(try_clone_message(m)?);
        }
    }
    if let Some(ledger) = params.ledger_path.as_deref() {
        view.push(mechanical_message(
            crate::action_ledger::build_pointer_message(ledger),
        ));
    }
    for (i, m) in messages.iter().enumerate() {
        if i < preamble_end {
            continue;
        }
        // marker 永不隐藏（它承载压缩后的摘要行）。
        if is_block_marker(&m.content) {
            view.push(try_clone_message(m)?);
            continue;
        }
        // 0da S2：白名单永不隐藏（跨压缩/截断/清零保留；与前置段豁免同口径）。
        if m.content.starts_with(crate::prompt::WHITELIST_PREFIX) {
            view.push(try_clone_message(m)?);
            continue;
        }
        if hidden.iter().any(|&(s, e)| i >= s && i < e) {
            continue;
        }
        view.push(try_clone_message(m)?);
    }
    // 0bz S3′（2026-10-04）：**D4 机械段落尾部**（块表之前）——D4 是逐
    // epoch 易变渲染（自编辑清单随编辑更新），放头部（marker 之前）时每次
    // 重渲都重价其后的整段前缀（recli 四跑离线对账实证：9/9 压缩后 +2 请求
    // 在 D4 槽全前缀重价＝「第 2 针」未消的机理）。落尾部后 D4 重渲只重价
    // 尾部自身，与块表同属窗口尾易变区。内容零改，只动位置。
    if let Some(d4) = params.d4_block.as_deref() {
        view.push(mechanical_message(d4.to_string()));
    }
    // 分块表**落尾部**（前缀纪律；见模块头注）。
    view.push(mechanical_message(render_block_table(
        &blocks, &markers, params,
    )));
    // 0bs ①（2026-09-25）：**结束自述通道常驻尾行**——告知面收口。0bm 轮
    // 实证：`[RUN_END]` 语法只挂 pull 面 `guide`（`blackboard_read section=
    // guide`）⇒ 137 工具轮零自述、`run_finished` 仍三键。单一来源＝
    // `model_stop`；每轮尾随（分块表之后＝窗口最末消息）。0bz S3 后的口径：
    // 头部结构消息（指针/D4）已常驻，尾部两行仍待分块出现——尾部追加不破
    // 前缀，无分块会话不注入。0bz S3′ 后指针常驻头部、D4 落尾部（均常驻）。
    view.push(mechanical_message(
        crate::model_stop::model_stop_resident_line(),
    ));
    Ok(view)
}

/// 0cz S2（设计 §5.1）＋0da S2（设计 §4.1/O3）：**无块清零面装配**——
/// `[边界前白名单块（原序；clear 是超集动作，设计 §5.3）]` ＋ `[台账指针]` ＋
/// `[边界轮起全部消息逐字（含清零 marker 本体）]` ＋ `[D4]` ＋ `[块表（有块时）]`
/// ＋ `[结束自述通道行（有块时）]`。0da S2 起**仅服务无块会话**（无块早退
/// 不应用 hidden 区间；有块清零改走常规装配——D1 修复），块表臂保留使本
/// 函数独立成立。与常规开窗形态同构：头部结构常驻、尾部易变区追加。
fn build_cleared_face(
    messages: &[Message],
    markers: &FaceMarkers,
    params: &ModelFaceParams,
) -> Result<Vec<Message>, std::io::Error> {
    let ranges = crate::action_ledger::round_ranges(messages);
    let hidden_end =
        clear_marker_index(messages).unwrap_or_else(|| markers.clear_hidden_end(&ranges));
    let mut view: Vec<Message> = Vec::new();
    view.try_reserve_exact(messages.len() + 4)
        .map_err(alloc_err_io)?;
    for m in messages.iter().take(hidden_end) {
        if m.content.starts_with(crate::prompt::WHITELIST_PREFIX) {
            view.push(try_clone_message(m)?);
        }
    }
    if let Some(ledger) = params.ledger_path.as_deref() {
        view.push(mechanical_message(
            crate::action_ledger::build_pointer_message(ledger),
        ));
    }
    for m in messages.iter().skip(hidden_end) {
        view.push(try_clone_message(m)?);
    }
    if let Some(d4) = params.d4_block.as_deref() {
        view.push(mechanical_message(d4.to_string()));
    }
    let blocks = blocks_outside_slider(messages, params.slider_tokens, params.block_tokens);
    if !blocks.is_empty() {
        view.push(mechanical_message(render_block_table(
            &blocks, markers, params,
        )));
        view.push(mechanical_message(
            crate::model_stop::model_stop_resident_line(),
        ));
    }
    Ok(view)
}

/// 0bz S3（2026-09-28）：早期（无分块）形态的**常驻头**——指针注入到首个
/// 轮次起点（`round_ranges[0].0`）；D4 机械段不进头部，0bz S3′（2026-10-04）
/// 起落窗口尾＝末条历史之后（与开窗形态同位）。指针注入点与开窗后
/// `preamble_end = blocks[0].msg_start` 恒同值（首个分块必为首个完整轮），
/// 故「无分块 → 有分块」的转换只发生尾部追加，face 前缀逐字节稳定。轮次
/// 尚未成形时不注入（成形那一刻 face 仅数 K token，一次性小成本，且此后
/// 注入点恒定）。
fn try_clone_messages_with_resident_head(
    messages: &[Message],
    params: &ModelFaceParams,
) -> Result<Vec<Message>, std::io::Error> {
    let Some(at) = crate::action_ledger::round_ranges(messages)
        .first()
        .map(|&(s, _)| s)
    else {
        return try_clone_messages(messages);
    };
    // 0bz S3′（2026-10-04）：常驻头只保留指针；D4 机械段落**窗口尾**（与
    // 开窗形态同位——末条历史之后），逐 epoch 重渲只重价尾部自身。开窗后
    // D4 位置不变（块表/结束行追加其后），前缀逐字节稳定。
    let mut view = try_clone_messages(messages)?;
    view.try_reserve_exact(2).map_err(alloc_err_io)?;
    if let Some(d4) = params.d4_block.as_deref() {
        view.push(mechanical_message(d4.to_string()));
    }
    if let Some(ledger) = params.ledger_path.as_deref() {
        view.insert(
            at,
            mechanical_message(crate::action_ledger::build_pointer_message(ledger)),
        );
    }
    Ok(view)
}

/// **模型面消息计数**（0bc S2④，2026-09-21）：与 [`build_model_face`] 逐条同
/// 口径（同一过滤、同一追加），但**不克隆任何消息**——只需条数的事件面
/// （压缩事件 `messages_kept`）用它替代「物化整面再取 `len`」。
pub fn model_face_message_count(messages: &[Message], params: &ModelFaceParams) -> usize {
    // 0da S2（设计 §4.1/§4.3，D1＋D4）：`face_markers` 恰算一次；清零态
    // **不再**独立分支——与 build 同构走常规路径（无块清零面分支保留）。
    let markers = face_markers(messages);
    let cleared = markers.cleared_before_round.is_some();
    let clear_end = if cleared {
        clear_marker_index(messages).unwrap_or_else(|| {
            markers.clear_hidden_end(&crate::action_ledger::round_ranges(messages))
        })
    } else {
        0
    };
    let blocks = blocks_outside_slider(messages, params.slider_tokens, params.block_tokens);
    if blocks.is_empty() {
        if clear_end > 0 {
            // 无块清零面计数（build_cleared_face 同口径）。
            let mut count = messages
                .iter()
                .take(clear_end)
                .filter(|m| m.content.starts_with(crate::prompt::WHITELIST_PREFIX))
                .count();
            if params.ledger_path.is_some() {
                count += 1;
            }
            count += messages.len() - clear_end;
            if params.d4_block.is_some() {
                count += 1;
            }
            return count;
        }
        // 0bz S3：常驻头与 build_model_face 同口径——轮次成形后 ＋指针 ＋D4。
        let mut count = messages.len();
        if !crate::action_ledger::round_ranges(messages).is_empty() {
            if params.ledger_path.is_some() {
                count += 1;
            }
            if params.d4_block.is_some() {
                count += 1;
            }
        }
        return count;
    }
    let ranges = crate::action_ledger::round_ranges(messages);
    let start = slider_start(messages, params.slider_tokens);
    let hidden = hidden_message_ranges(messages, &ranges, &blocks, &markers, start, clear_end);
    let preamble_end = blocks[0].msg_start;
    // 前置段计数（0da S2）：清零区间内仅白名单（与 build 前置段同口径）。
    let mut count = messages
        .iter()
        .enumerate()
        .take(preamble_end)
        .filter(|(i, m)| *i >= clear_end || m.content.starts_with(crate::prompt::WHITELIST_PREFIX))
        .count();
    if params.ledger_path.is_some() {
        count += 1;
    }
    if params.d4_block.is_some() {
        count += 1;
    }
    for (i, m) in messages.iter().enumerate() {
        if i < preamble_end {
            continue;
        }
        if is_block_marker(&m.content) {
            count += 1;
            continue;
        }
        // 0da S2：白名单永不隐藏（与 build 主循环同口径）。
        if m.content.starts_with(crate::prompt::WHITELIST_PREFIX) {
            count += 1;
            continue;
        }
        if hidden.iter().any(|&(s, e)| i >= s && i < e) {
            continue;
        }
        count += 1;
    }
    // 0bs ①：尾部追加＝分块表 ＋ 结束自述通道行（与 build_model_face 同口径）。
    count + 2
}

// 0bc S2④ 测试缝（`#[cfg(test)]`，线程本地）：构造性注入「装配路径分配
// 失败」——线程本地隔离，测试并行下同线程 set→用、互不串扰。
#[cfg(test)]
thread_local! {
    pub(crate) static FORCE_ALLOC_FAILURE: std::cell::Cell<bool> =
        const { std::cell::Cell::new(false) };
}

/// 分配失败统一映射（0bc S2④）：与 journal 装配路径同一词面约定
/// （`ErrorKind::OutOfMemory`；调用方以此进终态收口）。
fn alloc_err_io(_: std::collections::TryReserveError) -> std::io::Error {
    alloc_error()
}

fn alloc_error() -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::OutOfMemory,
        "model face assembly allocation failed (0bc S2④ fallible path)",
    )
}

fn try_clone_str(s: &str) -> std::io::Result<String> {
    let mut out = String::new();
    out.try_reserve_exact(s.len()).map_err(alloc_err_io)?;
    out.push_str(s);
    Ok(out)
}

fn try_clone_message(m: &Message) -> std::io::Result<Message> {
    #[cfg(test)]
    {
        if FORCE_ALLOC_FAILURE.with(|flag| flag.replace(false)) {
            return Err(alloc_error());
        }
    }
    let mut tool_calls: Vec<ToolCall> = Vec::new();
    tool_calls
        .try_reserve_exact(m.tool_calls.len())
        .map_err(alloc_err_io)?;
    for tc in &m.tool_calls {
        tool_calls.push(ToolCall {
            name: try_clone_str(&tc.name)?,
            // 残余登记：`Value` 深克隆无 `try_reserve` 对应物（见装配函数注）。
            arguments: tc.arguments.clone(),
            call_id: try_clone_str(&tc.call_id)?,
        });
    }
    Ok(Message {
        role: m.role,
        content: try_clone_str(&m.content)?,
        tool_call_id: m.tool_call_id.as_deref().map(try_clone_str).transpose()?,
        tool_calls,
        reasoning_content: m
            .reasoning_content
            .as_deref()
            .map(try_clone_str)
            .transpose()?,
        round: m.round,
    })
}

fn try_clone_messages(messages: &[Message]) -> std::io::Result<Vec<Message>> {
    let mut out: Vec<Message> = Vec::new();
    out.try_reserve_exact(messages.len())
        .map_err(alloc_err_io)?;
    for m in messages {
        out.push(try_clone_message(m)?);
    }
    Ok(out)
}

/// 模型面估算（chars/2；**阶梯量尺**，设计 §3.1）。
pub fn model_face_estimate(messages: &[Message], params: &ModelFaceParams) -> u64 {
    estimate_model_face_tokens(messages, params)
}

/// 模型面估算的**不物化**实现（2026-09-16 实现批，卫生项）：与
/// [`build_model_face`] 逐条同口径（同一过滤与同一追加），但不克隆整段会话
/// ——500K 工作点上每轮多次调用，物化版本是 MB 级字符串拷贝。
pub fn estimate_model_face_tokens(messages: &[Message], params: &ModelFaceParams) -> u64 {
    // 0da S2（设计 §4.1/§4.3，D1＋D4）：`face_markers` 恰算一次；清零态
    // **不再**独立分支——与 build 同构走常规路径（无块清零面分支保留，
    // estimate ≡ build 逐条同口径）。
    let markers = face_markers(messages);
    let clear_end = if markers.cleared_before_round.is_some() {
        clear_marker_index(messages).unwrap_or_else(|| {
            markers.clear_hidden_end(&crate::action_ledger::round_ranges(messages))
        })
    } else {
        0
    };
    let blocks = blocks_outside_slider(messages, params.slider_tokens, params.block_tokens);
    if blocks.is_empty() {
        if clear_end > 0 {
            return estimate_cleared_face_tokens(messages, &markers, params, clear_end);
        }
        // 0bz S3：常驻结构与 build_model_face 同口径——轮次成形后计入指针
        // （头部）与 D4（0bz S3′ 起居窗口尾；计数与位置无关）。
        let mut total = crate::controller::estimate_messages_tokens(messages);
        if !crate::action_ledger::round_ranges(messages).is_empty() {
            if let Some(ledger) = params.ledger_path.as_deref() {
                total = total.saturating_add(injected_estimate(
                    &crate::action_ledger::build_pointer_message(ledger),
                ));
            }
            if let Some(d4) = params.d4_block.as_deref() {
                total = total.saturating_add(injected_estimate(d4));
            }
        }
        return total;
    }
    let ranges = crate::action_ledger::round_ranges(messages);
    let start = slider_start(messages, params.slider_tokens);
    let hidden = hidden_message_ranges(messages, &ranges, &blocks, &markers, start, clear_end);
    let preamble_end = blocks[0].msg_start;
    // 前置段（0da S2）：清零区间内仅白名单（与 build 前置段同口径）。
    let mut total = 0u64;
    for (i, m) in messages.iter().enumerate().take(preamble_end) {
        if i >= clear_end || m.content.starts_with(crate::prompt::WHITELIST_PREFIX) {
            total = total.saturating_add(crate::controller::estimate_message_tokens(m));
        }
    }
    if let Some(ledger) = params.ledger_path.as_deref() {
        total = total.saturating_add(injected_estimate(
            &crate::action_ledger::build_pointer_message(ledger),
        ));
    }
    if let Some(d4) = params.d4_block.as_deref() {
        total = total.saturating_add(injected_estimate(d4));
    }
    for (i, m) in messages.iter().enumerate() {
        if i < preamble_end {
            continue;
        }
        if is_block_marker(&m.content) {
            total = total.saturating_add(crate::controller::estimate_message_tokens(m));
            continue;
        }
        // 0da S2：白名单永不隐藏（与 build 主循环同口径）。
        if m.content.starts_with(crate::prompt::WHITELIST_PREFIX) {
            total = total.saturating_add(crate::controller::estimate_message_tokens(m));
            continue;
        }
        if hidden.iter().any(|&(s, e)| i >= s && i < e) {
            continue;
        }
        total = total.saturating_add(crate::controller::estimate_message_tokens(m));
    }
    let index = injected_estimate(&render_block_table(&blocks, &markers, params));
    let channel = injected_estimate(&crate::model_stop::model_stop_resident_line());
    total
        .saturating_add(index)
        .saturating_add(channel)
        .saturating_add(params.static_overhead_tokens)
}

/// 机械注入串（无 tool_calls 的 user 消息）的估算——与
/// `estimate_message_tokens` 同尺（chars/2）。
fn injected_estimate(text: &str) -> u64 {
    text.chars().count() as u64 / 2
}

/// 0da S2：无块清零面估算（`build_cleared_face` 的不物化镜像——逐条同口径；
/// 仅由 [`estimate_model_face_tokens`] 的无块清零分支到达，markers/clear_end
/// 单次计算下传）。
fn estimate_cleared_face_tokens(
    messages: &[Message],
    markers: &FaceMarkers,
    params: &ModelFaceParams,
    clear_end: usize,
) -> u64 {
    let mut total = 0u64;
    for m in messages.iter().take(clear_end) {
        if m.content.starts_with(crate::prompt::WHITELIST_PREFIX) {
            total = total.saturating_add(crate::controller::estimate_message_tokens(m));
        }
    }
    if let Some(ledger) = params.ledger_path.as_deref() {
        total = total.saturating_add(injected_estimate(
            &crate::action_ledger::build_pointer_message(ledger),
        ));
    }
    for m in messages.iter().skip(clear_end) {
        total = total.saturating_add(crate::controller::estimate_message_tokens(m));
    }
    if let Some(d4) = params.d4_block.as_deref() {
        total = total.saturating_add(injected_estimate(d4));
    }
    let blocks = blocks_outside_slider(messages, params.slider_tokens, params.block_tokens);
    if !blocks.is_empty() {
        let index = injected_estimate(&render_block_table(&blocks, markers, params));
        let channel = injected_estimate(&crate::model_stop::model_stop_resident_line());
        total = total.saturating_add(index).saturating_add(channel);
    }
    total.saturating_add(params.static_overhead_tokens)
}

fn mechanical_message(content: String) -> Message {
    Message {
        role: crate::gateway::model::Role::User,
        content,
        tool_call_id: None,
        tool_calls: Vec::new(),
        reasoning_content: None,
        round: None,
    }
}

/// 分块表渲染（设计 §2：`[块#k] 轮次 a–b ≈Ntk | 主工具目标 TOP-n |
/// 关键动作计数 | 台账 [seq] | run=…`；带状态的块附回放指针）。
///
/// 2026-09-16 实现批：① 三键补齐（`台账 [seq]` 来自 marker 反解，缺则如实留空）；
/// ② 未闭合**残段**显式标注（它不参与压缩/截断）；③ 回放指针附**两段门**提示
/// （0ct 确认性转换后＝`.gsa` 内部区首读附一次性确认提示，不影响内容读取——
/// 回放是生命线，提示只为认知不对齐兜底）。
pub fn render_block_table(
    blocks: &[ContextBlock],
    markers: &FaceMarkers,
    params: &ModelFaceParams,
) -> String {
    let mut lines = vec![format!("{BLOCK_TABLE_PREFIX} v0.1]")];
    lines.push(
        "本表列出**工作现场（你最近工作的连续轮次）以外**的分块；表内每块按会话起点编号、\
         跨轮次稳定（回放与压缩都按块号指定）。**本表在窗口尾部，逐轮刷新**；\
         未压缩/未截断的分块逐字仍在窗口内"
            .to_string(),
    );
    for block in blocks {
        // 0cz S2：清零态优先（**整块**已清零——242 批收拢为 is_block_cleared
        // 单一判据，与机制排除同源）。
        let state = if block.closed && markers.is_block_cleared(block.first_round, block.last_round)
        {
            "已清零"
        } else if block.closed && markers.is_round_cleared(block.first_round) {
            // 242 批（P2）：**跨清零边界块**（首轮在边界前、尾轮在边界后）
            // ——边界前段已由清零臂移出窗口、边界后段仍在窗口照常参与兜底
            // （如实标态；本批前误标「原文」）。
            "跨清零边界（前段已移出）"
        } else {
            match (block.closed, markers.state(block.number)) {
                (false, _) => "残段（仍在增长）",
                (true, BlockState::Live) => "原文",
                (true, BlockState::Compressed) => "已压缩",
                (true, BlockState::Truncated) => "已截断",
                (true, BlockState::Cleared) => "已清零",
            }
        };
        let ledger = match markers.ledger_range(block.number) {
            Some((from, to)) => format!("台账 [seq] {from}-{to}"),
            None if matches!(state, "原文" | "残段（仍在增长）") => {
                "台账 [seq] —（原文未外挂）".to_string()
            }
            None => "台账 [seq] —（本 marker 未带定位行）".to_string(),
        };
        // 0bh ⑮（2026-09-22，设计 §4.1）：块号 ↔ **定位符** `r<轮>·b<块>·s<seq>`
        // ——`s`＝journal 事件行号（marker「原文定位（四项）」的 journal 行反解；
        // 旧 marker 缺行时退用台账 seq 并在形态上不改写）。指针 ≤24 B/条。
        let pointer = match markers.pointer_for(block.number, block.first_round) {
            Some(ptr) => format!(" | 指针 {ptr}"),
            None => String::new(),
        };
        let ledger = format!("{ledger}{pointer}");
        let tools = if block.tools.is_empty() {
            "无工具".to_string()
        } else {
            block
                .tools
                .iter()
                .take(3)
                .map(|(name, n)| format!("{name}×{n}"))
                .collect::<Vec<_>>()
                .join(" ")
        };
        let targets = if block.targets.is_empty() {
            "（无）".to_string()
        } else {
            block
                .targets
                .iter()
                .take(2)
                .map(|t| truncate_chars(t, 48))
                .collect::<Vec<_>>()
                .join(" ")
        };
        let mut line = format!(
            "[块#{}] 轮次 {}-{} ≈{}tk token | {state} | 动作: {tools} | 目标: {targets} \
             | {ledger} | run={}",
            block.number,
            block.first_round + 1,
            block.last_round + 1,
            block.estimate_tokens,
            params.run_id,
        );
        if matches!(state, "已压缩" | "已截断") {
            let path = params
                .archive_tag
                .as_deref()
                .map(|tag| relative_block_archive(tag, block.number))
                .unwrap_or_else(|| "（见分块摘要 marker）".to_string());
            line.push_str(&format!(
                " | 原文回放: {path}（read_file offset/limit 分页；\
                 首次读 `.gsa` 会附一次性 [session_volume_notice] 确认提示，\
                 不影响内容读取）"
            ));
        }
        lines.push(line);
    }
    lines.push(format!(
        "说明: 工作现场与**残段**（不足一块的正在增长部分）都不参与压缩/截断；\
         **指针列与原文回放路径仅「已压缩/已截断」块具备**（原文块与残段逐字仍在窗口内、无指针）；\
         「已压缩/已截断」块的逐字原文仍全量留存在本地会话档案（本对话全量留档、不被覆盖写入）、\
         按块档案与 journal 三处，可按块回放。缩略词 [seq] ＝外挂台账行号。run={}",
        params.run_id
    ));
    lines.push("[/上下文分块表]".to_string());
    lines.join("\n")
}

/// 分块回放档案（绝对路径）：`{cwd}/.gsa/compaction/blocks/<tag>-block-<k>.md`。
pub fn block_archive_path(root: &Path, tag: &str, number: u32) -> PathBuf {
    root.join(".gsa")
        .join("compaction")
        .join("blocks")
        .join(format!("{tag}-block-{number:04}.md"))
}

/// 会话卷相对形态（分块表／marker 里给模型看的形态）。
pub fn relative_block_archive(tag: &str, number: u32) -> String {
    format!(".gsa/compaction/blocks/{tag}-block-{number:04}.md")
}

/// 0cz S2（设计 §5.2）：清零现场存档（绝对路径）——
/// `{cwd}/.gsa/compaction/blocks/<tag>-clear.md`（边界前全部轮次逐字留档，
/// 与按块回放同目录同性质）。
pub fn clear_archive_path(root: &Path, tag: &str) -> PathBuf {
    root.join(".gsa")
        .join("compaction")
        .join("blocks")
        .join(format!("{tag}-clear.md"))
}

/// 0cz S2：清零现场存档的会话卷相对形态。
pub fn relative_clear_archive(tag: &str) -> String {
    format!(".gsa/compaction/blocks/{tag}-clear.md")
}

/// 分块档案/会话标签（session id 前 8 字符；缺省用 run id 的短尾）。
pub fn archive_tag(session_id: Option<&str>, run_id: &str) -> String {
    if let Some(id) = session_id.filter(|s| !s.is_empty()) {
        let tag: String = id
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
            .take(8)
            .collect();
        if !tag.is_empty() {
            return tag;
        }
    }
    let tail: String = run_id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .rev()
        .take(8)
        .collect::<Vec<char>>()
        .into_iter()
        .rev()
        .collect();
    if tail.is_empty() {
        "session".to_string()
    } else {
        tail
    }
}

/// 阶梯里的**硬截断刻度**（T1；设计 §3 表最后一档）。阶梯结构固定为四档
/// （0bh ④ 定稿 2026-09-22：两软 ＋ 硬提醒 ＋ 硬截断），缺档时退化为最大刻度。
pub fn ladder_truncate_tokens(ladder: &[crate::context_scale::LadderStep; 4]) -> u64 {
    ladder
        .iter()
        .filter(|s| s.tier == crate::context_scale::LadderTier::HardTruncate)
        .map(|s| s.tokens)
        .max()
        .unwrap_or_else(|| ladder.iter().map(|s| s.tokens).max().unwrap_or(0))
}

/// 压缩 marker 的**模型面声明**（不变量 I5：压缩只作用于模型面、本地全量留档）。
pub const MODEL_FACE_DECLARATION: &str = "本次动作只作用于**当前上下文窗口**：本地单对话全量留档、\
    不被覆盖写入（会话归档时随黑板一起打包）；被处理分块的逐字原文可按块回放（见上指针）。";

/// marker 的**分块台账定位行**（三键之一；`块#k [seq] a-b`）。
///
/// 2026-09-16 实现批补齐（设计 §2 §11「块表 ↔ 台账 `[seq]` ↔ journal run+sequence
/// 齐备率 100%」）——由 [`parse_marker_ledger_seqs`] 反解，分块表逐行显示。
pub fn render_ledger_locator_line(entries: &[(u32, (u64, u64))]) -> String {
    if entries.is_empty() {
        return String::new();
    }
    let mut parts: Vec<String> = entries
        .iter()
        .map(|(number, (from, to))| format!("块#{number} [seq] {from}-{to}"))
        .collect();
    parts.sort();
    format!("台账定位: {}", parts.join("; "))
}

/// 0bh ③＋⑮（2026-09-22）：**定位符行**（`指针: r<轮>·b<块>·s<seq>；…`）——
/// 压缩回执/截断回执与分块表共用同一形态（设计 §4.1：`s`＝journal 事件
/// 行号；三生成点之「压缩回执」）。入参＝每块（块号，块首轮）＋窗口
/// journal 起点（`locators.journal_seq`）；缺 journal 起点或缺块 ⇒ 空串
/// （如实留空、不虚构）。
pub fn render_pointer_line(
    blocks: &[(u32, usize)],
    ledger_locators: &[(u32, (u64, u64))],
    journal_from: Option<u64>,
) -> String {
    let Some(jfrom) = journal_from else {
        return String::new();
    };
    let mut parts: Vec<String> = blocks
        .iter()
        .filter(|(number, _)| ledger_locators.iter().any(|(n, _)| n == number))
        .map(|(number, first_round)| format!("r{}·b{number}·s{jfrom}", first_round + 1))
        .collect();
    if parts.is_empty() {
        return String::new();
    }
    parts.sort();
    format!(
        "指针（回查：blackboard_read section=journal anchor=）: {}",
        parts.join("；")
    )
}

/// 分块压缩 marker（`[前文上下文已压缩 v0.4-分块压缩]`；restore-retained）。
///
/// `summary` ＝ 模型产出的语义摘要块（机械识别，可为空 ⇒ 纯结构化轨）；
/// `selection_note` ＝ 0bk ③ 区间对账行（缺省行为／声明 vs 实得；空不渲染）。
#[allow(clippy::too_many_arguments)]
pub fn compression_marker(
    numbers: &[u32],
    rounds_from: usize,
    rounds_to: usize,
    summary_id: &str,
    digest: &str,
    archive_path: &Path,
    session: Option<&str>,
    mechanical_rows: &str,
    summary: Option<&str>,
    selection_note: &str,
    replay: &str,
    locators: &crate::summary::LocatorPointers,
    ledger_locators: &[(u32, (u64, u64))],
    pointer_line: &str,
) -> String {
    let semantic = summary
        .map(|s| format!("\n== 语义摘要（模型产出） ==\n{s}\n"))
        .unwrap_or_default();
    let selection_note_line = if selection_note.is_empty() {
        String::new()
    } else {
        format!("区间说明: {selection_note}\n")
    };
    let numbers_text = render_block_numbers(numbers);
    let from = rounds_from + 1;
    let to = rounds_to + 1;
    let rounds = rounds_to.saturating_sub(rounds_from) + 1;
    let version = BLOCK_MARKER_COMPRESSED_VERSION;
    let locator_lines = locators.render_marker_lines(archive_path, digest);
    let ledger_line = render_ledger_locator_line(ledger_locators);
    let pointer_display = if pointer_line.is_empty() {
        String::new()
    } else {
        format!("{pointer_line}\n")
    };
    let archive_display = archive_path.display().to_string();
    let session_display = session.unwrap_or("（无）");
    format!(
        "[前文上下文已压缩 {version}]\n\
         {BLOCK_MARKER_RANGE_LABEL}{}\n\
         {selection_note_line}\
         摘要 ID: {summary_id}\n被处理轮次: 轮 {}-{}（{} 轮）\n\
         {ledger_line}\n\
         {pointer_display}\
         摘要存档: {archive_display}\n摘要 digest: sha256:{digest}\n\
         黑板会话: {session_display}\n\
         {locator_lines}\n\
         == 机械摘要行（结构化轨：命令／动作／结果类） ==\n{mechanical_rows}\n\
         {semantic}\
         == 原文回放（按块） ==\n{replay}\n\
         == 模型面声明 ==\n{MODEL_FACE_DECLARATION}\n\
         [/前文上下文已压缩]",
        numbers_text, from, to, rounds,
    )
}

/// 分块截断 marker（T1；`[前文上下文已压缩 v0.4-分块截断]`）。
#[allow(clippy::too_many_arguments)]
pub fn truncation_marker(
    numbers: &[u32],
    rounds_from: usize,
    rounds_to: usize,
    freed_tokens: u64,
    archive_path: &Path,
    locators: &crate::summary::LocatorPointers,
    replay: &str,
    ledger_locators: &[(u32, (u64, u64))],
    pointer_line: &str,
) -> String {
    let numbers_text = render_block_numbers(numbers);
    let count = numbers.len();
    let from = rounds_from + 1;
    let to = rounds_to + 1;
    let version = BLOCK_MARKER_TRUNCATED_VERSION;
    let locator_lines = locators.render_marker_lines(archive_path, "（截断记录，无摘要）");
    let ledger_line = render_ledger_locator_line(ledger_locators);
    let pointer_display = if pointer_line.is_empty() {
        String::new()
    } else {
        format!("{pointer_line}\n")
    };
    let archive_display = archive_path.display().to_string();
    format!(
        "[前文上下文已压缩 {version}]\n\
         {BLOCK_MARKER_RANGE_LABEL}{}\n\
         被截断: {} 块 ≈{freed_tokens}tk token（模型面估算）\n\
         被处理轮次: 轮 {}-{}\n\
         {ledger_line}\n\
         {pointer_display}\
         截断记录存档: {archive_display}\n\
         {locator_lines}\n\
         == 原文回放（按块） ==\n{replay}\n\
         == 模型面声明 ==\n{MODEL_FACE_DECLARATION}\n\
         [/前文上下文已压缩]",
        numbers_text, count, from, to,
    )
}

/// 0cz S2（设计 §5.4）：清零 marker——投影层边界存根。边界之前的全部内容
/// 不再渲染（本 marker 即留在窗口里的那一行存根）；`handover_note` 带交接
/// 写入的落点回执（防裸清护栏的可见面）。
#[allow(clippy::too_many_arguments)]
pub fn clear_marker(
    boundary_round0: usize,
    blocks_cleared: &[u32],
    rounds_cleared: usize,
    archive_path: &Path,
    session: Option<&str>,
    replay: &str,
    handover_note: &str,
) -> String {
    let r = boundary_round0 + 1;
    let numbers_text = if blocks_cleared.is_empty() {
        "（无分块——边界前内容全部在主滑块面）".to_string()
    } else {
        render_block_numbers(blocks_cleared)
    };
    let archive_display = archive_path.display().to_string();
    let session_display = session.unwrap_or("（无）");
    let replay_block = if replay.is_empty() {
        String::new()
    } else {
        format!("== 原文回放（按块） ==\n{replay}\n")
    };
    format!(
        "[前文上下文已清零 {CLEAR_MARKER_VERSION}]\n\
         {CLEAR_MARKER_BOUNDARY_LABEL}{r} 轮]\n\
         已处理分块: {numbers_text}（清零 {rounds_cleared} 轮；读数见清零回执与分块表）\n\
         交接摘要: {handover_note}\n\
         清零存档: {archive_display}（边界前全部轮次逐字留档；read_file offset/limit 分页）\n\
         黑板会话: {session_display}\n\
         {replay_block}\
         你主动发起的清零已生效：边界之前的全部内容已移出当前窗口（**未销毁**，逐字留档可回查）。\
         你的工作计划与结论在黑板（blackboard_read），完整历史在 .gsa 会话记录（读工具可回查）。\
         请从台账继续。\n\
         == 模型面声明 ==\n{MODEL_FACE_DECLARATION}\n\
         [/前文上下文已清零]"
    )
}

/// 0cz S2（设计 §5.2）：清零现场逐字留档（边界前 `[0, upto)` 区间整段
/// markdown——与按块回放互补；模型经 read_file 分页拉回完整前置现场）。
pub fn clear_transcript_markdown(
    messages: &[Message],
    upto: usize,
    boundary_round0: usize,
    run_id: &str,
    session: Option<&str>,
) -> String {
    let mut out = format!(
        "# ORZ 清零现场留档（轮 1–{}）\n\n\
         - 边界: 第 {} 轮（清零发起轮；本档覆盖其之前全部轮次）\n\
         - 消息区间: [0, {upto})\n- run: {run_id}\n\
         - 会话／黑板会话: {}\n- 生成时间: {}\n\n\
         > 陈旧性标注：内容截至轮次 {boundary}；文件可能已变更，编辑前须新鲜读取。\n",
        boundary_round0,
        boundary_round0 + 1,
        session.unwrap_or("（无）"),
        crate::controller::chrono_utc_now(),
        boundary = boundary_round0 + 1,
    );
    for (i, m) in messages[..upto.min(messages.len())].iter().enumerate() {
        let role = match m.role {
            crate::gateway::model::Role::Assistant => "assistant",
            crate::gateway::model::Role::Tool => "tool",
            crate::gateway::model::Role::User => "user",
            crate::gateway::model::Role::System => "system",
        };
        out.push_str(&format!("\n### [{i}] {role}\n"));
        if m.role == crate::gateway::model::Role::Assistant {
            for tc in &m.tool_calls {
                out.push_str(&format!(
                    "[工具调用] {} args={}\n",
                    tc.name,
                    serde_json::to_string(&tc.arguments).unwrap_or_default()
                ));
            }
        }
        if m.role == crate::gateway::model::Role::Tool {
            out.push_str(&format!(
                "（call_id={}）\n",
                m.tool_call_id.as_deref().unwrap_or("?")
            ));
        }
        out.push_str(&m.content);
        out.push('\n');
    }
    out
}

/// 块号区间渲染（压缩、合并连续段：`1-4, 6`）。
pub fn render_block_numbers(numbers: &[u32]) -> String {
    let mut sorted: Vec<u32> = numbers.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    let mut parts: Vec<String> = Vec::new();
    let mut i = 0usize;
    while i < sorted.len() {
        let mut j = i;
        while j + 1 < sorted.len() && sorted[j + 1] == sorted[j] + 1 {
            j += 1;
        }
        parts.push(if i == j {
            format!("{}", sorted[i])
        } else {
            format!("{}-{}", sorted[i], sorted[j])
        });
        i = j + 1;
    }
    parts.join(", ")
}

/// 回放指针文本（块号 → 档案路径；块头附陈旧性标注，设计 §6）。
pub fn replay_line(block: &ContextBlock, path: &str) -> String {
    format!(
        "- 块#{} 轮次 {}-{}: 完整内容见 {}（read_file offset/limit 分页；\
         首次读 `.gsa` 会附一次性 [session_volume_notice] 确认提示，不影响内容读取；\
         内容截至轮次 {}；文件可能已变更，编辑前须新鲜读取）",
        block.number,
        block.first_round + 1,
        block.last_round + 1,
        path,
        block.last_round + 1,
    )
}

/// 一个块的逐字档案正文（分块回放面；同 `.gsa/session/terminal` 先例）。
pub fn block_archive_markdown(
    block: &ContextBlock,
    messages: &[Message],
    run_id: &str,
    session: Option<&str>,
) -> String {
    let ranges = crate::action_ledger::round_ranges(messages);
    let mut out = format!(
        "# ORZ 上下文分块原文（块 #{}）\n\n\
         - 轮次: {}-{}（会话内 1 基）\n- 估算: ≈{}tk token\n- run: {run_id}\n\
         - 会话／黑板会话: {}\n- 生成时间: {}\n\n\
         > 陈旧性标注：内容截至轮次 {}；文件可能已变更，编辑前须新鲜读取。\n",
        block.number,
        block.first_round + 1,
        block.last_round + 1,
        block.estimate_tokens,
        session.unwrap_or("（无）"),
        crate::controller::chrono_utc_now(),
        block.last_round + 1,
    );
    for round in block.first_round..=block.last_round {
        let Some(&(s, e)) = ranges.get(round) else {
            break;
        };
        out.push_str(&format!("\n## 轮次 {}\n", round + 1));
        for m in &messages[s..e] {
            match m.role {
                crate::gateway::model::Role::Assistant => {
                    out.push_str(&format!("\n### assistant（轮次 {}）\n", round + 1));
                    for tc in &m.tool_calls {
                        out.push_str(&format!(
                            "[工具调用] {} args={}\n",
                            tc.name,
                            serde_json::to_string(&tc.arguments).unwrap_or_default()
                        ));
                    }
                }
                crate::gateway::model::Role::Tool => {
                    out.push_str(&format!(
                        "\n### tool（call_id={}）\n",
                        m.tool_call_id.as_deref().unwrap_or("?")
                    ));
                }
                crate::gateway::model::Role::User => {
                    out.push_str("\n### user（注入块或用户输入）\n");
                }
                crate::gateway::model::Role::System => {
                    out.push_str("\n### system\n");
                }
            }
            out.push_str(&m.content);
            out.push('\n');
        }
    }
    out
}

fn truncate_chars(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let mut s: String = text.chars().take(max).collect();
    s.push('…');
    s
}

// ===========================================================================
// 0bz S1（2026-09-28，`GAP-CONTEXT-FACE-TRANSIENT-FORK` / 110 档）：模型面
// **前缀指纹**。逐轮对投影视图做逐消息 sha256 指纹，与上一请求判定**首分歧
// 消息**并落 journal 事件（`face_fingerprint`）——纯观测面：不改请求内容、
// 不进工具面、模型零感知。读数用途＝把前缀缓存 miss 的三源（压缩后第 2 针
// ／空跑 `context_compress` 分叉／自发塌陷）从 token 级推断定位到具体消息
// （0bi §10-④「逐轮模型面前缀指纹」候选的落地；110 立项档 §3/§5）。
//
// 哈希输入＝**wire 同字段**投影（`map_message` 映射的 role/content/
// tool_call_id/tool_calls/reasoning_content 五字段；`round` 不落 wire 故
// 不入哈希——否则内部轮章变化会造成假分歧）。键序经 serde_json BTreeMap
// 规范化（字典序），跨轮次逐字节确定。
// ===========================================================================

/// 单条投影消息的指纹条目。`head` 是诊断面（分歧时落 journal 的头部预览），
/// 仅驻内存、不参与哈希。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FaceFingerprintEntry {
    pub role: &'static str,
    pub chars: usize,
    /// sha256（wire 投影）前 12 hex。
    pub hash: String,
    pub head: String,
}

/// 一次请求视图的逐消息指纹。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FaceFingerprint {
    pub entries: Vec<FaceFingerprintEntry>,
}

/// 首分歧判定结果（与上一请求对比）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FaceDivergence {
    pub index: usize,
    /// `mutated`＝同位置消息内容变了；`inserted`＝尾部追加（位置超出上一
    /// 请求）；`removed`＝尾部收缩（本请求比上一请求短）。
    pub kind: &'static str,
    pub role: &'static str,
    pub chars: usize,
    pub hash: String,
    pub head: String,
    pub prev_chars: Option<usize>,
    pub prev_hash: Option<String>,
    pub prev_head: Option<String>,
}

fn role_label(role: &crate::gateway::model::Role) -> &'static str {
    match role {
        crate::gateway::model::Role::System => "system",
        crate::gateway::model::Role::User => "user",
        crate::gateway::model::Role::Assistant => "assistant",
        crate::gateway::model::Role::Tool => "tool",
    }
}

/// 单消息诊断头（≤120 chars；tool_calls/reasoning 消息给机械摘要，不给正文）。
fn message_head(m: &Message) -> String {
    if !m.content.is_empty() {
        return truncate_chars(&m.content, 120);
    }
    if !m.tool_calls.is_empty() {
        let ids: Vec<&str> = m.tool_calls.iter().map(|t| t.call_id.as_str()).collect();
        return truncate_chars(&format!("[tool_calls: {}]", ids.join(",")), 120);
    }
    if m.tool_call_id.is_some() {
        return "[tool_result]".to_string();
    }
    if m.reasoning_content.is_some() {
        return "[reasoning]".to_string();
    }
    "[empty]".to_string()
}

/// wire 投影哈希（12 hex）：哈希输入与 transport `map_message` 同字段集。
fn message_wire_hash(m: &Message) -> String {
    let projection = serde_json::json!({
        "content": m.content,
        "reasoning_content": m.reasoning_content,
        "role": role_label(&m.role),
        "tool_call_id": m.tool_call_id,
        "tool_calls": m
            .tool_calls
            .iter()
            .map(|tc| serde_json::json!({
                "arguments": tc.arguments,
                "call_id": tc.call_id,
                "name": tc.name,
            }))
            .collect::<Vec<_>>(),
    });
    let bytes = orz_assurance::canonical_json(&projection).unwrap_or_default();
    orz_assurance::sha256_hex(&bytes)[..12].to_string()
}

/// 逐消息指纹（投影视图 → 条目表）。
pub fn face_fingerprint(messages: &[Message]) -> FaceFingerprint {
    let entries = messages
        .iter()
        .map(|m| FaceFingerprintEntry {
            role: role_label(&m.role),
            chars: m.content.chars().count(),
            hash: message_wire_hash(m),
            head: message_head(m),
        })
        .collect();
    FaceFingerprint { entries }
}

impl FaceFingerprint {
    /// 视图内容总字符数（content 面；不含 wire 包装开销）。
    pub fn total_chars(&self) -> usize {
        self.entries.iter().map(|e| e.chars).sum()
    }

    /// 全脸摘要（sha256 前 16 hex）——对逐条 hash 串再哈希，跨轮次对账键。
    pub fn digest(&self) -> String {
        let joined = self
            .entries
            .iter()
            .map(|e| e.hash.as_str())
            .collect::<Vec<_>>()
            .join(";");
        orz_assurance::sha256_hex(joined.as_bytes())[..16].to_string()
    }

    /// 紧凑逐条表：`role:chars:hash12` 用 `;` 连接（journal 事件载荷面）。
    pub fn compact(&self) -> String {
        self.entries
            .iter()
            .map(|e| format!("{}:{}:{}", e.role, e.chars, e.hash))
            .collect::<Vec<_>>()
            .join(";")
    }

    /// 与上一请求的首分歧判定（最长公共哈希前缀 LCP）。
    ///
    /// - `None`＝逐条哈希完全一致（含两者同长）。
    /// - `removed`＝本请求在 LCP 处结束且上一请求更长（尾部收缩）。
    /// - `mutated`＝LCP 处两请求都有消息但哈希不同。
    /// - `inserted`＝LCP 等于上一请求长度（尾部追加；正常轮的尾巴即此形）。
    pub fn first_divergence(&self, prev: &FaceFingerprint) -> Option<FaceDivergence> {
        let lcp = self
            .entries
            .iter()
            .zip(prev.entries.iter())
            .take_while(|(a, b)| a.hash == b.hash)
            .count();
        if lcp == self.entries.len() && lcp == prev.entries.len() {
            return None;
        }
        if lcp == self.entries.len() {
            // 尾部收缩：分歧位置在本请求之外，携带上一请求侧的首条被删消息。
            let prev_e = &prev.entries[lcp];
            return Some(FaceDivergence {
                index: lcp,
                kind: "removed",
                role: prev_e.role,
                chars: 0,
                hash: String::new(),
                head: String::new(),
                prev_chars: Some(prev_e.chars),
                prev_hash: Some(prev_e.hash.clone()),
                prev_head: Some(prev_e.head.clone()),
            });
        }
        let cur_e = &self.entries[lcp];
        if lcp < prev.entries.len() {
            let prev_e = &prev.entries[lcp];
            debug_assert_ne!(cur_e.hash, prev_e.hash, "LCP 是最大公共前缀");
            Some(FaceDivergence {
                index: lcp,
                kind: "mutated",
                role: cur_e.role,
                chars: cur_e.chars,
                hash: cur_e.hash.clone(),
                head: cur_e.head.clone(),
                prev_chars: Some(prev_e.chars),
                prev_hash: Some(prev_e.hash.clone()),
                prev_head: Some(prev_e.head.clone()),
            })
        } else {
            Some(FaceDivergence {
                index: lcp,
                kind: "inserted",
                role: cur_e.role,
                chars: cur_e.chars,
                hash: cur_e.hash.clone(),
                head: cur_e.head.clone(),
                prev_chars: None,
                prev_hash: None,
                prev_head: None,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gateway::model::{Role, ToolCall};

    fn decl(call_id: &str, name: &str, target: &str) -> Message {
        Message {
            role: Role::Assistant,
            content: String::new(),
            tool_call_id: None,
            tool_calls: vec![ToolCall {
                call_id: call_id.to_string(),
                name: name.to_string(),
                arguments: serde_json::json!({ "path": target }),
            }],
            reasoning_content: None,
            round: None,
        }
    }

    fn tool(call_id: &str, content: &str) -> Message {
        Message {
            role: Role::Tool,
            content: content.to_string(),
            tool_call_id: Some(call_id.to_string()),
            tool_calls: Vec::new(),
            reasoning_content: None,
            round: None,
        }
    }

    /// 10 轮 × 约 4K 估算/轮的会话（用于分块与主滑块测试）。
    fn conversation(rounds: usize, per_round_chars: usize) -> Vec<Message> {
        let mut out = vec![Message {
            role: Role::User,
            content: "任务：实现更正批".to_string(),
            tool_call_id: None,
            tool_calls: Vec::new(),
            reasoning_content: None,
            round: None,
        }];
        for r in 0..rounds {
            let id = format!("c{r}");
            out.push(decl(&id, "read_file", &format!("src/f{r}.rs")));
            out.push(tool(&id, &"x".repeat(per_round_chars)));
        }
        out
    }

    /// 0ap S1 钉①（2026-09-18，设计 §6）：读数表与折叠状态**一致性对账**
    /// ——`slider_readout` 的可压缩数/估算必须与 `blocks_outside_slider` ∩
    /// `face_markers`（closed ∧ Live，即 `compress_blocks_now` 同一可压缩
    /// 判定）逐块对账；压缩 marker 落地后读数恰降对应块；整段在滑块内 ⇒
    /// 全零读数。读时现算（零新增记账）的对账钉。
    #[test]
    fn slider_readout_matches_block_and_marker_state() {
        let messages = conversation(10, 8_000); // ≈4K 估算/轮
        let blocks = blocks_outside_slider(&messages, 12_000, 12_000);
        let markers = face_markers(&messages);
        let compressible: Vec<&ContextBlock> = blocks
            .iter()
            .filter(|b| b.closed && markers.state(b.number) == BlockState::Live)
            .collect();
        assert!(compressible.len() >= 2, "fixture needs several blocks");

        let readout = slider_readout(&messages, 12_000, 12_000);
        assert_eq!(readout.compressible_blocks, compressible.len());
        assert_eq!(
            readout.compressible_estimate_tokens,
            compressible.iter().map(|b| b.estimate_tokens).sum::<u64>()
        );
        assert_eq!(readout.total_blocks, blocks.len());

        // 块 1 压缩落地（marker 反解）⇒ 读数恰降块 1 的数量与估算。
        let first = compressible[0];
        let mut compressed = messages.clone();
        compressed.push(Message {
            role: Role::User,
            content: format!(
                "[前文上下文已压缩 {}]\n已处理分块: {}",
                BLOCK_MARKER_COMPRESSED_VERSION, first.number
            ),
            tool_call_id: None,
            tool_calls: Vec::new(),
            reasoning_content: None,
            round: None,
        });
        let after = slider_readout(&compressed, 12_000, 12_000);
        assert_eq!(after.compressible_blocks, readout.compressible_blocks - 1);
        assert_eq!(
            after.compressible_estimate_tokens,
            readout.compressible_estimate_tokens - first.estimate_tokens
        );
        assert_eq!(after.total_blocks, readout.total_blocks);

        // 整段放得下滑块 ⇒ 全零读数（NothingToCompress 的账面依据）。
        let small = conversation(3, 1_000);
        let empty = slider_readout(&small, 160_000, 32_000);
        assert_eq!(
            empty,
            SliderReadout {
                compressible_blocks: 0,
                compressible_estimate_tokens: 0,
                total_blocks: 0,
            }
        );
    }

    #[test]
    fn blocks_are_numbered_from_the_conversation_start_and_stay_stable() {
        let messages = conversation(10, 8_000); // ≈4K 估算/轮
        let blocks = blocks_outside_slider(&messages, 12_000, 12_000);
        assert!(blocks.len() >= 2, "expected several blocks: {blocks:?}");
        assert_eq!(blocks[0].number, 1);
        assert_eq!(blocks[0].first_round, 0);
        assert_eq!(blocks[1].number, 2);
        assert_eq!(blocks[1].first_round, blocks[0].last_round + 1);
        // 闭合性：除最后一块外全部闭合（末块仍在增长 ⇒ 不可压缩）。
        assert!(blocks[..blocks.len() - 1].iter().all(|b| b.closed));
        // 追加一轮后，既有块号与首轮不变（锚在起点 ⇒ 去重键稳定）。
        let mut grown = messages.clone();
        grown.push(decl("c9b", "read_file", "src/f9b.rs"));
        grown.push(tool("c9b", &"x".repeat(8_000)));
        let grown_blocks = blocks_outside_slider(&grown, 12_000, 12_000);
        assert!(grown_blocks.len() >= blocks.len());
        for (before, after) in blocks.iter().zip(grown_blocks.iter()) {
            assert_eq!(before.number, after.number);
            assert_eq!(before.first_round, after.first_round);
        }
    }

    #[test]
    fn no_blocks_when_the_whole_conversation_fits_the_slider() {
        let messages = conversation(3, 1_000);
        assert!(blocks_outside_slider(&messages, 160_000, 32_000).is_empty());
        let params = ModelFaceParams {
            slider_tokens: 160_000,
            block_tokens: 32_000,
            ledger_path: None,
            archive_tag: None,
            run_id: "RUN-TEST".to_string(),
            d4_block: None,
            static_overhead_tokens: 0,
        };
        // 模型面＝原文（字节级一致）；阶梯量尺＝全量估算。
        assert_eq!(
            build_model_face(&messages, &params).expect("face"),
            messages
        );
        assert_eq!(
            model_face_estimate(&messages, &params),
            crate::controller::estimate_messages_tokens(&messages)
        );
    }

    #[test]
    fn resident_head_from_first_round_and_opening_keeps_prefix_byte_stable() {
        // 0bz S3 钉①（2026-09-28；机理实证＝`RUN-CLI-6ab99969` r40：旧口径
        // 指针/D4 拖到首个分块形成才插入 face 前部 ⇒ 开窗轮整窗 miss
        // 135,754 tk）：指针自首个轮次即在 face[1]、D4 居窗口尾（0bz S3′
        // 移位后口径）；首个分块形成（开窗）时 face 前缀逐字节不变，只有
        // 尾部追加（块表／RUN_END）。
        let params = || ModelFaceParams {
            slider_tokens: 160_000,
            block_tokens: 32_000,
            ledger_path: Some(std::path::PathBuf::from(".gsa/ledger/current.md")),
            archive_tag: Some("sess0001".to_string()),
            run_id: "RUN-TEST".to_string(),
            d4_block: Some("D4 机械段（0bz S3 钉）".to_string()),
            static_overhead_tokens: 0,
        };
        let mut messages = conversation(3, 1_000);
        let early = build_model_face(&messages, &params()).expect("face");
        assert_eq!(early[0], messages[0], "题面恒为 face[0]");
        assert!(
            early[1]
                .content
                .contains(crate::action_ledger::LEDGER_FOLD_POINTER_PREFIX),
            "指针必须自首个轮次即在 face[1]"
        );
        assert_eq!(
            early.last().expect("non-empty").content,
            "D4 机械段（0bz S3 钉）",
            "0bz S3′：D4 落窗口尾（常驻头只留指针）"
        );
        // 估算/计数同口径（常驻头计入）。
        assert!(
            model_face_estimate(&messages, &params())
                > crate::controller::estimate_messages_tokens(&messages)
        );
        assert_eq!(
            model_face_message_count(&messages, &params()),
            messages.len() + 2
        );
        // 长到溢出滑块 ⇒ 首个分块形成（开窗转换）。
        for _ in 0..60 {
            messages.extend(conversation(1, 8_000).into_iter().skip(1));
        }
        assert!(
            !blocks_outside_slider(&messages, 160_000, 32_000).is_empty(),
            "测试构造必须已形成首个分块"
        );
        let opened = build_model_face(&messages, &params()).expect("face");
        assert_eq!(
            early[..2],
            opened[..2],
            "开窗转换不得改写 face 前缀（题面＋指针）"
        );
        let d4_at = opened
            .iter()
            .position(|m| m.content == "D4 机械段（0bz S3 钉）")
            .expect("D4 必在开窗 face 内");
        assert!(
            d4_at >= opened.len() - 3,
            "0bz S3′：D4 开窗后仍居尾部（块表/结束行之前），实际 index {d4_at}/{}",
            opened.len()
        );
        assert!(opened.len() > early.len(), "开窗只在尾部追加结构行");
    }

    #[test]
    fn d4_rerender_diverges_only_at_its_tail_slot() {
        // 0bz S3′ 钉（2026-10-04）：D4 重渲（epoch 更新）只重价尾部自身——
        // 题面、指针、marker 与全部历史逐字节稳定（「第 2 针」机械锁：
        // recli 四跑离线对账实证 9/9 压缩后 +2 在 D4 头部槽全前缀重价，
        // 移尾后该重价面收敛到 D4 槽本身）。本 fixture 无压缩 marker；
        // marker 在场的开窗形状由变体钉
        // `d4_rerender_diverges_only_at_its_tail_slot_with_marker` 覆盖。
        let mut messages = conversation(2, 1_000);
        for _ in 0..60 {
            messages.extend(conversation(1, 8_000).into_iter().skip(1));
        }
        assert!(
            !blocks_outside_slider(&messages, 160_000, 32_000).is_empty(),
            "钉在开窗路径（已成块）"
        );
        let params = |d4: &str| ModelFaceParams {
            slider_tokens: 160_000,
            block_tokens: 32_000,
            ledger_path: Some(std::path::PathBuf::from(".gsa/ledger/current.md")),
            archive_tag: Some("sess0001".to_string()),
            run_id: "RUN-TEST".to_string(),
            d4_block: Some(d4.to_string()),
            static_overhead_tokens: 0,
        };
        let before = build_model_face(&messages, &params("D4 旧 epoch")).expect("face");
        let after = build_model_face(&messages, &params("D4 新 epoch ×2")).expect("face");
        assert_eq!(before.len(), after.len(), "D4 重渲不改面长度");
        let div = before
            .iter()
            .zip(after.iter())
            .position(|(a, b)| a.content != b.content)
            .expect("D4 内容差异必现");
        assert!(
            div >= before.len() - 3,
            "分歧必须落在尾部 D4 槽（实际 index {div}/{}）",
            before.len()
        );
        assert_eq!(after[div].content, "D4 新 epoch ×2");
        assert_eq!(
            &before[..div],
            &after[..div],
            "D4 重渲不得改写其前任何字节（本 fixture 无 marker；marker 在场形状见变体钉）"
        );
    }

    #[test]
    fn d4_rerender_diverges_only_at_its_tail_slot_with_marker() {
        // 0bz S3′ 钉变体（2026-10-04 审查处置批）：生产第 2 针的准确形状＝
        // marker 在场的开窗脸（压缩落地后的 +2 请求）。marker 作为历史区
        // 普通消息由逐字节前缀比较覆盖；本钉显式断言 fixture 已成块（开窗
        // 路径）且 marker 确在 face 内，重渲分歧仍只落尾部 D4 槽。
        let params = |d4: &str| ModelFaceParams {
            slider_tokens: 160_000,
            block_tokens: 32_000,
            ledger_path: Some(std::path::PathBuf::from(".gsa/ledger/current.md")),
            archive_tag: Some("sess0001".to_string()),
            run_id: "RUN-TEST".to_string(),
            d4_block: Some(d4.to_string()),
            static_overhead_tokens: 0,
        };
        let mk_marker = |n: u32, spec: &str| -> String {
            format!(
                "{} {}]
{}{}
摘要 ID: compaction-RUN-TEST-{n:03}
台账定位: .gsa/ledger/current.md
",
                crate::prompt::CONTEXT_COMPRESSED_PREFIX,
                BLOCK_MARKER_COMPRESSED_VERSION,
                BLOCK_MARKER_RANGE_LABEL,
                spec
            )
        };
        let mut messages = conversation(60, 8_000);
        // 压缩落地：marker 插入块 1 首轮起点（生产 insert_at 口径）。
        let blocks = blocks_outside_slider(&messages, 160_000, 32_000);
        let ranges = crate::action_ledger::round_ranges(&messages);
        let b1 = blocks.first().expect("block1");
        let at = ranges[b1.first_round].0;
        messages.insert(at, mechanical_message(mk_marker(1, "1")));
        assert!(
            !blocks_outside_slider(&messages, 160_000, 32_000).is_empty(),
            "钉在开窗路径（已成块）"
        );
        let before = build_model_face(&messages, &params("D4 旧 epoch")).expect("face");
        let after = build_model_face(&messages, &params("D4 新 epoch ×2")).expect("face");
        assert!(
            before
                .iter()
                .any(|m| m.content.contains("compaction-RUN-TEST-001")),
            "marker 必在 face 内（生产第 2 针的准确形状）"
        );
        assert_eq!(before.len(), after.len(), "D4 重渲不改面长度");
        let div = before
            .iter()
            .zip(after.iter())
            .position(|(a, b)| a.content != b.content)
            .expect("D4 内容差异必现");
        assert!(
            div >= before.len() - 3,
            "分歧必须落在尾部 D4 槽（实际 index {div}/{}）",
            before.len()
        );
        assert_eq!(after[div].content, "D4 新 epoch ×2");
        assert_eq!(
            &before[..div],
            &after[..div],
            "marker 在场时 D4 重渲同样不得改写其前任何字节（含 marker 与历史）"
        );
    }

    #[test]
    fn compression_landing_marker_is_byte_stable_across_next_landing() {
        // 0bz S3 修码②钉（2026-09-28）：连续两次压缩落地——marker1 字节与
        // 指针位在 marker2 落地后不变（+1→+2 前缀纪律的机械锁；生产几何＝
        // marker 插入被压首轮起点、face 装配自动隐藏被压块，D4 重渲与
        // marker 同拍落在 +1——本轮面已无 +1→+2 二次写手，本钉防回退）。
        let params = || ModelFaceParams {
            slider_tokens: 160_000,
            block_tokens: 32_000,
            ledger_path: Some(std::path::PathBuf::from(".gsa/ledger/current.md")),
            archive_tag: Some("sess0001".to_string()),
            run_id: "RUN-TEST".to_string(),
            d4_block: Some("D4 机械段（0bz S3 钉）".to_string()),
            static_overhead_tokens: 0,
        };
        let mk_marker = |n: u32, spec: &str| -> String {
            format!(
                "{} {}]
{}{}
摘要 ID: compaction-RUN-TEST-{n:03}
台账定位: .gsa/ledger/current.md
",
                crate::prompt::CONTEXT_COMPRESSED_PREFIX,
                BLOCK_MARKER_COMPRESSED_VERSION,
                BLOCK_MARKER_RANGE_LABEL,
                spec
            )
        };
        let mut messages = conversation(60, 8_000);
        // 第一次落地：压块 1（marker 插入块 1 首轮起点＝生产 insert_at 口径）。
        let blocks1 = blocks_outside_slider(&messages, 160_000, 32_000);
        let ranges1 = crate::action_ledger::round_ranges(&messages);
        let b1 = blocks1.first().expect("block1");
        let at1 = ranges1[b1.first_round].0;
        messages.insert(at1, mechanical_message(mk_marker(1, "1")));
        let face1 = build_model_face(&messages, &params()).expect("face1");
        let m1 = face1
            .iter()
            .position(|m| m.content.contains("compaction-RUN-TEST-001"))
            .expect("marker1 在 face1");
        // 第二次落地：压块 2（ranges/blocks 按落地后的 messages 重算）。
        let blocks2 = blocks_outside_slider(&messages, 160_000, 32_000);
        let b2 = blocks2.iter().find(|b| b.number == 2).expect("block2");
        let ranges2 = crate::action_ledger::round_ranges(&messages);
        let at2 = ranges2[b2.first_round].0;
        messages.insert(at2, mechanical_message(mk_marker(2, "2")));
        let face2 = build_model_face(&messages, &params()).expect("face2");
        // 断言：marker1 字节与位置不变、指针位不动、前缀（到指针位）逐字节一致。
        let m1b = face2
            .iter()
            .position(|m| m.content.contains("compaction-RUN-TEST-001"))
            .expect("marker1 在 face2 存续");
        assert_eq!(m1, m1b, "marker1 的 face 位置不得移动");
        assert_eq!(face1[m1], face2[m1b], "marker1 必须逐字节稳定");
        let p1 = face1
            .iter()
            .position(|m| {
                m.content
                    .contains(crate::action_ledger::LEDGER_FOLD_POINTER_PREFIX)
            })
            .expect("指针在 face1");
        let p2 = face2
            .iter()
            .position(|m| {
                m.content
                    .contains(crate::action_ledger::LEDGER_FOLD_POINTER_PREFIX)
            })
            .expect("指针在 face2");
        assert_eq!(p1, p2, "指针位在第二次落地后不动");
        assert_eq!(&face1[..=p1], &face2[..=p1], "+1→+2 前缀逐字节稳定");
    }

    #[test]
    fn model_face_hides_compressed_blocks_and_keeps_the_slider_verbatim() {
        let messages = conversation(10, 8_000);
        let params = ModelFaceParams {
            slider_tokens: 12_000,
            block_tokens: 12_000,
            ledger_path: None,
            archive_tag: Some("sess0001".to_string()),
            run_id: "RUN-TEST".to_string(),
            d4_block: None,
            static_overhead_tokens: 0,
        };
        let blocks = blocks_outside_slider(&messages, 12_000, 12_000);
        let mut with_marker = messages.clone();
        with_marker.push(Message {
            role: Role::User,
            content: compression_marker(
                &[1],
                blocks[0].first_round,
                blocks[0].last_round,
                "compaction-RUN-TEST-0001",
                "0",
                Path::new(".gsa/compaction/x.md"),
                Some("sess"),
                "[1] 轮次 1: read_file 目标=src/f0.rs",
                None,
                "",
                "- 块#1: 完整内容见 .gsa/compaction/blocks/block-0001.md",
                &crate::summary::LocatorPointers::default(),
                &[(1, (12, 25))],
                "指针（回查：blackboard_read section=journal anchor=）: r1·b1·s12",
            ),
            tool_call_id: None,
            tool_calls: Vec::new(),
            reasoning_content: None,
            round: None,
        });
        let face = build_model_face(&with_marker, &params).expect("face");
        // 块 1 的**原文消息**不在模型面里（分块表可能提到目标名，故按消息
        // 身份断言：块 1 首轮的工具回复 `call_id=c0` 与其声明都不在场）。
        assert!(
            !face.iter().any(|m| m.tool_call_id.as_deref() == Some("c0")),
            "compressed block tool result must leave the model face"
        );
        // marker 本身留在模型面里（压缩后的摘要行）。
        assert!(face.iter().any(|m| is_block_marker(&m.content)));
        // 本地面零覆盖：`messages` 一条不少（只多出 marker）。
        assert_eq!(with_marker.len(), messages.len() + 1);
        // 主滑块内容逐字保留（最后一轮原文仍在）。
        assert!(face.iter().any(|m| m.content.contains(&"x".repeat(8_000))));
    }

    /// **0bc S2④ 构造性注入（2026-09-21）**：装配路径分配失败＝`OutOfMemory`
    /// 上抛（不 abort、不 panic）；且计数面与物化面逐条同口径（1398 的替代
    /// 不得漂移）。
    #[test]
    fn model_face_alloc_failure_maps_to_out_of_memory_and_count_is_consistent() {
        let messages = conversation(10, 8_000);
        let params = ModelFaceParams {
            slider_tokens: 12_000,
            block_tokens: 12_000,
            ledger_path: None,
            archive_tag: Some("sess0001".to_string()),
            run_id: "RUN-TEST".to_string(),
            d4_block: None,
            static_overhead_tokens: 0,
        };
        // 注入一次：装配必须上抛 OutOfMemory（不 abort）。
        FORCE_ALLOC_FAILURE.with(|flag| flag.set(true));
        let err = build_model_face(&messages, &params).expect_err("注入的分配失败必须上抛");
        assert_eq!(err.kind(), std::io::ErrorKind::OutOfMemory);
        // 注入只消费一次；常规装配恢复，且计数面与物化面一致。
        let face = build_model_face(&messages, &params).expect("face");
        assert_eq!(
            model_face_message_count(&messages, &params),
            face.len(),
            "计数必须与物化面一致"
        );
    }

    #[test]
    fn block_table_lists_state_and_replay_pointer() {
        let messages = conversation(10, 8_000);
        let params = ModelFaceParams {
            slider_tokens: 12_000,
            block_tokens: 12_000,
            ledger_path: None,
            archive_tag: Some("sess0001".to_string()),
            run_id: "RUN-TEST".to_string(),
            d4_block: None,
            static_overhead_tokens: 0,
        };
        let blocks = blocks_outside_slider(&messages, 12_000, 12_000);
        let mut markers = FaceMarkers::default();
        markers.compressed.insert(1);
        markers.truncated.insert(2);
        let table = render_block_table(&blocks, &markers, &params);
        assert!(table.starts_with(BLOCK_TABLE_PREFIX));
        assert!(table.contains("[块#1]"));
        assert!(table.contains("已压缩"));
        assert!(table.contains("已截断"));
        assert!(table.contains("原文回放:"));
        assert!(table.contains("run=RUN-TEST"));
    }

    /// 0bs ① 钉（2026-09-25）：结束自述通道**常驻尾行**——有分块时每轮随
    /// 分块表进模型面（表之后＝窗口最末消息）；无分块会话不注入（早期会话
    /// 字节形态不变量保持）；同一面内分块表只出现一次（0bs ④ 单源口径）。
    #[test]
    fn end_channel_resident_line_rides_the_face_tail_after_the_block_table() {
        let messages = conversation(10, 8_000);
        let params = ModelFaceParams {
            slider_tokens: 12_000,
            block_tokens: 12_000,
            ledger_path: None,
            archive_tag: Some("sess0001".to_string()),
            run_id: "RUN-TEST".to_string(),
            d4_block: None,
            static_overhead_tokens: 0,
        };
        let face = build_model_face(&messages, &params).expect("face");
        let tail = face.last().expect("非空模型面");
        assert_eq!(
            tail.content,
            crate::model_stop::model_stop_resident_line(),
            "尾行＝结束自述通道（单一来源）"
        );
        let before = &face[face.len() - 2];
        assert!(
            before.content.starts_with(BLOCK_TABLE_PREFIX),
            "通道行紧随分块表之后：{}",
            before.content
        );
        assert_eq!(
            face.iter()
                .filter(|m| m.content.starts_with(BLOCK_TABLE_PREFIX))
                .count(),
            1,
            "同一模型面内分块表只出现一次"
        );
        // 无分块会话：原样返回（不注入通道行）。
        let small = conversation(3, 1_000);
        let small_params = ModelFaceParams {
            slider_tokens: 160_000,
            block_tokens: 32_000,
            ledger_path: None,
            archive_tag: None,
            run_id: "RUN-TEST".to_string(),
            d4_block: None,
            static_overhead_tokens: 0,
        };
        assert_eq!(
            build_model_face(&small, &small_params).expect("face"),
            small
        );
    }

    /// **P0 修复钉子（2026-09-16 实现批，审查 R-1）**：未闭合的**残段**
    /// ① 声明的轮区间必须**止于工作现场（主滑块）之前**——旧实现写成
    /// `ranges.len()-1`（会话末轮），会把主滑块及其后所有新轮次圈进隐藏区间；
    /// ② 即便脏 marker 把残段点名「已截断」，主滑块逐字仍必须留在模型面
    /// （`hidden_message_ranges` 的两道护栏：未闭合不隐藏 ＋ 区间钳到滑块起点）。
    #[test]
    fn open_residual_stops_before_the_slider_and_is_never_hidden() {
        let messages = conversation(12, 8_000); // ≈4K 估算/轮
        let params = ModelFaceParams {
            slider_tokens: 12_000,
            // y=25K ⇒ 块 1 在第 7 轮闭合，其后 2 轮成为**未闭合残段**（覆盖
            // 「不足 y 的尾部」这一常态，而不是恰好闭合的边界特例）。
            block_tokens: 25_000,
            ledger_path: None,
            archive_tag: Some("sess0001".to_string()),
            run_id: "RUN-RESIDUAL".to_string(),
            d4_block: None,
            static_overhead_tokens: 0,
        };
        let blocks = blocks_outside_slider(&messages, 12_000, 25_000);
        let residual = blocks.last().expect("应存在未闭合残段");
        assert!(!residual.closed);
        let ranges = crate::action_ledger::round_ranges(&messages);
        let slider = slider_start(&messages, 12_000).expect("应存在主滑块起点");
        assert!(
            ranges[residual.last_round].1 <= slider,
            "残段声明轮区间不得越入主滑块：residual=轮 {}-{} 滑块起点={slider}",
            residual.first_round + 1,
            residual.last_round + 1
        );
        // 脏 marker：把残段点名截断。
        let mut poisoned = messages.clone();
        poisoned.push(Message {
            role: Role::User,
            content: truncation_marker(
                &[residual.number],
                residual.first_round,
                residual.last_round,
                0,
                Path::new(".gsa/compaction/x.md"),
                &crate::summary::LocatorPointers::default(),
                "- 块#1 …",
                &[],
                "",
            ),
            tool_call_id: None,
            tool_calls: Vec::new(),
            reasoning_content: None,
            round: None,
        });
        let face = build_model_face(&poisoned, &params).expect("face");
        let newest = messages.last().expect("末轮消息");
        assert!(
            face.iter().any(|m| m.content == newest.content),
            "主滑块逐字不得被隐藏（I1 硬护栏）"
        );
        // 残段本身仍在场（未闭合 ⇒ 不隐藏、不可压缩、不可截断）。
        assert!(
            face.iter()
                .any(|m| m.tool_call_id.as_deref() == Some("c11")),
            "残段与工作现场的工具回复都必须在场"
        );
    }

    #[test]
    fn markers_round_trip_block_numbers_and_never_get_hidden() {
        let marker = compression_marker(
            &[1, 2, 3, 5],
            0,
            4,
            "id",
            "digest",
            Path::new("a.md"),
            None,
            "rows",
            Some("[SEMANTIC_SUMMARY] 目标: x"),
            "",
            "- 块#1 …",
            &crate::summary::LocatorPointers::default(),
            &[(1, (12, 25)), (2, (26, 40))],
            "指针（回查：blackboard_read section=journal anchor=）: r1·b1·s12；r1·b2·s12",
        );
        let messages = vec![Message {
            role: Role::User,
            content: marker.clone(),
            tool_call_id: None,
            tool_calls: Vec::new(),
            reasoning_content: None,
            round: None,
        }];
        let markers = face_markers(&messages);
        assert_eq!(
            markers.compressed.iter().copied().collect::<Vec<_>>(),
            vec![1, 2, 3, 5]
        );
        // 三键：台账 `[seq]` 由 marker 反解（块号去重后仍逐块可查）。
        assert_eq!(markers.ledger_range(1), Some((12, 25)));
        assert_eq!(markers.ledger_range(2), Some((26, 40)));
        assert_eq!(markers.ledger_range(3), None);
        assert!(is_block_marker(&marker));
        assert_eq!(render_block_numbers(&[5, 1, 3, 2, 3]), "1-3, 5");
        // 历史 marker（无 v8 版本号/块号行）不参与状态反解。
        assert!(parse_marker_blocks("[前文上下文已压缩 v0.3]\n内容").is_none());
        assert!(parse_marker_blocks("普通对话").is_none());
    }

    #[test]
    fn truncation_marker_marks_blocks_idempotently() {
        let marker = truncation_marker(
            &[1, 2],
            0,
            3,
            88_000,
            Path::new("a.md"),
            &crate::summary::LocatorPointers::default(),
            "- 块#1 …",
            &[],
            "",
        );
        let messages = vec![Message {
            role: Role::User,
            content: marker,
            tool_call_id: None,
            tool_calls: Vec::new(),
            reasoning_content: None,
            round: None,
        }];
        let markers = face_markers(&messages);
        assert_eq!(markers.state(1), BlockState::Truncated);
        assert_eq!(markers.state(3), BlockState::Live);
        // 幂等：重复出现同一 marker 不改变状态（块号去重）。
        let mut twice = messages.clone();
        twice.extend(messages.clone());
        let markers = face_markers(&twice);
        assert_eq!(markers.truncated.len(), 2);
    }

    // --- 0bz S1（2026-09-28）：模型面前缀指纹钉子（先红后绿钉的机械面） ---

    fn user_msg(content: &str) -> Message {
        Message {
            role: Role::User,
            content: content.to_string(),
            tool_call_id: None,
            tool_calls: Vec::new(),
            reasoning_content: None,
            round: None,
        }
    }

    #[test]
    fn face_fingerprint_same_wire_same_hash_and_round_field_excluded() {
        let a = face_fingerprint(&[user_msg("前文内容"), tool("c1", "结果")]);
        let b = face_fingerprint(&[user_msg("前文内容"), tool("c1", "结果")]);
        assert_eq!(a, b, "同 wire 内容必须同指纹");
        // `round` 不落 wire（transport map_message 剔除）⇒ 改轮章不得改哈希。
        let mut renumbered = tool("c1", "结果");
        renumbered.round = Some(7);
        let c = face_fingerprint(&[user_msg("前文内容"), renumbered]);
        assert_eq!(a.entries[1].hash, c.entries[1].hash, "round 入哈希＝假分歧");
        // 内容变 ⇒ 哈希变。
        let d = face_fingerprint(&[user_msg("前文内容!"), tool("c1", "结果")]);
        assert_ne!(a.entries[0].hash, d.entries[0].hash);
        // tool_calls（arguments）入哈希。
        let e = face_fingerprint(&[decl("c1", "read_file", "a.txt"), user_msg("x")]);
        let f = face_fingerprint(&[decl("c1", "read_file", "b.txt"), user_msg("x")]);
        assert_ne!(e.entries[0].hash, f.entries[0].hash, "arguments 入哈希");
    }

    #[test]
    fn face_fingerprint_digest_and_compact_shape() {
        let fp = face_fingerprint(&[user_msg("hello"), tool("c9", "done")]);
        assert_eq!(fp.digest().len(), 16);
        assert!(fp.digest().bytes().all(|b| b.is_ascii_hexdigit()));
        assert_eq!(
            fp.compact(),
            format!(
                "user:5:{};tool:4:{}",
                fp.entries[0].hash, fp.entries[1].hash
            )
        );
        assert_eq!(fp.total_chars(), 9);
        // head 诊断面：tool_calls 消息给机械摘要而非空串。
        let d = face_fingerprint(&[decl("c1", "read_file", "a.txt")]);
        assert!(d.entries[0].head.starts_with("[tool_calls: c1]"));
    }

    #[test]
    fn first_divergence_insert_mutate_remove_and_none() {
        let base = face_fingerprint(&[user_msg("a"), user_msg("b"), user_msg("c")]);
        // 无分歧。
        assert!(
            base.first_divergence(&face_fingerprint(&[
                user_msg("a"),
                user_msg("b"),
                user_msg("c")
            ]))
            .is_none()
        );
        // 正常轮尾巴＝尾部追加两条（decl+tool）⇒ inserted at index=3。
        let grown = face_fingerprint(&[
            user_msg("a"),
            user_msg("b"),
            user_msg("c"),
            decl("c1", "read_file", "a.txt"),
            tool("c1", "内容"),
        ]);
        let div = grown
            .first_divergence(&base)
            .expect("appended tail must diverge");
        assert_eq!(div.kind, "inserted");
        assert_eq!(div.index, 3);
        assert!(div.prev_hash.is_none());
        // 早期消息被改（空跑 compress / 自发塌陷的候选形）⇒ mutated at 早期位。
        let mutated = face_fingerprint(&[
            user_msg("a"),
            user_msg("B!"),
            user_msg("c"),
            decl("c1", "read_file", "a.txt"),
            tool("c1", "内容"),
        ]);
        let div = mutated
            .first_divergence(&base)
            .expect("early mutation must diverge");
        assert_eq!(div.kind, "mutated");
        assert_eq!(div.index, 1);
        assert_eq!(
            div.prev_hash.as_deref(),
            Some(base.entries[1].hash.as_str())
        );
        assert_eq!(div.prev_chars, Some(1));
        // 尾部收缩（压缩落地后的新窗口更短）⇒ removed。
        let shrunk = face_fingerprint(&[user_msg("a")]);
        let div = shrunk
            .first_divergence(&base)
            .expect("shrunk tail must diverge");
        assert_eq!(div.kind, "removed");
        assert_eq!(div.index, 1);
        assert_eq!(
            div.prev_hash.as_deref(),
            Some(base.entries[1].hash.as_str())
        );
    }

    // ===== 0cz S2（2026-10-11，设计 §5）钉组：模型主动清零（投影层第三动作）
    // ===== 钉②（本地面零 diff / 隐藏口径）＋钉③（marker 反解与恢复重建）

    /// 构造「6 轮 ＋ 边界 marker 插在第 5 轮声明前」的会话（boundary_round0=4
    /// ⇒ 清零边界＝第 5 轮，轮 1–4 隐藏）。
    fn cleared_conversation() -> Vec<Message> {
        let mut messages = conversation(6, 8_000); // ≈4K 估算/轮
        let marker = clear_marker(
            4,
            &[1, 2],
            4,
            Path::new(".gsa/compaction/blocks/tag-clear.md"),
            Some("sess-abcd"),
            "- 块#1 轮次 1-2: 完整内容见 .gsa/compaction/blocks/tag-block-0001.md\n\
             - 块#2 轮次 3-4: 完整内容见 .gsa/compaction/blocks/tag-block-0002.md",
            "黑板 notes r5@normal（【交接摘要】，42 字符）",
        );
        let idx = messages
            .iter()
            .position(|m| {
                m.role == Role::Assistant && m.tool_calls.iter().any(|tc| tc.call_id == "c4")
            })
            .expect("round 5 declaration");
        messages.insert(
            idx,
            Message {
                role: Role::User,
                content: marker,
                tool_call_id: None,
                tool_calls: Vec::new(),
                reasoning_content: None,
                round: None,
            },
        );
        messages
    }

    #[test]
    fn clear_marker_parses_and_boundary_hides_preboundary_rounds() {
        let messages = cleared_conversation();
        let markers = face_markers(&messages);
        // 钉③（反解）：边界轮号与「最后 marker 优先」语义。
        assert_eq!(markers.cleared_before_round, Some(5));
        assert!(markers.is_round_cleared(0));
        assert!(!markers.is_round_cleared(4), "边界轮本身不清零");
        // 注册面：注入文本（绝不持久化）＋ restore-retained（恢复重建）。
        let marker_content = messages
            .iter()
            .find(|m| m.content.starts_with(CLEAR_MARKER_PREFIX))
            .unwrap()
            .content
            .clone();
        assert!(crate::prompt::is_injected_block_text(&marker_content));
        assert!(crate::prompt::is_restore_retained_block(&marker_content));

        let params = ModelFaceParams {
            slider_tokens: 12_000,
            block_tokens: 12_000,
            ledger_path: None,
            archive_tag: None,
            run_id: "RUN-CZ".to_string(),
            d4_block: None,
            static_overhead_tokens: 0,
        };
        // 钉②（装配口径）：边界前（含原 prompt 前言）全部隐藏；边界轮起逐字
        // 保留；marker 本体在窗口（存根）。
        let face = build_model_face(&messages, &params).unwrap();
        assert!(
            face.iter()
                .any(|m| m.content.starts_with(CLEAR_MARKER_PREFIX)),
            "marker 存根必须在窗口"
        );
        assert!(
            !face.iter().any(|m| {
                m.role == Role::Assistant && m.tool_calls.iter().any(|tc| tc.call_id == "c0")
            }),
            "边界前声明必须隐藏（分块表的指针列不算泄漏——那是合法的回放面）"
        );
        assert!(
            !face.iter().any(|m| {
                m.role == Role::Assistant && m.tool_calls.iter().any(|tc| tc.call_id == "c3")
            }),
            "边界前最后一轮声明必须隐藏"
        );
        assert!(
            !face.iter().any(|m| m.content.contains("任务：实现更正批")),
            "边界前前言（原 prompt）必须隐藏"
        );
        assert!(
            face.iter().any(|m| {
                m.role == Role::Assistant && m.tool_calls.iter().any(|tc| tc.call_id == "c4")
            }),
            "边界轮起逐字保留"
        );
        assert!(
            face.iter().any(|m| {
                m.role == Role::Assistant && m.tool_calls.iter().any(|tc| tc.call_id == "c5")
            }),
            "边界后轮次逐字保留"
        );
        // 钉②（估算口径）：清零面估算 ＜ 无 marker 同会话（坍缩到近静态）；
        // 且与「face 内容总量」同向（不物化路径与装配路径一致）。
        let est_cleared = estimate_model_face_tokens(&messages, &params);
        let open = conversation(6, 8_000);
        let est_open = estimate_model_face_tokens(&open, &params);
        assert!(
            est_cleared * 2 < est_open,
            "清零后估算必须坍缩（{est_cleared} vs {est_open}）"
        );
        // 块表：边界前块渲染「已清零」；读数：清零块不进可压集合。
        let blocks = blocks_outside_slider(&messages, 12_000, 12_000);
        assert!(!blocks.is_empty());
        let table = render_block_table(&blocks, &markers, &params);
        assert!(table.contains("已清零"), "{table}");
        let readout = slider_readout(&messages, 12_000, 12_000);
        assert_eq!(readout.compressible_blocks, 0, "{readout:?}");
        // 钉③（恢复重建）：同一消息数组原样重放（模拟侧车恢复）⇒ 同一面。
        let restored = build_model_face(&messages, &params).unwrap();
        assert_eq!(face.len(), restored.len());
    }

    #[test]
    fn second_clear_marker_supersedes_first_boundary() {
        let mut messages = cleared_conversation();
        // 第二次清零（边界＝第 6 轮，0 基 5）——最后 marker 优先。
        let idx = messages
            .iter()
            .position(|m| {
                m.role == Role::Assistant && m.tool_calls.iter().any(|tc| tc.call_id == "c5")
            })
            .expect("round 6 declaration");
        messages.insert(
            idx,
            Message {
                role: Role::User,
                content: clear_marker(
                    5,
                    &[],
                    1,
                    Path::new(".gsa/compaction/blocks/tag-clear.md"),
                    Some("sess-abcd"),
                    "",
                    "黑板 notes r6@normal（【交接摘要】）",
                ),
                tool_call_id: None,
                tool_calls: Vec::new(),
                reasoning_content: None,
                round: None,
            },
        );
        let markers = face_markers(&messages);
        assert_eq!(markers.cleared_before_round, Some(6));
        assert!(markers.is_round_cleared(4), "前一边界轮随新边界一同隐藏");
        assert!(!markers.is_round_cleared(5), "新边界轮保留");
    }

    // ===== 0da S2（2026-10-11，设计 §4.5）钉组：D1 修复——清零态下常规
    // ===== 装配链恢复（回落钉）＋白名单豁免双路径

    /// 构造「8 轮 ＋ 清零 marker 插在第 4 轮声明前」的会话（boundary_round0=3
    /// ⇒ 清零边界＝第 4 轮＝块边（轮 1–3 恰为首块）——轮 1–3 整块清零、
    /// 轮 4–8 为后边界区 ≈20K 估算，slider=12K 下形成后边界闭合块）。
    fn cleared_conversation_with_post_boundary_blocks() -> Vec<Message> {
        let mut messages = conversation(8, 8_000);
        let idx = messages
            .iter()
            .position(|m| {
                m.role == Role::Assistant && m.tool_calls.iter().any(|tc| tc.call_id == "c3")
            })
            .expect("round 4 declaration");
        messages.insert(
            idx,
            Message {
                role: Role::User,
                content: clear_marker(
                    3,
                    &[1],
                    3,
                    Path::new(".gsa/compaction/blocks/tag-clear.md"),
                    Some("sess-abcd"),
                    "- 块#1 轮次 1-3: 完整内容见 .gsa/compaction/blocks/tag-block-0001.md",
                    "黑板 notes r4@normal（【交接摘要】）",
                ),
                tool_call_id: None,
                tool_calls: Vec::new(),
                reasoning_content: None,
                round: None,
            },
        );
        messages
    }

    /// 0da S2 钉（设计 §4.5-①④，D1 回落钉核心）：清零态下**常规装配链恢复**
    /// ——后边界闭合块打上压缩 marker ⇒ face 排除该块消息、估算回落、
    /// 读数可压集合恰清零（修复前：build_cleared_face 短路使压缩/截断 marker
    /// 不落隐藏，面估算单调上涨）；estimate ≡ build 逐条同口径（token 对拍）。
    #[test]
    fn cleared_face_honors_post_boundary_compression_and_estimate_falls() {
        let messages = cleared_conversation_with_post_boundary_blocks();
        let params = ModelFaceParams {
            slider_tokens: 12_000,
            block_tokens: 12_000,
            ledger_path: None,
            archive_tag: None,
            run_id: "RUN-CZ-D1".to_string(),
            d4_block: None,
            static_overhead_tokens: 0,
        };
        // 后边界闭合块（first_round ≥ 3，0 基——边界＝块边）。
        let blocks = blocks_outside_slider(&messages, 12_000, 12_000);
        let target = blocks
            .iter()
            .find(|b| b.closed && b.first_round >= 3)
            .expect("post-boundary closed block must exist");
        let est_before = estimate_model_face_tokens(&messages, &params);
        // 压缩前：后边界 Live 块内容在面内（v8：机械不替模型删——可见）。
        let face_before = build_model_face(&messages, &params).unwrap();
        assert!(
            face_before.iter().any(|m| m.role == Role::Tool
                && m.tool_call_id.as_deref() == Some(&format!("c{}", target.first_round))),
            "压缩前后边界块应在面内（Live 不隐藏）"
        );
        // 插入压缩 marker（该块）——marker 插在其首轮声明前（生产形态）。
        let mut marked = messages.clone();
        let idx = marked
            .iter()
            .position(|m| {
                m.role == Role::Assistant
                    && m.tool_calls
                        .iter()
                        .any(|tc| tc.call_id == format!("c{}", target.first_round))
            })
            .expect("block first round declaration");
        let compressed_marker = format!(
            "[前文上下文已压缩 v0.4-分块压缩]\n\
             已处理分块: {}\n\
             被处理轮次: 轮 {}-{}\n\
             == 模型面声明 ==\n声明正文\n[/前文上下文已压缩]",
            target.number,
            target.first_round + 1,
            target.last_round + 1,
        );
        marked.insert(
            idx,
            Message {
                role: Role::User,
                content: compressed_marker,
                tool_call_id: None,
                tool_calls: Vec::new(),
                reasoning_content: None,
                round: None,
            },
        );
        let markers = face_markers(&marked);
        let face = build_model_face(&marked, &params).unwrap();
        // 回落断言①：被压缩块的消息退出面（边界轮 c2 声明与其工具结果）。
        for r in target.first_round..=target.last_round {
            assert!(
                !face.iter().any(|m| m.role == Role::Assistant
                    && m.tool_calls.iter().any(|tc| tc.call_id == format!("c{r}"))),
                "被压缩块轮 {r} 的声明必须移出面（D1 修复：常规装配链在清零态生效）"
            );
        }
        // 回落断言②：估算回落。
        let est_after = estimate_model_face_tokens(&marked, &params);
        assert!(
            est_after < est_before,
            "清零态下压缩必须使面估算回落（{est_after} !< {est_before}）——D1 回落钉"
        );
        // 回落断言③：读数可压集合恰清零（清零块被 D1 过滤＋后边界块已压缩；
        // 滑块内残余不成块）。
        let readout = slider_readout(&marked, 12_000, 12_000);
        assert_eq!(
            readout.compressible_blocks, 0,
            "清零块不进可压集合＋后边界块已压缩 ⇒ 恰零：{readout:?}"
        );
        // 回落断言④：块表双态（边界前块「已清零」＋后边界块「已压缩」）。
        let table = render_block_table(
            &blocks_outside_slider(&marked, 12_000, 12_000),
            &markers,
            &params,
        );
        assert!(table.contains("已清零"), "{table}");
        assert!(table.contains("已压缩"), "{table}");
        // 钉（estimate ≡ build）：face 内容逐条估算（尾部表/停行已在 face 内）
        // ＝ 估算入口（D1：装配/估算/计数三分同源）。
        let face_sum: u64 = face
            .iter()
            .map(crate::controller::estimate_message_tokens)
            .sum();
        assert_eq!(
            face_sum, est_after,
            "estimate 必须与 build 逐条同口径（D1：装配/估算/计数三分同源）"
        );
    }

    /// 0da S2 钉（设计 §4.2/§4.5-②③，白名单豁免）：跨清零保留在**两条装配
    /// 路径**都成立——有块（常规装配前置段豁免）与无块（build_cleared_face
    /// 保留臂）；豁免只对白名单，原 prompt 仍随边界隐藏。
    #[test]
    fn cleared_face_whitelist_survives_in_both_assembly_paths() {
        let whitelist = format!("{} 任务背景：甲", crate::prompt::WHITELIST_PREFIX);
        let params = ModelFaceParams {
            slider_tokens: 12_000,
            block_tokens: 12_000,
            ledger_path: None,
            archive_tag: None,
            run_id: "RUN-CZ-WL".to_string(),
            d4_block: None,
            static_overhead_tokens: 0,
        };
        // 路径一：有块（常规装配）——cleared_conversation（6 轮，边界 5）。
        let mut with_blocks = cleared_conversation();
        with_blocks.insert(
            0,
            Message {
                role: Role::User,
                content: whitelist.clone(),
                tool_call_id: None,
                tool_calls: Vec::new(),
                reasoning_content: None,
                round: None,
            },
        );
        let face = build_model_face(&with_blocks, &params).unwrap();
        assert!(
            face.iter().any(|m| m.content == whitelist),
            "有块清零面必须保留白名单（前置段豁免）"
        );
        assert!(
            !face.iter().any(|m| m.content.contains("任务：实现更正批")),
            "豁免只对白名单——原 prompt 仍随边界隐藏"
        );
        // 路径二：无块（build_cleared_face）——2 轮小会话（≈1K/轮，无块）。
        let mut small = conversation(2, 2_000);
        let idx = small
            .iter()
            .position(|m| {
                m.role == Role::Assistant && m.tool_calls.iter().any(|tc| tc.call_id == "c1")
            })
            .expect("round 2 declaration");
        small.insert(
            idx,
            Message {
                role: Role::User,
                content: clear_marker(
                    1,
                    &[],
                    1,
                    Path::new(".gsa/compaction/blocks/tag-clear.md"),
                    Some("sess-abcd"),
                    "",
                    "黑板 notes r2@normal（【交接摘要】）",
                ),
                tool_call_id: None,
                tool_calls: Vec::new(),
                reasoning_content: None,
                round: None,
            },
        );
        small.insert(
            0,
            Message {
                role: Role::User,
                content: whitelist.clone(),
                tool_call_id: None,
                tool_calls: Vec::new(),
                reasoning_content: None,
                round: None,
            },
        );
        assert!(blocks_outside_slider(&small, 12_000, 12_000).is_empty());
        let face_small = build_model_face(&small, &params).unwrap();
        assert!(
            face_small.iter().any(|m| m.content == whitelist),
            "无块清零面必须保留白名单（保留臂）"
        );
        assert!(
            !face_small
                .iter()
                .any(|m| m.content.contains("任务：实现更正批")),
            "无块清零面：原 prompt 随边界隐藏"
        );
        assert!(
            face_small
                .iter()
                .any(|m| m.role == Role::Assistant
                    && m.tool_calls.iter().any(|tc| tc.call_id == "c1")),
            "无块清零面：边界轮起逐字保留"
        );
        // 计数同口径：无块清零面 count ＝ face len。
        assert_eq!(
            model_face_message_count(&small, &params),
            face_small.len(),
            "count 与 build 同口径（无块清零面）"
        );
    }

    // ===== 242 批（2026-10-11，0da S2 审查处置）钉组：跨清零边界块兜底回收
    // ===== （P2）＋脏 marker 不虚构隐藏（P3）

    /// 构造「8 轮×约3K 估算/轮 ＋ 清零 marker 插在第 4 轮声明前」的会话
    /// （boundary_round0=3 ⇒ 轮 1–3 清、轮 4 起保留；block_tokens=12K 恰
    /// 4 轮/块 ⇒ 首块＝轮 1–4＝**跨清零边界块**（首轮已清零、尾轮未清零）
    /// ——分区不保证在清零边界处对齐的可达形态）。
    fn cleared_conversation_with_straddling_block() -> Vec<Message> {
        let mut messages = conversation(8, 6_000);
        let idx = messages
            .iter()
            .position(|m| {
                m.role == Role::Assistant && m.tool_calls.iter().any(|tc| tc.call_id == "c3")
            })
            .expect("round 4 declaration");
        messages.insert(
            idx,
            Message {
                role: Role::User,
                content: clear_marker(
                    3,
                    &[],
                    3,
                    Path::new(".gsa/compaction/blocks/tag-clear.md"),
                    Some("sess-abcd"),
                    "",
                    "黑板 notes r4@normal（【交接摘要】）",
                ),
                tool_call_id: None,
                tool_calls: Vec::new(),
                reasoning_content: None,
                round: None,
            },
        );
        messages
    }

    /// 242 批钉（P2 核心）：**跨清零边界块照常参与兜底回收**——①判据：首轮
    /// 清零但非整块清零（`is_block_cleared` 收窄）；②回收：进可压集合（修复
    /// 前被首轮端点判据永久排除，读数 0）；③标态：块表如实「跨清零边界」
    /// 而非误标「原文」；④压缩生效：marker 落地后块臂隐藏其边界后段（含
    /// 边界轮），估算回落（修复前块臂跳过 ⇒ marker 声明「已压缩」而正文
    /// 仍在面）；⑤estimate ≡ build 逐条对拍。
    #[test]
    fn straddling_block_stays_reclaimable_and_truthfully_labeled() {
        let messages = cleared_conversation_with_straddling_block();
        let params = ModelFaceParams {
            slider_tokens: 12_000,
            block_tokens: 12_000,
            ledger_path: None,
            archive_tag: None,
            run_id: "RUN-0DA-P2".to_string(),
            d4_block: None,
            static_overhead_tokens: 0,
        };
        let blocks = blocks_outside_slider(&messages, 12_000, 12_000);
        let straddle = blocks
            .iter()
            .find(|b| b.closed && b.first_round == 0 && b.last_round >= 3)
            .expect("跨边界闭合块必须存在（轮 1–4 恰 12K）");
        let markers = face_markers(&messages);
        // ①判据：首轮清零 ∧ 非整块清零。
        assert!(markers.is_round_cleared(straddle.first_round));
        assert!(
            !markers.is_block_cleared(straddle.first_round, straddle.last_round),
            "跨边界块（first=清零侧、last=保留侧）不得判整块清零"
        );
        // ②回收：跨边界 Live 块进可压集合（读数恰 1——修复前 0）。
        let readout = slider_readout(&messages, 12_000, 12_000);
        assert_eq!(
            readout.compressible_blocks, 1,
            "跨边界块必须可压（修复前首轮判据永久排除）：{readout:?}"
        );
        // ③标态：块表「跨清零边界（前段已移出）」，不误标「原文」/「已清零」。
        let table = render_block_table(&blocks, &markers, &params);
        assert!(
            table.contains("跨清零边界（前段已移出）"),
            "跨边界块须如实标态：{table}"
        );
        assert!(!table.contains("已清零"), "整块清零标态只归整块：{table}");
        // ④压缩生效：marker 落地 ⇒ 块臂隐藏边界后段，估算回落。
        let est_before = estimate_model_face_tokens(&messages, &params);
        let mut marked = messages.clone();
        let idx = marked
            .iter()
            .position(|m| {
                m.role == Role::Assistant && m.tool_calls.iter().any(|tc| tc.call_id == "c0")
            })
            .expect("block first round declaration");
        marked.insert(
            idx,
            Message {
                role: Role::User,
                content: format!(
                    "[前文上下文已压缩 {}]\n已处理分块: {}",
                    BLOCK_MARKER_COMPRESSED_VERSION, straddle.number
                ),
                tool_call_id: None,
                tool_calls: Vec::new(),
                reasoning_content: None,
                round: None,
            },
        );
        let est_after = estimate_model_face_tokens(&marked, &params);
        assert!(
            est_after < est_before,
            "跨边界块压缩必须使面估算回落（{est_after} !< {est_before}）"
        );
        let face = build_model_face(&marked, &params).unwrap();
        assert!(
            !face
                .iter()
                .any(|m| m.role == Role::Assistant
                    && m.tool_calls.iter().any(|tc| tc.call_id == "c3")),
            "边界轮（块尾）须随块臂隐藏（修复前：块臂跳过 ⇒ marker 说已压缩而正文在面）"
        );
        assert!(
            face.iter()
                .any(|m| m.role == Role::Assistant
                    && m.tool_calls.iter().any(|tc| tc.call_id == "c4")),
            "滑块内轮次不受影响"
        );
        // ⑤estimate ≡ build 逐条对拍。
        let face_sum: u64 = face
            .iter()
            .map(crate::controller::estimate_message_tokens)
            .sum();
        assert_eq!(face_sum, est_after, "estimate ≡ build（三分同源）");
    }

    /// 242 批钉（P3）：脏 marker（有清零前缀、边界行不可解析）**不虚构
    /// 隐藏**——面/计数/估算与无 marker 基线恰差 marker 消息自身（前置段
    /// 与主循环同一 gated 推导；修复前 `hidden_message_ranges` 内部仅按
    /// 前缀自查 ⇒ 主循环隐藏而前置段不隐藏的两段口径分裂）。
    #[test]
    fn dirty_clear_marker_without_boundary_hides_nothing() {
        let marker = format!(
            "{}（边界行缺失——脏数据不虚构边界）\n[/前文上下文已清零]",
            CLEAR_MARKER_PREFIX
        );
        let mut messages = conversation(6, 8_000);
        let marker_msg = Message {
            role: Role::User,
            content: marker.clone(),
            tool_call_id: None,
            tool_calls: Vec::new(),
            reasoning_content: None,
            round: None,
        };
        messages.insert(3, marker_msg.clone());
        let baseline = conversation(6, 8_000);
        let params = ModelFaceParams {
            slider_tokens: 12_000,
            block_tokens: 12_000,
            ledger_path: None,
            archive_tag: None,
            run_id: "RUN-0DA-DIRTY".to_string(),
            d4_block: None,
            static_overhead_tokens: 0,
        };
        assert_eq!(
            face_markers(&messages).cleared_before_round,
            None,
            "脏 marker 不产出边界（parse_clear_boundary 负例）"
        );
        let dirty_face = build_model_face(&messages, &params).unwrap();
        let base_face = build_model_face(&baseline, &params).unwrap();
        assert_eq!(
            dirty_face.len(),
            base_face.len() + 1,
            "脏 marker 只增自身、隐藏零条目（面口径）"
        );
        assert!(
            dirty_face.iter().any(|m| m.content == marker),
            "脏 marker 消息本体仍在面（无边界 ⇒ 不隐藏任何内容）"
        );
        assert_eq!(
            model_face_message_count(&messages, &params),
            model_face_message_count(&baseline, &params) + 1,
            "计数口径同面"
        );
        assert_eq!(
            estimate_model_face_tokens(&messages, &params),
            estimate_model_face_tokens(&baseline, &params)
                + crate::controller::estimate_message_tokens(&marker_msg),
            "估算口径同面（脏 marker 只加自身估算）"
        );
    }
}
