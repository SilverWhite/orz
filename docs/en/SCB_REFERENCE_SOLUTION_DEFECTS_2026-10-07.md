# SlopCodeBench — reference-solution defects beyond KNOWN_ISSUES (4 problems): mechanical evidence

> **Status**: attachment 3 of the SCBench official-inquiry letter (2026-10-07). Prepared for the
> problem maintainers; raw tooling output, transcribed without interpretation.
> **Where this comes from**: the oracle-validation step of the **official** `scb_to_harbor.py`
> conversion pipeline while converting the full 36-problem set to Harbor tasks. This is the
> conversion tool's own verdict, not our judgement of the problems.
> **Who ran it**: the conversion was run by the orz maintainer; the four newly found defects were
> then surfaced by an AI review pass (GLM 5.3 Flash) over the tool's output, and are handed to you
> here with the raw evidence so you can verify directly.
> **What we are not claiming**: we are not the authors of these problems or their tests, and we are
> not asserting a root cause inside the reference solutions. We are reporting that **the official
> pipeline's oracle validation fails on these four problems at this pin**, and giving the
> checkpoint-level comparison that made us think the reference solution — rather than the tests —
> is the more likely culprit.

---

## 1. What "oracle validation failed (rc=4)" means here

`scb_to_harbor.py --validate-with-oracle` converts each problem into a Harbor task and then, as a
fidelity check, **runs the problem's own reference solution against the problem's own test suite at
each checkpoint**. A pass means the reference solution satisfies its own tests. Exit code `4` is the
pipeline's "oracle validation failed" verdict: at least one checkpoint's reference solution did not
pass its tests.

Reproduction command (as run, full set):

```
cd D:/tb-eval/scbench/scb-problems
uv run scripts/scb_to_harbor.py --org silverwhite --all --validate-with-oracle
```

Pins: harness `main=31ceea3`, `scb-problems` `38d627e`. Per-problem conversion logs are attached
under `convert_logs/<problem>.log`.

## 2. Summary

| Problem | Pipeline verdict | Checkpoint(s) failing | Reference solution (oracle) | Agent run at the same checkpoint |
|---|---|---|---|---|
| **env_manager** | rc=4 | ck3 | strict 0.9840 | ck3 **isolated 1.0000**, core 3/3, 176/187 full |
| **file_backup** | rc=4 | ck2, ck3, ck4 | ck2 strict 0.84 / **core 0.0**; ck3 strict 0.6765 / **core 0.0**; ck4 strict 0.7528 | ck2 **core 1/1**; ck3 **core 1/1**; ck4 core 1/1 |
| **mvvault** | rc=4 | ck5 | strict 0.9946, core 0.875, isolated 0.9667 | ck5 **isolated 1.0000**, core 8/8, 174/185 full |
| **test_translator** | rc=4 | ck1–ck6 (+ ck7/ck8 structural) | ck1 strict 0.7549 / **core 0.6667**; ck2 0.6880 / **core 0.6667**; ck4 0.7277 / **core 0.6667**; ck5 0.7320 / **core 0.6667**; ck6 0.7559 / core 0.9369 | ck1 **core 21/21** (90/102 full); ck3 **isolated 1.0000**, core 6/6 |

Note the shape: on every one of these problems there is at least one checkpoint where the reference
solution fails a **Core** test while the agent run either passes the whole Core group or passes 100%
of that checkpoint's own (isolated) tests. Both columns are produced by the same harness, on the same
checkpoint boundaries, with the same test extraction.

The agent column is the orz k=1 run over the same problems (carrier orz v0.8.14, model
DeepSeek-V4.1-Flash), reported in the round report attached as attachment 1; the per-checkpoint
records are in attachment 2.

---

## 3. Per-problem detail (tool output verbatim)

### 3.1 env_manager — `rc=4`, checkpoint_3

Tool output:

```
ERROR: [env_manager] failed (exit 4): Oracle validation failed for env_manager
- checkpoint_3: strict_pass_rate=0.983957219251337
```

Agent run at ck3: core 3/3, full 176/187, strict 0.9412, **isolated 1.0000**.

Reading: the reference solution fails roughly 1.6% of that checkpoint's tests, while the agent run
passes **100%** of the checkpoint's own tests (isolated) and the whole Core group. The agent's lower
strict number comes from other checkpoint-accumulated tests; at the checkpoint in question the
reference solution is the one that does not close its own suite.

### 3.2 file_backup — `rc=4`, checkpoint_2 / 3 / 4

Tool output:

```
ERROR: [file_backup] failed (exit 4): Oracle validation failed for file_backup
- checkpoint_2: strict_pass_rate=0.84
- checkpoint_2: core_pass_rate=0.0
- checkpoint_2: isolated_pass_rate=0.5555555555555556
- checkpoint_3: strict_pass_rate=0.6764705882352942
- checkpoint_3: core_pass_rate=0.0
- checkpoint_3: isolated_pass_rate=0.2222222222222222
- checkpoint_4: strict_pass_rate=0.7528089887640449
```

Agent run: ck2 core 1/1 (32/50 full, isolated 0.7222); ck3 core 1/1 (44/68, isolated 0.6667);
ck4 core 1/1 (62/89, isolated 0.8571).

