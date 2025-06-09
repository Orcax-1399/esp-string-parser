use std::collections::HashMap;
use std::fs;
use std::io::{Cursor, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use crate::datatypes::read_u32;
use crate::utils::EspError;

/// Bethesda字符串文件类型
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
        let content_size = self.raw_data.len() as u32;
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
        for i in 0..string_count {
            let directory_address = 8 + (i * 8) as u64;
            cursor.seek(SeekFrom::Start(directory_address))?;
            
            let string_id = read_u32(&mut cursor)?;
            let relative_offset = read_u32(&mut cursor)?;
            let absolute_offset = string_data_start + relative_offset as u64;
            
            if absolute_offset >= data.len() as u64 {
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
    pub fn load_from_directory(directory: &Path, plugin_name: &str, language: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let mut set = StringFileSet::new(plugin_name.to_string(), language.to_string());
        
        for file_type in [StringFileType::STRINGS, StringFileType::ILSTRINGS, StringFileType::DLSTRINGS] {
            let filename = format!("{}_{}.{}", plugin_name, language, file_type.to_extension());
            let filepath = directory.join(filename);
            
            if filepath.exists() {
                let string_file = StringFile::new(filepath)?;
                set.files.insert(file_type, string_file);
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
} 