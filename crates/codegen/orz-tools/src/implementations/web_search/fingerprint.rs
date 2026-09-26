//! 0bs ⑩（2026-09-26 用户令 + 裁决）：**指纹伪装抓取 sidecar**。
//!
//! 背景：三引擎里的百度/360 对裸 reqwest 的 TLS/HTTP2 指纹有反爬（百度
//! 直接返回验证页）。裁决＝走 **curl_cffi 子进程**（本机已实证 0.16.3
//! 可用），固定内嵌脚本、`impersonate="chrome"`；python 或 curl_cffi
//! 缺失、超时、异常时**如实回落** reqwest（调用方不把回落伪装成指纹
//! 成功）。理由＝不新增 Rust 依赖、不动 Cargo/重建面；Rust `wreq`
//! （自带浏览器指纹）记为目标形态，留待后续批。
//!
//! 契约：`fetch(url, timeout) -> Option<(body, final_url)>`——`None` 表示
//! sidecar 不可用/失败（调用方回落）；`Some` 的 body 就是响应报文
//! （f 落临时文件后读回，stdout 只回最终 URL，避免大页在管道容量上打转）。

use std::sync::LazyLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

/// 开关（`auto`（缺省）/ 其余按 `off` 语义断开指纹优先；回落链不受影响）。
pub const ENV_FINGERPRINT: &str = "ORZ_RETRIEVAL_FINGERPRINT";
/// python 可执行名（缺省 `python`；Windows 上常为 `py`/`python3`）。
pub const ENV_PYTHON: &str = "ORZ_RETRIEVAL_PYTHON";

/// 内嵌 sidecar 脚本（`python -c` 直传）：argv[1]=url、argv[2]=落盘路径；
/// stdout 回 `<status>\t<final_url>`；退出码 0=成功、3=curl_cffi 缺失、
/// 4=请求失败（DNS/超时/连接被拒——由调用方回落 reqwest 取得稳定 cause）。
const SIDECAR_SCRIPT: &str = r#"
import sys
url, out = sys.argv[1], sys.argv[2]
try:
    from curl_cffi import requests
except Exception:
    sys.exit(3)
try:
    r = requests.get(url, impersonate="chrome", timeout=20, allow_redirects=True)
except Exception:
    sys.exit(4)
with open(out, "wb") as f:
    f.write(r.content)
sys.stdout.write("%d\t%s" % (r.status_code, r.url or ""))
"#;

/// 指纹优先是否启用（env 缺省 `auto`=启用；`off`/`0`/`false`/`no` 关闭）。
pub fn enabled() -> bool {
    std::env::var(ENV_FINGERPRINT)
        .map(|v| {
            let v = v.trim().to_ascii_lowercase();
            !matches!(v.as_str(), "off" | "0" | "false" | "no")
        })
        .unwrap_or(true)
}

/// python + curl_cffi 可用性探测（进程内缓存一次；探测失败缓存 `None`，
/// 不反复 spawn）。
fn python_bin() -> Option<&'static str> {
    static PROBED: LazyLock<Option<String>> = LazyLock::new(|| {
        let candidates: Vec<String> = std::env::var(ENV_PYTHON)
            .ok()
            .into_iter()
            .chain(["python", "python3", "py"].into_iter().map(str::to_string))
            .collect();
        for candidate in candidates {
            let probe = std::process::Command::new(&candidate)
                .arg("-c")
                .arg("import curl_cffi")
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();
            if matches!(probe, Ok(status) if status.success()) {
                return Some(candidate);
            }
        }
        None
    });
    PROBED.as_deref()
}

/// 指纹抓取：`Some((body, final_url))`＝成功；`None`＝不可用/失败（调用方
/// 回落 reqwest，且按回落路径如实报告 cause）。
pub async fn fetch(url: &str, timeout: Duration) -> Option<(String, String)> {
    let python = python_bin()?;
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let out_path = std::env::temp_dir().join(format!(
        "orz-fingerprint-{}-{}.body",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    let mut child = tokio::process::Command::new(python)
        .arg("-c")
        .arg(SIDECAR_SCRIPT)
        .arg(url)
        .arg(&out_path)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .ok()?;
    let stdout = child.stdout.take();
    let status = match tokio::time::timeout(timeout, child.wait()).await {
        Ok(Ok(status)) => status,
        // 超时/等待失败：杀掉子进程，回落（不豁免预算）。
        Ok(Err(_)) | Err(_) => {
            let _ = child.kill().await;
            let _ = tokio::fs::remove_file(&out_path).await;
            return None;
        }
    };
    if !status.success() {
        let _ = tokio::fs::remove_file(&out_path).await;
        return None;
    }
    let mut final_url = String::new();
    if let Some(mut stdout) = stdout {
        use tokio::io::AsyncReadExt as _;
        let _ = stdout.read_to_string(&mut final_url).await;
    }
    let final_url = final_url.trim().to_string();
    // `<status>\t<final_url>`：非 2xx 视为"不可用"→ 回落 reqwest（由它给出
    // 稳定的 `network_error` 语义；sidecar 不做状态码翻译）。
    let (status, final_url) = match final_url.split_once('\t') {
        Some((status, url)) => (status.trim().to_string(), url.trim().to_string()),
        None => (String::new(), final_url),
    };
    let status_ok = status
        .parse::<u16>()
        .map(|code| (200..300).contains(&code))
        .unwrap_or(false);
    if !status_ok {
        let _ = tokio::fs::remove_file(&out_path).await;
        return None;
    }
    let body = tokio::fs::read(&out_path).await.ok()?;
    let _ = tokio::fs::remove_file(&out_path).await;
    let body = String::from_utf8_lossy(&body).into_owned();
    Some((
        body,
        if final_url.starts_with("http") {
            final_url
        } else {
            url.to_string()
        },
    ))
}

#[cfg(test)]
mod tests {
    /// 开关语义：缺省启用；显式 off 家族关闭（回落链不受影响）。
    #[test]
    fn enabled_switch_reads_env_semantics() {
        // 直接验证判定函数的内联语义（进程 env 在并行测试下不可安全改写，
        // 与 local_segmented 的 env 缝同纪律）。
        let parse = |raw: Option<&str>| match raw {
            Some(v) => {
                let v = v.trim().to_ascii_lowercase();
                !matches!(v.as_str(), "off" | "0" | "false" | "no")
            }
            None => true,
        };
        assert!(parse(None));
        assert!(parse(Some("auto")));
        assert!(!parse(Some("off")));
        assert!(!parse(Some("FALSE")));
    }
}
