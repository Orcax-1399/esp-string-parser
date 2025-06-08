use serde::{Serialize, Deserialize};

/// 提取的字符串结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedString {
    /// EDID字段(编辑器ID)
    pub editor_id: Option<String>,
    /// 完整FormID (包含主文件)
    pub form_id: String,
    /// 原始文本（提取时为原文，导入时为翻译文本）
    pub original_text: String,
    /// 记录类型
    pub record_type: String,
    /// 子记录类型
    pub subrecord_type: String,
}

impl ExtractedString {
    /// 创建新的提取字符串
    pub fn new(
        editor_id: Option<String>,
        form_id: String,
        record_type: String,
        subrecord_type: String,
        original_text: String,
    ) -> Self {
        ExtractedString {
            editor_id,
            form_id,
            original_text,
            record_type,
            subrecord_type,
        }
    }
    
    /// 获取字符串类型（动态计算）
    pub fn get_string_type(&self) -> String {
        format!("{} {}", self.record_type, self.subrecord_type)
    }
    
    /// 生成唯一标识符用于匹配
    pub fn get_unique_key(&self) -> String {
        format!("{}|{}|{}", 
            self.editor_id.as_deref().unwrap_or(""), 
            self.form_id, 
            self.get_string_type()
        )
    }
} 