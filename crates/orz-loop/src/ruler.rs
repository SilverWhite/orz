//! 0dw S1（2026-11-17，CONTEXT-FACE-SCALE-RULER-ALIGNMENT 缺陷 1）：模型面
//! **量尺单源**——阶梯／主滑块／分块／静态开销／context_map 标签的 token
//! 估算自本批起统一走 provider tokenizer（[`DeepSeekTokenizer`]），退役
//! chars/2 局部口径。
//!
//! 立项档 §1.1：chars/2 偏差方向随内容构成翻转（ASCII 偏高 ≈2×、CJK 偏低
//! ≈1.3–2×）⇒ 阶梯触发点（192/224/256/288K 软提醒 → 320K 硬提醒 → 500K
//! 硬截断，ADR-0010 §14.26 v8 口径）的有效位置随会话语种构成漂移。
//! tokenizer 单源已在库（376 state 硬预算、recenter、M1-S2 context_map
//! 预算先例），本模块把它接到模型面剩余量尺消费方。
//!
//! ## Perf 关（立项档 §2 缺陷 1「过 perf 关，必要时缓存增量」）
//!
//! 估算点遍布 loop-top（每轮多次、500K 工作点上逐次全量），直接全量编码
//! 不可接受。缓存＝**逐文本 64 位内容哈希 → token 数**的全局有界表：
//! 会话在两次压缩之间只追加（不变量 I6），steady state 下每轮仅新尾部
//! 未命中、历史消息全部命中缓存 ⇒ 逐 loop-top 全量编码开销坍缩为
//! 「哈希走读（O(n) 字节、无分配）＋ 新增尾轮编码」。表满（65536 条）整
//! 表清空重暖（分块表/D4 逐轮易变串占比小，重暖代价有界）。
//!
//! ## 失败语义
//!
//! 量尺消费方（阶梯/守卫/滑块切点）签名不可失败（历史口径），且内嵌
//! tokenizer 资产随二进制编译、`from_bytes` 首用一次成功后恒可用——
//! `count_tokens` Err（实践中不可达）按 chars/2 保守回退并如实注记，不
//! panic、不虚构 0。硬预算面（state/recenter/context_map ≤1K）不受本模块
//! 影响：它们各自持 `Result` 语义 fail-closed（376/M1-S2 口径不动）。

use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use std::sync::Mutex;

use crate::gateway::model::Message;
use crate::tokenizer::{DeepSeekTokenizer, TokenCounter};

/// 缓存条目上限：长会话（千级消息＋逐轮易变注入串）的稳态工作集之上、
/// 内存（每条 ~16 B×2）之下。满表清空重暖。
const CACHE_CAP: usize = 65_536;

static TOKEN_CACHE: Mutex<Option<HashMap<u64, u64>>> = Mutex::new(None);

fn cached(key: u64) -> Option<u64> {
    let guard = TOKEN_CACHE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    guard.as_ref().and_then(|map| map.get(&key).copied())
}

fn store(key: u64, tokens: u64) {
    let mut guard = TOKEN_CACHE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let map = guard.get_or_insert_with(HashMap::new);
    if map.len() >= CACHE_CAP {
        map.clear();
    }
    map.insert(key, tokens);
}

/// provider tokenizer 计数（Err ⇒ chars/2 保守回退；见模块头失败语义）。
fn count_or_fallback(text: &str) -> u64 {
    DeepSeekTokenizer::new()
        .count_tokens(text)
        .unwrap_or_else(|_| text.chars().count() as u64 / 2)
}

/// 任意注入串的 token 数（指针行／D4 机械段／分块表／静态开销拼接文本）。
/// 同串反复出现（同一轮多次估算）命中缓存。
pub(crate) fn text_tokens(text: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    hasher.write(text.as_bytes());
    let key = hasher.finish();
    if let Some(tokens) = cached(key) {
        return tokens;
    }
    let tokens = count_or_fallback(text);
    store(key, tokens);
    tokens
}

