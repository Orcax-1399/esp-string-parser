use super::*;
use std::collections::HashMap;
use std::path::PathBuf;
use tempfile::TempDir;

/// 创建测试用的StringFile
fn create_test_string_file() -> StringFile {
    let mut entries = HashMap::new();

    entries.insert(1, StringEntry::new(1, "Iron Sword".to_string()));
    entries.insert(2, StringEntry::new(2, "Steel Dagger".to_string()));
    entries.insert(100, StringEntry::new(100, "Dragon's Breath".to_string()));

    StringFile {
        path: PathBuf::from("test.STRINGS"),
        file_type: StringFileType::STRINGS,
        plugin_name: "TestMod".to_string(),
        language: "english".to_string(),
        entries,
    }
}

#[test]
fn test_update_string() {
    let mut file = create_test_string_file();

    assert!(file.update_string(1, "铁剑".to_string()).is_ok());
    assert_eq!(file.get_string(1).unwrap().content, "铁剑");

    assert!(file.update_string(999, "不存在".to_string()).is_err());
}

#[test]
fn test_add_string() {
    let mut file = create_test_string_file();

    assert!(file.add_string(200, "新物品".to_string()).is_ok());
    assert_eq!(file.get_string(200).unwrap().content, "新物品");

    assert!(file.add_string(1, "重复".to_string()).is_err());
}

#[test]
fn test_remove_string() {
    let mut file = create_test_string_file();

    assert!(file.remove_string(1).is_some());
    assert!(file.get_string(1).is_none());

    assert!(file.remove_string(999).is_none());
}

#[test]
fn test_rebuild_strings() {
    let file = create_test_string_file();

    let result = file.rebuild();
    assert!(result.is_ok());

    let data = result.unwrap();

    assert!(data.len() > 8);

    let count = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    assert_eq!(count, 3);
}

#[test]
fn test_write_and_read_roundtrip() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("TestMod_english.STRINGS");

    let mut original_file = create_test_string_file();
    original_file.update_string(1, "测试中文".to_string()).unwrap();

    assert!(original_file.write_to_file(file_path.clone()).is_ok());

    let loaded_file = StringFile::new(file_path).unwrap();

    assert_eq!(loaded_file.count(), 3);
    assert_eq!(loaded_file.get_string(1).unwrap().content, "测试中文");
    assert_eq!(loaded_file.get_string(2).unwrap().content, "Steel Dagger");
}

#[test]
fn test_rebuild_dlstrings() {
    let mut entries = HashMap::new();
    entries.insert(1, StringEntry::new(1, "对话内容".to_string()));

    let file = StringFile {
        path: PathBuf::from("test.DLSTRINGS"),
        file_type: StringFileType::DLSTRINGS,
        plugin_name: "TestMod".to_string(),
        language: "chinese".to_string(),
        entries,
    };

    let result = file.rebuild();
    assert!(result.is_ok());

    let data = result.unwrap();
    assert!(data.len() > 8);
}

#[test]
fn test_string_file_set_update() {
    let mut set = StringFileSet::new("TestMod".to_string(), "english".to_string());

    let strings_file = create_test_string_file();
    set.add_file(StringFileType::STRINGS, strings_file);

    assert!(set
        .update_string(StringFileType::STRINGS, 1, "更新的文本".to_string())
        .is_ok());

    let entry = set.get_string_by_type(StringFileType::STRINGS, 1).unwrap();
    assert_eq!(entry.content, "更新的文本");
}

#[test]
fn test_batch_updates() {
    let mut file = create_test_string_file();

    let mut updates = HashMap::new();
    updates.insert(1, "铁剑".to_string());
    updates.insert(2, "钢制匕首".to_string());

    assert!(file.update_strings(updates).is_ok());

    assert_eq!(file.get_string(1).unwrap().content, "铁剑");
    assert_eq!(file.get_string(2).unwrap().content, "钢制匕首");
}

