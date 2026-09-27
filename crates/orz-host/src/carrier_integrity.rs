//! 运行时载体完整性自检（0bw②，2026-09-27；`WRITE_CONTROL_MECHANICAL_
//! DESIGN_2026-09-26.md` §6「载体完整性自检：运行时形态」）。
//!
//! **形态**：发布打包时在载体目录生成 `carrier-manifest.json`（逐文件
//! sha256＋长度）；启动时若清单在位则逐条复验，失配/缺失 ⇒ 明确告警横幅。
//! **失败姿态＝审计腿**（设计 §1「高阻力＋强审计、非绝对保证」）：清单
//! 缺席（开发树/未打包形态）⇒ 静默跳过；清单在位但失配 ⇒ stderr 告警
//! ＋tracing，**不拒绝启动**（L1 锁死面已阻止模型写载体；本自检是检测，
//! 不能防住「能改二进制的攻击者删清单」——对外措辞守三档纪律）。
//!
//! 清单生成器＝父仓 `scripts/generate_carrier_manifest.py`（发布/重建批
//! 在打包后调用；设计 §6 注：生成时机涉发布链，随载体重建批实施）。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use orz_assurance::journal::sha256_hex;

/// 清单文件名（载体目录平铺；生成器与运行时同一常量语义）。
pub const MANIFEST_FILENAME: &str = "carrier-manifest.json";

/// 清单 `kind` 固定值——形态哨（生成器写入，运行时校验）。
const MANIFEST_KIND: &str = "orz-carrier-manifest";

/// 单条清单目：文件 sha256 与字节长度。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ManifestEntry {
    pub sha256: String,
    pub size_bytes: u64,
}

/// 载体清单（生成器产出；`kind` 形态哨防拿错文件当清单）。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CarrierManifest {
    pub kind: String,
    pub version: String,
    pub generated_at: String,
    pub entries: BTreeMap<String, ManifestEntry>,
}

/// 自检结论。`Violated` 携带逐文件差异行（告警横幅直接消费）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntegrityReport {
    /// 清单不在位（开发树/未打包形态）——静默跳过。
    NoManifest,
    /// 清单在位且全部目逐字节一致。
    Clean { checked: usize },
    /// 清单在位但存在失配/缺失文件。
    Violated { findings: Vec<String> },
}

/// 载体清单路径（`current_exe()` 父目录平铺）。
pub fn manifest_path(install_dir: &Path) -> PathBuf {
    install_dir.join(MANIFEST_FILENAME)
}

/// 读入并做形态校验（`kind` 不符 ⇒ 视为无清单：宁可不检，不可误报）。
pub fn load_manifest(install_dir: &Path) -> Option<CarrierManifest> {
    let bytes = std::fs::read(manifest_path(install_dir)).ok()?;
    let manifest: CarrierManifest = serde_json::from_slice(&bytes).ok()?;
    if manifest.kind != MANIFEST_KIND {
        return None;
    }
    Some(manifest)
}

