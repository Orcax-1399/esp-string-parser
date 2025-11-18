use esp_extractor::bsa::{BsaArchive, BsaStringsProvider};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== 测试 BSA 直接访问 ===\n");

    // 1. 打开 BSA
    let bsa_path = "testFile/ccbgssse002-exoticarrows.bsa";
    println!("1. 打开 BSA: {}", bsa_path);
    let bsa = BsaArchive::open(bsa_path)?;
    println!("   ✓ BSA 打开成功\n");

    // 2. 列出所有文件
    println!("2. 列出 BSA 中的所有文件:");
    let files = bsa.file_list();
    println!("   总文件数: {}", files.len());

    // 3. 查找 strings 文件
    println!("\n3. 查找 strings 文件:");
    let strings_files: Vec<_> = files.iter()
        .filter(|f| f.to_lowercase().contains("strings") ||
                    f.to_lowercase().ends_with(".strings") ||
                    f.to_lowercase().ends_with(".ilstrings") ||
                    f.to_lowercase().ends_with(".dlstrings"))
        .collect();

    if strings_files.is_empty() {
        println!("   ⚠ 未找到 strings 文件");
        println!("\n   前 20 个文件:");
        for (i, file) in files.iter().take(20).enumerate() {
            println!("   {}: {}", i + 1, file);
        }
    } else {
        println!("   找到 {} 个 strings 文件:", strings_files.len());
        for file in &strings_files {
            println!("   - {}", file);
        }
    }

    // 4. 测试 BsaStringsProvider
    println!("\n4. 测试 BsaStringsProvider:");
    let plugin_path = Path::new("testFile/ccbgssse002-exoticarrows.esl");
    match BsaStringsProvider::open_for_plugin(plugin_path) {
        Ok(provider) => {
            println!("   ✓ BsaStringsProvider 打开成功");

            let strings_list = provider.list_strings_files();
            println!("   找到 {} 个 strings 文件:", strings_list.len());
            for file in &strings_list {
                println!("   - {}", file);
            }

            // 尝试提取一个 strings 文件
            if !strings_list.is_empty() {
                println!("\n5. 尝试提取第一个 strings 文件:");
                match provider.extract_strings("ccbgssse002-exoticarrows", "english", "STRINGS") {
                    Ok(data) => {
                        println!("   ✓ 成功提取，大小: {} 字节", data.len());
                        println!("   前 100 字节: {:?}", &data[..data.len().min(100)]);
                    }
                    Err(e) => {
                        println!("   ✗ 提取失败: {}", e);
                    }
                }
            }
        }
        Err(e) => {
            println!("   ✗ BsaStringsProvider 打开失败: {}", e);
        }
    }

    Ok(())
}
