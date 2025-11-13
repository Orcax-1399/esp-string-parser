# 插件加载指南

本文档说明 ESP 字符串解析器 v0.4.0 的三种插件加载方式及其使用场景。

---

## 📚 三种加载方式对比

### 1. **`LoadedPlugin::load_auto()`** - 智能自动加载（推荐）

✅ **最简单、最智能的方式**

- 自动检测插件是否设置 `LOCALIZED` 标志
- 如果是本地化插件，自动加载 STRING 文件
- 如果 STRING 文件加载失败，自动降级为普通插件模式
- **适合大多数场景**

```rust
use esp_extractor::LoadedPlugin;

// 自动检测并选择合适的加载方式
let loaded = LoadedPlugin::load_auto(
    "MyMod.esp".into(),
    Some("english")
)?;

match loaded {
    LoadedPlugin::Standard(plugin) => {
        println!("普通插件: {}", plugin.get_name());
        let strings = plugin.extract_strings();
    }
    LoadedPlugin::Localized(context) => {
        println!("本地化插件: {}", context.plugin().get_name());
        println!("STRING 文件数: {}", context.string_files().files.len());
        let strings = context.plugin().extract_strings();
    }
}

// 或使用统一接口（无需模式匹配）
let strings = loaded.extract_strings();
println!("提取了 {} 个字符串", strings.len());
```

**优点**：
- ✅ 自动化处理，无需手动判断
- ✅ 容错性好（STRING 文件缺失时自动降级）
- ✅ 代码简洁

**缺点**：
- ⚠️ 返回 enum，需要模式匹配（但提供了统一接口方法）

---

### 2. **`Plugin::load()`** - 基础加载（精确控制）

✅ **需要精确控制时使用**

- 只解析 ESP/ESM/ESL 文件本身
- **不会**自动加载 STRING 文件（即使设置了 LOCALIZED 标志）
- 适合只需要 ESP 文件结构，不关心字符串内容的场景

```rust
use esp_extractor::Plugin;

// 只加载 ESP 文件
let plugin = Plugin::load("MyMod.esp".into())?;

// 检查是否为本地化插件
if plugin.is_localized() {
    println!("这是一个本地化插件，但 STRING 文件未加载");
    println!("字符串将显示为 StringID_xxx");
}

// 提取字符串（本地化插件将显示 StringID）
let strings = plugin.extract_strings();
```

**优点**：
- ✅ 轻量级，只加载必要数据
- ✅ 职责单一，行为可预测
- ✅ 适合批量处理（不需要读取 STRING 文件）

**缺点**：
- ⚠️ 本地化插件的字符串会显示为 `StringID_xxx`
- ⚠️ 需要手动处理 STRING 文件

**使用场景**：
- 📊 分析 ESP 文件结构
- 🔍 查找特定记录
- 📈 统计信息（记录数、组数等）
- 🚀 批量处理（不需要实际字符串内容）

---

### 3. **`LocalizedPluginContext::load()`** - 本地化插件专用

✅ **明确知道是本地化插件时使用**

- 同时加载 ESP 文件和 STRING 文件
- 如果 STRING 文件不存在或加载失败，会返回错误
- 适合已知是本地化插件的场景

```rust
use esp_extractor::LocalizedPluginContext;

// 显式加载本地化插件
let context = LocalizedPluginContext::load(
    "Skyrim.esm".into(),
    "english"
)?;

// 访问插件
let plugin = context.plugin();
println!("插件名: {}", plugin.get_name());

// 访问 STRING 文件
let string_files = context.string_files();
println!("STRING 文件数: {}", string_files.files.len());

// 提取字符串（从 STRING 文件读取实际内容）
let strings = plugin.extract_strings();
```

**优点**：
- ✅ 明确的语义，代码可读性高
- ✅ 类型安全，保证 STRING 文件已加载
- ✅ 可以直接访问 `StringFileSet`

**缺点**：
- ⚠️ 如果不是本地化插件或 STRING 文件缺失，会报错
- ⚠️ 需要事先知道插件类型

