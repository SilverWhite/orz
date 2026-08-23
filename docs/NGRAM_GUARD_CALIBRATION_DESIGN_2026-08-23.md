# N-GRAM GUARD CALIBRATION 设计（2026-08-23：阈值 0.7 + 流内累计命中 + 口径统一）

> 状态：`implemented`（2026-08-23 设计定稿 + S1-S4 全部闭合；S1 代码
> 实施 + S2 测试闭合（orz 2563121 → 1c87681）+ S1 全面审查处理
> （D1 边界登记修正 + I1 审计日志修正）+ S3 重建（Linux musl，
> ORZ-BUILD-MOUNT-001，BUILD_EXIT=0，三件套 08:50 HKT）+ S4 复验闭环
> （G4/G5 对照零 3-gram trip、零 400、命中率全 ≥90%）——**计数 30 → 29**）。
> 入口：ADR-0010 §14.35 第 20 项 / BACKLOG 0d 后续 8 / TODO P0-0d 后续 8 /
> CLI_PROJECT_INDEX。关联：输出健康哨兵 3-gram 路径②（
> DEEPSEEK_OUTPUT_BUDGET_AND_STALL_GUARD_DESIGN §3.3/§3.5）。

## 1. 来源与问题

S4 冒烟（sweep-s4-g4/g5）与旧轮 G4 官方冒烟各出现一次 3-gram 路径②触发，
ratio 均显示 0.60（实际 0.600–0.609，`{:.2}` 舍入）、均为**正常推理收尾
自引用**边界误触发、单发即 trip（consecutive=1 → 降级一次）、任务继续
且非致命：

- 旧轮 portfolio-optimization（G4 official，L=200 二进制）：1 次；
- 本轮 video-processing（sweep-s4-g4，L=400 二进制）：1 次（收尾自检阶段，
  距结束约 2 分钟）。

两次独立任务都落在同一 0.60 上沿，说明正常推理自引用模式稳定压在阈值
附近。讨论裁决（2026-08-23 用户）：**阈值提升 + 流内累计命中 + 统一口径，
足够覆盖即可**。

## 2. 定案设计

### 2.1 阈值

- `DEGENERATION_NGRAM_REPEAT_RATIO` 0.60 → **0.70**；
- `ratio > 阈值` 严格大于语义保留；
- 信号表 / 常量注释 / 设计文档同步 0.70。

### 2.2 流内累计命中（与路径①纪律对齐）

- 每次 feed 时 1K token 窗口 ratio > 阈值 → 计 1 次命中；
- 流内累计 ≥ `NGRAM_HIT_LIMIT = 3` 才 trip（触发一次降级，consecutive
  递增，DEGENERATION_LIMIT 语义不变）；
- 1–2 次命中仅审计留痕：审计内容 = ratio + 窗口 token 数 + 族（3-gram
  无「重复 span」语义，不复用 span 上下文）；复用 `audit_hits` 通道，
  量小不聚合；
- 命中计数随流结束丢弃（同路径①）；间隔不重置。

### 2.3 统一口径

- WARN 中 `{ratio:.2}` → `{ratio:.3}`（或原始值），避免 0.600x 显示成
  0.60 误导判断；
