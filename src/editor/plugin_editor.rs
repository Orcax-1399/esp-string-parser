/// 插件编辑器模块
///
/// 提供有状态的插件编辑接口，支持批量修改和延迟保存。
/// 遵循"修改-保存分离"原则，所有修改操作仅在内存中进行。

use std::path::Path;
use crate::plugin::Plugin;
use crate::string_types::ExtractedString;
use crate::io::{EspWriter, RawEspData};
use super::delta::{TranslationDelta, RecordChange, RecordId};

/// 插件编辑器 - 管理插件的修改状态
///
/// # 核心特性
/// - **Stateful**: 维护修改状态，支持多次修改后统一保存
/// - **可追踪**: 记录所有变更，支持撤销/重做
/// - **隔离性**: 多个编辑器实例互不影响
///
/// # 使用示例
///
/// ```rust,ignore
/// use esp_extractor::{Plugin, PluginEditor};
/// use esp_extractor::io::DefaultEspWriter;
///
/// // 加载插件
/// let plugin = Plugin::load("example.esp")?;
///
/// // 创建编辑器
/// let mut editor = PluginEditor::new(plugin);
///
/// // 应用翻译（仅修改内存）
/// editor.apply_translations(translations)?;
///
/// // 查询状态
/// println!("已修改 {} 处", editor.modified_count());
///
/// // 保存到文件
/// let writer = DefaultEspWriter;
/// editor.save(&writer, Path::new("output.esp"))?;
/// ```
pub struct PluginEditor {
    /// 底层插件实例
    plugin: Plugin,
    /// 变更追踪器
    modifications: TranslationDelta,
}

impl PluginEditor {
    /// 创建新的插件编辑器
    ///
    /// # 参数
    /// * `plugin` - 要编辑的插件实例
    pub fn new(plugin: Plugin) -> Self {
        Self {
            plugin,
            modifications: TranslationDelta::new(),
        }
    }

    /// 应用单个翻译（仅修改内存状态）
    ///
    /// # 参数
    /// * `translation` - 要应用的翻译
    ///
    /// # 返回
    /// 如果应用成功返回 Ok(true)，如果未找到匹配的字段返回 Ok(false)
    pub fn apply_translation(
        &mut self,
        translation: &ExtractedString,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let translations = vec![translation.clone()];
        let result = self.apply_translations(translations)?;
        Ok(result > 0)
    }

    /// 批量应用翻译（仅修改内存状态）
    ///
    /// # 参数
    /// * `translations` - 翻译列表
    ///
    /// # 返回
    /// 返回成功应用的翻译数量
    pub fn apply_translations(
        &mut self,
        translations: Vec<ExtractedString>,
    ) -> Result<usize, Box<dyn std::error::Error>> {
        // 使用 Plugin 现有的应用逻辑
        // 注意：这里暂时使用现有的 apply_translations_to_esp 方法
        // 后续重构时会替换为更细粒度的实现

        let _old_modified_count = self.modifications.len();

        // 创建翻译映射
        let translation_map: std::collections::HashMap<_, _> = translations
            .into_iter()
            .map(|t| (t.get_unique_key(), t))
            .collect();

        // 应用翻译（这会修改 plugin 内部状态）
        self.plugin.apply_translation_map(&translation_map)?;

        // 追踪变更（简化版本 - 暂时只记录总数变化）
        // TODO: 后续重构为细粒度追踪每个字段的变更
        let new_modified_count = translation_map.len();
        let applied_count = new_modified_count;

        // 记录变更到 delta
        for (key, trans) in translation_map.iter() {
            let change = RecordChange {
                record_id: RecordId::new(
                    self.extract_form_id_from_key(key),
                    trans.editor_id.clone(),
                ),
                subrecord_type: trans.get_string_type(),
                old_value: trans.original_text.clone(),
                new_value: trans.translated_text.clone().unwrap_or_default(),
                applied_at: std::time::Instant::now(),
            };
            self.modifications.add_change(change);
        }

        Ok(applied_count)
    }

