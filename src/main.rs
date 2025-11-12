#[cfg(feature = "cli")]
use clap::Parser;
use std::path::PathBuf;
use esp_extractor::{Plugin, ExtractedString, SUPPORTED_EXTENSIONS};
use esp_extractor::StringFile;
use esp_extractor::group::{Group, GroupChild};

#[cfg(debug_assertions)]
use esp_extractor::EspDebugger;

#[cfg(feature = "cli")]
#[derive(Parser)]
#[command(name = "esp_extractor")]
#[command(about = "从ESP/ESM/ESL文件中提取可翻译字符串，或解析Bethesda字符串文件")]
#[command(version = "0.2.0")]
struct Cli {
    /// 输入文件路径（ESP/ESM/ESL或字符串文件）
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
    
    /// 应用翻译：从JSON文件应用翻译到ESP文件
    #[arg(long)]
    apply_file: Option<PathBuf>,
    
    /// 应用翻译：从JSON字符串应用指定的翻译对象
    #[arg(long)]
    apply_jsonstr: Option<String>,
    
    /// 应用翻译：从标准输入读取JSON翻译对象
    #[arg(long)]
    apply_partial_stdin: bool,
    
    /// 测试模式：解析文件后直接重建，用于验证解析和重建逻辑
    #[arg(long)]
    test_rebuild: bool,
    
    /// 对比两个ESP文件的结构差异
    #[arg(long)]
    compare_files: Option<PathBuf>,
    
    /// 字符串文件操作：解析字符串文件并输出JSON
    #[arg(long)]
    parse_strings: Option<PathBuf>,
}

#[cfg(feature = "cli")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    
    validate_input(&cli.input)?;
    validate_partial_options(&cli)?;
    
    // 处理不同的操作模式
    if cli.test_rebuild {
        return handle_test_rebuild(&cli);
    }
    
    if let Some(compare_file) = &cli.compare_files {
        return handle_file_comparison(&cli, compare_file);
    }
    
    if let Some(string_file) = &cli.parse_strings {
        return handle_string_file_parsing(&cli, string_file);
    }
    
    if cli.apply_partial_stdin {
        return handle_translation_stdin(&cli);
    }
    
    if let Some(translation_file) = &cli.apply_file {
        return handle_translation_file(&cli, translation_file);
    }
    
    if let Some(translation_json) = &cli.apply_jsonstr {
        return handle_translation_jsonstr(&cli, translation_json);
    }
    
    // 默认模式：根据文件类型自动选择处理方式
    let extension = cli.input.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_lowercase());
    
    let string_extensions = ["strings", "ilstrings", "dlstrings"];
    if string_extensions.iter().any(|&ext| Some(ext) == extension.as_deref()) {
        // 字符串文件：解析并输出JSON
        handle_string_file_parsing(&cli, &cli.input)
    } else {
        // ESP文件：字符串提取
        handle_string_extraction(&cli)
    }
}

/// 验证输入文件
fn validate_input(input: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    if !input.exists() {
        return Err(format!("输入文件不存在: {:?}", input).into());
    }
    
    let extension = input.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_lowercase());
    
    // 支持ESP/ESM/ESL文件和字符串文件
    let string_extensions = ["strings", "ilstrings", "dlstrings"];
    let is_esp_file = SUPPORTED_EXTENSIONS.iter().any(|&ext| Some(ext) == extension.as_deref());
    let is_string_file = string_extensions.iter().any(|&ext| Some(ext) == extension.as_deref());
    
    if !is_esp_file && !is_string_file {
        return Err("输入文件必须是ESP、ESM、ESL或字符串文件（STRINGS、ILSTRINGS、DLSTRINGS）".into());
    }
    
    Ok(())
}

/// 验证翻译选项（确保只使用一种方式）
fn validate_partial_options(cli: &Cli) -> Result<(), Box<dyn std::error::Error>> {
    let translation_count = [
        cli.apply_jsonstr.is_some(),
        cli.apply_file.is_some(),
        cli.apply_partial_stdin,
    ].iter().filter(|&&x| x).count();
    
    if translation_count > 1 {
        return Err("只能使用一种翻译方式：--apply-jsonstr、--apply-file 或 --apply-partial-stdin".into());
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

/// 处理翻译应用（从字符串）
fn handle_translation_jsonstr(cli: &Cli, translation_json: &str) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(debug_assertions)]
    if !cli.quiet {
        println!("正在应用翻译到: {:?} (从命令行参数)", cli.input);
    }
    
    let translations = parse_translation_json(translation_json)?;
    apply_translations(cli, translations)
}

