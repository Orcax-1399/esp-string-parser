//! 基本使用示例
//! 
//! 这个示例展示了如何使用esp_extractor库的基本功能：
//! 1. 加载和分析ESP文件
//! 2. 提取可翻译字符串
//! 3. 应用翻译到ESP文件

use esp_extractor::{LoadedPlugin, ExtractedString, is_supported_file, SUPPORTED_EXTENSIONS, VERSION};
use std::path::{Path, PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("ESP字符串提取工具 v{}", VERSION);
    println!("支持的文件格式: {:?}", SUPPORTED_EXTENSIONS);

    // 从命令行获取文件路径
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("用法: {} <ESP文件路径> [模式]", args[0]);
        println!("示例: {} example.esp", args[0]);
        println!("示例: {} example.esp apply", args[0]);
        println!();
        println!("模式:");
        println!("  (无参数)  - 提取字符串并保存到JSON");
        println!("  apply     - 演示翻译应用功能");
        return Ok(());
    }

    let file_path = PathBuf::from(&args[1]);
    let mode = args.get(2).map(|s| s.as_str()).unwrap_or("extract");

    // 检查文件是否存在和格式是否支持
    if !file_path.exists() {
        eprintln!("错误: 文件不存在: {:?}", file_path);
        return Ok(());
    }

    if !is_supported_file(&file_path) {
        eprintln!("错误: 不支持的文件格式。支持的格式: {:?}", SUPPORTED_EXTENSIONS);
        return Ok(());
    }

    println!("正在分析文件: {:?}", file_path);

    // 加载插件（使用新的智能加载器）
    let loaded = LoadedPlugin::load_auto(file_path.clone(), Some("english"))?;
    let plugin = loaded.plugin();
    
    // 显示基本信息
    println!("\n=== 插件信息 ===");
    println!("插件名: {}", plugin.get_name());
    println!("插件类型: {}", plugin.get_type());
    println!("是否为主文件: {}", plugin.is_master());
    println!("是否本地化: {}", plugin.is_localized());
    
    // 显示统计信息
    let stats = plugin.get_stats();
    println!("\n=== 统计信息 ===");
    println!("{}", stats);
    
    // 提取字符串
    println!("\n=== 提取字符串 ===");
    let strings = plugin.extract_strings();
    
    if strings.is_empty() {
        println!("未找到可翻译的字符串");
        return Ok(());
    }
    
    println!("找到 {} 个可翻译字符串", strings.len());
    
    // 显示前5个字符串作为示例
    println!("\n前5个字符串示例:");
    for (i, string) in strings.iter().take(5).enumerate() {
        println!("{}. [{}] {}: \"{}\"", 
            i + 1,
            string.form_id,
            string.get_string_type(),
            if string.original_text.chars().count() > 50 {
                format!("{}...", string.original_text.chars().take(50).collect::<String>())
            } else {
                string.original_text.clone()
            }
        );
    }
    
    if strings.len() > 5 {
        println!("... 还有 {} 个字符串", strings.len() - 5);
    }
    
    // 保存到JSON文件
    let output_path = file_path.with_extension("json");
    let json_output = serde_json::to_string_pretty(&strings)?;
    std::fs::write(&output_path, json_output)?;
    println!("\n字符串已保存到: {:?}", output_path);
    
    // 根据模式执行不同操作
    match mode {
        "apply" => {
            demonstrate_translation_application(&file_path, &strings)?;
        }
        _ => {
            println!("\n提示: 使用 '{} {} apply' 来演示翻译应用功能", args[0], args[1]);
        }
    }
    
    Ok(())
}

