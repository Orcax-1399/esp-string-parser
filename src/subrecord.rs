use crate::datatypes::{read_u16};
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
    /// 解析子记录
    pub fn parse(cursor: &mut Cursor<&[u8]>) -> Result<Self, Box<dyn std::error::Error>> {
        // 检查是否有足够的数据读取头部
        if cursor.position() + 6 > cursor.get_ref().len() as u64 {
            return Err("Insufficient data for subrecord header".into());
        }
        
        // 读取记录类型 (4字节)
        let mut type_bytes = [0u8; 4];
        cursor.read_exact(&mut type_bytes)?;
        let record_type = String::from_utf8_lossy(&type_bytes).into_owned();
        
        // 读取数据大小 (2字节)
        let size = read_u16(cursor)?;
        
        // 检查是否有足够的数据
        if cursor.position() + size as u64 > cursor.get_ref().len() as u64 {
            return Err(format!("Insufficient data for subrecord data: expected {} bytes", size).into());
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