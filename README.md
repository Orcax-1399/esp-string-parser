# ESP字符串提取工具 - Rust版本

一个用于从 Elder Scrolls 游戏的 ESP/ESM/ESL 插件文件中提取可翻译字符串的命令行工具。

## 🎯 功能特性

- 支持 ESP、ESM、ESL 文件格式
- 自动解析压缩记录
- 多编码支持 (UTF-8, Windows-1252, Windows-1250, Windows-1251)
- 智能字符串过滤，排除变量名和无效文本
- **🌏 支持中文、日文、韩文等Unicode字符**
- 本地化插件支持
- 详细的插件统计信息
- JSON 格式输出
- **🔄 完整的翻译工作流：提取 → 翻译 → 应用**

## 📦 安装

确保已安装 Rust (1.70+)，然后编译项目：

```bash
cargo build --release
```

编译完成后，可执行文件位于 `target/release/esp_extractor`。

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
    "string_type": "WEAP FULL",
    "original_text": "Iron Sword",
    "index": null,
    "record_type": "WEAP",
    "subrecord_type": "FULL",
    "encoding": "utf-8"
}
```

**修改后：**
```json
{
    "editor_id": "IronSword", 
    "form_id": "00012BB7|Skyrim.esm",
    "string_type": "WEAP FULL",
    "original_text": "铁剑",
    "index": null,
    "record_type": "WEAP",
    "subrecord_type": "FULL",
    "encoding": "utf-8"
}
```

### 第三步：应用翻译

#### 方法一：完整文件翻译
```bash
esp_extractor -i "MyMod.esp" --apply-translations "MyMod_strings.json" -o "MyMod_CN.esp"
```

#### 方法二：部分对象翻译（推荐）
```bash
esp_extractor -i "MyMod.esp" --apply-partial '[{"editor_id":"IronSword","form_id":"00012BB7|Skyrim.esm","string_type":"WEAP FULL","original_text":"铁剑","index":null,"record_type":"WEAP","subrecord_type":"FULL","encoding":"utf-8"}]' -o "MyMod_CN.esp"
```

**输出示例：**
```
已创建备份文件: "MyMod.2025-6-8-22-49-32.bak"
正在应用翻译: "MyMod_strings.json" -> "MyMod.esp"
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
- `--apply-translations <JSON_FILE>`: 从JSON文件应用翻译到ESP文件
- `--apply-partial <JSON_STRING>`: 从JSON字符串应用指定的翻译对象（部分翻译）

### 测试模式
- `--test-rebuild`: 测试模式，解析文件后直接重建（不做任何修改），用于验证解析和重建逻辑

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
esp_extractor -i "Data/MyMod.esp" --apply-translations "MyMod_CN.json" -o "Data/MyMod_CN.esp"

# 部分对象翻译（推荐）
esp_extractor -i "Data/MyMod.esp" --apply-partial '[{"editor_id":"IronSword","form_id":"00012BB7|Skyrim.esm","string_type":"WEAP FULL","original_text":"铁剑","index":null,"record_type":"WEAP","subrecord_type":"FULL","encoding":"utf-8"}]'

# 自动命名输出文件
esp_extractor -i "Data/MyMod.esp" --apply-translations "MyMod_CN.json"
# 输出: MyMod_translated.esp

esp_extractor -i "Data/MyMod.esp" --apply-partial '[...]'
# 输出: MyMod_partial.esp
```

### 测试文件重建
```bash
# 测试解析和重建逻辑（用于调试）
esp_extractor -i "Data/MyMod.esp" --test-rebuild

# 指定输出文件
esp_extractor -i "Data/MyMod.esp" --test-rebuild -o "MyMod_test.esp"
```

## 📄 输出格式

程序输出 JSON 格式的字符串数组：

```json
{
  "editor_id": "IronSword",
  "form_id": "00012BB7|Skyrim.esm", 
  "string_type": "WEAP FULL",
  "original_text": "Iron Sword",
  "index": null,
  "record_type": "WEAP",
  "subrecord_type": "FULL",
  "encoding": "utf-8"
}
```

### 字段说明
- `editor_id`: 编辑器ID
- `form_id`: FormID|主文件名  
- `string_type`: 字符串类型
- `original_text`: 原始文本（提取时为原文，应用翻译时修改为译文）
- `index`: 字符串索引
- `record_type`: 记录类型
- `subrecord_type`: 子记录类型  
- `encoding`: 字符串编码

### 🔑 匹配机制
应用翻译时使用三重匹配确保精确性：
- `editor_id` + `form_id` + `string_type` 
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
- **三重匹配验证**：确保 `editor_id + form_id + string_type` 匹配正确
- **自动备份**：程序会自动创建 `.bak` 备份文件
- 使用翻译工具（如 ChatGPT、DeepL）处理大量文本
- 保持游戏术语的一致性

### 2. 部分翻译工作流
```bash
# 1. 提取所有字符串
esp_extractor -i "MyMod.esp" -o "all_strings.json"

# 2. 选择需要翻译的条目，复制到单独文件或直接使用
# 3. 修改 original_text 字段为翻译文本
# 4. 应用部分翻译
esp_extractor -i "MyMod.esp" --apply-partial '[翻译的JSON对象]' -o "MyMod_CN.esp"
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

## ��️ 开发

### 项目结构
```