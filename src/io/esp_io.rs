/// ESP 文件 IO 实现
///
/// 提供基于文件系统的默认 ESP 文件读写实现
use std::path::Path;
use super::traits::{EspReader, EspWriter, RawEspData};

/// 默认的 ESP 文件读取器（基于 std::fs）
#[derive(Debug, Clone, Default)]
pub struct DefaultEspReader;

impl EspReader for DefaultEspReader {
    fn read(&self, path: &Path) -> Result<RawEspData, Box<dyn std::error::Error>> {
        let bytes = std::fs::read(path)?;
        Ok(RawEspData { bytes })
    }
}

/// 默认的 ESP 文件写入器（基于 std::fs）
#[derive(Debug, Clone, Default)]
pub struct DefaultEspWriter;

impl EspWriter for DefaultEspWriter {
    fn write(&self, data: &RawEspData, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        // 确保父目录存在
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(path, &data.bytes)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_esp_reader() {
        // 创建临时测试文件
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_esp_reader.esp");

        let test_data = b"TES4\x00\x00\x00\x00";
        std::fs::write(&test_file, test_data).unwrap();

        // 测试读取
        let reader = DefaultEspReader;
        let result = reader.read(&test_file).unwrap();

        assert_eq!(result.bytes, test_data);

        // 清理
        std::fs::remove_file(&test_file).unwrap();
    }

    #[test]
    fn test_default_esp_writer() {
        // 创建临时测试路径
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_esp_writer.esp");

        let test_data = RawEspData {
            bytes: b"TES4\x00\x00\x00\x00".to_vec(),
        };

        // 测试写入
        let writer = DefaultEspWriter;
        writer.write(&test_data, &test_file).unwrap();

        // 验证
        let written_data = std::fs::read(&test_file).unwrap();
        assert_eq!(written_data, test_data.bytes);

        // 清理
        std::fs::remove_file(&test_file).unwrap();
    }

    #[test]
    fn test_writer_creates_parent_dirs() {
        let temp_dir = std::env::temp_dir();
        let nested_dir = temp_dir.join("test_esp_nested").join("subdir");
        let test_file = nested_dir.join("test.esp");

        let test_data = RawEspData {
            bytes: vec![1, 2, 3, 4],
        };

        // 测试自动创建父目录
        let writer = DefaultEspWriter;
        writer.write(&test_data, &test_file).unwrap();

        assert!(test_file.exists());

        // 清理
        std::fs::remove_dir_all(temp_dir.join("test_esp_nested")).unwrap();
    }
}
