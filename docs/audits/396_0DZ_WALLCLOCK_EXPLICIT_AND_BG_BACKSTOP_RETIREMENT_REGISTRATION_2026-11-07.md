# 396 批：0dz 立项——进程墙钟显式化与 30h 绝对兜底退役（2026-11-07）

> 日期：2026-11-07（用户令「请进行立项吧」；提案原话「模型在启动进程的时候要显式提醒模型需要设置墙钟，现在会忘了设置墙钟而被默认墙钟杀进程。显式传了墙钟的进程就不用时间限制好了，兜底的30h取消如何？」）
> 性质：立项登记批——零源码、零重建、零换装；计数 **72 → 73**。
> 勘定＝主会话当轮只读源码核查（`orz-tools` 两文件），机制三层与教学面全部实证在案。

## §0 结论

**0dz 立项**（`pending`；S1 落码随下一源码批、S2 进体随下次重建换装 0.9.14-7、S3 真机复验）。范围＝后台任务寿命语义三件：①启动时 just-in-time 墙钟提醒 ②显式墙钟即唯一上限 ③30h 绝对兜底连根退役。前台缺省（5min）／auto-bg 180s 预算／生产 `max_timeout_secs` 不动。代价（用户裁决接受）＝后台任务再无机械上限，残余控制＝`kill_task`＋桥生命周期＋提醒行。

## §1 现状机制勘定（双层 30h 夹取＋教学面四处）

1. **模型面绝对上限**＝`ABSOLUTE_MAX_TIMEOUT_MS = 108_000_000ms＝30h`（`orz-tools/src/implementations/grok_build/bash/mod.rs:642`）：模型**显式传的** timeout 亦被 `.min()` 夹到 ≤30h（`bash/mod.rs:1395`）——「3d、150ep」级 72h 显式墙钟在模型面即被砍半。
2. **后台执行器兜底**＝`DEFAULT_BG_MAX_RUNTIME = 30h`（`computer/local/terminal.rs:58`，0dh S2 自 10h 放宽）：**唯一执法点**为 poll tick 的 `process.timeout.min(max_runtime)`（`terminal.rs:52,813`）——`timeout: 0`/缺省（解析 `Duration::MAX`）的后台任务同样被 30h 杀；env `GROK_BG_MAX_RUNTIME_MS` 可调高（无天花板）但不可关（`BG_MAX_RUNTIME_FLOOR ≥1h`，「兜底不可被配置掉」系写入注释的哲学；`terminal.rs:54,5817-5841` 三条测试钉死此语义）。
3. **模型面与执行器锁步**＝`absolute_max_timeout_is_30h_and_matches_executor_backstop` 钉（`bash/mod.rs:5137`）把两层锁死同一 30h。
4. **教学面**＝「~30h safety cap」四处（`bash/mod.rs:348/1578/1769/1772`）。
5. **前台另层（本项不动）**＝omit 缺省 5 分钟（transport-safe）／生产 `max_timeout_secs` 10h（xai-grok-shell `BashToolConfig`）／180s 阻塞预算 auto-bg（超阈后台化不杀，TER T1.2 语义）。

用户事故类目＝后台长训练（多日级）或被 auto-bg 的长命令，无论模型是否显式设墙钟，最终均死于 30h 层；忘设墙钟只是显性触发形态。

## §2 立项内容（三件一并落，半拆造假教学）

1. **启动时 just-in-time 提醒**：后台任务（显式 `is_background` 与 auto-backgrounded 同口径）spawn 时若未显式传 timeout，启动结果文案（`bash/mod.rs:1578` 现成后台启动行）追加「未设墙钟：任务将运行至退出或手动停止为止；长时间任务建议显式设 timeout」。形态＝结果行内嵌、静态工具描述不加长、零常驻注入（TER「结果文案 just-in-time」纪律）。
2. **显式墙钟即唯一上限**：模型面绝对夹取退役（`bash/mod.rs:1395` `.min(ABSOLUTE_MAX_TIMEOUT_MS)` 臂撤除；显式值多大照传）。前台仍受 `max_timeout_secs` 与 auto-bg 预算约束＝生产前台语义不变；后台显式值从此只受自身约束。
3. **30h 兜底连根退役**：撤 `DEFAULT_BG_MAX_RUNTIME`＋`bg_max_runtime_from_env`＋`GROK_BG_MAX_RUNTIME_MS`＋`BG_MAX_RUNTIME_FLOOR`＋lockstep 钉（`bash/mod.rs:5137`）＋`terminal.rs:5817-5841` 三条 env 兜底测试；教学面四处「~30h safety cap」改如实表述（「运行至退出或你停止为止」）。**只拆执行器留模型面（或反之）都会造成教学文案与机制的假承诺**（299「教学文案承诺假不变量」家族），故三件一并落。

## §3 代价与残余控制（用户裁决接受）

- 后台任务自此再无任何机械上限：忘设墙钟、或显式传超大值的任务，运行至**进程自然退出 / 模型 `kill_task` / 会话与桥退出随进程树回收**为止。
- 残余控制＝`kill_task`（模型自主）＋桥/子进程生命周期（宿主级回收）＋①的 just-in-time 提醒行（唯一预防层）。与 0dh「不做收割」、0bq「run 尾扫净」既有裁决同向；个人机资源面用户自担。
- 教学面必须与机制同步为「无上限」如实表述，不得残留 30h 承诺。

## §4 判据（S3 真机复验随换装窗口）

1. 显式 `timeout > 30h`（如 72h）不再被模型面夹取、不被执行器杀（journal/任务面可核）。
2. 无 timeout 后台任务越过原 30h 界不受限运行（真机长任务观察）。
3. 未显式设 timeout 的后台启动结果行携带提醒文案；显式设值时无此行。
4. 教学面四处零 30h 残留（字节判据）；`GROK_BG_MAX_RUNTIME_MS` 死配置清零。

## §5 批序

- **S1 落码**随下一源码批（与 0dy S1 同窗，均可进 0.9.14-7）。
- **S2 载体重建换装 0.9.14-7**（orz-tools 编译期进体）。
- **S3 真机复验**＝§4 四判据，与 0dy S3（0dx 活体两判据＋0dt 判据 4＋方向键手感）合并同一换装窗口执行。

## §6 入口

- TER 权威 [`TOOL_EXECUTION_LAYER_REFORM_DESIGN`](../TOOL_EXECUTION_LAYER_REFORM_DESIGN_2026-09-03.md)（「10h 绝对兜底」原设）／ADR-0010 §14.55、§14.87（0dh 30h 放宽）／0dh 立项与 S2（294 批档起）。
- 代码面：`orz-tools/src/implementations/grok_build/bash/mod.rs`（L348/642/1395/1578/1769/1772/5137）／`orz-tools/src/computer/local/terminal.rs`（L47-75/813/5817-5841）。
- 台账：TODO 396 行／BACKLOG 本批记录指针／索引头 v4.356；0dz 明细见 TODO P1-0dz／BACKLOG 开放项行。
