//! Slash-command registry — Chinese-name commands over English slash names
//! (Python `commands.py` shape; v1 subset).

use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandDef {
    pub slash: &'static str,
    pub name_zh: &'static str,
    pub description_zh: &'static str,
    /// Category used by the (future) command palette.
    pub category: &'static str,
}

const BUILTIN: &[CommandDef] = &[
    CommandDef {
        slash: "/help",
        name_zh: "帮助",
        description_zh: "显示帮助覆盖层",
        category: "系统",
    },
    CommandDef {
        slash: "/status",
        name_zh: "状态",
        description_zh: "显示当前运行状态",
        category: "系统",
    },
    CommandDef {
        slash: "/stop",
        name_zh: "停止",
        description_zh: "取消当前运行",
        category: "运行",
    },
    CommandDef {
        slash: "/open",
        name_zh: "打开",
        description_zh: "打开文件或对象",
        category: "导航",
    },
    CommandDef {
        slash: "/toggle-explorer",
        name_zh: "探索器",
        description_zh: "显示/隐藏探索器面板（待实施）",
        category: "视图",
    },
    CommandDef {
        slash: "/toggle-events",
        name_zh: "事件栏",
        description_zh: "显示/隐藏事件栏（待实施）",
        category: "视图",
    },
    CommandDef {
        slash: "/toggle-markers",
        name_zh: "标记栏",
        description_zh: "显示/隐藏右侧标记栏",
        category: "视图",
    },
];

/// Registry with prefix search — slash commands are matched on the slash
/// text; a bare "/" lists the defaults.
#[derive(Debug, Clone, Default)]
pub struct CommandRegistry {
    by_slash: BTreeMap<&'static str, &'static CommandDef>,
}

impl CommandRegistry {
    pub fn with_builtins() -> Self {
        let mut reg = Self::default();
        for def in BUILTIN {
            reg.register(def);
        }
        reg
    }

    pub fn register(&mut self, def: &'static CommandDef) {
        self.by_slash.insert(def.slash, def);
    }

    pub fn get(&self, slash: &str) -> Option<&'static CommandDef> {
        self.by_slash.get(slash).copied()
    }

    /// Commands whose slash name starts with *prefix* (deterministic order).
    pub fn search(&self, prefix: &str) -> Vec<&'static CommandDef> {
        self.by_slash
            .range(prefix..)
            .filter(|(k, _)| k.starts_with(prefix))
            .map(|(_, v)| *v)
            .collect()
    }

    /// The default six shown on a bare "/" (Python `get_defaults`).
    pub fn get_defaults(&self) -> Vec<&'static CommandDef> {
        ["/help", "/status", "/stop", "/open"]
            .iter()
            .filter_map(|s| self.get(s))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_registry_resolves() {
        let reg = CommandRegistry::with_builtins();
        assert_eq!(reg.get("/help").unwrap().name_zh, "帮助");
        assert_eq!(reg.get("/toggle-markers").unwrap().description_zh, "显示/隐藏右侧标记栏");
        assert_eq!(reg.get("/nope"), None);
    }

    #[test]
    fn prefix_search_is_deterministic() {
        let reg = CommandRegistry::with_builtins();
        let toggles: Vec<&str> = reg.search("/toggle").iter().map(|d| d.slash).collect();
        assert_eq!(toggles, ["/toggle-events", "/toggle-explorer", "/toggle-markers"]);
        assert!(reg.search("/x").is_empty());
    }

    #[test]
    fn defaults_list() {
        let reg = CommandRegistry::with_builtins();
        let defs = reg.get_defaults();
        assert!(defs.iter().any(|d| d.slash == "/help"));
        assert!(defs.iter().any(|d| d.slash == "/status"));
    }
}
