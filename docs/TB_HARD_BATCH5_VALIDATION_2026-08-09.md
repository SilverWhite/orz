# TB hard B 组 5 题验证批（2026-08-09 计划定稿，待执行）

**状态**: ✅ **已执行完成（2026-08-09）**。**目标**：验证当前修改（orz `502ff84`：L1 GROK_HOME 注入 + 三代理审查修复）的可用性——**守卫+无回归为主**，解题正确性作为附带观察。新窗口按此执行，勿重开讨论。

## 执行结果（2026-08-09）

**1/5 PASS**（model-extraction-relu-logits ✅ 1.0；gpt2-codegolf / make-doom-for-mips / path-tracing / train-fasttext ❌ 0.0）。**守卫+无回归验证目标达成**：5/5 无挂死无硬杀、wallclock 首次真实触发且优雅收尾（path-tracing 59min 满预算）、stall 零触发、restart_requested ×2 为存量机制正常工作、L1 注入零干扰（journal 实时落卷 + .gsa 结构正常）、headless 零行为变化。全部 5 题同一新二进制（00:42 构建 33m46s 增量、strings 验证 max-wallclock/grok-home 命中）。明细已回填 `TERMINAL_BENCH_2_EVAL` §1 台账。

执行记录：Job A（3×900s 题共用卷 b3-900s，wallclock 1740，29m23s）/ Job B（path-tracing 独立卷，wallclock 3540，60m04s）/ Job C（train-fasttext 独立卷，wallclock 7140，9m40s）。环境注记：Job C 首次 Bash 后台启动被 harness 偶发 kill（harbor 进程死、容器孤儿——docker rm 清理后重启成功）；PowerShell 5.1 传参剥离 `--mounts` JSON 内嵌双引号（JSONDecodeError char 2）——**mounts 参数必须走 Bash 单引号**。

## 1. 背景与决策（用户 2026-08-09 裁决）

- **不跑 TB 全量 89**——全量正式测试改跑别的（另行安排）。
- TB 探索性诊断继续：从 hard 池抽 5 道**未跑过**的题，用**当前最新源码重建的守卫二进制**跑，验证当前修改可用性。
- **选题 = B 组为主**（ML/系统类，探索新领域）：`model-extraction-relu-logits` / `gpt2-codegolf` / `make-doom-for-mips` / `path-tracing` / `train-fasttext`。
- **验证口径 = 守卫+无回归为主**：5 题能否正常跑完（无挂死/守卫不误杀/L1 注入不破坏解题），解题正确性附带观察（不因失败深入归因，除非守卫/框架归因）。

## 2. 盘点（2026-08-09）

- TB2 共 **30 个 hard**，已跑 11（best-observed 9/11，见 `TERMINAL_BENCH_2_EXPLORATORY_SCORE_AUDIT_2026-08-08.md` 勘误口径），**剩余 19 未跑**。
- 30 个 hard 均无浏览器依赖（`D:\tb-eval\list_hard.py` 扫描 tests 目录 selenium/chromium/playwright/headless/browser 零命中——不会重蹈 filter-js-from-html 的 Chromium 卡死）。
- B 组 5 题均有预构建镜像 `alexgshaw/*:20251031`（`task.toml: docker_image` 确认）。
- 环境（2026-08-09 检查）：Docker daemon 正常、D 盘 125GB 空闲 ✓。

## 3. 执行步骤

### 步骤 1：重建 orz-linux 守卫二进制（**必须**，当前二进制不含 502ff84）

**当前 `D:\tb-eval\orz-linux\orz` 是 20:55 构建（含 P0-P2 守卫，但不含昨晚 L1 + 三代理审查修复）**——验证"当前修改可用性"必须重建。构建命令（交接文档 `TERMINAL_BENCH_2_EVAL` §2）：

```powershell
MSYS_NO_PATHCONV=1 docker run --rm -v D:/CLI:/orz -v D:/tb-eval/cargo-config.toml:/root/.cargo/config.toml -v D:/tb-eval/orz-target:/target -v D:/tb-eval/orz-linux:/out -w /orz/orz rust:1.97-slim bash -c "$(cat /d/tb-eval/build_orz_aliyun.sh)"
```

