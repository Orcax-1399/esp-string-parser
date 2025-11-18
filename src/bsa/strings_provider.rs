//! BSA Strings 文件提供者
//!
//! 专门用于从 BSA 归档中提取 .strings / .ilstrings / .dlstrings 文件

use super::{BsaArchive, BsaError};
use std::path::Path;

/// 官方主文件列表（这些文件共享 "Skyrim - Interface.bsa"）
/// 注意：不含扩展名，因为 plugin_name 来自 file_stem()
const OFFICIAL_MASTER_FILES: &[&str] = &[
    "skyrim",
    "update",
    "dawnguard",
    "dragonborn",
    "hearthfires",
];

/// 从 BSA 中提取 Strings 文件的专用接口
pub struct BsaStringsProvider {
    /// 已打开的 BSA 归档
    archive: BsaArchive,
}

impl BsaStringsProvider {
    /// 为指定的插件打开对应的 BSA 归档
    ///
    /// # 查找规则
    /// 1. 优先查找同名 BSA（例如 `MyMod.esp` → `MyMod.bsa`）
    /// 2. 对于官方主文件，使用 `Skyrim - Interface.bsa`
    ///
    /// # 参数
    /// - `plugin_path`: 插件文件路径（.esp/.esm/.esl）
    ///
    /// # 返回
    /// - 成功：返回 `BsaStringsProvider`
    /// - 失败：找不到 BSA 或无法打开
    pub fn open_for_plugin<P: AsRef<Path>>(plugin_path: P) -> Result<Self, BsaError> {
        let plugin_path = plugin_path.as_ref();
        let plugin_dir = plugin_path
            .parent()
            .ok_or_else(|| BsaError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "无法获取插件目录",
            )))?;

        let plugin_name = plugin_path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| BsaError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "无法获取插件名称",
            )))?;

        // 检查是否为官方主文件
        let bsa_path = if Self::is_official_master(plugin_name) {
            // 官方文件使用 Skyrim - Interface.bsa
            plugin_dir.join("Skyrim - Interface.bsa")
        } else {
            // 普通 mod 使用同名 BSA
            plugin_dir.join(format!("{}.bsa", plugin_name))
        };

        // 检查 BSA 是否存在
        if !bsa_path.exists() {
            return Err(BsaError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("BSA 文件不存在: {}", bsa_path.display()),
            )));
        }

        // 打开 BSA
        let archive = BsaArchive::open(bsa_path)?;

        Ok(Self { archive })
    }

    /// 提取指定的 strings 文件
    ///
    /// # 路径规则
    /// - 优先尝试 `strings/` 目录（小写 s）
    /// - 失败后尝试 `Strings/` 目录（大写 S）
    ///
    /// # 参数
    /// - `plugin_name`: 插件名称（不含扩展名），例如 "Skyrim"
    /// - `language`: 语言代码，例如 "english"
    /// - `extension`: 文件扩展名，例如 "STRINGS" / "ILSTRINGS" / "DLSTRINGS"
    ///
    /// # 返回
    /// - 成功：strings 文件的原始字节数据
    /// - 失败：`BsaError::NotFound` 或解压错误
    pub fn extract_strings(
        &self,
        plugin_name: &str,
        language: &str,
        extension: &str,
    ) -> Result<Vec<u8>, BsaError> {
        // 生成文件名：PluginName_Language.EXTENSION
        let filename = format!("{}_{}.{}", plugin_name, language, extension);

        // 尝试路径变体
        let path_variants = vec![
            format!("strings/{}", filename.to_lowercase()),  // 优先：strings/ + 小写
            format!("Strings/{}", filename),                  // 备选：Strings/ + 原样
            format!("strings/{}", filename),                  // 备选：strings/ + 原样
            format!("Strings/{}", filename.to_lowercase()),  // 备选：Strings/ + 小写
        ];

        // 依次尝试每个路径变体
        for path in &path_variants {
            match self.archive.extract(path) {
                Ok(data) => return Ok(data),
                Err(BsaError::NotFound(_)) => continue,  // 尝试下一个
                Err(e) => return Err(e),                  // 其他错误直接返回
            }
        }

        // 所有路径都失败
        Err(BsaError::NotFound(format!(
            "在 BSA 中找不到 strings 文件: {} (尝试了 {} 个路径变体)",
            filename,
            path_variants.len()
        )))
    }

    /// 列出 BSA 中所有的 strings 文件
    ///
    /// # 返回
    /// 所有 `.strings`, `.ilstrings`, `.dlstrings` 文件的路径列表
    pub fn list_strings_files(&self) -> Vec<String> {
        self.archive
            .file_list()
            .into_iter()
            .filter(|path| {
                let lower = path.to_lowercase();
                lower.ends_with(".strings")
                    || lower.ends_with(".ilstrings")
                    || lower.ends_with(".dlstrings")
            })
            .collect()
    }

    /// 检查插件是否为官方主文件
    fn is_official_master(plugin_name: &str) -> bool {
        let lower = plugin_name.to_lowercase();
        OFFICIAL_MASTER_FILES.contains(&lower.as_str())
    }

    /// 获取底层 BSA 归档的引用（供高级用途）
    pub fn archive(&self) -> &BsaArchive {
        &self.archive
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_official_master() {
        // plugin_name 来自 file_stem()，不含扩展名
        assert!(BsaStringsProvider::is_official_master("Skyrim"));
        assert!(BsaStringsProvider::is_official_master("skyrim"));
        assert!(BsaStringsProvider::is_official_master("SKYRIM"));
        assert!(BsaStringsProvider::is_official_master("Update"));
        assert!(BsaStringsProvider::is_official_master("Dawnguard"));
        assert!(BsaStringsProvider::is_official_master("Dragonborn"));
        assert!(BsaStringsProvider::is_official_master("HearthFires"));

        assert!(!BsaStringsProvider::is_official_master("MyMod"));
        assert!(!BsaStringsProvider::is_official_master("CustomContent"));
    }
}
