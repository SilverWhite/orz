# 0bs 检索线真网在线验证（2026-09-26，主会话轮）

> **口径**：用户令「请你进行一轮真网在线验证吧」——对 0bs c 轮（[`0BS_PROGRESS_2026-09-26c.md`](0BS_PROGRESS_2026-09-26c.md)）落码的 ⑨⑩ 三引擎/指纹/解析器与 ⑫ 垂直源做**真实网络**读数。c 轮反例自查如实标注「真网行为未验证」，本档补该面。
> **方法**：① **网络腿**＝从 [`fingerprint.rs`](../../orz/crates/codegen/orz-tools/src/implementations/web_search/fingerprint.rs) **原样提取**生产内嵌 sidecar 脚本（curl_cffi 0.16.3、`impersonate="chrome"`）直跑＋裸抓（python urllib、无浏览器指纹）A/B；引擎 URL 模板取自 `ENGINE_REGISTRY`/`VERTICAL_REGISTRY` 原文（查询词 `rust tokio`）。② **解析腿**＝真网落盘 HTML 喂给**真实 Rust 解析器**（`parse_engine_serp`／`parse_vertical_serp`，一次性集成测试跑后即删，树面零残留）。
> **边界**：agent 环内的端到端检索（工具面接线、预算、注记语义）不在本档——那属 S4 真机轮；DDG 代理腿无代理环境未测。

## 一、网络腿读数（A/B）

| 引擎 | 裸抓（无指纹） | sidecar（chrome 指纹） | 判读 |
|---|---|---|---|
| baidu | **200**／1,010,805 B | **200**／1,024,631 B | 与 09-25 勘查（裸=1,438 B 验证页）**不同**：反爬是启发式的，今日对裸 urllib 也发全量 SERP——**不可把「裸抓可达」当稳定态**，指纹优先仍有价值 |
| 360search | **FAIL**（SSL `UNEXPECTED_EOF`，TLS 握手被掐） | **200**／367,811 B | **侧车必要性实锤**：360 在 TLS 层识别并切断非浏览器指纹 |
| duckduckgo | 202／14,200 B（**挑战页**，非结果） | **exit 4**（20 s 超时） | **直连不可用维持**（勘查结论）；结果面＝0 命中，代理链才有意义 |
| github（垂直） | 200／25,936 B | 200／30,762 B | API 稳定 |
| arxiv（垂直） | **406 Not Acceptable**（默认 UA 被拒） | **200**／13,385 B（Atom） | **回落腿注意点**：arXiv 对非浏览器 UA 406——指纹 off/侧车缺失时 reqwest 回落腿或拿不到 arXiv（`local_http` 未配 UA；登记为 0bv S4 观察点） |

## 二、解析腿读数（真实 Rust 解析器 × 真网 HTML）

| 体 | 解析器 | 命中 | 首条 |
|---|---|---:|---|
| baidu.fp | `parse_engine_serp("baidu")` | **8** | 浅析 rust 大明星 Tokio - 知乎（`baidu.com/link?url=…` 跳转壳） |
| baidu.bare | 同上 | **9** | Tokio - An asynchronous Rust runtime（跳转壳） |
| 360.fp | `parse_engine_serp("360search")` | **6** | Rust —— Tokio 源神,启动! - 知乎（**`data-mdurl` 直链生效**，非跳转壳） |
| github.fp | `parse_vertical_serp("github")` | **5** | tokio-rs/tokio |
| arxiv.fp | `parse_vertical_serp("arxiv")` | **5** | （Atom 解析正常） |
| ddg.bare | —（挑战页） | **0** | 如实空 |

**百度版式漂移观察**：`data-tools` 标记今日在侧车/裸抓两体中均为 **0**（SearXNG 移植参考的选择器之一），但解析器**后备提取路径生效**（8–9 命中）——单标记漂移未致失效；命中 URL 均为百度 `/link?url=` 跳转壳，消费侧需展开（S4 读数时留意）。

## 三、结论

1. **⑨⑩ 主链真网成立**：三引擎中 360/百度直连链今日实测可达且解析命中 6–9 条；指纹侧车对 360 为**硬前提**（TLS 掐断）、对百度为**稳定态保险**（启发式反爬）。
2. **DDG 维持「代理链专属」**：直连两腿皆不可用，默认链不含 DDG 的裁决与实测一致。
3. **⑫ 垂直源真网成立**：github/arxiv 解析 5/5；arXiv 的 UA 敏感是回落腿唯一注意点。
4. **移交 0bv/S4**：DDG 代理腿读数、百度跳转壳展开读数、指纹 off 对照（`ORZ_RETRIEVAL_FINGERPRINT=off`）在真实 agent 内的 cause 如实性——随下一轮真机（已登记杂项轮 0bv）。

## 附：证据与复现

- 探针产物：`.tmp-online-probe/`（sidecar.py＝源码原样提取；各引擎 `.bare.html`／`.fp.html` 落盘体）。
- 解析腿测试文件 `orz-tools/tests/tmp_live_serp_probe.rs` 跑后即删（树面零残留；`git status` 复核）。
- 环境读数：curl_cffi **0.16.3**（与 c 轮裁决引证一致）；无代理 env（DDG 代理腿未测）。
