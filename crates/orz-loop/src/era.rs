//! 0dx 会话内检查点时代机制（CONTEXT-CHECKPOINT-ERA，2026-10-30 立项）——
//! 时代切换的机械执行核。
//!
//! 设计权威：[`docs/CONTEXT_CHECKPOINT_ERA_DESIGN_2026-10-30.md`]（父仓；
//! v1.1 七裁决全确认）。语义：**会话＝话题分类器／时代＝话题内检查点／
//! run＝一次执行**。触发是输入的句法属性（外生、零模型参与、零 API 调用）：
//!
//! | 输入形态 | 语义 |
//! |---|---|
//! | `/checkpoint`（idle） | 手动时代重置；无随行输入则切换后不拉起 run |
//! | 无 `/btw` 前缀输入（idle） | 新时代开局：机械切换后拉起 run 灌入 |
//! | 无 `/btw` 前缀输入（在途） | 沿 0di 排队；下一 loop-top 切换后灌入 |
//! | `/btw` 前缀输入 | 同时代补充：照走 0di 排队，零行为变化 |
//!
//! 时代切换动作（纯机械）：①无损整段折叠（当前时代全部消息逐字落单个
//! 回放块，`.gsa/compaction/blocks/`，不摘要不打包）②工作窗重置（投影层
//! 边界臂，本地面零覆盖——I3）③黑板模型分区清空（plan/notes/findings/
//! state；机械分区与 journal/ledger 不动）④时代边界 journal 事件（含
//! LIF/RLI 特征快照与本时代用量——**入账不入面**，裁决 7 用户终裁）。
//!
//! 折叠核被两个调用点共享：宿主 idle 路径（`handle_session_prompt` 前置，
//! 事件随下一 run 落账）与 loop 主车道 loop-top drain 路径（在途排队项
//! 触发，事件当 run 落账）。marker 机制面（常量/反解/构造）在
//! [`crate::model_face`]，与清零 marker 同族同构。

use crate::blackboard::SharedBlackboard;
use crate::gateway::model::Message;
use orz_assurance::lif::TemporalSessionSnapshot;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// 0di 队列项：补充文本 ＋ 来源位。`mechanical = true` ＝ 机械生产者
/// （如 SelfCheck 工具栏投递）——照旧逐字投递、不参与时代触发判定；
/// 用户输入项（缺省）才做 `/btw`／`/checkpoint`／无前缀分流（协议即
/// 纪律，但不追溯适用于既有机械生产者——来源在入队时显式声明，不靠
/// 文本形状猜）。
///
/// `era_folded`＝本项已触发过一次时代折叠（391 批：drain 折叠成功后该项
/// 若因预算越线回退延迟队列，重试时**不再重折**——否则一次排队输入可
/// 在两次 loop-top 间打出两个时代边界）。入队构造恒 false；drain 成功
/// 折叠后置 true 随项携带。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupplementItem {
    pub text: String,
    pub mechanical: bool,
    pub era_folded: bool,
}

impl SupplementItem {
    pub fn user(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            mechanical: false,
            era_folded: false,
        }
    }

    pub fn mechanical(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            mechanical: true,
            era_folded: false,
        }
    }
}

/// 用户输入意图（前缀句法解析；外生、零歧义）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputIntent {
    /// `/btw <rest>`——同时代补充；`rest` 已剥前缀并去首空白。
    SameEra(String),
    /// `/btw`（无内容）——idle 拒发；在途丢弃。
    EmptySameEra,
    /// `/checkpoint [rest]`——手动时代切换；`rest` 为随行输入（可为空）。
    Checkpoint(Option<String>),
    /// 无前缀——是否换代由调用方结合会话状态裁决（有前置内容 ⇒ 换代）。
    Plain,
}

/// 前缀词法边界：`/btw`／`/checkpoint` 须独立成词——后随串空白、结尾、
/// **紧邻标点**（全角 `：，。；！？、` 与 ASCII `:` `,` 算边界并剥除前导
/// 标点）或**任意非 ASCII 字符**（391 批词界扩围：`/checkpoint修一下`
/// ＝带参切换、`/btw：补充` ⇔ `/btw 补充`）。理由＝代价不对称：漏判
/// （判 Plain）⇒ 意外换代＝整窗上下文折叠，扩判代价只是把标点/紧邻字
/// 剥进 rest；斜杠命令注册表无非 ASCII 后缀成员，扩判无冲突。ASCII
/// 字母数字紧随仍是普通词延续（`/btwxyz`、`/checkpoint2` 不算前缀）。
/// 首空白容忍（`/btw  xxx` ⇔ `/btw xxx`）。
const WORD_BOUNDARY_PUNCT: &[char] = &['：', '，', '。', '；', '！', '？', '、', ':', ','];

