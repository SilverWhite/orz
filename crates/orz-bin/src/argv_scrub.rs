//! 0cg (2026-10-01) 墙钟输入通道的进程可见面擦写。
//!
//! `--max-wallclock <sec>` / `ORZ_MAX_WALLCLOCK` 是竞赛敏感量：派生 shell
//! 里 `ps`/`pgrep` 可读 `/proc/<pid>/cmdline`（137 批三通道核查：14 卷命
//! 中），`env`/`/proc/*/environ` 反映继承环境。主流程（main.rs）在启动期
//! 把取值收进进程内静态并 `remove_var`；本模块再对 Linux 的 argv/environ
//! 内存区做 best-effort 零化——`/proc/self/stat` 给出两区边界（arg 48–49、
//! env 50–51 字段），`/proc/self/mem` 原位写回。
//!
//! 纪律：**fail-soft**——任何一步不可达（无 /proc、权限收紧、解析失败）
//! 即静默跳过；擦写是混淆面不是门，绝不能因它破坏启动。非 Linux 平台
//! （Windows GetCommandLine 无等价原位语义）不做擦写，登记为已知边界。

/// 纯函数：在 cmdline 区字节里定位 `flag` token 及其后随 value token 的
/// （start, end）区间——与 main.rs 的解析形态一致（分离 token，不支持
/// `--flag=value`）。NUL 分隔符保留不动（只清 token 本体，cmdline 里的
/// 该两个 token 变为空串，`ps`/`pgrep` 不再显示）。
pub fn zero_target_ranges(region: &[u8], flag: &str) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let tokens = token_spans(region);
    for (i, (start, end)) in tokens.iter().enumerate() {
        if &region[*start..*end] == flag.as_bytes() {
            out.push((*start, *end));
            if let Some(&(vstart, vend)) = tokens.get(i + 1) {
                out.push((vstart, vend));
            }
            break;
        }
    }
    out
}

/// 纯函数：在 environ 区字节里定位 `KEY=...` 条目的区间（只清该条目，
/// 不动其它条目与其分隔符）。
pub fn zero_env_entry_ranges(region: &[u8], key: &str) -> Vec<(usize, usize)> {
    let prefix = format!("{key}=");
    token_spans(region)
        .into_iter()
        .filter(|(start, end)| region[*start..*end].starts_with(prefix.as_bytes()))
        .collect()
}

/// 纯函数：把区间内字节置零（分隔符不受影响）。
pub fn zero_ranges(region: &mut [u8], ranges: &[(usize, usize)]) {
    for (start, end) in ranges {
        region[*start..*end].fill(0);
    }
}

/// NUL 分隔 token 的 spans（空 token 跳过——零化后的残留空位不再是 token）。
fn token_spans(region: &[u8]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut start = 0;
    for (i, b) in region.iter().enumerate() {
        if *b == 0 {
            if i > start {
                out.push((start, i));
            }
            start = i + 1;
        }
    }
    if region.len() > start {
        out.push((start, region.len()));
    }
    out
}

/// 解析 `/proc/self/stat`：comm 可含空格/括号，从最后一个 `)` 后取字段；
/// `fields[0]` = 字段 3（state），故字段 N ⇒ `fields[N - 3]`。返回
/// `(arg_start, arg_end, env_start, env_end)`（字段 48–51）。
pub fn proc_region_bounds(stat: &str) -> Option<(usize, usize, usize, usize)> {
    let close = stat.rfind(')')?;
    let fields: Vec<&str> = stat[close + 1..].split_whitespace().collect();
    let arg_start: usize = fields.get(45)?.parse().ok()?;
    let arg_end: usize = fields.get(46)?.parse().ok()?;
    let env_start: usize = fields.get(47)?.parse().ok()?;
    let env_end: usize = fields.get(48)?.parse().ok()?;
    Some((arg_start, arg_end, env_start, env_end))
}

