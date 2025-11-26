use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::utils::{create_backup, EspError};
use crate::io::StringFileReader;

use super::file::StringFile;
use super::io::build_filename_variants;
use super::{StringEntry, StringFileType};

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
#[derive(Debug, Clone)]
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

    /// 从内存字节数据创建字符串文件集合
    ///
    /// # 参数
    /// * `files_data` - 各类型 STRING 文件的字节数据映射
    /// * `plugin_name` - 插件名称
    /// * `language` - 语言标识
    pub fn from_memory(
        files_data: HashMap<StringFileType, &[u8]>,
        plugin_name: String,
        language: String,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut set = StringFileSet::new(plugin_name.clone(), language.clone());

        for (file_type, data) in files_data {
            let string_file = StringFile::from_bytes(data, plugin_name.clone(), language.clone(), file_type)?;
            set.files.insert(file_type, string_file);
        }

        Ok(set)
    }

    /// 使用自定义 Reader 加载指定目录下的所有字符串文件（v0.6.0 - P2.4）
    ///
    /// 通过依赖注入支持自定义 IO 实现，便于测试和扩展。
    ///
    /// # 参数
    /// * `directory` - STRING 文件所在目录
    /// * `plugin_name` - 插件名称（不含扩展名）
    /// * `language` - 语言标识（如 "english"）
    /// * `reader` - 实现 StringFileReader trait 的读取器
    ///
    /// # 示例
    /// ```rust,ignore
    /// use esp_extractor::{StringFileSet, DefaultStringFileReader};
    /// let reader = DefaultStringFileReader;
    /// let set = StringFileSet::load_from_directory_with_reader(
    ///     Path::new("Strings"),
    ///     "Skyrim",
    ///     "english",
    ///     &reader
    /// )?;
    /// ```
    pub fn load_from_directory_with_reader(
        directory: &Path,
        plugin_name: &str,
        language: &str,
        reader: &dyn StringFileReader,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut set = StringFileSet::new(plugin_name.to_string(), language.to_string());

        for file_type in [
            StringFileType::STRINGS,
            StringFileType::ILSTRINGS,
            StringFileType::DLSTRINGS,
        ] {
            for filepath in build_filename_variants(directory, plugin_name, language, file_type) {
                if filepath.exists() {
                    let string_file = reader.read(&filepath)?;
                    set.files.insert(file_type, string_file);
                    break;
                }
            }
        }

        Ok(set)
    }

    /// 加载指定目录下的所有字符串文件
    ///
    /// 支持大小写不敏感的文件名匹配，会尝试以下变体：
    /// - 原始名称
    /// - 全小写
    /// - 全大写
    pub fn load_from_directory(
        directory: &Path,
        plugin_name: &str,
        language: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut set = StringFileSet::new(plugin_name.to_string(), language.to_string());

        for file_type in [
            StringFileType::STRINGS,
            StringFileType::ILSTRINGS,
            StringFileType::DLSTRINGS,
        ] {
            for filepath in build_filename_variants(directory, plugin_name, language, file_type) {
                if filepath.exists() {
                    let string_file = StringFile::new(filepath)?;
                    set.files.insert(file_type, string_file);
                    break;
                }
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
        for file_type in [
            StringFileType::STRINGS,
            StringFileType::ILSTRINGS,
            StringFileType::DLSTRINGS,
        ] {
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
        let mut all_ids = HashSet::new();
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
    pub fn apply_translations(
        &mut self,
        translations: &HashMap<(StringFileType, u32), String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for ((file_type, id), content) in translations {
            self.update_string(*file_type, *id, content.clone())?;
        }
        Ok(())
    }

    /// 写入所有STRING文件到指定目录
    pub fn write_all(&self, directory: &Path) -> Result<(), Box<dyn std::error::Error>> {
        for (file_type, file) in &self.files {
            let filename = format!("{}_{}.{}", self.plugin_name, self.language, file_type.to_extension());
            let filepath = directory.join(filename);

            if filepath.exists() {
                let backup_path = create_backup(&filepath)?;
                println!("已创建备份: {:?}", backup_path);
            }

            file.write_to_file(filepath)?;
        }
        Ok(())
    }

    /// 写入单个STRING文件
    pub fn write_file(&self, file_type: StringFileType, directory: &Path) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(file) = self.files.get(&file_type) {
            let filename = format!("{}_{}.{}", self.plugin_name, self.language, file_type.to_extension());
            let filepath = directory.join(filename);

            if filepath.exists() {
                let backup_path = create_backup(&filepath)?;
                println!("已创建备份: {:?}", backup_path);
            }

            file.write_to_file(filepath)?;
            Ok(())
        } else {
            Err("指定的STRING文件类型不存在".into())
        }
    }
}
