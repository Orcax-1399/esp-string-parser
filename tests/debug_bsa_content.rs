//! 调试 BSA 内容，查看实际文件列表

use esp_extractor::bsa::BsaArchive;
use std::path::PathBuf;

#[test]
fn debug_skyrim_bsa_content() {
    let bsa_path = PathBuf::from(r"E:\Bethesda\Skyrim\WBV4\iniRePather-Skyrim\Mod Organizer 2\Stock Game\Data\Skyrim - Interface.bsa");

    if !bsa_path.exists() {
        println!("⚠️ BSA 文件不存在：{}", bsa_path.display());
        return;
    }

    println!("正在打开 BSA: {}", bsa_path.display());

    let archive = match BsaArchive::open(&bsa_path) {
        Ok(a) => a,
        Err(e) => {
            println!("❌ 无法打开 BSA: {}", e);
            return;
        }
    };

    println!("✓ BSA 打开成功\n");

    // 获取所有文件列表
    let file_list = archive.file_list();
    println!("BSA 中总文件数: {}\n", file_list.len());

    // 查找所有 strings 相关的文件
    println!("=== 查找 strings 相关文件 ===");
    let strings_files: Vec<&String> = file_list.iter()
        .filter(|path| path.to_lowercase().contains("string"))
        .collect();

    if strings_files.is_empty() {
        println!("❌ 未找到任何包含 'string' 的文件");
    } else {
        println!("找到 {} 个 strings 相关文件:", strings_files.len());
        for (i, file) in strings_files.iter().enumerate() {
            println!("  {}. {}", i + 1, file);
        }
    }

    // 显示前50个文件，了解BSA的结构
    println!("\n=== BSA 文件结构（前50个文件）===");
    for (i, file) in file_list.iter().take(50).enumerate() {
        println!("  {}. {}", i + 1, file);
    }

    // 尝试提取一个strings文件
    println!("\n=== 尝试提取 strings 文件 ===");
    let test_paths = vec![
        "strings/skyrim_english.strings",
        "Strings/Skyrim_english.STRINGS",
        "strings/Skyrim_english.STRINGS",
        "Strings/skyrim_english.strings",
        "interface/strings/skyrim_english.strings",
        "Interface/Strings/Skyrim_english.STRINGS",
    ];

    for path in test_paths {
        match archive.extract(path) {
            Ok(data) => {
                println!("✓ 成功提取: {} ({} bytes)", path, data.len());
            }
            Err(e) => {
                println!("  ✗ 失败: {} - {}", path, e);
            }
        }
    }
}

#[test]
fn debug_bsa_strings_provider() {
    use esp_extractor::bsa::BsaStringsProvider;

    let esp_path = PathBuf::from(r"E:\Bethesda\Skyrim\WBV4\iniRePather-Skyrim\Mod Organizer 2\Stock Game\Data\Skyrim.esm");

    println!("正在为插件打开 BSA: {}", esp_path.display());

    let provider = match BsaStringsProvider::open_for_plugin(&esp_path) {
        Ok(p) => {
            println!("✓ BsaStringsProvider 创建成功");
            p
        }
        Err(e) => {
            println!("❌ 无法创建 BsaStringsProvider: {}", e);
            return;
        }
    };

    // 尝试提取
    let plugin_name = "Skyrim";
    let language = "english";

    println!("\n尝试提取 {}_{}.STRINGS", plugin_name, language);
    match provider.extract_strings(plugin_name, language, "STRINGS") {
        Ok(data) => println!("✓ 成功提取 STRINGS: {} bytes", data.len()),
        Err(e) => println!("❌ 失败: {}", e),
    }

    println!("\n尝试提取 {}_{}.ILSTRINGS", plugin_name, language);
    match provider.extract_strings(plugin_name, language, "ILSTRINGS") {
        Ok(data) => println!("✓ 成功提取 ILSTRINGS: {} bytes", data.len()),
        Err(e) => println!("❌ 失败: {}", e),
    }

    println!("\n尝试提取 {}_{}.DLSTRINGS", plugin_name, language);
    match provider.extract_strings(plugin_name, language, "DLSTRINGS") {
        Ok(data) => println!("✓ 成功提取 DLSTRINGS: {} bytes", data.len()),
        Err(e) => println!("❌ 失败: {}", e),
    }
}
