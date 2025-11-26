use super::Plugin;
use crate::datatypes::read_u32;
use crate::record::Record;
use crate::group::{Group, GroupChild};
use crate::string_types::ExtractedString;
use crate::string_file::StringFileType;
use crate::string_routes::StringRouter;
use std::collections::HashMap;
use std::io::Cursor;
use std::path::PathBuf;
use std::sync::Arc;

impl Plugin {
    /// 从翻译文件创建新的ESP文件
    #[allow(deprecated)]
    pub fn apply_translations(
        input_path: PathBuf,
        output_path: PathBuf,
        translations: Vec<ExtractedString>,
        language: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        #[cfg(debug_assertions)]
        let backup_path = crate::utils::create_backup(&input_path)?;
        #[cfg(not(debug_assertions))]
        let _backup_path = crate::utils::create_backup(&input_path)?;

        #[cfg(debug_assertions)]
        println!("已创建备份文件: {:?}", backup_path);

        let mut plugin = Self::new(input_path, language)?;

        // 确定输出目录：如果output_path是文件，使用父目录；如果是目录，直接使用
        let output_dir = if output_path.is_dir() {
            Some(output_path.as_path())
        } else {
            output_path.parent()
        };

        // 使用统一的翻译应用接口（自动判断本地化/非本地化）
        plugin.apply_translations_unified(translations, output_dir)?;

        Ok(())
    }

    /// 统一应用翻译（自动判断本地化/非本地化插件）
    ///
    /// # 参数
    /// * `translations` - 翻译列表
    /// * `output_dir` - 可选输出目录，如果为None则覆盖原文件
    ///
    /// # 行为
    /// - 本地化插件：写入STRING文件到 output_dir/strings/ 或原目录
    /// - 普通插件：写入ESP文件到 output_dir/xxx.esp 或原路径
    pub fn apply_translations_unified(
        &mut self,
        translations: Vec<ExtractedString>,
        output_dir: Option<&std::path::Path>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.is_localized() {
            // 本地化插件：应用翻译到STRING文件
            self.apply_translations_to_string_files(translations, output_dir)
        } else {
            // 普通插件：应用翻译到ESP文件
            self.apply_translations_to_esp(translations, output_dir)
        }
    }

    /// 应用翻译到STRING文件（本地化插件）
    fn apply_translations_to_string_files(
        &mut self,
        translations: Vec<ExtractedString>,
        output_dir: Option<&std::path::Path>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // 第一步：遍历ESP，建立 UniqueKey -> (StringFileType, StringID) 映射
        // 注意：先不借用string_files，避免借用冲突
        let mut string_id_map: HashMap<String, (StringFileType, u32)> = HashMap::new();

        for group in &self.groups {
            self.build_string_id_map_from_group(group, &mut string_id_map)?;
        }

        #[cfg(debug_assertions)]
        println!("从ESP文件中提取了 {} 个StringID映射", string_id_map.len());

        // 第二步：获取string_files的可变引用并更新
        let string_files = self.string_files.as_mut()
            .ok_or("本地化插件但未加载STRING文件")?;

        let mut applied_count = 0;
        for trans in translations {
            let key = trans.get_unique_key();
            if let Some((file_type, string_id)) = string_id_map.get(&key) {
                // 使用 get_text_to_apply() 来获取翻译文本（优先）或原文
                let text_to_apply = trans.get_text_to_apply().to_string();
                match string_files.update_string(*file_type, *string_id, text_to_apply) {
                    Ok(_) => applied_count += 1,
                    Err(_e) => {
                        #[cfg(debug_assertions)]
                        eprintln!("警告: 无法更新StringID {}: {}", string_id, _e);
                    }
                }
            } else {
                #[cfg(debug_assertions)]
                eprintln!("警告: 未找到翻译键对应的StringID: {}", key);
            }
        }

        println!("成功应用了 {} 个翻译到STRING文件", applied_count);

        // 第三步：写入STRING文件
        let output_path = if let Some(dir) = output_dir {
            // 输出到指定目录：output_dir/strings/
            dir.join("strings")
        } else {
            // 覆盖原文件
            self.path.parent().unwrap().to_path_buf()
        };

        std::fs::create_dir_all(&output_path)?;

        #[cfg(debug_assertions)]
        println!("准备写入STRING文件到: {:?}", output_path);

        string_files.write_all(&output_path)?;

        println!("STRING文件已成功写入");

        Ok(())
    }

