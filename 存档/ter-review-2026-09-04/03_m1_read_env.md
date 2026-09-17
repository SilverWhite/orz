# TER M1 阅读/环境面切片审查报告（03：T1.10–T1.13）

> 审查人：根代理（承接 a3 切片；a3 代理因消息/并发异常中断未产出，
> 由根代理以提交态静态复核补全）。日期：2026-09-04。基线：orz
> 43d7b9d3（T1.10）/ e0eaa251（T1.11）/ ea444869（T1.12）/ b18fabf5
> （T1.13 Linux 门修复），HEAD 901cdcc3；主仓库 runtime/ 与 assurance/
> 提交态。只读静态核对，未复跑 cargo/pytest。

## 1. 总评

T1.10–T1.12 的实现与审计声明吻合：read_file coarse gate 默认/上限 16/32K
→ 64K（下限 8K 逃生阀）并在工具描述/TOML clamp/测试档位三处同步；
output_object 检索 API（pattern/行区间/尾部）语义干净且有 4 个单测，
host 映射 + journal 三字段 + console 透传齐全；env_snapshot（PATH 存在性 +
并发 --version 1s 超时 + 输入在场 + 无 allowlist/任务结论）与 env 分区
白名单渲染/越权守卫齐备。T1.13 Linux 门修复（b18fabf5）的 2 处编译问题
修复方式正确。

发现 **P3 × 3**、**NOT-VERIFIED/待实机 × 3**；无 P0/P1 功能缺陷
（跨切片的 idle-kill 生产者缺口见 02 M1L-1，与 T1.13 审计 §1.3 表述
相关）。

## 2. 发现清单

### M1R-1（P3；W-F13a 边界）——64KB≈≤16K token 估算只对 ASCII 成立；
多字节内容（CJK/emoji）可逼近或超过读档 token 上限

- 位置：read_file/mod.rs 常量注释（“64 KiB ASCII ≈ ≤16K token < 25K
  读档/50K 单轮预算”）；限制链断言无多字节用例。
- 影响：64KB UTF-8 CJK ≈ 21K 字符 → 依 tokenizer 1–1.5 token/字可达
  21–32K token，可能越过 25K 读档档；单轮 50K 仍有余量但会挤压其它
  上下文。建议补多字节压测或按“最坏 3 byte/char 估算”复核档位。

### M1R-2（P3；W-F13b 健壮性）——`search_output_object` 返回整行文本，
无单行长度上限；超长单行（minified/JSON/base64 dump）命中会整体回传

- 位置：output_object.rs `search_output_object`（text: line 原样入 hits）。
- 影响：检索对象可 MB 级；若模型直接经该 API 消费命中行，单条命中可能
  达到整行 MB 级，撑爆注入预算。read_file/grep 直读路径已有行档保护，
  但 API 本身缺钳制。建议对命中行做长度钳制/截断标记，或在调用侧注明
  上限语义。

### M1R-3（P3；W-F13a 验收口径）——“vm.js 级文件 ≤2 次读完”实际还受
1000 行档约束（64KB 单行文件仍回信封）

- 位置：read_file 行档 MAX_LINES_READ=1000 保留（设计“行数 limit 语义
  保留”）；验收行“vm.js 级文件 ≤2 次读完”未注明行前提。
- 建议：验收口径补“≤64KB 且 ≤1000 行；否则结构化分段轮数按
  offset/limit 计”。

### M1R-4（NOT-VERIFIED；T1.12 墙内行为）——env_snapshot 的 1s 并发版本
探测在 Windows no-AC/LOW IL 沙箱内是否全部成功、整快照是否仍 ≤5s，未在
实机验证（T1.12 实测 ≈1.6s 为本机/开发环境数据）

- 建议：T2.4 实机 DryRun/enforcement-probe 加入一次
  `blackboard_read section=env` 墙内计时断言。

### M1R-5（NOT-VERIFIED；T1.13 门数字）——709/201/273 与 clippy/fmt
净的计数无法在本环境复跑（工作树被 0l 改动污染；纪律禁 cargo/pytest）

- 静态证据：声称的新测试名均可在提交态定位（output_object 4、read_file
  64K 用例、env 3+2、processes 4+2 等）；无与声称明显矛盾之处。门审计
  的“tool_running idle_killed 由单测锁定”除外（见 02 M1L-1）。

### M1R-6（记录）——Linux 门为 glibc release（非 musl 静态），musl +
smoke 复验登记为发布流水线后续项（T1.13 审计 §3），与 M3 计划一致；
不构成缺口。

## 3. 逐项符合性表

| 验收项（TODO2） | 判定 | 证据 |
|---|---|---|
| T1.10 64KB 档（默认=上限 64K、下限 8K、>64K 信封） | PASS | read_file/mod.rs 常量 + env/TOML clamp 8–64K + 工具描述同步；测试 default_64k…60k 全量/76K 信封 |
| T1.10 行档 1000 保留、结构化分段仍成立 | PASS | MAX_LINES_READ 未改；信封 preview≤4K + offset 路径测试保留 |
| T1.11 output_object API（pattern/行区间/尾部/缺失） | PASS | output_object.rs 4 测试；1-based 闭区间 + clamp；统一解码链 read_lines |
| T1.11 host 映射 + journal 三字段 + console 透传 | PASS | tools.rs terminal_output_object_from_output（限 run_terminal_cmd+truncated）；lib.rs host 实机 30K 测试；host_exec 三字段配对 + console 重建透传 |
| T1.12 env_snapshot（≤5s、白名单、输入在场、无 allowlist/任务结论） | PASS（本机实测数值 NOT-VERIFIED，见 M1R-4） | env_snapshot.rs PROBES + 1s timeout + VALUE_CAP；env.rs 白名单/排序/预算/空态 + 3 测试 |
| T1.12 section=env 越权守卫 | PASS | controller.rs render_env_section Err + host_exec O4 + blackboard.rs 2 回达测试 |
| T1.13 Linux 编译修复（xai-tty-utils 借用 + orz-tui EventType） | PASS | b18fabf5 两处 diff 正确；Windows cargo check 复检绿为门审计声明（未复跑） |
| T1.13 事件链 verifier/新生产者 | PARTIAL | Python fixture/verifier 273 声称可信（runtime/fixtures 齐全）；“tool_running idle_killed Rust 生产者”缺位（02 M1L-1） |

## 4. 正面发现

- output_object 的“对象 id=落盘 log 路径”设计把检索对象与既有
  read_file/grep 工具面打通，避免另造模型工具，符合零新面原则。
- env 快照生产侧注册表与渲染侧白名单双保险：生产侧只能产出
  tool/language/package/input/connectivity kind（PROBES 静态注册表 +
  input 行），渲染侧再拒越权 kind——“不泄露 allowlist/任务结论”在两层
  都有机械保障。
- read_file 档位改动把工具描述、结构注释、TOML clamp、测试门限四处同步
  更新，未发现 16K 残留（git grep 复核 43d7b9d3 后 HEAD 均无旧档位文案
  泄漏）。

## 5. 结论

T1.10–T1.12 实现合理、与设计及审计一致；T1.13 门除 idle-kill 生产者
声明过度（02 M1L-1）外基本可信。建议 P3 项（M1R-1/2/3）并入
review-handling；墙内 env 计时与检索对象端到端消费放 T2.4/M3 实机复验。
