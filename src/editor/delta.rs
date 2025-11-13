/// 翻译变更追踪模块
///
/// 该模块实现变更追踪系统，支持撤销/重做功能。
/// 记录所有对插件进行的修改操作，便于审计和回滚。

use std::time::Instant;

/// 翻译变更追踪器
///
/// # 功能
/// - 记录所有翻译修改操作
/// - 支持撤销/重做
/// - 提供变更历史查询
///
/// # 实现细节
/// - 使用两个栈实现撤销/重做：undo_stack 和 redo_stack
/// - 所有变更按时间顺序存储在 changes 向量中
/// - 栈中存储的是索引而非实际数据，避免数据拷贝
#[derive(Debug, Clone)]
pub struct TranslationDelta {
    /// 所有变更的完整记录
    changes: Vec<RecordChange>,
    /// 撤销栈（存储 changes 中的索引）
    undo_stack: Vec<usize>,
    /// 重做栈（存储 changes 中的索引）
    redo_stack: Vec<usize>,
}

/// 单个记录的变更
///
/// 记录单个字段的修改前后值，用于支持撤销/重做
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordChange {
    /// 记录标识符
    pub record_id: RecordId,
    /// 子记录类型（如 "FULL", "DESC" 等）
    pub subrecord_type: String,
    /// 修改前的值
    pub old_value: String,
    /// 修改后的值
    pub new_value: String,
    /// 应用时间戳
    pub applied_at: Instant,
}

/// 记录标识符
///
/// 用于唯一标识一个记录，支持通过 FormID 或 EditorID 查找
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RecordId {
    /// FormID（必须）
    pub form_id: u32,
    /// 编辑器 ID（可选）
    pub editor_id: Option<String>,
}

impl RecordId {
    /// 创建新的记录标识符
    pub fn new(form_id: u32, editor_id: Option<String>) -> Self {
        Self { form_id, editor_id }
    }

    /// 从 FormID 创建
    pub fn from_form_id(form_id: u32) -> Self {
        Self {
            form_id,
            editor_id: None,
        }
    }
}

