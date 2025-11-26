# 变更日志

本文档记录了esp_extractor库的所有重要变更。

## [0.7.0] - 2025-11-27

### 代码架构重构

本版本专注于代码架构优化，将大文件拆分为职责清晰的小模块，提升可维护性。

#### 模块拆分

**plugin.rs 拆分** (1264 行 → 7 个文件)
```
src/plugin/
├── mod.rs           # Plugin 结构体和公共接口 (150 行)
├── parser.rs        # 加载和解析逻辑 (240 行)
├── strings.rs       # 字符串提取 (180 行)
├── translate.rs     # 翻译应用 (380 行)
├── writer.rs        # 文件写入 (130 行)
├── stats.rs         # 统计信息 (70 行)
└── esl.rs           # ESL FormID 重编号 (80 行)
```

**string_file.rs 拆分** (1119 行 → 6 个文件)
```
src/string_file/
├── mod.rs           # 公共接口和基础类型 (95 行)
├── file.rs          # StringFile 结构体和方法 (380 行)
├── set.rs           # StringFileSet 集合操作 (320 行)
├── bsa.rs           # BSA fallback 逻辑 (70 行)
├── io.rs            # 文件名解析工具 (54 行)
└── tests.rs         # 测试模块 (271 行)
```

#### 新增模块

**字符串路由模块** (`src/string_routes/`)
- `StringRouter` trait: 定义字符串路由接口
- `DefaultStringRouter`: 基于 `string_records.json` 的默认实现
- 支持自定义路由实现，便于扩展

```rust
pub trait StringRouter: Send + Sync + Debug {
    fn get_string_subrecord_types(&self, record_type: &str) -> Option<&[String]>;
    fn supports_strings(&self, record_type: &str, subrecord_type: &str) -> bool;
}
```

#### IO 抽象层增强

新增依赖注入方法，支持自定义 IO 实现：
- `Plugin::load_with_reader()`: 使用自定义 reader 加载插件
- `StringFileSet::load_from_directory_with_reader()`: 使用自定义 reader 加载字符串文件
- 原有方法保持向后兼容

#### 测试验证

- 所有 54 个库测试通过
- Clippy 检查通过
- 向后兼容：公共 API 保持不变

#### 重构效果

- 代码行数：2383 行 → 分散到 13 个文件（平均 ~183 行/文件）
- 职责清晰：每个模块负责特定功能
- 易于维护：相关代码集中在一起
- 易于扩展：StringRouter 和 IO 层可自定义

## [0.6.0] - 2025-11-18

### BSA Fallback 与字段优化

#### 新增功能
- BSA 归档 Fallback 机制：当文件系统找不到 STRING 文件时，自动从 BSA 归档中提取
  - 基于 `ba2` crate 实现 TES4 格式 BSA 读取
  - 路径规范化（大小写容错、路径分隔符统一）
  - 支持官方主文件特殊规则（Skyrim.esm 等共享 `Skyrim - Interface.bsa`）
  - 优先级：文件系统 → BSA → 失败
- 修复 AMMO.DESC 字段缺失：在 `string_records.json` 中添加 AMMO 的 DESC 支持

#### 字段命名优化
- ExtractedString 字段重命名：`original_text` → `text`
  - 统一字段语义：提取时为原文，应用时为译文
- 新增 index 字段：用于同类型子记录的索引区分（v0.5.1 引入）

#### 测试验证
- ccbgssse002-exoticarrows.esl：成功从 BSA 提取 59 个字符串
  - STRINGS: 44 个
  - DLSTRINGS: 15 个（含 2 个 AMMO.DESC）
  - ILSTRINGS: 0 个

#### 架构改进
- 新增 `src/bsa/` 模块：
  - `BsaArchive`: 底层 BSA 文件访问
  - `BsaStringsProvider`: STRING 文件专用提取器
- 集成到 `LoadedPlugin::load_auto()` 工作流
- CLI 工具自动启用 BSA fallback

#### 依赖更新
- 新增：`ba2 = "3"` - Bethesda 归档文件解析

## [0.5.2] - 2025-11-14

### 性能一致性优化

#### 关键修复
- 解决 load_auto 隐性性能问题：修复本地化插件重复加载导致的性能损失
  - 问题原因：`LoadedPlugin::load_auto()` 对本地化插件会加载 ESP 文件两次
  - 优化方案：引入 `LocalizedPluginContext::new_with_plugin()` 复用已加载的 Plugin
- 性能提升：Skyrim.esm (238MB) 加载时间从 3.28 秒降至 1.36 秒（测试环境：Ryzen 7 5800H, NVMe, Release 构建）
- 代码清理：使用条件编译保护所有调试日志，Release 模式输出简洁

#### 架构改进
- 新增 `LocalizedPluginContext::new_with_plugin()` API
- 消除重复的 ESP 文件解析
- 保持向后兼容，原有 API 不受影响
- 新增性能对比测试工具 `examples/performance_comparison.rs`

## [0.5.1] - 2025-11-14

### STRING 类型路由修复

#### 重要修复
- 修复 STRING 文件���型路由规则：简化路由逻辑，提升字符串提取完整性
  - INFO 记录 → 所有子记录路由到 ILSTRINGS
  - DESC/CNAM 子记录 → 无论 record 类型，都路由到 DLSTRINGS
  - 其他所有 → 默认路由到 STRINGS
