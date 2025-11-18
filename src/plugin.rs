use crate::datatypes::{read_u32, RawString};
use crate::record::Record;
use crate::group::{Group, GroupChild};
use crate::string_types::ExtractedString;
use crate::string_file::{StringFileSet, StringFileType};
use crate::utils::{is_valid_string, EspError};
// SpecialRecordHandler 已简化，不再需要在 plugin.rs 中使用
use std::collections::HashMap;
use std::path::PathBuf;
use std::io::{Cursor, Read};
use std::sync::Arc;
use serde_json;
use memmap2::Mmap;
use rayon::prelude::*;

/// ESP插件解析器
#[derive(Debug)]
pub struct Plugin {
    /// 文件路径
    pub path: PathBuf,
    /// 头部记录
    pub header: Record,
    /// 组列表
    pub groups: Vec<Group>,
    /// 主文件列表
    pub masters: Vec<String>,
    /// 字符串记录定义
    pub string_records: HashMap<String, Vec<String>>,
    /// STRING文件集合（仅本地化插件有值）
    string_files: Option<StringFileSet>,
    /// 语言标识（用于STRING文件查找）
    /// 注意：此字段仅用于向后兼容 deprecated 的 `new()` 方法
    #[allow(dead_code)]
    language: String,
    /// 内存映射文件（性能优化：零拷贝访问文件数据）
    #[allow(dead_code)]
    mmap: Option<Arc<Mmap>>,
}