**使用场景**：
- 📝 处理已知的本地化插件（如 Skyrim.esm, Fallout4.esm）
- 🔧 需要修改 STRING 文件
- 📦 打包/解包本地化资源

---

## 🎯 推荐使用流程图

```
开始
  ↓
确定使用场景
  ↓
  ├─→ 通用场景，不确定插件类型？
  │     → 使用 LoadedPlugin::load_auto() ✅ 最推荐
  │
  ├─→ 只需要 ESP 结构，不关心字符串内容？
  │     → 使用 Plugin::load()
  │
  └─→ 明确知道是本地化插件，需要处理 STRING 文件？
        → 使用 LocalizedPluginContext::load()
```

---

## 📝 实际示例：字符串提取工具

### 场景：提取任意插件的字符串

```rust
use esp_extractor::LoadedPlugin;
use std::path::PathBuf;

fn extract_strings_from_any_plugin(
    path: PathBuf,
    language: Option<&str>,
) -> Result<Vec<ExtractedString>, Box<dyn std::error::Error>> {
    // 使用智能加载器
    let loaded = LoadedPlugin::load_auto(path, language)?;

    // 统一接口提取字符串
    Ok(loaded.extract_strings())
}

// 使用
let strings = extract_strings_from_any_plugin(
    "SomeUnknownMod.esp".into(),
    Some("english")
)?;

println!("提取了 {} 个字符串", strings.len());
```

### 场景：只统计记录数量（不需要字符串）

```rust
use esp_extractor::Plugin;

fn count_records(path: PathBuf) -> Result<usize, Box<dyn std::error::Error>> {
    // 使用基础加载器（更快）
    let plugin = Plugin::load(path)?;

    let stats = plugin.get_stats();
    Ok(stats.record_count)
}
```

### 场景：修改本地化插件的 STRING 文件

```rust
use esp_extractor::LocalizedPluginContext;

fn modify_string_files(path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    // 使用本地化上下文
    let mut context = LocalizedPluginContext::load(path, "english")?;

    // 修改 STRING 文件
    let string_files = context.string_files_mut();
    // ... 修改操作 ...

    // 保存
    context.save_string_files(Path::new("output"))?;

    Ok(())
}
```

---

## ⚠️ 迁移指南：从 v0.3.x 升级

### 旧代码（v0.3.x）

```rust
// 自动检测并加载
let plugin = Plugin::new(path, Some("english"))?;
```

### 新代码（v0.4.0+）- 推荐写法

```rust
// 推荐：使用智能加载器
let loaded = LoadedPlugin::load_auto(path, Some("english"))?;
let strings = loaded.extract_strings();
```

### 新代码（v0.4.0+）- 保守写法

```rust
// 保守：如果只需要 ESP 文件
let plugin = Plugin::load(path)?;

// 如果需要 STRING 文件，手动判断
if plugin.is_localized() {
    let context = LocalizedPluginContext::load(path, "english")?;
    // 使用 context
} else {
    // 使用 plugin
}
```

---

## 🔮 未来计划

在 v0.5.0 中，我们计划：
- 为 `LoadedPlugin` 添加更多便捷方法
- 支持懒加载 STRING 文件（按需加载）
- 提供异步加载 API

---

## 💡 常见问题

### Q: 为什么 `Plugin::load()` 不自动加载 STRING 文件？

**A:** 遵循**单一职责原则**（SOLID）。`Plugin::load()` 只负责解析 ESP 文件结构，不负责加载外部资源。这样：
- 代码职责清晰
- 性能更好（不需要字符串内容时避免额外 IO）
- 测试更容易

### Q: 我应该用哪个 API？

**A:**
- 🟢 **99% 的情况** → `LoadedPlugin::load_auto()`
- 🟡 **只需要结构** → `Plugin::load()`
- 🟠 **明确本地化插件** → `LocalizedPluginContext::load()`

### Q: 旧代码会被破坏吗？

**A:** 不会！`Plugin::new()` 标记为 `deprecated` 但仍可用。建议尽快迁移到新 API。

---

**文档版本**: v0.4.0
**最后更新**: 2025-11-13