/// Linux 启动期一次调用：零化 argv 区的 `flag`(+value) token 与 environ 区
/// 的 `KEY=` 条目。best-effort——读不到 /proc/self/{stat,mem} 或写不回时
/// 静默返回（调用方已先 `remove_var`，继承面已收口；本层只影响
/// `/proc/*/cmdline`、`/proc/<orz-pid>/environ` 的外部读数）。
#[cfg(target_os = "linux")]
pub fn scrub_linux(flag: &str, env_key: &str) {
    use std::io::{Read, Seek, SeekFrom, Write};

    let Ok(stat) = std::fs::read_to_string("/proc/self/stat") else {
        return;
    };
    let Some((arg_start, arg_end, env_start, env_end)) = proc_region_bounds(&stat) else {
        return;
    };
    let Ok(mut mem) = std::fs::File::open("/proc/self/mem") else {
        return;
    };
    let mut scrub_region = |start: usize, end: usize, pick: fn(&[u8], &str) -> Vec<(usize, usize)>,
                            needle: &str| {
        if end <= start {
            return;
        }
        let mut buf = vec![0u8; end - start];
        if mem.seek(SeekFrom::Start(start as u64)).is_err() {
            return;
        }
        if mem.read_exact(&mut buf).is_err() {
            return;
        }
        let ranges = pick(&buf, needle);
        if ranges.is_empty() {
            return;
        }
        zero_ranges(&mut buf, &ranges);
        if mem.seek(SeekFrom::Start(start as u64)).is_err() {
            return;
        }
        let _ = mem.write_all(&buf);
    };
    scrub_region(arg_start, arg_end, zero_target_ranges, flag);
    scrub_region(env_start, env_end, zero_env_entry_ranges, env_key);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// NUL 分隔的 token 拼接（避免 `\0` 与数字相邻被读作八进制转义）。
    fn region(parts: &[&str]) -> Vec<u8> {
        let mut out = Vec::new();
        for part in parts {
            out.extend_from_slice(part.as_bytes());
            out.push(0);
        }
        out
    }

    #[test]
    fn zeroing_removes_flag_and_value_tokens_only() {
        let mut region = region(&["--real", "--max-wallclock", "3600", "--stdio"]);
        let ranges = zero_target_ranges(&region, "--max-wallclock");
        assert_eq!(ranges.len(), 2);
        zero_ranges(&mut region, &ranges);
        let text = String::from_utf8_lossy(&region).to_string();
        assert!(text.contains("--real") && text.contains("--stdio"));
        assert!(!text.contains("3600") && !text.contains("max-wallclock"));
        assert!(zero_target_ranges(&region, "--max-wallclock").is_empty());
    }

    #[test]
    fn flag_without_following_token_zeroes_flag_only() {
        let region = region(&["--max-wallclock"]);
        let ranges = zero_target_ranges(&region, "--max-wallclock");
        assert_eq!(ranges.len(), 1);
    }

    #[test]
    fn env_entry_zeroing_is_selective() {
        let mut region = region(&["PATH=/bin", "ORZ_MAX_WALLCLOCK=3600", "HOME=/root"]);
        let ranges = zero_env_entry_ranges(&region, "ORZ_MAX_WALLCLOCK");
        assert_eq!(ranges.len(), 1);
        zero_ranges(&mut region, &ranges);
        let text = String::from_utf8_lossy(&region).to_string();
        assert!(text.contains("PATH=/bin") && text.contains("HOME=/root"));
        assert!(!text.contains("3600"));
    }

    #[test]
    fn proc_region_bounds_parses_comm_with_spaces_and_parens() {
        // comm=「a b(c)」含空格与嵌套括号；`fields[0]` = state（字段 3），
        // 随后 4..=47 填字段 4–47，末四位即字段 48–51。
        let mut stat = String::from("1234 (a b(c)) R");
        for i in 4..48 {
            stat.push(' ');
            stat.push_str(&i.to_string());
        }
        stat.push_str(" 1400000000 1400000100 1400000200 1400000300 0 0");
        let (a, b, c, d) = proc_region_bounds(&stat).expect("parses");
        assert_eq!(
            (a, b, c, d),
            (
                1_400_000_000,
                1_400_000_100,
                1_400_000_200,
                1_400_000_300
            )
        );
    }
}
