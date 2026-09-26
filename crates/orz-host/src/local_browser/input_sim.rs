//! 本地浏览器车道 · 输入拟真 v1（0bs ⑪，2026-09-26；用户令第 3/4 令 §4.6④
//! 「输入拟真＝统一固定标准、单一源」）。
//!
//! 单一源＝本模块常数表。模型下发输入指令（`browser_control` 的
//! `type`/`key`/`click`/`scroll` 动作）后，机械层按本标准自动施加拟真时序
//! ——逐字符延迟、句间停顿、提交前停顿、错字-修正回路、鼠标轨迹、滚动
//! 节奏、动作后停留、会话 pacing——模型不可调参、也不感知细节（用户令：
//! 「常驻，由机械层直接机械做……模型下发指令后机械层自动做对应的输入动作
//! 拟真，避免拦截和风控」）。
//!
//! 一切随机由 `seed` 派生（同一 seed ⇒ 同一计划），测试与 journal 可复核；
//! 落点执行在 `cdp.rs`（把 [`SimEvent`] 计划翻译成 CDP `Input.*` 调用）。
//!
//! v1 参数（可引用人类操作数据的保守中位；调研档 §4.6）：
//! - 逐字符键入 200ms ± 40%（120–280ms）；
//! - 句读符（`.。!！?？`）后 1.5s 句间停顿；
//! - 提交（Enter）前 0.6s 停顿；
//! - 2% 逐字符错字率：错字后 120ms 回退 Backspace 修正；
//! - 鼠标轨迹 WindMouse（gravity=9 / wind=3 / max_step=10px / 阈值 1.5px）
//!   目标速度 800px/s；按下-抬起驻留 60–140ms；
//! - 滚动 110px／20ms（wheel deltaY 累进）；
//! - 每个输入动作完成后 0.8s 停留（页面反应窗）；
//! - 会话内两个输入动作间 pacing ≥5s（0–50% jitter；沿用 `serp.rs`
//!   反污染前身口径）。

// —— v1 固定标准（单一源；勿在别处散落常数） ——
pub(crate) const TYPE_CHAR_MS: u64 = 200;
pub(crate) const TYPE_CHAR_JITTER_PCT: u64 = 40;
pub(crate) const SENTENCE_GAP_MS: u64 = 1_500;
pub(crate) const PRE_SUBMIT_MS: u64 = 600;
pub(crate) const TYPO_RATE_PERMILLE: u64 = 20; // 2%
pub(crate) const TYPO_CORRECTION_MS: u64 = 120;
pub(crate) const MOUSE_SPEED_PX_PER_S: f64 = 800.0;
pub(crate) const MOUSE_PRESS_MIN_MS: u64 = 60;
pub(crate) const MOUSE_PRESS_MAX_MS: u64 = 140;
pub(crate) const WIND_GRAVITY: f64 = 9.0;
pub(crate) const WIND_WIND: f64 = 3.0;
pub(crate) const WIND_MAX_STEP: f64 = 10.0;
pub(crate) const WIND_THRESHOLD: f64 = 1.5;
pub(crate) const WHEEL_DELTA_PX: i64 = 110;
pub(crate) const WHEEL_INTERVAL_MS: u64 = 20;
pub(crate) const SETTLE_MS: u64 = 800;
pub(crate) const SESSION_PACING_MS: u64 = 5_000;
pub(crate) const SESSION_PACING_JITTER_PCT: u64 = 50;
/// 移动前的「反应窗」（按下前的微停顿，避免瞬移式点击）。
pub(crate) const PRE_MOVE_MS: u64 = 120;

/// Deterministic SplitMix64 — 不引入新依赖；同 seed 同计划（可核性）。
pub(crate) struct SimRng(u64);

impl SimRng {
    pub(crate) fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `[lo, hi]` (inclusive); `lo == hi` returns `lo`.
    pub(crate) fn range(&mut self, lo: u64, hi: u64) -> u64 {
        if hi <= lo {
            return lo;
        }
        lo + self.next_u64() % (hi - lo + 1)
    }

    /// `base` ± `pct`%（整型线性抖动）。
    pub(crate) fn jitter_pct(&mut self, base: u64, pct: u64) -> u64 {
        let delta = base * pct / 100;
        self.range(base.saturating_sub(delta), base + delta)
    }

    /// 千分位概率判定。
    pub(crate) fn chance_permille(&mut self, permille: u64) -> bool {
        if permille == 0 {
            return false;
        }
        self.range(1, 1_000) <= permille
    }

