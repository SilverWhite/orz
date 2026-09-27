#!/usr/bin/env python3
"""0by S1 复现探针：ACAF 签名器的「安装密钥库根」路径（秒级，离线）。

背景（0by / GAP-ACAF-SIGNER-UNREACHABLE，104 批立项）：桌面形态下凡需
控制票据的宿主工具（`browser_read` / `web_fetch` / `run_terminal_cmd` /
`search_replace`）被 `control_ticket_rejected: signer_unreachable` 拒，detail
＝`io error: 管道正在被关闭。 (os error 232)`。客户端把签名器子进程的 stderr
丢了（`crates/orz-loop/src/acaf.rs::spawn_child` 用 `Stdio::null()`），所以
「签名器为何退出」当时不可见。

本探针不依赖模型与网络：直接按客户端用法拉起 `orz-signer.exe`，只换
`ORZ_SIGNER_KEYSTORE_ROOT` 一个变量，即可在秒级给出「红 / 绿」两态。

两种用法：

1) 变量隔离矩阵（默认）——对每个载体目录测两个密钥库根：

       python scripts/probe_acaf_keystore_root.py
       python scripts/probe_acaf_keystore_root.py --bin-dir D:\\tb-eval\\orz-windows

   矩阵的第二个根 `<acaf>\\keystore` 不是臆造：`scripts/dogfood_launch.ps1`
   把它算成 `$keystore = Join-Path $AcafRoot 'keystore'` 并据此导出
   `ORZ_ACAF_KEYSTORE`，而同一脚本用的是 `& $provision $AcafRoot $manifest`
   ——provision 把密钥库建在 `<acaf>` 本身。两者错开一层目录。

2) 启动器装配钉子（0by S2 的先红后绿钉）——**读真实启动器脚本**、取出它
   自己声明的三处口径（provision 的密钥库参数 / 导出的 `ORZ_ACAF_KEYSTORE`
   / manifest），在空目录上真跑一次 `-DryRun`（该模式**会真的 provision**），
   再用它导出的那个密钥库根驱动签名器：

       python scripts/probe_acaf_keystore_root.py --check-launcher
       python scripts/probe_acaf_keystore_root.py --check-launcher --script .tmp-b107-launcher-prefix.ps1

   在役口径（四处独立来源一致）：**密钥库根＝`<acaf>\\keystore`**、manifest＝
   `<acaf>\\signer-manifest.json`——容器侧常量 `/etc/orz-acaf/keystore`、
   载体重建批的 provision 落点、历年实跑命令、两个启动器的导出值。
   红（修复前形态）：脚本导出 `<acaf>\\keystore`，却把 provision 建在 `<acaf>`
   ⇒ 口径自相矛盾 ＋ 新目录下签名器 exit 1（`installation key root is not a
   directory`）。绿（修复后）：provision 落点＝导出根＝`<acaf>\\keystore`
   ⇒ 签名器 exit 0 应答。

退出码：0＝所有被测格子符合预期；1＝有格子不符（红/绿翻转即回归信号）。
"""

from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path

DEFAULT_BIN_DIRS = (
    r"D:\tb-eval\orz-windows",
    r"D:\tb-eval\orz-windows-080",
    r"D:\tb-eval\orz-windows-080c",
)
DEFAULT_DONOR = r"D:\tb-eval\orz-windows"
DEFAULT_LAUNCHER = r"scripts\dogfood_launch.ps1"
PROBE_SESSION = "PROBE-0BY-S1"


def _signer_names() -> tuple[str, str]:
    if os.name == "nt":
        return "orz-signer.exe", "orz-acaf-provision.exe"
    return "orz-signer", "orz-acaf-provision"


def _request() -> str:
    return json.dumps(
        {
            "id": 1,
            "method": "initialize_session",
            "params": {
                "session_id": PROBE_SESSION,
                "agent_id": "probe",
                "goal_version": 1,
                "goal_digest": "0" * 64,
                "policy_revision": 1,
            },
        }
    )


def drive_signer(bin_dir: Path, keystore_root: Path, manifest: Path) -> dict:
    """按客户端用法投一条 `initialize_session`，回收 exit / stderr / 应答。"""
    signer_name, _ = _signer_names()
    signer = bin_dir / signer_name
    env = {
        **os.environ,
        "ORZ_SIGNER_MANIFEST": str(manifest),
        "ORZ_SIGNER_KEYSTORE_ROOT": str(keystore_root),
    }
    proc = subprocess.Popen(
        [str(signer)],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        env=env,
        text=True,
        encoding="utf-8",
        errors="replace",
    )
    try:
        out, err = proc.communicate(_request() + "\n", timeout=30)
    except subprocess.TimeoutExpired:
        proc.kill()
        out, err = proc.communicate()
        err = (err or "") + "\n[probe] 签名器 30s 内未应答"
    answered = '"session_key_hex"' in (out or "")
    return {
        "exit": proc.returncode,
        "answered": answered,
        "stderr": " ".join((err or "").split()),
        "stdout_tail": (out or "").strip()[:160],
    }


