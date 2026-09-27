# 093 载体重建与换装（双平台 · 0.7.3 → **0.8.0**）

> **日期**：2026-09-27；**用户令**：「请进行重建吧，序号为0.8.0」（沿 091 先例：重建先行，落账待令）。
> **进体内容**＝0bv 过夜批（REV-083-21/22 修复＋REV-083-05/11/12＋0bw①–④，见[`过夜批档`](0BV_REMAINING_ITEMS_AND_S4_FIXES_2026-09-27.md)）＋同日复审修复（P0×1/P1×2/P2×11/P3×9 全处置，同档 §12）＋版本 bump 两文件两行。**本批即上述树面的首个进体载体，亦是 0bw② 的首个带 manifest 载体、0bw① 的首个 exec 后真机读数载体。**
> **源冻结＝worktree 态**（orz 子模块 HEAD `c558d6eb` 未动 ⇒ `orz_source_manifest.sha256` 按设计不动）；**未提交／未推送／未发行**（发行面仍停 `v0.7.0`）。

## 1. 版本 bump（两文件两行）

- `orz/crates/orz-bin/Cargo.toml:3` 0.7.3 → **0.8.0**；`orz/Cargo.lock:5598` orz-bin 条目同步。`cargo metadata --locked` **exit 0**。

## 2. Windows 增量重建与换装

