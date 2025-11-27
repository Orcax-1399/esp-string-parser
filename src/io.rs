/// IO 抽象层模块
///
/// 该模块提供了文件读写的抽象接口，遵循依赖倒置原则。
/// 支持依赖注入、测试 mock 和替换 IO 实现（如内存 IO、网络 IO 等）。
///
/// # 架构设计
///
/// - **traits**: 定义 Reader/Writer trait 接口
/// - **esp_io**: ESP 文件的默认实现
/// - **string_file_io**: STRING 文件的默认实现
///
/// # 使用示例
///
/// ```rust,ignore
/// use esp_extractor::io::{DefaultEspReader, EspReader};
///
/// let reader = DefaultEspReader;
/// let data = reader.read(Path::new("example.esp"))?;
/// ```
pub mod traits;
pub mod esp_io;
pub mod string_file_io;

// === 导出 trait 定义 ===
pub use traits::{
    EspReader, EspWriter, RawEspData, StringFileReader, StringFileSetReader, StringFileWriter,
};

// === 导出默认实现 ===
pub use esp_io::{DefaultEspReader, DefaultEspWriter};
pub use string_file_io::{
    DefaultStringFileReader, DefaultStringFileSetReader, DefaultStringFileWriter,
};