def _verdict(cell: dict) -> str:
    if cell["answered"]:
        return "绿（密钥库可加载，签名器应答）"
    if "not a directory" in cell["stderr"]:
        return "红（密钥库根不可用：目录不存在）"
    return "红（其他）"


def matrix(paths: list[str]) -> int:
    rows: list[tuple[str, str, bool, dict]] = []
    for raw in paths:
        bin_dir = Path(raw)
        _, provision_name = _signer_names()
        if not (bin_dir / provision_name).is_file():
            print(f"跳过（非载体目录）：{bin_dir}", flush=True)
            continue
        acaf = bin_dir / "acaf"
        manifest = acaf / "signer-manifest.json"
        for label, root in (("provision 实际位置 <acaf>", acaf),
                            ("启动器实际导出 <acaf>\\keystore", acaf / "keystore")):
            cell = drive_signer(bin_dir, root, manifest)
            rows.append((str(bin_dir), label, root.is_dir(), cell))
            print(
                f"[{'绿' if cell['answered'] else '红'}] {bin_dir}\n"
                f"      keystore={root}  exists={root.is_dir()}\n"
                f"      exit={cell['exit']}  {_verdict(cell)}\n"
                f"      stderr={cell['stderr'] or '(空)'}",
                flush=True,
            )
    print("\n== 汇总（载体 × 密钥库根） ==")
    for name, label, exists, cell in rows:
        print(
            f"{'绿' if cell['answered'] else '红'}  {Path(name).name:<18} "
            f"exists={str(exists):<5} {label}"
        )
    return 0


def _extract_once(text: str, pattern: str, what: str) -> str:
    """取出启动器脚本里唯一的一处口径；形态变了就响亮报错（不猜）。"""
    hits = re.findall(pattern, text, flags=re.MULTILINE)
    if len(hits) != 1:
        raise SystemExit(
            f"[钉子] 启动器形态已变：{what} 命中 {len(hits)} 次（期望 1）——请复核本钉子"
        )
    return hits[0] if isinstance(hits[0], str) else hits[0][0]


def _resolve_path_expr(expr: str, symbols: dict[str, str]) -> str:
    """解 `$Var` / `Join-Path $Var 'x'`（可嵌套）两种形态。"""
    expr = expr.strip()
    if expr in symbols:
        return symbols[expr]
    match = re.fullmatch(r"Join-Path\s+(.+?)\s+['\"]([^'\"]+)['\"]", expr)
    if match:
        return os.path.join(
            _resolve_path_expr(match.group(1), symbols), match.group(2)
        )
    raise SystemExit(f"[钉子] 认不得的路径表达式：{expr}——请复核本钉子")