fn strip_word<'a>(input: &'a str, word: &str) -> Option<&'a str> {
    let trimmed = input.trim_start();
    let rest = trimmed.strip_prefix(word)?;
    if rest.is_empty()
        || rest.starts_with(char::is_whitespace)
        || rest.starts_with(|c: char| !c.is_ascii())
        || WORD_BOUNDARY_PUNCT.iter().any(|p| rest.starts_with(*p))
    {
        Some(rest.trim_start_matches(WORD_BOUNDARY_PUNCT).trim_start())
    } else {
        None
    }
}

/// 解析用户输入意图（纯函数；宿主 prompt 路径与 loop drain 路径共用）。
pub fn parse_input_intent(prompt: &str) -> InputIntent {
    if let Some(rest) = strip_word(prompt, "/btw") {
        return if rest.is_empty() {
            InputIntent::EmptySameEra
        } else {
            InputIntent::SameEra(rest.to_string())
        };
    }
    if let Some(rest) = strip_word(prompt, "/checkpoint") {
        return InputIntent::Checkpoint((!rest.is_empty()).then(|| rest.to_string()));
    }
    InputIntent::Plain
}

/// 时代切换的触发来源（journal 事件 `trigger` 字段）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EraTrigger {
    /// `/checkpoint` 命令（idle 手动）。
    CheckpointCommand,
    /// 无前缀输入（idle 自动）。
    UnprefixedInput,
    /// 无前缀输入（在途排队、loop-top 安全边界执行）。
    UnprefixedQueuedInput,
}

impl EraTrigger {
    pub fn as_str(&self) -> &'static str {
        match self {
            EraTrigger::CheckpointCommand => "checkpoint_command",
            EraTrigger::UnprefixedInput => "unprefixed_input",
            EraTrigger::UnprefixedQueuedInput => "unprefixed_queued_input",
        }
    }
}

/// 本时代累计用量（调用数／prompt tokens）——会话侧车跨 run 累计；
/// run 级五项细目仍在 `run_usage_metered`，此处只留时代对账两键
/// （0dx S4 读数面；**入账不入面**，裁决 7）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EraUsage {
    pub model_calls: u64,
    pub prompt_tokens: u64,
}

impl EraUsage {
    pub fn saturating_add(&mut self, model_calls: u64, prompt_tokens: u64) {
        self.model_calls = self.model_calls.saturating_add(model_calls);
        self.prompt_tokens = self.prompt_tokens.saturating_add(prompt_tokens);
    }
}

/// LIF/RLI 特征快照（时代边界事件的仪表载荷；**入账不入面**——裁决 7：
/// P7 本线不写模型面内容＋0cp S4 提醒面在现实任务事实死亡＋D7 不能假设
/// 现实任务能被分出域特征）。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EraFeatureSnapshot {
    /// 当前驻留域（LIF temporal；`as_str` 机器键）。
    pub domain: Option<String>,
    /// 会话内已完成决策轮数（跨 prompt 单调）。
    pub session_rounds: Option<u64>,
    /// RLI 影子 T̂（秒；影子未启用 ⇒ `None`——不落假值）。
    pub t_hat_secs: Option<f64>,
    /// 各通道持续越线计数最大值（RLI streak；影子未启用 ⇒ `None`）。
    pub streak_max: Option<u64>,
    /// 域 spike 进入／回归累计（0cp D7；影子未启用 ⇒ `None`）。
    pub spike_entries: Option<u64>,
    pub spike_returns: Option<u64>,
    /// LIF 域切换时间线条数（temporal spikes）。
    pub domain_spikes: Option<u64>,
}

impl EraFeatureSnapshot {
    /// 从 LIF 会话快照提取（idle 路径＝侧车快照；在途路径＝活体引擎
    /// `temporal_session_snapshot()`——同一形状，两路同源）。
    pub fn from_temporal_snapshot(snapshot: &TemporalSessionSnapshot) -> Self {
        let shadow = snapshot.rli_shadow.as_ref();
        let streak_max = shadow.map(|s| s.channels.iter().map(|c| c.streak).max().unwrap_or(0));
        Self {
            domain: Some(snapshot.current_domain.as_str().to_string()),
            session_rounds: Some(snapshot.round),
            t_hat_secs: shadow.map(|s| s.t_hat),
            streak_max,
            spike_entries: shadow.map(|s| s.spike_entries),
            spike_returns: shadow.map(|s| s.spike_returns),
            domain_spikes: Some(snapshot.spikes.len() as u64),
        }
    }
}

