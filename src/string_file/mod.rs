mod bsa;
mod file;
mod io;
mod set;

#[cfg(test)]
mod tests;

pub use file::StringFile;
pub use set::{StringFileSet, StringFileStats};

/// Bethesda字符串文件类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StringFileType {
    /// 对话字符串文件
    DLSTRINGS,
    /// 界面字符串文件
    ILSTRINGS,
    /// 一般字符串文件
    STRINGS,
}

impl StringFileType {
    /// 从文件扩展名获取字符串文件类型
    pub fn from_extension(extension: &str) -> Option<Self> {
        match extension.to_uppercase().as_str() {
            "DLSTRINGS" => Some(StringFileType::DLSTRINGS),
            "ILSTRINGS" => Some(StringFileType::ILSTRINGS),
            "STRINGS" => Some(StringFileType::STRINGS),
            _ => None,
        }
    }

    /// 获取文件扩展名
    pub fn to_extension(&self) -> &'static str {
        match self {
            StringFileType::DLSTRINGS => "DLSTRINGS",
            StringFileType::ILSTRINGS => "ILSTRINGS",
            StringFileType::STRINGS => "STRINGS",
        }
    }

    /// 检查是否需要长度前缀（DLSTRINGS和ILSTRINGS需要）
    pub fn has_length_prefix(&self) -> bool {
        matches!(self, StringFileType::DLSTRINGS | StringFileType::ILSTRINGS)
    }
}

/// 字符串数据对象
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StringEntry {
    /// 字符串ID
    pub id: u32,
    /// 目录条目在文件中的位置
    pub directory_address: u64,
    /// 相对偏移量
    pub relative_offset: u32,
    /// 绝对偏移量（字符串数据起始位置）
    pub absolute_offset: u64,
    /// 字符串长度（仅对DLSTRINGS/ILSTRINGS有效）
    pub length: Option<u32>,
    /// 字符串内容（UTF-8编码）
    pub content: String,
    /// 原始字节数据
    pub raw_data: Vec<u8>,
}

impl StringEntry {
    /// 创建新的字符串条目
    pub fn new(id: u32, content: String) -> Self {
        let raw_data = content.as_bytes().to_vec();
        Self {
            id,
            directory_address: 0,
            relative_offset: 0,
            absolute_offset: 0,
            length: Some(raw_data.len() as u32),
            content,
            raw_data,
        }
    }

    /// 获取字符串的总大小（包括长度前缀和空终止符）
    pub fn get_total_size(&self, file_type: &StringFileType) -> u32 {
        // 使用content的实际字节长度，而不是raw_data，确保一致性
        let content_size = self.content.len() as u32;
        let null_terminator = 1u32; // 空终止符

        if file_type.has_length_prefix() {
            4 + content_size + null_terminator // 长度前缀(4) + 内容 + 空终止符
        } else {
            content_size + null_terminator // 内容 + 空终止符
        }
    }
}
