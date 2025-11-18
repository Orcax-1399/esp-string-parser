//! BSA (Bethesda Archive) 文件访问模块
//!
//! 提供对 Bethesda 游戏引擎使用的 BSA 归档格式的读取支持。
//! 主要用于从 BSA 中提取 strings 文件作为 fallback 机制。

mod strings_provider;

use std::path::Path;
use thiserror::Error;
use ba2::{
    prelude::*,
    tes4::{Archive, ArchiveKey, DirectoryKey, ArchiveOptions, FileCompressionOptions}
};

pub use strings_provider::BsaStringsProvider;

/// BSA 操作相关错误
#[derive(Debug, Error)]
pub enum BsaError {
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),

    #[error("ba2/tes4 解析错误: {0}")]
    Ba2(#[from] ba2::tes4::Error),

    #[error("文件在归档中不存在: {0}")]
    NotFound(String),
}

/// BSA 归档访问器
///
/// 提供对 TES4 风格 BSA 文件（Oblivion / Fallout 3 / NV / Skyrim）的读取能力
pub struct BsaArchive {
    /// 底层 ba2 归档对象
    archive: Archive<'static>,
    /// 归档元数据
    meta: ArchiveOptions,
}

impl BsaArchive {
    /// 打开一个 TES4 风格的 BSA 归档
    ///
    /// # 参数
    /// - `path`: BSA 文件路径
    ///
    /// # 返回
    /// - 成功：返回 `BsaArchive`
    /// - 失败：返回 `BsaError::Io` 或 `BsaError::Ba2`
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, BsaError> {
        let (archive, meta) = Archive::read(path.as_ref())?;

        Ok(Self {
            archive,
            meta,
        })
    }

    /// 返回归档中所有文件的"逻辑路径"列表
    ///
    /// # 行为
    /// - 路径统一为小写、使用 `/` 分隔的相对路径
    /// - 例如：`"meshes/armor/iron/ironcuisse.nif"`
    /// - 返回顺序按字典序排序
    pub fn file_list(&self) -> Vec<String> {
        let mut files = Vec::new();

        // 遍历所有目录（Archive 的键是目录）
        for (dir_key, directory) in &self.archive {
            // 获取目录名（ba2 内部使用 bstr 格式）
            let dir_name = String::from_utf8_lossy(dir_key.name()).to_string();

            // 遍历该目录下的所有文件
            for (file_key, _file) in directory {
                let file_name = String::from_utf8_lossy(file_key.name()).to_string();

                // 拼接路径：dir_name + '/' + file_name
                let logical_path = if dir_name.is_empty() {
                    file_name
                } else {
                    format!("{}/{}", dir_name, file_name)
                };

                // 规范化：小写 + 统一斜杠
                let normalized = Self::normalize_path(&logical_path);
                files.push(normalized);
            }
        }

        // 按字典序排序
        files.sort();
        files
    }

    /// 提取指定逻辑路径的文件内容
    ///
    /// # 参数
    /// - `logical_path`: 调用方传入的路径，不区分大小写，允许使用 `/` 或 `\`
    ///
    /// # 返回
    /// - 成功：文件原始字节数据（解压之后）
    /// - 失败：`BsaError::NotFound` 或 `BsaError::Ba2`
    pub fn extract(&self, logical_path: &str) -> Result<Vec<u8>, BsaError> {
        // 规范化输入路径
        let normalized = Self::normalize_path(logical_path);

        // 拆分为目录和文件名
        let (dir_name, file_name) = Self::split_path(&normalized);

        // 生成目录 key（ba2 的 ArchiveKey 对应目录）
        let dir_key = ArchiveKey::from(dir_name.as_bytes());

        // 查找目录
        let directory = self
            .archive
            .get(&dir_key)
            .ok_or_else(|| BsaError::NotFound(format!("目录不存在: {}", dir_name)))?;

        // 生成文件 key（ba2 的 DirectoryKey 对应文件）
        let file_key = DirectoryKey::from(file_name.as_bytes());

        // 查找文件
        let file = directory
            .get(&file_key)
            .ok_or_else(|| BsaError::NotFound(format!("文件不存在: {}/{}", dir_name, file_name)))?;

        // 解压文件到内存
        let compression_options: FileCompressionOptions = self.meta.into();
        let mut buffer = Vec::new();
        file.write(&mut buffer, &compression_options)?;

        Ok(buffer)
    }

    /// 规范化路径：小写 + 统一为 `/` 分隔符 + 移除前导 `/`
    fn normalize_path(path: &str) -> String {
        path.to_lowercase()
            .replace('\\', "/")
            .trim_start_matches('/')
            .to_string()
    }

    /// 把逻辑路径拆分为 (dir, file)
    ///
    /// - 输入已经是小写，且统一为 '/'
    /// - 如果没有 '/'，dir 为空字符串
    fn split_path(path: &str) -> (String, String) {
        match path.rfind('/') {
            Some(pos) => {
                let dir = path[..pos].to_string();
                let file = path[pos + 1..].to_string();
                (dir, file)
            }
            None => (String::new(), path.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_path() {
        assert_eq!(
            BsaArchive::normalize_path("Meshes\\Armor\\Iron\\IronCuisse.NIF"),
            "meshes/armor/iron/ironcuisse.nif"
        );
        assert_eq!(
            BsaArchive::normalize_path("/strings/Skyrim_English.STRINGS"),
            "strings/skyrim_english.strings"
        );
    }

    #[test]
    fn test_split_path() {
        assert_eq!(
            BsaArchive::split_path("meshes/armor/iron/cuisse.nif"),
            ("meshes/armor/iron".to_string(), "cuisse.nif".to_string())
        );
        assert_eq!(
            BsaArchive::split_path("file.txt"),
            (String::new(), "file.txt".to_string())
        );
    }
}
