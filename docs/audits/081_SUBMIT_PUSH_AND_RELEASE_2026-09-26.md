# 0.7.0 提交推送与 GitHub Release（2026-09-26）

> **本批＝用户令「请进行提交与推送吧，0.7.0也发布上去」**——把 080 批已重建换装、
> 但停在树面未提交的整批改动**落账并推送**，再**双平台打包并发布 GitHub Release `v0.7.0`**。
> 提交面＝orz 两笔（落码 `abba6886` ＋ 版本冻结 `c5245558`，已推 `cli`）＋父仓一笔
> （本档＋pin＋manifest＋账本＋README 发布面）。发行面＝`v0.7.0`（首个相对已发布
> `v0.6.13` 的正式增量版；0.6.14–0.6.17 四个内部中间载体不单独发行，内容随本版一次带齐）。

## 0. 结论速览

| 项 | 读数 |
|---|---|
| orz 提交 | `abba6886`（**37 文件，+4497/−434**；0br S3 批五-八＋0bs a/b/c 三轮＋0bt 四件＋⑮⑯）＋ `c5245558`（bump 0.6.15 → **0.7.0**，两文件两行） |
| orz 推送 | `a8430054..c5245558` → `cli/feat/fusion-architecture`，exit 0；工作树 clean |
| 源清单 | `orz_source_manifest.sha256` **1498 → 1500 条**（+2＝`web_search/fingerprint.rs`、`local_browser/input_sim.rs`） |
| 父仓 | 本档＋pin → `c5245558`＋README 发布面 → **v0.7.0**＋索引 v4.55＋TODO／BACKLOG 台账 |
| 打包 | `D:\tb-eval\rel-081-stage\`：zip **28,556,923 B** `d099d9f3…`／tar.gz **37,008,587 B** `78cec631…`／顶层 `SHA256SUMS` **191 B** `f84530c9…` |
| 包完整性 | 解包回读六件与在役载体 **6/6 MATCH**；容器（`alpine:3.20`）`sha256sum -c` **zip 4/4＋tar 4/4＋顶层 2/2 全 OK** |
| 发行 | GitHub Release **`v0.7.0`**（非 draft／非 prerelease，发布于 2026-09-26T10:03:32Z）；tag `object.sha`＝本批父仓提交 `044250df…`；三资产 digest 与回下载**逐位一致**（§2.1） |
| 计数 | **不变**（**未闭合 57**）：本批为提交推送与发行，不新增／不闭合开放项 |

## 1. 提交面

### 1.1 orz 子模块（两笔）

| 提交 | 内容 |
|---|---|
| `abba6886` | `feat(0br S3 批五-八 ＋ 0bs a/b/c 三轮 ＋ 0bt 四件 ＋ ⑮⑯)`——37 文件、+4497/−434 |
| `c5245558` | `chore(release): bump version 0.6.15 -> 0.7.0（载体重建源冻结）`——`crates/orz-bin/Cargo.toml` ＋ `Cargo.lock` 两文件两行 |

落码面逐件（提交正文全文见 `git log`）：

1. **0br S3 批五-八**：归档分组规则修复（ARC 审计 journal 退出活动水位判定）／回档
   `unarchive_session_on_demand`（不动 journal 与侧车）／探索器三组重构＋信任清单只读投影
   （`TrustStore::decisions()`）／信任窗 Web 化（`POST /api/trust` → `orz trust <cwd>`）；
2. **0bs a 轮**：0bt④ 权限判定来源落账（`PermitSource` 封闭集观测字段）＋ auto mode 核验
   （会话初始 yolo；无客户端不再等审批 300 s）；
3. **0bs b 轮**：`read_file` 长行取用面（`line`／`char_offset`／`char_limit`）／`--build-info`
   载体版本旁路／宿主 shell 通道纪律（无 unix 工具改写表＋`cd /d` 纠正）／浏览器输入拟真 v1
   （新增 `input_sim.rs`：单一源常数、错字回路、WindMouse、会话 pacing）；
4. **0bs c 轮**：检索三引擎（`360search`／`baidu`／`duckduckgo`，Bing 家族出默认链保留注册）＋
   SearXNG 移植解析器＋curl_cffi 指纹 sidecar（不可用则如实回落）＋降级序（本地 HTTP 改链路中段）＋
   垂直源族（github／stackexchange／arxiv／openalex／crossref／npm）＋本地浏览器动作面放开与
   下载／脚本唯二门禁（`force_prompt`）；
5. **⑮⑯**：`REJECT_ONCE_LABEL` 与 transport 错误文案、MCP OAuth 客户端名用户层去 Grok 化。

### 1.2 验证读数（沿 c 轮与 080 批实测，未重跑）

- `local_browser` 95/0；`retention` 18/0；orz-workspace `permission` 516/0；orz-tools `web_search` 52/0；
  `cargo check` 三 crate 全 `Finished`；
- 载体面：0.7.0 双平台重建 exit 0（080 档），在役 `--build-info` 读数 `version=0.7.0`。

## 2. 打包与发行

- 暂存 `D:\tb-eval\rel-081-stage\`（本地件不入库），入口脚本
  [`.tmp-b081-package.ps1`](../../.tmp-b081-package.ps1)（沿 066／074／075／076／077 形态）；
  包 README 由 0.6.15 包 README 增量改写（新增「0.7.0 更新」节＝检索线／真机浏览器车道／
  Web 工作台收口／阅读权限身份面四组，＋首段摘要与中间载体不发行说明，**19,704 B**）。

| 资产 | 大小 (B) | SHA256 |
|---|---:|---|
| `orz-0.7.0-windows-x86_64.zip` | 28,556,923 | `d099d9f3f9aa317ef3fb228da8be896c93320e7c7033f8499a6e9e6203dbd1c6` |
| `orz-0.7.0-linux-x86_64.tar.gz` | 37,008,587 | `78cec631db5b0d3e404d2b0908e6276cfd960d6f0ee840ff21d1dc57b375693a` |
| `SHA256SUMS`（顶层） | 191 | `f84530c9f452328d183d900f51dd3ce2d1c744b160d451880489ab7def41bb95` |

- 包内容＝三件二进制＋`README.md`＋`SHA256SUMS`（`hash *name`、LF 行尾）。
- 包内六件与在役载体逐件同源同哈希（**6/6 MATCH**）：

| 件 | 大小 (B) | SHA256 |
|---|---:|---|
| `orz.exe` | 56,828,928 | `3558df7943e8166a805947617e26ff94409fc0c7807d957ecf41159f859444f8` |
| `orz-signer.exe` | 6,740,480 | `fa24e9f8cbd94c2c1bb3eab93e8a21da7fcc6e4e82a8a378df23a2103b02f81c` |
| `orz-acaf-provision.exe` | 6,640,128 | `069bba61c0b9f9715906e3063ce4c3380fa43e72bb8712fcbe4919dcc8e7c088` |
| `orz`（Linux） | 115,167,400 | `dd831719ae201c5e81af2e0d2db2a83142ad87905093dbe47591af6388c80a85` |
| `orz-signer` | 1,401,744 | `1d88ec37b63252450dd9dca3d09bef591483367a2420785461273722218c3760` |
| `orz-acaf-provision` | 1,220,496 | `0a1efffcb0c863293a94d43939601fe081ef72570258c0019c4a5cc3eba61f45` |

- 容器内 `sha256sum -c`（`alpine:3.20`）：zip 侧 4/4、tar 侧 4/4、顶层 2/2 —— **全 OK**。
- **发行口径**：`0.7.0＝正式发行`（相对已发布 `v0.6.13`）；中间载体 0.6.14／0.6.15／0.6.16／0.6.17
  **不单独发行**（沿「中间过渡版不发」先例），README 发布面对齐 **v0.7.0**。

### 2.1 上传面核证（本档提交后回读）

- **创建读数**：`gh release create v0.7.0 --target 044250df3d0a182d5aed927b0b60afa8938f80d5 …`
  ⇒ <https://github.com/SilverWhite/CLI/releases/tag/v0.7.0>；
  `gh api repos/SilverWhite/CLI/git/ref/tags/v0.7.0` 回读
  `object.sha = 044250df3d0a182d5aed927b0b60afa8938f80d5`（`type=commit`）⇒ **tag 指向本批父仓提交**；
  release 读数 `draft=false`／`prerelease=false`／`published_at = 2026-09-26T10:03:32Z`／
  `target_commitish` 与上同值。
- **流程摩擦（已解）**：首跑用短 SHA（`--target 044250df`）返回
  `HTTP 422 … Release.target_commitish is invalid`；同一提交改全 40 位 SHA 即过
  ——登记为发行操作注记（`gh release create --target` 用全 SHA）。
- **资产 digest 回读**（`gh api repos/SilverWhite/CLI/releases/tags/v0.7.0`）：

| 资产 | 远端 size | 远端 digest | 本地 size／SHA256 | 判读 |
|---|---:|---|---|---|
| `orz-0.7.0-windows-x86_64.zip` | 28,556,923 | `sha256:d099d9f3f9aa317ef3fb228da8be896c93320e7c7033f8499a6e9e6203dbd1c6` | 28,556,923／同值 | **逐位一致** |
| `orz-0.7.0-linux-x86_64.tar.gz` | 37,008,587 | `sha256:78cec631db5b0d3e404d2b0908e6276cfd960d6f0ee840ff21d1dc57b375693a` | 37,008,587／同值 | **逐位一致** |
| `SHA256SUMS`（顶层） | 191 | `sha256:f84530c9f452328d183d900f51dd3ce2d1c744b160d451880489ab7def41bb95` | 191／同值 | **逐位一致** |

- **回下载核验（认证 API 资产面完整回下载）**：三资产经
  `gh api repos/SilverWhite/CLI/releases/assets/{id} -H "Accept: application/octet-stream"`
  回读（资产 id＝`590410262`／`590410251`／`590410253`）——zip 28,556,923 B、
  tar.gz 37,008,587 B、`SHA256SUMS` 191 B，SHA256 与本地**逐位一致**。
- **私有仓口径沿 075 §8.1 不变**：匿名 `https://github.com/…/releases/download/v0.7.0/…`
  返回 404，资产面核证以**认证 API 路径**为准。