/// 折叠输入（显式全参——核无自我状态，宿主/loop 两路各自装配）。
pub struct EraFoldInput<'a> {
    pub messages: &'a mut Vec<Message>,
    pub blackboard: &'a SharedBlackboard,
    pub session_id: Option<&'a str>,
    /// 会话工作目录（`.gsa/compaction/blocks/` 与台账路径的根）。
    pub cwd: &'a Path,
    /// 执行折叠的 run（档案档头记载；idle 裸 `/checkpoint` 无 run ⇒ `None`）。
    pub run_id: Option<&'a str>,
    pub trigger: EraTrigger,
    pub features: EraFeatureSnapshot,
    /// 被关闭时代的累计用量（调用方取自会话侧车/活体累计）。
    pub era_usage: EraUsage,
    /// 外挂台账路径（折叠轮次补挂行；marker 台账定位）。
    pub ledger_path: &'a Path,
    /// 滑块/分块参数（块号分组与折叠读数同尺；controller 持单源）。
    pub slider_tokens: u64,
    pub block_tokens: u64,
}

/// 折叠回执（调用方落 journal 事件与用户面反馈）。
#[derive(Debug, Clone)]
pub struct EraFoldOutcome {
    /// `era_boundary` 事件载荷（调用方原样 record；本核不触 journal）。
    pub payload: serde_json::Value,
    /// 时代边界标记全文（已插入 `messages` 尾部）。
    pub marker: String,
    /// 边界轮号（1 基会话全局；＝被关闭时代之后的第一轮）。
    pub boundary_round1: usize,
    /// 被关闭的时代号（1 基）。
    pub era_closed: u64,
    /// 时代档案相对路径（会话卷形态）。
    pub archive_relative: String,
    /// 本次补挂台账行的块号（已压缩/截断/清零块不重挂）。
    pub ledger_blocks: Vec<u32>,
    pub rounds_folded: usize,
    pub messages_folded: usize,
    /// 折叠前后模型面估算差（chars/2 口径）。
    pub freed_tokens: u64,
    /// 黑板模型分区移出条数（plan/notes/findings 条目 ＋ state 1/0）。
    pub board_entries_removed: usize,
    pub archive_write_failed: bool,
}

/// 时代序号（1 基）＝既有时代 marker 数 ＋ 1（会话消息列表是唯一真源）。
/// 全形判定（391 批）：前缀＋「时代边界」行可反解才算数——用户恰以
/// marker 前缀开头的真实输入不得膨胀时代号（否则同号档案覆盖）。
pub fn era_number_in(messages: &[Message]) -> u64 {
    messages
        .iter()
        .filter(|m| is_era_marker(&m.content))
        .count() as u64
        + 1
}

/// 全形时代 marker 判定：前缀＋边界行反解成功（脏数据/仿造前缀 ⇒ false）。
pub fn is_era_marker(content: &str) -> bool {
    content.starts_with(crate::model_face::ERA_MARKER_PREFIX)
        && crate::model_face::parse_era_boundary(content).is_some()
}

/// 时代回放档案：`{cwd}/.gsa/compaction/blocks/{tag}-era-{n:03}.md`
/// （与 v0.4 分块同目录同性质；设计 §1.2-1）。
pub fn era_archive_path(root: &Path, tag: &str, era: u64) -> std::path::PathBuf {
    root.join(".gsa")
        .join("compaction")
        .join("blocks")
        .join(format!("{tag}-era-{era:03}.md"))
}

/// 时代档案的会话卷相对形态。
pub fn relative_era_archive(tag: &str, era: u64) -> String {
    format!(".gsa/compaction/blocks/{tag}-era-{era:03}.md")
}