/// 修改信息结构
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

    /// 加载插件文件（v0.4.0+ 推荐方法）
    ///
    /// 只解析 ESP/ESM/ESL 文件本身，不加载 STRING 文件。
    /// 如需处理本地化插件，请使用 `LocalizedPluginContext::load()`。
    ///
    /// # 参数
    /// * `path` - ESP/ESM/ESL文件路径
    ///
    /// # 返回
    /// 返回解析后的 Plugin 实例
    ///
    /// # 示例
    /// ```rust,ignore
    /// let plugin = Plugin::load("example.esp".into())?;
    /// ```
    pub fn load(path: PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let string_records = Self::load_string_records()?;

        // 使用内存映射文件（零拷贝，性能提升 ~500-600ms）
        let file = std::fs::File::open(&path)?;
        let mmap = unsafe { Mmap::map(&file)? };
        let mmap = Arc::new(mmap);

        let mut cursor = Cursor::new(&mmap[..]);

        let header = Record::parse(&mut cursor)?;
        Self::validate_esp_file(&header)?;

        let masters = Self::extract_masters(&header);
        let groups = Self::parse_groups(&mut cursor, &mmap[..])?;

        Ok(Plugin {
            path,
            header,
            groups,
            masters,
            string_records,
            string_files: None,
            language: String::new(),
            mmap: Some(mmap),
        })
    }

    /// 创建新的插件实例（已弃用，请使用 `Plugin::load()`）
    ///
    /// # 参数
    /// * `path` - ESP/ESM/ESL文件路径
    /// * `language` - 语言标识（用于加载STRING文件），默认为"english"
    ///
    /// # 自动加载STRING文件
    /// 如果插件设置了LOCALIZED标志，会自动尝试加载同目录下的STRING文件
    ///
    /// # 废弃说明
    /// 此方法违反单一职责原则（自动加载 STRING 文件），将在 v1.0.0 移除。
    /// 请使用 `Plugin::load()` 代替，如需处理本地化插件请使用 `LocalizedPluginContext`。
    #[deprecated(
        since = "0.4.0",
        note = "使用 Plugin::load() 代替。如需加载 STRING 文件，请使用 LocalizedPluginContext::load()"
    )]
    pub fn new(path: PathBuf, language: Option<&str>) -> Result<Self, Box<dyn std::error::Error>> {
        let language = language.unwrap_or("english").to_string();
        let string_records = Self::load_string_records()?;

        // 使用内存映射文件（零拷贝，性能提升 ~500-600ms）
        let file = std::fs::File::open(&path)?;
        let mmap = unsafe { Mmap::map(&file)? };
        let mmap = Arc::new(mmap);

        let mut cursor = Cursor::new(&mmap[..]);

        let header = Record::parse(&mut cursor)?;
        Self::validate_esp_file(&header)?;

        let masters = Self::extract_masters(&header);
        let groups = Self::parse_groups(&mut cursor, &mmap[..])?;

        // 检查是否为本地化插件
        let is_localized = header.flags & 0x00000080 != 0;

        // 自动加载STRING文件（如果是本地化插件）
        let string_files = if is_localized {
            let plugin_dir = path.parent().ok_or("无法获取插件目录")?;
            let plugin_name = path.file_stem()
                .and_then(|s| s.to_str())
                .ok_or("无法获取插件名称")?;

            // 尝试多个可能的STRING文件位置
            let search_dirs = vec![
                plugin_dir.to_path_buf(),                    // 同目录
                plugin_dir.join("Strings"),                  // Strings子目录（常见于开发环境）
                plugin_dir.join("strings"),                  // strings子目录（小写）
            ];

            let mut loaded_set: Option<StringFileSet> = None;

            for dir in search_dirs {
                if !dir.exists() {
                    continue;
                }

                match StringFileSet::load_from_directory(&dir, plugin_name, &language) {
                    Ok(set) if !set.files.is_empty() => {
                        #[cfg(debug_assertions)]
                        println!("已加载STRING文件: {} 个文件类型（从 {:?}）", set.files.len(), dir);
                        loaded_set = Some(set);
                        break;
                    }
                    Ok(_) => {
                        // 找到目录但没有STRING文件，继续搜索
                        #[cfg(debug_assertions)]
                        eprintln!("提示: {:?} 目录下未找到STRING文件", dir);
                    }
                    Err(_e) => {
                        #[cfg(debug_assertions)]
                        eprintln!("警告: 无法从 {:?} 加载STRING文件: {}", dir, _e);
                    }
                }
            }

            if loaded_set.is_none() {
                #[cfg(debug_assertions)]
                eprintln!("警告: 本地化插件但未找到任何STRING文件");
            }

            loaded_set
        } else {
            None
        };

        Ok(Plugin {
            path,
            header,
            groups,
            masters,
            string_records,
            string_files,
            language,
            mmap: Some(mmap),
        })
    }
    
    /// 验证ESP文件格式
    fn validate_esp_file(header: &Record) -> Result<(), Box<dyn std::error::Error>> {
        if !matches!(header.record_type.as_str(), "TES4" | "TES3") {
            return Err(EspError::InvalidFormat.into());
        }
        Ok(())
    }
    
    /// 解析所有组（并行版本，性能提升 1.5-2x）
    fn parse_groups(cursor: &mut Cursor<&[u8]>, data: &[u8]) -> Result<Vec<Group>, Box<dyn std::error::Error>> {
        // 第一遍：快速扫描获取所有顶级 Group 边界
        let group_ranges = Self::scan_group_boundaries(cursor, data)?;

        if group_ranges.is_empty() {
            return Ok(Vec::new());
        }

        // 第二遍：并行解析每个 Group
        let groups: Result<Vec<Group>, String> = group_ranges
            .par_iter()
            .map(|&(start, size)| -> Result<Group, String> {
                let end = start + size as u64;
                if end > data.len() as u64 {
                    return Err(format!("Group 边界超出数据范围: {}..{} (数据长度: {})", start, end, data.len()));
                }
                let group_data = &data[start as usize..end as usize];
                let mut group_cursor = Cursor::new(group_data);
                Group::parse(&mut group_cursor).map_err(|e| e.to_string())
            })
            .collect();

        groups.map_err(|e| e.into())
    }

    /// 扫描顶级 Group 边界（用于并行解析）
    fn scan_group_boundaries(cursor: &mut Cursor<&[u8]>, data: &[u8]) -> Result<Vec<(u64, u32)>, Box<dyn std::error::Error>> {
        let mut boundaries = Vec::new();
        let start_pos = cursor.position();

        while cursor.position() < data.len() as u64 {
            let pos = cursor.position();

            // 检查是否有足够的数据读取 Group 头部（至少8字节：类型4字节+大小4字节）
            if pos + 8 > data.len() as u64 {
                break;
            }

            // 读取类型标识
            let mut type_bytes = [0u8; 4];
            if cursor.read_exact(&mut type_bytes).is_err() {
                break;
            }

            // 验证是否为 GRUP
            if &type_bytes != b"GRUP" {
                return Err(format!("在位置 {} 期望 GRUP，但找到 {}",
                    pos, String::from_utf8_lossy(&type_bytes)).into());
            }

            // 读取 Group 大小
            let size = read_u32(cursor)?;

            // 验证大小合理性
            if size < 24 || size > 200_000_000 {
                return Err(format!("在位置 {} 发现异常 Group 大小: {} bytes", pos, size).into());
            }

            // 记录边界（起始位置，大小）
            boundaries.push((pos, size));

            // 跳到下一个 Group
            cursor.set_position(pos + size as u64);
        }

        // 恢复到开始位置
        cursor.set_position(start_pos);

        Ok(boundaries)
    }
    
    /// 创建翻译映射
    fn create_translation_map(translations: Vec<ExtractedString>) -> HashMap<String, ExtractedString> {
        translations
            .into_iter()
            .map(|t| (t.get_unique_key(), t))
            .collect()
    }
    
    /// 加载字符串记录定义
    fn load_string_records() -> Result<HashMap<String, Vec<String>>, Box<dyn std::error::Error>> {
        let json_data = include_str!("../data/string_records.json");
        Ok(serde_json::from_str(json_data)?)
    }
    
    /// 从头部记录提取主文件列表
    fn extract_masters(header: &Record) -> Vec<String> {
        header.subrecords.iter()
            .filter(|sr| sr.record_type == "MAST")
            .map(|sr| RawString::parse_zstring(&sr.data).content)
            .collect()
    }

    /// 根据记录类型和子记录类型确定应该使用哪个STRING文件类型
    ///
    /// # 映射规则
    /// - INFO 记录 → ILSTRINGS（对话信息）
    /// - DESC/CNAM 子记录 → DLSTRINGS（描述文本/内容，通常是较长的文本）
    /// - 其他所有字符串子记录 (FULL/NNAM等) → STRINGS (默认)
    fn determine_string_file_type(record_type: &str, subrecord_type: &str) -> StringFileType {
        // INFO 记录 → ILSTRINGS
        // INFO 记录包含对话信息，按照 Bethesda 约定存储在 ILSTRINGS 中
        if record_type == "INFO" {
            return StringFileType::ILSTRINGS;
        }

        // DESC 和 CNAM 子记录 → DLSTRINGS
        // 这些通常是较长的描述性文本或内容，按照 Bethesda 约定存储在 DLSTRINGS 中
        if matches!(subrecord_type, "DESC" | "CNAM") {
            return StringFileType::DLSTRINGS;
        }

        // 默认 → STRINGS
        // 包括 FULL, NNAM, SHRT, DNAM 等常规名称和简短文本
        StringFileType::STRINGS
    }

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

        let string_types = match self.string_records.get(&record.record_type) {
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
    
    /// 格式化FormID
    fn format_form_id(&self, form_id: u32) -> String {
        let master_index = (form_id >> 24) as usize;
        let master_file = if master_index < self.masters.len() {
            &self.masters[master_index]
        } else {
            self.path.file_name().unwrap().to_str().unwrap()
        };
        
        format!("{:08X}|{}", form_id, master_file)
    }
    
    /// 获取插件名称
    pub fn get_name(&self) -> &str {
        self.path.file_name().unwrap().to_str().unwrap()
    }
    
    /// 获取插件类型
    pub fn get_type(&self) -> &str {
        match self.path.extension().and_then(|ext| ext.to_str()) {
            Some("esp") => "插件 (ESP)",
            Some("esm") => "主文件 (ESM)",
            Some("esl") => "轻量级文件 (ESL)",
            _ => "未知",
        }
    }
    
    /// 是否为主文件
    pub fn is_master(&self) -> bool {
        self.path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_lowercase() == "esm")
            .unwrap_or(false)
    }
    
    /// 是否本地化
    pub fn is_localized(&self) -> bool {
        self.header.flags & 0x00000080 != 0
    }

    /// 设置 STRING 文件集合（用于外部加载的 STRING 文件）
    ///
    /// 这个方法主要用于 LocalizedPluginContext 将加载的 STRING 文件
    /// 设置到 Plugin 对象中，以便 extract_strings() 等方法可以访问。
    pub fn set_string_files(&mut self, string_files: StringFileSet) {
        self.string_files = Some(string_files);
    }

    /// 是否为轻量插件 (Light Plugin/ESL)
    ///
    /// 检查插件是否为轻量插件，通过以下两种方式之一判断：
    /// 1. 文件扩展名为 .esl
    /// 2. 头部记录的 LightMaster 标志 (0x00000200) 被设置
    ///
    /// 根据 mapping 文档：Python 版本的 `is_light()` 方法
    pub fn is_light(&self) -> bool {
        // 方式1：检查文件扩展名
        if let Some(ext) = self.path.extension() {
            if ext.to_string_lossy().to_lowercase() == "esl" {
                return true;
            }
        }

        // 方式2：检查 LightMaster 标志 (0x00000200)
        const LIGHT_MASTER_FLAG: u32 = 0x00000200;
        (self.header.flags & LIGHT_MASTER_FLAG) != 0
    }
    
    /// 获取统计信息
    pub fn get_stats(&self) -> PluginStats {
        let strings = self.extract_strings();
        
        PluginStats {
            name: self.get_name().to_string(),
            plugin_type: self.get_type().to_string(),
            is_master: self.is_master(),
            is_localized: self.is_localized(),
            master_count: self.masters.len(),
            group_count: self.count_total_groups(),
            record_count: self.count_records(),
            string_count: strings.len(),
        }
    }
    
    /// 统计记录数量
    fn count_records(&self) -> usize {
        1 + self.groups.iter().map(|g| self.count_group_records(g)).sum::<usize>()
    }
    
    /// 统计组中的记录数量
    #[allow(clippy::only_used_in_recursion)]
    fn count_group_records(&self, group: &Group) -> usize {
        group.children.iter().map(|child| match child {
            GroupChild::Group(subgroup) => self.count_group_records(subgroup),
            GroupChild::Record(_) => 1,
        }).sum()
    }
    
    /// 统计总组数
    fn count_total_groups(&self) -> usize {
        self.groups.len() + self.groups.iter().map(|g| self.count_subgroups(g)).sum::<usize>()
    }
    
    /// 统计子组数量
    #[allow(clippy::only_used_in_recursion)]
    fn count_subgroups(&self, group: &Group) -> usize {
        group.children.iter().map(|child| match child {
            GroupChild::Group(subgroup) => 1 + self.count_subgroups(subgroup),
            GroupChild::Record(_) => 0,
        }).sum()
    }

    /// 重编号 FormID 以符合 ESL (Light Plugin) 规范
    ///
    /// 将插件中所有记录的 FormID 重新编号，从 0x800 开始，适用于轻量插件。
    /// 仅修改属于当前插件的记录（非来自外部主文件的记录）。
    ///
    /// # ESL 限制
    /// - 最多支持 2048 (0x800) 个记录
    /// - FormID 的低12位 (0x000-0xFFF) 用于记录编号
    ///
    /// # 错误
    /// - 如果记录数超过 2048 个，返回错误
    ///
    /// # 参考
    /// 根据 mapping 文档的 Python 版本 `eslify_formids()` 方法实现
    pub fn eslify_formids(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // 提取所有记录的可变引用
        let mut all_records = Vec::new();
        for group in &mut self.groups {
            Self::extract_group_records_mut(group, &mut all_records);
        }

        // 从 0x800 开始编号
        let mut current_formid = 0x800u32;

        for record in all_records {
            // 获取主文件索引（FormID 高字节）
            let master_index = (record.form_id >> 24) as usize;

            // 仅修改属于当前插件的记录（非外部主文件）
            if master_index >= self.masters.len() {
                // 保留高20位，替换低12位
                let high_bits = record.form_id & 0xFFFFF000;
                let new_formid = high_bits | (current_formid & 0xFFF);

                record.form_id = new_formid;
                record.is_modified = true;

                current_formid += 1;

                // ESL 限制：最多 2048 (0x800) 个记录
                if current_formid > 0xFFF {
                    return Err(format!(
                        "ESL 插件记录数超过限制！最多支持 2048 个记录，当前已处理 {} 个",
                        current_formid - 0x800
                    ).into());
                }
            }
        }

        #[cfg(debug_assertions)]
        println!("ESL FormID 重编号完成：共 {} 个记录", current_formid - 0x800);

        Ok(())
    }

    /// 递归提取组中所有记录的可变引用
    fn extract_group_records_mut<'a>(
        group: &'a mut Group,
        records: &mut Vec<&'a mut Record>
    ) {
        for child in &mut group.children {
            match child {
                GroupChild::Record(record) => records.push(record),
                GroupChild::Group(nested_group) => {
                    Self::extract_group_records_mut(nested_group, records);
                }
            }
        }
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

        // 获取支持的字符串子记录类型
        let valid_subrecord_types = self.string_records.get(&record.record_type);

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
        // 使用引用而非克隆（性能优化 ~5-10ms）
        let string_records = &self.string_records;
        let masters = &self.masters;
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
                string_records,
                masters,
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

    /// 写入文件
    pub fn write_to_file(&self, path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let mut output = Vec::new();
        
        self.write_record(&self.header, &mut output)?;
        
        for group in &self.groups {
            self.write_group(group, &mut output)?;
        }
        
        std::fs::write(path, output)?;
        Ok(())
    }
    
    /// 写入记录
    pub(crate) fn write_record(&self, record: &Record, output: &mut Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
        use crate::datatypes::RecordFlags;
        use std::borrow::Cow;

        // 写入记录类型
        output.extend_from_slice(&record.record_type_bytes);

        // 判断记录是否原本就是压缩的
        let is_originally_compressed = record.flags & RecordFlags::COMPRESSED.bits() != 0;

        // 处理数据部分（使用 Cow 避免不必要的克隆，性能优化 ~500-800ms）
        let data_to_write: Cow<[u8]> = if record.is_modified {
            // 如果记录被修改，重新序列化子记录（需要新分配）
            let mut subrecord_data = Vec::new();
            for subrecord in &record.subrecords {
                subrecord_data.extend_from_slice(&subrecord.record_type_bytes);
                subrecord_data.extend_from_slice(&(subrecord.data.len() as u16).to_le_bytes());
                subrecord_data.extend_from_slice(&subrecord.data);
            }

            // 如果原本是压缩的，重新压缩
            if is_originally_compressed {
                let compressed = record.recompress_data()?;
                #[cfg(debug_assertions)]
                println!("🔄 重新压缩记录 {}: 解压大小 {} -> 压缩大小 {}",
                    record.record_type, subrecord_data.len(), compressed.len());
                Cow::Owned(compressed)
            } else {
                #[cfg(debug_assertions)]
                println!("📝 修改非压缩记录 {}: 大小 {}", record.record_type, subrecord_data.len());
                Cow::Owned(subrecord_data)
            }
        } else {
            // 使用原始数据（零拷贝借用，避免 35MB 内存拷贝）
            if let Some(compressed_data) = &record.original_compressed_data {
                #[cfg(debug_assertions)]
                println!("📦 保持压缩记录 {}: 原始压缩大小 {} (原始data_size: {})",
                    record.record_type, compressed_data.len(), record.data_size);
                Cow::Borrowed(compressed_data.as_slice())
            } else {
                #[cfg(debug_assertions)]
                println!("📄 保持未压缩记录 {}: 大小 {} (原始data_size: {})",
                    record.record_type, record.raw_data.len(), record.data_size);
                Cow::Borrowed(&record.raw_data)
            }
        };
        
        // 写入数据大小
        let actual_size = data_to_write.len() as u32;
        output.extend_from_slice(&actual_size.to_le_bytes());
        
        #[cfg(debug_assertions)]
        if actual_size != record.data_size && !record.is_modified {
            println!("⚠️  记录 {} 大小不匹配: 写入 {} vs 原始 {}", 
                record.record_type, actual_size, record.data_size);
        }
        
        // 写入其他头部字段
        output.extend_from_slice(&record.flags.to_le_bytes());
        output.extend_from_slice(&record.form_id.to_le_bytes());
        output.extend_from_slice(&record.timestamp.to_le_bytes());
        output.extend_from_slice(&record.version_control_info.to_le_bytes());
        output.extend_from_slice(&record.internal_version.to_le_bytes());
        output.extend_from_slice(&record.unknown.to_le_bytes());
        
        // 写入数据部分
        output.extend_from_slice(&data_to_write);
        
        Ok(())
    }
    
    /// 写入组
    pub(crate) fn write_group(&self, group: &Group, output: &mut Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
        // 写入组头部
        output.extend_from_slice(b"GRUP");
        
        // 临时占位符，稍后计算实际大小
        let size_pos = output.len();
        output.extend_from_slice(&[0u8; 4]);
        
        output.extend_from_slice(&group.label);
        output.extend_from_slice(&group.group_type.to_i32().to_le_bytes());
        output.extend_from_slice(&group.timestamp.to_le_bytes());
        output.extend_from_slice(&group.version_control_info.to_le_bytes());
        output.extend_from_slice(&group.unknown.to_le_bytes());
        
        // 写入子元素
        for child in &group.children {
            match child {
                GroupChild::Group(subgroup) => {
                    self.write_group(subgroup, output)?;
                }
                GroupChild::Record(record) => {
                    self.write_record(record, output)?;
                }
            }
        }
        
        // 计算并写入实际大小（需要包含"GRUP"的4字节）
        let actual_size = (output.len() - size_pos + 4) as u32;
        let size_bytes = actual_size.to_le_bytes();
        output[size_pos..size_pos + 4].copy_from_slice(&size_bytes);
        
        Ok(())
    }
}

