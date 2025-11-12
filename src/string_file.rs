use std::collections::HashMap;
use std::fs;
use std::io::{Cursor, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use crate::datatypes::read_u32;
use crate::utils::EspError;

/// Bethesda字符串文件类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StringFileType {
    /// 对话字符串文件
    DLSTRINGS,
    /// 界面字符串文件
    ILSTRINGS,
    /// 一般字符串文件
    STRINGS,
}

impl StringFileType {
    /// 从文件扩展名获取字符串文件类型
    pub fn from_extension(extension: &str) -> Option<Self> {
        match extension.to_uppercase().as_str() {
            "DLSTRINGS" => Some(StringFileType::DLSTRINGS),
            "ILSTRINGS" => Some(StringFileType::ILSTRINGS),
            "STRINGS" => Some(StringFileType::STRINGS),
            _ => None,
        }
    }
    
    /// 获取文件扩展名
    pub fn to_extension(&self) -> &'static str {
        match self {
            StringFileType::DLSTRINGS => "DLSTRINGS",
            StringFileType::ILSTRINGS => "ILSTRINGS",
            StringFileType::STRINGS => "STRINGS",
        }
    }
    
    /// 检查是否需要长度前缀（DLSTRINGS和ILSTRINGS需要）
    pub fn has_length_prefix(&self) -> bool {
        matches!(self, StringFileType::DLSTRINGS | StringFileType::ILSTRINGS)
    }
}

/// 字符串数据对象
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StringEntry {
    /// 字符串ID
    pub id: u32,
    /// 目录条目在文件中的位置
    pub directory_address: u64,
    /// 相对偏移量
    pub relative_offset: u32,
    /// 绝对偏移量（字符串数据起始位置）
    pub absolute_offset: u64,
    /// 字符串长度（仅对DLSTRINGS/ILSTRINGS有效）
    pub length: Option<u32>,
    /// 字符串内容（UTF-8编码）
    pub content: String,
    /// 原始字节数据
    pub raw_data: Vec<u8>,
}

impl StringEntry {
    /// 创建新的字符串条目
    pub fn new(id: u32, content: String) -> Self {
        let raw_data = content.as_bytes().to_vec();
        Self {
            id,
            directory_address: 0,
            relative_offset: 0,
            absolute_offset: 0,
            length: Some(raw_data.len() as u32),
            content,
            raw_data,
        }
    }
    
    /// 获取字符串的总大小（包括长度前缀和空终止符）
    pub fn get_total_size(&self, file_type: &StringFileType) -> u32 {
        // 使用content的实际字节长度，而不是raw_data，确保一致性
        let content_size = self.content.as_bytes().len() as u32;
        let null_terminator = 1u32; // 空终止符

        if file_type.has_length_prefix() {
            4 + content_size + null_terminator // 长度前缀(4) + 内容 + 空终止符
        } else {
            content_size + null_terminator // 内容 + 空终止符
        }
    }
}

/// Bethesda字符串文件解析器
pub struct StringFile {
    /// 文件路径
    pub path: PathBuf,
    /// 文件类型
    pub file_type: StringFileType,
    /// 语言标识符
    pub language: String,
    /// 关联的插件名称
    pub plugin_name: String,
    /// 字符串条目映射（ID -> StringEntry）
    pub entries: HashMap<u32, StringEntry>,
}

impl StringFile {
    /// 从文件路径创建新的字符串文件实例
    pub fn new(path: PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let (plugin_name, language, file_type) = Self::parse_filename(&path)?;
        
        if !path.exists() {
            return Err(format!("字符串文件不存在: {:?}", path).into());
        }
        
        let entries = Self::parse_file(&path, &file_type)?;
        
        Ok(StringFile {
            path,
            file_type,
            language,
            plugin_name,
            entries,
        })
    }
    

    
    /// 解析文件名获取插件名、语言和文件类型
    fn parse_filename(path: &Path) -> Result<(String, String, StringFileType), Box<dyn std::error::Error>> {
        let filename = path.file_stem()
            .and_then(|s| s.to_str())
            .ok_or("无效的文件名")?;
        
        let extension = path.extension()
            .and_then(|s| s.to_str())
            .ok_or("无效的文件扩展名")?;
        
        let file_type = StringFileType::from_extension(extension)
            .ok_or("不支持的字符串文件类型")?;
        
        // 解析文件名格式：PluginName_Language
        let parts: Vec<&str> = filename.rsplitn(2, '_').collect();
        if parts.len() != 2 {
            return Err("文件名格式错误，应为：PluginName_Language.EXTENSION".into());
        }
        
        let language = parts[0].to_string();
        let plugin_name = parts[1].to_string();
        
        Ok((plugin_name, language, file_type))
    }
    
