# 370 断连根因复现件 —— exec_policy.rs:974 panic（2026-10-27）

## 结论（一行）

`orz-tools/src/types/exec_policy.rs` 的 `pipe_interpreter_hit()`（规则⑧ encoded-exec
管道臂）按**字节**偏移扫描命令串（`as_bytes()`），却在 974 行用同一偏移**切片
UTF-8 字符串** `command[start..j]`；当扫描被 Latin-1 空白字节（0xA0＝U+00A0
NBSP、0x85＝U+0085 NEL，`(byte as char).is_whitespace()` 为真）打断在多字节
汉字中间时，切片 panic；release profile `panic = "abort"` → 0xC0000409
（FAST_FAIL_FATAL_APP_EXIT，WER 桶 BEX64）→ `orz --stdio` 进程死亡 → run
停在 `tool_started` 无终态 → web 桥换代子进程（18524→7444），前端呈现为断连。

## 触发形状

PowerShell `-Pattern 'a|中文|b'` 中的 `|` 被无引号感知的扫描器当作管道；管道
后「词」是含 0xA0 字节的汉字（如 **格**＝E6 A0 BC）→ 974 行 panic。
今晚三次（22:25/22:43/22:53）全是 `Select-String -Path crates\orz-loop\src\
context_scale.rs -Pattern '…|摘要块格式|语义摘要块'` 形状。

## 复位验证（双源）

1. debug 端到端复现（真 OrzHost＋AgentLoopController＋真工具集＋yolo 桥＋
   RUN-12 崩溃原命令）→ panic at
   `crates\codegen\orz-tools\src\types\exec_policy.rs:974:31`。
2. 生产崩溃二进制（md5 6aff2359…，D:\tb-eval\orz-windows\orz.exe＝
   %LOCALAPPDATA%\orz\bin\orz.exe）无符号，但其 .rdata 内嵌 `#[track_caller]`
   Location 结构 @RVA 0x3602108：ptr→`…exec_policy.rs`，**line=974, col=31**
   （另有 line=960 col=34＝同函数 960 行 `command[i..]` 兄弟位点）。与 debug
   复现逐字一致。
3. 崩溃存根：RVA 0xadde95＝`mov ecx,7; int 29h`（abort thunk），WER 三报
   同偏移；10/22 以来 12 次同类崩溃（0xc00004xxx），偏移随出件平移、同日一致
   ——同一 panic 位点在不同二进制里的确定性表现。

## 复现测试（放回 orz-host/src/lib.rs tests 模块即可；修复后应转绿）

```rust
/// 370 断连根因回归钉：RUN-7ef4c983-12 崩溃原命令端到端
/// （exec_policy pipe_interpreter_hit 字节切片 @974：修前 abort，修后应过）。
#[tokio::test]
async fn repro_370_terminal_crash_end_to_end() {
    unsafe { std::env::set_var("ORZ_YOLO", "1") };
    let dir = test_dir();
    let journal = JournalRecorder::new(dir.join("j"));
    tokio::task::LocalSet::new()
        .run_until(async {
            let permission = PermissionBridge::spawn("repro-370-session", None, &dir)
                .expect("permission bridge spawn");
            let host = OrzHost::with_credential_reader(
                journal,
                &dir,
                WorkspaceTrust::ObservedTrusted,
                Some(permission),
                failing_reader(),
            )
            .expect("host build");
            let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
                orz_loop::gateway::fake::ScriptedResponse::tool_calls(vec![ToolCall {
                    name: "run_terminal_cmd".to_string(),
                    arguments: serde_json::json!({
                        "command": "cd D:\\CLI\\orz; Select-String -Path crates\\orz-loop\\src\\context_scale.rs -Pattern 'window_remaining_notice|至少命中|MODEL_SUMMARY_PREFIX|摘要块格式|语义摘要块' | Select-Object LineNumber,Line",
                        "description": "repro 370",
                    }),
                    call_id: "call-repro".to_string(),
                }]),
                orz_loop::gateway::fake::ScriptedResponse::text("完成"),
                orz_loop::gateway::fake::ScriptedResponse::text("完成"),
            ]));
            let controller = orz_loop::AgentLoopController::with_gateway(gateway)
                .with_acaf_fail_closed(false);
            controller
                .run_turn(&host, "复现", "RUN-REPRO370", "manifest-sha", 0, None, None, None)
                .await
                .expect("run turn");
        })
        .await;
    let _ = std::fs::remove_dir_all(&dir);
}
```

要点（排查中踩过的坑）：
- `permission=None` 走 no-bridge fail-closed（lib.rs P1-1，ORZ_YOLO 不生效），
  必须真桥；`PermissionBridge::spawn` 内部 `spawn_local`，须包 `LocalSet`。
- 直连 `LocalTerminalBackend::run()` 喂同一命令**不崩**（底层执行无恙）——
  panic 在分发面的 exec_policy 审查，必须走完整 loop。

## 修复建议（供 orz 侧裁决）

`pipe_interpreter_hit` 改按 `char_indices()` 扫描（空白/断词判定用真实
`char`，勿 `bytes[j] as char`）；或切片前做 char-boundary 归一。注意：
- 960 行 `command[i..].find('|')` 与 `i = j.max(...)` 同病（i 可能停汉字中间）；
- 语义不变：char 口径下「格」非空白，词＝`摘要块格式`，不属解释器闭集 →
  Allow，与现状判定结果一致（现状是 panic 而非误判）；
- 建议加回归钉：`-Pattern 'a|格式'` 类命令须 Allow 且不 panic；
- 排查期间 orz 侧临时加的两处 `repro_370` 测试已撤树，本文件为唯一存档。