/// 时代档案正文（边界前 `[0, upto)` 逐字整段——无损、不打包；档头带
/// 时代号／边界轮／触发来源／边界时间戳）。
pub fn era_archive_markdown(
    messages: &[Message],
    upto: usize,
    boundary_round1: usize,
    era: u64,
    trigger: EraTrigger,
    run_id: Option<&str>,
    session: Option<&str>,
) -> String {
    let mut out = format!(
        "# ORZ 时代回放档案（时代 {era}；轮 1–{}）\n\n\
         - 边界: 第 {boundary_round1} 轮（时代边界；本档覆盖其之前全部轮次）\n\
         - 触发: {}\n- 消息区间: [0, {upto})\n- run: {}\n\
         - 会话／黑板会话: {}\n- 生成时间: {}\n\n\
         > 陈旧性标注：内容截至轮次 {boundary_prev}；文件可能已变更，编辑前须新鲜读取。\n",
        boundary_round1.saturating_sub(1),
        trigger.as_str(),
        run_id.unwrap_or("（无——切换发生于 run 之间）"),
        session.unwrap_or("（无）"),
        crate::controller::chrono_utc_now(),
        boundary_prev = boundary_round1.saturating_sub(1),
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
        // 无损保真（391 批）：reasoning_content 逐字入档——消息体的组成
        // 字段一个不丢（空缺如实缺节）。
        if let Some(reasoning) = m.reasoning_content.as_deref().filter(|r| !r.is_empty()) {
            out.push_str("[推理内容]\n");
            out.push_str(reasoning);
            out.push('\n');
        }
        out.push_str(&m.content);
        out.push('\n');
    }
    out
}

/// 折叠后分块分组（与 [`crate::model_face::blocks_outside_slider`] 同算法：
/// 自会话起点按 `block_tokens` 估算分组、只收完整轮、首个不完整轮即止）——
/// 折叠后这些轮全部越出主滑块，读时现算的块号与此处一致（编号锚在会话起点）。
fn post_fold_blocks(
    ranges: &[(usize, usize)],
    messages: &[Message],
    block_tokens: u64,
) -> Vec<(u32, usize, usize)> {
    let block_tokens = block_tokens.max(1);
    let mut out = Vec::new();
    let mut cur_first: Option<usize> = None;
    let mut cur_last: usize = 0;
    let mut cur_total: u64 = 0;
    let mut number: u32 = 0;
    for (idx, &(s, e)) in ranges.iter().enumerate() {
        if !crate::action_ledger::is_round_complete(messages, (s, e)) {
            break;
        }
        cur_first.get_or_insert(idx);
        // 391 批（块尾组口径）：末轮必须记**它真正累计到的最后一轮**
        // （`cur_last`），不得写 `ranges.len() - 1`——后者把未完整尾轮
        // 圈进块号分组，与读时 `blocks_outside_slider` 的残段口径分叉
        // （2026-09-16 P0 修复注释同款禁令）。台账定位行 seq 区间由此
        // 与读时分块表严格一致。
        cur_last = idx;
        cur_total =
            cur_total.saturating_add(crate::controller::estimate_messages_tokens(&messages[s..e]));
        if cur_total >= block_tokens {
            number += 1;
            out.push((number, cur_first.take().unwrap_or(idx), idx));
            cur_total = 0;
        }
    }
    if let Some(first) = cur_first {
        number += 1;
        out.push((number, first, cur_last.max(first)));
    }
    out
}

