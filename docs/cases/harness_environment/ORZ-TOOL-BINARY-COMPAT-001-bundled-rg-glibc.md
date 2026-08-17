# ORZ-TOOL-BINARY-COMPAT-001 — 打包工具二进制兼容 + 失败被吞（案例候选）

- **状态**：`candidate`（2026-08-17 晋级候选；未宣称产品级闭环）
- **晋级裁决**：构建/评测环境类事故具备明确复现命令、机械根因、三层处置
  （工具契约 + 构建守卫 + 归因纪律），晋级为精选案例候选（`harness_environment`
  类）。
- **来源证据**：`docs/incidents/ORZ-TOOL-BINARY-COMPAT-001.md`
- **能力**：①打包进 orz 的搜索工具二进制与任意运行容器兼容（静态链接守卫）；
  ②grep 对「搜索失败 / 零范围 / 真无匹配」做机械分型，绝不把失败吞成
  "No matches found"；③归因纪律——命令/操作错误先排查环境与机械因素，再归因
  模型或命令纪律。
- **回归入口**：
  - 构建守卫：`orz/crates/codegen/orz-tools/build.rs` 的
    `elf_is_dynamically_linked`（非 Windows 覆盖路径 PT_INTERP 校验；
    动态二进制构建即失败并提示）。
  - 工具契约：`orz/crates/codegen/orz-tools/src/implementations/grok_build/
    grep/mod.rs` 的 `finalize_grep` 结局三型 + `probe_files_searched`
    （`rg --files` 探针，同过滤集、10K 截断）。
  - 构建脚本：`D:\tb-eval\build_orz_aliyun.sh` / `build_orz.sh`（静态 musl rg
    覆盖，不再指向 `/usr/bin/rg`）。
  - 容器冒烟：任务镜像 `alexgshaw/make-doom-for-mips:20251031` 内
    `rg --version` 可运行 + 已知字符串断言匹配（vm.js `syscallNum` ≥1 命中）。
- **验证记录**：2026-08-17 容器实机复现——错误 rg（glibc 动态）加载失败
  exit 1 + 空 stdout；官方静态 musl rg 15.0.0 同命令命中 20 处（--stats:
  10 files searched）。工具测试 grep 模块 42 / types 561 / orz-loop console 68
  通过、clippy 无新增告警。Linux musl 重建后容器冒烟回归待执行。
- **复盘要点（归因纪律）**：命令或操作出现错误、或结果与已知事实明显矛盾（如
  pattern 实测存在却空结果）时，先按顺序排查环境与机械因素：①二进制/运行时兼容
  （动态链接、glibc/musl、架构）；②工具包装是否吞掉 stderr/退出码；③路径与
  作用域解析（cwd/display_cwd/根）；④沙箱/权限/ignore/隐藏语义；⑤构建打包来源
  （覆盖路径、产物架构）。确认机械层正常后，才进入模型/命令纪律归因。
- **边界**：构建守卫只覆盖非 Windows 覆盖路径；`--files` 探针仅空结果路径运行、
  命中路径 `files_searched` 留空（后续 `--json` 面或 stats 位置收敛后再定）；
  list_dir ignored/truncated 计数未实施（FUS-TOOL-SCOPE-CONTRACT 后续项）；
  Linux 重建后的容器冒烟正式回归未执行。
