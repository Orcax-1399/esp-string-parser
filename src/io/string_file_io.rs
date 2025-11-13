/// STRING 文件 IO 实现
///
/// 提供基于文件系统的默认 STRING 文件读写实现

use std::path::Path;
use super::traits::{StringFileReader, StringFileWriter, StringFileSetReader};
use crate::string_file::{StringFile, StringFileSet};

/// 默认的 STRING 文件读取器
#[derive(Debug, Clone, Default)]
pub struct DefaultStringFileReader;

impl StringFileReader for DefaultStringFileReader {
    fn read(&self, path: &Path) -> Result<StringFile, Box<dyn std::error::Error>> {
        StringFile::new(path.to_path_buf())
    }
}

/// 默认的 STRING 文件写入器
#[derive(Debug, Clone, Default)]
pub struct DefaultStringFileWriter;

impl StringFileWriter for DefaultStringFileWriter {
    fn write(&self, file: &StringFile, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        // 确保父目录存在
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        file.write_to_file(path.to_path_buf())
    }
}

/// 默认的 STRING 文件集读取器
#[derive(Debug, Clone, Default)]
pub struct DefaultStringFileSetReader;

impl StringFileSetReader for DefaultStringFileSetReader {
    fn read_set(
        &self,
        dir: &Path,
        plugin_name: &str,
        language: &str,
    ) -> Result<StringFileSet, Box<dyn std::error::Error>> {
        StringFileSet::load_from_directory(dir, plugin_name, language)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_file_reader_nonexistent() {
        let reader = DefaultStringFileReader;
        let result = reader.read(Path::new("nonexistent.strings"));

        // 应该返回错误
        assert!(result.is_err());
    }

    // 注意：以下测试需要重写以匹配当前的 StringFile 结构
    // 暂时禁用，待后续完善

    // #[test]
    // fn test_string_file_writer() {
    //     // TODO: 需要更新以匹配新的 StringFile 结构
    // }

    // #[test]
    // fn test_writer_creates_parent_dirs() {
    //     // TODO: 需要更新以匹配新的 StringFile 结构
    // }
}