#[test]
fn test_real_fishing_strings_files() {
    let test_dir = PathBuf::from("TestFile");

    if !test_dir.exists() {
        println!("跳过测试：TestFile目录不存在");
        return;
    }

    let strings_path = test_dir.join("ccbgssse001-fish_english.STRINGS");
    if strings_path.exists() {
        let strings_file = StringFile::new(strings_path).unwrap();
        println!("STRINGS文件包含 {} 个字符串", strings_file.count());
        assert!(strings_file.count() > 0);

        let ids = strings_file.get_string_ids();
        for id in ids.iter().take(5) {
            if let Some(entry) = strings_file.get_string(*id) {
                println!("  [{}] {}", id, entry.content);
            }
        }
    }

    let dlstrings_path = test_dir.join("ccbgssse001-fish_english.DLSTRINGS");
    if dlstrings_path.exists() {
        let dlstrings_file = StringFile::new(dlstrings_path).unwrap();
        println!("DLSTRINGS文件包含 {} 个字符串", dlstrings_file.count());
        assert!(dlstrings_file.count() > 0);
    }

    let ilstrings_path = test_dir.join("ccbgssse001-fish_english.ILSTRINGS");
    if ilstrings_path.exists() {
        let ilstrings_file = StringFile::new(ilstrings_path).unwrap();
        println!("ILSTRINGS文件包含 {} 个字符串", ilstrings_file.count());
    }
}

#[test]
fn test_real_file_write_and_reload() {
    let test_dir = PathBuf::from("TestFile");

    if !test_dir.exists() {
        println!("跳过测试：TestFile目录不存在");
        return;
    }

    let strings_path = test_dir.join("ccbgssse001-fish_english.STRINGS");
    if !strings_path.exists() {
        println!("跳过测试：STRING文件不存在");
        return;
    }

    let mut original_file = StringFile::new(strings_path).unwrap();
    let original_count = original_file.count();
    println!("原始文件包含 {} 个有效字符串", original_count);

    let ids = original_file.get_string_ids();
    if let Some(&first_id) = ids.first() {
        let original_text = original_file.get_string(first_id).unwrap().content.clone();
        println!("原始文本 [{}]: {}", first_id, original_text);

        original_file.update_string(first_id, "钓鱼测试".to_string()).unwrap();

        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().join("ccbgssse001-fish_chinese.STRINGS");
        original_file.write_to_file(temp_path.clone()).unwrap();

        let reloaded_file = StringFile::new(temp_path).unwrap();
        println!("重新加载后包含 {} 个字符串", reloaded_file.count());

        assert_eq!(reloaded_file.count(), original_count, "写入前后字符串数量应该一致");

        assert_eq!(
            reloaded_file.get_string(first_id).unwrap().content,
            "钓鱼测试",
            "修改的字符串内容应该正确"
        );

        if ids.len() > 1 {
            let second_id = ids[1];
            let original_second = original_file.get_string(second_id).unwrap().content.clone();
            let reloaded_second = reloaded_file.get_string(second_id).unwrap().content.clone();
            assert_eq!(original_second, reloaded_second, "未修改的字符串应该保持不变");
        }

        println!("✓ 读写循环测试通过！所有验证成功");
    }
}

#[test]
fn test_from_bytes() {
    let test_file = create_test_string_file();

    let bytes = test_file.rebuild().unwrap();

    let loaded_file = StringFile::from_bytes(
        &bytes,
        "TestPlugin".to_string(),
        "english".to_string(),
        StringFileType::STRINGS,
    )
    .unwrap();

    assert_eq!(loaded_file.plugin_name, "TestPlugin");
    assert_eq!(loaded_file.language, "english");
    assert_eq!(loaded_file.file_type, StringFileType::STRINGS);
    assert_eq!(loaded_file.count(), test_file.count());

    assert_eq!(loaded_file.get_string(1).unwrap().content, "Iron Sword");
    assert_eq!(loaded_file.get_string(2).unwrap().content, "Steel Dagger");

    println!("✓ from_bytes 测试通过！");
}

#[test]
fn test_string_file_set_from_memory() {
    let strings_file = create_test_string_file();

    let strings_bytes = strings_file.rebuild().unwrap();

    let mut files_data = HashMap::new();
    files_data.insert(StringFileType::STRINGS, strings_bytes.as_slice());

    let set = StringFileSet::from_memory(files_data, "TestMod".to_string(), "chinese".to_string()).unwrap();

    assert_eq!(set.plugin_name, "TestMod");
    assert_eq!(set.language, "chinese");
    assert_eq!(set.files.len(), 1);

    assert!(set.get_file(&StringFileType::STRINGS).is_some());

    let string1 = set.get_string_by_type(StringFileType::STRINGS, 1);
    assert!(string1.is_some());
    assert_eq!(string1.unwrap().content, "Iron Sword");

    println!("✓ StringFileSet::from_memory 测试通过！");
}
