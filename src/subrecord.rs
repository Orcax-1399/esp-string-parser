use crate::datatypes::{read_u16, read_u32};
use std::io::{Read, Cursor};

/// 子记录结构
#[derive(Debug, Clone)]
pub struct Subrecord {
    /// 4字符记录类型（原始字节）
    pub record_type_bytes: [u8; 4],
    /// 4字符记录类型（字符串，用于比较）
    pub record_type: String,
    /// 数据大小
    pub size: u16,
    /// 原始数据
    pub data: Vec<u8>,
}

impl Subrecord {
    /// 解析子记录（包括 XXXX 超大子记录）
    pub fn parse(cursor: &mut Cursor<&[u8]>) -> Result<Self, Box<dyn std::error::Error>> {
        // 检查是否有足够的数据读取头部
        if cursor.position() + 6 > cursor.get_ref().len() as u64 {
            return Err("Insufficient data for subrecord header".into());
        }

        let current_pos = cursor.position();

        // 读取记录类型 (4字节)
        let mut type_bytes = [0u8; 4];
        cursor.read_exact(&mut type_bytes)?;
        let record_type = String::from_utf8_lossy(&type_bytes).into_owned();

        // 读取数据大小 (2字节)
        let size = read_u16(cursor)?;

        // 🔧 特殊处理：XXXX 超大子记录
        if record_type == "XXXX" {
            // XXXX 的 size 必须是 4
            if size != 4 {
                return Err(format!("XXXX subrecord size should be 4, got {}", size).into());
            }

            // 读取真实字段大小（32位整数）
            let field_size = read_u32(cursor)?;

            #[cfg(debug_assertions)]
            {
                eprintln!("📦 检测到 XXXX 超大子记录");
                eprintln!("  真实数据大小: {} bytes (0x{:X})", field_size, field_size);
            }

            // 读取后续 subrecord 的头部
            let mut next_type_bytes = [0u8; 4];
            cursor.read_exact(&mut next_type_bytes)?;
            let next_type = String::from_utf8_lossy(&next_type_bytes).into_owned();

            let next_size = read_u16(cursor)?;
            if next_size != 0 {
                eprintln!(
                    "  ⚠️ 警告：XXXX 后续子记录 {} 的 size 不为 0，实际值: {}",
                    next_type, next_size
                );
            }

            #[cfg(debug_assertions)]
            eprintln!("  后续子记录类型: {}", next_type);

            // 读取实际数据
            let mut data = vec![0u8; field_size as usize];
            cursor.read_exact(&mut data)?;

            #[cfg(debug_assertions)]
            eprintln!("  ✓ XXXX 子记录解析成功");

            // 返回一个表示实际子记录的 Subrecord
            // 注意：size 字段用 u16，但实际大小可能超过 65535
            // 我们将其设置为 0 作为标记，实际大小由 data.len() 决定
            Ok(Subrecord {
                record_type_bytes: next_type_bytes,
                record_type: next_type,
                size: 0,  // 标记为 XXXX 子记录
                data,
            })
        } else {
            // 普通子记录处理
            // 检查是否有足够的数据
            if cursor.position() + size as u64 > cursor.get_ref().len() as u64 {
                let error_msg = format!(
                    "Insufficient data for subrecord data: type='{}' (0x{:02X?}), expected {} bytes, but only {} bytes remaining (pos=0x{:X}, total={})",
                    record_type,
                    type_bytes,
                    size,
                    cursor.get_ref().len() as u64 - cursor.position(),
                    current_pos,
                    cursor.get_ref().len()
                );
                return Err(error_msg.into());
            }

            // 读取数据
            let mut data = vec![0u8; size as usize];
            cursor.read_exact(&mut data)?;

            Ok(Subrecord {
                record_type_bytes: type_bytes,
                record_type,
                size,
                data,
            })
        }
    }
    
    /// 获取子记录类型
    pub fn get_type(&self) -> &str {
        &self.record_type
    }
    
    /// 获取数据
    pub fn get_data(&self) -> &[u8] {
        &self.data
    }
    
    /// 检查是否为字符串类型的子记录
    pub fn is_string_type(&self, string_types: &[String]) -> bool {
        string_types.iter().any(|t| t == &self.record_type)
    }
} 