//! F4 失败目标聚合分区（P2-12 COMPRESSION-LINGUISTIC-FORMAL-LAYER 方案 A，
//! 2026-09-02 定案；ADR 转正式设计与实施待登记）。
//!
//! 目的：压缩的「注意事项」槽需要有信息密度的失败事实——同一失败目标
//! （F4 身份 (kind, id)）在同一 plan epoch 内反复失败时，只保留一行聚合：
//! 计数 + 错误码集合 + 首末发生时间 + 行内域序列标注。这是纯机械、零模型、
//! 确定性的记录结构；它只消费工具失败事件在写入时已携带的结构化字段
//! （失败目标身份、结构化错误码、LIF 当轮域值与轮号），不解析日志内容。
//!
//! 口径（讨论稿 §3 / §6 收口，2026-09-02 用户裁决）：
//! - 行键 = F4 身份 (kind, id)；同一目标失败 N 次计 N（一行内计 N）；
//!   不按日志分行、不重复写多条；错误码集合区分失败模式。
//! - 错误码：结构化 code（非日志内容）；行内集合全留/不留二选一——
//!   本结构保留全部 code→count（渲染层受槽位上限约束时走既有截断+指针）。
//! - 首末时间：相对 run 起点墙钟秒（与 temporal 分区 `t` 同刻度）。
//! - 域：不参与行键（方案 A）——失败事件写入时按所属决策轮盖章
//!   （LIF 域 + 决策轮号），行内维护域段书签：同域并入当前段、异域开
//!   新段；同一目标跨域不切行（「跨域不合并」已撤销）。切换轮标签可能
//!   带判定误差——域只是行内标注，不产生行碎片（flicker 由段序吸收）。
//! - 聚合状态归黑板：epoch 作用域，随黑板 plan epoch 轮换重置、随
//!   EpochSnapshot 归档/恢复（与 exec/actions 同纪律）。
//!
//! 已知边界（2026-09-02 审查登记）：
//! - `t` 与轮号是 run 作用域（run-relative LIF 原点）：行不得跨 run 边界
//!   累计（新 run 重建 LifEngine 后原点不同）。当前接线——CLI 新 plan_id
//!   轮换清空、ACP 每 prompt 重建黑板——不产生跨 run 混轴；恢复同一 epoch
//!   继续累计属未来切片。
//! - 段轮区间 = 该目标在该域的失败事件首末轮（非域整体驻留区间）；同域
//!   隔空合并保留时间轴近似（同域并入当前段，非连续同域各自成段）。
//!
//! > B1 取代注（2026-09-03，P2-13 / R5 conversation-relative 轴 + 会话级
//! > 续载）：上条已知边界的「run 作用域 / ACP 每 prompt 重建黑板 / 恢复
//! > 同 epoch 继续累计属未来切片」在 ACP 路径已被 B1 关闭——黑板（含本
//! > 分区）随会话快照跨 prompt 保留，LIF 轴原点 = `session_started_at`，
//! > 写入的 `t` 与决策轮号为会话相对且跨 prompt 单调累计，行可跨 prompt
//! > 续算（P2-12 登记的「未来切片」随 B1 实现落地）。该边界现仅适用于
//! > CLI 单 run 与 `--plan`/epoch 轮换路径（run-relative、随轮换重置）。
//! > 处置登记：仓库根 `docs/audits/P2-13_B1_REVIEW_HANDLING_2026-09-03.md`。

use orz_assurance::lif::Domain;
use serde::{Deserialize, Serialize};

/// 行内错误码计数（`[timeout×3, deny×2]`）；顺序 = 首次出现顺序（确定性）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeCount {
    pub code: String,
    pub count: u64,
}

/// 行内域段书签：同域并入当前段、异域开新段；`from_round..=to_round` 是
/// 该目标在本段内失败事件的决策轮范围（首末轮），非域的整体驻留区间。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainSegment {
    pub domain: Domain,
    pub from_round: u64,
    pub to_round: u64,
}

/// 一个失败目标（(kind, id)）在 plan epoch 内的聚合行。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FailureTargetRow {
    /// F4 kind：cmd_target / anchor_target / file_target / url_target。
    pub kind: String,
    /// F4 sha256 身份（确定性、非原始文本）。
    pub id: String,
    /// 目标显示预览（≤80 B：cmd_preview / path / canonical_url）。
    pub preview: String,
    /// 失败事件计数（同一目标失败 N 次计 N）。
    pub count: u64,
    /// 错误码行内集合（全留/不留；顺序 = 首次出现顺序）。
    pub codes: Vec<CodeCount>,
    /// 首末发生时间（相对 run 起点墙钟秒，与 temporal `t` 同刻度）。
    pub first_t: f64,
    pub last_t: f64,
    /// 行内域序列标注（写时盖章 + 同域并入/异域开段）。
    pub segments: Vec<DomainSegment>,
}

impl FailureTargetRow {
    fn new(
        kind: &str,
        id: &str,
        preview: &str,
        code: &str,
        t: f64,
        round: u64,
        domain: Domain,
    ) -> Self {
        let mut codes = Vec::new();
        if !code.is_empty() {
            codes.push(CodeCount {
                code: code.to_string(),
                count: 1,
            });
        }
        Self {
            kind: kind.to_string(),
            id: id.to_string(),
            preview: preview.to_string(),
            count: 1,
            codes,
            first_t: t,
            last_t: t,
            segments: vec![DomainSegment {
                domain,
                from_round: round,
                to_round: round,
            }],
        }
    }

