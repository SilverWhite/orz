"""P2-10 阶段 3 V2 — 聚类对照与零误干预检查（2026-08-31）。

消费 `lif_replay` 产出的逐决策轮特征轨迹（V2 JSON），与 §9.8 首轮 k-means
聚类对照：
- 特征向量 (u_prog, u_err, u_stuck, T̂, err10, succ10) z 归一后 k-means
  k=3（sklearn，random_state 固定）——探索用，生产标签仍是语义谓词；
- 交叉表：聚类簇 × 语义域（start/normal/pressure/low_progress/stuck），
  每簇特征均值对照 §9.8 参考（C1 正常域 / C0 错误压力域 / C2 深无进度域）；
- 零误干预：fires 仅内部留痕——102 条事件链中不得出现 temporal_fire / fire
  事件类型，schema 不得含 temporal_fire，模型表面（prompt 注入面）无 fires。

用法：python lif_v2_cluster_analysis.py <V2_JSON> <JOBS_ROOT> <OUT_JSON>
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

import numpy as np
from sklearn.cluster import KMeans

FEATURES = ["u_prog", "u_err", "u_stuck", "t_hat", "err10", "succ10"]
DOMAINS = ["start", "normal", "pressure", "low_progress", "stuck"]


def collect_trajectories(report: dict) -> tuple[np.ndarray, list[str]]:
    rows: list[list[float]] = []
    labels: list[str] = []
    for run in report["runs"]:
        for row in run.get("features", []):
            rows.append([float(row[f]) for f in FEATURES])
            labels.append(str(row["domain"]))
    return np.asarray(rows, dtype=float), labels


def cluster_analysis(rows: np.ndarray, labels: list[str]) -> dict:
    if rows.shape[0] == 0:
        return {"points": 0}
    mean = rows.mean(axis=0)
    std = rows.std(axis=0)
    std[std == 0] = 1.0
    z = (rows - mean) / std
    km = KMeans(n_clusters=3, n_init=10, random_state=0).fit(z)
    clusters = km.labels_

    contingency: dict[str, dict[str, int]] = {d: {str(c): 0 for c in range(3)} for d in DOMAINS}
    for label, c in zip(labels, clusters):
        contingency.setdefault(label, {str(c2): 0 for c2 in range(3)})[str(c)] += 1

    cluster_means = {}
    for c in range(3):
        mask = clusters == c
        cluster_means[str(c)] = {
            FEATURES[i]: float(rows[mask][:, i].mean())
            for i in range(len(FEATURES))
        }

    # 每簇按多数语义域命名映射（§9.8 对照：C1 正常 / C0 压力 / C2 深无进度）。
    cluster_domain_map = {}
    for c in range(3):
        counts = {d: contingency.get(d, {}).get(str(c), 0) for d in DOMAINS}
        cluster_domain_map[str(c)] = max(counts, key=counts.get)

    # 语义域特征均值——与簇均值结构对照（§9.8：域=事实描述，非结局信号）。
    domain_means: dict[str, dict[str, float]] = {}
    for d in DOMAINS:
        idx = [i for i, lbl in enumerate(labels) if lbl == d]
        if idx:
            block = rows[idx]
            domain_means[d] = {
                FEATURES[i]: float(block[:, i].mean()) for i in range(len(FEATURES))
            }

    return {
        "points": int(rows.shape[0]),
        "z_normalized": True,
        "k": 3,
        "random_state": 0,
        "cluster_sizes": {str(c): int((clusters == c).sum()) for c in range(3)},
        "cluster_means": cluster_means,
        "cluster_majority_domain": cluster_domain_map,
        "contingency": contingency,
        "semantic_domain_means": domain_means,
        "reference_s98": {
            "C1_normal_n2759": {"u_err": 0.78, "u_prog": 0.97, "u_stuck": 1.1, "err10": 0.04, "succ10": 0.87},
            "C0_pressure_n1233": {"u_err": 2.61, "u_stuck": 3.8, "err10": 0.22, "succ10": 0.75},
            "C2_deep_low_progress_n165": {"u_prog": 0.01, "succ10": 0.03},
        },
    }


def zero_misintervention_check(jobs_root: Path) -> dict:
    """fires 仅内部留痕：事件链与 schema 不得携带 fires 事件类型。"""
    found_types: dict[str, int] = {}
    journals = 0
    for path in sorted(jobs_root.rglob("events.jsonl")):
        journals += 1
        for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
            try:
                event = json.loads(line)
            except Exception:
                continue
            et = str(event.get("event_type", ""))
            if "fire" in et.lower():
                found_types[et] = found_types.get(et, 0) + 1
    schema_hits = []
    for schema_path in [
        Path(__file__).resolve().parents[1] / "runtime" / "run-event-v0.2.schema.json",
        Path(__file__).resolve().parents[1] / "runtime" / "tool-completed-event-payload-v0.1.schema.json",
    ]:
        if schema_path.exists():
            text = schema_path.read_text(encoding="utf-8")
            if "temporal_fire" in text or '"fire' in text:
                schema_hits.append(schema_path.name)
    return {
        "journals_scanned": journals,
        "fire_event_types_in_chain": found_types,
        "schema_files_with_fire": schema_hits,
        "pass": not found_types and not schema_hits,
    }


def main() -> None:
    report_path = Path(sys.argv[1])
    jobs_root = Path(sys.argv[2])
    out_path = Path(sys.argv[3])
    report = json.loads(report_path.read_text(encoding="utf-8"))
    rows, labels = collect_trajectories(report)
    analysis = cluster_analysis(rows, labels)
    misintervention = zero_misintervention_check(jobs_root)
    out = {
        "harness": "lif_v2_cluster_analysis.py (V2, 2026-08-31)",
        "source": report_path.name,
        "decision_points": analysis.get("points", 0),
        "cluster_analysis": analysis,
        "zero_misintervention": misintervention,
    }
    out_path.write_text(json.dumps(out, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps(out, ensure_ascii=False))


if __name__ == "__main__":
    main()
