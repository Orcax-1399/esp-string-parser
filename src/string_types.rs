use serde::{Serialize, Deserialize};

/// 提取的字符串结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedString {
    /// EDID字段(编辑器ID)
    pub editor_id: Option<String>,
    /// 完整FormID (包含主文件)
    pub form_id: String,
    /// 原始文本
    pub original_text: String,
    /// 翻译文本（可选，应用翻译时使用）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub translated_text: Option<String>,
    /// 记录类型
    pub record_type: String,
    /// 子记录类型
    pub subrecord_type: String,
    /// 子记录索引（用于特殊记录类型如 INFO/PERK/QUST）
    /// None 表示普通记录，Some(i) 表示特殊记录的第 i 个子记录
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<i32>,
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
            translated_text: None,
            record_type,
            subrecord_type,
            index: None,
        }
    }

    /// 创建带索引的提取字符串（用于特殊记录）
    pub fn new_with_index(
        editor_id: Option<String>,
        form_id: String,
        record_type: String,
        subrecord_type: String,
        original_text: String,
        index: i32,
    ) -> Self {
        ExtractedString {
            editor_id,
            form_id,
            original_text,
            translated_text: None,
            record_type,
            subrecord_type,
            index: Some(index),
        }
    }

    /// 获取要应用的文本（优先使用翻译文本，否则使用原文）
    pub fn get_text_to_apply(&self) -> &str {
        self.translated_text.as_deref().unwrap_or(&self.original_text)
    }
    
    /// 获取字符串类型（动态计算）
    pub fn get_string_type(&self) -> String {
        format!("{} {}", self.record_type, self.subrecord_type)
    }
    
    /// 生成唯一标识符用于匹配
    pub fn get_unique_key(&self) -> String {
        if let Some(index) = self.index {
            // 特殊记录：包含索引
            format!("{}|{}|{}|{}",
                self.editor_id.as_deref().unwrap_or(""),
                self.form_id,
                self.get_string_type(),
                index
            )
        } else {
            // 普通记录：无索引
            format!("{}|{}|{}",
                self.editor_id.as_deref().unwrap_or(""),
                self.form_id,
                self.get_string_type()
            )
        }
    }
} 