/// 处理翻译应用（从文件）
fn handle_translation_file(cli: &Cli, translation_file: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    if !translation_file.exists() {
        return Err(format!("翻译文件不存在: {:?}", translation_file).into());
    }
    
    #[cfg(debug_assertions)]
    if !cli.quiet {
        println!("正在应用翻译到: {:?} (从文件: {:?})", cli.input, translation_file);
    }
    
    let translation_json = std::fs::read_to_string(translation_file)
        .map_err(|e| format!("读取翻译文件失败: {}", e))?;
    
    let translations = parse_translation_json(&translation_json)?;
    apply_translations(cli, translations)
}

/// 处理翻译应用（从标准输入）
fn handle_translation_stdin(cli: &Cli) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(debug_assertions)]
    if !cli.quiet {
        println!("正在应用翻译到: {:?} (从标准输入)", cli.input);
    }
    
    if !cli.quiet {
        eprintln!("等待从标准输入读取JSON数据... (Ctrl+D结束输入)");
    }
    
    use std::io::Read;
    let mut buffer = String::new();
    std::io::stdin().read_to_string(&mut buffer)
        .map_err(|e| format!("从标准输入读取失败: {}", e))?;
    
    let translations = parse_translation_json(&buffer)?;
    apply_translations(cli, translations)
}

/// 解析翻译JSON
fn parse_translation_json(json_str: &str) -> Result<Vec<ExtractedString>, Box<dyn std::error::Error>> {
    serde_json::from_str(json_str)
        .map_err(|e| format!("解析翻译JSON失败: {}", e).into())
}

