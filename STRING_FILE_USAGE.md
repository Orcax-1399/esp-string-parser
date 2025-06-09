# Bethesda字符串文件解析功能

本工具现在支持解析Bethesda游戏的字符串文件（.STRINGS、.ILSTRINGS、.DLSTRINGS），并将其转换为JSON格式。

## 支持的文件类型

- **STRINGS** - 一般字符串文件
- **ILSTRINGS** - 界面字符串文件  
- **DLSTRINGS** - 对话字符串文件

## 文件命名规范

字符串文件通常遵循以下命名格式：
```
{插件名}_{语言}.{类型}
```

例如：
- `Dragonborn_english.ILSTRINGS`
- `Dragonborn_chinese.ILSTRINGS`
- `Skyrim_english.STRINGS`

## 使用方法

### 1. 自动检测模式

直接指定字符串文件，工具会自动检测文件类型并解析：

```bash
# 解析字符串文件并输出JSON
esp_extractor -i Dragonborn_english.ILSTRINGS

# 指定输出文件
esp_extractor -i Dragonborn_english.ILSTRINGS -o dragonborn_strings.json
```

### 2. 显式解析模式

使用 `--parse-strings` 参数明确指定要解析字符串文件：

```bash
esp_extractor --parse-strings Dragonborn_english.ILSTRINGS
```

### 3. 查看统计信息

使用 `--stats` 参数查看字符串文件的统计信息：

```bash
esp_extractor -i Dragonborn_english.ILSTRINGS --stats
```

输出示例：
```
=== 字符串文件统计 ===
插件名称: Dragonborn
语言: english
文件类型: ILSTRINGS
字符串数量: 1234
内容总大小: 45678 字节
原始数据大小: 50000 字节
平均字符串长度: 37.0 字符
```

### 4. 静默模式

使用 `--quiet` 参数只输出错误信息：

```bash
esp_extractor -i Dragonborn_english.ILSTRINGS --quiet
```

## 输出格式

解析后的JSON文件包含字符串条目数组，每个条目包含以下字段：

```json
[
  {
    "id": 12345,
    "directory_address": 1024,
    "relative_offset": 0,
    "absolute_offset": 2048,
    "length": 25,
    "content": "这是一个示例字符串",
    "raw_data": [228, 184, 173, 230, 150, 135, ...]
  }
]
```

字段说明：
- `id`: 字符串ID
- `directory_address`: 目录条目在文件中的位置
- `relative_offset`: 相对偏移量
- `absolute_offset`: 绝对偏移量
- `length`: 字符串长度（仅DLSTRINGS/ILSTRINGS有效）
- `content`: 字符串内容（UTF-8编码）
- `raw_data`: 原始字节数据

## 文件结构说明

### 文件头部（8字节）
```
0x00-0x03: 字符串数量 (Uint32, 小端序)
0x04-0x07: 字符串数据总长度 (Uint32, 小端序)
```

### 目录条目（每个8字节）
```
0x00-0x03: 字符串ID (Uint32, 小端序)
0x04-0x07: 相对偏移量 (Uint32, 小端序)
```

### 字符串数据
#### DLSTRINGS/ILSTRINGS格式：
```
0x00-0x03: 字符串长度 (Uint32, 小端序)
0x04-0xN:  字符串内容 (UTF-8)
0xN+1:     空终止符 (0x00)
```

#### STRINGS格式：
```
0x00-0xN:  字符串内容 (UTF-8)
0xN+1:     空终止符 (0x00)
```

## 编程接口

如果你想在Rust代码中使用字符串文件解析功能：

```rust
use esp_extractor::StringFile;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 解析字符串文件
    let string_file = StringFile::new(PathBuf::from("Dragonborn_english.ILSTRINGS"))?;
    
    // 获取统计信息
    let stats = string_file.get_stats();
    println!("{}", stats);
    
    // 获取特定字符串
    if let Some(entry) = string_file.get_string(12345) {
        println!("字符串内容: {}", entry.content);
    }
    
    // 查找包含特定文本的字符串
    let matching_strings = string_file.find_strings_containing("Dragon");
    println!("找到 {} 个包含'Dragon'的字符串", matching_strings.len());
    
    Ok(())
}
```

## 注意事项

1. **编码支持**：工具支持UTF-8和Windows-1252编码的自动检测
2. **内存使用**：大文件处理时请注意内存使用情况
3. **文件完整性**：工具会验证文件格式和数据完整性
4. **错误处理**：遇到损坏的文件时会提供详细的错误信息

## 常见问题

**Q: 为什么解析出的字符串数量与预期不符？**
A: 可能是文件损坏或格式不正确。使用 `--stats` 参数查看详细信息。

**Q: 如何处理不同编码的字符串文件？**
A: 工具会自动检测并处理UTF-8和Windows-1252编码。

**Q: 可以批量处理多个字符串文件吗？**
A: 目前需要逐个处理，可以使用脚本进行批量操作。 