# FR-N01／FR-C04 提交与推送记录（载体 0.6.1）（2026-09-17）

- 用户指示（2026-09-17）：「请提交并进行推送吧，当前代理不可用，请使用真机环境进行直连推送」。
- 范围：把 orz 侧**已闭环**的两批改动落成提交并推送；父仓账本 pin 推进到新 orz 提交、重算
  `orz_source_manifest.sha256`、同步账本并推送。
- **明确不在本批**：0am 影子 RLI 批（用户口径＝落实随 **O2** 裁决放行，保持未提交）；Linux
  musl 三件套重建（用户 2026-09-17 指示「Linux 暂时用不上，重建不急」）。
- 性质：**记账＋提交＋推送批**（零新计数、零新代码逻辑；本体代码在本轮之前已落码并取得读数）。

## 1. orz 三笔提交（`feat/fusion-architecture`）

| 序 | 提交 | 内容 | 提交前复核读数 |
|---|---|---|---|
| 1 | `26dcce1b` | 摩擦盘点 **FR-C04 采②**：`EditRecord.run` 写时章（journal run id；空＝旧无章行）＋`render_run_context_block` 增 `current_run`，D4 自编辑清单按「本 run 自编辑文件／会话历史自编辑文件（此前 N 个 run ＋/或无章旧行）」两段渲染＋钉子 3 条 | orz-loop **785/0/3**（＝782 基线＋3 钉）、`cargo fmt -p orz-loop` 干净 |
| 2 | `05db4b3d` | **FR-N01**：`candidate_is_under_raw` Windows 分支改字节级 `eq_ignore_ascii_case`（禁字符串字节切片），修进程级 panic（`read_file("存档/docs/README.md")`）＋钉子 2 条 | orz-tools lib **2886/0/6**、`fmt` 干净、clippy 零新增；提交后定向复跑钉子 3 用例 **3 passed** |
| 3 | `1b047158` | `chore(release)`：bump 0.6.0 → **0.6.1**（载体重建源冻结）——含 Windows 三件套哈希、ACAF 重 provision、字面量 11 项核证与边界 | 见 §3（载体面读数） |

本批只提交**已闭环**内容；0am 相关改动整批留在工作树（未提交、未推送），随 O2 裁决处置。

## 2. 提交分离方法与复核口径（本轮新做法）

本工作树同时混存两条独立批次的改动（FR-C04 与 0am），且两者在 `blackboard.rs`／`controller.rs`
同文件不同符号处相邻。直接 `git add` 会把 0am 批一并卷入，因此本批采用**hunk 级分离**：

1. 用 `git diff -U3` 生成差分，按 hunk 过滤（剔除含 `rli_shadow` 的 0am hunk；`controller.rs`
   只保留 `run: String::new()` 两处 fixture hunk；`Cargo.lock` 只保留 `version = "0.6.1"` 一行），
   以 `git apply --cached` 入暂存区；
2. **隔离复核**：`git stash push --keep-index -u` 把 0am 批与 FR-N01 改动暂时搁置，使工作树恰好等于
   待提交内容，实跑 `cargo test -p orz-loop` 与 `fmt`，随后 `git stash pop` 原样恢复（原子恢复无冲突）。
   本批 FR-C04 的 785/0/3 即由此口径取得（**不是**含 0am 的同树读数 790/0/3）；
3. 提交后 `git status` 复核：工作树剩余项**仅为** 0am 批（11 改 2 未跟踪）。

暂存与过滤脚本、差分、测试日志留在仓库根的 `.tmp-*`（门禁排除面，作为一手证据保留）。

## 3. 载体 0.6.1 与 ACAF

- Windows 三件套（`D:\tb-eval\orz-windows`，旧件留 `.0.6.0-bak`）：`orz.exe` 53,937,152 B
  `5C991621…`／`orz-signer.exe` 6,742,528 B `759A1DEF…`／`orz-acaf-provision.exe` 6,642,176 B
  `EEE03BE9…`。
- clean 全量重建后 signer 哈希变更 ⇒ **ACAF manifest 按 052 先例重 provision**（keystore 保留，旧
  manifest 留 `.bak-20260917-061`），manifest↔signer 一致。
- 边界（如实登记）：**0.6.1 的二进制由含 0am 影子批的工作树构建**，而本批提交**不含** 0am ⇒
  「源冻结提交」与「实际二进制源」之间有已知差异；0am 批合回后如需可重现二进制，应再走一次重建流程。

## 4. 父仓账本与门禁

- **pin 推进**：`orz` 子模块指针 `5041c3dc` → **`1b047158`**。
- **manifest 重算**：`orz_source_manifest.sha256` 1457 条，差异恰 **12 文件 × 2 行**（＝三笔提交的
  文件面：FR-C04 九文件 ＋ `resources.rs` ＋ `crates/orz-bin/Cargo.toml` ＋ `Cargo.lock`）。