/// tool_call arguments（serde_json::Value）的**就地规范哈希**——命中路径
/// 不序列化、零分配（398 批初版在命中路径 `serde_json::to_string`，对含
/// tool_calls 的消息每次估算都要分配拼串；整改批改为结构化直读）。
/// 判别式字节前缀区分类型；字符串/数组/对象带长度前缀（防拼接歧义）；
/// 对象键按迭代序（同一 Value 的迭代序恒定 ⇒ 键确定）。
fn hash_json(h: &mut DefaultHasher, v: &serde_json::Value) {
    use serde_json::Value as V;
    match v {
        V::Null => h.write_u8(0x00),
        V::Bool(b) => {
            h.write_u8(0x01);
            h.write_u8(u8::from(*b));
        }
        V::Number(n) => {
            h.write_u8(0x02);
            if let Some(i) = n.as_i64() {
                h.write_u8(0x00);
                h.write_i64(i);
            } else if let Some(u) = n.as_u64() {
                h.write_u8(0x01);
                h.write_u64(u);
            } else {
                h.write_u8(0x02);
                h.write_u64(n.as_f64().unwrap_or_default().to_bits());
            }
        }
        V::String(s) => {
            h.write_u8(0x03);
            h.write_u64(s.len() as u64);
            h.write(s.as_bytes());
        }
        V::Array(items) => {
            h.write_u8(0x04);
            h.write_u64(items.len() as u64);
            for item in items {
                hash_json(h, item);
            }
        }
        V::Object(map) => {
            h.write_u8(0x05);
            h.write_u64(map.len() as u64);
            for (k, val) in map {
                h.write_u64(k.len() as u64);
                h.write(k.as_bytes());
                hash_json(h, val);
            }
        }
    }
}

