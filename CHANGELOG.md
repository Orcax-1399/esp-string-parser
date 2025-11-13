# 变更日志

本文档记录了esp_extractor库的所有重要变更。

## [0.4.0] - 2025-11-13

### 架构重构 🏗️

这是一个重大架构升级版本，引入了 **IO 抽象层**和**编辑器层**，遵循 SOLID 原则，大幅提升代码质量和可维护性。

### 新增 ✨

#### IO 抽象层 (`io` 模块)
- `EspReader` / `EspWriter` trait - ESP 文件 IO 抽象
- `StringFileReader` / `StringFileWriter` trait - STRING 文件 IO 抽象
- `DefaultEspReader` / `DefaultEspWriter` - 基于 std::fs 的默认实现
- 支持依赖注入和 mock 测试

#### 编辑器层 (`editor` 模块)
- `PluginEditor` - Stateful 插件编辑器，支持批量修改和延迟保存
- `TranslationDelta` - 变更追踪系统，支持撤销/重做
- `RecordChange` - 单条记录变更
- `RecordId` - 记录标识符

#### 本地化插件支持
- `LocalizedPluginContext` - 本地化插件上下文，组合 Plugin 和 StringFileSet

#### 智能加载器 (`plugin_loader` 模块) ⭐ 推荐
- **`LoadedPlugin::load_auto()`** - 智能自动加载插件
  - 自动检测 LOCALIZED 标志
  - 本地化插件自动加载 STRING 文件
  - STRING 文件缺失时自动降级为普通模式
  - 提供统一接口，简化使用
- `LoadedPlugin` enum - 统一表示普通/本地化插件

### 变更 🔄

- **`Plugin::new()` → `Plugin::load()`**
  - `Plugin::load(path)` - 只解析 ESP 文件，不加载 STRING 文件（新推荐方式）
  - `Plugin::new(path, language)` 标记为 `#[deprecated]`，将在 v1.0.0 移除

- **内部方法可见性调整**
  - `Plugin::apply_translation_map()` / `write_record()` / `write_group()` 改为 `pub(crate)`
  - 新增 `Plugin::write_to_buffer()` 支持 PluginEditor

### 改进 💡

- **遵循 SOLID 原则**
  - 单一职责：Plugin 不再自动加载 STRING 文件
  - 开闭原则：通过 trait 扩展功能
  - 依赖倒置：面向接口编程

- **修改-保存分离**
  - 所有修改操作仅在内存中进行
  - 需要显式调用 `save()` 才会写入文件

- **Debug trait 实现**
  - `Plugin`, `StringFile`, `StringFileSet` 实现 `Debug`

### 代码示例 📝

#### 旧 API (v0.3.x)
```rust
// 自动检测并加载 STRING 文件
let plugin = Plugin::new(path, Some("english"))?;
let strings = plugin.extract_strings();
```

#### 新 API (v0.4.0) - 推荐方式 ⭐
```rust
// 方式 1: 智能自动加载（最推荐）
let loaded = LoadedPlugin::load_auto(path, Some("english"))?;
let strings = loaded.extract_strings();

// 方式 2: 精确控制（只需要 ESP 结构）
let plugin = Plugin::load(path)?;
let strings = plugin.extract_strings();

// 方式 3: 明确的本地化插件
let context = LocalizedPluginContext::load(path, "english")?;
let strings = context.plugin().extract_strings();
```

### 文档 📖

- 新增 `docs/plugin-loading-guide.md` - 完整的插件加载指南
  - 三种加载方式详细对比
  - 使用场景流程图
  - 实际示例代码
  - 迁移指南

### 向后兼容性 ⚠️

- ✅ **向后兼容**：旧 API 标记为 deprecated 但仍可用
- ⚠️ **Breaking Changes (v1.0.0)**：`Plugin::new()` 将被移除
- 📚 **推荐升级**：使用 `LoadedPlugin::load_auto()` 获得最佳体验

## [0.2.0] - 2024-12-19

### 新增功能
- 🎯 **库模式**：将项目重构为可重用的Rust库
- 📝 **完整API文档**：添加了详细的文档和使用示例
- ⚡ **便捷函数**：提供了`extract_strings_from_file`和`apply_translations_to_file`等便捷API
- 🔧 **文件对比工具**：新增`--compare-files`功能，可对比两个ESP文件的结构差异
- 🎨 **特性分离**：CLI功能现在是可选特性，库可以独立使用

### 重大修复
- 🐛 **GRUP大小计算修复**：解决了GRUP头部大小计算错误导致的4字节差异问题
- 📦 **压缩记录处理**：完全重写了压缩记录的解析和重建逻辑
- 🔄 **翻译应用优化**：修复了翻译应用过程中的多个关键问题

### 技术改进
- 🚀 **代码优化**：大幅减少代码重复，提高可维护性
- 🧪 **测试完善**：添加了完整的单元测试和文档测试
- 📚 **示例程序**：提供了完整的使用示例

### 破坏性变更
- API接口重新设计，与0.1.0版本不兼容
- 命令行参数略有调整

## [0.1.0] - 2024-12-18

### 初始版本
- ✨ 基本的ESP/ESM/ESL文件解析功能
- 🔤 字符串提取功能
- 🌍 翻译应用功能
- 📊 文件统计信息
- 🐛 调试和重建工具

### 已知问题
- GRUP大小计算不准确
- 压缩记录处理不完善
- 缺少完整的错误处理 