    /// 从组中构建StringID映射
    fn build_string_id_map_from_group(
        &self,
        group: &Group,
        map: &mut HashMap<String, (StringFileType, u32)>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for child in &group.children {
            match child {
                GroupChild::Group(subgroup) => {
                    self.build_string_id_map_from_group(subgroup, map)?;
                }
                GroupChild::Record(record) => {
                    self.build_string_id_map_from_record(record, map)?;
                }
            }
        }
        Ok(())
    }

    /// 从记录构建 StringID 映射（用于本地化插件）
    ///
    /// 使用与提取逻辑完全一致的全局索引计数器
    fn build_string_id_map_from_record(
        &self,
        record: &crate::record::Record,
        map: &mut HashMap<String, (StringFileType, u32)>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // 获取编辑器ID
        let editor_id = record.get_editor_id();
        let form_id_str = self.format_form_id(record.form_id);

        // 获取支持的字符串子记录类型（v0.6.0 - P2.3）
        let valid_subrecord_types = self.string_router().get_string_subrecord_types(&record.record_type);

        // 全局索引计数器（与提取/应用逻辑完全一致）
        let mut index = 0i32;

        for subrecord in &record.subrecords {
            if let Some(types) = valid_subrecord_types {
                if types.contains(&subrecord.record_type) {
                    // 读取StringID
                    let mut cursor = Cursor::new(&subrecord.data[..]);
                    if let Ok(string_id) = read_u32(&mut cursor) {
                        // 确定文件类型
                        let file_type = Self::determine_string_file_type(
                            &record.record_type,
                            &subrecord.record_type,
                        );

                        // 构建唯一键（所有字段都包含索引）
                        let key = format!(
                            "{}|{}|{} {}|{}",
                            editor_id.as_deref().unwrap_or(""),
                            form_id_str,
                            record.record_type,
                            subrecord.record_type,
                            index
                        );

                        map.insert(key, (file_type, string_id));
                    }

                    index += 1; // 每个 string subrecord 递增
                }
            }
        }

        Ok(())
    }

    /// 应用翻译到ESP文件（普通插件）
    fn apply_translations_to_esp(
        &mut self,
        translations: Vec<ExtractedString>,
        output_dir: Option<&std::path::Path>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // 使用现有的翻译映射逻辑
        let translation_map = Self::create_translation_map(translations);
        self.apply_translation_map(&translation_map)?;

        // 写入文件
        let output_path = if let Some(dir) = output_dir {
            // 输出到指定目录：output_dir/xxx.esp
            dir.join(self.path.file_name().unwrap())
        } else {
            // 覆盖原文件
            self.path.clone()
        };

        std::fs::create_dir_all(output_path.parent().unwrap())?;

        #[cfg(debug_assertions)]
        println!("准备写入ESP文件到: {:?}", output_path);

        self.write_to_file(output_path)?;

        println!("ESP文件已成功写入");

        Ok(())
    }

    /// 应用翻译映射
    pub(crate) fn apply_translation_map(&mut self, translations: &HashMap<String, ExtractedString>) -> Result<(), Box<dyn std::error::Error>> {
        // 克隆 Arc 以避免借用冲突（v0.6.0 - P2.3）
        let string_router = Arc::clone(&self.string_router);
        let masters = self.masters.clone();
        let plugin_name = self.get_name().to_string();

        println!("开始应用翻译映射，翻译表中有 {} 个条目", translations.len());

        #[cfg(debug_assertions)]
        {
            println!("翻译表中的键值示例:");
            for (i, key) in translations.keys().take(3).enumerate() {
                println!("  {}: {}", i + 1, key);
            }
            if translations.len() > 3 {
                println!("  ... 还有 {} 个键", translations.len() - 3);
            }
        }

        let mut applied_count = 0;
        for group in &mut self.groups {
            applied_count += apply_translations_to_group(
                group,
                translations,
                string_router.as_ref(),
                &masters,
                &plugin_name
            )?;
        }

        println!("成功应用了 {} 个翻译", applied_count);
        if applied_count == 0 {
            println!("⚠️ 警告：没有任何翻译被应用，可能原因：");
            println!("  1. 翻译文件中的键与ESP文件中的字符串不匹配");
            println!("  2. FormID格式不正确");
            println!("  3. 记录类型或子记录类型不匹配");
        }

        Ok(())
    }

