//! L2 命令面机械审查（0bw v1 → **0cb v2 保底化**，2026-09-29）：`run_terminal_cmd`
//! 的 best-effort 风险闸（block／warn／allow 三分类）与留痕面文案。
//!
//! 设计权威：[`docs/WRITE_CONTROL_BACKSTOP_REVISION_DESIGN_2026-09-29.md`]（v2.0
//! §1 规则面）修订 [`docs/WRITE_CONTROL_MECHANICAL_DESIGN_2026-09-26.md`]（v1 §4）。
//! **v2 定位＝灾难硬边界保底**：block 面收窄为**封闭枚举恰 5 条规则**
//! （[`BLOCK_RULES`]）——根级递归删除／raw 设备卷毁写／引导固件与安全机制翻转／
//! 注册表蜂巢修改／载体自保护；v1 的整树位置锁、重定向即写扫系统树、
//! `/dev//proc//sys` 前缀词元扫**全部退役**（一般写动作——装 `/usr`、清
//! `SoftwareDistribution`、`>/dev/null`、`+O/dev/null`——交回审批组件，表外恒
//! allow）。warn 面＝`broad-destructive`（留痕）与 `elevation`（提权）纯留痕。
//!
//! 匹配＝「程序位词元精确」＋「短语」两法；`cmd /c`／`powershell -Command` 等
//! 内容位**递归展开一层**。**不解析完整 shell 语法**——变量拼接、别名、`.NET`
//! 直调、脚本内命令等不保证覆盖（best-effort，声明边界；命中即拒、绝不因解析
//! 失败放行）。命令内路径 token 经 [`crate::types::write_control`] 同一绑定归一化；
//! 规则 1 目标比对＝[`write_control::path_equals_root`]（恰为树根／卷根本体，
//! 子目录级精准删除放行）；规则 5 载体集比对＝包含语义（恒拒面）。
//!
//! **0cb 审查处理批（2026-09-29，用户裁决「过严臂进一步收窄」）**：① 规则 2
//! PhysicalDrive 词元扫收窄到写侧目标位（`of=` 值／`mkfs*` 目标词）——读侧
//! `if=` 与查询类提及放行；`mkfs*` 同样目标位判定（镜像文件构建放行），
//! `/dev/mapper` 增补进块设备形态集；② 规则 1 动词集补 `ri`（Remove-Item
//! 别名）＋Windows 盘符相对根形态（`\Windows`）按 `%SystemDrive%` 补全比对；
//! ③ 规则 4 蜂巢判定收窄到目标位词元（数据值提及不误拦）。
//!
//! **0cc v3（2026-09-29 晚，规则 5 宿主机灾难保底收窄）**：规则 5 目标集缩为
//! **宿主状态两窄目标**（`.gsa` 会话卷＋ACAF keystore 根／signer manifest——
//! [`write_control::HostStateTargets`] 装配同源 env 调用点解析）；v2 的安装
//! 目录／三件套／`grok-home`／⊆cwd 降级**双面全退役**——`/usr/local/bin`
//! 安装、写载体三件套、软链指入安装目录全部放行（build-pov-ray 结构性 0 的
//! 修复面；契约面零变化＝schema v0.3 枚举不动，`carrier-write` id 沿用）。
//!
//! **0cc v3.1 审查处理批（2026-09-30）**：① **祖先链臂（P2）**——扫荡动词
//! （删除/搬移，[`ANCESTOR_SWEEP_VERBS`]）下目标为 keystore 根／manifest 的
//! 严格祖先同落 `carrier-write` block（宿主原生 keystore 实况位于已退役的
//! 载体安装目录内，载体面退役不得连带放行信任锚的扫荡摧毁；入位写不触发）；
//! ② 动词表补 `install`／`ln`（P3-1——两族可向宿主态目标落盘/建链接）。
//! 契约面零变化：`carrier-write` id 沿用，祖先 face id（`carrier:*-ancestor`）
//! 为 L1/L2 文案层，不入 schema 枚举。
//! **0cq S2（2026-10-04，写控误拦两族修复）**：recli 三跑三条真机误拦
//! （181 批 §4b；S1 勘定＝三例同根——`path_candidates` 裸词空白拆片把 echo
//! 散文撕成 `/`、`.gsa/usr)` 伪词元 × 规则 1/5 全局词扫描；另叠加 find 读
//! 排除模式值位被当写目标）。修复四件：① 裸词不再空白拆片（kv 值保留）；
//! ② 规则 1 扫描精准化（动词与旗**同段**武装、仅扫**动词位之后**的本段
//! 词——`cd /` 头部裸 `/` 不再算删除目标）；③ 规则 5 arm 收窄（`>` 目标位
//! 为 `/dev/null`/`NUL`/fd 数字不武装）＋扫描段内化（仅**写段**）＋读模式
//! 值豁免（`READ_PATTERN_OPTIONS`／kv 前缀闭集）；真机三例全原文回归钉＋
//! 真阳性对照钉（`rm -rf /`／写 `.gsa`／目标位 `.gsa` 全保留）。
//! **186 批（2026-10-04，全面审查发现处置）**：三件——`segment_is_write`
//! 重定向臂收窄为**非 nullish** 重定向（复合命令无关写动词 × nullish 读段
//! 残余误拦面收口）；规则 1 detail 续行文案回归修复（185 批引入 34 空格）；
//! 读模式豁免两闭集表补 0cb 防膨胀长度钉。
//! **0ct（2026-10-07，`.gsa` 读向放行）**：规则 5 写段扫描补**读方向词位
//! 豁免**（用户裁决「`.gsa` 读全开放、只读不改」；来源＝0cr D4 meshctl
//! `cp .gsa/… /tmp/…` 读向复制被拦，200 批 §2/§3）——(a) 复制族源位豁免
//! （复制/安装不改动源，仅落点是写方向；`mv`/`ln`/`tee` 不豁免——搬移即
//! 源位写／硬链写别名／多落点同写）；(b) 重定向武装段纯读程序豁免
//! （`cat .gsa/x > /tmp/y` 是读不是写；`find`/`sort`/`sed`/`awk`/`xargs`
//! 有写向旁路面不入表）。写向保护保留面逐钉对照（落点位 `.gsa` 全拦）。
//! **212 修复（2026-10-07，209 全民审查 P1 两族旁路收口）**：读向豁免的
//! 落点竞选旁路——(a) nullish 重定向目标位与 fd 重定向残片（`2>` 分词产出
//! 裸数字词元）不再竞选"末位落点"（修复前 `cp a .gsa/b > /dev/null`、
//! `… >/dev/null 2>&1` 整命令放行）；(b) 目标值型旗
//! （[`COPY_DEST_VALUE_FLAGS`]：`-t`/`--target-directory` 等闭集）在段＝
//! 整段不豁免（修复前旗值落点位 `.gsa` 被当源位豁免；`=` 形由主扫描 kv
//! 拆值本已覆盖）。真实语料零实踩（209 调查：官方轮 5 起写向全被拦、
//! `.gsa` 卷影 234 快照零模型异物），修复为不变量卫生性质；误伤面＝经
//! `-t` 从 `.gsa` 读向复制的罕见形状（保守方向既登记代价）。

use std::path::{Path, PathBuf};

use crate::types::write_control;

/// block 规则封闭集（恰 5 条；v2 设计 §1 表——**新增表项必改本表与测试**）。
pub const BLOCK_RULES: [&str; 5] = [
    "catastrophic-recursive-delete",
    "raw-device-write",
    "boot-firmware-flip",
    "registry-hive-delete",
    "carrier-write",
];

/// warn 规则封闭集（纯留痕、不阻断）。
pub const WARN_RULES: [&str; 2] = ["broad-destructive", "elevation"];

/// 命中事实（block／warn 共用）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandFinding {
    /// 规则身份（封闭集：[`BLOCK_RULES`] / [`WARN_RULES`]）。
    pub rule: &'static str,
    /// 机械细节（含命中词元/目标路径与根）。
    pub detail: String,
}

/// 三分类审查结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandReview {
    /// 表外——零改动放行。
    Allow,
    /// 高危形态：留痕（结果头部行）但**不阻断**。
    Warn(CommandFinding),
    /// 锁死面命中：命令**不执行**，返回机械拒绝文案。
    Block(CommandFinding),
}

/// 0bw③（2026-09-27）：一次命令审查的结构化报告——`write_control_review`
/// journal 事件族的 producer 面（schema：`runtime/write-control-review-
/// event-payload-v0.2.schema.json`）。`CommandReview::report()` 构造。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CommandReviewReport {
    /// 分类（封闭集：`allow` / `warn` / `block`，与 schema 枚举一致）。
    pub review: String,
    /// 命中规则 id（封闭集见 [`CommandFinding::rule`]）；allow 恒 `None`。
    pub rule: Option<String>,
    /// 机械细节（命中词元/目标路径）；allow 恒 `None`。
    pub detail: Option<String>,
}

impl CommandReview {
    /// 结构化报告（审查判定已经发生；本函数零额外判定）。
    pub fn report(&self) -> CommandReviewReport {
        match self {
            CommandReview::Allow => CommandReviewReport {
                review: "allow".to_string(),
                rule: None,
                detail: None,
            },
            CommandReview::Warn(finding) => CommandReviewReport {
                review: "warn".to_string(),
                rule: Some(finding.rule.to_string()),
                detail: Some(finding.detail.clone()),
            },
            CommandReview::Block(finding) => CommandReviewReport {
                review: "block".to_string(),
                rule: Some(finding.rule.to_string()),
                detail: Some(finding.detail.clone()),
            },
        }
    }
}

/// 0bw③：审查报告的工具→宿主传递队列项。
///
/// 命令原文不随事件重复入账（tool_started 已载原文）——以 sha256＋长度
/// 关联。`call_id` 由 bash 工具落账点填充。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EnqueuedCommandReview {
    pub call_id: String,
    pub report: CommandReviewReport,
    /// 被审查命令的 sha256（小写 64 hex）。
    pub command_sha256: String,
    /// 被审查命令的字节长度（≥1；空命令在 bash 审查点的空命令防线被跳过，
    /// 不出事件——2026-09-27 复审 P2 使本不变量机械化）。
    pub command_len: u64,
}

/// 0bw③：审查报告队列——bash 工具 push、宿主 drain 的 `Resources` 通道
/// （`ReportedTaskCompletions` 同型 `State<Vec<_>>`）。
pub type CommandReviewQueue = crate::types::resources::State<Vec<EnqueuedCommandReview>>;

// ─── 规则表（v2 封闭枚举；设计 §1——恰 5 条 block，表外恒 allow）──────────

// 规则 3 `boot-firmware-flip`：引导／安全机制翻转类程序（程序位词元精确；
// `.exe`/`.com` 后缀剥离后比对）。format/diskpart/mkfs 等已移规则 2。
const FLIP_PROGRAMS: &[&str] = &[
    "bcdedit",
    "set-executionpolicy",
    "set-mppreference",
    "add-mppreference",
    "remove-mppreference",
    "set-mpcomputerstatus",
    "set-netfirewallprofile",
];

/// 规则 3 短语（程序词元＋紧随参数拼接，前缀匹配）。
const FLIP_PHRASES: &[&str] = &[
    "netsh advfirewall set",
    "netsh firewall set",
    "wevtutil cl",
    "clear-eventlog",
    "fltmc unload",
    "sc stop windefend",
    "sc config windefend",
    "sc delete windefend",
    "sc stop mpssvc",
    "sc config mpssvc",
    "sc delete mpssvc",
    "net stop windefend",
    "net stop mpssvc",
    "stop-service windefend",
    "stop-service mpssvc",
    "stop-service -name windefend",
    "stop-service -name mpssvc",
];

/// 规则 2 `raw-device-write`：卷/分区破坏类程序（程序位精确）。
const RAW_DEVICE_PROGRAMS: &[&str] = &["format", "diskpart"];
/// 规则 2 短语：卷影/备份删除。
const RAW_DEVICE_PHRASES: &[&str] = &["vssadmin delete", "wbadmin delete"];
/// 规则 2 块设备目标前缀（`dd of=` 值与 `mkfs*` 目标词共用；设计 §1 规则 2
/// 「落块设备」形态集——`/dev/vd*`（virtio，评测容器主盘形态）与
/// `/dev/mapper*`（LVM/设备映射器根卷）为 0cb 审查处理批登记增补；
/// `/dev/null` 显式豁免）。
const BLOCK_DEVICE_PREFIXES: &[&str] = &[
    "/dev/sd",
    "/dev/vd",
    "/dev/nvme",
    "/dev/mmcblk",
    "/dev/mapper",
];
/// 规则 2 块设备目标的 `\\.\PhysicalDrive*` 形态按子串命中——**仅限写侧
/// 目标位**（`of=` 值／`mkfs*` 目标词；0cb 审查处理批收窄：位置无关词元扫
/// 退役，读侧 `dd if=\\.\PhysicalDrive0`（备份/取证）与查询类提及不拦）。
const PHYSICAL_DRIVE_TOKEN: &str = "physicaldrive";
/// raw 设备显式豁免（`+O/dev/null`、`>/dev/null`、`dd of=/dev/null` 一律放行）。
const NULL_DEVICE: &str = "/dev/null";

/// 规则 1 `catastrophic-recursive-delete`：删除动词（封闭；设计 §1——
/// `rm`/`rmdir`/`rd`/`del`/`erase`/`Remove-Item`＋`ri`（Remove-Item 的
/// PowerShell 别名，0cb 审查处理批补齐——否则 `ri C:\Windows -Recurse`
/// 落 warn 不 block））。
const CATASTROPHIC_DELETE_VERBS: &[&str] =
    &["rm", "rmdir", "rd", "del", "erase", "remove-item", "ri"];

