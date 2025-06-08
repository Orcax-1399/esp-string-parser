use crate::datatypes::{read_u16, read_u32, RecordFlags};
use crate::subrecord::Subrecord;
use std::io::{Read, Cursor};
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use std::io::Write;

/// 记录结构
#[derive(Debug)]
pub struct Record {
    /// 记录类型（原始4字节）
    pub record_type_bytes: [u8; 4],
    /// 记录类型字符串（用于比较）
    pub record_type: String,
    /// 数据大小
    pub data_size: u32,
    /// 标志位（原始32位数据）
    pub flags: u32,
    /// FormID
    pub form_id: u32,
    /// 时间戳
    pub timestamp: u16,
    /// 版本控制信息
    pub version_control_info: u16,
    /// 内部版本
    pub internal_version: u16,
    /// 未知字段
    pub unknown: u16,
    /// 原始压缩数据（如果记录是压缩的，保存原始压缩字节）
    pub original_compressed_data: Option<Vec<u8>>,
    /// 原始数据（用于保持压缩记录的完整性）
    pub raw_data: Vec<u8>,
    /// 子记录列表
    pub subrecords: Vec<Subrecord>,
    /// 是否已被修改（用于智能压缩处理）
    pub is_modified: bool,
}

impl Record {
    /// 解析记录
    pub fn parse(cursor: &mut Cursor<&[u8]>) -> Result<Self, Box<dyn std::error::Error>> {
        Self::validate_header_size(cursor)?;
        
        let mut type_bytes = [0u8; 4];
        cursor.read_exact(&mut type_bytes)?;
        let record_type = String::from_utf8_lossy(&type_bytes).into_owned();
        
        let data_size = read_u32(cursor)?;
        Self::validate_data_size(data_size, &record_type)?;
        
        let flags_raw_bytes = read_u32(cursor)?;
        
        #[cfg(debug_assertions)]
        Self::debug_record_parsing(&record_type, flags_raw_bytes, cursor.position());
        
        let form_id = read_u32(cursor)?;
        let timestamp = read_u16(cursor)?;
        let version_control_info = read_u16(cursor)?;
        let internal_version = read_u16(cursor)?;
        let unknown = read_u16(cursor)?;
        
        #[cfg(debug_assertions)]
        Self::debug_record_details(&record_type, form_id, data_size, timestamp, version_control_info, internal_version, unknown, flags_raw_bytes);
        
        Self::validate_data_availability(cursor, data_size)?;
        
        let mut data = vec![0u8; data_size as usize];
        cursor.read_exact(&mut data)?;
        
        let (final_data, parse_subrecords, original_compressed) = 
            Self::handle_compression(&data, flags_raw_bytes, &record_type)?;

        let subrecords = if parse_subrecords {
            Self::parse_subrecords(&final_data)?
        } else {
            Vec::new()
        };

        Ok(Record {
            record_type_bytes: type_bytes,
            record_type,
            data_size,
            flags: flags_raw_bytes,
            form_id,
            timestamp,
            version_control_info,
            internal_version,
            unknown,
            original_compressed_data: original_compressed,
            raw_data: final_data,
            subrecords,
            is_modified: false,
        })
    }
    
    /// 验证头部大小
    fn validate_header_size(cursor: &Cursor<&[u8]>) -> Result<(), Box<dyn std::error::Error>> {
        if cursor.position() + 24 > cursor.get_ref().len() as u64 {
            return Err("Insufficient data for record header".into());
        }
        Ok(())
    }
    
    /// 验证数据大小
    fn validate_data_size(data_size: u32, record_type: &str) -> Result<(), Box<dyn std::error::Error>> {
        if data_size > 100_000_000 {  // 100MB限制
            return Err(format!("记录 {} 数据大小异常: {} bytes (可能数据损坏)", 
                record_type, data_size).into());
        }
        Ok(())
    }
    
    /// 验证数据可用性
    fn validate_data_availability(cursor: &Cursor<&[u8]>, data_size: u32) -> Result<(), Box<dyn std::error::Error>> {
        if cursor.position() + data_size as u64 > cursor.get_ref().len() as u64 {
            return Err(format!("Insufficient data for record data: expected {} bytes", data_size).into());
        }
        Ok(())
    }
    
