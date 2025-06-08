use crate::datatypes::{read_u32, RawString};
use crate::record::Record;
use crate::group::{Group, GroupChild};
use crate::string_types::ExtractedString;
use crate::utils::{is_valid_string, EspError};
use std::collections::HashMap;
use std::path::PathBuf;
use std::io::Cursor;
use serde_json;

/// ESP插件解析器
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
}

/// 修改信息结构
impl Plugin {
    /// 从翻译文件创建新的ESP文件
    pub fn apply_translations(
        input_path: PathBuf,
        output_path: PathBuf,
        translations: Vec<ExtractedString>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        #[cfg(debug_assertions)]
        let backup_path = crate::utils::create_backup(&input_path)?;
        #[cfg(not(debug_assertions))]
        let _backup_path = crate::utils::create_backup(&input_path)?;
        
        #[cfg(debug_assertions)]
        println!("已创建备份文件: {:?}", backup_path);
        
        let mut plugin = Self::new(input_path)?;
        let translation_map = Self::create_translation_map(translations);
        plugin.apply_translation_map(&translation_map)?;
        plugin.write_to_file(output_path)?;
        
        Ok(())
    }

    /// 创建新的插件实例
    pub fn new(path: PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let string_records = Self::load_string_records()?;
        let data = std::fs::read(&path)?;
        let mut cursor = Cursor::new(&data[..]);
        
        let header = Record::parse(&mut cursor)?;
        Self::validate_esp_file(&header)?;
        
        let masters = Self::extract_masters(&header);
        let groups = Self::parse_groups(&mut cursor, &data)?;
        
        Ok(Plugin {
            path,
            header,
            groups,
            masters,
            string_records,
        })
    }
    
    /// 验证ESP文件格式
    fn validate_esp_file(header: &Record) -> Result<(), Box<dyn std::error::Error>> {
        if !matches!(header.record_type.as_str(), "TES4" | "TES3") {
            return Err(EspError::InvalidFormat.into());
        }
        Ok(())
    }
    
    /// 解析所有组
    fn parse_groups(cursor: &mut Cursor<&[u8]>, data: &[u8]) -> Result<Vec<Group>, Box<dyn std::error::Error>> {
        let mut groups = Vec::new();
        while cursor.position() < data.len() as u64 {
            let group = Group::parse(cursor)?;
            groups.push(group);
        }
        Ok(groups)
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
    
    /// 提取所有字符串
    pub fn extract_strings(&self) -> Vec<ExtractedString> {
        let mut strings = Vec::new();
        for group in &self.groups {
            strings.extend(self.extract_group_strings(group));
        }
        strings
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
    fn extract_record_strings(&self, record: &Record) -> Vec<ExtractedString> {
        let mut strings = Vec::new();
        
        let string_types = match self.string_records.get(&record.record_type) {
            Some(types) => types,
            None => return strings,
        };
        
        let editor_id = record.get_editor_id();
        let form_id_str = self.format_form_id(record.form_id);
        
        for subrecord in &record.subrecords {
            if string_types.contains(&subrecord.record_type) {
                if let Some(extracted) = self.extract_string_from_subrecord(
                    subrecord, &editor_id, &form_id_str, &record.record_type
                ) {
                    strings.push(extracted);
                }
            }
        }
        
        strings
    }
    
    /// 从子记录中提取字符串
    fn extract_string_from_subrecord(
        &self, 
        subrecord: &crate::subrecord::Subrecord, 
        editor_id: &Option<String>,
        form_id_str: &str,
        record_type: &str
    ) -> Option<ExtractedString> {
        let raw_string = if self.header.flags & 0x00000080 != 0 {
            // 本地化插件：数据是字符串ID
            let mut cursor = Cursor::new(&subrecord.data[..]);
            let string_id = read_u32(&mut cursor).unwrap_or(0);
            RawString {
                content: format!("StringID_{}", string_id),
                encoding: "ascii".to_string(),
            }
        } else {
            // 普通插件：直接解析字符串
            RawString::parse_zstring(&subrecord.data)
        };
        
                        if is_valid_string(&raw_string.content) {
                    Some(ExtractedString::new(
                        editor_id.clone(),
                        form_id_str.to_string(),
                        record_type.to_string(),
                        subrecord.record_type.clone(),
                        raw_string.content,
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
    fn count_subgroups(&self, group: &Group) -> usize {
        group.children.iter().map(|child| match child {
            GroupChild::Group(subgroup) => 1 + self.count_subgroups(subgroup),
            GroupChild::Record(_) => 0,
        }).sum()
    }
    
    /// 应用翻译映射
    fn apply_translation_map(&mut self, translations: &HashMap<String, ExtractedString>) -> Result<(), Box<dyn std::error::Error>> {
        let string_records = self.string_records.clone();
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
                &string_records, 
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
    fn write_record(&self, record: &Record, output: &mut Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
        use crate::datatypes::RecordFlags;
        
        // 写入记录类型
        output.extend_from_slice(&record.record_type_bytes);
        
        // 判断记录是否原本就是压缩的
        let is_originally_compressed = record.flags & RecordFlags::COMPRESSED.bits() != 0;
        
        // 处理数据部分
        let data_to_write = if record.is_modified {
            // 如果记录被修改，重新序列化子记录
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
                compressed
            } else {
                #[cfg(debug_assertions)]
                println!("📝 修改非压缩记录 {}: 大小 {}", record.record_type, subrecord_data.len());
                subrecord_data
            }
        } else {
            // 使用原始数据
            if let Some(compressed_data) = &record.original_compressed_data {
                #[cfg(debug_assertions)]
                println!("📦 保持压缩记录 {}: 原始压缩大小 {} (原始data_size: {})", 
                    record.record_type, compressed_data.len(), record.data_size);
                compressed_data.clone()
            } else {
                #[cfg(debug_assertions)]
                println!("📄 保持未压缩记录 {}: 大小 {} (原始data_size: {})", 
                    record.record_type, record.raw_data.len(), record.data_size);
                record.raw_data.clone()
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
    fn write_group(&self, group: &Group, output: &mut Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
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
    
    for subrecord in &mut record.subrecords {
        if string_types.contains(&subrecord.record_type) {
            let string_type = format!("{} {}", record.record_type, subrecord.record_type);
            let key = format!("{}|{}|{}", 
                editor_id.as_deref().unwrap_or(""),
                form_id_str,
                string_type
            );
            
            #[cfg(debug_assertions)]
            println!("尝试匹配键: {}", key);
            
            if let Some(translation) = translations.get(&key) {
                if !translation.original_text.is_empty() {
                    
                    #[cfg(debug_assertions)]
                    println!("✓ 成功应用翻译: [{}] {} -> \"{}\"", 
                        translation.form_id,
                        translation.get_string_type(),
                        if translation.original_text.chars().count() > 50 {
                            format!("{}...", translation.original_text.chars().take(50).collect::<String>())
                        } else {
                            translation.original_text.clone()
                        }
                    );
                    
                    let encoded_data = encode_string_with_encoding(&translation.original_text, "utf-8")?;
                    subrecord.data = encoded_data;
                    subrecord.size = subrecord.data.len() as u16;
                    modified = true;
                    applied_count += 1;
                }
            }
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

