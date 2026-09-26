# 089 载体重建与换装（双平台 · 0.7.1 → 0.7.2）

> **日期**：2026-09-26 深夜起跑（Windows 腿）/ 2026-09-27 收尾（Linux 腿 00:14 完成）；**用户令**：「请进行重建吧」（重建先行，落账待令）。
> **进体内容**＝0bv 五件（① 浏览器 SERP 链首接缝＋预算单账本并账／② 工作区切换 B 形态全栈／③ f13 候选行号／④ f14 版式剧变提示／⑤ f15 压缩骨架单一源＋回执接线，轮 `RUN-CLI-6ab7d8b7`）＋F4 预置红修复（`projection.rs` 测试期望同步，**测试面不进二进制**）＋fmt 收编（`agent_loop.rs`／`host_exec/serp.rs`）＋版本 bump 两文件两行。**源冻结＝worktree 态**（orz 子模块 HEAD `42a14d16` 未动 ⇒ 源清单按设计不动）；**未提交／未推送／未发行**。

## 1. 版本 bump（两文件两行）

- `orz/crates/orz-bin/Cargo.toml:3` 0.7.1 → **0.7.2**；`orz/Cargo.lock:5598` orz-bin 条目同步。`cargo metadata --locked` **exit 0**。（锁内另四处 `0.7.2`＝async-broadcast／gix-chunk／gix-quote／tracing-chrome 第三方巧合同号，未动。）

## 2. Windows 增量重建与换装