    /// 同一目标再次失败：计数 +1；错误码集合并入（首次出现顺序）；
    /// 域段书签归并——同域并入当前段（扩展 to_round）、异域开新段。
    fn absorb(&mut self, code: &str, t: f64, round: u64, domain: Domain) {
        self.count = self.count.saturating_add(1);
        self.last_t = t;
        if !code.is_empty() {
            match self.codes.iter_mut().find(|c| c.code == code) {
                Some(existing) => existing.count = existing.count.saturating_add(1),
                None => self.codes.push(CodeCount {
                    code: code.to_string(),
                    count: 1,
                }),
            }
        }
        match self.segments.last_mut() {
            Some(last) if last.domain == domain => {
                last.to_round = last.to_round.max(round);
            }
            _ => self.segments.push(DomainSegment {
                domain,
                from_round: round,
                to_round: round,
            }),
        }
    }
}

/// F4 失败目标聚合分区（epoch 作用域；见模块头注释）。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct FailureAgg {
    #[serde(default)]
    pub rows: Vec<FailureTargetRow>,
}

impl FailureAgg {
    /// 记录一次携带 F4 身份的失败事件。`code` 为空串时不记错误码（事件
    /// 无结构化码的极端兜底）；行键 (kind, id) 幂等聚合。
    #[allow(clippy::too_many_arguments)] // 单一写入口：身份 3 + 载荷 4，保持机械直写
    pub fn record(
        &mut self,
        kind: &str,
        id: &str,
        preview: &str,
        code: &str,
        t: f64,
        round: u64,
        domain: Domain,
    ) {
        match self.rows.iter_mut().find(|r| r.kind == kind && r.id == id) {
            Some(row) => row.absorb(code, t, round, domain),
            None => self.rows.push(FailureTargetRow::new(
                kind, id, preview, code, t, round, domain,
            )),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(agg: &mut FailureAgg, id: &str, code: &str, t: f64, round: u64, domain: Domain) {
        agg.record("cmd_target", id, "python train.py", code, t, round, domain);
    }

    #[test]
    fn same_target_merges_count_first_last_and_codes() {
        let mut agg = FailureAgg::default();
        rec(&mut agg, "a", "tool_timeout", 10.0, 1, Domain::Normal);
        rec(&mut agg, "a", "tool_timeout", 40.0, 2, Domain::Normal);
        rec(&mut agg, "a", "execution_failed", 90.0, 3, Domain::Pressure);
        assert_eq!(agg.rows.len(), 1);
        let row = &agg.rows[0];
        assert_eq!(row.count, 3);
        assert_eq!(row.first_t, 10.0);
        assert_eq!(row.last_t, 90.0);
        // 错误码集合按首次出现顺序，各码计数独立。
        assert_eq!(
            row.codes,
            vec![
                CodeCount {
                    code: "tool_timeout".into(),
                    count: 2
                },
                CodeCount {
                    code: "execution_failed".into(),
                    count: 1
                },
            ]
        );
    }

    #[test]
    fn distinct_targets_stay_separate_rows_in_first_seen_order() {
        let mut agg = FailureAgg::default();
        rec(&mut agg, "b", "tool_timeout", 5.0, 1, Domain::Start);
        rec(&mut agg, "a", "tool_timeout", 7.0, 2, Domain::Start);
        rec(&mut agg, "b", "tool_timeout", 9.0, 3, Domain::Start);
        assert_eq!(agg.rows.len(), 2);
        assert_eq!(agg.rows[0].id, "b");
        assert_eq!(agg.rows[0].count, 2);
        assert_eq!(agg.rows[1].id, "a");
        assert_eq!(agg.rows[1].count, 1);
    }

    #[test]
    fn domain_segments_merge_same_and_open_new_on_change() {
        let mut agg = FailureAgg::default();
        rec(&mut agg, "a", "tool_timeout", 10.0, 10, Domain::Normal);
        rec(&mut agg, "a", "tool_timeout", 30.0, 12, Domain::Normal);
        // 异域开新段；回到同域不并入旧段（非连续同域保持独立，保留时间轴）。
        rec(&mut agg, "a", "tool_timeout", 60.0, 13, Domain::Pressure);
        rec(&mut agg, "a", "tool_timeout", 90.0, 15, Domain::Normal);
        let row = &agg.rows[0];
        assert_eq!(
            row.segments,
            vec![
                DomainSegment {
                    domain: Domain::Normal,
                    from_round: 10,
                    to_round: 12
                },
                DomainSegment {
                    domain: Domain::Pressure,
                    from_round: 13,
                    to_round: 13
                },
                DomainSegment {
                    domain: Domain::Normal,
                    from_round: 15,
                    to_round: 15
                },
            ]
        );
    }

    #[test]
    fn empty_code_records_no_code_entries() {
        let mut agg = FailureAgg::default();
        agg.record("file_target", "x", "a.rs", "", 1.0, 1, Domain::Start);
        assert!(agg.rows[0].codes.is_empty());
        assert_eq!(agg.rows[0].count, 1);
    }

    #[test]
    fn serde_roundtrip_preserves_rows() {
        let mut agg = FailureAgg::default();
        rec(&mut agg, "a", "tool_timeout", 10.0, 1, Domain::Normal);
        let json = serde_json::to_string(&agg).unwrap();
        let back: FailureAgg = serde_json::from_str(&json).unwrap();
        assert_eq!(back, agg);
    }
}
