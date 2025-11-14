//! 性能对比测试程序
//!
//! 这个程序用于对比两种加载方式的性能差异：
//! 1. CLI 方式：使用 Plugin::new() (已弃用但仍在使用)
//! 2. load_auto 方式：使用 LoadedPlugin::load_auto()
//!
//! 目标：验证并修复 load_auto 对本地化插件重复加载 ESP 文件的问题

use esp_extractor::{Plugin, LoadedPlugin};
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
    // 测试 1: CLI 方式 (Plugin::new)
    // ============================================================

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📌 测试 1: CLI 方式 (Plugin::new)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let mut cli_times = Vec::new();

    for round in 1..=TEST_ROUNDS {
        println!("\n  第 {}/{} 轮测试...", round, TEST_ROUNDS);

        let start = Instant::now();

        #[allow(deprecated)]
        let plugin = Plugin::new(file_path.clone(), None)?;

        let duration = start.elapsed();
        cli_times.push(duration);

        // 输出详细信息
        println!("    ✓ 加载完成");
        println!("    ⏱️  耗时: {:.3} 秒", duration.as_secs_f64());
        println!("    📝 插件名: {}", plugin.get_name());
        println!("    🌍 是否本地化: {}", plugin.is_localized());
        println!("    📊 字符串数量: {}", plugin.extract_strings().len());
    }

    let cli_avg = cli_times.iter().sum::<std::time::Duration>() / cli_times.len() as u32;

    println!("\n  📈 CLI 方式统计:");
    println!("    平均耗时: {:.3} 秒", cli_avg.as_secs_f64());
    println!("    最快: {:.3} 秒", cli_times.iter().min().unwrap().as_secs_f64());
    println!("    最慢: {:.3} 秒", cli_times.iter().max().unwrap().as_secs_f64());

    // ============================================================
    // 测试 2: load_auto 方式
    // ============================================================

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📌 测试 2: load_auto 方式 (LoadedPlugin::load_auto)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let mut load_auto_times = Vec::new();

    for round in 1..=TEST_ROUNDS {
        println!("\n  第 {}/{} 轮测试...", round, TEST_ROUNDS);

        let start = Instant::now();

        let loaded = LoadedPlugin::load_auto(file_path.clone(), Some("english"))?;

        let duration = start.elapsed();
        load_auto_times.push(duration);

        // 输出详细信息
        let plugin = loaded.plugin();
        println!("    ✓ 加载完成");
        println!("    ⏱️  耗时: {:.3} 秒", duration.as_secs_f64());
        println!("    📝 插件名: {}", plugin.get_name());
        println!("    🌍 是否本地化: {}", plugin.is_localized());
        println!("    📊 字符串数量: {}", plugin.extract_strings().len());
        println!("    🔤 STRING 文件: {}", if loaded.is_localized() { "已加载" } else { "未加载" });
    }

    let load_auto_avg = load_auto_times.iter().sum::<std::time::Duration>() / load_auto_times.len() as u32;

    println!("\n  📈 load_auto 方式统计:");
    println!("    平均耗时: {:.3} 秒", load_auto_avg.as_secs_f64());
    println!("    最快: {:.3} 秒", load_auto_times.iter().min().unwrap().as_secs_f64());
    println!("    最慢: {:.3} 秒", load_auto_times.iter().max().unwrap().as_secs_f64());

    // ============================================================
    // 性能对比分析
    // ============================================================

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 性能对比分析");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let difference = load_auto_avg.as_secs_f64() - cli_avg.as_secs_f64();
    let ratio = load_auto_avg.as_secs_f64() / cli_avg.as_secs_f64();

    println!("  CLI 方式平均:       {:.3} 秒", cli_avg.as_secs_f64());
    println!("  load_auto 方式平均: {:.3} 秒", load_auto_avg.as_secs_f64());
    println!("  ───────────────────────────────");
    println!("  性能差异:           {:.3} 秒 ({:+.1}%)",
        difference,
        (ratio - 1.0) * 100.0
    );
    println!("  速度比率:           {:.2}x", ratio);

    // 分析结果
    println!("\n🔍 分析结论:");

    if ratio > 1.8 {
        println!("  ⚠️  load_auto 显著慢于 CLI 方式 ({:.2}x)！", ratio);
        println!("  💡 原因分析:");
        println!("     - load_auto 可能对本地化插件重复加载 ESP 文件");
        println!("     - 第一次加载用于检查 is_localized()");
        println!("     - 第二次加载在 LocalizedPluginContext::load() 中");
        println!("  ✅ 需要优化: 复用第一次加载的 Plugin 实例");
    } else if ratio > 1.2 {
        println!("  ⚡ load_auto 略慢于 CLI 方式 ({:.2}x)", ratio);
        println!("  💡 这可能是由于额外的 STRING 文件加载开销");
        println!("  ℹ️  性能差异在可接受范围内");
    } else {
        println!("  ✅ 两种方式性能相近 ({:.2}x)", ratio);
        println!("  🎉 load_auto 已优化，无重复加载问题！");
    }

    println!("\n=================================================");
    println!("            测试完成！");
    println!("=================================================\n");

    Ok(())
}