/// 时代切换执行核（纯机械、零模型参与、零 API 调用；设计 §1.2 五步）。
///
/// `Err(String)` ＝ 中性拒绝（无可折叠内容等）；调用方如实回传/记录，
/// 不视为故障。成功路径就地改写 `messages`（尾部插入时代 marker——I3
/// 本地面零覆盖：既有消息逐字不动）。
pub fn apply_era_fold(input: EraFoldInput<'_>) -> Result<EraFoldOutcome, String> {
    let EraFoldInput {
        messages,
        blackboard,
        session_id,
        cwd,
        run_id,
        trigger,
        features,
        era_usage,
        ledger_path,
        slider_tokens,
        block_tokens,
    } = input;
    // 防空折：上一枚时代 marker 之后必须有实质内容（否则切换后紧随的
    // 无前缀输入会空转折叠——手动 /checkpoint 后首轮输入正是此形态）。
    // 全形判定（391 批）：仿造前缀文本不作数。
    if messages
        .last()
        .is_some_and(|last| is_era_marker(&last.content))
    {
        return Err("无可折叠内容：上一时代边界之后尚无新内容。".to_string());
    }
    let ranges = crate::action_ledger::round_ranges(messages);
    let boundary_round1 = ranges.len() + 1;
    // 折叠区＝marker 之前的全部消息（含未成轮的首部杂谈与未完整尾轮——
    // 无损折叠不做 keep 窗；工作窗重置由投影层边界臂承担）。
    let fold_end = messages.len();
    if fold_end == 0 {
        return Err("无可折叠内容：会话为空。".to_string());
    }
    let markers_before = crate::model_face::face_markers(messages);
    let era_closed = era_number_in(messages);
    // 折叠读数基线（chars/2 口径；marker 插入前的模型面估算）。
    let estimate_params = crate::model_face::ModelFaceParams {
        slider_tokens,
        block_tokens,
        ledger_path: None,
        archive_tag: Some(crate::model_face::archive_tag(
            session_id,
            run_id.unwrap_or("era"),
        )),
        run_id: run_id.unwrap_or("era").to_string(),
        d4_block: None,
        static_overhead_tokens: 0,
    };
    let before_tokens = crate::model_face::estimate_model_face_tokens(messages, &estimate_params);

    // ③ 黑板模型分区清空（plan/notes/findings ＋ state；机械分区与
    // journal/ledger 不动——设计 §1.4/§1.5）。
    let notes_removed = blackboard.write().clear_model_note_sections(&[
        crate::blackboard::ModelNoteSection::Plan,
        crate::blackboard::ModelNoteSection::Notes,
        crate::blackboard::ModelNoteSection::Findings,
    ]);
    let state_cleared = blackboard.write().clear_state();
    let board_entries_removed = notes_removed + usize::from(state_cleared);

    // ① 无损整段折叠（单块逐字落盘；失败如实标注不阻塞——同清零口径）。
    // digest 对**脱敏后**全文计算（391 批）：`write_archive_retry` 落盘前
    // 统一接 orz-secrets 机械脱敏，digest 若对脱敏前原文计算则与文件字节
    // 永久失配、复核不可为。脱敏幂等（占位串不再命中密钥形态），写入侧
    // 二次脱敏零失真。
    let tag = crate::model_face::archive_tag(session_id, run_id.unwrap_or("era"));
    let era = era_closed;
    let archive_path = era_archive_path(cwd, &tag, era);
    let markdown_raw = era_archive_markdown(
        messages,
        fold_end,
        boundary_round1,
        era,
        trigger,
        run_id,
        session_id,
    );
    let markdown = orz_secrets::redact_secrets(&markdown_raw);
    let digest = crate::summary::archive_digest(&markdown);
    let archive_dir = archive_path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_default();
    let archive_write_failed =
        !crate::summary::write_archive_retry(archive_dir.as_path(), &archive_path, &markdown);

    // 台账补挂：折叠轮次中仍为原文（Live）的块补挂行；已被压缩/截断/
    // 清零覆盖的块其行在原动作时已挂（不重挂——防重复）。分组沿折叠后
    // 口径（全部折叠轮越出主滑块）；各块 seq 子区间＝按块行数切分连续
    // 编号（压缩 `ledger_locators` 同法）。清零边界**逐轮**判定（391 批）：
    // 跨清零边界的块，边界前轮已挂、边界后轮从未挂——按轮跳过已清轮，
    // 其余轮照挂（整块跳过会使后段行永不入台账）。
    let groups = post_fold_blocks(&ranges, messages, block_tokens);
    let mut ledger_blocks: Vec<u32> = Vec::new();
    let mut appended_rows: Vec<crate::action_ledger::ActionLedgerRow> = Vec::new();
    let mut rows_per_block: Vec<usize> = Vec::new();
    for (number, first_round, last_round) in &groups {
        if markers_before.state(*number) != crate::model_face::BlockState::Live {
            continue;
        }
        let mut rows_in_block = 0usize;
        for round in *first_round..=*last_round {
            if markers_before.is_round_cleared(round) {
                continue;
            }
            let Some(&range) = ranges.get(round) else {
                break;
            };
            let rows = crate::action_ledger::rows_for_round(messages, range, round);
            rows_in_block += rows.len();
            appended_rows.extend(rows);
        }
        // 零行块（清零后段无行可挂）不进台账定位清单——marker 不出现空区间。
        if rows_in_block > 0 {
            ledger_blocks.push(*number);
            rows_per_block.push(rows_in_block);
        }
    }
    let ledger_range =
        match crate::action_ledger::append_ledger_rows_range(ledger_path, &appended_rows) {
            Ok(range) => range,
            Err(e) => {
                tracing::warn!("era fold: ledger append failed (best-effort): {e}");
                None
            }
        };
    let mut ledger_lines: Vec<String> = Vec::new();
    if let Some((from, to)) = ledger_range {
        let mut cursor = from;
        for (i, &number) in ledger_blocks.iter().enumerate() {
            let is_last = i + 1 == ledger_blocks.len();
            let end = if is_last {
                to
            } else {
                cursor
                    .saturating_add(rows_per_block[i] as u64)
                    .saturating_sub(1)
                    .min(to)
            };
            if cursor <= end {
                ledger_lines.push(format!("块#{number} [seq] {cursor}-{end}"));
            }
            cursor = end.saturating_add(1);
        }
    }

    // ②+④ marker 插入（尾部；投影层边界臂据此隐藏 [0, idx)）。幸存面
    // 封闭清单：系统提示／工具定义／时代标记／新输入——marker 文本按
    // 设计 §1.2-3 指针字段定稿，无语义摘要槽、无特征行（裁决 4/7）。
    // 边界时间戳单源（391 批）：一次取时同喂 marker 与事件载荷——两处
    // 各自 `chrono_utc_now()` 的微秒级漂移自此消灭。
    let boundary_at = crate::controller::chrono_utc_now();
    let marker = crate::model_face::era_marker(
        boundary_round1,
        &ledger_lines,
        &relative_era_archive(&tag, era),
        &digest,
        &boundary_at,
        session_id,
        archive_write_failed,
    );
    messages.push(Message {
        role: crate::gateway::model::Role::User,
        content: marker.clone(),
        tool_call_id: None,
        tool_calls: Vec::new(),
        reasoning_content: None,
        round: None,
    });

    // 折叠读数（chars/2 口径；与清零同尺）。
    let after = crate::model_face::estimate_model_face_tokens(messages, &estimate_params);
    let rounds_folded = ranges.len();
    let messages_folded = fold_end;
    let archive_relative = relative_era_archive(&tag, era);

    let payload = serde_json::json!({
        "era": era,
        "boundary_round": boundary_round1,
        "trigger": trigger.as_str(),
        "boundary_at": boundary_at,
        "archive": {
            "path": archive_relative,
            "digest_sha256": digest,
            "rounds": rounds_folded,
            "messages": messages_folded,
            "write_failed": archive_write_failed,
        },
        "features": {
            "domain": features.domain,
            "session_rounds": features.session_rounds,
            "t_hat_secs": features.t_hat_secs,
            "streak_max": features.streak_max,
            "spike_entries": features.spike_entries,
            "spike_returns": features.spike_returns,
            "domain_spikes": features.domain_spikes,
        },
        "usage": {
            "model_calls": era_usage.model_calls,
            "prompt_tokens": era_usage.prompt_tokens,
        },
        "board_entries_removed": board_entries_removed,
    });

    Ok(EraFoldOutcome {
        payload,
        marker,
        boundary_round1,
        era_closed: era,
        archive_relative,
        ledger_blocks,
        rounds_folded,
        messages_folded,
        freed_tokens: before_tokens.saturating_sub(after),
        board_entries_removed,
        archive_write_failed,
    })
}

