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
    let delete_verb_present = entries
        .iter()
        .any(|e| CATASTROPHIC_DELETE_VERBS.contains(&e.prog.as_str()));
    if delete_verb_present
        && words
            .iter()
            .any(|w| CATASTROPHIC_RECURSIVE_FLAGS.contains(&norm_word(&w.text).as_str()))
    {
        for w in &words {
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
    let has_write_verb = entries
        .iter()
        .any(|e| DESTRUCTIVE_VERBS.contains(&e.prog.as_str()) || e.prog == "dd")
        || words.iter().any(|w| w.text == ">" || w.text == ">>");
    let has_sweep_verb = entries
        .iter()
        .any(|e| ANCESTOR_SWEEP_VERBS.contains(&e.prog.as_str()));
    if has_write_verb || has_sweep_verb {
        for w in &words {
            for raw in path_candidates(&w.text) {
                if let Some(detail) = carrier_target_detail(cwd, &raw, host_state, has_sweep_verb) {
                    return CommandReview::Block(CommandFinding {
                        rule: "carrier-write",
                        detail,
                    });
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
pub fn block_message(finding: &CommandFinding) -> String {
    let guidance = if finding.rule == "catastrophic-recursive-delete" {
        "该目标是卷根/根本性树根本体；如需删除，请改为对具体文件或子目录的精准删除。"
    } else {
        ""
    };
    format!(
        "Error: command blocked by the mechanical write control backstop (rule: {rule}). \
         {detail}. 已越过保底硬边界——命令未执行。{guidance}本闸为封闭枚举灾难保底\
         （{n} 条 block 规则），非沙箱。",
        rule = finding.rule,
        detail = finding.detail,
        guidance = guidance,
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

/// 从词元提取路径候选（`of=…`／`-path=…` 取等号右值；引号剥除；含空白词拆片兜底）。
fn path_candidates(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let consider = |out: &mut Vec<String>, value: &str| {
        push_candidate(out, value);
        if value.contains(char::is_whitespace) {
            for piece in value.split_whitespace() {
                push_candidate(out, piece);
            }
        }
    };
    if let Some((key, value)) = text.split_once('=') {
        if key.is_empty() || key.starts_with('-') || key.eq_ignore_ascii_case("of") {
            consider(&mut out, value);
            return out;
        }
    }
    consider(&mut out, text);
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
}