    /// 调试记录解析信息
    #[cfg(debug_assertions)]
    fn debug_record_parsing(record_type: &str, flags: u32, position: u64) {
        if ["STAT", "CONT", "GLOB", "ARMO", "WEAP", "NPC_"].contains(&record_type) {
            println!("=== 解析记录 {} (位置: 0x{:X}) ===", record_type, position - 16);
            println!("原始标志位: 0x{:08X} ({:032b})", flags, flags);
            
            let flag_bytes = flags.to_le_bytes();
            println!("标志位字节序列: [{:02X} {:02X} {:02X} {:02X}]", 
                flag_bytes[0], flag_bytes[1], flag_bytes[2], flag_bytes[3]);
        }
    }
    
    /// 调试记录详细信息
    #[cfg(debug_assertions)]
    fn debug_record_details(record_type: &str, form_id: u32, data_size: u32, timestamp: u16, 
                           version_control_info: u16, internal_version: u16, unknown: u16, flags: u32) {
        if ["STAT", "CONT", "GLOB", "ARMO", "WEAP", "NPC_"].contains(&record_type) {
            println!("FormID: 0x{:08X}", form_id);
            println!("数据大小: {} bytes", data_size);
            println!("时间戳: {}", timestamp);
            println!("版本控制: {}", version_control_info);
            println!("内部版本: {}", internal_version);
            println!("未知字段: {}", unknown);
            
            Self::debug_flags_internal(flags);
            
            if flags == 0 {
                println!("⚠ 警告: 标志位为零！可能存在解析问题");
            }
            
            println!("===========================");
        }
    }
    
    /// 调试标志位信息
    #[cfg(debug_assertions)]
    fn debug_flags_internal(flags: u32) {
        if flags & RecordFlags::COMPRESSED.bits() != 0 {
            println!("  - 包含压缩标志 (0x{:08X})", RecordFlags::COMPRESSED.bits());
        }
        if flags & RecordFlags::DELETED.bits() != 0 {
            println!("  - 包含删除标志 (0x{:08X})", RecordFlags::DELETED.bits());
        }
        if flags & RecordFlags::PERSISTENT.bits() != 0 {
            println!("  - 包含持久化标志 (0x{:08X})", RecordFlags::PERSISTENT.bits());
        }
        if flags & RecordFlags::DISABLED.bits() != 0 {
            println!("  - 包含禁用标志 (0x{:08X})", RecordFlags::DISABLED.bits());
        }
        if flags & RecordFlags::MASTER_FILE.bits() != 0 {
            println!("  - 包含主文件标志 (0x{:08X})", RecordFlags::MASTER_FILE.bits());
        }
        if flags & RecordFlags::LOCALIZED.bits() != 0 {
            println!("  - 包含本地化标志 (0x{:08X})", RecordFlags::LOCALIZED.bits());
        }
        if flags & RecordFlags::LIGHT_MASTER.bits() != 0 {
            println!("  - 包含轻量级主文件标志 (0x{:08X})", RecordFlags::LIGHT_MASTER.bits());
        }
        if flags & RecordFlags::VISIBLE_DISTANT.bits() != 0 {
            println!("  - 包含远距离可见标志 (0x{:08X})", RecordFlags::VISIBLE_DISTANT.bits());
        }
    }
    
    /// 处理压缩数据
    fn handle_compression(data: &[u8], flags: u32, record_type: &str) -> Result<(Vec<u8>, bool, Option<Vec<u8>>), Box<dyn std::error::Error>> {
        if flags & RecordFlags::COMPRESSED.bits() != 0 {
            match Self::decompress_data(data) {
                Ok(decompressed) => {
                    #[cfg(debug_assertions)]
                    println!("成功解压记录 {}: {} -> {} bytes", record_type, data.len(), decompressed.len());
                    
                    Ok((decompressed, true, Some(data.to_vec())))
                },
                Err(e) => {
                    eprintln!("警告: 记录 {} 解压失败: {}，跳过子记录解析", record_type, e);
                    Ok((data.to_vec(), false, Some(data.to_vec())))
                }
            }
        } else {
            Ok((data.to_vec(), true, None))
        }
    }
    
