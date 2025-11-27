/// 编辑器层模块
///
/// 该模块提供有状态的编辑接口，支持变更追踪、撤销/重做等高级功能。
/// 遵循"修改-保存分离"原则，所有修改操作仅在内存中进行，需要显式调用保存。
///
/// # 架构设计
///
/// - **plugin_editor**: 插件编辑器，管理 Plugin 的修改状态
/// - **delta**: 变更追踪系统，支持撤销/重做
///
/// # 使用示例
///
/// ```rust,ignore
/// use esp_extractor::{Plugin, PluginEditor};
/// use esp_extractor::io::DefaultEspWriter;
///
/// // 加载 + 编辑 + 保存工作流
/// let plugin = Plugin::load("example.esp")?;
/// let mut editor = PluginEditor::new(plugin);
///
/// editor.apply_translations(translations)?;
/// println!("修改了 {} 处", editor.modified_count());
///
/// let writer = DefaultEspWriter;
/// editor.save(&writer, Path::new("output.esp"))?;
/// ```
pub mod delta;
pub mod plugin_editor;

// === 导出公共接口 ===
pub use delta::{RecordChange, RecordId, TranslationDelta};
pub use plugin_editor::PluginEditor;

