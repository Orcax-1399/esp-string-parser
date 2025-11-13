# ESP 字符串解析器 - 架构重构计划

> **版本**: v0.4.0 重构计划
> **创建日期**: 2025-11-13
> **状态**: 待实施

---

## 目录

- [一、当前架构问题分析](#一当前架构问题分析)
- [二、目标架构设计](#二目标架构设计)
- [三、具体重构步骤](#三具体重构步骤)
- [四、API 变更对照表](#四api-变更对照表)
- [五、模块职责重新划分](#五模块职责重新划分)
- [六、测试策略](#六测试策略)
- [七、迁移路径](#七迁移路径)
- [八、风险评估](#八风险评估)
- [九、实施优先级总结](#九实施优先级总结)
- [十、参考实现示例](#十参考实现示例)

---

## 一、当前架构问题分析

### 1.1 紧耦合的"读-改-写"流程

**问题描述**：
- `Plugin::apply_translations()` 是静态方法，直接完成"读取→修改→保存"全流程
- 无法维护多个 Plugin 实例的修改状态
- 翻译应用后立即写入文件，无法批量操作或延迟保存

**代码示例**（当前）：
```rust
// 当前 API - 一次性完成所有操作
Plugin::apply_translations(
    input_path,
    output_path,
    translations,
    language,
)?;
// ↑ 内部完成：读取 → 修改 → 立即写入
```

**问题影响**：
- ❌ 无法支持"修改多次，保存一次"的工作流
- ❌ 无法同时维护多个插件的编辑状态
- ❌ 无法在保存前预览或验证修改

---

### 1.2 职责不清的模块边界

#### Plugin 模块承担过多职责

**当前 Plugin 的职责**：
1. 文件解析 (parse)
2. 字符串提取 (extract)
3. 翻译应用 (apply)
4. 文件写入 (write)
5. StringFile 管理
6. FormID 格式化
7. 统计信息生成

**违反原则**：
- ❌ 单一职责原则 (SRP)
- ❌ 开闭原则 (OCP)
- ❌ 接口隔离原则 (ISP)

#### StringFile 与 Plugin 紧密耦合

**问题代码**：
```rust
pub struct Plugin {
    string_files: Option<StringFileSet>,  // 直接持有
    // ...
}

impl Plugin {
    pub fn new(path: PathBuf, language: Option<&str>) -> Result<Self> {
        // ...
        // 自动加载逻辑硬编码
        let string_files = if is_localized {
            StringFileSet::load_from_directory(...)
        } else {
            None
        };
        // ...
    }
}
```

**问题影响**：
- ❌ 无法独立测试 StringFile
- ❌ 无法复用 StringFile 加载逻辑
- ❌ 难以扩展其他文件格式

---

### 1.3 缺少抽象层

**当前实现**：
- 没有定义读写操作的 trait
- 缺少状态管理的抽象
- ESP 和 STRING 文件的处理逻辑混在一起

**代码体现**：
```rust
// 直接在 Plugin 中硬编码文件操作
impl Plugin {
    pub fn write_to_file(&self, path: PathBuf) -> Result<()> {
        let data = std::fs::read(&self.path)?;  // 直接调用 std::fs
        // ...
    }
}
```

**问题影响**：
- ❌ 难以 mock 测试
- ❌ 无法替换 IO 实现（如内存/网络）
- ❌ 扩展性差

---

### 1.4 测试困难

**问题列举**：
1. 大量静态方法和内部私有方法
2. 全局状态依赖（`string_records` 硬编码加载）
3. 难以 mock 文件系统操作
4. 集成测试需要真实文件

**示例**：
```rust
// 静态方法 - 难以测试
pub fn apply_translations(
    input_path: PathBuf,  // 必须是真实文件路径
    output_path: PathBuf,
    translations: Vec<ExtractedString>,
    language: Option<&str>,
) -> Result<()>
```

---

### 1.5 状态管理混乱

**问题**：
- `Record::is_modified` 标志位存在，但 Plugin 层没有整体状态追踪
- 无法查询"哪些记录被修改了"
- 无法撤销或比较修改前后的差异

**当前代码**：
```rust
pub struct Record {
    pub is_modified: bool,  // 存在但未被充分利用
    // ...
}
```

---

## 二、目标架构设计

### 2.1 核心设计原则

1. **关注点分离 (SoC)**: 解析/修改/序列化各司其职
2. **依赖倒置 (DIP)**: 面向接口编程，减少具体实现依赖
3. **单一职责 (SRP)**: 每个模块只做一件事
4. **可测试性**: 所有模块可独立测试

---

### 2.2 新架构层次

```
┌─────────────────────────────────────────────┐
│          Application Layer (CLI)            │
│  - 命令行参数解析                              │
│  - 工作流编排                                 │
└─────────────────────────────────────────────┘
                     ↓
┌─────────────────────────────────────────────┐
│         Service Layer (服务层)               │
│  - TranslationService                       │
│  - ExtractionService                        │
│  - ComparisonService                        │
└─────────────────────────────────────────────┘
                     ↓
┌─────────────────────────────────────────────┐
│        Domain Layer (领域层)                 │
│  - Plugin (状态管理)                         │
│  - StringFileSet (状态管理)                  │
│  - TranslationDelta (变更追踪)               │
└─────────────────────────────────────────────┘
                     ↓
┌─────────────────────────────────────────────┐
│     Infrastructure Layer (基础设施层)        │
│  - EspReader/EspWriter (IO trait实现)       │
│  - StringFileReader/StringFileWriter        │
│  - RecordParser/GroupParser                 │
└─────────────────────────────────────────────┘
                     ↓
┌─────────────────────────────────────────────┐
│         Data Layer (数据层)                  │
│  - Record, Group, Subrecord (不变数据)       │
│  - StringEntry (不变数据)                    │
└─────────────────────────────────────────────┘
```

---

## 三、具体重构步骤

### 阶段 1: 基础设施层重构 (优先级: ⭐⭐⭐ 核心)

#### 1.1 定义 Reader/Writer Trait

**新增文件**: `src/io/traits.rs`

```rust
/// ESP 文件读取 trait
pub trait EspReader {
    fn read(&self, path: &Path) -> Result<RawEspData>;
}

/// ESP 文件写入 trait
pub trait EspWriter {
    fn write(&self, data: &RawEspData, path: &Path) -> Result<()>;
}

/// STRING 文件读取 trait
pub trait StringFileReader {
    fn read(&self, path: &Path) -> Result<StringFile>;
}

/// STRING 文件写入 trait
pub trait StringFileWriter {
    fn write(&self, file: &StringFile, path: &Path) -> Result<()>;
}
```

---

#### 1.2 实现具体的 Reader/Writer

**新增文件**: `src/io/esp_io.rs`

```rust
pub struct DefaultEspReader;

impl EspReader for DefaultEspReader {
    fn read(&self, path: &Path) -> Result<RawEspData> {
        let data = std::fs::read(path)?;
        Ok(RawEspData { bytes: data })
    }
}

pub struct DefaultEspWriter;

impl EspWriter for DefaultEspWriter {
    fn write(&self, data: &RawEspData, path: &Path) -> Result<()> {
        std::fs::write(path, &data.bytes)?;
        Ok(())
    }
}
```

**新增文件**: `src/io/string_file_io.rs`

```rust
pub struct DefaultStringFileReader;
pub struct DefaultStringFileWriter;

// 实现逻辑...
```

**影响范围**:
- ✅ 新增模块，不影响现有 API
- ✅ 为后续重构打基础

---

### 阶段 2: 解耦 Plugin 与 StringFile (优先级: ⭐⭐⭐ 核心)

#### 2.1 移除 Plugin 对 StringFileSet 的直接持有

**当前设计**:
```rust
pub struct Plugin {
    string_files: Option<StringFileSet>,  // ❌ 紧耦合
    // ...
}
```

**重构后**:
```rust
pub struct Plugin {
    // ✅ 移除 string_files 字段
    pub path: PathBuf,
    pub header: Record,
    pub groups: Vec<Group>,
    pub masters: Vec<String>,
    pub string_records: HashMap<String, Vec<String>>,
    // language 字段也可以移除，或作为元数据保留
}

/// 新增：本地化插件上下文（组合模式）
pub struct LocalizedPluginContext {
    plugin: Plugin,
    string_files: StringFileSet,
}
```

---

#### 2.2 分离加载逻辑

**当前**:
```rust
// ❌ Plugin::new() 自动加载 STRING 文件
impl Plugin {
    pub fn new(path: PathBuf, language: Option<&str>) -> Result<Self> {
        // ... 复杂的自动加载逻辑
    }
}
```

**重构后**:
```rust
// ✅ 基础加载（不加载 STRING）
impl Plugin {
    pub fn load(path: PathBuf) -> Result<Self> {
        // 只解析 ESP 文件本身
    }
}

// ✅ STRING 文件单独加载
impl StringFileSet {
    pub fn load_for_plugin(plugin: &Plugin, language: &str) -> Result<Self> {
        // 根据 plugin 路径和名称加载
    }
}

// ✅ 组合加载（便捷方法）
impl LocalizedPluginContext {
    pub fn load(path: PathBuf, language: &str) -> Result<Self> {
        let plugin = Plugin::load(path)?;
        let string_files = StringFileSet::load_for_plugin(&plugin, language)?;
        Ok(Self { plugin, string_files })
    }
}
```

**影响范围**:
- ⚠️ Breaking Change: 需要更新所有调用点
- ✅ 向后兼容: 保留 `Plugin::new()` 作为 `deprecated` 方法

---

### 阶段 3: Stateful API 设计 (优先级: ⭐⭐⭐ 核心)

#### 3.1 引入 PluginEditor

**新增文件**: `src/editor/plugin_editor.rs`

```rust
/// 插件编辑器 - 管理插件的修改状态
pub struct PluginEditor {
    plugin: Plugin,
    modifications: TranslationDelta,
}

impl PluginEditor {
    /// 创建编辑器
    pub fn new(plugin: Plugin) -> Self {
        Self {
            plugin,
            modifications: TranslationDelta::new(),
        }
    }

    /// 应用单个翻译（仅修改内存状态）
    pub fn apply_translation(&mut self, trans: &ExtractedString) -> Result<()> {
        // 记录修改到 delta
        // 应用到内部 plugin 数据
    }

    /// 批量应用翻译
    pub fn apply_translations(&mut self, trans: Vec<ExtractedString>) -> Result<()> {
        for t in trans {
            self.apply_translation(&t)?;
        }
        Ok(())
    }

    /// 状态查询
    pub fn is_modified(&self) -> bool {
        !self.modifications.is_empty()
    }

    pub fn get_modifications(&self) -> &TranslationDelta {
        &self.modifications
    }

    pub fn modified_count(&self) -> usize {
        self.modifications.len()
    }

    /// 撤销最后一次修改
    pub fn undo(&mut self) -> Result<()> {
        self.modifications.undo()?;
        // 重建 plugin 状态
        Ok(())
    }

    /// 重做
    pub fn redo(&mut self) -> Result<()> {
        self.modifications.redo()?;
        Ok(())
    }

    /// 保存到文件（需要显式调用）
    pub fn save(&self, writer: &dyn EspWriter, path: &Path) -> Result<()> {
        let data = self.plugin.serialize()?;
        writer.write(&data, path)
    }

    /// 保存到原路径
    pub fn save_to_original(&self, writer: &dyn EspWriter) -> Result<()> {
        self.save(writer, &self.plugin.path)
    }

    /// 获取底层 Plugin 的不可变引用
    pub fn plugin(&self) -> &Plugin {
        &self.plugin
    }
}
```

---

#### 3.2 引入 TranslationDelta (变更追踪)

**新增文件**: `src/editor/delta.rs`

```rust
use std::time::Instant;

/// 翻译变更追踪
pub struct TranslationDelta {
    changes: Vec<RecordChange>,
    undo_stack: Vec<usize>,  // 撤销栈（索引）
    redo_stack: Vec<usize>,  // 重做栈
}

/// 单个记录的变更
pub struct RecordChange {
    pub record_id: RecordId,
    pub subrecord_type: String,
    pub old_value: String,
    pub new_value: String,
    pub applied_at: Instant,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RecordId {
    pub form_id: u32,
    pub editor_id: Option<String>,
}

impl TranslationDelta {
    pub fn new() -> Self {
        Self {
            changes: Vec::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    pub fn add_change(&mut self, change: RecordChange) {
        let index = self.changes.len();
        self.changes.push(change);
        self.undo_stack.push(index);
        self.redo_stack.clear();  // 新操作清空重做栈
    }

    pub fn undo(&mut self) -> Result<&RecordChange> {
        let index = self.undo_stack.pop()
            .ok_or("没有可撤销的操作")?;
        self.redo_stack.push(index);
        Ok(&self.changes[index])
    }

    pub fn redo(&mut self) -> Result<&RecordChange> {
        let index = self.redo_stack.pop()
            .ok_or("没有可重做的操作")?;
        self.undo_stack.push(index);
        Ok(&self.changes[index])
    }

    pub fn len(&self) -> usize {
        self.undo_stack.len()
    }

    pub fn is_empty(&self) -> bool {
        self.undo_stack.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &RecordChange> {
        self.undo_stack.iter().map(|&idx| &self.changes[idx])
    }
}
```

**优势**:
- ✅ 支持多个 PluginEditor 实例
- ✅ 修改和保存完全解耦
- ✅ 可追溯修改历史
- ✅ 易于实现撤销/重做

---

### 阶段 4: 重构翻译应用逻辑 (优先级: ⭐⭐⭐ 核心)

#### 4.1 分离本地化/非本地化处理

**当前**: `apply_translations_unified()` 内部 if/else 判断

**重构后**: 使用 trait 统一接口

**新增文件**: `src/editor/applier.rs`

```rust
/// 翻译应用器 trait
pub trait TranslationApplier {
    fn apply(&mut self, translations: Vec<ExtractedString>) -> Result<()>;
    fn save(&self, output: &Path) -> Result<()>;
    fn is_modified(&self) -> bool;
}

/// ESP 文件翻译应用器（普通插件）
pub struct EspTranslationApplier {
    editor: PluginEditor,
    writer: Box<dyn EspWriter>,
}

impl TranslationApplier for EspTranslationApplier {
    fn apply(&mut self, translations: Vec<ExtractedString>) -> Result<()> {
        self.editor.apply_translations(translations)
    }

    fn save(&self, output: &Path) -> Result<()> {
        self.editor.save(self.writer.as_ref(), output)
    }

    fn is_modified(&self) -> bool {
        self.editor.is_modified()
    }
}

/// STRING 文件翻译应用器（本地化插件）
pub struct StringFileTranslationApplier {
    plugin: Plugin,            // 只读，用于映射
    string_files: StringFileSet,
    modified: bool,
}

impl TranslationApplier for StringFileTranslationApplier {
    fn apply(&mut self, translations: Vec<ExtractedString>) -> Result<()> {
        // 1. 遍历 ESP 构建 StringID 映射
        let string_id_map = build_string_id_map(&self.plugin)?;

        // 2. 更新 STRING 文件
        for trans in translations {
            let key = trans.get_unique_key();
            if let Some((file_type, string_id)) = string_id_map.get(&key) {
                let text = trans.get_text_to_apply();
                self.string_files.update_string(*file_type, *string_id, text.to_string())?;
                self.modified = true;
            }
        }
        Ok(())
    }

    fn save(&self, output: &Path) -> Result<()> {
        let string_dir = output.join("strings");
        std::fs::create_dir_all(&string_dir)?;
        self.string_files.write_all(&string_dir)
    }

    fn is_modified(&self) -> bool {
        self.modified
    }
}
```

---

#### 4.2 工厂模式创建 Applier

```rust
impl TranslationApplier {
    /// 为插件创建合适的翻译应用器
    pub fn for_plugin(
        plugin: Plugin,
        language: Option<&str>,
        writer: Box<dyn EspWriter>,
    ) -> Result<Box<dyn TranslationApplier>> {
        if plugin.is_localized() {
            let string_files = StringFileSet::load_for_plugin(
                &plugin,
                language.unwrap_or("english")
            )?;
            Ok(Box::new(StringFileTranslationApplier {
                plugin,
                string_files,
                modified: false,
            }))
        } else {
            let editor = PluginEditor::new(plugin);
            Ok(Box::new(EspTranslationApplier { editor, writer }))
        }
    }
}
```

---

### 阶段 5: 服务层设计 (优先级: ⭐⭐ 增强)

**新增文件**: `src/services/translation.rs`

```rust
/// 翻译服务 - 高层业务逻辑封装
pub struct TranslationService {
    reader: Box<dyn EspReader>,
    writer: Box<dyn EspWriter>,
}

impl TranslationService {
    pub fn new(reader: Box<dyn EspReader>, writer: Box<dyn EspWriter>) -> Self {
        Self { reader, writer }
    }

    /// 提取字符串
    pub fn extract_strings(&self, path: &Path) -> Result<Vec<ExtractedString>> {
        let plugin = Plugin::load(path.to_path_buf())?;
        Ok(plugin.extract_strings())
    }

    /// 应用翻译（一站式）
    pub fn apply_translations(
        &self,
        input: &Path,
        output: &Path,
        translations: Vec<ExtractedString>,
        language: Option<&str>,
    ) -> Result<()> {
        let plugin = Plugin::load(input.to_path_buf())?;
        let mut applier = TranslationApplier::for_plugin(
            plugin,
            language,
            self.writer.clone(),  // 假设实现了 Clone
        )?;

        applier.apply(translations)?;
        applier.save(output)?;

        Ok(())
    }
}
```

**新增文件**: `src/services/comparison.rs`

```rust
/// 文件对比服务
pub struct ComparisonService;

impl ComparisonService {
    pub fn compare_files(&self, file1: &Path, file2: &Path) -> Result<FileDiff> {
        // 对比两个 ESP 文件的结构差异
        todo!()
    }
}

pub struct FileDiff {
    pub added_records: Vec<Record>,
    pub removed_records: Vec<Record>,
    pub modified_records: Vec<(Record, Record)>,
}
```

**优势**:
- ✅ CLI 代码简化
- ✅ 易于编写集成测试
- ✅ 依赖注入友好

---

## 四、API 变更对照表

### 4.1 公共 API (Library Users)

| 旧 API | 新 API | 兼容性 | 备注 |
|--------|--------|--------|------|
| `Plugin::new(path, lang)` | `Plugin::load(path)` | ⚠️ Breaking | 旧方法保留并标记 `#[deprecated]` |
| `Plugin::apply_translations(...)` (静态) | `PluginEditor::new(plugin).apply(...).save(...)` | ⚠️ Breaking | 旧方法保留为便捷函数 |
| `plugin.extract_strings()` | `plugin.extract_strings()` | ✅ 兼容 | 无变化 |
| `plugin.is_localized()` | `plugin.is_localized()` | ✅ 兼容 | 无变化 |
| `plugin.get_stats()` | `plugin.get_stats()` | ✅ 兼容 | 无变化 |
| `StringFile::new(path)` | `StringFile::load(path)` | ⚠️ Breaking | 语义更清晰 |
| - | `LocalizedPluginContext::load(path, lang)` | ✅ 新增 | 本地化插件便捷加载 |
| - | `PluginEditor::new(plugin)` | ✅ 新增 | Stateful 编辑器 |
| - | `TranslationService::new(...)` | ✅ 新增 | 服务层 |

---

### 4.2 内部 API (仅影响库内部)

| 模块 | 变更类型 | 影响范围 |
|------|---------|---------|
| `plugin.rs` | 移除 `string_files` 字段 | 大 |
| `plugin.rs` | 移除 `apply_translations_to_string_files` 方法 | 中 |
| `plugin.rs` | `new()` 改为 `load()` | 大 |
| `record.rs` | 新增 `RecordEditor`（可选） | 小 |
| `io/` 模块 | 新增 Reader/Writer trait | 无（新增） |
| `editor/` 模块 | 新增 PluginEditor/Delta | 无（新增） |
| `services/` 模块 | 新增服务层 | 无（新增） |

---

## 五、模块职责重新划分

### 5.1 新的目录结构

```
src/
├── datatypes.rs          [不变] 基础类型定义（RawString, RecordFlags等）
├── record.rs             [修改] 只负责数据结构和解析
├── group.rs              [不变] 组数据结构
├── subrecord.rs          [不变] 子记录数据结构
├── string_types.rs       [不变] ExtractedString 定义
├── utils.rs              [不变] 工具函数
│
├── plugin.rs             [重构] Plugin 核心 - 只负责状态持有
│   - 移除 string_files 字段
│   - 移除 apply_translations 静态方法
│   - load() 替代 new()
│
├── string_file/          [新增] STRING 文件模块
│   ├── mod.rs
│   ├── types.rs          StringFile, StringEntry, StringFileSet
│   ├── parser.rs         解析逻辑（从现有 string_file.rs 迁移）
│   └── writer.rs         写入逻辑
│
├── io/                   [新增] IO 抽象层
│   ├── mod.rs
│   ├── traits.rs         EspReader/Writer trait 定义
│   ├── esp_io.rs         ESP 文件 IO 实现
│   └── string_file_io.rs STRING 文件 IO 实现
│
├── editor/               [新增] 编辑器层
│   ├── mod.rs
│   ├── plugin_editor.rs  PluginEditor (核心)
│   ├── delta.rs          TranslationDelta (变更追踪)
│   ├── applier.rs        TranslationApplier trait 及实现
│   └── validators.rs     翻译验证逻辑（可选）
│
├── services/             [新增] 服务层
│   ├── mod.rs
│   ├── translation.rs    TranslationService
│   ├── extraction.rs     ExtractionService (可选)
│   └── comparison.rs     ComparisonService (可选)
│
├── lib.rs                [修改] 导出新 API
└── main.rs               [修改] 使用新服务层
```

---

### 5.2 模块职责说明

| 模块 | 职责 | 依赖 |
|------|------|------|
| **datatypes** | 基础数据类型、编码处理 | 无 |
| **record/group/subrecord** | 不可变数据结构 | datatypes |
| **plugin** | ESP 文件状态持有、字符串提取 | record, group, datatypes |
| **string_file/** | STRING 文件状态管理 | datatypes |
| **io/** | 文件 IO 抽象与实现 | plugin, string_file |
| **editor/** | 状态修改、变更追踪 | plugin, string_file, io |
| **services/** | 业务逻辑封装 | editor, io |
| **main (CLI)** | 用户交互、工作流编排 | services |

---

## 六、测试策略

### 6.1 单元测试

#### 现有测试（保持）
- ✅ `record.rs`: NULL 填充处理测试
- ✅ `string_file.rs`: STRING 文件解析/写入测试

#### 新增测试

**PluginEditor 测试**:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_and_undo() {
        let plugin = Plugin::load("test.esp").unwrap();
        let mut editor = PluginEditor::new(plugin);

        let trans = ExtractedString { /* ... */ };
        editor.apply_translation(&trans).unwrap();
        assert_eq!(editor.modified_count(), 1);

        editor.undo().unwrap();
        assert_eq!(editor.modified_count(), 0);
    }

    #[test]
    fn test_multiple_modifications() {
        let mut editor = PluginEditor::new(plugin);

        editor.apply_translation(&trans1).unwrap();
        editor.apply_translation(&trans2).unwrap();
        assert_eq!(editor.modified_count(), 2);
    }
}
```

**TranslationDelta 测试**:
```rust
#[test]
fn test_delta_tracking() {
    let mut delta = TranslationDelta::new();

    delta.add_change(RecordChange { /* ... */ });
    assert_eq!(delta.len(), 1);

    delta.undo().unwrap();
    assert_eq!(delta.len(), 0);

    delta.redo().unwrap();
    assert_eq!(delta.len(), 1);
}
```

**TranslationApplier 测试**:
```rust
#[test]
fn test_esp_applier() {
    let plugin = Plugin::load("test.esp").unwrap();
    let writer = Box::new(MockEspWriter);
    let mut applier = EspTranslationApplier {
        editor: PluginEditor::new(plugin),
        writer,
    };

    applier.apply(translations).unwrap();
    assert!(applier.is_modified());
}
```

---

### 6.2 集成测试

**Stateful 工作流测试**:
```rust
#[test]
fn test_stateful_workflow() {
    // 加载插件
    let plugin = Plugin::load("TestFile/GostedDimensionalRift.esp").unwrap();

    // 创建编辑器
    let mut editor = PluginEditor::new(plugin);

    // 应用翻译
    let trans = vec![/* ... */];
    editor.apply_translations(trans).unwrap();

    // 验证状态
    assert!(editor.is_modified());
    assert_eq!(editor.modified_count(), trans.len());

    // 保存
    let writer = DefaultEspWriter;
    editor.save(&writer, Path::new("output.esp")).unwrap();

    // 验证文件
    assert!(Path::new("output.esp").exists());
}
```

**多插件并发测试**:
```rust
#[test]
fn test_multiple_plugins() {
    let mut editor1 = PluginEditor::new(Plugin::load("mod1.esp").unwrap());
    let mut editor2 = PluginEditor::new(Plugin::load("mod2.esp").unwrap());

    editor1.apply_translations(trans1).unwrap();
    editor2.apply_translations(trans2).unwrap();

    // 两个编辑器互不影响
    assert_eq!(editor1.modified_count(), trans1.len());
    assert_eq!(editor2.modified_count(), trans2.len());
}
```

---

### 6.3 兼容性测试

**确保旧 API 仍可用**:
```rust
#[test]
#[allow(deprecated)]
fn test_legacy_api() {
    // 旧 API 应该仍能正常工作
    let result = Plugin::apply_translations(
        PathBuf::from("input.esp"),
        PathBuf::from("output.esp"),
        translations,
        Some("english"),
    );

    assert!(result.is_ok());
}
```

**新旧 API 输出一致性**:
```rust
#[test]
fn test_api_consistency() {
    // 使用旧 API
    let old_result = /* ... */;

    // 使用新 API
    let plugin = Plugin::load("test.esp").unwrap();
    let mut editor = PluginEditor::new(plugin);
    editor.apply_translations(translations).unwrap();
    let new_result = /* ... */;

    // 验证结果一致
    assert_eq!(old_result, new_result);
}
```

---

## 七、迁移路径

### 7.1 阶段性发布计划

#### v0.4.0 (引入新 API)
- ✅ 引入新 API（`Plugin::load`, `PluginEditor` 等）
- ✅ 旧 API 标记 `#[deprecated]` 但保留
- ✅ 文档更新，推荐新用法
- ✅ 示例代码更新

**Changelog**:
```markdown
## [0.4.0] - 2025-XX-XX

### Added
- 新增 `PluginEditor` - Stateful 编辑器
- 新增 `TranslationDelta` - 变更追踪
- 新增 `TranslationApplier` trait - 统一翻译接口
- 新增 `io::traits` 模块 - Reader/Writer 抽象
- 新增 `LocalizedPluginContext` - 本地化插件便捷加载

### Changed
- `Plugin::new()` 重命名为 `Plugin::load()` (旧方法标记 deprecated)
- `Plugin::apply_translations()` 拆分为 `apply()` + `save()` (旧方法标记 deprecated)

### Deprecated
- `Plugin::new()` - 使用 `Plugin::load()` 代替
- `Plugin::apply_translations()` - 使用 `PluginEditor` 代替
```

---

#### v0.5.0 (过渡版本)
- ✅ 保持旧 API，发出 deprecation 警告
- ✅ 所有示例代码迁移到新 API
- ✅ 添加迁移指南文档

**Deprecation 警告示例**:
```rust
#[deprecated(
    since = "0.4.0",
    note = "Use `Plugin::load()` instead. This method will be removed in v1.0.0"
)]
pub fn new(path: PathBuf, language: Option<&str>) -> Result<Self> {
    // 内部仍调用新实现保持功能
    Self::load(path)
}
```

---

#### v1.0.0 (重大版本)
- ⚠️ 移除所有标记为 deprecated 的旧 API
- ✅ 完全基于新架构
- ✅ 性能优化和稳定性改进

**Breaking Changes**:
```markdown
## [1.0.0] - 2025-XX-XX

### Removed (Breaking)
- `Plugin::new()` - 使用 `Plugin::load()`
- `Plugin::apply_translations()` (静态方法) - 使用 `PluginEditor` 或 `TranslationService`
- `StringFile::new()` - 使用 `StringFile::load()`

### Migration Guide
见 MIGRATION.md
```

---

### 7.2 便捷包装函数（保持易用性）

即使在 v1.0.0，也提供高层便捷 API：

```rust
// v1.0.0+ 推荐的简单用法
pub mod convenience {
    /// 一行代码应用翻译（便捷函数）
    pub fn apply_translations(
        input: impl AsRef<Path>,
        output: impl AsRef<Path>,
        translations: Vec<ExtractedString>,
        language: Option<&str>,
    ) -> Result<()> {
        let service = TranslationService::default();
        service.apply_translations(
            input.as_ref(),
            output.as_ref(),
            translations,
            language,
        )
    }
}
```

---

## 八、风险评估

### 8.1 风险矩阵

| 风险项 | 影响 | 概率 | 优先级 | 缓解措施 |
|--------|------|------|--------|----------|
| 破坏现有用户代码 | 高 | 中 | 🔴 高 | 保留旧API，渐进式废弃；提供迁移指南 |
| 性能下降 | 中 | 低 | 🟡 中 | 基准测试，优化热路径 |
| 引入新Bug | 中 | 中 | 🟡 中 | 充分的集成测试，beta 测试 |
| 重构时间过长 | 低 | 中 | 🟢 低 | 分阶段实施，每阶段可独立发布 |
| 文档不及时 | 中 | 高 | 🟡 中 | 每个阶段同步更新文档和示例 |
| 学习曲线陡峭 | 中 | 中 | 🟡 中 | 提供迁移指南、示例代码 |

---

### 8.2 具体缓解措施

#### 向后兼容性
```rust
// 策略 1: 保留旧方法作为包装
#[deprecated(since = "0.4.0")]
pub fn new(path: PathBuf, language: Option<&str>) -> Result<Self> {
    let plugin = Self::load(path)?;
    // 自动加载 STRING 文件（如果需要）
    Ok(plugin)
}

// 策略 2: 提供过渡期 API
#[cfg(feature = "legacy-api")]
pub mod legacy {
    // 旧版完整实现
}
```

#### 性能监控
```rust
// 添加基准测试
#[bench]
fn bench_old_api(b: &mut Bencher) {
    b.iter(|| {
        Plugin::apply_translations(...) // 旧API
    });
}

#[bench]
fn bench_new_api(b: &mut Bencher) {
    b.iter(|| {
        let mut editor = PluginEditor::new(plugin);
        editor.apply_translations(...);
        editor.save(...);
    });
}
```

---

## 九、实施优先级总结

### 第一优先级 (核心功能，必须做) ⭐⭐⭐

1. ✅ **定义 IO trait** (`io/traits.rs`)
   - 工作量: 2小时
   - 风险: 低
   - 依赖: 无

2. ✅ **实现 PluginEditor 和 TranslationDelta**
   - 工作量: 8小时
   - 风险: 中
   - 依赖: 无

3. ✅ **解耦 Plugin 与 StringFileSet**
   - 工作量: 6小时
   - 风险: 高（Breaking Change）
   - 依赖: 1, 2

4. ✅ **重构 apply_translations 为 trait-based**
   - 工作量: 4小时
   - 风险: 中
   - 依赖: 2, 3

**总计**: ~20 小时

---

### 第二优先级 (增强功能，建议做) ⭐⭐

5. 服务层封装 (`services/`)
   - 工作量: 4小时
   - 风险: 低

6. 撤销/重做功能
   - 工作量: 3小时
   - 风险: 低

7. 变更追踪和 diff 功能
   - 工作量: 4小时
   - 风险: 低

**总计**: ~11 小时

---

### 第三优先级 (可选功能，未来考虑) ⭐

8. 插件系统（自定义 Reader/Writer）
9. 异步 IO 支持
10. 增量更新优化

---

## 十、参考实现示例

### 10.1 新的 Stateful 工作流

```rust
use esp_extractor::{Plugin, PluginEditor, io::DefaultEspWriter};

// ========== 加载插件 ==========
let plugin = Plugin::load("MyMod.esp")?;

// ========== 创建编辑器 ==========
let mut editor = PluginEditor::new(plugin);

// ========== 批量应用翻译（内存操作）==========
for translation in translations {
    editor.apply_translation(&translation)?;
}

// ========== 查询状态 ==========
println!("已修改 {} 处", editor.modified_count());

if editor.is_modified() {
    // 查看具体修改
    for change in editor.get_modifications().iter() {
        println!("修改: {:?} {} -> {}",
            change.record_id,
            change.old_value,
            change.new_value
        );
    }
}

// ========== 保存到文件（显式操作）==========
let writer = DefaultEspWriter;
editor.save(&writer, Path::new("MyMod_CN.esp"))?;
```

---

### 10.2 同时维护多个插件

```rust
use esp_extractor::{Plugin, PluginEditor};

// 加载多个插件
let plugin1 = Plugin::load("Mod1.esp")?;
let plugin2 = Plugin::load("Mod2.esp")?;
let plugin3 = Plugin::load("Mod3.esp")?;

// 创建多个编辑器（互不影响）
let mut editor1 = PluginEditor::new(plugin1);
let mut editor2 = PluginEditor::new(plugin2);
let mut editor3 = PluginEditor::new(plugin3);

// 并行修改
editor1.apply_translations(trans_mod1)?;
editor2.apply_translations(trans_mod2)?;
editor3.apply_translations(trans_mod3)?;

// 根据需要选择性保存
let writer = DefaultEspWriter;

if editor1.modified_count() > 0 {
    editor1.save(&writer, Path::new("Mod1_CN.esp"))?;
}

if editor2.modified_count() > 10 {  // 条件保存
    editor2.save(&writer, Path::new("Mod2_CN.esp"))?;
}

// editor3 可以继续修改，暂不保存
```

---

### 10.3 本地化插件处理

```rust
use esp_extractor::{LocalizedPluginContext, editor::StringFileTranslationApplier};

// ========== 加载本地化插件 ==========
let context = LocalizedPluginContext::load(
    "DismemberingFramework.esm".into(),
    "english",
)?;

println!("插件: {}", context.plugin().get_name());
println!("STRING 文件数: {}", context.string_files().files.len());

// ========== 提取字符串 ==========
let strings = context.plugin().extract_strings();
println!("提取了 {} 个字符串", strings.len());

// ========== 应用翻译 ==========
let mut applier = StringFileTranslationApplier {
    plugin: context.plugin().clone(),
    string_files: context.string_files().clone(),
    modified: false,
};

applier.apply(translations)?;

// ========== 保存 STRING 文件 ==========
applier.save(Path::new("output/"))?;
// 输出: output/strings/*.STRINGS, *.DLSTRINGS, *.ILSTRINGS
```

---

### 10.4 使用服务层（CLI 简化）

```rust
use esp_extractor::services::TranslationService;
use esp_extractor::io::{DefaultEspReader, DefaultEspWriter};

// ========== 创建服务 ==========
let service = TranslationService::new(
    Box::new(DefaultEspReader),
    Box::new(DefaultEspWriter),
);

// ========== 提取字符串 ==========
let strings = service.extract_strings(Path::new("input.esp"))?;
println!("提取了 {} 个字符串", strings.len());

// 导出为 JSON
let json = serde_json::to_string_pretty(&strings)?;
std::fs::write("strings.json", json)?;

// ========== 应用翻译（一行代码）==========
service.apply_translations(
    Path::new("input.esp"),
    Path::new("output.esp"),
    translations,
    Some("chinese"),
)?;

println!("翻译应用完成！");
```

---

### 10.5 撤销/重做示例

```rust
use esp_extractor::PluginEditor;

let mut editor = PluginEditor::new(plugin);

// 应用 3 个翻译
editor.apply_translation(&trans1)?;
editor.apply_translation(&trans2)?;
editor.apply_translation(&trans3)?;

assert_eq!(editor.modified_count(), 3);

// 撤销最后一个
editor.undo()?;
assert_eq!(editor.modified_count(), 2);

// 再撤销一个
editor.undo()?;
assert_eq!(editor.modified_count(), 1);

// 重做
editor.redo()?;
assert_eq!(editor.modified_count(), 2);

// 最终保存
let writer = DefaultEspWriter;
editor.save(&writer, Path::new("output.esp"))?;
```

---

### 10.6 自定义 Reader/Writer（插件系统）

```rust
use esp_extractor::io::{EspReader, EspWriter, RawEspData};

// ========== 实现内存 Reader（测试用）==========
struct MemoryEspReader {
    data: Vec<u8>,
}

impl EspReader for MemoryEspReader {
    fn read(&self, _path: &Path) -> Result<RawEspData> {
        Ok(RawEspData {
            bytes: self.data.clone(),
        })
    }
}

// ========== 实现网络 Writer ==========
struct NetworkEspWriter {
    api_endpoint: String,
}

impl EspWriter for NetworkEspWriter {
    fn write(&self, data: &RawEspData, path: &Path) -> Result<()> {
        // 上传到服务器
        let client = reqwest::blocking::Client::new();
        client.post(&self.api_endpoint)
            .body(data.bytes.clone())
            .send()?;
        Ok(())
    }
}

// ========== 使用自定义 IO ==========
let service = TranslationService::new(
    Box::new(MemoryEspReader { data: test_data }),
    Box::new(NetworkEspWriter {
        api_endpoint: "https://api.example.com/upload".to_string()
    }),
);
```

---

## 十一、后续计划

### 下一步行动

1. **确认重构计划** (本文档)
2. **创建 GitHub Issue** - 跟踪每个阶段的进度
3. **实施阶段 1** - IO trait 定义（2小时）
4. **实施阶段 2** - Plugin/StringFile 解耦（6小时）
5. **实施阶段 3** - PluginEditor 实现（8小时）
6. **编写测试** - 单元测试 + 集成测试（4小时）
7. **更新文档** - README, CHANGELOG, 示例（2小时）
8. **发布 v0.4.0** - 引入新 API

---

## 附录 A: 术语表

| 术语 | 说明 |
|------|------|
| **Stateful** | 有状态的，对象内部维护状态 |
| **Stateless** | 无状态的，函数式编程风格 |
| **SoC** | Separation of Concerns（关注点分离） |
| **DIP** | Dependency Inversion Principle（依赖倒置原则） |
| **SRP** | Single Responsibility Principle（单一职责原则） |
| **OCP** | Open-Closed Principle（开闭原则） |
| **ISP** | Interface Segregation Principle（接口隔离原则） |

---

## 附录 B: 参考资料

- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [The Rust Programming Language - Error Handling](https://doc.rust-lang.org/book/ch09-00-error-handling.html)
- [Refactoring Guru - Design Patterns](https://refactoring.guru/design-patterns)

---

**文档维护者**: ESP 字符串解析器开发团队
**最后更新**: 2025-11-13
**联系方式**: GitHub Issues
