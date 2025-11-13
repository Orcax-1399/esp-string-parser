/// IO 抽象层 - trait 定义
///
/// 该模块定义了文件读写的抽象接口，支持依赖注入和测试 mock。
/// 遵循依赖倒置原则（DIP），面向接口编程。

use std::path::Path;
use crate::string_file::{StringFile, StringFileSet};

/// ESP 文件原始数据
#[derive(Debug, Clone)]
pub struct RawEspData {
    /// 文件的原始字节数据
    pub bytes: Vec<u8>,
}

/// ESP 文件读取 trait
///
/// # 职责
/// - 从文件系统读取 ESP/ESM/ESL 文件的原始字节数据
/// - 不负责解析，仅负责 IO
///
/// # 实现示例
/// ```rust,ignore
/// pub struct DefaultEspReader;
/// impl EspReader for DefaultEspReader {
///     fn read(&self, path: &Path) -> Result<RawEspData, Box<dyn std::error::Error>> {
///         let bytes = std::fs::read(path)?;
///         Ok(RawEspData { bytes })
///     }
/// }
/// ```
pub trait EspReader {
    /// 读取 ESP 文件的原始数据
    ///
    /// # 参数
    /// * `path` - 文件路径
    ///
    /// # 返回
    /// 返回包含原始字节数据的 RawEspData
    fn read(&self, path: &Path) -> Result<RawEspData, Box<dyn std::error::Error>>;
}

/// ESP 文件写入 trait
///
/// # 职责
/// - 将序列化后的数据写入文件系统
/// - 不负责序列化，仅负责 IO
pub trait EspWriter {
    /// 写入 ESP 文件数据
    ///
    /// # 参数
    /// * `data` - 要写入的原始数据
    /// * `path` - 目标文件路径
    fn write(&self, data: &RawEspData, path: &Path) -> Result<(), Box<dyn std::error::Error>>;
}

/// STRING 文件读取 trait
///
/// # 职责
/// - 读取并解析 .STRINGS/.DLSTRINGS/.ILSTRINGS 文件
pub trait StringFileReader {
    /// 读取 STRING 文件
    ///
    /// # 参数
    /// * `path` - STRING 文件路径
    ///
    /// # 返回
    /// 返回解析后的 StringFile
    fn read(&self, path: &Path) -> Result<StringFile, Box<dyn std::error::Error>>;
}

/// STRING 文件写入 trait
///
/// # 职责
/// - 将 StringFile 序列化并写入文件系统
pub trait StringFileWriter {
    /// 写入 STRING 文件
    ///
    /// # 参数
    /// * `file` - 要写入的 StringFile
    /// * `path` - 目标文件路径
    fn write(&self, file: &StringFile, path: &Path) -> Result<(), Box<dyn std::error::Error>>;
}

/// STRING 文件集读取 trait（便捷接口）
///
/// # 职责
/// - 批量加载插件的所有 STRING 文件（STRINGS + DLSTRINGS + ILSTRINGS）
pub trait StringFileSetReader {
    /// 从目录加载插件的所有 STRING 文件
    ///
    /// # 参数
    /// * `dir` - STRING 文件所在目录
    /// * `plugin_name` - 插件名称（不含扩展名）
    /// * `language` - 语言标识（如 "english"）
    ///
    /// # 返回
    /// 返回包含所有 STRING 文件的 StringFileSet
    fn read_set(
        &self,
        dir: &Path,
        plugin_name: &str,
        language: &str,
    ) -> Result<StringFileSet, Box<dyn std::error::Error>>;
}