/// 块设备目标形态判定（规则 2 收窄后唯一判定入口：`dd of=` 值与 `mkfs*`
/// 目标词共用；`/dev/null` 豁免由调用方先行）。
fn is_block_device_target(value: &str) -> bool {
    BLOCK_DEVICE_PREFIXES.iter().any(|p| value.starts_with(p))
        || value.contains(PHYSICAL_DRIVE_TOKEN)
}

/// 规则 1 递归旗（封闭；设计 §1——`-r`/`-rf`/`-R`/`--recursive`/`-Recurse`/`/s`）。
const CATASTROPHIC_RECURSIVE_FLAGS: &[&str] = &["-r", "-rf", "-R", "--recursive", "-recurse", "/s"];

/// 规则 4 `registry-hive-delete`：`reg` 修改子命令（封闭三值）与受保护蜂巢。
const REG_MODIFY_SUBCOMMANDS: &[&str] = &["add", "delete", "import"];
const REG_PROTECTED_HIVES: &[&str] = &[
    "hklm",
    "hkey_local_machine",
    "hkcr",
    "hkey_classes_root",
    "hku",
    "hkey_users",
];

/// 规则 5 载体自保护写动词（破坏/修改面；命令内目标命中载体集即 block）。
/// `install`／`ln` 为 0cc v3.1 审查处理批补齐（P3-1——两族可向宿主态目标
/// 落盘/建链接：`install -m 600 … <keystore>`、`ln -sf … <manifest>`；
/// `install to /usr/local/bin` 等非宿主态目标仍放行——目标比对在后）。
const DESTRUCTIVE_VERBS: &[&str] = &[
    "rm",
    "rmdir",
    "rd",
    "del",
    "erase",
    "remove-item",
    "ri",
    "rmtree",
    "mv",
    "move",
    "move-item",
    "mi",
    "rename-item",
    "ren",
    "rename",
    "cp",
    "copy",
    "copy-item",
    "cpi",
    "xcopy",
    "robocopy",
    "install",
    "ln",
    "set-content",
    "add-content",
    "out-file",
    "new-item",
    "ni",
    "mkdir",
    "md",
    "touch",
    "tee",
    "icacls",
    "takeown",
    "attrib",
    "cacls",
    "set-acl",
    "chmod",
    "chown",
];

/// 删除类动词（broad-destructive 的前置）。
const DELETE_VERBS: &[&str] = &[
    "rm",
    "rmdir",
    "rd",
    "del",
    "erase",
    "remove-item",
    "ri",
    "rmtree",
];

/// 祖先扫荡动词（0cc v3.1 审查处理批——宿主态祖先链臂的动词门，P2）：删除＋
/// 搬移两族——扫荡删除或搬走受护目标的祖先即连带摧毁受护本体；`cp`/`mkdir`/
/// `install` 等入位写不毁祖先，不入本表。封闭子集：DELETE_VERBS 全体＋搬移族
/// （均 ∈ [`DESTRUCTIVE_VERBS`]，钉子断言包含关系防漂移）。
const ANCESTOR_SWEEP_VERBS: &[&str] = &[
    "rm",
    "rmdir",
    "rd",
    "del",
    "erase",
    "remove-item",
    "ri",
    "rmtree",
    "mv",
    "move",
    "move-item",
    "mi",
    "rename-item",
    "ren",
    "rename",
];

/// 读模式选项值位（0cq S2，2026-10-04）：这些选项的**值位词元**是匹配模式/
/// 查询表达式而非写目标——规则 5 目标扫描跳过其值（读语境豁免；闭集，新增
/// 表项必改测试）。动机＝recli 三跑误拦①（181 批 §4b / 0cq S1 勘定）：find
/// 的读排除模式 `-not -path '/workspace/.gsa/*'` 被词法提取为写目标——模型
/// 显式避开 `.gsa` 反被拦。
const READ_PATTERN_OPTIONS: &[&str] = &[
    "-path",
    "-ipath",
    "-lname",
    "-ilname",
    "-name",
    "-iname",
    "-regex",
    "-iregex",
    "-wholename",
    "--exclude",
    "--include",
    "--exclude-dir",
    "--include-dir",
    "--glob",
    "--iglob",
];

/// 读模式选项的 kv 单词元形态前缀（`--exclude=…`；与 [`READ_PATTERN_OPTIONS`]
/// 同族闭集——前词豁免只看独立词元，kv 形态的值在同一词元内）。
const READ_PATTERN_KV_PREFIXES: &[&str] = &[
    "--exclude=",
    "--include=",
    "--exclude-dir=",
    "--include-dir=",
    "--glob=",
    "--iglob=",
];

/// 非写目标的重定向目标位（0cq S2 arm 收窄）：`>/dev/null`／`>NUL`／fd 数字
/// （`2>&1` 经词法拆分后的 `1`）不构成写目标——仅含此类重定向的命令不武装
/// 规则 5 目标扫描（纯读＋弃音槽不该开写目标扫描）。
fn is_nullish_redirect_target(text: &str) -> bool {
    let n = norm_word(text);
    n == "/dev/null" || n == "nul" || n.chars().all(|c| c.is_ascii_digit())
}

// ─── 0ct 读方向词位豁免（2026-10-07，`.gsa` 读全开放、只读不改）──────────

/// 复制族写动词（读向源位豁免集）：复制/安装类动词**不改动源位**——源是
/// 读方向，仅落点（目标位词元）是写方向。**不入表**：`mv` 族（搬移即源位
/// 写——移出 `.gsa` 仍拦）、`ln`（硬链接目标位＝写别名向量，保守照扫）、
/// `tee`（多落点同时写）。闭集，新增表项必改钉子。
const COPY_SOURCE_EXEMPT_VERBS: &[&str] = &[
    "cp",
    "copy",
    "copy-item",
    "cpi",
    "xcopy",
    "robocopy",
    "install",
];

/// 目标位＝**第 2** 位置词元的复制族（`src dst` 语序——末位是旗
/// `/MIR` 等不是落点；末位判定会漏拦落点位 `.gsa`）。闭集。
const COPY_DEST_SECOND_VERBS: &[&str] = &["xcopy", "robocopy"];

/// 操作数恒纯读的程序（重定向武装段豁免集）：段程序 ∈ 本表时除重定向
/// 目标位外全部词位＝读方向（`cat .gsa/x > /tmp/y` 是读不是写）。
/// **不入表**（写向旁路面，保守照扫）：`find`（-delete/-exec）、`sort`
/// （-o 落点）、`sed`（-i 原位写）、`awk`（system() 旁路）、`xargs`
/// （派生执行）、`tee`（多落点）。闭集，新增表项必改钉子。
const OPERAND_PURE_READ_PROGRAMS: &[&str] = &[
    "cat",
    "ls",
    "head",
    "tail",
    "grep",
    "egrep",
    "fgrep",
    "rg",
    "stat",
    "file",
    "wc",
    "diff",
    "strings",
    "less",
    "more",
    "md5sum",
    "sha1sum",
    "sha256sum",
    "cksum",
    "readlink",
    "realpath",
    "dirname",
    "basename",
    "which",
];

/// 目标值型旗（复制族闭集，209 审查 P1 族 b）：段内出现任一（norm 后）即
/// **整段不豁免**（保守全扫＝0ct 前行为）——此类旗的**分离值是落点**，末位
/// 位置词元启发式会把真落点当源位豁免（`cp -t <gsa> src`、`install -t …`）；
/// `=` 形（`--target-directory=<gsa>`）不经本表、由主扫描
/// [`path_candidates`] 的 kv 拆值直接覆盖。误伤面＝经 `-t` 从 `.gsa` 读向
/// 复制的罕见形状（保守方向：宁可多扫）。闭集，新增表项必改钉子。
const COPY_DEST_VALUE_FLAGS: &[&str] =
    &["-t", "--target-directory", "-destination", "--destination"];

/// 规则 5 写段内的**读方向词位**豁免集（0ct）：写方向不变量（「只读不改」）
/// 只及真正的落点。返回段内豁免词位下标表；两类豁免——
/// (a) 复制族段：源位位置词元（目标位可辨时）——目标位＝末个位置词元
///     （[`COPY_DEST_SECOND_VERBS`] 为第 2 个）；目标位不可辨（无位置词元）
///     时**不豁免**（保守全扫——命中即拒、绝不因解析失败放行）。`-` 旗、
///     重定向符号与重定向目标位不进位置词元收集——**目标位不问 nullish**
///     （209 审查 P1 族 a：`/dev/null` 曾留在位置词元里竞选成"末位落点"，
///     把真落点位 `.gsa` 豁免成源位）；裸数字词元（`2>` 经分词产出 `2`＋`>`
///     两词元、`2>&1` 的 `&` 为段分隔）同为重定向残片、不作落点竞选；
///     [`COPY_DEST_VALUE_FLAGS`] 在段即整段不豁免（族 b）。
/// (b) 纯读程序段（[`OPERAND_PURE_READ_PROGRAMS`]）：除重定向符号与其
///     非空目标位外全部词位。
/// 表外动词/程序返回空表（保守全扫＝0ct 前行为）。
fn read_direction_exempt_positions(seg: &[Word]) -> Vec<usize> {
    let Some(pi) = segment_prog_index(seg) else {
        return Vec::new();
    };
    let prog = norm_prog(&seg[pi].text);
    if COPY_SOURCE_EXEMPT_VERBS.contains(&prog.as_str()) {
        // 族 b（209）：目标值型旗在段＝落点不可辨 → 整段不豁免（保守全扫）。
        if seg
            .iter()
            .any(|w| COPY_DEST_VALUE_FLAGS.contains(&norm_word(&w.text).as_str()))
        {
            return Vec::new();
        }
        // 重定向符号与其目标位（**不问 nullish**）＋裸数字词元（`2>` 分词
        // 残片）——全部不进位置词元收集（主扫描恒覆盖，绝不作落点竞选）。
        let mut skip: Vec<usize> = Vec::new();
        for (i, w) in seg.iter().enumerate() {
            if w.text == ">" || w.text == ">>" {
                if seg.get(i + 1).is_some() {
                    skip.push(i + 1);
                }
                continue;
            }
            if !w.text.is_empty() && w.text.chars().all(|c| c.is_ascii_digit()) {
                skip.push(i);
            }
        }
        let positional: Vec<usize> = seg
            .iter()
            .enumerate()
            .skip(pi + 1)
            .filter(|(i, w)| {
                !norm_word(&w.text).starts_with('-')
                    && w.text != ">"
                    && w.text != ">>"
                    && !skip.contains(i)
            })
            .map(|(i, _)| i)
            .collect();
        let dest = if COPY_DEST_SECOND_VERBS.contains(&prog.as_str()) {
            positional.get(1).copied()
        } else {
            positional.last().copied()
        };
        return match dest {
            Some(dest) => positional.into_iter().filter(|i| *i != dest).collect(),
            None => Vec::new(),
        };
    }
    if OPERAND_PURE_READ_PROGRAMS.contains(&prog.as_str()) {
        let mut exempt = Vec::with_capacity(seg.len());
        let mut after_redirect = false;
        for (i, w) in seg.iter().enumerate() {
            if w.text == ">" || w.text == ">>" {
                after_redirect = true;
                continue;
            }
            if after_redirect {
                after_redirect = false;
                continue;
            }
            exempt.push(i);
        }
        return exempt;
    }
    Vec::new()
}

/// 段程序位下标（镜像 [`program_entries`] 头部的包装词/赋值跳过逻辑）；
/// `None`＝段无程序位——内容位词元（`bash -c "…"`）交由内容递归条目，
/// 本段自身不判。
fn segment_prog_index(seg: &[Word]) -> Option<usize> {
    let mut idx = 0;
    while idx < seg.len() {
        let w = &seg[idx];
        let n = norm_word(&w.text);
        if is_assignment(&w.text) || WRAPPER_WORDS.contains(&n.as_str()) {
            if CONTENT_WORDS.contains(&n.as_str()) {
                return None;
            }
            idx += 1;
            continue;
        }
        return Some(idx);
    }
    None
}

/// 段内是否含**非 nullish** 重定向（与全局 `redirect_arms` arm 收窄同口径的
/// 段级版；186 批审查处置：`segment_is_write` 的重定向臂由「段内有 `>`」
/// 收窄为本判定——复合命令中无关写动词武装规则 5 后，仅含 `2>/dev/null`
/// 类弃音槽的读段不再被当写段扫描；`>` 为段末词（无目标位）保守按真处理）。
fn segment_has_real_redirect(seg: &[Word]) -> bool {
    seg.iter().enumerate().any(|(i, w)| {
        (w.text == ">" || w.text == ">>")
            && seg
                .get(i + 1)
                .is_none_or(|n| !is_nullish_redirect_target(&n.text))
    })
}

/// 段是否为写段（0cq S2 规则 5 扫描局部化）：程序位 ∈ 破坏/修改集∪`dd`，
/// 或段内含**非 nullish** 重定向词（186 批审查处置收窄）。目标扫描仅及写段
/// ——读段（`ls`/`find`/`grep` 段）的词元不再逐个比对宿主态目标。
fn segment_is_write(seg: &[Word]) -> bool {
    if segment_has_real_redirect(seg) {
        return true;
    }
    segment_prog_index(seg)
        .map(|i| {
            let prog = norm_prog(&seg[i].text);
            prog == "dd" || DESTRUCTIVE_VERBS.contains(&prog.as_str())
        })
        .unwrap_or(false)
}

/// 递归/强制旗（broad-destructive 的递归腿；warn 面，v1 集沿用）。
const RECURSIVE_FLAGS: &[&str] = &["-r", "-rf", "-fr", "-recurse", "-force", "/s", "/q"];