/// 对 `install_dir` 执行完整性自检（清单在位时）。
pub fn verify(install_dir: &Path) -> IntegrityReport {
    let Some(manifest) = load_manifest(install_dir) else {
        return IntegrityReport::NoManifest;
    };
    let mut findings = Vec::new();
    let mut checked = 0usize;
    for (rel, entry) in &manifest.entries {
        // 清单键为相对平铺路径（flat v1）——按「单个 Normal 组件」判定
        // （2026-09-27 复审 P2）：`..` 组件、根相对（前导分隔符）、盘符
        // 前缀（`C:foo` 在 Windows 解析为 Prefix+Normal 两组件）与多级
        // 路径全部拒绝；合法文件名含 `..` 子串（`a..b.txt`）不受影响。
        // 清单本身也是被保护面（0bw C4），此处防御的是生成器错误形态。
        if !is_flat_filename(rel) {
            findings.push(format!("{rel}: 清单键非法（相对平铺路径）"));
            continue;
        }
        let path = install_dir.join(rel);
        let Ok(bytes) = std::fs::read(&path) else {
            findings.push(format!("{rel}: 文件缺失"));
            continue;
        };
        checked += 1;
        if bytes.len() as u64 != entry.size_bytes {
            findings.push(format!(
                "{rel}: 长度不符（清单 {} ≠ 实际 {}）",
                entry.size_bytes,
                bytes.len()
            ));
            continue;
        }
        let actual = sha256_hex(&bytes);
        if actual != entry.sha256 {
            findings.push(format!("{rel}: sha256 失配"));
        }
    }
    // 未列文件检测（2026-09-27 复审 P3）：已列文件的篡改/缺失由上方逐条
    // 复验承接；顶层**新增**的清单外常规文件在此告警（flat v1 覆盖面补全；
    // 目录与 symlink 不入 flat 清单，亦不入此检测）。排除集与生成器镜像：
    // 清单自身＋机器本地 `.bak`/`-bak` 备份链（v0.8.0 重建批对齐——换装位
    // 的版本化 bak 链是站点考古件，非载体载荷，不作告警噪音）。
    if let Ok(dir_entries) = std::fs::read_dir(install_dir) {
        for dir_entry in dir_entries.flatten() {
            let Ok(file_type) = dir_entry.file_type() else {
                continue;
            };
            if !file_type.is_file() {
                continue;
            }
            let name = dir_entry.file_name().to_string_lossy().to_string();
            if name == MANIFEST_FILENAME || name.contains(".bak") || name.ends_with("-bak") {
                continue;
            }
            if manifest.entries.contains_key(&name) {
                continue;
            }
            findings.push(format!("{name}: 未列文件（清单外新增）"));
        }
    }
    if findings.is_empty() {
        IntegrityReport::Clean { checked }
    } else {
        IntegrityReport::Violated { findings }
    }
}

/// flat v1 清单键＝单个 `Normal` 组件的裸文件名（无分隔符/根/盘符/`..`
/// 组件；`Path::components` 在 Windows 下把 `C:foo` 解析为 Prefix+Normal
/// 两组件，恰可拒盘符相对形态）。
fn is_flat_filename(rel: &str) -> bool {
    let mut components = Path::new(rel).components();
    matches!(components.next(), Some(std::path::Component::Normal(_)))
        && components.next().is_none()
}

/// 启动自检入口：安装目录取自 `current_exe()` 父目录（与 0bw C2 同源）。
pub fn verify_at_startup() -> IntegrityReport {
    let Some(install_dir) = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
    else {
        return IntegrityReport::NoManifest;
    };
    verify(&install_dir)
}

