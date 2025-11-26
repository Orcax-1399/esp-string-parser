//! 调试 StringFileSet::load_from_bsa 失败原因

use esp_extractor::StringFileSet;
use std::path::PathBuf;

#[test]
fn debug_load_from_bsa() {
    let esp_path = PathBuf::from(r"E:\Bethesda\Skyrim\WBV4\iniRePather-Skyrim\Mod Organizer 2\Stock Game\Data\Skyrim.esm");
    let plugin_name = "Skyrim";
    let language = "english";

    println!("正在调用 StringFileSet::load_from_bsa");
    println!("  插件路径: {}", esp_path.display());
    println!("  插件名称: {}", plugin_name);
    println!("  语言: {}", language);
    println!();

    match StringFileSet::load_from_bsa(&esp_path, plugin_name, language) {
        Ok(set) => {
            println!("✅ 成功加载 STRING 文件集合");
            println!("  总字符串数: {}", set.total_count());

            use esp_extractor::StringFileType;
            for file_type in [StringFileType::STRINGS, StringFileType::ILSTRINGS, StringFileType::DLSTRINGS] {
                if let Some(file) = set.get_file(&file_type) {
                    println!("  - {:?}: {} 条", file_type, file.count());
                }
            }
        }
        Err(e) => {
            println!("❌ 加载失败: {}", e);
            panic!("load_from_bsa 失败");
        }
    }
}