## 3. 记账面（pin、清单、索引、计数）

- orz：**`c5245558`**（`abba6886` ＋ bump；两笔已推 `cli`）。
- 父仓：本档 ＋ `orz_source_manifest.sha256` 重生成（**1500 条**，差异 2 行＝两个新文件）
  ＋ 子模块 pin → `c5245558` ＋ README 发布面 → **v0.7.0** ＋ TODO P1-0bv 与 BACKLOG 0bv
  「已提交推送＋已发行」标注 ＋ 第二卷 §1.36 本批流水 ＋ 索引头行 → **v4.55**
  （v4.54 头行滚入 `存档/index/CLI_PROJECT_INDEX_FULL_2026-09-24.md`，104 → **105 行**）。
- **计数不变**（**未闭合 57**）：本批为提交推送与发行，不新增／不闭合开放项。
- 门禁 `python scripts/check_repository.py`：提交后复跑须 `valid: true`（见 §4）。

## 4. 门禁与账目核对

- 本批落账前实测门禁两处红：① `TODO.md:32` 头部台账行超长（**1282 > 1200**）；②
  `orz submodule working tree is dirty`。处置：① 按 2026-09-25 分卷口径把 P1 路由行的逐批
  流水压回第二卷、治理行只留「当前计数＋本批一句＋指针」，并同步收缩 BACKLOG P1 总览行；
  ② 随 orz 两笔提交转 clean。