    /// 创建翻译映射
    fn create_translation_map(translations: Vec<ExtractedString>) -> HashMap<String, ExtractedString> {
        translations
            .into_iter()
            .map(|t| (t.get_unique_key(), t))
            .collect()
    }
}

/// 对组应用翻译
fn apply_translations_to_group(
    group: &mut Group,
    translations: &HashMap<String, ExtractedString>,
    string_router: &dyn StringRouter,
    masters: &[String],
    plugin_name: &str,
) -> Result<usize, Box<dyn std::error::Error>> {
    let mut count = 0;
    for child in &mut group.children {
        match child {
            GroupChild::Group(subgroup) => {
                count += apply_translations_to_group(subgroup, translations, string_router, masters, plugin_name)?;
            }
            GroupChild::Record(record) => {
                count += apply_translations_to_record(record, translations, string_router, masters, plugin_name)?;
            }
        }
    }
    Ok(count)
}

/// 对记录应用翻译
///
/// 使用与提取逻辑完全一致的全局索引计数器，确保索引匹配正确
fn apply_translations_to_record(
    record: &mut Record,
    translations: &HashMap<String, ExtractedString>,
    string_router: &dyn StringRouter,
    masters: &[String],
    plugin_name: &str,
) -> Result<usize, Box<dyn std::error::Error>> {
    // 使用字符串路由器获取支持的子记录类型（v0.6.0 - P2.3）
    let string_types = match string_router.get_string_subrecord_types(&record.record_type) {
        Some(types) => types,
        None => return Ok(0),
    };

    let editor_id = record.get_editor_id();
    let form_id_str = format_form_id_helper(record.form_id, masters, plugin_name);

    let mut modified = false;
    let mut applied_count = 0;

    // 全局索引计数器（与提取逻辑完全一致）
    let mut index = 0i32;

    for subrecord in &mut record.subrecords {
        if string_types.contains(&subrecord.record_type) {
            let string_type = format!("{} {}", record.record_type, subrecord.record_type);
            // 构建带索引的 key（所有字段都包含 index）
            let key = format!("{}|{}|{}|{}",
                editor_id.as_deref().unwrap_or(""),
                form_id_str,
                string_type,
                index
            );

            #[cfg(debug_assertions)]
            println!("尝试匹配键（index={}）: {}", index, key);

            if let Some(translation) = translations.get(&key) {
                let text_to_apply = translation.get_text_to_apply();
                if !text_to_apply.is_empty() {

                    #[cfg(debug_assertions)]
                    println!("✓ 成功应用翻译（index={}）: [{}] {} -> \"{}\"",
                        index,
                        translation.form_id,
                        translation.get_string_type(),
                        if text_to_apply.chars().count() > 50 {
                            format!("{}...", text_to_apply.chars().take(50).collect::<String>())
                        } else {
                            text_to_apply.to_string()
                        }
                    );

                    let encoded_data = encode_string_with_encoding(text_to_apply, "utf-8")?;
                    subrecord.data = encoded_data;
                    subrecord.size = subrecord.data.len() as u16;
                    modified = true;
                    applied_count += 1;
                }
            }

            index += 1; // 每个 string subrecord 递增
        }
    }

    if modified {
        record.mark_modified();
    }

    Ok(applied_count)
}

/// 格式化FormID辅助函数
fn format_form_id_helper(form_id: u32, masters: &[String], plugin_name: &str) -> String {
    let master_index = (form_id >> 24) as usize;
    let master_file = if master_index < masters.len() {
        &masters[master_index]
    } else {
        plugin_name
    };

    format!("{:08X}|{}", form_id, master_file)
}

/// 使用指定编码编码字符串
fn encode_string_with_encoding(text: &str, encoding: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    #[allow(clippy::wildcard_in_or_patterns)]
    let mut result = match encoding.to_lowercase().as_str() {
        "utf8" | "utf-8" => text.as_bytes().to_vec(),
        "gbk" | "gb2312" => {
            encoding_rs::GBK.encode(text).0.into_owned()
        }
        "ascii" | _ => {
            text.chars()
                .map(|c| if c.is_ascii() { c as u8 } else { b'?' })
                .collect()
        }
    };

    // 添加null终止符
    result.push(0);
    Ok(result)
}
