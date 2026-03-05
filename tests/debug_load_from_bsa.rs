//! 调试 StringFileSet::load_from_bsa 失败原因

use esp_extractor::StringFileSet;
use std::path::PathBuf;

#[test]
fn debug_load_from_bsa() {
    // 这个测试用于本地调试，默认使用仓库内的测试素材。
    // 如果你想指向自己的 Skyrim 安装路径，可设置环境变量：
    // - ESP_DEBUG_ESP_PATH: 例如 E:\...\Data\Skyrim.esm
    // - ESP_DEBUG_LANGUAGE: 例如 english
    let esp_path = std::env::var("ESP_DEBUG_ESP_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("TestFile/Skyrim.esm"));

    if !esp_path.exists() {
        println!("⚠️ 跳过测试：ESP 文件不存在：{}", esp_path.display());
        return;
    }

    let plugin_name = esp_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Skyrim");

    let language = std::env::var("ESP_DEBUG_LANGUAGE").unwrap_or_else(|_| "english".to_string());

    if let Some(data_dir) = esp_path.parent() {
        let bsa_path = data_dir.join("Skyrim - Interface.bsa");
        if !bsa_path.exists() {
            println!("⚠️ 跳过测试：BSA 文件不存在：{}", bsa_path.display());
            return;
        }
    }

    println!("正在调用 StringFileSet::load_from_bsa");
    println!("  插件路径: {}", esp_path.display());
    println!("  插件名称: {}", plugin_name);
    println!("  语言: {}", language);
    println!();

    match StringFileSet::load_from_bsa(&esp_path, plugin_name, &language) {
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
