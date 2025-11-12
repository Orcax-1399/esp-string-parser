# ESP字符串提取工具 - 架构指南

## 目录

- [1. 项目概览](#1-项目概览)
- [2. 文件格式详解](#2-文件格式详解)
- [3. 核心架构](#3-核心架构)
- [4. 本地化机制](#4-本地化机制)
- [5. 数据流和工作流](#5-数据流和工作流)
- [6. 模块详解](#6-模块详解)
- [7. 关键设计决策](#7-关键设计决策)
- [8. 扩展指南](#8-扩展指南)

---

## 1. 项目概览

### 1.1 项目定位

本项目是一个用于处理Bethesda游戏引擎文件的Rust库和CLI工具，主要用于：
- 提取ESP/ESM/ESL文件中的可翻译字符串
- 应用翻译到游戏文件
- 处理外部字符串表文件（STRINGS/ILSTRINGS/DLSTRINGS）

### 1.2 支持的游戏

- The Elder Scrolls V: Skyrim Special Edition
- 理论上支持所有使用Creation Engine的游戏（Fallout 4、Fallout 76、Starfield等）

### 1.3 关键特性

```
✅ ESP/ESM/ESL文件解析
✅ STRING文件读写（STRINGS/ILSTRINGS/DLSTRINGS）
✅ 压缩记录支持（zlib）
✅ 多编码支持（UTF-8, Windows-1252等）
✅ 智能字符串过滤
✅ 本地化插件支持（StringID映射）
✅ 自动备份机制
```

---

## 2. 文件格式详解

### 2.1 ESP/ESM/ESL文件格式

#### 基本结构

```
┌─────────────────────────────────────┐
│ TES4 Header (Record)                │ ← 文件头，包含插件元信息
│  - Record Header (24 bytes)         │
│  - Subrecords (可变长度)             │
│    - HEDR: 文件版本                  │
│    - CNAM: 作者                      │
│    - MAST: 主文件列表                │
│    - ...                             │
├─────────────────────────────────────┤
│ GRUP 1 (Group)                      │ ← 记录组
│  - Group Header (24 bytes)          │
│  - Records / Subgroups               │
│    ├─ Record 1                       │
│    ├─ Record 2                       │
│    └─ GRUP (nested)                  │
├─────────────────────────────────────┤
│ GRUP 2 (Group)                      │
│  - ...                               │
└─────────────────────────────────────┘
```

#### Record Header（记录头）- 24字节

```
Offset  Size  Description
------  ----  -----------
0x00    4     Record Type (例如: "TES4", "WEAP", "NPC_")
0x04    4     Data Size (数据大小，不包含头部)
0x08    4     Flags (标志位)
0x0C    4     Form ID (唯一标识符)
0x10    2     Timestamp
0x12    2     Version Control Info
0x14    2     Internal Version
0x16    2     Unknown
```

#### 关键标志位

```rust
const MASTER_FILE = 0x00000001;     // ESM标志
const DELETED = 0x00000020;         // 已删除
const LOCALIZED = 0x00000080;       // 本地化（使用外部STRING文件）
const COMPRESSED = 0x00040000;      // 数据已压缩（zlib）
const LIGHT_MASTER = 0x00000200;    // 轻量级主文件（ESL）
```

#### Group Header（组头）- 24字节

```
Offset  Size  Description
------  ----  -----------
0x00    4     "GRUP" (固定标识)
0x04    4     Group Size (包含头部)
0x08    4     Label (组标签/类型)
0x0C    4     Group Type (0=Top, 1=World Children, ...)
0x10    2     Timestamp
0x12    2     Version Control Info
0x14    4     Unknown
0x18    4     Unknown
```

#### Subrecord（子记录）- 6字节头部

```
Offset  Size  Description
------  ----  -----------
0x00    4     Subrecord Type (例如: "FULL", "DESC", "EDID")
0x04    2     Data Size
0x06    N     Data (根据类型不同而不同)
```

---

### 2.2 STRING文件格式

Bethesda游戏使用外部字符串表文件存储本地化文本。有三种类型：

#### 文件类型分类

| 文件类型 | 用途 | 长度前缀 |
|---------|------|---------|
| **STRINGS** | 一般字符串（物品名称、描述等） | ❌ 无 |
| **ILSTRINGS** | 界面字符串 | ✅ 有 (4字节) |
| **DLSTRINGS** | 对话字符串 | ✅ 有 (4字节) |

#### 文件结构

```
┌──────────────────────────────────────┐
│ File Header (8 bytes)                │
│  - String Count (u32)                │ 字符串数量
│  - Data Size (u32)                   │ 数据区总大小
├──────────────────────────────────────┤
│ Directory Table (8 * Count bytes)   │ 目录表
│  - Entry 1:                          │
│    - String ID (u32)                 │
│    - Relative Offset (u32)           │
│  - Entry 2:                          │
│    - String ID (u32)                 │
│    - Relative Offset (u32)           │
│  - ...                               │
├──────────────────────────────────────┤
│ String Data (variable size)          │ 字符串数据区
│                                       │
│ [STRINGS格式]:                        │
│   - String Content (null-terminated) │
│                                       │
│ [DLSTRINGS/ILSTRINGS格式]:            │
│   - Length (u32)                     │ 长度前缀
│   - String Content (UTF-8)           │
│   - Null Terminator (0x00)           │
│                                       │
└──────────────────────────────────────┘
```

#### 关键特点

1. **StringID计算**：从0开始的自增序列，在ESP文件中引用
2. **编码**：UTF-8（官方文件）或Windows-1252（MOD文件）
3. **排序**：目录表按StringID排序
4. **长度前缀差异**：
   - STRINGS：直接存储null终止的字符串
   - DLSTRINGS/ILSTRINGS：4字节长度 + 字符串 + null终止

---

### 2.3 子记录类型 → STRING文件映射

这是**本地化ESP文件**的核心映射关系：

```rust
// 子记录类型决定应该从哪个STRING文件查找
fn determine_string_file_type(record_type: &str, subrecord_type: &str) -> StringFileType {
    // 1. 对话记录 → DLSTRINGS
    if record_type == "DIAL" || record_type == "INFO" {
        return StringFileType::DLSTRINGS;
    }

    // 2. 特定对话子记录 → DLSTRINGS
    if matches!(subrecord_type, "NAM1" | "RNAM") {
        return StringFileType::DLSTRINGS;
    }

    // 3. 界面/列表字符串 → ILSTRINGS
    if matches!(subrecord_type, "ITXT" | "CTDA") {
        return StringFileType::ILSTRINGS;
    }

    // 4. 一般字符串 → STRINGS (默认)
    // FULL, DESC, CNAM, NNAM, SHRT, DNAM, 等等
    StringFileType::STRINGS
}
```

#### 常见子记录映射表

| 子记录类型 | 说明 | STRING文件 | 示例记录类型 |
|-----------|------|-----------|-------------|
| FULL | 完整名称 | STRINGS | WEAP, ARMO, NPC_, BOOK |
| DESC | 描述文本 | STRINGS | WEAP, ARMO, BOOK, PERK |
| CNAM | 内容/条件 | STRINGS | BOOK, QUST |
| NNAM | 名称/注释 | STRINGS | QUST |
| SHRT | 简短名称 | STRINGS | NPC_ |
| NAM1 | 对话响应 | **DLSTRINGS** | INFO |
| RNAM | 对话提示 | **DLSTRINGS** | INFO, ACTI |
| ITXT | 界面文本 | **ILSTRINGS** | MESG |
| CTDA | 条件文本 | **ILSTRINGS** | - |

---

## 3. 核心架构

### 3.1 模块组织

```
esp_extractor/
├── lib.rs                    # 库入口，公共API
├── main.rs                   # CLI工具入口
│
├── datatypes.rs              # 基础数据类型
│   ├── RawString             # 多编码字符串
│   ├── RecordFlags           # 记录标志位
│   └── read/write函数         # 字节序处理
│
├── plugin.rs                 # ESP插件解析器（核心）
│   └── Plugin                # 顶层结构，管理整个文件
│
├── record.rs                 # 记录解析
│   └── Record                # 表示单个ESP记录
│
├── group.rs                  # 组解析
│   ├── Group                 # 表示GRUP块
│   ├── GroupType             # 组类型枚举
│   └── GroupChild            # 递归子结构
│
├── subrecord.rs              # 子记录解析
│   └── Subrecord             # 表示子记录
│
├── string_types.rs           # 提取字符串类型
│   └── ExtractedString       # 提取的字符串结构
│
├── string_file.rs            # STRING文件处理
│   ├── StringFile            # 单个STRING文件
│   ├── StringFileSet         # 多个STRING文件集合
│   ├── StringEntry           # 字符串条目
│   └── StringFileType        # 文件类型枚举
│
├── utils.rs                  # 工具函数
│   ├── is_valid_string       # 字符串验证
│   ├── create_backup         # 备份创建
│   └── EspError              # 错误类型
│
└── debug.rs                  # 调试工具（仅debug模式）
    └── EspDebugger           # 文件对比和分析
```

### 3.2 核心数据结构

#### Plugin（插件）

```rust
pub struct Plugin {
    pub path: PathBuf,                              // 文件路径
    pub header: Record,                             // TES4头部
    pub groups: Vec<Group>,                         // 所有GRUP组
    pub masters: Vec<String>,                       // 主文件列表
    pub string_records: HashMap<String, Vec<String>>, // 字符串记录定义
    // TODO: pub string_files: Option<StringFileSet>, // 外部STRING文件
}
```

#### Record（记录）

```rust
pub struct Record {
    pub record_type: String,        // 记录类型（4字符）
    pub data_size: u32,             // 数据大小
    pub flags: u32,                 // 标志位
    pub form_id: u32,               // FormID
    pub timestamp: u16,
    pub version_control_info: u16,
    pub internal_version: u16,
    pub unknown: u16,
    pub subrecords: Vec<Subrecord>, // 子记录列表
    pub original_data: Vec<u8>,     // 原始数据（压缩时）
    pub modified: bool,             // 是否已修改
}
```

#### Group（组）

```rust
pub struct Group {
    pub group_type: GroupType,      // 组类型
    pub label: u32,                 // 标签
    pub children: Vec<GroupChild>,  // 子元素（递归）
}

pub enum GroupChild {
    Record(Record),                 // 记录
    Group(Box<Group>),              // 子组（递归）
}
```

#### StringFile（STRING文件）

```rust
pub struct StringFile {
    pub path: PathBuf,
    pub file_type: StringFileType,               // STRINGS/ILSTRINGS/DLSTRINGS
    pub plugin_name: String,                     // 插件名
    pub language: String,                        // 语言（english/chinese等）
    pub entries: HashMap<u32, StringEntry>,      // StringID -> 字符串
}

pub struct StringEntry {
    pub id: u32,                    // StringID
    pub content: String,            // 字符串内容（UTF-8）
    pub raw_data: Vec<u8>,          // 原始字节数据
    pub length: Option<u32>,        // 长度（DLSTRINGS/ILSTRINGS）
    // 元数据（用于调试）
    pub directory_address: u64,
    pub relative_offset: u32,
    pub absolute_offset: u64,
}
```

---

## 4. 本地化机制

### 4.1 本地化标志位

```rust
const LOCALIZED = 0x00000080;  // 十进制128

// 检测本地化插件
fn is_localized(header_flags: u32) -> bool {
    header_flags & 0x00000080 != 0
}
```

当ESP文件的TES4头部设置了`LOCALIZED`标志时，所有字符串子记录存储的是**4字节StringID**，而非实际字符串。

### 4.2 本地化工作流

#### 提取字符串流程（本地化ESP）

```
1. 读取ESP文件
   ├─ 检测LOCALIZED标志
   └─ 如果本地化：需要加载STRING文件

2. 对于每个字符串子记录（FULL, DESC等）
   ├─ 读取4字节StringID
   ├─ 确定STRING文件类型（STRINGS/ILSTRINGS/DLSTRINGS）
   └─ 从对应STRING文件查找实际文本

3. 返回ExtractedString
   ├─ string_id: 原始StringID
   ├─ string_file_type: 所属STRING文件类型
   └─ original_text: 从STRING文件获取的实际文本
```

#### 应用翻译流程（本地化ESP）

```
1. 读取翻译的ExtractedString列表

2. 构建翻译映射
   ├─ (StringFileType, StringID) -> 翻译文本
   └─ 例如: (STRINGS, 123) -> "钢剑"

3. 更新STRING文件
   ├─ StringFile::update_string(id, new_text)
   └─ 批量更新所有翻译

4. 写回STRING文件
   ├─ 重建二进制数据
   ├─ 创建备份（.bak）
   └─ 写入文件
```

### 4.3 文件命名约定

```
插件名_语言.扩展名

示例：
- Skyrim_english.STRINGS
- Skyrim_chinese.STRINGS
- MyMod_english.DLSTRINGS
- MyMod_japanese.ILSTRINGS
```

### 4.4 当前实现状态

| 功能 | 状态 | 备注 |
|------|------|------|
| 检测本地化标志 | ✅ 已实现 | plugin.rs:168 |
| 读取StringID | ✅ 已实现 | plugin.rs:171 |
| STRING文件读取 | ✅ 已实现 | string_file.rs |
| STRING文件写入 | ✅ 已实现 | string_file.rs:355-418 |
| Plugin集成STRING | ❌ **待实现** | 需要添加string_files字段 |
| StringID查找映射 | ❌ **待实现** | 需要实现determine_string_file_type |
| ExtractedString添加字段 | ❌ **待实现** | 需要string_id和string_file_type |

---

## 5. 数据流和工作流

### 5.1 字符串提取流程

#### 普通ESP（非本地化）

```
文件 (MyMod.esp)
    ↓
Plugin::new(path)
    ↓
解析TES4头部 → 检测flags (LOCALIZED=0)
    ↓
解析所有GRUP → 递归处理子组和记录
    ↓
对于每个Record
    ↓
提取符合类型的Subrecord（FULL, DESC等）
    ↓
RawString::parse_zstring() → 直接读取字符串
    ↓
ExtractedString {
    editor_id,
    form_id,
    original_text,  ← 直接从ESP读取
    record_type,
    subrecord_type,
}
    ↓
JSON输出
```

#### 本地化ESP（带STRING文件）

```
文件 (MyMod.esp) + STRING文件集
    ↓
Plugin::new(path)
    ↓
解析TES4头部 → 检测flags (LOCALIZED=1)
    ↓
Plugin::load_string_files() → 查找并加载STRING文件
    ├─ MyMod_english.STRINGS
    ├─ MyMod_english.ILSTRINGS
    └─ MyMod_english.DLSTRINGS
    ↓
解析所有GRUP → 递归处理
    ↓
对于每个字符串Subrecord
    ↓
读取4字节StringID
    ↓
determine_string_file_type(record_type, subrecord_type)
    ↓
从对应STRING文件查找
    ├─ StringFileSet::get_string_by_type(STRINGS, id)
    ├─ StringFileSet::get_string_by_type(ILSTRINGS, id)
    └─ StringFileSet::get_string_by_type(DLSTRINGS, id)
    ↓
ExtractedString {
    editor_id,
    form_id,
    original_text,      ← 从STRING文件获取
    record_type,
    subrecord_type,
    string_id,          ← 保存StringID
    string_file_type,   ← 保存文件类型
}
    ↓
JSON输出
```

### 5.2 翻译应用流程

#### 普通ESP

```
翻译JSON → Vec<ExtractedString>
    ↓
Plugin::new(input_esp)
    ↓
创建翻译映射: HashMap<UniqueKey, ExtractedString>
    key = editor_id + form_id + record_type + subrecord_type
    ↓
Plugin::apply_translation_map()
    ↓
递归遍历所有Record和Subrecord
    ↓
匹配UniqueKey → 替换Subrecord.data
    ↓
标记Record为modified
    ↓
Plugin::write_to_file(output_esp)
    ↓
重建文件（保持压缩状态）
    ↓
输出文件
```

#### 本地化ESP

```
翻译JSON → Vec<ExtractedString> (包含string_id)
    ↓
Plugin::new(input_esp)
    ↓
Plugin::load_string_files()
    ↓
构建翻译映射: HashMap<(StringFileType, StringID), String>
    ↓
StringFileSet::apply_translations(translations)
    ↓
对每个(file_type, string_id, text)
    └─ StringFile::update_string(id, text)
        └─ 更新StringEntry.content
    ↓
StringFileSet::write_all(output_dir)
    ↓
对每个StringFile
    ├─ 创建备份 (.bak)
    ├─ rebuild() → 重建二进制数据
    └─ write_to_file()
    ↓
输出STRING文件
```

### 5.3 STRING文件读写流程

#### 读取流程

```
StringFile::new(path)
    ↓
解析文件名 → (plugin_name, language, file_type)
    ↓
读取文件数据
    ↓
解析文件头（8字节）
    ├─ string_count (u32)
    └─ data_size (u32)
    ↓
读取目录表（8 * count 字节）
    └─ 每个条目：StringID + Relative Offset
    ↓
对每个目录条目
    ├─ 计算绝对偏移：string_data_start + relative_offset
    ├─ 读取字符串数据
    │   ├─ [DLSTRINGS/ILSTRINGS] 读取4字节长度
    │   ├─ 读取字符串内容（UTF-8）
    │   └─ 读取null终止符
    └─ 创建StringEntry
    ↓
HashMap<StringID, StringEntry>
```

#### 写入流程

```
StringFile::write_to_file(path)
    ↓
rebuild() → 重建二进制数据
    ↓
1. 写入文件头
   ├─ string_count (u32)
   └─ data_size (u32)
    ↓
2. 准备排序的StringID列表
   └─ ids.sort()
    ↓
3. 计算每个字符串的偏移量
   └─ offset += entry.get_total_size()
    ↓
4. 写入目录表
   └─ 对每个ID：write_u32(id) + write_u32(offset)
    ↓
5. 写入字符串数据
   ├─ [DLSTRINGS/ILSTRINGS] write_u32(length)
   ├─ write(content.as_bytes())
   └─ write(0x00)  // null终止符
    ↓
Vec<u8> (完整二进制数据)
    ↓
创建备份（如果文件存在）
    ↓
fs::write(path, data)
```

---

## 6. 模块详解

### 6.1 plugin.rs - 核心插件解析器

**职责**：管理整个ESP文件的解析和操作

**关键方法**：

```rust
impl Plugin {
    // === 构造与解析 ===
    pub fn new(path: PathBuf) -> Result<Self>
    fn validate_esp_file(header: &Record) -> Result<()>
    fn parse_groups(cursor, data) -> Result<Vec<Group>>
    fn extract_masters(header: &Record) -> Vec<String>
    fn load_string_records() -> Result<HashMap>

    // === 字符串提取 ===
    pub fn extract_strings(&self) -> Vec<ExtractedString>
    fn extract_group_strings(&self, group: &Group) -> Vec<ExtractedString>
    fn extract_record_strings(&self, record: &Record) -> Vec<ExtractedString>
    fn extract_string_from_subrecord(...) -> Option<ExtractedString>

    // === 翻译应用 ===
    pub fn apply_translations(input, output, translations) -> Result<()>
    fn create_translation_map(Vec<ExtractedString>) -> HashMap
    fn apply_translation_map(&mut self, map: &HashMap) -> Result<()>
    fn apply_translation_to_group(&mut self, group, map, masters) -> Result<()>
    fn apply_translation_to_record(&mut self, record, map, masters) -> Result<bool>

    // === 文件写入 ===
    pub fn write_to_file(&self, path: PathBuf) -> Result<()>

    // === 信息获取 ===
    pub fn get_name(&self) -> &str
    pub fn get_type(&self) -> &str
    pub fn is_master(&self) -> bool
    pub fn is_localized(&self) -> bool
    pub fn get_stats(&self) -> PluginStats

    // === 工具方法 ===
    fn format_form_id(&self, form_id: u32) -> String
    fn count_group_records(&self, group: &Group) -> usize
    fn count_subgroups(&self, group: &Group) -> usize
}
```

**关键设计**：
- 递归处理GRUP嵌套结构
- 惰性加载：只在需要时解压缩数据
- FormID处理：自动识别主文件索引

### 6.2 record.rs - 记录解析

**职责**：处理单个ESP记录的解析和重建

**关键方法**：

```rust
impl Record {
    pub fn parse(cursor: &mut Cursor<&[u8]>) -> Result<Self>

    // 解压缩记录数据
    fn decompress_data(data: &[u8]) -> Result<Vec<u8>>

    // 子记录解析
    fn parse_subrecords(data: &[u8]) -> Result<Vec<Subrecord>>

    // 获取EDID（编辑器ID）
    pub fn get_editor_id(&self) -> Option<String>

    // 重建记录
    pub fn rebuild(&self) -> Result<Vec<u8>>
    fn rebuild_subrecords(&self) -> Vec<u8>
}
```

**压缩处理**：
```rust
if flags & 0x00040000 != 0 {
    // 压缩记录
    // 1. 保存原始压缩数据
    // 2. 解压缩（zlib）
    // 3. 解析子记录
    // 4. 重建时重新压缩
}
```

### 6.3 group.rs - 组解析

**职责**：处理GRUP块的递归结构

**关键结构**：

```rust
pub enum GroupType {
    Normal,      // 顶级组
    World,       // 世界子组
    Cell,        // 单元格子组
}

pub enum GroupChild {
    Record(Record),        // 叶子节点：记录
    Group(Box<Group>),     // 递归节点：子组
}

impl Group {
    pub fn parse(cursor: &mut Cursor<&[u8]>) -> Result<Self>
    fn parse_children(...) -> Result<Vec<GroupChild>>
    pub fn rebuild(&self) -> Result<Vec<u8>>
}
```

**递归处理**：
```rust
fn parse_children(...) -> Result<Vec<GroupChild>> {
    while position < group_end {
        let type_signature = read_type();

        match type_signature {
            "GRUP" => {
                // 递归解析子组
                let subgroup = Group::parse(cursor)?;
                children.push(GroupChild::Group(Box::new(subgroup)));
            }
            _ => {
                // 解析记录
                let record = Record::parse(cursor)?;
                children.push(GroupChild::Record(record));
            }
        }
    }
}
```

### 6.4 string_file.rs - STRING文件处理

**职责**：处理外部字符串表文件

**关键方法**：

```rust
impl StringFile {
    // === 读取 ===
    pub fn new(path: PathBuf) -> Result<Self>
    fn parse_filename(path: &Path) -> Result<(String, String, StringFileType)>
    fn parse_file(path, file_type) -> Result<HashMap<u32, StringEntry>>
    fn read_string_data(...) -> Result<(String, Vec<u8>, Option<u32>)>

    // === 查询 ===
    pub fn get_string(&self, id: u32) -> Option<&StringEntry>
    pub fn get_string_ids(&self) -> Vec<u32>
    pub fn find_strings_containing(&self, text: &str) -> Vec<&StringEntry>

    // === 修改 ===
    pub fn update_string(&mut self, id: u32, content: String) -> Result<()>
    pub fn update_strings(&mut self, updates: HashMap<u32, String>) -> Result<()>
    pub fn add_string(&mut self, id: u32, content: String) -> Result<()>
    pub fn remove_string(&mut self, id: u32) -> Option<StringEntry>

    // === 写入 ===
    pub fn rebuild(&self) -> Result<Vec<u8>>
    pub fn write_to_file(&self, path: PathBuf) -> Result<()>
}

impl StringFileSet {
    pub fn new(plugin_name: String, language: String) -> Self
    pub fn load_from_directory(dir, plugin, language) -> Result<Self>

    pub fn get_string(&self, id: u32) -> Option<&StringEntry>
    pub fn get_string_by_type(&self, type: StringFileType, id: u32) -> Option<&StringEntry>

    pub fn update_string(&mut self, type, id, content) -> Result<()>
    pub fn apply_translations(&mut self, translations) -> Result<()>

    pub fn write_all(&self, directory: &Path) -> Result<()>
    pub fn write_file(&self, type, directory) -> Result<()>
}
```

**关键点**：
- **自动检测文件类型**：通过扩展名识别
- **文件名解析**：提取插件名和语言
- **大小计算一致性**：使用`content.as_bytes().len()`而不是`raw_data.len()`

### 6.5 datatypes.rs - 基础数据类型

**职责**：提供底层数据结构和编码处理

**RawString - 多编码字符串**：

```rust
pub struct RawString {
    pub content: String,
    pub encoding: String,
}

impl RawString {
    pub fn decode(data: &[u8]) -> Self
    pub fn parse_zstring(data: &[u8]) -> Self       // null终止
    pub fn parse_bstring(cursor) -> Result<Self>    // 长度前缀
}
```

**支持的编码**：
- UTF-8（优先）
- Windows-1252（西欧）
- Windows-1250（中欧）
- Windows-1251（西里尔文）

**RecordFlags - 位标志**：

```rust
bitflags! {
    pub struct RecordFlags: u32 {
        const MASTER_FILE = 0x00000001;
        const LOCALIZED = 0x00000080;
        const COMPRESSED = 0x00040000;
        const LIGHT_MASTER = 0x00000200;
        // ... 更多标志位
    }
}
```

### 6.6 utils.rs - 工具函数

**字符串验证**：

```rust
pub fn is_valid_string(text: &str) -> bool {
    // 1. 检查黑名单
    if blacklist.contains(text) { return false; }

    // 2. 检查白名单
    if is_whitelisted(text) { return true; }

    // 3. 过滤变量名
    if is_variable_name(text) { return false; }

    // 4. 检查字符有效性
    text.chars().all(|c| !c.is_control() || c.is_whitespace())
}

fn is_variable_name(text: &str) -> bool {
    is_camel_case(text) || is_snake_case(text)
}
```

**备份创建**：

```rust
pub fn create_backup(file_path: &Path) -> Result<PathBuf> {
    let timestamp = chrono::Local::now().format("%Y-%m-%d-%H-%M-%S");
    let backup_path = file_path.with_extension(format!("{}.bak", timestamp));
    fs::copy(file_path, &backup_path)?;
    Ok(backup_path)
}
```

---

## 7. 关键设计决策

### 7.1 为什么使用HashMap而不是Vec？

**StringFile.entries: HashMap<u32, StringEntry>**

**原因**：
1. ✅ O(1) 查找复杂度（StringID查找）
2. ✅ 支持稀疏ID（ID可能不连续）
3. ✅ 方便更新和删除
4. ⚠️ 写入时需要排序（但只需一次）

### 7.2 为什么保留原始压缩数据？

**Record.original_data: Vec<u8>**

**原因**：
1. ✅ 验证解压缩正确性
2. ✅ 未修改时直接写回原数据
3. ✅ 避免重新压缩导致的数据差异
4. ⚠️ 增加内存占用（但仅压缩记录）

### 7.3 为什么ExtractedString使用唯一键？

**UniqueKey = editor_id + form_id + record_type + subrecord_type**

**原因**：
1. ✅ 避免ID冲突（不同对象可能共享FormID）
2. ✅ 支持部分翻译（只翻译特定字段）
3. ✅ 精确匹配（不会误替换）

示例冲突：
```
WEAP [0x12345] FULL = "Iron Sword"
WEAP [0x12345] DESC = "A simple iron sword"
```

如果只用FormID，会混淆FULL和DESC。

### 7.4 为什么STRING文件需要长度前缀？

**DLSTRINGS/ILSTRINGS有长度前缀，STRINGS没有**

**原因**：
- STRINGS: 简单字符串，null终止足够
- DLSTRINGS: 对话可能包含多行/特殊字符，长度前缀更可靠
- ILSTRINGS: 界面字符串需要快速跳过（性能优化）

### 7.5 为什么使用Copy trait for StringFileType？

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StringFileType { ... }
```

**原因**：
1. ✅ 避免所有权问题（可以直接解引用）
2. ✅ 小类型（仅3个变体），Copy无性能损失
3. ✅ 简化API（不需要.clone()）

### 7.6 为什么get_total_size使用content而不是raw_data？

```rust
// 错误的实现
let content_size = self.raw_data.len() as u32;

// 正确的实现
let content_size = self.content.as_bytes().len() as u32;
```

**原因**：
- `raw_data` 可能包含额外数据（长度前缀、元数据）
- `content` 是实际字符串内容，与rebuild()写入的数据一致
- 保证大小计算与实际写入的数据匹配

---

## 8. 扩展指南

### 8.1 添加对新游戏的支持

1. **验证文件格式兼容性**：
   ```rust
   // 检查Record Header是否为24字节
   // 检查Group Header是否为24字节
   // 检查是否使用相同的压缩算法（zlib）
   ```

2. **添加新的记录类型**：
   ```json
   // data/string_records.json
   {
     "NEWTYPE": ["FULL", "DESC"],
     ...
   }
   ```

3. **测试关键功能**：
   - 文件解析
   - 压缩记录处理
   - 字符串提取
   - 翻译应用

### 8.2 实现完整的本地化支持

**待实现功能清单**：

```rust
// 1. Plugin添加string_files字段
pub struct Plugin {
    // ... 现有字段
    pub string_files: Option<StringFileSet>,  // 🆕
}

// 2. 自动加载STRING文件
impl Plugin {
    pub fn new_with_strings(
        path: PathBuf,
        string_dir: Option<PathBuf>
    ) -> Result<Self> {
        let mut plugin = Self::new(path)?;

        if plugin.is_localized() {
            let string_dir = string_dir.unwrap_or_else(|| {
                plugin.path.parent().unwrap().to_path_buf()
            });

            plugin.string_files = Some(
                StringFileSet::load_from_directory(
                    &string_dir,
                    &plugin.get_name_without_ext(),
                    "english" // TODO: 自动检测或参数化
                )?
            );
        }

        Ok(plugin)
    }
}

// 3. 实现StringID查找
impl Plugin {
    fn determine_string_file_type(
        record_type: &str,
        subrecord_type: &str
    ) -> StringFileType {
        // 对话 → DLSTRINGS
        if record_type == "DIAL" || record_type == "INFO" {
            return StringFileType::DLSTRINGS;
        }

        if matches!(subrecord_type, "NAM1" | "RNAM") {
            return StringFileType::DLSTRINGS;
        }

        // 界面 → ILSTRINGS
        if matches!(subrecord_type, "ITXT" | "CTDA") {
            return StringFileType::ILSTRINGS;
        }

        // 默认 → STRINGS
        StringFileType::STRINGS
    }

    fn extract_string_from_subrecord(...) -> Option<ExtractedString> {
        let raw_string = if self.is_localized() {
            let mut cursor = Cursor::new(&subrecord.data[..]);
            let string_id = read_u32(&mut cursor)?;

            // 🆕 查找实际文本
            let file_type = Self::determine_string_file_type(
                record_type,
                &subrecord.record_type
            );

            if let Some(ref string_files) = self.string_files {
                if let Some(entry) = string_files.get_string_by_type(file_type, string_id) {
                    RawString {
                        content: entry.content.clone(),
                        encoding: "utf-8".to_string(),
                    }
                } else {
                    // 未找到，返回占位符
                    RawString {
                        content: format!("StringID_{}_{:?}", string_id, file_type),
                        encoding: "ascii".to_string(),
                    }
                }
            } else {
                // 没有加载STRING文件
                RawString {
                    content: format!("StringID_{}", string_id),
                    encoding: "ascii".to_string(),
                }
            }
        } else {
            // 普通插件
            RawString::parse_zstring(&subrecord.data)
        };

        // ... 创建ExtractedString
    }
}

// 4. ExtractedString添加字段
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedString {
    pub editor_id: Option<String>,
    pub form_id: String,
    pub original_text: String,
    pub record_type: String,
    pub subrecord_type: String,

    // 🆕 本地化相关字段
    pub string_id: Option<u32>,
    pub string_file_type: Option<StringFileType>,
}

// 5. 应用翻译到STRING文件
impl Plugin {
    pub fn apply_translations_to_string_files(
        &mut self,
        translations: Vec<ExtractedString>,
        output_dir: &Path
    ) -> Result<()> {
        let string_files = self.string_files.as_mut()
            .ok_or("本地化插件但未加载STRING文件")?;

        // 构建翻译映射
        let mut updates: HashMap<(StringFileType, u32), String> = HashMap::new();

        for trans in translations {
            if let (Some(string_id), Some(file_type)) =
                (trans.string_id, trans.string_file_type) {
                updates.insert((file_type, string_id), trans.original_text);
            }
        }

        // 应用翻译
        string_files.apply_translations(&updates)?;

        // 写回STRING文件
        string_files.write_all(output_dir)?;

        Ok(())
    }
}
```

### 8.3 性能优化建议

1. **大文件处理**：
   ```rust
   // 使用内存映射
   use memmap2::Mmap;

   let file = File::open(path)?;
   let mmap = unsafe { Mmap::map(&file)? };
   let mut cursor = Cursor::new(&mmap[..]);
   ```

2. **并行解析**：
   ```rust
   // 使用rayon并行处理组
   use rayon::prelude::*;

   let strings: Vec<ExtractedString> = groups
       .par_iter()
       .flat_map(|g| extract_group_strings(g))
       .collect();
   ```

3. **惰性加载**：
   ```rust
   // 只在需要时解压缩
   pub struct Record {
       data: LazyData,  // 延迟加载
   }

   enum LazyData {
       Compressed(Vec<u8>),
       Decompressed(Vec<u8>),
   }
   ```

### 8.4 添加新的子记录类型

1. **更新string_records.json**：
   ```json
   {
     "NEWREC": ["FULL", "DESC", "NEWFIELD"]
   }
   ```

2. **测试提取**：
   ```rust
   #[test]
   fn test_new_record_type() {
       let plugin = Plugin::new("test.esp")?;
       let strings = plugin.extract_strings();

       let new_strings: Vec<_> = strings.iter()
           .filter(|s| s.record_type == "NEWREC")
           .collect();

       assert!(!new_strings.is_empty());
   }
   ```

### 8.5 CLI工具增强

**建议添加的命令**：

```bash
# 查看插件信息
esp_extractor info MyMod.esp

# 验证文件完整性
esp_extractor validate MyMod.esp

# 对比两个插件
esp_extractor diff Original.esp Modified.esp

# 合并多个翻译文件
esp_extractor merge base.json patch1.json patch2.json -o merged.json

# 导出为其他格式
esp_extractor export MyMod.esp -f csv -o output.csv
esp_extractor export MyMod.esp -f po -o output.po  # GNU gettext format
```

---

## 附录

### A. 参考资料

- [UESP - Mod File Format](https://en.uesp.net/wiki/Skyrim_Mod:Mod_File_Format)
- [UESP - String Table File Format](https://en.uesp.net/wiki/Skyrim_Mod:String_Table_File_Format)
- [Creation Kit Documentation](https://www.creationkit.com/)
- [xEdit](https://github.com/TES5Edit/TES5Edit) - ESP文件编辑器（参考实现）

### B. 常见FormID前缀

| FormID范围 | 来源 | 说明 |
|-----------|------|------|
| 0x000000-0x000FFF | Skyrim.esm | 原版游戏 |
| 0x01000000- | 第一个主文件 | 根据加载顺序 |
| 0x02000000- | 第二个主文件 | |
| 0xFE000000- | ESL文件 | 轻量级插件 |

### C. 文件扩展名约定

| 扩展名 | 类型 | 说明 |
|-------|------|------|
| .esp | Plugin | 普通插件 |
| .esm | Master | 主文件（依赖项） |
| .esl | Light | 轻量级插件（不占用加载顺序） |
| .STRINGS | Strings | 一般字符串表 |
| .ILSTRINGS | IL Strings | 界面字符串表 |
| .DLSTRINGS | DL Strings | 对话字符串表 |

### D. 调试技巧

**查看文件结构**：
```bash
cargo run --features cli -- -i MyMod.esp --stats

# 对比两个文件
cargo run --features cli -- -i Original.esp --compare-files Modified.esp
```

**启用调试输出**：
```rust
#[cfg(debug_assertions)]
println!("调试信息: {}", value);
```

**使用hexdump查看二进制**：
```bash
hexdump -C MyMod.esp | head -100
```

---

## 版本历史

| 版本 | 日期 | 更新内容 |
|------|------|---------|
| 1.0 | 2025-11-12 | 初始版本，完整架构文档 |

---

**文档维护**：请在重大架构变更时更新此文档。

**贡献者**：如果你修改了核心架构，请在此文档中记录你的设计决策和原因。