    /// 解析字符串文件
    fn parse_file(path: &Path, file_type: &StringFileType) -> Result<HashMap<u32, StringEntry>, Box<dyn std::error::Error>> {
        let data = fs::read(path)?;
        if data.len() < 8 {
            return Err(EspError::InvalidFormat.into());
        }

        let mut cursor = Cursor::new(&data[..]);

        // 读取文件头（8字节）
        let string_count = read_u32(&mut cursor)?;
        let _data_size = read_u32(&mut cursor)?;

        if string_count == 0 {
            return Ok(HashMap::new());
        }

        // 计算字符串数据的起始位置
        let string_data_start = 8 + (string_count * 8) as u64;

        // 读取目录条目
        let mut entries = HashMap::new();
        #[cfg(debug_assertions)]
        let mut skipped_count = 0;

        for i in 0..string_count {
            let directory_address = 8 + (i * 8) as u64;
            cursor.seek(SeekFrom::Start(directory_address))?;

            let string_id = read_u32(&mut cursor)?;
            let relative_offset = read_u32(&mut cursor)?;
            let absolute_offset = string_data_start + relative_offset as u64;

            if absolute_offset >= data.len() as u64 {
                #[cfg(debug_assertions)]
                {
                    skipped_count += 1;
                }
                continue; // 跳过无效的偏移量
            }
            
            // 读取字符串数据
            cursor.seek(SeekFrom::Start(absolute_offset))?;
            let (content, raw_data, length) = Self::read_string_data(&mut cursor, file_type, &data)?;
            
            let entry = StringEntry {
                id: string_id,
                directory_address,
                relative_offset,
                absolute_offset,
                length,
                content,
                raw_data,
            };
            
            entries.insert(string_id, entry);
        }

        #[cfg(debug_assertions)]
        if skipped_count > 0 {
            println!("[parse_file] 警告：跳过了 {} 个无效偏移量的字符串（文件头声明{}个，实际解析{}个）",
                skipped_count, string_count, entries.len());
        }

        Ok(entries)
    }
    
    /// 读取字符串数据
    fn read_string_data(
        cursor: &mut Cursor<&[u8]>, 
        file_type: &StringFileType,
        data: &[u8]
    ) -> Result<(String, Vec<u8>, Option<u32>), Box<dyn std::error::Error>> {
        let start_pos = cursor.position() as usize;
        
        if file_type.has_length_prefix() {
            // DLSTRINGS/ILSTRINGS: 先读取长度字段
            let length = read_u32(cursor)?;
            let content_start = cursor.position() as usize;
            
            if content_start + length as usize > data.len() {
                return Err("字符串长度超出文件边界".into());
            }
            
            // 读取字符串内容（不包括空终止符）
            let string_bytes = &data[content_start..content_start + length as usize];
            
            // 查找空终止符
            let null_pos = string_bytes.iter().position(|&b| b == 0);
            let actual_string_bytes = if let Some(pos) = null_pos {
                &string_bytes[..pos]
            } else {
                string_bytes
            };
            
            let content = String::from_utf8_lossy(actual_string_bytes).to_string();
            
            // 原始数据包括长度字段
            let total_size = 4 + length as usize;
            let raw_data = data[start_pos..start_pos + total_size].to_vec();
            
            Ok((content, raw_data, Some(length)))
        } else {
            // STRINGS: 读取到空终止符
            let content_start = cursor.position() as usize;
            let remaining_data = &data[content_start..];
            
            let null_pos = remaining_data.iter().position(|&b| b == 0)
                .ok_or("未找到字符串终止符")?;
            
            let string_bytes = &remaining_data[..null_pos];
            let content = String::from_utf8_lossy(string_bytes).to_string();
            
            // 原始数据包括空终止符
            let raw_data = data[content_start..content_start + null_pos + 1].to_vec();
            
            Ok((content, raw_data, None))
        }
    }
    
    /// 获取字符串条目
    pub fn get_string(&self, id: u32) -> Option<&StringEntry> {
        self.entries.get(&id)
    }
    