def check_launcher(script: Path, donor: Path, scratch: Path) -> int:
    """0by S2 先红后绿钉：启动器自报的密钥库根必须能被签名器真正加载。"""
    signer_name, provision_name = _signer_names()
    text = script.read_text(encoding="utf-8")
    keystore_expr = _extract_once(text, r"^\$keystore\s*=\s*(.+?)\s*$", "`$keystore` 赋值")
    manifest_expr = _extract_once(text, r"^\$manifest\s*=\s*(.+?)\s*$", "`$manifest` 赋值")
    env_keystore = _extract_once(
        text, r"^\$env:ORZ_ACAF_KEYSTORE\s*=\s*(\S+)\s*$", "`ORZ_ACAF_KEYSTORE` 导出"
    )
    env_manifest = _extract_once(
        text, r"^\$env:ORZ_ACAF_MANIFEST\s*=\s*(\S+)\s*$", "`ORZ_ACAF_MANIFEST` 导出"
    )
    prov_call = re.findall(
        r"^\s*&\s*\$provision\s+(\S+)\s+(\S+)\s*$", text, flags=re.MULTILINE
    )
    if len(prov_call) != 1:
        raise SystemExit(
            f"[钉子] 启动器形态已变：provision 调用命中 {len(prov_call)} 次（期望 1）"
        )

    if scratch.exists():
        shutil.rmtree(scratch)
    bin_dir = scratch / "bin"
    bin_dir.mkdir(parents=True)
    for name in (signer_name, provision_name, "orz.exe" if os.name == "nt" else "orz"):
        source = donor / name
        if source.is_file():
            shutil.copy2(source, bin_dir / name)
    grok_home = donor / "grok-home"
    if grok_home.is_dir():
        shutil.copytree(grok_home, bin_dir / "grok-home")
    acaf_root = str(scratch / "acaf")
    task_file = scratch / "task.txt"
    task_file.write_text("dry-run only\n", encoding="utf-8")

    symbols = {"$AcafRoot": acaf_root}
    keystore_root = _resolve_path_expr(keystore_expr, symbols)
    manifest_path = _resolve_path_expr(manifest_expr, symbols)
    symbols.update({"$keystore": keystore_root, "$manifest": manifest_path})
    prov_keystore = _resolve_path_expr(prov_call[0][0], symbols)
    prov_manifest = _resolve_path_expr(prov_call[0][1], symbols)

    print(f"脚本 = {script}")
    expected_keystore = os.path.join(acaf_root, "keystore")
    checks = [
        ("provision 落点 ＝ 导出的密钥库根", prov_keystore == keystore_root,
         f"{prov_keystore} vs {keystore_root}"),
        ("密钥库根 ＝ <AcafRoot>\\keystore（在役口径）", keystore_root == expected_keystore,
         f"{keystore_root} vs {expected_keystore}"),
        ("manifest ＝ <AcafRoot>\\signer-manifest.json",
         manifest_path == os.path.join(acaf_root, "signer-manifest.json"),
         manifest_path),
        ("provision 的 manifest 参数 ＝ 导出的 manifest", prov_manifest == manifest_path,
         f"{prov_manifest} vs {manifest_path}"),
        ("导出用 `$keystore`（不是另算一条路径）", env_keystore == "$keystore", env_keystore),
        ("导出用 `$manifest`", env_manifest == "$manifest", env_manifest),
    ]
    ok = True
    for label, passed, detail in checks:
        ok = ok and passed
        print(f"  [{'绿' if passed else '红'}] {label}（{detail}）")

    # 按脚本自己声明的口径跑一次 provision（＝启动器 ③ 那一步的原样调用），
    # 再拿它导出的密钥库根驱动签名器。
    # 注：**不**在此处拉起 `powershell -File <启动器>`——本机已登记的启动器摩擦
    # 之一（`Get-FileHash` 在非交互父进程下不可用）会在 provision 之前就把脚本
    # 打断，那与本条判据无关，不得混进来。
    proc = subprocess.run(
        [str(bin_dir / provision_name), prov_keystore, prov_manifest],
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
    )
    print(f"provision exit={proc.returncode}（argv＝脚本声明的 {prov_keystore} / {prov_manifest}）")
    if proc.returncode != 0:
        ok = False
        print("  [红] provision 未通过：")
        print("      " + " ".join((proc.stderr or proc.stdout).split())[:400])

    cell = drive_signer(bin_dir, Path(keystore_root), Path(manifest_path))
    print(
        f"驱动签名器：ORZ_SIGNER_KEYSTORE_ROOT={keystore_root}\n"
        f"      exit={cell['exit']}  {_verdict(cell)}\n"
        f"      stderr={cell['stderr'] or '(空)'}"
    )
    ok = ok and cell["answered"]
    print(f"\n装配钉子={'绿（启动器导出的密钥库根可被签名器加载）' if ok else '红（口径不符或根不可用）'}")
    return 0 if ok else 1


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--bin-dir",
        action="append",
        default=None,
        help="被测载体目录（可重复；缺省为三个已知载体）",
    )
    parser.add_argument(
        "--check-launcher",
        metavar="SCRATCH_DIR",
        default=None,
        help="在空目录上真跑一次启动器装配（-DryRun）并校验其密钥库根",
    )
    parser.add_argument(
        "--script",
        default=DEFAULT_LAUNCHER,
        help=f"被校验的启动器脚本（缺省 {DEFAULT_LAUNCHER}）",
    )
    parser.add_argument(
        "--donor",
        default=DEFAULT_DONOR,
        help=f"复制三件套的来源载体目录（缺省 {DEFAULT_DONOR}）",
    )
    args = parser.parse_args()
    if args.check_launcher:
        script = Path(args.script)
        if not script.is_file():
            script = Path(r"D:\CLI") / args.script
        return check_launcher(script, Path(args.donor), Path(args.check_launcher))
    if args.bin_dir:
        return matrix(args.bin_dir)
    return matrix(list(DEFAULT_BIN_DIRS))


if __name__ == "__main__":
    sys.exit(main())
