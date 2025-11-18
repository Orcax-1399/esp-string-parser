use serde::{Serialize, Deserialize};

/// 提取的字符串结构
///
/// 此结构用于 ESP 文件的字符串提取和应用：
/// - 提取时：`text` 为 ESP 中的原始文本
/// - 应用时：`text` 为要写入 ESP 的新文本
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedString {
    /// EDID字段(编辑器ID)
    pub editor_id: Option<String>,
    /// 完整FormID (包含主文件)
    pub form_id: String,
    /// 文本内容
    /// - 提取时：ESP 中的原始文本
    /// - 应用时：要写入的新文本
    pub text: String,
    /// 记录类型
    pub record_type: String,
    /// 子记录类型
    pub subrecord_type: String,
    /// 子记录索引（按 Record 内出现顺序分配，从 0 开始）
    /// 所有字段都有索引，即使只有 1 个相同类型的字段
    pub index: i32,
}

impl ExtractedString {
    /// 创建新的提取字符串
    pub fn new(
        editor_id: Option<String>,
        form_id: String,
        record_type: String,
        subrecord_type: String,
        text: String,
        index: i32,
    ) -> Self {
        ExtractedString {
            editor_id,
            form_id,
            text,
            record_type,
            subrecord_type,
            index,
        }
    }

    /// 获取要应用的文本
    pub fn get_text_to_apply(&self) -> &str {
        &self.text
    }
    
    /// 获取字符串类型（动态计算）
    pub fn get_string_type(&self) -> String {
        format!("{} {}", self.record_type, self.subrecord_type)
    }
    
    /// 生成唯一标识符用于匹配
    ///
    /// 格式：{editor_id}|{form_id}|{record_type} {subrecord_type}|{index}
    /// 所有字段都包含 index，确保完全唯一性
    pub fn get_unique_key(&self) -> String {
        format!("{}|{}|{}|{}",
            self.editor_id.as_deref().unwrap_or(""),
            self.form_id,
            self.get_string_type(),
            self.index
        )
    }
} 