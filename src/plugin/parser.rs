use super::Plugin;
use crate::datatypes::{read_u32, RawString};
use crate::record::Record;
use crate::group::Group;
use crate::string_file::StringFileSet;
use crate::string_routes::DefaultStringRouter;
use crate::io::EspReader;
use crate::utils::EspError;
use std::collections::HashMap;
use std::path::PathBuf;
use std::io::{Cursor, Read};
use std::sync::Arc;
use serde_json;
use memmap2::Mmap;
use rayon::prelude::*;

impl Plugin {
    /// 使用自定义 Reader 加载插件文件（v0.6.0+ 新增 - P2.4）
    ///
    /// 通过依赖注入支持自定义 IO 实现（内存、网络等），便于测试和扩展。
    /// 只解析 ESP/ESM/ESL 文件本身，不加载 STRING 文件。
    ///
    /// # 参数
    /// * `path` - ESP/ESM/ESL文件路径
    /// * `reader` - 实现 EspReader trait 的读取器
    ///
    /// # 返回
    /// 返回解析后的 Plugin 实例
    ///
    /// # 示例
    /// ```rust,ignore
    /// use esp_extractor::{Plugin, DefaultEspReader};
    /// let reader = DefaultEspReader;
    /// let plugin = Plugin::load_with_reader("example.esp".into(), &reader)?;
    /// ```
    pub fn load_with_reader(
        path: PathBuf,
        reader: &dyn EspReader,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let string_records = Self::load_string_records()?;

        // 创建字符串路由器实例（v0.6.0 - P2.3）
        #[allow(deprecated)]
        let string_router = Arc::new(DefaultStringRouter::new(string_records.clone()));

        // 使用注入的 reader 读取数据（v0.6.0 - P2.4）
        let raw_data = reader.read(&path)?;
        let data_bytes = raw_data.bytes;

        // 保留对数据的引用以支持 mmap（虽然这里使用的是 Vec）
        let mmap: Option<Arc<Mmap>> = None;

        let mut cursor = Cursor::new(&data_bytes[..]);

        let header = Record::parse(&mut cursor)?;
        Self::validate_esp_file(&header)?;

        let masters = Self::extract_masters(&header);
        let groups = Self::parse_groups(&mut cursor, &data_bytes[..])?;

        #[allow(deprecated)]
        Ok(Plugin {
            path,
            header,
            groups,
            masters,
            string_records,
            string_router,
            string_files: None,
            language: String::new(),
            mmap,
        })
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

        // 创建字符串路由器实例（v0.6.0 - P2.3）
        #[allow(deprecated)]
        let string_router = Arc::new(DefaultStringRouter::new(string_records.clone()));

        // 使用内存映射文件（零拷贝，性能提升 ~500-600ms）
        let file = std::fs::File::open(&path)?;
        let mmap = unsafe { Mmap::map(&file)? };
        let mmap = Arc::new(mmap);

        let mut cursor = Cursor::new(&mmap[..]);

        let header = Record::parse(&mut cursor)?;
        Self::validate_esp_file(&header)?;

        let masters = Self::extract_masters(&header);
        let groups = Self::parse_groups(&mut cursor, &mmap[..])?;

        #[allow(deprecated)]
        Ok(Plugin {
            path,
            header,
            groups,
            masters,
            string_records,
            string_router,
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

        // 创建字符串路由器实例（v0.6.0 - P2.3）
        let string_router = Arc::new(DefaultStringRouter::new(string_records.clone()));

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
            string_router,
            string_files,
            language,
            mmap: Some(mmap),
        })
    }

    /// 验证ESP文件格式
    pub(crate) fn validate_esp_file(header: &Record) -> Result<(), Box<dyn std::error::Error>> {
        if !matches!(header.record_type.as_str(), "TES4" | "TES3") {
            return Err(EspError::InvalidFormat.into());
        }
        Ok(())
    }

    /// 解析所有组（并行版本，性能提升 1.5-2x）
    pub(crate) fn parse_groups(cursor: &mut Cursor<&[u8]>, data: &[u8]) -> Result<Vec<Group>, Box<dyn std::error::Error>> {
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

    /// 加载字符串记录定义
    pub(crate) fn load_string_records() -> Result<HashMap<String, Vec<String>>, Box<dyn std::error::Error>> {
        let json_data = include_str!("../../data/string_records.json");
        Ok(serde_json::from_str(json_data)?)
    }

    /// 从头部记录提取主文件列表
    pub(crate) fn extract_masters(header: &Record) -> Vec<String> {
        header.subrecords.iter()
            .filter(|sr| sr.record_type == "MAST")
            .map(|sr| RawString::parse_zstring(&sr.data).content)
            .collect()
    }
}