- 移除无效字段配置：
  - 移除 PERK.EPFD 和 PERK.EPF2（浮点数字段，非字符串）
  - 移除 AMMO.DESC（非字符串字段）
- StringID=0 特殊处理：正确过滤空字符串引用
- 增强调试日志：警告信息包含 record 类型、FormID 和 EditorID

#### 提取率提升
- Skyrim.esm: 从 21,502 条提升到 62,431 条
- Vigilant.esm: 7,610 条
- curio.esl: 81 条

#### 测试覆盖
- 5 个核心路由��则单元测试
- 3 个真实 ESP/ESM/ESL 文件验证
- 文档更新，反映新的简化路由规则

## [0.5.0] - 2025-11-14

### 性能优化版

#### 性能优化
- 显著性能提升：Skyrim.esm (250MB) 加载时间从 240 秒缩短到 9 秒（测试环境：Ryzen 7 5800H, NVMe, Release 构建）
- 内存映射文件（memmap2）：零拷贝文件访问
- Copy-on-Write（Cow）：减少内存拷贝
- 并行处理（rayon）：Group 和字符串提取并行化

#### 功能完善
- 完整的 10 种 GroupType 支持
- XXXX 超大子记录处理（> 65535 字节）
- 特殊记录索引跟踪（INFO/QUST/PERK）
- STRING 文件集成

#### 测试验证
- Skyrim.esm 集成测试：9/10 通过
- 成功提取 20,437 条字符串（含 2,835 条特殊记录）
- 加载 67,414 条 STRING 文件条目

## [0.4.0] - 2025-11-13

### 架构重构

这是一个重大架构升级版本，引入了 IO 抽象层和编辑器层，提升代码质量和可维护性。

### 新增

#### IO 抽象层 (`io` 模块)
- `EspReader` / `EspWriter` trait - ESP 文件 IO 抽象
- `StringFileReader` / `StringFileWriter` trait - STRING 文件 IO 抽象
- `DefaultEspReader` / `DefaultEspWriter` - 基于 std::fs 的默认实现
- 支持依赖注入和 mock 测试

#### 编辑器层 (`editor` 模块)
- `PluginEditor` - 插件编辑器，支持批量修改和延迟保存
- `TranslationDelta` - 变更追踪系统
- `RecordChange` - 单条记录变更
- `RecordId` - 记录标识符

#### 本地化插件支持
- `LocalizedPluginContext` - 本地化插件上下文，组合 Plugin 和 StringFileSet

#### 智能加载器 (`plugin_loader` 模块)
- `LoadedPlugin::load_auto()` - 自动加载插件
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

### 改进

- 架构调整：引入 trait 抽象以便替换 IO/字符串来源
  - 单一职责：Plugin 不再自动加载 STRING 文件
  - 通过 trait 扩展功能
  - 面向接口编程

- 修改-保存分离
  - 所有修改操作仅在内存中进行
  - 需要显式调用 `save()` 才会写入文件

- Debug trait 实现
  - `Plugin`, `StringFile`, `StringFileSet` 实现 `Debug`

### 代码示例

#### 旧 API (v0.3.x)
```rust
// 自动检测并加载 STRING 文件
let plugin = Plugin::new(path, Some("english"))?;
let strings = plugin.extract_strings();
```

#### 新 API (v0.4.0)
```rust
// 方式 1: 自动加载
let loaded = LoadedPlugin::load_auto(path, Some("english"))?;
let strings = loaded.extract_strings();

// 方式 2: 精确控制（只需要 ESP 结构）
let plugin = Plugin::load(path)?;
let strings = plugin.extract_strings();

// 方式 3: 明确的本地化插件
let context = LocalizedPluginContext::load(path, "english")?;
let strings = context.plugin().extract_strings();
```

### 文档

- 新增 `docs/plugin-loading-guide.md` - 完整的插件加载指南
  - 三种加载方式详细对比
  - 使用场景流程图
  - 实际示例代码
  - 迁移指南

### 向后兼容性

- 向后兼容：旧 API 标记为 deprecated 但仍可用
- Breaking Changes (v1.0.0)：`Plugin::new()` 将被移除
- 推荐升级：使用 `LoadedPlugin::load_auto()` 获得最佳体验

## [0.2.0] - 2024-12-19

### 新增功能
- 库模式：将项目重构为可重用的Rust库
- 完整API文档：添加了详细的文档和使用示例
- 便捷函数：提供了`extract_strings_from_file`和`apply_translations_to_file`等便捷API
- 文件对比工具：新增`--compare-files`功能，可对比两个ESP文件的结构差异
- 特性分离：CLI功能现在是可选特性，库可以独立使用

### 重大修复
- GRUP大小计算修复：解决了GRUP头部大小计算错误导致的4字节差异问题
- 压缩记录处理：完全重写了压缩记录的解析和重建逻辑
- 翻译应用优化：修复了翻译应用过程中的多个关键问题

### 技术改进
- 代码优化：大幅减少代码重复，提高可维护性
- 测试完善：添加了完整的单元测试和文档测试
- 示例程序：提供了完整的使用示例

### 破坏性变更
- API接口重新设计，与0.1.0版本不兼容
- 命令行参数略有调整

## [0.1.0] - 2024-12-18

### 初始版本
- 基本的ESP/ESM/ESL文件解析功能
- 字符串提取功能
- 翻译应用功能
- 文件统计信息
- 调试和重建工具

### 已知问题
- GRUP大小计算不准确
- 压缩记录处理不完善
- 缺少完整的错误处理 