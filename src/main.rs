use clap::Parser;
use std::path::PathBuf;
use esp_extractor::{Plugin, ExtractedString, EspDebugger, SUPPORTED_EXTENSIONS};

#[derive(Parser)]
#[command(name = "esp_extractor")]
#[command(about = "从ESP/ESM/ESL文件中提取可翻译字符串")]
#[command(version = "0.1.0")]
struct Cli {
    /// 输入ESP/ESM/ESL文件路径
    #[arg(short, long)]
    input: PathBuf,
    
    /// 输出JSON文件路径
    #[arg(short, long)]
    output: Option<PathBuf>,
    
    /// 包含本地化字符串(通过ID)
    #[arg(long)]
    include_localized: bool,
    
    /// 包含所有字符串(跳过验证)
    #[arg(long)]
    unfiltered: bool,
    
    /// 显示插件统计信息
    #[arg(long)]
    stats: bool,
    
    /// 静默模式(仅输出错误)
    #[arg(long)]
    quiet: bool,
    
    /// 应用翻译模式：从翻译JSON文件应用翻译到ESP文件
    #[arg(long)]
    apply_translations: Option<PathBuf>,
    
    /// 应用部分翻译：从JSON字符串应用指定的翻译对象
    #[arg(long)]
    apply_partial: Option<String>,
    
    /// 测试模式：解析文件后直接重建，用于验证解析和重建逻辑
    #[arg(long)]
    test_rebuild: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    
    validate_input(&cli.input)?;
    
    // 处理不同的操作模式
    if cli.test_rebuild {
        return handle_test_rebuild(&cli);
    }
    
    if let Some(partial_json) = &cli.apply_partial {
        return handle_partial_translation(&cli, partial_json);
    }
    
    if let Some(translation_file) = &cli.apply_translations {
        return handle_translation_application(&cli, translation_file);
    }
    
    // 默认模式：字符串提取
    handle_string_extraction(&cli)
}

/// 验证输入文件
fn validate_input(input: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    if !input.exists() {
        return Err(format!("输入文件不存在: {:?}", input).into());
    }
    
    let extension = input.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_lowercase());
    
    if !SUPPORTED_EXTENSIONS.iter().any(|&ext| Some(ext) == extension.as_deref()) {
        return Err("输入文件必须是ESP、ESM或ESL文件".into());
    }
    
    Ok(())
}

/// 处理测试重建模式
fn handle_test_rebuild(cli: &Cli) -> Result<(), Box<dyn std::error::Error>> {
    if !cli.quiet {
        println!("测试模式：解析并重建文件 {:?}", cli.input);
    }
    
    let output_path = get_rebuild_output_path(cli);
    test_rebuild_file(cli.input.clone(), output_path.clone())?;
    
    if !cli.quiet {
        println!("测试完成，重建文件输出到: {:?}", output_path);
        println!("请使用文件对比工具检查原文件和重建文件是否一致");
    }
    
    Ok(())
}

/// 处理部分翻译应用
fn handle_partial_translation(cli: &Cli, partial_json: &str) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(debug_assertions)]
    if !cli.quiet {
        println!("正在应用部分翻译到: {:?}", cli.input);
    }
    
    let translations: Vec<ExtractedString> = serde_json::from_str(partial_json)
        .map_err(|e| format!("解析部分翻译JSON失败: {}", e))?;
    
    let output_path = get_partial_output_path(cli);
    Plugin::apply_translations(cli.input.clone(), output_path.clone(), translations)
        .map_err(|e| format!("应用部分翻译失败: {}", e))?;
    
    if !cli.quiet {
        println!("部分翻译应用完成，输出到: {:?}", output_path);
    }
    
    Ok(())
}

/// 处理翻译文件应用
fn handle_translation_application(cli: &Cli, translation_file: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    if !translation_file.exists() {
        return Err(format!("翻译文件不存在: {:?}", translation_file).into());
    }
    
    #[cfg(debug_assertions)]
    if !cli.quiet {
        println!("正在应用翻译: {:?} -> {:?}", translation_file, cli.input);
    }
    
    let translations = load_translations(translation_file)?;
    let output_path = get_translation_output_path(cli);
    
    Plugin::apply_translations(cli.input.clone(), output_path.clone(), translations)
        .map_err(|e| format!("应用翻译失败: {}", e))?;
    
    if !cli.quiet {
        println!("翻译应用完成，输出到: {:?}", output_path);
    }
    
    Ok(())
}

/// 处理字符串提取
fn handle_string_extraction(cli: &Cli) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(debug_assertions)]
    if !cli.quiet {
        println!("正在解析插件: {:?}", cli.input);
    }
    
    let plugin = Plugin::new(cli.input.clone())
        .map_err(|e| format!("解析插件失败: {}", e))?;
    
    if cli.stats {
        println!("{}", plugin.get_stats());
        return Ok(());
    }
    
    let strings = plugin.extract_strings();
    let output_path = cli.output.as_ref()
        .map(|p| p.clone())
        .unwrap_or_else(|| cli.input.with_extension("json"));
    
    save_strings_to_file(&strings, &output_path)?;
    
    if !cli.quiet {
        print_extraction_summary(&plugin, &strings, &output_path);
    }
    
    Ok(())
}

/// 加载翻译文件
fn load_translations(translation_file: &PathBuf) -> Result<Vec<ExtractedString>, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(translation_file)
        .map_err(|e| format!("读取翻译文件失败: {}", e))?;
    
    serde_json::from_str(&content)
        .map_err(|e| format!("解析翻译文件失败: {}", e).into())
}

