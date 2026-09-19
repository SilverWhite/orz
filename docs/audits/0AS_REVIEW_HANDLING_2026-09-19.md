# 0as 审查处置批回执（2026-09-19）

> 触发：主会话对 0as 实施批（[`0AS_ENCODING_LOSSY_REFINEMENT_2026-09-19`](0AS_ENCODING_LOSSY_REFINEMENT_2026-09-19.md)）
> 的全面审查（设计合理性／实现合理性／设计-实现符合性）。总体结论：**通过**；本批处置审查
> 检出的全部建议级与注记级问题（P2×2、P3×2）。**零计数**（0as 维持开放、判据入账留提交批）、
> 工作树未提交、未推送、载体重建随用户放行。

## §1｜问题 → 处置对照

| 审查发现 | 级别 | 处置 | 结果 |
| --- | --- | --- | --- |
| Python 冻结参照 `decode_text` 零测试覆盖，"Rust/Python audits agree"可静默漂移 | P2 | 新增常驻测试 [`assurance/tests/test_ops_executor_decode_text.py`](../../assurance/tests/test_ops_executor_decode_text.py) | 9/9 绿（含 CI 同款 discovery 收集验证） |
| 双实现无差分核证，encoding_rs 与 CPython gb18030 表版本偏斜未核 | P2 | 全空间一次性差分核证＋四类分叉双侧钉死＋双侧文档声明范围（§3） | 差分完成（§3）；Rust 26/26、Python 9/9 绿 |
| 实施回执 §4 读数"2889+2"与总数 2897 疑不自洽 | P3 | 全量复跑取真数＋回执 §9 勘误（§4） | **勘误反转：原读数自洽**（漏记 6 ignored）；审查发现本身更正 |
| 实施回执 §2"gb18030 errors='replace' 与 encoding_rs 同粒度"表述 | P3 | 回执 §9 勘误：限定六案范围成立、一般性被差分证伪 | 越界四字节替换粒度 1 vs 2 FFFD（§3） |
| 注记级边界（F4 会话级 CP 残留／占位同形碰撞／比例分母含换行／平手梯序） | 注记 | 维持原登记不另处置，汇总入 §5 | — |

## §2｜P2-1：Python 冻结参照常驻测试钉

新文件 `assurance/tests/test_ops_executor_decode_text.py`（unittest 风格，`python -m
unittest discover -s assurance/tests` 可收集；pytest 同样兼容）：

- **六案 golden**：与 Rust 钉逐字节同期望（mixed／gb 胜出／平手梯序／多字节占位／BOM 分母／
  截断四字节）；
- **严格级回归**（判据②）：纯 UTF-8／空输入／BOM／纯 GB18030；
- **结构不变式**：比例自标签复算（Δ<0.005）、降级单元数 ≤ 整段 `from_utf8_lossy` 基线、
  lossy 级单元数 ≥ 1；
- **表边界钉**（§3 差分的三个代表例，Rust 侧镜像钉
  `gb18030_table_boundaries_vs_python_reference_pinned` 对应）：裸 0x80、A3A0、越界四字节。

读数：`9 passed`（2026-09-19 实测；CI 同款 `discover -p` 收集通过）；`compileall` 通过。

## §3｜P2-2：GB18030 全空间 Rust/Python 差分核证（一次性证据）

**方法**：仓外 scratch 工程（`encoding_rs = "=0.8.35"`，与 `orz/Cargo.lock` 同版）离线构建
dumper，遍历**全字节序列空间**：1 字节 256＋2 字节 23,940（lead 0x81–0xFE × trail
0x40–0x7E∪0x80–0xFE）＋四字节 1,587,600（全空间，合法与非法），共 **1,611,796 例**；
encoding_rs `GB18030.decode_without_bom_handling` 结果落二进制（字节＋had_errors＋输出
UTF-8），Python 3.12.0 `decode('gb18030')` 逐例比对**清洁度标志＋全文输出**。Rust 侧
4.1 s，Python 侧 2.4 s。

**结果（四类分叉，全部特征完备）**：

| 类 | 规模 | 内容 |
| --- | --- | --- |
| 清洁度分叉 | **恰 1 例**：裸 `0x80` | encoding_rs 清洁 → `€`（GB18030 欧元位）；CPython 报错 → 落 lossy。**唯一 label 分叉**（`gb18030` vs `utf-8-lossy:100.00%`） |
| 二字节映射分叉 | **恰 20 对** | GB18030-2000 PUA vs 2005+/WHATWG 正式字符：`A3A0`→U+E5E5/U+3000、`A6D9–A6DF`→U+E78D..92/U+FE10..15、`A6EC/A6ED/A6F3`→U+E794..96/U+FE17..19、`A8BC`→U+E7C7/U+1E3F、`FE59/FE61/FE66/FE67/FE6D/FE7E/FE90/FEA0`→U+E81E..E864/U+9FB4..9FBB。双侧均 `gb18030`，正文不同 |
| 四字节 clean-clean 分叉 | **恰 1 例**：`8135F437` | 上类的镜像（二字节覆盖集不同 ⇒ 线性排序偏移）：Rust→U+E7C7、Python→U+1E3F |
| 越界四字节（双侧均脏） | **499,604 例，模式统一** | encoding_rs 整序列替换 **1 个 U+FFFD**；CPython **2 个**（`FFFD '1' FFFD` 形）。**直接影响 0as 择优分支**：`[84 31 A5 30]` 一行 Rust GB 胜（1 单元，25.00%）、Python 平手梯序 UTF-8 胜（2 单元，50.00%）——输出与标签皆可分叉 |

