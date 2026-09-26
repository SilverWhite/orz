# 091 载体重建与换装（双平台 · 0.7.2 → 0.7.3）

> **日期**：2026-09-27；**用户令**：「请进行重建吧」（沿 089 先例：重建先行，落账待令）。
> **进体内容**＝0bv 结转批（run `RUN-CLI-6ab7f32f`：REV-083-07／08／10／18a–h 全落＋01 代码面＋09 告警面＋裁决 D-1…D-20，见 [`0BV_CARRYOVER 报告`](0BV_CARRYOVER_REV083_0BW_2026-09-27.md)）＋文档落字批（REV-01/03/04/06/13/15/20 落字＋approval.rs 头注释处置，见同报告 §7）＋版本 bump 两文件两行。**本批即上述树面的首个进体载体**。
> **源冻结＝worktree 态**（orz 子模块 HEAD `71af0ee3` 未动 ⇒ `orz_source_manifest.sha256` 按设计不动）；**未提交／未推送／未发行**（发行面仍停 `v0.7.0`）。

## 1. 版本 bump（两文件两行）

- `orz/crates/orz-bin/Cargo.toml:3` 0.7.2 → **0.7.3**；`orz/Cargo.lock:5598` orz-bin 条目同步。`cargo metadata --locked` **exit 0**。

## 2. Windows 增量重建与换装

