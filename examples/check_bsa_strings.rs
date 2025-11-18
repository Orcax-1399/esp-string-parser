use esp_extractor::bsa::BsaStringsProvider;
use esp_extractor::string_file::StringFile;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let plugin_path = Path::new("testFile/ccbgssse002-exoticarrows.esl");
    let provider = BsaStringsProvider::open_for_plugin(plugin_path)?;

    println!("=== 检查 BSA 中的 strings 文件 ===\n");

    for file_type in ["STRINGS", "ILSTRINGS", "DLSTRINGS"] {
        match provider.extract_strings("ccbgssse002-exoticarrows", "english", file_type) {
            Ok(data) => {
                println!("{}:", file_type);
                println!("  文件大小: {} 字节", data.len());

                // 解析文件
                match StringFile::from_bytes(
                    &data,
                    "ccbgssse002-exoticarrows".to_string(),
                    "english".to_string(),
                    match file_type {
                        "STRINGS" => esp_extractor::StringFileType::STRINGS,
                        "ILSTRINGS" => esp_extractor::StringFileType::ILSTRINGS,
                        "DLSTRINGS" => esp_extractor::StringFileType::DLSTRINGS,
                        _ => unreachable!(),
                    },
                ) {
                    Ok(string_file) => {
                        println!("  字符串数量: {}", string_file.count());

                        // 列出所有字符串ID
                        let mut ids = string_file.get_string_ids();
                        ids.sort();
                        println!("  字符串ID: {:?}", ids);
                    }
                    Err(e) => {
                        println!("  解析失败: {}", e);
                    }
                }
            }
            Err(e) => {
                println!("{}: 未找到 - {}", file_type, e);
            }
        }
        println!();
    }

    Ok(())
}