/// 演示翻译应用功能
fn demonstrate_translation_application(
    file_path: &Path,
    original_strings: &[ExtractedString]
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== 翻译应用演示 ===");
    
    if original_strings.is_empty() {
        println!("没有可翻译的字符串，跳过翻译演示");
        return Ok(());
    }
    
    // 创建示例翻译数据（翻译前几个字符串作为演示）
    let mut translations = Vec::new();
    
    for (i, string) in original_strings.iter().take(3).enumerate() {
        let mut translated = string.clone();
        
        // 简单的示例翻译（在原文前加上序号）
        let translated_text = format!("[翻译{}] {}", i + 1, string.original_text);
        translated.original_text = translated_text.clone();
        
        println!("准备翻译: [{}] {} -> \"{}\"",
            string.form_id,
            string.get_string_type(),
            translated_text
        );
        
        translations.push(translated);
    }
    
    if translations.is_empty() {
        println!("没有准备翻译数据");
        return Ok(());
    }
    
    println!("\n正在应用 {} 个翻译...", translations.len());
    
    // 准备输出文件路径
    let output_path = file_path.with_extension("translated.esp");
    
    // 应用翻译（使用旧 API 兼容性）
    #[allow(deprecated)]
    match esp_extractor::Plugin::apply_translations(file_path.to_path_buf(), output_path.clone(), translations, None) {
        Ok(()) => {
            println!("✓ 翻译应用成功！");
            println!("输出文件: {:?}", output_path);
            
            // 验证翻译结果
            verify_translation_result(&output_path)?;
        }
        Err(e) => {
            eprintln!("✗ 翻译应用失败: {}", e);
        }
    }
    
    Ok(())
}

/// 验证翻译结果
fn verify_translation_result(translated_file: &Path) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== 验证翻译结果 ===");

    // 重新加载翻译后的文件（使用新的加载器）
    let translated_loaded = LoadedPlugin::load_auto(translated_file.to_path_buf(), Some("english"))?;
    let translated_strings = translated_loaded.extract_strings();
    
    println!("翻译后文件包含 {} 个字符串", translated_strings.len());
    
    // 显示翻译后的前几个字符串
    println!("\n翻译后的字符串示例:");
    for (i, string) in translated_strings.iter().take(3).enumerate() {
        if string.original_text.starts_with("[翻译") {
            println!("✓ {}. [{}] {}: \"{}\"",
                i + 1,
                string.form_id,
                string.get_string_type(),
                if string.original_text.chars().count() > 50 {
                    format!("{}...", string.original_text.chars().take(50).collect::<String>())
                } else {
                    string.original_text.clone()
                }
            );
        }
    }
    
    println!("\n翻译验证完成！");
    Ok(())
}

/// 创建真实的翻译示例（可选功能）
#[allow(dead_code)]
fn create_realistic_translations(strings: &[ExtractedString]) -> Vec<ExtractedString> {
    let mut translations = Vec::new();
    
    for string in strings.iter().take(5) {
        let mut translated = string.clone();
        
        // 根据内容类型进行不同的翻译示例
        match string.record_type.as_str() {
            "WEAP" => {
                if string.subrecord_type == "FULL" {
                    // 武器名称翻译示例
                    translated.original_text = match string.original_text.to_lowercase().as_str() {
                        name if name.contains("sword") => string.original_text.replace("Sword", "剑").replace("sword", "剑"),
                        name if name.contains("bow") => string.original_text.replace("Bow", "弓").replace("bow", "弓"),
                        name if name.contains("dagger") => string.original_text.replace("Dagger", "匕首").replace("dagger", "匕首"),
                        _ => format!("[武器] {}", string.original_text),
                    };
                }
            }
            "ARMO" => {
                if string.subrecord_type == "FULL" {
                    // 装备名称翻译示例
                    translated.original_text = match string.original_text.to_lowercase().as_str() {
                        name if name.contains("helmet") => string.original_text.replace("Helmet", "头盔").replace("helmet", "头盔"),
                        name if name.contains("armor") => string.original_text.replace("Armor", "盔甲").replace("armor", "盔甲"),
                        name if name.contains("boots") => string.original_text.replace("Boots", "靴子").replace("boots", "靴子"),
                        _ => format!("[装备] {}", string.original_text),
                    };
                }
            }
            "NPC_" => {
                if string.subrecord_type == "FULL" {
                    // NPC名称翻译示例
                    translated.original_text = format!("[NPC] {}", string.original_text);
                }
            }
            _ => {
                // 其他类型的简单翻译
                translated.original_text = format!("[已翻译] {}", string.original_text);
            }
        }
        
        translations.push(translated);
    }
    
    translations
} 