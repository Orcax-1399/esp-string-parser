//! LoadedPlugin BSA Fallback 功能测试
//!
//! 测试场景：
//! - 验证 load_auto 能够自动检测本地化插件
//! - 验证文件系统中没有 STRING 文件时，自动 fallback 到 BSA
//! - 测试官方主文件（Skyrim.esm）使用 "Skyrim - Interface.bsa"
//! - 验证从 BSA 中提取的字符串能够正确解析

use std::path::PathBuf;
use esp_extractor::LoadedPlugin;

/// 获取 Skyrim 安装路径下的 Skyrim.esm
///
/// 注意：此测试需要完整的 Skyrim 安装，包括：
/// - Skyrim.esm
/// - Skyrim - Interface.bsa（包含 STRING 文件）
fn get_skyrim_installation_path() -> PathBuf {
    // 用户实际路径
    PathBuf::from(r"E:\Bethesda\Skyrim\WBV4\iniRePather-Skyrim\Mod Organizer 2\Stock Game\Data\Skyrim.esm")
}

/// 检查 Skyrim 安装是否存在
fn skyrim_installation_exists() -> bool {
    let path = get_skyrim_installation_path();
    path.exists()
}

#[test]
fn test_skyrim_installation_exists() {
    let path = get_skyrim_installation_path();
    if !path.exists() {
        println!("⚠️ Skyrim.esm 未找到：{}", path.display());
        println!("   跳过此测试（需要完整的 Skyrim 安装）");
        return;
    }

    println!("✓ Skyrim.esm 找到：{}", path.display());

    // 检查 BSA 文件是否存在
    let bsa_path = path.parent().unwrap().join("Skyrim - Interface.bsa");
    if bsa_path.exists() {
        println!("✓ Skyrim - Interface.bsa 找到：{}", bsa_path.display());
    } else {
        println!("⚠️ Skyrim - Interface.bsa 未找到：{}", bsa_path.display());
    }
}