/// 提权程序（elevation warn）。
const ELEVATION_PROGRAMS: &[&str] = &["sudo", "doas", "gsudo", "runas"];

/// wrapper/前缀词（找程序位时跳过；内容位 `-command` 等递归展开）。
const WRAPPER_WORDS: &[&str] = &[
    "sudo",
    "doas",
    "gsudo",
    "env",
    "nohup",
    "time",
    "exec",
    "command",
    "xargs",
    "cmd",
    "cmd.exe",
    "/c",
    "/k",
    "powershell",
    "powershell.exe",
    "pwsh",
    "pwsh.exe",
    "-command",
    "-c",
    "-lc",
    "-l",
    "bash",
    "bash.exe",
    "sh",
    "sh.exe",
    "zsh",
    "zsh.exe",
    "runas",
    "start-process",
    "-noprofile",
    "-nologo",
    "-verb",
];

/// 内容位词元（其后的内容词按子命令递归展开）。
const CONTENT_WORDS: &[&str] = &["-command", "-c", "-lc", "/c", "/k"];

// ─── 公开入口 ───────────────────────────────────────────────────────────

/// 审查一条 `run_terminal_cmd` 命令（生产入口：宿平台根本树根＋装配期宿主
/// 状态两窄目标）。
pub fn review_command(cwd: &Path, command: &str) -> CommandReview {
    let host_state = write_control::HostStateTargets::from_env();
    review_command_with(
        cwd,
        command,
        &host_state,
        &write_control::disaster_tree_roots(),
    )
}

/// 审查（注入式；测试与跨平台场景）。`disaster_roots`＝规则 1 的根本树根目标集；
/// `host_state`＝规则 5 的宿主状态两窄目标（0cc v3）。
///
/// 规则序＝设计 §1 表序（① catastrophic-recursive-delete → ② raw-device-write
/// → ③ boot-firmware-flip → ④ registry-hive-delete → ⑤ carrier-write）；
/// 随后 warn 面（broad-destructive／elevation 纯留痕）。表外恒 `Allow`。
pub fn review_command_with(
    cwd: &Path,
    command: &str,
    host_state: &write_control::HostStateTargets,
    disaster_roots: &[PathBuf],
) -> CommandReview {
    let scan = crate::util::unicode_confusables::normalize_confusables(command);
    let toks = tokenize(&scan);
    let words: Vec<Word> = toks
        .iter()
        .filter_map(|t| match t {
            Tok::Word(w) => Some(w.clone()),
            Tok::Sep => None,
        })
        .collect();
    let segments = segments(&toks);

    let mut entries: Vec<ProgEntry> = Vec::new();
    for seg in &segments {
        program_entries(seg, 0, &mut entries);
    }

    // ① catastrophic-recursive-delete：删除动词＋递归旗＋目标解析后**恰为**
    //    卷根／根本性树根本体（子目录级精准删除放行——「需精准删除」硬边界）。
    //    **0cq S2（2026-10-04）扫描精准化**：动词与递归旗**同段**才武装；
    //    目标扫描仅及**动词位之后**的本段词——复合命令其他段的词元（如头部
    //    `cd /` 的裸 `/`、echo 散文）不再被当作删除目标（recli 三跑误拦②
    //    ＝`echo "=== csv / json / yaml ==="` 经散文拆片伪造 `/` 词元 × 全局
    //    扫描，误报卷根删除；0cq S1 勘定）。
    for seg in &segments {
        let Some(pi) = segment_prog_index(seg) else {
            continue;
        };
        if !CATASTROPHIC_DELETE_VERBS.contains(&norm_prog(&seg[pi].text).as_str()) {
            continue;
        }
        if !seg
            .iter()
            .any(|w| CATASTROPHIC_RECURSIVE_FLAGS.contains(&norm_word(&w.text).as_str()))
        {
            continue;
        }
        for w in seg.iter().skip(pi + 1) {
            for raw in path_candidates(&w.text) {
                for form in disaster_target_forms(cwd, &raw) {
                    if write_control::is_volume_root(&form) {
                        return CommandReview::Block(CommandFinding {
                            rule: "catastrophic-recursive-delete",
                            detail: format!("recursive delete targets the volume root (`{raw}`)"),
                        });
                    }
                    if let Some(root) = disaster_roots
                        .iter()
                        .find(|r| write_control::path_equals_root(r, &form))
                    {
                        return CommandReview::Block(CommandFinding {
                            rule: "catastrophic-recursive-delete",
                            detail: format!(
                                "recursive delete targets a fundamental tree root \
                                 (`{raw}` ⇒ `{}`)",
                                root.to_string_lossy()
                            ),
                        });
                    }
                }
            }
        }
    }

    // ② raw-device-write：卷/分区破坏程序（format/diskpart 程序位）＋卷影
    //    删除＋`dd of=`/`mkfs*` 目标落块设备（`/dev/null` 显式豁免；目标位
    //    判定——读侧 `if=` 与查询类提及放行，0cb 审查处理批收窄）。
    for e in &entries {
        if RAW_DEVICE_PROGRAMS.contains(&e.prog.as_str()) {
            return CommandReview::Block(CommandFinding {
                rule: "raw-device-write",
                detail: format!("matched the raw-device-write rule (`{}`)", e.prog),
            });
        }
        if e.prog.starts_with("mkfs") {
            // `mkfs*` 目标须落块设备形态才拦（镜像文件构建——`mkfs.ext4
            // disk.img`——放行交审批组件；v2.1 收窄）。
            if e.words.iter().skip(1).any(|w| is_block_device_target(w)) {
                return CommandReview::Block(CommandFinding {
                    rule: "raw-device-write",
                    detail: format!("`mkfs` targeting a raw block device (`{}`)", e.prog),
                });
            }
        }
        if RAW_DEVICE_PHRASES
            .iter()
            .any(|ph| e.phrase == *ph || e.phrase.starts_with(&format!("{ph} ")))
        {
            return CommandReview::Block(CommandFinding {
                rule: "raw-device-write",
                detail: format!("matched the raw-device-write rule (`{}`)", e.phrase),
            });
        }
    }
    if entries.iter().any(|e| e.prog == "dd") {
        for w in &words {
            let normalized = norm_word(&w.text);
            let Some(value) = normalized.strip_prefix("of=") else {
                continue;
            };
            if value == NULL_DEVICE {
                continue;
            }
            if is_block_device_target(value) {
                return CommandReview::Block(CommandFinding {
                    rule: "raw-device-write",
                    detail: format!("`dd` writing a raw block device (`{value}`)"),
                });
            }
        }
    }

    // ③ boot-firmware-flip（沿 v1 safety-mechanism-flip 全集，去规则 2 移出项）。
    for e in &entries {
        if FLIP_PROGRAMS.contains(&e.prog.as_str()) {
            return CommandReview::Block(CommandFinding {
                rule: "boot-firmware-flip",
                detail: format!("matched the boot-firmware-flip rule (`{}`)", e.prog),
            });
        }
        if FLIP_PHRASES
            .iter()
            .any(|ph| e.phrase == *ph || e.phrase.starts_with(&format!("{ph} ")))
        {
            return CommandReview::Block(CommandFinding {
                rule: "boot-firmware-flip",
                detail: format!("matched the boot-firmware-flip rule (`{}`)", e.phrase),
            });
        }
    }

    // ④ registry-hive-delete：`reg add|delete|import` 落 HKLM/HKCR/HKU。
    //    目标位判定（0cb 审查处理批收窄）：`reg <sub> <target> …` 的第三词元
    //    （add/delete＝键路径、import＝脚本文件名）之前缀判定——任何位置的
    //    蜂巢提及（如 `/d hklm-…` 数据值）不再误拦。
    for e in &entries {
        if e.prog == "reg" {
            let sub = e.words.get(1).map(String::as_str).unwrap_or("");
            let hive = e
                .words
                .get(2)
                .map(|t| REG_PROTECTED_HIVES.iter().any(|h| t.starts_with(h)))
                .unwrap_or(false);
            if REG_MODIFY_SUBCOMMANDS.contains(&sub) && hive {
                return CommandReview::Block(CommandFinding {
                    rule: "registry-hive-delete",
                    detail: "registry add/delete/import on HKLM/HKCR/HKU".to_owned(),
                });
            }
        }
    }

    // ⑤ carrier-write：写动词（破坏/修改集＋`dd`＋重定向）＋目标命中**宿主
    //    状态两窄目标**（`.gsa` 会话卷／ACAF keystore 根／signer manifest；
    //    恒拒面）。v2 系统树不进本闸；v3 载体面（安装目录／三件套／
    //    `grok-home`）随 0cc 双面退役——`/usr/local/bin` 安装等载体面写交回
    //    审批组件（设计 §2 条 5）。v3.1 祖先链臂（P2）：扫荡动词（删除/搬移）
    //    下目标为受护目标祖先同落本规则——载体面退役不得连带放行 keystore／
    //    manifest 的扫荡摧毁；入位写（cp/mkdir/install）不触发祖先臂。
    //    **0cq S2（2026-10-04）三面收窄**（recli 三跑误拦①族；0cq S1 勘定）：
    //    (a) **arm 面**——重定向词仅当目标位非 null/fd 数字时武装
    //    （`2>/dev/null` 纯读弃音槽不开写目标扫描）；
    //    (b) **扫描面段内化**——目标扫描仅及**写段**（程序位 ∈ 破坏/修改集
    //    ∪`dd`，或段内含**非 nullish** 重定向词〔186 批审查处置收窄〕）；
    //    读段（ls/find/grep 段）词元不再逐个比对；
    //    (c) **读模式值豁免**——[`READ_PATTERN_OPTIONS`] 值位词元与
    //    [`READ_PATTERN_KV_PREFIXES`] 形态不作写目标（find 排除模式等）；
    //    (d) **读方向词位豁免（0ct，2026-10-07）**——写段内源位词元按
    //    [`read_direction_exempt_positions`] 豁免（复制族源位＋纯读程序段
    //    非目标位；`.gsa` 台账读全开放、只读不改——落点位写向保护保留）。
    let redirect_arms = words.iter().enumerate().any(|(i, w)| {
        (w.text == ">" || w.text == ">>")
            && words
                .get(i + 1)
                .is_none_or(|n| !is_nullish_redirect_target(&n.text))
    });
    let has_write_verb = entries
        .iter()
        .any(|e| DESTRUCTIVE_VERBS.contains(&e.prog.as_str()) || e.prog == "dd")
        || redirect_arms;
    let has_sweep_verb = entries
        .iter()
        .any(|e| ANCESTOR_SWEEP_VERBS.contains(&e.prog.as_str()));
    if has_write_verb || has_sweep_verb {
        for seg in &segments {
            if !segment_is_write(seg) {
                continue;
            }
            let exempt = read_direction_exempt_positions(seg);
            for (i, w) in seg.iter().enumerate() {
                if w.text == ">" || w.text == ">>" {
                    continue;
                }
                if exempt.contains(&i) {
                    continue;
                }
                let norm = norm_word(&w.text);
                if READ_PATTERN_KV_PREFIXES.iter().any(|p| norm.starts_with(p)) {
                    continue;
                }
                if i > 0 && READ_PATTERN_OPTIONS.contains(&norm_word(&seg[i - 1].text).as_str()) {
                    continue;
                }
                for raw in path_candidates(&w.text) {
                    if let Some(detail) =
                        carrier_target_detail(cwd, &raw, host_state, has_sweep_verb)
                    {
                        return CommandReview::Block(CommandFinding {
                            rule: "carrier-write",
                            detail,
                        });
                    }
                }
            }
        }
    }

    // ⑥ broad-destructive（warn 留痕；v1 臂沿用——根级递归删除已在 ① 转 block）。
    let delete_present = entries
        .iter()
        .any(|e| DELETE_VERBS.contains(&e.prog.as_str()));
    if delete_present {
        for w in &words {
            if w.text == ">" || w.text == ">>" {
                continue;
            }
            let expanded = expand_word(cwd, &w.text);
            if is_rootish(&expanded) || w.text.contains('*') || w.text.contains('?') {
                return CommandReview::Warn(CommandFinding {
                    rule: "broad-destructive",
                    detail: format!("broad/root-level delete target (`{}`)", w.text),
                });
            }
        }
        let recursive = words
            .iter()
            .any(|w| RECURSIVE_FLAGS.contains(&norm_word(&w.text).as_str()));
        if recursive {
            for w in &words {
                for raw in path_candidates(&w.text) {
                    let forms = path_forms(cwd, &raw);
                    if forms.iter().all(|f| !is_within(cwd, f)) {
                        return CommandReview::Warn(CommandFinding {
                            rule: "broad-destructive",
                            detail: format!(
                                "recursive-force delete outside the workspace (`{}`)",
                                w.text
                            ),
                        });
                    }
                }
            }
        }
    }

    // ⑦ elevation（warn 留痕）。
    for seg in &segments {
        if let Some(first) = seg.iter().find(|w| !is_assignment(&w.text)) {
            let n = norm_word(&first.text);
            if ELEVATION_PROGRAMS.contains(&n.as_str()) {
                return CommandReview::Warn(CommandFinding {
                    rule: "elevation",
                    detail: format!("privilege elevation (`{}`)", n),
                });
            }
        }
    }
    if words
        .windows(2)
        .any(|pair| norm_word(&pair[0].text) == "-verb" && norm_word(&pair[1].text) == "runas")
    {
        return CommandReview::Warn(CommandFinding {
            rule: "elevation",
            detail: "privilege elevation (`-Verb RunAs`)".to_owned(),
        });
    }

    CommandReview::Allow
}