    /// 获取所有字符串ID
    pub fn get_string_ids(&self) -> Vec<u32> {
        let mut ids: Vec<u32> = self.entries.keys().cloned().collect();
        ids.sort();
        ids
    }
    
    /// 获取字符串数量
    pub fn count(&self) -> usize {
        self.entries.len()
    }
    
    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    

    
    /// 获取文件统计信息
    pub fn get_stats(&self) -> StringFileStats {
        let total_content_size: usize = self.entries.values()
            .map(|entry| entry.content.len())
            .sum();
        
        let total_raw_size: usize = self.entries.values()
            .map(|entry| entry.raw_data.len())
            .sum();
        
        StringFileStats {
            plugin_name: self.plugin_name.clone(),
            language: self.language.clone(),
            file_type: self.file_type.clone(),
            string_count: self.entries.len(),
            total_content_size,
            total_raw_size,
            average_string_length: if self.entries.is_empty() { 
                0.0 
            } else { 
                total_content_size as f64 / self.entries.len() as f64 
            },
        }
    }
    
    /// 查找包含指定文本的字符串
    pub fn find_strings_containing(&self, text: &str) -> Vec<&StringEntry> {
        self.entries.values()
            .filter(|entry| entry.content.contains(text))
            .collect()
    }

    /// 更新字符串内容
    pub fn update_string(&mut self, id: u32, new_content: String) -> Result<(), EspError> {
        if let Some(entry) = self.entries.get_mut(&id) {
            // 更新内容
            entry.content = new_content.clone();
            // 更新原始字节数据（UTF-8编码）
            entry.raw_data = new_content.as_bytes().to_vec();
            // 更新长度
            entry.length = Some(entry.raw_data.len() as u32);
            Ok(())
        } else {
            Err(EspError::InvalidFormat)
        }
    }

    /// 批量更新字符串
    pub fn update_strings(&mut self, updates: HashMap<u32, String>) -> Result<(), EspError> {
        for (id, content) in updates {
            self.update_string(id, content)?;
        }
        Ok(())
    }

    /// 添加新字符串
    pub fn add_string(&mut self, id: u32, content: String) -> Result<(), EspError> {
        if self.entries.contains_key(&id) {
            return Err(EspError::InvalidFormat);
        }

        let entry = StringEntry::new(id, content);
        self.entries.insert(id, entry);
        Ok(())
    }

    /// 删除字符串
    pub fn remove_string(&mut self, id: u32) -> Option<StringEntry> {
        self.entries.remove(&id)
    }

    /// 重建STRING文件的二进制数据
    pub fn rebuild(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        use crate::datatypes::{write_u32};
        use std::io::Write;

        let mut buffer = Vec::new();

        // 1. 写入文件头（8字节）
        // 字符串数量
        let count = self.entries.len() as u32;
        write_u32(&mut buffer, count)?;

        // 计算数据区总大小
        let data_size: u32 = self.entries.values()
            .map(|e| e.get_total_size(&self.file_type))
            .sum();
        write_u32(&mut buffer, data_size)?;

        // 2. 准备排序的ID列表和目录条目
        let mut ids: Vec<u32> = self.entries.keys().cloned().collect();
        ids.sort();

        #[cfg(debug_assertions)]
        println!("[rebuild] 准备写入 {} 个字符串", ids.len());

        // 计算每个字符串的偏移量
        let mut offset = 0u32;
        let mut directory_entries = Vec::new();

        for id in &ids {
            directory_entries.push((*id, offset));
            let entry = &self.entries[id];
            let size = entry.get_total_size(&self.file_type);
            offset += size;
        }

        // 3. 写入目录条目（每个8字节：ID + 偏移量）
        for (id, offset) in &directory_entries {
            write_u32(&mut buffer, *id)?;
            write_u32(&mut buffer, *offset)?;
        }

        // 4. 写入字符串数据
        for id in &ids {
            let entry = &self.entries[id];

            if self.file_type.has_length_prefix() {
                // DLSTRINGS/ILSTRINGS: 长度前缀 + 内容 + null终止符
                let length = entry.content.len() as u32;
                write_u32(&mut buffer, length)?;
            }

            // 字符串内容（UTF-8编码）
            buffer.write_all(entry.content.as_bytes())?;

            // null终止符
            buffer.push(0);
        }

        #[cfg(debug_assertions)]
        println!("[rebuild] 写入完成，总大小 {} 字节", buffer.len());

        Ok(buffer)
    }