    /// `[0, 1)` 均匀浮点。
    pub(crate) fn f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// 一次输入拟真计划里的原子步骤（cdp.rs 逐条翻译成 CDP 调用）。
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum SimEvent {
    /// 键入一个字符（`Input.dispatchKeyEvent` keyDown(text) + keyUp）。
    KeyChar(char),
    /// 按键（Enter/Tab/Escape/方向键等）；`key` 用 CDP 键名（`Enter`…）。
    KeyDown(String),
    KeyUp(String),
    /// 回退键（错字修正）。
    Backspace,
    MouseMove(f64, f64),
    MouseDown(f64, f64),
    MouseUp(f64, f64),
    /// 滚轮（像素；`delta_y` 负值 = 向上）。
    Wheel { dx: i64, dy: i64 },
    Sleep(u64),
}

/// 会话 pacing：同会话两个输入动作之间的最小间隔——**≥5s**，上限侧加
/// 0–50% jitter（5.0–7.5s；下限恒为 5s，不向下漂）。
pub(crate) fn session_pacing_delay(seed: u64) -> u64 {
    let mut rng = SimRng::new(seed ^ 0x5E55_10AC);
    let upper = SESSION_PACING_MS * (100 + SESSION_PACING_JITTER_PCT) / 100;
    rng.range(SESSION_PACING_MS, upper)
}

fn char_delay(rng: &mut SimRng) -> u64 {
    rng.jitter_pct(TYPE_CHAR_MS, TYPE_CHAR_JITTER_PCT)
}

fn is_sentence_end(ch: char) -> bool {
    matches!(ch, '.' | '。' | '!' | '！' | '?' | '？')
}

/// 错字替代字符（键盘邻位近似：字母取 QWERTY 右邻，其余原样重复）。
fn typo_char(ch: char) -> char {
    const ROWS: &[&str] = &["qwertyuiop", "asdfghjkl", "zxcvbnm"];
    let lower = ch.to_ascii_lowercase();
    for row in ROWS {
        if let Some(idx) = row.find(lower) {
            let next = row.as_bytes()[(idx + 1).min(row.len() - 1)] as char;
            return if ch.is_ascii_uppercase() {
                next.to_ascii_uppercase()
            } else {
                next
            };
        }
    }
    ch
}

/// `type` 动作计划：逐字符键入（含句间停顿/错字-修正/提交前停顿/收尾停留）。
pub(crate) fn plan_type_text(text: &str, submit: bool, seed: u64) -> Vec<SimEvent> {
    let mut rng = SimRng::new(seed);
    let mut plan = Vec::new();
    for ch in text.chars() {
        if rng.chance_permille(TYPO_RATE_PERMILLE) {
            let wrong = typo_char(ch);
            if wrong != ch {
                plan.push(SimEvent::KeyChar(wrong));
                plan.push(SimEvent::Sleep(correction_delay(&mut rng)));
                plan.push(SimEvent::Backspace);
                plan.push(SimEvent::Sleep(char_delay(&mut rng)));
            }
        }
        plan.push(SimEvent::KeyChar(ch));
        plan.push(SimEvent::Sleep(char_delay(&mut rng)));
        if is_sentence_end(ch) {
            plan.push(SimEvent::Sleep(rng.jitter_pct(SENTENCE_GAP_MS, 20)));
        }
    }
    if submit {
        plan.push(SimEvent::Sleep(rng.jitter_pct(PRE_SUBMIT_MS, 20)));
        plan.push(SimEvent::KeyDown("Enter".to_string()));
        plan.push(SimEvent::KeyUp("Enter".to_string()));
    }
    plan.push(SimEvent::Sleep(rng.jitter_pct(SETTLE_MS, 20)));
    plan
}

fn correction_delay(rng: &mut SimRng) -> u64 {
    rng.jitter_pct(TYPO_CORRECTION_MS, 20)
}

/// `key` 动作计划：单键按下-抬起 + 收尾停留。
pub(crate) fn plan_key(key: &str, seed: u64) -> Vec<SimEvent> {
    let mut rng = SimRng::new(seed);
    vec![
        SimEvent::KeyDown(key.to_string()),
        SimEvent::Sleep(rng.jitter_pct(80, 40)),
        SimEvent::KeyUp(key.to_string()),
        SimEvent::Sleep(rng.jitter_pct(SETTLE_MS, 20)),
    ]
}

/// `click` 动作计划：WindMouse 轨迹（从 `from` 到 `to`）→ 按下 → 驻留 →
/// 抬起 → 收尾停留。轨迹点间 sleep 由 800px/s 与步长推出。
pub(crate) fn plan_click(
    from: (f64, f64),
    to: (f64, f64),
    seed: u64,
) -> Vec<SimEvent> {
    let mut rng = SimRng::new(seed);
    let mut plan = vec![SimEvent::Sleep(rng.jitter_pct(PRE_MOVE_MS, 30))];
    for (x, y) in wind_mouse_path(from, to, &mut rng) {
        let prev = plan
            .iter()
            .rev()
            .find_map(|e| match e {
                SimEvent::MouseMove(px, py) => Some((*px, *py)),
                _ => None,
            })
            .unwrap_or(from);
        let step = ((x - prev.0).powi(2) + (y - prev.1).powi(2)).sqrt();
        let ms = (step / MOUSE_SPEED_PX_PER_S * 1_000.0).round().max(1.0) as u64;
        plan.push(SimEvent::MouseMove(x, y));
        plan.push(SimEvent::Sleep(ms));
    }
    plan.push(SimEvent::MouseDown(to.0, to.1));
    plan.push(SimEvent::Sleep(
        rng.range(MOUSE_PRESS_MIN_MS, MOUSE_PRESS_MAX_MS),
    ));
    plan.push(SimEvent::MouseUp(to.0, to.1));
    plan.push(SimEvent::Sleep(rng.jitter_pct(SETTLE_MS, 20)));
    plan
}

/// `scroll` 动作计划：按 110px／20ms 节奏推滚轮（余量最后一格）。
pub(crate) fn plan_scroll(dx: i64, dy: i64, seed: u64) -> Vec<SimEvent> {
    let mut rng = SimRng::new(seed);
    let mut plan = Vec::new();
    let mut rem_x = dx;
    let mut rem_y = dy;
    while rem_x != 0 || rem_y != 0 {
        let step_x = rem_x.clamp(-WHEEL_DELTA_PX, WHEEL_DELTA_PX);
        let step_y = rem_y.clamp(-WHEEL_DELTA_PX, WHEEL_DELTA_PX);
        rem_x -= step_x;
        rem_y -= step_y;
        plan.push(SimEvent::Wheel {
            dx: step_x,
            dy: step_y,
        });
        plan.push(SimEvent::Sleep(WHEEL_INTERVAL_MS));
    }
    plan.push(SimEvent::Sleep(rng.jitter_pct(SETTLE_MS, 20)));
    plan
}

/// WindMouse 轨迹（gravity/wind/max_step/阈值 = v1 常数；仅返回路径点，
/// 不含起点）。速度语义由调用方按 `MOUSE_SPEED_PX_PER_S` 折算为 sleep。
fn wind_mouse_path(from: (f64, f64), to: (f64, f64), rng: &mut SimRng) -> Vec<(f64, f64)> {
    const SQRT3: f64 = 1.732_050_807_568_877_2;
    const SQRT5: f64 = 2.236_067_977_499_79;
    let (mut x, mut y) = from;
    let (mut vx, mut vy) = (0.0f64, 0.0f64);
    let (mut wx, mut wy) = (0.0f64, 0.0f64);
    let mut path = Vec::new();
    for _ in 0..4_096 {
        let dx = to.0 - x;
        let dy = to.1 - y;
        let dist = (dx * dx + dy * dy).sqrt();
        if dist < WIND_THRESHOLD {
            break;
        }
        if dist >= WIND_MAX_STEP {
            let wind_mag = WIND_WIND.min(dist);
            wx = wx / SQRT3 + (2.0 * rng.f64() - 1.0) * wind_mag / SQRT5;
            wy = wy / SQRT3 + (2.0 * rng.f64() - 1.0) * wind_mag / SQRT5;
            vx += dx / dist * WIND_GRAVITY;
            vy += dy / dist * WIND_GRAVITY;
        } else {
            vx += dx / dist * 4.0;
            vy += dy / dist * 4.0;
        }
        let v_mag = (vx * vx + vy * vy).sqrt();
        let step = v_mag.min(WIND_MAX_STEP);
        if v_mag > 0.0 {
            vx = (vx / v_mag) * step;
            vy = (vy / v_mag) * step;
        }
        x += vx;
        y += vy;
        path.push((x, y));
    }
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_is_seed_deterministic() {
        let a = plan_type_text("hello world, how are you?", true, 42);
        let b = plan_type_text("hello world, how are you?", true, 42);
        assert_eq!(a, b);
        let c = plan_type_text("hello world, how are you?", true, 43);
        assert_ne!(a, c);
    }

    #[test]
    fn char_delays_stay_within_v1_jitter_band() {
        let mut rng = SimRng::new(7);
        let mut lo = u64::MAX;
        let mut hi = 0;
        for _ in 0..2_000 {
            let d = char_delay(&mut rng);
            assert!((120..=280).contains(&d), "char delay {d} outside v1 band");
            lo = lo.min(d);
            hi = hi.max(d);
        }
        // 抖动确实展开（不是常数）：两端各自接近 ±40% 界。
        assert!(lo <= 150, "lower band not reached: {lo}");
        assert!(hi >= 250, "upper band not reached: {hi}");
    }

    #[test]
    fn typo_rate_is_about_two_percent_with_correction_pairs() {
        let text: String = std::iter::repeat_n('a', 5_000).collect();
        let plan = plan_type_text(&text, false, 9);
        let backspaces = plan
            .iter()
            .filter(|e| matches!(e, SimEvent::Backspace))
            .count();
        // 2% of 5000 = 100 typos; generous band for the deterministic seed.
        assert!(
            (40..=170).contains(&backspaces),
            "typo count {backspaces} not near 2%"
        );
        // 每个错字都紧跟一次修正（错字字符 + 修正延迟 + Backspace 成对）。
        for (idx, ev) in plan.iter().enumerate() {
            if let SimEvent::Backspace = ev {
                assert!(idx >= 2, "backspace without preceding typo char");
                assert!(matches!(
                    plan[idx - 1],
                    SimEvent::Sleep(ms) if ms <= 200
                ));
            }
        }
    }

    #[test]
    fn type_plan_carries_sentence_gap_and_submit_pause() {
        let plan = plan_type_text("Hi. ok", true, 5);
        let gaps = plan
            .iter()
            .filter(|e| matches!(e, SimEvent::Sleep(ms) if *ms >= 1_200))
            .count();
        assert!(gaps >= 1, "sentence gap missing: {plan:?}");
        assert!(
            plan.iter().any(|e| matches!(e, SimEvent::KeyDown(k) if k == "Enter")),
            "submit key missing"
        );
    }

    #[test]
    fn wind_mouse_path_converges_with_bounded_steps() {
        let mut rng = SimRng::new(11);
        let from = (0.0, 0.0);
        let to = (500.0, 300.0);
        let path = wind_mouse_path(from, to, &mut rng);
        assert!(path.len() >= 20, "path too coarse: {} points", path.len());
        let mut prev = from;
        for p in &path {
            let step = ((p.0 - prev.0).powi(2) + (p.1 - prev.1).powi(2)).sqrt();
            assert!(step <= WIND_MAX_STEP + 1e-6, "step too long: {step}");
            prev = *p;
        }
        let last = *path.last().unwrap();
        let rest = ((to.0 - last.0).powi(2) + (to.1 - last.1).powi(2)).sqrt();
        assert!(rest < WIND_THRESHOLD + WIND_MAX_STEP, "did not converge: {rest}");
    }

    #[test]
    fn click_plan_carries_press_dwell_and_settle() {
        let plan = plan_click((10.0, 10.0), (210.0, 110.0), 3);
        assert!(plan.iter().any(|e| matches!(e, SimEvent::MouseDown(..))));
        assert!(plan.iter().any(|e| matches!(e, SimEvent::MouseUp(..))));
        let dwell = plan
            .iter()
            .find_map(|e| match e {
                SimEvent::Sleep(ms) => Some(*ms),
                _ => None,
            });
        assert!(dwell.is_some());
        assert!(
            plan.iter().any(|e| matches!(e, SimEvent::Sleep(ms) if (60..=140).contains(ms))),
            "press dwell missing"
        );
        // 轨迹总时长 ≈ 距离/800px/s（240px ≈ 300ms，容差放宽 2×）。
        let total: u64 = plan
            .iter()
            .filter_map(|e| match e {
                SimEvent::Sleep(ms) => Some(*ms),
                _ => None,
            })
            .sum();
        assert!((250..=2_000).contains(&total), "click duration {total}ms off");
    }

    #[test]
    fn scroll_plan_chunks_at_110px_per_20ms() {
        let plan = plan_scroll(0, -240, 2);
        let wheels: Vec<(i64, i64)> = plan
            .iter()
            .filter_map(|e| match e {
                SimEvent::Wheel { dx, dy } => Some((*dx, *dy)),
                _ => None,
            })
            .collect();
        assert_eq!(wheels.len(), 3, "240 / 110 should chunk into 3 wheels");
        assert_eq!(
            wheels.iter().map(|(_, dy)| *dy).sum::<i64>(),
            -240,
            "chunks must sum to the requested delta"
        );
        assert!(wheels.iter().all(|(_, dy)| dy.abs() <= WHEEL_DELTA_PX));
    }

    #[test]
    fn session_pacing_stays_within_jitter_band() {
        for seed in 0..500 {
            let d = session_pacing_delay(seed);
            assert!(
                (5_000..=7_500).contains(&d),
                "pacing {d} outside 5s..7.5s band"
            );
        }
    }
}