/// block 面向模型的机械拒绝文案（经 `ToolError` 返回；随 tool 结果入 journal）。
///
/// v2 文案（设计 §1）：规则 id＋目标＋「已越过保底硬边界」；规则 1 附「需精准
/// 删除」指引（写明被拦目标、建议改为具体文件/子目录——用户裁决原话）。
/// block 文案冻结前缀（0am P8，2026-10-03）：写控兜底 block 的机械判定
/// 锚（loop 侧 ToolError 臂据此置 `write_control_block` 路由键；S2 v1.1
/// 写控 join 的生产实现）。与 [`block_message`] 单源。
pub const BLOCK_MESSAGE_PREFIX: &str =
    "Error: command blocked by the mechanical write control backstop";

pub fn block_message(finding: &CommandFinding) -> String {
    let guidance = if finding.rule == "catastrophic-recursive-delete" {
        "该目标是卷根/根本性树根本体；如需删除，请改为对具体文件或子目录的精准删除。"
    } else {
        ""
    };
    // 214（0ct 教育面；212 考虑项 i 落地，用户裁「同意，请落成小批」）：
    // `.gsa` 会话卷拦截附**备份/暂存指引**——0cs 同构（错误信封内给可执行
    // 替代物；应答式非主动提醒，不涉 P9 隐式提醒面）。写因调查（212 档 §1）
    // 实证模型受挫后把 harness 逐调用备份区当自己备份库（官方 5 起写向中
    // 3 起直接对症：mtl/ttr 备份入 rollback、l2m 写 journal）。仅会话卷臂
    // 携带（keystore/manifest/祖先臂是秘密与载体保护，备份指引不对症）；
    // 置于「未执行」语义之后，不软化拦截。
    let gsa_guidance =
        if finding.rule == "carrier-write" && finding.detail.contains("session volume") {
            "若意图是备份/暂存：`/tmp` 与工作区写向不受本闸限制；运行状态可记 \
         blackboard_write；`.gsa/rollback` 为运行时逐调用备份区，只读使用。"
        } else {
            ""
        };
    format!(
        "{prefix} (rule: {rule}). \
         {detail}. 已越过保底硬边界——命令未执行。{guidance}{gsa_guidance}本闸为封闭枚举灾难保底\
         （{n} 条 block 规则），非沙箱。",
        rule = finding.rule,
        prefix = BLOCK_MESSAGE_PREFIX,
        detail = finding.detail,
        guidance = guidance,
        gsa_guidance = gsa_guidance,
        n = BLOCK_RULES.len(),
    )
}

/// warn 结果头部行（随 tool 结果入 journal；不阻断）。
pub fn warn_line(finding: &CommandFinding) -> String {
    format!(
        "[写入管控·提示] {}（rule: {}；动作照常执行；best-effort 机械闸）",
        finding.detail, finding.rule
    )
}

// ─── 目标解析与载体集比对 ───────────────────────────────────────────────

/// 规则 5 目标解析：展开 → 绝对化 → 词法/近祖先 canonical → **宿主状态两窄
/// 目标**比对（`.gsa` 会话卷／ACAF keystore 根／signer manifest）。v2 退役面：
/// 系统树根比对与 `/dev//proc//sys` 前缀词元判不再进本闸（重定向目标、一般
/// 路径写交回审批组件——设计 §2）；v3 退役面：安装目录／三件套／`grok-home`
/// 比对随载体集退役（0cc §2 条 5）。`sweep_verb`＝命令含祖先扫荡动词
/// （v3.1 P2——祖先链臂仅在此下比对，先直接后祖先）。
fn carrier_target_detail(
    cwd: &Path,
    raw: &str,
    host_state: &write_control::HostStateTargets,
    sweep_verb: bool,
) -> Option<String> {
    let forms = path_forms(cwd, raw);
    let gsa = cwd.join(".gsa");
    let gsa_canonical = crate::types::resources::session_volume_canonical_root(cwd);
    for form in &forms {
        if write_control::path_hits_root(&gsa, form)
            || write_control::path_hits_root(&gsa_canonical, form)
        {
            return Some(format!(
                "target `{raw}` is inside the `.gsa` session volume"
            ));
        }
    }
    if let Some(hit) = host_state.hit(&forms) {
        return Some(match hit.rule {
            "carrier:keystore-root" => format!(
                "target `{raw}` is inside the ACAF keystore root (`{}`)",
                hit.root
            ),
            "carrier:signer-manifest" => format!(
                "target `{raw}` is the ACAF signer manifest (`{}`)",
                hit.root
            ),
            _ => format!(
                "target `{raw}` is inside the host-state protection set (`{}`)",
                hit.root
            ),
        });
    }
    if sweep_verb && let Some(hit) = host_state.hit_ancestor(&forms) {
        return Some(match hit.rule {
            "carrier:keystore-ancestor" => format!(
                "target `{raw}` is an ancestor of the ACAF keystore root (`{}`); \
                 sweeping it would destroy protected host state",
                hit.root
            ),
            "carrier:signer-manifest-ancestor" => format!(
                "target `{raw}` is an ancestor of the ACAF signer manifest (`{}`); \
                 sweeping it would destroy protected host state",
                hit.root
            ),
            _ => format!(
                "target `{raw}` is an ancestor of the host-state protection set (`{}`)",
                hit.root
            ),
        });
    }
    None
}

/// 词元路径形态（相对路径按 cwd 拼接；词内空白拆片兜底）。
fn path_forms(cwd: &Path, raw: &str) -> Vec<PathBuf> {
    let expanded = expand_word(cwd, raw);
    let p = PathBuf::from(&expanded);
    let abs = if p.is_absolute() { p } else { cwd.join(&p) };
    let lexical = orz_paths::normalize_lexically(&abs);
    write_control::candidate_forms(&lexical, None)
}

/// 规则 1 目标形态：`has_root` 形态（POSIX 风格 `/`、`/usr`——Windows 上
/// `is_absolute()` 为假）**保持原样不拼 cwd**（跨平台比对卷根/根本树根本体），
/// 其余同 [`path_forms`]。Windows 上另有盘符相对根形态补全（见
/// [`push_drive_qualified_forms`]）。
fn disaster_target_forms(cwd: &Path, raw: &str) -> Vec<PathBuf> {
    let expanded = expand_word(cwd, raw);
    let p = PathBuf::from(&expanded);
    let abs = if p.has_root() { p } else { cwd.join(&p) };
    let lexical = orz_paths::normalize_lexically(&abs);
    let mut forms = write_control::candidate_forms(&lexical, None);
    push_drive_qualified_forms(&mut forms);
    forms
}

/// Windows 盘符相对根形态补全（0cb 审查处理批，绕过面闭合）：`\Windows`、
/// `/usr` 等 `has_root() && !is_absolute()` 形态在 cmd/PowerShell 下
/// ≡ `%SystemDrive%` 上的同径路径，但与 `C:\Windows` 字面比对必败——按
/// `%SystemDrive%`（env 缺失回退 `C:`）生成盘符限定孪生形态一并比对。
/// POSIX 宿不存在该形态（`/` 开头即绝对），零操作。
fn push_drive_qualified_forms(forms: &mut Vec<PathBuf>) {
    #[cfg(windows)]
    {
        let drive = std::env::var("SystemDrive")
            .ok()
            .filter(|v| {
                let b = v.as_bytes();
                b.len() == 2 && b[1] == b':' && b[0].is_ascii_alphabetic()
            })
            .unwrap_or_else(|| "C:".to_owned());
        let mut qualified: Vec<PathBuf> = Vec::new();
        for form in forms.iter() {
            if form.has_root() && !form.is_absolute() {
                let twin = PathBuf::from(format!("{drive}{}", form.to_string_lossy()));
                if !forms.contains(&twin) && !qualified.contains(&twin) {
                    qualified.push(twin);
                }
            }
        }
        forms.extend(qualified);
    }
    #[cfg(not(windows))]
    {
        let _ = forms;
    }
}

/// 从词元提取路径候选（`of=…`／`-path=…` 取等号右值；引号剥除）。
/// **0cq S2（2026-10-04）拆片收紧**：等号右值保留空白拆片兜底（kv 值偶带
/// 空白的目标形态）；**裸词不再拆片**——recli 三跑误拦三例同根（181 批
/// §4b / 0cq S1 勘定）：echo 散文（`"=== csv / json / yaml ==="`、
/// `"(excluding .gsa/usr) =="`）被空白拆片成 `/`、`.gsa/usr)` 伪词元，
/// 经全局扫描分别误触规则 1 卷根与规则 5 `.gsa` 臂。引号内的空白本就是
/// 路径合法字符（整词候选直接可用），拆片只服务散文伪造面。
fn path_candidates(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    if let Some((key, value)) = text.split_once('=') {
        if key.is_empty() || key.starts_with('-') || key.eq_ignore_ascii_case("of") {
            push_candidate(&mut out, value);
            if value.contains(char::is_whitespace) {
                for piece in value.split_whitespace() {
                    push_candidate(&mut out, piece);
                }
            }
            return out;
        }
    }
    push_candidate(&mut out, text);
    out
}

fn push_candidate(out: &mut Vec<String>, value: &str) {
    let value = value.trim().trim_matches(|c| c == '"' || c == '\'');
    if looks_like_path(value) && !out.iter().any(|v| v == value) {
        out.push(value.to_owned());
    }
}

/// 近似路径形态判定（不追求完备：选项/赋值左值不取，含分隔符或特殊开头即候选；
/// 裸 `..` 取——递归删除目标的父级形态）。
fn looks_like_path(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    if s == ".." {
        return true;
    }
    let b = s.as_bytes();
    if b[0] == b'-' {
        return false;
    }
    let has_sep = s.contains('\\') || s.contains('/');
    let drive = b.len() >= 2 && b[1] == b':' && b[0].is_ascii_alphabetic();
    let special = s.starts_with('~')
        || s.starts_with('%')
        || s.starts_with("$env:")
        || s.starts_with("${")
        || s.starts_with("./")
        || s.starts_with("../")
        || s.starts_with(".\\")
        || s.starts_with("..\\")
        || s.starts_with('/')
        || s.starts_with('\\');
    drive || special || has_sep
}

// ─── 词元/环境展开 ──────────────────────────────────────────────────────

/// 环境引用展开（`%VAR%`／`$env:VAR`／`${VAR}`；值未知保持原样）。
fn expand_word(cwd: &Path, raw: &str) -> String {
    let mut s = raw.to_owned();
    if s == "~" || s.starts_with("~/") || s.starts_with("~\\") {
        if let Some(home) = home_dir() {
            s = format!("{home}{}", &s[1..]);
        }
    }
    for (name, value) in known_vars(cwd) {
        s = replace_ci(&s, &format!("%{name}%"), &value);
        s = replace_ci(&s, &format!("$env:{name}"), &value);
        s = replace_ci(&s, &format!("${{{name}}}"), &value);
        s = replace_ci(&s, &format!("${name}"), &value);
    }
    s
}

fn home_dir() -> Option<String> {
    std::env::var("USERPROFILE")
        .ok()
        .or_else(|| std::env::var("HOME").ok())
        .filter(|s| !s.is_empty())
}

fn known_vars(cwd: &Path) -> Vec<(String, String)> {
    let get = |k: &str| std::env::var(k).ok().filter(|v| !v.is_empty());
    vec![
        (
            "systemroot".to_owned(),
            get("SystemRoot").unwrap_or_else(|| r"C:\Windows".to_owned()),
        ),
        (
            "windir".to_owned(),
            get("SystemRoot").unwrap_or_else(|| r"C:\Windows".to_owned()),
        ),
        (
            "programfiles".to_owned(),
            get("ProgramFiles").unwrap_or_else(|| r"C:\Program Files".to_owned()),
        ),
        (
            "programfiles(x86)".to_owned(),
            get("ProgramFiles(x86)").unwrap_or_else(|| r"C:\Program Files (x86)".to_owned()),
        ),
        (
            "programdata".to_owned(),
            get("ProgramData").unwrap_or_else(|| r"C:\ProgramData".to_owned()),
        ),
        (
            "systemdrive".to_owned(),
            get("SystemDrive").unwrap_or_else(|| "C:".to_owned()),
        ),
        (
            "userprofile".to_owned(),
            get("USERPROFILE")
                .or_else(|| get("HOME"))
                .unwrap_or_default(),
        ),
        (
            "temp".to_owned(),
            get("TEMP").or_else(|| get("TMP")).unwrap_or_default(),
        ),
        ("cd".to_owned(), cwd.to_string_lossy().into_owned()),
        ("pwd".to_owned(), cwd.to_string_lossy().into_owned()),
    ]
}

/// ASCII 大小写不敏感替换（仅替字面量；逐位匹配）。
fn replace_ci(haystack: &str, needle: &str, value: &str) -> String {
    if needle.is_empty() || value.is_empty() {
        return haystack.to_owned();
    }
    let h = haystack.as_bytes();
    let n = needle.as_bytes();
    if n.len() > h.len() {
        return haystack.to_owned();
    }
    let mut out = String::with_capacity(haystack.len());
    let mut i = 0;
    while i < h.len() {
        if i + n.len() <= h.len() && h[i..i + n.len()].eq_ignore_ascii_case(n) {
            out.push_str(value);
            i += n.len();
        } else {
            // 保持 UTF-8 边界：单字节推入不安全，按字符推。
            let ch = haystack[i..].chars().next().unwrap();
            out.push(ch);
            i += ch.len_utf8();
        }
    }
    out
}

