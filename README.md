# ESP 字符串提取工具 (esp_extractor)

[![Crates.io](https://img.shields.io/crates/v/esp_extractor.svg)](https://crates.io/crates/esp_extractor)
[![Documentation](https://docs.rs/esp_extractor/badge.svg)](https://docs.rs/esp_extractor)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](https://opensource.org/licenses/MIT)

一个用于处理 Bethesda 游戏引擎（ESP/ESM/ESL）文件和字符串文件的 **高性能 Rust 库**。支持 Skyrim、Fallout 等游戏的 Mod 翻译工作流。

## 核心特性

- 模块化架构 - v0.7.0 代码重构，职责清晰的模块划分
- 性能优化 - v0.5.0 在本地测试中实现显著性能提升（详见性能测试章节）
- 字符串提取 - v0.5.1 修复 STRING 路由规则，提升提取完整性
- BSA 支持 - v0.6.0 支持从 BSA 归档自动提取 STRING 文件
- 分层架构 - IO 抽象层 + 编辑器层
- 自动加载 - 自动检测本地化插件，按需加载 STRING 文件
- 有状态编辑 - 支持批量修改、延迟保存
- 变更追踪 - 记录所有修改操作
- 可测试性 - 支持依赖注入和 mock 测试

## 📦 安装

### 作为库使用（推荐）

```toml
[dependencies]
esp_extractor = "0.7.0"
```

### 作为命令行工具

```bash
cargo install esp_extractor --features cli
```

## 🚀 快速开始

### 命令行工具

```bash
# 1. 提取字符串到 JSON
esp_extractor -i "MyMod.esp" -o "strings.json"

# 2. 编辑 JSON 文件中的 text 字段为翻译文本

# 3. 应用翻译
esp_extractor -i "MyMod.esp" --apply-file "strings_cn.json" -o "MyMod_CN.esp"
```

### 库 API

```rust
use esp_extractor::LoadedPlugin;

// 自动加载插件（包括 STRING 文件）
let loaded = LoadedPlugin::load_auto("MyMod.esp".into(), Some("english"))?;

// 提取字符串
let strings = loaded.extract_strings();
println!("提取到 {} 个字符串", strings.len());

// 保存到 JSON
let json = serde_json::to_string_pretty(&strings)?;
std::fs::write("strings.json", json)?;
```

## 🎯 主要功能

### ESP/ESM/ESL 文件处理
- 字符串提取和翻译应用
- 文件结构分析和调试
- 压缩记录支持（zlib）
- Light Plugin (ESL) 支持

### 字符串文件解析
- 支持 `.STRINGS`、`.ILSTRINGS`、`.DLSTRINGS` 文件
- 自动检测文件类型和编码
- 转换为 JSON 格式便于处理

### 高级特性
- **10 种 GroupType 支持**：完整的游戏数据结构解析
- **XXXX 超大子记录**：正确处理 > 65535 字节的子记录（如 NAVM）
- **特殊记录索引**：INFO/QUST/PERK 记录的索引跟踪
- **多编码支持**：自动检测 UTF-8、GB18030、Windows-1252 等

## 性能测试

### 测试环境
- 测试文件：Skyrim.esm (250 MB)
- 硬件：AMD Ryzen 7 5800H, NVMe SSD
- 构建：Release 模式 (`cargo build --release`)
- 测试脚本：`examples/performance_comparison.rs`

### 优化历程

| 版本 | 技术方案 | 加载时间 | 相对提升 |
|------|---------|---------|----------|
| v0.4.0 | `fs::read` + 克隆 | 240秒 | 基线 |
| v0.5.0 阶段1 | `memmap2` + `Cow` | 120秒 | 2x |
| v0.5.0 | `rayon` 并行化 | 9秒 | 26x |

### 优化技术

1. 内存映射文件（memmap2）- 零拷贝文件访问，按需分页加载
2. Copy-on-Write（Cow）- 减少内存拷贝
3. 并行处理（rayon）- Group 并行解析和字符串提取并行化

### 测试数据（Skyrim.esm）

- STRING 文件：67,414 条（3 种文件类型）
- 提取字符串：20,437 条（含 2,835 条特殊记录）

## 📋 使用示例

### 字符串提取

```bash
# 基础提取
esp_extractor -i "MyMod.esp" -o "strings.json"

# 包含本地化字符串（显示为 StringID）
esp_extractor -i "MyMod.esp" --include-localized -o "all_strings.json"

# 显示统计信息
esp_extractor -i "Skyrim.esm" --stats
```

### 字符串文件解析

```bash
# 自动检测文件类型
esp_extractor -i "Dragonborn_english.ILSTRINGS" -o "dragonborn.json"

# 查看统计信息
esp_extractor -i "Skyrim_english.STRINGS" --stats
```

### 翻译应用

```bash
# 从 JSON 文件应用翻译
esp_extractor -i "MyMod.esp" --apply-file "translations.json" -o "MyMod_CN.esp"

# 从 JSON 字符串应用部分翻译（适合少量修改）
esp_extractor -i "MyMod.esp" --apply-jsonstr '[{"editor_id":"IronSword","form_id":"00012BB7|Skyrim.esm","text":"铁剑","record_type":"WEAP","subrecord_type":"FULL","index":0}]' -o "MyMod_CN.esp"

# 从标准输入读取翻译（适合脚本处理）
cat translations.json | esp_extractor -i "MyMod.esp" --apply-partial-stdin -o "MyMod_CN.esp"
```

### ESL 插件处理

```bash
# 将 ESP 转换为 ESL（FormID 重编号）
esp_extractor -i "MyMod.esp" --eslify -o "MyMod.esl"

# 注意：最多支持 2048 条新记录
```

## 📄 输出格式

JSON 格式的字符串数组：

```json
{
  "editor_id": "IronSword",
  "form_id": "00012BB7|Skyrim.esm",
  "text": "Iron Sword",
  "record_type": "WEAP",
  "subrecord_type": "FULL",
  "index": 0
}
```

### 字段说明
- `editor_id`: 编辑器 ID
- `form_id`: FormID|主文件名
- `text`: 文本内容（提取时为原文，应用翻译时为译文）
- `record_type`: 记录类型（如 WEAP、NPC_、BOOK）
- `subrecord_type`: 子记录类型（如 FULL、DESC）
- `index`: 同类型子记录索引（从 0 开始）

### 匹配机制
应用翻译时使用 **四重匹配** 确保精确性：
- `editor_id` + `form_id` + `record_type` + `subrecord_type` + `index`（可选）

## 🎮 支持的游戏

- The Elder Scrolls V: Skyrim Special Edition
- The Elder Scrolls IV: Oblivion
- Fallout 3 / Fallout: New Vegas / Fallout 4

## ⚙️ 核心命令行选项

### 通用选项
- `-i, --input <FILE>`: 输入文件路径（必需）
- `-o, --output <FILE>`: 输出文件路径（可选）
- `--stats`: 显示文件统计信息
- `--quiet`: 静默模式

### 提取模式
- `--include-localized`: 包含本地化字符串（显示为 StringID）
- `--unfiltered`: 包含所有字符串，跳过智能过滤

### 翻译应用模式
- `--apply-file <JSON_FILE>`: 从 JSON 文件应用翻译
- `--apply-jsonstr <JSON_STRING>`: 从 JSON 字符串应用指定翻译
- `--apply-partial-stdin`: 从标准输入读取 JSON 翻译

### 高级功能
- `--eslify`: 转换为 ESL 插件（FormID 重编号）
- `--test-rebuild`: 测试解析和重建逻辑
- `--compare-files <FILE>`: 对比两个 ESP 文件的结构差异

完整选项请运行 `esp_extractor --help`。

## 💡 最佳实践

### 翻译工作流

1. **提取字符串**
   ```bash
   esp_extractor -i "MyMod.esp" -o "strings.json"
   ```

2. **编辑翻译**
   - 修改 JSON 文件中的 `original_text` 字段
   - 使用翻译工具（ChatGPT、DeepL）处理大量文本
   - 保持游戏术语的一致性

3. **应用翻译**
   ```bash
   esp_extractor -i "MyMod.esp" --apply-file "strings_cn.json" -o "MyMod_CN.esp"
   ```

4. **质量控制**
   - 在游戏中测试翻译效果
   - 检查特殊字符是否正确显示
   - 使用自动备份文件快速恢复（`.bak`）

### 性能建议

- 大文件处理：v0.5.0 已优化，在测试环境中 250MB 文件约 9 秒完成
- 部分翻译：只翻译需要的条目，减少文件大小和处理时间
- 并行处理：利用多核 CPU 加速（自动启用）

## 💻 库 API 使用

### 智能自动加载（推荐）

```rust
use esp_extractor::LoadedPlugin;

// 自动检测 LOCALIZED 标志并加载 STRING 文件
let loaded = LoadedPlugin::load_auto("MyMod.esp".into(), Some("english"))?;

// 提取字符串
let strings = loaded.extract_strings();
```

### 编辑器 API

```rust
use esp_extractor::{Plugin, PluginEditor, DefaultEspWriter};

// 加载插件
let plugin = Plugin::load("MyMod.esp".into())?;

// 创建编辑器
let mut editor = PluginEditor::new(plugin);

// 应用翻译
editor.apply_translations(translations)?;

// 保存修改
let writer = DefaultEspWriter;
editor.save(&writer, "MyMod_CN.esp".as_ref())?;
```

### 本地化插件处理

```rust
use esp_extractor::LocalizedPluginContext;

// 显式加载本地化插件（ESP + STRING 文件）
let context = LocalizedPluginContext::load("DLC.esm".into(), "english")?;

// 访问插件和 STRING 文件
println!("插件: {}", context.plugin().get_name());
println!("STRING 文件数: {}", context.string_files().files.len());

// 提取字符串（包含 STRING 文件内容）
let strings = context.plugin().extract_strings();
```

详细 API 文档请访问 [docs.rs](https://docs.rs/esp_extractor)。

## 📚 扩展文档

- [插件加载完整指南](docs/plugin-loading-guide.md)
- [STRING 文件使用说明](STRING_FILE_USAGE.md)
- [XXXX 超大子记录详解](XXXX_Subrecord_Handling.md)
- [Python 到 Rust 映射文档](esp_parser_mapping.md)

## 🛠️ 开发

```bash
# 构建库
cargo build

# 构建命令行工具
cargo build --features cli

# 运行测试
cargo test

# 生成文档
cargo doc --open
```

## 🤝 贡献

欢迎贡献代码！请查看 [CONTRIBUTING.md](CONTRIBUTING.md) 了解详细信息。

## 📜 许可证

本项目采用 MIT 或 Apache-2.0 双重许可证。详情请查看 [LICENSE-MIT](LICENSE-MIT) 和 [LICENSE-APACHE](LICENSE-APACHE) 文件。

## 🎉 致谢

- Bethesda Game Studios - 创造了这些出色的游戏
- ESP 文件格式的逆向工程社区
- Rust 社区提供的优秀库和工具

---

## 📝 版本历史

### v0.7.0 (2025-11-27) - 代码架构重构

**模块化重构**
- plugin.rs 拆分为 7 个子模块（parser, strings, translate, writer, stats, esl）
- string_file.rs 拆分为 6 个子模块（file, set, bsa, io, tests）
- 新增 string_routes 模块，支持自定义字符串路由
- IO 抽象层增强，支持依赖注入

**重构效果**
- 代码行数：2383 行 → 分散到 13 个文件（平均 ~183 行/文件）
- 职责清晰，易于维护和扩展
- 向后兼容：公共 API 保持不变

---

### v0.6.0 (2025-11-18) - BSA Fallback 与字段优化

**新增功能**
- BSA 归档 Fallback 机制：当文件系统找不到 STRING 文件时，自动从 BSA 归档中提取
  - 基于 `ba2` crate 实现 TES4 格式 BSA 读取
  - 路径规范化（大小写容错、路径分隔符统一）
  - 支持官方主文件特殊规则（Skyrim.esm 等共享 `Skyrim - Interface.bsa`）
  - 优先级：文件系统 → BSA → 失败
- 修复 AMMO.DESC 字段缺失：在 `string_records.json` 中添加 AMMO 的 DESC 支持

**字段命名优化**
- ExtractedString 字段重命名：`original_text` → `text`
  - 统一字段语义：提取时为原文，应用时为译文
- 新增 index 字段：用于同类型子记录的索引区分（v0.5.1 引入）

**测试验证**
- ccbgssse002-exoticarrows.esl：成功从 BSA 提取 59 个字符串
  - STRINGS: 44 个
  - DLSTRINGS: 15 个（含 2 个 AMMO.DESC）
  - ILSTRINGS: 0 个

**架构改进**
- 新增 `src/bsa/` 模块：
  - `BsaArchive`: 底层 BSA 文件访问
  - `BsaStringsProvider`: STRING 文件专用提取器
- 集成到 `LoadedPlugin::load_auto()` 工作流
- CLI 工具自动启用 BSA fallback

**依赖更新**
- 新增：`ba2 = "3"` - Bethesda 归档文件解析

---

### v0.5.2 (2025-11-14) - 性能一致性优化

**关键修复**
- 解决 load_auto 隐性性能问题：修复本地化插件重复加载导致的性能损失
  - 问题原因：`LoadedPlugin::load_auto()` 对本地化插件会加载 ESP 文件两次
  - 优化方案：引入 `LocalizedPluginContext::new_with_plugin()` 复用已加载的 Plugin
- 性能提升：Skyrim.esm (238MB) 加载时间从 3.28 秒降至 1.36 秒（测试环境同上）
- 代码清理：使用条件编译保护所有调试日志，Release 模式输出简洁

**架构改进**
- 新增 `LocalizedPluginContext::new_with_plugin()` API
- 消除重复的 ESP 文件解析
- 保持向后兼容，原有 API 不受影响
- 新增性能对比测试工具 `examples/performance_comparison.rs`

---

### v0.5.1 (2025-11-14) - STRING 类型路由修复

**重要修复**
- 修复 STRING 文件类型路由规则：简化路由逻辑，提升字符串提取完整性
  - INFO 记录 → 所有子记录路由到 ILSTRINGS
  - DESC/CNAM 子记录 → 无论 record 类型，都路由到 DLSTRINGS
  - 其他所有 → 默认路由到 STRINGS
- 移除无效字段配置：
  - 移除 PERK.EPFD 和 PERK.EPF2（浮点数字段，非字符串）
  - 移除 AMMO.DESC（非字符串字段）
- StringID=0 特殊处理：正确过滤空字符串引用
- 增强调试日志：警告信息包含 record 类型、FormID 和 EditorID

**提取率提升**
- Skyrim.esm: 从 21,502 条提升到 62,431 条
- Vigilant.esm: 7,610 条
- curio.esl: 81 条

**测试覆盖**
- 5 个核心路由规则单元测试
- 3 个真实 ESP/ESM/ESL 文件验证
- 文档更新，反映新的简化路由规则

---

### v0.5.0 (2025-11-14) - 性能优化版

**性能优化**
- 显著性能提升：Skyrim.esm (250MB) 加载时间从 240 秒缩短到 9 秒（测试环境见性能测试章节）
- 内存映射文件（memmap2）：零拷贝文件访问
- Copy-on-Write（Cow）：减少内存拷贝
- 并行处理（rayon）：Group 和字符串提取并行化

**功能完善**
- 完整的 10 种 GroupType 支持
- XXXX 超大子记录处理（> 65535 字节）
- 特殊记录索引跟踪（INFO/QUST/PERK）
- STRING 文件集成

**测试验证**
- Skyrim.esm 集成测试：9/10 通过
- 成功提取 20,437 条字符串（含 2,835 条特殊记录）
- 加载 67,414 条 STRING 文件条目

### v0.4.0 (2025) - 架构重构版

**架构升级**
- 分层架构：IO 抽象层 + 编辑器层
- 插件加载器
- 有状态编辑器和变更追踪
- 依赖注入和可测试性

**核心功能**
- ESP/ESM/ESL 文件解析
- STRING 文件支持
- 字符串提取和翻译应用
- 压缩记录支持

### v0.3.0 及更早版本

- 基础 ESP 文件解析
- 字符串提取功能
- JSON 格式输出

---

**当前版本**: v0.7.0
**推荐用途**: Mod 翻译、数据提取、批量处理、BSA 归档字符串提取