    /// 解压缩数据
    fn decompress_data(data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        if data.len() < 4 {
            return Err("压缩数据太短，无法包含解压大小".into());
        }
        
        let mut data_cursor = Cursor::new(data);
        let decompressed_size = read_u32(&mut data_cursor)?;
        
        Self::validate_decompressed_size(decompressed_size)?;
        
        let compressed_data = &data[4..];
        if compressed_data.is_empty() {
            return Err("没有压缩数据".into());
        }
        
        Self::validate_zlib_header(compressed_data)?;
        
        let mut decoder = ZlibDecoder::new(compressed_data);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;
        
        if decompressed.len() != decompressed_size as usize {
            return Err(format!("解压大小不匹配: 期望 {} bytes，实际 {} bytes", 
                decompressed_size, decompressed.len()).into());
        }
        
        Ok(decompressed)
    }
    
    /// 验证解压大小
    fn validate_decompressed_size(size: u32) -> Result<(), Box<dyn std::error::Error>> {
        if size == 0 {
            return Err("解压大小为0".into());
        }
        
        if size > 50_000_000 {  // 50MB限制
            return Err(format!("解压大小过大: {} bytes (可能数据损坏)", size).into());
        }
        
        Ok(())
    }
    
    /// 验证zlib头部
    fn validate_zlib_header(data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        if data.len() >= 2 {
            let first_byte = data[0];
            
            #[cfg(debug_assertions)]
            if first_byte != 0x78 {
                println!("警告: 不是标准zlib头部 (0x{:02X})，尝试解压", first_byte);
            }
        }
        Ok(())
    }
    
    /// 解析子记录
    fn parse_subrecords(data: &[u8]) -> Result<Vec<Subrecord>, Box<dyn std::error::Error>> {
        let mut subrecords = Vec::new();
        let mut cursor = Cursor::new(data);
        
        while cursor.position() < data.len() as u64 {
            let subrecord = Subrecord::parse(&mut cursor)?;
            subrecords.push(subrecord);
        }
        
        Ok(subrecords)
    }
    
    /// 获取记录类型
    pub fn get_type(&self) -> &str {
        &self.record_type
    }
    
    /// 获取FormID
    pub fn get_form_id(&self) -> u32 {
        self.form_id
    }
    
    /// 获取标志位
    pub fn get_flags(&self) -> RecordFlags {
        RecordFlags::from_bits_truncate(self.flags)
    }
    
    /// 查找子记录
    pub fn find_subrecord(&self, record_type: &str) -> Option<&Subrecord> {
        self.subrecords.iter().find(|sr| sr.record_type == record_type)
    }
    
    /// 查找所有匹配的子记录
    pub fn find_subrecords(&self, record_type: &str) -> Vec<&Subrecord> {
        self.subrecords.iter().filter(|sr| sr.record_type == record_type).collect()
    }
    
    /// 获取编辑器ID
    pub fn get_editor_id(&self) -> Option<String> {
        self.find_subrecord("EDID")
            .map(|sr| String::from_utf8_lossy(&sr.data).trim_end_matches('\0').to_string())
    }
    
    /// 重新压缩数据
    pub fn recompress_data(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let mut subrecord_data = Vec::new();
        for subrecord in &self.subrecords {
            subrecord_data.extend_from_slice(&subrecord.record_type_bytes);
            subrecord_data.extend_from_slice(&subrecord.size.to_le_bytes());
            subrecord_data.extend_from_slice(&subrecord.data);
        }
        
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&subrecord_data)?;
        let compressed_data = encoder.finish()?;
        
        let mut result = Vec::new();
        result.extend_from_slice(&(subrecord_data.len() as u32).to_le_bytes());
        result.extend_from_slice(&compressed_data);
        
        Ok(result)
    }
    
    /// 标记为已修改
    pub fn mark_modified(&mut self) {
        self.is_modified = true;
    }
    
    /// 调试标志位（公共方法）
    pub fn debug_flags(&self) {
        #[cfg(debug_assertions)]
        Self::debug_flags_internal(self.flags);
    }
} 