/// 会话共享时代用量句柄（宿主持有、跨 run 累计；controller 经
/// `with_era_usage` 注入，drain 折叠时读取并清零）。
pub type SharedEraUsage = Arc<Mutex<EraUsage>>;

/// 会话共享时代态（391 批统一账面）：**单一** `Arc<Mutex>` 同时承载
/// 时代累计用量与待命事件载荷——宿主检查点 sink 闭包与 controller
/// （drain 折叠清零换元／`run_started` 落账取走待命载荷）读写同一份，
/// 消灭 390 批「宿主 cell 初值快照」与 controller 内部态的双源漂移
/// （折叠后 run 失败窗口的账面/载荷失真即源于此）。
#[derive(Debug, Default, Clone)]
pub struct EraRunState {
    pub usage: EraUsage,
    /// idle 折叠产出、待下一 run `run_started` 后落账的事件载荷
    /// （取走即清；落账后 sink 写沿不再携带——重复落账窗口闭合）。
    pub pending_era_event: Option<serde_json::Value>,
}

pub type SharedEraState = Arc<Mutex<EraRunState>>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blackboard::SharedBlackboard;
    use crate::gateway::model::{Message, Role};

    fn user_msg(text: &str) -> Message {
        Message {
            role: Role::User,
            content: text.to_string(),
            tool_call_id: None,
            tool_calls: Vec::new(),
            reasoning_content: None,
            round: None,
        }
    }

    #[test]
    fn input_intent_prefixes_are_word_bounded() {
        assert_eq!(
            parse_input_intent("/btw 补充一下"),
            InputIntent::SameEra("补充一下".to_string())
        );
        assert_eq!(parse_input_intent("/btw"), InputIntent::EmptySameEra);
        assert_eq!(parse_input_intent("/btw  "), InputIntent::EmptySameEra);
        // 词界：/btwxyz 不是前缀。
        assert_eq!(parse_input_intent("/btwxyz"), InputIntent::Plain);
        assert_eq!(
            parse_input_intent("/checkpoint"),
            InputIntent::Checkpoint(None)
        );
        assert_eq!(
            parse_input_intent("/checkpoint 修一下编译"),
            InputIntent::Checkpoint(Some("修一下编译".to_string()))
        );
        assert_eq!(parse_input_intent("/checkpointx"), InputIntent::Plain);
        assert_eq!(parse_input_intent("普通输入"), InputIntent::Plain);
        // 首空白容忍。
        assert_eq!(
            parse_input_intent("  /btw 后置"),
            InputIntent::SameEra("后置".to_string())
        );
        // 391 批词界扩围：紧邻标点算边界（漏判＝意外换代，代价不对称）。
        assert_eq!(
            parse_input_intent("/btw：补充一下"),
            InputIntent::SameEra("补充一下".to_string())
        );
        assert_eq!(
            parse_input_intent("/checkpoint修一下"),
            InputIntent::Checkpoint(Some("修一下".to_string()))
        );
        assert_eq!(
            parse_input_intent("/checkpoint："),
            InputIntent::Checkpoint(None)
        );
        assert_eq!(
            parse_input_intent("/btw, note"),
            InputIntent::SameEra("note".to_string())
        );
        // 字母紧随仍不算前缀（普通词延续）。
        assert_eq!(parse_input_intent("/checkpoint2"), InputIntent::Plain);
    }

    #[test]
    fn era_usage_saturating_add() {
        let mut usage = EraUsage::default();
        usage.saturating_add(3, 100);
        usage.saturating_add(u64::MAX, 5);
        assert_eq!(usage.model_calls, u64::MAX);
        assert_eq!(usage.prompt_tokens, 105);
    }

    #[test]
    fn feature_snapshot_extracts_domain_and_shadow_fields() {
        let mut engine = orz_assurance::lif::LifEngine::new();
        engine.enable_rli_shadow();
        let snapshot = engine.temporal_session_snapshot();
        let features = EraFeatureSnapshot::from_temporal_snapshot(&snapshot);
        assert_eq!(features.domain.as_deref(), Some("start"));
        assert_eq!(features.session_rounds, Some(0));
        assert!(features.t_hat_secs.is_some());
        assert_eq!(features.spike_entries, Some(0));
        // 影子未启用 ⇒ Option 面 None（不落假值）。
        let bare = orz_assurance::lif::LifEngine::new();
        let bare_features =
            EraFeatureSnapshot::from_temporal_snapshot(&bare.temporal_session_snapshot());
        assert!(bare_features.t_hat_secs.is_none());
        assert!(bare_features.streak_max.is_none());
    }

    /// 测试工作目录（std::env::temp_dir 下带进程/用例唯一后缀；
    /// orz-loop 无 tempfile dev-dep——summary.rs 测试同法）。
    fn test_cwd(label: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "orz-era-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.subsec_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).expect("create test cwd");
        dir
    }

    #[test]
    fn fold_refuses_empty_and_marker_tailed_conversations() {
        let board = SharedBlackboard::new();
        let dir = test_cwd("refuse");
        let ledger = dir.join("ledger").join("current.md");
        let mut messages: Vec<Message> = Vec::new();
        let err = apply_era_fold(EraFoldInput {
            messages: &mut messages,
            blackboard: &board,
            session_id: None,
            cwd: &dir,
            run_id: None,
            trigger: EraTrigger::CheckpointCommand,
            features: EraFeatureSnapshot::default(),
            era_usage: EraUsage::default(),
            ledger_path: &ledger,
            slider_tokens: 160_000,
            block_tokens: 32_000,
        })
        .expect_err("empty conversation must refuse");
        assert!(err.contains("无可折叠内容"));

        // 上一枚 marker 之后无新内容 ⇒ 拒绝（防手动 /checkpoint 后首轮空转）。
        let mut messages = vec![user_msg("hello")];
        messages.push(Message {
            role: Role::User,
            content: format!(
                "[前文上下文已换代 {}]\n时代边界: 第 1 轮]\n[/前文上下文已换代]",
                crate::model_face::ERA_MARKER_VERSION
            ),
            tool_call_id: None,
            tool_calls: Vec::new(),
            reasoning_content: None,
            round: None,
        });
        let err = apply_era_fold(EraFoldInput {
            messages: &mut messages,
            blackboard: &board,
            session_id: None,
            cwd: &dir,
            run_id: None,
            trigger: EraTrigger::CheckpointCommand,
            features: EraFeatureSnapshot::default(),
            era_usage: EraUsage::default(),
            ledger_path: &ledger,
            slider_tokens: 160_000,
            block_tokens: 32_000,
        })
        .expect_err("marker-tailed conversation must refuse");
        assert!(err.contains("无可折叠内容"));

        // 391 批全形判定：仅前缀同形、无「时代边界」行可反解的仿造文本
        // 不作数——不触发防空折拒绝，照常折叠。
        let mut messages = vec![
            user_msg("hello"),
            Message {
                role: Role::User,
                content: "[前文上下文已换代 v0.4] 用户仿造前缀（无边界行）".to_string(),
                tool_call_id: None,
                tool_calls: Vec::new(),
                reasoning_content: None,
                round: None,
            },
        ];
        let outcome = apply_era_fold(EraFoldInput {
            messages: &mut messages,
            blackboard: &board,
            session_id: None,
            cwd: &dir,
            run_id: None,
            trigger: EraTrigger::CheckpointCommand,
            features: EraFeatureSnapshot::default(),
            era_usage: EraUsage::default(),
            ledger_path: &ledger,
            slider_tokens: 160_000,
            block_tokens: 32_000,
        })
        .expect("forged prefix must not trigger the empty-fold guard");
        assert_eq!(outcome.era_closed, 1);
    }

    /// 391 批（无损保真）：reasoning_content 与工具调用细节逐字入档。
    #[test]
    fn era_archive_markdown_preserves_reasoning_and_tool_calls() {
        let messages = vec![Message {
            role: Role::Assistant,
            content: "结论正文".to_string(),
            tool_call_id: None,
            tool_calls: vec![crate::gateway::model::ToolCall {
                name: "read_file".to_string(),
                arguments: serde_json::json!({"path": "a.md"}),
                call_id: "call_1".to_string(),
            }],
            reasoning_content: Some("先查文件再下结论。".to_string()),
            round: None,
        }];
        let md = era_archive_markdown(&messages, 1, 2, 1, EraTrigger::UnprefixedInput, None, None);
        assert!(md.contains("结论正文"));
        assert!(
            md.contains("先查文件再下结论。"),
            "reasoning_content 必须逐字入档"
        );
        assert!(md.contains("[推理内容]"));
        assert!(md.contains("[工具调用] read_file"));
        assert!(md.contains(r#""path":"a.md""#));
    }

    /// 391 批（时间戳单源）：marker「时代边界时间戳」行与事件载荷
    /// `boundary_at` 同值（同一瞬时，两次取时的微秒级漂移消灭）。
    #[test]
    fn era_marker_and_payload_share_boundary_timestamp() {
        let board = SharedBlackboard::new();
        let dir = test_cwd("ts");
        let ledger = dir.join("ledger").join("current.md");
        let mut messages = vec![user_msg("hello")];
        let outcome = apply_era_fold(EraFoldInput {
            messages: &mut messages,
            blackboard: &board,
            session_id: None,
            cwd: &dir,
            run_id: None,
            trigger: EraTrigger::CheckpointCommand,
            features: EraFeatureSnapshot::default(),
            era_usage: EraUsage::default(),
            ledger_path: &ledger,
            slider_tokens: 160_000,
            block_tokens: 32_000,
        })
        .expect("fold must succeed");
        let payload_at = outcome
            .payload
            .get("boundary_at")
            .and_then(serde_json::Value::as_str)
            .expect("boundary_at in payload");
        let marker_line = outcome
            .marker
            .lines()
            .find(|l| l.starts_with("时代边界时间戳: "))
            .expect("marker carries the timestamp line")
            .trim_start_matches("时代边界时间戳: ");
        assert_eq!(payload_at, marker_line);
    }
}
