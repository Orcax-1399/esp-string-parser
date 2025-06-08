# 部分翻译功能使用指南

## 🚀 问题解决

原来的 `--apply-jsonstr` 选项使用命令行参数传递 JSON 字符串，存在以下问题：

- **Windows**: 命令行长度限制 ~8191 字符
- **Linux**: 命令行长度限制 ~131072 字符 (128KB)
- **实际可用长度更短**：程序名、其他参数等会占用空间

当翻译数据较大时，可能超出命令行缓冲区限制，导致无法执行。

## 🎯 解决方案

我们现在提供了**三种方式**来应用部分翻译，用户可以根据实际情况选择：

### 1. 命令行参数（适合小数据）

```bash
# 原有方式，适合少量翻译条目
cargo run -- --input plugin.esp --apply-jsonstr '[{"editor_id":"TestItem","form_id":"00012345|plugin.esp","original_text":"翻译后的文本","record_type":"WEAP","subrecord_type":"FULL"}]'
```

**优点**: 简单直接  
**缺点**: 受命令行长度限制  
**适用**: 1-10 个翻译条目

### 2. 文件输入（推荐用于大数据）

```bash
# 从文件读取，避免命令行限制
cargo run -- --input plugin.esp --apply-file translations.json
```

**创建 `translations.json` 文件**:
```json
[
  {
    "editor_id": "TestItem1",
    "form_id": "00012345|plugin.esp",
    "original_text": "翻译后的文本1",
    "record_type": "WEAP",
    "subrecord_type": "FULL"
  },
  {
    "editor_id": "TestItem2", 
    "form_id": "00012346|plugin.esp",
    "original_text": "翻译后的文本2",
    "record_type": "ARMO",
    "subrecord_type": "FULL"
  }
]
```

**优点**: 无长度限制，可重复使用  
**缺点**: 需要创建文件  
**适用**: 大量翻译条目

### 3. 标准输入（适合脚本自动化）

```bash
# 从标准输入读取
cargo run -- --input plugin.esp --apply-partial-stdin
```

然后输入 JSON 数据，按 `Ctrl+D` (Unix) 或 `Ctrl+Z` (Windows) 结束输入。

**或者通过管道**:
```bash
# Unix/Linux/macOS
echo '[{"editor_id":"TestItem",...}]' | cargo run -- --input plugin.esp --apply-partial-stdin

# Windows PowerShell
echo '[{"editor_id":"TestItem",...}]' | cargo run -- --input plugin.esp --apply-partial-stdin

# 从文件管道
cat translations.json | cargo run -- --input plugin.esp --apply-partial-stdin
```

**优点**: 适合脚本自动化，无文件残留  
**缺点**: 相对复杂  
**适用**: 自动化脚本、CI/CD 流程

## 📋 使用示例

### 场景1: 快速测试单个翻译

```bash
cargo run -- --input plugin.esp --apply-jsonstr '[{"editor_id":"Sword01","form_id":"00001234|plugin.esp","original_text":"魔法剑","record_type":"WEAP","subrecord_type":"FULL"}]'
```

### 场景2: 批量应用翻译文件

```bash
# 1. 准备翻译文件 batch_translations.json
# 2. 应用翻译
cargo run -- --input plugin.esp --apply-file batch_translations.json --output translated_plugin.esp
```

### 场景3: 脚本自动化

```bash
#!/bin/bash
# generate_and_apply.sh

# 生成翻译数据
generate_translations() {
  echo '[
    {"editor_id":"Item1","form_id":"00001234|plugin.esp","original_text":"剑","record_type":"WEAP","subrecord_type":"FULL"},
    {"editor_id":"Item2","form_id":"00001235|plugin.esp","original_text":"盾","record_type":"ARMO","subrecord_type":"FULL"}
  ]'
}

# 应用翻译
generate_translations | cargo run -- --input plugin.esp --apply-partial-stdin
```

## ⚠️ 注意事项

### 互斥选项

**只能同时使用一种部分翻译方式**，程序会自动验证：

```bash
# ❌ 错误：同时使用多种方式
cargo run -- --input plugin.esp --apply-jsonstr '[]' --apply-file file.json

# 输出错误：只能使用一种部分翻译方式：--apply-jsonstr、--apply-file 或 --apply-partial-stdin
```

### JSON 格式验证

所有方式都会验证 JSON 格式和翻译数据的有效性：

- JSON 格式必须正确
- 翻译数组不能为空
- 每个翻译条目必须包含必要字段

### 输出文件

- 默认输出文件名：`原文件名.esp` （覆盖原文件名后缀）
- 可通过 `--output` 参数指定自定义输出路径

## 🎉 总结

| 方式 | JSON字符串 | 文件输入 | 标准输入 |
|------|------------|----------|----------|
| **限制** | ~8KB (Windows) | 无限制 | 无限制 |
| **适用场景** | 快速测试 | 批量处理 | 自动化脚本 |
| **复杂度** | 简单 | 中等 | 复杂 |
| **重复使用** | 困难 | 容易 | 中等 |

现在你可以根据实际需求选择最合适的方式来应用翻译，完全避免了命令行缓冲区限制的问题！🚀 