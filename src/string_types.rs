use serde::{Serialize, Deserialize};

/// 提取的字符串结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedString {
    /// EDID字段(编辑器ID)
    pub editor_id: Option<String>,
    /// 完整FormID (包含主文件)
    pub form_id: String,
    /// 字符串类型，如"WEAP FULL"
    pub string_type: String,
    /// 原始文本（提取时为原文，导入时为翻译文本）
    pub original_text: String,
    /// 字符串索引(用于多字符串记录)
    pub index: Option<u32>,
    /// 记录类型
    pub record_type: String,
    /// 子记录类型
    pub subrecord_type: String,
    /// 字符串编码
    pub encoding: String,
}

impl ExtractedString {
    /// 创建新的提取字符串
    pub fn new(
        editor_id: Option<String>,
        form_id: String,
        record_type: String,
        subrecord_type: String,
        original_text: String,
        encoding: String,
    ) -> Self {
        let string_type = format!("{} {}", record_type, subrecord_type);
        
        ExtractedString {
            editor_id,
            form_id,
            string_type,
            original_text,
            index: None,
            record_type,
            subrecord_type,
            encoding,
        }
    }
    
    /// 设置字符串索引
    pub fn with_index(mut self, index: u32) -> Self {
        self.index = Some(index);
        self
    }
    
    /// 生成唯一标识符用于匹配
    pub fn get_unique_key(&self) -> String {
        format!("{}|{}|{}", 
            self.editor_id.as_deref().unwrap_or(""), 
            self.form_id, 
            self.string_type
        )
    }
} 