//! # ESP字符串提取工具库
//! 
//! 这是一个用于处理Bethesda游戏引擎（ESP/ESM/ESL）文件的Rust库。
//! 支持字符串提取、翻译应用和文件结构调试。
//! 
//! ## 主要功能
//! 
//! - **字符串提取**：从ESP/ESM/ESL文件中提取可翻译的字符串
//! - **翻译应用**：将翻译后的字符串写回到游戏文件中
//! - **文件结构调试**：提供详细的文件结构分析和对比工具
//! - **压缩记录支持**：正确处理压缩和未压缩的记录
//! - **多编码支持**：支持UTF-8、GBK、ASCII等编码格式
//! 
//! ## 使用示例
//! 
//! ```rust,no_run
//! use esp_extractor::{Plugin, ExtractedString};
//! use std::path::PathBuf;
//! 
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // 加载ESP文件
//! let plugin = Plugin::new(PathBuf::from("example.esp"))?;
//! 
//! // 提取字符串
//! let strings = plugin.extract_strings();
//! println!("提取到 {} 个字符串", strings.len());
//! 
//! // 准备翻译数据
//! let translated_strings = strings; // 这里应该是你的翻译数据
//! 
//! // 应用翻译
//! Plugin::apply_translations(
//!     PathBuf::from("input.esp"),
//!     PathBuf::from("output.esp"),
//!     translated_strings
//! )?;
//! # Ok(())
//! # }
//! ```

// 核心模块
pub mod datatypes;
pub mod record;
pub mod group;
pub mod plugin;
pub mod subrecord;
pub mod string_types;
pub mod string_file;
pub mod utils;

// IO 抽象层（v0.4.0 新增）
pub mod io;

// 编辑器层（v0.4.0 新增）
pub mod editor;

// 本地化插件支持（v0.4.0 新增）
pub mod localized_context;

// 智能插件加载器（v0.4.0 新增）
pub mod plugin_loader;

// 调试模块（仅在debug模式下可用）
#[cfg(debug_assertions)]
pub mod debug;

// === 公共接口导出 ===

// 主要结构体
pub use plugin::Plugin;
pub use record::Record;
pub use group::{Group, GroupChild, GroupType};
pub use subrecord::Subrecord;
pub use string_types::ExtractedString;
pub use string_file::{StringFile, StringFileType, StringEntry, StringFileSet, StringFileStats};

// 数据类型和工具
pub use datatypes::{RecordFlags, RawString};
pub use utils::{is_valid_string, EspError};

// IO 层导出（v0.4.0 新增）
pub use io::{
    EspReader, EspWriter, StringFileReader, StringFileWriter, RawEspData,
    DefaultEspReader, DefaultEspWriter,
};

// 编辑器层导出（v0.4.0 新增）
pub use editor::{PluginEditor, TranslationDelta, RecordChange, RecordId};

// 本地化插件支持导出（v0.4.0 新增）
pub use localized_context::LocalizedPluginContext;

// 智能加载器导出（v0.4.0 新增）
pub use plugin_loader::LoadedPlugin;

// 调试工具（仅debug模式）
#[cfg(debug_assertions)]
pub use debug::EspDebugger;

// === 常量定义 ===

/// 支持的文件扩展名
pub const SUPPORTED_EXTENSIONS: &[&str] = &["esp", "esm", "esl"];

/// 库版本信息
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

// === 便捷函数 ===

/// 快速提取文件中的字符串
/// 
/// # 参数
/// 
/// * `file_path` - ESP/ESM/ESL文件路径
/// 
/// # 返回
/// 
/// 返回提取到的字符串列表
/// 
/// # 示例
/// 
/// ```rust,no_run
/// use esp_extractor::extract_strings_from_file;
/// use std::path::PathBuf;
/// 
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let strings = extract_strings_from_file(PathBuf::from("example.esp"))?;
/// println!("提取到 {} 个字符串", strings.len());
/// # Ok(())
/// # }
/// ```
pub fn extract_strings_from_file(file_path: std::path::PathBuf) -> std::result::Result<Vec<ExtractedString>, Box<dyn std::error::Error>> {
    let plugin = Plugin::new(file_path, None)?; // 使用默认语言
    Ok(plugin.extract_strings())
}

/// 快速应用翻译到文件
/// 
/// # 参数
/// 
/// * `input_path` - 输入文件路径
/// * `output_path` - 输出文件路径  
/// * `translations` - 翻译字符串列表
/// 
/// # 示例
/// 
/// ```rust,no_run
/// use esp_extractor::{apply_translations_to_file, ExtractedString};
/// use std::path::PathBuf;
/// 
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let translations: Vec<ExtractedString> = vec![]; // 你的翻译数据
/// apply_translations_to_file(
///     PathBuf::from("input.esp"),
///     PathBuf::from("output.esp"),
///     translations
/// )?;
/// # Ok(())
/// # }
/// ```
pub fn apply_translations_to_file(
    input_path: std::path::PathBuf,
    output_path: std::path::PathBuf,
    translations: Vec<ExtractedString>
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    Plugin::apply_translations(input_path, output_path, translations, None) // 使用默认语言
}

/// 验证文件是否为支持的ESP格式
/// 
/// # 参数
/// 
/// * `file_path` - 文件路径
/// 
/// # 返回
/// 
/// 如果文件是支持的格式则返回true
pub fn is_supported_file(file_path: &std::path::Path) -> bool {
    if !file_path.exists() {
        return false;
    }
    
    let extension = file_path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_lowercase());
    
    SUPPORTED_EXTENSIONS.iter().any(|&ext| Some(ext) == extension.as_deref())
}

// === 重新导出错误类型 ===

/// 库的主要错误类型
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

// === 测试模块 ===

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_supported_extensions() {
        assert!(SUPPORTED_EXTENSIONS.contains(&"esp"));
        assert!(SUPPORTED_EXTENSIONS.contains(&"esm"));
        assert!(SUPPORTED_EXTENSIONS.contains(&"esl"));
    }
    
    #[test]
    fn test_version_exists() {
        assert!(!VERSION.is_empty());
    }
} 