    /// 写入到文件
    pub fn write_to_file(&self, path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let data = self.rebuild()?;
        fs::write(path, data)?;
        Ok(())
    }
}

/// 字符串文件统计信息
#[derive(Debug, Clone)]
pub struct StringFileStats {
    pub plugin_name: String,
    pub language: String,
    pub file_type: StringFileType,
    pub string_count: usize,
    pub total_content_size: usize,
    pub total_raw_size: usize,
    pub average_string_length: f64,
}

impl std::fmt::Display for StringFileStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "=== 字符串文件统计 ===")?;
        writeln!(f, "插件名称: {}", self.plugin_name)?;
        writeln!(f, "语言: {}", self.language)?;
        writeln!(f, "文件类型: {:?}", self.file_type)?;
        writeln!(f, "字符串数量: {}", self.string_count)?;
        writeln!(f, "内容总大小: {} 字节", self.total_content_size)?;
        writeln!(f, "原始数据大小: {} 字节", self.total_raw_size)?;
        writeln!(f, "平均字符串长度: {:.1} 字符", self.average_string_length)?;
        Ok(())
    }
}

/// 字符串文件集合管理器
pub struct StringFileSet {
    /// 字符串文件映射 (文件类型 -> StringFile)
    pub files: HashMap<StringFileType, StringFile>,
    /// 插件名称
    pub plugin_name: String,
    /// 语言
    pub language: String,
}

impl StringFileSet {
    /// 为指定插件和语言创建字符串文件集合  
    pub fn new(plugin_name: String, language: String) -> Self {
        StringFileSet {
            files: HashMap::new(),
            plugin_name,
            language,
        }
    }
    
    /// 加载指定目录下的所有字符串文件
    ///
    /// 支持大小写不敏感的文件名匹配，会尝试以下变体：
    /// - 原始名称
    /// - 全小写
    /// - 全大写
    pub fn load_from_directory(directory: &Path, plugin_name: &str, language: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let mut set = StringFileSet::new(plugin_name.to_string(), language.to_string());

        for file_type in [StringFileType::STRINGS, StringFileType::ILSTRINGS, StringFileType::DLSTRINGS] {
            // 尝试多种文件名变体（支持大小写不敏感）
            let name_variants = vec![
                plugin_name.to_string(),                // 原始名称
                plugin_name.to_lowercase(),             // 全小写
                plugin_name.to_uppercase(),             // 全大写
            ];

            let mut found = false;
            for name_variant in name_variants {
                let filename = format!("{}_{}.{}", name_variant, language, file_type.to_extension());
                let filepath = directory.join(&filename);

                if filepath.exists() {
                    let string_file = StringFile::new(filepath)?;
                    set.files.insert(file_type, string_file);
                    found = true;
                    break;
                }

                // 也尝试扩展名小写的版本
                let filename_lower_ext = format!("{}_{}.{}",
                    name_variant,
                    language,
                    file_type.to_extension().to_lowercase()
                );
                let filepath_lower = directory.join(&filename_lower_ext);

                if filepath_lower.exists() {
                    let string_file = StringFile::new(filepath_lower)?;
                    set.files.insert(file_type, string_file);
                    found = true;
                    break;
                }
            }

            #[cfg(debug_assertions)]
            if !found {
                eprintln!("提示: 未找到 {:?} 文件（尝试了多种大小写变体）", file_type);
            }
        }

        Ok(set)
    }
    
    /// 获取指定类型的字符串文件
    pub fn get_file(&self, file_type: &StringFileType) -> Option<&StringFile> {
        self.files.get(file_type)
    }
    
    /// 获取指定类型的字符串文件（可变引用）
    pub fn get_file_mut(&mut self, file_type: &StringFileType) -> Option<&mut StringFile> {
        self.files.get_mut(file_type)
    }
    
    /// 添加字符串文件
    pub fn add_file(&mut self, file_type: StringFileType, string_file: StringFile) {
        self.files.insert(file_type, string_file);
    }
    
    /// 获取字符串（按优先级查找：STRINGS > ILSTRINGS > DLSTRINGS）
    pub fn get_string(&self, id: u32) -> Option<&StringEntry> {
        for file_type in [StringFileType::STRINGS, StringFileType::ILSTRINGS, StringFileType::DLSTRINGS] {
            if let Some(file) = self.files.get(&file_type) {
                if let Some(entry) = file.get_string(id) {
                    return Some(entry);
                }
            }
        }
        None
    }
    

    
    /// 获取总的字符串数量  
    pub fn total_count(&self) -> usize {
        self.files.values().map(|f| f.count()).sum()
    }
    