/// 根级目标判定（`/`、`C:\`、`~`、`$HOME`、通配符整体）。
fn is_rootish(expanded: &str) -> bool {
    let t = expanded.trim().trim_matches('"').trim_matches('\'');
    if t.is_empty() {
        return false;
    }
    let lower = t.to_ascii_lowercase();
    if lower == "/" || lower == "\\" || lower == "*" || lower == "/*" || lower == "\\*" {
        return true;
    }
    if matches!(
        lower.as_str(),
        "~" | "$home" | "$env:userprofile" | "%userprofile%"
    ) {
        return true;
    }
    // 盘根（`C:`、`C:\`、`C:/`）
    let b = lower.as_bytes();
    if b.len() == 2 && b[1] == b':' && b[0].is_ascii_alphabetic() {
        return true;
    }
    if b.len() == 3 && b[1] == b':' && (b[2] == b'\\' || b[2] == b'/') && b[0].is_ascii_alphabetic()
    {
        return true;
    }
    let p = Path::new(&lower);
    p.parent().is_none() && p.has_root()
}

fn is_within(base: &Path, candidate: &Path) -> bool {
    crate::types::resources::candidate_is_under(base, candidate)
}

fn is_assignment(text: &str) -> bool {
    match text.split_once('=') {
        Some((key, _)) => {
            !key.is_empty()
                && !key.starts_with('-')
                && !key.contains('\\')
                && !key.contains('/')
                && key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        }
        None => false,
    }
}

