/// 智能插件加载器
///
/// 提供自动检测和加载的便捷 API，同时保持底层 API 的灵活性。

use std::path::PathBuf;
use crate::{Plugin, LocalizedPluginContext};

/// 插件加载结果
///
/// 根据插件是否设置 LOCALIZED 标志，自动选择合适的加载方式。
#[derive(Debug)]
pub enum LoadedPlugin {
    /// 普通插件（未设置 LOCALIZED 标志）
    Standard(Plugin),
    /// 本地化插件（设置了 LOCALIZED 标志，已加载 STRING 文件）
    Localized(LocalizedPluginContext),
}

impl LoadedPlugin {
    /// 智能加载插件
    ///
    /// 自动检测插件类型并选择合适的加载策略：
    /// - 如果插件设置了 LOCALIZED 标志 → 加载为 `LocalizedPluginContext`
    /// - 如果是普通插件 → 加载为 `Plugin`
    ///
    /// # 参数
    /// * `path` - ESP/ESM/ESL 文件路径
    /// * `language` - 语言标识（仅对本地化插件有效），默认 "english"
    ///
    /// # 返回
    /// 返回 `LoadedPlugin` 枚举，可通过模式匹配处理
    ///
    /// # 示例
    /// ```rust,ignore
    /// use esp_extractor::LoadedPlugin;
    ///
    /// let loaded = LoadedPlugin::load_auto("MyMod.esp".into(), Some("english"))?;
    ///
    /// match loaded {
    ///     LoadedPlugin::Standard(plugin) => {
    ///         println!("普通插件: {}", plugin.get_name());
    ///     }
    ///     LoadedPlugin::Localized(context) => {
    ///         println!("本地化插件: {}", context.plugin().get_name());
    ///         println!("STRING 文件数: {}", context.string_files().files.len());
    ///     }
    /// }
    /// ```
    pub fn load_auto(
        path: PathBuf,
        language: Option<&str>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // 先加载基础 Plugin
        let plugin = Plugin::load(path.clone())?;

        // 检查是否为本地化插件
        if plugin.is_localized() {
            // 本地化插件：尝试加载 STRING 文件
            let lang = language.unwrap_or("english");

            match LocalizedPluginContext::load(path, lang) {
                Ok(context) => Ok(LoadedPlugin::Localized(context)),
                Err(e) => {
                    // STRING 文件加载失败，降级为普通插件
                    eprintln!(
                        "警告: 本地化插件 {} 的 STRING 文件加载失败: {}",
                        plugin.get_name(),
                        e
                    );
                    eprintln!("降级为普通插件模式（字符串将显示为 StringID）");
                    Ok(LoadedPlugin::Standard(plugin))
                }
            }
        } else {
            // 普通插件
            Ok(LoadedPlugin::Standard(plugin))
        }
    }

    /// 获取底层 Plugin 的引用（无论哪种类型）
    pub fn plugin(&self) -> &Plugin {
        match self {
            LoadedPlugin::Standard(plugin) => plugin,
            LoadedPlugin::Localized(context) => context.plugin(),
        }
    }

    /// 检查是否为本地化插件
    pub fn is_localized(&self) -> bool {
        matches!(self, LoadedPlugin::Localized(_))
    }
}

// 便捷方法：提取字符串（统一接口）
impl LoadedPlugin {
    /// 提取字符串（自动处理本地化/非本地化）
    pub fn extract_strings(&self) -> Vec<crate::ExtractedString> {
        self.plugin().extract_strings()
    }

    /// 获取插件名称
    pub fn get_name(&self) -> &str {
        self.plugin().get_name()
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> crate::plugin::PluginStats {
        self.plugin().get_stats()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_auto_concept() {
        // 测试概念验证
        // 实际测试需要真实的 ESP 文件

        // let loaded = LoadedPlugin::load_auto("test.esp".into(), None).unwrap();
        // match loaded {
        //     LoadedPlugin::Standard(plugin) => {
        //         assert!(!plugin.is_localized());
        //     }
        //     LoadedPlugin::Localized(context) => {
        //         assert!(context.plugin().is_localized());
        //     }
        // }
    }
}
