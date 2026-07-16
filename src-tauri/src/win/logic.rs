//! Windows 纯逻辑（全平台编译，可单测）。

/// 开始菜单快捷方式里的非应用条目：卸载器、帮助、官网链接等。
/// ponytail: 关键词表打底，真机验收发现漏网再补词。
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub fn is_noise_entry(name: &str) -> bool {
    const NOISE: &[&str] = &[
        "卸载", "uninstall", "帮助", "help", "文档", "documentation", "docs",
        "官网", "website", "readme", "release notes", "更新日志", "license",
    ];
    let lower = name.to_lowercase();
    NOISE.iter().any(|kw| lower.contains(kw))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noise_entries_filtered() {
        for name in [
            "卸载 微信",
            "Uninstall Foo",
            "foo uninstaller",
            "Node.js documentation",
            "VLC 帮助",
            "Epic 官网",
            "README",
            "website of thing",
        ] {
            assert!(is_noise_entry(name), "should filter: {name}");
        }
    }

    #[test]
    fn real_apps_kept() {
        for name in ["微信", "Google Chrome", "Visual Studio Code", "设置", "Everything"] {
            assert!(!is_noise_entry(name), "should keep: {name}");
        }
    }
}