    /// 获取所有字符串ID
    pub fn get_all_string_ids(&self) -> Vec<u32> {
        let mut all_ids = std::collections::HashSet::new();
        for file in self.files.values() {
            for id in file.get_string_ids() {
                all_ids.insert(id);
            }
        }
        let mut ids: Vec<u32> = all_ids.into_iter().collect();
        ids.sort();
        ids
    }

    /// 从指定类型的文件中获取字符串
    pub fn get_string_by_type(&self, file_type: StringFileType, id: u32) -> Option<&StringEntry> {
        self.files.get(&file_type)?.get_string(id)
    }

    /// 更新指定类型文件中的字符串
    pub fn update_string(&mut self, file_type: StringFileType, id: u32, new_content: String) -> Result<(), EspError> {
        if let Some(file) = self.files.get_mut(&file_type) {
            file.update_string(id, new_content)
        } else {
            Err(EspError::InvalidFormat)
        }
    }

    /// 批量更新字符串（指定文件类型）
    pub fn update_strings(&mut self, file_type: StringFileType, updates: HashMap<u32, String>) -> Result<(), EspError> {
        if let Some(file) = self.files.get_mut(&file_type) {
            file.update_strings(updates)
        } else {
            Err(EspError::InvalidFormat)
        }
    }

    /// 批量应用翻译（自动识别文件类型）
    pub fn apply_translations(&mut self, translations: &HashMap<(StringFileType, u32), String>) -> Result<(), Box<dyn std::error::Error>> {
        for ((file_type, id), content) in translations {
            self.update_string(*file_type, *id, content.clone())?;
        }
        Ok(())
    }

    /// 写入所有STRING文件到指定目录
    pub fn write_all(&self, directory: &Path) -> Result<(), Box<dyn std::error::Error>> {
        for (file_type, file) in &self.files {
            let filename = format!("{}_{}.{}",
                self.plugin_name,
                self.language,
                file_type.to_extension()
            );
            let filepath = directory.join(filename);

            // 创建备份
            if filepath.exists() {
                let backup_path = crate::utils::create_backup(&filepath)?;
                println!("已创建备份: {:?}", backup_path);
            }

            file.write_to_file(filepath)?;
        }
        Ok(())
    }

    /// 写入单个STRING文件
    pub fn write_file(&self, file_type: StringFileType, directory: &Path) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(file) = self.files.get(&file_type) {
            let filename = format!("{}_{}.{}",
                self.plugin_name,
                self.language,
                file_type.to_extension()
            );
            let filepath = directory.join(filename);

            // 创建备份
            if filepath.exists() {
                let backup_path = crate::utils::create_backup(&filepath)?;
                println!("已创建备份: {:?}", backup_path);
            }

            file.write_to_file(filepath)?;
            Ok(())
        } else {
            Err("指定的STRING文件类型不存在".into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

        // 测试更新现有字符串
        assert!(file.update_string(1, "铁剑".to_string()).is_ok());
        assert_eq!(file.get_string(1).unwrap().content, "铁剑");

        // 测试更新不存在的字符串
        assert!(file.update_string(999, "不存在".to_string()).is_err());
    }

    #[test]
    fn test_add_string() {
        let mut file = create_test_string_file();

        // 测试添加新字符串
        assert!(file.add_string(200, "新物品".to_string()).is_ok());
        assert_eq!(file.get_string(200).unwrap().content, "新物品");

        // 测试添加已存在的ID
        assert!(file.add_string(1, "重复".to_string()).is_err());
    }

    #[test]
    fn test_remove_string() {
        let mut file = create_test_string_file();

        // 测试删除字符串
        assert!(file.remove_string(1).is_some());
        assert!(file.get_string(1).is_none());

        // 测试删除不存在的字符串
        assert!(file.remove_string(999).is_none());
    }

    #[test]
    fn test_rebuild_strings() {
        let file = create_test_string_file();

        // 测试重建二进制数据
        let result = file.rebuild();
        assert!(result.is_ok());

        let data = result.unwrap();

        // 验证文件头（前8字节）
        assert!(data.len() > 8);

        // 读取字符串数量
        let count = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        assert_eq!(count, 3); // 我们创建了3个字符串
    }

    #[test]
    fn test_write_and_read_roundtrip() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("TestMod_english.STRINGS");

        // 创建并写入文件
        let mut original_file = create_test_string_file();
        original_file.update_string(1, "测试中文".to_string()).unwrap();

        assert!(original_file.write_to_file(file_path.clone()).is_ok());

        // 读取并验证
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
        // DLSTRINGS应该包含长度前缀，所以数据会更大
        assert!(data.len() > 8);
    }