/// 插件统计信息
pub struct PluginStats {
    pub name: String,
    pub plugin_type: String,
    pub is_master: bool,
    pub is_localized: bool,
    pub master_count: usize,
    pub group_count: usize,
    pub record_count: usize,
    pub string_count: usize,
}

impl std::fmt::Display for PluginStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "=== 插件统计信息 ===")?;
        writeln!(f, "名称: {}", self.name)?;
        writeln!(f, "类型: {}", self.plugin_type)?;
        writeln!(f, "主文件: {}", if self.is_master { "是" } else { "否" })?;
        writeln!(f, "本地化: {}", if self.is_localized { "是" } else { "否" })?;
        writeln!(f, "依赖主文件数: {}", self.master_count)?;
        writeln!(f, "组数量: {}", self.group_count)?;
        writeln!(f, "记录数量: {}", self.record_count)?;
        writeln!(f, "可翻译字符串数: {}", self.string_count)?;
        Ok(())
    }
}

/// 对组应用翻译
fn apply_translations_to_group(
    group: &mut Group,
    translations: &HashMap<String, ExtractedString>,
    string_records: &HashMap<String, Vec<String>>,
    masters: &[String],
    plugin_name: &str,
) -> Result<usize, Box<dyn std::error::Error>> {
    let mut count = 0;
    for child in &mut group.children {
        match child {
            GroupChild::Group(subgroup) => {
                count += apply_translations_to_group(subgroup, translations, string_records, masters, plugin_name)?;
            }
            GroupChild::Record(record) => {
                count += apply_translations_to_record(record, translations, string_records, masters, plugin_name)?;
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
    string_records: &HashMap<String, Vec<String>>,
    masters: &[String],
    plugin_name: &str,
) -> Result<usize, Box<dyn std::error::Error>> {
    let string_types = match string_records.get(&record.record_type) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_info_routes_to_ilstrings() {
        let file_type = Plugin::determine_string_file_type("INFO", "NAM1");
        assert_eq!(file_type, StringFileType::ILSTRINGS,
            "INFO记录应该路由到ILSTRINGS（对话信息）");
    }

    #[test]
    fn test_desc_routes_to_dlstrings() {
        // 任何record的DESC都应该路由到DLSTRINGS
        let file_type = Plugin::determine_string_file_type("PERK", "DESC");
        assert_eq!(file_type, StringFileType::DLSTRINGS,
            "PERK DESC应该路由到DLSTRINGS");

        let file_type = Plugin::determine_string_file_type("WEAP", "DESC");
        assert_eq!(file_type, StringFileType::DLSTRINGS,
            "WEAP DESC应该路由到DLSTRINGS");

        let file_type = Plugin::determine_string_file_type("MESG", "DESC");
        assert_eq!(file_type, StringFileType::DLSTRINGS,
            "MESG DESC应该路由到DLSTRINGS");
    }

    #[test]
    fn test_cnam_routes_to_dlstrings() {
        // 任何record的CNAM都应该路由到DLSTRINGS
        let file_type = Plugin::determine_string_file_type("QUST", "CNAM");
        assert_eq!(file_type, StringFileType::DLSTRINGS,
            "QUST CNAM应该路由到DLSTRINGS");

        let file_type = Plugin::determine_string_file_type("BOOK", "CNAM");
        assert_eq!(file_type, StringFileType::DLSTRINGS,
            "BOOK CNAM应该路由到DLSTRINGS");
    }

    #[test]
    fn test_full_routes_to_strings() {
        // FULL应该路由到STRINGS
        let file_type = Plugin::determine_string_file_type("WEAP", "FULL");
        assert_eq!(file_type, StringFileType::STRINGS,
            "WEAP FULL应该路由到STRINGS");

        let file_type = Plugin::determine_string_file_type("PERK", "FULL");
        assert_eq!(file_type, StringFileType::STRINGS,
            "PERK FULL应该路由到STRINGS");

        let file_type = Plugin::determine_string_file_type("DIAL", "FULL");
        assert_eq!(file_type, StringFileType::STRINGS,
            "DIAL FULL应该路由到STRINGS");
    }

    #[test]
    fn test_other_subrecords_route_to_strings() {
        // NNAM, DNAM, SHRT等其他subrecord应该路由到STRINGS
        let file_type = Plugin::determine_string_file_type("QUST", "NNAM");
        assert_eq!(file_type, StringFileType::STRINGS,
            "QUST NNAM应该路由到STRINGS");

        let file_type = Plugin::determine_string_file_type("MGEF", "DNAM");
        assert_eq!(file_type, StringFileType::STRINGS,
            "MGEF DNAM应该路由到STRINGS");

        let file_type = Plugin::determine_string_file_type("NPC_", "SHRT");
        assert_eq!(file_type, StringFileType::STRINGS,
            "NPC_ SHRT应该路由到STRINGS");

        let file_type = Plugin::determine_string_file_type("ACTI", "RNAM");
        assert_eq!(file_type, StringFileType::STRINGS,
            "ACTI RNAM应该路由到STRINGS");

        let file_type = Plugin::determine_string_file_type("MESG", "ITXT");
        assert_eq!(file_type, StringFileType::STRINGS,
            "MESG ITXT应该路由到STRINGS（不是DESC/CNAM）");
    }
}