/// 应用翻译
fn apply_translations(cli: &Cli, translations: Vec<ExtractedString>) -> Result<(), Box<dyn std::error::Error>> {
    if translations.is_empty() {
        return Err("翻译数据为空".into());
    }
    
    if !cli.quiet {
        println!("准备应用 {} 个翻译条目", translations.len());
        
        // 显示前3个翻译条目的详细信息
        for (i, translation) in translations.iter().take(3).enumerate() {
            println!("翻译条目 {}: [{}] {} -> \"{}\"", 
                i + 1,
                translation.form_id,
                translation.get_string_type(),
                if translation.original_text.chars().count() > 50 {
                    format!("{}...", translation.original_text.chars().take(50).collect::<String>())
                } else {
                    translation.original_text.clone()
                }
            );
        }
        if translations.len() > 3 {
            println!("... 还有 {} 个翻译条目", translations.len() - 3);
        }
    }
    
    let output_path = get_apply_output_path(cli);
    Plugin::apply_translations(cli.input.clone(), output_path.clone(), translations, None)
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
    
    let plugin = Plugin::new(cli.input.clone(), None)
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

/// 将字符串保存到文件
fn save_strings_to_file(strings: &[ExtractedString], output_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let json_output = serde_json::to_string_pretty(strings)
        .map_err(|e| format!("序列化JSON失败: {}", e))?;
    
    std::fs::write(output_path, &json_output)
        .map_err(|e| format!("写入文件失败: {}", e).into())
}

/// 打印提取摘要信息
fn print_extraction_summary(_plugin: &Plugin, strings: &[ExtractedString], output_path: &PathBuf) {
    #[cfg(debug_assertions)]
    let stats = _plugin.get_stats();
    
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
                string.get_string_type(), 
                if string.original_text.chars().count() > 50 {
                    format!("{}...", string.original_text.chars().take(50).collect::<String>())
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

/// 获取应用翻译输出路径
fn get_apply_output_path(cli: &Cli) -> PathBuf {
    cli.output.clone().unwrap_or_else(|| {
        let mut output = cli.input.clone();
        let stem = output.file_stem().unwrap().to_str().unwrap();
        let extension = output.extension().unwrap().to_str().unwrap();
        output.set_file_name(format!("{}.{}", stem, extension));
        output
    })
}

/// 测试文件重建功能
fn test_rebuild_file(input_path: PathBuf, output_path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let plugin = Plugin::new(input_path.clone(), None)?;
    
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
    let rebuilt_plugin = Plugin::new(output_path.clone(), None)?;
    
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
fn compare_file_sizes(_input_path: &PathBuf, _output_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(debug_assertions)]
    let original_size = std::fs::metadata(_input_path)?.len();
    #[cfg(debug_assertions)]
    let rebuilt_size = std::fs::metadata(_output_path)?.len();
    
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

/// 处理文件对比
fn handle_file_comparison(cli: &Cli, compare_file: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    if !compare_file.exists() {
        return Err(format!("对比文件不存在: {:?}", compare_file).into());
    }
    
    if !cli.quiet {
        println!("正在对比文件结构:");
        println!("  文件1: {:?}", cli.input);
        println!("  文件2: {:?}", compare_file);
    }
    
    let plugin1 = Plugin::new(cli.input.clone(), None)?;
    let plugin2 = Plugin::new(compare_file.clone(), None)?;
    
    // 对比基本信息
    println!("\n=== 基本信息对比 ===");
    println!("组数量: {} vs {}", plugin1.groups.len(), plugin2.groups.len());
    
    if plugin1.groups.len() != plugin2.groups.len() {
        println!("⚠️ 组数量不匹配！");
        return Ok(());
    }
    
    // 对比每个GRUP的大小
    println!("\n=== GRUP大小对比 ===");
    for (i, (group1, group2)) in plugin1.groups.iter().zip(plugin2.groups.iter()).enumerate() {
        let label1 = String::from_utf8_lossy(&group1.label);
        let label2 = String::from_utf8_lossy(&group2.label);
        
        if group1.size != group2.size {
            println!("⚠️ GRUP {} ('{}' vs '{}'): {} vs {} (差异: {})", 
                i, label1, label2, group1.size, group2.size, 
                (group2.size as i64) - (group1.size as i64));
                
            // 详细分析这个组的差异
            analyze_group_difference(group1, group2, i)?;
        } else {
            println!("✓ GRUP {} ('{}'): {} 字节 - 匹配", i, label1, group1.size);
        }
    }
    
    Ok(())
}

/// 分析组差异的详细原因
fn analyze_group_difference(group1: &Group, group2: &Group, group_index: usize) -> Result<(), Box<dyn std::error::Error>> {
    println!("  详细分析GRUP {}:", group_index);
    println!("    子元素数量: {} vs {}", group1.children.len(), group2.children.len());
    
    if group1.children.len() != group2.children.len() {
        println!("    ⚠️ 子元素数量不匹配！");
        return Ok(());
    }
    
    let mut total_diff = 0i64;
    
    for (i, (child1, child2)) in group1.children.iter().zip(group2.children.iter()).enumerate() {
        match (child1, child2) {
            (GroupChild::Record(r1), GroupChild::Record(r2)) => {
                if r1.data_size != r2.data_size {
                    let diff = (r2.data_size as i64) - (r1.data_size as i64);
                    total_diff += diff;
                    println!("    记录 {} ({}): {} vs {} (差异: {})", 
                        i, r1.record_type, r1.data_size, r2.data_size, diff);
                }
            }
            (GroupChild::Group(g1), GroupChild::Group(g2)) => {
                if g1.size != g2.size {
                    let diff = (g2.size as i64) - (g1.size as i64);
                    total_diff += diff;
                    println!("    子GRUP {} ('{}'): {} vs {} (差异: {})", 
                        i, String::from_utf8_lossy(&g1.label), g1.size, g2.size, diff);
                }
            }
            _ => {
                println!("    ⚠️ 子元素 {} 类型不匹配！", i);
            }
        }
    }
    
    let grup_diff = (group2.size as i64) - (group1.size as i64);
    println!("    计算的总差异: {} 字节", total_diff);
    println!("    实际GRUP差异: {} 字节", grup_diff);
    
    if total_diff != grup_diff {
        println!("    ⚠️ 差异不匹配！可能存在其他问题");
    }
    
    Ok(())
}

/// 处理字符串文件解析
fn handle_string_file_parsing(cli: &Cli, string_file_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    if !string_file_path.exists() {
        return Err(format!("字符串文件不存在: {:?}", string_file_path).into());
    }
    
    if !cli.quiet {
        println!("正在解析字符串文件: {:?}", string_file_path);
    }
    
    let string_file = StringFile::new(string_file_path.clone())?;
    
    if cli.stats {
        println!("{}", string_file.get_stats());
        return Ok(());
    }
    
    // 将字符串转换为JSON格式输出
    let entries: Vec<_> = string_file.entries.values().collect();
    let json_output = serde_json::to_string_pretty(&entries)
        .map_err(|e| format!("序列化JSON失败: {}", e))?;
    
    let output_path = cli.output.as_ref()
        .map(|p| p.clone())
        .unwrap_or_else(|| string_file_path.with_extension("json"));
    
    std::fs::write(&output_path, &json_output)
        .map_err(|e| format!("写入文件失败: {}", e))?;
    
    if !cli.quiet {
        println!("解析完成:");
        println!("  插件名: {}", string_file.plugin_name);
        println!("  语言: {}", string_file.language);
        println!("  文件类型: {:?}", string_file.file_type);
        println!("  字符串数量: {}", string_file.count());
        println!("  结果已写入: {:?}", output_path);
        
        // 显示前几个字符串样例
        let sample_entries: Vec<_> = string_file.entries.values().take(3).collect();
        if !sample_entries.is_empty() {
            println!("\n样例字符串:");
            for (i, entry) in sample_entries.iter().enumerate() {
                println!("{}. ID {}: \"{}\"", 
                    i + 1, 
                    entry.id,
                    if entry.content.chars().count() > 50 {
                        format!("{}...", entry.content.chars().take(50).collect::<String>())
                    } else {
                        entry.content.clone()
                    }
                );
            }
        }
    }
    
    Ok(())
}



#[cfg(not(feature = "cli"))]
fn main() {
    eprintln!("命令行工具功能未启用。请使用 --features cli 编译，或将此库用作依赖项。");
    std::process::exit(1);
}