其余全空间（含全部合法二字节/四字节映射、全部双侧皆脏的二字节形态）清洁度、映射、FFFD
粒度**逐例全等**。六案 golden 不在分叉集内，实施回执"六案逐字节对齐"声明维持为真。

**处置裁决**：**不做语义对齐**——统一替换粒度需任一侧自研 GB18030 解码器（encoding_rs 不
暴露错误字节位置、CPython codec 内建），比例失调；PUA↔正式字符属表版本选择（2000 vs
2005+），两侧各自自洽。改为：①四类分叉双侧钉死（任何 codec/依赖升版移动边界即被机械抓
住）；②双侧文档声明范围（`encoding.rs` module 头＋`ops_executor.py::decode_text`
docstring）；③差分配方与证据入本档，encoding_rs 升版时按 §3 配方重跑。

**dumper 配方**（复现用；scratch 工程已清理）：

```rust
// corpus: 1-byte 0x00..=0xFF; 2-byte lead 0x81..=0xFE × trail 0x40..=0x7E ∪ 0x80..=0xFE;
//         4-byte 全空间 b1/b3 ∈ 0x81..=0xFE, b2/b4 ∈ 0x30..=0x39（共 1,611,796 例）
for c in &cases {
    let (text, had_errors) = encoding_rs::GB18030.decode_without_bom_handling(c);
    // 落盘：<len:u32 LE><bytes><had_errors:u8><outlen:u32 LE><utf-8 out>
}
// Python 侧逐例：strict decode 干净与否 ↔ had_errors；同清洁度比全文；
// 另记 U+FFFD 计数（喂 0as 择优分支的 gb_units 口径）
```

## §4｜P3：实施回执勘误（原文不动，勘误随批）

1. **§4 读数补全（审查发现本身更正）**：原记"orz-tools lib 2889 passed＋2 failed"漏记
   **6 ignored**——2889+2+6=2897 与总数自洽，**非算术不自洽**；审查报告"疑不自洽"系漏数
   ignored，在此更正。主会话全量复核实读（本处置批，含新钉 1 条）：**2891 passed＋1 failed
   ＋6 ignored＝2898 总数**；1 failed＝`lsp::tests::e2e_restart_monitor_emits_failed_on_
   restart_init_error`（负载敏感，单测串行复跑 1 passed——RS-08 同族既有形态）。
2. **§2"gb18030 errors='replace' 与 encoding_rs 同粒度"**：限定**六案范围**成立；一般性被
   §3 差分证伪（越界四字节 1 vs 2 U+FFFD）。双侧边界钉与文档声明随本批落地。

## §5｜注记级边界汇总（维持登记，不另处置）

- **F4 会话级 CP 残留**：`SetConsoleOutputCP(65001)` 对同一控制台会话有会话级残留（后续
  程序随 CP 输出 UTF-8 至控制台关闭）；per-console、不持久化、best-effort，回执 §5 已载，
  接受。输入 CP（`SetConsoleCP`）不在 F4 范围。
- **占位同形碰撞**：正文合法含有 `⟨0x8F⟩` 同形文本时不可区分——低概率、形态已定稿入账。
- **比例分母含换行字节**：`<p>` 分母＝BOM 剥离后全部字节（含 `\n`）——已文档化、机械可复
  算（双侧不变式钉锁住）。
- **平手梯序例**：`[d6 d0 8f]` → `⟨0xD6⟩`＋错配合法对（U+040F）——回执 §6 已登记，S3/狗
  粮读数可感时再提占优度加权裁决，本批不翻案。

## §6｜随批改动清单与读数

子仓（未提交）：`crates/codegen/orz-tools/src/util/encoding.rs`（module 头范围声明＋新增
钉 `gb18030_table_boundaries_vs_python_reference_pinned` 3 断言）。其余 0as/在场批文件零
触碰（本批 hunk 与 0am/0ar S2/RS-14 逐不相交）。

父仓（未提交）：`assurance/tests/test_ops_executor_decode_text.py`（新，9 钉）、
`assurance/ops_executor.py`（docstring 范围声明）、本档、实施回执（§9 勘误）、
`TODO.md`／`docs/BACKLOG_AND_PRIORITIES.md`／`CLI_PROJECT_INDEX.md`（账本随批）。

读数（2026-09-19 实测）：orz-tools lib **2891 passed＋1 failed（LSP e2e 负载敏感，串行绿）
＋6 ignored＝2898**（=2897 基线＋1 新钉）；Python 新钉 **9/9**；`compileall` 0；
`PROTOC`/`CARGO_INCREMENTAL` 环境注记沿实施回执 §4。计数 **37 不变**（0as 闭合留提交批）。
