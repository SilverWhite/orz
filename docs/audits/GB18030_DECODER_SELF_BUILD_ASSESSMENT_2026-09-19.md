# GB18030 自研解码器调查与评估（2026-09-19）

> 触发：0as 审查处置批（[`0AS_REVIEW_HANDLING_2026-09-19`](0AS_REVIEW_HANDLING_2026-09-19.md)）
> 完成全空间差分核证后，用户问「自研GB18030解码器的难度大吗？Github上有相关项目吗」。
> 本档为调查与评估留档（纯分析件：零代码、零计数、零子仓改动）。
> **用户裁决（2026-09-19）：「那就先维持现状即可」**——四类分叉维持特征化＋双侧钉死＋文档化
> 现状，不启动自研；重开条件见 §5。

## §1｜背景：分叉现状与问题定义

0as 处置批差分（1,611,796 例全空间：1 字节 256＋2 字节 23,940＋四字节全空间）证得
encoding_rs 0.8.35（生产侧）与 CPython 3.12.0 `gb18030` codec（审计参照侧）存在四类分叉：

| 类 | 规模 | 实质 |
| --- | --- | --- |
| 清洁度分叉 | 恰 1 例（裸 `0x80`） | Rust 清洁 `€` vs Python 报错落 lossy——唯一 label 分叉 |
| 二字节映射分叉 | 20 对 | GB18030-2000 PUA（`A3A0`→U+E5E5 等）vs 2005+/WHATWG 正式字符（U+3000 等），双侧均 `gb18030`、正文不同 |
| 四字节 clean-clean 分叉 | 恰 1 例（`8135F437`） | 覆盖集不同 ⇒ 线性排序偏移的镜像例 |
| 越界四字节替换粒度 | 499,604 例，模式统一 | Rust 1 个 U+FFFD vs Python 2 个——可翻转 0as 择优分支（同一输入 25.00% vs 50.00%） |

统一上述语义需任一侧自研 GB18030 解码器（encoding_rs 不暴露错误字节位置、CPython codec
内建不可配），故有本评估。

## §2｜难度评估