- 前置：D: 余 5.6G（98%）⇒ `cargo clean --profile dev` 腾挪 **19.8 GiB**（→ 25G；连续第五批同族操作，已惯例化）。
- 构建：`cargo build --release` exit 0，**2m52s**（`Compiling orz-bin v0.7.2`）。
- 换装：构建产物三件 → `D:\tb-eval\orz-windows\`，**MATCH 3/3**（旧三件留 `.0.7.1-bak` 链）；`--build-info` 换装位读数 **`version=0.7.2 os=windows arch=x86_64 profile=release`**。

| 文件 | 尺寸 (B) | 与 0.7.1 差异 | SHA256（前 16） |
|---|---:|---:|---|
| `orz.exe` | 57,023,488 | **+135,168 B** | `3e931fe16dff5ccf` |
| `orz-signer.exe` | 6,740,480 | 0 | `8ef1a82d28b2033e` |
| `orz-acaf-provision.exe` | 6,640,128 | 0 | `b701a64b7fcb2dd3` |

- 注：signer／provision 尺寸与 0.7.1 逐位同、**哈希已变**（0.7.1 的 `2f76b0ce…`／`4beed214…`）——0bv 轮改了 orz-host／orz-web 等 orz-bin 依赖面，整 crate 重建重链接所致，属预期形态。

## 3. ACAF 重 provision

- 旧 manifest 留 `signer-manifest.json.bak-20260926-089`；provision **exit 0**；keystore 两件哈希前后逐位未动（`f37556ab653c`／`2aa80cb8d618`）；回显 `binary_sha256=8ef1a82d28b2033e…` ↔ 换装位 signer 逐位一致。

## 4. 字面量核证（Windows 二进制；对照＝0.7.1）

| 字节模式 | 0.7.1 | 0.7.2 | 判定 |
|---|---:|---:|---|
| `[browser_serp] fell back to local_http` | 0 | **4** | ① 链首让渡注记进体 ✅ |
| `browser_unavailable` | 1 | **2** | ① 让渡 cause 进体 ✅ |
| `candidate lines:` | 0 | **4** | ③ f13 候选行号进体 ✅ |
| `Formatting notice:` | 0 | **2** | ④ f14 版式提示进体 ✅ |
| `/api/workspace/switch` | 0 | **3** | ② 切换路由（服务端＋前端）进体 ✅ |
| `旧工作区会话（回看）` | 0 | **1** | ② 旧会话提示进体 ✅ |
| `（可选：不给则由机械层按最旧闭合块优先）` | 1 | 1 | ⑤ f15 骨架文案 **HEAD 已有**（`context_scale.rs` HEAD `:295`）——f15＝单一源重构＋回执接线，字节面保持 ✅ |
| `orz-build-info: version=` | 1 | 1 | 保持面 ✅ |
| `[写入管控·提示]` ／ `protected by the write control` | 1／2 | 1／2 | 保持面 ✅ |
| `No, and tell the AI what to do differently` | 1 | 1 | ⑮ 保持面 ✅ |
| `No, and tell Grok what to do differently` | 0 | 0 | 零回退 ✅ |
| `GrokBuild:` ／ `GROK_HOME` | 26／7 | 26／7 | 边界保留 ✅ |
| `360search` ／ `ORZ_RETRIEVAL_FINGERPRINT` | 3／1 | 3／1 | 保持面 ✅ |

## 5. 载体级 Web 探针（`.tmp-b089-win-web-probe.ps1`，端口 21560，沿 087 脚本）

- 全过：`/` 200／无令牌 **401**／伪 Host **401**／带令牌 **200**；`boot_keys = cwd,agent,trusted_workspaces`；归档 **29 件**（0.7.1 读数 28 → +1＝0bv 轮会话归档 `6ab7d8b7.json.gz`）；`leftover_orz_procs=0`。启动横幅 GBK 呈现为既有已知观察面（探针捕获路径，0bs ② 同族；本族覆盖面扩张已并件 0bv 并件三）。

## 6. Linux musl 三件套（Docker，直写换装位）

- `rust:1.97-slim` ＋ `build_orz_aliyun_trixie.sh`（ORZ-BUILD-MOUNT-001 契约＋嵌套 runtime 绑定＋`MSYS_NO_PATHCONV=1`）**exit 0**，**10m56s**；换装位旧三件预置 `.0.7.1-bak` 链后直写。

| 文件 | 尺寸 (B) | 与 0.7.1 差异 |
|---|---:|---:|
| `orz` | 115,425,384 | **+196,744 B** |
| `orz-signer` | 1,401,808 | +120 B |
| `orz-acaf-provision` | 1,220,528 | +88 B |

- SHA256（前 16）：`da21c88b53d93ec4`／`8c099782e54b7fc7`／`69a0338057170864`。
- ELF 静态核验（alpine 3.20 ＋ binutils/file）：三件 `static-pie linked`＋**`PT_INTERP=0`**（BuildID `21692ca5`／`c55dea1a`／`695916bf`）。
- 双向加载冒烟：bookworm-slim／alpine 3.20 **6/6 exit 1**（预期形态）。
- 警告面：代码警告 **7 条与基线同数**（`WindowsDpapiInstallationKeyStore`／`Mutex` unused import、`cmd`／`path`／`pid`×3 unused——非 Windows 分支同族）＋2 条工具链噪音（rustup override 注记／rg PATH 提示）。

## 7. 未做项

1. **未提交／未推送**：0bv 五件＋F4 修复＋fmt 收编＋本批 bump 同停树面（用户令「请进行重建吧」＝重建先行，落账待令）；源冻结＝worktree 态，`orz_source_manifest.sha256` 按设计不动（HEAD 未动）。
2. **本地打包与发行未做**（随发行令；发行面仍停 v0.7.0）；**S4 真机复验未跑**（0bv 验证点：浏览器 SERP 链首真网读数／切换矩阵四点／0bw 锁死面拦截·L2 留痕·CFA 1124 读数，随 0bv 轮）。
3. **预检未做**（默认口径）。

## 8. 本批摩擦（仅记录，不另立项）

1. **D: 余量回弹**（狗粮轮＋测试产物 → 5.6G，较 087 批的 12G 更紧）⇒ `cargo clean --profile dev` 腾挪 19.8 GiB 后 25G（连续第五批同族操作，已惯例化）。
2. 无新增（`MSYS_NO_PATHCONV=1`／alpine apk 现装等按 080/087 配方注记预先规避）。