- 脚本：`build_orz_aliyun.sh`（aliyun 镜像 + rsproxy + musl 静态 + `-p orz-bin -j 1`；增量构建复用 `orz-target` 缓存，预计 ~1h）。**挂载修正（2026-08-17）**：父仓库挂载为 `/orz`、工作目录 `/orz/orz`——orz-assurance `include_str!("../../../runtime/...")` 需解析到 `/orz/runtime`；脚本同时输出 orz / orz-signer / orz-acaf-provision 三件套。
- **构建后验证**：`strings /d/tb-eval/orz-linux/orz | grep -c "max-wallclock"`（守卫符号命中）+ `ls -la`（~66MB）+ 确认构建时间戳为新。
- **stale-binary 教训**（`TERMINAL_BENCH_2_EVAL` §6）：本次 5 题**必须全部用新二进制**，禁止旧二进制混跑。

### 步骤 2：5 题批 launch（同一二进制）

```powershell
# 每题一个 job 目录（gsa_volume 与 --mounts 对应）
PYTHONPATH=D:/tb-eval harbor run -d terminal-bench@2.0 -i <task> -a tb_agents.orz:Orz `
  -m deepseek-chat --ak orz_binary=D:/tb-eval/orz-linux/orz `
  --ak gsa_volume=D:/tb-eval/gsa-volumes/<job> `
  --ak max_wallclock=<agent超时-余量> `
  --agent-timeout-multiplier 2 `
  --mounts '[{"type": "bind", "source": "D:/tb-eval/gsa-volumes/<job>", "target": "/orz-gsa"}]' `
  --env-file D:\tb-eval\.env -o D:\tb-eval\jobs --n-concurrent 2 -y
```

- **必带 `PYTHONPATH=D:/tb-eval`**（否则 `No module named 'tb_agents'` 启动即失败）。
- `max_wallclock` 按任务超时：900s 任务 → 1740；1800s 任务 → 3540（`TERMINAL_BENCH_2_EVAL` §2）。
- **train-fasttext 注意**：训练类任务耗时可能长——launch 前查 `task.toml` 的默认超时，确认 `max_wallclock` 预算；若超时倍率 2× 仍紧，单独跑（不并 2 并发，避免共享 API 预算）。
- 5 题任务名：`model-extraction-relu-logits`、`gpt2-codegolf`、`make-doom-for-mips`、`path-tracing`、`train-fasttext`。

### 步骤 3：判定口径

- 探索性 best-observed（`EXPLORATORY_SCORE_AUDIT` §2 口径），每任务**首跑记录**；重跑仅限环境/守卫归因（如容器拉取失败、守卫误杀可复现）。
- 记录：reward.txt / CTRF summary / journal 终局（run_finished{completed} vs run_invalidated{wallclock/stall}）/ 耗时。

## 4. 验证点（对应本次修改）

| 修改 | 验证点 |
|---|---|
| 挂死守卫（P0-1/P0-2/P1-1，含审查修复 F1-F5） | 5 题无挂死无硬杀；`max_wallclock`/stall 兜底正常（如有触发，看终局事件归因是否合理） |
| L1 GROK_HOME 注入（`grok_home.rs`） | 容器内降级链不破坏解题：journal 正常落卷（`--mounts`）、`.gsa` 结构正常；无注入相关报错 |
| 三代理审查修复（502ff84：锁/失败轮/GRILL 清理等） | headless `-p` 路径**零行为变化**（grill 分支隔离验证）——5 题正常跑完即回归通过 |
| 解题正确性（附带观察） | verifier PASS 数、失败归因一句话（不深入） |

## 5. 成本

构建 ~1h（增量）+ 5 题 ~2-4h + ¥5-15（API 预算；train-fasttext 可能偏重，见步骤 2 注意）。

## 6. 完成后

- 结果回填 `TERMINAL_BENCH_2_EVAL` §1 台账（探索性口径）+ 本文档状态 ✅。
- 提交推送（用户手动惯例）；memory `fusion-phase-tracking` 更新。
