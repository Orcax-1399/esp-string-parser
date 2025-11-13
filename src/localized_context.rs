/// 本地化插件上下文
///
/// 该模块提供本地化插件（带 STRING 文件）的便捷处理接口。
/// 将 Plugin 和 StringFileSet 组合在一起，遵循组合模式。

use std::path::{Path, PathBuf};
use crate::Plugin;
use crate::StringFileSet;

/// 本地化插件上下文
///
/// 组合 Plugin 和 StringFileSet，提供统一的本地化插件操作接口。
///
/// # 使用场景
/// - 处理设置了 LOCALIZED 标志的 ESP/ESM 文件
/// - 需要同时访问 ESP 文件和 STRING 文件
///
/// # 示例
/// ```rust,ignore
/// use esp_extractor::LocalizedPluginContext;
///
/// // 加载本地化插件
/// let context = LocalizedPluginContext::load(
///     "DismemberingFramework.esm".into(),
///     "english",
/// )?;
///
/// // 访问插件
/// println!("插件名: {}", context.plugin().get_name());
///
/// // 访问 STRING 文件
/// println!("STRING 文件数: {}", context.string_files().files.len());
///
/// // 提取字符串（从 STRING 文件读取）
/// let strings = context.plugin().extract_strings();
/// ```
#[derive(Debug)]
pub struct LocalizedPluginContext {
    /// ESP/ESM/ESL 插件实例
    plugin: Plugin,
    /// STRING 文件集合
    string_files: StringFileSet,
    /// 语言标识
    language: String,
}

impl LocalizedPluginContext {
    /// 加载本地化插件及其 STRING 文件
    ///
    /// # 参数
    /// * `path` - ESP/ESM/ESL 文件路径
    /// * `language` - 语言标识（如 "english", "chinese" 等）
    ///
    /// # 返回
    /// 返回包含插件和 STRING 文件的上下文
    ///
    /// # 错误
    /// - 如果插件文件不存在或无效
    /// - 如果 STRING 文件加载失败
    /// - 如果插件未设置 LOCALIZED 标志（警告但不报错）
    pub fn load(path: PathBuf, language: &str) -> Result<Self, Box<dyn std::error::Error>> {
        // 加载插件
        let plugin = Plugin::load(path.clone())?;

        // 检查是否为本地化插件
        if !plugin.is_localized() {
            eprintln!(
                "警告: 插件 {} 未设置 LOCALIZED 标志，可能不包含 STRING 文件",
                plugin.get_name()
            );
        }

        // 加载 STRING 文件
        let string_files = Self::load_string_files(&path, &plugin, language)?;

        Ok(Self {
            plugin,
            string_files,
            language: language.to_string(),
        })
    }

    /// 加载 STRING 文件（内部辅助方法）
    fn load_string_files(
        path: &Path,
        _plugin: &Plugin,
        language: &str,
    ) -> Result<StringFileSet, Box<dyn std::error::Error>> {
        let plugin_dir = path.parent().ok_or("无法获取插件目录")?;
        let plugin_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or("无法获取插件名称")?;

        // 尝试多个可能的 STRING 文件位置
        let search_dirs = vec![
            plugin_dir.to_path_buf(),               // 同目录
            plugin_dir.join("Strings"),             // Strings子目录（常见于开发环境）
            plugin_dir.join("strings"),             // strings子目录（小写）
        ];

        for dir in search_dirs {
            if !dir.exists() {
                continue;
            }

            match StringFileSet::load_from_directory(&dir, plugin_name, language) {
                Ok(set) if !set.files.is_empty() => {
                    #[cfg(debug_assertions)]
                    println!(
                        "已加载 STRING 文件: {} 个文件类型（从 {:?}）",
                        set.files.len(),
                        dir
                    );
                    return Ok(set);
                }
                Ok(_) => {
                    // 找到目录但没有 STRING 文件，继续搜索
                    #[cfg(debug_assertions)]
                    eprintln!("提示: {:?} 目录下未找到 STRING 文件", dir);
                }
                Err(e) => {
                    #[cfg(debug_assertions)]
                    eprintln!("警告: 无法从 {:?} 加载 STRING 文件: {}", dir, e);
                }
            }
        }

        Err("未找到任何 STRING 文件".into())
    }

    /// 获取插件的不可变引用
    pub fn plugin(&self) -> &Plugin {
        &self.plugin
    }

    /// 获取插件的可变引用
    pub fn plugin_mut(&mut self) -> &mut Plugin {
        &mut self.plugin
    }

    /// 获取 STRING 文件集的不可变引用
    pub fn string_files(&self) -> &StringFileSet {
        &self.string_files
    }

    /// 获取 STRING 文件集的可变引用
    pub fn string_files_mut(&mut self) -> &mut StringFileSet {
        &mut self.string_files
    }

    /// 获取语言标识
    pub fn language(&self) -> &str {
        &self.language
    }

    /// 解构上下文，获取所有权
    ///
    /// # 返回
    /// 返回 (Plugin, StringFileSet, String) 元组
    pub fn into_parts(self) -> (Plugin, StringFileSet, String) {
        (self.plugin, self.string_files, self.language)
    }

    /// 保存 STRING 文件到指定目录
    ///
    /// # 参数
    /// * `output_dir` - 输出目录路径（STRING 文件将写入 output_dir/strings/）
    pub fn save_string_files(
        &self,
        output_dir: &Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let string_dir = output_dir.join("strings");
        std::fs::create_dir_all(&string_dir)?;
        self.string_files.write_all(&string_dir)
    }

    /// 生成上下文摘要
    pub fn summary(&self) -> String {
        format!(
            "本地化插件: {}, 语言: {}, STRING 文件数: {}",
            self.plugin.get_name(),
            self.language,
            self.string_files.files.len()
        )
    }
}

#[cfg(test)]
mod tests {
    // use super::*;

    #[test]
    fn test_localized_context_creation() {
        // 注意：这个测试需要一个有效的本地化 ESP 文件和 STRING 文件
        // 在实际项目中，应该使用测试 fixture
        // 这里只是演示 API 用法

        // let context = LocalizedPluginContext::load(
        //     "test.esm".into(),
        //     "english",
        // ).unwrap();
        //
        // assert!(context.plugin().is_localized());
        // assert!(!context.string_files().files.is_empty());
    }
}