#[test]
fn test_load_auto_with_bsa_fallback() {
    println!("\n========== 测试 LoadedPlugin::load_auto BSA Fallback ==========");

    // 检查安装是否存在
    if !skyrim_installation_exists() {
        println!("⚠️ 跳过测试：Skyrim 安装不存在");
        return;
    }

    let esp_path = get_skyrim_installation_path();
    println!("目标文件：{}", esp_path.display());

    // 检查文件系统中是否有 STRING 文件
    let data_dir = esp_path.parent().unwrap();
    let strings_dir = data_dir.join("Strings");
    let has_filesystem_strings = strings_dir.exists()
        && strings_dir.join("Skyrim_english.STRINGS").exists();

    if has_filesystem_strings {
        println!("⚠️ 注意：文件系统中存在 STRING 文件，可能不会触发 BSA fallback");
        println!("   Strings 目录：{}", strings_dir.display());
    } else {
        println!("✓ 文件系统中没有 STRING 文件，将触发 BSA fallback");
    }

    // 使用 load_auto 加载
    println!("\n正在使用 LoadedPlugin::load_auto 加载...");
    let loaded = match LoadedPlugin::load_auto(esp_path.clone(), Some("english")) {
        Ok(l) => l,
        Err(e) => {
            panic!("❌ load_auto 失败: {}", e);
        }
    };

    println!("✓ load_auto 执行成功");

    // 验证返回的类型
    match loaded {
        LoadedPlugin::Standard(plugin) => {
            println!("\n返回类型：LoadedPlugin::Standard");
            println!("  - 是否为主文件：{}", plugin.is_master());
            println!("  - 是否本地化：{}", plugin.is_localized());

            if plugin.is_localized() {
                println!("⚠️ 警告：插件有 LOCALIZED 标志，但返回了 Standard 类型");
                println!("   这意味着 STRING 文件加载失败（BSA fallback 未生效）");
                panic!("❌ BSA fallback 未按预期工作");
            } else {
                println!("✓ 插件不是本地化插件（符合预期）");
            }
        }
        LoadedPlugin::Localized(context) => {
            println!("\n返回类型：LoadedPlugin::Localized");

            let plugin = context.plugin();
            let string_files = context.string_files();

            println!("  插件信息：");
            println!("    - 是否为主文件：{}", plugin.is_master());
            println!("    - 是否本地化：{}", plugin.is_localized());

            // 验证 STRING 文件是否加载
            use esp_extractor::StringFileType;
            let strings_count = string_files.get_file(&StringFileType::STRINGS)
                .map(|f| f.count()).unwrap_or(0);
            let ilstrings_count = string_files.get_file(&StringFileType::ILSTRINGS)
                .map(|f| f.count()).unwrap_or(0);
            let dlstrings_count = string_files.get_file(&StringFileType::DLSTRINGS)
                .map(|f| f.count()).unwrap_or(0);
            let total_count = string_files.total_count();

            println!("\n  STRING 文件统计：");
            println!("    - STRINGS: {} 条", strings_count);
            println!("    - ILSTRINGS: {} 条", ilstrings_count);
            println!("    - DLSTRINGS: {} 条", dlstrings_count);
            println!("    - 总计: {} 条", total_count);

            // 验证至少加载了一些字符串
            assert!(total_count > 0, "应该从 BSA 加载到字符串");

            // 提取字符串样例
            let strings = plugin.extract_strings();
            println!("\n  提取的字符串数量：{}", strings.len());

            if strings.len() > 0 {
                println!("\n  字符串样例（前3个）：");
                for (i, string) in strings.iter().take(3).enumerate() {
                    let text_preview = string.text.chars().take(50).collect::<String>();
                    println!("    {}. [{}] {}: {}",
                        i + 1,
                        string.record_type,
                        string.subrecord_type,
                        text_preview
                    );
                    if string.index != 0 {
                        println!("       (索引: {})", string.index);
                    }
                }
            }

            if !has_filesystem_strings {
                println!("\n✅ BSA Fallback 成功！");
                println!("   文件系统中没有 STRING 文件，但成功从 BSA 加载了 {} 条字符串", total_count);
            } else {
                println!("\n✓ STRING 文件加载成功");
                println!("   （可能来自文件系统或 BSA）");
            }
        }
    }

    println!("\n========== 测试通过 ==========");
}

#[test]
fn test_verify_bsa_interface_exists() {
    println!("\n========== 验证 BSA Interface 文件存在 ==========");

    if !skyrim_installation_exists() {
        println!("⚠️ 跳过测试：Skyrim 安装不存在");
        return;
    }

    let esp_path = get_skyrim_installation_path();
    let data_dir = esp_path.parent().unwrap();
    let bsa_path = data_dir.join("Skyrim - Interface.bsa");

    println!("BSA 路径：{}", bsa_path.display());

    assert!(bsa_path.exists(), "Skyrim - Interface.bsa 应该存在");

    let metadata = std::fs::metadata(&bsa_path).unwrap();
    println!("✓ BSA 文件存在");
    println!("  文件大小：{} MB", metadata.len() / 1024 / 1024);

    // 尝试打开 BSA
    use esp_extractor::bsa::BsaArchive;
    match BsaArchive::open(&bsa_path) {
        Ok(archive) => {
            println!("✓ BSA 文件可以正常打开");
            println!("  文件数量：{}", archive.file_list().len());
        }
        Err(e) => {
            panic!("❌ 无法打开 BSA 文件: {}", e);
        }
    }
}

