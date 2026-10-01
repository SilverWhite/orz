//! Project-level agent directory discovery (0ce 保留面).
//!
//! 0ce（2026-10-01）死代码清退后的唯一保留职责：`.grok/agents/` 与
//! `.claude/agents/` 项目级目录枚举（消费方 `orz-workspace::folder_trust`
//! 的 repo_configs_present 探针，与 plugins 目录枚举互为镜像）。原 agent
//! 定义发现机制（discover/by_name/all_subagents/内置定义表）零服务消费，
//! 已整体退役。

use std::path::{Path, PathBuf};

/// Project-level agent directories to scan (`.grok/agents/` + `.claude/agents/` compat).
const PROJECT_AGENT_SUBDIRS: &[&str] = &[".grok/agents", ".claude/agents"];

/// Existing project-level agent dirs (`.grok/agents` / `.claude/agents`), walked
/// from `cwd` up to the git worktree root (inclusive). Returns
/// `(existing dirs, git_root)`. Mirrors [`crate::plugins::project_plugin_dirs`].
pub fn project_agent_dirs(cwd: Option<&Path>) -> (Vec<PathBuf>, Option<PathBuf>) {
    let Some(cwd) = cwd else {
        return (Vec::new(), None);
    };
    let chain = crate::repo::RepoDirChain::resolve(cwd);
    (project_agent_dirs_in(&chain.dirs), chain.git_root)
}

/// Existing project agent dirs (`.grok/agents` / `.claude/agents`) under each
/// dir of a precomputed cwd→git-root chain ([`crate::repo::RepoDirChain`]).
///
/// Single source of the `PROJECT_AGENT_SUBDIRS` walk: the folder-trust detector
/// (`repo_configs_present`) reuses its one shared chain here so detection can
/// never drift from discovery (adding a third project-agent dir updates both at
/// once).
pub fn project_agent_dirs_in(chain_dirs: &[PathBuf]) -> Vec<PathBuf> {
    crate::repo::existing_subdirs_along(chain_dirs, PROJECT_AGENT_SUBDIRS)
}

#[cfg(test)]
mod tests {
    /// 0ce 保留面钉：两个兼容目录都在走查集内，且只报真实存在的目录。
    #[test]
    fn project_agent_dirs_in_lists_only_existing_compat_dirs() {
        let dir = tempfile::tempdir().unwrap();
        assert!(super::project_agent_dirs_in(&[dir.path().to_path_buf()]).is_empty());

        std::fs::create_dir_all(dir.path().join(".claude/agents")).unwrap();
        let found = super::project_agent_dirs_in(&[dir.path().to_path_buf()]);
        assert_eq!(found, vec![dir.path().join(".claude/agents")]);
    }

    #[test]
    fn project_agent_dirs_without_cwd_is_empty() {
        assert_eq!(super::project_agent_dirs(None), (Vec::new(), None));
    }
}