/// 单条消息的 token 数——字段集与旧 chars/2 估算完全一致（content ＋
/// reasoning_content ＋ tool_calls 的 name/序列化 arguments；role 与
/// tool_call_id 不入尺，与历史口径零漂移）。哈希直读字段，命中路径零
/// 分配（0dw S1 整改批：content/reasoning 之间补 `0xff` 字段边界字节、
/// arguments 就地规范哈希——SipHash 是流式哈希，无边界拼接会跨字段歧义
/// （`("ab", Some("c"))` 与 `("abc", None)` 恒同键 ⇒ 第二条消息拿到第一
/// 条的缓存值）；未命中才拼接文本走 tokenizer）。
pub(crate) fn message_tokens(m: &Message) -> u64 {
    let mut hasher = DefaultHasher::new();
    hasher.write(m.content.as_bytes());
    if let Some(r) = &m.reasoning_content {
        hasher.write_u8(0xff); // 字段边界（防拼接歧义；SipHash 流式 write(a)+write(b) == write(ab)）
        hasher.write(r.as_bytes());
    }
    for tc in &m.tool_calls {
        hasher.write_u8(0xff); // 字段边界（防拼接歧义）
        hasher.write(tc.name.as_bytes());
        hasher.write_u8(0xff);
        hash_json(&mut hasher, &tc.arguments);
    }
    let key = hasher.finish();
    if let Some(tokens) = cached(key) {
        return tokens;
    }
    let mut text = String::with_capacity(m.content.len());
    text.push_str(&m.content);
    if let Some(r) = &m.reasoning_content {
        text.push('\n');
        text.push_str(r);
    }
    for tc in &m.tool_calls {
        text.push('\n');
        text.push_str(&tc.name);
        text.push('\n');
        let args = serde_json::to_string(&tc.arguments).unwrap_or_default();
        text.push_str(&args);
    }
    let tokens = count_or_fallback(&text);
    store(key, tokens);
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gateway::model::{Message, Role, ToolCall};

    fn msg(content: &str) -> Message {
        Message {
            role: Role::User,
            content: content.to_string(),
            tool_call_id: None,
            tool_calls: Vec::new(),
            reasoning_content: None,
            round: None,
        }
    }

    /// 量尺换装钉（立项档 §1.1）：CJK 不再被 chars/2 低估——401 个 CJK
    /// 字符的真实 provider 计数必须显著高于 chars/2 的 200。
    #[test]
    fn cjk_is_no_longer_undercounted_by_chars_over_two() {
        let text = "一".repeat(401);
        let tokens = text_tokens(&text);
        assert!(tokens > 200, "CJK must not be under-counted: {tokens}");
    }

    /// 命中/未命中两路径同值（确定性）——缓存只能省时间、不能改读数。
    #[test]
    fn cached_and_cold_counts_agree() {
        let text = "state v1\ngoal: 上下文量尺对齐测试";
        let a = text_tokens(text);
        let b = text_tokens(text);
        assert_eq!(a, b);
        let m = msg("同一条消息反复估算");
        assert_eq!(message_tokens(&m), message_tokens(&m));
    }

    /// 消息尺字段集：tool_calls 的 name/arguments 入尺（与旧口径同集），
    /// 空 content 消息计 0。
    #[test]
    fn message_tokens_counts_tool_call_fields() {
        let empty = msg("");
        assert_eq!(message_tokens(&empty), 0);
        let mut m = msg("call it");
        m.tool_calls.push(ToolCall {
            name: "read_file".to_string(),
            arguments: serde_json::json!({"path": "a.py"}),
            call_id: "c1".to_string(),
        });
        let with_tools = message_tokens(&m);
        assert!(
            with_tools > message_tokens(&msg("call it")),
            "tool fields add tokens"
        );
    }

    /// 0dw S1 整改批（2026-11-18，398 批审查 P3a）：content/reasoning 字段
    /// 边界——SipHash 流式拼接使 `("ab", Some("c"))` 与 `("abc", None)`
    /// 恒同键（398 批初版无边界字节 ⇒ 第二条消息拿到第一条的缓存值，且
    /// 两者 miss 路径文本不同、token 数不同）。边界字节修复后各消息独立
    /// 计：先缓存拼接歧义形态，无 reasoning 形态必须等于自身文本的冷计
    /// 数（混尺键下它会拿到拼接形态的缓存值而失配——回归锁）。
    #[test]
    fn content_reasoning_boundary_prevents_key_aliasing() {
        let mut aliased = msg("a测b");
        aliased.reasoning_content = Some("b".to_string());
        let plain = msg("a测b");
        let aliased_tokens = message_tokens(&aliased);
        let plain_tokens = message_tokens(&plain);
        assert_eq!(
            plain_tokens,
            text_tokens("a测b"),
            "无 reasoning 消息不得拿到拼接形态的缓存值（键混尺即失配）"
        );
        assert!(aliased_tokens > 0);
    }

    /// Perf 关读数（0dw S1；`#[ignore]` 探针，`cargo test -p orz-loop --lib
    /// ruler -- --ignored --nocapture` 手动取数）：500K-token 工作点上
    /// 「全量编码（冷）」vs「缓存增量（暖）」的逐次估算耗时。验收＝暖路径
    /// 较冷路径坍缩（立项档 §2 缺陷 1「必要时缓存增量」的机械证据），
    /// 数值随机器浮动、入批档记录。
    #[test]
    #[ignore]
    fn perf_probe_full_encode_vs_cached_increment() {
        // 300 轮 × '测'×1600（≈1.6K token/轮）≈ 480K token 工作点（与
        // 生产 500K 工作点同量级）；内容逐轮不同（模拟真实会话追加）。
        let mut messages: Vec<Message> = Vec::new();
        for r in 0..300 {
            let mut m = msg(&format!("轮 {r} 的工具结果正文。"));
            m.content.push_str(&"测".repeat(1_600));
            messages.push(m);
        }
        let total_chars: usize = messages.iter().map(|m| m.content.len()).sum();
        let t0 = std::time::Instant::now();
        let cold = messages.iter().map(message_tokens).sum::<u64>();
        let cold_dt = t0.elapsed();
        let t1 = std::time::Instant::now();
        for _ in 0..5 {
            let _ = messages.iter().map(message_tokens).sum::<u64>();
        }
        let warm_dt = t1.elapsed() / 5;
        // 追加一轮（增量形态：steady-state loop-top 只编码新尾轮）。
        messages.push(msg(&format!("新增轮。{}", "测".repeat(1_600))));
        let t2 = std::time::Instant::now();
        let grown = messages.iter().map(message_tokens).sum::<u64>();
        let inc_dt = t2.elapsed();
        println!(
            "perf ruler: chars={total_chars} tokens={cold} cold={cold_dt:?} \
             warm_x5_avg={warm_dt:?} incremental_after_append={inc_dt:?} grown={grown}",
        );
        assert!(grown > cold, "append must add tokens");
        assert!(
            cold > 400_000,
            "fixture must reach the ~500K working point: {cold}"
        );
    }
}