/// 告警横幅（stderr 消费；机械文案、逐文件差异行）。
pub fn violation_banner(findings: &[String]) -> String {
    let mut banner =
        String::from("[carrier-integrity] 警告：载体完整性自检未通过（清单在位但存在差异）：");
    for finding in findings {
        banner.push_str("\n[carrier-integrity]   - ");
        banner.push_str(finding);
    }
    banner
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_manifest(dir: &Path, entries: BTreeMap<String, ManifestEntry>) {
        let manifest = CarrierManifest {
            kind: MANIFEST_KIND.to_string(),
            version: "test".to_string(),
            generated_at: "2026-09-27T00:00:00Z".to_string(),
            entries,
        };
        std::fs::write(manifest_path(dir), serde_json::to_vec(&manifest).unwrap()).unwrap();
    }

    fn entry(bytes: &[u8]) -> ManifestEntry {
        ManifestEntry {
            sha256: sha256_hex(bytes),
            size_bytes: bytes.len() as u64,
        }
    }

    fn tempdir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "orz-carrier-integrity-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn missing_manifest_reports_no_manifest() {
        let dir = tempdir();
        assert_eq!(verify(&dir), IntegrityReport::NoManifest);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn clean_bundle_passes_with_checked_count() {
        let dir = tempdir();
        std::fs::write(dir.join("orz.exe"), b"binary-bytes").unwrap();
        std::fs::write(dir.join("orz-signer.exe"), b"signer-bytes").unwrap();
        let mut entries = BTreeMap::new();
        entries.insert("orz.exe".to_string(), entry(b"binary-bytes"));
        entries.insert("orz-signer.exe".to_string(), entry(b"signer-bytes"));
        write_manifest(&dir, entries);
        match verify(&dir) {
            IntegrityReport::Clean { checked } => assert_eq!(checked, 2),
            other => panic!("expected clean, got {other:?}"),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn tampered_and_missing_files_are_reported() {
        let dir = tempdir();
        std::fs::write(dir.join("orz.exe"), b"tampered!").unwrap();
        // orz-signer.exe 故意不落盘。
        let mut entries = BTreeMap::new();
        entries.insert("orz.exe".to_string(), entry(b"original"));
        entries.insert("orz-signer.exe".to_string(), entry(b"signer"));
        write_manifest(&dir, entries);
        match verify(&dir) {
            IntegrityReport::Violated { findings } => {
                assert_eq!(findings.len(), 2, "{findings:?}");
                assert!(findings.iter().any(|f| f.starts_with("orz.exe:")));
                assert!(
                    findings.iter().any(|f| f.starts_with("orz-signer.exe:")),
                    "{findings:?}"
                );
            }
            other => panic!("expected violated, got {other:?}"),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn traversal_and_absolute_keys_are_violations_not_reads() {
        let dir = tempdir();
        let mut entries = BTreeMap::new();
        entries.insert(
            "..\\escaped.txt".to_string(),
            ManifestEntry {
                sha256: "0".repeat(64),
                size_bytes: 0,
            },
        );
        entries.insert(
            "/abs/path.txt".to_string(),
            ManifestEntry {
                sha256: "0".repeat(64),
                size_bytes: 0,
            },
        );
        write_manifest(&dir, entries);
        match verify(&dir) {
            IntegrityReport::Violated { findings } => {
                assert_eq!(findings.len(), 2, "{findings:?}");
                assert!(findings.iter().all(|f| f.contains("清单键非法")));
            }
            other => panic!("expected violated, got {other:?}"),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 2026-09-27 复审 P2：盘符相对键（`C:foo`）按「Prefix+Normal 两组件」
    /// 被拒（此前三项检查全放行）；Linux 下 `C:foo` 为合法文件名不受影响
    /// ——判定按组件语义而非平台字面。
    #[cfg(windows)]
    #[test]
    fn drive_relative_keys_are_violations() {
        let dir = tempdir();
        let mut entries = BTreeMap::new();
        entries.insert(
            "C:foo".to_string(),
            ManifestEntry {
                sha256: "0".repeat(64),
                size_bytes: 0,
            },
        );
        write_manifest(&dir, entries);
        match verify(&dir) {
            IntegrityReport::Violated { findings } => {
                assert_eq!(findings.len(), 1, "{findings:?}");
                assert!(findings[0].contains("清单键非法"), "{findings:?}");
            }
            other => panic!("expected violated, got {other:?}"),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 合法文件名含 `..` 子串不受影响（此前 `rel.contains("..")` 子串判定
    /// 会误杀，2026-09-27 复审 P3 修正）。
    #[test]
    fn dotdot_inside_filename_is_a_valid_key() {
        let dir = tempdir();
        std::fs::write(dir.join("a..b.txt"), b"bytes").unwrap();
        let mut entries = BTreeMap::new();
        entries.insert("a..b.txt".to_string(), entry(b"bytes"));
        write_manifest(&dir, entries);
        match verify(&dir) {
            IntegrityReport::Clean { checked } => assert_eq!(checked, 1),
            other => panic!("expected clean, got {other:?}"),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 未列文件检测（2026-09-27 复审 P3）：顶层清单外常规文件 ⇒ Violated。
    #[test]
    fn unlisted_top_level_file_is_reported() {
        let dir = tempdir();
        std::fs::write(dir.join("orz.exe"), b"binary").unwrap();
        std::fs::write(dir.join("dropped.dll"), b"extra").unwrap();
        let mut entries = BTreeMap::new();
        entries.insert("orz.exe".to_string(), entry(b"binary"));
        write_manifest(&dir, entries);
        match verify(&dir) {
            IntegrityReport::Violated { findings } => {
                assert_eq!(findings.len(), 1, "{findings:?}");
                assert!(findings[0].contains("未列文件"), "{findings:?}");
                assert!(findings[0].starts_with("dropped.dll"), "{findings:?}");
            }
            other => panic!("expected violated, got {other:?}"),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn wrong_kind_manifest_is_ignored() {
        let dir = tempdir();
        let manifest = CarrierManifest {
            kind: "something-else".to_string(),
            version: "test".to_string(),
            generated_at: "2026-09-27T00:00:00Z".to_string(),
            entries: BTreeMap::new(),
        };
        std::fs::write(manifest_path(&dir), serde_json::to_vec(&manifest).unwrap()).unwrap();
        assert_eq!(verify(&dir), IntegrityReport::NoManifest);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