impl TranslationDelta {
    /// 创建新的变更追踪器
    pub fn new() -> Self {
        Self {
            changes: Vec::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    /// 添加一个变更
    ///
    /// # 行为
    /// - 将变更添加到 changes 列表
    /// - 将索引压入 undo_stack
    /// - 清空 redo_stack（因为新操作会使重做栈失效）
    ///
    /// # 参数
    /// * `change` - 要记录的变更
    pub fn add_change(&mut self, change: RecordChange) {
        let index = self.changes.len();
        self.changes.push(change);
        self.undo_stack.push(index);
        self.redo_stack.clear(); // 新操作清空重做栈
    }

    /// 撤销最后一次操作
    ///
    /// # 返回
    /// 返回被撤销的变更引用，如果没有可撤销的操作则返回错误
    pub fn undo(&mut self) -> Result<&RecordChange, String> {
        let index = self
            .undo_stack
            .pop()
            .ok_or_else(|| "没有可撤销的操作".to_string())?;
        self.redo_stack.push(index);
        Ok(&self.changes[index])
    }

    /// 重做最后一次撤销的操作
    ///
    /// # 返回
    /// 返回被重做的变更引用，如果没有可重做的操作则返回错误
    pub fn redo(&mut self) -> Result<&RecordChange, String> {
        let index = self
            .redo_stack
            .pop()
            .ok_or_else(|| "没有可重做的操作".to_string())?;
        self.undo_stack.push(index);
        Ok(&self.changes[index])
    }

    /// 获取当前有效变更的数量
    ///
    /// 注意：这是撤销栈的大小，不是总变更数
    pub fn len(&self) -> usize {
        self.undo_stack.len()
    }

    /// 检查是否有有效变更
    pub fn is_empty(&self) -> bool {
        self.undo_stack.is_empty()
    }

    /// 获取所有有效变更的迭代器
    ///
    /// 按应用顺序返回当前有效的变更
    pub fn iter(&self) -> impl Iterator<Item = &RecordChange> {
        self.undo_stack.iter().map(|&idx| &self.changes[idx])
    }

    /// 获取所有变更（包括已撤销的）
    pub fn all_changes(&self) -> &[RecordChange] {
        &self.changes
    }

    /// 检查是否可以撤销
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// 检查是否可以重做
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// 清空所有变更
    pub fn clear(&mut self) {
        self.changes.clear();
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    /// 获取特定记录的所有变更
    ///
    /// # 参数
    /// * `record_id` - 记录标识符
    ///
    /// # 返回
    /// 返回该记录的所有有效变更
    pub fn get_changes_for_record(&self, record_id: &RecordId) -> Vec<&RecordChange> {
        self.iter()
            .filter(|change| &change.record_id == record_id)
            .collect()
    }

    /// 生成变更摘要
    ///
    /// # 返回
    /// 返回人类可读的变更摘要字符串
    pub fn summary(&self) -> String {
        format!(
            "变更总数: {}, 有效变更: {}, 可撤销: {}, 可重做: {}",
            self.changes.len(),
            self.undo_stack.len(),
            self.can_undo(),
            self.can_redo()
        )
    }
}

impl Default for TranslationDelta {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for RecordChange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{:08X}] {}: \"{}\" -> \"{}\"",
            self.record_id.form_id,
            self.subrecord_type,
            if self.old_value.len() > 30 {
                format!("{}...", &self.old_value[..30])
            } else {
                self.old_value.clone()
            },
            if self.new_value.len() > 30 {
                format!("{}...", &self.new_value[..30])
            } else {
                self.new_value.clone()
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_change(form_id: u32, old: &str, new: &str) -> RecordChange {
        RecordChange {
            record_id: RecordId::from_form_id(form_id),
            subrecord_type: "FULL".to_string(),
            old_value: old.to_string(),
            new_value: new.to_string(),
            applied_at: Instant::now(),
        }
    }

    #[test]
    fn test_delta_basic() {
        let mut delta = TranslationDelta::new();
        assert_eq!(delta.len(), 0);
        assert!(delta.is_empty());

        delta.add_change(create_test_change(1, "old", "new"));
        assert_eq!(delta.len(), 1);
        assert!(!delta.is_empty());
    }

    #[test]
    fn test_undo_redo() {
        let mut delta = TranslationDelta::new();

        // 添加 3 个变更
        delta.add_change(create_test_change(1, "a", "b"));
        delta.add_change(create_test_change(2, "c", "d"));
        delta.add_change(create_test_change(3, "e", "f"));

        assert_eq!(delta.len(), 3);

        // 撤销一个
        let undone = delta.undo().unwrap();
        assert_eq!(undone.record_id.form_id, 3);
        assert_eq!(delta.len(), 2);

        // 再撤销一个
        delta.undo().unwrap();
        assert_eq!(delta.len(), 1);

        // 重做
        let redone = delta.redo().unwrap();
        assert_eq!(redone.record_id.form_id, 2);
        assert_eq!(delta.len(), 2);
    }

    #[test]
    fn test_new_change_clears_redo() {
        let mut delta = TranslationDelta::new();

        delta.add_change(create_test_change(1, "a", "b"));
        delta.add_change(create_test_change(2, "c", "d"));

        // 撤销
        delta.undo().unwrap();
        assert!(delta.can_redo());

        // 添加新变更应该清空重做栈
        delta.add_change(create_test_change(3, "e", "f"));
        assert!(!delta.can_redo());
    }

    #[test]
    fn test_undo_when_empty() {
        let mut delta = TranslationDelta::new();
        let result = delta.undo();
        assert!(result.is_err());
    }

    #[test]
    fn test_redo_when_empty() {
        let mut delta = TranslationDelta::new();
        let result = delta.redo();
        assert!(result.is_err());
    }

    #[test]
    fn test_get_changes_for_record() {
        let mut delta = TranslationDelta::new();

        let record_id = RecordId::from_form_id(100);
        delta.add_change(RecordChange {
            record_id: record_id.clone(),
            subrecord_type: "FULL".to_string(),
            old_value: "old1".to_string(),
            new_value: "new1".to_string(),
            applied_at: Instant::now(),
        });

        delta.add_change(create_test_change(200, "x", "y"));

        delta.add_change(RecordChange {
            record_id: record_id.clone(),
            subrecord_type: "DESC".to_string(),
            old_value: "old2".to_string(),
            new_value: "new2".to_string(),
            applied_at: Instant::now(),
        });

        let changes = delta.get_changes_for_record(&record_id);
        assert_eq!(changes.len(), 2);
    }

    #[test]
    fn test_clear() {
        let mut delta = TranslationDelta::new();
        delta.add_change(create_test_change(1, "a", "b"));
        delta.add_change(create_test_change(2, "c", "d"));

        delta.clear();

        assert_eq!(delta.len(), 0);
        assert!(delta.is_empty());
        assert!(!delta.can_undo());
        assert!(!delta.can_redo());
    }

    #[test]
    fn test_summary() {
        let mut delta = TranslationDelta::new();
        delta.add_change(create_test_change(1, "a", "b"));
        delta.add_change(create_test_change(2, "c", "d"));

        let summary = delta.summary();
        assert!(summary.contains("变更总数: 2"));
        assert!(summary.contains("有效变更: 2"));
    }
}