- 边界测试用 0.69x / 0.70x 明确断言（>` 严格大于）。

### 2.4 边界登记

- **0.60–0.70 近重复循环漏判**（带变体的复读，路径①的 400 字符精确匹配
  抓不到）→ **2026-08-23 审查处理（D1）修正**：stall 兜底仅覆盖 reasoning
  族（stall 触发条件=无 content/tool_calls 且 reasoning 在流动）——
  reasoning 族由 stall（600s/64K）兜住，响应变慢可接受；**content 族为
  已接受漏判**（已见可见输出，stall 不启用）：精确循环仍由路径① 400
  字符精确匹配兜住，带变体循环将输出至 `REQUEST_MAX_TOKENS`（256K）/
  长度截断，成本受上限约束；S4 复验观察该区间实机表现，若出现真实
  content 族退化再评估保留低阈值或独立兜底；
- **证据边界**：两次真实触发的原始 reasoning 字节未留（WARN 无上下文），
  无法离线回放验证 0.7 清除；验证靠单测 + e2e 退化流改造 + 后续实机观察；
- 路径①（滚动哈希 L=400 + 二级确认）与 stall 兜底（reasoning 族语义）
  不受影响。

## 3. 实施路由

S1 代码（常量 + 累计逻辑 + 审计 + 口径）→ S2 测试（0.69/0.70 边界、3 次
累计、间隔不重置、既有 3-gram 用例适配）→ S3 重建（Linux musl，
ORZ-BUILD-MOUNT-001）→ S4 复验（正常任务零 3-gram trip、构造 0.6–0.7 流
仅审计、≥0.7 三连才 trip、零 400、命中率 ≥90%）。
计数：设计轮不动（28）；实施放行入账 28 → 29；S3/S4 验证闭环 29 → 28。

**2026-08-23 S1 实施闭合（用户放行；orz 2563121）**：
`DEGENERATION_NGRAM_REPEAT_RATIO` 0.60→0.70（`>` 保留）+ 新增
`NGRAM_HIT_LIMIT=3`（流内累计命中：每次 feed 超阈值计 1 次、≥3 才 trip、
1–2 次仅审计=ratio+窗口 token 数+族、间隔不重置、流结束丢弃）+ WARN
口径 `{:.2}`→`{:.3}`；基础设计信号表/参数表同步
（DEEPSEEK_OUTPUT_BUDGET... §3.3/§3.5）；既有 3-gram ratio 用例适配（单份
core 0.694 恰为 0.69x 边界样本留给 S2、双份 core 0.825 过 0.70 + 2 次审计
+ 3/3 trip 断言）；orz-loop 579 通过 / 0 失败 / 3 ignored、fmt 干净、
clippy 无新增。计数注：设计轮不动（28）；实际执行时 AGENT-DELIVERY-FLOW
已先入账 28 → 29，本 S1 实施入账 **29 → 30**（S3/S4 验证闭环时逐项回落
29 → 28）。登记于 ADR-0010 §14.35 第 23 项 / BACKLOG 0d 后续 8 /
TODO P0-0d 后续 8 / CLI_PROJECT_INDEX。

**2026-08-23 S2 测试闭合（用户放行；orz 1c87681）**：新增 6 项——
① 0.69x 边界（content 单份 8 词 core ratio≈0.694 < 0.70：永不计数、
永不触发）+ reasoning 镜像；② 0.70x 边界（9 词 core ratio≈0.720 >
0.70：窗口填满后每次超阈值 feed 计 1 次，审计 1/3→2/3、第 3 次 trip
3/3，`>` 严格大于两侧明确断言）；③ 单超大 feed 计 1 次命中（约 1200
token 单 delta 仅 1 审计、不 trip——命中按 feed 粒度）；④ 间隔不重置
（命中 2 后插入单 feed 300 互异 token 压窗口至 0.70 以下、间隔自身不计
命中且计数保持 2；重灌恢复超阈值后第 3 次命中 trip 3/3）；⑤ 流结束
丢弃（新流首命中只审计不 trip——计数不跨请求累积）；⑥ 既有 3-gram
用例适配已于 S1 完成。orz-loop **585 通过 / 0 失败 / 3 ignored**、fmt
干净、clippy 无新增。登记于 ADR-0010 §14.35 第 24 项 / BACKLOG 0d
后续 8 / TODO P0-0d 后续 8 / CLI_PROJECT_INDEX。

**2026-08-23 S1 全面审查处理（用户指示处理审查全部问题；计数不变仍 30）**：
审查结论=设计与实现整体一致、无功能缺陷；处理 2 项——D1 边界登记修正
（§2.4：0.60–0.70 近重复循环的「stall 兜底」仅覆盖 reasoning 族；content
族为已接受漏判——精确循环由路径①兜住、带变体循环成本受 REQUEST_MAX_TOKENS
约束、S4 观察）+ 基础设计 §3.3 注记同步；I1 审计日志字段（调用方审计
WARN 按路径标注命中门槛 `rolling_hit_limit` / `ngram_hit_limit`，消除
NGRAM_HIT_LIMIT 日后单独调整时的误标风险）。orz-loop 585 通过 / 0 失败 /
3 ignored、fmt 干净、clippy 无新增。登记于 ADR-0010 §14.35 第 25 项 /
BACKLOG 0d 后续 8 / TODO P0-0d 后续 8 / CLI_PROJECT_INDEX。

**2026-08-23 S3 重建登记（用户指示进行重建；Linux musl，ORZ-BUILD-
MOUNT-001 契约，`build_orz_aliyun.sh`；计数不变仍 30，S4 复验闭环后
回落 30 → 29）**：容器增量构建（`rust:1.97-slim`；挂载 `D:\CLI:/orz`、
工作目录 `/orz/orz`；apt 阿里云镜像 + 官方 static.rust-lang.org + 静态
rg 15.0.0 源码安装；`-j 1`）**BUILD_EXIT=0**；三件套产物时间戳
**2026-08-23 08:50 HKT**（orz 104,664,664 B / orz-signer 1,388,592 B /
orz-acaf-provision 1,206,568 B；SHA256=orz
8E43E96861982BF2709A1501D540BCB5153CE6873C4AE7A40C03556B43210414 /
signer 8288EAAF9784C1406C707FC01F1FDE2C2A9EBBD24FFD04242D1F369CD25A2403 /
provision 54D2AFDAA9592A55C565409FB8E612C641BE563A827F3E0635B9174F7164466A）；
最小可执行冒烟=三件均正常加载执行（orz 无 TTY 报 TUI io error 属预期——
headless 真机面由 S4 任务容器验证；provision 打印 usage；signer 报
manifest 缺失）。对应源码=orz 05231f7（S1 实施 + 审查处理）+ 父仓库
35788a0。待续：S4 复验（正常任务零 3-gram trip、0.60–0.70 流仅审计、
≥0.70 三连才 trip、零真实 400、命中率 ≥90%；content 族 0.60–0.70 区间
实机观察）。登记于 ADR-0010 §14.35 第 26 项 / BACKLOG 0d 后续 8 /
TODO P0-0d 后续 8 / CLI_PROJECT_INDEX。

**2026-08-23 S4 复验闭环（用户放行；G4/G5 对照，sweep-s4n-g4 /
sweep-s4n-g5（code-from-image 经充值后单题重跑 sweep-s4n-g5-cfi），
k=1、官方方式（无 max_wallclock）、n-concurrent=1；**S3/S4 验证闭环，
计数 30 → 29**）**：新二进制（orz 05231f7 构建轮，104,664,664 B）
实机运行——

- **G4（git-multibranch / sam-cell-seg / portfolio-optimization /
  video-processing / mcmc-sampling-stan，2h05m，3/5 reward 1.0）**：
  **全 5 题零 3-gram trip、零输出健康哨兵触发、零退化、零 400**——
  含历史误触发对照：portfolio-optimization（旧 G4 官方轮 1 次 3-gram
  0.60 边界触发，本轮零触发且 reward 1.0）、video-processing（旧
  sweep-s4-g4 1 次 3-gram 0.60 边界触发，本轮零触发；本轮
  AgentTimeoutError=官方任务超时、机制无异常、journal 完整 300 请求）；
  sam-cell-seg 零复读触发（对照旧 L=200 轮 2 次）；命中率 96.92%–
  98.55% 全 ≥90%（5/5 有 journal）。
- **G5（path-tracing-reverse / mteb-retrieve / code-from-image /
  break-filter-js-from-html / sanitize-git-repo，主轮 4/5 后 harbor 遇
  httpx ConnectError 网络抖动退出；resume 补跑 code-from-image 时 API
  账户余额不足（Insufficient Balance）→ NonZeroAgentExitCodeError；
  用户充值后单题重跑（sweep-s4n-g5-cfi）reward 1.0 完成）**：全 5 题
  零 3-gram trip、零触发、零 400；命中率 94.11%–98.32% 全 ≥90%
  （5/5 有 journal）；两次异常均非哨兵、非 400、非设计问题。
- **构造流行为（判定层，本次实测）**：0.694（content+reasoning）<
  0.70 永不计数不触发、0.720 > 0.70 窗口填满后每次超阈值 feed 计 1 次、
  第 3 次才 trip（审计 1/3→2/3→3/3）；e2e 退化流中断+降级全绿——
  boundary 3 项 + e2e 2 项实测通过，全量 orz-loop 585/0/3。
- **观察项（登记）**：本轮 10 试次均无 3-gram 审计条目（无窗口 ratio
  > 0.70 出现）——历史 0.60–0.61 边界带本轮未再现；content 族
  0.60–0.70 区间实机样本仍为空，保持 §2.4 已接受漏判登记，后续实机
  继续观察。

**计数：S3/S4 验证闭环 30 → 29（AGENT-DELIVERY-FLOW S3/S4 待续，
闭环后 29 → 28）。**登记于 ADR-0010 §14.35 第 27 项 / BACKLOG 0d
后续 8 / TODO P0-0d 后续 8 / CLI_PROJECT_INDEX。

## 4. 验收标准（DoD）

- S2 全绿、fmt 干净、clippy 与基线一致；
- S3 重建成功、三件套时间戳更新；
- S4：正常任务零 3-gram trip（0.6x 仅审计）、构造 0.60–0.70 流不 trip、
  ≥0.70 连续 3 次命中才 trip、零真实 400、命中率 ≥90%。

## 5. 风险与回滚

- 阈值提升降低 0.60–0.70 近重复循环灵敏度 → stall 兜底补偿，已登记边界；
- 累计命中延迟 trip 至第 3 次命中 → 病态流连续 delta 超阈值，延迟可忽略；
- 回滚：阈值可回退 0.60；累计命中可独立关闭。