- 前置：D: 余 4.5G ⇒ `cargo clean --profile dev` 腾挪 **30.3 GiB**（→ 33G；连续第七批同族操作，已惯例化）。
- 构建（两轮）：首轮失败两折——① `protoc` 缺失（dev 缓存清空后 `orz-tools-api` build script 重跑暴露；既往惯例 `PROTOC=D:\tb-eval\.tools\protoc-25.3\bin\protoc.exe`，见 062/068 档）补 env 后；② **rustc OOM**（`hir_typeck` 期 alloc 失败）——与 Linux Docker 构建并行抢内存所致，降 `-j 4`（068 惯例）过。第二轮（复审修复 `carrier_integrity` 未列检测对齐 bak 语义进体）28.95s 增量，**exit 0**；警告 **1 条**＝`permission.rs:22` unused imports（18g 既有遗留，非本批引入）。
- 换装：构建产物三件 → `D:\tb-eval\orz-windows\`，**MATCH 3/3**（旧三件留 `.0.7.3-bak` 链）；`--build-info` 换装位读数 **`version=0.8.0 os=windows arch=x86_64 profile=release`**。

| 文件 | 尺寸 (B) | 与 0.7.3 差异 | SHA256（前 16；全异） |
|---|---:|---:|---|
| `orz.exe` | 57,093,120 | **+93,696 B** | `8831b21f4402c324` |
| `orz-signer.exe` | 6,740,480 | 0 | `54bb982fabb0b46c` |
| `orz-acaf-provision.exe` | 6,640,128 | 0 | `90eb52239203edfb` |

## 3. 首个 carrier-manifest（0bw② 落地）＋完整性自检活体

- **生成**：`python scripts/generate_carrier_manifest.py D:/tb-eval/orz-windows --version 0.8.0` ⇒ **3 entries**（三件套；manifest 自身排除）。
- **摩擦与修复（真机首用即暴露）**：初版扫描把换装位 **80 个历史 bak 链**（`.0.7.3-bak` 连字符形态／`.bak-<日期>` 点形态）全数入册（83 条），搬移拷贝即报 79 条假「文件缺失」＝告警疲劳。**修复**：生成器排除 `.bak`/`-bak` 族（机器本地站点考古件非载体载荷）；**连带裁决**：运行时「未列文件检测」排除集与生成器镜像（`carrier_integrity.rs` 同语义）——换装位日常调用不作 bak 噪音。
- **活体三分验证**（换装位真实二进制）：
  - 清洁搬移（4 件拷临时目录）：**零横幅**（stderr 仅 rollback 用法报错）＝Clean 静默语义 ✓；
  - 篡改态（signer 追加 1 字节）：横幅**恰 1 条** `orz-signer.exe: 长度不符（清单 6740480 ≠ 实际 6740488）` ✓；
  - 换装位本位（80 bak 在场）：`rollback list` **零 carrier-integrity 行**（对齐修复生效）✓。
- 单测：`carrier_integrity` **8/0**（Windows 本机，含未列文件钉）。

## 4. ACAF 重 provision（两轮，随二轮构建重跑）

- 旧 manifest 留 `signer-manifest.json.bak-20260927-093`；provision **exit 0**；keystore 哈希前后逐位未动；manifest 回显 `binary_sha256=54bb982fabb0b46c…` ↔ 换装位 signer 逐位一致。

## 5. 字面量核证（Windows 二进制，出现次数法；对照＝0.7.3-bak）

| 字节模式 | 0.7.3 | 0.8.0 | 判定 |
|---|---:|---:|---|
| `[carrier-integrity]` ／ `未列文件（清单外新增）` ／ `清单键非法` | 0／0／0 | **2／1／1** | 0bw② 进体 ✅ |
| `已回退` ／ `拒绝以 .gsa 域为回退目标` ／ `不接受旗标` | 0／0／0 | **1／1／1** | 0bw④ undo 面进体 ✅ |
| `write_control_review` | 0 | **2** | 0bw③ 事件族进体 ✅ |
| `[写入管控·提示]` ／ `protected by the write control` | 1／2 | 1／2 | 0bw L1/L2 保持面 ✅ |
| `workspace/switch` ／ `orz-build-info: version=` ／ `360search` | 3／1／3 | 3／1／3 | 0.7.3 面零回退 ✅ |
| 判官错误文案（`closed set`／`carry the rule` 等） | 0 | 0 | 两版一致缺席＝判官消费面在 assurance 门、本不入载体，口径自洽 ✅ |
| `No, and tell the AI what to do differently` ／ `GrokBuild:` ／ `GROK_HOME` | 1／26／7 | 1／26／7 | 边界保留 ✅ |

- 注：`unknown rule`／`-class rule` 整串 0＝Rust format! 串片段化所致，非缺席信号（片段面如上口径自洽）。

## 6. 载体级 Web 探针（`.tmp-b093-win-web-probe.ps1`，端口 21561，沿 087/089/091 脚本）

- 全过：`/` 200／无令牌 **401**／伪 Host **401**／带令牌 **200**；`boot_keys = cwd,agent,trusted_workspaces`；归档 0 件；`leftover_orz_procs=0`。启动横幅 GBK 呈现＝既有已知观察面。

## 7. Linux musl 三件套（Docker，直写换装位；两轮）

- `rust:1.97-slim` ＋ `build_orz_aliyun_trixie.sh`（ORZ-BUILD-MOUNT-001 契约＋`MSYS_NO_PATHCONV=1`；**全量日志重定向**——091 摩擦修复）两轮均 **exit 0**；换装位旧三件预置 `.0.7.3-bak` 链后直写。
- **站点卫生（首用摩擦）**：`orz-linux/` 换装位混有 29 个历史构建日志＋FROZEN md ⇒ 首版 manifest 入册 32 条。**处置**：日志/文件移入 `orz-linux/buildlogs/` 子目录（目录不入 flat 扫描亦不入未列检测）——站点清理而非工具特例，二轮 manifest **3 entries**。

| 文件 | 尺寸 (B) | 与 0.7.3 差异 | SHA256（前 16） | BuildID |
|---|---:|---:|---|---|
| `orz` | 115,524,112 | **+147,800 B** | `8baeb23ea1dc800b` | `408a3d9b` |
| `orz-signer` | 1,401,360 | 0 | `3299d0a546fc815e` | `35b4813b` |
| `orz-acaf-provision` | 1,220,208 | +240 B | `a94d742537c9b9e2` | `c41657b6` |

- ELF 静态核验（alpine 3.20 ＋ binutils/file）：三件 `static-pie linked`＋**`PT_INTERP=0`**。
- 双向加载冒烟：debian bookworm-slim／alpine 3.20 `orz --build-info` **exit 0** 且读数 `version=0.8.0`；signer（usage fatal 形态）／provision（usage 形态）可加载可执行。
- 警告面：`orz-config` 1 条＋`orz-assurance/sandbox/job_object.rs`（Linux-only 既有）等，**`carrier_integrity.rs`／`child_write_guard.rs` 零告警**。

## 8. Landlock exec 后真机读数（0bw① 结转项达成；新增永久回归钉）

- **新测试**：`orz-sandbox/tests/child_write_guard_exec.rs`（Linux-only）——lib 单测因不能 self-restrict 只验探测门；本测试把装挂放进**子进程**（pre_exec 同生产接线：`prepare_allow_dirs(表B)` ⇒ `install_best_effort`），对内核取真实读数。
- **读数（`rust:1.97-slim` 容器）**：`kernel landlock ABI = 3`；装挂后 **写 `/tmp` 照常成功**（allowlist 补集正面）＋**写 `/etc` 被内核拒**（`Permission denied`、`/etc` 零落盘）。**1 passed / 0**。
- 设计 §3.3「exec 后真机读数随 Linux 载体批收取」的结转项就此达成；容器 seccomp 封 landlock 时测试显式 SKIP（读数如实，不假绿）。

## 9. orz rollback 真机回环（0bw④ 结转项达成）

- 造真快照（store 格式：fnv1a32 key8＋`<millis13>-<call8>.bak`＋`.bak.meta` 目标侧车）→ 换装位真二进制：
  - `rollback list`：1 条、meta 目标 `src/main.txt` 如实展示；
  - `rollback restore <pointer>`：**exit 0**，`已回退 src/main.txt ← …（14 字节）；覆写前内容已存 …-rollback.bak`——**目标 14 字节原样写回**＋**覆写前 11 字节（broken edit）自动入窗**＝undo 可逆；
  - 回环后再 list：2 条（新前像＋原快照）。
- CLI 严格化冒烟：无动词/未知旗标/多余位置参数 ⇒ **exit 2**；空窗口 list ⇒ 显式报错 exit 1。

## 10. 切换矩阵 ③④（API 级走查；UI 点击级留狗粮轮）

- 双实例（W1:21562／W2:21563）＋信任流：`POST /api/trust`（W2 自信）= 200，**信任清单全局共享**（W1 boot 立即可见 W2 条目）。
- **`POST /api/workspace/switch {path: W2}` ＝ 200**——REV-083-21/22 修复的换装位端到端实证（修复前该调用恒 403/ReferenceError）；响应回显 `cwd=W2`；**boot.cwd 随切翻转**＝④「新会话 spawn 落点」服务端半边实证；`/api/conversations` 清单随 cwd 作用域重解析（W2 侧 0 条）。
- 未信任目标（`D:\Windows\System32`）switch ⇒ 400 拒（fail-closed）；`leftover_orz_procs=0`。
- **留狗粮轮**：③「旧会话标旧工作区会话」标签渲染与④真实 spawn 的 UI 级重走需真会话（API 面无既有会话可比对）；`链首就绪承接`采样同轮。

## 11. 未做项

1. **未提交／未推送／未发行**（沿 091「重建先行，落账待令」）；源冻结＝worktree 态（orz 子仓树面现为 **26 改＋5 新**——新增 `child_write_guard_exec.rs` 测试与本批两轮修复），父仓门禁 `error_count 1`＝「orz submodule working tree is dirty」预期项。
2. 本地打包与 GitHub Release 未做（随发行令；发行面仍停 `v0.7.0`）。
3. 矩阵 ③④ UI 点击级重走＋`链首就绪承接`采样（§10，随狗粮轮）；索引/BACKLOG/TODO 台账随落账批（D-1 延续）。

## 12. 本批摩擦（仅记录，不另立项）

1. **protoc env**：`cargo clean --profile dev` 后 release 首跑撞 `orz-tools-api` build script 缺 `protoc`——既往档（062/068）有 `PROTOC=D:\tb-eval\.tools\protoc-25.3\bin\protoc.exe` 惯例但未入脚本/文档正面；建议下批把 PROTOC 写入 `build_orz.ps1` 或文档构建节。
2. **rustc OOM**：Windows release 与 Linux Docker 构建并行 ⇒ rustc `hir_typeck` 期 OOM；降 `-j 4` 过——双平台构建宜串行（091 即串行）。
3. **bak 两形态**：换装位 bak 链存在 `-bak`（版本化）与 `.bak-`（日期化）两种形态，首版过滤器只 coverage 点形态——两处（生成器＋运行时）修正为双形态同滤。
4. **换装位卫生**：`orz-linux/` 混 29 个构建日志（首版 manifest 32 条）；移 `buildlogs/` 子目录后 3 条。
5. **bookworm 镜像名**：`bookworm-slim` 拉取失败，正名 `debian:bookworm-slim`。
6. **pre_exec unsafe 三折**：`pre_exec` 与 `install_best_effort` 双 unsafe fn 叠加，测试侧两次补 unsafe 块后过（E0133 → E0133 → 绿）。

## 13. 关联与关键词

[`0BV 过夜批与复审修复档`](0BV_REMAINING_ITEMS_AND_S4_FIXES_2026-09-27.md) ／
[`091 重建档`](091_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-27.md) ／
[`写入管控设计`](../WRITE_CONTROL_MECHANICAL_DESIGN_2026-09-26.md) ／
[`090 落账档`](090_SUBMIT_PUSH_2026-09-27.md) ／
[`092 落账档`](092_SUBMIT_PUSH_2026-09-27.md)

关键词：0.8.0、载体重建、双平台、首个 carrier-manifest、bak 双形态排除、完整性自检活体三分、
未列文件检测、ACAF 重 provision、字面量零回退、static-pie、PT_INTERP=0、Landlock ABI=3、
exec 后真机读数、rollback 真机回环、undo 可逆、切换矩阵 API 级、信任清单全局共享、
protoc env、rustc OOM 串行、未提交未推送未发行。
