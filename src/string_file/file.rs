use std::collections::HashMap;
use std::fs;
use std::io::{Cursor, Seek, SeekFrom};
use std::path::PathBuf;

use crate::datatypes::read_u32;
use crate::utils::EspError;

use super::io::parse_filename;
use super::{StringEntry, StringFileStats, StringFileType};

/// Bethesda字符串文件解析器
#[derive(Debug, Clone)]
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
        let (plugin_name, language, file_type) = parse_filename(&path)?;

        if !path.exists() {
            return Err(format!("字符串文件不存在: {:?}", path).into());
        }

        let data = fs::read(&path)?;
        let entries = Self::parse_bytes(&data, &file_type)?;

        Ok(StringFile {
            path,
            file_type,
            language,
            plugin_name,
            entries,
        })
    }

    /// 从内存字节数组创建字符串文件实例
    ///
    /// # 参数
    /// * `data` - STRING 文件的字节数据
    /// * `plugin_name` - 插件名称（例如："Skyrim"）
    /// * `language` - 语言标识（例如："english", "chinese"）
    /// * `file_type` - STRING 文件类型（STRINGS/DLSTRINGS/ILSTRINGS）
    pub fn from_bytes(
        data: &[u8],
        plugin_name: String,
        language: String,
        file_type: StringFileType,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let entries = Self::parse_bytes(data, &file_type)?;

        // 使用虚拟路径（内存加载时没有实际路径）
        let path = PathBuf::from(format!(
            "<memory>:{}_{}.{}",
            plugin_name,
            language,
            file_type.to_extension()
        ));

        Ok(StringFile {
            path,
            file_type,
            language,
            plugin_name,
            entries,
        })
    }

    /// 解析字符串文件字节数据
    fn parse_bytes(
        data: &[u8],
        file_type: &StringFileType,
    ) -> Result<HashMap<u32, StringEntry>, Box<dyn std::error::Error>> {
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
            let (content, raw_data, length) = Self::read_string_data(&mut cursor, file_type, data)?;

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
            println!(
                "[parse_bytes] 警告：跳过了 {} 个无效偏移量的字符串（文件头声明{}个，实际解析{}个）",
                skipped_count,
                string_count,
                entries.len()
            );
        }

        Ok(entries)
    }

    /// 读取字符串数据
    #[allow(clippy::type_complexity)]
    fn read_string_data(
        cursor: &mut Cursor<&[u8]>,
        file_type: &StringFileType,
        data: &[u8],
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

            let null_pos = remaining_data
                .iter()
                .position(|&b| b == 0)
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
        let total_content_size: usize = self.entries.values().map(|entry| entry.content.len()).sum();

        let total_raw_size: usize = self.entries.values().map(|entry| entry.raw_data.len()).sum();

        StringFileStats {
            plugin_name: self.plugin_name.clone(),
            language: self.language.clone(),
            file_type: self.file_type,
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
        self.entries
            .values()
            .filter(|entry| entry.content.contains(text))
            .collect()
    }

    /// 更新字符串内容
    pub fn update_string(&mut self, id: u32, new_content: String) -> Result<(), EspError> {
        if let Some(entry) = self.entries.get_mut(&id) {
            entry.content = new_content.clone();
            entry.raw_data = new_content.as_bytes().to_vec();
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
        use crate::datatypes::write_u32;
        use std::io::Write;

        let mut buffer = Vec::new();

        // 1. 写入文件头（8字节）
        // 字符串数量
        let count = self.entries.len() as u32;
        write_u32(&mut buffer, count)?;

        // 计算数据区总大小
        let data_size: u32 = self
            .entries
            .values()
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
