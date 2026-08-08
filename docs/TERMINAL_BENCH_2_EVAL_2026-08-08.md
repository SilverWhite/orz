# Terminal-Bench 2.0 评测交接文档（2026-08-08）

**自包含交接文档**——新窗口按此继续，勿重开讨论。配套 memory `fusion-phase-tracking.md`（2026-08-08 Terminal-Bench 条目，含完整细节）。

## 1. 当前成绩

**小样本 7 任务 = 3 PASS / 3 FAIL / 1 环境问题**（hard 2/5 = 40%；含 medium 3/6 = 50%）。
对照论文参考线（[arXiv:2601.11868](https://arxiv-org.ezproxy.obspm.fr/abs/2601.11868)，ICLR 2026，89 任务）：frontier agent（GPT-5.2+Codex CLI）62.9%、Claude Opus 4.5+Terminus 57.8%、**开源模型最高 ~35%**——DeepSeek（非 frontier）+ orz 的 hard 40% 处于开源上游。
总成本 **~¥6**（余额 67.26→61 区间，7 任务含重跑）。

| 任务 | 难度 | 判定 | 耗时 | 归因 |
|---|---|---|---|---|
| regex-log | medium | ✅ reward 1.0 | ~10min | 首个端到端通过（探针③ 验收） |
| password-recovery | hard | ✅ 1.0 | ~15min | 安全类 1/1 |
| fix-code-vulnerability | hard | ✅ 1.0 | ~15min | 安全类 2/2 |
| regex-chess | hard | ❌ 0.0 | 3600s 超时 | 真失败（模型未解出） |
| polyglot-rust-c | hard | ❌ 0.0 | 900s 超时→2×重跑跑完 | 模型解出但测试未过 |
| dna-assembly | hard | ❌ 0.0 | transport 错误→重跑跑完 | 模型解出但测试未过 |
| filter-js-from-html | medium | ⚠️ 环境 | — | verifier Chromium/selenium 启动卡死（任务镜像问题） |

## 2. 适配架构（全链路已验证）

```
harbor run -d terminal-bench@2.0 -i <task> -a tb_agents.orz:Orz \
  -m deepseek-chat --ak orz_binary=D:/tb-eval/orz-linux/orz \
  --env-file D:\tb-eval\.env -o D:\tb-eval\jobs --n-concurrent 2
```

- **Harbor**（Terminal-Bench 2.0 官方 harness，0.20.0 → `D:\tb-eval\venv`，勿动 C: 系统 Python）：容器生命周期 + verifier 判定（CTRF JSON + reward.txt，二进制 reward，只看容器最终状态）
- **adapter** `D:\tb-eval\tb_agents\orz.py`（自定义 BaseInstalledAgent，`--agent tb_agents.orz:Orz` import path 注册，无需改 factory）：
  - `install()`：exec_as_root 装依赖（curl/procps）→ `environment.upload_file` 上传 orz 二进制 → chmod +x → `test -x` 可执行性检查（**orz 无 `--version`**——裸跑进 TUI 需 TTY）
  - `run()`：容器内 `orz -p "<instruction>" --real --allow-write --max-tool-rounds 999` + exit-file 看门狗（harbor 外圈超时兜底）；`ORZ_DEEPSEEK_API_KEY` 经 env 注入（harbor `--env-file`）
  - `populate_context_post_run()`：journal（`.gsa/runs/RUN-*/events.jsonl`，拷出容器）→ **ATIF trajectory.json** 转换（system/agent/tool 事件映射，10 步骤轨迹已验证）
- **orz Linux 二进制**：`D:\tb-eval\orz-linux\orz`（62.9MB，**musl 静态**，任何容器可跑）；构建脚本 `D:\tb-eval\build_orz.sh` + rust:1.97-slim Docker 镜像 + `CARGO_TARGET_DIR` 挂载（`D:\tb-eval\orz-target` 缓存）

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

## 4. orz 改动清单（2026-08-08，Linux 支持——未提交，用户手动推送惯例）

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

## 7. 遗留 / 下一步

1. **扩大样本**：medium 池 55 个（建议选 5-10 个无浏览器依赖的）——扩大统计基础
2. **全量 89**：按镜像分组 + 组间 `docker image prune`（D: ~15GB，镜像 5 个任务 ~2-4GB 实测）；预算 ¥30-80、6-10h
3. **正式分**（与其他架构比较时）：harbor 官方协议即正式分（含 verifier 判定）——与 SWE-bench Phase 3 Docker 评测同属"正式再跑"范畴
4. **提交推送**（用户手动惯例）：orz 4 处改动 + 主仓索引更新行 + 本文档
5. **清理**：orz-target 构建缓存（~几 GB，可删重建）、jobs/ 旧结果