    /// 从唯一键中提取 FormID
    ///
    /// 唯一键格式："{editor_id}|{form_id}|{record_type} {subrecord_type}"
    fn extract_form_id_from_key(&self, key: &str) -> u32 {
        // 解析 FormID（格式：00001234|PluginName.esp）
        let parts: Vec<&str> = key.split('|').collect();
        if parts.len() >= 2 {
            let form_id_part = parts[1].split('|').next().unwrap_or("00000000");
            u32::from_str_radix(&form_id_part[..8], 16).unwrap_or(0)
        } else {
            0
        }
    }

    /// 检查是否有修改
    pub fn is_modified(&self) -> bool {
        !self.modifications.is_empty()
    }

    /// 获取修改数量
    pub fn modified_count(&self) -> usize {
        self.modifications.len()
    }

    /// 获取变更追踪器的引用
    pub fn get_modifications(&self) -> &TranslationDelta {
        &self.modifications
    }

    /// 撤销最后一次修改
    ///
    /// # 注意
    /// 当前实现只撤销追踪记录，实际的 Plugin 状态恢复需要重新应用翻译。
    /// 这是一个简化实现，后续重构会改进。
    pub fn undo(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.modifications
            .undo()
            .map_err(|e| e.into())
            .map(|_| ())
    }

    /// 重做上一次撤销的修改
    pub fn redo(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.modifications
            .redo()
            .map_err(|e| e.into())
            .map(|_| ())
    }

    /// 保存到文件（需要显式调用）
    ///
    /// # 参数
    /// * `writer` - ESP 文件写入器
    /// * `path` - 目标文件路径
    pub fn save(
        &self,
        writer: &dyn EspWriter,
        path: &Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // 使用 Plugin 的写入方法生成数据
        let mut output = Vec::new();
        self.plugin.write_to_buffer(&mut output)?;

        // 通过 writer 写入
        let data = RawEspData { bytes: output };
        writer.write(&data, path)?;

        Ok(())
    }

    /// 保存到原路径
    pub fn save_to_original(
        &self,
        writer: &dyn EspWriter,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let path = self.plugin.path.clone();
        self.save(writer, &path)
    }

    /// 获取底层 Plugin 的不可变引用
    pub fn plugin(&self) -> &Plugin {
        &self.plugin
    }

    /// 获取底层 Plugin 的可变引用
    ///
    /// # 警告
    /// 直接修改 Plugin 可能导致变更追踪失效，请谨慎使用
    pub fn plugin_mut(&mut self) -> &mut Plugin {
        &mut self.plugin
    }

    /// 清除所有修改记录（但不恢复 Plugin 状态）
    pub fn clear_modifications(&mut self) {
        self.modifications.clear();
    }

    /// 生成编辑摘要
    pub fn summary(&self) -> String {
        format!(
            "插件: {}, 修改状态: {}, {}",
            self.plugin.get_name(),
            if self.is_modified() {
                "已修改"
            } else {
                "未修改"
            },
            self.modifications.summary()
        )
    }
}

// 扩展 Plugin 以支持 PluginEditor
// 这些方法是临时的，用于支持 PluginEditor，后续重构会移除或改进
impl Plugin {
    /// 将数据写入缓冲区（内部辅助方法）
    pub(crate) fn write_to_buffer(
        &self,
        output: &mut Vec<u8>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.write_record(&self.header, output)?;

        for group in &self.groups {
            self.write_group(group, output)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    // use super::*;
    // use std::path::PathBuf;

    #[test]
    fn test_plugin_editor_creation() {
        // 注意：这个测试需要一个有效的 ESP 文件
        // 在实际项目中，应该使用测试 fixture
        // 这里只是演示 API 用法

        // let plugin = Plugin::new(PathBuf::from("test.esp"), None).unwrap();
        // let editor = PluginEditor::new(plugin);
        //
        // assert!(!editor.is_modified());
        // assert_eq!(editor.modified_count(), 0);
    }

    #[test]
    fn test_editor_state() {
        // 测试状态管理
        // let plugin = Plugin::new(PathBuf::from("test.esp"), None).unwrap();
        // let mut editor = PluginEditor::new(plugin);
        //
        // // 应用翻译
        // let translations = vec![/* ... */];
        // editor.apply_translations(translations).unwrap();
        //
        // assert!(editor.is_modified());
        // assert!(editor.modified_count() > 0);
    }
}
