//! 性能对比测试程序
//!
//! 这个程序用于对比两种加载方式的性能差异：
//! 1. 旧路径：使用 Plugin::new() + extract_strings()
//! 2. 旧路径（带 BSA fallback）：使用 LoadedPlugin::load_auto() + extract_strings()
//! 3. 快路径：使用 fast_extract::extract_strings_fast()
//!
//! 目标：验证“流式字符串提取”快路径的性能收益，并提供对比/回退依据

use esp_extractor::{Plugin, LoadedPlugin};
use esp_extractor::fast_extract;
use std::path::PathBuf;
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=================================================");
    println!("     ESP 字符串解析器 - 性能对比测试");
    println!("=================================================\n");

    // 从命令行获取文件路径
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("用法: {} <ESP文件路径>", args[0]);
        println!("示例: {} Skyrim.esm", args[0]);
        println!("\n推荐测试文件: Skyrim.esm (~300MB，本地化插件)");
        return Ok(());
    }

    let file_path = PathBuf::from(&args[1]);

    // 检查文件是否存在
    if !file_path.exists() {
        eprintln!("❌ 错误: 文件不存在: {:?}", file_path);
        return Ok(());
    }

    println!("📁 测试文件: {:?}", file_path);

    // 获取文件大小
    let file_size = std::fs::metadata(&file_path)?.len();
    println!("📊 文件大小: {:.2} MB\n", file_size as f64 / 1024.0 / 1024.0);

    // 运行多次测试取平均值
    const TEST_ROUNDS: usize = 3;

    println!("🔬 开始性能测试 (每种方式运行 {} 次取平均值)...\n", TEST_ROUNDS);

    // ============================================================
    // 测试 1: 旧路径 (Plugin::new + extract_strings)
    // ============================================================

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📌 测试 1: 旧路径 (Plugin::new + extract_strings)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let mut cli_times = Vec::new();

    for round in 1..=TEST_ROUNDS {
        println!("\n  第 {}/{} 轮测试...", round, TEST_ROUNDS);

        let start = Instant::now();

        #[allow(deprecated)]
        let plugin = Plugin::new(file_path.clone(), None)?;
        let strings = plugin.extract_strings();

        let duration = start.elapsed();
        cli_times.push(duration);

        // 输出详细信息
        println!("    ✓ 提取完成");
        println!("    ⏱️  耗时: {:.3} 秒", duration.as_secs_f64());
        println!("    📝 插件名: {}", plugin.get_name());
        println!("    🌍 是否本地化: {}", plugin.is_localized());
        println!("    📊 字符串数量: {}", strings.len());
    }

    let cli_avg = cli_times.iter().sum::<std::time::Duration>() / cli_times.len() as u32;

    println!("\n  📈 CLI 方式统计:");
    println!("    平均耗时: {:.3} 秒", cli_avg.as_secs_f64());
    println!("    最快: {:.3} 秒", cli_times.iter().min().unwrap().as_secs_f64());
    println!("    最慢: {:.3} 秒", cli_times.iter().max().unwrap().as_secs_f64());

    // ============================================================
    // 测试 2: 旧路径（带 BSA fallback）(LoadedPlugin::load_auto + extract_strings)
    // ============================================================

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📌 测试 2: 旧路径（带 BSA fallback）(LoadedPlugin::load_auto + extract_strings)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let mut load_auto_times = Vec::new();

    for round in 1..=TEST_ROUNDS {
        println!("\n  第 {}/{} 轮测试...", round, TEST_ROUNDS);

        let start = Instant::now();

        let loaded = LoadedPlugin::load_auto(file_path.clone(), Some("english"))?;
        let strings = loaded.extract_strings();

        let duration = start.elapsed();
        load_auto_times.push(duration);

        // 输出详细信息
        let plugin = loaded.plugin();
        println!("    ✓ 提取完成");
        println!("    ⏱️  耗时: {:.3} 秒", duration.as_secs_f64());
        println!("    📝 插件名: {}", plugin.get_name());
        println!("    🌍 是否本地化: {}", plugin.is_localized());
        println!("    📊 字符串数量: {}", strings.len());
        println!("    🔤 STRING 文件: {}", if loaded.is_localized() { "已加载" } else { "未加载" });
    }

    let load_auto_avg = load_auto_times.iter().sum::<std::time::Duration>() / load_auto_times.len() as u32;

    println!("\n  📈 load_auto 方式统计:");
    println!("    平均耗时: {:.3} 秒", load_auto_avg.as_secs_f64());
    println!("    最快: {:.3} 秒", load_auto_times.iter().min().unwrap().as_secs_f64());
    println!("    最慢: {:.3} 秒", load_auto_times.iter().max().unwrap().as_secs_f64());

    // ============================================================
    // 测试 3: 快路径 (fast_extract::extract_strings_fast)
    // ============================================================

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📌 测试 3: 快路径 (fast_extract::extract_strings_fast)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let mut fast_times = Vec::new();

    for round in 1..=TEST_ROUNDS {
        println!("\n  第 {}/{} 轮测试...", round, TEST_ROUNDS);

        let start = Instant::now();

        let strings = fast_extract::extract_strings_fast(file_path.as_ref(), "english")?;

        let duration = start.elapsed();
        fast_times.push(duration);

        println!("    ✓ 提取完成");
        println!("    ⏱️  耗时: {:.3} 秒", duration.as_secs_f64());
        println!("    📊 字符串数量: {}", strings.len());
    }

    let fast_avg = fast_times.iter().sum::<std::time::Duration>() / fast_times.len() as u32;

    println!("\n  📈 快路径统计:");
    println!("    平均耗时: {:.3} 秒", fast_avg.as_secs_f64());
    println!("    最快: {:.3} 秒", fast_times.iter().min().unwrap().as_secs_f64());
    println!("    最慢: {:.3} 秒", fast_times.iter().max().unwrap().as_secs_f64());

    // ============================================================
    // 性能对比分析
    // ============================================================

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 性能对比分析");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let diff_load_auto_vs_cli = load_auto_avg.as_secs_f64() - cli_avg.as_secs_f64();
    let ratio_load_auto_vs_cli = load_auto_avg.as_secs_f64() / cli_avg.as_secs_f64();

    let diff_fast_vs_cli = fast_avg.as_secs_f64() - cli_avg.as_secs_f64();
    let ratio_fast_vs_cli = fast_avg.as_secs_f64() / cli_avg.as_secs_f64();

    let diff_fast_vs_load_auto = fast_avg.as_secs_f64() - load_auto_avg.as_secs_f64();
    let ratio_fast_vs_load_auto = fast_avg.as_secs_f64() / load_auto_avg.as_secs_f64();

    println!("  CLI 方式平均:       {:.3} 秒", cli_avg.as_secs_f64());
    println!("  load_auto 方式平均: {:.3} 秒", load_auto_avg.as_secs_f64());
    println!("  快路径平均:         {:.3} 秒", fast_avg.as_secs_f64());
    println!("  ───────────────────────────────");
    println!("  load_auto vs CLI:   {:.3} 秒 ({:+.1}%)  {:.2}x",
        diff_load_auto_vs_cli,
        (ratio_load_auto_vs_cli - 1.0) * 100.0,
        ratio_load_auto_vs_cli,
    );
    println!("  快路径 vs CLI:       {:.3} 秒 ({:+.1}%)  {:.2}x",
        diff_fast_vs_cli,
        (ratio_fast_vs_cli - 1.0) * 100.0,
        ratio_fast_vs_cli,
    );
    println!("  快路径 vs load_auto: {:.3} 秒 ({:+.1}%)  {:.2}x",
        diff_fast_vs_load_auto,
        (ratio_fast_vs_load_auto - 1.0) * 100.0,
        ratio_fast_vs_load_auto,
    );

    // 分析结果
    println!("\n🔍 分析结论:");

    if ratio_fast_vs_load_auto < 0.7 {
        println!("  ✅ 快路径显著快于旧路径（load_auto）({:.2}x)", 1.0 / ratio_fast_vs_load_auto);
    } else if ratio_fast_vs_load_auto < 0.9 {
        println!("  ⚡ 快路径快于旧路径（load_auto）({:.2}x)", 1.0 / ratio_fast_vs_load_auto);
    } else {
        println!("  ℹ️  快路径与旧路径（load_auto）性能接近 ({:.2}x)", ratio_fast_vs_load_auto);
    }

    println!("\n=================================================");
    println!("            测试完成！");
    println!("=================================================\n");

    Ok(())
}