Reading: this is the sharpest case. At ck2 and ck3 the reference solution's **Core pass rate is
0.0** — it fails the entire Core group — and its isolated rate drops to 0.22 at ck3, while the agent
run passes Core at both checkpoints and closes the problem 4/4 checkpoints overall.

### 3.3 mvvault — `rc=4`, checkpoint_5

Tool output:

```
ERROR: [mvvault] failed (exit 4): Oracle validation failed for mvvault
- checkpoint_5: strict_pass_rate=0.9945945945945946
- checkpoint_5: core_pass_rate=0.875
- checkpoint_5: isolated_pass_rate=0.9666666666666667
```

Agent run at ck5: core 8/8, full 174/185, strict 0.9405, **isolated 1.0000**.

Reading: reference solution short by one test in the checkpoint's own suite (isolated 0.9667) and by
one Core test; the agent run passes 100% of that checkpoint's own tests and the full Core group.

### 3.4 test_translator — `rc=4`, checkpoint_1 through checkpoint_8

Tool output:

```
ERROR: [test_translator] failed (exit 4): Oracle validation failed for test_translator
- checkpoint_1: strict_pass_rate=0.7549019607843137
- checkpoint_1: core_pass_rate=0.6666666666666666
- checkpoint_1: isolated_pass_rate=0.7549019607843137
- checkpoint_2: strict_pass_rate=0.6880131362889984
- checkpoint_2: core_pass_rate=0.6666666666666666
- checkpoint_2: isolated_pass_rate=0.6745562130177515
- checkpoint_3: strict_pass_rate=0.7297297297297297
- checkpoint_4: strict_pass_rate=0.7277179236043095
- checkpoint_4: core_pass_rate=0.6666666666666666
- checkpoint_4: isolated_pass_rate=0.7232704402515723
- checkpoint_5: strict_pass_rate=0.7319848293299621
- checkpoint_5: core_pass_rate=0.6666666666666666
- checkpoint_5: isolated_pass_rate=0.7397504456327986
- checkpoint_6: strict_pass_rate=0.7558659217877095
- checkpoint_6: core_pass_rate=0.9368932038834952
- checkpoint_6: isolated_pass_rate=0.9375
- checkpoint_7: missing strict_pass_rate in {}
- checkpoint_8: missing step result
```

Agent run: ck1 core 21/21 (90/102, isolated 0.8824); ck3 core 6/6, isolated 1.0000; the agent closes
2/8 checkpoints, matching this problem's difficulty (it carries 2,069 tests in total, the largest in
the set).

Reading: on ck1, ck2, ck4 and ck5 the reference solution fails a third of its **Core** group; the
agent run passes the entire Core group at ck1 and ck3. Two further structural observations: the
reference oracle produced **no** `strict_pass_rate` for ck7 and **no step result at all** for ck8 —
worth a look on your side independently of the pass rates.

---

## 4. Known-issue footnotes (recorded for completeness, not part of the four)

These two are already covered by `KNOWN_ISSUES`; their oracle validation also returns rc=4 and is
listed here so the two columns can be compared on the same footing.

| Problem | Reference solution (oracle) | Agent run | Comment |
|---|---|---|---|
| dynamic_buffer | ck2 strict 0.96 / isolated 0.90; ck3 strict 0.9808; ck4 strict 0.7093 / core 0.5 / isolated 0.3824 | ck4 core 7/20, isolated 0.1765 | both sides fail ck4 — consistent with a known-issue problem, **not** evidence of a new reference defect |
| eve_market_tools | ck4 strict 0.9867 / core 0.875 / isolated 0.9231 | ck4 core 3/8, isolated 0.4615 | reference solution degrades at ck4, but the agent also fails there — recorded, not claimed as a new defect |

For completeness on the conversion run: `forge`, `textdrop` and `trajectory_api` showed transient
`rc=3` (infrastructure) and converged to `rc=0` on retry; they are not defect cases. All other 30
problems converted at `rc=0`.

## 5. What would settle this

The decisive check is on your side and is cheap: run the reference solution for the checkpoints listed
above and look at which tests fail. If the failing tests assert behaviour that the problem statement
does not require (or that the reference solution itself contradicts), the reference solution needs
the fix; if they assert required behaviour, then the reference solution genuinely does not satisfy its
own specification and the checkpoint's expectations may need revision. Either way, the raw tool
output above is what the official pipeline produced at pin `38d627e`.

## 6. Files in this attachment

| File | Content |
|---|---|
| `SCB_REFERENCE_SOLUTION_DEFECTS_2026-10-07.md` | this document |
| `convert_logs/env_manager.log` | full per-problem conversion log (tool output + log directory pointer) |
| `convert_logs/file_backup.log` | as above |
| `convert_logs/mvvault.log` | as above |
| `convert_logs/test_translator.log` | as above |
| `convert_logs/dynamic_buffer.log` | known-issue comparison |
| `convert_logs/eve_market_tools.log` | known-issue comparison |
| `convert_summary3.txt` | per-problem exit codes and timestamps for the full-set conversion pass |