// ─── 词法：token 化与程序位 ─────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
struct Word {
    text: String,
    /// 整词由引号包裹（`-Command "…"` 内容递归的判据）。
    fully_quoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Tok {
    Word(Word),
    /// 段分隔（`;` `&&` `||` `|` `&` `(` `)` 换行）。
    Sep,
}

/// 程序位条目（`prog` 归一化词元；`phrase`＝prog 起最多 4 词拼接；`words`＝段内归一化词表）。
#[derive(Debug, Clone)]
struct ProgEntry {
    prog: String,
    phrase: String,
    words: Vec<String>,
}

fn tokenize(command: &str) -> Vec<Tok> {
    fn flush(toks: &mut Vec<Tok>, cur: &mut String, quoted: &mut bool) {
        if !cur.is_empty() || *quoted {
            toks.push(Tok::Word(Word {
                text: std::mem::take(cur),
                fully_quoted: *quoted,
            }));
            *quoted = false;
        }
    }
    let mut toks = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    let mut quote: Option<char> = None;
    let mut chars = command.chars().peekable();
    while let Some(c) = chars.next() {
        if let Some(q) = quote {
            if c == q {
                quote = None;
            } else {
                cur.push(c);
            }
            continue;
        }
        match c {
            '\'' | '"' => {
                if cur.is_empty() {
                    quoted = true;
                }
                quote = Some(c);
            }
            c if c.is_whitespace() => flush(&mut toks, &mut cur, &mut quoted),
            ';' | '\n' => {
                flush(&mut toks, &mut cur, &mut quoted);
                toks.push(Tok::Sep);
            }
            '&' => {
                flush(&mut toks, &mut cur, &mut quoted);
                if chars.peek() == Some(&'&') {
                    chars.next();
                }
                toks.push(Tok::Sep);
            }
            '|' => {
                flush(&mut toks, &mut cur, &mut quoted);
                if chars.peek() == Some(&'|') {
                    chars.next();
                }
                toks.push(Tok::Sep);
            }
            '(' | ')' => {
                flush(&mut toks, &mut cur, &mut quoted);
                toks.push(Tok::Sep);
            }
            '>' => {
                flush(&mut toks, &mut cur, &mut quoted);
                let mut text = String::from(">");
                if chars.peek() == Some(&'>') {
                    chars.next();
                    text.push('>');
                }
                toks.push(Tok::Word(Word {
                    text,
                    fully_quoted: false,
                }));
            }
            _ => cur.push(c),
        }
    }
    flush(&mut toks, &mut cur, &mut quoted);
    toks
}

fn segments(toks: &[Tok]) -> Vec<Vec<Word>> {
    let mut out = Vec::new();
    let mut current: Vec<Word> = Vec::new();
    for tok in toks {
        match tok {
            Tok::Word(w) => current.push(w.clone()),
            Tok::Sep => {
                if !current.is_empty() {
                    out.push(std::mem::take(&mut current));
                }
            }
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

fn program_entries(seg: &[Word], depth: u8, out: &mut Vec<ProgEntry>) {
    if depth > 2 {
        return;
    }
    let mut idx = 0;
    while idx < seg.len() {
        let w = &seg[idx];
        let n = norm_word(&w.text);
        if is_assignment(&w.text) || WRAPPER_WORDS.contains(&n.as_str()) {
            if CONTENT_WORDS.contains(&n.as_str())
                && let Some(next) = seg.get(idx + 1)
            {
                push_content_entry(next, seg, idx + 1, depth, out);
                idx += 2;
                continue;
            }
            idx += 1;
            continue;
        }
        break;
    }
    if idx >= seg.len() {
        return;
    }
    let words: Vec<String> = seg[idx..].iter().map(|w| norm_word(&w.text)).collect();
    let phrase = words.iter().take(4).cloned().collect::<Vec<_>>().join(" ");
    out.push(ProgEntry {
        prog: norm_prog(&seg[idx].text),
        phrase,
        words,
    });
}

fn push_content_entry(w: &Word, seg: &[Word], idx: usize, depth: u8, out: &mut Vec<ProgEntry>) {
    if w.fully_quoted
        && w.text
            .contains(|c: char| c.is_whitespace() || c == ';' || c == '|')
    {
        let inner = tokenize(&w.text);
        for inner_seg in segments(&inner) {
            program_entries(&inner_seg, depth + 1, out);
        }
        return;
    }
    let words: Vec<String> = seg[idx..].iter().map(|w| norm_word(&w.text)).collect();
    let phrase = words.iter().take(4).cloned().collect::<Vec<_>>().join(" ");
    out.push(ProgEntry {
        prog: norm_prog(&w.text),
        phrase,
        words,
    });
}

/// 词元归一（小写；短语匹配用）。
fn norm_word(text: &str) -> String {
    text.trim().to_ascii_lowercase()
}

/// 程序名归一（小写＋剥离脚本/可执行后缀）。
fn norm_prog(text: &str) -> String {
    let lower = norm_word(text);
    for suffix in [".exe", ".com", ".bat", ".cmd", ".ps1"] {
        if let Some(stripped) = lower.strip_suffix(suffix) {
            return stripped.to_owned();
        }
    }
    lower
}

#[cfg(test)]
mod tests {

    // ─── 0cq S2（2026-10-04）写控误拦两族回归钉 ───────────────────────────
    // 来源＝recli 三跑三条真机误拦（181 批 §4b；命令原文经 journal sha256
    // 反查，0cq S1 勘定）。三例同根：`path_candidates` 裸词空白拆片把 echo
    // 散文撕成 `/`、`.gsa/usr)` 伪词元 × 规则 1/5 全局词扫描；另叠加 find
    // 读排除模式值位被当写目标。以下三例修复后**必须保持不拦**（block 即红）。

    /// 真机误拦①：find 读排除模式（sha `7672ecc56ea3` 缩尺）——修复前
    /// Block(carrier-write, "target `/proj/.gsa/*` is inside the `.gsa`
    /// session volume")；修复后 Allow（arm 面收窄：`2>/dev/null` 不武装＋
    /// `-path` 值位豁免）。
    #[test]
    fn fp_real_machine_find_exclusion_pattern_stays_allowed() {
        let r = review_linux(
            "find / -maxdepth 3 -not -path '/proc/*' -not -path '/proj/.gsa/*'              2>/dev/null | head -40",
        );
        assert_ne!(
            rule_of(&r),
            Some("carrier-write"),
            "读排除模式不得作写目标: {r:?}"
        );
        assert_eq!(r, CommandReview::Allow, "{r:?}");
    }

    /// 真机误拦②：echo 散文拆片伪形（sha `9b93a0ea4d6c` 缩尺）——修复前
    /// Block(carrier-write, "target `.gsa/usr)` …")；修复后 Allow（裸词不
    /// 再空白拆片；整词候选非路径不命中）。
    #[test]
    fn fp_real_machine_echo_prose_fragment_stays_allowed() {
        let r = review_linux(
            "echo \"== find test-ish (excluding .gsa/usr) ==\" &&              find / -not -path '*/.gsa*' 2>/dev/null",
        );
        assert_eq!(r, CommandReview::Allow, "{r:?}");
    }

    /// 真机误拦③：echo 散文拆片 `/` 词元（sha `7d0e68698f45` 实际触发形态
    /// ——`"=== csv / json / yaml / count / ids ==="` 拆出裸 `/` × 规则 1
    /// 全局扫描＝误报卷根删除；0cq S1 勘定勘误 181 §4b 的「不存在路径祖先
    /// 链展开」机理描述）。修复后无 block（warn 面 broad-destructive 为
    /// 规则 6 既有留痕、不阻断、不在本族）。
    #[test]
    fn fp_real_machine_echo_slash_fragment_no_catastrophic_block() {
        let r = review_linux(
            "cd /tmp && rm -rf t3demo &&              echo \"=== csv / json / yaml / count / ids ===\"",
        );
        let rule = rule_of(&r);
        assert_ne!(rule, Some("catastrophic-recursive-delete"), "{r:?}");
        assert_ne!(rule, Some("carrier-write"), "{r:?}");
    }

    /// 0cq S2 边界钉：复合命令头部的裸 `/`（`cd /`）不是删除目标——规则 1
    /// 扫描精准化（动词与旗同段武装、仅扫动词位之后）。
    #[test]
    fn compound_head_cd_root_is_not_a_delete_target() {
        let r = review_linux("cd / && rm -rf t3demo");
        assert_ne!(rule_of(&r), Some("catastrophic-recursive-delete"), "{r:?}");
        // 真阳性对照：动词段内的 `/` 仍拦。
        assert!(matches!(review_linux("rm -rf /"), CommandReview::Block(_)));
        assert!(matches!(
            review_linux("cd / && rm -rf /"),
            CommandReview::Block(_)
        ));
    }

    /// 0cq S2 同族臂收窄钉：纯读 `.gsa` 路径＋null 重定向＝Allow（修复前
    /// `>` 无条件武装规则 5 → 读 `.gsa` 日志被拦）。
    #[test]
    fn null_redirect_read_of_gsa_stays_allowed() {
        assert_eq!(
            review_linux("cat /proj/.gsa/logs/session.log 2>/dev/null"),
            CommandReview::Allow
        );
        // 真阳性对照：写 `.gsa` 仍拦（重定向目标位真实文件）。
        assert!(matches!(
            review_linux("find / -type f > /proj/.gsa/out.txt"),
            CommandReview::Block(_)
        ));
    }

    /// 0cq S2 读模式值豁免钉：写段内的 `--exclude=` kv 形态与 `-path` 值位
    /// 不作写目标；真阳性对照＝重定向目标位的 `.gsa` 路径仍拦（0ct 后
    /// grep 操作数位 `.gsa` 为读方向放行——原「操作数仍拦」断言随 0ct
    /// 确认性语义翻转，见 `gsa_redirect_armed_pure_read_program_allows_gsa_operand`）。
    #[test]
    fn read_pattern_values_are_not_write_targets() {
        assert_eq!(
            review_linux("grep -r --exclude=/proj/.gsa/x foo . > /proj/out.txt"),
            CommandReview::Allow
        );
        assert_ne!(
            rule_of(&review_linux(
                "grep -r -path /proj/.gsa/x foo . > /proj/out.txt"
            )),
            Some("carrier-write"),
            "-path 值位豁免"
        );
        assert_eq!(
            review_linux("grep -r foo /proj/.gsa/x > /proj/out.txt"),
            CommandReview::Allow,
            "grep 操作数＝读方向（0ct）"
        );
        assert!(matches!(
            review_linux("grep -r foo > /proj/.gsa/x"),
            CommandReview::Block(_)
        ));
    }

    /// 0cq S2 写段局部化钉：读段（ls/find）的 `.gsa` 词元不再逐个比对——
    /// 同命令的写段之外的读路径不触发规则 5。
    #[test]
    fn read_segment_gsa_tokens_are_not_scanned() {
        assert_eq!(
            review_linux("ls -la /proj/.gsa && rm -rf tmp/old"),
            CommandReview::Allow
        );
        assert_eq!(
            review_linux("touch /proj/a && find / -name '/proj/.gsa/*' 2>/dev/null"),
            CommandReview::Allow
        );
        // 真阳性对照：写段内目标仍拦。
        assert!(matches!(
            review_linux("ls /proj && cp x /proj/.gsa/y"),
            CommandReview::Block(_)
        ));
    }

    /// 186 批审查处置钉：复合命令中无关写动词武装规则 5 后，仅含 nullish
    /// 重定向的读段不再被当写段扫描（修复前 `touch /proj/a && cat
    /// /proj/.gsa/… 2>/dev/null` 仍误拦——cat 段的 `2>/dev/null` 被算写段
    /// 标志）；真阳性对照＝同复合形态下重定向真实写 `.gsa` 仍拦。
    #[test]
    fn compound_nullish_redirect_read_segment_stays_allowed() {
        assert_eq!(
            review_linux("touch /proj/a && cat /proj/.gsa/logs/session.log 2>/dev/null"),
            CommandReview::Allow
        );
        assert!(matches!(
            review_linux("touch /proj/a && cat /proj/.gsa/logs/session.log > /proj/.gsa/out.txt"),
            CommandReview::Block(_)
        ));
    }

    // ─── 0ct（2026-10-07）`.gsa` 读向放行钉 ────────────────────────────────
    // 来源＝0cr D4 meshctl 读向复制被拦（200 批 §2/§3；用户裁「.gsa 这个问题
    // 是读不是写……读肯定是要全部开放的啊，只不改就可以了」＝唯一不变量
    // 「只读不改」）。以下读向形状修复前 Block(carrier-write)、修复后必须
    // 不拦（block 即红）；每组附写向真阳性对照臂（写向保护保留面）。

    /// 真机触发例（0cr D4 meshctl，RUN-1a55a270-6 seq67 缩尺）：`.gsa` 台账
    /// 读向复制——源在 `.gsa`、落点在 /tmp＝读不是写。修复前
    /// Block(carrier-write, "target `/proj/.gsa/…` is inside the `.gsa`
    /// session volume")；修复后 Allow（复制族源位豁免）。
    #[test]
    fn gsa_read_direction_copy_source_stays_allowed() {
        assert_eq!(
            review_linux("cp /proj/.gsa/rollback/3d66aa37/*.bak /tmp/meshctl_prev.py"),
            CommandReview::Allow,
            "读向复制（源 .gsa → 落点 /tmp）不得拦: {:?}",
            review_linux("cp /proj/.gsa/rollback/3d66aa37/*.bak /tmp/meshctl_prev.py")
        );
        // 真阳性对照①：落点位 `.gsa` 仍拦（写向保护保留）。
        assert!(matches!(
            review_linux("cp x /proj/.gsa/y"),
            CommandReview::Block(_)
        ));
        // 真阳性对照②：源与落点双 `.gsa` 仍拦（落点在卷内＝写）。
        assert!(matches!(
            review_linux("cp /proj/.gsa/x /proj/.gsa/y"),
            CommandReview::Block(_)
        ));
    }

    /// 多源复制：目标位＝末个位置词元，其余位置词元皆源位（读向豁免）；
    /// 落点位翻转即拦。
    #[test]
    fn gsa_copy_dest_is_last_positional() {
        assert_eq!(
            review_linux("cp /tmp/a /proj/.gsa/b /tmp/dst"),
            CommandReview::Allow
        );
        assert!(matches!(
            review_linux("cp /tmp/a /tmp/b /proj/.gsa/dst"),
            CommandReview::Block(_)
        ));
    }

    /// xcopy/robocopy 的 `src dst` 语序：目标位＝第 2 位置词元（末位是旗
    /// `/MIR` 不是落点——末位判定会漏拦落点位 `.gsa`）。
    #[test]
    fn gsa_copy_dest_is_second_positional_for_xcopy_robocopy() {
        assert_eq!(
            review_linux("robocopy /proj/.gsa/logs /tmp/out /MIR"),
            CommandReview::Allow
        );
        assert!(matches!(
            review_linux("robocopy /tmp/logs /proj/.gsa/out /MIR"),
            CommandReview::Block(_)
        ));
        assert_eq!(
            review_linux("xcopy /proj/.gsa/src /tmp/dst"),
            CommandReview::Allow
        );
        assert!(matches!(
            review_linux("xcopy /tmp/src /proj/.gsa/dst"),
            CommandReview::Block(_)
        ));
    }

    /// `install` 落复制族：源位豁免、落点位扫描。
    #[test]
    fn gsa_install_source_exempt_dest_scanned() {
        assert_eq!(
            review_linux("install -m 600 /proj/.gsa/key /tmp/copy"),
            CommandReview::Allow
        );
        assert!(matches!(
            review_linux("install -m 600 /tmp/key /proj/.gsa/keystore"),
            CommandReview::Block(_)
        ));
    }

    /// 不入复制豁免表的保守面：`mv` 搬移即源位写（移出 `.gsa` 仍拦）、
    /// `ln` 硬链接目标位＝写别名向量（保守不豁免）、`tee` 多落点同写。
    #[test]
    fn gsa_move_ln_tee_sources_stay_scanned() {
        assert!(matches!(
            review_linux("mv /proj/.gsa/x /tmp/y"),
            CommandReview::Block(_)
        ));
        assert!(matches!(
            review_linux("ln -s /proj/.gsa/x /tmp/link"),
            CommandReview::Block(_)
        ));
        assert!(matches!(
            review_linux("cat /proj/.gsa/x | tee /tmp/a /proj/.gsa/b"),
            CommandReview::Block(_)
        ));
    }

    /// 重定向武装段的纯读程序：除重定向目标位外全部词位＝读方向
    /// （`cat .gsa/x > /tmp/y` 是读不是写）；真阳性对照＝目标位 `.gsa` 仍拦。
    #[test]
    fn gsa_redirect_armed_pure_read_program_allows_gsa_operand() {
        assert_eq!(
            review_linux("cat /proj/.gsa/logs/session.log > /tmp/out.txt"),
            CommandReview::Allow
        );
        assert_eq!(
            review_linux("grep -r foo /proj/.gsa/x > /proj/out.txt"),
            CommandReview::Allow
        );
        assert_eq!(
            review_linux(
                "touch /proj/a && head -50 /proj/.gsa/runs/RUN-1/events.jsonl > /tmp/e.txt"
            ),
            CommandReview::Allow
        );
        // 真阳性对照：重定向目标位 `.gsa` 仍拦（写向保护保留）。
        assert!(matches!(
            review_linux("grep -r foo > /proj/.gsa/out.txt"),
            CommandReview::Block(_)
        ));
        assert!(matches!(
            review_linux("cat /tmp/x > /proj/.gsa/out.txt"),
            CommandReview::Block(_)
        ));
    }

    /// 纯读表排除项（写向旁路面，保守不豁免——段内 `.gsa` 词元照扫）：
    /// `find`（-delete/-exec）、`sort`（-o 落点）、`sed`（-i 原位写）、
    /// `awk`（system() 旁路）、`xargs`（派生执行）。
    #[test]
    fn gsa_operand_pure_read_table_excludes_write_capable_readers() {
        assert!(matches!(
            review_linux("find /proj/.gsa -name '*' -delete > /tmp/log"),
            CommandReview::Block(_)
        ));
        assert!(matches!(
            review_linux("sort /proj/.gsa/x > /tmp/out"),
            CommandReview::Block(_)
        ));
        assert!(matches!(
            review_linux("sed -n 1p /proj/.gsa/x > /tmp/out"),
            CommandReview::Block(_)
        ));
    }

    /// 复制族段与重定向并存：重定向目标位不进位置词元收集（`cp src dst
    /// > log` 的落点位仍是 dst——`log` 误当末位目标会把落点位 `.gsa`
    /// 豁免成源位）。
    #[test]
    fn gsa_copy_dest_detection_ignores_redirect_targets() {
        assert!(matches!(
            review_linux("cp /tmp/a /proj/.gsa/b > /tmp/log"),
            CommandReview::Block(_)
        ));
        assert_eq!(
            review_linux("cp /proj/.gsa/x /tmp/y > /tmp/log"),
            CommandReview::Allow
        );
    }

    /// 209 审查 P1 族 a（212 修复钉）：nullish 重定向目标位（`/dev/null`）
    /// 与 fd 重定向残片（`2>` 分词产出裸数字词元；`2>&1` 的 `&` 为段分隔）
    /// **不问 nullish 一律不作落点竞选**——修复前真落点位 `.gsa` 被当源位
    /// 豁免整命令放行；读向形状（真落点在卷外）必须保持 Allow。
    #[test]
    fn gsa_nullish_redirect_target_and_fd_shards_never_dest_candidates() {
        // 修复前 Allow（旁路）→ 修复后必须 Block。
        assert!(
            matches!(
                review_linux("cp /tmp/a /proj/.gsa/b > /dev/null"),
                CommandReview::Block(_)
            ),
            "nullish redirect target must not become the dest candidate"
        );
        assert!(
            matches!(
                review_linux("cp /tmp/a /proj/.gsa/b >/dev/null 2>&1"),
                CommandReview::Block(_)
            ),
            "fd-dup shard must not become the dest candidate"
        );
        // 读向回归：真落点在卷外仍放行（读全开放不因本修复收窄）。
        assert_eq!(
            review_linux("cp /proj/.gsa/x /tmp/y > /dev/null"),
            CommandReview::Allow
        );
        assert_eq!(
            review_linux("cp /proj/.gsa/x /tmp/y >/dev/null 2>&1"),
            CommandReview::Allow
        );
    }

    /// 209 审查 P1 族 b（212 修复钉）：目标值型旗（`-t`/`--target-directory`
    /// 分离值形）在段＝落点不可辨 → 整段不豁免（保守全扫）——修复前真落点
    /// （旗值位 `.gsa`）被当源位豁免整命令放行。`=` 形不经本表、由主扫描
    /// kv 拆值覆盖（本钉一并锁住）。误伤面（经 `-t` 从 `.gsa` 读向复制被
    /// 拒）为保守方向的既登记代价，钉住备查。
    #[test]
    fn gsa_dest_value_flags_disable_source_exemption() {
        // 修复前 Allow（旁路）→ 修复后必须 Block。
        assert!(
            matches!(
                review_linux("cp -t /proj/.gsa/dir /tmp/src"),
                CommandReview::Block(_)
            ),
            "flag-value dest must be scanned"
        );
        assert!(matches!(
            review_linux("cp --target-directory /proj/.gsa/dir /tmp/src"),
            CommandReview::Block(_)
        ));
        assert!(matches!(
            review_linux("cp --target-directory=/proj/.gsa/dir /tmp/src"),
            CommandReview::Block(_)
        ));
        assert!(matches!(
            review_linux("install -t /proj/.gsa/dir /tmp/src"),
            CommandReview::Block(_)
        ));
        // 误伤面备查（保守方向既登记代价）：`-t` 读向复制也拒。
        assert!(matches!(
            review_linux("cp -t /tmp/dst /proj/.gsa/x"),
            CommandReview::Block(_)
        ));
    }

    use super::*;

    fn roots() -> Vec<PathBuf> {
        write_control::windows_disaster_tree_roots_with(
            Some(r"C:\Windows"),
            Some(r"C:\Program Files"),
            Some(r"C:\Program Files (x86)"),
            Some(r"C:\ProgramData"),
        )
    }

    fn linux_roots() -> Vec<PathBuf> {
        write_control::linux_disaster_tree_roots()
    }

    fn review(cmd: &str) -> CommandReview {
        review_command_with(Path::new(r"D:\proj"), cmd, &host_state_win(), &roots())
    }

    fn review_linux(cmd: &str) -> CommandReview {
        review_command_with(Path::new("/proj"), cmd, &host_state_linux(), &linux_roots())
    }

    /// 0cc v3 规则 5 宿主状态两窄目标（Windows 形态注入）。
    fn host_state_win() -> write_control::HostStateTargets {
        write_control::HostStateTargets {
            keystore_root: Some(PathBuf::from(r"D:\acaf\keystore")),
            signer_manifest: Some(PathBuf::from(r"D:\acaf\signer-manifest.json")),
        }
    }

    /// 0cc v3 规则 5 宿主状态两窄目标（Linux/容器形态注入——`/etc/orz-acaf`
    /// 为容器侧 keystore 约定落点）。
    fn host_state_linux() -> write_control::HostStateTargets {
        write_control::HostStateTargets {
            keystore_root: Some(PathBuf::from("/etc/orz-acaf/keystore")),
            signer_manifest: Some(PathBuf::from("/etc/orz-acaf/signer-manifest.json")),
        }
    }

    fn rule_of(review: &CommandReview) -> Option<&'static str> {
        match review {
            CommandReview::Block(f) | CommandReview::Warn(f) => Some(f.rule),
            CommandReview::Allow => None,
        }
    }

    /// 0cb 防膨胀钉①：deny 形状表＝封闭集恰 5 条 block 规则＋恰 2 条 warn 规则
    /// （新增表项必改本测试；与判官 `WRITE_CONTROL_RULE_CATEGORIES` 同形）。
    #[test]
    fn block_rule_table_is_the_closed_five() {
        assert_eq!(
            BLOCK_RULES,
            [
                "catastrophic-recursive-delete",
                "raw-device-write",
                "boot-firmware-flip",
                "registry-hive-delete",
                "carrier-write",
            ]
        );
        assert_eq!(WARN_RULES, ["broad-destructive", "elevation"]);
    }

    /// 0cb 防膨胀钉①（表级）：各封闭触发表行数钉死——加行必须改测试＋设计档。
    /// 0cb 审查处理批（2026-09-29）：钉覆盖补全至全部封闭表（warn 面与
    /// 词法结构表同钉——`DESTRUCTIVE_VERBS` 承载规则 5 block 面）。
    #[test]
    fn closed_trigger_tables_hold_their_registered_sizes() {
        assert_eq!(CATASTROPHIC_DELETE_VERBS.len(), 7);
        assert_eq!(CATASTROPHIC_RECURSIVE_FLAGS.len(), 6);
        assert_eq!(RAW_DEVICE_PROGRAMS.len(), 2);
        assert_eq!(RAW_DEVICE_PHRASES.len(), 2);
        assert_eq!(BLOCK_DEVICE_PREFIXES.len(), 5);
        assert_eq!(FLIP_PROGRAMS.len(), 7);
        assert_eq!(FLIP_PHRASES.len(), 17);
        assert_eq!(REG_MODIFY_SUBCOMMANDS.len(), 3);
        assert_eq!(REG_PROTECTED_HIVES.len(), 6);
        assert_eq!(DESTRUCTIVE_VERBS.len(), 39);
        assert_eq!(DELETE_VERBS.len(), 8);
        assert_eq!(ANCESTOR_SWEEP_VERBS.len(), 15);
        assert_eq!(ELEVATION_PROGRAMS.len(), 4);
        assert_eq!(WRAPPER_WORDS.len(), 32);
        assert_eq!(CONTENT_WORDS.len(), 5);
        // 0cq S2 读模式值豁免闭集（186 批审查处置补钉——185 批声明闭集但
        // 漏入本防膨胀钉）。
        assert_eq!(READ_PATTERN_OPTIONS.len(), 15);
        assert_eq!(READ_PATTERN_KV_PREFIXES.len(), 6);
        // 0ct 读方向词位豁免闭集（2026-10-07）。
        assert_eq!(COPY_SOURCE_EXEMPT_VERBS.len(), 7);
        assert_eq!(COPY_DEST_SECOND_VERBS.len(), 2);
        assert_eq!(OPERAND_PURE_READ_PROGRAMS.len(), 24);
        // 212 修复（209 审查 P1 族 b）目标值型旗闭集。
        assert_eq!(COPY_DEST_VALUE_FLAGS.len(), 4);
        // 0ct 保守边界：搬移/链接/多落点不入复制豁免表；写旁路读者不入
        // 纯读表；复制族 ⊆ 写动词表（本就是写段才扫描）；纯读表与写动词
        // 表互斥（相交会令写动词段整体豁免——写向不变量翻穿）。
        for verb in ["mv", "move", "move-item", "mi", "ln", "tee"] {
            assert!(
                !COPY_SOURCE_EXEMPT_VERBS.contains(&verb),
                "move/link/multi-dest verbs must stay fully scanned: {verb}"
            );
        }
        for prog in ["find", "sort", "sed", "awk", "xargs", "tee"] {
            assert!(
                !OPERAND_PURE_READ_PROGRAMS.contains(&prog),
                "write-capable readers must stay fully scanned: {prog}"
            );
        }
        for verb in COPY_SOURCE_EXEMPT_VERBS {
            assert!(
                DESTRUCTIVE_VERBS.contains(verb),
                "copy family must stay inside the write-verb table: {verb}"
            );
        }
        for prog in OPERAND_PURE_READ_PROGRAMS {
            assert!(
                !DESTRUCTIVE_VERBS.contains(prog),
                "pure-read programs must not be write verbs: {prog}"
            );
        }
        // 0cc v3.1 祖先链臂（P2）：扫荡表＝DELETE_VERBS 全体＋搬移族，且整体
        // ⊆ DESTRUCTIVE_VERBS；`install`/`ln`（P3-1）为写动词但非扫荡动词
        // （入位写不毁祖先）。
        for verb in DELETE_VERBS {
            assert!(
                ANCESTOR_SWEEP_VERBS.contains(verb),
                "sweep set must contain every delete verb: {verb}"
            );
        }
        for verb in ANCESTOR_SWEEP_VERBS {
            assert!(
                DESTRUCTIVE_VERBS.contains(verb),
                "sweep set must stay a subset of the write-verb table: {verb}"
            );
        }
        for positioned in ["install", "ln", "cp", "mkdir"] {
            assert!(
                DESTRUCTIVE_VERBS.contains(&positioned)
                    && !ANCESTOR_SWEEP_VERBS.contains(&positioned),
                "into-place write verbs must not be sweep verbs: {positioned}"
            );
        }
    }

    /// 0ch v4 L3/L1-L2 粒度一致性钉：设备安全节点表每一项都不得落进规则 2
    /// 块设备目标形态（`is_block_device_target`）——L3 内核授权面不得比
    /// L1/L2 block 面更宽；`/dev/null` 豁免锚两层面同在（设计档 §7.1）。
    #[test]
    fn device_safe_nodes_never_overlap_block_device_faces() {
        assert_eq!(write_control::DEVICE_SAFE_NODES.len(), 7);
        for node in write_control::DEVICE_SAFE_NODES {
            assert!(
                !is_block_device_target(node),
                "L3 safe node {node} must not be an L1/L2 block-device face"
            );
        }
        // 豁免锚：/dev/null 双面同族（L1/L2 NULL_DEVICE 豁免 ↔ L3 文件级授权）。
        assert!(write_control::DEVICE_SAFE_NODES.contains(&NULL_DEVICE));
    }

    /// 0cb③：`CommandReview::report()` 形状——allow 恒 null 臂、warn/block
    /// 恒 rule+detail 臂（schema XOR 语义的生产侧镜像）。
    #[test]
    fn review_report_shapes_match_schema_xor() {
        assert_eq!(
            review("cargo build").report(),
            CommandReviewReport {
                review: "allow".to_string(),
                rule: None,
                detail: None,
            }
        );
        let warn = review("sudo echo hi");
        let report = warn.report();
        assert_eq!(report.review, "warn");
        assert!(report.rule.is_some());
        assert!(report.detail.is_some());
    }

    #[test]
    fn dev_routine_commands_stay_allowed() {
        for cmd in [
            "cargo test -p orz-tools --lib",
            "git status --short",
            "rg --format json x src",
            "python scripts/check_repository.py",
            "echo hello",
            // 读系统文件：无写动词 → 放行（只读面不受影响）。
            "Get-Content C:\\Windows\\win.ini",
            "type C:\\Windows\\win.ini",
            "grep -n foo C:\\Windows\\System32\\drivers\\etc\\hosts",
            // 工作区内清理与复制：放行。
            "Remove-Item .tmp-0cb-check -Recurse -Force",
            "copy src\\a.rs src\\b.rs",
            // 命中短语但只是检索参数（程序位不是翻转类）→ 不误伤。
            "rg \"set-mppreference\" docs",
        ] {
            assert_eq!(review(cmd), CommandReview::Allow, "must allow: {cmd}");
        }
    }

    /// 0cb 防膨胀钉②：**正向放行集 fixture**（设计 §3——必须保持放行，
    /// 一行回归即红；本集正是 TB 2.1 轮 215 条误拦的高频形状）。
    #[test]
    fn positive_allow_fixture_must_stay_allowed() {
        for cmd in [
            "make install",
            "apt-get install build-essential",
            "pip install requests",
            "echo x > /dev/null",
            "cmake --build . 2>/dev/null",
            "povray +Iin.pov +Oout.png +O/dev/null",
            "Remove-Item C:\\Windows\\SoftwareDistribution\\Download\\old",
            "cat /proc/cpuinfo",
            "ln -s /etc /w",
            "dd if=x of=/dev/null",
        ] {
            let r = review(cmd);
            assert_eq!(r, CommandReview::Allow, "must allow: {cmd} -> {r:?}");
        }
    }

    /// 0cb 防膨胀钉③（规则 1 负向集）：根级递归删除命中。
    #[test]
    fn catastrophic_recursive_delete_blocks() {
        for cmd in [
            "rm -rf /",
            "rd /s C:\\",
            "Remove-Item C:\\Windows -Recurse",
            "Remove-Item -Recurse -Force C:\\ProgramData",
            "rmdir /s /q \"C:\\Program Files\"",
            "del /s /q C:\\ProgramData",
            "rm -rf ..",
            // `ri`＝Remove-Item 的 PowerShell 别名（0cb 审查处理批补齐——
            // 否则同形灾难只落 warn）。
            "ri C:\\Windows -Recurse",
        ] {
            let r = review(cmd);
            assert!(
                matches!(r, CommandReview::Block(_)),
                "must block: {cmd} -> {r:?}"
            );
            assert_eq!(
                rule_of(&r),
                Some("catastrophic-recursive-delete"),
                "rule for {cmd}"
            );
        }
        // Linux 载体（评测容器）同形：POSIX 根与根本树根（`/usr` 属 Linux
        // 根集——宿平台根集按平台分叉，Windows 根集不含 POSIX 树）。
        for cmd in ["rm -rf /", "rm -rf /etc", "rm -rf /usr", "rm -rf /var"] {
            let r = review_linux(cmd);
            assert!(matches!(r, CommandReview::Block(_)), "must block: {cmd}");
            assert_eq!(rule_of(&r), Some("catastrophic-recursive-delete"));
        }
        // 子目录级精准删除放行（「需精准删除」硬边界；无路径前缀宽扫）。
        for cmd in [
            "rm -rf /usr/local/build-cache",
            "Remove-Item C:\\Windows\\Temp -Recurse -Force",
        ] {
            let r = review(cmd);
            assert!(
                !matches!(r, CommandReview::Block(_)),
                "subtree delete must not block: {cmd} -> {r:?}"
            );
        }
    }

    /// 0cb 防膨胀钉③（规则 2 负向集）：raw 设备/卷毁写命中。
    #[test]
    fn raw_device_write_blocks() {
        for cmd in [
            "dd if=iso.img of=/dev/sda",
            "dd if=x of=\\\\.\\PhysicalDrive2",
            "mkfs.ext4 /dev/sdb1",
            "mkfs /dev/vdb",
            "format D: /y",
            "diskpart /s script.txt",
            "vssadmin delete shadows /all",
            "wbadmin delete catalog",
        ] {
            let r = review(cmd);
            assert!(
                matches!(r, CommandReview::Block(_)),
                "must block: {cmd} -> {r:?}"
            );
            assert_eq!(rule_of(&r), Some("raw-device-write"), "rule for {cmd}");
        }
    }

    /// 0cb 审查处理批（收窄后负测补口）：raw 设备**读侧**与镜像文件目标放行
    /// ——`of=` 才是写侧目标位（`dd if=\\.\PhysicalDrive0` 备份/取证读）、
    /// `mkfs*` 镜像文件构建、查询类 PhysicalDrive 提及（位置无关词元扫
    /// 已退役）。
    #[test]
    fn raw_device_read_side_and_image_targets_stay_allowed() {
        for cmd in [
            "dd if=\\\\.\\PhysicalDrive0 of=backup.img",
            "mkfs.ext4 /tmp/disk.img",
            "mkfs.ext4 disk.img",
            "mkfs -t ext4 /var/tmp/img.raw",
            "Get-PhysicalDisk -DeviceName PhysicalDrive0",
        ] {
            let r = review(cmd);
            assert_eq!(r, CommandReview::Allow, "must allow: {cmd} -> {r:?}");
        }
    }

    /// 0cb 审查处理批（绕过面闭合）：Windows 盘符相对根形态
    /// （`\Windows` ≡ `%SystemDrive%\Windows`）进规则 1 比对——此前与
    /// `C:\Windows` 字面比对必败、只落 warn。
    #[cfg(windows)]
    #[test]
    fn drive_relative_rooted_forms_are_qualified_against_system_drive() {
        for cmd in [
            "rm -rf \\Windows",
            "del /s /q \\ProgramData",
            "rd /s \\Windows",
        ] {
            let r = review(cmd);
            assert!(
                matches!(r, CommandReview::Block(_)),
                "must block: {cmd} -> {r:?}"
            );
            assert_eq!(
                rule_of(&r),
                Some("catastrophic-recursive-delete"),
                "rule for {cmd}"
            );
        }
        // 子目录不拦（精准删除放行）。
        let r = review("rm -rf \\Windows\\Temp");
        assert!(
            !matches!(r, CommandReview::Block(_)),
            "subtree delete must not block: {r:?}"
        );
    }

    /// 0cb 防膨胀钉③（规则 3 负向集）：引导固件与安全机制翻转命中。
    #[test]
    fn boot_firmware_flip_blocks() {
        for cmd in [
            "Set-MpPreference -EnableControlledFolderAccess Disabled",
            "powershell -Command \"Set-MpPreference -EnableControlledFolderAccess AuditMode\"",
            "Add-MpPreference -ControlledFolderAccessProtectedFolders \"D:\\x\"",
            "netsh advfirewall set allprofiles state off",
            "wevtutil cl Security",
            "Clear-EventLog -LogName System",
            "bcdedit /set testsigning on",
            "Set-ExecutionPolicy Bypass -Scope Process",
            "sc stop WinDefend",
            "sc config mpssvc start= disabled",
            "net stop mpssvc",
            "Stop-Service -Name WinDefend",
            "fltmc unload X",
        ] {
            let r = review(cmd);
            assert!(
                matches!(r, CommandReview::Block(_)),
                "must block: {cmd} -> {r:?}"
            );
            assert_eq!(rule_of(&r), Some("boot-firmware-flip"), "rule for {cmd}");
        }
    }

    /// 0cb 防膨胀钉③（规则 4 负向集）：注册表蜂巢修改命中。
    #[test]
    fn registry_hive_delete_blocks() {
        for cmd in [
            "reg delete HKLM\\Software\\Foo /f",
            "reg add HKLM\\SYSTEM\\CurrentControlSet /v x /d y",
            "reg add HKCR\\.0cb /v a /d b",
            "reg import hku\\probe.erb",
            "reg delete HKEY_LOCAL_MACHINE\\SOFTWARE\\X /f",
        ] {
            let r = review(cmd);
            assert!(
                matches!(r, CommandReview::Block(_)),
                "must block: {cmd} -> {r:?}"
            );
            assert_eq!(rule_of(&r), Some("registry-hive-delete"), "rule for {cmd}");
        }
        // 用户蜂巢与非修改子命令不拦；数据值中的蜂巢提及不再误拦
        // （目标位判定，0cb 审查处理批收窄）。
        assert_eq!(review("reg delete HKCU\\Env /v x"), CommandReview::Allow);
        assert_eq!(
            review("reg add HKCU\\Env /v x /d hklm-note"),
            CommandReview::Allow
        );
        assert_eq!(
            review("reg query HKLM\\SOFTWARE\\Microsoft"),
            CommandReview::Allow
        );
        assert_eq!(
            review("reg export HKLM\\Software out.reg"),
            CommandReview::Allow
        );
    }

    /// 0cb 防膨胀钉③（规则 5 负向集；0cc v3 收窄后形态）：宿主状态两窄目标
    /// 命中（恒拒面保持）——`.gsa` 会话卷＋ACAF keystore 根＋signer manifest。
    #[test]
    fn carrier_write_still_blocks() {
        for (cmd, kind) in [
            ("Set-Content .gsa\\journal\\x.txt hi", "session volume"),
            ("echo x > .gsa\\runs\\y", "session volume"),
            ("rm -rf D:\\proj\\.gsa\\*", "session volume"),
            (
                "Set-Content D:\\acaf\\keystore\\probe.txt hi",
                "keystore root",
            ),
            (
                "rm -rf D:\\acaf\\keystore\\installation-key.json",
                "keystore root",
            ),
            ("del D:\\acaf\\signer-manifest.json", "signer manifest"),
        ] {
            let r = review(cmd);
            assert!(
                matches!(r, CommandReview::Block(_)),
                "must block: {cmd} -> {r:?}"
            );
            assert_eq!(rule_of(&r), Some("carrier-write"), "rule for {cmd}");
            assert!(
                match &r {
                    CommandReview::Block(f) => f.detail.contains(kind),
                    _ => false,
                },
                "detail should name the host-state face ({kind}): {r:?}"
            );
        }
        // Linux/容器形态：keystore 与 manifest 同拦（keystore 在 /etc 子树，
        // 规则 1 只拦恰为本体——此处落规则 5）。
        for (cmd, kind) in [
            ("cp key.bin /etc/orz-acaf/keystore/key.bin", "keystore root"),
            (
                "rm -f /etc/orz-acaf/signer-manifest.json",
                "signer manifest",
            ),
        ] {
            let r = review_linux(cmd);
            assert!(
                matches!(r, CommandReview::Block(_)),
                "must block (linux): {cmd} -> {r:?}"
            );
            assert_eq!(rule_of(&r), Some("carrier-write"), "rule for {cmd}");
            assert!(
                match &r {
                    CommandReview::Block(f) => f.detail.contains(kind),
                    _ => false,
                },
                "detail should name the host-state face ({kind}): {r:?}"
            );
        }
    }

    /// 0cc v3 防膨胀钉②增补（退役面放行集——设计 §3 ②「0cc 验收放行例」）：
    /// 载体安装目录／三件套／`grok-home`／`_bgprobe`／软链指入安装目录全部
    /// 放行（build-pov-ray 结构性 0 的修复面；一行回归即红）。
    #[test]
    fn carrier_install_face_writes_retired_to_allow() {
        // Windows 形态：安装目录／三件套／grok-home／_bgprobe（v2 负向集轉正）。
        for cmd in [
            "Remove-Item D:\\app\\orz\\orz.exe",
            "Remove-Item D:\\app\\orz\\orz-signer.exe",
            "Remove-Item D:\\app\\orz\\grok-home\\creds.json",
            "del D:\\app\\orz\\_bgprobe.exe",
            "Set-Content D:\\app\\orz\\notes.txt hi",
        ] {
            let r = review(cmd);
            assert_eq!(
                r,
                CommandReview::Allow,
                "retired carrier face must allow: {cmd} -> {r:?}"
            );
        }
        // Linux 形态：/usr/local/bin 安装（build-pov-ray 题面形状）＋软链指入
        // 安装目录（0.8.5 重跑中 `/tmp/lnk` 软链绕道识破的合法化面）。
        for cmd in [
            "cp build/povray /usr/local/bin/povray",
            "install -m 755 build/povray /usr/local/bin/povray",
            "ln -s /app/orz/orz /usr/local/bin/orz",
            "mv povray /usr/local/bin/povray",
        ] {
            let r = review_linux(cmd);
            assert_eq!(
                r,
                CommandReview::Allow,
                "carrier install face must allow: {cmd} -> {r:?}"
            );
        }
    }

    /// 0cc v3.1 审查处理批（P2 祖先链臂＋P3-1 动词补齐）：扫荡动词（删除/搬移）
    /// 下目标为宿主态受护目标的祖先同落 `carrier-write` block——载体面退役不得
    /// 连带放行 keystore／manifest 的扫荡摧毁；`install`/`ln` 向宿主态目标
    /// 落盘/建链接直接命中；入位写（cp/mkdir/install 落非宿主态位）仍放行。
    #[test]
    fn host_state_ancestor_sweep_blocks_and_into_place_writes_stay_allowed() {
        // Windows 祖先扫荡：keystore 根 `D:\acaf\keystore` 的父目录与搬移。
        for (cmd, kind) in [
            ("rm -rf D:\\acaf", "ancestor of the ACAF keystore root"),
            ("rd /s /q D:\\acaf", "ancestor of the ACAF keystore root"),
            (
                "Remove-Item D:\\acaf -Recurse",
                "ancestor of the ACAF keystore root",
            ),
            (
                "mv D:\\acaf D:\\trash",
                "ancestor of the ACAF keystore root",
            ),
            (
                "ren D:\\acaf acaf-old",
                "ancestor of the ACAF keystore root",
            ),
            ("del D:\\acaf\\signer-manifest.json", "signer manifest"),
        ] {
            let r = review(cmd);
            assert!(
                matches!(r, CommandReview::Block(_)),
                "ancestor sweep must block: {cmd} -> {r:?}"
            );
            assert_eq!(rule_of(&r), Some("carrier-write"), "rule for {cmd}");
            assert!(
                match &r {
                    CommandReview::Block(f) => f.detail.contains(kind),
                    _ => false,
                },
                "detail should name the ancestor face ({kind}): {r:?}"
            );
        }
        // Linux/容器形态：`/etc/orz-acaf`＝keystore 根 `/etc/orz-acaf/keystore`
        // 的父目录；`rename` 非删除动词（规则 1 不接）故落本臂——搬移族同样
        // 门控。
        for cmd in [
            "rm -rf /etc/orz-acaf",
            "mv /etc/orz-acaf /tmp/old-acaf",
            "rename /etc /old-etc",
        ] {
            let r = review_linux(cmd);
            assert!(
                matches!(r, CommandReview::Block(_)),
                "ancestor sweep must block (linux): {cmd} -> {r:?}"
            );
            assert_eq!(rule_of(&r), Some("carrier-write"), "rule for {cmd}");
            assert!(
                match &r {
                    CommandReview::Block(f) => {
                        f.detail.contains("ancestor of the ACAF keystore root")
                    }
                    _ => false,
                },
                "detail should name the ancestor face: {r:?}"
            );
        }
        // P3-1：`install`/`ln` 补入写动词表后向宿主态目标直接命中。
        for (cmd, kind) in [
            (
                "install -m 600 key.bin D:\\acaf\\keystore\\installation-key.json",
                "inside the ACAF keystore root",
            ),
            (
                "ln -sf /tmp/fake.json D:\\acaf\\signer-manifest.json",
                "signer manifest",
            ),
            (
                "install -m 600 x /etc/orz-acaf/keystore/key.bin",
                "inside the ACAF keystore root",
            ),
        ] {
            let r = if cmd.contains("/etc/") {
                review_linux(cmd)
            } else {
                review(cmd)
            };
            assert!(
                matches!(r, CommandReview::Block(_)),
                "install/ln into host state must block: {cmd} -> {r:?}"
            );
            assert_eq!(rule_of(&r), Some("carrier-write"), "rule for {cmd}");
            assert!(
                match &r {
                    CommandReview::Block(f) => f.detail.contains(kind),
                    _ => false,
                },
                "detail should name the host-state face ({kind}): {r:?}"
            );
        }
        // 入位写与规则序回归：祖先臂不外溢——纯入位写仍纯放行；非祖先的
        // 递归删除不 **block**（至多 broad-destructive 留痕，含 `/usr/local`＝
        // build-pov-ray 面）；卷根递归删除仍由规则 1 先行接住（报
        // catastrophic-recursive-delete，不落祖先臂）。
        for cmd in [
            "cp backup D:\\acaf\\backup-store",
            "mkdir D:\\acaf\\newdir",
            "install -m 644 app.conf D:\\acaf\\app.conf",
        ] {
            let r = review(cmd);
            assert_eq!(
                r,
                CommandReview::Allow,
                "into-place write near (not on) host state must allow: {cmd} -> {r:?}"
            );
        }
        let r = review_linux("ln -s /etc /w");
        assert_eq!(
            r,
            CommandReview::Allow,
            "symlink elsewhere must allow: {r:?}"
        );
        for cmd in ["rm -rf D:\\app\\other", "rm -rf /usr/local"] {
            let r = if cmd.contains("/usr/") {
                review_linux(cmd)
            } else {
                review(cmd)
            };
            assert!(
                !matches!(r, CommandReview::Block(_)),
                "non-ancestor sweep must not block (warn-at-most): {cmd} -> {r:?}"
            );
        }
        let volume_root = review("rm -rf D:\\");
        assert_eq!(
            rule_of(&volume_root),
            Some("catastrophic-recursive-delete"),
            "volume-root recursive delete stays on rule 1: {volume_root:?}"
        );
    }

    /// 0cb §2 退役面一行回归：v1 会拦的系统树一般写动作（整树位置锁／重定向
    /// 扫系统树／`/dev·/proc·/sys` 前缀词元扫）现不再 **block**——交回审批
    /// 组件（递归删除类的 warn 留痕臂按设计保留，非阻断）。
    #[test]
    fn system_tree_general_writes_retired_to_allow() {
        for cmd in [
            "del \"C:\\Windows\\Temp\\x.txt\"",
            "cmd /c del \"C:\\Program Files\\App\\x\"",
            "Set-Content C:\\ProgramData\\app\\cfg.json hi",
            "echo x > C:\\Windows\\Temp\\y",
            "mkdir C:\\Program Files\\MyApp",
            "Set-Content /etc/hosts config",
            "echo note > /proc/self/notes",
            "icacls C:\\Windows\\Temp\\x /grant Users:F",
        ] {
            let r = review(cmd);
            assert_eq!(
                r,
                CommandReview::Allow,
                "retired deny must allow: {cmd} -> {r:?}"
            );
        }
        // 递归删除系统树子目录：不再 block，仅 warn 留痕（broad-destructive）。
        let r = review("Remove-Item -Recurse -Force C:\\Windows\\System32\\drivers");
        assert!(
            matches!(r, CommandReview::Warn(_)),
            "retired deny must not block (warn trace stays): {r:?}"
        );
    }

    #[test]
    fn warns_fire_for_broad_and_elevation_shapes() {
        for cmd in [
            "rm -rf D:\\other\\x",
            "del /s /q *",
            "sudo apt-get install foo",
            "Start-Process cmd -Verb RunAs",
        ] {
            let r = review(cmd);
            assert!(
                matches!(r, CommandReview::Warn(_)),
                "must warn: {cmd} -> {r:?}"
            );
        }
        assert_eq!(
            rule_of(&review("rm -rf D:\\other\\x")),
            Some("broad-destructive")
        );
        assert_eq!(
            rule_of(&review("sudo apt-get install foo")),
            Some("elevation")
        );
    }

    #[test]
    fn block_message_and_warn_line_carry_rule_identity() {
        let CommandReview::Block(f) = review_linux("rm -rf /usr") else {
            panic!("expected block");
        };
        assert_eq!(f.rule, "catastrophic-recursive-delete");
        let msg = block_message(&f);
        assert!(msg.contains("catastrophic-recursive-delete"));
        assert!(msg.contains("blocked by the mechanical write control"));
        assert!(msg.contains("已越过保底硬边界"));
        assert!(
            msg.contains("精准删除"),
            "rule 1 must carry the precise-delete guidance"
        );
        assert!(
            !msg.contains("备份/暂存"),
            "session-volume suggestion is carrier-write-only: {msg}"
        );

        let CommandReview::Block(f) = review("Set-MpPreference -DisableRealtimeMonitoring 1")
        else {
            panic!("expected block");
        };
        let msg = block_message(&f);
        assert!(msg.contains("boot-firmware-flip"));
        assert!(!msg.contains("精准删除"), "guidance is rule-1 only");

        let CommandReview::Warn(f) = review("rm -rf D:\\other") else {
            panic!("expected warn");
        };
        let line = warn_line(&f);
        assert!(line.starts_with("[写入管控·提示]"));
        assert!(line.contains("broad-destructive"));
    }

    /// 214（0ct 教育面；212 考虑项 i）：会话卷拦截信封携带**备份/暂存指引**
    /// （0cs 同构＝错误信封内给可执行替代物）。仅会话卷臂携带；
    /// keystore 臂与读向放行不携带；「未执行」语义在前＝不软化拦截。
    #[test]
    fn block_message_session_volume_carries_backup_suggestion() {
        let CommandReview::Block(f) = review_linux("cp /tmp/a /proj/.gsa/b") else {
            panic!("expected block");
        };
        assert_eq!(f.rule, "carrier-write");
        let msg = block_message(&f);
        assert!(
            msg.contains("若意图是备份/暂存"),
            "session-volume block must carry the backup suggestion: {msg}"
        );
        assert!(msg.contains("/tmp") && msg.contains("blackboard_write"));
        assert!(
            msg.contains("只读使用"),
            "rollback semantics must be taught: {msg}"
        );
        // 不软化：未执行语义仍在，且建议在其后。
        let hard = msg.find("已越过保底硬边界——命令未执行").unwrap();
        let sug = msg.find("若意图是备份/暂存").unwrap();
        assert!(hard < sug, "suggestion must not soften the block");
        // 恰一处（0cl 纪律）。
        assert_eq!(msg.matches("若意图是备份/暂存").count(), 1);

        // keystore/manifest 臂（秘密与载体保护）不带备份指引——判别式为
        // detail 的 "session volume" 措辞，直接构造非会话卷 finding 验证
        // （生产中 `.gsa/keystore` 由会话卷臂双覆盖先行，属会话卷拦截口径）。
        let keystore_finding = CommandFinding {
            rule: "carrier-write",
            detail: "target `/x` is inside the ACAF keystore root (`/etc/orz-acaf/keystore`)"
                .to_string(),
        };
        let msg2 = block_message(&keystore_finding);
        assert!(
            !msg2.contains("若意图是备份/暂存"),
            "keystore block must not carry the backup suggestion: {msg2}"
        );

        // 读向放行不受影响（对照回归）。
        assert_eq!(review_linux("cp /proj/.gsa/x /tmp/y"), CommandReview::Allow);
    }
}