- **账本同步面**：索引头行 v3.62／§8 `GAP-BYTE-BOUNDARY-PANIC`（补提交行）／`OBS-FRICTION-INVENTORY`
  （FR-C04 由「已落码」改「已落码并提交」）／BACKLOG 0an（新增「提交」条、边界改写为「仅剩 Linux
  载体」）／BACKLOG 摩擦盘点 §11 批注／TODO 0an 勾「代码提交」／盘点档 §7b FR-N01 行补提交信息并
  **修正该行残留的错误路由片段**（原 `../..//TODO.md` 与下行重复）。
- 计数：**不变量**（0an 仍开放：仅剩 Linux 载体，故未闭合总数维持 **36**）。
- 门禁 `python scripts/check_repository.py`：唯一错误＝`orz submodule working tree is dirty`
  （＝仍留工作树的 0am 批，合回前预期态）；`git diff --check` 干净。

## 5. 推送记录

- 用户口径：代理不可用（`http.proxy = http://127.0.0.1:7890` 不可达）⇒ 本轮**真机直连**推送
  （推送命令显式给出 refspec，遵循 ADR-0010 §14.70 口径）。
- 目标引用：父仓 `origin`（`https://github.com/SilverWhite/CLI.git`）分支 `main`；orz 子模块
  `cli` 远端（同一仓库）分支 `feat/fusion-architecture`。
- 结果（真机直连实测）：父仓 `8d435551 → 7207e033`（`refs/heads/main`）；orz `5041c3dc → 1b047158`
  （`refs/heads/feat/fusion-architecture`）——两次推送均 exit 0，推送后远端引用与本地一致
  （`git status` 两仓均无 ahead/behind）。
- 账本提交：父仓 **`7207e033`**（本报告与 pin／manifest／账本同步入库笔）。

## 6. 本轮新摩擦登记

| 序 | 摩擦 | 证据 | 处置与建议 |
|---|---|---|---|
| **N-01（环境）** | **代理不可用**：仓库配置 `http.proxy=http://127.0.0.1:7890`，本轮不可达 ⇒ 任何 `git push/fetch` 直连失败（`Failed to connect to 127.0.0.1 port 7890`） | 本轮推送首次尝试；用户指示「真机直连」 | 处置：推送时以 `-c http.proxy= -c https.proxy=` 覆盖为直连（本轮成功）。建议固化：推送前先确认代理可达性，不可达即直连并留痕 |
| **N-02（构建门）** | 本工作区 shell 内 `cargo test` 触发 `orz-tools-api` 构建脚本失败：`protoc not found`（`build.rs:49 tonic_build`）——即已判定「不处理」的 **FR-A01** 在工作区侧的复发形态 | 本轮 `cargo test -p orz-loop` 首次运行失败原文 | 处置：设 `PROTOC=D:\tb-eval\.tools\protoc-25.3\bin\protoc.exe` 后全绿（本批两轮测试均以此运行）。**取数配方留档**（属真机工具目录，非沙箱内） |
| **N-03（流程门）** | **同树混存两批独立改动**使「提交」无法整体 `git add`：0am 与 FR-C04 在 `blackboard.rs`／`controller.rs` 相邻，且 `Cargo.lock` 被两批各改一行 | 本轮提交分离过程（§2） | 处置：hunk 级过滤入暂存 ＋ `stash --keep-index -u` 隔离复核 ＋ `git stash pop` 原子恢复。建议：后续批次尽量**先提交再接下一批**，避免长期同树混存 |

## 7. 边界与未决

- **0am 影子 RLI 批**：仍在工作树未提交（11 改 ＋ 2 未跟踪）；落实与合回随 **O2** 裁决。
- **Linux 载体**：按用户 2026-09-17 指示不重建（Docker 代理切换＋字节级还原流程见 052／060 先例）。
- **本批未做**：任何计数变更、任何新立项、任何 0am 代码面改动。
- **口径提醒**：0.6.1 二进制含 0am 代码，本提交线不含 ⇒ 二者差异已在 §3 登记，不作隐性等价声明。

## 8. 入口与关键词

入口：[`BACKLOG 0an`](../BACKLOG_AND_PRIORITIES.md) ／ [`摩擦盘点 §7b`](FRICTION_INVENTORY_2026-09-17.md) ／
[`处理狗粮报告`](FRICTION_INVENTORY_TREATMENT_DOGFOOD_2026-09-17.md) ／ [`060 载体重建先例`](060_CARRIER_REBUILD_2026-09-16.md)。
关键词：提交、推送、真机直连、hunk 分离、stash 隔离复核、pin 推进、manifest 重算、载体 0.6.1、FR-N01、FR-C04、0am 未提交。
