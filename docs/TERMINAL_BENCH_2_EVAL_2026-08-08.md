# Terminal-Bench 2.0 评测交接文档（2026-08-08）

**自包含交接文档**——新窗口按此继续，勿重开讨论。配套 memory `fusion-phase-tracking.md`（2026-08-08 Terminal-Bench 条目，含完整细节）。

**定位勘误（2026-08-08 深夜）**：当前运行是 pre-beta 探索性工程诊断，主要用于暴露任务薄弱点、运行时挂死和 harness 适配问题，不是正式 benchmark/pass@1。统计口径审计见 `docs/TERMINAL_BENCH_2_EXPLORATORY_SCORE_AUDIT_2026-08-08.md`。

## 1. 当前成绩

**探索性混合诊断台账 12 题，按每任务 best-observed = 10 PASS / 2 FAIL（83%）**。其中实际为 **11 个 hard + 1 个 medium**（`custom-memory-heap-crash` 官方难度为 medium）；修正后的 hard 唯一任务 best-observed 为 **9/11（82%）**，第一次获得 verifier 分数的 hard 尝试为 **7/11**，全部有分数 hard 试次为 **9/17**。这些数值用于工程诊断，不与论文完整 89 题、重复运行的 resolution rate 直接比较（[arXiv:2601.11868](https://arxiv.org/abs/2601.11868)）。medium 池尚未系统展开，但已有 regex-log、filter-js-from-html、custom-memory-heap-crash 三项诊断运行。

**首批 7 任务（2026-08-08 早）** = 3 PASS / 3 FAIL / 1 环境问题（hard 2/5 = 40% 当时口径，未含后续）。总成本 ~¥6（余额 67.26→61 区间）。

| 任务 | 难度 | 判定 | 耗时 | 归因 |
|---|---|---|---|---|
| regex-log | medium | ✅ reward 1.0 | ~10min | 首个端到端通过（探针③ 验收） |
| password-recovery | hard | ✅ 1.0 | ~15min | 安全类 1/1 |
| fix-code-vulnerability | hard | ✅ 1.0 | ~15min | 安全类 2/2 |
| regex-chess | hard | ❌ 0.0 | 3600s 超时 | 真失败（模型未解出） |
| polyglot-rust-c | hard | ❌ 0.0 | 900s 超时→2×重跑跑完 | 模型解出但测试未过 |
| dna-assembly | hard | ❌ 0.0 | transport 错误→重跑跑完 | 模型解出但测试未过（**后续 16:02 首次出现 verifier PASS，但不构成稳定性证明**） |
| filter-js-from-html | medium | ⚠️ 环境 | — | verifier Chromium/selenium 启动卡死（任务镜像问题） |

**混合稳定性/机制诊断（2026-08-08 15:13 二进制，2 hard + 1 medium，A4-A6+C.1/C.2 后）**：cancel-async-tasks（hard）✅ 1.0 / custom-memory-heap-crash（medium）✅ 1.0（run_invalidated 终局但 verifier 判定通过）/ dna-assembly（hard）✅ 1.0（16:02 首过）。其中 dna-assembly 共 5 个有分数试次仅 1 次通过，另有 1 次无分数错误；该 PASS 同时带 `AgentTimeoutError`，只证明最终容器状态曾通过 verifier，不证明运行稳定。

**hard 补样本批 5 题（2026-08-08 晚间，探索性 best-observed）** = **5/5 PASS**：

| 任务 | 判定 | 耗时 | 备注 |
|---|---|---|---|
| configure-git-webserver | ✅ 1.0 | 快 | 配置类命中 |
| sparql-university | ✅ 1.0 | ~10min | 查询类命中（沙箱无 SPARQL 引擎，模型人工推演 30 学生/9 课/11 院系/7 教师） |
| feal-linear-cryptanalysis | ✅ 1.0 | 11m（双子星 job 合计） | 密码学强项 |
| feal-differential-cryptanalysis | ✅ 1.0 | 同上 | 密码学强项 |
| write-compressor | ✅ 1.0 | 14m20s（重跑） | 见 §6 失败→重跑记录 |

**⚠️ 本批关键教训：5 题首跑用的是无守卫旧二进制（17:02 构建，P0-2/P1-1 不在内），write-compressor 因此被 harness 1800s 硬杀（reward 0.0）——重建含守卫二进制后仅重跑 write-compressor 并 PASS（§6）**。前四题在旧二进制上的 verifier PASS 是真实任务结果；因此“选定任务集合 best-observed 5/5”成立，但不能表述为“新守卫二进制 5/5”。

**B 组 5 题验证批（2026-08-09，守卫二进制 00:42 重建含 502ff84；探索性 best-observed 首跑）** = **1/5 PASS**。本批核心目标为**守卫+无回归验证**（验证 L1 GROK_HOME 注入 + 三代理审查修复可用性），解题正确性为附带观察。计划：`docs/TB_HARD_BATCH5_VALIDATION_2026-08-09.md`。

| 任务 | 难度 | 判定 | 耗时（journal） | 终局/归因 |
|---|---|---|---|---|
| model-extraction-relu-logits | hard | ✅ 1.0 | 20.4min | run_invalidated{restart_requested} 但工作产物已落盘，verifier 1.0（停滞守卫兜底先例，同 custom-memory-heap-crash） |
| gpt2-codegolf | hard | ❌ 0.0 | 18.5min | run_finished{completed}，5 工具轮收工未解出 |
| make-doom-for-mips | hard | ❌ 0.0 | 6.7min | run_invalidated{restart_requested}，35 工具轮快速迭代后内容停滞触发 restart |
| path-tracing | hard | ❌ 0.0 | 59.0min | **run_invalidated{wallclock}——P0-2 max_wallclock 首次真实触发**：3540s 满预算优雅收尾（FIFO flush→文件恢复链→run_invalidated 写入→退出 0），harbor 报 completed 非硬杀，归因合理（1800s 任务 ×2 倍率预算真实耗尽）非误杀 |
| train-fasttext | hard | ❌ 0.0 | 8.7min | run_finished{completed}，11 工具轮未解出 |

**守卫+无回归验证结论（本批核心目标）**：**5/5 无挂死、无 harness 硬杀**（n_errored=0 / n_cancelled=0，全部正常完成）；wallclock 兜底真实生效且优雅收尾（path-tracing）；stall 零触发；restart_requested ×2 为存量内容停滞机制正常工作（其中 1 次带 verifier 1.0）；L1 GROK_HOME 注入零干扰（5 题 journal 全部实时落 `--mounts` 卷、`.gsa/runs/RUN-*` + whitelist 结构正常、无注入相关报错）；headless `-p` 路径零行为变化（grill 分支隔离验证）。**全部 5 题同一新二进制**（00:42 构建 66.5MB，strings 验证 `max-wallclock`×3 / `grok-home` 命中；stale-binary 教训遵守）。环境：镜像 5/5 预拉、Docker 正常、Job A 3 题共用卷 b3-900s（trial-uuid 子目录区分）、Job B/C 独立卷。

**失败归因（2026-08-09 复盘，4/4 方向正确，非模型方向错误）**：4 个 FAIL 全部有明确机制归因——

**缺口 1（框架，实锤）：异步工具（monitor/task）结果未回流**。orz 的 `monitor`/`task` 为异步工具（返回 task_id，结果需显式 `get_task_output` 取回），**无完成自动注入**。两个独立案例同一模式：gpt2-codegolf 启动 task（查 ckpt 格式）+ monitor（xxd 头部）后**从未 get_task_output**，2 空轮后 completed（58 事件 / 5 轮收工，产物缺失）；train-fasttext 前 4 个 monitor 均正确取回（模型熟练），但**最后一个训练 monitor（几十分钟）启动后模型声明"等待通知"即 completed**——训练结果未取回，model 文件缺失（verifier test_accuracy 0.0007s 秒败）。run_finished 时挂起任务结果被静默丢弃，无兜底。TB 长命令场景新暴露（polyglot 时代 run_tests 同步无此面）。**已缓解（2026-08-09，orz `cfa551f`）**：封禁整个 GrokBuild 异步调度生态——task / get_task_output / wait_tasks / get_terminal_command_output / kill_task / kill_terminal_command / monitor / scheduler_create / scheduler_delete / scheduler_list / workflow 共 11 工具（依据：CN §7.2 唯二子代理 + §4.5 不发展本地多代理调度器——task 即第三类子代理）+ `run_terminal_cmd enabled_background=false`（bash 同步化，后台依赖 kill_task 观察/取消）。模型只剩同步工具面，缺口从工具面根除；长命令走 P0-1 5min 超时 + P1-1 看门狗。设计裁决：**不自写/不从 Codex 借异步调度**（违反 §4.5；Codex 协议即同步 turn 哲学无物可借）；未来长任务需求走 run_tests 先例（host-owned 同步长任务）或 CN §5.1 显式评审。测试 orz-host 98 / orz-loop 124 / orz-tui 177 / orz-codex 33 / orz-bin 1 全绿。

**缺口 1 残余两条（2026-08-09 记录）**：① 继承 crate 调度代码未删除（"不删除"裁决——Slice #13 继承 crate 不动纪律 + orz-agent 内部测试锁定 TaskTool 存在 + terminal/session 底层耦合）——运行面无任何入口可达（工具集过滤 + SessionContext 无调度句柄注入），但未来扩展工具集时必须保持 BANNED 清单维护（tools.rs 封禁注释已自证）；② 未来若出现真实长任务需求（如 TB 训练类），正确路径为 run_tests 先例的 host-owned 同步长任务工具（超时/打点/落盘保证齐备）或按 CN §5.1 显式评审引入——禁止以任何形式复活异步调度生态。

**缺口 2（框架，与中立问询机制相关）：中立问询触发但未矫正行为**。path-tracing（147 调用 59min，131+ 次 read 纯像素逆向，wallclock 耗尽未动笔写 image.c）与 make-doom-for-mips（53 调用 6.7min 侦察循环后 restart）均触发 neutral_inquiry 5-6 次，但模型行为未被拉回——机制触发正确但矫正失效，详见 §7 中立问询机制检查（2026-08-09）。

**缺口 3（框架，承接 polyglot 教训）：TB 适配无 run_tests 反馈环 + completed 无产物校验**。TB 测试隐藏 + verifier 一次性，模型无法自测（polyglot "无反馈环盲改"教训重现）；`run_finished{completed}` 为模型单方面声明，交付物缺失（gpt2.c / model.bin / image.c 均不存在）框架零干预。SWE-bench 已有 ORZ_TEST_RUNNER 先例，TB adapter 可复用。

## 2. 适配架构（全链路已验证）

```
harbor run -d terminal-bench@2.0 -i <task> -a tb_agents.orz:Orz \
  -m deepseek-chat --ak orz_binary=D:/tb-eval/orz-linux/orz \
  --ak gsa_volume=D:/tb-eval/gsa-volumes/<job> \
  --ak max_wallclock=<agent超时-余量> \
  --agent-timeout-multiplier 2 \
  --mounts '[{"type": "bind", "source": "D:/tb-eval/gsa-volumes/<job>", "target": "/orz-gsa"}]' \
  --env-file D:\tb-eval\.env -o D:\tb-eval\jobs --n-concurrent 2 -y
```

**⚠️ 必带 PYTHONPATH**：`PYTHONPATH=D:/tb-eval`（否则 `No module named 'tb_agents'` 启动即失败——2026-08-08 晚实测）。
**⚠️ 每次评测前必须重建 orz-linux 二进制**（见 §6 stale-binary 教训）：`MSYS_NO_PATHCONV=1 docker run --rm -v D:/CLI/orz:/orz -v D:/tb-eval/cargo-config.toml:/root/.cargo/config.toml -v D:/tb-eval/orz-target:/target -v D:/tb-eval/orz-linux:/out -w /orz rust:1.97-slim bash -c "$(cat /d/tb-eval/build_orz_aliyun.sh)"`（增量 ~1h20m 全量；MSYS_NO_PATHCONV 防 `-w /orz` 被转成 `B:/Git/orz`）。
- `--ak max_wallclock` 按 job 级全局：900s×2 超时任务分 job 跑（900s 任务 agent 1800s → 1740；1800s 任务 3600s → 3540）

- **Harbor**（Terminal-Bench 2.0 官方 harness，0.20.0 → `D:\tb-eval\venv`，勿动 C: 系统 Python）：容器生命周期 + verifier 判定（CTRF JSON + reward.txt，二进制 reward，只看容器最终状态）
- **adapter** `D:\tb-eval\tb_agents\orz.py`（自定义 BaseInstalledAgent，`--agent tb_agents.orz:Orz` import path 注册，无需改 factory）：
  - `install()`：exec_as_root 装依赖（curl/procps）→ `environment.upload_file` 上传 orz 二进制 → chmod +x → `test -x` 可执行性检查（**orz 无 `--version`**——裸跑进 TUI 需 TTY）
  - `run()`：容器内 `orz -p "<instruction>" --real --allow-write --max-tool-rounds 999` + exit-file 看门狗（harbor 外圈超时兜底）；`ORZ_DEEPSEEK_API_KEY` 经 env 注入（harbor `--env-file`）
  - `populate_context_post_run()`：journal（`.gsa/runs/RUN-*/events.jsonl`，拷出容器）→ **ATIF trajectory.json** 转换（system/agent/tool 事件映射，10 步骤轨迹已验证）
- **orz Linux 二进制**：`D:\tb-eval\orz-linux\orz`（当前守卫构建约 66.5MB，**musl 静态**，任何容器可跑）；构建脚本 `D:\tb-eval\build_orz.sh` + rust:1.97-slim Docker 镜像 + `CARGO_TARGET_DIR` 挂载（`D:\tb-eval\orz-target` 缓存）

## 3. 环境（全在 D:，C:/B: 不动——用户裁决）

```
D:\tb-eval\
  venv\            harbor 0.20.0（独立 venv）
  tb_agents\orz.py  自定义 agent adapter
  orz-linux\orz      Linux musl 二进制（构建产物）
  orz-target\        构建缓存（CARGO_TARGET_DIR）
  terminal-bench-2\ 89 任务源码 clone（github 经 git 代理）
  cargo-config.toml / build_orz.sh / write_env_key.py / 辅助脚本
  .env               ORZ_DEEPSEEK_API_KEY（write_env_key.py 从 Windows 凭据写入，勿打印）
  jobs\              harbor 结果（每任务 result.json + trial.log + agent/trajectory.json + verifier/）
```

- 数据集：harbor download terminal-bench@2.0（89 任务，缓存 ~/.cache/harbor/tasks）；github 克隆需代理（见 §5）
- 任务镜像：`alexgshaw/*:20251031`（预构建，Docker Hub 拉取）

## 4. orz 改动清单（2026-08-08，Linux 支持——已提交于 orz `f03faeb`）

| 文件 | 改动 | 性质 |
|---|---|---|
| `orz-loop/src/gateway/credentials.rs` | 非 Windows 分支：读 `ORZ_DEEPSEEK_API_KEY` env（容器无 Credential Manager；Windows CredReadW 不变） | **ADR-0006 扩展**（Linux 容器通道，注释已记录） |
| `orz-host/src/session.rs` | TrustPolicy::Enforce 非 Windows 降级 MemoryInstallationKeyStore + warning（DPAPI Windows 专属） | 平台适配 |
| `orz-config/src/managed_text/source.rs` | `_metadata`→`metadata` 参数名（L235/236 用错名） | **Linux cfg bug 修复**（Windows 下 cfg 掉从未暴露） |
| `xai-fast-worktree/src/api.rs` | `validate_cwd_scan`/`scan_contains_cwd` 去 `#[cfg(test)]`（Linux 生产路径 `live_process_cwds` 调用） | **Linux cfg bug 修复** |

**重要事实**：orz 的 Linux target 从未真正构建过（rust-toolchain.toml 声明了但首次构建 2026-08-08 才做）——以上 2 个 cfg bug 即首次构建暴露。**clippy 注意**：session.rs 非 Windows 分支下 `WindowsDpapiInstallationKeyStore` import 可能 unused warning（后续清理）。

## 5. 环境问题修复表（网络——国内网络对国外源不稳）

| 问题 | 修复 |
|---|---|
| static.rust-lang.org / crates.io 不可达 | rsproxy.cn 镜像（RUSTUP_DIST_SERVER/UPDATE_ROOT + cargo config.toml 挂载） |
| deb.debian.org 不可达 | build_orz.sh 内 sed 换 USTC 镜像 |
| github.com 不可达 | **git 全局代理已设**：`git config --global http.proxy http://127.0.0.1:7890`（系统 Clash 代理 127.0.0.1:7890——harbor 内部 git clone 继承） |
| Docker Hub 拉取中断 | 重试（间歇性，slim 镜像更稳） |
| DeepSeek API 瞬时 transport 错误 | 重跑（SWE-bench 同款现象；orz generate_stream 无重试是已知记录） |

## 6. 已知问题 / 经验

1. **filter-js-from-html verifier 卡死**：任务测试用 selenium 驱动 Chromium（root 容器缺 `--no-sandbox` 类配置）→ pytest 挂起 900s 超时。**任务环境问题非链路问题**（pytest 正常启动证明管道畅通、orz 无残留进程）。后续挑任务避开浏览器依赖（`list_hard.py` 有扫描 flag）。
2. **agent 超时经验**：hard 任务 900s 默认太紧（polyglot 超时、regex-chess 3600s 仍失败）——超时≠失败，`--agent-timeout-multiplier 2` 可放开；但模型解出≠测试过（2× 重跑后 polyglot/dna 均 0.0）。
3. **凭据纪律**：key 写 `.env`（`write_env_key.py` 静默，Ctrl 打印进 transcript 被拒过一次——正确）；容器内 orz 读 `ORZ_DEEPSEEK_API_KEY` env。
4. **trajectory 转换**：journal→ATIF 为保守映射（10 步轨迹样例 OK）；工具参数/输出截断 8000 字符上限。
5. **orz 容器内运行参数**：必须 `--allow-write`（SWE-bench harness 同款，否则写工具全 Deny——首次 e2e 失败根因）。
6. **⚠️ stale-binary 教训（2026-08-08 晚，write-compressor 失败归因）**：5 题补样本批首跑用的是 **17:02 旧二进制**——挂死守卫 P0-2/P1-1 不在内（strings 验证 `max-wallclock`/`ORZ_STALL_TIMEOUT` 0 命中）。orz-bin 的 `--max-wallclock` 是**手写解析**（main.rs:46，非 clap），旧二进制对未知 flag **静默忽略照常运行**——write-compressor 11:58:50 后静默 6 分钟（旧式挂死，无守卫），max_wallclock(1740s) 与 stall(360s) 均缺席，harness 1800s 硬杀（AgentTimeoutError 精确命中）→ reward 0.0。**重建含守卫二进制（20:55，66.5MB，strings 全命中）后重跑：14m20s PASS（8 工具轮，journal 终局 run_finished{completed}，守卫零干扰）**。**教训：评测前必须重建 orz-linux 二进制（构建命令见 §2）**；`--max-wallclock` 静默忽略的坑同样影响 orz.exe 宿主路径（手写解析无报错）。
7. **Docker daemon 掉线**：2026-08-08 晚 C 盘写满（0 字节剩余）期间 Docker daemon 挂掉（`docker ps`/`logs` 挂起、`harbor run` 报 "Docker daemon is not running"）——与 C 盘满关联（Docker Desktop 日志/配置在 C 盘）。恢复：重启 Docker Desktop 应用（服务需提权，UI 重启即可），~30s 拉起。**任务区产出已全部落 D 盘**（pip 用 `PIP_CACHE_DIR=D:/`、harbor 缓存仅 566K 可忽略），详见 memory `task-area-c-drive-policy.md`。

## 7. 遗留 / 下一步

1. **扩大样本**：medium 池 55 个（建议选 5-10 个无浏览器依赖的）——扩大统计基础
2. **全量 89**：按镜像分组 + 组间 `docker image prune`（D: ~15GB，镜像 5 个任务 ~2-4GB 实测）；预算 ¥30-80、6-10h
3. **正式分**（进入真人 beta 或与其他架构比较时再做）：Harbor + verifier 是必要执行链，但仅此不足以形成正式可比成绩；还需冻结二进制/adapter/model、使用预声明样本与预算、固定重跑规则，并按完整 89 题或预先固定的代表性样本报告 pass@1、全部试次与置信区间。当前结果继续按探索性诊断使用。
4. **文档提交/推送**：由用户决定；本次探索性成绩审计与口径勘误保持未提交状态
5. **清理**：orz-target 构建缓存（~几 GB，可删重建）、jobs/ 旧结果