- 提交后复跑 `python scripts/check_repository.py` ⇒ **`valid: true`**（§4.1 补记读数）。

## 5. 边界与未做项

1. **S4 真机未跑**：0bs 复验五点（结束自述常驻尾行／启动器编码／lsp 全量并行／剥离计数不复发／
   压缩窗口回执）与 0bv 真网四点（DDG 代理腿／百度跳转壳展开／指纹 off 对照／arXiv 回落 UA）
   仍留下一轮真机，本批只做到「提交＋推送＋发行」。
2. **未做双平台预检**（默认口径）。
3. **回下载核验**：本仓为私有仓，匿名 `releases/download` 路径 404（075 §8.1 已定根因），
   资产面核证以认证 API `digest` 字段为准。

## 6. 本批摩擦（仅记录，不另立项）

1. **`git commit -F` 首段折行**：版本冻结笔的消息文件首段未留空行 ⇒ git 把五行折成一条
   subject；`--amend` 补空行即正。属操作面注记，非仓库缺陷。
2. **载体侧 `--release` 复用**：本批沿用 080 已构建产物直接打包（未重建），故包内哈希与
   080 表逐位一致；重建与打包解耦的形态沿用 075／077 先例。

## 7. 关联与关键词

[`080 重建档`](080_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-26.md) ／
[`0BS c 轮报告`](0BS_PROGRESS_2026-09-26c.md) ／
[`0BS 真网在线验证`](0BS_RETRIEVAL_ONLINE_PROBE_2026-09-26.md) ／
[`075 重建与发行档`](075_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-25.md) ／
[`ADR-0010`](../../adr/ADR-0010-fusion-runtime-and-agent-architecture.md)

关键词：0.7.0 发行、提交推送、orz `abba6886`／`c5245558`、manifest 1500 条、
rel-081-stage、SHA256SUMS、容器内 sha256sum -c、GitHub Release v0.7.0、
中间载体不发行、TODO 头部行分卷瘦身、门禁 valid true。
