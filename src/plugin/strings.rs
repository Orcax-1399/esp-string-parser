use super::Plugin;
use crate::datatypes::{read_u32, RawString};
use crate::record::Record;
use crate::group::{Group, GroupChild};
use crate::string_types::ExtractedString;
use crate::utils::is_valid_string;
use std::io::Cursor;
use rayon::prelude::*;

impl Plugin {
    /// 提取所有字符串（并行版本，性能提升 1.5-2x）
    pub fn extract_strings(&self) -> Vec<ExtractedString> {
        self.groups
            .par_iter()
            .flat_map(|group| self.extract_group_strings(group))
            .collect()
    }

    /// 从组中提取字符串
    fn extract_group_strings(&self, group: &Group) -> Vec<ExtractedString> {
        let mut strings = Vec::new();
        for child in &group.children {
            match child {
                GroupChild::Group(subgroup) => {
                    strings.extend(self.extract_group_strings(subgroup));
                }
                GroupChild::Record(record) => {
                    strings.extend(self.extract_record_strings(record));
                }
            }
        }
        strings
    }

    /// 从记录中提取字符串
    ///
    /// 所有 string subrecord 都按出现顺序分配索引（0, 1, 2...）
    fn extract_record_strings(&self, record: &Record) -> Vec<ExtractedString> {
        let mut strings = Vec::new();

        // 使用字符串路由器获取支持的子记录类型（v0.6.0 - P2.3）
        let string_types = match self.string_router().get_string_subrecord_types(&record.record_type) {
            Some(types) => types,
            None => return strings,
        };

        let editor_id = record.get_editor_id();
        let form_id_str = self.format_form_id(record.form_id);

        // 全局索引计数器：按 subrecord 在 record.subrecords 中的出现顺序
        let mut index = 0i32;

        for subrecord in &record.subrecords {
            if string_types.contains(&subrecord.record_type) {
                if let Some(extracted) = self.extract_string_from_subrecord_with_index(
                    subrecord, &editor_id, &form_id_str, &record.record_type, index
                ) {
                    strings.push(extracted);
                }
                index += 1; // 每个 string subrecord 递增
            }
        }

        strings
    }

    /// 从子记录中提取字符串（带索引支持）
    ///
    /// 所有字段都有 index 参数，按 Record 内的顺序分配
    fn extract_string_from_subrecord_with_index(
        &self,
        subrecord: &crate::subrecord::Subrecord,
        editor_id: &Option<String>,
        form_id_str: &str,
        record_type: &str,
        index: i32,
    ) -> Option<ExtractedString> {
        let raw_string = if self.is_localized() {
            // 本地化插件：数据是字符串ID（前4字节）
            let mut cursor = Cursor::new(&subrecord.data[..]);
            let string_id = match read_u32(&mut cursor) {
                Ok(id) => id,
                Err(_) => return None,
            };

            // StringID 为 0 表示无字符串或空字段，直接跳过不处理
            if string_id == 0 {
                return None;
            }

            // 确定应该从哪个STRING文件查找
            let file_type = Self::determine_string_file_type(record_type, &subrecord.record_type);

            // 从STRING文件查找实际文本
            if let Some(ref string_files) = self.string_files {
                if let Some(entry) = string_files.get_string_by_type(file_type, string_id) {
                    // 调试模式下输出成功提取的详细信息（设置环境变量 ESP_DEBUG_STRINGS=1 启用）
                    #[cfg(debug_assertions)]
                    if std::env::var("ESP_DEBUG_STRINGS").is_ok() {
                        let editor_id_str = editor_id.as_ref()
                            .map(|s| s.as_str())
                            .unwrap_or("<无EditorID>");
                        eprintln!(
                            "DEBUG: StringID {} 从 {:?} 文件提取 (来自 {}.{}, FormID: {}, EditorID: {}, 内容: \"{}\")",
                            string_id, file_type, record_type, &subrecord.record_type, form_id_str, editor_id_str,
                            &entry.content.chars().take(30).collect::<String>()
                        );
                    }

                    RawString {
                        content: entry.content.clone(),
                        encoding: "utf-8".to_string(),
                    }
                } else {
                    // STRING文件中未找到，返回占位符
                    #[cfg(debug_assertions)]
                    {
                        let editor_id_str = editor_id.as_ref()
                            .map(|s| s.as_str())
                            .unwrap_or("<无EditorID>");
                        eprintln!(
                            "警告: StringID {} 在 {:?} 文件中未找到 (来自 {}.{}, FormID: {}, EditorID: {})",
                            string_id, file_type, record_type, &subrecord.record_type, form_id_str, editor_id_str
                        );
                    }

                    RawString {
                        content: format!("StringID_{}_{:?}", string_id, file_type),
                        encoding: "ascii".to_string(),
                    }
                }
            } else {
                // 没有加载STRING文件
                #[cfg(debug_assertions)]
                {
                    let editor_id_str = editor_id.as_ref()
                        .map(|s| s.as_str())
                        .unwrap_or("<无EditorID>");
                    eprintln!(
                        "警告: 本地化插件但未加载STRING文件 (StringID: {}, 来自 {}.{}, FormID: {}, EditorID: {})",
                        string_id, record_type, &subrecord.record_type, form_id_str, editor_id_str
                    );
                }

                RawString {
                    content: format!("StringID_{}", string_id),
                    encoding: "ascii".to_string(),
                }
            }
        } else {
            // 普通插件：直接解析字符串
            RawString::parse_zstring(&subrecord.data)
        };

        if is_valid_string(&raw_string.content) {
            // 所有字段都有索引
            Some(ExtractedString::new(
                editor_id.clone(),
                form_id_str.to_string(),
                record_type.to_string(),
                subrecord.record_type.clone(),
                raw_string.content,
                index,
            ))
        } else {
            None
        }
    }
}
