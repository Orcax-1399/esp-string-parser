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
    #[allow(clippy::too_many_arguments)]
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
    #[allow(clippy::type_complexity)]
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
    fn validate_zlib_header(_data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        #[cfg(debug_assertions)]
        if _data.len() >= 2 {
            let first_byte = _data[0];
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
            // 检查剩余字节数
            let remaining = data.len() as u64 - cursor.position();

            // 子记录最小头部大小为 6 字节 (4字节类型 + 2字节大小)
            // 如果剩余字节 < 6，检查是否为 NULL 填充
            if remaining < 6 {
                let remaining_bytes = &data[cursor.position() as usize..];

                // 检查是否全为 NULL (0x00) - 这是合法的填充字节
                if remaining_bytes.iter().all(|&b| b == 0) {
                    // 这是 NULL 填充，安全跳过
                    #[cfg(debug_assertions)]
                    println!("跳过 {} 字节的 NULL 填充", remaining);
                    break;
                } else {
                    // 不是填充，这是真正的错误
                    return Err(format!(
                        "记录末尾有 {} 字节非 NULL 数据，无法解析为子记录: {:02X?}",
                        remaining, remaining_bytes
                    ).into());
                }
            }

            let pos_before = cursor.position();

            match Subrecord::parse(&mut cursor) {
                Ok(subrecord) => {
                    subrecords.push(subrecord);
                }
                Err(e) => {
                    eprintln!("\n❌ 子记录解析失败");
                    eprintln!("  数据总长度: {} bytes", data.len());
                    eprintln!("  失败位置: 0x{:X} ({})", pos_before, pos_before);
                    eprintln!("  剩余数据: {} bytes", remaining);
                    eprintln!("  已成功解析子记录数: {}", subrecords.len());

                    if subrecords.len() > 0 {
                        let last = &subrecords[subrecords.len() - 1];
                        eprintln!("  前一个成功的子记录: {} (size: {})", last.record_type, last.size);
                    }

                    // 显示失败位置附近的原始字节（前后各16字节）
                    let show_start = pos_before.saturating_sub(16) as usize;
                    let show_end = ((pos_before + 32).min(data.len() as u64)) as usize;
                    eprintln!("  失败位置附近的原始数据 (0x{:X} - 0x{:X}):", show_start, show_end);
                    eprintln!("    {:02X?}", &data[show_start..show_end]);

                    return Err(e);
                }
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试 1 字节 NULL 填充
    #[test]
    fn test_null_padding_1byte() {
        // 构造测试数据: EDID 子记录 + 1 字节 NULL 填充
        let data = vec![
            b'E', b'D', b'I', b'D', 0x04, 0x00, // EDID, size=4
            b't', b'e', b's', b't',              // 内容 "test"
            0x00,                                 // 1 字节填充
        ];

        let result = Record::parse_subrecords(&data);
        assert!(result.is_ok(), "应该成功解析带 1 字节填充的记录");

        let subrecords = result.unwrap();
        assert_eq!(subrecords.len(), 1, "应该解析出 1 个子记录");
        assert_eq!(subrecords[0].record_type, "EDID");
    }

    /// 测试 4 字节 NULL 填充
    #[test]
    fn test_null_padding_4bytes() {
        // 构造测试数据: EDID 子记录 + 4 字节 NULL 填充
        let data = vec![
            b'E', b'D', b'I', b'D', 0x04, 0x00,
            b't', b'e', b's', b't',
            0x00, 0x00, 0x00, 0x00, // 4 字节填充
        ];

        let result = Record::parse_subrecords(&data);
        assert!(result.is_ok(), "应该成功解析带 4 字节填充的记录");

        let subrecords = result.unwrap();
        assert_eq!(subrecords.len(), 1);
    }

    /// 测试 7 字节 NULL 填充（最大情况）
    #[test]
    fn test_null_padding_7bytes() {
        let data = vec![
            b'F', b'U', b'L', b'L', 0x05, 0x00,
            b'S', b'w', b'o', b'r', b'd',
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // 7 字节填充
        ];

        let result = Record::parse_subrecords(&data);
        assert!(result.is_ok(), "应该成功解析带 7 字节填充的记录");
    }

    /// 测试无填充的正常记录
    #[test]
    fn test_no_padding() {
        let data = vec![
            b'E', b'D', b'I', b'D', 0x04, 0x00,
            b't', b'e', b's', b't',
        ];

        let result = Record::parse_subrecords(&data);
        assert!(result.is_ok(), "应该成功解析无填充的记录");
        assert_eq!(result.unwrap().len(), 1);
    }

    /// 测试多个子记录 + 填充
    #[test]
    fn test_multiple_subrecords_with_padding() {
        let data = vec![
            b'E', b'D', b'I', b'D', 0x04, 0x00,
            b't', b'e', b's', b't',
            b'F', b'U', b'L', b'L', 0x05, 0x00,
            b'S', b'w', b'o', b'r', b'd',
            0x00, 0x00, // 2 字节填充
        ];

        let result = Record::parse_subrecords(&data);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2, "应该解析出 2 个子记录");
    }

    /// 测试非 NULL 的无效尾部数据应该报错
    #[test]
    fn test_invalid_trailing_data() {
        // 尾部有非 NULL 字节
        let data = vec![
            b'E', b'D', b'I', b'D', 0x04, 0x00,
            b't', b'e', b's', b't',
            0xFF, 0xAA, // 无效的尾部字节
        ];

        let result = Record::parse_subrecords(&data);
        assert!(result.is_err(), "非 NULL 的尾部数据应该报错");

        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("非 NULL 数据"), "错误信息应包含'非 NULL 数据'");
    }

    /// 测试混合非 NULL 填充（部分 NULL 部分非 NULL）
    #[test]
    fn test_mixed_invalid_padding() {
        let data = vec![
            b'E', b'D', b'I', b'D', 0x04, 0x00,
            b't', b'e', b's', b't',
            0x00, 0xFF, 0x00, // 混合填充
        ];

        let result = Record::parse_subrecords(&data);
        assert!(result.is_err(), "混合填充应该报错");
    }
} 