/// 将字符串保存到文件
fn save_strings_to_file(strings: &[ExtractedString], output_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let json_output = serde_json::to_string_pretty(strings)
        .map_err(|e| format!("序列化JSON失败: {}", e))?;
    
    std::fs::write(output_path, &json_output)
        .map_err(|e| format!("写入文件失败: {}", e).into())
}

/// 打印提取摘要信息
fn print_extraction_summary(plugin: &Plugin, strings: &[ExtractedString], output_path: &PathBuf) {
    let stats = plugin.get_stats();
    
    #[cfg(debug_assertions)]
    {
        println!("扫描到 {} 个组（包含子组）", stats.group_count);
        println!("扫描到 {} 个记录", stats.record_count);
    }
    
    println!("提取到 {} 个有效字符串", strings.len());
    println!("结果已写入: {:?}", output_path);
    
    // 显示样例字符串
    if !strings.is_empty() {
        println!("\n样例字符串:");
        for (i, string) in strings.iter().take(3).enumerate() {
            println!("{}. [{}] {}: \"{}\"", 
                i + 1, 
                string.form_id, 
                string.string_type, 
                if string.original_text.len() > 50 {
                    format!("{}...", &string.original_text[..50])
                } else {
                    string.original_text.clone()
                }
            );
        }
        
        if strings.len() > 3 {
            println!("... 还有 {} 个字符串", strings.len() - 3);
        }
    }
}

/// 获取重建输出路径
fn get_rebuild_output_path(cli: &Cli) -> PathBuf {
    cli.output.clone().unwrap_or_else(|| {
        let mut output = cli.input.clone();
        let stem = output.file_stem().unwrap().to_str().unwrap();
        let extension = output.extension().unwrap().to_str().unwrap();
        output.set_file_name(format!("{}_rebuilt.{}", stem, extension));
        output
    })
}

/// 获取部分翻译输出路径
fn get_partial_output_path(cli: &Cli) -> PathBuf {
    cli.output.clone().unwrap_or_else(|| {
        let mut output = cli.input.clone();
        let stem = output.file_stem().unwrap().to_str().unwrap();
        let extension = output.extension().unwrap().to_str().unwrap();
        output.set_file_name(format!("{}.{}", stem, extension));
        output
    })
}

/// 获取翻译输出路径
fn get_translation_output_path(cli: &Cli) -> PathBuf {
    cli.output.clone().unwrap_or_else(|| {
        let mut output = cli.input.clone();
        let stem = output.file_stem().unwrap().to_str().unwrap();
        let extension = output.extension().unwrap().to_str().unwrap();
        output.set_file_name(format!("{}_translated.{}", stem, extension));
        output
    })
}

/// 测试文件重建功能
fn test_rebuild_file(input_path: PathBuf, output_path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let plugin = Plugin::new(input_path.clone())?;
    
    #[cfg(debug_assertions)]
    {
        println!("解析完成:");
        println!("  插件名: {}", plugin.get_name());
        println!("  插件类型: {}", plugin.get_type());
        println!("  组数量: {}", plugin.groups.len());
    }
    
    // 生成调试信息（仅在debug模式下）
    generate_debug_info(&plugin, &input_path, &output_path)?;
    
    // 重建文件
    plugin.write_to_file(output_path.clone())?;
    
    // 文件大小对比
    compare_file_sizes(&input_path, &output_path)?;
    
    Ok(())
}

/// 生成调试信息
#[cfg(debug_assertions)]
fn generate_debug_info(plugin: &Plugin, input_path: &PathBuf, output_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let original_dump_path = input_path.with_extension("original.dump");
    println!("生成原始文件结构dump: {:?}", original_dump_path);
    EspDebugger::dump_file_structure(plugin, original_dump_path)?;
    
    // 解析重建文件并生成dump
    plugin.write_to_file(output_path.clone())?;
    let rebuilt_plugin = Plugin::new(output_path.clone())?;
    
    let rebuilt_dump_path = output_path.with_extension("rebuilt.dump");
    println!("生成重建文件结构dump: {:?}", rebuilt_dump_path);
    EspDebugger::dump_file_structure(&rebuilt_plugin, rebuilt_dump_path)?;
    
    let compare_path = input_path.with_extension("compare.txt");
    println!("生成结构对比报告: {:?}", compare_path);
    EspDebugger::compare_structures(input_path.clone(), output_path.clone(), compare_path)?;
    
    println!();
    println!("调试文件已生成:");
    println!("  - 原始文件结构: {:?}", input_path.with_extension("original.dump"));
    println!("  - 重建文件结构: {:?}", output_path.with_extension("rebuilt.dump"));
    println!("  - 结构对比报告: {:?}", input_path.with_extension("compare.txt"));
    println!();
    println!("请检查这些dump文件来诊断重建问题！");
    
    Ok(())
}

#[cfg(not(debug_assertions))]
fn generate_debug_info(_plugin: &Plugin, _input_path: &PathBuf, _output_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

/// 比较文件大小
fn compare_file_sizes(input_path: &PathBuf, output_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let original_size = std::fs::metadata(input_path)?.len();
    let rebuilt_size = std::fs::metadata(output_path)?.len();
    
    #[cfg(debug_assertions)]
    {
        println!("文件大小对比:");
        println!("  原文件: {} 字节", original_size);
        println!("  重建文件: {} 字节", rebuilt_size);
        
        if original_size == rebuilt_size {
            println!("✓ 文件大小一致");
        } else {
            println!("⚠ 文件大小不一致，差异: {} 字节", (rebuilt_size as i64) - (original_size as i64));
        }
    }
    
    Ok(())
}