    #[test]
    fn test_string_file_set_update() {
        let mut set = StringFileSet::new("TestMod".to_string(), "english".to_string());

        // 添加STRINGS文件
        let strings_file = create_test_string_file();
        set.add_file(StringFileType::STRINGS, strings_file);

        // 测试更新
        assert!(set.update_string(StringFileType::STRINGS, 1, "更新的文本".to_string()).is_ok());

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
        // 测试读取真实的Fishing DLC STRING文件
        let test_dir = PathBuf::from("TestFile");

        if !test_dir.exists() {
            println!("跳过测试：TestFile目录不存在");
            return;
        }

        // 测试STRINGS文件
        let strings_path = test_dir.join("ccbgssse001-fish_english.STRINGS");
        if strings_path.exists() {
            let strings_file = StringFile::new(strings_path).unwrap();
            println!("STRINGS文件包含 {} 个字符串", strings_file.count());
            assert!(strings_file.count() > 0);

            // 显示前5个字符串
            let ids = strings_file.get_string_ids();
            for id in ids.iter().take(5) {
                if let Some(entry) = strings_file.get_string(*id) {
                    println!("  [{}] {}", id, entry.content);
                }
            }
        }

        // 测试DLSTRINGS文件
        let dlstrings_path = test_dir.join("ccbgssse001-fish_english.DLSTRINGS");
        if dlstrings_path.exists() {
            let dlstrings_file = StringFile::new(dlstrings_path).unwrap();
            println!("DLSTRINGS文件包含 {} 个字符串", dlstrings_file.count());
            assert!(dlstrings_file.count() > 0);
        }

        // 测试ILSTRINGS文件
        let ilstrings_path = test_dir.join("ccbgssse001-fish_english.ILSTRINGS");
        if ilstrings_path.exists() {
            let ilstrings_file = StringFile::new(ilstrings_path).unwrap();
            println!("ILSTRINGS文件包含 {} 个字符串", ilstrings_file.count());
        }
    }

    #[test]
    fn test_real_file_write_and_reload() {
        // 测试真实文件的读取-修改-写入-重新读取循环
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

        // 读取原始文件
        let mut original_file = StringFile::new(strings_path).unwrap();
        let original_count = original_file.count();
        println!("原始文件包含 {} 个有效字符串", original_count);

        // 获取第一个字符串ID并修改
        let ids = original_file.get_string_ids();
        if let Some(&first_id) = ids.first() {
            let original_text = original_file.get_string(first_id).unwrap().content.clone();
            println!("原始文本 [{}]: {}", first_id, original_text);

            // 修改为中文
            original_file.update_string(first_id, "钓鱼测试".to_string()).unwrap();

            // 写入临时文件
            let temp_dir = TempDir::new().unwrap();
            let temp_path = temp_dir.path().join("ccbgssse001-fish_chinese.STRINGS");
            original_file.write_to_file(temp_path.clone()).unwrap();

            // 重新读取验证
            let reloaded_file = StringFile::new(temp_path).unwrap();
            println!("重新加载后包含 {} 个字符串", reloaded_file.count());

            // 验证数量一致（写入的和读取的应该相同）
            assert_eq!(reloaded_file.count(), original_count,
                "写入前后字符串数量应该一致");

            // 验证修改的字符串
            assert_eq!(reloaded_file.get_string(first_id).unwrap().content, "钓鱼测试",
                "修改的字符串内容应该正确");

            // 验证其他字符串没有被破坏
            if ids.len() > 1 {
                let second_id = ids[1];
                let original_second = original_file.get_string(second_id).unwrap().content.clone();
                let reloaded_second = reloaded_file.get_string(second_id).unwrap().content.clone();
                assert_eq!(original_second, reloaded_second,
                    "未修改的字符串应该保持不变");
            }

            println!("✓ 读写循环测试通过！所有验证成功");
        }
    }
} 