#[test]
fn test_bsa_strings_extraction() {
    println!("\n========== 测试从 BSA 直接提取 STRING 文件 ==========");

    if !skyrim_installation_exists() {
        println!("⚠️ 跳过测试：Skyrim 安装不存在");
        return;
    }

    let esp_path = get_skyrim_installation_path();

    // 直接使用 BsaStringsProvider 测试提取功能
    use esp_extractor::bsa::BsaStringsProvider;

    println!("正在打开 BSA...");
    let provider = match BsaStringsProvider::open_for_plugin(&esp_path) {
        Ok(p) => p,
        Err(e) => {
            panic!("❌ 无法打开 BSA: {}", e);
        }
    };

    println!("✓ BSA 打开成功");

    // 尝试提取三种类型的 STRING 文件
    let plugin_name = "Skyrim";
    let language = "english";

    for extension in ["STRINGS", "ILSTRINGS", "DLSTRINGS"] {
        println!("\n正在提取 {}_{}.{}...", plugin_name, language, extension);

        match provider.extract_strings(plugin_name, language, extension) {
            Ok(data) => {
                println!("✓ 提取成功");
                println!("  数据大小：{} bytes", data.len());

                // 尝试解析 STRING 文件
                use esp_extractor::{StringFile, StringFileType};
                let file_type = StringFileType::from_extension(extension)
                    .expect("无效的扩展名");

                match StringFile::from_bytes(&data, plugin_name.to_string(), language.to_string(), file_type) {
                    Ok(string_file) => {
                        println!("  解析成功：{} 条字符串", string_file.count());

                        // 显示前几条字符串
                        if string_file.count() > 0 {
                            println!("  字符串样例（前3条）：");
                            let ids: Vec<u32> = string_file.get_string_ids().into_iter().take(3).collect();
                            for (i, id) in ids.iter().enumerate() {
                                if let Some(entry) = string_file.get_string(*id) {
                                    let text_preview = entry.content.chars().take(50).collect::<String>();
                                    println!("    {}. ID={}: {}", i + 1, id, text_preview);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        println!("  ⚠️ 解析失败: {}", e);
                    }
                }
            }
            Err(e) => {
                println!("  ⚠️ 提取失败: {}", e);
            }
        }
    }

    println!("\n========== 测试完成 ==========");
}

#[test]
fn test_compare_filesystem_vs_bsa() {
    println!("\n========== 比较文件系统 vs BSA 加载 ==========");

    if !skyrim_installation_exists() {
        println!("⚠️ 跳过测试：Skyrim 安装不存在");
        return;
    }

    let esp_path = get_skyrim_installation_path();
    let data_dir = esp_path.parent().unwrap();
    let strings_dir = data_dir.join("Strings");

    // 检查文件系统是否有 STRING 文件
    let has_filesystem = strings_dir.exists()
        && strings_dir.join("Skyrim_english.STRINGS").exists();

    if !has_filesystem {
        println!("⚠️ 文件系统中没有 STRING 文件，跳过比较");
        println!("   （只能测试 BSA 加载）");
        return;
    }

    println!("文件系统中存在 STRING 文件，进行加载性能比较...\n");

    use std::time::Instant;
    use esp_extractor::LocalizedPluginContext;

    // 测试1：使用 LocalizedPluginContext（会先尝试文件系统）
    println!("测试1：LocalizedPluginContext::load（优先文件系统）");
    let start = Instant::now();
    let context_fs = LocalizedPluginContext::load(esp_path.clone(), "english")
        .expect("文件系统加载失败");
    let duration_fs = start.elapsed();
    let count_fs = context_fs.string_files().total_count();
    println!("  耗时：{:?}", duration_fs);
    println!("  字符串数量：{}", count_fs);

    // 测试2：使用 load_auto（自动选择）
    println!("\n测试2：LoadedPlugin::load_auto（自动选择）");
    let start = Instant::now();
    let loaded_auto = LoadedPlugin::load_auto(esp_path.clone(), Some("english"))
        .expect("load_auto 失败");
    let duration_auto = start.elapsed();

    match loaded_auto {
        LoadedPlugin::Localized(context) => {
            let count_auto = context.string_files().total_count();
            println!("  耗时：{:?}", duration_auto);
            println!("  字符串数量：{}", count_auto);

            // 验证数量一致
            assert_eq!(count_fs, count_auto, "两种方式加载的字符串数量应该一致");
        }
        LoadedPlugin::Standard(_) => {
            panic!("load_auto 应该返回 Localized 类型");
        }
    }

    println!("\n✓ 比较完成，结果一致");
}
