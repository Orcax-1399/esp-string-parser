use std::path::Path;

use crate::bsa::BsaStringsProvider;

use super::{StringFile, StringFileSet, StringFileType};

impl StringFileSet {
    /// 尝试从 BSA 归档中加载字符串文件（fallback 机制）
    ///
    /// # 参数
    /// - `plugin_path`: 插件文件的完整路径（用于定位 BSA）
    /// - `plugin_name`: 插件名称（不含扩展名）
    /// - `language`: 语言代码
    ///
    /// # 返回
    /// 成功时返回加载的 `StringFileSet`，失败时返回错误
    pub fn load_from_bsa(
        plugin_path: &Path,
        plugin_name: &str,
        language: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let bsa_provider = BsaStringsProvider::open_for_plugin(plugin_path)?;

        let mut set = StringFileSet::new(plugin_name.to_string(), language.to_string());

        for file_type in [
            StringFileType::STRINGS,
            StringFileType::ILSTRINGS,
            StringFileType::DLSTRINGS,
        ] {
            match bsa_provider.extract_strings(plugin_name, language, file_type.to_extension()) {
                Ok(data) => match StringFile::from_bytes(
                    &data,
                    plugin_name.to_string(),
                    language.to_string(),
                    file_type,
                ) {
                    Ok(string_file) => {
                        set.files.insert(file_type, string_file);

                        #[cfg(debug_assertions)]
                        eprintln!(
                            "✓ 从 BSA 中成功加载: {}_{}.{}",
                            plugin_name,
                            language,
                            file_type.to_extension()
                        );
                    }
                    Err(_e) => {
                        #[cfg(debug_assertions)]
                        eprintln!(
                            "⚠️ 从 BSA 提取的数据解析失败: {}_{}.{} - {}",
                            plugin_name,
                            language,
                            file_type.to_extension(),
                            _e
                        );
                    }
                },
                Err(_) => continue,
            }
        }

        if set.files.is_empty() {
            return Err("BSA 中未找到任何 strings 文件".into());
        }

        Ok(set)
    }
}
