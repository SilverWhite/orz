# 100 载体重建与换装（双平台 · 0.8.0 → **0.8.1**）

> **日期**：2026-09-27；**用户令**：「请先进行重建吧，然后再提交并推送，双平台安装包也进行发布」（沿 091/093 先例：重建先行，落账待令）。
> **进体内容**＝099 批 0bx 修复（`local_browser/cdp.rs` 就绪判定端点兜底＋关停位＋三枚钉子）＋`rollback_maintenance.rs` 测试告警清理＋版本 bump 两文件两行。**本批即 0bx 的首个含修复载体**。
> **源冻结＝worktree 态**（orz 子模块 HEAD `d0e29615` 未动 ⇒ `orz_source_manifest.sha256` 按设计不动，`--check` 读数 `valid`）；**未提交／未推送／未发行**（发行面仍停 `v0.8.0`）。

## §1 版本 bump（两文件两行）

- `orz/crates/orz-bin/Cargo.toml` 0.8.0 → **0.8.1**；`orz/Cargo.lock:5598` orz-bin 条目同步（锁内另有第三方 `0.8.1`／`0.8.12` 巧合同号，未动）。`cargo metadata --locked` **exit 0**。

## §2 Windows 增量重建与换装

- **腾挪**：构建前 D: 余 8.8 G ⇒ `cargo clean --profile dev` 释放 **27.8 GiB / 37,800 文件**（→ 34.6 G；连续第八批同族操作，已惯例化）。
- **构建**：`scripts/build_orz.ps1 -Release -Jobs 4` **exit 0，5m12s**（装配清单：宿主机 12 核／commit 使用 90.5%·余 2.6 GiB ⇒ `-Jobs 4` 显式；`PROTOC` 由脚本自动装配）。警告 **1 条**＝`permission.rs:22` unused imports（18g 既有遗留，非本批引入）。
- **换装**：三件 → `D:\tb-eval\orz-windows\`，**MATCH 3/3**（旧三件留 `.0.8.0-bak` 链）；`--build-info` 换装位读数 **`version=0.8.1 os=windows arch=x86_64 profile=release`**（exit 0）。

| 文件 | 尺寸 (B) | 与 0.8.0 差异 | SHA256（前 16） |
|---|---:|---:|---|
| `orz.exe` | 57,018,880 | **−74,240 B** | `898820d6caa5c7f7` |
| `orz-signer.exe` | 6,740,480 | 0（哈希变＝内嵌版本号） | `62d91d4721cea389` |
| `orz-acaf-provision.exe` | 6,640,128 | 0（同上） | `5647bf6e5e27db5c` |

## §3 ACAF 重 provision

- 旧 manifest 留 `signer-manifest.json.bak-20260927-100`；`orz-acaf-provision <keystore> <manifest>` **exit 0**；manifest 回显 `binary_sha256=62d91d4721cea389…` ↔ 换装位 `orz-signer.exe` **逐位一致**；keystore 两件哈希前后**逐位未动**（`f37556ab…`／`2aa80cb8…`）。

## §4 字面量核证（Windows，出现次数法；对照＝`orz.exe.0.8.0-bak`）

| 字节模式 | 0.8.0 | 0.8.1 | 判定 |
|---|---:|---:|---|
| `[carrier-integrity]`／`未列文件（清单外新增）`／`清单键非法` | 2／1／1 | 2／1／1 | 0bw② 保持 ✅ |
| `已回退`／`拒绝以 .gsa 域为回退目标` | 1／1 | 1／1 | 0bw④ 保持 ✅ |
| `write_control_review` | 2 | 2 | 0bw③ 保持 ✅ |
| `[写入管控·提示]`／`protected by the write control` | 1／2 | 1／2 | 0bw L1/L2 保持 ✅ |
| `workspace/switch`／`360search` | 3／3 | 3／3 | 0.7.x 面零回退 ✅ |
| `GrokBuild:`／`GROK_HOME` | 26／7 | 26／7 | 边界保留 ✅ |
| `orz-build-info: version=` | 1 | 1 | 版本字面量（读数由 `--build-info` 核） |

- **回退计数＝0**。**注**：0bx 修复是纯判定逻辑（无模型面字面量）⇒ 本版进体判据＝版本字面量＋源冻结一致性＋行为读数（S4，见 §8）。

## §5 载体级 Web 探针（`.tmp-b100-win-web-probe.ps1`，端口 21581，沿 087/089/091/093 脚本）

- 全过：`/` **200**／boot 无令牌 **401**／伪 Host **401**／带令牌 **200**；`boot_keys = cwd,agent,trusted_workspaces`；`archives_count = 29`（换装位工作区 `.gsa` 既有归档数）；**`leftover_orz_procs = 0`**。

## §6 Linux musl 三件套（Docker，直写换装位）

- `rust:1.97-slim` ＋ `scripts/build_orz_aliyun_trixie.sh`（ORZ-BUILD-MOUNT-001 契约；全量日志重定向）**exit 0**（约 15 min）；换装位旧三件预置 `.0.8.0-bak` 链后直写，产物与 `orz-target/x86_64-unknown-linux-musl/release` **MATCH 3/3**。

| 文件 | 尺寸 (B) | 与 0.8.0 差异 | SHA256（前 16） | BuildID |
|---|---:|---:|---|---|
| `orz` | 115,455,104 | **−69,008 B** | `25e0483fc412ec16` | `3f079f71` |
| `orz-signer` | 1,401,496 | −104 B | `cbaa4eed1a2c03c5` | `70bc58c9` |
| `orz-acaf-provision` | 1,220,128 | −80 B | `40fa9f65e054f972` | `3f3fa6de` |

- **ELF 核验**（alpine 3.20 ＋ `file`／`binutils`）：三件均 **`static-pie linked`**＋**`INTERP 段 = 0`**。
- **双向加载冒烟**：alpine 3.20 与 `debian:bookworm-slim` 内 `orz --build-info` **exit 0** 且读数 **`version=0.8.1 os=linux`**；`orz-signer`（manifest 缺席 fatal 形态）／`orz-acaf-provision`（usage 形态）均可加载可执行。
- **警告面**：`orz-assurance` 4／`orz-config` 1／`orz-host` 3（均为 Linux-only cfg 既有面：`unused import: std::sync::Mutex`／`WindowsDpapiInstallationKeyStore`／`PathBuf and Path`）——**`cdp.rs`（本批触碰面）零告警**。

## §7 载体清单重生成（0bw② 机制）

- `python scripts/generate_carrier_manifest.py D:/tb-eval/orz-windows --version 0.8.1` ⇒ **3 entries**；同命令对 `orz-linux` ⇒ **3 entries**；两侧 `generated_at` 2026-09-27T14:53:18Z／14:53:20Z。清单自身与 `.bak`／`-bak` 链按既定排除集不入册（0.8.0 批定的双形态过滤照旧）。
- 说明：本批**仅重生成**清单（换装位三件为改后二进制）；把清单**打进发布包**的裁决随发行批实施（095 档登记的待裁项＝0.8.1 包内纳入 `carrier-manifest.json`）。

## §8 未做与边界

1. **S4（agent 级真机承接读数）未跑**——0bx 的闭合判据是「`web_search` 链首真机承接（不再让渡）」，须经检索子代理真机轮；本批只到「修复进体＋真机钉绿」，故 0bx 维持 **`partial`**（计数 55 不变）。
2. **未提交／未推送／未发行**（随用户令后续批）；父仓门禁预期 `error_count 1`＝「orz submodule working tree is dirty」。
3. 双平台预检未做（默认口径，沿 087/089/091/093）。
4. Linux 载体**未做** ACAF 重 provision（打包/发行链沿用既有 Linux 侧 manifest 约定；本批只重签 Windows 换装位——与 093 同口径）。

## §9 本批摩擦（仅记录，不另立项）

1. **PowerShell 5.1 ＋ 中文脚本字面量**：换装脚本内一处中文提示串导致 `powershell -File` **解析报错**（`Unexpected token`／`string missing terminator`，两轮复现），改为 ASCII 串后即过。⇒ 惯例：批处理脚本正文用 ASCII，中文只进日志/文档（或改用 `pwsh`）。
2. **`orz --help` 挂起**：直接调 `orz.exe --help` 未打印用法且进程驻留（10 s+ 未退），人工 `Stop-Process` 收尾（`leftover_orz_procs=0`）。判读＝`--help` 非受支持入口（`orz` 缺省进交互形态等待输入）；**仅记录**，不作缺陷主张。
3. **磁盘**：起始余量 8.8 G（较 093 后 35 G 明显缩水，源于本日多轮构建）⇒ 按惯例清 dev 缓存腾挪；构建期未再触发 os error 112。
4. `protoc` 已由 `scripts/build_orz.ps1` 自动装配（093 摩擦 1 的收口已生效，本轮零手工干预）。

## §10 关联与关键词

[`099 就绪判定档`](099_BROWSER_READY_HANDOFF_2026-09-27.md)／[`093 重建档`](093_CARRIER_REBUILD_V080_DUAL_PLATFORM_2026-09-27.md)／[`091 重建档`](091_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-27.md)／[`写入管控设计`](../WRITE_CONTROL_MECHANICAL_DESIGN_2026-09-26.md)／`orz-host/src/local_browser/cdp.rs`。

关键词：0.8.1、载体重建、双平台、0bx 进体、就绪判定修复、ACAF 重 provision、字面量零回退、static-pie、
INTERP 段 0、carrier-manifest 3 entries、Web 探针 401/401/200、cargo clean dev 腾挪、PS 5.1 中文脚本摩擦、
orz --help 挂起观察、S4 待跑、未提交未推送未发行。
