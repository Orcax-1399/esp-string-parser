//! 记录类型索引说明
//!
//! ## 统一索引机制
//!
//! 自 v0.6.0 起，所有 string subrecord 都按 Record 内出现顺序分配索引（0, 1, 2...）。
//! 不再区分"特殊"和"普通"记录类型，所有字段都有 index。
//!
//! ## 常见的多字段记录类型
//!
//! 以下记录类型经常包含多个相同类型的 subrecord，因此索引尤为重要：
//!
//! - **MESG**: 消息/对话框记录
//!   - `ITXT`: 按钮/选项文本（可能有多个）
//!   - `DESC`: 消息描述（通常只有 1 个）
//!   - `FULL`: 消息标题（通常只有 1 个）
//!
//! - **INFO**: 对话信息记录
//!   - `NAM1`: 对话选项文本（可能有多个）
//!   - `RNAM`: 对话响应文本（可能有多个）
//!
//! - **PERK**: 技能/特长记录
//!   - `EPF2`: 技能等级 2 描述（可能有多个）
//!   - `EPF3`: 技能等级 3 描述（可能有多个）
//!
//! - **QUST**: 任务记录
//!   - `CNAM`: 任务目标/条件（可能有多个）
//!   - `NNAM`: 任务描述（可能有多个）
//!
//! ## 索引分配示例
//!
//! ```text
//! MESG Record:
//!   ┌─────────────┬───────┐
//!   │ Subrecord   │ Index │
//!   ├─────────────┼───────┤
//!   │ DESC        │   0   │  ← "请选择楼层"
//!   │ ITXT        │   1   │  ← "1楼"
//!   │ ITXT        │   2   │  ← "2楼"
//!   │ ITXT        │   3   │  ← "3楼"
//!   │ ITXT        │   4   │  ← "取消"
//!   └─────────────┴───────┘
//! ```
//!
//! ## 唯一键格式
//!
//! 所有 ExtractedString 的唯一键格式为：
//! ```text
//! {editor_id}|{form_id}|{record_type} {subrecord_type}|{index}
//! ```
//!
//! 示例：
//! ```text
//! DimRiftSpellMenuDormsMSG|0539C5C2|GostedDimensionalRift.esp|MESG ITXT|1
//! DimRiftSpellMenuDormsMSG|0539C5C2|GostedDimensionalRift.esp|MESG ITXT|2
//! ```

/// 特殊记录处理器（已简化为文档模块）
///
/// 自 v0.6.0 起，不再需要特殊的记录处理逻辑。
/// 所有记录都使用统一的索引分配机制。
pub struct SpecialRecordHandler;

impl SpecialRecordHandler {
    /// 常见的多字段记录类型（仅供参考）
    ///
    /// 这些记录类型经常包含多个相同类型的 subrecord。
    /// 注意：这只是信息性常量，不影响实际处理逻辑。
    pub const MULTI_FIELD_TYPES: &'static [&'static str] = &["MESG", "INFO", "PERK", "QUST"];
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multi_field_types_constant() {
        // 验证常量定义正确
        assert!(SpecialRecordHandler::MULTI_FIELD_TYPES.contains(&"MESG"));
        assert!(SpecialRecordHandler::MULTI_FIELD_TYPES.contains(&"INFO"));
        assert!(SpecialRecordHandler::MULTI_FIELD_TYPES.contains(&"PERK"));
        assert!(SpecialRecordHandler::MULTI_FIELD_TYPES.contains(&"QUST"));
        assert_eq!(SpecialRecordHandler::MULTI_FIELD_TYPES.len(), 4);
    }
}