- 前置：D: 余 5.9G ⇒ `cargo clean --profile dev` 腾挪 **17.1 GiB**（→ 22.2G；连续第六批同族操作，已惯例化）。
- 构建：`cargo build --release` exit 0，**1m49s**（`Compiling orz-bin v0.7.3`）；代码警告 **1 条**＝`unused imports: Path, PathBuf`（`orz-host/src/permission.rs:22`，18g RETIRED 镜像段转 `#[cfg(test)]` 后非测试臂遗留）——观察面，随落账批或下轮清零。
- 换装：构建产物三件 → `D:\tb-eval\orz-windows\`，**MATCH 3/3**（旧三件留 `.0.7.2-bak` 链）；`--build-info` 换装位读数 **`version=0.7.3 os=windows arch=x86_64 profile=release`**。

| 文件 | 尺寸 (B) | 与 0.7.2 差异 | SHA256（前 16；旧值全部不同） |
|---|---:|---:|---|
| `orz.exe` | 56,999,424 | **−24,064 B** | `c47a5a155085a1ce` |
| `orz-signer.exe` | 6,740,480 | 0 | `377d26a0e095212f` |
| `orz-acaf-provision.exe` | 6,640,128 | 0 | `1440727b89e77ce9` |

## 3. ACAF 重 provision

- 旧 manifest 留 `signer-manifest.json.bak-20260927-091`；provision **exit 0**；keystore 两件哈希前后逐位未动（`f37556ab653c`／`2aa80cb8d618`）；manifest 回显 `binary_sha256=377d26a0e095212f…` ↔ 换装位 signer 逐位一致。

## 4. 字面量核证（Windows 二进制；对照＝0.7.2-bak）

| 字节模式 | 0.7.2 | 0.7.3 | 判定 |
|---|---:|---:|---|
| `[permission] mode=` ／ `interactive-yolo` | 0／0 | **2／1** | REV-01 权限横幅进体 ✅ |
| `predates signer restart cutoff` | 0 | **12** | REV-08 respawn cutoff 拒绝 detail 进体 ✅ |
| `frame-ancestors` ／ `base-uri` | 0／0 | **1／1** | 18f orz-web CSP 进体 ✅ |
| `orz-build-info: version=` | 1 | 1 | 保持面 ✅ |
| `[写入管控·提示]` ／ `protected by the write control` | 1／2 | 1／2 | 0bw 保持面 ✅ |
| `No, and tell the AI what to do differently` | 1 | 1 | ⑮ 保持面 ✅ |
| `No, and tell Grok what to do differently` | 0 | 0 | 零回退 ✅ |
| `GrokBuild:` ／ `GROK_HOME` | 26／7 | 26／7 | 边界保留 ✅ |
| `360search` ／ `ORZ_RETRIEVAL_FINGERPRINT` | 3／1 | 3／1 | 保持面 ✅ |
| `candidate lines:` ／ `Formatting notice:` ／ `/api/workspace/switch` ／ `旧工作区会话（回看）` | 4／2／3／1 | 4／2／3／1 | 0.7.2 面零回退 ✅ |
| `user_confirmation_sha256` | 0 | 0 | REV-03 口径自洽（二进制本无人审摘要）✅ |

- 注：REV-10（journal writer 移 OS 线程）／18b（锁中毒）／18c（水位缓存）／18d（心跳 stamp）为行为面无新字面量，测试面读数见 [`0BV_CARRYOVER 报告 §4`](0BV_CARRYOVER_REV083_0BW_2026-09-27.md)（assurance 277/0、loop-acaf 14/0）。

## 5. 载体级 Web 探针（`.tmp-b091-win-web-probe.ps1`，端口 21561，沿 087/089 脚本）

- 全过：`/` 200／无令牌 **401**／伪 Host **401**／带令牌 **200**；`boot_keys = cwd,agent,trusted_workspaces`；归档 **29 件**；`leftover_orz_procs=0`。启动横幅 GBK 呈现为既有已知观察面（探针捕获路径，0bs ② 同族）。

## 6. Linux musl 三件套（Docker，直写换装位）

- `rust:1.97-slim` ＋ `build_orz_aliyun_trixie.sh`（ORZ-BUILD-MOUNT-001 契约＋`MSYS_NO_PATHCONV=1`）**exit 0**；换装位旧三件预置 `.0.7.2-bak` 链后直写。

| 文件 | 尺寸 (B) | 与 0.7.2 差异 | SHA256（前 16） | BuildID |
|---|---:|---:|---|---|
| `orz` | 115,376,312 | **−49,072 B** | `627dfa8ef5fb4bac` | `53ac1257` |
| `orz-signer` | 1,401,360 | −448 B | `bc9e5f877426e6e4` | `a704f3b4` |
| `orz-acaf-provision` | 1,219,968 | −560 B | `fa80022d6a11f0f4` | `f7fb34ff` |

- ELF 静态核验（alpine 3.20 ＋ binutils/file）：三件 `static-pie linked`＋**`PT_INTERP=0`**。
- 双向加载冒烟：bookworm-slim／alpine 3.20 `orz --build-info` **exit 0** 且读数 `version=0.7.3`；signer／provision 裸调用 exit 1（usage 形态＝可加载可执行）。
- 警告面读数**未取到**（本批起跑命令尾接 `tail` 截断了编译日志；exit 0＋盘上三件为硬证据）——摩擦见 §8。

## 7. 未做项

1. **未提交／未推送**：0bv 结转批＋文档落字批＋本批 bump 同停树面（用户令「请进行重建吧」＝重建先行，落账待令）；源冻结＝worktree 态，源清单按设计不动。
2. **本地打包与发行未做**（随发行令；发行面仍停 `v0.7.0`）；**S4 真机复验未跑**（0bv 验证点：浏览器 SERP 链首真网读数／切换矩阵四点／0bw 锁死面拦截·L2 留痕·CFA 1124 读数，随 0bv 轮）。
3. **预检未做**（默认口径）；索引/BACKLOG/TODO 台账未动（D-1 延续）。

## 8. 本批摩擦（仅记录，不另立项）

1. **D: 余量回弹**（5.9G）⇒ `cargo clean --profile dev` 腾挪 17.1 GiB 后 22.2G（连续第六批同族操作）。
2. **构建日志截断**：Linux 腿起跑命令尾接 `tail -30` 致编译期 `Finished`／警告数读数未落档——下批起跑改为全量重定向。
3. **Windows 腿警告 1 条**（`permission.rs:22` unused imports，18g 遗留）——随落账批或下轮清零；非本批引入。

## 9. 关联与关键词

[`0BV_CARRYOVER_REV083_0BW_2026-09-27`](0BV_CARRYOVER_REV083_0BW_2026-09-27.md) ／
[`089 重建档`](089_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-27.md) ／
[`090 落账档`](090_SUBMIT_PUSH_2026-09-27.md)

关键词：0.7.3、载体重建、双平台、源冻结 worktree 态、bump 两文件两行、REV-083 进体、
权限横幅、respawn cutoff、CSP、字面量核证零回退、ACAF 重 provision、static-pie、
PT_INTERP=0、未提交未推送未发行。
