# ESP字符串提取工具 (esp_extractor)

[![Crates.io](https://img.shields.io/crates/v/esp_extractor.svg)](https://crates.io/crates/esp_extractor)
[![Documentation](https://docs.rs/esp_extractor/badge.svg)](https://docs.rs/esp_extractor)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](https://opensource.org/licenses/MIT)

一个用于处理Bethesda游戏引擎（ESP/ESM/ESL）文件的Rust库。支持字符串提取、翻译应用和文件结构调试。

## 🎯 功能特性

- 🎮 **支持多种游戏格式**：ESP、ESM、ESL文件
- 📝 **字符串提取**：提取可翻译的文本内容
- 🌍 **翻译应用**：将翻译后的字符串写回游戏文件
- 🗜️ **压缩记录支持**：正确处理压缩和未压缩的记录
- 🔤 **多编码支持**：UTF-8、GBK、ASCII等编码格式
- 🐛 **调试工具**：详细的文件结构分析和对比功能
- ⚡ **高性能**：使用Rust编写，安全且高效
- **🌏 支持中文、日文、韩文等Unicode字符**
- 本地化插件支持
- 详细的插件统计信息
- JSON 格式输出
- **🔄 完整的翻译工作流：提取 → 翻译 → 应用**

## 📦 安装

### 作为库使用

将以下内容添加到你的 `Cargo.toml` 文件中：

```toml
[dependencies]
esp_extractor = "0.2.0"
```

### 作为命令行工具

```bash
cargo install esp_extractor --features cli
```

或者从源码构建：

```bash
git clone https://github.com/yourusername/esp-string-parser.git
cd esp-string-parser
cargo build --release --features cli
```

## 🚀 完整翻译工作流

### 第一步：提取字符串
```bash
esp_extractor -i "MyMod.esp" -o "MyMod_strings.json"
```

**输出示例：**
```
正在解析插件: "MyMod.esp"
扫描到 15 个组（包含子组）
扫描到 1250 个记录
提取到 324 个有效字符串
结果已写入: "MyMod_strings.json"

样例字符串:
1. [00012BB7|Skyrim.esm] WEAP FULL: "Iron Sword"
2. [00013BB9|MyMod.esp] NPC_ FULL: "神秘商人"
...
```

### 第二步：编辑翻译
直接修改JSON文件中的 `original_text` 字段为翻译文本：

**修改前：**
```json
{
    "editor_id": "IronSword",
    "form_id": "00012BB7|Skyrim.esm",
    "original_text": "Iron Sword",
    "record_type": "WEAP",
    "subrecord_type": "FULL",
}
```

**修改后：**
```json
{
  "editor_id": "IronSword",
  "form_id": "00012BB7|Skyrim.esm", 
  "original_text": "铁剑",
  "record_type": "WEAP",
  "subrecord_type": "FULL"
}
```

### 第三步：应用翻译

#### 方法一：完整文件翻译
```bash
esp_extractor -i "Data/MyMod.esp" --apply-file "MyMod_CN.json" -o "Data/MyMod_CN.esp"
```

#### 方法二：部分对象翻译（推荐）
```bash
esp_extractor -i "Data/MyMod.esp" --apply-jsonstr '[{"editor_id":"IronSword","form_id":"00012BB7|Skyrim.esm","original_text":"铁剑","record_type":"WEAP","subrecord_type":"FULL"}]' -o "Data/MyMod_CN.esp"
```

#### 方法三：从标准输入读取翻译
```bash
cat MyMod_translations.json | esp_extractor -i "Data/MyMod.esp" --apply-partial-stdin -o "Data/MyMod_CN.esp"
```

#### 方法四：自动命名输出文件（覆盖原文件）
```bash
esp_extractor -i "Data/MyMod.esp" --apply-file "MyMod_CN.json"
# 输出: MyMod.esp (覆盖原文件，会自动创建备份)

esp_extractor -i "Data/MyMod.esp" --apply-jsonstr '[...]'
# 输出: MyMod.esp (覆盖原文件，会自动创建备份)
```

**翻译应用输出示例：**
```
准备应用 1 个翻译条目
翻译条目 1: [00012BB7|Skyrim.esm] WEAP FULL -> "铁剑"
翻译应用完成，输出到: "MyMod_CN.esp"
```

## ⚙️ 命令行选项

### 提取模式
- `-i, --input <FILE>`: 输入ESP/ESM/ESL文件路径 (必需)
- `-o, --output <FILE>`: 输出JSON文件路径 (可选)
- `--include-localized`: 包含本地化字符串(显示为StringID)
- `--unfiltered`: 包含所有字符串，跳过智能过滤
- `--stats`: 仅显示插件统计信息
- `--quiet`: 静默模式

### 翻译应用模式
- `--apply-file <JSON_FILE>`: 从JSON文件应用翻译到ESP文件
- `--apply-jsonstr <JSON_STRING>`: 从JSON字符串应用指定的翻译对象
- `--apply-partial-stdin`: 从标准输入读取JSON翻译对象

### 测试和调试模式
- `--test-rebuild`: 测试模式，解析文件后直接重建（不做任何修改），用于验证解析和重建逻辑
- `--compare-files <ESP_FILE>`: 对比两个ESP文件的结构差异

## 📋 使用示例

### 基本提取
```bash
# 提取字符串到JSON文件
esp_extractor -i "Data/MyMod.esp"

# 指定输出文件名
esp_extractor -i "Data/MyMod.esp" -o "translations.json"
```

### 显示统计信息
```bash
esp_extractor -i "Data/Skyrim.esm" --stats
```

### 包含特殊情况
```bash
# 包含本地化字符串
esp_extractor -i "Data/MyMod.esp" --include-localized

# 包含所有字符串（不过滤）
esp_extractor -i "Data/MyMod.esp" --unfiltered
```

### 应用翻译
```bash
# 完整文件翻译
esp_extractor -i "Data/MyMod.esp" --apply-file "MyMod_CN.json" -o "Data/MyMod_CN.esp"

# JSON字符串翻译（推荐用于少量翻译）
esp_extractor -i "Data/MyMod.esp" --apply-jsonstr '[{"editor_id":"IronSword","form_id":"00012BB7|Skyrim.esm","original_text":"铁剑","record_type":"WEAP","subrecord_type":"FULL"}]' -o "Data/MyMod_CN.esp"

# 从标准输入读取翻译
cat MyMod_translations.json | esp_extractor -i "Data/MyMod.esp" --apply-partial-stdin -o "Data/MyMod_CN.esp"

# 自动命名输出文件（覆盖原文件）
esp_extractor -i "Data/MyMod.esp" --apply-file "MyMod_CN.json"
# 输出: MyMod.esp (覆盖原文件，会自动创建备份)

esp_extractor -i "Data/MyMod.esp" --apply-jsonstr '[...]'
# 输出: MyMod.esp (覆盖原文件，会自动创建备份)
```

### 测试文件重建
```bash
# 测试解析和重建逻辑（用于调试）
esp_extractor -i "Data/MyMod.esp" --test-rebuild

# 指定输出文件
esp_extractor -i "Data/MyMod.esp" --test-rebuild -o "MyMod_test.esp"
```

### 文件结构对比
```bash
# 对比两个ESP文件的结构差异
esp_extractor -i "Data/MyMod_Original.esp" --compare-files "Data/MyMod_Modified.esp"

# 静默模式对比（仅显示差异）
esp_extractor -i "Data/MyMod_Original.esp" --compare-files "Data/MyMod_Modified.esp" --quiet
```

## 📄 输出格式

程序输出 JSON 格式的字符串数组：

```json
{
  "editor_id": "IronSword",
  "form_id": "00012BB7|Skyrim.esm", 
  "original_text": "Iron Sword",
  "record_type": "WEAP",
  "subrecord_type": "FULL"
}
```

### 字段说明
- `editor_id`: 编辑器ID
- `form_id`: FormID|主文件名  
- `original_text`: 原始文本（提取时为原文，应用翻译时修改为译文）
- `record_type`: 记录类型
- `subrecord_type`: 子记录类型

### 🔑 匹配机制
应用翻译时使用三重匹配确保精确性：
- `editor_id` + `form_id` + `record_type + " " + subrecord_type` 
- 这避免了不同对象共享相同ID时的冲突

## 🎮 支持的记录类型

- **WEAP** (武器): FULL, DESC
- **ARMO** (装备): FULL, DESC  
- **NPC_** (NPC): FULL, SHRT
- **BOOK** (书籍): FULL, DESC, CNAM
- **QUST** (任务): FULL, CNAM, NNAM
- **INFO** (对话): NAM1, RNAM
- **DIAL** (对话主题): FULL
- **MESG** (消息): DESC, FULL, ITXT
- 以及更多... (详见 `data/string_records.json`)

## 🔍 字符串过滤规则

**自动过滤的内容：**
- 空字符串
- 驼峰命名变量 (`MyVariable`)
- 下划线命名变量 (`my_variable`)
- 黑名单文本 (`<p>`)
- 控制字符

**支持的字符：**
- ✅ 所有Unicode可打印字符（中文、日文、韩文等）
- ✅ 英文字符和数字
- ✅ 标点符号和空格

## 💡 翻译工作流最佳实践

### 1. 高效翻译建议
- **使用部分翻译**：只翻译需要的条目，减少文件大小
- **三重匹配验证**：确保 `editor_id + form_id + record_type + " " + subrecord_type` 匹配正确
- **自动备份**：程序会自动创建 `.bak` 备份文件
- 使用翻译工具（如 ChatGPT、DeepL）处理大量文本
- 保持游戏术语的一致性

### 2. 部分翻译工作流
```bash
# 1. 提取所有字符串
esp_extractor -i "MyMod.esp" -o "all_strings.json"

# 2. 选择需要翻译的条目，复制到单独文件或直接使用
# 3. 修改 original_text 字段为翻译文本
# 4. 应用翻译
esp_extractor -i "MyMod.esp" --apply-jsonstr '[翻译的JSON对象]' -o "MyMod_CN.esp"

# 或者从文件应用
esp_extractor -i "MyMod.esp" --apply-file "selected_translations.json" -o "MyMod_CN.esp"

# 或者从标准输入应用（适合脚本处理）
cat selected_translations.json | esp_extractor -i "MyMod.esp" --apply-partial-stdin -o "MyMod_CN.esp"
```

### 3. 质量控制
- 翻译完成后在游戏中测试
- 检查特殊字符是否正确显示
- 验证格式字符串是否保留
- 使用备份文件快速恢复

### 4. 版本管理
- 保留原始提取的JSON文件
- 程序自动创建时间戳备份文件
- 使用版本控制系统管理翻译文件

## 🛠️ 故障排除

### 常见问题

1. **"Invalid file format" 错误**
   - 确保文件是有效的 ESP/ESM/ESL 文件

2. **"Insufficient data" 错误**
   - 文件可能被截断或损坏

3. **编码问题**
   - 程序会自动尝试多种编码

4. **翻译应用失败**
   - 检查JSON文件格式
   - 确保FormID匹配

## 🎮 支持的游戏

- The Elder Scrolls V: Skyrim
- The Elder Scrolls V: Skyrim Special Edition
- Fallout 4
- Fallout: New Vegas
- Fallout 3
- The Elder Scrolls IV: Oblivion
- 其他使用Creation Engine/Gamebryo引擎的游戏

## 📄 文件格式支持

- **ESP** (Elder Scrolls Plugin)
- **ESM** (Elder Scrolls Master)
- **ESL** (Elder Scrolls Light)

## 📚 API文档

详细的API文档可以在 [docs.rs](https://docs.rs/esp_extractor) 上查看。

## 🎮 开发

### 🛠️ 构建

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

### 📁 目录结构

```
src/
├── lib.rs          # 库的主入口
├── main.rs         # 命令行工具入口
├── datatypes.rs    # 基础数据类型定义
├── record.rs       # 记录解析逻辑
├── group.rs        # 组解析逻辑
├── plugin.rs       # 插件主类
├── subrecord.rs    # 子记录解析
├── string_types.rs # 字符串类型定义
├── utils.rs        # 工具函数
└── debug.rs        # 调试工具
data/
└── string_records.json  # 字符串记录定义
```

## 🤝 贡献

欢迎贡献代码！请查看 [CONTRIBUTING.md](CONTRIBUTING.md) 了解详细信息。

## 📜 许可证

本项目采用 MIT 或 Apache-2.0 双重许可证。详情请查看 [LICENSE-MIT](LICENSE-MIT) 和 [LICENSE-APACHE](LICENSE-APACHE) 文件。

## 🎉 致谢

- Bethesda Game Studios - 创造了这些出色的游戏
- ESP文件格式的逆向工程社区
- Rust社区提供的优秀库和工具