| 组成 | 难度 | 说明 |
| --- | --- | --- |
| 结构解析（单/二/四字节） | 平凡，约 50 行 | lead 0x81–0xFE、trail 0x40–0x7E∪0x80–0xFE、四字节 b2/b4 限 0x30–0x39 |
| 映射表 | 零搬运成本 | WHATWG 规范直接提供 [`index-gb18030.txt`](https://encoding.spec.whatwg.org/index-gb18030.txt)（约 2.4 万条二字节）＋ `index-gb18030-ranges.txt`（207 行四字节范围表）；生成静态数组即可 |
| 解码循环＋maximal-subpart 错误语义 | 低，约 150–300 行 | WHATWG Encoding Standard 为逐行伪代码，照抄即可；自研即完全掌控错误消耗步长（统一替换粒度的关键） |
| 编码器 | **不需要** | 本仓写入侧恒 UTF-8（`encode_text_no_bom`）；GB18030 编码方向（未分配码位、四字节指针逆映射）才是麻烦的一半，可整个跳过 |
| 验证 | 基本免费 | 0as 处置批的 1,611,796 例全空间差分装置＋双侧边界钉（Rust
`gb18030_table_boundaries_vs_python_reference_pinned` / Python `test_ops_executor_decode_text.py`）正好就是新解码器的验收装置 |

**结论**：解码方向纯工程量约 **1–2 人日**（细心移植＋差分验证）。真正成本不在写代码而在
**表版本的长期维护责任**：WHATWG 规范面仍在动（[whatwg/encoding#312](https://github.com/whatwg/encoding/issues/312)
跟踪 GB18030-2022 的 18 个码位变更），CPython 侧亦无已解决的 2022 更新 issue——自研后
这些演进全部转为自己的表维护工作。

## §3｜GitHub 生态调研

| 项目 | 语言/形态 | 与本问题的关系 |
| --- | --- | --- |
| [hsivonen/encoding_rs](https://github.com/hsivonen/encoding_rs) | Rust | **生产侧在用**。文档声明其 GB18030 与 GB18030-2022 一致，唯一保留 `A3 A0 → U+3000` 的 web-compat 例外——与 §1 差分实测值吻合，印证 0as 特征化命中的是已知、有意边界 |
| [whatwg/encoding](https://github.com/whatwg/encoding) | 规范＋表 | 权威规范与两张表文件，自研最佳起点；#312 跟踪 2022 变更（18 码位） |
| [encoding_rs2](https://crates.io/crates/encoding_rs2) | Rust（fork） | 已全量实施 GB18030-2022 变更（上游为 Web 兼容未采） |
| [moonbit-community/GB18030](https://github.com/moonbit-community/GB18030) | MoonBit | 完整 Unicode 映射库，表处理可参考（非 Rust） |
| [CPython Modules/cjkcodecs](https://github.com/python/cpython/tree/main/Modules/cjkcodecs) | C | 审计车道在用 codec；未跟进 GB18030-2022，未检索到已解决的更新 issue（即对齐目标侧无上游动作可搭车） |
| [golang/text](https://github.com/golang/text) simplifiedchinese | Go | 全四字节支持，实现可对照 |
| ICU / JDK（JDK-8301119）/ glibc 补丁 | C/Java/libc | 均已收编 GB18030-2022 表 |

**生态方向结论**：正式字符/2022 表是大势（ICU、Java、encoding_rs、glibc 已跟进），CPython
审计车道是旧表孤例。**Rust 生态内除 encoding_rs 外无维护良好的独立 GB18030 crate**
（老 `encoding` crate 已停更，encoding_rs2 为 fork 非独立件）——自研在 Rust 侧没有现成
小件可拼，但表文件现成、规范伪代码现成。

## §4｜方向分析（三路线）

- **路线 A：把 Rust 拉向 CPython——不建议**。等于让生产采纳旧表（PUA 而非正式字符）和
  CPython 内部的逐字节重同步语义；后者不在任何规范内、CPython 将来可变，生产面白担回归
  风险。
- **路线 B：把 Python 参照拉向 WHATWG——可行**。纯 Python 移植 WHATWG 伪代码约 200 行，
  只动审计车道（`ops_executor.decode_text` 的 GB 级），生产零改动；审计面与生产一致且走
  上现代表。代价：「冻结参照」从标准库 codec 变为自移植实现——与 Rust 同源后差分只抓移植
  bug、不再抓 encoding_rs 回归（Rust 侧钉子仍在，可兜住上游回归）。
- **路线 C：维持现状——默认推荐**。四类分叉已特征化、双侧钉死、文档化，持续成本为零；
  0as 处置批 §3 已裁决不做语义对齐，本档评估证实该裁决在工程与生态两方面均站得住。

## §5｜裁决与重开条件

**用户裁决（2026-09-19）：「那就先维持现状即可」。** 采纳路线 C，不立项、不动码。

**重开条件**（满足其一可另行立项）：

1. 审计车道与生产**逐字节一致**成为硬需求（如机械审查面需以 Python 侧解码文本作判决依据）；
2. CPython 或 encoding_rs 任一侧表版本变动，使 0as 处置批的双侧边界钉开始频繁红（即分叉
   扩大或迁移，钉子失去"钉死已知边界"的意义）；
3. WHATWG #312 落规范或 CPython 上游收编 2022 表，造成新的系统性分叉。

若重开：**推荐路线 B**（Python 侧纯 WHATWG 移植，约 200 行＋1–2 人日，0as 差分装置直接
复用为验收）。

## §6｜来源与入口

- 来源：encoding_rs GitHub／docs.rs（GB18030-2022 差异声明）、whatwg/encoding #312、
  WHATWG index-gb18030.txt／index-gb18030-ranges.txt、encoding_rs2（crates.io）、
  moonbit-community/GB18030、CPython Modules/cjkcodecs、golang/text、JDK-8301119
  （各链接见 §2/§3 行内）。
- 相关档：[`0AS_REVIEW_HANDLING_2026-09-19`](0AS_REVIEW_HANDLING_2026-09-19.md)（差分证据
  与双侧钉）／[`0AS_ENCODING_LOSSY_REFINEMENT_2026-09-19`](0AS_ENCODING_LOSSY_REFINEMENT_2026-09-19.md)
  （实施回执与 §9 勘误）／[`BACKLOG 0as`](../BACKLOG_AND_PRIORITIES.md)／索引
  `GAP-ENCODING-LOSSY-REFINEMENT`。
